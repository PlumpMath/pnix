//! FRP Graph - Signal Dependency Graph Structure
//!
//! pnix-old의 meaning_core/src/frp_runtime.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! - 그래프 **구조**만 정의, 값 계산/상태 저장 없음
//! - topo_order(): 순서만 계산, 값 전파 없음 ✅ (Result 반환)
//! - has_cycle(): 구조 검증만 ✅
//! - values: HashMap 없음 (executor로 이관) ✅

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::fx::SignalId;

use super::signal::{SignalKind, SignalNode, StateId};

// ============================================================
// FRP Graph (Structure Only)
// ============================================================

/// 최대 signal 개수 제한 (DoS 방지)
const MAX_SIGNALS: usize = 10_000;

/// FRP Signal Graph (구조만, 값 없음)
///
/// ## 헌법 준수
///
/// - signals: 구조 정의 (HashMap<SignalId, SignalNode>) ✅
/// - values, current_time: **executor로 이관** ❌
/// - 크기 제한: MAX_SIGNALS (DoS 방지) ✅
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FrpGraph {
  /// Signal 노드들 (구조만)
  signals: HashMap<SignalId, SignalNode>,
  /// 다음 signal ID (빌더용)
  next_id: usize,
  /// 내보내는 signal들 (UI 등에서 사용)
  exported: Vec<SignalId>,
}

