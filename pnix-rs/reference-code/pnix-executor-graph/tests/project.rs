//! 프로젝트 매니페스트 파싱 테스트
//!
//! Y10a: 매니페스트 파싱 fixture 테스트
//! Y10b: 의존성 해석 테스트
//! Y10c: 락 파일 생성 및 검증 테스트

use pnix_executor_graph::project::{
  compute_project_hash, generate_lock_file, load_manifest, verify_lock_file, Dependency,
  DependencyResolver, LockFile, ManifestError, ResolverError,
};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_parse_basic_manifest() {
  // 기본 매니페스트 파싱 테스트
  let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .unwrap()
    .parent()
    .unwrap()
    .join("fixtures/project/basic.toml");

  let manifest = load_manifest(&manifest_path).unwrap();

  assert_eq!(manifest.name, "my-project");
  assert_eq!(manifest.version, "0.1.0");
  assert_eq!(
    manifest.description,
    Some("A sample Pnix project".to_string())
  );
  assert_eq!(manifest.license, Some("BSD-3-Clause".to_string()));
  assert_eq!(
    manifest.repository,
    Some("https://github.com/example/my-project".to_string())
  );
  assert_eq!(manifest.authors.len(), 2);
  assert_eq!(manifest.authors[0], "Alice <alice@example.com>");
  assert_eq!(manifest.authors[1], "Bob <bob@example.com>");
  assert_eq!(manifest.dependencies.len(), 3);

  // 첫 번째 의존성: path
  match &manifest.dependencies[0] {
    Dependency {
      name,
      path: Some(path),
      ..
    } => {
      assert_eq!(name, "lib1");
      assert!(path.ends_with("lib1"));
    }
    _ => panic!("Expected path dependency"),
  }

  // 두 번째 의존성: version
  match &manifest.dependencies[1] {
    Dependency {
      name,
      version: Some(version),
      ..
    } => {
      assert_eq!(name, "lib2");
      assert_eq!(version, "1.2.3");
    }
    _ => panic!("Expected version dependency"),
  }

  // 세 번째 의존성: git
  match &manifest.dependencies[2] {
    Dependency {
      name,
      git: Some(git),
      branch: Some(branch),
      ..
    } => {
      assert_eq!(name, "lib3");
      assert_eq!(git, "https://github.com/example/lib3");
      assert_eq!(branch, "main");
    }
    _ => panic!("Expected git dependency"),
  }
}

#[test]
fn test_parse_minimal_manifest() {
  // 최소 매니페스트 파싱 테스트
  let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .unwrap()
    .parent()
    .unwrap()
    .join("fixtures/project/minimal.toml");

  let manifest = load_manifest(&manifest_path).unwrap();

  assert_eq!(manifest.name, "minimal");
  assert_eq!(manifest.version, "0.1.0");
  assert_eq!(manifest.description, None);
  assert_eq!(manifest.license, None);
  assert_eq!(manifest.repository, None);
  assert_eq!(manifest.authors.len(), 0);
  assert_eq!(manifest.dependencies.len(), 0);
}

#[test]
fn test_parse_manifest_with_deps() {
  // 의존성이 있는 매니페스트 파싱 테스트
  let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .unwrap()
    .parent()
    .unwrap()
    .join("fixtures/project/with-deps.toml");

  let manifest = load_manifest(&manifest_path).unwrap();

  assert_eq!(manifest.name, "with-deps");
  assert_eq!(manifest.version, "1.0.0");
  assert_eq!(manifest.dependencies.len(), 4);

  // 로컬 경로 의존성
  assert_eq!(manifest.dependencies[0].name, "local-lib");
  assert!(manifest.dependencies[0].path.is_some());

  // 버전 의존성
  assert_eq!(manifest.dependencies[1].name, "remote-lib");
  assert_eq!(manifest.dependencies[1].version, Some("2.0.0".to_string()));

  // Git 태그 의존성
  assert_eq!(manifest.dependencies[2].name, "git-lib");
  assert_eq!(
    manifest.dependencies[2].git,
    Some("https://github.com/example/git-lib".to_string())
  );
  assert_eq!(manifest.dependencies[2].tag, Some("v1.0.0".to_string()));

  // Git rev 의존성
  assert_eq!(manifest.dependencies[3].name, "git-lib-rev");
  assert_eq!(
    manifest.dependencies[3].git,
    Some("https://github.com/example/git-lib-rev".to_string())
  );
  assert_eq!(
    manifest.dependencies[3].rev,
    Some("abc123def456".to_string())
  );
}

