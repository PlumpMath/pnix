//! pnix-eval 타입 처리 / Nix 호환 산술 / 비교 / 등호 회귀 테스트.
//! `${} placeholder` 수정에 이어 표면화된 약점들 (Int/Float 보존,
//! 정수 나눗셈, 문자열/리스트 lexicographic 비교, 구조적 등호) 을 잠근다.

use pnix_eval::{eval_expr, Value};

#[test]
fn list_with_paren_int_exprs() {
  let v = eval_expr("[ (1 + 2) (3 * 4) (10 - 5) ]").unwrap();
  if let Value::List(items) = v {
    assert_eq!(items.len(), 3);
    assert!(matches!(items[0], Value::Int(3)), "got {:?}", items[0]);
    assert!(matches!(items[1], Value::Int(12)), "got {:?}", items[1]);
    assert!(matches!(items[2], Value::Int(5)), "got {:?}", items[2]);
  } else {
    panic!("expected list, got {:?}", v);
  }
}

#[test]
fn list_with_paren_mixed() {
  let v = eval_expr("[ (1 + 2) (\"a\" + \"b\") ([1 2] ++ [3]) ]").unwrap();
  if let Value::List(items) = v {
    assert_eq!(items.len(), 3);
  } else {
    panic!("expected list");
  }
}

#[test]
fn int_plus_int_returns_int() {
  let v = eval_expr("1 + 2").unwrap();
  assert!(matches!(v, Value::Int(3)), "got {:?}", v);
}

#[test]
fn int_plus_float_returns_float() {
  let v = eval_expr("1 + 1.5").unwrap();
  if let Value::Float(f) = v {
    assert!((f - 2.5).abs() < 1e-9);
  } else {
    panic!("expected float, got {:?}", v);
  }
}

#[test]
fn int_minus_int_returns_int() {
  let v = eval_expr("10 - 3").unwrap();
  assert!(matches!(v, Value::Int(7)), "got {:?}", v);
}

#[test]
fn int_div_int_returns_int_in_nix() {
  // nix: integer division when both are ints
  let v = eval_expr("10 / 3").unwrap();
  assert!(matches!(v, Value::Int(3)), "got {:?}", v);
}

#[test]
fn int_div_zero_errors() {
  let err = eval_expr("1 / 0").unwrap_err();
  assert!(
    err.to_string().to_lowercase().contains("zero"),
    "got {}",
    err
  );
}

#[test]
fn negation_of_int() {
  let v = eval_expr("-5").unwrap();
  assert!(matches!(v, Value::Int(-5)), "got {:?}", v);
}

#[test]
fn negation_of_paren_expr() {
  let v = eval_expr("-(2 + 3)").unwrap();
  assert!(matches!(v, Value::Int(-5)), "got {:?}", v);
}

#[test]
fn comparison_int_int() {
  assert!(matches!(eval_expr("1 < 2").unwrap(), Value::Bool(true)));
  assert!(matches!(eval_expr("2 <= 2").unwrap(), Value::Bool(true)));
  assert!(matches!(eval_expr("3 > 2").unwrap(), Value::Bool(true)));
  assert!(matches!(eval_expr("3 >= 4").unwrap(), Value::Bool(false)));
}

#[test]
fn comparison_int_float_mixed() {
  assert!(matches!(eval_expr("1 < 1.5").unwrap(), Value::Bool(true)));
  assert!(matches!(eval_expr("2.0 == 2").unwrap(), Value::Bool(true)));
}

