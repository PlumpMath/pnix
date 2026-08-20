//! Proptest Patch 테스트: 속성 기반 테스트를 사용한 패치 기능 테스트
//!
//! Proptest를 사용하여 다양한 입력에 대한 패치 기능을 검증합니다.

#![cfg(feature = "proptest")]

use proptest::prelude::*;

use pnix_executor_graph::{apply_patch, FxCoreMeta, FxCoreModule, FxCorePatch, FxNode, PatchOp};

fn proptest_config() -> ProptestConfig {
  ProptestConfig {
    failure_persistence: None,
    ..ProptestConfig::default()
  }
}

fn base_module() -> FxCoreModule {
  FxCoreModule {
    meta: FxCoreMeta::default(),
    name: "base".to_string(),
    types: Vec::new(),
    adt_types: Vec::new(),
    adttypes: Vec::new(),
    inputs: Vec::new(),
    morphisms: Vec::new(),
    nodes: vec![FxNode {
      name: "base".to_string(),
      uses: "nix.base".to_string(),
      meta: None,
      ..Default::default()
    }],
    edges: Vec::new(),
    scopes: Vec::new(),
  }
}

fn node_names(prefix: &'static str) -> impl Strategy<Value = Vec<String>> {
  prop::collection::btree_set("[a-z]{1,6}", 0..8).prop_map(move |names| {
    names
      .into_iter()
      .map(|name| format!("{}{}", prefix, name))
      .collect()
  })
}

fn nodes_from_names(names: Vec<String>) -> Vec<FxNode> {
  names
    .into_iter()
    .map(|name| FxNode {
      name,
      uses: "nix.noop".to_string(),
      meta: None,
      ..Default::default()
    })
    .collect()
}

fn patch_from_nodes(nodes: Vec<FxNode>) -> FxCorePatch {
  FxCorePatch {
    version: 1,
    ops: nodes
      .into_iter()
      .map(|node| PatchOp::AddNode { node })
      .collect(),
  }
}

fn merge_patches(first: FxCorePatch, second: FxCorePatch) -> FxCorePatch {
  let mut ops = first.ops;
  ops.extend(second.ops);
  FxCorePatch { version: 1, ops }
}

fn module_signature(module: &FxCoreModule) -> String {
  serde_json::to_string(module).unwrap_or_default()
}

proptest! {
  #![proptest_config(proptest_config())]

  #[test]
  fn apply_patch_compose_matches_merged_patch(
    nodes1 in node_names("a_").prop_map(nodes_from_names),
    nodes2 in node_names("b_").prop_map(nodes_from_names),
  ) {
    let base = base_module();

    let patch1 = patch_from_nodes(nodes1.clone());
    let patch2 = patch_from_nodes(nodes2.clone());
    let merged = merge_patches(patch_from_nodes(nodes1), patch_from_nodes(nodes2));

    let after_first = apply_patch(base.clone(), patch1).expect("apply patch1");
    let after_both = apply_patch(after_first, patch2).expect("apply patch2");
    let merged_result = apply_patch(base, merged).expect("apply merged patch");

    prop_assert_eq!(module_signature(&after_both), module_signature(&merged_result));
  }
}
