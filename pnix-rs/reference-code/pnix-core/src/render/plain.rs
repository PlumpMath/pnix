//! Plain text 렌더링 (디버그용)
//!
//! FxCoreExpr를 읽기 쉬운 텍스트로 변환

use crate::fx::{FxCoreExpr, MeaningOpId};

/// FxCoreExpr → Plain text 문자열 변환
///
/// FxCoreExpr를 읽기 쉬운 텍스트로 변환합니다 (디버그용).
///
/// ## 헌법 준수 (P0-1, C1)
///
/// 텍스트 생성만, 파일 I/O 없음
pub fn to_plain(expr: &FxCoreExpr) -> String {
  match expr {
    // ========== Literals ==========
    FxCoreExpr::ConstInt(n) => format!("{}", n),
    FxCoreExpr::ConstFloat(f) => format_float(*f),
    FxCoreExpr::ConstBool(b) => format!("{}", b),
    FxCoreExpr::ConstString(s) => format!("\"{}\"", s),

    // ========== Parameters ==========
    FxCoreExpr::ParamSysTime => "t".to_string(),
    FxCoreExpr::ParamDeltaTime => "dt".to_string(),
    FxCoreExpr::SignalVar(id) => format!("signal[{}]", id.0),
    FxCoreExpr::Var(name) => name.clone(),

    // ========== Unary ==========
    FxCoreExpr::Unary { meta, arg } => {
      let inner = to_plain(arg);
      match meta.op {
        MeaningOpId::Neg => format!("(-{})", inner),
        MeaningOpId::Not => format!("(!{})", inner),
        MeaningOpId::Floor => format!("floor({})", inner),
        MeaningOpId::Ceil => format!("ceil({})", inner),
        MeaningOpId::Abs => format!("abs({})", inner),
        MeaningOpId::Sqrt => format!("sqrt({})", inner),
        MeaningOpId::Sin => format!("sin({})", inner),
        MeaningOpId::Cos => format!("cos({})", inner),
        MeaningOpId::Tan => format!("tan({})", inner),
        MeaningOpId::Exp => format!("exp({})", inner),
        MeaningOpId::Ln => format!("ln({})", inner),
        _ => format!("{}({})", format!("{:?}", meta.op).to_lowercase(), inner),
      }
    }

    // ========== Binary ==========
    FxCoreExpr::Binary { meta, lhs, rhs } => {
      let left = to_plain(lhs);
      let right = to_plain(rhs);
      match meta.op {
        MeaningOpId::Add => format!("({} + {})", left, right),
        MeaningOpId::Sub => format!("({} - {})", left, right),
        MeaningOpId::Mul => format!("({} * {})", left, right),
        MeaningOpId::Div => format!("({} / {})", left, right),
        MeaningOpId::Mod => format!("({} % {})", left, right),
        MeaningOpId::Pow => format!("({}^{})", left, right),
        MeaningOpId::Lt => format!("({} < {})", left, right),
        MeaningOpId::Gt => format!("({} > {})", left, right),
        MeaningOpId::Le => format!("({} <= {})", left, right),
        MeaningOpId::Ge => format!("({} >= {})", left, right),
        MeaningOpId::Eq => format!("({} == {})", left, right),
        MeaningOpId::Ne => format!("({} != {})", left, right),
        MeaningOpId::And => format!("({} && {})", left, right),
        MeaningOpId::Or => format!("({} || {})", left, right),
        MeaningOpId::ListCons => format!("({} : {})", left, right),
        MeaningOpId::Concat => format!("({} ++ {})", left, right),
        _ => format!(
          "{}({}, {})",
          format!("{:?}", meta.op).to_lowercase(),
          left,
          right
        ),
      }
    }

    // ========== Derived ==========
    FxCoreExpr::Derived { meta, args } => {
      match meta.op {
        MeaningOpId::SecondsFromTime => "seconds(t)".to_string(),
        MeaningOpId::MinutesFromTime => "minutes(t)".to_string(),
        MeaningOpId::HoursFromTime => "hours(t)".to_string(),
        MeaningOpId::AngleFromSecond => {
          if args.is_empty() {
            "angleFromSecond(s)".to_string()
          } else {
            format!("angleFromSecond({})", to_plain(&args[0]))
          }
        }
        MeaningOpId::AngleFromMinute => {
          if args.is_empty() {
            "angleFromMinute(m)".to_string()
          } else {
            format!("angleFromMinute({})", to_plain(&args[0]))
          }
        }
        MeaningOpId::AngleFromHour => {
          if args.is_empty() {
            "angleFromHour(h)".to_string()
          } else {
            format!("angleFromHour({})", to_plain(&args[0]))
          }
        }
        MeaningOpId::PositionFromAngle => {
          if args.len() >= 3 {
            format!(
              "posFromAngle({}, {}, {})",
              to_plain(&args[0]),
              to_plain(&args[1]),
              to_plain(&args[2])
            )
          } else {
            "posFromAngle(θ)".to_string()
          }
        }
        // CT operations
        MeaningOpId::CtFmap => {
          if args.len() >= 2 {
            format!("fmap({}, {})", to_plain(&args[0]), to_plain(&args[1]))
          } else {
            "fmap".to_string()
          }
        }
        MeaningOpId::CtPure => {
          if !args.is_empty() {
            format!("pure({})", to_plain(&args[0]))
          } else {
            "pure".to_string()
          }
        }
        MeaningOpId::CtBind => {
          if args.len() >= 2 {
            format!("bind({}, {})", to_plain(&args[0]), to_plain(&args[1]))
          } else {
            "bind".to_string()
          }
        }
        _ => {
          let args_str: Vec<_> = args.iter().map(to_plain).collect();
          format!(
            "{}({})",
            format!("{:?}", meta.op).to_lowercase(),
            args_str.join(", ")
          )
        }
      }
    }

    // ========== Control Flow ==========
    FxCoreExpr::If { cond, then_, else_ } => {
      format!(
        "if {} then {} else {}",
        to_plain(cond),
        to_plain(then_),
        to_plain(else_)
      )
    }

    // Y08a-11: Let - lazy semantics 보존
    FxCoreExpr::Let { name, value, body } => {
      format!("let {} = {} in {}", name, to_plain(value), to_plain(body))
    }

    // ========== Collections ==========
    FxCoreExpr::List(items) => {
      let elems: Vec<_> = items.iter().map(to_plain).collect();
      format!("[{}]", elems.join(", "))
    }
    FxCoreExpr::AttrSet(pairs) => {
      let entries: Vec<_> = pairs
        .iter()
        .map(|(k, v)| format!("{} = {}", k, to_plain(v)))
        .collect();
      format!("{{ {} }}", entries.join("; "))
    }

    // ========== Lambda/Select ==========
    FxCoreExpr::Lambda { param, body } => {
      format!("({}: {})", param, to_plain(body))
    }
    FxCoreExpr::Select { expr, attr } => {
      format!("{}.{}", to_plain(expr), attr)
    }

    // ========== Interop ==========
    FxCoreExpr::Interop { lang, code, .. } => {
      format!("[{}| {} ]", lang, code)
    }

    // ========== ADT Construct ==========
    FxCoreExpr::Construct { variant, args } => {
      if args.is_empty() {
        variant.clone()
      } else {
        let args_str: Vec<_> = args.iter().map(to_plain).collect();
        format!("{}({})", variant, args_str.join(", "))
      }
    }

    // Y08b-2: Throw - 런타임 에러
    FxCoreExpr::Throw { message } => format!("throw: {}", message),
  }
}

