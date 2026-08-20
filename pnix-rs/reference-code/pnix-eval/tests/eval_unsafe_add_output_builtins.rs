//! Regression cover for `unsafeAddOutputDependency` and
//! `unsafeAddOutputName` — newly added in slice #69 to close
//! the missing-builtin gap.
//!
//! 2026-05-05 audit findings (slice #69):
//!
//!   `builtins.unsafeAddOutputDependency` and
//!   `builtins.unsafeAddOutputName` were missing — pnix had
//!   their inverse (`unsafeDiscardOutputDependency`) but not
//!   the additive forms. Real Nix ships both halves of the
//!   pair: `unsafeAddOutputDependency` adds output-dependency
//!   markers (`!out!path`) to a string's context;
//!   `unsafeAddOutputName name str` adds output-name markers
//!   (`!name!path`) for multi-output derivation references.
//!
//!   Pre-fix `.px` code that called either builtin errored
//!   with `attribute 'unsafeAddOutputDependency' not found` —
//!   misleading error suggesting attrset-access failure
//!   rather than unimplemented builtin.
//!
//!   Implementation matches pnix's prefix-encoded context
//!   scheme (consistent with `unsafeDiscardOutputDependency`
//!   filter): plain-path context entries (no `!` or `=`
//!   prefix) get marked with `!out!path` or `!name!path`.
//!   Already-prefixed entries preserve their existing role.
//!   Plain-string input (no context) is a no-op.
//!
//! Truth-owner files:
//! - `crates/pnix-eval/src/interpret.rs` `apply_builtin`
//!   `"unsafeAddOutputDependency"` arm — new implementation.
//!   For each plain-path context entry, adds `!out!path` to
//!   the context. Preserves text and existing prefixed
//!   entries.
//! - `crates/pnix-eval/src/interpret.rs` `apply_builtin`
//!   `"unsafeAddOutputName"` arm — new implementation. Takes
//!   `name` (string) and `str`. For each plain-path context
//!   entry, adds `!<name>!path` to the context.
//! - `crates/pnix-eval/src/interpret.rs` builtins
//!   registration list — both new builtins added next to
//!   `unsafeDiscardOutputDependency`.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

fn err(src: &str) -> String {
  eval_expr(src).err().expect(src).to_string()
}

// ── unsafeAddOutputDependency: existence + happy paths ────────

