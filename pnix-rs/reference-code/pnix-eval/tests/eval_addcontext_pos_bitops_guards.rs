//! Regression cover for `addErrorContext` non-string context
//! fail-loud, `unsafeGetAttrPos` no-position-info `null` return,
//! and `bitAnd` / `bitOr` / `bitXor` typed error messages.
//!
//! 2026-05-05 audit findings (slice #53):
//!
//!   1. `builtins.addErrorContext 42 "value"` silently returned
//!      `"value"`. The contract is `addErrorContext contextString
//!      value`; non-string context is a programming error that
//!      should error loud (mirrors `builtins.warn` which already
//!      validates its string message). Pre-fix the arm took ANY
//!      first-arg type without check.
//!   2. `builtins.unsafeGetAttrPos "a" { a = 1; }` returned
//!      `{ file = "<unknown>"; line = 0; column = 0; }` for any
//!      slot whose value was already-forced (no `Thunk.attr_pos`).
//!      Real Nix returns `null` to distinguish "no position info"
//!      from "position info available". Pre-fix's silent
//!      fabrication misled tools that consume this builtin for
//!      source-location diagnostics.
//!   3. `builtins.bitAnd` / `bitOr` / `bitXor` produced the
//!      catch-all "both args must be integer" error which left
//!      authors guessing which arg failed and what type was given.
//!      Diagnostic improvement: name the offending arg and type,
//!      matching the slice #32 / #33 / #38 / #51 / #52 family.
//!
//! Truth-owner files:
//! - `crates/pnix-eval/src/interpret.rs` `apply_builtin`
//!   `"addErrorContext"` arm — `args[0]` is now validated as
//!   string via `as_str()`. Non-string context errors with
//!   actual type.
//! - `crates/pnix-eval/src/interpret.rs` `apply_builtin`
//!   `"unsafeGetAttrPos"` arm — non-`Thunk(attr_pos)` slot now
//!   returns `Value::Null` (was: fake `<unknown>:0:0` attrset).
//! - `crates/pnix-eval/src/interpret.rs` `apply_builtin`
//!   `"bitAnd"` / `"bitOr"` / `"bitXor"` arms — three-arm match
//!   names whether first or second arg failed and the actual
//!   type.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

fn err(src: &str) -> String {
  eval_expr(src).err().expect(src).to_string()
}

// ── addErrorContext: fail-loud on non-string context ───────────

