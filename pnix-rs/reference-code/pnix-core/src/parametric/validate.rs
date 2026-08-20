//! Parametric 검증: 파라미터 스펙의 유효성 검증

use std::collections::{BTreeMap, BTreeSet};

use super::const_eval::const_eval;
use super::error::{ParametricError, ParametricResult};
use super::ir::{
  Constraint, ConstraintExpr, ContextMode, ParamBinaryOp, ParamExpr, ParamExprKind, ParamUnaryOp,
  ParamValue, ParametricSpec, SignalRef, Unit, UnitScale,
};
use super::policy::CallPolicy;

/// 허용된 시그널 이름 목록
const ALLOWED_SIGNALS: &[&str] = &["time", "t", "dt", "delta_time", "frame", "fps", "seed"];

/// 파라미터 스펙 검증: 파라미터 스펙의 유효성 검증
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 검증만, 값 계산 없음
pub fn validate_spec(spec: &ParametricSpec) -> ParametricResult<()> {
  validate_spec_with_policy(spec, &CallPolicy::default_allowlist())
}

/// 파라미터 스펙 검증 (정책 지정): 파라미터 스펙의 유효성 검증 (정책 지정)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 검증만, 값 계산 없음
pub fn validate_spec_with_policy(
  spec: &ParametricSpec,
  policy: &CallPolicy,
) -> ParametricResult<()> {
  ensure_unique_params(spec)?;
  ensure_unique_signals(spec)?;
  ensure_unique_unit_scales(spec)?;
  ensure_unique_constraints(spec)?;
  ensure_target_exists(spec)?;
  ensure_exprs_known(spec)?;
  ensure_policy_valid(policy)?;
  ensure_calls_allowed(spec, policy)?;
  ensure_signal_context(spec)?;
  ensure_units_consistent(spec)?;
  ensure_constraints_consistent(spec)?;
  ensure_fixtures_valid(spec, policy)?;
  Ok(())
}

fn ensure_unique_params(spec: &ParametricSpec) -> ParametricResult<()> {
  let mut seen = BTreeSet::new();
  for p in &spec.params {
    if p.name.starts_with(super::SIGNAL_SYMBOL_PREFIX) {
      return Err(ParametricError::ReservedParamName {
        name: p.name.clone(),
      });
    }
    if !seen.insert(p.name.clone()) {
      return Err(ParametricError::DuplicateParam {
        name: p.name.clone(),
      });
    }
  }
  Ok(())
}

fn ensure_unique_signals(spec: &ParametricSpec) -> ParametricResult<()> {
  let mut seen = BTreeSet::new();
  for s in &spec.signals {
    if s.name.starts_with(super::SIGNAL_SYMBOL_PREFIX) {
      return Err(ParametricError::ReservedSignalName {
        name: s.name.clone(),
      });
    }
    if !seen.insert(s.name.clone()) {
      return Err(ParametricError::DuplicateSignal {
        name: s.name.clone(),
      });
    }
  }
  Ok(())
}

fn ensure_unique_unit_scales(spec: &ParametricSpec) -> ParametricResult<()> {
  let mut seen: BTreeMap<(String, String), f64> = BTreeMap::new();
  for scale in &spec.unit_scales {
    if !scale.factor.is_finite() || scale.factor <= 0.0 {
      return Err(ParametricError::UnitConversionInvalidFactor {
        factor: scale.factor,
      });
    }
    let key = (scale.from.label(), scale.to.label());
    if seen.contains_key(&key) {
      return Err(ParametricError::DuplicateUnitConversion {
        from: key.0,
        to: key.1,
      });
    }
    seen.insert(key, scale.factor);
  }
  Ok(())
}

fn ensure_unique_constraints(spec: &ParametricSpec) -> ParametricResult<()> {
  let mut seen = BTreeSet::new();
  for c in &spec.constraints {
    if !seen.insert(c.id.clone()) {
      return Err(ParametricError::DuplicateConstraint { id: c.id.clone() });
    }
  }
  Ok(())
}

fn ensure_target_exists(spec: &ParametricSpec) -> ParametricResult<()> {
  let exists = spec.params.iter().any(|p| p.name == spec.target.name);
  if !exists {
    return Err(ParametricError::TargetNotFound {
      name: spec.target.name.clone(),
    });
  }
  Ok(())
}

fn ensure_exprs_known(spec: &ParametricSpec) -> ParametricResult<()> {
  let param_names: BTreeSet<String> = spec.params.iter().map(|p| p.name.clone()).collect();
  let signal_names: BTreeSet<String> = spec.signals.iter().map(|s| s.name.clone()).collect();

  for c in &spec.constraints {
    let mut unknown_params = Vec::new();
    let mut unknown_signals = Vec::new();
    collect_unknowns(
      &c.expr,
      &param_names,
      &signal_names,
      &mut unknown_params,
      &mut unknown_signals,
    );

    if let Some(name) = unknown_params.pop() {
      return Err(ParametricError::UnknownParam { name });
    }
    if let Some(name) = unknown_signals.pop() {
      return Err(ParametricError::UnknownSignal { name });
    }
  }
  Ok(())
}

