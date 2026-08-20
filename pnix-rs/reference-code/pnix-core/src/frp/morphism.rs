//! FRP Morphism - FRP 노드를 Morphism 그래프로 노출
//!
//! pnix-old의 meaning_core/src/unified_meaning/frp_morphism.rs에서 마이그레이션.
//!
//! # 설계 원칙
//!
//! 1. **노드 = Morphism**: FRP SignalNode를 Morphism으로 래핑
//! 2. **그래프 = Composition**: Signal 의존성을 morphism 합성으로 표현
//! 3. **Port = Domain/Codomain**: 입출력 타입을 명시적으로 표현
//!
//! # 헌법 준수 (P0-1)
//!
//! - 구조 정의만, 값 계산/상태 변경 없음
//! - from_frp_runtime() 제외 (executor 영역 - FrpRuntime 사용)
//! - from_frp_graph() 사용 (pnix-core의 FrpGraph 구조)
//!
//! # 아키텍처
//!
//! ```text
//! SignalNode (FRP)     →  MorphismNode (CT)
//! SignalKind::Derived  →  MorphismNodeKind::Derived
//! FrpGraph             →  MorphismGraph
//! ```

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use super::graph::FrpGraph;
use super::signal::SignalKind;
use crate::fx::SignalId;

// ═══════════════════════════════════════════════════════════════
// 1. Port Types (입출력 포트)
// ═══════════════════════════════════════════════════════════════

/// 포트 타입 (Houdini/Maya 스타일)
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
  Signal(Box<PortType>),
}

impl PortType {
  /// 타입 이름 문자열
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
  pub fn default_signal() -> Self {
    PortType::Signal(Box::new(PortType::Float))
  }
}

// ═══════════════════════════════════════════════════════════════
// 2. Morphism Node (CT 노드)
// ═══════════════════════════════════════════════════════════════

/// Morphism 노드 ID
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MorphismNodeId(pub usize);

/// Morphism 노드 종류
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MorphismNodeKind {
  /// 입력 노드 (외부 이벤트)
  Input,
  /// 상수 노드
  Constant(String), // 값을 문자열로 저장
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

/// Morphism 노드
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MorphismNode {
  /// 노드 ID
  pub id: MorphismNodeId,
  /// 원본 Signal ID (FRP에서 변환된 경우)
  pub signal_id: Option<SignalId>,
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
  /// 구조 설정만, 값 계산 없음
  pub fn with_signal_id(mut self, signal_id: SignalId) -> Self {
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

/// Morphism 에지 (노드 간 연결)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MorphismEdge {
  /// 시작 노드
  pub from: MorphismNodeId,
  /// 시작 노드의 출력 포트 인덱스
  pub from_port: usize,
  /// 종료 노드
  pub to: MorphismNodeId,
  /// 종료 노드의 입력 포트 인덱스
  pub to_port: usize,
}

// ═══════════════════════════════════════════════════════════════
// 4. Morphism Graph
// ═══════════════════════════════════════════════════════════════

/// Morphism 그래프 (노드 + 에지)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MorphismGraph {
  /// 그래프 이름
  pub name: String,
  /// 노드들
  pub nodes: HashMap<MorphismNodeId, MorphismNode>,
  /// 에지들
  pub edges: Vec<MorphismEdge>,
  /// 다음 노드 ID
  next_id: usize,
}

pub const MORPHISM_REPLAY_ARTIFACT_SCHEMA: &str = "pnix-core-morphism-replay-artifact@v0.1";
pub const MORPHISM_REPLAY_REASON_SCHEMA_MISMATCH: &str = "MORPHISM_REPLAY_SCHEMA_MISMATCH";
pub const MORPHISM_REPLAY_REASON_GRAPH_NAME_MISMATCH: &str = "MORPHISM_REPLAY_GRAPH_NAME_MISMATCH";
pub const MORPHISM_REPLAY_REASON_SEED_MISMATCH: &str = "MORPHISM_REPLAY_SEED_MISMATCH";
pub const MORPHISM_REPLAY_REASON_NODE_SET_MISMATCH: &str = "MORPHISM_REPLAY_NODE_SET_MISMATCH";
pub const MORPHISM_REPLAY_REASON_EDGE_SET_MISMATCH: &str = "MORPHISM_REPLAY_EDGE_SET_MISMATCH";
pub const MORPHISM_REPLAY_REASON_HASH_MISMATCH: &str = "MORPHISM_REPLAY_HASH_MISMATCH";

/// Deterministic replay artifact for MorphismGraph comparisons.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MorphismReplayArtifact {
  pub schema: String,
  pub graph_name: String,
  pub seed: u64,
  pub nodes: Vec<String>,
  pub edges: Vec<String>,
}

impl MorphismReplayArtifact {
  /// Canonical replay hash (FNV-1a 64-bit, hex).
  pub fn hash_hex(&self) -> String {
    let mut payload = String::new();
    append_len_prefixed(&mut payload, &self.schema);
    append_len_prefixed(&mut payload, &self.graph_name);
    append_len_prefixed(&mut payload, &self.seed.to_string());
    append_len_prefixed(&mut payload, &self.nodes.len().to_string());
    for node in &self.nodes {
      append_len_prefixed(&mut payload, node);
    }
    append_len_prefixed(&mut payload, &self.edges.len().to_string());
    for edge in &self.edges {
      append_len_prefixed(&mut payload, edge);
    }
    format!("{:016x}", fnv1a64(payload.as_bytes()))
  }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MorphismReplayMismatch {
  pub reason_code: String,
  pub detail: String,
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
    signal_id: SignalId,
  ) -> MorphismNodeId {
    let id = MorphismNodeId(self.next_id);
    self.next_id += 1;

    let node = MorphismNode::new(id, name, kind).with_signal_id(signal_id);
    self.nodes.insert(id, node);
    id
  }

