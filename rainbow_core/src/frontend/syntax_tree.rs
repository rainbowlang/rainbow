use std::fmt;

use pest::iterators::Pair;
use id_tree::{InsertBehavior, Node, NodeId, NodeIdError, PreOrderTraversal, Tree, TreeBuilder};

use primitive::Prim;
use arena::*;
use frontend::grammar::Rule;

pub struct SyntaxTree<'i> {
  pub input: &'i str,
  pub nodes: Tree<NodeData>,
  pub constants: Arena<Prim>,
  pub symbols: Arena<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NodeData {
  pub node_type: NodeType,
  pub start_pos: usize,
  pub end_pos: usize,
}

pub type AstNode = Node<NodeData>;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum NodeType {
  Root,
  Primitive(ArenaId),
  Ident(ArenaId),
  Variable,
  List,
  Record,
  RecordEntry,
  Apply,
  Argument,
  Keyword(ArenaId),
  Block,
  BlockArgs,
}

impl<'i> SyntaxTree<'i> {
  pub fn from_input_and_pair(
    ns_symbols: &Arena<String>,
    input: &'i str,
    pair: Pair<'i, Rule>,
  ) -> Result<Self, NodeIdError> {
    let mut tree = SyntaxTree::for_input(ns_symbols, input);
    tree.consume_pair(pair, InsertBehavior::AsRoot)?;
    Ok(tree)
  }

  fn for_input(ns_symbols: &Arena<String>, input: &'i str) -> Self {
    let node_cap = input.len() / 4;
    let const_cap = input.len() / 16;
    SyntaxTree {
      input: input,
      nodes: TreeBuilder::new().with_node_capacity(node_cap).build(),
      constants: Arena::with_capacity(const_cap),
      symbols: ns_symbols.clone(),
    }
  }

  #[inline]
  fn intern_constant<T: Into<Prim>>(&mut self, c: T) -> NodeType {
    NodeType::Primitive(self.constants.intern(c.into()))
  }

  #[inline]
  pub fn lookup_constant(&self, id: ArenaId) -> &Prim {
    self.constants.resolve(id)
  }

