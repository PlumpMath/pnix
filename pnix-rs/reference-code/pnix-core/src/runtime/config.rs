//! Runtime 설정 타입
//!
//! pnix-old의 pnix_runtime/src/runtime.rs에서 마이그레이션.

use serde::{Deserialize, Serialize};

/// 런타임 설정
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
  /// 초기화 모드
  pub init_mode: InitMode,
  /// 심볼릭 엔진 설정
  pub symbolic_config: SymbolicConfig,
  /// 검증 레벨
  pub validation_level: ValidationLevel,
}

impl Default for RuntimeConfig {
  fn default() -> Self {
    Self {
      init_mode: InitMode::Empty,
      symbolic_config: SymbolicConfig::default(),
      validation_level: ValidationLevel::Full,
    }
  }
}

/// 초기화 모드
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InitMode {
  /// 빈 런타임
  Empty,
  /// 물리 기본 설정
  Physics,
  /// GR/텐서 기본 설정
  GeneralRelativity,
}

/// 심볼릭 엔진 설정
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolicConfig {
  /// 최대 단순화 반복 횟수
  pub max_simplify_iterations: usize,
  /// CT 검증 활성화
  pub ct_validation_enabled: bool,
  /// 텐서 검증 활성화
  pub tensor_validation_enabled: bool,
}

impl Default for SymbolicConfig {
  fn default() -> Self {
    Self {
      max_simplify_iterations: 100,
      ct_validation_enabled: true,
      tensor_validation_enabled: true,
    }
  }
}

/// 검증 레벨
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationLevel {
  /// 검증 없음
  None,
  /// 기본 검증 (단위만)
  Basic,
  /// 전체 검증 (단위 + 텐서)
  Full,
}
