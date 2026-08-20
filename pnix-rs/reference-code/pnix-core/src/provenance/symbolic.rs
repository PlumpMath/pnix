//! SymbolicProvenance - Transformation proof log
//!
//! Records all decisions during symbolic transformation for debugging/education/CI verification

use super::{BudgetTier, DifferentiabilityAnalysis, NonDifferentiableOp, TemporalDecision};
use crate::effects::EffectZone;
use serde::{Deserialize, Serialize};

/// Category Theory 검증 결과: CT 법칙 검증 결과 타입
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CtValidationResult {
  /// 검증 통과
  #[default]
  Ok,
  /// 검증 실패 (이유 포함)
  Failed(
    /// 실패 이유
    String,
  ),
  /// 검증 스킵됨 (CT 태그 없음)
  Skipped,
}

impl CtValidationResult {
  /// 검증 통과 여부 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn is_ok(&self) -> bool {
    matches!(self, Self::Ok)
  }

  /// 검증 실패 여부 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn is_failed(&self) -> bool {
    matches!(self, Self::Failed(_))
  }
}

/// egg 단순화 통계: egg 단순화 과정의 통계 정보
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SimplifyStats {
  /// 실행된 iteration 수
  pub iterations: usize,
  /// 생성된 e-class 수
  pub eclasses_created: usize,
  /// 사용된 노드 수
  pub nodes_used: usize,
  /// 타임아웃 발생 여부
  pub timed_out: bool,
}

impl SimplifyStats {
  /// 새 통계 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(iterations: usize, eclasses: usize, nodes: usize) -> Self {
    Self {
      iterations,
      eclasses_created: eclasses,
      nodes_used: nodes,
      timed_out: false,
    }
  }

  /// 타임아웃 표시
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변경만, 값 계산 없음
  pub fn with_timeout(mut self) -> Self {
    self.timed_out = true;
    self
  }
}

/// 근사 발생 지점 기록: 근사 변환이 발생한 지점의 기록
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApproxPoint {
  /// 근사 발생 위치 (표현식 경로)
  pub location: String,
  /// 근사 이유
  pub reason: String,
}

impl ApproxPoint {
  /// 새 근사 지점 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(location: impl Into<String>, reason: impl Into<String>) -> Self {
    Self {
      location: location.into(),
      reason: reason.into(),
    }
  }
}

/// FRP 캐시 후보 정보: FRP 캐싱을 위한 서브트리 후보 정보
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CachedSubexprInfo {
  /// 캐시 키 (stable hash)
  pub key: u64,
  /// 서브트리 크기 (노드 수)
  pub size: u32,
  /// Pretty-printed 표현식
  pub pretty: String,
}

/// FRP 캐시 런타임 통계 기록: FRP 캐시의 런타임 성능 통계 기록
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FrpCacheStatsRecord {
  /// 캐시 히트 수
  pub hits: u64,
  /// 캐시 미스 수
  pub misses: u64,
  /// 히트 비율 (0.0 ~ 1.0, executor에서 계산됨)
  pub hit_rate: f64,
}

impl FrpCacheStatsRecord {
  // 헌법 준수 (P0-1): 값 계산 함수 제거
  // new()에서 hit_rate 계산은 executor/runtime 계층에서 구현하세요.
  // 구조 생성만 허용: hit_rate는 0.0으로 초기화하고 executor에서 계산
  /// 새 캐시 통계 기록 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(hits: u64, misses: u64) -> Self {
    Self {
      hits,
      misses,
      hit_rate: 0.0,
    }
  }
}

