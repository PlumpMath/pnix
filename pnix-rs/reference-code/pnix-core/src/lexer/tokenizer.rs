//! Tokenizer - Nix/Clojure mixed code tokenizer
//!
//! ## 헌법 준수 (P0-1)
//!
//! 순수 토큰 분해, 실행 없음

use super::PnixToken;
use crate::diagnostics::{error_codes, ErrorCode};
use crate::text_width::char_width;
use thiserror::Error;

/// 토큰화 에러: 토큰화 중 발생하는 에러 타입
///
/// # Example
/// ```rust
/// use pnix_core::lexer::TokenizeError;
/// use pnix_core::diagnostics::error_codes;
/// let err = TokenizeError::InvalidChar {
///     code: error_codes::INVALID_CHAR,
///     ch: '@',
///     position: 3,
/// };
/// assert!(matches!(err, TokenizeError::InvalidChar { .. }));
/// ```
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TokenizeError {
  /// 잘못된 문자: 유효하지 않은 문자 발견
  #[error("[{code}] invalid character '{ch}' at position {position}")]
  InvalidChar {
    /// 에러 코드
    code: ErrorCode,
    /// 잘못된 문자
    ch: char,
    /// 문자 위치
    position: usize,
  },

  /// 일반 토큰화 에러: 토큰화 중 발생한 일반 에러
  #[error("[{code}] tokenize error: {message} at position {position}")]
  Tokenize {
    /// 에러 코드
    code: ErrorCode,
    /// 에러 메시지
    message: String,
    /// 에러 위치
    position: usize,
  },

  /// 닫히지 않은 문자열: 문자열 리터럴이 제대로 닫히지 않음
  #[error("[{code}] unclosed string")]
  UnclosedString {
    /// 에러 코드
    code: ErrorCode,
  },
}

/// Nix/Clojure 통합 토크나이저: Nix와 Clojure 코드를 토큰화하는 토크나이저
///
/// `clj { }` 블록 내부는 Clojure 토큰으로 처리합니다.
pub struct Tokenizer {
  /// 소스 코드 (문자 벡터)
  source: Vec<char>,
  /// 현재 위치 (바이트 오프셋)
  position: usize,
  /// 현재 라인 번호
  line: usize,
  /// 현재 컬럼 번호
  column: usize,
  /// Clojure 블록 내부 여부
  in_clj_block: bool,
  /// Clojure 블록 중첩 깊이
  clj_block_depth: usize,
}

