//! 직렬화 유틸리티
//!
//! pnix-old의 symbolic_core/src/ast/serialize.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 순수 변환 함수만, I/O 없음
//!
//! ## 사용 예
//!
//! ```ignore
//! use pnix_core::symbolic::serialize::{to_json, from_json, to_json_pretty};
//!
//! let expr = SymExpr::add(vec![SymExpr::var("x"), SymExpr::constant(1.0)]);
//!
//! // JSON 직렬화
//! let json = to_json(&expr)?;
//! println!("{}", json);
//!
//! // JSON 역직렬화
//! let restored: SymExpr = from_json(&json)?;
//! ```

use super::ct::CtTag;
use super::expr::{SymExpr, SymKind, Zone};
use serde_json;
use thiserror::Error;

/// JSON 직렬화 에러
///
/// # Example
/// ```rust
/// use pnix_core::symbolic::expr::SymExpr;
/// use pnix_core::symbolic::serialize::{to_json, SerializeError};
/// let expr = SymExpr::constant(1.0);
/// let result: Result<String, SerializeError> = to_json(&expr);
/// assert!(result.is_ok());
/// ```
#[derive(Debug, Error)]
pub enum SerializeError {
  #[error("JSON error: {0}")]
  Json(#[from] serde_json::Error),
}

/// SymExpr을 JSON 문자열로 직렬화 (compact)
pub fn to_json(expr: &SymExpr) -> Result<String, SerializeError> {
  Ok(serde_json::to_string(expr)?)
}

/// SymExpr을 JSON 문자열로 직렬화 (pretty print)
pub fn to_json_pretty(expr: &SymExpr) -> Result<String, SerializeError> {
  Ok(serde_json::to_string_pretty(expr)?)
}

/// JSON 문자열을 SymExpr로 역직렬화
pub fn from_json(json: &str) -> Result<SymExpr, SerializeError> {
  Ok(serde_json::from_str(json)?)
}

/// SymExpr을 serde_json::Value로 변환
pub fn to_json_value(expr: &SymExpr) -> Result<serde_json::Value, SerializeError> {
  Ok(serde_json::to_value(expr)?)
}

/// serde_json::Value를 SymExpr로 변환
pub fn from_json_value(value: serde_json::Value) -> Result<SymExpr, SerializeError> {
  Ok(serde_json::from_value(value)?)
}

// ─────────────────────────────────────────────
// LSP/로그용 간략 표현
// ─────────────────────────────────────────────

/// LSP hover 등에 사용할 간략한 타입 정보 문자열
///
/// 예: "Add(3 terms)", "Var(x)", "Tensor(g, rank=2)"
pub fn type_summary(expr: &SymExpr) -> String {
  match &expr.kind {
    SymKind::Var(name) => format!("Var({})", name),
    SymKind::Const(v) => {
      if v.fract() == 0.0 && v.abs() < 1e10 {
        format!("Const({})", *v as i64)
      } else {
        format!("Const({:.4})", v)
      }
    }
    SymKind::Exact(n) => format!("Exact({})", n),
    SymKind::Add(xs) => format!("Add({} terms)", xs.len()),
    SymKind::Mul(xs) => format!("Mul({} factors)", xs.len()),
    SymKind::Pow(_, _) => "Pow".to_string(),
    SymKind::Neg(_) => "Neg".to_string(),
    SymKind::Sin(_) => "Sin".to_string(),
    SymKind::Cos(_) => "Cos".to_string(),
    SymKind::Tan(_) => "Tan".to_string(),
    SymKind::Exp(_) => "Exp".to_string(),
    SymKind::Log(_) => "Log".to_string(),
    SymKind::Abs(_) => "Abs".to_string(),
    SymKind::Derivative(_, var) => format!("Derivative(d/d{})", var),
    SymKind::Tensor(t) => format!("Tensor({}, rank={})", t.name, t.indices.len()),
    SymKind::Contract(_, i1, i2) => format!("Contract({}, {})", i1, i2),
    SymKind::Raise(_, idx) => format!("Raise({})", idx),
    SymKind::Lower(_, idx) => format!("Lower({})", idx),
  }
}

/// 전체 표현식의 타입 정보 (재귀적 요약)
///
/// 예: "Add(Var(x), Const(1), Mul(2 factors))"
pub fn full_type_summary(expr: &SymExpr) -> String {
  match &expr.kind {
    SymKind::Var(name) => format!("Var({})", name),
    SymKind::Const(v) => {
      if v.fract() == 0.0 && v.abs() < 1e10 {
        format!("Const({})", *v as i64)
      } else {
        format!("Const({:.4})", v)
      }
    }
    SymKind::Exact(n) => format!("Exact({})", n),
    SymKind::Add(xs) => {
      let inner: Vec<_> = xs.iter().map(full_type_summary).collect();
      format!("Add({})", inner.join(", "))
    }
    SymKind::Mul(xs) => {
      let inner: Vec<_> = xs.iter().map(full_type_summary).collect();
      format!("Mul({})", inner.join(", "))
    }
    SymKind::Pow(base, exp) => {
      format!(
        "Pow({}, {})",
        full_type_summary(base),
        full_type_summary(exp)
      )
    }
    SymKind::Neg(x) => format!("Neg({})", full_type_summary(x)),
    SymKind::Sin(x) => format!("Sin({})", full_type_summary(x)),
    SymKind::Cos(x) => format!("Cos({})", full_type_summary(x)),
    SymKind::Tan(x) => format!("Tan({})", full_type_summary(x)),
    SymKind::Exp(x) => format!("Exp({})", full_type_summary(x)),
    SymKind::Log(x) => format!("Log({})", full_type_summary(x)),
    SymKind::Abs(x) => format!("Abs({})", full_type_summary(x)),
    SymKind::Derivative(x, var) => format!("D[{}, {}]", full_type_summary(x), var),
    SymKind::Tensor(t) => {
      let indices: Vec<_> = t
        .indices
        .iter()
        .map(|i| format!("{}:{:?}", i.name, i.position))
        .collect();
      format!("Tensor({}[{}])", t.name, indices.join(","))
    }
    SymKind::Contract(x, i1, i2) => {
      format!("Contract({}, {}, {})", full_type_summary(x), i1, i2)
    }
    SymKind::Raise(x, idx) => format!("Raise({}, {})", full_type_summary(x), idx),
    SymKind::Lower(x, idx) => format!("Lower({}, {})", full_type_summary(x), idx),
  }
}

/// Zone + CT 태그 정보 포함 요약 (LSP hover용)
pub fn hover_info(expr: &SymExpr, ct: Option<&CtTag>) -> String {
  let mut parts = vec![type_summary(expr)];

  // Zone 정보
  match expr.zone {
    Zone::Symbolic => parts.push("zone=symbolic".to_string()),
    Zone::Numeric => parts.push("zone=numeric".to_string()),
  }

  // CT 태그 정보
  if let Some(tag) = ct {
    parts.push(format!("cat={:?}", tag.category));
    if let Some(unit) = &tag.unit {
      parts.push(format!("unit={}", unit));
    }
  }

  parts.join(" | ")
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::approx_constant)]
mod tests {
  use super::*;

