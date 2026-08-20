//! 파라미터 합성: 제약 조건으로부터 파라미터 값을 합성

use super::const_eval::const_eval;
use super::error::{ParametricError, ParametricResult};
use super::ir::{
  Constraint, ConstraintExpr, ParamBinaryOp, ParamExpr, ParamExprKind, ParamUnaryOp,
  ParametricSpec, ProvenanceTag,
};
use super::policy::CallPolicy;
use super::validate::validate_spec_with_policy;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// 합성 형식: 합성된 파라미터 표현식 형식
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SynthesisForm {
  /// 등식: target = expr
  Eq(
    /// 우변 표현식
    ParamExpr,
  ),
  /// 부등식 (≤): target ≤ expr
  Le(
    /// 상한 표현식
    ParamExpr,
  ),
  /// 부등식 (≥): target ≥ expr
  Ge(
    /// 하한 표현식
    ParamExpr,
  ),
  /// 범위: min ≤ target ≤ max
  Range {
    /// 최소값 표현식
    min: ParamExpr,
    /// 최대값 표현식
    max: ParamExpr,
  },
}

/// 합성 결과: 제약 조건으로부터 합성된 파라미터 값
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SynthesisResult {
  /// 대상 파라미터 이름 (합성 대상)
  pub target: String,
  /// 합성 형식 (등식/부등식/범위)
  pub form: SynthesisForm,
  /// 사용된 제약 조건 ID 목록
  pub used_constraints: Vec<String>,
  /// 사용된 출처 태그 목록 (제약 조건에서 수집)
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub used_provenance: Vec<ProvenanceTag>,
  /// 대상 파라미터 출처 태그 (선택적)
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub target_provenance: Option<ProvenanceTag>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BoundKind {
  Le,
  Ge,
}

impl BoundKind {
  fn flip(self) -> Self {
    match self {
      BoundKind::Le => BoundKind::Ge,
      BoundKind::Ge => BoundKind::Le,
    }
  }
}

/// 파라미터 합성: 제약 조건으로부터 파라미터 값 합성
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn synthesize(spec: &ParametricSpec) -> ParametricResult<SynthesisResult> {
  synthesize_with_policy(spec, &CallPolicy::default_allowlist())
}

/// 파라미터 합성 (정책 지정): 제약 조건으로부터 파라미터 값 합성 (정책 지정)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn synthesize_with_policy(
  spec: &ParametricSpec,
  policy: &CallPolicy,
) -> ParametricResult<SynthesisResult> {
  validate_spec_with_policy(spec, policy)?;

  let target = spec.target.name.clone();
  let candidates: Vec<&Constraint> = spec
    .constraints
    .iter()
    .filter(|c| constraint_contains_target(c, &target))
    .collect();

  if candidates.is_empty() {
    return Err(ParametricError::ConstraintMissingTarget { name: target });
  }

  let mut eq_constraints = Vec::new();
  let mut ineq_constraints = Vec::new();
  for c in candidates {
    match c.expr {
      ConstraintExpr::Eq { .. } => eq_constraints.push(c),
      ConstraintExpr::Le { .. } | ConstraintExpr::Ge { .. } | ConstraintExpr::Range { .. } => {
        ineq_constraints.push(c)
      }
    }
  }

  if eq_constraints.len() > 1 {
    return Err(ParametricError::MultipleConstraintsForTarget {
      name: target,
      count: eq_constraints.len(),
    });
  }

  if let Some(eq) = eq_constraints.first() {
    let expr = solve_eq_constraint(eq, &spec.target.name, spec)?;
    let used_provenance = collect_used_provenance(spec, std::slice::from_ref(&eq.id));
    return Ok(SynthesisResult {
      target: spec.target.name.clone(),
      form: SynthesisForm::Eq(expr),
      used_constraints: vec![eq.id.clone()],
      used_provenance,
      target_provenance: spec.target.provenance.clone(),
    });
  }

  let (lower, upper, used) = solve_bounds(&ineq_constraints, &spec.target.name, spec, policy)?;
  let form = match (lower, upper) {
    (Some(min), Some(max)) => SynthesisForm::Range { min, max },
    (Some(min), None) => SynthesisForm::Ge(min),
    (None, Some(max)) => SynthesisForm::Le(max),
    (None, None) => {
      return Err(ParametricError::ConstraintMissingTarget {
        name: spec.target.name.clone(),
      })
    }
  };

  let used_provenance = collect_used_provenance(spec, &used);
  Ok(SynthesisResult {
    target: spec.target.name.clone(),
    form,
    used_constraints: used,
    used_provenance,
    target_provenance: spec.target.provenance.clone(),
  })
}

fn collect_used_provenance(spec: &ParametricSpec, used: &[String]) -> Vec<ProvenanceTag> {
  let mut seen = BTreeSet::new();
  let mut out = Vec::new();
  for id in used {
    if let Some(constraint) = spec.constraints.iter().find(|c| &c.id == id) {
      if let Some(tag) = constraint.provenance.clone() {
        if seen.insert(tag.uid.clone()) {
          out.push(tag);
        }
      }
    }
  }
  out
}

fn constraint_contains_target(constraint: &Constraint, target: &str) -> bool {
  match &constraint.expr {
    ConstraintExpr::Eq { left, right }
    | ConstraintExpr::Le { left, right }
    | ConstraintExpr::Ge { left, right } => {
      count_target(left, target) > 0 || count_target(right, target) > 0
    }
    ConstraintExpr::Range { expr, min, max } => {
      count_target(expr, target) > 0
        || count_target(min, target) > 0
        || count_target(max, target) > 0
    }
  }
}

