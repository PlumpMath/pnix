//! 프로젝트 의존성 해석
//!
//! Y10b: 로컬 경로 의존성 해석 + 의존성 그래프 구축 + 빌드 순서 결정

use crate::project::manifest::PnixManifest;
use crate::project::parser::{find_manifest, load_manifest, ManifestError};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};

/// 의존성 해석 에러
#[derive(Debug)]
pub enum ResolverError {
  Manifest(ManifestError),

  CircularDependency(Vec<String>),

  DependencyNotFound(String),

  InvalidDependencyPath(String),

  ResolutionFailed(String),
}

impl fmt::Display for ResolverError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Manifest(error) => write!(f, "Manifest error: {error}"),
      Self::CircularDependency(cycle) => {
        write!(f, "Circular dependency detected: {cycle:?}")
      }
      Self::DependencyNotFound(name) => write!(f, "Dependency not found: {name}"),
      Self::InvalidDependencyPath(path) => write!(f, "Invalid dependency path: {path}"),
      Self::ResolutionFailed(message) => write!(f, "Failed to resolve dependency: {message}"),
    }
  }
}

impl Error for ResolverError {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    match self {
      Self::Manifest(error) => Some(error),
      _ => None,
    }
  }
}

impl From<ManifestError> for ResolverError {
  fn from(error: ManifestError) -> Self {
    Self::Manifest(error)
  }
}

/// 프로젝트 정보 (매니페스트 + 경로)
#[derive(Debug, Clone)]
pub struct ProjectInfo {
  /// 프로젝트 이름
  pub name: String,

  /// 프로젝트 매니페스트
  pub manifest: PnixManifest,

  /// 프로젝트 루트 디렉토리
  #[allow(dead_code)] // 향후 사용 예정
  pub root: PathBuf,

  /// 매니페스트 파일 경로
  pub manifest_path: PathBuf,
}

/// 의존성 그래프
#[derive(Debug, Clone)]
pub struct DependencyGraph {
  /// 프로젝트 이름 → 프로젝트 정보
  pub projects: HashMap<String, ProjectInfo>,

  /// 프로젝트 이름 → 의존하는 프로젝트 이름들
  pub dependencies: HashMap<String, HashSet<String>>,

  /// 프로젝트 이름 → 의존되는 프로젝트 이름들
  pub dependents: HashMap<String, HashSet<String>>,
}

/// 빌드 순서 (위상 정렬 결과)
#[derive(Debug, Clone)]
pub struct BuildOrder {
  /// 빌드 순서 (의존성 순서대로)
  pub order: Vec<String>,
}

impl Default for DependencyGraph {
  fn default() -> Self {
    Self::new()
  }
}

impl DependencyGraph {
  /// 새 의존성 그래프 생성
  pub fn new() -> Self {
    Self {
      projects: HashMap::new(),
      dependencies: HashMap::new(),
      dependents: HashMap::new(),
    }
  }

  /// 프로젝트 추가
  pub fn add_project(&mut self, project: ProjectInfo) {
    let name = project.name.clone();
    self.projects.insert(name.clone(), project);
    self.dependencies.insert(name.clone(), HashSet::new());
    self.dependents.insert(name, HashSet::new());
  }

  /// 의존성 추가
  pub fn add_dependency(&mut self, from: &str, to: &str) -> Result<(), ResolverError> {
    // 프로젝트 존재 확인
    if !self.projects.contains_key(from) {
      return Err(ResolverError::DependencyNotFound(from.to_string()));
    }
    if !self.projects.contains_key(to) {
      return Err(ResolverError::DependencyNotFound(to.to_string()));
    }

    // 자기 자신 의존성 방지
    if from == to {
      return Err(ResolverError::CircularDependency(vec![from.to_string()]));
    }

    // 의존성 추가
    self
      .dependencies
      .entry(from.to_string())
      .or_default()
      .insert(to.to_string());

    self
      .dependents
      .entry(to.to_string())
      .or_default()
      .insert(from.to_string());

    Ok(())
  }

