//! LaTeX 렌더링 (교육용)
//!
//! SymExpr → LaTeX 변환
//!
//! ## 헌법 준수 (C1)
//!
//! 텍스트 생성만, 실행 없음

use super::expr::{IndexPosition, SymExpr, SymKind, TensorSymbol};

/// SymExpr → LaTeX 문자열
pub fn to_latex(expr: &SymExpr) -> String {
  match &expr.kind {
    SymKind::Var(name) => escape_latex(name),
    SymKind::Const(c) => format_float(*c),
    SymKind::Exact(n) => format!("{}", n),
    SymKind::Add(xs) => xs.iter().map(to_latex).collect::<Vec<_>>().join(" + "),
    SymKind::Mul(xs) => {
      xs.iter()
        .map(|e| {
          // 덧셈은 괄호로 감싸기
          if matches!(e.kind, SymKind::Add(_)) {
            format!("({})", to_latex(e))
          } else {
            to_latex(e)
          }
        })
        .collect::<Vec<_>>()
        .join(" \\cdot ")
    }
    SymKind::Pow(base, exp) => {
      let base_str = if needs_parens_for_base(base) {
        format!("({})", to_latex(base))
      } else {
        to_latex(base)
      };
      format!("{}^{{{}}}", base_str, to_latex(exp))
    }
    SymKind::Neg(x) => {
      if needs_parens_for_neg(x) {
        format!("-({})", to_latex(x))
      } else {
        format!("-{}", to_latex(x))
      }
    }
    SymKind::Sin(x) => format!("\\sin({})", to_latex(x)),
    SymKind::Cos(x) => format!("\\cos({})", to_latex(x)),
    SymKind::Tan(x) => format!("\\tan({})", to_latex(x)),
    SymKind::Exp(x) => format!("e^{{{}}}", to_latex(x)),
    SymKind::Log(x) => format!("\\ln({})", to_latex(x)),
    SymKind::Abs(x) => format!("\\left|{}\\right|", to_latex(x)),
    SymKind::Derivative(inner, var) => {
      format!("\\frac{{d}}{{d{}}} {}", escape_latex(var), to_latex(inner))
    }
    SymKind::Tensor(t) => tensor_to_latex(t),
    SymKind::Contract(e, idx1, idx2) => {
      format!(
        "\\text{{contract}}_{{{}{}}}({})",
        escape_latex(idx1),
        escape_latex(idx2),
        to_latex(e)
      )
    }
    SymKind::Raise(inner, idx) => {
      format!(
        "\\text{{raise}}_{{{}}}({})",
        escape_latex(idx),
        to_latex(inner)
      )
    }
    SymKind::Lower(inner, idx) => {
      format!(
        "\\text{{lower}}_{{{}}}({})",
        escape_latex(idx),
        to_latex(inner)
      )
    }
  }
}

/// TensorSymbol → LaTeX
fn tensor_to_latex(t: &TensorSymbol) -> String {
  let up_indices: Vec<_> = t
    .indices
    .iter()
    .filter(|i| i.position == IndexPosition::Upper)
    .map(|i| i.name.clone())
    .collect();
  let down_indices: Vec<_> = t
    .indices
    .iter()
    .filter(|i| i.position == IndexPosition::Lower)
    .map(|i| i.name.clone())
    .collect();

  let mut result = escape_latex(&t.name);
  if !up_indices.is_empty() {
    result.push_str(&format!("^{{{}}}", up_indices.join("")));
  }
  if !down_indices.is_empty() {
    result.push_str(&format!("_{{{}}}", down_indices.join("")));
  }
  result
}

/// 기본 LaTeX 이스케이프
fn escape_latex(s: &str) -> String {
  s.replace('_', "\\_")
    .replace('^', "\\^{}")
    .replace('%', "\\%")
    .replace('&', "\\&")
    .replace('#', "\\#")
    .replace('$', "\\$")
}

/// Float 포맷
fn format_float(f: f64) -> String {
  if f == f.floor() && f.abs() < 1e10 {
    format!("{}", f as i64)
  } else if f.abs() < 0.0001 || f.abs() > 1e6 {
    format!("{:.2e}", f)
  } else {
    format!("{}", f)
  }
}