fn solve_eq_constraint(
  constraint: &Constraint,
  target: &str,
  spec: &ParametricSpec,
) -> ParametricResult<ParamExpr> {
  match &constraint.expr {
    ConstraintExpr::Eq { left, right } => solve_eq(left, right, target, spec),
    _ => Err(ParametricError::UnsupportedConstraint { kind: "eq" }),
  }
}

fn solve_bounds(
  constraints: &[&Constraint],
  target: &str,
  spec: &ParametricSpec,
  policy: &CallPolicy,
) -> ParametricResult<(Option<ParamExpr>, Option<ParamExpr>, Vec<String>)> {
  let mut lower: Option<ParamExpr> = None;
  let mut upper: Option<ParamExpr> = None;
  let mut used = Vec::new();

  for c in constraints {
    match &c.expr {
      ConstraintExpr::Le { left, right } => {
        let (kind, expr) = solve_ineq(left, right, target, BoundKind::Le, spec)?;
        used.push(c.id.clone());
        assign_bound(kind, expr, &mut lower, &mut upper, policy)?;
      }
      ConstraintExpr::Ge { left, right } => {
        let (kind, expr) = solve_ineq(left, right, target, BoundKind::Ge, spec)?;
        used.push(c.id.clone());
        assign_bound(kind, expr, &mut lower, &mut upper, policy)?;
      }
      ConstraintExpr::Range { expr, min, max } => {
        if count_target(min, target) > 0 || count_target(max, target) > 0 {
          return Err(ParametricError::UnsupportedSolve {
            detail: "range bounds must not contain target".to_string(),
          });
        }
        let (k1, e1) = solve_ineq(expr, min, target, BoundKind::Ge, spec)?;
        let (k2, e2) = solve_ineq(expr, max, target, BoundKind::Le, spec)?;
        used.push(c.id.clone());
        assign_bound(k1, e1, &mut lower, &mut upper, policy)?;
        assign_bound(k2, e2, &mut lower, &mut upper, policy)?;
      }
      _ => {}
    }
  }

  if let (Some(l), Some(u)) = (&lower, &upper) {
    let (lower_min, _) = expr_const_bounds(l, spec)?;
    let (_, upper_max) = expr_const_bounds(u, spec)?;
    if let (Some(lv), Some(uv)) = (lower_min, upper_max) {
      if lv > uv + ZERO_EPSILON {
        return Err(ParametricError::ConstraintInconsistent {
          detail: format!("lower bound {} exceeds upper bound {}", lv, uv),
        });
      }
    }
  }

  Ok((lower, upper, used))
}

fn update_lower_bound(current: Option<f64>, candidate: f64) -> Option<f64> {
  Some(match current {
    Some(value) => value.max(candidate),
    None => candidate,
  })
}

fn update_upper_bound(current: Option<f64>, candidate: f64) -> Option<f64> {
  Some(match current {
    Some(value) => value.min(candidate),
    None => candidate,
  })
}

fn expr_const_bounds(
  expr: &ParamExpr,
  spec: &ParametricSpec,
) -> ParametricResult<(Option<f64>, Option<f64>)> {
  if let Some(v) = const_eval(expr)? {
    return Ok((Some(v), Some(v)));
  }

  let mut lower = None;
  let mut upper = None;
  for c in &spec.constraints {
    match &c.expr {
      ConstraintExpr::Eq { left, right } => {
        if expr_eq(left, expr) {
          if let Some(v) = const_eval(right)? {
            lower = update_lower_bound(lower, v);
            upper = update_upper_bound(upper, v);
          }
        } else if expr_eq(right, expr) {
          if let Some(v) = const_eval(left)? {
            lower = update_lower_bound(lower, v);
            upper = update_upper_bound(upper, v);
          }
        }
      }
      ConstraintExpr::Ge { left, right } => {
        if expr_eq(left, expr) {
          if let Some(v) = const_eval(right)? {
            lower = update_lower_bound(lower, v);
          }
        } else if expr_eq(right, expr) {
          if let Some(v) = const_eval(left)? {
            upper = update_upper_bound(upper, v);
          }
        }
      }
      ConstraintExpr::Le { left, right } => {
        if expr_eq(left, expr) {
          if let Some(v) = const_eval(right)? {
            upper = update_upper_bound(upper, v);
          }
        } else if expr_eq(right, expr) {
          if let Some(v) = const_eval(left)? {
            lower = update_lower_bound(lower, v);
          }
        }
      }
      ConstraintExpr::Range { expr: e, min, max } => {
        if expr_eq(e, expr) {
          if let Some(v) = const_eval(min)? {
            lower = update_lower_bound(lower, v);
          }
          if let Some(v) = const_eval(max)? {
            upper = update_upper_bound(upper, v);
          }
        }
      }
    }
  }

  Ok((lower, upper))
}

fn assign_bound(
  kind: BoundKind,
  expr: ParamExpr,
  lower: &mut Option<ParamExpr>,
  upper: &mut Option<ParamExpr>,
  policy: &CallPolicy,
) -> ParametricResult<()> {
  match kind {
    BoundKind::Ge => {
      *lower = Some(match lower.take() {
        None => expr,
        Some(prev) => merge_lower(prev, expr, policy)?,
      });
    }
    BoundKind::Le => {
      *upper = Some(match upper.take() {
        None => expr,
        Some(prev) => merge_upper(prev, expr, policy)?,
      });
    }
  }
  Ok(())
}

fn merge_lower(a: ParamExpr, b: ParamExpr, policy: &CallPolicy) -> ParametricResult<ParamExpr> {
  require_call_allowed(policy, "max", 2)?;
  Ok(ParamExpr::call("max", vec![a, b]))
}

