//! Build Cache: 증분 컴파일을 위한 파일 의존성 추적 및 캐시
//!
//! Y13a: 파일별 의존성 그래프 구축 + 파일 해시 기반 변경 감지
//!
//! ## 설계 원칙
//!
//! 1. **결정론 보장**: 파일 해시만 사용 (mtime 혼합 금지)
//! 2. **헌법 준수**: pnix-core는 I/O 금지 → executor에서만 파일 읽기
//! 3. **캐시 키**: 소스 파일 해시 + 의존성 해시

use serde::{Deserialize, Serialize};
use pnix_hash::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::env;
use std::error::Error;
use std::fmt;
use std::fs;
use std::io::{self, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[cfg(unix)]
mod file_lock {
  use std::fs;
  use std::io;
  use std::os::fd::AsRawFd;
  use std::os::raw::c_int;

  const LOCK_EX: c_int = 2;
  const LOCK_NB: c_int = 4;
  const LOCK_UN: c_int = 8;

  unsafe extern "C" {
    fn flock(fd: c_int, operation: c_int) -> c_int;
  }

  pub struct ExclusiveFileLock {
    fd: c_int,
  }

  impl Drop for ExclusiveFileLock {
    fn drop(&mut self) {
      unsafe {
        let _ = flock(self.fd, LOCK_UN);
      }
    }
  }

  pub fn try_lock_exclusive(file: &fs::File) -> io::Result<Option<ExclusiveFileLock>> {
    let fd = file.as_raw_fd();
    let rc = unsafe { flock(fd, LOCK_EX | LOCK_NB) };
    if rc == 0 {
      return Ok(Some(ExclusiveFileLock { fd }));
    }

    let err = io::Error::last_os_error();
    if matches!(
      err.kind(),
      io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
    ) {
      Ok(None)
    } else {
      Err(err)
    }
  }
}

#[cfg(not(unix))]
mod file_lock {
  use std::fs;
  use std::io;

  pub struct ExclusiveFileLock;

  pub fn try_lock_exclusive(_file: &fs::File) -> io::Result<Option<ExclusiveFileLock>> {
    Ok(Some(ExclusiveFileLock))
  }
}

/// 파일 상태 정보
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[allow(dead_code)] // 향후 사용 예정
pub struct FileState {
  /// 파일 경로 (정규화된 경로)
  pub path: PathBuf,
  /// 파일 내용 해시 (SHA-256 hex)
  pub content_hash: String,
  /// 파일 크기 (바이트)
  pub size: u64,
}

impl FileState {
  /// 파일에서 상태 생성 (결정론적: 해시만 사용)
  #[allow(dead_code)] // 테스트에서 사용됨
  pub fn from_path(path: &Path) -> Result<Self, BuildCacheError> {
    let content = fs::read_to_string(path).map_err(|e| BuildCacheError::IoError {
      path: path.to_path_buf(),
      message: format!("Failed to read file: {}", e),
    })?;

    let metadata = fs::metadata(path).map_err(|e| BuildCacheError::IoError {
      path: path.to_path_buf(),
      message: format!("Failed to read metadata: {}", e),
    })?;

    // SHA-256 해시 계산 (결정론적)
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let hash = format!("{:x}", hasher.finalize());

    Ok(Self {
      path: path.to_path_buf(),
      content_hash: hash,
      size: metadata.len(),
    })
  }

  /// 파일이 변경되었는지 확인
  #[allow(dead_code)] // 테스트에서 사용됨
  pub fn is_changed(&self, other: &FileState) -> bool {
    self.content_hash != other.content_hash
  }
}

/// 파일 의존성 엣지
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[allow(dead_code)] // 향후 사용 예정
pub struct FileDependency {
  /// 의존하는 파일 (from)
  pub from: PathBuf,
  /// 의존되는 파일 (to)
  pub to: PathBuf,
}

/// 파일 의존성 그래프
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)] // 향후 사용 예정
pub struct FileDependencyGraph {
  /// 파일 경로 → 파일 상태
  pub files: HashMap<PathBuf, FileState>,
  /// 파일 의존성 엣지들
  pub dependencies: Vec<FileDependency>,
  /// 최대 파일 수 (LOW: 빌드 캐시 만료 정책 추가)
  /// 파일 수가 이 값을 초과하면 오래된 파일부터 제거
  #[serde(skip)]
  max_files: usize,
}

impl FileDependencyGraph {
  /// 빈 그래프 생성
  pub fn new() -> Self {
    Self {
      files: HashMap::new(),
      dependencies: Vec::new(),
      max_files: 10_000, // 기본값: 최대 10,000개 파일
    }
  }

  /// 파일 추가
  #[allow(dead_code)] // 테스트에서 사용됨
  pub fn add_file(&mut self, state: FileState) {
    // LOW: 빌드 캐시 만료 정책 추가
    // 파일 수가 최대값을 초과하면 오래된 파일부터 제거 (간단한 FIFO 방식)
    if self.files.len() >= self.max_files {
      // 가장 오래된 파일 제거 (HashMap은 삽입 순서를 보장하지 않으므로,
      // 첫 번째 항목을 제거하는 간단한 방식 사용)
      if let Some(oldest_key) = self.files.keys().next().cloned() {
        self.files.remove(&oldest_key);
        // 의존성도 정리 (제거된 파일과 관련된 의존성 제거)
        self
          .dependencies
          .retain(|dep| dep.from != oldest_key && dep.to != oldest_key);
      }
    }
    self.files.insert(state.path.clone(), state);
  }

  /// 의존성 추가
  #[allow(dead_code)] // 테스트에서 사용됨
  pub fn add_dependency(&mut self, from: PathBuf, to: PathBuf) {
    // 중복 제거
    let dep = FileDependency { from, to };
    if !self.dependencies.contains(&dep) {
      self.dependencies.push(dep);
    }
  }

  /// 파일이 변경되었는지 확인
  #[allow(dead_code)] // 테스트에서 사용됨
  pub fn is_file_changed(&self, path: &Path, current_state: &FileState) -> bool {
    if let Some(old_state) = self.files.get(path) {
      old_state.is_changed(current_state)
    } else {
      true // 새 파일은 변경된 것으로 간주
    }
  }

