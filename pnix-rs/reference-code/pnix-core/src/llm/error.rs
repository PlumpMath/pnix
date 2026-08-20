//! # OWNER-LAW NOTE (2026-05-10)
//!
//! **pnix 는 LLM 없이 작동하는 self-sufficient deterministic AI 지능이다.**
//! 정본 헌법은 `CLAUDE.md` 의 `OWNER-LAW CONSTITUTION` section.
//!
//! 이 file 의 doc/comment 에 "LLM as X" / "LLM 이 ..." / "LLM 모드 / 연동 /
//! 호출 / 정제" 류 framing 이 남아있어도 모두 owner-law 위반으로
//! *superseded* 다. substrate 안에 LLM 자리 *없음*. 외부 LLM output 은
//! untrusted external prose 일 뿐 substrate enumeration category 가
//! 아님. 본문은 legacy framing 의 history retention.
//!
//! LLM 에러 타입 정의
//!
//! pnix-old의 pnix_llm/src/error.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 에러 타입 정의만, LLM 호출 없음
//!
//! ## 설계 철학
//!
//! LLM 관련 모든 에러를 단일 타입으로 통합하여
//! 에러 처리 로직을 일관되게 유지.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// LLM 관련 에러 타입
///
/// # Example
/// ```rust
/// use pnix_core::llm::LlmError;
/// let err = LlmError::TokenLimitExceeded { current: 120, max: 100 };
/// assert!(matches!(err, LlmError::TokenLimitExceeded { .. }));
/// ```
/// LLM 관련 에러 타입: LLM 작업 중 발생하는 에러 타입
#[derive(Debug, Error, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum LlmError {
  /// 토큰 제한 초과
  #[error("token limit exceeded: {current} > {max}")]
  TokenLimitExceeded {
    /// 현재 토큰 수
    current: usize,
    /// 최대 토큰 수
    max: usize,
  },

  /// 컨텍스트 윈도우 오버플로우
  #[error("context window overflow: {message}")]
  ContextOverflow {
    /// 에러 메시지
    message: String,
  },

  /// 모델 로드 실패
  #[error("model load failed: {model_name} - {reason}")]
  ModelLoadFailed {
    /// 모델 이름
    model_name: String,
    /// 실패 이유
    reason: String,
  },

  /// 추론(생성) 실패
  #[error("inference failed: {reason}")]
  InferenceFailed {
    /// 실패 이유
    reason: String,
  },

  /// 프롬프트 파싱 에러
  #[error("prompt parse error: {message}")]
  PromptParseError {
    /// 에러 메시지
    message: String,
  },

  /// SETO 쿼리 에러
  #[error("seto query error: {message}")]
  SetoQueryError {
    /// 에러 메시지
    message: String,
  },

  /// 메모리 관련 에러
  #[error("memory error: {message}")]
  MemoryError {
    /// 에러 메시지
    message: String,
  },

  /// 설정 에러
  #[error("config error: {message}")]
  ConfigError {
    /// 에러 메시지
    message: String,
  },

  /// 네트워크 에러 (외부 API 호출)
  #[error("network error: {message}")]
  NetworkError {
    /// 에러 메시지
    message: String,
  },

  /// 타임아웃
  #[error("operation timed out after {duration_ms}ms")]
  Timeout {
    /// 타임아웃 시간 (밀리초)
    duration_ms: u64,
  },

  /// 잠긴 도메인 접근 시도
  #[error("locked domain access: {domain}")]
  LockedDomainAccess {
    /// 도메인 이름
    domain: String,
  },

  /// 진실성 검증 실패
  #[error("truth verification failed: {statement}")]
  TruthVerificationFailed {
    /// 검증 실패한 문장
    statement: String,
  },

  /// 알 수 없는 에러
  #[error("unknown error: {message}")]
  Unknown {
    /// 에러 메시지
    message: String,
  },
}

impl LlmError {
  /// 토큰 제한 초과 에러 생성
  pub fn token_limit(current: usize, max: usize) -> Self {
    Self::TokenLimitExceeded { current, max }
  }

  /// 컨텍스트 오버플로우 에러 생성
  pub fn context_overflow(message: impl Into<String>) -> Self {
    Self::ContextOverflow {
      message: message.into(),
    }
  }

  /// 모델 로드 실패 에러 생성
  pub fn model_load(model_name: impl Into<String>, reason: impl Into<String>) -> Self {
    Self::ModelLoadFailed {
      model_name: model_name.into(),
      reason: reason.into(),
    }
  }

