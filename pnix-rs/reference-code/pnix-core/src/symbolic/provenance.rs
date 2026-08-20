//! Symbolic Provenance - 심볼릭 변환 증명 로그 시스템
//!
//! pnix-old의 meaning_core/src/unified_meaning/symbolic_provenance.rs에서 마이그레이션.
//!
//! "왜 이렇게 변환되었는지"를 추적하여 디버깅/교육/CI 검증에 활용.
//!
//! # 헌법 준수 (P0-1)
//!
//! - 구조 정의만, 값 계산/상태 변경 없음
//! - estimate_ir_cost: AST 분석 (값 계산 아님) ✅
//! - analyze_differentiability: AST 순회 분석 ✅
//!
//! # Feature Flag
//!
//! `symbolic-provenance` feature가 활성화되면:
//! - `SymbolicProvenance` 구조체가 완전히 활성화
//! - `IrExpr`에 `provenance` 필드 추가 가능

use serde::{Deserialize, Serialize};

use crate::effects::EffectZone;
use crate::ir::IrExpr;

// ═══════════════════════════════════════════════════════════════
// 1. Temporal Decision (Zone-aware 변환 추적용)
// ═══════════════════════════════════════════════════════════════

/// 시간 변수 승격 결정: 시간 변수가 TimeParam/DeltaTime으로 승격되는지 결정
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
  pub fn rejected_time(var_name: impl Into<String>, zone: EffectZone) -> Self {
    Self::RejectedByZone {
      var_name: var_name.into(),
      zone,
      reason: "TimeParam only allowed in Frp/Animation zones".to_string(),
    }
  }

  pub fn rejected_delta(var_name: impl Into<String>, zone: EffectZone) -> Self {
    Self::RejectedByZone {
      var_name: var_name.into(),
      zone,
      reason: "DeltaTime only allowed in Animation zone".to_string(),
    }
  }
}

// ═══════════════════════════════════════════════════════════════
// 2. CT Validation Result
// ═══════════════════════════════════════════════════════════════

/// 범주론 검증 결과: CT 법칙 검증 결과
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CtValidationResult {
  /// 검증 성공
  #[default]
  Ok,
  /// 검증 실패
  Failed(
    /// 실패 이유
    String,
  ),
  /// 검증 건너뜀
  Skipped,
}

// ═══════════════════════════════════════════════════════════════
// 3. Budget Tier (Adaptive Simplify용)
// ═══════════════════════════════════════════════════════════════

/// egg 단순화 예산 티어: 적응형 단순화를 위한 예산 레벨
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BudgetTier {
  /// 경량 티어 (낮은 비용, 빠른 처리)
  Light,
  /// 중간 티어 (중간 비용, 균형 처리)
  Medium,
  /// 중량 티어 (높은 비용, 정밀 처리)
  Heavy,
}

impl BudgetTier {
  /// 티어별 기본 iteration 예산
  pub fn default_iterations(&self) -> usize {
    match self {
      BudgetTier::Light => 5,
      BudgetTier::Medium => 15,
      BudgetTier::Heavy => 30,
    }
  }

  /// 한 단계 약화
  pub fn downgrade(&self) -> Self {
    match self {
      BudgetTier::Heavy => BudgetTier::Medium,
      BudgetTier::Medium => BudgetTier::Light,
      BudgetTier::Light => BudgetTier::Light,
    }
  }

  /// 비용 점수로부터 적절한 티어 선택
  pub fn from_cost(cost: u32) -> Self {
    if cost < 50 {
      BudgetTier::Light
    } else if cost < 200 {
      BudgetTier::Medium
    } else {
      BudgetTier::Heavy
    }
  }

  pub fn is_minimum(&self) -> bool {
    matches!(self, BudgetTier::Light)
  }
}

// ═══════════════════════════════════════════════════════════════
// 3-A. Cost Estimation
// ═══════════════════════════════════════════════════════════════

