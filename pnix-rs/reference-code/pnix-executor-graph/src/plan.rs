//! 실행 계획 수립 - 그래프 노드의 위상 정렬
//!
//! Stage-1 그래프는 적용을 위해 DAG여야 함. 사이클은 적용 에러를 발생시킴.
//! Stage-3/3.2: 조건부 엣지(when/unless)를 위한 게이트 의존성 추가
//! Stage-4: onfail 의존성 (참조된 노드가 타겟보다 먼저 실행되어야 함)

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// CONSTITUTIONAL NOTE (D3):
// Conditional edges add scheduling dependencies only.
// This is NOT meaning interpretation.
// Meaning is fully resolved in pnix-core via EdgeCond.
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashSet};

use anyhow::Result;

use crate::model::{CostHint, EdgeCond, FxCoreModule, NodeKind};

/// 실행 계획: 위상 정렬된 노드 순서
#[derive(Debug)]
pub struct Plan {
  /// 노드 실행 순서 (이름 목록)
  pub order: Vec<String>,
}

/// FxCore 그래프로부터 실행 계획 구성
///
/// 그래프에 사이클이 있으면 에러 반환 (의미 에러가 아닌 적용 에러)
///
/// Stage-3/3.2/4: 엣지에 cond(When/Unless/OnFail)가 있으면 의존성 추가:
/// - When/Unless: gate -> edge.to (게이트가 먼저 실행되어야 함)
/// - OnFail: referenced_node -> edge.to (참조된 노드가 먼저 실행되어 실패 여부를 알아야 함)
pub fn build_plan(fx: &FxCoreModule) -> Result<Plan> {
  // Handle Stage-0 case (no nodes)
  if fx.nodes.is_empty() {
    return Ok(Plan { order: vec![] });
  }

  // 결정론 보장: BTreeMap 사용하여 반복 순서 고정
  let mut indeg: BTreeMap<&str, usize> = fx.nodes.iter().map(|n| (n.name.as_str(), 0)).collect();
  let mut adj: BTreeMap<&str, Vec<&str>> =
    fx.nodes.iter().map(|n| (n.name.as_str(), vec![])).collect();
  let node_kind: BTreeMap<&str, NodeKind> =
    fx.nodes.iter().map(|n| (n.name.as_str(), n.kind)).collect();
  let node_meta: BTreeMap<&str, (i32, CostHint)> = fx
    .nodes
    .iter()
    .map(|n| (n.name.as_str(), (n.priority, n.cost)))
    .collect();

  // Track added edges to avoid duplicates (BTreeSet for deterministic iteration)
  let mut added_edges: BTreeSet<(&str, &str)> = BTreeSet::new();

  for e in &fx.edges {
    let to = e.to.as_str();
    if !indeg.contains_key(to) {
      anyhow::bail!("edge to `{}` refers to unknown node", to);
    }

    // Stage-2 input edges: do NOT add scheduling dependencies.
    // External inputs are available before execution and are not graph nodes.
    if e.from_input.is_none() {
      let from = e.from.as_str();
      if !adj.contains_key(from) {
        anyhow::bail!("edge from `{}` refers to unknown node", from);
      }
      // LOW: 자기 루프 별도 검증 없음 수정
      // 자기 자신을 참조하는 엣지는 의미가 없으므로 에러 반환
      if from == to {
        anyhow::bail!("self-loop edge detected: node `{}` references itself", from);
      }

      // Add normal edge dependency
      // LOW: 입력 엣지 중복 silent 드랍 수정
      // 중복 엣지 발견 시 경고 출력
      if !added_edges.contains(&(from, to)) {
        if let Some(targets) = adj.get_mut(from) {
          targets.push(to);
        }
        if let Some(deg) = indeg.get_mut(to) {
          *deg += 1;
        }
        added_edges.insert((from, to));
      } else {
        // 중복 엣지 발견 시 경고 출력
        eprintln!(
          "Warning: duplicate edge detected and ignored: {} -> {} (edge will be processed only once)",
          from, to
        );
      }
      // LOW: 입력 엣지 중복 silent 드랍
      // 중복 엣지 경고 없이 제거
      // 현재는 중복 엣지를 경고 없이 제거하여 사용자 알림 없음
    }

    // Stage-3/3.2/4: Add dependency for conditional edges
    // For When/Unless/AllWhen/AllUnless: gate must run before target
    // For OnFail: referenced node must run before target (to know if it failed)
    // For WhenUnless: both gates must run before target
    if let Some(cond) = &e.cond {
      // 모든 참조된 게이트/노드에 대한 의존성 추가
      let ref_names = cond.ref_names();

      // 각 참조된 게이트/노드 검증 및 의존성 추가
      for ref_str in &ref_names {
        if !adj.contains_key(*ref_str) {
          anyhow::bail!(
            "conditional edge to `{}` has guard `{}` referencing unknown node `{}`",
            to,
            cond_label(cond),
            ref_str
          );
        }

        // 게이트 종류 검증 (When/Unless/AllWhen/AllUnless/WhenUnless의 경우)
        if matches!(
          cond,
          EdgeCond::When(_)
            | EdgeCond::Unless(_)
            | EdgeCond::AllWhen(_)
            | EdgeCond::AllUnless(_)
            | EdgeCond::WhenUnless { .. }
        ) {
          let kind = node_kind.get(*ref_str).copied().unwrap_or(NodeKind::Normal);
          if kind != NodeKind::Gate {
            anyhow::bail!(
              "conditional edge to `{}` has guard `{}` referencing non-gate node `{}`",
              to,
              cond_label(cond),
              ref_str
            );
          }
        }

        // 의존성 추가 (중복 방지)
        if !added_edges.contains(&(*ref_str, to)) {
          if let Some(targets) = adj.get_mut(*ref_str) {
            targets.push(to);
          }
          if let Some(deg) = indeg.get_mut(to) {
            *deg += 1;
          }
          added_edges.insert((*ref_str, to));
        }
      }
    }
  }

  #[derive(Debug, Copy, Clone, Eq, PartialEq)]
  struct ReadyNode<'a> {
    name: &'a str,
    priority: i32,
    cost: CostHint,
  }

  impl Ord for ReadyNode<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
      // Highest priority first; then lower cost first; then lexicographically smallest name.
      // BinaryHeap is a max heap, so larger values pop first.
      // For priority: higher is better → self.priority.cmp(&other.priority) (descending)
      // For cost: lower is better → other.cost.cmp(&self.cost) (ascending, but reversed for max heap)
      // For name: lexicographically smaller is better → other.name.cmp(&self.name) (ascending, but reversed for max heap)
      self
        .priority
        .cmp(&other.priority) // higher priority first (descending)
        .then_with(|| other.cost.cmp(&self.cost)) // lower cost first (ascending, reversed)
        .then_with(|| other.name.cmp(self.name)) // lexicographically smallest name first (ascending, reversed)
    }
  }

  impl PartialOrd for ReadyNode<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
      Some(self.cmp(other))
    }
  }

  // Kahn's algorithm with deterministic cost/priority tie-breaking (Stage-4.2)
  let mut ready = BinaryHeap::new();
  for node in &fx.nodes {
    if indeg.get(node.name.as_str()).copied().unwrap_or(0) == 0 {
      ready.push(ReadyNode {
        name: node.name.as_str(),
        priority: node.priority,
        cost: node.cost,
      });
    }
  }

  let mut order = Vec::new();
  while let Some(n) = ready.pop() {
    order.push(n.name.to_string());
    if let Some(targets) = adj.get(n.name) {
      for &m in targets {
        if let Some(d) = indeg.get_mut(m) {
          *d -= 1;
          if *d == 0 {
            let (priority, cost) = node_meta.get(m).copied().unwrap_or((0, CostHint::Medium));
            ready.push(ReadyNode {
              name: m,
              priority,
              cost,
            });
          }
        }
      }
    }
  }

  // Cycle detection: if not all nodes visited, graph has cycle
  if order.len() != fx.nodes.len() {
    // 사이클에 포함된 노드 찾기: order에 없는 노드들
    let visited: HashSet<&str> = order.iter().map(|s| s.as_str()).collect();
    let cycle_nodes: Vec<&str> = fx
      .nodes
      .iter()
      .filter_map(|n| {
        if !visited.contains(n.name.as_str()) {
          Some(n.name.as_str())
        } else {
          None
        }
      })
      .collect();

    // MEDIUM: 분리 컴포넌트 게이트 검증 누락 수정 완료
    // 도달 불가능한 게이트 노드가 있는지 확인하여 에러 메시지 개선
    let unreachable_gates: Vec<&str> = cycle_nodes
      .iter()
      .filter_map(|&name| {
        let kind = node_kind.get(name).copied().unwrap_or(NodeKind::Normal);
        if kind == NodeKind::Gate {
          Some(name)
        } else {
          None
        }
      })
      .collect();

    if !unreachable_gates.is_empty() {
      anyhow::bail!(
        "graph has unreachable gate nodes in disconnected component (visited {} of {} nodes). Unreachable gates: {:?}. All unreachable nodes: {:?}",
        order.len(),
        fx.nodes.len(),
        unreachable_gates,
        cycle_nodes
      );
    }

    anyhow::bail!(
      "graph has a cycle; cannot produce topo plan (visited {} of {} nodes). Nodes in cycle: {:?}",
      order.len(),
      fx.nodes.len(),
      cycle_nodes
    );
  }

  Ok(Plan { order })
}

