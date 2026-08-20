//! ratchet: `fixtures/pnix-query-runtime/README.md` 와 on-disk fixture 목록이
//! 양방향으로 정렬되어야 한다.
//!
//! `fixtures/pnix-query-runtime/README.md` 는 각 fixture 를 카테고리 (A/B/C)
//! 별 표로 문서화하고, fixture 규칙 섹션 (`## 규칙`) 이 "gate 가 없는 fixture
//! 는 정본이 아니라 scratch" 를 규정한다. 따라서 on-disk 에 `.px` 가 있는데
//! README 에 언급되지 않으면 scratch 거나 drift 다. 반대로 README 가 언급하는
//! `.px` 가 on-disk 에 없으면 orphan 문서다.
//!
//! 이 ratchet 은 batch 261 의 `parse_all_fixture_px_files` 와 대칭이다:
//!  - parse ratchet: fixture 가 parser contract 를 지키는가
//!  - alignment ratchet: fixture 가 README 와 존재 양방향 match 하는가

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn fixtures_dir() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .and_then(Path::parent)
    .expect("CARGO_MANIFEST_DIR must resolve to workspace root via two parents")
    .join("fixtures/pnix-query-runtime")
}

fn on_disk_px_basenames(root: &Path) -> BTreeSet<String> {
  let mut out = BTreeSet::new();
  let entries = std::fs::read_dir(root).unwrap_or_else(|e| {
    panic!("fixtures/pnix-query-runtime not readable: {e}");
  });
  for entry in entries.flatten() {
    let path = entry.path();
    if path.extension().and_then(|s| s.to_str()) != Some("px") {
      continue;
    }
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
      out.insert(name.to_string());
    }
  }
  out
}

fn readme_referenced_px_basenames(readme: &Path) -> BTreeSet<String> {
  let body = std::fs::read_to_string(readme)
    .unwrap_or_else(|e| panic!("cannot read {}: {e}", readme.display()));
  // backtick-wrapped `*.px` 문자열을 모두 수집. 표 셀/인라인 모두 대응.
  let mut out = BTreeSet::new();
  let mut rest = body.as_str();
  while let Some(start) = rest.find('`') {
    rest = &rest[start + 1..];
    let Some(end) = rest.find('`') else { break };
    let token = &rest[..end];
    rest = &rest[end + 1..];
    // 경로 구분자 포함 토큰은 제외 (예: `scripts/run-px-phase7-gate.sh`).
    // fixture README 의 fixture 참조는 항상 basename 형태.
    if token.ends_with(".px") && !token.contains('/') && !token.contains(' ') {
      // `.px` 단독 (확장자 설명) / `*.px` glob 은 fixture 참조가 아니다.
      // stem 이 존재하고 alphanumeric + `-_` 조합일 때만 fixture 로 본다.
      let stem = &token[..token.len() - 3];
      let is_filename_stem = !stem.is_empty()
        && stem
          .chars()
          .all(|c| c.is_alphanumeric() || c == '-' || c == '_');
      if is_filename_stem {
        out.insert(token.to_string());
      }
    }
  }
  out
}

#[test]
fn fixture_files_and_readme_references_must_align() {
  let root = fixtures_dir();
  let readme = root.join("README.md");
  assert!(
    readme.is_file(),
    "README.md missing at {}",
    readme.display()
  );

  let on_disk = on_disk_px_basenames(&root);
  let referenced = readme_referenced_px_basenames(&readme);

  let undocumented: Vec<&String> = on_disk.difference(&referenced).collect();
  let orphan: Vec<&String> = referenced.difference(&on_disk).collect();

  let mut failed = false;
  if !undocumented.is_empty() {
    eprintln!(
      "FAIL: fixture on disk but not referenced in README.md: {:?}",
      undocumented
    );
    failed = true;
  }
  if !orphan.is_empty() {
    eprintln!(
      "FAIL: README.md references fixture not on disk: {:?}",
      orphan
    );
    failed = true;
  }
  if failed {
    panic!(
      "README ↔ fixture alignment drift (on_disk={} referenced={})",
      on_disk.len(),
      referenced.len()
    );
  }
}
