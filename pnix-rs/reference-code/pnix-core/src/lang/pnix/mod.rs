//! Pnix Language Frontend - AST 타입 및 파서 규칙
//!
//! pnix-old의 lang_pnix에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의 및 파싱만, 실행 로직 제외
//! - Surface AST 타입: `Expr` 등 (`.sam/.px` 정본 문법)
//! - Surface 파서: `parse_expr`, `parse_let_bindings`
//! - Unified AST 타입: `UnifiedExpr`, `PnixError`
//! - Lowering 함수: `lower_to_fx_core`, `fx_core_to_unified` (구조 변환만)
//!
//! 실행 로직 (`compile_pnix_to_fx`, `compile_pnix_to_program`)은 executor로 이동

pub mod ast_json;
pub mod diagnostics;
pub mod error;
pub mod ident;
pub mod lexer;
pub mod lower;
pub mod minimal;
pub mod module;
pub mod parser;
pub mod syntax;
pub mod ui_json;
pub mod unified;

// 에러 타입 re-export
pub use error::PnixError;
// Lowering 함수 re-export
pub use lower::{
  fx_core_to_unified, lower_to_fx_core, lower_to_fx_core_with_mode, pnix_expr_to_unified,
};
// 최소 표현식 파싱 함수 re-export
pub use minimal::{parse_minimal_expr, validate_minimal_expr};
// 모듈 파싱 함수 re-export
pub use module::{
  module_expr_to_ast as pnix_module_expr_to_ast,
  module_expr_to_ast_with_imports as pnix_module_expr_to_ast_with_imports,
  parse_module as parse_pnix_module, parse_module_with_imports as parse_pnix_module_with_imports,
  PnixModuleWithImports,
};
// 파서 함수 re-export
pub use parser::{parse_expr, parse_let_bindings};
// 안정 AST JSON projection re-export
pub use ast_json::{parse_expr_to_ast_json, pnix_expr_to_ast_json, PNIX_AST_JSON_FORMAT};
// 문법 타입 re-export
pub use syntax::{
  AttrItem, Expr, LetBinding, ListPattern, MatchAttrField, MatchListPattern, ParamPattern,
  PatternField, PnixAttrItem, PnixExpr, PnixLetBinding, PnixListPattern, PnixMatchAttrField,
  PnixMatchListPattern, PnixParamPattern, PnixPatternField,
};
// Unified 표현식 타입 re-export
pub use unified::{resolve_signals, ExecutionMode, UnifiedExpr};

use crate::lang::layer::LayerPipeline;
use crate::types::{CoreType, TypeInferencer};
use std::sync::Arc;

/// 레이어 파이프라인 적용: 레이어별 리라이트 훅 적용
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn apply_layer_pipeline(
  expr: PnixExpr,
  pipeline: &LayerPipeline,
) -> Result<PnixExpr, PnixError> {
  let expr = apply_layer_desugar(expr, pipeline);
  let expr = apply_layer_typing(expr, pipeline)?;
  Ok(apply_layer_normalize(expr, pipeline))
}

/// 레이어 표현식 파싱: PNIX 표현식을 파싱하고 레이어 파이프라인 훅 적용
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 파싱 및 변환만, 값 계산 없음
pub fn parse_layer_expr(
  source: &str,
  pipeline: Option<&LayerPipeline>,
) -> Result<PnixExpr, PnixError> {
  let expr = parse_expr(source)?;
  Ok(match pipeline {
    Some(p) => apply_layer_pipeline(expr, p)?,
    None => expr,
  })
}

/// 레이어 파이프라인과 함께 UnifiedExpr 변환: PNIX 표현식을 UnifiedExpr로 변환하고 레이어 파이프라인 적용
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn pnix_expr_to_unified_with_layer_pipeline(
  expr: &PnixExpr,
  pipeline: Option<&LayerPipeline>,
) -> Result<UnifiedExpr, PnixError> {
  let expr = match pipeline {
    Some(p) => apply_layer_pipeline(expr.clone(), p)?,
    None => expr.clone(),
  };
  pnix_expr_to_unified(&expr)
}

/// 레이어 속성 집합 표현식 파싱: 속성 집합 표현식을 레이어 표현식으로 파싱
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 파싱 및 변환만, 값 계산 없음
pub fn parse_layer_attrset_expr(
  expr: &PnixExpr,
  pipeline: Option<&LayerPipeline>,
) -> Result<PnixExpr, PnixError> {
  let layer_expr = attrset_to_layer_expr(expr)
    .ok_or_else(|| PnixError::lowering("unsupported layer attrset expression"))?;
  Ok(match pipeline {
    Some(p) => apply_layer_pipeline(layer_expr, p)?,
    None => layer_expr,
  })
}

/// SETO 레이어 호출: SETO 레이어 호출 표현식 (예: seto.x3d { ... })
#[derive(Debug, Clone)]
pub struct SetoLayerCall {
  /// 레이어 이름 (예: "x3d", "ui.scene")
  pub layer: String,
  /// 인자 목록 (PnixExpr 목록)
  pub args: Vec<PnixExpr>,
}

/// SETO 레이어 블록: SETO 레이어 블록 표현식 (예: seto.x3d { params } { body })
#[derive(Debug, Clone)]
pub struct SetoLayerBlock {
  /// 레이어 이름 (예: "x3d", "ui.scene")
  pub layer: String,
  /// 파라미터 (선택적, AttrSet 형태)
  pub params: Option<PnixExpr>,
  /// 본문 (문자열 또는 AttrSet)
  pub body: SetoLayerBody,
}

/// SETO 레이어 본문: SETO 레이어 본문 타입
#[derive(Debug, Clone)]
pub enum SetoLayerBody {
  /// 문자열 본문 (레이어 표현식 문자열)
  String(
    /// 레이어 표현식 문자열
    String,
  ),
  /// 속성 집합 본문 (PnixExpr AttrSet)
  AttrSet(
    /// 속성 집합 표현식
    PnixExpr,
  ),
}

/// SETO 레이어 호출 파싱: PNIX 표현식에서 SETO 레이어 호출 추출
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 파싱만, 값 계산 없음
pub fn parse_seto_layer_call(expr: &PnixExpr) -> Option<SetoLayerCall> {
  let (head, args) = flatten_apply_expr(expr);
  if args.is_empty() {
    return None;
  }
  let path = expr_to_path(head)?;
  let layer = path.strip_prefix("seto.")?.to_string();
  let args = args.into_iter().cloned().collect();
  Some(SetoLayerCall { layer, args })
}

/// SETO 레이어 블록 파싱: PNIX 표현식에서 SETO 레이어 블록 추출
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 파싱만, 값 계산 없음
pub fn parse_seto_layer_block(expr: &PnixExpr) -> Option<SetoLayerBlock> {
  let call = parse_seto_layer_call(expr)?;
  match call.args.as_slice() {
    [body] => Some(SetoLayerBlock {
      layer: call.layer,
      params: None,
      body: parse_seto_layer_body(body)?,
    }),
    [params, body] => match params {
      PnixExpr::AttrSet { .. } => Some(SetoLayerBlock {
        layer: call.layer,
        params: Some(params.clone()),
        body: parse_seto_layer_body(body)?,
      }),
      _ => None,
    },
    _ => None,
  }
}

/// SETO 레이어 본문 파싱: PNIX 표현식에서 SETO 레이어 본문 추출
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 파싱만, 값 계산 없음
pub fn parse_seto_layer_body(body: &PnixExpr) -> Option<SetoLayerBody> {
  match body {
    PnixExpr::String(value) => Some(SetoLayerBody::String(value.clone())),
    PnixExpr::AttrSet { .. } => Some(SetoLayerBody::AttrSet(body.clone())),
    _ => None,
  }
}

/// SETO 레이어 본문 표현식 파싱: SETO 레이어 본문을 PNIX 표현식으로 파싱
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 파싱 및 변환만, 값 계산 없음
pub fn parse_seto_layer_body_expr(
  body: &SetoLayerBody,
  pipeline: Option<&LayerPipeline>,
) -> Result<PnixExpr, PnixError> {
  match body {
    SetoLayerBody::String(value) => parse_layer_expr(value, pipeline),
    SetoLayerBody::AttrSet(expr) => parse_layer_attrset_expr(expr, pipeline),
  }
}

/// 속성 집합을 레이어 표현식으로 변환: 속성 집합 표현식을 레이어 표현식으로 변환
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn attrset_to_layer_expr(expr: &PnixExpr) -> Option<PnixExpr> {
  let PnixExpr::AttrSet { items, .. } = expr else {
    return None;
  };

  let mut exprs = Vec::new();
  for item in items {
    let (key_path, value) = match item {
      PnixAttrItem::Assign {
        key_path, value, ..
      } => (key_path, value),
      _ => return None,
    };
    if key_path.is_empty() {
      return None;
    }
    let ctor_name = key_path[0].as_str();
    let tail = &key_path[1..];
    let body = if tail.is_empty() {
      convert_body_value(value)?
    } else {
      let body_attrset = attrset_from_path(tail, (**value).clone());
      convert_body_attrset(&body_attrset)?
    };
    exprs.push(apply_constructor(ctor_name, body));
  }

  match exprs.len() {
    0 => None,
    1 => Some(exprs.remove(0)),
    _ => Some(PnixExpr::List(exprs)),
  }
}

fn flatten_apply_expr(expr: &PnixExpr) -> (&PnixExpr, Vec<&PnixExpr>) {
  let mut head = expr;
  let mut args = Vec::new();
  while let PnixExpr::Apply { func, arg } = head {
    args.push(arg.as_ref());
    head = func.as_ref();
  }
  args.reverse();
  (head, args)
}

fn convert_body_attrset(expr: &PnixExpr) -> Option<PnixExpr> {
  let PnixExpr::AttrSet { items, recursive } = expr else {
    return None;
  };
  let mut converted = Vec::new();
  for item in items {
    let (key_path, value, span) = match item {
      PnixAttrItem::Assign {
        key_path,
        value,
        span,
      } => (key_path, value, span),
      _ => return None,
    };
    if key_path.is_empty() {
      return None;
    }
    let key = key_path[0].clone();
    let tail = &key_path[1..];
    let value_expr = if tail.is_empty() {
      convert_body_value(value)?
    } else {
      build_body_value_from_tail(tail, value)?
    };
    converted.push(PnixAttrItem::Assign {
      key_path: vec![key],
      value: std::sync::Arc::new(value_expr),
      span: span.clone(),
    });
  }
  Some(PnixExpr::AttrSet {
    items: converted,
    recursive: *recursive,
  })
}

