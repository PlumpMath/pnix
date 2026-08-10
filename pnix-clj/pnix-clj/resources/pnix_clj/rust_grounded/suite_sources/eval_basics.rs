//! pnix-eval 기본 평가 테스트.

use pnix_eval::{eval_expr, Value};

#[test]
fn eval_arithmetic() {
  let v = eval_expr("1 + 2").unwrap();
  assert!(matches!(v, Value::Int(3)));
}

#[test]
fn eval_let_in() {
  let v = eval_expr("let x = 10; in x + 5").unwrap();
  assert!(matches!(v, Value::Int(15)));
}

#[test]
fn eval_let_is_recursive_by_default() {
  let v = eval_expr("let a = b + 1; b = 2; in a").unwrap();
  assert!(matches!(v, Value::Int(3)));
}

#[test]
fn eval_let_supports_self_recursive_lambda() {
  let v = eval_expr(
    "let sum = xs: if (builtins.length xs) == 0 then 0 else (builtins.head xs) + sum (builtins.tail xs); in sum [1 2 3]",
  )
  .unwrap();
  assert!(matches!(v, Value::Int(6)));
}

#[test]
fn eval_lambda_apply() {
  let v = eval_expr("(x: x + 1) 5").unwrap();
  assert!(matches!(v, Value::Int(6)));
}

#[test]
fn eval_curried_lambda() {
  let v = eval_expr("let add = x: y: x + y; in add 3 4").unwrap();
  assert!(matches!(v, Value::Int(7)));
}

