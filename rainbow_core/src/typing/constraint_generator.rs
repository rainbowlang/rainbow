use namespace::INamespace;
use std::collections::HashMap;

use frontend::{NodeData, SyntaxTree};
use id_tree::NodeId;

use super::type_errors::*;
use super::types::*;
use super::type_env::TypeEnv;
use super::substitution::*;


#[derive(Debug, PartialEq, Clone)]
pub struct Constraint(pub Type, pub Type, pub NodeData);

pub fn generate<NS>(
  ns: &NS,
  type_env: &mut TypeEnv,
  tree: &SyntaxTree,
) -> (Type, Vec<Constraint>, Vec<TypeError>)
where
  NS: INamespace,
{
  let mut generator = ConstraintGenerator::new(ns, tree);
  let root_node_id = tree.nodes.root_node_id().unwrap();
  let inferred_type = generator.recur(type_env, root_node_id);
  generator.sort_constraints();
  (inferred_type, generator.constraints, generator.errors)
}

struct ConstraintGenerator<'a, 'i, NS: INamespace + 'a> {
  functions: &'a NS,
  tree: &'i SyntaxTree<'i>,
  fresh_vars: FreshVarSupply,
  inside_try: bool,
  constraints: Vec<Constraint>,
  errors: Vec<TypeError>,
}

impl<'a, 'i, NS: INamespace> ConstraintGenerator<'a, 'i, NS> {
  fn new(functions: &'a NS, tree: &'i SyntaxTree) -> Self {
    ConstraintGenerator {
      functions: functions,
      tree: tree,
      fresh_vars: FreshVarSupply { count: 0 },
      inside_try: false,
      constraints: Vec::with_capacity(1024),
      errors: Vec::with_capacity(100),
    }
  }

  fn add_constraint(&mut self, node_data: NodeData, ty1: Type, ty2: Type) {
    self.constraints.push(Constraint(ty1, ty2, node_data));
  }

  fn add_constraint_at(&mut self, node_id: &NodeId, ty1: Type, ty2: Type) {
    self.add_constraint(
      self.tree.nodes.get(node_id).unwrap().data().clone(),
      ty1,
      ty2,
    )
  }

