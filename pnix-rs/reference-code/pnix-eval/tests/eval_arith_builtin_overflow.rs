//! Regression cover for `builtins.add` / `sub` / `mul` / `div`
//! integer overflow panic.
//!
//! 2026-05-05 audit findings (slice #44):
//!   - **`builtins.add 9223372036854775807 1`** panicked with
//!     "attempt to add with overflow" — a Rust panic that
//!     crashes the evaluator process. In debug builds this
//!     fires immediately; in release builds Rust would have
//!     silently wrapped the value (which is even worse for
//!     production correctness — users would see
//!     `-9223372036854775808` instead of an error).
//!   - The binary `+` / `-` / `*` / `/` operators (in
//!     `arith_op` at line 1980) ALREADY used
//!     `checked_add` / `checked_sub` / `checked_mul` /
//!     `checked_div` and surfaced typed `integer overflow`
//!     errors. The four `builtins.X` arith helpers (`add` /
//!     `sub` / `mul` / `div`) were the missing twins —
//!     `arith_binary` took its `int_op` parameter as
//!     `fn(i64, i64) -> i64` and used plain `+` / `-` / `*`
//!     / `/` lambdas, which panic on overflow.
//!   - Real Nix and pnix's own binary path both error on
//!     integer overflow. Match that contract: change the
//!     `int_op` parameter to `fn(i64, i64) -> Option<i64>`
//!     so callers pass `i64::checked_add` etc., and surface
//!     `None` as a typed error message naming the offending
//!     operator.
//!
//! Truth-owner files:
//! - `crates/pnix-eval/src/interpret.rs` `arith_binary` —
//!   signature change from `fn(i64, i64) -> i64` to
//!   `fn(i64, i64) -> Option<i64>`. New `op_name: &str`
//!   parameter for the error message. Float ops are
//!   unchanged (Rust float arithmetic produces +inf / -inf
//!   on overflow without panicking; that's matched against
//!   real Nix, which also lets float overflow flow as +inf).
//! - `crates/pnix-eval/src/interpret.rs` `apply_builtin`
//!   `"add"` / `"sub"` / `"mul"` / `"div"` arms — call sites
//!   updated to pass `i64::checked_add` etc.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}
fn err_msg(src: &str) -> String {
  format!("{}", eval_expr(src).expect_err(src))
}

// ── builtins.add overflow ──────────────────────────────────────

#[test]
fn builtins_add_overflow_errors_not_panics() {
  let m = err_msg("builtins.add 9223372036854775807 1");
  assert!(m.contains("integer overflow"), "got: {m}");
  assert!(m.contains("+"), "got: {m}");
  assert!(m.contains("9223372036854775807"), "got: {m}");
}

#[test]
fn builtins_add_negative_overflow_errors() {
  // i64::MIN - 1 via builtins.add
  let m = err_msg("builtins.add (-9223372036854775807) (-2)");
  assert!(m.contains("integer overflow"), "got: {m}");
}

#[test]
fn builtins_add_int_int_works() {
  assert_eq!(json("builtins.add 1 2"), "3");
  assert_eq!(
    json("builtins.add 9223372036854775806 1"),
    "9223372036854775807"
  );
}

// ── builtins.sub overflow ──────────────────────────────────────

#[test]
fn builtins_sub_overflow_errors() {
  let m = err_msg("builtins.sub (-9223372036854775807) 2");
  assert!(m.contains("integer overflow"), "got: {m}");
  assert!(m.contains("-"), "got: {m}");
}

#[test]
fn builtins_sub_int_int_works() {
  assert_eq!(json("builtins.sub 5 3"), "2");
}

// ── builtins.mul overflow ──────────────────────────────────────

#[test]
fn builtins_mul_overflow_errors() {
  let m = err_msg("builtins.mul 9223372036854775807 9223372036854775807");
  assert!(m.contains("integer overflow"), "got: {m}");
  assert!(m.contains("*"), "got: {m}");
}

