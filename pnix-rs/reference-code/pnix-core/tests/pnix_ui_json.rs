//! PNIX UI JSON 테스트: PNIX 표현식을 UI JSON으로 변환하는 테스트
//!
//! PNIX 표현식이 올바르게 UI JSON 형식으로 변환되는지 검증합니다.

use pnix_core::lang::pnix::parse_expr;
use pnix_core::lang::pnix::ui_json::{normalize_pnix_list_separators, pnix_expr_to_json};
use serde_json::json;

#[test]
fn pnix_ui_json_attrset_paths() {
  let expr = parse_expr("{ a = 1; b.c = \"ok\"; }").expect("parse");
  let value = pnix_expr_to_json(&expr).expect("json");
  assert_eq!(value, json!({"a": 1, "b": {"c": "ok"}}));
}

#[test]
fn pnix_ui_json_list() {
  let expr = parse_expr("[1 2 3]").expect("parse");
  let value = pnix_expr_to_json(&expr).expect("json");
  assert_eq!(value, json!([1, 2, 3]));
}

#[test]
fn pnix_ui_json_constructor_kind_scene() {
  let expr = parse_expr("scene { }").expect("parse");
  let value = pnix_expr_to_json(&expr).expect("json");
  assert_eq!(value.get("kind").and_then(|v| v.as_str()), Some("scene"));
}

#[test]
fn pnix_ui_json_apply_requires_attrset() {
  let expr = parse_expr("scene 3").expect("parse");
  let err = pnix_expr_to_json(&expr).expect_err("error");
  assert!(err.contains("requires attrset"));
}

#[test]
fn pnix_ui_json_recursive_attrset_rejected() {
  let expr = parse_expr("rec { a = 1; }").expect("parse");
  let err = pnix_expr_to_json(&expr).expect_err("error");
  assert!(err.contains("recursive attrset"));
}

#[test]
fn normalize_pnix_list_separators_removes_list_semicolons() {
  let src = "[1; 2; 3]";
  assert_eq!(normalize_pnix_list_separators(src), "[1 2 3]");
}

#[test]
fn normalize_pnix_list_separators_preserves_string_semicolons() {
  let src = "[\"a;b\"; \"c\"]";
  assert_eq!(normalize_pnix_list_separators(src), "[\"a;b\" \"c\"]");
}

#[test]
fn normalize_pnix_list_separators_keeps_semicolons_outside_list() {
  let src = "let x = 1; in [2; 3]";
  assert_eq!(normalize_pnix_list_separators(src), "let x = 1; in [2 3]");
}
