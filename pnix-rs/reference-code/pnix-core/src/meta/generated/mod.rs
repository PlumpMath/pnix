//! Generated Meta Module (Stage 2: Partial Self-Hosting)
//!
//! 자동 생성된 meta 계층 구현.
//! `--features self_hosted_meta`로 활성화.
//!
//! 규칙:
//! - 수동 구현과 구조적으로 동일해야 함
//! - 의미 판단 코드 포함 ❌
//! - core 타입 직접 의존 ❌

mod dsl_codegen_gen;
mod meta_ir_gen;
mod rust_codegen_gen;

pub use dsl_codegen_gen::*;
pub use meta_ir_gen::*;
pub use rust_codegen_gen::*;
