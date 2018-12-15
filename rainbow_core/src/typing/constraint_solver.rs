use std::collections::HashMap;

use super::constraint_generator::Constraint;
use super::substitution::{Subst, Substitutable};
use super::types::*;
use super::type_errors::*;

use frontend::NodeData;

pub fn solve(constraints: Vec<Constraint>, errors: &mut Vec<TypeError>) -> Subst {
  let mut subst = HashMap::new();

  if constraints.len() == 0 {
    return subst;
  }

  dbg!("\n\nstarting unification\n\n");

  let mut type_path: Vec<TypeLoc> = Vec::with_capacity(8);
  for Constraint(left, right, location) in constraints {
    let mut u = Unifier {
      errors: errors,
      subst: &mut subst,
      path: &mut type_path,
      left: &left,
      right: &right,
      location: &location,
    };
    u.unify();
  }

  dbg!("\n\nafter unification:");
  for (ref name, ref ty) in subst.iter() {
    dbg!("  {} = {}", name, ty);
  }

  minimize_substitution(subst).unwrap()
}

struct Unifier<'path, 'constraint, 'errs> {
  errors: &'errs mut Vec<TypeError>,
  subst: &'path mut Subst,
  path: &'path mut Vec<TypeLoc>,
  left: &'constraint Type,
  right: &'constraint Type,
  location: &'constraint NodeData,
}

impl<'p, 'c, 'e> Unifier<'p, 'c, 'e> {
  fn add_problem(&mut self, problem: ConstraintProblem) {
    self
      .errors
      .push(Problem::Constraint(self.path.clone(), problem).at(self.location.clone()));
  }

  fn recur(&mut self, loc: TypeLoc, left: &Type, right: &Type) -> Type {
    self.path.push(loc);
    let ty = {
      let child = Unifier {
        left: left,
        right: right,
        path: self.path,
        location: self.location,
        errors: self.errors,
        subst: self.subst,
      };
      child.unify()
    };
    self.path.pop();
    ty
  }

  fn unify(mut self) -> Type {
    use Type::*;
    let left = self.left.apply_substitution(self.subst);
    let right = self.right.apply_substitution(self.subst);

    if left == right {
      return left;
    }

    dbg!("applied substitution");
    dbg!("  from: {} ~ {}", self.left, self.right);
    dbg!("    to: {} ~ {}\n", left, right);

    if left == right {
      return left;
    }

    match (left, right) {
      (ty, Var(name)) => self.bind(name, ty),
      (Var(name), ty) => self.bind(name, ty),
      (List(left_el), List(right_el)) => self.recur(TypeLoc::ListElement, &left_el, &right_el),
      (Record(left_partial, left_fields), Record(right_partial, mut right_fields)) => {
        let mut fields = HashMap::new();

        dbg!("unifying record types:");
        dbg!("  left: {}", Record(left_partial, left_fields.clone()));
        dbg!("  right: {}", Record(right_partial, right_fields.clone()));

        for (name, left_field) in left_fields {
          match right_fields.remove(&name) {
            None => {
              if left_field.required() && !right_partial {
                self.add_problem(ConstraintProblem::FieldMissing(name.clone()));
              } else {
                fields.insert(
                  name,
                  left_field.map_type(|ty| ty.apply_substitution(self.subst)),
                );
              }
            }
            Some(ref right_field) => {
              if left_field.required() && right_field.optional() {
                self.add_problem(ConstraintProblem::FieldOptional(name.clone()));
              }
              let new_ty = {
                self.recur(
                  TypeLoc::Field(name.clone()),
                  left_field.get_type(),
                  right_field.get_type(),
                )
              };
              fields.insert(name, RecordField::new(new_ty, left_field.optional()));
            }
          }
        }

        // right_fields now only contains fields that were *not* in left_fields
        // if left was a partial type, we extend it with the fields from right.
        if left_partial {
          for (name, right_field) in right_fields {
            fields.insert(
              name,
              right_field.map_type(|ty| ty.apply_substitution(self.subst)),
            );
          }
        }

        let merged_type = Record(left_partial, fields);
        dbg!("  result: {}\n", merged_type);
        self.rebind(merged_type)
      }

      (Block(left_in, left_out), Block(right_in, right_out)) => {
        if left_in.len() > right_in.len() {
          self.add_problem(ConstraintProblem::BlockArity {
            expected: right_in.len(),
            actual: left_in.len(),
          });
        }
        let mut inputs: Vec<Type> = Vec::with_capacity(right_in.len());
        for (i, (e_in, a_in)) in left_in.into_iter().zip(right_in.into_iter()).enumerate() {
          let in_ty = self.recur(TypeLoc::BlockArg(i), &a_in, &e_in); // <== block input variance is inverted
          inputs.push(in_ty);
        }
        Block(
          inputs,
          Box::new(self.recur(TypeLoc::BlockBody, &left_out, &right_out)),
        )
      }
      (Any, _) => Any,
      (_, Any) => Any,
      (left, right) => {
        self.add_problem(ConstraintProblem::Incompatible(left.clone(), right));
        left
      }
    }
  }

