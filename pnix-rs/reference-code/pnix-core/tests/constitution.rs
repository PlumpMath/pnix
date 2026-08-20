//! Constitution 테스트: MeaningOp 파이프라인 및 회귀 테스트
//!
//! MeaningOp의 zone 및 IR 심볼 매핑이 올바른지 검증합니다.

use pnix_core::effects::EffectZone;
use pnix_core::fx::{MeaningOpId, UnifiedMeaningOp};

#[test]
fn meaning_op_pipeline_smoke() {
  for op in UnifiedMeaningOp::all() {
    let op = MeaningOpId::from(*op);
    let _zone = op.zone();
    let symbol = op.ir_symbol();
    assert!(!symbol.trim().is_empty());
  }
}

#[test]
fn meaning_op_regression_samples() {
  assert_eq!(MeaningOpId::Add.zone(), EffectZone::Pure);
  assert_eq!(MeaningOpId::InteropCall.zone(), EffectZone::Interop);
}
