//! IR 스냅샷 회귀 테스트
//!
//! Lowering 결과의 결정성을 검증하기 위한 스냅샷 테스트

use pnix_core::lang::pnix::{lower_to_fx_core, parse_expr, pnix_expr_to_unified};

#[test]
fn test_lowering_snapshot_int() {
  let source = "42";
  let expr = parse_expr(source).unwrap();
  let unified = pnix_expr_to_unified(&expr).unwrap();
  let fx_core = lower_to_fx_core(&unified).unwrap();

  // 스냅샷 비교 (향후 insta 크레이트 사용 가능)
  let snapshot = format!("{:?}", fx_core);
  assert!(!snapshot.is_empty(), "Lowering 결과가 비어있지 않아야 함");
}

#[test]
fn test_lowering_snapshot_arithmetic() {
  let source = "1 + 2 * 3";
  let expr = parse_expr(source).unwrap();
  let unified = pnix_expr_to_unified(&expr).unwrap();
  let fx_core = lower_to_fx_core(&unified).unwrap();

  let snapshot = format!("{:?}", fx_core);
  assert!(!snapshot.is_empty(), "Lowering 결과가 비어있지 않아야 함");
}

#[test]
fn test_lowering_snapshot_let() {
  let source = "let x = 1; y = 2; in x + y";
  let expr = parse_expr(source).unwrap();
  let unified = pnix_expr_to_unified(&expr).unwrap();
  let fx_core = lower_to_fx_core(&unified).unwrap();

  let snapshot = format!("{:?}", fx_core);
  assert!(!snapshot.is_empty(), "Lowering 결과가 비어있지 않아야 함");
}

#[test]
fn test_lowering_snapshot_attrset() {
  let source = "{ a = 1; b = 2; }";
  let expr = parse_expr(source).unwrap();
  let unified = pnix_expr_to_unified(&expr).unwrap();
  let fx_core = lower_to_fx_core(&unified).unwrap();

  let snapshot = format!("{:?}", fx_core);
  assert!(!snapshot.is_empty(), "Lowering 결과가 비어있지 않아야 함");
}

#[test]
fn test_lowering_determinism() {
  // 동일 입력에 대해 여러 번 lowering한 결과가 동일한지 확인
  let source = "let x = 1; y = 2; in { inherit x y; z = x + y; }";

  let expr1 = parse_expr(source).unwrap();
  let unified1 = pnix_expr_to_unified(&expr1).unwrap();
  let fx_core1 = lower_to_fx_core(&unified1).unwrap();

  let expr2 = parse_expr(source).unwrap();
  let unified2 = pnix_expr_to_unified(&expr2).unwrap();
  let fx_core2 = lower_to_fx_core(&unified2).unwrap();

  // Debug 포맷으로 비교 (향후 더 정교한 비교 가능)
  assert_eq!(
    format!("{:?}", fx_core1),
    format!("{:?}", fx_core2),
    "동일 입력에 대해 lowering 결과가 동일해야 함"
  );
}