  #[inline]
  pub fn node_str(&self, data: &NodeData) -> &'i str {
    &self.input[data.start_pos..data.end_pos]
  }

  #[inline]
  pub fn node_id_str(&self, id: &NodeId) -> Result<&'i str, NodeIdError> {
    self.nodes.get(id).map(|node| self.node_str(node.data()))
  }

  #[inline]
  pub fn node_id_to_symbol_id(&self, id: &NodeId) -> Result<ArenaId, NodeIdError> {
    use frontend::NodeType::{Ident, Keyword};
    let data = self.node_data(id)?;
    if let Ident(id) = data.node_type {
      Ok(id)
    } else if let Keyword(id) = data.node_type {
      Ok(id)
    } else {
      Err(NodeIdError::NodeIdNoLongerValid)
    }
  }

  #[inline]
  pub fn node_data(&self, id: &NodeId) -> Result<&NodeData, NodeIdError> {
    self.nodes.get(id).map(|node| node.data())
  }

  pub fn traverse<'a>(&'a self) -> Result<PreOrderTraversal<'a, NodeData>, NodeIdError> {
    self
      .nodes
      .traverse_pre_order(self.nodes.root_node_id().unwrap())
  }

  /*
  pub fn walk<E, F>(&self, f: F) -> Result<(), E>
  where
    E: From<NodeIdError>,
    F: FnMut(&Node<NodeData>) -> Result<WalkCmd, E>,
  {
    TreeWalk::walk(&self.nodes, f)
  }
  */

  fn consume_pair(
    &mut self,
    pair: Pair<'i, Rule>,
    insert_as: InsertBehavior,
  ) -> Result<(), NodeIdError> {
    use self::InsertBehavior::UnderNode;
    use self::NodeType::*;

    let node_type = match pair.as_rule() {
      Rule::apply => Apply,
      Rule::argument => Argument,
      Rule::variable => Variable,
      Rule::list => List,
      Rule::ident => Ident(self.symbols.intern(pair.as_str())),
      Rule::keyword => Keyword(self.symbols.intern({
        let s = pair.as_str();
        &s[0..s.len() - 1]
      })),
      Rule::record => Record,
      Rule::entry => RecordEntry,
      Rule::block => Block,
      Rule::block_args => BlockArgs,

      Rule::string => {
        let mut s = pair.as_str();
        s = &s[1..s.len() - 1];
        self.intern_constant(String::from(s))
      }

      Rule::bool => match pair.as_str() {
        "true" => self.intern_constant(true),
        "false" => self.intern_constant(false),
        _ => panic!(format!("grammar rule `bool` rule matched {:?}", pair)),
      },

      Rule::number => {
        let n: f64 = pair.as_str().parse().unwrap();
        self.intern_constant(n)
      }
      rule => panic!("can't treeify {:?}", rule),
    };

    let node_data = {
      let span = pair.clone().into_span();
      NodeData {
        node_type: node_type,
        start_pos: span.start(),
        end_pos: span.end(),
      }
    };
    let node_id = self.nodes.insert(Node::new(node_data), insert_as)?;
    for inner in pair.into_inner() {
      self.consume_pair(inner, UnderNode(&node_id))?;
    }
    Ok(())
  }

  fn print_node(&self, f: &mut fmt::Formatter, node_id: &NodeId) -> fmt::Result {
    use self::NodeType::*;
    use std::fmt::Write;

    let node = self.nodes.get(node_id).unwrap();
    let data = node.data();

    match data.node_type {
      Root => for child in node.children() {
        self.print_node(f, child)?;
        f.write_char('\n')?;
      },
      Primitive(id) => write!(f, "{}", self.constants.resolve(id))?,
      Variable => for (i, child) in node.children().into_iter().enumerate() {
        if i != 0 {
          f.write_char('.')?;
        }
        self.print_node(f, child)?;
      },
      Ident(id) => write!(f, "{}", self.symbols.resolve(id))?,
      Keyword(id) => write!(f, "{}:", self.symbols.resolve(id))?,
      List => {
        f.write_char('[')?;
        for (i, child) in node.children().into_iter().enumerate() {
          if i != 0 {
            f.write_char(' ')?;
          }
          self.print_node(f, child)?;
        }
        f.write_char(']')?;
      }
      Record => {
        f.write_char('[')?;
        for (i, child) in node.children().into_iter().enumerate() {
          if i > 0 {
            f.write_char(' ')?;
          }
          self.print_node(f, child)?;
        }
        f.write_char(']')?;
      }
      RecordEntry => {
        let children = node.children();
        self.print_node(f, &children[0])?;
        f.write_char('=')?;
        self.print_node(f, &children[1])?;
      }
      Apply | Argument => for (i, child) in node.children().into_iter().enumerate() {
        if i > 0 {
          f.write_char(' ')?;
        }
        self.print_node(f, child)?;
      },
      Block => {
        write!(f, "{{ ")?;
        for child in node.children() {
          self.print_node(f, child)?;
        }
        write!(f, " }}")?;
      }
      BlockArgs => {
        let children = node.children();
        if children.len() > 0 {
          for child in children {
            self.print_node(f, child)?;
            f.write_char(' ')?;
          }
          write!(f, "=> ")?;
        }
      }
    }
    Ok(())
  }
}

impl<'a> fmt::Display for SyntaxTree<'a> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    if let Some(node_id) = self.nodes.root_node_id() {
      self.print_node(f, node_id)?;
    }
    Ok(())
  }
}

#[test]
fn test_print_tree() {
  use test_helpers::*;
  let tree = parse_with_prelude(
    "each: offices
     do: { office => [
       name = try: office.name else: { stringify: office.id }
       employees = each: office.employees do: { e => upperCase: e.name }
     ] }",
  );

  assert_eq!(format!("{}", tree), "each: offices do: { office => [name=try: { office.name } else: { stringify: office.id } employees=each: office.employees do: { e => upperCase: e.name }] }");
}
