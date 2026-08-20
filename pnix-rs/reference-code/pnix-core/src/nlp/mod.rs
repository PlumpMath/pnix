//! NLP Schema Mapping
//!
//! pnix-old의 nlp_schema_mapper.rs를 pnix-new 패러다임에 적응
//!
//! ## 기능
//!
//! - 명사 → 타입(CoreType) 매핑
//! - 동사 → 화살표(ArrowType) 매핑
//! - 간단한 규칙 기반 추출기 (한국어/영어)
//! - 수학 문제 유형 분류 (ProblemType)
//!
//! # OWNER-LAW NOTE (2026-05-10)
//!
//! **pnix 는 LLM 없이 작동하는 deterministic AI substrate** (`CLAUDE.md`
//! OWNER-LAW CONSTITUTION). 이 module 은 deterministic NL → Schema/Morphism
//! mapping 만 담당하고 LLM 호출 없음. 아래 `LlmError` / `LlmResult` re-export 는
//! legacy compatibility shim 으로 *deprecated* 다 — 새 코드는
//! `crate::llm` 경로를 직접 cite 하지 않는다. 향후 refactor 에서 namespace
//! 를 LLM-free 이름으로 rename 한다.
//!
//! 헌법 P0-1 준수: 구조 매핑만, 값 계산 없음

pub mod config;
// Fix: Remove duplicate error module, use llm::error instead
// pub mod error;  // Removed - using llm::error instead
pub mod formula_dimension_inference;
pub mod locked_domain;
pub mod problem_type;
pub mod query_builder;
pub mod relation_classifier;
pub mod relation_composition;
pub mod schema_mapper;
pub mod truth_domain;
pub mod verification;

pub use config::{GenerationConfig, GenerationConfigBuilder};
// OWNER-LAW (2026-05-11): public re-export of `LlmError`/`LlmResult` is
// feature-gated behind `legacy-llm-names` (default OFF). pnix substrate has
// no LLM seat; default builds expose no `LlmError`/`LlmResult` symbol.
// Downstream callers that still need them must opt in via the feature and
// migrate to LLM-free error types. Internal callers use `crate::llm` (a
// private module alias) directly.
#[cfg(feature = "legacy-llm-names")]
#[deprecated(
  note = "OWNER-LAW: pnix has no LLM seat. Migrate to a LLM-free error type and disable the `legacy-llm-names` feature."
)]
pub use crate::llm::error::{LlmError, LlmResult};
pub use locked_domain::LockedDomain;
pub use problem_type::ProblemType;
pub use query_builder::{
  QueryBuildError, SetoFilter, SetoQuery, SetoQueryBuilder, SetoQueryType, SortBy,
};
pub use schema_mapper::*;
pub use truth_domain::TruthDomain;
pub use verification::{
  ProofStep, SetoNodeId, TruthVerification, VerificationRequest, VerificationResponse,
};
