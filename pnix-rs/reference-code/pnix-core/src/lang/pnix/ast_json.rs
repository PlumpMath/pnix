//! Stable JSON projection for the `.px` surface AST.
//!
//! This is deliberately *not* an evaluator. It only turns the parser's
//! `PnixExpr` tree into first-class data with a stable `kind` shape so later
//! substrate work can inspect `.px` programs without re-encoding semantic law
//! in Rust mirrors.

use serde_json::{json, Value};

use super::parser::parse_expr;
use super::syntax::{
  AttrKeySegment, PnixAttrItem, PnixExpr, PnixLetBinding, PnixListPattern, PnixLiteralPattern,
  PnixMatchArm, PnixMatchAttrField, PnixMatchListPattern, PnixParamPattern, PnixPath, PnixPathBase,
  PnixPattern, PnixPatternField, StringInterpPart,
};
use super::PnixError;

/// Version tag for the host-projected `.px` AST data shape.
///
/// Bump this only when downstream consumers must consciously migrate.
pub const PNIX_AST_JSON_FORMAT: &str = "pnix-ast-json-v0";

/// Parse a `.px` expression and return its stable JSON AST envelope.
///
/// This is a parser/transport boundary only: no evaluation, no imports, no
/// owner-law decisions.
pub fn parse_expr_to_ast_json(source: &str) -> Result<Value, PnixError> {
  let expr = parse_expr(source)?;
  Ok(json!({
    "format": PNIX_AST_JSON_FORMAT,
    "language": "pnix-surface",
    "root": pnix_expr_to_ast_json(&expr),
  }))
}

/// Project an already parsed `.px` expression into stable JSON AST data.
pub fn pnix_expr_to_ast_json(expr: &PnixExpr) -> Value {
  match expr {
    PnixExpr::Int(value) => json!({ "kind": "int", "value": value }),
    PnixExpr::Float(value) => json!({ "kind": "float", "value": value }),
    PnixExpr::Bool(value) => json!({ "kind": "bool", "value": value }),
    PnixExpr::Null => json!({ "kind": "null" }),
    PnixExpr::String(value) => json!({ "kind": "string", "value": value }),
    PnixExpr::StringInterp(parts) => json!({
      "kind": "string_interp",
      "parts": parts.iter().map(string_interp_part_to_ast_json).collect::<Vec<_>>(),
    }),
    PnixExpr::Path(path) => json!({
      "kind": "path",
      "path": path_to_ast_json(path),
    }),
    PnixExpr::Var(name) => json!({ "kind": "var", "name": name }),
    PnixExpr::Let { bindings, body } => json!({
      "kind": "let",
      "bindings": bindings.iter().map(let_binding_to_ast_json).collect::<Vec<_>>(),
      "body": pnix_expr_to_ast_json(body),
    }),
    PnixExpr::If { cond, then_, else_ } => json!({
      "kind": "if",
      "cond": pnix_expr_to_ast_json(cond),
      "then": pnix_expr_to_ast_json(then_),
      "else": pnix_expr_to_ast_json(else_),
    }),
    PnixExpr::Lambda { param, body } => json!({
      "kind": "lambda",
      "param": param_pattern_to_ast_json(param),
      "body": pnix_expr_to_ast_json(body),
    }),
    PnixExpr::Apply { func, arg } => json!({
      "kind": "apply",
      "func": pnix_expr_to_ast_json(func),
      "arg": pnix_expr_to_ast_json(arg),
    }),
    PnixExpr::AttrSet { items, recursive } => json!({
      "kind": "attr_set",
      "recursive": recursive,
      "items": items.iter().map(attr_item_to_ast_json).collect::<Vec<_>>(),
    }),
    PnixExpr::List(items) => json!({
      "kind": "list",
      "items": items.iter().map(pnix_expr_to_ast_json).collect::<Vec<_>>(),
    }),
    PnixExpr::Select { base, attr } => json!({
      "kind": "select",
      "base": pnix_expr_to_ast_json(base),
      "attr": attr,
    }),
    PnixExpr::SelectOrDefault {
      base,
      attr,
      default,
    } => json!({
      "kind": "select_or_default",
      "base": pnix_expr_to_ast_json(base),
      "attr": attr,
      "default": pnix_expr_to_ast_json(default),
    }),
    PnixExpr::Index { base, index } => json!({
      "kind": "index",
      "base": pnix_expr_to_ast_json(base),
      "index": pnix_expr_to_ast_json(index),
    }),
    PnixExpr::Binary { op, lhs, rhs } => json!({
      "kind": "binary",
      "op": op,
      "lhs": pnix_expr_to_ast_json(lhs),
      "rhs": pnix_expr_to_ast_json(rhs),
    }),
    PnixExpr::Unary { op, arg } => json!({
      "kind": "unary",
      "op": op,
      "arg": pnix_expr_to_ast_json(arg),
    }),
    PnixExpr::Construct { variant, args } => json!({
      "kind": "construct",
      "variant": variant,
      "args": args.iter().map(pnix_expr_to_ast_json).collect::<Vec<_>>(),
    }),
    PnixExpr::Match { scrutinee, arms } => json!({
      "kind": "match",
      "scrutinee": pnix_expr_to_ast_json(scrutinee),
      "arms": arms.iter().map(match_arm_to_ast_json).collect::<Vec<_>>(),
    }),
    PnixExpr::Import { path } => json!({
      "kind": "import",
      "path": pnix_expr_to_ast_json(path),
    }),
    PnixExpr::With { env, body } => json!({
      "kind": "with",
      "env": pnix_expr_to_ast_json(env),
      "body": pnix_expr_to_ast_json(body),
    }),
    PnixExpr::Assert { cond, body } => json!({
      "kind": "assert",
      "cond": pnix_expr_to_ast_json(cond),
      "body": pnix_expr_to_ast_json(body),
    }),
    PnixExpr::HasAttr { base, attr } => json!({
      "kind": "has_attr",
      "base": pnix_expr_to_ast_json(base),
      "attr": attr,
    }),
    PnixExpr::DynamicHasAttr { base, attr_expr } => json!({
      "kind": "dynamic_has_attr",
      "base": pnix_expr_to_ast_json(base),
      "attr_expr": pnix_expr_to_ast_json(attr_expr),
    }),
    PnixExpr::DynamicSelect { base, attr_expr } => json!({
      "kind": "dynamic_select",
      "base": pnix_expr_to_ast_json(base),
      "attr_expr": pnix_expr_to_ast_json(attr_expr),
    }),
    PnixExpr::DynamicSelectOrDefault {
      base,
      attr_expr,
      default,
    } => json!({
      "kind": "dynamic_select_or_default",
      "base": pnix_expr_to_ast_json(base),
      "attr_expr": pnix_expr_to_ast_json(attr_expr),
      "default": pnix_expr_to_ast_json(default),
    }),
  }
}

