//! Runtime symbol prohibition test
//!
//! pnix-core에 런타임/실행 심볼이 침투하지 않았는지 검증

use std::fs;

#[test]
fn forbid_runtime_symbols_in_src() {
  let bad = [
    "SystemTime",
    "Instant",
    "std::time::Duration",
    "core::time::Duration",
    "tokio",
    "reqwest",
    "std::fs",
    "std::process::Command",
  ];

  // emit_fs.rs 제거됨 - fs I/O는 executor에서 구현
  let allowed: [&str; 0] = [];

  // src/만 검사 (docs/examples 제외)
  let mut stack = vec![std::path::PathBuf::from("src")];
  while let Some(p) = stack.pop() {
    if p.is_dir() {
      for e in fs::read_dir(&p).unwrap() {
        stack.push(e.unwrap().path());
      }
    } else if p.extension().map(|x| x == "rs").unwrap_or(false) {
      // 허용된 파일은 건너뛰기
      let filename = p.file_name().and_then(|x| x.to_str()).unwrap_or("");
      if allowed.contains(&filename) {
        continue;
      }

      let s = fs::read_to_string(&p).unwrap();
      for k in bad {
        assert!(
          !s.contains(k),
          "forbidden runtime symbol `{}` detected in {}",
          k,
          p.display()
        );
      }
    }
  }
}