impl FrpGraph {
  /// 빈 FRP 그래프 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new() -> Self {
    Self::default()
  }

  /// Signal 개수 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn len(&self) -> usize {
    self.signals.len()
  }

  /// 그래프가 비어있는지
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn is_empty(&self) -> bool {
    self.signals.is_empty()
  }

  /// Signal 노드 조회
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn get(&self, id: SignalId) -> Option<&SignalNode> {
    self.signals.get(&id)
  }

  /// 모든 signal ID 반환
  pub fn signal_ids(&self) -> impl Iterator<Item = SignalId> + '_ {
    let mut ids: Vec<SignalId> = self.signals.keys().copied().collect();
    ids.sort_by_key(|id| id.0);
    ids.into_iter()
  }

  /// 모든 signal 노드 반환
  pub fn signals(&self) -> impl Iterator<Item = &SignalNode> {
    let mut nodes: Vec<&SignalNode> = self.signals.values().collect();
    nodes.sort_by_key(|node| node.id.0);
    nodes.into_iter()
  }

  /// Exported signal들 반환
  pub fn exported(&self) -> &[SignalId] {
    &self.exported
  }

  // ========== 빌더 메서드 (구조 추가) ==========

  /// 새 signal ID 할당
  fn alloc_signal_id(&mut self) -> SignalId {
    let id = SignalId(self.next_id);
    self.next_id += 1;
    id
  }

  /// 새 state ID 할당
  fn alloc_state_id(&mut self) -> StateId {
    let id = StateId(self.next_id);
    self.next_id += 1;
    id
  }

  /// State signal 추가 (STM Atom 참조)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// StateId만 할당, 실제 값은 executor에서 관리
  /// 크기 제한: MAX_SIGNALS (DoS 방지)
  pub fn add_state(&mut self, name: impl Into<String>) -> Result<(SignalId, StateId), String> {
    if self.signals.len() >= MAX_SIGNALS {
      return Err(format!(
        "Signal limit exceeded: cannot add more than {} signals (DoS protection)",
        MAX_SIGNALS
      ));
    }
    let signal_id = self.alloc_signal_id();
    let state_id = self.alloc_state_id();
    let node = SignalNode::new(signal_id, name, SignalKind::State { state_id });
    self.signals.insert(signal_id, node);
    Ok((signal_id, state_id))
  }

  /// 입력 signal 추가
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  /// 크기 제한: MAX_SIGNALS (DoS 방지)
  pub fn add_input(&mut self, name: impl Into<String>) -> Result<SignalId, String> {
    if self.signals.len() >= MAX_SIGNALS {
      return Err(format!(
        "Signal limit exceeded: cannot add more than {} signals (DoS protection)",
        MAX_SIGNALS
      ));
    }
    let id = self.alloc_signal_id();
    let node = SignalNode::input(id, name);
    self.signals.insert(id, node);
    Ok(id)
  }

  /// 상수 signal 추가
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  /// 크기 제한: MAX_SIGNALS (DoS 방지)
  pub fn add_constant(&mut self, name: impl Into<String>, value: f64) -> Result<SignalId, String> {
    if self.signals.len() >= MAX_SIGNALS {
      return Err(format!(
        "Signal limit exceeded: cannot add more than {} signals (DoS protection)",
        MAX_SIGNALS
      ));
    }
    let id = self.alloc_signal_id();
    let node = SignalNode::constant(id, name, value);
    self.signals.insert(id, node);
    Ok(id)
  }

  /// 시간 signal 추가
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  /// 크기 제한: MAX_SIGNALS (DoS 방지)
  pub fn add_time(&mut self) -> Result<SignalId, String> {
    if self.signals.len() >= MAX_SIGNALS {
      return Err(format!(
        "Signal limit exceeded: cannot add more than {} signals (DoS protection)",
        MAX_SIGNALS
      ));
    }
    let id = self.alloc_signal_id();
    let node = SignalNode::time(id);
    self.signals.insert(id, node);
    Ok(id)
  }

  /// 델타 시간 signal 추가
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  /// 크기 제한: MAX_SIGNALS (DoS 방지)
  pub fn add_delta_time(&mut self) -> Result<SignalId, String> {
    if self.signals.len() >= MAX_SIGNALS {
      return Err(format!(
        "Signal limit exceeded: cannot add more than {} signals (DoS protection)",
        MAX_SIGNALS
      ));
    }
    let id = self.alloc_signal_id();
    let node = SignalNode::delta_time(id);
    self.signals.insert(id, node);
    Ok(id)
  }

  /// Signal 노드 직접 추가
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  /// 크기 제한: MAX_SIGNALS (DoS 방지)
  pub fn add_node(&mut self, node: SignalNode) -> Result<SignalId, String> {
    if self.signals.len() >= MAX_SIGNALS {
      return Err(format!(
        "Signal limit exceeded: cannot add more than {} signals (DoS protection)",
        MAX_SIGNALS
      ));
    }
    let id = node.id;
    if self.signals.contains_key(&id) {
      return Err(format!("Signal ID already exists: {}", id.0));
    }
    self.signals.insert(id, node);
    // next_id 업데이트
    if id.0 >= self.next_id {
      self.next_id = id.0 + 1;
    }
    Ok(id)
  }

  /// Signal을 exported로 표시
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 설정만, 값 계산 없음
  pub fn export(&mut self, id: SignalId) {
    if !self.exported.contains(&id) {
      self.exported.push(id);
    }
  }

  // ========== 그래프 분석 (구조만, 값 없음) ==========

  /// 위상 정렬 순서 계산
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 순서만 계산, 값 전파 없음
  pub fn topo_order(&self) -> Result<Vec<SignalId>, FrpGraphError> {
    self.validate()?;
    let mut result = Vec::new();
    let mut visited = HashSet::new();
    let mut temp_visited = HashSet::new();

    // Fix: Deterministic order - sort signal IDs before iteration
    // HashMap iteration order is non-deterministic, violating determinism guarantees
    let mut signal_ids: Vec<SignalId> = self.signals.keys().copied().collect();
    signal_ids.sort_by_key(|id| id.0);

    for id in signal_ids {
      if !visited.contains(&id) {
        self.topo_visit(id, &mut visited, &mut temp_visited, &mut result)?;
      }
    }

    Ok(result)
  }

  fn topo_visit(
    &self,
    id: SignalId,
    visited: &mut HashSet<SignalId>,
    temp: &mut HashSet<SignalId>,
    result: &mut Vec<SignalId>,
  ) -> Result<(), FrpGraphError> {
    if visited.contains(&id) {
      return Ok(());
    }
    if temp.contains(&id) {
      return Err(FrpGraphError::CycleDetected);
    }

    temp.insert(id);

    if let Some(node) = self.signals.get(&id) {
      // Determinism: dependency traversal must not depend on input vector ordering.
      let mut deps = node.kind.dependencies();
      deps.sort_by_key(|dep| dep.0);
      deps.dedup_by_key(|dep| dep.0);
      for dep in deps {
        self.topo_visit(dep, visited, temp, result)?;
      }
    }

    temp.remove(&id);
    visited.insert(id);
    result.push(id);
    Ok(())
  }

  /// 사이클 존재 여부 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 검증만, 값 계산 없음
  pub fn has_cycle(&self) -> bool {
    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();

    // Fix: Deterministic order - sort signal IDs before iteration
    let mut signal_ids: Vec<SignalId> = self.signals.keys().copied().collect();
    signal_ids.sort_by_key(|id| id.0);

    for id in signal_ids {
      if self.has_cycle_util(id, &mut visited, &mut rec_stack) {
        return true;
      }
    }

    false
  }

  fn has_cycle_util(
    &self,
    id: SignalId,
    visited: &mut HashSet<SignalId>,
    rec_stack: &mut HashSet<SignalId>,
  ) -> bool {
    if rec_stack.contains(&id) {
      return true;
    }
    if visited.contains(&id) {
      return false;
    }

    visited.insert(id);
    rec_stack.insert(id);

    if let Some(node) = self.signals.get(&id) {
      // Determinism: stable recursion order for diagnostics/replay traces.
      let mut deps = node.kind.dependencies();
      deps.sort_by_key(|dep| dep.0);
      deps.dedup_by_key(|dep| dep.0);
      for dep in deps {
        if self.has_cycle_util(dep, visited, rec_stack) {
          return true;
        }
      }
    }

    rec_stack.remove(&id);
    false
  }

  /// 특정 signal에 의존하는 signal들 찾기
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn dependents(&self, id: SignalId) -> Vec<SignalId> {
    let mut dependents: Vec<SignalId> = self
      .signals
      .iter()
      .filter(|(_, node)| node.kind.dependencies().contains(&id))
      .map(|(&id, _)| id)
      .collect();
    // Determinism: HashMap iteration order must not leak into dependent traversal.
    dependents.sort_by_key(|signal_id| signal_id.0);
    dependents
  }

  /// 그래프 검증 (구조적 무결성)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 검증만, 값 계산 없음
  pub fn validate(&self) -> Result<(), FrpGraphError> {
    // 1. 사이클 검사
    if self.has_cycle() {
      return Err(FrpGraphError::CycleDetected);
    }

    // 2. 의존성 참조 검사
    let mut node_ids: Vec<SignalId> = self.signals.keys().copied().collect();
    node_ids.sort_by_key(|id| id.0);
    for node_id in node_ids {
      let Some(node) = self.signals.get(&node_id) else {
        continue;
      };
      let mut deps = node.kind.dependencies();
      deps.sort_by_key(|dep| dep.0);
      for dep in deps {
        if !self.signals.contains_key(&dep) {
          return Err(FrpGraphError::MissingDependency {
            signal: node_id,
            missing: dep,
          });
        }
      }
    }

    // 3. Fix: Exported signals 검사 - exported로 표시된 신호가 실제로 존재하는지 확인
    let mut exported_ids = self.exported.clone();
    exported_ids.sort_by_key(|id| id.0);
    exported_ids.dedup_by_key(|id| id.0);
    for exported_id in exported_ids {
      if !self.signals.contains_key(&exported_id) {
        return Err(FrpGraphError::MissingExportedSignal {
          signal: exported_id,
        });
      }
    }

    Ok(())
  }
}

