//! Regression cover for `builtins.langVersion` — newly added
//! in slice #70.
//!
//! 2026-05-05 audit findings (slice #70):
//!
//!   `builtins.langVersion` was missing — pnix exposed
//!   `currentSystem`, `nixVersion`, and `storeDir` but not
//!   `langVersion`. Real Nix returns an int (currently 6 as
//!   of Nix 2.18+). `.px` code that branched on language
//!   version (a common feature-detection pattern in Nixpkgs
//!   `lib`) hit `attribute 'langVersion' not found` — the
//!   same misleading-error pattern as slice #68 (`hashFile`)
//!   and slice #69 (`unsafeAdd*`).
//!
//!   Pnix tracks Nix language version 6 (the current at time
//!   of audit). The value is a constant `Value::Int(6)`,
//!   not a function — accessing `builtins.langVersion`
//!   yields the integer directly.
//!
//! Truth-owner files:
//! - `crates/pnix-eval/src/interpret.rs` `builtins_attrset`
//!   helper — `langVersion` added next to `nixVersion` /
//!   `currentSystem` / `storeDir`.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

// ── existence + value ─────────────────────────────────────────

#[test]
fn lang_version_is_six() {
  assert_eq!(json(r#"builtins.langVersion"#), "6");
}

#[test]
fn lang_version_is_int_typed() {
  assert_eq!(json(r#"builtins.typeOf builtins.langVersion"#), r#""int""#);
}

#[test]
fn lang_version_is_not_a_function() {
  // It's a constant value, not a partial application.
  assert_eq!(json(r#"builtins.langVersion + 0"#), "6");
}

// ── feature-detection patterns ────────────────────────────────

#[test]
fn lang_version_can_branch_on_value() {
  // Realistic feature-detection pattern: branch on
  // langVersion to use newer-API features when available.
  assert_eq!(
    json(r#"if builtins.langVersion >= 6 then "new" else "old""#),
    r#""new""#
  );
}

#[test]
fn lang_version_compares_equal_to_six() {
  assert_eq!(json(r#"builtins.langVersion == 6"#), "true");
}

#[test]
fn lang_version_compares_less_than_seven() {
  assert_eq!(json(r#"builtins.langVersion < 7"#), "true");
}

// ── pre-existing version builtins still work ──────────────────

#[test]
fn current_system_unchanged() {
  let v = json(r#"builtins.currentSystem"#);
  // Just verify it returns a string (system varies).
  assert!(v.starts_with("\""), "got: {v}");
  assert!(v.ends_with("\""), "got: {v}");
}

#[test]
fn nix_version_unchanged() {
  let v = json(r#"builtins.nixVersion"#);
  assert!(v.contains("pnix"), "got: {v}");
}

#[test]
fn store_dir_unchanged() {
  let v = json(r#"builtins.storeDir"#);
  assert!(
    v.contains("/nix/store") || v.contains("nix-store"),
    "got: {v}"
  );
}
