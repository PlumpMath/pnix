//! UUID 타입 정의 (순수 구조)
//!
//! pnix-old의 pnix_utils/src/uuid.rs에서 마이그레이션.
//! UUID 생성 함수(uuid_v4, uuid_v1)는 런타임 의존으로 executor에서 구현.

use core::fmt;
use serde::{Deserialize, Serialize};

/// UUID 구조체: 128비트 UUID를 표현하는 구조
///
/// UUID 생성은 executor에서 수행합니다.
/// 헌법 P0-1 준수: 구조 정의만, 값 계산 없음
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Uuid {
  /// 상위 64비트
  pub high: u64,
  /// 하위 64비트
  pub low: u64,
}

impl Uuid {
  /// 새 UUID 생성 (직접 값 지정)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub const fn new(high: u64, low: u64) -> Self {
    Self { high, low }
  }

  /// nil UUID (모든 비트가 0)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub const fn nil() -> Self {
    Self { high: 0, low: 0 }
  }

  /// UUID가 nil인지 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub const fn is_nil(&self) -> bool {
    self.high == 0 && self.low == 0
  }

  /// UUID 버전 반환 (4비트)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn version(&self) -> u8 {
    ((self.high >> 12) & 0xF) as u8
  }

  /// UUID variant 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn variant(&self) -> UuidVariant {
    let variant_bits = (self.low >> 62) as u8;
    match variant_bits {
      0b00 | 0b01 => UuidVariant::Ncs,
      0b10 => UuidVariant::Rfc4122,
      0b11 => UuidVariant::Microsoft,
      _ => UuidVariant::Ncs,
    }
  }

  /// 문자열에서 UUID 파싱
  ///
  /// 지원 형식:
  /// - `550e8400-e29b-41d4-a716-446655440000` (표준)
  /// - `550e8400e29b41d4a716446655440000` (하이픈 없음)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 파싱만, 값 계산 없음
  pub fn parse(s: &str) -> Result<Self, UuidError> {
    let s = s.trim();
    let clean: String = s.chars().filter(|c| *c != '-').collect();

    if clean.len() != 32 {
      return Err(UuidError::InvalidLength {
        expected: 32,
        actual: clean.len(),
      });
    }

    let high = u64::from_str_radix(&clean[0..16], 16)
      .map_err(|_| UuidError::InvalidHex(clean[0..16].to_string()))?;
    let low = u64::from_str_radix(&clean[16..32], 16)
      .map_err(|_| UuidError::InvalidHex(clean[16..32].to_string()))?;

    Ok(Self { high, low })
  }

  /// UUID 문자열 검증
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 검증만, 값 계산 없음
  pub fn is_valid(s: &str) -> bool {
    Self::parse(s).is_ok()
  }

  /// 바이트 배열로 변환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변환만, 값 계산 없음
  pub fn to_bytes(&self) -> [u8; 16] {
    let mut bytes = [0u8; 16];
    bytes[0..8].copy_from_slice(&self.high.to_be_bytes());
    bytes[8..16].copy_from_slice(&self.low.to_be_bytes());
    bytes
  }

  /// 바이트 배열에서 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn from_bytes(bytes: &[u8; 16]) -> Self {
    let high = u64::from_be_bytes([
      bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ]);
    let low = u64::from_be_bytes([
      bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
    ]);
    Self { high, low }
  }
}

impl fmt::Display for Uuid {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let bytes = self.to_bytes();
    write!(
            f,
            "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            bytes[0], bytes[1], bytes[2], bytes[3],
            bytes[4], bytes[5],
            bytes[6], bytes[7],
            bytes[8], bytes[9],
            bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
        )
  }
}

/// UUID Variant: UUID의 variant 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UuidVariant {
  /// NCS 호환
  Ncs,
  /// RFC 4122 표준
  Rfc4122,
  /// Microsoft 호환
  Microsoft,
}

/// UUID 에러: UUID 파싱/검증 관련 에러 타입
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UuidError {
  /// 잘못된 길이
  InvalidLength {
    /// 예상 길이
    expected: usize,
    /// 실제 길이
    actual: usize,
  },
  /// 잘못된 16진수
  InvalidHex(
    /// 잘못된 문자열
    String,
  ),
  /// 잘못된 형식
  InvalidFormat(
    /// 에러 메시지
    String,
  ),
}

impl fmt::Display for UuidError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      UuidError::InvalidLength { expected, actual } => {
        write!(
          f,
          "Invalid UUID length: expected {}, got {}",
          expected, actual
        )
      }
      UuidError::InvalidHex(s) => write!(f, "Invalid hex string: {}", s),
      UuidError::InvalidFormat(s) => write!(f, "Invalid UUID format: {}", s),
    }
  }
}

impl std::error::Error for UuidError {}

// NOTE: UUID 생성 함수 (uuid_v4, uuid_v1)는 런타임 의존 (랜덤/시간)으로
// executor/runtime 계층에서 구현합니다.

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_uuid_nil() {
    let uuid = Uuid::nil();
    assert!(uuid.is_nil());
    assert_eq!(uuid.high, 0);
    assert_eq!(uuid.low, 0);
  }

  #[test]
  fn test_uuid_parse() {
    let uuid = Uuid::parse("550e8400-e29b-41d4-a716-446655440000").unwrap();
    assert!(!uuid.is_nil());
    assert_eq!(uuid.version(), 4);
  }

  #[test]
  fn test_uuid_parse_no_hyphens() {
    let uuid = Uuid::parse("550e8400e29b41d4a716446655440000").unwrap();
    assert!(!uuid.is_nil());
  }

  #[test]
  fn test_uuid_display() {
    let uuid = Uuid::parse("550e8400-e29b-41d4-a716-446655440000").unwrap();
    assert_eq!(format!("{}", uuid), "550e8400-e29b-41d4-a716-446655440000");
  }

  #[test]
  fn test_uuid_bytes_roundtrip() {
    let uuid = Uuid::new(0x123456789ABCDEF0, 0xFEDCBA9876543210);
    let bytes = uuid.to_bytes();
    let restored = Uuid::from_bytes(&bytes);
    assert_eq!(uuid, restored);
  }

  #[test]
  fn test_uuid_is_valid() {
    assert!(Uuid::is_valid("550e8400-e29b-41d4-a716-446655440000"));
    assert!(Uuid::is_valid("550e8400e29b41d4a716446655440000"));
    assert!(!Uuid::is_valid("invalid"));
    assert!(!Uuid::is_valid("550e8400-e29b-41d4-a716-44665544000")); // 31자
  }

  #[test]
  fn test_uuid_error_display() {
    assert_eq!(
      format!(
        "{}",
        UuidError::InvalidLength {
          expected: 32,
          actual: 31
        }
      ),
      "Invalid UUID length: expected 32, got 31"
    );
  }
}
