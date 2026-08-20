//! Regression cover for `builtins.pow` integer precision.
//!
//! 2026-05-05 audit findings (slice #46):
//!   - `builtins.pow 2 63` returned the int `9223372036854775807`
//!     (i64::MAX) but the true value is 2^63 =
//!     9223372036854775808 (one MORE than i64::MAX). pnix
//!     silently saturated to i64::MAX with type "int" instead
//!     of either erroring or promoting to float. A `.px`
//!     author who wrote `pow 2 63 == someExpected` would get
//!     `false` for any expected value other than i64::MAX —
//!     hidden wrong result.
//!   - `builtins.pow 3 39` returned `4052555153018976256` but
//!     the true value is `4052555153018976267` — off by 11
//!     because the f64 round-trip lost precision on integers
//!     bigger than 2^53.
//!   - The bug was in `collapse_numeric`: the helper computed
//!     `f64.powf(b)` and then tried to coerce back to int
//!     when both operands were int. Its boundary check `<=
//!     i64::MAX as f64` let through values that, when cast to
//!     i64, saturate to i64::MAX silently. And its precision
//!     tolerance `(rounded - result).abs() < f64::EPSILON`
//!     was effectively zero for large floats — every integer
//!     past 2^53 is "close enough" to itself trivially, even
//!     if the f64 representation has lost low-order bits.
//!   - Fix: for int^int with non-negative exponent, use
//!     `i64::checked_pow` (exact integer arithmetic). On
//!     overflow, fall back to `f64.powf` so the result is at
//!     least a Float (the user can see they hit the float
//!     domain). Negative exponent / float operands keep the
//!     float path. `collapse_numeric` is removed (was its
//!     only caller).
//!
//! Truth-owner files:
//! - `crates/pnix-eval/src/interpret.rs` `apply_builtin`
//!   `"pow"` arm — int^int with non-negative exp uses
//!   `i64::checked_pow`; falls through to `f64.powf` on
//!   overflow / negative exp / non-int operands.
//! - `crates/pnix-eval/src/interpret.rs` `collapse_numeric`
//!   — removed (dead code; replaced with a comment block
//!   explaining why future try-int-then-float promotion
//!   should use `checked_X` exactly).

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

// ── int^int exact paths ────────────────────────────────────────

#[test]
fn pow_2_62_exact() {
  // 2^62 = 4611686018427387904 — fits in i64 exactly.
  assert_eq!(json("builtins.pow 2 62"), "4611686018427387904");
  assert_eq!(json("builtins.typeOf (builtins.pow 2 62)"), r#""int""#);
}

#[test]
fn pow_3_39_exact() {
  // Was: 4052555153018976256 (off by 11). Now: exact.
  assert_eq!(json("builtins.pow 3 39"), "4052555153018976267");
  assert_eq!(json("builtins.typeOf (builtins.pow 3 39)"), r#""int""#);
}

#[test]
fn pow_simple_cases() {
  assert_eq!(json("builtins.pow 2 10"), "1024");
  assert_eq!(json("builtins.pow 5 3"), "125");
  assert_eq!(json("builtins.pow 0 0"), "1");
  assert_eq!(json("builtins.pow 1 100"), "1");
  assert_eq!(json("builtins.pow 0 5"), "0");
}

#[test]
fn pow_negative_base_odd_works() {
  // (-2)^3 = -8.
  assert_eq!(json("builtins.pow (-2) 3"), "-8");
  assert_eq!(json("builtins.pow (-2) 4"), "16");
}

// ── int^int overflow → float fallback ─────────────────────────

#[test]
fn pow_2_63_overflow_falls_back_to_float() {
  // Was: silently returned i64::MAX as int. Now: Float
  // (approximate but at least typed as float so the user
  // sees they hit the float representation domain).
  assert_eq!(json("builtins.typeOf (builtins.pow 2 63)"), r#""float""#);
}

#[test]
fn pow_2_64_falls_back_to_float() {
  assert_eq!(json("builtins.typeOf (builtins.pow 2 64)"), r#""float""#);
}

#[test]
fn pow_3_40_falls_back_to_float() {
  // 3^40 > i64::MAX → float.
  assert_eq!(json("builtins.typeOf (builtins.pow 3 40)"), r#""float""#);
}

// ── float operands keep float path ─────────────────────────────

#[test]
fn pow_float_base_returns_float() {
  // Pre-existing path: float operand → float result.
  assert_eq!(json("builtins.pow 2.5 2"), "6.25");
}

#[test]
fn pow_negative_exponent_returns_float() {
  // Negative exponent → fractional → float.
  assert_eq!(json("builtins.pow 2 (-1)"), "0.5");
  assert_eq!(json("builtins.pow 4 (-2)"), "0.0625");
}

#[test]
fn pow_non_number_errors() {
  let m = format!(
    "{}",
    eval_expr(r#"builtins.pow "x" 2"#).expect_err("non-number base")
  );
  assert!(m.contains("expected number"), "got: {m}");
}

// ── round-trip property ────────────────────────────────────────

#[test]
fn pow_round_trip_within_int_range() {
  // For any reasonable int base/exp where the result fits in
  // i64, the result IS the exact integer.
  for (a, b, expected) in [
    (2_i64, 30_i64, "1073741824"),
    (3_i64, 20_i64, "3486784401"),
    (10_i64, 15_i64, "1000000000000000"),
    (7_i64, 22_i64, "3909821048582988049"),
  ] {
    let src = format!("builtins.pow {} {}", a, b);
    assert_eq!(json(&src), expected, "for {}^{}", a, b);
  }
}
