//! PNIX 에러 타입 정의
//!
//! pnix-old의 lang_pnix/error.rs에서 마이그레이션.
//!
//! 헌법 준수: 에러 타입 정의만, 실행 로직 없음

use crate::diagnostics::{error_codes, line_col_at, ErrorCode, Span};
use thiserror::Error;

/// Pnix 파싱/변환 에러: Pnix 언어 처리 중 발생하는 에러 타입
#[derive(Debug, Error, Clone)]
pub enum PnixError {
  /// 파싱 에러: Pnix 코드 파싱 중 발생한 에러
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
    /// 소스 위치
    span: Span,
  },

  /// 지원하지 않는 문법: 지원하지 않는 Pnix 문법
  #[error("Unsupported syntax: {message}")]
  UnsupportedSyntax {
    /// 에러 메시지
    message: String,
    /// 소스 위치 (선택적)
    span: Option<Span>,
  },

  /// 알 수 없는 빌트인: 정의되지 않은 빌트인 함수 참조
  #[error("Unknown builtin: {message}")]
  UnknownBuiltin {
    /// 에러 메시지
    message: String,
    /// 소스 위치 (선택적)
    span: Option<Span>,
  },

  /// 타입 에러: 타입 관련 에러
  #[error("Type error: {message}")]
  TypeError {
    /// 에러 메시지
    message: String,
    /// 소스 위치 (선택적)
    span: Option<Span>,
  },

  /// Lowering 에러: UnifiedExpr로 lowering 중 발생한 에러
  #[error("Lowering error: {message}")]
  Lowering {
    /// 에러 메시지
    message: String,
    /// 소스 위치 (선택적)
    span: Option<Span>,
  },
}

const DEFAULT_PARSE_LINE: usize = 1;
const DEFAULT_PARSE_COLUMN: usize = 1;
const DEFAULT_PARSE_MESSAGE: &str = "unspecified parse error";
const MAX_PARSE_MESSAGE_CHARS: usize = 240;
const PARSE_MESSAGE_TRUNCATED_SUFFIX: &str = "...";

fn normalize_parse_position(line: usize, column: usize) -> (usize, usize) {
  let normalized_line = if line == 0 { DEFAULT_PARSE_LINE } else { line };
  let normalized_column = if column == 0 {
    DEFAULT_PARSE_COLUMN
  } else {
    column
  };
  (normalized_line, normalized_column)
}

fn truncate_parse_message(message: &str) -> String {
  let len = message.chars().count();
  if len <= MAX_PARSE_MESSAGE_CHARS {
    return message.to_string();
  }

  let keep = MAX_PARSE_MESSAGE_CHARS - PARSE_MESSAGE_TRUNCATED_SUFFIX.chars().count();
  let mut truncated: String = message.chars().take(keep).collect();
  truncated.push_str(PARSE_MESSAGE_TRUNCATED_SUFFIX);
  truncated
}