impl Tokenizer {
  /// 새 토크나이저 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(source: &str) -> Self {
    let normalized = source.replace("\r\n", "\n").replace('\r', "\n");
    Self {
      source: normalized.chars().collect(),
      position: 0,
      line: 1,
      column: 1,
      in_clj_block: false,
      clj_block_depth: 0,
    }
  }

  /// 전체 소스를 토큰화
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 파싱만, 값 계산 없음
  pub fn tokenize(&mut self) -> Result<Vec<PnixToken>, TokenizeError> {
    let mut tokens = Vec::new();

    while self.position < self.source.len() {
      // 공백 건너뛰기
      if self.skip_whitespace() {
        continue;
      }

      // 주석 처리
      if self.peek() == Some('#') && self.peek_n(1) != Some('{') {
        if let Some(comment) = self.read_line_comment()? {
          tokens.push(comment);
          continue;
        }
      }

      // clj { 블록 감지
      if !self.in_clj_block && self.peek_clj_block() {
        self.in_clj_block = true;
        self.clj_block_depth = 1;
        tokens.push(PnixToken::CljBlockStart);
        self.advance_n(3); // "clj"
        self.skip_whitespace();
        self.advance(); // "{"
        continue;
      }

      // 블록 내부 처리
      if self.in_clj_block {
        // Clojure 블록 종료 체크
        if self.peek() == Some('}') && self.clj_block_depth == 1 {
          tokens.push(PnixToken::CljBlockEnd);
          self.advance();
          self.in_clj_block = false;
          self.clj_block_depth = 0;
          continue;
        }

        // Clojure 토큰 읽기
        match self.read_clj_token()? {
          Some(token) => tokens.push(token),
          None => break,
        }
      } else {
        // Nix 토큰 읽기
        match self.read_nix_token()? {
          Some(token) => tokens.push(token),
          None => break,
        }
      }
    }

    Ok(tokens)
  }

  /// Clojure 블록 시작 체크
  fn peek_clj_block(&self) -> bool {
    if self.position + 3 >= self.source.len() {
      return false;
    }

    // "clj" 키워드 체크
    if self.source[self.position..self.position + 3]
      .iter()
      .collect::<String>()
      != "clj"
    {
      return false;
    }

    // 단어 경계 체크
    if self.position > 0 {
      let prev = self.source[self.position - 1];
      if prev.is_alphanumeric() || prev == '-' {
        return false;
      }
    }

    // 다음 문자가 공백이고 그 다음이 '{'인지 체크
    let mut pos = self.position + 3;
    while pos < self.source.len() && self.source[pos].is_whitespace() {
      pos += 1;
    }

    pos < self.source.len() && self.source[pos] == '{'
  }

  /// Clojure 토큰 읽기
  fn read_clj_token(&mut self) -> Result<Option<PnixToken>, TokenizeError> {
    if self.position >= self.source.len() {
      return Ok(None);
    }

    let ch = self.source[self.position];

    match ch {
      '(' => {
        self.advance();
        Ok(Some(PnixToken::CljListStart))
      }
      ')' => {
        self.advance();
        Ok(Some(PnixToken::CljListEnd))
      }
      '[' => {
        self.advance();
        Ok(Some(PnixToken::CljVectorStart))
      }
      ']' => {
        self.advance();
        Ok(Some(PnixToken::CljVectorEnd))
      }
      '{' => {
        self.advance();
        self.clj_block_depth += 1;
        Ok(Some(PnixToken::CljMapStart))
      }
      '}' => {
        self.advance();
        if self.clj_block_depth > 0 {
          self.clj_block_depth -= 1;
        }
        Ok(Some(PnixToken::CljMapEnd))
      }
      '\'' => {
        self.advance();
        Ok(Some(PnixToken::CljQuote))
      }
      '`' => {
        self.advance();
        Ok(Some(PnixToken::CljSyntaxQuote))
      }
      '~' => {
        self.advance();
        if self.peek() == Some('@') {
          self.advance();
          Ok(Some(PnixToken::CljSpliceUnquote))
        } else {
          Ok(Some(PnixToken::CljUnquote))
        }
      }
      '@' => {
        self.advance();
        Ok(Some(PnixToken::CljDeref))
      }
      '#' => {
        self.advance();
        if self.peek() == Some('\'') {
          self.advance();
          Ok(Some(PnixToken::CljVarQuote))
        } else if self.peek() == Some('{') {
          self.advance();
          Ok(Some(PnixToken::CljSetStart))
        } else {
          // 키워드로 처리
          let keyword = self.read_identifier()?;
          Ok(Some(PnixToken::CljKeyword(keyword)))
        }
      }
      '"' => {
        let string = self.read_string()?;
        Ok(Some(PnixToken::CljString(string)))
      }
      ':' => {
        self.advance();
        let keyword = self.read_identifier()?;
        Ok(Some(PnixToken::CljKeyword(keyword)))
      }
      '-' | '+' => {
        // 음수/양수 부호 처리
        let sign = if ch == '-' { -1.0 } else { 1.0 };
        self.advance();
        if self.position < self.source.len() && self.source[self.position].is_ascii_digit() {
          let number = self.read_number()?;
          Ok(Some(PnixToken::CljNumber(number * sign)))
        } else {
          // 부호만 있으면 심볼로 처리
          Ok(Some(PnixToken::CljSymbol(if sign < 0.0 {
            "-".to_string()
          } else {
            "+".to_string()
          })))
        }
      }
      '0'..='9' => {
        let number = self.read_number()?;
        Ok(Some(PnixToken::CljNumber(number)))
      }
      _ if ch.is_alphabetic()
        || ch == '-'
        || ch == '*'
        || ch == '+'
        || ch == '!'
        || ch == '?'
        || ch == '='
        || ch == '<'
        || ch == '>' =>
      {
        let ident = self.read_identifier()?;
        // 특수 값 체크
        match ident.as_str() {
          "true" => Ok(Some(PnixToken::CljBool(true))),
          "false" => Ok(Some(PnixToken::CljBool(false))),
          "nil" => Ok(Some(PnixToken::CljNil)),
          _ => Ok(Some(PnixToken::CljSymbol(ident))),
        }
      }
      _ => Err(TokenizeError::InvalidChar {
        code: error_codes::INVALID_CHAR,
        ch,
        position: self.position,
      }),
    }
  }

  /// Nix 토큰 읽기
  fn read_nix_token(&mut self) -> Result<Option<PnixToken>, TokenizeError> {
    if self.position >= self.source.len() {
      return Ok(None);
    }

    let ch = self.source[self.position];

    match ch {
      '{' => {
        self.advance();
        Ok(Some(PnixToken::NixAttrSetStart))
      }
      '}' => {
        self.advance();
        Ok(Some(PnixToken::NixAttrSetEnd))
      }
      '[' => {
        self.advance();
        Ok(Some(PnixToken::NixListStart))
      }
      ']' => {
        self.advance();
        Ok(Some(PnixToken::NixListEnd))
      }
      '(' => {
        self.advance();
        Ok(Some(PnixToken::NixParenStart))
      }
      ')' => {
        self.advance();
        Ok(Some(PnixToken::NixParenEnd))
      }
      '=' => {
        self.advance();
        // == 체크
        if self.peek() == Some('=') {
          self.advance();
          Ok(Some(PnixToken::NixEqual))
        } else {
          Ok(Some(PnixToken::NixAssign))
        }
      }
      '!' => {
        self.advance();
        // != 체크
        if self.peek() == Some('=') {
          self.advance();
          Ok(Some(PnixToken::NixNotEqual))
        } else {
          Ok(Some(PnixToken::NixIdent("!".to_string())))
        }
      }
      ';' => {
        self.advance();
        Ok(Some(PnixToken::NixSemicolon))
      }
      ':' => {
        self.advance();
        Ok(Some(PnixToken::NixColon))
      }
      '.' => {
        // 상대 경로 확인: ./foo, ../bar
        let next_ch = self.peek_n(1);
        if next_ch == Some('/') {
          // ./path 형태
          self.advance(); // '.' 건너뛰기
          match self.read_path('.') {
            Ok(path) => Ok(Some(PnixToken::NixPath(path))),
            Err(_) => Ok(Some(PnixToken::NixDot)),
          }
        } else if next_ch == Some('.') && self.peek_n(2) == Some('/') {
          // ../path 형태
          self.advance(); // 첫 번째 '.' 건너뛰기
          match self.read_path('.') {
            Ok(path) => Ok(Some(PnixToken::NixPath(path))),
            Err(_) => Ok(Some(PnixToken::NixDot)),
          }
        } else {
          self.advance();
          if self.peek() == Some('.') && self.peek_n(1) == Some('.') {
            self.advance();
            self.advance();
            Ok(Some(PnixToken::NixEllipsis))
          } else {
            Ok(Some(PnixToken::NixDot))
          }
        }
      }
      '?' => {
        self.advance();
        Ok(Some(PnixToken::NixQuestion))
      }
      '@' => {
        self.advance();
        Ok(Some(PnixToken::NixAt))
      }
      '&' => {
        self.advance();
        if self.peek() == Some('&') {
          self.advance();
          Ok(Some(PnixToken::NixAnd))
        } else {
          Ok(Some(PnixToken::NixIdent("&".to_string())))
        }
      }
      '\'' => {
        // Y08a-7: 멀티라인 문자열 ''...'' 지원
        if self.position + 1 < self.source.len() && self.source[self.position + 1] == '\'' {
          // ''로 시작하는 멀티라인 문자열
          self.advance(); // 첫 번째 ' 건너뛰기
          self.advance(); // 두 번째 ' 건너뛰기
          let string = self.read_multiline_string()?;
          Ok(Some(PnixToken::NixString(string)))
        } else {
          // 단일 '는 Nix에서 사용되지 않음 (Clojure 컨텍스트가 아니면 에러)
          // 하지만 identifier의 일부일 수 있으므로 일단 identifier로 처리
          let ident = self.read_identifier()?;
          Ok(Some(PnixToken::NixIdent(ident)))
        }
      }
      '"' => {
        let string = self.read_string()?;
        Ok(Some(PnixToken::NixString(string)))
      }
      '<' => {
        // 경로 리터럴 <nix-path>
        self.advance();
        let path = self.read_until('>')?;
        Ok(Some(PnixToken::NixPath(path)))
      }
      '-' | '+' => {
        // 음수/양수 부호 처리
        let sign = if ch == '-' { -1.0 } else { 1.0 };
        self.advance();
        if self.position < self.source.len() && self.source[self.position].is_ascii_digit() {
          let number = self.read_number()?;
          Ok(Some(PnixToken::NixNumber(number * sign)))
        } else {
          // 부호만 있으면 연산자로 처리
          Ok(Some(PnixToken::NixIdent(if sign < 0.0 {
            "-".to_string()
          } else {
            "+".to_string()
          })))
        }
      }
      '*' => {
        // 곱셈 연산자
        self.advance();
        Ok(Some(PnixToken::NixIdent("*".to_string())))
      }
      '/' => {
        // 나눗셈 연산자 또는 경로
        if self.position + 1 < self.source.len() {
          let next_ch = self.source[self.position + 1];
          if next_ch.is_alphabetic() || next_ch == '.' || next_ch == '~' || next_ch == '/' {
            self.advance(); // '/' 건너뛰기
            match self.read_path('/') {
              Ok(path) => return Ok(Some(PnixToken::NixPath(path))),
              Err(_) => return Ok(Some(PnixToken::NixIdent("/".to_string()))),
            }
          }
        }
        self.advance();
        Ok(Some(PnixToken::NixIdent("/".to_string())))
      }
      '0'..='9' => {
        let number = self.read_number()?;
        Ok(Some(PnixToken::NixNumber(number)))
      }
      _ if ch.is_alphabetic() || ch == '_' => {
        let ident = self.read_identifier()?;
        // Nix 키워드 체크
        match ident.as_str() {
          "let" => Ok(Some(PnixToken::NixLet)),
          "in" => Ok(Some(PnixToken::NixIn)),
          "with" => Ok(Some(PnixToken::NixWith)),
          "if" => Ok(Some(PnixToken::NixIf)),
          "then" => Ok(Some(PnixToken::NixThen)),
          "else" => Ok(Some(PnixToken::NixElse)),
          "rec" => Ok(Some(PnixToken::NixRec)),
          "assert" => Ok(Some(PnixToken::NixAssert)),
          "or" => Ok(Some(PnixToken::NixOr)),
          "true" => Ok(Some(PnixToken::NixBool(true))),
          "false" => Ok(Some(PnixToken::NixBool(false))),
          "null" => Ok(Some(PnixToken::NixNull)),
          _ => Ok(Some(PnixToken::NixIdent(ident))),
        }
      }
      '~' => {
        // 홈 경로: ~/foo
        if self.peek_n(1) == Some('/') {
          self.advance(); // '~' 건너뛰기
          match self.read_path('~') {
            Ok(path) => Ok(Some(PnixToken::NixPath(path))),
            Err(_) => Ok(Some(PnixToken::NixIdent("~".to_string()))),
          }
        } else {
          self.advance();
          Ok(Some(PnixToken::NixIdent("~".to_string())))
        }
      }
      _ => Err(TokenizeError::InvalidChar {
        code: error_codes::INVALID_CHAR,
        ch,
        position: self.position,
      }),
    }
  }

  /// 공백 건너뛰기
  fn skip_whitespace(&mut self) -> bool {
    let mut skipped = false;
    while self.position < self.source.len() {
      let ch = self.source[self.position];
      if ch.is_whitespace() {
        self.advance();
        skipped = true;
      } else {
        break;
      }
    }
    skipped
  }

  /// 라인 주석 읽기
  fn read_line_comment(&mut self) -> Result<Option<PnixToken>, TokenizeError> {
    if self.peek() != Some('#') {
      return Ok(None);
    }

    self.advance(); // '#'

    let mut comment = String::new();
    while self.position < self.source.len() {
      let ch = self.source[self.position];
      if ch == '\n' {
        break;
      }
      comment.push(ch);
      self.advance();
    }

    Ok(Some(PnixToken::LineComment(comment)))
  }

  /// 문자열 읽기
  fn read_string(&mut self) -> Result<String, TokenizeError> {
    if self.peek() != Some('"') {
      return Err(TokenizeError::Tokenize {
        code: error_codes::TOKENIZE_ERROR,
        message: "not a string".to_string(),
        position: self.position,
      });
    }

    self.advance(); // '"'
    let mut result = String::new();
    let mut escape = false;

    while self.position < self.source.len() {
      let ch = self.source[self.position];

      if escape {
        match ch {
          'n' => result.push('\n'),
          't' => result.push('\t'),
          'r' => result.push('\r'),
          '\\' => result.push('\\'),
          '"' => result.push('"'),
          _ => result.push(ch),
        }
        escape = false;
        self.advance();
      } else if ch == '\\' {
        escape = true;
        self.advance();
      } else if ch == '"' {
        self.advance();
        return Ok(result);
      } else {
        result.push(ch);
        self.advance();
      }
    }

    Err(TokenizeError::UnclosedString {
      code: error_codes::UNCLOSED_STRING,
    })
  }

  /// Y08a-7: Nix 멀티라인 문자열 읽기 (''...'')
  ///
  /// Nix 멀티라인 문자열 규칙:
  /// - ''로 시작하고 ''로 끝남
  /// - 이스케이프: 문자열 내부의 ''는 '로 변환, $는 $$로 이스케이프
  /// - 줄바꿈은 그대로 유지
  /// - 종료 조건: ''를 만나면 종료 (간단한 구현)
  fn read_multiline_string(&mut self) -> Result<String, TokenizeError> {
    let mut result = String::new();

    while self.position < self.source.len() {
      if self.position + 1 < self.source.len()
        && self.source[self.position] == '\''
        && self.source[self.position + 1] == '\''
      {
        // '' 발견: 종료
        self.advance(); // 첫 번째 '
        self.advance(); // 두 번째 '
        return Ok(result);
      } else if self.position + 1 < self.source.len()
        && self.source[self.position] == '$'
        && self.source[self.position + 1] == '$'
      {
        // $$는 $로 변환 (이스케이프)
        result.push('$');
        self.advance(); // 첫 번째 $
        self.advance(); // 두 번째 $
      } else {
        // 일반 문자
        result.push(self.source[self.position]);
        self.advance();
      }
    }

    Err(TokenizeError::UnclosedString {
      code: error_codes::UNCLOSED_STRING,
    })
  }

  /// 숫자 읽기 (이미 부호는 처리됨)
  fn read_number(&mut self) -> Result<f64, TokenizeError> {
    let start = self.position;
    let mut has_dot = false;
    let mut num_str = String::new();

    while self.position < self.source.len() {
      let ch = self.source[self.position];

      match ch {
        '0'..='9' => {
          num_str.push(ch);
          self.advance();
        }
        '.' if !has_dot => {
          num_str.push(ch);
          has_dot = true;
          self.advance();
        }
        'e' | 'E' => {
          num_str.push(ch);
          self.advance();
          if self.peek() == Some('-') || self.peek() == Some('+') {
            num_str.push(self.source[self.position]);
            self.advance();
          }
        }
        _ => break,
      }
    }

    if num_str.is_empty() {
      return Err(TokenizeError::Tokenize {
        code: error_codes::TOKENIZE_ERROR,
        message: "empty number".to_string(),
        position: start,
      });
    }

    num_str.parse::<f64>().map_err(|_| TokenizeError::Tokenize {
      code: error_codes::TOKENIZE_ERROR,
      message: format!("invalid number: {}", num_str),
      position: start,
    })
  }

  /// 식별자 읽기
  fn read_identifier(&mut self) -> Result<String, TokenizeError> {
    let mut ident = String::new();

    while self.position < self.source.len() {
      let ch = self.source[self.position];

      if ch.is_alphanumeric()
        || ch == '-'
        || ch == '_'
        || ch == '*'
        || ch == '+'
        || ch == '!'
        || ch == '?'
        || ch == '='
        || ch == '<'
        || ch == '>'
        || ch == '\''
      // Nix: foldl', foldr' 등
      {
        ident.push(ch);
        self.advance();
      } else {
        break;
      }
    }

    if ident.is_empty() {
      Err(TokenizeError::Tokenize {
        code: error_codes::TOKENIZE_ERROR,
        message: "empty identifier".to_string(),
        position: self.position,
      })
    } else {
      Ok(ident)
    }
  }

  /// 특정 문자까지 읽기
  fn read_until(&mut self, end: char) -> Result<String, TokenizeError> {
    let mut result = String::new();

    while self.position < self.source.len() {
      let ch = self.source[self.position];
      if ch == end {
        self.advance();
        return Ok(result);
      }
      result.push(ch);
      self.advance();
    }

    Err(TokenizeError::Tokenize {
      code: error_codes::TOKENIZE_ERROR,
      message: format!("character '{}' not found", end),
      position: self.position,
    })
  }

  /// Nix path literal 읽기
  fn read_path(&mut self, start_char: char) -> Result<String, TokenizeError> {
    let mut path = String::new();
    path.push(start_char);

    while self.position < self.source.len() {
      let ch = self.source[self.position];

      // Path에 포함될 수 있는 문자
      if ch.is_alphanumeric() || ch == '/' || ch == '.' || ch == '-' || ch == '_' || ch == '~' {
        path.push(ch);
        self.advance();
      } else {
        break;
      }
    }

    // 최소 2문자 이상이어야 경로로 인식
    if path.len() > 1
      && (path.starts_with('/')
        || path.starts_with("./")
        || path.starts_with("../")
        || path.starts_with("~/"))
    {
      Ok(path)
    } else {
      Err(TokenizeError::Tokenize {
        code: error_codes::TOKENIZE_ERROR,
        message: "invalid path literal".to_string(),
        position: self.position,
      })
    }
  }

  /// 현재 문자 확인
  fn peek(&self) -> Option<char> {
    if self.position < self.source.len() {
      Some(self.source[self.position])
    } else {
      None
    }
  }

  /// n번째 앞 문자 확인
  fn peek_n(&self, n: usize) -> Option<char> {
    if self.position + n < self.source.len() {
      Some(self.source[self.position + n])
    } else {
      None
    }
  }

  /// 다음 문자로 이동
  fn advance(&mut self) {
    if self.position < self.source.len() {
      let ch = self.source[self.position];
      self.position += 1;
      if ch == '\n' {
        self.line += 1;
        self.column = 1;
      } else {
        let width = char_width(ch).max(1);
        self.column = self.column.saturating_add(width);
      }
    }
  }

  /// n개 문자 건너뛰기
  fn advance_n(&mut self, n: usize) {
    for _ in 0..n {
      self.advance();
    }
  }
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_tokenize_nix_simple() {
    let mut tokenizer = Tokenizer::new("let x = 42; in x");
    let tokens = tokenizer.tokenize().unwrap();

    assert!(tokens.contains(&PnixToken::NixLet));
    assert!(tokens.contains(&PnixToken::NixIdent("x".to_string())));
    assert!(tokens.contains(&PnixToken::NixAssign));
    assert!(tokens.contains(&PnixToken::NixNumber(42.0)));
  }

  #[test]
  fn test_tokenize_crlf() {
    let mut tokenizer = Tokenizer::new("let x = 1;\r\nin x");
    let tokens = tokenizer.tokenize().unwrap();

    assert!(tokens.contains(&PnixToken::NixLet));
    assert!(tokens.contains(&PnixToken::NixIn));
  }

  #[test]
  fn test_column_advances_by_display_width() {
    let mut tokenizer = Tokenizer::new("한😀a");
    assert_eq!(tokenizer.column, 1);
    tokenizer.advance(); // '한' width 2
    assert_eq!(tokenizer.column, 3);
    tokenizer.advance(); // '😀' width 2
    assert_eq!(tokenizer.column, 5);
    tokenizer.advance(); // 'a' width 1
    assert_eq!(tokenizer.column, 6);
  }

  #[test]
  fn test_tokenize_clj_block() {
    let mut tokenizer = Tokenizer::new("clj { (+ 1 2) }");
    let tokens = tokenizer.tokenize().unwrap();

    assert!(tokens.contains(&PnixToken::CljBlockStart));
    assert!(tokens.contains(&PnixToken::CljListStart));
    assert!(tokens.contains(&PnixToken::CljSymbol("+".to_string())));
    assert!(tokens.contains(&PnixToken::CljNumber(1.0)));
    assert!(tokens.contains(&PnixToken::CljNumber(2.0)));
    assert!(tokens.contains(&PnixToken::CljListEnd));
    assert!(tokens.contains(&PnixToken::CljBlockEnd));
  }

  #[test]
  fn test_tokenize_mixed() {
    let mut tokenizer = Tokenizer::new(
      r#"
let
  result = clj { (+ 1 2) };
in
  result
"#,
    );
    let tokens = tokenizer.tokenize().unwrap();

    assert!(tokens.contains(&PnixToken::NixLet));
    assert!(tokens.contains(&PnixToken::NixIdent("result".to_string())));
    assert!(tokens.contains(&PnixToken::CljBlockStart));
    assert!(tokens.contains(&PnixToken::CljSymbol("+".to_string())));
    assert!(tokens.contains(&PnixToken::CljBlockEnd));
    assert!(tokens.contains(&PnixToken::NixIn));
  }

  #[test]
  fn test_tokenize_nix_keywords() {
    let mut tokenizer = Tokenizer::new("let with if then else rec assert or in");
    let tokens = tokenizer.tokenize().unwrap();

    assert!(tokens.contains(&PnixToken::NixLet));
    assert!(tokens.contains(&PnixToken::NixWith));
    assert!(tokens.contains(&PnixToken::NixIf));
    assert!(tokens.contains(&PnixToken::NixThen));
    assert!(tokens.contains(&PnixToken::NixElse));
    assert!(tokens.contains(&PnixToken::NixRec));
    assert!(tokens.contains(&PnixToken::NixAssert));
    assert!(tokens.contains(&PnixToken::NixOr));
    assert!(tokens.contains(&PnixToken::NixIn));
  }

  #[test]
  fn test_tokenize_nix_booleans() {
    let mut tokenizer = Tokenizer::new("true false null");
    let tokens = tokenizer.tokenize().unwrap();

    assert!(tokens.contains(&PnixToken::NixBool(true)));
    assert!(tokens.contains(&PnixToken::NixBool(false)));
    assert!(tokens.contains(&PnixToken::NixNull));
  }

  #[test]
  fn test_tokenize_clj_keywords() {
    let mut tokenizer = Tokenizer::new("clj { :key :another-key }");
    let tokens = tokenizer.tokenize().unwrap();

    assert!(tokens.contains(&PnixToken::CljKeyword("key".to_string())));
    assert!(tokens.contains(&PnixToken::CljKeyword("another-key".to_string())));
  }

  #[test]
  fn test_tokenize_string() {
    let mut tokenizer = Tokenizer::new(r#""hello world""#);
    let tokens = tokenizer.tokenize().unwrap();

    assert!(tokens.contains(&PnixToken::NixString("hello world".to_string())));
  }

  #[test]
  fn test_tokenize_string_escape() {
    let mut tokenizer = Tokenizer::new(r#""hello\nworld""#);
    let tokens = tokenizer.tokenize().unwrap();

    assert!(tokens.contains(&PnixToken::NixString("hello\nworld".to_string())));
  }

  #[test]
  fn test_tokenize_multiline_string() {
    // Y08a-7: 멀티라인 문자열 ''...'' 파싱
    let mut tokenizer = Tokenizer::new("''hello\nworld''");
    let tokens = tokenizer.tokenize().unwrap();

    assert!(tokens.contains(&PnixToken::NixString("hello\nworld".to_string())));
  }

  #[test]
  fn test_tokenize_multiline_string_with_dollar() {
    // Y08a-7: 멀티라인 문자열 내부의 $$ 이스케이프
    let mut tokenizer = Tokenizer::new("''hello $$world''");
    let tokens = tokenizer.tokenize().unwrap();

    assert!(tokens.contains(&PnixToken::NixString("hello $world".to_string())));
  }

  #[test]
  fn test_preprocess_and_parse_py_block() {
    // Y08a-7: py { ... } 전처리 후 파싱
    use crate::lexer::preprocess_nix_code;

    let input = "py { x = 10 }";
    let preprocessed = preprocess_nix_code(input);

    // 전처리 결과에 ''가 포함되어야 함
    assert!(preprocessed.contains("''"));

    // 파싱 가능해야 함
    let mut tokenizer = Tokenizer::new(&preprocessed);
    let result = tokenizer.tokenize();
    assert!(result.is_ok());
  }

  #[test]
  fn test_preprocess_and_parse_js_block() {
    // Y08a-7: js { ... } 전처리 후 파싱
    use crate::lexer::preprocess_nix_code;

    let input = "js { const x = 1 + 2; }";
    let preprocessed = preprocess_nix_code(input);

    // 전처리 결과에 ''가 포함되어야 함
    assert!(preprocessed.contains("''"));

    // 파싱 가능해야 함
    let mut tokenizer = Tokenizer::new(&preprocessed);
    let result = tokenizer.tokenize();
    assert!(result.is_ok());
  }

  #[test]
  fn test_tokenize_float() {
    let mut tokenizer = Tokenizer::new("2.71 1e10 2.5e-3");
    let tokens = tokenizer.tokenize().unwrap();

    assert!(tokens.contains(&PnixToken::NixNumber(2.71)));
    assert!(tokens.contains(&PnixToken::NixNumber(1e10)));
    assert!(tokens.contains(&PnixToken::NixNumber(2.5e-3)));
  }

  #[test]
  fn test_tokenize_operators() {
    let mut tokenizer = Tokenizer::new("== != && ...");
    let tokens = tokenizer.tokenize().unwrap();

    assert!(tokens.contains(&PnixToken::NixEqual));
    assert!(tokens.contains(&PnixToken::NixNotEqual));
    assert!(tokens.contains(&PnixToken::NixAnd));
    assert!(tokens.contains(&PnixToken::NixEllipsis));
  }

  #[test]
  fn test_tokenize_comment() {
    let mut tokenizer = Tokenizer::new("# this is a comment\nlet x = 1");
    let tokens = tokenizer.tokenize().unwrap();

    assert!(tokens
      .iter()
      .any(|t| matches!(t, PnixToken::LineComment(_))));
    assert!(tokens.contains(&PnixToken::NixLet));
  }
}