/// 심볼릭 변환 증명 로그
///
/// 변환 과정의 모든 결정을 기록하여 추적/디버깅/검증에 활용
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SymbolicProvenance {
  // ─────────────────────────────────────────────────────────
  // 기본 필드
  // ─────────────────────────────────────────────────────────
  /// 적용된 rewrite 규칙들 (순서대로)
  pub applied_rules: Vec<String>,

  /// 변환 전 표현식 해시 (무결성 검증용)
  pub original_hash: u64,

  /// 변환 후 표현식 해시
  pub result_hash: u64,

  // ─────────────────────────────────────────────────────────
  // Precision
  // ─────────────────────────────────────────────────────────
  /// 근사 변환 발생 여부
  pub is_approximate: bool,

  /// 근사 발생 지점들
  pub approx_points: Vec<ApproxPoint>,

  // ─────────────────────────────────────────────────────────
  // Temporal
  // ─────────────────────────────────────────────────────────
  /// 사용된 Zone 컨텍스트
  pub zone: Option<EffectZone>,

  /// 시간 변수 승격 결정들
  pub temporal_decisions: Vec<TemporalDecision>,

  // ─────────────────────────────────────────────────────────
  // egg 통계
  // ─────────────────────────────────────────────────────────
  /// egg 단순화 통계
  pub stats: SimplifyStats,

  /// 사용된 예산 티어
  pub budget_tier: Option<BudgetTier>,

  // ─────────────────────────────────────────────────────────
  // CT 검증
  // ─────────────────────────────────────────────────────────
  /// Category Theory 검증 결과
  pub ct_validation: CtValidationResult,

  // ─────────────────────────────────────────────────────────
  // 디버깅/교육용
  // ─────────────────────────────────────────────────────────
  /// 원본 표현식 pretty print
  #[serde(skip_serializing_if = "Option::is_none")]
  pub original_expr_pretty: Option<String>,

  /// 결과 표현식 pretty print
  #[serde(skip_serializing_if = "Option::is_none")]
  pub result_expr_pretty: Option<String>,

  // ─────────────────────────────────────────────────────────
  // FRP Cache
  // ─────────────────────────────────────────────────────────
  /// FRP 캐시 가능 여부 (전체 표현식)
  pub frp_whole_cacheable: bool,

  /// FRP 캐시 후보 서브트리 정보
  pub frp_cached_subexprs: Vec<CachedSubexprInfo>,

  /// FRP 캐시 런타임 통계
  pub frp_cache_stats: Option<FrpCacheStatsRecord>,

  // ─────────────────────────────────────────────────────────
  // Differentiability
  // ─────────────────────────────────────────────────────────
  /// 미분 가능 여부 (전체 표현식)
  pub differentiable: bool,

  /// 미분 불가능 지점들
  pub non_differentiable_ops: Vec<NonDifferentiableOp>,
}

