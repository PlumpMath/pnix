//! Regression cover for lambda introspection (`functionArgs`),
//! fold ops, `elem`, and `attrNames` / `attrValues`.
//!
//! 2026-05-04 audit findings:
//!   - **two silent silent-empty bugs**: `builtins.attrNames`
//!     and `builtins.attrValues` on a non-attrset returned `[]`
//!     instead of erroring. Both `attrNames []` and
//!     `attrValues "string"` would silently slip through user
//!     guards that branch on emptiness. Now error with
//!     `expected attrset, got <type>`.
//!   - the other 15 probe cases (functionArgs shape across
//!     positional / pattern / @-bind, foldl' empty + strict
//!     accumulator, elem semantics including lambda-never-equal
//!     and int/float coercion) already match Nix and are pinned
//!     alongside.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

fn err_msg(src: &str) -> String {
  format!("{}", eval_expr(src).expect_err(src))
}

// ── functionArgs (lambda introspection) ───────────────────────

#[test]
fn function_args_positional_lambda_is_empty() {
  // Positional lambdas (`x: x`) have no named formal params,
  // so functionArgs is `{}`.
  assert_eq!(json("builtins.functionArgs (x: x)"), "{}");
}

#[test]
fn function_args_attr_pattern_returns_required_map() {
  // `{a, b ? 1}: …` — required (`a`) is `false`, has-default
  // (`b`) is `true`.
  assert_eq!(
    json("builtins.functionArgs ({a, b ? 1}: a + b)"),
    r#"{"a":false,"b":true}"#
  );
}

#[test]
fn function_args_with_at_bind_same_shape() {
  // `args@{a, b ? 1}` doesn't change the formal map; @-bind only
  // adds a name for the whole arg.
  assert_eq!(
    json("builtins.functionArgs (args@{a, b ? 1}: a + b)"),
    r#"{"a":false,"b":true}"#
  );
}

#[test]
fn function_args_on_non_function_errors() {
  let m = err_msg("builtins.functionArgs 42");
  assert!(m.contains("expected function"), "got: {m}");
}

// ── foldl' ────────────────────────────────────────────────────

#[test]
fn foldl_empty_list_returns_initial() {
  assert_eq!(json("builtins.foldl' (a: b: a + b) 0 []"), "0");
}

#[test]
fn foldl_basic_sum() {
  assert_eq!(json("builtins.foldl' (a: b: a + b) 0 [ 1 2 3 ]"), "6");
}

#[test]
fn foldl_throw_in_function_propagates() {
  // foldl' is strict in the accumulator — a throw inside the
  // step function surfaces immediately.
  let m = err_msg(r#"builtins.foldl' (a: b: throw "step") 0 [ 1 ]"#);
  assert!(m.contains("step"), "got: {m}");
}

#[test]
fn foldl_string_concat_via_acc() {
  // Accumulator type can be anything as long as the step
  // function is consistent.
  assert_eq!(
    json(r#"builtins.foldl' (a: b: a + b) "" [ "a" "b" "c" ]"#),
    r#""abc""#
  );
}

// ── builtins.elem ─────────────────────────────────────────────

#[test]
fn elem_finds_int() {
  assert_eq!(json("builtins.elem 2 [ 1 2 3 ]"), "true");
  assert_eq!(json("builtins.elem 5 [ 1 2 3 ]"), "false");
}

#[test]
fn elem_finds_string() {
  assert_eq!(json(r#"builtins.elem "x" [ "a" "x" "c" ]"#), "true");
}

#[test]
fn elem_int_float_coerce_in_equality() {
  // Mirror the `1 == 1.0` rule — elem uses values_equal.
  assert_eq!(json("builtins.elem 1 [ 1.0 ]"), "true");
}

#[test]
fn elem_lambda_never_equal_so_never_found() {
  // Lambdas are exempt from structural equality, so elem of a
  // lambda in a list of "the same" lambda is still false.
  assert_eq!(json("let f = x: x; in builtins.elem f [ f ]"), "false");
}

#[test]
fn elem_empty_list_returns_false() {
  assert_eq!(json("builtins.elem 1 []"), "false");
}

// ── attrNames / attrValues ────────────────────────────────────

#[test]
fn attr_names_sorted_lex() {
  assert_eq!(
    json(r#"builtins.attrNames { c = 3; a = 1; b = 2; }"#),
    r#"["a","b","c"]"#
  );
}

#[test]
fn attr_values_sorted_by_name() {
  // attrValues iterates in BTreeMap (lex by name) order, so
  // `c=3; a=1; b=2;` → `[1,2,3]`.
  assert_eq!(
    json(r#"builtins.attrValues { c = 3; a = 1; b = 2; }"#),
    "[1,2,3]"
  );
}

#[test]
fn attr_values_empty() {
  assert_eq!(json("builtins.attrValues { }"), "[]");
}

#[test]
fn attr_names_on_list_errors() {
  // Was: silent `[]`. Now: type error.
  let m = err_msg("builtins.attrNames [ 1 2 ]");
  assert!(m.contains("expected attrset"), "got: {m}");
  assert!(m.contains("got list"), "got: {m}");
}

#[test]
fn attr_values_on_list_errors() {
  let m = err_msg("builtins.attrValues [ 1 2 ]");
  assert!(m.contains("expected attrset"), "got: {m}");
}

#[test]
fn attr_names_on_string_errors() {
  let m = err_msg(r#"builtins.attrNames "hello""#);
  assert!(m.contains("expected attrset"), "got: {m}");
}