fn string_interp_part_to_ast_json(part: &StringInterpPart) -> Value {
  match part {
    StringInterpPart::Lit(value) => json!({
      "kind": "literal",
      "value": value,
    }),
    StringInterpPart::Expr(expr) => json!({
      "kind": "expr",
      "expr": pnix_expr_to_ast_json(expr),
    }),
  }
}

fn path_to_ast_json(path: &PnixPath) -> Value {
  match path {
    PnixPath::Relative(value) => json!({
      "kind": "relative",
      "value": value,
    }),
    PnixPath::Absolute(value) => json!({
      "kind": "absolute",
      "value": value,
    }),
    PnixPath::Search(value) => json!({
      "kind": "search",
      "value": value,
    }),
    PnixPath::Home(value) => json!({
      "kind": "home",
      "value": value,
    }),
    PnixPath::Interpolated { base, parts } => json!({
      "kind": "interpolated",
      "base": path_base_to_str(*base),
      "parts": parts.iter().map(string_interp_part_to_ast_json).collect::<Vec<_>>(),
    }),
  }
}

fn path_base_to_str(base: PnixPathBase) -> &'static str {
  match base {
    PnixPathBase::Relative => "relative",
    PnixPathBase::Absolute => "absolute",
    PnixPathBase::Home => "home",
  }
}

fn let_binding_to_ast_json(binding: &PnixLetBinding) -> Value {
  match binding {
    PnixLetBinding::Binding { pattern, value } => json!({
      "kind": "binding",
      "pattern": param_pattern_to_ast_json(pattern),
      "value": pnix_expr_to_ast_json(value),
    }),
    PnixLetBinding::Inherit { from, names } => json!({
      "kind": "inherit",
      "from": from.as_ref().map(|expr| pnix_expr_to_ast_json(expr)),
      "names": names,
    }),
  }
}

