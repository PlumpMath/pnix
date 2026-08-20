//! machines_heart.rs - Machine's Thinking Heart (Extended Structures)
//!
//! White Box AGI의 확장 구조 정의.
//! Evolution Loop, Rule System, CT Verification 등.
//!
//! ## 헌법 준수 (P0-1)
//!
//! - 타입/구조체 정의만 포함
//! - 실행 로직 (step, run, execute 등)은 executor로 이관
//! - 런타임 시간 의존성 제거 (wall-clock/monotonic clock API 등)
//!
//! ## 마이그레이션 출처
//!
//! `~/pnix-old/crates/meaning_core/src/machines_heart.rs`
//!
//! ## 기본 타입 참조
//!
//! `Goal`, `GoalType`, `Constraint`, `ConstraintType`, `SystemDiagnosticState`는
//! `crate::llm::intention`에 정의되어 있습니다.

use std::collections::HashMap;

// Re-export from llm module for convenience
pub use crate::llm::{Constraint, ConstraintType, Goal, GoalType, SystemDiagnosticState};

// ============================================================
// Intention Vector (확장)
// ============================================================

/// Intention Vector - 기계의 내부 의도 표현
///
/// NOTE: 기본 Goal 타입은 `crate::llm::Goal` 사용
#[derive(Debug, Clone)]
pub struct IntentionVector {
  /// Primary goal (주 목표)
  pub primary_goal: Goal,
  /// Sub-goals (하위 목표들)
  pub sub_goals: Vec<Goal>,
  /// Current diagnostic state (시스템 진단 상태)
  pub diagnostic_state: SystemDiagnosticState,
  /// Confidence level (확신도)
  pub confidence: f64,
  /// Priority weights (우선순위 가중치)
  pub priority_weights: HashMap<String, f64>,
}

impl IntentionVector {
  /// Create new intention vector
  pub fn new(primary_goal: Goal) -> Self {
    Self {
      primary_goal,
      sub_goals: Vec::new(),
      diagnostic_state: SystemDiagnosticState::default(),
      confidence: 1.0,
      priority_weights: HashMap::new(),
    }
  }

  /// Add sub-goal (builder pattern)
  pub fn with_sub_goal(mut self, goal: Goal) -> Self {
    self.sub_goals.push(goal);
    self
  }

  /// Set confidence level (builder pattern)
  pub fn with_confidence(mut self, confidence: f64) -> Self {
    self.confidence = confidence.clamp(0.0, 1.0);
    self
  }

  /// Set priority weight (builder pattern)
  pub fn with_priority(mut self, goal_id: impl Into<String>, weight: f64) -> Self {
    self.priority_weights.insert(goal_id.into(), weight);
    self
  }

  /// Set diagnostic state (builder pattern)
  pub fn with_diagnostic_state(mut self, state: SystemDiagnosticState) -> Self {
    self.diagnostic_state = state;
    self
  }
}

// ============================================================
// Evolution Loop Structures (Task 641)
// ============================================================

/// 진화 단계 기록: 진화 루프의 단일 단계 기록
#[derive(Debug, Clone)]
pub struct EvolutionStep {
  /// 단계 번호
  pub step: usize,
  /// 수행된 액션
  pub action: EvolutionAction,
  /// 액션 결과
  pub result: EvolutionResult,
  /// 타임스탬프 (밀리초, executor에서 설정)
  pub timestamp_ms: u64,
}

/// 진화 액션: 진화 루프에서 수행할 수 있는 액션 타입
#[derive(Debug, Clone, PartialEq)]
pub enum EvolutionAction {
  /// 현재 의도 해석
  InterpretIntention,
  /// 의도에서 코드 생성
  GenerateCode,
  /// 생성된 코드 실행
  Execute,
  /// CT 법칙 검증
  VerifyCT,
  /// 결과 기반 규칙 업데이트
  UpdateRules,
  /// 진화에 대한 반성
  Reflect,
}

/// 진화 결과: 진화 액션의 결과 타입
#[derive(Debug, Clone)]
pub enum EvolutionResult {
  /// 성공 (선택적 메시지 포함)
  Success(Option<String>),
  /// 실패 (에러 메시지 포함)
  Failure(String),
  /// 다음 단계로 계속
  Continue,
  /// 진화 완료
  Complete,
}

// ============================================================
// Memory State Structures
// ============================================================

/// 메모리 상태: 진화 루프를 위한 메모리 상태
#[derive(Debug, Clone, Default)]
pub struct MemoryState {
  /// 현재 메모리 항목들
  pub items: Vec<MemoryItem>,
  /// 최대 메모리 용량
  pub capacity: usize,
}

/// 단일 메모리 항목: 메모리에 저장되는 단일 항목
#[derive(Debug, Clone)]
pub struct MemoryItem {
  /// 항목 키
  pub key: String,
  /// 항목 값 (직렬화됨)
  pub value: String,
  /// 중요도 점수 (0.0 ~ 1.0)
  pub importance: f64,
  /// 접근 횟수
  pub access_count: usize,
}

