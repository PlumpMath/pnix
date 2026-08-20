//! Zone 경계 가드 및 폴백 시스템
//!
//! pnix-old의 symbolic_core/ast/zone.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 런타임 상태 없음
//!
//! ## 설계 원칙
//!
//! - **Go/No-Go 게이트**: 심볼릭 연산 실패 시에도 시스템 전체가 정상 동작
//! - **Graceful Degradation**: Symbolic 실패 → Numeric 자동 전환
//! - **검증 우선**: Zone 전이 전 CT 태그/불변식 검사
//!
//! ## Zone 전이 규칙
//!
//! ```text
//! Symbolic → Numeric: 명시적 `to_numeric()` 또는 실패 폴백
//! Numeric → Symbolic: 금지 (데이터 손실 위험)
//! ```

use super::expr::{SymExpr, SymKind, Zone};
use serde::{Deserialize, Serialize};

/// Zone 전이 결과
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ZoneTransition {
  /// 성공적으로 현재 Zone 유지
  Stayed(Zone),
  /// Symbolic → Numeric 폴백 발생
  FellBack {
    from: Zone,
    to: Zone,
    reason: FallbackReason,
  },
  /// 전이 거부 (Numeric → Symbolic 시도)
  Rejected(TransitionError),
}

/// 폴백 발생 이유
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FallbackReason {
  /// egg rewrite 실패 (타임아웃, 노드 한도 초과 등)
  RewriteFailed(String),
  /// CT 태그 검증 실패
  CtValidationFailed(String),
  /// 텐서 인덱스 불완전
  TensorIndexIncomplete,
  /// 수치 오버플로우/언더플로우
  NumericOverflow,
  /// 명시적 폴백 요청
  ExplicitRequest,
}

/// Zone 전이 에러
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransitionError {
  /// Numeric → Symbolic 전이 시도 (금지됨)
  NumericToSymbolicForbidden,
  /// 이미 같은 Zone
  AlreadyInZone(Zone),
}

impl std::fmt::Display for TransitionError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      TransitionError::NumericToSymbolicForbidden => {
        write!(f, "Numeric → Symbolic 전이는 금지됨 (데이터 손실 위험)")
      }
      TransitionError::AlreadyInZone(z) => {
        write!(f, "이미 {:?} Zone에 있음", z)
      }
    }
  }
}

impl std::error::Error for TransitionError {}

/// 폴백 이벤트 기록
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackEvent {
  pub reason: FallbackReason,
  pub expr_summary: String,
}

/// Zone 진입 에러
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZoneEntryError {
  /// 이미 Numeric Zone
  AlreadyNumeric,
  /// 불완전한 텐서 (자유 인덱스 미해결)
  IncompleteTensor,
  /// 지원하지 않는 텐서 rank
  UnsupportedTensorRank(usize),
  /// CT 카테고리 불일치
  CtCategoryMismatch(String),
}

impl std::fmt::Display for ZoneEntryError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ZoneEntryError::AlreadyNumeric => {
        write!(f, "이미 Numeric Zone에 있는 표현식")
      }
      ZoneEntryError::IncompleteTensor => {
        write!(f, "텐서 인덱스가 불완전함 (자유 인덱스 미해결)")
      }
      ZoneEntryError::UnsupportedTensorRank(r) => {
        write!(f, "지원하지 않는 텐서 rank: {} (최대 4)", r)
      }
      ZoneEntryError::CtCategoryMismatch(msg) => {
        write!(f, "CT 카테고리 불일치: {}", msg)
      }
    }
  }
}

impl std::error::Error for ZoneEntryError {}

// ─────────────────────────────────────────────
// 헬퍼 함수들
// ─────────────────────────────────────────────

