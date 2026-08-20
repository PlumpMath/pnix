//! MetaFx → Mermaid: Graph visualization codegen
//!
//! pnix 그래프를 Mermaid 다이어그램으로 변환.
//! 마이그레이션 검증 및 구조 이해 도구.
//!
//! ## 설계 원칙
//!
//! 1. **결정적 순서**: 동일 입력 → 동일 출력 (diff-friendly)
//! 2. **텍스트 우선**: 조건(when/unless/onfail)은 라벨에 명시
//! 3. **Gate 구분**: `{name}:::gate` 형태로 결정 노드 표시

use super::{MetaFxEdge, MetaFxModule, MetaFxNode, MetaFxScope};

/// Mermaid 출력 상세 수준
#[derive(Debug, Clone, Copy, Default)]
pub enum MermaidDetail {
  /// 노드 이름만
  Minimal,
  /// 노드 이름 + uses (기본값)
  #[default]
  Standard,
  /// 노드 이름 + uses + cost + priority
  Full,
}

/// MetaFx → Mermaid graph 변환
///
/// 결정적 순서로 출력하여 diff 비교가 용이함.
///
/// ## 헌법 준수 (P0-1, C1)
///
/// 텍스트 생성만, 파일 I/O 없음
pub fn generate_mermaid(meta: &MetaFxModule) -> String {
  generate_mermaid_with_detail(meta, MermaidDetail::Standard)
}

/// MetaFx → Mermaid graph 변환 (상세 수준 지정)
///
/// ## 헌법 준수 (P0-1, C1)
///
/// 텍스트 생성만, 파일 I/O 없음
pub fn generate_mermaid_with_detail(meta: &MetaFxModule, detail: MermaidDetail) -> String {
  let mut out = String::new();

  // 헤더
  out.push_str("graph TD\n");

  // 노드를 scope별로 그룹화 (결정적 순서)
  let (scoped_nodes, global_nodes) = partition_nodes_by_scope(meta);

  // 1. Scope subgraphs (이름순 정렬)
  let mut sorted_scopes: Vec<&MetaFxScope> = meta.scopes.iter().collect();
  sorted_scopes.sort_by(|a, b| a.name.cmp(&b.name));

  for scope in sorted_scopes {
    if scope.name == "global" {
      continue;
    }

    let scope_nodes = scoped_nodes
      .iter()
      .filter(|n| n.scope == scope.name)
      .collect::<Vec<_>>();

    if scope_nodes.is_empty() {
      continue;
    }

    out.push_str(&format!(
      "    subgraph {}[\"Scope: {} ({})\"]\n",
      escape_mermaid_id(&scope.name),
      scope.name,
      scope.policy
    ));

    // scope 내 노드 (이름순 정렬)
    let mut sorted_scope_nodes = scope_nodes;
    sorted_scope_nodes.sort_by(|a, b| a.name.cmp(&b.name));

    for node in sorted_scope_nodes {
      out.push_str(&format!("        {}\n", format_node(node, detail)));
    }

    out.push_str("    end\n");
  }

  // 2. Global 노드 (이름순 정렬)
  let mut sorted_global: Vec<&&MetaFxNode> = global_nodes.iter().collect();
  sorted_global.sort_by(|a, b| a.name.cmp(&b.name));

  for node in sorted_global {
    out.push_str(&format!("    {}\n", format_node(node, detail)));
  }

  out.push('\n');

  // 3. 엣지 (from, to, cond 순 정렬)
  let mut sorted_edges: Vec<&MetaFxEdge> = meta.edges.iter().collect();
  sorted_edges.sort_by(|a, b| {
    a.from
      .cmp(&b.from)
      .then_with(|| a.to.cmp(&b.to))
      .then_with(|| a.cond.cmp(&b.cond))
  });

  for (idx, edge) in sorted_edges.iter().enumerate() {
    out.push_str(&format!("    {}\n", format_edge(edge, idx)));
  }

  // 4. 스타일 정의
  out.push('\n');
  out.push_str("    classDef gate fill:#f9f,stroke:#333,stroke-width:2px\n");
  out.push_str("    classDef optional fill:#ffd,stroke:#999,stroke-dasharray:5 5\n");

  // 5. onfail 엣지 스타일 (빨간색 - 보조 정보)
  let onfail_indices: Vec<usize> = sorted_edges
    .iter()
    .enumerate()
    .filter(|(_, e)| {
      e.cond
        .as_ref()
        .map(|c| c.starts_with("onfail:"))
        .unwrap_or(false)
    })
    .map(|(i, _)| i)
    .collect();

  if !onfail_indices.is_empty() {
    let indices_str = onfail_indices
      .iter()
      .map(|i| i.to_string())
      .collect::<Vec<_>>()
      .join(",");
    out.push_str(&format!(
      "    linkStyle {} stroke:#f00,stroke-width:2px\n",
      indices_str
    ));
  }

  out
}

