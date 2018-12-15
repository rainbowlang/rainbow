use id_tree::{NodeId, NodeIdError};
use crate::frontend::SyntaxTree;
use super::Instruction;

pub fn emit<'i>(tree: &'i SyntaxTree<'i>) -> Result<Vec<Instruction>, NodeIdError> {
  if let Some(root_node_id) = tree.nodes.root_node_id() {
    let mut emitter = Emitter::new(tree);
    emitter.recur(root_node_id)?;
    Ok(emitter.instructions)
  } else {
    Ok(vec![])
  }
}

struct Emitter<'t> {
  tree: &'t SyntaxTree<'t>,
  instructions: Vec<Instruction>,
}

impl<'t> Emitter<'t> {
  pub fn new(tree: &'t SyntaxTree) -> Self {
    Emitter {
      tree: tree,
      instructions: Vec::with_capacity(1024),
    }
  }

  fn recur(&mut self, node_id: &NodeId) -> Result<(), NodeIdError> {
    use crate::frontend::NodeType::*;
    use super::Instruction::*;
    let node = self.tree.nodes.get(node_id)?;
    let data = node.data();
    dbg!("infer {:?}", node.data());
    match data.node_type {
      Root => for child_id in node.children() {
        self.recur(child_id)?;
      },
      Primitive(id) => {
        self.instructions.push(PushPrimitive { id: id });
      }
      List => {
        let children = node.children();
        let size = children.len();
        for elem_id in children {
          self.recur(elem_id)?;
        }
        self.instructions.push(MkList { size: size as u16 });
      }

      Record => {
        let children = node.children();
        let size = children.len();
        for entry_id in children {
          // get the field/value node ID's for the RecordEntry node
          let name_and_value_ids: Vec<_> =
            self.tree.nodes.children_ids(entry_id).unwrap().collect();

          let field_name = self.tree.node_id_to_symbol_id(&name_and_value_ids[0])?;
          self.instructions.push(PushKeyword { id: field_name });
          self.recur(&name_and_value_ids[1])?;
        }
        self.instructions.push(MkRecord { size: size as u16 });
      }

      Variable => {
        let children = node.children();
        let root_name = self.tree.node_id_to_symbol_id(&children[0])?;
        self.instructions.push(PushVar { id: root_name });

        for child_id in children[1..].iter() {
          let prop_name = self.tree.node_id_to_symbol_id(&child_id)?;
          self.instructions.push(PushProp { id: prop_name });
        }
      }

      Block => {
        let jump_ip = self.instructions.len();
        self.instructions.push(MkBlock { argc: 0, skip: 0 });
        let mut argc = 0;
        let children = node.children();
        if children.len() > 1 {
          let arg_node_ids = self.tree.nodes.get(&children[0])?.children();
          argc = arg_node_ids.len() as u8;
          for arg_node_id in arg_node_ids {
            let arg_name = self.tree.node_id_to_symbol_id(arg_node_id)?;
            self.instructions.push(Bind { id: arg_name });
          }
        }
        if children.len() > 0 {
          self.recur(&children[children.len() - 1])?;
        }
        let skip = self.instructions.len() - (jump_ip + 1);
        self.instructions[jump_ip] = MkBlock {
          argc: argc,
          skip: skip as u16,
        };
      }

      Apply => {
        let children = node.children();
        for child_id in children.iter() {
          let arg_children = self.tree.nodes.get(&child_id)?.children();
          let arg_name = self.tree.node_id_to_symbol_id(&arg_children[0])?;
          self.instructions.push(PushKeyword { id: arg_name });
          self.recur(&arg_children[1])?;
        }
        self.instructions.push(CallFunction {
          argc: children.len() as u16,
        });
      }
      // other node types won't be visited, and should emit no instructions
      _ => {}
    }
    return Ok(());
  }
}

#[cfg(test)]
mod tests {
  use super::emit;
  use crate::test_helpers::*;
  use crate::interpreter::Instruction::*;

  #[test]
  fn test_emit_var() {
    let tree = parse_with_prelude("x");
    let x_id = tree.symbols.find(&"x").unwrap();
    let instructions = emit(&tree).unwrap();
    assert_eq!(instructions, vec![PushVar { id: x_id }]);
  }

  #[test]
  fn test_emit_var_path() {
    let tree = parse_with_prelude("x.y");
    let instructions = emit(&tree).unwrap();
    let x_id = tree.symbols.find(&"x").unwrap();
    let y_id = tree.symbols.find(&"y").unwrap();
    assert_eq!(
      instructions,
      vec![PushVar { id: x_id }, PushProp { id: y_id }]
    );
  }

  #[test]
  fn test_emit_list() {
    let tree = parse_with_prelude("[ 1 2 3 ]");
    let instructions = emit(&tree).unwrap();
    assert_eq!(
      instructions,
      vec![
        PushPrimitive { id: 0 },
        PushPrimitive { id: 1 },
        PushPrimitive { id: 2 },
        MkList { size: 3 },
      ]
    );
  }

  #[test]
  fn test_emit_record() {
    use crate::test_helpers::*;
    let tree = parse_with_prelude("[ x = 3 y = \"hello\" ]");
    let instructions = emit(&tree).unwrap();
    let x_id = tree.symbols.find(&"x").unwrap();
    let y_id = tree.symbols.find(&"y").unwrap();
    assert_eq!(
      instructions,
      vec![
        PushKeyword { id: x_id },
        PushPrimitive { id: 0 },
        PushKeyword { id: y_id },
        PushPrimitive { id: 1 },
        MkRecord { size: 2 },
      ]
    );
  }

  #[test]
  fn test_emit_block() {
    use crate::test_helpers::*;
    let tree = parse_with_prelude("{ x y => [y x] }");
    let instructions = emit(&tree).unwrap();
    let x_id = tree.symbols.find(&"x").unwrap();
    let y_id = tree.symbols.find(&"y").unwrap();
    assert_eq!(
      instructions,
      vec![
        MkBlock { argc: 2, skip: 5 },
        Bind { id: x_id },
        Bind { id: y_id },
        PushVar { id: y_id },
        PushVar { id: x_id },
        MkList { size: 2 },
      ]
    );
  }

  #[test]
  fn test_emit_function_call() {
    use crate::test_helpers::*;
    let tree = parse_with_prelude("calc: 2 plus: 2");
    let instructions = emit(&tree).unwrap();
    let calc_id = tree.symbols.find(&"calc").unwrap();
    let plus_id = tree.symbols.find(&"plus").unwrap();
    assert_eq!(
      instructions,
      vec![
        PushKeyword { id: calc_id },
        PushPrimitive { id: 0 },
        PushKeyword { id: plus_id },
        PushPrimitive { id: 0 },
        CallFunction { argc: 2 },
      ]
    );
  }
}
