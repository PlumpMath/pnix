//! Regression cover for lambda calling-convention corners and the
//! attrset-builtins family (`listToAttrs`, `mapAttrs`, `filterAttrs`).
//!
//! 2026-05-04 audit findings:
//!   - one real silent-skip bug: `builtins.listToAttrs` silently
//!     dropped entries missing `name` *or* `value`, returning
//!     `{}` instead of erroring. Now errors with the failing
//!     index + which attribute is missing.
//!   - the other 17 probe cases (curry depth, partial application,
//!     over-arg apply, lambda self-ref + capture, mapAttrs /
//!     filterAttrs basic + non-bool-predicate guard) already
//!     match Nix and are pinned alongside as regression baselines.
//!
//! Production-readiness contract: every attrset-builtin error
//! includes `builtins.X:` prefix + the index / argument that
//! failed + the type that was wrong, so `.px` authors can grep
//! the message family and fix typos at the call site.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

fn err_msg(src: &str) -> String {
  format!("{}", eval_expr(src).expect_err(src))
}

// ── lambda calling convention ─────────────────────────────────────

#[test]
fn curry_three_args() {
  assert_eq!(json("(a: b: c: a + b + c) 1 2 3"), "6");
}

#[test]
fn curry_partial_returns_lambda() {
  // 1 of 3 args applied → still a lambda (b is the next param).
  let v = eval_expr("(a: b: c: a + b + c) 1").unwrap();
  use pnix_eval::Value;
  assert!(matches!(v, Value::Lambda { .. }));
}

#[test]
fn over_arg_application_errors() {
  // `(x: x) 1 2` — first apply returns `1`, then trying to call
  // `1 2` errors with "cannot apply non-function value".
  let m = err_msg("(x: x) 1 2");
  assert!(m.contains("cannot apply non-function"), "got: {m}");
}

#[test]
fn lambda_self_ref_in_let() {
  // Recursion via let-bound name. Bounded depth so default test
  // thread stack survives.
  assert_eq!(
    json("let f = n: if n == 0 then 0 else n + f (n - 1); in f 5"),
    "15"
  );
}

#[test]
fn lambda_captures_outer_let() {
  assert_eq!(json("let x = 10; f = y: x + y; in f 5"), "15");
}

#[test]
fn lambda_arg_shadows_outer() {
  // Inner `x` (param) wins over outer `x` (let).
  assert_eq!(json("let x = 10; f = x: x + 1; in f 100"), "101");
}

// ── listToAttrs ──────────────────────────────────────────────────

#[test]
fn list_to_attrs_basic() {
  assert_eq!(
    json(r#"builtins.listToAttrs [ { name = "a"; value = 1; } { name = "b"; value = 2; } ]"#),
    r#"{"a":1,"b":2}"#
  );
}

#[test]
fn list_to_attrs_duplicate_names_first_wins() {
  // Nix-compat: duplicate names → the **first** entry wins. Mirrors
  // `nix/src/libexpr/primops.cc::prim_listToAttrs`, which guards the
  // insert with `seen.insert(name).second`. The earlier pin here
  // claimed last-wins; that was wrong. Verified against
  // `nix-instantiate --eval --strict -E
  //  'builtins.listToAttrs [{name="a";value=1;}{name="a";value=2;}]'`
  // → `{ a = 1; }`. Cross-cover:
  // `eval_filter_elem_listtoattrs.rs::list_to_attrs_duplicate_first_wins`.
  assert_eq!(
    json(r#"builtins.listToAttrs [ { name = "a"; value = 1; } { name = "a"; value = 2; } ]"#),
    r#"{"a":1}"#
  );
}

#[test]
fn list_to_attrs_empty_list() {
  assert_eq!(json(r#"builtins.listToAttrs []"#), "{}");
}

#[test]
fn list_to_attrs_missing_value_errors() {
  // Was: silent-skipped, returning {}. Now: error names the
  // index and which attribute is missing.
  let m = err_msg(r#"builtins.listToAttrs [ { name = "a"; } ]"#);
  assert!(m.contains("missing 'value'"), "got: {m}");
  assert!(m.contains("index 0"), "got: {m}");
}

#[test]
fn list_to_attrs_missing_name_errors() {
  let m = err_msg(r#"builtins.listToAttrs [ { value = 1; } ]"#);
  assert!(m.contains("missing 'name'"), "got: {m}");
}

#[test]
fn list_to_attrs_non_string_name_errors() {
  let m = err_msg(r#"builtins.listToAttrs [ { name = 42; value = 1; } ]"#);
  assert!(m.contains("'name' of type int"), "got: {m}");
}

#[test]
fn list_to_attrs_non_attrset_element_errors() {
  let m = err_msg(r#"builtins.listToAttrs [ "not-an-attrset" ]"#);
  assert!(m.contains("must be attrset"), "got: {m}");
  assert!(m.contains("index 0"), "got: {m}");
}

// ── mapAttrs ────────────────────────────────────────────────────

#[test]
fn map_attrs_basic() {
  assert_eq!(
    json(r#"builtins.mapAttrs (k: v: v * 2) { a = 1; b = 2; }"#),
    r#"{"a":2,"b":4}"#
  );
}

#[test]
fn map_attrs_uses_key() {
  assert_eq!(
    json(r#"builtins.mapAttrs (k: v: k + "=" + (builtins.toString v)) { x = 1; y = 2; }"#),
    r#"{"x":"x=1","y":"y=2"}"#
  );
}

#[test]
fn map_attrs_empty() {
  assert_eq!(json(r#"builtins.mapAttrs (k: v: v) { }"#), "{}");
}

#[test]
fn map_attrs_on_list_errors() {
  let m = err_msg(r#"builtins.mapAttrs (k: v: v) [ 1 2 ]"#);
  assert!(m.contains("mapAttrs"), "got: {m}");
}

// ── filterAttrs ─────────────────────────────────────────────────

#[test]
fn filter_attrs_basic() {
  assert_eq!(
    json(r#"builtins.filterAttrs (k: v: v > 1) { a = 1; b = 2; c = 3; }"#),
    r#"{"b":2,"c":3}"#
  );
}

#[test]
fn filter_attrs_empty() {
  assert_eq!(json(r#"builtins.filterAttrs (k: v: true) { }"#), "{}");
}

#[test]
fn filter_attrs_predicate_uses_key() {
  assert_eq!(
    json(r#"builtins.filterAttrs (k: v: k == "x") { x = 1; y = 2; }"#),
    r#"{"x":1}"#
  );
}

#[test]
fn filter_attrs_non_bool_predicate_errors() {
  // Real Nix and pnix both error when predicate returns
  // something other than a Bool — production fail-loud.
  let m = err_msg(r#"builtins.filterAttrs (k: v: 1) { a = 1; }"#);
  assert!(m.contains("must return bool"), "got: {m}");
}
