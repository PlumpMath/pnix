//! Regression cover for three more numeric panic / silent-pass
//! shapes:
//!   - `builtins.genList` count overflow → Rust panic (capacity
//!     overflow inside `Vec::with_capacity`)
//!   - `builtins.floor` / `ceil` on +inf / -inf / NaN /
//!     out-of-i64-range floats → silent saturating cast (returns
//!     `i64::MAX` for +inf and out-of-range floats, `0` for NaN)
//!
//! 2026-05-05 audit findings (slice #45):
//!   - `builtins.genList (x: x) 9223372036854775807` panicked
//!     in `Vec::with_capacity` with "capacity overflow" — a
//!     Rust process crash. A `.px` author who computed a count
//!     from user input or arithmetic that overflowed would
//!     crash the evaluator. Fix: bound the count at
//!     `GENLIST_MAX_COUNT = 16 * 1024 * 1024` (16 MiB elements
//!     ≈ 256 MiB of pointer storage). Larger requests are user
//!     error, surface as a typed error.
//!   - `builtins.floor` / `ceil` used `value.floor() as i64`
//!     directly. Rust's `f64 as i64` cast:
//!       NaN              → 0
//!       +inf or > i64::MAX → i64::MAX
//!       -inf or < i64::MIN → i64::MIN
//!     So `floor (1.0e200 * 1.0e200)` (= +inf) silently
//!     returned `9223372036854775807`, `floor NaN` silently
//!     returned `0`, and `floor 1.0e200` (a finite but
//!     out-of-range float) silently saturated. Real Nix
//!     errors. Fix: new `float_to_i64_checked(value, context)`
//!     helper that rejects NaN, ±inf, and out-of-range floats
//!     with typed errors before the cast.
//!
//! Truth-owner files:
//! - `crates/pnix-eval/src/interpret.rs` `apply_builtin`
//!   `"genList"` arm — added `GENLIST_MAX_COUNT` bound, typed
//!   error on overflow.
//! - `crates/pnix-eval/src/interpret.rs` `apply_builtin`
//!   `"floor"` / `"ceil"` arms — route through the new
//!   `float_to_i64_checked` helper.
//! - `crates/pnix-eval/src/interpret.rs` `float_to_i64_checked`
//!   — new helper near `expect_i64`. Rejects NaN, ±inf, and
//!   floats outside the i64 representable range. The boundary
//!   uses strict `>= i64::MAX as f64` (because i64::MAX rounds
//!   UP to 9223372036854775808.0 as f64 — values that would
//!   round to that representation are rejected) and `< i64::MIN
//!   as f64` (i64::MIN is exactly representable).

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}
fn err_msg(src: &str) -> String {
  format!("{}", eval_expr(src).expect_err(src))
}

// ── builtins.genList capacity bound ────────────────────────────

#[test]
fn genlist_i64_max_errors_not_panics() {
  // Was: Rust panic "capacity overflow" inside
  // `Vec::with_capacity`.
  let m = err_msg("builtins.genList (x: x) 9223372036854775807");
  assert!(m.contains("genList"), "got: {m}");
  assert!(m.contains("exceeds maximum"), "got: {m}");
  assert!(m.contains("9223372036854775807"), "got: {m}");
}

#[test]
fn genlist_negative_count_still_errors() {
  // Slice-#X earlier invariant.
  let m = err_msg("builtins.genList (x: x) (-1)");
  assert!(m.contains("negative"), "got: {m}");
}

#[test]
fn genlist_zero_works() {
  assert_eq!(json("builtins.genList (x: x) 0"), "[]");
}

#[test]
fn genlist_legitimate_size_works() {
  // 100k elements is comfortably below the 16 MiB cap.
  assert_eq!(
    json("builtins.length (builtins.genList (x: x) 100000)"),
    "100000"
  );
}

// ── floor +inf / -inf / NaN / out-of-range ─────────────────────

#[test]
fn floor_inf_errors() {
  // Was: silently returned 9223372036854775807 (i64::MAX).
  let m = err_msg("builtins.floor (1.0e200 * 1.0e200)");
  assert!(m.contains("builtins.floor"), "got: {m}");
  assert!(m.contains("+inf"), "got: {m}");
}

#[test]
fn floor_neg_inf_errors() {
  let m = err_msg("builtins.floor (-(1.0e200 * 1.0e200))");
  assert!(m.contains("builtins.floor"), "got: {m}");
  assert!(m.contains("-inf"), "got: {m}");
}

#[test]
fn floor_nan_errors() {
  // Was: silently returned 0 (NaN as i64 = 0 in Rust).
  let inf = "(1.0e200 * 1.0e200)";
  let nan = format!("({inf} - {inf})");
  let m = err_msg(&format!("builtins.floor {nan}"));
  assert!(m.contains("builtins.floor"), "got: {m}");
  assert!(m.contains("NaN"), "got: {m}");
}

#[test]
fn floor_out_of_range_errors() {
  // Finite but bigger than i64::MAX.
  let m = err_msg("builtins.floor 1.0e200");
  assert!(m.contains("builtins.floor"), "got: {m}");
  assert!(m.contains("outside i64 range"), "got: {m}");
}

#[test]
fn floor_legitimate_works() {
  // Sanity: normal floor still works.
  assert_eq!(json("builtins.floor 3.7"), "3");
  assert_eq!(json("builtins.floor (-3.7)"), "-4");
  assert_eq!(json("builtins.floor 0.0"), "0");
  assert_eq!(json("builtins.floor 1.0e10"), "10000000000");
}

// ── ceil +inf / -inf / NaN / out-of-range ──────────────────────

#[test]
fn ceil_inf_errors() {
  let m = err_msg("builtins.ceil (1.0e200 * 1.0e200)");
  assert!(m.contains("builtins.ceil"), "got: {m}");
  assert!(m.contains("+inf"), "got: {m}");
}

#[test]
fn ceil_nan_errors() {
  let inf = "(1.0e200 * 1.0e200)";
  let nan = format!("({inf} - {inf})");
  let m = err_msg(&format!("builtins.ceil {nan}"));
  assert!(m.contains("builtins.ceil"), "got: {m}");
  assert!(m.contains("NaN"), "got: {m}");
}

#[test]
fn ceil_out_of_range_errors() {
  let m = err_msg("builtins.ceil 1.0e200");
  assert!(m.contains("outside i64 range"), "got: {m}");
}

#[test]
fn ceil_legitimate_works() {
  assert_eq!(json("builtins.ceil 3.2"), "4");
  assert_eq!(json("builtins.ceil (-3.2)"), "-3");
  assert_eq!(json("builtins.ceil 0.0"), "0");
}

// ── exact i64::MAX boundary ────────────────────────────────────

#[test]
fn floor_at_i64_max_boundary_errors() {
  // i64::MAX as f64 rounds UP to 9223372036854775808.0. A
  // float that close to or above i64::MAX can't round-trip as
  // a precise integer. We reject the >= i64::MAX boundary
  // explicitly.
  let m = err_msg("builtins.floor 9.223372036854776e18");
  assert!(m.contains("outside i64 range"), "got: {m}");
}

#[test]
fn floor_just_below_i64_max_works() {
  // 1.0e18 is well within i64 range (i64::MAX ≈ 9.22e18).
  assert_eq!(json("builtins.floor 1.0e18"), "1000000000000000000");
}
