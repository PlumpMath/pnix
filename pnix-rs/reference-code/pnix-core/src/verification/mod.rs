//! Verification: law types and validation scaffolding.

pub mod contract;
pub mod invariant;
pub mod law;

pub use contract::Contract;
pub use invariant::Invariant;
pub use law::{Law, LawKind};
