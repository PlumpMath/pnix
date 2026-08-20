//! pnix-eval 함수형 / lazy semantics / builtin 합성 회귀 테스트.
//!
//! Currying / partial application, function composition, lazy evaluation
//! (`&&` `||` `->` short-circuit, `let` lazy bindings, AttrSet / List
//! thunk-backed fields), `with` / `inherit`, parameter pattern variants,
//! builtin chain composition, recursive / mutually recursive definitions,
//! `tryEval` / `assert`, dynamic select, has-attr nested 등을 잠근다.

use pnix_eval::{eval_expr, Value};

// ===== 1. CURRYING / PARTIAL APPLICATION =====

#[test]
fn curry_user_lambda() {
  // f = a: b: a + b; f 1 2 == 3
  let v = eval_expr("let f = a: b: a + b; in f 1 2").unwrap();
  assert!(matches!(v, Value::Int(3)), "got {:?}", v);
}

#[test]
fn curry_partial_user_lambda() {
  // (a: b: a + b) 1 → b: 1 + b ; apply 2 → 3
  let v = eval_expr("let f = (a: b: a + b) 1; in f 2").unwrap();
  assert!(matches!(v, Value::Int(3)), "got {:?}", v);
}

#[test]
fn curry_builtin_add_partial() {
  // builtins.add is curried: (builtins.add 1) 2 == 3
  let v = eval_expr("let inc = builtins.add 1; in inc 41").unwrap();
  assert!(matches!(v, Value::Int(42)), "got {:?}", v);
}

#[test]
fn curry_builtin_less_than_partial() {
  let v = eval_expr("let lt5 = builtins.lessThan 5; in lt5 10").unwrap();
  assert!(matches!(v, Value::Bool(true)), "got {:?}", v);
}

#[test]
fn curry_builtin_map_partial() {
  let v = eval_expr("let inc = builtins.add 1; in builtins.map inc [1 2 3]").unwrap();
  if let Value::List(l) = v {
    assert!(matches!(l[0], Value::Int(2)));
    assert!(matches!(l[1], Value::Int(3)));
    assert!(matches!(l[2], Value::Int(4)));
  } else {
    panic!("expected list, got {:?}", v);
  }
}

// ===== 2. FUNCTION COMPOSITION =====

#[test]
fn compose_two_lambdas() {
  // compose = f: g: x: f (g x); compose double inc 5 → double 6 → 12
  let v = eval_expr(
    "let compose = f: g: x: f (g x);
         double = x: x * 2;
         inc = x: x + 1;
     in compose double inc 5",
  )
  .unwrap();
  assert!(matches!(v, Value::Int(12)), "got {:?}", v);
}

#[test]
fn compose_chained_via_let() {
  let v = eval_expr(
    "let inc = x: x + 1;
         double = x: x * 2;
         f = x: double (inc x);
     in f 5",
  )
  .unwrap();
  assert!(matches!(v, Value::Int(12)), "got {:?}", v);
}

#[test]
fn compose_builtin_map_filter_chain() {
  // [1..5] |> filter even |> map double
  let v = eval_expr(
    "let xs = [1 2 3 4 5];
         even = x: builtins.mod x 2 == 0;
         dbl = x: x * 2;
     in builtins.map dbl (builtins.filter even xs)",
  )
  .unwrap();
  if let Value::List(l) = v {
    assert_eq!(l.len(), 2);
    assert!(matches!(l[0], Value::Int(4)));
    assert!(matches!(l[1], Value::Int(8)));
  } else {
    panic!("got {:?}", v);
  }
}

#[test]
fn compose_foldl_sum() {
  // builtins.foldl' (a: b: a + b) 0 [1 2 3 4]
  let v = eval_expr("builtins.foldl' (a: b: a + b) 0 [1 2 3 4]").unwrap();
  assert!(matches!(v, Value::Int(10)), "got {:?}", v);
}

#[test]
fn compose_foldl_with_partial_builtin() {
  let v = eval_expr("builtins.foldl' builtins.add 0 [1 2 3 4 5]").unwrap();
  assert!(matches!(v, Value::Int(15)), "got {:?}", v);
}

// ===== 3. LAZY EVALUATION =====

