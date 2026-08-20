//! Regression cover for cyclic-value DoS guards on
//! `builtins.toJSON` and `builtins.deepSeq`.
//!
//! 2026-05-05 audit findings (slice #41):
//!   - `let r = { a = r; }; in builtins.toJSON r` overflowed the
//!     Rust call stack with `SIGABRT`. The boundary's
//!     `deep_force` (slice-#3X cycle-tracked) successfully
//!     un-thunked the outer level but the cyclic `Thunk` at
//!     the leaf was preserved in the returned tree. Then
//!     `check_json_finite` re-walked the tree, force-value'd
//!     the inner Thunk, got the same AttrSet back, recursed,
//!     repeat — heavy frames, stack blew up at ~57 iterations.
//!   - `let r = { a = r; }; in builtins.deepSeq r 99` had the
//!     same shape: `deep_force_value` had a `DEEP_FORCE_MAX_DEPTH
//!     = 1_000` depth limit but no Rc-pointer cycle tracking,
//!     so heavy frames per cycle iteration overflowed before
//!     depth=1000 was reached.
//!   - Real Nix errors with "infinite recursion encountered"
//!     on both. Match that shape: cycle detection via
//!     `Rc::ptr_eq` over the `Thunk.cache` pointers along the
//!     current descent path. Mirrors the existing
//!     `deep_force_visited` pattern (see commit aad08893 for
//!     the path-stack invariant discussion).
//!
//! Truth-owner files:
//! - `crates/pnix-eval/src/interpret.rs` `check_json_finite` /
//!   `check_json_finite_visited` — track-then-force-then-recurse
//!   pattern with per-call `Vec<Rc<RefCell<Option<Value>>>>`
//!   path stack. Encountering a `Thunk` whose cache `Rc`
//!   matches one already on the path errors with
//!   "infinite recursion encountered (cyclic value)".
//! - `crates/pnix-eval/src/interpret.rs` `deep_force_value` /
//!   `deep_force_value_at_depth` — same pattern at the
//!   `deepSeq` entry, additionally retains the existing
//!   `DEEP_FORCE_MAX_DEPTH = 1_000` depth limit for non-cyclic
//!   pathological deep nesting.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}
fn err_msg(src: &str) -> String {
  format!("{}", eval_expr(src).expect_err(src))
}

// ── builtins.toJSON cyclic ─────────────────────────────────────

#[test]
fn to_json_self_cycle_attrset_errors() {
  let m = err_msg("let r = { a = r; }; in builtins.toJSON r");
  assert!(m.contains("toJSON"), "got: {m}");
  assert!(m.contains("infinite recursion"), "got: {m}");
}

#[test]
fn to_json_mutual_cycle_attrsets_errors() {
  // a → b → a cycle through the let-binding cache.
  let m = err_msg(
    r#"
    let
      a = { x = b; };
      b = { y = a; };
    in builtins.toJSON a
  "#,
  );
  assert!(m.contains("infinite recursion"), "got: {m}");
}

#[test]
fn to_json_cycle_via_list_errors() {
  let m = err_msg("let r = [ r ]; in builtins.toJSON r");
  assert!(m.contains("infinite recursion"), "got: {m}");
}

#[test]
fn to_json_acyclic_shared_thunk_works() {
  // The same thunk reached via TWO independent paths must not
  // false-positive as a cycle. (DAG, not a cycle.) `path.pop()`
  // on exit ensures the visited stack is per-descent, not
  // accumulated.
  assert_eq!(
    json(r#"let x = 99; in builtins.toJSON { a = x; b = x; }"#),
    r#""{\"a\":99,\"b\":99}""#
  );
}

#[test]
fn to_json_concrete_attrset_works() {
  // Sanity: non-cyclic attrset still serialises.
  assert_eq!(
    json(r#"builtins.toJSON { a = 1; b = [ 2 3 ]; c = { d = "x"; }; }"#),
    r#""{\"a\":1,\"b\":[2,3],\"c\":{\"d\":\"x\"}}""#
  );
}

// ── builtins.deepSeq cyclic ────────────────────────────────────

#[test]
fn deep_seq_self_cycle_attrset_errors() {
  let m = err_msg("let r = { a = r; }; in builtins.deepSeq r 99");
  assert!(m.contains("deepSeq"), "got: {m}");
  assert!(m.contains("infinite recursion"), "got: {m}");
}

#[test]
fn deep_seq_self_cycle_list_errors() {
  let m = err_msg("let r = [ r ]; in builtins.deepSeq r 99");
  assert!(m.contains("infinite recursion"), "got: {m}");
}

#[test]
fn deep_seq_mutual_cycle_errors() {
  let m = err_msg(
    r#"
    let
      a = [ b ];
      b = [ a ];
    in builtins.deepSeq a 99
  "#,
  );
  assert!(m.contains("infinite recursion"), "got: {m}");
}

#[test]
fn deep_seq_acyclic_shared_thunk_works() {
  // DAG (shared, not cyclic) must complete normally.
  assert_eq!(
    json(r#"let x = 99; in builtins.deepSeq { a = x; b = x; } 7"#),
    "7"
  );
}

#[test]
fn deep_seq_concrete_passes_through() {
  // Sanity: existing slice #34 / earlier behaviour preserved.
  assert_eq!(json("builtins.deepSeq { a = 1; b = 2; } 99"), "99");
}

#[test]
fn deep_seq_inner_throw_still_fires() {
  // Inner throw inside a non-cyclic value still propagates
  // (slice #34 invariant). The cycle guard must not mask
  // legitimate errors during deep-force.
  let m = err_msg(r#"builtins.deepSeq { a = throw "inner"; } 99"#);
  assert!(m.contains("inner"), "got: {m}");
}
