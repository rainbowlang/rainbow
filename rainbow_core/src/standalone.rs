use std::collections::HashMap;
use std::fmt::{Display, Error as FmtError, Formatter};
use std::iter::FromIterator;

use crate::interpreter::{Block, List as IList, Record as IRecord, Value as IValue};
use crate::primitive::Prim;
use crate::typing::Type;
use crate::with_error::WithError;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Prim(Prim),
    List(Vec<Value>),
    Record(HashMap<String, Value>),
    Block(Block),
}

impl Value {
    pub fn type_of(&self) -> Type {
        use crate::primitive::Prim;
        match *self {
            Value::Prim(Prim::Number(_)) => Type::Num,
            Value::Prim(Prim::String(_)) => Type::Str,
            Value::Prim(Prim::Boolean(_)) => Type::Bool,
            Value::Prim(Prim::Time(_)) => Type::Time,
            Value::Prim(Prim::Money(_, _)) => Type::Money,
            Value::List(ref items) => {
                if items.len() == 0 {
                    Type::list_of(Type::Any)
                } else {
                    Type::list_of(items[0].type_of())
                }
            }
            Value::Record(ref fields) => {
                Type::record_from_iter(fields.iter().map(|(k, v)| (k.clone(), v.type_of())))
            }
            Value::Block(_) => Type::block_from_to(vec![], Type::Any),
        }
    }
}

impl Default for Value {
    fn default() -> Self {
        Value::from(false)
    }
}

impl WithError for Value {
    type Error = String;
}

impl IValue for Value {
    type List = Vec<Value>;
    type Record = HashMap<String, Value>;

    fn try_bool(&self) -> Result<bool, String> {
        match *self {
            Value::Prim(Prim::Boolean(b)) => Ok(b),
            _ => Err(format!("{} is not a boolean", self)),
        }
    }

    fn try_number(&self) -> Result<f64, String> {
        match *self {
            Value::Prim(Prim::Number(f)) => Ok(f),
            _ => Err(format!("{} is not a number", self)),
        }
    }

    fn try_string(&self) -> Result<&str, String> {
        match *self {
            Value::Prim(Prim::String(ref s)) => Ok(s),
            _ => Err(format!("{} is not a string", self)),
        }
    }

    fn try_time(&self) -> Result<u64, String> {
        match *self {
            Value::Prim(Prim::Time(t)) => Ok(t),
            _ => Err(format!("{} is not a time", self)),
        }
    }

    /*
    fn try_money(&self) -> Result<(String, f64), String> {
        match *self {
            Value::Prim(Prim::Money(ref currency, amount)) => Ok((currency.clone(), amount)),
            _ => Err(format!("{} is not a money", self))
        }
    }
    */

    fn try_list(&self) -> Result<Self::List, String> {
        match *self {
            Value::List(ref list) => Ok(list.clone()),
            _ => Err(format!("{} is not a list", self)),
        }
    }

    fn try_record(&self) -> Result<Self::Record, String> {
        match *self {
            Value::Record(ref map) => Ok(map.clone()),
            _ => Err(format!("{} is not a list", self)),
        }
    }

    fn try_block(&self) -> Result<&Block, String> {
        match *self {
            Value::Block(ref block) => Ok(block),
            _ => Err(format!("{} is not a block", self)),
        }
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Value {
        Value::Prim(Prim::Boolean(b))
    }
}

impl From<String> for Value {
    fn from(s: String) -> Value {
        Value::Prim(Prim::String(s))
    }
}

impl<'a> From<&'a str> for Value {
    fn from(s: &str) -> Value {
        Value::Prim(Prim::String(s.into()))
    }
}

impl From<u64> for Value {
    fn from(i: u64) -> Value {
        Value::Prim(Prim::Time(i))
    }
}

impl From<f64> for Value {
    fn from(f: f64) -> Value {
        Value::Prim(Prim::Number(f))
    }
}

impl From<Vec<Value>> for Value {
    fn from(values: Vec<Value>) -> Value {
        Value::List(values)
    }
}

impl From<Block> for Value {
    fn from(block: Block) -> Value {
        Value::Block(block)
    }
}

impl FromIterator<Value> for Value {
    fn from_iter<I>(thing: I) -> Value
    where
        I: IntoIterator<Item = Value>,
    {
        Value::from(thing.into_iter().collect::<Vec<_>>())
    }
}

impl FromIterator<(String, Value)> for Value {
    fn from_iter<I>(thing: I) -> Value
    where
        I: IntoIterator<Item = (String, Value)>,
    {
        Value::Record(thing.into_iter().collect::<HashMap<_, _>>())
    }
}

/*
impl<'ns, T: Into<Prim>> From<T> for Value {
    /// anything that can be converted to a `Prim` can also be converted to a `Value`.
    fn from(x: T) -> Value<'static> {
        Value::Prim(x.into())
    }
}
*/

impl Display for Value {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        use self::Value::*;
        match *self {
            Prim(ref p) => write!(f, "{}", p),
            List(ref items) => {
                write!(f, "[ ")?;
                for item in items.into_iter() {
                    write!(f, "{} ", item)?;
                }
                write!(f, "]")
            }
            Record(ref pairs) => {
                use std::fmt::Write;
                f.write_str("[ ")?;
                for (name, stx) in pairs.iter() {
                    write!(f, "{} = {} ", name, stx)?;
                }
                f.write_char(']')
            }
            Block(ref block) => write!(f, "{:?}", block),
        }
    }
}

impl IList<Value> for Vec<Value> {
    fn len(&self) -> usize {
        Vec::len(self)
    }

    fn at(&self, idx: usize) -> Option<Value> {
        self.as_slice().get(idx).cloned()
    }
}

impl IRecord<Value> for HashMap<String, Value> {
    fn at(&self, key: &str) -> Option<Value> {
        self.get(key).cloned()
    }
}
