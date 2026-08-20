//! Cache Policy - Adaptive tuning for compile-time caching
//!
//! pnix-old의 cache_policy.rs를 pnix-new 패러다임에 적응
//!
//! ## 헌법 준수
//!
//! - P0-1: 구조 매핑만, 값 계산 없음
//! - C1: 컴파일러 코어는 실행하지 않음 (PolicyAwareEvaluator 제외)
//!
//! ## 기능 (v1)
//!
//! - CachePolicy: 캐싱 설정
//! - PolicyTuner: 히트율 기반 자동 튜닝
//! - BoundedMemo: FIFO 퇴거 정책의 제한된 캐시
//!
//! FRP 평가 관련 기능은 pnix-executor에서 구현 예정
//!
//! # Policy Parameters
//!
//! | Parameter | Purpose | Default |
//! |-----------|---------|---------|
//! | `min_size` | Minimum subtree size to cache | 3 |
//! | `max_candidates` | Maximum cache candidates | 16 |
//! | `max_memo_entries` | Maximum memo table size | 256 |
//! | `target_hit_rate` | Desired cache hit rate | 0.7 |
//!
//! # Auto-tuning Rules
//!
//! Based on observed hit rate vs target:
//! - Hit rate too low → increase `min_size` (fewer, larger subtrees)
//! - Hit rate high + misses high → decrease `min_size` (more caching)
//! - Memo overflow → evict oldest entries (FIFO)

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

// ============================================================
// Cache Statistics
// ============================================================

/// 캐시 통계: 튜닝 결정을 위한 캐시 통계 정보
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CacheStats {
  /// 캐시 히트 수
  pub hits: u64,
  /// 캐시 미스 수
  pub misses: u64,
  /// 캐시된 키 개수
  pub cached_keys_count: usize,
}

// 헌법 준수 (P0-1): 값 계산 및 상태 변경 함수 제거
// hit_rate(), reset(), record_hit(), record_miss() 등의 값 계산/상태 변경 함수는
// 실행/상태 변경 로직은 executor/runtime 계층에서 구현하세요.

// ============================================================
// Cache Policy
// ============================================================

/// 캐시 정책 설정: 컴파일 타임 캐싱을 위한 정책 설정
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CachePolicy {
  /// Minimum subtree size (nodes) to consider for caching
  /// Smaller subtrees have more cache overhead than benefit
  pub min_size: u32,

  /// Maximum number of cache candidates per expression
  /// Limits analysis time and memory
  pub max_candidates: usize,

  /// Maximum memo table entries
  /// Prevents unbounded memory growth
  pub max_memo_entries: usize,

  /// Target hit rate for auto-tuning (0.0 - 1.0)
  /// Policy adjusts to approach this rate
  pub target_hit_rate: f64,

  /// Enable auto-tuning based on runtime stats
  pub auto_tune: bool,
}

impl Default for CachePolicy {
  fn default() -> Self {
    Self {
      min_size: 3,
      max_candidates: 16,
      max_memo_entries: 256,
      target_hit_rate: 0.7,
      auto_tune: true,
    }
  }
}

impl CachePolicy {
  /// Create a new policy with default values
  pub fn new() -> Self {
    Self::default()
  }

  /// Conservative policy - cache less, safer
  pub fn conservative() -> Self {
    Self {
      min_size: 5,
      max_candidates: 8,
      max_memo_entries: 128,
      target_hit_rate: 0.8,
      auto_tune: true,
    }
  }

  /// Aggressive policy - cache more, faster (maybe)
  pub fn aggressive() -> Self {
    Self {
      min_size: 2,
      max_candidates: 32,
      max_memo_entries: 512,
      target_hit_rate: 0.6,
      auto_tune: true,
    }
  }

  /// Disabled policy - no caching
  pub fn disabled() -> Self {
    Self {
      min_size: u32::MAX,
      max_candidates: 0,
      max_memo_entries: 0,
      target_hit_rate: 0.0,
      auto_tune: false,
    }
  }

