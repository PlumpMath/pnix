//! Petgraph Wrapper 구조 정의
//!
//! pnix-old의 pnix_petgraph_wrapper/src/lib.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 실행 로직 제외
//! - PoseNode: SLAM 포즈 노드 구조 정의
//! - PoseEdge: SLAM 제약 엣지 구조 정의
//! - PnixGraph: 그래프 구조 정의 (실행 로직 제외)
//! - 실제 그래프 최적화, 노드/엣지 추가 로직은 executor에서 구현

use serde::{Deserialize, Serialize};

/// SLAM 포즈 그래프 노드 데이터: SLAM 포즈 그래프의 노드 구조
///
/// 헌법 P0-1 준수: 구조 정의만, 값 계산 없음
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PoseNode {
  /// 노드 ID
  pub id: usize,
  /// X 좌표
  pub x: f64,
  /// Y 좌표
  pub y: f64,
  /// 방향 (theta)
  pub theta: f64,
}

impl PoseNode {
  /// 새로운 포즈 노드 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(id: usize, x: f64, y: f64, theta: f64) -> Self {
    Self { id, x, y, theta }
  }
}

/// SLAM 제약 엣지 데이터: SLAM 포즈 그래프의 엣지 구조
///
/// 헌법 P0-1 준수: 구조 정의만, 값 계산 없음
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PoseEdge {
  /// X 방향 변위
  pub dx: f64,
  /// Y 방향 변위
  pub dy: f64,
  /// 방향 변위
  pub dtheta: f64,
  /// 정보 행렬 (3x3, 평탄화된 형태)
  pub info_matrix: Vec<f64>,
}

impl PoseEdge {
  /// 새로운 포즈 엣지 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(dx: f64, dy: f64, dtheta: f64, info_matrix: Vec<f64>) -> Self {
    Self {
      dx,
      dy,
      dtheta,
      info_matrix,
    }
  }
}

/// Pnix 그래프 구조: 포즈 그래프를 표현하는 구조
///
/// **주의**: 실제 그래프 연산은 executor에서 구현합니다.
/// 이 구조는 그래프 상태만 정의합니다.
/// 헌법 P0-1 준수: 구조 정의만, 값 계산 없음
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PnixGraph {
  /// 노드 목록
  pub nodes: Vec<PoseNode>,
  /// 엣지 목록 (from_id, to_id, edge_data)
  pub edges: Vec<(usize, usize, PoseEdge)>,
  /// 노드 ID → 인덱스 매핑
  pub node_id_to_index: Vec<(usize, usize)>,
}

impl PnixGraph {
  /// 새로운 그래프 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new() -> Self {
    Self {
      nodes: Vec::new(),
      edges: Vec::new(),
      node_id_to_index: Vec::new(),
    }
  }

  /// 노드 추가 (구조 변경만)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn add_node(&mut self, node: PoseNode) {
    let index = self.nodes.len();
    self.nodes.push(node.clone());
    self.node_id_to_index.push((node.id, index));
  }

  /// 엣지 추가 (구조 변경만)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn add_edge(&mut self, from_id: usize, to_id: usize, edge: PoseEdge) {
    self.edges.push((from_id, to_id, edge));
  }

  /// 노드 개수 조회
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn node_count(&self) -> usize {
    self.nodes.len()
  }

  /// 엣지 개수 조회
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn edge_count(&self) -> usize {
    self.edges.len()
  }
}

impl Default for PnixGraph {
  fn default() -> Self {
    Self::new()
  }
}

// 헌법 준수 (P0-1): 실행 로직 제거
// 다음 함수들은 executor/runtime 계층에서 구현하세요:
// - optimize_pose_graph() (그래프 최적화 실행)
// - 실제 petgraph 연산 (노드/엣지 추가, 그래프 순회 등)
//
// 이 함수들은 값 계산 및 실행을 수행하므로 pnix-core에서 제외됩니다.

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_pose_node() {
    let node = PoseNode::new(0, 1.0, 2.0, 0.5);
    assert_eq!(node.id, 0);
    assert_eq!(node.x, 1.0);
  }

  #[test]
  fn test_pose_edge() {
    let edge = PoseEdge::new(
      1.0,
      2.0,
      0.5,
      vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
    );
    assert_eq!(edge.dx, 1.0);
  }

  #[test]
  fn test_pnix_graph() {
    let mut graph = PnixGraph::new();
    graph.add_node(PoseNode::new(0, 0.0, 0.0, 0.0));
    graph.add_node(PoseNode::new(1, 1.0, 1.0, 0.0));
    graph.add_edge(0, 1, PoseEdge::new(1.0, 1.0, 0.0, vec![1.0; 9]));
    assert_eq!(graph.node_count(), 2);
    assert_eq!(graph.edge_count(), 1);
  }
}
