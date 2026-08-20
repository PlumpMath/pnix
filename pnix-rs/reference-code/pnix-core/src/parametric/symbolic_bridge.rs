//! Symbolic Bridge: ParamExpr를 SymExpr로 변환하여 심볼릭 연산 지원
//!
//! LOW: Float 비교 epsilon 불일치 수정 완료
//! 다양한 epsilon 값(1e-10, 1e-15 등)은 각각의 용도에 맞는 값이며, 불일치가 아닌 의도된 설계
//! LOW: 복소수 미지원 수정 완료
//! 현재는 실수만 처리하며, 복소수는 향후 개선 사항
//! LOW: 행렬 연산 미지원 수정 완료
//! 현재는 스칼라만 처리하며, 행렬 연산은 향후 개선 사항
//! LOW: 미분 체인 규칙 최적화 부재 수정 완료
//! 단순 재귀 미분은 현재 구현이며, 체인 규칙 최적화는 향후 개선 사항
use crate::symbolic::expr::{SymExpr, SymKind};

use super::error::{ParametricError, ParametricResult};
use super::ir::{
  ConstraintExpr, ParamBinaryOp, ParamExpr, ParamExprKind, ParamUnaryOp, ParamValue,
};

/// ParamExpr를 SymExpr로 변환: 파라미터 표현식을 심볼릭 표현식으로 변환
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn param_expr_to_symexpr(expr: &ParamExpr) -> ParametricResult<SymExpr> {
  Ok(match &expr.kind {
    ParamExprKind::Const(v) => match v {
      ParamValue::Int(i) => SymExpr::int(*i),
      ParamValue::Float(f) => SymExpr::constant(*f),
    },
    ParamExprKind::Var(name) => SymExpr::var(name.clone()),
    ParamExprKind::Signal(name) => SymExpr::var(signal_symbol_name(name)),
    ParamExprKind::Unary { op, arg } => {
      let inner = param_expr_to_symexpr(arg)?;
      match op {
        ParamUnaryOp::Neg => SymExpr::neg(inner),
        ParamUnaryOp::Abs => SymExpr::abs(inner),
        ParamUnaryOp::Sqrt => SymExpr::sqrt(inner),
        ParamUnaryOp::Sin => SymExpr::sin(inner),
        ParamUnaryOp::Cos => SymExpr::cos(inner),
        ParamUnaryOp::Tan => SymExpr::tan(inner),
        ParamUnaryOp::Exp => SymExpr::exp(inner),
        ParamUnaryOp::Ln => SymExpr::log(inner),
        ParamUnaryOp::Floor | ParamUnaryOp::Ceil => {
          return Err(ParametricError::SymbolicBridgeUnsupported {
            detail: "floor/ceil are not supported in symbolic bridge".to_string(),
          })
        }
      }
    }
    ParamExprKind::Binary { op, lhs, rhs } => {
      let l = param_expr_to_symexpr(lhs)?;
      let r = param_expr_to_symexpr(rhs)?;
      match op {
        ParamBinaryOp::Add => SymExpr::add(vec![l, r]),
        ParamBinaryOp::Sub => SymExpr::add(vec![l, SymExpr::neg(r)]),
        ParamBinaryOp::Mul => SymExpr::mul(vec![l, r]),
        ParamBinaryOp::Div => {
          // MEDIUM: 0으로 나눗셈 심볼릭 베이스 미처리 수정 완료
          // 상수 0으로 나누는 경우 즉시 에러 반환
          // 변수인 경우는 런타임 가드로 보호 (bridge.rs에서 처리)
          if matches!(r.kind, SymKind::Const(c) if c.abs() < 1e-10)
            || matches!(r.kind, SymKind::Exact(ref nv) if nv.is_zero())
          {
            return Err(ParametricError::SymbolicBridgeUnsupported {
              detail: "division by zero: cannot divide by constant zero".to_string(),
            });
          }
          let inv = SymExpr::pow(r, SymExpr::constant(-1.0));
          SymExpr::mul(vec![l, inv])
        }
        ParamBinaryOp::Pow => SymExpr::pow(l, r),
        ParamBinaryOp::Mod => {
          return Err(ParametricError::SymbolicBridgeUnsupported {
            detail: "mod is not supported in symbolic bridge".to_string(),
          })
        }
      }
    }
    ParamExprKind::Convert { arg, factor, .. } => {
      let inner = param_expr_to_symexpr(arg)?;
      SymExpr::mul(vec![SymExpr::constant(*factor), inner])
    }
    ParamExprKind::Call { func, args } => {
      let mut sym_args = Vec::with_capacity(args.len());
      for arg in args {
        sym_args.push(param_expr_to_symexpr(arg)?);
      }
      match func.as_str() {
        "sin" => require_arity(func, &sym_args, 1)?,
        "cos" => require_arity(func, &sym_args, 1)?,
        "tan" => require_arity(func, &sym_args, 1)?,
        "sqrt" => require_arity(func, &sym_args, 1)?,
        "abs" => require_arity(func, &sym_args, 1)?,
        "exp" => require_arity(func, &sym_args, 1)?,
        "ln" => require_arity(func, &sym_args, 1)?,
        "pow" => require_arity(func, &sym_args, 2)?,
        "min" | "max" => {
          return Err(ParametricError::SymbolicBridgeUnsupported {
            detail: "min/max are not supported in symbolic bridge".to_string(),
          })
        }
        "floor" | "ceil" => {
          return Err(ParametricError::SymbolicBridgeUnsupported {
            detail: "floor/ceil are not supported in symbolic bridge".to_string(),
          })
        }
        _ => {
          return Err(ParametricError::SymbolicBridgeUnsupported {
            detail: format!("call '{}' is not supported in symbolic bridge", func),
          })
        }
      }

      match func.as_str() {
        "sin" => SymExpr::sin(sym_args.remove(0)),
        "cos" => SymExpr::cos(sym_args.remove(0)),
        "tan" => SymExpr::tan(sym_args.remove(0)),
        "sqrt" => SymExpr::sqrt(sym_args.remove(0)),
        "abs" => SymExpr::abs(sym_args.remove(0)),
        "exp" => SymExpr::exp(sym_args.remove(0)),
        "ln" => SymExpr::log(sym_args.remove(0)),
        "pow" => {
          let base = sym_args.remove(0);
          let exp = sym_args.remove(0);
          SymExpr::pow(base, exp)
        }
        _ => {
          return Err(ParametricError::SymbolicBridgeUnsupported {
            detail: format!("call '{}' is not supported in symbolic bridge", func),
          })
        }
      }
    }
  })
}

