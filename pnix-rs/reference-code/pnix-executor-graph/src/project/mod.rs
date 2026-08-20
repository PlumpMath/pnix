//! 프로젝트 매니페스트 파싱 및 관리
//!
//! Y10a: `pnix.toml` 또는 `pnix.nix` 형식의 프로젝트 설정 파일 파싱
//! Y10b: 의존성 해석 + 빌드 순서 결정
//! Y10c: 락 파일 생성 및 검증

mod lock;
mod manifest;
mod parser;
mod resolver;

pub use lock::{
  compute_dependency_hash, compute_project_hash, generate_lock_file, verify_lock_file,
  DependencyLock, GitLock, LockError, LockFile,
};
pub use manifest::{Dependency, PnixManifest};
pub use parser::{find_manifest, load_manifest, load_manifest_from_cwd, ManifestError};
pub use resolver::{BuildOrder, DependencyGraph, DependencyResolver, ProjectInfo, ResolverError};
