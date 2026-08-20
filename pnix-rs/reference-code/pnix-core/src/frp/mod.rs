//! FRP (Functional Reactive Programming) - Signal Graph Structure
//!
//! pnix-old의 meaning_core/src/frp_runtime.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! - **구조 정의만** 포함, 실행 로직 없음
//! - SignalKind, SignalNode 등은 **타입 정의**
//! - tick(), get_value(), values HashMap 등은 **executor로 이관**
//!
//! ## pnix-core vs executor 분리
//!
//! | pnix-core (이 모듈) | executor (frp_runtime) |
//! |---------------------|------------------------|
//! | SignalKind enum | FrpRuntime struct |
//! | SignalNode struct | tick(dt) 함수 |
//! | FrpGraph (구조) | values: HashMap |
//! | topo_order() -> Result | get_value() |
//!
//! ## 참조
//!
//! 실행 로직은 `crates/pnix-runtime-legacy/src/frp/mod.rs`에 있습니다.

mod graph;
pub mod morphism;
mod signal;

pub use graph::*;
pub use morphism::*;
pub use signal::*;

// CachedSignalInfo는 signal 모듈에서 re-export
pub use signal::CachedSignalInfo;
