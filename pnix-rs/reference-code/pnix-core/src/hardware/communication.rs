//! 통신 프로토콜 구조 정의
//!
//! pnix-old의 pnix_hardware/src/communication.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 실행 로직 제외
//! 통신 프로토콜 구현은 executor에서 수행

use serde::{Deserialize, Serialize};

/// 통신 에러 타입: 통신 작업 중 발생하는 에러 타입
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommunicationError {
  /// 연결되지 않음
  NotConnected,
  /// 타임아웃
  Timeout,
  /// 버스 에러
  BusError,
  /// 잘못된 주소
  InvalidAddress,
  /// 잘못된 데이터
  InvalidData,
  /// 프로토콜 에러
  ProtocolError(
    /// 에러 메시지
    String,
  ),
}

impl std::fmt::Display for CommunicationError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      CommunicationError::NotConnected => write!(f, "Device not connected"),
      CommunicationError::Timeout => write!(f, "Communication timeout"),
      CommunicationError::BusError => write!(f, "Bus error"),
      CommunicationError::InvalidAddress => write!(f, "Invalid device address"),
      CommunicationError::InvalidData => write!(f, "Invalid data"),
      CommunicationError::ProtocolError(msg) => write!(f, "Protocol error: {}", msg),
    }
  }
}

impl std::error::Error for CommunicationError {}

/// CAN 메시지 구조: CAN 버스 메시지 구조
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanMessage {
  /// CAN ID
  pub id: u32,
  /// 메시지 데이터
  pub data: Vec<u8>,
  /// 확장 ID 여부
  pub extended: bool,
  /// Remote Transmission Request 여부
  pub rtr: bool,
}

/// I2C 인터페이스 트레이트
///
/// **주의**: 메서드 구현은 executor에서 수행합니다.
pub trait I2cBus {
  /// I2C 버스 초기화 (executor에서 구현)
  fn initialize(&mut self, bus_number: u8) -> Result<(), CommunicationError>;

  /// I2C 쓰기 (executor에서 구현)
  fn write(&mut self, address: u8, data: &[u8]) -> Result<(), CommunicationError>;

  /// I2C 읽기 (executor에서 구현)
  fn read(&mut self, address: u8, buffer: &mut [u8]) -> Result<(), CommunicationError>;

  /// I2C 쓰기-읽기 (레지스터 읽기) (executor에서 구현)
  fn write_read(
    &mut self,
    address: u8,
    write_data: &[u8],
    read_buffer: &mut [u8],
  ) -> Result<(), CommunicationError>;
}

/// SPI 인터페이스 트레이트
///
/// **주의**: 메서드 구현은 executor에서 수행합니다.
pub trait SpiBus {
  /// SPI 버스 초기화 (executor에서 구현)
  fn initialize(&mut self, bus_number: u8, speed_hz: u32) -> Result<(), CommunicationError>;

  /// SPI 전송 (쓰기-읽기 동시) (executor에서 구현)
  fn transfer(&mut self, cs_pin: u8, data: &mut [u8]) -> Result<(), CommunicationError>;

  /// SPI 쓰기 (executor에서 구현)
  fn write(&mut self, cs_pin: u8, data: &[u8]) -> Result<(), CommunicationError>;

  /// SPI 읽기 (executor에서 구현)
  fn read(&mut self, cs_pin: u8, buffer: &mut [u8]) -> Result<(), CommunicationError>;
}

/// UART/Serial 인터페이스 트레이트
///
/// **주의**: 메서드 구현은 executor에서 수행합니다.
/// Duration은 구조 정의용이지만 실제 타임아웃 체크는 executor에서 수행합니다.
pub trait SerialPort {
  /// 시리얼 포트 열기 (executor에서 구현)
  fn open(&mut self, port: &str, baud_rate: u32) -> Result<(), CommunicationError>;

  /// 시리얼 포트 닫기 (executor에서 구현)
  fn close(&mut self) -> Result<(), CommunicationError>;

  /// 데이터 쓰기 (executor에서 구현)
  fn write(&mut self, data: &[u8]) -> Result<usize, CommunicationError>;

  /// 데이터 읽기 (executor에서 구현)
  /// timeout_ms는 밀리초 단위 (실제 타임아웃 체크는 executor에서)
  fn read(&mut self, buffer: &mut [u8], timeout_ms: u64) -> Result<usize, CommunicationError>;

  /// 사용 가능한 바이트 수 확인 (executor에서 구현)
  fn available(&self) -> Result<usize, CommunicationError>;
}

/// CAN Bus 인터페이스 트레이트
///
/// **주의**: 메서드 구현은 executor에서 수행합니다.
pub trait CanBus {
  /// CAN 버스 초기화 (executor에서 구현)
  fn initialize(&mut self, interface: &str, bitrate: u32) -> Result<(), CommunicationError>;

  /// CAN 메시지 전송 (executor에서 구현)
  fn send(&mut self, id: u32, data: &[u8]) -> Result<(), CommunicationError>;

  /// CAN 메시지 수신 (executor에서 구현)
  /// timeout_ms는 밀리초 단위 (실제 타임아웃 체크는 executor에서)
  fn receive(&mut self, timeout_ms: u64) -> Result<CanMessage, CommunicationError>;
}