fn ensure_units_consistent(spec: &ParametricSpec) -> ParametricResult<()> {
  let param_units: BTreeMap<String, Option<Unit>> = spec
    .params
    .iter()
    .map(|p| (p.name.clone(), p.unit.clone()))
    .collect();
  let signal_units: BTreeMap<String, Option<Unit>> = spec
    .signals
    .iter()
    .map(|s| (s.name.clone(), s.unit.clone()))
    .collect();

  for c in &spec.constraints {
    match &c.expr {
      ConstraintExpr::Eq { left, right }
      | ConstraintExpr::Le { left, right }
      | ConstraintExpr::Ge { left, right } => {
        let l = infer_unit(left, &param_units, &signal_units, &spec.unit_scales)?;
        let r = infer_unit(right, &param_units, &signal_units, &spec.unit_scales)?;
        ensure_units_match(l, r, "constraint")?;
      }
      ConstraintExpr::Range { expr, min, max } => {
        let e = infer_unit(expr, &param_units, &signal_units, &spec.unit_scales)?;
        let min_u = infer_unit(min, &param_units, &signal_units, &spec.unit_scales)?;
        let max_u = infer_unit(max, &param_units, &signal_units, &spec.unit_scales)?;
        ensure_units_match(e.clone(), min_u, "range-min")?;
        ensure_units_match(e, max_u, "range-max")?;
      }
    }
  }

  Ok(())
}

fn infer_unit(
  expr: &ParamExpr,
  params: &BTreeMap<String, Option<Unit>>,
  signals: &BTreeMap<String, Option<Unit>>,
  unit_scales: &[UnitScale],
) -> ParametricResult<Option<Unit>> {
  Ok(match &expr.kind {
    ParamExprKind::Const(_) => None,
    ParamExprKind::Var(name) => params.get(name).cloned().flatten(),
    ParamExprKind::Signal(name) => signals.get(name).cloned().flatten(),
    ParamExprKind::Unary { op, arg } => {
      let u = infer_unit(arg, params, signals, unit_scales)?;
      match op {
        super::ir::ParamUnaryOp::Neg
        | super::ir::ParamUnaryOp::Floor
        | super::ir::ParamUnaryOp::Ceil
        | super::ir::ParamUnaryOp::Abs => u,
        super::ir::ParamUnaryOp::Sqrt => {
          if let Some(unit) = u {
            if unit.dims.values().all(|v| v % 2 == 0) {
              let mut dims = unit.dims.clone();
              for v in dims.values_mut() {
                *v /= 2;
              }
              dims.retain(|_, v| *v != 0);
              Some(Unit { dims })
            } else {
              return Err(ParametricError::UnitUnsupportedOp { op: "sqrt" });
            }
          } else {
            None
          }
        }
        super::ir::ParamUnaryOp::Sin
        | super::ir::ParamUnaryOp::Cos
        | super::ir::ParamUnaryOp::Tan
        | super::ir::ParamUnaryOp::Exp
        | super::ir::ParamUnaryOp::Ln => {
          if let Some(unit) = u {
            if !unit.is_dimensionless() {
              return Err(ParametricError::UnitUnsupportedOp {
                op: "transcendental",
              });
            }
          }
          None
        }
      }
    }
    ParamExprKind::Binary { op, lhs, rhs } => {
      let l = infer_unit(lhs, params, signals, unit_scales)?;
      let r = infer_unit(rhs, params, signals, unit_scales)?;
      match op {
        ParamBinaryOp::Add | ParamBinaryOp::Sub => match (l, r) {
          (Some(a), Some(b)) => {
            ensure_units_match(Some(a.clone()), Some(b.clone()), "add/sub")?;
            Some(a)
          }
          (None, None) => None,
          (Some(a), None) | (None, Some(a)) => {
            return Err(ParametricError::UnitMismatch {
              left: a.label(),
              right: "1".to_string(),
              op: "add/sub",
            })
          }
        },
        ParamBinaryOp::Mul => match (l, r) {
          (Some(a), Some(b)) => Some(a.mul(&b)),
          (Some(a), None) | (None, Some(a)) => Some(a),
          (None, None) => None,
        },
        ParamBinaryOp::Div => match (l, r) {
          (Some(a), Some(b)) => Some(a.div(&b)),
          (Some(a), None) => Some(a),
          (None, Some(b)) => Some(Unit::dimensionless().div(&b)),
          (None, None) => None,
        },
        ParamBinaryOp::Mod => match (l, r) {
          (Some(a), Some(b)) => {
            ensure_units_match(Some(a.clone()), Some(b.clone()), "mod")?;
            Some(a)
          }
          (None, None) => None,
          _ => {
            return Err(ParametricError::UnitUnsupportedOp { op: "mod" });
          }
        },
        ParamBinaryOp::Pow => {
          let exp_int = match &rhs.kind {
            ParamExprKind::Const(super::ir::ParamValue::Int(i)) => Some(*i),
            ParamExprKind::Const(super::ir::ParamValue::Float(f)) if f.fract() == 0.0 => {
              Some(*f as i64)
            }
            _ => None,
          };
          let Some(exp) = exp_int else {
            return Err(ParametricError::UnitUnsupportedOp { op: "pow" });
          };
          l.map(|unit| unit.pow_i32(exp as i32))
        }
      }
    }
    ParamExprKind::Convert {
      arg,
      from,
      to,
      factor,
    } => {
      if !factor.is_finite() || *factor <= 0.0 {
        return Err(ParametricError::UnitConversionInvalidFactor { factor: *factor });
      }
      let arg_unit = infer_unit(arg, params, signals, unit_scales)?;
      match arg_unit {
        Some(unit) => {
          if unit != *from {
            return Err(ParametricError::UnitConversionArgUnit {
              expected: from.label(),
              found: unit.label(),
            });
          }
        }
        None => {
          return Err(ParametricError::UnitConversionArgUnit {
            expected: from.label(),
            found: "1".to_string(),
          });
        }
      }
      ensure_unit_scale_allowed(from, to, *factor, unit_scales)?;
      Some(to.clone())
    }
    ParamExprKind::Call { func, args } => {
      if func == "min" || func == "max" {
        let mut acc: Option<Unit> = None;
        for arg in args {
          let unit =
            infer_unit(arg, params, signals, unit_scales)?.unwrap_or_else(Unit::dimensionless);
          match &acc {
            None => acc = Some(unit),
            Some(existing) => {
              ensure_units_match(Some(existing.clone()), Some(unit), "min/max")?;
            }
          }
        }
        match acc {
          Some(u) if u.is_dimensionless() => None,
          other => other,
        }
      } else {
        for arg in args {
          if let Some(unit) = infer_unit(arg, params, signals, unit_scales)? {
            if !unit.is_dimensionless() {
              return Err(ParametricError::UnitUnsupportedOp { op: "call" });
            }
          }
        }
        None
      }
    }
  })
}

