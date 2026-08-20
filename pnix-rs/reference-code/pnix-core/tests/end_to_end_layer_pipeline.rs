//! End-to-End Layer Pipeline 테스트: 레이어 파이프라인 전체 흐름 테스트
//!
//! 레이어 표현식 파싱부터 FxCore 변환까지의 전체 파이프라인을 테스트합니다.

use std::fs;
use std::path::{Path, PathBuf};

use pnix_core::effects::EffectZone;
use pnix_core::fx::core_expr::analyze_expr_zone;
use pnix_core::lang::layer::{
  load_layer_laws_for_package_with, load_layer_meaning_for_package_with,
  load_layer_schema_pnix_with_base, LayerEngine, LayerParseError,
};
use pnix_core::lang::pnix::{parse_layer_expr, pnix_expr_to_unified_with_layer_pipeline};
use pnix_core::runtime::{Process, ProcessId};

fn template_path(rel: &str) -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .join("..")
    .join("..")
    .join(rel)
}

fn read_file(path: &Path) -> Result<String, LayerParseError> {
  fs::read_to_string(path).map_err(|err| LayerParseError::Io {
    path: path.display().to_string(),
    message: err.to_string(),
  })
}

#[test]
fn end_to_end_layer_pipeline_smoke() {
  let schema_path = template_path("docs/spec/pnix-seto/templates/layer_schema_template.px");
  let schema_src = fs::read_to_string(&schema_path).unwrap();
  let registry =
    load_layer_schema_pnix_with_base(&schema_src, schema_path.parent(), Some(&read_file)).unwrap();
  let package = registry
    .packages
    .get("math.rat")
    .expect("missing math.rat package");
  let base_dir = schema_path.parent().unwrap();

  let laws = load_layer_laws_for_package_with(package, base_dir, &read_file).unwrap();
  let _meaning = load_layer_meaning_for_package_with(package, base_dir, &read_file).unwrap();

  let engine = LayerEngine::new();
  let prepared = engine.prepare_with_laws(package, Some(&laws));
  assert!(prepared.report.is_ok());
  let pipeline = engine.build_pipeline(&prepared);
  assert!(pipeline.report.is_ok());

  let expr = parse_layer_expr("Rat.make 1 2 / Rat.make 3 4", Some(&pipeline)).unwrap();
  let unified = pnix_expr_to_unified_with_layer_pipeline(&expr, Some(&pipeline)).unwrap();
  let fx = pnix_core::lang::pnix::lower_to_fx_core(&unified).unwrap();

  let zone = analyze_expr_zone(&fx);
  assert_eq!(zone, EffectZone::Pure);

  let process = Process::new(ProcessId(0), zone, vec![], None);
  assert!(process.is_alive());
}