/// IrExpr 비용 추정 (적응형 Simplify용)
///
/// ## 헌법 준수 (P0-1)
///
/// AST 구조 분석만, 값 계산 없음 ✅
pub fn estimate_ir_cost(expr: &IrExpr) -> u32 {
  match expr {
    IrExpr::ConstFloat(_)
    | IrExpr::ConstInt(_)
    | IrExpr::ConstBool(_)
    | IrExpr::ConstString(_)
    | IrExpr::VarRef(_)
    | IrExpr::SignalRef(_)
    | IrExpr::TimeParam
    | IrExpr::DeltaTime => 1,

    IrExpr::Add(a, b) | IrExpr::Sub(a, b) | IrExpr::Mul(a, b) | IrExpr::Mod(a, b) => {
      3 + estimate_ir_cost(a) + estimate_ir_cost(b)
    }

    IrExpr::Div(a, b) => 4 + estimate_ir_cost(a) + estimate_ir_cost(b),

    IrExpr::Lt(a, b)
    | IrExpr::Gt(a, b)
    | IrExpr::Le(a, b)
    | IrExpr::Ge(a, b)
    | IrExpr::Eq(a, b)
    | IrExpr::Ne(a, b) => 3 + estimate_ir_cost(a) + estimate_ir_cost(b),

    IrExpr::And(a, b) | IrExpr::Or(a, b) => 2 + estimate_ir_cost(a) + estimate_ir_cost(b),

    IrExpr::Neg(a) | IrExpr::Not(a) => 2 + estimate_ir_cost(a),

    IrExpr::Floor(a) | IrExpr::Ceil(a) | IrExpr::Abs(a) => 3 + estimate_ir_cost(a),

    IrExpr::Sqrt(a) => 5 + estimate_ir_cost(a),
    IrExpr::Sin(a) | IrExpr::Cos(a) | IrExpr::Tan(a) => 6 + estimate_ir_cost(a),
    IrExpr::Exp(a) | IrExpr::Log(a) => 7 + estimate_ir_cost(a),

    IrExpr::Pow(a, b) => 8 + estimate_ir_cost(a) + estimate_ir_cost(b),

    IrExpr::Select(cond, then_br, else_br) => {
      4 + estimate_ir_cost(cond) + estimate_ir_cost(then_br) + estimate_ir_cost(else_br)
    }

    IrExpr::List(items) | IrExpr::Tuple(items) => {
      2 + items.iter().map(estimate_ir_cost).sum::<u32>()
    }

    IrExpr::AttrSet(pairs) => {
      3 + pairs
        .iter()
        .map(|(_, v)| 1 + estimate_ir_cost(v))
        .sum::<u32>()
    }

    IrExpr::Lambda { body, .. } => 5 + estimate_ir_cost(body),

    IrExpr::Apply { func, arg } => 4 + estimate_ir_cost(func) + estimate_ir_cost(arg),

    IrExpr::Let { bindings, body } => {
      5 + bindings
        .iter()
        .map(|(_, v)| 2 + estimate_ir_cost(v))
        .sum::<u32>()
        + estimate_ir_cost(body)
    }

    // 문자열 연산, 리스트 연산 등 기타 (중간 비용)
    _ => 4,
  }
}

/// 적응형 예산 티어 선택
pub fn select_adaptive_tier(expr: &IrExpr) -> BudgetTier {
  BudgetTier::from_cost(estimate_ir_cost(expr))
}

/// 적응형 단순화 결과: 적응형 예산 티어를 사용한 단순화 결과
#[derive(Clone, Debug)]
pub struct AdaptiveSimplifyResult {
  /// 최종 예산 티어
  pub final_tier: BudgetTier,
  /// 초기 비용 추정값
  pub initial_cost: u32,
  /// 예산 티어 다운그레이드 횟수
  pub downgrades: u32,
  /// 타임아웃 발생 여부
  pub had_timeout: bool,
}

impl AdaptiveSimplifyResult {
  pub fn new(initial_cost: u32, tier: BudgetTier) -> Self {
    Self {
      final_tier: tier,
      initial_cost,
      downgrades: 0,
      had_timeout: false,
    }
  }

  pub fn record_downgrade(&mut self) {
    self.downgrades += 1;
    self.final_tier = self.final_tier.downgrade();
    self.had_timeout = true;
  }
}

// ═══════════════════════════════════════════════════════════════
// 4. Simplify Stats (egg 통계)
// ═══════════════════════════════════════════════════════════════

/// 단순화 통계: egg 단순화 통계 정보
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SimplifyStats {
  /// 단순화 반복 횟수
  pub iterations: usize,
  /// 생성된 eclass 개수
  pub eclasses_created: usize,
  /// 사용된 노드 개수
  pub nodes_used: usize,
  /// 타임아웃 발생 여부
  pub timed_out: bool,
}