fn ensure_units_match(a: Option<Unit>, b: Option<Unit>, op: &'static str) -> ParametricResult<()> {
  match (a, b) {
    (Some(l), Some(r)) if l != r => Err(ParametricError::UnitMismatch {
      left: l.label(),
      right: r.label(),
      op,
    }),
    (Some(l), None) => Err(ParametricError::UnitMismatch {
      left: l.label(),
      right: "1".to_string(),
      op,
    }),
    (None, Some(r)) => Err(ParametricError::UnitMismatch {
      left: "1".to_string(),
      right: r.label(),
      op,
    }),
    _ => Ok(()),
  }
}

fn ensure_unit_scale_allowed(
  from: &Unit,
  to: &Unit,
  factor: f64,
  unit_scales: &[UnitScale],
) -> ParametricResult<()> {
  if *from == *to && factor == 1.0 {
    return Ok(());
  }
  let expected = unit_scales
    .iter()
    .find(|s| s.from == *from && s.to == *to)
    .map(|s| s.factor);
  match expected {
    Some(exp) if exp == factor => Ok(()),
    Some(exp) => Err(ParametricError::UnitConversionFactorMismatch {
      from: from.label(),
      to: to.label(),
      expected: exp,
      found: factor,
    }),
    None => Err(ParametricError::UnitConversionMissing {
      from: from.label(),
      to: to.label(),
    }),
  }
}

fn ensure_constraints_consistent(spec: &ParametricSpec) -> ParametricResult<()> {
  let mut var_const: BTreeMap<String, f64> = BTreeMap::new();

  for c in &spec.constraints {
    match &c.expr {
      ConstraintExpr::Eq { left, right } => {
        check_constant_equation(left, right, &c.id)?;
        check_var_constant_eq(left, right, &c.id, &mut var_const)?;
        check_var_constant_eq(right, left, &c.id, &mut var_const)?;
      }
      ConstraintExpr::Le { left, right } => {
        check_constant_inequality(left, right, &c.id, true)?;
        check_var_bound(left, right, &c.id, &var_const, true)?;
      }
      ConstraintExpr::Ge { left, right } => {
        check_constant_inequality(left, right, &c.id, false)?;
        check_var_bound(left, right, &c.id, &var_const, false)?;
      }
      ConstraintExpr::Range { expr, min, max } => {
        check_constant_range(expr, min, max, &c.id)?;
        check_var_range(expr, min, max, &c.id, &var_const)?;
      }
    }
  }

  Ok(())
}

fn check_constant_equation(left: &ParamExpr, right: &ParamExpr, id: &str) -> ParametricResult<()> {
  let Some(l) = const_eval(left)? else {
    return Ok(());
  };
  let Some(r) = const_eval(right)? else {
    return Ok(());
  };
  if !approx_eq(l, r) {
    return Err(ParametricError::ConstraintInconsistent {
      detail: format!("constraint {id} is false: {l} != {r}"),
    });
  }
  Ok(())
}

fn check_constant_inequality(
  left: &ParamExpr,
  right: &ParamExpr,
  id: &str,
  is_le: bool,
) -> ParametricResult<()> {
  let Some(l) = const_eval(left)? else {
    return Ok(());
  };
  let Some(r) = const_eval(right)? else {
    return Ok(());
  };
  let ok = if is_le { l <= r } else { l >= r };
  if !ok {
    return Err(ParametricError::ConstraintInconsistent {
      detail: format!("constraint {id} is false: {l} vs {r}"),
    });
  }
  Ok(())
}

fn check_constant_range(
  expr: &ParamExpr,
  min: &ParamExpr,
  max: &ParamExpr,
  id: &str,
) -> ParametricResult<()> {
  let Some(v) = const_eval(expr)? else {
    return Ok(());
  };
  let Some(min_v) = const_eval(min)? else {
    return Ok(());
  };
  let Some(max_v) = const_eval(max)? else {
    return Ok(());
  };
  if v < min_v || v > max_v {
    return Err(ParametricError::ConstraintInconsistent {
      detail: format!("constraint {id} is false: {v} not in [{min_v},{max_v}]"),
    });
  }
  Ok(())
}

fn check_var_constant_eq(
  var_side: &ParamExpr,
  value_side: &ParamExpr,
  id: &str,
  map: &mut BTreeMap<String, f64>,
) -> ParametricResult<()> {
  let ParamExprKind::Var(name) = &var_side.kind else {
    return Ok(());
  };
  let Some(v) = const_eval(value_side)? else {
    return Ok(());
  };
  if let Some(prev) = map.get(name) {
    if !approx_eq(*prev, v) {
      return Err(ParametricError::ConstraintInconsistent {
        detail: format!("constraint {id} conflicts: {name}={prev} vs {v}"),
      });
    }
  } else {
    map.insert(name.clone(), v);
  }
  Ok(())
}