/// 노드를 scope별로 분리
fn partition_nodes_by_scope(meta: &MetaFxModule) -> (Vec<&MetaFxNode>, Vec<&MetaFxNode>) {
  let scope_names: std::collections::HashSet<&str> =
    meta.scopes.iter().map(|s| s.name.as_str()).collect();

  let mut scoped = Vec::new();
  let mut global = Vec::new();

  for node in &meta.nodes {
    if node.scope != "global" && scope_names.contains(node.scope.as_str()) {
      scoped.push(node);
    } else {
      global.push(node);
    }
  }

  (scoped, global)
}

/// 노드 포맷팅
fn format_node(node: &MetaFxNode, detail: MermaidDetail) -> String {
  let id = escape_mermaid_id(&node.name);

  // Gate는 {} 형태 + :::gate 클래스
  if node.kind == "gate" {
    return format!("{}{{{}}}", id, node.name) + ":::gate";
  }

  // 일반 노드
  let label = match detail {
    MermaidDetail::Minimal => node.name.clone(),
    MermaidDetail::Standard => {
      format!("{}<br/>uses: {}", node.name, node.uses)
    }
    MermaidDetail::Full => {
      format!(
        "{}<br/>uses: {}<br/>cost: {} | pri: {}",
        node.name, node.uses, node.cost, node.priority
      )
    }
  };

  let class = if node.optional { ":::optional" } else { "" };

  format!("{}[{}]{}", id, label, class)
}

/// 엣지 포맷팅
fn format_edge(edge: &MetaFxEdge, _idx: usize) -> String {
  let from_id = escape_mermaid_id(&edge.from);
  let to_id = escape_mermaid_id(&edge.to);

  match &edge.cond {
    None => {
      format!("{} --> {}", from_id, to_id)
    }
    Some(cond) => {
      if let Some(gate) = cond.strip_prefix("when:") {
        // when: 실선 + 라벨
        format!("{} -->|when {}| {}", from_id, gate, to_id)
      } else if let Some(gate) = cond.strip_prefix("unless:") {
        // unless: 점선 + 라벨
        format!("{} -.->|unless {}| {}", from_id, gate, to_id)
      } else if let Some(node) = cond.strip_prefix("onfail:") {
        // onfail: 실선 + 라벨 (색상은 linkStyle로 보조)
        format!("{} -->|onfail {}| {}", from_id, node, to_id)
      } else {
        // 알 수 없는 조건
        format!("{} -->|{}| {}", from_id, cond, to_id)
      }
    }
  }
}

