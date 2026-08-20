//! Code transform host carriers.
//!
//! OWNER-LAW (2026-05-11): pnix is an LLM-independent deterministic AI
//! substrate. Code transforms emit deterministic patches by composing
//! typed rewrites over identifier tokens / CSTs / symbol graphs. The
//! `.px` owner laws in `stdlib/lib/gate/code-transform/*.px` define
//! *what counts as a valid transform*; this module hosts the
//! deterministic Rust carriers that *perform* the rewrites under an
//! `EvidenceLane::InternalOwnerLaw` promotion lane.
//!
//! Sub-modules:
//!
//! - `rename_symbol` — whole-word identifier rename across a bounded
//!   set of target paths. Mirror of
//!   `stdlib/lib/gate/code-transform/rename-symbol.px`. Token-based
//!   (no tree-sitter dependency yet); later slices may upgrade to CST
//!   matching for string-literal / comment awareness. Closes the full
//!   5-stage canonical chain (candidate → review → apply →
//!   rollback-handle → rollback-receipt) end-to-end.
//!
//! - `remove_unused_import` — request-shape classifier for the
//!   second deterministic code-transform. Mirror of
//!   `stdlib/lib/gate/code-transform/remove-unused-import.px`. The
//!   transform assumes the host has already pre-resolved candidate
//!   unused imports via tree-sitter / symbol resolver (with
//!   `used_in_macro` / `behind_cfg` flags); this carrier only owns
//!   the verdict ladder. Proves the canonical chain pattern
//!   generalizes to transform kinds other than rename-symbol.

pub mod add_edge;
pub mod add_extern_decl;
pub mod add_import;
pub mod add_node;
pub mod add_test_stub;
pub mod change_signature;
pub mod extract_function;
pub mod inline_function;
pub mod move_symbol;
pub mod pnix_graph;
pub mod pnix_graph_inventory;
pub mod remove_unused_import;
pub mod rename_symbol;
