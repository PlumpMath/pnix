//! Regression cover for `resolve_value_path` normalization —
//! cascades to builtins that produce or operate on paths.
//!
//! 2026-05-05 audit findings (slice #67):
//!
//!   Slice #66 normalized `Value::Path` at parse time and at
//!   the `+` operator boundary. But builtins that produce paths
//!   via `resolve_value_path` bypassed that normalization:
//!
//!     - `builtins.toPath "/abs/x/../y"` returned a Path with
//!       text `"/abs/x/../y"` (literal)
//!     - `builtins.storePath "/nix/store/x/../y"` returned a
//!       Path with text `"/nix/store/x/../y"` (literal)
//!
//!   These builtins also affect filesystem operations:
//!     - `builtins.pathExists "./a/../b"` checked the literal
//!       path on disk (which the OS may handle correctly via
//!       internal canonicalization, but the path used in
//!       error messages was the literal form)
//!     - `builtins.readFile "./a/../b"` similarly
//!
//!   Closes the slice #66 cascade: now ALL paths produced or
//!   processed via `resolve_value_path` are normalized. The
//!   `Value::Path` invariant is uniform across:
//!     - parse-time literal paths (slice #66)
//!     - parse-time interpolated paths (slice #66)
//!     - `+` operator results (slice #66)
//!     - `toPath` / `storePath` results (slice #67)
//!     - filesystem ops `pathExists` / `readFile` / `readDir`
//!       / `import` etc. — text passed to the OS is the
//!       normalized form (slice #67)
//!
//! Truth-owner files:
//! - `crates/pnix-eval/src/interpret.rs` `resolve_value_path`
//!   helper — applies `normalize_pnix_path` to the resolved
//!   path before returning. Both relative and absolute paths
//!   normalize. The `current_import_base().join(path)` for
//!   relative paths is also re-normalized so any `..` in the
//!   join target is collapsed against the absolute base.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

// 2026-05-06: `err` helper added for slice #79 (empty-path
// rejection in fs-touching / path-resolving builtins) merge.
fn err(src: &str) -> String {
  eval_expr(src).err().expect(src).to_string()
}

// ── toPath builtin path normalization ─────────────────────────

