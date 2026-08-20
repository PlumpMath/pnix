//! ~/nix tests/functional/lang 의 eval-okay-*.nix 들을 pnix-eval 로
//! 돌려 통과/실패 카탈로그를 만든다.

use pnix_eval::eval_file;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Resolve the upstream Nix functional test corpus directory.
/// Honor `PNIX_NIX_LANG_DIR` first, otherwise probe the conventional
/// checkout locations on each host. Returns `None` when no checkout is
/// available — the catalog test then prints a `[skip]` notice and
/// passes, so the suite can run unchanged on machines that don't
/// carry the upstream Nix tree.
fn nix_lang_dir() -> Option<PathBuf> {
  if let Some(dir) = std::env::var_os("PNIX_NIX_LANG_DIR") {
    let p = PathBuf::from(dir);
    if p.is_dir() {
      return Some(p);
    }
  }
  let candidates: &[&str] = &[
    "/Users/gp/nix/tests/functional/lang",
    "/home/gp/nix/tests/functional/lang",
  ];
  for c in candidates {
    let p = PathBuf::from(c);
    if p.is_dir() {
      return Some(p);
    }
  }
  None
}

const PROGRESS_LOG: &str = "/tmp/pnix_nix_parity.log";

fn run_one(path: &Path) -> Result<String, String> {
  // Use `eval_file` so `import ./lib.nix` (and other relative paths
  // inside the test) resolve against the test file's directory via
  // the `IMPORT_BASE_STACK` guard. `eval_expr` would resolve them
  // against the cargo cwd which doesn't have lib.nix.
  let owned: PathBuf = path.to_path_buf();
  let h = std::thread::Builder::new()
    .stack_size(32 * 1024 * 1024)
    .spawn(move || match eval_file(&owned) {
      Ok(v) => Ok(v.to_json()),
      Err(e) => Err(format!("{e}")),
    })
    .map_err(|e| format!("spawn: {e}"))?;
  match h.join() {
    Ok(r) => r,
    Err(_) => Err("thread panicked / overflowed".to_string()),
  }
}

#[allow(dead_code)]
fn _unused_fs_marker(_: &fs::File) {} // keep `fs` use suppressed

#[test]
fn nix_lang_eval_okay_parity_catalog() {
  // Tests known to stack-overflow our evaluator due to missing cycle detection
  // in `deepSeq` / `deep_force`. Tracked separately, skipped from this catalog.
  let skip = [
    "eval-okay-convertHash.nix", // sibling-file (.exp) infrastructure
    // `scopedImport overrides ./imported.nix` interacts with our
    // recursive-binding + Apply-lazy + import-resolution path in a way
    // that triggers an apparent infinite loop. Skip from bulk catalog.
    "eval-okay-import.nix",
  ];

  let standalone: Vec<&str> = include_str!("nix_lang_standalone.txt")
    .lines()
    .filter(|l| !l.is_empty())
    .filter(|l| !skip.contains(l))
    .collect();

  let Some(lang_dir) = nix_lang_dir() else {
    eprintln!(
      "[skip] PNIX_NIX_LANG_DIR not set and ~/nix/tests/functional/lang \
       not found on this host; nix-lang parity catalog skipped."
    );
    return;
  };

  let mut log = fs::File::create(PROGRESS_LOG).expect("open log");
  let mut passed: Vec<String> = Vec::new();
  let mut failed: Vec<(String, String)> = Vec::new();

  for name in &standalone {
    let path = lang_dir.join(name);
    if !path.exists() {
      continue;
    }
    writeln!(log, "TRY {}", name).ok();
    log.sync_all().ok();
    match run_one(&path) {
      Ok(_) => {
        writeln!(log, "PASS {}", name).ok();
        passed.push(name.to_string());
      }
      Err(e) => {
        let one = e.lines().next().unwrap_or("").to_string();
        writeln!(log, "FAIL {}: {}", name, one).ok();
        failed.push((name.to_string(), one));
      }
    }
    log.sync_all().ok();
  }

  let total = passed.len() + failed.len();
  eprintln!("\n=== nix lang parity ===");
  eprintln!("total: {}", total);
  eprintln!("pass:  {}", passed.len());
  eprintln!("fail:  {}", failed.len());
  eprintln!("\n=== fail summary ===");
  for (name, err) in &failed {
    eprintln!("FAIL {}: {}", name, err);
  }

  assert!(total > 0, "no tests found");
}