fn check_var_bound(
  var_side: &ParamExpr,
  bound_side: &ParamExpr,
  id: &str,
  map: &BTreeMap<String, f64>,
  is_le: bool,
) -> ParametricResult<()> {
  let ParamExprKind::Var(name) = &var_side.kind else {
    return Ok(());
  };
  let Some(bound) = const_eval(bound_side)? else {
    return Ok(());
  };
  let Some(value) = map.get(name) else {
    return Ok(());
  };
  let ok = if is_le {
    value <= &bound
  } else {
    value >= &bound
  };
  if !ok {
    return Err(ParametricError::ConstraintInconsistent {
      detail: format!("constraint {id} conflicts: {name}={value} vs {bound}"),
    });
  }
  Ok(())
}

fn check_var_range(
  expr: &ParamExpr,
  min: &ParamExpr,
  max: &ParamExpr,
  id: &str,
  map: &BTreeMap<String, f64>,
) -> ParametricResult<()> {
  let ParamExprKind::Var(name) = &expr.kind else {
    return Ok(());
  };
  let Some(v) = map.get(name) else {
    return Ok(());
  };
  let Some(min_v) = const_eval(min)? else {
    return Ok(());
  };
  let Some(max_v) = const_eval(max)? else {
    return Ok(());
  };
  if v < &min_v || v > &max_v {
    return Err(ParametricError::ConstraintInconsistent {
      detail: format!("constraint {id} conflicts: {name}={v} not in [{min_v},{max_v}]"),
    });
  }
  Ok(())
}

fn approx_eq(a: f64, b: f64) -> bool {
  (a - b).abs() <= 1e-9
}

fn ensure_signal_context(spec: &ParametricSpec) -> ParametricResult<()> {
  if matches!(spec.context, ContextMode::Pure) {
    let mut used = Vec::new();
    for c in &spec.constraints {
      collect_signals_from_constraint(&c.expr, &mut used);
    }
    if let Some(name) = used.pop() {
      return Err(ParametricError::SignalInPureContext { name });
    }
  }

  for s in &spec.signals {
    if !allowed_signal(s) {
      return Err(ParametricError::UnknownSignal {
        name: s.name.clone(),
      });
    }
  }
  Ok(())
}

fn allowed_signal(signal: &SignalRef) -> bool {
  ALLOWED_SIGNALS.iter().any(|n| *n == signal.name)
}

fn ensure_calls_allowed(spec: &ParametricSpec, policy: &CallPolicy) -> ParametricResult<()> {
  let mut calls = Vec::new();
  for c in &spec.constraints {
    collect_calls_from_constraint(&c.expr, &mut calls);
  }
  for (name, arity) in calls {
    let expected = policy
      .expected_arity(&name)
      .ok_or_else(|| ParametricError::UnsupportedCall { name: name.clone() })?;
    if expected != arity {
      return Err(ParametricError::UnsupportedCallArity {
        name,
        expected,
        found: arity,
      });
    }
  }
  Ok(())
}

fn collect_calls_from_constraint(expr: &ConstraintExpr, out: &mut Vec<(String, usize)>) {
  match expr {
    ConstraintExpr::Eq { left, right }
    | ConstraintExpr::Le { left, right }
    | ConstraintExpr::Ge { left, right } => {
      collect_calls_in_expr(left, out);
      collect_calls_in_expr(right, out);
    }
    ConstraintExpr::Range { expr, min, max } => {
      collect_calls_in_expr(expr, out);
      collect_calls_in_expr(min, out);
      collect_calls_in_expr(max, out);
    }
  }
}

fn collect_calls_in_expr(expr: &ParamExpr, out: &mut Vec<(String, usize)>) {
  match &expr.kind {
    ParamExprKind::Call { func, args } => {
      out.push((func.clone(), args.len()));
      for arg in args {
        collect_calls_in_expr(arg, out);
      }
    }
    ParamExprKind::Unary { arg, .. } => collect_calls_in_expr(arg, out),
    ParamExprKind::Binary { lhs, rhs, .. } => {
      collect_calls_in_expr(lhs, out);
      collect_calls_in_expr(rhs, out);
    }
    ParamExprKind::Convert { arg, .. } => collect_calls_in_expr(arg, out),
    ParamExprKind::Const(_) | ParamExprKind::Var(_) | ParamExprKind::Signal(_) => {}
  }
}

fn ensure_policy_valid(policy: &CallPolicy) -> ParametricResult<()> {
  let mut seen = BTreeSet::new();
  for call in &policy.calls {
    if call.arity == 0 {
      return Err(ParametricError::InvalidCallArity {
        name: call.name.clone(),
        arity: call.arity,
      });
    }
    if !seen.insert(call.name.clone()) {
      return Err(ParametricError::DuplicateCallPolicy {
        name: call.name.clone(),
      });
    }
  }
  Ok(())
}

fn collect_unknowns(
  expr: &ConstraintExpr,
  params: &BTreeSet<String>,
  signals: &BTreeSet<String>,
  unknown_params: &mut Vec<String>,
  unknown_signals: &mut Vec<String>,
) {
  match expr {
    ConstraintExpr::Eq { left, right }
    | ConstraintExpr::Le { left, right }
    | ConstraintExpr::Ge { left, right } => {
      collect_unknowns_in_expr(left, params, signals, unknown_params, unknown_signals);
      collect_unknowns_in_expr(right, params, signals, unknown_params, unknown_signals);
    }
    ConstraintExpr::Range { expr, min, max } => {
      collect_unknowns_in_expr(expr, params, signals, unknown_params, unknown_signals);
      collect_unknowns_in_expr(min, params, signals, unknown_params, unknown_signals);
      collect_unknowns_in_expr(max, params, signals, unknown_params, unknown_signals);
    }
  }
}

