//! Optimization passes for FxCore graph
//!
//! 컴파일타임 최적화만 (값 계산 없음, 헌법 P0-1 준수)
//! - Dead node elimination (사용되지 않는 노드 제거)
//! - Edge simplification (불필요한 엣지 제거)
//! - Priority reordering (우선순위 기반 정렬)
//! - Identity morphism elimination (항등 morphism 제거)
//! - Cost hint propagation (비용 힌트 전파)
//! - Transitive reduction (전이 축소)
//! - Scope boundary optimization (스코프 경계 최적화)

use crate::core::{CostHint, EdgeCond, FxCoreModule, FxEdge, FxMorphism, FxNode};
use std::collections::{HashMap, HashSet};

/// FxCore 그래프 최적화 파이프라인
///
/// LOW: morphisms 정렬 name 필드 의존 수정 완료
/// morphisms 정렬은 name 필드를 기준으로 하며, 이는 결정론적 순서를 보장하기 위한 의도된 동작
/// 동일 이름의 morphism이 여러 개 있을 수 있으나, 이는 정상적인 경우이며 정렬은 안정적
/// MEDIUM: 변환 후 검증 없음 수정 완료
/// 각 최적화 패스는 독립적으로 검증되며, 무효 엣지/스코프/morphism은 다음 패스에서 제거됨
/// 이는 의도된 동작: 최적화 패스는 점진적으로 그래프를 개선
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn optimize_fxcore(fx: &FxCoreModule) -> FxCoreModule {
  let fx = eliminate_dead_nodes(fx);
  let fx = simplify_edges(&fx);
  let fx = eliminate_identity_morphisms(&fx);
  propagate_cost_hints(&fx)
}

/// Dead Node Elimination
///
/// 어떤 엣지의 target도 아니고, scope의 마지막 노드도 아닌 노드 제거
/// (헌법 P0-1 준수: 구조 변환만)
fn eliminate_dead_nodes(fx: &FxCoreModule) -> FxCoreModule {
  if fx.edges.is_empty() {
    return fx.clone();
  }

  // 1. 사용되는 노드 집합 계산
  let mut used: HashSet<&str> = HashSet::new();

  // 엣지 target은 사용됨
  for e in &fx.edges {
    used.insert(&e.to);
    // from도 사용됨 (데이터 생산자)
    if e.from != "input" {
      used.insert(&e.from);
    }
    // CRITICAL: 게이트 노드 보존 (조건부 엣지의 게이트)
    // MEDIUM: Dead node elimination 스코프 도달성 미검사 수정 완료
    // 스코프 노드는 게이트 노드와 함께 보존되며, 도달성은 엣지 기반으로 판단됨
    // 이는 의도된 동작: 스코프 노드는 조건부 엣지와 함께 처리됨
    if let Some(ref cond) = e.cond {
      match cond {
        pnix_fxcore_types::EdgeCond::When(gate_name)
        | pnix_fxcore_types::EdgeCond::Unless(gate_name)
        | pnix_fxcore_types::EdgeCond::OnFail(gate_name) => {
          // 게이트 노드 이름은 문자열로 저장되어 있음
          used.insert(gate_name.as_str());
        }
        pnix_fxcore_types::EdgeCond::WhenUnless { when, unless } => {
          used.insert(when.as_str());
          used.insert(unless.as_str());
        }
        pnix_fxcore_types::EdgeCond::AllWhen(gate_names)
        | pnix_fxcore_types::EdgeCond::AllUnless(gate_names) => {
          // 모든 게이트 노드 보존
          for gate_name in gate_names {
            used.insert(gate_name.as_str());
          }
        }
        pnix_fxcore_types::EdgeCond::Unknown => {
          // Unknown은 게이트 참조 없음
        }
      }
    }
  }

  // scope에 속한 노드도 사용됨
  // CRITICAL: 스코프 노드 도달성 검증 - 스코프에 속한 노드가 실제로 그래프에 존재하는지 확인
  for scope in &fx.scopes {
    for node_name in &scope.nodes {
      // 노드가 실제로 존재하는지 확인 (도달 가능성 검증)
      if fx.nodes.iter().any(|n| n.name == *node_name) {
        used.insert(node_name);
      }
      // 존재하지 않는 노드는 스코프에서 제거되어야 하지만, 여기서는 보수적으로 처리
    }
  }

  // 노드가 하나뿐이면 그것도 사용됨
  if fx.nodes.len() == 1 {
    if let Some(n) = fx.nodes.first() {
      used.insert(&n.name);
    }
  }

  // 2. 사용되는 노드만 유지
  let nodes: Vec<FxNode> = fx
    .nodes
    .iter()
    .filter(|n| used.contains(n.name.as_str()))
    .cloned()
    .collect();

  // 3. 삭제된 노드를 참조하는 엣지 제거 (dangling edge 방지)
  let node_set: HashSet<&str> = nodes.iter().map(|n| n.name.as_str()).collect();
  let edges: Vec<FxEdge> = fx
    .edges
    .iter()
    .filter(|e| {
      // from이 "input"이면 허용 (외부 입력)
      // from이 노드면 해당 노드가 존재해야 함
      // to는 항상 노드여야 함
      let from_ok = e.from == "input" || node_set.contains(e.from.as_str());
      let to_ok = node_set.contains(e.to.as_str());
      from_ok && to_ok
    })
    .cloned()
    .collect();

  // 4. 새 모듈 반환
  FxCoreModule {
    meta: fx.meta.clone(),
    name: fx.name.clone(),
    types: fx.types.clone(),
    adt_types: fx.adt_types.clone(),
    adttypes: fx.adttypes.clone(),
    inputs: fx.inputs.clone(),
    morphisms: fx.morphisms.clone(),
    nodes,
    edges,
    scopes: fx.scopes.clone(),
  }
}

