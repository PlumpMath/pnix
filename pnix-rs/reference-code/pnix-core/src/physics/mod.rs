//! Physics CT 구조 정의
//!
//! pnix-old의 physics_ct에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 값 계산 함수 제외
//! - Trait 메서드 시그니처는 문서화만 (구현은 executor에서)
//! - 구조체/Enum 정의만 포함
//! - 값 계산 함수 (energy(), momentum(), conserves_energy() 등) 제외
//!
//! ## 설계 원칙
//!
//! - **CT-first**: 모든 물리 연산은 morphism으로 모델링
//! - **Cobordism as Process**: 물리 프로세스는 상태를 연결하는 표면
//! - **Conservation as Functor**: 에너지/운동량 보존 = functor 보존
//! - **Symmetry as Natural Transformation**: Noether 정리

pub mod cobordism;
pub mod conservation;
pub mod traits;

pub mod domains;

pub use cobordism::{Cobordism, Cobordism2, EquivalenceProof, EquivalenceType};
pub use conservation::{
  AngularMomentumConservation, ConservationBundle, ConservationLaw, ConservationResults,
  EnergyConservation, MomentumConservation,
};
pub use domains::{
  ConstrainedBody, ConstraintType, DenavitHartenbergParams, FactorType, FieldConstants, ForceField,
  JointConfig, JointType, Landmark2D, Particle, ParticleSystemState, Pose2D, RigidBodyState,
  SlamState, Twist, Wrench,
};
pub use traits::{PhysicalObject, PhysicalProcess, Symmetry, SymmetryTransform, Vec3};

/// 부동소수점 비교를 위한 허용 오차
pub const EPSILON: f64 = 1e-10;
