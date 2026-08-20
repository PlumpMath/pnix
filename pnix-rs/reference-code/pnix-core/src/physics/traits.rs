//! Physical Traits - CT Objects and Morphisms for Physics
//!
//! pnix-old의 physics_ct/src/traits.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! **중요**: Trait 메서드 시그니처는 문서화 목적으로만 포함됩니다.
//! 실제 구현(값 계산)은 executor에서 수행됩니다.
//!
//! ## Noether's Theorem in CT Language
//!
//! Noether's theorem states that every continuous symmetry corresponds to a conservation law.
//! In CT terms:
//! - **Symmetry** = Natural Transformation (structure-preserving map)
//! - **Conservation** = Functor Preservation (quantity invariant under the process)
//!
//! ## Energy Conservation as Functor
//!
//! If we have a functor `E: Phys → R` (energy functional), then energy conservation
//! for a process `P: A → B` means `E(A) = E(B)`, i.e., `E ∘ P = E`.

use serde::{Deserialize, Serialize};

/// A physical object = an Object in the Physics Category
///
/// Represents a physical state that can be measured and transformed.
/// Every physical state has:
/// - Energy (scalar)
/// - Momentum (vector)
/// - Equilibrium check (is it stable?)
///
/// # CT Interpretation
///
/// Objects are the "points" in our category. They represent snapshots
/// of physical reality at a given instant.
///
/// # Executor 구현
///
/// 다음 메서드들은 executor에서 구현됩니다:
/// - `energy() -> f64`: 총 에너지 계산
/// - `momentum() -> Vec3`: 선형 운동량 계산
/// - `is_equilibrium() -> bool`: 평형 상태 확인
/// - `angular_momentum() -> Vec3`: 각운동량 계산 (기본값: zero)
/// - `mass() -> f64`: 질량 (기본값: 1.0)
pub trait PhysicalObject: Clone + Send + Sync {
  /// Total energy of this physical state
  ///
  /// E = kinetic + potential
  /// **구현 위치**: executor
  fn energy(&self) -> f64;

  /// Linear momentum of this physical state
  ///
  /// p = m * v
  /// **구현 위치**: executor
  fn momentum(&self) -> crate::physics::Vec3;

  /// Check if the system is in equilibrium
  ///
  /// True if all forces/torques are balanced
  /// **구현 위치**: executor
  fn is_equilibrium(&self) -> bool;

  /// Angular momentum (optional, defaults to zero)
  /// **구현 위치**: executor
  fn angular_momentum(&self) -> crate::physics::Vec3 {
    crate::physics::Vec3::zeros()
  }

  /// Mass of the object (optional, useful for normalization)
  /// **구현 위치**: executor
  fn mass(&self) -> f64 {
    1.0
  }
}

/// A physical process = a Morphism in the Physics Category
///
/// Represents a physical transformation from state A to state B.
/// Corresponds to the "Cobordism" concept in string theory.
///
/// # CT Interpretation
///
/// Morphisms are the "arrows" in our category. They represent
/// physical processes that transform one state into another.
///
/// ## Composition
///
/// If we have `P1: A → B` and `P2: B → C`, we can compose them:
/// `P2 ∘ P1: A → C`
///
/// ## Action (Path Integral)
///
/// Every process has an "action" (∫ L dt) which measures the
/// "cost" of the path in configuration space.
///
/// # Executor 구현
///
/// 다음 메서드들은 executor에서 구현됩니다:
/// - `apply(&self, state: &A) -> B`: 프로세스 적용 (상태 변환)
/// - `action() -> f64`: 액션 계산 (Lagrangian 적분)
/// - `preserves_energy(&self, input: &A) -> bool`: 에너지 보존 확인
/// - `preserves_momentum(&self, input: &A) -> bool`: 운동량 보존 확인
/// - `duration() -> f64`: 프로세스 지속 시간
/// - `is_reversible() -> bool`: 가역성 확인 (기본값: true)
pub trait PhysicalProcess<A: PhysicalObject, B: PhysicalObject>: Clone + Send + Sync {
  /// Apply the process to transform state A into state B
  /// **구현 위치**: executor
  fn apply(&self, state: &A) -> B;

