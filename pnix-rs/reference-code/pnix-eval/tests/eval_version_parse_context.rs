//! Regression cover for `parseDrvName` / `splitVersion`
//! string-context propagation.
//!
//! 2026-05-05 audit findings (slice #61):
//!
//!   `builtins.parseDrvName "hello-1.0${./p}"` returned an
//!   attrset `{ name = "hello"; version = "1.0..."; }` where
//!   BOTH `name` and `version` values had EMPTY context. The
//!   pre-fix arms wrapped the parsed strings in plain
//!   `Value::String(_)` without checking the input's
//!   `string_context()`. Same shape as the `match` / `split`
//!   capture-group context loss closed in slice #51.
//!
//!   `builtins.splitVersion "1.0${./p}"` returned a list of
//!   string parts, each with EMPTY context. Same shape: each
//!   element is a substring of the input version string and
//!   should inherit the input's provenance.
//!
//!   Production-relevant: `.px` authors who parse a context-
//!   bearing derivation reference (e.g. comparing a stored
//!   derivation name to a target version) silently lost the
//!   dependency marker at the parse boundary. Downstream
//!   consumers (`hashString` / `getContext` / derivation
//!   realization) saw context-free strings and treated the
//!   parse result as self-contained — same insidious shape as
//!   slice #55's `toFile` content guard.
//!
//! Truth-owner files:
//! - `crates/pnix-eval/src/interpret.rs` `apply_builtin`
//!   `"parseDrvName"` arm — extracts `args[0].string_context()`
//!   and wraps both `name` and `version` values in
//!   `Value::string_with_context`.
//! - `crates/pnix-eval/src/interpret.rs` `apply_builtin`
//!   `"splitVersion"` arm — same shape: each result list
//!   element is `Value::string_with_context`.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

fn ctx_json(expr: &str) -> String {
  json(&format!("builtins.toJSON (builtins.getContext ({}))", expr))
}

// ── parseDrvName: name inherits context ───────────────────────

#[test]
fn parse_drv_name_value_inherits_input_context() {
  let ctx = ctx_json(r#"(builtins.parseDrvName "hello-1.0${./p}").name"#);
  assert!(ctx.contains("./p"), "got: {ctx}");
}

#[test]
fn parse_drv_version_value_inherits_input_context() {
  let ctx = ctx_json(r#"(builtins.parseDrvName "hello-1.0${./p}").version"#);
  assert!(ctx.contains("./p"), "got: {ctx}");
}

#[test]
fn parse_drv_no_context_input_no_context_output() {
  // Sanity: plain input → empty context output.
  let ctx = ctx_json(r#"(builtins.parseDrvName "hello-1.0").name"#);
  assert_eq!(ctx, r#""{}""#);
}

#[test]
fn parse_drv_text_unchanged() {
  // Sanity: text portions are unchanged (only context channel
  // was added).
  assert_eq!(
    json(r#"(builtins.parseDrvName "hello-1.0").name"#),
    r#""hello""#
  );
  assert_eq!(
    json(r#"(builtins.parseDrvName "hello-1.0").version"#),
    r#""1.0""#
  );
}

#[test]
fn parse_drv_multi_path_context_unioned() {
  // If input has multiple paths via `+`, both should
  // propagate.
  let ctx = ctx_json(r#"(builtins.parseDrvName ("a-" + ./p1 + "-" + ./p2)).name"#);
  // Both paths should be in the context (combined via slice
  // #54's `+ Path` semantics).
  // Note: the constructed string's text could vary based on
  // path display, so we only check both paths appear in
  // context.
  assert!(ctx.contains("./p1"), "got: {ctx}");
  assert!(ctx.contains("./p2"), "got: {ctx}");
}

// ── splitVersion: each part inherits context ──────────────────

#[test]
fn split_version_first_part_inherits_input_context() {
  let ctx = ctx_json(r#"builtins.elemAt (builtins.splitVersion "1.0${./p}") 0"#);
  assert!(ctx.contains("./p"), "got: {ctx}");
}

#[test]
fn split_version_second_part_inherits_input_context() {
  let ctx = ctx_json(r#"builtins.elemAt (builtins.splitVersion "1.0${./p}") 1"#);
  assert!(ctx.contains("./p"), "got: {ctx}");
}

#[test]
fn split_version_no_context_input_no_context_output() {
  // Sanity.
  let ctx = ctx_json(r#"builtins.elemAt (builtins.splitVersion "1.0") 0"#);
  assert_eq!(ctx, r#""{}""#);
}

#[test]
fn split_version_returns_list() {
  // Sanity: result is a list with text-only check.
  let v = json(r#"builtins.splitVersion "1.2.3""#);
  assert!(v.contains("\"1\""), "got: {v}");
  assert!(v.contains("\"2\""), "got: {v}");
  assert!(v.contains("\"3\""), "got: {v}");
}

// ── pre-existing argument guards still work ───────────────────

#[test]
fn parse_drv_non_string_errors() {
  let e = eval_expr(r#"builtins.parseDrvName 42"#)
    .err()
    .unwrap()
    .to_string();
  assert!(e.contains("parseDrvName"), "got: {e}");
}

#[test]
fn split_version_non_string_errors() {
  let e = eval_expr(r#"builtins.splitVersion 42"#)
    .err()
    .unwrap()
    .to_string();
  assert!(e.contains("splitVersion"), "got: {e}");
}
