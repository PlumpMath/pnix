//! Passes: Lowering, Optimization, Pipeline
//!
//! ## 모듈 구조
//!
//! - `traits`: 공통 최적화 패스 인터페이스
//! - `lowering`: IR 변환 (AST → Surface → FxCore → SSA)
//! - `optimize`: FxCore 최적화
//! - `ssa_opt`: SSA 최적화
//! - `dep_analysis`: 의존성 분석
//! - `cache_policy`: 캐시 정책
//! - `provenance`: 변환 추적 (디버깅/검증)
//! - `pipeline`: 파이프라인 정의
//! - `telemetry`: 컴파일 타임 텔레메트리

pub mod cache_policy;
pub mod dep_analysis;
pub mod frp_cache;
pub mod lowering;
pub mod lowering_ir;
pub mod optimize;
pub mod pipeline;
pub mod provenance;
pub mod ssa_opt;
pub mod telemetry;
pub mod telemetry_frame;

pub mod frp_morphism;

pub use frp_cache::{
  is_time_dependent, plan_frp_cache, stable_hash_ir, CacheCandidate, FrpCachePlan,
};
pub use frp_morphism::{
  MorphismEdge, MorphismGraph, MorphismNode, MorphismNodeId, MorphismNodeKind, PortType,
};
pub use telemetry::{
  ConsoleTelemetry, FrameRecord, TelemetryBus, TelemetryCollector, TelemetrySnapshot,
};
pub use telemetry_frame::FrpTelemetryFrame;
pub mod traits;
