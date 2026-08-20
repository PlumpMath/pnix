//! Regression cover for the `<` / `<=` / `>` / `>=` /
//! `builtins.lt` / `le` / `gt` / `ge` cyclic-list DoS guard.
//!
//! 2026-05-05 audit findings (slice #43):
//!   - `let r = [ r ]; in r < r` overflowed the Rust call stack
//!     with `SIGABRT`. `compare_values` recursed into list
//!     children without cycle tracking; both sides forced to
//!     the same cyclic List and the comparison recursion
//!     looped. The same shape applied to `<=` / `>` / `>=`
//!     binary operators and to the `builtins.lt` / `le` / `gt`
//!     / `ge` builtins (all five reach the same helper).
//!   - The attrset-cycle case was already safe: pnix rejects
//!     attrset comparison entirely (`cannot compare set with
//!     set`), so the cycle never gets a chance to recurse —
//!     real Nix has the same restriction.
//!   - Real Nix errors with "infinite recursion encountered"
//!     on the list-cycle case. Match that shape via the same
//!     depth-limit mechanism as slice #42's `values_equal`:
//!     reuse `VALUES_EQUAL_MAX_DEPTH = 64` so the comparison
//!     and equality families share a consistent budget.
//!
//! Truth-owner files:
//! - `crates/pnix-eval/src/interpret.rs` — new
//!   `compare_values_at_depth(l, r, depth) -> Result<Ordering>`
//!   helper, `compare_values` wrapper at depth 0. The single
//!   helper is reachable from binary `<` / `<=` / `>` / `>=`
//!   in `eval_binary` (a `compare_values(l, r)?.is_lt()`-style
//!   call site) and from the `builtins.lt` / `le` / `gt` /
//!   `ge` arms in `apply_builtin`, so the one fix closes all
//!   five surfaces.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}
fn err_msg(src: &str) -> String {
  format!("{}", eval_expr(src).expect_err(src))
}

// ── self-cycle on comparison operators ─────────────────────────

#[test]
fn lt_self_cycle_list_errors() {
  let m = err_msg("let r = [ r ]; in r < r");
  assert!(m.contains("infinite recursion"), "got: {m}");
}

#[test]
fn le_self_cycle_list_errors() {
  let m = err_msg("let r = [ r ]; in r <= r");
  assert!(m.contains("infinite recursion"), "got: {m}");
}

#[test]
fn gt_self_cycle_list_errors() {
  let m = err_msg("let r = [ r ]; in r > r");
  assert!(m.contains("infinite recursion"), "got: {m}");
}

#[test]
fn ge_self_cycle_list_errors() {
  let m = err_msg("let r = [ r ]; in r >= r");
  assert!(m.contains("infinite recursion"), "got: {m}");
}

// ── builtins.lt / le / gt / ge cycle ──────────────────────────

#[test]
fn builtins_lt_self_cycle_errors() {
  let m = err_msg("let r = [ r ]; in builtins.lt r r");
  assert!(m.contains("infinite recursion"), "got: {m}");
}

#[test]
fn builtins_le_self_cycle_errors() {
  let m = err_msg("let r = [ r ]; in builtins.le r r");
  assert!(m.contains("infinite recursion"), "got: {m}");
}

#[test]
fn builtins_gt_self_cycle_errors() {
  let m = err_msg("let r = [ r ]; in builtins.gt r r");
  assert!(m.contains("infinite recursion"), "got: {m}");
}

#[test]
fn builtins_ge_self_cycle_errors() {
  let m = err_msg("let r = [ r ]; in builtins.ge r r");
  assert!(m.contains("infinite recursion"), "got: {m}");
}

// ── attrset comparison stays rejected (pre-existing) ───────────

#[test]
fn attrset_compare_rejected_so_cycle_no_op() {
  // pnix rejects attrset comparison entirely; the cycle never
  // gets to recurse because the type check fires first. This
  // pin captures that the cycle guard doesn't change the
  // pre-existing rejection.
  let m = err_msg("let r = { a = r; }; in r < r");
  assert!(m.contains("cannot compare"), "got: {m}");
  assert!(m.contains("set"), "got: {m}");
}

// ── legitimate deep comparison stays correct ───────────────────

#[test]
fn lt_genlist_50_works() {
  // 50-element flat list — depth 1 in comparison terms.
  assert_eq!(
    json("(builtins.genList (x: x) 50) < (builtins.genList (x: x + 1) 50)"),
    "true"
  );
}

#[test]
fn lt_strings_works() {
  assert_eq!(json(r#""abc" < "abd""#), "true");
  assert_eq!(json(r#""abc" < "abc""#), "false");
  assert_eq!(json(r#""abd" < "abc""#), "false");
}

#[test]
fn lt_paths_works() {
  assert_eq!(json("/a < /b"), "true");
  assert_eq!(json("/b < /a"), "false");
}

#[test]
fn lt_int_float_works() {
  assert_eq!(json("1 < 2.0"), "true");
  assert_eq!(json("2.0 < 1"), "false");
}

#[test]
fn lt_list_length_tiebreak_works() {
  // When prefixes match and one is shorter, the shorter list
  // is "less".
  assert_eq!(json("[ 1 2 ] < [ 1 2 3 ]"), "true");
  assert_eq!(json("[ 1 2 3 ] < [ 1 2 ]"), "false");
}

#[test]
fn lt_acyclic_dag_works() {
  // The same value reached via two paths — DAG, not cycle.
  // depth grows by 1 per descent, so DAG with branching factor
  // 2 stays within the limit at any reasonable scale.
  assert_eq!(
    json("let x = 1; in [ [ x ] [ x ] ] < [ [ x ] [ x x ] ]"),
    "true"
  );
}

#[test]
fn lt_force_error_propagates() {
  // A throw inside the value being compared used to be folded
  // into a comparison error path — confirm it still propagates
  // cleanly (the new ?-propagation in `force_value(l.clone())?`
  // is unchanged from before this slice; this is a sanity
  // pin).
  let m = err_msg(r#"[ (throw "inner") ] < [ 1 ]"#);
  assert!(m.contains("inner"), "got: {m}");
}