  /// 미리 구성된 노드 직접 추가 (ID는 자동 재할당)
  #[cfg(test)]
  pub fn add_node_raw(&mut self, mut node: MorphismNode) -> MorphismNodeId {
    let id = MorphismNodeId(self.next_id);
    self.next_id += 1;
    node.id = id;
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
  /// 구조 접근만, 값 계산 없음
  pub fn find_by_signal_id(&self, signal_id: SignalId) -> Option<&MorphismNode> {
    self.nodes.values().find(|n| n.signal_id == Some(signal_id))
  }

  fn sorted_node_ids(&self) -> Vec<MorphismNodeId> {
    let mut ids: Vec<MorphismNodeId> = self.nodes.keys().copied().collect();
    ids.sort_by_key(|id| id.0);
    ids
  }

  /// Deterministic replay artifact snapshot.
  ///
  /// - `seed`는 실행 시나리오 식별값으로 artifact에 고정 저장된다.
  /// - 노드/에지는 canonical 문자열로 정렬되어 동일 구조에서 동일 결과를 보장한다.
  pub fn replay_artifact(&self, seed: u64) -> MorphismReplayArtifact {
    let mut nodes = Vec::new();
    for node_id in self.sorted_node_ids() {
      let Some(node) = self.nodes.get(&node_id) else {
        continue;
      };
      let signal = node
        .signal_id
        .map(|id| id.0.to_string())
        .unwrap_or_else(|| "-".to_string());
      let input_ports = node
        .input_ports
        .iter()
        .map(canonical_port_type)
        .collect::<Vec<_>>()
        .join(",");
      nodes.push(format!(
        "id={};signal={};name={};kind={};in=[{}];out={}",
        node.id.0,
        signal,
        node.name,
        canonical_node_kind(&node.kind),
        input_ports,
        canonical_port_type(&node.output_port)
      ));
    }

    let edges = canonical_replay_edges(&self.edges);

    MorphismReplayArtifact {
      schema: MORPHISM_REPLAY_ARTIFACT_SCHEMA.to_string(),
      graph_name: self.name.clone(),
      seed,
      nodes,
      edges,
    }
  }

  // ─────────────────────────────────────────────────────────
  // FRP Graph 변환 (헌법 준수: FrpGraph 사용)
  // ─────────────────────────────────────────────────────────

  /// FrpGraph에서 MorphismGraph 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// FrpRuntime 대신 FrpGraph (구조만) 사용 ✅
  pub fn from_frp_graph(frp_graph: &FrpGraph) -> Self {
    let mut graph = Self::new("frp_graph");
    let mut signal_to_node: HashMap<SignalId, MorphismNodeId> = HashMap::new();

    // 1. 모든 signal을 노드로 변환
    for signal_node in frp_graph.signals() {
      let kind = signal_kind_to_morphism_kind(&signal_node.kind);
      let node_id = graph.add_node_with_signal(&signal_node.name, kind, signal_node.id);
      signal_to_node.insert(signal_node.id, node_id);
    }

    // 2. 의존성을 에지로 변환
    for signal_node in frp_graph.signals() {
      let to_node_id = signal_to_node[&signal_node.id];

      for dep_id in signal_node.kind.dependencies() {
        if let Some(&from_node_id) = signal_to_node.get(&dep_id) {
          graph.add_edge(from_node_id, to_node_id);
        }
      }
    }

    graph
  }

  // ─────────────────────────────────────────────────────────
  // DOT 출력 (Graphviz)
  // ─────────────────────────────────────────────────────────

  /// DOT 형식 문자열 생성
  ///
  /// ## 헌법 준수 (P0-1, C1)
  ///
  /// 텍스트 생성만, 파일 I/O 없음
  pub fn to_dot(&self) -> String {
    let mut dot = String::new();
    dot.push_str(&format!("digraph {} {{\n", sanitize_name(&self.name)));
    dot.push_str("  rankdir=LR;\n");
    dot.push_str("  node [shape=box];\n\n");

    // 노드들
    let mut sorted_nodes: Vec<_> = self.nodes.iter().collect();
    sorted_nodes.sort_by_key(|(id, _)| id.0);
    for (id, node) in sorted_nodes {
      let color = kind_to_color(&node.kind);
      let label = format!("{}\\n[{}]", node.name, node.kind_name());
      dot.push_str(&format!(
        "  n{} [label=\"{}\", style=filled, fillcolor=\"{}\"];\n",
        id.0, label, color
      ));
    }

    dot.push('\n');

    // 에지들
    let mut sorted_edges: Vec<_> = self.edges.iter().collect();
    sorted_edges.sort_by_key(|edge| (edge.from.0, edge.to.0, edge.from_port, edge.to_port));
    for edge in sorted_edges {
      dot.push_str(&format!(
        "  n{} -> n{} [label=\"{}:{}\"];\n",
        edge.from.0, edge.to.0, edge.from_port, edge.to_port
      ));
    }

    dot.push_str("}\n");
    dot
  }
}

// ═══════════════════════════════════════════════════════════════
// 5. Node Replacement
// ═══════════════════════════════════════════════════════════════

/// 노드 대체 오류
#[derive(Debug, Clone)]
pub enum NodeReplacementError {
  NodeNotFound(MorphismNodeId),
  TypeMismatch { expected: PortType, got: PortType },
  CycleDetected,
}

impl std::fmt::Display for NodeReplacementError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      NodeReplacementError::NodeNotFound(id) => {
        write!(f, "Node not found: {:?}", id)
      }
      NodeReplacementError::TypeMismatch { expected, got } => {
        write!(f, "Type mismatch: expected {:?}, got {:?}", expected, got)
      }
      NodeReplacementError::CycleDetected => {
        write!(f, "Cycle detected in graph")
      }
    }
  }
}

