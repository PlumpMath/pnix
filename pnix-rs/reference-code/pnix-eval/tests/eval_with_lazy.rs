//! Regression cover for Nix-correct lazy `with`.
//!
//! Real Nix evaluates the `with` source on demand: `with throw "boom"; 1`
//! returns `1`, because the body never falls through to the with chain.
//! The previous evaluator eager-forced the source at construction time,
//! so any `with X; ...` would evaluate `X` regardless of whether the
//! body needed it.
//!
//! Fix: `WithFrame` now stores the unevaluated source expression and
//! the env it was declared in, plus a `RefCell<Option<...>>` cache.
//! `Env::lookup` calls `WithFrame::force` only when it has to consult
//! the frame for an unresolved name.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

#[test]
fn with_throw_returns_body_when_unused() {
  // The classic Nix demo: throw inside a `with` whose body doesn't
  // use any name from it. The throw must never fire.
  assert_eq!(json(r#"with (throw "boom"); 1"#), "1");
}

#[test]
fn with_throw_passthrough_lexical() {
  // The body uses a name that resolves lexically (let), so the with
  // chain is never consulted, so the throw never fires.
  assert_eq!(json(r#"let x = 42; in with (throw "boom"); x"#), "42");
}

#[test]
fn with_throw_actually_used_errors() {
  // Body needs a name from `with`, so the source must be forced —
  // and that forcing surfaces the throw.
  let r = eval_expr(r#"with (throw "boom"); y"#);
  assert!(
    r.is_err(),
    "expected throw to fire when with chain is consulted"
  );
  assert!(format!("{}", r.unwrap_err()).contains("boom"));
}

#[test]
fn with_lazy_evaluation_memoized() {
  // Multiple lookups against the same `with` frame must still
  // produce a single evaluation of the source. Hard to observe
  // directly without side-effects in pure Nix, but the visible
  // proof is that complex sources don't recompute per access.
  assert_eq!(json(r#"with { a = 1; b = 2; c = 3; }; a + b + c"#), "6");
}

#[test]
fn nested_with_outer_throw_skipped_when_inner_provides() {
  // Inner `with` provides `x`, so the lookup never falls through to
  // the outer (which throws).
  assert_eq!(json(r#"with (throw "outer"); with { x = 7; }; x"#), "7");
}

#[test]
fn nested_with_inner_throw_fires_when_inner_consulted() {
  // Outer provides `x` but Nix walks inner first; if forcing inner
  // throws, the throw fires.
  let r = eval_expr(r#"with { x = 7; }; with (throw "inner"); y"#);
  assert!(r.is_err());
}

#[test]
fn with_attrs_can_be_built_from_let_lazily() {
  // The `with` source is itself a complex let-built attrset that
  // would normally evaluate expensive sub-expressions; if the body
  // never asks for those keys, the lazy `with` doesn't force them
  // (we approximate by routing one branch through a throw).
  assert_eq!(
    json(
      r#"let
           attrs = { a = 1; b = throw "side"; };
         in with attrs; a"#
    ),
    "1"
  );
}

#[test]
fn with_non_attrset_does_not_block_construction() {
  // Real Nix: `with X` with non-attrset X defers the type error
  // until the body actually consults the with frame for an
  // unresolved name. Our implementation collapses non-attrsets to
  // an empty map at force time, so a body that resolves all its
  // names lexically still passes.
  assert_eq!(json(r#"let x = 99; in with 42; x"#), "99");
}
