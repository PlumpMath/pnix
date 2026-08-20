//! Hot-reloadable .px resource cache.
//!
//! 모든 .px 데이터 리소스를 파일 수정 시각(mtime) 기반으로 자동 감지하여
//! 재시작 없이 실시간 반영한다.
//!
//! `Arc<T>`를 반환하므로 clone 비용이 없고, 참조 유지도 안전하다.

use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

/// File mtime 기반 hot-reloadable .px cache.
/// 매 접근 시 stat()으로 mtime을 체크하고, 변경되었으면 재로드한다.
pub struct HotPxCache<T> {
  inner: RwLock<Option<CacheEntry<T>>>,
}

struct CacheEntry<T> {
  value: Arc<T>,
  path: PathBuf,
  file_mtime: Option<SystemTime>,
  last_stat: std::time::Instant,
}

/// stat() 재확인 간격. 이 시간 내에는 cached mtime을 신뢰.
const STAT_RECHECK_MS: u128 = 500;

impl<T> HotPxCache<T> {
  pub const fn new() -> Self {
    Self {
      inner: RwLock::new(None),
    }
  }
}

impl<T: Send + Sync> HotPxCache<T> {
  /// 캐시에서 값을 가져온다. 파일이 변경되었으면 자동으로 재로드.
  /// `Arc<T>`를 반환하므로 cheap clone + 참조 유지 가능.
  pub fn get(
    &self,
    path_fn: fn() -> PathBuf,
    parse_fn: fn(&crate::px::PxValue) -> T,
    default_fn: fn() -> T,
  ) -> Arc<T> {
    // ultra-fast path: 캐시가 있고 stat 재확인 간격 내면 stat() 자체를 skip
    {
      let guard = self.inner.read().unwrap_or_else(|e| e.into_inner());
      if let Some(entry) = guard.as_ref() {
        if entry.last_stat.elapsed().as_millis() < STAT_RECHECK_MS {
          return Arc::clone(&entry.value);
        }
      }
    }

    let path = path_fn();
    let current_mtime = file_mtime(&path);

    // fast path: read lock + mtime 비교
    {
      let guard = self.inner.read().unwrap_or_else(|e| e.into_inner());
      if let Some(entry) = guard.as_ref() {
        if entry.path == path && entry.file_mtime == current_mtime {
          // mtime 같으면 last_stat 갱신 (write lock 필요하지만 드물게 발생)
          drop(guard);
          if let Ok(mut wg) = self.inner.try_write() {
            if let Some(entry) = wg.as_mut() {
              entry.last_stat = std::time::Instant::now();
            }
          }
          let guard = self.inner.read().unwrap_or_else(|e| e.into_inner());
          if let Some(entry) = guard.as_ref() {
            return Arc::clone(&entry.value);
          }
        }
      }
    }

    // slow path: write lock + reload
    let mut guard = self.inner.write().unwrap_or_else(|e| e.into_inner());

    // double-check
    if let Some(entry) = guard.as_ref() {
      if entry.path == path && entry.file_mtime == current_mtime {
        return Arc::clone(&entry.value);
      }
    }

    // 2026-05-13: substrate-execution move. Previously used the
    // literal `parse_px_file`, which silently fell back to
    // `default_fn()` whenever a `.px` data file grew past pure
    // literal shape (e.g. `import + ++`). That fallback masked 21
    // doghouse-core test failures because `query-classifiers.px`
    // had outgrown the literal parser. The eval-fallback variant
    // handles both shapes — literal stays fast, expression-shape
    // resolves via pnix-eval. This is the substrate-execution
    // layer, not doghouse, so executing pnix is on-spec here.
    let value = Arc::new(
      crate::px::parse_px_file_with_pnix_eval_fallback(&path)
        .map(|pv| parse_fn(&pv))
        .unwrap_or_else(|_| default_fn()),
    );

    let result = Arc::clone(&value);
    *guard = Some(CacheEntry {
      value,
      path,
      file_mtime: current_mtime,
      last_stat: std::time::Instant::now(),
    });
    result
  }

  /// 캐시를 강제로 무효화한다. 다음 접근 시 재로드된다.
  pub fn invalidate(&self) {
    let mut guard = self.inner.write().unwrap_or_else(|e| e.into_inner());
    *guard = None;
  }
}

fn file_mtime(path: &Path) -> Option<SystemTime> {
  std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

// NOTE (2026-05-13 move): `invalidate_all_px_caches()` lived here
// before the substrate-execution split, but it called into doghouse-
// specific submodules (`expr` / `retrieval` / `conversation` /
// `auto_learn` / `action_engine`) — that aggregation is doghouse-
// owned glue, not substrate machinery. The function moved to
// `doghouse_core::px_cache_registry::invalidate_all_px_caches()`.
// Callers in doghouse should import from that path; this crate now
// owns only the generic `HotPxCache<T>` data structure.

#[cfg(test)]
mod tests {
  use super::*;
  use std::io::Write;

  #[test]
  fn hot_cache_reloads_on_file_change() {
    let dir = std::env::temp_dir().join(format!(
      "px-hot-test-{}",
      std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let px_path = dir.join("test.px");

    std::fs::write(&px_path, r#"{ value = "hello"; }"#).unwrap();

    static CACHE: HotPxCache<String> = HotPxCache::new();

    let path_clone = px_path.clone();
    std::env::set_var("_PX_HOT_TEST_PATH", px_path.to_str().unwrap());
    let path_fn = || PathBuf::from(std::env::var("_PX_HOT_TEST_PATH").unwrap());
    let parse_fn = |v: &crate::px::PxValue| -> String { v.get_str("value") };
    let default_fn = || "default".to_string();

    let v1 = CACHE.get(path_fn, parse_fn, default_fn);
    assert_eq!(v1.as_str(), "hello");

    // 파일 수정
    std::thread::sleep(std::time::Duration::from_millis(50));
    let mut f = std::fs::File::create(&path_clone).unwrap();
    f.write_all(br#"{ value = "world"; }"#).unwrap();
    f.sync_all().unwrap();

    // stat recheck interval을 넘기기 위해 invalidate
    CACHE.invalidate();
    let v2 = CACHE.get(path_fn, parse_fn, default_fn);
    assert_eq!(v2.as_str(), "world");

    let _ = std::fs::remove_dir_all(&dir);
  }
}