fn convert_body_value(expr: &PnixExpr) -> Option<PnixExpr> {
  match expr {
    PnixExpr::AttrSet { .. } => convert_body_attrset(expr),
    PnixExpr::List(items) => {
      let mut converted = Vec::with_capacity(items.len());
      for item in items {
        converted.push(convert_body_value(item)?);
      }
      Some(PnixExpr::List(converted))
    }
    _ => Some(expr.clone()),
  }
}

fn build_body_value_from_tail(tail: &[String], value: &PnixExpr) -> Option<PnixExpr> {
  if tail.is_empty() {
    return convert_body_value(value);
  }
  if tail.len() == 1 {
    let nested_value = convert_body_value(value)?;
    let nested_item = PnixAttrItem::Assign {
      key_path: vec![tail[0].clone()],
      value: std::sync::Arc::new(nested_value),
      span: crate::diagnostics::Span::empty(),
    };
    return Some(PnixExpr::AttrSet {
      items: vec![nested_item],
      recursive: false,
    });
  }

  let ctor_name = tail[0].as_str();
  let inner_tail = &tail[1..];
  let inner_attrset = attrset_from_path(inner_tail, value.clone());
  let inner_body = convert_body_attrset(&inner_attrset)?;
  Some(apply_constructor(ctor_name, inner_body))
}

fn apply_constructor(name: &str, body: PnixExpr) -> PnixExpr {
  let func = constructor_or_var(name);
  PnixExpr::Apply {
    func: Arc::new(func),
    arg: Arc::new(body),
  }
}

fn constructor_or_var(name: &str) -> PnixExpr {
  // After the parser was changed to keep bare uppercase identifiers as
  // `Var(...)` (Nix-compat), this helper must follow suit: an uppercase
  // attribute name in a layer attrset means the same thing as a
  // capitalized identifier in any other position — a variable reference.
  // ADT constructors only arise from explicit `Name(args)` syntax in
  // `parse_postfix`.
  PnixExpr::Var(name.to_string())
}

fn attrset_from_path(path: &[String], value: PnixExpr) -> PnixExpr {
  PnixExpr::AttrSet {
    items: vec![PnixAttrItem::Assign {
      key_path: path.to_vec(),
      value: Arc::new(value),
      span: crate::diagnostics::Span::empty(),
    }],
    recursive: false,
  }
}

fn apply_layer_desugar(expr: PnixExpr, pipeline: &LayerPipeline) -> PnixExpr {
  desugar_expr(expr, &pipeline.desugar.rules)
}

fn apply_layer_typing(expr: PnixExpr, pipeline: &LayerPipeline) -> Result<PnixExpr, PnixError> {
  validate_layer_typing(&expr, pipeline)?;
  enforce_layer_types(&expr, pipeline)?;
  Ok(expr)
}

fn apply_layer_normalize(expr: PnixExpr, pipeline: &LayerPipeline) -> PnixExpr {
  normalize_expr(expr, &pipeline.normalize.rules)
}

fn desugar_expr(expr: PnixExpr, rules: &std::collections::BTreeMap<String, String>) -> PnixExpr {
  match expr {
    PnixExpr::StringInterp(parts) => PnixExpr::StringInterp(
      parts
        .into_iter()
        .map(|part| match part {
          crate::lang::pnix::syntax::StringInterpPart::Lit(_) => part,
          crate::lang::pnix::syntax::StringInterpPart::Expr(expr) => {
            crate::lang::pnix::syntax::StringInterpPart::Expr(Arc::new(desugar_expr(
              Arc::unwrap_or_clone(expr),
              rules,
            )))
          }
        })
        .collect(),
    ),
    PnixExpr::Let { bindings, body } => PnixExpr::Let {
      bindings: bindings
        .into_iter()
        .map(|binding| match binding {
          crate::lang::pnix::syntax::PnixLetBinding::Binding { pattern, value } => {
            crate::lang::pnix::syntax::PnixLetBinding::Binding {
              pattern: desugar_param_pattern(pattern, rules),
              value: std::sync::Arc::new(desugar_expr(Arc::unwrap_or_clone(value), rules)),
            }
          }
          crate::lang::pnix::syntax::PnixLetBinding::Inherit { from, names } => {
            crate::lang::pnix::syntax::PnixLetBinding::Inherit {
              from: from.map(|expr| Arc::new(desugar_expr(Arc::unwrap_or_clone(expr), rules))),
              names,
            }
          }
        })
        .collect(),
      body: Arc::new(desugar_expr(Arc::unwrap_or_clone(body), rules)),
    },
    PnixExpr::If { cond, then_, else_ } => PnixExpr::If {
      cond: Arc::new(desugar_expr(Arc::unwrap_or_clone(cond), rules)),
      then_: Arc::new(desugar_expr(Arc::unwrap_or_clone(then_), rules)),
      else_: Arc::new(desugar_expr(Arc::unwrap_or_clone(else_), rules)),
    },
    PnixExpr::Lambda { param, body } => PnixExpr::Lambda {
      param: desugar_param_pattern(param, rules),
      body: Arc::new(desugar_expr(Arc::unwrap_or_clone(body), rules)),
    },
    PnixExpr::Apply { func, arg } => PnixExpr::Apply {
      func: Arc::new(desugar_expr(Arc::unwrap_or_clone(func), rules)),
      arg: Arc::new(desugar_expr(Arc::unwrap_or_clone(arg), rules)),
    },
    PnixExpr::AttrSet { items, recursive } => PnixExpr::AttrSet {
      items: items
        .into_iter()
        .map(|item| match item {
          crate::lang::pnix::syntax::PnixAttrItem::Assign {
            key_path,
            value,
            span,
          } => crate::lang::pnix::syntax::PnixAttrItem::Assign {
            key_path,
            value: std::sync::Arc::new(desugar_expr(Arc::unwrap_or_clone(value), rules)),
            span,
          },
          crate::lang::pnix::syntax::PnixAttrItem::DynamicAssign {
            key_path,
            value,
            span,
          } => crate::lang::pnix::syntax::PnixAttrItem::DynamicAssign {
            key_path: key_path
              .into_iter()
              .map(|seg| match seg {
                crate::lang::pnix::syntax::AttrKeySegment::Static(_) => seg,
                crate::lang::pnix::syntax::AttrKeySegment::Dynamic(expr) => {
                  crate::lang::pnix::syntax::AttrKeySegment::Dynamic(Arc::new(desugar_expr(
                    Arc::unwrap_or_clone(expr),
                    rules,
                  )))
                }
              })
              .collect(),
            value: std::sync::Arc::new(desugar_expr(Arc::unwrap_or_clone(value), rules)),
            span,
          },
          crate::lang::pnix::syntax::PnixAttrItem::Inherit { from, names, span } => {
            crate::lang::pnix::syntax::PnixAttrItem::Inherit {
              from: from.map(|expr| Arc::new(desugar_expr(Arc::unwrap_or_clone(expr), rules))),
              names,
              span,
            }
          }
        })
        .collect(),
      recursive,
    },
    PnixExpr::List(items) => PnixExpr::List(
      items
        .into_iter()
        .map(|item| desugar_expr(item, rules))
        .collect(),
    ),
    PnixExpr::Select { base, attr } => PnixExpr::Select {
      base: Arc::new(desugar_expr(Arc::unwrap_or_clone(base), rules)),
      attr,
    },
    PnixExpr::SelectOrDefault {
      base,
      attr,
      default,
    } => PnixExpr::SelectOrDefault {
      base: Arc::new(desugar_expr(Arc::unwrap_or_clone(base), rules)),
      attr,
      default: Arc::new(desugar_expr(Arc::unwrap_or_clone(default), rules)),
    },
    PnixExpr::Index { base, index } => PnixExpr::Index {
      base: Arc::new(desugar_expr(Arc::unwrap_or_clone(base), rules)),
      index: Arc::new(desugar_expr(Arc::unwrap_or_clone(index), rules)),
    },
    PnixExpr::Binary { op, lhs, rhs } => {
      let lhs = desugar_expr(Arc::unwrap_or_clone(lhs), rules);
      let rhs = desugar_expr(Arc::unwrap_or_clone(rhs), rules);
      if let Some(target) = rules.get(op.as_ref()) {
        let func = PnixExpr::Var(target.clone());
        let applied = PnixExpr::Apply {
          func: Arc::new(func),
          arg: Arc::new(lhs),
        };
        PnixExpr::Apply {
          func: Arc::new(applied),
          arg: Arc::new(rhs),
        }
      } else {
        PnixExpr::Binary {
          op,
          lhs: Arc::new(lhs),
          rhs: Arc::new(rhs),
        }
      }
    }
    PnixExpr::Unary { op, arg } => PnixExpr::Unary {
      op,
      arg: Arc::new(desugar_expr(Arc::unwrap_or_clone(arg), rules)),
    },
    PnixExpr::Construct { variant, args } => PnixExpr::Construct {
      variant,
      args: args
        .into_iter()
        .map(|arg| desugar_expr(arg, rules))
        .collect(),
    },
    PnixExpr::Match { scrutinee, arms } => PnixExpr::Match {
      scrutinee: Arc::new(desugar_expr(Arc::unwrap_or_clone(scrutinee), rules)),
      arms: arms
        .into_iter()
        .map(|arm| crate::lang::pnix::syntax::PnixMatchArm {
          pattern: arm.pattern,
          guard: arm
            .guard
            .map(|expr| Arc::new(desugar_expr(Arc::unwrap_or_clone(expr), rules))),
          body: Arc::new(desugar_expr(Arc::unwrap_or_clone(arm.body), rules)),
        })
        .collect(),
    },
    PnixExpr::Import { path } => PnixExpr::Import {
      path: Arc::new(desugar_expr(Arc::unwrap_or_clone(path), rules)),
    },
    PnixExpr::With { env, body } => PnixExpr::With {
      env: Arc::new(desugar_expr(Arc::unwrap_or_clone(env), rules)),
      body: Arc::new(desugar_expr(Arc::unwrap_or_clone(body), rules)),
    },
    PnixExpr::Assert { cond, body } => PnixExpr::Assert {
      cond: Arc::new(desugar_expr(Arc::unwrap_or_clone(cond), rules)),
      body: Arc::new(desugar_expr(Arc::unwrap_or_clone(body), rules)),
    },
    PnixExpr::HasAttr { base, attr } => PnixExpr::HasAttr {
      base: Arc::new(desugar_expr(Arc::unwrap_or_clone(base), rules)),
      attr,
    },
    PnixExpr::DynamicHasAttr { base, attr_expr } => PnixExpr::DynamicHasAttr {
      base: Arc::new(desugar_expr(Arc::unwrap_or_clone(base), rules)),
      attr_expr: Arc::new(desugar_expr(Arc::unwrap_or_clone(attr_expr), rules)),
    },
    PnixExpr::DynamicSelect { base, attr_expr } => PnixExpr::DynamicSelect {
      base: Arc::new(desugar_expr(Arc::unwrap_or_clone(base), rules)),
      attr_expr: Arc::new(desugar_expr(Arc::unwrap_or_clone(attr_expr), rules)),
    },
    PnixExpr::DynamicSelectOrDefault {
      base,
      attr_expr,
      default,
    } => PnixExpr::DynamicSelectOrDefault {
      base: Arc::new(desugar_expr(Arc::unwrap_or_clone(base), rules)),
      attr_expr: Arc::new(desugar_expr(Arc::unwrap_or_clone(attr_expr), rules)),
      default: Arc::new(desugar_expr(Arc::unwrap_or_clone(default), rules)),
    },
    other => other,
  }
}

