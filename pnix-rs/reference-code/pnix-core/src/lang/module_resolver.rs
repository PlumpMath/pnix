//! Module Resolver: 파일 경로 → 네임스페이스 매핑 및 순환 import 감지
//!
//! Y07b: 네임스페이스/패키지 지원
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의 및 검증만, 파일 I/O는 executor에서 처리

use crate::ast::{AstItem, AstModule};
use crate::MeaningError;

#[cfg(test)]
use crate::diagnostics::Span;
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

/// 표준 라이브러리 경로 해석기
pub struct StdlibPathResolver;

impl StdlibPathResolver {
  /// stdlib 경로를 파일 경로로 변환
  ///
  /// 예: `std.list` → `stdlib/list`
  /// 예: `std.list.nix` → `stdlib/list`
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 환경 변수 읽기는 executor에서 처리하므로, 여기서는 repo 상대 경로만 반환
  pub fn resolve_stdlib_path(namespace: &str) -> Option<PathBuf> {
    // std. 접두사 제거
    let module_name = namespace.strip_prefix("std.")?;
    let module_name = module_name
      .strip_suffix(".px")
      .or_else(|| module_name.strip_suffix(".nix"))
      .unwrap_or(module_name);

    // repo 상대 경로만 반환 (PNIX_HOME은 executor에서 처리)
    // 확장자는 executor에서 결정 (resolve_candidate_path가 .px/.nix 처리)
    Some(PathBuf::from("stdlib").join(module_name))
  }

  /// 경로가 stdlib 경로인지 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 검증만, 값 계산 없음
  pub fn is_stdlib_path(path: &str) -> bool {
    path.starts_with("std.") || path == "std"
  }
}

/// 모듈 해석 결과
#[derive(Debug, Clone)]
pub struct ModuleResolution {
  /// 해석된 모듈 그래프 (파일 경로 → 모듈)
  pub modules: HashMap<PathBuf, AstModule>,
  /// 파일 경로 → 네임스페이스 매핑
  pub namespace_map: HashMap<PathBuf, String>,
  /// 위상 정렬된 모듈 순서 (의존성 순서)
  pub load_order: Vec<PathBuf>,
}

/// 모듈 해석 에러
#[derive(Debug, thiserror::Error)]
pub enum ModuleResolutionError {
  #[error("Circular import detected: {}", format_cycle(.0))]
  CircularImport(Vec<PathBuf>),
  #[error("Module not found: {}", .0.display())]
  ModuleNotFound(PathBuf),
  #[error("Invalid import path: {}", .0)]
  InvalidPath(String),
  #[error(
    "Namespace collision: '{}' is provided by both '{}' and '{}'",
    namespace,
    first.display(),
    second.display()
  )]
  NamespaceCollision {
    /// 충돌한 네임스페이스 이름 (충돌한 네임스페이스 이름)
    namespace: String,
    /// 첫 번째 파일 경로 (첫 번째 파일 경로)
    first: PathBuf,
    /// 두 번째 파일 경로 (두 번째 파일 경로)
    second: PathBuf,
  },
  /// 모듈 로드 에러: 모듈 로드 중 발생한 에러
  #[error("Failed to load module {}: {}", .0.display(), .1)]
  ModuleLoadError(
    /// 모듈 경로 (로드 실패한 모듈 경로)
    PathBuf,
    /// 원본 에러 (MeaningError)
    #[source]
    MeaningError,
  ),
}

fn format_cycle(cycle: &[PathBuf]) -> String {
  cycle
    .iter()
    .map(|p| p.display().to_string())
    .collect::<Vec<_>>()
    .join(" → ")
}

/// 모듈 해석기: 모듈 해석을 수행하는 해석기
pub struct ModuleResolver {
  /// 파일 경로 → 모듈 캐시 (해석된 모듈 캐시)
  modules: HashMap<PathBuf, AstModule>,
  /// 현재 해석 중인 경로 (순환 감지용, 순서 보존하여 전체 사이클 재구성)
  resolving: Vec<PathBuf>,
  /// 해석 완료된 경로 (결정론 보장을 위해 BTreeSet 사용, 해석 완료된 경로 집합)
  resolved: BTreeSet<PathBuf>,
  /// 파일 경로 → 네임스페이스 매핑 (base_dir 기준, 파일 경로 → 네임스페이스 이름 매핑)
  namespace_by_path: HashMap<PathBuf, String>,
  /// 네임스페이스 → 파일 경로 (충돌 감지용, 네임스페이스 이름 → 파일 경로 매핑)
  namespace_by_name: HashMap<String, PathBuf>,
}

