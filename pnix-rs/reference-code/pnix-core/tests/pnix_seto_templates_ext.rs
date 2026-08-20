//! PNIX SETO 템플릿 확장 테스트: PNIX SETO 템플릿 확장 기능 테스트
//!
//! PNIX SETO 템플릿의 확장 기능을 테스트합니다.

use std::fs;
use std::path::PathBuf;

use pnix_core::lang::layer::build_capability_map;
use pnix_core::lang::layer::capability_map_path_for_package;
use pnix_core::lang::layer::load_layer_laws_for_package_with;
use pnix_core::lang::layer::load_layer_schema_pnix_with_base;
use pnix_core::lang::layer::load_layer_verification_report_for_package_with;
use pnix_core::lang::layer::load_verification_report_pnix;
use pnix_core::lang::layer::update_capability_map_from_schema_with;
use pnix_core::lang::layer::write_capability_map_for_package_configured_with;
use pnix_core::lang::layer::write_capability_map_for_package_with;
use pnix_core::lang::layer::LayerEngine;
use pnix_core::lang::layer::LayerFileSpec;
use pnix_core::lang::layer::LayerParseError;
use pnix_core::lang::pnix::parse_expr;

fn template_path(rel: &str) -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .join("..")
    .join("..")
    .join(rel)
}

fn read_template(rel: &str) -> String {
  let path = template_path(rel);
  fs::read_to_string(&path).unwrap_or_else(|err| {
    panic!("failed to read {}: {}", path.display(), err);
  })
}

fn read_file(path: &std::path::Path) -> Result<String, LayerParseError> {
  fs::read_to_string(path).map_err(|err| LayerParseError::Io {
    path: path.display().to_string(),
    message: err.to_string(),
  })
}

fn write_file(path: &std::path::Path, contents: &str) -> Result<(), LayerParseError> {
  fs::write(path, contents).map_err(|err| LayerParseError::Io {
    path: path.display().to_string(),
    message: err.to_string(),
  })
}

#[test]
fn parse_additional_templates() {
  let files = [
    "docs/spec/pnix-seto/templates/example_module_schema.px",
    "docs/spec/pnix-seto/schema.px",
    "docs/spec/pnix-seto/templates/layer_math_nat.px",
    "docs/spec/pnix-seto/templates/layer_math_nat.grammar.px",
    "docs/spec/pnix-seto/templates/layer_math_nat.meaning.px",
    "docs/spec/pnix-seto/templates/layer_math_nat.operators.px",
    "docs/spec/pnix-seto/templates/layer_math_int.px",
    "docs/spec/pnix-seto/templates/layer_math_int.grammar.px",
    "docs/spec/pnix-seto/templates/layer_math_int.meaning.px",
    "docs/spec/pnix-seto/templates/layer_math_int.operators.px",
    "docs/spec/pnix-seto/templates/operator_pack_algebra.px",
    "docs/spec/pnix-seto/templates/examples/index.px",
    "docs/spec/pnix-seto/templates/overlay_spec.px",
    "docs/spec/pnix-seto/templates/law_spec.px",
    "docs/spec/pnix-seto/templates/capability_map.px",
    "docs/spec/pnix-seto/templates/evidence_pack.px",
    "docs/spec/pnix-seto/templates/verification_report.px",
    "docs/spec/pnix-seto/templates/reports/math_nat.report.px",
    "docs/spec/pnix-seto/templates/reports/math_int.report.px",
    "docs/spec/pnix-seto/templates/reports/math_rat.report.px",
    "docs/spec/pnix-seto/templates/reports/lang_ko_grammar.report.px",
    "docs/spec/pnix-seto/templates/promotion_record.px",
  ];
  for rel in files {
    let src = read_template(rel);
    parse_expr(&src).unwrap();
  }
}

#[test]
fn merge_operator_pack_laws_into_layer() {
  let schema_path = template_path("docs/spec/pnix-seto/templates/example_module_schema.px");
  let schema_src = fs::read_to_string(&schema_path).unwrap();
  let registry =
    load_layer_schema_pnix_with_base(&schema_src, schema_path.parent(), Some(&read_file)).unwrap();
  let package = registry.packages.get("math.rat").unwrap();
  let base_dir = schema_path.parent().unwrap();
  let laws = load_layer_laws_for_package_with(package, base_dir, &read_file).unwrap();
  assert!(laws.contains("assoc"), "expected pack law 'assoc' to merge");
}

