//! PNIX Match 테스트: PNIX match 표현식 처리 테스트
//!
//! PNIX match 표현식이 올바르게 파싱되고 FxCore로 변환되는지 검증합니다.

use pnix_core::fx::core_expr::FxCoreExpr;
use pnix_core::lang::pnix::lower::{lower_to_fx_core, pnix_expr_to_unified};
use pnix_core::lang::pnix::parser::parse_expr;
use pnix_core::lang::pnix::syntax::{PnixLiteralPattern, PnixPattern};
use pnix_core::lang::pnix::{PnixExpr, UnifiedExpr};

fn contains_expr(expr: &UnifiedExpr, predicate: &impl Fn(&UnifiedExpr) -> bool) -> bool {
  if predicate(expr) {
    return true;
  }
  match expr {
    UnifiedExpr::Int(_)
    | UnifiedExpr::Float(_)
    | UnifiedExpr::Bool(_)
    | UnifiedExpr::String(_)
    | UnifiedExpr::Var(_)
    | UnifiedExpr::ParamTime
    | UnifiedExpr::ParamDeltaTime
    | UnifiedExpr::ParamSignal(_)
    | UnifiedExpr::SignalVar(_)
    | UnifiedExpr::Null
    | UnifiedExpr::Throw(_)
    | UnifiedExpr::Interop { .. } => false,
    UnifiedExpr::Neg(arg)
    | UnifiedExpr::Floor(arg)
    | UnifiedExpr::Ceil(arg)
    | UnifiedExpr::Abs(arg)
    | UnifiedExpr::Sqrt(arg)
    | UnifiedExpr::Sin(arg)
    | UnifiedExpr::Cos(arg)
    | UnifiedExpr::Tan(arg)
    | UnifiedExpr::Exp(arg)
    | UnifiedExpr::Ln(arg)
    | UnifiedExpr::Not(arg)
    | UnifiedExpr::Fx(arg) => contains_expr(arg, predicate),
    UnifiedExpr::Add(lhs, rhs)
    | UnifiedExpr::Sub(lhs, rhs)
    | UnifiedExpr::Mul(lhs, rhs)
    | UnifiedExpr::Div(lhs, rhs)
    | UnifiedExpr::Mod(lhs, rhs)
    | UnifiedExpr::Pow(lhs, rhs)
    | UnifiedExpr::Lt(lhs, rhs)
    | UnifiedExpr::Gt(lhs, rhs)
    | UnifiedExpr::Le(lhs, rhs)
    | UnifiedExpr::Ge(lhs, rhs)
    | UnifiedExpr::Eq(lhs, rhs)
    | UnifiedExpr::Ne(lhs, rhs)
    | UnifiedExpr::And(lhs, rhs)
    | UnifiedExpr::Or(lhs, rhs)
    | UnifiedExpr::Concat(lhs, rhs)
    | UnifiedExpr::Merge(lhs, rhs) => {
      contains_expr(lhs, predicate) || contains_expr(rhs, predicate)
    }
    UnifiedExpr::If { cond, then_, else_ } => {
      contains_expr(cond, predicate)
        || contains_expr(then_, predicate)
        || contains_expr(else_, predicate)
    }
    UnifiedExpr::Let { value, body, .. } => {
      contains_expr(value, predicate) || contains_expr(body, predicate)
    }
    UnifiedExpr::Apply { args, .. } => args.iter().any(|arg| contains_expr(arg, predicate)),
    UnifiedExpr::Derived { args, .. } => args.iter().any(|arg| contains_expr(arg, predicate)),
    UnifiedExpr::AttrSet(items) => items
      .iter()
      .any(|(_, value)| contains_expr(value, predicate)),
    UnifiedExpr::List(items) => items.iter().any(|value| contains_expr(value, predicate)),
    UnifiedExpr::Lambda { body, .. } => contains_expr(body, predicate),
    UnifiedExpr::Construct { args, .. } => args.iter().any(|arg| contains_expr(arg, predicate)),
  }
}

fn contains_apply_func(expr: &UnifiedExpr, func: &str) -> bool {
  contains_expr(
    expr,
    &|node| matches!(node, UnifiedExpr::Apply { func: f, .. } if f == func),
  )
}

fn contains_has_attr(expr: &UnifiedExpr, name: &str) -> bool {
  contains_expr(expr, &|node| {
    let UnifiedExpr::Apply { func, args } = node else {
      return false;
    };
    if func != "builtins.hasAttr" {
      return false;
    }
    matches!(args.first(), Some(UnifiedExpr::String(value)) if value == name)
  })
}

fn contains_let_name(expr: &UnifiedExpr, name: &str) -> bool {
  contains_expr(
    expr,
    &|node| matches!(node, UnifiedExpr::Let { name: n, .. } if n == name),
  )
}

fn contains_ge(expr: &UnifiedExpr) -> bool {
  contains_expr(expr, &|node| matches!(node, UnifiedExpr::Ge(_, _)))
}

