//! Breakpoint 구조 정의
//!
//! pnix-old의 pnix_debug_console/src/breakpoint.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 실행 로직 제외
//! - BreakpointLocation: 브레이크포인트 위치 구조 정의
//! - BreakpointCondition: 브레이크포인트 조건 구조 정의
//! - HitCountOperator: 히트 카운트 연산자 구조 정의
//! - Breakpoint: 브레이크포인트 구조 정의
//! - 실제 실행 로직 (hit(), should_trigger(), check_breakpoint() 등)은 executor에서 구현

use serde::{Deserialize, Serialize};

/// 브레이크포인트 ID
pub type BreakpointId = u64;

/// 브레이크포인트 위치
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BreakpointLocation {
  /// 파일 경로
  pub file: String,
  /// 라인 번호 (1-based)
  pub line: u32,
}

impl BreakpointLocation {
  /// 새로운 브레이크포인트 위치 생성
  pub fn new(file: impl Into<String>, line: u32) -> Self {
    Self {
      file: file.into(),
      line,
    }
  }
}

/// 브레이크포인트 조건
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BreakpointCondition {
  /// 항상 중단
  Always,
  /// 조건이 true일 때만 중단 (표현식 문자열)
  WhenTrue(String),
  /// 조건이 false일 때만 중단 (표현식 문자열)
  WhenFalse(String),
  /// 값이 변경될 때만 중단 (변수 이름)
  WhenChanged(String),
  /// 히트 카운트가 특정 값일 때 중단
  HitCount {
    /// 목표 히트 카운트
    count: u64,
    /// 연산자
    operator: HitCountOperator,
  },
}

/// 히트 카운트 연산자
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HitCountOperator {
  /// 같음
  Equal,
  /// 초과
  GreaterThan,
  /// 이상
  GreaterThanOrEqual,
  /// 미만
  LessThan,
  /// 이하
  LessThanOrEqual,
  /// 배수
  MultipleOf,
}

/// 브레이크포인트 구조
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Breakpoint {
  /// 브레이크포인트 ID
  pub id: BreakpointId,
  /// 위치
  pub location: BreakpointLocation,
  /// 활성화 여부
  pub enabled: bool,
  /// 조건
  pub condition: BreakpointCondition,
  /// 히트 카운트 (구조 정의만, 실제 증가는 executor에서)
  pub hit_count: u64,
  /// 무시할 히트 카운트 (조건부 브레이크포인트용)
  pub ignore_count: u64,
  /// 브레이크포인트가 트리거되었을 때 실행할 액션 (문자열)
  pub action: Option<String>,
}

impl Breakpoint {
  /// 새로운 브레이크포인트 생성
  pub fn new(id: BreakpointId, location: BreakpointLocation) -> Self {
    Self {
      id,
      location,
      enabled: true,
      condition: BreakpointCondition::Always,
      hit_count: 0,
      ignore_count: 0,
      action: None,
    }
  }

  /// 조건부 브레이크포인트 생성 (구조 변경만)
  pub fn with_condition(mut self, condition: BreakpointCondition) -> Self {
    self.condition = condition;
    self
  }
}

// 헌법 준수 (P0-1): 실행 로직 제거
// 다음 함수들은 executor/runtime 계층에서 구현하세요:
// - hit() (히트 카운트 증가)
// - should_trigger() -> bool (트리거 여부 확인, 표현식 평가 포함)
// - BreakpointManager (상태 관리)
// - check_breakpoint() (실행 중 브레이크포인트 확인)
//
// 이 함수들은 값 계산, 상태 변경, 또는 실행 로직을 수행하므로 pnix-core에서 제외됩니다.

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_breakpoint_location() {
    let loc = BreakpointLocation::new("test.rs", 10);
    assert_eq!(loc.file, "test.rs");
    assert_eq!(loc.line, 10);
  }

  #[test]
  fn test_breakpoint_creation() {
    let loc = BreakpointLocation::new("test.rs", 10);
    let bp = Breakpoint::new(1, loc);
    assert_eq!(bp.id, 1);
    assert!(bp.enabled);
    assert_eq!(bp.hit_count, 0);
  }

  #[test]
  fn test_breakpoint_with_condition() {
    let loc = BreakpointLocation::new("test.rs", 10);
    let bp =
      Breakpoint::new(1, loc).with_condition(BreakpointCondition::WhenTrue("x > 0".to_string()));
    match bp.condition {
      BreakpointCondition::WhenTrue(ref expr) => assert_eq!(expr, "x > 0"),
      _ => panic!("Expected WhenTrue condition"),
    }
  }
}
