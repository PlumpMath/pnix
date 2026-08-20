//! 언어별 에러 타입 정의
//!
//! pnix-old의 lang_pnix/src/error.rs, lang_python/src/error.rs, pnix_gh_codegen/src/error.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 에러 타입 정의만, 실행 없음

use std::fmt;

use super::error_code::ErrorCode;

/// Pnix 언어 에러: Pnix 언어 처리 중 발생하는 에러 타입
///
/// # Example
/// ```rust
/// use pnix_core::diagnostics::{error_codes, LangPnixError};
/// let err = LangPnixError::Parse {
///     code: error_codes::PARSE_ERROR,
///     message: "unexpected token".to_string(),
///     line: 1,
///     column: 1,
/// };
/// assert!(matches!(err, LangPnixError::Parse { .. }));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PnixError {
  /// 파싱 에러: Pnix 코드 파싱 중 발생한 에러
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
  /// 지원하지 않는 문법: 지원하지 않는 Pnix 문법
  UnsupportedSyntax(
    /// 에러 메시지
    String,
  ),
  /// 알 수 없는 빌트인: 정의되지 않은 빌트인 함수 참조
  UnknownBuiltin(
    /// 에러 메시지
    String,
  ),
  /// 타입 에러: 타입 관련 에러
  TypeError(
    /// 에러 메시지
    String,
  ),
  /// Lowering 에러: UnifiedExpr로 lowering 중 발생한 에러
  Lowering(
    /// 에러 메시지
    String,
  ),
  /// 파일을 찾을 수 없음: 요청한 파일을 찾을 수 없음
  FileNotFound(
    /// 파일 경로
    String,
  ),
  /// IO 에러: 파일 I/O 중 발생한 에러
  IoError(
    /// 에러 메시지
    String,
  ),
  /// 순환 의존성: 모듈 간 순환 의존성 감지
  CyclicDependency(
    /// 에러 메시지
    String,
  ),
}

impl fmt::Display for PnixError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Parse {
        code,
        message,
        line,
        column,
      } => {
        write!(
          f,
          "[{}] Parse error at line {}:{}: {}",
          code, line, column, message
        )
      }
      Self::UnsupportedSyntax(msg) => write!(f, "Unsupported syntax: {}", msg),
      Self::UnknownBuiltin(msg) => write!(f, "Unknown builtin: {}", msg),
      Self::TypeError(msg) => write!(f, "Type error: {}", msg),
      Self::Lowering(msg) => write!(f, "Lowering error: {}", msg),
      Self::FileNotFound(msg) => write!(f, "File not found: {}", msg),
      Self::IoError(msg) => write!(f, "IO error: {}", msg),
      Self::CyclicDependency(msg) => write!(f, "Cyclic dependency: {}", msg),
    }
  }
}

impl std::error::Error for PnixError {}

/// Python 언어 에러: Python 언어 처리 중 발생하는 에러 타입
///
/// # Example
/// ```rust
/// use pnix_core::diagnostics::PythonError;
/// let err = PythonError::UnsupportedSyntax("def".to_string());
/// assert!(matches!(err, PythonError::UnsupportedSyntax(_)));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PythonError {
  /// 파싱 에러: Python 코드 파싱 중 발생한 에러
  ParseError(
    /// 에러 메시지
    String,
  ),
  /// 지원하지 않는 문법: 지원하지 않는 Python 문법
  UnsupportedSyntax(
    /// 에러 메시지
    String,
  ),
  /// Lowering 에러: UnifiedExpr로 lowering 중 발생한 에러
  LoweringError(
    /// 에러 메시지
    String,
  ),
  /// Pnix 에러: Pnix 변환 중 발생한 에러
  PnixError(
    /// 에러 메시지
    String,
  ),
}

impl fmt::Display for PythonError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::ParseError(msg) => write!(f, "Parse error: {}", msg),
      Self::UnsupportedSyntax(msg) => write!(f, "Unsupported syntax: {}", msg),
      Self::LoweringError(msg) => write!(f, "Lowering error: {}", msg),
      Self::PnixError(msg) => write!(f, "Lang_pnix error: {}", msg),
    }
  }
}

impl std::error::Error for PythonError {}

