//! Cancellation 구조 정의
//!
//! pnix-old의 pnix_io_runtime/src/cancellation.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 실행 로직 제외
//! - CancellationToken: 취소 토큰 구조 정의
//! - CancellableIO: 취소 가능한 IO 구조 정의
//! - 실제 취소 체크 및 실행 로직은 executor에서 구현

use serde::{Deserialize, Serialize};

/// 취소 토큰 구조
///
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 정의만, 실행 로직 제외
/// - cancelled: 취소 상태 플래그 (구조 정의)
/// - 실제 취소 체크 및 실행은 executor에서 구현
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancellationToken {
  /// 취소 상태 (구조 정의만, 실제 체크는 executor에서)
  pub cancelled: bool,
}

impl CancellationToken {
  /// 새로운 취소 토큰 생성
  pub fn new() -> Self {
    Self { cancelled: false }
  }

  /// 취소 상태 확인 (구조 조회만)
  pub fn is_cancelled(&self) -> bool {
    self.cancelled
  }

  /// 활성 상태 확인 (구조 조회만)
  pub fn is_active(&self) -> bool {
    !self.cancelled
  }
}

impl Default for CancellationToken {
  fn default() -> Self {
    Self::new()
  }
}

/// 취소 가능한 IO 작업 구조
///
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 정의만, 실행 로직 제외
/// - io_id: IO 작업 식별자 (구조 정의만)
/// - token: 취소 토큰 (구조 정의)
/// - 실제 실행 및 취소 체크는 executor에서 구현
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancellableIO {
  /// IO 작업 식별자 (구조 정의만, 실제 IO는 executor에서 관리)
  pub io_id: String,
  /// 취소 토큰 (구조 정의만)
  pub token: CancellationToken,
}

impl CancellableIO {
  /// 취소 토큰과 함께 IO 생성
  pub fn new(io_id: String, token: CancellationToken) -> Self {
    Self { io_id, token }
  }

  /// 취소 토큰 참조 가져오기
  pub fn token(&self) -> &CancellationToken {
    &self.token
  }

  /// IO 식별자 조회
  pub fn io_id(&self) -> &str {
    &self.io_id
  }
}

// 헌법 준수 (P0-1): 실행 로직 제거
// 다음 함수들은 executor/runtime 계층에서 구현하세요:
// - cancel() (실제 취소 실행)
// - reset() (실제 리셋 실행)
// - run() -> Result<T, OsError> (실제 실행 및 취소 체크)
//
// 이 함수들은 값 계산 및 실행을 수행하므로 pnix-core에서 제외됩니다.

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_cancellation_token_creation() {
    let token = CancellationToken::new();
    assert!(!token.is_cancelled());
    assert!(token.is_active());
  }

  #[test]
  fn test_cancellable_io_creation() {
    let token = CancellationToken::new();
    let cancellable = CancellableIO::new("io_1".to_string(), token);
    assert!(cancellable.token().is_active());
    assert_eq!(cancellable.io_id(), "io_1");
  }
}