// ═══════════════════════════════════════════════════════════════
// 5. Approx/Cache 정보
// ═══════════════════════════════════════════════════════════════

/// 근사 발생 지점: 근사가 발생한 위치와 이유
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApproxPoint {
  /// 근사 발생 위치 (파일:라인:컬럼 또는 노드 ID)
  pub location: String,
  /// 근사 발생 이유
  pub reason: String,
}

/// FRP 캐시 후보 정보: 캐시 가능한 서브표현식 정보
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CachedSubexprInfo {
  /// 캐시 키 (표현식 해시)
  pub key: u64,
  /// 표현식 크기 (AST 노드 수)
  pub size: u32,
  /// 표현식 문자열 (디버깅/프로베넌스용)
  pub pretty: String,
}

/// FRP 캐시 런타임 통계 기록: FRP 캐시 성능 통계
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FrpCacheStatsRecord {
  /// 캐시 히트 횟수
  pub hits: u64,
  /// 캐시 미스 횟수
  pub misses: u64,
  /// 캐시 히트율 (0.0 ~ 1.0)
  pub hit_rate: f64,
}

// ═══════════════════════════════════════════════════════════════
// 6. Differentiability (미분 가능성 분석)
// ═══════════════════════════════════════════════════════════════

/// 미분 불가능 이유: 미분 불가능한 연산의 이유 타입
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DifferentiabilityReason {
  /// 불연속 연산 (예: floor, ceil)
  DiscontinuousOp(
    /// 연산 이름
    String,
  ),
  /// 비교 연산 (예: <, >, ==)
  ComparisonOp(
    /// 연산 이름
    String,
  ),
  /// 논리 연산 (예: &&, ||)
  LogicalOp(
    /// 연산 이름
    String,
  ),
  /// 분기 (if-then-else)
  Branching,
  /// 상수 (미분하면 0이지만 분석 목적상 기록)
  Constant,
  /// 비수치 타입 (문자열 등)
  NonNumeric(
    /// 타입 이름
    String,
  ),
  /// 미분 불가능 표현식 포함 (재귀적)
  ContainsNonDiff,
}

/// 미분 불가능 연산 기록
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NonDifferentiableOp {
  /// 발생 위치
  pub location: String,
  /// 미분 불가능 이유
  pub reason: DifferentiabilityReason,
}

/// 미분 가능성 분석 결과: 표현식의 미분 가능성 분석 결과
#[derive(Clone, Debug)]
pub struct DifferentiabilityAnalysis {
  /// 미분 가능 여부
  pub is_differentiable: bool,
  /// 미분 불가능 이슈 목록 (미분 불가능한 연산들)
  pub issues: Vec<NonDifferentiableOp>,
}

impl DifferentiabilityAnalysis {
  pub fn ok() -> Self {
    Self {
      is_differentiable: true,
      issues: Vec::new(),
    }
  }

  pub fn fail(location: impl Into<String>, reason: DifferentiabilityReason) -> Self {
    Self {
      is_differentiable: false,
      issues: vec![NonDifferentiableOp {
        location: location.into(),
        reason,
      }],
    }
  }

  pub fn merge(mut self, other: Self) -> Self {
    self.is_differentiable = self.is_differentiable && other.is_differentiable;
    self.issues.extend(other.issues);
    self
  }

  pub fn add_issue(&mut self, location: impl Into<String>, reason: DifferentiabilityReason) {
    self.is_differentiable = false;
    self.issues.push(NonDifferentiableOp {
      location: location.into(),
      reason,
    });
  }
}

/// IrExpr 미분 가능성 분석
///
/// ## 헌법 준수 (P0-1)
///
/// AST 순회 분석만, 값 계산 없음 ✅
pub fn analyze_differentiability(expr: &IrExpr) -> DifferentiabilityAnalysis {
  analyze_differentiability_impl(expr, "root")
}