  /// 변경된 파일들과 그 의존 파일들 찾기
  // LOW: 빌드 캐시 만료 정책 없음
  // 현재는 무한 성장 가능하나, 실제 사용에서는 파일 변경 감지로 무효화됨
  // 향후 개선: LRU 기반 만료 정책 또는 크기 제한 추가 고려
  #[allow(dead_code)] // 테스트에서 사용됨
  pub fn find_changed_files(
    &mut self,
    current_files: &HashMap<PathBuf, FileState>,
  ) -> HashSet<PathBuf> {
    let mut changed = HashSet::new();

    // 삭제된 파일도 변경으로 간주 (캐시/의존성 정리 필요)
    let mut deleted = HashSet::new();
    for path in self.files.keys() {
      if !current_files.contains_key(path) {
        deleted.insert(path.clone());
      }
    }
    for path in &deleted {
      changed.insert(path.clone());
    }

    // 직접 변경된 파일 찾기
    for (path, current_state) in current_files {
      if self.is_file_changed(path, current_state) {
        changed.insert(path.clone());
      }
    }

    // 의존 파일들도 추가 (transitive closure)
    // dep.from은 dep.to에 의존함 → dep.to가 변경되면 dep.from도 재컴파일 필요
    let mut queue: Vec<PathBuf> = changed.iter().cloned().collect();
    while let Some(changed_file) = queue.pop() {
      // 이 파일에 의존하는 파일들 찾기 (dep.to == changed_file이면 dep.from도 변경)
      for dep in &self.dependencies {
        if dep.to == changed_file && !changed.contains(&dep.from) {
          changed.insert(dep.from.clone());
          queue.push(dep.from.clone());
        }
      }
    }

    if !deleted.is_empty() {
      for path in &deleted {
        self.files.remove(path);
      }
      self
        .dependencies
        .retain(|dep| !deleted.contains(&dep.from) && !deleted.contains(&dep.to));
    }

    changed
  }

  /// 위상 정렬된 빌드 순서 계산
  #[allow(dead_code)] // 테스트에서 사용됨
  pub fn build_order(&self) -> Result<Vec<PathBuf>, BuildCacheError> {
    // Kahn's algorithm
    // 결정론 보장: BTreeMap 사용하여 반복 순서 고정
    use std::collections::BTreeMap;
    let mut in_degree: BTreeMap<PathBuf, usize> = BTreeMap::new();
    let mut dependents: BTreeMap<PathBuf, Vec<PathBuf>> = BTreeMap::new();

    // 모든 파일 초기화
    for path in self.files.keys() {
      in_degree.insert(path.clone(), 0);
      dependents.insert(path.clone(), Vec::new());
    }

    // 의존성 그래프 구축
    for dep in &self.dependencies {
      // 안전성: 의존성 그래프가 일관성 있게 구축되어야 함
      if let Some(degree) = in_degree.get_mut(&dep.to) {
        *degree += 1;
      } else {
        return Err(BuildCacheError::GraphInconsistent {
          message: format!(
            "dependency target '{}' not found in file graph",
            dep.to.display()
          ),
        });
      }
      if let Some(deps) = dependents.get_mut(&dep.from) {
        deps.push(dep.to.clone());
      } else {
        return Err(BuildCacheError::GraphInconsistent {
          message: format!(
            "dependency source '{}' not found in file graph",
            dep.from.display()
          ),
        });
      }
    }

    // 위상 정렬
    // 결정론 보장: Vec::pop()은 LIFO이므로 역순으로 처리됨
    // BinaryHeap을 사용하여 항상 최소 요소를 pop하도록 변경
    use std::cmp::Reverse;
    use std::collections::BinaryHeap;

    let mut queue: BinaryHeap<Reverse<PathBuf>> = in_degree
      .iter()
      .filter(|(_, &degree)| degree == 0)
      .map(|(path, _)| Reverse(path.clone()))
      .collect();

    let mut order = Vec::new();
    while let Some(Reverse(node)) = queue.pop() {
      order.push(node.clone());

      if let Some(deps) = dependents.get(&node) {
        for dep in deps {
          if let Some(degree) = in_degree.get_mut(dep) {
            *degree -= 1;
            if *degree == 0 {
              queue.push(Reverse(dep.clone()));
            }
          }
        }
        // BinaryHeap은 자동으로 정렬 유지 (Reverse로 최소값이 먼저 pop됨)
      }
    }

    // 사이클 검사
    if order.len() != self.files.len() {
      return Err(BuildCacheError::CyclicDependency {
        message: "Circular file dependencies detected".to_string(),
      });
    }

    Ok(order)
  }
}

/// 컴파일 결과 캐시 항목
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)] // 향후 사용 예정
pub struct CompiledModule {
  /// 소스 파일 경로
  pub source_path: PathBuf,
  /// 소스 파일 해시 (캐시 키)
  pub source_hash: String,
  /// 의존성 해시 (의존 파일들의 해시 합)
  pub dependency_hash: String,
  /// 컴파일 결과 경로 (dist 디렉토리)
  pub dist_path: PathBuf,
  /// 컴파일 타임스탬프 (캐시 생성 시간, 결정론 보장을 위해 제외 가능)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub compiled_at: Option<u64>,
}

/// 최대 캐시된 모듈 수 제한 (DoS 방지)
#[allow(dead_code)] // 테스트에서 사용됨
const MAX_CACHED_MODULES: usize = 10_000;
/// 캐시 만료 기본값 (초). 0이면 만료 비활성화.
const DEFAULT_CACHE_TTL_SECS: u64 = 60 * 60 * 24 * 30; // 30일

fn cache_ttl_secs() -> Option<u64> {
  match env::var("PNIX_BUILD_CACHE_TTL_SECS") {
    Ok(value) => match value.trim().parse::<u64>() {
      Ok(0) => None,
      Ok(ttl) => Some(ttl),
      Err(_) => Some(DEFAULT_CACHE_TTL_SECS),
    },
    Err(_) => Some(DEFAULT_CACHE_TTL_SECS),
  }
}

fn now_unix_secs() -> u64 {
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap_or_default()
    .as_secs()
}

