use std::fmt;
use pest::Error as PestError;
use id_tree::NodeIdError;
use crate::frontend::grammar::Rule;

#[derive(Debug)]
pub enum ParseError<'i> {
  Pest(PestError<'i, Rule>),
  NodeId(NodeIdError),
}

impl<'i> From<PestError<'i, Rule>> for ParseError<'i> {
  fn from(error: PestError<'i, Rule>) -> Self {
    ParseError::Pest(error)
  }
}

impl<'i> From<NodeIdError> for ParseError<'i> {
  fn from(error: NodeIdError) -> Self {
    ParseError::NodeId(error)
  }
}

impl<'i> fmt::Display for ParseError<'i> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match *self {
      ParseError::Pest(ref err) => write!(f, "{}", err),
      ParseError::NodeId(ref err) => write!(f, "internal parser error {:?}", err),
    }
  }
}