  // ─────────────────────────────────────────────────────────────────
  // Builder pattern
  // ─────────────────────────────────────────────────────────────────

  pub fn with_min_size(mut self, size: u32) -> Self {
    self.min_size = size;
    self
  }

  pub fn with_max_candidates(mut self, count: usize) -> Self {
    self.max_candidates = count;
    self
  }

  pub fn with_max_memo_entries(mut self, count: usize) -> Self {
    self.max_memo_entries = count;
    self
  }

  pub fn with_target_hit_rate(mut self, rate: f64) -> Self {
    self.target_hit_rate = rate.clamp(0.0, 1.0);
    self
  }

  pub fn with_auto_tune(mut self, enabled: bool) -> Self {
    self.auto_tune = enabled;
    self
  }

  /// Check if caching is enabled
  pub fn is_enabled(&self) -> bool {
    self.max_candidates > 0 && self.max_memo_entries > 0
  }
}

// ============================================================
// Auto-tuning
// ============================================================

/// 자동 튜닝 조정 결과: 캐시 정책 자동 튜닝의 조정 결과
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TuningAction {
  /// 변경 불필요
  NoChange,
  /// min_size 증가 (더 적고 큰 서브트리만 캐시)
  IncreaseMinSize,
  /// min_size 감소 (더 많은 서브트리 캐시)
  DecreaseMinSize,
  /// max_candidates 증가
  IncreaseMaxCandidates,
  /// max_candidates 감소
  DecreaseMaxCandidates,
}

/// 자동 튜닝 엔진: 관찰된 히트율 이력을 기반으로 캐시 정책을 조정하는 엔진
///
/// 간단한 임계값 기반 규칙을 사용하여 캐시 정책을 조정합니다.
/// 헌법 P0-1 준수: 구조 정의만, 실행 없음
#[derive(Clone, Debug)]
pub struct PolicyTuner {
  /// History of recent hit rates (for smoothing)
  hit_rate_history: Vec<f64>,
  /// Maximum history length
  #[allow(dead_code)]
  max_history: usize,
  /// Minimum samples before tuning
  #[allow(dead_code)]
  min_samples: usize,
  /// Cooldown frames between adjustments
  #[allow(dead_code)]
  cooldown: usize,
  /// Frames since last adjustment
  #[allow(dead_code)]
  frames_since_adjust: usize,
}

impl Default for PolicyTuner {
  fn default() -> Self {
    Self::new()
  }
}

impl PolicyTuner {
  pub fn new() -> Self {
    Self {
      hit_rate_history: Vec::new(),
      max_history: 10,
      min_samples: 5,
      cooldown: 10,
      frames_since_adjust: 0,
    }
  }

  /// Create with custom parameters
  pub fn with_params(max_history: usize, min_samples: usize, cooldown: usize) -> Self {
    Self {
      hit_rate_history: Vec::new(),
      max_history,
      min_samples,
      cooldown,
      frames_since_adjust: 0,
    }
  }

  /// Get current history length
  pub fn history_len(&self) -> usize {
    self.hit_rate_history.len()
  }

  // 헌법 준수 (P0-1): 값 계산 및 상태 변경 함수 제거
  // record_frame(), suggest_adjustment(), apply_adjustment(), tune(), avg_hit_rate(), reset() 등의
  // 값 계산/상태 변경 함수는 executor/runtime 계층에서 구현하세요.
}

// ============================================================
// Bounded Memo
// ============================================================

/// 크기 제한이 있는 메모 테이블 (FIFO 퇴거 정책)
///
/// FIFO가 LRU보다 선택된 이유:
/// - **예측 가능성**: 퇴거 순서가 결정론적
/// - **낮은 오버헤드**: 접근 시간 추적 불필요
/// - **FRP 적합성**: 최근 프레임이 캐시 사용을 지배함
///
/// > "FIFO는 예측 가능성과 낮은 오버헤드를 위해 선택됨;
/// > 퇴거 정책은 의미론적으로 관찰 가능하지 않음."
/// 헌법 P0-1 준수: 구조 정의만, 실행 없음
#[derive(Debug)]
pub struct BoundedMemo<K, V>
where
  K: std::hash::Hash + Eq + Clone,
{
  /// Key -> (value, insertion_order)
  entries: HashMap<K, (V, u64)>,
  /// Insertion counter for FIFO eviction
  counter: u64,
  /// Maximum entries
  max_entries: usize,
}

