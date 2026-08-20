//! FxCore 패치: 그래프 수정 작업 (노드 추가/제거/변경)

use anyhow::{bail, Result};
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

use crate::model::{CostHint, FxCoreModule, FxEdge, FxNode, NodeKind};
use crate::state_mutation_contract::enforce_state_mutation_contract;

/// 엣지 인덱스를 안전하게 deserialize: 음수 값 거부 및 범위 검증
fn deserialize_edge_index<'de, D>(deserializer: D) -> Result<usize, D::Error>
where
  D: Deserializer<'de>,
{
  struct EdgeIndexVisitor;

  impl<'de> Visitor<'de> for EdgeIndexVisitor {
    type Value = usize;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
      formatter.write_str("a non-negative integer")
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
      E: de::Error,
    {
      if value < 0 {
        return Err(E::custom(format!(
          "edge index must be non-negative, got {}",
          value
        )));
      }
      value.try_into().map_err(|_| {
        E::custom(format!(
          "edge index {} exceeds usize::MAX ({})",
          value,
          usize::MAX
        ))
      })
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
      E: de::Error,
    {
      value.try_into().map_err(|_| {
        E::custom(format!(
          "edge index {} exceeds usize::MAX ({})",
          value,
          usize::MAX
        ))
      })
    }
  }

  deserializer.deserialize_any(EdgeIndexVisitor)
}

/// FxCore 패치: 그래프 수정 작업 목록
#[derive(Debug, Deserialize)]
pub struct FxCorePatch {
  /// 패치 버전
  pub version: u32,
  /// Patch identity (optional in compat mode)
  #[serde(default)]
  pub patch_id: Option<String>,
  /// Idempotency key (optional in compat mode)
  #[serde(default)]
  pub idempotency_key: Option<String>,
  /// Commit actor (must be `sequencer` when present)
  #[serde(default)]
  pub committer: Option<String>,
  /// 패치 작업 목록
  #[serde(default)]
  pub ops: Vec<PatchOp>,
}

/// 패치 작업 결과: 작업 성공/실패 및 메시지
#[derive(Debug, Serialize)]
pub struct PatchOpResult {
  /// 작업 이름
  pub op: String,
  /// 성공 여부
  pub success: bool,
  /// 에러 메시지 (실패 시)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub message: Option<String>,
}

impl PatchOpResult {
  pub fn ok(op: &str) -> Self {
    Self {
      op: op.to_string(),
      success: true,
      message: None,
    }
  }

  pub fn skip(op: &str, reason: &str) -> Self {
    Self {
      op: op.to_string(),
      success: false,
      message: Some(reason.to_string()),
    }
  }
}

/// 패치 작업: 그래프를 수정하는 작업
#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum PatchOp {
  /// 모듈 전체 교체
  ReplaceModule {
    /// 새 모듈
    module: FxCoreModule,
  },
  /// 노드 추가
  AddNode {
    /// 추가할 노드
    node: FxNode,
  },
  /// 노드 교체
  ReplaceNode {
    /// 노드 이름
    node: String,
    /// 사용할 모피즘 이름 (옵셔널)
    #[serde(default)]
    uses: Option<String>,
    /// 노드 종류 (옵셔널)
    #[serde(default)]
    kind: Option<NodeKind>,
    /// 선택적 노드 여부 (옵셔널)
    #[serde(default)]
    optional: Option<bool>,
    /// 스코프 (옵셔널)
    #[serde(default)]
    scope: Option<String>,
    /// 비용 힌트 (옵셔널)
    #[serde(default)]
    cost: Option<CostHint>,
    /// 우선순위 (옵셔널)
    #[serde(default)]
    priority: Option<i32>,
  },
  /// 노드 제거
  RemoveNode {
    /// 제거할 노드 이름
    node: String,
    /// 엣지 자동 제거 여부 (기본값: true)
    #[serde(default = "default_prune_edges")]
    prune_edges: bool,
  },
  /// 엣지 추가
  AddEdge {
    /// 추가할 엣지
    edge: FxEdge,
  },
  /// 엣지 제거
  RemoveEdge {
    /// 제거할 엣지 인덱스
    #[serde(deserialize_with = "deserialize_edge_index")]
    index: usize,
  },
  /// 엣지 교체
  ReplaceEdge {
    /// 교체할 엣지 인덱스
    #[serde(deserialize_with = "deserialize_edge_index")]
    index: usize,
    /// 새 엣지
    edge: FxEdge,
  },
  /// Unknown patch operation (for backwards compatibility)
  #[serde(other)]
  Unknown,
}