#[test]
fn lazy_if_does_not_eval_unused_branch() {
  // (1/0) should not be evaluated
  let v = eval_expr("if true then 42 else (1 / 0)").unwrap();
  assert!(matches!(v, Value::Int(42)), "got {:?}", v);
}

#[test]
fn lazy_or_short_circuit() {
  let v = eval_expr("true || (1 / 0 == 0)").unwrap();
  assert!(matches!(v, Value::Bool(true)), "got {:?}", v);
}

#[test]
fn lazy_and_short_circuit() {
  let v = eval_expr("false && (1 / 0 == 0)").unwrap();
  assert!(matches!(v, Value::Bool(false)), "got {:?}", v);
}

#[test]
fn lazy_attrset_unused_field_is_not_evaluated() {
  // unused recursive field with division by zero must not blow up
  let v = eval_expr("let s = { used = 1; broken = 1 / 0; }; in s.used").unwrap();
  assert!(matches!(v, Value::Int(1)), "got {:?}", v);
}

#[test]
fn lazy_let_unused_binding_is_not_evaluated() {
  let v = eval_expr("let unused = 1 / 0; used = 7; in used").unwrap();
  assert!(matches!(v, Value::Int(7)), "got {:?}", v);
}

#[test]
fn lazy_list_element_evaluation_order() {
  // builtins.head only forces the head; tail with broken expr should be OK
  let v = eval_expr("builtins.head [ 1 (1 / 0) (1 / 0) ]").unwrap();
  assert!(matches!(v, Value::Int(1)), "got {:?}", v);
}

// ===== 4. RECURSIVE / SELF-REFERENTIAL =====

#[test]
fn recursive_factorial() {
  let v = eval_expr(
    "let fact = n: if n <= 1 then 1 else n * fact (n - 1);
     in fact 5",
  )
  .unwrap();
  assert!(matches!(v, Value::Int(120)), "got {:?}", v);
}

#[test]
fn mutually_recursive_let() {
  let v = eval_expr(
    "let
       isEven = n: if n == 0 then true else isOdd (n - 1);
       isOdd  = n: if n == 0 then false else isEven (n - 1);
     in isEven 10",
  )
  .unwrap();
  assert!(matches!(v, Value::Bool(true)), "got {:?}", v);
}

#[test]
fn rec_attrset_self_reference() {
  let v = eval_expr("rec { a = 1; b = a + 1; c = b + 1; }.c").unwrap();
  assert!(matches!(v, Value::Int(3)), "got {:?}", v);
}

#[test]
fn rec_attrset_higher_order_self() {
  let v =
    eval_expr("rec { f = n: if n <= 0 then 0 else n + f (n - 1); total = f 4; }.total").unwrap();
  assert!(matches!(v, Value::Int(10)), "got {:?}", v);
}

// ===== 5. WITH / INHERIT =====

#[test]
fn with_brings_attrs_into_scope() {
  let v = eval_expr("with { a = 1; b = 2; }; a + b").unwrap();
  assert!(matches!(v, Value::Int(3)), "got {:?}", v);
}

#[test]
fn nested_with_inner_wins() {
  let v = eval_expr("with { x = 1; }; with { x = 99; }; x").unwrap();
  assert!(matches!(v, Value::Int(99)), "got {:?}", v);
}

#[test]
fn let_overrides_with() {
  // let-binding takes precedence over with
  let v = eval_expr("with { x = 1; }; let x = 99; in x").unwrap();
  assert!(matches!(v, Value::Int(99)), "got {:?}", v);
}

#[test]
fn inherit_from_attrset() {
  let v = eval_expr("let src = { a = 1; b = 2; }; in let inherit (src) a b; in a + b").unwrap();
  assert!(matches!(v, Value::Int(3)), "got {:?}", v);
}

// ===== 6. PARAMETER PATTERNS =====

#[test]
fn attrset_param_pattern() {
  let v = eval_expr("({a, b}: a + b) { a = 1; b = 2; }").unwrap();
  assert!(matches!(v, Value::Int(3)), "got {:?}", v);
}

#[test]
fn attrset_param_with_default() {
  let v = eval_expr("({a, b ? 10}: a + b) { a = 1; }").unwrap();
  assert!(matches!(v, Value::Int(11)), "got {:?}", v);
}

