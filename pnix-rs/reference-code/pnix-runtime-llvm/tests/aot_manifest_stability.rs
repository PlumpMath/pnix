//! Test AOT manifest output stability
//!
//! Ensures that AOT manifest output is stable and deterministic
//! across multiple runs.

use pnix_runtime_llvm::{AotArtifactLayout, AotArtifactManifest, AotTarget};

#[test]
fn test_aot_manifest_stable_ordering() {
  // Create multiple manifests for same module
  let manifest1 = AotArtifactManifest::new(
    "test_module".to_string(),
    AotTarget::LinuxX86_64,
    "main".to_string(),
  );

  let manifest2 = AotArtifactManifest::new(
    "test_module".to_string(),
    AotTarget::LinuxX86_64,
    "main".to_string(),
  );

  // Serialize both to JSON
  let json1 = manifest1.to_json().unwrap();
  let json2 = manifest2.to_json().unwrap();

  // Should be identical (deterministic)
  assert_eq!(
    json1, json2,
    "Manifest JSON should be identical across runs"
  );

  // Verify no timestamps
  assert!(!json1.contains("\"build_timestamp\"") || json1.contains("\"build_timestamp\": null"));
}

#[test]
fn test_aot_layout_stable_paths() {
  // Create layouts for same module/target
  let layout1 = AotArtifactLayout::for_target(AotTarget::LinuxX86_64, "test_module");
  let layout2 = AotArtifactLayout::for_target(AotTarget::LinuxX86_64, "test_module");

  // Paths should be identical
  assert_eq!(layout1.binary_path, layout2.binary_path);
  assert_eq!(layout1.manifest_path, layout2.manifest_path);
  assert_eq!(layout1.library_path, layout2.library_path);
}

#[test]
fn test_aot_manifest_all_targets_stable() {
  // Test all targets produce stable manifests
  let targets = vec![
    AotTarget::LinuxX86_64,
    AotTarget::MacOSX86_64,
    AotTarget::MacOSArm64,
    AotTarget::WindowsX86_64,
  ];

  for target in targets {
    let manifest1 = AotArtifactManifest::new("test".to_string(), target, "main".to_string());

    let manifest2 = AotArtifactManifest::new("test".to_string(), target, "main".to_string());

    let json1 = manifest1.to_json().unwrap();
    let json2 = manifest2.to_json().unwrap();

    assert_eq!(
      json1, json2,
      "Manifest should be stable for target {:?}",
      target
    );
  }
}
