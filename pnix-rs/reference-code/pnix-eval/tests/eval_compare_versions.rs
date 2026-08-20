//! Regression cover for `builtins.compareVersions` Nix-canonical
//! semantics.
//!
//! 2026-05-04 audit findings: pnix-eval's `compare_versions` helper
//! diverged from Nix's `compareVersionComponents` in three ways, and
//! the silent-divergent results would mis-rank packages compared to
//! upstream `nix` / `nix-env -u`:
//!
//!   - **Trailing-zero / missing-component sign flip**: e.g.
//!     `compareVersions "1.2" "1.2.0"` returned `1`, claiming
//!     `"1.2" > "1.2.0"`. Nix says `-1` (empty component is older
//!     than a numeric `"0"`). Same flip for `"1" vs "1.0"`,
//!     `"1.0.0" vs "1.0"`, `"1" vs "1.0.0"`, etc.
//!   - **`numeric vs non-numeric` sign flip**: the `Err vs Ok` arm
//!     returned `1` (string > numeric), but Nix says numeric > string.
//!     Visible at `"1.0a1" vs "1.0a"`: Nix says `1` (`1` is numeric,
//!     `""` is missing → numeric newer), pnix said `-1`.
//!   - **Non-Nix `+` / `~` substring branches**: `compareVersions
//!     "1.0" "1.0+rev"` returned `1`, but Nix says `-1` (because
//!     splitVersion produces `["1","0"]` vs `["1","0","+rev"]` and
//!     the empty trailing component is older than the non-empty
//!     non-numeric `"+rev"`). Removed; the natural rules already
//!     produce the right answer for `+`/`~` cases.
//!
//! Truth-owner: `crates/pnix-eval/src/interpret.rs`
//! → `compare_version_component` + `compare_versions`. Mirrors
//! `nix/src/libutil/strings.cc` `compareVersionComponents`.

use pnix_eval::eval_expr;

fn cmp(v1: &str, v2: &str) -> i64 {
  let src = format!(r#"builtins.compareVersions "{}" "{}""#, v1, v2);
  match eval_expr(&src).unwrap_or_else(|e| panic!("eval {src}: {e}")) {
    pnix_eval::Value::Int(n) => n,
    other => panic!("compareVersions returned non-int: {other}"),
  }
}

// ── Trailing-zero / missing-component rule ────────────────────

#[test]
fn trailing_zero_makes_longer_newer() {
  // `1.2.0` is NEWER than `1.2` (the shorter has an empty trailing
  // component which is older than `"0"`).
  assert_eq!(cmp("1.2", "1.2.0"), -1);
  assert_eq!(cmp("1.2.0", "1.2"), 1);
}

#[test]
fn missing_components_strictly_older() {
  assert_eq!(cmp("1", "1.0"), -1);
  assert_eq!(cmp("1.0", "1"), 1);
  assert_eq!(cmp("1", "1.0.0"), -1);
  assert_eq!(cmp("1.0.0", "1"), 1);
}

#[test]
fn equal_components_are_zero() {
  assert_eq!(cmp("1.2", "1.2"), 0);
  assert_eq!(cmp("", ""), 0);
  assert_eq!(cmp("1.0pre", "1.0pre"), 0);
}

// ── Pre-release rule ──────────────────────────────────────────

#[test]
fn pre_is_older_than_release() {
  // `1.0pre1` < `1.0` because `pre` < empty trailing component.
  assert_eq!(cmp("1.0pre1", "1.0"), -1);
  assert_eq!(cmp("1.0", "1.0pre1"), 1);
  // `pre` alone (no number) is also older than the release.
  assert_eq!(cmp("1.0pre", "1.0"), -1);
  assert_eq!(cmp("1.0", "1.0pre"), 1);
}

#[test]
fn pre_with_number_orders_by_number() {
  // pre1 < pre2 (numeric component compare after the pre tag).
  assert_eq!(cmp("1.0pre1", "1.0pre2"), -1);
  assert_eq!(cmp("1.0pre2", "1.0pre1"), 1);
}

#[test]
fn pre_alone_older_than_pre_numbered() {
  // `pre` then nothing < `pre` then `1`: empty < numeric.
  assert_eq!(cmp("1.0pre", "1.0pre1"), -1);
  assert_eq!(cmp("1.0pre1", "1.0pre"), 1);
}

// ── Numeric vs non-numeric component ──────────────────────────

#[test]
fn numeric_component_outranks_non_numeric() {
  // `1.0a1` > `1.0a` because at index 3 we compare numeric `"1"` vs
  // empty (and `1` is numeric, empty is non-numeric → numeric wins).
  assert_eq!(cmp("1.0a1", "1.0a"), 1);
  assert_eq!(cmp("1.0a", "1.0a1"), -1);
}

#[test]
fn non_numeric_trailing_newer_than_empty() {
  // `1.0a` > `1.0` because the trailing `"a"` component is non-empty
  // and is not `"pre"`, so it sorts after the missing component.
  assert_eq!(cmp("1.0a", "1.0"), 1);
  assert_eq!(cmp("1.0", "1.0a"), -1);
}

#[test]
fn non_numeric_lex_compare() {
  assert_eq!(cmp("1.0a", "1.0b"), -1);
  assert_eq!(cmp("1.0b", "1.0a"), 1);
}

// ── Numeric value compare (not lexical) ───────────────────────

#[test]
fn numeric_compares_as_numbers_not_strings() {
  // Lexically `"10" < "2"`, but numerically `10 > 2`.
  assert_eq!(cmp("2", "10"), -1);
  assert_eq!(cmp("10", "2"), 1);
  assert_eq!(cmp("1.10", "1.2"), 1);
  assert_eq!(cmp("1.2", "1.10"), -1);
}

// ── `+` / `~` are NOT special (Nix-compatible) ────────────────

#[test]
fn plus_revision_is_newer_via_natural_rule() {
  // `splitVersion "1.0+rev"` = `["1","0","+rev"]`. At index 2 we
  // compare empty vs `"+rev"`; `"+rev"` is non-empty non-`"pre"`
  // so it sorts AFTER empty. Hence `"1.0+rev"` > `"1.0"`.
  // (Earlier code hard-coded `+` as older, which contradicted Nix.)
  assert_eq!(cmp("1.0", "1.0+rev"), -1);
  assert_eq!(cmp("1.0+rev", "1.0"), 1);
}

#[test]
fn tilde_in_component_is_lex_compared() {
  // `"1.0~rc"` vs `"1.0"`: same logic as `+`. `"~rc"` is non-empty
  // non-`"pre"` non-numeric → sorts after empty → `"1.0~rc"` newer.
  assert_eq!(cmp("1.0", "1.0~rc"), -1);
  assert_eq!(cmp("1.0~rc", "1.0"), 1);
}

// ── Type guard preserved ──────────────────────────────────────

#[test]
fn compare_versions_non_string_errors() {
  let m = format!(
    "{}",
    eval_expr(r#"builtins.compareVersions 1 "1.0""#).expect_err("non-string")
  );
  assert!(m.contains("expected two strings"), "got: {m}");
}
