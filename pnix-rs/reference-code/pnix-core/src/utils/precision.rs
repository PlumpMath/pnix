//! Symbolic Precision Types
//!
//! pnix-old의 symbolic_core/src/ast/precision.rs에서 마이그레이션
//!
//! ## 헌법 준수 (P0-1)
//!
//! 순수 마커 타입, 값 연산 없음
//!
//! ## 설계 원칙
//!
//! phantom type으로 Exact/Approx 정밀도를 컴파일 타임에 구분합니다.
//!
//! - `Exact`: 정확한 변환, 손실 없음
//! - `Approx`: 근사 변환 결과, 정밀도 손실 가능

use serde::{Deserialize, Serialize};

/// 심볼릭 정밀도 마커 trait: 심볼릭 변환의 정밀도를 나타내는 마커 trait
///
/// sealed trait 패턴으로 외부 구현을 방지합니다.
/// 헌법 P0-1 준수: 구조 정의만, 값 계산 없음
pub trait SymbolicPrecision: private::Sealed + Clone + Copy + Default {
  /// 정밀도 이름 (디버깅/로깅용)
  fn name() -> &'static str;

  /// 정확한 변환인지
  fn is_exact() -> bool;
}

mod private {
  pub trait Sealed {}
  impl Sealed for super::Exact {}
  impl Sealed for super::Approx {}
}

/// 정확한 변환 마커: 모든 연산이 정확한 변환을 나타내는 마커 타입
///
/// - 모든 연산이 정확함 (손실 없음)
/// - 코드 생성 허용
/// 헌법 P0-1 준수: 구조 정의만, 값 계산 없음
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Exact;

impl SymbolicPrecision for Exact {
  fn name() -> &'static str {
    "Exact"
  }
  fn is_exact() -> bool {
    true
  }
}

/// 근사 변환 결과 마커: 일부 연산에서 근사가 발생한 변환을 나타내는 마커 타입
///
/// - 일부 연산에서 근사 발생
/// - 분석/디버깅 용도로만 사용
/// 헌법 P0-1 준수: 구조 정의만, 값 계산 없음
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Approx;

impl SymbolicPrecision for Approx {
  fn name() -> &'static str {
    "Approx"
  }
  fn is_exact() -> bool {
    false
  }
}

/// 정밀도 변환 결과: 연산 결과의 정밀도를 나타내는 enum
///
/// 연산 결과가 Exact인지 Approx인지 런타임에 판단해야 할 때 사용합니다.
/// 헌법 P0-1 준수: 구조 정의만, 값 계산 없음
#[derive(Clone, Debug, PartialEq)]
pub enum PrecisionResult<T> {
  /// 정확한 결과
  Exact(
    /// 결과 값
    T,
  ),
  /// 근사 결과 (근사 이유 포함)
  Approx(
    /// 결과 값
    T,
    /// 근사 이유
    ApproxReason,
  ),
}

impl<T> PrecisionResult<T> {
  /// 정확한 결과인지
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn is_exact(&self) -> bool {
    matches!(self, PrecisionResult::Exact(_))
  }

  /// 근사 결과인지
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn is_approx(&self) -> bool {
    matches!(self, PrecisionResult::Approx(_, _))
  }

  /// 내부 값 추출
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변환만, 값 계산 없음
  pub fn into_inner(self) -> T {
    match self {
      PrecisionResult::Exact(t) => t,
      PrecisionResult::Approx(t, _) => t,
    }
  }

  /// 내부 값 참조
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn as_inner(&self) -> &T {
    match self {
      PrecisionResult::Exact(t) => t,
      PrecisionResult::Approx(t, _) => t,
    }
  }

  /// 근사 이유 반환 (있으면)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn approx_reason(&self) -> Option<&ApproxReason> {
    match self {
      PrecisionResult::Exact(_) => None,
      PrecisionResult::Approx(_, reason) => Some(reason),
    }
  }

  /// map 변환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변환만, 값 계산 없음
  pub fn map<U>(self, f: impl FnOnce(T) -> U) -> PrecisionResult<U> {
    match self {
      PrecisionResult::Exact(t) => PrecisionResult::Exact(f(t)),
      PrecisionResult::Approx(t, reason) => PrecisionResult::Approx(f(t), reason),
    }
  }
}

