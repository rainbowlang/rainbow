use std::fmt::{Display, Error as FmtError, Formatter};
use typing::Type;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Prim {
  Boolean(bool),
  Number(f64),
  String(String),
  Time(u64),
  Money(String, f64),
}

impl Prim {
  pub fn type_of(&self) -> Type {
    match *self {
      Prim::Number(_) => Type::Num,
      Prim::String(_) => Type::Str,
      Prim::Boolean(_) => Type::Bool,
      Prim::Time(_) => Type::Time,
      Prim::Money(_, _) => Type::Money,
    }
  }
}

impl From<bool> for Prim {
  fn from(b: bool) -> Prim {
    Prim::Boolean(b)
  }
}

impl From<f64> for Prim {
  fn from(f: f64) -> Prim {
    Prim::Number(f)
  }
}

impl From<i32> for Prim {
  fn from(i: i32) -> Prim {
    Prim::Number(i.into())
  }
}

impl<'a> From<&'a str> for Prim {
  fn from(s: &str) -> Prim {
    Prim::String(String::from(s))
  }
}

impl From<String> for Prim {
  fn from(s: String) -> Prim {
    Prim::String(s)
  }
}

impl Display for Prim {
  fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
    use self::Prim::*;
    match *self {
      Boolean(v) => write!(f, "{}", v),
      Number(v) => write!(f, "{}", v),
      String(ref v) => write!(f, "{:?}", v),
      Time(v) => write!(f, "{:?}", v),
      Money(ref currency, amount) => write!(f, "{}{}", amount, currency),
    }
  }
}
