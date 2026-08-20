//! JavaScript/TypeScript Language Frontend - AST 타입 및 파서 규칙
//!
//! pnix-old의 lang_js에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의 및 파서 규칙만, 실행 로직 제외
//! - AST 타입: `UnifiedExpr` (lang_pnix에서 재사용)
//! - 에러 타입: `JsError`
//! - Lowering 함수: `lower_to_fx_core` (구조 변환만)
//!
//! 실행 로직 (`compile_js_to_fx`, `compile_ts_to_fx`)은 executor로 이동

pub mod error;
pub mod lower;

pub use error::JsError;
pub use lower::lower_to_fx_core;
// UnifiedExpr는 lang_pnix에서 재사용
pub use crate::lang::pnix::UnifiedExpr;