// ============================================================
// Error Types
// ============================================================

/// FRP 그래프 에러
#[derive(Debug, Clone)]
pub enum FrpGraphError {
  /// 사이클 감지됨
  CycleDetected,
  /// 의존성 signal이 없음
  MissingDependency { signal: SignalId, missing: SignalId },
  /// Exported signal이 존재하지 않음
  MissingExportedSignal { signal: SignalId },
}

impl std::fmt::Display for FrpGraphError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      FrpGraphError::CycleDetected => write!(f, "Cycle detected in FRP graph"),
      FrpGraphError::MissingDependency { signal, missing } => {
        write!(
          f,
          "Signal {:?} depends on missing signal {:?}",
          signal, missing
        )
      }
      FrpGraphError::MissingExportedSignal { signal } => {
        write!(f, "Exported signal {:?} does not exist in graph", signal)
      }
    }
  }
}

impl std::error::Error for FrpGraphError {}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_empty_graph() {
    let graph = FrpGraph::new();
    assert!(graph.is_empty());
    assert_eq!(graph.len(), 0);
    assert!(!graph.has_cycle());
  }

  #[test]
  fn test_add_signals() {
    let mut graph = FrpGraph::new();

    let input = graph.add_input("mouse_x").unwrap();
    let constant = graph.add_constant("scale", 2.0).unwrap();
    let time = graph.add_time().unwrap();

    assert_eq!(graph.len(), 3);
    assert!(graph.get(input).is_some());
    assert!(graph.get(constant).is_some());
    assert!(graph.get(time).is_some());
  }

  #[test]
  fn test_add_node_rejects_duplicate_signal_id() {
    let mut graph = FrpGraph::new();
    graph
      .add_node(SignalNode::new(SignalId(7), "first", SignalKind::Input))
      .unwrap();
    let err = graph
      .add_node(SignalNode::new(
        SignalId(7),
        "second",
        SignalKind::Constant(1.0),
      ))
      .expect_err("duplicate signal id must be rejected");
    assert!(err.contains("Signal ID already exists"));
    assert_eq!(graph.len(), 1);
    assert_eq!(
      graph.get(SignalId(7)).map(|node| node.name.as_str()),
      Some("first")
    );
  }

  #[test]
  fn test_topo_order() {
    let mut graph = FrpGraph::new();

    let a = graph.add_input("a").unwrap();
    let b = graph.add_input("b").unwrap();

    // c depends on a, b
    let c_node = SignalNode::new(
      SignalId(graph.next_id),
      "c",
      SignalKind::Combine2 {
        source1: a,
        source2: b,
        ssa: crate::ssa::SsaBlock::default(),
      },
    );
    graph.next_id += 1;
    let c = graph.add_node(c_node).unwrap();

    let order = graph.topo_order().unwrap();

    // a, b should come before c
    let pos_a = order.iter().position(|&x| x == a).unwrap();
    let pos_b = order.iter().position(|&x| x == b).unwrap();
    let pos_c = order.iter().position(|&x| x == c).unwrap();

    assert!(pos_a < pos_c);
    assert!(pos_b < pos_c);
  }

  #[test]
  fn test_validate() {
    let mut graph = FrpGraph::new();
    graph.add_input("x").unwrap();
    graph.add_constant("y", 1.0).unwrap();

    assert!(graph.validate().is_ok());
  }

  #[test]
  fn test_signal_limit() {
    let mut graph = FrpGraph::new();

    // MAX_SIGNALS까지 추가 가능
    for i in 0..MAX_SIGNALS {
      let name = format!("signal_{}", i);
      assert!(graph.add_input(&name).is_ok());
    }
    assert_eq!(graph.len(), MAX_SIGNALS);

    // MAX_SIGNALS 초과 시 에러
    let result = graph.add_input("overflow");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Signal limit exceeded"));
  }

  #[test]
  fn test_dependents_are_sorted_deterministically() {
    let mut graph = FrpGraph::new();
    let root = graph.add_input("root").unwrap();

    // Intentionally insert dependents out of ID order.
    let high = SignalNode::new(
      SignalId(10),
      "high",
      SignalKind::Derived {
        deps: vec![root],
        ssa: crate::ssa::SsaBlock::default(),
      },
    );
    graph.add_node(high).unwrap();

    let low = SignalNode::new(
      SignalId(2),
      "low",
      SignalKind::Derived {
        deps: vec![root],
        ssa: crate::ssa::SsaBlock::default(),
      },
    );
    graph.add_node(low).unwrap();

    let dependents = graph.dependents(root);
    assert_eq!(dependents, vec![SignalId(2), SignalId(10)]);
  }

  #[test]
  fn test_signal_ids_are_sorted_deterministically() {
    let mut graph = FrpGraph::new();
    graph
      .add_node(SignalNode::new(SignalId(8), "n8", SignalKind::Input))
      .unwrap();
    graph
      .add_node(SignalNode::new(SignalId(3), "n3", SignalKind::Input))
      .unwrap();
    graph
      .add_node(SignalNode::new(SignalId(5), "n5", SignalKind::Input))
      .unwrap();

    let ids: Vec<SignalId> = graph.signal_ids().collect();
    assert_eq!(ids, vec![SignalId(3), SignalId(5), SignalId(8)]);
  }

  #[test]
  fn test_topo_order_is_stable_even_when_dependency_vector_order_differs() {
    let make_graph = |deps: Vec<SignalId>| {
      let mut graph = FrpGraph::new();
      graph
        .add_node(SignalNode::new(SignalId(2), "n2", SignalKind::Input))
        .unwrap();
      graph
        .add_node(SignalNode::new(SignalId(1), "n1", SignalKind::Input))
        .unwrap();
      graph
        .add_node(SignalNode::new(
          SignalId(0),
          "n0",
          SignalKind::Derived {
            deps,
            ssa: crate::ssa::SsaBlock::default(),
          },
        ))
        .unwrap();
      graph
    };

    let graph_a = make_graph(vec![SignalId(2), SignalId(1)]);
    let graph_b = make_graph(vec![SignalId(1), SignalId(2)]);

    let order_a = graph_a.topo_order().unwrap();
    let order_b = graph_b.topo_order().unwrap();
    assert_eq!(order_a, order_b);
    assert_eq!(order_a, vec![SignalId(1), SignalId(2), SignalId(0)]);
  }

  #[test]
  fn test_signals_iterator_is_sorted_by_signal_id() {
    let mut graph = FrpGraph::new();
    graph
      .add_node(SignalNode::new(SignalId(9), "n9", SignalKind::Input))
      .unwrap();
    graph
      .add_node(SignalNode::new(SignalId(1), "n1", SignalKind::Input))
      .unwrap();
    graph
      .add_node(SignalNode::new(SignalId(4), "n4", SignalKind::Input))
      .unwrap();

    let ids: Vec<SignalId> = graph.signals().map(|node| node.id).collect();
    assert_eq!(ids, vec![SignalId(1), SignalId(4), SignalId(9)]);
  }

  #[test]
  fn test_validate_missing_dependency_reports_lowest_signal_id_first() {
    let mut graph = FrpGraph::new();
    graph
      .add_node(SignalNode::new(
        SignalId(10),
        "high",
        SignalKind::Derived {
          deps: vec![SignalId(999)],
          ssa: crate::ssa::SsaBlock::default(),
        },
      ))
      .unwrap();
    graph
      .add_node(SignalNode::new(
        SignalId(2),
        "low",
        SignalKind::Derived {
          deps: vec![SignalId(123)],
          ssa: crate::ssa::SsaBlock::default(),
        },
      ))
      .unwrap();

    let err = graph.validate().expect_err("validation should fail");
    match err {
      FrpGraphError::MissingDependency { signal, missing } => {
        assert_eq!(signal, SignalId(2));
        assert_eq!(missing, SignalId(123));
      }
      other => panic!("expected MissingDependency, got {other:?}"),
    }
  }

  #[test]
  fn test_validate_missing_dependency_reports_lowest_missing_id_per_node() {
    let mut graph = FrpGraph::new();
    graph
      .add_node(SignalNode::new(
        SignalId(5),
        "n5",
        SignalKind::Derived {
          deps: vec![SignalId(50), SignalId(20)],
          ssa: crate::ssa::SsaBlock::default(),
        },
      ))
      .unwrap();

    let err = graph.validate().expect_err("validation should fail");
    match err {
      FrpGraphError::MissingDependency { signal, missing } => {
        assert_eq!(signal, SignalId(5));
        assert_eq!(missing, SignalId(20));
      }
      other => panic!("expected MissingDependency, got {other:?}"),
    }
  }
}
