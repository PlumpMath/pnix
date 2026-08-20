//! Symbolic Provenance - Transformation Proof Log
//!
//! pnix-old의 SymbolicProvenance를 pnix-new에 마이그레이션
//!
//! ## 설계 원칙
//!
//! - **debug-first**: Release 빌드에서는 비용 0 (feature-gated)
//! - **Effect Zone 연동**: Zone 기반 시간 승격 결정 기록
//! - **egg 통계 통합**: SimplifyStats 포함
//!
//! ## 헌법 준수 (P0-1)
//!
//! 순수 변환 추적 구조, 값 계산 없음

mod budget;
mod differentiability;
mod symbolic;
mod temporal;

pub use budget::{AdaptiveSimplifyResult, BudgetTier};
pub use differentiability::{
  DifferentiabilityAnalysis, DifferentiabilityReason, NonDifferentiableOp,
};
pub use symbolic::{
  ApproxPoint, CachedSubexprInfo, CtValidationResult, FrpCacheStatsRecord, ProvenanceBuilder,
  SimplifyStats, SymbolicProvenance,
};
pub use temporal::TemporalDecision;
