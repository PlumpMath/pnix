//! Regression cover for `builtins.tryEval` error-path coverage.
//!
//! Nix-correct shape (per the manual):
//!   `tryEval e` returns
//!     `{ success = true; value = e; }` on success,
//!     `{ success = false; value = false; }` on any thrown error.
//!
//! The previous implementation returned `value = null` on failure,
//! which silently bypassed every nixpkgs `value == false` guard.
//! This file pins both the shape and the coverage of error kinds
//! that must round-trip through `tryEval` as `success = false`.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

const FAIL: &str = r#"{"success":false,"value":false}"#;

#[test]
fn tryeval_value_on_success() {
  assert_eq!(
    json(r#"builtins.tryEval 42"#),
    r#"{"success":true,"value":42}"#
  );
  assert_eq!(
    json(r#"builtins.tryEval "hi""#),
    r#"{"success":true,"value":"hi"}"#
  );
  let v = eval_expr(r#"(builtins.tryEval [ 1 2 ]).value"#).unwrap();
  assert_eq!(v.to_json(), "[1,2]");
}

#[test]
fn tryeval_catches_throw() {
  assert_eq!(json(r#"builtins.tryEval (throw "boom")"#), FAIL);
}

#[test]
fn tryeval_catches_assertion_failure() {
  assert_eq!(json(r#"builtins.tryEval (assert false; 42)"#), FAIL);
}

#[test]
fn tryeval_catches_division_by_zero() {
  assert_eq!(json(r#"builtins.tryEval (1 / 0)"#), FAIL);
}

#[test]
fn tryeval_catches_modulo_by_zero() {
  // The `%` arm landed in 95e15a7e + tests in eval_modulo_op.rs.
  // Ensure tryEval treats modulo-by-zero the same as division.
  assert_eq!(json(r#"builtins.tryEval (1 % 0)"#), FAIL);
}

#[test]
fn tryeval_catches_undefined_variable() {
  assert_eq!(json(r#"builtins.tryEval undefined_var"#), FAIL);
}

#[test]
fn tryeval_catches_attr_not_found() {
  assert_eq!(json(r#"builtins.tryEval ({}.missing)"#), FAIL);
}

#[test]
fn tryeval_catches_type_error() {
  // `1 + "x"` is a type error; tryEval must catch it.
  assert_eq!(json(r#"builtins.tryEval (1 + "x")"#), FAIL);
}

#[test]
fn tryeval_catches_infinite_recursion() {
  // The just-landed force_value cycle guard surfaces "infinite
  // recursion encountered" as a regular Result::Err — tryEval
  // catches it like any other failure.
  assert_eq!(
    json(r#"builtins.tryEval (let s = { x = s.x; }; in s.x)"#),
    FAIL
  );
}

#[test]
fn tryeval_value_false_lets_nixpkgs_pattern_work() {
  // The classic `value == false` guard pattern that nixpkgs uses
  // after a tryEval. Was broken when tryEval returned null on
  // failure (because `null == false` is `false` in Nix).
  assert_eq!(
    json(
      r#"
      let r = builtins.tryEval (throw "boom");
      in if r.success then "ok" else "caught"
    "#
    ),
    r#""caught""#
  );
}

#[test]
fn tryeval_does_not_swallow_nested_value_throw() {
  // The outer tryEval succeeds (just builds a list), but the
  // inner element is lazy so its throw doesn't fire here.
  // Forcing the element later would throw — tryEval doesn't
  // deep-force.
  let v = eval_expr(r#"(builtins.tryEval [ (throw "side") 1 ]).success"#).unwrap();
  assert_eq!(v.to_json(), "true");
}
