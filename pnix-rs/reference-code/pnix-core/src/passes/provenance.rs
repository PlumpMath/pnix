//! Symbolic Provenance - 변환 추적 시스템
//!
//! pnix-old의 symbolic_provenance.rs를 pnix-new(그래프 기반)에 맞게 적응.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 변환 추적만, 값 계산 없음.
//!
//! ## 기능
//!
//! - 적용된 최적화 규칙 기록
//! - Zone 컨텍스트 추적
//! - CT 검증 결과 기록
//! - 통계 정보 수집
//!
//! ## 사용 예시
//!
//! ```ignore
//! let mut prov = Provenance::new();
//! prov.record_rule("identity_elimination");
//! prov.set_zone(EffectZone::Pure);
//! println!("{}", prov.summary());
//! ```

use crate::effects::EffectZone;
use serde::{Deserialize, Serialize};

// ============================================================
// Temporal Decision
// ============================================================

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

// ============================================================
// CT Validation Result
// ============================================================

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
  pub fn is_ok(&self) -> bool {
    matches!(self, Self::Ok)
  }

  pub fn is_failed(&self) -> bool {
    matches!(self, Self::Failed(_))
  }
}

// ============================================================
// Optimization Stats
// ============================================================

/// 최적화 통계: 최적화 과정의 통계 정보
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct OptStats {
  /// 실행된 패스 수
  pub passes_run: usize,
  /// 변환이 적용된 패스 수
  pub passes_applied: usize,
  /// 제거된 노드 수
  pub nodes_removed: usize,
  /// 제거된 엣지 수
  pub edges_removed: usize,
  /// 융합된 노드 수
  pub nodes_fused: usize,
  /// 타임아웃 발생 여부
  pub timed_out: bool,
}

impl OptStats {
  /// 노드 제거 기록
  pub fn record_node_removal(&mut self, count: usize) {
    self.nodes_removed += count;
  }

  /// 엣지 제거 기록
  pub fn record_edge_removal(&mut self, count: usize) {
    self.edges_removed += count;
  }

  /// 노드 융합 기록
  pub fn record_fusion(&mut self, count: usize) {
    self.nodes_fused += count;
  }

  /// 패스 적용 기록
  pub fn record_pass_applied(&mut self) {
    self.passes_run += 1;
    self.passes_applied += 1;
  }

  /// 패스 스킵 기록
  pub fn record_pass_skipped(&mut self) {
    self.passes_run += 1;
  }
}

// ============================================================
// Approximate Point
// ============================================================

/// 근사 발생 지점 기록: 근사 변환이 발생한 지점의 기록
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApproxPoint {
  /// 근사 발생 위치 (노드 이름 또는 경로)
  pub location: String,
  /// 근사 이유
  pub reason: String,
}

// ============================================================
// Provenance (Main Structure)
// ============================================================

/// 변환 추적 로그 (Provenance)
///
/// 컴파일 과정의 모든 결정을 기록하여 디버깅/검증에 활용.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Provenance {
  // ─────────────────────────────────────────────────────────
  // 기본 필드
  // ─────────────────────────────────────────────────────────
  /// 적용된 규칙들 (순서대로)
  pub applied_rules: Vec<String>,

  /// 변환 전 해시 (무결성 검증용)
  pub original_hash: u64,

  /// 변환 후 해시
  pub result_hash: u64,

  // ─────────────────────────────────────────────────────────
  // Zone 컨텍스트
  // ─────────────────────────────────────────────────────────
  /// 사용된 Zone 컨텍스트
  pub zone: Option<EffectZone>,

  /// 시간 변수 승격 결정들
  pub temporal_decisions: Vec<TemporalDecision>,

  // ─────────────────────────────────────────────────────────
  // Precision
  // ─────────────────────────────────────────────────────────
  /// 근사 변환 발생 여부
  pub is_approximate: bool,

  /// 근사 발생 지점들
  pub approx_points: Vec<ApproxPoint>,

  // ─────────────────────────────────────────────────────────
  // 통계
  // ─────────────────────────────────────────────────────────
  /// 최적화 통계
  pub stats: OptStats,

  // ─────────────────────────────────────────────────────────
  // CT 검증
  // ─────────────────────────────────────────────────────────
  /// Category Theory 검증 결과
  pub ct_validation: CtValidationResult,

  // ─────────────────────────────────────────────────────────
  // 디버깅용 (선택적)
  // ─────────────────────────────────────────────────────────
  /// 모듈 이름
  #[serde(skip_serializing_if = "Option::is_none")]
  pub module_name: Option<String>,

  /// 경고 메시지들
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub warnings: Vec<String>,
}

impl Provenance {
  /// 새 Provenance 생성
  pub fn new() -> Self {
    Self::default()
  }

  /// 모듈 이름으로 생성
  pub fn for_module(name: impl Into<String>) -> Self {
    let mut prov = Self::new();
    prov.module_name = Some(name.into());
    prov
  }

  // ─────────────────────────────────────────────────────────
  // 규칙 기록
  // ─────────────────────────────────────────────────────────