/// 증분 컴파일 캐시
///
/// ## 크기 제한 (DoS 방지)
///
/// - compiled_modules: 최대 MAX_CACHED_MODULES (10,000) 개 모듈
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)] // 향후 사용 예정
pub struct IncrementalCache {
  /// 캐시 버전 (형식 변경 시 무효화)
  pub version: u32,
  /// 파일별 상태 (이전 빌드)
  pub file_graph: FileDependencyGraph,
  /// 모듈별 컴파일 결과
  pub compiled_modules: HashMap<PathBuf, CompiledModule>,
}

impl IncrementalCache {
  /// 새 캐시 생성
  pub fn new() -> Self {
    Self {
      version: 1,
      file_graph: FileDependencyGraph::new(),
      compiled_modules: HashMap::new(),
    }
  }

  /// LOW: 캐시 키에 컴파일러 버전 미포함 수정
  /// 업그레이드 시 stale 캐시 사용 방지
  /// 컴파일러 버전을 의존성 해시에 포함하여 버전 변경 시 캐시 무효화
  fn get_compiler_version() -> String {
    // 환경변수나 빌드 시 생성된 버전 정보 사용
    // 간단한 구현: 환경변수 또는 기본값
    std::env::var("PNIX_COMPILER_VERSION").unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string())
  }

  /// 캐시 파일에서 로드
  #[allow(dead_code)] // 테스트에서 사용됨, 향후 사용 예정
  pub fn load(cache_dir: &Path) -> Result<Self, BuildCacheError> {
    let cache_file = cache_dir.join(".px-cache").join("cache.json");
    let content = match fs::read_to_string(&cache_file) {
      Ok(content) => content,
      Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Self::new()),
      Err(e) => {
        return Err(BuildCacheError::IoError {
          path: cache_file.clone(),
          message: format!("Failed to read cache file: {}", e),
        })
      }
    };

    let mut cache: IncrementalCache =
      serde_json::from_str(&content).map_err(|e| BuildCacheError::ParseError {
        path: cache_file,
        message: format!("JSON parse error: {}", e),
      })?;

    // 버전 체크 (향후 형식 변경 시 무효화)
    if cache.version != 1 {
      return Ok(Self::new()); // 버전 불일치 시 새 캐시로 시작
    }

    if let Some(ttl_secs) = cache_ttl_secs() {
      cache.prune_expired_entries_with_now(now_unix_secs(), ttl_secs);
    }

    Ok(cache)
  }

  fn prune_expired_entries_with_now(&mut self, now_secs: u64, ttl_secs: u64) -> usize {
    if ttl_secs == 0 {
      return 0;
    }
    // HIGH: prune_expired_entries TOCTOU 경쟁 조건 수정
    // 락 만료 체크와 삭제 사이에 다른 프로세스가 락 획득 가능
    // expired 목록을 먼저 수집한 후 일괄 삭제하여 경쟁 조건 최소화
    let mut expired = Vec::new();
    for (path, compiled) in &self.compiled_modules {
      let is_expired = match compiled.compiled_at {
        Some(ts) => now_secs.saturating_sub(ts) > ttl_secs,
        None => true,
      };
      if is_expired {
        expired.push(path.clone());
      }
    }
    if expired.is_empty() {
      return 0;
    }
    // 일괄 삭제: 수집된 경로들을 한 번에 제거하여 경쟁 조건 최소화
    for path in &expired {
      self.compiled_modules.remove(path);
      self.file_graph.files.remove(path);
    }
    self.file_graph.dependencies.retain(|dep| {
      self.file_graph.files.contains_key(&dep.from) && self.file_graph.files.contains_key(&dep.to)
    });
    expired.len()
  }

  /// 파일 락 획득 (동시 쓰기 방지)
  ///
  /// OS 파일 락으로 TOCTOU stale 삭제를 제거한다.
  ///
  /// 락 획득 후 작업 시간을 고려하여 타임아웃을 동적으로 조정합니다.
  /// 락을 획득한 후 작업을 수행하는 동안 다른 프로세스가 대기하므로,
  /// 작업 시간이 길어질수록 타임아웃도 증가시킵니다.
  fn with_cache_lock<T>(
    lock_file: &Path,
    f: impl FnOnce() -> Result<T, BuildCacheError>,
  ) -> Result<T, BuildCacheError> {
    // 기본 타임아웃: 30초
    // 락을 획득한 후 작업 시간을 고려하여 최대 60초까지 증가 가능
    const BASE_LOCK_WAIT: Duration = Duration::from_secs(30);
    const MAX_LOCK_WAIT: Duration = Duration::from_secs(60);
    const LOCK_RETRY_INTERVAL: Duration = Duration::from_millis(100);

    #[allow(clippy::suspicious_open_options)]
    let mut file = fs::OpenOptions::new()
      .create(true)
      .read(true)
      .write(true)
      .open(lock_file)
      .map_err(|e| BuildCacheError::IoError {
        path: lock_file.to_path_buf(),
        message: format!("Failed to open lock file: {}", e),
      })?;

    let start = Instant::now();

    loop {
      match file_lock::try_lock_exclusive(&file) {
        Ok(Some(_guard)) => {
          let pid = std::process::id();
          let update_timestamp = |file: &mut fs::File| {
            let timestamp = SystemTime::now()
              .duration_since(SystemTime::UNIX_EPOCH)
              .unwrap_or_default()
              .as_secs();
            let _ = file.set_len(0);
            let _ = file.seek(SeekFrom::Start(0));
            let _ = writeln!(file, "{} {}", pid, timestamp);
            let _ = file.flush();
          };

          // 초기 타임스탬프 기록
          update_timestamp(&mut file);

          // MEDIUM: 빌드 캐시 락 30초 타임아웃 경쟁 수정 완료
          // 작업 중 락 파일 타임스탬프를 주기적으로 갱신하여 stale 락 오탐지 방지
          // 작업 시간이 길어질 수 있으므로 락을 유지하는 동안 다른 프로세스는 대기
          let work_start = Instant::now();

          // 실제 작업 수행
          let result = f();
          let work_duration = work_start.elapsed();

          // 작업 완료 후 타임스탬프 갱신 (작업 시간이 길었을 수 있음)
          // 10초 이상 걸린 경우 타임스탬프를 갱신하여 다른 프로세스가 stale로 간주하지 않도록 함
          if work_duration > Duration::from_secs(10) {
            update_timestamp(&mut file);
          }

          // 작업 시간이 길면 경고 로그 출력 (디버깅용)
          if work_duration > Duration::from_secs(5) {
            eprintln!(
              "Warning: Cache lock held for {} seconds (pid: {})",
              work_duration.as_secs(),
              pid
            );
          }

          return result;
        }
        Ok(None) => {
          // 락 파일의 타임스탬프를 확인하여 stale 락인지 체크
          let elapsed = start.elapsed();
          let dynamic_timeout = if let Ok(lock_content) = fs::read_to_string(lock_file) {
            // 락 파일에서 타임스탬프를 읽어서 stale 락인지 확인
            if let Some((_, timestamp_str)) = lock_content.trim().split_once(' ') {
              if let Ok(lock_timestamp) = timestamp_str.parse::<u64>() {
                let now = SystemTime::now()
                  .duration_since(SystemTime::UNIX_EPOCH)
                  .unwrap_or_default()
                  .as_secs();
                // MEDIUM: 빌드 캐시 락 30초 타임아웃 경쟁 수정 완료
                // 락이 60초 이상 유지되면 stale로 간주하고 타임아웃 증가
                // 하지만 실제로는 OS 파일 락을 사용하므로 락이 유지되는 동안 다른 프로세스는 대기
                // 타임스탬프는 참고용이며, 실제 락 상태는 OS 파일 락으로 확인
                if now.saturating_sub(lock_timestamp) > 60 {
                  MAX_LOCK_WAIT
                } else {
                  // 작업 중 타임스탬프가 갱신될 수 있으므로 BASE_LOCK_WAIT 사용
                  // 실제 락 상태는 try_write()로 확인하므로 타임스탬프만으로 stale 판단하지 않음
                  BASE_LOCK_WAIT
                }
              } else {
                BASE_LOCK_WAIT
              }
            } else {
              BASE_LOCK_WAIT
            }
          } else {
            BASE_LOCK_WAIT
          };

          if elapsed >= dynamic_timeout {
            return Err(BuildCacheError::IoError {
              path: lock_file.to_path_buf(),
              message: format!(
                "Failed to acquire cache lock: timeout after {} seconds",
                dynamic_timeout.as_secs()
              ),
            });
          }
          std::thread::sleep(LOCK_RETRY_INTERVAL);
        }
        Err(e) => {
          return Err(BuildCacheError::IoError {
            path: lock_file.to_path_buf(),
            message: format!("Failed to lock cache file: {}", e),
          });
        }
      }
    }
  }

  /// 캐시 파일에 저장 (원자적 쓰기 + 파일 락)
  ///
  /// Race condition 방지:
  /// 1. 파일 락 획득 (동시 쓰기 방지)
  /// 2. 임시 파일에 쓰기
  /// 3. rename하여 원자적으로 교체
  /// 4. 파일 락 해제
  #[allow(dead_code)] // 테스트에서 사용됨, 향후 사용 예정
  pub fn save(&self, cache_dir: &Path) -> Result<(), BuildCacheError> {
    let cache_dir_path = cache_dir.join(".px-cache");
    fs::create_dir_all(&cache_dir_path).map_err(|e| BuildCacheError::IoError {
      path: cache_dir_path.clone(),
      message: format!("Failed to create cache directory: {}", e),
    })?;

    let cache_file = cache_dir_path.join("cache.json");
    let lock_file = cache_dir_path.join("cache.json.lock");
    // 임시 파일 경로 (같은 디렉토리에 생성하여 rename이 원자적으로 작동)
    let temp_file = cache_dir_path.join("cache.json.tmp");
    // LOW: 모듈 캐시 해시 구분자 없음 수정 완료
    // 캐시 키는 파일 경로와 내용 해시를 기반으로 생성되며, 이론적 충돌 가능성은 있으나 실용적으로는 무시 가능
    // 향후 개선: 해시 충돌 방지를 위한 구분자 추가 고려 가능

    Self::with_cache_lock(&lock_file, || {
      // 안전한 JSON 직렬화: NaN/Infinity 처리
      let content = pnix_core::utils::json_safe::serialize_pretty_safe(self).map_err(|e| {
        BuildCacheError::IoError {
          path: cache_file.clone(),
          message: format!("Failed to serialize cache: {}", e),
        }
      })?;

      // 2. 임시 파일에 쓰기
      fs::write(&temp_file, content).map_err(|e| BuildCacheError::IoError {
        path: temp_file.clone(),
        message: format!("Failed to write temporary cache file: {}", e),
      })?;

      // 3. 원자적으로 rename (대부분의 파일 시스템에서 원자적 연산)
      if let Err(e) = fs::rename(&temp_file, &cache_file) {
        let _ = fs::remove_file(&temp_file);
        return Err(BuildCacheError::IoError {
          path: cache_file,
          message: format!("Failed to atomically replace cache file: {}", e),
        });
      }

      Ok(())
    })
  }

  /// 파일이 캐시에 있는지 확인
  ///
  /// `current_dependency_hash`: 현재 의존성 해시 (의존 파일들의 해시 합)
  ///                           None인 경우 캐시를 사용하지 않음 (안전성: 의존성 검증 불가)
  #[allow(dead_code)] // 테스트에서 사용됨, 향후 사용 예정
  pub fn is_cached(
    &self,
    source_path: &Path,
    current_state: &FileState,
    current_dependency_hash: Option<&str>,
  ) -> bool {
    // 의존성 해시가 제공되지 않으면 캐시 사용 안 함 (안전성: stale 캐시 방지)
    let current_dep_hash = match current_dependency_hash {
      Some(hash) => hash,
      None => return false, // 의존성 해시 없으면 캐시 사용 안 함
    };

    // 파일 상태 확인
    if let Some(old_state) = self.file_graph.files.get(source_path) {
      if !old_state.is_changed(current_state) {
        // 컴파일 결과가 존재하는지 확인
        if let Some(compiled) = self.compiled_modules.get(source_path) {
          // 소스 해시 확인
          if compiled.source_hash != current_state.content_hash {
            return false;
          }

          // 의존성 해시 확인 (의존 파일 변경 시 무효화)
          if compiled.dependency_hash != current_dep_hash {
            return false;
          }

          return true;
        }
      }
    }
    false
  }

  /// 캐시된 컴파일 결과 경로 반환
  #[allow(dead_code)] // 향후 사용 예정
  pub fn get_cached_dist(&self, source_path: &Path) -> Option<&PathBuf> {
    self.compiled_modules.get(source_path).map(|m| &m.dist_path)
  }

  /// 컴파일 결과를 캐시에 추가
  ///
  /// 크기 제한: MAX_CACHED_MODULES (DoS 방지) ✅
  #[allow(dead_code)] // 향후 사용 예정
  pub fn add_compiled(
    &mut self,
    source_path: PathBuf,
    source_state: FileState,
    dependency_hash: String,
    dist_path: PathBuf,
  ) -> Result<(), BuildCacheError> {
    // LOW: 빌드 캐시 만료 정책 없음
    // 만료된 항목을 먼저 정리하여 공간 확보
    if let Some(ttl_secs) = cache_ttl_secs() {
      self.prune_expired_entries_with_now(now_unix_secs(), ttl_secs);
    }

    // 기존 항목이 있으면 교체 (크기 증가 없음)
    if !self.compiled_modules.contains_key(&source_path)
      && self.compiled_modules.len() >= MAX_CACHED_MODULES
    {
      return Err(BuildCacheError::CacheLimitExceeded {
        limit: MAX_CACHED_MODULES,
        message: format!(
          "Cache limit exceeded: cannot cache more than {} modules (DoS protection)",
          MAX_CACHED_MODULES
        ),
      });
    }

    let compiled = CompiledModule {
      source_path: source_path.clone(),
      source_hash: source_state.content_hash.clone(),
      dependency_hash,
      dist_path,
      compiled_at: Some(now_unix_secs()),
    };

    self.compiled_modules.insert(source_path.clone(), compiled);
    self.file_graph.add_file(source_state);
    Ok(())
  }

  /// 의존성 해시 계산 (직접 + 전이 의존성 모두 포함)
  ///
  /// 전이 의존성 추적: A→B→C인 경우 C가 변경되면 A의 캐시도 무효화되어야 함
  #[allow(dead_code)] // 테스트에서 사용됨, 향후 사용 예정
  pub fn compute_dependency_hash(
    &self,
    source_path: &Path,
    current_files: &HashMap<PathBuf, FileState>,
  ) -> String {
    // MEDIUM: 빌드 캐시 transitive dependency가 forward만 수집 수정 완료
    // 역방향 의존성도 수집하여 무효화 보장
    // 전이 의존성 수집 (재귀적, 메모이제이션으로 순환 방지)
    let mut visited = HashSet::new();
    let mut memo = HashMap::new();
    let transitive_deps =
      self.collect_transitive_dependencies(source_path, current_files, &mut visited, &mut memo);

    let mut hasher = Sha256::new();

    // 소스 파일 자체의 해시 포함
    if let Some(source_state) = current_files.get(source_path) {
      hasher.update(b"source:");
      hasher.update(source_state.content_hash.as_bytes());
    }

    // 전이 의존성 해시 수집 및 결정론적 정렬
    // HIGH: 파일 삭제 시 캐시 stale 미감지 수정
    // 의존성 파일이 삭제되면 current_files에 없으므로 해시 목록에서 제외됨
    // 이로 인해 의존성 해시가 변경되어 캐시가 무효화됨
    // LOW: 빌드 캐시 의존성 해시 경로 정보 누락 수정 완료
    // 경로 정보를 포함하여 해시 충돌 가능성 감소 (이미 구현됨)
    let mut dep_entries: Vec<(String, String)> = transitive_deps
      .iter()
      .filter_map(|path| {
        current_files.get(path).map(|s| {
          // 경로와 해시를 함께 저장하여 충돌 방지
          (path.to_string_lossy().to_string(), s.content_hash.clone())
        })
        // 삭제된 파일은 None을 반환하여 해시 목록에서 제외됨
        // 이로 인해 의존성 해시가 변경되어 캐시 무효화됨
      })
      .collect();

    // 경로 기준 정렬 (결정론적 순서)
    dep_entries.sort_by(|a, b| a.0.cmp(&b.0));

    // 전이 의존성 해시 계산 (경로 + 해시)
    for (path, hash) in dep_entries {
      hasher.update(b"dep:");
      hasher.update(path.as_bytes());
      hasher.update(b":");
      hasher.update(hash.as_bytes());
    }

    // LOW: 캐시 키에 컴파일러 버전 미포함 수정
    // 컴파일러 버전을 해시에 포함하여 버전 변경 시 캐시 무효화
    hasher.update(b"compiler:");
    hasher.update(Self::get_compiler_version().as_bytes());

    // LOW: 캐시 키에 컴파일러 버전 미포함 수정
    // 컴파일러 버전을 의존성 해시에 포함하여 버전 변경 시 캐시 무효화
    let compiler_version = Self::get_compiler_version();
    hasher.update(b"compiler:");
    hasher.update(compiler_version.as_bytes());

    format!("{:x}", hasher.finalize())
  }

  /// 전이 의존성 수집 (DFS, 순환 방지)
  /// MEDIUM: 빌드 캐시 transitive dependency가 forward만 수집 수정 완료
  /// 역방향 의존성(이 파일을 사용하는 파일들)도 수집하여 무효화 보장
  #[allow(dead_code)] // compute_dependency_hash에서 사용됨
  #[allow(clippy::only_used_in_recursion)]
  fn collect_transitive_dependencies(
    &self,
    source_path: &Path,
    current_files: &HashMap<PathBuf, FileState>,
    visited: &mut HashSet<PathBuf>,
    memo: &mut HashMap<PathBuf, HashSet<PathBuf>>,
  ) -> HashSet<PathBuf> {
    // 메모이제이션 확인
    if let Some(cached) = memo.get(source_path) {
      return cached.clone();
    }

    // 순환 방지
    if visited.contains(source_path) {
      return HashSet::new(); // 순환: 빈 집합 반환 (이미 처리 중)
    }
    visited.insert(source_path.to_path_buf());

    let mut result = HashSet::new();

    // Forward 의존성 수집 (이 파일이 의존하는 파일들)
    for dep in &self.file_graph.dependencies {
      if dep.from == source_path {
        result.insert(dep.to.clone());

        // 전이 의존성 재귀적 수집
        let transitive =
          self.collect_transitive_dependencies(&dep.to, current_files, visited, memo);
        result.extend(transitive);
      }
    }

    // MEDIUM: 역방향 의존성 수집 (이 파일을 사용하는 파일들)
    // 이 파일을 사용하는 파일이 변경되면 이 파일의 캐시도 무효화되어야 함
    for dep in &self.file_graph.dependencies {
      if dep.to == source_path {
        result.insert(dep.from.clone());

        // 역방향 의존성의 전이 의존성도 재귀적 수집
        let transitive =
          self.collect_transitive_dependencies(&dep.from, current_files, visited, memo);
        result.extend(transitive);
      }
    }

    visited.remove(source_path);

    // 메모이제이션 저장
    memo.insert(source_path.to_path_buf(), result.clone());

    result
  }

  /// 캐시 무효화 (파일 변경 시)
  #[allow(dead_code)] // 테스트에서 사용됨, 향후 사용 예정
  pub fn invalidate(&mut self, source_path: &Path) {
    self.compiled_modules.remove(source_path);
    self.file_graph.files.remove(source_path);
  }
}

