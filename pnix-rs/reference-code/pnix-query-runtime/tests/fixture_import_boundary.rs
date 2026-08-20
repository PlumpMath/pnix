//! ratchet: `fixtures/pnix-query-runtime/*.px` 는 host Rust 소스 트리를 import
//! 하지 않는다.
//!
//! `fixtures/pnix-query-runtime/README.md` §규칙:
//!   > fixture 는 host source import 를 하지 않는다. 카테고리 A
//!   > 의 mirrored `.px` data resource 는 허용 예외.
//!
//! 현재 이 규칙은 README 에만 선언되어 있고 enforcement 가 없다. fixture 가
//! standalone pnix-eval contract 를 깨고 Rust 소스 트리를 참조하기 시작하면
//! proof 의 의미가 무너진다 (doghouse 없이 돌아야 할 fixture 가 doghouse
//! 소스에 의존). 이 ratchet 이 정적으로 차단한다.
//!
//! 허용:
//!   - `.px` data resource 경로 (카테고리 A — mirrored data resource)
//!   - pnix stdlib / pnix-core builtin (file path 참조 없이)
//!
//! 금지:
//!   - `crates/doghouse-core/src/**`
//!   - former removed control source trees such as old `puck/src/**`
//!   - `crates/freecat-*` Rust 소스
//!
//! comment (`#` 시작) 는 검사에서 제외. 실제 code line 만 검사.

use std::path::{Path, PathBuf};

const FORBIDDEN_PATH_SEGMENTS: &[&str] = &[
  "crates/doghouse-core/src",
  "/puck/",
  "crates/pnix-core/src",
  "crates/freecat-",
];

fn fixtures_dir() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .and_then(Path::parent)
    .expect("CARGO_MANIFEST_DIR must resolve to workspace root via two parents")
    .join("fixtures/pnix-query-runtime")
}

fn scan_file(path: &Path) -> Vec<String> {
  let body = match std::fs::read_to_string(path) {
    Ok(b) => b,
    Err(e) => return vec![format!("{}: read error: {e}", path.display())],
  };
  let mut violations = Vec::new();
  for (idx, line) in body.lines().enumerate() {
    // comment line 은 제외. pnix `.px` 는 `#` 한 줄 주석만 지원.
    let trimmed = line.trim_start();
    if trimmed.starts_with('#') {
      continue;
    }
    for forbidden in FORBIDDEN_PATH_SEGMENTS {
      if line.contains(forbidden) {
        violations.push(format!(
          "{}:{}: forbidden path segment `{}` in non-comment line: {}",
          path.display(),
          idx + 1,
          forbidden,
          line.trim()
        ));
      }
    }
  }
  violations
}

fn walk(dir: &Path, out: &mut Vec<String>) {
  let entries = match std::fs::read_dir(dir) {
    Ok(e) => e,
    Err(_) => return,
  };
  for entry in entries.flatten() {
    let path = entry.path();
    if path.is_dir() {
      walk(&path, out);
    } else if path.extension().and_then(|s| s.to_str()) == Some("px") {
      out.extend(scan_file(&path));
    }
  }
}

#[test]
fn fixtures_must_not_import_rust_sources() {
  let root = fixtures_dir();
  assert!(
    root.is_dir(),
    "fixtures/pnix-query-runtime missing at {}",
    root.display()
  );
  let mut violations = Vec::new();
  walk(&root, &mut violations);
  if !violations.is_empty() {
    for v in &violations {
      eprintln!("FAIL {v}");
    }
    panic!("{} fixture import-boundary violation(s)", violations.len());
  }
}
