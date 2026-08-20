//! 프로젝트 락 파일 생성 및 검증
//!
//! Y10c: `pnix.lock` 생성 (결정론적 빌드)

use crate::project::manifest::PnixManifest;
use crate::project::resolver::{DependencyGraph, ProjectInfo};
use serde::{Deserialize, Serialize};
use pnix_hash::{Digest, Sha256};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

/// 락 파일 에러
#[derive(Debug)]
pub enum LockError {
  IoError(std::io::Error),

  ParseError(toml::de::Error),

  SerializeError(toml::ser::Error),

  Manifest(crate::project::parser::ManifestError),

  Mismatch(String),

  InvalidLock(String),
}

impl fmt::Display for LockError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::IoError(error) => write!(f, "Failed to read lock file: {error}"),
      Self::ParseError(error) => write!(f, "Failed to parse lock file: {error}"),
      Self::SerializeError(error) => write!(f, "Failed to serialize lock file: {error}"),
      Self::Manifest(error) => write!(f, "Manifest error: {error}"),
      Self::Mismatch(message) => write!(f, "Lock file mismatch: {message}"),
      Self::InvalidLock(message) => write!(f, "Invalid lock file: {message}"),
    }
  }
}

impl Error for LockError {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    match self {
      Self::IoError(error) => Some(error),
      Self::ParseError(error) => Some(error),
      Self::SerializeError(error) => Some(error),
      Self::Manifest(error) => Some(error),
      _ => None,
    }
  }
}

impl From<std::io::Error> for LockError {
  fn from(error: std::io::Error) -> Self {
    Self::IoError(error)
  }
}

impl From<toml::de::Error> for LockError {
  fn from(error: toml::de::Error) -> Self {
    Self::ParseError(error)
  }
}

impl From<toml::ser::Error> for LockError {
  fn from(error: toml::ser::Error) -> Self {
    Self::SerializeError(error)
  }
}

impl From<crate::project::parser::ManifestError> for LockError {
  fn from(error: crate::project::parser::ManifestError) -> Self {
    Self::Manifest(error)
  }
}

/// 락 파일 구조
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LockFile {
  /// 락 파일 버전
  pub version: String,

  /// 프로젝트 해시
  pub project_hash: String,

  /// 의존성 해시 (이름 → 해시)
  #[serde(default)]
  pub dependencies: BTreeMap<String, DependencyLock>,
}

/// 의존성 락 정보
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DependencyLock {
  /// 의존성 해시
  pub hash: String,

  /// 의존성 경로 (로컬 경로인 경우)
  #[serde(default)]
  pub path: Option<PathBuf>,

  /// 의존성 버전 (버전 의존성인 경우)
  #[serde(default)]
  pub version: Option<String>,

  /// Git 정보 (Git 의존성인 경우)
  #[serde(default)]
  pub git: Option<GitLock>,
}

/// Git 의존성 락 정보
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GitLock {
  /// Git 저장소 URL
  pub url: String,

  /// Git 브랜치/태그/rev
  #[serde(default)]
  pub branch: Option<String>,

  #[serde(default)]
  pub tag: Option<String>,

  #[serde(default)]
  pub rev: Option<String>,
}

impl LockFile {
  /// 새 락 파일 생성
  pub fn new(project_hash: String) -> Self {
    Self {
      version: "1.0".to_string(),
      project_hash,
      dependencies: BTreeMap::new(),
    }
  }

  /// 의존성 추가
  pub fn add_dependency(&mut self, name: String, dep_lock: DependencyLock) {
    self.dependencies.insert(name, dep_lock);
  }

  /// 락 파일 저장
  pub fn save(&self, path: &Path) -> Result<(), LockError> {
    let content = toml::to_string_pretty(self)?;
    fs::write(path, content)?;
    Ok(())
  }

  /// 락 파일 로드
  pub fn load(path: &Path) -> Result<Self, LockError> {
    if !path.exists() {
      return Err(LockError::IoError(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("Lock file not found: {}", path.display()),
      )));
    }

    let content = fs::read_to_string(path)?;
    let lock: LockFile = toml::from_str(&content)?;
    Ok(lock)
  }

  /// 락 파일 검증 (현재 프로젝트 해시와 비교)
  pub fn verify(&self, current_project_hash: &str) -> Result<(), LockError> {
    if self.project_hash != current_project_hash {
      return Err(LockError::Mismatch(format!(
        "Project hash mismatch: expected {}, got {}",
        self.project_hash, current_project_hash
      )));
    }
    Ok(())
  }
}

