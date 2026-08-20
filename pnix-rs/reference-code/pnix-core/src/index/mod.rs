//! Symbol Index 구조 정의
//!
//! pnix-old의 pnix_symbol_index/src/lib.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 인덱싱 실행 로직 제외

pub mod symbol_index;

pub use symbol_index::{GlobalSymbolIndex, SymbolDefinition, SymbolIndex, SymbolReference};
