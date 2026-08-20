//! 액추에이터 제어 구조 정의
//!
//! pnix-old의 pnix_hardware/src/actuator.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 실행 로직 제외
//! Motor trait의 메서드 구현은 executor에서 수행

use serde::{Deserialize, Serialize};

/// 액추에이터 에러 타입: 액추에이터 작업 중 발생하는 에러 타입
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActuatorError {
  /// 연결되지 않음
  NotConnected,
  /// 잘못된 명령
  InvalidCommand,
  /// 과전류
  OverCurrent,
  /// 과열
  OverTemperature,
  /// 통신 에러
  CommunicationError(
    /// 에러 메시지
    String,
  ),
}

impl std::fmt::Display for ActuatorError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ActuatorError::NotConnected => write!(f, "Actuator not connected"),
      ActuatorError::InvalidCommand => write!(f, "Invalid actuator command"),
      ActuatorError::OverCurrent => write!(f, "Over current protection triggered"),
      ActuatorError::OverTemperature => write!(f, "Over temperature protection triggered"),
      ActuatorError::CommunicationError(msg) => write!(f, "Communication error: {}", msg),
    }
  }
}

impl std::error::Error for ActuatorError {}

/// 모터 제어 모드: 모터 제어 모드 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MotorMode {
  /// 속도 제어
  Velocity,
  /// 위치 제어
  Position,
  /// 토크 제어
  Torque,
}

/// 모터 제어 명령: 모터 제어 명령 구조
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotorCommand {
  /// 제어 모드
  pub mode: MotorMode,
  /// 목표 값 (속도, 위치, 또는 토크)
  pub target: f64,
  /// 최대 가속도 제한 (선택적)
  pub max_acceleration: Option<f64>,
  /// 최대 속도 제한 (선택적)
  pub max_velocity: Option<f64>,
}

/// 모터 상태: 모터 현재 상태 구조
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotorStatus {
  /// 현재 위치
  pub position: f64,
  /// 현재 속도
  pub velocity: f64,
  /// 현재 전류
  pub current: f64,
  /// 현재 온도
  pub temperature: f64,
  /// 활성화 여부
  pub enabled: bool,
}

/// 모터 트레이트
///
/// **주의**: 메서드 구현은 executor에서 수행합니다.
/// pnix-core에는 trait 정의만 포함합니다.
pub trait Motor {
  /// 모터 초기화 (executor에서 구현)
  fn initialize(&mut self) -> Result<(), ActuatorError>;

  /// 모터 활성화 (executor에서 구현)
  fn enable(&mut self) -> Result<(), ActuatorError>;

  /// 모터 비활성화 (executor에서 구현)
  fn disable(&mut self) -> Result<(), ActuatorError>;

  /// 모터 제어 명령 전송 (executor에서 구현)
  fn set_command(&mut self, command: MotorCommand) -> Result<(), ActuatorError>;

  /// 현재 상태 가져오기 (executor에서 구현)
  fn get_status(&self) -> Result<MotorStatus, ActuatorError>;

  /// 비상 정지 (executor에서 구현)
  fn emergency_stop(&mut self) -> Result<(), ActuatorError>;
}