fn analyze_differentiability_impl(expr: &IrExpr, path: &str) -> DifferentiabilityAnalysis {
  match expr {
    // 미분 가능: 상수 및 변수
    IrExpr::ConstFloat(_) | IrExpr::ConstInt(_) => DifferentiabilityAnalysis::ok(),
    IrExpr::VarRef(_) | IrExpr::SignalRef(_) => DifferentiabilityAnalysis::ok(),
    IrExpr::TimeParam | IrExpr::DeltaTime => DifferentiabilityAnalysis::ok(),

    // 미분 가능: 산술 연산
    IrExpr::Add(a, b) => {
      let a_res = analyze_differentiability_impl(a, &format!("{}/add.left", path));
      let b_res = analyze_differentiability_impl(b, &format!("{}/add.right", path));
      a_res.merge(b_res)
    }
    IrExpr::Sub(a, b) => {
      let a_res = analyze_differentiability_impl(a, &format!("{}/sub.left", path));
      let b_res = analyze_differentiability_impl(b, &format!("{}/sub.right", path));
      a_res.merge(b_res)
    }
    IrExpr::Mul(a, b) => {
      let a_res = analyze_differentiability_impl(a, &format!("{}/mul.left", path));
      let b_res = analyze_differentiability_impl(b, &format!("{}/mul.right", path));
      a_res.merge(b_res)
    }
    IrExpr::Div(a, b) => {
      let a_res = analyze_differentiability_impl(a, &format!("{}/div.left", path));
      let b_res = analyze_differentiability_impl(b, &format!("{}/div.right", path));
      a_res.merge(b_res)
    }
    IrExpr::Neg(a) => analyze_differentiability_impl(a, &format!("{}/neg", path)),

    // 미분 가능: 수학 함수
    IrExpr::Sin(a) => analyze_differentiability_impl(a, &format!("{}/sin", path)),
    IrExpr::Cos(a) => analyze_differentiability_impl(a, &format!("{}/cos", path)),
    IrExpr::Tan(a) => analyze_differentiability_impl(a, &format!("{}/tan", path)),
    IrExpr::Exp(a) => analyze_differentiability_impl(a, &format!("{}/exp", path)),
    IrExpr::Log(a) => analyze_differentiability_impl(a, &format!("{}/log", path)),
    IrExpr::Sqrt(a) => analyze_differentiability_impl(a, &format!("{}/sqrt", path)),
    IrExpr::Pow(a, b) => {
      let a_res = analyze_differentiability_impl(a, &format!("{}/pow.base", path));
      let b_res = analyze_differentiability_impl(b, &format!("{}/pow.exp", path));
      a_res.merge(b_res)
    }
    IrExpr::Abs(a) => analyze_differentiability_impl(a, &format!("{}/abs", path)),

    // 미분 불가능: 불연속 연산
    IrExpr::Floor(_) => DifferentiabilityAnalysis::fail(
      path,
      DifferentiabilityReason::DiscontinuousOp("floor".to_string()),
    ),
    IrExpr::Ceil(_) => DifferentiabilityAnalysis::fail(
      path,
      DifferentiabilityReason::DiscontinuousOp("ceil".to_string()),
    ),
    IrExpr::Mod(_, _) => DifferentiabilityAnalysis::fail(
      path,
      DifferentiabilityReason::DiscontinuousOp("mod".to_string()),
    ),

    // 미분 불가능: 비교 연산
    IrExpr::Lt(_, _) => {
      DifferentiabilityAnalysis::fail(path, DifferentiabilityReason::ComparisonOp("<".to_string()))
    }
    IrExpr::Gt(_, _) => {
      DifferentiabilityAnalysis::fail(path, DifferentiabilityReason::ComparisonOp(">".to_string()))
    }
    IrExpr::Le(_, _) => DifferentiabilityAnalysis::fail(
      path,
      DifferentiabilityReason::ComparisonOp("<=".to_string()),
    ),
    IrExpr::Ge(_, _) => DifferentiabilityAnalysis::fail(
      path,
      DifferentiabilityReason::ComparisonOp(">=".to_string()),
    ),
    IrExpr::Eq(_, _) => DifferentiabilityAnalysis::fail(
      path,
      DifferentiabilityReason::ComparisonOp("==".to_string()),
    ),
    IrExpr::Ne(_, _) => DifferentiabilityAnalysis::fail(
      path,
      DifferentiabilityReason::ComparisonOp("!=".to_string()),
    ),

    // 미분 불가능: 논리 연산
    IrExpr::And(_, _) => {
      DifferentiabilityAnalysis::fail(path, DifferentiabilityReason::LogicalOp("and".to_string()))
    }
    IrExpr::Or(_, _) => {
      DifferentiabilityAnalysis::fail(path, DifferentiabilityReason::LogicalOp("or".to_string()))
    }
    IrExpr::Not(_) => {
      DifferentiabilityAnalysis::fail(path, DifferentiabilityReason::LogicalOp("not".to_string()))
    }

    // 미분 불가능: 조건문
    IrExpr::Select(_, _, _) => {
      DifferentiabilityAnalysis::fail(path, DifferentiabilityReason::Branching)
    }

    // 미분 불가능: 비수치 타입
    IrExpr::ConstBool(_) => DifferentiabilityAnalysis::fail(
      path,
      DifferentiabilityReason::NonNumeric("bool".to_string()),
    ),
    IrExpr::ConstString(_) => DifferentiabilityAnalysis::fail(
      path,
      DifferentiabilityReason::NonNumeric("string".to_string()),
    ),
    IrExpr::AttrSet(_) => DifferentiabilityAnalysis::fail(
      path,
      DifferentiabilityReason::NonNumeric("attrset".to_string()),
    ),

    // 컬렉션: 요소별 분석
    IrExpr::List(items) | IrExpr::Tuple(items) => {
      let mut result = DifferentiabilityAnalysis::ok();
      for (i, item) in items.iter().enumerate() {
        result = result.merge(analyze_differentiability_impl(
          item,
          &format!("{}/[{}]", path, i),
        ));
      }
      result
    }

    // Lambda/Apply/Let: 본문 분석
    IrExpr::Lambda { body, .. } => {
      analyze_differentiability_impl(body, &format!("{}/lambda.body", path))
    }
    IrExpr::Apply { func, arg } => {
      let f_res = analyze_differentiability_impl(func, &format!("{}/apply.func", path));
      let a_res = analyze_differentiability_impl(arg, &format!("{}/apply.arg", path));
      f_res.merge(a_res)
    }
    IrExpr::Let { bindings, body } => {
      let mut result = DifferentiabilityAnalysis::ok();
      for (name, expr) in bindings {
        result = result.merge(analyze_differentiability_impl(
          expr,
          &format!("{}/let.{}", path, name),
        ));
      }
      result.merge(analyze_differentiability_impl(
        body,
        &format!("{}/let.body", path),
      ))
    }

    // 문자열/리스트 연산 등 기타: 미분 불가능 (비수치)
    _ => DifferentiabilityAnalysis::fail(
      path,
      DifferentiabilityReason::NonNumeric("other".to_string()),
    ),
  }
}

