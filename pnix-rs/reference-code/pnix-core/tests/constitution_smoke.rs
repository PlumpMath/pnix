//! Constitution 스모크 테스트: MeaningOp → Zone → IR 매핑 스모크 테스트
//!
//! 모든 MeaningOp이 올바른 Zone과 IR 심볼을 가지는지 검증합니다.

use pnix_core::effects::EffectZone;
use pnix_core::fx::meaning_op::MeaningOpId;
use pnix_core::fx::op_table::UnifiedMeaningOp;

#[test]
fn constitution_smoke_meaning_to_zone_to_ir() {
  for op in UnifiedMeaningOp::all() {
    let meaning = MeaningOpId::from(*op);
    let symbol = meaning.ir_symbol();
    assert!(!symbol.is_empty(), "missing ir symbol for {:?}", op);
    let zone = meaning.zone();
    let zone_label = format!("{:?}", zone);
    assert!(!zone_label.is_empty(), "missing zone for {:?}", op);
  }
}

#[test]
fn constitution_smoke_no_zone_regression() {
  let critical_ops = [
    (MeaningOpId::Add, EffectZone::Pure),
    (MeaningOpId::InteropCall, EffectZone::Interop),
    (MeaningOpId::AtomNew, EffectZone::Stm),
    (MeaningOpId::Print, EffectZone::World),
  ];
  for (op, expected) in critical_ops {
    assert_eq!(op.zone(), expected, "Zone regression for {:?}", op);
  }
}
