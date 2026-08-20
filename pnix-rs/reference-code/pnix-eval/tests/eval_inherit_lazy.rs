//! Regression cover for Nix-correct lazy `inherit (from)`.
//!
//! `inherit (e) x;` is sugar for `x = e.x;`, and the resulting
//! binding is lazy: if the body never references `x`, neither `e`
//! nor `e.x` should evaluate. The previous implementation called
//! `eval(from_expr)?` at construction time, so `let s = throw "boom";
//! in (let inherit (s) a; in 1)` would surface the throw even
//! though the body never read `a`.
//!
//! Three sites had the same eager bug: the AttrSet `inherit (from)`
//! handler and two `let`-binding paths (the regular `let-in` arm and
//! the trampoline's `Let` short-circuit). All three now wrap each
//! inherited name in a `make_thunk(Select { base: from, attr: name })`
//! so the access is deferred to first force.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

#[test]
fn let_inherit_from_throw_unused() {
  // The classic Nix demo for inherit laziness.
  assert_eq!(
    json(r#"let s = throw "boom"; in (let inherit (s) a; in 1)"#),
    "1"
  );
}

#[test]
fn let_inherit_from_throw_used_fires() {
  let r = eval_expr(r#"let s = throw "boom"; in (let inherit (s) a; in a)"#);
  assert!(r.is_err(), "expected throw to fire when a is forced");
  assert!(format!("{}", r.unwrap_err()).contains("boom"));
}

#[test]
fn attrset_inherit_from_throw_unused() {
  // `{ inherit (s) a; }` should not force `s` if no caller reads `a`.
  // Picking a different attribute via `or default` proves it.
  let v = json(r#"let s = throw "boom"; in ({ inherit (s) a; }).b or 99"#);
  assert_eq!(v, "99");
}

#[test]
fn attrset_inherit_from_throw_used_fires() {
  let r = eval_expr(r#"let s = throw "boom"; in ({ inherit (s) a; }).a"#);
  assert!(r.is_err());
}

#[test]
fn inherit_from_lazy_preserves_field_thunks() {
  // The inherited binding must come from `s` *via* `s.a`, so any
  // sibling field's throw stays lazy.
  let v = json(
    r#"let s = { a = 1; b = throw "side"; };
       in (let inherit (s) a; in a)"#,
  );
  assert_eq!(v, "1");
}

#[test]
fn inherit_multiple_names_are_independent() {
  // `inherit (s) a b;` must build two thunks; touching `a` mustn't
  // force `b` (and vice versa).
  let v = json(
    r#"let s = { a = 1; b = throw "b-side"; };
       in (let inherit (s) a b; in a)"#,
  );
  assert_eq!(v, "1");
}

#[test]
fn inherit_chain_through_let() {
  // `let inherit (s) a; in let inherit a; in a` — both inherit
  // clauses should compose without forcing extra fields.
  let v = json(
    r#"let s = { a = 7; b = throw "side"; };
       in (let inherit (s) a; in (let inherit a; in a))"#,
  );
  assert_eq!(v, "7");
}

#[test]
fn rec_attrset_inherit_from_outer_lazy() {
  // `rec { inherit (s) a; b = a + 1; }` reads `s.a` through the rec
  // scope; non-touched throws stay lazy.
  let v = json(
    r#"let s = { a = 10; quirk = throw "side"; };
       in (rec { inherit (s) a; b = a + 1; }).b"#,
  );
  assert_eq!(v, "11");
}
