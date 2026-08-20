//! Pnix 통합 에러 타입 정의
//!
//! pnix-old의 pnix_error/src/lib.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 에러 타입 정의만, 런타임 에러 제외 (EvalError는 pnix-executor에서만 사용)
//!
//! pnix-core는 컴파일러이므로 ParseError, CompileError, TokenizeError만 포함

use super::error_code::{error_codes, ErrorCode};
use super::lang_errors::{GhCodegenError, PnixError as LangPnixError, PythonError};
use super::nix_macro_error::SourceLocation;
use crate::lexer::TokenizeError as LexerTokenizeError;
use thiserror::Error;

/// 에러 소스: 에러의 원본 소스 정보
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{0}")]
pub struct ErrorSource(
  /// 에러 메시지
  String,
);

impl ErrorSource {
  /// 새 에러 소스 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(message: impl Into<String>) -> Self {
    Self(message.into())
  }
}

/// Parser error types
///
/// # Example
/// ```rust
/// use pnix_core::diagnostics::ParseError;
/// let err = ParseError::from_syntax("unexpected");
/// assert!(matches!(err, ParseError::Parse { .. }));
/// ```
/// 파서 에러 타입: 파싱 중 발생하는 에러 타입
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
  /// 일반 파싱 에러
  #[error("[{code}] Parse error at line {line}:{column}: {message}")]
  Parse {
    /// 에러 코드
    code: ErrorCode,
    /// 에러 메시지
    message: String,
    /// 라인 번호
    line: usize,
    /// 컬럼 번호
    column: usize,
  },

  /// 예상치 못한 토큰: 예상과 다른 토큰 발견
  #[error("[{code}] Parse error at line {line}:{column}: expected {expected}, found {found}")]
  UnexpectedToken {
    /// 에러 코드
    code: ErrorCode,
    /// 예상 토큰
    expected: String,
    /// 실제 토큰
    found: String,
    /// 라인 번호
    line: usize,
    /// 컬럼 번호
    column: usize,
  },

  /// 예상 토큰 누락: 예상한 토큰을 찾지 못함
  #[error("[{code}] Parse error at line {line}:{column}: expected {expected}, found {found}")]
  ExpectedToken {
    /// 에러 코드
    code: ErrorCode,
    /// 예상 토큰
    expected: String,
    /// 실제 토큰
    found: String,
    /// 라인 번호
    line: usize,
    /// 컬럼 번호
    column: usize,
  },

  /// 예상치 못한 EOF: 파일 끝에 도달했지만 더 많은 토큰이 필요함
  #[error("[{code}] Parse error at line {line}:{column}: expected {expected}, found EOF")]
  UnexpectedEof {
    /// 에러 코드
    code: ErrorCode,
    /// 예상 토큰
    expected: String,
    /// 라인 번호
    line: usize,
    /// 컬럼 번호
    column: usize,
  },

  /// 중첩 깊이 초과: 중첩이 너무 깊어서 파싱 불가
  #[error("[{code}] Parse error at line {line}:{column}: nesting too deep")]
  NestingTooDeep {
    /// 에러 코드
    code: ErrorCode,
    /// 라인 번호
    line: usize,
    /// 컬럼 번호
    column: usize,
  },
}

const DEFAULT_PARSE_LINE: usize = 1;
const DEFAULT_PARSE_COLUMN: usize = 1;
const DEFAULT_PARSE_SUMMARY: &str = "unspecified parse error";
const MAX_PARSE_SUMMARY_CHARS: usize = 240;
const PARSE_SUMMARY_TRUNCATED_SUFFIX: &str = "...";
const UNSUPPORTED_PARSE_PREFIX: &str = "unsupported: ";

fn normalize_parse_position(line: usize, column: usize) -> (usize, usize) {
  let normalized_line = if line == 0 { DEFAULT_PARSE_LINE } else { line };
  let normalized_column = if column == 0 {
    DEFAULT_PARSE_COLUMN
  } else {
    column
  };
  (normalized_line, normalized_column)
}

