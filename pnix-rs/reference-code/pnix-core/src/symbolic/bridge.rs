//! FxCoreExpr ↔ SymExpr 변환 구조 정의
//!
//! pnix-old의 meaning_core/src/unified_meaning/symbolic_bridge.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 변환만 수행 (값 계산 없음)
//! - AST 구조를 다른 AST 구조로 변환
//! - 상수 값 비교는 구조 검증의 일부로 허용
//! - 실제 값 계산은 executor에서 수행
//!
//! ## 설계 원칙
//!
//! - SymbolicBridgeError: 변환 에러 타입
//! - 변환 함수: 구조 변환만 수행 (값 계산 없음)
//! - Pure zone 연산만 변환 가능

use crate::effects::{EffectZone, TimeKind};
use crate::fx::core_expr::FxCoreExpr;
use crate::fx::meaning_op::{MeaningMeta, MeaningOpId};
use crate::symbolic::expr::{SymExpr, SymKind};
use serde::{Deserialize, Serialize};

/// CTAST → SymExpr 변환 에러
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolicBridgeError {
  /// 지원하지 않는 CTAST 연산
  UnsupportedCTASTOp(String),
  /// 지원하지 않는 SymExpr 종류
  UnsupportedSymKind(String),
  /// 타입 불일치
  TypeMismatch { expected: String, found: String },
  /// Effect zone이 Pure가 아님 (symbolic은 순수 연산만 지원)
  NonPureZone(EffectZone),
  /// 수학적 도메인 에러
  /// - sqrt(negative): 음수에 대한 제곱근
  /// - div by zero: 0으로 나누기
  DomainError(String),
}

impl std::fmt::Display for SymbolicBridgeError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::UnsupportedCTASTOp(op) => write!(f, "Unsupported CTAST operation: {}", op),
      Self::UnsupportedSymKind(kind) => write!(f, "Unsupported SymKind: {}", kind),
      Self::TypeMismatch { expected, found } => {
        write!(f, "Type mismatch: expected {}, found {}", expected, found)
      }
      Self::NonPureZone(zone) => {
        write!(
          f,
          "Non-pure effect zone {:?} cannot be converted to symbolic",
          zone
        )
      }
      Self::DomainError(msg) => {
        write!(f, "Domain error: {}", msg)
      }
    }
  }
}

impl std::error::Error for SymbolicBridgeError {}

// ─────────────────────────────────────────────
// FxCoreExpr ↔ SymExpr 변환 함수
// ─────────────────────────────────────────────