  #[test]
  fn test_json_roundtrip_var() {
    let expr = SymExpr::var("x");
    let json = to_json(&expr).unwrap();
    let restored = from_json(&json).unwrap();
    assert!(matches!(restored.kind, SymKind::Var(ref name) if name == "x"));
  }

  #[test]
  fn test_json_roundtrip_const() {
    let expr = SymExpr::constant(3.14);
    let json = to_json(&expr).unwrap();
    let restored = from_json(&json).unwrap();
    if let SymKind::Const(v) = restored.kind {
      assert!((v - 3.14).abs() < 1e-10);
    } else {
      panic!("Expected Const");
    }
  }

  #[test]
  fn test_json_roundtrip_add() {
    let expr = SymExpr::add(vec![
      SymExpr::var("x"),
      SymExpr::var("y"),
      SymExpr::constant(1.0),
    ]);
    let json = to_json(&expr).unwrap();
    let restored = from_json(&json).unwrap();
    if let SymKind::Add(xs) = restored.kind {
      assert_eq!(xs.len(), 3);
    } else {
      panic!("Expected Add");
    }
  }

  #[test]
  fn test_json_roundtrip_complex() {
    // sin(x^2 + y)
    let expr = SymExpr::sin(SymExpr::add(vec![
      SymExpr::pow(SymExpr::var("x"), SymExpr::constant(2.0)),
      SymExpr::var("y"),
    ]));
    let json = to_json_pretty(&expr).unwrap();
    let restored = from_json(&json).unwrap();
    assert!(matches!(restored.kind, SymKind::Sin(_)));
  }

  #[test]
  fn test_type_summary() {
    let expr = SymExpr::add(vec![SymExpr::var("x"), SymExpr::constant(1.0)]);
    assert_eq!(type_summary(&expr), "Add(2 terms)");
  }

  #[test]
  fn test_full_type_summary() {
    let expr = SymExpr::add(vec![SymExpr::var("x"), SymExpr::constant(1.0)]);
    assert_eq!(full_type_summary(&expr), "Add(Var(x), Const(1))");
  }

  #[test]
  fn test_hover_info() {
    let expr = SymExpr::var("velocity");
    let tag = CtTag::scalar().with_unit("m/s");
    let info = hover_info(&expr, Some(&tag));
    assert!(info.contains("Var(velocity)"));
    assert!(info.contains("zone=symbolic"));
    assert!(info.contains("unit=m/s"));
  }
}
