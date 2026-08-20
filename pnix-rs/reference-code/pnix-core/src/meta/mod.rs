//! Meta: 자기 기술 IR (meta-circular 준비)
//!
//! pnix이 자기 자신을 기술하는 순수 구조 변환 레이어.
//! 실행 없음, 검증 없음, 의미 재해석 없음.
//!
//! ## 모듈 구조
//!
//! - `meta_ir`: MetaFx IR 정의 및 FxCore → MetaFx 변환
//! - `dsl_codegen`: MetaFx → pnix DSL 텍스트 생성
//! - `rust_codegen`: MetaFx → Rust struct 코드 생성 (읽기 전용)
//! - `mermaid_codegen`: MetaFx → Mermaid 다이어그램 생성 (시각화)
//! - `ct_diagram`: Category Theory 다이어그램 (DOT/Mermaid)
//! - `self_hosting_codegen`: Stage 2 자기 생성 코드
//! - `generated`: 자동 생성된 meta 구현 (self_hosted_meta feature)

mod ct_diagram;
mod dsl_codegen;
mod mermaid_codegen;
mod meta_ir;
mod rust_codegen;
mod self_hosting_codegen;

#[cfg(feature = "self_hosted_meta")]
pub mod generated;

pub use ct_diagram::*;
pub use dsl_codegen::*;
pub use mermaid_codegen::*;
pub use meta_ir::*;
pub use rust_codegen::*;
pub use self_hosting_codegen::*;
