//! Regression cover for `fold` non-list silent type-pass and
//! `groupBy` context-bearing key-function string parity.
//!
//! 2026-05-05 audit findings (slice #52):
//!
//!   1. `builtins.fold f init nonList` silently returned `init`
//!      for ANY non-list third arg (int, null, attrset, etc.).
//!      Pre-fix shape was `_ => Ok(init.clone())` — bypassed every
//!      `if r != init then ...` guard a `.px` author might write
//!      after a fold over a maybe-list. Same family as slices
//!      #32/#33/#38: list-typed builtin → fail-loud on type
//!      mismatch. The Nix-canonical `foldl'` / `foldr` already
//!      errored loudly; this pnix-only `fold` was the holdout.
//!   2. `builtins.groupBy keyFn list` rejected context-bearing
//!      key-function returns. Pre-fix the arm matched only
//!      `Value::String(_)`, missing `Value::StringContext { .. }`,
//!      and the error message "key function must return string"
//!      was a lie — the function DID return a string, just one
//!      with context. Same parity miss as slice #50's `isString`.
//!      Fix: use `as_str()` which handles both variants.
//!
//! Truth-owner files:
//! - `crates/pnix-eval/src/interpret.rs` `apply_builtin` `"fold"`
//!   arm — non-list third arg now errors with `fold: third arg
//!   must be list, got <T>`. Mirrors `foldl'` / `foldr` error
//!   shape.
//! - `crates/pnix-eval/src/interpret.rs` `apply_builtin`
//!   `"groupBy"` arm — key-function return uses `as_str()` and
//!   the error message names the actual returned type.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

fn err(src: &str) -> String {
  eval_expr(src).err().expect(src).to_string()
}

// ── fold: fail-loud on non-list ─────────────────────────────────

#[test]
fn fold_int_third_arg_errors_loud() {
  let e = err(r#"builtins.fold (a: b: a + b) 100 42"#);
  assert!(e.contains("fold"), "got: {e}");
  assert!(e.contains("must be list"), "got: {e}");
  assert!(e.contains("int"), "got: {e}");
}

#[test]
fn fold_null_third_arg_errors_loud() {
  let e = err(r#"builtins.fold (a: b: a + b) 100 null"#);
  assert!(e.contains("fold"), "got: {e}");
  assert!(e.contains("must be list"), "got: {e}");
  assert!(e.contains("null"), "got: {e}");
}

#[test]
fn fold_attrset_third_arg_errors_loud() {
  let e = err(r#"builtins.fold (a: b: a + b) 100 { x = 1; }"#);
  assert!(e.contains("fold"), "got: {e}");
  assert!(e.contains("must be list"), "got: {e}");
  assert!(e.contains("attrset") || e.contains("set"), "got: {e}");
}

#[test]
fn fold_string_third_arg_errors_loud() {
  let e = err(r#"builtins.fold (a: b: a + b) 100 "hi""#);
  assert!(e.contains("fold"), "got: {e}");
  assert!(e.contains("must be list"), "got: {e}");
  assert!(e.contains("string"), "got: {e}");
}

// ── fold: happy path ────────────────────────────────────────────

#[test]
fn fold_empty_list_returns_init() {
  // Sanity: empty-list still returns init unchanged. The fix
  // didn't regress this — empty list is a list.
  assert_eq!(json(r#"builtins.fold (a: b: a + b) 100 [ ]"#), r#"100"#);
}

#[test]
fn fold_three_element_list_left_associates() {
  // `fold f init [a b c]` = `f (f (f init a) b) c`. With sum,
  // `100 + 1 + 2 + 3` = 106.
  assert_eq!(
    json(r#"builtins.fold (a: b: a + b) 100 [ 1 2 3 ]"#),
    r#"106"#
  );
}

#[test]
fn fold_single_element_list_applies_once() {
  assert_eq!(json(r#"builtins.fold (a: b: a + b) 100 [ 42 ]"#), r#"142"#);
}

// ── groupBy: context-bearing key strings accepted ──────────────

#[test]
fn group_by_plain_string_key_unchanged() {
  // Sanity: plain string key still works.
  assert_eq!(
    json(r#"builtins.groupBy (x: if x < 5 then "lo" else "hi") [ 1 6 2 7 ]"#),
    r#"{"hi":[6,7],"lo":[1,2]}"#
  );
}

#[test]
fn group_by_context_bearing_key_accepted() {
  // Pre-fix: errored `key function must return string`. Now:
  // accepts the context-bearing return and groups by its text.
  // The interpolated path `${./p}` expands to the realised
  // store-path text, so the resulting key is `"a<storePath>"`,
  // not `"a"` — what matters here is that the call DOES NOT
  // error and DOES produce an attrset with a single group
  // containing both elements.
  let v = json(r#"builtins.groupBy (item: "a${./p}") [ "x" "y" ]"#);
  assert!(v.contains("\"x\""), "got: {v}");
  assert!(v.contains("\"y\""), "got: {v}");
  // Both elements grouped under the same key.
  let key_count = json(
    r#"builtins.length (builtins.attrNames (builtins.groupBy (item: "a${./p}") [ "x" "y" ]))"#,
  );
  assert_eq!(key_count, "1", "expected single group");
}

#[test]
fn group_by_int_key_still_errors_loud() {
  // Sanity: non-string key still errors with the actual type.
  let e = err(r#"builtins.groupBy (item: 42) [ "a" ]"#);
  assert!(e.contains("groupBy"), "got: {e}");
  assert!(e.contains("must return string"), "got: {e}");
  assert!(e.contains("int"), "got: {e}");
}

#[test]
fn group_by_null_key_errors_loud() {
  let e = err(r#"builtins.groupBy (item: null) [ "a" ]"#);
  assert!(e.contains("groupBy"), "got: {e}");
  assert!(e.contains("must return string"), "got: {e}");
  assert!(e.contains("null"), "got: {e}");
}

#[test]
fn group_by_list_key_errors_loud() {
  let e = err(r#"builtins.groupBy (item: [ ]) [ "a" ]"#);
  assert!(e.contains("groupBy"), "got: {e}");
  assert!(e.contains("must return string"), "got: {e}");
  assert!(e.contains("list"), "got: {e}");
}

#[test]
fn group_by_non_list_second_arg_errors_loud() {
  let e = err(r#"builtins.groupBy (x: "k") 42"#);
  assert!(e.contains("groupBy"), "got: {e}");
  assert!(e.contains("must be list"), "got: {e}");
  assert!(e.contains("int"), "got: {e}");
}

#[test]
fn group_by_empty_list_returns_empty_attrset() {
  assert_eq!(json(r#"builtins.groupBy (x: "k") [ ]"#), r#"{}"#);
}