#[test]
fn test_manifest_not_found() {
  // 존재하지 않는 매니페스트 파일 테스트
  let manifest_path = PathBuf::from("/nonexistent/pnix.toml");

  let result = load_manifest(&manifest_path);
  assert!(result.is_err());

  match result.unwrap_err() {
    ManifestError::NotFound(_) => {}
    _ => panic!("Expected NotFound error"),
  }
}

#[test]
fn test_manifest_invalid_toml() {
  // 잘못된 TOML 파일 테스트
  let temp_dir = TempDir::new().unwrap();
  let invalid_path = temp_dir.path().join("invalid.toml");

  // 잘못된 TOML 작성
  fs::write(&invalid_path, "name = [invalid").unwrap();

  let result = load_manifest(&invalid_path);
  assert!(result.is_err());

  match result.unwrap_err() {
    ManifestError::ParseError(_) => {}
    _ => panic!("Expected ParseError"),
  }
}

#[test]
fn test_manifest_empty_name() {
  // 빈 name 필드 테스트
  let temp_dir = TempDir::new().unwrap();
  let invalid_path = temp_dir.path().join("empty-name.toml");

  fs::write(&invalid_path, "name = \"\"\nversion = \"1.0.0\"").unwrap();

  let result = load_manifest(&invalid_path);
  assert!(result.is_err());

  match result.unwrap_err() {
    ManifestError::InvalidManifest(msg) => {
      assert!(msg.contains("name"));
    }
    _ => panic!("Expected InvalidManifest error"),
  }
}

#[test]
fn test_manifest_empty_version() {
  // 빈 version 필드 테스트
  let temp_dir = TempDir::new().unwrap();
  let invalid_path = temp_dir.path().join("empty-version.toml");

  fs::write(&invalid_path, "name = \"test\"\nversion = \"\"").unwrap();

  let result = load_manifest(&invalid_path);
  assert!(result.is_err());

  match result.unwrap_err() {
    ManifestError::InvalidManifest(msg) => {
      assert!(msg.contains("version"));
    }
    _ => panic!("Expected InvalidManifest error"),
  }
}

// ============================================================================
// Y10b: 의존성 해석 테스트
// ============================================================================

#[test]
fn test_resolve_simple_dependency() {
  // 단순 의존성 해석 테스트 (lib-b -> lib-a)
  let fixtures_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .unwrap()
    .parent()
    .unwrap()
    .join("fixtures/project/deps");

  let lib_b_path = fixtures_root.join("lib-b");
  let resolver = DependencyResolver::new(&fixtures_root);

  let graph = resolver.resolve(&lib_b_path).unwrap();

  // 프로젝트 확인
  assert_eq!(graph.projects.len(), 2);
  assert!(graph.projects.contains_key("lib-a"));
  assert!(graph.projects.contains_key("lib-b"));

  // 의존성 확인
  assert!(graph.dependencies.get("lib-b").unwrap().contains("lib-a"));
  assert!(graph.dependents.get("lib-a").unwrap().contains("lib-b"));

  // 빌드 순서 확인
  let build_order = graph.build_order().unwrap();
  assert_eq!(build_order.order.len(), 2);
  assert_eq!(build_order.order[0], "lib-a"); // lib-a가 먼저 빌드되어야 함
  assert_eq!(build_order.order[1], "lib-b");
}

