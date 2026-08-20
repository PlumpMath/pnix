//! Pnix 프로젝트 매니페스트 파서
//!
//! Y10a: `pnix.toml` 파일 파싱

use crate::project::manifest::PnixManifest;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

/// 매니페스트 파싱 에러
#[derive(Debug)]
pub enum ManifestError {
  NotFound(PathBuf),

  IoError(std::io::Error),

  ParseError(toml::de::Error),

  InvalidManifest(String),
}

impl fmt::Display for ManifestError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::NotFound(path) => write!(f, "Manifest file not found: {}", path.display()),
      Self::IoError(error) => write!(f, "Failed to read manifest file: {error}"),
      Self::ParseError(error) => write!(f, "Failed to parse TOML: {error}"),
      Self::InvalidManifest(message) => write!(f, "Invalid manifest: {message}"),
    }
  }
}

impl Error for ManifestError {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    match self {
      Self::IoError(error) => Some(error),
      Self::ParseError(error) => Some(error),
      _ => None,
    }
  }
}

impl From<std::io::Error> for ManifestError {
  fn from(error: std::io::Error) -> Self {
    Self::IoError(error)
  }
}

impl From<toml::de::Error> for ManifestError {
  fn from(error: toml::de::Error) -> Self {
    Self::ParseError(error)
  }
}

/// 프로젝트 루트에서 매니페스트 파일 찾기
///
/// 현재 디렉토리부터 상위로 올라가며 `pnix.toml` 또는 `pnix.nix` 파일을 찾습니다.
/// MEDIUM: find_manifest가 루트까지 검색 수정 완료
/// 홈 디렉토리 내부에서 시작했을 때만 홈 경계까지 검색하여
/// 공유 시스템에서 악의적 pnix.toml 발견 방지
pub fn find_manifest(start_dir: &Path) -> Option<PathBuf> {
  let mut current = start_dir.to_path_buf();

  // 홈 디렉토리 경로 가져오기 (검색 상한선)
  let home_dir = std::env::var_os("HOME").map(PathBuf::from).or({
    // Windows에서 HOME이 없으면 USERPROFILE 사용
    #[cfg(windows)]
    {
      std::env::var_os("USERPROFILE").map(PathBuf::from)
    }
    #[cfg(not(windows))]
    {
      None
    }
  });

  let within_home = home_dir
    .as_ref()
    .map_or(false, |home| current.starts_with(home));

  // 최대 검색 깊이 제한 (보안: 무한 루프 방지)
  const MAX_SEARCH_DEPTH: usize = 100;
  let mut depth = 0;

  loop {
    // 깊이 제한 체크
    if depth >= MAX_SEARCH_DEPTH {
      break;
    }
    depth += 1;

    // pnix.toml 확인
    let toml_path = current.join("pnix.toml");
    if toml_path.exists() {
      return Some(toml_path);
    }

    // pnix.nix 확인 (향후 지원)
    // let nix_path = current.join("pnix.nix");
    // if nix_path.exists() {
    //   return Some(nix_path);
    // }

    // 상위 디렉토리로 이동
    match current.parent() {
      Some(parent) => {
        // 홈 디렉토리 내부에서 시작한 경우에만 홈 경계 적용
        if within_home {
          if let Some(ref home) = home_dir {
            if parent == home.as_path() || !parent.starts_with(home) {
              // 홈 디렉토리에 도달했거나 홈 디렉토리 밖으로 나갔으면 중단
              break;
            }
          }
        }
        // 루트 디렉토리에 도달하면 중단 (보안: 시스템 루트 접근 방지)
        if parent == Path::new("/") || parent == Path::new("\\") {
          break;
        }
        current = parent.to_path_buf();
      }
      None => break,
    }
  }

  None
}

/// 매니페스트 파일 로드
///
/// `pnix.toml` 파일을 읽어서 `PnixManifest`로 파싱합니다.
pub fn load_manifest(manifest_path: &Path) -> Result<PnixManifest, ManifestError> {
  // 파일 존재 확인
  if !manifest_path.exists() {
    return Err(ManifestError::NotFound(manifest_path.to_path_buf()));
  }

  // 파일 읽기
  let content = fs::read_to_string(manifest_path).map_err(ManifestError::IoError)?;

  // TOML 파싱
  let manifest: PnixManifest = toml::from_str(&content).map_err(ManifestError::ParseError)?;

  // 기본 검증
  if manifest.name.is_empty() {
    return Err(ManifestError::InvalidManifest(
      "manifest.name cannot be empty".to_string(),
    ));
  }

  if manifest.version.is_empty() {
    return Err(ManifestError::InvalidManifest(
      "manifest.version cannot be empty".to_string(),
    ));
  }

  // 경로 정규화 (workspace 기준)
  let manifest_dir = manifest_path.parent().ok_or_else(|| {
    ManifestError::InvalidManifest("Manifest path has no parent directory".to_string())
  })?;

  let mut normalized_manifest = manifest;

  // 의존성 경로 정규화
  for dep in &mut normalized_manifest.dependencies {
    if let Some(ref path) = dep.path {
      // 상대 경로를 절대 경로로 변환
      let absolute_path = if path.is_relative() {
        manifest_dir.join(path)
      } else {
        path.clone()
      };

      // 정규화 (.. 제거 등)
      // 경로가 존재하지 않아도 정규화 시도 (의존성이 아직 빌드되지 않았을 수 있음)
      dep.path = Some(
        absolute_path.canonicalize().unwrap_or(absolute_path), // 존재하지 않으면 원본 경로 사용
      );
    }
  }

  Ok(normalized_manifest)
}

/// 현재 작업 디렉토리에서 매니페스트 찾기 및 로드
pub fn load_manifest_from_cwd() -> Result<PnixManifest, ManifestError> {
  let cwd = std::env::current_dir().map_err(ManifestError::IoError)?;

  let manifest_path =
    find_manifest(&cwd).ok_or_else(|| ManifestError::NotFound(cwd.join("pnix.toml")))?;

  load_manifest(&manifest_path)
}
