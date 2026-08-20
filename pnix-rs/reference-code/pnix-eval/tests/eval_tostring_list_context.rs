//! Regression cover for `builtins.toString` of lists / paths /
//! `__toString` / `outPath` — context propagation through every
//! shape the coercion pipeline can take.
//!
//! 2026-05-05 audit findings (slice #57):
//!
//!   1. `builtins.toString [ "a${./p1}" "b${./p2}" ]` produced
//!      `"a... b..."` with EMPTY context. The list-coercion path
//!      in `coerce_to_string_for_to_string` joined element texts
//!      but completely dropped element contexts. Real Nix
//!      coerces each element via the same `${}` interpolation
//!      semantics — the result string carries the union of
//!      every element's context. Pre-fix any `.px` author
//!      building a path-list (e.g. derivation deps via
//!      `toString [ ./bin ./lib ]`) silently lost every
//!      dependency marker.
//!
//!   2. `builtins.toString [ "a" ./p ]` produced `"a ./p"` with
//!      empty context. Path elements inside the list should
//!      contribute their display form to the result context
//!      (mirrors `${./p}` interpolation). Pre-fix path
//!      dependencies in mixed-content lists were silently lost.
//!
//!   3. `__toString` / `outPath` recursive coercion paths also
//!      dropped element-side context if the recursed value was
//!      itself a list or context-bearing string. Same shape as
//!      (1) — the recursive helper now propagates context all
//!      the way up.
//!
//! Truth-owner files:
//! - `crates/pnix-eval/src/interpret.rs`
//!   `coerce_to_string_for_to_string_with_context` —
//!   context-aware coercion. Returns `(text, BTreeSet<String>)`
//!   where the context set unions every recursive contribution
//!   (string-context input, path display form, list element
//!   contexts, `__toString`/`outPath` recursive contexts).
//! - `crates/pnix-eval/src/interpret.rs` `apply_builtin`
//!   `"toString"` arm — uses the new context-aware helper to
//!   build `Value::string_with_context` with the unioned
//!   context.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

fn ctx_json(expr: &str) -> String {
  json(&format!("builtins.toJSON (builtins.getContext ({}))", expr))
}

// ── toString of list of context-bearing strings ───────────────

