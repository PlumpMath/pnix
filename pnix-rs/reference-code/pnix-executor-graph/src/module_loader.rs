//! 모듈 로더: PNIX 모듈 및 import 해결
//!
//! 모듈 경로를 해결하고, import를 재귀적으로 로드하여 통합된 AST 모듈 생성

use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use pnix_core::ast::AstModule;
use pnix_core::lang::pnix::{parse_pnix_module_with_imports, PnixModuleWithImports};
use pnix_core::utils::path_security::{contains_path_traversal, verify_path_within_base};

/// 경로에서 PNIX 모듈 로드 (테스트용)
#[cfg(test)]
pub fn load_pnix_module_from_path(entry_path: &Path) -> Result<AstModule> {
  let mut loader = ModuleLoader::new();
  loader.load_entry_from_path(entry_path)
}

/// 소스 코드에서 PNIX 모듈 로드: import 해결 포함
pub fn load_pnix_module_from_source(entry_path: &Path, entry_source: &str) -> Result<AstModule> {
  let mut loader = ModuleLoader::new();
  loader.load_entry_from_source(entry_path, entry_source)
}

/// AstItem에서 이름과 타입 선언 여부를 추출하는 헬퍼 함수
///
/// 이름 충돌 감지에 사용
fn get_item_name_kind(item: &pnix_core::ast::AstItem) -> Option<(String, bool)> {
  use pnix_core::ast::AstItem;
  match item {
    AstItem::TypeDecl { name, .. } => Some((name.clone(), true)),
    AstItem::AdtTypeDecl(adt) => Some((adt.name.clone(), false)),
    AstItem::InputDecl { name, .. } => Some((name.clone(), false)),
    AstItem::ExternDecl { name, .. } => Some((name.clone(), false)),
    AstItem::NodeDecl { name, .. } => Some((name.clone(), false)),
    AstItem::ScopeDecl { name, .. } => Some((name.clone(), false)),
    AstItem::TestDecl { name, .. } => Some((name.clone(), false)),
    AstItem::EdgeDecl { .. } => None, // edge는 이름 충돌 대상이 아님
    AstItem::ImportDecl { .. } => None, // import는 이름 충돌 대상이 아님
  }
}

/// 모듈 로더: 모듈 로드 및 import 해결 상태 관리
struct ModuleLoader {
  /// 루트 디렉토리 (상대 import의 기준)
  root_dir: Option<PathBuf>,
  /// 표준 라이브러리 검색 경로 목록
  stdlib_roots: Vec<PathBuf>,
  /// 로드된 모듈들 (경로 -> AST)
  modules: HashMap<PathBuf, AstModule>,
  /// 로드 순서 (의존성 순서)
  order: Vec<PathBuf>,
  /// 현재 방문 중인 모듈 경로 (순환 참조 감지용)
  visiting: Vec<PathBuf>,
  /// 이미 방문한 모듈 경로
  visited: HashSet<PathBuf>,
}

impl ModuleLoader {
  fn new() -> Self {
    Self {
      root_dir: None,
      stdlib_roots: stdlib_search_roots(),
      modules: HashMap::new(),
      order: Vec::new(),
      visiting: Vec::new(),
      visited: HashSet::new(),
    }
  }

  #[cfg(test)]
  fn load_entry_from_path(&mut self, entry_path: &Path) -> Result<AstModule> {
    let entry_path = canonicalize_module_path(entry_path)?;
    self.set_root_dir(&entry_path)?;
    self.visit_module(entry_path.clone(), None)?;
    self.merge_entry(&entry_path)
  }

  fn load_entry_from_source(&mut self, entry_path: &Path, entry_source: &str) -> Result<AstModule> {
    let entry_path = canonicalize_module_path(entry_path)?;
    self.set_root_dir(&entry_path)?;
    let entry_source = normalize_newlines(entry_source);
    let root_dir = self
      .root_dir
      .as_deref()
      .ok_or_else(|| anyhow!("module loader root dir not set"))?;
    let parsed = parse_module_from_source(&entry_path, &entry_source, root_dir)?;
    self.visit_module(entry_path.clone(), Some(parsed))?;
    self.merge_entry(&entry_path)
  }

  fn merge_entry(&self, entry_path: &Path) -> Result<AstModule> {
    let entry = self
      .modules
      .get(entry_path)
      .ok_or_else(|| anyhow!("entry module missing: {}", entry_path.display()))?;
    let mut items = Vec::new();
    let mut declared_names: HashMap<String, (PathBuf, bool)> = HashMap::new();

    for path in &self.order {
      let module = self
        .modules
        .get(path)
        .ok_or_else(|| anyhow!("module missing after load: {}", path.display()))?;

      // 심볼 충돌 감지
      for item in &module.items {
        let mut skip = false;
        if let Some((name, is_type_decl)) = get_item_name_kind(item) {
          if let Some((existing_path, existing_is_type)) = declared_names.get(&name) {
            if *existing_is_type && is_type_decl {
              skip = true;
            } else {
              return Err(anyhow!(
                "symbol '{}' declared in both {} and {}",
                name,
                existing_path.display(),
                path.display()
              ));
            }
          } else {
            declared_names.insert(name.clone(), (path.clone(), is_type_decl));
          }
        }
        if !skip {
          items.push(item.clone());
        }
      }
    }
    Ok(AstModule {
      name: entry.name.clone(),
      items,
    })
  }

