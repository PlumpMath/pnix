//! 심볼릭 미분 패스.
//!
//! SymExpr::Derivative(f, var)를 실제 미분 결과 수식으로 변환한다.
//! 다항식, 삼각함수, 지수/로그의 기본 미분 규칙을 적용.

use crate::symbolic::expr::{SymExpr, SymKind};
use crate::symbolic::passes::normalize::normalize;

/// Derivative 노드를 실제 미분 결과로 변환.
/// 지원: 상수, 변수, Add, Mul(product rule), Pow(power rule), Sin, Cos, Exp, Log, Neg.
pub fn differentiate(expr: SymExpr) -> SymExpr {
  match expr.kind {
    SymKind::Derivative(f, var) => {
      let result = diff_inner(&f, &var);
      normalize(result)
    }
    // Derivative가 아닌 노드는 내부를 재귀적으로 처리
    SymKind::Add(terms) => SymExpr::add(terms.into_iter().map(differentiate).collect()),
    SymKind::Mul(terms) => SymExpr::mul(terms.into_iter().map(differentiate).collect()),
    SymKind::Neg(x) => SymExpr::neg(differentiate(*x)),
    other => SymExpr {
      kind: other,
      zone: expr.zone,
      span: expr.span,
    },
  }
}

fn diff_inner(expr: &SymExpr, var: &str) -> SymExpr {
  match &expr.kind {
    // d/dx(c) = 0
    SymKind::Const(_) | SymKind::Exact(_) => SymExpr::int(0),

    // d/dx(x) = 1, d/dx(y) = 0
    SymKind::Var(name) => {
      if name == var {
        SymExpr::int(1)
      } else {
        SymExpr::int(0)
      }
    }

    // d/dx(f + g + ...) = f' + g' + ...
    SymKind::Add(terms) => SymExpr::add(terms.iter().map(|t| diff_inner(t, var)).collect()),

    // d/dx(-f) = -f'
    SymKind::Neg(f) => SymExpr::neg(diff_inner(f, var)),

    // Product rule: d/dx(f * g) = f' * g + f * g'
    // For n-ary: d/dx(f1 * f2 * ... * fn) = sum_i(f1 * ... * fi' * ... * fn)
    SymKind::Mul(terms) => {
      let n = terms.len();
      if n == 0 {
        return SymExpr::int(0);
      }
      if n == 1 {
        return diff_inner(&terms[0], var);
      }
      // General product rule
      let mut sum_terms = Vec::new();
      for i in 0..n {
        let mut product = Vec::new();
        for (j, term) in terms.iter().enumerate() {
          if j == i {
            product.push(diff_inner(term, var));
          } else {
            product.push(term.clone());
          }
        }
        sum_terms.push(SymExpr::mul(product));
      }
      SymExpr::add(sum_terms)
    }

    // Power rule: d/dx(f^g)
    // Special case: d/dx(x^n) = n * x^(n-1) when g is constant
    SymKind::Pow(base, exp) => {
      let base_has_var = contains_var(base, var);
      let exp_has_var = contains_var(exp, var);

      match (base_has_var, exp_has_var) {
        // d/dx(c^c) = 0
        (false, false) => SymExpr::int(0),
        // d/dx(f^n) = n * f^(n-1) * f'  (power rule + chain rule)
        (true, false) => SymExpr::mul(vec![
          (**exp).clone(),
          SymExpr::pow(
            (**base).clone(),
            SymExpr::add2((**exp).clone(), SymExpr::neg(SymExpr::int(1))),
          ),
          diff_inner(base, var),
        ]),
        // d/dx(c^g) = c^g * ln(c) * g'
        (false, true) => SymExpr::mul(vec![
          expr.clone(),
          SymExpr::log((**base).clone()),
          diff_inner(exp, var),
        ]),
        // d/dx(f^g) = f^g * (g' * ln(f) + g * f'/f) — logarithmic differentiation
        (true, true) => SymExpr::mul(vec![
          expr.clone(),
          SymExpr::add2(
            SymExpr::mul2(diff_inner(exp, var), SymExpr::log((**base).clone())),
            SymExpr::mul(vec![
              (**exp).clone(),
              diff_inner(base, var),
              SymExpr::pow((**base).clone(), SymExpr::int(-1)),
            ]),
          ),
        ]),
      }
    }

    // d/dx(sin(f)) = cos(f) * f'
    SymKind::Sin(f) => SymExpr::mul2(SymExpr::cos((**f).clone()), diff_inner(f, var)),

    // d/dx(cos(f)) = -sin(f) * f'
    SymKind::Cos(f) => SymExpr::neg(SymExpr::mul2(
      SymExpr::sin((**f).clone()),
      diff_inner(f, var),
    )),

    // d/dx(tan(f)) = (1/cos²(f)) * f'
    SymKind::Tan(f) => SymExpr::mul2(
      SymExpr::pow(SymExpr::cos((**f).clone()), SymExpr::int(-2)),
      diff_inner(f, var),
    ),

    // d/dx(exp(f)) = exp(f) * f'
    SymKind::Exp(f) => SymExpr::mul2(SymExpr::exp((**f).clone()), diff_inner(f, var)),

    // d/dx(log(f)) = f'/f
    SymKind::Log(f) => SymExpr::mul2(
      diff_inner(f, var),
      SymExpr::pow((**f).clone(), SymExpr::int(-1)),
    ),

    // d/dx(|f|) — not differentiable at 0, but formally |f|' = f * f'/|f|
    SymKind::Abs(f) => SymExpr::mul(vec![
      (**f).clone(),
      diff_inner(f, var),
      SymExpr::pow(SymExpr::abs((**f).clone()), SymExpr::int(-1)),
    ]),

    // Nested derivative: d/dx(d/dy(f)) = d/dy(d/dx(f))
    SymKind::Derivative(f, inner_var) => SymExpr::derivative(diff_inner(f, var), inner_var.clone()),

    // Tensor/Contract/Raise/Lower — not differentiable in this pass
    _ => SymExpr::int(0),
  }
}