#[test]
fn test_resolve_chain_dependency() {
  // 체인 의존성 해석 테스트 (app -> lib-c -> lib-b -> lib-a)
  let fixtures_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .unwrap()
    .parent()
    .unwrap()
    .join("fixtures/project/deps");

  let app_path = fixtures_root.join("app");
  let resolver = DependencyResolver::new(&fixtures_root);

  let graph = resolver.resolve(&app_path).unwrap();

  // 프로젝트 확인
  assert_eq!(graph.projects.len(), 4);
  assert!(graph.projects.contains_key("lib-a"));
  assert!(graph.projects.contains_key("lib-b"));
  assert!(graph.projects.contains_key("lib-c"));
  assert!(graph.projects.contains_key("app"));

  // 의존성 확인
  assert!(graph.dependencies.get("lib-b").unwrap().contains("lib-a"));
  assert!(graph.dependencies.get("lib-c").unwrap().contains("lib-a"));
  assert!(graph.dependencies.get("lib-c").unwrap().contains("lib-b"));
  assert!(graph.dependencies.get("app").unwrap().contains("lib-c"));

  // 빌드 순서 확인
  let build_order = graph.build_order().unwrap();
  assert_eq!(build_order.order.len(), 4);

  // lib-a가 가장 먼저 빌드되어야 함
  assert_eq!(build_order.order[0], "lib-a");

  // lib-b는 lib-a 다음
  assert_eq!(build_order.order[1], "lib-b");

  // lib-c는 lib-a와 lib-b 다음
  assert_eq!(build_order.order[2], "lib-c");

  // app은 마지막
  assert_eq!(build_order.order[3], "app");
}

#[test]
fn test_resolve_circular_dependency() {
  // 순환 의존성 검출 테스트 (cycle-a <-> cycle-b)
  let fixtures_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .unwrap()
    .parent()
    .unwrap()
    .join("fixtures/project/deps");

  let cycle_a_path = fixtures_root.join("cycle-a");
  let resolver = DependencyResolver::new(&fixtures_root);

  let graph = resolver.resolve(&cycle_a_path).unwrap();

  // 프로젝트 확인
  assert_eq!(graph.projects.len(), 2);
  assert!(graph.projects.contains_key("cycle-a"));
  assert!(graph.projects.contains_key("cycle-b"));

  // 순환 의존성 확인
  assert!(graph
    .dependencies
    .get("cycle-a")
    .unwrap()
    .contains("cycle-b"));
  assert!(graph
    .dependencies
    .get("cycle-b")
    .unwrap()
    .contains("cycle-a"));

  // 빌드 순서는 순환 의존성으로 인해 실패해야 함
  let result = graph.build_order();
  assert!(result.is_err());

  match result.unwrap_err() {
    ResolverError::CircularDependency(remaining) => {
      assert_eq!(remaining.len(), 2);
      assert!(remaining.contains(&"cycle-a".to_string()));
      assert!(remaining.contains(&"cycle-b".to_string()));
    }
    _ => panic!("Expected CircularDependency error"),
  }

  // 순환 검출 함수 테스트
  let cycles = graph.detect_cycles();
  assert!(!cycles.is_empty());
  // 순환 경로가 포함되어야 함
  assert!(cycles.iter().any(|cycle| {
    cycle.contains(&"cycle-a".to_string()) && cycle.contains(&"cycle-b".to_string())
  }));
}

#[test]
fn test_resolve_no_dependencies() {
  // 의존성 없는 프로젝트 해석 테스트
  let fixtures_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .unwrap()
    .parent()
    .unwrap()
    .join("fixtures/project/deps");

  let lib_a_path = fixtures_root.join("lib-a");
  let resolver = DependencyResolver::new(&fixtures_root);

  let graph = resolver.resolve(&lib_a_path).unwrap();

  // 프로젝트 확인
  assert_eq!(graph.projects.len(), 1);
  assert!(graph.projects.contains_key("lib-a"));

  // 의존성 없음
  assert!(graph.dependencies.get("lib-a").unwrap().is_empty());

  // 빌드 순서 확인
  let build_order = graph.build_order().unwrap();
  assert_eq!(build_order.order.len(), 1);
  assert_eq!(build_order.order[0], "lib-a");
}

// ============================================================================
// Y10c: 락 파일 생성 및 검증 테스트
// ============================================================================

#[test]
fn test_compute_project_hash() {
  // 프로젝트 해시 계산 테스트
  let fixtures_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .unwrap()
    .parent()
    .unwrap()
    .join("fixtures/project/deps");

  let lib_a_path = fixtures_root.join("lib-a");
  let manifest_path = lib_a_path.join("pnix.toml");
  let manifest = load_manifest(&manifest_path).unwrap();

  // 해시 계산
  let hash1 = compute_project_hash(&manifest, &lib_a_path).unwrap();
  let hash2 = compute_project_hash(&manifest, &lib_a_path).unwrap();

  // 동일한 입력은 동일한 해시 생성 (결정론)
  assert_eq!(hash1, hash2);
  assert_eq!(hash1.len(), 64); // SHA-256 hex (32 bytes * 2)
}