fn normalize_parse_summary(message: &str) -> String {
  let mut normalized = String::with_capacity(message.len());
  let mut pending_space = false;
  let mut chars = message.chars().peekable();
  while let Some(ch) = chars.next() {
    if ch == '\u{1b}' {
      // Strip ANSI escape sequences (for example: "\x1b[31m") to keep summaries canonical.
      if matches!(chars.peek(), Some('[')) {
        chars.next();
        while let Some(next) = chars.next() {
          if ('@'..='~').contains(&next) {
            break;
          }
        }
      }
      pending_space = true;
      continue;
    }

    // Keep diagnostics log-friendly by stripping control characters.
    if ch.is_control() {
      pending_space = true;
      continue;
    }
    if ch.is_whitespace() {
      pending_space = true;
      continue;
    }
    if pending_space && !normalized.is_empty() {
      normalized.push(' ');
    }
    normalized.push(ch);
    pending_space = false;
  }

  if normalized.is_empty() {
    return DEFAULT_PARSE_SUMMARY.to_string();
  }

  truncate_parse_summary(&normalized)
}

fn truncate_parse_summary(message: &str) -> String {
  let len = message.chars().count();
  if len <= MAX_PARSE_SUMMARY_CHARS {
    return message.to_string();
  }

  let keep = MAX_PARSE_SUMMARY_CHARS - PARSE_SUMMARY_TRUNCATED_SUFFIX.chars().count();
  let mut truncated: String = message.chars().take(keep).collect();
  truncated.push_str(PARSE_SUMMARY_TRUNCATED_SUFFIX);
  truncated
}

fn normalize_unsupported_summary(message: &str) -> String {
  let mut prefixed = String::with_capacity(UNSUPPORTED_PARSE_PREFIX.len() + message.len());
  prefixed.push_str(UNSUPPORTED_PARSE_PREFIX);
  prefixed.push_str(message);
  truncate_parse_summary(&prefixed)
}

/// Tokenizer error types
///
/// Re-export from lexer module
pub use crate::lexer::TokenizeError;

/// Compiler error types
///
/// # Example
/// ```rust
/// use pnix_core::diagnostics::{error_codes, CompileError};
/// let err = CompileError::TypeError {
///     code: error_codes::TYPE_MISMATCH,
///     expected: "Int".to_string(),
///     actual: "Bool".to_string(),
///     source: None,
/// };
/// assert!(matches!(err, CompileError::TypeError { .. }));
/// ```
/// 컴파일 에러 타입: 컴파일 중 발생하는 에러 타입
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum CompileError {
  #[error("[{code}] Compile error: {message}")]
  Compile {
    /// 에러 코드
    code: ErrorCode,
    /// 에러 메시지
    message: String,
    /// 원본 에러 (선택)
    #[source]
    source: Option<ErrorSource>,
  },

  #[error("[{code}] Compile error at {location}: {message}")]
  CompileAt {
    /// 에러 코드
    code: ErrorCode,
    /// 에러 메시지
    message: String,
    /// 소스 위치
    location: SourceLocation,
    /// 원본 에러 (선택)
    #[source]
    source: Option<ErrorSource>,
  },

  #[error("[{code}] Type error: expected {expected}, found {actual}")]
  TypeError {
    /// 에러 코드
    code: ErrorCode,
    /// 예상 타입
    expected: String,
    /// 실제 타입
    actual: String,
    /// 원본 에러 (선택)
    #[source]
    source: Option<ErrorSource>,
  },

  #[error("[{code}] Type error at {location}: expected {expected}, found {actual}")]
  TypeErrorAt {
    /// 에러 코드
    code: ErrorCode,
    /// 예상 타입
    expected: String,
    /// 실제 타입
    actual: String,
    /// 소스 위치
    location: SourceLocation,
    /// 원본 에러 (선택)
    #[source]
    source: Option<ErrorSource>,
  },

  #[error("[{code}] Undefined variable: {name}{hint}")]
  UndefinedVariable {
    /// 에러 코드
    code: ErrorCode,
    /// 변수 이름
    name: String,
    /// 힌트 메시지
    hint: String,
    /// 원본 에러 (선택)
    #[source]
    source: Option<ErrorSource>,
  },

  #[error("[{code}] Undefined variable at {location}: {name}{hint}")]
  UndefinedVariableAt {
    /// 에러 코드
    code: ErrorCode,
    /// 변수 이름
    name: String,
    /// 소스 위치
    location: SourceLocation,
    /// 힌트 메시지
    hint: String,
    /// 원본 에러 (선택)
    #[source]
    source: Option<ErrorSource>,
  },
}

// ============================================================================
// Extension methods for CompileError
// ============================================================================

