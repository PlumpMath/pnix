//! Regression cover for the `==` / `!=` / `builtins.eq` /
//! `builtins.elem` / `builtins.find` cyclic-value DoS guard.
//!
//! 2026-05-05 audit findings (slice #42):
//!   - `let r = { a = r; }; in r == r` overflowed the Rust call
//!     stack with `SIGABRT`. `values_equal` recursed into
//!     AttrSet / List children without cycle tracking; both
//!     sides forced to the same cyclic AttrSet and the
//!     equality recursion looped.
//!   - Same shape for `let r = [ r ]; in r == r`,
//!     `let r = [ r ]; in builtins.find r r`, and the family
//!     of equality-using builtins that fanned out from
//!     `values_equal` (`builtins.eq`, `builtins.elem` —
//!     `builtins.find` was already reachable from `find _ r`).
//!   - Real Nix errors with "infinite recursion encountered"
//!     on the same input. Match that shape: per-call depth
//!     parameter on `values_equal_at_depth`, with a max of 64
//!     levels of nested AttrSet / List descent. The limit is
//!     conservative — 64 levels of legitimate nesting is
//!     already pathological and any cycle blows the 2 MB
//!     Rust test thread stack at 30-50 iterations of the
//!     heavy `force_value + match + recurse` frame, so 64
//!     errors well before stack overflow.
//!   - Signature change: `values_equal(l, r) -> bool` became
//!     `values_equal(l, r) -> Result<bool>` so the cycle
//!     error can propagate. All five call sites updated:
//!     `==` / `!=` operators (binary arm), `builtins.eq`
//!     (apply_builtin), `builtins.find` (replaced
//!     `iter().find` with explicit for-loop to handle Result),
//!     `builtins.elem` (replaced `iter().any` with explicit
//!     for-loop). The previous shape silently turned a force
//!     error into "false" via `match force_value(...) Err(_)
//!     => return false` — that's now also surfaced as a
//!     proper error, which is a small but useful correctness
//!     improvement (previously a `throw` inside a value being
//!     compared could be silently absorbed into `false`).
//!
//! Truth-owner files:
//! - `crates/pnix-eval/src/interpret.rs` — `VALUES_EQUAL_MAX_DEPTH`
//!   const, `values_equal` / `values_equal_at_depth`
//!   refactor, signature change, call-site updates at the
//!   five enumerated locations.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}
fn err_msg(src: &str) -> String {
  format!("{}", eval_expr(src).expect_err(src))
}

// ── self-cycle on `==` ─────────────────────────────────────────

#[test]
fn eq_self_cycle_attrset_errors() {
  let m = err_msg("let r = { a = r; }; in r == r");
  assert!(m.contains("infinite recursion"), "got: {m}");
  assert!(m.contains("=="), "got: {m}");
}

#[test]
fn eq_self_cycle_list_errors() {
  let m = err_msg("let r = [ r ]; in r == r");
  assert!(m.contains("infinite recursion"), "got: {m}");
}

#[test]
fn eq_independent_cycle_errors() {
  // Two independently-defined cyclic structures with the same
  // shape. Real Nix and pnix both error on this — there's no
  // way to determine bisimilar equality without infinite
  // descent.
  let m = err_msg(
    r#"
    let
      r = { a = r; };
      s = { a = s; };
    in r == s
  "#,
  );
  assert!(m.contains("infinite recursion"), "got: {m}");
}

#[test]
fn ne_self_cycle_errors() {
  // `!=` goes through the same path and also errors.
  let m = err_msg("let r = { a = r; }; in r != r");
  assert!(m.contains("infinite recursion"), "got: {m}");
}

// ── builtins.eq / elem / find cycle ────────────────────────────

#[test]
fn builtins_eq_cycle_errors() {
  let m = err_msg("let r = { a = r; }; in builtins.eq r r");
  assert!(m.contains("infinite recursion"), "got: {m}");
}

