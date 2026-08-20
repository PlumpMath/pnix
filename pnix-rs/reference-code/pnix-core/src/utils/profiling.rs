//! Profiling 구조 정의
//!
//! pnix-old의 pnix_profiling에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 실행 로직 제외
//! - Metric: 프로파일링 메트릭 구조 정의
//! - Bottleneck: 병목 정보 구조 정의
//! - BottleneckSeverity: 병목 심각도 enum 정의
//! - 실제 시간 측정, 수집, 탐지 로직은 executor에서 구현

use serde::{Deserialize, Serialize};

/// 프로파일링 메트릭 구조: 프로파일링 메트릭을 저장하는 구조
///
/// 헌법 P0-1 준수: 구조 정의만, 값 계산 없음
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
  /// 메트릭 이름
  pub name: String,
  /// 측정된 시간 (밀리초, 구조 정의만)
  pub duration_ms: u64,
  /// 호출 횟수
  pub count: usize,
  /// 평균 시간 (밀리초, 구조 정의만)
  pub avg_duration_ms: u64,
  /// 최소 시간 (밀리초, 구조 정의만)
  pub min_duration_ms: u64,
  /// 최대 시간 (밀리초, 구조 정의만)
  pub max_duration_ms: u64,
  /// 총 시간 (밀리초, 구조 정의만)
  pub total_duration_ms: u64,
}

impl Metric {
  /// 새로운 메트릭 생성 (구조만)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(name: impl Into<String>) -> Self {
    Self {
      name: name.into(),
      duration_ms: 0,
      count: 0,
      avg_duration_ms: 0,
      min_duration_ms: 0,
      max_duration_ms: 0,
      total_duration_ms: 0,
    }
  }
}

/// 병목 심각도: 성능 병목의 심각도 레벨 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum BottleneckSeverity {
  /// 낮음
  Low,
  /// 중간
  Medium,
  /// 높음
  High,
  /// 매우 높음
  Critical,
}

impl BottleneckSeverity {
  /// 심각도 설명 (구조적 정보만)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn description(&self) -> &'static str {
    match self {
      BottleneckSeverity::Low => "낮음",
      BottleneckSeverity::Medium => "중간",
      BottleneckSeverity::High => "높음",
      BottleneckSeverity::Critical => "매우 높음",
    }
  }
}

/// 병목 정보 구조: 성능 병목 정보를 저장하는 구조
///
/// 헌법 P0-1 준수: 구조 정의만, 값 계산 없음
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bottleneck {
  /// 메트릭 이름
  pub metric_name: String,
  /// 심각도
  pub severity: BottleneckSeverity,
  /// 총 시간 (밀리초, 구조 정의만)
  pub total_duration_ms: u64,
  /// 평균 시간 (밀리초, 구조 정의만)
  pub avg_duration_ms: u64,
  /// 호출 횟수
  pub count: usize,
  /// 총 실행 시간 대비 비율 (백분율)
  pub percentage: f64,
  /// 이유
  pub reason: String,
}

impl Bottleneck {
  /// 새로운 병목 생성 (구조만)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(
    metric_name: impl Into<String>,
    severity: BottleneckSeverity,
    total_duration_ms: u64,
    avg_duration_ms: u64,
    count: usize,
    percentage: f64,
    reason: impl Into<String>,
  ) -> Self {
    Self {
      metric_name: metric_name.into(),
      severity,
      total_duration_ms,
      avg_duration_ms,
      count,
      percentage,
      reason: reason.into(),
    }
  }
}

// 헌법 준수 (P0-1): 실행 로직 제거
// 다음 함수들은 executor/runtime 계층에서 구현하세요:
// - Metric::add_measurement() (측정값 추가, 시간 계산)
// - Metric::reset() (메트릭 리셋)
// - ProfilingCollector 구조체 및 메서드들 (시간 측정, 수집)
// - BottleneckDetector 구조체 및 detect() 메서드 (병목 탐지 로직)
// - Timer 구조체 (실행 시간 측정)
// - Counter 구조체 (작업 횟수 추적)
// - MemorySnapshot, MemoryTracker 등 메모리 프로파일링 구조 (런타임 상태)
//
// 이 함수들은 값 계산, 상태 변경, 또는 시간 측정을 수행하므로 pnix-core에서 제외됩니다.

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_metric_creation() {
    let metric = Metric::new("test_operation");
    assert_eq!(metric.name, "test_operation");
    assert_eq!(metric.count, 0);
  }

  #[test]
  fn test_bottleneck_creation() {
    let bottleneck = Bottleneck::new(
      "slow_op",
      BottleneckSeverity::High,
      1000,
      100,
      10,
      50.0,
      "Too slow",
    );
    assert_eq!(bottleneck.metric_name, "slow_op");
    assert_eq!(bottleneck.severity, BottleneckSeverity::High);
    assert_eq!(bottleneck.percentage, 50.0);
  }

  #[test]
  fn test_bottleneck_severity_description() {
    assert_eq!(BottleneckSeverity::Low.description(), "낮음");
    assert_eq!(BottleneckSeverity::Critical.description(), "매우 높음");
  }
}