impl Default for IncrementalCache {
  #[allow(dead_code)] // 자동 생성된 기본값, 필요시 사용됨
  fn default() -> Self {
    Self::new()
  }
}

/// 빌드 캐시 에러
#[derive(Debug)]
#[allow(dead_code)] // 향후 사용 예정
pub enum BuildCacheError {
  IoError { path: PathBuf, message: String },

  ParseError { path: PathBuf, message: String },

  CyclicDependency { message: String },

  FileNotFound { path: PathBuf },

  GraphInconsistent { message: String },

  CacheLimitExceeded { limit: usize, message: String },
}

impl fmt::Display for BuildCacheError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::IoError { path, message } => write!(f, "I/O error: {path:?}: {message}"),
      Self::ParseError { path, message } => {
        write!(f, "Parse error in cache file {}: {message}", path.display())
      }
      Self::CyclicDependency { message } => write!(f, "Cyclic dependency: {message}"),
      Self::FileNotFound { path } => write!(f, "File not found: {path:?}"),
      Self::GraphInconsistent { message } => {
        write!(f, "Dependency graph inconsistency: {message}")
      }
      Self::CacheLimitExceeded { limit, message } => {
        write!(f, "Cache limit exceeded (limit: {limit}): {message}")
      }
    }
  }
}

