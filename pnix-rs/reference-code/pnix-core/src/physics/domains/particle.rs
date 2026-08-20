//! Particle System Dynamics as CT Morphisms
//!
//! pnix-old의 physics_ct/src/domains/particle.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 값 계산 함수 제외
//! - `energy()`, `momentum()`, `kinetic_energy()` 등 값 계산 함수 제외
//!
//! ## 설계 원칙
//!
//! An N-particle system's state is a point in phase space R^(6N):
//! - Positions: N × R³
//! - Velocities: N × R³
//!
//! ## CT Mapping
//!
//! - **ParticleSystemState** = Object (configuration of all particles)
//! - **ParticleSystemStep** = Morphism (time evolution under forces)
//! - **Collision** = 2-morphism (impulse-based path correction)
//! - **Symmetry**: Translation invariance → Total momentum conservation

use serde::{Deserialize, Serialize};

/// A single particle in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Particle {
  /// Unique identifier
  pub id: u64,
  /// Position (m)
  pub position: crate::physics::Vec3,
  /// Velocity (m/s)
  pub velocity: crate::physics::Vec3,
  /// Mass (kg)
  pub mass: f64,
  /// Radius for collision detection (m)
  pub radius: f64,
  /// Charge (for electrostatic interactions, optional)
  pub charge: f64,
}

impl Default for Particle {
  fn default() -> Self {
    Self {
      id: 0,
      position: crate::physics::Vec3::zeros(),
      velocity: crate::physics::Vec3::zeros(),
      mass: 1.0,
      radius: 0.1,
      charge: 0.0,
    }
  }
}

impl Particle {
  /// Create a new particle at the origin
  pub fn new(id: u64, mass: f64, radius: f64) -> Self {
    Self {
      id,
      mass,
      radius,
      ..Default::default()
    }
  }

  /// Create a particle with position and velocity
  pub fn with_state(
    mut self,
    position: crate::physics::Vec3,
    velocity: crate::physics::Vec3,
  ) -> Self {
    self.position = position;
    self.velocity = velocity;
    self
  }

  /// Set the charge for electrostatic interactions
  pub fn with_charge(mut self, charge: f64) -> Self {
    self.charge = charge;
    self
  }

  // **주의**: 다음 함수들은 값 계산이므로 executor에서 구현됩니다:
  // - `kinetic_energy() -> f64`: 운동 에너지 계산
  // - `momentum() -> Vec3`: 운동량 계산
}

/// Particle system state = Object in the Physics Category
///
/// Represents the complete state of an N-particle system at an instant.
///
/// # Executor 구현
///
/// 다음 메서드들은 executor에서 구현됩니다:
/// - `energy() -> f64`: 총 에너지 계산
/// - `momentum() -> Vec3`: 총 운동량 계산
/// - `add_particle()`, `remove_particle()`: 상태 변경
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticleSystemState {
  /// All particles in the system
  pub particles: Vec<Particle>,
  /// Bounding box min (for spatial partitioning)
  pub bounds_min: crate::physics::Vec3,
  /// Bounding box max
  pub bounds_max: crate::physics::Vec3,
}

impl ParticleSystemState {
  /// Create an empty particle system
  pub fn new() -> Self {
    Self {
      particles: Vec::new(),
      bounds_min: crate::physics::Vec3::new(-100.0, -100.0, -100.0),
      bounds_max: crate::physics::Vec3::new(100.0, 100.0, 100.0),
    }
  }

  // **주의**: 다음 함수들은 값 계산/상태 변경이므로 executor에서 구현됩니다:
  // - `add_particle()`, `remove_particle()`: 상태 변경
  // - `energy() -> f64`: 총 에너지 계산
  // - `momentum() -> Vec3`: 총 운동량 계산
}

impl Default for ParticleSystemState {
  fn default() -> Self {
    Self::new()
  }
}
