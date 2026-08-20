//! SETO Pack 로딩 성능 측정
//!
//! 실제 로딩 시간을 측정하여 성능 개선 효과 확인

use std::path::PathBuf;

fn main() {
  let pack_path = PathBuf::from("/tmp/setopack.bin");

  if !pack_path.exists() {
    eprintln!("Pack file not found at {:?}", pack_path);
    eprintln!("Please run: cargo run --package pnix-ego-sphere --bin pnix_setoc --features seto-pack-writer");
    return;
  }

  #[cfg(all(feature = "seto", feature = "seto-pack", feature = "seto-pack-json"))]
  {
    use pnix_runtime_kernel::SetoRegistry;

    println!("=== Pack 로딩 성능 측정 ===\n");

    let mut times = Vec::new();
    for i in 0..20 {
      let start = Instant::now();
      let mut registry = SetoRegistry::new();
      if let Err(e) = registry.load_from_pack(&pack_path) {
        eprintln!("Error on run {}: {:?}", i, e);
        continue;
      }
      let duration = start.elapsed();
      times.push(duration.as_nanos());

      if i == 0 {
        let report = registry.report();
        println!("로드된 데이터:");
        println!("  SETOs: {}", report.seto_defs);
        println!("  Rules: {}", report.rule_defs);
        println!("  Domains: {:?}", report.domains);
        println!();
      }
    }

    if !times.is_empty() {
      let avg = times.iter().sum::<u128>() / times.len() as u128;
      let min = *times.iter().min().unwrap();
      let max = *times.iter().max().unwrap();

      println!("측정 결과 ({}회):", times.len());
      println!(
        "  평균: {:.2}ms ({:.2}μs)",
        avg as f64 / 1_000_000.0,
        avg as f64 / 1_000.0
      );
      println!(
        "  최소: {:.2}ms ({:.2}μs)",
        min as f64 / 1_000_000.0,
        min as f64 / 1_000.0
      );
      println!(
        "  최대: {:.2}ms ({:.2}μs)",
        max as f64 / 1_000_000.0,
        max as f64 / 1_000.0
      );
    }
  }

  #[cfg(not(all(feature = "seto", feature = "seto-pack", feature = "seto-pack-json")))]
  {
    eprintln!("Required features not enabled. Use:");
    eprintln!("  cargo run --example measure_pack_perf --features seto,seto-pack,seto-pack-json");
  }
}
