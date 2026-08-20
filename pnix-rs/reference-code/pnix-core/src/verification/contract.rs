//! Contract definitions and minimal checking helpers.

use super::Invariant;

/// 계약: 함수의 사전 조건과 사후 조건을 정의하는 계약 구조
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Contract {
  /// 사전 조건 목록
  pub preconditions: Vec<Invariant>,
  /// 사후 조건 목록
  pub postconditions: Vec<Invariant>,
}

impl Contract {
  /// 사전 조건 검사 (i64)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 검증만, 값 계산 없음
  pub fn check_pre_i64(&self, value: i64) -> bool {
    self
      .preconditions
      .iter()
      .all(|inv| inv.check_i64(value))
  }

  /// 사후 조건 검사 (i64)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 검증만, 값 계산 없음
  pub fn check_post_i64(&self, value: i64) -> bool {
    self
      .postconditions
      .iter()
      .all(|inv| inv.check_i64(value))
  }

  /// 사전 조건 검사 (Option<i64>)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 검증만, 값 계산 없음
  pub fn check_pre_option_i64(&self, value: Option<i64>) -> bool {
    self
      .preconditions
      .iter()
      .all(|inv| inv.check_option_i64(value))
  }

  /// 사후 조건 검사 (Option<i64>)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 검증만, 값 계산 없음
  pub fn check_post_option_i64(&self, value: Option<i64>) -> bool {
    self
      .postconditions
      .iter()
      .all(|inv| inv.check_option_i64(value))
  }

  /// 사전 조건 검사 (slice i64)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 검증만, 값 계산 없음
  pub fn check_pre_slice_i64(&self, values: &[i64]) -> bool {
    self
      .preconditions
      .iter()
      .all(|inv| inv.check_slice_i64(values))
  }

  /// 사후 조건 검사 (slice i64)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 검증만, 값 계산 없음
  pub fn check_post_slice_i64(&self, values: &[i64]) -> bool {
    self
      .postconditions
      .iter()
      .all(|inv| inv.check_slice_i64(values))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn contract_checks_i64() {
    let contract = Contract {
      preconditions: vec![Invariant::Positive],
      postconditions: vec![Invariant::InRange { min: 0, max: 10 }],
    };
    assert!(contract.check_pre_i64(1));
    assert!(!contract.check_pre_i64(0));
    assert!(contract.check_post_i64(10));
    assert!(!contract.check_post_i64(11));
  }

  #[test]
  fn contract_checks_slice() {
    let contract = Contract {
      preconditions: vec![Invariant::Sorted, Invariant::Unique],
      postconditions: vec![Invariant::Positive],
    };
    assert!(contract.check_pre_slice_i64(&[1, 2, 3]));
    assert!(!contract.check_pre_slice_i64(&[2, 1]));
    assert!(contract.check_post_slice_i64(&[1, 2, 3]));
    assert!(!contract.check_post_slice_i64(&[1, -2]));
  }
}
