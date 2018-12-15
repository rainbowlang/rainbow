use std::hash::Hash;
use std::fmt::{Display, Error as FmtError, Formatter};
use crate::arena::ArenaId;

use crate::typing::Type;

/// Signature defines the types of inputs/outputs to a function.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Signature<Id: Clone + Hash + Eq = ArenaId> {
  args: Vec<Argument<Id>>,
  return_type: Type,
  // total functions guarantee that they will return a value
  total: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Argument<Id: Clone + Hash + Eq = ArenaId> {
  pub name: Id,
  pub ty: Type,
  pub variadic: bool,
  pub required: bool,
}

impl<Id: Clone + Hash + Eq> Signature<Id> {
  pub fn with_capacity(capacity: usize) -> Signature<Id> {
    Signature {
      args: Vec::with_capacity(capacity),
      return_type: Type::Never,
      total: false,
    }
  }

  pub fn add_argument(&mut self, arg: Argument<Id>) {
    self.args.push(arg);
  }

  pub fn set_total(&mut self, total: bool) {
    self.total = total;
  }

  pub fn set_return_type(&mut self, ty: Type) {
    self.return_type = ty;
  }

  #[inline]
  pub fn name(&self) -> Id {
    self.args[0].name.clone()
  }

  #[inline]
  pub fn returns(&self) -> &Type {
    &self.return_type
  }

  #[inline]
  pub fn is_total(&self) -> bool {
    self.total
  }

  pub fn arg(&self, name: Id) -> Option<&Argument<Id>> {
    self.args.iter().filter(|spec| spec.name == name).next()
  }

  #[inline]
  pub fn args(&self) -> ::std::slice::Iter<Argument<Id>> {
    self.args.iter()
  }
}

impl<Id: Clone + Hash + Eq + Display> Display for Signature<Id> {
  fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
    use std::fmt::Write;
    let mut first = true;
    for arg in self.args.iter() {
      if first {
        first = false;
      } else {
        f.write_char(' ')?;
      }
      if arg.variadic {
        f.write_char('[')?;
      }
      write!(f, "{}", arg.name)?;
      if arg.variadic {
        f.write_char(']')?;
      }
      if !arg.required {
        f.write_char('?')?;
      }
      f.write_str(": ")?;
      write!(f, "{}", arg.ty)?;
    }
    write!(f, " :: {}", self.return_type)
  }
}
