//! Parametric 표현식 출력: ParamExpr를 UnifiedExpr, FxSurfaceExpr, PNIX 코드로 변환

use super::error::{ParametricError, ParametricResult};
use super::ir::{
  ConstraintExpr, ParamBinaryOp, ParamExpr, ParamExprKind, ParamUnaryOp, ParamValue, ProvenanceTag,
};
use super::synth::{SynthesisForm, SynthesisResult};
use crate::fx::surface::FxSurfaceExpr;
use crate::lang::pnix::UnifiedExpr;

pub fn emit_unified(expr: &ParamExpr) -> ParametricResult<UnifiedExpr> {
  Ok(match &expr.kind {
    ParamExprKind::Const(v) => match v {
      ParamValue::Int(i) => UnifiedExpr::Int(*i),
      ParamValue::Float(f) => UnifiedExpr::Float(*f),
    },
    ParamExprKind::Var(name) => UnifiedExpr::Var(name.clone()),
    ParamExprKind::Signal(name) => signal_to_unified(name),
    ParamExprKind::Unary { op, arg } => {
      let inner = emit_unified(arg)?;
      match op {
        ParamUnaryOp::Neg => UnifiedExpr::Neg(Box::new(inner)),
        ParamUnaryOp::Floor => UnifiedExpr::Floor(Box::new(inner)),
        ParamUnaryOp::Ceil => UnifiedExpr::Ceil(Box::new(inner)),
        ParamUnaryOp::Abs => UnifiedExpr::Abs(Box::new(inner)),
        ParamUnaryOp::Sqrt => UnifiedExpr::Sqrt(Box::new(inner)),
        ParamUnaryOp::Sin => UnifiedExpr::Sin(Box::new(inner)),
        ParamUnaryOp::Cos => UnifiedExpr::Cos(Box::new(inner)),
        ParamUnaryOp::Tan => UnifiedExpr::Tan(Box::new(inner)),
        ParamUnaryOp::Exp => UnifiedExpr::Exp(Box::new(inner)),
        ParamUnaryOp::Ln => UnifiedExpr::Ln(Box::new(inner)),
      }
    }
    ParamExprKind::Binary { op, lhs, rhs } => {
      let l = emit_unified(lhs)?;
      let r = emit_unified(rhs)?;
      match op {
        ParamBinaryOp::Add => UnifiedExpr::Add(Box::new(l), Box::new(r)),
        ParamBinaryOp::Sub => UnifiedExpr::Sub(Box::new(l), Box::new(r)),
        ParamBinaryOp::Mul => UnifiedExpr::Mul(Box::new(l), Box::new(r)),
        ParamBinaryOp::Div => UnifiedExpr::Div(Box::new(l), Box::new(r)),
        ParamBinaryOp::Mod => UnifiedExpr::Mod(Box::new(l), Box::new(r)),
        ParamBinaryOp::Pow => UnifiedExpr::Pow(Box::new(l), Box::new(r)),
      }
    }
    ParamExprKind::Convert { arg, factor, .. } => {
      ensure_conversion_factor(*factor)?;
      let inner = emit_unified(arg)?;
      let scale = UnifiedExpr::Float(*factor);
      UnifiedExpr::Mul(Box::new(scale), Box::new(inner))
    }
    ParamExprKind::Call { func, args } => UnifiedExpr::Apply {
      func: func.clone(),
      args: args
        .iter()
        .map(emit_unified)
        .collect::<ParametricResult<Vec<_>>>()?,
    },
  })
}

