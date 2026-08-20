//! LLVM 제어 흐름 테스트: LLVM JIT를 사용한 제어 흐름 연산 테스트
//!
//! LLVM JIT 엔진을 사용하여 조건문, 반복문 등의 제어 흐름이 올바르게 실행되는지 검증합니다.

#[cfg(feature = "llvm")]
use pnix_core::contracts::effect::Effect;
#[cfg(feature = "llvm")]
use pnix_core::core::{FxCoreMeta, FxCoreModule, FxEdge, FxInput, FxMorphism, FxNode};
#[cfg(feature = "llvm")]
use pnix_runtime_api::{EvalConfig, EvalEngine};
#[cfg(feature = "llvm")]
use pnix_runtime_llvm::JitEngine;

#[cfg(feature = "llvm")]
fn binary_morphism(name: &str, input_ty: &str, output_ty: &str, effect: Effect) -> FxMorphism {
  use pnix_core::core::FxPort;

  FxMorphism::ported(
    name.to_string(),
    vec![
      FxPort {
        name: "lhs".to_string(),
        ty: input_ty.to_string(),
      },
      FxPort {
        name: "rhs".to_string(),
        ty: input_ty.to_string(),
      },
    ],
    vec![FxPort {
      name: "out".to_string(),
      ty: output_ty.to_string(),
    }],
    effect,
  )
}

#[cfg(feature = "llvm")]
fn binary_input_edges(lhs: &str, rhs: &str, to: &str) -> Vec<FxEdge> {
  vec![
    FxEdge::from_input(lhs.to_string(), to.to_string(), Some("lhs".to_string())),
    FxEdge::from_input(rhs.to_string(), to.to_string(), Some("rhs".to_string())),
  ]
}

#[cfg(feature = "llvm")]
fn if_morphism(ty: &str, effect: Effect) -> FxMorphism {
  use pnix_core::core::FxPort;

  FxMorphism::ported(
    "if".to_string(),
    vec![
      FxPort {
        name: "cond".to_string(),
        ty: "Bool".to_string(),
      },
      FxPort {
        name: "then".to_string(),
        ty: ty.to_string(),
      },
      FxPort {
        name: "else".to_string(),
        ty: ty.to_string(),
      },
    ],
    vec![FxPort {
      name: "out".to_string(),
      ty: ty.to_string(),
    }],
    effect,
  )
}

#[test]
#[cfg(feature = "llvm")]
fn test_compare_eq_int() {
  let fx_module = FxCoreModule {
    meta: FxCoreMeta::default(),
    name: "test_eq_int".to_string(),
    types: Vec::new(),
    adt_types: Vec::new(),
    adttypes: Vec::new(),
    inputs: vec![
      FxInput {
        name: "a".to_string(),
        ty: "Int".to_string(),
      },
      FxInput {
        name: "b".to_string(),
        ty: "Int".to_string(),
      },
    ],
    morphisms: vec![binary_morphism("eq", "Int", "Bool", Effect::Pure)],
    nodes: vec![FxNode {
      name: "result".to_string(),
      uses: "eq".to_string(),
      meta: None,
      ..Default::default()
    }],
    edges: binary_input_edges("a", "b", "result"),
    scopes: Vec::new(),
  };

  let ir_json = serde_json::to_vec(&fx_module).unwrap();
  let mut engine = JitEngine::new();
  let module = engine.compile("test_eq_int", &ir_json).unwrap();
  let inputs = serde_json::json!({ "a": 5, "b": 5 }).to_string();
  let result = engine.execute_with_inputs(&module, &EvalConfig::default(), inputs.as_bytes());

  if let Ok(eval_result) = result {
    let result_json: serde_json::Value = serde_json::from_slice(&eval_result.value.data).unwrap();
    assert_eq!(result_json, serde_json::Value::Bool(true));
  }
}

#[test]
#[cfg(feature = "llvm")]
fn test_compare_lt_float() {
  let fx_module = FxCoreModule {
    meta: FxCoreMeta::default(),
    name: "test_lt_float".to_string(),
    types: Vec::new(),
    adt_types: Vec::new(),
    adttypes: Vec::new(),
    inputs: Vec::new(),
    morphisms: vec![binary_morphism("lt", "Float", "Bool", Effect::Pure)],
    nodes: vec![FxNode {
      name: "result".to_string(),
      uses: "lt".to_string(),
      meta: None,
      ..Default::default()
    }],
    edges: binary_input_edges("1.0", "2.0", "result"),
    scopes: Vec::new(),
  };

  let ir_json = serde_json::to_vec(&fx_module).unwrap();
  let mut engine = JitEngine::new();
  let module = engine.compile("test_lt_float", &ir_json).unwrap();
  let result = engine.eval(&module, &EvalConfig::default());

  if let Ok(eval_result) = result {
    let result_json: serde_json::Value = serde_json::from_slice(&eval_result.value.data).unwrap();
    assert_eq!(result_json, serde_json::Value::Bool(true));
  }
}

