//! Regression cover for `builtins.toJSON` lambda guard,
//! `builtins.tryEval` abort-propagation contract, and
//! `builtins.sort` predicate / argument-shape guards.
//!
//! 2026-05-05 audit findings (slice #35):
//!   - **`builtins.toJSON` silently flattened lambdas to placeholder
//!     strings**: `builtins.toJSON (x: x)` returned the string
//!     `"<lambda:x>"`, and `builtins.toJSON [ (x: x) ]` returned
//!     `["<lambda:x>"]`. Real Nix errors with "cannot convert a
//!     function to JSON". A user piping a config attrset to
//!     `toJSON` for sidecar persistence would silently leak a
//!     non-roundtrippable placeholder. Fixed by extending
//!     `check_json_finite` (the pre-`to_json` validator) to also
//!     reject `Value::Lambda` and `Value::BuiltinPartial` with the
//!     "cannot serialize function as JSON" shape.
//!   - **`builtins.tryEval` was catching `abort`**: `tryEval (abort
//!     "boom")` returned `{success=false; value=false;}`,
//!     silently turning the language's hard-fail into a
//!     recoverable signal. Nix specifies that `abort` is
//!     unrecoverable and propagates through `tryEval`. Fixed by
//!     re-raising errors whose message starts with
//!     "evaluation aborted:" inside the `tryEval` lazy intercept.
//!     Non-abort errors (`throw`, type errors, division by zero)
//!     still get wrapped as `{success=false;}` per Nix semantics.
//!   - **`builtins.sort` was truthy-coercing the comparator**:
//!     `sort (a: b: 42) [1 2]` returned `[1,2]` because anything
//!     other than `Bool(true)` was treated as `Greater`. And
//!     `sort (a: b: a < b) 42` silently returned `42` (non-list
//!     identity passthrough). Now both error: the comparator must
//!     return bool (matches `filter` / `partition` / `any` / `all`
//!     family pinned in slices #33/#34), and the second argument
//!     must be a list.
//!
//! Truth-owner files:
//! - `crates/pnix-eval/src/interpret.rs` — `check_json_finite`
//!   (Lambda/BuiltinPartial arms); `apply_builtin` `"sort"` arm;
//!   `maybe_apply_lazy_builtin` `("tryEval", 0)` arm.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}
fn err_msg(src: &str) -> String {
  format!("{}", eval_expr(src).expect_err(src))
}

// ── builtins.toJSON lambda guard ──────────────────────────────