  /// 규칙 적용 기록
  pub fn record_rule(&mut self, rule: impl Into<String>) {
    self.applied_rules.push(rule.into());
  }

  /// 여러 규칙 한번에 기록
  pub fn record_rules<S: Into<String>>(&mut self, rules: impl IntoIterator<Item = S>) {
    self
      .applied_rules
      .extend(rules.into_iter().map(|r| r.into()));
  }

  // ─────────────────────────────────────────────────────────
  // Zone 기록
  // ─────────────────────────────────────────────────────────

  /// Zone 컨텍스트 설정
  pub fn set_zone(&mut self, zone: EffectZone) {
    self.zone = Some(zone);
  }

  /// 시간 변수 승격 결정 기록
  pub fn record_temporal(&mut self, decision: TemporalDecision) {
    self.temporal_decisions.push(decision);
  }

  // ─────────────────────────────────────────────────────────
  // Precision 기록
  // ─────────────────────────────────────────────────────────

  /// 근사 발생 기록
  pub fn record_approx(&mut self, location: impl Into<String>, reason: impl Into<String>) {
    self.is_approximate = true;
    self.approx_points.push(ApproxPoint {
      location: location.into(),
      reason: reason.into(),
    });
  }

  // ─────────────────────────────────────────────────────────
  // 해시 기록
  // ─────────────────────────────────────────────────────────

  /// 원본 해시 설정
  pub fn set_original_hash(&mut self, hash: u64) {
    self.original_hash = hash;
  }

  /// 결과 해시 설정
  pub fn set_result_hash(&mut self, hash: u64) {
    self.result_hash = hash;
  }

  // ─────────────────────────────────────────────────────────
  // CT 검증
  // ─────────────────────────────────────────────────────────

  /// CT 검증 성공
  pub fn ct_passed(&mut self) {
    self.ct_validation = CtValidationResult::Ok;
  }

  /// CT 검증 실패
  pub fn ct_failed(&mut self, reason: impl Into<String>) {
    self.ct_validation = CtValidationResult::Failed(reason.into());
  }

  /// CT 검증 스킵
  pub fn ct_skipped(&mut self) {
    self.ct_validation = CtValidationResult::Skipped;
  }

  // ─────────────────────────────────────────────────────────
  // 경고
  // ─────────────────────────────────────────────────────────

  /// 경고 추가
  pub fn add_warning(&mut self, msg: impl Into<String>) {
    self.warnings.push(msg.into());
  }

  // ─────────────────────────────────────────────────────────
  // 출력
  // ─────────────────────────────────────────────────────────

  /// 요약 문자열 생성
  pub fn summary(&self) -> String {
    let mut s = String::new();
    if let Some(ref name) = self.module_name {
      s.push_str(&format!("Module: {}\n", name));
    }
    s.push_str(&format!("Rules: {} applied\n", self.applied_rules.len()));
    s.push_str(&format!("Approximate: {}\n", self.is_approximate));
    if let Some(zone) = self.zone {
      s.push_str(&format!("Zone: {:?}\n", zone));
    }
    s.push_str(&format!("CT validation: {:?}\n", self.ct_validation));
    s.push_str(&format!(
      "Stats: {} passes, {} nodes removed, {} fused\n",
      self.stats.passes_run, self.stats.nodes_removed, self.stats.nodes_fused
    ));
    s
  }

  /// 상세 리포트 생성
  pub fn to_report(&self) -> String {
    let mut s = String::new();

    s.push_str("═══════════════════════════════════════\n");
    s.push_str("           Provenance Report           \n");
    s.push_str("═══════════════════════════════════════\n\n");

    // Module
    if let Some(ref name) = self.module_name {
      s.push_str(&format!("Module: {}\n", name));
    }

    // Hashes
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
      s.push_str(&format!("  {}. {}\n", i + 1, rule));
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

    // Statistics
    s.push_str("Statistics:\n");
    s.push_str(&format!("  Passes run: {}\n", self.stats.passes_run));
    s.push_str(&format!(
      "  Passes applied: {}\n",
      self.stats.passes_applied
    ));
    s.push_str(&format!("  Nodes removed: {}\n", self.stats.nodes_removed));
    s.push_str(&format!("  Edges removed: {}\n", self.stats.edges_removed));
    s.push_str(&format!("  Nodes fused: {}\n", self.stats.nodes_fused));
    if self.stats.timed_out {
      s.push_str("  ⚠️ TIMED OUT\n");
    }
    s.push('\n');

    // CT validation
    s.push_str(&format!("CT validation: {:?}\n", self.ct_validation));

    // Warnings
    if !self.warnings.is_empty() {
      s.push_str("\nWarnings:\n");
      for warn in &self.warnings {
        s.push_str(&format!("  ⚠️ {}\n", warn));
      }
    }

    s.push_str("═══════════════════════════════════════\n");
    s
  }
}

// ============================================================
// Provenance Builder
// ============================================================

/// Provenance 빌더: fluent API를 제공하는 Provenance 빌더
#[derive(Default)]
pub struct ProvenanceBuilder {
  prov: Provenance,
}

impl ProvenanceBuilder {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn module(mut self, name: impl Into<String>) -> Self {
    self.prov.module_name = Some(name.into());
    self
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

