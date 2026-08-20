//! FxLowering - FxSurfaceExpr → FxCoreExpr 변환
//!
//! 언어별 표면 AST를 공통 의미론 AST로 변환한다.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 순수 구조 변환, 값 계산 없음
//!
//! ## 주요 변환
//!
//! - `param.system_time`, `param/system-time`, `__time__` → `ParamSysTime`
//! - `param.delta_time`, `param/delta-time`, `__dt__` → `ParamDeltaTime`
//! - `floor`, `Math/floor` → `Floor`
//! - `+`, `-`, `*`, `/`, `%` → `Add`, `Sub`, `Mul`, `Div`, `Mod`
//!
//! ## 예시
//!
//! ```text
//! FxSurfaceExpr::InfixOp { op: "%", left: Call("floor", [Ident("param.system_time")]), right: 60 }
//!     ↓ lower_surface_to_core()
//! FxCoreExpr::Mod(Floor(ParamSysTime), ConstFloat(60.0))
//! ```

use super::core_expr::FxCoreExpr;
use super::meaning_op::{MeaningMeta, MeaningOpId};
use super::surface::FxSurfaceExpr;
use thiserror::Error;

/// FxSurfaceExpr → FxCoreExpr 변환 에러
#[derive(Debug, Error)]
pub enum FxLoweringError {
  /// 알 수 없는 중위 연산자
  #[error("unknown infix operator '{op}'")]
  UnknownInfixOp {
    /// 연산자 문자열
    op: String,
  },
  /// 알 수 없는 전위 연산자
  #[error("unknown prefix operator '{op}'")]
  UnknownPrefixOp {
    /// 연산자 문자열
    op: String,
  },
}

/// FxSurfaceExpr → FxCoreExpr 변환
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn lower_surface_to_core(expr: &FxSurfaceExpr) -> Result<FxCoreExpr, FxLoweringError> {
  match expr {
    // ========== Literals ==========
    FxSurfaceExpr::ConstInt(v) => Ok(FxCoreExpr::int(*v)),
    FxSurfaceExpr::ConstFloat(v) => Ok(FxCoreExpr::float(*v)),
    FxSurfaceExpr::ConstBool(v) => Ok(FxCoreExpr::bool(*v)),
    FxSurfaceExpr::ConstString(s) => Ok(FxCoreExpr::string(s)),

    // ========== Identifiers ==========
    FxSurfaceExpr::Ident(name) => Ok(lower_ident(name)),

    // ========== Call ==========
    FxSurfaceExpr::Call { func, args } => lower_call(func, args),

    // ========== InfixOp ==========
    FxSurfaceExpr::InfixOp { op, left, right } => {
      let l = lower_surface_to_core(left)?;
      let r = lower_surface_to_core(right)?;
      lower_infix_op(op, l, r)
    }

    // ========== PrefixOp ==========
    FxSurfaceExpr::PrefixOp { op, arg } => {
      let a = lower_surface_to_core(arg)?;
      lower_prefix_op(op, a)
    }

    // ========== If ==========
    FxSurfaceExpr::If { cond, then_, else_ } => Ok(FxCoreExpr::if_then_else(
      lower_surface_to_core(cond)?,
      lower_surface_to_core(then_)?,
      lower_surface_to_core(else_)?,
    )),

    // ========== Let ==========
    FxSurfaceExpr::Let { name, value, body } => {
      // Let 바인딩은 변수 치환으로 처리 (substitute)
      let val = lower_surface_to_core(value)?;
      let bod = lower_surface_to_core(body)?;
      // body에서 name을 val로 치환
      Ok(substitute_var(&bod, name, &val))
    }

    // ========== Fx ==========
    FxSurfaceExpr::Fx(inner) => lower_surface_to_core(inner),
  }
}

