//! LaTeX 렌더링 (교육용)
//!
//! FxCoreExpr를 LaTeX 수학 표현식으로 변환

use crate::fx::{FxCoreExpr, MeaningOpId};

/// FxCoreExpr → LaTeX 문자열 변환
///
/// FxCoreExpr를 LaTeX 수학 표현식으로 변환합니다 (교육용).
///
/// ## 헌법 준수 (P0-1, C1)
///
/// 텍스트 생성만, 파일 I/O 없음
pub fn to_latex(expr: &FxCoreExpr) -> String {
  match expr {
    // ========== Literals ==========
    FxCoreExpr::ConstInt(n) => format!("{}", n),
    FxCoreExpr::ConstFloat(f) => format_float(*f),
    FxCoreExpr::ConstBool(b) => format!("\\text{{{}}}", b),
    FxCoreExpr::ConstString(s) => format!("\\text{{\"{}\"}}", escape_latex(s)),

    // ========== Parameters ==========
    FxCoreExpr::ParamSysTime => "t".to_string(),
    FxCoreExpr::ParamDeltaTime => "\\Delta t".to_string(),
    FxCoreExpr::SignalVar(id) => format!("s_{{{}}}", id.0),
    FxCoreExpr::Var(name) => escape_latex(name),

    // ========== Unary ==========
    FxCoreExpr::Unary { meta, arg } => {
      let inner = to_latex(arg);
      match meta.op {
        MeaningOpId::Neg => format!("-{}", wrap_if_complex(arg, &inner)),
        MeaningOpId::Not => format!("\\neg {}", wrap_if_complex(arg, &inner)),
        MeaningOpId::Floor => format!("\\lfloor {} \\rfloor", inner),
        MeaningOpId::Ceil => format!("\\lceil {} \\rceil", inner),
        MeaningOpId::Abs => format!("\\left| {} \\right|", inner),
        MeaningOpId::Sqrt => format!("\\sqrt{{{}}}", inner),
        MeaningOpId::Sin => format!("\\sin({})", inner),
        MeaningOpId::Cos => format!("\\cos({})", inner),
        MeaningOpId::Tan => format!("\\tan({})", inner),
        MeaningOpId::Exp => format!("e^{{{}}}", inner),
        MeaningOpId::Ln => format!("\\ln({})", inner),
        _ => format!(
          "\\text{{{}}}({})",
          format!("{:?}", meta.op).to_lowercase(),
          inner
        ),
      }
    }

    // ========== Binary ==========
    FxCoreExpr::Binary { meta, lhs, rhs } => {
      let left = to_latex(lhs);
      let right = to_latex(rhs);
      match meta.op {
        MeaningOpId::Add => format!("{} + {}", left, right),
        MeaningOpId::Sub => format!("{} - {}", left, wrap_if_add(rhs, &right)),
        MeaningOpId::Mul => {
          let l = wrap_if_additive(lhs, &left);
          let r = wrap_if_additive(rhs, &right);
          format!("{} \\cdot {}", l, r)
        }
        MeaningOpId::Div => format!("\\frac{{{}}}{{{}}}", left, right),
        MeaningOpId::Mod => format!("{} \\mod {}", left, right),
        MeaningOpId::Pow => format!("{}^{{{}}}", wrap_if_complex(lhs, &left), right),
        MeaningOpId::Lt => format!("{} < {}", left, right),
        MeaningOpId::Gt => format!("{} > {}", left, right),
        MeaningOpId::Le => format!("{} \\leq {}", left, right),
        MeaningOpId::Ge => format!("{} \\geq {}", left, right),
        MeaningOpId::Eq => format!("{} = {}", left, right),
        MeaningOpId::Ne => format!("{} \\neq {}", left, right),
        MeaningOpId::And => format!("{} \\land {}", left, right),
        MeaningOpId::Or => format!("{} \\lor {}", left, right),
        MeaningOpId::ListCons => format!("{} : {}", left, right),
        MeaningOpId::Concat => format!("{} \\mathbin{{++}} {}", left, right),
        _ => format!(
          "\\text{{{}}}({}, {})",
          format!("{:?}", meta.op).to_lowercase(),
          left,
          right
        ),
      }
    }

    // ========== Derived ==========
    FxCoreExpr::Derived { meta, args } => {
      match meta.op {
        MeaningOpId::SecondsFromTime => "\\lfloor t \\rfloor \\mod 60".to_string(),
        MeaningOpId::MinutesFromTime => "\\lfloor t / 60 \\rfloor \\mod 60".to_string(),
        MeaningOpId::HoursFromTime => "\\lfloor t / 3600 \\rfloor \\mod 12".to_string(),
        MeaningOpId::AngleFromSecond => {
          if args.is_empty() {
            "\\frac{s \\cdot 2\\pi}{60}".to_string()
          } else {
            let s = to_latex(&args[0]);
            format!("\\frac{{{} \\cdot 2\\pi}}{{60}}", s)
          }
        }
        MeaningOpId::AngleFromMinute => {
          if args.is_empty() {
            "\\frac{m \\cdot 2\\pi}{60}".to_string()
          } else {
            let m = to_latex(&args[0]);
            format!("\\frac{{{} \\cdot 2\\pi}}{{60}}", m)
          }
        }
        MeaningOpId::AngleFromHour => {
          if args.is_empty() {
            "\\frac{h \\cdot 2\\pi}{12}".to_string()
          } else {
            let h = to_latex(&args[0]);
            format!("\\frac{{{} \\cdot 2\\pi}}{{12}}", h)
          }
        }
        MeaningOpId::PositionFromAngle => {
          if args.len() >= 3 {
            let angle = to_latex(&args[0]);
            let cx = to_latex(&args[1]);
            let r = to_latex(&args[2]);
            format!(
              "({} + {} \\cdot \\cos({}), {} + {} \\cdot \\sin({}))",
              cx, r, angle, cx, r, angle
            )
          } else {
            "\\text{pos}(\\theta)".to_string()
          }
        }
        // CT operations
        MeaningOpId::CtFmap => {
          if args.len() >= 2 {
            format!(
              "{} \\langle\\$\\rangle {}",
              to_latex(&args[0]),
              to_latex(&args[1])
            )
          } else {
            "\\text{fmap}".to_string()
          }
        }
        MeaningOpId::CtPure => {
          if !args.is_empty() {
            format!("\\text{{pure}}({})", to_latex(&args[0]))
          } else {
            "\\text{pure}".to_string()
          }
        }
        MeaningOpId::CtBind => {
          if args.len() >= 2 {
            format!("{} \\gg\\!\\!= {}", to_latex(&args[0]), to_latex(&args[1]))
          } else {
            "\\text{bind}".to_string()
          }
        }
        _ => {
          let args_str: Vec<_> = args.iter().map(to_latex).collect();
          format!(
            "\\text{{{}}}({})",
            format!("{:?}", meta.op).to_lowercase(),
            args_str.join(", ")
          )
        }
      }
    }

    // ========== Control Flow ==========
    FxCoreExpr::If { cond, then_, else_ } => {
      format!(
        "\\begin{{cases}} {} & \\text{{if }} {} \\\\ {} & \\text{{otherwise}} \\end{{cases}}",
        to_latex(then_),
        to_latex(cond),
        to_latex(else_)
      )
    }

    // Y08a-11: Let - lazy semantics 보존
    FxCoreExpr::Let { name, value, body } => {
      format!(
        "\\text{{let }} {} = {} \\text{{ in }} {}",
        escape_latex(name),
        to_latex(value),
        to_latex(body)
      )
    }

    // ========== Collections ==========
    FxCoreExpr::List(items) => {
      let elems: Vec<_> = items.iter().map(to_latex).collect();
      format!("[{}]", elems.join(", "))
    }
    FxCoreExpr::AttrSet(pairs) => {
      let entries: Vec<_> = pairs
        .iter()
        .map(|(k, v)| format!("{} = {}", escape_latex(k), to_latex(v)))
        .collect();
      format!("\\{{ {} \\}}", entries.join(", "))
    }

    // ========== Lambda/Select ==========
    FxCoreExpr::Lambda { param, body } => {
      format!("\\lambda {} . {}", escape_latex(param), to_latex(body))
    }
    FxCoreExpr::Select { expr, attr } => {
      format!("{}.{}", to_latex(expr), escape_latex(attr))
    }

    // ========== Interop ==========
    FxCoreExpr::Interop { lang, code, .. } => {
      format!("\\text{{[{}]: {}}}", lang, escape_latex(code))
    }

    // ========== ADT Construct ==========
    FxCoreExpr::Construct { variant, args } => {
      if args.is_empty() {
        format!("\\text{{{}}}", escape_latex(variant))
      } else {
        let args_str: Vec<_> = args.iter().map(to_latex).collect();
        format!(
          "\\text{{{}}}({})",
          escape_latex(variant),
          args_str.join(", ")
        )
      }
    }

    // Y08b-2: Throw - 런타임 에러
    FxCoreExpr::Throw { message } => format!("\\text{{throw: {}}}", escape_latex(message)),
  }
}