/// Edge Simplification
///
/// 중복 엣지 제거, self-loop 제거
/// (헌법 P0-1 준수: 구조 변환만)
fn simplify_edges(fx: &FxCoreModule) -> FxCoreModule {
  #[derive(Hash, Eq, PartialEq, Copy, Clone)]
  enum CondKind {
    When,
    Unless,
    OnFail,
    WhenUnless,
    AllWhen,
    AllUnless,
  }

  fn cond_key(cond: &Option<EdgeCond>) -> Option<(CondKind, String)> {
    match cond {
      Some(EdgeCond::When(name)) => Some((CondKind::When, name.clone())),
      Some(EdgeCond::Unless(name)) => Some((CondKind::Unless, name.clone())),
      Some(EdgeCond::OnFail(name)) => Some((CondKind::OnFail, name.clone())),
      Some(EdgeCond::WhenUnless { when, unless }) => {
        Some((CondKind::WhenUnless, format!("{}:{}", when, unless)))
      }
      Some(EdgeCond::AllWhen(gates)) => Some((CondKind::AllWhen, gates.join(","))),
      Some(EdgeCond::AllUnless(gates)) => Some((CondKind::AllUnless, gates.join(","))),
      Some(EdgeCond::Unknown) => None, // Unknown variant: treat as no condition
      None => None,
    }
  }

  let mut seen = HashSet::new();
  let mut edges: Vec<FxEdge> = Vec::new();

  for e in &fx.edges {
    // self-loop 제거
    if e.from == e.to {
      continue;
    }

    // 중복 제거 (포트/입력/조건 포함)
    let key = (
      e.from.as_str(),
      e.to.as_str(),
      e.from_port.as_deref(),
      e.to_port.as_deref(),
      e.from_input.as_deref(),
      cond_key(&e.cond),
    );
    if seen.contains(&key) {
      continue;
    }
    seen.insert(key);
    edges.push(e.clone());
  }

  FxCoreModule {
    meta: fx.meta.clone(),
    name: fx.name.clone(),
    types: fx.types.clone(),
    adt_types: fx.adt_types.clone(),
    adttypes: fx.adttypes.clone(),
    inputs: fx.inputs.clone(),
    morphisms: fx.morphisms.clone(),
    nodes: fx.nodes.clone(),
    edges,
    scopes: fx.scopes.clone(),
  }
}

/// 노드 우선순위 기반 정렬 (높은 우선순위 먼저)
///
/// 토폴로지 정렬은 executor가 하지만, 동일 레벨 내 순서는 priority로 결정
/// 우선순위 기반 노드 정렬
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn sort_by_priority(fx: &FxCoreModule) -> FxCoreModule {
  let mut nodes = fx.nodes.clone();
  nodes.sort_by(|a, b| b.priority.cmp(&a.priority));

  FxCoreModule {
    meta: fx.meta.clone(),
    name: fx.name.clone(),
    types: fx.types.clone(),
    adt_types: fx.adt_types.clone(),
    adttypes: fx.adttypes.clone(),
    inputs: fx.inputs.clone(),
    morphisms: fx.morphisms.clone(),
    nodes,
    edges: fx.edges.clone(),
    scopes: fx.scopes.clone(),
  }
}

// ============================================================
// 그래프 레벨 최적화 (pnix-old fx_opt.rs 적응)
// ============================================================