impl std::error::Error for NodeReplacementError {}

impl MorphismGraph {
  /// 노드 대체 (Dynamic IR)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변환만, 값 계산 없음
  pub fn replace_node(
    &mut self,
    old_id: MorphismNodeId,
    new_node: MorphismNode,
  ) -> Result<MorphismNodeId, NodeReplacementError> {
    let old_node = self
      .nodes
      .get(&old_id)
      .ok_or(NodeReplacementError::NodeNotFound(old_id))?;

    if old_node.output_port != new_node.output_port {
      return Err(NodeReplacementError::TypeMismatch {
        expected: old_node.output_port.clone(),
        got: new_node.output_port.clone(),
      });
    }

    let new_id = MorphismNodeId(self.next_id);
    self.next_id += 1;

    let mut node = new_node;
    node.id = new_id;
    self.nodes.insert(new_id, node);

    for edge in &mut self.edges {
      if edge.from == old_id {
        edge.from = new_id;
      }
      if edge.to == old_id {
        edge.to = new_id;
      }
    }

    self.nodes.remove(&old_id);

    Ok(new_id)
  }

  /// 노드 삭제 (연결된 에지도 함께 삭제)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 제거만, 값 계산 없음
  pub fn remove_node(&mut self, id: MorphismNodeId) -> Option<MorphismNode> {
    self.edges.retain(|e| e.from != id && e.to != id);
    self.nodes.remove(&id)
  }

  /// 사이클 감지
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 검증만, 값 계산 없음
  pub fn has_cycle(&self) -> bool {
    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();

    for node_id in self.sorted_node_ids() {
      if self.has_cycle_dfs(node_id, &mut visited, &mut rec_stack) {
        return true;
      }
    }
    false
  }

  fn has_cycle_dfs(
    &self,
    node_id: MorphismNodeId,
    visited: &mut HashSet<MorphismNodeId>,
    rec_stack: &mut HashSet<MorphismNodeId>,
  ) -> bool {
    if rec_stack.contains(&node_id) {
      return true;
    }
    if visited.contains(&node_id) {
      return false;
    }

    visited.insert(node_id);
    rec_stack.insert(node_id);

    for edge in &self.edges {
      if edge.from == node_id && self.has_cycle_dfs(edge.to, visited, rec_stack) {
        return true;
      }
    }

    rec_stack.remove(&node_id);
    false
  }
}

// ═══════════════════════════════════════════════════════════════
// 6. CT Law Validation
// ═══════════════════════════════════════════════════════════════

/// CT 법칙 위반 종류
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CtLawViolation {
  PortTypeMismatch {
    edge_from: MorphismNodeId,
    edge_to: MorphismNodeId,
    output_type: PortType,
    input_type: PortType,
  },
  MissingInputPortType {
    node_id: MorphismNodeId,
    port_index: usize,
  },
  CycleDetected,
  DisconnectedNode(MorphismNodeId),
  UnconnectedInputPort {
    node_id: MorphismNodeId,
    port_index: usize,
  },
  FunctorIdentityViolation {
    node_id: MorphismNodeId,
    input_type: PortType,
    output_type: PortType,
  },
  FunctorCompositionViolation {
    path_a: Vec<MorphismNodeId>,
    path_b: Vec<MorphismNodeId>,
    result_type_a: PortType,
    result_type_b: PortType,
  },
  MultiOutputTypeMismatch {
    node_id: MorphismNodeId,
    output_types: Vec<PortType>,
  },
}

impl std::fmt::Display for CtLawViolation {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      CtLawViolation::PortTypeMismatch {
        edge_from,
        edge_to,
        output_type,
        input_type,
      } => {
        write!(
          f,
          "Port type mismatch: {:?} ({:?}) -> {:?} ({:?})",
          edge_from, output_type, edge_to, input_type
        )
      }
      CtLawViolation::CycleDetected => {
        write!(
          f,
          "Cycle detected in morphism graph (violates DAG requirement)"
        )
      }
      CtLawViolation::MissingInputPortType {
        node_id,
        port_index,
      } => {
        write!(
          f,
          "Missing input port type: node {:?} port {}",
          node_id, port_index
        )
      }
      CtLawViolation::DisconnectedNode(id) => {
        write!(f, "Disconnected node: {:?}", id)
      }
      CtLawViolation::UnconnectedInputPort {
        node_id,
        port_index,
      } => {
        write!(
          f,
          "Unconnected input port {} on node {:?}",
          port_index, node_id
        )
      }
      CtLawViolation::FunctorIdentityViolation {
        node_id,
        input_type,
        output_type,
      } => {
        write!(
          f,
          "Functor identity law violation at {:?}: fmap id should preserve type, but {:?} -> {:?}",
          node_id, input_type, output_type
        )
      }
      CtLawViolation::FunctorCompositionViolation {
        path_a,
        path_b,
        result_type_a,
        result_type_b,
      } => {
        write!(
          f,
          "Functor composition law violation: path {:?} yields {:?}, but path {:?} yields {:?}",
          path_a, result_type_a, path_b, result_type_b
        )
      }
      CtLawViolation::MultiOutputTypeMismatch {
        node_id,
        output_types,
      } => {
        write!(
          f,
          "Multiple output type mismatch at {:?}: outputs have inconsistent types {:?}",
          node_id, output_types
        )
      }
    }
  }
}