  /// Calculate the action (Lagrangian integral) of this process
  ///
  /// S = ∫ L dt where L = T - V (kinetic - potential)
  /// **구현 위치**: executor
  fn action(&self) -> f64;

  /// Check if this process preserves energy (within tolerance)
  ///
  /// This is the CT formulation of energy conservation:
  /// the energy functor is preserved by this morphism.
  /// **구현 위치**: executor
  fn preserves_energy(&self, input: &A) -> bool;

  /// Check if this process preserves momentum (within tolerance)
  ///
  /// In a closed system, linear momentum is conserved.
  /// **구현 위치**: executor
  fn preserves_momentum(&self, input: &A) -> bool;

  /// Duration of this process (time morphism parameter)
  /// **구현 위치**: executor
  fn duration(&self) -> f64;

  /// Is this process reversible?
  ///
  /// Thermodynamically reversible processes don't increase entropy.
  /// **구현 위치**: executor
  fn is_reversible(&self) -> bool {
    true
  }
}

/// Symmetry type for Noether's theorem
///
/// Each symmetry corresponds to a conservation law.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Symmetry {
  /// Time translation invariance → Energy conservation
  TimeTranslation,
  /// Space translation invariance → Momentum conservation
  SpaceTranslation,
  /// Rotation invariance → Angular momentum conservation
  Rotation,
  /// Lorentz boost invariance → Center of mass motion
  LorentzBoost,
  /// Gauge invariance (e.g., EM) → Charge conservation
  Gauge,
  /// Custom symmetry with a name
  Custom(&'static str),
}

impl Symmetry {
  /// Get the conserved quantity name for this symmetry
  pub fn conserved_quantity(&self) -> &'static str {
    match self {
      Symmetry::TimeTranslation => "energy",
      Symmetry::SpaceTranslation => "momentum",
      Symmetry::Rotation => "angular_momentum",
      Symmetry::LorentzBoost => "center_of_mass",
      Symmetry::Gauge => "charge",
      Symmetry::Custom(name) => name,
    }
  }
}

/// A symmetry transformation = a Natural Transformation
///
/// If a process `P: A → B` is invariant under a symmetry `S`,
/// then the corresponding quantity is conserved.
///
/// # Executor 구현
///
/// 다음 메서드들은 executor에서 구현됩니다:
/// - `symmetry() -> Symmetry`: 대칭성 타입 반환
/// - `transform<O: PhysicalObject>(&self, state: &O) -> O`: 대칭 변환 적용
/// - `is_invariant<A, B, P>(&self, process: &P, input: &A) -> bool`: 불변성 확인
pub trait SymmetryTransform: Clone + Send + Sync {
  /// The type of symmetry this represents
  /// **구현 위치**: executor
  fn symmetry(&self) -> Symmetry;

  /// Apply the symmetry transformation to a state
  /// **구현 위치**: executor
  fn transform<O: PhysicalObject + Clone>(&self, state: &O) -> O;

  /// Check if a process is invariant under this symmetry
  /// **구현 위치**: executor
  fn is_invariant<A, B, P>(&self, process: &P, input: &A) -> bool
  where
    A: PhysicalObject,
    B: PhysicalObject,
    P: PhysicalProcess<A, B>;
}

/// 3D 벡터 타입 (구조 정의만)
///
/// **주의**: 실제 Vec3 구현은 executor에서 제공됩니다.
/// pnix-core에서는 구조 정의만 포함합니다.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Vec3 {
  pub x: f64,
  pub y: f64,
  pub z: f64,
}

impl Vec3 {
  /// 새 Vec3 생성
  pub fn new(x: f64, y: f64, z: f64) -> Self {
    Self { x, y, z }
  }

  /// 영벡터 생성
  pub fn zeros() -> Self {
    Self {
      x: 0.0,
      y: 0.0,
      z: 0.0,
    }
  }
}
