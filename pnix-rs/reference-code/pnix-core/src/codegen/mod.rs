//! Codegen: Text generation only (no toolchain calls)
//!
//! 헌법 C1 준수: 텍스트 생성만, 컴파일러/링커 호출 금지
//!
//! ## Supported Languages
//!
//! - JavaScript (JS) - web-native
//! - TypeScript (TS) - typed JavaScript
//! - Python (Py) - scientific computing
//! - Clojure (Clj) - Lisp on JVM
//! - Nix - functional package language
//!
//! ## Draw IR
//!
//! - `draw_ir`: 그래픽 렌더링 명령어 정의 (DrawCmd, DrawExpr)

pub mod draw_ir;
pub mod emit;

#[cfg(test)]
mod test_spec_artifacts;
// emit_fs 제거됨 - fs I/O는 헌법 P0-1 위반, executor.md 참조
pub mod gen_graph;
pub mod gh;
pub mod llvm;
pub mod normalize;
pub mod target;

pub use draw_ir::{DrawCmd, DrawExpr, DrawModule};
pub use emit::Artifacts;
pub use gen_graph::generate;
pub use llvm::LlvmCompiledModule;
pub use target::{CodeGenError, CodeGenResult, ConversionSafety, GeneratedCode, TargetLanguage};