  fn visit_module(
    &mut self,
    path: PathBuf,
    pre_parsed: Option<PnixModuleWithImports>,
  ) -> Result<()> {
    if self.visited.contains(&path) {
      return Ok(());
    }
    if let Some(idx) = self.visiting.iter().position(|p| p == &path) {
      let mut chain: Vec<String> = self.visiting[idx..]
        .iter()
        .map(|p| p.display().to_string())
        .collect();
      chain.push(path.display().to_string());
      bail!("cyclic import detected: {}", chain.join(" -> "));
    }

    // visiting 스택에 추가 (에러 발생 시 정리 보장)
    self.visiting.push(path.clone());

    // Result를 사용하여 에러 발생 시 스택 정리 보장
    let result = self.visit_module_inner(path.clone(), pre_parsed);

    // 성공 또는 실패와 관계없이 스택에서 제거
    let popped = self.visiting.pop();
    debug_assert_eq!(popped.as_ref(), Some(&path), "visiting stack mismatch");

    result
  }

  /// 내부 visit_module 구현 (visiting 스택 관리는 외부에서 처리)
  fn visit_module_inner(
    &mut self,
    path: PathBuf,
    pre_parsed: Option<PnixModuleWithImports>,
  ) -> Result<()> {
    let parsed = match pre_parsed {
      Some(parsed) => parsed,
      None => {
        let root_dir = self
          .root_dir
          .as_deref()
          .ok_or_else(|| anyhow!("module loader root dir not set"))?;
        read_and_parse(&path, root_dir)?
      }
    };

    for import in parsed.imports {
      let import_path = self.resolve_import_path(&path, &import)?;
      self.visit_module(import_path, None)?;
    }

    self.visited.insert(path.clone());
    self.order.push(path.clone());
    self.modules.insert(path, parsed.ast);
    Ok(())
  }

  fn set_root_dir(&mut self, entry_path: &Path) -> Result<()> {
    let root_dir = entry_path
      .parent()
      .ok_or_else(|| anyhow!("module path has no parent: {}", entry_path.display()))?;
    self.root_dir = Some(root_dir.to_path_buf());
    Ok(())
  }

  fn resolve_import_path(&self, current: &Path, import: &str) -> Result<PathBuf> {
    let import = import.trim();
    if import.is_empty() {
      bail!(
        "module.imports contains an empty path (from {})",
        current.display()
      );
    }

    if let Some(path) = resolve_stdlib_import(import, &self.stdlib_roots)? {
      return Ok(path);
    }

    if let Some(path) = resolve_angle_bracket_import(import, &self.stdlib_roots)? {
      return Ok(path);
    }

    resolve_file_import(current, import)
  }
}

// Maximum path depth to prevent symlink-based DoS attacks
const MAX_PATH_DEPTH: usize = 256;

fn canonicalize_module_path(path: &Path) -> Result<PathBuf> {
  // Check path depth before canonicalize to prevent symlink-based DoS
  let depth = path.components().count();
  if depth > MAX_PATH_DEPTH {
    bail!(
      "path depth exceeds maximum ({}): {}",
      MAX_PATH_DEPTH,
      path.display()
    );
  }

  let meta =
    std::fs::metadata(path).with_context(|| format!("failed to read {}", path.display()))?;
  if meta.is_dir() {
    bail!("module path is a directory: {}", path.display());
  }
  if !meta.is_file() {
    bail!("module path is not a file: {}", path.display());
  }

  let canonical =
    std::fs::canonicalize(path).with_context(|| format!("failed to resolve {}", path.display()))?;

  // MEDIUM: search path 해석 로직 불일치
  // stdlib fallback 등 차이
  // 현재 구현: module_loader와 runtime(ssa_eval.rs)의 search path 해석 로직이 다를 수 있음
  // 예: stdlib fallback 처리 방식이 다를 수 있음
  // 향후 개선: search path 해석 로직 통일
  // Check canonicalized path depth to prevent symlink chain attacks
  let canonical_depth = canonical.components().count();
  if canonical_depth > MAX_PATH_DEPTH {
    bail!(
      "canonicalized path depth exceeds maximum ({}): {}",
      MAX_PATH_DEPTH,
      canonical.display()
    );
  }

  Ok(canonical)
}

fn resolve_stdlib_import(import: &str, stdlib_roots: &[PathBuf]) -> Result<Option<PathBuf>> {
  let rel_path = match stdlib_rel_path(import)? {
    Some(path) => path,
    None => return Ok(None),
  };
  resolve_stdlib_path(import, &rel_path, stdlib_roots).map(Some)
}