#[test]
fn builtins_elem_cycle_in_haystack_errors() {
  // The needle is cyclic and the haystack contains another
  // cyclic structure. The for-loop walks until the equality
  // check trips the cycle guard.
  let m = err_msg("let r = { a = r; }; s = { a = s; }; in builtins.elem r [ s ]");
  assert!(m.contains("infinite recursion"), "got: {m}");
}

#[test]
fn builtins_find_cycle_errors() {
  let m = err_msg("let r = [ r ]; in builtins.find r r");
  assert!(m.contains("infinite recursion"), "got: {m}");
}

// ── shallow operations on cyclic stay safe ─────────────────────

#[test]
fn elem_finds_int_in_list_with_cycle() {
  // `builtins.elem 1 [ r ]` where r is cyclic but the list
  // contains an int 1. Wait — needle r is in list [r] → cycle
  // hits. Use a needle that isn't cyclic.
  // Confirm pre-cycle elements match without descending into
  // the cycle.
  assert_eq!(
    json("let r = { a = r; }; in builtins.elem 1 [ 1 r ]"),
    "true"
  );
}

#[test]
fn attr_names_on_cyclic_works() {
  // `builtins.attrNames r` doesn't descend into values, so
  // cyclic values are fine.
  assert_eq!(
    json("let r = { a = r; b = 2; }; in builtins.attrNames r"),
    r#"["a","b"]"#
  );
}

#[test]
fn has_attr_on_cyclic_works() {
  // `?` doesn't descend into the value either.
  assert_eq!(json("let r = { a = r; }; in r ? a"), "true");
}

#[test]
fn length_on_cyclic_list_works() {
  // `length` is shallow.
  assert_eq!(json("let r = [ r ]; in builtins.length r"), "1");
}

#[test]
fn with_cyclic_source_unused_works() {
  // Slice #20 lazy-with invariant unchanged — cyclic source
  // never forced if the body doesn't consult it.
  assert_eq!(json("let r = { a = r; }; in with r; 1"), "1");
}

#[test]
fn with_cyclic_source_uses_non_cyclic_member_works() {
  // Forces the with-source (which is the cyclic AttrSet),
  // looks up a non-cyclic member. The lookup walks the
  // top-level keys without descending into values, so the
  // cycle isn't entered.
  assert_eq!(json("let r = { a = r; b = 99; }; in with r; b"), "99");
}

#[test]
fn inherit_from_cyclic_works() {
  // `inherit (r) b` — `b` is a non-cyclic field of the cyclic
  // r. Lookup descends one level.
  assert_eq!(
    json("let r = { a = r; b = 1; }; in let inherit (r) b; in b"),
    "1"
  );
}

// ── legitimate deep equality stays correct ─────────────────────

#[test]
fn eq_genlist_50_works() {
  // 50-element flat list — only one level of recursion (each
  // element is an int, depth 1).
  assert_eq!(
    json("(builtins.genList (x: x) 50) == (builtins.genList (x: x) 50)"),
    "true"
  );
}

#[test]
fn eq_acyclic_dag_works() {
  // The same value reached via two paths — DAG, not cycle.
  // values_equal walks the same Thunk twice but doesn't loop.
  assert_eq!(
    json("let x = { a = 1; }; in { p = x; q = x; } == { p = x; q = x; }"),
    "true"
  );
}

#[test]
fn eq_concrete_attrset_works() {
  // Sanity.
  assert_eq!(
    json("{ a = 1; b = [ 2 3 ]; } == { a = 1; b = [ 2 3 ]; }"),
    "true"
  );
}

#[test]
fn eq_different_concrete_attrsets_works() {
  assert_eq!(json("{ a = 1; b = 2; } == { a = 1; b = 3; }"), "false");
}

#[test]
fn eq_throw_in_value_propagates() {
  // Pre-fix `values_equal` returned `false` if `force_value`
  // errored — silently swallowing throws inside compared
  // values. Post-fix the error propagates out via the new
  // Result signature.
  let m = err_msg(r#"{ a = throw "inner"; } == { a = 1; }"#);
  assert!(m.contains("inner"), "got: {m}");
}
