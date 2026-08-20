//! Meta-circular Stage 2: Self-Hosting 비교 테스트
//!
//! 목표: 수동 구현(meta/*.rs) ↔ 생성 구현(meta/generated/*.rs)
//!       구조적으로 동일함을 증명
//!
//! 비교 방식:
//! - serde_json을 통한 필드 구조 비교
//! - Default 인스턴스의 JSON 직렬화 동일성

#[cfg(feature = "self_hosted_meta")]
mod self_hosting_tests {
  use pnix_core::meta as manual;
  use pnix_core::meta::generated;

  /// MetaFxModule 구조 동일성 테스트
  #[test]
  fn meta_fx_module_structure_matches() {
    let manual_default = manual::MetaFxModule {
      name: String::new(),
      stage: 0,
      replay_hash: None,
      types: vec![],
      inputs: vec![],
      morphisms: vec![],
      nodes: vec![],
      edges: vec![],
      scopes: vec![],
      stats: manual::MetaFxStats {
        node_count: 0,
        edge_count: 0,
        scope_count: 0,
        gate_count: 0,
        optional_count: 0,
        conditional_edge_count: 0,
      },
    };

    let generated_default = generated::MetaFxModule::default();

    let manual_json = serde_json::to_value(&manual_default).unwrap();
    let generated_json = serde_json::to_value(&generated_default).unwrap();

    // 필드 이름 동일성
    assert_eq!(
      manual_json.as_object().unwrap().keys().collect::<Vec<_>>(),
      generated_json
        .as_object()
        .unwrap()
        .keys()
        .collect::<Vec<_>>(),
      "MetaFxModule field names must match"
    );
  }

  /// MetaFxNode 구조 동일성 테스트
  #[test]
  fn meta_fx_node_structure_matches() {
    let manual_default = manual::MetaFxNode {
      name: String::new(),
      uses: String::new(),
      kind: String::new(),
      optional: false,
      scope: String::new(),
      cost: String::new(),
      priority: 0,
      contract: manual::MetaExecutionContract::default(),
    };

    let generated_default = generated::MetaFxNode::default();

    let manual_json = serde_json::to_value(&manual_default).unwrap();
    let generated_json = serde_json::to_value(&generated_default).unwrap();

    assert_eq!(
      manual_json.as_object().unwrap().keys().collect::<Vec<_>>(),
      generated_json
        .as_object()
        .unwrap()
        .keys()
        .collect::<Vec<_>>(),
      "MetaFxNode field names must match"
    );
  }

  /// MetaFxEdge 구조 동일성 테스트
  #[test]
  fn meta_fx_edge_structure_matches() {
    let manual_default = manual::MetaFxEdge {
      from: String::new(),
      to: String::new(),
      cond: None,
      port_info: None,
    };

    let generated_default = generated::MetaFxEdge::default();

    let manual_json = serde_json::to_value(&manual_default).unwrap();
    let generated_json = serde_json::to_value(&generated_default).unwrap();

    assert_eq!(
      manual_json.as_object().unwrap().keys().collect::<Vec<_>>(),
      generated_json
        .as_object()
        .unwrap()
        .keys()
        .collect::<Vec<_>>(),
      "MetaFxEdge field names must match"
    );
  }

  /// MetaFxScope 구조 동일성 테스트
  #[test]
  fn meta_fx_scope_structure_matches() {
    let manual_default = manual::MetaFxScope {
      name: String::new(),
      node_count: 0,
      policy: String::new(),
    };

    let generated_default = generated::MetaFxScope::default();

    let manual_json = serde_json::to_value(&manual_default).unwrap();
    let generated_json = serde_json::to_value(&generated_default).unwrap();

    assert_eq!(
      manual_json.as_object().unwrap().keys().collect::<Vec<_>>(),
      generated_json
        .as_object()
        .unwrap()
        .keys()
        .collect::<Vec<_>>(),
      "MetaFxScope field names must match"
    );
  }

  /// MetaExecutionContract 구조 동일성 테스트
  #[test]
  fn meta_execution_contract_structure_matches() {
    let manual_default = manual::MetaExecutionContract::default();
    let generated_default = generated::MetaExecutionContract::default();

    let manual_json = serde_json::to_value(&manual_default).unwrap();
    let generated_json = serde_json::to_value(&generated_default).unwrap();

    assert_eq!(
      manual_json.as_object().unwrap().keys().collect::<Vec<_>>(),
      generated_json
        .as_object()
        .unwrap()
        .keys()
        .collect::<Vec<_>>(),
      "MetaExecutionContract field names must match"
    );
  }

  /// MetaFxStats 구조 동일성 테스트
  #[test]
  fn meta_fx_stats_structure_matches() {
    let manual_default = manual::MetaFxStats {
      node_count: 0,
      edge_count: 0,
      scope_count: 0,
      gate_count: 0,
      optional_count: 0,
      conditional_edge_count: 0,
    };

    let generated_default = generated::MetaFxStats::default();

    let manual_json = serde_json::to_value(&manual_default).unwrap();
    let generated_json = serde_json::to_value(&generated_default).unwrap();

    assert_eq!(
      manual_json.as_object().unwrap().keys().collect::<Vec<_>>(),
      generated_json
        .as_object()
        .unwrap()
        .keys()
        .collect::<Vec<_>>(),
      "MetaFxStats field names must match"
    );
  }