fn collect_unknowns_in_expr(
  expr: &ParamExpr,
  params: &BTreeSet<String>,
  signals: &BTreeSet<String>,
  unknown_params: &mut Vec<String>,
  unknown_signals: &mut Vec<String>,
) {
  match &expr.kind {
    ParamExprKind::Const(_) => {}
    ParamExprKind::Var(name) => {
      if !params.contains(name) {
        unknown_params.push(name.clone());
      }
    }
    ParamExprKind::Signal(name) => {
      if !signals.contains(name) {
        unknown_signals.push(name.clone());
      }
    }
    ParamExprKind::Unary { arg, .. } => {
      collect_unknowns_in_expr(arg, params, signals, unknown_params, unknown_signals);
    }
    ParamExprKind::Binary { lhs, rhs, .. } => {
      collect_unknowns_in_expr(lhs, params, signals, unknown_params, unknown_signals);
      collect_unknowns_in_expr(rhs, params, signals, unknown_params, unknown_signals);
    }
    ParamExprKind::Convert { arg, .. } => {
      collect_unknowns_in_expr(arg, params, signals, unknown_params, unknown_signals);
    }
    ParamExprKind::Call { args, .. } => {
      for arg in args {
        collect_unknowns_in_expr(arg, params, signals, unknown_params, unknown_signals);
      }
    }
  }
}

fn collect_signals_from_constraint(expr: &ConstraintExpr, out: &mut Vec<String>) {
  match expr {
    ConstraintExpr::Eq { left, right }
    | ConstraintExpr::Le { left, right }
    | ConstraintExpr::Ge { left, right } => {
      collect_signals(left, out);
      collect_signals(right, out);
    }
    ConstraintExpr::Range { expr, min, max } => {
      collect_signals(expr, out);
      collect_signals(min, out);
      collect_signals(max, out);
    }
  }
}

fn collect_signals(expr: &ParamExpr, out: &mut Vec<String>) {
  match &expr.kind {
    ParamExprKind::Signal(name) => out.push(name.clone()),
    ParamExprKind::Unary { arg, .. } => collect_signals(arg, out),
    ParamExprKind::Binary { lhs, rhs, .. } => {
      collect_signals(lhs, out);
      collect_signals(rhs, out);
    }
    ParamExprKind::Convert { arg, .. } => collect_signals(arg, out),
    ParamExprKind::Call { args, .. } => {
      for arg in args {
        collect_signals(arg, out);
      }
    }
    ParamExprKind::Const(_) | ParamExprKind::Var(_) => {}
  }
}

const FIXTURE_EPS: f64 = 1e-9;

fn ensure_fixtures_valid(spec: &ParametricSpec, policy: &CallPolicy) -> ParametricResult<()> {
  if spec.fixtures.is_empty() {
    return Ok(());
  }

  let param_names: BTreeSet<String> = spec.params.iter().map(|p| p.name.clone()).collect();
  let signal_names: BTreeSet<String> = spec.signals.iter().map(|s| s.name.clone()).collect();

  for fixture in &spec.fixtures {
    if fixture.id.trim().is_empty() {
      return Err(ParametricError::FixtureInvalidValue {
        fixture: "<unnamed>".to_string(),
        kind: "id",
        name: "id".to_string(),
        detail: "fixture id must not be empty".to_string(),
      });
    }
    let fixture_id = fixture.id.clone();

    let params = normalize_fixture_map(&fixture_id, "param", &fixture.params, &param_names)?;
    let signals = normalize_fixture_map(&fixture_id, "signal", &fixture.signals, &signal_names)?;

    for constraint in &spec.constraints {
      eval_constraint_with_fixture(constraint, &params, &signals, &fixture_id, policy)?;
    }
  }

  Ok(())
}

fn normalize_fixture_map(
  fixture_id: &str,
  kind: &'static str,
  values: &BTreeMap<String, ParamValue>,
  known: &BTreeSet<String>,
) -> ParametricResult<BTreeMap<String, f64>> {
  let mut out = BTreeMap::new();
  for (name, value) in values {
    if !known.contains(name) {
      return Err(ParametricError::FixtureUnknownBinding {
        fixture: fixture_id.to_string(),
        kind,
        name: name.clone(),
      });
    }
    let v = param_value_to_f64(value);
    if !v.is_finite() {
      return Err(ParametricError::FixtureInvalidValue {
        fixture: fixture_id.to_string(),
        kind,
        name: name.clone(),
        detail: "value is not finite".to_string(),
      });
    }
    out.insert(name.clone(), v);
  }
  Ok(out)
}

fn param_value_to_f64(value: &ParamValue) -> f64 {
  match value {
    ParamValue::Int(i) => *i as f64,
    ParamValue::Float(f) => *f,
  }
}

