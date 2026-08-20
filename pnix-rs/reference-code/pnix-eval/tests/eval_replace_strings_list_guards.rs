//! Regression cover for `builtins.replaceStrings` non-list
//! `from` / `to` argument silent-pass.
//!
//! 2026-05-05 audit findings (slice #38):
//!   - **`replaceStrings` was silently coercing non-list `from` /
//!     `to` arguments to empty `Vec`s**, then the post-coerce
//!     length check (`from.len() != to.len()`) saw `0 == 0` and
//!     the for-loop walked over zero patterns, returning the
//!     haystack unchanged. So:
//!     - `replaceStrings 42 99 "abc"` → `"abc"` silently
//!     - `replaceStrings "from" "to" "abc"` → `"abc"` silently
//!     - `replaceStrings null null "abc"` → `"abc"` silently
//!     A `.px` author who swapped argument order, fed in a
//!     misconfigured upstream, or coerced from a JSON loader
//!     that returned a string instead of a list saw "no
//!     replacements happened" instead of a typed error — the
//!     classic "code didn't crash but did the wrong thing"
//!     production-bug shape.
//!     The asymmetric case (one list, one non-list, length
//!     mismatch) already errored via the length-check arm, but
//!     with a misleading message ("'from' and 'to' lists must
//!     have equal length") that hid the real type mismatch.
//!     Only the both-non-list case silently passed.
//!   - Fix: replace the silent `_ => Vec::new()` in both `from`
//!     and `to` arms with a typed error pinning the offending
//!     argument's type, mirroring the rest of the pnix builtin
//!     family (`filter` / `elem` / `concatStringsSep` — slices
//!     #32 / #33). The length-equal check stays for the case
//!     where both args are valid lists of mismatched length.
//!
//! Truth-owner files:
//! - `crates/pnix-eval/src/interpret.rs` `apply_builtin`
//!   `"replaceStrings"` arm — `from` / `to` non-list arms
//!   replaced with typed errors.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}
fn err_msg(src: &str) -> String {
  format!("{}", eval_expr(src).expect_err(src))
}

#[test]
fn rs_clean_call_works() {
  assert_eq!(
    json(r#"builtins.replaceStrings [ "a" "b" ] [ "X" "Y" ] "abc""#),
    r#""XYc""#
  );
}

#[test]
fn rs_empty_lists_returns_haystack_unchanged() {
  // Sanity: empty `from` / `to` lists are valid and just don't
  // replace anything. This must keep working — only non-list
  // args are the new error path.
  assert_eq!(json(r#"builtins.replaceStrings [] [] "abc""#), r#""abc""#);
}

#[test]
fn rs_from_int_errors() {
  // Was: silently returned haystack because `42.iter()` fell
  // back to `Vec::new()` and length check passed `0 == 1`
  // … wait, the asymmetric case errored via length. The
  // SILENT case was both-non-list.
  let m = err_msg(r#"builtins.replaceStrings 42 [ "X" ] "abc""#);
  assert!(m.contains("'from' must be list"), "got: {m}");
  assert!(m.contains("got int"), "got: {m}");
}

#[test]
fn rs_to_string_errors() {
  // The asymmetric case used to error with the misleading
  // length-mismatch message; now it errors with the correct
  // type message.
  let m = err_msg(r#"builtins.replaceStrings [ "a" ] "X" "abc""#);
  assert!(m.contains("'to' must be list"), "got: {m}");
  assert!(m.contains("got string"), "got: {m}");
}

#[test]
fn rs_both_int_errors() {
  // Was: silently returned `"abc"` because both args coerced
  // to empty vec, length check `0 == 0` passed, for-loop ran
  // zero iterations.
  let m = err_msg(r#"builtins.replaceStrings 42 99 "abc""#);
  assert!(m.contains("'from' must be list"), "got: {m}");
  assert!(m.contains("got int"), "got: {m}");
}

#[test]
fn rs_both_string_errors() {
  // Most production-relevant case: both args came back as
  // strings from a JSON loader that flattened a list.
  let m = err_msg(r#"builtins.replaceStrings "from" "to" "abc""#);
  assert!(m.contains("'from' must be list"), "got: {m}");
  assert!(m.contains("got string"), "got: {m}");
}

#[test]
fn rs_both_null_errors() {
  // Misconfigured upstream returning null for both lists.
  let m = err_msg(r#"builtins.replaceStrings null null "abc""#);
  assert!(m.contains("'from' must be list"), "got: {m}");
  assert!(m.contains("got null"), "got: {m}");
}

#[test]
fn rs_from_attrset_to_list_errors() {
  let m = err_msg(r#"builtins.replaceStrings { a = 1; } [ "X" ] "abc""#);
  assert!(m.contains("'from' must be list"), "got: {m}");
  assert!(m.contains("got set"), "got: {m}");
}

#[test]
fn rs_from_list_length_mismatch_still_errors_distinctly() {
  // The length-check error path is preserved for the case where
  // both args ARE lists but of different length. The error
  // message is the original one — different from the type
  // error, so callers can distinguish "you got the type wrong"
  // from "you got the count wrong".
  let m = err_msg(r#"builtins.replaceStrings [ "a" ] [ "X" "Y" ] "abc""#);
  assert!(m.contains("equal length"), "got: {m}");
  assert!(
    !m.contains("must be list"),
    "should not say type-error: {m}"
  );
}

#[test]
fn rs_haystack_non_string_still_errors() {
  // Slice scope check: third-arg-non-string already errored
  // before this fix. Make sure that path is unchanged.
  let m = err_msg(r#"builtins.replaceStrings [ "a" ] [ "X" ] 42"#);
  assert!(m.contains("third argument"), "got: {m}");
}

#[test]
fn rs_from_element_non_string_still_errors() {
  // Element-level type check (a list whose elements aren't all
  // strings) already errored. Confirm unchanged.
  let m = err_msg(r#"builtins.replaceStrings [ 1 ] [ "X" ] "abc""#);
  assert!(m.contains("'from' element must be string"), "got: {m}");
}