/// 표현식이 순수 스칼라인지 확인 (텐서/미분 없음)
pub fn is_scalar_only(expr: &SymExpr) -> bool {
  match &expr.kind {
    SymKind::Var(_) | SymKind::Const(_) | SymKind::Exact(_) => true,
    SymKind::Add(xs) | SymKind::Mul(xs) => xs.iter().all(is_scalar_only),
    SymKind::Pow(a, b) => is_scalar_only(a) && is_scalar_only(b),
    SymKind::Neg(x)
    | SymKind::Sin(x)
    | SymKind::Cos(x)
    | SymKind::Tan(x)
    | SymKind::Exp(x)
    | SymKind::Log(x)
    | SymKind::Abs(x) => is_scalar_only(x),
    // 텐서/미분 관련은 스칼라 아님
    SymKind::Derivative(_, _)
    | SymKind::Tensor(_)
    | SymKind::Contract(_, _, _)
    | SymKind::Raise(_, _)
    | SymKind::Lower(_, _) => false,
  }
}

/// Zone 전환이 가능한지 확인
pub fn can_transition(from: Zone, to: Zone) -> bool {
  match (from, to) {
    (Zone::Symbolic, Zone::Numeric) => true,  // 허용
    (Zone::Numeric, Zone::Symbolic) => false, // 금지
    _ => true,                                // 같은 Zone
  }
}

/// 표현식에서 발견되는 최대 텐서 rank
pub fn max_tensor_rank(expr: &SymExpr) -> usize {
  match &expr.kind {
    SymKind::Tensor(t) => t.indices.len(),
    SymKind::Add(xs) | SymKind::Mul(xs) => xs.iter().map(max_tensor_rank).max().unwrap_or(0),
    SymKind::Pow(a, b) => max_tensor_rank(a).max(max_tensor_rank(b)),
    SymKind::Neg(x)
    | SymKind::Sin(x)
    | SymKind::Cos(x)
    | SymKind::Tan(x)
    | SymKind::Exp(x)
    | SymKind::Log(x)
    | SymKind::Abs(x)
    | SymKind::Derivative(x, _)
    | SymKind::Contract(x, _, _)
    | SymKind::Raise(x, _)
    | SymKind::Lower(x, _) => max_tensor_rank(x),
    SymKind::Var(_) | SymKind::Const(_) | SymKind::Exact(_) => 0,
  }
}

/// 불완전한 텐서 포함 여부 확인
pub fn contains_incomplete_tensor(expr: &SymExpr) -> bool {
  match &expr.kind {
    SymKind::Tensor(t) => {
      // 이름이 빈 인덱스 = 불완전
      t.indices.iter().any(|idx| idx.name.is_empty())
    }
    SymKind::Add(xs) | SymKind::Mul(xs) => xs.iter().any(contains_incomplete_tensor),
    SymKind::Pow(a, b) => contains_incomplete_tensor(a) || contains_incomplete_tensor(b),
    SymKind::Neg(x)
    | SymKind::Sin(x)
    | SymKind::Cos(x)
    | SymKind::Tan(x)
    | SymKind::Exp(x)
    | SymKind::Log(x)
    | SymKind::Abs(x)
    | SymKind::Derivative(x, _)
    | SymKind::Contract(x, _, _)
    | SymKind::Raise(x, _)
    | SymKind::Lower(x, _) => contains_incomplete_tensor(x),
    SymKind::Var(_) | SymKind::Const(_) | SymKind::Exact(_) => false,
  }
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_can_transition() {
    assert!(can_transition(Zone::Symbolic, Zone::Numeric));
    assert!(!can_transition(Zone::Numeric, Zone::Symbolic));
    assert!(can_transition(Zone::Symbolic, Zone::Symbolic));
  }

  #[test]
  fn test_is_scalar_only() {
    // 스칼라 표현식
    let scalar = SymExpr::add(vec![SymExpr::var("x"), SymExpr::constant(1.0)]);
    assert!(is_scalar_only(&scalar));

    // 미분 포함 = 스칼라 아님
    let with_deriv = SymExpr::derivative(SymExpr::var("x"), "t");
    assert!(!is_scalar_only(&with_deriv));
  }

  #[test]
  fn test_max_tensor_rank() {
    let scalar = SymExpr::var("x");
    assert_eq!(max_tensor_rank(&scalar), 0);
  }
}