#[test]
fn to_json_concrete_values_pass_through() {
  // Sanity: existing JSON shapes still work.
  assert_eq!(json("builtins.toJSON 42"), "\"42\"");
  assert_eq!(json("builtins.toJSON null"), "\"null\"");
  assert_eq!(json("builtins.toJSON [ 1 2 3 ]"), "\"[1,2,3]\"");
  assert_eq!(
    json(r#"builtins.toJSON { a = 1; b = "x"; }"#),
    r#""{\"a\":1,\"b\":\"x\"}""#
  );
}

#[test]
fn to_json_lambda_errors() {
  // Was: returned the string `"<lambda:x>"`.
  let m = err_msg("builtins.toJSON (x: x)");
  assert!(m.contains("cannot serialize function"), "got: {m}");
}

#[test]
fn to_json_lambda_inside_list_errors() {
  // The validator must descend into lists.
  let m = err_msg("builtins.toJSON [ (x: x) ]");
  assert!(m.contains("cannot serialize function"), "got: {m}");
}

#[test]
fn to_json_lambda_inside_attrset_errors() {
  let m = err_msg(r#"builtins.toJSON { f = (x: x); }"#);
  assert!(m.contains("cannot serialize function"), "got: {m}");
}

#[test]
fn to_json_builtin_partial_errors() {
  // `builtins.add 1` is a BuiltinPartial — also a function.
  let m = err_msg("builtins.toJSON (builtins.add 1)");
  assert!(m.contains("cannot serialize function"), "got: {m}");
}

// ── builtins.tryEval abort propagation ────────────────────────

#[test]
fn try_eval_catches_throw() {
  // Non-abort errors still get wrapped — that's the whole point
  // of tryEval.
  assert_eq!(
    json(r#"(builtins.tryEval (throw "boom")).success"#),
    "false"
  );
  assert_eq!(json(r#"(builtins.tryEval (throw "boom")).value"#), "false");
}

#[test]
fn try_eval_catches_assert() {
  assert_eq!(
    json(r#"(builtins.tryEval (assert false; 1)).success"#),
    "false"
  );
}

#[test]
fn try_eval_passes_through_success() {
  assert_eq!(json(r#"(builtins.tryEval 42).success"#), "true");
  assert_eq!(json(r#"(builtins.tryEval 42).value"#), "42");
}

#[test]
fn try_eval_does_not_catch_abort() {
  // Was: returned `{success=false; value=false;}`. Nix-correct:
  // abort propagates, tryEval errors out with the abort message.
  let m = err_msg(r#"builtins.tryEval (abort "boom")"#);
  assert!(m.contains("evaluation aborted"), "got: {m}");
  assert!(m.contains("boom"), "got: {m}");
}

#[test]
fn try_eval_abort_propagates_through_outer_attrset() {
  // Even when the abort is nested inside a tryEval that's itself
  // inside an attrset access, the abort still wins.
  let m = err_msg(r#"(builtins.tryEval (abort "deep")).success"#);
  assert!(m.contains("evaluation aborted"), "got: {m}");
  assert!(m.contains("deep"), "got: {m}");
}

// ── builtins.sort guards ──────────────────────────────────────

#[test]
fn sort_basic_ascending() {
  assert_eq!(json("builtins.sort (a: b: a < b) [ 3 1 2 ]"), "[1,2,3]");
}

#[test]
fn sort_basic_descending() {
  assert_eq!(json("builtins.sort (a: b: a > b) [ 1 3 2 ]"), "[3,2,1]");
}

#[test]
fn sort_empty_list() {
  assert_eq!(json("builtins.sort (a: b: a < b) []"), "[]");
}

#[test]
fn sort_singleton() {
  assert_eq!(json("builtins.sort (a: b: a < b) [ 42 ]"), "[42]");
}

#[test]
fn sort_non_bool_comparator_errors() {
  // Was: silently returned the input list because anything other
  // than Bool(true) was treated as Greater.
  let m = err_msg("builtins.sort (a: b: 42) [ 1 2 ]");
  assert!(m.contains("comparator must return bool"), "got: {m}");
  assert!(m.contains("got int"), "got: {m}");
}

#[test]
fn sort_string_comparator_errors() {
  let m = err_msg(r#"builtins.sort (a: b: "yes") [ 1 2 ]"#);
  assert!(m.contains("comparator must return bool"), "got: {m}");
  assert!(m.contains("got string"), "got: {m}");
}

#[test]
fn sort_non_list_second_arg_errors() {
  // Was: silently returned the non-list arg unchanged.
  let m = err_msg("builtins.sort (a: b: a < b) 42");
  assert!(m.contains("second argument must be list"), "got: {m}");
  assert!(m.contains("got int"), "got: {m}");
}

#[test]
fn sort_non_list_attrset_errors() {
  let m = err_msg(r#"builtins.sort (a: b: a < b) { x = 1; }"#);
  assert!(m.contains("second argument must be list"), "got: {m}");
  // pnix's `type_name` follows Nix convention (`set`, not `attrset`).
  assert!(m.contains("got set"), "got: {m}");
}

#[test]
fn sort_comparator_throw_propagates() {
  // If the comparator throws, the sort fails with that error
  // (not silently gives up and returns identity).
  let m = err_msg(r#"builtins.sort (a: b: throw "cmp") [ 1 2 ]"#);
  assert!(m.contains("cmp"), "got: {m}");
}