/// Format float nicely
fn format_float(f: f64) -> String {
  if f == f.floor() && f.abs() < 1e10 {
    format!("{}", f as i64)
  } else if f.abs() < 0.0001 || f.abs() > 1e6 {
    format!("{:.2e}", f)
  } else {
    format!("{}", f)
  }
}

/// Escape LaTeX special characters
fn escape_latex(s: &str) -> String {
  // Order matters: escape braces first, then things that need braces
  s.replace('\\', "\\textbackslash{}")
    .replace('{', "\\{")
    .replace('}', "\\}")
    .replace('_', "\\_")
    .replace('%', "\\%")
    .replace('&', "\\&")
    .replace('#', "\\#")
    .replace('$', "\\$")
    .replace('^', "\\^{}")
    .replace('~', "\\textasciitilde{}")
}

/// Wrap expression in parentheses if it's complex
fn wrap_if_complex(expr: &FxCoreExpr, rendered: &str) -> String {
  match expr {
    FxCoreExpr::Binary { .. } | FxCoreExpr::If { .. } => format!("({})", rendered),
    _ => rendered.to_string(),
  }
}

/// Wrap in parentheses if it's an addition (for subtraction RHS)
fn wrap_if_add(expr: &FxCoreExpr, rendered: &str) -> String {
  match expr {
    FxCoreExpr::Binary { meta, .. } if meta.op == MeaningOpId::Add => {
      format!("({})", rendered)
    }
    _ => rendered.to_string(),
  }
}

