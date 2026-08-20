//! LLVM 부동소수점 연산 테스트: LLVM JIT를 사용한 부동소수점 연산 테스트
//!
//! LLVM JIT 엔진을 사용하여 부동소수점 연산이 올바르게 실행되는지 검증합니다.

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
fn assert_close(actual: f64, expected: f64) {
  let diff = (actual - expected).abs();
  assert!(diff < 1e-9, "expected {}, got {}", expected, actual);
}

#[test]
#[cfg(feature = "llvm")]
fn test_float_constant_add() {
  let fx_module = FxCoreModule {
    meta: FxCoreMeta::default(),
    name: "test_float_add".to_string(),
    types: Vec::new(),
    adt_types: Vec::new(),
    adttypes: Vec::new(),
    inputs: Vec::new(),
    morphisms: vec![binary_morphism("add", "Float", "Float", Effect::Pure)],
    nodes: vec![FxNode {
      name: "result".to_string(),
      uses: "add".to_string(),
      meta: None,
      ..Default::default()
    }],
    edges: binary_input_edges("1.5", "2.25", "result"),
    scopes: Vec::new(),
  };

  let ir_json = serde_json::to_vec(&fx_module).unwrap();
  let mut engine = JitEngine::new();
  let module = engine.compile("test_float_add", &ir_json).unwrap();
  let eval_result = engine
    .eval(&module, &EvalConfig::default())
    .expect("jit eval");
  let result_json: serde_json::Value = serde_json::from_slice(&eval_result.value.data).unwrap();
  let value = result_json.as_f64().unwrap();
  assert_close(value, 3.75);
}

#[test]
#[cfg(feature = "llvm")]
fn test_float_input_add() {
  let fx_module = FxCoreModule {
    meta: FxCoreMeta::default(),
    name: "test_float_input_add".to_string(),
    types: Vec::new(),
    adt_types: Vec::new(),
    adttypes: Vec::new(),
    inputs: vec![
      FxInput {
        name: "x".to_string(),
        ty: "Float".to_string(),
      },
      FxInput {
        name: "y".to_string(),
        ty: "Float".to_string(),
      },
    ],
    morphisms: vec![binary_morphism("add", "Float", "Float", Effect::Pure)],
    nodes: vec![FxNode {
      name: "result".to_string(),
      uses: "add".to_string(),
      meta: None,
      ..Default::default()
    }],
    edges: binary_input_edges("x", "y", "result"),
    scopes: Vec::new(),
  };

  let ir_json = serde_json::to_vec(&fx_module).unwrap();
  let mut engine = JitEngine::new();
  let module = engine.compile("test_float_input_add", &ir_json).unwrap();
  let inputs = serde_json::json!({ "x": 2.0, "y": 4.5 }).to_string();
  let eval_result = engine
    .execute_with_inputs(&module, &EvalConfig::default(), inputs.as_bytes())
    .expect("jit eval");
  let result_json: serde_json::Value = serde_json::from_slice(&eval_result.value.data).unwrap();
  let value = result_json.as_f64().unwrap();
  assert_close(value, 6.5);
}

#[test]
#[cfg(feature = "llvm")]
fn test_float_unary_sqrt() {
  let fx_module = FxCoreModule {
    meta: FxCoreMeta::default(),
    name: "test_float_sqrt".to_string(),
    types: Vec::new(),
    adt_types: Vec::new(),
    adttypes: Vec::new(),
    inputs: Vec::new(),
    morphisms: vec![FxMorphism::simple(
      "sqrt".to_string(),
      "Float".to_string(),
      "Float".to_string(),
      Effect::Pure,
    )],
    nodes: vec![FxNode {
      name: "result".to_string(),
      uses: "sqrt".to_string(),
      meta: None,
      ..Default::default()
    }],
    edges: vec![FxEdge {
      from: "input".to_string(),
      to: "result".to_string(),
      from_input: Some("9.0".to_string()),
      from_port: None,
      to_port: None,
      cond: None,
    }],
    scopes: Vec::new(),
  };

  let ir_json = serde_json::to_vec(&fx_module).unwrap();
  let mut engine = JitEngine::new();
  let module = engine.compile("test_float_sqrt", &ir_json).unwrap();
  let eval_result = engine
    .eval(&module, &EvalConfig::default())
    .expect("jit eval");
  let result_json: serde_json::Value = serde_json::from_slice(&eval_result.value.data).unwrap();
  let value = result_json.as_f64().unwrap();
  assert_close(value, 3.0);
}
