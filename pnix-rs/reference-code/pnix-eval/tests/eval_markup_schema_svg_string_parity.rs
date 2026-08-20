//! Regression cover for `mathmlXmlToJson` / `openmathXmlToJson`
//! / `schemaValidate` / `svgEmit` etc. string-arg parity.
//! Extends the slice #50 / #64 / #71 family across the
//! markup / schema / svg / math support modules.
//!
//! 2026-05-05 audit findings (slice #72):
//!
//!   Several support-module functions had the same parity
//!   miss as slice #50 (`isString`) and slice #64
//!   (`resolve_value_path`). Each was narrowly matching
//!   `Value::String(...)` only, silently rejecting
//!   context-bearing strings:
//!     - `mathmlXmlToJson "<math>${./marker}</math>"` errored
//!     - `openmathXmlToJson "<OMOBJ>${./...}</OMOBJ>"` errored
//!     - `schemaValidate { kind = "string"; } "x${./p}"`
//!       reported a false type-mismatch
//!     - `svgEmit` / `svgSchemaValidate` etc. via
//!       `svg_input_to_json` had the same gap
//!     - `xml_value_to_string` (internal XML emit helper)
//!       errored on context-bearing string values
//!
//!   All fixes use `as_str()` (or add an explicit
//!   `Value::StringContext` arm) — the canonical idiom.
//!
//! Truth-owner files:
//! - `crates/pnix-eval/src/math_markup.rs`
//!   `mathml_xml_to_json` / `openmath_xml_to_json` — narrow
//!   `Value::String(expr)` match replaced with `as_str()`.
//! - `crates/pnix-eval/src/markup.rs` `xml_value_to_string` —
//!   `Value::StringContext` arm added next to `Value::String`.
//! - `crates/pnix-eval/src/schema.rs` `validate` `"string"`
//!   case — accepts both String and StringContext.
//! - `crates/pnix-eval/src/schema.rs` `value_string_len` —
//!   StringContext arm added.
//! - `crates/pnix-eval/src/svg.rs` `svg_input_to_json` —
//!   uses `as_str()`.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

fn err(src: &str) -> String {
  eval_expr(src).err().expect(src).to_string()
}

// ── mathmlXmlToJson with context-bearing string ───────────────

#[test]
fn mathml_xml_to_json_context_bearing_no_type_error() {
  // The XML text constructed via interpolation carries
  // context. Parse succeeds — the type check should NOT
  // fire on the context-bearing string.
  let v = json(r#"builtins.mathmlXmlToJson "<math>${./m}</math>""#);
  // Just verify the call returned something (the resolved
  // path is treated as text content of the math element).
  assert!(!v.is_empty(), "got: {v}");
}

#[test]
fn mathml_xml_to_json_plain_string_unchanged() {
  // Sanity: plain MathML XML still parses.
  let v = json(r#"builtins.mathmlXmlToJson "<math><mn>1</mn></math>""#);
  assert!(!v.is_empty(), "got: {v}");
}

#[test]
fn mathml_xml_to_json_int_arg_still_errors() {
  let e = err(r#"builtins.mathmlXmlToJson 42"#);
  assert!(e.contains("mathmlXmlToJson"), "got: {e}");
}

// ── openmathXmlToJson with context-bearing string ─────────────

#[test]
fn openmath_xml_to_json_context_bearing_no_type_error() {
  let v = json(r#"builtins.openmathXmlToJson "<OMOBJ>${./m}</OMOBJ>""#);
  assert!(!v.is_empty(), "got: {v}");
}

#[test]
fn openmath_xml_to_json_int_arg_still_errors() {
  let e = err(r#"builtins.openmathXmlToJson 42"#);
  assert!(e.contains("openmathXmlToJson"), "got: {e}");
}

// ── schemaValidate accepts context-bearing string value ──────

#[test]
fn schema_validate_string_context_bearing_value() {
  // schema kind "string" should accept context-bearing
  // string values (slice #50: context-bearing IS a string).
  let v = json(
    r#"
    (builtins.schemaValidate { kind = "string"; } "x${./p}").ok
  "#,
  );
  assert_eq!(v, "true", "expected validation OK, got: {v}");
}

#[test]
fn schema_validate_string_kind_int_value_still_fails() {
  // Sanity: int value against string schema still errors.
  let v = json(
    r#"
    (builtins.schemaValidate { kind = "string"; } 42).ok
  "#,
  );
  assert_eq!(v, "false");
}

#[test]
fn schema_validate_with_min_length_and_context_bearing() {
  // String schema with minLength check on context-bearing
  // value. Pre-fix would fail at type check before reaching
  // the constraint validation.
  let v = json(
    r#"
    (builtins.schemaValidate
      { kind = "string"; minLength = 1; }
      "ok${./m}").ok
  "#,
  );
  assert_eq!(v, "true", "got: {v}");
}

// ── parity check with isString (slice #50) ────────────────────

#[test]
fn is_string_parity_unchanged() {
  // Sanity: slice #50 still in effect.
  assert_eq!(json(r#"builtins.isString "x${./p}""#), "true");
}

// ── pre-existing happy paths still work ──────────────────────

#[test]
fn schema_validate_basic_string_unchanged() {
  let v = json(
    r#"
    (builtins.schemaValidate { kind = "string"; } "hello").ok
  "#,
  );
  assert_eq!(v, "true");
}

#[test]
fn mathml_xml_to_json_attrset_unchanged() {
  // Attrset input path unchanged (XML JSON attrset shape).
  // Just verify the attrset branch still triggers the
  // expected path.
  let e = err(r#"builtins.mathmlXmlToJson { wrong = true; }"#);
  // Errors at attrset-shape check.
  assert!(e.contains("mathmlXmlToJson"), "got: {e}");
}