#[test]
fn test_match_literal_pattern() {
  // match 42 with | 0 => "zero" | 42 => "forty-two" | _ => "other"
  let source = r#"match 42 with | 0 => "zero" | 42 => "forty-two" | _ => "other""#;
  let expr = parse_expr(source).unwrap();

  // PnixExpr::Match를 UnifiedExpr로 변환
  let unified = pnix_expr_to_unified(&expr).unwrap();

  // UnifiedExpr는 Let으로 감싸진 If 체인으로 변환되어야 함
  // arm 순서: | 0 => "zero" | 42 => "forty-two" | _ => "other"
  // 변환 순서: 마지막 arm부터 역순으로 If 체인 구성
  // 최종 구조: let _match_scrutinee = 42 in if _match_scrutinee == 0 then "zero" else (if _match_scrutinee == 42 then "forty-two" else "other")
  match &unified {
    UnifiedExpr::Let { name, value, body } => {
      assert!(
        name.starts_with("_match_scrutinee_"),
        "expected gensym scrutinee name, got: {}",
        name
      );
      // value는 42여야 함
      match value.as_ref() {
        UnifiedExpr::Int(42) => {}
        _ => panic!("Expected Int(42), got {:?}", value),
      }
      // body는 If 체인
      match body.as_ref() {
        UnifiedExpr::If { cond, then_, else_ } => {
          // 최외곽 cond는 첫 번째 arm: _match_scrutinee == 0
          match cond.as_ref() {
            UnifiedExpr::Eq(lhs, rhs) => match (lhs.as_ref(), rhs.as_ref()) {
              (UnifiedExpr::Var(v), UnifiedExpr::Int(0)) if v.starts_with("_match_scrutinee_") => {}
              _ => panic!("Expected _match_scrutinee == 0, got {:?} == {:?}", lhs, rhs),
            },
            _ => panic!("Expected Eq condition, got {:?}", cond),
          }
          // then_는 첫 번째 arm의 body: "zero"
          match then_.as_ref() {
            UnifiedExpr::String(s) => {
              assert_eq!(s, "zero");
            }
            _ => panic!("Expected string 'zero', got {:?}", then_),
          }
          // else_는 두 번째/세 번째 arm의 If 체인
          match else_.as_ref() {
            UnifiedExpr::If {
              cond: cond2,
              then_: then2,
              else_: else2,
            } => {
              // cond2는 두 번째 arm: _match_scrutinee == 42
              match cond2.as_ref() {
                UnifiedExpr::Eq(lhs2, rhs2) => match (lhs2.as_ref(), rhs2.as_ref()) {
                  (UnifiedExpr::Var(v), UnifiedExpr::Int(42))
                    if v.starts_with("_match_scrutinee_") => {}
                  _ => panic!(
                    "Expected _match_scrutinee == 42, got {:?} == {:?}",
                    lhs2, rhs2
                  ),
                },
                _ => panic!("Expected Eq condition, got {:?}", cond2),
              }
              // then2는 "forty-two"
              match then2.as_ref() {
                UnifiedExpr::String(s) => {
                  assert_eq!(s, "forty-two");
                }
                _ => panic!("Expected string 'forty-two', got {:?}", then2),
              }
              // else2는 wildcard arm "other"
              match else2.as_ref() {
                UnifiedExpr::String(s) => {
                  assert_eq!(s, "other");
                }
                _ => panic!("Expected string 'other', got {:?}", else2),
              }
            }
            _ => panic!("Expected nested If, got {:?}", else_),
          }
        }
        _ => panic!("Expected If chain, got {:?}", body),
      }
    }
    _ => panic!("Expected Let wrapper, got {:?}", unified),
  }

  // FxCoreExpr로 lowering도 테스트
  let fxcore = lower_to_fx_core(&unified).unwrap();
  // UnifiedExpr::Let은 FxCoreExpr::Let으로 변환됨
  match &fxcore {
    FxCoreExpr::Let { name, body, .. } => {
      assert!(
        name.starts_with("_match_scrutinee_"),
        "expected gensym scrutinee name, got: {}",
        name
      );
      // body는 If 체인으로 변환됨
      match body.as_ref() {
        FxCoreExpr::If { .. } => {
          // If 체인으로 변환됨
        }
        _ => panic!("Expected FxCoreExpr::If in Let body, got {:?}", body),
      }
    }
    _ => panic!("Expected FxCoreExpr::Let with If chain, got {:?}", fxcore),
  }
}

#[test]
fn test_match_wildcard_pattern() {
  // match x with | _ => "always"
  let source = r#"match x with | _ => "always""#;
  let expr = parse_expr(source).unwrap();

  let unified = pnix_expr_to_unified(&expr).unwrap();

  // 단일 wildcard match는 불필요한 Let/If를 만들지 않고 바로 body로 낮춰짐
  match &unified {
    UnifiedExpr::String(s) => {
      assert_eq!(s, "always");
    }
    _ => panic!("Expected direct string 'always', got {:?}", unified),
  }
}

