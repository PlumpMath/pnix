//! 의도/목표 시스템 타입 정의
//!
//! pnix-old의 meaning_core/src/machines_heart.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 순수 구조 타입만, 값 계산/상태 변경 없음.
//!
//! ## 사용 목적
//!
//! - White Box AGI의 의도 표현
//! - 시스템 목표/제약 정의
//! - 진단용 상태 표현
//!
//! ## 구현 위치
//!
//! IntentionVector의 상태 업데이트/실행 로직은 core 밖(executor/별도 crate)에서 담당합니다.

use serde::{Deserialize, Serialize};

/// 목표 유형
///
/// 기계의 목표가 어떤 종류인지 분류합니다.
///
/// # 예시
/// ```ignore
/// use pnix_core::llm::GoalType;
///
/// let goal_type = GoalType::Achieve;
/// assert_eq!(goal_type, GoalType::Achieve);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoalType {
  /// 특정 상태 달성 (종료형)
  Achieve,
  /// 조건 유지 (지속형)
  Maintain,
  /// 특정 상태 회피
  Avoid,
  /// 정보 질의/학습
  Query,
  /// 행동 수행
  Perform,
}

/// 제약 유형
///
/// 목표 실행 시 준수해야 하는 제약의 종류를 분류합니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstraintType {
  /// 안전 제약 (절대 위반 불가)
  Safety,
  /// 자원 제약 (메모리, 시간 등)
  Resource,
  /// 윤리 제약 (lib/*.sam 규칙에서 유래)
  Ethical,
  /// 논리 제약 (수학적 일관성)
  Logical,
}

/// 제약 정의
///
/// 목표 실행 시 준수해야 하는 개별 제약을 정의합니다.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Constraint {
  /// 제약 이름
  pub name: String,
  /// 제약 유형
  pub constraint_type: ConstraintType,
  /// 제약 충족 여부 (정적 분석 결과)
  pub satisfied: bool,
}

impl Constraint {
  /// 새 제약 생성
  pub fn new(name: impl Into<String>, constraint_type: ConstraintType) -> Self {
    Self {
      name: name.into(),
      constraint_type,
      satisfied: false,
    }
  }

  /// 안전 제약 생성
  pub fn safety(name: impl Into<String>) -> Self {
    Self::new(name, ConstraintType::Safety)
  }

  /// 자원 제약 생성
  pub fn resource(name: impl Into<String>) -> Self {
    Self::new(name, ConstraintType::Resource)
  }

  /// 윤리 제약 생성
  pub fn ethical(name: impl Into<String>) -> Self {
    Self::new(name, ConstraintType::Ethical)
  }

  /// 논리 제약 생성
  pub fn logical(name: impl Into<String>) -> Self {
    Self::new(name, ConstraintType::Logical)
  }
}

/// 목표 정의: 기계의 개별 목표를 정의하는 구조
///
/// 기계의 개별 목표를 정의합니다.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Goal {
  /// 목표 식별자 (목표의 고유 ID)
  pub id: String,
  /// 사람이 읽을 수 있는 설명 (목표의 설명)
  pub description: String,
  /// 목표 유형 (목표의 종류)
  pub goal_type: GoalType,
  /// 완료 상태 (0.0 = 미시작, 1.0 = 완료, 진행률)
  pub progress: f64,
  /// 준수해야 하는 제약 목록 (목표 실행 시 준수할 제약들)
  pub constraints: Vec<Constraint>,
}

impl Goal {
  /// 새 목표 생성
  pub fn new(id: impl Into<String>, description: impl Into<String>, goal_type: GoalType) -> Self {
    Self {
      id: id.into(),
      description: description.into(),
      goal_type,
      progress: 0.0,
      constraints: Vec::new(),
    }
  }

  /// 달성 목표 생성
  pub fn achieve(id: impl Into<String>, description: impl Into<String>) -> Self {
    Self::new(id, description, GoalType::Achieve)
  }

  /// 유지 목표 생성
  pub fn maintain(id: impl Into<String>, description: impl Into<String>) -> Self {
    Self::new(id, description, GoalType::Maintain)
  }

  /// 회피 목표 생성
  pub fn avoid(id: impl Into<String>, description: impl Into<String>) -> Self {
    Self::new(id, description, GoalType::Avoid)
  }

  /// 질의 목표 생성
  pub fn query(id: impl Into<String>, description: impl Into<String>) -> Self {
    Self::new(id, description, GoalType::Query)
  }

  /// 수행 목표 생성
  pub fn perform(id: impl Into<String>, description: impl Into<String>) -> Self {
    Self::new(id, description, GoalType::Perform)
  }

  /// 제약 추가
  pub fn with_constraint(mut self, constraint: Constraint) -> Self {
    self.constraints.push(constraint);
    self
  }
}

/// 시스템 진단 상태: 시스템 자가 진단용 상태 표현 (인간 감정이 아님)
///
/// 시스템 자가 진단용 상태 표현 (인간 감정이 아님).
///
/// ## 구현 위치
///
/// 상태 업데이트 로직은 core 밖(executor/별도 crate)에서 담당합니다.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemDiagnosticState {
  /// 탐색 동기 (0.0 ~ 1.0, 새로운 정보 탐색 의지)
  pub curiosity: f64,
  /// 위험 회피 수준 (0.0 ~ 1.0, 위험한 행동 회피 정도)
  pub caution: f64,
  /// 목표 진행 만족도 (0.0 ~ 1.0, 목표 달성 진행에 대한 만족도)
  pub satisfaction: f64,
  /// 반복 실패로 인한 좌절감 (0.0 ~ 1.0, 반복 실패로 인한 좌절 수준)
  pub frustration: f64,
}

impl Default for SystemDiagnosticState {
  fn default() -> Self {
    Self {
      curiosity: 0.5,
      caution: 0.5,
      satisfaction: 0.0,
      frustration: 0.0,
    }
  }
}

// **주의**: 값 계산 함수(update, clamp)는 P0-1 위반이므로 제거되었습니다.
// IntentionVector, 런타임 상태 관리, 목표 진행률 업데이트 로직은
// core 밖(executor/별도 crate)에서 담당합니다.

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_goal_type_serde() {
    let goal_type = GoalType::Achieve;
    let json = serde_json::to_string(&goal_type).unwrap();
    assert!(json.contains("achieve"));
  }

  #[test]
  fn test_constraint_type_serde() {
    let constraint_type = ConstraintType::Safety;
    let json = serde_json::to_string(&constraint_type).unwrap();
    assert!(json.contains("safety"));
  }

  #[test]
  fn test_goal_builder() {
    let goal = Goal::achieve("goal1", "Test goal").with_constraint(Constraint::safety("no_harm"));

    assert_eq!(goal.id, "goal1");
    assert_eq!(goal.goal_type, GoalType::Achieve);
    assert_eq!(goal.constraints.len(), 1);
    assert_eq!(goal.constraints[0].constraint_type, ConstraintType::Safety);
  }

  #[test]
  fn test_system_diagnostic_state_default() {
    let state = SystemDiagnosticState::default();
    assert_eq!(state.curiosity, 0.5);
    assert_eq!(state.caution, 0.5);
    assert_eq!(state.satisfaction, 0.0);
    assert_eq!(state.frustration, 0.0);
  }
}