// ═══════════════════════════════════════════════════════════════
// 7. Symbolic Provenance (메인 구조체)
// ═══════════════════════════════════════════════════════════════

/// 심볼릭 변환 증명 로그: 심볼릭 변환 과정의 추적 정보
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SymbolicProvenance {
  /// 적용된 규칙 목록 (어떤 rewrite 규칙이 적용되었는지)
  pub applied_rules: Vec<String>,
  /// 원본 표현식 해시 (변환 전 표현식의 해시)
  pub original_hash: u64,
  /// 결과 표현식 해시 (변환 후 표현식의 해시)
  pub result_hash: u64,
  /// 근사 여부 (정확도 손실이 발생했는지)
  pub is_approximate: bool,
  /// 근사 발생 지점 목록 (근사가 발생한 위치와 이유)
  pub approx_points: Vec<ApproxPoint>,
  /// 효과 영역 (선택적, 변환 시 효과 영역)
  pub zone: Option<EffectZone>,
  /// 시간 변수 승격 결정 목록 (시간 변수가 TimeParam/DeltaTime으로 승격되었는지)
  pub temporal_decisions: Vec<TemporalDecision>,
  /// 단순화 통계 (egg 단순화 통계)
  pub stats: SimplifyStats,
  /// 예산 티어 (선택적, 적응형 단순화에서 사용한 예산 티어)
  pub budget_tier: Option<BudgetTier>,
  /// CT 검증 결과 (범주론 법칙 검증 결과)
  pub ct_validation: CtValidationResult,
  /// 원본 표현식 문자열 (선택적, 디버깅용)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub original_expr_pretty: Option<String>,
  /// 결과 표현식 문자열 (선택적, 디버깅용)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub result_expr_pretty: Option<String>,
  /// FRP 전체 캐시 가능 여부 (전체 표현식이 시간 독립적인지)
  pub frp_whole_cacheable: bool,
  /// FRP 캐시된 하위 표현식 목록 (시간 독립적인 서브트리들)
  pub frp_cached_subexprs: Vec<CachedSubexprInfo>,
  /// FRP 캐시 통계 (선택적, 캐시 성능 통계)
  pub frp_cache_stats: Option<FrpCacheStatsRecord>,
  /// 미분 가능 여부 (표현식이 미분 가능한지)
  pub differentiable: bool,
  /// 미분 불가능 연산 목록 (미분 불가능한 연산들)
  pub non_differentiable_ops: Vec<NonDifferentiableOp>,
}