fn eval_constraint_with_fixture(
  constraint: &Constraint,
  params: &BTreeMap<String, f64>,
  signals: &BTreeMap<String, f64>,
  fixture_id: &str,
  policy: &CallPolicy,
) -> ParametricResult<()> {
  let cid = constraint.id.as_str();
  match &constraint.expr {
    ConstraintExpr::Eq { left, right } => {
      let l = eval_expr_with_fixture(left, params, signals, fixture_id, cid, policy)?;
      let r = eval_expr_with_fixture(right, params, signals, fixture_id, cid, policy)?;
      if !approx_eq(l, r) {
        return Err(ParametricError::FixtureConstraintFailed {
          fixture: fixture_id.to_string(),
          constraint: constraint.id.clone(),
          detail: format!("eq mismatch: left={} right={} eps={}", l, r, FIXTURE_EPS),
        });
      }
    }
    ConstraintExpr::Le { left, right } => {
      let l = eval_expr_with_fixture(left, params, signals, fixture_id, cid, policy)?;
      let r = eval_expr_with_fixture(right, params, signals, fixture_id, cid, policy)?;
      if l > r + FIXTURE_EPS {
        return Err(ParametricError::FixtureConstraintFailed {
          fixture: fixture_id.to_string(),
          constraint: constraint.id.clone(),
          detail: format!("le mismatch: left={} right={} eps={}", l, r, FIXTURE_EPS),
        });
      }
    }
    ConstraintExpr::Ge { left, right } => {
      let l = eval_expr_with_fixture(left, params, signals, fixture_id, cid, policy)?;
      let r = eval_expr_with_fixture(right, params, signals, fixture_id, cid, policy)?;
      if l + FIXTURE_EPS < r {
        return Err(ParametricError::FixtureConstraintFailed {
          fixture: fixture_id.to_string(),
          constraint: constraint.id.clone(),
          detail: format!("ge mismatch: left={} right={} eps={}", l, r, FIXTURE_EPS),
        });
      }
    }
    ConstraintExpr::Range { expr, min, max } => {
      let v = eval_expr_with_fixture(expr, params, signals, fixture_id, cid, policy)?;
      let min_v = eval_expr_with_fixture(min, params, signals, fixture_id, cid, policy)?;
      let max_v = eval_expr_with_fixture(max, params, signals, fixture_id, cid, policy)?;
      if v + FIXTURE_EPS < min_v || v > max_v + FIXTURE_EPS {
        return Err(ParametricError::FixtureConstraintFailed {
          fixture: fixture_id.to_string(),
          constraint: constraint.id.clone(),
          detail: format!(
            "range mismatch: value={} min={} max={} eps={}",
            v, min_v, max_v, FIXTURE_EPS
          ),
        });
      }
    }
  }
  Ok(())
}

