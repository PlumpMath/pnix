//! Regression cover for Nix-correct `with`-vs-lexical lookup priority.
//!
//! The previous evaluator implemented `with` by `bind`-injecting the
//! attrset into a fresh local env, which made `with` shadow any
//! enclosing `let` binding. Real Nix says lexical bindings (let /
//! lambda parameter / rec) always win over `with`, and inner `with`
//! wins over outer `with`. The fix introduces a `with_chain` on
//! `Env` that lookup consults only after every lexical scope has
//! failed.
//!
//! Reference: NixOS Nix manual §"with-expression":
//!   "The with-expression is not strictly equivalent to a let-binding;
//!    the variables in the with do not shadow lexical bindings."

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

#[test]
fn enclosing_let_overrides_with() {
  // `let x = 1; in with { x = 2; }; x` → 1
  assert_eq!(json("let x = 1; in with { x = 2; }; x"), "1");
}

#[test]
fn enclosing_let_overrides_inner_with_too() {
  assert_eq!(
    json("let x = 1; in with { x = 2; }; with { x = 3; }; x"),
    "1"
  );
}

#[test]
fn lambda_param_overrides_with() {
  assert_eq!(json("with { x = 2; }; (x: x) 7"), "7");
}

#[test]
fn rec_attr_overrides_with() {
  // `rec { x = 1; y = x; }` — y looks up `x` and should see the rec
  // binding 1, not the with-supplied 2.
  let v = json("with { x = 2; }; let s = rec { x = 1; y = x; }; in s.y");
  assert_eq!(v, "1");
}

#[test]
fn with_provides_when_lexical_absent() {
  assert_eq!(json("with { x = 2; }; x"), "2");
}

#[test]
fn nested_with_inner_wins() {
  assert_eq!(json("with { x = 1; }; with { x = 2; }; x"), "2");
}

#[test]
fn nested_with_outer_visible_when_inner_misses() {
  assert_eq!(json("with { a = 1; }; with { b = 2; }; a"), "1");
}

#[test]
fn with_chain_walks_inner_first_for_each_name() {
  // `a` shadowed by inner; `b` only in outer.
  let v = json("with { a = 1; b = 2; }; with { a = 99; }; [ a b ]");
  assert_eq!(v, "[99,2]");
}

#[test]
fn let_binding_inside_with_body_overrides_with() {
  // `with X; let x = …; in x` — `let` is inside the `with` body but
  // it is still lexically inner so it wins.
  assert_eq!(json("with { x = 2; }; let x = 1; in x"), "1");
}

#[test]
fn deeply_nested_let_with_let_shadowing() {
  // Sanity check: the lexical `let` chain remains correct after the
  // refactor. Three nested lets, innermost wins.
  let src = "let x = 1; in let x = 2; in let x = 3; in x";
  assert_eq!(json(src), "3");
}

#[test]
fn with_does_not_leak_into_caller_scope() {
  // `with` body's attrs must not leak into the caller. Here the inner
  // function's `with { x = 99; }; x` looks up `x`: the lambda's own
  // scope has only `a`, but the enclosing let already binds
  // `x = 42` lexically, so Nix-correct behaviour is to find that
  // `42` first (lexical wins) and never reach the `with` frame.
  // The caller-side `x` is the same lexical 42. Both elements are 42
  // — that is the proof that `with` did not redefine `x` in the
  // surrounding lexical scope.
  let v = json(
    "let f = a: with { x = 99; }; x;
         x = 42; in
       [ (f 0) x ]",
  );
  assert_eq!(v, "[42,42]");
}

#[test]
fn with_supplies_x_only_when_no_lexical_x_in_chain() {
  // Same shape as the previous test but no `x` in the let — now
  // `with` is the only source for `x`, so the function returns 99.
  let v = json(
    "let f = a: with { x = 99; }; x; in
       f 0",
  );
  assert_eq!(v, "99");
}