fn normalize_expr(expr: PnixExpr, rules: &std::collections::BTreeMap<String, String>) -> PnixExpr {
  match expr {
    PnixExpr::StringInterp(parts) => PnixExpr::StringInterp(
      parts
        .into_iter()
        .map(|part| match part {
          crate::lang::pnix::syntax::StringInterpPart::Lit(_) => part,
          crate::lang::pnix::syntax::StringInterpPart::Expr(expr) => {
            crate::lang::pnix::syntax::StringInterpPart::Expr(Arc::new(normalize_expr(
              Arc::unwrap_or_clone(expr),
              rules,
            )))
          }
        })
        .collect(),
    ),
    PnixExpr::Let { bindings, body } => PnixExpr::Let {
      bindings: bindings
        .into_iter()
        .map(|binding| match binding {
          crate::lang::pnix::syntax::PnixLetBinding::Binding { pattern, value } => {
            crate::lang::pnix::syntax::PnixLetBinding::Binding {
              pattern,
              value: Arc::new(normalize_expr(Arc::unwrap_or_clone(value), rules)),
            }
          }
          crate::lang::pnix::syntax::PnixLetBinding::Inherit { from, names } => {
            crate::lang::pnix::syntax::PnixLetBinding::Inherit {
              from: from.map(|expr| Arc::new(normalize_expr(Arc::unwrap_or_clone(expr), rules))),
              names,
            }
          }
        })
        .collect(),
      body: Arc::new(normalize_expr(Arc::unwrap_or_clone(body), rules)),
    },
    PnixExpr::If { cond, then_, else_ } => PnixExpr::If {
      cond: Arc::new(normalize_expr(Arc::unwrap_or_clone(cond), rules)),
      then_: Arc::new(normalize_expr(Arc::unwrap_or_clone(then_), rules)),
      else_: Arc::new(normalize_expr(Arc::unwrap_or_clone(else_), rules)),
    },
    PnixExpr::Lambda { param, body } => PnixExpr::Lambda {
      param,
      body: Arc::new(normalize_expr(Arc::unwrap_or_clone(body), rules)),
    },
    PnixExpr::Apply { func, arg } => {
      let func = normalize_expr(Arc::unwrap_or_clone(func), rules);
      let arg = normalize_expr(Arc::unwrap_or_clone(arg), rules);
      normalize_apply(
        PnixExpr::Apply {
          func: Arc::new(func),
          arg: Arc::new(arg),
        },
        rules,
      )
    }
    PnixExpr::AttrSet { items, recursive } => PnixExpr::AttrSet {
      items: items
        .into_iter()
        .map(|item| match item {
          crate::lang::pnix::syntax::PnixAttrItem::Assign {
            key_path,
            value,
            span,
          } => crate::lang::pnix::syntax::PnixAttrItem::Assign {
            key_path,
            value: Arc::new(normalize_expr(Arc::unwrap_or_clone(value), rules)),
            span,
          },
          crate::lang::pnix::syntax::PnixAttrItem::DynamicAssign {
            key_path,
            value,
            span,
          } => crate::lang::pnix::syntax::PnixAttrItem::DynamicAssign {
            key_path: key_path
              .into_iter()
              .map(|seg| match seg {
                crate::lang::pnix::syntax::AttrKeySegment::Static(_) => seg,
                crate::lang::pnix::syntax::AttrKeySegment::Dynamic(expr) => {
                  crate::lang::pnix::syntax::AttrKeySegment::Dynamic(Arc::new(normalize_expr(
                    Arc::unwrap_or_clone(expr),
                    rules,
                  )))
                }
              })
              .collect(),
            value: Arc::new(normalize_expr(Arc::unwrap_or_clone(value), rules)),
            span,
          },
          crate::lang::pnix::syntax::PnixAttrItem::Inherit { from, names, span } => {
            crate::lang::pnix::syntax::PnixAttrItem::Inherit {
              from: from.map(|expr| Arc::new(normalize_expr(Arc::unwrap_or_clone(expr), rules))),
              names,
              span,
            }
          }
        })
        .collect(),
      recursive,
    },
    PnixExpr::List(items) => PnixExpr::List(
      items
        .into_iter()
        .map(|item| normalize_expr(item, rules))
        .collect(),
    ),
    PnixExpr::Select { base, attr } => PnixExpr::Select {
      base: Arc::new(normalize_expr(Arc::unwrap_or_clone(base), rules)),
      attr,
    },
    PnixExpr::SelectOrDefault {
      base,
      attr,
      default,
    } => PnixExpr::SelectOrDefault {
      base: Arc::new(normalize_expr(Arc::unwrap_or_clone(base), rules)),
      attr,
      default: Arc::new(normalize_expr(Arc::unwrap_or_clone(default), rules)),
    },
    PnixExpr::Index { base, index } => PnixExpr::Index {
      base: Arc::new(normalize_expr(Arc::unwrap_or_clone(base), rules)),
      index: Arc::new(normalize_expr(Arc::unwrap_or_clone(index), rules)),
    },
    PnixExpr::Binary { op, lhs, rhs } => PnixExpr::Binary {
      op,
      lhs: Arc::new(normalize_expr(Arc::unwrap_or_clone(lhs), rules)),
      rhs: Arc::new(normalize_expr(Arc::unwrap_or_clone(rhs), rules)),
    },
    PnixExpr::Unary { op, arg } => PnixExpr::Unary {
      op,
      arg: Arc::new(normalize_expr(Arc::unwrap_or_clone(arg), rules)),
    },
    PnixExpr::Construct { variant, args } => PnixExpr::Construct {
      variant,
      args: args
        .into_iter()
        .map(|arg| normalize_expr(arg, rules))
        .collect(),
    },
    PnixExpr::Match { scrutinee, arms } => PnixExpr::Match {
      scrutinee: Arc::new(normalize_expr(Arc::unwrap_or_clone(scrutinee), rules)),
      arms: arms
        .into_iter()
        .map(|arm| crate::lang::pnix::syntax::PnixMatchArm {
          pattern: arm.pattern,
          guard: arm
            .guard
            .map(|expr| Arc::new(normalize_expr(Arc::unwrap_or_clone(expr), rules))),
          body: Arc::new(normalize_expr(Arc::unwrap_or_clone(arm.body), rules)),
        })
        .collect(),
    },
    PnixExpr::Import { path } => PnixExpr::Import {
      path: Arc::new(normalize_expr(Arc::unwrap_or_clone(path), rules)),
    },
    PnixExpr::With { env, body } => PnixExpr::With {
      env: Arc::new(normalize_expr(Arc::unwrap_or_clone(env), rules)),
      body: Arc::new(normalize_expr(Arc::unwrap_or_clone(body), rules)),
    },
    PnixExpr::Assert { cond, body } => PnixExpr::Assert {
      cond: Arc::new(normalize_expr(Arc::unwrap_or_clone(cond), rules)),
      body: Arc::new(normalize_expr(Arc::unwrap_or_clone(body), rules)),
    },
    PnixExpr::HasAttr { base, attr } => PnixExpr::HasAttr {
      base: Arc::new(normalize_expr(Arc::unwrap_or_clone(base), rules)),
      attr,
    },
    PnixExpr::DynamicHasAttr { base, attr_expr } => PnixExpr::DynamicHasAttr {
      base: Arc::new(normalize_expr(Arc::unwrap_or_clone(base), rules)),
      attr_expr: Arc::new(normalize_expr(Arc::unwrap_or_clone(attr_expr), rules)),
    },
    PnixExpr::DynamicSelect { base, attr_expr } => PnixExpr::DynamicSelect {
      base: Arc::new(normalize_expr(Arc::unwrap_or_clone(base), rules)),
      attr_expr: Arc::new(normalize_expr(Arc::unwrap_or_clone(attr_expr), rules)),
    },
    PnixExpr::DynamicSelectOrDefault {
      base,
      attr_expr,
      default,
    } => PnixExpr::DynamicSelectOrDefault {
      base: Arc::new(normalize_expr(Arc::unwrap_or_clone(base), rules)),
      attr_expr: Arc::new(normalize_expr(Arc::unwrap_or_clone(attr_expr), rules)),
      default: Arc::new(normalize_expr(Arc::unwrap_or_clone(default), rules)),
    },
    other => other,
  }
}

