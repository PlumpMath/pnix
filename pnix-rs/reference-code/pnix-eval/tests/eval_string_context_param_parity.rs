//! Regression cover for `getEnv` / `xmlParse` / `htmlParse`
//! string-arg parity — extends the slice #50 / #64 family of
//! "context-bearing strings ARE strings" parity fixes.
//!
//! 2026-05-05 audit findings (slice #71):
//!
//!   Three more builtins had the same parity miss as slice
//!   #50 (`isString`) and slice #64 (`resolve_value_path`) —
//!   they only matched `Value::String(s)` for their string
//!   argument, falling through to `_ => Err("expected
//!   string")` for `Value::StringContext`. So:
//!     - `builtins.getEnv "PATH${./marker}"` errored
//!     - `builtins.xmlParse "<a>${./content}</a>"` errored
//!     - `builtins.htmlParse "<p>${./body}</p>"` errored
//!
//!   All three were silently rejecting context-bearing strings
//!   despite both shapes being strings. Same fix shape: use
//!   `as_str()` (which handles both variants).
//!
//!   Production-relevant: `.px` authors who construct strings
//!   via `+ Path` (slice #54 produces context-bearing strings)
//!   couldn't pass the result to these builtins. Workaround
//!   was `unsafeDiscardStringContext` — but that's the wrong
//!   tool, since the builtin SHOULD accept context-bearing
//!   strings (it just reads the text and the operation is
//!   text-only).
//!
//! Truth-owner files:
//! - `crates/pnix-eval/src/interpret.rs` `apply_builtin`
//!   `"getEnv"` arm — now uses `args[0].as_str()` (was:
//!   narrow `Value::String(s)` match).
//! - `crates/pnix-eval/src/interpret.rs` `apply_builtin`
//!   `"xmlParse"` arm — same `as_str()` pattern.
//! - `crates/pnix-eval/src/interpret.rs` `apply_builtin`
//!   `"htmlParse"` arm — same.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

fn err(src: &str) -> String {
  eval_expr(src).err().expect(src).to_string()
}

// ── getEnv: context-bearing string accepted ───────────────────

#[test]
fn get_env_plain_string_unchanged() {
  // Sanity: plain string still works (returns "" for unset
  // var; pnix's getenv_allowed gate may also apply).
  let v = json(r#"builtins.getEnv "DEFINITELY_NOT_SET_XYZ_SLICE71""#);
  assert_eq!(v, r#""""#);
}

#[test]
fn get_env_context_bearing_string_no_type_error() {
  // Pre-fix: errored "expected string". Now: type check
  // passes, returns "" (var is unset and even if set, the
  // env-lookup with the resolved path text would not match
  // any real env var).
  let v = json(r#"builtins.getEnv "DEFINITELY_NOT_SET_${./marker}""#);
  // Just verify the call didn't error with a type message.
  assert_eq!(v, r#""""#);
}

#[test]
fn get_env_int_arg_still_errors() {
  let e = err(r#"builtins.getEnv 42"#);
  assert!(e.contains("getEnv"), "got: {e}");
  assert!(e.contains("expected string"), "got: {e}");
  assert!(e.contains("int"), "got: {e}");
}

#[test]
fn get_env_null_arg_still_errors() {
  let e = err(r#"builtins.getEnv null"#);
  assert!(e.contains("getEnv"), "got: {e}");
  assert!(e.contains("expected string"), "got: {e}");
  assert!(e.contains("null"), "got: {e}");
}

// ── xmlParse: context-bearing string accepted ─────────────────

#[test]
fn xml_parse_plain_string_unchanged() {
  // Sanity.
  let v = json(r#"builtins.xmlParse "<a/>""#);
  // Just check it returns something (parsed XML structure).
  assert!(!v.is_empty(), "got: {v}");
}

#[test]
fn xml_parse_context_bearing_string_no_type_error() {
  // The XML text constructed via `+ Path` carries context.
  // Pre-fix errored at type check. Now: parse succeeds
  // (whatever the actual parse outcome).
  let v = json(r#"builtins.xmlParse "<a>${./marker}</a>""#);
  // Result is a parsed XML structure (attrset/list). Just
  // verify the call succeeded without a type error.
  assert!(!v.is_empty(), "got: {v}");
}

#[test]
fn xml_parse_int_arg_still_errors() {
  let e = err(r#"builtins.xmlParse 42"#);
  assert!(e.contains("xmlParse"), "got: {e}");
  assert!(e.contains("expected string"), "got: {e}");
  assert!(e.contains("int"), "got: {e}");
}

// ── htmlParse: context-bearing string accepted ────────────────

#[test]
fn html_parse_plain_string_unchanged() {
  let v = json(r#"builtins.htmlParse "<p>hi</p>""#);
  assert!(!v.is_empty(), "got: {v}");
}

#[test]
fn html_parse_context_bearing_string_no_type_error() {
  let v = json(r#"builtins.htmlParse "<p>${./body}</p>""#);
  assert!(!v.is_empty(), "got: {v}");
}

#[test]
fn html_parse_null_arg_still_errors() {
  let e = err(r#"builtins.htmlParse null"#);
  assert!(e.contains("htmlParse"), "got: {e}");
  assert!(e.contains("expected string"), "got: {e}");
  assert!(e.contains("null"), "got: {e}");
}

// ── parity check with isString family (slice #50) ─────────────

#[test]
fn is_string_context_bearing_returns_true() {
  // Sanity: slice #50 — context-bearing string IS a string.
  assert_eq!(json(r#"builtins.isString "x${./p}""#), "true");
}

#[test]
fn type_of_context_bearing_returns_string() {
  // Sanity: typeOf knows context-bearing is "string".
  assert_eq!(json(r#"builtins.typeOf "x${./p}""#), r#""string""#);
}
