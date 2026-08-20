//! FRP Morphism - FRP/Animation 노드를 Morphism 그래프로 노출
//!
//! pnix-old의 meaning_core/unified_meaning/frp_morphism.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 그래프 구조 정의만, 실행 없음 (FrpRuntime 의존성 제외)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ═══════════════════════════════════════════════════════════════
// 1. Port Types (입출력 포트)
// ═══════════════════════════════════════════════════════════════

/// 포트 타입: Houdini/Maya 스타일의 입출력 포트 타입
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PortType {
  /// 실수 값
  Float,
  /// 정수 값
  Int,
  /// 시간 값
  Time,
  /// 델타 타임
  DeltaTime,
  /// 제네릭 값
  Any,
  /// 시그널 스트림
  Signal(
    /// 내부 타입
    Box<PortType>,
  ),
}

impl PortType {
  /// 타입 이름 문자열
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn name(&self) -> &'static str {
    match self {
      PortType::Float => "Float",
      PortType::Int => "Int",
      PortType::Time => "Time",
      PortType::DeltaTime => "DeltaTime",
      PortType::Any => "Any",
      PortType::Signal(_) => "Signal",
    }
  }

  /// 기본 포트 타입 (Float Signal)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn default_signal() -> Self {
    PortType::Signal(Box::new(PortType::Float))
  }
}

// ═══════════════════════════════════════════════════════════════
// 2. Morphism Node (CT 노드)
// ═══════════════════════════════════════════════════════════════

/// Morphism 노드 ID: Morphism 그래프의 노드 식별자
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MorphismNodeId(pub usize);

/// Morphism 노드 종류: Morphism 노드의 종류 타입
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MorphismNodeKind {
  /// 입력 노드 (외부 이벤트)
  Input,
  /// 상수 노드
  Constant(
    /// 상수 값 (문자열로 저장)
    String,
  ),
  /// 시간 노드
  Time,
  /// 델타 타임 노드
  DeltaTime,
  /// 파생 노드 (다른 노드들로부터 계산)
  Derived,
  /// State 노드 (STM)
  State,
  /// Scan 노드 (fold/reduce)
  Scan,
  /// Combine 노드 (두 입력 조합)
  Combine,
}

/// Morphism 노드: 범주론적 morphism을 나타내는 노드
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MorphismNode {
  /// 노드 ID
  pub id: MorphismNodeId,
  /// 원본 Signal ID (FRP에서 변환된 경우, 옵셔널)
  pub signal_id: Option<usize>,
  /// 노드 이름
  pub name: String,
  /// 노드 종류
  pub kind: MorphismNodeKind,
  /// 입력 포트 타입들
  pub input_ports: Vec<PortType>,
  /// 출력 포트 타입
  pub output_port: PortType,
}

impl MorphismNode {
  /// 새 노드 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(id: MorphismNodeId, name: impl Into<String>, kind: MorphismNodeKind) -> Self {
    let (input_ports, output_port) = match &kind {
      MorphismNodeKind::Input => (vec![], PortType::default_signal()),
      MorphismNodeKind::Constant(_) => (vec![], PortType::Float),
      MorphismNodeKind::Time => (vec![], PortType::Time),
      MorphismNodeKind::DeltaTime => (vec![], PortType::DeltaTime),
      MorphismNodeKind::Derived => (vec![PortType::Any], PortType::Float),
      MorphismNodeKind::State => (vec![], PortType::default_signal()),
      MorphismNodeKind::Scan => (
        vec![PortType::default_signal(), PortType::Float],
        PortType::Float,
      ),
      MorphismNodeKind::Combine => (
        vec![PortType::default_signal(), PortType::default_signal()],
        PortType::Float,
      ),
    };

    Self {
      id,
      signal_id: None,
      name: name.into(),
      kind,
      input_ports,
      output_port,
    }
  }

  /// Signal ID와 연결
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변경만, 값 계산 없음
  pub fn with_signal_id(mut self, signal_id: usize) -> Self {
    self.signal_id = Some(signal_id);
    self
  }

  /// 종류 이름
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn kind_name(&self) -> &'static str {
    match self.kind {
      MorphismNodeKind::Input => "Input",
      MorphismNodeKind::Constant(_) => "Const",
      MorphismNodeKind::Time => "Time",
      MorphismNodeKind::DeltaTime => "DeltaTime",
      MorphismNodeKind::Derived => "Derived",
      MorphismNodeKind::State => "State",
      MorphismNodeKind::Scan => "Scan",
      MorphismNodeKind::Combine => "Combine",
    }
  }
}

