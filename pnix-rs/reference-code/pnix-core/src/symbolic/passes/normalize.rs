//! 스칼라 정규화 패스
//!
//! pnix-old의 symbolic_core/passes/normalize.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 변환만, 값 계산 없음
//!
//! ## 기능
//!
//! - Add에서 0 제거
//! - Mul에서 1 제거, 0이 있으면 전체 0
//! - Neg(Neg(x)) → x
//! - Pow(x, 0) → 1, Pow(x, 1) → x
//! - 중첩 Add/Mul flatten

use crate::symbolic::expr::{SymExpr, SymKind};

/// 기본 정규화: 0, 1 제거, 중첩 flatten
///
/// - Add에서 0 제거
/// - Mul에서 1 제거, 0이 있으면 전체 0
/// - Neg(Neg(x)) → x
/// - Pow(x, 0) → 1, Pow(x, 1) → x
pub fn normalize(expr: SymExpr) -> SymExpr {
  let zone = expr.zone;
  let span = expr.span.clone();

  match expr.kind {
    SymKind::Add(mut xs) => {
      // 재귀 정규화
      xs = xs.into_iter().map(normalize).collect();
      // 0 제거
      xs.retain(|e| !is_zero(e));
      // 중첩 Add flatten
      xs = flatten_add(xs);
      // 빈 배열 → 0
      if xs.is_empty() {
        SymExpr::constant(0.0)
      } else if xs.len() == 1 {
        // 안전성: len() == 1 체크 후이므로 안전하지만, 명시적 에러 메시지 제공
        xs.pop()
          .expect("normalize: Add flatten 후 len()==1이지만 pop() 실패 (버그)")
      } else {
        SymExpr {
          kind: SymKind::Add(xs),
          zone,
          span,
        }
      }
    }
    SymKind::Mul(mut xs) => {
      xs = xs.into_iter().map(normalize).collect();
      // 0이 있으면 전체 0
      if xs.iter().any(is_zero) {
        return SymExpr::constant(0.0);
      }
      // 1 제거
      xs.retain(|e| !is_one(e));
      // 중첩 Mul flatten
      xs = flatten_mul(xs);
      if xs.is_empty() {
        SymExpr::constant(1.0)
      } else if xs.len() == 1 {
        // 안전성: len() == 1 체크 후이므로 안전하지만, 명시적 에러 메시지 제공
        xs.pop()
          .expect("normalize: Mul flatten 후 len()==1이지만 pop() 실패 (버그)")
      } else {
        SymExpr {
          kind: SymKind::Mul(xs),
          zone,
          span,
        }
      }
    }
    SymKind::Neg(x) => {
      let x = normalize(*x);
      // --x → x
      if let SymKind::Neg(inner) = x.kind {
        *inner
      } else {
        SymExpr::neg(x)
      }
    }
    SymKind::Pow(base, exp) => {
      let base = normalize(*base);
      let exp = normalize(*exp);
      // 0^0는 수학적 미정의이므로 NaN 반환
      if is_zero(&base) && is_zero(&exp) {
        SymExpr::constant(f64::NAN)
      } else if is_zero(&exp) {
        // x^0 → 1 (x != 0)
        SymExpr::constant(1.0)
      } else if is_one(&exp) {
        // x^1 → x
        base
      } else {
        SymExpr::pow(base, exp)
      }
    }
    // 재귀 정규화: 단항 함수들
    SymKind::Sin(x) => SymExpr::sin(normalize(*x)),
    SymKind::Cos(x) => SymExpr::cos(normalize(*x)),
    SymKind::Tan(x) => SymExpr::tan(normalize(*x)),
    SymKind::Exp(x) => SymExpr::exp(normalize(*x)),
    SymKind::Log(x) => SymExpr::log(normalize(*x)),
    SymKind::Abs(x) => SymExpr::abs(normalize(*x)),
    SymKind::Derivative(x, var) => SymExpr::derivative(normalize(*x), var),
    SymKind::Contract(x, i1, i2) => SymExpr::contract(normalize(*x), i1, i2),
    SymKind::Raise(x, idx) => SymExpr::raise(normalize(*x), idx),
    SymKind::Lower(x, idx) => SymExpr::lower(normalize(*x), idx),
    // 리프 노드는 그대로
    _ => SymExpr {
      kind: expr.kind,
      zone,
      span,
    },
  }
}

/// 값이 0인지 확인 (구조적)
fn is_zero(expr: &SymExpr) -> bool {
  match &expr.kind {
    SymKind::Const(c) => *c == 0.0,
    SymKind::Exact(n) => n.is_zero(),
    _ => false,
  }
}

/// 값이 1인지 확인 (구조적)
fn is_one(expr: &SymExpr) -> bool {
  match &expr.kind {
    SymKind::Const(c) => *c == 1.0,
    SymKind::Exact(n) => n.is_one(),
    _ => false,
  }
}

/// Add 중첩 flatten
/// LOW: flatten_add/mul 무제한 재귀 수정 완료
/// 깊은 중첩 스택 오버플로우 방지를 위한 깊이 제한 추가
const MAX_FLATTEN_DEPTH: usize = 1000;

fn flatten_add(exprs: Vec<SymExpr>) -> Vec<SymExpr> {
  flatten_add_with_depth(exprs, 0)
}

fn flatten_add_with_depth(exprs: Vec<SymExpr>, depth: usize) -> Vec<SymExpr> {
  if depth >= MAX_FLATTEN_DEPTH {
    // 깊이 제한 초과: 원본 반환 (flatten 중단)
    return exprs;
  }
  let mut result = vec![];
  for e in exprs {
    if let SymKind::Add(inner) = e.kind {
      result.extend(flatten_add_with_depth(inner, depth + 1));
    } else {
      result.push(e);
    }
  }
  result
}