impl SymbolicProvenance {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn skip(reason: impl Into<String>) -> Self {
    let mut prov = Self::new();
    prov.applied_rules.push(reason.into());
    prov
  }

  // 규칙 기록
  pub fn record_rule(&mut self, rule: impl Into<String>) {
    self.applied_rules.push(rule.into());
  }

  pub fn record_rules<S: Into<String>>(&mut self, rules: impl IntoIterator<Item = S>) {
    self
      .applied_rules
      .extend(rules.into_iter().map(|r| r.into()));
  }

  // Precision 기록
  pub fn record_approx(&mut self, location: impl Into<String>, reason: impl Into<String>) {
    self.is_approximate = true;
    self.approx_points.push(ApproxPoint {
      location: location.into(),
      reason: reason.into(),
    });
  }

  pub fn record_large_exponent(&mut self, exp: i64, limit: i64) {
    self.record_approx(
      "Pow",
      format!("large exponent {} exceeds limit {}", exp, limit),
    );
  }

  pub fn record_non_integer_exponent(&mut self, exp: f64) {
    self.record_approx("Pow", format!("non-integer exponent {}", exp));
  }

  // Temporal 기록
  pub fn set_zone(&mut self, zone: EffectZone) {
    self.zone = Some(zone);
  }

  pub fn record_temporal(&mut self, decision: TemporalDecision) {
    self.temporal_decisions.push(decision);
  }

  pub fn record_time_promotion(&mut self) {
    self
      .temporal_decisions
      .push(TemporalDecision::PromotedToTimeParam);
  }

  pub fn record_delta_promotion(&mut self) {
    self
      .temporal_decisions
      .push(TemporalDecision::PromotedToDeltaTime);
  }

  // egg 통계
  pub fn set_stats(&mut self, stats: SimplifyStats) {
    self.stats = stats;
  }

  pub fn record_timeout(&mut self, iterations: usize) {
    self.stats.timed_out = true;
    self.stats.iterations = iterations;
    self.record_approx("egg", format!("timeout after {} iterations", iterations));
  }

  // 해시
  pub fn set_original_hash(&mut self, hash: u64) {
    self.original_hash = hash;
  }

  pub fn set_result_hash(&mut self, hash: u64) {
    self.result_hash = hash;
  }

  // FRP Cache
  pub fn record_frp_cache(&mut self, whole_cacheable: bool, candidates: Vec<CachedSubexprInfo>) {
    self.frp_whole_cacheable = whole_cacheable;
    self.frp_cached_subexprs = candidates;
  }

  pub fn record_frp_whole_cacheable(&mut self) {
    self.frp_whole_cacheable = true;
  }

  pub fn add_cached_subexpr(&mut self, key: u64, size: u32, pretty: impl Into<String>) {
    self.frp_cached_subexprs.push(CachedSubexprInfo {
      key,
      size,
      pretty: pretty.into(),
    });
  }

  pub fn record_frp_cache_stats(&mut self, hits: u64, misses: u64) {
    let total = hits + misses;
    let hit_rate = if total == 0 {
      0.0
    } else {
      hits as f64 / total as f64
    };
    self.frp_cache_stats = Some(FrpCacheStatsRecord {
      hits,
      misses,
      hit_rate,
    });
  }

  // Differentiability
  pub fn record_differentiability(&mut self, analysis: &DifferentiabilityAnalysis) {
    self.differentiable = analysis.is_differentiable;
    self.non_differentiable_ops = analysis.issues.clone();
  }

  pub fn set_differentiable(&mut self, value: bool) {
    self.differentiable = value;
  }

  pub fn add_non_differentiable(
    &mut self,
    location: impl Into<String>,
    reason: DifferentiabilityReason,
  ) {
    self.differentiable = false;
    self.non_differentiable_ops.push(NonDifferentiableOp {
      location: location.into(),
      reason,
    });
  }