/// Wrap in parentheses if additive (for multiplication)
fn wrap_if_additive(expr: &FxCoreExpr, rendered: &str) -> String {
  match expr {
    FxCoreExpr::Binary { meta, .. }
      if meta.op == MeaningOpId::Add || meta.op == MeaningOpId::Sub =>
    {
      format!("({})", rendered)
    }
    _ => rendered.to_string(),
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
    assert_eq!(to_latex(&FxCoreExpr::int(42)), "42");
    assert_eq!(to_latex(&FxCoreExpr::float(2.71)), "2.71");
    assert_eq!(to_latex(&FxCoreExpr::bool(true)), "\\text{true}");
  }

  #[test]
  fn test_time_params() {
    assert_eq!(to_latex(&FxCoreExpr::time()), "t");
    assert_eq!(to_latex(&FxCoreExpr::dt()), "\\Delta t");
  }

  #[test]
  fn test_arithmetic() {
    let expr = FxCoreExpr::add(FxCoreExpr::var("x"), FxCoreExpr::int(1));
    assert_eq!(to_latex(&expr), "x + 1");

    let expr = FxCoreExpr::mul(FxCoreExpr::var("x"), FxCoreExpr::var("y"));
    assert_eq!(to_latex(&expr), "x \\cdot y");

    let expr = FxCoreExpr::div(FxCoreExpr::var("a"), FxCoreExpr::var("b"));
    assert_eq!(to_latex(&expr), "\\frac{a}{b}");
  }

  #[test]
  fn test_unary() {
    let expr = FxCoreExpr::sin(FxCoreExpr::var("x"));
    assert_eq!(to_latex(&expr), "\\sin(x)");

    let expr = FxCoreExpr::sqrt(FxCoreExpr::var("x"));
    assert_eq!(to_latex(&expr), "\\sqrt{x}");

    let expr = FxCoreExpr::floor(FxCoreExpr::var("x"));
    assert_eq!(to_latex(&expr), "\\lfloor x \\rfloor");
  }

  #[test]
  fn test_seconds_from_time() {
    let expr = FxCoreExpr::seconds_from_time();
    let latex = to_latex(&expr);
    assert!(latex.contains("\\lfloor t \\rfloor"));
    assert!(latex.contains("60"));
  }

  #[test]
  fn test_conditional() {
    let expr = FxCoreExpr::if_then_else(
      FxCoreExpr::lt(FxCoreExpr::var("x"), FxCoreExpr::int(0)),
      FxCoreExpr::neg(FxCoreExpr::var("x")),
      FxCoreExpr::var("x"),
    );
    let latex = to_latex(&expr);
    assert!(latex.contains("\\begin{cases}"));
    assert!(latex.contains("\\text{if }"));
  }

  #[test]
  fn test_lambda() {
    let expr = FxCoreExpr::Lambda {
      param: "x".to_string(),
      body: Box::new(FxCoreExpr::mul(FxCoreExpr::var("x"), FxCoreExpr::int(2))),
    };
    let latex = to_latex(&expr);
    assert!(latex.contains("\\lambda x"));
  }

  #[test]
  fn test_escape_latex() {
    assert_eq!(escape_latex("x_1"), "x\\_1");
    assert_eq!(escape_latex("a^2"), "a\\^{}2");
    assert_eq!(escape_latex("100%"), "100\\%");
  }

  #[test]
  fn test_format_float() {
    assert_eq!(format_float(42.0), "42");
    assert_eq!(format_float(2.71), "2.71");
  }
}