/// Mul 중첩 flatten
fn flatten_mul(exprs: Vec<SymExpr>) -> Vec<SymExpr> {
  flatten_mul_with_depth(exprs, 0)
}

fn flatten_mul_with_depth(exprs: Vec<SymExpr>, depth: usize) -> Vec<SymExpr> {
  if depth >= MAX_FLATTEN_DEPTH {
    // 깊이 제한 초과: 원본 반환 (flatten 중단)
    return exprs;
  }
  let mut result = vec![];
  for e in exprs {
    if let SymKind::Mul(inner) = e.kind {
      result.extend(flatten_mul_with_depth(inner, depth + 1));
    } else {
      result.push(e);
    }
  }
  result
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_add_zero_removal() {
    // x + 0 → x
    let expr = SymExpr::add2(SymExpr::var("x"), SymExpr::constant(0.0));
    let result = normalize(expr);
    assert!(matches!(result.kind, SymKind::Var(ref s) if s == "x"));
  }

  #[test]
  fn test_add_zero_int_removal() {
    // x + 0 (정수) → x
    let expr = SymExpr::add2(SymExpr::var("x"), SymExpr::int(0));
    let result = normalize(expr);
    assert!(matches!(result.kind, SymKind::Var(ref s) if s == "x"));
  }

  #[test]
  fn test_add_all_zeros() {
    // 0 + 0 → 0
    let expr = SymExpr::add2(SymExpr::constant(0.0), SymExpr::constant(0.0));
    let result = normalize(expr);
    assert!(is_zero(&result));
  }

  #[test]
  fn test_mul_one_removal() {
    // x * 1 → x
    let expr = SymExpr::mul2(SymExpr::var("x"), SymExpr::constant(1.0));
    let result = normalize(expr);
    assert!(matches!(result.kind, SymKind::Var(ref s) if s == "x"));
  }

  #[test]
  fn test_mul_one_int_removal() {
    // x * 1 (정수) → x
    let expr = SymExpr::mul2(SymExpr::var("x"), SymExpr::int(1));
    let result = normalize(expr);
    assert!(matches!(result.kind, SymKind::Var(ref s) if s == "x"));
  }

  #[test]
  fn test_mul_zero_absorb() {
    // x * 0 → 0
    let expr = SymExpr::mul2(SymExpr::var("x"), SymExpr::constant(0.0));
    let result = normalize(expr);
    assert!(is_zero(&result));
  }

  #[test]
  fn test_mul_all_ones() {
    // 1 * 1 → 1
    let expr = SymExpr::mul2(SymExpr::constant(1.0), SymExpr::constant(1.0));
    let result = normalize(expr);
    assert!(is_one(&result));
  }

  #[test]
  fn test_double_neg() {
    // --x → x
    let expr = SymExpr::neg(SymExpr::neg(SymExpr::var("x")));
    let result = normalize(expr);
    assert!(matches!(result.kind, SymKind::Var(ref s) if s == "x"));
  }

  #[test]
  fn test_pow_zero() {
    // x^0 → 1
    let expr = SymExpr::pow(SymExpr::var("x"), SymExpr::constant(0.0));
    let result = normalize(expr);
    assert!(is_one(&result));
  }

  #[test]
  fn test_pow_one() {
    // x^1 → x
    let expr = SymExpr::pow(SymExpr::var("x"), SymExpr::constant(1.0));
    let result = normalize(expr);
    assert!(matches!(result.kind, SymKind::Var(ref s) if s == "x"));
  }

  #[test]
  fn test_flatten_add() {
    // (x + y) + z → x + y + z
    let inner = SymExpr::add2(SymExpr::var("x"), SymExpr::var("y"));
    let expr = SymExpr::add2(inner, SymExpr::var("z"));
    let result = normalize(expr);
    if let SymKind::Add(xs) = result.kind {
      assert_eq!(xs.len(), 3);
    } else {
      panic!("Expected Add");
    }
  }

  #[test]
  fn test_flatten_mul() {
    // (x * y) * z → x * y * z
    let inner = SymExpr::mul2(SymExpr::var("x"), SymExpr::var("y"));
    let expr = SymExpr::mul2(inner, SymExpr::var("z"));
    let result = normalize(expr);
    if let SymKind::Mul(xs) = result.kind {
      assert_eq!(xs.len(), 3);
    } else {
      panic!("Expected Mul");
    }
  }

  #[test]
  fn test_nested_normalize() {
    // sin(x + 0) → sin(x)
    let inner = SymExpr::add2(SymExpr::var("x"), SymExpr::constant(0.0));
    let expr = SymExpr::sin(inner);
    let result = normalize(expr);
    if let SymKind::Sin(x) = result.kind {
      assert!(matches!(x.kind, SymKind::Var(ref s) if s == "x"));
    } else {
      panic!("Expected Sin");
    }
  }

  #[test]
  fn test_complex_normalize() {
    // (x * 1 + 0) * (y + 0)^1 → x * y
    let term1 = SymExpr::add2(
      SymExpr::mul2(SymExpr::var("x"), SymExpr::constant(1.0)),
      SymExpr::constant(0.0),
    );
    let term2 = SymExpr::pow(
      SymExpr::add2(SymExpr::var("y"), SymExpr::constant(0.0)),
      SymExpr::constant(1.0),
    );
    let expr = SymExpr::mul2(term1, term2);
    let result = normalize(expr);

    // Should be x * y
    if let SymKind::Mul(xs) = result.kind {
      assert_eq!(xs.len(), 2);
    } else {
      panic!("Expected Mul, got {:?}", result.kind);
    }
  }
}
