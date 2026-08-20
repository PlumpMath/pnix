//! 레이어 상호작용 VM AE 테스트: VM에서의 레이어 상호작용 및 AE 테스트
//!
//! VM에서 레이어 상호작용 및 AE(Adverse Effect) 처리가 올바르게 동작하는지 검증합니다.

use std::fs;
use std::path::{Path, PathBuf};

use pnix_core::effects::EffectZone;
use pnix_core::fx::meaning_op::MeaningOpId;
use pnix_core::fx::op_table::UnifiedMeaningOp;
use pnix_core::lang::layer::{
  load_layer_laws_for_package_with, load_layer_meaning_for_package_with,
  load_layer_schema_pnix_with_base, LayerParseError,
};
use pnix_core::lang::pnix::PnixExpr;

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
fn verification_laws_and_meaning_docs_load() {
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
  let meaning = load_layer_meaning_for_package_with(package, base_dir, &read_file).unwrap();

  assert!(laws.contains("rat_add_assoc"));
  assert!(matches!(meaning, PnixExpr::AttrSet { .. }));
}

#[test]
fn meaning_ops_map_to_action_ops() {
  let meaning = MeaningOpId::Add;
  let unified = UnifiedMeaningOp::from(&meaning);
  assert_eq!(MeaningOpId::from(unified), meaning);
  assert!(!unified.ir_symbol().is_empty());
}

#[test]
fn action_ops_map_to_effect_zones() {
  assert_eq!(UnifiedMeaningOp::Add.zone(), EffectZone::Pure);
  assert_eq!(UnifiedMeaningOp::Print.zone(), EffectZone::World);
}