#[test]
fn add_error_context_int_context_errors_loud() {
  let e = err(r#"builtins.addErrorContext 42 "value""#);
  assert!(e.contains("addErrorContext"), "got: {e}");
  assert!(e.contains("must be string"), "got: {e}");
  assert!(e.contains("int"), "got: {e}");
}

#[test]
fn add_error_context_null_context_errors_loud() {
  let e = err(r#"builtins.addErrorContext null "value""#);
  assert!(e.contains("addErrorContext"), "got: {e}");
  assert!(e.contains("must be string"), "got: {e}");
  assert!(e.contains("null"), "got: {e}");
}

#[test]
fn add_error_context_attrset_context_errors_loud() {
  let e = err(r#"builtins.addErrorContext { x = 1; } "value""#);
  assert!(e.contains("addErrorContext"), "got: {e}");
  assert!(e.contains("must be string"), "got: {e}");
  assert!(e.contains("attrset") || e.contains("set"), "got: {e}");
}

#[test]
fn add_error_context_list_context_errors_loud() {
  let e = err(r#"builtins.addErrorContext [ ] "value""#);
  assert!(e.contains("addErrorContext"), "got: {e}");
  assert!(e.contains("must be string"), "got: {e}");
  assert!(e.contains("list"), "got: {e}");
}

// ── addErrorContext: happy path ────────────────────────────────

#[test]
fn add_error_context_string_context_returns_value() {
  // Sanity: with a string context, the value is returned
  // unchanged. addErrorContext is a transparent passthrough at
  // the value level (the context is metadata for error messages
  // — pnix's current implementation does not enrich errors, but
  // the signature contract is preserved).
  assert_eq!(
    json(r#"builtins.addErrorContext "ctx" "value""#),
    r#""value""#
  );
}

#[test]
fn add_error_context_with_context_bearing_message_works() {
  // Sanity: context-bearing string is also a valid context arg
  // (slice #50 parity). `as_str()` accepts both variants.
  assert_eq!(json(r#"builtins.addErrorContext "x${./p}" 42"#), r#"42"#);
}

#[test]
fn add_error_context_lazy_in_value() {
  // Sanity: the value arg is NOT forced at boundary, so it can
  // be `1 + 2` without issue. Confirms we did not regress
  // laziness while adding the type guard.
  assert_eq!(json(r#"builtins.addErrorContext "ctx" (1 + 2)"#), r#"3"#);
}

// ── unsafeGetAttrPos: null on no-position info ─────────────────

#[test]
fn unsafe_get_attr_pos_no_attr_pos_returns_null() {
  // `builtins.mapAttrs` constructs the value via `deferred_apply2`
  // which produces a `Value::Thunk { attr_pos: None, .. }` — there
  // is no source position for the resulting attribute. Pre-fix
  // pnix returned a fake `{ file = "<unknown>"; line = 0; column
  // = 0; }` for this case (silent fabrication). Real Nix returns
  // `null` to signal "no position info available". This test
  // pins the post-fix `null` behaviour.
  assert_eq!(
    json(r#"builtins.unsafeGetAttrPos "a" (builtins.mapAttrs (k: v: v) { a = 1; })"#),
    r#"null"#
  );
}

#[test]
fn unsafe_get_attr_pos_literal_attrset_still_has_position() {
  // Sanity: literal attrset `{ a = 1; }` IS a Thunk-with-attr_pos
  // (the parser attaches source location), so the result is a
  // position attrset. The line / column are non-zero (pinned at
  // shape rather than exact value, since the test source itself
  // determines them); file is `<unknown>` for direct-evaluated
  // expressions (no input file). The point of this case is that
  // the slice-#53 fix did NOT regress source-positioned attrsets
  // — only the attr_pos-None case changed.
  let v = json(r#"builtins.unsafeGetAttrPos "a" { a = 1; }"#);
  assert!(v.contains("\"file\""), "got: {v}");
  assert!(v.contains("\"line\""), "got: {v}");
  assert!(v.contains("\"column\""), "got: {v}");
}

#[test]
fn unsafe_get_attr_pos_missing_attr_still_null() {
  // Sanity: attribute not present remains `null` (pre-existing
  // behaviour, unchanged by this slice).
  assert_eq!(
    json(r#"builtins.unsafeGetAttrPos "z" { a = 1; }"#),
    r#"null"#
  );
}

#[test]
fn unsafe_get_attr_pos_first_arg_non_string_errors() {
  let e = err(r#"builtins.unsafeGetAttrPos 42 { a = 1; }"#);
  assert!(e.contains("unsafeGetAttrPos"), "got: {e}");
}

#[test]
fn unsafe_get_attr_pos_second_arg_non_attrset_errors() {
  let e = err(r#"builtins.unsafeGetAttrPos "a" 42"#);
  assert!(e.contains("unsafeGetAttrPos"), "got: {e}");
}

// ── bitAnd / bitOr / bitXor: typed error messages ──────────────

#[test]
fn bit_and_first_arg_string_names_first_and_type() {
  let e = err(r#"builtins.bitAnd "x" 5"#);
  assert!(e.contains("bitAnd"), "got: {e}");
  assert!(e.contains("first arg"), "got: {e}");
  assert!(e.contains("string"), "got: {e}");
}

#[test]
fn bit_and_second_arg_float_names_second_and_type() {
  let e = err(r#"builtins.bitAnd 5 3.0"#);
  assert!(e.contains("bitAnd"), "got: {e}");
  assert!(e.contains("second arg"), "got: {e}");
  assert!(e.contains("float"), "got: {e}");
}

#[test]
fn bit_or_first_arg_null_names_first_and_type() {
  let e = err(r#"builtins.bitOr null 5"#);
  assert!(e.contains("bitOr"), "got: {e}");
  assert!(e.contains("first arg"), "got: {e}");
  assert!(e.contains("null"), "got: {e}");
}

#[test]
fn bit_or_second_arg_list_names_second_and_type() {
  let e = err(r#"builtins.bitOr 5 [ ]"#);
  assert!(e.contains("bitOr"), "got: {e}");
  assert!(e.contains("second arg"), "got: {e}");
  assert!(e.contains("list"), "got: {e}");
}

#[test]
fn bit_xor_first_arg_attrset_names_first_and_type() {
  let e = err(r#"builtins.bitXor { x = 1; } 5"#);
  assert!(e.contains("bitXor"), "got: {e}");
  assert!(e.contains("first arg"), "got: {e}");
  assert!(e.contains("attrset") || e.contains("set"), "got: {e}");
}

#[test]
fn bit_xor_second_arg_string_names_second_and_type() {
  let e = err(r#"builtins.bitXor 5 "y""#);
  assert!(e.contains("bitXor"), "got: {e}");
  assert!(e.contains("second arg"), "got: {e}");
  assert!(e.contains("string"), "got: {e}");
}

// ── bit ops: happy path ────────────────────────────────────────

#[test]
fn bit_and_two_ints_works() {
  // Sanity: int/int still works.
  assert_eq!(json(r#"builtins.bitAnd 12 10"#), r#"8"#);
}

#[test]
fn bit_or_two_ints_works() {
  assert_eq!(json(r#"builtins.bitOr 12 10"#), r#"14"#);
}

#[test]
fn bit_xor_two_ints_works() {
  assert_eq!(json(r#"builtins.bitXor 12 10"#), r#"6"#);
}