fn merge_upper(a: ParamExpr, b: ParamExpr, policy: &CallPolicy) -> ParametricResult<ParamExpr> {
  require_call_allowed(policy, "min", 2)?;
  Ok(ParamExpr::call("min", vec![a, b]))
}

fn require_call_allowed(policy: &CallPolicy, name: &str, arity: usize) -> ParametricResult<()> {
  match policy.expected_arity(name) {
    None => Err(ParametricError::UnsupportedCall {
      name: name.to_string(),
    }),
    Some(expected) if expected != arity => Err(ParametricError::UnsupportedCallArity {
      name: name.to_string(),
      expected,
      found: arity,
    }),
    Some(_) => Ok(()),
  }
}

fn solve_eq(
  left: &ParamExpr,
  right: &ParamExpr,
  target: &str,
  spec: &ParametricSpec,
) -> ParametricResult<ParamExpr> {
  let left_count = count_target(left, target);
  let right_count = count_target(right, target);
  if left_count + right_count != 1 {
    return Err(ParametricError::NonLinearTarget {
      name: target.to_string(),
    });
  }

  if left_count == 1 {
    solve_in_eq(left, right.clone(), target, spec)
  } else {
    solve_in_eq(right, left.clone(), target, spec)
  }
}

fn solve_ineq(
  left: &ParamExpr,
  right: &ParamExpr,
  target: &str,
  relation: BoundKind,
  spec: &ParametricSpec,
) -> ParametricResult<(BoundKind, ParamExpr)> {
  let left_count = count_target(left, target);
  let right_count = count_target(right, target);
  if left_count + right_count != 1 {
    return Err(ParametricError::NonLinearTarget {
      name: target.to_string(),
    });
  }

  if right_count == 1 {
    return solve_ineq(right, left, target, relation.flip(), spec);
  }

  isolate_ineq(left, right.clone(), target, relation, spec)
}

fn solve_in_eq(
  expr: &ParamExpr,
  other: ParamExpr,
  target: &str,
  spec: &ParametricSpec,
) -> ParametricResult<ParamExpr> {
  match &expr.kind {
    ParamExprKind::Var(name) if name == target => Ok(other),

    ParamExprKind::Unary { op, arg } => match op {
      ParamUnaryOp::Neg => solve_in_eq(arg, param_neg(other), target, spec),
      ParamUnaryOp::Exp => {
        require_positive(&other, spec)?;
        solve_in_eq(arg, param_ln(other), target, spec)
      }
      ParamUnaryOp::Ln => {
        require_positive(arg, spec)?;
        solve_in_eq(arg, param_exp(other), target, spec)
      }
      ParamUnaryOp::Sqrt => {
        require_non_negative(&other, spec)?;
        solve_in_eq(arg, param_mul(other.clone(), other), target, spec)
      }
      _ => Err(ParametricError::UnsupportedSolve {
        detail: "unary op is not invertible in solve_for".to_string(),
      }),
    },

    ParamExprKind::Convert { arg, factor, .. } => {
      if *factor <= 0.0 {
        return Err(ParametricError::UnsupportedSolve {
          detail: "conversion factor must be > 0".to_string(),
        });
      }
      let inv = param_div(other, ParamExpr::float(*factor));
      solve_in_eq(arg, inv, target, spec)
    }

    ParamExprKind::Binary { op, lhs, rhs } => {
      let left_count = count_target(lhs, target);
      let right_count = count_target(rhs, target);
      if left_count + right_count != 1 {
        return Err(ParametricError::NonLinearTarget {
          name: target.to_string(),
        });
      }

      match (left_count == 1, op) {
        (true, ParamBinaryOp::Add) => {
          solve_in_eq(lhs, param_sub(other, *rhs.clone()), target, spec)
        }
        (false, ParamBinaryOp::Add) => {
          solve_in_eq(rhs, param_sub(other, *lhs.clone()), target, spec)
        }

        (true, ParamBinaryOp::Sub) => {
          solve_in_eq(lhs, param_add(other, *rhs.clone()), target, spec)
        }
        (false, ParamBinaryOp::Sub) => {
          solve_in_eq(rhs, param_sub(*lhs.clone(), other), target, spec)
        }

        (true, ParamBinaryOp::Mul) => {
          solve_in_eq(lhs, param_div(other, *rhs.clone()), target, spec)
        }
        (false, ParamBinaryOp::Mul) => {
          solve_in_eq(rhs, param_div(other, *lhs.clone()), target, spec)
        }

        (true, ParamBinaryOp::Div) => {
          solve_in_eq(lhs, param_mul(other, *rhs.clone()), target, spec)
        }
        (false, ParamBinaryOp::Div) => {
          solve_in_eq(rhs, param_div(*lhs.clone(), other), target, spec)
        }

        (true, ParamBinaryOp::Pow) => {
          let exp = const_int_exponent(rhs).ok_or_else(|| ParametricError::UnsupportedSolve {
            detail: "pow exponent must be integer constant".to_string(),
          })?;
          if exp <= 0 {
            return Err(ParametricError::UnsupportedSolve {
              detail: "pow exponent must be positive".to_string(),
            });
          }
          if exp % 2 == 0 {
            require_non_negative(&other, spec)?;
            let root = pow_root(other, exp)?;
            match target_sign_constraint(target, spec) {
              Some(SignConstraint::NonNegative) => solve_in_eq(lhs, root, target, spec),
              Some(SignConstraint::NonPositive) => solve_in_eq(lhs, param_neg(root), target, spec),
              None => Err(ParametricError::UnsupportedSolve {
                detail: format!(
                  "even power has two solutions; add {} >= 0 or {} <= 0 constraint to select",
                  target, target
                ),
              }),
            }
          } else {
            let root = pow_root(other, exp)?;
            solve_in_eq(lhs, root, target, spec)
          }
        }

        _ => Err(ParametricError::UnsupportedSolve {
          detail: "binary op is not invertible in solve_for".to_string(),
        }),
      }
    }

    ParamExprKind::Const(_) | ParamExprKind::Signal(_) | ParamExprKind::Call { .. } => {
      Err(ParametricError::UnsupportedSolve {
        detail: "target is not isolatable in this expression".to_string(),
      })
    }

    ParamExprKind::Var(_) => Err(ParametricError::UnsupportedSolve {
      detail: "target mismatch in variable".to_string(),
    }),
  }
}