// ============================================================
// Rule System Structures
// ============================================================

/// 규칙 집합: 진화를 위한 규칙 집합
#[derive(Debug, Clone, Default)]
pub struct RuleSet {
  /// 윤리 규칙들
  pub ethical_rules: Vec<EthicalRule>,
  /// 안전 규칙들
  pub safety_rules: Vec<SafetyRule>,
  /// 접두사 필터들 (lib/*.sam)
  pub prefix_filters: Vec<PrefixFilter>,
}

/// 윤리 규칙: 윤리적 제약 규칙
#[derive(Debug, Clone)]
pub struct EthicalRule {
  /// 규칙 식별자
  pub id: String,
  /// 사람이 읽을 수 있는 설명
  pub description: String,
  /// 우선순위 (높을수록 중요)
  pub priority: u32,
  /// 활성화 여부
  pub active: bool,
}

/// 안전 규칙: 안전 제약 규칙
#[derive(Debug, Clone)]
pub struct SafetyRule {
  /// 규칙 식별자
  pub id: String,
  /// 사람이 읽을 수 있는 설명
  pub description: String,
  /// 매칭할 패턴 (glob 또는 regex)
  pub pattern: String,
  /// 매칭 시 수행할 액션
  pub action: FilterAction,
}

/// 접두사 필터: lib/*.sam용 접두사 필터
#[derive(Debug, Clone)]
pub struct PrefixFilter {
  /// 필터 패턴 (glob)
  pub pattern: String,
  /// 수행할 액션
  pub action: FilterAction,
  /// 설명
  pub description: String,
}

/// 필터 액션: 필터 매칭 시 수행할 액션 타입
#[derive(Debug, Clone, PartialEq)]
pub enum FilterAction {
  /// 액션 허용
  Allow,
  /// 액션 거부
  Deny,
  /// 확인 요구
  Confirm,
  /// 로그하고 계속
  Log,
}

// ============================================================
// CT Verification Structures (Task 642)
// ============================================================

/// CT 검증 상태: 범주론 법칙 검증 상태
#[derive(Debug, Clone, Default)]
pub struct CTVerificationState {
  /// 검증할 법칙들
  pub laws_to_verify: Vec<CTLaw>,
  /// 검증된 법칙들
  pub verified_laws: Vec<CTLaw>,
  /// 실패한 법칙들
  pub failed_laws: Vec<CTLaw>,
  /// 검증 에러들
  pub errors: Vec<CTVerificationError>,
}

/// CT 검증 에러: 범주론 법칙 검증 에러
#[derive(Debug, Clone)]
pub struct CTVerificationError {
  /// 실패한 법칙
  pub law: CTLaw,
  /// 에러 메시지
  pub message: String,
  /// 컨텍스트 정보 (선택적)
  pub context: Option<String>,
}

/// Category Theory laws to verify
#[derive(Debug, Clone, PartialEq)]
pub enum CTLaw {
  /// Identity law: id . f = f = f . id
  Identity,
  /// Associativity: (f . g) . h = f . (g . h)
  Associativity,
  /// Functor preservation: F(f . g) = F(f) . F(g)
  FunctorComposition,
  /// Natural transformation: eta_B . F(f) = G(f) . eta_A
  NaturalTransformation,
  /// Monad left identity: return >>= f = f
  MonadLeftIdentity,
  /// Monad right identity: m >>= return = m
  MonadRightIdentity,
  /// Monad associativity: (m >>= f) >>= g = m >>= (x -> f x >>= g)
  MonadAssociativity,
}

// ============================================================
// Conscience Interface Structures (Task 647)
// ============================================================

/// 양심 인터페이스: 규칙 수정을 위한 양심 인터페이스
#[derive(Debug, Clone, Default)]
pub struct ConscienceInterface {
  /// 규칙 수정 이력
  pub modification_history: Vec<RuleModification>,
  /// 활성 규칙들
  pub active_rules: RuleSet,
}

/// 규칙 수정 기록: 규칙 수정 기록
#[derive(Debug, Clone)]
pub struct RuleModification {
  /// 수정 타입
  pub modification_type: ModificationType,
  /// 수정되는 규칙 ID
  pub rule_id: String,
  /// 수정을 요청한 주체 (선택적)
  pub modifier: Option<String>,
  /// 수정 이유 (선택적)
  pub reason: Option<String>,
  /// 타임스탬프 (밀리초, executor에서 설정)
  pub timestamp_ms: u64,
}

/// 수정 타입: 규칙 수정 타입
#[derive(Debug, Clone, PartialEq)]
pub enum ModificationType {
  /// 새 규칙 추가
  Add,
  /// 기존 규칙 제거
  Remove,
  /// 기존 규칙 업데이트
  Update,
  /// 비활성 규칙 활성화
  Enable,
  /// 활성 규칙 비활성화
  Disable,
}

