//! PnixToken - Token types for Nix/Clojure mixed code
//!
//! ## 헌법 준수 (P0-1)
//!
//! 순수 enum 정의, 값 계산 없음

use serde::{Deserialize, Serialize};

/// Pnix 통합 토큰: Nix와 Clojure 토큰을 통합한 토큰 타입
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PnixToken {
  // ========== Nix Tokens ==========
  /// Nix identifier
  NixIdent(
    /// 식별자 이름
    String,
  ),
  /// Nix string literal
  NixString(
    /// 문자열 값
    String,
  ),
  /// Nix number literal
  NixNumber(
    /// 숫자 값
    f64,
  ),
  /// Nix boolean literal
  NixBool(
    /// 불리언 값
    bool,
  ),
  /// Nix null literal
  NixNull,
  /// Nix path literal
  NixPath(
    /// 경로 문자열
    String,
  ),

  // ========== Nix Keywords ==========
  /// let
  NixLet,
  /// in
  NixIn,
  /// with
  NixWith,
  /// if
  NixIf,
  /// then
  NixThen,
  /// else
  NixElse,
  /// rec
  NixRec,
  /// assert
  NixAssert,
  /// or
  NixOr,
  /// && (logical and)
  NixAnd,

  // ========== Nix Delimiters ==========
  /// { (attribute set start)
  NixAttrSetStart,
  /// } (attribute set end)
  NixAttrSetEnd,
  /// [ (list start)
  NixListStart,
  /// ] (list end)
  NixListEnd,
  /// ( (parenthesis start)
  NixParenStart,
  /// ) (parenthesis end)
  NixParenEnd,
  /// = (assignment)
  NixAssign,
  /// == (equality)
  NixEqual,
  /// != (not equal)
  NixNotEqual,
  /// ; (semicolon)
  NixSemicolon,
  /// : (colon)
  NixColon,
  /// . (dot)
  NixDot,
  /// ? (question mark)
  NixQuestion,
  /// @ (at sign)
  NixAt,
  /// ... (ellipsis)
  NixEllipsis,

  // ========== Clojure Tokens ==========
  /// clj { (Clojure block start)
  CljBlockStart,
  /// } (Clojure block end)
  CljBlockEnd,
  /// Clojure symbol
  CljSymbol(
    /// 심볼 이름
    String,
  ),
  /// Clojure keyword (e.g., :key)
  CljKeyword(
    /// 키워드 이름
    String,
  ),
  /// Clojure string literal
  CljString(
    /// 문자열 값
    String,
  ),
  /// Clojure number literal
  CljNumber(
    /// 숫자 값
    f64,
  ),
  /// Clojure boolean literal
  CljBool(
    /// 불리언 값
    bool,
  ),
  /// Clojure nil
  CljNil,

  // ========== Clojure Delimiters ==========
  /// ( (list start)
  CljListStart,
  /// ) (list end)
  CljListEnd,
  /// [ (vector start)
  CljVectorStart,
  /// ] (vector end)
  CljVectorEnd,
  /// { (map start)
  CljMapStart,
  /// } (map end)
  CljMapEnd,
  /// #{ (set start)
  CljSetStart,
  /// } (set end)
  CljSetEnd,
  /// ' (quote)
  CljQuote,
  /// ` (syntax quote)
  CljSyntaxQuote,
  /// ~ (unquote)
  CljUnquote,
  /// ~@ (splice unquote)
  CljSpliceUnquote,
  /// @ (deref)
  CljDeref,
  /// #' (var quote)
  CljVarQuote,

  // ========== Common ==========
  /// Whitespace
  Whitespace,
  /// Block comment
  Comment(
    /// 주석 내용
    String,
  ),
  /// Line comment (# or ;)
  LineComment(
    /// 주석 내용
    String,
  ),
  /// Newline
  Newline,

  // ========== Debug ==========
  /// Position info (for debugging)
  Position {
    /// 라인 번호
    line: usize,
    /// 컬럼 번호
    column: usize,
  },
}

impl PnixToken {
  /// Check if token is a Nix token
  pub fn is_nix(&self) -> bool {
    matches!(
      self,
      PnixToken::NixIdent(_)
        | PnixToken::NixString(_)
        | PnixToken::NixNumber(_)
        | PnixToken::NixBool(_)
        | PnixToken::NixNull
        | PnixToken::NixPath(_)
        | PnixToken::NixLet
        | PnixToken::NixIn
        | PnixToken::NixWith
        | PnixToken::NixIf
        | PnixToken::NixThen
        | PnixToken::NixElse
        | PnixToken::NixRec
        | PnixToken::NixAssert
        | PnixToken::NixOr
        | PnixToken::NixAnd
        | PnixToken::NixAttrSetStart
        | PnixToken::NixAttrSetEnd
        | PnixToken::NixListStart
        | PnixToken::NixListEnd
        | PnixToken::NixParenStart
        | PnixToken::NixParenEnd
        | PnixToken::NixAssign
        | PnixToken::NixEqual
        | PnixToken::NixNotEqual
        | PnixToken::NixSemicolon
        | PnixToken::NixColon
        | PnixToken::NixDot
        | PnixToken::NixQuestion
        | PnixToken::NixAt
        | PnixToken::NixEllipsis
    )
  }

  /// Check if token is a Clojure token
  pub fn is_clj(&self) -> bool {
    matches!(
      self,
      PnixToken::CljBlockStart
        | PnixToken::CljBlockEnd
        | PnixToken::CljSymbol(_)
        | PnixToken::CljKeyword(_)
        | PnixToken::CljString(_)
        | PnixToken::CljNumber(_)
        | PnixToken::CljBool(_)
        | PnixToken::CljNil
        | PnixToken::CljListStart
        | PnixToken::CljListEnd
        | PnixToken::CljVectorStart
        | PnixToken::CljVectorEnd
        | PnixToken::CljMapStart
        | PnixToken::CljMapEnd
        | PnixToken::CljSetStart
        | PnixToken::CljSetEnd
        | PnixToken::CljQuote
        | PnixToken::CljSyntaxQuote
        | PnixToken::CljUnquote
        | PnixToken::CljSpliceUnquote
        | PnixToken::CljDeref
        | PnixToken::CljVarQuote
    )
  }

  /// Check if token is whitespace or comment
  pub fn is_trivia(&self) -> bool {
    matches!(
      self,
      PnixToken::Whitespace
        | PnixToken::Comment(_)
        | PnixToken::LineComment(_)
        | PnixToken::Newline
    )
  }
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_is_nix() {
    assert!(PnixToken::NixLet.is_nix());
    assert!(PnixToken::NixIdent("x".into()).is_nix());
    assert!(!PnixToken::CljSymbol("+".into()).is_nix());
  }

  #[test]
  fn test_is_clj() {
    assert!(PnixToken::CljSymbol("+".into()).is_clj());
    assert!(PnixToken::CljKeyword("key".into()).is_clj());
    assert!(!PnixToken::NixLet.is_clj());
  }

  #[test]
  fn test_is_trivia() {
    assert!(PnixToken::Whitespace.is_trivia());
    assert!(PnixToken::LineComment("comment".into()).is_trivia());
    assert!(!PnixToken::NixLet.is_trivia());
  }

  #[test]
  fn test_token_serde() {
    let token = PnixToken::NixNumber(42.0);
    let json = serde_json::to_string(&token).unwrap();
    let restored: PnixToken = serde_json::from_str(&json).unwrap();
    assert_eq!(token, restored);
  }
}
