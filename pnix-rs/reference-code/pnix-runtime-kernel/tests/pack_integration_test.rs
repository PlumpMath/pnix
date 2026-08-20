//! SETO Pack 런타임 통합 테스트
//!
//! 실제 런타임에서 pack 파일을 로드하고 사용하는지 확인

#[cfg(all(feature = "seto", feature = "seto-pack", feature = "seto-pack-json"))]
mod tests {
  use pnix_runtime_kernel::SetoRegistry;
  use std::path::PathBuf;

  #[test]
  #[ignore] // 실제 pack 파일이 필요함
  fn test_runtime_load_pack() {
    let pack_path = PathBuf::from("/tmp/setopack.bin");

    if !pack_path.exists() {
      eprintln!("Skipping test: pack file not found at {:?}", pack_path);
      return;
    }

    let mut registry = SetoRegistry::new();

    // Pack 파일에서 로드
    match registry.load_from_pack(&pack_path) {
      Ok(()) => {
        let report = registry.report();
        println!("Successfully loaded pack:");
        println!("  SETO files: {}", report.seto_files);
        println!("  Rule files: {}", report.rule_files);
        println!("  SETO defs: {}", report.seto_defs);
        println!("  Rule defs: {}", report.rule_defs);
        println!("  Domains: {:?}", report.domains);

        assert!(report.seto_defs > 0, "Should have loaded at least one SETO");
      }
      Err(e) => {
        panic!("Failed to load pack: {:?}", e);
      }
    }
  }

  #[test]
  #[ignore]
  fn test_runtime_pack_vs_legacy() {
    // Pack 로딩과 Legacy 로딩이 같은 결과를 주는지 확인
    // (실제 구현은 나중에)
  }
}
