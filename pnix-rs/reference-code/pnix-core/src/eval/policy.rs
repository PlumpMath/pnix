//! Evaluation Policy 구조 정의
//!
//! pnix-old의 pnix_unified_eval/src/lib.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 실행 로직 제외
//! - EvalPolicy: 평가 정책 구조 정의
//! - AllowedIO: 허용된 IO 작업 구조 정의
//! - 실제 평가 실행, 권한 검사 로직은 executor에서 구현

use serde::{Deserialize, Serialize};

/// 평가 정책 구조 (순수 데이터)
///
/// 평가 중 어떤 effect zone이 허용되는지 제어합니다.
/// Pure zone은 항상 허용됩니다. World zone은 명시적 권한이 필요합니다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalPolicy {
  /// World zone 작업 허용 (IO, 네트워크, 파일 접근)
  pub allow_world: bool,
  /// 최대 실행 시간 (밀리초, 0 = 무제한, 구조 정의만)
  pub timeout_ms: u64,
  /// 최대 메모리 사용량 (바이트, 0 = 무제한, 구조 정의만)
  pub max_memory: usize,
  /// 허용된 IO 작업 (World zone이 활성화된 경우)
  pub allowed_io: AllowedIO,
}

impl EvalPolicy {
  /// Pure zone만 허용하는 정책 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn pure_only() -> Self {
    Self {
      allow_world: false,
      timeout_ms: 0,
      max_memory: 0,
      allowed_io: AllowedIO::default(),
    }
  }

  /// World zone을 허용하는 정책 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn allow_world_zone() -> Self {
    Self {
      allow_world: true,
      timeout_ms: 0,
      max_memory: 0,
      allowed_io: AllowedIO {
        print: true,
        file_read: true,
        file_write: false,
        network: false,
      },
    }
  }

  /// 전체 권한 정책 생성 (주의해서 사용)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn full_access() -> Self {
    Self {
      allow_world: true,
      timeout_ms: 0,
      max_memory: 0,
      allowed_io: AllowedIO {
        print: true,
        file_read: true,
        file_write: true,
        network: true,
      },
    }
  }

  /// 타임아웃 설정 (구조 변경만)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변경만, 값 계산 없음
  pub fn with_timeout(mut self, ms: u64) -> Self {
    self.timeout_ms = ms;
    self
  }

  /// 최대 메모리 설정 (구조 변경만)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변경만, 값 계산 없음
  pub fn with_max_memory(mut self, bytes: usize) -> Self {
    self.max_memory = bytes;
    self
  }

  /// Print 작업 허용 설정 (구조 변경만)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변경만, 값 계산 없음
  pub fn with_print(mut self) -> Self {
    self.allowed_io.print = true;
    self
  }
}

impl Default for EvalPolicy {
  fn default() -> Self {
    Self::pure_only()
  }
}

/// 허용된 IO 작업 구조 (순수 데이터)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AllowedIO {
  /// Print/println 작업 허용
  pub print: bool,
  /// 파일 읽기 작업 허용
  pub file_read: bool,
  /// 파일 쓰기 작업 허용
  pub file_write: bool,
  /// 네트워크 작업 허용
  pub network: bool,
}

// 헌법 준수 (P0-1): 실행 로직 제거
// 다음 함수들은 executor/runtime 계층에서 구현하세요:
// - EvalPolicy::can_print() -> bool (권한 검사, 값 계산)
// - EvalPolicy::can_read_file() -> bool (권한 검사, 값 계산)
// - EvalPolicy::can_write_file() -> bool (권한 검사, 값 계산)
// - EvalPolicy::can_network() -> bool (권한 검사, 값 계산)
// - 실제 평가 실행 로직 (eval_once 등)
//
// 이 함수들은 값 계산 또는 실행 로직을 수행하므로 pnix-core에서 제외됩니다.

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_eval_policy_pure_only() {
    let policy = EvalPolicy::pure_only();
    assert!(!policy.allow_world);
  }

  #[test]
  fn test_eval_policy_allow_world() {
    let policy = EvalPolicy::allow_world_zone();
    assert!(policy.allow_world);
    assert!(policy.allowed_io.print);
    assert!(policy.allowed_io.file_read);
  }

  #[test]
  fn test_eval_policy_with_timeout() {
    let policy = EvalPolicy::pure_only().with_timeout(1000);
    assert_eq!(policy.timeout_ms, 1000);
  }

  #[test]
  fn test_allowed_io_default() {
    let io = AllowedIO::default();
    assert!(!io.print);
    assert!(!io.file_read);
  }
}
