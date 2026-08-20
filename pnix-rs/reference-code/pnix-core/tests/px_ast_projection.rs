//! `.px` AST projection first slice.
//!
//! This test pins the missing core boundary behind the Rust mirror pressure:
//! `.px` source must become stable first-class AST data before the runtime can
//! move more evaluator/receipt machinery into `.px` itself. The host parser is
//! still a bootloader here; no semantic law is evaluated in Rust.

use pnix_core::lang::pnix::{parse_expr_to_ast_json, PNIX_AST_JSON_FORMAT};

#[test]
fn px_source_projects_to_stable_first_class_ast_data() {
  let ast = parse_expr_to_ast_json(
    r#"
    let
      row = {
        cue = "fact:ast-core";
        intent = "refactor";
        weight = 0.85;
      };
    in [ row.cue row.intent row.weight ]
    "#,
  )
  .expect("source parses into AST projection");

  assert_eq!(ast["format"], PNIX_AST_JSON_FORMAT);
  assert_eq!(ast["language"], "pnix-surface");
  assert_eq!(ast["root"]["kind"], "let");

  let bindings = ast["root"]["bindings"].as_array().expect("bindings");
  assert_eq!(bindings.len(), 1);
  assert_eq!(bindings[0]["kind"], "binding");
  assert_eq!(bindings[0]["pattern"]["kind"], "ident");
  assert_eq!(bindings[0]["pattern"]["name"], "row");

  let row_value = &bindings[0]["value"];
  assert_eq!(row_value["kind"], "attr_set");
  assert_eq!(row_value["recursive"], false);
  let row_items = row_value["items"].as_array().expect("attr items");
  assert_eq!(
    row_items
      .iter()
      .map(|item| item["key_path"][0].as_str().unwrap())
      .collect::<Vec<_>>(),
    vec!["cue", "intent", "weight"]
  );

  let body_items = ast["root"]["body"]["items"].as_array().expect("body list");
  assert_eq!(ast["root"]["body"]["kind"], "list");
  assert_eq!(body_items.len(), 3);
  assert_eq!(body_items[0]["kind"], "select");
  assert_eq!(body_items[0]["base"]["kind"], "var");
  assert_eq!(body_items[0]["base"]["name"], "row");
  assert_eq!(body_items[0]["attr"], "cue");
}

#[test]
fn px_ast_projection_is_deterministic_for_same_source() {
  let source = r#"let x = { a = 1; b = [ true false null ]; }; in x.b"#;

  let first = parse_expr_to_ast_json(source).expect("first parse");
  let second = parse_expr_to_ast_json(source).expect("second parse");

  assert_eq!(first, second);
}

#[test]
fn px_ast_projection_keeps_parse_errors_at_parse_boundary() {
  let err = parse_expr_to_ast_json("let x = ; in x").expect_err("invalid source");
  let message = err.to_string();

  assert!(
    message.contains("expected") || message.contains("parse"),
    "parse error should remain a parser boundary error, got: {message}"
  );
}
