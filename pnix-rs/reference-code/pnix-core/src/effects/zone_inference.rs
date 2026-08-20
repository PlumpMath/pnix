//! Zone Inference - Effect zone 추론 및 검증
//!
//! pnix-old의 meaning_core/src/unified_meaning/zone_inference.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의 및 순수 분석 함수만, 실행 로직 제외
//! - ZoneContext: 변수 zone 추적 구조
//! - ZoneResult: zone 추론 결과 구조
//! - ZoneViolation: zone 위반 구조
//! - infer_op_zone, check_op_in_zone: 순수 분석 함수 (값 계산 없음)
//! - 실제 zone 추론 실행은 executor에서 구현

use crate::effects::EffectZone;
use crate::fx::op_table::UnifiedMeaningOp;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================
// Zone Context
// ============================================================

/// Zone 추론 컨텍스트 - 변수 zone 추적
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneContext {
  /// 변수 이름 → 추론된 zone
  var_zones: HashMap<String, EffectZone>,
  /// 현재 스코프의 zone (중첩 표현식용)
  scope_zone: EffectZone,
}

impl ZoneContext {
  /// 새로운 zone 컨텍스트 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new() -> Self {
    Self {
      var_zones: HashMap::new(),
      scope_zone: EffectZone::Pure,
    }
  }

  /// 변수를 zone에 바인딩
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변경만, 값 계산 없음
  pub fn bind(&mut self, name: &str, zone: EffectZone) {
    self.var_zones.insert(name.to_string(), zone);
  }

  /// 변수의 zone 조회
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn lookup(&self, name: &str) -> Option<EffectZone> {
    self.var_zones.get(name).copied()
  }

  /// 현재 스코프 zone 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn scope_zone(&self) -> EffectZone {
    self.scope_zone
  }

  /// 새로운 스코프 진입 (zone과 함께)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변경만, 값 계산 없음
  pub fn enter_scope(&mut self, zone: EffectZone) {
    self.scope_zone = self.scope_zone.join(zone);
  }

  /// 중첩 스코프를 위한 자식 컨텍스트 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 복사만, 값 계산 없음
  pub fn child(&self) -> Self {
    Self {
      var_zones: self.var_zones.clone(),
      scope_zone: self.scope_zone,
    }
  }
}

impl Default for ZoneContext {
  fn default() -> Self {
    Self::new()
  }
}

// ============================================================
// Zone Result
// ============================================================

/// 표현식의 zone 추론 결과
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneResult {
  /// 이 표현식에 대해 추론된 zone
  pub zone: EffectZone,
  /// 발견된 zone 위반들
  pub violations: Vec<ZoneViolation>,
}

impl ZoneResult {
  /// Pure zone 결과 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn pure() -> Self {
    Self {
      zone: EffectZone::Pure,
      violations: vec![],
    }
  }

  /// 특정 zone으로 결과 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn with_zone(zone: EffectZone) -> Self {
    Self {
      zone,
      violations: vec![],
    }
  }

  /// 두 결과를 join (격자 상위 레벨 선택)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 병합만, 값 계산 없음
  pub fn join(&self, other: &ZoneResult) -> ZoneResult {
    let mut violations = self.violations.clone();
    violations.extend(other.violations.iter().cloned());
    ZoneResult {
      zone: self.zone.join(other.zone),
      violations,
    }
  }
}

// ============================================================
// Zone Violation
// ============================================================

/// Zone 위반 (effectful 연산이 pure 컨텍스트에서 사용됨)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneViolation {
  /// 위반한 연산
  pub op: UnifiedMeaningOp,
  /// 연산이 요구하는 zone
  pub op_zone: EffectZone,
  /// 컨텍스트 zone
  pub context_zone: EffectZone,
  /// 위반 메시지
  pub message: String,
}

impl ZoneViolation {
  /// 새로운 zone 위반 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(
    op: UnifiedMeaningOp,
    op_zone: EffectZone,
    context_zone: EffectZone,
    message: String,
  ) -> Self {
    Self {
      op,
      op_zone,
      context_zone,
      message,
    }
  }
}

// ============================================================
// Zone Inference Functions (순수 분석 함수)
// ============================================================

/// 단일 연산의 zone 추론 (순수 함수)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 접근만, 값 계산 없음
pub fn infer_op_zone(op: UnifiedMeaningOp) -> EffectZone {
  op.zone()
}

/// 연산이 주어진 컨텍스트 zone에서 유효한지 검사 (순수 함수)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 비교만, 값 계산 없음
pub fn check_op_in_zone(
  op: UnifiedMeaningOp,
  context_zone: EffectZone,
) -> Result<(), ZoneViolation> {
  let op_zone = op.zone();
  if op_zone.is_subzone_of(context_zone) {
    Ok(())
  } else {
    Err(ZoneViolation::new(
      op,
      op_zone,
      context_zone,
      format!(
        "Operation {:?} requires {:?} zone but context is {:?}",
        op, op_zone, context_zone
      ),
    ))
  }
}

/// 연산 시퀀스의 zone 추론 (순수 함수)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 분석만, 값 계산 없음
pub fn infer_sequence_zone(ops: &[UnifiedMeaningOp]) -> EffectZone {
  ops
    .iter()
    .map(|op| op.zone())
    .fold(EffectZone::Pure, EffectZone::join)
}

// 헌법 준수 (P0-1): 실행 로직 제거
// 다음 함수들은 executor/runtime 계층에서 구현하세요:
// - ZoneInfer trait 구현 (AST 순회 및 zone 추론 실행)
// - FxCoreExpr::infer_zone() 구현 (실제 AST 순회)
// - SimpleExpr::infer_zone() 구현 (테스트용 표현식 순회)
//
// 이 함수들은 AST 순회 및 실행을 수행하므로 pnix-core에서 제외됩니다.

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_zone_context() {
    let mut ctx = ZoneContext::new();
    ctx.bind("x", EffectZone::Frp);
    assert_eq!(ctx.lookup("x"), Some(EffectZone::Frp));
    assert_eq!(ctx.lookup("y"), None);
  }

  #[test]
  fn test_zone_result_join() {
    let r1 = ZoneResult::with_zone(EffectZone::Pure);
    let r2 = ZoneResult::with_zone(EffectZone::Frp);
    let joined = r1.join(&r2);
    assert_eq!(joined.zone, EffectZone::Frp);
  }

  #[test]
  fn test_infer_sequence_zone() {
    use crate::fx::op_table::UnifiedMeaningOp;
    let ops = vec![UnifiedMeaningOp::Add, UnifiedMeaningOp::Sin];
    let zone = infer_sequence_zone(&ops);
    // Add와 Sin 모두 Pure zone이므로 Pure가 반환되어야 함
    assert_eq!(zone, EffectZone::Pure);
  }
}
