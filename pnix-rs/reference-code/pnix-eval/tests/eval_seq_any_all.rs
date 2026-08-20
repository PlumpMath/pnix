//! Regression cover for `builtins.seq` shallow-force semantics
//! and `builtins.any` / `builtins.all` non-bool predicate guards.
//!
//! 2026-05-04 audit findings:
//!   - **`builtins.seq` was deep-forcing its first argument**:
//!     pnix-eval registered `seq` under the default arg-force
//!     policy (full `deep_force`), so `seq { a = throw "e"; } 1`
//!     fired the `throw` and errored out instead of evaluating
//!     to `1`. Nix's `prim_seq` calls `forceValue` (shallow,
//!     thunk-only — does not recurse into list elements or
//!     attrset values), and the shallow / lazy contract is the
//!     whole point of `seq`. Code that uses `seq` to "stage" an
//!     attrset's evaluation without touching its inner thunks
//!     was silently broken under the old policy. Moved `seq` to
//!     the `lazy_in_elements` group so the boundary uses
//!     `force_value` (shallow). `deepSeq` is unchanged — it
//!     keeps the deep-force contract by name.
//!   - **`builtins.any` / `builtins.all` truthy-coerced non-bool
//!     predicate**: `any (x: x) [1]` returned `true`, `all (x: 42)
//!     [1]` returned `true`. `partition` and `filterAttrs`
//!     already errored on non-bool predicates — `any` and `all`
//!     were the inconsistent two. Now both error with
//!     `predicate must return bool, got <type> at index N`,
//!     matching `filter`'s shape from slice #33. Non-list second
//!     argument also moved from silent-`false` to a typed error.
//!
//! Truth-owner files:
//! - `crates/pnix-eval/src/interpret.rs` — `lazy_in_elements`
//!   builtin list (added `seq`); `apply_builtin` arms for
//!   `"any"` and `"all"`.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}
fn err_msg(src: &str) -> String {
  format!("{}", eval_expr(src).expect_err(src))
}

// ── builtins.seq shallow-force semantics ──────────────────────

#[test]
fn seq_passes_through_when_first_arg_is_concrete() {
  assert_eq!(json("builtins.seq 1 2"), "2");
  assert_eq!(json("builtins.seq null 2"), "2");
  assert_eq!(json("builtins.seq [ 1 2 3 ] 99"), "99");
}