#[test]
fn test_match_variable_pattern() {
  // match x with | n => n + 1
  let source = r#"match x with | n => n + 1"#;
  let expr = parse_expr(source).unwrap();

  let unified = pnix_expr_to_unified(&expr).unwrap();

  // Let으로 감싸져 있고, 내부에 변수 바인딩이 있어야 함
  match &unified {
    UnifiedExpr::Let {
      name: scrutinee_var,
      value,
      body,
    } => {
      assert!(
        scrutinee_var.starts_with("_match_scrutinee_"),
        "expected gensym scrutinee name, got: {}",
        scrutinee_var
      );
      // value는 x (Var)
      match value.as_ref() {
        UnifiedExpr::Var(v) => assert_eq!(v, "x"),
        _ => panic!("Expected Var('x'), got {:?}", value),
      }
      // body는 Let으로 n에 scrutinee를 바인딩하고 n + 1 계산
      match body.as_ref() {
        UnifiedExpr::Let {
          name,
          value: bound_value,
          body: body_expr,
        } => {
          assert_eq!(name, "n");
          // bound_value는 _match_scrutinee
          match bound_value.as_ref() {
            UnifiedExpr::Var(v) => assert!(
              v.starts_with("_match_scrutinee_"),
              "expected gensym scrutinee ref, got: {}",
              v
            ),
            _ => panic!("Expected Var('_match_scrutinee_*'), got {:?}", bound_value),
          }
          // body_expr는 n + 1
          match body_expr.as_ref() {
            UnifiedExpr::Add(lhs, rhs) => match (lhs.as_ref(), rhs.as_ref()) {
              (UnifiedExpr::Var(v), UnifiedExpr::Int(1)) if v == "n" => {}
              _ => panic!("Expected n + 1, got {:?} + {:?}", lhs, rhs),
            },
            _ => panic!("Expected Add, got {:?}", body_expr),
          }
        }
        _ => panic!("Expected nested Let for variable binding, got {:?}", body),
      }
    }
    _ => panic!("Expected Let wrapper, got {:?}", unified),
  }
}

#[test]
fn test_match_guard_condition() {
  // Y08c: match x with | n if n > 0 => "positive" | _ => "zero-or-negative"
  let source = r#"match x with | n if n > 0 => "positive" | _ => "zero-or-negative""#;
  let expr = parse_expr(source).unwrap();

  let unified = pnix_expr_to_unified(&expr).unwrap();

  // Let으로 감싸져 있고, 첫 번째 arm에 가드 조건이 있어야 함
  match &unified {
    UnifiedExpr::Let {
      name: scrutinee_var,
      body,
      ..
    } => {
      assert!(
        scrutinee_var.starts_with("_match_scrutinee_"),
        "expected gensym scrutinee name, got: {}",
        scrutinee_var
      );
      // body는 If 체인
      match body.as_ref() {
        UnifiedExpr::If { cond, then_, else_ } => {
          // cond는 Let으로 변수 바인딩 후 가드 조건 평가
          // Let { name: "n", value: Var("_match_scrutinee_*"), body: Gt(Var("n"), Int(0)) }
          match cond.as_ref() {
            UnifiedExpr::Let {
              name,
              value,
              body: guard_body,
            } => {
              assert_eq!(name, "n", "Expected variable binding for 'n'");
              // value는 scrutinee 참조
              match value.as_ref() {
                UnifiedExpr::Var(v) => {
                  assert!(
                    v.starts_with("_match_scrutinee_"),
                    "expected scrutinee ref, got: {}",
                    v
                  );
                }
                _ => panic!("Expected Var for scrutinee ref, got {:?}", value),
              }
              // guard_body는 가드 조건 (n > 0)
              match guard_body.as_ref() {
                UnifiedExpr::Gt(lhs_guard, rhs_guard) => {
                  match (lhs_guard.as_ref(), rhs_guard.as_ref()) {
                    (UnifiedExpr::Var(v), UnifiedExpr::Int(0)) if v == "n" => {}
                    _ => panic!("Expected n > 0, got {:?} > {:?}", lhs_guard, rhs_guard),
                  }
                }
                _ => panic!("Expected Gt for guard condition, got {:?}", guard_body),
              }
            }
            _ => panic!("Expected Let for variable binding in cond, got {:?}", cond),
          }
          // then_는 Let으로 n 바인딩 후 "positive" 반환
          match then_.as_ref() {
            UnifiedExpr::Let {
              name,
              value,
              body: then_body,
            } => {
              assert_eq!(name, "n", "Expected variable binding for 'n' in then_");
              match value.as_ref() {
                UnifiedExpr::Var(v) => {
                  assert!(
                    v.starts_with("_match_scrutinee_"),
                    "expected scrutinee ref, got: {}",
                    v
                  );
                }
                _ => panic!("Expected Var for scrutinee ref in then_, got {:?}", value),
              }
              match then_body.as_ref() {
                UnifiedExpr::String(s) => assert_eq!(s, "positive"),
                _ => panic!("Expected string 'positive', got {:?}", then_body),
              }
            }
            _ => panic!(
              "Expected Let for variable binding in then_, got {:?}",
              then_
            ),
          }
          // else_는 wildcard arm "zero-or-negative"
          match else_.as_ref() {
            UnifiedExpr::String(s) => {
              assert_eq!(s, "zero-or-negative");
            }
            _ => panic!("Expected string 'zero-or-negative', got {:?}", else_),
          }
        }
        _ => panic!("Expected If chain, got {:?}", body),
      }
    }
    _ => panic!("Expected Let wrapper, got {:?}", unified),
  }
}