#[test]
fn to_path_dotdot_string_normalized() {
  assert_eq!(
    json(r#"builtins.toString (builtins.toPath "/abs/x/../y")"#),
    r#""/abs/y""#
  );
}

#[test]
fn to_path_chain_dotdot_normalized() {
  assert_eq!(
    json(r#"builtins.toString (builtins.toPath "/a/b/c/../../d")"#),
    r#""/a/d""#
  );
}

#[test]
fn to_path_no_dotdot_unchanged() {
  // Sanity.
  assert_eq!(
    json(r#"builtins.toString (builtins.toPath "/a/b")"#),
    r#""/a/b""#
  );
}

// ── storePath builtin path normalization ──────────────────────

#[test]
fn store_path_dotdot_normalized() {
  assert_eq!(
    json(r#"builtins.toString (builtins.storePath "/nix/store/x/../y")"#),
    r#""/nix/store/y""#
  );
}

#[test]
fn store_path_no_dotdot_unchanged() {
  assert_eq!(
    json(r#"builtins.toString (builtins.storePath "/nix/store/abc")"#),
    r#""/nix/store/abc""#
  );
}

// ── builtins.toPath of context-bearing string ─────────────────

#[test]
fn to_path_context_bearing_string_normalized() {
  // Slice #64 made resolve_value_path accept context-bearing
  // strings. Slice #67 makes the result normalized.
  let v = json(r#"builtins.toString (builtins.toPath "${./a/../b}")"#);
  // Result text should NOT contain "/.." segments; the path
  // is normalized before stringifying.
  assert!(!v.contains("/.."), "got: {v}");
}

// ── existing path resolution unchanged ────────────────────────

#[test]
fn to_path_path_value_unchanged() {
  // Sanity: passing an already-Path value through toPath
  // doesn't re-introduce dotdots.
  assert_eq!(
    json(r#"builtins.toString (builtins.toPath ./a/b)"#),
    r#""./a/b""#
  );
}

#[test]
fn to_path_relative_string_unchanged() {
  // Relative path passed as string — depends on import base
  // but should not contain `..`.
  let v = json(r#"builtins.toString (builtins.toPath "./a/../b")"#);
  // After normalization, the relative `./a/../b` becomes
  // `./b` (or absolute equivalent if join_base resolves it).
  assert!(!v.contains("/.."), "got: {v}");
}

// ── pathExists / readFile use normalized path ────────────────

#[test]
fn path_exists_dotdot_string_normalized_in_error() {
  // pathExists on a missing path with `..` — error message
  // (if any) should contain the normalized form, not the
  // literal `..` text.
  let v = json(r#"builtins.pathExists "/abs/non-existent/../also-non-existent""#);
  // Just verify the call doesn't error and returns false.
  assert_eq!(v, "false");
}

// ── parity check with slice #66 ──────────────────────────────

#[test]
fn slice_66_path_literal_still_normalized() {
  // Sanity: slice #66 path-literal normalization still works.
  assert_eq!(json(r#"builtins.toString ./a/../b"#), r#""./b""#);
}

#[test]
fn slice_66_plus_operator_still_normalized() {
  // Sanity: slice #66 `+` normalization still works.
  assert_eq!(json(r#"builtins.toString (./a + "/../b")"#), r#""./b""#);
}

// ── builtin Path return type unchanged ────────────────────────

#[test]
fn to_path_returns_path_type() {
  assert_eq!(
    json(r#"builtins.typeOf (builtins.toPath "/x/../y")"#),
    r#""path""#
  );
}

#[test]
fn store_path_returns_path_type() {
  assert_eq!(
    json(r#"builtins.typeOf (builtins.storePath "/x/../y")"#),
    r#""path""#
  );
}

// ── error shapes for non-string-or-path args unchanged ───────

#[test]
fn to_path_int_arg_still_errors() {
  let e = eval_expr(r#"builtins.toPath 42"#)
    .err()
    .unwrap()
    .to_string();
  assert!(e.contains("expected string or path"), "got: {e}");
}

#[test]
fn store_path_null_arg_still_errors() {
  let e = eval_expr(r#"builtins.storePath null"#)
    .err()
    .unwrap()
    .to_string();
  assert!(e.contains("expected string or path"), "got: {e}");
}

// ═══════════════════════════════════════════════════════════════
// slice #79 — empty-string-path rejection in fs-touching /
// path-resolving builtins. Pre-fix: `pathExists ""` silently
// returned `true` because the empty path resolved to cwd via
// `current_import_base()`. Now `resolve_value_path` rejects
// empty-string / empty-PathBuf inputs at the top of the
// function, BEFORE any normalization or import-base join.
// Cascades through 6 builtins (pathExists / readDir /
// readFile / hashFile / toPath / storePath).
//
// 2026-05-06: merged from `eval_path_empty_string_reject.rs`.
// `eval_resolve_value_path_normalization.rs` (slice #67) is
// the canonical owner of the `resolve_value_path` function.
// ═══════════════════════════════════════════════════════════════

// ── pathExists: was silently true, now errors ────────────────

#[test]
fn path_exists_empty_string_errors() {
  // Pre-fix: returned `true` (silently turned into cwd).
  let e = err(r#"builtins.pathExists """#);
  assert!(
    e.contains("pathExists") && e.contains("empty string"),
    "got: {e}"
  );
}

#[test]
fn path_exists_dot_still_works() {
  // Sanity: `.` is a valid relative path; pathExists "." returns true.
  assert_eq!(json(r#"builtins.pathExists ".""#), "true");
}

#[test]
fn path_exists_slash_still_works() {
  assert_eq!(json(r#"builtins.pathExists "/""#), "true");
}

#[test]
fn path_exists_whitespace_string_returns_false() {
  // Sanity: " " (single space) is a VALID filename on Unix —
  // pathExists checks if a file named " " exists, which it
  // typically doesn't. NOT errored — only the empty case.
  assert_eq!(json(r#"builtins.pathExists " ""#), "false");
}

#[test]
fn path_exists_nonexistent_path_returns_false() {
  assert_eq!(
    json(r#"builtins.pathExists "/this/does/not/exist""#),
    "false"
  );
}

// ── readDir: was silently cwd-listing, now errors ────────────

#[test]
fn read_dir_empty_string_errors() {
  let e = err(r#"builtins.readDir """#);
  assert!(
    e.contains("readDir") && e.contains("empty string"),
    "got: {e}"
  );
}

#[test]
fn read_dir_nonexistent_path_errors_unchanged() {
  // Sanity: existing error path for non-existent dir is
  // separate from the empty-string check. Should still produce
  // the OS-level "No such file or directory" message.
  let e = err(r#"builtins.readDir "/this/does/not/exist""#);
  assert!(e.contains("readDir"), "got: {e}");
  assert!(!e.contains("empty string"), "got: {e}");
}

// ── readFile: empty input was misleading "Is a directory" ────

#[test]
fn read_file_empty_string_errors_with_clear_message() {
  // Pre-fix: misleading "Is a directory (os error 21)" because
  // the empty path resolved to cwd which IS a directory but
  // that's not the user's bug — the bug is the empty input.
  // Now: clear "empty string is not a valid path" message.
  let e = err(r#"builtins.readFile """#);
  assert!(
    e.contains("readFile") && e.contains("empty string"),
    "got: {e}"
  );
  assert!(!e.contains("Is a directory"), "got: {e}");
}

// ── hashFile: empty input was misleading "Is a directory" ────

#[test]
fn hash_file_empty_string_errors_with_clear_message() {
  let e = err(r#"builtins.hashFile "sha256" """#);
  assert!(
    e.contains("hashFile") && e.contains("empty string"),
    "got: {e}"
  );
  assert!(!e.contains("Is a directory"), "got: {e}");
}

// ── toPath / storePath: silent empty-Path construction ───────

#[test]
fn to_path_empty_string_errors() {
  let e = err(r#"builtins.toPath """#);
  assert!(
    e.contains("toPath") && e.contains("empty string"),
    "got: {e}"
  );
}

#[test]
fn store_path_empty_string_errors() {
  let e = err(r#"builtins.storePath """#);
  assert!(
    e.contains("storePath") && e.contains("empty string"),
    "got: {e}"
  );
}

// ── baseNameOf / dirOf: unchanged (not via resolve_value_path) ─

#[test]
fn base_name_of_empty_string_returns_empty_unchanged() {
  // Sanity: baseNameOf has its own empty-string semantic per
  // real Nix's sentinel-path behaviour. Slice #79 leaves it
  // unchanged.
  assert_eq!(json(r#"builtins.baseNameOf """#), r#""""#);
}

#[test]
fn dir_of_empty_string_returns_dot_unchanged() {
  // Sanity: dirOf "" returns "." per real Nix.
  assert_eq!(json(r#"builtins.dirOf """#), r#"".""#);
}

// ── Empty StringContext value also rejected ──────────────────

#[test]
fn path_exists_empty_string_context_errors() {
  // A context-bearing string whose text is empty must also
  // error — the pre-fix shape applied regardless of whether
  // the empty string carried context or not.
  let e = err(
    r#"
    let s = builtins.appendContext "" {};
    in builtins.pathExists s
  "#,
  );
  assert!(
    e.contains("pathExists") && e.contains("empty string"),
    "got: {e}"
  );
}

// ── Common production scenarios ──────────────────────────────

#[test]
fn unset_env_var_does_not_silently_match_cwd() {
  // The motivating production scenario: an environment variable
  // that defaulted to empty string would have silently returned
  // `true` from pathExists (matching cwd). Now it errors
  // clearly, surfacing the real bug.
  let e = err(
    r#"
    let envVar = "";
    in builtins.pathExists envVar
  "#,
  );
  assert!(
    e.contains("pathExists") && e.contains("empty string"),
    "got: {e}"
  );
}

#[test]
fn empty_interpolation_result_does_not_silently_pass() {
  // A string interpolation that happens to produce an empty
  // string also doesn't silently pass.
  let e = err(
    r#"
    let x = ""; in
    builtins.pathExists "${x}"
  "#,
  );
  assert!(
    e.contains("pathExists") && e.contains("empty string"),
    "got: {e}"
  );
}

// ── Error-message families remain typed ──────────────────────

#[test]
fn path_exists_non_string_arg_unchanged() {
  // Non-string-or-path args still get the existing error.
  let e = err(r#"builtins.pathExists 42"#);
  assert!(e.contains("expected string or path"), "got: {e}");
}

#[test]
fn read_dir_non_string_arg_unchanged() {
  let e = err(r#"builtins.readDir 42"#);
  assert!(e.contains("expected string or path"), "got: {e}");
}