  /// 추론 실패 에러 생성
  pub fn inference(reason: impl Into<String>) -> Self {
    Self::InferenceFailed {
      reason: reason.into(),
    }
  }

  /// 프롬프트 파싱 에러 생성
  pub fn prompt_parse(message: impl Into<String>) -> Self {
    Self::PromptParseError {
      message: message.into(),
    }
  }

  /// SETO 쿼리 에러 생성
  pub fn seto_query(message: impl Into<String>) -> Self {
    Self::SetoQueryError {
      message: message.into(),
    }
  }

  /// 메모리 에러 생성
  pub fn memory(message: impl Into<String>) -> Self {
    Self::MemoryError {
      message: message.into(),
    }
  }

  /// 설정 에러 생성
  pub fn config(message: impl Into<String>) -> Self {
    Self::ConfigError {
      message: message.into(),
    }
  }

  /// 네트워크 에러 생성
  pub fn network(message: impl Into<String>) -> Self {
    Self::NetworkError {
      message: message.into(),
    }
  }

  /// 타임아웃 에러 생성
  pub fn timeout(duration_ms: u64) -> Self {
    Self::Timeout { duration_ms }
  }

  /// 잠긴 도메인 접근 에러 생성
  pub fn locked_domain(domain: impl Into<String>) -> Self {
    Self::LockedDomainAccess {
      domain: domain.into(),
    }
  }

  /// 진실성 검증 실패 에러 생성
  pub fn truth_verification(statement: impl Into<String>) -> Self {
    Self::TruthVerificationFailed {
      statement: statement.into(),
    }
  }

  /// 알 수 없는 에러 생성
  pub fn unknown(message: impl Into<String>) -> Self {
    Self::Unknown {
      message: message.into(),
    }
  }

  /// 에러가 재시도 가능한지 확인
  pub fn is_retryable(&self) -> bool {
    matches!(
      self,
      Self::NetworkError { .. } | Self::Timeout { .. } | Self::InferenceFailed { .. }
    )
  }

  /// 에러가 치명적인지 확인 (복구 불가)
  pub fn is_fatal(&self) -> bool {
    matches!(
      self,
      Self::ModelLoadFailed { .. } | Self::ConfigError { .. } | Self::LockedDomainAccess { .. }
    )
  }
}

/// LLM 결과 타입 별칭
pub type LlmResult<T> = Result<T, LlmError>;

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_token_limit_error() {
    let err = LlmError::token_limit(5000, 4096);
    assert!(matches!(
      err,
      LlmError::TokenLimitExceeded {
        current: 5000,
        max: 4096
      }
    ));
    assert_eq!(err.to_string(), "token limit exceeded: 5000 > 4096");
  }

  #[test]
  fn test_context_overflow_error() {
    let err = LlmError::context_overflow("too many messages");
    assert_eq!(
      err.to_string(),
      "context window overflow: too many messages"
    );
  }

  #[test]
  fn test_model_load_error() {
    let err = LlmError::model_load("gpt-4", "file not found");
    assert_eq!(err.to_string(), "model load failed: gpt-4 - file not found");
  }

  #[test]
  fn test_is_retryable() {
    assert!(LlmError::network("connection reset").is_retryable());
    assert!(LlmError::timeout(5000).is_retryable());
    assert!(LlmError::inference("OOM").is_retryable());
    assert!(!LlmError::config("invalid").is_retryable());
  }

  #[test]
  fn test_is_fatal() {
    assert!(LlmError::model_load("x", "y").is_fatal());
    assert!(LlmError::config("bad config").is_fatal());
    assert!(LlmError::locked_domain("ethics").is_fatal());
    assert!(!LlmError::network("timeout").is_fatal());
  }

  #[test]
  fn test_seto_query_error() {
    let err = LlmError::seto_query("invalid pattern");
    assert_eq!(err.to_string(), "seto query error: invalid pattern");
  }

  #[test]
  fn test_memory_error() {
    let err = LlmError::memory("entry not found");
    assert_eq!(err.to_string(), "memory error: entry not found");
  }

  #[test]
  fn test_error_equality() {
    let err1 = LlmError::token_limit(100, 50);
    let err2 = LlmError::token_limit(100, 50);
    assert_eq!(err1, err2);
  }

  #[test]
  fn test_locked_domain_access() {
    let err = LlmError::locked_domain("Politics");
    assert_eq!(err.to_string(), "locked domain access: Politics");
    assert!(err.is_fatal());
  }

  #[test]
  fn test_truth_verification() {
    let err = LlmError::truth_verification("2+2=5");
    assert_eq!(err.to_string(), "truth verification failed: 2+2=5");
  }
}
