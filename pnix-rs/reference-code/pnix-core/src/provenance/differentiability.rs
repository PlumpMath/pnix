//! Differentiability Analysis - Expression differentiability tracking
//!
//! Tracks which operations prevent automatic differentiation

use serde::{Deserialize, Serialize};

/// 미분 불가능 이유: 자동 미분을 방해하는 연산의 이유 타입
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DifferentiabilityReason {
  /// 불연속 연산 (floor, ceil, mod)
  DiscontinuousOp(
    /// 연산 이름
    String,
  ),
  /// 비교 연산 (0 또는 1 반환, 미분 불가)
  ComparisonOp(
    /// 연산 이름
    String,
  ),
  /// 논리 연산
  LogicalOp(
    /// 연산 이름
    String,
  ),
  /// Select/조건문 (브랜치)
  Branching,
  /// 상수 (미분 = 0, 기술적으로는 가능)
  Constant,
  /// 문자열/AttrSet (비수치)
  NonNumeric(
    /// 타입 이름
    String,
  ),
  /// 미분 불가능 하위 표현식 포함
  ContainsNonDiff,
}

impl DifferentiabilityReason {
  /// 불연속 연산 여부 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn is_discontinuous(&self) -> bool {
    matches!(self, Self::DiscontinuousOp(_))
  }

  /// 비교 연산 여부 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn is_comparison(&self) -> bool {
    matches!(self, Self::ComparisonOp(_))
  }

  /// 브랜칭 여부 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn is_branching(&self) -> bool {
    matches!(self, Self::Branching)
  }
}

/// 미분 불가능 연산 기록: 미분 불가능한 연산의 위치와 이유를 기록하는 구조
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NonDifferentiableOp {
  /// 연산 위치/이름
  pub location: String,
  /// 불가능 이유
  pub reason: DifferentiabilityReason,
}

impl NonDifferentiableOp {
  /// 새 미분 불가능 연산 기록 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(location: impl Into<String>, reason: DifferentiabilityReason) -> Self {
    Self {
      location: location.into(),
      reason,
    }
  }

  /// 불연속 연산용 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn discontinuous(location: impl Into<String>, op: impl Into<String>) -> Self {
    Self::new(
      location,
      DifferentiabilityReason::DiscontinuousOp(op.into()),
    )
  }

  /// 비교 연산용 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn comparison(location: impl Into<String>, op: impl Into<String>) -> Self {
    Self::new(location, DifferentiabilityReason::ComparisonOp(op.into()))
  }

  /// 논리 연산용 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn logical(location: impl Into<String>, op: impl Into<String>) -> Self {
    Self::new(location, DifferentiabilityReason::LogicalOp(op.into()))
  }

  /// 브랜칭용 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn branching(location: impl Into<String>) -> Self {
    Self::new(location, DifferentiabilityReason::Branching)
  }
}

/// 미분 가능성 분석 결과: 표현식의 미분 가능성 분석 결과
#[derive(Clone, Debug, Default)]
pub struct DifferentiabilityAnalysis {
  /// 전체 표현식 미분 가능 여부
  pub is_differentiable: bool,
  /// 미분 불가능 지점들
  pub issues: Vec<NonDifferentiableOp>,
}