  // 요약/출력
  pub fn summary(&self) -> String {
    let mut s = String::new();
    s.push_str(&format!("rules: {} applied\n", self.applied_rules.len()));
    s.push_str(&format!("approximate: {}\n", self.is_approximate));
    if let Some(zone) = self.zone {
      s.push_str(&format!("zone: {:?}\n", zone));
    }
    s.push_str(&format!("iterations: {}\n", self.stats.iterations));
    if self.stats.timed_out {
      s.push_str("timed out\n");
    }
    s
  }

  pub fn to_report(&self) -> String {
    let mut s = String::new();

    s.push_str("===========================================\n");
    s.push_str("        Symbolic Provenance Report         \n");
    s.push_str("===========================================\n\n");

    s.push_str(&format!("Original hash: 0x{:016x}\n", self.original_hash));
    s.push_str(&format!("Result hash:   0x{:016x}\n", self.result_hash));
    s.push('\n');

    if let Some(zone) = self.zone {
      s.push_str(&format!("Zone: {:?}\n", zone));
    }

    s.push_str(&format!("Exact: {}\n", !self.is_approximate));
    if !self.approx_points.is_empty() {
      s.push_str("Approximations:\n");
      for ap in &self.approx_points {
        s.push_str(&format!("  - [{}] {}\n", ap.location, ap.reason));
      }
    }
    s.push('\n');

    s.push_str(&format!("Applied rules ({}):\n", self.applied_rules.len()));
    for (i, rule) in self.applied_rules.iter().enumerate() {
      s.push_str(&format!("  {}. {}\n", i + 1, humanize_rule(rule.as_str())));
    }
    s.push('\n');

    if !self.temporal_decisions.is_empty() {
      s.push_str("Temporal decisions:\n");
      for td in &self.temporal_decisions {
        s.push_str(&format!("  - {:?}\n", td));
      }
      s.push('\n');
    }

    s.push_str("egg statistics:\n");
    s.push_str(&format!("  iterations: {}\n", self.stats.iterations));
    s.push_str(&format!("  e-classes: {}\n", self.stats.eclasses_created));
    s.push_str(&format!("  nodes: {}\n", self.stats.nodes_used));
    if self.stats.timed_out {
      s.push_str("  TIMED OUT\n");
    }

    s.push_str(&format!("\nCT validation: {:?}\n", self.ct_validation));

    if self.zone == Some(EffectZone::Frp) || self.zone == Some(EffectZone::Animation) {
      s.push_str("\nFRP Cache:\n");
      if self.frp_whole_cacheable {
        s.push_str("  whole expression: CACHEABLE\n");
      } else if !self.frp_cached_subexprs.is_empty() {
        s.push_str(&format!(
          "  cached subtrees: {}\n",
          self.frp_cached_subexprs.len()
        ));
        for (i, c) in self.frp_cached_subexprs.iter().enumerate() {
          s.push_str(&format!(
            "    {}. [key=0x{:08x}, size={}] {}\n",
            i + 1,
            c.key,
            c.size,
            c.pretty
          ));
        }
      } else {
        s.push_str("  no caching (fully time-dependent)\n");
      }

      if let Some(ref stats) = self.frp_cache_stats {
        s.push_str(&format!(
          "  runtime: {} hits, {} misses ({:.1}% hit rate)\n",
          stats.hits,
          stats.misses,
          stats.hit_rate * 100.0
        ));
      }
    }

    s.push_str("===========================================\n");
    s
  }
}

/// 규칙 이름 인간 친화적으로 변환
fn humanize_rule(rule: &str) -> &str {
  match rule {
    "pow->sqrt" => "x^0.5 -> sqrt(x)",
    "pow->mul" => "x^2 -> x*x",
    "pow->div" => "x^-1 -> 1/x",
    "add_zero" => "x + 0 -> x",
    "mul_one" => "x * 1 -> x",
    "mul_zero" => "x * 0 -> 0",
    "double_neg" => "--x -> x",
    "add_assoc" => "(a+b)+c -> a+(b+c)",
    "mul_assoc" => "(a*b)*c -> a*(b*c)",
    "add_comm" => "a+b -> b+a",
    "mul_comm" => "a*b -> b*a",
    _ => rule,
  }
}

// ═══════════════════════════════════════════════════════════════
// 8. Provenance Builder (fluent API)
// ═══════════════════════════════════════════════════════════════

