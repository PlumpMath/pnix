//! Regression cover for path normalization in equality (`==`,
//! `!=`) and comparison (`<`, `<=`, `>`, `>=`).
//!
//! 2026-05-05 audit findings (slice #65):
//!
//!   `./a/../b == ./b` returned `false`. The `compare_values`
//!   and `values_equal` Path arms compared `PathBuf`s literally
//!   — Rust's `PathBuf` does NOT collapse `..` (parent dir)
//!   components. So semantically-equivalent path representations
//!   compared unequal:
//!     - `./a/../b == ./b` returned `false` (should be `true`)
//!     - `./a/../b < ./b` ordered by literal text
//!     - `./a/b/c/../d == ./a/b/d` returned `false`
//!     - `/abs/x/../y == /abs/y` returned `false`
//!
//!   Pre-fix some collapses already happened (via Rust's
//!   `Path::components()` which skips empty parts and `.`):
//!     - `./a == ././a` returned `true` (single dots collapsed)
//!     - `./a/b == ./a//b` returned `true` (double slash
//!       collapsed)
//!     - `./a == ./a/` returned `true` (trailing slash
//!       collapsed)
//!   But `..` was NOT collapsed — the only path-equivalence
//!   case pnix missed.
//!
//!   Real Nix normalizes paths at construction; pnix normalizes
//!   at comparison time (via the new `normalize_pnix_path`
//!   helper). The helper collapses `.` (skip) and `..` (pop the
//!   last normal component, no-op past root, push if at start
//!   of relative path with no normal predecessor).
//!
//!   Production-relevant: `.px` authors who construct paths via
//!   `+` (e.g., `./project + "/build/.." + "/output"`) silently
//!   got false negatives on equality checks. List dedup via
//!   `==` on a list of paths failed to dedup equivalent paths.
//!   Sort order was determined by raw text, not semantics.
//!
//! Truth-owner files:
//! - `crates/pnix-eval/src/interpret.rs` `normalize_pnix_path`
//!   helper — new function. Walks components, collapses `.`
//!   and `..`. Preserves relative-path-ness (re-prepends `./`
//!   if all components collapsed and original was relative).
//! - `crates/pnix-eval/src/interpret.rs` `compare_values_at_depth`
//!   Path arm — compares normalized paths.
//! - `crates/pnix-eval/src/interpret.rs` `values_equal_at_depth`
//!   Path arm — compares normalized paths.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

// ── basic dotdot normalization ────────────────────────────────

#[test]
fn path_eq_collapses_dotdot() {
  assert_eq!(json(r#"./a/../b == ./b"#), "true");
}

#[test]
fn path_eq_collapses_dotdot_chain() {
  assert_eq!(json(r#"./a/b/c/../../d == ./a/d"#), "true");
}

#[test]
fn path_eq_collapses_multiple_dotdot() {
  assert_eq!(json(r#"./a/../b/../c == ./c"#), "true");
}

#[test]
fn path_eq_with_dotdot_no_collapse_needed() {
  assert_eq!(json(r#"./a == ./a"#), "true");
}

// ── still-different paths remain unequal ──────────────────────

#[test]
fn path_eq_different_paths_unequal() {
  assert_eq!(json(r#"./a/b == ./b/a"#), "false");
}

#[test]
fn path_eq_different_after_normalize() {
  assert_eq!(json(r#"./a/../b == ./c"#), "false");
}

// ── existing collapses still work ─────────────────────────────

#[test]
fn path_eq_single_dot_unchanged() {
  // Pre-existing: ./a == ././a
  assert_eq!(json(r#"./a == ././a"#), "true");
}

#[test]
fn path_eq_double_slash_unchanged() {
  // Pre-existing: ./a/b == ./a//b
  assert_eq!(json(r#"./a/b == ./a//b"#), "true");
}

#[test]
fn path_eq_trailing_slash_unchanged() {
  // Pre-existing: ./a == ./a/
  assert_eq!(json(r#"./a == ./a/"#), "true");
}

// ── absolute paths ────────────────────────────────────────────

#[test]
fn path_eq_absolute_dotdot() {
  assert_eq!(json(r#"/abs/x/../y == /abs/y"#), "true");
}

#[test]
fn path_eq_absolute_dotdot_at_root() {
  // /x/.. should normalize to /
  assert_eq!(json(r#"/x/../y == /y"#), "true");
}

// ── comparison ────────────────────────────────────────────────

#[test]
fn path_lt_normalized() {
  // Compare ./a/../b vs ./c — both normalize to ./b vs ./c.
  // ./b < ./c by lex text.
  assert_eq!(json(r#"./a/../b < ./c"#), "true");
}

#[test]
fn path_le_equal_after_normalize() {
  assert_eq!(json(r#"./a/../b <= ./b"#), "true");
}

#[test]
fn path_ge_equal_after_normalize() {
  assert_eq!(json(r#"./a/../b >= ./b"#), "true");
}

#[test]
fn path_gt_normalized() {
  assert_eq!(json(r#"./z/../b > ./a"#), "true");
}

// ── inequality (!=) ───────────────────────────────────────────

#[test]
fn path_neq_collapsed_paths() {
  assert_eq!(json(r#"./a/../b != ./b"#), "false");
}

// ── relative path edge cases ──────────────────────────────────

#[test]
fn path_dotdot_at_start_preserved() {
  // ../foo cannot be safely collapsed (no normal predecessor).
  // Both sides preserve `..` and they should be equal.
  assert_eq!(json(r#"../foo == ../foo"#), "true");
}

#[test]
fn path_dotdot_canceling_to_self() {
  // ./a/../b should normalize to ./b (collapsing produces
  // empty intermediate, but `b` remains).
  assert_eq!(json(r#"./a/../b == ./b"#), "true");
}

// ── list / attrset deep equality with paths ───────────────────

#[test]
fn list_of_paths_equal_after_normalize() {
  assert_eq!(json(r#"[ ./a/../b ./c ] == [ ./b ./c ]"#), "true");
}

#[test]
fn attrset_of_paths_equal_after_normalize() {
  assert_eq!(json(r#"{ x = ./a/../b; } == { x = ./b; }"#), "true");
}

// ── Path / String type mismatch unchanged ─────────────────────

#[test]
fn path_eq_string_type_mismatch_unchanged() {
  // Path != string even with same text.
  assert_eq!(json(r#"./foo == "./foo""#), "false");
}
