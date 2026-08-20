//! Regression cover for `builtins.toJSON` / `builtins.fromJSON`.
//!
//! These names were registered as `BuiltinPartial` aliases in
//! `builtins_attrset()` but had no matching `apply_builtin` arm,
//! so calling `builtins.toJSON 42` silently returned the
//! unfinished `BuiltinPartial { name: "toJSON", args: [Int(42)] }`
//! instead of the JSON string. The arms now route through
//! `Value::to_json()` (cycle-safe) and `markup::json_to_value`.

use pnix_eval::{eval_expr, Value};

fn json_str(src: &str) -> String {
  match eval_expr(src).expect(src) {
    Value::String(s) => s,
    other => panic!("expected String, got {other:?} for {src}"),
  }
}

// 2026-05-06: helpers for slice #78 (fromJSON int overflow) merge —
// `json` returns the canonical JSON projection of any Value (cycle-
// safe); `err` returns the error string. These mirror the
// `eval_zipattrswith_lazy_guard.rs` / `eval_resolve_value_path_*` /
// most other audit-slice tests' helper convention so merged blocks
// can be moved between owners verbatim.
fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

fn err(src: &str) -> String {
  eval_expr(src).err().expect(src).to_string()
}

#[test]
fn to_json_int() {
  assert_eq!(json_str("builtins.toJSON 42"), "42");
}

#[test]
fn to_json_float() {
  assert_eq!(json_str("builtins.toJSON 3.5"), "3.5");
}

#[test]
fn to_json_bool() {
  assert_eq!(json_str("builtins.toJSON true"), "true");
  assert_eq!(json_str("builtins.toJSON false"), "false");
}

#[test]
fn to_json_null() {
  assert_eq!(json_str("builtins.toJSON null"), "null");
}

