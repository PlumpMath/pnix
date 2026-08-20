//! Regression cover for i64::MIN-edge integer overflow panics
//! in `builtins.mod`, `builtins.neg`, and unary `-`.
//!
//! 2026-05-05 audit findings (slice #47):
//!   - `builtins.mod (i64::MIN) (-1)` panicked with "attempt
//!     to calculate the remainder with overflow". The `mod`
//!     arm used Rust's plain `%` operator, which Rust's
//!     overflow rules reject for i64::MIN % -1 (the
//!     intermediate `i64::MIN / -1` is the canonical two's-
//!     complement overflow case). The mathematical result is
//!     `0` and would be safe if computed correctly, but the
//!     plain `%` operator panics on the input combination.
//!     The binary `%` operator (in `arith_op`) already used
//!     `checked_rem` — `builtins.mod` was the missing twin.
//!   - `-(i64::MIN)` panicked with "attempt to negate with
//!     overflow" (i64::MIN has no positive counterpart in
//!     i64). Both the unary `-` operator and `builtins.neg`
//!     were affected. Fix: `i64::checked_neg`.
//!   - `builtins.sub` with i64::MIN was already covered by
//!     slice #44's `arith_binary` rewrite (uses
//!     `i64::checked_sub`); pinned here for parity.
//!
//! Note: i64::MIN itself is not constructible as a literal
//! (parser rejects `-9223372036854775808` because the
//! `9223372036854775808` part exceeds i64::MAX before the
//! unary minus is applied — see slice #39 probe). All tests
//! in this file construct i64::MIN via arithmetic
//! (`0 - 9223372036854775807 - 1`) which subtraction-
//! overflow-checks correctly via the binary `-` operator.
//!
//! Truth-owner files:
//! - `crates/pnix-eval/src/interpret.rs` `apply_builtin`
//!   `"mod"` arm — uses `i64::checked_rem` for the int/int
//!   path.
//! - `crates/pnix-eval/src/interpret.rs` `apply_builtin`
//!   `"neg"` arm — uses `i64::checked_neg`.
//! - `crates/pnix-eval/src/interpret.rs` `eval_unary` `"-"`
//!   arm — uses `i64::checked_neg`.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}
fn err_msg(src: &str) -> String {
  format!("{}", eval_expr(src).expect_err(src))
}

// ── i64::MIN constructible via arithmetic ──────────────────────

#[test]
fn i64_min_constructible_via_subtraction() {
  // The parser rejects `-9223372036854775808` as a literal,
  // but `0 - 9223372036854775807 - 1` reaches it through
  // checked subtraction.
  assert_eq!(
    json("let big = 9223372036854775807; in 0 - big - 1"),
    "-9223372036854775808"
  );
}

// ── builtins.mod i64::MIN % -1 ────────────────────────────────

#[test]
fn builtins_mod_i64_min_neg_one_errors() {
  // Was: Rust panic "attempt to calculate the remainder with
  // overflow". Now: typed integer overflow error.
  let m = err_msg("let big = 9223372036854775807; m = 0 - big - 1; in builtins.mod m (-1)");
  assert!(m.contains("integer overflow"), "got: {m}");
  assert!(m.contains("%"), "got: {m}");
}

#[test]
fn binary_mod_i64_min_neg_one_errors() {
  // The binary `%` operator already had this protection
  // (slice-#X earlier). Pin parity with the builtin.
  let m = err_msg("let big = 9223372036854775807; m = 0 - big - 1; in m % (-1)");
  assert!(m.contains("integer overflow"), "got: {m}");
}

#[test]
fn builtins_mod_zero_errors() {
  let m = err_msg("builtins.mod 1 0");
  assert!(m.contains("division by zero"), "got: {m}");
}

#[test]
fn builtins_mod_normal_works() {
  assert_eq!(json("builtins.mod 10 3"), "1");
  assert_eq!(json("builtins.mod (-10) 3"), "-1");
  assert_eq!(json("builtins.mod 10 (-3)"), "1");
  // -i64::MAX % -1 = 0 (no overflow because -i64::MAX fits)
  assert_eq!(
    json("let big = 9223372036854775807; m = 0 - big; in builtins.mod m (-1)"),
    "0"
  );
}

// ── unary `-` on i64::MIN ──────────────────────────────────────

#[test]
fn unary_neg_i64_min_errors() {
  // Was: Rust panic "attempt to negate with overflow".
  let m = err_msg("let big = 9223372036854775807; m = 0 - big - 1; in -m");
  assert!(m.contains("integer overflow"), "got: {m}");
}

#[test]
fn unary_neg_normal_works() {
  assert_eq!(json("-5"), "-5");
  assert_eq!(json("- 5"), "-5");
  assert_eq!(json("- (- 5)"), "5");
  assert_eq!(json("-9223372036854775807"), "-9223372036854775807");
  assert_eq!(json("-(-9223372036854775807)"), "9223372036854775807");
}

// ── builtins.neg on i64::MIN ───────────────────────────────────

#[test]
fn builtins_neg_i64_min_errors() {
  let m = err_msg("let big = 9223372036854775807; m = 0 - big - 1; in builtins.neg m");
  assert!(m.contains("integer overflow"), "got: {m}");
}

#[test]
fn builtins_neg_normal_works() {
  assert_eq!(json("builtins.neg 5"), "-5");
  assert_eq!(json("builtins.neg (-5)"), "5");
  assert_eq!(json("builtins.neg 0"), "0");
  assert_eq!(
    json("builtins.neg 9223372036854775807"),
    "-9223372036854775807"
  );
}

#[test]
fn builtins_neg_float_unchanged() {
  // Float negation has no overflow concern.
  assert_eq!(json("builtins.neg 1.5"), "-1.5");
  assert_eq!(json("builtins.neg (-2.5)"), "2.5");
  assert_eq!(json("builtins.neg 0.0"), "-0.0");
}

// ── builtins.sub i64::MIN parity (slice #44 invariant) ─────────

#[test]
fn builtins_sub_i64_min_minus_one_errors() {
  // i64::MIN - 1 underflows. Slice #44's arith_binary already
  // routes builtins.sub through checked_sub; pinning here for
  // parity with the new mod / neg fixes.
  let m = err_msg("let big = 9223372036854775807; m = 0 - big - 1; in builtins.sub m 1");
  assert!(m.contains("integer overflow"), "got: {m}");
}

// ── parity: binary and builtin overflow same family ────────────

#[test]
fn binary_neg_and_builtins_neg_match() {
  let bin = err_msg("let big = 9223372036854775807; m = 0 - big - 1; in -m");
  let bul = err_msg("let big = 9223372036854775807; m = 0 - big - 1; in builtins.neg m");
  assert!(bin.contains("integer overflow"), "binary: {bin}");
  assert!(bul.contains("integer overflow"), "builtin: {bul}");
}

#[test]
fn binary_mod_and_builtins_mod_match() {
  let bin = err_msg("let big = 9223372036854775807; m = 0 - big - 1; in m % (-1)");
  let bul = err_msg("let big = 9223372036854775807; m = 0 - big - 1; in builtins.mod m (-1)");
  assert!(bin.contains("integer overflow"), "binary: {bin}");
  assert!(bul.contains("integer overflow"), "builtin: {bul}");
}
