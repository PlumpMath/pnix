//! Regression cover for `builtins.hasAttr`, `builtins.removeAttrs`,
//! and `builtins.concatLists` type guards.
//!
//! 2026-05-04 audit findings: three more silent-pass / silent-empty
//! cases that bypassed user input-shape guards:
//!
//!   - **`builtins.hasAttr` silent-false on bad input**:
//!     `hasAttr "a" 42` returned `false`, and `hasAttr 42 { a=1; }`
//!     returned `false`. A `.px` author writing
//!     `if builtins.hasAttr "k" x then … else …` would silently take
//!     the else-branch for any `x` that wasn't an attrset (and never
//!     learn that the input shape was wrong). `hasAttr` is now a
//!     hard type assertion on both arguments — name must be a
//!     string, value must be an attrset.
//!   - **`builtins.removeAttrs` silent-noop / silent-empty**:
//!     `removeAttrs 42 [ "x" ]` returned `{}` (silently substituting
//!     an empty attrset), and `removeAttrs { a=1; } 42` returned
//!     the original (silently treating non-list as no-op). Both
//!     now error. Names list elements that are not strings
//!     (`removeAttrs { a=1; } [ 42 ]`) used to be silently dropped;
//!     now error.
//!   - **`builtins.concatLists` silent-empty**:
//!     `concatLists 42` returned `[]`, and
//!     `concatLists [1 2]` (inner non-list elements) returned `[]`.
//!     Both now error and the inner-element check forces lazy
//!     thunks so a thunk-of-non-list is also caught.
//!
//! Truth-owner: `crates/pnix-eval/src/interpret.rs::apply_builtin`
//! arms for `"hasAttr"`, `"removeAttrs"`, and `"concatLists"`.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

fn err_msg(src: &str) -> String {
  format!("{}", eval_expr(src).expect_err(src))
}

// ── builtins.hasAttr type guards ───────────────────────────────

#[test]
fn has_attr_happy_path_still_works() {
  assert_eq!(json(r#"builtins.hasAttr "a" { a = 1; b = 2; }"#), "true");
  assert_eq!(json(r#"builtins.hasAttr "x" { a = 1; }"#), "false");
  assert_eq!(json(r#"builtins.hasAttr "a" {}"#), "false");
}

#[test]
fn has_attr_non_attrset_value_errors() {
  // Was: silent `false` for any non-attrset.
  let m = err_msg(r#"builtins.hasAttr "a" 42"#);
  assert!(m.contains("second argument must be attrset"), "got: {m}");
  assert!(m.contains("got int"), "got: {m}");
}

#[test]
fn has_attr_non_attrset_value_null_errors() {
  let m = err_msg(r#"builtins.hasAttr "a" null"#);
  assert!(m.contains("second argument must be attrset"), "got: {m}");
  assert!(m.contains("got null"), "got: {m}");
}

#[test]
fn has_attr_non_string_key_errors() {
  // Was: silent `false`.
  let m = err_msg(r#"builtins.hasAttr 42 { a = 1; }"#);
  assert!(m.contains("first argument must be string"), "got: {m}");
  assert!(m.contains("got int"), "got: {m}");
}

// ── builtins.removeAttrs type guards ──────────────────────────

#[test]
fn remove_attrs_happy_path_still_works() {
  assert_eq!(
    json(r#"builtins.removeAttrs { a = 1; b = 2; c = 3; } [ "b" ]"#),
    r#"{"a":1,"c":3}"#
  );
  assert_eq!(json(r#"builtins.removeAttrs { a = 1; } []"#), r#"{"a":1}"#);
  // Removing a missing key is still a no-op (per Nix).
  assert_eq!(
    json(r#"builtins.removeAttrs { a = 1; } [ "missing" ]"#),
    r#"{"a":1}"#
  );
}

#[test]
fn remove_attrs_non_attrset_first_arg_errors() {
  // Was: silently returned `{}`.
  let m = err_msg(r#"builtins.removeAttrs 42 [ "x" ]"#);
  assert!(m.contains("first argument must be attrset"), "got: {m}");
  assert!(m.contains("got int"), "got: {m}");
}

#[test]
fn remove_attrs_non_list_second_arg_errors() {
  // Was: silently returned the original attrset.
  let m = err_msg(r#"builtins.removeAttrs { a = 1; } 42"#);
  assert!(
    m.contains("second argument must be list of strings"),
    "got: {m}"
  );
  assert!(m.contains("got int"), "got: {m}");
}

#[test]
fn remove_attrs_non_string_in_names_list_errors() {
  // Was: silently filtered out the non-string entry; original
  // attrset returned. Now errors at index 0 with the bad type.
  let m = err_msg(r#"builtins.removeAttrs { a = 1; } [ 42 ]"#);
  assert!(m.contains("name-list element at index 0"), "got: {m}");
  assert!(m.contains("not a string"), "got: {m}");
  assert!(m.contains("int"), "got: {m}");
}

#[test]
fn remove_attrs_thunk_in_names_list_resolves() {
  // The element is forced before being checked, so a thunk
  // resolving to a string is fine.
  assert_eq!(
    json(r#"builtins.removeAttrs { a = 1; b = 2; } [ ("b" + "") ]"#),
    r#"{"a":1}"#
  );
}

// ── builtins.concatLists type guards ──────────────────────────

#[test]
fn concat_lists_happy_path_still_works() {
  assert_eq!(json("builtins.concatLists [[1 2] [3 4] []]"), "[1,2,3,4]");
  assert_eq!(json("builtins.concatLists []"), "[]");
  // Single nested list flattens to its contents.
  assert_eq!(json("builtins.concatLists [[1] [2] [3]]"), "[1,2,3]");
}

#[test]
fn concat_lists_non_list_arg_errors() {
  // Was: silently returned `[]`.
  let m = err_msg("builtins.concatLists 42");
  assert!(m.contains("argument must be list"), "got: {m}");
  assert!(m.contains("got int"), "got: {m}");
}

#[test]
fn concat_lists_inner_non_list_errors() {
  // Was: silently dropped non-list elements (silent-empty when
  // every element was non-list).
  let m = err_msg("builtins.concatLists [1 2]");
  assert!(m.contains("element at index 0"), "got: {m}");
  assert!(m.contains("not a list"), "got: {m}");
  assert!(m.contains("int"), "got: {m}");
}

#[test]
fn concat_lists_inner_partial_non_list_errors_at_first() {
  // The first list-shaped element doesn't mask the second
  // non-list element — we walk left to right.
  let m = err_msg("builtins.concatLists [[1 2] 42 [3]]");
  assert!(m.contains("element at index 1"), "got: {m}");
  assert!(m.contains("not a list"), "got: {m}");
}

#[test]
fn concat_lists_thunk_resolving_to_list_works() {
  // If the inner element is lazy and resolves to a list, that
  // is fine — the new code forces and then matches.
  assert_eq!(
    json("builtins.concatLists [ (if true then [1] else 0) [2] ]"),
    "[1,2]"
  );
}