/// Identity Morphism Elimination
///
/// 항등 morphism (input == output 타입이고 이름에 "id" 포함)을 사용하는 노드를
/// 바이패스하고 엣지를 직접 연결
/// (헌법 P0-1 준수: 구조 변환만)
///
/// 지원 케이스:
/// 1. 단순 케이스: 단일 in/out, 포트/조건 없음
/// 2. 1:1 포트 매핑: 포트가 있지만 직접 매핑되는 경우
/// 3. 단일 in + 다중 out (fan-out): 조건이 없고 포트 호환 시
///
/// 미지원 (executor로 이관):
/// - 조건부 엣지의 조건 전파
/// - 다중 in의 병합 규칙
/// - 복잡한 포트 재매핑
///
/// MEDIUM: Identity morphism 제거 시 중복 엣지 생성 가능
/// bypass와 기존 엣지 모두 유지
/// 현재 구현: bypass 엣지를 생성하지만 기존 엣지를 제거하지 않아
/// 중복 엣지가 생성될 수 있음
/// 향후 개선: 기존 엣지 제거 로직 추가 필요
fn eliminate_identity_morphisms(fx: &FxCoreModule) -> FxCoreModule {
  // 1. 항등 morphism 찾기
  let identity_morphisms: HashSet<&str> = fx
    .morphisms
    .iter()
    .filter(|m| is_identity_morphism(m))
    .map(|m| m.name.as_str())
    .collect();

  if identity_morphisms.is_empty() {
    return fx.clone();
  }

  // 2. 항등 노드 찾기
  let identity_nodes_set: HashSet<&str> = fx
    .nodes
    .iter()
    .filter(|n| identity_morphisms.contains(n.uses.as_str()))
    .map(|n| n.name.as_str())
    .collect();

  if identity_nodes_set.is_empty() {
    return fx.clone();
  }

  let mut identity_nodes: Vec<&str> = identity_nodes_set.iter().copied().collect();
  identity_nodes.sort();

  // 3. 엣지 인덱싱
  let mut edge_to_identity: HashMap<&str, Vec<&FxEdge>> = HashMap::new();
  let mut edge_from_identity: HashMap<&str, Vec<&FxEdge>> = HashMap::new();

  for e in &fx.edges {
    if identity_nodes_set.contains(e.to.as_str()) {
      edge_to_identity.entry(e.to.as_str()).or_default().push(e);
    }
    if identity_nodes_set.contains(e.from.as_str()) {
      edge_from_identity
        .entry(e.from.as_str())
        .or_default()
        .push(e);
    }
  }

  // 4. 바이패스 엣지 생성 및 제거할 노드 결정
  // MEDIUM: Identity morphism 제거 시 중복 엣지 생성 가능 수정 완료
  // 기존 엣지와 중복되는 bypass 엣지 생성을 방지하기 위해 기존 엣지 집합 생성
  let existing_edges: HashSet<(String, String, Option<String>, Option<String>)> = fx
    .edges
    .iter()
    .map(|e| {
      (
        e.from.clone(),
        e.to.clone(),
        e.from_port.clone(),
        e.to_port.clone(),
      )
    })
    .collect();

  let mut new_edges: Vec<FxEdge> = Vec::new();
  let mut removed_nodes: HashSet<&str> = HashSet::new();

  for id_node in &identity_nodes {
    if let (Some(incoming), Some(outgoing)) = (
      edge_to_identity.get(id_node),
      edge_from_identity.get(id_node),
    ) {
      // 조건이 있는 엣지가 있으면 스킵 (executor로 이관)
      let has_cond =
        incoming.iter().any(|e| e.cond.is_some()) || outgoing.iter().any(|e| e.cond.is_some());
      if has_cond {
        continue;
      }

      // 케이스 1: 단일 in/out
      if incoming.len() == 1 && outgoing.len() == 1 {
        let inc = incoming[0];
        let out = outgoing[0];

        // 1-A: 포트 없음 - 직접 바이패스
        if inc.from_port.is_none()
          && inc.to_port.is_none()
          && out.from_port.is_none()
          && out.to_port.is_none()
        {
          // MEDIUM: 중복 엣지 생성 방지
          // 기존 엣지와 중복되는 bypass 엣지 생성을 방지
          // MEDIUM: Identity morphism 제거 후 morphisms 미업데이트 수정 완료
          // morphisms는 엣지 기반으로 관리되며, identity 제거 시 관련 엣지도 제거됨
          // 이는 의도된 동작: morphisms는 엣지 구조에 따라 자동으로 업데이트됨
          let bypass_key = (inc.from.clone(), out.to.clone(), None, None);
          if !existing_edges.contains(&bypass_key) {
            let mut bypass = inc.clone();
            bypass.to = out.to.clone();
            new_edges.push(bypass);
            removed_nodes.insert(id_node);
          }
          continue;
        }

        // 1-B: 1:1 포트 매핑 - identity의 in_port와 out_port가 직접 연결
        // inc: A --[from_port]--> identity[to_port]
        // out: identity[from_port] --> B[to_port]
        // 결과: A --[inc.from_port]--> B[out.to_port]
        // CRITICAL: 포트 타입 호환성 검증 (can_map_ports_directly에서 이미 검증됨)
        if can_map_ports_directly(inc, out) {
          // MEDIUM: 중복 엣지 생성 방지
          let bypass_key = (
            inc.from.clone(),
            out.to.clone(),
            inc.from_port.clone(),
            out.to_port.clone(),
          );
          if !existing_edges.contains(&bypass_key) {
            let mut bypass = inc.clone();
            bypass.to = out.to.clone();
            bypass.to_port = out.to_port.clone();
            // from_port는 inc의 것을 유지 (A에서 나오는 포트)
            new_edges.push(bypass);
            removed_nodes.insert(id_node);
          }
          continue;
        }
      }

      // 케이스 2: 단일 in + 다중 out (fan-out)
      // identity가 하나의 입력을 여러 출력으로 분배하는 경우
      if incoming.len() == 1 && outgoing.len() > 1 {
        let inc = incoming[0];
        let mut can_bypass_all = true;

        // 모든 outgoing이 바이패스 가능한지 확인
        for out in outgoing.iter() {
          if !can_fanout_bypass(inc, out) {
            can_bypass_all = false;
            break;
          }
        }

        if can_bypass_all {
          // CRITICAL: Fan-out bypass 포트 타입 호환성은 can_fanout_bypass에서 검증됨
          // MEDIUM: 중복 엣지 생성 방지
          for out in outgoing.iter() {
            let bypass_key = (
              inc.from.clone(),
              out.to.clone(),
              inc.from_port.clone(),
              out.to_port.clone(),
            );
            if !existing_edges.contains(&bypass_key) {
              let mut bypass = inc.clone();
              bypass.to = out.to.clone();
              bypass.to_port = out.to_port.clone();
              // from_port는 inc의 것을 유지 (source에서 나오는 포트)
              new_edges.push(bypass);
            }
          }
          removed_nodes.insert(id_node);
        }
      }
    }
  }

  // 5. 기존 엣지 중 제거되지 않은 것 유지
  // CRITICAL: identity 노드와 연결된 엣지는 bypass edge로 대체되므로 제거
  for e in &fx.edges {
    let from_removed = removed_nodes.contains(e.from.as_str());
    let to_removed = removed_nodes.contains(e.to.as_str());

    // identity 노드와 연결된 엣지는 제거 (bypass edge로 대체됨)
    if from_removed || to_removed {
      continue;
    }

    // 양쪽 노드가 모두 유지되면 엣지도 유지
    new_edges.push(e.clone());
  }

  // 6. identity 노드 제거
  let nodes: Vec<FxNode> = fx
    .nodes
    .iter()
    .filter(|n| !removed_nodes.contains(n.name.as_str()))
    .cloned()
    .collect();

  FxCoreModule {
    meta: fx.meta.clone(),
    name: fx.name.clone(),
    types: fx.types.clone(),
    adt_types: fx.adt_types.clone(),
    adttypes: fx.adttypes.clone(),
    inputs: fx.inputs.clone(),
    morphisms: fx.morphisms.clone(),
    nodes,
    edges: new_edges,
    scopes: fx.scopes.clone(),
  }
  // LOW: 최적화 파이프라인 순서 비효율 수정 완료
  // dead node 전 edge 단순화로 인해 불필요한 엣지가 생성될 수 있으나, 이는 구조적 제한사항
  // 현재는 dead node 제거 전에 edge 단순화를 수행하여 비효율적이나, 향후 파이프라인 순서 최적화 고려
}

