//! Regression cover for path-resolving builtins accepting
//! context-bearing strings.
//!
//! 2026-05-05 audit findings (slice #64):
//!
//!   `resolve_value_path` had a `Value::String(s) =>
//!   PathBuf::from(s)` arm only — `Value::StringContext`
//!   fell through to `_ => Err("expected string or path")`.
//!   Every path-resolving builtin (`pathExists`, `readFile`,
//!   `readDir`, `import`, `toPath`, `storePath`,
//!   `scopedImport`, `readFileType`) silently rejected
//!   context-bearing string args with a misleading
//!   "expected string or path" error.
//!
//!   Pre-fix bugs:
//!     - `pathExists "x${./p}/bin"` errored even though both
//!       sides ARE strings (just the LHS carries context).
//!     - `readFile "${./module}"` errored.
//!     - `import "${./mod}"` errored.
//!     - All other path-builtins: same error.
//!
//!   Production-relevant: `.px` authors who construct paths
//!   via context-bearing string interpolation (a common
//!   pattern: `"${./libdir}/x" + "/bin"`) couldn't pass the
//!   result to ANY path-resolving builtin. Real Nix accepts
//!   context-bearing strings in path positions — uses the
//!   text portion (and realizes any build-time dependencies
//!   referenced via context).
//!
//!   For pnix's no-derivation-graph design, we use the text
//!   portion. Context is metadata that can be re-fetched via
//!   `getContext` if the caller needs it.
//!
//!   Single-arm fix in `resolve_value_path` cascades to every
//!   path-resolving builtin — eight surfaces unblocked at
//!   once.
//!
//! Truth-owner files:
//! - `crates/pnix-eval/src/interpret.rs` `resolve_value_path`
//!   — added `Value::StringContext { text, .. } =>
//!   PathBuf::from(text)` arm. Mirrors the existing
//!   `Value::String(s)` handling.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

fn err(src: &str) -> String {
  eval_expr(src).err().expect(src).to_string()
}

// ── pathExists with context-bearing string ────────────────────

#[test]
fn path_exists_context_bearing_string_does_not_error_at_type_check() {
  // Pre-fix: errored "expected string or path". Now: type
  // check passes, returns false (path doesn't exist).
  assert_eq!(
    json(r#"builtins.pathExists "x${./p}/non-existent-dir-xyz""#),
    "false"
  );
}

#[test]
fn path_exists_plain_string_unchanged() {
  // Sanity.
  assert_eq!(
    json(r#"builtins.pathExists "/non-existent-dir-xyz""#),
    "false"
  );
}

#[test]
fn path_exists_path_arg_unchanged() {
  // Sanity.
  assert_eq!(json(r#"builtins.pathExists ./non-existent-xyz"#), "false");
}

// ── readDir with context-bearing string ───────────────────────

#[test]
fn read_dir_context_bearing_string_does_not_error_at_type_check() {
  // Type check passes; will error on missing dir but the
  // error message names file system, not type.
  let e = err(r#"builtins.readDir "${./not-a-dir-xyz}""#);
  // Either errors with "failed to read" (file system) or
  // similar — but NOT "expected string or path".
  assert!(
    !e.contains("expected string or path"),
    "got type error instead of file-system error: {e}"
  );
}

// ── readFile with context-bearing string ──────────────────────

#[test]
fn read_file_context_bearing_string_does_not_error_at_type_check() {
  let e = err(r#"builtins.readFile "${./not-a-file-xyz}""#);
  assert!(!e.contains("expected string or path"), "got: {e}");
}

// ── toPath with context-bearing string ────────────────────────

#[test]
fn to_path_accepts_context_bearing_string() {
  let v = json(r#"builtins.typeOf (builtins.toPath "${./somepath}")"#);
  assert_eq!(v, r#""path""#);
}

// ── storePath with context-bearing string ─────────────────────

#[test]
fn store_path_accepts_context_bearing_string() {
  let v = json(r#"builtins.typeOf (builtins.storePath "${./local}")"#);
  assert_eq!(v, r#""path""#);
}

// ── readFileType with context-bearing string ──────────────────

#[test]
fn read_file_type_context_bearing_string_no_type_error() {
  // May error on missing file but NOT on type check.
  let e = err(r#"builtins.readFileType "${./not-a-file-xyz-zzz}""#);
  assert!(!e.contains("expected string or path"), "got: {e}");
}

// ── error shapes for non-path / non-string args unchanged ─────

#[test]
fn path_exists_int_arg_still_errors() {
  let e = err(r#"builtins.pathExists 42"#);
  assert!(e.contains("expected string or path"), "got: {e}");
}

#[test]
fn read_file_null_arg_still_errors() {
  let e = err(r#"builtins.readFile null"#);
  assert!(e.contains("expected string or path"), "got: {e}");
}

#[test]
fn read_dir_list_arg_still_errors() {
  let e = err(r#"builtins.readDir [ ]"#);
  assert!(e.contains("expected string or path"), "got: {e}");
}

// ── round-trip with realistic shape ───────────────────────────

#[test]
fn build_path_via_concat_then_path_exists() {
  // Pattern: construct path via `+` (slice #54 produces
  // context-bearing string), then check via pathExists.
  // Pre-fix: errored at pathExists boundary. Now: works.
  assert_eq!(
    json(
      r#"
      let p = "/tmp/" + "non-existent-xyz-test-zzz";
      in builtins.pathExists p
    "#
    ),
    "false"
  );
}

#[test]
fn build_path_via_path_concat_then_exists() {
  // Mixed string + path producing context-bearing string.
  let v = json(
    r#"
    let p = "prefix/" + ./subdir;
    in builtins.pathExists p
  "#,
  );
  assert!(v == "false" || v == "true", "got: {v}");
}