/// 근사 발생 이유: 근사 변환이 발생한 이유 타입
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApproxReason {
  /// 큰 지수 (limit 초과)
  LargeExponent {
    /// 지수 값
    exp: i64,
    /// 제한 값
    limit: i64,
  },
  /// 비정수 지수 (문자열로 저장, f64는 Eq 미지원)
  NonIntegerExponent {
    /// 지수 문자열 표현
    exp_str: String,
  },
  /// 타임아웃 (불완전 처리)
  Timeout {
    /// 실행된 iteration 수
    iterations: usize,
  },
  /// 수치 근사 (부동소수점 연산)
  NumericApprox,
  /// 정밀도 손실
  PrecisionLoss {
    /// 손실 원인
    source: String,
  },
  /// 기타
  Other(
    /// 이유 설명
    String,
  ),
}

impl ApproxReason {
  /// 비정수 지수 근사 이유 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn non_integer_exponent(exp: f64) -> Self {
    Self::NonIntegerExponent {
      exp_str: format!("{}", exp),
    }
  }

  /// 큰 지수 근사 이유 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn large_exponent(exp: i64, limit: i64) -> Self {
    Self::LargeExponent { exp, limit }
  }

  /// 타임아웃 근사 이유 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn timeout(iterations: usize) -> Self {
    Self::Timeout { iterations }
  }

  /// 정밀도 손실 근사 이유 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn precision_loss(source: impl Into<String>) -> Self {
    Self::PrecisionLoss {
      source: source.into(),
    }
  }
}

impl std::fmt::Display for ApproxReason {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ApproxReason::LargeExponent { exp, limit } => {
        write!(f, "large exponent {} exceeds limit {}", exp, limit)
      }
      ApproxReason::NonIntegerExponent { exp_str } => {
        write!(f, "non-integer exponent {}", exp_str)
      }
      ApproxReason::Timeout { iterations } => {
        write!(f, "timeout after {} iterations", iterations)
      }
      ApproxReason::NumericApprox => {
        write!(f, "numeric approximation")
      }
      ApproxReason::PrecisionLoss { source } => {
        write!(f, "precision loss from {}", source)
      }
      ApproxReason::Other(msg) => {
        write!(f, "{}", msg)
      }
    }
  }
}

/// 정밀도 추적 정보: 런타임에 정밀도를 추적하기 위한 정보 구조
///
/// 헌법 P0-1 준수: 구조 정의만, 값 계산 없음
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrecisionInfo {
  /// 정확한 연산인지
  pub is_exact: bool,
  /// 근사가 발생했다면 그 이유들
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub approx_reasons: Vec<ApproxReason>,
}