#[test]
fn unsafe_add_output_dependency_exists() {
  // Pre-fix: error "attribute not found".
  assert_eq!(
    json(r#"builtins.typeOf builtins.unsafeAddOutputDependency"#),
    r#""lambda""#
  );
}

#[test]
fn unsafe_add_output_dependency_plain_string_no_op() {
  // No context to mark — result is the same plain string.
  assert_eq!(json(r#"builtins.unsafeAddOutputDependency "x""#), r#""x""#);
}

#[test]
fn unsafe_add_output_dependency_preserves_text() {
  // Text portion unchanged.
  let v = json(
    r#"
    builtins.unsafeAddOutputDependency "x${./p}"
  "#,
  );
  // Contains "x" prefix and the resolved path text.
  assert!(v.starts_with("\"x"), "got: {v}");
}

#[test]
fn unsafe_add_output_dependency_adds_marker_to_context() {
  // The added marker is `!out!path`. Verify it appears.
  let v = json(
    r#"
    builtins.toJSON (builtins.getContext (
      builtins.unsafeAddOutputDependency "x${./p}"
    ))
  "#,
  );
  // Original `./p` is in context PLUS `!out!./p` marker.
  assert!(v.contains("./p"), "got: {v}");
  assert!(v.contains("!out!"), "got: {v}");
}

#[test]
fn unsafe_add_output_dependency_idempotent_on_marked_entry() {
  // Already-marked entries preserve role.
  let v = json(
    r#"
    builtins.length (builtins.attrNames (builtins.getContext (
      builtins.unsafeAddOutputDependency
        (builtins.unsafeAddOutputDependency "x${./p}")
    )))
  "#,
  );
  // Two entries: original `./p` + `!out!./p`. Calling twice
  // doesn't add another `!!out!!out!./p` because the inner
  // call already prefixed (and prefix filter excludes `!`-
  // prefixed from re-marking).
  assert_eq!(v, "2");
}

#[test]
fn unsafe_add_output_dependency_int_arg_errors() {
  let e = err(r#"builtins.unsafeAddOutputDependency 42"#);
  assert!(e.contains("unsafeAddOutputDependency"), "got: {e}");
  assert!(e.contains("expected string"), "got: {e}");
  assert!(e.contains("int"), "got: {e}");
}

#[test]
fn unsafe_add_output_dependency_null_arg_errors() {
  let e = err(r#"builtins.unsafeAddOutputDependency null"#);
  assert!(e.contains("unsafeAddOutputDependency"), "got: {e}");
  assert!(e.contains("null"), "got: {e}");
}

// ── unsafeAddOutputName: existence + happy paths ──────────────

#[test]
fn unsafe_add_output_name_exists() {
  assert_eq!(
    json(r#"builtins.typeOf builtins.unsafeAddOutputName"#),
    r#""lambda""#
  );
}

#[test]
fn unsafe_add_output_name_partial_returns_lambda() {
  assert_eq!(
    json(r#"builtins.typeOf (builtins.unsafeAddOutputName "out")"#),
    r#""lambda""#
  );
}

#[test]
fn unsafe_add_output_name_plain_string_no_op() {
  assert_eq!(json(r#"builtins.unsafeAddOutputName "out" "x""#), r#""x""#);
}

#[test]
fn unsafe_add_output_name_adds_named_marker() {
  let v = json(
    r#"
    builtins.toJSON (builtins.getContext (
      builtins.unsafeAddOutputName "dev" "x${./p}"
    ))
  "#,
  );
  assert!(v.contains("./p"), "got: {v}");
  assert!(v.contains("!dev!"), "got: {v}");
}

#[test]
fn unsafe_add_output_name_different_names_different_markers() {
  let v_out = json(
    r#"
    builtins.toJSON (builtins.getContext (
      builtins.unsafeAddOutputName "out" "x${./p}"
    ))
  "#,
  );
  let v_dev = json(
    r#"
    builtins.toJSON (builtins.getContext (
      builtins.unsafeAddOutputName "dev" "x${./p}"
    ))
  "#,
  );
  assert!(v_out.contains("!out!"), "out: {v_out}");
  assert!(v_dev.contains("!dev!"), "dev: {v_dev}");
}

#[test]
fn unsafe_add_output_name_int_name_arg_errors() {
  let e = err(r#"builtins.unsafeAddOutputName 42 "x""#);
  assert!(e.contains("unsafeAddOutputName"), "got: {e}");
  assert!(e.contains("first arg"), "got: {e}");
  assert!(e.contains("must be string"), "got: {e}");
  assert!(e.contains("int"), "got: {e}");
}

#[test]
fn unsafe_add_output_name_int_str_arg_errors() {
  let e = err(r#"builtins.unsafeAddOutputName "out" 42"#);
  assert!(e.contains("unsafeAddOutputName"), "got: {e}");
  assert!(e.contains("second arg"), "got: {e}");
  assert!(e.contains("must be string"), "got: {e}");
  assert!(e.contains("int"), "got: {e}");
}

// ── round-trip with unsafeDiscardOutputDependency ─────────────

#[test]
fn add_then_discard_round_trip() {
  // After Add then Discard, the `!out!` marker is gone.
  // Original `./p` remains (Discard doesn't strip plain
  // paths).
  let v = json(
    r#"
    builtins.toJSON (builtins.getContext (
      builtins.unsafeDiscardOutputDependency
        (builtins.unsafeAddOutputDependency "x${./p}")
    ))
  "#,
  );
  // `./p` remains, `!out!` markers stripped.
  assert!(v.contains("./p"), "got: {v}");
  assert!(!v.contains("!out!"), "got: {v}");
}

// ── pre-existing builtins still work ──────────────────────────

#[test]
fn unsafe_discard_output_dependency_unchanged() {
  // Sanity: the inverse builtin still works.
  assert_eq!(
    json(r#"builtins.unsafeDiscardOutputDependency "x""#),
    r#""x""#
  );
}

#[test]
fn unsafe_discard_string_context_unchanged() {
  assert_eq!(json(r#"builtins.unsafeDiscardStringContext "x""#), r#""x""#);
}
