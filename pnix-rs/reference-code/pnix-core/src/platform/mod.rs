//! Platform 구조 정의
//!
//! pnix-old의 pnix_platform, pnix_hardware/src/platform.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 플랫폼 감지 실행 로직 제외
//! 환경 변수 읽기 등 런타임 로직은 executor로 이관

pub mod android;
pub mod desktop;
pub mod ios;

pub use android::{AndroidFileSystem, AndroidLog};
pub use desktop::{DesktopFileSystem, DesktopLog};
pub use ios::{IosFileSystem, IosLog};

/// 플랫폼 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Platform {
  /// iOS 플랫폼
  IOS,
  /// Android 플랫폼
  Android,
  /// Desktop 플랫폼 (Linux, macOS, Windows)
  Desktop,
}

impl Platform {
  /// 플랫폼 이름 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn name(&self) -> &'static str {
    match self {
      Platform::IOS => "ios",
      Platform::Android => "android",
      Platform::Desktop => "desktop",
    }
  }
}

// ============================================================
// Hardware Mode (pnix_hardware/src/platform.rs에서 마이그레이션)
// ============================================================

/// 환경 변수 이름: PNIX_HARDWARE_EMULATION
///
/// 이 환경 변수가 설정되면 에뮬레이션 모드를 강제합니다.
/// (실제 환경 변수 읽기는 executor에서 수행)
pub const EMULATION_ENV_VAR: &str = "PNIX_HARDWARE_EMULATION";

/// 하드웨어 모드
///
/// 실제 하드웨어 또는 에뮬레이션 모드를 나타냅니다.
#[derive(
  Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
pub enum HardwareMode {
  /// 실제 하드웨어 드라이버 사용
  Real,
  /// 에뮬레이션 모드 (기본값)
  #[default]
  Emulated,
}

impl HardwareMode {
  /// 문자열로 변환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn as_str(&self) -> &'static str {
    match self {
      HardwareMode::Real => "real",
      HardwareMode::Emulated => "emulated",
    }
  }

  /// 에뮬레이션 모드인지 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 검증만, 값 계산 없음
  pub fn is_emulated(&self) -> bool {
    matches!(self, HardwareMode::Emulated)
  }

  /// 실제 하드웨어 모드인지 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 검증만, 값 계산 없음
  pub fn is_real(&self) -> bool {
    matches!(self, HardwareMode::Real)
  }
}

/// 환경 변수 값 파싱 (순수 함수)
///
/// 빈 문자열, "0", "false", "no", "off"는 false, 나머지는 true
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
///
/// # Examples
///
/// ```rust
/// use pnix_core::platform::parse_env_bool;
///
/// assert!(!parse_env_bool(""));
/// assert!(!parse_env_bool("0"));
/// assert!(!parse_env_bool("false"));
/// assert!(parse_env_bool("1"));
/// assert!(parse_env_bool("true"));
/// ```
pub fn parse_env_bool(value: &str) -> bool {
  !matches!(
    value.trim().to_ascii_lowercase().as_str(),
    "" | "0" | "false" | "no" | "off"
  )
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_platform_name() {
    assert_eq!(Platform::IOS.name(), "ios");
    assert_eq!(Platform::Android.name(), "android");
    assert_eq!(Platform::Desktop.name(), "desktop");
  }

  #[test]
  fn test_hardware_mode() {
    assert!(HardwareMode::Emulated.is_emulated());
    assert!(!HardwareMode::Emulated.is_real());
    assert!(HardwareMode::Real.is_real());
    assert!(!HardwareMode::Real.is_emulated());
  }

  #[test]
  fn test_parse_env_bool_falsey() {
    for v in ["", "0", "false", "FALSE", "no", "off", "  0  "] {
      assert!(!parse_env_bool(v), "expected false for {v:?}");
    }
  }

  #[test]
  fn test_parse_env_bool_truthy() {
    for v in ["1", "true", "TRUE", "yes", "on", "anything"] {
      assert!(parse_env_bool(v), "expected true for {v:?}");
    }
  }
}
