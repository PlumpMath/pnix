//! 압축 관련 타입 정의 (순수 구조)
//!
//! pnix-old의 pnix_utils/src/compression.rs에서 마이그레이션.
//! 압축/해제 함수는 executor에서 구현 (P0-1 준수).

use core::fmt;
use serde::{Deserialize, Serialize};

/// 압축 에러: 압축/해제 관련 에러 타입
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionError {
  /// 잘못된 데이터 형식
  InvalidFormat(
    /// 에러 메시지
    String,
  ),
  /// 압축 해제 실패
  DecompressionFailed(
    /// 실패 이유
    String,
  ),
  /// 지원하지 않는 형식
  UnsupportedFormat,
  /// IO 에러
  IoError(
    /// 에러 메시지
    String,
  ),
  /// 데이터 손상
  DataCorruption,
}

impl fmt::Display for CompressionError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      CompressionError::InvalidFormat(msg) => write!(f, "Invalid format: {}", msg),
      CompressionError::DecompressionFailed(msg) => {
        write!(f, "Decompression failed: {}", msg)
      }
      CompressionError::UnsupportedFormat => write!(f, "Unsupported compression format"),
      CompressionError::IoError(msg) => write!(f, "IO error: {}", msg),
      CompressionError::DataCorruption => write!(f, "Data corruption detected"),
    }
  }
}

impl std::error::Error for CompressionError {}

/// 압축 형식: 지원하는 압축 알고리즘 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionFormat {
  /// RLE (Run-Length Encoding)
  Rle,
  /// LZ77 기반 압축
  Lz77,
  /// 간단한 DEFLATE 스타일 압축
  Simple,
}

/// 압축 레벨: 압축 속도와 압축률의 균형 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionLevel {
  /// 빠른 압축 (낮은 압축률)
  Fast,
  /// 균형 잡힌 압축
  Default,
  /// 최대 압축 (느림)
  Best,
}

impl Default for CompressionLevel {
  fn default() -> Self {
    Self::Default
  }
}

/// 압축 통계: 압축 작업의 통계 정보
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompressionStats {
  /// 원본 크기 (바이트)
  pub original_size: usize,
  /// 압축된 크기 (바이트)
  pub compressed_size: usize,
  /// 사용된 압축 형식
  pub format: CompressionFormat,
  /// 사용된 압축 레벨
  pub level: CompressionLevel,
}

impl CompressionStats {
  /// 새 통계 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(
    original_size: usize,
    compressed_size: usize,
    format: CompressionFormat,
    level: CompressionLevel,
  ) -> Self {
    Self {
      original_size,
      compressed_size,
      format,
      level,
    }
  }

  /// 압축률 계산 (0.0 ~ 1.0, 낮을수록 좋음)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn ratio(&self) -> f64 {
    if self.original_size == 0 {
      1.0
    } else {
      self.compressed_size as f64 / self.original_size as f64
    }
  }

  /// 절약된 바이트 수
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn saved_bytes(&self) -> isize {
    self.original_size as isize - self.compressed_size as isize
  }

  /// 절약된 비율 (백분율)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn saved_percent(&self) -> f64 {
    if self.original_size == 0 {
      0.0
    } else {
      (1.0 - self.ratio()) * 100.0
    }
  }
}

// NOTE: 압축/해제 함수 (rle_compress, lz77_compress, compress, decompress 등)는
// P0-1 위반으로 executor/runtime 계층에서 구현합니다.

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_compression_error_display() {
    assert_eq!(
      format!("{}", CompressionError::UnsupportedFormat),
      "Unsupported compression format"
    );
    assert_eq!(
      format!("{}", CompressionError::DataCorruption),
      "Data corruption detected"
    );
  }

  #[test]
  fn test_compression_level_default() {
    assert_eq!(CompressionLevel::default(), CompressionLevel::Default);
  }

  #[test]
  fn test_compression_stats() {
    let stats = CompressionStats::new(1000, 400, CompressionFormat::Lz77, CompressionLevel::Best);

    assert_eq!(stats.ratio(), 0.4);
    assert_eq!(stats.saved_bytes(), 600);
    assert!((stats.saved_percent() - 60.0).abs() < 0.01);
  }

  #[test]
  fn test_compression_stats_empty() {
    let stats = CompressionStats::new(0, 0, CompressionFormat::Rle, CompressionLevel::Fast);
    assert_eq!(stats.ratio(), 1.0);
    assert_eq!(stats.saved_percent(), 0.0);
  }
}
