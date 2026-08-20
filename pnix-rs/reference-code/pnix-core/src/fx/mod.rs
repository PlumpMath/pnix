//! FX Legacy AST Types
//!
//! pnix-old의 FxCoreExpr/FxSurfaceExpr를 pnix-new에 마이그레이션
//!
//! ## 패러다임 차이
//!
//! | 측면 | pnix-old | pnix-new |
//! |-----|----------|----------|
//! | 중심 IR | FxCoreExpr (트리) | FxCoreModule (DAG) |
//! | 표면 AST | FxSurfaceExpr | DSL 파서 직접 생성 |
//! | 연산 ID | MeaningOpId | Morphism 이름 |
//!
//! ## 사용 목적
//!
//! - 표면 구문 파싱 (pnix/clojure)
//! - 레거시 포맷 호환
//! - 노드 morphism 표현식
//!
//! ## 헌법 준수 (P0-1)
//!
//! 순수 AST 구조 정의, 값 계산 없음

pub mod core_expr;
pub mod dependency;
pub mod lowering;
pub mod meaning_op;
pub mod op_table;
pub mod opt;
pub mod surface;

pub use core_expr::{FxBinding, FxCoreExpr, FxDrawCmd, FxProgram, SignalId};
pub use dependency::{collect_deps, order_binding_names, order_bindings, FxOrderError};
pub use lowering::{lower_surface_to_core, FxLoweringError};
pub use meaning_op::{
  load_meaning_ops_from_seto, meaning_op_meta, MeaningMeta, MeaningOpId, MeaningOpSpec, UnifiedMeta,
};
pub use op_table::UnifiedMeaningOp;
pub use opt::{FxAlgebraicSimplify, FxConstFold, FxFrpPatternLift, FxOptPass, FxOptPipeline};
pub use surface::FxSurfaceExpr;