#[allow(dead_code)]
pub fn apply_patch(module: FxCoreModule, patch: FxCorePatch) -> Result<FxCoreModule> {
  apply_patch_with_results(module, patch).map(|(module, _)| module)
}

pub fn apply_patch_with_results(
  mut module: FxCoreModule,
  patch: FxCorePatch,
) -> Result<(FxCoreModule, Vec<PatchOpResult>)> {
  enforce_state_mutation_contract(
    "fxcore_patch",
    patch.patch_id.as_deref(),
    patch.idempotency_key.as_deref(),
    patch.committer.as_deref(),
  )?;

  if patch.version != 1 {
    bail!("unsupported patch version {}", patch.version);
  }

  let base_replay_hash = module.meta.replay_hash.clone();
  let mut edge_index_map: Vec<Option<usize>> = (0..module.edges.len()).map(Some).collect();

  let mut results = Vec::new();
  for op in patch.ops {
    let label = patch_op_label(&op);
    match op {
      PatchOp::ReplaceModule { module: next } => {
        module = next;
      }
      PatchOp::AddNode { node } => {
        if module.nodes.iter().any(|n| n.name == node.name) {
          bail!("patch add_node failed: node '{}' already exists", node.name);
        }
        module.nodes.push(node);
      }
      PatchOp::ReplaceNode {
        node,
        uses,
        kind,
        optional,
        scope,
        cost,
        priority,
      } => {
        let target = module
          .nodes
          .iter_mut()
          .find(|n| n.name == node)
          .ok_or_else(|| anyhow::anyhow!("patch replace_node failed: node '{}' not found", node))?;

        if let Some(uses) = uses {
          target.uses = uses;
        }
        if let Some(kind) = kind {
          target.kind = kind;
        }
        if let Some(optional) = optional {
          target.optional = optional;
        }
        if let Some(scope) = scope {
          target.scope = scope;
        }
        if let Some(cost) = cost {
          target.cost = cost;
        }
        if let Some(priority) = priority {
          target.priority = priority;
        }
      }
      PatchOp::RemoveNode { node, prune_edges } => {
        let index = module
          .nodes
          .iter()
          .position(|n| n.name == node)
          .ok_or_else(|| anyhow::anyhow!("patch remove_node failed: node '{}' not found", node))?;
        module.nodes.remove(index);
        if prune_edges {
          let old_edges = std::mem::take(&mut module.edges);
          let mut new_edges = Vec::with_capacity(old_edges.len());
          let mut old_to_new = vec![None; old_edges.len()];
          for (idx, edge) in old_edges.into_iter().enumerate() {
            if edge.from != node && edge.to != node {
              old_to_new[idx] = Some(new_edges.len());
              new_edges.push(edge);
            }
          }
          module.edges = new_edges;
          remap_edge_index_map(&mut edge_index_map, &old_to_new);
        }
      }
      PatchOp::AddEdge { edge } => {
        module.edges.push(edge);
      }
      PatchOp::RemoveEdge { index } => {
        let index = translate_edge_index(index, &edge_index_map, module.edges.len())
          .map_err(|err| anyhow::anyhow!("patch remove_edge failed: {}", err))?;
        if index >= module.edges.len() {
          bail!(
            "patch remove_edge failed: index {} out of range (len={})",
            index,
            module.edges.len()
          );
        }
        module.edges.remove(index);
        apply_edge_removal(&mut edge_index_map, index);
      }
      PatchOp::ReplaceEdge { index, edge } => {
        let index = translate_edge_index(index, &edge_index_map, module.edges.len())
          .map_err(|err| anyhow::anyhow!("patch replace_edge failed: {}", err))?;
        if index >= module.edges.len() {
          bail!(
            "patch replace_edge failed: index {} out of range (len={})",
            index,
            module.edges.len()
          );
        }
        module.edges[index] = edge;
      }
      PatchOp::Unknown => {
        // Unknown variant: skip operation (for backwards compatibility)
        // This allows deserialization of future enum variants without breaking existing code
        results.push(PatchOpResult::skip(
          label,
          "unknown patch operation variant",
        ));
        continue;
      }
    }
    results.push(PatchOpResult::ok(label));
  }

  // Preserve base replay_hash (hot-swap keeps dist/pnix.replay.json identity)
  module.meta.replay_hash = base_replay_hash;

  Ok((module, results))
}