/// Provenance 빌더: fluent API로 SymbolicProvenance 생성
#[derive(Default)]
pub struct ProvenanceBuilder {
  /// 내부 Provenance 구조체 (빌더가 구성 중인 증명 로그)
  prov: SymbolicProvenance,
}

impl ProvenanceBuilder {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn zone(mut self, zone: EffectZone) -> Self {
    self.prov.zone = Some(zone);
    self
  }

  pub fn original_hash(mut self, hash: u64) -> Self {
    self.prov.original_hash = hash;
    self
  }

  pub fn result_hash(mut self, hash: u64) -> Self {
    self.prov.result_hash = hash;
    self
  }

  pub fn rule(mut self, rule: impl Into<String>) -> Self {
    self.prov.applied_rules.push(rule.into());
    self
  }

  pub fn rules<S: Into<String>>(mut self, rules: impl IntoIterator<Item = S>) -> Self {
    self
      .prov
      .applied_rules
      .extend(rules.into_iter().map(|r| r.into()));
    self
  }

  pub fn stats(mut self, stats: SimplifyStats) -> Self {
    self.prov.stats = stats;
    self
  }

  pub fn approximate(mut self, location: &str, reason: &str) -> Self {
    self.prov.record_approx(location, reason);
    self
  }

  pub fn temporal(mut self, decision: TemporalDecision) -> Self {
    self.prov.temporal_decisions.push(decision);
    self
  }

  pub fn build(self) -> SymbolicProvenance {
    self.prov
  }
}

// ═══════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_provenance_creation() {
    let prov = SymbolicProvenance::new();
    assert!(!prov.is_approximate);
    assert!(prov.applied_rules.is_empty());
  }

  #[test]
  fn test_record_rule() {
    let mut prov = SymbolicProvenance::new();
    prov.record_rule("add_zero");
    prov.record_rule("mul_one");

    assert_eq!(prov.applied_rules.len(), 2);
    assert_eq!(prov.applied_rules[0], "add_zero");
  }

  #[test]
  fn test_budget_tier_from_cost() {
    assert_eq!(BudgetTier::from_cost(10), BudgetTier::Light);
    assert_eq!(BudgetTier::from_cost(50), BudgetTier::Medium);
    assert_eq!(BudgetTier::from_cost(200), BudgetTier::Heavy);
  }

  #[test]
  fn test_cost_estimation_leaf() {
    assert_eq!(estimate_ir_cost(&IrExpr::ConstFloat(42.0)), 1);
    assert_eq!(estimate_ir_cost(&IrExpr::VarRef("x".to_string())), 1);
    assert_eq!(estimate_ir_cost(&IrExpr::TimeParam), 1);
  }

  #[test]
  fn test_cost_estimation_binary() {
    let add_expr = IrExpr::Add(
      Box::new(IrExpr::VarRef("a".to_string())),
      Box::new(IrExpr::VarRef("b".to_string())),
    );
    assert_eq!(estimate_ir_cost(&add_expr), 5);
  }

  #[test]
  fn test_differentiability_constants() {
    let float_const = IrExpr::ConstFloat(42.0);
    let analysis = analyze_differentiability(&float_const);
    assert!(analysis.is_differentiable);
  }

  #[test]
  fn test_differentiability_floor_not_differentiable() {
    let floor_expr = IrExpr::Floor(Box::new(IrExpr::VarRef("x".to_string())));
    let analysis = analyze_differentiability(&floor_expr);
    assert!(!analysis.is_differentiable);
    assert!(matches!(
      analysis.issues[0].reason,
      DifferentiabilityReason::DiscontinuousOp(_)
    ));
  }

  #[test]
  fn test_builder() {
    let prov = ProvenanceBuilder::new()
      .zone(EffectZone::Animation)
      .rule("add_zero")
      .original_hash(0x1234)
      .build();

    assert_eq!(prov.zone, Some(EffectZone::Animation));
    assert_eq!(prov.applied_rules.len(), 1);
  }

  #[test]
  fn test_adaptive_simplify_result() {
    let mut result = AdaptiveSimplifyResult::new(30, BudgetTier::Light);
    assert_eq!(result.final_tier, BudgetTier::Light);

    result.record_downgrade();
    assert_eq!(result.final_tier, BudgetTier::Light);
    assert!(result.had_timeout);
  }
}