fn param_pattern_to_ast_json(pattern: &PnixParamPattern) -> Value {
  match pattern {
    PnixParamPattern::Ident(name) => json!({
      "kind": "ident",
      "name": name,
    }),
    PnixParamPattern::AttrSet { fields, ellipsis } => json!({
      "kind": "attr_set",
      "fields": fields.iter().map(pattern_field_to_ast_json).collect::<Vec<_>>(),
      "ellipsis": ellipsis,
    }),
    PnixParamPattern::List(list) => json!({
      "kind": "list",
      "pattern": list_pattern_to_ast_json(list),
    }),
    PnixParamPattern::AttrSetWithBind {
      bind_name,
      fields,
      ellipsis,
    } => json!({
      "kind": "attr_set_with_bind",
      "bind_name": bind_name,
      "fields": fields.iter().map(pattern_field_to_ast_json).collect::<Vec<_>>(),
      "ellipsis": ellipsis,
    }),
  }
}

fn pattern_field_to_ast_json(field: &PnixPatternField) -> Value {
  json!({
    "name": field.name,
    "default": field.default.as_ref().map(pnix_expr_to_ast_json),
  })
}

fn list_pattern_to_ast_json(pattern: &PnixListPattern) -> Value {
  json!({
    "items": pattern.items,
    "tail": pattern.tail,
  })
}

fn attr_item_to_ast_json(item: &PnixAttrItem) -> Value {
  match item {
    PnixAttrItem::Assign {
      key_path, value, ..
    } => json!({
      "kind": "assign",
      "key_path": key_path,
      "value": pnix_expr_to_ast_json(value),
    }),
    PnixAttrItem::DynamicAssign {
      key_path, value, ..
    } => json!({
      "kind": "dynamic_assign",
      "key_path": key_path.iter().map(attr_key_segment_to_ast_json).collect::<Vec<_>>(),
      "value": pnix_expr_to_ast_json(value),
    }),
    PnixAttrItem::Inherit { from, names, .. } => json!({
      "kind": "inherit",
      "from": from.as_ref().map(|expr| pnix_expr_to_ast_json(expr)),
      "names": names,
    }),
  }
}

fn attr_key_segment_to_ast_json(segment: &AttrKeySegment) -> Value {
  match segment {
    AttrKeySegment::Static(name) => json!({
      "kind": "static",
      "name": name,
    }),
    AttrKeySegment::Dynamic(expr) => json!({
      "kind": "dynamic",
      "expr": pnix_expr_to_ast_json(expr),
    }),
  }
}

fn match_arm_to_ast_json(arm: &PnixMatchArm) -> Value {
  json!({
    "pattern": match_pattern_to_ast_json(&arm.pattern),
    "guard": arm.guard.as_ref().map(|expr| pnix_expr_to_ast_json(expr)),
    "body": pnix_expr_to_ast_json(&arm.body),
  })
}

fn match_pattern_to_ast_json(pattern: &PnixPattern) -> Value {
  match pattern {
    PnixPattern::Wildcard => json!({ "kind": "wildcard" }),
    PnixPattern::Var(name) => json!({
      "kind": "var",
      "name": name,
    }),
    PnixPattern::Literal(literal) => json!({
      "kind": "literal",
      "literal": literal_pattern_to_ast_json(literal),
    }),
    PnixPattern::AttrSet { fields, ellipsis } => json!({
      "kind": "attr_set",
      "fields": fields.iter().map(match_attr_field_to_ast_json).collect::<Vec<_>>(),
      "ellipsis": ellipsis,
    }),
    PnixPattern::List(pattern) => json!({
      "kind": "list",
      "pattern": match_list_pattern_to_ast_json(pattern),
    }),
    PnixPattern::Constructor { variant, args } => json!({
      "kind": "constructor",
      "variant": variant,
      "args": args.iter().map(match_pattern_to_ast_json).collect::<Vec<_>>(),
    }),
  }
}

fn literal_pattern_to_ast_json(pattern: &PnixLiteralPattern) -> Value {
  match pattern {
    PnixLiteralPattern::Int(value) => json!({
      "kind": "int",
      "value": value,
    }),
    PnixLiteralPattern::Float(value) => json!({
      "kind": "float",
      "value": value,
    }),
    PnixLiteralPattern::Bool(value) => json!({
      "kind": "bool",
      "value": value,
    }),
    PnixLiteralPattern::String(value) => json!({
      "kind": "string",
      "value": value,
    }),
    PnixLiteralPattern::Null => json!({ "kind": "null" }),
  }
}

fn match_attr_field_to_ast_json(field: &PnixMatchAttrField) -> Value {
  json!({
    "name": field.name,
    "pattern": field.pattern.as_ref().map(|pattern| match_pattern_to_ast_json(pattern)),
  })
}

fn match_list_pattern_to_ast_json(pattern: &PnixMatchListPattern) -> Value {
  json!({
    "items": pattern.items.iter().map(match_pattern_to_ast_json).collect::<Vec<_>>(),
    "tail": pattern.tail,
  })
}
