//! Conservation Laws as Functors
//!
//! pnix-old의 physics_ct/src/conservation.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 값 계산 함수 제외
//! - `extract()`, `is_preserved()` 등 값 계산 함수는 executor에서 구현
//!
//! ## 설계 원칙
//!
//! In Category Theory, a functor F: C → D preserves structure:
//! - F(id_A) = id_{F(A)}
//! - F(g ∘ f) = F(g) ∘ F(f)
//!
//! Conservation laws in physics are exactly this!
//! - Energy functor E: Phys → R  (energy is a number)
//! - Momentum functor P: Phys → R³ (momentum is a vector)
//!
//! A process "conserves energy" iff E(A) = E(B) for morphism f: A → B

use super::traits::{PhysicalObject, PhysicalProcess, Symmetry};
use serde::{Deserialize, Serialize};

/// A conservation law = a functor from Physics to some quantity
///
/// # Executor 구현
///
/// 다음 메서드들은 executor에서 구현됩니다:
/// - `name() -> &'static str`: 보존 법칙 이름
/// - `associated_symmetry() -> Symmetry`: 관련 대칭성 (Noether 정리)
/// - `extract<O: PhysicalObject>(&self, state: &O) -> Self::Quantity`: 보존량 추출
/// - `is_preserved<A, B, P>(&self, process: &P, input: &A) -> bool`: 보존 여부 확인
pub trait ConservationLaw {
  /// The type of conserved quantity
  type Quantity;

  /// Name of this conservation law
  /// **구현 위치**: executor
  fn name(&self) -> &'static str;

  /// The symmetry associated with this law (Noether's theorem)
  /// **구현 위치**: executor
  fn associated_symmetry(&self) -> Symmetry;

  /// Extract the conserved quantity from a physical state
  /// **구현 위치**: executor
  fn extract<O: PhysicalObject>(&self, state: &O) -> Self::Quantity;

  /// Check if a process preserves this quantity
  /// **구현 위치**: executor
  fn is_preserved<A, B, P>(&self, process: &P, input: &A) -> bool
  where
    A: PhysicalObject,
    B: PhysicalObject,
    P: PhysicalProcess<A, B>;
}

/// Energy conservation law
///
/// Associated symmetry: Time translation invariance
/// Conserved quantity: Total energy (kinetic + potential)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EnergyConservation {
  /// Tolerance for floating-point comparison
  pub tolerance: f64,
}

impl Default for EnergyConservation {
  fn default() -> Self {
    Self {
      tolerance: super::EPSILON,
    }
  }
}

/// Momentum conservation law
///
/// Associated symmetry: Space translation invariance
/// Conserved quantity: Total linear momentum
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MomentumConservation {
  /// Tolerance for floating-point comparison
  pub tolerance: f64,
}

impl Default for MomentumConservation {
  fn default() -> Self {
    Self {
      tolerance: super::EPSILON,
    }
  }
}

/// Angular momentum conservation law
///
/// Associated symmetry: Rotation invariance
/// Conserved quantity: Total angular momentum
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AngularMomentumConservation {
  /// Tolerance for floating-point comparison
  pub tolerance: f64,
}

impl Default for AngularMomentumConservation {
  fn default() -> Self {
    Self {
      tolerance: super::EPSILON,
    }
  }
}

/// A bundle of multiple conservation laws to check at once
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConservationBundle {
  pub energy: EnergyConservation,
  pub momentum: MomentumConservation,
  pub angular_momentum: AngularMomentumConservation,
}

/// Results of checking multiple conservation laws
///
/// **주의**: 실제 `check_all()` 함수는 executor에서 구현됩니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConservationResults {
  pub energy_conserved: bool,
  pub momentum_conserved: bool,
  pub angular_momentum_conserved: bool,
}

impl ConservationResults {
  /// 모든 보존 법칙이 만족되는지 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 검증만, 값 계산 없음
  pub fn all_conserved(&self) -> bool {
    self.energy_conserved && self.momentum_conserved && self.angular_momentum_conserved
  }

  /// 최소 하나의 보존 법칙이 만족되는지 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 검증만, 값 계산 없음
  pub fn any_conserved(&self) -> bool {
    self.energy_conserved || self.momentum_conserved || self.angular_momentum_conserved
  }

  /// 만족된 보존 법칙 개수 계산
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 계산만, 값 계산 없음
  pub fn count_conserved(&self) -> usize {
    let mut count = 0;
    if self.energy_conserved {
      count += 1;
    }
    if self.momentum_conserved {
      count += 1;
    }
    if self.angular_momentum_conserved {
      count += 1;
    }
    count
  }
}
