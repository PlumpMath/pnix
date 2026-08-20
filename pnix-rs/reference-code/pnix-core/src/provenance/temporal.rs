//! Temporal Decision - Time variable promotion tracking
//!
//! Zone-aware time variable promotion decisions

use crate::effects::EffectZone;
use serde::{Deserialize, Serialize};

/// 시간 변수 승격 결정: Zone-aware 변환 추적을 위한 시간 변수 승격 결정
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TemporalDecision {
  /// 일반 변수로 유지 (승격 없음)
  KeptAsVar,
  /// TimeParam으로 승격됨
  PromotedToTimeParam,
  /// DeltaTime으로 승격됨
  PromotedToDeltaTime,
  /// Zone 제한으로 승격 거부됨
  RejectedByZone {
    /// 변수 이름
    var_name: String,
    /// 효과 영역
    zone: EffectZone,
    /// 거부 이유
    reason: String,
  },
}

impl TemporalDecision {
  /// TimeParam 승격 거부 (Zone 제한)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn rejected_time(var_name: impl Into<String>, zone: EffectZone) -> Self {
    Self::RejectedByZone {
      var_name: var_name.into(),
      zone,
      reason: "TimeParam only allowed in Frp/Animation zones".to_string(),
    }
  }

  /// DeltaTime 승격 거부 (Zone 제한)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn rejected_delta(var_name: impl Into<String>, zone: EffectZone) -> Self {
    Self::RejectedByZone {
      var_name: var_name.into(),
      zone,
      reason: "DeltaTime only allowed in Animation zone".to_string(),
    }
  }

  /// 거부 여부 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn is_rejected(&self) -> bool {
    matches!(self, Self::RejectedByZone { .. })
  }

  /// 승격 여부 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn is_promoted(&self) -> bool {
    matches!(self, Self::PromotedToTimeParam | Self::PromotedToDeltaTime)
  }
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_temporal_decision_variants() {
    let kept = TemporalDecision::KeptAsVar;
    assert!(!kept.is_promoted());
    assert!(!kept.is_rejected());

    let promoted = TemporalDecision::PromotedToTimeParam;
    assert!(promoted.is_promoted());
    assert!(!promoted.is_rejected());

    let rejected = TemporalDecision::rejected_time("t", EffectZone::Pure);
    assert!(!rejected.is_promoted());
    assert!(rejected.is_rejected());
  }

  #[test]
  fn test_rejected_time() {
    let decision = TemporalDecision::rejected_time("t", EffectZone::Pure);

    if let TemporalDecision::RejectedByZone {
      var_name,
      zone,
      reason,
    } = decision
    {
      assert_eq!(var_name, "t");
      assert_eq!(zone, EffectZone::Pure);
      assert!(reason.contains("TimeParam"));
    } else {
      panic!("Expected RejectedByZone");
    }
  }

  #[test]
  fn test_rejected_delta() {
    let decision = TemporalDecision::rejected_delta("dt", EffectZone::Frp);

    if let TemporalDecision::RejectedByZone {
      var_name,
      zone,
      reason,
    } = decision
    {
      assert_eq!(var_name, "dt");
      assert_eq!(zone, EffectZone::Frp);
      assert!(reason.contains("DeltaTime"));
    } else {
      panic!("Expected RejectedByZone");
    }
  }

  #[test]
  fn test_serde() {
    let decision = TemporalDecision::PromotedToTimeParam;
    let json = serde_json::to_string(&decision).unwrap();
    let restored: TemporalDecision = serde_json::from_str(&json).unwrap();
    assert_eq!(decision, restored);
  }
}