/// JavaScript/TypeScript 언어 에러: JavaScript/TypeScript 언어 처리 중 발생하는 에러 타입
///
/// # Example
/// ```rust
/// use pnix_core::diagnostics::JsError;
/// let err = JsError::TypeError("expected number".to_string());
/// assert!(matches!(err, JsError::TypeError(_)));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JsError {
  /// 파싱 에러: JavaScript/TypeScript 코드 파싱 중 발생한 에러
  ParseError(
    /// 에러 메시지
    String,
  ),
  /// 지원하지 않는 문법: 지원하지 않는 JavaScript/TypeScript 문법
  UnsupportedSyntax(
    /// 에러 메시지
    String,
  ),
  /// 타입 에러: 타입 관련 에러
  TypeError(
    /// 에러 메시지
    String,
  ),
  /// Lowering 에러: UnifiedExpr로 lowering 중 발생한 에러
  LoweringError(
    /// 에러 메시지
    String,
  ),
}

impl fmt::Display for JsError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::ParseError(msg) => write!(f, "Parse error: {}", msg),
      Self::UnsupportedSyntax(msg) => write!(f, "Unsupported syntax: {}", msg),
      Self::TypeError(msg) => write!(f, "Type error: {}", msg),
      Self::LoweringError(msg) => write!(f, "Lowering error: {}", msg),
    }
  }
}

impl std::error::Error for JsError {}

/// GH 코드 생성 에러: GH 코드 생성 중 발생하는 에러 타입
///
/// # Example
/// ```rust
/// use pnix_core::diagnostics::GhCodegenError;
/// let err = GhCodegenError::MissingInput { input: "x".to_string(), context: None };
/// assert!(matches!(err, GhCodegenError::MissingInput { .. }));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GhCodegenError {
  /// 필수 입력 누락: 필수 입력이 제공되지 않음
  MissingInput {
    /// 입력 이름 (누락된 입력 이름)
    input: String,
    /// 컨텍스트 (선택적, 에러 컨텍스트)
    context: Option<String>,
  },
  /// 지원하지 않는 연산: 지원하지 않는 연산 사용
  UnsupportedOperation {
    /// 연산 이름 (지원하지 않는 연산 이름)
    operation: String,
    /// 컨텍스트 (선택적, 에러 컨텍스트)
    context: Option<String>,
  },
  /// 변환 에러: 코드 변환 중 발생한 에러
  ConversionError {
    /// 에러 메시지
    message: String,
    /// 컨텍스트 (선택적, 에러 컨텍스트)
    context: Option<String>,
  },
}

impl GhCodegenError {
  /// 컨텍스트 추가
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변경만, 값 계산 없음
  pub fn with_context(self, context: impl Into<String>) -> Self {
    match self {
      Self::MissingInput { input, .. } => Self::MissingInput {
        input,
        context: Some(context.into()),
      },
      Self::UnsupportedOperation { operation, .. } => Self::UnsupportedOperation {
        operation,
        context: Some(context.into()),
      },
      Self::ConversionError { message, .. } => Self::ConversionError {
        message,
        context: Some(context.into()),
      },
    }
  }
}

impl fmt::Display for GhCodegenError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::MissingInput { input, context } => {
        write!(f, "Missing required input: {}", input)?;
        if let Some(ctx) = context {
          write!(f, " (context: {})", ctx)?;
        }
        Ok(())
      }
      Self::UnsupportedOperation { operation, context } => {
        write!(f, "Unsupported operation: {}", operation)?;
        if let Some(ctx) = context {
          write!(f, " (context: {})", ctx)?;
        }
        Ok(())
      }
      Self::ConversionError { message, context } => {
        write!(f, "Conversion error: {}", message)?;
        if let Some(ctx) = context {
          write!(f, " (context: {})", ctx)?;
        }
        Ok(())
      }
    }
  }
}

impl std::error::Error for GhCodegenError {}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_pnix_error_display() {
    let err = PnixError::Parse {
      code: crate::diagnostics::error_codes::PARSE_ERROR,
      message: "syntax error".to_string(),
      line: 1,
      column: 2,
    };
    assert!(format!("{}", err).contains("Parse error"));
  }

  #[test]
  fn test_python_error_display() {
    let err = PythonError::ParseError("invalid syntax".to_string());
    assert!(format!("{}", err).contains("Parse error"));
  }

  #[test]
  fn test_gh_codegen_error_display() {
    let err = GhCodegenError::MissingInput {
      input: "node".to_string(),
      context: None,
    };
    assert!(format!("{}", err).contains("Missing required input"));
  }

  #[test]
  fn test_gh_codegen_error_with_context() {
    let err = GhCodegenError::MissingInput {
      input: "node".to_string(),
      context: None,
    };
    let err = err.with_context("test context");
    match err {
      GhCodegenError::MissingInput { context, .. } => {
        assert_eq!(context, Some("test context".to_string()));
      }
      _ => panic!("Expected MissingInput"),
    }
  }

  #[test]
  fn test_js_error_display() {
    let err = JsError::ParseError("syntax error".to_string());
    assert!(format!("{}", err).contains("Parse error"));
  }
}
