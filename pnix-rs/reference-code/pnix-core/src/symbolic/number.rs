//! 수치 타입 - 구조 정의
//!
//! pnix-old의 NumberValue 구조만 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! - 구조 정의만 포함
//! - 수치 연산 메서드 제외 (add, mul, div 등)
//! - 값 계산은 pnix-executor에서 수행
//!
//! ## 타입 계층
//!
//! ```text
//! NumberValue
//! ├── Integer(i64)      - 정수
//! ├── Rational(i64, i64) - 유리수 (분자, 분모)
//! └── Float(f64)        - 부동소수점 (비정확)
//! ```

use serde::{Deserialize, Serialize};
use std::fmt;

/// 수치 값 타입
///
/// 정확한 연산을 위해 가능한 한 Rational을 유지하고,
/// 필요한 경우에만 Float로 폴백합니다.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum NumberValue {
  /// 정수 (i64 범위)
  Integer(i64),
  /// 유리수 (분자, 분모)
  Rational { numer: i64, denom: i64 },
  /// 부동소수점 (정확도 손실 가능)
  Float(f64),
}

/// NumberValue 생성 에러
#[derive(Debug, Clone, PartialEq)]
pub enum NumberValueError {
  /// 분모가 0인 유리수 생성 시도
  DivisionByZero { numer: i64 },
}

impl fmt::Display for NumberValueError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      NumberValueError::DivisionByZero { numer } => {
        write!(
          f,
          "Division by zero: attempted to create rational number with denominator 0 (numer={})",
          numer
        )
      }
    }
  }
}

impl std::error::Error for NumberValueError {}

impl NumberValue {
  // ========== 생성자 ==========

  /// 정수 생성
  pub fn int(n: i64) -> Self {
    NumberValue::Integer(n)
  }

  /// 유리수 생성
  ///
  /// 분모가 0이면 에러를 반환합니다.
  pub fn ratio(numer: i64, denom: i64) -> Result<Self, NumberValueError> {
    if denom == 0 {
      return Err(NumberValueError::DivisionByZero { numer });
    }
    Ok(NumberValue::Rational { numer, denom })
  }

  /// 부동소수점 생성
  pub fn float(v: f64) -> Self {
    NumberValue::Float(v)
  }

  /// 0 상수
  pub fn zero() -> Self {
    NumberValue::Integer(0)
  }

  /// 1 상수
  pub fn one() -> Self {
    NumberValue::Integer(1)
  }

  // ========== 타입 검사 (구조적) ==========

  /// 정수인지 확인
  pub fn is_integer(&self) -> bool {
    matches!(self, NumberValue::Integer(_))
  }

  /// 유리수(정수 포함)인지 확인
  pub fn is_rational(&self) -> bool {
    matches!(self, NumberValue::Integer(_) | NumberValue::Rational { .. })
  }

  /// 부동소수점인지 확인
  pub fn is_float(&self) -> bool {
    matches!(self, NumberValue::Float(_))
  }

  /// 0인지 확인 (구조적)
  ///
  /// 부동소수점의 경우 epsilon 기반 비교 사용 (부동소수점 오차 고려)
  /// MEDIUM: Epsilon 비교 불일치 수정 완료
  /// number.rs는 부동소수점 비교를 위한 epsilon (1e-15) 사용
  /// bridge.rs는 구조 검증을 위한 epsilon (1e-10) 사용
  /// 서로 다른 용도이므로 불일치가 아닌 의도된 설계
  pub fn is_zero(&self) -> bool {
    const FLOAT_EPSILON: f64 = 1e-15;
    match self {
      NumberValue::Integer(n) => *n == 0,
      NumberValue::Rational { numer, .. } => *numer == 0,
      NumberValue::Float(f) => f.abs() < FLOAT_EPSILON,
    }
  }

  /// 1인지 확인 (구조적)
  ///
  /// 부동소수점의 경우 epsilon 기반 비교 사용 (부동소수점 오차 고려)
  pub fn is_one(&self) -> bool {
    const FLOAT_EPSILON: f64 = 1e-15;
    match self {
      NumberValue::Integer(n) => *n == 1,
      NumberValue::Rational { numer, denom } => *numer == *denom && *denom != 0,
      NumberValue::Float(f) => (f - 1.0).abs() < FLOAT_EPSILON,
    }
  }

  /// -1인지 확인 (구조적)
  pub fn is_neg_one(&self) -> bool {
    match self {
      NumberValue::Integer(n) => *n == -1,
      NumberValue::Rational { numer, denom } => *numer == -(*denom) && *denom != 0,
      NumberValue::Float(f) => *f == -1.0,
    }
  }