#[test]
fn test_generate_lock_file() {
  // 락 파일 생성 테스트
  let fixtures_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .unwrap()
    .parent()
    .unwrap()
    .join("fixtures/project/deps");

  let lib_b_path = fixtures_root.join("lib-b");
  let resolver = DependencyResolver::new(&fixtures_root);

  let graph = resolver.resolve(&lib_b_path).unwrap();

  // 락 파일 생성
  let lock = generate_lock_file(&graph, &lib_b_path).unwrap();

  // 락 파일 구조 확인
  assert_eq!(lock.version, "1.0");
  assert!(!lock.project_hash.is_empty());
  assert_eq!(lock.dependencies.len(), 1); // lib-a 의존성

  // 의존성 해시 확인
  assert!(lock.dependencies.contains_key("lib-a"));
  let lib_a_lock = lock.dependencies.get("lib-a").unwrap();
  assert!(!lib_a_lock.hash.is_empty());
  assert_eq!(lib_a_lock.hash.len(), 64);
}

#[test]
fn test_lock_file_save_and_load() {
  // 락 파일 저장 및 로드 테스트
  let temp_dir = TempDir::new().unwrap();
  let lock_path = temp_dir.path().join("pnix.lock");

  // 락 파일 생성
  let mut lock = LockFile::new("test-hash".to_string());
  lock.add_dependency(
    "test-dep".to_string(),
    pnix_executor_graph::project::DependencyLock {
      hash: "dep-hash".to_string(),
      path: Some(PathBuf::from("../test-dep")),
      version: Some("1.0.0".to_string()),
      git: None,
    },
  );

  // 저장
  lock.save(&lock_path).unwrap();

  // 로드
  let loaded = LockFile::load(&lock_path).unwrap();

  // 비교
  assert_eq!(lock.version, loaded.version);
  assert_eq!(lock.project_hash, loaded.project_hash);
  assert_eq!(lock.dependencies.len(), loaded.dependencies.len());
  assert_eq!(
    lock.dependencies.get("test-dep").unwrap().hash,
    loaded.dependencies.get("test-dep").unwrap().hash
  );
}

#[test]
fn test_lock_file_verify() {
  // 락 파일 검증 테스트
  let fixtures_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .unwrap()
    .parent()
    .unwrap()
    .join("fixtures/project/deps");

  let lib_b_path = fixtures_root.join("lib-b");
  let resolver = DependencyResolver::new(&fixtures_root);

  let graph = resolver.resolve(&lib_b_path).unwrap();

  // 락 파일 생성
  let lock = generate_lock_file(&graph, &lib_b_path).unwrap();

  // 검증 (성공해야 함)
  verify_lock_file(&lock, &graph, &lib_b_path).unwrap();
}

#[test]
fn test_lock_file_deterministic() {
  // 락 파일 결정론 테스트 (동일 소스 → 동일 락 파일)
  let fixtures_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .unwrap()
    .parent()
    .unwrap()
    .join("fixtures/project/deps");

  let lib_b_path = fixtures_root.join("lib-b");
  let resolver = DependencyResolver::new(&fixtures_root);

  // 첫 번째 락 파일 생성
  let graph1 = resolver.resolve(&lib_b_path).unwrap();
  let lock1 = generate_lock_file(&graph1, &lib_b_path).unwrap();

  // 두 번째 락 파일 생성 (동일한 입력)
  let graph2 = resolver.resolve(&lib_b_path).unwrap();
  let lock2 = generate_lock_file(&graph2, &lib_b_path).unwrap();

  // 동일한 해시 생성 (결정론)
  assert_eq!(lock1.project_hash, lock2.project_hash);
  assert_eq!(lock1.dependencies.len(), lock2.dependencies.len());

  for (name, dep1) in &lock1.dependencies {
    let dep2 = lock2.dependencies.get(name).unwrap();
    assert_eq!(dep1.hash, dep2.hash);
  }
}