/// CT 법칙 검증 결과
#[derive(Debug, Clone)]
pub struct CtLawValidationResult {
  pub is_valid: bool,
  pub violations: Vec<CtLawViolation>,
}

impl CtLawValidationResult {
  /// 검증 성공 결과 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn ok() -> Self {
    Self {
      is_valid: true,
      violations: Vec::new(),
    }
  }

  /// 검증 실패 결과 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn fail(violations: Vec<CtLawViolation>) -> Self {
    Self {
      is_valid: false,
      violations,
    }
  }
}

impl MorphismGraph {
  /// CT 법칙 검증
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 검증만, 값 계산 없음
  pub fn validate_ct_laws(&self) -> CtLawValidationResult {
    let mut violations = Vec::new();

    // 1. 사이클 검사
    if self.has_cycle() {
      violations.push(CtLawViolation::CycleDetected);
    }

    // 2. 포트 타입 호환성 검사
    let mut sorted_edges: Vec<&MorphismEdge> = self.edges.iter().collect();
    sorted_edges.sort_by_key(|edge| (edge.from.0, edge.to.0, edge.from_port, edge.to_port));
    for edge in sorted_edges {
      if let (Some(from_node), Some(to_node)) =
        (self.nodes.get(&edge.from), self.nodes.get(&edge.to))
      {
        let Some(input_type) = to_node.input_ports.get(edge.to_port).cloned() else {
          violations.push(CtLawViolation::MissingInputPortType {
            node_id: edge.to,
            port_index: edge.to_port,
          });
          continue;
        };

        if !port_types_compatible(&from_node.output_port, &input_type) {
          violations.push(CtLawViolation::PortTypeMismatch {
            edge_from: edge.from,
            edge_to: edge.to,
            output_type: from_node.output_port.clone(),
            input_type,
          });
        }
      }
    }

    // 3. 필수 입력 포트 연결 검사
    for node_id in self.sorted_node_ids() {
      let Some(node) = self.nodes.get(&node_id) else {
        continue;
      };
      let required_inputs = match &node.kind {
        MorphismNodeKind::Derived => 1,
        MorphismNodeKind::Scan => 1,
        MorphismNodeKind::Combine => 2,
        _ => 0,
      };

      let connected_inputs = self.edges.iter().filter(|e| e.to == node_id).count();

      if connected_inputs < required_inputs {
        for port_idx in connected_inputs..required_inputs {
          violations.push(CtLawViolation::UnconnectedInputPort {
            node_id,
            port_index: port_idx,
          });
        }
      }
    }

    // 4. 단절 노드 검사
    for node_id in self.find_disconnected_nodes() {
      violations.push(CtLawViolation::DisconnectedNode(node_id));
    }

    // 5. Functor Identity Law 검사
    violations.extend(self.check_functor_identity_law());

    // 6. 다중 출력 타입 일관성 검사
    violations.extend(self.check_multi_output_consistency());

    if violations.is_empty() {
      CtLawValidationResult::ok()
    } else {
      CtLawValidationResult::fail(violations)
    }
  }

  fn check_functor_identity_law(&self) -> Vec<CtLawViolation> {
    let mut violations = Vec::new();

    for node_id in self.sorted_node_ids() {
      let Some(node) = self.nodes.get(&node_id) else {
        continue;
      };
      if matches!(node.kind, MorphismNodeKind::Derived)
        && (node.name.contains("identity") || node.name.contains("_id") || node.name == "id")
      {
        if let Some(input_type) = node.input_ports.first() {
          if !port_types_compatible(input_type, &node.output_port)
            || (input_type != &node.output_port
              && !matches!(input_type, PortType::Any)
              && !matches!(node.output_port, PortType::Any))
          {
            violations.push(CtLawViolation::FunctorIdentityViolation {
              node_id,
              input_type: input_type.clone(),
              output_type: node.output_port.clone(),
            });
          }
        }
      }
    }

    violations
  }