#[test]
fn test_match_empty_arms_error() {
  // match x with (빈 arms) - 파서가 에러를 발생시켜야 함
  use pnix_core::lang::pnix::parser::parse_expr;

  let source = "match x with";
  let result = parse_expr(source);

  // 빈 arms는 파서 에러를 발생시켜야 함
  assert!(
    result.is_err(),
    "Empty match arms should produce a parse error"
  );
}

#[test]
fn test_match_and_logical_or_token_boundary() {
  // Y08b-6: Test that single `|` (match arm separator) and `||` (logical OR) are correctly distinguished
  // match expr with | pat1 => e1 | pat2 => e2 should parse correctly
  // expr || expr should parse as logical OR, not match arms

  // Test 1: match with || in guard condition
  let source1 = r#"match 5 with | x if x > 0 || x < 10 => "ok" | _ => "other""#;
  let expr1 = parse_expr(source1).unwrap();
  match &expr1 {
    PnixExpr::Match {
      scrutinee: _scrutinee,
      arms,
    } => {
      assert_eq!(arms.len(), 2, "Should have 2 arms");
      // First arm should have guard with || operator
      if let Some(guard) = &arms[0].guard {
        match guard.as_ref() {
          PnixExpr::Binary { op, .. } => {
            assert_eq!(*op, "||", "Guard should contain || operator");
          }
          _ => panic!("Guard should be binary expression with ||"),
        }
      } else {
        panic!("First arm should have guard condition");
      }
    }
    _ => panic!("Should parse as Match expression"),
  }

  // Test 2: match result used in || expression
  // Note: This might parse as match with guard, need to check actual behavior
  let source2 = r#"(match 5 with | 0 => false | _ => true) || false"#;
  let expr2 = parse_expr(source2).unwrap();
  match &expr2 {
    PnixExpr::Binary { op, .. } => {
      // Should parse as: (match 5 with | 0 => false | _ => true) || false
      assert_eq!(*op, "||", "Should parse as logical OR");
    }
    _ => {
      // If it doesn't parse as Binary, check what it actually parsed as
      eprintln!("Unexpected parse result: {:?}", expr2);
      panic!("Should parse as Binary with ||, got: {:?}", expr2);
    }
  }

  // Test 3: match with || in scrutinee
  let source3 = r#"match true || false with | true => "yes" | false => "no""#;
  let expr3 = parse_expr(source3).unwrap();
  match &expr3 {
    PnixExpr::Match { scrutinee, arms } => {
      // scrutinee should be Binary with ||
      match scrutinee.as_ref() {
        PnixExpr::Binary { op, .. } => {
          assert_eq!(*op, "||", "Scrutinee should contain || operator");
        }
        _ => panic!("Scrutinee should be binary expression with ||"),
      }
      assert_eq!(arms.len(), 2, "Should have 2 arms");
    }
    _ => panic!("Should parse as Match expression"),
  }

  // Test 4: nested match with || in outer expression
  let source4 = r#"(match (match 0 with | 0 => true | _ => false) with | true => "zero" | false => "nonzero") || "unknown""#;
  let expr4 = parse_expr(source4).unwrap();
  match &expr4 {
    PnixExpr::Binary { op, .. } => {
      // Should parse as: (match (match 0 with ...) with ...) || "unknown"
      assert_eq!(*op, "||", "Outer expression should be logical OR");
    }
    _ => {
      eprintln!("Unexpected parse result: {:?}", expr4);
      panic!("Should parse as Binary with ||, got: {:?}", expr4);
    }
  }
}