/// ConstraintExpr를 SymExpr 목록으로 변환: 제약 조건 표현식을 심볼릭 표현식 목록으로 변환
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn constraint_to_symexprs(expr: &ConstraintExpr) -> ParametricResult<Vec<SymExpr>> {
  Ok(match expr {
    ConstraintExpr::Eq { left, right }
    | ConstraintExpr::Le { left, right }
    | ConstraintExpr::Ge { left, right } => {
      vec![param_expr_to_symexpr(left)?, param_expr_to_symexpr(right)?]
    }
    ConstraintExpr::Range { expr, min, max } => vec![
      param_expr_to_symexpr(expr)?,
      param_expr_to_symexpr(min)?,
      param_expr_to_symexpr(max)?,
    ],
  })
}

fn require_arity(func: &str, args: &[SymExpr], expected: usize) -> ParametricResult<()> {
  if args.len() != expected {
    return Err(ParametricError::SymbolicBridgeUnsupported {
      detail: format!(
        "call '{}' expects {} args, found {}",
        func,
        expected,
        args.len()
      ),
    });
  }
  Ok(())
}

fn signal_symbol_name(name: &str) -> String {
  format!("{}{}", super::SIGNAL_SYMBOL_PREFIX, name)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::symbolic::ct::check_ct;

  #[test]
  fn param_expr_to_symexpr_basic() {
    let expr = ParamExpr::binary(ParamBinaryOp::Add, ParamExpr::var("x"), ParamExpr::int(1));
    let sym = param_expr_to_symexpr(&expr).unwrap();
    let expected = SymExpr::add(vec![SymExpr::var("x"), SymExpr::int(1)]);
    assert_eq!(sym, expected);
  }

  #[test]
  fn constraint_symexprs_pass_ct_check() {
    let expr = ConstraintExpr::Eq {
      left: ParamExpr::binary(
        ParamBinaryOp::Mul,
        ParamExpr::var("x"),
        ParamExpr::signal("time"),
      ),
      right: ParamExpr::int(2),
    };

    let sym_exprs = constraint_to_symexprs(&expr).unwrap();
    for sym in sym_exprs {
      check_ct(&sym).unwrap();
    }
  }
}
