use super::types::Type;
use frontend::NodeData;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeError {
  location: NodeData,
  error: Problem,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Problem {
  UnknownFunction,
  UnknownKeyword(String),
  Constraint(Vec<TypeLoc>, ConstraintProblem),
}

impl Problem {
  pub fn at(self, location: NodeData) -> TypeError {
    TypeError {
      location: location,
      error: self,
    }
  }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConstraintProblem {
  AlreadyBound { name: String, old: Type, new: Type },
  InfiniteType(String, Type),
  RebindUndefined(String),
  Incompatible(Type, Type),
  BlockArity { expected: usize, actual: usize },
  FieldMissing(String),
  FieldOptional(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Loc {
  Element(usize),
  Field(usize),
  BlockBody,
  ArgName(usize),
  ArgValue(usize),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TypeLoc {
  ListElement,
  Field(String),
  BlockArg(usize),
  BlockBody,
}
