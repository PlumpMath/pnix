//! Regression cover for `concatStrings` fail-loud + context
//! propagation, and `match` / `split` capture-group context
//! propagation.
//!
//! 2026-05-05 audit findings (slice #51):
//!
//!   1. `builtins.concatStrings 42` silently produced `""` (non-
//!      list arg → empty string fallback). Production-relevant:
//!      the same silent-pass shape closed for every other typed
//!      list builtin in slices #32, #33, #38. `concatStrings` was
//!      the last holdout.
//!   2. `builtins.concatStrings [ "a" 1 "b" ]` silently dropped
//!      the non-string element via `filter_map(as_str)` and
//!      returned `"ab"`. Same shape as the slice #38 (replace-
//!      Strings) and slice #49 (concatStringsSep) tightenings.
//!   3. `builtins.concatStrings [ "a${./p1}" "b${./p2}" ]`
//!      silently lost both `./p1` and `./p2` contexts. The
//!      slice-#49 family closed this shape for `concatStringsSep`
//!      / `substring` / `replaceStrings` / `toString`;
//!      `concatStrings` was missed.
//!   4. `builtins.match "(.+)" "x${./p}"` returned a list with a
//!      single capture-group string whose context was empty.
//!      Captures are literally substrings of the haystack, so
//!      they should inherit haystack provenance — same shape as
//!      `substring`.
//!   5. `builtins.split "-" "x${./p}-y"` returned a list of
//!      alternating literal-segment strings and capture-group
//!      lists; every result string had empty context. Same
//!      reasoning as `match`.
//!
//! Truth-owner files:
//! - `crates/pnix-eval/src/interpret.rs` `apply_builtin`
//!   `"concatStrings"` arm — list-required, every-element-string-
//!   required, unions every element's context.
//! - `crates/pnix-eval/src/interpret.rs` `apply_builtin`
//!   `"match"` arm — capture-group strings inherit haystack
//!   `string_context()`.
//! - `crates/pnix-eval/src/interpret.rs` `apply_builtin`
//!   `"split"` arm — every result string (literal segments,
//!   capture-group elements, leading/trailing/empty-separator
//!   pieces) inherits haystack context.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

fn err(src: &str) -> String {
  eval_expr(src).err().expect(src).to_string()
}

fn ctx_json(expr: &str) -> String {
  json(&format!("builtins.toJSON (builtins.getContext ({}))", expr))
}

// ── concatStrings: fail-loud ────────────────────────────────────

#[test]
fn concat_strings_non_list_errors_loud() {
  let e = err(r#"builtins.concatStrings 42"#);
  assert!(e.contains("concatStrings"), "got: {e}");
  assert!(e.contains("must be list"), "got: {e}");
  assert!(e.contains("int"), "got: {e}");
}

#[test]
fn concat_strings_non_string_element_errors_loud() {
  let e = err(r#"builtins.concatStrings [ "a" 1 "b" ]"#);
  assert!(e.contains("concatStrings"), "got: {e}");
  assert!(e.contains("element at index 1"), "got: {e}");
  assert!(e.contains("int"), "got: {e}");
  assert!(e.contains("not a string"), "got: {e}");
}

#[test]
fn concat_strings_attrset_element_errors_loud() {
  let e = err(r#"builtins.concatStrings [ "a" { x = 1; } ]"#);
  assert!(e.contains("concatStrings"), "got: {e}");
  assert!(e.contains("element at index 1"), "got: {e}");
  assert!(e.contains("attrset") || e.contains("set"), "got: {e}");
}

#[test]
fn concat_strings_null_element_errors_loud() {
  let e = err(r#"builtins.concatStrings [ "a" null ]"#);
  assert!(e.contains("concatStrings"), "got: {e}");
  assert!(e.contains("element at index 1"), "got: {e}");
  assert!(e.contains("null"), "got: {e}");
}

// ── concatStrings: happy path ────────────────────────────────────

#[test]
fn concat_strings_empty_list_returns_empty_string() {
  assert_eq!(json(r#"builtins.concatStrings [ ]"#), r#""""#);
}

#[test]
fn concat_strings_joins_strings_no_separator() {
  assert_eq!(
    json(r#"builtins.concatStrings [ "a" "b" "c" ]"#),
    r#""abc""#
  );
}

// ── concatStrings: context propagation ──────────────────────────

#[test]
fn concat_strings_unions_all_element_contexts() {
  let ctx = ctx_json(r#"builtins.concatStrings [ "a${./p1}" "b${./p2}" ]"#);
  assert!(ctx.contains("./p1"), "got: {ctx}");
  assert!(ctx.contains("./p2"), "got: {ctx}");
}

#[test]
fn concat_strings_no_context_unchanged() {
  let ctx = ctx_json(r#"builtins.concatStrings [ "a" "b" ]"#);
  assert_eq!(ctx, r#""{}""#);
}

// ── match: capture-group context propagation ────────────────────

#[test]
fn match_capture_group_inherits_haystack_context() {
  let ctx = ctx_json(r#"builtins.elemAt (builtins.match "(.+)" "x${./p}") 0"#);
  assert!(ctx.contains("./p"), "got: {ctx}");
}

#[test]
fn match_no_match_returns_null_unchanged() {
  // Sanity: when match fails, return is still `null` (unchanged
  // by the slice fix). Confirms we did not regress the no-match
  // path.
  assert_eq!(json(r#"builtins.match "x" "y""#), r#"null"#);
}

#[test]
fn match_haystack_no_context_no_capture_context() {
  let ctx = ctx_json(r#"builtins.elemAt (builtins.match "(.+)" "abc") 0"#);
  assert_eq!(ctx, r#""{}""#);
}

// ── split: literal-segment + capture context propagation ────────

#[test]
fn split_first_segment_inherits_haystack_context() {
  // First segment "x" inherits haystack `./p` context.
  let ctx = ctx_json(r#"builtins.elemAt (builtins.split "-" "x${./p}-y") 0"#);
  assert!(ctx.contains("./p"), "got: {ctx}");
}

#[test]
fn split_trailing_segment_inherits_haystack_context() {
  // Trailing segment "y" also inherits haystack context.
  let ctx = ctx_json(r#"builtins.elemAt (builtins.split "-" "x${./p}-y") 2"#);
  assert!(ctx.contains("./p"), "got: {ctx}");
}

#[test]
fn split_no_context_haystack_no_segment_context() {
  let ctx = ctx_json(r#"builtins.elemAt (builtins.split "-" "x-y") 0"#);
  assert_eq!(ctx, r#""{}""#);
}