impl CompileError {
  /// Create from AST to CT conversion error
  pub fn from_ast_to_ct(message: impl Into<String>, location: Option<SourceLocation>) -> Self {
    match location {
      Some(loc) => CompileError::CompileAt {
        code: error_codes::COMPILE_ERROR,
        message: format!("AST→CT: {}", message.into()),
        location: loc,
        source: None,
      },
      None => CompileError::Compile {
        code: error_codes::COMPILE_ERROR,
        message: format!("AST→CT: {}", message.into()),
        source: None,
      },
    }
  }

  /// Create from CT optimization error
  pub fn from_ct_optimization(message: impl Into<String>, optimization: impl Into<String>) -> Self {
    CompileError::Compile {
      code: error_codes::COMPILE_ERROR,
      message: format!(
        "CT optimization ({}): {}",
        optimization.into(),
        message.into()
      ),
      source: None,
    }
  }

  /// Create from CT to IR conversion error
  pub fn from_ct_to_ir(message: impl Into<String>, ct_node: impl Into<String>) -> Self {
    CompileError::Compile {
      code: error_codes::COMPILE_ERROR,
      message: format!("CT→IR ({}): {}", ct_node.into(), message.into()),
      source: None,
    }
  }

  /// Create from IR to Rust conversion error
  pub fn from_ir_to_rust(message: impl Into<String>, ir_node: impl Into<String>) -> Self {
    CompileError::Compile {
      code: error_codes::COMPILE_ERROR,
      message: format!("IR→Rust ({}): {}", ir_node.into(), message.into()),
      source: None,
    }
  }

  /// Create from type inference error
  pub fn from_type_inference(message: impl Into<String>, expression: impl Into<String>) -> Self {
    CompileError::TypeError {
      code: error_codes::TYPE_MISMATCH,
      expected: format!("inferred type for {}", expression.into()),
      actual: message.into(),
      source: None,
    }
  }

  /// Create from conversion error
  pub fn from_conversion(message: impl Into<String>) -> Self {
    CompileError::Compile {
      code: error_codes::COMPILE_ERROR,
      message: format!("conversion: {}", message.into()),
      source: None,
    }
  }

  pub fn undefined_variable(name: impl Into<String>, candidates: &[String]) -> Self {
    let name = name.into();
    let hint = suggestion_hint(&name, candidates);
    CompileError::UndefinedVariable {
      code: error_codes::UNDEFINED_VARIABLE,
      name,
      hint,
      source: None,
    }
  }

  pub fn undefined_variable_at(
    name: impl Into<String>,
    candidates: &[String],
    location: SourceLocation,
  ) -> Self {
    let name = name.into();
    let hint = suggestion_hint(&name, candidates);
    CompileError::UndefinedVariableAt {
      code: error_codes::UNDEFINED_VARIABLE,
      name,
      location,
      hint,
      source: None,
    }
  }
}

fn suggestion_hint(name: &str, candidates: &[String]) -> String {
  best_suggestion(name, candidates)
    .map(|s| format!(" (did you mean '{}'?)", s))
    .unwrap_or_default()
}

fn best_suggestion(name: &str, candidates: &[String]) -> Option<String> {
  let threshold = suggestion_threshold(name);
  let mut best: Option<(usize, &str)> = None;
  for cand in candidates {
    let dist = levenshtein_distance(name, cand);
    if dist == 0 || dist > threshold {
      continue;
    }
    match best {
      None => best = Some((dist, cand.as_str())),
      Some((best_dist, best_name)) => {
        let cand_len = cand.chars().count();
        let best_len = best_name.chars().count();
        let cand_key = (dist, cand_len, cand.as_str());
        let best_key = (best_dist, best_len, best_name);
        if cand_key < best_key {
          best = Some((dist, cand.as_str()));
        }
      }
    }
  }
  best.map(|(_, name)| name.to_string())
}

fn suggestion_threshold(name: &str) -> usize {
  let len = name.chars().count();
  if len <= 3 {
    1
  } else if len <= 6 {
    2
  } else {
    3
  }
}

fn levenshtein_distance(a: &str, b: &str) -> usize {
  // HIGH: Levenshtein 거리 바이트 기반 계산 수정
  // UTF-8 멀티바이트 문자를 올바르게 처리하기 위해 문자 기반으로 계산
  let a_chars: Vec<char> = a.chars().collect();
  let b_chars: Vec<char> = b.chars().collect();
  if a_chars.is_empty() {
    return b_chars.len();
  }
  if b_chars.is_empty() {
    return a_chars.len();
  }
  let b_len = b_chars.len();

  // 동적 프로그래밍 테이블 초기화
  let mut prev = vec![0; b_len + 1];
  let mut curr = vec![0; b_len + 1];
  for (j, prev_val) in prev.iter_mut().enumerate().take(b_len + 1) {
    *prev_val = j;
  }

  for (i, &a_ch) in a_chars.iter().enumerate() {
    curr[0] = i + 1;
    for (j, &b_ch) in b_chars.iter().enumerate() {
      let cost = usize::from(a_ch != b_ch);
      let insert = curr[j] + 1;
      let delete = prev[j + 1] + 1;
      let replace = prev[j] + cost;
      curr[j + 1] = insert.min(delete).min(replace);
    }
    prev.clone_from(&curr);
  }
  prev[b_len]
}