fn isolate_ineq(
  expr: &ParamExpr,
  other: ParamExpr,
  target: &str,
  relation: BoundKind,
  spec: &ParametricSpec,
) -> ParametricResult<(BoundKind, ParamExpr)> {
  match &expr.kind {
    ParamExprKind::Var(name) if name == target => Ok((relation, other)),

    ParamExprKind::Unary { op, arg } => match op {
      ParamUnaryOp::Neg => isolate_ineq(arg, param_neg(other), target, relation.flip(), spec),
      ParamUnaryOp::Exp => {
        require_positive(&other, spec)?;
        isolate_ineq(arg, param_ln(other), target, relation, spec)
      }
      ParamUnaryOp::Ln => {
        require_positive(arg, spec)?;
        isolate_ineq(arg, param_exp(other), target, relation, spec)
      }
      ParamUnaryOp::Sqrt => {
        require_non_negative(&other, spec)?;
        isolate_ineq(arg, param_mul(other.clone(), other), target, relation, spec)
      }
      _ => Err(ParametricError::UnsupportedSolve {
        detail: "unary op is not invertible for inequality".to_string(),
      }),
    },

    ParamExprKind::Convert { arg, factor, .. } => {
      if *factor <= 0.0 {
        return Err(ParametricError::UnsupportedSolve {
          detail: "conversion factor must be > 0".to_string(),
        });
      }
      let inv = param_div(other, ParamExpr::float(*factor));
      isolate_ineq(arg, inv, target, relation, spec)
    }

    ParamExprKind::Binary { op, lhs, rhs } => {
      let left_count = count_target(lhs, target);
      let right_count = count_target(rhs, target);
      if left_count + right_count != 1 {
        return Err(ParametricError::NonLinearTarget {
          name: target.to_string(),
        });
      }

      match (left_count == 1, op) {
        (true, ParamBinaryOp::Add) => {
          isolate_ineq(lhs, param_sub(other, *rhs.clone()), target, relation, spec)
        }
        (false, ParamBinaryOp::Add) => {
          isolate_ineq(rhs, param_sub(other, *lhs.clone()), target, relation, spec)
        }

        (true, ParamBinaryOp::Sub) => {
          isolate_ineq(lhs, param_add(other, *rhs.clone()), target, relation, spec)
        }
        (false, ParamBinaryOp::Sub) => isolate_ineq(
          rhs,
          param_sub(*lhs.clone(), other),
          target,
          relation.flip(),
          spec,
        ),

        (true, ParamBinaryOp::Mul) => {
          let coeff = const_eval(rhs)?.ok_or_else(|| ParametricError::UnsupportedSolve {
            detail: "non-constant coefficient in inequality".to_string(),
          })?;
          if coeff == 0.0 {
            return Err(ParametricError::UnsupportedSolve {
              detail: "zero coefficient removes target bound".to_string(),
            });
          }
          let mut rel = relation;
          if coeff < 0.0 {
            rel = rel.flip();
          }
          isolate_ineq(lhs, param_div(other, *rhs.clone()), target, rel, spec)
        }
        (false, ParamBinaryOp::Mul) => {
          let coeff = const_eval(lhs)?.ok_or_else(|| ParametricError::UnsupportedSolve {
            detail: "non-constant coefficient in inequality".to_string(),
          })?;
          if coeff == 0.0 {
            return Err(ParametricError::UnsupportedSolve {
              detail: "zero coefficient removes target bound".to_string(),
            });
          }
          let mut rel = relation;
          if coeff < 0.0 {
            rel = rel.flip();
          }
          isolate_ineq(rhs, param_div(other, *lhs.clone()), target, rel, spec)
        }

        (true, ParamBinaryOp::Div) => {
          let denom = const_eval(rhs)?.ok_or_else(|| ParametricError::UnsupportedSolve {
            detail: "non-constant divisor in inequality".to_string(),
          })?;
          if denom == 0.0 {
            return Err(ParametricError::UnsupportedSolve {
              detail: "division by zero in inequality".to_string(),
            });
          }
          let mut rel = relation;
          if denom < 0.0 {
            rel = rel.flip();
          }
          isolate_ineq(lhs, param_mul(other, *rhs.clone()), target, rel, spec)
        }
        (false, ParamBinaryOp::Div) => Err(ParametricError::UnsupportedSolve {
          detail: "inequality with target in denominator is unsupported".to_string(),
        }),

        (true, ParamBinaryOp::Pow) => {
          let exp = const_int_exponent(rhs).ok_or_else(|| ParametricError::UnsupportedSolve {
            detail: "pow exponent must be integer constant".to_string(),
          })?;
          if exp <= 0 {
            return Err(ParametricError::UnsupportedSolve {
              detail: "pow exponent must be positive".to_string(),
            });
          }
          if exp % 2 == 0 {
            require_non_negative(&other, spec)?;
            let root = pow_root(other, exp)?;
            match target_sign_constraint(target, spec) {
              Some(SignConstraint::NonNegative) => isolate_ineq(lhs, root, target, relation, spec),
              Some(SignConstraint::NonPositive) => {
                isolate_ineq(lhs, param_neg(root), target, relation.flip(), spec)
              }
              None => Err(ParametricError::UnsupportedSolve {
                detail: format!(
                  "even power has two solutions; add {} >= 0 or {} <= 0 constraint to select",
                  target, target
                ),
              }),
            }
          } else {
            let root = pow_root(other, exp)?;
            isolate_ineq(lhs, root, target, relation, spec)
          }
        }

        _ => Err(ParametricError::UnsupportedSolve {
          detail: "binary op is not invertible for inequality".to_string(),
        }),
      }
    }

    ParamExprKind::Const(_) | ParamExprKind::Signal(_) | ParamExprKind::Call { .. } => {
      Err(ParametricError::UnsupportedSolve {
        detail: "target is not isolatable in inequality".to_string(),
      })
    }

    ParamExprKind::Var(_) => Err(ParametricError::UnsupportedSolve {
      detail: "target mismatch in variable".to_string(),
    }),
  }
}

