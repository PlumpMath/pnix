//! Regression cover for `<` / `<=` / `>` / `>=` /
//! `builtins.lessThan` / `sort` comparison of context-bearing
//! strings.
//!
//! 2026-05-05 audit findings (slice #60):
//!
//!   `compare_values_at_depth` had an arm for
//!   `(Value::String, Value::String)` only, with no arm for
//!   `(Value::StringContext, Value::StringContext)` or the
//!   mixed `(String, StringContext)` / `(StringContext,
//!   String)` cases. The fall-through `_ => Err("cannot
//!   compare {} with {}")` made any comparison touching a
//!   context-bearing string error with the misleading
//!   "cannot compare string with string" — claiming the
//!   operands weren't strings when they ARE strings.
//!
//!   Same shape as slice #59's equality fix. Pre-fix bugs:
//!     - `"x${./p}" < "y"` errored (should compare text).
//!     - `sort cmp [ "x${./a}" "x${./b}" ]` errored on every
//!       comparison — sorting a list of derivation references
//!       was impossible.
//!     - `builtins.lessThan` on context-bearing strings
//!       errored.
//!     - `<=` / `>` / `>=` (which all delegate to
//!       `compare_values`) all errored too.
//!
//!   Real Nix's comparison operators are text-only for strings
//!   (context ignored). The fix uses `as_str()` (which already
//!   handles both `Value::String` and `Value::StringContext`)
//!   under a guard pattern matching all four combinations.
//!
//! Truth-owner files:
//! - `crates/pnix-eval/src/interpret.rs` `compare_values_at_depth`
//!   — the `(Value::String(a), Value::String(b)) => Ok(a.cmp(b))`
//!   arm was replaced with a guard pattern covering all four
//!   String/StringContext combinations.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

fn err(src: &str) -> String {
  eval_expr(src).err().expect(src).to_string()
}

// ── < operator ────────────────────────────────────────────────

#[test]
fn lt_two_context_strings_compares_text() {
  // Pre-fix errored. Now compares text.
  assert_eq!(
    json(r#""x${./a}" < "x${./b}""#),
    // Two paths produce different store-path texts; compare
    // those. We don't pin which one is "less" because that
    // depends on the hash, but the call must not error.
    {
      let v = json(r#""x${./a}" < "x${./b}""#);
      assert!(v == "true" || v == "false", "got: {v}");
      v
    }
  );
}

#[test]
fn lt_context_vs_plain_string() {
  // Pre-fix errored. Now compares text.
  let v = json(r#""x${./a}" < "z""#);
  assert!(v == "true" || v == "false", "got: {v}");
}

#[test]
fn lt_plain_vs_context_string() {
  let v = json(r#""a" < "x${./p}""#);
  assert!(v == "true" || v == "false", "got: {v}");
}

#[test]
fn lt_plain_strings_unchanged() {
  // Sanity.
  assert_eq!(json(r#""abc" < "abd""#), "true");
}

// ── <= operator ───────────────────────────────────────────────

#[test]
fn le_two_context_strings_with_same_text_returns_true() {
  // Same text → equal → `<=` is true.
  assert_eq!(json(r#""x${./a}" <= "x${./a}""#), "true");
}

#[test]
fn le_plain_strings_unchanged() {
  assert_eq!(json(r#""abc" <= "abc""#), "true");
}

// ── > operator ────────────────────────────────────────────────

#[test]
fn gt_context_string_vs_plain() {
  // Don't pin direction (depends on store path hash). Just
  // verify it doesn't error.
  let v = json(r#""z${./p}" > "a""#);
  assert!(v == "true" || v == "false", "got: {v}");
}

#[test]
fn gt_plain_strings_unchanged() {
  assert_eq!(json(r#""b" > "a""#), "true");
}

// ── >= operator ───────────────────────────────────────────────

#[test]
fn ge_context_strings_same_text() {
  assert_eq!(json(r#""x${./a}" >= "x${./a}""#), "true");
}

// ── builtins.lessThan ─────────────────────────────────────────

#[test]
fn lessthan_builtin_with_context_string() {
  let v = json(r#"builtins.lessThan "x${./a}" "y""#);
  assert!(v == "true" || v == "false", "got: {v}");
}

#[test]
fn lessthan_builtin_with_two_context_strings() {
  let v = json(r#"builtins.lessThan "x${./a}" "x${./b}""#);
  assert!(v == "true" || v == "false", "got: {v}");
}

// ── sort with context-bearing list ────────────────────────────

#[test]
fn sort_list_of_context_strings_works() {
  // Pre-fix: errored on every comparison. Now: sorts.
  let v = json(
    r#"
    builtins.sort (a: b: a < b) [ "b${./bp}" "a${./ap}" "c${./cp}" ]
  "#,
  );
  // Result is a list of three context-bearing strings (in
  // some order). Just verify no error and length 3.
  let len = json(
    r#"
    builtins.length (
      builtins.sort (a: b: a < b) [ "b${./bp}" "a${./ap}" "c${./cp}" ]
    )
  "#,
  );
  assert_eq!(len, "3");
  assert!(v.contains("/nix/store/"), "got: {v}");
}

// ── builtins.lt / le / gt / ge (slice #43 family) ─────────────

#[test]
fn builtin_lt_with_context_string() {
  let v = json(r#"builtins.lt "x${./a}" "y""#);
  assert!(v == "true" || v == "false", "got: {v}");
}

#[test]
fn builtin_le_with_context_string() {
  let v = json(r#"builtins.le "x${./a}" "x${./a}""#);
  assert_eq!(v, "true");
}

#[test]
fn builtin_gt_with_context_string() {
  let v = json(r#"builtins.gt "z${./p}" "a""#);
  assert!(v == "true" || v == "false", "got: {v}");
}

#[test]
fn builtin_ge_with_context_string() {
  let v = json(r#"builtins.ge "x${./a}" "x${./a}""#);
  assert_eq!(v, "true");
}

// ── existing comparison cases unchanged ───────────────────────

#[test]
fn int_lt_unchanged() {
  assert_eq!(json(r#"1 < 2"#), "true");
}

#[test]
fn list_lt_unchanged() {
  assert_eq!(json(r#"[ 1 2 ] < [ 1 3 ]"#), "true");
}

#[test]
fn type_mismatch_int_string_still_errors() {
  // Sanity: real type mismatch still errors.
  let e = err(r#"1 < "a""#);
  assert!(e.contains("cannot compare"), "got: {e}");
  assert!(e.contains("int"), "got: {e}");
  assert!(e.contains("string"), "got: {e}");
}

#[test]
fn type_mismatch_attrset_still_errors() {
  let e = err(r#"{ a = 1; } < { a = 2; }"#);
  assert!(e.contains("cannot compare"), "got: {e}");
  assert!(e.contains("set"), "got: {e}");
}

#[test]
fn type_mismatch_lambda_still_errors() {
  let e = err(r#"(x: x) < (y: y)"#);
  assert!(e.contains("cannot compare"), "got: {e}");
  assert!(e.contains("lambda"), "got: {e}");
}
