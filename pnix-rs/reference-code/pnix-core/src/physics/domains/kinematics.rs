//! Robot Kinematics as CT Morphisms
//!
//! pnix-old의 physics_ct/src/domains/kinematics.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 값 계산 함수 제외
//! - `energy()`, `momentum()`, `is_valid()`, `distance_to()` 등 값 계산 함수 제외
//! - `forward_kinematics()`, `inverse_kinematics()` 등 값 계산 함수 제외
//!
//! ## 설계 원칙
//!
//! A robot arm's configuration lives on:
//! - Joint space: Qⁿ (n joint angles)
//! - Task space: SE(3) (end-effector pose)
//!
//! ## CT Mapping
//!
//! - **Joint Config** = Object in the Kinematics category
//! - **FK (Forward Kinematics)** = Functor: JointSpace → TaskSpace
//! - **IK (Inverse Kinematics)** = Morphism finding preimage of FK
//! - **Jacobian** = Natural transformation (differential of FK)

use serde::{Deserialize, Serialize};

/// Joint configuration = Object in the Kinematics Category
///
/// Represents the complete state of a robot arm's joints.
///
/// # Executor 구현
///
/// 다음 메서드들은 executor에서 구현됩니다:
/// - `energy() -> f64`: 운동 에너지 계산
/// - `momentum() -> Vec3`: 운동량 계산
/// - `is_valid() -> bool`: 조인트 한계 확인
/// - `distance_to(&self, other: &JointConfig) -> f64`: 거리 계산
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JointConfig {
  /// Joint angles (radians)
  pub angles: Vec<f64>,
  /// Joint velocities (rad/s)
  pub velocities: Vec<f64>,
  /// Number of degrees of freedom
  pub dof: usize,
  /// Joint limits (min, max) for each joint
  pub limits: Vec<(f64, f64)>,
}

impl JointConfig {
  /// Create a new joint configuration with given DOF
  pub fn new(dof: usize) -> Self {
    Self {
      angles: vec![0.0; dof],
      velocities: vec![0.0; dof],
      dof,
      limits: vec![(-std::f64::consts::PI, std::f64::consts::PI); dof],
    }
  }

  /// Create from angles with default limits
  pub fn from_angles(angles: Vec<f64>) -> Self {
    let dof = angles.len();
    Self {
      angles,
      velocities: vec![0.0; dof],
      dof,
      limits: vec![(-std::f64::consts::PI, std::f64::consts::PI); dof],
    }
  }

  // **주의**: 다음 함수들은 값 계산이므로 executor에서 구현됩니다:
  // - `is_valid() -> bool`: 조인트 한계 확인
  // - `distance_to(&self, other: &JointConfig) -> f64`: 거리 계산
  // - `energy() -> f64`: 운동 에너지 계산
  // - `momentum() -> Vec3`: 운동량 계산
}

/// Denavit-Hartenberg parameters for a robot link
///
/// Standard DH parameters for forward kinematics calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DenavitHartenbergParams {
  /// Link length (a)
  pub a: f64,
  /// Link twist (α)
  pub alpha: f64,
  /// Link offset (d)
  pub d: f64,
  /// Joint angle (θ)
  pub theta: f64,
}

impl DenavitHartenbergParams {
  /// Create new DH parameters
  pub fn new(a: f64, alpha: f64, d: f64, theta: f64) -> Self {
    Self { a, alpha, d, theta }
  }
}