fn default_prune_edges() -> bool {
  true
}

fn patch_op_label(op: &PatchOp) -> &'static str {
  match op {
    PatchOp::ReplaceModule { .. } => "replace_module",
    PatchOp::AddNode { .. } => "add_node",
    PatchOp::ReplaceNode { .. } => "replace_node",
    PatchOp::RemoveNode { .. } => "remove_node",
    PatchOp::AddEdge { .. } => "add_edge",
    PatchOp::RemoveEdge { .. } => "remove_edge",
    PatchOp::ReplaceEdge { .. } => "replace_edge",
    PatchOp::Unknown => "unknown",
  }
}

fn translate_edge_index(
  index: usize,
  edge_index_map: &[Option<usize>],
  edge_len: usize,
) -> Result<usize> {
  if index < edge_index_map.len() {
    edge_index_map[index].ok_or_else(|| anyhow::anyhow!("index {} refers to removed edge", index))
  } else if index < edge_len {
    Ok(index)
  } else {
    bail!("index {} out of range (len={})", index, edge_len);
  }
}

fn remap_edge_index_map(edge_index_map: &mut [Option<usize>], old_to_new: &[Option<usize>]) {
  for entry in edge_index_map.iter_mut() {
    if let Some(old_index) = *entry {
      *entry = old_to_new.get(old_index).copied().unwrap_or(None);
    }
  }
}

