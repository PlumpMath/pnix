//! 리소스 제한 테스트: FxCore 모듈의 리소스 제한 검증 테스트
//!
//! FxCore 모듈이 리소스 제한을 올바르게 준수하는지 검증합니다.

use pnix_core::contracts::{verify_resource_limits, ResourceLimits};
use pnix_core::core::{FxCoreMeta, FxCoreModule, FxEdge, FxNode};

fn make_fxcore(node_count: usize, edge_count: usize) -> FxCoreModule {
  let mut nodes = Vec::new();
  for idx in 0..node_count {
    nodes.push(FxNode {
      name: format!("n{}", idx),
      uses: "noop".to_string(),
      meta: None,
      ..Default::default()
    });
  }

  let mut edges = Vec::new();
  if node_count > 0 {
    for idx in 0..edge_count {
      let from = format!("n{}", idx % node_count);
      let to = format!("n{}", (idx + 1) % node_count);
      edges.push(FxEdge::simple(from, to));
    }
  }

  FxCoreModule {
    meta: FxCoreMeta::default(),
    name: "resource-limit-test".to_string(),
    types: Vec::new(),
    adt_types: vec![],
    adttypes: vec![],
    inputs: Vec::new(),
    morphisms: Vec::new(),
    nodes,
    edges,
    scopes: Vec::new(),
  }
}

#[test]
fn resource_limits_accept_within_bounds() {
  let fx = make_fxcore(2, 1);
  let limits = ResourceLimits {
    max_nodes: 2,
    max_edges: 2,
    max_input_bytes: 1024,
  };

  verify_resource_limits(&fx, &limits).expect("resource limits should allow small graph");
}

#[test]
fn resource_limits_reject_too_many_nodes() {
  let fx = make_fxcore(2, 1);
  let limits = ResourceLimits {
    max_nodes: 1,
    max_edges: 10,
    max_input_bytes: 1024,
  };

  let err = verify_resource_limits(&fx, &limits).expect_err("expected node limit failure");
  assert!(
    err
      .to_string()
      .contains("Graph exceeds resource limit: nodes=2 > max=1"),
    "unexpected error: {}",
    err
  );
}

#[test]
fn resource_limits_reject_too_many_edges() {
  let fx = make_fxcore(2, 3);
  let limits = ResourceLimits {
    max_nodes: 10,
    max_edges: 2,
    max_input_bytes: 1024,
  };

  let err = verify_resource_limits(&fx, &limits).expect_err("expected edge limit failure");
  assert!(
    err
      .to_string()
      .contains("Graph exceeds resource limit: edges=3 > max=2"),
    "unexpected error: {}",
    err
  );
}