// ═══════════════════════════════════════════════════════════════
// 3. Morphism Edge (연결)
// ═══════════════════════════════════════════════════════════════

/// Morphism 에지: Morphism 노드 간의 연결
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MorphismEdge {
  /// 시작 노드 ID
  pub from: MorphismNodeId,
  /// 시작 노드의 출력 포트 인덱스
  pub from_port: usize,
  /// 종료 노드 ID
  pub to: MorphismNodeId,
  /// 종료 노드의 입력 포트 인덱스
  pub to_port: usize,
}

// ═══════════════════════════════════════════════════════════════
// 4. Morphism Graph
// ═══════════════════════════════════════════════════════════════

/// Morphism 그래프: 노드와 에지로 구성된 범주론적 그래프
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MorphismGraph {
  /// 그래프 이름
  pub name: String,
  /// 노드들 (노드 ID → 노드 매핑)
  pub nodes: HashMap<MorphismNodeId, MorphismNode>,
  /// 에지들
  pub edges: Vec<MorphismEdge>,
  /// 다음 노드 ID (내부 사용)
  next_id: usize,
}

impl Default for MorphismGraph {
  fn default() -> Self {
    Self::new("untitled")
  }
}

impl MorphismGraph {
  /// 새 그래프 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(name: impl Into<String>) -> Self {
    Self {
      name: name.into(),
      nodes: HashMap::new(),
      edges: Vec::new(),
      next_id: 0,
    }
  }

  /// 노드 추가
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn add_node(&mut self, name: impl Into<String>, kind: MorphismNodeKind) -> MorphismNodeId {
    let id = MorphismNodeId(self.next_id);
    self.next_id += 1;

    let node = MorphismNode::new(id, name, kind);
    self.nodes.insert(id, node);
    id
  }

  /// Signal ID와 연결된 노드 추가
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn add_node_with_signal(
    &mut self,
    name: impl Into<String>,
    kind: MorphismNodeKind,
    signal_id: usize,
  ) -> MorphismNodeId {
    let id = MorphismNodeId(self.next_id);
    self.next_id += 1;

    let node = MorphismNode::new(id, name, kind).with_signal_id(signal_id);
    self.nodes.insert(id, node);
    id
  }

  /// 에지 추가 (연결)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn add_edge(&mut self, from: MorphismNodeId, to: MorphismNodeId) {
    self.edges.push(MorphismEdge {
      from,
      from_port: 0,
      to,
      to_port: self.count_edges_to(to),
    });
  }

  /// 특정 노드로의 에지 수
  fn count_edges_to(&self, node_id: MorphismNodeId) -> usize {
    self.edges.iter().filter(|e| e.to == node_id).count()
  }

  /// 노드 조회
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn get_node(&self, id: MorphismNodeId) -> Option<&MorphismNode> {
    self.nodes.get(&id)
  }

  /// 노드 수
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn node_count(&self) -> usize {
    self.nodes.len()
  }

  /// 에지 수
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn edge_count(&self) -> usize {
    self.edges.len()
  }

  /// Signal ID로 노드 찾기
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 검색만, 값 계산 없음
  pub fn find_by_signal_id(&self, signal_id: usize) -> Option<&MorphismNode> {
    self.nodes.values().find(|n| n.signal_id == Some(signal_id))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_morphism_graph_new() {
    let graph = MorphismGraph::new("test");
    assert_eq!(graph.name, "test");
    assert_eq!(graph.node_count(), 0);
    assert_eq!(graph.edge_count(), 0);
  }

  #[test]
  fn test_add_node() {
    let mut graph = MorphismGraph::new("test");
    let node_id = graph.add_node("input1", MorphismNodeKind::Input);
    assert_eq!(graph.node_count(), 1);
    assert!(graph.get_node(node_id).is_some());
  }

  #[test]
  fn test_add_edge() {
    let mut graph = MorphismGraph::new("test");
    let node1 = graph.add_node("input1", MorphismNodeKind::Input);
    let node2 = graph.add_node("derived1", MorphismNodeKind::Derived);
    graph.add_edge(node1, node2);
    assert_eq!(graph.edge_count(), 1);
  }
}