impl<K, V> BoundedMemo<K, V>
where
  K: std::hash::Hash + Eq + Clone,
  V: Clone,
{
  pub fn new(max_entries: usize) -> Self {
    Self {
      entries: HashMap::new(),
      counter: 0,
      max_entries,
    }
  }

  pub fn get(&self, key: &K) -> Option<&V> {
    self.entries.get(key).map(|(v, _)| v)
  }

  pub fn contains_key(&self, key: &K) -> bool {
    self.entries.contains_key(key)
  }

  pub fn insert(&mut self, key: K, value: V) {
    // Evict if at capacity
    if self.entries.len() >= self.max_entries && !self.entries.contains_key(&key) {
      self.evict_oldest();
    }

    self.counter += 1;
    self.entries.insert(key, (value, self.counter));
  }

  pub fn remove(&mut self, key: &K) -> Option<V> {
    self.entries.remove(key).map(|(v, _)| v)
  }

  pub fn clear(&mut self) {
    self.entries.clear();
    self.counter = 0;
  }

  pub fn len(&self) -> usize {
    self.entries.len()
  }

  pub fn is_empty(&self) -> bool {
    self.entries.is_empty()
  }

  /// Evict oldest entry using FIFO strategy.
  fn evict_oldest(&mut self) {
    if let Some((oldest_key, _)) = self
      .entries
      .iter()
      .min_by_key(|(_, (_, order))| order)
      .map(|(k, v)| (k.clone(), v.clone()))
    {
      self.entries.remove(&oldest_key);
    }
  }

  /// Evict entries not in the given key set
  pub fn retain_keys(&mut self, keys: &HashSet<K>) {
    self.entries.retain(|k, _| keys.contains(k));
  }

  /// Update max_entries (may trigger eviction)
  pub fn set_max_entries(&mut self, max: usize) {
    self.max_entries = max;
    while self.entries.len() > self.max_entries {
      self.evict_oldest();
    }
  }

  /// Get current max entries
  pub fn max_entries(&self) -> usize {
    self.max_entries
  }

  /// Get all keys
  pub fn keys(&self) -> impl Iterator<Item = &K> {
    self.entries.keys()
  }
}

// ============================================================
// Graph Cache (pnix-new specific)
// ============================================================

/// FxCore 그래프 노드 계산 캐시: 그래프 순회 중 중간 결과를 캐시하는 캐시
///
/// 컴파일 타임 사용을 위해 설계됨 (런타임 실행 아님)
/// 헌법 P0-1 준수: 구조 정의만, 실행 없음
pub struct GraphCache {
  /// Policy configuration
  policy: CachePolicy,
  /// Tuner for auto-adjustment
  #[allow(dead_code)]
  tuner: PolicyTuner,
  /// Node result cache (node_id -> serialized result)
  node_cache: BoundedMemo<String, String>,
  /// Statistics
  stats: CacheStats,
}

impl Default for GraphCache {
  fn default() -> Self {
    Self::new(CachePolicy::default())
  }
}

impl GraphCache {
  pub fn new(policy: CachePolicy) -> Self {
    let max_entries = policy.max_memo_entries;
    Self {
      policy,
      tuner: PolicyTuner::new(),
      node_cache: BoundedMemo::new(max_entries),
      stats: CacheStats::default(),
    }
  }

  /// Create with default policy
  pub fn with_defaults() -> Self {
    Self::default()
  }

