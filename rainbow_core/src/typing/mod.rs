//! The Rainbow type system.
//!
//! ## Typing rules
//!
//! Because Rainbow does not aspire to be a general purpose language, it's
//! type-system is somewhat necessarily lowest-common-denominator. The capabilities
//! and rules of types are biased towards not requiring much thought from
//! authors, and providing control to language bindings/integrators.
//!
//! ### Primitives
//!
//! Primitive types only satisfy themselves.
//!
//! The primitive types are:
//!
//! * `string`
//! * `number`
//! * `boolean`
//! * `time`
//!
//! ### Lists
//!
//! Lists are homogeneously typed. A list type `Left` is satisfied by another list
//! type `Right` iff the element type of `Left` is satisfied by the element type
//! of `Right`.
//!
//! The type of a list containing elements of type `E` is written `[ E ]`
//!
//! ### Records
//!
//! Records contain a fixed set of field identifiers that map to types. A given
//! field may be optional.
//!
//! A record type `Left` is satisfied by another record type `Right` if every
//! non-optional field in `Left` is present (and non-optional) in `Right` and has
//! the same type.
//!
//! The type of a record with a required field `foo` of type `F` and optional
//! field `bar` of type `B` is written `[ foo = F bar = B? ]`.
//!
//! ### Blocks
//!
//! Blocks are typed by a (possibly empty) list of input types and an output type.
//! A block type `Left` is satisfied by another block type `Right` iff:
//!
//! 1. `Right` expects _at most_ as many inputs as `Left`.
//! 2. Each input type of `Left` is satisfied by corresponding input type of
//! `Right`.
//! 3. The output type of `Left` is satisfied by the output type of `Right`.
//!
//! The type of a block taking arguments of types `A` and `B` and returns a value
//! of type `C` is written `{ A B => C }`.
//!
//! The type of a block taking no arguments and returning type `T` is written `{ T
//! }`
//!
//! ### Functions
//!
//! Functions are typed by a set of identifiers mapping to input types and their
//! output type. Any one of these identifiers may be variadic, and any but the
//! first (the function name) may be optional. Because Rainbow can only call
//! functions (it is not possible to define new functions in Rainbow, or pass
//! functions as values), there is no concept of satisfiability for a function
//! types.
//!
//! _(The below notation is subject to change)_
//!
//! However, it's still useful to be able to write down the type of a function for
//! documentation. The type of a function named `foo` taking a `foo` argument of
//! type `F`, a variadic number of arguments named `bar` of type `B`, an optional
//! argument `baz` of type `Z`,  and returning type `C` would therefore be written
//! `foo:F [bar]:B baz:?Z => C`.
//!
//! A more useful example is the type of `if`, which is written as follows:
//!
//! ```rainbow,ignore
//! {
//!     if: boolean
//!     [and]: { boolean }
//!     [or]: { boolean }
//!     then: { A }
//!     else: { A }
//! }
//! ```
//!
//!
pub mod types;
mod type_errors;
mod substitution;
mod type_env;
mod constraint_generator;
mod constraint_solver;

#[cfg(test)]
mod tests;

pub use self::types::*;
pub use self::type_errors::*;

use std::collections::HashMap;

use frontend::SyntaxTree;
use namespace::INamespace;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeCheckerResult {
  pub inputs: HashMap<String, Type>,
  pub output: Type,
  pub errors: Vec<TypeError>,
}

impl TypeCheckerResult {
  pub fn unwrap(self) -> (Type, HashMap<String, Type>) {
    if self.errors.is_empty() {
      return (self.output, self.inputs);
    }
    panic!("Unwrap called on {:?}", self)
  }
}

/// Determine the type of an expression given a `Namespace` and iterator of global variable types.
pub fn type_of<NS, G>(ns: &NS, globals: G, tree: &SyntaxTree) -> TypeCheckerResult
where
  NS: INamespace,
  G: IntoIterator<Item = (String, Type)>,
{
  use self::type_env::TypeEnv;
  use self::constraint_generator::generate;
  use self::constraint_solver::solve;
  use self::substitution::Substitutable;

  let mut initial_env: TypeEnv = globals.into_iter().collect();
  let (inferred_type, constraints, mut errors) = generate(ns, &mut initial_env, tree);

  #[cfg(test)]
  {
    use self::constraint_generator::Constraint;
    dbg!("constraints:");
    for &Constraint(ref lft, ref rgt, _) in constraints.iter() {
      dbg!("  {} ~ {}", lft, rgt);
    }
  }

  let subst = solve(constraints, &mut errors);

  let mut inferred_globals: HashMap<_, Type> = initial_env.apply_substitution(&subst).into();
  inferred_globals.retain(|k, _v| initial_env.contains_key(k));

  TypeCheckerResult {
    inputs: inferred_globals,
    output: inferred_type.apply_substitution(&subst),
    errors: errors,
  }
}
