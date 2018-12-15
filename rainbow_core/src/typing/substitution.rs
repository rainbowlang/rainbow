use super::types::*;
use std::collections::{HashMap, HashSet};

/// TV is the name part of a type variable
type TV = String;

/// A map from type variables to types.
pub type Subst = HashMap<TV, Type>;

/// Trait for types that can apply substitutions to themselves.
///
/// Types implementing the `Substitutable` trait (defined below) _apply_ substitutions by replacing
/// any type variables they contain with the value of `substitution.get(var_name).or(original_var)`.
pub trait Substitutable {
    fn apply_substitution(&self, subs: &Subst) -> Self;
    fn free_vars(&self) -> Option<HashSet<TV>>;

    /// Check if `var` occurs somewhere inside this substitutable.
    fn contains_var(&self, var: &TV) -> bool {
        self.free_vars()
            .map(|set| set.contains(var))
            .unwrap_or(false)
    }
}

/// extend_vars merges two optional sets of type variable names
pub fn extend_vars<T: Substitutable>(vars: Option<HashSet<TV>>, t: &T) -> Option<HashSet<TV>> {
    match (vars, t.free_vars()) {
        (Some(mut vars), Some(more)) => {
            vars.extend(more);
            Some(vars)
        }
        (None, None) => None,
        (some, None) => some,
        (None, some) => some,
    }
}
