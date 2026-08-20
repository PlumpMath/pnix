//! Regression cover for `+` operator with path/string operands —
//! string-context propagation and silent-metadata-loss guards.
//!
//! 2026-05-05 audit findings (slice #54):
//!
//!   1. `"text" + ./path` produced a plain `Value::String` whose
//!      context was empty. Real Nix adds the path to the result
//!      string's context (matching the slice #49 / #51 family
//!      where every path-aware string op propagates dependencies).
//!      Pre-fix any `.px` author building a derivation reference
//!      via `+` (e.g. `"echo " + ./script` or `"foo://" + ./repo`)
//!      would silently lose the path's role as a build-time
//!      dependency.
//!
//!   2. `./path + "x${./p}"` produced a `Value::Path` whose path
//!      cannot carry string context, so the right-hand string's
//!      context (containing `./p`) was silently dropped. Now
//!      this errors fail-loud with a clear message naming the
//!      `unsafeDiscardStringContext` escape hatch — the metadata
//!      loss must be visible to the author. Plain (no-context)
//!      string on the right still produces a Path (sanity).
//!
//! Truth-owner files:
//! - `crates/pnix-eval/src/interpret.rs` `+` operator path arms
//!   in `binary_op`. The `Path + Path` case is unchanged (no
//!   context). The `Path + String/StringContext` case errors when
//!   the right-hand string carries non-empty context. The
//!   `String/StringContext + Path` case unions left context with
//!   the path's display form (path becomes a build-time
//!   dependency marker).

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

fn err(src: &str) -> String {
  eval_expr(src).err().expect(src).to_string()
}

fn ctx_json(expr: &str) -> String {
  json(&format!("builtins.toJSON (builtins.getContext ({}))", expr))
}

// ── string + path: path becomes context marker ─────────────────

#[test]
fn plain_string_plus_path_adds_path_to_context() {
  // Pre-fix `"text" + ./p` had empty context. Now `./p` is
  // added to the context as a build-time dependency marker.
  let ctx = ctx_json(r#""text" + ./p"#);
  assert!(ctx.contains("./p"), "got: {ctx}");
}

#[test]
fn context_bearing_string_plus_path_unions_contexts() {
  // Left-hand string carries `./a` context; right-hand path
  // adds `./b`. Result string's context should contain both.
  let ctx = ctx_json(r#""x${./a}" + ./b"#);
  assert!(ctx.contains("./a"), "got: {ctx}");
  assert!(ctx.contains("./b"), "got: {ctx}");
}

#[test]
fn string_plus_path_text_concatenates() {
  // Sanity: text portion still concatenates (the value-level
  // contract is preserved; only the context channel was changed).
  let v = json(r#""prefix-" + ./somepath"#);
  assert!(v.contains("prefix-"), "got: {v}");
  assert!(v.contains("./somepath"), "got: {v}");
}

// ── path + plain string: still produces Path ───────────────────

#[test]
fn path_plus_plain_string_still_produces_path() {
  // Sanity: path + plain (no-context) string is unchanged —
  // result is `Value::Path("./foo/bar")`.
  let v = json(r#"./foo + "/bar""#);
  // Path JSON encoding carries the path text.
  assert!(v.contains("./foo/bar"), "got: {v}");
}

// ── path + context-bearing string: fail-loud ───────────────────

#[test]
fn path_plus_context_bearing_string_errors_loud() {
  // Pre-fix silently produced `Path("./foo<resolved-./p>")`
  // and lost the `./p` context. Now errors with a message
  // naming `unsafeDiscardStringContext` as the escape hatch.
  let e = err(r#"./foo + "x${./p}""#);
  assert!(e.contains("operator +"), "got: {e}");
  assert!(e.contains("path"), "got: {e}");
  assert!(e.contains("context"), "got: {e}");
  // Mentions the escape hatch so authors can fix intentionally.
  assert!(e.contains("unsafeDiscardStringContext"), "got: {e}");
}

#[test]
fn path_plus_discarded_context_string_works() {
  // Escape hatch path: explicitly strip context first → path
  // result is allowed (no metadata loss because the author
  // explicitly chose to discard).
  let v = json(r#"./foo + (builtins.unsafeDiscardStringContext "x${./p}")"#);
  assert!(v.contains("./foo"), "got: {v}");
}

// ── path + path: unchanged ─────────────────────────────────────

#[test]
fn path_plus_path_unchanged() {
  let v = json(r#"./foo + /bar"#);
  assert!(v.contains("./foo/bar"), "got: {v}");
}

// ── string + path: parity with ${} interpolation ───────────────

#[test]
fn plus_path_matches_interpolation_context() {
  // `"x" + ./p` should produce the same result-context as
  // `"x${./p}"` — both are derivation-reference shapes.
  let plus_ctx = ctx_json(r#""x" + ./p"#);
  let interp_ctx = ctx_json(r#""x${./p}""#);
  // Both contain `./p` (text differs because interpolation
  // resolves the store path, but the context channel is the
  // same shape).
  assert!(plus_ctx.contains("./p"), "plus: {plus_ctx}");
  assert!(interp_ctx.contains("./p"), "interp: {interp_ctx}");
}

// ── existing operator + behaviours still work ──────────────────

#[test]
fn string_plus_string_still_works() {
  assert_eq!(json(r#""a" + "b""#), r#""ab""#);
}

#[test]
fn list_plus_list_still_works() {
  assert_eq!(json(r#"[ 1 2 ] + [ 3 ]"#), r#"[1,2,3]"#);
}

#[test]
fn attrset_plus_attrset_still_works() {
  let v = json(r#"{ a = 1; } + { b = 2; }"#);
  assert!(v.contains("\"a\":1"), "got: {v}");
  assert!(v.contains("\"b\":2"), "got: {v}");
}

#[test]
fn int_plus_int_still_works() {
  assert_eq!(json(r#"1 + 2"#), r#"3"#);
}

// ── Negative parity sanity: error shapes unchanged ─────────────

#[test]
fn int_plus_string_still_errors() {
  let e = err(r#"42 + "hi""#);
  assert!(e.contains("unsupported operand types"), "got: {e}");
}

#[test]
fn null_plus_int_still_errors() {
  let e = err(r#"null + 1"#);
  assert!(e.contains("unsupported operand types"), "got: {e}");
}