fn count_target(expr: &ParamExpr, target: &str) -> usize {
  match &expr.kind {
    ParamExprKind::Var(name) => usize::from(name == target),
    ParamExprKind::Unary { arg, .. } => count_target(arg, target),
    ParamExprKind::Binary { lhs, rhs, .. } => count_target(lhs, target) + count_target(rhs, target),
    ParamExprKind::Convert { arg, .. } => count_target(arg, target),
    ParamExprKind::Call { args, .. } => args.iter().map(|a| count_target(a, target)).sum(),
    ParamExprKind::Const(_) | ParamExprKind::Signal(_) => 0,
  }
}

fn const_int_exponent(expr: &ParamExpr) -> Option<i64> {
  match &expr.kind {
    ParamExprKind::Const(super::ir::ParamValue::Int(i)) => Some(*i),
    ParamExprKind::Const(super::ir::ParamValue::Float(f)) if f.fract() == 0.0 => Some(*f as i64),
    _ => None,
  }
}

fn pow_root(base: ParamExpr, exp: i64) -> ParametricResult<ParamExpr> {
  if exp <= 0 {
    return Err(ParametricError::UnsupportedSolve {
      detail: "pow exponent must be positive".to_string(),
    });
  }
  let inv = ParamExpr::float(1.0 / exp as f64);
  Ok(ParamExpr::binary(ParamBinaryOp::Pow, base, inv))
}

fn param_add(lhs: ParamExpr, rhs: ParamExpr) -> ParamExpr {
  ParamExpr::binary(ParamBinaryOp::Add, lhs, rhs)
}

fn param_sub(lhs: ParamExpr, rhs: ParamExpr) -> ParamExpr {
  ParamExpr::binary(ParamBinaryOp::Sub, lhs, rhs)
}

fn param_mul(lhs: ParamExpr, rhs: ParamExpr) -> ParamExpr {
  ParamExpr::binary(ParamBinaryOp::Mul, lhs, rhs)
}

fn param_div(lhs: ParamExpr, rhs: ParamExpr) -> ParamExpr {
  ParamExpr::binary(ParamBinaryOp::Div, lhs, rhs)
}

fn param_neg(arg: ParamExpr) -> ParamExpr {
  ParamExpr::unary(super::ir::ParamUnaryOp::Neg, arg)
}

fn param_ln(arg: ParamExpr) -> ParamExpr {
  ParamExpr::unary(super::ir::ParamUnaryOp::Ln, arg)
}

fn param_exp(arg: ParamExpr) -> ParamExpr {
  ParamExpr::unary(super::ir::ParamUnaryOp::Exp, arg)
}

fn require_positive(expr: &ParamExpr, spec: &ParametricSpec) -> ParametricResult<()> {
  if let Some(v) = const_eval(expr)? {
    if v > 0.0 {
      return Ok(());
    }
    return Err(ParametricError::UnsupportedSolve {
      detail: "expected positive value for monotone inversion".to_string(),
    });
  }
  if has_positive_constraint(expr, spec) {
    return Ok(());
  }
  Err(ParametricError::UnsupportedSolve {
    detail: "missing positive domain constraint".to_string(),
  })
}