  /// 위상 정렬로 빌드 순서 계산
  pub fn build_order(&self) -> Result<BuildOrder, ResolverError> {
    // Kahn's algorithm

    // 1. in-degree 계산 (각 프로젝트가 의존하는 프로젝트 수)
    // 결정론 보장: 프로젝트 이름을 정렬하여 순서 고정
    let mut project_names: Vec<String> = self.projects.keys().cloned().collect();
    project_names.sort();
    let mut in_degree: HashMap<String, usize> =
      project_names.iter().map(|name| (name.clone(), 0)).collect();

    for (name, deps) in &self.dependencies {
      *in_degree.entry(name.clone()).or_insert(0) = deps.len();
    }

    // 2. in-degree == 0인 프로젝트들로 시작 (의존성 없는 프로젝트)
    // 결정론 보장: 정렬된 프로젝트 이름 순서로 반복
    let mut ready: Vec<String> = project_names
      .iter()
      .filter_map(|name| {
        if in_degree.get(name).copied().unwrap_or(0) == 0 {
          Some(name.clone())
        } else {
          None
        }
      })
      .collect();

    // 결정론적 순서를 위해 정렬 (이미 정렬되어 있지만 명시적으로 유지)
    ready.sort();

    let mut ordered: Vec<String> = Vec::new();

    // 3. 위상 정렬
    while let Some(project) = ready.pop() {
      ordered.push(project.clone());

      // 이 프로젝트에 의존하는 프로젝트들의 in-degree 감소
      if let Some(dependents) = self.dependents.get(&project) {
        // 결정론 보장: HashSet 반복 순서가 비결정적이므로 정렬하여 순서 고정
        let mut sorted_dependents: Vec<String> = dependents.iter().cloned().collect();
        sorted_dependents.sort();
        for dependent in sorted_dependents {
          if let Some(deg) = in_degree.get_mut(&dependent) {
            *deg = deg.saturating_sub(1);
            if *deg == 0 {
              ready.push(dependent.clone());
            }
          }
        }
        // 결정론적 순서 유지
        ready.sort();
      }
    }

    // 4. 모든 프로젝트를 방문했는지 확인 (순환 의존성 검출)
    if ordered.len() != self.projects.len() {
      let ordered_set: HashSet<&String> = ordered.iter().collect();
      // 결정론 보장: 정렬된 프로젝트 이름 순서로 반복
      let mut remaining: Vec<String> = project_names
        .iter()
        .filter(|name| !ordered_set.contains(*name))
        .cloned()
        .collect();
      remaining.sort(); // 결정론적 순서 보장
      return Err(ResolverError::CircularDependency(remaining));
    }

    Ok(BuildOrder { order: ordered })
  }

  /// 순환 의존성 검출
  pub fn detect_cycles(&self) -> Vec<Vec<String>> {
    let mut cycles = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();
    let mut rec_stack: HashSet<String> = HashSet::new();
    let mut path: Vec<String> = Vec::new();

    // 결정론 보장: 프로젝트 이름을 정렬하여 순서 고정
    let mut project_names: Vec<String> = self.projects.keys().cloned().collect();
    project_names.sort();

    for project_name in &project_names {
      if !visited.contains(project_name) {
        self.dfs_cycle(
          project_name,
          &mut visited,
          &mut rec_stack,
          &mut path,
          &mut cycles,
        );
      }
    }

    cycles
  }

