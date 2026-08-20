//! ratchet: `fixtures/pnix-query-runtime/*.px` 는 첫 비공백 줄이
//! `# <filename.px>` 주석이어야 한다.
//!
//! 기존 5 개 fixture 모두 이 convention 을 따른다 (batch 263 alignment ratchet
//! 작성 시 확인). README 가 fixture 를 카테고리 표로 관리하는 만큼, 파일 자체가
//! 자기 이름을 선언하지 않으면 rename / copy-paste drift 가 섞일 수 있다.
//! self-identifying header 를 정본으로 고정한다.
//!
//! 검증:
//!   - 첫 non-empty line 이 `# ` 로 시작한다
//!   - 뒤따르는 토큰이 파일 basename (`.px` 포함) 과 일치한다

use std::path::{Path, PathBuf};

fn fixtures_dir() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .and_then(Path::parent)
    .expect("CARGO_MANIFEST_DIR must resolve to workspace root via two parents")
    .join("fixtures/pnix-query-runtime")
}

fn first_non_empty_line(body: &str) -> Option<&str> {
  body.lines().find(|l| !l.trim().is_empty())
}

fn check_file(path: &Path) -> Option<String> {
  let basename = path.file_name()?.to_str()?;
  let body = std::fs::read_to_string(path)
    .map_err(|e| format!("read {}: {e}", path.display()))
    .ok()?;
  let Some(first) = first_non_empty_line(&body) else {
    return Some(format!("{}: empty file", path.display()));
  };
  let expected = format!("# {basename}");
  if first.trim_end() == expected {
    None
  } else {
    Some(format!(
      "{}: first line `{}` must equal `{}`",
      path.display(),
      first.trim_end(),
      expected
    ))
  }
}

#[test]
fn fixtures_must_start_with_filename_comment() {
  let root = fixtures_dir();
  assert!(
    root.is_dir(),
    "fixtures/pnix-query-runtime missing at {}",
    root.display()
  );
  let mut fails = Vec::new();
  let entries =
    std::fs::read_dir(&root).unwrap_or_else(|e| panic!("read_dir {}: {e}", root.display()));
  for entry in entries.flatten() {
    let path = entry.path();
    if path.extension().and_then(|s| s.to_str()) != Some("px") {
      continue;
    }
    if let Some(fail) = check_file(&path) {
      fails.push(fail);
    }
  }
  if !fails.is_empty() {
    for f in &fails {
      eprintln!("FAIL {f}");
    }
    panic!("{} fixture header convention violation(s)", fails.len());
  }
}