/// 식별자 변환
fn lower_ident(name: &str) -> FxCoreExpr {
  // 특수 파라미터 처리
  match name {
    // pnix 스타일
    "param.system_time" | "param.systemTime" | "param.time" => FxCoreExpr::time(),
    "param.delta_time" | "param.deltaTime" | "param.dt" => FxCoreExpr::dt(),

    // clojure 스타일
    "param/system-time" | "param/time" => FxCoreExpr::time(),
    "param/delta-time" | "param/dt" => FxCoreExpr::dt(),

    // 공통 스타일
    "__time__" | "time" | "t" => FxCoreExpr::time(),
    "__dt__" | "dt" => FxCoreExpr::dt(),

    // 일반 변수
    _ => FxCoreExpr::var(name),
  }
}

/// 함수 호출 변환
fn lower_call(func: &str, args: &[FxSurfaceExpr]) -> Result<FxCoreExpr, FxLoweringError> {
  let lowered_args: Vec<FxCoreExpr> = args
    .iter()
    .map(lower_surface_to_core)
    .collect::<Result<_, _>>()?;

  match lowered_args.len() {
    // 단항 함수
    1 => {
      let mut iter = lowered_args.into_iter();
      let Some(arg) = iter.next() else {
        return Ok(make_interop_call(func, Vec::new()));
      };
      Ok(match func {
        "floor" | "Math/floor" | "builtins.floor" => FxCoreExpr::floor(arg),
        "ceil" | "Math/ceil" | "builtins.ceil" => FxCoreExpr::ceil(arg),
        "abs" | "Math/abs" | "builtins.abs" => FxCoreExpr::abs(arg),
        "sqrt" | "Math/sqrt" | "builtins.sqrt" => FxCoreExpr::sqrt(arg),
        "sin" | "Math/sin" | "builtins.sin" => FxCoreExpr::sin(arg),
        "cos" | "Math/cos" | "builtins.cos" => FxCoreExpr::cos(arg),
        "tan" | "Math/tan" | "builtins.tan" => FxCoreExpr::tan(arg),
        "exp" | "Math/exp" | "builtins.exp" => FxCoreExpr::exp(arg),
        "ln" | "log" | "Math/log" | "builtins.log" => FxCoreExpr::ln(arg),
        "not" | "!" => FxCoreExpr::not(arg),
        "-" => FxCoreExpr::neg(arg),
        // 알 수 없는 단항 함수 → InteropCall로 보존
        _ => make_interop_call(func, vec![arg]),
      })
    }

    // 이항 함수
    2 => {
      let mut iter = lowered_args.into_iter();
      let Some(a) = iter.next() else {
        return Ok(make_interop_call(func, Vec::new()));
      };
      let Some(b) = iter.next() else {
        return Ok(make_interop_call(func, vec![a]));
      };
      Ok(match func {
        "mod" | "rem" | "%" => FxCoreExpr::modulo(a, b),
        "add" | "+" => FxCoreExpr::add(a, b),
        "sub" | "-" => FxCoreExpr::sub(a, b),
        "mul" | "*" => FxCoreExpr::mul(a, b),
        "div" | "/" => FxCoreExpr::div(a, b),
        "pow" | "**" | "^" => FxCoreExpr::pow(a, b),
        "<" | "lt" => FxCoreExpr::lt(a, b),
        ">" | "gt" => FxCoreExpr::gt(a, b),
        "<=" | "le" => FxCoreExpr::le(a, b),
        ">=" | "ge" => FxCoreExpr::ge(a, b),
        "=" | "==" | "eq" => FxCoreExpr::eq(a, b),
        "!=" | "not=" | "ne" => FxCoreExpr::ne(a, b),
        "and" | "&&" => FxCoreExpr::and(a, b),
        "or" | "||" => FxCoreExpr::or(a, b),
        // 알 수 없는 이항 함수 → InteropCall로 보존
        _ => make_interop_call(func, vec![a, b]),
      })
    }

    // 다른 아리티 함수 → InteropCall로 보존
    _ => Ok(make_interop_call(func, lowered_args)),
  }
}