  fn recur(&mut self, type_env: &mut TypeEnv, node_id: &NodeId) -> Type /* Result<Type, NodeIdError> */
  {
    use frontend::NodeType::*;
    let node = self.tree.nodes.get(node_id).unwrap();
    let data = node.data();
    dbg!("infer {:?}", node.data());
    match data.node_type {
      Root => node
        .children()
        .into_iter()
        .fold(Type::Any, |_, child_id| self.recur(type_env, child_id)),
      Primitive(id) => self.tree.lookup_constant(id).type_of(),
      List => {
        let elem_type_var = self.fresh_vars.next().unwrap();
        for elem_id in node.children() {
          let elem_ty = self.recur(type_env, elem_id);
          self.add_constraint_at(elem_id, elem_type_var.clone(), elem_ty);
        }
        Type::list_of(elem_type_var)
      }

      Record => {
        let children = node.children();
        let mut field_types: HashMap<String, RecordField> =
          HashMap::with_capacity(children.len() / 2);
        for entry_id in children {
          // each child here is a RecordEntry
          let name_and_value_ids: Vec<_> =
            self.tree.nodes.children_ids(entry_id).unwrap().collect();
          let field_type = self.recur(type_env, &name_and_value_ids[1]);
          // todo: check field_name_node.data().node_type !== Ident
          let field_name = self.tree.node_id_str(&name_and_value_ids[0]).unwrap();
          field_types.insert(
            String::from(field_name),
            RecordField::new(field_type, false),
          );
        }
        Type::record_from_map(field_types)
      }

      Variable => {
        let children = node.children();
        let root_name = self.tree.node_id_str(&children[0]).unwrap();
        let scheme = type_env.get_or_let_fresh(&String::from(root_name), &mut self.fresh_vars);
        let root_ty = scheme.instantiate(&mut self.fresh_vars);

        if children.len() == 1 {
          return root_ty;
        }

        let leaf_type = self.fresh_vars.next().unwrap();

        let root_var = children[1..]
          .iter()
          .rev()
          .fold(leaf_type.clone(), |field_var, child_id| {
            let next_var = self.fresh_vars.next().unwrap();
            let path_segment_text = self.tree.node_id_str(child_id).unwrap();
            let record_ty =
              Type::record_with_one_field(path_segment_text, field_var.clone(), self.inside_try);

            // create a NodeData that covers the entire variable path up to and including this segment
            let subpath_node_data = NodeData {
              node_type: Variable,
              start_pos: data.start_pos,
              end_pos: self.tree.nodes.get(child_id).unwrap().data().end_pos,
            };
            self.add_constraint(subpath_node_data, next_var.clone(), record_ty);
            next_var
          });

        self.add_constraint(data.clone(), root_ty, root_var);
        leaf_type
      }

      Block => {
        let children = node.children();
        let (in_types, out_type) = if children.len() > 1 {
          let arg_node_ids = self.tree.nodes.get(&children[0]).unwrap().children();
          let mut in_types = Vec::with_capacity(arg_node_ids.len());
          let mut local_env = type_env.child();
          for (id, ty) in arg_node_ids.into_iter().zip(&mut self.fresh_vars) {
            in_types.push(ty.clone());
            local_env.explicitly_define(String::from(self.tree.node_id_str(id).unwrap()), ty);
          }
          (in_types, self.recur(&mut local_env, &children[1]))
        } else {
          (vec![], self.recur(type_env, &children[0]))
        };
        Type::block_from_to(in_types, out_type)
      }

      Apply => {
        let children = node.children();
        debug_assert!(children.len() > 0);

        let arg0 = self.tree.nodes.get(&children[0]).unwrap();
        let func_name = self
          .tree
          .node_id_str(&arg0.children()[0])
          .unwrap()
          .trim_right_matches(':');
        let sig = match self.functions.get_signature(func_name) {
          None => {
            self.errors.push(
              Problem::UnknownFunction
                .at(self.tree.nodes.get(&children[0]).unwrap().data().clone()),
            );
            return Type::Any;
          }
          Some(s) => s,
        };

        // create a local substitution for any type variables in the signature
        let sig_subst: Option<Subst> = sig
          .args()
          .fold(None, |vars, arg| extend_vars(vars, &arg.ty))
          .map(|vars| vars.into_iter().zip(&mut self.fresh_vars).collect());

        for child_id in children {
          let arg_children = self.tree.nodes.get(&child_id).unwrap().children();
          let kw_node_data = self.tree.node_data(&arg_children[0]).unwrap();
          let kw_sym_id = if let Keyword(kw_id) = kw_node_data.node_type {
            Some(kw_id)
          } else {
            None
          };
          let arg_ty = match kw_sym_id.and_then(|id| sig.arg(id)) {
            Some(spec) => spec.ty.clone(),
            None => {
              let node_data = self.tree.node_data(&arg_children[0]).unwrap().clone();
              self
                .errors
                .push(Problem::UnknownKeyword(func_name.into()).at(node_data));
              Type::Any
            }
          };

          let arg_type = match sig_subst {
            Some(ref s) => arg_ty.apply_substitution(s),
            None => arg_ty.clone(),
          };

          let stx_type = { self.recur(type_env, &arg_children[1]) };
          self.add_constraint_at(&arg_children[1], stx_type, arg_type);
        }

        let out = self.fresh_vars.next().unwrap();

        let return_type = match sig_subst {
          Some(ref s) => sig.returns().apply_substitution(s),
          None => sig.returns().clone(),
        };

        self.add_constraint(data.clone(), out.clone(), return_type);
        out
      }
      // the constraint generator should never visit other node types
      _ => Type::Never,
    }
  }

  pub fn sort_constraints(&mut self) {
    self
      .constraints
      .sort_by_key(|&Constraint(ref lft, ref rgt, _)| {
        vec![lft, rgt].into_iter().fold(0, |count, ty| {
          count + ty.free_vars().map(|vars| vars.len()).unwrap_or(0)
        })
      })
  }
}

struct FreshVarSupply {
  count: usize,
}

impl Iterator for FreshVarSupply {
  type Item = Type;

  fn next(&mut self) -> Option<Type> {
    // let letter = (((self.count % 26) as u8) + ('a' as u8)) as char;
    // let number = self.count / 26;
    self.count += 1;
    Some(Type::Var(format!("${}", self.count)))
  }
}