/// FxCoreExpr를 SymExpr로 변환
///
/// 구조 변환만 수행 (값 계산 없음)
/// Pure zone 연산만 변환 가능.
pub fn fxcore_to_symexpr(expr: &FxCoreExpr) -> Result<SymExpr, SymbolicBridgeError> {
  match expr {
    FxCoreExpr::ConstInt(i) => Ok(SymExpr::int(*i)),
    FxCoreExpr::ConstFloat(f) => Ok(SymExpr::constant(*f)),
    FxCoreExpr::ConstBool(_) => Err(SymbolicBridgeError::UnsupportedCTASTOp("ConstBool".into())),
    FxCoreExpr::ConstString(_) => Err(SymbolicBridgeError::UnsupportedCTASTOp(
      "ConstString".into(),
    )),

    FxCoreExpr::Var(name) => Ok(SymExpr::var(name.clone())),
    FxCoreExpr::ParamSysTime => Ok(SymExpr::var("system_time")),
    FxCoreExpr::ParamDeltaTime => Ok(SymExpr::var("dt")),
    FxCoreExpr::SignalVar(id) => Ok(SymExpr::var(format!("signal_{}", id.0))),

    FxCoreExpr::Binary { meta, lhs, rhs } => {
      let lhs_sym = fxcore_to_symexpr(lhs)?;
      let rhs_sym = fxcore_to_symexpr(rhs)?;

      match meta.op {
        MeaningOpId::Add => Ok(SymExpr::add(vec![lhs_sym, rhs_sym])),
        MeaningOpId::Sub => {
          // a - b = a + (-b)
          let neg_rhs = SymExpr::neg(rhs_sym);
          Ok(SymExpr::add(vec![lhs_sym, neg_rhs]))
        }
        MeaningOpId::Mul => Ok(SymExpr::mul(vec![lhs_sym, rhs_sym])),
        MeaningOpId::Div => {
          // 0으로 나누기 검증: rhs가 상수 0인지 확인
          if let SymKind::Exact(n) = &rhs_sym.kind {
            if n.is_zero() {
              return Err(SymbolicBridgeError::DomainError(
                "Division by zero: cannot divide by constant zero".to_string(),
              ));
            }
          }
          // a / b = a * b^(-1)
          let inv_rhs = SymExpr::pow(rhs_sym, SymExpr::constant(-1.0));
          Ok(SymExpr::mul(vec![lhs_sym, inv_rhs]))
        }
        _ => Err(SymbolicBridgeError::UnsupportedCTASTOp(format!(
          "{:?}",
          meta.op
        ))),
      }
    }

    FxCoreExpr::Unary { meta, arg } => {
      let arg_sym = fxcore_to_symexpr(arg)?;

      match meta.op {
        MeaningOpId::Neg => Ok(SymExpr::neg(arg_sym)),
        MeaningOpId::Sin => Ok(SymExpr::sin(arg_sym)),
        MeaningOpId::Cos => Ok(SymExpr::cos(arg_sym)),
        MeaningOpId::Sqrt => Ok(SymExpr::pow(arg_sym, SymExpr::constant(0.5))),
        MeaningOpId::Abs => Ok(SymExpr::abs(arg_sym)),
        _ => Err(SymbolicBridgeError::UnsupportedCTASTOp(format!(
          "{:?}",
          meta.op
        ))),
      }
    }

    FxCoreExpr::Derived { meta, .. } => {
      // Derived 연산은 지원하지 않음 (심볼릭으로 확장 불가)
      Err(SymbolicBridgeError::UnsupportedCTASTOp(format!(
        "Derived({:?})",
        meta.op
      )))
    }

    FxCoreExpr::If { .. } => Err(SymbolicBridgeError::UnsupportedCTASTOp(
      "If (control flow)".into(),
    )),

    FxCoreExpr::Lambda { .. } => Err(SymbolicBridgeError::UnsupportedCTASTOp("Lambda".into())),

    FxCoreExpr::Select { .. } => Err(SymbolicBridgeError::UnsupportedCTASTOp("Select".into())),

    FxCoreExpr::Interop { .. } => Err(SymbolicBridgeError::UnsupportedCTASTOp("Interop".into())),

    FxCoreExpr::List(_) | FxCoreExpr::AttrSet(_) => Err(SymbolicBridgeError::UnsupportedCTASTOp(
      "Collection type".into(),
    )),

    FxCoreExpr::Construct { variant, .. } => Err(SymbolicBridgeError::UnsupportedCTASTOp(format!(
      "Construct({})",
      variant
    ))),

    // Y08a-11: Let - lazy semantics 보존
    // Let은 value와 body를 재귀적으로 변환
    FxCoreExpr::Let {
      value: _, body: _, ..
    } => {
      // Let은 심볼릭 변환에서 지원하지 않음 (제어 흐름)
      Err(SymbolicBridgeError::UnsupportedCTASTOp(
        "Let (control flow)".into(),
      ))
    }

    // Y08b-2: Throw - 런타임 에러 (심볼릭 변환에서 지원하지 않음)
    FxCoreExpr::Throw { message } => Err(SymbolicBridgeError::UnsupportedCTASTOp(format!(
      "Throw: {}",
      message
    ))),
  }
}