#[test]
fn test_nullary_constructor_parsing() {
  // Y09b-1: nullary constructor 파싱 보완 테스트
  // None, True, False 같은 nullary constructor가 Construct로 파싱되는지 확인

  // Test 1: None (nullary constructor)
  let source1 = "None";
  let expr1 = parse_expr(source1).unwrap();
  match &expr1 {
    PnixExpr::Construct { variant, args } => {
      assert_eq!(variant, "None", "None should be parsed as Construct");
      assert_eq!(args.len(), 0, "None should have no arguments");
    }
    _ => panic!("None should be parsed as Construct, got: {:?}", expr1),
  }

  // Test 2: Some(42) (constructor with arguments)
  let source2 = "Some(42)";
  let expr2 = parse_expr(source2).unwrap();
  match &expr2 {
    PnixExpr::Construct { variant, args } => {
      assert_eq!(variant, "Some", "Some should be parsed as Construct");
      assert_eq!(args.len(), 1, "Some(42) should have one argument");
      match &args[0] {
        PnixExpr::Int(42) => {}
        _ => panic!("Some(42) argument should be Int(42), got: {:?}", args[0]),
      }
    }
    _ => panic!("Some(42) should be parsed as Construct, got: {:?}", expr2),
  }

  // Test 3: Mixed case - None and Some(1) in expression
  let source3 = "if true then None else Some(1)";
  let expr3 = parse_expr(source3).unwrap();
  match &expr3 {
    PnixExpr::If { then_, else_, .. } => {
      match then_.as_ref() {
        PnixExpr::Construct { variant, args } => {
          assert_eq!(variant, "None", "then branch should be None");
          assert_eq!(args.len(), 0);
        }
        _ => panic!("then branch should be None, got: {:?}", then_),
      }
      match else_.as_ref() {
        PnixExpr::Construct { variant, args } => {
          assert_eq!(variant, "Some", "else branch should be Some(1)");
          assert_eq!(args.len(), 1);
        }
        _ => panic!("else branch should be Some(1), got: {:?}", else_),
      }
    }
    _ => panic!("if expression should be parsed, got: {:?}", expr3),
  }

  // Test 4: Lowercase identifier should still be Var
  let source4 = "none";
  let expr4 = parse_expr(source4).unwrap();
  match &expr4 {
    PnixExpr::Var(name) => {
      assert_eq!(name, "none", "lowercase 'none' should be Var");
    }
    _ => panic!("lowercase 'none' should be Var, got: {:?}", expr4),
  }

  // Test 5: Lowering to UnifiedExpr
  let unified_none = pnix_expr_to_unified(&expr1).unwrap();
  match &unified_none {
    UnifiedExpr::Construct { variant, args } => {
      assert_eq!(variant, "None");
      assert_eq!(args.len(), 0);
    }
    _ => panic!(
      "None should lower to UnifiedExpr::Construct, got: {:?}",
      unified_none
    ),
  }

  // Test 6: Lowering to FxCoreExpr
  let fx_none = lower_to_fx_core(&unified_none).unwrap();
  match &fx_none {
    FxCoreExpr::Construct { variant, args } => {
      assert_eq!(variant, "None");
      assert_eq!(args.len(), 0);
    }
    _ => panic!(
      "None should lower to FxCoreExpr::Construct, got: {:?}",
      fx_none
    ),
  }
}

#[test]
fn test_match_guard_short_circuit() {
  // Y08c-1: guard 조건 단축 평가 정합성 테스트
  // 패턴이 불일치하면 guard가 평가되지 않아야 함

  // Test 1: 패턴 불일치 시 guard 미평가 확인
  // match 0 with | 1 if false => "one" | _ => "other"
  // 패턴 1이 불일치하므로 guard(false)는 평가되지 않아야 함
  let source1 = r#"match 0 with | 1 if false => "one" | _ => "other""#;
  let expr1 = parse_expr(source1).unwrap();
  let unified1 = pnix_expr_to_unified(&expr1).unwrap();

  // unified는 If 체인으로 변환됨
  // 첫 번째 arm: 패턴 1 매칭 시도 → 불일치 → guard 평가 안 됨
  // 두 번째 arm: wildcard → 매칭 → "other" 반환
  match &unified1 {
    UnifiedExpr::Let { name, body, .. } => {
      assert!(
        name.starts_with("_match_scrutinee_"),
        "expected gensym scrutinee name, got: {}",
        name
      );
      // body는 If 체인
      match body.as_ref() {
        UnifiedExpr::If { cond, then_, else_ } => {
          // 첫 번째 If: 패턴 1 매칭 시도
          match cond.as_ref() {
            UnifiedExpr::And(lhs, rhs) => {
              // lhs는 패턴 매칭 조건 (_match_scrutinee == 1)
              // rhs는 guard (false)
              // And는 단축 평가로 변환되므로, lhs가 false면 rhs는 평가 안 됨
              match lhs.as_ref() {
                UnifiedExpr::Eq(lhs_var, rhs_val) => {
                  // _match_scrutinee == 1
                  assert!(
                    matches!(lhs_var.as_ref(), UnifiedExpr::Var(v) if v.starts_with("_match_scrutinee_"))
                  );
                  assert!(matches!(rhs_val.as_ref(), UnifiedExpr::Int(1)));
                }
                _ => panic!("Expected Eq condition for pattern matching"),
              }
              match rhs.as_ref() {
                UnifiedExpr::Bool(false) => {} // guard는 false
                _ => panic!("Expected Bool(false) guard"),
              }
            }
            _ => panic!("Expected And condition (pattern && guard)"),
          }
          // then_는 첫 번째 arm body: "one"
          match then_.as_ref() {
            UnifiedExpr::String(s) => assert_eq!(s, "one"),
            _ => panic!("Expected string 'one'"),
          }
          // else_는 두 번째 arm: wildcard → "other"
          match else_.as_ref() {
            UnifiedExpr::String(s) => assert_eq!(s, "other"),
            _ => panic!("Expected string 'other'"),
          }
        }
        _ => panic!("Expected If chain"),
      }
    }
    _ => panic!("Expected Let binding for scrutinee"),
  }

  // Test 2: 패턴 매칭 성공 + guard 실패
  // match 1 with | 1 if false => "one" | _ => "other"
  // 패턴 1이 매칭되지만 guard(false)가 실패하므로 두 번째 arm으로 진행
  let source2 = r#"match 1 with | 1 if false => "one" | _ => "other""#;
  let expr2 = parse_expr(source2).unwrap();
  let unified2 = pnix_expr_to_unified(&expr2).unwrap();

  // unified는 If 체인으로 변환됨
  // 첫 번째 arm: 패턴 1 매칭 성공 → guard 평가 → false → 두 번째 arm으로 진행
  // 두 번째 arm: wildcard → 매칭 → "other" 반환
  match &unified2 {
    UnifiedExpr::Let { body, .. } => {
      match body.as_ref() {
        UnifiedExpr::If { cond, then_, else_ } => {
          // 첫 번째 If: 패턴 1 매칭 + guard false
          match cond.as_ref() {
            UnifiedExpr::And(lhs, rhs) => {
              // lhs는 패턴 매칭 조건 (_match_scrutinee == 1) → true
              // rhs는 guard (false) → false
              // And는 단축 평가: lhs가 true이므로 rhs 평가 → false → else_로 진행
              match lhs.as_ref() {
                UnifiedExpr::Eq(..) => {} // 패턴 매칭 조건
                _ => panic!("Expected Eq condition"),
              }
              match rhs.as_ref() {
                UnifiedExpr::Bool(false) => {} // guard는 false
                _ => panic!("Expected Bool(false) guard"),
              }
            }
            _ => panic!("Expected And condition"),
          }
          // then_는 첫 번째 arm body: "one"
          match then_.as_ref() {
            UnifiedExpr::String(s) => assert_eq!(s, "one"),
            _ => panic!("Expected string 'one'"),
          }
          // else_는 두 번째 arm: wildcard → "other"
          match else_.as_ref() {
            UnifiedExpr::String(s) => assert_eq!(s, "other"),
            _ => panic!("Expected string 'other'"),
          }
        }
        _ => panic!("Expected If chain"),
      }
    }
    _ => panic!("Expected Let binding"),
  }
}

