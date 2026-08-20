//! Budget Tier - Adaptive simplification budget
//!
//! Expression complexity estimation and tiered budget allocation

use serde::{Deserialize, Serialize};

/// egg 단순화 예산 티어: 표현식 복잡도에 따른 적응형 단순화 예산 티어
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BudgetTier {
  /// 최소 예산 (작은 표현식)
  Light,
  /// 중간 예산 (일반 표현식)
  #[default]
  Medium,
  /// 최대 예산 (복잡한 표현식)
  Heavy,
}

impl BudgetTier {
  /// 티어별 기본 iteration 예산
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn default_iterations(&self) -> usize {
    match self {
      BudgetTier::Light => 5,
      BudgetTier::Medium => 15,
      BudgetTier::Heavy => 30,
    }
  }

  /// 한 단계 약화
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변환만, 값 계산 없음
  pub fn downgrade(&self) -> Self {
    match self {
      BudgetTier::Heavy => BudgetTier::Medium,
      BudgetTier::Medium => BudgetTier::Light,
      BudgetTier::Light => BudgetTier::Light,
    }
  }

  /// 비용 점수로부터 적절한 티어 선택
  ///
  /// # Cost Thresholds
  /// - Light: cost < 50 (간단한 산술 표현식)
  /// - Medium: 50 <= cost < 200 (중간 복잡도)
  /// - Heavy: cost >= 200 (복잡한 중첩 표현식)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변환만, 값 계산 없음
  pub fn from_cost(cost: u32) -> Self {
    if cost < 50 {
      BudgetTier::Light
    } else if cost < 200 {
      BudgetTier::Medium
    } else {
      BudgetTier::Heavy
    }
  }

  /// 이 티어가 최소 티어인지 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn is_minimum(&self) -> bool {
    matches!(self, BudgetTier::Light)
  }

  /// 이 티어가 최대 티어인지 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn is_maximum(&self) -> bool {
    matches!(self, BudgetTier::Heavy)
  }
}

/// 적응형 Simplify 결과: 적응형 단순화 과정의 결과
#[derive(Clone, Debug)]
pub struct AdaptiveSimplifyResult {
  /// 최종 사용된 예산 티어
  pub final_tier: BudgetTier,
  /// 초기 추정 비용
  pub initial_cost: u32,
  /// 다운그레이드 횟수
  pub downgrades: u32,
  /// 타임아웃 발생 여부
  pub had_timeout: bool,
}

impl AdaptiveSimplifyResult {
  /// 새 결과 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(initial_cost: u32, tier: BudgetTier) -> Self {
    Self {
      final_tier: tier,
      initial_cost,
      downgrades: 0,
      had_timeout: false,
    }
  }

  /// 다운그레이드 기록
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변경만, 값 계산 없음
  pub fn record_downgrade(&mut self) {
    self.downgrades += 1;
    self.final_tier = self.final_tier.downgrade();
    self.had_timeout = true;
  }

  /// 성공적으로 완료되었는지 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn is_successful(&self) -> bool {
    !self.had_timeout
  }

  /// 최소 티어까지 다운그레이드되었는지 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn reached_minimum(&self) -> bool {
    self.final_tier.is_minimum()
  }
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_budget_tier_iterations() {
    assert_eq!(BudgetTier::Light.default_iterations(), 5);
    assert_eq!(BudgetTier::Medium.default_iterations(), 15);
    assert_eq!(BudgetTier::Heavy.default_iterations(), 30);
  }

  #[test]
  fn test_budget_tier_downgrade() {
    assert_eq!(BudgetTier::Heavy.downgrade(), BudgetTier::Medium);
    assert_eq!(BudgetTier::Medium.downgrade(), BudgetTier::Light);
    assert_eq!(BudgetTier::Light.downgrade(), BudgetTier::Light);
  }

  #[test]
  fn test_budget_tier_from_cost() {
    assert_eq!(BudgetTier::from_cost(10), BudgetTier::Light);
    assert_eq!(BudgetTier::from_cost(49), BudgetTier::Light);
    assert_eq!(BudgetTier::from_cost(50), BudgetTier::Medium);
    assert_eq!(BudgetTier::from_cost(199), BudgetTier::Medium);
    assert_eq!(BudgetTier::from_cost(200), BudgetTier::Heavy);
    assert_eq!(BudgetTier::from_cost(1000), BudgetTier::Heavy);
  }

  #[test]
  fn test_budget_tier_is_minimum() {
    assert!(BudgetTier::Light.is_minimum());
    assert!(!BudgetTier::Medium.is_minimum());
    assert!(!BudgetTier::Heavy.is_minimum());
  }

  #[test]
  fn test_budget_tier_is_maximum() {
    assert!(!BudgetTier::Light.is_maximum());
    assert!(!BudgetTier::Medium.is_maximum());
    assert!(BudgetTier::Heavy.is_maximum());
  }

  #[test]
  fn test_adaptive_result_new() {
    let result = AdaptiveSimplifyResult::new(30, BudgetTier::Light);
    assert_eq!(result.final_tier, BudgetTier::Light);
    assert_eq!(result.initial_cost, 30);
    assert_eq!(result.downgrades, 0);
    assert!(!result.had_timeout);
    assert!(result.is_successful());
  }

  #[test]
  fn test_adaptive_result_downgrade() {
    let mut result = AdaptiveSimplifyResult::new(250, BudgetTier::Heavy);

    result.record_downgrade();
    assert_eq!(result.final_tier, BudgetTier::Medium);
    assert_eq!(result.downgrades, 1);
    assert!(result.had_timeout);
    assert!(!result.is_successful());

    result.record_downgrade();
    assert_eq!(result.final_tier, BudgetTier::Light);
    assert_eq!(result.downgrades, 2);
    assert!(result.reached_minimum());
  }

  #[test]
  fn test_serde() {
    let tier = BudgetTier::Medium;
    let json = serde_json::to_string(&tier).unwrap();
    let restored: BudgetTier = serde_json::from_str(&json).unwrap();
    assert_eq!(tier, restored);
  }
}