/// 프로젝트 해시 계산
///
/// 매니페스트 내용과 소스 파일 해시를 결합하여 결정론적 해시 생성
pub fn compute_project_hash(
  manifest: &PnixManifest,
  project_root: &Path,
) -> Result<String, LockError> {
  let mut hasher = Sha256::new();

  // 1. 매니페스트 내용 해시 (정규화된 TOML)
  let manifest_toml = toml::to_string(manifest).map_err(LockError::SerializeError)?;
  hasher.update(manifest_toml.as_bytes());

  // 2. 소스 파일 해시 (`.px`, `.sam` 파일)
  let source_files = find_source_files(project_root)?;
  let mut source_hashes: Vec<(String, String)> = source_files
    .iter()
    .map(|path| {
      // CRITICAL: 파일 읽기 실패 시 에러 전파 (빈 문자열로 대체하지 않음)
      let content = fs::read_to_string(path).map_err(|e| {
        LockError::IoError(std::io::Error::new(
          e.kind(),
          format!("Failed to read source file {}: {}", path.display(), e),
        ))
      })?;
      let mut file_hasher = Sha256::new();
      file_hasher.update(content.as_bytes());
      let hash = format!("{:x}", file_hasher.finalize());

      // 상대 경로 사용 (결정론적)
      let rel_path = path
        .strip_prefix(project_root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();
      Ok((rel_path, hash))
    })
    .collect::<Result<Vec<_>, LockError>>()?;

  // 정렬 (결정론적 순서)
  source_hashes.sort_by_key(|(path, _)| path.clone());

  for (path, hash) in source_hashes {
    hasher.update(path.as_bytes());
    hasher.update(hash.as_bytes());
  }

  Ok(format!("{:x}", hasher.finalize()))
}

/// 의존성 해시 계산
pub fn compute_dependency_hash(project_info: &ProjectInfo) -> Result<String, LockError> {
  compute_project_hash(&project_info.manifest, &project_info.root)
}

/// 소스 파일 찾기 (`.px`, `.sam` 파일)
fn find_source_files(root: &Path) -> Result<Vec<PathBuf>, LockError> {
  let mut files = Vec::new();

  if !root.exists() {
    return Ok(files);
  }

  let entries = fs::read_dir(root)?;

  for entry in entries {
    let entry = entry?;
    let path = entry.path();

    if path.is_dir() {
      // 재귀적으로 탐색 (하위 디렉토리 제외: dist, target, .git 등)
      let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

      if !dir_name.starts_with('.') && dir_name != "dist" && dir_name != "target" {
        let mut sub_files = find_source_files(&path)?;
        files.append(&mut sub_files);
      }
    } else if let Some(ext) = path.extension() {
      if ext == "px" || ext == "sam" {
        files.push(path);
      }
    }
  }

  Ok(files)
}

/// 락 파일 생성
pub fn generate_lock_file(
  graph: &DependencyGraph,
  project_root: &Path,
) -> Result<LockFile, LockError> {
  // 프로젝트 루트에서 매니페스트 찾기
  let manifest_path = crate::project::parser::find_manifest(project_root).ok_or_else(|| {
    LockError::InvalidLock(format!(
      "Manifest not found in project root: {}",
      project_root.display()
    ))
  })?;

  let manifest = crate::project::parser::load_manifest(&manifest_path)?;

  // 프로젝트 정보 찾기
  let project_info = graph.projects.get(&manifest.name).ok_or_else(|| {
    LockError::InvalidLock(format!("Project {} not found in graph", manifest.name))
  })?;

  // 프로젝트 해시 계산
  let project_hash = compute_project_hash(&project_info.manifest, &project_info.root)?;

  let mut lock = LockFile::new(project_hash);

  // 의존성 해시 계산 (자기 자신 제외)
  for (name, dep_info) in &graph.projects {
    if name == &manifest.name {
      continue; // 자기 자신은 제외
    }

    let dep_hash = compute_dependency_hash(dep_info)?;

    let dep_lock = DependencyLock {
      hash: dep_hash,
      path: dep_info
        .manifest_path
        .parent()
        .and_then(|p| p.strip_prefix(project_root).ok())
        .map(|p| p.to_path_buf()),
      version: Some(dep_info.manifest.version.clone()),
      git: None, // Git 의존성은 향후 지원
    };

    lock.add_dependency(name.clone(), dep_lock);
  }

  Ok(lock)
}

/// 락 파일 검증 (의존성 그래프와 비교)
pub fn verify_lock_file(
  lock: &LockFile,
  graph: &DependencyGraph,
  project_root: &Path,
) -> Result<(), LockError> {
  // 프로젝트 루트에서 매니페스트 찾기
  let manifest_path = crate::project::parser::find_manifest(project_root).ok_or_else(|| {
    LockError::InvalidLock(format!(
      "Manifest not found in project root: {}",
      project_root.display()
    ))
  })?;

  let manifest = crate::project::parser::load_manifest(&manifest_path)?;

  // 프로젝트 정보 찾기
  let project_info = graph.projects.get(&manifest.name).ok_or_else(|| {
    LockError::InvalidLock(format!("Project {} not found in graph", manifest.name))
  })?;

  // 프로젝트 해시 검증
  let current_project_hash = compute_project_hash(&project_info.manifest, &project_info.root)?;
  lock.verify(&current_project_hash)?;

  // 의존성 해시 검증 (자기 자신 제외)
  for (name, dep_info) in &graph.projects {
    if name == &manifest.name {
      continue;
    }

    let dep_lock = lock
      .dependencies
      .get(name)
      .ok_or_else(|| LockError::Mismatch(format!("Dependency {} not found in lock file", name)))?;

    let current_dep_hash = compute_dependency_hash(dep_info)?;

    if dep_lock.hash != current_dep_hash {
      return Err(LockError::Mismatch(format!(
        "Dependency {} hash mismatch: expected {}, got {}",
        name, dep_lock.hash, current_dep_hash
      )));
    }
  }

  Ok(())
}