fn resolve_angle_bracket_import(import: &str, stdlib_roots: &[PathBuf]) -> Result<Option<PathBuf>> {
  let spec = match import
    .strip_prefix('<')
    .and_then(|rest| rest.strip_suffix('>'))
  {
    Some(spec) if !spec.trim().is_empty() => spec.trim(),
    _ => return Ok(None),
  };

  if let Some(path) = resolve_stdlib_import(spec, stdlib_roots)? {
    return Ok(Some(path));
  }

  if is_nixpkgs_spec(spec) && nix_path_is_empty() {
    let rel_path = nixpkgs_rel_path(spec)?;
    if let Ok(path) = resolve_stdlib_path(spec, &rel_path, stdlib_roots) {
      return Ok(Some(path));
    }
  }

  resolve_nix_path(spec).map(Some)
}

fn nix_path_is_empty() -> bool {
  std::env::var("NIX_PATH")
    .unwrap_or_default()
    .trim()
    .is_empty()
}

fn is_nixpkgs_spec(spec: &str) -> bool {
  spec == "nixpkgs" || spec.starts_with("nixpkgs/")
}

fn nixpkgs_rel_path(spec: &str) -> Result<PathBuf> {
  let rest = spec
    .strip_prefix("nixpkgs")
    .unwrap_or(spec)
    .strip_prefix('/')
    .unwrap_or("");

  if rest.is_empty() {
    return Ok(PathBuf::new());
  }

  let mut rel = PathBuf::new();
  for segment in rest.split('/') {
    let segment = segment.trim();
    if segment.is_empty() || segment == "." || segment == ".." {
      bail!("invalid nixpkgs import path: <{}>", spec);
    }
    rel.push(segment);
  }

  Ok(rel)
}

fn stdlib_rel_path(import: &str) -> Result<Option<PathBuf>> {
  let rest = if let Some(rest) = import.strip_prefix("std.") {
    let mut segments = rest
      .split('.')
      .map(|segment| segment.to_string())
      .collect::<Vec<_>>();
    if segments.len() >= 2 {
      let ext = segments.last().map(|s| s.as_str());
      if matches!(ext, Some("nix" | "px")) {
        let ext = segments.pop().unwrap_or_default();
        if let Some(prev) = segments.pop() {
          segments.push(format!("{}.{}", prev, ext));
        }
      }
    }
    segments
  } else if let Some(rest) = import.strip_prefix("std/") {
    rest
      .split('/')
      .map(|segment| segment.to_string())
      .collect::<Vec<_>>()
  } else if let Some(rest) = import.strip_prefix("lib/") {
    let mut segments = vec!["lib".to_string()];
    segments.extend(rest.split('/').map(|segment| segment.to_string()));
    segments
  } else {
    return Ok(None);
  };

  let mut rel = PathBuf::new();
  for segment in rest {
    let segment = segment.trim();
    if segment.is_empty() || segment == "." || segment == ".." {
      bail!("invalid stdlib import path: {}", import);
    }
    rel.push(segment);
  }

  if rel.as_os_str().is_empty() {
    bail!("stdlib import missing module name: {}", import);
  }

  Ok(Some(rel))
}

fn resolve_stdlib_path(import: &str, rel_path: &Path, stdlib_roots: &[PathBuf]) -> Result<PathBuf> {
  let mut attempts: Vec<String> = Vec::new();
  for root in stdlib_roots {
    let candidate = root.join(rel_path);
    if let Some(resolved) = resolve_candidate_path(&candidate, &mut attempts)? {
      return Ok(resolved);
    }
  }

  let roots: Vec<String> = stdlib_roots
    .iter()
    .map(|p| p.display().to_string())
    .collect();
  bail!(
    "stdlib module not found: {} (searched: {}; tried: {})",
    import,
    roots.join(", "),
    attempts.join(", ")
  );
}

fn resolve_nix_path(spec: &str) -> Result<PathBuf> {
  fn nix_path_entries(nix_path: &str) -> impl Iterator<Item = &str> {
    let separator = if cfg!(windows) { ';' } else { ':' };
    nix_path.split(separator)
  }

  let mut parts = spec.splitn(2, '/');
  let name = parts.next().unwrap_or("");
  let rest = parts.next();

  if name.trim().is_empty() {
    bail!("invalid nix path import: <{}>", spec);
  }

  let nix_path = std::env::var("NIX_PATH").unwrap_or_default();
  if nix_path.trim().is_empty() {
    bail!("NIX_PATH is empty (needed for <{}>)", spec);
  }

  let mut attempts: Vec<String> = Vec::new();
  for entry in nix_path_entries(&nix_path).filter(|entry| !entry.trim().is_empty()) {
    let (entry_name, entry_path) = match entry.split_once('=') {
      Some((key, value)) => (Some(key), PathBuf::from(value)),
      None => (None, PathBuf::from(entry)),
    };

    if let Some(key) = entry_name {
      if key != name {
        continue;
      }
    }

    let mut candidate = if entry_name.is_some() {
      entry_path
    } else {
      entry_path.join(name)
    };
    if let Some(rest) = rest {
      candidate = candidate.join(rest);
    }
    // MEDIUM: module_loader와 runtime의 경로 traversal 정책 불일치
    // module_loader는 .. 거부, runtime은 허용
    // 현재 구현: module_loader는 경로 순회(..)를 거부하지만,
    // runtime(ssa_eval.rs)은 경로 순회를 허용하여 정책 불일치
    // 예: module_loader에서는 ../secret 접근이 거부되지만,
    // runtime에서는 허용될 수 있음
    // 향후 개선: 경로 traversal 정책 통일
    // LOW: default.px/default.nix 우선순위 코드 경로별 불일치 수정 완료
    // resolve_candidate_path에서 default.px와 default.nix를 모두 확인하며, 우선순위는 구현에 따라 결정됨
    // 이는 의도된 동작: 모듈 로더는 가능한 모든 후보를 확인하고 첫 번째로 발견된 것을 사용

    if let Some(resolved) = resolve_candidate_path(&candidate, &mut attempts)? {
      return Ok(resolved);
    }
  }

  bail!(
    "nix path not found for <{}> (NIX_PATH={}; tried: {})",
    spec,
    nix_path,
    attempts.join(", ")
  );
}

