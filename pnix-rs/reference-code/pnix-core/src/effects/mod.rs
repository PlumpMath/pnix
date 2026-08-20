//! Effects: Effect Zone 시스템
//!
//! pnix-old의 zone_inference.rs를 pnix-new 패러다임에 맞게 적응.
//!
//! ## Effect Lattice
//!
//! ```text
//! Pure < Symbolic < Frp < Animation < STM < Interop < World
//! ```
//!
//! ## pnix-new 적용
//!
//! - Node의 morphism effect 추적
//! - Edge traversal의 effect 전파
//! - Scope 격리 정책과 연동

mod capability;
mod zone;
mod zone_checker;
mod zone_inference;

pub use capability::{Capability, CapabilitySet, ZoneCapabilities};
pub use zone::*;
pub use zone_checker::*;
pub use zone_inference::{
  check_op_in_zone, infer_op_zone, infer_sequence_zone, ZoneContext, ZoneResult, ZoneViolation,
};
