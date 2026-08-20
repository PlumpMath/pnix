//! Hardware 구조 정의
//!
//! pnix-old의 pnix_hardware에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 실행 로직 제외
//! 센서 읽기, 모터 제어, 통신 프로토콜 구현 등은 executor에서 수행

pub mod actuator;
pub mod communication;
pub mod platform;
pub mod safety;
pub mod sensor;

pub use actuator::{ActuatorError, Motor, MotorCommand, MotorMode, MotorStatus};
pub use communication::{CanBus, CanMessage, CommunicationError, I2cBus, SerialPort, SpiBus};
pub use platform::HardwareMode;
pub use safety::{
  EmergencyStopConfig, SafetyError, SafetySystemConfig, SpeedLimiterConfig, WatchdogTimerConfig,
};
pub use sensor::{
  CameraImage, EncoderData, ImageFormat, ImuData, LidarScan, Sensor, SensorData, SensorError,
};