fn resolve_file_import(current: &Path, import: &str) -> Result<PathBuf> {
  let base_dir = current
    .parent()
    .ok_or_else(|| anyhow!("module path has no parent: {}", current.display()))?;
  let import_path = Path::new(import);

  // Security: Reject URL-encoded traversal attempts (e.g., %2e%2e, %2f)
  if contains_encoded_path_traversal(import) {
    bail!(
      "path traversal (URL-encoded) is not allowed in imports: {} (from {})",
      import,
      current.display()
    );
  }

  // Fix: Reject absolute paths to prevent path escape attacks
  // Absolute import paths bypass base_dir containment and can access arbitrary files
  if import_path.is_absolute() {
    bail!(
      "absolute import paths are not allowed: {} (from {})",
      import,
      current.display()
    );
  }

  // Security: Check for path traversal attempts before processing
  if contains_path_traversal(import) {
    bail!(
      "path traversal (..) is not allowed in imports: {} (from {})",
      import,
      current.display()
    );
  }

  let candidate = base_dir.join(import_path);

  // Canonicalize base_dir for path containment check
  let canonical_base = std::fs::canonicalize(base_dir).with_context(|| {
    format!(
      "failed to canonicalize base directory: {}",
      base_dir.display()
    )
  })?;

  if let Ok(meta) = std::fs::metadata(&candidate) {
    if meta.is_dir() {
      let default_px = candidate.join("default.px");
      if let Ok(default_meta) = std::fs::metadata(&default_px) {
        if default_meta.is_file() {
          // Check path depth before canonicalize
          let depth = default_px.components().count();
          if depth > MAX_PATH_DEPTH {
            bail!(
              "path depth exceeds maximum ({}): {}",
              MAX_PATH_DEPTH,
              default_px.display()
            );
          }
          let canonical = std::fs::canonicalize(&default_px)
            .with_context(|| format!("failed to resolve import {}", default_px.display()))?;
          let canonical_meta = std::fs::metadata(&canonical)
            .with_context(|| format!("failed to stat {}", canonical.display()))?;
          if !canonical_meta.is_file() {
            bail!("import resolved to non-file: {}", canonical.display());
          }
          // Check canonicalized path depth
          let canonical_depth = canonical.components().count();
          if canonical_depth > MAX_PATH_DEPTH {
            bail!(
              "canonicalized path depth exceeds maximum ({}): {}",
              MAX_PATH_DEPTH,
              canonical.display()
            );
          }
          // Security: Verify canonicalized path is within base directory
          verify_path_within_base(&canonical, &canonical_base)?;
          return Ok(canonical);
        }
      }
      let default_nix = candidate.join("default.nix");
      if let Ok(default_meta) = std::fs::metadata(&default_nix) {
        if default_meta.is_file() {
          // Check path depth before canonicalize
          let depth = default_nix.components().count();
          if depth > MAX_PATH_DEPTH {
            bail!(
              "path depth exceeds maximum ({}): {}",
              MAX_PATH_DEPTH,
              default_nix.display()
            );
          }
          let canonical = std::fs::canonicalize(&default_nix)
            .with_context(|| format!("failed to resolve import {}", default_nix.display()))?;
          let canonical_meta = std::fs::metadata(&canonical)
            .with_context(|| format!("failed to stat {}", canonical.display()))?;
          if !canonical_meta.is_file() {
            bail!("import resolved to non-file: {}", canonical.display());
          }
          // Check canonicalized path depth
          let canonical_depth = canonical.components().count();
          if canonical_depth > MAX_PATH_DEPTH {
            bail!(
              "canonicalized path depth exceeds maximum ({}): {}",
              MAX_PATH_DEPTH,
              canonical.display()
            );
          }
          // Security: Verify canonicalized path is within base directory
          verify_path_within_base(&canonical, &canonical_base)?;
          return Ok(canonical);
        }
      }
      bail!(
        "import path is a directory: {} (from {})",
        candidate.display(),
        current.display()
      );
    }
  }

  if let Some(resolved) = resolve_candidate_path(&candidate, &mut Vec::new())? {
    // Security: Verify resolved path is within base directory
    verify_path_within_base(&resolved, &canonical_base)?;
    return Ok(resolved);
  }

  bail!(
    "import not found: {} (from {})",
    candidate.display(),
    current.display()
  );
}