/// Format float nicely
fn format_float(f: f64) -> String {
  if f == f.floor() && f.abs() < 1e10 {
    format!("{}.0", f as i64)
  } else if f.abs() < 0.0001 || f.abs() > 1e6 {
    format!("{:.2e}", f)
  } else {
    format!("{}", f)
  }
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_literals() {
    assert_eq!(to_plain(&FxCoreExpr::int(42)), "42");
    assert_eq!(to_plain(&FxCoreExpr::float(2.71)), "2.71");
    assert_eq!(to_plain(&FxCoreExpr::bool(true)), "true");
    assert_eq!(to_plain(&FxCoreExpr::string("hello")), "\"hello\"");
  }

  #[test]
  fn test_time_params() {
    assert_eq!(to_plain(&FxCoreExpr::time()), "t");
    assert_eq!(to_plain(&FxCoreExpr::dt()), "dt");
  }

  #[test]
  fn test_arithmetic() {
    let expr = FxCoreExpr::add(FxCoreExpr::var("x"), FxCoreExpr::int(1));
    assert_eq!(to_plain(&expr), "(x + 1)");

    let expr = FxCoreExpr::mul(FxCoreExpr::var("x"), FxCoreExpr::var("y"));
    assert_eq!(to_plain(&expr), "(x * y)");

    let expr = FxCoreExpr::div(FxCoreExpr::var("a"), FxCoreExpr::var("b"));
    assert_eq!(to_plain(&expr), "(a / b)");
  }

  #[test]
  fn test_unary() {
    let expr = FxCoreExpr::sin(FxCoreExpr::var("x"));
    assert_eq!(to_plain(&expr), "sin(x)");

    let expr = FxCoreExpr::sqrt(FxCoreExpr::var("x"));
    assert_eq!(to_plain(&expr), "sqrt(x)");

    let expr = FxCoreExpr::floor(FxCoreExpr::var("x"));
    assert_eq!(to_plain(&expr), "floor(x)");
  }

  #[test]
  fn test_conditional() {
    let expr = FxCoreExpr::if_then_else(
      FxCoreExpr::lt(FxCoreExpr::var("x"), FxCoreExpr::int(0)),
      FxCoreExpr::neg(FxCoreExpr::var("x")),
      FxCoreExpr::var("x"),
    );
    let plain = to_plain(&expr);
    assert!(plain.contains("if"));
    assert!(plain.contains("then"));
    assert!(plain.contains("else"));
  }

  #[test]
  fn test_lambda() {
    let expr = FxCoreExpr::Lambda {
      param: "x".to_string(),
      body: Box::new(FxCoreExpr::mul(FxCoreExpr::var("x"), FxCoreExpr::int(2))),
    };
    let plain = to_plain(&expr);
    assert!(plain.contains("x:"));
  }

  #[test]
  fn test_list() {
    let expr = FxCoreExpr::List(vec![
      FxCoreExpr::int(1),
      FxCoreExpr::int(2),
      FxCoreExpr::int(3),
    ]);
    assert_eq!(to_plain(&expr), "[1, 2, 3]");
  }

  #[test]
  fn test_format_float() {
    assert_eq!(format_float(42.0), "42.0");
    assert_eq!(format_float(2.71), "2.71");
  }
}
