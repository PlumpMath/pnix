//! Audit-clean baselines for filesystem builtins.
//!
//! `builtins.pathExists`, `readFile`, `readDir` already follow the
//! Nix manual + production fail-loud guarantee: every error
//! includes the operation, the path, and the OS error message.
//! Pinned here so future filesystem-surface refactors (vfs adapter,
//! sandboxing layer, mock infrastructure) cannot silently flip
//! these to silent-pass shapes.
//!
//! Production-readiness contract:
//!   - `pathExists` is total — returns `true`/`false`, never errors.
//!   - `readFile` / `readDir` errors include
//!     `builtins.X: <action> '<path>': <os reason>` so a `.px`
//!     author can grep the message and decide what to do.
//!   - `readDir` returns `{ <name> = <kind>; … }` where `<kind>`
//!     is one of the canonical strings `regular`, `directory`,
//!     `symlink`, etc. The map shape is sorted by name (BTreeMap).
//!   - Both `pathExists` and `readFile` accept either `Value::Path`
//!     or `Value::String` — the string form lets `.px` authors
//!     compose paths from interpolation without explicit
//!     conversion.

use pnix_eval::eval_expr;
use std::sync::Once;

const PROBE_DIR: &str = "/tmp/pnix-fs-probe-fixtures";

fn fixtures() -> &'static str {
  static ONCE: Once = Once::new();
  ONCE.call_once(|| {
    let _ = std::fs::remove_dir_all(PROBE_DIR);
    std::fs::create_dir_all(format!("{PROBE_DIR}/dir")).unwrap();
    std::fs::write(format!("{PROBE_DIR}/dir/file.txt"), "hello\n").unwrap();
    std::fs::write(format!("{PROBE_DIR}/multi.txt"), "line1\nline2\n").unwrap();
  });
  PROBE_DIR
}

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

fn err_msg(src: &str) -> String {
  format!("{}", eval_expr(src).expect_err(src))
}

// ── pathExists ─────────────────────────────────────────────────────

#[test]
fn path_exists_existing_file() {
  fixtures();
  assert_eq!(
    json(&format!(r#"builtins.pathExists {PROBE_DIR}/multi.txt"#)),
    "true"
  );
}

#[test]
fn path_exists_existing_directory() {
  fixtures();
  assert_eq!(
    json(&format!(r#"builtins.pathExists {PROBE_DIR}/dir"#)),
    "true"
  );
}

#[test]
fn path_exists_missing_returns_false() {
  fixtures();
  assert_eq!(
    json(&format!(r#"builtins.pathExists {PROBE_DIR}/nonexistent"#)),
    "false"
  );
}

#[test]
fn path_exists_missing_deeply_nested_returns_false() {
  // No partial-error semantics — even when many segments are missing.
  assert_eq!(
    json(r#"builtins.pathExists /nope/really/nope/nope"#),
    "false"
  );
}

#[test]
fn path_exists_accepts_string_arg() {
  fixtures();
  assert_eq!(
    json(&format!(r#"builtins.pathExists "{PROBE_DIR}/multi.txt""#)),
    "true"
  );
}

// ── readFile ──────────────────────────────────────────────────────

#[test]
fn read_file_returns_contents() {
  fixtures();
  assert_eq!(
    json(&format!(r#"builtins.readFile {PROBE_DIR}/multi.txt"#)),
    r#""line1\nline2\n""#
  );
}

#[test]
fn read_file_missing_errors_with_path_and_os_message() {
  fixtures();
  let m = err_msg(&format!(r#"builtins.readFile {PROBE_DIR}/nonexistent"#));
  assert!(m.contains("builtins.readFile"), "got: {m}");
  assert!(m.contains("nonexistent"), "got: {m}");
  // OS error 2 wording can be locale-dependent; just confirm
  // the OS-error envelope is present.
  assert!(m.contains("os error") || m.contains("No such"), "got: {m}");
}

#[test]
fn read_file_on_directory_errors() {
  fixtures();
  let m = err_msg(&format!(r#"builtins.readFile {PROBE_DIR}/dir"#));
  assert!(m.contains("builtins.readFile"), "got: {m}");
  assert!(m.contains(PROBE_DIR), "got: {m}");
}

// ── readDir ───────────────────────────────────────────────────────

#[test]
fn read_dir_returns_name_to_kind_map() {
  fixtures();
  let v = json(&format!(r#"builtins.readDir {PROBE_DIR}"#));
  // Map is sorted by name (BTreeMap-backed); both entries present.
  assert_eq!(v, r#"{"dir":"directory","multi.txt":"regular"}"#);
}

#[test]
fn read_dir_subdirectory() {
  fixtures();
  assert_eq!(
    json(&format!(r#"builtins.readDir {PROBE_DIR}/dir"#)),
    r#"{"file.txt":"regular"}"#
  );
}

#[test]
fn read_dir_missing_errors() {
  fixtures();
  let m = err_msg(&format!(r#"builtins.readDir {PROBE_DIR}/nonexistent"#));
  assert!(m.contains("builtins.readDir"), "got: {m}");
  assert!(m.contains("nonexistent"), "got: {m}");
}

#[test]
fn read_dir_on_regular_file_errors() {
  fixtures();
  let m = err_msg(&format!(r#"builtins.readDir {PROBE_DIR}/multi.txt"#));
  assert!(m.contains("builtins.readDir"), "got: {m}");
  // Either "Not a directory" or path mention is enough — the
  // exact OS-error text is locale-dependent.
  assert!(
    m.contains("Not a directory") || m.contains("multi.txt"),
    "got: {m}"
  );
}
