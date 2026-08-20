//! Rewrite 엔진 구조 및 가드 함수
//!
//! pnix-old의 symbolic_core/rewrite/engine.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의 및 가드 함수만, egg 실행 로직 없음
//!
//! ## 설계 원칙
//!
//! - egg는 "스칼라 대수 전용"으로 제한
//! - 텐서가 포함된 표현식은 egg 대상에서 제외
//! - CT 훅으로 rewrite 전후 검증 가능

use super::super::ct::CtContext;
use super::super::expr::{SymExpr, SymKind};
use serde::{Deserialize, Serialize};

/// 순수 스칼라 표현식인지 검사 (가드)
/// 텐서가 포함되어 있으면 egg에서 처리하지 않음
pub fn is_pure_scalar(expr: &SymExpr) -> bool {
  match &expr.kind {
    SymKind::Tensor(_) => false,
    SymKind::Contract(_, _, _) => false,
    SymKind::Raise(_, _) | SymKind::Lower(_, _) => false,
    SymKind::Var(_) | SymKind::Const(_) | SymKind::Exact(_) => true,
    SymKind::Neg(x) => is_pure_scalar(x),
    SymKind::Add(xs) | SymKind::Mul(xs) => xs.iter().all(is_pure_scalar),
    SymKind::Pow(b, e) => is_pure_scalar(b) && is_pure_scalar(e),
    SymKind::Sin(x)
    | SymKind::Cos(x)
    | SymKind::Tan(x)
    | SymKind::Exp(x)
    | SymKind::Log(x)
    | SymKind::Abs(x) => is_pure_scalar(x),
    SymKind::Derivative(_, _) => {
      // Derivative는 결과 타입이 스칼라임을 보장할 수 없음 (텐서 미분 포함).
      // 보수적으로 egg rewrite 대상에서 제외.
      false
    }
  }
}

/// CT 훅 콜백 결과
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CtHookResult {
  /// CT 검증 통과
  Ok,
  /// CT 검증 실패 (경고만, 계속 진행)
  Warning(String),
  /// CT 검증 실패 (중단)
  Error(String),
}

/// CT 훅 설정
#[derive(Clone, Debug, Default)]
pub struct CtHooks {
  /// rewrite 전 CT 검증 활성화
  pub pre_check: bool,
  /// rewrite 후 CT 검증 활성화
  pub post_check: bool,
  /// CT 컨텍스트 (단위/변수 바인딩)
  pub context: CtContext,
  /// 검증 실패 시 동작: true면 에러, false면 경고만
  pub strict: bool,
}

impl CtHooks {
  /// CT 훅 없음 (기본값)
  pub fn none() -> Self {
    Self::default()
  }

  /// 전후 검증 모두 활성화
  pub fn full(ctx: CtContext) -> Self {
    Self {
      pre_check: true,
      post_check: true,
      context: ctx,
      strict: false,
    }
  }

  /// 사후 검증만 활성화
  pub fn post_only(ctx: CtContext) -> Self {
    Self {
      pre_check: false,
      post_check: true,
      context: ctx,
      strict: false,
    }
  }

  /// 엄격 모드 설정
  pub fn with_strict(mut self, strict: bool) -> Self {
    self.strict = strict;
    self
  }
}

/// 단순화 연산 통계
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SimplifyStats {
  /// 실제 사용된 노드 수
  pub nodes_used: usize,
  /// 실제 반복 횟수
  pub iterations_run: usize,
  /// 소요 시간 (밀리초)
  pub elapsed_ms: u64,
  /// 타임아웃 발생 여부
  pub timed_out: bool,
}

impl SimplifyStats {
  /// 새 통계 생성
  pub fn new(nodes: usize, iterations: usize, elapsed_ms: u64, timed_out: bool) -> Self {
    Self {
      nodes_used: nodes,
      iterations_run: iterations,
      elapsed_ms,
      timed_out,
    }
  }
}

/// 리소스 한도 초과 타입
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceLimitKind {
  /// 노드 수 초과
  Nodes,
  /// 반복 횟수 초과
  Iterations,
  /// 타임아웃
  Timeout,
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::approx_constant)]
mod tests {
  use super::*;

  #[test]
  fn test_is_pure_scalar_var() {
    let x = SymExpr::var("x");
    assert!(is_pure_scalar(&x));
  }

  #[test]
  fn test_is_pure_scalar_const() {
    let c = SymExpr::constant(3.14);
    assert!(is_pure_scalar(&c));
  }

  #[test]
  fn test_is_pure_scalar_add() {
    let sum = SymExpr::add(vec![SymExpr::var("x"), SymExpr::var("y")]);
    assert!(is_pure_scalar(&sum));
  }

  #[test]
  fn test_is_pure_scalar_tensor_false() {
    use super::super::super::expr::{IndexPosition, TensorIndex, TensorSymbol};

    let tensor = SymExpr::tensor(TensorSymbol {
      name: "T".to_string(),
      indices: vec![TensorIndex::new("i", IndexPosition::Upper)],
      symmetries: vec![],
    });
    assert!(!is_pure_scalar(&tensor));
  }

  #[test]
  fn test_is_pure_scalar_derivative_false() {
    let expr = SymExpr::derivative(SymExpr::var("x"), "x");
    assert!(!is_pure_scalar(&expr));
  }

  #[test]
  fn test_ct_hooks_none() {
    let hooks = CtHooks::none();
    assert!(!hooks.pre_check);
    assert!(!hooks.post_check);
  }

  #[test]
  fn test_ct_hooks_full() {
    let ctx = CtContext::default();
    let hooks = CtHooks::full(ctx);
    assert!(hooks.pre_check);
    assert!(hooks.post_check);
  }

  #[test]
  fn test_simplify_stats() {
    let stats = SimplifyStats::new(100, 5, 50, false);
    assert_eq!(stats.nodes_used, 100);
    assert_eq!(stats.iterations_run, 5);
    assert_eq!(stats.elapsed_ms, 50);
    assert!(!stats.timed_out);
  }
}
