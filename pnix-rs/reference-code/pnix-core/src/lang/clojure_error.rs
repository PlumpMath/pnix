//! Clojure Error Types
//!
//! ## 헌법 준수 (P0-1)
//!
//! 에러 타입 정의만, 실행 없음

use std::fmt;

/// Clojure 언어 에러: Clojure 언어 처리 중 발생하는 에러 타입
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClojureError {
  /// 파싱 에러: Clojure 코드 파싱 중 발생한 에러
  Parse(
    /// 에러 메시지
    String,
  ),
  /// 지원하지 않는 문법: 지원하지 않는 Clojure 문법
  UnsupportedSyntax(
    /// 에러 메시지
    String,
  ),
  /// 알 수 없는 심볼: 정의되지 않은 심볼 참조
  UnknownSymbol(
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
  /// 매크로 확장 에러: 매크로 확장 중 발생한 에러 (JVM interop용 - 비활성)
  MacroExpansion(
    /// 에러 메시지
    String,
  ),
  /// 스레드 에러: 스레드 관련 에러 (pnix-thread용)
  ThreadError(
    /// 에러 메시지
    String,
  ),
  /// 타임아웃 에러: 작업 타임아웃
  Timeout(
    /// 에러 메시지
    String,
  ),
}

impl fmt::Display for ClojureError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Parse(msg) => write!(f, "Parse error: {}", msg),
      Self::UnsupportedSyntax(msg) => write!(f, "Unsupported syntax: {}", msg),
      Self::UnknownSymbol(msg) => write!(f, "Unknown symbol: {}", msg),
      Self::TypeError(msg) => write!(f, "Type error: {}", msg),
      Self::Lowering(msg) => write!(f, "Lowering error: {}", msg),
      Self::MacroExpansion(msg) => write!(f, "Macro expansion error: {}", msg),
      Self::ThreadError(msg) => write!(f, "Thread error: {}", msg),
      Self::Timeout(msg) => write!(f, "Timeout: {}", msg),
    }
  }
}

impl std::error::Error for ClojureError {}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_clojure_error_display() {
    let err = ClojureError::Parse("syntax error".to_string());
    assert!(format!("{}", err).contains("Parse error"));
  }

  #[test]
  fn test_thread_error() {
    let err = ClojureError::ThreadError("join failed".to_string());
    assert!(format!("{}", err).contains("Thread error"));
  }

  #[test]
  fn test_timeout_error() {
    let err = ClojureError::Timeout("5000ms exceeded".to_string());
    assert!(format!("{}", err).contains("Timeout"));
  }
}