// ============================================================================
// Extension methods for ParseError
// ============================================================================

impl ParseError {
  /// Create from syntax error
  pub fn from_syntax(msg: impl Into<String>) -> Self {
    Self::from_syntax_at(msg, DEFAULT_PARSE_LINE, DEFAULT_PARSE_COLUMN)
  }

  /// Create from syntax error with explicit source position.
  pub fn from_syntax_at(msg: impl Into<String>, line: usize, column: usize) -> Self {
    let message = msg.into();
    let normalized = normalize_parse_summary(&message);
    let (line, column) = normalize_parse_position(line, column);
    ParseError::Parse {
      code: error_codes::PARSE_ERROR,
      message: normalized,
      line,
      column,
    }
  }

  /// Create from unsupported syntax
  pub fn from_unsupported(msg: impl Into<String>) -> Self {
    Self::from_unsupported_at(msg, DEFAULT_PARSE_LINE, DEFAULT_PARSE_COLUMN)
  }

  /// Create from unsupported syntax with explicit source position.
  pub fn from_unsupported_at(msg: impl Into<String>, line: usize, column: usize) -> Self {
    let message = msg.into();
    let normalized = normalize_parse_summary(&message);
    let (line, column) = normalize_parse_position(line, column);
    ParseError::UnexpectedToken {
      code: error_codes::UNEXPECTED_TOKEN,
      expected: "valid syntax".to_string(),
      found: normalize_unsupported_summary(&normalized),
      line,
      column,
    }
  }

  /// Create empty input error
  pub fn empty_input() -> Self {
    ParseError::UnexpectedEof {
      code: error_codes::UNEXPECTED_EOF,
      expected: "expression".to_string(),
      line: DEFAULT_PARSE_LINE,
      column: DEFAULT_PARSE_COLUMN,
    }
  }
}

// ============================================================================
// Unified Error Hierarchy
// ============================================================================

/// Top-level unified error type that wraps all error categories.
///
/// This provides a single error type for propagating errors across module
/// boundaries while preserving the original error information.
///
/// # Usage
///
/// ```rust,ignore
/// use pnix_core::diagnostics::pnix_error::PnixError;
///
/// fn some_operation() -> Result<(), PnixError> {
///     // ParseError automatically converts to PnixError
///     let _result = parse_something()?;
///     Ok(())
/// }
/// ```
///
/// # Example
/// ```rust
/// use pnix_core::diagnostics::{ParseError, PnixError};
/// let err = PnixError::Parse(ParseError::from_syntax("unexpected"));
/// assert!(matches!(err, PnixError::Parse(_)));
/// ```
/// 통합 에러 타입: 모든 에러를 통합하는 최상위 에러 타입
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum PnixError {
  /// Parse errors (syntax parsing)
  #[error("Parse error: {0}")]
  Parse(
    /// 파싱 에러
    #[source]
    ParseError,
  ),

  /// Tokenize errors (lexical analysis)
  #[error("Tokenize error: {0}")]
  Tokenize(
    /// 토큰화 에러
    #[source]
    TokenizeError,
  ),

  /// Compile errors (compilation phase)
  #[error("Compile error: {0}")]
  Compile(
    /// 컴파일 에러
    #[source]
    CompileError,
  ),

  /// Language-specific errors
  #[error("Pnix error: {0}")]
  Pnix(
    /// Pnix 언어 에러
    #[source]
    LangPnixError,
  ),

  #[error("Python error: {0}")]
  Python(
    /// Python 에러
    #[source]
    PythonError,
  ),

  #[error("GH codegen error: {0}")]
  GhCodegen(
    /// GH 코드 생성 에러
    #[source]
    GhCodegenError,
  ),

  /// IO errors
  #[error("IO error: {0}")]
  Io(
    /// IO 에러 메시지
    String,
  ),

  /// Error with attached contextual message while preserving original category
  #[error("{context}: {source}")]
  Context {
    /// 컨텍스트 메시지
    context: String,
    /// 원본 에러
    #[source]
    source: Box<PnixError>,
  },

  /// Generic error with message
  #[error("{0}")]
  Other(
    /// 일반 에러 메시지
    String,
  ),
}