#[test]
fn attrset_param_at_binding() {
  let v = eval_expr("(args@{a, b}: args.a + args.b + a + b) { a = 1; b = 2; }").unwrap();
  assert!(matches!(v, Value::Int(6)), "got {:?}", v);
}

#[test]
fn attrset_param_ellipsis_allows_extras() {
  let v = eval_expr("({a, ...}: a) { a = 1; b = 2; c = 3; }").unwrap();
  assert!(matches!(v, Value::Int(1)), "got {:?}", v);
}

// ===== 7. BUILTIN COMPOSITION =====

#[test]
fn builtin_gen_list_then_filter_then_map() {
  // genList (i: i) 10 → [0..9]; filter > 4; map *2
  let v =
    eval_expr("builtins.map (x: x * 2) (builtins.filter (x: x > 4) (builtins.genList (i: i) 10))")
      .unwrap();
  if let Value::List(l) = v {
    // [5,6,7,8,9] *2 → [10,12,14,16,18]
    assert_eq!(l.len(), 5);
    assert!(matches!(l[0], Value::Int(10)));
    assert!(matches!(l[4], Value::Int(18)));
  } else {
    panic!("got {:?}", v);
  }
}

#[test]
fn builtin_concat_map_works() {
  let v = eval_expr("builtins.concatMap (x: [x x]) [1 2 3]").unwrap();
  if let Value::List(l) = v {
    assert_eq!(l.len(), 6);
    assert!(matches!(l[0], Value::Int(1)));
    assert!(matches!(l[1], Value::Int(1)));
    assert!(matches!(l[5], Value::Int(3)));
  } else {
    panic!("got {:?}", v);
  }
}

#[test]
fn builtin_all_any_chain() {
  let positive = eval_expr("builtins.all (x: x > 0) [1 2 3]").unwrap();
  assert!(matches!(positive, Value::Bool(true)));
  let any_neg = eval_expr("builtins.any (x: x < 0) [1 -2 3]").unwrap();
  assert!(matches!(any_neg, Value::Bool(true)));
}

#[test]
fn builtin_attrnames_attrvalues_compose() {
  let v = eval_expr("builtins.length (builtins.attrNames { a = 1; b = 2; c = 3; })").unwrap();
  assert!(matches!(v, Value::Int(3)));
}

#[test]
fn builtin_list_to_attrs_then_select() {
  let v = eval_expr(
    "(builtins.listToAttrs [{ name = \"a\"; value = 1; } { name = \"b\"; value = 2; }]).b",
  )
  .unwrap();
  assert!(matches!(v, Value::Int(2)), "got {:?}", v);
}

// ===== 8. EVALUATION ORDER / SHARING =====

#[test]
fn shared_thunk_evaluated_only_once_in_let() {
  // Tricky: laziness + sharing. Hard to test side effects in pnix, so we test correctness.
  let v = eval_expr("let x = 1 + 2; in [ x x x ]").unwrap();
  if let Value::List(l) = v {
    assert!(matches!(l[0], Value::Int(3)));
    assert!(matches!(l[1], Value::Int(3)));
    assert!(matches!(l[2], Value::Int(3)));
  } else {
    panic!();
  }
}

#[test]
fn lazy_recursive_attrset_field_used_late() {
  // 'b' depends on 'a', but 'a' itself is recursive expression
  let v = eval_expr(
    "let s = rec {
        a = if cond then 1 else 2;
        cond = true;
        b = a + 10;
      };
     in s.b",
  )
  .unwrap();
  assert!(matches!(v, Value::Int(11)), "got {:?}", v);
}

#[test]
fn lazy_self_referencing_default() {
  // Default uses other arg name
  let v = eval_expr("({a, b ? a + 1}: b) { a = 5; }").unwrap();
  assert!(matches!(v, Value::Int(6)), "got {:?}", v);
}

// ===== 9. STRING OPERATIONS / BUILTINS COMPOSITION =====

#[test]
fn builtin_concat_strings_sep_with_map() {
  let v =
    eval_expr("builtins.concatStringsSep \", \" (builtins.map (x: builtins.toString x) [1 2 3])")
      .unwrap();
  assert_eq!(v.as_str(), Some("1, 2, 3"));
}