pub fn emit_fx_surface(expr: &ParamExpr) -> ParametricResult<FxSurfaceExpr> {
  Ok(match &expr.kind {
    ParamExprKind::Const(v) => match v {
      ParamValue::Int(i) => FxSurfaceExpr::ConstInt(*i),
      ParamValue::Float(f) => FxSurfaceExpr::ConstFloat(*f),
    },
    ParamExprKind::Var(name) => FxSurfaceExpr::Ident(name.clone()),
    ParamExprKind::Signal(name) => FxSurfaceExpr::Ident(signal_to_surface_ident(name)),
    ParamExprKind::Unary { op, arg } => {
      let inner = emit_fx_surface(arg)?;
      match op {
        ParamUnaryOp::Neg => FxSurfaceExpr::PrefixOp {
          op: "-".to_string(),
          arg: Box::new(inner),
        },
        ParamUnaryOp::Floor => FxSurfaceExpr::Call {
          func: "floor".to_string(),
          args: vec![inner],
        },
        ParamUnaryOp::Ceil => FxSurfaceExpr::Call {
          func: "ceil".to_string(),
          args: vec![inner],
        },
        ParamUnaryOp::Abs => FxSurfaceExpr::Call {
          func: "abs".to_string(),
          args: vec![inner],
        },
        ParamUnaryOp::Sqrt => FxSurfaceExpr::Call {
          func: "sqrt".to_string(),
          args: vec![inner],
        },
        ParamUnaryOp::Sin => FxSurfaceExpr::Call {
          func: "sin".to_string(),
          args: vec![inner],
        },
        ParamUnaryOp::Cos => FxSurfaceExpr::Call {
          func: "cos".to_string(),
          args: vec![inner],
        },
        ParamUnaryOp::Tan => FxSurfaceExpr::Call {
          func: "tan".to_string(),
          args: vec![inner],
        },
        ParamUnaryOp::Exp => FxSurfaceExpr::Call {
          func: "exp".to_string(),
          args: vec![inner],
        },
        ParamUnaryOp::Ln => FxSurfaceExpr::Call {
          func: "ln".to_string(),
          args: vec![inner],
        },
      }
    }
    ParamExprKind::Binary { op, lhs, rhs } => {
      let l = emit_fx_surface(lhs)?;
      let r = emit_fx_surface(rhs)?;
      let op_str = match op {
        ParamBinaryOp::Add => "+",
        ParamBinaryOp::Sub => "-",
        ParamBinaryOp::Mul => "*",
        ParamBinaryOp::Div => "/",
        ParamBinaryOp::Mod => "%",
        ParamBinaryOp::Pow => "^",
      };
      FxSurfaceExpr::InfixOp {
        op: op_str.to_string(),
        left: Box::new(l),
        right: Box::new(r),
      }
    }
    ParamExprKind::Convert { arg, factor, .. } => {
      ensure_conversion_factor(*factor)?;
      let l = FxSurfaceExpr::ConstFloat(*factor);
      let r = emit_fx_surface(arg)?;
      FxSurfaceExpr::InfixOp {
        op: "*".to_string(),
        left: Box::new(l),
        right: Box::new(r),
      }
    }
    ParamExprKind::Call { func, args } => FxSurfaceExpr::Call {
      func: func.clone(),
      args: args
        .iter()
        .map(emit_fx_surface)
        .collect::<ParametricResult<Vec<_>>>()?,
    },
  })
}