impl From<ParseError> for PnixError {
  fn from(e: ParseError) -> Self {
    PnixError::Parse(e)
  }
}

impl From<LexerTokenizeError> for PnixError {
  fn from(e: LexerTokenizeError) -> Self {
    PnixError::Tokenize(e)
  }
}

impl From<CompileError> for PnixError {
  fn from(e: CompileError) -> Self {
    PnixError::Compile(e)
  }
}

impl From<LangPnixError> for PnixError {
  fn from(e: LangPnixError) -> Self {
    PnixError::Pnix(e)
  }
}

impl From<PythonError> for PnixError {
  fn from(e: PythonError) -> Self {
    PnixError::Python(e)
  }
}

impl From<GhCodegenError> for PnixError {
  fn from(e: GhCodegenError) -> Self {
    PnixError::GhCodegen(e)
  }
}

impl From<String> for PnixError {
  fn from(s: String) -> Self {
    PnixError::Other(s)
  }
}

impl From<&str> for PnixError {
  fn from(s: &str) -> Self {
    PnixError::Other(s.to_string())
  }
}

/// Result type alias for PnixError
pub type PnixResult<T> = Result<T, PnixError>;

// ============================================================================
// Error Conversion Traits
// ============================================================================

/// 로컬 에러 타입을 표준 PnixError 타입으로 변환하는 트레잇
///
/// 모듈별 에러 타입에 이 트레잇을 구현하면 `?` 연산자를 사용하여
/// 에러를 자동으로 전파할 수 있습니다.
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
/// 이 에러를 PnixError로 변환하는 트레잇
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub trait IntoPnixError {
  /// 이 에러를 PnixError로 변환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변환만, 값 계산 없음
  fn into_pnix_error(self) -> PnixError;
}

/// 에러에 컨텍스트를 추가하는 트레잇
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub trait ErrorContext<T> {
  /// 에러에 컨텍스트 메시지 추가
  #[allow(clippy::result_large_err)]
  fn with_context<F, S>(self, f: F) -> Result<T, PnixError>
  where
    F: FnOnce() -> S,
    S: Into<String>;
}

impl<T, E: Into<PnixError>> ErrorContext<T> for Result<T, E> {
  #[allow(clippy::result_large_err)]
  fn with_context<F, S>(self, f: F) -> Result<T, PnixError>
  where
    F: FnOnce() -> S,
    S: Into<String>,
  {
    self.map_err(|e| {
      let pnix_err = e.into();
      PnixError::Context {
        context: f().into(),
        source: Box::new(pnix_err),
      }
    })
  }
}

// ============================================================================
// Error Display Helpers
// ============================================================================

/// 디버깅을 위해 전체 컨텍스트와 함께 에러 포맷팅
///
/// ## 헌법 준수 (P0-1, C1)
///
/// 텍스트 생성만, 파일 I/O 없음
pub fn format_error_with_context(error: &PnixError) -> String {
  match error {
    PnixError::Parse(e) => format!("[Parse] {}", e),
    PnixError::Tokenize(e) => format!("[Tokenize] {}", e),
    PnixError::Compile(e) => format!("[Compile] {}", e),
    PnixError::Pnix(e) => format!("[Pnix] {}", e),
    PnixError::Python(e) => format!("[Python] {}", e),
    PnixError::GhCodegen(e) => format!("[GhCodegen] {}", e),
    PnixError::Io(msg) => format!("[IO] {}", msg),
    PnixError::Context { context, source } => {
      format!(
        "[Context] {}: {}",
        context,
        format_error_with_context(source)
      )
    }
    PnixError::Other(msg) => format!("[Error] {}", msg),
  }
}