#[test]
fn string_interp_with_call_chain() {
  let v = eval_expr("let f = x: builtins.toString (x + 1); in \"v=${f 41}\"").unwrap();
  assert_eq!(v.as_str(), Some("v=42"));
}

#[test]
fn substring_compose() {
  let v = eval_expr(r#"builtins.substring 0 3 "hello world""#).unwrap();
  assert_eq!(v.as_str(), Some("hel"));
}

#[test]
fn string_length_call() {
  let v = eval_expr(r#"builtins.stringLength "hello""#).unwrap();
  assert!(matches!(v, Value::Int(5)));
}

// ===== 10. EDGE / CORNER CASES =====

#[test]
fn nested_paren_arith() {
  let v = eval_expr("((1 + 2) * (3 + 4))").unwrap();
  assert!(matches!(v, Value::Int(21)));
}

#[test]
fn lambda_returning_lambda_in_list() {
  // [ (x: x+1) (x: x+2) ] → list of two lambdas; apply head 10 → 11
  let v = eval_expr("builtins.head [ (x: x + 1) (x: x + 2) ] 10").unwrap();
  assert!(matches!(v, Value::Int(11)), "got {:?}", v);
}

#[test]
fn higher_order_via_attrset() {
  // f.g 5 where f = { g = x: x*x; }
  let v = eval_expr("({ g = x: x * x; }.g) 5").unwrap();
  assert!(matches!(v, Value::Int(25)), "got {:?}", v);
}

#[test]
fn fix_y_combinator_style() {
  // Self-application via let-rec
  let v = eval_expr(
    "let fact = n: if n <= 0 then 1 else n * fact (n - 1);
     in fact 6",
  )
  .unwrap();
  assert!(matches!(v, Value::Int(720)), "got {:?}", v);
}

#[test]
fn deeply_nested_function_application() {
  let v = eval_expr("let inc = x: x + 1; in inc (inc (inc (inc (inc 0))))").unwrap();
  assert!(matches!(v, Value::Int(5)), "got {:?}", v);
}

#[test]
fn call_with_paren_result() {
  let v = eval_expr("(x: y: x + y) (1 + 2) (3 * 4)").unwrap();
  assert!(matches!(v, Value::Int(15)), "got {:?}", v);
}

#[test]
fn select_chain_then_apply() {
  let v = eval_expr(
    "let m = { ops = { add = a: b: a + b; }; };
     in m.ops.add 10 20",
  )
  .unwrap();
  assert!(matches!(v, Value::Int(30)), "got {:?}", v);
}

#[test]
fn builtin_try_eval_catches_failure() {
  let v = eval_expr("(builtins.tryEval (1 / 0)).success").unwrap();
  assert!(matches!(v, Value::Bool(false)), "got {:?}", v);
}

#[test]
fn builtin_try_eval_passes_success() {
  let v = eval_expr("(builtins.tryEval 42).value").unwrap();
  assert!(matches!(v, Value::Int(42)), "got {:?}", v);
}

#[test]
fn assert_then_continues_on_pass() {
  let v = eval_expr("assert 1 < 2; 99").unwrap();
  assert!(matches!(v, Value::Int(99)));
}

#[test]
fn assert_errors_on_fail() {
  assert!(eval_expr("assert 1 > 2; 99").is_err());
}

// ===== 11. NIX SPECIFIC: HAS-ATTR / SELECT-OR =====

#[test]
fn select_or_default() {
  let v = eval_expr("{ a = 1; }.b or 99").unwrap();
  assert!(matches!(v, Value::Int(99)));
}

#[test]
fn select_or_with_paren_default_expr() {
  let v = eval_expr("{ a = 1; }.b or (1 + 2)").unwrap();
  assert!(matches!(v, Value::Int(3)));
}

#[test]
fn has_attr_nested() {
  let v = eval_expr("{ a = { b = 1; }; } ? a.b").unwrap();
  assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn dynamic_attr_access() {
  let v = eval_expr(r#"let key = "a"; s = { a = 42; }; in s.${key}"#).unwrap();
  assert!(matches!(v, Value::Int(42)), "got {:?}", v);
}