/// 거듭제곱 밑에 괄호 필요한지
fn needs_parens_for_base(expr: &SymExpr) -> bool {
  matches!(
    expr.kind,
    SymKind::Add(_) | SymKind::Mul(_) | SymKind::Neg(_)
  )
}

/// 부정에 괄호 필요한지
fn needs_parens_for_neg(expr: &SymExpr) -> bool {
  matches!(expr.kind, SymKind::Add(_) | SymKind::Mul(_))
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::approx_constant)]
mod tests {
  use super::*;

  #[test]
  fn test_var() {
    let x = SymExpr::var("x");
    assert_eq!(to_latex(&x), "x");
  }

  #[test]
  fn test_const() {
    let c = SymExpr::constant(3.14);
    assert_eq!(to_latex(&c), "3.14");

    let c = SymExpr::constant(42.0);
    assert_eq!(to_latex(&c), "42");
  }

  #[test]
  fn test_add() {
    let sum = SymExpr::add2(SymExpr::var("x"), SymExpr::var("y"));
    assert_eq!(to_latex(&sum), "x + y");
  }

  #[test]
  fn test_mul() {
    let prod = SymExpr::mul2(SymExpr::var("x"), SymExpr::var("y"));
    assert_eq!(to_latex(&prod), "x \\cdot y");
  }

  #[test]
  fn test_mul_add_parens() {
    // x * (a + b)
    let expr = SymExpr::mul2(
      SymExpr::var("x"),
      SymExpr::add2(SymExpr::var("a"), SymExpr::var("b")),
    );
    assert_eq!(to_latex(&expr), "x \\cdot (a + b)");
  }

  #[test]
  fn test_pow() {
    let p = SymExpr::pow(SymExpr::var("x"), SymExpr::int(2));
    assert_eq!(to_latex(&p), "x^{2}");
  }

  #[test]
  fn test_neg() {
    let neg = SymExpr::neg(SymExpr::var("x"));
    assert_eq!(to_latex(&neg), "-x");
  }

  #[test]
  fn test_sin_cos() {
    let s = SymExpr::sin(SymExpr::var("x"));
    assert_eq!(to_latex(&s), "\\sin(x)");

    let c = SymExpr::cos(SymExpr::var("x"));
    assert_eq!(to_latex(&c), "\\cos(x)");
  }

  #[test]
  fn test_exp_log() {
    let e = SymExpr::exp(SymExpr::var("x"));
    assert_eq!(to_latex(&e), "e^{x}");

    let l = SymExpr::log(SymExpr::var("x"));
    assert_eq!(to_latex(&l), "\\ln(x)");
  }

  #[test]
  fn test_abs() {
    let a = SymExpr::abs(SymExpr::var("x"));
    assert_eq!(to_latex(&a), "\\left|x\\right|");
  }

  #[test]
  fn test_derivative() {
    let d = SymExpr::derivative(SymExpr::var("f"), "x");
    assert_eq!(to_latex(&d), "\\frac{d}{dx} f");
  }

  #[test]
  fn test_tensor() {
    use super::super::expr::{TensorIndex, TensorSymbol};

    let t = TensorSymbol {
      name: "R".to_string(),
      indices: vec![
        TensorIndex::new("a", IndexPosition::Upper),
        TensorIndex::new("b", IndexPosition::Lower),
      ],
      symmetries: vec![],
    };
    let expr = SymExpr::tensor(t);
    assert_eq!(to_latex(&expr), "R^{a}_{b}");
  }

  #[test]
  fn test_complex_expr() {
    // sin(x)^2 + cos(x)^2
    let expr = SymExpr::add2(
      SymExpr::pow(SymExpr::sin(SymExpr::var("x")), SymExpr::int(2)),
      SymExpr::pow(SymExpr::cos(SymExpr::var("x")), SymExpr::int(2)),
    );
    let latex = to_latex(&expr);
    assert!(latex.contains("\\sin"));
    assert!(latex.contains("\\cos"));
    assert!(latex.contains("^{2}"));
  }
}