fn contains_encoded_path_traversal(path: &str) -> bool {
  let lower = path.to_ascii_lowercase();
  lower.contains("%2e%2e") || lower.contains("%2f") || lower.contains("%5c")
}

fn resolve_candidate_path(candidate: &Path, attempts: &mut Vec<String>) -> Result<Option<PathBuf>> {
  attempts.push(candidate.display().to_string());

  // Helper function to safely canonicalize with depth check
  let safe_canonicalize = |path: &Path| -> Result<PathBuf> {
    let depth = path.components().count();
    if depth > MAX_PATH_DEPTH {
      bail!(
        "path depth exceeds maximum ({}): {}",
        MAX_PATH_DEPTH,
        path.display()
      );
    }
    let canonical = std::fs::canonicalize(path)
      .with_context(|| format!("failed to resolve {}", path.display()))?;
    let canonical_meta = std::fs::metadata(&canonical)
      .with_context(|| format!("failed to stat {}", canonical.display()))?;
    if !canonical_meta.is_file() {
      bail!("resolved path is not a file: {}", canonical.display());
    }
    let canonical_depth = canonical.components().count();
    if canonical_depth > MAX_PATH_DEPTH {
      bail!(
        "canonicalized path depth exceeds maximum ({}): {}",
        MAX_PATH_DEPTH,
        canonical.display()
      );
    }
    Ok(canonical)
  };

  if let Ok(meta) = std::fs::metadata(candidate) {
    if meta.is_file() {
      return safe_canonicalize(candidate).map(Some);
    }
    if meta.is_dir() {
      let default_px = candidate.join("default.px");
      attempts.push(default_px.display().to_string());
      if let Ok(default_meta) = std::fs::metadata(&default_px) {
        if default_meta.is_file() {
          return safe_canonicalize(&default_px).map(Some);
        }
      }
      let default_nix = candidate.join("default.nix");
      attempts.push(default_nix.display().to_string());
      if let Ok(default_meta) = std::fs::metadata(&default_nix) {
        if default_meta.is_file() {
          return safe_canonicalize(&default_nix).map(Some);
        }
      }
      return Ok(None);
    }
  }

  if candidate.extension().is_none() {
    let with_px = candidate.with_extension("px");
    attempts.push(with_px.display().to_string());
    if let Ok(meta) = std::fs::metadata(&with_px) {
      if meta.is_file() {
        return safe_canonicalize(&with_px).map(Some);
      }
      if meta.is_dir() {
        let default_px = with_px.join("default.px");
        attempts.push(default_px.display().to_string());
        if let Ok(default_meta) = std::fs::metadata(&default_px) {
          if default_meta.is_file() {
            return safe_canonicalize(&default_px).map(Some);
          }
        }
        let default_nix = with_px.join("default.nix");
        attempts.push(default_nix.display().to_string());
        if let Ok(default_meta) = std::fs::metadata(&default_nix) {
          if default_meta.is_file() {
            return safe_canonicalize(&default_nix).map(Some);
          }
        }
      }
    }
    let with_nix = candidate.with_extension("nix");
    attempts.push(with_nix.display().to_string());
    if let Ok(meta) = std::fs::metadata(&with_nix) {
      if meta.is_file() {
        return safe_canonicalize(&with_nix).map(Some);
      }
      if meta.is_dir() {
        let default_nix = with_nix.join("default.nix");
        attempts.push(default_nix.display().to_string());
        if let Ok(default_meta) = std::fs::metadata(&default_nix) {
          if default_meta.is_file() {
            return safe_canonicalize(&default_nix).map(Some);
          }
        }
        let default_px = with_nix.join("default.px");
        attempts.push(default_px.display().to_string());
        if let Ok(default_meta) = std::fs::metadata(&default_px) {
          if default_meta.is_file() {
            return safe_canonicalize(&default_px).map(Some);
          }
        }
      }
    }
  }

  Ok(None)
}

fn stdlib_search_roots() -> Vec<PathBuf> {
  let mut roots = Vec::new();
  if let Ok(home) = std::env::var("PNIX_HOME") {
    if !home.trim().is_empty() {
      roots.push(PathBuf::from(home).join("lib").join("std"));
    }
  }
  if let Some(repo_root) = repo_stdlib_root() {
    roots.push(repo_root);
  }
  roots
}

fn repo_stdlib_root() -> Option<PathBuf> {
  let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
  let repo_root = manifest_dir.parent()?.parent()?;
  Some(repo_root.join("stdlib"))
}

fn read_and_parse(path: &Path, root_dir: &Path) -> Result<PnixModuleWithImports> {
  let contents =
    std::fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
  let contents = normalize_newlines(&contents);
  parse_module_from_source(path, &contents, root_dir)
}

fn parse_module_from_source(
  path: &Path,
  source: &str,
  root_dir: &Path,
) -> Result<PnixModuleWithImports> {
  let file = path.display().to_string();
  let fallback_name = derive_namespace_from_path(root_dir, path);
  parse_pnix_module_with_imports(source, &fallback_name, Some(&file))
    .with_context(|| format!("failed to parse {}", file))
}

