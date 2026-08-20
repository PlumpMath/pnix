//! Lexer: Tokenization for Nix/Clojure mixed code
//!
//! pnix-old의 pnix_tokenizer를 pnix-new에 마이그레이션
//!
//! ## 주요 기능
//!
//! - **통합 토큰화**: Nix와 Clojure 코드를 하나의 파이프라인에서 처리
//! - **Clojure 블록 인식**: `clj { }` 블록을 자동으로 감지하여 Clojure 토큰으로 처리
//!
//! ## 헌법 준수 (P0-1)
//!
//! 순수 토큰 분해, 실행 없음

pub mod block_context;
pub mod block_parser;
mod nix_preprocess;
mod streaming;
mod token;
mod tokenizer;

pub use block_context::{
  BlockContext, BlockContextAnalyzer, DependencyEdge, DependencyGraph, SymbolInfo, SymbolType,
};
pub use block_parser::{BlockParser, BlockParserConfig, BlockSummary, LanguageBlock};
pub use nix_preprocess::preprocess_nix_code;
pub use streaming::StreamingTokenizer;
pub use token::PnixToken;
pub use tokenizer::{TokenizeError, Tokenizer};
