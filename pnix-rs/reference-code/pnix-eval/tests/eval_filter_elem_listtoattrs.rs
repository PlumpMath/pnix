//! Regression cover for `builtins.elem` non-list guard,
//! `builtins.filter` non-bool predicate guard, and
//! `builtins.listToAttrs` Nix-canonical first-wins on duplicate
//! names.
//!
//! 2026-05-04 audit findings:
//!   - **`builtins.elem _ x` silent-false on non-list `x`**:
//!     `elem 1 42`, `elem 1 null`, `elem 1 "abc"`, `elem 1 { a=1; }`
//!     all returned `false`. A `.px` guard like
//!     `if builtins.elem k xs then … else …` would silently take
//!     the else-branch for any `xs` that wasn't a list (and never
//!     learn the input shape was wrong). Now errors.
//!   - **`builtins.filter` truthy-coerced non-bool predicate**:
//!     `filter (x: x) [1 2 3]` returned `[1 2 3]` (every int ≠ 0
//!     was treated as truthy), and `filter (x: 42) [1]` returned
//!     `[1]`. `filterAttrs` already errored on non-bool — `filter`
//!     was the inconsistent one. Now both behave the same.
//!   - **`builtins.listToAttrs` last-wins on duplicate name**: pnix
//!     used `BTreeMap::insert` which overwrites, but Nix's
//!     `prim_listToAttrs` (`nix/src/libexpr/primops.cc`) skips
//!     duplicates via `seen.insert(name).second` — so the **first**
//!     entry wins. The previous in-source comment claimed
//!     "Nix-compat: last-wins", but that was wrong; reproducible
//!     under `nix-instantiate --eval --strict -E
//!     'builtins.listToAttrs [{name="a";value=1;}{name="a";value=2;}]'`
//!     → `{ a = 1; }`. Switched to `BTreeMap::entry().or_insert()`.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}
fn err_msg(src: &str) -> String {
  format!("{}", eval_expr(src).expect_err(src))
}

// ── builtins.elem non-list guard ──────────────────────────────

#[test]
fn elem_happy_path_still_works() {
  assert_eq!(json("builtins.elem 1 [ 1 2 3 ]"), "true");
  assert_eq!(json("builtins.elem 4 [ 1 2 3 ]"), "false");
  assert_eq!(json("builtins.elem 1 [ ]"), "false");
  assert_eq!(json(r#"builtins.elem "a" [ "a" "b" ]"#), "true");
}

#[test]
fn elem_non_list_int_errors() {
  let m = err_msg("builtins.elem 1 42");
  assert!(m.contains("second argument must be list"), "got: {m}");
  assert!(m.contains("got int"), "got: {m}");
}

#[test]
fn elem_non_list_null_errors() {
  let m = err_msg("builtins.elem 1 null");
  assert!(m.contains("second argument must be list"), "got: {m}");
  assert!(m.contains("got null"), "got: {m}");
}

#[test]
fn elem_non_list_string_errors() {
  let m = err_msg(r#"builtins.elem 1 "abc""#);
  assert!(m.contains("second argument must be list"), "got: {m}");
  assert!(m.contains("got string"), "got: {m}");
}

#[test]
fn elem_non_list_attrset_errors() {
  let m = err_msg("builtins.elem 1 { a = 1; }");
  assert!(m.contains("second argument must be list"), "got: {m}");
  assert!(m.contains("got set"), "got: {m}");
}

// ── builtins.filter non-bool predicate guard ──────────────────

#[test]
fn filter_bool_predicate_works() {
  assert_eq!(json("builtins.filter (x: x > 1) [ 1 2 3 ]"), "[2,3]");
  assert_eq!(json("builtins.filter (x: false) [ 1 2 3 ]"), "[]");
  assert_eq!(json("builtins.filter (x: true) [ 1 2 3 ]"), "[1,2,3]");
  assert_eq!(json("builtins.filter (x: x > 0) []"), "[]");
}

#[test]
fn filter_int_predicate_errors_at_first_offender() {
  // Was: every int treated as truthy → list returned unchanged.
  let m = err_msg("builtins.filter (x: x) [ 1 2 3 ]");
  assert!(m.contains("predicate must return bool"), "got: {m}");
  assert!(m.contains("got int"), "got: {m}");
  assert!(m.contains("at index 0"), "got: {m}");
}

#[test]
fn filter_constant_int_predicate_errors() {
  let m = err_msg("builtins.filter (x: 42) [ 1 ]");
  assert!(m.contains("predicate must return bool"), "got: {m}");
}

#[test]
fn filter_string_predicate_errors() {
  let m = err_msg(r#"builtins.filter (x: "yes") [ 1 ]"#);
  assert!(m.contains("predicate must return bool"), "got: {m}");
  assert!(m.contains("got string"), "got: {m}");
}

#[test]
fn filter_non_list_second_arg_errors() {
  let m = err_msg("builtins.filter (x: true) 42");
  assert!(m.contains("second argument must be list"), "got: {m}");
  assert!(m.contains("got int"), "got: {m}");
}

#[test]
fn filter_predicate_index_reported_correctly() {
  // Bool then int → error should pin the int position (index 1).
  let m = err_msg("builtins.filter (x: if x == 1 then true else x) [ 1 2 ]");
  assert!(m.contains("predicate must return bool"), "got: {m}");
  assert!(m.contains("at index 1"), "got: {m}");
}

// ── builtins.listToAttrs duplicate-name first-wins ────────────

#[test]
fn list_to_attrs_unique_names_works() {
  assert_eq!(
    json(r#"builtins.listToAttrs [ { name = "a"; value = 1; } { name = "b"; value = 2; } ]"#),
    r#"{"a":1,"b":2}"#
  );
}

#[test]
fn list_to_attrs_duplicate_first_wins() {
  // Nix says first wins. Verified against
  // `nix-instantiate --eval --strict -E
  //  'builtins.listToAttrs [{name="a";value=1;}{name="a";value=2;}]'`
  // → `{ a = 1; }`. Pre-fix pnix-eval returned `{a:2}`.
  assert_eq!(
    json(r#"builtins.listToAttrs [ { name = "a"; value = 1; } { name = "a"; value = 2; } ]"#),
    r#"{"a":1}"#
  );
}

#[test]
fn list_to_attrs_duplicate_first_wins_three_way() {
  // Three duplicates collapse to the first.
  assert_eq!(
    json(
      r#"builtins.listToAttrs [
           { name = "x"; value = 10; }
           { name = "x"; value = 20; }
           { name = "x"; value = 30; }
         ]"#
    ),
    r#"{"x":10}"#
  );
}

#[test]
fn list_to_attrs_duplicate_value_lazy_kept() {
  // The losing entry's `value` is never forced. A losing entry
  // that holds `throw` would burn if we accidentally evaluated it.
  assert_eq!(
    json(
      r#"builtins.listToAttrs [
           { name = "k"; value = 1; }
           { name = "k"; value = throw "x"; }
         ]"#
    ),
    r#"{"k":1}"#
  );
}