  /// Get cached result for a node
  ///
  /// 헌법 준수 (P0-1): 상태 변경 제거
  /// record_hit(), record_miss() 호출은 executor/runtime 계층에서 구현하세요.
  pub fn get(&self, node_id: &str) -> Option<&String> {
    self.node_cache.get(&node_id.to_string())
  }

  /// Cache a node result
  ///
  /// 헌법 준수 (P0-1): 상태 변경 제거
  /// cached_keys_count 업데이트는 executor/runtime 계층에서 구현하세요.
  pub fn insert(&mut self, node_id: String, result: String) {
    self.node_cache.insert(node_id, result);
    // cached_keys_count는 executor에서 업데이트
  }

  /// Check if policy allows caching for given node size
  pub fn should_cache(&self, node_size: u32) -> bool {
    self.policy.is_enabled() && node_size >= self.policy.min_size
  }

  // Apply auto-tuning based on collected stats
  // 헌법 준수 (P0-1): 값 계산 및 상태 변경 함수 제거
  // auto_tune()는 값 계산 및 상태 변경이므로 executor/runtime 계층에서 구현하세요.

  /// Get current statistics
  pub fn stats(&self) -> &CacheStats {
    &self.stats
  }

  /// Get current policy
  pub fn policy(&self) -> &CachePolicy {
    &self.policy
  }

  // 헌법 준수 (P0-1): 상태 변경 함수 제거
  // reset()는 상태 변경이므로 executor/runtime 계층에서 구현하세요.

  /// Get cache size
  pub fn cache_size(&self) -> usize {
    self.node_cache.len()
  }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
  use super::*;

  // ─────────────────────────────────────────────────────────────────
  // CacheStats Tests
  // ─────────────────────────────────────────────────────────────────

  // 헌법 준수 (P0-1): 값 계산 테스트 제거
  // hit_rate(), record_hit(), record_miss() 테스트는 executor에서 수행하세요.

  // ─────────────────────────────────────────────────────────────────
  // Policy Tests
  // ─────────────────────────────────────────────────────────────────

  #[test]
  fn test_default_policy() {
    let policy = CachePolicy::default();
    assert_eq!(policy.min_size, 3);
    assert_eq!(policy.max_candidates, 16);
    assert!(policy.auto_tune);
    assert!(policy.is_enabled());
  }

  #[test]
  fn test_conservative_policy() {
    let policy = CachePolicy::conservative();
    assert!(policy.min_size > CachePolicy::default().min_size);
    assert!(policy.max_candidates < CachePolicy::default().max_candidates);
  }

  #[test]
  fn test_aggressive_policy() {
    let policy = CachePolicy::aggressive();
    assert!(policy.min_size < CachePolicy::default().min_size);
    assert!(policy.max_candidates > CachePolicy::default().max_candidates);
  }

  #[test]
  fn test_disabled_policy() {
    let policy = CachePolicy::disabled();
    assert_eq!(policy.max_candidates, 0);
    assert!(!policy.auto_tune);
    assert!(!policy.is_enabled());
  }

  #[test]
  fn test_policy_builder() {
    let policy = CachePolicy::new()
      .with_min_size(5)
      .with_max_candidates(32)
      .with_target_hit_rate(0.8);

    assert_eq!(policy.min_size, 5);
    assert_eq!(policy.max_candidates, 32);
    assert!((policy.target_hit_rate - 0.8).abs() < 0.001);
  }

  #[test]
  fn test_policy_hit_rate_clamping() {
    let policy = CachePolicy::new().with_target_hit_rate(1.5);
    assert!((policy.target_hit_rate - 1.0).abs() < 0.001);

    let policy = CachePolicy::new().with_target_hit_rate(-0.5);
    assert!((policy.target_hit_rate - 0.0).abs() < 0.001);
  }

  // ─────────────────────────────────────────────────────────────────
  // Tuner Tests
  // ─────────────────────────────────────────────────────────────────

  // 헌법 준수 (P0-1): 값 계산 테스트 제거
  // suggest_adjustment(), apply_adjustment(), tune(), avg_hit_rate(), reset() 테스트는 executor에서 수행하세요.