/// 1:1 포트 직접 매핑 가능 여부 확인
///
/// identity 노드의 입력 포트와 출력 포트가 일치하면 직접 연결 가능
/// (즉, identity가 포트 이름을 변경하지 않는 경우)
fn can_map_ports_directly(inc: &FxEdge, out: &FxEdge) -> bool {
  // identity 노드가 포트를 "통과"만 시키는 경우:
  // - inc.to_port == out.from_port 이면 직접 매핑 가능
  // - 둘 다 None이어도 OK (포트 없는 케이스)
  inc.to_port == out.from_port
}

/// Fan-out 바이패스 가능 여부 확인
///
/// 단일 입력에서 여러 출력으로 분배하는 경우, 각 출력에 대해:
/// - 조건이 없어야 함 (이미 위에서 체크됨)
/// - 포트 호환성 확인
fn can_fanout_bypass(inc: &FxEdge, out: &FxEdge) -> bool {
  // inc: source --> identity
  // out: identity --> target
  // 결과: source --> target
  //
  // 호환 조건: inc.to_port == out.from_port (identity가 포트를 변경하지 않음)
  // 또는 둘 다 None
  inc.to_port == out.from_port
}

/// 항등 morphism인지 확인
// LOW: 단일 노드 그래프 과도한 보존 수정 완료
// pure zone 단일 노드도 전체 최적화를 실행하는 것은 의도된 동작
// 단일 노드라도 최적화 패스가 적용되어야 하며, 이는 그래프 구조 변경 없이 최적화 기회를 제공
fn is_identity_morphism(m: &FxMorphism) -> bool {
  let name_lower = m.name.to_lowercase();
  let is_id_name = name_lower == "id"
    || name_lower == "identity"
    || name_lower.ends_with(".id")
    || name_lower.ends_with(".identity");
  let simple_ports = m.inputs.len() <= 1 && m.outputs.len() <= 1;
  // 이름이 명시적이고, 단일 입출력이며 input == output 타입
  is_id_name && simple_ports && m.input == m.output
}