impl PrecisionInfo {
  /// 정확한 정밀도 정보 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn exact() -> Self {
    Self {
      is_exact: true,
      approx_reasons: vec![],
    }
  }

  /// 근사 정밀도 정보 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn approx(reason: ApproxReason) -> Self {
    Self {
      is_exact: false,
      approx_reasons: vec![reason],
    }
  }

  /// 두 정밀도 정보를 병합 (하나라도 approx면 approx)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 병합만, 값 계산 없음
  pub fn merge(self, other: Self) -> Self {
    let is_exact = self.is_exact && other.is_exact;
    let mut reasons = self.approx_reasons;
    reasons.extend(other.approx_reasons);
    Self {
      is_exact,
      approx_reasons: reasons,
    }
  }

  /// 근사 이유 추가
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn add_reason(&mut self, reason: ApproxReason) {
    self.is_exact = false;
    self.approx_reasons.push(reason);
  }

  /// 정확한지 여부
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn is_exact(&self) -> bool {
    self.is_exact
  }

  /// 근사인지 여부
  pub fn is_approx(&self) -> bool {
    !self.is_exact
  }
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_precision_traits() {
    assert_eq!(Exact::name(), "Exact");
    assert!(Exact::is_exact());

    assert_eq!(Approx::name(), "Approx");
    assert!(!Approx::is_exact());
  }

  #[test]
  fn test_precision_result_exact() {
    let exact: PrecisionResult<i32> = PrecisionResult::Exact(42);
    assert!(exact.is_exact());
    assert!(!exact.is_approx());
    assert_eq!(exact.as_inner(), &42);
    assert!(exact.approx_reason().is_none());
    assert_eq!(exact.into_inner(), 42);
  }

  #[test]
  fn test_precision_result_approx() {
    let approx: PrecisionResult<i32> =
      PrecisionResult::Approx(100, ApproxReason::LargeExponent { exp: 10, limit: 8 });
    assert!(!approx.is_exact());
    assert!(approx.is_approx());
    assert_eq!(approx.as_inner(), &100);
    assert!(approx.approx_reason().is_some());
    assert_eq!(approx.into_inner(), 100);
  }

  #[test]
  fn test_precision_result_map() {
    let exact: PrecisionResult<i32> = PrecisionResult::Exact(10);
    let mapped = exact.map(|x| x * 2);
    assert!(mapped.is_exact());
    assert_eq!(mapped.into_inner(), 20);

    let approx: PrecisionResult<i32> = PrecisionResult::Approx(10, ApproxReason::NumericApprox);
    let mapped = approx.map(|x| x * 2);
    assert!(mapped.is_approx());
    assert_eq!(mapped.into_inner(), 20);
  }

  #[test]
  fn test_approx_reason_display() {
    assert_eq!(
      ApproxReason::large_exponent(10, 8).to_string(),
      "large exponent 10 exceeds limit 8"
    );
    assert_eq!(
      ApproxReason::non_integer_exponent(0.5).to_string(),
      "non-integer exponent 0.5"
    );
    assert_eq!(
      ApproxReason::timeout(100).to_string(),
      "timeout after 100 iterations"
    );
    assert_eq!(
      ApproxReason::NumericApprox.to_string(),
      "numeric approximation"
    );
    assert_eq!(
      ApproxReason::precision_loss("float conversion").to_string(),
      "precision loss from float conversion"
    );
  }

  #[test]
  fn test_precision_info_exact() {
    let info = PrecisionInfo::exact();
    assert!(info.is_exact());
    assert!(!info.is_approx());
    assert!(info.approx_reasons.is_empty());
  }

  #[test]
  fn test_precision_info_approx() {
    let info = PrecisionInfo::approx(ApproxReason::NumericApprox);
    assert!(!info.is_exact());
    assert!(info.is_approx());
    assert_eq!(info.approx_reasons.len(), 1);
  }

  #[test]
  fn test_precision_info_merge() {
    let a = PrecisionInfo::exact();
    let b = PrecisionInfo::approx(ApproxReason::NumericApprox);
    let merged = a.merge(b);

    assert!(!merged.is_exact());
    assert_eq!(merged.approx_reasons.len(), 1);
  }

  #[test]
  fn test_precision_info_merge_both_exact() {
    let a = PrecisionInfo::exact();
    let b = PrecisionInfo::exact();
    let merged = a.merge(b);

    assert!(merged.is_exact());
    assert!(merged.approx_reasons.is_empty());
  }

  #[test]
  fn test_precision_info_add_reason() {
    let mut info = PrecisionInfo::exact();
    assert!(info.is_exact());

    info.add_reason(ApproxReason::NumericApprox);
    assert!(!info.is_exact());
    assert_eq!(info.approx_reasons.len(), 1);

    info.add_reason(ApproxReason::timeout(50));
    assert_eq!(info.approx_reasons.len(), 2);
  }

  #[test]
  fn test_precision_info_serde() {
    let info = PrecisionInfo::approx(ApproxReason::large_exponent(100, 10));
    let json = serde_json::to_string(&info).unwrap();
    let restored: PrecisionInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(info, restored);
  }
}
