//! 경로 보안 유틸리티
//!
//! 경로 탈출(path traversal) 공격을 방지하는 공통 검증 함수들
//!
//! NOTE: 이 모듈은 경로 보안 검증을 위해 `std::io::Error`를 사용합니다.
//! boundary check에서 예외 처리되어야 합니다 (경로 보안은 핵심 기능).

use std::path::{Path, PathBuf};
// NOTE: 경로 보안 검증을 위해 io::Error 필요 (boundary check 예외)
use crate::diagnostics::PnixError;
#[allow(clippy::wildcard_imports)]
use std::io;

/// 경로 검증 에러 타입: 경로 보안 검증 관련 에러 타입
#[derive(Debug, thiserror::Error)]
pub enum PathSecurityError {
  #[error("failed to canonicalize base directory: {0}")]
  CanonicalizeBase(#[source] io::Error),
  #[error("failed to canonicalize path: {0}")]
  CanonicalizePath(#[source] io::Error),
  #[error("path traversal detected: {path} is outside base directory {base}")]
  PathTraversal { path: PathBuf, base: PathBuf },
}

pub type Result<T> = std::result::Result<T, PathSecurityError>;

impl From<io::Error> for PnixError {
  fn from(e: io::Error) -> Self {
    PnixError::Io(e.to_string())
  }
}

/// 경로가 base directory 내부에 있는지 검증
///
/// canonicalize 후 `starts_with` 검증을 수행하여 경로 탈출 공격을 방지합니다.
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 검증만, 값 계산 없음
///
/// # 보안
/// - 입력 경로와 base 경로를 모두 canonicalize하여 심볼릭 링크 우회 방지
/// - canonicalize된 경로가 base directory 내부인지 확인
/// - `..` 컴포넌트가 포함되어 있어도 정규화 후 검증하므로 우회 불가
///
/// # 예제
/// ```no_run
/// use std::path::PathBuf;
/// use pnix_core::utils::path_security::verify_path_within_base;
///
/// let base = PathBuf::from("/safe/base");
/// let path = PathBuf::from("/safe/base/sub/file.txt");
/// assert!(verify_path_within_base(&path, &base).is_ok());
///
/// let malicious = PathBuf::from("/safe/base/../../../etc/passwd");
/// assert!(verify_path_within_base(&malicious, &base).is_err());
/// ```
pub fn verify_path_within_base(path: &Path, base: &Path) -> Result<PathBuf> {
  // Canonicalize both paths to resolve symlinks and normalize
  let canonical_base = base
    .canonicalize()
    .map_err(PathSecurityError::CanonicalizeBase)?;

  let canonical_path = path
    .canonicalize()
    .map_err(PathSecurityError::CanonicalizePath)?;

  // Security: Verify canonicalized path is within base directory
  if !canonical_path.starts_with(&canonical_base) {
    return Err(PathSecurityError::PathTraversal {
      path: canonical_path,
      base: canonical_base,
    });
  }

  Ok(canonical_path)
}

/// 경로 문자열에 `..` 컴포넌트가 포함되어 있는지 사전 검사
///
/// canonicalize 전에 빠른 사전 검사로 명백한 경로 탈출 시도를 차단합니다.
/// 단, 이 검사만으로는 충분하지 않으며 `verify_path_within_base`와 함께 사용해야 합니다.
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 검증만, 값 계산 없음
///
/// # 예제
/// ```
/// use pnix_core::utils::path_security::contains_path_traversal;
///
/// assert!(contains_path_traversal("../../../etc/passwd"));
/// assert!(contains_path_traversal("foo/bar/../baz"));
/// assert!(!contains_path_traversal("foo/bar/baz"));
/// assert!(!contains_path_traversal("..config.px")); // substring이지만 경로 컴포넌트 아님
/// ```
pub fn contains_path_traversal(path: &str) -> bool {
  // Check for ".." as an actual path component across platforms.
  Path::new(path)
    .components()
    .any(|component| matches!(component, std::path::Component::ParentDir))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_contains_path_traversal() {
    assert!(contains_path_traversal("../../../etc/passwd"));
    assert!(contains_path_traversal("foo/bar/../baz"));
    assert!(contains_path_traversal("../file.txt"));
    assert!(contains_path_traversal("file/.."));
    assert!(!contains_path_traversal("foo/bar/baz"));
    assert!(!contains_path_traversal("..config.px")); // substring but not component
  }

  #[test]
  fn test_pnix_error_from_io_error() {
    let io_err = io::Error::other("disk failure");
    let pnix_err: PnixError = io_err.into();
    assert!(matches!(pnix_err, PnixError::Io(_)));
    assert!(pnix_err.to_string().contains("disk failure"));
  }

  // Note: Integration tests for verify_path_within_base require filesystem access
  // and are tested in pnix-executor-graph and pnix-compat integration tests
}