fn normalize_apply(expr: PnixExpr, rules: &std::collections::BTreeMap<String, String>) -> PnixExpr {
  match expr {
    PnixExpr::Apply { func, arg } => match Arc::unwrap_or_clone(func) {
      PnixExpr::Apply {
        func: inner_func,
        arg: lhs,
      } => {
        let func_expr = Arc::unwrap_or_clone(inner_func);
        if let Some(name) = expr_to_path(&func_expr) {
          if let Some(rule) = rules.get(&name) {
            if name == "Rat.make" {
              if let (PnixExpr::Int(num), PnixExpr::Int(den)) = (&*lhs, &*arg) {
                let (n, d) = normalize_rat_make(*num, *den);
                let inner = PnixExpr::Apply {
                  func: Arc::new(PnixExpr::Var(name)),
                  arg: Arc::new(PnixExpr::Int(n)),
                };
                return PnixExpr::Apply {
                  func: Arc::new(inner),
                  arg: Arc::new(PnixExpr::Int(d)),
                };
              }
            }
            if rule != &name && !rule.trim().is_empty() {
              let applied = PnixExpr::Apply {
                func: Arc::new(PnixExpr::Apply {
                  func: Arc::new(func_expr),
                  arg: lhs,
                }),
                arg,
              };
              return PnixExpr::Apply {
                func: Arc::new(PnixExpr::Var(rule.clone())),
                arg: Arc::new(applied),
              };
            }
          }
        }
        let inner = PnixExpr::Apply {
          func: Arc::new(func_expr),
          arg: lhs,
        };
        PnixExpr::Apply {
          func: Arc::new(inner),
          arg,
        }
      }
      other => {
        let name = expr_to_path(&other);
        if let Some(name) = name {
          if name == "Rat.make" {
            return PnixExpr::Apply {
              func: Arc::new(other),
              arg,
            };
          }
          if let Some(rule) = rules.get(&name) {
            if rule != &name && !rule.trim().is_empty() {
              let applied = PnixExpr::Apply {
                func: Arc::new(other),
                arg,
              };
              return PnixExpr::Apply {
                func: Arc::new(PnixExpr::Var(rule.clone())),
                arg: Arc::new(applied),
              };
            }
          }
        }
        PnixExpr::Apply {
          func: Arc::new(other),
          arg,
        }
      }
    },
    other => other,
  }
}

fn validate_layer_typing(expr: &PnixExpr, pipeline: &LayerPipeline) -> Result<(), PnixError> {
  let mut missing = std::collections::BTreeSet::new();
  collect_missing_ops(
    expr,
    &pipeline.typing.rules,
    &pipeline.desugar.rules,
    &mut missing,
  );
  if !missing.is_empty() {
    let mut list: Vec<_> = missing.into_iter().collect();
    list.sort();
    return Err(PnixError::TypeError {
      message: format!("unknown operators in layer expression: {}", list.join(", ")),
      span: None,
    });
  }

  let mut invalid = Vec::new();
  collect_invalid_op_arities(
    expr,
    &pipeline.typing.rules,
    &pipeline.desugar.rules,
    &mut invalid,
  );
  if invalid.is_empty() {
    return validate_apply_arities(expr, pipeline);
  }
  invalid.sort();
  Err(PnixError::TypeError {
    message: format!("invalid operator typing arity: {}", invalid.join(", ")),
    span: None,
  })
}

fn collect_missing_ops(
  expr: &PnixExpr,
  typing: &std::collections::BTreeMap<String, String>,
  desugar: &std::collections::BTreeMap<String, String>,
  missing: &mut std::collections::BTreeSet<String>,
) {
  match expr {
    PnixExpr::Binary { op, lhs, rhs } => {
      if !typing.contains_key(op.as_ref()) && !desugar.contains_key(op.as_ref()) {
        missing.insert(op.to_string());
      }
      collect_missing_ops(lhs, typing, desugar, missing);
      collect_missing_ops(rhs, typing, desugar, missing);
    }
    PnixExpr::Unary { op, arg } => {
      if !typing.contains_key(op.as_ref()) && !desugar.contains_key(op.as_ref()) {
        missing.insert(op.to_string());
      }
      collect_missing_ops(arg, typing, desugar, missing);
    }
    PnixExpr::Let { bindings, body } => {
      for binding in bindings {
        match binding {
          crate::lang::pnix::syntax::PnixLetBinding::Binding { pattern: _, value } => {
            collect_missing_ops(value, typing, desugar, missing);
          }
          crate::lang::pnix::syntax::PnixLetBinding::Inherit { from, names: _ } => {
            if let Some(expr) = from {
              collect_missing_ops(expr, typing, desugar, missing);
            }
          }
        }
      }
      collect_missing_ops(body, typing, desugar, missing);
    }
    PnixExpr::If { cond, then_, else_ } => {
      collect_missing_ops(cond, typing, desugar, missing);
      collect_missing_ops(then_, typing, desugar, missing);
      collect_missing_ops(else_, typing, desugar, missing);
    }
    PnixExpr::Lambda { param: _, body } => {
      collect_missing_ops(body, typing, desugar, missing);
    }
    PnixExpr::Apply { func, arg } => {
      collect_missing_ops(func, typing, desugar, missing);
      collect_missing_ops(arg, typing, desugar, missing);
    }
    PnixExpr::AttrSet { items, .. } => {
      for item in items {
        match item {
          crate::lang::pnix::syntax::PnixAttrItem::Assign {
            key_path: _, value, ..
          } => {
            collect_missing_ops(value, typing, desugar, missing);
          }
          crate::lang::pnix::syntax::PnixAttrItem::DynamicAssign {
            key_path, value, ..
          } => {
            for seg in key_path {
              if let crate::lang::pnix::syntax::AttrKeySegment::Dynamic(expr) = seg {
                collect_missing_ops(expr, typing, desugar, missing);
              }
            }
            collect_missing_ops(value, typing, desugar, missing);
          }
          crate::lang::pnix::syntax::PnixAttrItem::Inherit { from, names: _, .. } => {
            if let Some(expr) = from {
              collect_missing_ops(expr, typing, desugar, missing);
            }
          }
        }
      }
    }
    PnixExpr::List(items) => {
      for item in items {
        collect_missing_ops(item, typing, desugar, missing);
      }
    }
    PnixExpr::Select { base, .. } => collect_missing_ops(base, typing, desugar, missing),
    PnixExpr::SelectOrDefault { base, default, .. } => {
      collect_missing_ops(base, typing, desugar, missing);
      collect_missing_ops(default, typing, desugar, missing);
    }
    PnixExpr::Index { base, index } => {
      collect_missing_ops(base, typing, desugar, missing);
      collect_missing_ops(index, typing, desugar, missing);
    }
    PnixExpr::Match { scrutinee, arms } => {
      collect_missing_ops(scrutinee, typing, desugar, missing);
      for arm in arms {
        if let Some(expr) = &arm.guard {
          collect_missing_ops(expr, typing, desugar, missing);
        }
        collect_missing_ops(&arm.body, typing, desugar, missing);
      }
    }
    PnixExpr::Import { path } => collect_missing_ops(path, typing, desugar, missing),
    PnixExpr::With { env, body } => {
      collect_missing_ops(env, typing, desugar, missing);
      collect_missing_ops(body, typing, desugar, missing);
    }
    PnixExpr::Assert { cond, body } => {
      collect_missing_ops(cond, typing, desugar, missing);
      collect_missing_ops(body, typing, desugar, missing);
    }
    PnixExpr::HasAttr { base, .. } => collect_missing_ops(base, typing, desugar, missing),
    PnixExpr::DynamicHasAttr { base, attr_expr } => {
      collect_missing_ops(base, typing, desugar, missing);
      collect_missing_ops(attr_expr, typing, desugar, missing);
    }
    PnixExpr::DynamicSelect { base, attr_expr } => {
      collect_missing_ops(base, typing, desugar, missing);
      collect_missing_ops(attr_expr, typing, desugar, missing);
    }
    PnixExpr::DynamicSelectOrDefault {
      base,
      attr_expr,
      default,
    } => {
      collect_missing_ops(base, typing, desugar, missing);
      collect_missing_ops(attr_expr, typing, desugar, missing);
      collect_missing_ops(default, typing, desugar, missing);
    }
    PnixExpr::StringInterp(parts) => {
      for part in parts {
        if let crate::lang::pnix::syntax::StringInterpPart::Expr(expr) = part {
          collect_missing_ops(expr, typing, desugar, missing);
        }
      }
    }
    PnixExpr::Construct { args, .. } => {
      for arg in args {
        collect_missing_ops(arg, typing, desugar, missing);
      }
    }
    PnixExpr::Int(_)
    | PnixExpr::Float(_)
    | PnixExpr::Bool(_)
    | PnixExpr::Null
    | PnixExpr::String(_)
    | PnixExpr::Path(_)
    | PnixExpr::Var(_) => {}
  }
}