pub fn emit_pnix_string(expr: &ParamExpr) -> ParametricResult<String> {
  Ok(match &expr.kind {
    ParamExprKind::Const(v) => match v {
      ParamValue::Int(i) => i.to_string(),
      ParamValue::Float(f) => format!("{}", f),
    },
    ParamExprKind::Var(name) => name.clone(),
    ParamExprKind::Signal(name) => signal_to_pnix_ident(name),
    ParamExprKind::Unary { op, arg } => match op {
      ParamUnaryOp::Neg => format!("(0 - {})", emit_pnix_string(arg)?),
      ParamUnaryOp::Floor => format!("floor {}", wrap_pnix_arg(arg)?),
      ParamUnaryOp::Ceil => format!("ceil {}", wrap_pnix_arg(arg)?),
      ParamUnaryOp::Abs => format!("abs {}", wrap_pnix_arg(arg)?),
      ParamUnaryOp::Sqrt => format!("sqrt {}", wrap_pnix_arg(arg)?),
      ParamUnaryOp::Sin => format!("sin {}", wrap_pnix_arg(arg)?),
      ParamUnaryOp::Cos => format!("cos {}", wrap_pnix_arg(arg)?),
      ParamUnaryOp::Tan => format!("tan {}", wrap_pnix_arg(arg)?),
      ParamUnaryOp::Exp => format!("exp {}", wrap_pnix_arg(arg)?),
      ParamUnaryOp::Ln => format!("ln {}", wrap_pnix_arg(arg)?),
    },
    ParamExprKind::Binary { op, lhs, rhs } => {
      let op_str = match op {
        ParamBinaryOp::Add => "+",
        ParamBinaryOp::Sub => "-",
        ParamBinaryOp::Mul => "*",
        ParamBinaryOp::Div => "/",
        ParamBinaryOp::Mod => "%",
        ParamBinaryOp::Pow => "^",
      };
      format!(
        "({} {} {})",
        emit_pnix_string(lhs)?,
        op_str,
        emit_pnix_string(rhs)?
      )
    }
    ParamExprKind::Convert { arg, factor, .. } => {
      ensure_conversion_factor(*factor)?;
      format!("({} * {})", factor, emit_pnix_string(arg)?)
    }
    ParamExprKind::Call { func, args } => {
      if args.is_empty() {
        return Err(ParametricError::EmitUnsupported {
          detail: "call with zero args is not supported in pnix output".to_string(),
        });
      }
      let mut out = func.clone();
      for arg in args {
        out.push(' ');
        out.push_str(&wrap_pnix_arg(arg)?);
      }
      out
    }
  })
}

pub fn emit_constraint_unified(expr: &ConstraintExpr) -> ParametricResult<UnifiedExpr> {
  Ok(match expr {
    ConstraintExpr::Eq { left, right } => UnifiedExpr::Eq(
      Box::new(emit_unified(left)?),
      Box::new(emit_unified(right)?),
    ),
    ConstraintExpr::Le { left, right } => UnifiedExpr::Le(
      Box::new(emit_unified(left)?),
      Box::new(emit_unified(right)?),
    ),
    ConstraintExpr::Ge { left, right } => UnifiedExpr::Ge(
      Box::new(emit_unified(left)?),
      Box::new(emit_unified(right)?),
    ),
    ConstraintExpr::Range { expr, min, max } => {
      let ge = UnifiedExpr::Ge(Box::new(emit_unified(expr)?), Box::new(emit_unified(min)?));
      let le = UnifiedExpr::Le(Box::new(emit_unified(expr)?), Box::new(emit_unified(max)?));
      UnifiedExpr::And(Box::new(ge), Box::new(le))
    }
  })
}

pub fn emit_constraint_surface(expr: &ConstraintExpr) -> ParametricResult<FxSurfaceExpr> {
  Ok(match expr {
    ConstraintExpr::Eq { left, right } => FxSurfaceExpr::InfixOp {
      op: "==".to_string(),
      left: Box::new(emit_fx_surface(left)?),
      right: Box::new(emit_fx_surface(right)?),
    },
    ConstraintExpr::Le { left, right } => FxSurfaceExpr::InfixOp {
      op: "<=".to_string(),
      left: Box::new(emit_fx_surface(left)?),
      right: Box::new(emit_fx_surface(right)?),
    },
    ConstraintExpr::Ge { left, right } => FxSurfaceExpr::InfixOp {
      op: ">=".to_string(),
      left: Box::new(emit_fx_surface(left)?),
      right: Box::new(emit_fx_surface(right)?),
    },
    ConstraintExpr::Range { expr, min, max } => {
      let ge = FxSurfaceExpr::InfixOp {
        op: ">=".to_string(),
        left: Box::new(emit_fx_surface(expr)?),
        right: Box::new(emit_fx_surface(min)?),
      };
      let le = FxSurfaceExpr::InfixOp {
        op: "<=".to_string(),
        left: Box::new(emit_fx_surface(expr)?),
        right: Box::new(emit_fx_surface(max)?),
      };
      FxSurfaceExpr::InfixOp {
        op: "&&".to_string(),
        left: Box::new(ge),
        right: Box::new(le),
      }
    }
  })
}

