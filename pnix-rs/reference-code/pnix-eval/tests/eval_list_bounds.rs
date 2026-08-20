//! Regression cover for list-op bounds checking.
//!
//! Several builtins previously returned `null` / `[]` silently on
//! invalid arguments, where Nix raises an error. Silent-pass shapes
//! bypass `if r != null then …` guards and let off-by-one bugs in
//! `.px` propagate undetected. This audit slice tightens:
//!
//!   - `builtins.head []`           → error "list is empty"
//!     (was: `null`)
//!   - `builtins.tail []`           → error "list is empty"
//!     (was: `[]`)
//!   - `builtins.elemAt l (-1)`     → error "negative index"
//!     (was: silently treated as `0`)
//!   - `builtins.elemAt l n` n≥len  → error "out of bounds"
//!     (was: `null`)
//!   - `builtins.genList f (-N)`    → error "negative count"
//!     (was: `[]`)
//!   - `builtins.take (-N) l`       → error "negative count"
//!     (was: `[]`)
//!   - `builtins.drop (-N) l`       → error "negative count"
//!     (was: full `l`)
//!
//! Positive-path behaviours that match Nix and stayed unchanged
//! (over-bound `take`/`drop`, `elemAt 0`, `genList _ 0`, `tail [x]`)
//! are pinned alongside.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

fn err_msg(src: &str) -> String {
  format!("{}", eval_expr(src).expect_err(src))
}

// ── head / tail empty errors ─────────────────────────────────────────

#[test]
fn head_empty_errors() {
  assert!(err_msg("builtins.head []").contains("list is empty"));
}

#[test]
fn tail_empty_errors() {
  assert!(err_msg("builtins.tail []").contains("list is empty"));
}

#[test]
fn head_singleton() {
  assert_eq!(json("builtins.head [ 7 ]"), "7");
}

#[test]
fn tail_singleton_returns_empty() {
  assert_eq!(json("builtins.tail [ 7 ]"), "[]");
}

// ── elemAt bounds ────────────────────────────────────────────────────

#[test]
fn elemat_in_bounds() {
  assert_eq!(json("builtins.elemAt [ 10 20 30 ] 0"), "10");
  assert_eq!(json("builtins.elemAt [ 10 20 30 ] 2"), "30");
}

#[test]
fn elemat_negative_errors() {
  let m = err_msg("builtins.elemAt [ 10 20 30 ] (-1)");
  assert!(m.contains("negative index"), "got: {m}");
}

#[test]
fn elemat_over_bound_errors() {
  let m = err_msg("builtins.elemAt [ 10 20 30 ] 5");
  assert!(m.contains("out of bounds"), "got: {m}");
}

#[test]
fn elemat_at_len_errors() {
  // index == len is also out of bounds.
  let m = err_msg("builtins.elemAt [ 10 20 30 ] 3");
  assert!(m.contains("out of bounds"), "got: {m}");
}

// ── genList ──────────────────────────────────────────────────────────

#[test]
fn genlist_zero() {
  assert_eq!(json("builtins.genList (i: i) 0"), "[]");
}

#[test]
fn genlist_three() {
  assert_eq!(json("builtins.genList (i: i * i) 3"), "[0,1,4]");
}

#[test]
fn genlist_negative_errors() {
  let m = err_msg("builtins.genList (i: i) (-1)");
  assert!(m.contains("negative count"), "got: {m}");
}

// ── take ─────────────────────────────────────────────────────────────

#[test]
fn take_zero_returns_empty() {
  assert_eq!(json("builtins.take 0 [ 1 2 3 ]"), "[]");
}

#[test]
fn take_more_than_len_returns_all() {
  // Nix-correct: take > len returns the whole list, no error.
  assert_eq!(json("builtins.take 5 [ 1 2 3 ]"), "[1,2,3]");
}

#[test]
fn take_negative_errors() {
  let m = err_msg("builtins.take (-1) [ 1 2 3 ]");
  assert!(m.contains("negative count"), "got: {m}");
}

// ── drop ─────────────────────────────────────────────────────────────

#[test]
fn drop_zero_returns_full() {
  assert_eq!(json("builtins.drop 0 [ 1 2 3 ]"), "[1,2,3]");
}

#[test]
fn drop_more_than_len_returns_empty() {
  assert_eq!(json("builtins.drop 5 [ 1 2 3 ]"), "[]");
}

#[test]
fn drop_negative_errors() {
  let m = err_msg("builtins.drop (-1) [ 1 2 3 ]");
  assert!(m.contains("negative count"), "got: {m}");
}

// ── length ───────────────────────────────────────────────────────────

#[test]
fn length_empty_zero() {
  assert_eq!(json("builtins.length []"), "0");
}

#[test]
fn length_three() {
  assert_eq!(json("builtins.length [ 1 2 3 ]"), "3");
}