fn collect_invalid_op_arities(
  expr: &PnixExpr,
  typing: &std::collections::BTreeMap<String, String>,
  desugar: &std::collections::BTreeMap<String, String>,
  invalid: &mut Vec<String>,
) {
  match expr {
    PnixExpr::Binary { op, lhs, rhs } => {
      if let Some(rule) = typing.get(op.as_ref()) {
        if let Some(arity) = typing_rule_arity(rule) {
          if arity < 2 {
            invalid.push(format!("{op} expects >=2 args, typing={rule}"));
          }
        } else {
          invalid.push(format!("{op} has invalid typing rule: {rule}"));
        }
      } else if !desugar.contains_key(op.as_ref()) {
        invalid.push(format!("{op} missing typing rule"));
      }
      collect_invalid_op_arities(lhs, typing, desugar, invalid);
      collect_invalid_op_arities(rhs, typing, desugar, invalid);
    }
    PnixExpr::Unary { op, arg } => {
      if let Some(rule) = typing.get(op.as_ref()) {
        if let Some(arity) = typing_rule_arity(rule) {
          if arity < 1 {
            invalid.push(format!("{op} expects >=1 arg, typing={rule}"));
          }
        } else {
          invalid.push(format!("{op} has invalid typing rule: {rule}"));
        }
      } else if !desugar.contains_key(op.as_ref()) {
        invalid.push(format!("{op} missing typing rule"));
      }
      collect_invalid_op_arities(arg, typing, desugar, invalid);
    }
    PnixExpr::Let { bindings, body } => {
      for binding in bindings {
        match binding {
          crate::lang::pnix::syntax::PnixLetBinding::Binding { pattern: _, value } => {
            collect_invalid_op_arities(value, typing, desugar, invalid);
          }
          crate::lang::pnix::syntax::PnixLetBinding::Inherit { from, names: _ } => {
            if let Some(expr) = from {
              collect_invalid_op_arities(expr, typing, desugar, invalid);
            }
          }
        }
      }
      collect_invalid_op_arities(body, typing, desugar, invalid);
    }
    PnixExpr::If { cond, then_, else_ } => {
      collect_invalid_op_arities(cond, typing, desugar, invalid);
      collect_invalid_op_arities(then_, typing, desugar, invalid);
      collect_invalid_op_arities(else_, typing, desugar, invalid);
    }
    PnixExpr::Lambda { param: _, body } => {
      collect_invalid_op_arities(body, typing, desugar, invalid);
    }
    PnixExpr::Apply { func, arg } => {
      collect_invalid_op_arities(func, typing, desugar, invalid);
      collect_invalid_op_arities(arg, typing, desugar, invalid);
    }
    PnixExpr::AttrSet { items, .. } => {
      for item in items {
        match item {
          crate::lang::pnix::syntax::PnixAttrItem::Assign {
            key_path: _, value, ..
          } => {
            collect_invalid_op_arities(value, typing, desugar, invalid);
          }
          crate::lang::pnix::syntax::PnixAttrItem::DynamicAssign {
            key_path, value, ..
          } => {
            for seg in key_path {
              if let crate::lang::pnix::syntax::AttrKeySegment::Dynamic(expr) = seg {
                collect_invalid_op_arities(expr, typing, desugar, invalid);
              }
            }
            collect_invalid_op_arities(value, typing, desugar, invalid);
          }
          crate::lang::pnix::syntax::PnixAttrItem::Inherit { from, names: _, .. } => {
            if let Some(expr) = from {
              collect_invalid_op_arities(expr, typing, desugar, invalid);
            }
          }
        }
      }
    }
    PnixExpr::List(items) => {
      for item in items {
        collect_invalid_op_arities(item, typing, desugar, invalid);
      }
    }
    PnixExpr::Select { base, .. } => collect_invalid_op_arities(base, typing, desugar, invalid),
    PnixExpr::SelectOrDefault { base, default, .. } => {
      collect_invalid_op_arities(base, typing, desugar, invalid);
      collect_invalid_op_arities(default, typing, desugar, invalid);
    }
    PnixExpr::Index { base, index } => {
      collect_invalid_op_arities(base, typing, desugar, invalid);
      collect_invalid_op_arities(index, typing, desugar, invalid);
    }
    PnixExpr::Match { scrutinee, arms } => {
      collect_invalid_op_arities(scrutinee, typing, desugar, invalid);
      for arm in arms {
        if let Some(expr) = &arm.guard {
          collect_invalid_op_arities(expr, typing, desugar, invalid);
        }
        collect_invalid_op_arities(&arm.body, typing, desugar, invalid);
      }
    }
    PnixExpr::Import { path } => collect_invalid_op_arities(path, typing, desugar, invalid),
    PnixExpr::With { env, body } => {
      collect_invalid_op_arities(env, typing, desugar, invalid);
      collect_invalid_op_arities(body, typing, desugar, invalid);
    }
    PnixExpr::Assert { cond, body } => {
      collect_invalid_op_arities(cond, typing, desugar, invalid);
      collect_invalid_op_arities(body, typing, desugar, invalid);
    }
    PnixExpr::HasAttr { base, .. } => collect_invalid_op_arities(base, typing, desugar, invalid),
    PnixExpr::DynamicHasAttr { base, attr_expr } => {
      collect_invalid_op_arities(base, typing, desugar, invalid);
      collect_invalid_op_arities(attr_expr, typing, desugar, invalid);
    }
    PnixExpr::DynamicSelect { base, attr_expr } => {
      collect_invalid_op_arities(base, typing, desugar, invalid);
      collect_invalid_op_arities(attr_expr, typing, desugar, invalid);
    }
    PnixExpr::DynamicSelectOrDefault {
      base,
      attr_expr,
      default,
    } => {
      collect_invalid_op_arities(base, typing, desugar, invalid);
      collect_invalid_op_arities(attr_expr, typing, desugar, invalid);
      collect_invalid_op_arities(default, typing, desugar, invalid);
    }
    PnixExpr::StringInterp(parts) => {
      for part in parts {
        if let crate::lang::pnix::syntax::StringInterpPart::Expr(expr) = part {
          collect_invalid_op_arities(expr, typing, desugar, invalid);
        }
      }
    }
    PnixExpr::Construct { args, .. } => {
      for arg in args {
        collect_invalid_op_arities(arg, typing, desugar, invalid);
      }
    }
    PnixExpr::Int(_)
    | PnixExpr::Float(_)
    | PnixExpr::Bool(_)
    | PnixExpr::Null
    | PnixExpr::String(_)
    | PnixExpr::Path(_)
    | PnixExpr::Var(_) => {}
  }
}

fn typing_rule_arity(rule: &str) -> Option<usize> {
  let trimmed = rule.trim();
  let rhs = match trimmed.split_once("=>") {
    Some((_, rhs)) => rhs.trim(),
    None => trimmed,
  };
  let mut depth: i32 = 0;
  let mut count = 0usize;
  let chars: Vec<char> = rhs.chars().collect();
  let mut idx = 0usize;
  while idx < chars.len() {
    let ch = chars[idx];
    match ch {
      '(' | '[' | '{' => depth += 1,
      ')' | ']' | '}' => {
        if depth > 0 {
          depth -= 1;
        }
      }
      '-' => {
        if depth == 0 && idx + 1 < chars.len() && chars[idx + 1] == '>' {
          count += 1;
          idx += 1;
        }
      }
      _ => {}
    }
    idx += 1;
  }
  if count == 0 {
    None
  } else {
    Some(count)
  }
}

fn enforce_layer_types(expr: &PnixExpr, pipeline: &LayerPipeline) -> Result<(), PnixError> {
  let mut inferencer = TypeInferencer::new();
  for (token, rule) in &pipeline.typing.rules {
    let Some(func) = pipeline.desugar.rules.get(token) else {
      continue;
    };
    let Some(ty) = parse_typing_rule(rule) else {
      return Err(PnixError::TypeError {
        message: format!("invalid typing rule for operator {token}: {rule}"),
        span: None,
      });
    };
    let ty = generalize_type_vars(&ty);
    inferencer.register_symbol(func.clone(), ty);
  }

  let mut vars = Vec::new();
  collect_var_names(expr, &mut vars);
  for name in vars {
    if !inferencer.has_symbol(&name) {
      inferencer.register_unbound_var(name);
    }
  }

  let result = inferencer.infer_expr(expr);
  if result.errors.is_empty() {
    return Ok(());
  }
  let mut messages: Vec<String> = result.errors.into_iter().map(|e| e.to_string()).collect();
  messages.sort();
  Err(PnixError::TypeError {
    message: format!("layer typing failed: {}", messages.join("; ")),
    span: None,
  })
}

fn parse_typing_rule(rule: &str) -> Option<CoreType> {
  let trimmed = rule.trim();
  let rhs = match trimmed.split_once("=>") {
    Some((_, rhs)) => rhs.trim(),
    None => trimmed,
  };
  parse_core_type(rhs)
}

fn parse_core_type(src: &str) -> Option<CoreType> {
  let trimmed = src.trim();
  if trimmed.is_empty() {
    return Some(CoreType::Unit);
  }
  let stripped = strip_outer_parens(trimmed);
  if stripped != trimmed {
    return parse_core_type(stripped);
  }
  let parts = split_top_level_arrows(stripped);
  if parts.len() > 1 {
    let mut it = parts.into_iter().rev();
    let first = it.next()?;
    let mut ty = parse_core_type(&first)?;
    for part in it {
      let arg = parse_core_type(&part)?;
      ty = CoreType::Arrow(Box::new(arg), Box::new(ty));
    }
    return Some(ty);
  }
  Some(CoreType::parse(stripped))
}

fn strip_outer_parens(src: &str) -> &str {
  let chars: Vec<char> = src.chars().collect();
  if chars.len() < 2 || chars[0] != '(' || chars[chars.len() - 1] != ')' {
    return src;
  }
  let mut depth: i32 = 0;
  for (idx, ch) in chars.iter().enumerate() {
    match ch {
      '(' => depth += 1,
      ')' => {
        depth -= 1;
        if depth == 0 && idx + 1 != chars.len() {
          return src;
        }
      }
      _ => {}
    }
  }
  if depth == 0 {
    &src[1..src.len() - 1]
  } else {
    src
  }
}

fn split_top_level_arrows(src: &str) -> Vec<String> {
  let mut parts = Vec::new();
  let mut depth: i32 = 0;
  let mut last = 0usize;
  let chars: Vec<char> = src.chars().collect();
  let mut idx = 0usize;
  while idx < chars.len() {
    match chars[idx] {
      '(' | '[' | '{' => depth += 1,
      ')' | ']' | '}' => depth = depth.saturating_sub(1),
      '-' if depth == 0 && idx + 1 < chars.len() && chars[idx + 1] == '>' => {
        let seg: String = chars[last..idx].iter().collect();
        parts.push(seg.trim().to_string());
        idx += 1;
        last = idx + 1;
      }
      _ => {}
    }
    idx += 1;
  }
  let seg: String = chars[last..].iter().collect();
  parts.push(seg.trim().to_string());
  parts.into_iter().filter(|s| !s.is_empty()).collect()
}

fn generalize_type_vars(ty: &CoreType) -> CoreType {
  let mut vars = std::collections::BTreeSet::new();
  collect_type_vars(ty, &mut vars);
  if vars.is_empty() {
    return ty.clone();
  }
  CoreType::Forall {
    vars: vars.into_iter().collect(),
    body: Box::new(ty.clone()),
  }
}