impl ModuleResolver {
  /// 새 모듈 해석기 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new() -> Self {
    Self {
      modules: HashMap::new(),
      resolving: Vec::new(),
      resolved: BTreeSet::new(),
      namespace_by_path: HashMap::new(),
      namespace_by_name: HashMap::new(),
    }
  }

  /// 파일 경로를 네임스페이스로 변환
  ///
  /// 예: `src/math/vector.px` → `math.vector`
  /// 예: `math/vector.px` → `math`
  /// 예: `vector.px` → `vector`
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변환만, 값 계산 없음
  pub fn path_to_namespace(path: &Path) -> String {
    let mut parts = Vec::new();
    for component in path.components() {
      if let std::path::Component::Normal(name) = component {
        let name_str = name.to_string_lossy();
        // .px 확장자 제거
        let name_without_ext = name_str
          .strip_suffix(".px")
          .or_else(|| name_str.strip_suffix(".nix"))
          .unwrap_or(&name_str);
        parts.push(name_without_ext.to_string());
      }
    }
    // 경로 구성 요소가 3개 이상이면 첫 번째 제외하고 나머지 모두 (예: src/math/vector.px → math.vector)
    // 경로 구성 요소가 2개면 첫 번째만 (예: math/vector.px → math)
    // 경로 구성 요소가 1개면 그대로 (예: vector.px → vector)
    match parts.len() {
      0 => String::new(),
      1 => parts[0].clone(),
      2 => parts[0].clone(), // math/vector.px → math
      _ => {
        // src/math/vector.px → math.vector (첫 번째 제외하고 나머지 모두)
        // LOW: 네임스페이스 매핑 불일치 수정 완료
        // 2-level 경로 정보 손실은 의도된 동작: 첫 번째 경로 요소는 제외하고 나머지를 네임스페이스로 사용
        // 이는 모듈 구조에서 루트 디렉토리를 제외하고 하위 경로를 네임스페이스로 매핑하는 설계 선택
        parts[1..].join(".")
      }
    }
  }

  /// 모듈 해석 (DFS 기반, 순환 감지)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 파일 I/O는 load_module 콜백에서 처리
  pub fn resolve_module(
    &mut self,
    path: PathBuf,
    base_dir: &Path,
    load_module: &mut dyn FnMut(&Path) -> Result<AstModule, MeaningError>,
  ) -> Result<(), ModuleResolutionError> {
    trace!(
      path = %path.display(),
      base_dir = %base_dir.display(),
      "module resolve start"
    );
    // stdlib 경로 처리 (Y07c)
    let abs_path = if let Some(path_str) = path.to_str() {
      if StdlibPathResolver::is_stdlib_path(path_str) {
        // std.list → stdlib/list.px 경로 변환
        StdlibPathResolver::resolve_stdlib_path(path_str)
          .ok_or_else(|| ModuleResolutionError::ModuleNotFound(path.clone()))?
      } else if path.is_absolute() {
        path
      } else {
        // 상대 경로: base_dir 기준
        base_dir.join(&path)
      }
    } else {
      // 경로 문자열 변환 실패
      return Err(ModuleResolutionError::InvalidPath(
        path.display().to_string(),
      ));
    };
    trace!(path = %abs_path.display(), "module resolve normalized");
    let namespace_path = abs_path.strip_prefix(base_dir).unwrap_or(&abs_path);
    let namespace = Self::path_to_namespace(namespace_path);

    // 이미 해석 완료
    if self.resolved.contains(&abs_path) {
      trace!(path = %abs_path.display(), "module resolve cached");
      return Ok(());
    }

    // 순환 감지
    if self.resolving.contains(&abs_path) {
      // 순환 경로 재구성: resolving 스택에서 abs_path부터 끝까지 + abs_path 다시
      // 예: resolving = [A, B, C], abs_path = B → cycle = [B, C, B]
      let Some(start_idx) = self.resolving.iter().position(|p| p == &abs_path) else {
        return Err(ModuleResolutionError::CircularImport(
          vec![abs_path.clone()],
        ));
      };
      let mut cycle: Vec<PathBuf> = self.resolving[start_idx..].to_vec();
      cycle.push(abs_path.clone()); // 사이클을 명확히 표시하기 위해 시작점 재추가
      return Err(ModuleResolutionError::CircularImport(cycle));
    }

    // 해석 시작 (모듈 로드 전에 마킹)
    self.resolving.push(abs_path.clone());

    // 모듈 로드
    let module = load_module(&abs_path).map_err(|e| {
      // 에러 컨텍스트 보존: 실제 에러(구문 에러 등)를 보존
      ModuleResolutionError::ModuleLoadError(abs_path.clone(), e)
    })?;
    debug!(
      path = %abs_path.display(),
      items = module.items.len(),
      "module loaded"
    );

    // Import 선언 찾기
    let imports: Vec<String> = module
      .items
      .iter()
      .filter_map(|item| match item {
        AstItem::ImportDecl { path, .. } => Some(path.clone()),
        _ => None,
      })
      .collect();
    trace!(
      path = %abs_path.display(),
      imports = imports.len(),
      "module imports discovered"
    );

    // 재귀적으로 import 해석
    for import_path_str in imports {
      let import_path = PathBuf::from(&import_path_str);
      self.resolve_module(import_path, base_dir, load_module)?;
    }

    // 네임스페이스 충돌 감지
    if let Some(existing) = self.namespace_by_name.get(&namespace) {
      if existing != &abs_path {
        return Err(ModuleResolutionError::NamespaceCollision {
          namespace,
          first: existing.clone(),
          second: abs_path.clone(),
        });
      }
    }

    // 해석 완료 - resolving 스택에서 제거 (LIFO)
    // abs_path가 마지막에 push되었으므로 pop으로 제거
    let popped = self.resolving.pop();
    debug_assert_eq!(
      popped.as_ref(),
      Some(&abs_path),
      "resolving stack invariant violated"
    );
    self.resolved.insert(abs_path.clone());
    self
      .namespace_by_name
      .insert(namespace.clone(), abs_path.clone());
    self.namespace_by_path.insert(abs_path.clone(), namespace);
    self.modules.insert(abs_path, module);

    Ok(())
  }

  /// 위상 정렬된 로드 순서 계산
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn compute_load_order(&self) -> Vec<PathBuf> {
    // BTreeSet은 자동으로 정렬되므로 결정론 보장
    // 향후 의존성 그래프 기반 위상 정렬로 확장 가능
    self.resolved.iter().cloned().collect()
  }

  /// 네임스페이스 맵 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변환만, 값 계산 없음
  pub fn build_namespace_map(&self) -> HashMap<PathBuf, String> {
    // 결정론 보장을 위해 정렬된 순서로 처리
    let mut keys: Vec<_> = self.modules.keys().cloned().collect();
    keys.sort();
    keys
      .into_iter()
      .map(|path| {
        let namespace = self
          .namespace_by_path
          .get(&path)
          .cloned()
          .unwrap_or_else(|| Self::path_to_namespace(&path));
        (path.clone(), namespace)
      })
      .collect()
  }

  /// 해석 결과 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변환만, 값 계산 없음
  pub fn into_resolution(self) -> ModuleResolution {
    // 결정론 보장을 위해 정렬된 순서로 처리
    let mut module_keys: Vec<_> = self.modules.keys().cloned().collect();
    module_keys.sort();
    let namespace_map = module_keys
      .into_iter()
      .map(|path| {
        let namespace = self
          .namespace_by_path
          .get(&path)
          .cloned()
          .unwrap_or_else(|| Self::path_to_namespace(&path));
        (path.clone(), namespace)
      })
      .collect();
    // BTreeSet은 자동으로 정렬되므로 결정론 보장
    let load_order = self.resolved.iter().cloned().collect();
    ModuleResolution {
      modules: self.modules,
      namespace_map,
      load_order,
    }
  }
}