  fn check_multi_output_consistency(&self) -> Vec<CtLawViolation> {
    let mut violations = Vec::new();

    for node_id in self.sorted_node_ids() {
      let mut outgoing: Vec<_> = self.edges.iter().filter(|e| e.from == node_id).collect();
      outgoing.sort_by_key(|edge| (edge.to.0, edge.to_port, edge.from_port));

      if outgoing.len() > 1 {
        let mut expected_types = Vec::new();
        for edge in &outgoing {
          if let Some(to_node) = self.nodes.get(&edge.to) {
            let expected = to_node
              .input_ports
              .get(edge.to_port)
              .cloned()
              .unwrap_or(PortType::Any);
            expected_types.push(expected);
          }
        }

        let concrete_types: Vec<_> = expected_types
          .iter()
          .filter(|t| !matches!(t, PortType::Any))
          .collect();

        if concrete_types.len() > 1 {
          let first = concrete_types[0];
          let all_compatible = concrete_types
            .iter()
            .all(|t| port_types_compatible(first, t));
          if !all_compatible {
            violations.push(CtLawViolation::MultiOutputTypeMismatch {
              node_id,
              output_types: expected_types,
            });
          }
        }
      }
    }

    violations
  }

  /// 단절 노드 검사
  pub fn find_disconnected_nodes(&self) -> Vec<MorphismNodeId> {
    let mut connected: HashSet<MorphismNodeId> = HashSet::new();

    for edge in &self.edges {
      connected.insert(edge.from);
      connected.insert(edge.to);
    }

    let mut disconnected: Vec<MorphismNodeId> = self
      .nodes
      .iter()
      .filter(|(id, node)| {
        !connected.contains(id)
          && !matches!(
            node.kind,
            MorphismNodeKind::Input
              | MorphismNodeKind::Constant(_)
              | MorphismNodeKind::Time
              | MorphismNodeKind::DeltaTime
          )
      })
      .map(|(id, _)| *id)
      .collect();
    disconnected.sort_by_key(|id| id.0);
    disconnected
  }
}

/// Replay artifact mismatch를 reason code로 분류한다.
pub fn classify_replay_mismatch(
  baseline: &MorphismReplayArtifact,
  replay: &MorphismReplayArtifact,
) -> Option<MorphismReplayMismatch> {
  if baseline.schema != replay.schema {
    return Some(MorphismReplayMismatch {
      reason_code: MORPHISM_REPLAY_REASON_SCHEMA_MISMATCH.to_string(),
      detail: format!(
        "schema mismatch: baseline={} replay={}",
        baseline.schema, replay.schema
      ),
    });
  }
  if baseline.graph_name != replay.graph_name {
    return Some(MorphismReplayMismatch {
      reason_code: MORPHISM_REPLAY_REASON_GRAPH_NAME_MISMATCH.to_string(),
      detail: format!(
        "graph_name mismatch: baseline={} replay={}",
        baseline.graph_name, replay.graph_name
      ),
    });
  }
  if baseline.seed != replay.seed {
    return Some(MorphismReplayMismatch {
      reason_code: MORPHISM_REPLAY_REASON_SEED_MISMATCH.to_string(),
      detail: format!(
        "seed mismatch: baseline={} replay={}",
        baseline.seed, replay.seed
      ),
    });
  }
  if baseline.nodes != replay.nodes {
    return Some(MorphismReplayMismatch {
      reason_code: MORPHISM_REPLAY_REASON_NODE_SET_MISMATCH.to_string(),
      detail: format!(
        "node snapshot mismatch: baseline_count={} replay_count={}",
        baseline.nodes.len(),
        replay.nodes.len()
      ),
    });
  }
  if baseline.edges != replay.edges {
    return Some(MorphismReplayMismatch {
      reason_code: MORPHISM_REPLAY_REASON_EDGE_SET_MISMATCH.to_string(),
      detail: format!(
        "edge snapshot mismatch: baseline_count={} replay_count={}",
        baseline.edges.len(),
        replay.edges.len()
      ),
    });
  }
  if baseline.hash_hex() != replay.hash_hex() {
    return Some(MorphismReplayMismatch {
      reason_code: MORPHISM_REPLAY_REASON_HASH_MISMATCH.to_string(),
      detail: "artifact hash mismatch".to_string(),
    });
  }

  None
}

// ═══════════════════════════════════════════════════════════════
// 7. Helper Functions
// ═══════════════════════════════════════════════════════════════

fn canonical_port_type(port: &PortType) -> String {
  match port {
    PortType::Float => "Float".to_string(),
    PortType::Int => "Int".to_string(),
    PortType::Time => "Time".to_string(),
    PortType::DeltaTime => "DeltaTime".to_string(),
    PortType::Any => "Any".to_string(),
    PortType::Signal(inner) => format!("Signal<{}>", canonical_port_type(inner)),
  }
}

fn canonical_node_kind(kind: &MorphismNodeKind) -> String {
  match kind {
    MorphismNodeKind::Input => "Input".to_string(),
    MorphismNodeKind::Constant(value) => format!("Constant({value})"),
    MorphismNodeKind::Time => "Time".to_string(),
    MorphismNodeKind::DeltaTime => "DeltaTime".to_string(),
    MorphismNodeKind::Derived => "Derived".to_string(),
    MorphismNodeKind::State => "State".to_string(),
    MorphismNodeKind::Scan => "Scan".to_string(),
    MorphismNodeKind::Combine => "Combine".to_string(),
  }
}

fn append_len_prefixed(payload: &mut String, value: &str) {
  payload.push_str(&value.len().to_string());
  payload.push(':');
  payload.push_str(value);
  payload.push('|');
}