pub fn emit_constraint_pnix(expr: &ConstraintExpr) -> ParametricResult<String> {
  match expr {
    ConstraintExpr::Eq { left, right } => Ok(format!(
      "({} == {})",
      emit_pnix_string(left)?,
      emit_pnix_string(right)?
    )),
    ConstraintExpr::Le { left, right } => Ok(format!(
      "({} <= {})",
      emit_pnix_string(left)?,
      emit_pnix_string(right)?
    )),
    ConstraintExpr::Ge { left, right } => Ok(format!(
      "({} >= {})",
      emit_pnix_string(left)?,
      emit_pnix_string(right)?
    )),
    ConstraintExpr::Range { expr, min, max } => Ok(format!(
      "(({} >= {}) && ({} <= {}))",
      emit_pnix_string(expr)?,
      emit_pnix_string(min)?,
      emit_pnix_string(expr)?,
      emit_pnix_string(max)?
    )),
  }
}

pub fn emit_synthesis_form_unified(
  target: &str,
  form: &SynthesisForm,
) -> ParametricResult<UnifiedExpr> {
  let target_expr = ParamExpr::var(target);
  let constraint = synthesis_form_to_constraint(target_expr, form)?;
  emit_constraint_unified(&constraint)
}

pub fn emit_synthesis_form_surface(
  target: &str,
  form: &SynthesisForm,
) -> ParametricResult<FxSurfaceExpr> {
  let target_expr = ParamExpr::var(target);
  let constraint = synthesis_form_to_constraint(target_expr, form)?;
  emit_constraint_surface(&constraint)
}

pub fn emit_synthesis_form_pnix(target: &str, form: &SynthesisForm) -> ParametricResult<String> {
  let target_expr = ParamExpr::var(target);
  let constraint = synthesis_form_to_constraint(target_expr, form)?;
  emit_constraint_pnix(&constraint)
}

pub fn emit_synthesis_result_unified(res: &SynthesisResult) -> ParametricResult<UnifiedExpr> {
  let target_expr = target_expr_with_provenance(&res.target, &res.target_provenance);
  let constraint = synthesis_form_to_constraint(target_expr, &res.form)?;
  emit_constraint_unified(&constraint)
}

pub fn emit_synthesis_result_surface(res: &SynthesisResult) -> ParametricResult<FxSurfaceExpr> {
  let target_expr = target_expr_with_provenance(&res.target, &res.target_provenance);
  let constraint = synthesis_form_to_constraint(target_expr, &res.form)?;
  emit_constraint_surface(&constraint)
}

pub fn emit_synthesis_result_pnix(res: &SynthesisResult) -> ParametricResult<String> {
  let target_expr = target_expr_with_provenance(&res.target, &res.target_provenance);
  let constraint = synthesis_form_to_constraint(target_expr, &res.form)?;
  emit_constraint_pnix(&constraint)
}

fn synthesis_form_to_constraint(
  target: ParamExpr,
  form: &SynthesisForm,
) -> ParametricResult<ConstraintExpr> {
  Ok(match form {
    SynthesisForm::Eq(expr) => ConstraintExpr::Eq {
      left: target,
      right: expr.clone(),
    },
    SynthesisForm::Le(expr) => ConstraintExpr::Le {
      left: target,
      right: expr.clone(),
    },
    SynthesisForm::Ge(expr) => ConstraintExpr::Ge {
      left: target,
      right: expr.clone(),
    },
    SynthesisForm::Range { min, max } => ConstraintExpr::Range {
      expr: target,
      min: min.clone(),
      max: max.clone(),
    },
  })
}

