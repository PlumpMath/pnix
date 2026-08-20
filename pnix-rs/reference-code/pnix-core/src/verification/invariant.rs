//! Invariant definitions and minimal checking helpers.

use std::collections::BTreeSet;

/// 불변식: 값이 만족해야 하는 조건 타입
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Invariant {
  /// null이 아님
  NonNull,
  /// 양수
  Positive,
  /// 범위 내 (min 이상 max 이하)
  InRange {
    /// 최소값
    min: i64,
    /// 최대값
    max: i64,
  },
  /// 정렬됨
  Sorted,
  /// 고유함 (중복 없음)
  Unique,
  /// 사용자 정의 불변식
  Custom(
    /// 불변식 이름
    String
  ),
}

impl Invariant {
  /// 단일 숫자 값에 대한 불변식 검사
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 검증만, 값 계산 없음
  pub fn check_i64(&self, value: i64) -> bool {
    match self {
      Invariant::Positive => value > 0,
      Invariant::InRange { min, max } => value >= *min && value <= *max,
      Invariant::Custom(_) => false,
      Invariant::NonNull | Invariant::Sorted | Invariant::Unique => true,
    }
  }

  /// Optional 숫자 값에 대한 불변식 검사 (NonNull 명시적 처리)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 검증만, 값 계산 없음
  pub fn check_option_i64(&self, value: Option<i64>) -> bool {
    match self {
      Invariant::NonNull => value.is_some(),
      _ => value.map_or(false, |v| self.check_i64(v)),
    }
  }

  /// 숫자 슬라이스에 대한 불변식 검사
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 검증만, 값 계산 없음
  pub fn check_slice_i64(&self, values: &[i64]) -> bool {
    match self {
      Invariant::Sorted => values.windows(2).all(|w| w[0] <= w[1]),
      Invariant::Unique => {
        let mut set = BTreeSet::new();
        values.iter().all(|v| set.insert(*v))
      }
      Invariant::Positive => values.iter().all(|v| *v > 0),
      Invariant::InRange { min, max } => values.iter().all(|v| *v >= *min && *v <= *max),
      Invariant::Custom(_) => false,
      Invariant::NonNull => true,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn invariant_checks_i64() {
    assert!(Invariant::Positive.check_i64(1));
    assert!(!Invariant::Positive.check_i64(0));
    assert!(Invariant::InRange { min: -1, max: 2 }.check_i64(2));
    assert!(!Invariant::InRange { min: -1, max: 2 }.check_i64(3));
  }

  #[test]
  fn invariant_checks_option() {
    assert!(Invariant::NonNull.check_option_i64(Some(1)));
    assert!(!Invariant::NonNull.check_option_i64(None));
    assert!(!Invariant::Positive.check_option_i64(None));
  }

  #[test]
  fn invariant_checks_slice() {
    assert!(Invariant::Sorted.check_slice_i64(&[1, 2, 2, 3]));
    assert!(!Invariant::Sorted.check_slice_i64(&[2, 1]));
    assert!(Invariant::Unique.check_slice_i64(&[1, 2, 3]));
    assert!(!Invariant::Unique.check_slice_i64(&[1, 2, 1]));
    assert!(Invariant::Positive.check_slice_i64(&[1, 2, 3]));
  }
}
