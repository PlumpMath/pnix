//! 리소스 제한 테스트: Executor Graph의 리소스 제한 검증 테스트
//!
//! Executor Graph가 리소스 제한을 올바르게 준수하는지 검증합니다.

use std::fs;
use std::path::{Path, PathBuf};

use pnix_fxcore_types::{FxCoreMeta, FxCoreModule, FxEdge, FxNode, FXCORE_VERSION};

fn write_text(path: &Path, contents: &str) {
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent).expect("create parent dirs");
  }
  fs::write(path, contents).expect("write file");
}

use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn make_dist(node_count: usize, edge_count: usize) -> PathBuf {
  let mut base = std::env::temp_dir();
  let nanos = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .expect("time")
    .as_nanos();
  let counter = COUNTER.fetch_add(1, Ordering::SeqCst);
  base.push(format!(
    "pnix-resource-limit-{}-{}-n{}",
    nanos, counter, node_count
  ));

  let nodes: Vec<FxNode> = (0..node_count)
    .map(|idx| FxNode {
      name: format!("n{}", idx),
      uses: "noop".to_string(),
      meta: None,
      ..Default::default()
    })
    .collect();

  let edges: Vec<FxEdge> = if node_count == 0 {
    Vec::new()
  } else {
    (0..edge_count)
      .map(|idx| {
        let from = format!("n{}", idx % node_count);
        let to = format!("n{}", (idx + 1) % node_count);
        FxEdge::simple(from, to)
      })
      .collect()
  };

  let fx = FxCoreModule {
    meta: FxCoreMeta {
      version: FXCORE_VERSION.to_string(),
      ..Default::default()
    },
    name: "resource-limit-test".to_string(),
    types: Vec::new(),
    adt_types: Vec::new(),
    adttypes: Vec::new(),
    inputs: Vec::new(),
    morphisms: Vec::new(),
    nodes,
    edges,
    scopes: Vec::new(),
  };

  let replay_path = base.join("pnix.replay.json");
  write_text(&replay_path, r#"{"replay_hash":"test"}"#);

  let fx_path = base.join("ir").join("fxcore.canon.json");
  let fx_text = serde_json::to_string_pretty(&fx).expect("fxcore json");
  write_text(&fx_path, &fx_text);

  base
}

#[tokio::test]
async fn graph_mode_rejects_node_limit() {
  let dist = make_dist(2, 0);
  let args = vec![
    "pnix".to_string(),
    "--mode".to_string(),
    "graph".to_string(),
    "--dist".to_string(),
    dist.display().to_string(),
    "--dry-run".to_string(),
    "--max-nodes".to_string(),
    "1".to_string(),
  ];

  let err = pnix_executor_graph::run_cli(args)
    .await
    .expect_err("expected node limit failure");
  assert!(
    err
      .to_string()
      .contains("Graph exceeds resource limit: nodes=2 > max=1"),
    "unexpected error: {}",
    err
  );
}

#[tokio::test]
async fn inputs_reject_max_bytes() {
  let dist = make_dist(1, 0);
  let args = vec![
    "pnix".to_string(),
    "--mode".to_string(),
    "graph".to_string(),
    "--dist".to_string(),
    dist.display().to_string(),
    "--dry-run".to_string(),
    "--max-input-bytes".to_string(),
    "5".to_string(),
    "--inputs-json".to_string(),
    "{\"x\":123}".to_string(),
  ];

  let err = pnix_executor_graph::run_cli(args)
    .await
    .expect_err("expected max-input-bytes failure");
  assert!(
    err
      .to_string()
      .contains("inputs exceed max-input-bytes limit"),
    "unexpected error: {}",
    err
  );
}