fn normalize_parse_message(message: &str) -> String {
  let mut normalized = String::with_capacity(message.len());
  let mut pending_space = false;
  let mut chars = message.chars().peekable();
  while let Some(ch) = chars.next() {
    if ch == '\u{1b}' {
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
    return DEFAULT_PARSE_MESSAGE.to_string();
  }

  truncate_parse_message(&normalized)
}

impl PnixError {
  /// 라인/컬럼 위치에서 Parse 에러 생성 (에러 코드 지정 가능)
  pub fn parse_with_code_at(
    message: impl Into<String>,
    code: ErrorCode,
    line: usize,
    column: usize,
    span: Span,
  ) -> Self {
    let raw_message = message.into();
    let (line, column) = normalize_parse_position(line, column);
    let message = normalize_parse_message(&raw_message);
    PnixError::Parse {
      code,
      message,
      line,
      column,
      span,
    }
  }

  /// 라인/컬럼 위치에서 Parse 에러 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn parse_at(message: impl Into<String>, line: usize, column: usize, span: Span) -> Self {
    PnixError::parse_with_code_at(message, error_codes::PARSE_ERROR, line, column, span)
  }

  /// Span에서 line/column을 계산하여 Parse 에러 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn parse_at_span(message: impl Into<String>, span: Span, source: &str) -> Self {
    let (line, column) = line_col_at(source, span.start);
    PnixError::parse_with_code_at(message, error_codes::PARSE_ERROR, line, column, span)
  }

  /// Lowering 에러 생성 (span 없음)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn lowering(message: impl Into<String>) -> Self {
    PnixError::Lowering {
      message: message.into(),
      span: None,
    }
  }

  /// Lowering 에러 생성 (span 포함)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn lowering_with_span(message: impl Into<String>, span: Span) -> Self {
    PnixError::Lowering {
      message: message.into(),
      span: Some(span),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::diagnostics::Span;

  #[test]
  fn test_parse_error_with_span() {
    let span = Span::new(10, 20);
    let error = PnixError::parse_at("syntax error", 2, 5, span.clone());

    match error {
      PnixError::Parse {
        span: error_span,
        line,
        column,
        ..
      } => {
        assert_eq!(error_span, span);
        assert_eq!(line, 2);
        assert_eq!(column, 5);
      }
      other => panic!("Expected Parse error, got {:?}", other),
    }
  }

  #[test]
  fn test_parse_error_from_span() {
    let source = "let x = 42;\nlet y = x + z;";
    let span = Span::new(24, 25); // 'z' 위치
    let error = PnixError::parse_at_span("unknown variable", span.clone(), source);

    match error {
      PnixError::Parse {
        span: error_span,
        line,
        column,
        ..
      } => {
        assert_eq!(error_span, span);
        assert_eq!(line, 2);
        assert_eq!(column, 13); // 'z'는 두 번째 줄의 13번째 컬럼
      }
      other => panic!("Expected Parse error, got {:?}", other),
    }
  }

  #[test]
  fn test_lowering_error_without_span() {
    let error = PnixError::lowering("unsupported operation");

    match error {
      PnixError::Lowering { message, span } => {
        assert_eq!(message, "unsupported operation");
        assert_eq!(span, None);
      }
      other => panic!("Expected Lowering error, got {:?}", other),
    }
  }

  #[test]
  fn test_lowering_error_with_span() {
    let span = Span::new(5, 15);
    let error = PnixError::lowering_with_span("type mismatch", span.clone());

    match error {
      PnixError::Lowering {
        message,
        span: error_span,
      } => {
        assert_eq!(message, "type mismatch");
        assert_eq!(error_span, Some(span));
      }
      other => panic!("Expected Lowering error, got {:?}", other),
    }
  }

  #[test]
  fn test_parse_error_normalizes_message_and_position() {
    let span = Span::new(0, 0);
    let error = PnixError::parse_with_code_at(
      " \n\u{1b}[31mbad\u{1b}[0m\t token\u{0000}",
      error_codes::TOKENIZE_ERROR,
      0,
      0,
      span.clone(),
    );

    match error {
      PnixError::Parse {
        code,
        message,
        line,
        column,
        span: error_span,
      } => {
        assert_eq!(code, error_codes::TOKENIZE_ERROR);
        assert_eq!(message, "bad token");
        assert_eq!(line, 1);
        assert_eq!(column, 1);
        assert_eq!(error_span, span);
      }
      other => panic!("Expected Parse error, got {:?}", other),
    }
  }

  #[test]
  fn test_parse_error_normalizes_empty_message() {
    let span = Span::new(1, 1);
    let error = PnixError::parse_at(" \n\t ", 0, 0, span);

    match error {
      PnixError::Parse {
        message,
        line,
        column,
        ..
      } => {
        assert_eq!(message, DEFAULT_PARSE_MESSAGE);
        assert_eq!(line, 1);
        assert_eq!(column, 1);
      }
      other => panic!("Expected Parse error, got {:?}", other),
    }
  }

  #[test]
  fn test_parse_error_display_has_code_position_and_summary() {
    let span = Span::new(2, 3);
    let error =
      PnixError::parse_with_code_at("unexpected token", error_codes::EXPECTED_TOKEN, 9, 17, span);
    let rendered = error.to_string();
    assert!(rendered.contains("[E0003]"));
    assert!(rendered.contains("line 9:17"));
    assert!(rendered.contains("unexpected token"));
  }
}