impl SymbolicProvenance {
  /// 새 Provenance 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new() -> Self {
    Self::default()
  }

  /// 스킵된 변환용
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn skip(reason: impl Into<String>) -> Self {
    let mut prov = Self::new();
    prov.applied_rules.push(reason.into());
    prov
  }

  // ─────────────────────────────────────────────────────────
  // 규칙 기록
  // ─────────────────────────────────────────────────────────

  /// rewrite 규칙 적용 기록
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn record_rule(&mut self, rule: impl Into<String>) {
    self.applied_rules.push(rule.into());
  }

  /// 여러 규칙 한번에 기록
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn record_rules<S: Into<String>>(&mut self, rules: impl IntoIterator<Item = S>) {
    self
      .applied_rules
      .extend(rules.into_iter().map(|r| r.into()));
  }

  // ─────────────────────────────────────────────────────────
  // Precision 기록
  // ─────────────────────────────────────────────────────────

  /// 근사 발생 기록
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn record_approx(&mut self, location: impl Into<String>, reason: impl Into<String>) {
    self.is_approximate = true;
    self.approx_points.push(ApproxPoint::new(location, reason));
  }

  /// 큰 지수로 인한 근사
  ///
  /// ## 헌법 준수 (P0-1, C1)
  ///
  /// 텍스트 생성만, 값 계산 없음
  pub fn record_large_exponent(&mut self, exp: i64, limit: i64) {
    self.record_approx(
      "Pow",
      format!("large exponent {} exceeds limit {}", exp, limit),
    );
  }

  /// 비정수 지수로 인한 근사
  ///
  /// ## 헌법 준수 (P0-1, C1)
  ///
  /// 텍스트 생성만, 값 계산 없음
  pub fn record_non_integer_exponent(&mut self, exp: f64) {
    self.record_approx("Pow", format!("non-integer exponent {}", exp));
  }

  // ─────────────────────────────────────────────────────────
  // Temporal 기록
  // ─────────────────────────────────────────────────────────

  /// Zone 컨텍스트 설정
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변경만, 값 계산 없음
  pub fn set_zone(&mut self, zone: EffectZone) {
    self.zone = Some(zone);
  }

  /// 시간 변수 승격 결정 기록
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn record_temporal(&mut self, decision: TemporalDecision) {
    self.temporal_decisions.push(decision);
  }

  /// TimeParam 승격 기록
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn record_time_promotion(&mut self) {
    self
      .temporal_decisions
      .push(TemporalDecision::PromotedToTimeParam);
  }

  /// DeltaTime 승격 기록
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn record_delta_promotion(&mut self) {
    self
      .temporal_decisions
      .push(TemporalDecision::PromotedToDeltaTime);
  }

  // ─────────────────────────────────────────────────────────
  // egg 통계 기록
  // ─────────────────────────────────────────────────────────

  /// egg 통계 설정
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변경만, 값 계산 없음
  pub fn set_stats(&mut self, stats: SimplifyStats) {
    self.stats = stats;
  }

  /// 타임아웃 기록
  ///
  /// ## 헌법 준수 (P0-1, C1)
  ///
  /// 텍스트 생성만, 값 계산 없음
  pub fn record_timeout(&mut self, iterations: usize) {
    self.stats.timed_out = true;
    self.stats.iterations = iterations;
    self.record_approx("egg", format!("timeout after {} iterations", iterations));
  }

  // ─────────────────────────────────────────────────────────
  // 해시 계산
  // ─────────────────────────────────────────────────────────

  /// 원본 해시 설정
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변경만, 값 계산 없음
  pub fn set_original_hash(&mut self, hash: u64) {
    self.original_hash = hash;
  }

  /// 결과 해시 설정
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변경만, 값 계산 없음
  pub fn set_result_hash(&mut self, hash: u64) {
    self.result_hash = hash;
  }

  // ─────────────────────────────────────────────────────────
  // FRP Cache 기록
  // ─────────────────────────────────────────────────────────

  /// FRP 캐시 계획 기록
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변경만, 값 계산 없음
  pub fn record_frp_cache(&mut self, whole_cacheable: bool, candidates: Vec<CachedSubexprInfo>) {
    self.frp_whole_cacheable = whole_cacheable;
    self.frp_cached_subexprs = candidates;
  }

  /// 전체 표현식 캐시 가능으로 기록
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변경만, 값 계산 없음
  pub fn record_frp_whole_cacheable(&mut self) {
    self.frp_whole_cacheable = true;
  }

  /// FRP 캐시 후보 추가
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn add_cached_subexpr(&mut self, key: u64, size: u32, pretty: impl Into<String>) {
    self.frp_cached_subexprs.push(CachedSubexprInfo {
      key,
      size,
      pretty: pretty.into(),
    });
  }

  /// FRP 캐시 런타임 통계 기록
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn record_frp_cache_stats(&mut self, hits: u64, misses: u64) {
    self.frp_cache_stats = Some(FrpCacheStatsRecord::new(hits, misses));
  }

  // ─────────────────────────────────────────────────────────
  // Differentiability 기록
  // ─────────────────────────────────────────────────────────

  /// 미분 가능성 분석 결과 기록
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 복사만, 값 계산 없음
  pub fn record_differentiability(&mut self, analysis: &DifferentiabilityAnalysis) {
    self.differentiable = analysis.is_differentiable;
    self.non_differentiable_ops = analysis.issues.clone();
  }

  /// 표현식 미분 가능으로 기록
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변경만, 값 계산 없음
  pub fn set_differentiable(&mut self, value: bool) {
    self.differentiable = value;
  }

  // ─────────────────────────────────────────────────────────
  // 요약/출력
  // ─────────────────────────────────────────────────────────

  /// 요약 문자열 생성
  ///
  /// ## 헌법 준수 (P0-1, C1)
  ///
  /// 텍스트 생성만, 값 계산 없음
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

  /// 상세 리포트 생성
  pub fn to_report(&self) -> String {
    let mut s = String::new();

    s.push_str("===========================================\n");
    s.push_str("        Symbolic Provenance Report         \n");
    s.push_str("===========================================\n\n");

    // 해시
    s.push_str(&format!("Original hash: 0x{:016x}\n", self.original_hash));
    s.push_str(&format!("Result hash:   0x{:016x}\n", self.result_hash));
    s.push('\n');

    // Zone
    if let Some(zone) = self.zone {
      s.push_str(&format!("Zone: {:?}\n", zone));
    }

    // Precision
    s.push_str(&format!("Exact: {}\n", !self.is_approximate));
    if !self.approx_points.is_empty() {
      s.push_str("Approximations:\n");
      for ap in &self.approx_points {
        s.push_str(&format!("  - [{}] {}\n", ap.location, ap.reason));
      }
    }
    s.push('\n');

    // Applied rules
    s.push_str(&format!("Applied rules ({}):\n", self.applied_rules.len()));
    for (i, rule) in self.applied_rules.iter().enumerate() {
      s.push_str(&format!("  {}. {}\n", i + 1, humanize_rule(rule)));
    }
    s.push('\n');

    // Temporal decisions
    if !self.temporal_decisions.is_empty() {
      s.push_str("Temporal decisions:\n");
      for td in &self.temporal_decisions {
        s.push_str(&format!("  - {:?}\n", td));
      }
      s.push('\n');
    }

    // egg stats
    s.push_str("egg statistics:\n");
    s.push_str(&format!("  iterations: {}\n", self.stats.iterations));
    s.push_str(&format!("  e-classes: {}\n", self.stats.eclasses_created));
    s.push_str(&format!("  nodes: {}\n", self.stats.nodes_used));
    if self.stats.timed_out {
      s.push_str("  TIMED OUT\n");
    }

    // CT validation
    s.push_str(&format!("\nCT validation: {:?}\n", self.ct_validation));

    // FRP cache
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
// Provenance Builder (fluent API)
// ═══════════════════════════════════════════════════════════════

/// Provenance 빌더: fluent API를 제공하는 SymbolicProvenance 빌더
#[derive(Default)]
pub struct ProvenanceBuilder {
  prov: SymbolicProvenance,
}

impl ProvenanceBuilder {
  /// 새 빌더 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new() -> Self {
    Self::default()
  }

  /// Zone 설정
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변경만, 값 계산 없음
  pub fn zone(mut self, zone: EffectZone) -> Self {
    self.prov.zone = Some(zone);
    self
  }

  /// 원본 해시 설정
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변경만, 값 계산 없음
  pub fn original_hash(mut self, hash: u64) -> Self {
    self.prov.original_hash = hash;
    self
  }

  /// 결과 해시 설정
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변경만, 값 계산 없음
  pub fn result_hash(mut self, hash: u64) -> Self {
    self.prov.result_hash = hash;
    self
  }

  /// 규칙 추가
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn rule(mut self, rule: impl Into<String>) -> Self {
    self.prov.applied_rules.push(rule.into());
    self
  }

  /// 규칙들 추가
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn rules<S: Into<String>>(mut self, rules: impl IntoIterator<Item = S>) -> Self {
    self
      .prov
      .applied_rules
      .extend(rules.into_iter().map(|r| r.into()));
    self
  }

  /// 통계 설정
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변경만, 값 계산 없음
  pub fn stats(mut self, stats: SimplifyStats) -> Self {
    self.prov.stats = stats;
    self
  }

  /// 근사 지점 추가
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn approximate(mut self, location: &str, reason: &str) -> Self {
    self.prov.record_approx(location, reason);
    self
  }

  /// 시간 결정 추가
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn temporal(mut self, decision: TemporalDecision) -> Self {
    self.prov.temporal_decisions.push(decision);
    self
  }

  /// Provenance 빌드
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 반환만, 값 계산 없음
  pub fn build(self) -> SymbolicProvenance {
    self.prov
  }
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

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
  fn test_record_approx() {
    let mut prov = SymbolicProvenance::new();
    prov.record_large_exponent(10, 8);

    assert!(prov.is_approximate);
    assert_eq!(prov.approx_points.len(), 1);
    assert!(prov.approx_points[0].reason.contains("10"));
  }

  #[test]
  fn test_temporal_decision() {
    let mut prov = SymbolicProvenance::new();
    prov.set_zone(EffectZone::Frp);
    prov.record_time_promotion();

    assert_eq!(prov.zone, Some(EffectZone::Frp));
    assert_eq!(prov.temporal_decisions.len(), 1);
  }

  #[test]
  fn test_builder() {
    let prov = ProvenanceBuilder::new()
      .zone(EffectZone::Animation)
      .rule("add_zero")
      .rule("mul_one")
      .original_hash(0x1234)
      .result_hash(0x5678)
      .build();

    assert_eq!(prov.zone, Some(EffectZone::Animation));
    assert_eq!(prov.applied_rules.len(), 2);
    assert_eq!(prov.original_hash, 0x1234);
  }

  #[test]
  fn test_summary() {
    let mut prov = SymbolicProvenance::new();
    prov.record_rule("add_zero");
    prov.set_zone(EffectZone::Frp);

    let summary = prov.summary();
    assert!(summary.contains("1 applied"));
    assert!(summary.contains("Frp"));
  }

  #[test]
  fn test_report() {
    let mut prov = SymbolicProvenance::new();
    prov.set_original_hash(0xDEADBEEF);
    prov.set_result_hash(0xCAFEBABE);
    prov.record_rule("pow->sqrt");
    prov.record_time_promotion();

    let report = prov.to_report();
    assert!(report.contains("deadbeef"));
    assert!(report.contains("sqrt"));
  }

  #[test]
  fn test_humanize_rule() {
    assert_eq!(humanize_rule("pow->sqrt"), "x^0.5 -> sqrt(x)");
    assert_eq!(humanize_rule("unknown"), "unknown");
  }

  #[test]
  fn test_frp_cache() {
    let mut prov = SymbolicProvenance::new();
    prov.set_zone(EffectZone::Frp);
    prov.record_frp_whole_cacheable();

    assert!(prov.frp_whole_cacheable);
    assert!(prov.frp_cached_subexprs.is_empty());
  }

  #[test]
  fn test_frp_cache_subtrees() {
    let mut prov = SymbolicProvenance::new();
    prov.set_zone(EffectZone::Animation);
    prov.add_cached_subexpr(0x12345678, 5, "(a + b)");
    prov.add_cached_subexpr(0xDEADBEEF, 3, "c");

    assert!(!prov.frp_whole_cacheable);
    assert_eq!(prov.frp_cached_subexprs.len(), 2);
  }

  #[test]
  fn test_frp_cache_stats() {
    let mut prov = SymbolicProvenance::new();
    prov.record_frp_cache_stats(80, 20);

    let stats = prov.frp_cache_stats.as_ref().unwrap();
    assert_eq!(stats.hits, 80);
    assert_eq!(stats.misses, 20);
    // hit_rate 계산: hits / (hits + misses) (executor에서 구현)
    let expected_hit_rate = stats.hits as f64 / (stats.hits + stats.misses) as f64;
    assert!((expected_hit_rate - 0.8).abs() < 0.001);
  }

  #[test]
  fn test_ct_validation() {
    assert!(CtValidationResult::Ok.is_ok());
    assert!(!CtValidationResult::Ok.is_failed());

    assert!(!CtValidationResult::Failed("err".into()).is_ok());
    assert!(CtValidationResult::Failed("err".into()).is_failed());
  }

  #[test]
  fn test_simplify_stats() {
    let stats = SimplifyStats::new(10, 5, 20);
    assert_eq!(stats.iterations, 10);
    assert!(!stats.timed_out);

    let timeout_stats = stats.with_timeout();
    assert!(timeout_stats.timed_out);
  }

  #[test]
  fn test_serde() {
    let prov = ProvenanceBuilder::new()
      .zone(EffectZone::Pure)
      .rule("test")
      .build();

    let json = serde_json::to_string(&prov).unwrap();
    let restored: SymbolicProvenance = serde_json::from_str(&json).unwrap();
    assert_eq!(prov.zone, restored.zone);
    assert_eq!(prov.applied_rules, restored.applied_rules);
  }
}
