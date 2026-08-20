//! Block Cache 구조 정의
//!
//! pnix-old의 pnix_block_cache/src/lib.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 캐시 실행 로직 제외

pub mod block_cache;

pub use block_cache::{BlockCache, CacheEntry, CacheStats};
