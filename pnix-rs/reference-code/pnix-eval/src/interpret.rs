//! pnix-eval tree-walking interpreter.

use crate::fx_map::{fx_hashmap_with_capacity, FxHashMap, FxHashSet};
use crate::value::{AttrSourcePos, Env, SelectBorrowed, Value};
use crate::{markup, math_markup, schema, svg, x3d, xml_format_schema};
use anyhow::{anyhow, Result};
use pnix_core::lang::pnix::syntax::{
  PnixAttrItem, PnixExpr, PnixLetBinding, PnixListPattern, PnixParamPattern, PnixPatternField,
  StringInterpPart,
};
use regex::{Regex, RegexBuilder};
use pnix_hash::{Digest, Sha256};
use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

const READFILE_MAX_SIZE: u64 = 100 * 1024 * 1024;
const REGEX_SIZE_LIMIT_BYTES: usize = 1_000_000;
const REGEX_DFA_SIZE_LIMIT_BYTES: usize = 1_000_000;
const DEEP_FORCE_MAX_DEPTH: usize = 1_000;
const INLINE_PARSE_CACHE_MAX_ENTRIES: usize = 256;
const INLINE_PARSE_CACHE_MAX_SOURCE_BYTES: usize = 1024 * 1024;
const READFILE_CACHE_MAX_ENTRIES: usize = 256;
const READFILE_CACHE_MAX_BYTES: u64 = 4 * 1024 * 1024;
const READDIR_CACHE_MAX_ENTRIES: usize = 128;
const CANONICAL_PATH_CACHE_MAX_ENTRIES: usize = 1024;
const NORMALIZED_PATH_CACHE_MAX_ENTRIES: usize = 2048;
const REGEX_CACHE_MAX_ENTRIES: usize = 128;
const REGEX_CACHE_MAX_PATTERN_BYTES: usize = 64 * 1024;
const MATCH_ANCHORED_PATTERN_CACHE_MAX_ENTRIES: usize = 128;
// perf (2026-06-19): 128 → 4096. import VALUE 캐시도 dependency-graph 캐시와
// 같은 thrashing 함정에 빠진다. 한글 NL 거울(`korean-nl-mirror.px`)의 transitive
// 폐포는 ~570 모듈이라 128 cap 으로는 evict-one-per-insert 가 일어나, serve 의
// 상주 chat-eval 스레드에서 매 요청마다 루트 모듈(`m`)이 쫓겨난다. 그러면 다음
// 요청은 `m` 을 다시 import → 폐포 전체를 다시 평가/다시 lex(=`char_width` 가
// on-CPU 표본 1위가 되는 원인). 폐포보다 큰 cap 으로 키우면 `m` 과 forced thunk
// 가 요청 간 유지되어 force_node_count 가 콜드(~212k) → 웜으로 급감한다.
// workspace 가 ~1.3k 모듈이므로 IMPORT_DEPENDENCY_GRAPH 캐시와 같은 4096 으로
// 맞춘다(엔트리는 Arc<Value> 포인터 + 의존성 스냅샷이라 메모리는 thread 당 수 MB).
const IMPORT_VALUE_CACHE_MAX_ENTRIES: usize = 4096;
// 4096: the workspace has ~1.3k .px modules; a cap below the transitive
// closure makes the graph DFS thrash its own cache (evict-one per
// insert) and re-walk subtrees combinatorially — measured as ~60-70s of
// pure import_dependency_graph_build on the serve catch-all cold boot.
// Entries are path+hash records (~KB), so the worst case is a few MB
// per thread.
const IMPORT_DEPENDENCY_GRAPH_CACHE_MAX_ENTRIES: usize = 4096;
const INLINE_PARSE_CACHE_INITIAL_ENTRIES: usize = 64;
const READFILE_CACHE_INITIAL_ENTRIES: usize = 64;
const READDIR_CACHE_INITIAL_ENTRIES: usize = 32;
const CANONICAL_PATH_CACHE_INITIAL_ENTRIES: usize = 128;
const NORMALIZED_PATH_CACHE_INITIAL_ENTRIES: usize = 256;
const REGEX_CACHE_INITIAL_ENTRIES: usize = 32;
const MATCH_ANCHORED_PATTERN_CACHE_INITIAL_ENTRIES: usize = 32;
const HEX_LOWER: &[u8; 16] = b"0123456789abcdef";

pub const ENV_IMPORT_VALUE_CACHE: &str = "PNIX_EVAL_IMPORT_VALUE_CACHE";
pub const ENV_DISK_AST_CACHE: &str = "PNIX_EVAL_DISK_AST_CACHE";

fn eval_cache_env_default_on_value(var_name: &str) -> bool {
  match std::env::var(var_name).ok().as_deref() {
    None => true,
    Some("0") | Some("false") | Some("no") | Some("FALSE") | Some("NO") => false,
    _ => true,
  }
}

pub fn import_value_cache_production_default_on() -> bool {
  eval_cache_env_default_on_value(ENV_IMPORT_VALUE_CACHE)
}

pub fn disk_ast_cache_production_default_on() -> bool {
  eval_cache_env_default_on_value(ENV_DISK_AST_CACHE)
}

thread_local! {
  /// When true, perf fixture observations force cold materialization counters
  /// (disk AST cache disabled for this thread) so loaded_file_count reflects
  /// actual file reads instead of cross-run disk cache hits.
  static PERF_FIXTURE_OBSERVATION: Cell<bool> = const { Cell::new(false) };
}

pub fn perf_fixture_observation_active() -> bool {
  PERF_FIXTURE_OBSERVATION.with(|c| c.get())
}

fn import_value_cache_enabled() -> bool {
  static ENABLED: OnceLock<bool> = OnceLock::new();
  *ENABLED.get_or_init(import_value_cache_production_default_on)
}

fn disk_ast_cache_enabled() -> bool {
  if perf_fixture_observation_active() {
    return false;
  }
  static ENABLED: OnceLock<bool> = OnceLock::new();
  *ENABLED.get_or_init(disk_ast_cache_production_default_on)
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EvalPerfStats {
  pub io_ms: u128,
  pub source_hash_ms: u128,
  pub parse_ms: u128,
  pub import_resolve_ms: u128,
  pub normalize_ms: u128,
  pub deep_force_ms: u128,
  pub cache_lookup_ms: u128,
  pub loaded_file_count: usize,
  pub import_count: usize,
  pub source_bytes_loaded: usize,
  pub source_hash_count: usize,
  pub parse_count: usize,
  pub file_read_count: usize,
  pub ast_cache_lookup_count: usize,
  pub ast_cache_hit_count: usize,
  pub ast_cache_miss_count: usize,
  pub preparsed_ast_hit_count: usize,
  pub inline_parse_cache_hit_count: usize,
  pub disk_ast_cache_hit_count: usize,
  pub disk_ast_binary_cache_hit_count: usize,
  pub disk_ast_json_cache_hit_count: usize,
  pub disk_ast_cache_stale_count: usize,
  pub disk_ast_cache_stale_mtime_count: usize,
  pub disk_ast_cache_stale_len_count: usize,
  pub disk_ast_cache_stale_sha_count: usize,
  pub disk_ast_cache_stale_evaluator_version_count: usize,
  pub disk_ast_cache_stale_feature_flags_count: usize,
  pub source_read_skipped_by_ast_cache_hit_count: usize,
  pub ast_cache_fast_header_hit_count: usize,
  pub ast_cache_strict_sha_revalidated_count: usize,
  pub parsed_ast_miss_count: usize,
  pub read_file_cache_hit_count: usize,
  pub read_file_cache_miss_count: usize,
  pub read_dir_cache_hit_count: usize,
  pub read_dir_cache_miss_count: usize,
  pub canonical_path_cache_hit_count: usize,
  pub canonical_path_cache_miss_count: usize,
  pub path_normalize_cache_hit_count: usize,
  pub path_normalize_cache_miss_count: usize,
  pub regex_cache_hit_count: usize,
  pub regex_cache_miss_count: usize,
  pub match_anchored_pattern_cache_hit_count: usize,
  pub match_anchored_pattern_cache_miss_count: usize,
  pub import_value_cache_hit_count: usize,
  pub import_value_cache_miss_count: usize,
  pub import_value_cache_store_count: usize,
  pub import_value_cache_uncacheable_count: usize,
  pub import_value_cache_dependency_stale_count: usize,
  pub import_dependency_static_count: usize,
  pub import_dependency_dynamic_count: usize,
  pub dependency_graph_cache_hit_count: usize,
  pub dependency_graph_cache_miss_count: usize,
  pub dependency_graph_cache_store_count: usize,
  pub builtin_partial_arity_fast_path_count: usize,
  pub builtins_select_fast_path_count: usize,
  pub builtins_has_attr_fast_path_count: usize,
  pub force_node_count: usize,
  pub force_attr_count: usize,
  pub force_list_count: usize,
  pub force_thunk_count: usize,
  pub force_max_depth: usize,
}

impl EvalPerfStats {
  pub fn cache_hit(&self) -> bool {
    self.ast_cache_hit_count > 0
      || self.preparsed_ast_hit_count > 0
      || self.inline_parse_cache_hit_count > 0
      || self.disk_ast_cache_hit_count > 0
      || self.ast_cache_fast_header_hit_count > 0
      || self.read_file_cache_hit_count > 0
      || self.read_dir_cache_hit_count > 0
      || self.canonical_path_cache_hit_count > 0
      || self.path_normalize_cache_hit_count > 0
      || self.regex_cache_hit_count > 0
      || self.match_anchored_pattern_cache_hit_count > 0
      || self.import_value_cache_hit_count > 0
      || self.import_dependency_static_count > 0
      || self.dependency_graph_cache_hit_count > 0
  }

  pub fn cache_miss(&self) -> bool {
    self.ast_cache_miss_count > 0
      || self.parsed_ast_miss_count > 0
      || self.disk_ast_cache_stale_count > 0
      || self.read_file_cache_miss_count > 0
      || self.read_dir_cache_miss_count > 0
      || self.canonical_path_cache_miss_count > 0
      || self.path_normalize_cache_miss_count > 0
      || self.regex_cache_miss_count > 0
      || self.match_anchored_pattern_cache_miss_count > 0
      || self.import_value_cache_miss_count > 0
      || self.import_value_cache_dependency_stale_count > 0
      || self.import_dependency_dynamic_count > 0
      || self.dependency_graph_cache_miss_count > 0
  }
}

fn push_hex_lower(bytes: &[u8], out: &mut String) {
  out.reserve(bytes.len().saturating_mul(2));
  for &byte in bytes {
    out.push(HEX_LOWER[(byte >> 4) as usize] as char);
    out.push(HEX_LOWER[(byte & 0x0f) as usize] as char);
  }
}

fn push_u64_hex_lower_16(value: u64, out: &mut String) {
  out.reserve(16);
  for shift in (0..=60).rev().step_by(4) {
    out.push(HEX_LOWER[((value >> shift) & 0x0f) as usize] as char);
  }
}

fn hex_lower(bytes: &[u8]) -> String {
  let mut out = String::with_capacity(bytes.len().saturating_mul(2));
  push_hex_lower(bytes, &mut out);
  out
}

#[derive(Clone)]
struct ReadFileCacheEntry {
  len: u64,
  mtime_ns: Option<u128>,
  content: String,
}

#[derive(Clone)]
struct ReadDirCacheEntry {
  len: u64,
  mtime_ns: Option<u128>,
  entries: BTreeMap<String, String>,
}

#[derive(Clone)]
struct ImportDependencySnapshot {
  path: PathBuf,
  source_len: u64,
  source_mtime_ns: Option<u128>,
}

struct ImportDependencyFrame {
  dependencies: Vec<ImportDependencySnapshot>,
}

struct ImportDependencyFrameGuard {
  active: bool,
}

#[derive(Clone)]
struct ImportDependencyGraphEntry {
  source_len: u64,
  source_mtime_ns: u128,
  direct_imports: Vec<PathBuf>,
  transitive_imports: Vec<PathBuf>,
  dependency_hash: String,
}

thread_local! {
  static IMPORT_BASE_STACK: RefCell<Vec<PathBuf>> = const { RefCell::new(Vec::new()) };
  /// Active import chain — every `eval_file_at_path` invocation
  /// pushes the canonical absolute path of the file before
  /// evaluating its body and pops on exit. A second push of a path
  /// already on the stack is a cycle (`a.px` imports `b.px` imports
  /// `a.px`) and surfaces as a clear error instead of recursing
  /// until the Rust call stack overflows.
  static IMPORT_FILE_STACK: RefCell<Vec<PathBuf>> = const { RefCell::new(Vec::new()) };
  static IMPORT_DEPENDENCY_FRAMES: RefCell<Vec<ImportDependencyFrame>> =
    const { RefCell::new(Vec::new()) };
  /// 2026-05-05: cross-call interpolation depth counter. The
  /// `coerce_to_string_for_interpolation_inner` function takes a
  /// `depth` parameter, but it only protects within-call AttrSet
  /// `__toString` chains. A pathological pattern like
  /// `let s = { __toString = self: "${s}"; }; in "${s}"` re-enters
  /// `eval` for each `"${s}"` interpolation, which then calls
  /// `coerce_to_string_for_interpolation(s)` afresh with depth = 0,
  /// so the per-call check never fires and the Rust call stack
  /// blows up. This thread-local persists across the eval re-entry
  /// and surfaces the cycle as a typed error before it can DoS the
  /// evaluator. The `coerce_to_string_for_interpolation` wrapper
  /// increments on entry and decrements on `InterpDepthGuard::drop`,
  /// so the counter is exception-safe.
  static INTERP_COERCE_DEPTH: RefCell<usize> = const { RefCell::new(0) };
  static EVAL_PERF_STATS: RefCell<EvalPerfStats> = RefCell::new(EvalPerfStats::default());
  static INLINE_PARSE_CACHE: RefCell<FxHashMap<String, Arc<PnixExpr>>> =
    RefCell::new(fx_hashmap_with_capacity(INLINE_PARSE_CACHE_INITIAL_ENTRIES));
  static READ_FILE_CACHE: RefCell<FxHashMap<PathBuf, ReadFileCacheEntry>> =
    RefCell::new(fx_hashmap_with_capacity(READFILE_CACHE_INITIAL_ENTRIES));
  static READ_DIR_CACHE: RefCell<FxHashMap<PathBuf, ReadDirCacheEntry>> =
    RefCell::new(fx_hashmap_with_capacity(READDIR_CACHE_INITIAL_ENTRIES));
  static CANONICAL_PATH_CACHE: RefCell<FxHashMap<PathBuf, PathBuf>> =
    RefCell::new(fx_hashmap_with_capacity(CANONICAL_PATH_CACHE_INITIAL_ENTRIES));
  static NORMALIZED_PATH_CACHE: RefCell<FxHashMap<PathBuf, PathBuf>> =
    RefCell::new(fx_hashmap_with_capacity(NORMALIZED_PATH_CACHE_INITIAL_ENTRIES));
  static REGEX_CACHE: RefCell<FxHashMap<String, Regex>> =
    RefCell::new(fx_hashmap_with_capacity(REGEX_CACHE_INITIAL_ENTRIES));
  static MATCH_ANCHORED_PATTERN_CACHE: RefCell<FxHashMap<String, Arc<str>>> =
    RefCell::new(fx_hashmap_with_capacity(MATCH_ANCHORED_PATTERN_CACHE_INITIAL_ENTRIES));
}

fn evict_one_cache_entry<K, V>(cache: &mut FxHashMap<K, V>, max_entries: usize)
where
  K: Clone + Eq + std::hash::Hash,
{
  if cache.len() < max_entries {
    return;
  }
  if let Some(key) = cache.keys().next().cloned() {
    cache.remove(&key);
  } else {
    cache.clear();
  }
}

pub fn reset_eval_perf_stats() {
  EVAL_PERF_STATS.with(|stats| {
    *stats.borrow_mut() = EvalPerfStats::default();
  });
}

/// Clears thread-local import/preparsed caches so perf fixture observations record
/// per-eval materialization counters instead of cross-test cache hits.
pub fn clear_eval_caches_for_perf_observation() {
  PREPARSED_IMPORTS.with(|cell| cell.borrow_mut().clear());
  IMPORT_VALUE_CACHE.with(|cell| cell.borrow_mut().clear());
  IMPORT_DEPENDENCY_GRAPH_CACHE.with(|cell| cell.borrow_mut().clear());
}

pub fn reset_eval_perf_stats_for_fixture_observation() {
  clear_eval_caches_for_perf_observation();
  reset_eval_perf_stats();
  PERF_FIXTURE_OBSERVATION.with(|c| c.set(true));
}

pub fn take_eval_perf_stats() -> EvalPerfStats {
  EVAL_PERF_STATS.with(|stats| {
    let snapshot = stats.borrow().clone();
    *stats.borrow_mut() = EvalPerfStats::default();
    snapshot
  })
}

fn record_eval_perf(update: impl FnOnce(&mut EvalPerfStats)) {
  EVAL_PERF_STATS.with(|stats| {
    update(&mut stats.borrow_mut());
  });
}

pub(crate) fn record_inline_parse(_source_bytes: usize, elapsed: std::time::Duration) {
  record_eval_perf(|stats| {
    stats.parse_ms += elapsed.as_millis();
    stats.parse_count += 1;
  });
}

pub(crate) fn parse_expr_arc_with_inline_cache(source: &str) -> Result<Arc<PnixExpr>> {
  let cacheable = source.len() <= INLINE_PARSE_CACHE_MAX_SOURCE_BYTES;
  if cacheable {
    let lookup_started = cache_lookup_timing_started();
    let cached = INLINE_PARSE_CACHE.with(|cache| cache.borrow().get(source).cloned());
    record_cache_lookup_elapsed(lookup_started);
    if let Some(expr) = cached {
      record_inline_parse_cache_hit();
      return Ok(expr);
    }
  }

  record_ast_cache_miss();
  let parse_started = std::time::Instant::now();
  let parsed =
    pnix_core::lang::pnix::parse_expr(source).map_err(|e| anyhow!("parse error: {}", e))?;
  record_inline_parse(source.len(), parse_started.elapsed());
  let parsed = Arc::new(parsed);
  if cacheable {
    INLINE_PARSE_CACHE.with(|cache| {
      let mut cache = cache.borrow_mut();
      evict_one_cache_entry(&mut cache, INLINE_PARSE_CACHE_MAX_ENTRIES);
      cache.insert(source.to_string(), parsed.clone());
    });
  }
  Ok(parsed)
}

fn record_cache_lookup(elapsed: std::time::Duration) {
  record_eval_perf(|stats| {
    stats.cache_lookup_ms += elapsed.as_millis();
    stats.ast_cache_lookup_count += 1;
  });
}

/// T-2 (2026-06-12): cache-lookup wall-clock pairs behind the same
/// host opt-out as the deep-force clocks (T-1). Post-D-1/D-2 the W=8
/// daemon profile showed `mach_absolute_time` at 256 leaf samples —
/// the canonicalize/import-cache lookups each paid an `Instant` pair
/// purely to accrue `cache_lookup_ms`, which no daemon surface reads.
/// Counter semantics under the opt-out: `ast_cache_lookup_count` still
/// increments (clock-free), `cache_lookup_ms` reads 0 — identical
/// contract to `deep_force_ms`.
#[inline]
fn cache_lookup_timing_started() -> Option<std::time::Instant> {
  deep_force_timing_enabled().then(std::time::Instant::now)
}

#[inline]
fn record_cache_lookup_elapsed(started: Option<std::time::Instant>) {
  match started {
    Some(s) => record_cache_lookup(s.elapsed()),
    // Keep the lookup COUNT accurate even with clocks off.
    None => record_eval_perf(|stats| {
      stats.ast_cache_lookup_count += 1;
    }),
  }
}

fn record_preparsed_ast_hit() {
  record_eval_perf(|stats| {
    stats.ast_cache_hit_count += 1;
    stats.preparsed_ast_hit_count += 1;
  });
}

fn record_read_file_cache_hit() {
  record_eval_perf(|stats| {
    stats.ast_cache_hit_count += 1;
    stats.read_file_cache_hit_count += 1;
  });
}

fn record_read_file_cache_miss() {
  record_eval_perf(|stats| {
    stats.ast_cache_miss_count += 1;
    stats.read_file_cache_miss_count += 1;
  });
}

fn record_read_dir_cache_hit() {
  record_eval_perf(|stats| {
    stats.ast_cache_hit_count += 1;
    stats.read_dir_cache_hit_count += 1;
  });
}

fn record_read_dir_cache_miss() {
  record_eval_perf(|stats| {
    stats.ast_cache_miss_count += 1;
    stats.read_dir_cache_miss_count += 1;
  });
}

fn record_import_value_cache_hit() {
  record_eval_perf(|stats| {
    stats.ast_cache_hit_count += 1;
    stats.import_value_cache_hit_count += 1;
  });
}

fn record_import_value_cache_miss() {
  record_eval_perf(|stats| {
    stats.ast_cache_miss_count += 1;
    stats.import_value_cache_miss_count += 1;
  });
}

fn record_import_value_cache_store() {
  record_eval_perf(|stats| {
    stats.import_value_cache_store_count += 1;
  });
}

fn record_import_value_cache_uncacheable() {
  record_eval_perf(|stats| {
    stats.import_value_cache_uncacheable_count += 1;
  });
}

fn record_import_value_cache_dependency_stale() {
  record_eval_perf(|stats| {
    stats.import_value_cache_dependency_stale_count += 1;
  });
}

fn record_import_dependency_static() {
  record_eval_perf(|stats| {
    stats.import_dependency_static_count += 1;
  });
}

fn record_import_dependency_dynamic() {
  record_eval_perf(|stats| {
    stats.import_dependency_dynamic_count += 1;
  });
}

fn record_dependency_graph_cache_hit() {
  record_eval_perf(|stats| {
    stats.ast_cache_hit_count += 1;
    stats.dependency_graph_cache_hit_count += 1;
  });
}

fn record_dependency_graph_cache_miss() {
  record_eval_perf(|stats| {
    stats.ast_cache_miss_count += 1;
    stats.dependency_graph_cache_miss_count += 1;
  });
}

fn record_dependency_graph_cache_store() {
  record_eval_perf(|stats| {
    stats.dependency_graph_cache_store_count += 1;
  });
}

fn record_canonical_path_cache_hit() {
  record_eval_perf(|stats| {
    stats.ast_cache_hit_count += 1;
    stats.canonical_path_cache_hit_count += 1;
  });
}

fn record_canonical_path_cache_miss() {
  record_eval_perf(|stats| {
    stats.ast_cache_miss_count += 1;
    stats.canonical_path_cache_miss_count += 1;
  });
}

fn record_path_normalize_cache_hit() {
  record_eval_perf(|stats| {
    stats.ast_cache_hit_count += 1;
    stats.path_normalize_cache_hit_count += 1;
  });
}

fn record_path_normalize_cache_miss() {
  record_eval_perf(|stats| {
    stats.ast_cache_miss_count += 1;
    stats.path_normalize_cache_miss_count += 1;
  });
}

fn record_regex_cache_hit() {
  record_eval_perf(|stats| {
    stats.ast_cache_hit_count += 1;
    stats.regex_cache_hit_count += 1;
  });
}

fn record_regex_cache_miss() {
  record_eval_perf(|stats| {
    stats.ast_cache_miss_count += 1;
    stats.regex_cache_miss_count += 1;
  });
}

fn record_match_anchored_pattern_cache_hit() {
  record_eval_perf(|stats| {
    stats.match_anchored_pattern_cache_hit_count += 1;
  });
}

fn record_match_anchored_pattern_cache_miss() {
  record_eval_perf(|stats| {
    stats.match_anchored_pattern_cache_miss_count += 1;
  });
}

fn record_builtins_select_fast_path() {
  record_eval_perf(|stats| {
    stats.builtins_select_fast_path_count += 1;
  });
}

fn record_builtin_partial_arity_fast_path() {
  record_eval_perf(|stats| {
    stats.builtin_partial_arity_fast_path_count += 1;
  });
}

fn record_builtins_has_attr_fast_path() {
  record_eval_perf(|stats| {
    stats.builtins_has_attr_fast_path_count += 1;
  });
}

#[derive(Default)]
struct DeepForcePerf {
  node_count: usize,
  attr_count: usize,
  list_count: usize,
  thunk_count: usize,
  max_depth: usize,
}

fn observe_deep_force_value(value: &Value, depth: usize, perf: &mut DeepForcePerf) {
  perf.node_count += 1;
  perf.max_depth = perf.max_depth.max(depth);
  match value {
    Value::AttrSet(_) => perf.attr_count += 1,
    Value::List(_) => perf.list_count += 1,
    Value::Thunk { .. } => perf.thunk_count += 1,
    _ => {}
  }
}

/// Opt-OUT knob for the deep-force wall-clock instrumentation
/// (2026-06-11). Each deep-force entry pays two clock syscalls
/// (Instant::now + elapsed) purely to accrue `deep_force_ms` — ~2.5%
/// of substrate-wide user CPU, and relatively more once allocation
/// cost drops (PxHeap). Default is ON so every existing perf-summary
/// surface (one-shot stderr, perf reports, daemon summaries) keeps
/// byte-for-byte behavior; ONLY an explicit
/// `PNIX_EVAL_DISABLE_PERF_TIMING=1` skips the clocks, in which case
/// `deep_force_ms` reads 0 while every force_* COUNTER keeps
/// recording exactly as before (counters never depended on the
/// clock). Intended for throughput-sensitive operators (`--serve` /
/// `--daemon`) and benchmarking; never set implicitly.
fn deep_force_timing_enabled() -> bool {
  if DEEP_FORCE_TIMING_HOST_DISABLED.load(std::sync::atomic::Ordering::Relaxed) {
    return false;
  }
  static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
  *ENABLED.get_or_init(|| {
    !std::env::var("PNIX_EVAL_DISABLE_PERF_TIMING").is_ok_and(|v| {
      matches!(
        v.as_str(),
        "1" | "true" | "TRUE" | "yes" | "YES" | "on" | "ON"
      )
    })
  })
}

static DEEP_FORCE_TIMING_HOST_DISABLED: std::sync::atomic::AtomicBool =
  std::sync::atomic::AtomicBool::new(false);

/// Host-driven form of the `PNIX_EVAL_DISABLE_PERF_TIMING` opt-out
/// (T-1, 2026-06-11). A host binary that KNOWS no perf-summary surface
/// will consume `deep_force_ms` this process (e.g. pnixc-meta one-shot
/// without `PNIXC_META_ONE_SHOT_PERF_STDERR`) calls this once before
/// evaluation to skip the per-deep-force clock pair. Post-campaign the
/// clocks measure ~8% of substrate-wide user CPU (the fixed syscall
/// cost grew relatively as eval shrank 4×). Same contract as the env
/// knob: `deep_force_ms` reads 0; every force_* COUNTER keeps
/// recording. There is deliberately no re-enable — flip-flopping
/// mid-eval would make `deep_force_ms` meaningless.
pub fn disable_deep_force_perf_timing() {
  DEEP_FORCE_TIMING_HOST_DISABLED.store(true, std::sync::atomic::Ordering::Relaxed);
}

fn record_deep_force_perf(perf: &DeepForcePerf, elapsed: std::time::Duration) {
  record_eval_perf(|stats| {
    stats.deep_force_ms += elapsed.as_millis();
    stats.force_node_count += perf.node_count;
    stats.force_attr_count += perf.attr_count;
    stats.force_list_count += perf.list_count;
    stats.force_thunk_count += perf.thunk_count;
    stats.force_max_depth = stats.force_max_depth.max(perf.max_depth);
  });
}

fn value_is_deep_force_leaf(value: &Value) -> bool {
  matches!(
    value,
    Value::Null
      | Value::Bool(_)
      | Value::Int(_)
      | Value::Float(_)
      | Value::String(_)
      | Value::StringContext { .. }
      | Value::Path(_)
      | Value::Lambda { .. }
      | Value::BuiltinPartial { .. }
  )
}

fn record_inline_parse_cache_hit() {
  record_eval_perf(|stats| {
    stats.ast_cache_hit_count += 1;
    stats.inline_parse_cache_hit_count += 1;
  });
}

fn record_disk_ast_cache_hit() {
  record_eval_perf(|stats| {
    stats.ast_cache_hit_count += 1;
    stats.disk_ast_cache_hit_count += 1;
  });
}

fn record_disk_ast_binary_cache_hit() {
  record_disk_ast_cache_hit();
  record_eval_perf(|stats| {
    stats.disk_ast_binary_cache_hit_count += 1;
  });
}

fn record_disk_ast_json_cache_hit() {
  record_disk_ast_cache_hit();
  record_eval_perf(|stats| {
    stats.disk_ast_json_cache_hit_count += 1;
  });
}

fn record_source_read_skipped_by_ast_cache_hit() {
  record_eval_perf(|stats| {
    stats.source_read_skipped_by_ast_cache_hit_count += 1;
  });
}

fn record_ast_cache_fast_header_hit() {
  record_eval_perf(|stats| {
    stats.ast_cache_fast_header_hit_count += 1;
  });
}

fn record_ast_cache_strict_sha_revalidated() {
  record_eval_perf(|stats| {
    stats.ast_cache_strict_sha_revalidated_count += 1;
  });
}

fn record_ast_cache_miss() {
  record_eval_perf(|stats| {
    stats.ast_cache_miss_count += 1;
    stats.parsed_ast_miss_count += 1;
  });
}

fn record_disk_ast_cache_stale(reason: AstDiskCacheStaleReason) {
  record_eval_perf(|stats| {
    stats.ast_cache_miss_count += 1;
    stats.disk_ast_cache_stale_count += 1;
    match reason {
      AstDiskCacheStaleReason::Mtime => stats.disk_ast_cache_stale_mtime_count += 1,
      AstDiskCacheStaleReason::Len => stats.disk_ast_cache_stale_len_count += 1,
      AstDiskCacheStaleReason::SourceSha256 => stats.disk_ast_cache_stale_sha_count += 1,
      AstDiskCacheStaleReason::EvaluatorVersion => {
        stats.disk_ast_cache_stale_evaluator_version_count += 1
      }
      AstDiskCacheStaleReason::FeatureFlags => stats.disk_ast_cache_stale_feature_flags_count += 1,
    }
  });
}

fn record_file_read(source_bytes: usize, elapsed: std::time::Duration) {
  record_eval_perf(|stats| {
    stats.io_ms += elapsed.as_millis();
    stats.loaded_file_count += 1;
    stats.file_read_count += 1;
    stats.source_bytes_loaded += source_bytes;
  });
}

fn record_source_hash(elapsed: std::time::Duration) {
  record_eval_perf(|stats| {
    stats.source_hash_ms += elapsed.as_millis();
    stats.source_hash_count += 1;
  });
}

fn record_file_parse(_source_bytes: usize, elapsed: std::time::Duration) {
  record_eval_perf(|stats| {
    stats.parse_ms += elapsed.as_millis();
    stats.parse_count += 1;
  });
}

fn record_normalize(elapsed: std::time::Duration) {
  record_eval_perf(|stats| {
    stats.normalize_ms += elapsed.as_millis();
  });
}

struct InterpDepthGuard;

// 2026-05-05 (slice #40 + slice #58): cross-call coerce-to-
// string cycle limit. The same thread-local guard is shared
// between interpolation (`${...}`) and the `builtins.toString`
// builtin path (slice #58 extension). Each cycle iteration
// via `__toString` returning a string with `${self}` or via
// `__toString` calling `builtins.toString self` adds many
// Rust call frames — eval StringInterp / Apply nodes, coerce
// helpers, apply_value, lambda body eval, recursive apply_
// builtin, plus the inner apply machinery. The toString path
// adds MORE frames per cycle than the interp path (roughly
// 2-3x as many eval frames per round-trip), so the limit must
// be tight enough to fire before the Rust test thread stack
// (default 2 MB) overflows.
//
// Each level adds heavy eval frames; 8 leaves headroom for
// legitimate two- or three-deep `__toString` / `outPath`
// chains while catching the cycle within ~5-10 KB of stack
// remaining. Bidirectional cycles (interp ↔ toString) add 2
// guard entries per round-trip, so a limit of 8 gives ~4
// round-trips before firing — still ample for non-cyclic
// chains. The within-call `depth` parameter in
// `coerce_to_string_for_*_inner` / `_with_context` uses
// `COERCE_INTERP_DEPTH_LIMIT` (64) — that's fine for the
// per-call recursion which has lighter frames.
const INTERP_COERCE_CYCLE_LIMIT: usize = 8;

impl InterpDepthGuard {
  fn enter() -> Result<Self> {
    INTERP_COERCE_DEPTH.with(|d| {
      let mut d = d.borrow_mut();
      if *d > INTERP_COERCE_CYCLE_LIMIT {
        return Err(anyhow!(
          "string interpolation coercion cycle: \
           `__toString` re-entered the interpolation path more than {} times \
           (likely a self-referential `__toString` returning `\"${{self}}\"`)",
          INTERP_COERCE_CYCLE_LIMIT
        ));
      }
      *d += 1;
      Ok(())
    })?;
    Ok(Self)
  }
}

impl Drop for InterpDepthGuard {
  fn drop(&mut self) {
    INTERP_COERCE_DEPTH.with(|d| {
      let mut d = d.borrow_mut();
      if *d > 0 {
        *d -= 1;
      }
    });
  }
}

struct ImportBaseGuard;

impl ImportBaseGuard {
  fn push(base: PathBuf) -> Self {
    IMPORT_BASE_STACK.with(|stack| stack.borrow_mut().push(base));
    Self
  }
}

impl Drop for ImportBaseGuard {
  fn drop(&mut self) {
    IMPORT_BASE_STACK.with(|stack| {
      stack.borrow_mut().pop();
    });
  }
}

/// RAII guard that records that we are currently importing `path`.
/// `push_checked` returns an error if `path` is already on the stack
/// (cycle detected) and otherwise pushes it. `Drop` pops.
///
/// S11 perf slice (2026-05-21): the guard now exposes the
/// canonicalized form via `canon()` so callers (e.g.
/// `eval_file_at_path`) can reuse it for the PREPARSED_IMPORTS cache
/// lookup. Previously canonicalize() ran inside push_checked and the
/// result was thrown away; the cache lookup then used the raw `path`,
/// so callers passing different forms of the same file
/// (`./foo.px` vs `/abs/foo.px`) missed the cache and re-canonicalized
/// + re-parsed. Reusing the canon path means (a) one fewer
/// canonicalize() syscall per import and (b) higher cache hit rate
/// across worker threads passing varied path forms.
struct ImportFileGuard {
  canon: PathBuf,
}

fn canonicalize_import_path_with_cache(path: &Path) -> PathBuf {
  let cache_started = cache_lookup_timing_started();
  let cached = CANONICAL_PATH_CACHE.with(|cache| cache.borrow().get(path).cloned());
  record_cache_lookup_elapsed(cache_started);
  if let Some(canon) = cached {
    record_canonical_path_cache_hit();
    return canon;
  }
  record_canonical_path_cache_miss();

  match path.canonicalize() {
    Ok(canon) => {
      CANONICAL_PATH_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        evict_one_cache_entry(&mut cache, CANONICAL_PATH_CACHE_MAX_ENTRIES);
        cache.insert(path.to_path_buf(), canon.clone());
      });
      canon
    }
    Err(_) => path.to_path_buf(),
  }
}

impl ImportFileGuard {
  fn push_checked(path: &Path) -> Result<Self> {
    let canon = canonicalize_import_path_with_cache(path);
    IMPORT_FILE_STACK.with(|stack| -> Result<()> {
      let mut s = stack.borrow_mut();
      if s.iter().any(|p| p == &canon) {
        let mut chain = String::new();
        for (index, path) in s.iter().enumerate() {
          if index > 0 {
            chain.push_str(" -> ");
          }
          let _ = write!(&mut chain, "{}", path.display());
        }
        return Err(anyhow!("import cycle: {} -> {}", chain, canon.display()));
      }
      s.push(canon.clone());
      Ok(())
    })?;
    Ok(Self { canon })
  }

  fn canon(&self) -> &Path {
    &self.canon
  }
}

impl Drop for ImportFileGuard {
  fn drop(&mut self) {
    IMPORT_FILE_STACK.with(|stack| {
      stack.borrow_mut().pop();
    });
  }
}

fn push_unique_import_dependency(
  dependencies: &mut Vec<ImportDependencySnapshot>,
  dependency: &ImportDependencySnapshot,
) {
  if dependencies
    .iter()
    .any(|existing| existing.path == dependency.path)
  {
    return;
  }
  dependencies.push(dependency.clone());
}

fn record_import_dependencies_to_parent(dependencies: &[ImportDependencySnapshot]) {
  if dependencies.is_empty() {
    return;
  }
  IMPORT_DEPENDENCY_FRAMES.with(|frames| {
    let mut frames = frames.borrow_mut();
    let Some(parent) = frames.last_mut() else {
      return;
    };
    for dependency in dependencies {
      push_unique_import_dependency(&mut parent.dependencies, dependency);
    }
  });
}

impl ImportDependencyFrameGuard {
  fn push(self_dependency: ImportDependencySnapshot) -> Self {
    IMPORT_DEPENDENCY_FRAMES.with(|frames| {
      frames.borrow_mut().push(ImportDependencyFrame {
        dependencies: vec![self_dependency],
      });
    });
    Self { active: true }
  }

  fn finish(mut self) -> Vec<ImportDependencySnapshot> {
    self.active = false;
    IMPORT_DEPENDENCY_FRAMES.with(|frames| {
      frames
        .borrow_mut()
        .pop()
        .map(|frame| frame.dependencies)
        .unwrap_or_default()
    })
  }
}

impl Drop for ImportDependencyFrameGuard {
  fn drop(&mut self) {
    if self.active {
      IMPORT_DEPENDENCY_FRAMES.with(|frames| {
        frames.borrow_mut().pop();
      });
    }
  }
}

fn with_current_import_base<R>(f: impl FnOnce(Option<&PathBuf>) -> R) -> R {
  IMPORT_BASE_STACK.with(|stack| {
    let stack = stack.borrow();
    f(stack.last())
  })
}

// Pre-parsed AST registry. Populated once per thread by callers
// (e.g. pnixc-meta uses `init_preparsed_imports` to register
// build-time-parsed `runtime.px` / `evaluator.px` ASTs). When
// `eval_file_at_path` is called on a registered path, we skip the
// file read + parse and use the cached AST directly.
//
// Thread-local because `PnixExpr` contains `Arc<T>` (not `Sync`);
// pnixc-meta uses a single sized worker thread so per-thread
// registration matches the actual execution model.
//
// Path keys must be canonicalized (matches what callers passed to
// `eval_file_at_path` after `resolve_value_path`).
#[derive(Clone)]
struct PreparsedImportCacheEntry {
  source_len: u64,
  source_mtime_ns: Option<u128>,
  ast: Arc<pnix_core::lang::pnix::syntax::PnixExpr>,
}

fn preparsed_import_cache_insert(
  canon_path: PathBuf,
  source_len: u64,
  source_mtime_ns: Option<u128>,
  ast: Arc<pnix_core::lang::pnix::syntax::PnixExpr>,
) {
  PREPARSED_IMPORTS.with(|cell| {
    cell.borrow_mut().insert(
      canon_path,
      PreparsedImportCacheEntry {
        source_len,
        source_mtime_ns,
        ast,
      },
    );
  });
}

fn preparsed_import_cache_get(path: &Path, canon_path: &Path) -> Option<Arc<PnixExpr>> {
  let metadata = fs::metadata(path).ok()?;
  let source_len = metadata.len();
  let source_mtime_ns = metadata_mtime_ns(&metadata);
  PREPARSED_IMPORTS.with(|cell| {
    let cache = cell.borrow();
    let entry = cache.get(canon_path)?;
    if entry.source_len != source_len || entry.source_mtime_ns != source_mtime_ns {
      return None;
    }
    Some(entry.ast.clone())
  })
}

thread_local! {
  static PREPARSED_IMPORTS: RefCell<FxHashMap<PathBuf, PreparsedImportCacheEntry>> =
    RefCell::new(FxHashMap::default());
  static IMPORT_VALUE_CACHE: RefCell<FxHashMap<PathBuf, ImportValueCacheEntry>> =
    RefCell::new(fx_hashmap_with_capacity(IMPORT_VALUE_CACHE_MAX_ENTRIES));
  static IMPORT_DEPENDENCY_GRAPH_CACHE: RefCell<FxHashMap<PathBuf, ImportDependencyGraphEntry>> =
    RefCell::new(fx_hashmap_with_capacity(IMPORT_DEPENDENCY_GRAPH_CACHE_MAX_ENTRIES));
  // import-cycle 무한 재귀 가드 (2026-06-11): import_dependency_graph_build
  // 가 부르는 load_baked_expr_at_path 는 캐시 히트마다
  // touch_import_dependency_graph_cache → ensure_import_dependency_graph_cached
  // 로 재진입하는데, 그 재진입이 매번 빈 `building` 집합으로 시작하면
  // `.px` import 순환 (예: coding-candidate-dispatcher ↔
  // parametric-mirror-plate 렌즈 레지스트리) 에서 ensure → build → load →
  // touch → ensure 가 영원히 돈다. 이 집합은 "지금 이 스레드에서 그래프를
  // 빌드 중인 canon path" 를 ensure 호출 경계 너머로 공유해서 재진입을
  // 즉시 끊는다.
  static IMPORT_DEPENDENCY_GRAPH_BUILDING: RefCell<FxHashSet<PathBuf>> =
    RefCell::new(FxHashSet::default());
}

#[derive(Clone)]
struct ImportValueCacheEntry {
  source_len: u64,
  source_mtime_ns: u128,
  dependency_hash: String,
  dependencies: Vec<ImportDependencySnapshot>,
  value: Value,
}

pub fn init_preparsed_imports(
  map: std::collections::HashMap<PathBuf, pnix_core::lang::pnix::syntax::PnixExpr>,
) {
  PREPARSED_IMPORTS.with(|cell| {
    let mut imports = fx_hashmap_with_capacity(map.len());
    imports.extend(map.into_iter().map(|(path, expr)| {
      let (source_len, source_mtime_ns) = fs::metadata(&path)
        .ok()
        .map(|metadata| (metadata.len(), metadata_mtime_ns(&metadata)))
        .unwrap_or((0, None));
      (
        path,
        PreparsedImportCacheEntry {
          source_len,
          source_mtime_ns,
          ast: Arc::new(expr),
        },
      )
    }));
    *cell.borrow_mut() = imports;
  });
  IMPORT_VALUE_CACHE.with(|cell| {
    cell.borrow_mut().clear();
  });
  IMPORT_DEPENDENCY_GRAPH_CACHE.with(|cell| {
    cell.borrow_mut().clear();
  });
}

/// Path to the on-disk AST cache directory. Each cached AST lives at
/// `<dir>/<sha256-of-canonical-path>.ast.json` and embeds the source
/// file's mtime so we can detect edits and refuse stale caches.
///
/// Honest scope: this is the *parse-output* cache (PnixExpr JSON),
/// not the response cache. It eliminates the parser's wall-clock cost
/// across process restarts — utterance-decomposer.px alone is 277KB
/// of source and parses in seconds; language-syntax-profile.px is
/// 1.8MB and dominates cold boot.
///
/// Operator overrides via PNIX_EVAL_AST_CACHE_DIR. Default is
/// `$HOME/.cache/pnix-eval/ast/` or `/tmp/pnix-eval-ast/` if no HOME.
/// Returns None if neither resolves — in that case the cache is
/// transparent (in-memory only, like the pre-2026-05-28 behavior).
fn ast_cache_dir() -> Option<&'static PathBuf> {
  if !disk_ast_cache_enabled() {
    return None;
  }
  static AST_CACHE_DIR: OnceLock<Option<PathBuf>> = OnceLock::new();
  AST_CACHE_DIR
    .get_or_init(|| {
      if let Ok(dir) = std::env::var("PNIX_EVAL_AST_CACHE_DIR") {
        let p = PathBuf::from(dir);
        let _ = fs::create_dir_all(&p);
        return Some(p);
      }
      if let Some(home) = home_dir_os() {
        let p = PathBuf::from(home.as_os_str())
          .join(".cache")
          .join("pnix-eval")
          .join("ast");
        if fs::create_dir_all(&p).is_ok() {
          return Some(p);
        }
      }
      let p = PathBuf::from("/tmp").join("pnix-eval-ast");
      if fs::create_dir_all(&p).is_ok() {
        return Some(p);
      }
      None
    })
    .as_ref()
}

/// Compute the on-disk cache filename for a canonical .px path.
/// Uses a stable SHA-like hash of the path string so different
/// runs/instances writing the same source file land on the same
/// cache file. Sanity-bounded: result is always a valid filename.
fn ast_cache_file_for(canon_path: &Path) -> Option<PathBuf> {
  let dir = ast_cache_dir()?;
  let s = canon_path.to_string_lossy();
  // FNV-1a 64-bit hash — no crypto here, just identity. Same input
  // → same filename. Avoids pulling in sha2 just for naming.
  let mut h: u64 = 0xcbf29ce484222325;
  for b in s.as_bytes() {
    h ^= u64::from(*b);
    h = h.wrapping_mul(0x100000001b3);
  }
  let mut filename = String::with_capacity(25);
  push_u64_hex_lower_16(h, &mut filename);
  filename.push_str(".ast.json");
  Some(dir.join(filename))
}

fn ast_binary_cache_file_for(canon_path: &Path) -> Option<PathBuf> {
  let dir = ast_cache_dir()?;
  let s = canon_path.to_string_lossy();
  let mut h: u64 = 0xcbf29ce484222325;
  for b in s.as_bytes() {
    h ^= u64::from(*b);
    h = h.wrapping_mul(0x100000001b3);
  }
  let mut filename = String::with_capacity(24);
  push_u64_hex_lower_16(h, &mut filename);
  filename.push_str(".ast.bin");
  Some(dir.join(filename))
}

/// Try to load a cached AST for the given source file. Returns Some only if
/// the cache file exists, deserializes cleanly, AND the embedded source
/// identity matches the current source. The v1 cache identity is deliberately
/// stricter than the original mtime-only cache: source sha256, evaluator
/// version, and feature flag surface must match before we reuse the AST.
/// Any IO / parse error silently returns None (best-effort cache).
#[derive(serde::Deserialize)]
struct AstDiskCacheEntry {
  source_mtime_ns: AstDiskCacheMtime,
  source_len: Option<u64>,
  source_sha256: Option<String>,
  evaluator_version: Option<String>,
  feature_flags: Option<String>,
  expr: pnix_core::lang::pnix::syntax::PnixExpr,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct AstBinaryCacheEntry {
  source_mtime_ns: u128,
  source_len: u64,
  source_sha256: String,
  evaluator_version: String,
  feature_flags: String,
  expr: pnix_core::lang::pnix::syntax::PnixExpr,
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum AstDiskCacheMtime {
  String(String),
  Number(u64),
}

impl AstDiskCacheMtime {
  fn into_u128(self) -> Option<u128> {
    match self {
      AstDiskCacheMtime::String(value) => value.parse().ok(),
      AstDiskCacheMtime::Number(value) => Some(u128::from(value)),
    }
  }
}

fn ast_cache_evaluator_version() -> &'static str {
  concat!("pnix-eval@", env!("CARGO_PKG_VERSION"))
}

fn ast_cache_feature_flags() -> &'static str {
  "default-empty"
}

fn ast_cache_strict_sha_value_enabled(value: &str) -> bool {
  !matches!(
    value.trim(),
    "" | "0" | "false" | "FALSE" | "False" | "off" | "OFF" | "Off"
  )
}

fn ast_cache_strict_sha_enabled() -> bool {
  static STRICT_SHA: OnceLock<bool> = OnceLock::new();
  *STRICT_SHA.get_or_init(|| {
    std::env::var("PNIX_EVAL_AST_CACHE_STRICT_SHA")
      .map(|value| ast_cache_strict_sha_value_enabled(&value))
      .unwrap_or(false)
  })
}

fn source_sha256_hex(source: &str) -> String {
  hex_lower(&Sha256::digest(source.as_bytes()))
}

fn source_sha256_hex_prefix_32(source: &str) -> String {
  let digest = Sha256::digest(source.as_bytes());
  let mut out = String::with_capacity(32);
  push_hex_lower(&digest[..16], &mut out);
  out
}

fn hash_source_text(source: &str) -> String {
  let hash_started = std::time::Instant::now();
  let hash = source_sha256_hex(source);
  record_source_hash(hash_started.elapsed());
  hash
}

enum AstDiskCacheLookup {
  BinaryHit(pnix_core::lang::pnix::syntax::PnixExpr),
  JsonHit(pnix_core::lang::pnix::syntax::PnixExpr),
  Miss,
  Stale(AstDiskCacheStaleReason),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AstDiskCacheStaleReason {
  Mtime,
  Len,
  SourceSha256,
  EvaluatorVersion,
  FeatureFlags,
}

fn binary_cache_header_stale_reason(
  entry: &AstBinaryCacheEntry,
  source_len: u64,
  source_mtime_ns: u128,
) -> Option<AstDiskCacheStaleReason> {
  if entry.source_mtime_ns != source_mtime_ns {
    return Some(AstDiskCacheStaleReason::Mtime);
  }
  if entry.source_len != source_len {
    return Some(AstDiskCacheStaleReason::Len);
  }
  if entry.evaluator_version != ast_cache_evaluator_version() {
    return Some(AstDiskCacheStaleReason::EvaluatorVersion);
  }
  if entry.feature_flags != ast_cache_feature_flags() {
    return Some(AstDiskCacheStaleReason::FeatureFlags);
  }
  None
}

#[cfg(test)]
fn binary_cache_identity_matches(
  entry: &AstBinaryCacheEntry,
  source_len: u64,
  source_mtime_ns: u128,
  source_sha256: &str,
) -> bool {
  binary_cache_identity_stale_reason(entry, source_len, source_mtime_ns, source_sha256).is_none()
}

fn binary_cache_identity_stale_reason(
  entry: &AstBinaryCacheEntry,
  source_len: u64,
  source_mtime_ns: u128,
  source_sha256: &str,
) -> Option<AstDiskCacheStaleReason> {
  if let Some(reason) = binary_cache_header_stale_reason(entry, source_len, source_mtime_ns) {
    return Some(reason);
  }
  if entry.source_sha256 != source_sha256 {
    return Some(AstDiskCacheStaleReason::SourceSha256);
  }
  None
}

fn read_binary_ast_cache_entry(canon_path: &Path) -> Option<AstBinaryCacheEntry> {
  let Some(cache_file) = ast_binary_cache_file_for(canon_path) else {
    return None;
  };
  let Ok(raw) = fs::read(&cache_file) else {
    return None;
  };
  let Ok(entry) = serde_json::from_slice::<AstBinaryCacheEntry>(&raw) else {
    return None;
  };
  Some(entry)
}

fn try_load_binary_ast_from_disk_fast(
  canon_path: &Path,
  source_len: u64,
  source_mtime_ns: Option<u128>,
) -> AstDiskCacheLookup {
  let Some(entry) = read_binary_ast_cache_entry(canon_path) else {
    return AstDiskCacheLookup::Miss;
  };
  let Some(source_mtime_ns) = source_mtime_ns else {
    return AstDiskCacheLookup::Miss;
  };
  if let Some(reason) = binary_cache_header_stale_reason(&entry, source_len, source_mtime_ns) {
    return AstDiskCacheLookup::Stale(reason);
  }
  AstDiskCacheLookup::BinaryHit(entry.expr)
}

fn try_load_binary_ast_from_disk(
  canon_path: &Path,
  source_len: u64,
  source_mtime_ns: Option<u128>,
  source_sha256: &str,
) -> AstDiskCacheLookup {
  let Some(entry) = read_binary_ast_cache_entry(canon_path) else {
    return AstDiskCacheLookup::Miss;
  };
  let Some(source_mtime_ns) = source_mtime_ns else {
    return AstDiskCacheLookup::Miss;
  };
  if let Some(reason) =
    binary_cache_identity_stale_reason(&entry, source_len, source_mtime_ns, source_sha256)
  {
    return AstDiskCacheLookup::Stale(reason);
  }
  AstDiskCacheLookup::BinaryHit(entry.expr)
}

fn try_load_ast_from_disk(
  canon_path: &Path,
  source_len: u64,
  source_mtime_ns: Option<u128>,
  source_sha256: &str,
) -> AstDiskCacheLookup {
  match try_load_binary_ast_from_disk(canon_path, source_len, source_mtime_ns, source_sha256) {
    AstDiskCacheLookup::BinaryHit(expr) => return AstDiskCacheLookup::BinaryHit(expr),
    AstDiskCacheLookup::Stale(reason) => return AstDiskCacheLookup::Stale(reason),
    AstDiskCacheLookup::JsonHit(_) => unreachable!("binary loader cannot return JSON hits"),
    AstDiskCacheLookup::Miss => {}
  }
  let Some(cache_file) = ast_cache_file_for(canon_path) else {
    return AstDiskCacheLookup::Miss;
  };
  let Ok(raw) = fs::read_to_string(&cache_file) else {
    return AstDiskCacheLookup::Miss;
  };
  let Ok(entry) = serde_json::from_str::<AstDiskCacheEntry>(&raw) else {
    return AstDiskCacheLookup::Miss;
  };
  let Some(recorded_mtime) = entry.source_mtime_ns.into_u128() else {
    return AstDiskCacheLookup::Miss;
  };
  let Some(source_mtime_ns) = source_mtime_ns else {
    return AstDiskCacheLookup::Miss;
  };
  if recorded_mtime != source_mtime_ns {
    return AstDiskCacheLookup::Stale(AstDiskCacheStaleReason::Mtime);
  }
  if entry.source_len != Some(source_len) {
    return AstDiskCacheLookup::Stale(AstDiskCacheStaleReason::Len);
  }
  if entry.source_sha256.as_deref() != Some(source_sha256) {
    return AstDiskCacheLookup::Stale(AstDiskCacheStaleReason::SourceSha256);
  }
  if entry.evaluator_version.as_deref() != Some(ast_cache_evaluator_version()) {
    return AstDiskCacheLookup::Stale(AstDiskCacheStaleReason::EvaluatorVersion);
  }
  if entry.feature_flags.as_deref() != Some(ast_cache_feature_flags()) {
    return AstDiskCacheLookup::Stale(AstDiskCacheStaleReason::FeatureFlags);
  }
  AstDiskCacheLookup::JsonHit(entry.expr)
}

/// Write the baked AST to disk for the next boot. Best-effort —
/// any IO error silently drops the write (cache stays in-memory).
fn save_ast_to_disk(
  canon_path: &Path,
  source_len: u64,
  source_mtime_ns: Option<u128>,
  source_sha256: &str,
  expr: &pnix_core::lang::pnix::syntax::PnixExpr,
) {
  let Some(cache_file) = ast_cache_file_for(canon_path) else {
    return;
  };
  let Some(source_mtime_ns) = source_mtime_ns else {
    return;
  };
  if let Some(binary_cache_file) = ast_binary_cache_file_for(canon_path) {
    let binary_entry = AstBinaryCacheEntry {
      source_mtime_ns,
      source_len,
      source_sha256: source_sha256.to_string(),
      evaluator_version: ast_cache_evaluator_version().to_string(),
      feature_flags: ast_cache_feature_flags().to_string(),
      expr: expr.clone(),
    };
    if let Ok(bytes) = serde_json::to_vec(&binary_entry) {
      let _ = fs::write(binary_cache_file, bytes);
    }
  }
  // Store mtime as a string to avoid JSON 53-bit integer truncation;
  // u128 can exceed JSON's safe-integer range.
  let payload = serde_json::json!({
    "source_mtime_ns": source_mtime_ns.to_string(),
    "source_len": source_len,
    "source_sha256": source_sha256,
    "evaluator_version": ast_cache_evaluator_version(),
    "feature_flags": ast_cache_feature_flags(),
    "source_path": canon_path.to_string_lossy(),
    "expr": expr,
  });
  let Ok(s) = serde_json::to_string(&payload) else {
    return;
  };
  let _ = fs::write(&cache_file, s);
}

fn metadata_mtime_ns(metadata: &fs::Metadata) -> Option<u128> {
  metadata
    .modified()
    .ok()
    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
    .map(|d| d.as_nanos())
}

fn read_file_text_with_cache(path: &Path, metadata: &fs::Metadata) -> Result<String> {
  let len = metadata.len();
  let mtime_ns = metadata_mtime_ns(metadata);
  let cache_started = cache_lookup_timing_started();
  let cached = READ_FILE_CACHE.with(|cache| {
    cache
      .borrow()
      .get(path)
      .filter(|entry| entry.len == len && entry.mtime_ns == mtime_ns)
      .map(|entry| entry.content.clone())
  });
  record_cache_lookup_elapsed(cache_started);
  if let Some(content) = cached {
    record_read_file_cache_hit();
    return Ok(content);
  }
  record_read_file_cache_miss();

  let read_started = std::time::Instant::now();
  let content = match fs::read_to_string(path) {
    Ok(content) => content,
    Err(e) if e.kind() == std::io::ErrorKind::InvalidData => {
      return Err(anyhow!(
        "builtins.readFile: '{}' is not a valid UTF-8 text file (binary file or invalid encoding)",
        path.display()
      ))
    }
    Err(e) => {
      return Err(anyhow!(
        "builtins.readFile: failed to read '{}': {}",
        path.display(),
        e
      ))
    }
  };
  record_file_read(content.len(), read_started.elapsed());
  let content = match content.strip_prefix('\u{FEFF}') {
    Some(stripped) => stripped.to_string(),
    None => content,
  };

  if len <= READFILE_CACHE_MAX_BYTES {
    let entry = ReadFileCacheEntry {
      len,
      mtime_ns,
      content: content.clone(),
    };
    READ_FILE_CACHE.with(|cache| {
      let mut cache = cache.borrow_mut();
      evict_one_cache_entry(&mut cache, READFILE_CACHE_MAX_ENTRIES);
      cache.insert(path.to_path_buf(), entry);
    });
  }
  Ok(content)
}

fn file_type_to_nix_string(file_type: fs::FileType) -> &'static str {
  if file_type.is_dir() {
    "directory"
  } else if file_type.is_file() {
    "regular"
  } else if file_type.is_symlink() {
    "symlink"
  } else {
    "unknown"
  }
}

fn compile_regex_with_cache(pattern: &str, builtin_name: &str) -> Result<Regex> {
  if pattern.len() <= REGEX_CACHE_MAX_PATTERN_BYTES {
    let cache_started = cache_lookup_timing_started();
    let cached = REGEX_CACHE.with(|cache| cache.borrow().get(pattern).cloned());
    record_cache_lookup_elapsed(cache_started);
    if let Some(re) = cached {
      record_regex_cache_hit();
      return Ok(re);
    }
  }
  record_regex_cache_miss();

  let re = RegexBuilder::new(pattern)
    .size_limit(REGEX_SIZE_LIMIT_BYTES)
    .dfa_size_limit(REGEX_DFA_SIZE_LIMIT_BYTES)
    .build()
    .map_err(|e| anyhow!("{builtin_name}: invalid regex: {}", e))?;

  if pattern.len() <= REGEX_CACHE_MAX_PATTERN_BYTES {
    REGEX_CACHE.with(|cache| {
      let mut cache = cache.borrow_mut();
      evict_one_cache_entry(&mut cache, REGEX_CACHE_MAX_ENTRIES);
      cache.insert(pattern.to_string(), re.clone());
    });
  }
  Ok(re)
}

fn read_dir_entries_with_cache(path: &Path) -> Result<BTreeMap<String, String>> {
  let metadata = fs::metadata(path).map_err(|e| {
    anyhow!(
      "builtins.readDir: failed to read '{}': {}",
      path.display(),
      e
    )
  })?;
  let len = metadata.len();
  let mtime_ns = metadata_mtime_ns(&metadata);
  let cache_started = cache_lookup_timing_started();
  let cached = READ_DIR_CACHE.with(|cache| {
    cache
      .borrow()
      .get(path)
      .filter(|entry| entry.len == len && entry.mtime_ns == mtime_ns)
      .map(|entry| entry.entries.clone())
  });
  record_cache_lookup_elapsed(cache_started);
  if let Some(entries) = cached {
    record_read_dir_cache_hit();
    return Ok(entries);
  }
  record_read_dir_cache_miss();

  let read_started = std::time::Instant::now();
  let entries = fs::read_dir(path).map_err(|e| {
    anyhow!(
      "builtins.readDir: failed to read '{}': {}",
      path.display(),
      e
    )
  })?;
  let mut result = BTreeMap::new();
  for entry in entries {
    let entry = entry.map_err(|e| anyhow!("builtins.readDir: error reading entry: {}", e))?;
    let file_type = entry
      .file_type()
      .map_err(|e| anyhow!("builtins.readDir: error getting file type: {}", e))?;
    result.insert(
      entry.file_name().to_string_lossy().into_owned(),
      file_type_to_nix_string(file_type).to_string(),
    );
  }
  record_eval_perf(|stats| {
    stats.io_ms += read_started.elapsed().as_millis();
  });

  let entry = ReadDirCacheEntry {
    len,
    mtime_ns,
    entries: result.clone(),
  };
  READ_DIR_CACHE.with(|cache| {
    let mut cache = cache.borrow_mut();
    evict_one_cache_entry(&mut cache, READDIR_CACHE_MAX_ENTRIES);
    cache.insert(path.to_path_buf(), entry);
  });
  Ok(result)
}

fn touch_import_dependency_graph_cache(
  path: &Path,
  canon_path: &Path,
  expr: &pnix_core::lang::pnix::syntax::PnixExpr,
) {
  let Ok(metadata) = fs::metadata(path) else {
    return;
  };
  let source_len = metadata.len();
  let source_mtime_ns = metadata_mtime_ns(&metadata);
  let _ = ensure_import_dependency_graph_cached(canon_path, source_len, source_mtime_ns, expr);
}

fn load_baked_expr_at_path(
  path: &Path,
  canon_path: &Path,
) -> Result<Arc<pnix_core::lang::pnix::syntax::PnixExpr>> {
  // Fast path: if this path was pre-parsed at build time (pnixc-meta
  // registers runtime.px / evaluator.px), skip read + parse entirely.
  // S11 perf slice (2026-05-21): cache key is the canonicalized form
  // computed inside push_checked above. Callers passing the same file
  // via different path forms (e.g. `./foo.px` vs the symlink-resolved
  // absolute) now hit the same entry. Previously the lookup used the
  // raw `path` and missed.
  let cache_started = cache_lookup_timing_started();
  let preparsed = preparsed_import_cache_get(path, canon_path);
  record_cache_lookup_elapsed(cache_started);
  if let Some(ast) = preparsed {
    record_preparsed_ast_hit();
    touch_import_dependency_graph_cache(path, canon_path, ast.as_ref());
    return Ok(ast);
  }
  // Disk cache (2026-05-28): per-source-file AST cache that
  // survives process restarts. Cuts cold boot wall-clock for
  // doghouse-http worker thread from ~30s to <1s on warm disk.
  // Skipped if the current source hash, evaluator version, or feature
  // flag surface differs from the recorded cache entry.
  let source_metadata =
    fs::metadata(path).map_err(|e| anyhow!("metadata {}: {}", path.display(), e))?;
  let source_len = source_metadata.len();
  let source_mtime_ns = metadata_mtime_ns(&source_metadata);
  let strict_sha = ast_cache_strict_sha_enabled();
  let mut disk_cache_known_stale = None;
  if !strict_sha {
    let disk_cache_started = cache_lookup_timing_started();
    let fast_cached = try_load_binary_ast_from_disk_fast(canon_path, source_len, source_mtime_ns);
    record_cache_lookup_elapsed(disk_cache_started);
    match fast_cached {
      AstDiskCacheLookup::BinaryHit(cached) => {
        record_disk_ast_binary_cache_hit();
        record_ast_cache_fast_header_hit();
        record_source_read_skipped_by_ast_cache_hit();
        let cached = Arc::new(cached);
        preparsed_import_cache_insert(
          canon_path.to_path_buf(),
          source_len,
          source_mtime_ns,
          cached.clone(),
        );
        touch_import_dependency_graph_cache(path, canon_path, cached.as_ref());
        return Ok(cached);
      }
      AstDiskCacheLookup::Stale(reason) => {
        disk_cache_known_stale = Some(reason);
      }
      AstDiskCacheLookup::JsonHit(_) => {
        unreachable!("binary fast loader cannot return JSON hits")
      }
      AstDiskCacheLookup::Miss => {}
    }
  }
  let read_started = std::time::Instant::now();
  let source = fs::read_to_string(path).map_err(|e| anyhow!("read {}: {}", path.display(), e))?;
  record_file_read(source.len(), read_started.elapsed());
  let source_sha256 = hash_source_text(&source);
  if strict_sha {
    record_ast_cache_strict_sha_revalidated();
  }

  let disk_cache_started = cache_lookup_timing_started();
  let disk_cached = if let Some(reason) = disk_cache_known_stale {
    AstDiskCacheLookup::Stale(reason)
  } else {
    try_load_ast_from_disk(canon_path, source_len, source_mtime_ns, &source_sha256)
  };
  record_cache_lookup_elapsed(disk_cache_started);
  match disk_cached {
    AstDiskCacheLookup::BinaryHit(cached) => {
      record_disk_ast_binary_cache_hit();
      let cached = Arc::new(cached);
      preparsed_import_cache_insert(
        canon_path.to_path_buf(),
        source_len,
        source_mtime_ns,
        cached.clone(),
      );
      touch_import_dependency_graph_cache(path, canon_path, cached.as_ref());
      Ok(cached)
    }
    AstDiskCacheLookup::JsonHit(cached) => {
      record_disk_ast_json_cache_hit();
      save_ast_to_disk(
        canon_path,
        source_len,
        source_mtime_ns,
        &source_sha256,
        &cached,
      );
      let cached = Arc::new(cached);
      preparsed_import_cache_insert(
        canon_path.to_path_buf(),
        source_len,
        source_mtime_ns,
        cached.clone(),
      );
      touch_import_dependency_graph_cache(path, canon_path, cached.as_ref());
      Ok(cached)
    }
    AstDiskCacheLookup::Stale(reason) => {
      record_disk_ast_cache_stale(reason);
      let parse_started = std::time::Instant::now();
      let parsed = pnix_core::lang::pnix::parse_expr(&source)
        .map_err(|e| anyhow!("parse {}: {}", path.display(), e))?;
      record_file_parse(source.len(), parse_started.elapsed());
      let normalize_started = std::time::Instant::now();
      let baked = match path.parent() {
        Some(parent) => bake_relative_paths_in_expr(parsed, parent),
        None => parsed,
      };
      record_normalize(normalize_started.elapsed());
      save_ast_to_disk(
        canon_path,
        source_len,
        source_mtime_ns,
        &source_sha256,
        &baked,
      );
      let baked = Arc::new(baked);
      preparsed_import_cache_insert(
        canon_path.to_path_buf(),
        source_len,
        source_mtime_ns,
        baked.clone(),
      );
      touch_import_dependency_graph_cache(path, canon_path, baked.as_ref());
      Ok(baked)
    }
    AstDiskCacheLookup::Miss => {
      record_ast_cache_miss();
      let parse_started = std::time::Instant::now();
      let parsed = pnix_core::lang::pnix::parse_expr(&source)
        .map_err(|e| anyhow!("parse {}: {}", path.display(), e))?;
      record_file_parse(source.len(), parse_started.elapsed());
      // Bake every literal relative path (`PnixPath::Relative(_)`) in the
      // AST to its absolute form using the file's parent dir. Without this,
      // a `let x = import ./y.px; ...` whose `x` is only referenced inside
      // a lambda body would force the `import` call long after the
      // `ImportBaseGuard` for this file has popped — `current_import_base()`
      // would return `None` (or the wrong base), and `./y.px` would
      // erroneously resolve against cwd. Baking at load time fixes the
      // closure-captured-relative-import bug structurally.
      let normalize_started = std::time::Instant::now();
      let baked = match path.parent() {
        Some(parent) => bake_relative_paths_in_expr(parsed, parent),
        None => parsed,
      };
      record_normalize(normalize_started.elapsed());
      // Save to disk cache so next boot can skip the parse step.
      save_ast_to_disk(
        canon_path,
        source_len,
        source_mtime_ns,
        &source_sha256,
        &baked,
      );
      // PERF (2026-05-28): also insert the baked AST into the thread-
      // local cache so subsequent imports of the same path skip the
      // file-read + parse + bake steps. Without this, a long-running
      // host (e.g. doghouse-http warm worker thread) re-parses every
      // .px on every request — measured at ~30s per HTTP request for
      // the agent plate set. After this insert, repeated imports are
      // near-instant.
      //
      // Hot-reload note: thread-local AST cache keys include source
      // len/mtime so an on-disk edit invalidates the preparsed fast path
      // without requiring a process restart. Disk cache still validates
      // source hash/version/feature flags across boots.
      let baked = Arc::new(baked);
      preparsed_import_cache_insert(
        canon_path.to_path_buf(),
        source_len,
        source_mtime_ns,
        baked.clone(),
      );
      touch_import_dependency_graph_cache(path, canon_path, baked.as_ref());
      Ok(baked)
    }
  }
}

fn push_import_file_guard_and_record(path: &Path) -> Result<ImportFileGuard> {
  let import_started = std::time::Instant::now();
  let file_guard = ImportFileGuard::push_checked(path)?;
  record_eval_perf(|stats| {
    stats.import_resolve_ms += import_started.elapsed().as_millis();
    stats.import_count += 1;
  });
  Ok(file_guard)
}

fn literal_import_path_from_expr(expr: &PnixExpr) -> Option<PathBuf> {
  use pnix_core::lang::pnix::syntax::{PnixExpr as E, PnixPath};
  match expr {
    E::Path(PnixPath::Absolute(s) | PnixPath::Relative(s)) => Some(PathBuf::from(s.as_str())),
    E::String(s) if !s.is_empty() => Some(PathBuf::from(s.as_str())),
    _ => None,
  }
}

fn collect_literal_import_paths_from_attr_item(
  item: &pnix_core::lang::pnix::syntax::PnixAttrItem,
  out: &mut Vec<PathBuf>,
) {
  match item {
    PnixAttrItem::Assign { value, .. } => {
      collect_literal_import_paths_from_expr(value.as_ref(), out);
    }
    PnixAttrItem::DynamicAssign {
      key_path, value, ..
    } => {
      for seg in key_path {
        if let pnix_core::lang::pnix::syntax::AttrKeySegment::Dynamic(expr) = seg {
          collect_literal_import_paths_from_expr(expr.as_ref(), out);
        }
      }
      collect_literal_import_paths_from_expr(value.as_ref(), out);
    }
    PnixAttrItem::Inherit { from, .. } => {
      if let Some(expr) = from {
        collect_literal_import_paths_from_expr(expr.as_ref(), out);
      }
    }
  }
}

fn collect_literal_import_paths_from_expr(expr: &PnixExpr, out: &mut Vec<PathBuf>) {
  use pnix_core::lang::pnix::syntax::{PnixExpr as E, PnixLetBinding, StringInterpPart};
  match expr {
    E::Import { path } => {
      if let Some(import_path) = literal_import_path_from_expr(path.as_ref()) {
        out.push(import_path);
      }
    }
    E::StringInterp(parts) => {
      for part in parts {
        if let StringInterpPart::Expr(nested) = part {
          collect_literal_import_paths_from_expr(nested.as_ref(), out);
        }
      }
    }
    E::Let { bindings, body } => {
      for binding in bindings {
        match binding {
          PnixLetBinding::Binding { value, .. } => {
            collect_literal_import_paths_from_expr(value.as_ref(), out);
          }
          PnixLetBinding::Inherit { from, .. } => {
            if let Some(expr) = from {
              collect_literal_import_paths_from_expr(expr.as_ref(), out);
            }
          }
        }
      }
      collect_literal_import_paths_from_expr(body, out);
    }
    E::If { cond, then_, else_ } => {
      collect_literal_import_paths_from_expr(cond, out);
      collect_literal_import_paths_from_expr(then_, out);
      collect_literal_import_paths_from_expr(else_, out);
    }
    E::Lambda { body, .. } => collect_literal_import_paths_from_expr(body, out),
    E::Apply { func, arg } => {
      collect_literal_import_paths_from_expr(func, out);
      collect_literal_import_paths_from_expr(arg, out);
    }
    E::AttrSet { items, .. } => {
      for item in items {
        collect_literal_import_paths_from_attr_item(item, out);
      }
    }
    E::List(items) | E::Construct { args: items, .. } => {
      for item in items {
        collect_literal_import_paths_from_expr(item, out);
      }
    }
    E::Select { base, .. } | E::HasAttr { base, .. } => {
      collect_literal_import_paths_from_expr(base, out);
    }
    E::SelectOrDefault { base, default, .. } => {
      collect_literal_import_paths_from_expr(base, out);
      collect_literal_import_paths_from_expr(default, out);
    }
    E::Unary { arg, .. } => collect_literal_import_paths_from_expr(arg, out),
    E::Binary { lhs, rhs, .. } => {
      collect_literal_import_paths_from_expr(lhs, out);
      collect_literal_import_paths_from_expr(rhs, out);
    }
    E::Match { scrutinee, arms } => {
      collect_literal_import_paths_from_expr(scrutinee, out);
      for arm in arms {
        if let Some(guard) = &arm.guard {
          collect_literal_import_paths_from_expr(guard.as_ref(), out);
        }
        collect_literal_import_paths_from_expr(&arm.body, out);
      }
    }
    E::With { env, body } => {
      collect_literal_import_paths_from_expr(env, out);
      collect_literal_import_paths_from_expr(body, out);
    }
    E::Assert { cond, body } => {
      collect_literal_import_paths_from_expr(cond, out);
      collect_literal_import_paths_from_expr(body, out);
    }
    E::DynamicHasAttr { base, attr_expr } => {
      collect_literal_import_paths_from_expr(base, out);
      collect_literal_import_paths_from_expr(attr_expr, out);
    }
    E::DynamicSelect { base, attr_expr } => {
      collect_literal_import_paths_from_expr(base, out);
      collect_literal_import_paths_from_expr(attr_expr, out);
    }
    E::DynamicSelectOrDefault {
      base,
      attr_expr,
      default,
    } => {
      collect_literal_import_paths_from_expr(base, out);
      collect_literal_import_paths_from_expr(attr_expr, out);
      collect_literal_import_paths_from_expr(default, out);
    }
    E::Index { base, index } => {
      collect_literal_import_paths_from_expr(base, out);
      collect_literal_import_paths_from_expr(index, out);
    }
    E::Int(_) | E::Float(_) | E::Bool(_) | E::Null | E::String(_) | E::Var(_) | E::Path(_) => {}
  }
}

fn import_graph_canonical_path(path: &Path) -> PathBuf {
  // D-1 (2026-06-12): route through the existing canonical-path cache
  // instead of a raw `fs::canonicalize`. This sits on the per-import
  // dependency-snapshot path, so the daemon/serve pool paid a FULL
  // canonicalize (one `getattrlist` syscall per path component) per
  // transitive import per query per worker — the W=8 profile showed
  // getattrlist as 10498 leaf samples vs eval_machine's 946, with the
  // workers serializing on kernel inode locks. Freshness semantics are
  // unchanged: the mtime/len/sha validation still runs on the canon
  // path every time; only the path-resolution step is memoized (same
  // contract as the import-file-guard call site above).
  let normalized = normalize_pnix_path(path);
  canonicalize_import_path_with_cache(&normalized)
}

/// D-2 (2026-06-12): per-eval memo for import freshness stats. Import
/// CACHE HITS re-stat every transitive dependency on every import
/// expression (~hundreds of `fs::metadata` of the same few dozen files
/// per query) — post-D-1 the W=8 daemon profile still showed
/// `stat$INODE64` at 1212 leaf samples. Within one top-level
/// evaluation a consistent metadata snapshot is at least as sound as
/// racing the filesystem mid-eval, so the memo answers repeats. OFF by
/// default: long-lived test/host processes that REWRITE .px files
/// between evals rely on fresh stats. Hosts opt in
/// ([`enable_import_stat_memo`]) and mark eval boundaries
/// ([`clear_import_stat_memo`] — the daemon worker calls it before
/// each query, so cross-query edit detection is unchanged; one-shot is
/// a single eval and never needs the clear).
static IMPORT_STAT_MEMO_ENABLED: std::sync::atomic::AtomicBool =
  std::sync::atomic::AtomicBool::new(false);

/// Opt-in serve-mode freshness policy (default OFF = today's per-job
/// clear). When set, [`clear_import_stat_memo`] becomes a no-op so the
/// memo stays warm across eval boundaries — eliminating the per-request
/// re-stat of the whole import tree (measured 2026-06-13 as the single
/// biggest on-CPU cost of the conjecture chat lane, larger than the eval
/// itself: ~570 transitive modules `fs::metadata`'d on every request).
/// The tradeoff is the same shape as the WM background-flush knob: an
/// operator who edits `.px` files in a live serve process will not see
/// the change until restart. Long-lived dev loops that rewrite `.px`
/// between requests keep the default (clear ON).
static IMPORT_STAT_MEMO_PERSIST: std::sync::atomic::AtomicBool =
  std::sync::atomic::AtomicBool::new(false);

thread_local! {
  static IMPORT_STAT_MEMO: RefCell<FxHashMap<PathBuf, Option<(u64, Option<u128>)>>> =
    RefCell::new(FxHashMap::default());
}

/// Opt in to per-eval import-stat memoization (see
/// `IMPORT_STAT_MEMO_ENABLED`). Process-wide, one-way.
pub fn enable_import_stat_memo() {
  IMPORT_STAT_MEMO_ENABLED.store(true, std::sync::atomic::Ordering::Relaxed);
}

/// Keep the import-stat memo warm across eval boundaries (serve-mode
/// freshness opt-in). Process-wide; see [`IMPORT_STAT_MEMO_PERSIST`].
pub fn set_import_stat_memo_persistent(persist: bool) {
  IMPORT_STAT_MEMO_PERSIST.store(persist, std::sync::atomic::Ordering::Relaxed);
}

/// Drop the calling thread's import-stat memo. Hosts call this at each
/// evaluation boundary (e.g. per daemon query) so file edits between
/// evaluations are observed exactly as before the memo existed. When
/// [`set_import_stat_memo_persistent`] is on, this is a no-op (the memo
/// stays warm — see that flag's doc for the tradeoff).
pub fn clear_import_stat_memo() {
  if IMPORT_STAT_MEMO_PERSIST.load(std::sync::atomic::Ordering::Relaxed) {
    return;
  }
  IMPORT_STAT_MEMO.with(|memo| memo.borrow_mut().clear());
}

/// `fs::metadata(path)` reduced to the freshness pair `(len, mtime)`,
/// answered from the per-eval memo when enabled. `None` = metadata
/// error (missing file) — memoized too, so a path that vanished stays
/// vanished for the rest of this evaluation.
fn import_stat_len_mtime(path: &Path) -> Option<(u64, Option<u128>)> {
  if !IMPORT_STAT_MEMO_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
    let metadata = fs::metadata(path).ok()?;
    return Some((metadata.len(), metadata_mtime_ns(&metadata)));
  }
  if let Some(cached) = IMPORT_STAT_MEMO.with(|memo| memo.borrow().get(path).cloned()) {
    return cached;
  }
  let fresh = fs::metadata(path)
    .ok()
    .map(|metadata| (metadata.len(), metadata_mtime_ns(&metadata)));
  IMPORT_STAT_MEMO.with(|memo| {
    memo.borrow_mut().insert(path.to_path_buf(), fresh);
  });
  fresh
}

fn import_dependency_snapshot_for_path(path: &Path) -> Result<(PathBuf, u64, Option<u128>)> {
  let canon = import_graph_canonical_path(path);
  let (len, mtime_ns) = import_stat_len_mtime(&canon)
    .ok_or_else(|| anyhow!("metadata {}: not found or unreadable", canon.display()))?;
  Ok((canon, len, mtime_ns))
}

fn compute_import_dependency_hash(
  self_path: &Path,
  self_len: u64,
  self_mtime_ns: u128,
  transitive_paths: &[PathBuf],
) -> Result<String> {
  let mut hasher = Sha256::new();
  hasher.update(b"self:");
  hasher.update(self_path.to_string_lossy().as_bytes());
  hasher.update(b":");
  let mut self_len_hex = String::new();
  push_u64_hex_lower_16(self_len, &mut self_len_hex);
  hasher.update(self_len_hex.as_bytes());
  hasher.update(b":");
  hasher.update(self_mtime_ns.to_string().as_bytes());

  let mut dep_entries = Vec::new();
  for dep_path in transitive_paths {
    let (canon, len, mtime_ns) = import_dependency_snapshot_for_path(dep_path)?;
    dep_entries.push((canon, len, mtime_ns));
  }
  dep_entries.sort_by(|a, b| a.0.cmp(&b.0));

  for (path, len, mtime_ns) in dep_entries {
    hasher.update(b"dep:");
    hasher.update(path.to_string_lossy().as_bytes());
    hasher.update(b":");
    let mut len_hex = String::new();
    push_u64_hex_lower_16(len, &mut len_hex);
    hasher.update(len_hex.as_bytes());
    hasher.update(b":");
    if let Some(mtime_ns) = mtime_ns {
      hasher.update(mtime_ns.to_string().as_bytes());
    } else {
      hasher.update(b"missing-mtime");
    }
  }

  Ok(hex_lower(&hasher.finalize()))
}

fn import_dependency_graph_hash_is_current(
  canon_path: &Path,
  entry: &ImportDependencyGraphEntry,
) -> bool {
  let Ok(current_hash) = compute_import_dependency_hash(
    canon_path,
    entry.source_len,
    entry.source_mtime_ns,
    &entry.transitive_imports,
  ) else {
    return false;
  };
  current_hash == entry.dependency_hash
}

fn import_dependency_graph_lookup(
  canon_path: &Path,
  source_len: u64,
  source_mtime_ns: u128,
) -> Option<ImportDependencyGraphEntry> {
  let cache_started = cache_lookup_timing_started();
  let cached = IMPORT_DEPENDENCY_GRAPH_CACHE.with(|cache| {
    cache
      .borrow()
      .get(canon_path)
      .filter(|entry| entry.source_len == source_len && entry.source_mtime_ns == source_mtime_ns)
      .cloned()
  });
  record_cache_lookup_elapsed(cache_started);
  let Some(cached) = cached else {
    record_dependency_graph_cache_miss();
    return None;
  };
  if !import_dependency_graph_hash_is_current(canon_path, &cached) {
    record_dependency_graph_cache_miss();
    return None;
  }
  record_dependency_graph_cache_hit();
  Some(cached)
}

fn import_dependency_graph_store(canon_path: &Path, entry: ImportDependencyGraphEntry) {
  IMPORT_DEPENDENCY_GRAPH_CACHE.with(|cache| {
    let mut cache = cache.borrow_mut();
    evict_one_cache_entry(&mut cache, IMPORT_DEPENDENCY_GRAPH_CACHE_MAX_ENTRIES);
    cache.insert(canon_path.to_path_buf(), entry);
  });
  record_dependency_graph_cache_store();
}

fn import_dependency_graph_build(
  canon_path: &Path,
  source_len: u64,
  source_mtime_ns: u128,
  expr: &PnixExpr,
  building: &mut FxHashSet<PathBuf>,
  reentrant_edges: &mut Vec<String>,
) -> Result<ImportDependencyGraphEntry> {
  let mut direct_raw = Vec::new();
  collect_literal_import_paths_from_expr(expr, &mut direct_raw);
  let mut direct_imports = Vec::new();
  for import_path in direct_raw {
    let canon = import_graph_canonical_path(&import_path);
    if !direct_imports.iter().any(|existing| existing == &canon) {
      direct_imports.push(canon);
    }
  }
  direct_imports.sort();

  let mut transitive = BTreeSet::new();
  for dep_path in &direct_imports {
    if building.contains(dep_path) {
      reentrant_edges.push(format!(
        "{} -> {}",
        canon_path.display(),
        dep_path.display()
      ));
      continue;
    }
    building.insert(dep_path.clone());
    transitive.insert(dep_path.clone());

    let dep_metadata =
      fs::metadata(dep_path).map_err(|e| anyhow!("metadata {}: {}", dep_path.display(), e))?;
    let dep_len = dep_metadata.len();
    let Some(dep_mtime_ns) = metadata_mtime_ns(&dep_metadata) else {
      building.remove(dep_path);
      continue;
    };

    if let Some(child_graph) = import_dependency_graph_lookup(dep_path, dep_len, dep_mtime_ns) {
      transitive.extend(child_graph.transitive_imports.iter().cloned());
    } else {
      let dep_expr = load_baked_expr_at_path(dep_path, dep_path)?;
      let child_graph =
        if let Some(cached) = import_dependency_graph_lookup(dep_path, dep_len, dep_mtime_ns) {
          cached
        } else {
          let built = import_dependency_graph_build(
            dep_path,
            dep_len,
            dep_mtime_ns,
            dep_expr.as_ref(),
            building,
            reentrant_edges,
          )?;
          import_dependency_graph_store(dep_path, built.clone());
          built
        };
      transitive.extend(child_graph.transitive_imports.iter().cloned());
    }
    building.remove(dep_path);
  }

  let transitive_imports: Vec<PathBuf> = transitive.into_iter().collect();
  let dependency_hash =
    compute_import_dependency_hash(canon_path, source_len, source_mtime_ns, &transitive_imports)?;
  Ok(ImportDependencyGraphEntry {
    source_len,
    source_mtime_ns,
    direct_imports,
    transitive_imports,
    dependency_hash,
  })
}

fn ensure_import_dependency_graph_cached(
  canon_path: &Path,
  source_len: u64,
  source_mtime_ns: Option<u128>,
  expr: &PnixExpr,
) -> Option<ImportDependencyGraphEntry> {
  let source_mtime_ns = source_mtime_ns?;
  if let Some(cached) = import_dependency_graph_lookup(canon_path, source_len, source_mtime_ns) {
    return Some(cached);
  }

  // 이 canon path 의 그래프 빌드가 이미 이 스레드 콜스택 위에서 진행 중이면
  // (import 순환 안에서 load_baked_expr_at_path 캐시 히트가 touch 로 재진입한
  // 경우) 여기서 끊는다. 바깥 빌드가 끝나면서 entry 를 store 하므로 이
  // None 은 일시적 미스일 뿐 영구 손실이 아니다.
  let reentered = IMPORT_DEPENDENCY_GRAPH_BUILDING
    .with(|building| !building.borrow_mut().insert(canon_path.to_path_buf()));
  if reentered {
    return None;
  }
  struct BuildingGuard(PathBuf);
  impl Drop for BuildingGuard {
    fn drop(&mut self) {
      IMPORT_DEPENDENCY_GRAPH_BUILDING.with(|building| {
        building.borrow_mut().remove(&self.0);
      });
    }
  }
  let _building_guard = BuildingGuard(canon_path.to_path_buf());

  let mut building = FxHashSet::default();
  let mut reentrant_edges = Vec::new();
  let graph = import_dependency_graph_build(
    canon_path,
    source_len,
    source_mtime_ns,
    expr,
    &mut building,
    &mut reentrant_edges,
  )
  .ok()?;
  import_dependency_graph_store(canon_path, graph.clone());
  Some(graph)
}

/// One node in the import graph with high direct or transitive fanout.
#[derive(Clone, Debug, serde::Serialize, PartialEq, Eq)]
pub struct ImportGraphHeavyNode {
  pub path: String,
  pub direct_fanout: usize,
  pub transitive_fanout: usize,
}

/// Static import/dependency graph profile for a `.px` fixture (Perf P7).
///
/// Built from literal `import` edges only — no evaluation. Host transport
/// may pair this with `EvalPerfStats` from a matching eval run.
#[derive(Clone, Debug, serde::Serialize, PartialEq, Eq)]
pub struct ImportGraphProfile {
  pub root_fixture: String,
  pub max_depth: usize,
  pub import_count: usize,
  pub direct_import_count: usize,
  pub repeated_imports: Vec<String>,
  pub heavy_nodes: Vec<ImportGraphHeavyNode>,
  pub recursive_or_reentrant_edges: Vec<String>,
  pub fixture_imports_previous_fixture: bool,
  pub estimated_materialization_nodes: usize,
}

fn import_graph_direct_depth(
  path: &Path,
  cache: &FxHashMap<PathBuf, ImportDependencyGraphEntry>,
  visiting: &mut FxHashSet<PathBuf>,
) -> usize {
  let key = path.to_path_buf();
  if !visiting.insert(key.clone()) {
    return 0;
  }
  let Some(entry) = cache.get(&key) else {
    visiting.remove(&key);
    return 0;
  };
  let mut best = 0usize;
  for child in &entry.direct_imports {
    best = best.max(1 + import_graph_direct_depth(child, cache, visiting));
  }
  visiting.remove(&key);
  best
}

fn path_is_sibling_fixture_import(root_parent: &Path, dep: &Path) -> bool {
  dep.extension().is_some_and(|ext| ext == "px")
    && dep.parent().is_some_and(|parent| parent == root_parent)
}

/// Profile literal import graph for a `.px` file without evaluating it.
pub fn profile_import_graph_at_path(path: &Path) -> Result<ImportGraphProfile> {
  let canon = import_graph_canonical_path(path);
  let source_metadata =
    fs::metadata(path).map_err(|e| anyhow!("metadata {}: {}", path.display(), e))?;
  let source_len = source_metadata.len();
  let source_mtime_ns = metadata_mtime_ns(&source_metadata)
    .ok_or_else(|| anyhow!("missing mtime for {}", path.display()))?;
  let expr = load_baked_expr_at_path(path, &canon)?;
  let mut reentrant_edges = Vec::new();
  let mut building = FxHashSet::default();
  let root_graph = import_dependency_graph_build(
    &canon,
    source_len,
    source_mtime_ns,
    expr.as_ref(),
    &mut building,
    &mut reentrant_edges,
  )?;
  import_dependency_graph_store(&canon, root_graph.clone());

  let cache_snapshot = IMPORT_DEPENDENCY_GRAPH_CACHE.with(|cache| cache.borrow().clone());
  let max_depth = import_graph_direct_depth(&canon, &cache_snapshot, &mut FxHashSet::default());

  let mut inbound: FxHashMap<PathBuf, usize> = FxHashMap::default();
  for entry in cache_snapshot.values() {
    for dep in &entry.direct_imports {
      *inbound.entry(dep.clone()).or_insert(0) += 1;
    }
  }
  let mut repeated_imports: Vec<String> = inbound
    .iter()
    .filter(|(_, count)| **count > 1)
    .map(|(path, _)| path.display().to_string())
    .collect();
  repeated_imports.sort();

  let mut heavy_nodes: Vec<ImportGraphHeavyNode> = cache_snapshot
    .iter()
    .filter(|(_, entry)| entry.direct_imports.len() >= 3 || entry.transitive_imports.len() >= 8)
    .map(|(path, entry)| ImportGraphHeavyNode {
      path: path.display().to_string(),
      direct_fanout: entry.direct_imports.len(),
      transitive_fanout: entry.transitive_imports.len(),
    })
    .collect();
  heavy_nodes.sort_by(|a, b| a.path.cmp(&b.path));

  let root_parent = canon
    .parent()
    .ok_or_else(|| anyhow!("fixture has no parent dir: {}", canon.display()))?;
  let fixture_imports_previous_fixture = root_graph
    .direct_imports
    .iter()
    .any(|dep| path_is_sibling_fixture_import(root_parent, dep));

  // Root-local closure size: root file + transitive import targets for this
  // fixture only. Do not use global cache length — sequential profiles would
  // otherwise inherit nodes from prior fixtures in the same process.
  let estimated_materialization_nodes = 1 + root_graph.transitive_imports.len();

  Ok(ImportGraphProfile {
    root_fixture: path.display().to_string(),
    max_depth,
    import_count: root_graph.transitive_imports.len(),
    direct_import_count: root_graph.direct_imports.len(),
    repeated_imports,
    heavy_nodes,
    recursive_or_reentrant_edges: reentrant_edges,
    fixture_imports_previous_fixture,
    estimated_materialization_nodes,
  })
}

fn import_dependency_snapshot_is_current(dependency: &ImportDependencySnapshot) -> bool {
  // D-2: answered from the per-eval stat memo when the host enabled it
  // — this is THE hot freshness probe (every import-value cache hit
  // re-checks its whole transitive set here).
  let Some((len, mtime_ns)) = import_stat_len_mtime(&dependency.path) else {
    return false;
  };
  if len != dependency.source_len {
    return false;
  }
  mtime_ns == dependency.source_mtime_ns
}

fn import_value_cache_dependencies_are_current(entry: &ImportValueCacheEntry) -> bool {
  entry
    .dependencies
    .iter()
    .all(import_dependency_snapshot_is_current)
}

fn import_value_cache_dependency_hash_for(
  canon_path: &Path,
  source_len: u64,
  source_mtime_ns: u128,
) -> String {
  import_dependency_graph_lookup(canon_path, source_len, source_mtime_ns)
    .map(|entry| entry.dependency_hash)
    .unwrap_or_default()
}

fn import_value_cache_dependency_hash_is_current(
  canon_path: &Path,
  source_len: u64,
  source_mtime_ns: u128,
  stored_hash: &str,
) -> bool {
  if stored_hash.is_empty() {
    return true;
  }
  let Some(graph) = import_dependency_graph_lookup(canon_path, source_len, source_mtime_ns) else {
    return false;
  };
  stored_hash == graph.dependency_hash
}

fn import_value_cache_get(
  canon_path: &Path,
  source_len: u64,
  source_mtime_ns: Option<u128>,
) -> Option<ImportValueCacheEntry> {
  if !import_value_cache_enabled() {
    return None;
  }
  let Some(source_mtime_ns) = source_mtime_ns else {
    record_import_value_cache_miss();
    return None;
  };
  let cache_started = cache_lookup_timing_started();
  let cached = IMPORT_VALUE_CACHE.with(|cache| cache.borrow().get(canon_path).cloned());
  record_cache_lookup_elapsed(cache_started);
  let Some(cached) = cached else {
    record_import_value_cache_miss();
    return None;
  };
  if cached.source_len != source_len || cached.source_mtime_ns != source_mtime_ns {
    record_import_value_cache_miss();
    return None;
  }
  if !import_value_cache_dependency_hash_is_current(
    canon_path,
    source_len,
    source_mtime_ns,
    &cached.dependency_hash,
  ) || !import_value_cache_dependencies_are_current(&cached)
  {
    record_import_value_cache_miss();
    record_import_value_cache_dependency_stale();
    return None;
  }
  record_import_value_cache_hit();
  Some(cached)
}

fn import_value_cache_put(
  canon_path: &Path,
  source_len: u64,
  source_mtime_ns: Option<u128>,
  dependencies: Vec<ImportDependencySnapshot>,
  value: &Value,
) {
  if !import_value_cache_enabled() {
    return;
  }
  let Some(source_mtime_ns) = source_mtime_ns else {
    return;
  };
  if dependencies
    .iter()
    .any(|dependency| dependency.source_mtime_ns.is_none())
  {
    return;
  }
  let dependency_hash =
    import_value_cache_dependency_hash_for(canon_path, source_len, source_mtime_ns);
  IMPORT_VALUE_CACHE.with(|cache| {
    let mut cache = cache.borrow_mut();
    evict_one_cache_entry(&mut cache, IMPORT_VALUE_CACHE_MAX_ENTRIES);
    cache.insert(
      canon_path.to_path_buf(),
      ImportValueCacheEntry {
        source_len,
        source_mtime_ns,
        dependency_hash,
        dependencies,
        value: value.clone(),
      },
    );
  });
  record_import_value_cache_store();
}

fn import_value_cache_impure_builtin_name(name: &str) -> bool {
  matches!(
    name,
    "import"
      | "scopedImport"
      | "readFile"
      | "readDir"
      | "readFileType"
      | "pathExists"
      | "hashFile"
      | "toFile"
      | "getEnv"
      | "fetchGit"
      | "fetchTarball"
      | "fetchTree"
      | "storePath"
      | "derivation"
      | "derivationStrict"
      | "abort"
      | "throw"
      | "trace"
      | "traceVerbose"
      | "warn"
      | "break"
      | "pnixMount"
      | "pnixMounts"
      | "pnixRun"
      | "pnixUmount"
  )
}

fn import_value_cache_static_import_path_expr(expr: &PnixExpr) -> bool {
  match expr {
    PnixExpr::Path(path) => matches!(
      path,
      pnix_core::lang::pnix::syntax::PnixPath::Relative(_)
        | pnix_core::lang::pnix::syntax::PnixPath::Absolute(_)
    ),
    PnixExpr::String(s) => !s.is_empty(),
    _ => false,
  }
}

fn import_value_cache_blocked_by_param_pattern(pattern: &PnixParamPattern) -> bool {
  match pattern {
    PnixParamPattern::Ident(_) => false,
    PnixParamPattern::AttrSet { fields, .. } | PnixParamPattern::AttrSetWithBind { fields, .. } => {
      fields.iter().any(|field| {
        field
          .default
          .as_ref()
          .is_some_and(import_value_cache_blocked_by_expr)
      })
    }
    PnixParamPattern::List(_) => false,
  }
}

fn import_value_cache_blocked_by_attr_item(item: &PnixAttrItem) -> bool {
  match item {
    PnixAttrItem::Assign { value, .. } => import_value_cache_blocked_by_expr(value),
    PnixAttrItem::DynamicAssign {
      key_path, value, ..
    } => {
      key_path.iter().any(|seg| match seg {
        pnix_core::lang::pnix::syntax::AttrKeySegment::Static(_) => false,
        pnix_core::lang::pnix::syntax::AttrKeySegment::Dynamic(expr) => {
          import_value_cache_blocked_by_expr(expr.as_ref())
        }
      }) || import_value_cache_blocked_by_expr(value)
    }
    PnixAttrItem::Inherit { from, names, .. } => {
      names
        .iter()
        .any(|name| import_value_cache_impure_builtin_name(name))
        || from
          .as_ref()
          .is_some_and(|expr| import_value_cache_blocked_by_expr(expr.as_ref()))
    }
  }
}

fn import_value_cache_blocked_by_expr(expr: &PnixExpr) -> bool {
  use pnix_core::lang::pnix::syntax::PnixExpr as E;
  match expr {
    E::Int(_) | E::Float(_) | E::Bool(_) | E::Null | E::String(_) | E::Path(_) => false,
    E::Var(name) => import_value_cache_impure_builtin_name(name),
    E::StringInterp(parts) => parts.iter().any(|part| match part {
      StringInterpPart::Lit(_) => false,
      StringInterpPart::Expr(expr) => import_value_cache_blocked_by_expr(expr.as_ref()),
    }),
    E::Let { bindings, body } => {
      bindings.iter().any(|binding| match binding {
        PnixLetBinding::Binding { pattern, value } => {
          import_value_cache_blocked_by_param_pattern(pattern)
            || import_value_cache_blocked_by_expr(value)
        }
        PnixLetBinding::Inherit { from, names } => {
          names
            .iter()
            .any(|name| import_value_cache_impure_builtin_name(name))
            || from
              .as_ref()
              .is_some_and(|expr| import_value_cache_blocked_by_expr(expr.as_ref()))
        }
      }) || import_value_cache_blocked_by_expr(body)
    }
    E::If { cond, then_, else_ } => {
      import_value_cache_blocked_by_expr(cond)
        || import_value_cache_blocked_by_expr(then_)
        || import_value_cache_blocked_by_expr(else_)
    }
    E::Lambda { param, body } => {
      import_value_cache_blocked_by_param_pattern(param) || import_value_cache_blocked_by_expr(body)
    }
    E::Apply { func, arg } => {
      import_value_cache_blocked_by_expr(func) || import_value_cache_blocked_by_expr(arg)
    }
    E::AttrSet { items, .. } => items.iter().any(import_value_cache_blocked_by_attr_item),
    E::List(items) | E::Construct { args: items, .. } => {
      items.iter().any(import_value_cache_blocked_by_expr)
    }
    E::Select { base, attr } => {
      import_value_cache_impure_builtin_name(attr) || import_value_cache_blocked_by_expr(base)
    }
    E::SelectOrDefault {
      base,
      attr,
      default,
    } => {
      import_value_cache_impure_builtin_name(attr)
        || import_value_cache_blocked_by_expr(base)
        || import_value_cache_blocked_by_expr(default)
    }
    E::Index { base, index }
    | E::Binary {
      lhs: base,
      rhs: index,
      ..
    } => import_value_cache_blocked_by_expr(base) || import_value_cache_blocked_by_expr(index),
    E::Unary { arg, .. } => import_value_cache_blocked_by_expr(arg),
    E::Match { scrutinee, arms } => {
      import_value_cache_blocked_by_expr(scrutinee)
        || arms.iter().any(|arm| {
          arm
            .guard
            .as_ref()
            .is_some_and(|expr| import_value_cache_blocked_by_expr(expr.as_ref()))
            || import_value_cache_blocked_by_expr(&arm.body)
        })
    }
    E::Import { path } => {
      if import_value_cache_static_import_path_expr(path.as_ref()) {
        record_import_dependency_static();
        false
      } else {
        record_import_dependency_dynamic();
        true
      }
    }
    E::With { env, body } => {
      import_value_cache_blocked_by_expr(env) || import_value_cache_blocked_by_expr(body)
    }
    E::Assert { cond, body } => {
      import_value_cache_blocked_by_expr(cond) || import_value_cache_blocked_by_expr(body)
    }
    E::HasAttr { base, .. } => import_value_cache_blocked_by_expr(base),
    E::DynamicHasAttr { base, attr_expr } => {
      import_value_cache_blocked_by_expr(base) || import_value_cache_blocked_by_expr(attr_expr)
    }
    E::DynamicSelect { .. } => true,
    E::DynamicSelectOrDefault { .. } => true,
  }
}

pub fn eval_file_at_path(path: &Path) -> Result<Value> {
  // Cycle guard first — refuse to even read a file we're already
  // evaluating further up the call chain. The base-path guard sits
  // inside this so the cycle detection surfaces before we walk any
  // child relative imports.
  let file_guard = push_import_file_guard_and_record(path)?;
  let source_metadata =
    fs::metadata(file_guard.canon()).map_err(|e| anyhow!("metadata {}: {}", path.display(), e))?;
  let source_len = source_metadata.len();
  let source_mtime_ns = metadata_mtime_ns(&source_metadata);
  if let Some(cached) = import_value_cache_get(file_guard.canon(), source_len, source_mtime_ns) {
    record_import_dependencies_to_parent(&cached.dependencies);
    return Ok(cached.value.clone());
  }
  let self_dependency = ImportDependencySnapshot {
    path: file_guard.canon().to_path_buf(),
    source_len,
    source_mtime_ns,
  };
  let dependency_frame = ImportDependencyFrameGuard::push(self_dependency);
  let expr_owned = load_baked_expr_at_path(path, file_guard.canon())?;
  let import_value_cacheable = !import_value_cache_blocked_by_expr(expr_owned.as_ref());
  if !import_value_cacheable {
    record_import_value_cache_uncacheable();
  }
  let env = Env::new();
  let _base_guard = path
    .parent()
    .map(|parent| ImportBaseGuard::push(parent.to_path_buf()));
  let value = eval(expr_owned.as_ref(), &env)?;
  // Fully force the loaded value while the import-base guard is
  // still on the stack. Lazy thunks deferred past the guard's drop
  // can't resolve relative `import ./...` inside the file because
  // `current_import_base()` returns `None` once the guard pops —
  // see `resolve_value_path` and the v0 minimal tesseract harness
  // (`v0_run_with_owner_law.px`) which exercises this path.
  //
  // Belt-and-suspenders: AST baking above already turns relative path
  // *literals* into absolutes, but a relative path can still arrive at
  // eval time via string-coercion / interpolation / dynamic
  // construction. For those the dynamic stack remains authoritative.
  let value = deep_force(value)?;
  let dependencies = dependency_frame.finish();
  record_import_dependencies_to_parent(&dependencies);
  if import_value_cacheable {
    import_value_cache_put(
      file_guard.canon(),
      source_len,
      source_mtime_ns,
      dependencies,
      &value,
    );
  }
  Ok(value)
}

/// Walks `expr` and replaces every `PnixExpr::Path(PnixPath::Relative(s))`
/// with `PnixExpr::Path(PnixPath::Absolute(base.join(s)))`. Other path
/// flavours (Absolute, Search, Home, Interpolated) pass through; for
/// Interpolated, the embedded expression parts are recursed. Other AST
/// nodes mirror `desugar_expr`'s shape so every sub-expression slot is
/// visited.
fn bake_relative_paths_in_expr(
  expr: pnix_core::lang::pnix::syntax::PnixExpr,
  base: &Path,
) -> pnix_core::lang::pnix::syntax::PnixExpr {
  use pnix_core::lang::pnix::syntax::{
    AttrKeySegment, PnixAttrItem, PnixExpr as E, PnixLetBinding, PnixMatchArm, PnixPath,
    StringInterpPart,
  };
  let go = |e: pnix_core::lang::pnix::syntax::PnixExpr| -> pnix_core::lang::pnix::syntax::PnixExpr {
    bake_relative_paths_in_expr(e, base)
  };
  let go_arc =
    |e: std::sync::Arc<pnix_core::lang::pnix::syntax::PnixExpr>| -> std::sync::Arc<pnix_core::lang::pnix::syntax::PnixExpr> {
      std::sync::Arc::new(bake_relative_paths_in_expr(
        std::sync::Arc::unwrap_or_clone(e),
        base,
      ))
    };
  match expr {
    E::Int(_) | E::Float(_) | E::Bool(_) | E::Null | E::String(_) | E::Var(_) => expr,
    E::Path(p) => match p {
      PnixPath::Relative(s) => {
        // Join base with the relative path. PathBuf::join correctly
        // handles a leading `./` or `../` segment. Then normalise.
        let joined = normalize_pnix_path(&base.join(&s));
        E::Path(PnixPath::Absolute(joined.to_string_lossy().into_owned()))
      }
      // Absolute / Search (<...>) / Home (~/) keep their original
      // resolution semantics; only literal relatives need baking.
      PnixPath::Absolute(_) | PnixPath::Search(_) | PnixPath::Home(_) => E::Path(p),
      PnixPath::Interpolated { base: b, parts } => {
        let mut baked_parts = Vec::with_capacity(parts.len());
        for part in parts {
          baked_parts.push(match part {
            StringInterpPart::Lit(s) => StringInterpPart::Lit(s),
            StringInterpPart::Expr(e) => StringInterpPart::Expr(go_arc(e)),
          });
        }
        E::Path(PnixPath::Interpolated {
          base: b,
          parts: baked_parts,
        })
      }
    },
    E::StringInterp(parts) => {
      let mut baked_parts = Vec::with_capacity(parts.len());
      for part in parts {
        baked_parts.push(match part {
          StringInterpPart::Lit(s) => StringInterpPart::Lit(s),
          StringInterpPart::Expr(e) => StringInterpPart::Expr(go_arc(e)),
        });
      }
      E::StringInterp(baked_parts)
    }
    E::Let { bindings, body } => {
      let mut baked_bindings = Vec::with_capacity(bindings.len());
      for binding in bindings {
        baked_bindings.push(match binding {
          PnixLetBinding::Binding { pattern, value } => PnixLetBinding::Binding {
            pattern: bake_relative_paths_in_param_pattern(pattern, base),
            value: go_arc(value),
          },
          PnixLetBinding::Inherit { from, names } => PnixLetBinding::Inherit {
            from: from.map(go_arc),
            names,
          },
        });
      }
      E::Let {
        bindings: baked_bindings,
        body: go_arc(body),
      }
    }
    E::If { cond, then_, else_ } => E::If {
      cond: go_arc(cond),
      then_: go_arc(then_),
      else_: go_arc(else_),
    },
    E::Lambda { param, body } => E::Lambda {
      param: bake_relative_paths_in_param_pattern(param, base),
      body: go_arc(body),
    },
    E::Apply { func, arg } => E::Apply {
      func: go_arc(func),
      arg: go_arc(arg),
    },
    E::AttrSet { items, recursive } => {
      let mut baked_items = Vec::with_capacity(items.len());
      for item in items {
        baked_items.push(match item {
          PnixAttrItem::Assign {
            key_path,
            value,
            span,
          } => PnixAttrItem::Assign {
            key_path,
            value: go_arc(value),
            span,
          },
          PnixAttrItem::DynamicAssign {
            key_path,
            value,
            span,
          } => PnixAttrItem::DynamicAssign {
            key_path: {
              let mut baked_key_path = Vec::with_capacity(key_path.len());
              for seg in key_path {
                baked_key_path.push(match seg {
                  AttrKeySegment::Static(_) => seg,
                  AttrKeySegment::Dynamic(e) => AttrKeySegment::Dynamic(go_arc(e)),
                });
              }
              baked_key_path
            },
            value: go_arc(value),
            span,
          },
          PnixAttrItem::Inherit { from, names, span } => PnixAttrItem::Inherit {
            from: from.map(go_arc),
            names,
            span,
          },
        });
      }
      E::AttrSet {
        items: baked_items,
        recursive,
      }
    }
    E::List(items) => {
      let mut baked_items = Vec::with_capacity(items.len());
      for item in items {
        baked_items.push(go(item));
      }
      E::List(baked_items)
    }
    E::Select { base: b, attr } => E::Select {
      base: go_arc(b),
      attr,
    },
    E::SelectOrDefault {
      base: b,
      attr,
      default,
    } => E::SelectOrDefault {
      base: go_arc(b),
      attr,
      default: go_arc(default),
    },
    E::Index { base: b, index } => E::Index {
      base: go_arc(b),
      index: go_arc(index),
    },
    E::Binary { op, lhs, rhs } => E::Binary {
      op,
      lhs: go_arc(lhs),
      rhs: go_arc(rhs),
    },
    E::Unary { op, arg } => E::Unary {
      op,
      arg: go_arc(arg),
    },
    E::Construct { variant, args } => {
      let mut baked_args = Vec::with_capacity(args.len());
      for arg in args {
        baked_args.push(go(arg));
      }
      E::Construct {
        variant,
        args: baked_args,
      }
    }
    E::Match { scrutinee, arms } => {
      let mut baked_arms = Vec::with_capacity(arms.len());
      for arm in arms {
        baked_arms.push(PnixMatchArm {
          pattern: arm.pattern,
          guard: arm.guard.map(go_arc),
          body: go_arc(arm.body),
        });
      }
      E::Match {
        scrutinee: go_arc(scrutinee),
        arms: baked_arms,
      }
    }
    E::Import { path } => E::Import { path: go_arc(path) },
    E::With { env, body } => E::With {
      env: go_arc(env),
      body: go_arc(body),
    },
    E::Assert { cond, body } => E::Assert {
      cond: go_arc(cond),
      body: go_arc(body),
    },
    E::HasAttr { base: b, attr } => E::HasAttr {
      base: go_arc(b),
      attr,
    },
    E::DynamicHasAttr { base: b, attr_expr } => E::DynamicHasAttr {
      base: go_arc(b),
      attr_expr: go_arc(attr_expr),
    },
    E::DynamicSelect { base: b, attr_expr } => E::DynamicSelect {
      base: go_arc(b),
      attr_expr: go_arc(attr_expr),
    },
    E::DynamicSelectOrDefault {
      base: b,
      attr_expr,
      default,
    } => E::DynamicSelectOrDefault {
      base: go_arc(b),
      attr_expr: go_arc(attr_expr),
      default: go_arc(default),
    },
  }
}

fn bake_relative_paths_in_param_pattern(
  pattern: pnix_core::lang::pnix::syntax::PnixParamPattern,
  base: &Path,
) -> pnix_core::lang::pnix::syntax::PnixParamPattern {
  use pnix_core::lang::pnix::syntax::{PnixParamPattern as P, PnixPatternField};
  match pattern {
    P::Ident(_) | P::List(_) => pattern,
    P::AttrSet { fields, ellipsis } => {
      let mut baked_fields = Vec::with_capacity(fields.len());
      for field in fields {
        baked_fields.push(PnixPatternField {
          name: field.name,
          default: field.default.map(|e| bake_relative_paths_in_expr(e, base)),
        });
      }
      P::AttrSet {
        fields: baked_fields,
        ellipsis,
      }
    }
    P::AttrSetWithBind {
      bind_name,
      fields,
      ellipsis,
    } => {
      let mut baked_fields = Vec::with_capacity(fields.len());
      for field in fields {
        baked_fields.push(PnixPatternField {
          name: field.name,
          default: field.default.map(|e| bake_relative_paths_in_expr(e, base)),
        });
      }
      P::AttrSetWithBind {
        bind_name,
        fields: baked_fields,
        ellipsis,
      }
    }
  }
}

thread_local! {
  /// Thunk caches currently being forced on this thread's call
  /// stack. A second force_value entry on a cache that is already
  /// here means `eval` re-entered the same thunk before the first
  /// invocation could populate the cache — i.e. a self-referential
  /// expression like `let x = x; in x` or `let s = { x = s.x; };
  /// in s.x`. Without this guard those expressions stack-overflow;
  /// with it, they surface as a clear `infinite recursion
  /// encountered` error.
  static FORCING_THUNKS: RefCell<Vec<Arc<std::sync::OnceLock<Value>>>> =
    const { RefCell::new(Vec::new()) };

  /// Current `eval()` call depth on this thread. Used by
  /// `enter_eval` to cap recursion before it overflows the Rust
  /// stack, which on Linux is uncatchable (SIGSEGV converted to
  /// `fatal runtime error: stack overflow, aborting`).
  ///
  /// Calibration on the default 64 MiB pnixc-meta worker stack
  /// (release build, this codebase): a `go N` recursion bench
  /// stack-overflows around N=20,000, which is ~60,000 eval frames
  /// (each `go` adds roughly three nested eval calls). Defaulting
  /// the cap to 16,384 leaves comfortable safety margin while still
  /// allowing real programs that recurse a few thousand deep.
  static EVAL_DEPTH: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Hard ceiling for `eval()` recursion. Overridable per-thread via
/// the `PNIX_EVAL_MAX_DEPTH` env var read once at process start.
/// Static cell so reads in the hot path are a plain pointer load,
/// not an env-var lookup.
static EVAL_MAX_DEPTH: std::sync::OnceLock<usize> = std::sync::OnceLock::new();

fn eval_max_depth() -> usize {
  *EVAL_MAX_DEPTH.get_or_init(|| {
    std::env::var("PNIX_EVAL_MAX_DEPTH")
      .ok()
      .and_then(|s| s.parse().ok())
      .unwrap_or(16_384)
  })
}

/// RAII guard returned by `enter_eval`: increments `EVAL_DEPTH` on
/// construction and decrements on Drop. Returned even when the body
/// panics; the depth count survives normal early-return errors too.
struct EvalDepthGuard;

impl Drop for EvalDepthGuard {
  fn drop(&mut self) {
    EVAL_DEPTH.with(|d| d.set(d.get().saturating_sub(1)));
  }
}

/// Increment the per-thread eval-depth counter or return `Err` if
/// the depth ceiling has been reached. The returned guard MUST be
/// kept alive for the duration of the `eval()` body so the
/// decrement runs on every exit path (including unwinding panics
/// from `catch_unwind`).
fn enter_eval() -> Result<EvalDepthGuard> {
  let max = eval_max_depth();
  let next = EVAL_DEPTH.with(|d| {
    let n = d.get() + 1;
    d.set(n);
    n
  });
  if next > max {
    // Decrement before returning the Err so the failing call still
    // balances the counter (we won't be holding a guard).
    EVAL_DEPTH.with(|d| d.set(d.get().saturating_sub(1)));
    anyhow::bail!(
      "eval recursion depth exceeded {} (override via PNIX_EVAL_MAX_DEPTH)",
      max
    );
  }
  Ok(EvalDepthGuard)
}

/// Ex-2 (2026-06-11): RAII carrier for one FORCING_THUNKS entry, held
/// inside a machine `ForceThunk` frame. Construction pushes the cache
/// onto the cycle-guard stack; Drop pops one entry — so the explicit
/// frame-stack unwind on an early `?` return (the `Vec<Frame>` drop)
/// balances the cycle guard exactly like `force_value`'s manual
/// push/pop does around its native eval call. Drop order across
/// multiple unwinding frames doesn't matter: each Drop pops one of
/// the top N entries, all of which belong to the unwinding frames.
struct ForcingGuard;

impl ForcingGuard {
  fn push(cache: Arc<std::sync::OnceLock<Value>>) -> ForcingGuard {
    FORCING_THUNKS.with(|stack| stack.borrow_mut().push(cache));
    ForcingGuard
  }
}

impl Drop for ForcingGuard {
  fn drop(&mut self) {
    FORCING_THUNKS.with(|stack| {
      stack.borrow_mut().pop();
    });
  }
}

/// Shallowly force a value: if it is a `Thunk`, evaluate the underlying
/// expression in its captured environment, memoize, and unwrap until a
/// non-`Thunk` value is reached. Does NOT recurse into list elements or
/// attrset fields — those stay lazy.
pub fn force_value(mut value: Value) -> Result<Value> {
  loop {
    match value {
      Value::Thunk {
        expr,
        env,
        cache,
        attr_pos: _,
      } => {
        if let Some(cached) = cache.get().cloned() {
          value = cached;
          continue;
        }
        // Cycle guard: refuse to recurse into a thunk we're
        // already evaluating elsewhere on this stack. Compare by
        // `Arc::ptr_eq` so unrelated thunks that happened to reuse
        // an allocator slot don't false-positive.
        let already_forcing =
          FORCING_THUNKS.with(|stack| stack.borrow().iter().any(|c| Arc::ptr_eq(c, &cache)));
        if already_forcing {
          return Err(anyhow!("infinite recursion encountered"));
        }
        FORCING_THUNKS.with(|stack| stack.borrow_mut().push(cache.clone()));
        // Ex-1: the thunk already owns its `Arc<Env>` — enter the
        // machine in the Shared state so the first env capture is a
        // refcount bump, not an `Arc::new(env.clone())`.
        let result = eval_arc_shared(expr.clone(), env.clone());
        FORCING_THUNKS.with(|stack| {
          stack.borrow_mut().pop();
        });
        let forced = result?;
        // OnceLock::set returns Err if already set (race with another
        // thread also forcing this thunk). Either way, the cache is
        // now populated with a valid Value; ignore the race outcome.
        let _ = cache.set(forced.clone());
        value = forced;
      }
      other => return Ok(other),
    }
  }
}

/// Recursively force a value, forcing all thunks inside lists / attrsets.
/// Used by JSON/display surfaces that need a fully strict value.
///
/// Cycle-safe: tracks the Thunk caches currently on the descent path
/// by holding their `Rc` clones (not just raw pointers) so that
/// self-referential structures like `let as = { y = as; }; in as` or
/// `let xs = [xs]; in xs` terminate instead of recursing forever.
///
/// Why hold the `Rc` and not just the address: an earlier version
/// stored only `Arc::as_ptr(cache) as usize`. That broke nested
/// function-call attrsets like `{a={b={wrap=(mk[..])}}}` because
/// `force_value` consumes the outer thunk, the outer cache's `Rc`
/// count drops to zero, the allocator reuses the address for a new
/// thunk created during the inner function call, and the new thunk's
/// pointer matched a stale "visited" entry — so the inner subtree's
/// children were skipped and surfaced as `"<thunk>"` in JSON output
/// (puck `pnix3d_scene.px` regression). Holding the `Rc` keeps the
/// allocation pinned for the duration of the descent and makes the
/// visited check sound.
pub fn deep_force(value: Value) -> Result<Value> {
  let started = deep_force_timing_enabled().then(std::time::Instant::now);
  let mut perf = DeepForcePerf::default();
  if value_is_deep_force_leaf(&value) {
    observe_deep_force_value(&value, 0, &mut perf);
    record_deep_force_perf(
      &perf,
      started.map_or_else(Default::default, |s| s.elapsed()),
    );
    return Ok(value);
  }
  let mut path: Vec<Arc<std::sync::OnceLock<Value>>> = Vec::with_capacity(8);
  let mut value = value;
  let result = deep_force_in_place(&mut value, &mut path, 0, &mut perf);
  record_deep_force_perf(
    &perf,
    started.map_or_else(Default::default, |s| s.elapsed()),
  );
  result.map(|()| value)
}

// DF-1 perf slice (2026-06-10): deep force mutates the owned value tree
// IN PLACE instead of rebuilding every `Vec` / `BTreeMap` level on each
// call. The pre-fix shape (`deep_force_visited(value) -> Value`) built a
// brand-new `Vec::with_capacity` per list and re-inserted every key into
// a brand-new `BTreeMap` per attrset — a full container-tree
// reallocation on every deep-force boundary, even when the tree
// contained zero thunks. On the math substrate-wide workload this
// rebuild (plus its malloc/free traffic) dominated the leaf profile.
// In-place replacement keeps: the same DFS visit order, the same
// `observe_deep_force_value` telemetry sequence (one observe per node;
// thunk nodes observed again after forcing), the same cycle guard
// (Arc-pinned cache path — see the doc comment on `deep_force` above
// for why the `Rc` is held), and the same error propagation points.
// Only the wasted container reconstruction is gone.
fn deep_force_in_place(
  slot: &mut Value,
  path: &mut Vec<Arc<std::sync::OnceLock<Value>>>,
  depth: usize,
  perf: &mut DeepForcePerf,
) -> Result<()> {
  let was_thunk = matches!(&*slot, Value::Thunk { .. });
  observe_deep_force_value(slot, depth, perf);
  let mut pushed = false;
  if let Value::Thunk { cache, .. } = &*slot {
    if path.iter().any(|c| Arc::ptr_eq(c, cache)) {
      // Genuine self-cycle along the current descent path — return
      // the forced value without descending again.
      let owned = std::mem::replace(slot, Value::Null);
      *slot = force_value(owned)?;
      observe_deep_force_value(slot, depth, perf);
      return Ok(());
    }
    path.push(cache.clone());
    pushed = true;
  }
  if was_thunk {
    let owned = std::mem::replace(slot, Value::Null);
    *slot = force_value(owned)?;
    observe_deep_force_value(slot, depth, perf);
  }
  match slot {
    Value::List(items) => {
      for item in Arc::make_mut(items).iter_mut() {
        deep_force_in_place(item, path, depth + 1, perf)?;
      }
    }
    Value::AttrSet(map) => {
      for v in Arc::make_mut(map).values_mut() {
        deep_force_in_place(v, path, depth + 1, perf)?;
      }
    }
    _ => {}
  }
  if pushed {
    path.pop();
  }
  Ok(())
}

fn make_thunk(expr: &PnixExpr, env: &Env) -> Value {
  Value::Thunk {
    expr: Arc::new(expr.clone()),
    env: Arc::new(env.clone()),
    cache: Arc::new(std::sync::OnceLock::new()),
    attr_pos: None,
  }
}

fn make_thunk_arc(expr: Arc<PnixExpr>, env: &Env) -> Value {
  Value::Thunk {
    expr,
    env: Arc::new(env.clone()),
    cache: Arc::new(std::sync::OnceLock::new()),
    attr_pos: None,
  }
}

// R3 perf slice (2026-06-10): thunk constructor for callers that hold a
// shared `Arc<Env>` snapshot. The per-field/per-element constructors
// above wrap a FRESH `Arc::new(env.clone())` per thunk; attrset/list
// literals build N thunks against the SAME immutable env, so one shared
// snapshot Arc (refcount-bumped per thunk) replaces N Env clones + N
// Arc allocations. Sharing is observationally identical because the
// literal arms never mutate the env during construction and `Env::bind`
// is copy-on-write (`Arc::make_mut`) for everyone else.
fn make_thunk_shared_env(
  expr: Arc<PnixExpr>,
  env: Arc<Env>,
  attr_pos: Option<AttrSourcePos>,
) -> Value {
  Value::Thunk {
    expr,
    env,
    cache: Arc::new(std::sync::OnceLock::new()),
    attr_pos,
  }
}

/// Convert a parser `Span` into an `AttrSourcePos`, dropping the result
/// if line / column were not populated (the parser only fills them for
/// AttrItem-level spans).
fn attr_source_pos_from_span(span: &pnix_core::diagnostics::Span) -> Option<AttrSourcePos> {
  if span.line == 0 {
    return None;
  }
  Some(AttrSourcePos {
    byte_pos: span.start,
    line: span.line,
    column: span.column,
    file: Arc::new(span.file.clone().unwrap_or_else(|| "<unknown>".to_string())),
  })
}

/// Build a lazy two-argument application: when forced, evaluates
/// `apply_value(apply_value(f, a), b)`. Used by builtins like `mapAttrs`
/// that need to defer their per-element function calls so the caller's
/// laziness invariants (e.g. `mapAttrs throw set` not throwing until a
/// field is accessed) are preserved.
// 2026-05-05 (slice #62): single-arg lazy application. Mirrors
// `deferred_apply2` but for one-argument functions. Used by
// `map` and `genList` to produce a list whose elements are
// thunks for `f x` rather than already-evaluated values. Pre-fix
// `map` / `genList` eagerly applied the function during list
// construction, so `length (map throw [1 2 3])` errored on the
// first element instead of returning 3 — every legitimate lazy
// pattern that mapped over a list and then took its length /
// head / first slice was broken.
// S15 perf slice (2026-05-21): the Apply AST inside deferred_apply /
// deferred_apply2 is *the same expression every time* — only the
// env's __pnix_f / __pnix_a (/ __pnix_b) bindings vary per call. We
// cache the AST in static OnceLocks so each call Arc::clone the
// already-allocated Arc<PnixExpr> instead of building 3 (or 4) fresh
// nodes per element. For a `map f xs` over a 1000-element list this
// saves ~3000 (4000) PnixExpr heap allocations. Env, OnceLock, and
// per-element bindings remain unchanged (they are element-specific).
static DEFERRED_APPLY_AST: std::sync::OnceLock<Arc<PnixExpr>> = std::sync::OnceLock::new();

fn deferred_apply_ast() -> Arc<PnixExpr> {
  DEFERRED_APPLY_AST
    .get_or_init(|| {
      Arc::new(PnixExpr::Apply {
        func: Arc::new(PnixExpr::Var("__pnix_f".to_string())),
        arg: Arc::new(PnixExpr::Var("__pnix_a".to_string())),
      })
    })
    .clone()
}

static DEFERRED_APPLY2_AST: std::sync::OnceLock<Arc<PnixExpr>> = std::sync::OnceLock::new();

fn deferred_apply2_ast() -> Arc<PnixExpr> {
  DEFERRED_APPLY2_AST
    .get_or_init(|| {
      Arc::new(PnixExpr::Apply {
        func: Arc::new(PnixExpr::Apply {
          func: Arc::new(PnixExpr::Var("__pnix_f".to_string())),
          arg: Arc::new(PnixExpr::Var("__pnix_a".to_string())),
        }),
        arg: Arc::new(PnixExpr::Var("__pnix_b".to_string())),
      })
    })
    .clone()
}

fn deferred_apply(f: Value, a: Value) -> Value {
  let mut env = Env::with_capacity(2);
  env.bind("__pnix_f".to_string(), f);
  env.bind("__pnix_a".to_string(), a);
  Value::Thunk {
    expr: deferred_apply_ast(),
    env: Arc::new(env),
    cache: Arc::new(std::sync::OnceLock::new()),
    attr_pos: None,
  }
}

fn deferred_apply2(f: Value, a: Value, b: Value) -> Value {
  let mut env = Env::with_capacity(3);
  env.bind("__pnix_f".to_string(), f);
  env.bind("__pnix_a".to_string(), a);
  env.bind("__pnix_b".to_string(), b);
  Value::Thunk {
    expr: deferred_apply2_ast(),
    env: Arc::new(env),
    cache: Arc::new(std::sync::OnceLock::new()),
    attr_pos: None,
  }
}

/// Insert a value at a multi-segment attribute path, creating and merging
/// intermediate attrsets as needed.
///
/// Nix semantics:
///   `{ a.b = 1; a.c = 2; }` ≡ `{ a = { b = 1; c = 2; }; }`
///   `{ a = { b = 1; }; a.c = 2; }` merges both definitions.
///   `{ a = { b = 1; }; a = { c = 2; }; }` also merges (two attrset
///   literals at the same path).
///   `{ a = 1; a = 2; }` is a duplicate error.
fn insert_nested_attr(
  map: &mut BTreeMap<String, Value>,
  path: &[String],
  value: Value,
) -> Result<()> {
  if path.is_empty() {
    return Err(anyhow!("empty attribute path"));
  }
  let head = &path[0];
  if path.len() == 1 {
    if let Some(existing) = map.get(head) {
      // Try to merge if both sides are attrset thunks (Nix-compat path
      // assignment merging). Otherwise this is a duplicate.
      if let (Some(existing_items), Some(existing_env), Some(new_items), Some(new_env)) = (
        attrset_thunk_items(existing),
        attrset_thunk_env(existing),
        attrset_thunk_items(&value),
        attrset_thunk_env(&value),
      ) {
        // Build a merged attrset expression that contains both sets of
        // items in declaration order. Recursive flag is kept from either
        // side that was recursive (Nix collapses both into a single
        // attrset literal).
        let recursive = attrset_thunk_recursive(existing) || attrset_thunk_recursive(&value);
        let mut merged_items = existing_items.clone();
        for item in new_items {
          merged_items.push(item.clone());
        }
        let merged_expr = PnixExpr::AttrSet {
          items: merged_items,
          recursive,
        };
        // Use the existing env to preserve the original capture; sibling
        // env should match for the common case (same enclosing scope).
        let _ = new_env;
        map.insert(head.clone(), make_thunk(&merged_expr, existing_env));
        return Ok(());
      }
      return Err(anyhow!(
        "attribute '{}' already defined at this level",
        head
      ));
    }
    map.insert(head.clone(), value);
    return Ok(());
  }
  // Path length > 1 — descend into / extend the intermediate.
  if !map.contains_key(head) {
    map.insert(head.clone(), Value::AttrSet(Arc::new(BTreeMap::new())));
  }
  let entry = map.get(head).cloned().unwrap();
  match entry {
    Value::AttrSet(mut inner) => {
      // V-2: the fetched Arc may be shared; make_mut deep-clones in
      // that case — the cost the pre-Arc form paid unconditionally.
      insert_nested_attr(Arc::make_mut(&mut inner), &path[1..], value)?;
      map.insert(head.clone(), Value::AttrSet(inner));
      Ok(())
    }
    Value::Thunk { .. } => {
      // Nix-compat: `a = rec { ... }; a.c = expr;` extends the rec set
      // (rather than colliding). Splice a fresh `AttrItem::Assign` for
      // the remaining path into the existing thunk's items list — that
      // way the new attr participates in the rec scope and `expr` can
      // reference siblings.
      let existing_items = attrset_thunk_items(&entry).cloned();
      let existing_env = attrset_thunk_env(&entry).cloned();
      let recursive = attrset_thunk_recursive(&entry);
      let (Some(mut items), Some(env)) = (existing_items, existing_env) else {
        return Err(anyhow!(
          "attribute path conflict: '{}' is already a non-attrset value",
          head
        ));
      };
      // Build an AttrItem::Assign for the rest of the path.
      let rest_path: Vec<String> = path[1..].to_vec();
      let value_expr = thunk_to_expr(&value);
      let span = pnix_core::diagnostics::Span::empty();
      items.push(PnixAttrItem::Assign {
        key_path: rest_path,
        value: Arc::new(value_expr),
        span,
      });
      let merged_expr = PnixExpr::AttrSet { items, recursive };
      map.insert(head.clone(), make_thunk(&merged_expr, &env));
      Ok(())
    }
    _ => Err(anyhow!(
      "attribute path conflict: '{}' is already a non-attrset value",
      head
    )),
  }
}

/// Recover an underlying `PnixExpr` from a thunk so we can splice it
/// into a synthesized `AttrItem::Assign`. For non-thunk values we cannot
/// reliably round-trip back to source-level AST, so we wrap them with a
/// no-op binding via the recursive expr extension is impossible — return
/// a placeholder error.
fn thunk_to_expr(v: &Value) -> PnixExpr {
  if let Value::Thunk { expr, .. } = v {
    (**expr).clone()
  } else {
    // Embed the value via an unevaluatable marker. In practice this
    // path is hit only when the new RHS is already-eager (rare for
    // path-extension; usually it's a thunk built from `make_thunk`).
    PnixExpr::String(embedded_value_marker(v))
  }
}

fn embedded_value_marker(v: &Value) -> String {
  use std::fmt::Write as _;

  let mut marker = String::from("<embedded:");
  let _ = write!(&mut marker, "{v}");
  marker.push('>');
  marker
}

fn attrset_thunk_items(v: &Value) -> Option<&Vec<PnixAttrItem>> {
  let Value::Thunk { expr, .. } = v else {
    return None;
  };
  if let PnixExpr::AttrSet { items, .. } = expr.as_ref() {
    Some(items)
  } else {
    None
  }
}

fn attrset_thunk_env(v: &Value) -> Option<&Env> {
  let Value::Thunk { env, .. } = v else {
    return None;
  };
  Some(env)
}

fn attrset_thunk_recursive(v: &Value) -> bool {
  let Value::Thunk { expr, .. } = v else {
    return false;
  };
  matches!(
    expr.as_ref(),
    PnixExpr::AttrSet {
      recursive: true,
      ..
    }
  )
}

/// Force a value into a list and force each element. Use when a builtin
/// must iterate strict list elements (e.g. catAttrs, partition, sort).
pub fn force_list_items(v: &Value) -> Result<Vec<Value>> {
  let v = force_if_thunk(v)?;
  match v.as_ref() {
    Value::List(items) => {
      let mut out = Vec::with_capacity(items.len());
      for item in items.iter() {
        out.push(force_if_thunk(item)?.into_owned());
      }
      Ok(out)
    }
    other => Err(anyhow!("expected a list, got {:?}", other_type(other))),
  }
}

/// Force a value into an attrset and force each value. Use when a builtin
/// must iterate strict attrset values.
pub fn force_attrset_values(v: &Value) -> Result<BTreeMap<String, Value>> {
  let v = force_if_thunk(v)?;
  match v.as_ref() {
    Value::AttrSet(map) => {
      let mut out = BTreeMap::new();
      for (key, value) in map.iter() {
        out.insert(key.clone(), force_if_thunk(value)?.into_owned());
      }
      Ok(out)
    }
    other => Err(anyhow!("expected an attrset, got {:?}", other_type(other))),
  }
}

fn other_type(v: &Value) -> &'static str {
  type_name(v)
}

/// Names that Nix exposes as bare globals in addition to `builtins.<name>`.
/// See nix/src/libexpr/primops.cc — primops without the `__` prefix.
fn global_builtin_alias(name: &str) -> bool {
  matches!(
    name,
    "abort"
      | "baseNameOf"
      | "break"
      | "dirOf"
      | "fetchGit"
      | "fetchTarball"
      | "fetchTree"
      | "import"
      | "isNull"
      | "map"
      | "placeholder"
      | "removeAttrs"
      | "scopedImport"
      | "throw"
      | "toString"
  )
}

pub fn eval(expr: &PnixExpr, env: &Env) -> Result<Value> {
  eval_machine(Arc::new(expr.clone()), EnvRef::Borrowed(env))
}

// A7 (2026-06-11): pub(crate) so value.rs callers that already hold
// an Arc<PnixExpr> (recursive-binding exprs, with-frame sources) skip
// the `Arc::new(expr.clone())` re-wrap the bare `eval` entry pays.
pub(crate) fn eval_arc(expr: Arc<PnixExpr>, env: &Env) -> Result<Value> {
  eval_machine(expr, EnvRef::Borrowed(env))
}

// Ex-1 (2026-06-11): entry for callers that already hold an `Arc<Env>`
// (thunk forcing is the dominant one). The machine starts in the
// Shared env state, so the first env capture is a refcount bump
// instead of the `Arc::new(env.clone())` the borrowed entry pays at
// its first transition.
pub(crate) fn eval_arc_shared(expr: Arc<PnixExpr>, env: Arc<Env>) -> Result<Value> {
  eval_machine(expr, EnvRef::Shared(env))
}

/// R7 perf slice (2026-06-10): when an `Apply` argument is a plain
/// `Var` already lexically bound to a thunk, return that thunk to use
/// directly as the argument (laziness, cycle detection, and error
/// points are those of the inner thunk — identical to what forcing the
/// would-be wrapper observes). The four special global names are the
/// ones the Var arm short-circuits before `env.lookup`, so they never
/// resolve to a binding. Every other case returns `None` and the
/// caller builds the usual wrapper thunk.
fn arg_var_passthrough(arg: &PnixExpr, env: &Env) -> Option<Value> {
  let PnixExpr::Var(name) = arg else {
    return None;
  };
  if matches!(name.as_str(), "builtins" | "true" | "false" | "null") {
    return None;
  }
  env.var_thunk_passthrough(name)
}

/// Ex-1 (2026-06-11, SICP §5.4 explicit-control — design owner:
/// `project-wiki/maps/host-explicit-control-54-design-map.md`):
/// borrowed-or-shared env slot for the machine. `Borrowed` keeps the
/// caller's borrow, preserving R6's allocation-free property for
/// evals that never capture the env (pure Binary / Select / If-chain
/// trees materialize no `Arc<Env>` at all); `Shared` is the snapshot
/// Arc that transitions produce (R4) and that thunk-forcing callers
/// hand in directly via `eval_arc_shared`.
enum EnvRef<'a> {
  Borrowed(&'a Env),
  Shared(Arc<Env>),
}

impl<'a> EnvRef<'a> {
  #[inline]
  fn env(&self) -> &Env {
    match self {
      EnvRef::Borrowed(e) => e,
      EnvRef::Shared(e) => e,
    }
  }

  /// Owned snapshot Arc. `Borrowed` pays the one-time
  /// `Arc::new(env.clone())` exactly where the pre-Ex-1 owned loop
  /// paid it at entry; `Shared` is a refcount bump.
  #[inline]
  fn arc(&self) -> Arc<Env> {
    match self {
      EnvRef::Borrowed(e) => Arc::new((*e).clone()),
      EnvRef::Shared(e) => e.clone(),
    }
  }

  #[inline]
  fn cheap_clone(&self) -> EnvRef<'a> {
    match self {
      EnvRef::Borrowed(e) => EnvRef::Borrowed(e),
      EnvRef::Shared(e) => EnvRef::Shared(e.clone()),
    }
  }
}

/// Heap continuation for the explicit-control machine. Plain enum —
/// no `Rc<dyn Fn>` (the St-1/St-2 dyn-call cost lesson); fields are
/// Arc bumps / Value moves only. Each frame holds the `EvalDepthGuard`
/// charged for its child's evaluation: popping (or unwinding the
/// `Vec` on an early `?` return) drops the guard, so depth accounting
/// is balanced on every exit path exactly like the recursive
/// evaluator's RAII.
enum Frame<'a> {
  /// `if` condition pending; `node` re-borrowed for the branches.
  IfCond {
    node: Arc<PnixExpr>,
    env: EnvRef<'a>,
    charge: EvalDepthGuard,
  },
  /// Binary lhs pending; op/rhs re-borrowed from `node` at apply
  /// (covers the short-circuit forms too — the lhs verdict decides
  /// whether rhs is evaluated, preserving `&&`/`||`/`->` laziness
  /// and operand labels).
  BinLhs {
    node: Arc<PnixExpr>,
    env: EnvRef<'a>,
    charge: EvalDepthGuard,
  },
  /// Binary rhs pending; `lhs` already forced.
  BinRhs {
    node: Arc<PnixExpr>,
    lhs: Value,
    charge: EvalDepthGuard,
  },
  /// Select base pending; attr re-borrowed from `node` at apply.
  SelAttr {
    node: Arc<PnixExpr>,
    charge: EvalDepthGuard,
  },
  /// Apply func pending; arg expr + env kept for thunk construction.
  ApplyFunc {
    arg: Arc<PnixExpr>,
    env: EnvRef<'a>,
    charge: EvalDepthGuard,
  },
  /// Ex-2: a thunk's body is being evaluated (boundary forcing,
  /// inlined from `force_value` — semantics verbatim). On apply:
  /// release the cycle-guard entry, memoize, pass the forced value
  /// on. `_forcing`'s Drop also fires on unwind, balancing
  /// FORCING_THUNKS on the error path.
  ForceThunk {
    cache: Arc<std::sync::OnceLock<Value>>,
    _forcing: ForcingGuard,
    charge: EvalDepthGuard,
  },
  /// Ex-3: Unary operand pending; op re-borrowed from `node`.
  UnaryArg {
    node: Arc<PnixExpr>,
    charge: EvalDepthGuard,
  },
  /// Ex-3: `assert` condition pending; body re-borrowed from `node`
  /// and entered as a tail transition (the If-branch precedent).
  AssertCond {
    node: Arc<PnixExpr>,
    env: EnvRef<'a>,
    charge: EvalDepthGuard,
  },
  /// Ex-3: `?` base pending; the dotted-segment walk runs at apply
  /// (its inner force_value hops are data walks, not eval recursion).
  HasAttrBase {
    node: Arc<PnixExpr>,
    charge: EvalDepthGuard,
  },
}

enum Step<'a> {
  Eval(Arc<PnixExpr>, EnvRef<'a>),
  Return(Value),
}

/// The explicit-control evaluator (Ex-1). Generalizes the previous
/// trampoline: tail transitions (`if` branch taken / `let` body /
/// `Apply → Lambda` body) update the Eval state in place exactly as
/// the trampoline's loop did, and NON-tail continuations (if-cond,
/// Binary operands, Select base, Apply func) live as heap `Frame`s
/// instead of native `eval_arc` recursion — zero native stack growth
/// per `.px` eval level across the covered spine. Uncovered forms
/// fall back to `eval_inner`, whose child evals re-enter this machine
/// natively (hybrid: native depth grows only at uncovered-form
/// boundaries).
///
/// Depth-guard accounting is 1:1 with the recursive evaluator: the
/// machine entry charges one level (the old trampoline entry), and
/// every frame push charges one level for its child (the old
/// `eval_arc(child)` call), released when the frame pops — explicitly
/// dropped at the top of each apply arm so the charge dies before the
/// successor frame's charge, matching the old call/return boundary.
///
/// Every value crossing a frame boundary is forced first (the
/// `Step::Return` path below), matching the old trampoline's
/// `force_value` wrap on every `eval_arc` return — and the force runs
/// while the consuming frame's charge is still alive, same as forcing
/// inside the old child call.
///
/// Carried semantics: S13 Arc-expr transitions, R4 shared-Arc env,
/// R6 deferred env materialization (via `EnvRef::Borrowed`), R9
/// plain-Var callee fast path, R7 arg passthrough — comments at the
/// original sites apply unchanged.
fn eval_machine(expr: Arc<PnixExpr>, init_env: EnvRef<'_>) -> Result<Value> {
  // Recursion guard. Returns Err if this thread is already deeper
  // than EVAL_MAX_DEPTH eval frames (a `.px` program recursing
  // through uncovered forms / builtins can still grow native stack;
  // the structured Err keeps pnixc-meta workers alive).
  let _depth_guard = enter_eval()?;
  let mut frames: Vec<Frame> = Vec::new();
  let mut step = Step::Eval(expr, init_env);
  loop {
    step = match step {
      Step::Eval(e, env) => match e.as_ref() {
        PnixExpr::Null => Step::Return(Value::Null),
        PnixExpr::Bool(b) => Step::Return(Value::Bool(*b)),
        PnixExpr::Int(i) => Step::Return(Value::Int(*i)),
        PnixExpr::Float(f) => Step::Return(Value::Float(*f)),
        PnixExpr::String(s) => Step::Return(Value::String(s.clone())),
        PnixExpr::Var(name) => Step::Return(eval_var(name, env.env())?),
        PnixExpr::If { cond, .. } => {
          let cond = cond.clone();
          frames.push(Frame::IfCond {
            node: e,
            env: env.cheap_clone(),
            charge: enter_eval()?,
          });
          Step::Eval(cond, env)
        }
        PnixExpr::Binary { lhs, .. } => {
          let lhs = lhs.clone();
          frames.push(Frame::BinLhs {
            node: e,
            env: env.cheap_clone(),
            charge: enter_eval()?,
          });
          Step::Eval(lhs, env)
        }
        PnixExpr::Select { base, attr } => 'sel: {
          if let PnixExpr::Var(base_name) = base.as_ref() {
            if base_name == "builtins" {
              if let Some(value) = fast_builtin_attr_value(attr) {
                record_builtins_select_fast_path();
                break 'sel Step::Return(value);
              }
            }
            // R5 perf slice (2026-06-10): `m.field` on an already-
            // forced binding clones only the selected field. Special
            // global names and every needs-evaluation case (`None`)
            // fall through to the generic path unchanged.
            if !matches!(base_name.as_str(), "builtins" | "true" | "false" | "null") {
              if let Some(found) = env.env().select_attr_borrowed(base_name, attr) {
                break 'sel Step::Return(select_borrowed_to_result(found, attr)?);
              }
            }
          }
          let base = base.clone();
          frames.push(Frame::SelAttr {
            node: e,
            charge: enter_eval()?,
          });
          Step::Eval(base, env)
        }
        // R4: lambda values capture the shared snapshot Arc directly
        // (`EnvRef::arc` is a bump when one exists).
        PnixExpr::Lambda { param, body } => Step::Return(Value::Lambda {
          param: Arc::new(param.clone()),
          body: body.clone(),
          env: env.arc(),
        }),
        PnixExpr::Let { bindings, body } => {
          check_let_no_duplicates(bindings)?;
          let mut local_env =
            Env::with_recursive_bindings(env.env(), recursive_let_bindings(bindings));
          for binding in bindings {
            match binding {
              PnixLetBinding::Binding { pattern, value, .. } => match pattern {
                PnixParamPattern::Ident(_) => {}
                _ => {
                  let val = eval_arc(value.clone(), &local_env)?;
                  bind_pattern(&mut local_env, pattern, &val)?;
                }
              },
              PnixLetBinding::Inherit { names, from, .. } => {
                if let Some(from_expr) = from {
                  // Lazy: `let inherit (e) x; in body` is sugar for
                  // `let x = e.x; in body`; if `body` doesn't touch
                  // `x`, neither `e` nor `e.x` should evaluate.
                  for name in names {
                    let select_expr = PnixExpr::Select {
                      base: from_expr.clone(),
                      attr: name.clone(),
                    };
                    local_env.bind(
                      name.clone(),
                      make_thunk_arc(Arc::new(select_expr), &local_env),
                    );
                  }
                } else {
                  for name in names {
                    if let Some(v) = local_env.lookup(name)? {
                      local_env.bind(name.clone(), v);
                    }
                  }
                }
              }
            }
          }
          Step::Eval(body.clone(), EnvRef::Shared(Arc::new(local_env)))
        }
        PnixExpr::Apply { func, arg } => 'app: {
          // R9 perf slice (2026-06-10): plain-Var callee fast path.
          // For an Ident-pattern lambda the call only needs (param
          // name, body, captured env) — extract those borrowed.
          // `maybe_apply_lazy_builtin` is skipped safely: it matches
          // BuiltinPartial only, never Lambda. Every fallback case
          // (`None`) takes the generic path unchanged.
          if let PnixExpr::Var(func_name) = func.as_ref() {
            if !matches!(func_name.as_str(), "builtins" | "true" | "false" | "null") {
              if let Some((param_name, body, lam_env)) =
                env.env().var_lambda_ident_call_parts(func_name)
              {
                let arg_thunk = match arg_var_passthrough(arg, env.env()) {
                  Some(existing) => existing,
                  None => make_thunk_shared_env(arg.clone(), env.arc(), None),
                };
                let mut new_env = Env::with_parent_arc_capacity(lam_env, 1);
                new_env.bind(param_name, arg_thunk);
                break 'app Step::Eval(body, EnvRef::Shared(Arc::new(new_env)));
              }
            }
          }
          let func = func.clone();
          frames.push(Frame::ApplyFunc {
            arg: arg.clone(),
            env: env.cheap_clone(),
            charge: enter_eval()?,
          });
          Step::Eval(func, env)
        }
        PnixExpr::Unary { arg, .. } => {
          let arg = arg.clone();
          frames.push(Frame::UnaryArg {
            node: e,
            charge: enter_eval()?,
          });
          Step::Eval(arg, env)
        }
        // Ex-3: `with` is a pure env transition — the source is NOT
        // evaluated here (laziness: `with throw "boom"; 1` is 1); the
        // first body lookup that falls through to the with-frame
        // forces it. Tail like Let.
        PnixExpr::With {
          env: with_env,
          body,
        } => {
          let local_env = Env::with_with_attrs(env.env(), with_env.clone(), env.env().clone());
          Step::Eval(body.clone(), EnvRef::Shared(Arc::new(local_env)))
        }
        PnixExpr::Assert { cond, .. } => {
          let cond = cond.clone();
          frames.push(Frame::AssertCond {
            node: e,
            env: env.cheap_clone(),
            charge: enter_eval()?,
          });
          Step::Eval(cond, env)
        }
        PnixExpr::HasAttr { base, attr } => 'has: {
          if let PnixExpr::Var(base_name) = base.as_ref() {
            if base_name == "builtins" && !attr.contains('.') && fast_builtin_attr_exists(attr) {
              record_builtins_has_attr_fast_path();
              break 'has Step::Return(Value::Bool(true));
            }
          }
          let base = base.clone();
          frames.push(Frame::HasAttrBase {
            node: e,
            charge: enter_eval()?,
          });
          Step::Eval(base, env)
        }
        // Uncovered forms: interpreted path unchanged (`eval_inner`
        // child evals re-enter this machine natively — native depth
        // grows only at these boundaries). Deliberately native:
        // SelectOrDefault and StringInterp carry Err-CATCH semantics
        // (any-failure → default / unresolved-placeholder recovery) —
        // the machine's unwind is propagate-only, and a catch frame
        // would tax every Err path; their native depth is bounded by
        // syntax nesting, not data recursion. Index/Match/Dynamic*
        // are rare; Import is cold.
        _ => Step::Return(eval_inner(&e, env.env())?),
      },
      Step::Return(raw) => {
        // Boundary force: every child crossing the old `eval_arc`
        // boundary returned forced, with the force running under
        // that child's depth charge — the consuming frame's charge
        // is still alive here, matching. Ex-2: thunk forcing is
        // inlined as a ForceThunk frame (semantics verbatim from
        // `force_value`) instead of a native `force_value` →
        // machine re-entry per thunk hop — force chains no longer
        // grow native stack, and each hop skips a full machine
        // entry. The depth charge on the frame matches the machine
        // entry the old per-hop `eval_arc_shared` call paid.
        if let Value::Thunk {
          expr,
          env,
          cache,
          attr_pos: _,
        } = raw
        {
          if let Some(cached) = cache.get().cloned() {
            Step::Return(cached)
          } else {
            // Cycle guard: refuse to recurse into a thunk we're
            // already evaluating on this thread (frame stack or an
            // enclosing native force_value). Compare by Arc::ptr_eq
            // so unrelated thunks that happened to reuse an
            // allocator slot don't false-positive.
            let already_forcing =
              FORCING_THUNKS.with(|stack| stack.borrow().iter().any(|c| Arc::ptr_eq(c, &cache)));
            if already_forcing {
              return Err(anyhow!("infinite recursion encountered"));
            }
            frames.push(Frame::ForceThunk {
              cache: cache.clone(),
              charge: enter_eval()?,
              _forcing: ForcingGuard::push(cache),
            });
            Step::Eval(expr, EnvRef::Shared(env))
          }
        } else {
          let v = raw;
          match frames.pop() {
            None => return Ok(v),
            Some(Frame::ForceThunk {
              cache,
              _forcing,
              charge,
            }) => {
              drop(charge);
              drop(_forcing);
              // OnceLock::set returns Err if already set (race with
              // another thread also forcing this thunk). Either way
              // the cache now holds a valid Value; ignore the race
              // outcome — verbatim from force_value.
              let _ = cache.set(v.clone());
              Step::Return(v)
            }
            Some(Frame::UnaryArg { node, charge }) => {
              drop(charge);
              let PnixExpr::Unary { op, .. } = node.as_ref() else {
                unreachable!("UnaryArg frame holds a non-Unary node")
              };
              Step::Return(eval_unary(op, &v)?)
            }
            Some(Frame::AssertCond { node, env, charge }) => {
              drop(charge);
              let PnixExpr::Assert { body, .. } = node.as_ref() else {
                unreachable!("AssertCond frame holds a non-Assert node")
              };
              match v {
                Value::Bool(true) => Step::Eval(body.clone(), env),
                Value::Bool(false) => return Err(anyhow!("assertion failed")),
                other => {
                  return Err(anyhow!(
                    "assert: condition must be bool, got {}",
                    type_name(&other)
                  ))
                }
              }
            }
            Some(Frame::HasAttrBase { node, charge }) => {
              drop(charge);
              let PnixExpr::HasAttr { attr, .. } = node.as_ref() else {
                unreachable!("HasAttrBase frame holds a non-HasAttr node")
              };
              // Dotted-path walk verbatim from the eval_inner arm; the
              // per-segment force hops are data walks (thunk caches),
              // not eval recursion.
              let mut current = v;
              let mut present = true;
              for segment in attr.split('.') {
                current = force_value(current)?;
                match current {
                  Value::AttrSet(ref map) => match map.get(segment) {
                    Some(next) => current = next.clone(),
                    None => {
                      present = false;
                      break;
                    }
                  },
                  _ => {
                    present = false;
                    break;
                  }
                }
              }
              Step::Return(Value::Bool(present))
            }
            Some(Frame::IfCond { node, env, charge }) => {
              drop(charge);
              let PnixExpr::If { then_, else_, .. } = node.as_ref() else {
                unreachable!("IfCond frame holds a non-If node")
              };
              if expect_bool(&v, "if condition")? {
                Step::Eval(then_.clone(), env)
              } else {
                Step::Eval(else_.clone(), env)
              }
            }
            Some(Frame::BinLhs { node, env, charge }) => {
              drop(charge);
              let PnixExpr::Binary { op, rhs, .. } = node.as_ref() else {
                unreachable!("BinLhs frame holds a non-Binary node")
              };
              // Short-circuit logical operators: lhs decides whether
              // rhs is evaluated (operand labels verbatim from the
              // eval_inner arm).
              match op.as_ref() {
                "&&" => {
                  if !expect_bool(&v, "&&: left operand")? {
                    Step::Return(Value::Bool(false))
                  } else {
                    let rhs = rhs.clone();
                    frames.push(Frame::BinRhs {
                      node: node.clone(),
                      lhs: v,
                      charge: enter_eval()?,
                    });
                    Step::Eval(rhs, env)
                  }
                }
                "||" => {
                  if expect_bool(&v, "||: left operand")? {
                    Step::Return(Value::Bool(true))
                  } else {
                    let rhs = rhs.clone();
                    frames.push(Frame::BinRhs {
                      node: node.clone(),
                      lhs: v,
                      charge: enter_eval()?,
                    });
                    Step::Eval(rhs, env)
                  }
                }
                "->" => {
                  if !expect_bool(&v, "->: left operand")? {
                    Step::Return(Value::Bool(true))
                  } else {
                    let rhs = rhs.clone();
                    frames.push(Frame::BinRhs {
                      node: node.clone(),
                      lhs: v,
                      charge: enter_eval()?,
                    });
                    Step::Eval(rhs, env)
                  }
                }
                _ => {
                  let rhs = rhs.clone();
                  frames.push(Frame::BinRhs {
                    node: node.clone(),
                    lhs: v,
                    charge: enter_eval()?,
                  });
                  Step::Eval(rhs, env)
                }
              }
            }
            Some(Frame::BinRhs { node, lhs, charge }) => {
              drop(charge);
              let PnixExpr::Binary { op, .. } = node.as_ref() else {
                unreachable!("BinRhs frame holds a non-Binary node")
              };
              match op.as_ref() {
                "&&" => Step::Return(Value::Bool(expect_bool(&v, "&&: right operand")?)),
                "||" => Step::Return(Value::Bool(expect_bool(&v, "||: right operand")?)),
                "->" => Step::Return(Value::Bool(expect_bool(&v, "->: right operand")?)),
                _ => Step::Return(eval_binary(op, &lhs, &v)?),
              }
            }
            Some(Frame::SelAttr { node, charge }) => {
              drop(charge);
              let PnixExpr::Select { attr, .. } = node.as_ref() else {
                unreachable!("SelAttr frame holds a non-Select node")
              };
              Step::Return(select_from_value(&v, attr)?)
            }
            Some(Frame::ApplyFunc { arg, env, charge }) => {
              drop(charge);
              let func_val = v;
              if let Some(result) = maybe_apply_lazy_builtin(&func_val, &arg, env.env()) {
                Step::Return(result?)
              } else {
                // Hot path: every function application. R7: a plain-Var
                // arg already bound to a thunk passes that thunk through
                // (no wrapper alloc); R4: the thunk's env side is a
                // refcount bump of the shared Arc when one exists.
                let arg_thunk = match arg_var_passthrough(&arg, env.env()) {
                  Some(existing) => existing,
                  None => make_thunk_shared_env(arg.clone(), env.arc(), None),
                };
                match func_val {
                  Value::Lambda {
                    param,
                    body,
                    env: lam_env,
                  } => {
                    // R3: the lambda's captured env is already an
                    // Arc<Env>; hand it straight through as the parent.
                    let mut new_env =
                      Env::with_parent_arc_capacity(lam_env, pattern_binding_capacity(&param));
                    bind_pattern(&mut new_env, &param, &arg_thunk)?;
                    Step::Eval(body, EnvRef::Shared(Arc::new(new_env)))
                  }
                  other => Step::Return(apply_value(other, arg_thunk)?),
                }
              }
            }
          }
        }
      }
    };
  }
}

fn string_interp_literal_capacity(parts: &[StringInterpPart]) -> usize {
  parts.iter().fold(0usize, |capacity, part| match part {
    StringInterpPart::Lit(lit) => capacity.saturating_add(lit.len()),
    StringInterpPart::Expr(_) => capacity,
  })
}

/// Select projection helpers, shared verbatim between `eval_inner`'s
/// Select arm and St-2 compiled subtrees (single source — the error
/// strings and outcome mapping live exactly once).
fn select_from_value(base_val: &Value, attr: &str) -> Result<Value> {
  match base_val {
    Value::AttrSet(map) => map
      .get(attr)
      .cloned()
      .ok_or_else(|| anyhow!("attribute '{}' not found", attr)),
    _ => Err(anyhow!("cannot select '{}' from non-attrset", attr)),
  }
}

fn select_borrowed_to_result(found: SelectBorrowed, attr: &str) -> Result<Value> {
  match found {
    SelectBorrowed::Attr(Some(v)) => Ok(v),
    SelectBorrowed::Attr(None) => Err(anyhow!("attribute '{}' not found", attr)),
    SelectBorrowed::NonAttrset => Err(anyhow!("cannot select '{}' from non-attrset", attr)),
  }
}

/// Var evaluation, shared verbatim between `eval_inner`'s Var arm and
/// St-1 compiled subtrees (single source — byte-identical semantics).
fn eval_var(name: &str, env: &Env) -> Result<Value> {
  if name == "builtins" {
    return Ok(builtins_attrset());
  }
  if name == "true" {
    return Ok(Value::Bool(true));
  }
  if name == "false" {
    return Ok(Value::Bool(false));
  }
  if name == "null" {
    return Ok(Value::Null);
  }
  // First, allow user bindings to shadow globals.
  if let Some(v) = env.lookup(name)? {
    return Ok(v);
  }
  // Nix exposes a fixed set of builtins as bare globals (in addition to
  // `builtins.<name>`). See nix/src/libexpr/eval.cc EvalState::initEnv.
  if global_builtin_alias(name) {
    return Ok(builtin_partial_value(name));
  }
  Err(anyhow!("undefined variable: {}", name))
}

fn eval_inner(expr: &PnixExpr, env: &Env) -> Result<Value> {
  match expr {
    PnixExpr::Null => Ok(Value::Null),
    PnixExpr::Bool(b) => Ok(Value::Bool(*b)),
    PnixExpr::Int(i) => Ok(Value::Int(*i)),
    PnixExpr::Float(f) => Ok(Value::Float(*f)),
    PnixExpr::String(s) => Ok(Value::String(s.clone())),
    PnixExpr::Path(p) => {
      use pnix_core::lang::pnix::syntax::{PnixPath, PnixPathBase, StringInterpPart};
      match p {
        // 2026-05-05 (slice #66): normalize Path at
        // construction. Real Nix normalizes paths at parse
        // time — `./a/../b` becomes `./b`, `/abs/x/../y`
        // becomes `/abs/y`. Pre-fix pnix preserved the
        // literal text, so `toString ./a/../b` returned
        // `"./a/../b"` (silently denormalized) and `dirOf
        // ./a/../b` returned `Path("./a/..")` (silently
        // wrong parent — should be `./` since the path's
        // semantic parent is `./b`'s parent which is `./`).
        // Slice #65 already normalized at comparison time;
        // this slice closes the symmetry by normalizing at
        // construction so every Path-typed value is
        // canonical from the start.
        PnixPath::Relative(s) | PnixPath::Absolute(s) | PnixPath::Search(s) | PnixPath::Home(s) => {
          Ok(Value::Path(normalize_pnix_path(&std::path::PathBuf::from(
            s,
          ))))
        }
        PnixPath::Interpolated { base, parts } => {
          // Concatenate part strings: literal segments verbatim,
          // interp parts coerced to string. Real Nix coerces each
          // sub-expression with the path coercion rule (`toString`
          // for strings/paths/numbers; attrset with `outPath` /
          // `__toString`); we route through the existing
          // string-interpolation coercion since paths-in-strings
          // need the same shape.
          let mut s = String::with_capacity(string_interp_literal_capacity(parts));
          for part in parts {
            match part {
              StringInterpPart::Lit(lit) => s.push_str(lit),
              StringInterpPart::Expr(e) => {
                let val = eval_arc(e.clone(), env)?;
                let coerced = coerce_to_string_for_interpolation(val)?;
                s.push_str(&coerced);
              }
            }
          }
          let p = match base {
            PnixPathBase::Relative => std::path::PathBuf::from(s),
            PnixPathBase::Absolute => std::path::PathBuf::from(s),
            PnixPathBase::Home => {
              if let Some(home) = home_dir_os() {
                let mut hp = std::path::PathBuf::from(home.as_os_str());
                // `s` starts with the part after `~`, e.g. "/foo".
                // Strip the leading `/` so PathBuf::push doesn't
                // treat it as absolute and discard `home`.
                let rest = s.trim_start_matches('/');
                if !rest.is_empty() {
                  hp.push(rest);
                }
                hp
              } else {
                std::path::PathBuf::from(format!("~{}", s))
              }
            }
          };
          // 2026-05-05 (slice #66): normalize interpolated
          // Path values too. Same rationale as the literal-
          // path case above.
          Ok(Value::Path(normalize_pnix_path(&p)))
        }
      }
    }

    PnixExpr::Var(name) => eval_var(name, env),

    PnixExpr::Let { bindings, body } => {
      // `recursive_let_bindings` already provides lazy on-demand evaluation
      // for simple Ident bindings via `RecursiveBindings::lookup`. We only
      // need to eagerly evaluate when the binding pattern is a destructuring
      // form (AttrSet / List / AttrSetWithBind), since those need the value
      // to bind sub-names.
      check_let_no_duplicates(bindings)?;
      let mut local_env = Env::with_recursive_bindings(env, recursive_let_bindings(bindings));
      for binding in bindings {
        match binding {
          PnixLetBinding::Binding { pattern, value, .. } => match pattern {
            PnixParamPattern::Ident(_) => {
              // Lazy: rely on RecursiveBindings::lookup. Do not force the value here.
            }
            _ => {
              let val = eval_arc(value.clone(), &local_env)?;
              bind_pattern(&mut local_env, pattern, &val)?;
            }
          },
          PnixLetBinding::Inherit { names, from, .. } => {
            if let Some(from_expr) = from {
              // Lazy mirror of the AttrSet inherit handler.
              for name in names {
                let select_expr = PnixExpr::Select {
                  base: from_expr.clone(),
                  attr: name.clone(),
                };
                local_env.bind(
                  name.clone(),
                  make_thunk_arc(Arc::new(select_expr), &local_env),
                );
              }
            } else {
              for name in names {
                if let Some(v) = local_env.lookup(name)? {
                  local_env.bind(name.clone(), v);
                }
              }
            }
          }
        }
      }
      eval_arc(body.clone(), &local_env)
    }

    PnixExpr::If { cond, then_, else_ } => {
      let c = eval_arc(cond.clone(), env)?;
      if expect_bool(&c, "if condition")? {
        eval_arc(then_.clone(), env)
      } else {
        eval_arc(else_.clone(), env)
      }
    }

    PnixExpr::Lambda { param, body } => Ok(Value::Lambda {
      param: Arc::new(param.clone()),
      body: body.clone(),
      env: Arc::new(env.clone()),
    }),

    PnixExpr::Apply { func, arg } => {
      // R9: same plain-Var Ident-lambda callee fast path as the
      // trampoline's Apply arm — see Env::var_lambda_ident_call_parts.
      if let PnixExpr::Var(func_name) = func.as_ref() {
        if !matches!(func_name.as_str(), "builtins" | "true" | "false" | "null") {
          if let Some((param_name, body, lam_env)) = env.var_lambda_ident_call_parts(func_name) {
            let arg_thunk = match arg_var_passthrough(arg, env) {
              Some(existing) => existing,
              None => make_thunk_arc(arg.clone(), env),
            };
            let mut new_env = Env::with_parent_arc_capacity(lam_env, 1);
            new_env.bind(param_name, arg_thunk);
            return eval_arc(body, &new_env);
          }
        }
      }
      let func_val = eval_arc(func.clone(), env)?;
      if let Some(result) = maybe_apply_lazy_builtin(&func_val, arg, env) {
        return result;
      }
      // Nix-compat: function arguments are passed *unforced*. This is
      // essential for the fix-point combinator and many overlay/extend
      // patterns where `f x` would otherwise re-enter the recursive
      // binding for `x` and trip cycle detection. Lambda body (or
      // builtin impl) forces the arg only when its value is actually
      // needed.
      // R7: same plain-Var thunk pass-through as the trampoline's
      // Apply arm — see Env::var_thunk_passthrough.
      let arg_thunk = match arg_var_passthrough(arg, env) {
        Some(existing) => existing,
        None => make_thunk_arc(arg.clone(), env),
      };
      apply_value(func_val, arg_thunk)
    }

    PnixExpr::AttrSet { items, recursive } => {
      // Each field becomes a `Value::Thunk` that captures `local_env` and the
      // field's expression. The thunk is forced only when the field is
      // selected. For recursive sets, `local_env` carries the
      // `RecursiveBindings` so a field's expression can reference siblings
      // without forcing them at construction time.
      let mut map = BTreeMap::new();
      let local_env = if *recursive {
        Env::with_recursive_bindings(env, recursive_attrset_bindings(items))
      } else {
        env.clone()
      };
      // R3 perf slice (2026-06-10): one shared Arc<Env> snapshot for
      // every field thunk of this literal (lazily created so empty /
      // thunk-free attrsets pay nothing). `local_env` is not mutated
      // during the field loop — fields land in `map`, not in the env —
      // so all field thunks observe the identical environment they
      // observed pre-slice, minus one Env clone + one Arc allocation
      // per field.
      let mut shared_env: Option<Arc<Env>> = None;
      for item in items {
        match item {
          PnixAttrItem::Assign {
            key_path,
            value,
            span,
          } => {
            // Nix-compat: multi-segment key path `a.b.c = v` merges with
            // sibling `a.x = w` into `a = { b.c = v; x = w; }`. Pnix had
            // collapsed key_path to its first segment, which silently
            // dropped intermediate attrsets and clobbered siblings.
            //
            // Carry the binding span through to the resulting thunk so
            // `builtins.unsafeGetAttrPos` can recover the source line
            // and column without re-parsing.
            let attr_pos = attr_source_pos_from_span(span);
            let env_arc = shared_env
              .get_or_insert_with(|| Arc::new(local_env.clone()))
              .clone();
            let leaf = make_thunk_shared_env(value.clone(), env_arc, attr_pos);
            insert_nested_attr(&mut map, key_path, leaf)?;
          }
          PnixAttrItem::Inherit { names, from, .. } => {
            // Nix-correct duplicate-attribute guard: a name already
            // bound by an earlier `Assign` in the same attrset
            // (e.g. `rec { x = 1; inherit x; }`) is a duplicate,
            // matching the error real Nix emits and matching what
            // plain `{ a = 1; a = 2; }` already does via
            // `insert_nested_attr`. Without this guard the inherit
            // clause silently shadowed the prior binding.
            if let Some(from_expr) = from {
              for name in names {
                if map.contains_key(name) {
                  return Err(anyhow!(
                    "attribute '{}' already defined at this level",
                    name
                  ));
                }
                let select_expr = PnixExpr::Select {
                  base: from_expr.clone(),
                  attr: name.clone(),
                };
                let env_arc = shared_env
                  .get_or_insert_with(|| Arc::new(local_env.clone()))
                  .clone();
                map.insert(
                  name.clone(),
                  make_thunk_shared_env(Arc::new(select_expr), env_arc, None),
                );
              }
            } else {
              let lookup_env = if *recursive { &local_env } else { env };
              for name in names {
                if map.contains_key(name) {
                  return Err(anyhow!(
                    "attribute '{}' already defined at this level",
                    name
                  ));
                }
                if let Some(v) = lookup_env.lookup(name)? {
                  map.insert(name.clone(), v);
                }
              }
            }
          }
          PnixAttrItem::DynamicAssign {
            key_path, value, ..
          } => {
            // Evaluate dynamic segments to strings, then merge into the
            // attrset like a normal nested-path assignment.
            let mut resolved_path: Vec<String> = Vec::with_capacity(key_path.len());
            for seg in key_path {
              match seg {
                pnix_core::lang::pnix::syntax::AttrKeySegment::Static(s) => {
                  resolved_path.push(s.clone());
                }
                pnix_core::lang::pnix::syntax::AttrKeySegment::Dynamic(expr) => {
                  let v = eval_arc(expr.clone(), &local_env)?;
                  // Nix-compat: a `null` dynamic segment makes the whole
                  // assignment vanish (used in conditional attrset extension).
                  if matches!(v, Value::Null) {
                    resolved_path.clear();
                    break;
                  }
                  let Some(s) = v.as_str() else {
                    return Err(anyhow!(
                      "dynamic attribute name must be a string or null, got {}",
                      type_name(&v)
                    ));
                  };
                  resolved_path.push(s.to_string());
                }
              }
            }
            if resolved_path.is_empty() {
              continue;
            }
            let env_arc = shared_env
              .get_or_insert_with(|| Arc::new(local_env.clone()))
              .clone();
            let leaf = make_thunk_shared_env(value.clone(), env_arc, None);
            insert_nested_attr(&mut map, &resolved_path, leaf)?;
          }
        }
      }
      Ok(Value::AttrSet(Arc::new(map)))
    }

    PnixExpr::List(items) => {
      // Each list element becomes a thunk; forced only when accessed via
      // head/tail/elemAt/index/iteration that needs the value.
      //
      // R3 perf slice (2026-06-10): one shared Arc<Env> snapshot for all
      // element thunks of this literal (the caller env is read-only
      // here), replacing one Env clone + one Arc allocation per element.
      if items.is_empty() {
        return Ok(Value::List(Arc::new(Vec::new())));
      }
      let env_arc = Arc::new(env.clone());
      let mut result = Vec::with_capacity(items.len());
      for item in items {
        result.push(make_thunk_shared_env(
          Arc::new(item.clone()),
          env_arc.clone(),
          None,
        ));
      }
      Ok(Value::List(Arc::new(result)))
    }

    PnixExpr::Select { base, attr } => {
      if let PnixExpr::Var(base_name) = base.as_ref() {
        if base_name == "builtins" {
          if let Some(value) = fast_builtin_attr_value(attr) {
            record_builtins_select_fast_path();
            return Ok(value);
          }
        }
        // R5 perf slice (2026-06-10): `m.field` on an already-forced
        // binding clones only the selected field instead of evaluating
        // the Var (whole-container clone via force_value cache) and
        // then keeping one field. The special global names are exactly
        // those the Var arm short-circuits before env lookup; for them
        // (and for every needs-evaluation case, signalled by `None`)
        // fall through to the generic path unchanged.
        if !matches!(base_name.as_str(), "builtins" | "true" | "false" | "null") {
          if let Some(found) = env.select_attr_borrowed(base_name, attr) {
            return select_borrowed_to_result(found, attr);
          }
        }
      }
      let base_val = eval_arc(base.clone(), env)?;
      select_from_value(&base_val, attr)
    }

    PnixExpr::SelectOrDefault {
      base,
      attr,
      default,
    } => {
      if let PnixExpr::Var(base_name) = base.as_ref() {
        if base_name == "builtins" {
          if let Some(value) = fast_builtin_attr_value(attr) {
            record_builtins_select_fast_path();
            return Ok(value);
          }
        }
        // R5 perf slice (2026-06-10): same borrow fast path as the
        // Select arm. Every non-hit projection outcome (missing attr,
        // non-attrset) maps to the default expression, matching the
        // generic arm's catch-all semantics; needs-evaluation cases
        // (`None`) fall through so a throwing base still reaches the
        // generic `Err(_) => default` handling.
        if !matches!(base_name.as_str(), "builtins" | "true" | "false" | "null") {
          if let Some(found) = env.select_attr_borrowed(base_name, attr) {
            return match found {
              SelectBorrowed::Attr(Some(v)) => Ok(v),
              _ => eval_arc(default.clone(), env),
            };
          }
        }
      }
      // Nix-compat: `e1.path or default` catches *any* failure in the
      // whole select chain (intermediate missing attributes, type
      // mismatch, etc.), not just a missing leaf. So `bs.bar.foo or
      // "x"` falls to `"x"` when `bs.bar` itself is missing — without
      // this, deeply chained `or` patterns blow up mid-walk.
      let base_val = match eval_arc(base.clone(), env) {
        Ok(v) => v,
        Err(_) => return eval_arc(default.clone(), env),
      };
      match &base_val {
        Value::AttrSet(map) => match map.get(attr) {
          Some(v) => Ok(v.clone()),
          None => eval_arc(default.clone(), env),
        },
        _ => eval_arc(default.clone(), env),
      }
    }

    PnixExpr::Binary { op, lhs, rhs } => {
      // Short-circuit logical operators: lhs decides whether rhs is evaluated.
      match op.as_ref() {
        "&&" => {
          let l = eval_arc(lhs.clone(), env)?;
          if !expect_bool(&l, "&&: left operand")? {
            return Ok(Value::Bool(false));
          }
          let r = eval_arc(rhs.clone(), env)?;
          return Ok(Value::Bool(expect_bool(&r, "&&: right operand")?));
        }
        "||" => {
          let l = eval_arc(lhs.clone(), env)?;
          if expect_bool(&l, "||: left operand")? {
            return Ok(Value::Bool(true));
          }
          let r = eval_arc(rhs.clone(), env)?;
          return Ok(Value::Bool(expect_bool(&r, "||: right operand")?));
        }
        "->" => {
          let l = eval_arc(lhs.clone(), env)?;
          if !expect_bool(&l, "->: left operand")? {
            return Ok(Value::Bool(true));
          }
          let r = eval_arc(rhs.clone(), env)?;
          return Ok(Value::Bool(expect_bool(&r, "->: right operand")?));
        }
        _ => {}
      }
      let l = eval_arc(lhs.clone(), env)?;
      let r = eval_arc(rhs.clone(), env)?;
      eval_binary(op, &l, &r)
    }

    PnixExpr::Unary { op, arg } => {
      let a = eval_arc(arg.clone(), env)?;
      eval_unary(op, &a)
    }

    PnixExpr::StringInterp(parts) => {
      let mut result = String::with_capacity(string_interp_literal_capacity(parts));
      let mut combined_context: BTreeSet<String> = BTreeSet::new();
      for part in parts {
        match part {
          StringInterpPart::Lit(s) => result.push_str(s),
          StringInterpPart::Expr(e) => match eval_arc(e.clone(), env) {
            Ok(val) => {
              // Aggregate string context from interpolated parts.
              // - `Value::String`: no context, contributes nothing.
              // - `Value::StringContext`: union its context.
              // - `Value::Path`: coerce-to-string contributes the path
              //   itself as a context element (general-purpose
              //   provenance — pnix is not nix-store, so no /nix/store
              //   hashing; the literal path is a stable identifier).
              if let Some(ctx) = val.string_context() {
                extend_string_context(&mut combined_context, ctx);
              }
              if let Value::Path(ref p) = val {
                combined_context.insert(path_display_string(p));
              }
              let coerced = coerce_to_string_for_interpolation(val)?;
              result.push_str(&coerced);
            }
            Err(err) => {
              if let Some(name) = unresolved_placeholder_name(e, &err) {
                result.push_str("${");
                result.push_str(name);
                result.push('}');
              } else {
                return Err(err);
              }
            }
          },
        }
      }
      Ok(Value::string_with_context(result, combined_context))
    }

    PnixExpr::Import { path } => {
      let path_val = eval_arc(path.clone(), env)?;
      let file_path = resolve_value_path(&path_val, "import")?;
      eval_file_at_path(&file_path)
    }

    PnixExpr::With {
      env: with_env,
      body,
    } => {
      // Nix-correct `with`: priority + laziness.
      //   priority: lexical bindings (let / lambda / rec) win over
      //     the `with` attrset; inner `with` wins over outer `with`.
      //     Enforced by `Env::lookup`'s two-pass walk.
      //   laziness: the `with` source is *not* evaluated here —
      //     `with throw "boom"; 1` must return `1`. The frame
      //     captures the source expression and its declaration
      //     env; the first body lookup that falls through to this
      //     frame forces the source and memoizes it.
      let local_env = Env::with_with_attrs(env, with_env.clone(), env.clone());
      eval_arc(body.clone(), &local_env)
    }

    PnixExpr::Assert { cond, body } => {
      // 2026-05-05: previously used `is_true()` which truthy-coerced
      // any non-`false`/`null`/`Int(0)` value, so `assert "yes"; 42`
      // silently returned `42` and `assert 0; 42` silently failed.
      // Real Nix errors with "value is a <type> while a Boolean was
      // expected" when the asserted condition is not a bool — that
      // shape catches authors who pipe `findFirst`/`tryEval`/
      // `attrByPath` results (which can be non-bool) into `assert`
      // without an explicit `!= null` / `== "ok"` step. Match Nix:
      // bool-only contract; non-bool is a typed error, not a silent
      // pass-through or coerced fail.
      let c = eval_arc(cond.clone(), env)?;
      match c {
        Value::Bool(true) => eval_arc(body.clone(), env),
        Value::Bool(false) => Err(anyhow!("assertion failed")),
        other => Err(anyhow!(
          "assert: condition must be bool, got {}",
          type_name(&other)
        )),
      }
    }

    PnixExpr::Index { base, index } => {
      let base_val = eval_arc(base.clone(), env)?;
      let idx_val = eval_arc(index.clone(), env)?;
      let i = idx_val.as_f64().map(|f| f as usize).unwrap_or(0);
      match &base_val {
        Value::List(items) if i < items.len() => Ok(items[i].clone()),
        _ => Ok(Value::Null),
      }
    }

    PnixExpr::HasAttr { base, attr } => {
      if let PnixExpr::Var(base_name) = base.as_ref() {
        if base_name == "builtins" && !attr.contains('.') && fast_builtin_attr_exists(attr) {
          record_builtins_has_attr_fast_path();
          return Ok(Value::Bool(true));
        }
      }
      // `attr` may be a dotted path joined with '.': "a.b.c".
      // Each segment must traverse into a nested attrset; force thunks
      // along the way so we see the underlying attrset shape.
      let base_val = eval_arc(base.clone(), env)?;
      let mut current = base_val;
      for segment in attr.split('.') {
        current = force_value(current)?;
        match current {
          Value::AttrSet(ref map) => match map.get(segment) {
            Some(v) => current = v.clone(),
            None => return Ok(Value::Bool(false)),
          },
          _ => return Ok(Value::Bool(false)),
        }
      }
      Ok(Value::Bool(true))
    }

    PnixExpr::DynamicHasAttr { base, attr_expr } => {
      // The parser concatenates dotted dynamic-paths into a single string
      // expression (e.g. `set ? "${a}".b` becomes attr_expr that evaluates
      // to "a.b"). To handle nested access we split on '.' and walk.
      let base_val = eval_arc(base.clone(), env)?;
      let attr_val = eval_arc(attr_expr.clone(), env)?;
      let attr_str = attr_val.as_str().unwrap_or("");
      let mut current = base_val;
      for segment in attr_str.split('.') {
        current = force_value(current)?;
        match current {
          Value::AttrSet(ref map) => match map.get(segment) {
            Some(v) => current = v.clone(),
            None => return Ok(Value::Bool(false)),
          },
          _ => return Ok(Value::Bool(false)),
        }
      }
      Ok(Value::Bool(true))
    }

    PnixExpr::DynamicSelect { base, attr_expr } => {
      // Same dotted-path handling as DynamicHasAttr — walk segments.
      let base_val = eval_arc(base.clone(), env)?;
      let attr_val = eval_arc(attr_expr.clone(), env)?;
      let attr_str = attr_val.as_str().unwrap_or("");
      let mut current = base_val;
      for segment in attr_str.split('.') {
        current = force_value(current)?;
        match current {
          Value::AttrSet(ref map) => match map.get(segment).cloned() {
            Some(v) => current = v,
            None => return Err(anyhow!("dynamic attr '{}' not found", segment)),
          },
          _ => return Err(anyhow!("cannot dynamic select from non-attrset")),
        }
      }
      Ok(current)
    }

    PnixExpr::DynamicSelectOrDefault {
      base,
      attr_expr,
      default,
    } => {
      let base_val = eval_arc(base.clone(), env)?;
      let attr_val = eval_arc(attr_expr.clone(), env)?;
      let attr_str = attr_val.as_str().unwrap_or("");
      match &base_val {
        Value::AttrSet(map) => match map.get(attr_str) {
          Some(v) => Ok(v.clone()),
          None => eval_arc(default.clone(), env),
        },
        _ => eval_arc(default.clone(), env),
      }
    }

    PnixExpr::Construct { variant, args } => {
      let mut evaluated = Vec::with_capacity(args.len());
      for a in args {
        evaluated.push(eval(a, env)?);
      }
      // ADT: { __variant = "Some"; __args = [...]; }
      let mut map = BTreeMap::new();
      map.insert("__variant".to_string(), Value::String(variant.clone()));
      map.insert("__args".to_string(), Value::List(Arc::new(evaluated)));
      Ok(Value::AttrSet(Arc::new(map)))
    }

    PnixExpr::Match { scrutinee, arms } => {
      let scrutinee_val = eval_arc(scrutinee.clone(), env)?;
      for arm in arms {
        let mut arm_env = Env::with_parent_capacity(env, 1);
        arm_env.bind("__matched__".to_string(), scrutinee_val.clone());
        // guard check
        if let Some(guard) = &arm.guard {
          let g = eval_arc(guard.clone(), &arm_env)?;
          if !expect_bool(&g, "match arm guard")? {
            continue;
          }
        }
        return eval_arc(arm.body.clone(), &arm_env);
      }
      Ok(Value::Null)
    }
  }
}

fn recursive_attrset_bindings(items: &[PnixAttrItem]) -> FxHashMap<String, Arc<PnixExpr>> {
  let mut bindings = fx_hashmap_with_capacity(items.len());
  for item in items {
    match item {
      PnixAttrItem::Assign {
        key_path, value, ..
      } => {
        if let Some(key) = key_path.first() {
          bindings.insert(key.clone(), value.clone());
        }
      }
      // Nix-compat: a `${"name"} = expr;` item with a literal-string
      // dynamic head also participates in the recursive scope. Pure
      // dynamic items whose head requires evaluation cannot, since the
      // recursive env is built before any field is forced.
      PnixAttrItem::DynamicAssign {
        key_path, value, ..
      } => {
        if let Some(first) = key_path.first() {
          if let pnix_core::lang::pnix::syntax::AttrKeySegment::Dynamic(expr) = first {
            if let PnixExpr::String(s) = expr.as_ref() {
              bindings.insert(s.clone(), value.clone());
            }
          } else if let pnix_core::lang::pnix::syntax::AttrKeySegment::Static(s) = first {
            bindings.insert(s.clone(), value.clone());
          }
        }
      }
      PnixAttrItem::Inherit { names, from, .. } => {
        // For `rec { inherit (s) a b; }`, each `a` / `b` is sugar
        // for `a = s.a; b = s.b;` and must participate in the
        // recursive scope. The previous implementation skipped
        // inherit clauses entirely, so a sibling binding like
        // `b = a + 1;` couldn't see `a`.
        //
        // We only handle the `inherit (from) ...` form here. The
        // `inherit name;` (no-from) form pulls a name from the
        // *outer* scope, which is the same name the rec scope
        // would shadow if we registered it — registering would
        // turn it into a self-cycle. The attrset evaluator's
        // direct path still binds those names by looking them up
        // in `env` (the outer scope), so we leave them alone.
        if let Some(from_expr) = from {
          for name in names {
            bindings.insert(
              name.clone(),
              Arc::new(PnixExpr::Select {
                base: from_expr.clone(),
                attr: name.clone(),
              }),
            );
          }
        }
      }
    }
  }
  bindings
}

// 2026-05-05: detect duplicate names introduced by a single
// `let` form. `recursive_let_bindings` uses `BTreeMap::insert`,
// which silently overwrote duplicates, so `let x = 1; x = 2; in x`
// produced `2`, `let inherit ({a=99;}) a; a = 1; in a` produced
// `99`, and `let inherit (X) a; inherit (Y) a; in a` produced
// the second inherit's value — every shape was a silent-pass.
// Real Nix rejects duplicates with "attribute '<name>' already
// defined" (the same shape attrset literals already used; pnix's
// attrset literal duplicate check has been in place since slice
// #5). Match Nix: the let form has the same one-binding-per-name
// contract, and a duplicate must surface as a typed error
// pinning the offending name. Names come from:
//   * `Binding { pattern: Ident(n), .. }` → `n`
//   * `Binding { pattern: AttrSetWithBind { bind_name, fields, .. } }`
//     → `bind_name` AND every `fields[i].name`
//   * `Binding { pattern: AttrSet { fields, .. } }` → every
//     `fields[i].name` (this is the `let { a, b } = expr; in ...`
//     destructuring form)
//   * `Binding { pattern: List(_) }` → not currently walked here;
//     list-pattern destructuring in `let` is rare and the inner
//     names are nested. If duplicate-name shapes are reported
//     against list-pattern lets, the walk can be extended.
//   * `Inherit { names, .. }` → every `names[i]`
fn check_let_no_duplicates(bindings: &[PnixLetBinding]) -> Result<()> {
  let mut seen: FxHashSet<String> = FxHashSet::default();
  let mut record = |name: &str| -> Result<()> {
    if !seen.insert(name.to_string()) {
      return Err(anyhow!(
        "let: name '{}' is defined more than once at this level",
        name
      ));
    }
    Ok(())
  };
  for binding in bindings {
    match binding {
      PnixLetBinding::Binding { pattern, .. } => match pattern {
        PnixParamPattern::Ident(n) => record(n)?,
        PnixParamPattern::AttrSetWithBind {
          bind_name, fields, ..
        } => {
          record(bind_name)?;
          for f in fields {
            record(&f.name)?;
          }
        }
        PnixParamPattern::AttrSet { fields, .. } => {
          for f in fields {
            record(&f.name)?;
          }
        }
        PnixParamPattern::List(_) => {
          // List-pattern destructuring in `let` is rare; skip
          // for now. Extending this walk if a duplicate-name
          // shape is reported against list-pattern lets is
          // straightforward.
        }
      },
      PnixLetBinding::Inherit { names, .. } => {
        for name in names {
          record(name)?;
        }
      }
    }
  }
  Ok(())
}

fn recursive_let_bindings(bindings: &[PnixLetBinding]) -> FxHashMap<String, Arc<PnixExpr>> {
  let mut exprs = fx_hashmap_with_capacity(bindings.len());
  for binding in bindings {
    if let PnixLetBinding::Binding { pattern, value, .. } = binding {
      match pattern {
        PnixParamPattern::Ident(name) => {
          exprs.insert(name.clone(), value.clone());
        }
        PnixParamPattern::AttrSetWithBind { bind_name, .. } => {
          exprs.insert(bind_name.clone(), value.clone());
        }
        PnixParamPattern::AttrSet { .. } | PnixParamPattern::List(_) => {}
      }
    }
  }
  exprs
}

fn is_undefined_variable_message(message: &str, name: &str) -> bool {
  const PREFIX: &str = "undefined variable: ";
  message.strip_prefix(PREFIX) == Some(name)
}

fn unresolved_placeholder_name<'a>(expr: &'a PnixExpr, err: &anyhow::Error) -> Option<&'a str> {
  let PnixExpr::Var(name) = expr else {
    return None;
  };
  let message = err.to_string();
  if is_undefined_variable_message(&message, name) {
    Some(name)
  } else {
    None
  }
}

const COERCE_INTERP_DEPTH_LIMIT: usize = 64;

/// Build a Nix-style fake store-path string for `${./path}`-style
/// interpolation. Real Nix copies the path into a content-addressed
/// store and emits `/nix/store/<32-base32-hash>-<basename>`. pnix has
/// no store, so we emit `/nix/store/<32-hex-from-abs-path-sha256>-<basename>`
/// — same shape, deterministic per absolute path string. The fake hash
/// is exactly 32 chars so downstream `baseNameOf` + `substring 33`
/// math (used by upstream `eval-okay-context.nix`) lines up with real
/// Nix.
fn fake_store_path_string(p: &Path) -> String {
  let abs = if p.is_absolute() {
    p.to_path_buf()
  } else {
    with_current_import_base(|base| match base {
      Some(base) => base.join(p),
      None => p.to_path_buf(),
    })
  };
  let basename = abs
    .file_name()
    .map(|s| s.to_string_lossy())
    .unwrap_or(Cow::Borrowed("unknown"));
  let mut hasher = Sha256::new();
  hasher.update(abs.to_string_lossy().as_bytes());
  let digest = hasher.finalize();
  let mut hex = String::with_capacity(32);
  for b in digest.iter().take(16) {
    hex.push(HEX_LOWER[(b >> 4) as usize] as char);
    hex.push(HEX_LOWER[(b & 0x0f) as usize] as char);
  }
  let mut out = String::with_capacity(11 + hex.len() + 1 + basename.len());
  out.push_str("/nix/store/");
  out.push_str(&hex);
  out.push('-');
  out.push_str(basename.as_ref());
  out
}

fn path_display_string(path: &Path) -> String {
  path.to_string_lossy().into_owned()
}

fn concat_strs(left: &str, right: &str) -> String {
  let mut out = String::with_capacity(left.len().saturating_add(right.len()));
  out.push_str(left);
  out.push_str(right);
  out
}

fn concat_path_display_with_str(path: &Path, suffix: &str) -> String {
  let prefix = path.to_string_lossy();
  let mut out = String::with_capacity(prefix.len().saturating_add(suffix.len()));
  out.push_str(prefix.as_ref());
  out.push_str(suffix);
  out
}

fn concat_str_with_path_display(prefix: &str, path: &Path) -> String {
  let suffix = path.to_string_lossy();
  let mut out = String::with_capacity(prefix.len().saturating_add(suffix.len()));
  out.push_str(prefix);
  out.push_str(suffix.as_ref());
  out
}

fn concat_path_displays(left: &Path, right: &Path) -> String {
  let left_text = left.to_string_lossy();
  let right_text = right.to_string_lossy();
  let mut out = String::with_capacity(left_text.len().saturating_add(right_text.len()));
  out.push_str(left_text.as_ref());
  out.push_str(right_text.as_ref());
  out
}

fn generic_closure_key_signature(value: &Value) -> String {
  match value {
    Value::Int(i) => tagged_display_signature("i:", i),
    Value::Float(f) => tagged_display_signature("f:", f),
    Value::Bool(b) => tagged_str_signature("b:", if *b { "true" } else { "false" }),
    Value::String(s) => tagged_str_signature("s:", s),
    Value::StringContext { text, .. } => tagged_str_signature("s:", text),
    Value::Path(p) => tagged_path_signature("p:", p),
    other => tagged_debug_signature("o:", other),
  }
}

fn tagged_str_signature(tag: &str, text: &str) -> String {
  let mut signature = String::with_capacity(tag.len() + text.len());
  signature.push_str(tag);
  signature.push_str(text);
  signature
}

fn prefixed_string(prefix: &str, suffix: &str) -> String {
  let mut text = String::with_capacity(prefix.len() + suffix.len());
  text.push_str(prefix);
  text.push_str(suffix);
  text
}

fn output_name_context_marker(name: &str, entry: &str) -> String {
  let mut marker = String::with_capacity(name.len() + entry.len() + 2);
  marker.push('!');
  marker.push_str(name);
  marker.push('!');
  marker.push_str(entry);
  marker
}

fn anchored_regex_pattern(pattern: &str) -> Arc<str> {
  const PREFIX: &str = "(?s)^(?:";
  const SUFFIX: &str = ")$";
  if pattern.len() <= REGEX_CACHE_MAX_PATTERN_BYTES {
    let lookup_started = cache_lookup_timing_started();
    let cached = MATCH_ANCHORED_PATTERN_CACHE.with(|cache| cache.borrow().get(pattern).cloned());
    record_cache_lookup_elapsed(lookup_started);
    if let Some(anchored) = cached {
      record_match_anchored_pattern_cache_hit();
      return anchored;
    }
  }
  record_match_anchored_pattern_cache_miss();
  let mut anchored = String::with_capacity(PREFIX.len() + pattern.len() + SUFFIX.len());
  anchored.push_str(PREFIX);
  anchored.push_str(pattern);
  anchored.push_str(SUFFIX);
  let anchored: Arc<str> = Arc::from(anchored);
  if pattern.len() <= REGEX_CACHE_MAX_PATTERN_BYTES {
    MATCH_ANCHORED_PATTERN_CACHE.with(|cache| {
      let mut cache = cache.borrow_mut();
      evict_one_cache_entry(&mut cache, MATCH_ANCHORED_PATTERN_CACHE_MAX_ENTRIES);
      cache.insert(pattern.to_string(), anchored.clone());
    });
  }
  anchored
}

fn to_file_store_name(hash_prefix: &str, safe_name: &str) -> String {
  const PREFIX: &str = "toFile-";
  let mut store_name =
    String::with_capacity(PREFIX.len() + hash_prefix.len() + 1 + safe_name.len());
  store_name.push_str(PREFIX);
  store_name.push_str(hash_prefix);
  store_name.push('-');
  store_name.push_str(safe_name);
  store_name
}

fn tagged_path_signature(tag: &str, path: &Path) -> String {
  let text = path.to_string_lossy();
  let mut signature = String::with_capacity(tag.len().saturating_add(text.len()));
  signature.push_str(tag);
  signature.push_str(text.as_ref());
  signature
}

fn tagged_display_signature<T: std::fmt::Display>(tag: &str, value: T) -> String {
  use std::fmt::Write;

  let mut signature = String::with_capacity(tag.len() + 24);
  signature.push_str(tag);
  let _ = write!(&mut signature, "{value}");
  signature
}

fn tagged_debug_signature<T: std::fmt::Debug>(tag: &str, value: T) -> String {
  use std::fmt::Write;

  let mut signature = String::with_capacity(tag.len() + 32);
  signature.push_str(tag);
  let _ = write!(&mut signature, "{value:?}");
  signature
}

fn coerce_to_string_for_interpolation(val: Value) -> Result<String> {
  // Cross-call cycle protection: a `__toString` that returns a
  // string containing `${self}` re-enters `eval` (via the
  // `StringInterp` branch), which calls back into this function
  // with a fresh `depth = 0`. The thread-local counter persists
  // across that re-entry, so the cycle is caught before the Rust
  // call stack overflows. See the `INTERP_COERCE_DEPTH` block
  // above for the full rationale.
  let depth_guard = InterpDepthGuard::enter()?;
  let result = coerce_to_string_for_interpolation_inner(val, 0);
  drop(depth_guard);
  result
}

/// `builtins.toString` coercion. Looser than `${...}` interpolation:
/// integers / floats / booleans / null / lists are all valid inputs.
/// Per the Nix manual:
///   - string             → unchanged
///   - path               → display form (NOT a fake store path —
///     `${./p}` interpolation goes through `fake_store_path_string`,
///     `toString ./p` does not)
///   - integer / float    → text representation
///   - boolean true       → "1"; boolean false → ""
///   - null               → ""
///   - list               → element-wise toString joined with spaces
///   - attrset __toString → invoke `(self: ...)` and re-coerce
///   - attrset outPath    → re-coerce the outPath value
///   - lambda / function  → error
/// Walk a value graph and reject any `Value::Float` that is not
/// finite (`+inf`, `-inf`, `NaN`). Used by `builtins.toJSON` so
/// non-representable floats surface as a clear
/// `<context>: cannot serialize float <value> as JSON` error
/// instead of silently flattening to `null`. Called recursively
/// through lists and attrsets; cycles end at the visited Thunk
/// guard in `to_json` / `deep_force` so we don't add a separate
/// cycle stack here.
/// Convert a `pnix_toml::Value` (parsed by `builtins.fromTOML`) to a
/// pnix `Value`. Mirrors `markup::json_to_value` shape for JSON.
/// TOML date/time types lower to a string (the canonical text form),
/// since pnix has no temporal type.
fn toml_to_value(v: &pnix_toml::Value) -> Value {
  match v {
    pnix_toml::Value::String(s) => Value::String(s.clone()),
    pnix_toml::Value::Integer(i) => Value::Int(*i),
    pnix_toml::Value::Float(f) => Value::Float(*f),
    pnix_toml::Value::Boolean(b) => Value::Bool(*b),
    pnix_toml::Value::Datetime(dt) => Value::String(dt.clone()),
    pnix_toml::Value::Array(items) => {
      let mut out = Vec::with_capacity(items.len());
      for item in items {
        out.push(toml_to_value(item));
      }
      Value::List(Arc::new(out))
    }
    pnix_toml::Value::Table(map) => {
      let mut out = BTreeMap::new();
      for (k, v) in map {
        out.insert(k.clone(), toml_to_value(v));
      }
      Value::AttrSet(Arc::new(out))
    }
  }
}

#[cfg(test)]
fn check_json_finite(val: &Value, context: &str) -> Result<()> {
  // 2026-05-05: cyclic structures (`let r = { a = r; }; in r`)
  // used to overflow the Rust call stack here — the recursive
  // descent would force the inner thunk, get the same AttrSet
  // back, descend into it again, etc. Real Nix errors with
  // "infinite recursion encountered" on `toJSON` of a cyclic
  // value. The cycle is detected by tracking the `Rc` cache
  // pointers of every `Thunk` we've already entered along the
  // current descent path, the same pattern `deep_force_visited`
  // uses (see commit aad08893 for the path-stack invariant
  // discussion). The path is per-call (not thread-local) so it
  // doesn't false-trigger on independent calls; a normal
  // (non-cyclic) shared subterm is visited once per descent.
  if value_is_deep_force_leaf(val) {
    return check_json_finite_leaf(val, context);
  }
  let mut path: Vec<Arc<std::sync::OnceLock<Value>>> = Vec::with_capacity(8);
  check_json_finite_visited(val, context, &mut path, None)
}

fn check_json_finite_and_collect_contexts(val: &Value, context: &str) -> Result<BTreeSet<String>> {
  let mut collected = BTreeSet::new();
  if value_is_deep_force_leaf(val) {
    check_json_finite_leaf(val, context)?;
    collect_json_context_leaf(val, &mut collected);
    return Ok(collected);
  }
  let mut path: Vec<Arc<std::sync::OnceLock<Value>>> = Vec::with_capacity(8);
  check_json_finite_visited(val, context, &mut path, Some(&mut collected))?;
  Ok(collected)
}

fn check_json_finite_leaf(val: &Value, context: &str) -> Result<()> {
  match val {
    Value::Float(f) if !f.is_finite() => Err(non_finite_json_float_error(*f, context)),
    Value::Lambda { .. } | Value::BuiltinPartial { .. } => Err(json_function_error(context)),
    _ => Ok(()),
  }
}

fn non_finite_json_float_error(value: f64, context: &str) -> anyhow::Error {
  let kind = if value.is_nan() {
    "NaN"
  } else if value.is_sign_positive() {
    "+inf"
  } else {
    "-inf"
  };
  anyhow!("{}: cannot serialize float {} as JSON", context, kind)
}

fn json_function_error(context: &str) -> anyhow::Error {
  anyhow!("{}: cannot serialize function as JSON", context)
}

fn collect_json_context_leaf(val: &Value, out: &mut BTreeSet<String>) {
  match val {
    Value::StringContext { context, .. } => {
      extend_string_context(out, context);
    }
    Value::Path(p) => {
      out.insert(path_display_string(p));
    }
    _ => {}
  }
}

fn check_json_finite_visited(
  val: &Value,
  context: &str,
  path: &mut Vec<Arc<std::sync::OnceLock<Value>>>,
  mut collected_contexts: Option<&mut BTreeSet<String>>,
) -> Result<()> {
  // If `val` is a Thunk, track its cache on the path BEFORE we
  // force it. The previous shape only tracked thunks at the
  // top-level call site; thunks reached via AttrSet / List
  // descent slipped past the Arc::ptr_eq check because we forced
  // them first and then dispatched on the forced shape. Mirror
  // `deep_force_visited`'s pattern: track-then-force-then-
  // recurse-on-forced, with `pushed` controlling pop-on-exit.
  if let Value::Thunk { cache, .. } = val {
    if path.iter().any(|c| Arc::ptr_eq(c, cache)) {
      return Err(anyhow!(
        "{}: infinite recursion encountered (cyclic value)",
        context
      ));
    }
    path.push(cache.clone());
    let forced = force_value(val.clone())?;
    let result =
      check_json_finite_visited(&forced, context, path, collected_contexts.as_deref_mut());
    path.pop();
    return result;
  }

  match val {
    Value::Float(f) => {
      if !f.is_finite() {
        Err(non_finite_json_float_error(*f, context))
      } else {
        Ok(())
      }
    }
    // 2026-05-05: previously `Value::to_json` silently flattened
    // lambdas / builtin partials to placeholder strings like
    // `"<lambda:x>"` / `"<builtin:foo>"`, so `builtins.toJSON
    // (x: x)` returned a string rather than erroring. Real Nix
    // errors with "cannot convert a function to JSON". Match that
    // shape and reject before `to_json` is reached.
    Value::Lambda { .. } | Value::BuiltinPartial { .. } => Err(json_function_error(context)),
    Value::StringContext { context, .. } => {
      if let Some(out) = collected_contexts.as_deref_mut() {
        extend_string_context(out, context);
      }
      Ok(())
    }
    Value::Path(p) => {
      if let Some(out) = collected_contexts.as_deref_mut() {
        out.insert(path_display_string(p));
      }
      Ok(())
    }
    Value::List(items) => {
      let mut r = Ok(());
      for item in items.iter() {
        if let e @ Err(_) =
          check_json_finite_visited(item, context, path, collected_contexts.as_deref_mut())
        {
          r = e;
          break;
        }
      }
      r
    }
    Value::AttrSet(map) => {
      let mut r = Ok(());
      for v in map.values() {
        if let e @ Err(_) =
          check_json_finite_visited(v, context, path, collected_contexts.as_deref_mut())
        {
          r = e;
          break;
        }
      }
      r
    }
    _ => Ok(()),
  }
}

// 2026-05-05 (slice #57): context-aware variant. Returns the
// coerced text AND the union of every context-bearing piece's
// context. Pre-fix `coerce_to_string_for_to_string` only
// returned text — `Value::List` and the `__toString` /
// `outPath` recursive paths silently dropped the inner
// element contexts. Real Nix's `toString` propagates the
// context union (mirrors `${value}` interpolation semantics).
//
// The fix matters for `builtins.toString` of a list of
// derivation references. Pre-fix
// `toString [ "a${./p1}" "b${./p2}" ]` produced `"a... b..."`
// with empty context — silent metadata loss. Now the
// resulting string carries `{./p1, ./p2}` so downstream
// consumers (`hashString`, `getContext`, derivation
// realization) see the correct dependencies.
fn coerce_to_string_for_to_string_with_context(
  val: Value,
  depth: usize,
) -> Result<(String, BTreeSet<String>)> {
  if depth > COERCE_INTERP_DEPTH_LIMIT {
    return Err(anyhow!(
      "toString coercion depth exceeded (max: {})",
      COERCE_INTERP_DEPTH_LIMIT
    ));
  }
  match val {
    Value::String(s) => Ok((s, BTreeSet::new())),
    Value::StringContext { text, context } => Ok((text, context)),
    // Path arm: pnix's `Path` doesn't carry string context, but
    // the path is a build-time dependency marker when it appears
    // inside a string-coercion. Match `${./p}` interpolation
    // semantics — the path's display form is added to the
    // result context. Slice #54 established the same pattern
    // for `string + path`; slice #57 extends it to list
    // coercion via `toString`.
    Value::Path(p) => {
      let path_text = path_display_string(&p);
      let mut ctx = BTreeSet::new();
      ctx.insert(path_text.clone());
      Ok((path_text, ctx))
    }
    Value::Int(i) => Ok((i.to_string(), BTreeSet::new())),
    Value::Float(f) => Ok((f.to_string(), BTreeSet::new())),
    Value::Bool(true) => Ok(("1".to_string(), BTreeSet::new())),
    Value::Bool(false) => Ok((String::new(), BTreeSet::new())),
    Value::Null => Ok((String::new(), BTreeSet::new())),
    Value::List(items) => {
      let mut out = String::with_capacity(items.len().saturating_sub(1));
      let mut combined_ctx: BTreeSet<String> = BTreeSet::new();
      for (idx, item) in Arc::unwrap_or_clone(items).into_iter().enumerate() {
        let forced = force_value(item)?;
        let (text, ctx) = coerce_to_string_for_to_string_with_context(forced, depth + 1)?;
        if idx > 0 {
          out.push(' ');
        }
        out.push_str(&text);
        combined_ctx.extend(ctx);
      }
      Ok((out, combined_ctx))
    }
    Value::AttrSet(map) => {
      if let Some(to_string_fn) = map.get("__toString").cloned() {
        let to_string_fn = force_value(to_string_fn)?;
        let self_value = Value::AttrSet(map.clone());
        let coerced = apply_value(to_string_fn, self_value)?;
        return coerce_to_string_for_to_string_with_context(coerced, depth + 1);
      }
      if let Some(out_path) = map.get("outPath").cloned() {
        let out_path = force_value(out_path)?;
        return coerce_to_string_for_to_string_with_context(out_path, depth + 1);
      }
      Err(anyhow!(
        "cannot coerce a set to a string with toString: \
         set has neither '__toString' nor 'outPath' attribute"
      ))
    }
    Value::Lambda { .. } | Value::BuiltinPartial { .. } => Err(anyhow!(
      "cannot coerce a function to a string with toString"
    )),
    Value::Thunk { .. } => {
      let forced = force_value(val)?;
      coerce_to_string_for_to_string_with_context(forced, depth + 1)
    }
  }
}

fn coerce_to_string_for_interpolation_inner(val: Value, depth: usize) -> Result<String> {
  if depth > COERCE_INTERP_DEPTH_LIMIT {
    return Err(anyhow!(
      "string interpolation coercion depth exceeded (max: {})",
      COERCE_INTERP_DEPTH_LIMIT
    ));
  }
  match val {
    Value::String(s) => Ok(s),
    // Context-bearing strings coerce to their text. Context aggregation
    // is handled by the StringInterp eval branch (which builds the
    // outgoing string's context from all interpolated parts).
    Value::StringContext { text, .. } => Ok(text),
    // Interpolating a path coerces to a Nix-style fake store path
    // (`/nix/store/<32-char-hex>-<basename>`). Two reasons:
    // (1) pnix is general-purpose, not nix-store, but the shape of the
    //     emitted string must match real Nix so that downstream
    //     `baseNameOf` / `substring` math (used by the upstream
    //     `eval-okay-context.nix` parity test) lines up;
    // (2) `${./path}` semantics in real Nix include "copy to store and
    //     interpolate the store path"; we keep that surface deterministic
    //     by hashing the absolute path string instead of the file
    //     contents, since pnix doesn't have a content-addressed store.
    // Other Path consumers (`import`, `readFile`, etc.) keep working on
    // the underlying `Value::Path`; only the interpolation surface flips
    // to the fake-store form.
    Value::Path(p) => Ok(fake_store_path_string(&p)),
    Value::AttrSet(map) => {
      if let Some(to_string_fn) = map.get("__toString").cloned() {
        let to_string_fn = force_value(to_string_fn)?;
        let self_value = Value::AttrSet(map.clone());
        let coerced = apply_value(to_string_fn, self_value)?;
        return coerce_to_string_for_interpolation_inner(coerced, depth + 1);
      }
      if let Some(out_path) = map.get("outPath").cloned() {
        let out_path = force_value(out_path)?;
        return coerce_to_string_for_interpolation_inner(out_path, depth + 1);
      }
      Err(anyhow!(
        "cannot coerce a set to a string in interpolation: \
         set has neither '__toString' nor 'outPath' attribute"
      ))
    }
    Value::Int(_) => Err(anyhow!(
      "cannot coerce an integer to a string in interpolation: \
       use builtins.toString to convert"
    )),
    Value::Float(_) => Err(anyhow!(
      "cannot coerce a float to a string in interpolation: \
       use builtins.toString to convert"
    )),
    Value::Bool(_) => Err(anyhow!(
      "cannot coerce a boolean to a string in interpolation: \
       use builtins.toString to convert"
    )),
    Value::Null => Err(anyhow!("cannot coerce null to a string in interpolation")),
    Value::List(_) => Err(anyhow!(
      "cannot coerce a list to a string in interpolation: \
       use builtins.concatStringsSep or map to a string list first"
    )),
    Value::Lambda { .. } | Value::BuiltinPartial { .. } => Err(anyhow!(
      "cannot coerce a function to a string in interpolation"
    )),
    Value::Thunk {
      expr,
      env,
      cache,
      attr_pos,
    } => {
      let forced = force_value(Value::Thunk {
        expr,
        env,
        cache,
        attr_pos,
      })?;
      coerce_to_string_for_interpolation_inner(forced, depth + 1)
    }
  }
}

fn getenv_allowed(name: &str) -> bool {
  if name.starts_with("PNIX_") || name.starts_with("HYPNIX_") {
    return true;
  }
  getenv_allow_list().iter().any(|item| item == name)
}

fn getenv_allow_list() -> &'static [String] {
  static GETENV_ALLOW_LIST: OnceLock<Vec<String>> = OnceLock::new();
  GETENV_ALLOW_LIST.get_or_init(|| {
    let Ok(raw) = std::env::var("PNIX_GETENV_ALLOW") else {
      return Vec::new();
    };
    raw
      .split(',')
      .map(str::trim)
      .filter(|item| !item.is_empty())
      .map(str::to_string)
      .collect()
  })
}

fn bind_pattern(env: &mut Env, pattern: &PnixParamPattern, value: &Value) -> Result<()> {
  match pattern {
    // Ident pattern is structural — it does not need to peek inside the
    // value, so we keep `value` lazy. This is what makes the fix-point
    // combinator work: `f x` passes `x` as a thunk to `f`, and `f`
    // binds it to `self` without forcing.
    PnixParamPattern::Ident(name) => {
      env.bind(name.clone(), value.clone());
      Ok(())
    }
    // Destructuring patterns must inspect the structure (attrset keys
    // / list arity), so the value has to be forced first.
    PnixParamPattern::AttrSet { fields, ellipsis } => {
      // 2026-05-05: pattern validity is checked BEFORE forcing the
      // argument. Nix-correct order: a malformed pattern (duplicate
      // field name) is the lambda-author's bug, independent of what
      // the caller passes. So `({ a, a }: a) (throw "x")` should
      // surface the duplicate-formal error, not the throw.
      check_attrset_pattern_no_dup(None, fields)?;
      let forced = force_value(value.clone())?;
      bind_attrset_pattern(env, None, fields, *ellipsis, &forced)
    }
    PnixParamPattern::AttrSetWithBind {
      bind_name,
      fields,
      ellipsis,
    } => {
      check_attrset_pattern_no_dup(Some(bind_name.as_str()), fields)?;
      let forced = force_value(value.clone())?;
      bind_attrset_pattern(env, Some(bind_name.as_str()), fields, *ellipsis, &forced)
    }
    PnixParamPattern::List(pattern) => {
      let forced = force_value(value.clone())?;
      bind_list_pattern(env, pattern, &forced)
    }
  }
}

fn pattern_binding_capacity(pattern: &PnixParamPattern) -> usize {
  match pattern {
    PnixParamPattern::Ident(_) => 1,
    PnixParamPattern::AttrSet { fields, .. } => fields.len(),
    PnixParamPattern::AttrSetWithBind { fields, .. } => fields.len() + 1,
    PnixParamPattern::List(pattern) => pattern.items.len() + usize::from(pattern.tail.is_some()),
  }
}

fn check_attrset_pattern_no_dup(
  bind_name: Option<&str>,
  fields: &[PnixPatternField],
) -> Result<()> {
  let mut seen: FxHashSet<&str> = FxHashSet::default();
  if let Some(bn) = bind_name {
    seen.insert(bn);
  }
  for field in fields {
    if !seen.insert(field.name.as_str()) {
      return Err(anyhow!(
        "duplicate formal function argument '{}'",
        field.name
      ));
    }
  }
  Ok(())
}

fn bind_attrset_pattern(
  env: &mut Env,
  bind_name: Option<&str>,
  fields: &[PnixPatternField],
  ellipsis: bool,
  value: &Value,
) -> Result<()> {
  // Pattern duplicate-name check runs in `bind_pattern` BEFORE
  // the argument is forced, so we don't repeat it here.
  let Value::AttrSet(map) = value else {
    return Err(anyhow!("attrset pattern expected attrset, got {}", value));
  };

  if let Some(bind_name) = bind_name {
    env.bind(bind_name.to_string(), value.clone());
  }

  let mut expected: BTreeSet<&str> = BTreeSet::new();
  for field in fields {
    expected.insert(field.name.as_str());
  }

  // Pass 1: bind every field provided in the actual argument value. We do
  // this first so that field defaults can reference siblings (e.g.
  // `{ x ? y, y ? x }` — Nix-compat: defaults are thunks resolved against
  // the fully-populated lambda scope, not the declaration order).
  for field in fields {
    if let Some(v) = map.get(&field.name) {
      env.bind(field.name.clone(), v.clone());
    }
  }

  // Pass 2: for fields missing from the argument, bind the default as a
  // lazy thunk in the same scope. Forcing it later goes through the env
  // populated by pass 1, so cross-references resolve.
  for field in fields {
    if map.contains_key(&field.name) {
      continue;
    }
    if let Some(default_expr) = &field.default {
      let thunk = make_thunk(default_expr, env);
      env.bind(field.name.clone(), thunk);
      continue;
    }
    return Err(anyhow!("missing required attribute '{}'", field.name));
  }

  if !ellipsis {
    for key in map.keys() {
      if !expected.contains(key.as_str()) {
        return Err(anyhow!("unexpected attribute '{}'", key));
      }
    }
  }

  Ok(())
}

fn bind_list_pattern(env: &mut Env, pattern: &PnixListPattern, value: &Value) -> Result<()> {
  let Value::List(items) = value else {
    return Err(anyhow!("list pattern expected list, got {}", value));
  };

  if items.len() < pattern.items.len() {
    return Err(anyhow!(
      "list pattern expected at least {} items, got {}",
      pattern.items.len(),
      items.len()
    ));
  }
  if pattern.tail.is_none() && items.len() != pattern.items.len() {
    return Err(anyhow!(
      "list pattern expected exactly {} items, got {}",
      pattern.items.len(),
      items.len()
    ));
  }

  for (name, item) in pattern.items.iter().zip(items.iter()) {
    env.bind(name.clone(), item.clone());
  }
  if let Some(tail_name) = &pattern.tail {
    env.bind(
      tail_name.clone(),
      Value::List(Arc::new(items[pattern.items.len()..].to_vec())),
    );
  }

  Ok(())
}

fn apply_value(func: Value, arg: Value) -> Result<Value> {
  match func {
    Value::Lambda { param, body, env } => {
      // R3 perf slice (2026-06-10): reuse the lambda's captured
      // Arc<Env> as the parent directly (refcount bump) instead of
      // Env-clone + fresh Arc per application.
      let mut new_env = Env::with_parent_arc_capacity(env, pattern_binding_capacity(&param));
      bind_pattern(&mut new_env, &param, &arg)?;
      eval_arc(body.clone(), &new_env)
    }
    Value::BuiltinPartial { name, mut args } => {
      args.push(arg);
      if let Some(required) = hot_builtin_min_arity(name.as_ref()) {
        if args.len() < required {
          record_builtin_partial_arity_fast_path();
          return Ok(Value::BuiltinPartial { name, args });
        }
      }
      apply_builtin(&name, &args)
    }
    // Nix-compat: an attrset with a `__functor` attribute is callable —
    // `set arg` desugars to `set.__functor set arg`. Used heavily by
    // nixpkgs (lib.makeOverridable etc.).
    // R8 perf slice (2026-06-10): match the map by VALUE and move it
    // back into the self argument. The pre-fix `ref map` +
    // `func.clone()` deep-cloned the entire attrset (every key
    // String + every value subtree) on every functor application
    // just to hand the callee the same value we already owned.
    Value::AttrSet(map) => {
      if let Some(functor) = map.get("__functor").cloned() {
        let functor = force_value(functor)?;
        // First apply __functor to self (the attrset itself)
        let with_self = apply_value(functor, Value::AttrSet(map))?;
        // Then apply the result to the user-supplied argument
        return apply_value(with_self, arg);
      }
      Err(anyhow!(
        "cannot apply non-function value (attrset has no __functor)"
      ))
    }
    Value::Thunk { .. } => {
      let forced = force_value(func)?;
      apply_value(forced, arg)
    }
    _ => Err(anyhow!("cannot apply non-function value")),
  }
}

fn eval_binary(op: &str, l: &Value, r: &Value) -> Result<Value> {
  match op {
    // `Binary` short-circuit path in `eval_inner` already routes
    // `&&` / `||` / `->` through `expect_bool` before reaching
    // here, but `eval_binary` is also reachable when both
    // operands are pre-forced by other paths — keep the same
    // bool-only contract instead of falling back to truthy-coerce.
    "&&" => {
      return Ok(Value::Bool(
        expect_bool(l, "&&: left operand")? && expect_bool(r, "&&: right operand")?,
      ));
    }
    "||" => {
      return Ok(Value::Bool(
        expect_bool(l, "||: left operand")? || expect_bool(r, "||: right operand")?,
      ));
    }
    "->" => {
      return Ok(Value::Bool(
        !expect_bool(l, "->: left operand")? || expect_bool(r, "->: right operand")?,
      ));
    }
    "==" => return Ok(Value::Bool(values_equal(l, r)?)),
    "!=" => return Ok(Value::Bool(!values_equal(l, r)?)),
    "//" => {
      return match (l, r) {
        (Value::AttrSet(lm), Value::AttrSet(rm)) => {
          // S34 perf slice (2026-05-21): empty-side fast path. When
          // either operand is empty, skip the iter().map().clone()
          // extend loop. The merge result is byte-identical because
          // extending with an empty iter is a no-op. Common in
          // overlay-style env propagation `x // { override = ...; }`
          // where one side is constructed empty and grown later.
          if rm.is_empty() {
            Ok(Value::AttrSet(lm.clone()))
          } else if lm.is_empty() {
            Ok(Value::AttrSet(rm.clone()))
          } else {
            let mut res = (**lm).clone();
            for (k, v) in rm.iter() {
              res.insert(k.clone(), v.clone());
            }
            Ok(Value::AttrSet(Arc::new(res)))
          }
        }
        (Value::Null, _) => Ok(r.clone()),
        (_, Value::Null) => Ok(l.clone()),
        _ => Err(anyhow!(
          "//: both operands must be attrsets (or null), got {} and {}",
          type_name(l),
          type_name(r)
        )),
      };
    }
    _ => {}
  }

  if op == "+" {
    // Generic string concat: matches both `String` and `StringContext`,
    // unions the contexts (if any) and produces a `StringContext` only
    // when the result actually has provenance markers.
    if let (Some(ls), Some(rs)) = (l.as_str(), r.as_str()) {
      let mut combined: BTreeSet<String> = BTreeSet::new();
      if let Some(c) = l.string_context() {
        extend_string_context(&mut combined, c);
      }
      if let Some(c) = r.string_context() {
        extend_string_context(&mut combined, c);
      }
      let text = concat_strs(ls, rs);
      return Ok(Value::string_with_context(text, combined));
    }
    if let (Value::List(ll), Value::List(rl)) = (l, r) {
      let mut res = Vec::with_capacity(ll.len() + rl.len());
      extend_value_list(&mut res, ll);
      extend_value_list(&mut res, rl);
      return Ok(Value::List(Arc::new(res)));
    }
    if let (Value::AttrSet(lm), Value::AttrSet(rm)) = (l, r) {
      let mut res = (**lm).clone();
      for (k, v) in rm.iter() {
        res.insert(k.clone(), v.clone());
      }
      return Ok(Value::AttrSet(Arc::new(res)));
    }
    // 2026-05-05 (slice #54): path/string + string/path now
    // propagates string context so the path's role as a build-
    // time dependency survives the operation. Pre-fix three
    // silent metadata-loss shapes:
    //
    //   1. `string + path` — produced a plain `Value::String`,
    //      losing the path's provenance. Real Nix adds the path
    //      to the result string's context (same shape as a
    //      `${./p}` interpolation). Slice #49/#51 family extension.
    //
    //   2. `path + (context-bearing string)` — silently dropped
    //      the right-hand string's context because the result
    //      type is `Value::Path` which cannot carry string
    //      context. A `.px` author concatenating a derivation-
    //      backed string to a path would silently lose the
    //      build-time dependency marker. Now errors fail-loud:
    //      the metadata loss must be visible.
    //
    //   3. `path + plain-string` — unchanged (still produces
    //      Path), no context to preserve.
    //
    // Path + path is unchanged — paths do not carry context.
    if let (Value::Path(_), Value::Path(_)) = (l, r) {
      let lp = match l {
        Value::Path(p) => p,
        _ => unreachable!(),
      };
      let rp = match r {
        Value::Path(p) => p,
        _ => unreachable!(),
      };
      let combined = concat_path_displays(&lp, &rp);
      // 2026-05-05 (slice #66): normalize the result so
      // `./a + ./../b` produces `./b` (collapsed) rather
      // than `./a./../b` literal.
      return Ok(Value::Path(normalize_pnix_path(&std::path::PathBuf::from(
        combined,
      ))));
    }
    if matches!(l, Value::Path(_)) && matches!(r, Value::String(_) | Value::StringContext { .. }) {
      let lp = match l {
        Value::Path(p) => p,
        _ => unreachable!(),
      };
      let rs = r.as_str().unwrap_or("");
      let r_ctx = r.string_context();
      // If the right-hand string carries context, the result
      // must preserve it. Path cannot carry context, so we
      // refuse rather than silently drop. Authors who genuinely
      // want a Path can use `unsafeDiscardStringContext` first.
      if r_ctx.map(|c| !c.is_empty()).unwrap_or(false) {
        return Err(anyhow!(
          "operator +: path + context-bearing string would silently \
           drop string context (path cannot carry context). Use \
           string + path (which produces a context-preserving string) \
           or strip context first via builtins.unsafeDiscardStringContext."
        ));
      }
      let combined = concat_path_display_with_str(&lp, rs);
      // 2026-05-05 (slice #66): normalize the result so
      // `./a + "/../b"` produces `./b`.
      return Ok(Value::Path(normalize_pnix_path(&std::path::PathBuf::from(
        combined,
      ))));
    }
    if matches!(l, Value::String(_) | Value::StringContext { .. }) && matches!(r, Value::Path(_)) {
      let ls = l.as_str().unwrap_or("");
      let rp = match r {
        Value::Path(p) => p,
        _ => unreachable!(),
      };
      let mut combined: BTreeSet<String> = BTreeSet::new();
      if let Some(c) = l.string_context() {
        extend_string_context(&mut combined, c);
      }
      // Add the path's display form to the context — matches
      // `${./p}` interpolation semantics. The path becomes a
      // build-time dependency marker on the resulting string.
      combined.insert(path_display_string(&rp));
      let text = concat_str_with_path_display(ls, &rp);
      return Ok(Value::string_with_context(text, combined));
    }
  }
  if op == "++" {
    if let (Value::List(ll), Value::List(rl)) = (l, r) {
      let mut res = Vec::with_capacity(ll.len() + rl.len());
      extend_value_list(&mut res, ll);
      extend_value_list(&mut res, rl);
      return Ok(Value::List(Arc::new(res)));
    }
    return Err(anyhow!(
      "++: both operands must be lists, got {} and {}",
      type_name(l),
      type_name(r)
    ));
  }

  if matches!(op, "+" | "-" | "*" | "/" | "%") && is_numeric(l) && is_numeric(r) {
    return arith_op(op, l, r);
  }

  if matches!(op, "<" | ">" | "<=" | ">=") {
    return order_compare(op, l, r);
  }

  Err(anyhow!(
    "operator {}: unsupported operand types {} and {}",
    op,
    type_name(l),
    type_name(r)
  ))
}

fn arith_op(op: &str, l: &Value, r: &Value) -> Result<Value> {
  match (l, r) {
    (Value::Int(x), Value::Int(y)) => match op {
      "+" => x
        .checked_add(*y)
        .map(Value::Int)
        .ok_or_else(|| anyhow!("integer overflow: {} + {}", x, y)),
      "-" => x
        .checked_sub(*y)
        .map(Value::Int)
        .ok_or_else(|| anyhow!("integer overflow: {} - {}", x, y)),
      "*" => x
        .checked_mul(*y)
        .map(Value::Int)
        .ok_or_else(|| anyhow!("integer overflow: {} * {}", x, y)),
      "/" => {
        if *y == 0 {
          return Err(anyhow!("division by zero"));
        }
        x.checked_div(*y)
          .map(Value::Int)
          .ok_or_else(|| anyhow!("integer overflow: {} / {}", x, y))
      }
      "%" => {
        if *y == 0 {
          return Err(anyhow!("modulo by zero"));
        }
        // `checked_rem` covers `i64::MIN % -1` overflow which `%` itself
        // would panic on. Mirrors `builtins.mod` zero/overflow handling.
        x.checked_rem(*y)
          .map(Value::Int)
          .ok_or_else(|| anyhow!("integer overflow: {} % {}", x, y))
      }
      _ => unreachable!("arith_op called with non-arith op {}", op),
    },
    _ => {
      let lf = l
        .as_f64()
        .ok_or_else(|| anyhow!("arith {}: non-numeric operand {}", op, type_name(l)))?;
      let rf = r
        .as_f64()
        .ok_or_else(|| anyhow!("arith {}: non-numeric operand {}", op, type_name(r)))?;
      let result = match op {
        "+" => lf + rf,
        "-" => lf - rf,
        "*" => lf * rf,
        "/" => {
          if rf == 0.0 {
            return Err(anyhow!("division by zero"));
          }
          lf / rf
        }
        "%" => {
          if rf == 0.0 {
            return Err(anyhow!("modulo by zero"));
          }
          // f64 `%` is IEEE 754 remainder (C `fmod`-equivalent
          // semantics for finite operands). Matches what `builtins.mod`
          // does when either side is float.
          lf % rf
        }
        _ => unreachable!("arith_op called with non-arith op {}", op),
      };
      Ok(Value::Float(result))
    }
  }
}

fn order_compare(op: &str, l: &Value, r: &Value) -> Result<Value> {
  let ord = compare_values(l, r)?;
  let answer = match op {
    "<" => ord == std::cmp::Ordering::Less,
    "<=" => ord != std::cmp::Ordering::Greater,
    ">" => ord == std::cmp::Ordering::Greater,
    ">=" => ord != std::cmp::Ordering::Less,
    _ => unreachable!("order_compare called with non-order op {}", op),
  };
  Ok(Value::Bool(answer))
}

fn compare_values(l: &Value, r: &Value) -> Result<std::cmp::Ordering> {
  compare_values_at_depth(l, r, 0)
}

// 2026-05-05: same depth-limit shape as `values_equal_at_depth`
// (slice #42). `let r = [ r ]; in r < r` overflowed the Rust
// call stack with `SIGABRT` because `compare_values` recursed
// into list children without cycle tracking. Real Nix errors
// with "infinite recursion encountered" on the same input.
// `compare_values` is reachable from `<` / `>` / `<=` / `>=`
// binary operators AND from `builtins.lt` / `le` / `gt` / `ge`,
// so the single helper closes all five surfaces. Reuses the
// `VALUES_EQUAL_MAX_DEPTH` constant for a consistent depth
// budget across the equality / comparison family.
fn compare_values_at_depth(l: &Value, r: &Value, depth: usize) -> Result<std::cmp::Ordering> {
  use std::cmp::Ordering;
  if depth > VALUES_EQUAL_MAX_DEPTH {
    return Err(anyhow!(
      "comparison: infinite recursion encountered (cyclic value or pathologically deep comparison, max depth {})",
      VALUES_EQUAL_MAX_DEPTH
    ));
  }
  // Match equality's S25 fast path: comparison is read-only for
  // non-thunks, so only materialize an owned Value when a side is
  // actually lazy. The previous shape cloned both operands on every
  // recursive list comparison just to call `force_value`.
  let l_ref = force_if_thunk(l)?;
  let r_ref = force_if_thunk(r)?;
  match (l_ref.as_ref(), r_ref.as_ref()) {
    (Value::Int(a), Value::Int(b)) => Ok(a.cmp(b)),
    (Value::Float(a), Value::Float(b)) => a
      .partial_cmp(b)
      .ok_or_else(|| anyhow!("comparison with NaN")),
    (Value::Int(a), Value::Float(b)) => (*a as f64)
      .partial_cmp(b)
      .ok_or_else(|| anyhow!("comparison with NaN")),
    (Value::Float(a), Value::Int(b)) => a
      .partial_cmp(&(*b as f64))
      .ok_or_else(|| anyhow!("comparison with NaN")),
    // 2026-05-05 (slice #60): comparison compares STRING TEXT
    // only — context is ignored. Real Nix's `<` / `<=` / `>` /
    // `>=` are text-only for strings; two strings sort by text
    // regardless of build-time-dependency context. Pre-fix
    // pnix had `(String(a), String(b)) => Ok(a.cmp(b))` ONLY,
    // so any comparison touching a `Value::StringContext`
    // fell through to the catch-all `_ => Err("cannot compare
    // string with string")` — a misleading error claiming the
    // operands aren't strings when they ARE strings.
    //
    // Same shape as slice #59's equality fix. Production hit
    // shapes:
    //   - `sort` on a list of derivation-reference strings
    //     (e.g. `sort cmp [ ./a-drv.script ./b-drv.script ]`)
    //     errored on every comparison.
    //   - `if name < "expected" then ...` on a context-bearing
    //     name errored instead of comparing.
    //   - `builtins.lessThan` on context-bearing strings
    //     errored.
    //   - `compareVersions` is unaffected because it already
    //     uses `as_str()` (slice #31 family).
    //
    // The fix uses `as_str()` which already handles both
    // `Value::String` and `Value::StringContext`, so all four
    // combinations (String/String, String/Context, Context/
    // String, Context/Context) compare texts uniformly.
    (l, r)
      if matches!(l, Value::String(_) | Value::StringContext { .. })
        && matches!(r, Value::String(_) | Value::StringContext { .. }) =>
    {
      Ok(l.as_str().unwrap_or("").cmp(r.as_str().unwrap_or("")))
    }
    // 2026-05-05 (slice #65): compare normalized paths so
    // semantically-equivalent representations sort together.
    // Pre-fix `./a/../b < ./b` compared the literal PathBuf
    // strings, so the result depended on raw component order
    // rather than path semantics. Now both sides normalize
    // (collapse `.` and `..`) before comparison.
    (Value::Path(a), Value::Path(b)) => Ok(normalize_pnix_path(a).cmp(&normalize_pnix_path(b))),
    (Value::List(a), Value::List(b)) => {
      for (xa, xb) in a.iter().zip(b.iter()) {
        match compare_values_at_depth(xa, xb, depth + 1)? {
          Ordering::Equal => continue,
          other => return Ok(other),
        }
      }
      Ok(a.len().cmp(&b.len()))
    }
    _ => Err(anyhow!(
      "cannot compare {} with {}",
      type_name(l_ref.as_ref()),
      type_name(r_ref.as_ref())
    )),
  }
}

// 2026-05-05: equality comparison depth limit. `r == r` for a
// cyclic `r` (`let r = { a = r; }; in r == r`) overflowed the
// Rust call stack with `SIGABRT` because `values_equal`
// recursed into AttrSet / List children without cycle
// tracking. Real Nix errors with "infinite recursion
// encountered" on the same input. The limit is chosen
// conservatively: 64 levels of nested attrsets / lists is
// already pathological for legitimate use; cycles overflow
// the 2 MB Rust test thread stack at ~30-50 iterations of the
// heavy `force_value + match + recurse` frame, so 64 errors
// well before the stack runs out. The depth is per-call; it
// does not leak across independent `==` evaluations.
const VALUES_EQUAL_MAX_DEPTH: usize = 64;

fn values_equal(l: &Value, r: &Value) -> Result<bool> {
  values_equal_at_depth(l, r, 0)
}

/// S25 perf helper (2026-05-21): force a value into an owned form
/// only when it is actually a `Value::Thunk`. Non-thunks borrow
/// through. Used by read-only consumers (e.g. `values_equal_at_depth`)
/// that previously cloned both sides into owned forms just so they
/// could call `force_value` (which consumes its argument).
fn force_if_thunk(v: &Value) -> Result<Cow<'_, Value>> {
  match v {
    // DF-1 perf slice (2026-06-10): when the thunk cache is already
    // populated with a non-thunk value, borrow straight out of the
    // cache instead of going through `force_value(v.clone())`. The
    // pre-fix path deep-cloned the cached value on every read-only
    // forced access (`cache.get().cloned()` inside `force_value`) —
    // for a cached attrset/list that is a full container-tree copy
    // per access. The cache `OnceLock` lives inside the thunk's own
    // `Arc`, so the borrow is tied to `v`'s lifetime. A cached value
    // that is itself a thunk (defensive case force_value's loop
    // handles) falls through to the owned path unchanged.
    Value::Thunk { cache, .. } => {
      if let Some(cached) = cache.get() {
        if !matches!(cached, Value::Thunk { .. }) {
          return Ok(Cow::Borrowed(cached));
        }
      }
      Ok(Cow::Owned(force_value(v.clone())?))
    }
    _ => Ok(Cow::Borrowed(v)),
  }
}

fn values_equal_at_depth(l: &Value, r: &Value, depth: usize) -> Result<bool> {
  if depth > VALUES_EQUAL_MAX_DEPTH {
    return Err(anyhow!(
      "==: infinite recursion encountered (cyclic value or pathologically deep equality, max depth {})",
      VALUES_EQUAL_MAX_DEPTH
    ));
  }
  // S25 perf slice (2026-05-21): only clone-into-owned when the value
  // is actually a Thunk (force_value consumes its argument). Non-thunk
  // Values are read-only here -- equality only inspects shape and
  // element content -- so a borrowed reference suffices. Pre-fix this
  // function did `force_value(l.clone()) + force_value(r.clone())`
  // unconditionally, deep-cloning both sides every time. Root values
  // are typically non-thunk, and most list/attrset elements forced
  // through recursive calls are already cached after first force --
  // so the twin clone was largely wasted work.
  let l_ref = force_if_thunk(l)?;
  let r_ref = force_if_thunk(r)?;
  Ok(match (l_ref.as_ref(), r_ref.as_ref()) {
    (Value::Null, Value::Null) => true,
    (Value::Bool(a), Value::Bool(b)) => a == b,
    (Value::Int(a), Value::Int(b)) => a == b,
    (Value::Float(a), Value::Float(b)) => a == b,
    (Value::Int(a), Value::Float(b)) | (Value::Float(b), Value::Int(a)) => (*a as f64) == *b,
    // 2026-05-05 (slice #59): equality compares STRING TEXT
    // only — context is ignored. Real Nix's `==` is text-only
    // for strings; two strings with the same text are equal
    // regardless of build-time-dependency context. Pre-fix
    // pnix had `(String(a), String(b)) => a == b` ONLY, so:
    //   - `"x${./p}" == "x${./p}"` returned `false` (same
    //     text + same context, but no arm matched) — this fell
    //     through to the catch-all `_ => false` at the bottom.
    //   - `"abc" == ("a" + "b" + "c")` could be either true or
    //     false depending on whether the constructed string had
    //     context (slice #54's `+ Path` would add context).
    // Production-relevant: any `if str == "expected" then ...`
    // branch on a context-bearing string ALWAYS took the else
    // branch, silently flipping logic. Also breaks set / list
    // dedup via equality. The fix uses `as_str()` which already
    // handles both `Value::String` and `Value::StringContext`,
    // so all four combinations (String/String, String/Context,
    // Context/String, Context/Context) compare texts uniformly.
    (l, r)
      if matches!(l, Value::String(_) | Value::StringContext { .. })
        && matches!(r, Value::String(_) | Value::StringContext { .. }) =>
    {
      l.as_str() == r.as_str()
    }
    // 2026-05-05 (slice #65): compare normalized paths so
    // semantically-equivalent representations are equal.
    // Pre-fix `./a/../b == ./b` returned `false` because
    // PathBuf comparison was literal (kept `..` in components
    // rather than collapsing). Real Nix normalizes paths at
    // construction; pnix normalizes at comparison time.
    (Value::Path(a), Value::Path(b)) => normalize_pnix_path(a) == normalize_pnix_path(b),
    (Value::List(a), Value::List(b)) => {
      if a.len() != b.len() {
        false
      } else {
        let mut all_eq = true;
        for (x, y) in a.iter().zip(b.iter()) {
          if !values_equal_at_depth(x, y, depth + 1)? {
            all_eq = false;
            break;
          }
        }
        all_eq
      }
    }
    (Value::AttrSet(a), Value::AttrSet(b)) => {
      if a.len() != b.len() {
        false
      } else {
        let mut all_eq = true;
        for (k, va) in a.iter() {
          let Some(vb) = b.get(k) else {
            all_eq = false;
            break;
          };
          if !values_equal_at_depth(va, vb, depth + 1)? {
            all_eq = false;
            break;
          }
        }
        all_eq
      }
    }
    (Value::Lambda { .. }, _)
    | (_, Value::Lambda { .. })
    | (Value::BuiltinPartial { .. }, _)
    | (_, Value::BuiltinPartial { .. }) => false,
    _ => false,
  })
}

fn is_numeric(v: &Value) -> bool {
  matches!(v, Value::Int(_) | Value::Float(_))
}

pub(crate) fn type_name(v: &Value) -> &'static str {
  match v {
    Value::Null => "null",
    Value::Bool(_) => "bool",
    Value::Int(_) => "int",
    Value::Float(_) => "float",
    Value::String(_) | Value::StringContext { .. } => "string",
    Value::Path(_) => "path",
    Value::List(_) => "list",
    Value::AttrSet(_) => "set",
    Value::Lambda { .. } | Value::BuiltinPartial { .. } => "lambda",
    Value::Thunk { .. } => "thunk",
  }
}

fn eval_unary(op: &str, a: &Value) -> Result<Value> {
  match op {
    "-" => match a {
      // 2026-05-05: `-i64::MIN` overflows in Rust's `-` because
      // i64::MIN has no positive counterpart in i64. Use
      // `checked_neg` to surface a typed error instead of a
      // panic. Same fix shape as the slice #44 family
      // (checked_X for arithmetic that would otherwise panic
      // in debug / wrap in release).
      Value::Int(i) => i
        .checked_neg()
        .map(Value::Int)
        .ok_or_else(|| anyhow!("integer overflow: -{}", i)),
      Value::Float(f) => Ok(Value::Float(-f)),
      _ => Err(anyhow!("cannot negate")),
    },
    "!" => Ok(Value::Bool(!expect_bool(a, "!: operand")?)),
    _ => Err(anyhow!("unknown unary op: {}", op)),
  }
}

const BUILTIN_FUNCTION_NAMES: &[&str] = &[
  // batch 263 (2026-04-18): builtin parity 확장. Nix stdlib 에서 widely used
  // 되는 추가 builtins (substring / stringLength / hasPrefix / hasSuffix / elem
  // / lessThan / listToAttrs / removeAttrs / add / sub / mul / div) 을 추가.
  // M1-11 legacy fallback 제거의 prerequisite.
  "map",
  "mapAttrs",
  "filter",
  "filterAttrs",
  "fold",
  "find",
  "length",
  "head",
  "tail",
  "drop",
  "take",
  "elemAt",
  "attrNames",
  "attrValues",
  "keys",
  "values",
  "get",
  "mapGet",
  "set",
  "mapSet",
  "mapKeys",
  "mapValues",
  "merge",
  "mapMerge",
  "intersectAttrs",
  "hasAttr",
  "getAttr",
  // 2026-05-06 (slice #77): missing-builtin gap. attrByPath
  // walks a path list into nested attrsets (returns default
  // if any step is missing). getAttrs returns a subset
  // attrset for a list of names. Both are standard real-Nix
  // builtins commonly used in nixpkgs `lib.attrsets` style
  // code.
  "attrByPath",
  "getAttrs",
  "typeOf",
  "toString",
  "toJSON",
  "fromJSON",
  "fromTOML",
  "hashString",
  "hashFile",
  "toXML",
  "sort",
  "concatLists",
  "concatStringsSep",
  "concatStrings",
  "concatMap",
  "foldl'",
  "foldr",
  "genList",
  "all",
  "any",
  "and",
  "or",
  "not",
  "eq",
  "lt",
  "le",
  "gt",
  "ge",
  "append",
  "cons",
  "reverse",
  "reverseList",
  "zip",
  "flatten",
  "isString",
  "isList",
  "isAttrs",
  "isBool",
  "isInt",
  "isFloat",
  "isNull",
  // Finite/infinite/NaN detection on floats. `isFinite` is the
  // production-safe guard for `.px` authors after arithmetic
  // that might overflow into ±inf (e.g. `1.0e308 * 10`); pair
  // it with an explicit fallback rather than letting the
  // non-finite value flow into `toJSON` (which now errors).
  "isFinite",
  "isInf",
  "isNaN",
  "ontologyLift",
  "stringContextToProvenance",
  "ontologyEvaluate",
  "ontologySelect",
  "ontologyPromote",
  "ontologyPromoteWithLane",
  "ontologyQuery",
  "ontologyEmit",
  "replaceStrings",
  "groupBy",
  "catAttrs",
  "partition",
  // batch 263: 추가 string builtins.
  "substring",
  "stringLength",
  // v0.16-BO (2026-05-20): Korean Hangul jongseong (final
  // consonant) classifier. Substrate-native — uses
  // pnix_core::lang::ko::decompose_hangul_syllable. Returns
  // "none" / "regular" / "rieul" / "non-korean" for the
  // jongseong allomorph dispatch in Korean case marker lens
  // (stdlib/lib/coding/korean-case-marker-lens.px).
  // Owner thesis: Korean postposition algebra is the substrate
  // ontology engine's first-class NL surface; jongseong
  // classification is a substrate-internal Rust primitive (not
  // external NLP like mecab-ko / lindera).
  "koreanFinalConsonantKind",
  // batch 263: list/attrset 조작 builtins.
  "elem",
  "listToAttrs",
  "removeAttrs",
  // batch 263: 산술 predicate / explicit arithmetic.
  "lessThan",
  "add",
  "sub",
  "mul",
  "div",
  "mod",
  "neg",
  "abs",
  "pow",
  "sqrt",
  "floor",
  "ceil",
  "exp",
  "ln",
  "log",
  "sin",
  "cos",
  "tan",
  "atan2",
  "readFile",
  "readFileType",
  "toFile",
  "getEnv",
  "tryEval",
  "seq",
  "deepSeq",
  "trace",
  "traceVerbose",
  "warn",
  "addErrorContext",
  "functionArgs",
  "schemaValidate",
  "schemaNormalize",
  "schemaExplain",
  "svgSchemaNormalize",
  "svgSchemaValidate",
  "svgSchemaExplain",
  "svgEmit",
  "svgRenderPacket",
  "xmlParse",
  "xmlEmit",
  "htmlParse",
  "htmlEmit",
  "mathmlXmlToJson",
  "openmathXmlToJson",
  "mathmlEmit",
  "openmathEmit",
  "x3dXmlToJson",
  "x3dSchemaNormalize",
  "x3dSchemaValidate",
  "x3dSchemaExplain",
  "x3dFrpGraph",
  "x3dSyncPlan",
  "x3dX3domFragment",
  "x3dX3domHtml",
  "x3dX3domPatch",
  "x3dRenderPacket",
  // Bio/office schema + format builtins. Implementations live in
  // `xml_format_schema.rs` as clean-room XML-family validation: XML strings
  // are parsed, ASTs are structurally checked, and family wrappers apply
  // small project-authored root checks without DTD/XSD vendoring.
  "cellmlSchemaNormalize",
  "cellmlSchemaValidate",
  "cellmlSchemaExplain",
  "cmlSchemaNormalize",
  "cmlSchemaValidate",
  "cmlSchemaExplain",
  "neuromlSchemaNormalize",
  "neuromlSchemaValidate",
  "neuromlSchemaExplain",
  "pdbmlSchemaNormalize",
  "pdbmlSchemaValidate",
  "pdbmlSchemaExplain",
  "sbmlSchemaNormalize",
  "sbmlSchemaValidate",
  "sbmlSchemaExplain",
  "biopaxSchemaNormalize",
  "biopaxSchemaValidate",
  "biopaxSchemaExplain",
  "giftiSchemaNormalize",
  "giftiSchemaValidate",
  "giftiSchemaExplain",
  "lemsSchemaNormalize",
  "lemsSchemaValidate",
  "lemsSchemaExplain",
  "omexSchemaNormalize",
  "omexSchemaValidate",
  "omexSchemaExplain",
  "pharmmlSchemaNormalize",
  "pharmmlSchemaValidate",
  "pharmmlSchemaExplain",
  "sbgnmlSchemaNormalize",
  "sbgnmlSchemaValidate",
  "sbgnmlSchemaExplain",
  "sedmlSchemaNormalize",
  "sedmlSchemaValidate",
  "sedmlSchemaExplain",
  "vtkSchemaNormalize",
  "vtkSchemaValidate",
  "vtkSchemaExplain",
  "xdmfSchemaNormalize",
  "xdmfSchemaValidate",
  "xdmfSchemaExplain",
  "ifcxmlSchemaNormalize",
  "ifcxmlSchemaValidate",
  "ifcxmlSchemaExplain",
  "mathmlSchemaNormalize",
  "mathmlSchemaValidate",
  "mathmlSchemaExplain",
  "openmathSchemaNormalize",
  "openmathSchemaValidate",
  "openmathSchemaExplain",
  "xmlSchemaNormalize",
  "xmlSchemaValidate",
  "xmlSchemaExplain",
  "colladaSchemaNormalize",
  "colladaSchemaValidate",
  "colladaSchemaExplain",
  "programSchemaNormalize",
  "programSchemaValidate",
  "programSchemaExplain",
  "hanimSchemaNormalize",
  "hanimSchemaValidate",
  "hanimSchemaExplain",
  "hanimSchemaValidateJointHierarchy",
  "excelXmlToJson",
  "excelEmit",
  "excelToOds",
  "odsToExcel",
  "excelFormulaToOpenFormula",
  "openFormulaToExcel",
  "excelStyleToOds",
  "odsStyleToExcel",
  "excelAdvancedToOds",
  "odsAdvancedToExcel",
  "isPath",
  "pathExists",
  "pnixMount",
  "pnixMounts",
  "pnixRun",
  "pnixUmount",
  "readDir",
  "baseNameOf",
  "dirOf",
  "toPath",
  "storePath",
  "match",
  "split",
  "hasContext",
  "unsafeDiscardStringContext",
  "getContext",
  "addDrvOutputDependencies",
  "unsafeDiscardOutputDependency",
  "unsafeAddOutputDependency",
  "unsafeAddOutputName",
  "appendContext",
  "unsafeGetAttrPos",
  "zipAttrsWith",
  "compareVersions",
  "splitVersion",
  "parseDrvName",
  "abort",
  "throw",
  "isFunction",
  "import",
  "break",
  "placeholder",
  "derivationStrict",
  // 2026-05-06 (slice #80): high-level derivation builder.
  // Real Nix exposes both `derivation` and `derivationStrict`;
  // pnix had only the latter. Most nixpkgs-style code calls
  // `builtins.derivation` (directly or via mkDerivation
  // wrappers) — so the missing-builtin gap was production-
  // relevant. New arm at `apply_builtin` clones the input
  // attrs and layers the standard `outPath` / `drvPath` /
  // `type = "derivation"` fields on top.
  "derivation",
  "bitAnd",
  "bitOr",
  "bitXor",
  "genericClosure",
  // Real Nix exposes `scopedImport` at both `scopedImport` (global)
  // and `builtins.scopedImport`. nixpkgs uses both spellings —
  // `let scopedImport = builtins.scopedImport;` is a common
  // wrap. Without the alias here `builtins ? scopedImport`
  // returned `false`. The arm impl is at line 3069 (no change).
  "scopedImport",
];

fn builtins_attrset() -> Value {
  let mut map = BTreeMap::new();
  for &name in BUILTIN_FUNCTION_NAMES {
    map.insert(name.to_string(), builtin_partial_value(name));
  }
  map.insert(
    "currentSystem".to_string(),
    Value::String(current_system().to_string()),
  );
  map.insert(
    "nixVersion".to_string(),
    Value::String("2.18.0-pnix".to_string()),
  );
  // 2026-05-05 (slice #70): `builtins.langVersion` — the Nix
  // language version. Real Nix returns an int (currently 6 as
  // of Nix 2.18+). Pre-fix pnix didn't expose this builtin —
  // .px code that branched on `langVersion` (e.g., feature-
  // detection in nixpkgs lib) errored with "attribute
  // 'langVersion' not found" — misleading-error pattern same
  // as slice #68 / #69. Pnix tracks Nix language version 6
  // (the current at time of audit).
  map.insert("langVersion".to_string(), Value::Int(6));
  map.insert(
    "storeDir".to_string(),
    Value::String(get_store_dir().to_string()),
  );
  Value::AttrSet(Arc::new(map))
}

fn fast_builtin_attr_value(attr: &str) -> Option<Value> {
  match attr {
    // Constant builtins must return their constant value, not a function.
    "currentSystem" => Some(Value::String(current_system().to_string())),
    "nixVersion" => Some(Value::String("2.18.0-pnix".to_string())),
    "langVersion" => Some(Value::Int(6)),
    "storeDir" => Some(Value::String(get_store_dir().to_string())),
    _ => fast_builtin_function_name_arc(attr).map(|name| Value::BuiltinPartial {
      name,
      args: Vec::new(),
    }),
  }
}

fn builtin_partial_value(name: &str) -> Value {
  builtin_partial_value_with_args(name, Vec::new())
}

fn builtin_partial_value_with_args(name: &str, args: Vec<Value>) -> Value {
  Value::BuiltinPartial {
    name: fast_builtin_function_name_arc(name).unwrap_or_else(|| Arc::from(name)),
    args,
  }
}

fn fast_builtin_attr_exists(attr: &str) -> bool {
  matches!(
    attr,
    "currentSystem" | "nixVersion" | "langVersion" | "storeDir"
  ) || builtin_function_name_arcs().contains_key(attr)
}

fn fast_builtin_function_name_arc(attr: &str) -> Option<Arc<str>> {
  builtin_function_name_arcs().get(attr).cloned()
}

fn builtin_function_name_arcs() -> &'static FxHashMap<&'static str, Arc<str>> {
  static BUILTIN_FUNCTION_NAME_ARCS: OnceLock<FxHashMap<&'static str, Arc<str>>> = OnceLock::new();
  BUILTIN_FUNCTION_NAME_ARCS.get_or_init(|| {
    let mut map = fx_hashmap_with_capacity(BUILTIN_FUNCTION_NAMES.len());
    for &name in BUILTIN_FUNCTION_NAMES {
      map.insert(name, Arc::from(name));
    }
    map
  })
}

fn attr_names_list(m: &BTreeMap<String, Value>) -> Vec<Value> {
  let mut out = Vec::with_capacity(m.len());
  for key in m.keys() {
    out.push(Value::String(key.clone()));
  }
  out
}

fn attr_values_list(m: &BTreeMap<String, Value>) -> Vec<Value> {
  let mut out = Vec::with_capacity(m.len());
  for value in m.values() {
    out.push(value.clone());
  }
  out
}

fn extend_value_list(out: &mut Vec<Value>, items: &[Value]) {
  for item in items {
    out.push(item.clone());
  }
}

fn context_to_string_list(context: &BTreeSet<String>) -> Vec<Value> {
  let mut out = Vec::with_capacity(context.len());
  for entry in context {
    out.push(Value::String(entry.clone()));
  }
  out
}

fn extend_string_context(out: &mut BTreeSet<String>, context: &BTreeSet<String>) {
  for entry in context {
    out.insert(entry.clone());
  }
}

fn force_builtin_args(args: &[Value], deep: bool) -> Result<Vec<Value>> {
  let mut out = Vec::with_capacity(args.len());
  for arg in args {
    let value = if deep {
      deep_force(arg.clone())?
    } else {
      force_if_thunk(arg)?.into_owned()
    };
    out.push(value);
  }
  Ok(out)
}

// DF-1 perf slice (2026-06-10): structural scan mirroring deep force's
// descent topology EXACTLY — descends only `List` / `AttrSet`; `Lambda`
// / `BuiltinPartial` / scalars are leaves (deep_force does not descend
// into lambda envs or partial-application arg vectors, so neither does
// this scan). A `false` answer means `deep_force` on this value is a
// pure identity rebuild, which the builtin boundary can skip entirely.
fn value_contains_thunk_for_deep_force(value: &Value) -> bool {
  match value {
    Value::Thunk { .. } => true,
    Value::List(items) => items.iter().any(value_contains_thunk_for_deep_force),
    Value::AttrSet(map) => map.values().any(value_contains_thunk_for_deep_force),
    _ => false,
  }
}

// DF-1 perf slice (2026-06-10): telemetry-parity walk for the builtin
// boundary's already-forced fast path. When the boundary skips
// `deep_force(arg.clone())` because the arg holds no thunk anywhere,
// this read-only walk reproduces the exact `observe_deep_force_value`
// visit sequence the skipped deep force would have produced (same DFS,
// same depths, thunk_count contribution zero), so `force_node_count` /
// `force_attr_count` / `force_list_count` / `force_max_depth` in the
// P0 perf summary stay byte-identical with the pre-slice behavior.
// `deep_force_ms` still accrues the (now much smaller) walk duration.
fn record_deep_force_already_forced_walk(value: &Value) {
  let started = deep_force_timing_enabled().then(std::time::Instant::now);
  let mut perf = DeepForcePerf::default();
  observe_forced_value_tree(value, 0, &mut perf);
  record_deep_force_perf(
    &perf,
    started.map_or_else(Default::default, |s| s.elapsed()),
  );
}

fn observe_forced_value_tree(value: &Value, depth: usize, perf: &mut DeepForcePerf) {
  observe_deep_force_value(value, depth, perf);
  match value {
    Value::List(items) => {
      for item in items.iter() {
        observe_forced_value_tree(item, depth + 1, perf);
      }
    }
    Value::AttrSet(map) => {
      for v in map.values() {
        observe_forced_value_tree(v, depth + 1, perf);
      }
    }
    _ => {}
  }
}

fn apply_builtin(name: &str, args: &[Value]) -> Result<Value> {
  // Two-tier arg forcing:
  //
  // 1. Lazy-in-element builtins (map, filter, sort, foldl', foldr, concatMap,
  //    all, any, find, elem): only
  //    shallow-force args. They either access a subset, short-circuit, or
  //    pass elements through a user-provided lambda where laziness must
  //    be preserved.
  //
  // 2. All other builtins: deep-force args so list/attrset element slots
  //    no longer hold thunks. This guarantees `match item { Value::AttrSet
  //    => ... }` style consumption sees concrete values.
  // Some builtins must not force ANY of their args at the boundary.
  // Either their semantics are deliberately lazy in specific positions
  // (e.g. `foldl'`'s initial accumulator), or they are shape-only
  // inspectors (`length`, `typeOf`, `is*`), list accessors
  // (`head`, `tail`, `elemAt`), or attrset shape/accessors/subset
  // extractors (`attrNames`, `getAttrs`, `intersectAttrs`, etc.)
  // that can borrow non-thunk list/attrset inputs after a per-arm
  // shallow force. For these, hand `args` through verbatim; the
  // impl will force only what it touches.
  // 2026-05-06 (slice #77): `attrByPath` joins this bucket
  // because its second arg (the default) must stay lazy — real
  // Nix evaluates the default only when the path is missing.
  // Boundary `force_value` would fire a `throw "default"` even
  // when the path resolves successfully. The impl forces args[0]
  // (path) and args[2] (attrset) manually.
  let no_force_at_all = matches!(
    name,
    "foldl'"
      | "fold"
      | "foldl"
      | "foldr"
      | "attrByPath"
      | "length"
      | "typeOf"
      | "isNull"
      | "isString"
      | "isList"
      | "isAttrs"
      | "isBool"
      | "isInt"
      | "isFloat"
      | "isPath"
      | "isFunction"
      | "isFinite"
      | "isInf"
      | "isNaN"
      | "hasContext"
      | "unsafeDiscardStringContext"
      | "getContext"
      | "addDrvOutputDependencies"
      | "unsafeDiscardOutputDependency"
      | "unsafeAddOutputDependency"
      | "unsafeAddOutputName"
      | "appendContext"
      | "trace"
      | "traceVerbose"
      | "warn"
      | "addErrorContext"
      | "head"
      | "tail"
      | "drop"
      | "take"
      | "elemAt"
      | "append"
      | "cons"
      | "reverse"
      | "reverseList"
      | "zip"
      | "flatten"
      | "attrNames"
      | "attrValues"
      | "keys"
      | "values"
      | "mapKeys"
      | "mapValues"
      | "hasAttr"
      | "getAttr"
      | "get"
      | "mapGet"
      | "set"
      | "mapSet"
      | "merge"
      | "mapMerge"
      | "getAttrs"
      | "intersectAttrs"
      | "removeAttrs"
  );
  let lazy_in_elements = matches!(
    name,
    "map"
      | "mapAttrs"
      | "filter"
      | "filterAttrs"
      | "sort"
      | "foldl'"
      | "foldl"
      | "foldr"
      | "concatMap"
      // 2026-05-05 (slice #62): concatLists and genList are
      // list-producing builtins whose inner element values
      // should stay lazy — `length (concatLists [...])` and
      // `length (genList f n)` must not force inner element
      // thunks. Pre-fix the boundary fell into the `else`
      // branch which deep-forced every inner value, breaking
      // `length (genList (i: throw) 10)` (errored instead of
      // returning 10) and `length (concatLists [ [1] [throw] ])`
      // (errored instead of returning 2).
      | "concatLists"
      | "genList"
      | "all"
      | "any"
      | "groupBy"
      | "find"
      | "elem"
      | "concatStringsSep"
      | "concatStrings"
      | "replaceStrings"
      | "tryEval"
      // 2026-05-05 (slice #75): attrValues / keys / values /
      // mapKeys / mapValues are lazy in the resulting list
      // elements — same contract as attrNames. Pre-fix
      // these were missing from `lazy_in_elements`, so the
      // boundary deep-forced the input attrset, eagerly
      // evaluating every value. `length (attrValues { a=1;
      // b=throw "x"; })` errored on the throw instead of
      // returning 2. Cascades to `keys`/`values`/`mapKeys`/
      // `mapValues` which are aliases for the same shape.
      // 2026-05-05 (slice #76): getAttr and catAttrs extract
      // attribute values from attrsets — same lazy contract as
      // attrValues. Pre-fix `getAttr "a" { a = 1; b = throw "x"; }`
      // errored because the boundary deep_force fired the throw
      // at attr `b` even though only `a` was requested. Same for
      // `length (catAttrs "a" [ { a = 1; } { a = throw "x"; } ])`
      // — extracted values must stay lazy so `length` doesn't
      // force them. Cascades the slice #75 contract from list-
      // producing extractors (`attrValues`/`keys`/...) to the
      // single-attr accessor (`getAttr`) and list-of-attrset
      // extractor (`catAttrs`). Note: `catAttrs` impl also needs
      // a per-element shallow force since list elements may now
      // arrive as thunks.
      | "catAttrs"
      | "unsafeGetAttrPos"
      | "genericClosure"
      | "listToAttrs"
      | "seq"
  );
  // DF-1 perf slice (2026-06-10): borrow-through fast paths. The
  // pre-fix boundary cloned EVERY arg into a fresh owned Vec on EVERY
  // non-`no_force_at_all` builtin call — for the shallow bucket via
  // `force_if_thunk(arg)?.into_owned()` (a full container clone even
  // for non-thunk args), for the deep bucket via
  // `deep_force(arg.clone())` (a full clone PLUS a full rebuild). In
  // the steady state most args are already forced, so both buckets can
  // hand the caller's slice through unchanged:
  //   - shallow bucket: safe when no arg is a root-level thunk
  //     (force_if_thunk on a non-thunk is identity).
  //   - deep bucket: safe when no arg holds a thunk ANYWHERE in its
  //     deep-force descent (deep_force is then an identity rebuild).
  //     The telemetry-parity walk keeps the P0 force_* counters
  //     byte-identical with the skipped deep force.
  // When a thunk IS present the original force path runs unchanged.
  let forced_args: Vec<Value>;
  let args: &[Value] = if no_force_at_all {
    args
  } else if lazy_in_elements {
    if args.iter().any(|a| matches!(a, Value::Thunk { .. })) {
      forced_args = force_builtin_args(args, false)?;
      &forced_args
    } else {
      args
    }
  } else if args.iter().any(value_contains_thunk_for_deep_force) {
    forced_args = force_builtin_args(args, true)?;
    &forced_args
  } else {
    for arg in args {
      record_deep_force_already_forced_walk(arg);
    }
    args
  };
  match name {
    // 2026-05-05 (slice #62): Nix-compat: `map f xs` is lazy
    // in the resulting elements. Each output element is a
    // thunk for `f x_i`; only forced when accessed. Without
    // this, `length (map throw [1 2 3])` errored on the first
    // element instead of returning 3 — every legitimate lazy
    // pattern that mapped over a list and then took its
    // length / head / first slice was broken. Mirrors the
    // existing `mapAttrs` laziness contract (line ~2714).
    "map" if args.len() >= 2 => {
      let f = args[0].clone();
      match &args[1] {
        Value::List(items) => {
          let mut r = Vec::with_capacity(items.len());
          for i in items.iter() {
            r.push(deferred_apply(f.clone(), i.clone()));
          }
          Ok(Value::List(Arc::new(r)))
        }
        other => Err(anyhow!(
          "builtins.map: second arg must be list, got {}",
          type_name(other)
        )),
      }
    }
    "mapAttrs" if args.len() >= 2 => {
      // Nix-compat: `mapAttrs f attrs` is lazy in the resulting values.
      // Each output value is a thunk for `f key oldValue`; it is only
      // forced when the field is accessed. Without this, `mapAttrs throw
      // alphabet` would throw on construction even when the caller never
      // touches the result.
      let f = args[0].clone();
      let input = match &args[1] {
        Value::AttrSet(items) => items,
        _ => return Err(anyhow!("mapAttrs: need attrset")),
      };
      let mut out = BTreeMap::new();
      for (key, value) in input.iter() {
        let key_name = key.clone();
        out.insert(
          key_name.clone(),
          deferred_apply2(f.clone(), Value::String(key_name), value.clone()),
        );
      }
      Ok(Value::AttrSet(Arc::new(out)))
    }
    "filter" if args.len() >= 2 => {
      let f = &args[0];
      match &args[1] {
        Value::List(items) => {
          let mut r = Vec::with_capacity(items.len());
          for (idx, i) in items.iter().enumerate() {
            // The predicate must return a bool. Previously
            // `is_true()` truthy-coerced any non-false / non-null
            // value, so `filter (x: 42) [1] => [1]` and
            // `filter (x: x) [1 2 3] => [1 2 3]` all silently
            // passed. Match Nix and `filterAttrs` instead.
            match apply_value(f.clone(), i.clone())? {
              Value::Bool(true) => r.push(i.clone()),
              Value::Bool(false) => {}
              other => {
                return Err(anyhow!(
                  "builtins.filter: predicate must return bool, got {} at index {}",
                  type_name(&other),
                  idx
                ));
              }
            }
          }
          Ok(Value::List(Arc::new(r)))
        }
        other => Err(anyhow!(
          "builtins.filter: second argument must be list, got {}",
          type_name(other)
        )),
      }
    }
    "filterAttrs" if args.len() >= 2 => {
      let f = &args[0];
      match &args[1] {
        Value::AttrSet(items) => {
          let mut out = BTreeMap::new();
          for (key, value) in items.iter() {
            let partial = apply_value(f.clone(), Value::String(key.clone()))?;
            let keep = apply_value(partial, value.clone())?;
            match keep {
              Value::Bool(true) => {
                out.insert(key.clone(), value.clone());
              }
              Value::Bool(false) => {}
              other => {
                return Err(anyhow!(
                  "filterAttrs: predicate must return bool, got {}",
                  other
                ))
              }
            }
          }
          Ok(Value::AttrSet(Arc::new(out)))
        }
        _ => Err(anyhow!("filterAttrs: need attrset")),
      }
    }
    // 2026-05-05 (slice #52): closed silent type-pass on non-list
    // third arg. Pre-fix `fold f init nonList` silently returned
    // `init` for ANY non-list value (int / null / attrset / etc.),
    // which hid bugs where `.px` authors accidentally piped a
    // non-list (often a single-element value or an attrset they
    // forgot to `attrValues`) into `fold`. The Nix-canonical
    // `foldl'` / `foldr` already errored loudly (slices upstream),
    // and this `fold` is a pnix-only helper that should match the
    // same contract. Mirror the `foldl'` / `foldr` error shape.
    "fold" if args.len() >= 3 => {
      let func = force_if_thunk(&args[0])?;
      let init = args[1].clone();
      let list = force_if_thunk(&args[2])?;
      match list {
        Cow::Owned(Value::List(items)) => {
          let mut acc = init;
          for item in Arc::unwrap_or_clone(items) {
            acc = apply_value(apply_value(func.as_ref().clone(), acc)?, item)?;
          }
          Ok(acc)
        }
        Cow::Borrowed(Value::List(items)) => {
          let mut acc = init;
          for item in items.iter() {
            acc = apply_value(apply_value(func.as_ref().clone(), acc)?, item.clone())?;
          }
          Ok(acc)
        }
        other => Err(anyhow!(
          "builtins.fold: third arg must be list, got {}",
          type_name(other.as_ref())
        )),
      }
    }
    "find" if args.len() >= 2 => match &args[1] {
      Value::List(items) => {
        let mut found: Option<Value> = None;
        for item in items.iter() {
          if values_equal(item, &args[0])? {
            found = Some(item.clone());
            break;
          }
        }
        Ok(found.unwrap_or(Value::Null))
      }
      _ => Err(anyhow!("builtins.find: second arg must be list")),
    },
    "length" if args.len() >= 1 => {
      let value = force_if_thunk(&args[0])?;
      match value.as_ref() {
        Value::List(l) => Ok(Value::Int(l.len() as i64)),
        Value::String(s) => Ok(Value::Int(s.len() as i64)),
        Value::StringContext { text, .. } => Ok(Value::Int(text.len() as i64)),
        other => Err(anyhow!(
          "builtins.length: expected list or string, got {}",
          type_name(other)
        )),
      }
    }
    "head" if args.len() >= 1 => {
      let value = force_if_thunk(&args[0])?;
      match value.as_ref() {
        // Nix-correct: `head []` is an error ("list is empty"), not
        // a silent `null`. Silent-null bypassed every `if r != null
        // then …` guard a `.px` author might write after a head
        // call on a possibly-empty list.
        Value::List(l) if !l.is_empty() => Ok(l[0].clone()),
        Value::List(_) => Err(anyhow!("builtins.head: list is empty")),
        other => Err(anyhow!(
          "builtins.head: expected list, got {}",
          type_name(other)
        )),
      }
    }
    "tail" if args.len() >= 1 => {
      let value = force_if_thunk(&args[0])?;
      match value.as_ref() {
        // Nix-correct: `tail []` errors. The previous implementation
        // returned `[]`, which silently passed but doesn't match Nix.
        Value::List(l) if !l.is_empty() => Ok(Value::List(Arc::new(l[1..].to_vec()))),
        Value::List(_) => Err(anyhow!("builtins.tail: list is empty")),
        other => Err(anyhow!(
          "builtins.tail: expected list, got {}",
          type_name(other)
        )),
      }
    }
    "drop" if args.len() >= 2 => {
      let count_value = force_if_thunk(&args[0])?;
      // Nix-correct: negative count is an error. Older Nix versions
      // were tolerant; current Nix rejects. We follow current.
      let count = expect_i64(count_value.as_ref(), "builtins.drop")?;
      let list_value = force_if_thunk(&args[1])?;
      let Value::List(items) = list_value.as_ref() else {
        return Err(anyhow!(
          "builtins.drop: second arg must be list, got {}",
          type_name(list_value.as_ref())
        ));
      };
      if count < 0 {
        return Err(anyhow!(
          "builtins.drop: negative count {} not allowed",
          count
        ));
      }
      let count = usize::try_from(count).unwrap_or(usize::MAX);
      let start = count.min(items.len());
      Ok(Value::List(Arc::new(items[start..].to_vec())))
    }
    "take" if args.len() >= 2 => {
      let count_value = force_if_thunk(&args[0])?;
      let count = expect_i64(count_value.as_ref(), "builtins.take")?;
      let list_value = force_if_thunk(&args[1])?;
      let Value::List(items) = list_value.as_ref() else {
        return Err(anyhow!(
          "builtins.take: second arg must be list, got {}",
          type_name(list_value.as_ref())
        ));
      };
      if count < 0 {
        return Err(anyhow!(
          "builtins.take: negative count {} not allowed",
          count
        ));
      }
      let count = usize::try_from(count).unwrap_or(usize::MAX);
      let end = count.min(items.len());
      Ok(Value::List(Arc::new(items[..end].to_vec())))
    }
    "attrNames" if args.len() >= 1 => {
      let value = force_if_thunk(&args[0])?;
      match value.as_ref() {
        Value::AttrSet(m) => Ok(Value::List(Arc::new(attr_names_list(m)))),
        // Nix-correct + production fail-loud: non-attrset is a
        // type error, not silently `[]`. Silent-empty bypassed
        // user `if attrNames x == [] then …` guards.
        other => Err(anyhow!(
          "builtins.attrNames: expected attrset, got {}",
          type_name(other)
        )),
      }
    }
    "attrValues" if args.len() >= 1 => {
      let value = force_if_thunk(&args[0])?;
      match value.as_ref() {
        Value::AttrSet(m) => Ok(Value::List(Arc::new(attr_values_list(m)))),
        other => Err(anyhow!(
          "builtins.attrValues: expected attrset, got {}",
          type_name(other)
        )),
      }
    }
    "keys" | "mapKeys" if args.len() >= 1 => {
      let value = force_if_thunk(&args[0])?;
      match value.as_ref() {
        Value::AttrSet(m) => Ok(Value::List(Arc::new(attr_names_list(m)))),
        _ => Err(anyhow!("builtins.keys: expected attrset")),
      }
    }
    "values" | "mapValues" if args.len() >= 1 => {
      let value = force_if_thunk(&args[0])?;
      match value.as_ref() {
        Value::AttrSet(m) => Ok(Value::List(Arc::new(attr_values_list(m)))),
        _ => Err(anyhow!("builtins.values: expected attrset")),
      }
    }
    "get" | "mapGet" if args.len() >= 2 => {
      let attrs_value = force_if_thunk(&args[0])?;
      let Value::AttrSet(attrs) = attrs_value.as_ref() else {
        return Err(anyhow!(
          "builtins.get: first arg must be attrset, got {}",
          type_name(attrs_value.as_ref())
        ));
      };
      let key_value = force_if_thunk(&args[1])?;
      let Some(key) = key_value.as_ref().as_str() else {
        return Err(anyhow!(
          "builtins.get: second arg must be string, got {}",
          type_name(key_value.as_ref())
        ));
      };
      Ok(attrs.get(key).cloned().unwrap_or(Value::Null))
    }
    "set" | "mapSet" if args.len() >= 3 => {
      let attrs_value = force_if_thunk(&args[0])?;
      let Value::AttrSet(attrs) = attrs_value.as_ref() else {
        return Err(anyhow!(
          "builtins.set: first arg must be attrset, got {}",
          type_name(attrs_value.as_ref())
        ));
      };
      let key_value = force_if_thunk(&args[1])?;
      let Some(key) = key_value.as_ref().as_str() else {
        return Err(anyhow!(
          "builtins.set: second arg must be string, got {}",
          type_name(key_value.as_ref())
        ));
      };
      let mut out = (**attrs).clone();
      out.insert(key.to_string(), args[2].clone());
      Ok(Value::AttrSet(Arc::new(out)))
    }
    "merge" | "mapMerge" if args.len() >= 2 => {
      let lhs_value = force_if_thunk(&args[0])?;
      let Value::AttrSet(lhs) = lhs_value.as_ref() else {
        return Err(anyhow!(
          "builtins.merge: first arg must be attrset, got {}",
          type_name(lhs_value.as_ref())
        ));
      };
      let rhs_value = force_if_thunk(&args[1])?;
      let Value::AttrSet(rhs) = rhs_value.as_ref() else {
        return Err(anyhow!(
          "builtins.merge: second arg must be attrset, got {}",
          type_name(rhs_value.as_ref())
        ));
      };
      if rhs.is_empty() {
        return Ok(Value::AttrSet(lhs.clone()));
      }
      if lhs.is_empty() {
        return Ok(Value::AttrSet(rhs.clone()));
      }
      let mut out = (**lhs).clone();
      for (k, v) in rhs.iter() {
        out.insert(k.clone(), v.clone());
      }
      Ok(Value::AttrSet(Arc::new(out)))
    }
    "intersectAttrs" if args.len() >= 2 => {
      // Nix-compat: keep keys present in BOTH attrsets, take VALUES from
      // the second arg. Values from the first arg are never forced —
      // `intersectAttrs { a = abort "l"; } { b = ...; }` must not throw.
      let first_value = force_if_thunk(&args[0])?;
      let Value::AttrSet(first) = first_value.as_ref() else {
        return Err(anyhow!(
          "builtins.intersectAttrs: first arg expected attrset, got {}",
          first_value.as_ref()
        ));
      };
      let second_value = force_if_thunk(&args[1])?;
      let Value::AttrSet(second) = second_value.as_ref() else {
        return Err(anyhow!(
          "builtins.intersectAttrs: second arg expected attrset, got {}",
          second_value.as_ref()
        ));
      };
      let mut out = BTreeMap::new();
      for (key, value) in second.iter() {
        if first.contains_key(key) {
          out.insert(key.clone(), value.clone());
        }
      }
      Ok(Value::AttrSet(Arc::new(out)))
    }
    "hasAttr" if args.len() >= 2 => {
      let key = force_if_thunk(&args[0])?;
      let Some(k) = key.as_ref().as_str() else {
        return Err(anyhow!(
          "builtins.hasAttr: first argument must be string, got {}",
          type_name(key.as_ref())
        ));
      };
      let attrs = force_if_thunk(&args[1])?;
      match attrs.as_ref() {
        Value::AttrSet(m) => Ok(Value::Bool(m.contains_key(k))),
        other => Err(anyhow!(
          "builtins.hasAttr: second argument must be attrset, got {}",
          type_name(other)
        )),
      }
    }
    "getAttr" if args.len() >= 2 => {
      let key = force_if_thunk(&args[0])?;
      let Some(attr_name) = key.as_ref().as_str() else {
        return Err(anyhow!(
          "builtins.getAttr: first argument must be string, got {}",
          key.as_ref()
        ));
      };
      let attrset = force_if_thunk(&args[1])?;
      let Value::AttrSet(attrs) = attrset.as_ref() else {
        return Err(anyhow!(
          "builtins.getAttr: second argument must be attrset, got {}",
          attrset.as_ref()
        ));
      };
      attrs
        .get(attr_name)
        .cloned()
        .ok_or_else(|| anyhow!("builtins.getAttr: attribute '{}' not found", attr_name))
    }
    "attrByPath" if args.len() >= 3 => {
      // `args` arrived verbatim because `attrByPath` is in
      // `no_force_at_all` — args[1] (the default) must stay
      // lazy until the path is known to be missing.
      let path_v = force_if_thunk(&args[0])?;
      let Value::List(path) = path_v.as_ref() else {
        return Err(anyhow!(
          "builtins.attrByPath: first argument must be list of strings, got {}",
          type_name(path_v.as_ref())
        ));
      };
      let default = args[1].clone();
      let mut current = args[2].clone();
      for segment in path.iter() {
        let segment = force_if_thunk(segment)?;
        let Some(name) = segment.as_ref().as_str() else {
          return Err(anyhow!(
            "builtins.attrByPath: path segment must be string, got {}",
            type_name(segment.as_ref())
          ));
        };
        let forced = force_if_thunk(&current)?;
        let Value::AttrSet(attrs) = forced.as_ref() else {
          return Ok(default);
        };
        let next = match attrs.get(name) {
          Some(v) => v.clone(),
          None => return Ok(default),
        };
        drop(forced);
        current = next;
      }
      Ok(current)
    }
    "getAttrs" if args.len() >= 2 => {
      let names_value = force_if_thunk(&args[0])?;
      let Value::List(names) = names_value.as_ref() else {
        return Err(anyhow!(
          "builtins.getAttrs: first argument must be list of strings, got {}",
          type_name(names_value.as_ref())
        ));
      };
      let attrs_value = force_if_thunk(&args[1])?;
      let Value::AttrSet(attrs) = attrs_value.as_ref() else {
        return Err(anyhow!(
          "builtins.getAttrs: second argument must be attrset, got {}",
          type_name(attrs_value.as_ref())
        ));
      };
      let mut result = BTreeMap::new();
      for name_v in names.iter() {
        let name_v = force_if_thunk(name_v)?;
        let Some(name) = name_v.as_ref().as_str() else {
          return Err(anyhow!(
            "builtins.getAttrs: name list element must be string, got {}",
            type_name(name_v.as_ref())
          ));
        };
        let value = attrs
          .get(name)
          .cloned()
          .ok_or_else(|| anyhow!("builtins.getAttrs: attribute '{}' missing in set", name))?;
        result.insert(name.to_string(), value);
      }
      Ok(Value::AttrSet(Arc::new(result)))
    }
    "typeOf" if args.len() >= 1 => {
      let value = force_if_thunk(&args[0])?;
      Ok(Value::String(
        match value.as_ref() {
          Value::Null => "null",
          Value::Bool(_) => "bool",
          Value::Int(_) => "int",
          Value::Float(_) => "float",
          Value::String(_) | Value::StringContext { .. } => "string",
          Value::Path(_) => "path",
          Value::List(_) => "list",
          Value::AttrSet(_) => "set",
          Value::Lambda { .. } | Value::BuiltinPartial { .. } => "lambda",
          Value::Thunk { .. } => "thunk",
        }
        .to_string(),
      ))
    }
    "toString" if args.len() >= 1 => {
      // 2026-05-05 (slice #49 + #57): preserve context for ALL
      // input shapes — not just `string + context`. Slice #49
      // closed the str-in case (string with context → result
      // inherits). Slice #57 closes the LIST case (every list
      // element's context is unioned into the result) AND the
      // PATH case (a path appearing inside the list adds its
      // display form to the result context, mirroring `${./p}`
      // interpolation semantics). The rewrite uses the new
      // `coerce_to_string_for_to_string_with_context` helper
      // which collects context recursively — `__toString` /
      // `outPath` recursive paths also propagate context now.
      //
      // 2026-05-05 (slice #58): cross-call cycle guard via
      // `InterpDepthGuard`. Pre-fix
      //   `let r = { __toString = self: builtins.toString self; };
      //    in builtins.toString r`
      // overflowed the Rust call stack with SIGABRT — `toString`
      // re-entered through the `__toString` lambda body which
      // itself called `toString` again, but the within-call
      // `depth` parameter reset to 0 each time. This is the
      // same cross-call cycle shape slice #40 closed for
      // string interpolation; reuse the same guard since the
      // cycle CAN cross between the two paths (a `__toString`
      // returning a string with `${...}` interpolation would
      // alternate between them). The within-call `depth` in
      // `coerce_to_string_for_to_string_with_context` still
      // protects per-call recursion; the thread-local guard
      // catches cross-call cycles before the Rust call stack
      // blows up.
      let _depth_guard = InterpDepthGuard::enter()?;
      let (text, context) = coerce_to_string_for_to_string_with_context(args[0].clone(), 0)?;
      Ok(Value::string_with_context(text, context))
    }
    "isNull" if args.len() >= 1 => {
      let value = force_if_thunk(&args[0])?;
      Ok(Value::Bool(matches!(value.as_ref(), Value::Null)))
    }
    // 2026-05-05: must accept BOTH `Value::String` AND
    // `Value::StringContext` — both ARE strings as far as the
    // user is concerned. The distinction is internal (provenance
    // tracking for context-bearing strings like `"x${./p}"`).
    // Pre-fix `isString "x${./p}"` returned `false`, which is a
    // Nix-canonical mismatch and a real production bug — user
    // code branching on `if isString x then ... else ...` would
    // silently take the wrong branch for any context-bearing
    // string. Same shape applies to `typeOf` (which already
    // handles both — see line ~2206) and to `as_str()` (which
    // also handles both). Only the `isString` predicate had the
    // narrow `Value::String(_)` match.
    "isString" if args.len() >= 1 => {
      let value = force_if_thunk(&args[0])?;
      Ok(Value::Bool(matches!(
        value.as_ref(),
        Value::String(_) | Value::StringContext { .. }
      )))
    }
    "isList" if args.len() >= 1 => {
      let value = force_if_thunk(&args[0])?;
      Ok(Value::Bool(matches!(value.as_ref(), Value::List(_))))
    }
    "isAttrs" if args.len() >= 1 => {
      let value = force_if_thunk(&args[0])?;
      Ok(Value::Bool(matches!(value.as_ref(), Value::AttrSet(_))))
    }
    "isBool" if args.len() >= 1 => {
      let value = force_if_thunk(&args[0])?;
      Ok(Value::Bool(matches!(value.as_ref(), Value::Bool(_))))
    }
    "isInt" if args.len() >= 1 => {
      let value = force_if_thunk(&args[0])?;
      Ok(Value::Bool(matches!(value.as_ref(), Value::Int(_))))
    }
    "isFloat" if args.len() >= 1 => {
      let value = force_if_thunk(&args[0])?;
      Ok(Value::Bool(matches!(value.as_ref(), Value::Float(_))))
    }
    // Finite/infinite/NaN tests. `isFinite` returns `true` for
    // any normal Int or finite Float and `false` for ±inf or NaN.
    // `isInf` and `isNaN` are scoped to floats — Ints can't
    // represent infinity in pnix.
    "isFinite" if args.len() >= 1 => {
      let value = force_if_thunk(&args[0])?;
      Ok(Value::Bool(match value.as_ref() {
        Value::Int(_) => true,
        Value::Float(f) => f.is_finite(),
        _ => false,
      }))
    }
    "isInf" if args.len() >= 1 => {
      let value = force_if_thunk(&args[0])?;
      Ok(Value::Bool(matches!(
        value.as_ref(),
        Value::Float(f) if f.is_infinite()
      )))
    }
    "isNaN" if args.len() >= 1 => {
      let value = force_if_thunk(&args[0])?;
      Ok(Value::Bool(matches!(
        value.as_ref(),
        Value::Float(f) if f.is_nan()
      )))
    }
    "isFunction" if args.len() >= 1 => {
      let value = force_if_thunk(&args[0])?;
      Ok(Value::Bool(matches!(
        value.as_ref(),
        Value::Lambda { .. } | Value::BuiltinPartial { .. }
      )))
    }
    "elemAt" if args.len() >= 2 => {
      // Nix-correct: negative index errors; out-of-bounds errors
      // ("list index N is out of bounds"). The previous
      // implementation silently returned `null` for both, which
      // bypassed `if r != null then …` guards.
      let index = force_if_thunk(&args[1])?;
      let i = expect_i64(index.as_ref(), "builtins.elemAt")?;
      let list = force_if_thunk(&args[0])?;
      let Value::List(l) = list.as_ref() else {
        return Err(anyhow!(
          "builtins.elemAt: first arg must be list, got {}",
          type_name(list.as_ref())
        ));
      };
      if i < 0 {
        return Err(anyhow!("builtins.elemAt: negative index {} not allowed", i));
      }
      let idx = i as usize;
      if idx >= l.len() {
        return Err(anyhow!(
          "builtins.elemAt: list index {} is out of bounds (length {})",
          i,
          l.len()
        ));
      }
      Ok(l[idx].clone())
    }
    "concatStringsSep" if args.len() >= 2 => {
      // Nix-correct: every list element must be a string. The
      // previous impl silently `to_json()`-ed non-string elements
      // (so `[ "a" 1 ]` became `"a,1"`), bypassing user guards.
      // Real Nix errors with "all list elements must be strings".
      //
      // 2026-05-05: also union string-context provenance from the
      // separator + every element. Previously the result was
      // always `Value::String` (no context), so user code that
      // passed context-bearing strings (`"x${./path}"`) through
      // `concatStringsSep` silently lost the path context that
      // real Nix uses to track derivation dependencies. Now
      // matches the binary `+` operator's context union shape
      // (interpret.rs line 1905).
      let sep_value = force_if_thunk(&args[0])?;
      let sep = sep_value.as_ref().as_str().ok_or_else(|| {
        anyhow!(
          "builtins.concatStringsSep: separator must be string, got {}",
          type_name(sep_value.as_ref())
        )
      })?;
      let list_value = force_if_thunk(&args[1])?;
      let items = match list_value.as_ref() {
        Value::List(l) => l,
        other => {
          return Err(anyhow!(
            "builtins.concatStringsSep: second arg must be list, got {}",
            type_name(other)
          ));
        }
      };
      let item_text_len = items
        .iter()
        .filter_map(|value| value.as_str().map(str::len))
        .fold(0usize, usize::saturating_add);
      let sep_text_len = items.len().saturating_sub(1).saturating_mul(sep.len());
      let mut out = String::with_capacity(sep_text_len.saturating_add(item_text_len));
      let mut combined: BTreeSet<String> = BTreeSet::new();
      if let Some(c) = sep_value.as_ref().string_context() {
        extend_string_context(&mut combined, c);
      }
      for (i, v) in items.iter().enumerate() {
        let value = force_if_thunk(v)?;
        match value.as_ref().as_str() {
          Some(s) => {
            if i > 0 {
              out.push_str(sep);
            }
            out.push_str(s);
            if let Some(c) = value.as_ref().string_context() {
              extend_string_context(&mut combined, c);
            }
          }
          None => {
            // Regression fix (2026-06-10): report the FORCED element's
            // type. With lazy list elements `v` itself is usually a
            // thunk here, so `type_name(v)` printed "is thunk" instead
            // of the element's actual type.
            return Err(anyhow!(
              "builtins.concatStringsSep: list element at index {} is {}, not a string",
              i,
              type_name(value.as_ref())
            ));
          }
        }
      }
      Ok(Value::string_with_context(out, combined))
    }
    // 2026-05-05 (slice #51): closed three silent shapes:
    //   1. non-list arg silently produced `""` (silent type-pass);
    //      now errors with `concatStrings: arg must be list, got <T>`.
    //   2. non-string list element was silently filter_map'd out
    //      (`[ "a" 1 "b" ]` → `"ab"`); now errors with
    //      `concatStrings: list element at index N is <T>, not string`.
    //      Same shape as the slice #38 (replaceStrings) and slice #49
    //      (concatStringsSep) tightenings.
    //   3. context-bearing inputs lost their context (the joined
    //      string returned `Value::String(_)` with no provenance);
    //      now unions every element's context. Mirrors the slice #49
    //      `concatStringsSep` propagation.
    "concatStrings" if args.len() >= 1 => {
      let list_value = force_if_thunk(&args[0])?;
      let items = match list_value.as_ref() {
        Value::List(l) => l,
        other => {
          return Err(anyhow!(
            "builtins.concatStrings: arg must be list, got {}",
            type_name(other)
          ));
        }
      };
      let item_text_len = items
        .iter()
        .filter_map(|value| value.as_str().map(str::len))
        .fold(0usize, usize::saturating_add);
      let mut out = String::with_capacity(item_text_len);
      let mut combined: BTreeSet<String> = BTreeSet::new();
      for (i, v) in items.iter().enumerate() {
        let value = force_if_thunk(v)?;
        match value.as_ref().as_str() {
          Some(s) => {
            out.push_str(s);
            if let Some(c) = value.as_ref().string_context() {
              extend_string_context(&mut combined, c);
            }
          }
          None => {
            // Regression fix (2026-06-10): same forced-type reporting
            // as concatStringsSep above.
            return Err(anyhow!(
              "builtins.concatStrings: list element at index {} is {}, not a string",
              i,
              type_name(value.as_ref())
            ));
          }
        }
      }
      Ok(Value::string_with_context(out, combined))
    }
    "sort" if args.len() >= 2 => {
      // 2026-05-05: previously the comparator's return value was
      // truthy-coerced (`Ok(Bool(true))` => Less, everything else
      // => Greater), so `sort (a: b: 42) [1 2]` silently returned
      // `[1 2]` and `sort (a: b: a < b) 42` silently returned `42`
      // (non-list identity). Both shapes hid bugs. Match Nix:
      // comparator must return bool, second arg must be list.
      let comparator = &args[0];
      match &args[1] {
        Value::List(items) => {
          let mut sort_err: Option<anyhow::Error> = None;
          let mut sorted = (**items).clone();
          sorted.sort_by(|a, b| {
            if sort_err.is_some() {
              return std::cmp::Ordering::Equal;
            }
            match apply_value(comparator.clone(), a.clone()).and_then(|f| apply_value(f, b.clone()))
            {
              Ok(Value::Bool(true)) => std::cmp::Ordering::Less,
              Ok(Value::Bool(false)) => std::cmp::Ordering::Greater,
              Ok(other) => {
                sort_err = Some(anyhow!(
                  "builtins.sort: comparator must return bool, got {}",
                  type_name(&other)
                ));
                std::cmp::Ordering::Equal
              }
              Err(e) => {
                sort_err = Some(e);
                std::cmp::Ordering::Equal
              }
            }
          });
          if let Some(e) = sort_err {
            return Err(e);
          }
          Ok(Value::List(Arc::new(sorted)))
        }
        other => Err(anyhow!(
          "builtins.sort: second argument must be list, got {}",
          type_name(other)
        )),
      }
    }
    "genList" if args.len() >= 2 => {
      // Nix-correct: negative count errors. The previous code
      // silently returned `[]` for negative `n`, which silently
      // bypassed user guards on length-arithmetic mistakes.
      //
      // 2026-05-05: also bound the count above. Previously
      // `builtins.genList (x: x) 9223372036854775807` (i64::MAX)
      // panicked with "capacity overflow" inside
      // `Vec::with_capacity` — a Rust process crash, not a
      // pnix error. A `.px` author who computed a count from
      // user input or an arithmetic chain that overflowed
      // would see the evaluator process exit. Cap the count at
      // a reasonable upper bound (16 MiB elements ≈ 256 MiB of
      // pointers — already pathologically large for any real
      // generated list, and small enough that Vec allocation
      // can't trip the OS allocator's hard limit). Larger
      // counts are user error and surface as a typed error.
      const GENLIST_MAX_COUNT: i64 = 16 * 1024 * 1024;
      let func = &args[0];
      let n = expect_i64(&args[1], "builtins.genList")?;
      if n < 0 {
        return Err(anyhow!(
          "builtins.genList: negative count {} not allowed",
          n
        ));
      }
      if n > GENLIST_MAX_COUNT {
        return Err(anyhow!(
          "builtins.genList: count {} exceeds maximum {} (request would allocate a vector that overflows Rust's Vec::with_capacity or exhausts the host allocator)",
          n,
          GENLIST_MAX_COUNT
        ));
      }
      // 2026-05-05 (slice #62): Nix-compat: `genList f n` is
      // lazy in the resulting elements. Each output element is
      // a thunk for `f i`; only forced when accessed. Pre-fix
      // `genList throw n` errored at construction time even if
      // the result was never indexed — broke patterns like
      // `length (genList ... 10)` or `head (genList ... 100)`.
      let mut result = Vec::with_capacity(n as usize);
      for i in 0..n {
        result.push(deferred_apply(func.clone(), Value::Int(i)));
      }
      Ok(Value::List(Arc::new(result)))
    }
    "all" if args.len() >= 2 => {
      let pred = &args[0];
      match &args[1] {
        Value::List(items) => {
          for (idx, item) in items.iter().enumerate() {
            // Predicate must return bool. The previous `is_true()`
            // truthy-coerced any non-false / non-null value, so
            // `all (x: 42) [1]` returned `true` silently. `partition`
            // and `filterAttrs` already errored on non-bool — `all`
            // and `any` were the inconsistent two.
            match apply_value(pred.clone(), item.clone())? {
              Value::Bool(true) => continue,
              Value::Bool(false) => return Ok(Value::Bool(false)),
              other => {
                return Err(anyhow!(
                  "builtins.all: predicate must return bool, got {} at index {}",
                  type_name(&other),
                  idx
                ));
              }
            }
          }
          Ok(Value::Bool(true))
        }
        other => Err(anyhow!(
          "builtins.all: second argument must be list, got {}",
          type_name(other)
        )),
      }
    }
    "any" if args.len() >= 2 => {
      let pred = &args[0];
      match &args[1] {
        Value::List(items) => {
          for (idx, item) in items.iter().enumerate() {
            match apply_value(pred.clone(), item.clone())? {
              Value::Bool(true) => return Ok(Value::Bool(true)),
              Value::Bool(false) => continue,
              other => {
                return Err(anyhow!(
                  "builtins.any: predicate must return bool, got {} at index {}",
                  type_name(&other),
                  idx
                ));
              }
            }
          }
          Ok(Value::Bool(false))
        }
        other => Err(anyhow!(
          "builtins.any: second argument must be list, got {}",
          type_name(other)
        )),
      }
    }
    "foldl'" if args.len() >= 3 => {
      // We came in with NO force at the boundary (foldl' is registered
      // as `no_force_at_all`). Force the bits we need:
      //  - func: must be a function-like value to apply.
      //  - list: must be a list to iterate; force its spine.
      // The init accumulator stays unforced — that is the whole point of
      // the lazy-init semantics. Each `op acc x` may or may not touch
      // `acc`, and `op` controls when it is forced.
      let func = force_if_thunk(&args[0])?;
      let init = args[1].clone();
      let list = force_if_thunk(&args[2])?;
      match list {
        Cow::Owned(Value::List(items)) => {
          let mut acc = init;
          for item in Arc::unwrap_or_clone(items) {
            acc = apply_value(apply_value(func.as_ref().clone(), acc)?, item)?;
          }
          Ok(acc)
        }
        Cow::Borrowed(Value::List(items)) => {
          let mut acc = init;
          for item in items.iter() {
            acc = apply_value(apply_value(func.as_ref().clone(), acc)?, item.clone())?;
          }
          Ok(acc)
        }
        other => Err(anyhow!(
          "builtins.foldl': third arg must be list, got {}",
          type_name(other.as_ref())
        )),
      }
    }
    "foldr" if args.len() >= 3 => {
      // Right fold: `foldr op nul [x1 x2 x3]` = `op x1 (op x2 (op x3 nul))`.
      // Note arg order: `op item acc` (item-first), opposite of foldl'.
      // Like foldl', `nul` is left lazy at the boundary so
      // `foldr op (throw "x") []` returns the unforced initial without
      // evaluating it (Nix-compat).
      let func = force_if_thunk(&args[0])?;
      let init = args[1].clone();
      let list = force_if_thunk(&args[2])?;
      match list {
        Cow::Owned(Value::List(items)) => {
          let mut acc = init;
          for item in Arc::unwrap_or_clone(items).into_iter().rev() {
            acc = apply_value(apply_value(func.as_ref().clone(), item)?, acc)?;
          }
          Ok(acc)
        }
        Cow::Borrowed(Value::List(items)) => {
          let mut acc = init;
          for item in items.iter().rev() {
            acc = apply_value(apply_value(func.as_ref().clone(), item.clone())?, acc)?;
          }
          Ok(acc)
        }
        other => Err(anyhow!(
          "builtins.foldr: third arg must be list, got {}",
          type_name(other.as_ref())
        )),
      }
    }
    "concatLists" if args.len() >= 1 => match &args[0] {
      Value::List(lists) => {
        let mut result = Vec::with_capacity(lists.len());
        for (idx, l) in lists.iter().enumerate() {
          // Force each element so a lazy thunk that resolves to a
          // non-list still triggers the type guard rather than being
          // silently dropped (the previous `if let Value::List(...)`
          // path treated thunks and non-lists alike as empty).
          let forced = force_if_thunk(l)?;
          match forced {
            Cow::Owned(Value::List(items)) => {
              result.reserve(items.len());
              result.extend(Arc::unwrap_or_clone(items));
            }
            Cow::Borrowed(Value::List(items)) => {
              result.reserve(items.len());
              result.extend(items.iter().cloned());
            }
            Cow::Owned(other) => {
              return Err(anyhow!(
                "builtins.concatLists: list element at index {} is {}, not a list",
                idx,
                type_name(&other)
              ));
            }
            other => {
              return Err(anyhow!(
                "builtins.concatLists: list element at index {} is {}, not a list",
                idx,
                type_name(other.as_ref())
              ));
            }
          }
        }
        Ok(Value::List(Arc::new(result)))
      }
      other => Err(anyhow!(
        "builtins.concatLists: argument must be list, got {}",
        type_name(other)
      )),
    },
    "concatMap" if args.len() >= 2 => {
      let func = &args[0];
      match &args[1] {
        Value::List(items) => {
          let mut result = Vec::with_capacity(items.len());
          for item in items.iter() {
            let mapped = apply_value(func.clone(), item.clone())?;
            match force_if_thunk(&mapped)? {
              Cow::Owned(Value::List(values)) => {
                result.reserve(values.len());
                result.extend(Arc::unwrap_or_clone(values));
              }
              Cow::Borrowed(Value::List(values)) => {
                result.reserve(values.len());
                result.extend(values.iter().cloned());
              }
              other => {
                return Err(anyhow!(
                  "builtins.concatMap: function must return list, got {}",
                  type_name(other.as_ref())
                ))
              }
            }
          }
          Ok(Value::List(Arc::new(result)))
        }
        _ => Err(anyhow!("builtins.concatMap: second arg must be list")),
      }
    }
    "replaceStrings" if args.len() >= 3 => {
      // Nix-compat: scan the input character by character, looking up the
      // longest matching `from` pattern. Replacements (`to[i]`) are forced
      // ONLY when the corresponding pattern actually matches — `to[j]` for
      // a non-matching `from[j]` (e.g. `throw "unreachable"`) must stay
      // unevaluated. Each `from[i]` is forced to a String when checked.
      //
      // 2026-05-05: previously the `from` / `to` arms fell back to an
      // empty `Vec` when the argument wasn't a list. That meant
      // `replaceStrings 42 99 "abc"`, `replaceStrings "f" "t" "abc"`,
      // and `replaceStrings null null "abc"` all silently returned
      // the haystack unchanged because the post-coerce length check
      // saw `0 == 0` and the for-loop walked over zero patterns.
      // An author who swapped arg order, fed in a misconfigured
      // upstream, or coerced from a JSON loader that returned a
      // string instead of a list would see "no replacements
      // happened" instead of a typed error. Match the rest of the
      // pnix builtin family (`filter` / `elem` / `concatStringsSep`
      // — slices #32 / #33): non-list `from` or `to` is a typed
      // error pinning the offending argument.
      let from_items: &[Value] = match &args[0] {
        Value::List(l) => l.as_slice(),
        other => {
          return Err(anyhow!(
            "builtins.replaceStrings: 'from' must be list, got {}",
            type_name(other)
          ));
        }
      };
      let to_items: &[Value] = match &args[1] {
        Value::List(l) => l.as_slice(),
        other => {
          return Err(anyhow!(
            "builtins.replaceStrings: 'to' must be list, got {}",
            type_name(other)
          ));
        }
      };
      if from_items.len() != to_items.len() {
        return Err(anyhow!(
          "builtins.replaceStrings: 'from' and 'to' lists must have equal length"
        ));
      }
      let haystack = args[2]
        .as_str()
        .ok_or_else(|| anyhow!("builtins.replaceStrings: third argument must be string"))?;
      if from_items.is_empty() {
        return Ok(args[2].clone());
      }

      // Pre-force each `from` to a string (we have to know their text to scan),
      // but defer forcing each `to` until its `from` matches.
      let mut from_strs = Vec::with_capacity(from_items.len());
      for item in from_items {
        let v = force_if_thunk(item)?;
        let Some(s) = v.as_ref().as_str() else {
          return Err(anyhow!(
            "builtins.replaceStrings: 'from' element must be string"
          ));
        };
        from_strs.push(s.to_string());
      }

      // 2026-05-05: union string-context from haystack + every
      // `to[i]` element that actually gets used. Real Nix
      // propagates context through replaceStrings — the result
      // string carries the union of all input-string contexts
      // that contributed to it. Pnix used to produce a bare
      // `Value::String`, dropping context silently. The `to[i]`
      // contexts are unioned only on first-use (matching the
      // existing laziness contract: `to[j]` for an unused `j`
      // remains unforced AND its context is not added).
      let mut combined: BTreeSet<String> = BTreeSet::new();
      if let Some(c) = args[2].string_context() {
        extend_string_context(&mut combined, c);
      }
      let mut out = String::with_capacity(haystack.len());
      let bytes = haystack.as_bytes();
      let mut i = 0;
      // Cache forced `to` strings (and their contexts) so a
      // multi-match pattern doesn't re-force or re-add context.
      let mut to_cached: Vec<Option<String>> = vec![None; to_items.len()];
      let mut to_ctx_added: Vec<bool> = vec![false; to_items.len()];
      while i <= bytes.len() {
        let mut matched = None;
        for (idx, f) in from_strs.iter().enumerate() {
          let fb = f.as_bytes();
          if i + fb.len() <= bytes.len() && &bytes[i..i + fb.len()] == fb {
            matched = Some((idx, fb.len()));
            break;
          }
        }
        match matched {
          Some((idx, flen)) => {
            if to_cached[idx].is_none() {
              let v = force_if_thunk(&to_items[idx])?;
              let s = v
                .as_ref()
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| anyhow!("builtins.replaceStrings: 'to' element must be string"))?;
              if !to_ctx_added[idx] {
                if let Some(c) = v.as_ref().string_context() {
                  extend_string_context(&mut combined, c);
                }
                to_ctx_added[idx] = true;
              }
              to_cached[idx] = Some(s);
            }
            out.push_str(to_cached[idx].as_ref().unwrap());
            if flen == 0 {
              // Empty pattern: emit one source character then advance.
              // Matching remains byte-indexed, but copying bytes as
              // `u8 as char` corrupts UTF-8 (`한국어` became mojibake).
              if i < bytes.len() {
                let ch = haystack[i..]
                  .chars()
                  .next()
                  .expect("i is maintained on utf-8 char boundaries");
                out.push(ch);
                i += ch.len_utf8();
              } else {
                i += 1;
              }
            } else {
              i += flen;
            }
          }
          None => {
            if i < bytes.len() {
              let ch = haystack[i..]
                .chars()
                .next()
                .expect("i is maintained on utf-8 char boundaries");
              out.push(ch);
              i += ch.len_utf8();
            } else {
              i += 1;
            }
          }
        }
      }
      Ok(Value::string_with_context(out, combined))
    }
    // 2026-05-05 (slice #52): the narrow `Value::String(_)` match
    // was the same parity miss as slice #50's `isString`. A
    // context-bearing string IS still a string from the .px
    // author's POV (the distinction is internal provenance
    // tracking), and the key function returning `"x${./p}"` is
    // returning a perfectly valid string. Pre-fix the error
    // message lied — it claimed the function "must return string"
    // when the function DID return a string. Use `as_str()` which
    // handles both `Value::String` AND `Value::StringContext`.
    // Note: attrset keys are stored as plain `String` in the
    // `BTreeMap`, so we drop context at the boundary; the .px
    // author can re-fetch context via `getContext` on the input
    // list elements if needed.
    "groupBy" if args.len() >= 2 => {
      let key_fn = &args[0];
      match &args[1] {
        Value::List(items) => {
          let mut groups: BTreeMap<String, Vec<Value>> = BTreeMap::new();
          for item in items.iter() {
            let key_value = apply_value(key_fn.clone(), item.clone())?;
            let Some(key) = key_value.as_str() else {
              return Err(anyhow!(
                "builtins.groupBy: key function must return string, got {}",
                type_name(&key_value)
              ));
            };
            groups
              .entry(key.to_string())
              .or_default()
              .push(item.clone());
          }
          let mut out = BTreeMap::new();
          for (key, values) in groups {
            out.insert(key, Value::List(Arc::new(values)));
          }
          Ok(Value::AttrSet(Arc::new(out)))
        }
        other => Err(anyhow!(
          "builtins.groupBy: second arg must be list, got {}",
          type_name(other)
        )),
      }
    }
    "catAttrs" if args.len() >= 2 => {
      let Some(attr_name) = args[0].as_str() else {
        return Err(anyhow!("builtins.catAttrs: first arg must be string"));
      };
      match &args[1] {
        Value::List(items) => {
          let mut result = Vec::with_capacity(items.len());
          for item in items.iter() {
            let forced = force_if_thunk(item)?;
            let Value::AttrSet(attrs) = forced.as_ref() else {
              return Err(anyhow!(
                "builtins.catAttrs: list element must be attrset, got {}",
                forced.as_ref()
              ));
            };
            if let Some(value) = attrs.get(attr_name) {
              result.push(value.clone());
            }
          }
          Ok(Value::List(Arc::new(result)))
        }
        _ => Err(anyhow!("builtins.catAttrs: second arg must be list")),
      }
    }
    "partition" if args.len() >= 2 => {
      let pred = &args[0];
      match &args[1] {
        Value::List(items) => {
          let mut right = Vec::with_capacity(items.len() / 2);
          let mut wrong = Vec::with_capacity(items.len() / 2);
          for item in items.iter() {
            match apply_value(pred.clone(), item.clone())? {
              Value::Bool(true) => right.push(item.clone()),
              Value::Bool(false) => wrong.push(item.clone()),
              other => {
                return Err(anyhow!(
                  "builtins.partition: predicate must return bool, got {}",
                  other
                ))
              }
            }
          }
          let mut result = BTreeMap::new();
          result.insert("right".to_string(), Value::List(Arc::new(right)));
          result.insert("wrong".to_string(), Value::List(Arc::new(wrong)));
          Ok(Value::AttrSet(Arc::new(result)))
        }
        _ => Err(anyhow!("builtins.partition: second arg must be list")),
      }
    }
    // `builtins.and / or / not` mirror the language operators —
    // bool-only, no truthy-coerce. These are pnix-only helpers
    // (they don't exist in real Nix) but they exist precisely so
    // `.px` authors can pipe boolean expressions through `map` /
    // `foldl`, so they should match the operator contract.
    "and" if args.len() >= 2 => Ok(Value::Bool(
      expect_bool(&args[0], "builtins.and: first arg")?
        && expect_bool(&args[1], "builtins.and: second arg")?,
    )),
    "or" if args.len() >= 2 => Ok(Value::Bool(
      expect_bool(&args[0], "builtins.or: first arg")?
        || expect_bool(&args[1], "builtins.or: second arg")?,
    )),
    "not" if args.len() >= 1 => Ok(Value::Bool(!expect_bool(&args[0], "builtins.not: arg")?)),
    "eq" if args.len() >= 2 => Ok(Value::Bool(values_equal(&args[0], &args[1])?)),
    "lt" if args.len() >= 2 => Ok(Value::Bool(compare_values(&args[0], &args[1])?.is_lt())),
    "le" if args.len() >= 2 => Ok(Value::Bool(!compare_values(&args[0], &args[1])?.is_gt())),
    "gt" if args.len() >= 2 => Ok(Value::Bool(compare_values(&args[0], &args[1])?.is_gt())),
    "ge" if args.len() >= 2 => Ok(Value::Bool(!compare_values(&args[0], &args[1])?.is_lt())),
    "append" if args.len() >= 2 => {
      let lhs_value = force_if_thunk(&args[0])?;
      let Value::List(lhs) = lhs_value.as_ref() else {
        return Err(anyhow!(
          "builtins.append: first arg must be list, got {}",
          type_name(lhs_value.as_ref())
        ));
      };
      let rhs_value = force_if_thunk(&args[1])?;
      let Value::List(rhs) = rhs_value.as_ref() else {
        return Err(anyhow!(
          "builtins.append: second arg must be list, got {}",
          type_name(rhs_value.as_ref())
        ));
      };
      let mut out = Vec::with_capacity(lhs.len() + rhs.len());
      extend_value_list(&mut out, lhs);
      extend_value_list(&mut out, rhs);
      Ok(Value::List(Arc::new(out)))
    }
    "cons" if args.len() >= 2 => {
      let tail_value = force_if_thunk(&args[1])?;
      let Value::List(tail) = tail_value.as_ref() else {
        return Err(anyhow!(
          "builtins.cons: second arg must be list, got {}",
          type_name(tail_value.as_ref())
        ));
      };
      let mut out = Vec::with_capacity(tail.len() + 1);
      out.push(args[0].clone());
      extend_value_list(&mut out, tail);
      Ok(Value::List(Arc::new(out)))
    }
    "reverse" | "reverseList" if args.len() >= 1 => match force_if_thunk(&args[0])?.as_ref() {
      Value::List(items) => {
        let mut out = Vec::with_capacity(items.len());
        for item in items.iter().rev() {
          out.push(item.clone());
        }
        Ok(Value::List(Arc::new(out)))
      }
      _ => Err(anyhow!("builtins.reverse: expected list")),
    },
    "zip" if args.len() >= 2 => {
      let lhs_value = force_if_thunk(&args[0])?;
      let Value::List(lhs) = lhs_value.as_ref() else {
        return Err(anyhow!(
          "builtins.zip: first arg must be list, got {}",
          type_name(lhs_value.as_ref())
        ));
      };
      let rhs_value = force_if_thunk(&args[1])?;
      let Value::List(rhs) = rhs_value.as_ref() else {
        return Err(anyhow!(
          "builtins.zip: second arg must be list, got {}",
          type_name(rhs_value.as_ref())
        ));
      };
      let mut out = Vec::with_capacity(lhs.len().min(rhs.len()));
      for (a, b) in lhs.iter().zip(rhs.iter()) {
        out.push(Value::List(Arc::new(vec![a.clone(), b.clone()])));
      }
      Ok(Value::List(Arc::new(out)))
    }
    "flatten" if args.len() >= 1 => {
      let value = force_if_thunk(&args[0])?;
      let items = match value.as_ref() {
        Value::List(items) => items,
        _ => return Err(anyhow!("builtins.flatten: expected list")),
      };
      {
        let mut out = Vec::with_capacity(items.len());
        for item in items.iter() {
          flatten_value_for_builtin(item, &mut out)?;
        }
        Ok(Value::List(Arc::new(out)))
      }
    }
    // 2026-05-05 (slice #71): accept context-bearing strings.
    // Pre-fix the match arm `Value::String(s) => s` only
    // handled plain strings — context-bearing strings fell
    // through to `_ => Err("expected string")`. So
    // `getEnv ("PATH" + ./marker)` (or any string built via
    // slice #54's `+ Path` semantics) errored despite both
    // shapes being strings. Same parity-miss family as slice
    // #50 (`isString`) and slice #64 (`resolve_value_path`):
    // any builtin that takes a string arg should use
    // `as_str()` or accept both `Value::String` AND
    // `Value::StringContext`. Closes the parity gap for the
    // env-var lookup family.
    "getEnv" if args.len() >= 1 => {
      let Some(name) = args[0].as_str() else {
        return Err(anyhow!(
          "builtins.getEnv: expected string, got {}",
          type_name(&args[0])
        ));
      };
      if getenv_allowed(name) {
        Ok(Value::String(std::env::var(name).unwrap_or_default()))
      } else {
        Ok(Value::String(String::new()))
      }
    }
    "tryEval" if args.len() >= 1 => Ok(try_eval_result(Ok(args[0].clone()))),
    "seq" if args.len() >= 2 => Ok(args[1].clone()),
    "deepSeq" if args.len() >= 2 => {
      deep_force_value(&args[0])?;
      Ok(args[1].clone())
    }
    "trace" if args.len() >= 2 => {
      let message = force_if_thunk(&args[0])?;
      emit_trace_message(message.as_ref());
      Ok(args[1].clone())
    }
    "traceVerbose" if args.len() >= 2 => {
      if verbose_mode_enabled() {
        let message = force_if_thunk(&args[0])?;
        emit_trace_message(message.as_ref());
      }
      Ok(args[1].clone())
    }
    "warn" if args.len() >= 2 => {
      let message_value = force_if_thunk(&args[0])?;
      let Some(message) = message_value.as_ref().as_str() else {
        return Err(anyhow!("builtins.warn: expected string message"));
      };
      eprintln!("warning: {}", message);
      Ok(args[1].clone())
    }
    // 2026-05-05 (slice #53): the first argument is the error
    // context message — Nix-canonical contract is `addErrorContext
    // contextString value`. Pre-fix any first-arg type was silently
    // accepted (`addErrorContext 42 "v"` returned `"v"` cleanly),
    // which hid `.px` author errors where the context arg was
    // accidentally a non-string (often a forgotten `toString` call
    // or wrong-shape interpolation). The contract documents a
    // string; non-string is a programming error and must error
    // loud. Mirror `warn` (line ~3603) which already validates its
    // string message arg. Note: the value (args[1]) is returned
    // unchanged whether or not it errors at evaluation; pnix's
    // current implementation does NOT enrich error messages with
    // the context string (deferred Nix-compat surface), so the
    // type guard is effectively the only behavioural change.
    "addErrorContext" if args.len() >= 2 => {
      let context_value = force_if_thunk(&args[0])?;
      let Some(_context) = context_value.as_ref().as_str() else {
        return Err(anyhow!(
          "builtins.addErrorContext: context arg must be string, got {}",
          type_name(context_value.as_ref())
        ));
      };
      Ok(args[1].clone())
    }
    "functionArgs" if args.len() >= 1 => function_args_result(&args[0]),
    "schemaValidate" if args.len() >= 2 => schema::schema_validate(&args[0], &args[1]),
    "schemaNormalize" if args.len() >= 2 => schema::schema_normalize(&args[0], &args[1]),
    "schemaExplain" if args.len() >= 2 => schema::schema_explain(&args[0], &args[1]),
    // 2026-05-05 (slice #71): same context-bearing string
    // parity miss as `getEnv` and the slice #50/#64 family.
    // `xmlParse` and `htmlParse` only matched `Value::String`,
    // so context-bearing strings (e.g., XML text constructed
    // via `+ ./xml-source` per slice #54) errored despite
    // both being strings. `as_str()` handles both variants.
    "xmlParse" if args.len() >= 1 => match args[0].as_str() {
      Some(input) => markup::xml_parse(input),
      None => Err(anyhow!(
        "builtins.xmlParse: expected string, got {}",
        type_name(&args[0])
      )),
    },
    // 2026-05-05 (slice #73): emit functions propagate context.
    // Pre-fix `xmlEmit`/`htmlEmit`/`svgEmit`/`mathmlEmit`/
    // `openmathEmit` returned plain `Value::String`, silently
    // dropping context from any context-bearing string buried
    // in the input attrset tree (attribute values, text
    // children, etc.). Same family as slice #57 (`toString` of
    // list-of-context-strings drops context). Walk the input
    // tree and union all contexts; wrap result with
    // `Value::string_with_context` (preserving the same text).
    "xmlEmit" if args.len() >= 1 => {
      let text = markup::xml_emit(&args[0])?;
      let ctx = collect_value_contexts(&args[0]);
      Ok(Value::string_with_context(text, ctx))
    }
    "htmlParse" if args.len() >= 1 => match args[0].as_str() {
      Some(input) => markup::html_parse(input),
      None => Err(anyhow!(
        "builtins.htmlParse: expected string, got {}",
        type_name(&args[0])
      )),
    },
    "htmlEmit" if args.len() >= 1 => {
      let text = markup::html_emit(&args[0])?;
      let ctx = collect_value_contexts(&args[0]);
      Ok(Value::string_with_context(text, ctx))
    }
    "svgSchemaNormalize" if args.len() >= 1 => svg::svg_schema_normalize(&args[0]),
    "svgSchemaValidate" if args.len() >= 1 => svg::svg_schema_validate(&args[0]),
    "svgSchemaExplain" if args.len() >= 1 => svg::svg_schema_explain(&args[0]),
    "svgEmit" if args.len() >= 1 => {
      let text = svg::svg_emit(&args[0])?;
      let ctx = collect_value_contexts(&args[0]);
      Ok(Value::string_with_context(text, ctx))
    }
    "svgRenderPacket" if args.len() >= 2 => svg::svg_render_packet(&args[0], &args[1]),
    "mathmlXmlToJson" if args.len() >= 1 => math_markup::mathml_xml_to_json(&args[0]),
    "openmathXmlToJson" if args.len() >= 1 => math_markup::openmath_xml_to_json(&args[0]),
    "mathmlEmit" if args.len() >= 1 => {
      let text = math_markup::mathml_emit(&args[0])?;
      let ctx = collect_value_contexts(&args[0]);
      Ok(Value::string_with_context(text, ctx))
    }
    "openmathEmit" if args.len() >= 1 => {
      let text = math_markup::openmath_emit(&args[0])?;
      let ctx = collect_value_contexts(&args[0]);
      Ok(Value::string_with_context(text, ctx))
    }
    "x3dXmlToJson" if args.len() >= 1 => x3d::x3d_xml_to_json(&args[0]),
    "x3dSchemaNormalize" if args.len() >= 1 => x3d::x3d_schema_normalize(&args[0]),
    "x3dSchemaValidate" if args.len() >= 1 => x3d::x3d_schema_validate(&args[0]),
    "x3dSchemaExplain" if args.len() >= 1 => x3d::x3d_schema_explain(&args[0]),
    "x3dFrpGraph" if args.len() >= 1 => x3d::x3d_frp_graph(&args[0]),
    "x3dSyncPlan" if args.len() >= 2 => x3d::x3d_sync_plan(&args[0], &args[1]),
    "x3dX3domFragment" if args.len() >= 1 => x3d::x3d_x3dom_fragment(&args[0]),
    "x3dX3domHtml" if args.len() >= 1 => x3d::x3d_x3dom_html(&args[0]),
    "x3dX3domPatch" if args.len() >= 2 => x3d::x3d_x3dom_patch(&args[0], &args[1]),
    "x3dRenderPacket" if args.len() >= 2 => x3d::x3d_render_packet(&args[0], &args[1]),
    // Bio domain schemas — clean-room XML-family impl. Mirror the
    // svg/x3d normalize/validate/explain shape so wrapper signatures
    // (`XmlAst → XmlAst`, `XmlAst → AttrSet`, `XmlAst → String`)
    // hold without a domain-specific native crate.
    "cellmlSchemaNormalize" if args.len() >= 1 => {
      xml_format_schema::xml_family_normalize(&args[0], "builtins.cellmlSchemaNormalize")
    }
    "cellmlSchemaValidate" if args.len() >= 1 => {
      xml_format_schema::xml_family_validate(&args[0], "builtins.cellmlSchemaValidate")
    }
    "cellmlSchemaExplain" if args.len() >= 1 => {
      xml_format_schema::xml_family_explain(&args[0], "builtins.cellmlSchemaExplain")
    }
    "cmlSchemaNormalize" if args.len() >= 1 => {
      xml_format_schema::xml_family_normalize(&args[0], "builtins.cmlSchemaNormalize")
    }
    "cmlSchemaValidate" if args.len() >= 1 => {
      xml_format_schema::xml_family_validate(&args[0], "builtins.cmlSchemaValidate")
    }
    "cmlSchemaExplain" if args.len() >= 1 => {
      xml_format_schema::xml_family_explain(&args[0], "builtins.cmlSchemaExplain")
    }
    "neuromlSchemaNormalize" if args.len() >= 1 => {
      xml_format_schema::xml_family_normalize(&args[0], "builtins.neuromlSchemaNormalize")
    }
    "neuromlSchemaValidate" if args.len() >= 1 => {
      xml_format_schema::xml_family_validate(&args[0], "builtins.neuromlSchemaValidate")
    }
    "neuromlSchemaExplain" if args.len() >= 1 => {
      xml_format_schema::xml_family_explain(&args[0], "builtins.neuromlSchemaExplain")
    }
    "pdbmlSchemaNormalize" if args.len() >= 1 => {
      xml_format_schema::xml_family_normalize(&args[0], "builtins.pdbmlSchemaNormalize")
    }
    "pdbmlSchemaValidate" if args.len() >= 1 => {
      xml_format_schema::xml_family_validate(&args[0], "builtins.pdbmlSchemaValidate")
    }
    "pdbmlSchemaExplain" if args.len() >= 1 => {
      xml_format_schema::xml_family_explain(&args[0], "builtins.pdbmlSchemaExplain")
    }
    "sbmlSchemaNormalize" if args.len() >= 1 => {
      xml_format_schema::xml_family_normalize(&args[0], "builtins.sbmlSchemaNormalize")
    }
    "sbmlSchemaValidate" if args.len() >= 1 => {
      xml_format_schema::xml_family_validate(&args[0], "builtins.sbmlSchemaValidate")
    }
    "sbmlSchemaExplain" if args.len() >= 1 => {
      xml_format_schema::xml_family_explain(&args[0], "builtins.sbmlSchemaExplain")
    }
    // Bio/scientific clean-room XML-family validators (no native codec yet — input
    // AST validated with pnix-xml-core). Future native crates can replace
    // each body without changing the wrapper signature.
    "biopaxSchemaNormalize" if args.len() >= 1 => {
      xml_format_schema::xml_family_normalize(&args[0], "builtins.biopaxSchemaNormalize")
    }
    "biopaxSchemaValidate" if args.len() >= 1 => {
      xml_format_schema::xml_family_validate(&args[0], "builtins.biopaxSchemaValidate")
    }
    "biopaxSchemaExplain" if args.len() >= 1 => {
      xml_format_schema::xml_family_explain(&args[0], "builtins.biopaxSchemaExplain")
    }
    "giftiSchemaNormalize" if args.len() >= 1 => {
      xml_format_schema::xml_family_normalize(&args[0], "builtins.giftiSchemaNormalize")
    }
    "giftiSchemaValidate" if args.len() >= 1 => {
      xml_format_schema::xml_family_validate(&args[0], "builtins.giftiSchemaValidate")
    }
    "giftiSchemaExplain" if args.len() >= 1 => {
      xml_format_schema::xml_family_explain(&args[0], "builtins.giftiSchemaExplain")
    }
    "lemsSchemaNormalize" if args.len() >= 1 => {
      xml_format_schema::xml_family_normalize(&args[0], "builtins.lemsSchemaNormalize")
    }
    "lemsSchemaValidate" if args.len() >= 1 => {
      xml_format_schema::xml_family_validate(&args[0], "builtins.lemsSchemaValidate")
    }
    "lemsSchemaExplain" if args.len() >= 1 => {
      xml_format_schema::xml_family_explain(&args[0], "builtins.lemsSchemaExplain")
    }
    "omexSchemaNormalize" if args.len() >= 1 => {
      xml_format_schema::xml_family_normalize(&args[0], "builtins.omexSchemaNormalize")
    }
    "omexSchemaValidate" if args.len() >= 1 => {
      xml_format_schema::xml_family_validate(&args[0], "builtins.omexSchemaValidate")
    }
    "omexSchemaExplain" if args.len() >= 1 => {
      xml_format_schema::xml_family_explain(&args[0], "builtins.omexSchemaExplain")
    }
    "pharmmlSchemaNormalize" if args.len() >= 1 => {
      xml_format_schema::xml_family_normalize(&args[0], "builtins.pharmmlSchemaNormalize")
    }
    "pharmmlSchemaValidate" if args.len() >= 1 => {
      xml_format_schema::xml_family_validate(&args[0], "builtins.pharmmlSchemaValidate")
    }
    "pharmmlSchemaExplain" if args.len() >= 1 => {
      xml_format_schema::xml_family_explain(&args[0], "builtins.pharmmlSchemaExplain")
    }
    "sbgnmlSchemaNormalize" if args.len() >= 1 => {
      xml_format_schema::xml_family_normalize(&args[0], "builtins.sbgnmlSchemaNormalize")
    }
    "sbgnmlSchemaValidate" if args.len() >= 1 => {
      xml_format_schema::xml_family_validate(&args[0], "builtins.sbgnmlSchemaValidate")
    }
    "sbgnmlSchemaExplain" if args.len() >= 1 => {
      xml_format_schema::xml_family_explain(&args[0], "builtins.sbgnmlSchemaExplain")
    }
    "sedmlSchemaNormalize" if args.len() >= 1 => {
      xml_format_schema::xml_family_normalize(&args[0], "builtins.sedmlSchemaNormalize")
    }
    "sedmlSchemaValidate" if args.len() >= 1 => {
      xml_format_schema::xml_family_validate(&args[0], "builtins.sedmlSchemaValidate")
    }
    "sedmlSchemaExplain" if args.len() >= 1 => {
      xml_format_schema::xml_family_explain(&args[0], "builtins.sedmlSchemaExplain")
    }
    "vtkSchemaNormalize" if args.len() >= 1 => {
      xml_format_schema::xml_family_normalize(&args[0], "builtins.vtkSchemaNormalize")
    }
    "vtkSchemaValidate" if args.len() >= 1 => {
      xml_format_schema::xml_family_validate(&args[0], "builtins.vtkSchemaValidate")
    }
    "vtkSchemaExplain" if args.len() >= 1 => {
      xml_format_schema::xml_family_explain(&args[0], "builtins.vtkSchemaExplain")
    }
    "xdmfSchemaNormalize" if args.len() >= 1 => {
      xml_format_schema::xml_family_normalize(&args[0], "builtins.xdmfSchemaNormalize")
    }
    "xdmfSchemaValidate" if args.len() >= 1 => {
      xml_format_schema::xml_family_validate(&args[0], "builtins.xdmfSchemaValidate")
    }
    "xdmfSchemaExplain" if args.len() >= 1 => {
      xml_format_schema::xml_family_explain(&args[0], "builtins.xdmfSchemaExplain")
    }
    "ifcxmlSchemaNormalize" if args.len() >= 1 => {
      xml_format_schema::xml_family_normalize(&args[0], "builtins.ifcxmlSchemaNormalize")
    }
    "ifcxmlSchemaValidate" if args.len() >= 1 => {
      xml_format_schema::xml_family_validate(&args[0], "builtins.ifcxmlSchemaValidate")
    }
    "ifcxmlSchemaExplain" if args.len() >= 1 => {
      xml_format_schema::xml_family_explain(&args[0], "builtins.ifcxmlSchemaExplain")
    }
    "mathmlSchemaNormalize" if args.len() >= 1 => {
      xml_format_schema::xml_family_normalize(&args[0], "builtins.mathmlSchemaNormalize")
    }
    "mathmlSchemaValidate" if args.len() >= 1 => {
      xml_format_schema::xml_family_validate(&args[0], "builtins.mathmlSchemaValidate")
    }
    "mathmlSchemaExplain" if args.len() >= 1 => {
      xml_format_schema::xml_family_explain(&args[0], "builtins.mathmlSchemaExplain")
    }
    "openmathSchemaNormalize" if args.len() >= 1 => {
      xml_format_schema::xml_family_normalize(&args[0], "builtins.openmathSchemaNormalize")
    }
    "openmathSchemaValidate" if args.len() >= 1 => {
      xml_format_schema::xml_family_validate(&args[0], "builtins.openmathSchemaValidate")
    }
    "openmathSchemaExplain" if args.len() >= 1 => {
      xml_format_schema::xml_family_explain(&args[0], "builtins.openmathSchemaExplain")
    }
    "xmlSchemaNormalize" if args.len() >= 1 => {
      xml_format_schema::xml_family_normalize(&args[0], "builtins.xmlSchemaNormalize")
    }
    "xmlSchemaValidate" if args.len() >= 1 => {
      xml_format_schema::xml_family_validate(&args[0], "builtins.xmlSchemaValidate")
    }
    "xmlSchemaExplain" if args.len() >= 1 => {
      xml_format_schema::xml_family_explain(&args[0], "builtins.xmlSchemaExplain")
    }
    "colladaSchemaNormalize" if args.len() >= 1 => {
      xml_format_schema::xml_family_normalize(&args[0], "builtins.colladaSchemaNormalize")
    }
    "colladaSchemaValidate" if args.len() >= 1 => {
      xml_format_schema::xml_family_validate(&args[0], "builtins.colladaSchemaValidate")
    }
    "colladaSchemaExplain" if args.len() >= 1 => {
      xml_format_schema::xml_family_explain(&args[0], "builtins.colladaSchemaExplain")
    }
    "programSchemaNormalize" if args.len() >= 1 => {
      xml_format_schema::xml_family_normalize(&args[0], "builtins.programSchemaNormalize")
    }
    "programSchemaValidate" if args.len() >= 1 => {
      xml_format_schema::xml_family_validate(&args[0], "builtins.programSchemaValidate")
    }
    "programSchemaExplain" if args.len() >= 1 => {
      xml_format_schema::xml_family_explain(&args[0], "builtins.programSchemaExplain")
    }
    "hanimSchemaNormalize" if args.len() >= 1 => {
      xml_format_schema::xml_family_normalize(&args[0], "builtins.hanimSchemaNormalize")
    }
    "hanimSchemaValidate" if args.len() >= 1 => {
      xml_format_schema::xml_family_validate(&args[0], "builtins.hanimSchemaValidate")
    }
    // hanim explain takes (json, path); the clean-room XML baseline ignores path.
    "hanimSchemaExplain" if args.len() >= 2 => {
      xml_format_schema::xml_family_explain(&args[0], "builtins.hanimSchemaExplain")
    }
    "hanimSchemaValidateJointHierarchy" if args.len() >= 1 => {
      xml_format_schema::xml_family_validate(&args[0], "builtins.hanimSchemaValidateJointHierarchy")
    }
    // Office formats (Excel ↔ ODS) — XML containers, native XML-family
    // baseline; conversion normalizes XML AST only. Parse/emit route through
    // the existing generic `markup::xml_*` so the resulting AST shape matches
    // other markup builtins.
    "excelXmlToJson" if args.len() >= 1 => {
      xml_format_schema::xml_format_xml_to_json(&args[0], "builtins.excelXmlToJson")
    }
    "excelEmit" if args.len() >= 1 => {
      xml_format_schema::xml_format_emit(&args[0], "builtins.excelEmit").map(Value::String)
    }
    "excelToOds" if args.len() >= 1 => {
      xml_format_schema::xml_format_convert(&args[0], "builtins.excelToOds")
    }
    "odsToExcel" if args.len() >= 1 => {
      xml_format_schema::xml_format_convert(&args[0], "builtins.odsToExcel")
    }
    "excelFormulaToOpenFormula" if args.len() >= 1 => {
      xml_format_schema::xml_format_convert(&args[0], "builtins.excelFormulaToOpenFormula")
    }
    "openFormulaToExcel" if args.len() >= 1 => {
      xml_format_schema::xml_format_convert(&args[0], "builtins.openFormulaToExcel")
    }
    "excelStyleToOds" if args.len() >= 1 => {
      xml_format_schema::xml_format_convert(&args[0], "builtins.excelStyleToOds")
    }
    "odsStyleToExcel" if args.len() >= 1 => {
      xml_format_schema::xml_format_convert(&args[0], "builtins.odsStyleToExcel")
    }
    "excelAdvancedToOds" if args.len() >= 1 => {
      xml_format_schema::xml_format_convert(&args[0], "builtins.excelAdvancedToOds")
    }
    "odsAdvancedToExcel" if args.len() >= 1 => {
      xml_format_schema::xml_format_convert(&args[0], "builtins.odsAdvancedToExcel")
    }
    "isPath" if args.len() >= 1 => {
      let value = force_if_thunk(&args[0])?;
      Ok(Value::Bool(matches!(value.as_ref(), Value::Path(_))))
    }
    "pathExists" if args.len() >= 1 => {
      let path = resolve_value_path(&args[0], "builtins.pathExists")?;
      Ok(Value::Bool(path.exists()))
    }
    "pnixMount" if args.len() >= 1 => crate::package_mount::builtin_pnix_mount(args),
    "pnixUmount" if args.len() >= 1 => crate::package_mount::builtin_pnix_umount(args),
    "pnixMounts" => crate::package_mount::builtin_pnix_mounts(args),
    "pnixRun" if args.len() >= 1 => crate::package_mount::builtin_pnix_run(args),
    "readDir" if args.len() >= 1 => {
      let path = resolve_value_path(&args[0], "builtins.readDir")?;
      let entries = read_dir_entries_with_cache(&path)?;
      let mut out = BTreeMap::new();
      for (name, file_type) in entries {
        out.insert(name, Value::String(file_type));
      }
      Ok(Value::AttrSet(Arc::new(out)))
    }
    "baseNameOf" if args.len() >= 1 => {
      // Nix-compat (does NOT match Rust's Path::file_name): treat the
      // input as a raw string, strip a single trailing '/' (so `a/` → `a`
      // but `a//` → ``), then return the part after the last '/'. Special
      // sentinel paths (`""`, `.`, `..`) round-trip unchanged.
      let raw = match &args[0] {
        Value::String(s) => s.clone(),
        Value::StringContext { text, .. } => text.clone(),
        Value::Path(p) => path_display_string(p),
        _ => {
          return Err(anyhow!(
            "builtins.baseNameOf: expected string or path, got {}",
            type_name(&args[0])
          ));
        }
      };
      let trimmed = raw.strip_suffix('/').unwrap_or(&raw);
      let base = match trimmed.rfind('/') {
        Some(idx) => &trimmed[idx + 1..],
        None => trimmed,
      };
      // 2026-05-05: preserve string-context from a context-bearing
      // input (slice #49 family). If the input is `Value::StringContext`,
      // the result string carries the same provenance markers.
      // Path / plain-String inputs have no context to start with.
      Ok(Value::string_with_optional_context_ref(
        base.to_string(),
        args[0].string_context(),
      ))
    }
    "dirOf" if args.len() >= 1 => {
      // Nix-compat: dirOf of a string returns the directory portion as a
      // string (everything up to the last '/'). For Path values, return a
      // Path. Trailing '/' is preserved as part of the directory name only
      // when the input is exactly `/`.
      let (raw, is_path) = match &args[0] {
        Value::String(s) => (s.clone(), false),
        Value::StringContext { text, .. } => (text.clone(), false),
        Value::Path(p) => (path_display_string(p), true),
        _ => {
          return Err(anyhow!(
            "builtins.dirOf: expected string or path, got {}",
            type_name(&args[0])
          ));
        }
      };
      if raw == "/" {
        return Ok(if is_path {
          Value::Path(std::path::PathBuf::from("/"))
        } else {
          Value::String("/".to_string())
        });
      }
      let dir = match raw.rfind('/') {
        Some(0) => "/".to_string(),
        Some(idx) => raw[..idx].to_string(),
        None => ".".to_string(),
      };
      // 2026-05-05: preserve string-context (slice #49 family).
      Ok(if is_path {
        Value::Path(std::path::PathBuf::from(dir))
      } else {
        Value::string_with_optional_context_ref(dir, args[0].string_context())
      })
    }
    "toPath" if args.len() >= 1 => Ok(Value::Path(resolve_value_path(
      &args[0],
      "builtins.toPath",
    )?)),
    "storePath" if args.len() >= 1 => Ok(Value::Path(resolve_value_path(
      &args[0],
      "builtins.storePath",
    )?)),
    "match" if args.len() >= 2 => {
      let Some(pattern) = args[0].as_str() else {
        return Err(anyhow!("builtins.match: first arg must be string (regex)"));
      };
      let Some(s) = args[1].as_str() else {
        return Err(anyhow!("builtins.match: second arg must be string"));
      };
      let anchored_pattern = anchored_regex_pattern(pattern);
      let re = compile_regex_with_cache(&anchored_pattern, "builtins.match")?;
      // 2026-05-05 (slice #51): capture-group strings inherit the
      // haystack's context. Each capture is literally a substring of
      // the haystack, so its provenance is the haystack's provenance
      // — same shape as `substring` (slice #49). Pre-fix, capture
      // groups returned `Value::String(_)` with empty context, so
      // any user code matching against context-bearing strings (e.g.
      // `match "(.+)" "x${./p}"`) silently lost the path dependency
      // marker. Production-relevant: derivations that extract data
      // from a context-bearing path string would silently drop the
      // build-time dependency.
      let haystack_ctx = args[1].string_context();
      match re.captures(s) {
        Some(caps) => {
          let mut groups = Vec::with_capacity(caps.len().saturating_sub(1));
          for idx in 1..caps.len() {
            groups.push(match caps.get(idx) {
              Some(m) => {
                Value::string_with_optional_context_ref(m.as_str().to_string(), haystack_ctx)
              }
              None => Value::Null,
            });
          }
          Ok(Value::List(Arc::new(groups)))
        }
        None => Ok(Value::Null),
      }
    }
    "split" if args.len() >= 2 => {
      let Some(pattern) = args[0].as_str() else {
        return Err(anyhow!("builtins.split: first arg must be string (regex)"));
      };
      let Some(s) = args[1].as_str() else {
        return Err(anyhow!("builtins.split: second arg must be string"));
      };
      if pattern.is_empty() {
        return Err(anyhow!("builtins.split: regex pattern cannot be empty"));
      }
      let re = compile_regex_with_cache(pattern, "builtins.split")?;
      // 2026-05-05 (slice #51): every result element that comes
      // from the haystack — both the alternating literal segments
      // and the capture-group substrings — inherits the haystack's
      // string context. Pre-fix every result piece was a bare
      // `Value::String(_)` with empty context, so splitting a
      // context-bearing string silently dropped its provenance.
      // Empty separator-position strings (between adjacent matches
      // or at the trailing edge) also inherit the context: they
      // are still positional pieces of the haystack, even when
      // their text is empty.
      let haystack_ctx = args[1].string_context();
      let mk_str =
        |text: String| -> Value { Value::string_with_optional_context_ref(text, haystack_ctx) };
      let mut result = Vec::with_capacity(1);
      let mut last_end = 0;
      let capture_count = re.captures_len().saturating_sub(1);
      let has_capture_groups = capture_count > 0;
      for caps in re.captures_iter(s) {
        let mat = caps.get(0).expect("captures_iter always returns group 0");
        // Nix-compat: result must alternate string / capture-list / string ...
        // so any time two matches are adjacent (no text between them) we
        // emit an empty string between their capture-lists. This also
        // covers the leading position when the first match starts at 0.
        if mat.start() > last_end {
          result.push(mk_str(s[last_end..mat.start()].to_string()));
        } else {
          result.push(mk_str(String::new()));
        }
        let groups = if has_capture_groups {
          let mut groups = Vec::with_capacity(capture_count);
          for idx in 1..=capture_count {
            groups.push(match caps.get(idx) {
              Some(m) => mk_str(m.as_str().to_string()),
              None => Value::Null,
            });
          }
          groups
        } else {
          vec![]
        };
        result.push(Value::List(Arc::new(groups)));
        last_end = mat.end();
      }
      if last_end < s.len() {
        result.push(mk_str(s[last_end..].to_string()));
      } else {
        result.push(mk_str(String::new()));
      }
      Ok(Value::List(Arc::new(result)))
    }
    // Strip provenance markers — return the bare text. The result is
    // always `Value::String(_)` (no context).
    "unsafeDiscardStringContext" if args.len() >= 1 => {
      let value = force_if_thunk(&args[0])?;
      match value.as_ref() {
        Value::String(s) => Ok(Value::String(s.clone())),
        Value::StringContext { text, .. } => Ok(Value::String(text.clone())),
        _ => Err(anyhow!(
          "builtins.unsafeDiscardStringContext: expected string"
        )),
      }
    }
    "hasContext" if args.len() >= 1 => {
      let value = force_if_thunk(&args[0])?;
      match value.as_ref() {
        Value::String(_) => Ok(Value::Bool(false)),
        Value::StringContext { context, .. } => Ok(Value::Bool(!context.is_empty())),
        _ => Err(anyhow!("builtins.hasContext: expected string")),
      }
    }
    "getContext" if args.len() >= 1 => {
      // Return `{ <ctx-elem-id> = { path = true; }; ... }`. In Nix this
      // attrset describes per-element context flags (path / outputs /
      // allOutputs); pnix collapses everything to `path = true` since
      // we don't track derivation outputs (general-purpose provenance).
      let value = force_if_thunk(&args[0])?;
      let context: &BTreeSet<String> = match value.as_ref() {
        Value::String(_) => return Ok(Value::AttrSet(Arc::new(BTreeMap::new()))),
        Value::StringContext { context, .. } => context,
        _ => return Err(anyhow!("builtins.getContext: expected string")),
      };
      let mut out = BTreeMap::new();
      for elem in context {
        let mut entry = BTreeMap::new();
        entry.insert("path".to_string(), Value::Bool(true));
        out.insert(elem.clone(), Value::AttrSet(Arc::new(entry)));
      }
      Ok(Value::AttrSet(Arc::new(out)))
    }
    // Add a derivation-output marker. pnix has no derivations so this
    // is a fake-pass that just preserves any existing context. The
    // string still gains the marker via a synthetic key so round-trips
    // through `getContext` / `appendContext` see something.
    "addDrvOutputDependencies" if args.len() >= 1 => {
      let value = force_if_thunk(&args[0])?;
      let (text, mut context) = match value.as_ref() {
        Value::String(s) => (s.clone(), BTreeSet::<String>::new()),
        Value::StringContext { text, context } => (text.clone(), context.clone()),
        _ => {
          return Err(anyhow!(
            "builtins.addDrvOutputDependencies: expected string"
          ));
        }
      };
      // Use a stable synthetic marker so the result reflects the call
      // even though pnix has no actual derivation graph.
      context.insert(prefixed_string("!out!", &text));
      Ok(Value::string_with_context(text, context))
    }
    "unsafeDiscardOutputDependency" if args.len() >= 1 => {
      // Mirror Nix: strip the `!output!` marker but keep path-context.
      // Since pnix collapses both into the single context set, drop
      // entries that look like `!out!<name>`.
      let value = force_if_thunk(&args[0])?;
      let (text, context) = match value.as_ref() {
        Value::String(s) => return Ok(Value::String(s.clone())),
        Value::StringContext { text, context } => (text.clone(), context),
        _ => {
          return Err(anyhow!(
            "builtins.unsafeDiscardOutputDependency: expected string"
          ));
        }
      };
      let mut filtered = BTreeSet::new();
      for c in context {
        if !c.starts_with("!out!") && !c.starts_with("=") && !c.starts_with("!") {
          filtered.insert(c.clone());
        }
      }
      Ok(Value::string_with_context(text, filtered))
    }
    // 2026-05-05 (slice #69): the inverse of
    // `unsafeDiscardOutputDependency`. Adds an output-dependency
    // marker (`!out!<path>`) for every plain-path entry in the
    // string's context. After this builtin, downstream consumers
    // (the slice #56 `appendContext` / `getContext` round-trip,
    // derivation realization) see the context as "depends on
    // built outputs of these derivations" rather than "depends on
    // the derivation source files". Pre-fix this builtin was
    // missing — `.px` code that called it errored with
    // `attribute 'unsafeAddOutputDependency' not found` (a
    // misleading error pointing at attrset access rather than
    // unimplemented builtin). Same shape as slice #68's
    // `hashFile` addition. Plain-string input is a no-op (no
    // context entries to mark).
    "unsafeAddOutputDependency" if args.len() >= 1 => {
      let value = force_if_thunk(&args[0])?;
      let (text, mut context) = match value.as_ref() {
        Value::String(s) => return Ok(Value::String(s.clone())),
        Value::StringContext { text, context } => (text.clone(), context.clone()),
        _ => {
          return Err(anyhow!(
            "builtins.unsafeAddOutputDependency: expected string, got {}",
            type_name(value.as_ref())
          ));
        }
      };
      // Only mark plain-path entries. Already-prefixed entries
      // (`!out!`, `!name!`, `=`) preserve their existing role.
      let mut new_entries = Vec::with_capacity(context.len());
      for entry in &context {
        if !entry.starts_with('!') && !entry.starts_with('=') {
          new_entries.push(prefixed_string("!out!", entry));
        }
      }
      for entry in new_entries {
        context.insert(entry);
      }
      Ok(Value::string_with_context(text, context))
    }
    // 2026-05-05 (slice #69): adds an output-name marker
    // (`!<name>!<path>`) for every plain-path entry in the
    // string's context. Used by Nixpkgs `lib` to construct
    // multi-output derivation references (e.g., naming a
    // specific output like "out", "dev", or "lib"). Pre-fix
    // missing — same misleading-error shape as
    // unsafeAddOutputDependency. The `name` argument must be a
    // string (matches real Nix's API). Plain-string input is a
    // no-op (no context entries to mark).
    "unsafeAddOutputName" if args.len() >= 2 => {
      let name_value = force_if_thunk(&args[0])?;
      let name = name_value.as_ref().as_str().ok_or_else(|| {
        anyhow!(
          "builtins.unsafeAddOutputName: first arg (name) must be string, got {}",
          type_name(name_value.as_ref())
        )
      })?;
      let value = force_if_thunk(&args[1])?;
      let (text, mut context) = match value.as_ref() {
        Value::String(s) => return Ok(Value::String(s.clone())),
        Value::StringContext { text, context } => (text.clone(), context.clone()),
        _ => {
          return Err(anyhow!(
            "builtins.unsafeAddOutputName: second arg must be string, got {}",
            type_name(value.as_ref())
          ));
        }
      };
      let mut new_entries = Vec::with_capacity(context.len());
      for entry in &context {
        if !entry.starts_with('!') && !entry.starts_with('=') {
          new_entries.push(output_name_context_marker(name, entry));
        }
      }
      for entry in new_entries {
        context.insert(entry);
      }
      Ok(Value::string_with_context(text, context))
    }
    // `appendContext s ctx` overlays additional context entries from an
    // attrset matching the `getContext` shape.
    // 2026-05-05 (slice #56): each value in the attrset must
    // match the `getContext` result shape — an attrset with
    // optional fields `path: bool`, `outputs: [string]`,
    // `allOutputs: bool`. Pre-fix the implementation just
    // iterated `extra.keys()` and inserted each key into the
    // context set, ignoring the value entirely. So
    // `appendContext "x" { "/a" = "wrong-shape"; }` and
    // `appendContext "x" { "/a" = { outputs = 42; }; }` and
    // `appendContext "x" { "/a" = null; }` all silently
    // succeeded — production code that constructed a malformed
    // shape (often via a `mapAttrs`-style transform that produced
    // wrong types) would silently dispatch into the context set
    // without any indication that the shape was invalid.
    // Real Nix validates the shape and errors on mismatch.
    // Fix: validate each value is an attrset, and each known
    // field has the correct type.
    "appendContext" if args.len() >= 2 => {
      let string_value = force_if_thunk(&args[0])?;
      let (text, mut context) = match string_value.as_ref() {
        Value::String(s) => (s.clone(), BTreeSet::<String>::new()),
        Value::StringContext { text, context } => (text.clone(), context.clone()),
        _ => return Err(anyhow!("builtins.appendContext: first arg must be string")),
      };
      let extra_value = force_if_thunk(&args[1])?;
      let Value::AttrSet(extra) = extra_value.as_ref() else {
        return Err(anyhow!(
          "builtins.appendContext: second arg must be attrset"
        ));
      };
      for (k, v) in extra.iter() {
        let spec_value = force_if_thunk(v)?;
        let Value::AttrSet(spec) = spec_value.as_ref() else {
          return Err(anyhow!(
            "builtins.appendContext: context value for '{}' must be an attrset \
             matching the getContext result shape \
             ({{ path = bool; outputs = [string]; allOutputs = bool; }}), got {}",
            k,
            type_name(spec_value.as_ref())
          ));
        };
        // Validate known field types when present. Unknown fields
        // are allowed (forward-compat / future-Nix shape additions),
        // but if a field IS present with the documented name we
        // require its type to match the canonical shape.
        if let Some(path_v) = spec.get("path") {
          let path_value = force_if_thunk(path_v)?;
          if !matches!(path_value.as_ref(), Value::Bool(_)) {
            return Err(anyhow!(
              "builtins.appendContext: context value for '{}' field 'path' \
               must be bool, got {}",
              k,
              type_name(path_value.as_ref())
            ));
          }
        }
        if let Some(all_v) = spec.get("allOutputs") {
          let all_value = force_if_thunk(all_v)?;
          if !matches!(all_value.as_ref(), Value::Bool(_)) {
            return Err(anyhow!(
              "builtins.appendContext: context value for '{}' field 'allOutputs' \
               must be bool, got {}",
              k,
              type_name(all_value.as_ref())
            ));
          }
        }
        if let Some(outputs_v) = spec.get("outputs") {
          let outputs_value = force_if_thunk(outputs_v)?;
          let Value::List(outs) = outputs_value.as_ref() else {
            return Err(anyhow!(
              "builtins.appendContext: context value for '{}' field 'outputs' \
               must be list of strings, got {}",
              k,
              type_name(outputs_value.as_ref())
            ));
          };
          for (idx, out) in outs.iter().enumerate() {
            let out_value = force_if_thunk(out)?;
            if out_value.as_ref().as_str().is_none() {
              return Err(anyhow!(
                "builtins.appendContext: context value for '{}' field 'outputs' \
                 element at index {} must be string, got {}",
                k,
                idx,
                type_name(out_value.as_ref())
              ));
            }
          }
        }
        context.insert(k.clone());
      }
      Ok(Value::string_with_context(text, context))
    }
    "unsafeGetAttrPos" if args.len() >= 2 => {
      // Read the *raw* attrset value (without forcing) so we can look at
      // the binding-site `attr_pos` carried by `Value::Thunk`. The
      // builtin is registered as `lazy_in_elements`, so deep_force did
      // not strip the thunk wrapper.
      let Some(attr_name) = args[0].as_str() else {
        return Err(anyhow!(
          "builtins.unsafeGetAttrPos: expected string attribute name"
        ));
      };
      let Value::AttrSet(map) = &args[1] else {
        return Err(anyhow!("builtins.unsafeGetAttrPos: expected attrset"));
      };
      let Some(slot) = map.get(attr_name) else {
        return Ok(Value::Null);
      };
      // 2026-05-05 (slice #53): when the slot has no `attr_pos`
      // (already-forced value, builtin-injected binding, or any
      // value that wasn't a `Value::Thunk` carrying its source
      // position), real Nix returns `null`. Pre-fix pnix returned
      // a fake `{ file = "<unknown>"; line = 0; column = 0; }`
      // attrset, which is silent fabrication: any tool that
      // consumes `unsafeGetAttrPos` for source-location diagnostics
      // (linters, error formatters, IDE jump-to-definition) would
      // silently accept the fake position as real. Returning `null`
      // matches the Nix-canonical contract and lets consumers
      // distinguish "no position info available" from "position
      // available". Forced values produced by `with` chains, by
      // builtin construction, or by arithmetic results have no
      // source position — that's the legitimate `null` case.
      let (file, line, column) = match slot {
        Value::Thunk {
          attr_pos: Some(p), ..
        } => ((*p.file).clone(), p.line as i64, p.column as i64),
        _ => return Ok(Value::Null),
      };
      let mut pos = BTreeMap::new();
      pos.insert("file".to_string(), Value::String(file));
      pos.insert("line".to_string(), Value::Int(line));
      pos.insert("column".to_string(), Value::Int(column));
      Ok(Value::AttrSet(Arc::new(pos)))
    }
    // 2026-05-05 (slice #63): Nix-compat: `zipAttrsWith f xs`
    // is lazy in the resulting attrset values. Each output
    // value is a thunk for `f key valueList`; only forced
    // when the field is accessed. Pre-fix the impl applied
    // the function eagerly via two `apply_value` calls,
    // so any throw in the function body fired at construction
    // — `length (attrNames (zipAttrsWith throw [...]))`
    // errored instead of returning the number of unique keys,
    // and `r ? a` (which only checks key presence) errored
    // instead of returning true. Mirrors the slice #62
    // `map` / `genList` and predates `mapAttrs` laziness
    // contracts.
    "zipAttrsWith" if args.len() >= 2 => {
      let func = args[0].clone();
      let Value::List(items) = &args[1] else {
        return Err(anyhow!(
          "builtins.zipAttrsWith: second arg must be list, got {}",
          type_name(&args[1])
        ));
      };

      let mut keys = BTreeSet::new();
      let mut attrsets = Vec::with_capacity(items.len());
      for item in items.iter() {
        let Value::AttrSet(attrs) = item else {
          return Err(anyhow!(
            "builtins.zipAttrsWith: list elements must be attrsets, got {}",
            type_name(item)
          ));
        };
        for key in attrs.keys() {
          keys.insert(key.clone());
        }
        attrsets.push(attrs.clone());
      }

      let mut result = BTreeMap::new();
      for key in keys {
        let mut values = Vec::with_capacity(attrsets.len());
        for attrs in &attrsets {
          if let Some(value) = attrs.get(&key) {
            values.push(value.clone());
          }
        }
        // Defer the two-stage application `f key valueList`
        // — produce a thunk that computes `f key valueList`
        // only when forced.
        let deferred = deferred_apply2(
          func.clone(),
          Value::String(key.clone()),
          Value::List(Arc::new(values)),
        );
        result.insert(key, deferred);
      }
      Ok(Value::AttrSet(Arc::new(result)))
    }
    "compareVersions" if args.len() >= 2 => {
      let Some(v1) = args[0].as_str() else {
        return Err(anyhow!("builtins.compareVersions: expected two strings"));
      };
      let Some(v2) = args[1].as_str() else {
        return Err(anyhow!("builtins.compareVersions: expected two strings"));
      };
      Ok(Value::Int(compare_versions(v1, v2)))
    }
    // 2026-05-05 (slice #61): result strings inherit the input
    // string's context. Each element of the split result is a
    // substring of the input version string — same shape as
    // `match` capture groups (slice #51) and `substring`
    // (slice #49). Pre-fix the result list contained plain
    // `Value::String(_)` with empty context, so any author
    // splitting a derivation-reference version string silently
    // dropped the dependency markers at the split boundary.
    "splitVersion" if args.len() >= 1 => {
      let Some(version) = args[0].as_str() else {
        return Err(anyhow!("builtins.splitVersion: expected string"));
      };
      let ctx = args[0].string_context();
      let parts = split_version_components(version);
      let mut values = Vec::with_capacity(parts.len());
      for part in parts {
        values.push(Value::string_with_optional_context_ref(
          part.to_string(),
          ctx,
        ));
      }
      Ok(Value::List(Arc::new(values)))
    }
    // 2026-05-05 (slice #61): both `name` and `version` values
    // are substrings of the input — they inherit the input's
    // context. Same family as `splitVersion` and `match` /
    // `split` capture groups. Pre-fix `parseDrvName "hello-
    // 1.0${./p}"` returned an attrset whose `.name` and
    // `.version` were plain `Value::String(_)` with empty
    // context, silently dropping the dependency marker at the
    // parse boundary.
    "parseDrvName" if args.len() >= 1 => {
      let Some(value) = args[0].as_str() else {
        return Err(anyhow!("builtins.parseDrvName: expected string"));
      };
      let ctx = args[0].string_context();
      let (name, version) = parse_drv_name(value);
      let mut result = BTreeMap::new();
      result.insert(
        "name".to_string(),
        Value::string_with_optional_context_ref(name, ctx),
      );
      result.insert(
        "version".to_string(),
        Value::string_with_optional_context_ref(version, ctx),
      );
      Ok(Value::AttrSet(Arc::new(result)))
    }
    "abort" if args.len() >= 1 => {
      // 2026-05-05: previously fell back to `format!("{}", args[0])`
      // when the message wasn't a string, so `abort 42` silently
      // produced `evaluation aborted: 42` and `abort [ 1 ]` produced
      // `evaluation aborted: [1]`. Real Nix rejects non-string
      // arguments to `abort` with "value is a <type> while a string
      // was expected", same as `throw` (already pinned at the arm
      // below). The `evaluation aborted: ` prefix is load-bearing
      // — slice #35's `tryEval` abort-propagation marker keys off
      // that exact prefix — so we keep the prefix and tighten the
      // type contract by erroring before the prefix is emitted.
      let Some(message) = args[0].as_str() else {
        return Err(anyhow!(
          "builtins.abort: argument must be string, got {}",
          args[0]
        ));
      };
      Err(anyhow!("evaluation aborted: {}", message))
    }
    "throw" if args.len() >= 1 => {
      let Some(message) = args[0].as_str() else {
        return Err(anyhow!(
          "builtins.throw: argument must be string, got {}",
          args[0]
        ));
      };
      Err(anyhow!(message.to_string()))
    }
    "import" if args.len() >= 1 => {
      // Equivalent to `import <path>`. Mirrors `PnixExpr::Import`.
      let path = resolve_value_path(&args[0], "builtins.import")?;
      eval_file_at_path(&path)
    }
    "scopedImport" if args.len() >= 2 => {
      // `scopedImport attrs fn` loads `fn` (a path) with the bindings
      // in `attrs` injected into its top-level lexical scope. Common
      // use: bootstrapping override patterns
      // (`overrides.import = fn: scopedImport overrides fn;`).
      let Value::AttrSet(attrs) = &args[0] else {
        return Err(anyhow!(
          "builtins.scopedImport: first arg must be attrset, got {}",
          type_name(&args[0])
        ));
      };
      let path = resolve_value_path(&args[1], "builtins.scopedImport")?;
      let file_guard = push_import_file_guard_and_record(&path)?;
      let expr = load_baked_expr_at_path(&path, file_guard.canon())
        .map_err(|e| anyhow!("scopedImport: load {}: {}", path.display(), e))?;
      // Inject `attrs` as the scope: bind each attr name → its (lazy) value.
      let mut env = Env::with_capacity(attrs.len());
      for (k, v) in attrs.iter() {
        env.bind(k.clone(), v.clone());
      }
      // Resolve relative imports inside the loaded file against its dir.
      let _guard = path
        .parent()
        .map(|p| ImportBaseGuard::push(p.to_path_buf()));
      eval_arc(expr.clone(), &env)
    }
    // BFS fixpoint over an attrset graph.
    //   `genericClosure { startSet = [...]; operator = fn; }` — start
    //   with `startSet` items, apply `operator` to each item not seen
    //   before (deduped by `.key`), enqueue results, repeat until no
    //   new items. Heavy use in nixpkgs (`closePropagation`,
    //   `closureInfo`, etc.).
    "genericClosure" if args.len() >= 1 => {
      let Value::AttrSet(arg_map) = &args[0] else {
        return Err(anyhow!(
          "builtins.genericClosure: expected attrset {{ startSet, operator }}"
        ));
      };
      let start_set = arg_map.get("startSet").cloned().ok_or_else(|| {
        anyhow!("builtins.genericClosure: argument missing required attribute 'startSet'")
      })?;
      let operator = arg_map.get("operator").cloned().ok_or_else(|| {
        anyhow!("builtins.genericClosure: argument missing required attribute 'operator'")
      })?;
      let start_list = match force_value(start_set)? {
        Value::List(l) => l,
        other => {
          return Err(anyhow!(
            "builtins.genericClosure: 'startSet' must be a list, got {}",
            type_name(&other)
          ));
        }
      };
      let mut seen_keys: FxHashSet<String> = FxHashSet::default();
      let mut output: Vec<Value> = Vec::with_capacity(start_list.len());
      let mut work: Vec<Value> = Arc::unwrap_or_clone(start_list);
      while let Some(item) = work.pop() {
        let item = force_value(item)?;
        let Value::AttrSet(item_map) = &item else {
          return Err(anyhow!(
            "builtins.genericClosure: item must be attrset with .key, got {}",
            type_name(&item)
          ));
        };
        let key_val = item_map
          .get("key")
          .cloned()
          .ok_or_else(|| anyhow!("builtins.genericClosure: item missing 'key' attribute"))?;
        let key_val = force_value(key_val)?;
        let key_sig = generic_closure_key_signature(&key_val);
        if !seen_keys.insert(key_sig) {
          continue;
        }
        output.push(item.clone());
        // Apply operator and queue any further items it returns.
        let next = apply_value(operator.clone(), item)?;
        let next_list = match next {
          Value::List(l) => l,
          other => {
            return Err(anyhow!(
              "builtins.genericClosure: operator must return list, got {}",
              type_name(&other)
            ));
          }
        };
        // Push in reverse so we visit in stack-natural order; the test's
        // `sort` on output makes ordering irrelevant for correctness.
        for v in Arc::unwrap_or_clone(next_list).into_iter().rev() {
          work.push(v);
        }
      }
      Ok(Value::List(Arc::new(output)))
    }
    // 2026-05-05 (slice #53): error messages now name which arg
    // failed and what type was given. Pre-fix the catch-all "both
    // args must be integer" left .px authors guessing whether the
    // first or second arg was the offender (and whether it was a
    // float, string, list, etc.). Same diagnostic family as the
    // typed-list-builtin tightenings (slices #32 / #33 / #38 /
    // #51 / #52). bitwise ops require strict `Int` — `Float` is
    // rejected, mirroring the slice #45 boundary where `floor` /
    // `ceil` saturate-to-int is the only sanctioned float→int
    // path; bitwise on a float would silently truncate via the
    // `Float -> Int` cast, which is exactly the silent-precision-
    // loss shape the audit closes.
    "bitAnd" if args.len() >= 2 => match (&args[0], &args[1]) {
      (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a & b)),
      (Value::Int(_), other) => Err(anyhow!(
        "builtins.bitAnd: second arg must be integer, got {}",
        type_name(other)
      )),
      (other, _) => Err(anyhow!(
        "builtins.bitAnd: first arg must be integer, got {}",
        type_name(other)
      )),
    },
    "bitOr" if args.len() >= 2 => match (&args[0], &args[1]) {
      (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a | b)),
      (Value::Int(_), other) => Err(anyhow!(
        "builtins.bitOr: second arg must be integer, got {}",
        type_name(other)
      )),
      (other, _) => Err(anyhow!(
        "builtins.bitOr: first arg must be integer, got {}",
        type_name(other)
      )),
    },
    "bitXor" if args.len() >= 2 => match (&args[0], &args[1]) {
      (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a ^ b)),
      (Value::Int(_), other) => Err(anyhow!(
        "builtins.bitXor: second arg must be integer, got {}",
        type_name(other)
      )),
      (other, _) => Err(anyhow!(
        "builtins.bitXor: first arg must be integer, got {}",
        type_name(other)
      )),
    },
    // pnix has no nix-store, so `placeholder` returns a stable opaque
    // string carrying the output name as both text and string context —
    // enough for `${placeholder "out"}/lib/...` patterns to round-trip.
    "placeholder" if args.len() >= 1 => {
      let Some(name) = args[0].as_str() else {
        return Err(anyhow!("builtins.placeholder: expected string"));
      };
      let text = prefixed_string("/pnix-placeholder/", name);
      let mut ctx = BTreeSet::new();
      ctx.insert(prefixed_string("=placeholder!", name));
      Ok(Value::string_with_context(text, ctx))
    }
    // `derivationStrict` is the low-level derivation primop. pnix is
    // not a build system — fake-pass by returning the input attrset
    // augmented with `outPath` and `drvPath` placeholders so common
    // nixpkgs idioms (`(derivation { ... }).outPath`) resolve.
    "derivationStrict" if args.len() >= 1 => {
      let mut map = match &args[0] {
        Value::AttrSet(m) => m.clone(),
        _ => {
          return Err(anyhow!(
            "builtins.derivationStrict: expected attrset, got {}",
            type_name(&args[0])
          ));
        }
      };
      let name = map
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("unnamed");
      let placeholder = prefixed_string("/pnix-placeholder/derivation/", name);
      let mut ctx = BTreeSet::new();
      ctx.insert(prefixed_string("!out!", name));
      let path_value = Value::string_with_context(placeholder.clone(), ctx);
      Arc::make_mut(&mut map)
        .entry("outPath".to_string())
        .or_insert(path_value.clone());
      Arc::make_mut(&mut map)
        .entry("drvPath".to_string())
        .or_insert(path_value);
      Arc::make_mut(&mut map)
        .entry("type".to_string())
        .or_insert(Value::String("derivation".to_string()));
      Ok(Value::AttrSet(map))
    }
    // 2026-05-06 (slice #80): closes the `builtins.derivation`
    // missing-builtin gap. Real Nix has both `derivation`
    // (high-level — returns a derivation value with standard
    // fields wired in) and `derivationStrict` (low-level —
    // returns the strict attrs map). pnix had `derivationStrict`
    // since long ago, but `derivation` was missing — every
    // `.px` author writing `builtins.derivation { ... }` (the
    // common idiom that every `mkDerivation` / `stdenv.mkDerivation`
    // call eventually compiles to) hit `attribute 'derivation'
    // not found`. Pre-fix workaround was to use `derivationStrict`
    // directly, but that's not what nixpkgs-style code does.
    //
    // Implementation: thin wrapper around `derivationStrict`.
    // Real Nix's `derivation` does:
    //   1. Call `derivationStrict attrs` to get the strict shape.
    //   2. Spread `attrs` into the result so user-provided fields
    //      pass through.
    //   3. Ensure `outPath` / `drvPath` / `type = "derivation"`
    //      are set (which `derivationStrict` already does).
    // pnix matches: clone the input attrs, then layer the
    // `derivationStrict`-computed fields on top via `or_insert`
    // (so user-provided overrides win). The result is a single
    // attrset that round-trips through both call sites uniformly.
    "derivation" if args.len() >= 1 => {
      let mut map = match &args[0] {
        Value::AttrSet(m) => m.clone(),
        _ => {
          return Err(anyhow!(
            "builtins.derivation: expected attrset, got {}",
            type_name(&args[0])
          ));
        }
      };
      let name = map
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("unnamed");
      let placeholder = prefixed_string("/pnix-placeholder/derivation/", name);
      let mut ctx = BTreeSet::new();
      ctx.insert(prefixed_string("!out!", name));
      let path_value = Value::string_with_context(placeholder.clone(), ctx);
      Arc::make_mut(&mut map)
        .entry("outPath".to_string())
        .or_insert(path_value.clone());
      Arc::make_mut(&mut map)
        .entry("drvPath".to_string())
        .or_insert(path_value);
      Arc::make_mut(&mut map)
        .entry("type".to_string())
        .or_insert(Value::String("derivation".to_string()));
      Ok(Value::AttrSet(map))
    }
    "break" if args.len() >= 1 => {
      // `builtins.break v` is normally a debugger entry-point. We pass the
      // value through (no debugger in pnix-eval) so callers can use it as
      // an identity hook.
      Ok(args[0].clone())
    }
    "ontologyLift" if args.len() >= 2 => {
      // Bridge string-context provenance into ontology facts.
      //
      // - `ontologyLift { ...attrs } "context"` — keep existing
      //   semantics: the attrs become the lifted fact, with
      //   `ontology-context` / `ontology-status` annotated.
      // - `ontologyLift "text-with-context" "context"` — auto-build a
      //   fact attrset `{ value = "text"; }` and seed
      //   `provenance_refs` from the string's `string_context`. This
      //   lets `${./file}` interpolations carry through to ontology
      //   evidence without explicit plumbing.
      // - In both cases, if the input attrs contain string-context
      //   values they are NOT walked (would require deep traversal);
      //   only the top-level string is bridged.
      let mut r = match &args[0] {
        Value::AttrSet(m) => (**m).clone(),
        Value::String(s) => {
          let mut m = BTreeMap::new();
          m.insert("value".to_string(), Value::String(s.clone()));
          m
        }
        Value::StringContext { text, context } => {
          let mut m = BTreeMap::new();
          m.insert("value".to_string(), Value::String(text.clone()));
          // Seed provenance_refs from the string's context so the
          // ontology engine sees the path / derivation markers.
          m.insert(
            "provenance_refs".to_string(),
            Value::List(Arc::new(context_to_string_list(context))),
          );
          m
        }
        _ => BTreeMap::new(),
      };
      let c = match &args[1] {
        Value::String(s) => s.clone(),
        Value::StringContext { text, .. } => text.clone(),
        _ => "unknown".into(),
      };
      r.insert("ontology-context".into(), Value::String(c));
      r.insert("ontology-status".into(), Value::String("Candidate".into()));
      Ok(Value::AttrSet(Arc::new(r)))
    }
    // Direct string-context → provenance_refs bridge. Returns the list
    // of context elements so callers can stitch them into custom
    // ontology shapes without round-tripping through `ontologyLift`.
    "stringContextToProvenance" if args.len() >= 1 => {
      let entries: Vec<Value> = match &args[0] {
        Value::String(_) => Vec::new(),
        Value::StringContext { context, .. } => context_to_string_list(context),
        _ => {
          return Err(anyhow!(
            "builtins.stringContextToProvenance: expected string"
          ));
        }
      };
      Ok(Value::List(Arc::new(entries)))
    }
    // batch 265 (2026-04-18): G2.3 ontologyEvaluate — constant stub 제거.
    // 입력 interpretation 의 shape 에 따라 6 축 evaluation 을 deterministic
    // 하게 계산. Pure — 실제 domain-specific score 는 downstream 이 policy
    // 를 가지고 재계산할 수 있지만, 여기서 canonical axis shape 은 보장.
    //
    // 입력: args[0] = policy (attrset, 현재 unused), args[1] = interpretation
    //       (attrset: facts / proof_refs / provenance_refs / status / losses)
    // 출력: interpretation 원본 + 6 개 evaluation-* 필드 덮어쓰기
    "ontologyEvaluate" if args.len() >= 2 => {
      let interp = match &args[1] {
        Value::AttrSet(m) => (**m).clone(),
        _ => BTreeMap::new(),
      };
      let (coh, cov, loss, cost, repl, safe) = compute_evaluation_axes(&interp);
      let mut r = interp;
      r.insert("coherence".into(), Value::Float(coh));
      r.insert("coverage".into(), Value::Float(cov));
      r.insert("loss".into(), Value::Float(loss));
      r.insert("cost".into(), Value::Float(cost));
      r.insert("replayability".into(), Value::Float(repl));
      r.insert("safety".into(), Value::Float(safe));
      r.insert("evaluation-coherence".into(), Value::Float(coh));
      r.insert("evaluation-coverage".into(), Value::Float(cov));
      r.insert("evaluation-loss".into(), Value::Float(loss));
      r.insert("evaluation-cost".into(), Value::Float(cost));
      r.insert("evaluation-replayability".into(), Value::Float(repl));
      r.insert("evaluation-safety".into(), Value::Float(safe));
      // deterministic 요약 score (tie-break 직전 단계).
      let score = (coh + cov + repl + safe) - (loss + cost);
      r.insert("score".into(), Value::Float(score));
      r.insert("evaluation-score".into(), Value::Float(score));
      Ok(Value::AttrSet(Arc::new(r)))
    }
    // OWNER-LAW (2026-05-11): pnix-eval `ontologyPromote` is the legacy
    // 2-arity entry point. It must (a) respect the judgement action
    // (Accept / Reject / Hold / Contradict) — not blindly emit "Accepted",
    // and (b) treat its evidence lane as `InternalOwnerLaw` because the
    // 2-arity caller did not declare an external lane. External callers
    // (ExternalWebSearch / ExternalApi / TransducerOutput / HumanProvidedProse
    // / ToolExecutionResult / PeerEvidence) must use `ontologyPromoteWithLane`
    // (3-arity) so that `Accept` is downgraded to `Candidate`.
    "ontologyPromote" if args.len() >= 2 => {
      let r = ontology_promote_eval(&args[1], "InternalOwnerLaw");
      Ok(Value::AttrSet(Arc::new(r)))
    }
    "ontologyPromoteWithLane" if args.len() >= 3 => {
      // args[0]=policy, args[1]=lane (string), args[2]=judgement (attrset)
      let lane = match &args[1] {
        Value::String(s) => s.clone(),
        _ => "Unknown".to_string(),
      };
      let r = ontology_promote_eval(&args[2], &lane);
      Ok(Value::AttrSet(Arc::new(r)))
    }
    // batch 265 (2026-04-18): G2.4 ontologySelect — ACP (Accept Criterion
    // Policy) 완성. 입력 args[0]=policy (현재 unused), args[1]=interpretations.
    // 반환은 deterministic tie-break 순서로 best 선택. 빈 리스트 → Null.
    //
    // tie-break 순서 (convergence.md ontology 정본): score → safety →
    // replayability → lower loss → lower cost → lexical interpretation-id.
    "ontologySelect" if args.len() >= 2 => {
      let candidates = match &args[1] {
        Value::List(items) => items,
        other => return Ok(other.clone()),
      };
      if candidates.is_empty() {
        return Ok(Value::Null);
      }
      let mut keyed: Vec<(EvalSelectKey, usize)> = Vec::with_capacity(candidates.len());
      for (idx, value) in candidates.iter().enumerate() {
        keyed.push((eval_select_key(value), idx));
      }
      // sort_by descending — higher score / safety / replay, lower loss /
      // cost / lex-id. Rust sort 는 ascending 이므로 key compare 를 reverse.
      keyed.sort_by(|a, b| b.0.cmp(&a.0));
      let best_idx = keyed[0].1;
      Ok(candidates[best_idx].clone())
    }
    // batch 264 (2026-04-18): G2.1 ontologyQuery — pure query descriptor.
    // 실제 store lookup 은 doghouse 쪽 downstream 에서 수행. pnix-eval 은
    // query 를 "query-kind: ontology-query" 로 marker 를 붙여 canonical
    // request envelope 로 감싸기만 한다. 호출 형태:
    //   builtins.ontologyQuery { context = "..."; subject = "..."; predicate = "..."; }
    // 반환: { query-kind = "ontology-query"; context = ...; subject = ...; predicate = ...; }
    "ontologyQuery" if args.len() >= 1 => {
      let mut envelope = match args.last() {
        Some(Value::AttrSet(m)) => (**m).clone(),
        _ => BTreeMap::new(),
      };
      envelope
        .entry("query-kind".to_string())
        .or_insert_with(|| Value::String("ontology-query".to_string()));
      Ok(Value::AttrSet(Arc::new(envelope)))
    }
    // batch 264 (2026-04-18): G2.2 ontologyEmit — ExpressionProjectionRecord
    // surface forms 4 개 (openmath / mathml-content / canonical-text /
    // freecat-geometry) 를 canonical 로 채운다. 입력에 누락된 form 은
    // Value::Null 로 채워서 downstream 이 일관된 shape 을 본다. Pure —
    // 실제 projection 은 downstream (expression projection pipeline) 이 담당.
    "ontologyEmit" if args.len() >= 1 => {
      let mut record = match args.last() {
        Some(Value::AttrSet(m)) => (**m).clone(),
        _ => BTreeMap::new(),
      };
      record
        .entry("projection-family".to_string())
        .or_insert_with(|| Value::String("expmath".to_string()));
      // 4 canonical surface form key — 누락 시 null.
      let mut surface_forms = match record.remove("surface-forms") {
        Some(Value::AttrSet(m)) => Arc::unwrap_or_clone(m),
        _ => BTreeMap::new(),
      };
      for key in [
        "openmath",
        "mathml-content",
        "canonical-text",
        "freecat-geometry",
      ] {
        surface_forms.entry(key.to_string()).or_insert(Value::Null);
      }
      record.insert(
        "surface-forms".to_string(),
        Value::AttrSet(Arc::new(surface_forms)),
      );
      record
        .entry("emit-kind".to_string())
        .or_insert_with(|| Value::String("expression-projection".to_string()));
      Ok(Value::AttrSet(Arc::new(record)))
    }
    // batch 263 (2026-04-18): 추가 builtin 구현.
    "substring" if args.len() >= 3 => {
      // Nix-correct:
      //   - start < 0 errors with "negative start position"
      //   - len  < 0 means "until end of string"
      //   - start > len(s) returns ""
      //   - start + len running past end is clamped to end
      //   - indices are byte-based (Nix C-string semantics); we
      //     respect UTF-8 by snapping the end down to the nearest
      //     char boundary so we never emit invalid UTF-8.
      let start = expect_i64(&args[0], "builtins.substring")?;
      let len = expect_i64(&args[1], "builtins.substring")?;
      let s = args[2].as_str().ok_or_else(|| {
        anyhow!(
          "builtins.substring: third arg must be string, got {}",
          type_name(&args[2])
        )
      })?;
      if start < 0 {
        return Err(anyhow!(
          "builtins.substring: negative start position {} not allowed",
          start
        ));
      }
      let start_b = start as usize;
      if start_b >= s.len() {
        return Ok(Value::String(String::new()));
      }
      // Byte-clamped end. `len < 0` → to end. Otherwise clamp to
      // string length.
      let raw_end = if len < 0 {
        s.len()
      } else {
        start_b.saturating_add(len as usize).min(s.len())
      };
      // 2026-05-19: snap start FIRST, then end. The pre-fix order
      // (end first, then start) could cross-snap: if a caller indexed
      // mid-byte into a multi-byte char and the requested length
      // landed inside the SAME char, the end snap would push end_b
      // down past the start snap-up target, producing
      // `start_b > end_b` and panicking on `&s[start_b..end_b]`. This
      // surfaced when Korean (or any multi-byte UTF-8) substrate code
      // walked utterances byte-by-byte and called substring with
      // small `len`. New rule: start-up snap first, then end-down
      // snap bounded by `>= start_b`, then explicit `start_b > end_b`
      // → empty fallback (real Nix semantics: when the requested
      // window lies entirely inside a single char it can only
      // honestly return empty).
      let mut start_b = start_b;
      while start_b < s.len() && !s.is_char_boundary(start_b) {
        start_b += 1;
      }
      // After snap-up start_b may have moved past the original
      // raw_end. Cap raw_end up to at least start_b before snapping
      // end down, then verify the post-snap window is non-empty.
      let mut end_b = if raw_end > start_b { raw_end } else { start_b };
      while end_b > start_b && !s.is_char_boundary(end_b) {
        end_b -= 1;
      }
      if start_b > end_b {
        // Defensive: should be impossible after the above, but if it
        // somehow happens, return empty rather than panic.
        return Ok(Value::String(String::new()));
      }
      // 2026-05-05: preserve string context from the input. Real
      // Nix's `substring` propagates the context unchanged (the
      // slice is "from" the original context-bearing string, so
      // it inherits the same provenance markers). pnix used to
      // produce a bare `Value::String`, silently losing the
      // context — same family as `concatStringsSep`'s drop.
      let text = s[start_b..end_b].to_string();
      Ok(Value::string_with_optional_context_ref(
        text,
        args[2].string_context(),
      ))
    }
    "stringLength" if args.len() >= 1 => {
      // Nix-correct: byte length, not char count. nixpkgs and the
      // upstream test corpus both rely on byte semantics —
      // `stringLength "é"` is 2 (UTF-8 bytes), not 1 (Unicode
      // codepoint). Previous impl used `chars().count()`.
      let s = args[0].as_str().ok_or_else(|| {
        anyhow!(
          "builtins.stringLength: expected string, got {}",
          type_name(&args[0])
        )
      })?;
      Ok(Value::Int(s.len() as i64))
    }
    "koreanFinalConsonantKind" if args.len() >= 1 => {
      // v0.16-BO: Korean Hangul jongseong (final consonant)
      // classifier. Looks at the LAST Unicode character of the
      // input string and dispatches to the Korean case marker
      // lens's 3-way allomorph rule:
      //
      //   "none"        — last char is a Hangul syllable with
      //                   NO jongseong (받침 없음: 가, 나, 수, 메모리)
      //   "regular"     — last char has jongseong ≠ ㄹ (받침 ㄱ,ㄴ,
      //                   ㄷ,ㅁ,ㅂ,ㅅ,ㅇ,ㅈ,ㅊ,ㅋ,ㅌ,ㅍ,ㅎ etc.):
      //                   값, 각, 문, 컴퓨터
      //   "rieul"       — last char has jongseong == ㄹ
      //                   (한글, 술, 물, 코드)
      //   "non-korean"  — last char is NOT a Hangul syllable
      //                   (English noun like "count", number "42",
      //                   mixed-script). Caller decides fallback
      //                   policy (BN lens currently defaults
      //                   non-Korean to "none" finalKind).
      //
      // Substrate-native: uses pnix_core::lang::ko::
      // decompose_hangul_syllable. NO external NLP dependency
      // (no mecab-ko, no lindera, no rustkorean). The 19 × 21 ×
      // 28 = 11,172 Hangul syllable algebra is mathematical;
      // King Sejong's design is a closed combinatorial table,
      // not a probabilistic NLP problem.
      let s = args[0].as_str().ok_or_else(|| {
        anyhow!(
          "builtins.koreanFinalConsonantKind: expected string, got {}",
          type_name(&args[0])
        )
      })?;
      let kind = match s.chars().last() {
        None => "non-korean", // empty string — substrate honest
        Some(ch) => {
          match pnix_core::lang::ko::decompose_hangul_syllable(ch) {
            None => "non-korean",
            Some(syl) => match syl.jongseong {
              None => "none",
              // ㄹ jamo char is U+11AF (initial-ㄹ is U+1105;
              // standalone ㄹ is U+3139). decompose_hangul_syllable
              // returns the JONGSEONG-position jamo. Match on the
              // canonical Hangul jongseong jamo char.
              Some(jong) if jong == 'ㄹ' => "rieul",
              Some(_) => "regular",
            },
          }
        }
      };
      Ok(Value::String(kind.to_string()))
    }
    "elem" if args.len() >= 2 => {
      let needle = &args[0];
      match &args[1] {
        Value::List(items) => {
          let mut found = false;
          for v in items.iter() {
            if values_equal(v, needle)? {
              found = true;
              break;
            }
          }
          Ok(Value::Bool(found))
        }
        other => Err(anyhow!(
          "builtins.elem: second argument must be list, got {}",
          type_name(other)
        )),
      }
    }
    "listToAttrs" if args.len() >= 1 => {
      // Nix-correct + production fail-loud: every list element
      // must be an attrset shaped like `{ name = "..."; value =
      // …; }`. Previously we silent-skipped entries missing
      // `name` or `value`, which let typos bypass user guards.
      // Nix-compat: duplicate names → **first** entry wins
      // (matches `nix/src/libexpr/primops.cc::prim_listToAttrs`,
      // which `seen.insert(name).second`-guards the add). The
      // earlier comment claimed last-wins; it was wrong. See
      // `eval_filter_elem_listtoattrs.rs::list_to_attrs_duplicate_first_wins`.
      let items = match &args[0] {
        Value::List(l) => l,
        other => {
          return Err(anyhow!(
            "builtins.listToAttrs: expected list, got {}",
            type_name(other)
          ));
        }
      };
      let mut out = BTreeMap::new();
      for (idx, item) in items.iter().enumerate() {
        let item = force_if_thunk(item)?;
        let m = match item.as_ref() {
          Value::AttrSet(m) => m,
          other => {
            return Err(anyhow!(
              "builtins.listToAttrs: list element at index {} must be attrset, got {}",
              idx,
              type_name(other)
            ));
          }
        };
        let name_v = m.get("name").ok_or_else(|| {
          anyhow!(
            "builtins.listToAttrs: list element at index {} is missing 'name'",
            idx
          )
        })?;
        let name_v = force_if_thunk(name_v)?;
        let name = name_v.as_ref().as_str().ok_or_else(|| {
          anyhow!(
            "builtins.listToAttrs: list element at index {} has 'name' of type {}, expected string",
            idx,
            type_name(name_v.as_ref())
          )
        })?;
        let value = m.get("value").cloned().ok_or_else(|| {
          anyhow!(
            "builtins.listToAttrs: list element at index {} is missing 'value'",
            idx
          )
        })?;
        // Nix: first entry wins. Skip duplicates.
        out.entry(name.to_string()).or_insert(value);
      }
      Ok(Value::AttrSet(Arc::new(out)))
    }
    "removeAttrs" if args.len() >= 2 => {
      // Lazy in attrset values (do not force them); force the list of
      // names so we know which keys to drop. Both args are now type
      // guarded — previous code silently returned the original
      // attrset on a non-list name arg, and silently returned an empty
      // attrset on a non-attrset first arg.
      let attrs_value = force_if_thunk(&args[0])?;
      let Value::AttrSet(attrs) = attrs_value.as_ref() else {
        return Err(anyhow!(
          "builtins.removeAttrs: first argument must be attrset, got {}",
          type_name(attrs_value.as_ref())
        ));
      };
      let names_value = force_if_thunk(&args[1])?;
      let names_list = match names_value.as_ref() {
        Value::List(items) => items,
        other => {
          return Err(anyhow!(
            "builtins.removeAttrs: second argument must be list of strings, got {}",
            type_name(other)
          ));
        }
      };
      let mut remove_names: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
      for (idx, item) in names_list.iter().enumerate() {
        let forced = force_if_thunk(item)?;
        let Some(name) = forced.as_ref().as_str() else {
          return Err(anyhow!(
            "builtins.removeAttrs: name-list element at index {} is {}, not a string",
            idx,
            type_name(forced.as_ref())
          ));
        };
        remove_names.insert(name.to_string());
      }
      let mut filtered: BTreeMap<String, Value> = BTreeMap::new();
      for (key, value) in attrs.iter() {
        if !remove_names.contains(key.as_str()) {
          filtered.insert(key.clone(), value.clone());
        }
      }
      Ok(Value::AttrSet(Arc::new(filtered)))
    }
    "lessThan" if args.len() >= 2 => order_compare("<", &args[0], &args[1]),
    "add" if args.len() >= 2 => {
      arith_binary(&args[0], &args[1], "+", i64::checked_add, |a, b| a + b)
    }
    "sub" if args.len() >= 2 => {
      arith_binary(&args[0], &args[1], "-", i64::checked_sub, |a, b| a - b)
    }
    "mul" if args.len() >= 2 => {
      arith_binary(&args[0], &args[1], "*", i64::checked_mul, |a, b| a * b)
    }
    "div" if args.len() >= 2 => match (&args[0], &args[1]) {
      (Value::Int(_), Value::Int(0)) => Err(anyhow!("builtins.div: division by zero")),
      _ => {
        if let Some(d) = args[1].as_f64() {
          if d == 0.0 {
            return Err(anyhow!("builtins.div: division by zero"));
          }
        }
        arith_binary(&args[0], &args[1], "/", i64::checked_div, |a, b| a / b)
      }
    },
    "mod" if args.len() >= 2 => match (&args[0], &args[1]) {
      (Value::Int(a), Value::Int(b)) => {
        if *b == 0 {
          Err(anyhow!("builtins.mod: division by zero"))
        } else {
          // 2026-05-05: i64::MIN % -1 is the canonical
          // two's-complement remainder overflow case (the
          // mathematical result `0` is fine, but Rust's `%`
          // operator panics on this combination because the
          // intermediate division would overflow). The binary
          // `%` operator already uses `checked_rem`; the
          // `builtins.mod` arm was the missing twin.
          a.checked_rem(*b)
            .map(Value::Int)
            .ok_or_else(|| anyhow!("integer overflow: {} % {}", a, b))
        }
      }
      _ => {
        let a = expect_f64(&args[0], "builtins.mod")?;
        let b = expect_f64(&args[1], "builtins.mod")?;
        if b == 0.0 {
          Err(anyhow!("builtins.mod: division by zero"))
        } else {
          Ok(Value::Float(a % b))
        }
      }
    },
    "neg" if args.len() >= 1 => match &args[0] {
      // 2026-05-05: `-i64::MIN` overflows because i64::MIN
      // has no positive counterpart. Use `checked_neg` to
      // surface a typed error instead of a Rust panic. The
      // binary unary `-` operator (in `eval_unary`) had the
      // same panic shape — it now also uses checked_neg via
      // this same family. (See slice #47 spec.)
      Value::Int(i) => i
        .checked_neg()
        .map(Value::Int)
        .ok_or_else(|| anyhow!("integer overflow: -{}", i)),
      Value::Float(f) => Ok(Value::Float(-f)),
      _ => Err(anyhow!("builtins.neg: argument must be number")),
    },
    "abs" if args.len() >= 1 => match &args[0] {
      Value::Int(i) => i
        .checked_abs()
        .map(Value::Int)
        .ok_or_else(|| anyhow!("builtins.abs: integer overflow")),
      Value::Float(f) => Ok(Value::Float(f.abs())),
      _ => Err(anyhow!("builtins.abs: argument must be number")),
    },
    "pow" if args.len() >= 2 => {
      // 2026-05-05: previously routed every `pow` through `f64.powf`
      // and then `collapse_numeric` to maybe coerce back to int. That
      // shape silently lost precision: `pow 2 63` returned the int
      // `9223372036854775807` (i64::MAX) but the true value is 2^63
      // = 9223372036854775808 (one MORE than i64::MAX); `pow 3 39`
      // returned `4052555153018976256` but the true value is
      // 4052555153018976267 — off by 11 from precision loss in f64.
      // The `collapse_numeric` boundary check was `<= i64::MAX as
      // f64`, which let through values that, when cast to i64,
      // saturate to i64::MAX silently.
      // Fix: for int^int with non-negative exponent, use
      // `i64::checked_pow` (exact integer arithmetic). On overflow,
      // fall back to `f64.powf` so the result is at least a Float
      // (the user can see they hit the float representation domain).
      // Negative exponent / float operands keep the float path.
      if let (Value::Int(a), Value::Int(b)) = (&args[0], &args[1]) {
        if *b >= 0 {
          if let Ok(exp) = u32::try_from(*b) {
            if let Some(v) = a.checked_pow(exp) {
              return Ok(Value::Int(v));
            }
          }
          // Overflow or exp > u32::MAX → fall through to float.
        }
      }
      let a = expect_f64(&args[0], "builtins.pow")?;
      let b = expect_f64(&args[1], "builtins.pow")?;
      Ok(Value::Float(a.powf(b)))
    }
    "sqrt" if args.len() >= 1 => {
      let value = expect_f64(&args[0], "builtins.sqrt")?;
      if value < 0.0 {
        Err(anyhow!("builtins.sqrt: cannot take sqrt of negative"))
      } else {
        Ok(Value::Float(value.sqrt()))
      }
    }
    // 2026-05-05: previously `value.floor() as i64` (and same
    // for ceil) was a saturating-and-NaN-zero cast — Rust's
    // `as i64` rules:
    //   NaN              → 0
    //   +inf or > i64::MAX → i64::MAX
    //   -inf or < i64::MIN → i64::MIN
    // So `floor (1.0e200 * 1.0e200)` (= +inf) silently
    // returned `9223372036854775807`, `floor (NaN)` silently
    // returned `0`, and `floor 1.0e200` (a finite but
    // out-of-i64-range float) silently saturated. Real Nix
    // errors on these inputs. Match: reject NaN, reject values
    // outside `[i64::MIN as f64, i64::MAX as f64]` with a
    // typed error before the cast. Note that the i64::MAX
    // boundary is exclusive on the float side because
    // `i64::MAX as f64 = 9223372036854775808.0` (not exactly
    // representable; round-up by 1) — using strict `>` and
    // `<` against the float cast of i64::MAX/MIN keeps the
    // round-trip safe.
    "floor" if args.len() >= 1 => {
      let value = expect_f64(&args[0], "builtins.floor")?;
      let floored = value.floor();
      Ok(Value::Int(float_to_i64_checked(floored, "builtins.floor")?))
    }
    "ceil" if args.len() >= 1 => {
      let value = expect_f64(&args[0], "builtins.ceil")?;
      let ceiled = value.ceil();
      Ok(Value::Int(float_to_i64_checked(ceiled, "builtins.ceil")?))
    }
    "exp" if args.len() >= 1 => Ok(Value::Float(expect_f64(&args[0], "builtins.exp")?.exp())),
    "ln" | "log" if args.len() >= 1 => {
      let value = expect_f64(&args[0], "builtins.ln")?;
      if value <= 0.0 {
        Err(anyhow!("builtins.ln: argument must be positive"))
      } else {
        Ok(Value::Float(value.ln()))
      }
    }
    "sin" if args.len() >= 1 => Ok(Value::Float(expect_f64(&args[0], "builtins.sin")?.sin())),
    "cos" if args.len() >= 1 => Ok(Value::Float(expect_f64(&args[0], "builtins.cos")?.cos())),
    "tan" if args.len() >= 1 => Ok(Value::Float(expect_f64(&args[0], "builtins.tan")?.tan())),
    "atan2" if args.len() >= 2 => Ok(Value::Float(
      expect_f64(&args[0], "builtins.atan2")?.atan2(expect_f64(&args[1], "builtins.atan2")?),
    )),
    "readFileType" if args.len() >= 1 => {
      let path = resolve_value_path(&args[0], "builtins.readFileType")?;
      let metadata = fs::symlink_metadata(&path).map_err(|e| {
        anyhow!(
          "builtins.readFileType: failed to get metadata for '{}': {}",
          path.display(),
          e
        )
      })?;
      Ok(Value::String(
        file_type_to_nix_string(metadata.file_type()).to_string(),
      ))
    }
    "readFile" if args.len() >= 1 => {
      let path = resolve_value_path(&args[0], "builtins.readFile")?;
      let metadata = fs::metadata(&path).map_err(|e| {
        anyhow!(
          "builtins.readFile: cannot access '{}': {}",
          path.display(),
          e
        )
      })?;
      if metadata.len() > READFILE_MAX_SIZE {
        return Err(anyhow!(
          "builtins.readFile: file '{}' is too large ({} bytes, max {} bytes)",
          path.display(),
          metadata.len(),
          READFILE_MAX_SIZE
        ));
      }
      let content = read_file_text_with_cache(&path, &metadata)?;
      Ok(Value::String(content))
    }
    "toXML" if args.len() >= 1 => {
      // Mirrors nix/src/libexpr/value-to-xml.cc — produces a strict
      // depth-first XML rendering of the value. Recursive attrsets are
      // forced (each field shows its current value, not the source
      // expression). Position info is omitted (nix accepts both forms;
      // the canonical lang test omits it).
      let value = deep_force(args[0].clone())?;
      let mut out = String::with_capacity(
        TO_XML_HEADER.len() + xml_initial_capacity(&value) + TO_XML_FOOTER.len(),
      );
      out.push_str(TO_XML_HEADER);
      write_value_as_xml(&value, &mut out, 1);
      out.push_str(TO_XML_FOOTER);
      Ok(Value::String(out))
    }
    "toFile" if args.len() >= 2 => {
      let Some(name) = args[0].as_str() else {
        return Err(anyhow!("builtins.toFile: first argument must be string"));
      };
      let Some(contents) = args[1].as_str() else {
        return Err(anyhow!("builtins.toFile: second argument must be string"));
      };
      // 2026-05-05 (slice #55): toFile content must NOT have
      // non-empty string context. Real Nix rejects this case
      // because `toFile` produces a deterministic store path
      // whose hash is computed from the literal content text —
      // if the content references a derivation (via context),
      // the derivation would have to be realized BEFORE toFile
      // can compute the resulting path, but `toFile` is an
      // evaluation-time operation, not a build-time one.
      // Pre-fix pnix silently accepted context-bearing content
      // and produced a path file containing the interpolated
      // text, but lost the dependency tracking — any
      // downstream consumer reading the resulting Path would
      // see a path that LOOKS like a self-contained store
      // entry but secretly depended on derivations whose
      // build-time markers were silently dropped at this
      // boundary. Error fail-loud and name the
      // `unsafeDiscardStringContext` escape hatch for authors
      // who genuinely want to discard.
      if let Some(ctx) = args[1].string_context() {
        if !ctx.is_empty() {
          return Err(anyhow!(
            "builtins.toFile: content cannot reference derivations / paths via string context. \
             toFile produces an evaluation-time store entry whose hash is computed from the \
             literal content; build-time dependencies cannot be encoded into that hash. \
             Use builtins.unsafeDiscardStringContext if you want to inline the resolved text \
             without tracking the dependency."
          ));
        }
      }

      let hash_prefix = source_sha256_hex_prefix_32(contents);
      let mut safe_name = String::with_capacity(name.len());
      for ch in name.chars() {
        safe_name.push(
          if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_' | '+') {
            ch
          } else {
            '_'
          },
        );
      }

      let mut store_dir = std::env::temp_dir();
      store_dir.push("pnix-nix-store");
      fs::create_dir_all(&store_dir).map_err(|e| {
        anyhow!(
          "builtins.toFile: failed to create '{}': {}",
          store_dir.display(),
          e
        )
      })?;

      let mut out_path = store_dir;
      out_path.push(to_file_store_name(&hash_prefix, &safe_name));
      fs::write(&out_path, contents).map_err(|e| {
        anyhow!(
          "builtins.toFile: failed to write '{}': {}",
          out_path.display(),
          e
        )
      })?;
      Ok(Value::Path(out_path))
    }
    // JSON round-trip surface. The names were already registered as
    // `BuiltinPartial` aliases in `builtins_attrset()`, but had no
    // matching `apply_builtin` arm — so e.g. `builtins.toJSON 42`
    // silently returned the unfinished `BuiltinPartial { name:
    // "toJSON", args: [Int(42)] }` instead of the JSON string.
    // `Value::to_json()` is already cycle-safe (commit aad08893 path
    // stack), and `markup::json_to_value` handles `null`/`bool`/
    // `number`/`string`/`array`/`object` -> `Value`. Both surfaces
    // need only `args[0]` since the boundary deep-forces it for us.
    "toJSON" if args.len() >= 1 => {
      // Nix-correct + production fail-loud: JSON has no
      // representation for `inf` / `-inf` / `NaN`. Real Nix
      // errors with "cannot serialize float Inf as JSON" rather
      // than silently flattening to `null`. The previous impl
      // routed through `Value::to_json`, which uses
      // `serde_json::Number::from_f64` and returns `null` for
      // non-finite floats — that bypassed user `r != null`
      // guards.
      //
      // 2026-05-05 (slice #74): propagate string context from
      // input tree to result. Same fix as slice #73's emit
      // family. Pre-fix `toJSON [ "x${./p}" ]` returned plain
      // `Value::String("[\"x...\"]")` with empty context —
      // silent metadata loss. The serialized JSON text
      // VISIBLY contains the resolved store path, but
      // downstream consumers (subsequent string operations,
      // hashing) saw no dependency.
      // Reuse the same context rules while performing the finite
      // JSON precheck so we do not walk the input tree twice.
      let ctx = check_json_finite_and_collect_contexts(&args[0], "builtins.toJSON")?;
      let text = args[0].to_json();
      Ok(Value::string_with_context(text, ctx))
    }
    "fromJSON" if args.len() >= 1 => {
      let s = args[0]
        .as_str()
        .ok_or_else(|| anyhow!("builtins.fromJSON: expected string"))?;
      let parsed: serde_json::Value =
        serde_json::from_str(s).map_err(|e| anyhow!("builtins.fromJSON: parse error: {}", e))?;
      // 2026-05-06 (slice #78): post-validate against silent
      // i64 overflow. serde_json silently widens overflowing
      // integer literals to f64, losing precision (e.g.
      // `"999999999999999999999"` → `Float(1e+21)`). Real Nix
      // errors on integer overflow during JSON parse. This pre-
      // scan walks the source for integer-shaped tokens
      // (`-?\d+` not followed by `.` / `e` / `E`) and rejects
      // any that don't fit in i64. Float tokens pass through
      // serde unchanged.
      markup::check_json_no_int_overflow(s)?;
      Ok(markup::json_to_value(&parsed))
    }
    "fromTOML" if args.len() >= 1 => {
      // Parse a TOML document and return the equivalent attrset.
      // Error path is fail-loud: invalid TOML, non-string input,
      // and non-finite float values inside the TOML all surface
      // at the call site.
      let s = args[0].as_str().ok_or_else(|| {
        anyhow!(
          "builtins.fromTOML: expected string, got {}",
          type_name(&args[0])
        )
      })?;
      let parsed =
        pnix_toml::parse(s).map_err(|e| anyhow!("builtins.fromTOML: parse error: {}", e))?;
      Ok(toml_to_value(&parsed))
    }
    "hashString" if args.len() >= 2 => {
      // Production policy: only cryptographically current hashes
      // are exposed. SHA-256 and SHA-512 are supported. MD5 and
      // SHA-1 are explicitly rejected even though older Nix
      // accepts them — they are unsuitable for content
      // addressing in a 2026-era language. Real Nix is moving
      // the same direction; deprecation in upstream is in
      // progress.
      let algo = args[0].as_str().ok_or_else(|| {
        anyhow!(
          "builtins.hashString: first argument (algo) must be string, got {}",
          type_name(&args[0])
        )
      })?;
      let data = args[1].as_str().ok_or_else(|| {
        anyhow!(
          "builtins.hashString: second argument (data) must be string, got {}",
          type_name(&args[1])
        )
      })?;
      use pnix_hash::Digest;
      let hex = match algo {
        "sha256" => {
          let mut h = pnix_hash::Sha256::new();
          h.update(data.as_bytes());
          let bytes = h.finalize();
          hex_lower(&bytes)
        }
        "sha512" => {
          let mut h = pnix_hash::Sha512::new();
          h.update(data.as_bytes());
          let bytes = h.finalize();
          hex_lower(&bytes)
        }
        "md5" | "sha1" => {
          return Err(anyhow!(
            "builtins.hashString: algorithm '{}' is not supported \
             (cryptographically broken — use 'sha256' or 'sha512')",
            algo
          ));
        }
        other => {
          return Err(anyhow!(
            "builtins.hashString: unsupported algorithm '{}' \
             (supported: 'sha256', 'sha512')",
            other
          ));
        }
      };
      // 2026-05-05: preserve string-context from the hashed
      // input. Real Nix's `hashString` propagates context — the
      // hash *depends on* the path, so the dependency is real.
      // Pre-fix produced a bare hex `Value::String`, silently
      // losing the dependency marker. Same shape as the slice
      // #49 family (concatStringsSep / substring /
      // replaceStrings / toString).
      Ok(Value::string_with_optional_context_ref(
        hex,
        args[1].string_context(),
      ))
    }
    // 2026-05-05 (slice #68): `builtins.hashFile algo path` —
    // hash the CONTENT of a file at the given path. Same
    // production-grade algorithm policy as hashString: only
    // SHA-256 and SHA-512 supported; MD5 and SHA-1 explicitly
    // rejected. Pre-fix pnix did not implement hashFile at
    // all, so any `.px` code that used it errored with
    // `attribute 'hashFile' not found` — a misleading error
    // suggesting a typo or missing attribute, when the actual
    // issue was that the builtin wasn't implemented.
    //
    // The result string carries the path's context (the hash
    // depends on the file content, which depends on the path
    // itself — same dependency-tracking logic as hashString).
    "hashFile" if args.len() >= 2 => {
      let algo = args[0].as_str().ok_or_else(|| {
        anyhow!(
          "builtins.hashFile: first argument (algo) must be string, got {}",
          type_name(&args[0])
        )
      })?;
      let path = resolve_value_path(&args[1], "builtins.hashFile")?;
      let bytes = fs::read(&path).map_err(|e| {
        anyhow!(
          "builtins.hashFile: failed to read '{}': {}",
          path.display(),
          e
        )
      })?;
      use pnix_hash::Digest;
      let hex = match algo {
        "sha256" => {
          let mut h = pnix_hash::Sha256::new();
          h.update(&bytes);
          let bytes = h.finalize();
          hex_lower(&bytes)
        }
        "sha512" => {
          let mut h = pnix_hash::Sha512::new();
          h.update(&bytes);
          let bytes = h.finalize();
          hex_lower(&bytes)
        }
        "md5" | "sha1" => {
          return Err(anyhow!(
            "builtins.hashFile: algorithm '{}' is not supported \
             (cryptographically broken — use 'sha256' or 'sha512')",
            algo
          ));
        }
        other => {
          return Err(anyhow!(
            "builtins.hashFile: unsupported algorithm '{}' \
             (supported: 'sha256', 'sha512')",
            other
          ));
        }
      };
      // Propagate the path's context to the result hex
      // string. The hash output depends on both the file
      // content AND which path was hashed; preserving the
      // path-context lets downstream consumers track the
      // dependency.
      Ok(Value::string_with_optional_context_ref(
        hex,
        args[1].string_context(),
      ))
    }
    _ => Ok(builtin_partial_value_with_args(name, args.to_vec())),
  }
}

// batch 263 (2026-04-18): arith builtin 의 공통 Int/Float promote 로직.
//
// 2026-05-05: previously the `int_op` parameter was `fn(i64,
// i64) -> i64`, which silently used Rust's wrapping `+` / `-` /
// `*` operations. In debug builds this panicked with "attempt
// to add with overflow" — a Rust panic that crashes the
// evaluator process; in release builds it would have wrapped
// silently, which is even worse for production correctness.
// Real Nix and pnix's binary `+` / `-` / `*` (`arith_op` at
// line 1980) already use `checked_add` / `checked_sub` /
// `checked_mul` and surface a typed `integer overflow` error.
// `builtins.add` / `sub` / `mul` / `div` were the missing
// twins. Match them: take the int op as `fn(i64, i64) ->
// Option<i64>` (so callers pass `i64::checked_add` etc.),
// surface `None` as a typed overflow error with the operator
// name in the message.
fn arith_binary(
  a: &Value,
  b: &Value,
  op_name: &str,
  int_op: fn(i64, i64) -> Option<i64>,
  float_op: fn(f64, f64) -> f64,
) -> Result<Value> {
  match (a, b) {
    (Value::Int(x), Value::Int(y)) => int_op(*x, *y)
      .map(Value::Int)
      .ok_or_else(|| anyhow!("integer overflow: {} {} {}", x, op_name, y)),
    (Value::Float(x), Value::Float(y)) => Ok(Value::Float(float_op(*x, *y))),
    (Value::Int(x), Value::Float(y)) => Ok(Value::Float(float_op(*x as f64, *y))),
    (Value::Float(x), Value::Int(y)) => Ok(Value::Float(float_op(*x, *y as f64))),
    _ => Err(anyhow!("arith: non-numeric operand")),
  }
}

fn maybe_apply_lazy_builtin(
  func: &Value,
  arg_expr: &Arc<PnixExpr>,
  env: &Env,
) -> Option<Result<Value>> {
  let Value::BuiltinPartial { name, args } = func else {
    return None;
  };

  match (name.as_ref(), args.len()) {
    // 2026-05-05: `tryEval` must NOT catch `abort`. In Nix, `abort`
    // is the unrecoverable hard-fail and propagates through
    // `tryEval` with `value` never being constructed. Previously
    // we routed every `Err` through `try_eval_result(Err(_))` and
    // returned `{success=false; value=false;}`, silently turning
    // `abort` into a recoverable signal — production code that
    // wraps risky `assert`/`throw` paths in `tryEval` would also
    // silently absorb a `builtins.abort` from a corrupted upstream
    // module. The `abort` arm produces an error whose message
    // begins with "evaluation aborted:"; we re-raise on that
    // marker, and only the non-abort errors continue into
    // `try_eval_result` to be wrapped as `{success=false;}`.
    ("tryEval", 0) => match eval_arc(arg_expr.clone(), env) {
      Ok(v) => Some(Ok(try_eval_result(Ok(v)))),
      Err(e) => {
        if e.to_string().starts_with("evaluation aborted:") {
          Some(Err(e))
        } else {
          Some(Ok(try_eval_result(Err(e))))
        }
      }
    },
    ("seq", 1) => Some(eval_arc(arg_expr.clone(), env)),
    ("deepSeq", 1) => {
      Some(deep_force_value(&args[0]).and_then(|_| eval_arc(arg_expr.clone(), env)))
    }
    ("addErrorContext", 1) => {
      // 2026-05-05 (slice #53): pre-fix the partial-handler used
      // `unwrap_or_else(|| format!("{}", args[0]))` which silently
      // formatted the non-string context (so `addErrorContext 42 "v"`
      // produced a `42:` error prefix instead of erroring on the
      // type mismatch). Mirror the line ~3624 main arm which now
      // requires `args[0]` to be a string. If the partial handler
      // is reached, the context arg has already been validated by
      // the boundary force; we still re-check to keep the error
      // message specific.
      // Regression fix (2026-06-10): force the CONTEXT arg before the
      // string check. The 2026-06 "keep trace payloads lazy" change
      // moved addErrorContext into the no-boundary-force bucket (so the
      // VALUE payload stays lazy — correct), but this partial handler
      // still assumed args[0] arrived pre-forced and rejected every
      // thunk-wrapped context with a spurious "got thunk" type error.
      // Only the context is forced here; the value arg remains lazy via
      // the eval_arc-with-map_err shape below.
      let ctx_forced = match force_if_thunk(&args[0]) {
        Ok(v) => v,
        Err(e) => return Some(Err(e)),
      };
      let Some(ctx) = ctx_forced.as_ref().as_str().map(str::to_string) else {
        return Some(Err(anyhow!(
          "builtins.addErrorContext: context arg must be string, got {}",
          type_name(ctx_forced.as_ref())
        )));
      };
      Some(eval_arc(arg_expr.clone(), env).map_err(|err| anyhow!("{}: {}", ctx, err)))
    }
    // Nix-compat: `foldl'` is NOT strict in its initial accumulator.
    // `foldl' op (throw "x") [1 2]` returns the result of the last op
    // without ever forcing the throw. We keep init as an unforced thunk
    // by intercepting the second-arg apply.
    ("foldl'", 1) => {
      let mut new_args = args.clone();
      new_args.push(make_thunk_arc(arg_expr.clone(), env));
      Some(Ok(Value::BuiltinPartial {
        name: name.clone(),
        args: new_args,
      }))
    }
    // Same lazy-init semantics for foldr: `foldr op (throw "x") []`
    // returns the unforced initial without forcing the throw.
    ("foldr", 1) => {
      let mut new_args = args.clone();
      new_args.push(make_thunk_arc(arg_expr.clone(), env));
      Some(Ok(Value::BuiltinPartial {
        name: name.clone(),
        args: new_args,
      }))
    }
    _ => None,
  }
}

fn try_eval_result(result: Result<Value>) -> Value {
  let mut attrs = BTreeMap::new();
  match result {
    Ok(value) => {
      attrs.insert("success".to_string(), Value::Bool(true));
      attrs.insert("value".to_string(), value);
    }
    Err(_) => {
      // Nix-correct: on failure `value` is `false` (Bool), not
      // `null`. The manual: "Return a set containing the
      // attributes success (true if e evaluated successfully,
      // false if an error was thrown) and value, equal to e if
      // successful and false on error." Real nixpkgs code
      // matches on `value == false` after a failed tryEval, so
      // returning Null silently bypassed those checks.
      attrs.insert("success".to_string(), Value::Bool(false));
      attrs.insert("value".to_string(), Value::Bool(false));
    }
  }
  Value::AttrSet(Arc::new(attrs))
}

fn emit_trace_message(value: &Value) {
  match value {
    Value::String(s) => eprintln!("trace: {}", s),
    other => eprintln!("trace: {}", other),
  }
}

fn verbose_mode_enabled() -> bool {
  static VERBOSE_MODE: OnceLock<bool> = OnceLock::new();
  *VERBOSE_MODE.get_or_init(
    || matches!(std::env::var("PNIX_VERBOSE"), Ok(v) if v != "0" && !v.trim().is_empty()),
  )
}

fn function_args_result(value: &Value) -> Result<Value> {
  match value {
    Value::Lambda { param, .. } => {
      let mut result = BTreeMap::new();
      match param.as_ref() {
        PnixParamPattern::Ident(_) | PnixParamPattern::List(_) => {}
        PnixParamPattern::AttrSet { fields, .. }
        | PnixParamPattern::AttrSetWithBind { fields, .. } => {
          for field in fields {
            result.insert(field.name.clone(), Value::Bool(field.default.is_some()));
          }
        }
      }
      Ok(Value::AttrSet(Arc::new(result)))
    }
    Value::BuiltinPartial { .. } => Ok(Value::AttrSet(Arc::new(BTreeMap::new()))),
    _ => Err(anyhow!("builtins.functionArgs: expected function")),
  }
}

fn deep_force_value(value: &Value) -> Result<()> {
  // 2026-05-05: previously the depth limit (1_000) was the only
  // cycle protection here. For cyclic values like `let r = { a
  // = r; }; in r`, the recursion adds heavy frames per cycle
  // iteration and the 2 MB Rust test thread stack overflows
  // long before depth=1000 is reached. Real Nix errors with
  // "infinite recursion encountered" on `deepSeq` of a cyclic
  // value. Add Rc-pointer cycle tracking via a per-call path
  // stack, mirroring `deep_force_visited` and the new
  // `check_json_finite_visited`.
  let started = deep_force_timing_enabled().then(std::time::Instant::now);
  let mut perf = DeepForcePerf::default();
  if value_is_deep_force_leaf(value) {
    observe_deep_force_value(value, 0, &mut perf);
    record_deep_force_perf(
      &perf,
      started.map_or_else(Default::default, |s| s.elapsed()),
    );
    return Ok(());
  }
  let mut path: Vec<Arc<std::sync::OnceLock<Value>>> = Vec::with_capacity(8);
  let result = deep_force_value_at_depth(value, 0, &mut path, &mut perf);
  record_deep_force_perf(
    &perf,
    started.map_or_else(Default::default, |s| s.elapsed()),
  );
  result
}

fn deep_force_value_at_depth(
  value: &Value,
  depth: usize,
  path: &mut Vec<Arc<std::sync::OnceLock<Value>>>,
  perf: &mut DeepForcePerf,
) -> Result<()> {
  if depth > DEEP_FORCE_MAX_DEPTH {
    anyhow::bail!("builtins.deepSeq: max traversal depth exceeded");
  }
  observe_deep_force_value(value, depth, perf);
  match value {
    Value::Thunk { cache, .. } => {
      if path.iter().any(|c| Arc::ptr_eq(c, cache)) {
        return Err(anyhow!(
          "builtins.deepSeq: infinite recursion encountered (cyclic value)"
        ));
      }
      path.push(cache.clone());
      let forced = force_value(value.clone())?;
      let result = deep_force_value_at_depth(&forced, depth + 1, path, perf);
      path.pop();
      return result;
    }
    Value::List(items) => {
      for item in items.iter() {
        deep_force_value_at_depth(item, depth + 1, path, perf)?;
      }
    }
    Value::AttrSet(map) => {
      for value in map.values() {
        deep_force_value_at_depth(value, depth + 1, path, perf)?;
      }
    }
    _ => {}
  }
  Ok(())
}

fn expect_i64(value: &Value, context: &str) -> Result<i64> {
  match value {
    Value::Int(i) => Ok(*i),
    _ => Err(anyhow!("{context}: expected integer, got {}", value)),
  }
}

// 2026-05-05: shared helper for `builtins.floor` / `ceil` (and
// any future float-to-int cast). The bare `as i64` cast in Rust
// silently saturates on out-of-range and silently maps NaN to 0,
// which the audit catches as silent-pass. Reject NaN and
// out-of-range explicitly with a typed error so authors see the
// real input problem instead of a wrong-but-non-erroring number.
fn float_to_i64_checked(value: f64, context: &str) -> Result<i64> {
  if value.is_nan() {
    return Err(anyhow!("{context}: cannot convert NaN to integer"));
  }
  if !value.is_finite() {
    return Err(anyhow!(
      "{context}: cannot convert {} to integer",
      if value > 0.0 { "+inf" } else { "-inf" }
    ));
  }
  // i64::MAX as f64 = 9223372036854775808.0 (rounded up — not
  // exactly representable), and i64::MIN as f64 = -9223372036854775808.0
  // (exactly representable). Use strict `<` against MAX-cast and
  // `>=` against MIN-cast so the boundary cases that round
  // outside i64 are rejected.
  if value >= i64::MAX as f64 || value < i64::MIN as f64 {
    return Err(anyhow!("{context}: float {} is outside i64 range", value));
  }
  Ok(value as i64)
}

// 2026-05-05: shared helper for the bool-required positions —
// `if`, `&&`, `||`, `->`, `!`, `assert`, match-arm guard, and the
// pnix-specific `builtins.and / or / not` helpers. Previously
// every site called `Value::is_true()`, which truthy-coerces any
// non-`false` / non-`null` / non-`Int(0)` value, so user code
// like `if maybe_str then a else b` (where `maybe_str` came back
// as `"yes"` / `"no"` / `""` / `null` from a deserialiser)
// silently followed the wrong branch instead of erroring on the
// type mismatch. This is the same fail-loud rule already applied
// to `filter` / `any` / `all` / `partition` / `sort` / `assert`
// (slices #33 / #34 / #35 / this slice) — the language operators
// now share the contract their builtin counterparts enforce.
fn expect_bool(value: &Value, context: &str) -> Result<bool> {
  match value {
    Value::Bool(b) => Ok(*b),
    _ => Err(anyhow!(
      "{context}: expected bool, got {}",
      type_name(value)
    )),
  }
}

fn hot_builtin_min_arity(name: &str) -> Option<usize> {
  match name {
    // Stage2 smoke fixtures call `builtins.match pattern s` thousands of
    // times. The first application (`builtins.match pattern`) cannot execute
    // yet, so avoid entering the large `apply_builtin` dispatch table only to
    // rebuild the same partial.
    "match" => Some(2),
    _ => None,
  }
}

fn expect_f64(value: &Value, context: &str) -> Result<f64> {
  value
    .as_f64()
    .ok_or_else(|| anyhow!("{context}: expected number, got {}", value))
}

// 2026-05-05: `collapse_numeric` removed — its only caller was
// `builtins.pow`, which now uses `i64::checked_pow` for the
// int^int path (exact arithmetic) and `f64.powf` only for the
// float fallback. The collapse helper had a precision-loss bug
// (the `<= i64::MAX as f64` boundary let through values that
// saturate on cast, and the `f64::EPSILON` tolerance was
// effectively zero for large floats), so its lazy "try int,
// fall back to float" shape was unsound. If a future op needs
// the same try-int-then-float promotion, it should use
// `i64::checked_X` exactly the way `pow` does — not a
// floating-point round-trip.

fn flatten_value_for_builtin(value: &Value, out: &mut Vec<Value>) -> Result<()> {
  match force_if_thunk(value)? {
    Cow::Owned(Value::List(items)) => {
      out.reserve(items.len());
      for item in items.iter() {
        flatten_value_for_builtin(&item, out)?;
      }
    }
    Cow::Owned(other) => out.push(other),
    Cow::Borrowed(Value::List(items)) => {
      out.reserve(items.len());
      for item in items.iter() {
        flatten_value_for_builtin(item, out)?;
      }
    }
    Cow::Borrowed(other) => out.push(other.clone()),
  }
  Ok(())
}

// 2026-05-05 (slice #65): normalize a path's components,
// collapsing `.` (current dir) and `..` (parent dir) so that
// equivalent path representations compare equal. Real Nix
// normalizes paths at construction time; pnix's `Value::Path`
// preserves the original `PathBuf` text. The pre-fix
// `compare_values` and `values_equal` Path arms compared
// `PathBuf`s literally — `./a/../b == ./b` returned `false`,
// `./a/../b < ./b` errored or returned the wrong ordering.
// Used by both equality and comparison arms; centralizes the
// normalization logic in one place.
//
// Algorithm: walk components, collapse `.` (skip), collapse
// `..` if previous component is a normal name (pop), but keep
// `..` at the start of relative paths (where there's no normal
// component to pop) so we don't silently flip to a different
// path. For absolute paths (rooted at `/`), `..` past root is
// no-op (can't escape).
//
// Preserves the relative-path-ness: a path that started with
// `./` remains relative after normalization (we re-prepend
// `./` if all components collapsed to empty, e.g. `./a/..`
// normalizes to `.`).
fn normalize_pnix_path(path: &std::path::Path) -> std::path::PathBuf {
  let cache_started = cache_lookup_timing_started();
  let cached = NORMALIZED_PATH_CACHE.with(|cache| cache.borrow().get(path).cloned());
  record_cache_lookup_elapsed(cache_started);
  if let Some(normalized) = cached {
    record_path_normalize_cache_hit();
    return normalized;
  }
  record_path_normalize_cache_miss();

  let normalized = normalize_pnix_path_uncached(path);
  NORMALIZED_PATH_CACHE.with(|cache| {
    let mut cache = cache.borrow_mut();
    evict_one_cache_entry(&mut cache, NORMALIZED_PATH_CACHE_MAX_ENTRIES);
    cache.insert(path.to_path_buf(), normalized.clone());
  });
  normalized
}

fn normalize_pnix_path_uncached(path: &std::path::Path) -> std::path::PathBuf {
  use std::path::{Component, PathBuf};
  fn push_component<'a>(out: &mut Vec<Component<'a>>, component: Component<'a>) {
    match component {
      Component::CurDir => {} // skip
      Component::ParentDir => match out.last() {
        Some(Component::Normal(_)) => {
          out.pop();
        }
        Some(Component::RootDir) => {
          // Can't go above root; skip.
        }
        _ => out.push(component),
      },
      other => out.push(other),
    }
  }

  let estimated_components = path.components().size_hint().0;
  let mut components = path.components();
  let first = components.next();
  let started_with_curdir = matches!(first, Some(Component::CurDir));
  let mut out: Vec<Component> = Vec::with_capacity(estimated_components);
  if let Some(component) = first {
    push_component(&mut out, component);
  }
  for component in components {
    push_component(&mut out, component);
  }
  if out.is_empty() {
    PathBuf::from(if started_with_curdir { "." } else { "." })
  } else {
    let mut result = if started_with_curdir
      && !matches!(out.first(), Some(Component::RootDir))
      && !matches!(out.first(), Some(Component::ParentDir))
    {
      PathBuf::from(".")
    } else {
      PathBuf::new()
    };
    for c in out {
      result.push(c.as_os_str());
    }
    result
  }
}

// 2026-05-05 (slice #64): also accept `Value::StringContext`.
// Pre-fix the match arm `Value::String(s) => PathBuf::from(s)`
// only handled plain strings — context-bearing strings fell
// through to the catch-all `_ => Err("expected string or
// path")`. So `pathExists "x${./p}/bin"`, `readFile "${./f}"`,
// `import "${./module}"`, `readDir "${./d}"`, `toPath
// "${./p}"`, `storePath "${./p}"`, `scopedImport scope
// "${./module}"`, `readFileType "${./p}"` ALL silently
// rejected context-bearing string args with a misleading
// "expected string or path" error. Real Nix accepts these
// — the path operation uses the resolved text and may also
// realize the build-time dependencies. For pnix's
// no-derivation-graph design, we use the text portion (the
// context is metadata that can be re-fetched via
// `getContext` if the caller needs it). Fix matches every
// path-resolving builtin in one place — single arm extension
// closes the silent-rejection across the entire family.
// 2026-05-05 (slice #73): walk a value tree and collect all
// string contexts. Used by emit functions (xmlEmit, htmlEmit,
// svgEmit, mathmlEmit, openmathEmit) to propagate context
// from input attrset tree to output emit string. Pre-fix the
// emit family returned plain `Value::String`, silently
// dropping any build-time-dependency markers buried in
// attribute values or text children of the input tree.
//
// Mirrors the slice #57 toString helper but specialized for
// emit's "input is a tree, output is a single string" shape.
// Walks: String/StringContext (collect own context), List
// (recurse elements), AttrSet (recurse values), Path (add
// path display form to context — same shape as slice #54
// `string + path`). Other variants contribute no context.
//
// Cycles in the input tree could cause infinite recursion;
// the slice #41 deepSeq path-stack pattern would be needed
// for full cycle safety. For now, emit functions are not
// expected to receive cyclic input — if they do, the
// downstream emit (xml_emit / etc.) would already error or
// blow stack. Adding cycle safety here is a future slice.
fn collect_value_contexts(value: &Value) -> BTreeSet<String> {
  let mut out = BTreeSet::new();
  collect_value_contexts_into(value, &mut out);
  out
}

fn collect_value_contexts_into(value: &Value, out: &mut BTreeSet<String>) {
  match value {
    Value::StringContext { context, .. } => {
      extend_string_context(out, context);
    }
    Value::Path(p) => {
      out.insert(path_display_string(p));
    }
    Value::List(items) => {
      for item in items.iter() {
        collect_value_contexts_into(item, out);
      }
    }
    Value::AttrSet(map) => {
      for v in map.values() {
        collect_value_contexts_into(v, out);
      }
    }
    Value::Thunk { cache, .. } => {
      // Best-effort: if the thunk has been forced, walk the
      // cached value. Otherwise skip (don't force here — we
      // don't want emit's context-collection to trigger
      // expensive thunk evaluation; the actual emit call
      // already did the forcing through value_to_json /
      // similar).
      if let Some(forced) = cache.get() {
        collect_value_contexts_into(forced, out);
      }
    }
    Value::String(_)
    | Value::Int(_)
    | Value::Float(_)
    | Value::Bool(_)
    | Value::Null
    | Value::Lambda { .. }
    | Value::BuiltinPartial { .. } => {}
  }
}

fn resolve_value_path(value: &Value, context: &str) -> Result<PathBuf> {
  // 2026-05-06 (slice #79): reject empty path. Pre-fix
  // `resolve_value_path("")` produced an empty PathBuf, then
  // the relative-path branch joined it onto `current_import_base()`
  // — silently turning empty into the import-base directory
  // (cwd). That made `builtins.pathExists ""` return `true`
  // and `builtins.readDir ""` return the cwd's contents — both
  // silently incorrect. Real Nix errors with "filename can't be
  // empty" for empty path inputs. Match.
  let path = match value {
    Value::String(s) => {
      if s.is_empty() {
        return Err(anyhow!("{context}: empty string is not a valid path"));
      }
      PathBuf::from(s)
    }
    Value::StringContext { text, .. } => {
      if text.is_empty() {
        return Err(anyhow!("{context}: empty string is not a valid path"));
      }
      PathBuf::from(text)
    }
    Value::Path(p) => {
      if p.as_os_str().is_empty() {
        return Err(anyhow!("{context}: empty path is not valid"));
      }
      p.clone()
    }
    _ => return Err(anyhow!("{context}: expected string or path")),
  };
  // 2026-05-05 (slice #67): normalize the resolved path. Slice
  // #66 normalized `Value::Path` at parse time, but builtins
  // that produce paths via `resolve_value_path` (toPath,
  // storePath, etc.) bypassed that normalization. Now every
  // path consumer (filesystem ops + Path-returning builtins)
  // sees the canonical form. Closes the slice #66 cascade.
  let path = normalize_pnix_path(&path);
  if path.is_relative() {
    with_current_import_base(|base| match base {
      Some(base) => Ok(normalize_pnix_path(&base.join(&path))),
      None => Ok(path),
    })
  } else {
    Ok(path)
  }
}

// batch 265 (2026-04-18): G2.3 ontologyEvaluate 6 축 계산.
//
// interpretation shape 기반 deterministic axes:
//   coherence:      interpretation-id 존재 + contradicted 상태 아님 → 1.0 else 0.5
//   coverage:       facts list 크기 기준 (>=3 → 1.0, 1~2 → 0.5, 0 → 0.0)
//   loss:           losses list 의 합 (없으면 0.0)
//   cost:           cost scalar (없으면 1.0)
//   replayability:  proof_refs / provenance_refs 중 하나라도 있으면 1.0 else 0.5
//   safety:         status != "contradicted" → 1.0 else 0.0
fn compute_evaluation_axes(interp: &BTreeMap<String, Value>) -> (f64, f64, f64, f64, f64, f64) {
  let has_id = interp
    .get("interpretation-id")
    .and_then(|v| v.as_str())
    .map(|s| !s.is_empty())
    .unwrap_or(false);
  let status = interp
    .get("status")
    .and_then(|v| v.as_str())
    .unwrap_or("candidate");

  let coherence = if has_id && status != "contradicted" {
    1.0
  } else {
    0.5
  };

  let facts_len = match interp.get("facts") {
    Some(Value::List(l)) => l.len(),
    _ => 0,
  };
  let coverage = if facts_len >= 3 {
    1.0
  } else if facts_len >= 1 {
    0.5
  } else {
    0.0
  };

  let loss = match interp.get("losses") {
    Some(Value::List(items)) => items.iter().filter_map(|v| v.as_f64()).sum(),
    _ => 0.0,
  };

  let cost = interp.get("cost").and_then(|v| v.as_f64()).unwrap_or(1.0);

  let has_proof = matches!(
    interp.get("proof_refs"),
    Some(Value::List(l)) if !l.is_empty()
  );
  let has_prov = matches!(
    interp.get("provenance_refs"),
    Some(Value::List(l)) if !l.is_empty()
  );
  let replayability = if has_proof || has_prov { 1.0 } else { 0.5 };

  let safety = if status == "contradicted" { 0.0 } else { 1.0 };

  (coherence, coverage, loss, cost, replayability, safety)
}

// batch 265 (2026-04-18): ontologySelect tie-break 키.
// convergence.md tie-break 순서 (내림차순): score → safety → replayability
// → -loss → -cost → -lexical-id.
// Value::Float 직접 비교는 NaN 문제 있어 i64 quantize 로 변환 (×1e6 후 반올림).
#[derive(Debug, Eq, PartialEq)]
struct EvalSelectKey {
  score: i64,
  safety: i64,
  replayability: i64,
  neg_loss: i64,
  neg_cost: i64,
  neg_lex_id: std::cmp::Reverse<String>,
}

impl PartialOrd for EvalSelectKey {
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for EvalSelectKey {
  fn cmp(&self, other: &Self) -> std::cmp::Ordering {
    self
      .score
      .cmp(&other.score)
      .then_with(|| self.safety.cmp(&other.safety))
      .then_with(|| self.replayability.cmp(&other.replayability))
      .then_with(|| self.neg_loss.cmp(&other.neg_loss))
      .then_with(|| self.neg_cost.cmp(&other.neg_cost))
      .then_with(|| self.neg_lex_id.cmp(&other.neg_lex_id))
  }
}

/// OWNER-LAW lane-aware promotion at .px eval time.
///
/// Mirrors `pnix_core::ontology::ontology_promote_with_lane`:
///   - `JudgementAction::Accept` →
///       `Accepted`  if lane ∈ { InternalOwnerLaw, InternalAcceptedMemory }
///       `Candidate` otherwise (external lane downgrade)
///   - `Reject` → `Rejected`
///   - `Hold` → `Held`
///   - `Contradict` → `Contradicted`
///   - missing/unknown action → `Held` (conservative)
///
/// Returns the original judgement attrset extended with `promotion-status`
/// and `promotion-lane` fields so .px / receipts can read them back.
fn ontology_promote_eval(judgement: &Value, lane: &str) -> BTreeMap<String, Value> {
  let mut r = match judgement {
    Value::AttrSet(m) => (**m).clone(),
    _ => BTreeMap::new(),
  };
  let action: String = r
    .get("action")
    .and_then(|v| match v {
      Value::String(s) => Some(s.clone()),
      _ => None,
    })
    .unwrap_or_else(|| "Hold".to_string());
  let allow_direct_accepted = matches!(lane, "InternalOwnerLaw" | "InternalAcceptedMemory");
  let status = match action.as_str() {
    "Accept" => {
      if allow_direct_accepted {
        "Accepted"
      } else {
        // OWNER-LAW lane gate: external Accept must downgrade to Candidate.
        // Promotion to Accepted requires owner-law proof + replay +
        // negative/Held proof on a separate lane-aware call.
        "Candidate"
      }
    }
    "Reject" => "Rejected",
    "Contradict" => "Contradicted",
    _ => "Held",
  };
  r.insert(
    "promotion-status".to_string(),
    Value::String(status.to_string()),
  );
  r.insert(
    "promotion-lane".to_string(),
    Value::String(lane.to_string()),
  );
  if !allow_direct_accepted && action == "Accept" {
    // OWNER-LAW (2026-05-11): distinguish derived-lane clamp from
    // external-lane clamp so audit can tell which kind of untrusted
    // path fired. `InternalDerivedReasoning` is internal but still
    // clamps because composed predicates are new semantic claims.
    let reason = if lane == "InternalDerivedReasoning" {
      format!(
        "derived lane {lane} Accept→Candidate (owner-law: composed predicates are new semantic claims; Accepted parents do not transfer)"
      )
    } else {
      format!(
        "external lane {lane} Accept→Candidate (owner-law: external prose needs owner-law proof for Accepted)"
      )
    };
    r.insert("promotion-reason".to_string(), Value::String(reason));
  }
  r
}

fn eval_select_key(v: &Value) -> EvalSelectKey {
  let quantize = |f: f64| -> i64 { (f * 1_000_000.0).round() as i64 };
  let attrs = match v {
    Value::AttrSet(m) => m,
    _ => {
      return EvalSelectKey {
        score: 0,
        safety: 0,
        replayability: 0,
        neg_loss: 0,
        neg_cost: 0,
        neg_lex_id: std::cmp::Reverse(String::new()),
      }
    }
  };
  // 만약 이미 evaluation-* 값이 채워져 있으면 사용, 없으면 계산.
  let (coh, cov, loss, cost, repl, safe) = if attrs.contains_key("evaluation-score") {
    let f = |k: &str, d: f64| attrs.get(k).and_then(|v| v.as_f64()).unwrap_or(d);
    (
      f("evaluation-coherence", 1.0),
      f("evaluation-coverage", 1.0),
      f("evaluation-loss", 0.0),
      f("evaluation-cost", 1.0),
      f("evaluation-replayability", 1.0),
      f("evaluation-safety", 1.0),
    )
  } else {
    compute_evaluation_axes(attrs)
  };
  let score = attrs
    .get("evaluation-score")
    .and_then(|v| v.as_f64())
    .unwrap_or_else(|| (coh + cov + repl + safe) - (loss + cost));
  let id = attrs
    .get("interpretation-id")
    .and_then(|v| v.as_str())
    .unwrap_or("")
    .to_string();
  EvalSelectKey {
    score: quantize(score),
    safety: quantize(safe),
    replayability: quantize(repl),
    neg_loss: -quantize(loss),
    neg_cost: -quantize(cost),
    neg_lex_id: std::cmp::Reverse(id),
  }
}

fn current_system() -> &'static str {
  #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
  {
    return "aarch64-darwin";
  }
  #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
  {
    return "x86_64-darwin";
  }
  #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
  {
    return "aarch64-linux";
  }
  #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
  {
    return "x86_64-linux";
  }
  #[allow(unreachable_code)]
  "unknown"
}

fn home_dir_os() -> Option<&'static OsString> {
  static HOME_DIR: OnceLock<Option<OsString>> = OnceLock::new();
  HOME_DIR.get_or_init(|| std::env::var_os("HOME")).as_ref()
}

fn get_store_dir() -> &'static str {
  static STORE_DIR: OnceLock<String> = OnceLock::new();
  STORE_DIR
    .get_or_init(|| std::env::var("NIX_STORE_DIR").unwrap_or_else(|_| "/nix/store".to_string()))
    .as_str()
}

fn split_version_components(s: &str) -> Vec<&str> {
  let mut result = Vec::with_capacity(4);
  let mut component_start: Option<usize> = None;
  let mut last_was_digit = None;

  for (idx, ch) in s.char_indices() {
    let is_digit = ch.is_ascii_digit();
    let is_sep = ch == '.' || ch == '-';
    if is_sep {
      if let Some(start) = component_start.take() {
        result.push(&s[start..idx]);
      }
      last_was_digit = None;
    } else if last_was_digit.is_some() && last_was_digit != Some(is_digit) {
      if let Some(start) = component_start {
        result.push(&s[start..idx]);
      }
      component_start = Some(idx);
      last_was_digit = Some(is_digit);
    } else {
      if component_start.is_none() {
        component_start = Some(idx);
      }
      last_was_digit = Some(is_digit);
    }
  }

  if let Some(start) = component_start {
    result.push(&s[start..]);
  }

  result
}

fn parse_drv_name(s: &str) -> (String, String) {
  let mut split_idx = None;
  let bytes = s.as_bytes();
  for idx in (0..bytes.len().saturating_sub(1)).rev() {
    if bytes[idx] == b'-' && bytes[idx + 1].is_ascii_digit() {
      split_idx = Some(idx);
      break;
    }
  }

  match split_idx {
    Some(idx) => (s[..idx].to_string(), s[idx + 1..].to_string()),
    None => (s.to_string(), String::new()),
  }
}

/// Compare a single version component pair, mirroring Nix's
/// `compareVersionComponents` in `nix/src/libutil/strings.cc`.
///
/// Rules (in order):
///   - Both numeric → numeric compare.
///   - One numeric, the other not → numeric is newer (+1 / -1).
///   - Both non-numeric:
///     * equal → 0.
///     * `""` (missing trailing component) is older than any non-empty
///       non-`"pre"` component, but newer than `"pre"` itself.
///     * `"pre"` sorts before any other non-empty non-numeric.
///     * otherwise lexical compare.
///
/// The previous implementation handled `+` and `~` as substring escapes
/// and treated `Err vs Ok` as `string > numeric`, both of which diverged
/// from Nix in opposite directions for `1.2` vs `1.2.0`, `1.0a1` vs
/// `1.0a`, and `1.0` vs `1.0+rev`. See `eval_compare_versions.rs` for
/// regression coverage.
fn compare_version_component(v1: &str, v2: &str) -> i64 {
  use std::cmp::Ordering;
  let n1 = v1.parse::<i64>().ok();
  let n2 = v2.parse::<i64>().ok();
  match (n1, n2) {
    (Some(a), Some(b)) => match a.cmp(&b) {
      Ordering::Less => -1,
      Ordering::Greater => 1,
      Ordering::Equal => 0,
    },
    (Some(_), None) => 1,
    (None, Some(_)) => -1,
    (None, None) => {
      if v1 == v2 {
        0
      } else if v1.is_empty() {
        if v2 == "pre" {
          1
        } else {
          -1
        }
      } else if v2.is_empty() {
        if v1 == "pre" {
          -1
        } else {
          1
        }
      } else if v1 == "pre" {
        -1
      } else if v2 == "pre" {
        1
      } else {
        match v1.cmp(v2) {
          Ordering::Less => -1,
          Ordering::Greater => 1,
          Ordering::Equal => 0,
        }
      }
    }
  }
}

fn compare_versions(v1: &str, v2: &str) -> i64 {
  let parts1 = split_version_components(v1);
  let parts2 = split_version_components(v2);
  let max_len = parts1.len().max(parts2.len());
  for i in 0..max_len {
    let c1 = parts1.get(i).copied().unwrap_or("");
    let c2 = parts2.get(i).copied().unwrap_or("");
    let cmp = compare_version_component(c1, c2);
    if cmp != 0 {
      return cmp;
    }
  }
  0
}

const TO_XML_HEADER: &str = "<?xml version='1.0' encoding='utf-8'?>\n<expr>\n";
const TO_XML_FOOTER: &str = "</expr>\n";
const XML_INDENT_CACHE: [&str; 17] = [
  "",
  "  ",
  "    ",
  "      ",
  "        ",
  "          ",
  "            ",
  "              ",
  "                ",
  "                  ",
  "                    ",
  "                      ",
  "                        ",
  "                          ",
  "                            ",
  "                              ",
  "                                ",
];

fn xml_initial_capacity(value: &Value) -> usize {
  match value {
    Value::Null | Value::Bool(_) | Value::Int(_) | Value::Float(_) => 32,
    Value::String(s) => 32 + s.len(),
    Value::StringContext { text, .. } => 32 + text.len(),
    Value::Path(_) => 96,
    Value::List(items) => 32 + items.len().saturating_mul(32),
    Value::AttrSet(map) => {
      48 + map
        .keys()
        .map(|key| 40usize.saturating_add(key.len()))
        .sum::<usize>()
    }
    Value::Lambda { .. } | Value::BuiltinPartial { .. } | Value::Thunk { .. } => 32,
  }
}

/// Render a `Value` to Nix-compatible XML (`builtins.toXML`).
/// Mirrors nix/src/libexpr/value-to-xml.cc, 2-space indent.
fn write_value_as_xml(value: &Value, out: &mut String, depth: usize) {
  use std::fmt::Write as _;

  match value {
    Value::Null => {
      push_xml_indent(out, depth);
      out.push_str("<null />\n");
    }
    Value::Bool(b) => {
      push_xml_indent(out, depth);
      let _ = writeln!(
        out,
        "<bool value=\"{}\" />",
        if *b { "true" } else { "false" }
      );
    }
    Value::Int(i) => {
      push_xml_indent(out, depth);
      let _ = writeln!(out, "<int value=\"{}\" />", i);
    }
    Value::Float(f) => {
      push_xml_indent(out, depth);
      out.push_str("<float value=\"");
      if f.fract() == 0.0 && f.abs() < 1e15 {
        let _ = write!(out, "{:.6}", f);
      } else {
        let _ = write!(out, "{}", f);
      }
      out.push_str("\" />\n");
    }
    Value::String(s) => {
      push_xml_indent(out, depth);
      let _ = writeln!(out, "<string value=\"{}\" />", xml_escape_attr(s));
    }
    Value::StringContext { text, .. } => {
      push_xml_indent(out, depth);
      let _ = writeln!(out, "<string value=\"{}\" />", xml_escape_attr(text));
    }
    Value::Path(p) => {
      push_xml_indent(out, depth);
      let path_text = p.to_string_lossy();
      let _ = writeln!(
        out,
        "<path value=\"{}\" />",
        xml_escape_attr(path_text.as_ref())
      );
    }
    Value::List(items) => {
      push_xml_indent(out, depth);
      out.push_str("<list>\n");
      for item in items.iter() {
        write_value_as_xml(item, out, depth + 1);
      }
      push_xml_indent(out, depth);
      out.push_str("</list>\n");
    }
    Value::AttrSet(map) => {
      push_xml_indent(out, depth);
      out.push_str("<attrs>\n");
      for (k, v) in map.iter() {
        push_xml_indent(out, depth + 1);
        let _ = writeln!(out, "<attr name=\"{}\">", xml_escape_attr(k));
        write_value_as_xml(v, out, depth + 2);
        push_xml_indent(out, depth + 1);
        out.push_str("</attr>\n");
      }
      push_xml_indent(out, depth);
      out.push_str("</attrs>\n");
    }
    Value::Lambda { .. } | Value::BuiltinPartial { .. } => {
      push_xml_indent(out, depth);
      out.push_str("<function />\n");
    }
    Value::Thunk { .. } => {
      push_xml_indent(out, depth);
      out.push_str("<unevaluated />\n");
    }
  }
}

fn push_xml_indent(out: &mut String, depth: usize) {
  if let Some(indent) = XML_INDENT_CACHE.get(depth) {
    out.push_str(indent);
  } else {
    for _ in 0..depth {
      out.push_str("  ");
    }
  }
}

fn xml_escape_attr(s: &str) -> Cow<'_, str> {
  let Some((first_escape, first_char)) = s.char_indices().find(|(_, c)| xml_attr_needs_escape(*c))
  else {
    return Cow::Borrowed(s);
  };

  let mut out = String::with_capacity(s.len() + 8);
  out.push_str(&s[..first_escape]);
  push_xml_attr_escaped_char(&mut out, first_char);
  for c in s[first_escape + first_char.len_utf8()..].chars() {
    push_xml_attr_escaped_char(&mut out, c);
  }
  Cow::Owned(out)
}

fn xml_attr_needs_escape(c: char) -> bool {
  matches!(c, '<' | '>' | '&' | '"' | '\'')
}

fn push_xml_attr_escaped_char(out: &mut String, c: char) {
  match c {
    '<' => out.push_str("&lt;"),
    '>' => out.push_str("&gt;"),
    '&' => out.push_str("&amp;"),
    '"' => out.push_str("&quot;"),
    '\'' => out.push_str("&apos;"),
    _ => out.push(c),
  }
}

fn path_string_for_pxir(
  path: &std::path::Path,
  workspace_root: Option<&std::path::Path>,
) -> String {
  if let Some(workspace) = workspace_root {
    if let Ok(rel) = path.strip_prefix(workspace) {
      return rel.to_string_lossy().into_owned();
    }
  }
  path.to_string_lossy().into_owned()
}

/// Build a canonical PxIR artifact record for one `.px` owner file.
///
/// Host-only transport metadata for Perf P4 pxmeta manifest. Evaluates the
/// module export surface and dependency graph; semantics are unchanged from
/// ordinary `eval_file_at_path`.
pub fn build_pxir_record_for_path(
  path: &std::path::Path,
  compiler_version: &str,
  workspace_root: Option<&std::path::Path>,
) -> Result<crate::pxir::PxirArtifactRecord> {
  let canon =
    fs::canonicalize(path).map_err(|e| anyhow!("pxir canonicalize {}: {}", path.display(), e))?;
  let source =
    fs::read_to_string(&canon).map_err(|e| anyhow!("pxir read {}: {}", canon.display(), e))?;
  let source_hash = hash_source_text(&source);
  let metadata =
    fs::metadata(&canon).map_err(|e| anyhow!("pxir metadata {}: {}", canon.display(), e))?;
  let source_len = metadata.len();
  let source_mtime_ns = metadata_mtime_ns(&metadata)
    .ok_or_else(|| anyhow!("pxir missing mtime: {}", canon.display()))?;
  let expr = load_baked_expr_at_path(&canon, &canon)?;
  let graph =
    ensure_import_dependency_graph_cached(&canon, source_len, Some(source_mtime_ns), expr.as_ref())
      .ok_or_else(|| anyhow!("pxir dependency graph unavailable: {}", canon.display()))?;

  let rel_path = path_string_for_pxir(&canon, workspace_root);
  let call_graph: Vec<String> = graph
    .direct_imports
    .iter()
    .map(|import_path| path_string_for_pxir(import_path, workspace_root))
    .collect();

  let value = eval_file_at_path(&canon)?;
  // The record consumes only the TOP-LEVEL attrset keys; WHNF is enough
  // to enumerate them. deep_force here walked every leaf of every
  // registry owner (mirror plates included) and dominated the serve
  // cold boot at ~60s for 24 owners — leaf errors now surface where
  // the leaf is actually used, same as any normal eval.
  let forced = force_value(value)?;
  let symbol_table = match forced {
    Value::AttrSet(map) => {
      let mut keys: Vec<String> = map.keys().cloned().collect();
      keys.sort();
      keys
    }
    _ => Vec::new(),
  };
  let classified = crate::pxir::classify_symbol_tables(&symbol_table);

  Ok(crate::pxir::PxirArtifactRecord {
    rel_path,
    source_hash,
    dependency_hash: graph.dependency_hash,
    symbol_table,
    owner_table: classified.owner_table,
    gate_table: classified.gate_table,
    dispatch_table: classified.dispatch_table,
    receipt_schema_table: classified.receipt_schema_table,
    effect_lock_table: classified.effect_lock_table,
    call_graph,
    compiler_version: compiler_version.to_string(),
    evaluator_version: ast_cache_evaluator_version().to_string(),
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn bounded_cache_eviction_removes_one_entry_without_clearing_all() {
    let mut cache: FxHashMap<String, usize> = FxHashMap::default();
    cache.insert("a".to_string(), 1);
    cache.insert("b".to_string(), 2);
    cache.insert("c".to_string(), 3);

    evict_one_cache_entry(&mut cache, 3);

    assert_eq!(cache.len(), 2);
    assert!(cache
      .values()
      .any(|value| *value == 1 || *value == 2 || *value == 3));
  }

  #[test]
  fn ast_disk_cache_entry_accepts_string_mtime() {
    let raw = r#"{
      "source_mtime_ns": "123",
      "source_path": "/tmp/example.px",
      "expr": { "Int": 7 }
    }"#;
    let entry: AstDiskCacheEntry = serde_json::from_str(raw).expect("cache entry");
    assert_eq!(entry.source_mtime_ns.into_u128(), Some(123));
    assert_eq!(entry.expr, PnixExpr::Int(7));
  }

  #[test]
  fn ast_disk_cache_entry_accepts_legacy_numeric_mtime() {
    let raw = r#"{
      "source_mtime_ns": 42,
      "source_path": "/tmp/example.px",
      "expr": { "Bool": true }
    }"#;
    let entry: AstDiskCacheEntry = serde_json::from_str(raw).expect("cache entry");
    assert_eq!(entry.source_mtime_ns.into_u128(), Some(42));
    assert_eq!(entry.expr, PnixExpr::Bool(true));
  }

  #[test]
  fn source_sha256_hex_matches_known_digest() {
    assert_eq!(
      source_sha256_hex("abc"),
      "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
  }

  #[test]
  fn source_sha256_hex_prefix_32_matches_full_digest_prefix() {
    let full = source_sha256_hex("abc");
    assert_eq!(source_sha256_hex_prefix_32("abc"), full[..32]);
  }

  #[test]
  fn ast_cache_strict_sha_value_parser_matches_process_policy_surface() {
    for value in ["", "0", "false", "FALSE", "False", "off", "OFF", "Off"] {
      assert!(!ast_cache_strict_sha_value_enabled(value), "{value:?}");
    }
    for value in ["1", "true", "on", "yes", "strict", " enabled "] {
      assert!(ast_cache_strict_sha_value_enabled(value), "{value:?}");
    }
  }

  #[test]
  fn ast_disk_cache_entry_accepts_source_hash_identity_metadata() {
    let source_hash = source_sha256_hex("7");
    let raw = format!(
      r#"{{
      "source_mtime_ns": "123",
      "source_len": 1,
      "source_sha256": "{source_hash}",
      "evaluator_version": "{}",
      "feature_flags": "{}",
      "source_path": "/tmp/example.px",
      "expr": {{ "Int": 7 }}
    }}"#,
      ast_cache_evaluator_version(),
      ast_cache_feature_flags()
    );
    let entry: AstDiskCacheEntry = serde_json::from_str(&raw).expect("cache entry");
    assert_eq!(entry.source_mtime_ns.into_u128(), Some(123));
    assert_eq!(entry.source_len, Some(1));
    assert_eq!(entry.source_sha256.as_deref(), Some(source_hash.as_str()));
    assert_eq!(
      entry.evaluator_version.as_deref(),
      Some(ast_cache_evaluator_version())
    );
    assert_eq!(
      entry.feature_flags.as_deref(),
      Some(ast_cache_feature_flags())
    );
    assert_eq!(entry.expr, PnixExpr::Int(7));
  }

  #[test]
  fn ast_binary_cache_entry_accepts_source_hash_identity_metadata() {
    let source_hash = source_sha256_hex("7");
    let entry = AstBinaryCacheEntry {
      source_mtime_ns: 123,
      source_len: 1,
      source_sha256: source_hash.clone(),
      evaluator_version: ast_cache_evaluator_version().to_string(),
      feature_flags: ast_cache_feature_flags().to_string(),
      expr: PnixExpr::Int(7),
    };
    let bytes = serde_json::to_vec(&entry).expect("encode binary cache");
    let decoded: AstBinaryCacheEntry = serde_json::from_slice(&bytes).expect("decode binary cache");
    assert!(binary_cache_identity_matches(
      &decoded,
      1,
      123,
      &source_hash
    ));
    assert_eq!(decoded.expr, PnixExpr::Int(7));
  }

  #[test]
  fn eval_file_reuses_source_hash_verified_disk_ast_cache() {
    let unique = std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .expect("time")
      .as_nanos();
    let dir = std::env::temp_dir().join(format!(
      "pnix-eval-source-hash-disk-cache-{}-{}",
      std::process::id(),
      unique
    ));
    std::fs::create_dir_all(&dir).expect("mkdir");
    let file = dir.join("fixture.px");
    std::fs::write(&file, "41").expect("write fixture");
    let canon = std::fs::canonicalize(&file).expect("canonicalize fixture");
    if let Some(cache_file) = ast_cache_file_for(&canon) {
      let _ = std::fs::remove_file(cache_file);
    }

    init_preparsed_imports(std::collections::HashMap::new());
    reset_eval_perf_stats();
    assert_eq!(
      eval_file_at_path(&file).expect("first eval").to_json(),
      "41"
    );
    let first = take_eval_perf_stats();
    assert_eq!(first.disk_ast_cache_hit_count, 0);
    assert!(first.parse_count >= 1);

    init_preparsed_imports(std::collections::HashMap::new());
    reset_eval_perf_stats();
    assert_eq!(
      eval_file_at_path(&file).expect("second eval").to_json(),
      "41"
    );
    let second = take_eval_perf_stats();
    assert_eq!(second.disk_ast_cache_hit_count, 1);
    assert_eq!(second.disk_ast_binary_cache_hit_count, 1);
    assert_eq!(second.disk_ast_json_cache_hit_count, 0);
    assert_eq!(second.parse_count, 0);
    assert_eq!(second.file_read_count, 0);
    assert_eq!(second.source_hash_count, 0);
    assert_eq!(second.ast_cache_fast_header_hit_count, 1);
    assert_eq!(second.source_read_skipped_by_ast_cache_hit_count, 1);

    if let Some(cache_file) = ast_cache_file_for(&canon) {
      let _ = std::fs::remove_file(cache_file);
    }
    if let Some(cache_file) = ast_binary_cache_file_for(&canon) {
      let _ = std::fs::remove_file(cache_file);
    }
    let _ = std::fs::remove_file(&file);
    let _ = std::fs::remove_dir(&dir);
  }

  #[test]
  fn eval_file_reuses_static_import_value_cache() {
    let unique = std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .expect("time")
      .as_nanos();
    let dir = std::env::temp_dir().join(format!(
      "pnix-eval-import-value-cache-{}-{}",
      std::process::id(),
      unique
    ));
    std::fs::create_dir_all(&dir).expect("mkdir");
    let file = dir.join("fixture.px");
    std::fs::write(&file, "{ x = 41; y = [1 2 3]; }").expect("write fixture");

    init_preparsed_imports(std::collections::HashMap::new());
    reset_eval_perf_stats();
    let first = eval_file_at_path(&file).expect("first eval");
    assert_eq!(first.to_json(), r#"{"x":41,"y":[1,2,3]}"#);
    let first_stats = take_eval_perf_stats();
    assert_eq!(first_stats.import_value_cache_hit_count, 0);
    assert_eq!(first_stats.import_value_cache_store_count, 1);
    assert_eq!(first_stats.import_value_cache_uncacheable_count, 0);

    reset_eval_perf_stats();
    let second = eval_file_at_path(&file).expect("second eval");
    assert_eq!(second.to_json(), first.to_json());
    let second_stats = take_eval_perf_stats();
    assert_eq!(second_stats.import_value_cache_hit_count, 1);
    assert_eq!(second_stats.import_value_cache_store_count, 0);
    assert_eq!(second_stats.file_read_count, 0);
    assert_eq!(second_stats.parse_count, 0);

    let _ = std::fs::remove_file(&file);
    let _ = std::fs::remove_dir(&dir);
  }

  #[test]
  fn eval_file_reuses_static_import_value_cache_with_static_imports() {
    let unique = std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .expect("time")
      .as_nanos();
    let dir = std::env::temp_dir().join(format!(
      "pnix-eval-import-value-cache-static-import-{}-{}",
      std::process::id(),
      unique
    ));
    std::fs::create_dir_all(&dir).expect("mkdir");
    let child = dir.join("child.px");
    let parent = dir.join("parent.px");
    std::fs::write(&child, "{ answer = 42; }").expect("write child");
    std::fs::write(&parent, "import ./child.px").expect("write parent");

    init_preparsed_imports(std::collections::HashMap::new());
    reset_eval_perf_stats();
    let first = eval_file_at_path(&parent).expect("first eval");
    assert_eq!(first.to_json(), r#"{"answer":42}"#);
    let first_stats = take_eval_perf_stats();
    assert_eq!(first_stats.import_value_cache_hit_count, 0);
    assert_eq!(first_stats.import_value_cache_store_count, 2);
    assert_eq!(first_stats.import_value_cache_uncacheable_count, 0);
    assert_eq!(first_stats.import_dependency_static_count, 1);

    reset_eval_perf_stats();
    let second = eval_file_at_path(&parent).expect("second eval");
    assert_eq!(second.to_json(), first.to_json());
    let second_stats = take_eval_perf_stats();
    assert_eq!(second_stats.import_value_cache_hit_count, 1);
    assert_eq!(second_stats.import_value_cache_store_count, 0);
    assert_eq!(second_stats.file_read_count, 0);
    assert_eq!(second_stats.parse_count, 0);

    let _ = std::fs::remove_file(&parent);
    let _ = std::fs::remove_file(&child);
    let _ = std::fs::remove_dir(&dir);
  }

  #[test]
  fn eval_file_dependency_graph_preserves_b_cache_when_sibling_c_changes() {
    let unique = std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .expect("time")
      .as_nanos();
    let dir = std::env::temp_dir().join(format!(
      "pnix-eval-import-dependency-graph-sibling-{}-{}",
      std::process::id(),
      unique
    ));
    std::fs::create_dir_all(&dir).expect("mkdir");
    let b = dir.join("b.px");
    let c = dir.join("c.px");
    let root = dir.join("root.px");
    std::fs::write(&b, "{ x = 1; }").expect("write b");
    std::fs::write(&c, "{ y = 2; }").expect("write c");
    std::fs::write(
      &root,
      "let b = import ./b.px; c = import ./c.px; in { bx = b.x; cy = c.y; }",
    )
    .expect("write root");

    init_preparsed_imports(std::collections::HashMap::new());
    reset_eval_perf_stats();
    let first = eval_file_at_path(&root).expect("first eval");
    assert_eq!(first.to_json(), r#"{"bx":1,"cy":2}"#);
    let first_stats = take_eval_perf_stats();
    assert_eq!(first_stats.dependency_graph_cache_store_count, 3);
    assert_eq!(first_stats.import_value_cache_store_count, 3);

    std::fs::write(&c, "{ y = 99; }").expect("rewrite c");
    reset_eval_perf_stats();
    let second = eval_file_at_path(&root).expect("second eval");
    assert_eq!(second.to_json(), r#"{"bx":1,"cy":99}"#);
    let second_stats = take_eval_perf_stats();
    assert!(
      second_stats.dependency_graph_cache_hit_count >= 1,
      "unchanged b.px dependency graph should hit cache, got {:?}",
      second_stats
    );

    reset_eval_perf_stats();
    let b_only = eval_file_at_path(&b).expect("b-only eval");
    assert_eq!(b_only.to_json(), r#"{"x":1}"#);
    let b_stats = take_eval_perf_stats();
    assert_eq!(b_stats.dependency_graph_cache_hit_count, 1);
    assert_eq!(b_stats.import_value_cache_hit_count, 1);
    assert_eq!(b_stats.parse_count, 0);
    assert_eq!(b_stats.file_read_count, 0);

    let _ = std::fs::remove_file(&root);
    let _ = std::fs::remove_file(&b);
    let _ = std::fs::remove_file(&c);
    let _ = std::fs::remove_dir(&dir);
  }

  #[test]
  fn eval_file_does_not_cache_readfile_import_value() {
    let unique = std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .expect("time")
      .as_nanos();
    let dir = std::env::temp_dir().join(format!(
      "pnix-eval-import-value-cache-readfile-{}-{}",
      std::process::id(),
      unique
    ));
    std::fs::create_dir_all(&dir).expect("mkdir");
    let data = dir.join("data.txt");
    let file = dir.join("fixture.px");
    std::fs::write(&data, "alpha").expect("write data");
    std::fs::write(&file, "builtins.readFile ./data.txt").expect("write fixture");

    init_preparsed_imports(std::collections::HashMap::new());
    reset_eval_perf_stats();
    let first = eval_file_at_path(&file).expect("first eval");
    assert_eq!(first.as_str(), Some("alpha"));
    let first_stats = take_eval_perf_stats();
    assert_eq!(first_stats.import_value_cache_hit_count, 0);
    assert_eq!(first_stats.import_value_cache_store_count, 0);
    assert_eq!(first_stats.import_value_cache_uncacheable_count, 1);

    std::fs::write(&data, "bravo-longer").expect("rewrite data");
    reset_eval_perf_stats();
    let second = eval_file_at_path(&file).expect("second eval");
    assert_eq!(second.as_str(), Some("bravo-longer"));
    let second_stats = take_eval_perf_stats();
    assert_eq!(second_stats.import_value_cache_hit_count, 0);
    assert_eq!(second_stats.import_value_cache_store_count, 0);
    assert_eq!(second_stats.import_value_cache_uncacheable_count, 1);

    let _ = std::fs::remove_file(&file);
    let _ = std::fs::remove_file(&data);
    let _ = std::fs::remove_dir(&dir);
  }

  #[test]
  fn eval_file_does_not_cache_builtins_alias_readfile_import_value() {
    let unique = std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .expect("time")
      .as_nanos();
    let dir = std::env::temp_dir().join(format!(
      "pnix-eval-import-value-cache-readfile-alias-{}-{}",
      std::process::id(),
      unique
    ));
    std::fs::create_dir_all(&dir).expect("mkdir");
    let data = dir.join("data.txt");
    let file = dir.join("fixture.px");
    std::fs::write(&data, "alpha").expect("write data");
    std::fs::write(&file, "let b = builtins; in b.readFile ./data.txt").expect("write fixture");

    init_preparsed_imports(std::collections::HashMap::new());
    reset_eval_perf_stats();
    let first = eval_file_at_path(&file).expect("first eval");
    assert_eq!(first.as_str(), Some("alpha"));
    let first_stats = take_eval_perf_stats();
    assert_eq!(first_stats.import_value_cache_hit_count, 0);
    assert_eq!(first_stats.import_value_cache_store_count, 0);
    assert_eq!(first_stats.import_value_cache_uncacheable_count, 1);

    std::fs::write(&data, "bravo-longer").expect("rewrite data");
    reset_eval_perf_stats();
    let second = eval_file_at_path(&file).expect("second eval");
    assert_eq!(second.as_str(), Some("bravo-longer"));
    let second_stats = take_eval_perf_stats();
    assert_eq!(second_stats.import_value_cache_hit_count, 0);
    assert_eq!(second_stats.import_value_cache_store_count, 0);
    assert_eq!(second_stats.import_value_cache_uncacheable_count, 1);

    let _ = std::fs::remove_file(&file);
    let _ = std::fs::remove_file(&data);
    let _ = std::fs::remove_dir(&dir);
  }

  #[test]
  fn ast_disk_cache_rejects_same_mtime_wrong_source_hash() {
    let unique = std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .expect("time")
      .as_nanos();
    let dir = std::env::temp_dir().join(format!(
      "pnix-eval-source-hash-stale-cache-{}-{}",
      std::process::id(),
      unique
    ));
    std::fs::create_dir_all(&dir).expect("mkdir");
    let file = dir.join("fixture.px");
    std::fs::write(&file, "41").expect("write fixture");
    let canon = std::fs::canonicalize(&file).expect("canonicalize fixture");
    let metadata = std::fs::metadata(&file).expect("metadata");
    let mtime_ns = metadata_mtime_ns(&metadata).expect("mtime");
    let cache_file = ast_cache_file_for(&canon).expect("cache file");
    if let Some(binary_cache_file) = ast_binary_cache_file_for(&canon) {
      let _ = std::fs::remove_file(binary_cache_file);
    }
    let stale_payload = serde_json::json!({
      "source_mtime_ns": mtime_ns.to_string(),
      "source_len": metadata.len(),
      "source_sha256": source_sha256_hex("42"),
      "evaluator_version": ast_cache_evaluator_version(),
      "feature_flags": ast_cache_feature_flags(),
      "source_path": canon.to_string_lossy(),
      "expr": { "Int": 41 },
    });
    std::fs::write(&cache_file, stale_payload.to_string()).expect("write stale cache");

    let current_hash = source_sha256_hex("41");
    assert!(matches!(
      try_load_ast_from_disk(&canon, metadata.len(), Some(mtime_ns), &current_hash),
      AstDiskCacheLookup::Stale(AstDiskCacheStaleReason::SourceSha256)
    ));

    let _ = std::fs::remove_file(cache_file);
    if let Some(binary_cache_file) = ast_binary_cache_file_for(&canon) {
      let _ = std::fs::remove_file(binary_cache_file);
    }
    let _ = std::fs::remove_file(&file);
    let _ = std::fs::remove_dir(&dir);
  }

  #[test]
  fn stale_json_ast_cache_records_source_hash_reason() {
    let unique = std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .expect("time")
      .as_nanos();
    let dir = std::env::temp_dir().join(format!(
      "pnix-eval-source-hash-stale-telemetry-{}-{}",
      std::process::id(),
      unique
    ));
    std::fs::create_dir_all(&dir).expect("mkdir");
    let file = dir.join("fixture.px");
    std::fs::write(&file, "41").expect("write fixture");
    let canon = std::fs::canonicalize(&file).expect("canonicalize fixture");
    let metadata = std::fs::metadata(&file).expect("metadata");
    let mtime_ns = metadata_mtime_ns(&metadata).expect("mtime");
    let cache_file = ast_cache_file_for(&canon).expect("cache file");
    if let Some(binary_cache_file) = ast_binary_cache_file_for(&canon) {
      let _ = std::fs::remove_file(binary_cache_file);
    }
    let stale_payload = serde_json::json!({
      "source_mtime_ns": mtime_ns.to_string(),
      "source_len": metadata.len(),
      "source_sha256": source_sha256_hex("42"),
      "evaluator_version": ast_cache_evaluator_version(),
      "feature_flags": ast_cache_feature_flags(),
      "source_path": canon.to_string_lossy(),
      "expr": { "Int": 41 },
    });
    std::fs::write(&cache_file, stale_payload.to_string()).expect("write stale cache");

    init_preparsed_imports(std::collections::HashMap::new());
    reset_eval_perf_stats();
    assert_eq!(
      eval_file_at_path(&file)
        .expect("eval with stale cache")
        .to_json(),
      "41"
    );
    let stats = take_eval_perf_stats();
    assert_eq!(stats.disk_ast_cache_stale_count, 1);
    assert_eq!(stats.disk_ast_cache_stale_sha_count, 1);
    assert_eq!(stats.disk_ast_cache_stale_mtime_count, 0);
    assert_eq!(stats.disk_ast_cache_stale_len_count, 0);
    assert_eq!(stats.disk_ast_cache_stale_evaluator_version_count, 0);
    assert_eq!(stats.disk_ast_cache_stale_feature_flags_count, 0);
    assert_eq!(stats.parse_count, 1);

    let _ = std::fs::remove_file(cache_file);
    if let Some(binary_cache_file) = ast_binary_cache_file_for(&canon) {
      let _ = std::fs::remove_file(binary_cache_file);
    }
    let _ = std::fs::remove_file(&file);
    let _ = std::fs::remove_dir(&dir);
  }

  #[test]
  fn deep_force_telemetry_counts_nested_values() {
    let mut attrs = BTreeMap::new();
    attrs.insert(
      "items".to_string(),
      Value::List(Arc::new(vec![Value::Int(1), Value::Bool(true)])),
    );
    reset_eval_perf_stats();
    let forced = deep_force(Value::AttrSet(Arc::new(attrs))).expect("deep force");
    assert!(matches!(forced, Value::AttrSet(_)));
    let stats = take_eval_perf_stats();
    assert!(stats.force_node_count >= 4, "{stats:?}");
    assert_eq!(stats.force_attr_count, 1);
    assert_eq!(stats.force_list_count, 1);
    assert_eq!(stats.force_thunk_count, 0);
    assert!(stats.force_max_depth >= 2, "{stats:?}");
  }

  #[test]
  fn deep_force_leaf_fast_path_keeps_telemetry() {
    reset_eval_perf_stats();
    let forced = deep_force(Value::Int(7)).expect("deep force leaf");
    assert!(matches!(forced, Value::Int(7)));
    let stats = take_eval_perf_stats();
    assert_eq!(stats.force_node_count, 1);
    assert_eq!(stats.force_attr_count, 0);
    assert_eq!(stats.force_list_count, 0);
    assert_eq!(stats.force_thunk_count, 0);
    assert_eq!(stats.force_max_depth, 0);
  }

  #[test]
  fn deep_force_value_telemetry_counts_deepseq_values() {
    let value = Value::List(Arc::new(vec![Value::AttrSet(Arc::new(BTreeMap::new()))]));
    reset_eval_perf_stats();
    deep_force_value(&value).expect("deep force value");
    let stats = take_eval_perf_stats();
    assert!(stats.force_node_count >= 2, "{stats:?}");
    assert_eq!(stats.force_attr_count, 1);
    assert_eq!(stats.force_list_count, 1);
    assert_eq!(stats.force_thunk_count, 0);
    assert!(stats.force_max_depth >= 1, "{stats:?}");
  }

  #[test]
  fn deep_force_value_leaf_fast_path_keeps_telemetry() {
    reset_eval_perf_stats();
    deep_force_value(&Value::String("ok".to_string())).expect("deep force value leaf");
    let stats = take_eval_perf_stats();
    assert_eq!(stats.force_node_count, 1);
    assert_eq!(stats.force_attr_count, 0);
    assert_eq!(stats.force_list_count, 0);
    assert_eq!(stats.force_thunk_count, 0);
    assert_eq!(stats.force_max_depth, 0);
  }

  #[test]
  fn check_json_finite_leaf_fast_path_preserves_non_finite_error() {
    check_json_finite(&Value::Int(7), "builtins.toJSON").expect("finite int");
    let err = check_json_finite(&Value::Float(f64::NAN), "builtins.toJSON")
      .expect_err("nan should be rejected");
    assert_eq!(
      err.to_string(),
      "builtins.toJSON: cannot serialize float NaN as JSON"
    );
    let err = check_json_finite(
      &Value::BuiltinPartial {
        name: Arc::from("map"),
        args: Vec::new(),
      },
      "builtins.toJSON",
    )
    .expect_err("function should be rejected");
    assert_eq!(
      err.to_string(),
      "builtins.toJSON: cannot serialize function as JSON"
    );
    let err = check_json_finite(
      &Value::List(Arc::new(vec![Value::Int(1), Value::Float(f64::INFINITY)])),
      "builtins.toJSON",
    )
    .expect_err("nested non-finite float should be rejected");
    assert_eq!(
      err.to_string(),
      "builtins.toJSON: cannot serialize float +inf as JSON"
    );
    let mut nested = BTreeMap::new();
    nested.insert(
      "f".to_string(),
      Value::BuiltinPartial {
        name: Arc::from("map"),
        args: Vec::new(),
      },
    );
    let err = check_json_finite(&Value::AttrSet(Arc::new(nested)), "builtins.toJSON")
      .expect_err("nested function should be rejected");
    assert_eq!(
      err.to_string(),
      "builtins.toJSON: cannot serialize function as JSON"
    );
  }

  #[test]
  fn to_json_finite_check_collects_contexts_and_preserves_rejection() {
    let mut context = BTreeSet::new();
    context.insert("ctx-a".to_string());
    let context_string = Value::StringContext {
      text: "x".to_string(),
      context,
    };
    let result = apply_builtin(
      "toJSON",
      &[Value::List(Arc::new(vec![
        context_string.clone(),
        Value::Int(1),
      ]))],
    )
    .expect("toJSON context value");
    match result {
      Value::StringContext { text, context } => {
        assert_eq!(text, "[\"x\",1]");
        assert!(context.contains("ctx-a"));
      }
      other => panic!("expected context-bearing JSON string, got {other:?}"),
    }

    let err = check_json_finite_and_collect_contexts(
      &Value::List(Arc::new(vec![
        context_string,
        Value::BuiltinPartial {
          name: Arc::from("map"),
          args: Vec::new(),
        },
      ])),
      "builtins.toJSON",
    )
    .expect_err("function should still be rejected while collecting contexts");
    assert_eq!(
      err.to_string(),
      "builtins.toJSON: cannot serialize function as JSON"
    );
  }

  #[test]
  fn cat_attrs_borrows_non_thunk_attrsets_preserves_surface() {
    let mut first = BTreeMap::new();
    first.insert("x".to_string(), Value::Int(1));
    first.insert(
      "ignored".to_string(),
      Value::List(Arc::new(vec![Value::Int(9)])),
    );
    let mut second = BTreeMap::new();
    second.insert("x".to_string(), Value::Int(2));
    let third = BTreeMap::new();

    let result = apply_builtin(
      "catAttrs",
      &[
        Value::String("x".to_string()),
        Value::List(Arc::new(vec![
          Value::AttrSet(Arc::new(first)),
          Value::AttrSet(Arc::new(second)),
          Value::AttrSet(Arc::new(third)),
        ])),
      ],
    )
    .expect("catAttrs attrsets");
    assert_eq!(result.to_json(), "[1,2]");

    let err = apply_builtin(
      "catAttrs",
      &[
        Value::String("x".to_string()),
        Value::List(Arc::new(vec![Value::Int(1)])),
      ],
    )
    .expect_err("non-attrset element should still fail");
    assert_eq!(
      err.to_string(),
      "builtins.catAttrs: list element must be attrset, got 1"
    );
  }

  #[test]
  fn group_by_preserves_lazy_elements_when_key_function_ignores_argument() {
    let grouped_keys = crate::eval_expr(
      r#"builtins.attrNames (builtins.groupBy (item: "k") [(builtins.throw "boom")])"#,
    )
    .expect("groupBy key function ignores thrown element");
    assert_eq!(grouped_keys.to_json(), r#"["k"]"#);

    let forced_group_item = crate::eval_expr(
      r#"
        let grouped = builtins.groupBy (item: "k") [(builtins.throw "boom")];
        in builtins.head grouped.k
      "#,
    )
    .expect_err("grouped element should remain lazy until selected");
    assert!(
      forced_group_item.to_string().contains("boom"),
      "got: {forced_group_item}"
    );
  }

  #[test]
  fn sort_preserves_lazy_elements_when_comparator_ignores_arguments() {
    let sorted_len = crate::eval_expr(
      r#"builtins.length (builtins.sort (a: b: false) [(builtins.throw "left") (builtins.throw "right")])"#,
    )
    .expect("sort comparator ignores thrown elements");
    assert_eq!(sorted_len.to_json(), "2");

    let forced_sorted_item = crate::eval_expr(
      r#"
        let sorted = builtins.sort (a: b: false) [(builtins.throw "left") (builtins.throw "right")];
        in builtins.head sorted
      "#,
    )
    .expect_err("sorted element should remain lazy until selected");
    let msg = forced_sorted_item.to_string();
    assert!(msg.contains("left") || msg.contains("right"), "got: {msg}");
  }

  #[test]
  fn concat_map_forces_only_returned_list_spine() {
    let mapped = crate::eval_expr(r#"builtins.concatMap (x: let y = [x]; in y) [1 2]"#)
      .expect("concatMap should accept a thunk that resolves to a list");
    assert_eq!(mapped.to_json(), "[1,2]");

    let mapped_len = crate::eval_expr(
      r#"builtins.length (builtins.concatMap (x: let y = [(builtins.throw "lazy")]; in y) [1])"#,
    )
    .expect("concatMap should not force elements inside returned lists");
    assert_eq!(mapped_len.to_json(), "1");

    let forced_item = crate::eval_expr(
      r#"
        let mapped = builtins.concatMap (x: let y = [(builtins.throw "lazy")]; in y) [1];
        in builtins.head mapped
      "#,
    )
    .expect_err("concatMap result element should remain lazy until selected");
    assert!(
      forced_item.to_string().contains("lazy"),
      "got: {forced_item}"
    );
  }

  #[test]
  fn concat_string_builtins_force_string_elements_shallowly() {
    let concat = crate::eval_expr(
      r#"
        let first = "a"; second = "b";
        in builtins.concatStrings [first second]
      "#,
    )
    .expect("concatStrings should accept thunks that resolve to strings");
    assert_eq!(concat.to_json(), "\"ab\"");

    let concat_sep = crate::eval_expr(
      r#"
        let sep = "-"; first = "a"; second = "b";
        in builtins.concatStringsSep sep [first second]
      "#,
    )
    .expect("concatStringsSep should accept thunks that resolve to strings");
    assert_eq!(concat_sep.to_json(), "\"a-b\"");

    let thrown = crate::eval_expr(
      r#"
        let bad = builtins.throw "string element";
        in builtins.concatStrings ["a" bad]
      "#,
    )
    .expect_err("concatStrings must still force required string elements");
    assert!(thrown.to_string().contains("string element"));
  }

  #[test]
  fn strict_name_builtins_borrow_non_thunk_inputs_preserve_surface() {
    let mut nested_inner = BTreeMap::new();
    nested_inner.insert("b".to_string(), Value::Int(3));
    let mut nested_outer = BTreeMap::new();
    nested_outer.insert("a".to_string(), Value::AttrSet(Arc::new(nested_inner)));
    let attr_by_path = apply_builtin(
      "attrByPath",
      &[
        Value::List(Arc::new(vec![
          Value::String("a".to_string()),
          Value::String("b".to_string()),
        ])),
        Value::Int(0),
        Value::AttrSet(Arc::new(nested_outer)),
      ],
    )
    .expect("attrByPath nested attr");
    assert_eq!(attr_by_path.to_json(), "3");

    let mut attrs = BTreeMap::new();
    attrs.insert("keep".to_string(), Value::Int(1));
    attrs.insert("drop".to_string(), Value::Int(2));
    let get_attrs = apply_builtin(
      "getAttrs",
      &[
        Value::List(Arc::new(vec![Value::String("keep".to_string())])),
        Value::AttrSet(Arc::new(attrs)),
      ],
    )
    .expect("getAttrs subset");
    assert_eq!(get_attrs.to_json(), "{\"keep\":1}");

    let concat_lists = apply_builtin(
      "concatLists",
      &[Value::List(Arc::new(vec![
        Value::List(Arc::new(vec![Value::Int(1)])),
        Value::List(Arc::new(vec![Value::Int(2), Value::Int(3)])),
      ]))],
    )
    .expect("concatLists nested lists");
    assert_eq!(concat_lists.to_json(), "[1,2,3]");

    let replaced = apply_builtin(
      "replaceStrings",
      &[
        Value::List(Arc::new(vec![Value::String("a".to_string())])),
        Value::List(Arc::new(vec![Value::String("b".to_string())])),
        Value::String("a-a".to_string()),
      ],
    )
    .expect("replaceStrings");
    assert_eq!(replaced.to_json(), "\"b-b\"");

    let mut entry = BTreeMap::new();
    entry.insert("name".to_string(), Value::String("x".to_string()));
    entry.insert("value".to_string(), Value::Int(4));
    let list_to_attrs = apply_builtin(
      "listToAttrs",
      &[Value::List(Arc::new(vec![Value::AttrSet(Arc::new(entry))]))],
    )
    .expect("listToAttrs");
    assert_eq!(list_to_attrs.to_json(), "{\"x\":4}");

    let mut remove_input = BTreeMap::new();
    remove_input.insert("x".to_string(), Value::Int(1));
    remove_input.insert("y".to_string(), Value::Int(2));
    let removed = apply_builtin(
      "removeAttrs",
      &[
        Value::AttrSet(Arc::new(remove_input)),
        Value::List(Arc::new(vec![Value::String("x".to_string())])),
      ],
    )
    .expect("removeAttrs");
    assert_eq!(removed.to_json(), "{\"y\":2}");
  }

  #[test]
  fn shallow_builtin_arg_forcing_preserves_non_thunk_surface() {
    let args = vec![
      Value::String("x".to_string()),
      Value::List(Arc::new(vec![Value::Int(1), Value::Int(2)])),
    ];
    let forced = force_builtin_args(&args, false).expect("shallow force args");
    assert_eq!(forced[0].to_json(), "\"x\"");
    assert_eq!(forced[1].to_json(), "[1,2]");

    let length = apply_builtin("length", &[forced[1].clone()]).expect("length list");
    assert_eq!(length.to_json(), "2");
  }

  #[test]
  fn fold_builtins_borrow_non_thunk_lists_preserve_surface() {
    let add = builtin_partial_value("add");
    let sum_left = apply_builtin(
      "foldl'",
      &[
        add.clone(),
        Value::Int(0),
        Value::List(Arc::new(vec![Value::Int(1), Value::Int(2), Value::Int(3)])),
      ],
    )
    .expect("foldl sum");
    assert_eq!(sum_left.to_json(), "6");

    let sum_right = apply_builtin(
      "foldr",
      &[
        add,
        Value::Int(0),
        Value::List(Arc::new(vec![Value::Int(1), Value::Int(2), Value::Int(3)])),
      ],
    )
    .expect("foldr sum");
    assert_eq!(sum_right.to_json(), "6");

    let empty = apply_builtin(
      "foldr",
      &[
        builtin_partial_value("add"),
        Value::String("init".to_string()),
        Value::List(Arc::new(Vec::new())),
      ],
    )
    .expect("foldr empty");
    assert_eq!(empty.to_json(), "\"init\"");
  }

  #[test]
  fn fold_preserves_lazy_inputs_when_function_ignores_them() {
    let ignores_item = crate::eval_expr(
      r#"
        builtins.fold (acc: item: acc) 7 [(builtins.throw "unused item")]
      "#,
    )
    .expect("fold should not force list elements that the function ignores");
    assert_eq!(ignores_item.to_json(), "7");

    let ignores_acc = crate::eval_expr(
      r#"
        builtins.fold (acc: item: item) (builtins.throw "unused accumulator") [9]
      "#,
    )
    .expect("fold should not force the accumulator before the function needs it");
    assert_eq!(ignores_acc.to_json(), "9");
  }

  #[test]
  fn shape_only_builtins_borrow_non_thunk_inputs_preserve_surface() {
    let list = Value::List(Arc::new(vec![Value::Int(1), Value::Int(2), Value::Int(3)]));
    let length = apply_builtin("length", std::slice::from_ref(&list)).expect("length list");
    assert_eq!(length.to_json(), "3");
    let is_list = apply_builtin("isList", std::slice::from_ref(&list)).expect("isList");
    assert_eq!(is_list.to_json(), "true");
    let type_of_list = apply_builtin("typeOf", std::slice::from_ref(&list)).expect("typeOf list");
    assert_eq!(type_of_list.to_json(), "\"list\"");

    let mut attrs = BTreeMap::new();
    attrs.insert("x".to_string(), Value::Int(1));
    let attrset = Value::AttrSet(Arc::new(attrs));
    let is_attrs = apply_builtin("isAttrs", std::slice::from_ref(&attrset)).expect("isAttrs");
    assert_eq!(is_attrs.to_json(), "true");
    let type_of_attrs =
      apply_builtin("typeOf", std::slice::from_ref(&attrset)).expect("typeOf attrs");
    assert_eq!(type_of_attrs.to_json(), "\"set\"");

    let is_finite = apply_builtin("isFinite", &[Value::Float(1.25)]).expect("isFinite");
    assert_eq!(is_finite.to_json(), "true");
  }

  #[test]
  fn list_accessor_builtins_borrow_non_thunk_lists_preserve_surface() {
    let list = Value::List(Arc::new(vec![Value::Int(1), Value::Int(2), Value::Int(3)]));

    let head = apply_builtin("head", std::slice::from_ref(&list)).expect("head list");
    assert_eq!(head.to_json(), "1");

    let tail = apply_builtin("tail", std::slice::from_ref(&list)).expect("tail list");
    assert_eq!(tail.to_json(), "[2,3]");

    let elem = apply_builtin("elemAt", &[list, Value::Int(2)]).expect("elemAt list");
    assert_eq!(elem.to_json(), "3");
  }

  #[test]
  fn list_partition_builtins_borrow_non_thunk_lists_preserve_surface() {
    let list = Value::List(Arc::new(vec![Value::Int(1), Value::Int(2), Value::Int(3)]));

    let dropped = apply_builtin("drop", &[Value::Int(1), list.clone()]).expect("drop list");
    assert_eq!(dropped.to_json(), "[2,3]");

    let taken = apply_builtin("take", &[Value::Int(2), list]).expect("take list");
    assert_eq!(taken.to_json(), "[1,2]");
  }

  #[test]
  fn list_partition_builtins_preserve_lazy_elements() {
    let taken =
      crate::eval_expr(r#"builtins.length (builtins.take 1 [1 (builtins.throw "boom")])"#)
        .expect("take length should not force dropped element");
    assert_eq!(taken.to_json(), "1");

    let dropped =
      crate::eval_expr(r#"builtins.length (builtins.drop 1 [(builtins.throw "boom") 2])"#)
        .expect("drop length should not force dropped element");
    assert_eq!(dropped.to_json(), "1");
  }

  #[test]
  fn list_constructor_builtins_borrow_non_thunk_lists_preserve_surface() {
    let lhs = Value::List(Arc::new(vec![Value::Int(1), Value::Int(2)]));
    let rhs = Value::List(Arc::new(vec![Value::Int(3)]));

    let appended = apply_builtin("append", &[lhs.clone(), rhs.clone()]).expect("append");
    assert_eq!(appended.to_json(), "[1,2,3]");

    let consed = apply_builtin("cons", &[Value::Int(0), lhs.clone()]).expect("cons");
    assert_eq!(consed.to_json(), "[0,1,2]");

    let reversed = apply_builtin("reverse", std::slice::from_ref(&lhs)).expect("reverse");
    assert_eq!(reversed.to_json(), "[2,1]");

    let zipped = apply_builtin("zip", &[lhs, rhs]).expect("zip");
    assert_eq!(zipped.to_json(), "[[1,3]]");
  }

  #[test]
  fn list_constructor_builtins_preserve_lazy_elements() {
    let appended =
      crate::eval_expr(r#"builtins.length (builtins.append [1] [(builtins.throw "boom")])"#)
        .expect("append length should not force appended element");
    assert_eq!(appended.to_json(), "2");

    let consed = crate::eval_expr(r#"builtins.length (builtins.cons (builtins.throw "boom") [1])"#)
      .expect("cons length should not force head element");
    assert_eq!(consed.to_json(), "2");

    let reversed =
      crate::eval_expr(r#"builtins.length (builtins.reverse [(builtins.throw "boom") 1])"#)
        .expect("reverse length should not force element");
    assert_eq!(reversed.to_json(), "2");

    let zipped =
      crate::eval_expr(r#"builtins.length (builtins.zip [(builtins.throw "boom")] [1])"#)
        .expect("zip length should not force paired element");
    assert_eq!(zipped.to_json(), "1");
  }

  #[test]
  fn attrset_accessor_builtins_borrow_non_thunk_attrsets_preserve_surface() {
    let mut attrs = BTreeMap::new();
    attrs.insert("a".to_string(), Value::Int(1));
    attrs.insert("b".to_string(), Value::Int(2));
    let attrset = Value::AttrSet(Arc::new(attrs));

    let names = apply_builtin("attrNames", std::slice::from_ref(&attrset)).expect("attrNames");
    assert_eq!(names.to_json(), "[\"a\",\"b\"]");

    let values = apply_builtin("attrValues", std::slice::from_ref(&attrset)).expect("attrValues");
    assert_eq!(values.to_json(), "[1,2]");

    let has = apply_builtin(
      "hasAttr",
      &[Value::String("a".to_string()), attrset.clone()],
    )
    .expect("hasAttr");
    assert_eq!(has.to_json(), "true");

    let got =
      apply_builtin("getAttr", &[Value::String("b".to_string()), attrset]).expect("getAttr");
    assert_eq!(got.to_json(), "2");
  }

  #[test]
  fn get_builtins_borrow_non_thunk_attrsets_preserve_surface() {
    let mut attrs = BTreeMap::new();
    attrs.insert("a".to_string(), Value::Int(1));
    attrs.insert("b".to_string(), Value::Int(2));
    let attrset = Value::AttrSet(Arc::new(attrs));

    let got =
      apply_builtin("get", &[attrset.clone(), Value::String("a".to_string())]).expect("get");
    assert_eq!(got.to_json(), "1");

    let missing = apply_builtin("get", &[attrset.clone(), Value::String("z".to_string())])
      .expect("get missing");
    assert_eq!(missing.to_json(), "null");

    let alias =
      apply_builtin("mapGet", &[attrset, Value::String("b".to_string())]).expect("mapGet");
    assert_eq!(alias.to_json(), "2");
  }

  #[test]
  fn set_builtins_borrow_non_thunk_attrsets_preserve_surface() {
    let mut attrs = BTreeMap::new();
    attrs.insert("a".to_string(), Value::Int(1));
    let attrset = Value::AttrSet(Arc::new(attrs));

    let set = apply_builtin(
      "set",
      &[
        attrset.clone(),
        Value::String("b".to_string()),
        Value::Int(2),
      ],
    )
    .expect("set");
    assert_eq!(set.to_json(), "{\"a\":1,\"b\":2}");

    let alias = apply_builtin(
      "mapSet",
      &[attrset, Value::String("a".to_string()), Value::Int(3)],
    )
    .expect("mapSet");
    assert_eq!(alias.to_json(), "{\"a\":3}");
  }

  #[test]
  fn merge_builtins_borrow_non_thunk_attrsets_preserve_surface() {
    let mut lhs = BTreeMap::new();
    lhs.insert("a".to_string(), Value::Int(1));
    lhs.insert("shared".to_string(), Value::Int(2));

    let mut rhs = BTreeMap::new();
    rhs.insert("b".to_string(), Value::Int(3));
    rhs.insert("shared".to_string(), Value::Int(4));

    let merged = apply_builtin(
      "merge",
      &[
        Value::AttrSet(Arc::new(lhs.clone())),
        Value::AttrSet(Arc::new(rhs.clone())),
      ],
    )
    .expect("merge");
    assert_eq!(merged.to_json(), "{\"a\":1,\"b\":3,\"shared\":4}");

    let alias = apply_builtin(
      "mapMerge",
      &[Value::AttrSet(Arc::new(lhs)), Value::AttrSet(Arc::new(rhs))],
    )
    .expect("mapMerge");
    assert_eq!(alias.to_json(), "{\"a\":1,\"b\":3,\"shared\":4}");
  }

  #[test]
  fn attrset_subset_builtins_borrow_non_thunk_inputs_preserve_surface() {
    let mut attrs = BTreeMap::new();
    attrs.insert("a".to_string(), Value::Int(1));
    attrs.insert("b".to_string(), Value::Int(2));
    attrs.insert("c".to_string(), Value::Int(3));
    let attrset = Value::AttrSet(Arc::new(attrs));

    let picked = apply_builtin(
      "getAttrs",
      &[
        Value::List(Arc::new(vec![Value::String("b".to_string())])),
        attrset.clone(),
      ],
    )
    .expect("getAttrs");
    assert_eq!(picked.to_json(), "{\"b\":2}");

    let removed = apply_builtin(
      "removeAttrs",
      &[
        attrset.clone(),
        Value::List(Arc::new(vec![Value::String("a".to_string())])),
      ],
    )
    .expect("removeAttrs");
    assert_eq!(removed.to_json(), "{\"b\":2,\"c\":3}");

    let mut filter = BTreeMap::new();
    filter.insert("a".to_string(), Value::Int(0));
    filter.insert("c".to_string(), Value::Int(0));
    let intersected = apply_builtin(
      "intersectAttrs",
      &[Value::AttrSet(Arc::new(filter)), attrset],
    )
    .expect("intersectAttrs");
    assert_eq!(intersected.to_json(), "{\"a\":1,\"c\":3}");
  }

  #[test]
  fn compare_values_borrows_non_thunk_lists_preserves_order_surface() {
    use std::cmp::Ordering;

    let left = Value::List(Arc::new(vec![
      Value::Int(1),
      Value::String("a".to_string()),
    ]));
    let right = Value::List(Arc::new(vec![
      Value::Int(1),
      Value::String("b".to_string()),
    ]));
    assert_eq!(
      compare_values(&left, &right).expect("compare lists"),
      Ordering::Less
    );

    let mut context = BTreeSet::new();
    context.insert("ctx".to_string());
    let context_string = Value::StringContext {
      text: "b".to_string(),
      context,
    };
    assert_eq!(
      compare_values(&Value::String("a".to_string()), &context_string)
        .expect("compare context string"),
      Ordering::Less
    );
  }

  #[test]
  fn flatten_value_for_builtin_reserves_nested_lists_and_preserves_surface() {
    let value = Value::List(Arc::new(vec![
      Value::List(Arc::new(vec![
        Value::Int(1),
        Value::List(Arc::new(vec![Value::Int(2), Value::Int(3)])),
      ])),
      Value::Int(4),
    ]));
    let mut out = Vec::with_capacity(1);
    flatten_value_for_builtin(&value, &mut out).expect("flatten value");
    assert_eq!(Value::List(Arc::new(out)).to_json(), "[1,2,3,4]");
  }

  #[test]
  fn flatten_forces_only_list_spines_and_preserves_scalar_payloads() {
    let flattened = crate::eval_expr(
      r#"
        let nested = [1 [2]];
        in builtins.flatten [nested 3]
      "#,
    )
    .expect("flatten should force thunk-to-list spines");
    assert_eq!(flattened.to_json(), "[1,2,3]");

    let lazy_attr_payload = crate::eval_expr(
      r#"
        builtins.length (builtins.flatten [{ a = builtins.throw "lazy attr"; }])
      "#,
    )
    .expect("flatten should not deep-force attrset payload values");
    assert_eq!(lazy_attr_payload.to_json(), "1");

    let forced_payload = crate::eval_expr(
      r#"
        let flattened = builtins.flatten [{ a = builtins.throw "lazy attr"; }];
        in (builtins.head flattened).a
      "#,
    )
    .expect_err("flattened attr payload should remain lazy until selected");
    assert!(
      forced_payload.to_string().contains("lazy attr"),
      "got: {forced_payload}"
    );
  }

  #[test]
  fn anchored_regex_pattern_cache_reuses_compiled_key_text() {
    reset_eval_perf_stats();
    let first = anchored_regex_pattern("[a-z]+");
    let second = anchored_regex_pattern("[a-z]+");
    assert_eq!(first.as_ref(), "(?s)^(?:[a-z]+)$");
    assert!(Arc::ptr_eq(&first, &second));

    let stats = take_eval_perf_stats();
    assert_eq!(stats.match_anchored_pattern_cache_miss_count, 1);
    assert_eq!(stats.match_anchored_pattern_cache_hit_count, 1);
    assert!(stats.cache_hit());
    assert!(stats.cache_miss());
  }

  #[test]
  fn match_builtin_partial_arity_fast_path_defers_until_second_arg() {
    reset_eval_perf_stats();
    let func = Value::BuiltinPartial {
      name: Arc::from("match"),
      args: Vec::new(),
    };
    let partial = apply_value(func, Value::String("x".to_string())).expect("partial");
    match &partial {
      Value::BuiltinPartial { name, args } => {
        assert_eq!(name.as_ref(), "match");
        assert_eq!(args.len(), 1);
      }
      _ => panic!("expected deferred match partial"),
    }
    let stats = take_eval_perf_stats();
    assert_eq!(stats.builtin_partial_arity_fast_path_count, 1);

    let matched = apply_value(partial, Value::String("x".to_string())).expect("matched");
    match matched {
      Value::List(items) => assert!(items.is_empty()),
      _ => panic!("expected empty match group list"),
    }
  }

  #[test]
  fn fast_builtin_attr_exists_uses_registry_without_value_materialization() {
    assert!(fast_builtin_attr_exists("currentSystem"));
    assert!(fast_builtin_attr_exists("map"));
    assert!(fast_builtin_attr_exists("scopedImport"));
    assert!(!fast_builtin_attr_exists("definitelyMissingBuiltin"));
  }

  #[test]
  fn fast_builtin_function_names_are_interned() {
    let first = fast_builtin_attr_value("map").expect("first builtin");
    let second = fast_builtin_attr_value("map").expect("second builtin");
    match (first, second) {
      (Value::BuiltinPartial { name: first, .. }, Value::BuiltinPartial { name: second, .. }) => {
        assert!(Arc::ptr_eq(&first, &second))
      }
      (left, right) => panic!("expected builtin partials, got {left:?} and {right:?}"),
    }
  }

  #[test]
  fn store_dir_lookup_reuses_process_local_snapshot() {
    let first = get_store_dir();
    let second = get_store_dir();
    assert!(!first.is_empty());
    assert_eq!(first, second);
    assert_eq!(first.as_ptr(), second.as_ptr());
  }

  #[test]
  fn home_dir_lookup_reuses_process_local_snapshot() {
    let first = home_dir_os().cloned();
    let second = home_dir_os().cloned();
    assert_eq!(first, second);
  }

  #[test]
  fn getenv_allow_list_reuses_process_local_snapshot() {
    let first = getenv_allow_list();
    let second = getenv_allow_list();
    assert_eq!(first.as_ptr(), second.as_ptr());
    assert!(getenv_allowed("PNIX_TEST_ALLOWED_BY_PREFIX"));
    assert!(getenv_allowed("HYPNIX_TEST_ALLOWED_BY_PREFIX"));
  }

  #[test]
  fn verbose_mode_reuses_process_local_snapshot() {
    let first = verbose_mode_enabled();
    let second = verbose_mode_enabled();
    assert_eq!(first, second);
  }

  #[test]
  fn builtins_attrset_and_global_alias_reuse_interned_builtin_names() {
    let interned = fast_builtin_function_name_arc("map").expect("interned map builtin");

    let Value::AttrSet(builtins) = builtins_attrset() else {
      panic!("expected builtins attrset");
    };
    let Some(Value::BuiltinPartial {
      name: from_attrset, ..
    }) = builtins.get("map")
    else {
      panic!("expected map builtin in builtins attrset");
    };
    assert!(Arc::ptr_eq(&interned, from_attrset));

    let alias = eval(&PnixExpr::Var("map".to_string()), &Env::new()).expect("map alias");
    let Value::BuiltinPartial {
      name: from_alias, ..
    } = alias
    else {
      panic!("expected map alias builtin partial");
    };
    assert!(Arc::ptr_eq(&interned, &from_alias));
  }

  #[test]
  fn apply_builtin_partial_fallback_reuses_interned_builtin_names() {
    let interned = fast_builtin_function_name_arc("hashString").expect("interned hashString");
    let partial = apply_builtin("hashString", &[Value::String("sha256".to_string())])
      .expect("partial hashString");

    let Value::BuiltinPartial { name, args } = partial else {
      panic!("expected hashString partial");
    };
    assert!(Arc::ptr_eq(&interned, &name));
    assert_eq!(args.len(), 1);
  }

  #[test]
  fn push_u64_hex_lower_16_preserves_cache_filename_surface() {
    let mut out = String::new();
    push_u64_hex_lower_16(0x000f_abcd_1234_5678, &mut out);
    assert_eq!(out, "000fabcd12345678");
  }

  #[test]
  fn to_xml_writer_preserves_scalar_output() {
    let mut out = String::new();
    write_value_as_xml(&Value::Int(7), &mut out, 1);
    assert_eq!(out, "  <int value=\"7\" />\n");

    out.clear();
    write_value_as_xml(&Value::String("a&b".to_string()), &mut out, 1);
    assert_eq!(out, "  <string value=\"a&amp;b\" />\n");

    out.clear();
    write_value_as_xml(&Value::Float(1.0), &mut out, 1);
    assert_eq!(out, "  <float value=\"1.000000\" />\n");

    out.clear();
    write_value_as_xml(&Value::Float(1.25), &mut out, 1);
    assert_eq!(out, "  <float value=\"1.25\" />\n");

    out.clear();
    write_value_as_xml(
      &Value::Path(std::path::PathBuf::from("/tmp/a&b")),
      &mut out,
      1,
    );
    assert_eq!(out, "  <path value=\"/tmp/a&amp;b\" />\n");
  }

  #[test]
  fn to_xml_writer_preserves_nested_output() {
    let mut attrs = BTreeMap::new();
    attrs.insert(
      "items".to_string(),
      Value::List(Arc::new(vec![Value::Bool(true)])),
    );

    let mut out = String::new();
    write_value_as_xml(&Value::AttrSet(Arc::new(attrs)), &mut out, 1);
    assert_eq!(
      out,
      concat!(
        "  <attrs>\n",
        "    <attr name=\"items\">\n",
        "      <list>\n",
        "        <bool value=\"true\" />\n",
        "      </list>\n",
        "    </attr>\n",
        "  </attrs>\n"
      )
    );
  }

  #[test]
  fn embedded_value_marker_preserves_display_surface() {
    assert_eq!(embedded_value_marker(&Value::Int(7)), "<embedded:7>");
  }

  #[test]
  fn xml_escape_attr_borrows_when_unchanged() {
    assert!(matches!(
      xml_escape_attr("plain-text"),
      Cow::Borrowed("plain-text")
    ));
  }

  #[test]
  fn xml_escape_attr_escapes_only_when_needed() {
    assert!(matches!(
      xml_escape_attr("a<&\"'b"),
      Cow::Owned(ref escaped) if escaped == "a&lt;&amp;&quot;&apos;b"
    ));
  }

  #[test]
  fn replace_strings_empty_patterns_preserve_original_string_value() {
    let mut context = BTreeSet::new();
    context.insert("!out!/tmp/source".to_string());
    let haystack = Value::StringContext {
      text: "unchanged".to_string(),
      context,
    };

    let result = apply_builtin(
      "replaceStrings",
      &[
        Value::List(Arc::new(vec![])),
        Value::List(Arc::new(vec![])),
        haystack,
      ],
    )
    .expect("replaceStrings empty pattern list");

    match result {
      Value::StringContext { text, context } => {
        assert_eq!(text, "unchanged");
        assert_eq!(context.len(), 1);
        assert!(context.contains("!out!/tmp/source"));
      }
      other => panic!("expected original context-bearing string, got {other:?}"),
    }
  }
}