#[test]
fn build_capability_map_from_report() {
  let schema_path = template_path("docs/spec/pnix-seto/templates/example_module_schema.px");
  let schema_src = fs::read_to_string(&schema_path).unwrap();
  let registry =
    load_layer_schema_pnix_with_base(&schema_src, schema_path.parent(), Some(&read_file)).unwrap();
  let package = registry.packages.get("math.rat").unwrap();
  let base_dir = schema_path.parent().unwrap();
  let laws = load_layer_laws_for_package_with(package, base_dir, &read_file).unwrap();

  let report_path = template_path("docs/spec/pnix-seto/templates/verification_report.px");
  let report_src = fs::read_to_string(&report_path).unwrap();
  let report = load_verification_report_pnix(&report_src).unwrap();
  assert!(report.passed.contains("dot_comm"));
  assert!(report.failed.contains("dot_distrib"));

  let map = build_capability_map(package, &laws, Some(&report));
  assert_eq!(map.layer, "math.rat");
  assert!(!map.laws.is_empty());
  assert!(map.coverage >= 0.0 && map.coverage <= 1.0);
  assert_eq!(map.last_verified.as_deref(), Some("2026-02-01"));
}

#[test]
fn write_capability_map_to_file() {
  let schema_path = template_path("docs/spec/pnix-seto/templates/example_module_schema.px");
  let schema_src = fs::read_to_string(&schema_path).unwrap();
  let registry =
    load_layer_schema_pnix_with_base(&schema_src, schema_path.parent(), Some(&read_file)).unwrap();
  let package = registry.packages.get("math.rat").unwrap();
  let base_dir = schema_path.parent().unwrap();
  let laws = load_layer_laws_for_package_with(package, base_dir, &read_file).unwrap();
  let report_path = template_path("docs/spec/pnix-seto/templates/verification_report.px");
  let report_src = fs::read_to_string(&report_path).unwrap();
  let report = load_verification_report_pnix(&report_src).unwrap();

  let filename = format!(
    "capability_map_test_{}.px",
    std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .unwrap()
      .as_nanos()
  );
  let path = std::env::temp_dir().join(filename);
  write_capability_map_for_package_with(package, &laws, Some(&report), &path, &write_file).unwrap();

  let src = fs::read_to_string(&path).unwrap();
  parse_expr(&src).unwrap();
  assert!(src.contains("capability"));
  let _ = fs::remove_file(&path);
}

#[test]
fn write_capability_map_from_schema_path() {
  let schema_path = template_path("docs/spec/pnix-seto/templates/example_module_schema.px");
  let schema_src = fs::read_to_string(&schema_path).unwrap();
  let registry =
    load_layer_schema_pnix_with_base(&schema_src, schema_path.parent(), Some(&read_file)).unwrap();
  let package = registry.packages.get("math.rat").unwrap();
  let base_dir = schema_path.parent().unwrap();
  let laws = load_layer_laws_for_package_with(package, base_dir, &read_file).unwrap();
  let report = load_layer_verification_report_for_package_with(package, base_dir, &read_file)
    .unwrap()
    .unwrap();

  let filename = format!(
    "capability_map_schema_{}.px",
    std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .unwrap()
      .as_nanos()
  );
  let temp_path = std::env::temp_dir().join(filename);

  let mut overridden = package.clone();
  overridden.capability_map = Some(LayerFileSpec {
    kind: "pnix".to_string(),
    file: temp_path.to_string_lossy().to_string(),
  });
  let written = write_capability_map_for_package_configured_with(
    &overridden,
    base_dir,
    &laws,
    Some(&report),
    &write_file,
  )
  .unwrap()
  .unwrap();
  assert_eq!(written, temp_path);
  let src = fs::read_to_string(&written).unwrap();
  parse_expr(&src).unwrap();
  let _ = fs::remove_file(&written);
}

#[test]
fn schema_capability_map_path_is_resolvable() {
  let schema_path = template_path("docs/spec/pnix-seto/templates/example_module_schema.px");
  let schema_src = fs::read_to_string(&schema_path).unwrap();
  let registry =
    load_layer_schema_pnix_with_base(&schema_src, schema_path.parent(), Some(&read_file)).unwrap();
  let package = registry.packages.get("math.rat").unwrap();
  let base_dir = schema_path.parent().unwrap();
  let path = capability_map_path_for_package(package, base_dir)
    .unwrap()
    .unwrap();
  assert!(path.ends_with("capability_map.px"));
}