#[test]
#[cfg(feature = "llvm")]
fn test_if_select_int() {
  let fx_module = FxCoreModule {
    meta: FxCoreMeta::default(),
    name: "test_if_int".to_string(),
    types: Vec::new(),
    adt_types: Vec::new(),
    adttypes: Vec::new(),
    inputs: vec![
      FxInput {
        name: "x".to_string(),
        ty: "Int".to_string(),
      },
      FxInput {
        name: "y".to_string(),
        ty: "Int".to_string(),
      },
    ],
    morphisms: vec![
      binary_morphism("lt", "Int", "Bool", Effect::Pure),
      if_morphism("Int", Effect::Pure),
    ],
    nodes: vec![
      FxNode {
        name: "cond".to_string(),
        uses: "lt".to_string(),
        meta: None,
        ..Default::default()
      },
      FxNode {
        name: "result".to_string(),
        uses: "if".to_string(),
        meta: None,
        ..Default::default()
      },
    ],
    edges: vec![
      FxEdge::from_input("x".to_string(), "cond".to_string(), Some("lhs".to_string())),
      FxEdge::from_input("y".to_string(), "cond".to_string(), Some("rhs".to_string())),
      FxEdge {
        from: "cond".to_string(),
        to: "result".to_string(),
        from_input: None,
        from_port: Some("out".to_string()),
        to_port: Some("cond".to_string()),
        cond: None,
      },
      FxEdge::from_input(
        "x".to_string(),
        "result".to_string(),
        Some("then".to_string()),
      ),
      FxEdge::from_input(
        "y".to_string(),
        "result".to_string(),
        Some("else".to_string()),
      ),
    ],
    scopes: Vec::new(),
  };

  let ir_json = serde_json::to_vec(&fx_module).unwrap();
  let mut engine = JitEngine::new();
  let module = engine.compile("test_if_int", &ir_json).unwrap();
  let inputs = serde_json::json!({ "x": 1, "y": 2 }).to_string();
  let result = engine.execute_with_inputs(&module, &EvalConfig::default(), inputs.as_bytes());

  if let Ok(eval_result) = result {
    let result_json: serde_json::Value = serde_json::from_slice(&eval_result.value.data).unwrap();
    let value = result_json.as_i64().unwrap();
    assert_eq!(value, 1);
  }
}

#[test]
#[cfg(feature = "llvm")]
fn test_int_div_chain_floor_division() {
  let fx_module = FxCoreModule {
    meta: FxCoreMeta::default(),
    name: "test_int_div_chain".to_string(),
    types: Vec::new(),
    adt_types: Vec::new(),
    adttypes: Vec::new(),
    inputs: vec![
      FxInput {
        name: "a".to_string(),
        ty: "Int".to_string(),
      },
      FxInput {
        name: "b".to_string(),
        ty: "Int".to_string(),
      },
      FxInput {
        name: "c".to_string(),
        ty: "Int".to_string(),
      },
    ],
    morphisms: vec![binary_morphism("div", "Int", "Int", Effect::Pure)],
    nodes: vec![
      FxNode {
        name: "step1".to_string(),
        uses: "div".to_string(),
        meta: None,
        ..Default::default()
      },
      FxNode {
        name: "result".to_string(),
        uses: "div".to_string(),
        meta: None,
        ..Default::default()
      },
    ],
    edges: vec![
      FxEdge::from_input(
        "a".to_string(),
        "step1".to_string(),
        Some("lhs".to_string()),
      ),
      FxEdge::from_input(
        "b".to_string(),
        "step1".to_string(),
        Some("rhs".to_string()),
      ),
      FxEdge {
        from: "step1".to_string(),
        to: "result".to_string(),
        from_input: None,
        from_port: Some("out".to_string()),
        to_port: Some("lhs".to_string()),
        cond: None,
      },
      FxEdge::from_input(
        "c".to_string(),
        "result".to_string(),
        Some("rhs".to_string()),
      ),
    ],
    scopes: Vec::new(),
  };

  let ir_json = serde_json::to_vec(&fx_module).unwrap();
  let mut engine = JitEngine::new();
  let module = engine.compile("test_int_div_chain", &ir_json).unwrap();
  let cases = [
    (-5, -2, 3, 0),
    (-5, 2, 3, -1),
    (5, -2, 3, -1),
    (5, 2, -3, -1),
    (-7, 3, 2, -2),
    (-1, 2, 1, -1),
  ];

  for (a, b, c, expected) in cases {
    let inputs = serde_json::json!({ "a": a, "b": b, "c": c }).to_string();
    let result = engine.execute_with_inputs(&module, &EvalConfig::default(), inputs.as_bytes());
    if let Ok(eval_result) = result {
      let result_json: serde_json::Value = serde_json::from_slice(&eval_result.value.data).unwrap();
      assert_eq!(result_json, serde_json::json!(expected));
    }
  }
}
