//! Regression cover for the bool-required positions: `if`,
//! `&&`, `||`, `->`, `!`, `assert`, match-arm guard, and the
//! pnix-only `builtins.and` / `builtins.or` / `builtins.not`
//! helpers.
//!
//! 2026-05-05 audit findings (slice #36):
//!   - **Every language operator that "wants a bool" was silently
//!     truthy-coercing.** Each site called
//!     `Value::is_true()`, which classifies any non-`false` /
//!     non-`null` / non-`Int(0)` value as truthy. So `if "yes"
//!     then a else b` followed `then`, `if 0 then a else b`
//!     followed `else`, and `if [ ] then a else b` followed
//!     `then` (because an empty list is "truthy" under
//!     `is_true()`). Real Nix errors with "value is a <type>
//!     while a Boolean was expected" at every one of these
//!     positions. A `.px` author writing
//!     `if maybeBool then ... else ...` where `maybeBool` came
//!     back from a deserialiser as `"true"` / `"false"` /
//!     `null` instead of an actual bool would silently follow
//!     the wrong branch — exactly the class of bug the
//!     production fail-loud contract is supposed to catch.
//!   - **`assert` had the same hole**: `assert "yes"; 42`
//!     silently returned `42`, `assert 0; 42` silently failed.
//!     Now both error with `assert: condition must be bool, got
//!     <type>`.
//!   - **`builtins.and` / `or` / `not`** (pnix-only helpers,
//!     don't exist in real Nix; added so `.px` authors can pipe
//!     boolean expressions through `map` / `foldl`) were also
//!     truthy-coercing. They now mirror the operator contract.
//!
//! Truth-owner files:
//! - `crates/pnix-eval/src/interpret.rs` — new `expect_bool`
//!   helper; `If` arms in tail loop and `eval_inner`; `Binary`
//!   short-circuit `&&` / `||` / `->`; `eval_binary` non-short-
//!   circuit fallback for the same; `eval_unary` `!`; `Assert`;
//!   match-arm guard at `Match`; `apply_builtin` `"and"` /
//!   `"or"` / `"not"` arms.
//!
//! Note on `->`: the parser desugars `a -> b` into
//! `(!a) || b`, so error messages for `->` route through the
//! `!` and `||` validators rather than naming `->` directly.
//! Tests below pin both halves so the desugaring stays
//! observable.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}
fn err_msg(src: &str) -> String {
  format!("{}", eval_expr(src).expect_err(src))
}

// ── `if` condition must be bool ───────────────────────────────

#[test]
fn if_bool_true() {
  assert_eq!(json("if true then 1 else 2"), "1");
}

#[test]
fn if_bool_false() {
  assert_eq!(json("if false then 1 else 2"), "2");
}

