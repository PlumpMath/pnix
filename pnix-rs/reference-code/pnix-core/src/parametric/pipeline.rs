//! Parametric 파이프라인: 합성 및 출력을 통합한 파이프라인

use super::emit::{
  emit_synthesis_result_pnix, emit_synthesis_result_surface, emit_synthesis_result_unified,
};
use super::error::ParametricResult;
use super::ir::ParametricSpec;
use super::synth::synthesize;
use crate::fx::surface::FxSurfaceExpr;
use crate::lang::pnix::UnifiedExpr;

/// 파라미터 합성 → UnifiedExpr: 제약 조건으로부터 UnifiedExpr 생성
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn synthesize_to_unified(spec: &ParametricSpec) -> ParametricResult<UnifiedExpr> {
  let res = synthesize(spec)?;
  emit_synthesis_result_unified(&res)
}

/// 파라미터 합성 → FxSurfaceExpr: 제약 조건으로부터 FxSurfaceExpr 생성
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn synthesize_to_surface(spec: &ParametricSpec) -> ParametricResult<FxSurfaceExpr> {
  let res = synthesize(spec)?;
  emit_synthesis_result_surface(&res)
}

/// 파라미터 합성 → Pnix 문자열: 제약 조건으로부터 Pnix DSL 문자열 생성
///
/// ## 헌법 준수 (P0-1, C1)
///
/// 텍스트 생성만, 파일 I/O 없음
pub fn synthesize_to_pnix(spec: &ParametricSpec) -> ParametricResult<String> {
  let res = synthesize(spec)?;
  emit_synthesis_result_pnix(&res)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::lang::pnix::parser::parse_expr;
  use crate::parametric::ir::{
    Constraint, ConstraintExpr, ContextMode, ParamExpr, ParamRole, ParamVar, TargetVar,
  };

  #[test]
  fn pipeline_emits_parsable_pnix() {
    let spec = ParametricSpec {
      context: ContextMode::Pure,
      params: vec![ParamVar::new("x", ParamRole::Output)],
      signals: vec![],
      unit_scales: vec![],
      fixtures: vec![],
      constraints: vec![Constraint {
        id: "c1".to_string(),
        expr: ConstraintExpr::Eq {
          left: ParamExpr::var("x"),
          right: ParamExpr::int(3),
        },
        provenance: None,
      }],
      target: TargetVar::new("x"),
    };

    let src = synthesize_to_pnix(&spec).unwrap();
    let _ = parse_expr(&src).unwrap();
  }
}
