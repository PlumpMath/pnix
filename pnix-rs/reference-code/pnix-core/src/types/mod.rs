//! Types: Category Theory 기반 타입 시스템
//!
//! pnix-old의 schema_arrow.rs를 pnix-new 패러다임에 맞게 적응.
//!
//! ## 핵심 차이
//!
//! - pnix-old: 표현식 타입 (Int, Float, Product)
//! - pnix-new: 그래프 노드/엣지 타입 (morphism 시그니처)
//!
//! ## 모듈 구조
//!
//! - `core_type`: 기본 타입 정의
//! - `schema_arrow`: 타입 변환 morphism
//! - `type_checker`: FxCoreModule 타입 검증
//! - `meta_schema`: Higher-kinded 타입과 메타 스키마
//! - `tensor`: 텐서/인덱스 표기법 타입

mod core_type;
mod meta_schema;
mod schema_arrow;
pub mod tensor;
mod type_checker;
mod type_inference;

pub use core_type::*;
pub use meta_schema::*;
pub use schema_arrow::*;
pub use tensor::{Index, Symmetry, TensorSymbol, Variance};
pub use type_checker::*;
pub use type_inference::*;