  fn bind(&mut self, var_name: String, ty: Type) -> Type {
    use ConstraintProblem::*;

    if ty.contains_var(&var_name) {
      self.add_problem(InfiniteType(var_name.clone(), ty.clone()));
      return ty;
    }

    let typ = ty.apply_substitution(self.subst);

    {
      let maybe_exists = { self.subst.get(&var_name).cloned() };
      if let Some(prev_ty) = maybe_exists {
        if typ != prev_ty {
          self.add_problem(AlreadyBound {
            name: var_name,
            old: prev_ty.clone(),
            new: ty.clone(),
          });
          return prev_ty.clone();
        }
      }
    }

    dbg!("bind {} = {}\n", var_name, typ);
    self.subst.insert(var_name, typ.clone());
    typ
  }

  fn rebind(&mut self, new_type: Type) -> Type {
    use ConstraintProblem::*;
    for maybe_var in &[self.left, self.right] {
      if let Some(var_name) = maybe_var.var_name() {
        if new_type.contains_var(var_name) {
          self.add_problem(InfiniteType(var_name.clone(), new_type.clone()));
        } else if !self.subst.contains_key(var_name) {
          self.add_problem(RebindUndefined(var_name.clone()));
        } else {
          self.subst.insert(var_name.clone(), new_type.clone());
        }
      }
    }
    new_type
  }
}


/// Repeatedly replace any `var1 = var2` binding in `subst` with `var1 = subst.get(var2)`
///
/// This _should_ replace all type variables as long as there is some concrete type for var2
fn minimize_substitution(mut subst: Subst) -> Result<Subst, String> {
  fn finalize_record(ty: Type) -> Type {
    match ty {
      Type::Record(true, fields) => Type::Record(
        false,
        fields
          .into_iter()
          .map(|(name, field)| (name, field.map_type(finalize_record)))
          .collect(),
      ),
      Type::List(elem_type) => Type::list_of(finalize_record(*elem_type)),
      other => other,
    }
  }

  loop {
    let mut progress = 0;
    let mut next_subst: Subst = HashMap::new();
    for (type_var, ty) in subst.iter() {
      if let Type::Var(ref other_name) = *ty {
        if let Some(other_type) = subst.get(other_name) {
          if !other_type.contains_var(type_var) {
            progress += 1;
            next_subst.insert(
              type_var.clone(),
              finalize_record(other_type.apply_substitution(&subst)),
            );
            continue;
          } else {
            return Err(format!("infinite type: {} contains {}", other_type, ty));
          }
        }
      }

      let mut next_type = ty.apply_substitution(&subst);
      loop {
        let next_next_type = next_type.apply_substitution(&subst);
        if next_type == next_next_type {
          break;
        }

        next_type = next_next_type;
      }
      next_subst.insert(type_var.clone(), finalize_record(next_type));
    }

    subst = next_subst;

    if progress == 0 {
      break;
    }
  }

  #[cfg(test)]
  {
    dbg!("");
    dbg!("After minimization:");
    for (ref name, ref ty) in subst.iter() {
      dbg!("  {} = {}", name, ty);
    }
  }
  Ok(subst)
}