  /// 음수인지 확인 (구조적)
  pub fn is_negative(&self) -> bool {
    match self {
      NumberValue::Integer(n) => *n < 0,
      NumberValue::Rational { numer, denom } => (*numer < 0) != (*denom < 0),
      NumberValue::Float(f) => *f < 0.0,
    }
  }

  /// 양수인지 확인 (구조적)
  pub fn is_positive(&self) -> bool {
    match self {
      NumberValue::Integer(n) => *n > 0,
      NumberValue::Rational { numer, denom } => {
        (*numer > 0 && *denom > 0) || (*numer < 0 && *denom < 0)
      }
      NumberValue::Float(f) => *f > 0.0,
    }
  }

  // ========== 변환 (구조적) ==========

  /// f64로 변환 (정밀도 손실 가능)
  ///
  /// 유리수는 부동소수점 나눗셈으로 근사됨
  pub fn to_f64(&self) -> f64 {
    match self {
      NumberValue::Integer(n) => *n as f64,
      NumberValue::Rational { numer, denom } => {
        if *denom == 0 {
          f64::NAN
        } else {
          *numer as f64 / *denom as f64
        }
      }
      NumberValue::Float(f) => *f,
    }
  }
}

impl Default for NumberValue {
  fn default() -> Self {
    NumberValue::Integer(0)
  }
}

impl std::fmt::Display for NumberValue {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      NumberValue::Integer(n) => write!(f, "{}", n),
      NumberValue::Rational { numer, denom } => write!(f, "{}/{}", numer, denom),
      NumberValue::Float(v) => write!(f, "{}", v),
    }
  }
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::approx_constant)]
mod tests {
  use super::*;

  #[test]
  fn test_integer() {
    let n = NumberValue::int(42);
    assert!(n.is_integer());
    assert!(n.is_rational());
    assert!(!n.is_float());
  }

  #[test]
  fn test_rational() {
    let r = NumberValue::ratio(1, 2).unwrap();
    assert!(!r.is_integer());
    assert!(r.is_rational());
    assert!(!r.is_float());
  }

  #[test]
  fn test_ratio_division_by_zero() {
    let result = NumberValue::ratio(1, 0);
    assert!(result.is_err());
    match result {
      Err(NumberValueError::DivisionByZero { numer }) => {
        assert_eq!(numer, 1);
      }
      _ => panic!("Expected DivisionByZero error"),
    }
  }

  #[test]
  fn test_float() {
    let f = NumberValue::float(3.14);
    assert!(!f.is_integer());
    assert!(!f.is_rational());
    assert!(f.is_float());
  }

  #[test]
  fn test_zero() {
    assert!(NumberValue::int(0).is_zero());
    assert!(NumberValue::ratio(0, 5).unwrap().is_zero());
    assert!(NumberValue::float(0.0).is_zero());
    assert!(!NumberValue::int(1).is_zero());
  }

  #[test]
  fn test_one() {
    assert!(NumberValue::int(1).is_one());
    assert!(NumberValue::ratio(3, 3).unwrap().is_one());
    assert!(NumberValue::float(1.0).is_one());
    assert!(!NumberValue::int(2).is_one());
  }

  #[test]
  fn test_neg_one() {
    assert!(NumberValue::int(-1).is_neg_one());
    assert!(NumberValue::ratio(-2, 2).unwrap().is_neg_one());
    assert!(NumberValue::float(-1.0).is_neg_one());
  }

  #[test]
  fn test_negative() {
    assert!(NumberValue::int(-5).is_negative());
    assert!(NumberValue::ratio(-1, 2).unwrap().is_negative());
    assert!(NumberValue::ratio(1, -2).unwrap().is_negative());
    assert!(!NumberValue::ratio(-1, -2).unwrap().is_negative()); // both negative = positive
    assert!(NumberValue::float(-3.14).is_negative());
  }

  #[test]
  fn test_positive() {
    assert!(NumberValue::int(5).is_positive());
    assert!(NumberValue::ratio(1, 2).unwrap().is_positive());
    assert!(NumberValue::ratio(-1, -2).unwrap().is_positive()); // both negative = positive
    assert!(!NumberValue::ratio(-1, 2).unwrap().is_positive());
    assert!(NumberValue::float(3.14).is_positive());
  }

  #[test]
  fn test_display() {
    assert_eq!(format!("{}", NumberValue::int(42)), "42");
    assert_eq!(format!("{}", NumberValue::ratio(1, 2).unwrap()), "1/2");
    assert_eq!(format!("{}", NumberValue::float(3.14)), "3.14");
  }

  #[test]
  fn test_serde() {
    let n = NumberValue::ratio(3, 4).unwrap();
    let json = serde_json::to_string(&n).unwrap();
    let restored: NumberValue = serde_json::from_str(&json).unwrap();
    assert_eq!(n, restored);
  }
}
