//! Regression cover for `map` / `genList` / `concatLists`
//! laziness — list-producing builtins must not eagerly force
//! their elements.
//!
//! 2026-05-05 audit findings (slice #62):
//!
//!   `builtins.map (x: throw "x") [ 1 2 3 ]` errored
//!   immediately at construction time. The pre-fix impl applied
//!   the function eagerly via `apply_value(f, i)?`, so any
//!   throw in the function body fired before the result was
//!   accessed. Real Nix's `map` is lazy in the result elements
//!   — each output element is a thunk, only forced when
//!   accessed. Pre-fix `length (map throw [1 2 3])` errored
//!   instead of returning 3.
//!
//!   `builtins.genList (i: throw "x") 5` errored similarly —
//!   the impl eagerly applied the function to each index.
//!
//!   `builtins.concatLists [ [1] [(throw "x")] ]` errored
//!   because the apply_builtin boundary deep-forced its
//!   argument. concatLists wasn't in the `lazy_in_elements`
//!   list, so the boundary's `else` branch deep-forced every
//!   inner thunk. `head (concatLists [...])` should return 1
//!   without forcing the throw.
//!
//!   Production-relevant: every lazy pattern that maps /
//!   genLists / concatLists with potentially-failing or
//!   expensive elements and then takes a partial slice
//!   (`length`, `head`, `elemAt 0`) was broken. Authors who
//!   wrote `let xs = map f bigList; in if needed then
//!   xs else fallback` ALWAYS forced all of `bigList` even
//!   if `xs` was never used.
//!
//! Truth-owner files:
//! - `crates/pnix-eval/src/interpret.rs` `apply_builtin`
//!   `lazy_in_elements` set — added `"concatLists"` and
//!   `"genList"` so the apply boundary uses the shallow-force
//!   path instead of `deep_force`.
//! - `crates/pnix-eval/src/interpret.rs` `apply_builtin`
//!   `"map"` arm — replaced eager `apply_value` with
//!   `deferred_apply` (new helper that mirrors
//!   `deferred_apply2` for one-argument lambdas).
//! - `crates/pnix-eval/src/interpret.rs` `apply_builtin`
//!   `"genList"` arm — same `deferred_apply` rewrite for the
//!   `f i` application loop.
//! - `crates/pnix-eval/src/interpret.rs` `deferred_apply`
//!   helper — new function constructing a `Value::Thunk` for
//!   `f x` without forcing.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

fn err(src: &str) -> String {
  eval_expr(src).err().expect(src).to_string()
}

// ── map: lazy in elements ─────────────────────────────────────

