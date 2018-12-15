/**
 * This lexer is *not* currently used. I think I wanted to make a hand-written
 * parser for better error messages but I really don't remember why now.
 */
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TokenKind {
  Colon,
  Dot,
  LCurly,
  RCurly,
  LSquare,
  RSquare,
  Arrow,
  Equals,
  Ident,
  Bool,
  String,
  Number,
  UnclosedString,
  Unknown,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Token<'i> {
  pub input: &'i str,
  pub kind: TokenKind,
  pub start_pos: usize,
  pub end_pos: usize,
}

pub struct Lexer<'i> {
  input: &'i str,
  current: Option<char>,
  position: usize,
}

pub fn lex<'i>(input: &'i str) -> Lexer<'i> {
  Lexer {
    input: input,
    current: None,
    position: 0,
  }
}

impl<'i> Iterator for Lexer<'i> {
  type Item = Token<'i>;

  fn next(&mut self) -> Option<Token<'i>> {
    self.seek(|c| !c.is_whitespace());
    if self.current == None {
      return None;
    }
    Some(match self.current.unwrap() {
      '"' => match self.distance_to_next(|c| c == '"') {
        Some(len) => self.token(TokenKind::String, Some(len + 1)),
        None => self.token(TokenKind::UnclosedString, None),
      },
      '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' => {
        match self.distance_to_next(|c| !c.is_digit(10)) {
          Some(len) => match self.input[(self.position + len)..].chars().next() {
            Some('.') | Some(',') => {
              println!("Found dot at {}", len + self.position);
              let start = self.position;
              self.position += len;
              let full_len = self
                .distance_to_next(|c| !c.is_digit(10))
                .map(|len2| len2 + len);
              self.position = start;
              self.token(TokenKind::Number, full_len)
            }
            _ => self.token(TokenKind::Number, Some(len)),
          },
          None => self.token(TokenKind::Number, None),
        }
      }
      '{' => self.token(TokenKind::LCurly, Some(1)),
      '}' => self.token(TokenKind::RCurly, Some(1)),
      '[' => self.token(TokenKind::LSquare, Some(1)),
      ']' => self.token(TokenKind::RSquare, Some(1)),
      ':' => self.token(TokenKind::Colon, Some(1)),
      '.' => self.token(TokenKind::Dot, Some(1)),
      '=' => {
        if let Some('>') = self.input[self.position..].chars().skip(1).next() {
          self.token(TokenKind::Arrow, Some(2))
        } else {
          self.token(TokenKind::Equals, Some(1))
        }
      }
      ch => if self.input[self.position..].starts_with("true") {
        self.token(TokenKind::Bool, Some(4))
      } else if self.input[self.position..].starts_with("false") {
        self.token(TokenKind::Bool, Some(5))
      } else if ch.is_alphabetic() {
        let len = { self.distance_to_next(|c| !(c.is_alphanumeric() || c == '-' || c == '_')) };
        self.token(TokenKind::Ident, len)
      } else {
        let len = { self.distance_to_next(|c| c.is_whitespace()) };
        self.token(TokenKind::Unknown, len)
      },
    })
  }
}

impl<'i> Lexer<'i> {
  fn seek<F: Fn(char) -> bool>(&mut self, pred: F) {
    if let Some((offset, ch)) = self.search(pred) {
      self.position += offset;
      self.current = Some(ch);
    } else {
      self.position = self.input.len();
      self.current = None;
    }
  }

  fn distance_to_next<F: Fn(char) -> bool>(&self, pred: F) -> Option<usize> {
    for (offset, ch) in self.input[self.position..].char_indices().skip(1) {
      if pred(ch) {
        return Some(offset);
      }
    }
    None
  }

  fn search<F: Fn(char) -> bool>(&self, pred: F) -> Option<(usize, char)> {
    for (offset, ch) in self.input[self.position..].char_indices() {
      if pred(ch) {
        return Some((offset, ch));
      }
    }
    None
  }

  #[inline(always)]
  fn token(&mut self, kind: TokenKind, len: Option<usize>) -> Token<'i> {
    let end_pos = len
      .map(|len| len + self.position)
      .unwrap_or(self.input.len());
    let token = Token {
      kind: kind,
      input: self.input,
      start_pos: self.position,
      end_pos: end_pos,
    };
    self.position = end_pos;
    token
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  macro_rules! assert_lex {
    ($input: expr $(, $variant:ident($start:expr, $end:expr))*) => {{
      let input = $input;
      let expected = vec![$(Token{ input: input, kind: TokenKind::$variant, start_pos: $start, end_pos: $end }, )*].into_iter();
      let actual = lex(input);
      for (e, a) in expected.zip(actual) {
        assert_eq!(e, a);
      }
    }};
  }

  #[test]
  fn test_literals() {
    assert_lex!(
      "1 \"foo\" 1.24 true false",
      Number(0, 1),
      String(2, 7),
      Number(8, 12),
      Bool(13, 17),
      Bool(18, 23)
    );
  }

  #[test]
  fn test_func_list_and_block() {
    assert_lex!(
      "each: [1 2 3] do: {x=> calc: x times: 2}",
      Ident(0, 4),
      Colon(4, 5),
      LSquare(6, 7),
      Number(7, 8),
      Number(9, 10),
      Number(11, 12),
      RSquare(12, 13),
      Ident(14, 16),
      Colon(16, 17),
      LCurly(18, 19),
      Ident(19, 20),
      Arrow(20, 22),
      Ident(23, 27),
      Colon(27, 28),
      Ident(29, 30),
      Ident(31, 36),
      Colon(36, 37),
      Number(38, 39),
      RCurly(39, 40)
    );
  }

  #[test]
  fn test_records_and_variables() {
    assert_lex!(
      "with: [ foo= 25 ] do : { it => it.foo }",
      Ident(0, 4),
      Colon(4, 5),
      LSquare(6, 7),
      Ident(8, 11),
      Equals(11, 12),
      Number(13, 15),
      RSquare(16, 17),
      Ident(18, 20),
      Colon(21, 22),
      LCurly(23, 24),
      Ident(25, 27),
      Arrow(28, 30),
      Ident(31, 33),
      Dot(33, 34),
      Ident(34, 37),
      RCurly(38, 39)
    )
  }
}