/// 에러가 복구 가능한지 확인 (치명적이지 않은 에러)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 분석만, 값 계산 없음
pub fn is_recoverable_error(error: &PnixError) -> bool {
  matches!(
    error,
    PnixError::Parse(_) | PnixError::Tokenize(_) | PnixError::Pnix(LangPnixError::Parse { .. })
  ) || matches!(error, PnixError::Context { source, .. } if is_recoverable_error(source))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_pnix_error_from_parse_error() {
    let parse_err = ParseError::empty_input();
    let pnix_err: PnixError = parse_err.into();
    assert!(matches!(pnix_err, PnixError::Parse(_)));
  }

  #[test]
  fn test_pnix_error_from_tokenize_error() {
    use crate::lexer::TokenizeError;
    let tokenize_err = TokenizeError::UnclosedString {
      code: error_codes::UNCLOSED_STRING,
    };
    let pnix_err: PnixError = tokenize_err.into();
    assert!(matches!(pnix_err, PnixError::Tokenize(_)));
  }

  #[test]
  fn test_pnix_error_from_compile_error() {
    let compile_err = CompileError::Compile {
      code: error_codes::COMPILE_ERROR,
      message: "test error".to_string(),
      source: None,
    };
    let pnix_err: PnixError = compile_err.into();
    assert!(matches!(pnix_err, PnixError::Compile(_)));
  }

  #[test]
  fn test_pnix_error_from_string() {
    let pnix_err: PnixError = "custom error".into();
    assert!(matches!(pnix_err, PnixError::Other(_)));
    assert_eq!(pnix_err.to_string(), "custom error");
  }

  #[test]
  fn test_pnix_error_from_io_message() {
    let pnix_err = PnixError::Io("disk failure".to_string());
    assert!(matches!(pnix_err, PnixError::Io(_)));
    assert!(pnix_err.to_string().contains("disk failure"));
  }

  #[test]
  fn test_compile_error_conversion_helpers() {
    let err = CompileError::from_ast_to_ct("invalid node", None);
    assert!(matches!(err, CompileError::Compile { .. }));

    let loc = SourceLocation {
      file: None,
      line: 10,
      column: 5,
    };
    let err = CompileError::from_ast_to_ct("invalid node", Some(loc));
    assert!(matches!(err, CompileError::CompileAt { .. }));

    let err = CompileError::from_ct_optimization("failed", "fusion");
    assert!(err.to_string().contains("fusion"));

    let err = CompileError::from_type_inference("cannot infer", "x + y");
    assert!(matches!(err, CompileError::TypeError { .. }));
  }

  #[test]
  fn test_parse_error_conversion_helpers() {
    let err = ParseError::from_syntax("unexpected token");
    assert!(matches!(err, ParseError::Parse { .. }));

    let err = ParseError::from_syntax_at("unexpected token", 2, 5);
    match err {
      ParseError::Parse { line, column, .. } => {
        assert_eq!(line, 2);
        assert_eq!(column, 5);
      }
      other => panic!("expected Parse variant, got {:?}", other),
    }

    let err = ParseError::from_unsupported("feature X");
    assert!(matches!(err, ParseError::UnexpectedToken { .. }));

    let err = ParseError::from_unsupported_at("feature X", 4, 8);
    match err {
      ParseError::UnexpectedToken { line, column, .. } => {
        assert_eq!(line, 4);
        assert_eq!(column, 8);
      }
      other => panic!("expected UnexpectedToken variant, got {:?}", other),
    }

    let err = ParseError::empty_input();
    assert!(matches!(err, ParseError::UnexpectedEof { .. }));
  }

  #[test]
  fn test_parse_error_helper_default_position_is_one_based() {
    let syntax = ParseError::from_syntax("oops");
    let unsupported = ParseError::from_unsupported("feature X");
    let empty = ParseError::empty_input();

    match syntax {
      ParseError::Parse { line, column, .. } => {
        assert_eq!(line, DEFAULT_PARSE_LINE);
        assert_eq!(column, DEFAULT_PARSE_COLUMN);
      }
      other => panic!("expected Parse variant, got {:?}", other),
    }

    match unsupported {
      ParseError::UnexpectedToken { line, column, .. } => {
        assert_eq!(line, DEFAULT_PARSE_LINE);
        assert_eq!(column, DEFAULT_PARSE_COLUMN);
      }
      other => panic!("expected UnexpectedToken variant, got {:?}", other),
    }

    match empty {
      ParseError::UnexpectedEof { line, column, .. } => {
        assert_eq!(line, DEFAULT_PARSE_LINE);
        assert_eq!(column, DEFAULT_PARSE_COLUMN);
      }
      other => panic!("expected UnexpectedEof variant, got {:?}", other),
    }
  }

  #[test]
  fn test_parse_error_position_helper_normalizes_zero_to_one_based() {
    let syntax = ParseError::from_syntax_at("oops", 0, 0);
    match syntax {
      ParseError::Parse { line, column, .. } => {
        assert_eq!(line, DEFAULT_PARSE_LINE);
        assert_eq!(column, DEFAULT_PARSE_COLUMN);
      }
      other => panic!("expected Parse variant, got {:?}", other),
    }

    let unsupported = ParseError::from_unsupported_at("feature X", 0, 3);
    match unsupported {
      ParseError::UnexpectedToken { line, column, .. } => {
        assert_eq!(line, DEFAULT_PARSE_LINE);
        assert_eq!(column, 3);
      }
      other => panic!("expected UnexpectedToken variant, got {:?}", other),
    }
  }

  #[test]
  fn test_parse_error_display_format_is_standardized() {
    let parse = ParseError::Parse {
      code: error_codes::PARSE_ERROR,
      message: "unexpected token".to_string(),
      line: 3,
      column: 7,
    };
    assert_eq!(
      parse.to_string(),
      "[E0001] Parse error at line 3:7: unexpected token"
    );

    let unexpected = ParseError::UnexpectedToken {
      code: error_codes::UNEXPECTED_TOKEN,
      expected: "identifier".to_string(),
      found: "}".to_string(),
      line: 2,
      column: 9,
    };
    assert_eq!(
      unexpected.to_string(),
      "[E0002] Parse error at line 2:9: expected identifier, found }"
    );

    let expected = ParseError::ExpectedToken {
      code: error_codes::EXPECTED_TOKEN,
      expected: "identifier".to_string(),
      found: "}".to_string(),
      line: 4,
      column: 5,
    };
    assert_eq!(
      expected.to_string(),
      "[E0003] Parse error at line 4:5: expected identifier, found }"
    );

    let eof = ParseError::UnexpectedEof {
      code: error_codes::UNEXPECTED_EOF,
      expected: "]".to_string(),
      line: 6,
      column: 1,
    };
    assert_eq!(
      eof.to_string(),
      "[E0004] Parse error at line 6:1: expected ], found EOF"
    );

    let deep = ParseError::NestingTooDeep {
      code: error_codes::NESTING_TOO_DEEP,
      line: 8,
      column: 2,
    };
    assert_eq!(
      deep.to_string(),
      "[E0005] Parse error at line 8:2: nesting too deep"
    );
  }

  #[test]
  fn test_parse_error_helper_normalizes_multiline_summary() {
    let err = ParseError::from_syntax("  unexpected\n token\t in input  ");
    assert_eq!(
      err.to_string(),
      "[E0001] Parse error at line 1:1: unexpected token in input"
    );
  }

  #[test]
  fn test_parse_error_helper_fallback_for_whitespace_only_message() {
    let err = ParseError::from_syntax(" \n\t ");
    assert_eq!(
      err.to_string(),
      "[E0001] Parse error at line 1:1: unspecified parse error"
    );
  }

  #[test]
  fn test_parse_error_unsupported_helper_normalizes_summary() {
    let err = ParseError::from_unsupported("  experimental\n form\t ");
    match err {
      ParseError::UnexpectedToken { found, .. } => {
        assert_eq!(found, "unsupported: experimental form");
      }
      other => panic!("expected UnexpectedToken variant, got {:?}", other),
    }
  }

  #[test]
  fn test_parse_error_unsupported_helper_truncates_prefixed_summary() {
    let long = "x".repeat(MAX_PARSE_SUMMARY_CHARS + 80);
    let err = ParseError::from_unsupported(long);
    match err {
      ParseError::UnexpectedToken { found, .. } => {
        assert!(found.starts_with(UNSUPPORTED_PARSE_PREFIX));
        assert_eq!(found.chars().count(), MAX_PARSE_SUMMARY_CHARS);
        assert!(found.ends_with(PARSE_SUMMARY_TRUNCATED_SUFFIX));
      }
      other => panic!("expected UnexpectedToken variant, got {:?}", other),
    }
  }

  #[test]
  fn test_parse_error_helper_strips_control_characters() {
    let err = ParseError::from_syntax("bad\u{0000}token\u{007f}here");
    assert_eq!(
      err.to_string(),
      "[E0001] Parse error at line 1:1: bad token here"
    );
  }

  #[test]
  fn test_parse_error_helper_strips_ansi_escape_sequences() {
    let err = ParseError::from_syntax("\u{1b}[31munexpected\u{1b}[0m token");
    assert_eq!(
      err.to_string(),
      "[E0001] Parse error at line 1:1: unexpected token"
    );
  }

  #[test]
  fn test_levenshtein_distance_handles_unicode_scalars() {
    assert_eq!(levenshtein_distance("가나다", "가나다"), 0);
    assert_eq!(levenshtein_distance("가나다", "가다"), 1);
    assert_eq!(levenshtein_distance("변수", "변수1"), 1);
  }

  #[test]
  fn test_best_suggestion_is_deterministic_on_tie() {
    let candidates = vec!["vaule".to_string(), "valeu".to_string()];
    assert_eq!(
      best_suggestion("value", &candidates),
      Some("valeu".to_string())
    );
  }

  #[test]
  fn test_undefined_variable_hint_prefers_closest_candidate() {
    let candidates = vec!["value".to_string(), "count".to_string()];
    let err = CompileError::undefined_variable("valeu", &candidates);
    match err {
      CompileError::UndefinedVariable { hint, .. } => {
        assert_eq!(hint, " (did you mean 'value'?)");
      }
      other => panic!("expected UndefinedVariable, got {:?}", other),
    }

    let exact = CompileError::undefined_variable("value", &candidates);
    match exact {
      CompileError::UndefinedVariable { hint, .. } => {
        assert!(hint.is_empty());
      }
      other => panic!("expected UndefinedVariable, got {:?}", other),
    }
  }

  #[test]
  fn test_format_error_with_context() {
    let err = PnixError::Compile(CompileError::Compile {
      code: error_codes::COMPILE_ERROR,
      message: "test".to_string(),
      source: None,
    });
    let formatted = format_error_with_context(&err);
    assert!(formatted.starts_with("[Compile]"));
  }

  #[test]
  fn test_is_recoverable_error() {
    use crate::lexer::TokenizeError;
    // Recoverable errors
    assert!(is_recoverable_error(&PnixError::Parse(
      ParseError::empty_input()
    )));
    assert!(is_recoverable_error(&PnixError::Tokenize(
      TokenizeError::UnclosedString {
        code: error_codes::UNCLOSED_STRING,
      }
    )));
    assert!(is_recoverable_error(&PnixError::Pnix(
      LangPnixError::Parse {
        code: error_codes::PARSE_ERROR,
        message: "test".to_string(),
        line: 1,
        column: 1,
      }
    )));

    // Non-recoverable errors
    assert!(!is_recoverable_error(&PnixError::Compile(
      CompileError::Compile {
        code: error_codes::COMPILE_ERROR,
        message: "error".to_string(),
        source: None,
      }
    )));
  }

  #[test]
  fn test_error_context_trait() {
    fn may_fail() -> Result<i32, ParseError> {
      Err(ParseError::empty_input())
    }

    let result: Result<i32, PnixError> = may_fail().with_context(|| "during parsing");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, PnixError::Context { .. }));
    assert!(err.to_string().contains("parsing"));
  }

  #[test]
  fn test_error_context_preserves_recoverable_error_category() {
    fn may_fail() -> Result<i32, ParseError> {
      Err(ParseError::empty_input())
    }

    let err = may_fail()
      .with_context(|| "while parsing module")
      .expect_err("expected parse error");
    assert!(is_recoverable_error(&err));
  }

  #[test]
  fn test_pnix_result_type_alias() {
    #[allow(clippy::result_large_err)]
    fn returns_pnix_result() -> PnixResult<i32> {
      Ok(42)
    }

    #[allow(clippy::result_large_err)]
    fn returns_pnix_result_err() -> PnixResult<i32> {
      Err(PnixError::Other("test".to_string()))
    }

    assert_eq!(returns_pnix_result().unwrap(), 42);
    assert!(returns_pnix_result_err().is_err());
  }

  #[test]
  fn test_parse_error_helper_truncates_overlong_summary() {
    let long = "a".repeat(MAX_PARSE_SUMMARY_CHARS + 50);
    let err = ParseError::from_syntax(long);
    match err {
      ParseError::Parse { message, .. } => {
        assert_eq!(message.chars().count(), MAX_PARSE_SUMMARY_CHARS);
        assert!(message.ends_with(PARSE_SUMMARY_TRUNCATED_SUFFIX));
      }
      other => panic!("expected Parse variant, got {:?}", other),
    }
  }

  #[test]
  fn test_parse_error_helper_preserves_summary_at_length_limit() {
    let exact = "b".repeat(MAX_PARSE_SUMMARY_CHARS);
    let err = ParseError::from_syntax(exact.clone());
    match err {
      ParseError::Parse { message, .. } => {
        assert_eq!(message, exact);
      }
      other => panic!("expected Parse variant, got {:?}", other),
    }
  }
}