/// Mermaid ID 이스케이프
fn escape_mermaid_id(s: &str) -> String {
  // Mermaid ID에서 특수문자 제거/변환
  s.replace(['.', '-', ' ', ':'], "_")
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::meta::{MetaExecutionContract, MetaFxStats};

  fn make_test_meta() -> MetaFxModule {
    MetaFxModule {
      name: "test".into(),
      stage: 1,
      replay_hash: None,
      types: vec![],
      inputs: vec![],
      morphisms: vec![],
      nodes: vec![
        MetaFxNode {
          name: "solve".into(),
          uses: "physics.solve".into(),
          kind: "normal".into(),
          optional: false,
          scope: "global".into(),
          cost: "medium".into(),
          priority: 0,
          contract: MetaExecutionContract::default(),
        },
        MetaFxNode {
          name: "g1".into(),
          uses: "gate.check".into(),
          kind: "gate".into(),
          optional: false,
          scope: "global".into(),
          cost: "tiny".into(),
          priority: 10,
          contract: MetaExecutionContract::default(),
        },
        MetaFxNode {
          name: "render".into(),
          uses: "graphics.render".into(),
          kind: "normal".into(),
          optional: true,
          scope: "render_scope".into(),
          cost: "heavy".into(),
          priority: 0,
          contract: MetaExecutionContract::default(),
        },
      ],
      edges: vec![
        MetaFxEdge {
          from: "solve".into(),
          to: "g1".into(),
          cond: None,
          port_info: None,
        },
        MetaFxEdge {
          from: "solve".into(),
          to: "render".into(),
          cond: Some("when:g1".into()),
          port_info: None,
        },
      ],
      scopes: vec![MetaFxScope {
        name: "render_scope".into(),
        node_count: 1,
        policy: "isolate".into(),
      }],
      stats: MetaFxStats {
        node_count: 3,
        edge_count: 2,
        scope_count: 1,
        gate_count: 1,
        optional_count: 1,
        conditional_edge_count: 1,
      },
    }
  }

  #[test]
  fn test_generate_mermaid_basic() {
    let meta = make_test_meta();
    let mermaid = generate_mermaid(&meta);

    assert!(mermaid.starts_with("graph TD"));
    assert!(mermaid.contains("g1{g1}:::gate"));
    assert!(mermaid.contains("-->|when g1|"));
    assert!(mermaid.contains("classDef gate"));
  }

  #[test]
  fn test_mermaid_is_deterministic() {
    let meta = make_test_meta();
    let m1 = generate_mermaid(&meta);
    let m2 = generate_mermaid(&meta);

    assert_eq!(m1, m2, "Mermaid output must be deterministic");
  }

  #[test]
  fn test_mermaid_scope_subgraph() {
    let meta = make_test_meta();
    let mermaid = generate_mermaid(&meta);

    assert!(mermaid.contains("subgraph render_scope"));
    assert!(mermaid.contains("Scope: render_scope (isolate)"));
    assert!(mermaid.contains("end"));
  }

  #[test]
  fn test_mermaid_gate_shape() {
    let meta = make_test_meta();
    let mermaid = generate_mermaid(&meta);

    // Gate는 {} 형태 + :::gate 클래스
    assert!(mermaid.contains("g1{g1}:::gate"));
  }

  #[test]
  fn test_mermaid_optional_class() {
    let meta = make_test_meta();
    let mermaid = generate_mermaid(&meta);

    // optional 노드는 :::optional 클래스
    assert!(mermaid.contains(":::optional"));
  }

  #[test]
  fn test_mermaid_edge_conditions() {
    let meta = make_test_meta();
    let mermaid = generate_mermaid(&meta);

    // when 조건은 라벨로 표시
    assert!(mermaid.contains("-->|when g1|"));
  }

  #[test]
  fn test_mermaid_onfail_edge() {
    let mut meta = make_test_meta();
    meta.edges.push(MetaFxEdge {
      from: "solve".into(),
      to: "fallback".into(),
      cond: Some("onfail:render".into()),
      port_info: None,
    });
    meta.nodes.push(MetaFxNode {
      name: "fallback".into(),
      uses: "fallback.handler".into(),
      kind: "normal".into(),
      optional: false,
      scope: "global".into(),
      cost: "light".into(),
      priority: 0,
      contract: MetaExecutionContract::default(),
    });

    let mermaid = generate_mermaid(&meta);

    // onfail은 라벨 텍스트로 표시 (1차 정보)
    assert!(mermaid.contains("-->|onfail render|"));
    // 색상은 linkStyle로 보조 (2차 정보)
    assert!(mermaid.contains("linkStyle"));
    assert!(mermaid.contains("stroke:#f00"));
  }

  #[test]
  fn test_mermaid_unless_edge() {
    let mut meta = make_test_meta();
    meta.edges.push(MetaFxEdge {
      from: "solve".into(),
      to: "skip_node".into(),
      cond: Some("unless:g1".into()),
      port_info: None,
    });
    meta.nodes.push(MetaFxNode {
      name: "skip_node".into(),
      uses: "skip.handler".into(),
      kind: "normal".into(),
      optional: false,
      scope: "global".into(),
      cost: "light".into(),
      priority: 0,
      contract: MetaExecutionContract::default(),
    });

    let mermaid = generate_mermaid(&meta);

    // unless는 점선 + 라벨
    assert!(mermaid.contains("-.->|unless g1|"));
  }

  #[test]
  fn test_mermaid_sorted_output() {
    let meta = make_test_meta();
    let mermaid = generate_mermaid(&meta);

    // 노드는 이름순 정렬되어야 함
    // global 노드: g1, solve (알파벳순)
    let g1_pos = mermaid.find("g1{g1}").unwrap();
    let solve_pos = mermaid.find("solve[").unwrap();

    assert!(
      g1_pos < solve_pos,
      "Nodes should be sorted alphabetically (g1 before solve)"
    );
  }

  #[test]
  fn test_mermaid_detail_levels() {
    let meta = make_test_meta();

    let minimal = generate_mermaid_with_detail(&meta, MermaidDetail::Minimal);
    let standard = generate_mermaid_with_detail(&meta, MermaidDetail::Standard);
    let full = generate_mermaid_with_detail(&meta, MermaidDetail::Full);

    // Minimal: uses 없음
    assert!(minimal.contains("solve[solve]"));

    // Standard: uses 있음
    assert!(standard.contains("uses: physics.solve"));

    // Full: cost, priority 있음
    assert!(full.contains("cost: medium"));
    assert!(full.contains("pri: 0"));
  }
}