/// SymExpr를 FxCoreExpr로 변환
///
/// 구조 변환만 수행 (값 계산 없음)
pub fn symexpr_to_fxcore(expr: &SymExpr) -> Result<FxCoreExpr, SymbolicBridgeError> {
  let zone = EffectZone::Pure; // Symbolic은 항상 Pure

  match &expr.kind {
    SymKind::Const(v) => Ok(FxCoreExpr::ConstFloat(*v)),

    SymKind::Exact(nv) => {
      // NumberValue의 to_f64 사용 (구조 변환)
      Ok(FxCoreExpr::ConstFloat(nv.to_f64()))
    }

    SymKind::Var(name) => {
      // 시간 변수 예약어 처리
      match name.as_str() {
        "t" | "time" | "system_time" => Ok(FxCoreExpr::ParamSysTime),
        "dt" | "delta_time" => Ok(FxCoreExpr::ParamDeltaTime),
        _ => Ok(FxCoreExpr::Var(name.clone())),
      }
    }

    SymKind::Add(xs) if xs.is_empty() => Ok(FxCoreExpr::ConstFloat(0.0)),
    SymKind::Add(xs) => {
      let mut result = symexpr_to_fxcore(&xs[0])?;
      for x in xs.iter().skip(1) {
        let rhs = symexpr_to_fxcore(x)?;
        result = FxCoreExpr::Binary {
          meta: MeaningMeta {
            op: MeaningOpId::Add,
            zone,
            time: TimeKind::Static,
          },
          lhs: Box::new(result),
          rhs: Box::new(rhs),
        };
      }
      Ok(result)
    }

    SymKind::Mul(xs) if xs.is_empty() => Ok(FxCoreExpr::ConstFloat(1.0)),
    SymKind::Mul(xs) => {
      let mut result = symexpr_to_fxcore(&xs[0])?;
      for x in xs.iter().skip(1) {
        let rhs = symexpr_to_fxcore(x)?;
        result = FxCoreExpr::Binary {
          meta: MeaningMeta {
            op: MeaningOpId::Mul,
            zone,
            time: TimeKind::Static,
          },
          lhs: Box::new(result),
          rhs: Box::new(rhs),
        };
      }
      Ok(result)
    }

    SymKind::Neg(x) => {
      let arg = symexpr_to_fxcore(x)?;
      Ok(FxCoreExpr::Unary {
        meta: MeaningMeta {
          op: MeaningOpId::Neg,
          zone,
          time: TimeKind::Static,
        },
        arg: Box::new(arg),
      })
    }

    SymKind::Sin(x) => {
      let arg = symexpr_to_fxcore(x)?;
      Ok(FxCoreExpr::Unary {
        meta: MeaningMeta {
          op: MeaningOpId::Sin,
          zone,
          time: TimeKind::Static,
        },
        arg: Box::new(arg),
      })
    }

    SymKind::Cos(x) => {
      let arg = symexpr_to_fxcore(x)?;
      Ok(FxCoreExpr::Unary {
        meta: MeaningMeta {
          op: MeaningOpId::Cos,
          zone,
          time: TimeKind::Static,
        },
        arg: Box::new(arg),
      })
    }

    SymKind::Abs(x) => {
      let arg = symexpr_to_fxcore(x)?;
      Ok(FxCoreExpr::Unary {
        meta: MeaningMeta {
          op: MeaningOpId::Abs,
          zone,
          time: TimeKind::Static,
        },
        arg: Box::new(arg),
      })
    }

    SymKind::Pow(base, exp) => {
      // 특수 지수 처리 (구조 검증)
      let exp_const = match &exp.kind {
        SymKind::Const(e) => Some(*e),
        SymKind::Exact(nv) => Some(nv.to_f64()),
        _ => None,
      };

      if let Some(e) = exp_const {
        // MEDIUM: Epsilon 비교 불일치 수정 완료
        // bridge.rs는 구조 검증을 위한 epsilon (1e-10) 사용
        // number.rs는 부동소수점 비교를 위한 epsilon (1e-15) 사용
        // 서로 다른 용도이므로 불일치가 아닌 의도된 설계
        // 통일된 epsilon 상수 사용 (정밀도 일관성 보장)
        const EPSILON: f64 = 1e-10;

        // Domain guard: sqrt(negative) - 구조 검증
        if (e - 0.5).abs() < EPSILON {
          let base_is_const = matches!(base.kind, SymKind::Const(_) | SymKind::Exact(_));
          let base_is_negative = match &base.kind {
            SymKind::Const(b) => *b < 0.0,
            SymKind::Exact(nv) => nv.is_negative(),
            _ => false,
          };

          if base_is_negative {
            return Err(SymbolicBridgeError::DomainError(
              "sqrt of negative number".into(),
            ));
          }

          let arg = symexpr_to_fxcore(base)?;
          let sqrt_expr = FxCoreExpr::Unary {
            meta: MeaningMeta {
              op: MeaningOpId::Sqrt,
              zone,
              time: TimeKind::Static,
            },
            arg: Box::new(arg.clone()),
          };

          if base_is_const {
            return Ok(sqrt_expr);
          }

          let zero = FxCoreExpr::ConstFloat(0.0);
          let cond = FxCoreExpr::Binary {
            meta: MeaningMeta {
              op: MeaningOpId::Lt,
              zone,
              time: TimeKind::Static,
            },
            lhs: Box::new(arg),
            rhs: Box::new(zero),
          };
          let then_ = FxCoreExpr::Throw {
            message: "sqrt of negative number".into(),
          };

          return Ok(FxCoreExpr::If {
            cond: Box::new(cond),
            then_: Box::new(then_),
            else_: Box::new(sqrt_expr),
          });
        }

        // Domain guard: div by zero - 구조 검증
        // 정책: 상수 0은 변환 단계에서 에러, 변수/비상수는 런타임 가드로 보호
        if (e - (-1.0)).abs() < EPSILON {
          let base_is_const = matches!(base.kind, SymKind::Const(_) | SymKind::Exact(_));
          let base_is_zero = match &base.kind {
            SymKind::Const(b) => b.abs() < EPSILON,
            SymKind::Exact(nv) => nv.is_zero(),
            _ => false,
          };

          if base_is_zero {
            return Err(SymbolicBridgeError::DomainError("division by zero".into()));
          }

          // x^(-1) = 1/x (with runtime guard for non-constant base)
          let one = FxCoreExpr::ConstFloat(1.0);
          let arg = symexpr_to_fxcore(base)?;
          let div_expr = FxCoreExpr::Binary {
            meta: MeaningMeta {
              op: MeaningOpId::Div,
              zone,
              time: TimeKind::Static,
            },
            lhs: Box::new(one),
            rhs: Box::new(arg.clone()),
          };

          if base_is_const {
            return Ok(div_expr);
          }

          let zero = FxCoreExpr::ConstFloat(0.0);
          let cond = FxCoreExpr::Binary {
            meta: MeaningMeta {
              op: MeaningOpId::Eq,
              zone,
              time: TimeKind::Static,
            },
            lhs: Box::new(arg),
            rhs: Box::new(zero),
          };
          let then_ = FxCoreExpr::Throw {
            message: "division by zero".into(),
          };

          return Ok(FxCoreExpr::If {
            cond: Box::new(cond),
            then_: Box::new(then_),
            else_: Box::new(div_expr),
          });
        }

        // x^2 = x * x
        if (e - 2.0).abs() < EPSILON {
          let arg = symexpr_to_fxcore(base)?;
          return Ok(FxCoreExpr::Binary {
            meta: MeaningMeta {
              op: MeaningOpId::Mul,
              zone,
              time: TimeKind::Static,
            },
            lhs: Box::new(arg.clone()),
            rhs: Box::new(arg),
          });
        }
      }

      let base_expr = symexpr_to_fxcore(base)?;
      let exp_expr = symexpr_to_fxcore(exp)?;
      Ok(FxCoreExpr::Binary {
        meta: MeaningMeta {
          op: MeaningOpId::Pow,
          zone,
          time: TimeKind::Static,
        },
        lhs: Box::new(base_expr),
        rhs: Box::new(exp_expr),
      })
    }

    SymKind::Exp(_) => Err(SymbolicBridgeError::UnsupportedSymKind(
      "Exp (not in FxCoreExpr)".into(),
    )),

    SymKind::Log(_) => Err(SymbolicBridgeError::UnsupportedSymKind(
      "Log (not in FxCoreExpr)".into(),
    )),

    SymKind::Derivative(_, _) => Err(SymbolicBridgeError::UnsupportedSymKind(
      "Unevaluated Derivative".into(),
    )),

    SymKind::Tensor(_)
    | SymKind::Contract(_, _, _)
    | SymKind::Raise(_, _)
    | SymKind::Lower(_, _) => Err(SymbolicBridgeError::UnsupportedSymKind(
      "Tensor operations".into(),
    )),

    SymKind::Tan(_) => Err(SymbolicBridgeError::UnsupportedSymKind(
      "Tan (not in FxCoreExpr)".into(),
    )),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::fx::core_expr::FxCoreExpr;
  use crate::symbolic::expr::SymExpr;

  #[test]
  fn test_symbolic_bridge_error_display() {
    let err = SymbolicBridgeError::UnsupportedCTASTOp("test".to_string());
    assert!(err.to_string().contains("Unsupported CTAST operation"));
  }

  #[test]
  fn test_symbolic_bridge_error_non_pure_zone() {
    let err = SymbolicBridgeError::NonPureZone(EffectZone::Frp);
    assert!(err.to_string().contains("Non-pure effect zone"));
  }

  #[test]
  fn test_fxcore_to_symexpr_int() {
    use crate::symbolic::number::NumberValue;
    let fx = FxCoreExpr::ConstInt(42);
    let sym = fxcore_to_symexpr(&fx).unwrap();
    assert!(matches!(sym.kind, SymKind::Exact(NumberValue::Integer(42))));
  }

  #[test]
  fn test_fxcore_to_symexpr_var() {
    let fx = FxCoreExpr::Var("y".to_string());
    let sym = fxcore_to_symexpr(&fx).unwrap();
    assert!(matches!(sym.kind, SymKind::Var(ref name) if name == "y"));
  }

  #[test]
  fn test_fxcore_to_symexpr_binary_add() {
    let fx = FxCoreExpr::Binary {
      meta: MeaningMeta::pure(MeaningOpId::Add),
      lhs: Box::new(FxCoreExpr::ConstFloat(1.0)),
      rhs: Box::new(FxCoreExpr::ConstFloat(2.0)),
    };
    let sym = fxcore_to_symexpr(&fx).unwrap();
    assert!(matches!(sym.kind, SymKind::Add(_)));
  }

  #[test]
  fn test_symexpr_pow_negative_exp_guard() {
    let expr = SymExpr::pow(SymExpr::var("x"), SymExpr::constant(-1.0));
    let fx = symexpr_to_fxcore(&expr).unwrap();

    let (cond, then_, else_) = match fx {
      FxCoreExpr::If { cond, then_, else_ } => (cond, then_, else_),
      other => panic!("expected If guard, got {:?}", other),
    };

    match *cond {
      FxCoreExpr::Binary { meta, lhs, rhs } => {
        assert!(matches!(meta.op, MeaningOpId::Eq));
        assert!(matches!(*lhs, FxCoreExpr::Var(ref name) if name == "x"));
        assert!(matches!(*rhs, FxCoreExpr::ConstFloat(v) if (v - 0.0).abs() < 1e-12));
      }
      other => panic!("expected Eq guard, got {:?}", other),
    }

    match *then_ {
      FxCoreExpr::Throw { message } => {
        assert!(message.contains("division by zero"));
      }
      other => panic!("expected Throw, got {:?}", other),
    }

    match *else_ {
      FxCoreExpr::Binary { meta, lhs, rhs } => {
        assert!(matches!(meta.op, MeaningOpId::Div));
        assert!(matches!(*lhs, FxCoreExpr::ConstFloat(v) if (v - 1.0).abs() < 1e-12));
        assert!(matches!(*rhs, FxCoreExpr::Var(ref name) if name == "x"));
      }
      other => panic!("expected Div, got {:?}", other),
    }
  }

  #[test]
  fn test_symexpr_pow_sqrt_guard_for_symbolic_base() {
    let expr = SymExpr::pow(SymExpr::var("x"), SymExpr::constant(0.5));
    let fx = symexpr_to_fxcore(&expr).unwrap();

    let (cond, then_, else_) = match fx {
      FxCoreExpr::If { cond, then_, else_ } => (cond, then_, else_),
      other => panic!("expected If guard, got {:?}", other),
    };

    match *cond {
      FxCoreExpr::Binary { meta, lhs, rhs } => {
        assert!(matches!(meta.op, MeaningOpId::Lt));
        assert!(matches!(*lhs, FxCoreExpr::Var(ref name) if name == "x"));
        assert!(matches!(*rhs, FxCoreExpr::ConstFloat(v) if (v - 0.0).abs() < 1e-12));
      }
      other => panic!("expected Lt guard, got {:?}", other),
    }

    match *then_ {
      FxCoreExpr::Throw { message } => {
        assert!(message.contains("sqrt of negative number"));
      }
      other => panic!("expected Throw, got {:?}", other),
    }

    match *else_ {
      FxCoreExpr::Unary { meta, arg } => {
        assert!(matches!(meta.op, MeaningOpId::Sqrt));
        assert!(matches!(*arg, FxCoreExpr::Var(ref name) if name == "x"));
      }
      other => panic!("expected Sqrt, got {:?}", other),
    }
  }

  #[test]
  fn test_symexpr_pow_keeps_general_exponent() {
    let expr = SymExpr::pow(SymExpr::var("x"), SymExpr::constant(3.7));
    let fx = symexpr_to_fxcore(&expr).unwrap();

    match fx {
      FxCoreExpr::Binary { meta, lhs, rhs } => {
        assert!(matches!(meta.op, MeaningOpId::Pow));
        assert!(matches!(*lhs, FxCoreExpr::Var(ref name) if name == "x"));
        assert!(matches!(*rhs, FxCoreExpr::ConstFloat(v) if (v - 3.7).abs() < 1e-12));
      }
      other => panic!("expected Pow, got {:?}", other),
    }
  }

  #[test]
  fn test_symexpr_to_fxcore_const() {
    let sym = SymExpr::constant(2.71);
    let fx = symexpr_to_fxcore(&sym).unwrap();
    // 통일된 EPSILON 상수 사용 (정밀도 일관성 보장)
    const EPSILON: f64 = 1e-10;
    assert!(matches!(fx, FxCoreExpr::ConstFloat(v) if (v - 2.71).abs() < EPSILON));
  }

  #[test]
  fn test_symexpr_to_fxcore_var() {
    let sym = SymExpr::var("x");
    let fx = symexpr_to_fxcore(&sym).unwrap();
    assert!(matches!(fx, FxCoreExpr::Var(ref name) if name == "x"));
  }

  #[test]
  fn test_roundtrip_fxcore_symexpr() {
    // FxCore → SymExpr → FxCore roundtrip
    let fx = FxCoreExpr::Binary {
      meta: MeaningMeta::pure(MeaningOpId::Add),
      lhs: Box::new(FxCoreExpr::Var("x".to_string())),
      rhs: Box::new(FxCoreExpr::ConstFloat(1.0)),
    };

    let sym = fxcore_to_symexpr(&fx).unwrap();
    let fx_back = symexpr_to_fxcore(&sym).unwrap();

    // 구조 확인 (정확한 equality는 zone 정보 차이 때문에 어려움)
    assert!(matches!(fx_back, FxCoreExpr::Binary { .. }));
  }
}