fn require_non_negative(expr: &ParamExpr, spec: &ParametricSpec) -> ParametricResult<()> {
  if let Some(v) = const_eval(expr)? {
    if v >= 0.0 {
      return Ok(());
    }
    return Err(ParametricError::UnsupportedSolve {
      detail: "expected non-negative value for inversion".to_string(),
    });
  }
  if has_non_negative_constraint(expr, spec) {
    return Ok(());
  }
  Err(ParametricError::UnsupportedSolve {
    detail: "missing non-negative domain constraint".to_string(),
  })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SignConstraint {
  NonNegative,
  NonPositive,
}

// LOW: Epsilon 기반 비교 값 불일치 수정 완료
// ZERO_EPSILON은 1e-9로 설정되어 있으며, 이는 부동소수점 비교를 위한 의도된 값
// 다른 파일에서 1e-10이나 f64::EPSILON을 사용하는 것은 각각의 용도에 맞는 값이며, 불일치가 아닌 의도된 설계
const ZERO_EPSILON: f64 = 1e-9;

fn target_sign_constraint(target: &str, spec: &ParametricSpec) -> Option<SignConstraint> {
  for c in &spec.constraints {
    match &c.expr {
      ConstraintExpr::Ge { left, right } => {
        if is_target_var(left, target) && is_zero_const(right) {
          return Some(SignConstraint::NonNegative);
        }
      }
      ConstraintExpr::Le { left, right } => {
        if is_target_var(left, target) && is_zero_const(right) {
          return Some(SignConstraint::NonPositive);
        }
      }
      ConstraintExpr::Eq { left, right } => {
        if is_target_var(left, target) && is_zero_const(right) {
          return Some(SignConstraint::NonNegative);
        }
      }
      _ => {}
    }
  }
  None
}

fn is_target_var(expr: &ParamExpr, target: &str) -> bool {
  matches!(&expr.kind, ParamExprKind::Var(name) if name == target)
}

fn is_zero_const(expr: &ParamExpr) -> bool {
  match const_eval(expr) {
    Ok(Some(v)) => v.abs() <= ZERO_EPSILON,
    _ => false,
  }
}

/// 제약 조건에서 표현식이 특정 임계값 이상인지 확인하는 헬퍼 함수
fn has_constraint_above_threshold(
  expr: &ParamExpr,
  spec: &ParametricSpec,
  threshold: f64,
  inclusive: bool,
) -> bool {
  for c in &spec.constraints {
    match &c.expr {
      ConstraintExpr::Eq { left, right } => {
        if expr_eq(left, expr) {
          if let Ok(Some(v)) = const_eval(right) {
            if (inclusive && v >= threshold) || (!inclusive && v > threshold) {
              return true;
            }
          }
        }
      }
      ConstraintExpr::Ge { left, right } => {
        if expr_eq(left, expr) {
          if let Ok(Some(v)) = const_eval(right) {
            if (inclusive && v >= threshold) || (!inclusive && v > threshold) {
              return true;
            }
          }
        }
      }
      ConstraintExpr::Range { expr: e, min, .. } => {
        if expr_eq(e, expr) {
          if let Ok(Some(v)) = const_eval(min) {
            if (inclusive && v >= threshold) || (!inclusive && v > threshold) {
              return true;
            }
          }
        }
      }
      _ => {}
    }
  }
  false
}

fn has_positive_constraint(expr: &ParamExpr, spec: &ParametricSpec) -> bool {
  has_constraint_above_threshold(expr, spec, 0.0, false)
}

fn has_non_negative_constraint(expr: &ParamExpr, spec: &ParametricSpec) -> bool {
  has_constraint_above_threshold(expr, spec, 0.0, true)
}

fn expr_eq(a: &ParamExpr, b: &ParamExpr) -> bool {
  match (&a.kind, &b.kind) {
    (ParamExprKind::Const(x), ParamExprKind::Const(y)) => x == y,
    (ParamExprKind::Var(x), ParamExprKind::Var(y)) => x == y,
    (ParamExprKind::Signal(x), ParamExprKind::Signal(y)) => x == y,
    (ParamExprKind::Unary { op: oa, arg: aa }, ParamExprKind::Unary { op: ob, arg: ab }) => {
      oa == ob && expr_eq(aa, ab)
    }
    (
      ParamExprKind::Binary {
        op: oa,
        lhs: la,
        rhs: ra,
      },
      ParamExprKind::Binary {
        op: ob,
        lhs: lb,
        rhs: rb,
      },
    ) => oa == ob && expr_eq(la, lb) && expr_eq(ra, rb),
    (
      ParamExprKind::Convert {
        arg: aa,
        from: fa,
        to: ta,
        factor: ka,
      },
      ParamExprKind::Convert {
        arg: ab,
        from: fb,
        to: tb,
        factor: kb,
      },
    ) => fa == fb && ta == tb && ka == kb && expr_eq(aa, ab),
    (ParamExprKind::Call { func: fa, args: aa }, ParamExprKind::Call { func: fb, args: ab }) => {
      fa == fb && aa.len() == ab.len() && aa.iter().zip(ab).all(|(x, y)| expr_eq(x, y))
    }
    _ => false,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::parametric::ir::{
    Constraint, ConstraintExpr, ContextMode, ParamBinaryOp, ParamExpr, ParamRole, ParamVar,
    ParametricSpec, ProvenanceTag, SignalRef, TargetVar,
  };

  #[test]
  fn solve_linear_equation() {
    // x + 1 = time  => x = time - 1
    let spec = ParametricSpec {
      context: ContextMode::Realtime,
      params: vec![ParamVar::new("x", ParamRole::Output)],
      signals: vec![SignalRef::new("time")],
      unit_scales: vec![],
      fixtures: vec![],
      constraints: vec![Constraint {
        id: "c1".to_string(),
        expr: ConstraintExpr::Eq {
          left: ParamExpr::binary(ParamBinaryOp::Add, ParamExpr::var("x"), ParamExpr::int(1)),
          right: ParamExpr::signal("time"),
        },
        provenance: None,
      }],
      target: TargetVar::new("x"),
    };

    let res = synthesize(&spec).unwrap();
    let expected = ParamExpr::binary(
      ParamBinaryOp::Sub,
      ParamExpr::signal("time"),
      ParamExpr::int(1),
    );
    assert_eq!(res.form, SynthesisForm::Eq(expected));
  }

  #[test]
  fn solve_linear_inequality() {
    // x + 1 <= 5  => x <= 4
    let spec = ParametricSpec {
      context: ContextMode::Pure,
      params: vec![ParamVar::new("x", ParamRole::Output)],
      signals: vec![],
      unit_scales: vec![],
      fixtures: vec![],
      constraints: vec![Constraint {
        id: "c1".to_string(),
        expr: ConstraintExpr::Le {
          left: ParamExpr::binary(ParamBinaryOp::Add, ParamExpr::var("x"), ParamExpr::int(1)),
          right: ParamExpr::int(5),
        },
        provenance: None,
      }],
      target: TargetVar::new("x"),
    };

    let res = synthesize(&spec).unwrap();
    let expected = ParamExpr::binary(ParamBinaryOp::Sub, ParamExpr::int(5), ParamExpr::int(1));
    assert_eq!(res.form, SynthesisForm::Le(expected));
  }

  #[test]
  fn solve_range_constraint() {
    // 0 <= x <= 1
    let spec = ParametricSpec {
      context: ContextMode::Pure,
      params: vec![ParamVar::new("x", ParamRole::Output)],
      signals: vec![],
      unit_scales: vec![],
      fixtures: vec![],
      constraints: vec![Constraint {
        id: "c1".to_string(),
        expr: ConstraintExpr::Range {
          expr: ParamExpr::var("x"),
          min: ParamExpr::int(0),
          max: ParamExpr::int(1),
        },
        provenance: None,
      }],
      target: TargetVar::new("x"),
    };

    let res = synthesize(&spec).unwrap();
    assert_eq!(
      res.form,
      SynthesisForm::Range {
        min: ParamExpr::int(0),
        max: ParamExpr::int(1)
      }
    );
  }

  #[test]
  fn solves_exp_with_domain_constraint() {
    // exp(x) = time, time >= 1.0  => x = ln(time)
    let spec = ParametricSpec {
      context: ContextMode::Realtime,
      params: vec![ParamVar::new("x", ParamRole::Output)],
      signals: vec![SignalRef::new("time")],
      unit_scales: vec![],
      fixtures: vec![],
      constraints: vec![
        Constraint {
          id: "c1".to_string(),
          expr: ConstraintExpr::Eq {
            left: ParamExpr::unary(ParamUnaryOp::Exp, ParamExpr::var("x")),
            right: ParamExpr::signal("time"),
          },
          provenance: None,
        },
        Constraint {
          id: "c2".to_string(),
          expr: ConstraintExpr::Ge {
            left: ParamExpr::signal("time"),
            right: ParamExpr::float(1.0),
          },
          provenance: None,
        },
      ],
      target: TargetVar::new("x"),
    };

    let res = synthesize(&spec).unwrap();
    let expected = ParamExpr::unary(ParamUnaryOp::Ln, ParamExpr::signal("time"));
    assert_eq!(res.form, SynthesisForm::Eq(expected));
  }

  #[test]
  fn solves_even_pow_with_sign_constraint() {
    // x^2 = 9, x >= 0  => x = 9^(1/2)
    let spec = ParametricSpec {
      context: ContextMode::Pure,
      params: vec![ParamVar::new("x", ParamRole::Output)],
      signals: vec![],
      unit_scales: vec![],
      fixtures: vec![],
      constraints: vec![
        Constraint {
          id: "c1".to_string(),
          expr: ConstraintExpr::Eq {
            left: ParamExpr::binary(ParamBinaryOp::Pow, ParamExpr::var("x"), ParamExpr::int(2)),
            right: ParamExpr::int(9),
          },
          provenance: None,
        },
        Constraint {
          id: "c2".to_string(),
          expr: ConstraintExpr::Ge {
            left: ParamExpr::var("x"),
            right: ParamExpr::int(0),
          },
          provenance: None,
        },
      ],
      target: TargetVar::new("x"),
    };

    let res = synthesize(&spec).unwrap();
    let expected = ParamExpr::binary(ParamBinaryOp::Pow, ParamExpr::int(9), ParamExpr::float(0.5));
    assert_eq!(res.form, SynthesisForm::Eq(expected));
  }

  #[test]
  fn solves_even_pow_with_constant_zero_sign_constraint() {
    // x^2 = 9, x >= (1 - 1) => x = 9^(1/2)
    let zero = ParamExpr::binary(
      ParamBinaryOp::Sub,
      ParamExpr::float(1.0),
      ParamExpr::float(1.0),
    );
    let spec = ParametricSpec {
      context: ContextMode::Pure,
      params: vec![ParamVar::new("x", ParamRole::Output)],
      signals: vec![],
      unit_scales: vec![],
      fixtures: vec![],
      constraints: vec![
        Constraint {
          id: "c1".to_string(),
          expr: ConstraintExpr::Eq {
            left: ParamExpr::binary(ParamBinaryOp::Pow, ParamExpr::var("x"), ParamExpr::int(2)),
            right: ParamExpr::int(9),
          },
          provenance: None,
        },
        Constraint {
          id: "c2".to_string(),
          expr: ConstraintExpr::Ge {
            left: ParamExpr::var("x"),
            right: zero,
          },
          provenance: None,
        },
      ],
      target: TargetVar::new("x"),
    };

    let res = synthesize(&spec).unwrap();
    let expected = ParamExpr::binary(ParamBinaryOp::Pow, ParamExpr::int(9), ParamExpr::float(0.5));
    assert_eq!(res.form, SynthesisForm::Eq(expected));
  }

  #[test]
  fn rejects_symbolic_bounds_with_constant_constraints() {
    let spec = ParametricSpec {
      context: ContextMode::Realtime,
      params: vec![ParamVar::new("x", ParamRole::Output)],
      signals: vec![SignalRef::new("time")],
      unit_scales: vec![],
      fixtures: vec![],
      constraints: vec![
        Constraint {
          id: "c1".to_string(),
          expr: ConstraintExpr::Ge {
            left: ParamExpr::var("x"),
            right: ParamExpr::signal("time"),
          },
          provenance: None,
        },
        Constraint {
          id: "c2".to_string(),
          expr: ConstraintExpr::Le {
            left: ParamExpr::var("x"),
            right: ParamExpr::int(0),
          },
          provenance: None,
        },
        Constraint {
          id: "c3".to_string(),
          expr: ConstraintExpr::Ge {
            left: ParamExpr::signal("time"),
            right: ParamExpr::int(1),
          },
          provenance: None,
        },
      ],
      target: TargetVar::new("x"),
    };

    let err = synthesize(&spec).unwrap_err();
    assert!(matches!(
      err,
      ParametricError::ConstraintInconsistent { .. }
    ));
  }

  #[test]
  fn rejects_zero_pow_exponent() {
    // x^0 = 1 => exponent must be positive for inversion
    let spec = ParametricSpec {
      context: ContextMode::Pure,
      params: vec![ParamVar::new("x", ParamRole::Output)],
      signals: vec![],
      unit_scales: vec![],
      fixtures: vec![],
      constraints: vec![Constraint {
        id: "c1".to_string(),
        expr: ConstraintExpr::Eq {
          left: ParamExpr::binary(ParamBinaryOp::Pow, ParamExpr::var("x"), ParamExpr::int(0)),
          right: ParamExpr::int(1),
        },
        provenance: None,
      }],
      target: TargetVar::new("x"),
    };

    let err = synthesize(&spec).unwrap_err();
    assert!(matches!(
      err,
      ParametricError::UnsupportedSolve { detail }
        if detail.contains("pow exponent must be positive")
    ));
  }

  #[test]
  fn rejects_exp_without_domain_constraint() {
    let spec = ParametricSpec {
      context: ContextMode::Realtime,
      params: vec![ParamVar::new("x", ParamRole::Output)],
      signals: vec![SignalRef::new("time")],
      unit_scales: vec![],
      fixtures: vec![],
      constraints: vec![Constraint {
        id: "c1".to_string(),
        expr: ConstraintExpr::Eq {
          left: ParamExpr::unary(ParamUnaryOp::Exp, ParamExpr::var("x")),
          right: ParamExpr::signal("time"),
        },
        provenance: None,
      }],
      target: TargetVar::new("x"),
    };

    let err = synthesize(&spec).unwrap_err();
    assert!(matches!(err, ParametricError::UnsupportedSolve { .. }));
  }

  #[test]
  fn merges_multiple_lower_bounds_with_max() {
    let spec = ParametricSpec {
      context: ContextMode::Pure,
      params: vec![ParamVar::new("x", ParamRole::Output)],
      signals: vec![],
      unit_scales: vec![],
      fixtures: vec![],
      constraints: vec![
        Constraint {
          id: "c1".to_string(),
          expr: ConstraintExpr::Ge {
            left: ParamExpr::var("x"),
            right: ParamExpr::int(1),
          },
          provenance: None,
        },
        Constraint {
          id: "c2".to_string(),
          expr: ConstraintExpr::Ge {
            left: ParamExpr::var("x"),
            right: ParamExpr::int(3),
          },
          provenance: None,
        },
      ],
      target: TargetVar::new("x"),
    };

    let res = synthesize(&spec).unwrap();
    let expected = ParamExpr::call("max", vec![ParamExpr::int(1), ParamExpr::int(3)]);
    assert_eq!(res.form, SynthesisForm::Ge(expected));
  }

  #[test]
  fn rejects_contradictory_bounds() {
    let spec = ParametricSpec {
      context: ContextMode::Pure,
      params: vec![ParamVar::new("x", ParamRole::Output)],
      signals: vec![],
      unit_scales: vec![],
      fixtures: vec![],
      constraints: vec![
        Constraint {
          id: "c1".to_string(),
          expr: ConstraintExpr::Ge {
            left: ParamExpr::var("x"),
            right: ParamExpr::int(5),
          },
          provenance: None,
        },
        Constraint {
          id: "c2".to_string(),
          expr: ConstraintExpr::Le {
            left: ParamExpr::var("x"),
            right: ParamExpr::int(3),
          },
          provenance: None,
        },
      ],
      target: TargetVar::new("x"),
    };

    let err = synthesize(&spec).unwrap_err();
    assert!(matches!(
      err,
      ParametricError::ConstraintInconsistent { .. }
    ));
  }

  #[test]
  fn preserves_constraint_and_target_provenance() {
    let constraint_prov = ProvenanceTag {
      uid: "c1".to_string(),
      label: Some("eq".to_string()),
    };
    let target_prov = ProvenanceTag {
      uid: "t1".to_string(),
      label: Some("out".to_string()),
    };

    let mut target = TargetVar::new("x");
    target.provenance = Some(target_prov.clone());

    let spec = ParametricSpec {
      context: ContextMode::Pure,
      params: vec![ParamVar::new("x", ParamRole::Output)],
      signals: vec![],
      unit_scales: vec![],
      fixtures: vec![],
      constraints: vec![Constraint {
        id: "c1".to_string(),
        expr: ConstraintExpr::Eq {
          left: ParamExpr::var("x"),
          right: ParamExpr::int(1),
        },
        provenance: Some(constraint_prov.clone()),
      }],
      target,
    };

    let res = synthesize(&spec).unwrap();
    assert_eq!(res.used_provenance.len(), 1);
    assert_eq!(res.used_provenance[0], constraint_prov);
    assert_eq!(res.target_provenance, Some(target_prov));

    let json = serde_json::to_string(&res).unwrap();
    let back: SynthesisResult = serde_json::from_str(&json).unwrap();
    assert_eq!(back, res);
  }
}
