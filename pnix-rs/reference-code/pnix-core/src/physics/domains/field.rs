//! Force Fields as CT Functors
//!
//! pnix-old의 physics_ct/src/domains/field.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 값 계산 함수 제외
//! - `force_at()`, `potential_at()` 등 값 계산 함수 제외
//!
//! ## 설계 원칙
//!
//! A force field is a map from position to force:
//! - Gravity: F = -G * M * m / r² * r̂
//! - Electric: F = k * q1 * q2 / r² * r̂
//! - Magnetic: F = q * v × B
//!
//! ## CT Mapping
//!
//! - **ForceField** = Functor from Position category to Force category
//! - **FieldComposition** = Natural transformation (superposition)
//! - **Potential** = Scalar field (energy landscape)
//! - **Conservation** = Curl-free fields have potential → energy conservation

use serde::{Deserialize, Serialize};

/// Physical constants for field calculations
pub mod constants {
  /// Gravitational constant (m³ kg⁻¹ s⁻²)
  pub const G: f64 = 6.67430e-11;
  /// Coulomb constant (N m² C⁻²)
  pub const K_E: f64 = 8.987_551_787e9;
  /// Vacuum permittivity (F/m)
  pub const EPSILON_0: f64 = 8.8541878128e-12;
  /// Vacuum permeability (H/m)
  pub const MU_0: f64 = 1.25663706212e-6;
  /// Speed of light (m/s)
  pub const C: f64 = 299792458.0;
}

/// Field constants (re-export for convenience)
pub use constants as FieldConstants;

/// A force field = Functor from Position → Force
///
/// Maps each point in space to a force vector.
///
/// # Executor 구현
///
/// 다음 메서드들은 executor에서 구현됩니다:
/// - `force_at(&self, position: Vec3) -> Vec3`: 위치에서의 힘 계산
/// - `potential_at(&self, position: Vec3) -> f64`: 위치에서의 포텐셜 계산
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ForceField {
  /// Uniform field (constant force everywhere)
  Uniform {
    /// Force vector (N)
    force: crate::physics::Vec3,
  },

  /// Gravitational field from a point mass
  PointGravity {
    /// Source position
    source: crate::physics::Vec3,
    /// Source mass (kg)
    mass: f64,
    /// Gravitational constant (use `constants::G` for physical)
    g: f64,
  },

  /// Electric field from a point charge
  PointCharge {
    /// Source position
    source: crate::physics::Vec3,
    /// Charge (C)
    charge: f64,
    /// Coulomb constant (use `constants::K_E` for physical)
    k: f64,
  },

  /// Magnetic field (uniform)
  UniformMagnetic {
    /// Magnetic field vector (T)
    b_field: crate::physics::Vec3,
  },

  /// Magnetic dipole field
  MagneticDipole {
    /// Dipole position
    source: crate::physics::Vec3,
    /// Magnetic moment (A⋅m²)
    moment: crate::physics::Vec3,
  },

  /// Spring-like restoring field (toward a point)
  Spring {
    /// Anchor position
    anchor: crate::physics::Vec3,
    /// Spring constant (N/m)
    k: f64,
    /// Rest length (m)
    rest_length: f64,
  },

  /// Damping field (opposes velocity)
  Damping {
    /// Damping coefficient (kg/s)
    coefficient: f64,
  },
}
