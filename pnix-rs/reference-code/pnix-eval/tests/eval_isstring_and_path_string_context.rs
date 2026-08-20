//! Regression cover for `builtins.isString` accepting
//! context-bearing strings, and for `dirOf` / `baseNameOf` /
//! `hashString` preserving string context.
//!
//! 2026-05-05 audit findings (slice #50):
//!   - **CRITICAL: `builtins.isString "x${./p}"` returned
//!     `false`**. The arm matched only `Value::String(_)` and
//!     missed `Value::StringContext { .. }` (the
//!     pnix-internal variant for context-bearing strings).
//!     Real Nix has no such distinction — both ARE strings.
//!     A `.px` author writing `if isString x then ... else
//!     ...` would silently take the wrong branch for any
//!     context-bearing string. This is the worst kind of
//!     audit miss because `isString` is a TYPE PREDICATE —
//!     its job is to answer "is this a string?", and it was
//!     answering "no" for strings that carry context.
//!   - `builtins.dirOf "x${./p}/file"` /
//!     `builtins.baseNameOf "x${./p}/file"` lost the input
//!     string's context. Same shape as slice #49's family
//!     (substring / concatStringsSep / replaceStrings /
//!     toString).
//!   - `builtins.hashString "sha256" "x${./p}"` lost
//!     context. The hash result depends on the path, so the
//!     dependency is real and must be tracked. Real Nix
//!     propagates context through hashString.
//!
//! Truth-owner files:
//! - `crates/pnix-eval/src/interpret.rs` `apply_builtin`
//!   `"isString"` arm — `matches!` extended to accept BOTH
//!   `Value::String(_)` AND `Value::StringContext { .. }`.
//! - `crates/pnix-eval/src/interpret.rs` `"baseNameOf"` /
//!   `"dirOf"` arms — string-input case now uses
//!   `string_with_context` with the input's
//!   `string_context()` to inherit provenance markers. Path
//!   inputs unchanged (paths don't carry string context).
//! - `crates/pnix-eval/src/interpret.rs` `"hashString"` arm
//!   — same propagation pattern; the hex result inherits
//!   the data argument's context.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

fn ctx_json(expr: &str) -> String {
  json(&format!("builtins.toJSON (builtins.getContext ({}))", expr))
}

// ── isString accepts context-bearing strings ───────────────────

#[test]
fn is_string_plain_string() {
  // Sanity: plain string still passes.
  assert_eq!(json(r#"builtins.isString "abc""#), "true");
}

#[test]
fn is_string_context_bearing_string() {
  // Was: false (the worst kind of type-predicate miss).
  assert_eq!(json(r#"builtins.isString "x${./p}""#), "true");
}

#[test]
fn is_string_negative_cases_unchanged() {
  // Sanity: non-strings still fail isString.
  assert_eq!(json("builtins.isString 42"), "false");
  assert_eq!(json("builtins.isString 1.0"), "false");
  assert_eq!(json("builtins.isString null"), "false");
  assert_eq!(json("builtins.isString [ 1 ]"), "false");
  assert_eq!(json(r#"builtins.isString { a = 1; }"#), "false");
  assert_eq!(json("builtins.isString /tmp"), "false");
  assert_eq!(json("builtins.isString (x: x)"), "false");
}

#[test]
fn is_string_after_concat_with_context() {
  // The slice #49 fix made `+` / `concatStringsSep` produce
  // context-bearing strings; this fix lets `isString` see them.
  assert_eq!(json(r#"builtins.isString ("a" + "x${./p}")"#), "true");
  assert_eq!(
    json(r#"builtins.isString (builtins.concatStringsSep "-" [ "a${./p}" "b" ])"#),
    "true"
  );
}

#[test]
fn type_of_already_handled_both_variants() {
  // Pin parity reference: `typeOf` already returned "string"
  // for both variants (line ~2206). The fix brings `isString`
  // in line with `typeOf`.
  assert_eq!(json(r#"builtins.typeOf "abc""#), r#""string""#);
  assert_eq!(json(r#"builtins.typeOf "x${./p}""#), r#""string""#);
}

// ── dirOf / baseNameOf preserve context ────────────────────────

#[test]
fn dir_of_inherits_string_context() {
  let ctx = ctx_json(r#"builtins.dirOf "x${./p}/file""#);
  assert!(ctx.contains("./p"), "got: {ctx}");
}

#[test]
fn base_name_of_inherits_string_context() {
  let ctx = ctx_json(r#"builtins.baseNameOf "x${./p}/file""#);
  assert!(ctx.contains("./p"), "got: {ctx}");
}

#[test]
fn dir_of_no_context_unchanged() {
  let ctx = ctx_json(r#"builtins.dirOf "/foo/bar""#);
  assert_eq!(ctx, r#""{}""#);
}

#[test]
fn base_name_of_no_context_unchanged() {
  let ctx = ctx_json(r#"builtins.baseNameOf "/foo/bar""#);
  assert_eq!(ctx, r#""{}""#);
}

#[test]
fn dir_of_path_input_unchanged() {
  // Path inputs are unchanged — they don't carry string
  // context, and the result is a Path not a String.
  assert_eq!(
    json(r#"builtins.toJSON (builtins.dirOf ./foo/bar)"#),
    r#""\"./foo\"""#
  );
}

#[test]
fn dir_of_value_unchanged() {
  // The text value is unchanged from the pre-fix contract.
  assert_eq!(json(r#"builtins.dirOf "/foo/bar/baz""#), r#""/foo/bar""#);
}

#[test]
fn base_name_of_value_unchanged() {
  assert_eq!(json(r#"builtins.baseNameOf "/foo/bar/baz""#), r#""baz""#);
}

// ── hashString preserves data-arg context ──────────────────────

#[test]
fn hash_string_inherits_data_context() {
  let ctx = ctx_json(r#"builtins.hashString "sha256" "x${./p}""#);
  assert!(ctx.contains("./p"), "got: {ctx}");
}

#[test]
fn hash_string_no_context_unchanged() {
  let ctx = ctx_json(r#"builtins.hashString "sha256" "hello""#);
  assert_eq!(ctx, r#""{}""#);
}

#[test]
fn hash_string_value_unchanged() {
  // Pre-fix value (hex digest) is unchanged.
  assert_eq!(
    json(r#"builtins.stringLength (builtins.hashString "sha256" "hello")"#),
    "64"
  );
}