// ============================================================
// Rule Evolution Structures
// ============================================================

/// 규칙 진화 제안: 규칙 진화 제안
#[derive(Debug, Clone)]
pub struct RuleEvolutionSuggestion {
  /// 제안 식별자
  pub id: String,
  /// 제안된 변경사항
  pub change: RuleChange,
  /// 제안에 대한 신뢰도 (0.0 ~ 1.0)
  pub confidence: f64,
  /// 이 제안을 지지하는 증거들
  pub evidence: Vec<EvolutionEvidence>,
  /// 제안 타입
  pub suggestion_type: SuggestionType,
}

/// 제안 타입: 규칙 진화 제안 타입
#[derive(Debug, Clone, PartialEq)]
pub enum SuggestionType {
  /// 기존 규칙 강화
  Strengthen,
  /// 기존 규칙 약화
  Weaken,
  /// 새 규칙 추가
  AddNew,
  /// 규칙 제거
  Remove,
  /// 여러 규칙 병합
  Merge,
}

/// 진화 증거: 진화 제안을 위한 증거
#[derive(Debug, Clone)]
pub struct EvolutionEvidence {
  /// 증거 타입
  pub evidence_type: String,
  /// 증거 설명
  pub description: String,
  /// 이 증거의 가중치
  pub weight: f64,
}

/// 규칙 변경 명세: 규칙 변경 명세 타입
#[derive(Debug, Clone)]
pub enum RuleChange {
  /// 윤리 규칙 추가
  AddEthicalRule(EthicalRule),
  /// 안전 규칙 추가
  AddSafetyRule(SafetyRule),
  /// 접두사 필터 추가
  AddPrefixFilter(PrefixFilter),
  /// ID로 규칙 제거
  RemoveRule(String),
  /// 규칙 우선순위 업데이트
  UpdatePriority {
    /// 규칙 ID
    rule_id: String,
    /// 새로운 우선순위
    new_priority: u32,
  },
  /// 규칙 패턴 업데이트
  UpdatePattern {
    /// 규칙 ID
    rule_id: String,
    /// 새로운 패턴
    new_pattern: String,
  },
}

/// 규칙 스냅샷: 버전 관리를 위한 규칙 스냅샷
#[derive(Debug, Clone)]
pub struct RuleSnapshot {
  /// 버전 번호
  pub version: u64,
  /// 규칙 스냅샷
  pub rules: RuleSet,
  /// 스냅샷 생성 이유
  pub reason: String,
  /// 타임스탬프 (밀리초, executor에서 설정)
  pub timestamp_ms: u64,
}

// ============================================================
// Evolution Loop State (Structure Only)
// ============================================================

/// 진화 루프 상태: 진화 루프 상태 (구조 정의만)
///
/// NOTE: 실행 로직 (step, run, execute 등)은 executor에서 구현
#[derive(Debug, Clone)]
pub struct EvolutionLoopState {
  /// 현재 의도
  pub intention: IntentionVector,
  /// 메모리 상태
  pub memory: MemoryState,
  /// 규칙 집합
  pub rules: RuleSet,
  /// CT 검증 상태
  pub ct_verification: CTVerificationState,
  /// 진화 이력
  pub history: Vec<EvolutionStep>,
  /// 현재 단계 번호
  pub current_step: usize,
  /// 진화 완료 여부
  pub is_complete: bool,
}

impl EvolutionLoopState {
  /// Create new evolution loop state
  pub fn new(intention: IntentionVector) -> Self {
    Self {
      intention,
      memory: MemoryState::default(),
      rules: RuleSet::default(),
      ct_verification: CTVerificationState::default(),
      history: Vec::new(),
      current_step: 0,
      is_complete: false,
    }
  }

  /// Set rules (builder pattern)
  pub fn with_rules(mut self, rules: RuleSet) -> Self {
    self.rules = rules;
    self
  }

  /// Set memory (builder pattern)
  pub fn with_memory(mut self, memory: MemoryState) -> Self {
    self.memory = memory;
    self
  }
}

/// 규칙 진화 엔진 상태: 규칙 진화 엔진 상태 (구조만)
///
/// NOTE: 실행 로직 (analyze, apply 등)은 executor에서 구현
#[derive(Debug, Clone, Default)]
pub struct RuleEvolutionEngineState {
  /// 활성 규칙들
  pub rules: RuleSet,
  /// 규칙 이력 (스냅샷들)
  pub snapshots: Vec<RuleSnapshot>,
  /// 대기 중인 제안들
  pub suggestions: Vec<RuleEvolutionSuggestion>,
  /// 현재 버전
  pub version: u64,
}

impl RuleEvolutionEngineState {
  /// Create new engine state
  pub fn new() -> Self {
    Self::default()
  }

  /// Set rules (builder pattern)
  pub fn with_rules(mut self, rules: RuleSet) -> Self {
    self.rules = rules;
    self
  }
}
