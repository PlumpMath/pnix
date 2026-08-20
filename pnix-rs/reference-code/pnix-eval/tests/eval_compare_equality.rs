//! Audit-clean baselines for comparison and equality semantics.
//!
//! The 2026-05-04 mixed-type comparison / equality probe found no
//! semantic bugs — every case below already matched the Nix manual.
//! This file exists as a *regression pin* so future evaluator
//! changes (operator precedence rewrites, equality fast paths,
//! arithmetic optimisations) cannot silently break them.
//!
//! Production-readiness note: comparison errors are *fail-loud* —
//! pnix evaluates `0.0 / 0.0` as `division by zero` rather than
//! producing NaN and silently propagating it through `<` / `==`.
//! That trades a small slice of Nix-bit-compatibility (Nix permits
//! `nan < 1.0` to return `false`) for an error that points at the
//! actual divide. `.px` authors who need explicit NaN behaviour can
//! synthesize one with a future builtin or `.px` helper without the
//! evaluator silently swallowing arithmetic mistakes.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

fn err_msg(src: &str) -> String {
  format!("{}", eval_expr(src).expect_err(src))
}

// ── ordered comparison: type errors ─────────────────────────────────

#[test]
fn lt_int_with_string_errors() {
  let m = err_msg(r#"1 < "a""#);
  assert!(m.contains("cannot compare"), "got: {m}");
}

#[test]
fn lt_list_with_int_errors() {
  let m = err_msg("[1] < 1");
  assert!(m.contains("cannot compare"), "got: {m}");
}

#[test]
fn lt_null_with_null_errors() {
  // Nix: null is not orderable.
  let m = err_msg("null < null");
  assert!(m.contains("cannot compare"), "got: {m}");
}

// ── ordered comparison: numeric across int / float ──────────────────

#[test]
fn lt_int_float_promotes() {
  assert_eq!(json("1 < 1.5"), "true");
  assert_eq!(json("1.0 < 2"), "true");
  assert_eq!(json("2.0 < 2"), "false");
}

// ── ordered comparison: lex on strings, lists, paths ────────────────

#[test]
fn lt_string_lex() {
  assert_eq!(json(r#""a" < "b""#), "true");
  assert_eq!(json(r#""b" < "a""#), "false");
  assert_eq!(json(r#""a" < "a""#), "false");
}

#[test]
fn lt_list_lex() {
  assert_eq!(json("[1 2] < [1 3]"), "true");
  assert_eq!(json("[1 2] < [1 2]"), "false");
  // Shorter prefix is less than the longer one.
  assert_eq!(json("[1] < [1 2]"), "true");
}

// ── equality: type-strict, no coercion ──────────────────────────────

#[test]
fn eq_int_float_same_value_true() {
  // Numeric equality coerces across int/float (real Nix-correct).
  assert_eq!(json("1 == 1.0"), "true");
  assert_eq!(json("1.0 == 1"), "true");
}

#[test]
fn eq_int_string_returns_false_no_coercion() {
  // Strings vs ints are different types — never equal.
  assert_eq!(json(r#"1 == "1""#), "false");
}

#[test]
fn eq_null_anything() {
  assert_eq!(json("null == null"), "true");
  assert_eq!(json("null == 0"), "false");
  assert_eq!(json(r#"null == """#), "false");
  assert_eq!(json("null == false"), "false");
}

// ── lambda equality: never equal ───────────────────────────────────

#[test]
fn eq_lambda_lambda_always_false() {
  assert_eq!(json("(x: x) == (x: x)"), "false");
}

#[test]
fn eq_lambda_self_returns_false() {
  // Even `f == f` is false in Nix; functions are exempt from
  // structural equality.
  assert_eq!(json("let f = x: x; in f == f"), "false");
}

#[test]
fn eq_attrset_with_lambda_field_always_false() {
  // If any field is a lambda, the whole set is unequal even when
  // every other field matches.
  assert_eq!(
    json(r#"{ a = 1; f = x: x; } == { a = 1; f = x: x; }"#),
    "false"
  );
}

// ── deep structural equality ────────────────────────────────────────

#[test]
fn eq_list_deep_match() {
  assert_eq!(json("[1 [2 3]] == [1 [2 3]]"), "true");
}

#[test]
fn eq_list_deep_mismatch() {
  assert_eq!(json("[1 [2 3]] == [1 [2 4]]"), "false");
}

#[test]
fn eq_attrset_deep_match() {
  assert_eq!(
    json(r#"{ a = { b = 1; }; } == { a = { b = 1; }; }"#),
    "true"
  );
}

#[test]
fn eq_attrset_deep_mismatch() {
  assert_eq!(
    json(r#"{ a = { b = 1; }; } == { a = { b = 2; }; }"#),
    "false"
  );
}

#[test]
fn eq_attrset_extra_key_inequal() {
  assert_eq!(json("{ a = 1; b = 2; } == { a = 1; }"), "false");
  assert_eq!(json("{ a = 1; } == { a = 1; b = 2; }"), "false");
}

// ── divide-by-zero is fail-loud (no NaN propagation) ────────────────

#[test]
fn float_divide_by_zero_errors() {
  // pnix raises an error rather than producing NaN — that's a
  // deliberate fail-loud choice; see this file's header.
  let m = err_msg("0.0 / 0.0");
  assert!(m.contains("division by zero"), "got: {m}");
}

#[test]
fn float_zero_divide_in_modulo_errors() {
  let m = err_msg("0.0 % 0.0");
  assert!(m.contains("modulo by zero"), "got: {m}");
}
