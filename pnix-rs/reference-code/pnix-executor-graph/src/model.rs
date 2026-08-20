//! FxCore model types (shared)
//!
//! The executor must not carry a “shadow IR model”.
//! Import the canonical IR types from `pnix-fxcore-types`.

pub use pnix_fxcore_types::*;

/// FxCore versions accepted by this executor during migration window.
pub const SUPPORTED_FXCORE_VERSIONS: &[&str] = pnix_fxcore_types::FXCORE_COMPAT_VERSIONS;