fn apply_edge_removal(edge_index_map: &mut [Option<usize>], removed_index: usize) {
  for entry in edge_index_map.iter_mut() {
    if let Some(current) = *entry {
      if current == removed_index {
        *entry = None;
      } else if current > removed_index {
        *entry = Some(current - 1);
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::model::{
    CostHint, Effect, ExecutionContract, FxCoreMeta, FxEdge, FxInput, FxMorphism, FxNode, NodeKind,
  };

  fn base_module() -> FxCoreModule {
    FxCoreModule {
      meta: FxCoreMeta::default(),
      name: "test".to_string(),
      inputs: vec![FxInput {
        name: "x".to_string(),
        ty: "Int".to_string(),
      }],
      types: Vec::new(),
      adt_types: Vec::new(),
      adttypes: Vec::new(),
      morphisms: vec![
        FxMorphism {
          name: "add".to_string(),
          input: "Int".to_string(),
          output: "Int".to_string(),
          inputs: Vec::new(),
          outputs: Vec::new(),
          effect: Effect::Pure,
        },
        FxMorphism {
          name: "mul".to_string(),
          input: "Int".to_string(),
          output: "Int".to_string(),
          inputs: Vec::new(),
          outputs: Vec::new(),
          effect: Effect::Pure,
        },
      ],
      nodes: vec![
        FxNode {
          name: "n1".to_string(),
          uses: "add".to_string(),
          kind: NodeKind::Normal,
          optional: false,
          scope: "global".to_string(),
          cost: CostHint::Medium,
          priority: 0,
          contract: ExecutionContract::default(),

          meta: None,
        },
        FxNode {
          name: "n2".to_string(),
          uses: "add".to_string(),
          kind: NodeKind::Normal,
          optional: false,
          scope: "global".to_string(),
          cost: CostHint::Medium,
          priority: 0,
          contract: ExecutionContract::default(),

          meta: None,
        },
      ],
      edges: vec![
        FxEdge {
          from: "input".to_string(),
          to: "n1".to_string(),
          from_port: None,
          to_port: None,
          from_input: Some("x".to_string()),
          cond: None,
        },
        FxEdge {
          from: "n1".to_string(),
          to: "n2".to_string(),
          from_port: None,
          to_port: None,
          from_input: None,
          cond: None,
        },
      ],
      scopes: Vec::new(),
    }
  }

  #[test]
  fn test_replace_node_uses() {
    let patch = FxCorePatch {
      version: 1,
      patch_id: None,
      idempotency_key: None,
      committer: None,
      ops: vec![PatchOp::ReplaceNode {
        node: "n1".to_string(),
        uses: Some("mul".to_string()),
        kind: None,
        optional: None,
        scope: None,
        cost: None,
        priority: None,
      }],
    };

    let module = apply_patch(base_module(), patch).expect("patch apply");
    let n1 = module.nodes.iter().find(|n| n.name == "n1").unwrap();
    assert_eq!(n1.uses, "mul");
  }

  #[test]
  fn test_remove_node_prunes_edges() {
    let patch = FxCorePatch {
      version: 1,
      patch_id: None,
      idempotency_key: None,
      committer: None,
      ops: vec![PatchOp::RemoveNode {
        node: "n1".to_string(),
        prune_edges: true,
      }],
    };

    let module = apply_patch(base_module(), patch).expect("patch apply");
    assert!(module.nodes.iter().all(|n| n.name != "n1"));
    assert!(module.edges.is_empty());
  }

  #[test]
  fn test_remove_node_prune_remaps_edge_indices() {
    let mut module = base_module();
    module.nodes.push(FxNode {
      name: "n3".to_string(),
      uses: "add".to_string(),
      kind: NodeKind::Normal,
      optional: false,
      scope: "global".to_string(),
      cost: CostHint::Medium,
      priority: 0,
      contract: ExecutionContract::default(),

      meta: None,
    });
    module.edges.push(FxEdge {
      from: "n2".to_string(),
      to: "n3".to_string(),
      from_port: None,
      to_port: None,
      from_input: None,
      cond: None,
    });

    let patch = FxCorePatch {
      version: 1,
      patch_id: None,
      idempotency_key: None,
      committer: None,
      ops: vec![
        PatchOp::RemoveNode {
          node: "n1".to_string(),
          prune_edges: true,
        },
        PatchOp::ReplaceEdge {
          index: 2,
          edge: FxEdge {
            from: "n2".to_string(),
            to: "n3".to_string(),
            from_port: None,
            to_port: None,
            from_input: Some("y".to_string()),
            cond: None,
          },
        },
      ],
    };

    let module = apply_patch(module, patch).expect("patch apply");
    assert_eq!(module.edges.len(), 1);
    assert_eq!(module.edges[0].from_input, Some("y".to_string()));
  }

  #[test]
  fn test_replace_edge() {
    let patch = FxCorePatch {
      version: 1,
      patch_id: None,
      idempotency_key: None,
      committer: None,
      ops: vec![PatchOp::ReplaceEdge {
        index: 0,
        edge: FxEdge {
          from: "input".to_string(),
          to: "n2".to_string(),
          from_port: None,
          to_port: None,
          from_input: Some("x".to_string()),
          cond: None,
        },
      }],
    };

    let module = apply_patch(base_module(), patch).expect("patch apply");
    assert_eq!(module.edges[0].to, "n2");
  }

  #[test]
  fn test_replace_module() {
    let mut next = base_module();
    next.name = "patched".to_string();
    next.nodes.clear();

    let patch = FxCorePatch {
      version: 1,
      patch_id: None,
      idempotency_key: None,
      committer: None,
      ops: vec![PatchOp::ReplaceModule { module: next }],
    };

    let module = apply_patch(base_module(), patch).expect("patch apply");
    assert_eq!(module.name, "patched");
    assert!(module.nodes.is_empty());
  }

  #[test]
  fn test_patch_preserves_replay_hash() {
    let mut base = base_module();
    base.meta.replay_hash = Some("base_hash".to_string());

    let mut next = base_module();
    next.meta.replay_hash = Some("new_hash".to_string());

    let patch = FxCorePatch {
      version: 1,
      patch_id: None,
      idempotency_key: None,
      committer: None,
      ops: vec![PatchOp::ReplaceModule { module: next }],
    };

    let (module, _results) = apply_patch_with_results(base, patch).expect("patch apply");
    assert_eq!(module.meta.replay_hash.as_deref(), Some("base_hash"));
  }

  #[test]
  fn test_patch_rejects_non_sequencer_committer() {
    let patch = FxCorePatch {
      version: 1,
      patch_id: None,
      idempotency_key: None,
      committer: Some("manual".to_string()),
      ops: vec![],
    };
    let err = apply_patch(base_module(), patch).expect_err("non-sequencer must fail");
    assert!(err
      .to_string()
      .contains("STATE_MUTATION_NON_SEQUENCER_COMMIT"));
  }

  #[test]
  fn test_edge_index_negative_rejected() {
    // 음수 인덱스는 deserialization 시 거부되어야 함
    let json = r#"{"op": "remove_edge", "index": -1}"#;
    let result: Result<PatchOp, _> = serde_json::from_str(json);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("non-negative"));

    let json2 = r#"{"op": "replace_edge", "index": -5, "edge": {"from": "a", "to": "b"}}"#;
    let result2: Result<PatchOp, _> = serde_json::from_str(json2);
    assert!(result2.is_err());
    assert!(result2.unwrap_err().to_string().contains("non-negative"));
  }

  #[test]
  #[cfg(target_pointer_width = "32")]
  fn test_edge_index_overflow_rejected() {
    // 32비트 시스템에서만 테스트: usize::MAX(u32::MAX)를 초과하는 u64 값 사용
    let huge_value = (u32::MAX as u64) + 1;
    let json = format!(r#"{{"op": "remove_edge", "index": {}}}"#, huge_value);
    let result: Result<PatchOp, _> = serde_json::from_str(&json);
    assert!(result.is_err());
    assert!(result
      .unwrap_err()
      .to_string()
      .contains("exceeds usize::MAX"));
  }

  #[test]
  #[cfg(target_pointer_width = "64")]
  fn test_edge_index_overflow_rejected() {
    // 64비트 시스템에서는 usize::MAX == u64::MAX이므로 JSON 파싱에서
    // u64 범위를 초과하는 값을 거부하는지 테스트
    // u64::MAX + 1 같은 값은 JSON에서 표현 불가하므로 매우 큰 문자열로 테스트
    let json = r#"{"op": "remove_edge", "index": 99999999999999999999999999999}"#;
    let result: Result<PatchOp, _> = serde_json::from_str(json);
    assert!(result.is_err()); // JSON 파싱 자체가 실패해야 함
  }

  #[test]
  fn test_edge_index_valid_accepted() {
    // 유효한 인덱스는 정상적으로 deserialize되어야 함
    let json = r#"{"op": "remove_edge", "index": 0}"#;
    let result: Result<PatchOp, _> = serde_json::from_str(json);
    assert!(result.is_ok());
    match result.unwrap() {
      PatchOp::RemoveEdge { index } => assert_eq!(index, 0),
      _ => panic!("Expected RemoveEdge"),
    }

    let json2 = r#"{"op": "replace_edge", "index": 42, "edge": {"from": "a", "to": "b"}}"#;
    let result2: Result<PatchOp, _> = serde_json::from_str(json2);
    assert!(result2.is_ok());
    match result2.unwrap() {
      PatchOp::ReplaceEdge { index, .. } => assert_eq!(index, 42),
      _ => panic!("Expected ReplaceEdge"),
    }
  }

  #[test]
  fn test_patch_json_schema_replace_edge() {
    let json = r#"{
            "version": 1,
            "ops": [
                {
                    "op": "replace_edge",
                    "index": 0,
                    "edge": {
                        "from": "a",
                        "to": "b",
                        "from_port": null,
                        "to_port": null,
                        "from_input": null,
                        "cond": null
                    }
                }
            ]
        }"#;

    let patch: FxCorePatch = serde_json::from_str(json).expect("patch json parse");
    assert_eq!(patch.ops.len(), 1);
    match &patch.ops[0] {
      PatchOp::ReplaceEdge { index, edge } => {
        assert_eq!(*index, 0);
        assert_eq!(edge.from, "a");
        assert_eq!(edge.to, "b");
      }
      _ => panic!("expected replace_edge"),
    }
  }
}