fn canonical_replay_edges(edges: &[MorphismEdge]) -> Vec<String> {
  let mut by_target: HashMap<usize, Vec<&MorphismEdge>> = HashMap::new();
  for edge in edges {
    by_target.entry(edge.to.0).or_default().push(edge);
  }

  let mut target_ids: Vec<usize> = by_target.keys().copied().collect();
  target_ids.sort_unstable();

  let mut normalized = Vec::new();
  for target_id in target_ids {
    let Some(incoming) = by_target.get(&target_id) else {
      continue;
    };
    let mut incoming = incoming.clone();

    let mut to_ports: Vec<usize> = incoming.iter().map(|edge| edge.to_port).collect();
    to_ports.sort_unstable();
    let is_dense_unique = to_ports.iter().enumerate().all(|(idx, port)| *port == idx);

    if is_dense_unique {
      // add_edge()로 구성된 그래프는 to_port가 순서 의존적이므로
      // replay artifact에서는 입력 집합 기준으로 정규화해 결정성을 보장한다.
      incoming.sort_by_key(|edge| (edge.from.0, edge.from_port, edge.to_port));
      for (canonical_to_port, edge) in incoming.into_iter().enumerate() {
        normalized.push((edge.from.0, edge.from_port, edge.to.0, canonical_to_port));
      }
    } else {
      for edge in incoming {
        normalized.push((edge.from.0, edge.from_port, edge.to.0, edge.to_port));
      }
    }
  }

  normalized.sort_by_key(|(from, from_port, to, to_port)| (*from, *to, *to_port, *from_port));
  normalized
    .into_iter()
    .map(|(from, from_port, to, to_port)| format!("{from}:{from_port}->{to}:{to_port}"))
    .collect()
}

fn fnv1a64(bytes: &[u8]) -> u64 {
  let mut hash = 0xcbf29ce484222325u64;
  for byte in bytes {
    hash ^= u64::from(*byte);
    hash = hash.wrapping_mul(0x100000001b3);
  }
  hash
}

/// SignalKind → MorphismNodeKind 변환 (pnix-core SignalKind 사용)
fn signal_kind_to_morphism_kind(kind: &SignalKind) -> MorphismNodeKind {
  match kind {
    SignalKind::Input => MorphismNodeKind::Input,
    SignalKind::Constant(v) => MorphismNodeKind::Constant(v.to_string()),
    SignalKind::Time => MorphismNodeKind::Time,
    SignalKind::DeltaTime => MorphismNodeKind::DeltaTime,
    SignalKind::Derived { .. } => MorphismNodeKind::Derived,
    SignalKind::State { .. } => MorphismNodeKind::State,
    SignalKind::Scan { .. } => MorphismNodeKind::Scan,
    SignalKind::Combine2 { .. } => MorphismNodeKind::Combine,
  }
}

/// 노드 종류별 색상
fn kind_to_color(kind: &MorphismNodeKind) -> &'static str {
  match kind {
    MorphismNodeKind::Input => "lightblue",
    MorphismNodeKind::Constant(_) => "lightyellow",
    MorphismNodeKind::Time => "lightgreen",
    MorphismNodeKind::DeltaTime => "lightgreen",
    MorphismNodeKind::Derived => "white",
    MorphismNodeKind::State => "lightpink",
    MorphismNodeKind::Scan => "lightgray",
    MorphismNodeKind::Combine => "lightgray",
  }
}

/// DOT 이름 정리
fn sanitize_name(name: &str) -> String {
  name
    .chars()
    .map(|c| {
      if c.is_alphanumeric() || c == '_' {
        c
      } else {
        '_'
      }
    })
    .collect()
}

/// 포트 타입 호환성 검사
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 분석만, 값 계산 없음
pub fn port_types_compatible(output: &PortType, input: &PortType) -> bool {
  if matches!(output, PortType::Any) || matches!(input, PortType::Any) {
    return true;
  }

  if output == input {
    return true;
  }

  if let (PortType::Signal(out_inner), PortType::Signal(in_inner)) = (output, input) {
    return port_types_compatible(out_inner, in_inner);
  }

  if matches!(
    (output, input),
    (PortType::Float, PortType::Int) | (PortType::Int, PortType::Float)
  ) {
    return true;
  }

  if matches!(output, PortType::Time | PortType::DeltaTime) && matches!(input, PortType::Float) {
    return true;
  }

  false
}

