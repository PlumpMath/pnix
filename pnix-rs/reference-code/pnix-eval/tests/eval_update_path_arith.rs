//! Regression cover for the `//` update operator, nested-path
//! attrset merge semantics, and arithmetic edge cases (overflow,
//! infinity).
//!
//! 2026-05-04 audit findings:
//!   - one real bug: `builtins.toJSON (1.0e308 * 10.0)` silently
//!     returned `"null"` (because `serde_json::Number::from_f64`
//!     rejects ±inf / NaN). Real Nix and the production fail-loud
//!     contract: error
//!     `builtins.toJSON: cannot serialize float +inf as JSON`.
//!     Fixed by adding `check_json_finite` walk before
//!     serialization. New `builtins.isFinite` / `isInf` / `isNaN`
//!     give `.px` authors the explicit guard.
//!   - the other fourteen probe cases (shallow `//`, nested-path
//!     merge, integer-overflow `checked_*`) already match Nix and
//!     are pinned alongside as regression baselines.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

fn err_msg(src: &str) -> String {
  format!("{}", eval_expr(src).expect_err(src))
}

// ── // (update operator) is *shallow* ──────────────────────────────

#[test]
fn update_shallow_replaces_top_level_attr_value() {
  // Real Nix-correct: `//` replaces the value of `a` *entirely* —
  // no recursive merge into the inner attrset. So `{a={x=1;};} //
  // {a={y=2;};}` evaluates to `{a={y=2;};}`, not `{a={x=1;y=2;};}`.
  assert_eq!(
    json(r#"{ a = { x = 1; }; } // { a = { y = 2; }; }"#),
    r#"{"a":{"y":2}}"#
  );
}

#[test]
fn update_adds_new_keys() {
  assert_eq!(json(r#"{ a = 1; } // { b = 2; }"#), r#"{"a":1,"b":2}"#);
}

#[test]
fn update_overrides_int_with_attrset() {
  // Type-blind at top level — int can be replaced by attrset.
  assert_eq!(
    json(r#"{ a = 1; } // { a = { x = 2; }; }"#),
    r#"{"a":{"x":2}}"#
  );
}

#[test]
fn update_overrides_attrset_with_int() {
  assert_eq!(json(r#"{ a = { x = 1; }; } // { a = 2; }"#), r#"{"a":2}"#);
}

#[test]
fn update_with_empty_set_is_identity() {
  assert_eq!(json(r#"{ a = 1; b = 2; } // { }"#), r#"{"a":1,"b":2}"#);
  assert_eq!(json(r#"{ } // { a = 1; }"#), r#"{"a":1}"#);
}

// ── nested-path attrset merge ──────────────────────────────────────

#[test]
fn nested_path_merges_same_subtree() {
  // `{ a.b = 1; a.c = 2; }` → `{ a = { b = 1; c = 2; }; }`
  assert_eq!(json(r#"{ a.b = 1; a.c = 2; }.a"#), r#"{"b":1,"c":2}"#);
}

#[test]
fn explicit_assign_then_path_merges() {
  // `{ a = { b = 1; }; a.c = 2; }` — explicit attrset on a, then
  // nested-path adds c. The two should merge.
  assert_eq!(
    json(r#"{ a = { b = 1; }; a.c = 2; }.a"#),
    r#"{"b":1,"c":2}"#
  );
}

#[test]
fn two_explicit_attrset_assigns_merge() {
  // `{ a = { b = 1; }; a = { c = 2; }; }` — both fully assign
  // `a` with attrsets; pnix merges them (Nix-compat). Distinct
  // from `{ a = 1; a = 2; }` where the leaf collision errors.
  assert_eq!(
    json(r#"{ a = { b = 1; }; a = { c = 2; }; }"#),
    r#"{"a":{"b":1,"c":2}}"#
  );
}

#[test]
fn duplicate_leaf_path_errors() {
  // `{ a.b = 1; a.b = 2; }` — same leaf path twice → duplicate error.
  let m = err_msg(r#"{ a.b = 1; a.b = 2; }"#);
  assert!(m.contains("already defined"), "got: {m}");
}

// ── arithmetic overflow (checked_*; fail-loud) ─────────────────────

#[test]
fn integer_addition_overflow_errors() {
  let m = err_msg("9223372036854775807 + 1");
  assert!(m.contains("integer overflow"), "got: {m}");
}

#[test]
fn integer_multiplication_overflow_errors() {
  let m = err_msg("10000000000 * 10000000000");
  assert!(m.contains("integer overflow"), "got: {m}");
}

#[test]
fn int_times_float_promotes_to_float() {
  // Mixed-type arithmetic promotes to float — large int values
  // round to the nearest representable f64.
  let v = json("9223372036854775807 * 2.0");
  // 9.22…e18 * 2 ≈ 1.84…e19 (sign is positive)
  assert!(v.contains("e+19") || v.contains("e19"), "got: {v}");
}

// ── float infinity is fail-loud through toJSON ─────────────────────

#[test]
fn float_arithmetic_can_overflow_to_inf() {
  // Plain arithmetic does *not* error on overflow to infinity —
  // it produces ±inf as IEEE 754 demands. Authors who care must
  // guard with `isFinite` (next test).
  let v = eval_expr("1.0e308 * 10.0").unwrap();
  use pnix_eval::Value;
  assert!(matches!(v, Value::Float(f) if f.is_infinite()));
}

#[test]
fn to_json_on_infinity_errors() {
  // The previously-silent bug: `toJSON` returned `"null"` for
  // ±inf / NaN. Now errors with the float kind in the message.
  let m = err_msg("builtins.toJSON (1.0e308 * 10.0)");
  assert!(
    m.contains("cannot serialize float +inf as JSON"),
    "got: {m}"
  );
}

#[test]
fn to_json_on_infinity_inside_attrset_errors() {
  let m = err_msg("builtins.toJSON { x = 1.0e308 * 10.0; }");
  assert!(m.contains("cannot serialize"), "got: {m}");
}

#[test]
fn to_json_on_finite_floats_works() {
  assert_eq!(json("builtins.toJSON 3.5"), r#""3.5""#);
  // Either `"0"` or `"0.0"` is a valid JSON number for 0.0; pin
  // pnix's choice (serde_json's default) so codegen can rely on
  // it. Update this expectation alongside any deliberate change
  // to the number-formatting backend.
  assert_eq!(json("builtins.toJSON 0.0"), r#""0.0""#);
}

// ── isFinite / isInf / isNaN guard surface ─────────────────────────

#[test]
fn is_finite_int() {
  assert_eq!(json("builtins.isFinite 42"), "true");
  assert_eq!(json("builtins.isFinite (-7)"), "true");
}

#[test]
fn is_finite_normal_float() {
  assert_eq!(json("builtins.isFinite 3.5"), "true");
  assert_eq!(json("builtins.isFinite 0.0"), "true");
}

#[test]
fn is_finite_inf_returns_false() {
  assert_eq!(json("builtins.isFinite (1.0e308 * 10.0)"), "false");
}

#[test]
fn is_inf_only_floats() {
  assert_eq!(json("builtins.isInf (1.0e308 * 10.0)"), "true");
  assert_eq!(json("builtins.isInf 3.5"), "false");
  // Ints can't be infinity in pnix — `isInf` returns false even
  // for huge ints.
  assert_eq!(json("builtins.isInf 42"), "false");
}
