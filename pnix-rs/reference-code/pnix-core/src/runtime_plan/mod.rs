//! 런타임 계획: UnifiedExpr를 런타임 실행 계획으로 변환
//!
//! 런타임에서 직접 실행 가능한 형태로 변환하는 중간 표현

pub mod error;
pub mod ir;
pub mod lower;

pub use error::{RuntimePlanError, RuntimePlanResult};
pub use ir::{RpBinaryOp, RpNode, RpUnaryOp, RpValue, RuntimePlan};
pub use lower::unified_to_runtime_plan;