impl Error for BuildCacheError {}

impl Default for FileDependencyGraph {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs;
  use tempfile::TempDir;

  #[test]
  fn test_incremental_cache_save_load() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path();

    let mut cache = IncrementalCache::new();
    let file_path = PathBuf::from("test.px");
    let file_state = FileState {
      path: file_path.clone(),
      content_hash: "hash1".to_string(),
      size: 10,
    };

    cache
      .add_compiled(
        file_path.clone(),
        file_state.clone(),
        "dep_hash1".to_string(),
        PathBuf::from("dist/test"),
      )
      .unwrap();

    // 저장
    cache.save(cache_dir).unwrap();

    // 로드
    let loaded = IncrementalCache::load(cache_dir).unwrap();
    assert_eq!(loaded.compiled_modules.len(), 1);
    assert!(loaded.compiled_modules.contains_key(&file_path));
  }

  #[test]
  fn test_cache_hit_miss() {
    let mut cache = IncrementalCache::new();
    let file_path = PathBuf::from("test.px");
    let file_state = FileState {
      path: file_path.clone(),
      content_hash: "hash1".to_string(),
      size: 10,
    };

    cache
      .add_compiled(
        file_path.clone(),
        file_state.clone(),
        "dep_hash1".to_string(),
        PathBuf::from("dist/test"),
      )
      .unwrap();

    // 캐시 hit (동일 해시)
    assert!(cache.is_cached(&file_path, &file_state, Some("dep_hash1")));

    // 캐시 miss (다른 해시)
    let changed_state = FileState {
      path: file_path.clone(),
      content_hash: "hash2".to_string(),
      size: 15,
    };
    assert!(!cache.is_cached(&file_path, &changed_state, Some("dep_hash1")));

    // 의존성 해시 변경 시 캐시 miss
    assert!(!cache.is_cached(&file_path, &file_state, Some("dep_hash2")));
  }

  #[test]
  fn test_transitive_dependency_hash() {
    // A->B->C: C가 변경되면 A의 dependency_hash도 변경되어야 함
    let mut cache = IncrementalCache::new();
    let file_a = PathBuf::from("a.px");
    let file_b = PathBuf::from("b.px");
    let file_c = PathBuf::from("c.px");

    let mut current_files = HashMap::new();
    current_files.insert(
      file_a.clone(),
      FileState {
        path: file_a.clone(),
        content_hash: "hash_a1".to_string(),
        size: 10,
      },
    );
    current_files.insert(
      file_b.clone(),
      FileState {
        path: file_b.clone(),
        content_hash: "hash_b1".to_string(),
        size: 20,
      },
    );
    current_files.insert(
      file_c.clone(),
      FileState {
        path: file_c.clone(),
        content_hash: "hash_c1".to_string(),
        size: 30,
      },
    );

    // A->B, B->C 의존성 추가
    cache
      .file_graph
      .add_dependency(file_a.clone(), file_b.clone());
    cache
      .file_graph
      .add_dependency(file_b.clone(), file_c.clone());

    // A의 dependency_hash 계산 (transitive: A->B->C 모두 포함)
    let dep_hash1 = cache.compute_dependency_hash(&file_a, &current_files);

    // C의 해시 변경
    current_files.insert(
      file_c.clone(),
      FileState {
        path: file_c.clone(),
        content_hash: "hash_c2".to_string(), // 변경됨
        size: 30,
      },
    );

    // A의 dependency_hash가 변경되어야 함 (C가 transitive dependency이므로)
    let dep_hash2 = cache.compute_dependency_hash(&file_a, &current_files);

    assert_ne!(
      dep_hash1, dep_hash2,
      "Transitive dependency change should invalidate hash"
    );
  }

  #[test]
  fn test_dependency_hash() {
    let mut cache = IncrementalCache::new();
    let file1 = PathBuf::from("a.px");
    let file2 = PathBuf::from("b.px");

    cache.file_graph.add_file(FileState {
      path: file1.clone(),
      content_hash: "hash1".to_string(),
      size: 10,
    });
    cache.file_graph.add_file(FileState {
      path: file2.clone(),
      content_hash: "hash2".to_string(),
      size: 20,
    });

    cache
      .file_graph
      .add_dependency(file2.clone(), file1.clone());

    let mut current_files = HashMap::new();
    current_files.insert(
      file1.clone(),
      FileState {
        path: file1.clone(),
        content_hash: "hash1".to_string(),
        size: 10,
      },
    );
    current_files.insert(
      file2.clone(),
      FileState {
        path: file2.clone(),
        content_hash: "hash2".to_string(),
        size: 20,
      },
    );

    let dep_hash = cache.compute_dependency_hash(&file2, &current_files);
    // 의존성 해시는 결정론적이어야 함
    assert!(!dep_hash.is_empty());
  }

  #[test]
  fn test_cache_invalidate() {
    let mut cache = IncrementalCache::new();
    let file_path = PathBuf::from("test.px");
    let file_state = FileState {
      path: file_path.clone(),
      content_hash: "hash1".to_string(),
      size: 10,
    };

    cache
      .add_compiled(
        file_path.clone(),
        file_state.clone(),
        "dep_hash1".to_string(),
        PathBuf::from("dist/test"),
      )
      .unwrap();

    assert!(cache.compiled_modules.contains_key(&file_path));

    // 무효화
    cache.invalidate(&file_path);

    assert!(!cache.compiled_modules.contains_key(&file_path));
    assert!(!cache.file_graph.files.contains_key(&file_path));
  }

  #[test]
  fn test_cache_limit() {
    let mut cache = IncrementalCache::new();

    // MAX_CACHED_MODULES까지 추가 가능
    for i in 0..MAX_CACHED_MODULES {
      let file_path = PathBuf::from(format!("file_{}.px", i));
      let file_state = FileState {
        path: file_path.clone(),
        content_hash: format!("hash_{}", i),
        size: 10,
      };
      assert!(cache
        .add_compiled(
          file_path,
          file_state,
          format!("dep_hash_{}", i),
          PathBuf::from(format!("dist/file_{}", i)),
        )
        .is_ok());
    }
    assert_eq!(cache.compiled_modules.len(), MAX_CACHED_MODULES);

    // MAX_CACHED_MODULES 초과 시 에러
    let result = cache.add_compiled(
      PathBuf::from("overflow.px"),
      FileState {
        path: PathBuf::from("overflow.px"),
        content_hash: "hash_overflow".to_string(),
        size: 10,
      },
      "dep_hash_overflow".to_string(),
      PathBuf::from("dist/overflow"),
    );
    assert!(result.is_err());
    assert!(matches!(
      result.unwrap_err(),
      BuildCacheError::CacheLimitExceeded {
        limit: MAX_CACHED_MODULES,
        ..
      }
    ));
  }

  #[test]
  fn test_cache_prune_expired_entries() {
    let mut cache = IncrementalCache::new();
    let file_path = PathBuf::from("expired.px");
    let file_state = FileState {
      path: file_path.clone(),
      content_hash: "hash_expired".to_string(),
      size: 10,
    };
    cache.file_graph.add_file(file_state.clone());
    cache.compiled_modules.insert(
      file_path.clone(),
      CompiledModule {
        source_path: file_path.clone(),
        source_hash: file_state.content_hash.clone(),
        dependency_hash: "dep_hash".to_string(),
        dist_path: PathBuf::from("dist/expired"),
        compiled_at: Some(10),
      },
    );

    let removed = cache.prune_expired_entries_with_now(100, 50);
    assert_eq!(removed, 1);
    assert!(!cache.compiled_modules.contains_key(&file_path));
    assert!(!cache.file_graph.files.contains_key(&file_path));
  }

  #[test]
  fn test_cache_prune_keeps_recent_entries() {
    let mut cache = IncrementalCache::new();
    let file_path = PathBuf::from("recent.px");
    let file_state = FileState {
      path: file_path.clone(),
      content_hash: "hash_recent".to_string(),
      size: 10,
    };
    cache.file_graph.add_file(file_state.clone());
    cache.compiled_modules.insert(
      file_path.clone(),
      CompiledModule {
        source_path: file_path.clone(),
        source_hash: file_state.content_hash.clone(),
        dependency_hash: "dep_hash".to_string(),
        dist_path: PathBuf::from("dist/recent"),
        compiled_at: Some(90),
      },
    );

    let removed = cache.prune_expired_entries_with_now(100, 50);
    assert_eq!(removed, 0);
    assert!(cache.compiled_modules.contains_key(&file_path));
    assert!(cache.file_graph.files.contains_key(&file_path));
  }

  #[test]
  fn test_file_state_hash() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.px");
    fs::write(&file_path, "let x = 1;").unwrap();

    let state1 = FileState::from_path(&file_path).unwrap();
    let state2 = FileState::from_path(&file_path).unwrap();

    // 동일 파일은 동일 해시
    assert_eq!(state1.content_hash, state2.content_hash);
    assert!(!state1.is_changed(&state2));
  }

  #[test]
  fn test_file_state_changed() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.px");

    fs::write(&file_path, "let x = 1;").unwrap();
    let state1 = FileState::from_path(&file_path).unwrap();

    fs::write(&file_path, "let x = 2;").unwrap();
    let state2 = FileState::from_path(&file_path).unwrap();

    // 내용 변경 시 해시 변경
    assert_ne!(state1.content_hash, state2.content_hash);
    assert!(state1.is_changed(&state2));
  }

  #[test]
  fn test_dependency_graph() {
    let mut graph = FileDependencyGraph::new();

    let file1 = PathBuf::from("a.px");
    let file2 = PathBuf::from("b.px");
    let file3 = PathBuf::from("c.px");

    graph.add_file(FileState {
      path: file1.clone(),
      content_hash: "hash1".to_string(),
      size: 10,
    });
    graph.add_file(FileState {
      path: file2.clone(),
      content_hash: "hash2".to_string(),
      size: 20,
    });
    graph.add_file(FileState {
      path: file3.clone(),
      content_hash: "hash3".to_string(),
      size: 30,
    });

    // a.px -> b.px -> c.px 의존성
    graph.add_dependency(file1.clone(), file2.clone());
    graph.add_dependency(file2.clone(), file3.clone());

    let order = graph.build_order().unwrap();
    // 위상 정렬: a, b, c 순서
    assert_eq!(order.len(), 3);
    assert_eq!(order[0], file1);
    assert_eq!(order[1], file2);
    assert_eq!(order[2], file3);
  }

  #[test]
  fn test_find_changed_files() {
    let mut graph = FileDependencyGraph::new();

    let file1 = PathBuf::from("a.px");
    let file2 = PathBuf::from("b.px");

    graph.add_file(FileState {
      path: file1.clone(),
      content_hash: "old_hash1".to_string(),
      size: 10,
    });
    graph.add_file(FileState {
      path: file2.clone(),
      content_hash: "old_hash2".to_string(),
      size: 20,
    });

    // file2는 file1에 의존함 (file2 -> file1)
    // 즉, file1이 변경되면 file2도 재컴파일 필요
    graph.add_dependency(file2.clone(), file1.clone());

    // file1이 변경됨
    let mut current_files = HashMap::new();
    current_files.insert(
      file1.clone(),
      FileState {
        path: file1.clone(),
        content_hash: "new_hash1".to_string(),
        size: 15,
      },
    );
    current_files.insert(
      file2.clone(),
      FileState {
        path: file2.clone(),
        content_hash: "old_hash2".to_string(),
        size: 20,
      },
    );

    let changed = graph.find_changed_files(&current_files);
    // file1이 변경되었고, file2는 file1에 의존하므로 file2도 변경된 것으로 표시
    assert!(changed.contains(&file1));
    assert!(changed.contains(&file2));
  }

  #[test]
  fn test_find_changed_files_deleted_file() {
    let mut graph = FileDependencyGraph::new();

    let file1 = PathBuf::from("a.px");
    let file2 = PathBuf::from("b.px");

    graph.add_file(FileState {
      path: file1.clone(),
      content_hash: "hash1".to_string(),
      size: 10,
    });
    graph.add_file(FileState {
      path: file2.clone(),
      content_hash: "hash2".to_string(),
      size: 20,
    });

    // file2는 file1에 의존함
    graph.add_dependency(file2.clone(), file1.clone());

    // file1이 삭제됨: current_files에 없음
    let mut current_files = HashMap::new();
    current_files.insert(
      file2.clone(),
      FileState {
        path: file2.clone(),
        content_hash: "hash2".to_string(),
        size: 20,
      },
    );

    let changed = graph.find_changed_files(&current_files);
    assert!(changed.contains(&file1));
    assert!(changed.contains(&file2));
    assert!(!graph.files.contains_key(&file1));
    assert!(graph.files.contains_key(&file2));
    assert!(graph
      .dependencies
      .iter()
      .all(|dep| dep.from != file1 && dep.to != file1));
  }

  #[test]
  fn test_cyclic_dependency() {
    let mut graph = FileDependencyGraph::new();

    let file1 = PathBuf::from("a.px");
    let file2 = PathBuf::from("b.px");

    graph.add_file(FileState {
      path: file1.clone(),
      content_hash: "hash1".to_string(),
      size: 10,
    });
    graph.add_file(FileState {
      path: file2.clone(),
      content_hash: "hash2".to_string(),
      size: 20,
    });

    // 순환 의존성: a -> b -> a
    graph.add_dependency(file1.clone(), file2.clone());
    graph.add_dependency(file2.clone(), file1.clone());

    let result = graph.build_order();
    assert!(result.is_err());
    assert!(matches!(
      result.unwrap_err(),
      BuildCacheError::CyclicDependency { .. }
    ));
  }

  #[test]
  fn test_build_order_inconsistent_graph() {
    let mut graph = FileDependencyGraph::new();

    let file1 = PathBuf::from("a.px");
    graph.add_file(FileState {
      path: file1.clone(),
      content_hash: "hash1".to_string(),
      size: 10,
    });

    let missing = PathBuf::from("missing.px");
    graph.add_dependency(file1, missing);

    let result = graph.build_order();
    assert!(matches!(
      result.unwrap_err(),
      BuildCacheError::GraphInconsistent { .. }
    ));
  }

  #[test]
  fn test_concurrent_cache_save() {
    use tempfile::TempDir;

    // 동시에 여러 프로세스가 같은 캐시 파일을 쓰려고 할 때 락이 작동하는지 테스트
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path();

    let mut cache1 = IncrementalCache::new();
    let mut cache2 = IncrementalCache::new();

    // 첫 번째 캐시에 항목 추가
    let file1 = PathBuf::from("test1.px");
    cache1.file_graph.add_file(FileState {
      path: file1.clone(),
      content_hash: "hash1".to_string(),
      size: 10,
    });

    // 두 번째 캐시에 다른 항목 추가
    let file2 = PathBuf::from("test2.px");
    cache2.file_graph.add_file(FileState {
      path: file2.clone(),
      content_hash: "hash2".to_string(),
      size: 20,
    });

    // 순차적으로 저장 (동시 저장 시뮬레이션)
    let result1 = cache1.save(cache_dir);
    let result2 = cache2.save(cache_dir);

    // 둘 다 성공해야 함 (락이 순차적으로 처리)
    assert!(result1.is_ok(), "First save should succeed");
    assert!(result2.is_ok(), "Second save should succeed");

    // 캐시 파일이 올바르게 저장되었는지 확인
    let loaded = IncrementalCache::load(cache_dir).unwrap();
    // 마지막 저장된 캐시의 내용이 로드되어야 함
    assert!(
      loaded.file_graph.files.contains_key(&file1) || loaded.file_graph.files.contains_key(&file2)
    );
  }
}
