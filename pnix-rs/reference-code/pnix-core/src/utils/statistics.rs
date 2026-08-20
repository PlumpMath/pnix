//! 통계 타입 정의 (순수 구조)
//!
//! pnix-old의 pnix_utils/src/statistics.rs에서 마이그레이션.
//! 계산 함수는 executor에서 구현 (P0-1 준수).

use core::fmt;
use serde::{Deserialize, Serialize};

/// 통계 에러: 통계 계산 관련 에러 타입
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatisticsError {
  /// 빈 데이터셋
  EmptyData,
  /// 데이터 부족
  InsufficientData {
    /// 필요한 데이터 개수
    required: usize,
    /// 제공된 데이터 개수
    provided: usize,
  },
  /// 계산 불가능
  ComputationError(
    /// 에러 메시지
    String,
  ),
}

impl fmt::Display for StatisticsError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      StatisticsError::EmptyData => write!(f, "Empty dataset"),
      StatisticsError::InsufficientData { required, provided } => {
        write!(
          f,
          "Insufficient data: need {} but got {}",
          required, provided
        )
      }
      StatisticsError::ComputationError(msg) => write!(f, "Computation error: {}", msg),
    }
  }
}

impl std::error::Error for StatisticsError {}

/// 기술 통계 결과: 데이터셋의 기술 통계량을 저장하는 구조체
///
/// 계산은 executor에서 수행하고 결과만 이 구조체에 저장합니다.
/// 헌법 P0-1 준수: 구조 정의만, 값 계산 없음
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DescriptiveStats {
  /// 데이터 개수
  pub count: usize,
  /// 평균
  pub mean: f64,
  /// 중앙값
  pub median: f64,
  /// 최빈값
  pub mode: f64,
  /// 표준편차
  pub std_dev: f64,
  /// 분산
  pub variance: f64,
  /// 최솟값
  pub min: f64,
  /// 최댓값
  pub max: f64,
  /// 범위 (max - min)
  pub range: f64,
  /// 1사분위수
  pub q1: f64,
  /// 2사분위수 (중앙값)
  pub q2: f64,
  /// 3사분위수
  pub q3: f64,
  /// 사분위 범위 (q3 - q1)
  pub iqr: f64,
  /// 왜도
  pub skewness: f64,
  /// 첨도
  pub kurtosis: f64,
}

impl Default for DescriptiveStats {
  fn default() -> Self {
    Self {
      count: 0,
      mean: 0.0,
      median: 0.0,
      mode: 0.0,
      std_dev: 0.0,
      variance: 0.0,
      min: 0.0,
      max: 0.0,
      range: 0.0,
      q1: 0.0,
      q2: 0.0,
      q3: 0.0,
      iqr: 0.0,
      skewness: 0.0,
      kurtosis: 0.0,
    }
  }
}

// NOTE: 계산 함수 (mean, median, std_dev, variance, histogram, linear_regression 등)는
// P0-1 위반으로 executor에서 구현합니다.
// (pnix-core는 실행/값 계산 금지)

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_statistics_error_display() {
    assert_eq!(format!("{}", StatisticsError::EmptyData), "Empty dataset");
    assert_eq!(
      format!(
        "{}",
        StatisticsError::InsufficientData {
          required: 10,
          provided: 5
        }
      ),
      "Insufficient data: need 10 but got 5"
    );
  }

  #[test]
  fn test_descriptive_stats_default() {
    let stats = DescriptiveStats::default();
    assert_eq!(stats.count, 0);
    assert_eq!(stats.mean, 0.0);
  }
}