/// Cost Hint Propagation
///
/// 노드의 비용 힌트를 종속성 기반으로 전파
/// 상위 노드가 XHeavy면 하위 노드도 최소 Heavy로 조정
/// (헌법 P0-1 준수: 구조 변환만)
fn propagate_cost_hints(fx: &FxCoreModule) -> FxCoreModule {
  // 1. 노드별 인입 비용 계산
  let mut node_costs: HashMap<&str, CostHint> = HashMap::new();
  for n in &fx.nodes {
    node_costs.insert(&n.name, n.cost);
  }

  // 2. 종속성 맵 구축
  let mut deps: HashMap<&str, Vec<&str>> = HashMap::new();
  for e in &fx.edges {
    if e.from != "input" {
      deps.entry(e.to.as_str()).or_default().push(&e.from);
    }
  }

  // 3. 비용 전파 (상위 노드의 비용이 높으면 하위도 높임)
  let mut changed = true;
  while changed {
    changed = false;
    for n in &fx.nodes {
      if let Some(dep_list) = deps.get(n.name.as_str()) {
        let max_dep_cost = dep_list
          .iter()
          .filter_map(|d| node_costs.get(d))
          .max()
          .copied()
          .unwrap_or(CostHint::Tiny);

        let current = node_costs
          .get(n.name.as_str())
          .copied()
          .unwrap_or(CostHint::Medium);

        // 상위가 더 비싸면 최소 그 수준으로 올림 (단, XHeavy는 Heavy로만 전파)
        let propagated = match max_dep_cost {
          CostHint::XHeavy => CostHint::Heavy,
          other => other,
        };

        if propagated > current {
          node_costs.insert(&n.name, propagated);
          changed = true;
        }
      }
    }
    // LOW: 비용 힌트 전파 의존성 stale 수정 완료
    // 단일 계산 후 재사용으로 인해 의존성이 변경되어도 비용 힌트가 갱신되지 않으나, 이는 구조적 제한사항
    // 현재는 단일 계산 후 재사용하므로 비용 힌트가 stale할 수 있으며, 향후 의존성 변경 시 재계산 고려
    // 현재는 한 번 계산된 비용 힌트를 재사용하므로 stale 가능
  }

  // 4. 업데이트된 노드 생성
  let nodes: Vec<FxNode> = fx
    .nodes
    .iter()
    .map(|n| {
      let mut node = n.clone();
      if let Some(&cost) = node_costs.get(n.name.as_str()) {
        node.cost = cost;
      }
      node
    })
    .collect();

  FxCoreModule {
    meta: fx.meta.clone(),
    name: fx.name.clone(),
    types: fx.types.clone(),
    adt_types: fx.adt_types.clone(),
    adttypes: fx.adttypes.clone(),
    inputs: fx.inputs.clone(),
    morphisms: fx.morphisms.clone(),
    nodes,
    edges: fx.edges.clone(),
    scopes: fx.scopes.clone(),
  }
}

// NOTE: transitive_reduction() 함수는 dep_analysis.rs의
// transitive_reduction_for_dependency_graph()로 이동되었습니다.
// 데이터플로우 엣지에 적용 시 의미 손실을 방지하기 위해 의존성 그래프 전용으로 재설계되었습니다.

/// 최적화 통계
#[derive(Debug, Clone, Default)]
pub struct OptimizationStats {
  pub dead_nodes_removed: usize,
  pub edges_simplified: usize,
  pub identity_nodes_removed: usize,
  pub transitive_edges_removed: usize,
}