/// 알 수 없는 함수를 InteropCall로 변환
/// 함수명을 ConstString으로 args 앞에 추가하여 보존
fn make_interop_call(func: &str, args: Vec<FxCoreExpr>) -> FxCoreExpr {
  let mut call_args = vec![FxCoreExpr::string(func)];
  call_args.extend(args);
  FxCoreExpr::Derived {
    meta: MeaningMeta::interop(MeaningOpId::InteropCall),
    args: call_args,
  }
}

/// 중위 연산자 변환
fn lower_infix_op(
  op: &str,
  left: FxCoreExpr,
  right: FxCoreExpr,
) -> Result<FxCoreExpr, FxLoweringError> {
  Ok(match op {
    "+" => FxCoreExpr::add(left, right),
    "-" => FxCoreExpr::sub(left, right),
    "*" => FxCoreExpr::mul(left, right),
    "/" => FxCoreExpr::div(left, right),
    "%" | "mod" => FxCoreExpr::modulo(left, right),
    "**" | "^" => FxCoreExpr::pow(left, right),
    "<" => FxCoreExpr::lt(left, right),
    ">" => FxCoreExpr::gt(left, right),
    "<=" => FxCoreExpr::le(left, right),
    ">=" => FxCoreExpr::ge(left, right),
    "==" | "=" => FxCoreExpr::eq(left, right),
    "!=" | "not=" => FxCoreExpr::ne(left, right),
    "&&" | "and" => FxCoreExpr::and(left, right),
    "||" | "or" => FxCoreExpr::or(left, right),
    _ => return Err(FxLoweringError::UnknownInfixOp { op: op.to_string() }),
  })
}

/// 전위 연산자 변환
fn lower_prefix_op(op: &str, arg: FxCoreExpr) -> Result<FxCoreExpr, FxLoweringError> {
  Ok(match op {
    "-" => FxCoreExpr::neg(arg),
    "!" | "not" => FxCoreExpr::not(arg),
    _ => return Err(FxLoweringError::UnknownPrefixOp { op: op.to_string() }),
  })
}

