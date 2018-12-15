use std::fmt::Debug;
use crate::with_error::WithError;


#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Apply<V: Debug + PartialEq + Clone, K = u16> {
  args: Vec<(K, V)>,
}

impl<V: Debug + PartialEq + Clone, K> From<Vec<(K, V)>> for Apply<V, K> {
  fn from(args: Vec<(K, V)>) -> Self {
    Apply { args: args }
  }
}

impl<V: Debug + PartialEq + Clone + WithError, K: 'static> Apply<V, K> {
  pub fn new() -> Self {
    Apply { args: Vec::new() }
  }

  pub fn with_capacity(capacity: usize) -> Self {
    Apply {
      args: Vec::with_capacity(capacity),
    }
  }

  #[inline]
  pub fn func_id(&self) -> &K {
    &self.args[0].0
  }

  pub fn get<U: ?Sized>(&self, name: &U) -> Option<&V>
  where
    K: ::std::borrow::Borrow<U>,
    U: PartialEq<K>,
  {
    for &(ref xname, ref x) in self.args.iter() {
      if name == xname {
        return Some(x);
      }
    }
    None
  }

  pub fn demand<U: ?Sized>(&self, name: &U) -> Result<&V, V::Error>
  where
    K: ::std::borrow::Borrow<U>,
    U: PartialEq<K> + Debug,
  {
    self
      .get(name)
      .ok_or_else(|| V::Error::from(format!("Missing required argument {:?}", name)))
  }

  pub fn all<U: PartialEq<K>>(&self, name: U) -> Vec<&V> {
    let mut result: Vec<&V> = Vec::with_capacity(self.args.len());
    for &(ref xname, ref x) in self.args.iter() {
      if name == *xname {
        result.push(x)
      }
    }
    result
  }

  pub fn iter(&self) -> ::std::slice::Iter<(K, V)> {
    self.args.iter()
  }

  pub fn into_iter(self) -> ::std::vec::IntoIter<(K, V)> {
    self.args.into_iter()
  }

  pub fn len(&self) -> usize {
    self.args.len()
  }
}
