//! Constraint System as CT Morphisms
//!
//! pnix-old의 physics_ct/src/domains/constraint.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 값 계산 함수 제외
//! - `energy()`, `momentum()`, `kinetic_energy()` 등 값 계산 함수 제외
//! - `solve()`, `project()` 등 값 계산 함수 제외
//!
//! ## 설계 원칙
//!
//! A constraint restricts the configuration space Q to a submanifold C ⊂ Q:
//! - Holonomic constraints: g(q) = 0 (position-level)
//! - Non-holonomic constraints: A(q)q̇ = 0 (velocity-level)
//!
//! ## CT Mapping
//!
//! - **ConstrainedState** = Object in a restricted subcategory
//! - **ConstraintProjection** = Morphism projecting Q → C
//! - **ConstraintSolver** = Functor that enforces constraints at each step

use serde::{Deserialize, Serialize};

/// A constrained body with identifier
///
/// # Executor 구현
///
/// 다음 메서드들은 executor에서 구현됩니다:
/// - `energy() -> f64`: 에너지 계산
/// - `momentum() -> Vec3`: 운동량 계산
/// - `kinetic_energy() -> f64`: 운동 에너지 계산
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstrainedBody {
  /// Unique identifier for the body
  pub id: u64,
  /// Position (m)
  pub position: crate::physics::Vec3,
  /// Orientation (quaternion: w, x, y, z)
  pub orientation: (f64, f64, f64, f64),
  /// Linear velocity (m/s)
  pub linear_velocity: crate::physics::Vec3,
  /// Angular velocity (rad/s)
  pub angular_velocity: crate::physics::Vec3,
  /// Mass (kg)
  pub mass: f64,
  /// Inverse mass (1/kg, 0 for static bodies)
  pub inv_mass: f64,
  /// Is this a static (immovable) body?
  pub is_static: bool,
}

impl Default for ConstrainedBody {
  fn default() -> Self {
    Self {
      id: 0,
      position: crate::physics::Vec3::zeros(),
      orientation: (1.0, 0.0, 0.0, 0.0), // identity quaternion
      linear_velocity: crate::physics::Vec3::zeros(),
      angular_velocity: crate::physics::Vec3::zeros(),
      mass: 1.0,
      inv_mass: 1.0,
      is_static: false,
    }
  }
}

impl ConstrainedBody {
  /// Create a new dynamic body
  pub fn new(id: u64, mass: f64) -> Self {
    Self {
      id,
      mass,
      inv_mass: if mass > 1e-10 { 1.0 / mass } else { 0.0 },
      ..Default::default()
    }
  }

  /// Create a static (immovable) body
  pub fn new_static(id: u64) -> Self {
    Self {
      id,
      mass: f64::INFINITY,
      inv_mass: 0.0,
      is_static: true,
      ..Default::default()
    }
  }

  /// Set position
  pub fn with_position(mut self, position: crate::physics::Vec3) -> Self {
    self.position = position;
    self
  }

  /// Set orientation
  pub fn with_orientation(mut self, orientation: (f64, f64, f64, f64)) -> Self {
    self.orientation = orientation;
    self
  }

  // **주의**: 다음 함수들은 값 계산이므로 executor에서 구현됩니다:
  // - `kinetic_energy() -> f64`: 운동 에너지 계산
  // - `energy() -> f64`: 총 에너지 계산
  // - `momentum() -> Vec3`: 운동량 계산
}

/// Joint type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JointType {
  /// Revolute joint (rotation around a single axis)
  Revolute,
  /// Prismatic joint (translation along a single axis)
  Prismatic,
  /// Fixed joint (no relative motion)
  Fixed,
  /// Ball joint (3-DOF rotation, spherical)
  Ball,
  /// Universal joint (2-DOF rotation)
  Universal,
}

/// Constraint type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConstraintType {
  /// Holonomic constraint: g(q) = 0
  Holonomic,
  /// Non-holonomic constraint: A(q)q̇ = 0
  NonHolonomic,
  /// Unilateral constraint: g(q) ≥ 0 (contact)
  Unilateral,
}
