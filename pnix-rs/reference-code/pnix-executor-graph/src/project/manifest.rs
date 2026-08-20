//! Pnix 프로젝트 매니페스트 구조체 정의
//!
//! Y10a: 프로젝트 매니페스트 필드 정의

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Pnix 프로젝트 매니페스트
///
/// `pnix.toml` 파일의 구조를 정의합니다.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PnixManifest {
  /// 프로젝트 이름
  pub name: String,

  /// 프로젝트 버전
  pub version: String,

  /// 프로젝트 설명 (선택적)
  #[serde(default)]
  pub description: Option<String>,

  /// 프로젝트 라이선스 (선택적)
  #[serde(default)]
  pub license: Option<String>,

  /// 프로젝트 저장소 URL (선택적)
  #[serde(default)]
  pub repository: Option<String>,

  /// 프로젝트 작성자 (선택적)
  #[serde(default)]
  pub authors: Vec<String>,

  /// 의존성 목록
  #[serde(default, deserialize_with = "deserialize_dependencies")]
  pub dependencies: Vec<Dependency>,
}

/// 프로젝트 의존성
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dependency {
  /// 의존성 이름
  pub name: String,

  /// 의존성 버전 (선택적)
  #[serde(default)]
  pub version: Option<String>,

  /// 로컬 경로 의존성 (`path = "../lib"`)
  #[serde(default)]
  pub path: Option<PathBuf>,

  /// Git 저장소 의존성 (선택적)
  #[serde(default)]
  pub git: Option<String>,

  /// Git 브랜치/태그 (선택적)
  #[serde(default)]
  pub branch: Option<String>,

  /// Git 태그 (선택적)
  #[serde(default)]
  pub tag: Option<String>,

  /// Git 커밋 해시 (선택적)
  #[serde(default)]
  pub rev: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum DependenciesRepr {
  List(Vec<Dependency>),
  Map(BTreeMap<String, DependencySpec>),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum DependencySpec {
  Simple(String),
  Detailed(DependencyDetail),
}

#[derive(Debug, Clone, Deserialize)]
struct DependencyDetail {
  #[serde(default)]
  version: Option<String>,
  #[serde(default)]
  path: Option<PathBuf>,
  #[serde(default)]
  git: Option<String>,
  #[serde(default)]
  branch: Option<String>,
  #[serde(default)]
  tag: Option<String>,
  #[serde(default)]
  rev: Option<String>,
}

fn deserialize_dependencies<'de, D>(deserializer: D) -> Result<Vec<Dependency>, D::Error>
where
  D: serde::Deserializer<'de>,
{
  let repr = DependenciesRepr::deserialize(deserializer)?;
  let deps = match repr {
    DependenciesRepr::List(items) => items,
    DependenciesRepr::Map(map) => map
      .into_iter()
      .map(|(name, spec)| spec.into_dependency(name))
      .collect(),
  };
  Ok(deps)
}

impl DependencySpec {
  fn into_dependency(self, name: String) -> Dependency {
    match self {
      DependencySpec::Simple(version) => Dependency {
        name,
        version: Some(version),
        path: None,
        git: None,
        branch: None,
        tag: None,
        rev: None,
      },
      DependencySpec::Detailed(detail) => Dependency {
        name,
        version: detail.version,
        path: detail.path,
        git: detail.git,
        branch: detail.branch,
        tag: detail.tag,
        rev: detail.rev,
      },
    }
  }
}

impl PnixManifest {
  /// 기본 매니페스트 생성
  pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
    Self {
      name: name.into(),
      version: version.into(),
      description: None,
      license: None,
      repository: None,
      authors: Vec::new(),
      dependencies: Vec::new(),
    }
  }

  /// 의존성 추가
  pub fn add_dependency(&mut self, dep: Dependency) {
    self.dependencies.push(dep);
  }
}

impl Dependency {
  /// 로컬 경로 의존성 생성
  pub fn path(name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
    Self {
      name: name.into(),
      version: None,
      path: Some(path.into()),
      git: None,
      branch: None,
      tag: None,
      rev: None,
    }
  }

  /// 버전 의존성 생성
  pub fn version(name: impl Into<String>, version: impl Into<String>) -> Self {
    Self {
      name: name.into(),
      version: Some(version.into()),
      path: None,
      git: None,
      branch: None,
      tag: None,
      rev: None,
    }
  }

  /// Git 의존성 생성
  pub fn git(name: impl Into<String>, git: impl Into<String>) -> Self {
    Self {
      name: name.into(),
      version: None,
      path: None,
      git: Some(git.into()),
      branch: None,
      tag: None,
      rev: None,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::PnixManifest;
  use std::path::PathBuf;

  #[test]
  fn parse_dependencies_list() {
    let manifest: PnixManifest = toml::from_str(
      r#"
name = "app"
version = "0.1.0"

[[dependencies]]
name = "lib"
path = "../lib"
"#,
    )
    .expect("manifest should parse");

    assert_eq!(manifest.dependencies.len(), 1);
    let dep = &manifest.dependencies[0];
    assert_eq!(dep.name, "lib");
    assert_eq!(dep.path.as_ref(), Some(&PathBuf::from("../lib")));
  }

  #[test]
  fn parse_dependencies_map() {
    let manifest: PnixManifest = toml::from_str(
      r#"
name = "app"
version = "0.1.0"

[dependencies]
bar = { path = "../bar" }
foo = "1.2.3"
"#,
    )
    .expect("manifest should parse");

    assert_eq!(manifest.dependencies.len(), 2);
    assert_eq!(manifest.dependencies[0].name, "bar");
    assert_eq!(
      manifest.dependencies[0].path.as_ref(),
      Some(&PathBuf::from("../bar"))
    );
    assert_eq!(manifest.dependencies[1].name, "foo");
    assert_eq!(manifest.dependencies[1].version.as_deref(), Some("1.2.3"));
  }
}
