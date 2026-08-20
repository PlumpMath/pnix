//! SETO 로딩 성능 비교: Legacy vs Pack
//!
//! 실제 로딩 시간을 측정하여 성능 개선 효과 확인

use std::path::PathBuf;

fn main() {
  println!("=== SETO 로딩 성능 비교 ===\n");

  // Pack 로딩 측정
  let pack_path = PathBuf::from("/tmp/setopack.bin");
  if pack_path.exists() {
    #[cfg(all(feature = "seto", feature = "seto-pack", feature = "seto-pack-json"))]
    {
      use pnix_runtime_kernel::SetoRegistry;

      println!("1. Pack 로딩 측정:");
      let mut pack_times = Vec::new();
      for _ in 0..10 {
        let start = Instant::now();
        let mut registry = SetoRegistry::new();
        if registry.load_from_pack(&pack_path).is_ok() {
          let duration = start.elapsed();
          pack_times.push(duration.as_nanos());
          if pack_times.len() == 1 {
            let report = registry.report();
            println!(
              "   로드된 데이터: SETO {}개, Rules {}개",
              report.seto_defs, report.rule_defs
            );
          }
        }
      }

      if !pack_times.is_empty() {
        let avg = pack_times.iter().sum::<u128>() / pack_times.len() as u128;
        let min = *pack_times.iter().min().unwrap();
        let max = *pack_times.iter().max().unwrap();
        println!(
          "   평균: {:.2}ms ({:.2}μs)",
          avg as f64 / 1_000_000.0,
          avg as f64 / 1_000.0
        );
        println!(
          "   최소: {:.2}ms ({:.2}μs)",
          min as f64 / 1_000_000.0,
          min as f64 / 1_000.0
        );
        println!(
          "   최대: {:.2}ms ({:.2}μs)",
          max as f64 / 1_000_000.0,
          max as f64 / 1_000.0
        );
        println!();
      }
    }
  } else {
    println!("1. Pack 로딩: Pack 파일 없음 (/tmp/setopack.bin)\n");
  }

  // Legacy 로딩 측정
  let seto_root = PathBuf::from("data/meaning");
  if seto_root.exists() {
    #[cfg(feature = "seto-legacy")]
    {
      use pnix_runtime_kernel::SetoRegistry;

      println!("2. Legacy 로딩 측정 (TOML/DB 스캔):");
      let mut legacy_times = Vec::new();
      for _ in 0..10 {
        let start = Instant::now();
        let mut registry = SetoRegistry::new();
        if registry.load_legacy().is_ok() {
          let duration = start.elapsed();
          legacy_times.push(duration.as_nanos());
          if legacy_times.len() == 1 {
            let report = registry.report();
            println!(
              "   로드된 데이터: SETO {}개, Rules {}개",
              report.seto_defs, report.rule_defs
            );
          }
        }
      }

      if !legacy_times.is_empty() {
        let avg = legacy_times.iter().sum::<u128>() / legacy_times.len() as u128;
        let min = *legacy_times.iter().min().unwrap();
        let max = *legacy_times.iter().max().unwrap();
        println!(
          "   평균: {:.2}ms ({:.2}μs)",
          avg as f64 / 1_000_000.0,
          avg as f64 / 1_000.0
        );
        println!(
          "   최소: {:.2}ms ({:.2}μs)",
          min as f64 / 1_000_000.0,
          min as f64 / 1_000.0
        );
        println!(
          "   최대: {:.2}ms ({:.2}μs)",
          max as f64 / 1_000_000.0,
          max as f64 / 1_000.0
        );
        println!();
      }
    }
  } else {
    println!("2. Legacy 로딩: 데이터 디렉토리 없음 (data/meaning)\n");
  }

  println!("=== 측정 완료 ===");
}