fn target_expr_with_provenance(target: &str, provenance: &Option<ProvenanceTag>) -> ParamExpr {
  ParamExpr {
    kind: ParamExprKind::Var(target.to_string()),
    provenance: provenance.clone(),
  }
}

fn signal_to_unified(name: &str) -> UnifiedExpr {
  match name {
    "time" | "t" | "system_time" => UnifiedExpr::ParamTime,
    "dt" | "delta_time" => UnifiedExpr::ParamDeltaTime,
    _ => UnifiedExpr::ParamSignal(name.to_string()),
  }
}

fn ensure_conversion_factor(factor: f64) -> ParametricResult<()> {
  if !factor.is_finite() || factor <= 0.0 {
    return Err(ParametricError::UnitConversionInvalidFactor { factor });
  }
  Ok(())
}

fn signal_to_surface_ident(name: &str) -> String {
  match name {
    "time" | "t" | "system_time" => "param.system_time".to_string(),
    "dt" | "delta_time" => "param.delta_time".to_string(),
    _ => format!("signal.{}", name),
  }
}

fn signal_to_pnix_ident(name: &str) -> String {
  match name {
    "time" | "t" | "system_time" => "param.system_time".to_string(),
    "dt" | "delta_time" => "param.delta_time".to_string(),
    _ => format!("signal.{}", name),
  }
}

fn wrap_pnix_arg(expr: &ParamExpr) -> ParametricResult<String> {
  let s = emit_pnix_string(expr)?;
  match expr.kind {
    ParamExprKind::Const(_) | ParamExprKind::Var(_) | ParamExprKind::Signal(_) => Ok(s),
    _ => Ok(format!("({})", s)),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::fx::lowering::lower_surface_to_core;
  use crate::lang::pnix::lower::lower_to_fx_core_with_mode;
  use crate::lang::pnix::parser::parse_expr;
  use crate::lang::pnix::unified::ExecutionMode;
  use crate::parametric::synth::SynthesisForm;
  use serde_json::to_string;

  #[test]
  fn emit_unified_and_surface_match_core() {
    let expr = ParamExpr::binary(
      ParamBinaryOp::Add,
      ParamExpr::signal("time"),
      ParamExpr::int(1),
    );

    let unified = emit_unified(&expr).unwrap();
    let surface = emit_fx_surface(&expr).unwrap();

    // Y08a-9: resolve_signals 파이프라인 적용
    // ParamExpr에서 생성된 UnifiedExpr는 ParamSignal을 포함할 수 있으므로 Realtime 모드 사용
    let core_from_unified = lower_to_fx_core_with_mode(
      &unified,
      ExecutionMode::Realtime,
      &["time"], // allowlist: time 시그널 허용
    )
    .unwrap();
    let core_from_surface = lower_surface_to_core(&surface).unwrap();

    let left = to_string(&core_from_unified).unwrap();
    let right = to_string(&core_from_surface).unwrap();
    assert_eq!(left, right);
  }

  #[test]
  fn pnix_output_parses() {
    let expr = ParamExpr::binary(
      ParamBinaryOp::Mul,
      ParamExpr::call("sin", vec![ParamExpr::signal("time")]),
      ParamExpr::int(2),
    );

    let src = emit_pnix_string(&expr).unwrap();
    let _ = parse_expr(&src).unwrap();
  }

  #[test]
  fn synthesis_eq_pnix_parses() {
    let form = SynthesisForm::Eq(ParamExpr::int(3));
    let src = emit_synthesis_form_pnix("x", &form).unwrap();
    let _ = parse_expr(&src).unwrap();
  }

  #[test]
  fn synthesis_range_pnix_parses() {
    let form = SynthesisForm::Range {
      min: ParamExpr::int(0),
      max: ParamExpr::int(1),
    };
    let src = emit_synthesis_form_pnix("x", &form).unwrap();
    let _ = parse_expr(&src).unwrap();
  }
}