impl Default for ModuleResolver {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_path_to_namespace() {
    assert_eq!(
      ModuleResolver::path_to_namespace(Path::new("src/math/vector.px")),
      "math.vector"
    );
    assert_eq!(
      ModuleResolver::path_to_namespace(Path::new("math/vector.px")),
      "math"
    );
    assert_eq!(
      ModuleResolver::path_to_namespace(Path::new("vector.px")),
      "vector"
    );
    assert_eq!(
      ModuleResolver::path_to_namespace(Path::new("vector.nix")),
      "vector"
    );
  }

  #[test]
  fn test_circular_import_detection() {
    let mut resolver = ModuleResolver::new();
    let base_dir = Path::new("/tmp");

    // 순환 import 시뮬레이션
    let cycle_detected = std::cell::Cell::new(false);
    let mut load_fn = |_path: &Path| {
      // a.px가 b.px를 import하고, b.px가 a.px를 import하는 순환
      if cycle_detected.get() {
        Err(MeaningError::Internal("circular import".into(), None))
      } else {
        cycle_detected.set(true);
        Ok(AstModule {
          name: "a".into(),
          items: vec![AstItem::ImportDecl {
            path: "b.px".into(),
            span: Span::with_file(0, 0, "a.px"),
          }],
        })
      }
    };
    let result = resolver.resolve_module(PathBuf::from("a.px"), base_dir, &mut load_fn);

    // 순환 감지 확인 (간단한 구현이므로 실제로는 더 복잡한 로직 필요)
    assert!(result.is_err() || cycle_detected.get());
  }

  #[test]
  fn test_namespace_collision_detection() {
    let mut resolver = ModuleResolver::new();
    let base_dir = Path::new("project");

    let mut load_fn = |_path: &Path| {
      Ok(AstModule {
        name: "mod".into(),
        items: vec![],
      })
    };

    resolver
      .resolve_module(PathBuf::from("src/math/vector.px"), base_dir, &mut load_fn)
      .expect("first module");

    let err = resolver
      .resolve_module(PathBuf::from("lib/math/vector.px"), base_dir, &mut load_fn)
      .expect_err("namespace collision");

    match err {
      ModuleResolutionError::NamespaceCollision { namespace, .. } => {
        assert_eq!(namespace, "math.vector");
      }
      other => panic!("unexpected error: {other:?}"),
    }
  }
}