fn collect_type_vars(ty: &CoreType, vars: &mut std::collections::BTreeSet<String>) {
  match ty {
    CoreType::Var(name) => {
      vars.insert(name.clone());
    }
    CoreType::Product(a, b) | CoreType::Arrow(a, b) | CoreType::Sum(a, b) => {
      collect_type_vars(a, vars);
      collect_type_vars(b, vars);
    }
    CoreType::Optional(inner) | CoreType::List(inner) => collect_type_vars(inner, vars),
    CoreType::Record(fields) => {
      for (_, ty) in fields {
        collect_type_vars(ty, vars);
      }
    }
    CoreType::Forall { vars: bound, body } => {
      let mut nested = std::collections::BTreeSet::new();
      collect_type_vars(body, &mut nested);
      for name in bound {
        nested.remove(name);
      }
      vars.extend(nested);
    }
    CoreType::Unit | CoreType::Named(_) => {}
  }
}

fn collect_var_names(expr: &PnixExpr, vars: &mut Vec<String>) {
  match expr {
    PnixExpr::Var(name) => vars.push(name.clone()),
    PnixExpr::StringInterp(parts) => {
      for part in parts {
        if let crate::lang::pnix::syntax::StringInterpPart::Expr(expr) = part {
          collect_var_names(expr, vars);
        }
      }
    }
    PnixExpr::Let { bindings, body } => {
      for binding in bindings {
        match binding {
          crate::lang::pnix::syntax::PnixLetBinding::Binding { pattern: _, value } => {
            collect_var_names(value, vars);
          }
          crate::lang::pnix::syntax::PnixLetBinding::Inherit { from, names: _ } => {
            if let Some(expr) = from {
              collect_var_names(expr, vars);
            }
          }
        }
      }
      collect_var_names(body, vars);
    }
    PnixExpr::If { cond, then_, else_ } => {
      collect_var_names(cond, vars);
      collect_var_names(then_, vars);
      collect_var_names(else_, vars);
    }
    PnixExpr::Lambda { param: _, body } => {
      collect_var_names(body, vars);
    }
    PnixExpr::Apply { func, arg } => {
      collect_var_names(func, vars);
      collect_var_names(arg, vars);
    }
    PnixExpr::AttrSet { items, .. } => {
      for item in items {
        match item {
          crate::lang::pnix::syntax::PnixAttrItem::Assign {
            key_path: _, value, ..
          } => {
            collect_var_names(value, vars);
          }
          crate::lang::pnix::syntax::PnixAttrItem::DynamicAssign {
            key_path, value, ..
          } => {
            for seg in key_path {
              if let crate::lang::pnix::syntax::AttrKeySegment::Dynamic(expr) = seg {
                collect_var_names(expr, vars);
              }
            }
            collect_var_names(value, vars);
          }
          crate::lang::pnix::syntax::PnixAttrItem::Inherit { from, names: _, .. } => {
            if let Some(expr) = from {
              collect_var_names(expr, vars);
            }
          }
        }
      }
    }
    PnixExpr::List(items) => {
      for item in items {
        collect_var_names(item, vars);
      }
    }
    PnixExpr::Select { base, .. } => collect_var_names(base, vars),
    PnixExpr::SelectOrDefault { base, default, .. } => {
      collect_var_names(base, vars);
      collect_var_names(default, vars);
    }
    PnixExpr::Index { base, index } => {
      collect_var_names(base, vars);
      collect_var_names(index, vars);
    }
    PnixExpr::Binary { lhs, rhs, .. } => {
      collect_var_names(lhs, vars);
      collect_var_names(rhs, vars);
    }
    PnixExpr::Unary { arg, .. } => {
      collect_var_names(arg, vars);
    }
    PnixExpr::Construct { args, .. } => {
      for arg in args {
        collect_var_names(arg, vars);
      }
    }
    PnixExpr::Match { scrutinee, arms } => {
      collect_var_names(scrutinee, vars);
      for arm in arms {
        if let Some(expr) = &arm.guard {
          collect_var_names(expr, vars);
        }
        collect_var_names(&arm.body, vars);
      }
    }
    PnixExpr::Import { path } => collect_var_names(path, vars),
    PnixExpr::With { env, body } => {
      collect_var_names(env, vars);
      collect_var_names(body, vars);
    }
    PnixExpr::Assert { cond, body } => {
      collect_var_names(cond, vars);
      collect_var_names(body, vars);
    }
    PnixExpr::HasAttr { base, .. } => collect_var_names(base, vars),
    PnixExpr::DynamicHasAttr { base, attr_expr } => {
      collect_var_names(base, vars);
      collect_var_names(attr_expr, vars);
    }
    PnixExpr::DynamicSelect { base, attr_expr } => {
      collect_var_names(base, vars);
      collect_var_names(attr_expr, vars);
    }
    PnixExpr::DynamicSelectOrDefault {
      base,
      attr_expr,
      default,
    } => {
      collect_var_names(base, vars);
      collect_var_names(attr_expr, vars);
      collect_var_names(default, vars);
    }
    PnixExpr::Int(_)
    | PnixExpr::Float(_)
    | PnixExpr::Bool(_)
    | PnixExpr::Null
    | PnixExpr::String(_)
    | PnixExpr::Path(_) => {}
  }
}

fn validate_apply_arities(expr: &PnixExpr, pipeline: &LayerPipeline) -> Result<(), PnixError> {
  let mut func_arity: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
  for (token, rule) in &pipeline.typing.rules {
    let Some(arity) = typing_rule_arity(rule) else {
      continue;
    };
    if let Some(func) = pipeline.desugar.rules.get(token) {
      func_arity.insert(func.clone(), arity);
    }
  }
  if func_arity.is_empty() {
    return Ok(());
  }

  let mut errors = Vec::new();
  collect_invalid_apply_arities(expr, &func_arity, &mut errors);
  if errors.is_empty() {
    Ok(())
  } else {
    errors.sort();
    Err(PnixError::TypeError {
      message: format!("invalid operator application arity: {}", errors.join(", ")),
      span: None,
    })
  }
}

fn collect_invalid_apply_arities(
  expr: &PnixExpr,
  func_arity: &std::collections::BTreeMap<String, usize>,
  errors: &mut Vec<String>,
) {
  match expr {
    PnixExpr::Apply { .. } => {
      let (head, args) = flatten_apply(expr);
      if let PnixExpr::Var(name) = head {
        if let Some(&arity) = func_arity.get(name) {
          if args.len() > arity {
            errors.push(format!("{name} expects {arity} args, got {}", args.len()));
          }
        }
      }
      collect_invalid_apply_children(head, &args, func_arity, errors);
    }
    PnixExpr::StringInterp(parts) => {
      for part in parts {
        if let crate::lang::pnix::syntax::StringInterpPart::Expr(expr) = part {
          collect_invalid_apply_arities(expr, func_arity, errors);
        }
      }
    }
    PnixExpr::Let { bindings, body } => {
      for binding in bindings {
        match binding {
          crate::lang::pnix::syntax::PnixLetBinding::Binding { pattern: _, value } => {
            collect_invalid_apply_arities(value, func_arity, errors);
          }
          crate::lang::pnix::syntax::PnixLetBinding::Inherit { from, names: _ } => {
            if let Some(expr) = from {
              collect_invalid_apply_arities(expr, func_arity, errors);
            }
          }
        }
      }
      collect_invalid_apply_arities(body, func_arity, errors);
    }
    PnixExpr::If { cond, then_, else_ } => {
      collect_invalid_apply_arities(cond, func_arity, errors);
      collect_invalid_apply_arities(then_, func_arity, errors);
      collect_invalid_apply_arities(else_, func_arity, errors);
    }
    PnixExpr::Lambda { param: _, body } => {
      collect_invalid_apply_arities(body, func_arity, errors);
    }
    PnixExpr::AttrSet { items, .. } => {
      for item in items {
        match item {
          crate::lang::pnix::syntax::PnixAttrItem::Assign {
            key_path: _, value, ..
          } => {
            collect_invalid_apply_arities(value, func_arity, errors);
          }
          crate::lang::pnix::syntax::PnixAttrItem::DynamicAssign {
            key_path, value, ..
          } => {
            for seg in key_path {
              if let crate::lang::pnix::syntax::AttrKeySegment::Dynamic(expr) = seg {
                collect_invalid_apply_arities(expr, func_arity, errors);
              }
            }
            collect_invalid_apply_arities(value, func_arity, errors);
          }
          crate::lang::pnix::syntax::PnixAttrItem::Inherit { from, names: _, .. } => {
            if let Some(expr) = from {
              collect_invalid_apply_arities(expr, func_arity, errors);
            }
          }
        }
      }
    }
    PnixExpr::List(items) => {
      for item in items {
        collect_invalid_apply_arities(item, func_arity, errors);
      }
    }
    PnixExpr::Select { base, .. } => collect_invalid_apply_arities(base, func_arity, errors),
    PnixExpr::SelectOrDefault { base, default, .. } => {
      collect_invalid_apply_arities(base, func_arity, errors);
      collect_invalid_apply_arities(default, func_arity, errors);
    }
    PnixExpr::Index { base, index } => {
      collect_invalid_apply_arities(base, func_arity, errors);
      collect_invalid_apply_arities(index, func_arity, errors);
    }
    PnixExpr::Binary { lhs, rhs, .. } => {
      collect_invalid_apply_arities(lhs, func_arity, errors);
      collect_invalid_apply_arities(rhs, func_arity, errors);
    }
    PnixExpr::Unary { arg, .. } => {
      collect_invalid_apply_arities(arg, func_arity, errors);
    }
    PnixExpr::Match { scrutinee, arms } => {
      collect_invalid_apply_arities(scrutinee, func_arity, errors);
      for arm in arms {
        if let Some(expr) = &arm.guard {
          collect_invalid_apply_arities(expr, func_arity, errors);
        }
        collect_invalid_apply_arities(&arm.body, func_arity, errors);
      }
    }
    PnixExpr::Import { path } => collect_invalid_apply_arities(path, func_arity, errors),
    PnixExpr::With { env, body } => {
      collect_invalid_apply_arities(env, func_arity, errors);
      collect_invalid_apply_arities(body, func_arity, errors);
    }
    PnixExpr::Assert { cond, body } => {
      collect_invalid_apply_arities(cond, func_arity, errors);
      collect_invalid_apply_arities(body, func_arity, errors);
    }
    PnixExpr::HasAttr { base, .. } => collect_invalid_apply_arities(base, func_arity, errors),
    PnixExpr::DynamicHasAttr { base, attr_expr } => {
      collect_invalid_apply_arities(base, func_arity, errors);
      collect_invalid_apply_arities(attr_expr, func_arity, errors);
    }
    PnixExpr::DynamicSelect { base, attr_expr } => {
      collect_invalid_apply_arities(base, func_arity, errors);
      collect_invalid_apply_arities(attr_expr, func_arity, errors);
    }
    PnixExpr::DynamicSelectOrDefault {
      base,
      attr_expr,
      default,
    } => {
      collect_invalid_apply_arities(base, func_arity, errors);
      collect_invalid_apply_arities(attr_expr, func_arity, errors);
      collect_invalid_apply_arities(default, func_arity, errors);
    }
    PnixExpr::Construct { args, .. } => {
      for arg in args {
        collect_invalid_apply_arities(arg, func_arity, errors);
      }
    }
    PnixExpr::Int(_)
    | PnixExpr::Float(_)
    | PnixExpr::Bool(_)
    | PnixExpr::Null
    | PnixExpr::String(_)
    | PnixExpr::Path(_)
    | PnixExpr::Var(_) => {}
  }
}