  pub fn approximate(mut self, location: &str, reason: &str) -> Self {
    self.prov.record_approx(location, reason);
    self
  }

  pub fn temporal(mut self, decision: TemporalDecision) -> Self {
    self.prov.temporal_decisions.push(decision);
    self
  }

  pub fn warning(mut self, msg: impl Into<String>) -> Self {
    self.prov.warnings.push(msg.into());
    self
  }

  pub fn build(self) -> Provenance {
    self.prov
  }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_provenance_creation() {
    let prov = Provenance::new();
    assert!(!prov.is_approximate);
    assert!(prov.applied_rules.is_empty());
  }

  #[test]
  fn test_provenance_for_module() {
    let prov = Provenance::for_module("test_module");
    assert_eq!(prov.module_name, Some("test_module".to_string()));
  }

  #[test]
  fn test_record_rule() {
    let mut prov = Provenance::new();
    prov.record_rule("identity_elimination");
    prov.record_rule("dead_code");

    assert_eq!(prov.applied_rules.len(), 2);
    assert_eq!(prov.applied_rules[0], "identity_elimination");
  }

  #[test]
  fn test_record_rules() {
    let mut prov = Provenance::new();
    prov.record_rules(["rule1", "rule2", "rule3"]);

    assert_eq!(prov.applied_rules.len(), 3);
  }

  #[test]
  fn test_record_approx() {
    let mut prov = Provenance::new();
    prov.record_approx("node_a", "precision loss");

    assert!(prov.is_approximate);
    assert_eq!(prov.approx_points.len(), 1);
    assert_eq!(prov.approx_points[0].location, "node_a");
  }

  #[test]
  fn test_temporal_decision() {
    let mut prov = Provenance::new();
    prov.set_zone(EffectZone::Frp);
    prov.record_temporal(TemporalDecision::PromotedToTimeParam);

    assert_eq!(prov.zone, Some(EffectZone::Frp));
    assert_eq!(prov.temporal_decisions.len(), 1);
  }

  #[test]
  fn test_temporal_rejection() {
    let decision = TemporalDecision::rejected_time("t", EffectZone::Pure);

    if let TemporalDecision::RejectedByZone { var_name, zone, .. } = decision {
      assert_eq!(var_name, "t");
      assert_eq!(zone, EffectZone::Pure);
    } else {
      panic!("Expected RejectedByZone");
    }
  }

  #[test]
  fn test_ct_validation() {
    let mut prov = Provenance::new();
    assert!(matches!(prov.ct_validation, CtValidationResult::Ok));

    prov.ct_failed("identity law violated");
    assert!(prov.ct_validation.is_failed());

    prov.ct_passed();
    assert!(prov.ct_validation.is_ok());
  }

  #[test]
  fn test_opt_stats() {
    let mut stats = OptStats::default();

    stats.record_node_removal(3);
    stats.record_edge_removal(2);
    stats.record_fusion(1);
    stats.record_pass_applied();
    stats.record_pass_skipped();

    assert_eq!(stats.nodes_removed, 3);
    assert_eq!(stats.edges_removed, 2);
    assert_eq!(stats.nodes_fused, 1);
    assert_eq!(stats.passes_run, 2);
    assert_eq!(stats.passes_applied, 1);
  }

  #[test]
  fn test_builder() {
    let prov = ProvenanceBuilder::new()
      .module("test")
      .zone(EffectZone::Animation)
      .rule("fusion")
      .rule("elimination")
      .original_hash(0x1234)
      .result_hash(0x5678)
      .build();

    assert_eq!(prov.module_name, Some("test".to_string()));
    assert_eq!(prov.zone, Some(EffectZone::Animation));
    assert_eq!(prov.applied_rules.len(), 2);
    assert_eq!(prov.original_hash, 0x1234);
  }

  #[test]
  fn test_summary() {
    let mut prov = Provenance::for_module("example");
    prov.record_rule("dead_code");
    prov.set_zone(EffectZone::Pure);

    let summary = prov.summary();
    assert!(summary.contains("example"));
    assert!(summary.contains("1 applied"));
    assert!(summary.contains("Pure"));
  }

  #[test]
  fn test_report() {
    let prov = ProvenanceBuilder::new()
      .module("test_report")
      .original_hash(0xDEADBEEF)
      .result_hash(0xCAFEBABE)
      .rule("identity_elimination")
      .zone(EffectZone::Frp)
      .temporal(TemporalDecision::PromotedToTimeParam)
      .warning("deprecated pattern")
      .build();

    let report = prov.to_report();
    assert!(report.contains("test_report"));
    assert!(report.contains("deadbeef"));
    assert!(report.contains("identity_elimination"));
    assert!(report.contains("Frp"));
    assert!(report.contains("deprecated pattern"));
  }

  #[test]
  fn test_warnings() {
    let mut prov = Provenance::new();
    prov.add_warning("unused node");
    prov.add_warning("potential cycle");

    assert_eq!(prov.warnings.len(), 2);

    let report = prov.to_report();
    assert!(report.contains("unused node"));
    assert!(report.contains("potential cycle"));
  }
}