impl DifferentiabilityAnalysis {
  /// 새 분석 결과 생성 (미분 가능)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn ok() -> Self {
    Self {
      is_differentiable: true,
      issues: Vec::new(),
    }
  }

  /// 미분 불가능으로 표시
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn fail(location: impl Into<String>, reason: DifferentiabilityReason) -> Self {
    Self {
      is_differentiable: false,
      issues: vec![NonDifferentiableOp::new(location, reason)],
    }
  }

  /// 두 분석 결과 결합
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 병합만, 값 계산 없음
  pub fn merge(mut self, other: Self) -> Self {
    self.is_differentiable = self.is_differentiable && other.is_differentiable;
    self.issues.extend(other.issues);
    self
  }

  /// 문제 추가
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn add_issue(&mut self, location: impl Into<String>, reason: DifferentiabilityReason) {
    self.is_differentiable = false;
    self.issues.push(NonDifferentiableOp::new(location, reason));
  }

  /// 문제 존재 여부 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn has_issues(&self) -> bool {
    !self.issues.is_empty()
  }

  /// 문제 개수 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn issue_count(&self) -> usize {
    self.issues.len()
  }

  /// 특정 이유 타입의 문제들 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 필터링만, 값 계산 없음
  pub fn issues_by_reason(
    &self,
    check: impl Fn(&DifferentiabilityReason) -> bool,
  ) -> Vec<&NonDifferentiableOp> {
    self.issues.iter().filter(|op| check(&op.reason)).collect()
  }
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_differentiability_reason() {
    let disc = DifferentiabilityReason::DiscontinuousOp("floor".into());
    assert!(disc.is_discontinuous());
    assert!(!disc.is_comparison());

    let comp = DifferentiabilityReason::ComparisonOp("<".into());
    assert!(comp.is_comparison());
    assert!(!comp.is_discontinuous());

    let branch = DifferentiabilityReason::Branching;
    assert!(branch.is_branching());
  }

  #[test]
  fn test_non_differentiable_op_constructors() {
    let disc = NonDifferentiableOp::discontinuous("root/floor", "floor");
    assert!(disc.reason.is_discontinuous());

    let comp = NonDifferentiableOp::comparison("root/lt", "<");
    assert!(comp.reason.is_comparison());

    let branch = NonDifferentiableOp::branching("root/select");
    assert!(branch.reason.is_branching());
  }

  #[test]
  fn test_analysis_ok() {
    let analysis = DifferentiabilityAnalysis::ok();
    assert!(analysis.is_differentiable);
    assert!(analysis.issues.is_empty());
    assert!(!analysis.has_issues());
  }

  #[test]
  fn test_analysis_fail() {
    let analysis = DifferentiabilityAnalysis::fail(
      "root",
      DifferentiabilityReason::DiscontinuousOp("floor".into()),
    );
    assert!(!analysis.is_differentiable);
    assert_eq!(analysis.issue_count(), 1);
    assert!(analysis.has_issues());
  }

  #[test]
  fn test_analysis_merge() {
    let a = DifferentiabilityAnalysis::ok();
    let b = DifferentiabilityAnalysis::fail("branch", DifferentiabilityReason::Branching);

    let merged = a.merge(b);
    assert!(!merged.is_differentiable);
    assert_eq!(merged.issue_count(), 1);
  }

  #[test]
  fn test_analysis_add_issue() {
    let mut analysis = DifferentiabilityAnalysis::ok();
    analysis.add_issue(
      "root/floor",
      DifferentiabilityReason::DiscontinuousOp("floor".into()),
    );

    assert!(!analysis.is_differentiable);
    assert_eq!(analysis.issue_count(), 1);
  }

  #[test]
  fn test_issues_by_reason() {
    let mut analysis = DifferentiabilityAnalysis::ok();
    analysis.add_issue(
      "a",
      DifferentiabilityReason::DiscontinuousOp("floor".into()),
    );
    analysis.add_issue("b", DifferentiabilityReason::ComparisonOp("<".into()));
    analysis.add_issue("c", DifferentiabilityReason::DiscontinuousOp("ceil".into()));

    let disc = analysis.issues_by_reason(|r| r.is_discontinuous());
    assert_eq!(disc.len(), 2);

    let comp = analysis.issues_by_reason(|r| r.is_comparison());
    assert_eq!(comp.len(), 1);
  }

  #[test]
  fn test_serde() {
    let reason = DifferentiabilityReason::DiscontinuousOp("floor".into());
    let json = serde_json::to_string(&reason).unwrap();
    let restored: DifferentiabilityReason = serde_json::from_str(&json).unwrap();
    assert_eq!(reason, restored);
  }
}