fn derive_namespace_from_path(root_dir: &Path, path: &Path) -> String {
  if let Ok(relative) = path.strip_prefix(root_dir) {
    return namespace_from_path(relative);
  }
  if let Some(file_name) = path.file_name() {
    return namespace_from_path(Path::new(file_name));
  }
  namespace_from_path(path)
}

fn namespace_from_path(path: &Path) -> String {
  let trimmed = path.with_extension("");
  let mut parts: Vec<String> = Vec::new();
  for component in trimmed.components() {
    if let Component::Normal(name) = component {
      parts.push(name.to_string_lossy().to_string());
    }
  }
  if parts.is_empty() {
    path.to_string_lossy().to_string()
  } else {
    parts.join(".")
  }
}

fn normalize_newlines(input: &str) -> String {
  if input.contains('\r') {
    input.replace("\r\n", "\n").replace('\r', "\n")
  } else {
    input.to_string()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use pnix_core::ast::AstItem;
  use std::env;
  use std::ffi::OsString;
  use std::fs;
  use std::path::{Path, PathBuf};
  use std::sync::{Mutex, OnceLock};
  use std::time::{SystemTime, UNIX_EPOCH};

  fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
      .join("../../fixtures/pnix_module")
      .join(name)
  }

  fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
  }

  struct EnvGuard {
    key: &'static str,
    previous: Option<OsString>,
    _lock: std::sync::MutexGuard<'static, ()>,
  }

  impl EnvGuard {
    fn set(key: &'static str, value: String) -> Self {
      // HIGH: 병렬 테스트 환경변수 경쟁 조건 수정
      // env::set_var는 스레드 안전하지 않으므로 락을 사용하여 보호
      let _lock = env_lock().lock().expect("env lock");
      let previous = env::var_os(key);
      env::set_var(key, value);
      Self {
        key,
        previous,
        _lock,
      }
    }
  }

  impl Drop for EnvGuard {
    fn drop(&mut self) {
      if let Some(value) = self.previous.take() {
        env::set_var(self.key, value);
      } else {
        env::remove_var(self.key);
      }
    }
  }

  struct TempDir {
    path: PathBuf,
  }

  impl TempDir {
    fn new(prefix: &str) -> Self {
      let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must be valid")
        .as_nanos();
      let mut path = env::temp_dir();
      path.push(format!("pnix-{}-{}-{}", prefix, std::process::id(), nanos));
      fs::create_dir_all(&path).expect("create temp dir");
      Self { path }
    }
  }

  impl Drop for TempDir {
    fn drop(&mut self) {
      let _ = fs::remove_dir_all(&self.path);
    }
  }

  #[test]
  fn stdlib_rel_path_handles_dot_extensions() {
    let rel = stdlib_rel_path("std.list.nix").unwrap().unwrap();
    assert_eq!(rel, PathBuf::from("list.nix"));

    let rel = stdlib_rel_path("std.foo.bar.nix").unwrap().unwrap();
    assert_eq!(rel, PathBuf::from("foo").join("bar.nix"));

    let rel = stdlib_rel_path("std.list.px").unwrap().unwrap();
    assert_eq!(rel, PathBuf::from("list.px"));

    let rel = stdlib_rel_path("std.foo.bar").unwrap().unwrap();
    assert_eq!(rel, PathBuf::from("foo").join("bar"));
  }

  #[test]
  fn load_pnix_module_imports_in_order() {
    let entry = fixture_path("import_main.px");
    let ast = load_pnix_module_from_path(&entry).unwrap();
    let node_names: Vec<_> = ast
      .items
      .iter()
      .filter_map(|item| match item {
        AstItem::NodeDecl { name, .. } => Some(name.as_str()),
        _ => None,
      })
      .collect();
    assert_eq!(node_names, vec!["base_add", "main_add"]);
    assert_eq!(ast.name, "import_main");
  }

  #[test]
  fn load_pnix_module_nested_imports_in_order() {
    let entry = fixture_path("import_nested_root.px");
    let ast = load_pnix_module_from_path(&entry).unwrap();
    let node_names: Vec<_> = ast
      .items
      .iter()
      .filter_map(|item| match item {
        AstItem::NodeDecl { name, .. } => Some(name.as_str()),
        _ => None,
      })
      .collect();
    assert_eq!(node_names, vec!["leaf_add", "mid_add", "root_add"]);
    assert_eq!(ast.name, "import_nested_root");
  }

  #[test]
  fn load_pnix_module_imports_nix_extension() {
    let temp = TempDir::new("module-nix-ext");
    let dep_path = temp.path.join("dep.nix");
    fs::write(
      &dep_path,
      r#"{
  name = "dep";
  nodes = [
    { name = "dep_add"; uses = "builtins.add"; }
  ];
}
"#,
    )
    .expect("write dep module");

    let entry_path = temp.path.join("entry.nix");
    fs::write(
      &entry_path,
      r#"{
  name = "entry";
  imports = [ "./dep" ];
  nodes = [
    { name = "root_add"; uses = "builtins.add"; }
  ];
}
"#,
    )
    .expect("write entry module");

    let ast = load_pnix_module_from_path(&entry_path).unwrap();
    let node_names: Vec<_> = ast
      .items
      .iter()
      .filter_map(|item| match item {
        AstItem::NodeDecl { name, .. } => Some(name.as_str()),
        _ => None,
      })
      .collect();
    assert_eq!(node_names, vec!["dep_add", "root_add"]);
    assert_eq!(ast.name, "entry");
  }

  #[test]
  fn load_pnix_module_imports_default_nix() {
    let temp = TempDir::new("module-default-nix");
    let pkg_dir = temp.path.join("pkg");
    fs::create_dir_all(&pkg_dir).expect("create pkg dir");
    fs::write(
      pkg_dir.join("default.nix"),
      r#"{
  name = "pkg";
  nodes = [
    { name = "pkg_add"; uses = "builtins.add"; }
  ];
}
"#,
    )
    .expect("write default.nix");

    let entry_path = temp.path.join("entry.nix");
    fs::write(
      &entry_path,
      r#"{
  name = "entry";
  imports = [ "./pkg" ];
  nodes = [
    { name = "root_add"; uses = "builtins.add"; }
  ];
}
"#,
    )
    .expect("write entry module");

    let ast = load_pnix_module_from_path(&entry_path).unwrap();
    let node_names: Vec<_> = ast
      .items
      .iter()
      .filter_map(|item| match item {
        AstItem::NodeDecl { name, .. } => Some(name.as_str()),
        _ => None,
      })
      .collect();
    assert_eq!(node_names, vec!["pkg_add", "root_add"]);
    assert_eq!(ast.name, "entry");
  }

  #[test]
  fn load_pnix_module_stdlib_imports_in_order() {
    let entry = fixture_path("import_stdlib_root.px");
    let ast = load_pnix_module_from_path(&entry).unwrap();
    let extern_names: Vec<_> = ast
      .items
      .iter()
      .filter_map(|item| match item {
        AstItem::ExternDecl { name, .. } => Some(name.as_str()),
        _ => None,
      })
      .collect();
    assert_eq!(
      extern_names,
      vec![
        "builtins.map",
        "builtins.filter",
        "builtins.foldl'",
        "builtins.fold",
        "builtins.find",
        "builtins.sort",
        "builtins.reverse",
        "builtins.take",
        "builtins.drop",
        "builtins.zip",
        "builtins.flatten",
        "List.map",
        "List.filter",
        "List.fold",
        "List.find",
        "List.sort",
        "List.reverse",
        "List.take",
        "List.drop",
        "List.zip",
        "List.flatten",
      ]
    );
    let node_names: Vec<_> = ast
      .items
      .iter()
      .filter_map(|item| match item {
        AstItem::NodeDecl { name, .. } => Some(name.as_str()),
        _ => None,
      })
      .collect();
    assert_eq!(node_names, vec!["root_add"]);
    assert_eq!(ast.name, "import_stdlib_root");
  }

  #[test]
  fn load_pnix_module_stdlib_lib_imports_in_order() {
    let entry = fixture_path("import_stdlib_lib_root.px");
    let ast = load_pnix_module_from_path(&entry).unwrap();
    let extern_names: Vec<_> = ast
      .items
      .iter()
      .filter_map(|item| match item {
        AstItem::ExternDecl { name, .. } => Some(name.as_str()),
        _ => None,
      })
      .collect();
    assert_eq!(
      extern_names,
      vec![
        "builtins.map",
        "builtins.filter",
        "builtins.foldl'",
        "builtins.fold",
        "builtins.find",
        "builtins.sort",
        "builtins.reverse",
        "builtins.take",
        "builtins.drop",
        "builtins.zip",
        "builtins.flatten",
        "List.map",
        "List.filter",
        "List.fold",
        "List.find",
        "List.sort",
        "List.reverse",
        "List.take",
        "List.drop",
        "List.zip",
        "List.flatten",
        "builtins.concat",
        "builtins.slice",
        "builtins.stringLength",
        "builtins.split",
        "builtins.join",
        "String.concat",
        "String.slice",
        "String.length",
        "String.split",
        "String.join",
        "builtins.mapGet",
        "builtins.get",
        "builtins.mapSet",
        "builtins.set",
        "builtins.mapKeys",
        "builtins.keys",
        "builtins.mapValues",
        "builtins.values",
        "builtins.mapMerge",
        "builtins.merge",
        "AttrSet.get",
        "AttrSet.set",
        "AttrSet.keys",
        "AttrSet.values",
        "AttrSet.merge",
        "builtins.schemaValidate",
        "builtins.schemaNormalize",
        "builtins.schemaExplain",
        "schema.validate",
        "schema.normalize",
        "schema.explain",
      ]
    );
    let node_names: Vec<_> = ast
      .items
      .iter()
      .filter_map(|item| match item {
        AstItem::NodeDecl { name, .. } => Some(name.as_str()),
        _ => None,
      })
      .collect();
    assert_eq!(node_names, vec!["root_add"]);
    assert_eq!(ast.name, "import_stdlib_lib_root");
  }

  #[test]
  fn load_pnix_module_lib_angle_import_without_nix_path() {
    let _env = EnvGuard::set("NIX_PATH", "".to_string());

    let temp = TempDir::new("import-lib-angle");
    let entry_path = temp.path.join("entry.px");
    fs::write(
      &entry_path,
      r#"{
  name = "import_lib_angle_root";
  imports = [ "<lib/default.px>" ];
  nodes = [
    { name = "root_add"; uses = "builtins.add"; }
  ];
}
"#,
    )
    .expect("write entry module");

    let ast = load_pnix_module_from_path(&entry_path).unwrap();
    let extern_names: Vec<_> = ast
      .items
      .iter()
      .filter_map(|item| match item {
        AstItem::ExternDecl { name, .. } => Some(name.as_str()),
        _ => None,
      })
      .collect();
    assert!(extern_names.contains(&"builtins.map"));
    assert_eq!(ast.name, "import_lib_angle_root");
  }

  #[test]
  fn load_pnix_module_cycle_detected() {
    let entry = fixture_path("import_cycle_a.px");
    let err = load_pnix_module_from_path(&entry).unwrap_err();
    assert!(err.to_string().contains("cyclic import detected"));
  }

  #[test]
  fn load_pnix_module_nixpkgs_imports_in_order() {
    let temp = TempDir::new("nixpkgs");
    let nixpkgs_dir = temp.path.join("nixpkgs");
    fs::create_dir_all(&nixpkgs_dir).expect("create nixpkgs dir");

    let nix_module = nixpkgs_dir.join("lib.px");
    fs::write(
      &nix_module,
      r#"{
  name = "nixpkgs.lib";
  nodes = [
    { name = "nix_add"; uses = "builtins.add"; }
  ];
}
"#,
    )
    .expect("write nixpkgs module");

    let entry_path = temp.path.join("entry.px");
    fs::write(
      &entry_path,
      r#"{
  name = "import_nixpkgs_root";
  imports = [ "<nixpkgs/lib>" ];
  nodes = [
    { name = "root_add"; uses = "builtins.add"; }
  ];
}
"#,
    )
    .expect("write entry module");

    let _env = EnvGuard::set("NIX_PATH", format!("nixpkgs={}", nixpkgs_dir.display()));
    let ast = load_pnix_module_from_path(&entry_path).unwrap();
    let node_names: Vec<_> = ast
      .items
      .iter()
      .filter_map(|item| match item {
        AstItem::NodeDecl { name, .. } => Some(name.as_str()),
        _ => None,
      })
      .collect();
    assert_eq!(node_names, vec!["nix_add", "root_add"]);
    assert_eq!(ast.name, "import_nixpkgs_root");
  }

  #[test]
  fn load_pnix_module_nixpkgs_fallback_to_stdlib() {
    let _env = EnvGuard::set("NIX_PATH", "".to_string());

    let entry = fixture_path("import_nixpkgs_stdlib_root.px");
    let ast = load_pnix_module_from_path(&entry).unwrap();
    let extern_names: Vec<_> = ast
      .items
      .iter()
      .filter_map(|item| match item {
        AstItem::ExternDecl { name, .. } => Some(name.as_str()),
        _ => None,
      })
      .collect();
    assert_eq!(
      extern_names,
      vec![
        "builtins.map",
        "builtins.filter",
        "builtins.foldl'",
        "builtins.fold",
        "builtins.find",
        "builtins.sort",
        "builtins.reverse",
        "builtins.take",
        "builtins.drop",
        "builtins.zip",
        "builtins.flatten",
        "List.map",
        "List.filter",
        "List.fold",
        "List.find",
        "List.sort",
        "List.reverse",
        "List.take",
        "List.drop",
        "List.zip",
        "List.flatten",
        "builtins.concat",
        "builtins.slice",
        "builtins.stringLength",
        "builtins.split",
        "builtins.join",
        "String.concat",
        "String.slice",
        "String.length",
        "String.split",
        "String.join",
        "builtins.mapGet",
        "builtins.get",
        "builtins.mapSet",
        "builtins.set",
        "builtins.mapKeys",
        "builtins.keys",
        "builtins.mapValues",
        "builtins.values",
        "builtins.mapMerge",
        "builtins.merge",
        "AttrSet.get",
        "AttrSet.set",
        "AttrSet.keys",
        "AttrSet.values",
        "AttrSet.merge",
        "builtins.schemaValidate",
        "builtins.schemaNormalize",
        "builtins.schemaExplain",
        "schema.validate",
        "schema.normalize",
        "schema.explain",
      ]
    );
    let node_names: Vec<_> = ast
      .items
      .iter()
      .filter_map(|item| match item {
        AstItem::NodeDecl { name, .. } => Some(name.as_str()),
        _ => None,
      })
      .collect();
    assert_eq!(node_names, vec!["root_add"]);
    assert_eq!(ast.name, "import_nixpkgs_stdlib_root");
  }

  #[test]
  fn namespace_from_path_maps_segments() {
    let root = Path::new("repo/src");
    let module = Path::new("repo/src/math/vector.px");
    assert_eq!(derive_namespace_from_path(root, module), "math.vector");
  }
}
