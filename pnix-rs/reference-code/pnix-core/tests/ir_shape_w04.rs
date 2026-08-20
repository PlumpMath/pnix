//! W04: spec 기반 검증 테스트

use pnix_core::contracts::effect::Effect;
use pnix_core::contracts::verify::verify_fxcore_with_spec;
use pnix_core::core::{FxCoreModule, FxMorphism, FxNode};
use pnix_core::diagnostics::Diagnostics;
use pnix_core::spec::Spec;

#[test]
fn test_builtin_node_success() {
  // builtin node 1개 이상 "성공" 케이스
  let spec = Spec::with_defaults();
  let mut diags = Diagnostics::default();

  let fx = FxCoreModule {
    meta: Default::default(),
    name: "test".to_string(),
    types: vec!["Num".to_string()],
    adt_types: vec![],
    adttypes: vec![],
    inputs: vec![],
    morphisms: vec![FxMorphism::simple(
      "add".to_string(),
      "Num".to_string(),
      "Num".to_string(),
      Effect::Pure,
    )],
    nodes: vec![FxNode {
      name: "node1".to_string(),
      uses: "add".to_string(),
      kind: pnix_core::core::NodeKind::Normal,
      optional: false,
      scope: "global".to_string(),
      cost: pnix_core::core::CostHint::Medium,
      priority: 0,
      contract: pnix_core::core::ExecutionContract {
        required_inputs: vec![],
        may_skip: false,
        skip_policy: pnix_core::core::SkipPolicy::Error,
        replay: None,
      },

      meta: None,
    }],
    edges: vec![],
    scopes: vec![],
  };

  let report = verify_fxcore_with_spec(&fx, &mut diags, &spec).unwrap();
  if !report.ok {
    eprintln!("Verification failed: {:?}", report);
    eprintln!("Diagnostics: {:?}", diags);
  }
  assert!(report.ok);
}

#[test]
fn test_unknown_builtin_fails() {
  // unknown builtin "실패" 케이스
  let spec = Spec::with_defaults();
  let mut diags = Diagnostics::default();

  let fx = FxCoreModule {
    meta: Default::default(),
    name: "test".to_string(),
    types: vec![],
    adt_types: vec![],
    adttypes: vec![],
    inputs: vec![],
    morphisms: vec![],
    nodes: vec![FxNode {
      name: "node1".to_string(),
      uses: "unknown_builtin_xyz".to_string(),
      kind: pnix_core::core::NodeKind::Normal,
      optional: false,
      scope: "global".to_string(),
      cost: pnix_core::core::CostHint::Medium,
      priority: 0,
      contract: pnix_core::core::ExecutionContract {
        required_inputs: vec![],
        may_skip: false,
        skip_policy: pnix_core::core::SkipPolicy::Error,
        replay: None,
      },

      meta: None,
    }],
    edges: vec![],
    scopes: vec![],
  };

  let result = verify_fxcore_with_spec(&fx, &mut diags, &spec);
  assert!(result.is_err());
  assert!(result
    .unwrap_err()
    .to_string()
    .contains("unknown builtin used in node"));
}

#[test]
fn test_unknown_type_fails() {
  // unknown type "실패" 케이스
  let spec = Spec::with_defaults();
  let mut diags = Diagnostics::default();

  let fx = FxCoreModule {
    meta: Default::default(),
    name: "test".to_string(),
    types: vec![],
    adt_types: vec![],
    adttypes: vec![],
    inputs: vec![pnix_core::core::FxInput {
      name: "x".to_string(),
      ty: "UnknownTypeXYZ".to_string(),
    }],
    morphisms: vec![],
    nodes: vec![],
    edges: vec![],
    scopes: vec![],
  };

  let result = verify_fxcore_with_spec(&fx, &mut diags, &spec);
  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("unknown type"));
}

#[test]
fn test_user_defined_type_allowed() {
  // 사용자 정의 타입 허용 케이스
  let spec = Spec::with_defaults();
  let mut diags = Diagnostics::default();

  let fx = FxCoreModule {
    meta: Default::default(),
    name: "test".to_string(),
    types: vec!["MyCustomType".to_string()],
    adt_types: vec![],
    adttypes: vec![],
    inputs: vec![pnix_core::core::FxInput {
      name: "x".to_string(),
      ty: "MyCustomType".to_string(),
    }],
    morphisms: vec![],
    nodes: vec![],
    edges: vec![],
    scopes: vec![],
  };

  let report = verify_fxcore_with_spec(&fx, &mut diags, &spec).unwrap();
  if !report.ok {
    eprintln!("Verification failed: {:?}", report);
    eprintln!("Diagnostics: {:?}", diags);
  }
  assert!(report.ok);
}