fn eval_expr_with_fixture(
  expr: &ParamExpr,
  params: &BTreeMap<String, f64>,
  signals: &BTreeMap<String, f64>,
  fixture_id: &str,
  constraint_id: &str,
  policy: &CallPolicy,
) -> ParametricResult<f64> {
  let out = match &expr.kind {
    ParamExprKind::Const(v) => param_value_to_f64(v),
    ParamExprKind::Var(name) => {
      *params
        .get(name)
        .ok_or_else(|| ParametricError::FixtureMissingBinding {
          fixture: fixture_id.to_string(),
          kind: "param",
          name: name.clone(),
        })?
    }
    ParamExprKind::Signal(name) => {
      *signals
        .get(name)
        .ok_or_else(|| ParametricError::FixtureMissingBinding {
          fixture: fixture_id.to_string(),
          kind: "signal",
          name: name.clone(),
        })?
    }
    ParamExprKind::Unary { op, arg } => {
      let v = eval_expr_with_fixture(arg, params, signals, fixture_id, constraint_id, policy)?;
      match op {
        ParamUnaryOp::Neg => -v,
        ParamUnaryOp::Floor => v.floor(),
        ParamUnaryOp::Ceil => v.ceil(),
        ParamUnaryOp::Abs => v.abs(),
        ParamUnaryOp::Sqrt => {
          if v < 0.0 {
            return Err(ParametricError::FixtureConstraintFailed {
              fixture: fixture_id.to_string(),
              constraint: constraint_id.to_string(),
              detail: "sqrt domain error: value < 0".to_string(),
            });
          }
          v.sqrt()
        }
        ParamUnaryOp::Sin => v.sin(),
        ParamUnaryOp::Cos => v.cos(),
        ParamUnaryOp::Tan => v.tan(),
        ParamUnaryOp::Exp => v.exp(),
        ParamUnaryOp::Ln => {
          if v <= 0.0 {
            return Err(ParametricError::FixtureConstraintFailed {
              fixture: fixture_id.to_string(),
              constraint: constraint_id.to_string(),
              detail: "ln domain error: value <= 0".to_string(),
            });
          }
          v.ln()
        }
      }
    }
    ParamExprKind::Binary { op, lhs, rhs } => {
      let l = eval_expr_with_fixture(lhs, params, signals, fixture_id, constraint_id, policy)?;
      let r = eval_expr_with_fixture(rhs, params, signals, fixture_id, constraint_id, policy)?;
      match op {
        ParamBinaryOp::Add => l + r,
        ParamBinaryOp::Sub => l - r,
        ParamBinaryOp::Mul => l * r,
        ParamBinaryOp::Div => {
          if r == 0.0 {
            return Err(ParametricError::FixtureConstraintFailed {
              fixture: fixture_id.to_string(),
              constraint: constraint_id.to_string(),
              detail: "division by zero".to_string(),
            });
          }
          l / r
        }
        ParamBinaryOp::Mod => {
          if r == 0.0 {
            return Err(ParametricError::FixtureConstraintFailed {
              fixture: fixture_id.to_string(),
              constraint: constraint_id.to_string(),
              detail: "mod by zero".to_string(),
            });
          }
          l % r
        }
        ParamBinaryOp::Pow => l.powf(r),
      }
    }
    ParamExprKind::Convert { arg, factor, .. } => {
      if !factor.is_finite() || *factor <= 0.0 {
        return Err(ParametricError::FixtureConstraintFailed {
          fixture: fixture_id.to_string(),
          constraint: constraint_id.to_string(),
          detail: "conversion factor must be finite and > 0".to_string(),
        });
      }
      let v = eval_expr_with_fixture(arg, params, signals, fixture_id, constraint_id, policy)?;
      v * factor
    }
    ParamExprKind::Call { func, args } => {
      let expected =
        policy
          .expected_arity(func)
          .ok_or_else(|| ParametricError::FixtureConstraintFailed {
            fixture: fixture_id.to_string(),
            constraint: constraint_id.to_string(),
            detail: format!("call '{}' is not allowed", func),
          })?;
      if expected != args.len() {
        return Err(ParametricError::FixtureConstraintFailed {
          fixture: fixture_id.to_string(),
          constraint: constraint_id.to_string(),
          detail: format!(
            "call '{}' expects {} args, found {}",
            func,
            expected,
            args.len()
          ),
        });
      }
      let mut values = Vec::with_capacity(args.len());
      for arg in args {
        values.push(eval_expr_with_fixture(
          arg,
          params,
          signals,
          fixture_id,
          constraint_id,
          policy,
        )?);
      }
      match func.as_str() {
        "sin" => values[0].sin(),
        "cos" => values[0].cos(),
        "tan" => values[0].tan(),
        "sqrt" => {
          if values[0] < 0.0 {
            return Err(ParametricError::FixtureConstraintFailed {
              fixture: fixture_id.to_string(),
              constraint: constraint_id.to_string(),
              detail: "sqrt domain error: value < 0".to_string(),
            });
          }
          values[0].sqrt()
        }
        "abs" => values[0].abs(),
        "exp" => values[0].exp(),
        "ln" => {
          if values[0] <= 0.0 {
            return Err(ParametricError::FixtureConstraintFailed {
              fixture: fixture_id.to_string(),
              constraint: constraint_id.to_string(),
              detail: "ln domain error: value <= 0".to_string(),
            });
          }
          values[0].ln()
        }
        "floor" => values[0].floor(),
        "ceil" => values[0].ceil(),
        "pow" => values[0].powf(values[1]),
        "min" => values[0].min(values[1]),
        "max" => values[0].max(values[1]),
        _ => {
          return Err(ParametricError::FixtureConstraintFailed {
            fixture: fixture_id.to_string(),
            constraint: constraint_id.to_string(),
            detail: format!("call '{}' is not supported", func),
          })
        }
      }
    }
  };

  if !out.is_finite() {
    return Err(ParametricError::FixtureConstraintFailed {
      fixture: fixture_id.to_string(),
      constraint: constraint_id.to_string(),
      detail: "expression evaluated to non-finite value".to_string(),
    });
  }
  Ok(out)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::parametric::ir::{
    Constraint, ConstraintExpr, ContextMode, Fixture, ParamBinaryOp, ParamExpr, ParamRole,
    ParamValue, ParamVar, ParametricSpec, SignalRef, TargetVar, Unit, UnitScale,
  };
  use crate::parametric::policy::{CallPolicy, CallSpec};
  use std::collections::BTreeMap;

  #[test]
  fn rejects_signal_in_pure_context() {
    let spec = ParametricSpec {
      context: ContextMode::Pure,
      params: vec![ParamVar::new("x", ParamRole::Input)],
      signals: vec![SignalRef::new("time")],
      unit_scales: vec![],
      fixtures: vec![],
      constraints: vec![Constraint {
        id: "c1".to_string(),
        expr: ConstraintExpr::Eq {
          left: ParamExpr::var("x"),
          right: ParamExpr::signal("time"),
        },
        provenance: None,
      }],
      target: TargetVar::new("x"),
    };

    let err = validate_spec(&spec).unwrap_err();
    assert!(matches!(err, ParametricError::SignalInPureContext { .. }));
  }

  #[test]
  fn rejects_reserved_param_name() {
    let name = format!("{}x", super::super::SIGNAL_SYMBOL_PREFIX);
    let spec = ParametricSpec {
      context: ContextMode::Pure,
      params: vec![ParamVar::new(&name, ParamRole::Input)],
      signals: vec![],
      unit_scales: vec![],
      fixtures: vec![],
      constraints: vec![],
      target: TargetVar::new(&name),
    };

    let err = validate_spec(&spec).unwrap_err();
    assert!(matches!(err, ParametricError::ReservedParamName { .. }));
  }

  #[test]
  fn rejects_reserved_signal_name() {
    let name = format!("{}time", super::super::SIGNAL_SYMBOL_PREFIX);
    let spec = ParametricSpec {
      context: ContextMode::Pure,
      params: vec![ParamVar::new("x", ParamRole::Input)],
      signals: vec![SignalRef::new(&name)],
      unit_scales: vec![],
      fixtures: vec![],
      constraints: vec![],
      target: TargetVar::new("x"),
    };

    let err = validate_spec(&spec).unwrap_err();
    assert!(matches!(err, ParametricError::ReservedSignalName { .. }));
  }

  #[test]
  fn detects_conflicting_eq_constraints() {
    let spec = ParametricSpec {
      context: ContextMode::Pure,
      params: vec![ParamVar::new("x", ParamRole::Input)],
      signals: vec![],
      unit_scales: vec![],
      fixtures: vec![],
      constraints: vec![
        Constraint {
          id: "c1".to_string(),
          expr: ConstraintExpr::Eq {
            left: ParamExpr::var("x"),
            right: ParamExpr::int(1),
          },
          provenance: None,
        },
        Constraint {
          id: "c2".to_string(),
          expr: ConstraintExpr::Eq {
            left: ParamExpr::var("x"),
            right: ParamExpr::int(2),
          },
          provenance: None,
        },
      ],
      target: TargetVar::new("x"),
    };

    let err = validate_spec(&spec).unwrap_err();
    assert!(matches!(
      err,
      ParametricError::ConstraintInconsistent { .. }
    ));
  }

  #[test]
  fn detects_unit_mismatch() {
    let mut a = ParamVar::new("a", ParamRole::Input);
    a.unit = Some(Unit::new("m"));
    let mut b = ParamVar::new("b", ParamRole::Input);
    b.unit = Some(Unit::new("s"));
    let spec = ParametricSpec {
      context: ContextMode::Pure,
      params: vec![a, b],
      signals: vec![],
      unit_scales: vec![],
      fixtures: vec![],
      constraints: vec![Constraint {
        id: "c1".to_string(),
        expr: ConstraintExpr::Eq {
          left: ParamExpr::var("a"),
          right: ParamExpr::var("b"),
        },
        provenance: None,
      }],
      target: TargetVar::new("a"),
    };

    let err = validate_spec(&spec).unwrap_err();
    assert!(matches!(err, ParametricError::UnitMismatch { .. }));
  }

  #[test]
  fn allows_unit_multiplication() {
    let mut a = ParamVar::new("a", ParamRole::Input);
    a.unit = Some(Unit::new("m"));
    let mut b = ParamVar::new("b", ParamRole::Input);
    b.unit = Some(Unit::new("s"));
    let mut c = ParamVar::new("c", ParamRole::Output);
    c.unit = Some(a.unit.clone().unwrap().mul(&b.unit.clone().unwrap()));

    let spec = ParametricSpec {
      context: ContextMode::Pure,
      params: vec![a, b, c],
      signals: vec![],
      unit_scales: vec![],
      fixtures: vec![],
      constraints: vec![Constraint {
        id: "c1".to_string(),
        expr: ConstraintExpr::Eq {
          left: ParamExpr::var("c"),
          right: ParamExpr::binary(ParamBinaryOp::Mul, ParamExpr::var("a"), ParamExpr::var("b")),
        },
        provenance: None,
      }],
      target: TargetVar::new("c"),
    };

    validate_spec(&spec).unwrap();
  }

  #[test]
  fn validates_unit_conversion_with_scale_table() {
    let mut a = ParamVar::new("a", ParamRole::Input);
    a.unit = Some(Unit::new("cm"));
    let mut b = ParamVar::new("b", ParamRole::Output);
    b.unit = Some(Unit::new("m"));

    let spec = ParametricSpec {
      context: ContextMode::Pure,
      params: vec![a, b],
      signals: vec![],
      unit_scales: vec![UnitScale {
        from: Unit::new("cm"),
        to: Unit::new("m"),
        factor: 0.01,
      }],
      fixtures: vec![],
      constraints: vec![Constraint {
        id: "c1".to_string(),
        expr: ConstraintExpr::Eq {
          left: ParamExpr::convert(ParamExpr::var("a"), Unit::new("cm"), Unit::new("m"), 0.01),
          right: ParamExpr::var("b"),
        },
        provenance: None,
      }],
      target: TargetVar::new("b"),
    };

    validate_spec(&spec).unwrap();
  }

  #[test]
  fn rejects_unsupported_call() {
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
          right: ParamExpr::call("foo", vec![ParamExpr::int(1)]),
        },
        provenance: None,
      }],
      target: TargetVar::new("x"),
    };

    let err = validate_spec(&spec).unwrap_err();
    assert!(matches!(err, ParametricError::UnsupportedCall { .. }));
  }

  #[test]
  fn rejects_call_arity_mismatch() {
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
          right: ParamExpr::call("pow", vec![ParamExpr::int(2)]),
        },
        provenance: None,
      }],
      target: TargetVar::new("x"),
    };

    let err = validate_spec(&spec).unwrap_err();
    assert!(matches!(err, ParametricError::UnsupportedCallArity { .. }));
  }

  #[test]
  fn rejects_invalid_policy() {
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
        provenance: None,
      }],
      target: TargetVar::new("x"),
    };

    let policy = CallPolicy {
      calls: vec![CallSpec {
        name: "sin".to_string(),
        arity: 0,
      }],
    };

    let err = validate_spec_with_policy(&spec, &policy).unwrap_err();
    assert!(matches!(err, ParametricError::InvalidCallArity { .. }));
  }

  #[test]
  fn fixture_validation_passes() {
    let mut params = BTreeMap::new();
    params.insert("x".to_string(), ParamValue::Int(1));
    let fixture = Fixture {
      id: "f1".to_string(),
      params,
      signals: BTreeMap::new(),
    };

    let spec = ParametricSpec {
      context: ContextMode::Pure,
      params: vec![ParamVar::new("x", ParamRole::Output)],
      signals: vec![],
      unit_scales: vec![],
      fixtures: vec![fixture],
      constraints: vec![Constraint {
        id: "c1".to_string(),
        expr: ConstraintExpr::Eq {
          left: ParamExpr::var("x"),
          right: ParamExpr::int(1),
        },
        provenance: None,
      }],
      target: TargetVar::new("x"),
    };

    validate_spec(&spec).unwrap();
  }

  #[test]
  fn fixture_validation_fails_on_mismatch() {
    let mut params = BTreeMap::new();
    params.insert("x".to_string(), ParamValue::Int(2));
    let fixture = Fixture {
      id: "f1".to_string(),
      params,
      signals: BTreeMap::new(),
    };

    let spec = ParametricSpec {
      context: ContextMode::Pure,
      params: vec![ParamVar::new("x", ParamRole::Output)],
      signals: vec![],
      unit_scales: vec![],
      fixtures: vec![fixture],
      constraints: vec![Constraint {
        id: "c1".to_string(),
        expr: ConstraintExpr::Eq {
          left: ParamExpr::var("x"),
          right: ParamExpr::int(1),
        },
        provenance: None,
      }],
      target: TargetVar::new("x"),
    };

    let err = validate_spec(&spec).unwrap_err();
    assert!(matches!(
      err,
      ParametricError::FixtureConstraintFailed { .. }
    ));
  }
}
