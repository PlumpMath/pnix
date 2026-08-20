//! Platform detection helpers for the hardware crate
//!
//! pnix-old의 pnix_hardware/src/platform.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 런타임 감지 로직 제외
//!
//! ## 참고
//!
//! 실제 하드웨어 모드 감지는 executor에서 구현합니다.
//! 이 모듈은 구조 정의만 포함합니다.

/// 하드웨어 모드
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HardwareMode {
  /// 실제 하드웨어 사용
  Real,
  /// 에뮬레이션 모드
  Emulated,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_hardware_mode_variants() {
    assert!(matches!(HardwareMode::Real, HardwareMode::Real));
    assert!(matches!(HardwareMode::Emulated, HardwareMode::Emulated));
    assert_ne!(HardwareMode::Real, HardwareMode::Emulated);
  }
}