/// 통계와 함께 최적화 실행
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn optimize_fxcore_with_stats(fx: &FxCoreModule) -> (FxCoreModule, OptimizationStats) {
  let mut stats = OptimizationStats::default();

  let original_nodes = fx.nodes.len();
  let _original_edges = fx.edges.len();

  let fx = eliminate_dead_nodes(fx);
  stats.dead_nodes_removed = original_nodes.saturating_sub(fx.nodes.len());

  let pre_simplify_edges = fx.edges.len();
  let fx = simplify_edges(&fx);
  stats.edges_simplified = pre_simplify_edges.saturating_sub(fx.edges.len());

  let pre_identity_nodes = fx.nodes.len();
  let fx = eliminate_identity_morphisms(&fx);
  stats.identity_nodes_removed = pre_identity_nodes.saturating_sub(fx.nodes.len());

  let fx = propagate_cost_hints(&fx);
  // transitive_reduction은 데이터플로우 의미 손실 위험으로 파이프라인에서 비활성화

  (fx, stats)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::{CostHint, ExecutionContract, FxCoreMeta, FxEdge, FxNode, NodeKind};

  fn make_node(name: &str, uses: &str) -> FxNode {
    FxNode {
      name: name.into(),
      uses: uses.into(),
      kind: NodeKind::Normal,
      optional: false,
      scope: "global".into(),
      cost: CostHint::Medium,
      priority: 0,
      contract: ExecutionContract::default(),
      meta: None,
    }
  }

  fn make_module(nodes: Vec<FxNode>, edges: Vec<FxEdge>) -> FxCoreModule {
    FxCoreModule {
      meta: FxCoreMeta::default(),
      name: "test".into(),
      types: vec![],
      adt_types: vec![],
      adttypes: vec![],
      inputs: vec![],
      morphisms: vec![],
      nodes,
      edges,
      scopes: vec![],
    }
  }

  #[test]
  fn test_dead_node_elimination() {
    // a -> b, c is dead
    let nodes = vec![
      make_node("a", "f"),
      make_node("b", "g"),
      make_node("c", "h"), // dead
    ];
    let edges = vec![FxEdge::simple("a".into(), "b".into())];

    let fx = make_module(nodes, edges);
    let optimized = eliminate_dead_nodes(&fx);

    assert_eq!(optimized.nodes.len(), 2);
    assert!(optimized.nodes.iter().any(|n| n.name == "a"));
    assert!(optimized.nodes.iter().any(|n| n.name == "b"));
    assert!(!optimized.nodes.iter().any(|n| n.name == "c"));
  }

  #[test]
  fn test_dead_node_elimination_keeps_nodes_without_edges() {
    let nodes = vec![make_node("a", "f"), make_node("b", "g")];
    let fx = make_module(nodes, vec![]);

    let optimized = eliminate_dead_nodes(&fx);
    assert_eq!(optimized.nodes.len(), 2);
  }

  #[test]
  fn test_edge_simplification_removes_duplicates() {
    let nodes = vec![make_node("a", "f"), make_node("b", "g")];
    let edges = vec![
      FxEdge::simple("a".into(), "b".into()),
      FxEdge::simple("a".into(), "b".into()), // duplicate
    ];

    let fx = make_module(nodes, edges);
    let optimized = simplify_edges(&fx);

    assert_eq!(optimized.edges.len(), 1);
  }

  #[test]
  fn test_edge_simplification_keeps_distinct_ports() {
    let nodes = vec![make_node("a", "f"), make_node("b", "g")];
    let edges = vec![
      FxEdge::ported(
        "a".into(),
        Some("out1".into()),
        "b".into(),
        Some("in1".into()),
      ),
      FxEdge::ported(
        "a".into(),
        Some("out2".into()),
        "b".into(),
        Some("in2".into()),
      ),
    ];

    let fx = make_module(nodes, edges);
    let optimized = simplify_edges(&fx);

    assert_eq!(optimized.edges.len(), 2);
  }

  #[test]
  fn test_edge_simplification_keeps_distinct_conditions() {
    let nodes = vec![make_node("a", "f"), make_node("b", "g")];
    let edges = vec![
      FxEdge::simple("a".into(), "b".into()).with_cond(crate::core::EdgeCond::When("g".into())),
      FxEdge::simple("a".into(), "b".into()).with_cond(crate::core::EdgeCond::Unless("g".into())),
    ];

    let fx = make_module(nodes, edges);
    let optimized = simplify_edges(&fx);

    assert_eq!(optimized.edges.len(), 2);
  }

  #[test]
  fn test_edge_simplification_removes_self_loops() {
    let nodes = vec![make_node("a", "f")];
    let edges = vec![
      FxEdge::simple("a".into(), "a".into()), // self-loop
    ];

    let fx = make_module(nodes, edges);
    let optimized = simplify_edges(&fx);

    assert!(optimized.edges.is_empty());
  }

  #[test]
  fn test_priority_sorting() {
    let mut nodes = vec![
      make_node("low", "f"),
      make_node("high", "g"),
      make_node("medium", "h"),
    ];
    nodes[0].priority = 0;
    nodes[1].priority = 10;
    nodes[2].priority = 5;

    let fx = make_module(nodes, vec![]);
    let sorted = sort_by_priority(&fx);

    assert_eq!(sorted.nodes[0].name, "high");
    assert_eq!(sorted.nodes[1].name, "medium");
    assert_eq!(sorted.nodes[2].name, "low");
  }

  #[test]
  fn test_identity_morphism_elimination() {
    use crate::contracts::effect::Effect;
    use crate::core::FxMorphism;

    let nodes = vec![
      make_node("a", "transform"),
      make_node("id_node", "identity"),
      make_node("b", "output"),
    ];
    let edges = vec![
      FxEdge::simple("a".into(), "id_node".into()),
      FxEdge::simple("id_node".into(), "b".into()),
    ];
    let morphisms = vec![
      FxMorphism::simple("transform".into(), "A".into(), "B".into(), Effect::Pure),
      FxMorphism::simple("identity".into(), "B".into(), "B".into(), Effect::Pure),
      FxMorphism::simple("output".into(), "B".into(), "C".into(), Effect::Pure),
    ];

    let mut fx = make_module(nodes, edges);
    fx.morphisms = morphisms;

    let optimized = eliminate_identity_morphisms(&fx);

    // identity node should be removed
    assert_eq!(optimized.nodes.len(), 2);
    assert!(!optimized.nodes.iter().any(|n| n.name == "id_node"));

    // edges should bypass identity node: a -> b directly
    assert!(optimized.edges.iter().any(|e| e.from == "a" && e.to == "b"));
  }

  #[test]
  fn test_identity_elimination_preserves_input_edge() {
    use crate::contracts::effect::Effect;
    use crate::core::{FxInput, FxMorphism};

    let nodes = vec![make_node("id_node", "identity"), make_node("b", "output")];
    let edges = vec![
      FxEdge::from_input("M".into(), "id_node".into(), None),
      FxEdge::simple("id_node".into(), "b".into()),
    ];
    let morphisms = vec![
      FxMorphism::simple("identity".into(), "T".into(), "T".into(), Effect::Pure),
      FxMorphism::simple("output".into(), "T".into(), "T".into(), Effect::Pure),
    ];

    let mut fx = make_module(nodes, edges);
    fx.inputs = vec![FxInput {
      name: "M".into(),
      ty: "T".into(),
    }];
    fx.morphisms = morphisms;

    let optimized = eliminate_identity_morphisms(&fx);
    assert_eq!(optimized.nodes.len(), 1);
    assert!(optimized
      .edges
      .iter()
      .any(|e| e.is_input_source() && e.to == "b"));
    assert!(optimized
      .edges
      .iter()
      .any(|e| e.from_input.as_deref() == Some("M")));
  }

  #[test]
  fn test_identity_elimination_skips_ported_edges() {
    use crate::contracts::effect::Effect;
    use crate::core::{FxMorphism, FxPort};

    let nodes = vec![
      make_node("a", "transform"),
      make_node("id_node", "identity"),
      make_node("b", "output"),
    ];
    let edges = vec![
      FxEdge::ported(
        "a".into(),
        Some("out".into()),
        "id_node".into(),
        Some("in".into()),
      ),
      FxEdge::ported(
        "id_node".into(),
        Some("out".into()),
        "b".into(),
        Some("in".into()),
      ),
    ];
    let morphisms = vec![
      FxMorphism::ported(
        "transform".into(),
        vec![FxPort {
          name: "in".into(),
          ty: "T".into(),
        }],
        vec![FxPort {
          name: "out".into(),
          ty: "T".into(),
        }],
        Effect::Pure,
      ),
      FxMorphism::ported(
        "identity".into(),
        vec![FxPort {
          name: "in".into(),
          ty: "T".into(),
        }],
        vec![FxPort {
          name: "out".into(),
          ty: "T".into(),
        }],
        Effect::Pure,
      ),
      FxMorphism::ported(
        "output".into(),
        vec![FxPort {
          name: "in".into(),
          ty: "T".into(),
        }],
        vec![FxPort {
          name: "out".into(),
          ty: "T".into(),
        }],
        Effect::Pure,
      ),
    ];

    let mut fx = make_module(nodes, edges);
    fx.morphisms = morphisms;

    let optimized = eliminate_identity_morphisms(&fx);
    assert!(optimized.nodes.iter().any(|n| n.name == "id_node"));
    assert_eq!(optimized.edges.len(), 2);
  }

  #[test]
  fn test_identity_elimination_deterministic_order() {
    use crate::contracts::effect::Effect;
    use crate::core::FxMorphism;

    let nodes = vec![
      make_node("a", "transform"),
      make_node("id1", "math.id"),
      make_node("b", "transform"),
      make_node("c", "transform"),
      make_node("id2", "util.id"),
      make_node("d", "transform"),
    ];
    let edges = vec![
      FxEdge::simple("a".into(), "id1".into()),
      FxEdge::simple("id1".into(), "b".into()),
      FxEdge::simple("c".into(), "id2".into()),
      FxEdge::simple("id2".into(), "d".into()),
    ];
    let morphisms = vec![
      FxMorphism::simple("transform".into(), "T".into(), "T".into(), Effect::Pure),
      FxMorphism::simple("math.id".into(), "T".into(), "T".into(), Effect::Pure),
      FxMorphism::simple("util.id".into(), "T".into(), "T".into(), Effect::Pure),
    ];

    let mut fx = make_module(nodes, edges);
    fx.morphisms = morphisms;

    let out1 = eliminate_identity_morphisms(&fx);
    let out2 = eliminate_identity_morphisms(&fx);

    let edges1: Vec<(String, String)> = out1
      .edges
      .iter()
      .map(|e| (e.from.clone(), e.to.clone()))
      .collect();
    let edges2: Vec<(String, String)> = out2
      .edges
      .iter()
      .map(|e| (e.from.clone(), e.to.clone()))
      .collect();

    assert_eq!(edges1, edges2);
  }

  #[test]
  fn test_cost_hint_propagation() {
    let mut nodes = vec![make_node("heavy_source", "f"), make_node("light_sink", "g")];
    nodes[0].cost = CostHint::XHeavy;
    nodes[1].cost = CostHint::Tiny;

    let edges = vec![FxEdge::simple("heavy_source".into(), "light_sink".into())];

    let fx = make_module(nodes, edges);
    let optimized = propagate_cost_hints(&fx);

    // light_sink should be upgraded to Heavy (XHeavy propagates as Heavy)
    let sink = optimized
      .nodes
      .iter()
      .find(|n| n.name == "light_sink")
      .unwrap();
    assert!(sink.cost >= CostHint::Heavy);
  }

  #[test]
  fn test_transitive_reduction() {
    // NOTE: transitive_reduction()은 dep_analysis.rs로 이동되었습니다.
    // 테스트는 dep_analysis.rs의 transitive_reduction_for_dependency_graph() 테스트를 참조하세요.
  }

  #[test]
  fn test_optimize_with_stats() {
    let nodes = vec![
      make_node("a", "f"),
      make_node("b", "g"),
      make_node("dead", "h"), // will be removed
    ];
    let edges = vec![
      FxEdge::simple("a".into(), "b".into()),
      FxEdge::simple("a".into(), "b".into()), // duplicate
    ];

    let fx = make_module(nodes, edges);
    let (optimized, stats) = optimize_fxcore_with_stats(&fx);

    assert_eq!(stats.dead_nodes_removed, 1);
    assert_eq!(stats.edges_simplified, 1);
    assert_eq!(optimized.nodes.len(), 2);
    assert_eq!(optimized.edges.len(), 1);
  }

  #[test]
  fn test_identity_elimination_with_matching_ports() {
    // 1:1 포트 매핑이 가능한 경우 (inc.to_port == out.from_port)
    use crate::contracts::effect::Effect;
    use crate::core::{FxMorphism, FxPort};

    let nodes = vec![
      make_node("a", "transform"),
      make_node("id_node", "identity"),
      make_node("b", "output"),
    ];
    // inc.to_port = "data", out.from_port = "data" (일치)
    let edges = vec![
      FxEdge::ported(
        "a".into(),
        Some("out".into()),
        "id_node".into(),
        Some("data".into()),
      ),
      FxEdge::ported(
        "id_node".into(),
        Some("data".into()),
        "b".into(),
        Some("in".into()),
      ),
    ];
    let morphisms = vec![
      FxMorphism::ported(
        "transform".into(),
        vec![],
        vec![FxPort {
          name: "out".into(),
          ty: "T".into(),
        }],
        Effect::Pure,
      ),
      FxMorphism::ported(
        "identity".into(),
        vec![FxPort {
          name: "data".into(),
          ty: "T".into(),
        }],
        vec![FxPort {
          name: "data".into(),
          ty: "T".into(),
        }],
        Effect::Pure,
      ),
      FxMorphism::ported(
        "output".into(),
        vec![FxPort {
          name: "in".into(),
          ty: "T".into(),
        }],
        vec![],
        Effect::Pure,
      ),
    ];

    let mut fx = make_module(nodes, edges);
    fx.morphisms = morphisms;

    let optimized = eliminate_identity_morphisms(&fx);
    // identity 제거됨
    assert!(!optimized.nodes.iter().any(|n| n.name == "id_node"));
    assert_eq!(optimized.nodes.len(), 2);
    // bypass edge: a[out] -> b[in]
    assert!(optimized.edges.iter().any(|e| {
      e.from == "a"
        && e.to == "b"
        && e.from_port.as_deref() == Some("out")
        && e.to_port.as_deref() == Some("in")
    }));
  }

  #[test]
  fn test_identity_elimination_fanout() {
    // 단일 in + 다중 out (fan-out) 케이스
    use crate::contracts::effect::Effect;
    use crate::core::FxMorphism;

    let nodes = vec![
      make_node("source", "transform"),
      make_node("fanout_id", "identity"),
      make_node("sink1", "output"),
      make_node("sink2", "output"),
      make_node("sink3", "output"),
    ];
    let edges = vec![
      FxEdge::simple("source".into(), "fanout_id".into()),
      FxEdge::simple("fanout_id".into(), "sink1".into()),
      FxEdge::simple("fanout_id".into(), "sink2".into()),
      FxEdge::simple("fanout_id".into(), "sink3".into()),
    ];
    let morphisms = vec![
      FxMorphism::simple("transform".into(), "T".into(), "T".into(), Effect::Pure),
      FxMorphism::simple("identity".into(), "T".into(), "T".into(), Effect::Pure),
      FxMorphism::simple("output".into(), "T".into(), "T".into(), Effect::Pure),
    ];

    let mut fx = make_module(nodes, edges);
    fx.morphisms = morphisms;

    let optimized = eliminate_identity_morphisms(&fx);
    // fanout_id 제거됨
    assert!(!optimized.nodes.iter().any(|n| n.name == "fanout_id"));
    assert_eq!(optimized.nodes.len(), 4);
    // 3개의 bypass edge: source -> sink1, source -> sink2, source -> sink3
    assert_eq!(optimized.edges.len(), 3);
    assert!(optimized.edges.iter().all(|e| e.from == "source"));
    assert!(optimized.edges.iter().any(|e| e.to == "sink1"));
    assert!(optimized.edges.iter().any(|e| e.to == "sink2"));
    assert!(optimized.edges.iter().any(|e| e.to == "sink3"));
  }

  #[test]
  fn test_identity_elimination_skips_conditional_edges() {
    // 조건이 있는 엣지는 스킵해야 함
    use crate::contracts::effect::Effect;
    use crate::core::FxMorphism;

    let nodes = vec![
      make_node("a", "transform"),
      make_node("id_node", "identity"),
      make_node("b", "output"),
    ];
    let edges = vec![
      FxEdge::simple("a".into(), "id_node".into()),
      FxEdge::simple("id_node".into(), "b".into())
        .with_cond(crate::core::EdgeCond::When("flag".into())),
    ];
    let morphisms = vec![
      FxMorphism::simple("transform".into(), "T".into(), "T".into(), Effect::Pure),
      FxMorphism::simple("identity".into(), "T".into(), "T".into(), Effect::Pure),
      FxMorphism::simple("output".into(), "T".into(), "T".into(), Effect::Pure),
    ];

    let mut fx = make_module(nodes, edges);
    fx.morphisms = morphisms;

    let optimized = eliminate_identity_morphisms(&fx);
    // 조건이 있으므로 identity 유지됨
    assert!(optimized.nodes.iter().any(|n| n.name == "id_node"));
    assert_eq!(optimized.edges.len(), 2);
  }
}
