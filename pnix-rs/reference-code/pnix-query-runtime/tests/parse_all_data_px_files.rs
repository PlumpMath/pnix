//! ratchet: `crates/pnix-query-runtime/data/` 아래 모든 `.px` 파일이
//! `pnix_query_runtime::px::parse_px_file` 로 실제 parse 되어야 한다.
//!
//! kernel 은 data/ 의 일부 파일만 실제로 load 하므로, parse 실패가
//! silent 하게 숨는 경우가 있다 (예: stale syntax 나 pnix-query-runtime
//! 쪽 parser 만의 restriction 위반). 이 test 는 모든 `.px` 가 적어도
//! parse 에는 성공하는지 full sweep 으로 보호한다.
//!
//! kernel 바깥 mirrored data tree 에는 이미 동일 ratchet 이 있지만 그쪽만
//! scan 하므로 pnix-query-runtime/data 쪽은 커버되지 않았다. 두 쪽은
//! `data_px_mirror_drift` ratchet 으로 내용 byte-identical 이 강제되지만,
//! 양쪽 parser 구현이 미묘하게 달라 drift 가 먼저 생길 수 있다.

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

#[test]
fn all_data_px_files_must_parse() {
  let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data");
  let mut fails = Vec::new();
  walk(&root, &mut fails);
  if !fails.is_empty() {
    for f in &fails {
      eprintln!("FAIL {f}");
    }
    panic!("{} .px file(s) failed to parse", fails.len());
  }
}