#[test]
fn seq_top_level_throw_propagates() {
  // The top-level thunk is shallow-forced, so a top-level
  // `throw` (i.e. the entire first arg is the `throw`) must fire.
  let m = err_msg(r#"builtins.seq (throw "top") 1"#);
  assert!(m.contains("top"), "got: {m}");
}

#[test]
fn seq_attrset_with_throw_value_does_not_force_inner() {
  // Was: deep-forced and threw "inner". Nix-correct: 1.
  // This is the load-bearing case that proves `seq` is shallow:
  // staging an attrset for evaluation doesn't fire its inner
  // thunks.
  assert_eq!(json(r#"builtins.seq { a = throw "inner"; } 1"#), "1");
}

#[test]
fn seq_list_with_throw_element_does_not_force_inner() {
  // Same shape for lists.
  assert_eq!(json(r#"builtins.seq [ (throw "inner") ] 1"#), "1");
}

#[test]
fn seq_nested_attrset_only_shallow_forces_outer() {
  // The throw is nested two levels deep; shallow force on the
  // outer attrset must not cascade.
  assert_eq!(
    json(r#"builtins.seq { a = { b = throw "deep"; }; } 1"#),
    "1"
  );
}

#[test]
fn seq_via_let_binding_still_shallow() {
  // The first arg is a thunk that resolves to an attrset with a
  // `throw` value. Resolution gives the attrset (shallow), the
  // throw stays unforced.
  assert_eq!(
    json(r#"let x = { a = throw "x"; }; in builtins.seq x 1"#),
    "1"
  );
}

#[test]
fn trace_family_preserves_lazy_return_payloads_under_seq() {
  // `trace`/`warn`/`traceVerbose` return their second argument.
  // They must not deep-force that payload just to print/validate the
  // message. `seq` shallow-forces the returned attrset and should not
  // touch its inner throwing field.
  assert_eq!(
    json(r#"builtins.seq (builtins.trace "hello" { a = throw "trace payload"; }) 1"#),
    "1"
  );
  assert_eq!(
    json(r#"builtins.seq (builtins.warn "hello" { a = throw "warn payload"; }) 1"#),
    "1"
  );
  assert_eq!(
    json(r#"builtins.seq (builtins.traceVerbose "hello" { a = throw "verbose payload"; }) 1"#),
    "1"
  );
}

#[test]
fn trace_family_still_forces_message_arguments() {
  let m = err_msg(r#"builtins.trace (throw "trace message") 1"#);
  assert!(m.contains("trace message"), "got: {m}");

  let m = err_msg(r#"builtins.warn (throw "warn message") 1"#);
  assert!(m.contains("warn message"), "got: {m}");

  let m = err_msg(r#"builtins.warn 42 1"#);
  assert!(m.contains("expected string message"), "got: {m}");
}

#[test]
fn seq_second_arg_throw_propagates() {
  // Second arg is shallow-forced too, so a top-level `throw`
  // there fires.
  let m = err_msg(r#"builtins.seq 1 (throw "second")"#);
  assert!(m.contains("second"), "got: {m}");
}

#[test]
fn deep_seq_still_forces_inner() {
  // Sanity: `deepSeq` is untouched and still deep-forces.
  let m = err_msg(r#"builtins.deepSeq { a = throw "inner"; } 1"#);
  assert!(m.contains("inner"), "got: {m}");
  let m = err_msg(r#"builtins.deepSeq [ (throw "inner") ] 1"#);
  assert!(m.contains("inner"), "got: {m}");
  let m = err_msg(r#"builtins.deepSeq { a = { b = throw "deep"; }; } 1"#);
  assert!(m.contains("deep"), "got: {m}");
}

#[test]
fn deep_seq_concrete_passes_through() {
  assert_eq!(json("builtins.deepSeq null 1"), "1");
  assert_eq!(json("builtins.deepSeq 42 1"), "1");
  assert_eq!(json(r#"builtins.deepSeq { a = 1; b = 2; } 99"#), "99");
}

// ── builtins.any non-bool predicate guard ─────────────────────

#[test]
fn any_bool_predicate_works() {
  assert_eq!(json("builtins.any (x: x > 0) [ 1 2 3 ]"), "true");
  assert_eq!(json("builtins.any (x: x > 5) [ 1 2 3 ]"), "false");
  assert_eq!(json("builtins.any (x: true) []"), "false");
  assert_eq!(json("builtins.any (x: x > 0) []"), "false");
}

#[test]
fn any_int_predicate_errors() {
  // Was: returned true silently.
  let m = err_msg("builtins.any (x: x) [ 1 ]");
  assert!(m.contains("predicate must return bool"), "got: {m}");
  assert!(m.contains("got int"), "got: {m}");
  assert!(m.contains("at index 0"), "got: {m}");
}

#[test]
fn any_constant_int_predicate_errors() {
  let m = err_msg("builtins.any (x: 42) [ 1 ]");
  assert!(m.contains("predicate must return bool"), "got: {m}");
  assert!(m.contains("got int"), "got: {m}");
}

#[test]
fn any_predicate_index_pinned_to_first_offender() {
  // Bool false then non-bool int → error at index 1.
  let m = err_msg("builtins.any (x: if x == 1 then false else x) [ 1 2 ]");
  assert!(m.contains("predicate must return bool"), "got: {m}");
  assert!(m.contains("at index 1"), "got: {m}");
}

#[test]
fn any_non_list_second_arg_errors() {
  let m = err_msg("builtins.any (x: x > 0) 42");
  assert!(m.contains("second argument must be list"), "got: {m}");
  assert!(m.contains("got int"), "got: {m}");
}

#[test]
fn any_short_circuits_on_first_true() {
  // First true short-circuits, so a later throw is never reached.
  assert_eq!(
    json(r#"builtins.any (x: x > 0) [ 1 (throw "later") ]"#),
    "true"
  );
}

// ── builtins.all non-bool predicate guard ─────────────────────

#[test]
fn all_bool_predicate_works() {
  assert_eq!(json("builtins.all (x: x > 0) [ 1 2 3 ]"), "true");
  assert_eq!(json("builtins.all (x: x > 1) [ 1 2 3 ]"), "false");
  assert_eq!(json("builtins.all (x: false) []"), "true");
  assert_eq!(json("builtins.all (x: x > 0) []"), "true");
}

#[test]
fn all_int_predicate_errors() {
  // Was: returned true silently.
  let m = err_msg("builtins.all (x: 42) [ 1 ]");
  assert!(m.contains("predicate must return bool"), "got: {m}");
  assert!(m.contains("got int"), "got: {m}");
  assert!(m.contains("at index 0"), "got: {m}");
}

#[test]
fn all_predicate_index_pinned_to_first_offender() {
  // Bool true then non-bool int → error at index 1.
  let m = err_msg("builtins.all (x: if x == 1 then true else x) [ 1 2 ]");
  assert!(m.contains("predicate must return bool"), "got: {m}");
  assert!(m.contains("at index 1"), "got: {m}");
}

#[test]
fn all_non_list_second_arg_errors() {
  let m = err_msg("builtins.all (x: x > 0) 42");
  assert!(m.contains("second argument must be list"), "got: {m}");
  assert!(m.contains("got int"), "got: {m}");
}

#[test]
fn all_short_circuits_on_first_false() {
  // First false short-circuits, so a later throw is never reached.
  assert_eq!(
    json(r#"builtins.all (x: x > 0) [ 0 (throw "later") ]"#),
    "false"
  );
}