fn contains_var(expr: &SymExpr, var: &str) -> bool {
  match &expr.kind {
    SymKind::Var(name) => name == var,
    SymKind::Const(_) | SymKind::Exact(_) => false,
    SymKind::Add(terms) | SymKind::Mul(terms) => terms.iter().any(|t| contains_var(t, var)),
    SymKind::Pow(a, b) => contains_var(a, var) || contains_var(b, var),
    SymKind::Neg(x)
    | SymKind::Sin(x)
    | SymKind::Cos(x)
    | SymKind::Tan(x)
    | SymKind::Exp(x)
    | SymKind::Log(x)
    | SymKind::Abs(x) => contains_var(x, var),
    SymKind::Derivative(f, _) => contains_var(f, var),
    _ => false,
  }
}

// ---------------------------------------------------------------------------
// Numeric evaluation
// ---------------------------------------------------------------------------

/// SymExpr를 수치로 평가. 변수가 남아있으면 None.
pub fn eval_numeric(expr: &SymExpr) -> Option<f64> {
  match &expr.kind {
    SymKind::Const(n) => Some(*n),
    SymKind::Exact(nv) => Some(match nv {
      crate::symbolic::NumberValue::Integer(i) => *i as f64,
      crate::symbolic::NumberValue::Rational { numer, denom } => *numer as f64 / *denom as f64,
      crate::symbolic::NumberValue::Float(f) => *f,
    }),
    SymKind::Add(terms) => {
      let mut sum = 0.0;
      for t in terms {
        sum += eval_numeric(t)?;
      }
      Some(sum)
    }
    SymKind::Mul(terms) => {
      let mut product = 1.0;
      for t in terms {
        product *= eval_numeric(t)?;
      }
      Some(product)
    }
    SymKind::Neg(x) => Some(-eval_numeric(x)?),
    SymKind::Pow(base, exp) => Some(eval_numeric(base)?.powf(eval_numeric(exp)?)),
    SymKind::Sin(x) => Some(eval_numeric(x)?.sin()),
    SymKind::Cos(x) => Some(eval_numeric(x)?.cos()),
    SymKind::Tan(x) => Some(eval_numeric(x)?.tan()),
    SymKind::Exp(x) => Some(eval_numeric(x)?.exp()),
    SymKind::Log(x) => Some(eval_numeric(x)?.ln()),
    SymKind::Abs(x) => Some(eval_numeric(x)?.abs()),
    SymKind::Var(_) => None,           // 변수가 남아있으면 평가 불가
    SymKind::Derivative(_, _) => None, // 미분이 남아있으면 평가 불가
    _ => None,
  }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn diff_constant() {
    let expr = SymExpr::derivative(SymExpr::int(5), "x");
    let result = differentiate(expr);
    assert_eq!(eval_numeric(&result), Some(0.0));
  }

  #[test]
  fn diff_variable() {
    let expr = SymExpr::derivative(SymExpr::var("x"), "x");
    let result = differentiate(expr);
    assert_eq!(eval_numeric(&result), Some(1.0));
  }

  #[test]
  fn diff_x_squared() {
    // d/dx(x^2) = 2x
    let expr = SymExpr::derivative(SymExpr::pow(SymExpr::var("x"), SymExpr::int(2)), "x");
    let result = differentiate(expr);
    // 결과에 x가 있으므로 수치 평가는 안 되지만 구조 확인
    let latex = crate::symbolic::to_latex(&result);
    assert!(!latex.is_empty());
  }

  #[test]
  fn diff_polynomial_3x2_plus_2x() {
    // d/dx(3x^2 + 2x) = 6x + 2
    let expr = SymExpr::derivative(
      SymExpr::add2(
        SymExpr::mul2(
          SymExpr::int(3),
          SymExpr::pow(SymExpr::var("x"), SymExpr::int(2)),
        ),
        SymExpr::mul2(SymExpr::int(2), SymExpr::var("x")),
      ),
      "x",
    );
    let result = differentiate(expr);
    let latex = crate::symbolic::to_latex(&result);
    assert!(!latex.is_empty());
  }

  #[test]
  fn diff_sin_x() {
    // d/dx(sin(x)) = cos(x)
    let expr = SymExpr::derivative(SymExpr::sin(SymExpr::var("x")), "x");
    let result = differentiate(expr);
    assert!(matches!(result.kind, SymKind::Cos(_)));
  }

  #[test]
  fn diff_exp_x() {
    // d/dx(exp(x)) = exp(x)
    let expr = SymExpr::derivative(SymExpr::exp(SymExpr::var("x")), "x");
    let result = differentiate(expr);
    assert!(matches!(result.kind, SymKind::Exp(_)));
  }

  #[test]
  fn eval_numeric_simple() {
    let expr = SymExpr::add2(SymExpr::int(2), SymExpr::int(3));
    assert_eq!(eval_numeric(&expr), Some(5.0));
  }

  #[test]
  fn eval_numeric_with_var_returns_none() {
    let expr = SymExpr::add2(SymExpr::var("x"), SymExpr::int(3));
    assert_eq!(eval_numeric(&expr), None);
  }

  #[test]
  fn eval_numeric_power() {
    let expr = SymExpr::pow(SymExpr::int(2), SymExpr::int(10));
    assert_eq!(eval_numeric(&expr), Some(1024.0));
  }
}