#[test]
fn eval_attrset_lambda_pattern_with_default_and_binding() {
  let v =
    eval_expr(r#"(args@{ x, y ? x + 1, ... }: args.x + y + args.z) { x = 3; z = 5; }"#).unwrap();
  assert!(matches!(v, Value::Int(12)));
}

#[test]
fn eval_attrset_lambda_pattern_rejects_unexpected_attrs_without_ellipsis() {
  let err = eval_expr(r#"({ x }: x) { x = 1; y = 2; }"#).expect_err("unexpected attr should fail");
  assert!(err.to_string().contains("unexpected attribute 'y'"));
}

#[test]
fn eval_list_lambda_pattern_binds_tail() {
  let v = eval_expr(r#"([x, y, ...rest]: x + y + builtins.length rest) [1 2 3 4]"#).unwrap();
  assert!(matches!(v, Value::Int(5)));
}

#[test]
fn eval_list_lambda_pattern_checks_arity() {
  let err = eval_expr(r#"([x, y]: x) [1]"#).expect_err("short list should fail");
  assert!(err
    .to_string()
    .contains("list pattern expected at least 2 items, got 1"));
}

#[test]
fn eval_attrset_select() {
  let v = eval_expr("{ a = 1; b = 2; }.a").unwrap();
  assert!(matches!(v, Value::Int(1)));
}

#[test]
fn eval_rec_attrset() {
  let v = eval_expr("rec { a = 1; b = a + 2; }.b").unwrap();
  assert!(matches!(v, Value::Int(3)));
}

#[test]
fn eval_rec_attrset_forward_reference() {
  let v = eval_expr("rec { a = b + 1; b = 2; }.a").unwrap();
  assert!(matches!(v, Value::Int(3)));
}

#[test]
fn eval_rec_attrset_lambda_uses_late_binding() {
  let v = eval_expr("rec { f = x: x + seed; seed = 2; }.f 3").unwrap();
  assert!(matches!(v, Value::Int(5)));
}

#[test]
fn eval_rec_attrset_cycle_errors() {
  let err = eval_expr("rec { a = b; b = a; }.a").expect_err("cycle should fail");
  let err = err.to_string();
  assert!(
    err.contains("recursive attrset cycle"),
    "cycle error should mention recursive attrset cycle: {}",
    err
  );
}

#[test]
fn eval_builtins_map() {
  let v = eval_expr("builtins.map (x: x * 2) [1 2 3]").unwrap();
  if let Value::List(items) = v {
    assert_eq!(items.len(), 3);
    assert!(matches!(items[0], Value::Int(2)));
    assert!(matches!(items[1], Value::Int(4)));
    assert!(matches!(items[2], Value::Int(6)));
  } else {
    panic!("expected list");
  }
}

#[test]
fn eval_builtins_filter() {
  let v = eval_expr("builtins.filter (x: x > 2) [1 2 3 4 5]").unwrap();
  if let Value::List(items) = v {
    assert_eq!(items.len(), 3);
  } else {
    panic!("expected list");
  }
}

#[test]
fn eval_builtins_length() {
  let v = eval_expr("builtins.length [1 2 3 4]").unwrap();
  assert!(matches!(v, Value::Int(4)));
}

#[test]
fn eval_builtins_foldl() {
  let v = eval_expr("builtins.foldl' (acc: x: acc + x) 0 [1 2 3 4]").unwrap();
  assert!(matches!(v, Value::Int(10)));
}

#[test]
fn eval_with_expr() {
  let v = eval_expr("with { x = 42; }; x").unwrap();
  assert!(matches!(v, Value::Int(42)));
}

#[test]
fn eval_assert_pass() {
  let v = eval_expr("assert true; 42").unwrap();
  assert!(matches!(v, Value::Int(42)));
}

#[test]
fn eval_assert_fail() {
  assert!(eval_expr("assert false; 42").is_err());
}

#[test]
fn eval_if_then_else() {
  let v = eval_expr("if true then 1 else 2").unwrap();
  assert!(matches!(v, Value::Int(1)));
}

#[test]
fn eval_string_interp() {
  let v = eval_expr(r#"let name = "world"; in "hello ${name}""#).unwrap();
  assert_eq!(v.as_str(), Some("hello world"));
}

#[test]
fn eval_string_interp_preserves_unresolved_placeholder_literal() {
  let v = eval_expr(r#"let known = "x"; in "prefix ${known} ${value}""#).unwrap();
  assert_eq!(v.as_str(), Some("prefix x ${value}"));
}

#[test]
fn eval_string_interp_rejects_integer_silently() {
  let err = eval_expr(r#""value=${1}""#).unwrap_err();
  let msg = err.to_string();
  assert!(
    msg.contains("cannot coerce an integer"),
    "expected integer coercion error, got: {msg}"
  );
}

#[test]
fn eval_string_interp_rejects_list() {
  let err = eval_expr(r#""value=${[1 2 3]}""#).unwrap_err();
  let msg = err.to_string();
  assert!(
    msg.contains("cannot coerce a list"),
    "expected list coercion error, got: {msg}"
  );
}

#[test]
fn eval_string_interp_rejects_attrset_without_to_string_or_out_path() {
  let err = eval_expr(r#""value=${ { a = 1; } }""#).unwrap_err();
  let msg = err.to_string();
  assert!(
    msg.contains("cannot coerce a set") && msg.contains("__toString"),
    "expected set coercion error mentioning __toString, got: {msg}"
  );
}

#[test]
fn eval_string_interp_rejects_boolean() {
  let err = eval_expr(r#""value=${true}""#).unwrap_err();
  assert!(err.to_string().contains("cannot coerce a boolean"));
}

#[test]
fn eval_string_interp_rejects_null() {
  let err = eval_expr(r#""value=${null}""#).unwrap_err();
  assert!(err.to_string().contains("cannot coerce null"));
}

#[test]
fn eval_string_interp_rejects_lambda() {
  let err = eval_expr(r#""value=${ x: x }""#).unwrap_err();
  assert!(err.to_string().contains("cannot coerce a function"));
}

#[test]
fn eval_string_interp_accepts_attrset_out_path() {
  let v = eval_expr(r#""path=${ { outPath = "/nix/store/x"; } }""#).unwrap();
  assert_eq!(v.as_str(), Some("path=/nix/store/x"));
}

#[test]
fn eval_string_interp_accepts_attrset_to_string() {
  let v = eval_expr(r#""greeting=${ { __toString = self: "hello"; } }""#).unwrap();
  assert_eq!(v.as_str(), Some("greeting=hello"));
}

#[test]
fn eval_string_interp_to_string_receives_self() {
  let v = eval_expr(r#""${ { __toString = self: self.label; label = "abc"; } }""#).unwrap();
  assert_eq!(v.as_str(), Some("abc"));
}

#[test]
fn eval_string_interp_to_string_takes_priority_over_out_path() {
  let v =
    eval_expr(r#""${ { __toString = _: "from-toString"; outPath = "from-outPath"; } }""#).unwrap();
  assert_eq!(v.as_str(), Some("from-toString"));
}

#[test]
fn eval_string_interp_out_path_can_be_path_value() {
  // outPath is a string here; nested chain via __toString returning a string
  let v = eval_expr(r#""${ { outPath = { __toString = _: "deep"; }; } }""#).unwrap();
  assert_eq!(v.as_str(), Some("deep"));
}

#[test]
fn eval_string_interp_explicit_to_string_builtin_still_works() {
  let v = eval_expr(r#""value=${builtins.toString 42}""#).unwrap();
  assert_eq!(v.as_str(), Some("value=42"));
}

#[test]
fn eval_has_attr() {
  let v = eval_expr("{ a = 1; } ? a").unwrap();
  assert!(matches!(v, Value::Bool(true)));
  let v2 = eval_expr("{ a = 1; } ? b").unwrap();
  assert!(matches!(v2, Value::Bool(false)));
}

#[test]
fn eval_list_concat() {
  let v = eval_expr("[1 2] ++ [3 4]").unwrap();
  if let Value::List(items) = v {
    assert_eq!(items.len(), 4);
  } else {
    panic!("expected list");
  }
}

#[test]
fn eval_attrset_merge() {
  let v = eval_expr("{ a = 1; } // { b = 2; }").unwrap();
  if let Value::AttrSet(map) = v {
    assert_eq!(map.len(), 2);
  } else {
    panic!("expected attrset");
  }
}

#[test]
fn eval_ontology_lift() {
  let v = eval_expr(r#"builtins.ontologyLift { value = "test"; } "context""#).unwrap();
  if let Value::AttrSet(map) = v {
    assert_eq!(
      map.get("ontology-status").and_then(|v| v.as_str()),
      Some("Candidate")
    );
  } else {
    panic!("expected attrset");
  }
}