  // ─────────────────────────────────────────────────────────────────
  // BoundedMemo Tests
  // ─────────────────────────────────────────────────────────────────

  #[test]
  fn test_bounded_memo_basic() {
    let mut memo: BoundedMemo<u64, f64> = BoundedMemo::new(10);

    memo.insert(1, 1.0);
    memo.insert(2, 2.0);

    assert_eq!(memo.get(&1), Some(&1.0));
    assert_eq!(memo.get(&2), Some(&2.0));
    assert_eq!(memo.get(&3), None);
  }

  #[test]
  fn test_bounded_memo_eviction() {
    let mut memo: BoundedMemo<u64, i32> = BoundedMemo::new(3);

    memo.insert(1, 10);
    memo.insert(2, 20);
    memo.insert(3, 30);
    assert_eq!(memo.len(), 3);

    // Insert 4th, should evict oldest (1)
    memo.insert(4, 40);
    assert_eq!(memo.len(), 3);
    assert_eq!(memo.get(&1), None); // Evicted
    assert_eq!(memo.get(&4), Some(&40));
  }

  #[test]
  fn test_bounded_memo_retain_keys() {
    let mut memo: BoundedMemo<u64, i32> = BoundedMemo::new(10);

    memo.insert(1, 10);
    memo.insert(2, 20);
    memo.insert(3, 30);

    let mut keep = HashSet::new();
    keep.insert(1);
    keep.insert(3);

    memo.retain_keys(&keep);
    assert_eq!(memo.len(), 2);
    assert_eq!(memo.get(&1), Some(&10));
    assert_eq!(memo.get(&2), None);
    assert_eq!(memo.get(&3), Some(&30));
  }

  #[test]
  fn test_bounded_memo_string_keys() {
    let mut memo: BoundedMemo<String, String> = BoundedMemo::new(10);

    memo.insert("node_a".to_string(), "result_a".to_string());
    memo.insert("node_b".to_string(), "result_b".to_string());

    assert_eq!(
      memo.get(&"node_a".to_string()),
      Some(&"result_a".to_string())
    );
    assert!(memo.contains_key(&"node_b".to_string()));
    assert!(!memo.contains_key(&"node_c".to_string()));
  }

  #[test]
  fn test_bounded_memo_set_max_entries() {
    let mut memo: BoundedMemo<u64, i32> = BoundedMemo::new(10);

    for i in 0..10 {
      memo.insert(i, i as i32);
    }
    assert_eq!(memo.len(), 10);

    // Reduce max_entries, should trigger eviction
    memo.set_max_entries(5);
    assert_eq!(memo.len(), 5);
    assert_eq!(memo.max_entries(), 5);
  }

  // ─────────────────────────────────────────────────────────────────
  // GraphCache Tests
  // ─────────────────────────────────────────────────────────────────

  #[test]
  fn test_graph_cache_basic() {
    let mut cache = GraphCache::with_defaults();

    cache.insert("node_1".to_string(), "result_1".to_string());
    assert_eq!(cache.get("node_1"), Some(&"result_1".to_string()));
    assert_eq!(cache.get("node_2"), None);
  }

  // 헌법 준수 (P0-1): 값 계산 테스트 제거
  // stats().hits, stats().misses 테스트는 executor에서 수행하세요.

  #[test]
  fn test_graph_cache_should_cache() {
    let cache = GraphCache::new(CachePolicy::default().with_min_size(5));

    assert!(cache.should_cache(5));
    assert!(cache.should_cache(10));
    assert!(!cache.should_cache(4));
    assert!(!cache.should_cache(1));
  }

  #[test]
  fn test_graph_cache_disabled() {
    let cache = GraphCache::new(CachePolicy::disabled());
    assert!(!cache.should_cache(100));
  }

  // 헌법 준수 (P0-1): 상태 변경 테스트 제거
  // reset() 테스트는 executor에서 수행하세요.
}
