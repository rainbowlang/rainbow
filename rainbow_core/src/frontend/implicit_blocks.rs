use id_tree::{InsertBehavior, Node, NodeIdError, RemoveBehavior, SwapBehavior};

use typing::Type;
use namespace::INamespace;
use frontend::{NodeData, NodeType, SyntaxTree};

/// Rewrite a syntax tree, adding & removing implicit blocks.
pub fn rewrite<NS: INamespace>(ns: &NS, tree: &mut SyntaxTree) -> Result<(), NodeIdError> {
  let mut nodes_to_wrap = Vec::with_capacity(10);
  let mut nodes_to_unwrap = Vec::with_capacity(10);
  for node in tree.traverse()? {
    if node.data().node_type == NodeType::Apply {
      let children = node.children();
      let arg0 = tree.nodes.get(&children[0]).unwrap();
      let signature = {
        let func_name = tree
          .node_id_str(&arg0.children()[0])?
          .trim_right_matches(':');
        let sig = ns.get_signature(func_name);
        // can't rewrite args if we don't have a signature
        if sig.is_none() {
          continue;
        }
        sig.unwrap()
      };
      for arg_node_id in children {
        let arg_children = tree.nodes.get(&arg_node_id)?.children();
        let keyword = tree.node_id_to_symbol_id(&arg_children[0])?;
        let arg_spec = match signature.arg(keyword) {
          Some(spec) => spec,
          None => continue,
        };
        let val_node = tree.nodes.get(&arg_children[1])?;
        match (&arg_spec.ty, val_node.data().node_type) {
          (&Type::Block(_, _), NodeType::Block) => {
            // do nothing
          }
          (&Type::Block(_, _), _some_other_node_type) => {
            nodes_to_wrap.push(arg_children[1].clone());
          }
          (_other_type, NodeType::Block) => {
            let block_children = val_node.children();
            // zero-argument block was provided where value was expected
            if block_children.len() == 1 {
              nodes_to_unwrap.push(arg_children[1].clone());
            }
            // function expected a value but user provided a block with inputs,
            // leave it alone for the type checker to error on.
          }
          (_, _) => {
            // do nothing
          }
        }
      }
    }
  }


  // wrap each node we found during traversal
  for node_id in nodes_to_wrap {
    // insert a new block node with an empty args list, then move this node under it
    let (start_pos, end_pos) = {
      let data = tree.nodes.get(&node_id).unwrap().data();
      (data.start_pos, data.end_pos)
    };
    let block_node_id = tree.nodes.insert(
      Node::new(NodeData {
        node_type: NodeType::Block,
        start_pos: start_pos,
        end_pos: end_pos,
      }),
      InsertBehavior::UnderNode(&node_id),
    )?;
    // swapping moves the old node to the last child of the new block node
    tree
      .nodes
      .swap_nodes(&node_id, &block_node_id, SwapBehavior::TakeChildren)?;
  }

  for node_id in nodes_to_unwrap {
    let first_child_id = { tree.nodes.get(&node_id).unwrap().children()[0].clone() };
    // swapping moves the `block` node to be last child of the contained body
    tree
      .nodes
      .swap_nodes(&node_id, &first_child_id, SwapBehavior::TakeChildren)?;
    // then we can safely delete it
    tree
      .nodes
      .remove_node(node_id, RemoveBehavior::DropChildren)?;
    // note to future self: there is a `RemoveBehavior::LiftChildren` but that
    // **does not work!**, it will stick the block body at the _end_ of whatever
    // node previously contained the block
  }

  Ok(())
}


#[cfg(test)]
mod tests {
  use test_helpers::*;

  #[test]
  fn test_implicit_block_wrapping() {
    let tree1 = parse_with_prelude("if: true then: 1 else: 2");
    let tree2 = parse_with_prelude("if: true then: { 1 } else: { 2 }");

    assert_eq!(format!("{}", tree1), format!("{}", tree2));

    let trav1 = tree1
      .nodes
      .traverse_pre_order(tree1.nodes.root_node_id().unwrap())
      .unwrap();
    let trav2 = tree2
      .nodes
      .traverse_pre_order(tree2.nodes.root_node_id().unwrap())
      .unwrap();
    for (node1, node2) in trav1.zip(trav2) {
      let data1 = node1.data();
      let data2 = node2.data();
      assert_eq!(data1.node_type, data2.node_type);
    }
  }

  #[test]
  fn test_redundant_block_unwrapping() {
    let tree1 = parse_with_prelude("if: { true } then: { 1 } else: { 2 }");
    let tree2 = parse_with_prelude("if: true then: { 1 } else: { 2 }");

    assert_eq!(format!("{}", tree1), format!("{}", tree2));

    let trav1 = tree1
      .nodes
      .traverse_pre_order(tree1.nodes.root_node_id().unwrap())
      .unwrap();
    let trav2 = tree2
      .nodes
      .traverse_pre_order(tree2.nodes.root_node_id().unwrap())
      .unwrap();
    for (node1, node2) in trav1.zip(trav2) {
      let data1 = node1.data();
      let data2 = node2.data();
      assert_eq!(data1.node_type, data2.node_type);
    }
  }
}