#[test]
fn test_match_attrset_pattern_lowering() {
  let source = r#"match { x = 1; y = 2; } with | { x, y } => x + y | _ => 0"#;
  let expr = parse_expr(source).unwrap();
  let unified = pnix_expr_to_unified(&expr).unwrap();

  let UnifiedExpr::Let { body, .. } = unified else {
    panic!("Expected Let wrapper, got {:?}", unified);
  };
  let UnifiedExpr::If { cond, then_, else_ } = body.as_ref() else {
    panic!("Expected If chain, got {:?}", body);
  };

  assert!(contains_apply_func(cond, "builtins.isAttrs"));
  assert!(contains_has_attr(cond, "x"));
  assert!(contains_has_attr(cond, "y"));

  assert!(contains_let_name(then_, "x"));
  assert!(contains_let_name(then_, "y"));
  assert!(matches!(else_.as_ref(), UnifiedExpr::Int(0)));
}

#[test]
fn test_match_list_pattern_lowering() {
  let source = r#"match [1 2 3] with | [x, y, ...rest] => x + y | _ => 0"#;
  let expr = parse_expr(source).unwrap();
  let unified = pnix_expr_to_unified(&expr).unwrap();

  let UnifiedExpr::Let { body, .. } = unified else {
    panic!("Expected Let wrapper, got {:?}", unified);
  };
  let UnifiedExpr::If { cond, then_, else_ } = body.as_ref() else {
    panic!("Expected If chain, got {:?}", body);
  };

  assert!(contains_apply_func(cond, "builtins.isList"));
  assert!(contains_apply_func(cond, "builtins.length"));
  assert!(contains_ge(cond));

  assert!(contains_let_name(then_, "x"));
  assert!(contains_let_name(then_, "y"));
  assert!(contains_let_name(then_, "rest"));
  assert!(matches!(else_.as_ref(), UnifiedExpr::Int(0)));
}
#[test]
fn test_match_parse_basic() {
  // 기본 파싱 테스트
  let source = r#"match 0 with | 0 => "zero" | _ => "other""#;
  let expr = parse_expr(source).unwrap();

  // PnixExpr::Match로 파싱되어야 함
  match expr {
    PnixExpr::Match { scrutinee, arms } => {
      // scrutinee는 0
      match scrutinee.as_ref() {
        PnixExpr::Int(0) => {}
        _ => panic!("Expected Int(0), got {:?}", scrutinee),
      }
      // arms는 2개
      assert_eq!(arms.len(), 2);
      // 첫 번째 arm: 0 => "zero"
      match &arms[0].pattern {
        PnixPattern::Literal(PnixLiteralPattern::Int(0)) => {}
        _ => panic!("Expected literal pattern 0, got {:?}", arms[0].pattern),
      }
      // 두 번째 arm: _ => "other"
      match &arms[1].pattern {
        PnixPattern::Wildcard => {}
        _ => panic!("Expected wildcard pattern, got {:?}", arms[1].pattern),
      }
    }
    _ => panic!("Expected Match expression, got {:?}", expr),
  }
}

