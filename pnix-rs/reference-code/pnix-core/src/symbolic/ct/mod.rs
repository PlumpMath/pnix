//! CT (Category Theory) 검증
//!
//! 심볼릭 표현의 타입 안전성 검증
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 검증만, 값 계산 없음
//!
//! ## 주요 검증 규칙
//!
//! - 텐서 수축: 반대 position, 같은 space
//! - 자유 인덱스: Add에서 동일해야 함
//! - 단위 일치: Add에서 같은 단위

mod check;
mod context;
mod errors;
mod tags;

pub use check::{
  analyze_indices, check_ct, contains_tensor_indices, free_indices_of_expr, IndexUsage,
};
pub use context::CtContext;
pub use errors::CtError;
pub use tags::{CtCategory, CtTag};
