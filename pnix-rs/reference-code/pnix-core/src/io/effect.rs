//! IO Effect 구조 정의
//!
//! pnix-old의 pnix_io_runtime/src/io_effect.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 실행 로직 제외
//! - EffectType: Effect 타입 enum
//! - Effect: Effect 구조체
//! - EffectValidator: Effect 검증 구조 (검증 로직은 구조 분석만, 실행 제외)
//! - 실제 실행 로직 (run_io, run_effect 등)은 executor에서 구현

use serde::{Deserialize, Serialize};

/// Effect 타입 태그
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffectType {
  /// 파일 시스템 읽기
  FsRead,
  /// 파일 시스템 쓰기
  FsWrite,
  /// 파일 시스템 삭제
  FsDelete,
  /// 프로세스 실행
  ProcessSpawn,
  /// 네트워크 I/O
  NetworkIO,
  /// 환경 변수 접근
  EnvAccess,
  /// 시스템 시간 접근
  TimeAccess,
}

/// Effect 태그가 붙은 값
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Effect<T> {
  /// Effect 타입
  pub effect_type: EffectType,
  /// Effect 값
  pub value: T,
}

impl<T> Effect<T> {
  /// 새로운 Effect 생성
  pub fn new(effect_type: EffectType, value: T) -> Self {
    Self { effect_type, value }
  }
}

/// Effect 검증기 구조
///
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 분석만, 실행 로직 제외
/// - validate_*: Effect 사용 가능 여부 검증 (구조 분석만)
/// - is_blocking, has_side_effect: Effect 특성 확인 (구조 분석만)
/// - 실제 실행 로직은 executor에서 구현
pub struct EffectValidator;

impl EffectValidator {
  /// STM 트랜잭션 내에서 Effect 사용 가능 여부 확인 (구조 분석만)
  pub fn can_use_in_stm(effect_type: &EffectType) -> bool {
    Self::validate_stm(effect_type).is_ok()
  }

  /// STM 트랜잭션 내에서 Effect 사용 검증 (구조 분석만)
  ///
  /// STM 내에서 허용되는 Effect:
  /// - TimeAccess: 읽기 전용, side-effect 없음
  /// - EnvAccess: 환경 변수 읽기
  /// - FsRead: 읽기 전용
  ///
  /// STM 내에서 금지되는 Effect:
  /// - FsWrite, FsDelete: 롤백 불가능한 부작용
  /// - ProcessSpawn: 롤백 불가능
  /// - NetworkIO: 롤백 불가능, 재시도 시 중복 요청 위험
  pub fn validate_stm(effect_type: &EffectType) -> Result<(), String> {
    match effect_type {
            EffectType::TimeAccess | EffectType::EnvAccess | EffectType::FsRead => Ok(()),
            EffectType::FsWrite => Err(
                "FsWrite cannot be used inside STM: writes cannot be rolled back".to_string(),
            ),
            EffectType::FsDelete => Err(
                "FsDelete cannot be used inside STM: deletes cannot be rolled back".to_string(),
            ),
            EffectType::ProcessSpawn => Err(
                "ProcessSpawn cannot be used inside STM: spawned processes cannot be killed on rollback".to_string(),
            ),
            EffectType::NetworkIO => Err(
                "NetworkIO cannot be used inside STM: network requests cannot be undone and may cause duplicates on retry".to_string(),
            ),
        }
  }

  /// 순수 함수에서 Effect 사용 가능 여부 확인 (구조 분석만)
  pub fn can_use_in_pure(effect_type: &EffectType) -> bool {
    Self::validate_pure(effect_type).is_ok()
  }

  /// 순수 함수에서 Effect 사용 검증 (구조 분석만)
  ///
  /// 순수 함수에서 허용되는 Effect:
  /// - TimeAccess: 결정적이지 않지만 side-effect 없음 (로깅/프로파일링용)
  ///
  /// 순수 함수에서 금지되는 Effect:
  /// - 모든 I/O (파일, 네트워크, 프로세스)
  /// - EnvAccess: 환경에 따라 결과가 달라짐 (비결정적)
  pub fn validate_pure(effect_type: &EffectType) -> Result<(), String> {
    match effect_type {
      EffectType::TimeAccess => Ok(()), // 허용: side-effect 없음 (로깅/프로파일링용)
      _ => Err(format!(
        "Effect {:?} cannot be used in pure function: causes side-effects or non-determinism",
        effect_type
      )),
    }
  }

  /// async context에서 Effect 사용 가능 여부 확인 (구조 분석만)
  pub fn can_use_in_async(effect_type: &EffectType) -> bool {
    Self::validate_async(effect_type).is_ok()
  }

  /// async context에서 Effect 사용 검증 (구조 분석만)
  pub fn validate_async(effect_type: &EffectType) -> Result<(), String> {
    match effect_type {
      EffectType::NetworkIO | EffectType::TimeAccess | EffectType::EnvAccess => Ok(()),
      EffectType::FsRead | EffectType::FsWrite | EffectType::FsDelete => Err(format!(
        "Blocking effect {:?} cannot be used in async context",
        effect_type
      )),
      EffectType::ProcessSpawn => Err(format!(
        "Process spawn {:?} cannot be used in async context",
        effect_type
      )),
    }
  }

  /// Effect 타입이 blocking인지 확인 (구조 분석만)
  pub fn is_blocking(effect_type: &EffectType) -> bool {
    matches!(
      effect_type,
      EffectType::FsRead | EffectType::FsWrite | EffectType::FsDelete | EffectType::ProcessSpawn
    )
  }

  /// Effect 타입이 side-effect를 가지는지 확인 (구조 분석만)
  pub fn has_side_effect(effect_type: &EffectType) -> bool {
    matches!(
      effect_type,
      EffectType::FsWrite | EffectType::FsDelete | EffectType::ProcessSpawn | EffectType::EnvAccess
    )
  }
}

// 헌법 준수 (P0-1): 실행 로직 제거
// 다음 트레잇과 함수들은 executor/runtime 계층에서 구현하세요:
// - IORuntime trait (run_io, run_effect)
//
// 이 함수들은 값 계산 및 실행을 수행하므로 pnix-core에서 제외됩니다.

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_effect_creation() {
    let effect = Effect::new(EffectType::FsRead, "test");
    assert_eq!(effect.effect_type, EffectType::FsRead);
  }

  #[test]
  fn test_effect_validator_stm() {
    assert!(EffectValidator::can_use_in_stm(&EffectType::TimeAccess));
    assert!(!EffectValidator::can_use_in_stm(&EffectType::FsWrite));
  }

  #[test]
  fn test_effect_validator_pure() {
    assert!(EffectValidator::can_use_in_pure(&EffectType::TimeAccess));
    assert!(!EffectValidator::can_use_in_pure(&EffectType::FsRead));
  }

  #[test]
  fn test_is_blocking() {
    assert!(EffectValidator::is_blocking(&EffectType::FsRead));
    assert!(!EffectValidator::is_blocking(&EffectType::TimeAccess));
  }
}