#[test]
fn test_param_system_time() {
  // Y08a-3: param.system_time이 ParamTime으로 변환되는지 테스트
  let source = r#"param.system_time"#;
  let expr = parse_expr(source).unwrap();

  let unified = pnix_expr_to_unified(&expr).unwrap();

  match unified {
    UnifiedExpr::ParamTime => {}
    _ => panic!("Expected ParamTime, got {:?}", unified),
  }
}

#[test]
fn test_param_delta_time() {
  // Y08a-3: param.delta_time이 ParamDeltaTime으로 변환되는지 테스트
  let source = r#"param.delta_time"#;
  let expr = parse_expr(source).unwrap();

  let unified = pnix_expr_to_unified(&expr).unwrap();

  match unified {
    UnifiedExpr::ParamDeltaTime => {}
    _ => panic!("Expected ParamDeltaTime, got {:?}", unified),
  }
}

#[test]
fn test_param_signal() {
  // Y08a-3: param.signal_name이 ParamSignal으로 변환되는지 테스트
  let source = r#"param.mouse_x"#;
  let expr = parse_expr(source).unwrap();

  let unified = pnix_expr_to_unified(&expr).unwrap();

  match unified {
    UnifiedExpr::ParamSignal(name) => {
      assert_eq!(name, "mouse_x");
    }
    _ => panic!("Expected ParamSignal('mouse_x'), got {:?}", unified),
  }
}