fn flatten_apply<'a>(expr: &'a PnixExpr) -> (&'a PnixExpr, Vec<&'a PnixExpr>) {
  let mut args = Vec::new();
  let mut head = expr;
  while let PnixExpr::Apply { func, arg } = head {
    args.push(arg.as_ref());
    head = func.as_ref();
  }
  args.reverse();
  (head, args)
}

fn collect_invalid_apply_children(
  head: &PnixExpr,
  args: &[&PnixExpr],
  func_arity: &std::collections::BTreeMap<String, usize>,
  errors: &mut Vec<String>,
) {
  collect_invalid_apply_arities(head, func_arity, errors);
  for arg in args {
    collect_invalid_apply_arities(arg, func_arity, errors);
  }
}

fn normalize_rat_make(num: i64, den: i64) -> (i64, i64) {
  if den == 0 {
    return (num, den);
  }
  let mut n = num as i128;
  let mut d = den as i128;
  let g = gcd_i128(n.abs(), d.abs());
  if g != 0 {
    n /= g;
    d /= g;
  }
  if d < 0 {
    n = -n;
    d = -d;
  }
  (n as i64, d as i64)
}

fn gcd_i128(mut a: i128, mut b: i128) -> i128 {
  while b != 0 {
    let r = a % b;
    a = b;
    b = r;
  }
  a
}

fn expr_to_path(expr: &PnixExpr) -> Option<String> {
  match expr {
    PnixExpr::Var(name) => Some(name.clone()),
    PnixExpr::Construct { variant, args } if args.is_empty() => Some(variant.clone()),
    PnixExpr::Select { base, attr } => {
      let mut base = expr_to_path(base)?;
      base.push('.');
      base.push_str(attr);
      Some(base)
    }
    _ => None,
  }
}

