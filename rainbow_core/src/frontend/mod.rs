mod lexer;
mod parse_error;
mod grammar;
mod syntax_tree;
mod implicit_blocks;

use pest;

pub use self::parse_error::*;
pub use self::grammar::*;
pub use self::syntax_tree::*;
pub use id_tree::NodeId;

use crate::namespace::INamespace;

pub fn parse<'i, NS: INamespace>(
  namespace: &NS,
  rule: Rule,
  input: &'i str,
) -> Result<SyntaxTree<'i>, ParseError<'i>> {
  use pest::Parser;

  let mut pairs = RainbowGrammar::parse(rule, input)?;

  if let Some(pair) = pairs.next() {
    if pair.as_str().len() != input.len() {
      Err(
        pest::Error::CustomErrorPos {
          message: "extra input".into(),
          pos: pair.into_span().end_pos(),
        }.into(),
      )
    } else {
      let mut tree = SyntaxTree::from_input_and_pair(namespace.symbols(), input, pair)?;
      implicit_blocks::rewrite(namespace, &mut tree)?;
      Ok(tree)
    }
  } else {
    Err(
      pest::Error::CustomErrorPos {
        message: "no input".into(),
        pos: pest::Position::from_start(input).at_start().unwrap(),
      }.into(),
    )
  }
}

/*

pub fn parse_loose<'i, NS: INamespace>(
  namespace: &NS,
  rule: Rule,
  input: &'i mut String,
) -> Result<(usize, SyntaxTree<'i>), ParseError<'i>> {
  use pest;
  use pest::Parser;

  if let Some(pair) = pairs.next() {
    let parsed_len = { pair.as_str().len() };
    let mut tree = SyntaxTree::from_input_and_pair(namespace.symbols(), input, pair)?;
    implicit_blocks::rewrite(namespace, &mut tree)?;
    Ok((parsed_len, tree))
  } else {
    Err(
      pest::Error::ParsingError {
        positives: vec![Rule::variable, Rule::apply],
        negatives: vec![],
        pos: pest::Position::from_start(input).at_start().unwrap(),
      }.into(),
    )
  }
}

fn get_pairs(
  rule: Rule,
  input: &'i mut String,
  max_errors: usize,
) -> Result<pest::iterators::Pairs<'i, Rule>, pest::Error<'i, Rule>> {
  let mut result = RainbowGrammar::parse(rule, input);
  for n in (0..max_errors) {
    match result {
      Err(pest::Error::ParsingError {
        pos,
        positives,
        negatives,
      }) => {
        if positives.is_empty() {
          break;
        }
        if positives.iter().any(|rule| rule == Rule::variable) {
          // a variable would match here, generate one and continue parsing
          let var_name = format!("parse_error____{}", n);
          input.insert_str(pos.pos(), &var_name);
        }
      }
      something_else => break,
    }
    result = RainbowGrammar::parse(rule, input);
  }
  result
}

*/