#[test]
fn update_capability_map_from_schema_writes_file() {
  let schema_path = template_path("docs/spec/pnix-seto/templates/example_module_schema.px");
  let schema_src = fs::read_to_string(&schema_path).unwrap();
  let registry =
    load_layer_schema_pnix_with_base(&schema_src, schema_path.parent(), Some(&read_file)).unwrap();
  let package = registry.packages.get("math.rat").unwrap();
  let base_dir = schema_path.parent().unwrap();
  let laws = load_layer_laws_for_package_with(package, base_dir, &read_file).unwrap();

  let filename = format!(
    "capability_map_update_{}.px",
    std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .unwrap()
      .as_nanos()
  );
  let temp_path = std::env::temp_dir().join(filename);
  let mut overridden = package.clone();
  overridden.capability_map = Some(LayerFileSpec {
    kind: "pnix".to_string(),
    file: temp_path.to_string_lossy().to_string(),
  });

  let written =
    update_capability_map_from_schema_with(&overridden, base_dir, &laws, &read_file, &write_file)
      .unwrap()
      .unwrap();
  assert_eq!(written, temp_path);
  let src = fs::read_to_string(&written).unwrap();
  parse_expr(&src).unwrap();
  let _ = fs::remove_file(&written);
}

#[test]
fn apply_verification_report_to_pipeline() {
  let schema_path = template_path("docs/spec/pnix-seto/templates/example_module_schema.px");
  let schema_src = fs::read_to_string(&schema_path).unwrap();
  let registry =
    load_layer_schema_pnix_with_base(&schema_src, schema_path.parent(), Some(&read_file)).unwrap();
  let package = registry.packages.get("math.rat").unwrap();
  let base_dir = schema_path.parent().unwrap();
  let laws = load_layer_laws_for_package_with(package, base_dir, &read_file).unwrap();

  let report_src = r#"
  {
    report = {
      overlay_id = "math.rat.v0";
      status = "failed";
      passed = [ "rat_add_comm" ];
      failed = [ "rat_mul_assoc" ];
      warnings = [ "numeric overflow on witness" ];
      generated_at = "2026-02-02";
    };
  }
  "#;
  let report = load_verification_report_pnix(report_src).unwrap();

  let engine = LayerEngine::new();
  let prepared = engine.prepare_with_laws_and_report(package, &laws, &report);
  assert!(
    prepared
      .report
      .errors
      .iter()
      .any(|err| err.contains("rat_mul_assoc")),
    "expected failed law to raise error"
  );
  assert!(
    prepared
      .report
      .warnings
      .iter()
      .any(|warn| warn.contains("verification warning")),
    "expected verification warning to be surfaced"
  );
}

#[test]
fn optional_failed_law_is_warning() {
  let schema_path = template_path("docs/spec/pnix-seto/templates/example_module_schema.px");
  let schema_src = fs::read_to_string(&schema_path).unwrap();
  let registry =
    load_layer_schema_pnix_with_base(&schema_src, schema_path.parent(), Some(&read_file)).unwrap();
  let package = registry.packages.get("math.rat").unwrap();
  let base_dir = schema_path.parent().unwrap();
  let laws = load_layer_laws_for_package_with(package, base_dir, &read_file).unwrap();

  let report_src = r#"
  {
    report = {
      overlay_id = "math.rat.v0";
      status = "failed";
      passed = [ ];
      failed = [ "assoc" ];
      warnings = [ ];
      generated_at = "2026-02-02";
    };
  }
  "#;
  let report = load_verification_report_pnix(report_src).unwrap();

  let engine = LayerEngine::new();
  let prepared = engine.prepare_with_laws_and_report(package, &laws, &report);
  assert!(
    prepared
      .report
      .errors
      .iter()
      .all(|err| !err.contains("assoc")),
    "optional law failure should not be an error"
  );
  assert!(
    prepared
      .report
      .warnings
      .iter()
      .any(|warn| warn.contains("assoc")),
    "optional law failure should be a warning"
  );
}
