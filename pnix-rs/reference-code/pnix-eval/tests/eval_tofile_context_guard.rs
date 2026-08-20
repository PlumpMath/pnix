//! Regression cover for `builtins.toFile` content-context guard.
//!
//! 2026-05-05 audit findings (slice #55):
//!
//!   `builtins.toFile "n" "x${./p}"` silently accepted the
//!   context-bearing content and produced a `Value::Path`
//!   pointing at a deterministic file under
//!   `/tmp/pnix-nix-store/`. The string context (containing
//!   `./p` as a build-time dependency marker) was silently
//!   dropped at this boundary because `Value::Path` cannot
//!   carry context. Real Nix rejects this case because:
//!
//!     - `toFile` is an EVALUATION-time operation. Its output
//!       path's hash is computed from the LITERAL content text.
//!     - If the content contains a derivation reference (via
//!       string context), the derivation must be REALIZED first
//!       to compute the resolved content.
//!     - But realization is a BUILD-time operation, not an
//!       evaluation-time one — so the dependency cannot be
//!       expressed in the toFile result.
//!
//!   Pre-fix any `.px` author writing
//!   `toFile "config" ("DATABASE_URL=" + ./url-derivation)`
//!   would silently lose the path dependency at the toFile
//!   boundary — the resulting file's path would look correct,
//!   but downstream consumers (derivations that reference the
//!   resulting path) would have NO marker that they depended
//!   on `./url-derivation`. The bug surfaces only when the
//!   derivation tree is actually realized — and by then the
//!   evaluation-time silent drop has propagated through
//!   every consumer.
//!
//!   Fix: error fail-loud with a message naming
//!   `unsafeDiscardStringContext` as the explicit escape hatch.
//!   Mirrors slice #54's `path + context-bearing string` guard.
//!
//! Truth-owner files:
//! - `crates/pnix-eval/src/interpret.rs` `apply_builtin`
//!   `"toFile"` arm — checks `args[1].string_context()`
//!   before performing the digest + write. Non-empty context
//!   → error.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

fn err(src: &str) -> String {
  eval_expr(src).err().expect(src).to_string()
}

// ── toFile: fail-loud on context-bearing content ───────────────

#[test]
fn to_file_context_bearing_content_errors_loud() {
  let e = err(r#"builtins.toFile "name" "x${./p}""#);
  assert!(e.contains("toFile"), "got: {e}");
  assert!(e.contains("context"), "got: {e}");
}

#[test]
fn to_file_error_names_escape_hatch() {
  // The error message must point at unsafeDiscardStringContext
  // so authors can opt in to the discard if they really want it.
  let e = err(r#"builtins.toFile "name" "x${./p}""#);
  assert!(e.contains("unsafeDiscardStringContext"), "got: {e}");
}

#[test]
fn to_file_context_via_concat_errors_loud() {
  // The same shape via `+` operator (slice #54 produces
  // context-bearing string from `string + path`).
  let e = err(r#"builtins.toFile "name" ("text-" + ./script)"#);
  assert!(e.contains("toFile"), "got: {e}");
  assert!(e.contains("context"), "got: {e}");
}

// ── toFile: happy path unchanged ───────────────────────────────

#[test]
fn to_file_plain_string_content_works() {
  let v = json(r#"builtins.toFile "name" "literal content""#);
  // Returns a path under the pnix store dir with a hashed name.
  assert!(v.contains("toFile-"), "got: {v}");
  assert!(v.contains("name"), "got: {v}");
}

#[test]
fn to_file_empty_content_works() {
  let v = json(r#"builtins.toFile "empty" """#);
  assert!(v.contains("toFile-"), "got: {v}");
  assert!(v.contains("empty"), "got: {v}");
}

#[test]
fn to_file_deterministic_for_same_input() {
  // Sanity: same name + content produces same path. The hash
  // is content-based.
  let v1 = json(r#"builtins.toFile "n" "abc""#);
  let v2 = json(r#"builtins.toFile "n" "abc""#);
  assert_eq!(v1, v2);
}

#[test]
fn to_file_different_content_different_paths() {
  // Sanity: content change → path change.
  let v1 = json(r#"builtins.toFile "n" "abc""#);
  let v2 = json(r#"builtins.toFile "n" "abd""#);
  assert_ne!(v1, v2);
}

// ── toFile: existing argument-type guards still work ───────────

#[test]
fn to_file_non_string_name_errors() {
  let e = err(r#"builtins.toFile 42 "content""#);
  assert!(e.contains("toFile"), "got: {e}");
  assert!(
    e.contains("first argument") || e.contains("must be string"),
    "got: {e}"
  );
}

#[test]
fn to_file_non_string_content_errors() {
  let e = err(r#"builtins.toFile "n" 42"#);
  assert!(e.contains("toFile"), "got: {e}");
  assert!(
    e.contains("second argument") || e.contains("must be string"),
    "got: {e}"
  );
}

// ── toFile: escape hatch via unsafeDiscardStringContext ────────

#[test]
fn to_file_with_explicit_context_discard_works() {
  // Author opts in to discarding context: explicit call to
  // unsafeDiscardStringContext makes the metadata loss visible
  // at the call site rather than silent inside toFile.
  let v = json(r#"builtins.toFile "n" (builtins.unsafeDiscardStringContext "x${./p}")"#);
  assert!(v.contains("toFile-"), "got: {v}");
  assert!(v.contains("\"n\"") || v.ends_with("-n\""), "got: {v}");
}
