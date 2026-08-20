//! Morphism 에러 타입
//!
//! pnix-old의 ct_morphism/src/lib.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 순수 에러 타입 정의, 값 계산 없음

use serde::{Deserialize, Serialize};

/// Morphism 관련 에러
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MorphismError {
  /// Morphism을 찾을 수 없음
  NotFound(String),
  /// 잘못된 입력
  InvalidInput(String),
  /// 실행 실패
  ExecutionFailed(String),
  /// 타입 불일치
  TypeMismatch { expected: String, got: String },
  /// Lock 중독 (멀티스레딩 에러)
  LockPoisoned(String),
  /// 트랜잭션 충돌
  TransactionConflict(String),
  /// 스키마 검증 실패 (Phase 5.1)
  SchemaValidation {
    schema: String,
    value: String,
    message: String,
  },
}

impl std::fmt::Display for MorphismError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      MorphismError::NotFound(name) => write!(f, "Morphism not found: {}", name),
      MorphismError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
      MorphismError::ExecutionFailed(msg) => write!(f, "Execution failed: {}", msg),
      MorphismError::TypeMismatch { expected, got } => {
        write!(f, "Type mismatch: expected {}, got {}", expected, got)
      }
      MorphismError::LockPoisoned(msg) => write!(f, "Lock poisoned: {}", msg),
      MorphismError::TransactionConflict(msg) => {
        write!(f, "Transaction conflict: {}", msg)
      }
      MorphismError::SchemaValidation {
        schema,
        value,
        message,
      } => {
        write!(
          f,
          "Schema validation failed: {} (schema: {}, value: {})",
          message, schema, value
        )
      }
    }
  }
}

impl std::error::Error for MorphismError {}