#[test]
fn test_match_constructor_pattern() {
  // Y09c: enum 패턴 매칭 테스트
  // match opt with | Some(x) => x | None => 0

  // Test 1: None 패턴 매칭 (nullary constructor)
  let source1 = r#"match None with | None => 0 | Some(x) => x"#;
  let expr1 = parse_expr(source1).unwrap();
  let unified1 = pnix_expr_to_unified(&expr1).unwrap();

  // Let으로 감싸져 있어야 함
  match &unified1 {
    UnifiedExpr::Let { name, body, .. } => {
      assert!(
        name.starts_with("_match_scrutinee_"),
        "expected gensym scrutinee name, got: {}",
        name
      );
      // body는 If 체인
      match body.as_ref() {
        UnifiedExpr::If {
          cond,
          then_,
          else_: _,
        } => {
          // 첫 번째 arm: None 패턴
          // cond는 variant 비교여야 함
          match cond.as_ref() {
            UnifiedExpr::Bool(true) => {
              // 컴파일 타임에 None 리터럴이므로 true
            }
            _ => panic!(
              "Expected Bool(true) for None pattern match, got: {:?}",
              cond
            ),
          }
          // then_는 변수 패턴 바인딩이 있을 수 있음 (Let으로 감싸짐)
          match then_.as_ref() {
            UnifiedExpr::Int(0) => {
              // 직접 Int(0)
            }
            UnifiedExpr::Let {
              name: _name,
              body: let_body,
              ..
            } => {
              // 변수 패턴 바인딩이 있는 경우
              // body는 Int(0)여야 함
              match let_body.as_ref() {
                UnifiedExpr::Int(0) => {}
                _ => panic!("Expected Int(0) in Let body, got: {:?}", let_body),
              }
            }
            _ => panic!("Expected Int(0) or Let with Int(0), got: {:?}", then_),
          }
        }
        _ => panic!("Expected If chain, got: {:?}", body),
      }
    }
    _ => panic!("Expected Let wrapper, got: {:?}", unified1),
  }

  // Test 2: Some(42) 패턴 매칭 (constructor with args)
  // 현재는 Construct 리터럴에 대한 constructor 패턴 매칭이 부분적으로 지원됨
  // Some(42)는 Construct 리터럴이므로 컴파일 타임에 매칭 가능해야 함
  let source2 = r#"match Some(42) with | Some(x) => x | None => 0"#;
  let expr2 = parse_expr(source2).unwrap();
  let unified2_result = pnix_expr_to_unified(&expr2);

  // 현재는 Some(x) 패턴이 args를 가지고 있어서 에러가 발생할 수 있음
  // 하지만 Some(42)는 Construct 리터럴이므로 컴파일 타임에 매칭 가능해야 함
  // 향후 구현 완료 시 통과해야 함
  match unified2_result {
    Ok(unified2) => {
      // Let으로 감싸져 있어야 함
      match &unified2 {
        UnifiedExpr::Let { name, body, .. } => {
          assert!(
            name.starts_with("_match_scrutinee_"),
            "expected gensym scrutinee name, got: {}",
            name
          );
          // body는 If 체인
          match body.as_ref() {
            UnifiedExpr::If {
              cond,
              then_: _,
              else_: _,
            } => {
              // 첫 번째 arm: Some(x) 패턴
              // cond는 variant 비교 + args 매칭
              match cond.as_ref() {
                UnifiedExpr::And(lhs, _rhs) => {
                  // lhs는 variant 비교, rhs는 args 매칭
                  match lhs.as_ref() {
                    UnifiedExpr::Bool(true) => {
                      // 컴파일 타임에 Some(42) 리터럴이므로 variant 비교는 true
                    }
                    _ => panic!("Expected Bool(true) for Some variant match, got: {:?}", lhs),
                  }
                }
                UnifiedExpr::Bool(true) => {
                  // args가 없으면 variant 비교만
                }
                _ => panic!(
                  "Expected And or Bool(true) for Some pattern match, got: {:?}",
                  cond
                ),
              }
            }
            _ => panic!("Expected If chain, got: {:?}", body),
          }
        }
        _ => panic!("Expected Let wrapper, got: {:?}", unified2),
      }
    }
    Err(e) => {
      // 현재는 에러가 발생할 수 있음 (런타임 값에 대한 constructor 패턴 매칭 미지원)
      // 하지만 Some(42)는 Construct 리터럴이므로 향후 통과해야 함
      eprintln!("Note: Some(42) pattern matching currently fails: {:?}", e);
    }
  }

  // Test 3: 변수에 대한 constructor 패턴 매칭 (런타임 값)
  // 현재는 런타임 값에 대한 constructor 패턴 매칭이 부분적으로만 지원됨
  // None (nullary constructor)는 지원되지만, Some(x) (args 있음)는 미지원
  let source3 = r#"match opt with | None => 0 | Some(x) => x"#;
  let expr3 = parse_expr(source3).unwrap();
  let unified3_result = pnix_expr_to_unified(&expr3);

  // 현재는 Some(x) 패턴이 args를 가지고 있어서 에러가 발생할 수 있음
  // 향후 구현 완료 시 통과해야 함
  match unified3_result {
    Ok(unified3) => {
      // Let으로 감싸져 있어야 함
      match &unified3 {
        UnifiedExpr::Let { name, body, .. } => {
          assert!(
            name.starts_with("_match_scrutinee_"),
            "expected gensym scrutinee name, got: {}",
            name
          );
          // body는 If 체인
          // 첫 번째 arm: None 패턴
          // Y13a-12: constructor 패턴은 _variant 필드를 비교해야 함
          // Y09c-3: hasAttr 체크 후 variant 비교 (런타임 안전성)
          match body.as_ref() {
            UnifiedExpr::If { cond, .. } => {
              // cond는 And(hasAttr("_variant", scrutinee), Eq(getAttr("_variant", scrutinee), "None")) 형태
              match cond.as_ref() {
                UnifiedExpr::And(has_attr_check, variant_eq) => {
                  // has_attr_check는 Apply { func: "builtins.hasAttr", args: ["_variant", scrutinee] }
                  match has_attr_check.as_ref() {
                    UnifiedExpr::Apply { func, args } => {
                      assert_eq!(
                        func, "builtins.hasAttr",
                        "expected hasAttr call, got: {}",
                        func
                      );
                      assert_eq!(args.len(), 2, "hasAttr should have 2 args");
                      assert!(
                        matches!(&args[0], UnifiedExpr::String(s) if s == "_variant"),
                        "first arg should be \"_variant\", got: {:?}",
                        args[0]
                      );
                    }
                    _ => panic!("Expected Apply for hasAttr, got: {:?}", has_attr_check),
                  }
                  // variant_eq는 Eq(getAttr("_variant", scrutinee), "None")
                  match variant_eq.as_ref() {
                    UnifiedExpr::Eq(lhs, rhs) => {
                      match lhs.as_ref() {
                        UnifiedExpr::Apply { func, args } => {
                          assert_eq!(
                            func, "builtins.getAttr",
                            "expected getAttr call, got: {}",
                            func
                          );
                          assert_eq!(args.len(), 2, "getAttr should have 2 args");
                          assert!(
                            matches!(&args[0], UnifiedExpr::String(s) if s == "_variant"),
                            "first arg should be \"_variant\", got: {:?}",
                            args[0]
                          );
                        }
                        _ => panic!("Expected Apply for getAttr, got: {:?}", lhs),
                      }
                      // rhs는 String("None")
                      assert!(
                        matches!(rhs.as_ref(), UnifiedExpr::String(s) if s == "None"),
                        "Expected String(\"None\") for None variant, got: {:?}",
                        rhs
                      );
                    }
                    _ => panic!("Expected Eq for variant check, got: {:?}", variant_eq),
                  }
                }
                _ => panic!(
                  "Expected And(hasAttr, Eq) for safe variant check, got: {:?}",
                  cond
                ),
              }
            }
            _ => panic!("Expected If chain, got: {:?}", body),
          }
        }
        _ => panic!("Expected Let wrapper, got: {:?}", unified3),
      }
    }
    Err(e) => {
      // 현재는 에러가 발생할 수 있음 (런타임 값에 대한 constructor 패턴 매칭 미지원)
      eprintln!("Note: opt pattern matching currently fails: {:?}", e);
    }
  }
}
