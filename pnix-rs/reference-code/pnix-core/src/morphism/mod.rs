//! Morphism 시스템 구조 정의
//!
//! pnix-old의 ct_morphism/src/lib.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 값 계산 함수 제외
//! - `Morphism::apply()` - 값 계산 함수 제외 (executor에서 구현)
//! - `MorphismRegistry::apply()` - 값 계산 함수 제외 (executor에서 구현)
//! - `ComposedMorphism::apply()` - 값 계산 함수 제외 (executor에서 구현)
//!
//! ## 설계 원칙
//!
//! - **Trait 정의**: Morphism trait 시그니처만 포함 (apply() 제외)
//! - **Registry 구조**: MorphismRegistry 구조 정의만 (실제 등록/조회는 executor)
//! - **Composition**: 구조적 합성 정보만 (실제 합성 실행은 executor)

pub mod error;
pub mod registry;
pub mod traits;

pub use error::MorphismError;
pub use registry::{ComposedMorphism, MorphismInfo, MorphismRegistry};
pub use traits::Morphism;
