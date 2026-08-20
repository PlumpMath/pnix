//! 안전 기능 구조 정의
//!
//! pnix-old의 pnix_hardware/src/safety.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 실행 로직 제외
//! EmergencyStop, WatchdogTimer, SpeedLimiter의 실행 로직은 executor에서 수행

use serde::{Deserialize, Serialize};

/// 안전 시스템 에러: 안전 시스템 작업 중 발생하는 에러 타입
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SafetyError {
  /// 비상 정지 활성화됨
  EmergencyStopActive,
  /// 워치독 타임아웃
  WatchdogTimeout,
  /// 안전 제한 초과
  SafetyLimitExceeded(
    /// 제한 초과 메시지
    String,
  ),
}

impl std::fmt::Display for SafetyError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      SafetyError::EmergencyStopActive => write!(f, "Emergency stop is active"),
      SafetyError::WatchdogTimeout => write!(f, "Watchdog timeout"),
      SafetyError::SafetyLimitExceeded(msg) => write!(f, "Safety limit exceeded: {}", msg),
    }
  }
}

impl std::error::Error for SafetyError {}

/// Emergency Stop 설정: 비상 정지 시스템의 설정 구조
///
/// **주의**: 실제 실행 로직(`Arc<AtomicBool>`, monotonic clock 등)은 executor에서 구현합니다.
/// 헌법 P0-1 준수: 구조 정의만, 실행 로직 제외
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergencyStopConfig {
  /// 활성화 여부 (실제 상태는 executor에서 관리)
  pub enabled: bool,
}

/// Watchdog Timer 설정: 워치독 타이머의 설정 구조
///
/// **주의**: 실제 실행 로직(`Arc<Mutex<...>>`, 타임아웃 체크 등)은 executor에서 구현합니다.
/// 헌법 P0-1 준수: 구조 정의만, 실행 로직 제외
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchdogTimerConfig {
  /// 타임아웃 (밀리초)
  pub timeout_ms: u64,
  /// 활성화 여부 (실제 상태는 executor에서 관리)
  pub enabled: bool,
}

/// 안전 속도 제한기 설정: 속도 제한기의 설정 구조
///
/// **주의**: 실제 값 계산 로직은 executor에서 구현합니다.
/// 헌법 P0-1 준수: 구조 정의만, 실행 로직 제외
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeedLimiterConfig {
  /// 최대 선형 속도 (m/s)
  pub max_linear_velocity: f64,
  /// 최대 각속도 (rad/s)
  pub max_angular_velocity: f64,
  /// 최대 가속도 (m/s²)
  pub max_acceleration: f64,
}

// **주의**: 값 계산 함수는 P0-1 위반이므로 제거되었습니다.
// 실제 속도/가속도 제한 계산은 executor에서 수행합니다.
// 실행/상태 업데이트는 executor/runtime 계층에서 담당

/// 통합 안전 시스템 설정: 모든 안전 기능을 통합하는 설정 구조
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetySystemConfig {
  /// 비상 정지 설정
  pub emergency_stop: EmergencyStopConfig,
  /// 워치독 타이머 설정
  pub watchdog: WatchdogTimerConfig,
  /// 속도 제한기 설정
  pub speed_limiter: SpeedLimiterConfig,
}
