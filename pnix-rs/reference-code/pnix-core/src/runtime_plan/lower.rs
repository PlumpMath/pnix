//! UnifiedExpr → RuntimePlan 변환: UnifiedExpr를 런타임 계획으로 lowering

use crate::lang::pnix::UnifiedExpr;

use super::error::{RuntimePlanError, RuntimePlanResult};
use super::ir::{RpBinaryOp, RpNode, RpUnaryOp, RpValue, RuntimePlan};

/// UnifiedExpr를 런타임 계획으로 변환
///
/// UnifiedExpr를 런타임에서 직접 실행 가능한 RuntimePlan으로 lowering합니다.
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn unified_to_runtime_plan(expr: &UnifiedExpr) -> RuntimePlanResult<RuntimePlan> {
  Ok(RuntimePlan {
    root: lower_node(expr)?,
  })
}

fn lower_node(expr: &UnifiedExpr) -> RuntimePlanResult<RpNode> {
  match expr {
    UnifiedExpr::Int(v) => Ok(RpNode::Const(RpValue::Int(*v))),
    UnifiedExpr::Float(v) => Ok(RpNode::Const(RpValue::Float(*v))),
    UnifiedExpr::Bool(v) => Ok(RpNode::Const(RpValue::Bool(*v))),
    UnifiedExpr::String(v) => Ok(RpNode::Const(RpValue::String(v.clone()))),
    UnifiedExpr::Var(name) => Ok(RpNode::Var(name.clone())),

    UnifiedExpr::ParamTime => Ok(RpNode::GetSignal {
      name: "time".to_string(),
    }),
    UnifiedExpr::ParamDeltaTime => Ok(RpNode::GetSignal {
      name: "dt".to_string(),
    }),
    UnifiedExpr::ParamSignal(name) => Err(RuntimePlanError::SignalResolutionRequired {
      detail: format!(
        "ParamSignal '{}' must be resolved before runtime plan",
        name
      ),
    }),
    UnifiedExpr::SignalVar(name) => Ok(RpNode::GetSignal { name: name.clone() }),

    UnifiedExpr::Add(lhs, rhs) => Ok(RpNode::Binary {
      op: RpBinaryOp::Add,
      lhs: Box::new(lower_node(lhs)?),
      rhs: Box::new(lower_node(rhs)?),
    }),
    // Y-CLAUDE-6: ++ 연산자로 명시적 문자열 연결
    UnifiedExpr::Concat(lhs, rhs) => Ok(RpNode::Binary {
      op: RpBinaryOp::Concat,
      lhs: Box::new(lower_node(lhs)?),
      rhs: Box::new(lower_node(rhs)?),
    }),
    // Y10c: // 병합 연산자: a // b → AttrSet 병합
    UnifiedExpr::Merge(lhs, rhs) => Ok(RpNode::Call {
      func: "builtins.merge".to_string(),
      args: vec![lower_node(lhs)?, lower_node(rhs)?],
    }),
    UnifiedExpr::Sub(lhs, rhs) => Ok(RpNode::Binary {
      op: RpBinaryOp::Sub,
      lhs: Box::new(lower_node(lhs)?),
      rhs: Box::new(lower_node(rhs)?),
    }),
    UnifiedExpr::Mul(lhs, rhs) => Ok(RpNode::Binary {
      op: RpBinaryOp::Mul,
      lhs: Box::new(lower_node(lhs)?),
      rhs: Box::new(lower_node(rhs)?),
    }),
    UnifiedExpr::Div(lhs, rhs) => Ok(RpNode::Binary {
      op: RpBinaryOp::Div,
      lhs: Box::new(lower_node(lhs)?),
      rhs: Box::new(lower_node(rhs)?),
    }),
    UnifiedExpr::Mod(lhs, rhs) => Ok(RpNode::Binary {
      op: RpBinaryOp::Mod,
      lhs: Box::new(lower_node(lhs)?),
      rhs: Box::new(lower_node(rhs)?),
    }),
    UnifiedExpr::Pow(lhs, rhs) => Ok(RpNode::Binary {
      op: RpBinaryOp::Pow,
      lhs: Box::new(lower_node(lhs)?),
      rhs: Box::new(lower_node(rhs)?),
    }),
    UnifiedExpr::Neg(arg) => Ok(RpNode::Unary {
      op: RpUnaryOp::Neg,
      arg: Box::new(lower_node(arg)?),
    }),

    UnifiedExpr::Floor(arg) => Ok(RpNode::Unary {
      op: RpUnaryOp::Floor,
      arg: Box::new(lower_node(arg)?),
    }),
    UnifiedExpr::Ceil(arg) => Ok(RpNode::Unary {
      op: RpUnaryOp::Ceil,
      arg: Box::new(lower_node(arg)?),
    }),
    UnifiedExpr::Abs(arg) => Ok(RpNode::Unary {
      op: RpUnaryOp::Abs,
      arg: Box::new(lower_node(arg)?),
    }),
    UnifiedExpr::Sqrt(arg) => Ok(RpNode::Unary {
      op: RpUnaryOp::Sqrt,
      arg: Box::new(lower_node(arg)?),
    }),
    UnifiedExpr::Sin(arg) => Ok(RpNode::Unary {
      op: RpUnaryOp::Sin,
      arg: Box::new(lower_node(arg)?),
    }),
    UnifiedExpr::Cos(arg) => Ok(RpNode::Unary {
      op: RpUnaryOp::Cos,
      arg: Box::new(lower_node(arg)?),
    }),
    UnifiedExpr::Tan(arg) => Ok(RpNode::Unary {
      op: RpUnaryOp::Tan,
      arg: Box::new(lower_node(arg)?),
    }),
    UnifiedExpr::Exp(arg) => Ok(RpNode::Unary {
      op: RpUnaryOp::Exp,
      arg: Box::new(lower_node(arg)?),
    }),
    UnifiedExpr::Ln(arg) => Ok(RpNode::Unary {
      op: RpUnaryOp::Ln,
      arg: Box::new(lower_node(arg)?),
    }),

    UnifiedExpr::Lt(lhs, rhs) => Ok(RpNode::Binary {
      op: RpBinaryOp::Lt,
      lhs: Box::new(lower_node(lhs)?),
      rhs: Box::new(lower_node(rhs)?),
    }),
    UnifiedExpr::Gt(lhs, rhs) => Ok(RpNode::Binary {
      op: RpBinaryOp::Gt,
      lhs: Box::new(lower_node(lhs)?),
      rhs: Box::new(lower_node(rhs)?),
    }),
    UnifiedExpr::Le(lhs, rhs) => Ok(RpNode::Binary {
      op: RpBinaryOp::Le,
      lhs: Box::new(lower_node(lhs)?),
      rhs: Box::new(lower_node(rhs)?),
    }),
    UnifiedExpr::Ge(lhs, rhs) => Ok(RpNode::Binary {
      op: RpBinaryOp::Ge,
      lhs: Box::new(lower_node(lhs)?),
      rhs: Box::new(lower_node(rhs)?),
    }),
    UnifiedExpr::Eq(lhs, rhs) => Ok(RpNode::Binary {
      op: RpBinaryOp::Eq,
      lhs: Box::new(lower_node(lhs)?),
      rhs: Box::new(lower_node(rhs)?),
    }),
    UnifiedExpr::Ne(lhs, rhs) => Ok(RpNode::Binary {
      op: RpBinaryOp::Ne,
      lhs: Box::new(lower_node(lhs)?),
      rhs: Box::new(lower_node(rhs)?),
    }),

    UnifiedExpr::And(lhs, rhs) => Ok(RpNode::Binary {
      op: RpBinaryOp::And,
      lhs: Box::new(lower_node(lhs)?),
      rhs: Box::new(lower_node(rhs)?),
    }),
    UnifiedExpr::Or(lhs, rhs) => Ok(RpNode::Binary {
      op: RpBinaryOp::Or,
      lhs: Box::new(lower_node(lhs)?),
      rhs: Box::new(lower_node(rhs)?),
    }),
    UnifiedExpr::Not(arg) => Ok(RpNode::Unary {
      op: RpUnaryOp::Not,
      arg: Box::new(lower_node(arg)?),
    }),

    UnifiedExpr::If { cond, then_, else_ } => Ok(RpNode::Select {
      cond: Box::new(lower_node(cond)?),
      then_: Box::new(lower_node(then_)?),
      else_: Box::new(lower_node(else_)?),
    }),

    UnifiedExpr::Let { name, value, body } => Ok(RpNode::Let {
      name: name.clone(),
      value: Box::new(lower_node(value)?),
      body: Box::new(lower_node(body)?),
    }),
    UnifiedExpr::Lambda { param, body } => Ok(RpNode::Lambda {
      param: param.clone(),
      body: Box::new(lower_node(body)?),
    }),

    UnifiedExpr::Apply { func, args } => {
      let mut lowered = Vec::with_capacity(args.len());
      for arg in args {
        lowered.push(lower_node(arg)?);
      }
      Ok(RpNode::Call {
        func: func.clone(),
        args: lowered,
      })
    }

    UnifiedExpr::Fx(body) => lower_node(body),

    UnifiedExpr::Interop { lang, code } => Ok(RpNode::InteropCall {
      symbol: format!("interop:{}", lang),
      args: vec![RpNode::Const(RpValue::String(code.clone()))],
    }),

    UnifiedExpr::Derived { op, args } => {
      let mut lowered = Vec::with_capacity(args.len());
      for arg in args {
        lowered.push(lower_node(arg)?);
      }
      Ok(RpNode::Derived {
        op: *op,
        args: lowered,
      })
    }

    UnifiedExpr::AttrSet(pairs) => {
      let mut lowered = Vec::with_capacity(pairs.len());
      for (k, v) in pairs {
        lowered.push((k.clone(), lower_node(v)?));
      }
      Ok(RpNode::AttrSet(lowered))
    }

    UnifiedExpr::List(items) => {
      let mut lowered = Vec::with_capacity(items.len());
      for item in items {
        lowered.push(lower_node(item)?);
      }
      Ok(RpNode::List(lowered))
    }

    UnifiedExpr::Null => Ok(RpNode::Construct {
      variant: "Null".to_string(),
      args: Vec::new(),
    }),

    UnifiedExpr::Construct { variant, args } => {
      let mut lowered = Vec::with_capacity(args.len());
      for arg in args {
        lowered.push(lower_node(arg)?);
      }
      Ok(RpNode::Construct {
        variant: variant.clone(),
        args: lowered,
      })
    }

    // Y08b-2: Throw - 런타임 에러 발생 (non-exhaustive match 등)
    UnifiedExpr::Throw(msg) => Ok(RpNode::Throw {
      message: msg.clone(),
    }),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::fx::meaning_op::MeaningOpId;
  use crate::lang::pnix::UnifiedExpr;

  #[test]
  fn signalvar_lowers_to_getsignal() {
    let expr = UnifiedExpr::SignalVar("time".to_string());
    let plan = unified_to_runtime_plan(&expr).unwrap();
    assert!(matches!(plan.root, RpNode::GetSignal { name } if name == "time"));
  }

  #[test]
  fn param_signal_requires_resolution() {
    let expr = UnifiedExpr::ParamSignal("time".to_string());
    let err = unified_to_runtime_plan(&expr).unwrap_err();
    assert!(matches!(
      err,
      RuntimePlanError::SignalResolutionRequired { .. }
    ));
  }

  #[test]
  fn derived_allows_non_time_ops() {
    let expr = UnifiedExpr::Derived {
      op: MeaningOpId::AngleFromSecond,
      args: Vec::new(),
    };
    let plan = unified_to_runtime_plan(&expr).unwrap();
    assert!(matches!(plan.root, RpNode::Derived { op, .. } if op == MeaningOpId::AngleFromSecond));
  }

  #[test]
  fn let_lowers_to_runtime_plan() {
    let expr = UnifiedExpr::Let {
      name: "x".to_string(),
      value: Box::new(UnifiedExpr::int(1)),
      body: Box::new(UnifiedExpr::var("x")),
    };
    let plan = unified_to_runtime_plan(&expr).unwrap();
    assert!(matches!(plan.root, RpNode::Let { ref name, .. } if name == "x"));
  }

  #[test]
  fn lambda_lowers_to_runtime_plan() {
    let expr = UnifiedExpr::Lambda {
      param: "x".to_string(),
      body: Box::new(UnifiedExpr::var("x")),
    };
    let plan = unified_to_runtime_plan(&expr).unwrap();
    assert!(matches!(plan.root, RpNode::Lambda { ref param, .. } if param == "x"));
  }

  #[test]
  fn throw_lowers_to_runtime_plan() {
    let expr = UnifiedExpr::Throw("boom".to_string());
    let plan = unified_to_runtime_plan(&expr).unwrap();
    assert!(matches!(plan.root, RpNode::Throw { ref message } if message == "boom"));
  }
}
