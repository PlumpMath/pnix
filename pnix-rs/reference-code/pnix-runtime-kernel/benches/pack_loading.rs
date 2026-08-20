//! SETO Pack 로딩 성능 벤치마크
//!
//! Legacy TOML/DB 로딩 vs Pack 로딩 비교

#[cfg(feature = "seto-pack")]
use pnix_seto_pack::SetoPack;
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::time::Instant;

fn run_bench(label: &str, iterations: usize, mut f: impl FnMut()) {
  let start = Instant::now();
  for _ in 0..iterations {
    f();
  }
  let elapsed = start.elapsed();
  let per_iter_us = elapsed.as_secs_f64() * 1_000_000.0 / iterations.max(1) as f64;
  println!(
    "{}: {} iterations in {:?} ({:.3} us/iter)",
    label, iterations, elapsed, per_iter_us
  );
}

fn bench_legacy_loading() {
  let seto_root = PathBuf::from("data/meaning");
  let rules_root = PathBuf::from("data/meaning/rules");

  if !seto_root.exists() || !rules_root.exists() {
    eprintln!("Skipping benchmark: data directories not found");
    return;
  }

  run_bench("legacy_seto_tree_scan", 100, || {
    let count = count_files(&seto_root);
    black_box(count);
  });
}

fn bench_pack_loading() {
  let pack_path = PathBuf::from("/tmp/setopack.bin");

  if !pack_path.exists() {
    eprintln!("Skipping benchmark: pack file not found at {:?}", pack_path);
    return;
  }

  #[cfg(feature = "seto-pack")]
  run_bench("pack_seto_loading", 100, || match SetoPack::load(&pack_path) {
    Ok(pack) => {
      black_box(pack);
    }
    Err(e) => eprintln!("Pack loading error: {:?}", e),
  });

  #[cfg(not(feature = "seto-pack"))]
  eprintln!("Skipping pack loader benchmark: build without seto-pack feature");
}

fn bench_comparison() {
  let pack_path = PathBuf::from("/tmp/setopack.bin");
  let seto_root = PathBuf::from("data/meaning");

  if !pack_path.exists() || !seto_root.exists() {
    eprintln!("Skipping comparison: required files not found");
    return;
  }

  run_bench("seto_loading_comparison/legacy_tree_scan", 100, || {
    let count = count_files(&seto_root);
    black_box(count);
  });

  // Pack 로딩
  #[cfg(feature = "seto-pack")]
  {
    run_bench("seto_loading_comparison/pack", 100, || {
      if let Ok(pack) = SetoPack::load(&pack_path) {
        black_box(pack);
      }
    });
  }
}

fn count_files(root: &Path) -> usize {
  let mut count = 0usize;
  let mut stack = vec![root.to_path_buf()];
  while let Some(dir) = stack.pop() {
    let Ok(entries) = std::fs::read_dir(&dir) else {
      continue;
    };
    for entry in entries.flatten() {
      let Ok(file_type) = entry.file_type() else {
        continue;
      };
      if file_type.is_dir() {
        stack.push(entry.path());
      } else if file_type.is_file() {
        count = count.saturating_add(1);
      }
    }
  }
  count
}

fn main() {
  bench_legacy_loading();
  bench_pack_loading();
  bench_comparison();
}
