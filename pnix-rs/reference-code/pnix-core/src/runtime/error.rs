//! Runtime 에러 타입
//!
//! pnix-old의 pnix_runtime/src/runtime.rs에서 마이그레이션.

use thiserror::Error;

/// 런타임 에러
///
/// # Example
/// ```rust
/// use pnix_core::runtime::RuntimeError;
/// let err = RuntimeError::Parse("invalid input".to_string());
/// assert!(matches!(err, RuntimeError::Parse(_)));
/// ```
/// 런타임 에러: 런타임 실행 중 발생하는 에러 타입
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum RuntimeError {
  #[error("Symbolic error: {0}")]
  Symbolic(
    /// 에러 메시지
    String,
  ),

  #[error("Parse error: {0}")]
  Parse(
    /// 에러 메시지
    String,
  ),

  #[error("Evaluation error: {0}")]
  Eval(
    /// 에러 메시지
    String,
  ),

  #[error("Binding not found: {0}")]
  BindingNotFound(
    /// 바인딩 이름
    String,
  ),
}