  fn dfs_cycle(
    &self,
    project: &str,
    visited: &mut HashSet<String>,
    rec_stack: &mut HashSet<String>,
    path: &mut Vec<String>,
    cycles: &mut Vec<Vec<String>>,
  ) {
    visited.insert(project.to_string());
    rec_stack.insert(project.to_string());
    path.push(project.to_string());

    if let Some(deps) = self.dependencies.get(project) {
      // 결정론 보장: HashSet 반복 순서가 비결정적이므로 정렬하여 순서 고정
      let mut sorted_deps: Vec<String> = deps.iter().cloned().collect();
      sorted_deps.sort();
      for dep in sorted_deps {
        if !visited.contains(&dep) {
          self.dfs_cycle(&dep, visited, rec_stack, path, cycles);
        } else if rec_stack.contains(&dep) {
          // 순환 발견: path에서 dep 위치 찾기
          // 안전성: rec_stack.contains(dep)가 true이므로 path에 dep이 있어야 함
          if let Some(cycle_start) = path.iter().position(|p| p == &dep) {
            cycles.push(path[cycle_start..].to_vec());
          } else {
            // 예상치 못한 경우: rec_stack에는 있지만 path에는 없음 (버그)
            // 순환은 감지했지만 경로를 구성할 수 없음 - 빈 순환으로 처리
            cycles.push(vec![dep.clone()]);
          }
        }
      }
    }

    rec_stack.remove(project);
    path.pop();
  }
}

/// 프로젝트 의존성 해석기
pub struct DependencyResolver {
  /// 프로젝트 루트 디렉토리
  #[allow(dead_code)] // 향후 사용 예정
  root: PathBuf,
}

impl DependencyResolver {
  /// 새 해석기 생성
  pub fn new(root: impl Into<PathBuf>) -> Self {
    Self { root: root.into() }
  }

  /// 프로젝트와 모든 의존성 해석
  pub fn resolve(&self, start_project: &Path) -> Result<DependencyGraph, ResolverError> {
    let mut graph = DependencyGraph::new();
    let mut resolved: HashSet<PathBuf> = HashSet::new();

    // 시작 프로젝트부터 재귀적으로 해석
    self.resolve_recursive(start_project, &mut graph, &mut resolved)?;

    Ok(graph)
  }

  #[allow(clippy::only_used_in_recursion)]
  fn resolve_recursive(
    &self,
    project_path: &Path,
    graph: &mut DependencyGraph,
    resolved: &mut HashSet<PathBuf>,
  ) -> Result<(), ResolverError> {
    // 정규화된 경로
    let canonical_path = project_path.canonicalize().map_err(|e| {
      ResolverError::InvalidDependencyPath(format!(
        "Failed to canonicalize path {}: {}",
        project_path.display(),
        e
      ))
    })?;

    // 이미 해석된 프로젝트는 건너뛰기
    if resolved.contains(&canonical_path) {
      return Ok(());
    }

    // 매니페스트 찾기
    let manifest_path = find_manifest(&canonical_path).ok_or_else(|| {
      ResolverError::Manifest(ManifestError::NotFound(canonical_path.join("pnix.toml")))
    })?;

    // 매니페스트 로드
    let manifest = load_manifest(&manifest_path)?;

    // 프로젝트 정보 생성
    let project_info = ProjectInfo {
      name: manifest.name.clone(),
      manifest: manifest.clone(),
      root: canonical_path.clone(),
      manifest_path: manifest_path.clone(),
    };

    // 그래프에 추가
    graph.add_project(project_info);
    resolved.insert(canonical_path.clone());

    // 의존성 해석
    for dep in &manifest.dependencies {
      if let Some(ref dep_path) = dep.path {
        // 로컬 경로 의존성
        let dep_absolute_path = if dep_path.is_absolute() {
          dep_path.clone()
        } else {
          canonical_path.join(dep_path)
        };

        // 의존성 프로젝트 재귀 해석
        self.resolve_recursive(&dep_absolute_path, graph, resolved)?;

        // 의존성 그래프에 추가
        let dep_manifest_path = find_manifest(&dep_absolute_path).ok_or_else(|| {
          ResolverError::DependencyNotFound(format!(
            "Dependency manifest not found: {}",
            dep_absolute_path.display()
          ))
        })?;

        let dep_manifest = load_manifest(&dep_manifest_path)?;

        graph.add_dependency(&manifest.name, &dep_manifest.name)?;
      }
      // Git 의존성은 향후 지원 (Y10b는 로컬 경로만)
    }

    Ok(())
  }
}