fn desugar_param_pattern(
  pattern: crate::lang::pnix::syntax::PnixParamPattern,
  rules: &std::collections::BTreeMap<String, String>,
) -> crate::lang::pnix::syntax::PnixParamPattern {
  match pattern {
    crate::lang::pnix::syntax::PnixParamPattern::Ident(_) => pattern,
    crate::lang::pnix::syntax::PnixParamPattern::AttrSet { fields, ellipsis } => {
      crate::lang::pnix::syntax::PnixParamPattern::AttrSet {
        fields: fields
          .into_iter()
          .map(|field| crate::lang::pnix::syntax::PnixPatternField {
            name: field.name,
            default: field.default.map(|expr| desugar_expr(expr, rules)),
          })
          .collect(),
        ellipsis,
      }
    }
    crate::lang::pnix::syntax::PnixParamPattern::List(list) => {
      crate::lang::pnix::syntax::PnixParamPattern::List(list)
    }
    crate::lang::pnix::syntax::PnixParamPattern::AttrSetWithBind {
      bind_name,
      fields,
      ellipsis,
    } => crate::lang::pnix::syntax::PnixParamPattern::AttrSetWithBind {
      bind_name,
      fields: fields
        .into_iter()
        .map(|field| crate::lang::pnix::syntax::PnixPatternField {
          name: field.name,
          default: field.default.map(|expr| desugar_expr(expr, rules)),
        })
        .collect(),
      ellipsis,
    },
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::diagnostics::Span;
  use crate::lang::pnix::syntax::{AttrKeySegment, PnixMatchArm, PnixPattern, StringInterpPart};

  fn strip_attr_key_segment(segment: &AttrKeySegment) -> AttrKeySegment {
    match segment {
      AttrKeySegment::Static(name) => AttrKeySegment::Static(name.clone()),
      AttrKeySegment::Dynamic(expr) => AttrKeySegment::Dynamic(Arc::new(strip_expr_span(expr))),
    }
  }

  fn strip_attr_item_span(item: &PnixAttrItem) -> PnixAttrItem {
    match item {
      PnixAttrItem::Assign {
        key_path, value, ..
      } => PnixAttrItem::Assign {
        key_path: key_path.clone(),
        value: strip_expr_span(value),
        span: Span::empty(),
      },
      PnixAttrItem::DynamicAssign {
        key_path, value, ..
      } => PnixAttrItem::DynamicAssign {
        key_path: key_path.iter().map(strip_attr_key_segment).collect(),
        value: strip_expr_span(value),
        span: Span::empty(),
      },
      PnixAttrItem::Inherit { from, names, .. } => PnixAttrItem::Inherit {
        from: from.as_ref().map(|expr| Arc::new(strip_expr_span(expr))),
        names: names.clone(),
        span: Span::empty(),
      },
    }
  }

  fn strip_string_interp_part(part: &StringInterpPart) -> StringInterpPart {
    match part {
      StringInterpPart::Lit(value) => StringInterpPart::Lit(value.clone()),
      StringInterpPart::Expr(expr) => StringInterpPart::Expr(Arc::new(strip_expr_span(expr))),
    }
  }

  fn strip_pattern_span(pattern: &PnixPattern) -> PnixPattern {
    match pattern {
      PnixPattern::Wildcard => PnixPattern::Wildcard,
      PnixPattern::Var(name) => PnixPattern::Var(name.clone()),
      PnixPattern::Literal(value) => PnixPattern::Literal(value.clone()),
      PnixPattern::AttrSet { fields, ellipsis } => PnixPattern::AttrSet {
        fields: fields
          .iter()
          .map(|field| PnixMatchAttrField {
            name: field.name.clone(),
            pattern: field
              .pattern
              .as_ref()
              .map(|pat| Arc::new(strip_pattern_span(pat))),
          })
          .collect(),
        ellipsis: *ellipsis,
      },
      PnixPattern::List(list) => PnixPattern::List(PnixMatchListPattern {
        items: list.items.iter().map(strip_pattern_span).collect(),
        tail: list.tail.clone(),
      }),
      PnixPattern::Constructor { variant, args } => PnixPattern::Constructor {
        variant: variant.clone(),
        args: args.iter().map(strip_pattern_span).collect(),
      },
    }
  }

  fn strip_pattern_field_span(field: &PnixPatternField) -> PnixPatternField {
    PnixPatternField {
      name: field.name.clone(),
      default: field.default.as_ref().map(strip_expr_span),
    }
  }

  fn strip_param_pattern_span(pattern: &PnixParamPattern) -> PnixParamPattern {
    match pattern {
      PnixParamPattern::Ident(name) => PnixParamPattern::Ident(name.clone()),
      PnixParamPattern::AttrSet { fields, ellipsis } => PnixParamPattern::AttrSet {
        fields: fields.iter().map(strip_pattern_field_span).collect(),
        ellipsis: *ellipsis,
      },
      PnixParamPattern::List(list) => PnixParamPattern::List(PnixListPattern {
        items: list.items.clone(),
        tail: list.tail.clone(),
      }),
      PnixParamPattern::AttrSetWithBind {
        bind_name,
        fields,
        ellipsis,
      } => PnixParamPattern::AttrSetWithBind {
        bind_name: bind_name.clone(),
        fields: fields.iter().map(strip_pattern_field_span).collect(),
        ellipsis: *ellipsis,
      },
    }
  }

  fn strip_let_binding_span(binding: &PnixLetBinding) -> PnixLetBinding {
    match binding {
      PnixLetBinding::Binding { pattern, value } => PnixLetBinding::Binding {
        pattern: strip_param_pattern_span(pattern),
        value: strip_expr_span(value),
      },
      PnixLetBinding::Inherit { from, names } => PnixLetBinding::Inherit {
        from: from.as_ref().map(|expr| Arc::new(strip_expr_span(expr))),
        names: names.clone(),
      },
    }
  }

  fn strip_match_arm_span(arm: &PnixMatchArm) -> PnixMatchArm {
    PnixMatchArm {
      pattern: strip_pattern_span(&arm.pattern),
      guard: arm
        .guard
        .as_ref()
        .map(|expr| Arc::new(strip_expr_span(expr))),
      body: strip_expr_span(&arm.body),
    }
  }

  fn strip_expr_span(expr: &PnixExpr) -> PnixExpr {
    match expr {
      PnixExpr::Int(v) => PnixExpr::Int(*v),
      PnixExpr::Float(v) => PnixExpr::Float(*v),
      PnixExpr::Bool(v) => PnixExpr::Bool(*v),
      PnixExpr::Null => PnixExpr::Null,
      PnixExpr::String(value) => PnixExpr::String(value.clone()),
      PnixExpr::StringInterp(parts) => {
        PnixExpr::StringInterp(parts.iter().map(strip_string_interp_part).collect())
      }
      PnixExpr::Path(path) => PnixExpr::Path(path.clone()),
      PnixExpr::Var(name) => PnixExpr::Var(name.clone()),
      PnixExpr::Let { bindings, body } => PnixExpr::Let {
        bindings: bindings.iter().map(strip_let_binding_span).collect(),
        body: Arc::new(strip_expr_span(body)),
      },
      PnixExpr::If { cond, then_, else_ } => PnixExpr::If {
        cond: Arc::new(strip_expr_span(cond)),
        then_: Arc::new(strip_expr_span(then_)),
        else_: Arc::new(strip_expr_span(else_)),
      },
      PnixExpr::Lambda { param, body } => PnixExpr::Lambda {
        param: strip_param_pattern_span(param),
        body: Arc::new(strip_expr_span(body)),
      },
      PnixExpr::Apply { func, arg } => PnixExpr::Apply {
        func: Arc::new(strip_expr_span(func)),
        arg: Arc::new(strip_expr_span(arg)),
      },
      PnixExpr::AttrSet { items, recursive } => PnixExpr::AttrSet {
        items: items.iter().map(strip_attr_item_span).collect(),
        recursive: *recursive,
      },
      PnixExpr::List(items) => PnixExpr::List(items.iter().map(strip_expr_span).collect()),
      PnixExpr::Select { base, attr } => PnixExpr::Select {
        base: Arc::new(strip_expr_span(base)),
        attr: attr.clone(),
      },
      PnixExpr::SelectOrDefault {
        base,
        attr,
        default,
      } => PnixExpr::SelectOrDefault {
        base: Arc::new(strip_expr_span(base)),
        attr: attr.clone(),
        default: Arc::new(strip_expr_span(default)),
      },
      PnixExpr::Index { base, index } => PnixExpr::Index {
        base: Arc::new(strip_expr_span(base)),
        index: Arc::new(strip_expr_span(index)),
      },
      PnixExpr::Binary { op, lhs, rhs } => PnixExpr::Binary {
        op,
        lhs: Arc::new(strip_expr_span(lhs)),
        rhs: Arc::new(strip_expr_span(rhs)),
      },
      PnixExpr::Unary { op, arg } => PnixExpr::Unary {
        op,
        arg: Arc::new(strip_expr_span(arg)),
      },
      PnixExpr::Construct { variant, args } => PnixExpr::Construct {
        variant: variant.clone(),
        args: args.iter().map(strip_expr_span).collect(),
      },
      PnixExpr::Match { scrutinee, arms } => PnixExpr::Match {
        scrutinee: Arc::new(strip_expr_span(scrutinee)),
        arms: arms.iter().map(strip_match_arm_span).collect(),
      },
      PnixExpr::Import { path } => PnixExpr::Import {
        path: Arc::new(strip_expr_span(path)),
      },
      PnixExpr::With { env, body } => PnixExpr::With {
        env: Arc::new(strip_expr_span(env)),
        body: Arc::new(strip_expr_span(body)),
      },
      PnixExpr::Assert { cond, body } => PnixExpr::Assert {
        cond: Arc::new(strip_expr_span(cond)),
        body: Arc::new(strip_expr_span(body)),
      },
      PnixExpr::HasAttr { base, attr } => PnixExpr::HasAttr {
        base: Arc::new(strip_expr_span(base)),
        attr: attr.clone(),
      },
      PnixExpr::DynamicHasAttr { base, attr_expr } => PnixExpr::DynamicHasAttr {
        base: Arc::new(strip_expr_span(base)),
        attr_expr: Arc::new(strip_expr_span(attr_expr)),
      },
      PnixExpr::DynamicSelect { base, attr_expr } => PnixExpr::DynamicSelect {
        base: Arc::new(strip_expr_span(base)),
        attr_expr: Arc::new(strip_expr_span(attr_expr)),
      },
      PnixExpr::DynamicSelectOrDefault {
        base,
        attr_expr,
        default,
      } => PnixExpr::DynamicSelectOrDefault {
        base: Arc::new(strip_expr_span(base)),
        attr_expr: Arc::new(strip_expr_span(attr_expr)),
        default: Arc::new(strip_expr_span(default)),
      },
    }
  }

  #[test]
  fn parse_seto_layer_call_string_arg() {
    let expr = parse_expr("seto.ui.scene \"hi\"").expect("parse expr");
    let call = parse_seto_layer_call(&expr).expect("seto call");
    assert_eq!(call.layer, "ui.scene");
    assert_eq!(call.args.len(), 1);
    match &call.args[0] {
      PnixExpr::String(value) => assert_eq!(value, "hi"),
      other => panic!("expected string arg, got {:?}", other),
    }
  }

  #[test]
  fn parse_seto_layer_call_attrset_arg() {
    let expr = parse_expr("seto.x3d { Shape = { size = 1; }; }").expect("parse expr");
    let call = parse_seto_layer_call(&expr).expect("seto call");
    assert_eq!(call.layer, "x3d");
    assert_eq!(call.args.len(), 1);
    assert!(matches!(call.args[0], PnixExpr::AttrSet { .. }));
  }

  #[test]
  fn parse_seto_layer_call_two_args() {
    let expr =
      parse_expr("seto.x3d { timeline = { n1 = 0; }; } { Shape = { }; }").expect("parse expr");
    let call = parse_seto_layer_call(&expr).expect("seto call");
    assert_eq!(call.layer, "x3d");
    assert_eq!(call.args.len(), 2);
    assert!(matches!(call.args[0], PnixExpr::AttrSet { .. }));
    assert!(matches!(call.args[1], PnixExpr::AttrSet { .. }));
  }

  #[test]
  fn attrset_to_layer_expr_shape_geometry_box() {
    let expr = parse_expr("{ Shape = { geometry.Box.size = [1 2 3]; }; }").expect("parse expr");
    let layer = attrset_to_layer_expr(&expr).expect("layer expr");
    let PnixExpr::Apply { func, arg } = layer else {
      panic!("expected apply");
    };
    // Nix-compat: capitalized layer head is now a `Var`, not a
    // constructor — see `parse_layer_attrset_expr_basic`.
    assert!(matches!(
      func.as_ref(),
      PnixExpr::Var(name) if name == "Shape"
    ));
    let PnixExpr::AttrSet { items, .. } = *arg else {
      panic!("expected attrset body");
    };
    assert_eq!(items.len(), 1);
    match &items[0] {
      PnixAttrItem::Assign {
        key_path, value, ..
      } => {
        assert_eq!(key_path, &vec!["geometry".to_string()]);
        let PnixExpr::Apply { func, arg } = value else {
          panic!("expected geometry constructor");
        };
        assert!(matches!(
          func.as_ref(),
          PnixExpr::Var(name) if name == "Box"
        ));
        let PnixExpr::AttrSet { items, .. } = &**arg else {
          panic!("expected box attrset");
        };
        assert_eq!(items.len(), 1);
        match &items[0] {
          PnixAttrItem::Assign {
            key_path, value, ..
          } => {
            assert_eq!(key_path, &vec!["size".to_string()]);
            assert!(matches!(value, PnixExpr::List(values) if values.len() == 3));
          }
          _ => panic!("expected size assignment"),
        }
      }
      _ => panic!("expected geometry assignment"),
    }
  }

  #[test]
  fn attrset_to_layer_expr_animation_target() {
    let expr = parse_expr("{ animation.target = scene; }").expect("parse expr");
    let layer = attrset_to_layer_expr(&expr).expect("layer expr");
    let PnixExpr::Apply { func, arg } = layer else {
      panic!("expected apply");
    };
    assert!(matches!(
      func.as_ref(),
      PnixExpr::Var(ref name) if name == "animation"
    ));
    let PnixExpr::AttrSet { items, .. } = *arg else {
      panic!("expected attrset body");
    };
    assert_eq!(items.len(), 1);
    match &items[0] {
      PnixAttrItem::Assign {
        key_path, value, ..
      } => {
        assert_eq!(key_path, &vec!["target".to_string()]);
        assert!(matches!(value, PnixExpr::Var(ref name) if name == "scene"));
      }
      _ => panic!("expected target assignment"),
    }
  }

  #[test]
  fn parse_layer_attrset_expr_basic() {
    let expr = parse_expr("{ Shape = { }; }").expect("parse expr");
    let layer = parse_layer_attrset_expr(&expr, None).expect("layer expr");
    // After Nix-compat fix, bare uppercase identifiers in layer attrsets
    // become `Var(...)` (matching what the language parser does for any
    // capitalized identifier). The constructor surface only fires for
    // explicit `Name(args)` syntax.
    assert!(matches!(
      layer,
      PnixExpr::Apply {
        func,
        arg: _,
      } if matches!(
        func.as_ref(),
        PnixExpr::Var(name) if name == "Shape"
      )
    ));
  }

  #[test]
  fn parse_seto_layer_block_attrset_body() {
    let expr =
      parse_expr("seto.x3d { timeline = { n1 = 0; }; } { Shape = { }; }").expect("parse expr");
    let block = parse_seto_layer_block(&expr).expect("seto block");
    assert_eq!(block.layer, "x3d");
    assert!(matches!(block.params, Some(PnixExpr::AttrSet { .. })));
    assert!(matches!(block.body, SetoLayerBody::AttrSet(_)));
  }

  #[test]
  fn parse_seto_layer_block_string_body() {
    let expr = parse_expr("seto.ui.scene { } \"scene\"").expect("parse expr");
    let block = parse_seto_layer_block(&expr).expect("seto block");
    assert_eq!(block.layer, "ui.scene");
    assert!(matches!(block.params, Some(PnixExpr::AttrSet { .. })));
    assert!(matches!(block.body, SetoLayerBody::String(ref s) if s == "scene"));
  }

  #[test]
  fn attrset_layer_expr_matches_string_layer_expr() {
    let attrset = parse_expr("{ Shape = { geometry.Box.size = [1 2 3]; }; }").expect("parse expr");
    let attrset_expr = parse_layer_attrset_expr(&attrset, None).expect("layer expr");
    let string_expr = parse_layer_expr("Shape { geometry = Box { size = [1 2 3]; }; }", None)
      .expect("parse string layer expr");
    assert_eq!(
      strip_expr_span(&attrset_expr),
      strip_expr_span(&string_expr)
    );
  }
}
