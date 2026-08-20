//! Parser diagnostics helpers.

use super::lexer::Token;

pub(crate) fn format_expected_found(
  expected: impl Into<String>,
  found: impl Into<String>,
) -> String {
  format!("expected {}, found {}", expected.into(), found.into())
}

pub(crate) fn token_desc(token: &Token) -> String {
  match token {
    Token::Ident(name) => format!("identifier '{}'", name),
    Token::Int(_) => "int".to_string(),
    Token::Float(_) => "float".to_string(),
    Token::String(_) => "string".to_string(),
    Token::StringInterp(_) => "interpolated string".to_string(),
    Token::Path(_) => "path".to_string(),
    Token::PathInterp(_, _) => "interpolated path".to_string(),
    Token::Keyword(kw) => format!("keyword '{}'", kw),
    Token::Symbol(ch) => format!("symbol '{}'", ch),
    Token::Op(op) => format!("operator '{}'", op),
  }
}
