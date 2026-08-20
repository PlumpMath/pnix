//! Rigid Body Dynamics as CT Morphisms
//!
//! pnix-old의 physics_ct/src/domains/rigid_body.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 값 계산 함수 제외
//! - `energy()`, `momentum()`, `position()`, `orientation()` 등 값 계산 함수 제외
//! - `box_body()`, `sphere_body()` 등 값 계산 함수 제외
//!
//! ## 설계 원칙
//!
//! A rigid body's state lives on SE(3) × se(3):
//! - Position + Orientation: SE(3) = R³ × SO(3)
//! - Linear + Angular velocity: se(3) (the Lie algebra)
//!
//! ## CT Mapping
//!
//! - **State** = Object in the physics category
//! - **Motion** = Morphism (Newton-Euler integration)
//! - **Collision** = 2-morphism (path equivalence)

use serde::{Deserialize, Serialize};

/// Twist = velocity of a rigid body (linear + angular)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Twist {
  /// Linear velocity (m/s)
  pub linear: crate::physics::Vec3,
  /// Angular velocity (rad/s)
  pub angular: crate::physics::Vec3,
}

impl Default for Twist {
  fn default() -> Self {
    Self {
      linear: crate::physics::Vec3::zeros(),
      angular: crate::physics::Vec3::zeros(),
    }
  }
}

/// Wrench = force + torque on a rigid body
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Wrench {
  /// Force (N)
  pub force: crate::physics::Vec3,
  /// Torque (N⋅m)
  pub torque: crate::physics::Vec3,
}

impl Default for Wrench {
  fn default() -> Self {
    Self {
      force: crate::physics::Vec3::zeros(),
      torque: crate::physics::Vec3::zeros(),
    }
  }
}

/// Rigid body state = Object in the Physics Category
///
/// Represents the complete state of a rigid body at an instant.
///
/// # Executor 구현
///
/// 다음 메서드들은 executor에서 구현됩니다:
/// - `energy() -> f64`: 총 에너지 계산
/// - `momentum() -> Vec3`: 선형 운동량 계산
/// - `angular_momentum() -> Vec3`: 각운동량 계산
/// - `position() -> Vec3`: 위치 계산
/// - `orientation() -> Quat`: 방향 계산
/// - `box_body()`, `sphere_body()`: 관성 텐서 계산
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RigidBodyState {
  /// Position (x, y, z)
  pub position: crate::physics::Vec3,
  /// Orientation (quaternion: w, x, y, z)
  pub orientation: (f64, f64, f64, f64), // (w, x, y, z)
  /// Linear and angular velocity
  pub twist: Twist,
  /// Mass (kg)
  pub mass: f64,
  /// Inertia tensor (3x3 matrix, flattened: [m00, m01, m02, m10, m11, m12, m20, m21, m22])
  pub inertia: [f64; 9],
  /// Identifier for this body
  pub id: u64,
}

impl RigidBodyState {
  /// Create a new rigid body at the origin with unit mass
  pub fn new(mass: f64, inertia: [f64; 9]) -> Self {
    Self {
      position: crate::physics::Vec3::zeros(),
      orientation: (1.0, 0.0, 0.0, 0.0), // identity quaternion
      twist: Twist::default(),
      mass,
      inertia,
      id: 0,
    }
  }

  // **주의**: 다음 함수들은 값 계산이므로 executor에서 구현됩니다:
  // - `box_body(mass, half_extents) -> Self`: 박스 관성 텐서 계산
  // - `sphere_body(mass, radius) -> Self`: 구 관성 텐서 계산
  // - `position() -> Vec3`: 위치 반환
  // - `orientation() -> Quat`: 방향 반환
  // - `energy() -> f64`: 에너지 계산
  // - `momentum() -> Vec3`: 운동량 계산
}
