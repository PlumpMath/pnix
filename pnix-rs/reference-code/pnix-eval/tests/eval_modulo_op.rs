//! Modulo operator (`%`) regression cover. Lexer/parser already accept
//! `%` (see `Token::Op("%")` in the lexer and the multiplication-class
//! arm in the parser); this covers the evaluator side that was
//! previously missing — `10 % 3` used to error with
//! `operator %: unsupported operand types int and int`.

use pnix_eval::eval_expr;

fn json(src: &str) -> String {
  eval_expr(src).expect(src).to_json()
}

#[test]
fn int_modulo_int() {
  assert_eq!(json("10 % 3"), "1");
  assert_eq!(json("10 % 5"), "0");
}

#[test]
fn negative_int_modulo_follows_truncated_remainder() {
  // Rust's `i64::checked_rem` follows truncated-toward-zero remainder,
  // matching the rest of `pnix-eval`'s integer arithmetic and Nix's
  // `builtins.mod` (which delegates to the same operation).
  assert_eq!(json("(-10) % 3"), "-1");
  assert_eq!(json("10 % (-3)"), "1");
}

#[test]
fn modulo_by_zero_errors() {
  let err = eval_expr("10 % 0").expect_err("expected modulo-by-zero error");
  let msg = err.to_string();
  assert!(
    msg.contains("modulo by zero"),
    "expected `modulo by zero`, got: {msg}"
  );
}

#[test]
fn float_modulo_int_promotes_to_float() {
  let v = eval_expr("10.5 % 3").expect("eval ok");
  // `10.5 % 3` = `1.5` under IEEE 754 / C `fmod` semantics.
  assert_eq!(v.to_json(), "1.5");
}

#[test]
fn modulo_inside_parens_in_list() {
  // Same precedence class as `*` and `/`. Sanity-check it composes.
  let v = eval_expr("[ (10 % 3) (5 % 2) (7 % 4) ]").expect("eval ok");
  assert_eq!(v.to_json(), "[1,1,3]");
}

#[test]
fn modulo_matches_builtins_mod() {
  // Both surfaces should agree on the same operation.
  assert_eq!(json("17 % 5"), json("builtins.mod 17 5"));
  assert_eq!(json("100 % 7"), json("builtins.mod 100 7"));
}