#[test]
fn if_string_errors() {
  let m = err_msg(r#"if "yes" then 1 else 2"#);
  assert!(m.contains("if condition"), "got: {m}");
  assert!(m.contains("expected bool"), "got: {m}");
  assert!(m.contains("got string"), "got: {m}");
}

#[test]
fn if_int_zero_errors() {
  // Was: silently followed `else` because Int(0) is "falsy".
  let m = err_msg("if 0 then 1 else 2");
  assert!(m.contains("expected bool"), "got: {m}");
  assert!(m.contains("got int"), "got: {m}");
}

#[test]
fn if_int_nonzero_errors() {
  let m = err_msg("if 42 then 1 else 2");
  assert!(m.contains("expected bool"), "got: {m}");
  assert!(m.contains("got int"), "got: {m}");
}

#[test]
fn if_null_errors() {
  let m = err_msg("if null then 1 else 2");
  assert!(m.contains("expected bool"), "got: {m}");
  assert!(m.contains("got null"), "got: {m}");
}

#[test]
fn if_list_errors() {
  // Was: silently followed `then` because non-empty list is
  // "truthy".
  let m = err_msg("if [ 1 ] then 1 else 2");
  assert!(m.contains("expected bool"), "got: {m}");
  assert!(m.contains("got list"), "got: {m}");
}

// ── `&&` short-circuit + bool-only ────────────────────────────

#[test]
fn and_bool_pair() {
  assert_eq!(json("true && true"), "true");
  assert_eq!(json("true && false"), "false");
  assert_eq!(json("false && true"), "false");
  assert_eq!(json("false && false"), "false");
}

#[test]
fn and_short_circuits_on_false_left() {
  // RHS never forced when LHS is false — throw on RHS doesn't fire.
  assert_eq!(json(r#"false && (throw "right")"#), "false");
}

#[test]
fn and_string_left_errors() {
  let m = err_msg(r#""yes" && true"#);
  assert!(m.contains("&&"), "got: {m}");
  assert!(m.contains("left operand"), "got: {m}");
  assert!(m.contains("got string"), "got: {m}");
}

#[test]
fn and_int_left_errors() {
  let m = err_msg("42 && true");
  assert!(m.contains("&&"), "got: {m}");
  assert!(m.contains("got int"), "got: {m}");
}

#[test]
fn and_null_left_errors() {
  // Was: silently returned `false` because `null.is_true() ==
  // false`. Now: error.
  let m = err_msg("null && true");
  assert!(m.contains("&&"), "got: {m}");
  assert!(m.contains("got null"), "got: {m}");
}

#[test]
fn and_string_right_errors() {
  // LHS true forces RHS; non-bool RHS is also an error now.
  let m = err_msg(r#"true && "yes""#);
  assert!(m.contains("&&"), "got: {m}");
  assert!(m.contains("right operand"), "got: {m}");
  assert!(m.contains("got string"), "got: {m}");
}

// ── `||` short-circuit + bool-only ────────────────────────────

#[test]
fn or_bool_pair() {
  assert_eq!(json("true || false"), "true");
  assert_eq!(json("false || true"), "true");
  assert_eq!(json("false || false"), "false");
}

#[test]
fn or_short_circuits_on_true_left() {
  assert_eq!(json(r#"true || (throw "right")"#), "true");
}

#[test]
fn or_string_left_errors() {
  let m = err_msg(r#""yes" || false"#);
  assert!(m.contains("||"), "got: {m}");
  assert!(m.contains("left operand"), "got: {m}");
  assert!(m.contains("got string"), "got: {m}");
}

#[test]
fn or_int_left_errors() {
  let m = err_msg("42 || false");
  assert!(m.contains("||"), "got: {m}");
  assert!(m.contains("got int"), "got: {m}");
}

#[test]
fn or_null_left_errors() {
  // Was: silently fell through to RHS because null is "falsy".
  let m = err_msg("null || false");
  assert!(m.contains("||"), "got: {m}");
  assert!(m.contains("got null"), "got: {m}");
}

#[test]
fn or_string_right_errors() {
  let m = err_msg(r#"false || "yes""#);
  assert!(m.contains("||"), "got: {m}");
  assert!(m.contains("right operand"), "got: {m}");
}

// ── `->` (parser desugars to `!lhs || rhs`) ───────────────────

#[test]
fn impl_bool_truth_table() {
  assert_eq!(json("true -> true"), "true");
  assert_eq!(json("true -> false"), "false");
  assert_eq!(json("false -> true"), "true");
  assert_eq!(json("false -> false"), "true");
}

#[test]
fn impl_short_circuits_on_false_left() {
  assert_eq!(json(r#"false -> (throw "rhs")"#), "true");
}

#[test]
fn impl_non_bool_left_errors() {
  // Routes through the `!` validator (parser desugar).
  let m = err_msg(r#""yes" -> true"#);
  assert!(m.contains("expected bool"), "got: {m}");
  assert!(m.contains("got string"), "got: {m}");
}

#[test]
fn impl_non_bool_right_errors() {
  // Routes through the `||` right-operand validator.
  let m = err_msg("true -> 42");
  assert!(m.contains("expected bool"), "got: {m}");
  assert!(m.contains("got int"), "got: {m}");
}

// ── `!` unary ─────────────────────────────────────────────────

#[test]
fn not_bool() {
  assert_eq!(json("!true"), "false");
  assert_eq!(json("!false"), "true");
}

#[test]
fn not_string_errors() {
  let m = err_msg(r#"!"yes""#);
  assert!(m.contains("!"), "got: {m}");
  assert!(m.contains("got string"), "got: {m}");
}

#[test]
fn not_int_errors() {
  let m = err_msg("!42");
  assert!(m.contains("!"), "got: {m}");
  assert!(m.contains("got int"), "got: {m}");
}

#[test]
fn not_null_errors() {
  // Was: silently `true` because `null.is_true() == false`.
  let m = err_msg("!null");
  assert!(m.contains("!"), "got: {m}");
  assert!(m.contains("got null"), "got: {m}");
}

// ── `assert` ──────────────────────────────────────────────────

#[test]
fn assert_true_passes() {
  assert_eq!(json("assert true; 42"), "42");
}

#[test]
fn assert_false_errors() {
  let m = err_msg("assert false; 42");
  assert!(m.contains("assertion failed"), "got: {m}");
}

#[test]
fn assert_string_errors() {
  // Was: silently passed because `"yes".is_true() == true`.
  let m = err_msg(r#"assert "yes"; 42"#);
  assert!(m.contains("assert"), "got: {m}");
  assert!(m.contains("must be bool"), "got: {m}");
  assert!(m.contains("got string"), "got: {m}");
}

#[test]
fn assert_int_errors() {
  let m = err_msg("assert 42; 0");
  assert!(m.contains("must be bool"), "got: {m}");
  assert!(m.contains("got int"), "got: {m}");
}

#[test]
fn assert_null_errors() {
  // Was: silently failed with "assertion failed". Now: typed
  // error pinning `null` so the user knows their predicate
  // returned the wrong type, not the wrong answer.
  let m = err_msg("assert null; 0");
  assert!(m.contains("must be bool"), "got: {m}");
  assert!(m.contains("got null"), "got: {m}");
}

// ── `builtins.and` / `or` / `not` ─────────────────────────────

#[test]
fn builtins_and_bool_pair() {
  assert_eq!(json("builtins.and true true"), "true");
  assert_eq!(json("builtins.and true false"), "false");
  assert_eq!(json("builtins.and false true"), "false");
}

#[test]
fn builtins_and_non_bool_errors() {
  let m = err_msg(r#"builtins.and "yes" true"#);
  assert!(m.contains("builtins.and"), "got: {m}");
  assert!(m.contains("got string"), "got: {m}");
}

#[test]
fn builtins_or_bool_pair() {
  assert_eq!(json("builtins.or true false"), "true");
  assert_eq!(json("builtins.or false false"), "false");
}

#[test]
fn builtins_or_non_bool_errors() {
  let m = err_msg("builtins.or 42 false");
  assert!(m.contains("builtins.or"), "got: {m}");
  assert!(m.contains("got int"), "got: {m}");
}

#[test]
fn builtins_not_bool() {
  assert_eq!(json("builtins.not true"), "false");
  assert_eq!(json("builtins.not false"), "true");
}

#[test]
fn builtins_not_non_bool_errors() {
  let m = err_msg("builtins.not null");
  assert!(m.contains("builtins.not"), "got: {m}");
  assert!(m.contains("got null"), "got: {m}");
}
