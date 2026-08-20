//! ratchet: `fixtures/pnix-query-runtime/` 아래 모든 `.px` 파일이
//! `pnix_query_runtime::px::parse_px_file` 로 실제 parse 되어야 한다.
//!
//! 이 디렉토리는 `scripts/run-px-phase7-gate.sh` 의 proof fixture 집합이다
//! (참조: `fixtures/pnix-query-runtime/README.md`). gate 는 각 fixture 를
//! `pnixc --expr --emit eval` 또는 `pnix-query-repl --file` 로 실행해서
//! 출력 marker 를 검증하지만, parser restriction 이 바뀌어 fixture 가 silent
//! 하게 parse 되지 못하게 되는 regression 은 이 ratchet 이 먼저 잡는다.
//!
//! `parse_all_data_px_files` ratchet 이 `crates/pnix-query-runtime/data/` 를
//! sweep 하는 것과 대칭이다 — 두 쪽의 parser contract 가 proof fixture 에도
//! 동일하게 적용되는지를 lock 한다.

use std::path::{Path, PathBuf};

fn walk(dir: &Path, fails: &mut Vec<String>) {
  let entries = match std::fs::read_dir(dir) {
    Ok(e) => e,
    Err(_) => return,
  };
  for entry in entries.flatten() {
    let path = entry.path();
    if path.is_dir() {
      walk(&path, fails);
    } else if path.extension().and_then(|s| s.to_str()) == Some("px") {
      if let Err(e) = pnix_query_runtime::px::parse_px_file(&path) {
        fails.push(format!("{}: {e}", path.display()));
      }
    }
  }
}

fn fixtures_dir() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .and_then(Path::parent)
    .expect("CARGO_MANIFEST_DIR must resolve to workspace root via two parents")
    .join("fixtures/pnix-query-runtime")
}

#[test]
fn all_fixture_px_files_must_parse() {
  let root = fixtures_dir();
  assert!(
    root.is_dir(),
    "fixtures/pnix-query-runtime must exist at {}",
    root.display()
  );
  let mut fails = Vec::new();
  walk(&root, &mut fails);
  if !fails.is_empty() {
    for f in &fails {
      eprintln!("FAIL {f}");
    }
    panic!("{} .px fixture file(s) failed to parse", fails.len());
  }
}
