use std::rc::Rc;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use std::iter::FromIterator;
use scope::Scope;
use super::substitution::*;
use super::types::Type;

/// This is the container for state as we walk the AST and infer types. It's mostly a wrapper around
/// a `Scope<Scheme>`, but also tracks the set of undefined variable names.
#[derive(Debug, Clone)]
pub struct TypeEnv {
  schemes: Scope<Scheme>,
  // the set of undefined names (and their schemes!) is always at the top-level
  undefined: Rc<RefCell<HashSet<String>>>,
}

impl TypeEnv {
  #[allow(dead_code)] // used by tests
  pub fn empty() -> TypeEnv {
    TypeEnv {
      schemes: Scope::new(),
      undefined: Rc::new(RefCell::new(HashSet::new())),
    }
  }

  pub fn child(&self) -> TypeEnv {
    TypeEnv {
      schemes: self.schemes.new_child(),
      undefined: self.undefined.clone(),
    }
  }

  pub fn explicitly_define(&mut self, name: String, ty: Type) {
    self.schemes.insert(name, Scheme::new(ty));
  }

  pub fn contains_key(&self, name: &String) -> bool {
    self.schemes.get(name).is_some()
  }

  /// get the scheme for the given name, or instantiate a new scheme with a var from `fresh_vars`.
  ///
  /// adds `name` to the `self.undefined` set if there was no pre-existing scheme.
  pub fn get_or_let_fresh<Fresh>(&mut self, name: &String, fresh_vars: &mut Fresh) -> Rc<Scheme>
  where
    Fresh: Iterator<Item = Type>,
  {
    self.schemes.get(name).unwrap_or_else(|| {
      self.undefined.borrow_mut().insert(name.clone());
      self
        .schemes
        .insert_at_root(name.clone(), Scheme::new(fresh_vars.next().unwrap()));
      self.schemes.get(name).unwrap()
    })
  }
}

impl FromIterator<(String, Type)> for TypeEnv {
  fn from_iter<I>(pairs: I) -> TypeEnv
  where
    I: IntoIterator<Item = (String, Type)>,
  {
    TypeEnv {
      schemes: pairs
        .into_iter()
        .map(|(name, ty)| (name, Scheme::new(ty)))
        .collect(),
      undefined: Rc::new(RefCell::new(HashSet::new())),
    }
  }
}

impl From<Vec<(String, Type)>> for TypeEnv {
  fn from(pairs: Vec<(String, Type)>) -> TypeEnv {
    pairs.into_iter().collect()
  }
}

impl Into<HashMap<String, Type>> for TypeEnv {
  fn into(self) -> HashMap<String, Type> {
    let schemes = self.schemes.flatten();
    HashMap::from_iter(
      schemes
        .into_iter()
        .map(|(name, scheme)| (name, scheme.ty.clone())),
    )
  }
}

impl Substitutable for Scope<Scheme> {
  fn apply_substitution(&self, subs: &Subst) -> Self {
    self.map_clone(|thing| thing.apply_substitution(subs))
  }

  fn free_vars(&self) -> Option<HashSet<String>> {
    self
      .flatten()
      .into_iter()
      .fold(None, |vars, (_name, thing)| {
        extend_vars(vars, thing.as_ref())
      })
  }
}

/// A type scheme models a polymorphic type. The simplest example is an identity block `{ x => x }`,
/// which for any type `A`, has the type `{ A => A }`, or a constant block `{ x => y }` which has
/// the type `{ A => Y }` (where `Y` is the type of the variable `y`, defined in some outer scope).
#[derive(Debug, Clone)]
pub struct Scheme {
  vars: HashSet<String>,
  ty: Type,
}

impl Scheme {
  /// Create a new empty type scheme with the given type.
  fn new(ty: Type) -> Scheme {
    Scheme {
      vars: HashSet::new(),
      ty: ty,
    }
  }

  /// Generalize a type scheme by closing over all free type variables.
  ///
  /// Why this dead code is here: it was implemented in the "Write You a Haskell" code, but only
  /// used for let bindings, which Rainbow doesn't have. I am leaving it here in case:
  ///
  ///   1. There's a bug in the rest of the code here and I've missed a place where I should be
  ///      be generalizing a type scheme.
  ///
  ///   2. There is a need for an explicit `let` binding in Rainbow. (Currently the prelude defines a
  ///      function `with: { ... } do: { x => ... }`, which serves much the same purpose).
  #[allow(dead_code)]
  fn generalize(env: &TypeEnv, ty: Type) -> Scheme {
    let vars = match (ty.free_vars(), env.free_vars()) {
      (None, _) => HashSet::new(),
      (Some(lft), None) => lft,
      (Some(lft), Some(rgt)) => lft.difference(&rgt).cloned().collect(),
    };
    Scheme { ty: ty, vars: vars }
  }

  /// Instantiate a scheme by creating fresh type variables for every variable in `self.vars` that
  /// does *not* appear in the current `TypeEnv`.
  pub fn instantiate<Fresh: Iterator<Item = Type>>(&self, fresh: &mut Fresh) -> Type {
    // build a new substitution containing fresh vars for every var in `self.vars`
    let subs: Subst = self.vars.iter().cloned().zip(fresh).collect();
    // apply that substitution to `self.ty`. This
    self.ty.apply_substitution(&subs)
  }
}

impl Substitutable for Scheme {
  fn apply_substitution(&self, subs: &Subst) -> Self {
    let subs2: Subst = subs
      .iter()
      .filter_map(|(var, t)| {
        if self.vars.contains(var) {
          None
        } else {
          Some((var.clone(), t.clone()))
        }
      })
      .collect();
    Scheme {
      vars: self.vars.clone(),
      ty: self.ty.apply_substitution(&subs2),
    }
  }

  fn free_vars(&self) -> Option<HashSet<String>> {
    self
      .ty
      .free_vars()
      .map(|vars| vars.difference(&self.vars).cloned().collect())
  }
}

impl Substitutable for TypeEnv {
  fn apply_substitution(&self, subs: &Subst) -> Self {
    TypeEnv {
      schemes: self.schemes.apply_substitution(subs),
      undefined: self.undefined.clone(),
    }
  }

  fn free_vars(&self) -> Option<HashSet<String>> {
    self.schemes.free_vars()
  }
}