#[test]
fn to_json_string() {
  assert_eq!(json_str(r#"builtins.toJSON "hello""#), "\"hello\"");
}

#[test]
fn to_json_list() {
  assert_eq!(json_str("builtins.toJSON [ 1 2 3 ]"), "[1,2,3]");
}

#[test]
fn to_json_attrset() {
  // BTreeMap → keys are sorted, so output is deterministic.
  assert_eq!(
    json_str(r#"builtins.toJSON { a = 1; b = "x"; }"#),
    "{\"a\":1,\"b\":\"x\"}"
  );
}

#[test]
fn from_json_int() {
  // round-trip
  let v = eval_expr(r#"builtins.fromJSON "42""#).unwrap();
  assert!(matches!(v, Value::Int(42)));
}

#[test]
fn from_json_attrset() {
  let v = eval_expr(r#"builtins.fromJSON "{\"a\":1,\"b\":\"x\"}""#).unwrap();
  match v {
    Value::AttrSet(m) => {
      assert!(matches!(m.get("a"), Some(Value::Int(1))));
      assert!(matches!(m.get("b"), Some(Value::String(s)) if s == "x"));
    }
    other => panic!("expected AttrSet, got {other:?}"),
  }
}

#[test]
fn from_json_list() {
  let v = eval_expr(r#"builtins.fromJSON "[1,2,3]""#).unwrap();
  match v {
    Value::List(items) => {
      assert_eq!(items.len(), 3);
      assert!(matches!(items[0], Value::Int(1)));
      assert!(matches!(items[2], Value::Int(3)));
    }
    other => panic!("expected List, got {other:?}"),
  }
}

#[test]
fn from_json_invalid_errors() {
  let err = eval_expr(r#"builtins.fromJSON "{ not json""#).expect_err("expected parse error");
  assert!(
    err.to_string().contains("parse error"),
    "expected parse-error message, got: {err}"
  );
}

#[test]
fn round_trip_attrset() {
  // toJSON then fromJSON should reproduce the same value (ignoring
  // type-tags pnix doesn't expose in JSON).
  let v =
    eval_expr(r#"builtins.fromJSON (builtins.toJSON { a = 1; b = [ 2 3 ]; c = "x"; })"#).unwrap();
  match v {
    Value::AttrSet(m) => {
      assert_eq!(m.len(), 3);
      assert!(matches!(m.get("a"), Some(Value::Int(1))));
      assert!(matches!(m.get("b"), Some(Value::List(_))));
      assert!(matches!(m.get("c"), Some(Value::String(s)) if s == "x"));
    }
    other => panic!("expected AttrSet, got {other:?}"),
  }
}

// ═══════════════════════════════════════════════════════════════
// slice #78 — `fromJSON` integer overflow detection. Closes
// silent precision-loss shape: serde_json widens any int
// literal that does not fit i64 to f64 (lost ~13 digits).
// Real Nix errors on integer overflow during JSON parse.
// `markup::check_json_no_int_overflow` walks the source
// byte-by-byte, identifies integer-shaped numeric tokens
// (`-?\d+` not followed by `.`/`e`/`E`), and rejects any
// that don't fit i64.
//
// 2026-05-06: merged from `eval_fromjson_int_overflow.rs`.
// `eval_json_builtins.rs` is the canonical fromJSON owner
// (per its header).
// ═══════════════════════════════════════════════════════════════

// ── Overflow → error (the fix) ────────────────────────────────

#[test]
fn from_json_huge_positive_int_errors() {
  // Pre-fix: silently became Float(1e+21).
  let e = err(r#"builtins.fromJSON "999999999999999999999""#);
  assert!(
    e.contains("fromJSON") && e.contains("too large"),
    "got: {e}"
  );
  assert!(e.contains("999999999999999999999"), "got: {e}");
}

#[test]
fn from_json_huge_negative_int_errors() {
  let e = err(r#"builtins.fromJSON "-999999999999999999999""#);
  assert!(
    e.contains("fromJSON") && e.contains("too large"),
    "got: {e}"
  );
  assert!(e.contains("-999999999999999999999"), "got: {e}");
}

#[test]
fn from_json_i64_max_plus_one_errors() {
  // i64::MAX is 9223372036854775807; +1 is 9223372036854775808.
  let e = err(r#"builtins.fromJSON "9223372036854775808""#);
  assert!(e.contains("too large"), "got: {e}");
}

#[test]
fn from_json_i64_min_minus_one_errors() {
  // i64::MIN is -9223372036854775808; -1 is -9223372036854775809.
  let e = err(r#"builtins.fromJSON "-9223372036854775809""#);
  assert!(e.contains("too large"), "got: {e}");
}

// ── i64 boundaries still work (not over-aggressive) ───────────

#[test]
fn from_json_i64_max_works() {
  assert_eq!(
    json(r#"builtins.fromJSON "9223372036854775807""#),
    "9223372036854775807"
  );
}

#[test]
fn from_json_i64_min_works() {
  assert_eq!(
    json(r#"builtins.fromJSON "-9223372036854775808""#),
    "-9223372036854775808"
  );
}

#[test]
fn from_json_typeof_max_int_is_int() {
  assert_eq!(
    json(r#"builtins.typeOf (builtins.fromJSON "9223372036854775807")"#),
    r#""int""#
  );
}

// ── Floats still work (not over-aggressive on float-shaped) ──

#[test]
fn from_json_regular_float_works() {
  assert_eq!(json(r#"builtins.fromJSON "1.5""#), "1.5");
}

#[test]
fn from_json_negative_float_works() {
  assert_eq!(json(r#"builtins.fromJSON "-1.5""#), "-1.5");
}

#[test]
fn from_json_huge_float_works() {
  // Float-shaped (has `e`), so the integer-overflow check
  // doesn't apply. serde_json handles f64 normally.
  assert_eq!(json(r#"builtins.fromJSON "1e308""#), "1e+308");
}

#[test]
fn from_json_scientific_int_mantissa_treated_as_float() {
  // "1e3" has `e` → float-shaped per JSON spec, even though
  // the mantissa is an integer. Real Nix returns 1000.0.
  assert_eq!(json(r#"builtins.fromJSON "1e3""#), "1000.0");
}

#[test]
fn from_json_typeof_float_is_float() {
  assert_eq!(
    json(r#"builtins.typeOf (builtins.fromJSON "1.5")"#),
    r#""float""#
  );
}

// ── Embedded contexts (containers, strings) ───────────────────

#[test]
fn from_json_array_with_overflow_errors() {
  // Overflow inside an array is detected.
  let e = err(r#"builtins.fromJSON "[1, 999999999999999999999]""#);
  assert!(e.contains("too large"), "got: {e}");
}

#[test]
fn from_json_object_with_overflow_errors() {
  let e = err(r#"builtins.fromJSON "{\"x\": 999999999999999999999}""#);
  assert!(e.contains("too large"), "got: {e}");
}

#[test]
fn from_json_nested_overflow_errors() {
  let e = err(r#"builtins.fromJSON "{\"a\": [{\"b\": 999999999999999999999}]}""#);
  assert!(e.contains("too large"), "got: {e}");
}

#[test]
fn from_json_string_containing_big_number_text_is_safe() {
  // A string whose CONTENT is a giant number must NOT trigger
  // the integer-overflow check — strings are skipped.
  assert_eq!(
    json(r#"builtins.fromJSON "\"999999999999999999999\"""#),
    r#""999999999999999999999""#
  );
}

#[test]
fn from_json_string_with_escape_quote_safe() {
  // String with embedded escaped quote followed by overflow-
  // shaped digits — the scanner must handle escape sequences
  // correctly so it doesn't end the string early.
  assert_eq!(
    json(r#"builtins.fromJSON "\"a\\\"b 999999999999999999999\"""#),
    r#""a\"b 999999999999999999999""#
  );
}

#[test]
fn from_json_array_of_floats_unchanged() {
  // Float-only array passes through cleanly.
  assert_eq!(
    json(r#"builtins.fromJSON "[1.5, 2.5, 3.5]""#),
    "[1.5,2.5,3.5]"
  );
}

#[test]
fn from_json_mixed_int_float_unchanged() {
  // Mixed integer (within i64 range) and float values.
  assert_eq!(
    json(r#"builtins.fromJSON "[1, 2.5, 100, -50]""#),
    "[1,2.5,100,-50]"
  );
}

// ── Sanity: existing behaviour unchanged ──────────────────────

#[test]
fn from_json_zero_works() {
  assert_eq!(json(r#"builtins.fromJSON "0""#), "0");
}

#[test]
fn from_json_negative_one_works() {
  assert_eq!(json(r#"builtins.fromJSON "-1""#), "-1");
}

#[test]
fn from_json_invalid_still_errors_with_serde_message() {
  // Order check: invalid JSON triggers serde's parse-error
  // message before the overflow check. The error mentions
  // "parse error" rather than "too large".
  let e = err(r#"builtins.fromJSON "not a number""#);
  assert!(e.contains("parse error"), "got: {e}");
  assert!(!e.contains("too large"), "got: {e}");
}

#[test]
fn from_json_non_string_arg_unchanged() {
  // Non-string arg error path unchanged.
  let e = err(r#"builtins.fromJSON 42"#);
  assert!(e.contains("fromJSON") && e.contains("string"), "got: {e}");
}

#[test]
fn from_json_empty_array_unchanged() {
  assert_eq!(json(r#"builtins.fromJSON "[]""#), "[]");
}

#[test]
fn from_json_empty_object_unchanged() {
  assert_eq!(json(r#"builtins.fromJSON "{}""#), "{}");
}

// ── Note: -0 silent float-widening intentionally NOT fixed ───

#[test]
fn from_json_minus_zero_is_float_documented_deferred() {
  // Documented current behaviour. `-0` in source produces
  // Float(-0.0) because serde_json's parser stores signed-zero
  // as f64 (i64 representation can't carry sign-of-zero).
  // Fixing this needs source-text tracking; deferred to a
  // future slice with a more complete approach.
  //
  // This test asserts CURRENT behaviour, not desired behaviour —
  // when a future slice fixes this, this test should be moved
  // to assert the corrected behaviour (Int(0)).
  assert_eq!(
    json(r#"builtins.typeOf (builtins.fromJSON "-0")"#),
    r#""float""#
  );
}