#[test]
fn map_length_does_not_force_elements() {
  // Pre-fix: errored. Now: returns 3.
  assert_eq!(
    json(r#"builtins.length (builtins.map (x: throw "x") [ 1 2 3 ])"#),
    "3"
  );
}

#[test]
fn map_head_only_forces_first() {
  assert_eq!(
    json(r#"builtins.head (builtins.map (x: x + 1) [ 1 (throw "x") 3 ])"#),
    "2"
  );
}

#[test]
fn map_elemAt_only_forces_indexed() {
  // Index 0 → only first element forced.
  assert_eq!(
    json(r#"builtins.elemAt (builtins.map (x: x * 10) [ 1 (throw "x") ]) 0"#),
    "10"
  );
}

#[test]
fn map_force_throws_when_accessed() {
  // Sanity: forcing the throwing element does fire.
  let e = err(r#"builtins.elemAt (builtins.map (x: throw "x") [ 1 2 3 ]) 1"#);
  assert!(e.contains("x"), "got: {e}");
}

#[test]
fn map_happy_path_unchanged() {
  // Sanity: basic functionality intact.
  assert_eq!(json(r#"builtins.map (x: x + 1) [ 1 2 3 ]"#), "[2,3,4]");
}

// ── genList: lazy in elements ─────────────────────────────────

#[test]
fn gen_list_length_does_not_force_function() {
  // Pre-fix: errored. Now: returns 5.
  assert_eq!(
    json(r#"builtins.length (builtins.genList (i: throw "x") 5)"#),
    "5"
  );
}

#[test]
fn gen_list_head_only_forces_index_zero() {
  assert_eq!(
    json(r#"builtins.head (builtins.genList (i: if i == 0 then 99 else throw "x") 10)"#),
    "99"
  );
}

#[test]
fn gen_list_elemAt_only_forces_indexed() {
  assert_eq!(
    json(r#"builtins.elemAt (builtins.genList (i: if i == 2 then 7 else throw "x") 10) 2"#),
    "7"
  );
}

#[test]
fn gen_list_force_throws_when_accessed() {
  let e = err(r#"builtins.elemAt (builtins.genList (i: throw "x") 5) 0"#);
  assert!(e.contains("x"), "got: {e}");
}

#[test]
fn gen_list_zero_count_returns_empty() {
  // Sanity.
  assert_eq!(json(r#"builtins.genList (i: i) 0"#), "[]");
}

#[test]
fn gen_list_happy_path_unchanged() {
  assert_eq!(json(r#"builtins.genList (i: i * 2) 4"#), "[0,2,4,6]");
}

// ── concatLists: lazy in inner-list elements ──────────────────

#[test]
fn concat_lists_length_does_not_force_inner() {
  // Pre-fix: errored. Now: returns 2.
  assert_eq!(
    json(r#"builtins.length (builtins.concatLists [ [ 1 ] [ (throw "x") ] ])"#),
    "2"
  );
}

#[test]
fn concat_lists_head_returns_first_without_forcing_throw() {
  assert_eq!(
    json(r#"builtins.head (builtins.concatLists [ [ 1 ] [ (throw "x") ] ])"#),
    "1"
  );
}

#[test]
fn concat_lists_elemAt_only_forces_indexed() {
  assert_eq!(
    json(r#"builtins.elemAt (builtins.concatLists [ [ 1 2 ] [ 3 (throw "x") ] ]) 2"#),
    "3"
  );
}

#[test]
fn concat_lists_force_throws_when_accessed() {
  let e = err(r#"builtins.elemAt (builtins.concatLists [ [ 1 ] [ (throw "x") ] ]) 1"#);
  assert!(e.contains("x"), "got: {e}");
}

#[test]
fn concat_lists_happy_path_unchanged() {
  assert_eq!(
    json(r#"builtins.concatLists [ [ 1 ] [ 2 3 ] [ 4 ] ]"#),
    "[1,2,3,4]"
  );
}

#[test]
fn concat_lists_outer_must_be_list_unchanged() {
  // Sanity: outer-arg type guard still works.
  let e = err(r#"builtins.concatLists 42"#);
  assert!(e.contains("concatLists"), "got: {e}");
  assert!(e.contains("must be list"), "got: {e}");
}

#[test]
fn concat_lists_inner_must_be_list_unchanged() {
  // Sanity: inner-element type guard still works.
  let e = err(r#"builtins.concatLists [ [ 1 ] 42 ]"#);
  assert!(e.contains("concatLists"), "got: {e}");
  assert!(e.contains("not a list"), "got: {e}");
}

// ── parity with mapAttrs (already lazy per slice family) ──────

#[test]
fn mapAttrs_length_lazy_unchanged() {
  // Sanity: mapAttrs was already lazy; this slice didn't break
  // it.
  assert_eq!(
    json(
      r#"
      builtins.length (
        builtins.attrNames (
          builtins.mapAttrs (k: v: throw "x") { a = 1; b = 2; }
        )
      )
    "#
    ),
    "2"
  );
}

// ── slice #36 / #34 family: predicates still strict ───────────

#[test]
fn filter_predicate_still_forces_each() {
  // filter MUST force the predicate result for each element to
  // decide whether to keep. So a throw in the predicate fires.
  let e = err(r#"builtins.filter (x: x > 0) [ 1 (throw "x") 3 ]"#);
  assert!(e.contains("x"), "got: {e}");
}

#[test]
fn all_predicate_still_strict_in_used_args() {
  // all forces the predicate. Sanity.
  assert_eq!(json(r#"builtins.all (x: x > 0) [ 1 2 3 ]"#), "true");
}

// ── lazy patterns in let-bindings ─────────────────────────────

#[test]
fn unused_map_in_let_does_not_throw() {
  assert_eq!(
    json(
      r#"
      let
        xs = builtins.map (x: throw "x") [ 1 2 3 ];
        in 99
    "#
    ),
    "99"
  );
}

#[test]
fn unused_genList_in_let_does_not_throw() {
  assert_eq!(
    json(
      r#"
      let
        xs = builtins.genList (i: throw "x") 100;
      in 99
    "#
    ),
    "99"
  );
}