/// 변수 치환 (Let 바인딩 처리용)
fn substitute_var(expr: &FxCoreExpr, name: &str, value: &FxCoreExpr) -> FxCoreExpr {
  match expr {
    FxCoreExpr::Var(n) if n == name => value.clone(),

    FxCoreExpr::Unary { meta, arg } => FxCoreExpr::Unary {
      meta: meta.clone(),
      arg: Box::new(substitute_var(arg, name, value)),
    },

    FxCoreExpr::Binary { meta, lhs, rhs } => FxCoreExpr::Binary {
      meta: meta.clone(),
      lhs: Box::new(substitute_var(lhs, name, value)),
      rhs: Box::new(substitute_var(rhs, name, value)),
    },

    FxCoreExpr::If { cond, then_, else_ } => FxCoreExpr::If {
      cond: Box::new(substitute_var(cond, name, value)),
      then_: Box::new(substitute_var(then_, name, value)),
      else_: Box::new(substitute_var(else_, name, value)),
    },

    FxCoreExpr::Derived { meta, args } => FxCoreExpr::Derived {
      meta: meta.clone(),
      args: args
        .iter()
        .map(|a| substitute_var(a, name, value))
        .collect(),
    },

    FxCoreExpr::List(items) => FxCoreExpr::List(
      items
        .iter()
        .map(|item| substitute_var(item, name, value))
        .collect(),
    ),

    FxCoreExpr::AttrSet(pairs) => FxCoreExpr::AttrSet(
      pairs
        .iter()
        .map(|(k, v)| (k.clone(), substitute_var(v, name, value)))
        .collect(),
    ),

    // Lambda - 바운드 변수 섀도잉 처리
    FxCoreExpr::Lambda { param, body } => {
      if param == name {
        // 바운드 변수와 동일한 이름이면 치환하지 않음 (섀도잉)
        expr.clone()
      } else {
        // 바운드 변수와 다르면 body에서 재귀적으로 치환
        FxCoreExpr::Lambda {
          param: param.clone(),
          body: Box::new(substitute_var(body, name, value)),
        }
      }
    }

    FxCoreExpr::Select { expr: e, attr } => FxCoreExpr::Select {
      expr: Box::new(substitute_var(e, name, value)),
      attr: attr.clone(),
    },

    // Pass through
    other => other.clone(),
  }
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;
  use crate::fx::FxSurfaceExpr as S;

  #[test]
  fn test_lower_const() {
    let surface = S::int(42);
    let core = lower_surface_to_core(&surface).unwrap();
    assert!(matches!(core, FxCoreExpr::ConstInt(42)));
  }

  #[test]
  fn test_lower_param_system_time() {
    // pnix style
    let surface = S::ident("param.system_time");
    let core = lower_surface_to_core(&surface).unwrap();
    assert!(matches!(core, FxCoreExpr::ParamSysTime));

    // clojure style
    let surface = S::ident("param/system-time");
    let core = lower_surface_to_core(&surface).unwrap();
    assert!(matches!(core, FxCoreExpr::ParamSysTime));

    // common style
    let surface = S::ident("__time__");
    let core = lower_surface_to_core(&surface).unwrap();
    assert!(matches!(core, FxCoreExpr::ParamSysTime));
  }

  #[test]
  fn test_lower_floor_call() {
    // floor(x)
    let surface = S::call("floor", vec![S::ident("__time__")]);
    let core = lower_surface_to_core(&surface).unwrap();

    match core {
      FxCoreExpr::Unary { arg, .. } => {
        assert!(matches!(*arg, FxCoreExpr::ParamSysTime));
      }
      _ => panic!("Expected Unary (Floor)"),
    }
  }

  #[test]
  fn test_lower_infix_mod() {
    // x % 60
    let surface = S::infix("%", S::int(123), S::int(60));
    let core = lower_surface_to_core(&surface).unwrap();

    match core {
      FxCoreExpr::Binary { lhs, rhs, .. } => {
        assert!(matches!(*lhs, FxCoreExpr::ConstInt(123)));
        assert!(matches!(*rhs, FxCoreExpr::ConstInt(60)));
      }
      _ => panic!("Expected Binary (Mod)"),
    }
  }

  #[test]
  fn test_lower_unknown_infix_op() {
    let surface = S::infix("???", S::int(1), S::int(2));
    let err = lower_surface_to_core(&surface).unwrap_err();
    assert!(matches!(err, FxLoweringError::UnknownInfixOp { .. }));
  }

  #[test]
  fn test_lower_unknown_prefix_op() {
    let surface = S::prefix("~", S::int(1));
    let err = lower_surface_to_core(&surface).unwrap_err();
    assert!(matches!(err, FxLoweringError::UnknownPrefixOp { .. }));
  }

  #[test]
  fn test_lower_seconds_pnix() {
    // floor(param.system_time) % 60
    let surface = S::infix(
      "%",
      S::call("floor", vec![S::ident("param.system_time")]),
      S::float(60.0),
    );
    let core = lower_surface_to_core(&surface).unwrap();

    // Should be Mod(Floor(ParamSysTime), ConstFloat(60))
    match core {
      FxCoreExpr::Binary { lhs, rhs, .. } => {
        match *lhs {
          FxCoreExpr::Unary { arg, .. } => {
            assert!(matches!(*arg, FxCoreExpr::ParamSysTime));
          }
          _ => panic!("Expected Floor"),
        }
        assert!(matches!(*rhs, FxCoreExpr::ConstFloat(f) if (f - 60.0).abs() < 1e-10));
      }
      _ => panic!("Expected Binary (Mod)"),
    }
  }

  #[test]
  fn test_lower_seconds_clojure() {
    // (mod (floor param/system-time) 60)
    let surface = S::call(
      "mod",
      vec![
        S::call("floor", vec![S::ident("param/system-time")]),
        S::float(60.0),
      ],
    );
    let core = lower_surface_to_core(&surface).unwrap();

    // Should be Mod(Floor(ParamSysTime), ConstFloat(60))
    match core {
      FxCoreExpr::Binary { lhs, rhs, .. } => {
        match *lhs {
          FxCoreExpr::Unary { arg, .. } => {
            assert!(matches!(*arg, FxCoreExpr::ParamSysTime));
          }
          _ => panic!("Expected Floor"),
        }
        assert!(matches!(*rhs, FxCoreExpr::ConstFloat(f) if (f - 60.0).abs() < 1e-10));
      }
      _ => panic!("Expected Binary (Mod)"),
    }
  }

  #[test]
  fn test_lower_var_reference() {
    // hours (일반 변수)
    let surface = S::ident("hours");
    let core = lower_surface_to_core(&surface).unwrap();
    assert!(matches!(core, FxCoreExpr::Var(ref name) if name == "hours"));
  }

  #[test]
  fn test_lower_arithmetic_chain() {
    // hours * 30 + minutes * 0.5
    let surface = S::infix(
      "+",
      S::infix("*", S::ident("hours"), S::float(30.0)),
      S::infix("*", S::ident("minutes"), S::float(0.5)),
    );
    let core = lower_surface_to_core(&surface).unwrap();

    match core {
      FxCoreExpr::Binary { .. } => {
        // Add at top level
      }
      _ => panic!("Expected Binary (Add)"),
    }
  }

  #[test]
  fn test_lower_pow() {
    // x ** 2
    let surface = S::infix("**", S::ident("x"), S::int(2));
    let core = lower_surface_to_core(&surface).unwrap();

    match core {
      FxCoreExpr::Binary { meta, lhs, rhs } => {
        assert_eq!(meta.op, MeaningOpId::Pow);
        assert!(matches!(*lhs, FxCoreExpr::Var(ref name) if name == "x"));
        assert!(matches!(*rhs, FxCoreExpr::ConstInt(2)));
      }
      _ => panic!("Expected Binary (Pow)"),
    }
  }

  #[test]
  fn test_lower_exp_ln() {
    // exp(x)
    let surface = S::call("exp", vec![S::ident("x")]);
    let core = lower_surface_to_core(&surface).unwrap();
    match core {
      FxCoreExpr::Unary { meta, .. } => {
        assert_eq!(meta.op, MeaningOpId::Exp);
      }
      _ => panic!("Expected Unary (Exp)"),
    }

    // ln(x)
    let surface = S::call("ln", vec![S::ident("x")]);
    let core = lower_surface_to_core(&surface).unwrap();
    match core {
      FxCoreExpr::Unary { meta, .. } => {
        assert_eq!(meta.op, MeaningOpId::Ln);
      }
      _ => panic!("Expected Unary (Ln)"),
    }
  }

  #[test]
  fn test_substitute_var() {
    // x + x → 5 + 5
    let expr = FxCoreExpr::add(FxCoreExpr::var("x"), FxCoreExpr::var("x"));
    let value = FxCoreExpr::int(5);
    let result = substitute_var(&expr, "x", &value);

    match result {
      FxCoreExpr::Binary { lhs, rhs, .. } => {
        assert!(matches!(*lhs, FxCoreExpr::ConstInt(5)));
        assert!(matches!(*rhs, FxCoreExpr::ConstInt(5)));
      }
      _ => panic!("Expected Binary"),
    }
  }

  #[test]
  fn test_substitute_lambda_shadowing() {
    // (λx. x + y) with x=10 → (λx. x + y) (not (λx. 10 + y))
    let lambda = FxCoreExpr::Lambda {
      param: "x".to_string(),
      body: Box::new(FxCoreExpr::add(FxCoreExpr::var("x"), FxCoreExpr::var("y"))),
    };
    let result = substitute_var(&lambda, "x", &FxCoreExpr::int(10));

    // Lambda with param "x" should not substitute inner x
    match result {
      FxCoreExpr::Lambda { param, body } => {
        assert_eq!(param, "x");
        // body should still have Var("x") not ConstInt(10)
        match *body {
          FxCoreExpr::Binary { lhs, .. } => {
            assert!(matches!(*lhs, FxCoreExpr::Var(ref name) if name == "x"));
          }
          _ => panic!("Expected Binary"),
        }
      }
      _ => panic!("Expected Lambda"),
    }
  }
}