fn cond_label(cond: &EdgeCond) -> &'static str {
  match cond {
    EdgeCond::When(_) => "when",
    EdgeCond::Unless(_) => "unless",
    EdgeCond::OnFail(_) => "onfail",
    EdgeCond::WhenUnless { .. } => "when_unless",
    EdgeCond::AllWhen(_) => "all_when",
    EdgeCond::AllUnless(_) => "all_unless",
    EdgeCond::Unknown => "unknown",
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::model::{CostHint, EdgeCond, FxEdge, FxNode, NodeKind};

  fn make_node(name: &str, uses: &str) -> FxNode {
    FxNode {
      name: name.into(),
      uses: uses.into(),
      kind: NodeKind::Normal,
      optional: false,
      scope: "global".into(),
      cost: CostHint::Medium,
      priority: 0,
      contract: Default::default(),
      meta: None,
    }
  }

  fn make_gate(name: &str, uses: &str) -> FxNode {
    FxNode {
      name: name.into(),
      uses: uses.into(),
      kind: NodeKind::Gate,
      optional: false,
      scope: "global".into(),
      cost: CostHint::Light,
      priority: 0,
      contract: Default::default(),
      meta: None,
    }
  }

  #[test]
  fn topo_sort_simple_chain() {
    let fx = FxCoreModule {
      meta: Default::default(),
      name: "test".into(),
      inputs: vec![],
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      morphisms: vec![],
      nodes: vec![make_node("n1", "a.f"), make_node("n2", "b.g")],
      edges: vec![FxEdge {
        from: "n1".into(),
        to: "n2".into(),
        from_port: None,
        to_port: None,
        from_input: None,
        cond: None,
      }],
      scopes: vec![],
    };

    let plan = build_plan(&fx).unwrap();
    assert_eq!(plan.order, vec!["n1", "n2"]);
  }

  #[test]
  fn topo_sort_empty() {
    let fx = FxCoreModule {
      meta: Default::default(),
      name: "test".into(),
      inputs: vec![],
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      morphisms: vec![],
      nodes: vec![],
      edges: vec![],
      scopes: vec![],
    };

    let plan = build_plan(&fx).unwrap();
    assert!(plan.order.is_empty());
  }

  #[test]
  fn gate_dependency_ordering_when() {
    // Graph: n1 -> r1 (when g1), g1 is gate
    // Expected order: n1 first, g1 before r1, r1 last
    let fx = FxCoreModule {
      meta: Default::default(),
      name: "test".into(),
      inputs: vec![],
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      morphisms: vec![],
      nodes: vec![
        make_node("n1", "py.normalize"),
        make_gate("g1", "py.is_valid"),
        make_node("r1", "deno.render"),
      ],
      edges: vec![
        FxEdge {
          from: "n1".into(),
          to: "g1".into(),
          from_port: None,
          to_port: None,
          from_input: None,
          cond: None,
        },
        FxEdge {
          from: "n1".into(),
          to: "r1".into(),
          from_port: None,
          to_port: None,
          from_input: None,
          cond: Some(EdgeCond::When("g1".into())),
        },
      ],
      scopes: vec![],
    };

    let plan = build_plan(&fx).unwrap();

    let n1_pos = plan.order.iter().position(|x| x == "n1").unwrap();
    let g1_pos = plan.order.iter().position(|x| x == "g1").unwrap();
    let r1_pos = plan.order.iter().position(|x| x == "r1").unwrap();

    assert!(n1_pos < g1_pos, "n1 should come before g1");
    assert!(n1_pos < r1_pos, "n1 should come before r1");
    assert!(
      g1_pos < r1_pos,
      "g1 should come before r1 (gate dependency)"
    );
  }

  #[test]
  fn gate_dependency_ordering_unless() {
    // Graph: n1 -> r1 (when g1), n1 -> rf (unless g1) - if/else pattern
    let fx = FxCoreModule {
      meta: Default::default(),
      name: "test".into(),
      inputs: vec![],
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      morphisms: vec![],
      nodes: vec![
        make_node("n1", "py.normalize"),
        make_gate("g1", "py.is_valid"),
        make_node("r1", "deno.render"),
        make_node("rf", "deno.render_fallback"),
      ],
      edges: vec![
        FxEdge {
          from: "n1".into(),
          to: "g1".into(),
          from_port: None,
          to_port: None,
          from_input: None,
          cond: None,
        },
        FxEdge {
          from: "n1".into(),
          to: "r1".into(),
          from_port: None,
          to_port: None,
          from_input: None,
          cond: Some(EdgeCond::When("g1".into())),
        },
        FxEdge {
          from: "n1".into(),
          to: "rf".into(),
          from_port: None,
          to_port: None,
          from_input: None,
          cond: Some(EdgeCond::Unless("g1".into())),
        },
      ],
      scopes: vec![],
    };

    let plan = build_plan(&fx).unwrap();

    let g1_pos = plan.order.iter().position(|x| x == "g1").unwrap();
    let r1_pos = plan.order.iter().position(|x| x == "r1").unwrap();
    let rf_pos = plan.order.iter().position(|x| x == "rf").unwrap();

    assert!(
      g1_pos < r1_pos,
      "g1 should come before r1 (when dependency)"
    );
    assert!(
      g1_pos < rf_pos,
      "g1 should come before rf (unless dependency)"
    );
  }

  #[test]
  fn topo_sort_respects_priority_then_cost_then_name() {
    // No edges: all nodes are ready at once.
    // Expected order: higher priority first; within priority, lower cost first; then name.
    let mut a = make_node("a", "noop");
    a.priority = 0;
    a.cost = CostHint::Medium;

    let mut b = make_node("b", "noop");
    b.priority = 10;
    b.cost = CostHint::Medium;

    let mut c = make_node("c", "noop");
    c.priority = 10;
    c.cost = CostHint::Light;

    let mut aa = make_node("aa", "noop");
    aa.priority = 10;
    aa.cost = CostHint::Light;

    let fx = FxCoreModule {
      meta: Default::default(),
      name: "test".into(),
      inputs: vec![],
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      morphisms: vec![],
      nodes: vec![a, b, c, aa],
      edges: vec![],
      scopes: vec![],
    };

    let plan = build_plan(&fx).unwrap();
    assert_eq!(plan.order, vec!["aa", "c", "b", "a"]);
  }

  #[test]
  fn topo_sort_ignores_input_edges_for_dependencies() {
    let fx = FxCoreModule {
      meta: Default::default(),
      name: "test".into(),
      inputs: vec![],
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      morphisms: vec![],
      nodes: vec![make_node("n1", "noop")],
      edges: vec![FxEdge {
        from: "input".into(),
        to: "n1".into(),
        from_port: None,
        to_port: None,
        from_input: Some("x".into()),
        cond: None,
      }],
      scopes: vec![],
    };

    let plan = build_plan(&fx).unwrap();
    assert_eq!(plan.order, vec!["n1"]);
  }

  #[test]
  fn onfail_dependency_ordering() {
    // Graph: n1 -> r1, input -> rf (onfail r1)
    // Expected: r1 must run before rf (to know if it failed)
    let fx = FxCoreModule {
      meta: Default::default(),
      name: "test".into(),
      inputs: vec![],
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      morphisms: vec![],
      nodes: vec![
        make_node("n1", "py.normalize"),
        make_node("r1", "deno.render"),
        make_node("rf", "deno.fallback"),
      ],
      edges: vec![
        FxEdge {
          from: "n1".into(),
          to: "r1".into(),
          from_port: None,
          to_port: None,
          from_input: None,
          cond: None,
        },
        FxEdge {
          from: "n1".into(),
          to: "rf".into(),
          from_port: None,
          to_port: None,
          from_input: None,
          cond: Some(EdgeCond::OnFail("r1".into())),
        },
      ],
      scopes: vec![],
    };

    let plan = build_plan(&fx).unwrap();

    let r1_pos = plan.order.iter().position(|x| x == "r1").unwrap();
    let rf_pos = plan.order.iter().position(|x| x == "rf").unwrap();

    assert!(
      r1_pos < rf_pos,
      "r1 should come before rf (onfail dependency)"
    );
  }

  #[test]
  fn cond_ref_missing_is_reported_as_cond_error() {
    let fx = FxCoreModule {
      meta: Default::default(),
      name: "test".into(),
      inputs: vec![],
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      morphisms: vec![],
      nodes: vec![make_gate("g1", "a.f"), make_node("n1", "b.g")],
      edges: vec![FxEdge {
        from: "g1".into(),
        to: "n1".into(),
        from_port: None,
        to_port: None,
        from_input: None,
        cond: Some(EdgeCond::When("missing".into())),
      }],
      scopes: vec![],
    };

    let err = build_plan(&fx).unwrap_err();
    assert!(err.to_string().contains("conditional edge"));
    assert!(err.to_string().contains("unknown node `missing`"));
  }

  #[test]
  fn cond_when_requires_gate_node() {
    let fx = FxCoreModule {
      meta: Default::default(),
      name: "test".into(),
      inputs: vec![],
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      morphisms: vec![],
      nodes: vec![make_node("n0", "a.f"), make_node("n1", "b.g")],
      edges: vec![FxEdge {
        from: "n0".into(),
        to: "n1".into(),
        from_port: None,
        to_port: None,
        from_input: None,
        cond: Some(EdgeCond::When("n0".into())),
      }],
      scopes: vec![],
    };

    let err = build_plan(&fx).unwrap_err();
    assert!(err.to_string().contains("referencing non-gate node `n0`"));
  }
}
