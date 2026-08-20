//! Minimal pnix surface validator.
//!
//! This enforces the core subset defined in `docs/pnix-core-grammar.md`.

use crate::lang::pnix::error::PnixError;
use crate::lang::pnix::ident::is_ident;
use crate::lang::pnix::parser::parse_expr;
use crate::lang::pnix::syntax::{
  AttrKeySegment, PnixAttrItem, PnixExpr, PnixLetBinding, PnixParamPattern,
};

/// Minimal parser options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MinimalParseOptions {
  /// Allow experimental/extended syntax in minimal parsing entrypoint.
  ///
  /// Default is `false` to keep minimal core fail-closed.
  pub allow_experimental_syntax: bool,
}

impl Default for MinimalParseOptions {
  fn default() -> Self {
    Self {
      allow_experimental_syntax: false,
    }
  }
}

fn unsupported(feature: &str) -> PnixError {
  PnixError::UnsupportedSyntax {
    message: format!("pnix minimal core does not support {feature}"),
    span: None,
  }
}

/// 최소 표현식 검증: 표현식이 최소 표현식인지 확인
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 검증만, 값 계산 없음
pub fn validate_minimal_expr(expr: &PnixExpr) -> Result<(), PnixError> {
  match expr {
    PnixExpr::Int(_)
    | PnixExpr::Float(_)
    | PnixExpr::Bool(_)
    | PnixExpr::Null
    | PnixExpr::String(_) => Ok(()),
    PnixExpr::Var(name) => {
      if !is_ident(name) {
        return Err(unsupported("non-ident variable"));
      }
      Ok(())
    }
    PnixExpr::StringInterp(_) => Err(unsupported("string interpolation")),
    PnixExpr::Path(_) => Err(unsupported("path literal")),
    PnixExpr::Let { bindings, body } => {
      for binding in bindings {
        match binding {
          PnixLetBinding::Binding { pattern, value } => {
            match pattern {
              PnixParamPattern::Ident(name) => {
                if !is_ident(name) {
                  return Err(unsupported("non-ident let binding name"));
                }
              }
              _ => return Err(unsupported("non-ident let pattern")),
            }
            validate_minimal_expr(value)?;
          }
          PnixLetBinding::Inherit { .. } => return Err(unsupported("inherit binding")),
        }
      }
      validate_minimal_expr(body)
    }
    PnixExpr::Lambda { param, body } => {
      match param {
        PnixParamPattern::Ident(name) => {
          if !is_ident(name) {
            return Err(unsupported("non-ident lambda parameter"));
          }
        }
        _ => return Err(unsupported("non-ident lambda parameter")),
      }
      validate_minimal_expr(body)
    }
    PnixExpr::Apply { func, arg } => {
      validate_minimal_expr(func)?;
      validate_minimal_expr(arg)
    }
    PnixExpr::AttrSet { items, recursive } => {
      if *recursive {
        return Err(unsupported("recursive attrset"));
      }
      for item in items {
        match item {
          PnixAttrItem::Assign {
            key_path, value, ..
          } => {
            if key_path.len() != 1 {
              return Err(unsupported("nested attrset key path"));
            }
            if let Some(key) = key_path.first() {
              if !is_ident(key) {
                return Err(unsupported("non-ident attrset key"));
              }
            }
            validate_minimal_expr(value)?;
          }
          PnixAttrItem::DynamicAssign { key_path, .. } => {
            for segment in key_path {
              if let AttrKeySegment::Dynamic(_) = segment {
                return Err(unsupported("dynamic attrset key"));
              }
            }
            return Err(unsupported("dynamic attrset assignment"));
          }
          PnixAttrItem::Inherit { .. } => return Err(unsupported("attrset inherit")),
        }
      }
      Ok(())
    }
    PnixExpr::List(items) => {
      for item in items {
        validate_minimal_expr(item)?;
      }
      Ok(())
    }
    PnixExpr::If { .. } => Err(unsupported("if expression")),
    PnixExpr::Select { .. } => Err(unsupported("attr selection")),
    PnixExpr::SelectOrDefault { .. } => Err(unsupported("attr selection with default")),
    PnixExpr::Index { .. } => Err(unsupported("list index")),
    PnixExpr::Binary { .. } => Err(unsupported("binary operator")),
    PnixExpr::Unary { .. } => Err(unsupported("unary operator")),
    PnixExpr::Construct { .. } => Err(unsupported("ADT constructor")),
    PnixExpr::Match { .. } => Err(unsupported("match expression")),
    PnixExpr::Import { .. } => Err(unsupported("import expression")),
    PnixExpr::With { .. } => Err(unsupported("with expression")),
    PnixExpr::Assert { .. } => Err(unsupported("assert expression")),
    PnixExpr::HasAttr { .. } => Err(unsupported("has-attr expression")),
    PnixExpr::DynamicHasAttr { .. } => Err(unsupported("dynamic has-attr expression")),
    PnixExpr::DynamicSelect { .. } => Err(unsupported("dynamic attr selection")),
    PnixExpr::DynamicSelectOrDefault { .. } => Err(unsupported("dynamic attr selection default")),
  }
}

/// 최소 표현식 파싱: 최소 표현식만 허용하는 파서
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 파싱만, 값 계산 없음
pub fn parse_minimal_expr(source: &str) -> Result<PnixExpr, PnixError> {
  parse_minimal_expr_with_options(source, MinimalParseOptions::default())
}

/// 최소 표현식 파싱(옵션): 기본은 strict(minimal only), 명시 옵션으로만 확장을 허용한다.
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 파싱/검증만, 값 계산 없음
pub fn parse_minimal_expr_with_options(
  source: &str,
  options: MinimalParseOptions,
) -> Result<PnixExpr, PnixError> {
  let expr = parse_expr(source)?;
  if !options.allow_experimental_syntax {
    validate_minimal_expr(&expr)?;
  }
  Ok(expr)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn validate_minimal_ok() {
    let expr = parse_expr("let x = 1; y = { a = 2; }; in f x y").unwrap();
    validate_minimal_expr(&expr).unwrap();
  }

  #[test]
  fn reject_if_expr() {
    let expr = parse_expr("if true then 1 else 2").unwrap();
    assert!(validate_minimal_expr(&expr).is_err());
  }

  #[test]
  fn reject_attrset_nested_key() {
    let expr = parse_expr("{ a.b = 1; }").unwrap();
    assert!(validate_minimal_expr(&expr).is_err());
  }

  #[test]
  fn reject_lambda_pattern() {
    let expr = parse_expr("{ x }: x").unwrap();
    assert!(validate_minimal_expr(&expr).is_err());
  }

  #[test]
  fn parse_minimal_default_rejects_experimental_syntax() {
    let err = parse_minimal_expr("if true then 1 else 2").unwrap_err();
    assert!(matches!(err, PnixError::UnsupportedSyntax { .. }));
  }

  #[test]
  fn parse_minimal_allows_experimental_when_option_is_enabled() {
    let expr = parse_minimal_expr_with_options(
      "if true then 1 else 2",
      MinimalParseOptions {
        allow_experimental_syntax: true,
      },
    )
    .unwrap();
    assert!(matches!(expr, PnixExpr::If { .. }));
  }

  #[test]
  fn parse_minimal_without_experimental_option_is_strict() {
    let err =
      parse_minimal_expr_with_options("if true then 1 else 2", MinimalParseOptions::default())
        .unwrap_err();
    assert!(matches!(err, PnixError::UnsupportedSyntax { .. }));
  }
}
