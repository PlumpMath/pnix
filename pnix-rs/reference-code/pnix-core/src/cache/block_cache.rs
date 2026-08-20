//! Block Cache 구조 정의
//!
//! pnix-old의 pnix_block_cache/src/lib.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 캐시 실행 로직(시간 측정, 파일 해시 계산) 제외
//!
//! ## 참고
//!
//! 실제 캐시 실행 로직은 executor에서 구현합니다.
//! 이 모듈은 구조 정의만 포함합니다.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 파일 메타데이터 해시
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileMetadata {
  /// 파일 크기
  pub size: u64,
  /// 파일 해시 (실제 계산은 executor에서)
  pub hash: u64,
}

/// 블록 파싱 결과 캐시
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockCache {
  /// 캐시 엔트리 맵 (파일 경로 → 엔트리)
  pub cache: HashMap<String, CacheEntry>,
  /// 최대 캐시 크기
  pub max_size: usize,
  /// 캐시 히트 횟수 (실제 업데이트는 executor에서)
  pub hits: u64,
  /// 캐시 미스 횟수 (실제 업데이트는 executor에서)
  pub misses: u64,
  /// 퇴거된 엔트리 수 (실제 업데이트는 executor에서)
  pub evictions: u64,
}

/// 캐시 엔트리
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
  /// 파싱된 블록들 (실제 파싱은 executor에서)
  /// JSON으로 직렬화 가능한 구조 (executor에서 LanguageBlock으로 변환)
  pub blocks: Vec<serde_json::Value>,
  /// 파일 메타데이터
  pub metadata: FileMetadata,
  /// 생성 시간 (타임스탬프, 실제 계산은 executor에서)
  pub created_at: u64,
  /// 마지막 접근 시간 (LRU용, 실제 계산은 executor에서)
  pub last_accessed: u64,
  /// 접근 횟수 (실제 업데이트는 executor에서)
  pub access_count: u64,
}

impl BlockCache {
  /// 새 캐시 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new() -> Self {
    Self::with_max_size(100)
  }

  /// 최대 크기 지정하여 캐시 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn with_max_size(max_size: usize) -> Self {
    Self {
      cache: HashMap::new(),
      max_size,
      hits: 0,
      misses: 0,
      evictions: 0,
    }
  }
}

impl Default for BlockCache {
  fn default() -> Self {
    Self::new()
  }
}

/// 캐시 통계
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CacheStats {
  /// 현재 캐시 크기
  pub size: usize,
  /// 최대 캐시 크기
  pub max_size: usize,
  /// 캐시 히트 횟수
  pub hits: u64,
  /// 캐시 미스 횟수
  pub misses: u64,
  /// 퇴거 횟수
  pub evictions: u64,
  /// 캐시 히트율 (0.0 ~ 1.0)
  pub hit_rate: f64,
}

impl CacheStats {
  /// 통계 요약 문자열 반환
  ///
  /// ## 헌법 준수 (P0-1, C1)
  ///
  /// 텍스트 생성만, 파일 I/O 없음
  pub fn summary(&self) -> String {
    format!(
      "Cache: {}/{} entries, hit rate: {:.1}%, hits: {}, misses: {}, evictions: {}",
      self.size,
      self.max_size,
      self.hit_rate * 100.0,
      self.hits,
      self.misses,
      self.evictions
    )
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_block_cache_creation() {
    let cache = BlockCache::new();
    assert_eq!(cache.max_size, 100);
    assert_eq!(cache.cache.len(), 0);
  }

  #[test]
  fn test_block_cache_with_max_size() {
    let cache = BlockCache::with_max_size(50);
    assert_eq!(cache.max_size, 50);
  }

  #[test]
  fn test_cache_stats_summary() {
    let stats = CacheStats {
      size: 10,
      max_size: 100,
      hits: 50,
      misses: 50,
      evictions: 5,
      hit_rate: 0.5,
    };
    let summary = stats.summary();
    assert!(summary.contains("10/100"));
    assert!(summary.contains("50.0%"));
  }
}
