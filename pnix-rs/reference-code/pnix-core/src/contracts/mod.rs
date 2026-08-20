//! Contracts: Effect, Purity, Determinism verification

pub mod effect;
pub mod verify;

pub use verify::{
  verify_input_size, verify_resource_limits, ClosureReport, ResourceLimits, VerificationReport,
};