#[test]
fn to_string_list_unions_element_contexts() {
  let ctx = ctx_json(r#"builtins.toString [ "a${./p1}" "b${./p2}" ]"#);
  assert!(ctx.contains("./p1"), "got: {ctx}");
  assert!(ctx.contains("./p2"), "got: {ctx}");
}

#[test]
fn to_string_list_text_unchanged() {
  // Sanity: text portion still joins with space (Nix-canonical).
  // The interpolated path text has the resolved store-path form.
  let v = json(r#"builtins.toString [ "a" "b" ]"#);
  assert_eq!(v, r#""a b""#);
}

#[test]
fn to_string_list_three_strings_with_context() {
  let ctx = ctx_json(r#"builtins.toString [ "a${./p1}" "b${./p2}" "c${./p3}" ]"#);
  assert!(ctx.contains("./p1"), "got: {ctx}");
  assert!(ctx.contains("./p2"), "got: {ctx}");
  assert!(ctx.contains("./p3"), "got: {ctx}");
}

#[test]
fn to_string_list_no_context_unchanged() {
  let ctx = ctx_json(r#"builtins.toString [ "a" "b" "c" ]"#);
  assert_eq!(ctx, r#""{}""#);
}

// ── toString of list with path elements ───────────────────────

#[test]
fn to_string_list_path_element_adds_to_context() {
  // Path elements inside a list contribute their display form
  // to the result context (build-time dep marker).
  let ctx = ctx_json(r#"builtins.toString [ "prefix" ./somepath ]"#);
  assert!(ctx.contains("./somepath"), "got: {ctx}");
}

#[test]
fn to_string_single_path_adds_to_context() {
  // Sanity check on the new path arm: bare path coerced to
  // string also carries the path in context.
  let ctx = ctx_json(r#"builtins.toString ./p"#);
  assert!(ctx.contains("./p"), "got: {ctx}");
}

#[test]
fn to_string_list_mixed_path_and_context_string() {
  let ctx = ctx_json(r#"builtins.toString [ "a${./p1}" ./p2 "c" ]"#);
  assert!(ctx.contains("./p1"), "got: {ctx}");
  assert!(ctx.contains("./p2"), "got: {ctx}");
}

// ── toString of __toString returning context-bearing ──────────

#[test]
fn to_string_method_returning_context_bearing_string() {
  let ctx = ctx_json(r#"builtins.toString { __toString = self: "result-${./p}"; }"#);
  assert!(ctx.contains("./p"), "got: {ctx}");
}

#[test]
fn to_string_outpath_path_value_adds_context() {
  let ctx = ctx_json(r#"builtins.toString { outPath = ./foo; }"#);
  assert!(ctx.contains("./foo"), "got: {ctx}");
}

// ── toString happy paths unchanged ────────────────────────────

#[test]
fn to_string_int_unchanged() {
  assert_eq!(json(r#"builtins.toString 42"#), r#""42""#);
}

#[test]
fn to_string_negative_int_unchanged() {
  assert_eq!(json(r#"builtins.toString (-42)"#), r#""-42""#);
}

#[test]
fn to_string_zero_unchanged() {
  assert_eq!(json(r#"builtins.toString 0"#), r#""0""#);
}

#[test]
fn to_string_true_unchanged() {
  assert_eq!(json(r#"builtins.toString true"#), r#""1""#);
}

#[test]
fn to_string_false_unchanged() {
  assert_eq!(json(r#"builtins.toString false"#), r#""""#);
}

#[test]
fn to_string_null_unchanged() {
  assert_eq!(json(r#"builtins.toString null"#), r#""""#);
}

#[test]
fn to_string_float_unchanged() {
  // pnix's `f.to_string()` renders 1.5 as "1.5"; integer-valued
  // floats render without trailing ".0" (matching Rust default,
  // not Nix's "1.000000" but pnix-canonical).
  assert_eq!(json(r#"builtins.toString 1.5"#), r#""1.5""#);
}

#[test]
fn to_string_list_of_ints_unchanged() {
  assert_eq!(json(r#"builtins.toString [ 1 2 3 ]"#), r#""1 2 3""#);
}

#[test]
fn to_string_attrset_outpath_unchanged() {
  assert_eq!(
    json(r#"builtins.toString { outPath = "/some/path"; }"#),
    r#""/some/path""#
  );
}

#[test]
fn to_string_attrset_method_unchanged() {
  assert_eq!(
    json(r#"builtins.toString { __toString = self: "stringified"; }"#),
    r#""stringified""#
  );
}

// ── error shapes unchanged ────────────────────────────────────

#[test]
fn to_string_attrset_no_method_errors_loud() {
  let e = eval_expr(r#"builtins.toString { a = 1; }"#).err();
  assert!(e.is_some(), "expected error");
  let e = e.unwrap().to_string();
  assert!(e.contains("cannot coerce a set"), "got: {e}");
  assert!(
    e.contains("__toString") || e.contains("outPath"),
    "got: {e}"
  );
}

#[test]
fn to_string_lambda_errors_loud() {
  let e = eval_expr(r#"builtins.toString (x: x)"#).err();
  assert!(e.is_some(), "expected error");
  let e = e.unwrap().to_string();
  assert!(e.contains("cannot coerce a function"), "got: {e}");
}

// ── parity: + ↔ ${} ↔ toString context propagation ────────────

#[test]
fn to_string_path_matches_plus_path_context() {
  // Slice #54 established `string + path` adds path to context.
  // Slice #57 establishes `toString [ ... ./p ... ]` does the
  // same. These should agree on the context set.
  let plus_ctx = ctx_json(r#""prefix" + ./p"#);
  let to_string_ctx = ctx_json(r#"builtins.toString [ "prefix" ./p ]"#);
  // Both have ./p in context.
  assert!(plus_ctx.contains("./p"), "plus: {plus_ctx}");
  assert!(to_string_ctx.contains("./p"), "toString: {to_string_ctx}");
}
