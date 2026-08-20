//! Unified Evaluation 구조 정의
//!
//! pnix-old의 pnix_unified_eval/src/lib.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 평가 실행 로직(HashMap 값 계산, 런타임 상태) 제외
//!
//! ## 참고
//!
//! 실제 평가 실행 로직은 executor에서 구현합니다.
//! 이 모듈은 구조 정의만 포함합니다.

pub mod policy;
pub mod result;

pub use policy::{AllowedIO, EvalPolicy};
pub use result::{Language, UnifiedResult};
