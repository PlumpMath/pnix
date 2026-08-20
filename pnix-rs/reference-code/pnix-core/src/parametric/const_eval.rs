//! 상수 평가: 파라미터 표현식을 컴파일 타임에 상수로 평가

use super::error::{ParametricError, ParametricResult};
use super::ir::{ParamBinaryOp, ParamExpr, ParamExprKind, ParamUnaryOp, ParamValue};

pub fn const_eval(expr: &ParamExpr) -> ParametricResult<Option<f64>> {
  match &expr.kind {
    ParamExprKind::Const(v) => Ok(Some(match v {
      ParamValue::Int(i) => *i as f64,
      ParamValue::Float(f) => *f,
    })),
    ParamExprKind::Var(_) | ParamExprKind::Signal(_) => Ok(None),
    ParamExprKind::Call { func, args } => {
      if func == "min" || func == "max" {
        if args.is_empty() {
          return Err(ParametricError::InvalidConstantExpr {
            detail: "min/max requires at least one argument".to_string(),
          });
        }
        let mut values = Vec::with_capacity(args.len());
        for arg in args {
          let Some(v) = const_eval(arg)? else {
            return Ok(None);
          };
          values.push(v);
        }
        let mut out = values[0];
        for v in values.into_iter().skip(1) {
          out = if func == "min" {
            out.min(v)
          } else {
            out.max(v)
          };
        }
        if !out.is_finite() {
          return Err(ParametricError::InvalidConstantExpr {
            detail: "constant evaluation overflow".to_string(),
          });
        }
        Ok(Some(out))
      } else {
        Ok(None)
      }
    }
    ParamExprKind::Unary { op, arg } => {
      let Some(v) = const_eval(arg)? else {
        return Ok(None);
      };
      let out = match op {
        ParamUnaryOp::Neg => -v,
        ParamUnaryOp::Floor => v.floor(),
        ParamUnaryOp::Ceil => v.ceil(),
        ParamUnaryOp::Abs => v.abs(),
        ParamUnaryOp::Sqrt => {
          if v < 0.0 {
            return Err(ParametricError::InvalidConstantExpr {
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
            return Err(ParametricError::InvalidConstantExpr {
              detail: "ln domain error: value <= 0".to_string(),
            });
          }
          v.ln()
        }
      };
      if !out.is_finite() {
        return Err(ParametricError::InvalidConstantExpr {
          detail: "constant evaluation overflow".to_string(),
        });
      }
      Ok(Some(out))
    }
    ParamExprKind::Convert { arg, factor, .. } => {
      if !factor.is_finite() || *factor <= 0.0 {
        return Err(ParametricError::UnitConversionInvalidFactor { factor: *factor });
      }
      let Some(v) = const_eval(arg)? else {
        return Ok(None);
      };
      let out = v * factor;
      if !out.is_finite() {
        return Err(ParametricError::InvalidConstantExpr {
          detail: "constant evaluation overflow".to_string(),
        });
      }
      Ok(Some(out))
    }
    ParamExprKind::Binary { op, lhs, rhs } => {
      let Some(a) = const_eval(lhs)? else {
        return Ok(None);
      };
      let Some(b) = const_eval(rhs)? else {
        return Ok(None);
      };
      let out = match op {
        ParamBinaryOp::Add => a + b,
        ParamBinaryOp::Sub => a - b,
        ParamBinaryOp::Mul => a * b,
        ParamBinaryOp::Div => {
          if b == 0.0 {
            return Err(ParametricError::InvalidConstantExpr {
              detail: "division by zero".to_string(),
            });
          }
          a / b
        }
        ParamBinaryOp::Mod => {
          if b == 0.0 {
            return Err(ParametricError::InvalidConstantExpr {
              detail: "mod by zero".to_string(),
            });
          }
          a % b
        }
        ParamBinaryOp::Pow => a.powf(b),
      };
      if !out.is_finite() {
        return Err(ParametricError::InvalidConstantExpr {
          detail: "constant evaluation overflow".to_string(),
        });
      }
      Ok(Some(out))
    }
  }
}