#[test]
fn equality_string_string() {
  assert!(matches!(
    eval_expr(r#""a" == "a""#).unwrap(),
    Value::Bool(true)
  ));
}

#[test]
fn equality_list_list() {
  assert!(matches!(
    eval_expr("[1 2] == [1 2]").unwrap(),
    Value::Bool(true)
  ));
  assert!(matches!(
    eval_expr("[1 2] == [1 3]").unwrap(),
    Value::Bool(false)
  ));
}

#[test]
fn equality_attrset_attrset() {
  assert!(matches!(
    eval_expr("{a=1;} == {a=1;}").unwrap(),
    Value::Bool(true)
  ));
  assert!(matches!(
    eval_expr("{a=1;} == {a=2;}").unwrap(),
    Value::Bool(false)
  ));
}

#[test]
fn string_plus_string_concat() {
  let v = eval_expr(r#""hi " + "there""#).unwrap();
  assert_eq!(v.as_str(), Some("hi there"));
}

#[test]
fn string_plus_int_should_error() {
  // nix: error
  let r = eval_expr(r#""x" + 1"#);
  assert!(r.is_err(), "expected error, got {:?}", r);
}

#[test]
fn list_plus_plus_concat() {
  let v = eval_expr("[1 2] ++ [3 4]").unwrap();
  if let Value::List(l) = v {
    assert_eq!(l.len(), 4);
  } else {
    panic!("expected list");
  }
}

#[test]
fn list_plus_plus_with_paren_exprs() {
  let v = eval_expr("[ (1+1) ] ++ [ (2*3) ]").unwrap();
  if let Value::List(l) = v {
    assert_eq!(l.len(), 2);
    assert!(matches!(l[0], Value::Int(2)));
    assert!(matches!(l[1], Value::Int(6)));
  } else {
    panic!("expected list, got {:?}", v);
  }
}

#[test]
fn attrset_merge_update_op() {
  let v = eval_expr("{ a = 1; } // { b = 2; }").unwrap();
  if let Value::AttrSet(m) = v {
    assert_eq!(m.len(), 2);
  } else {
    panic!("expected set");
  }
}

#[test]
fn boolean_logical_ops() {
  assert!(matches!(
    eval_expr("true && false").unwrap(),
    Value::Bool(false)
  ));
  assert!(matches!(
    eval_expr("true || false").unwrap(),
    Value::Bool(true)
  ));
  assert!(matches!(eval_expr("!true").unwrap(), Value::Bool(false)));
  assert!(matches!(
    eval_expr("true -> false").unwrap(),
    Value::Bool(false)
  ));
}

#[test]
fn modulo_int_int() {
  let v = eval_expr("builtins.bitAnd 5 3");
  let _ = v;
  // explicit modulo doesn't exist in nix; check arithmetic
  let v2 = eval_expr("10 - (10 / 3) * 3").unwrap();
  assert!(matches!(v2, Value::Int(1)), "got {:?}", v2);
}

#[test]
fn equal_int_int() {
  assert!(matches!(eval_expr("1 == 1").unwrap(), Value::Bool(true)));
}

#[test]
fn equal_int_float_same_value() {
  // nix: 1 == 1.0 → true
  assert!(
    matches!(eval_expr("1 == 1.0").unwrap(), Value::Bool(true)),
    "expected true for 1 == 1.0"
  );
}

#[test]
fn equal_int_string_should_be_false() {
  // nix: types differ → false (no error)
  assert!(matches!(
    eval_expr(r#"1 == "1""#).unwrap(),
    Value::Bool(false)
  ));
}

#[test]
fn lt_string_string_lex() {
  // nix supports lexical string comparison
  assert!(matches!(
    eval_expr(r#""a" < "b""#).unwrap(),
    Value::Bool(true)
  ));
}

#[test]
fn lt_list_list_lex() {
  // nix supports lex list comparison
  assert!(matches!(
    eval_expr(r#"[1 2] < [1 3]"#).unwrap(),
    Value::Bool(true)
  ));
}

#[test]
fn list_equal_with_paren_arith_inside() {
  assert!(matches!(
    eval_expr("[ (1 + 1) ] == [ 2 ]").unwrap(),
    Value::Bool(true)
  ));
}

#[test]
fn float_div_zero_errors() {
  let r = eval_expr("1.0 / 0.0");
  assert!(r.is_err(), "expected error, got {:?}", r);
}

#[test]
fn builtins_div_int_int_zero_errors() {
  let r = eval_expr("builtins.div 5 0");
  assert!(r.is_err(), "expected error, got {:?}", r);
}

#[test]
fn builtins_div_int_int_returns_int() {
  let v = eval_expr("builtins.div 10 3").unwrap();
  assert!(matches!(v, Value::Int(3)), "got {:?}", v);
}

#[test]
fn builtins_mod_int_int() {
  let v = eval_expr("builtins.mod 10 3").unwrap();
  assert!(matches!(v, Value::Int(1)), "got {:?}", v);
}

#[test]
fn builtins_lessthan_string_string() {
  let v = eval_expr(r#"builtins.lessThan "a" "b""#).unwrap();
  assert!(matches!(v, Value::Bool(true)), "got {:?}", v);
}

#[test]
fn negation_inside_list_paren() {
  let v = eval_expr("[ (-1) (-2) (-3) ]").unwrap();
  if let Value::List(items) = v {
    assert!(matches!(items[0], Value::Int(-1)));
    assert!(matches!(items[1], Value::Int(-2)));
    assert!(matches!(items[2], Value::Int(-3)));
  } else {
    panic!("expected list");
  }
}

#[test]
fn arith_overflow_int_should_error_or_promote() {
  // i64::MAX + 1 — current behavior?
  let r = eval_expr("9223372036854775807 + 1");
  // Either error OR promote to float - both acceptable; silent wrap is bad
  match r {
    Ok(Value::Int(v)) => panic!("silent integer wrap to {} is a bug", v),
    Ok(Value::Float(_)) => {}
    Err(_) => {}
    _ => panic!("unexpected"),
  }
}

#[test]
fn equality_attrset_nested() {
  assert!(matches!(
    eval_expr("{a={b=1;};} == {a={b=1;};}").unwrap(),
    Value::Bool(true)
  ));
  assert!(matches!(
    eval_expr("{a={b=1;};} == {a={b=2;};}").unwrap(),
    Value::Bool(false)
  ));
}

#[test]
fn equality_function_returns_false_in_nix() {
  // Nix: comparing functions always false
  let r = eval_expr("(x: x) == (x: x)");
  // Acceptable: false OR error - silent true is bad
  match r {
    Ok(Value::Bool(true)) => panic!("function == function returning true is a bug"),
    _ => {}
  }
}
