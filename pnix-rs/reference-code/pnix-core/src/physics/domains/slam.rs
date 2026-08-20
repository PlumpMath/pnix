//! SLAM (Simultaneous Localization and Mapping) as CT Morphisms
//!
//! pnix-old의 physics_ct/src/domains/slam.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 값 계산 함수 제외
//! - `compose()`, `inverse()`, `diff()`, `distance_to()` 등 값 계산 함수 제외
//!
//! ## 설계 원칙
//!
//! SLAM can be modeled as a factor graph optimization problem:
//! - **Pose Graph**: Vertices are robot poses (SE(2) or SE(3))
//! - **Landmarks**: Observed features in the environment
//! - **Factors**: Constraints (odometry, loop closures, observations)
//!
//! ## CT Mapping
//!
//! - **State** = Object in the SLAM category (poses + landmarks)
//! - **Factor** = Morphism (constraint between states)
//! - **Loop Closure** = 2-morphism (path equivalence in pose graph)
//! - **Optimization** = Finding the most likely state given all factors

use serde::{Deserialize, Serialize};

/// 2D Pose (x, y, theta) for planar SLAM
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Pose2D {
  pub x: f64,
  pub y: f64,
  pub theta: f64,
}

impl Default for Pose2D {
  fn default() -> Self {
    Self {
      x: 0.0,
      y: 0.0,
      theta: 0.0,
    }
  }
}

impl Pose2D {
  pub fn new(x: f64, y: f64, theta: f64) -> Self {
    Self { x, y, theta }
  }

  // **주의**: 다음 함수들은 값 계산이므로 executor에서 구현됩니다:
  // - `compose(&self, other: &Pose2D) -> Pose2D`: 포즈 합성
  // - `inverse(&self) -> Pose2D`: 역 포즈
  // - `diff(&self, other: &Pose2D) -> Pose2D`: 포즈 차이
  // - `distance_to(&self, other: &Pose2D) -> f64`: 거리 계산
  // - `to_se3(&self) -> Isometry3`: SE(3) 변환
}

/// 2D Landmark (point feature)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Landmark2D {
  pub id: u64,
  pub x: f64,
  pub y: f64,
}

impl Landmark2D {
  pub fn new(id: u64, x: f64, y: f64) -> Self {
    Self { id, x, y }
  }
}

/// Factor type in the SLAM factor graph
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FactorType {
  /// Odometry factor (pose-to-pose constraint)
  Odometry,
  /// Loop closure factor (pose-to-pose constraint)
  LoopClosure,
  /// Observation factor (pose-to-landmark constraint)
  Observation,
  /// Prior factor (absolute pose constraint)
  Prior,
}

/// SLAM state = Object in the SLAM Category
///
/// Represents the complete state of a SLAM system (poses + landmarks).
///
/// # Executor 구현
///
/// 다음 메서드들은 executor에서 구현됩니다:
/// - `energy() -> f64`: 총 에너지 계산
/// - `momentum() -> Vec3`: 운동량 계산
/// - `add_pose()`, `add_landmark()`: 상태 변경
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlamState {
  /// Robot poses (SE(2) or SE(3))
  pub poses: Vec<Pose2D>,
  /// Observed landmarks
  pub landmarks: Vec<Landmark2D>,
  /// Factor graph edges (factor type, source, target)
  pub factors: Vec<(FactorType, usize, usize)>,
}

impl SlamState {
  /// Create a new empty SLAM state
  pub fn new() -> Self {
    Self {
      poses: Vec::new(),
      landmarks: Vec::new(),
      factors: Vec::new(),
    }
  }

  // **주의**: 다음 함수들은 값 계산/상태 변경이므로 executor에서 구현됩니다:
  // - `add_pose()`, `add_landmark()`, `add_factor()`: 상태 변경
  // - `energy() -> f64`: 총 에너지 계산
  // - `optimize()`: 최적화 실행
}

impl Default for SlamState {
  fn default() -> Self {
    Self::new()
  }
}