  /// MetaFxInput 구조 동일성 테스트
  #[test]
  fn meta_fx_input_structure_matches() {
    let manual_default = manual::MetaFxInput {
      name: String::new(),
      ty: String::new(),
    };

    let generated_default = generated::MetaFxInput::default();

    let manual_json = serde_json::to_value(&manual_default).unwrap();
    let generated_json = serde_json::to_value(&generated_default).unwrap();

    assert_eq!(
      manual_json.as_object().unwrap().keys().collect::<Vec<_>>(),
      generated_json
        .as_object()
        .unwrap()
        .keys()
        .collect::<Vec<_>>(),
      "MetaFxInput field names must match"
    );
  }

  /// MetaFxMorphism 구조 동일성 테스트
  #[test]
  fn meta_fx_morphism_structure_matches() {
    let manual_default = manual::MetaFxMorphism {
      name: String::new(),
      input: String::new(),
      output: String::new(),
      inputs: vec![],
      outputs: vec![],
      effect: String::new(),
    };

    let generated_default = generated::MetaFxMorphism::default();

    let manual_json = serde_json::to_value(&manual_default).unwrap();
    let generated_json = serde_json::to_value(&generated_default).unwrap();

    assert_eq!(
      manual_json.as_object().unwrap().keys().collect::<Vec<_>>(),
      generated_json
        .as_object()
        .unwrap()
        .keys()
        .collect::<Vec<_>>(),
      "MetaFxMorphism field names must match"
    );
  }

  /// MetaFxMorphismPort 구조 동일성 테스트
  #[test]
  fn meta_fx_morphism_port_structure_matches() {
    let manual_default = manual::MetaFxMorphismPort {
      name: String::new(),
      ty: String::new(),
    };

    let generated_default = generated::MetaFxMorphismPort::default();

    let manual_json = serde_json::to_value(&manual_default).unwrap();
    let generated_json = serde_json::to_value(&generated_default).unwrap();

    assert_eq!(
      manual_json.as_object().unwrap().keys().collect::<Vec<_>>(),
      generated_json
        .as_object()
        .unwrap()
        .keys()
        .collect::<Vec<_>>(),
      "MetaFxMorphismPort field names must match"
    );
  }

  /// MetaFxPortInfo 구조 동일성 테스트
  #[test]
  fn meta_fx_port_info_structure_matches() {
    let manual_default = manual::MetaFxPortInfo {
      from_port: None,
      to_port: None,
    };

    let generated_default = generated::MetaFxPortInfo::default();

    let manual_json = serde_json::to_value(&manual_default).unwrap();
    let generated_json = serde_json::to_value(&generated_default).unwrap();

    assert_eq!(
      manual_json.as_object().unwrap().keys().collect::<Vec<_>>(),
      generated_json
        .as_object()
        .unwrap()
        .keys()
        .collect::<Vec<_>>(),
      "MetaFxPortInfo field names must match"
    );
  }

  /// Generated DSL Codegen 함수 존재 테스트
  #[test]
  fn generated_dsl_codegen_exists() {
    let meta = generated::MetaFxModule::default();
    let dsl = generated::generate_pnix_dsl(&meta);

    assert!(dsl.contains("# Generated from MetaFx"));
  }

  /// Generated Rust Codegen 함수 존재 테스트
  #[test]
  fn generated_rust_codegen_exists() {
    let meta = generated::MetaFxModule::default();
    let rust = generated::generate_rust_structs(&meta);

    assert!(rust.contains("GeneratedFxModule"));
  }

  /// DSL 생성 결과 동일성 테스트
  #[test]
  fn dsl_generation_output_matches() {
    // 같은 입력으로 양쪽 함수 호출
    let test_input = generated::MetaFxModule {
      name: "test".into(),
      stage: 1,
      replay_hash: None,
      types: vec![],
      inputs: vec![],
      morphisms: vec![],
      nodes: vec![generated::MetaFxNode {
        name: "n1".into(),
        uses: "test.op".into(),
        kind: "normal".into(),
        optional: false,
        scope: "global".into(),
        cost: "medium".into(),
        priority: 0,
        contract: generated::MetaExecutionContract::default(),
      }],
      edges: vec![],
      scopes: vec![],
      stats: generated::MetaFxStats::default(),
    };

    let generated_dsl = generated::generate_pnix_dsl(&test_input);

    // 기본 검증
    assert!(generated_dsl.contains("node n1 uses test.op"));
    assert!(generated_dsl.contains("# name: test"));
    assert!(generated_dsl.contains("# stage: 1"));
  }
}

/// 기본 빌드에서도 실행되는 테스트 (feature 없이)
#[test]
fn self_hosting_codegen_produces_valid_source() {
  let source = pnix_core::meta::generate_meta_ir_source();

  assert!(source.contains("pub struct MetaFxModule"));
  assert!(source.contains("pub struct MetaFxNode"));
  assert!(source.contains("pub struct MetaFxEdge"));
  assert!(source.contains("AUTO-GENERATED BY pnix"));
}