#[test]
fn builtins_mul_int_int_works() {
  assert_eq!(json("builtins.mul 3 4"), "12");
}

// ── builtins.div overflow ──────────────────────────────────────

#[test]
fn builtins_div_handles_neg_max_safely() {
  // i64::MIN / -1 IS the only int division overflow case (the
  // result `+i64::MAX + 1` overflows i64). But pnix's parser
  // rejects `-9223372036854775808` as a literal (slice #39
  // probe), so we can't directly construct i64::MIN. The
  // closest constructible case is -i64::MAX / -1, which is
  // i64::MAX (not overflow). This pin captures that the
  // common case doesn't panic.
  assert_eq!(
    json("builtins.div (-9223372036854775807) (-1)"),
    "9223372036854775807"
  );
  assert_eq!(json("builtins.div 10 3"), "3");
}

#[test]
fn builtins_div_zero_int_int_errors() {
  let m = err_msg("builtins.div 1 0");
  assert!(m.contains("division by zero"), "got: {m}");
}

#[test]
fn builtins_div_zero_float_errors() {
  let m = err_msg("builtins.div 1.0 0.0");
  assert!(m.contains("division by zero"), "got: {m}");
}

// ── float ops still produce +inf without panicking ─────────────

#[test]
fn builtins_mul_float_overflow_produces_inf() {
  // Float overflow is Nix-canonical silent +inf — match real
  // Nix. The toJSON boundary catches +inf with a typed error
  // (slice #35) but the bare arithmetic value is +inf.
  // toString of inf produces "inf".
  assert_eq!(
    json("builtins.toString (builtins.mul 1.0e200 1.0e200)"),
    r#""inf""#
  );
}

#[test]
fn builtins_div_float_zero_errors() {
  // Float division by zero errors (slice #35). pnix is
  // intentionally stricter than real Nix here.
  let m = err_msg("builtins.div 1.0 0.0");
  assert!(m.contains("division by zero"), "got: {m}");
}

// ── operator parity (binary / builtin same shape) ──────────────

#[test]
fn binary_and_builtin_add_overflow_match() {
  // Both surfaces should produce the same error family. The
  // exact message text differs slightly (binary says
  // "integer overflow: X + Y", builtin says the same), but
  // both must error rather than panic.
  let bin = err_msg("9223372036854775807 + 1");
  let bul = err_msg("builtins.add 9223372036854775807 1");
  assert!(bin.contains("integer overflow"), "binary: {bin}");
  assert!(bul.contains("integer overflow"), "builtin: {bul}");
}

#[test]
fn binary_and_builtin_mul_overflow_match() {
  let bin = err_msg("9223372036854775807 * 9223372036854775807");
  let bul = err_msg("builtins.mul 9223372036854775807 9223372036854775807");
  assert!(bin.contains("integer overflow"), "binary: {bin}");
  assert!(bul.contains("integer overflow"), "builtin: {bul}");
}

// ── float overflow / NaN — observable behaviour pin ────────────

#[test]
fn float_overflow_produces_inf_not_panic() {
  // The eval result is Float(+inf); to_json renders it as null
  // (Value::to_json's behaviour for non-finite floats is
  // unchanged). The important thing is no panic and no error.
  // Use builtins.typeOf to confirm the value exists as a float.
  assert_eq!(json("builtins.typeOf (1.0e200 * 1.0e200)"), r#""float""#);
}

#[test]
fn float_inf_minus_inf_produces_nan() {
  // inf - inf = NaN. pnix produces a Float(NaN) silently in
  // arithmetic; the toJSON boundary catches this with the
  // slice #35 NaN guard. Match real Nix's silent NaN behaviour
  // in arithmetic.
  let inf = "(1.0e200 * 1.0e200)";
  let src = format!("builtins.typeOf ({inf} - {inf})");
  assert_eq!(json(&src), r#""float""#);
}
