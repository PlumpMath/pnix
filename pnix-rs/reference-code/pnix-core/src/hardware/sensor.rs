//! 센서 인터페이스 구조 정의
//!
//! pnix-old의 pnix_hardware/src/sensor.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 실행 로직 제외
//! Sensor trait의 메서드 구현은 executor에서 수행

use serde::{Deserialize, Serialize};

/// 센서 에러 타입: 센서 작업 중 발생하는 에러 타입
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SensorError {
  /// 연결되지 않음
  NotConnected,
  /// 읽기 타임아웃
  ReadTimeout,
  /// 잘못된 데이터
  InvalidData,
  /// 통신 에러
  CommunicationError(
    /// 에러 메시지
    String,
  ),
  /// 캘리브레이션 에러
  CalibrationError(
    /// 에러 메시지
    String,
  ),
}

impl std::fmt::Display for SensorError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      SensorError::NotConnected => write!(f, "Sensor not connected"),
      SensorError::ReadTimeout => write!(f, "Sensor read timeout"),
      SensorError::InvalidData => write!(f, "Invalid sensor data"),
      SensorError::CommunicationError(msg) => write!(f, "Communication error: {}", msg),
      SensorError::CalibrationError(msg) => write!(f, "Calibration error: {}", msg),
    }
  }
}

impl std::error::Error for SensorError {}

/// IMU 데이터 구조: 관성 측정 장치 데이터
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImuData {
  /// 가속도 (m/s², [x, y, z])
  pub acceleration: [f64; 3],
  /// 각속도 (rad/s, [x, y, z])
  pub angular_velocity: [f64; 3],
  /// 자력계 (Gauss, [x, y, z])
  pub magnetometer: [f64; 3],
  /// 온도 (Celsius)
  pub temperature: f64,
  // timestamp는 런타임 상태이므로 executor에서 관리
}

/// LiDAR 스캔 데이터: LiDAR 스캔 결과 데이터
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LidarScan {
  /// 거리 목록 (m)
  pub ranges: Vec<f64>,
  /// 각도 목록 (rad)
  pub angles: Vec<f64>,
  /// 강도 목록 (선택적)
  pub intensities: Option<Vec<f64>>,
  // timestamp는 런타임 상태이므로 executor에서 관리
}

/// 카메라 이미지 데이터: 카메라 이미지 데이터 구조
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraImage {
  /// 이미지 너비 (픽셀)
  pub width: u32,
  /// 이미지 높이 (픽셀)
  pub height: u32,
  /// 이미지 데이터 (RGB 또는 Depth 데이터)
  pub data: Vec<u8>,
  /// 이미지 포맷
  pub format: ImageFormat,
  // timestamp는 런타임 상태이므로 executor에서 관리
}

/// 이미지 포맷: 이미지 데이터 포맷 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageFormat {
  /// RGB 포맷
  RGB,
  /// RGBA 포맷
  RGBA,
  /// Depth 포맷
  Depth,
  /// Grayscale 포맷
  Grayscale,
}

/// 엔코더 데이터: 엔코더 데이터 구조
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncoderData {
  /// 위치 (펄스 수 또는 각도)
  pub position: i64,
  /// 속도 (rad/s 또는 m/s)
  pub velocity: f64,
  // timestamp는 런타임 상태이므로 executor에서 관리
}

/// 통합 센서 데이터 타입: 모든 센서 데이터를 통합하는 타입
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SensorData {
  /// IMU 데이터
  Imu(ImuData),
  /// LiDAR 스캔 데이터
  Lidar(LidarScan),
  /// 카메라 이미지 데이터
  Camera(CameraImage),
  /// 엔코더 데이터
  Encoder(EncoderData),
}

/// 센서 트레이트
///
/// **주의**: 메서드 구현은 executor에서 수행합니다.
/// pnix-core에는 trait 정의만 포함합니다.
pub trait Sensor {
  /// 센서 초기화 (executor에서 구현)
  fn initialize(&mut self) -> Result<(), SensorError>;

  /// 센서 연결 확인 (executor에서 구현)
  fn is_connected(&self) -> bool;

  /// 센서 읽기 (executor에서 구현)
  fn read(&mut self) -> Result<(), SensorError>;

  /// 최신 데이터 가져오기 (executor에서 구현)
  fn get_latest_data(&self) -> Option<SensorData>;

  /// 센서 캘리브레이션 (executor에서 구현)
  fn calibrate(&mut self) -> Result<(), SensorError>;

  /// 센서 종료 (executor에서 구현)
  fn shutdown(&mut self) -> Result<(), SensorError>;
}