// ═══════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_create_graph() {
    let mut graph = MorphismGraph::new("test");

    let input = graph.add_node("mouse_x", MorphismNodeKind::Input);
    let time = graph.add_node("time", MorphismNodeKind::Time);
    let derived = graph.add_node("pos", MorphismNodeKind::Derived);

    graph.add_edge(input, derived);
    graph.add_edge(time, derived);

    assert_eq!(graph.node_count(), 3);
    assert_eq!(graph.edge_count(), 2);
  }

  #[test]
  fn test_to_dot() {
    let mut graph = MorphismGraph::new("test_graph");

    let a = graph.add_node("A", MorphismNodeKind::Input);
    let b = graph.add_node("B", MorphismNodeKind::Derived);
    graph.add_edge(a, b);

    let dot = graph.to_dot();
    assert!(dot.contains("digraph"));
    assert!(dot.contains("n0 -> n1"));
  }

  #[test]
  fn test_to_dot_preserves_port_mapping_for_parallel_edges() {
    let mut graph = MorphismGraph::new("dot_ports");
    let a = graph.add_node("A", MorphismNodeKind::Input);
    let b = graph.add_node("B", MorphismNodeKind::Derived);
    graph.add_edge(a, b);
    graph.add_edge(a, b);

    let dot = graph.to_dot();
    assert!(dot.contains("n0 -> n1 [label=\"0:0\"]"));
    assert!(dot.contains("n0 -> n1 [label=\"0:1\"]"));
  }

  #[test]
  fn test_to_dot_is_deterministic_across_equivalent_graphs() {
    let mut g1 = MorphismGraph::new("det_graph");
    let g1_a = g1.add_node("A", MorphismNodeKind::Input);
    let g1_b = g1.add_node("B", MorphismNodeKind::Derived);
    let g1_c = g1.add_node("C", MorphismNodeKind::Derived);
    g1.add_edge(g1_a, g1_b);
    g1.add_edge(g1_a, g1_c);
    g1.add_edge(g1_b, g1_c);

    let mut g2 = MorphismGraph::new("det_graph");
    let g2_a = g2.add_node("A", MorphismNodeKind::Input);
    let g2_b = g2.add_node("B", MorphismNodeKind::Derived);
    let g2_c = g2.add_node("C", MorphismNodeKind::Derived);
    g2.add_edge(g2_a, g2_b);
    g2.add_edge(g2_a, g2_c);
    g2.add_edge(g2_b, g2_c);

    assert_eq!(g1.to_dot(), g2.to_dot());
  }

  #[test]
  fn test_node_replacement() {
    let mut graph = MorphismGraph::new("test");

    let old_id = graph.add_node("old", MorphismNodeKind::Input);
    let target = graph.add_node("target", MorphismNodeKind::Derived);
    graph.add_edge(old_id, target);

    let new_node = MorphismNode::new(MorphismNodeId(0), "new", MorphismNodeKind::Input);

    let new_id = graph.replace_node(old_id, new_node).unwrap();
    assert_ne!(old_id, new_id);
    assert!(graph.get_node(old_id).is_none());
    assert!(graph.get_node(new_id).is_some());
    assert!(graph.edges.iter().any(|e| e.from == new_id));
  }

  #[test]
  fn test_cycle_detection() {
    let mut graph = MorphismGraph::new("test");

    let a = graph.add_node("A", MorphismNodeKind::Derived);
    let b = graph.add_node("B", MorphismNodeKind::Derived);
    let c = graph.add_node("C", MorphismNodeKind::Derived);

    graph.add_edge(a, b);
    graph.add_edge(b, c);
    assert!(!graph.has_cycle());

    graph.add_edge(c, a);
    assert!(graph.has_cycle());
  }

  #[test]
  fn test_port_type() {
    assert_eq!(PortType::Float.name(), "Float");
    assert_eq!(PortType::default_signal().name(), "Signal");
  }

  #[test]
  fn test_ct_law_valid_graph() {
    let mut graph = MorphismGraph::new("valid");

    let input = graph.add_node("input", MorphismNodeKind::Input);
    let derived = graph.add_node("derived", MorphismNodeKind::Derived);
    graph.add_edge(input, derived);

    let result = graph.validate_ct_laws();
    assert!(result.is_valid);
    assert!(result.violations.is_empty());
  }

  #[test]
  fn test_ct_law_cycle_violation() {
    let mut graph = MorphismGraph::new("cyclic");

    let a = graph.add_node("A", MorphismNodeKind::Derived);
    let b = graph.add_node("B", MorphismNodeKind::Derived);

    graph.add_edge(a, b);
    graph.add_edge(b, a);

    let result = graph.validate_ct_laws();
    assert!(!result.is_valid);
    assert!(result
      .violations
      .iter()
      .any(|v| matches!(v, CtLawViolation::CycleDetected)));
  }

  #[test]
  fn test_port_type_compatibility() {
    assert!(port_types_compatible(&PortType::Any, &PortType::Float));
    assert!(port_types_compatible(&PortType::Float, &PortType::Any));
    assert!(port_types_compatible(&PortType::Float, &PortType::Float));
    assert!(port_types_compatible(&PortType::Float, &PortType::Int));
    assert!(port_types_compatible(&PortType::Time, &PortType::Float));
  }

  #[test]
  fn test_missing_input_port_type_violation() {
    let mut graph = MorphismGraph::new("missing_port");

    let a = graph.add_node("A", MorphismNodeKind::Derived);
    let b = graph.add_node("B", MorphismNodeKind::Derived);
    let c = graph.add_node("C", MorphismNodeKind::Derived);

    graph.add_edge(a, b);
    graph.add_edge(c, b);

    let result = graph.validate_ct_laws();
    assert!(!result.is_valid);
    assert!(result.violations.iter().any(|v| {
      matches!(
        v,
        CtLawViolation::MissingInputPortType {
          node_id,
          port_index
        } if *node_id == b && *port_index == 1
      )
    }));
  }

  #[test]
  fn test_find_disconnected_nodes_are_sorted_deterministically() {
    let mut graph = MorphismGraph::new("disc_sorted");
    let first = graph.add_node("d1", MorphismNodeKind::Derived);
    let second = graph.add_node("d2", MorphismNodeKind::Derived);
    let _input = graph.add_node("input", MorphismNodeKind::Input);

    let disconnected = graph.find_disconnected_nodes();
    assert_eq!(disconnected, vec![first, second]);
  }

  #[test]
  fn test_validate_ct_laws_disconnected_violation_order_is_sorted() {
    let mut graph = MorphismGraph::new("disc_violation_order");
    let first = graph.add_node("d1", MorphismNodeKind::Derived);
    let second = graph.add_node("d2", MorphismNodeKind::Derived);
    let _input = graph.add_node("input", MorphismNodeKind::Input);

    let result = graph.validate_ct_laws();
    let disconnected: Vec<MorphismNodeId> = result
      .violations
      .iter()
      .filter_map(|violation| match violation {
        CtLawViolation::DisconnectedNode(node_id) => Some(*node_id),
        _ => None,
      })
      .collect();
    assert_eq!(disconnected, vec![first, second]);
  }

  #[test]
  fn test_replay_artifact_is_deterministic_when_edge_storage_order_changes() {
    let mut graph = MorphismGraph::new("replay_det");
    let a = graph.add_node("A", MorphismNodeKind::Input);
    let b = graph.add_node("B", MorphismNodeKind::Derived);
    let c = graph.add_node("C", MorphismNodeKind::Derived);
    graph.add_edge(a, b);
    graph.add_edge(a, c);
    graph.add_edge(b, c);

    let baseline = graph.replay_artifact(77);
    graph.edges.reverse();
    let replay = graph.replay_artifact(77);

    assert_eq!(baseline, replay);
    assert_eq!(baseline.hash_hex(), replay.hash_hex());
    assert!(classify_replay_mismatch(&baseline, &replay).is_none());
  }

  #[test]
  fn test_replay_artifact_is_deterministic_when_incoming_edge_order_differs() {
    let mut baseline_graph = MorphismGraph::new("replay_incoming_order");
    let in_a = baseline_graph.add_node("in_a", MorphismNodeKind::Input);
    let in_b = baseline_graph.add_node("in_b", MorphismNodeKind::Input);
    let combine = baseline_graph.add_node("combine", MorphismNodeKind::Combine);
    baseline_graph.add_edge(in_a, combine);
    baseline_graph.add_edge(in_b, combine);

    let mut replay_graph = MorphismGraph::new("replay_incoming_order");
    let replay_a = replay_graph.add_node("in_a", MorphismNodeKind::Input);
    let replay_b = replay_graph.add_node("in_b", MorphismNodeKind::Input);
    let replay_combine = replay_graph.add_node("combine", MorphismNodeKind::Combine);
    replay_graph.add_edge(replay_b, replay_combine);
    replay_graph.add_edge(replay_a, replay_combine);

    let baseline = baseline_graph.replay_artifact(2026);
    let replay = replay_graph.replay_artifact(2026);

    assert_eq!(baseline.edges, replay.edges);
    assert_eq!(baseline.hash_hex(), replay.hash_hex());
    assert!(classify_replay_mismatch(&baseline, &replay).is_none());
  }

  #[test]
  fn test_replay_mismatch_classifies_seed_and_edge_reason_codes() {
    let mut graph = MorphismGraph::new("replay_reason");
    let a = graph.add_node("A", MorphismNodeKind::Input);
    let b = graph.add_node("B", MorphismNodeKind::Derived);
    graph.add_edge(a, b);

    let baseline = graph.replay_artifact(1);
    let seed_changed = graph.replay_artifact(2);
    let seed_mismatch = classify_replay_mismatch(&baseline, &seed_changed).expect("seed mismatch");
    assert_eq!(
      seed_mismatch.reason_code,
      MORPHISM_REPLAY_REASON_SEED_MISMATCH
    );

    let mut edge_changed_graph = graph.clone();
    edge_changed_graph.edges.clear();
    let edge_changed = edge_changed_graph.replay_artifact(1);
    let edge_mismatch = classify_replay_mismatch(&baseline, &edge_changed).expect("edge mismatch");
    assert_eq!(
      edge_mismatch.reason_code,
      MORPHISM_REPLAY_REASON_EDGE_SET_MISMATCH
    );
  }

  #[test]
  fn test_replay_hash_encoding_is_unambiguous_for_node_edge_boundaries() {
    let baseline = MorphismReplayArtifact {
      schema: MORPHISM_REPLAY_ARTIFACT_SCHEMA.to_string(),
      graph_name: "hash_boundary".to_string(),
      seed: 9,
      nodes: vec!["n0".to_string(), "n1".to_string()],
      edges: vec!["e0".to_string()],
    };
    let replay = MorphismReplayArtifact {
      schema: MORPHISM_REPLAY_ARTIFACT_SCHEMA.to_string(),
      graph_name: "hash_boundary".to_string(),
      seed: 9,
      nodes: vec!["n0".to_string()],
      edges: vec!["n1|e0".to_string()],
    };

    let legacy_payload_baseline = format!(
      "{}|{}|{}|{}|{}",
      baseline.schema,
      baseline.graph_name,
      baseline.seed,
      baseline.nodes.join("|"),
      baseline.edges.join("|")
    );
    let legacy_payload_replay = format!(
      "{}|{}|{}|{}|{}",
      replay.schema,
      replay.graph_name,
      replay.seed,
      replay.nodes.join("|"),
      replay.edges.join("|")
    );
    assert_eq!(
      legacy_payload_baseline, legacy_payload_replay,
      "legacy payload format is ambiguous for node/edge boundaries"
    );
    assert_ne!(baseline.nodes, replay.nodes);
    assert_ne!(baseline.edges, replay.edges);
    assert_ne!(baseline.hash_hex(), replay.hash_hex());
  }
}
