use std::fmt::Debug;
use std::iter::FromIterator;
use with_error::WithError;

use super::{Block, Machine};

/// Trait to be implemented by the value representation of a language binding.
///
/// Rainbow is quite benign in terms of what it can do with the data you pass to it:
/// functions never mutate inputs or retain long-lived references.
///
/// Rainbow is run in environments that don't guarantee type-safety, and panics
/// due to a conversion error are **strictly** forbidden, so there is still a
/// lot of interpreter (un)boxing & type-checking when accessing values from Rust.
pub trait Value
  : Sized
  + Debug
  + Clone
  + WithError
  + PartialEq
  + From<bool>
  + From<String>
  + From<u64>
  + From<f64>
  + From<Vec<Self>>
  + FromIterator<Self>
  + FromIterator<(String, Self)>
  + From<Block> {
  type List: List<Self>;
  type Record: Record<Self>;

  fn try_bool(&self) -> Result<bool, Self::Error>;
  fn try_string(&self) -> Result<&str, Self::Error>;
  fn try_number(&self) -> Result<f64, Self::Error>;
  fn try_time(&self) -> Result<u64, Self::Error>;
  fn try_list(&self) -> Result<Self::List, Self::Error>;
  fn try_record(&self) -> Result<Self::Record, Self::Error>;
  fn try_block(&self) -> Result<&Block, Self::Error>;
  fn callable(&self) -> bool {
    self.try_block().is_ok()
  }
  fn try_call(&self, vm: &mut Machine<Self>, args: Vec<Self>) -> Result<Self, Self::Error> {
    self
      .try_block()
      .and_then(|block| vm.eval_block(block, args))
  }
}

/// List operation trait.
pub trait List<V: Value>: IntoIterator<Item = V> + Debug {
  fn len(&self) -> usize;
  fn at(&self, idx: usize) -> Option<V>;
}

/// Operations permissible on records
pub trait Record<V: Value>: IntoIterator<Item = (String, V)> + Debug {
  fn at(&self, key: &str) -> Option<V>;
}
