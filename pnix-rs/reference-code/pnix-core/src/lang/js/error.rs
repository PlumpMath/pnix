//! JavaScript/TypeScript 에러 타입 정의
//!
//! pnix-old의 lang_js/error.rs에서 마이그레이션.
//!
//! 헌법 준수: 에러 타입 정의만, 실행 로직 없음

use thiserror::Error;

/// JavaScript/TypeScript 파싱/변환 에러: JavaScript/TypeScript 코드 처리 중 발생하는 에러 타입
#[derive(Debug, Error, Clone)]
pub enum JsError {
  /// 파싱 에러: JavaScript/TypeScript 코드 파싱 중 발생한 에러
  #[error("Parse error: {0}")]
  Parse(
    /// 에러 메시지
    String,
  ),

  /// 지원하지 않는 문법: 지원하지 않는 JavaScript/TypeScript 문법
  #[error("Unsupported syntax: {0}")]
  UnsupportedSyntax(
    /// 에러 메시지
    String,
  ),

  /// 타입 에러: 타입 관련 에러
  #[error("Type error: {0}")]
  TypeError(
    /// 에러 메시지
    String,
  ),

  /// Lowering 에러: UnifiedExpr로 lowering 중 발생한 에러
  #[error("Lowering error: {0}")]
  Lowering(
    /// 에러 메시지
    String,
  ),
}
