//! Timeout 구조 정의
//!
//! pnix-old의 pnix_io_runtime/src/timeout.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 실행 로직 제외
//! - TimeoutError: 타임아웃 에러 구조 정의
//! - TimeoutIO: 타임아웃이 설정된 IO 구조 정의
//! - 실제 타임아웃 체크 및 실행 로직은 executor에서 구현

use serde::{Deserialize, Serialize};

/// 타임아웃 에러 구조
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimeoutError {
  /// 에러 메시지
  pub message: String,
  /// 타임아웃 지속 시간 (초)
  pub duration_seconds: f64,
}

impl TimeoutError {
  /// 새로운 타임아웃 에러 생성
  pub fn new(message: String, duration_seconds: f64) -> Self {
    Self {
      message,
      duration_seconds,
    }
  }
}

/// 타임아웃이 설정된 IO 작업 구조
///
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 정의만, 실행 로직 제외
/// - io_id: IO 작업 식별자 (구조 정의만)
/// - timeout_seconds: 타임아웃 시간 (구조 정의)
/// - 실제 타임아웃 체크 및 실행은 executor에서 구현
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutIO {
  /// IO 작업 식별자 (구조 정의만, 실제 IO는 executor에서 관리)
  pub io_id: String,
  /// 타임아웃 시간 (초) (구조 정의만)
  pub timeout_seconds: f64,
}

impl TimeoutIO {
  /// 타임아웃을 설정한 IO 생성
  pub fn new(io_id: String, timeout_seconds: f64) -> Self {
    Self {
      io_id,
      timeout_seconds,
    }
  }

  /// 타임아웃 시간 조회
  pub fn timeout_seconds(&self) -> f64 {
    self.timeout_seconds
  }

  /// IO 식별자 조회
  pub fn io_id(&self) -> &str {
    &self.io_id
  }
}

// 헌법 준수 (P0-1): 실행 로직 제거
// 다음 함수들은 executor/runtime 계층에서 구현하세요:
// - run() -> Result<T, OsError> (실제 실행 및 타임아웃 체크)
// - is_timed_out() -> bool (실제 타임아웃 체크)
//
// 이 함수들은 값 계산 및 실행을 수행하므로 pnix-core에서 제외됩니다.

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_timeout_error_creation() {
    let err = TimeoutError::new("test timeout".to_string(), 5.0);
    assert_eq!(err.message, "test timeout");
    assert_eq!(err.duration_seconds, 5.0);
  }

  #[test]
  fn test_timeout_io_creation() {
    let timeout_io = TimeoutIO::new("io_1".to_string(), 10.0);
    assert_eq!(timeout_io.timeout_seconds(), 10.0);
    assert_eq!(timeout_io.io_id(), "io_1");
  }
}
