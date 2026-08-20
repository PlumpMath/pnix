//! Physics Domains - 물리 도메인 타입 정의
//!
//! pnix-old의 physics_ct/src/domains에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 값 계산 함수 제외
//! - `energy()`, `momentum()`, `kinetic_energy()` 등 값 계산 함수 제외
//! - `is_valid()`, `distance_to()` 등 값 계산 함수 제외
//! - 구조체/Enum 정의만 포함
//!
//! ## 도메인
//!
//! - **Kinematics**: 로봇 팔 IK/FK (Denavit-Hartenberg)
//! - **Rigid Body**: SE(3) dynamics for solid objects
//! - **Particle**: N-body systems with collision detection
//! - **SLAM**: Factor graph for localization and mapping
//! - **Field**: Force fields (gravity, EM, Lorentz force)
//! - **Constraint**: Joints and contacts (sequential impulses solver)

pub mod constraint;
pub mod field;
pub mod kinematics;
pub mod particle;
pub mod rigid_body;
pub mod slam;

pub use constraint::{ConstrainedBody, ConstraintType, JointType};
pub use field::{FieldConstants, ForceField};
pub use kinematics::{DenavitHartenbergParams, JointConfig};
pub use particle::{Particle, ParticleSystemState};
pub use rigid_body::{RigidBodyState, Twist, Wrench};
pub use slam::{FactorType, Landmark2D, Pose2D, SlamState};
