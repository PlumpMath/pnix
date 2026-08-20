//! Phase 7 P2: Judgment pipeline — pnix가 소유하는 판단 오케스트레이션.
//!
//! doghouse에서 이동한 핵심 판단 로직:
//! - `ontology_query_decision`: evaluate → select → promote 오케스트레이션
//! - `ontology_query_lift`: context lift + fact lowering
//! - `ontology_query_policy_for_intent`: EvaluationPolicy 구성
//! - `computation_ontology_query_decision`: 계산 경로 wrapper
//! - `append_ontology_query_decision`: 결과를 facts/notes에 기록 + events 수집
//!
//! IO가 없다. QueryRouteSpec은 호출자(doghouse)가 해결해서 매개변수로 전달.
//! pnix-core의 순수성(no IO, no network, no time) 유지.

use crate::judgement_protocol::{JudgementEvent, PromotionEvent};
use crate::ontology::{
  ontology_evaluate, ontology_lift_fact, ontology_promote_with_lane, ontology_select, ContextId,
  ContextualFact, EvaluationPolicy, EvaluationVector, EvidenceLane, Interpretation,
  InterpretationId, LossKind, LossPolicy, MeaningLift, PromotionDecision, SelectionOutcome,
};
use std::collections::{BTreeMap, HashSet};

// ---------------------------------------------------------------------------
// QueryRouteSpec: route별 정책 (IO 없는 pure data)
// ---------------------------------------------------------------------------

/// route별 쿼리 정책. doghouse가 .px에서 로딩하여 전달한다.
#[derive(Debug, Clone)]
pub struct QueryRouteSpec {
  pub query_context: String,
  pub include_hop_knowledge: bool,
  pub default_preview: usize,
  /// Ontology evaluation policy overrides (0.0 means "use default").
  pub policy_coverage: f64,
  pub policy_coherence: f64,
  pub policy_loss: f64,
  pub policy_cost: f64,
  pub policy_accept_threshold: f64,
}

impl Default for QueryRouteSpec {
  fn default() -> Self {
    Self {
      query_context: "Doghouse.QueryModel.default".to_string(),
      include_hop_knowledge: true,
      default_preview: 5,
      policy_coverage: 0.0,
      policy_coherence: 0.0,
      policy_loss: 0.0,
      policy_cost: 0.0,
      policy_accept_threshold: 0.0,
    }
  }
}

// ---------------------------------------------------------------------------
// Intent types (판단에 필요한 최소 subset)
// ---------------------------------------------------------------------------

/// 출력 범위. doghouse intent.rs에서 정의된 것의 pure mirror.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputScope {
  Brief,
  Standard,
  Detailed,
}

/// 판단에 필요한 intent 정보의 최소 subset.
/// doghouse의 full QueryIntent에서 추출하여 전달.
#[derive(Debug, Clone)]
pub struct JudgementIntent {
  /// doghouse `IntentType` 같은 caller-local intent classifier label.
  /// pnix-core는 enum을 소유하지 않고, projection parity를 위해 문자열만 보관한다.
  pub intent_type: Option<String>,
  pub output_scope: OutputScope,
}

impl JudgementIntent {
  pub fn new(output_scope: OutputScope) -> Self {
    Self {
      intent_type: None,
      output_scope,
    }
  }

  pub fn with_intent_type(output_scope: OutputScope, intent_type: impl Into<String>) -> Self {
    Self {
      intent_type: Some(intent_type.into()),
      output_scope,
    }
  }
}

// ---------------------------------------------------------------------------
// OntologyQueryDecision: 판단 결과 구조체
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct OntologyQueryDecision {
  pub lift: MeaningLift,
  pub lifted_facts: Vec<ContextualFact>,
  pub evaluations: Vec<EvaluationVector>,
  pub selection: SelectionOutcome,
  pub promotion: PromotionDecision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OntologyDecisionOwner<'a> {
  pub namespace: &'a str,
  pub context: &'a str,
  pub subject: &'a str,
}

impl<'a> OntologyDecisionOwner<'a> {
  pub const fn new(namespace: &'a str, context: &'a str, subject: &'a str) -> Self {
    Self {
      namespace,
      context,
      subject,
    }
  }

  pub const fn doghouse() -> Self {
    Self::new("doghouse", "Doghouse.OntologyQuery", "doghouse")
  }
}

// ---------------------------------------------------------------------------
// DecisionEvents: pending event accumulator
// ---------------------------------------------------------------------------

/// 판단 결과를 수집하는 accumulator.
/// ontology_query_decision이 호출될 때마다 여기에 pending decision이 쌓인다.
/// persist boundary에서 actual request_id + seq로 materialize한다.
#[derive(Default)]
pub struct DecisionEvents {
  pub pending: Vec<PendingDecisionEvent>,
}

#[derive(Clone)]
pub struct PendingDecisionEvent {
  pub route: String,
  pub selection: SelectionOutcome,
  pub promotion: PromotionDecision,
  pub trace: Vec<String>,
  pub provenance: Vec<String>,
}

impl DecisionEvents {
  pub fn push_decision(
    &mut self,
    route: &str,
    decision: &OntologyQueryDecision,
    provenance: &[String],
  ) {
    self.pending.push(PendingDecisionEvent {
      route: route.to_string(),
      selection: decision.selection.clone(),
      promotion: decision.promotion.clone(),
      trace: vec![format!("policy:{route}")],
      provenance: provenance.to_vec(),
    });
  }

  pub fn into_protocol_events(
    self,
    request_id: &str,
    seq: u64,
  ) -> (Vec<JudgementEvent>, Vec<PromotionEvent>) {
    let mut judgement_events = Vec::with_capacity(self.pending.len());
    let mut promotion_events = Vec::with_capacity(self.pending.len());
    for (index, pending) in self.pending.into_iter().enumerate() {
      let judgement_event_id = format!("je.{request_id}.{seq}.{index}.{}", pending.route);
      let judgement_event = JudgementEvent::from_selection_with_event_id(
        judgement_event_id,
        request_id,
        seq,
        &pending.selection,
        pending.trace,
        pending.provenance,
      );
      let promotion_event = PromotionEvent::from_promotion(&judgement_event.id, &pending.promotion);
      judgement_events.push(judgement_event);
      promotion_events.push(promotion_event);
    }
    (judgement_events, promotion_events)
  }
}

// ---------------------------------------------------------------------------
// Core judgment functions (pure, no IO)
// ---------------------------------------------------------------------------

/// Interpretation 생성 헬퍼.
pub fn interpretation_with_refs(id: impl Into<String>, fact_refs: Vec<String>) -> Interpretation {
  Interpretation {
    id: InterpretationId::from(id.into()),
    observation_refs: vec![],
    fact_refs,
    lift_refs: vec![],
    conflict_refs: vec![],
    loss: None,
  }
}

/// EvaluationPolicy 구성. route spec + intent scope 반영.
pub fn ontology_query_policy_for_intent(
  route: &str,
  route_spec: &QueryRouteSpec,
  intent: Option<&JudgementIntent>,
) -> EvaluationPolicy {
  let mut policy = EvaluationPolicy {
    id: format!("ontology.query.{route}"),
    ..EvaluationPolicy::default()
  };
  if route_spec.policy_coverage > 0.0 {
    policy.weights.coverage = route_spec.policy_coverage;
  }
  if route_spec.policy_coherence > 0.0 {
    policy.weights.coherence = route_spec.policy_coherence;
  }
  if route_spec.policy_loss > 0.0 {
    policy.weights.loss = route_spec.policy_loss;
  }
  if route_spec.policy_cost > 0.0 {
    policy.weights.cost = route_spec.policy_cost;
  }
  if route_spec.policy_accept_threshold > 0.0 {
    policy.accept_threshold = route_spec.policy_accept_threshold;
  }
  if let Some(intent) = intent {
    match intent.output_scope {
      OutputScope::Brief => {
        policy.weights.cost = policy.weights.cost.max(0.8);
      }
      OutputScope::Detailed => {
        if policy.accept_threshold == EvaluationPolicy::default().accept_threshold {
          policy.accept_threshold = 0.65;
        }
      }
      OutputScope::Standard => {}
    }
  }
  policy
}

/// Context lift + fact lowering.
pub fn ontology_query_lift(
  route: &str,
  route_spec: &QueryRouteSpec,
  facts: &[ContextualFact],
  intent: Option<&JudgementIntent>,
) -> (MeaningLift, Vec<ContextualFact>) {
  let from_context = facts
    .first()
    .map(|fact| fact.context.clone())
    .unwrap_or_else(|| ContextId::from("Doghouse.Source".to_string()));
  let mut notes = vec![format!("query-route:{route}")];
  if let Some(intent) = intent {
    if let Some(intent_type) = &intent.intent_type {
      notes.push(format!("intent:{intent_type}"));
    }
    notes.push(format!("scope:{:?}", intent.output_scope));
  }
  let lift = MeaningLift {
    id: format!("lift.doghouse.query.{route}"),
    from_context,
    to_context: ContextId::from(route_spec.query_context.clone()),
    object_map: BTreeMap::new(),
    relation_map: BTreeMap::new(),
    loss: Some(LossPolicy {
      kind: LossKind::Lossless,
      notes,
    }),
  };
  let lifted_facts = facts
    .iter()
    .map(|fact| ontology_lift_fact(&lift, fact))
    .collect();
  (lift, lifted_facts)
}

/// 핵심 판단 오케스트레이션 (lane-aware): evaluate → select → lane-gated promote.
///
/// OWNER-LAW (2026-05-10): pnix 는 LLM 없이 작동하는 deterministic AI substrate
/// (`CLAUDE.md` OWNER-LAW CONSTITUTION). external lane 의 `Accept` 는
/// `ontology_promote_with_lane` 가 자동으로 `Candidate` 로 downgrade 한다 —
/// external prose 는 owner-law proof 없이 `Accepted` 가 될 수 없다. 새 caller 는
/// 이 lane-aware 시그니처를 사용하고, lane 을 명시한다.
pub fn ontology_query_decision_with_lane(
  route: &str,
  route_spec: &QueryRouteSpec,
  interpretations: Vec<Interpretation>,
  facts: &[ContextualFact],
  intent: Option<&JudgementIntent>,
  lane: EvidenceLane,
) -> Option<OntologyQueryDecision> {
  if interpretations.is_empty() || facts.is_empty() {
    return None;
  }
  let policy = ontology_query_policy_for_intent(route, route_spec, intent);
  let (lift, lifted_facts) = ontology_query_lift(route, route_spec, facts, intent);
  let lifted_interpretations = interpretations
    .into_iter()
    .map(|mut interpretation| {
      interpretation.lift_refs.push(lift.id.clone());
      interpretation
    })
    .collect::<Vec<_>>();
  let evaluations = lifted_interpretations
    .iter()
    .map(|interpretation| ontology_evaluate(&policy, interpretation, &lifted_facts))
    .collect::<Vec<_>>();
  let selection = ontology_select(&policy, &lifted_interpretations, &lifted_facts)?;
  let promotion = ontology_promote_with_lane(&policy, lane, &selection.judgement);
  Some(OntologyQueryDecision {
    lift,
    lifted_facts,
    evaluations,
    selection,
    promotion,
  })
}

/// Backward-compatible internal-only orchestration (pre-lane callers).
///
/// **WARNING (2026-05-10):** 이 함수는 `EvidenceLane::InternalOwnerLaw` 로
/// 고정된다. external evidence (web search / API / OCR / human prose / tool
/// result / peer evidence) 에서는 절대 호출하지 않는다 — `ontology_query_decision_with_lane`
/// 을 명시 lane 과 함께 사용한다. external Accept 가 이 함수를 통하면 owner-law
/// 위반 (Accepted 직접 승격) 이다.
pub fn ontology_query_decision(
  route: &str,
  route_spec: &QueryRouteSpec,
  interpretations: Vec<Interpretation>,
  facts: &[ContextualFact],
  intent: Option<&JudgementIntent>,
) -> Option<OntologyQueryDecision> {
  ontology_query_decision_with_lane(
    route,
    route_spec,
    interpretations,
    facts,
    intent,
    EvidenceLane::InternalOwnerLaw,
  )
}

/// 계산 경로 전용 wrapper.
pub fn computation_ontology_query_decision(
  route: &str,
  route_spec: &QueryRouteSpec,
  source_facts: &[ContextualFact],
  direct_predicates: &[&str],
  intent: &JudgementIntent,
  fact_ref_ids: impl Fn(&[ContextualFact]) -> Vec<String>,
  filtered_fact_ref_ids: impl Fn(&[ContextualFact], &[&str]) -> Vec<String>,
) -> Option<OntologyQueryDecision> {
  if source_facts.is_empty() {
    return None;
  }
  let route_id = route.replace('-', ".");
  let direct_refs = filtered_fact_ref_ids(source_facts, direct_predicates);
  let rich_refs = fact_ref_ids(source_facts);
  let mut interpretations = Vec::new();
  if !direct_refs.is_empty() {
    interpretations.push(interpretation_with_refs(
      format!("interp.{route_id}.direct"),
      direct_refs.clone(),
    ));
  }
  if !rich_refs.is_empty() {
    if rich_refs != direct_refs {
      interpretations.push(interpretation_with_refs(
        format!("interp.{route_id}.rich"),
        rich_refs,
      ));
    } else if interpretations.is_empty() {
      interpretations.push(interpretation_with_refs(
        format!("interp.{route_id}.rich"),
        direct_refs,
      ));
    }
  }
  ontology_query_decision(
    route,
    route_spec,
    interpretations,
    source_facts,
    Some(intent),
  )
}

// ---------------------------------------------------------------------------
// append_ontology_query_decision: 결과를 facts/notes에 기록 + events 수집
// ---------------------------------------------------------------------------

/// 판단 결과를 facts/notes에 기록하고, events accumulator에 pending decision을 추가.
pub fn append_ontology_query_decision(
  facts: &mut Vec<ContextualFact>,
  notes: &mut Vec<String>,
  provenance: &[String],
  route: &str,
  decision: &OntologyQueryDecision,
  events: Option<&mut DecisionEvents>,
  // fact 생성 콜백 — doghouse의 make_fact를 주입
  make_fact: impl Fn(String, &str, &str, &str, String, Vec<String>) -> ContextualFact,
) {
  append_ontology_query_decision_for_owner(
    facts,
    notes,
    provenance,
    route,
    decision,
    events,
    OntologyDecisionOwner::doghouse(),
    make_fact,
  );
}

/// 판단 결과를 facts/notes에 기록하고, events accumulator에 pending decision을 추가.
/// owner는 fact id/context/subject namespace를 결정한다.
pub fn append_ontology_query_decision_for_owner(
  facts: &mut Vec<ContextualFact>,
  notes: &mut Vec<String>,
  provenance: &[String],
  route: &str,
  decision: &OntologyQueryDecision,
  events: Option<&mut DecisionEvents>,
  owner: OntologyDecisionOwner<'_>,
  make_fact: impl Fn(String, &str, &str, &str, String, Vec<String>) -> ContextualFact,
) {
  if let Some(events) = events {
    events.push_decision(route, decision, provenance);
  }
  let provenance = provenance.to_vec();
  facts.push(make_fact(
    format!("fact.{}.ontology.lift.{route}", owner.namespace),
    owner.context,
    owner.subject,
    "ontology-lift-id",
    decision.lift.id.clone(),
    provenance.clone(),
  ));
  facts.push(make_fact(
    format!("fact.{}.ontology.context.{route}", owner.namespace),
    owner.context,
    owner.subject,
    "ontology-query-context",
    decision.lift.to_context.0.clone(),
    provenance.clone(),
  ));
  facts.push(make_fact(
    format!("fact.{}.ontology.evaluations.{route}", owner.namespace),
    owner.context,
    owner.subject,
    "ontology-evaluation-count",
    decision.evaluations.len().to_string(),
    provenance.clone(),
  ));
  for (index, evaluation) in decision.evaluations.iter().enumerate().take(4) {
    facts.push(make_fact(
      format!(
        "fact.{}.ontology.eval.{route}.{index}.interpretation",
        owner.namespace
      ),
      owner.context,
      owner.subject,
      "ontology-evaluation-interpretation",
      evaluation.interpretation.0.clone(),
      provenance.clone(),
    ));
    facts.push(make_fact(
      format!(
        "fact.{}.ontology.eval.{route}.{index}.score",
        owner.namespace
      ),
      owner.context,
      owner.subject,
      "ontology-evaluation-score-candidate",
      format!("{:.4}", evaluation.score),
      provenance.clone(),
    ));
  }
  facts.push(make_fact(
    format!("fact.{}.ontology.selected.{route}", owner.namespace),
    owner.context,
    owner.subject,
    "ontology-selected-interpretation",
    decision
      .selection
      .judgement
      .chosen_interpretation
      .as_ref()
      .map(|id| id.0.clone())
      .unwrap_or_else(|| "none".to_string()),
    provenance.clone(),
  ));
  facts.push(make_fact(
    format!("fact.{}.ontology.score.{route}", owner.namespace),
    owner.context,
    owner.subject,
    "ontology-evaluation-score",
    format!("{:.4}", decision.selection.evaluation.score),
    provenance.clone(),
  ));
  facts.push(make_fact(
    format!("fact.{}.ontology.action.{route}", owner.namespace),
    owner.context,
    owner.subject,
    "ontology-judgement-action",
    format!("{:?}", decision.selection.judgement.action),
    provenance.clone(),
  ));
  facts.push(make_fact(
    format!("fact.{}.ontology.promotion.{route}", owner.namespace),
    owner.context,
    owner.subject,
    "ontology-promotion-status",
    format!("{:?}", decision.promotion.target_status),
    provenance.clone(),
  ));
  notes.push(format!(
    "ontology-lift:route:{route}:context:{}:facts:{}",
    decision.lift.to_context.0,
    decision.lifted_facts.len()
  ));
  notes.push(format!(
    "ontology-evaluate:route:{route}:candidates:{}",
    decision.evaluations.len()
  ));
  // 6축 평가 점수 — 선택된 interpretation의 evaluation vector
  let ev = &decision.selection.evaluation;
  notes.push(format!(
    "ontology-evaluation-axes:coherence={:.2}:coverage={:.2}:loss={:.2}:cost={:.2}:replayability={:.2}:safety={:.2}:score={:.4}",
    ev.coherence, ev.coverage, ev.loss_penalty, ev.cost, ev.replayability, ev.safety, ev.score
  ));
  notes.push(format!(
    "ontology-select:route:{route}:interpretation:{}",
    decision
      .selection
      .judgement
      .chosen_interpretation
      .as_ref()
      .map(|id| id.0.as_str())
      .unwrap_or("none")
  ));
  notes.push(format!(
    "ontology-promote:route:{route}:status:{:?}",
    decision.promotion.target_status
  ));
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use super::*;
  use crate::ontology::{ContextId, ContextualFact, LayerId, MeaningId, MeaningStatus};

  fn test_fact(subj: &str, pred: &str, obj: &str) -> ContextualFact {
    ContextualFact {
      id: Some(MeaningId::from(format!("fact.test.{subj}.{pred}"))),
      context: ContextId::from("test"),
      layer: LayerId::from("L1"),
      subj: subj.to_string(),
      pred: pred.to_string(),
      obj: obj.to_string(),
      status: MeaningStatus::Candidate,
      confidence: 0.9,
      provenance_refs: vec![],
      proof_refs: vec![],
      contradiction_refs: vec![],
      loss: None,
      timestamp: None,
    }
  }

  fn test_make_fact(
    id: String,
    context: &str,
    _layer: &str,
    pred: &str,
    obj: String,
    _provenance: Vec<String>,
  ) -> ContextualFact {
    ContextualFact {
      id: Some(MeaningId::from(id)),
      context: ContextId::from(context),
      layer: LayerId::from("doghouse"),
      subj: "ontology-decision".to_string(),
      pred: pred.to_string(),
      obj,
      status: MeaningStatus::Candidate,
      confidence: 1.0,
      provenance_refs: vec![],
      proof_refs: vec![],
      contradiction_refs: vec![],
      loss: None,
      timestamp: None,
    }
  }

  #[test]
  fn ontology_query_decision_produces_result_with_default_spec() {
    let facts = vec![
      test_fact("힘", "definition-ko", "물체의 운동 상태를 변화시키는 원인"),
      test_fact("힘", "formula", "F=ma"),
    ];
    let interps = vec![
      interpretation_with_refs(
        "interp.test.direct",
        vec!["fact.test.힘.definition-ko".to_string()],
      ),
      interpretation_with_refs(
        "interp.test.rich",
        vec![
          "fact.test.힘.definition-ko".to_string(),
          "fact.test.힘.formula".to_string(),
        ],
      ),
    ];
    let spec = QueryRouteSpec {
      query_context: "Test.Physics".to_string(),
      ..Default::default()
    };
    let result = ontology_query_decision("test-route", &spec, interps, &facts, None);
    assert!(result.is_some());
    let decision = result.unwrap();
    assert!(!decision.evaluations.is_empty());
    assert!(decision.selection.evaluation.score > 0.0);
  }

  #[test]
  fn ontology_query_lift_preserves_intent_label_and_scope_notes() {
    let facts = vec![test_fact("힘", "definition-ko", "test")];
    let spec = QueryRouteSpec {
      query_context: "Test.Physics".to_string(),
      ..Default::default()
    };
    let intent = JudgementIntent::with_intent_type(OutputScope::Detailed, "Definition");
    let (lift, _) = ontology_query_lift("test-route", &spec, &facts, Some(&intent));
    let notes = lift
      .loss
      .as_ref()
      .map(|loss| loss.notes.as_slice())
      .unwrap_or(&[]);
    assert!(notes.iter().any(|note| note == "intent:Definition"));
    assert!(notes.iter().any(|note| note == "scope:Detailed"));
  }

  #[test]
  fn append_decision_creates_facts_and_notes() {
    let facts = vec![test_fact("힘", "definition-ko", "test")];
    let interps = vec![interpretation_with_refs(
      "interp.test",
      vec!["fact.test.힘.definition-ko".to_string()],
    )];
    let spec = QueryRouteSpec {
      query_context: "Test".to_string(),
      ..Default::default()
    };
    let decision = ontology_query_decision("test", &spec, interps, &facts, None).unwrap();
    let mut out_facts = Vec::new();
    let mut out_notes = Vec::new();
    let mut events = DecisionEvents::default();
    append_ontology_query_decision(
      &mut out_facts,
      &mut out_notes,
      &["source:test".to_string()],
      "test",
      &decision,
      Some(&mut events),
      test_make_fact,
    );
    assert!(!out_facts.is_empty());
    assert!(!out_notes.is_empty());
    assert_eq!(events.pending.len(), 1);
    assert_eq!(events.pending[0].route, "test");
  }

  #[test]
  fn decision_events_materialize_with_actual_seq() {
    let facts = vec![test_fact("x", "pred", "val")];
    let interps = vec![interpretation_with_refs(
      "interp.x",
      vec!["fact.test.x.pred".to_string()],
    )];
    let spec = QueryRouteSpec::default();
    let decision = ontology_query_decision("route", &spec, interps, &facts, None).unwrap();
    let mut events = DecisionEvents::default();
    events.push_decision("route", &decision, &["source:test".to_string()]);
    let (je, pe) = events.into_protocol_events("jr.integration.1", 42);
    assert_eq!(je.len(), 1);
    assert_eq!(pe.len(), 1);
    assert_eq!(je[0].seq, 42);
    assert!(je[0].id.0.contains("42"));
    assert!(je[0].id.0.contains("route"));
  }
}

// ===========================================================================
// Search Judgment: 검색 결과 판단 (emergent search에서 추출)
// doghouse에는 ureq/HTTP + snippet 수집만 남기고, 판단 로직은 여기에.
// ===========================================================================

// ===========================================================================
// Peer Judgment: P2P peer evidence 판단 (Phase 7 P3)
// ===========================================================================

use crate::judgement_protocol::PeerEvidence;

/// peer heartbeat를 판단하는 최소 함수.
/// PeerEvidence → ContextualFact(Candidate) lift → evaluate → select → promote.
pub fn judge_peer_evidence(evidence: &PeerEvidence) -> OntologyQueryDecision {
  let fact = evidence.to_contextual_fact();
  let facts = vec![fact];
  let interp = interpretation_with_refs(
    format!("interp.peer.{}", evidence.peer_id),
    facts
      .iter()
      .filter_map(|f| f.id.as_ref().map(|id| id.0.clone()))
      .collect(),
  );

  let policy = crate::ontology::EvaluationPolicy {
    id: "peer-evidence".into(),
    accept_threshold: 0.5,
    hold_threshold: 0.2,
    ..Default::default()
  };

  let route_spec = QueryRouteSpec {
    query_context: "P2P.PeerEvidence".to_string(),
    ..Default::default()
  };

  let (lift, lifted_facts) = ontology_query_lift("peer-heartbeat", &route_spec, &facts, None);

  let lifted_interps = vec![{
    let mut i = interp;
    i.lift_refs.push(lift.id.clone());
    i
  }];

  let evaluations = lifted_interps
    .iter()
    .map(|i| ontology_evaluate(&policy, i, &lifted_facts))
    .collect::<Vec<_>>();

  let selection = ontology_select(&policy, &lifted_interps, &lifted_facts).unwrap_or_else(|| {
    crate::ontology::SelectionOutcome {
      evaluation: evaluations[0].clone(),
      judgement: crate::ontology::JudgementRecord {
        id: format!("judge.peer.{}.fallback", evidence.peer_id),
        evaluation: evaluations[0].id.clone(),
        action: crate::ontology::JudgementAction::Hold,
        chosen_interpretation: Some(lifted_interps[0].id.clone()),
        chosen_fact_refs: vec![],
        notes: vec!["peer-evidence-fallback".to_string()],
      },
    }
  });

  // OWNER-LAW (2026-05-10): peer heartbeat is external evidence — the remote
  // peer's owner-law is not under this substrate's control. lane-gate so
  // `Accept` becomes `Candidate`, not `Accepted`. Promotion to `Accepted`
  // requires owner-law proof + replay + provenance + negative/Held proof on
  // *this* substrate.
  let promotion =
    ontology_promote_with_lane(&policy, EvidenceLane::PeerEvidence, &selection.judgement);

  OntologyQueryDecision {
    lift,
    lifted_facts,
    evaluations,
    selection,
    promotion,
  }
}

#[cfg(test)]
mod peer_judgment_tests {
  use super::*;
  use crate::judgement_protocol::PeerStatus;

  #[test]
  fn peer_evidence_judgement_lifts_and_promotes_candidate_fact() {
    let evidence = PeerEvidence {
      peer_id: "node-alpha".to_string(),
      capabilities: vec!["physics".to_string()],
      latency_ms: Some(12),
      status: PeerStatus::Available,
      observed_at: Some("2026-04-11T12:00:00Z".to_string()),
    };

    let decision = judge_peer_evidence(&evidence);
    assert_eq!(decision.lift.to_context.0, "P2P.PeerEvidence");
    assert_eq!(decision.lifted_facts.len(), 1);
    assert_eq!(
      decision.lifted_facts[0].status,
      crate::ontology::MeaningStatus::Candidate
    );
    assert_eq!(decision.lifted_facts[0].subj, "node-alpha");
    assert_eq!(decision.lifted_facts[0].pred, "peer-status");
    assert_eq!(decision.lifted_facts[0].obj, "available");
    assert!(decision.lifted_facts[0]
      .provenance_refs
      .iter()
      .any(|p| p == "capability:physics"));
    assert_eq!(
      decision.promotion.judgement,
      decision.selection.judgement.id
    );
  }
}

// ===========================================================================
// Search Judgment: 검색 결과 판단 (emergent search에서 추출)
// ===========================================================================

// ===========================================================================
// Evidence pipeline (OWNER-LAW 2026-05-11):
//
// pnix is an LLM-independent deterministic AI substrate (CLAUDE.md OWNER-LAW
// CONSTITUTION). External web search is a Held-reopen *evidence sensor*, not
// an answer engine. The substrate must lift external prose through this
// staged pipeline rather than treating a snippet as a fact:
//
//   SearchSnippet  (raw search result; legacy/short carrier)
//     -> EvidencePointer  (which URL, which query, when retrieved)
//     -> EvidencePassage  (extracted passage + source-kind + trust)
//     -> EvidenceFact     (subj/pred/obj + polarity + provenance)
//     -> RequiredFact coverage check
//     -> ontology_query_decision_with_lane(.., ExternalWebSearch)
//
// Each step preserves provenance and is deterministic given the input. The
// ontology gate (`EvidenceLane::ExternalWebSearch`) downgrades any `Accept`
// to `Candidate`; promotion to `Accepted` requires owner-law proof.
// ===========================================================================

/// Source-kind classifier for an evidence passage.
///
/// OWNER-LAW: external prose is untrusted; classifying the source kind is
/// the first step toward replayable trust. `Blocked` keeps legally denied
/// source families at trust 0.0 before any evidence can be promoted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
  /// Source family denied by legal/provenance policy.
  Blocked,
  /// Encyclopedia entry whose terms are handled outside the denied family.
  Encyclopedia,
  /// Academic / preprint (arXiv, PubMed, ACM, IEEE, journal page).
  Academic,
  /// Standards / specification body (NIST, ISO, W3C, RFC).
  Standard,
  /// Vendor / product documentation (Rust docs, MDN, official APIs).
  VendorDoc,
  /// News / press release.
  News,
  /// Forum / community / Q&A (StackExchange, Reddit, mailing list).
  Community,
  /// Personal blog / unattributed page.
  Blog,
  /// Unknown / unclassified source.
  Unknown,
}

/// Polarity of an evidence fact relative to the question being asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Polarity {
  /// Evidence supports the proposition.
  Supports,
  /// Evidence contradicts the proposition.
  Contradicts,
  /// Evidence is on-topic but does not directly support or contradict.
  Irrelevant,
}

/// Pointer to an evidence-bearing source (one URL, one snippet, one query).
///
/// This is *not* a fact yet. It only says "evidence may exist at this URL."
/// The next stage (`EvidencePassage`) extracts the actual passage; the stage
/// after that (`EvidenceFact`) lowers the passage into an ontology fact.
#[derive(Debug, Clone)]
pub struct EvidencePointer {
  /// Source URL.
  pub source_url: String,
  /// Snippet shown by the search engine.
  pub snippet: String,
  /// The query that produced this pointer (for replay).
  pub query_ref: String,
  /// Retrieval timestamp (deterministic input — caller-supplied).
  pub retrieved_at: String,
  /// Heuristic source-kind guess from URL/title.
  pub source_kind_guess: SourceKind,
}

/// Passage extracted from an evidence pointer (post-fetch, pre-relation-lift).
#[derive(Debug, Clone)]
pub struct EvidencePassage {
  /// Source URL.
  pub source_url: String,
  /// The actual passage text (bounded length).
  pub passage: String,
  /// Source title (page heading).
  pub title: String,
  /// Confirmed source kind after fetching.
  pub source_kind: SourceKind,
  /// Retrieval timestamp.
  pub retrieved_at: String,
}

/// Evidence fact lowered from a passage (subj/pred/obj + polarity + trust).
///
/// OWNER-LAW: never auto-Accepted. The `ExternalWebSearch` lane in
/// `ontology_promote_with_lane` clamps `Accept` -> `Candidate`. Accepted
/// promotion requires owner-law proof.
#[derive(Debug, Clone)]
pub struct EvidenceFact {
  pub subj: String,
  pub pred: String,
  pub obj: String,
  pub polarity: Polarity,
  pub source_url: String,
  pub source_kind: SourceKind,
  /// Trust prior in [0.0, 1.0]. Computed deterministically from
  /// `source_kind` + retrieval freshness + caller policy.
  pub trust: f64,
  /// The passage this fact was extracted from (for replay).
  pub passage: String,
  /// Provenance refs (URLs, prior facts, etc.).
  pub provenance_refs: Vec<String>,
  /// Optional reference to a `RequiredFact` this evidence covers.
  pub covers_required_fact: Option<String>,
}

/// What kind of fact the substrate needs to close a Held question.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactNeed {
  /// Need evidence supporting a candidate answer.
  Support,
  /// Need evidence refuting a candidate answer (negative-Held proof).
  Refute,
  /// Need evidence clarifying ambiguous context.
  Clarify,
}

/// A specific fact the substrate needs to find. Built from a `HeldPart`.
#[derive(Debug, Clone)]
pub struct RequiredFact {
  /// Stable id for this requirement (used by `EvidenceFact.covers_required_fact`).
  pub id: String,
  pub subj: String,
  pub pred: String,
  pub obj: String,
  pub kind: FactNeed,
  /// Optional choice label (e.g., "A" / "B" for multiple-choice).
  pub for_choice: Option<String>,
}

/// One held sub-question pulled out of an utterance + Held judgement.
#[derive(Debug, Clone)]
pub struct HeldPart {
  /// Subject the Held is about.
  pub subject: String,
  /// Predicate that is missing or ambiguous.
  pub missing_predicate: Option<String>,
  /// Context that needs narrowing (e.g., "double-slit experiment").
  pub missing_context: Option<String>,
  /// The required fact this part demands, if known.
  pub required_fact: Option<RequiredFact>,
}

/// A search plan derived from one or more `RequiredFact`s.
#[derive(Debug, Clone)]
pub struct SearchPlan {
  /// Queries to run (replayable).
  pub queries: Vec<SearchQuery>,
  /// Required facts this plan is supposed to cover.
  pub required_facts: Vec<RequiredFact>,
  /// Source-trust policy (which `SourceKind` to allow / require).
  pub source_policy: SourceTrustPolicy,
  /// Maximum reopen depth before the substrate gives up and stays Held.
  pub max_reopen_depth: usize,
}

/// Source-trust policy for a search plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceTrustPolicy {
  /// Strict: only `Encyclopedia`/`Academic`/`Standard`/`VendorDoc`.
  Strict,
  /// Normal: also allow `News`.
  Normal,
  /// Relaxed: also allow `Community`/`Blog` (lower trust prior).
  Relaxed,
}

/// State of the Held -> Reopen loop. Each iteration accumulates evidence.
#[derive(Debug, Clone)]
pub struct HeldReopenState {
  /// Reopen depth so far.
  pub depth: usize,
  /// Required facts still missing evidence.
  pub missing: Vec<RequiredFact>,
  /// Evidence collected during this reopen sequence.
  pub evidence: Vec<EvidenceFact>,
  /// Previous queries already run (for de-duplication).
  pub previous_queries: Vec<String>,
}

impl HeldReopenState {
  /// Initial state — depth 0, no evidence yet.
  pub fn new(missing: Vec<RequiredFact>) -> Self {
    Self {
      depth: 0,
      missing,
      evidence: Vec::new(),
      previous_queries: Vec::new(),
    }
  }

  /// Whether the loop has met its budget.
  pub fn is_exhausted(&self, budget: usize) -> bool {
    self.depth >= budget
  }

  /// Whether all `RequiredFact`s have at least one supporting evidence.
  pub fn is_covered(&self) -> bool {
    self.missing.iter().all(|need| {
      self.evidence.iter().any(|ev| {
        ev.covers_required_fact.as_deref() == Some(need.id.as_str())
          && matches!(ev.polarity, Polarity::Supports)
      })
    })
  }
}

/// Heuristic source-kind classifier from a URL + title.
///
/// Pure data, deterministic. Caller may override after fetching.
pub fn classify_source_kind(url: &str, title: &str) -> SourceKind {
  let u = url.to_ascii_lowercase();
  let t = title.to_ascii_lowercase();
  if u.contains("wikipedia.org") || u.contains("namu.wiki") {
    return SourceKind::Blocked;
  }
  if u.contains("britannica.com") {
    return SourceKind::Encyclopedia;
  }
  if u.contains("arxiv.org")
    || u.contains("pubmed")
    || u.contains("acm.org")
    || u.contains("ieee.org")
    || u.contains("doi.org")
    || u.contains("nature.com")
    || u.contains("science.org")
  {
    return SourceKind::Academic;
  }
  if u.contains("nist.gov")
    || u.contains("iso.org")
    || u.contains("w3.org")
    || u.contains("ietf.org")
    || u.contains("rfc-editor.org")
  {
    return SourceKind::Standard;
  }
  if u.contains("docs.rs")
    || u.contains("developer.mozilla.org")
    || u.contains("doc.rust-lang.org")
    || u.contains("docs.python.org")
  {
    return SourceKind::VendorDoc;
  }
  if u.contains("stackoverflow.com")
    || u.contains("stackexchange.com")
    || u.contains("reddit.com")
    || t.contains("forum")
  {
    return SourceKind::Community;
  }
  if u.contains("blog.") || u.contains("medium.com") || u.contains("substack.com") {
    return SourceKind::Blog;
  }
  if t.contains("news") || u.contains("news") {
    return SourceKind::News;
  }
  SourceKind::Unknown
}

/// Trust prior for a `SourceKind` under a `SourceTrustPolicy`.
///
/// Returns 0.0 when the policy forbids the kind. Otherwise a deterministic
/// prior in (0, 1] that the caller multiplies into evidence confidence.
pub fn source_kind_trust_prior(kind: SourceKind, policy: SourceTrustPolicy) -> f64 {
  use SourceKind::*;
  match (policy, kind) {
    (_, Blocked) => 0.0,

    (SourceTrustPolicy::Strict, Encyclopedia) => 0.85,
    (SourceTrustPolicy::Strict, Academic) => 0.95,
    (SourceTrustPolicy::Strict, Standard) => 0.95,
    (SourceTrustPolicy::Strict, VendorDoc) => 0.85,
    (SourceTrustPolicy::Strict, _) => 0.0,

    (SourceTrustPolicy::Normal, Encyclopedia) => 0.85,
    (SourceTrustPolicy::Normal, Academic) => 0.95,
    (SourceTrustPolicy::Normal, Standard) => 0.95,
    (SourceTrustPolicy::Normal, VendorDoc) => 0.85,
    (SourceTrustPolicy::Normal, News) => 0.55,
    (SourceTrustPolicy::Normal, Community) => 0.0,
    (SourceTrustPolicy::Normal, Blog) => 0.0,
    (SourceTrustPolicy::Normal, Unknown) => 0.0,

    (SourceTrustPolicy::Relaxed, Encyclopedia) => 0.85,
    (SourceTrustPolicy::Relaxed, Academic) => 0.95,
    (SourceTrustPolicy::Relaxed, Standard) => 0.95,
    (SourceTrustPolicy::Relaxed, VendorDoc) => 0.85,
    (SourceTrustPolicy::Relaxed, News) => 0.55,
    (SourceTrustPolicy::Relaxed, Community) => 0.40,
    (SourceTrustPolicy::Relaxed, Blog) => 0.30,
    (SourceTrustPolicy::Relaxed, Unknown) => 0.20,
  }
}

/// Lift a legacy `SearchSnippet` into an `EvidencePointer`.
///
/// Pure data; no IO. The caller (search adapter) fetches the source page
/// to produce an `EvidencePassage`.
pub fn evidence_pointer_from_snippet(
  snippet: &SearchSnippet,
  query_ref: &str,
  retrieved_at: &str,
) -> EvidencePointer {
  EvidencePointer {
    source_url: snippet.url.clone(),
    snippet: snippet.text.clone(),
    query_ref: query_ref.to_string(),
    retrieved_at: retrieved_at.to_string(),
    source_kind_guess: classify_source_kind(&snippet.url, &snippet.title),
  }
}

// ===========================================================================
// EvidenceFact lowering + lane-aware judge (OWNER-LAW 2026-05-11):
// the actual loop closing the carrier types defined above.
// ===========================================================================

/// Lift an `EvidencePassage` into an `EvidenceFact` against a question.
///
/// OWNER-LAW: this is the deterministic relation lowering step. The current
/// implementation is the minimal slice — it treats the whole passage as one
/// fact whose subject is the utterance's content tokens, predicate is
/// `"evidence-found-in"`, and object is the source kind. Polarity defaults
/// to `Supports` because a returned passage is at-best on-topic; the
/// promotion gate still keeps it as `Candidate` (external lane). Future
/// versions parse the passage into multiple subj/pred/obj triples.
pub fn evidence_fact_from_passage(
  passage: &EvidencePassage,
  utterance: &str,
  policy: SourceTrustPolicy,
  required: Option<&RequiredFact>,
) -> EvidenceFact {
  let trust = source_kind_trust_prior(passage.source_kind, policy);
  let (subj, pred, obj) = match required {
    Some(r) => (r.subj.clone(), r.pred.clone(), r.obj.clone()),
    None => (
      utterance.trim().to_string(),
      "evidence-found-in".to_string(),
      format!("{:?}", passage.source_kind),
    ),
  };
  let provenance_refs = vec![
    passage.source_url.clone(),
    format!("retrieved-at:{}", passage.retrieved_at),
    format!("title:{}", passage.title),
  ];
  EvidenceFact {
    subj,
    pred,
    obj,
    polarity: Polarity::Supports,
    source_url: passage.source_url.clone(),
    source_kind: passage.source_kind,
    trust,
    passage: passage.passage.clone(),
    provenance_refs,
    covers_required_fact: required.map(|r| r.id.clone()),
  }
}

/// Lower an `EvidenceFact` to a `ContextualFact(Candidate)`.
///
/// OWNER-LAW: external evidence enters the substrate as `Candidate` only.
/// Promotion to `Accepted` requires `ontology_promote_with_lane` with an
/// `InternalOwnerLaw` lane, which itself requires owner-law proof + replay
/// + negative/Held proof — so this helper deliberately hard-codes
/// `MeaningStatus::Candidate` regardless of `EvidenceFact.polarity`.
pub fn evidence_fact_to_contextual_fact(
  fact: &EvidenceFact,
  context: ContextId,
  layer: crate::ontology::LayerId,
) -> ContextualFact {
  use crate::ontology::MeaningStatus;
  // Confidence = trust prior; the ontology evaluator may further dampen
  // based on coherence / replayability axes. Polarity is preserved in
  // provenance so a downstream contradiction detector can act on it.
  let mut provenance_refs = fact.provenance_refs.clone();
  provenance_refs.push(format!("polarity:{:?}", fact.polarity));
  if let Some(ref need) = fact.covers_required_fact {
    provenance_refs.push(format!("covers-required-fact:{}", need));
  }
  ContextualFact {
    id: None,
    context,
    layer,
    subj: fact.subj.clone(),
    pred: fact.pred.clone(),
    obj: fact.obj.clone(),
    status: MeaningStatus::Candidate,
    confidence: fact.trust,
    provenance_refs,
    proof_refs: Vec::new(),
    contradiction_refs: Vec::new(),
    loss: None,
    timestamp: None,
  }
}

/// EvidenceFact-aware judgement: lower a list of `EvidenceFact`s into
/// `ContextualFact(Candidate)`s and run the lane-aware ontology decision.
///
/// OWNER-LAW (2026-05-11): this is the canonical owner-law-aligned judge
/// for external web search results. It uses `EvidenceLane::ExternalWebSearch`
/// so any `Accept` outcome auto-downgrades to `Candidate` — the promotion
/// to `Accepted` from this lane requires a separate owner-law gate call.
pub fn judge_evidence_facts(
  utterance: &str,
  facts: &[EvidenceFact],
) -> Option<OntologyQueryDecision> {
  use crate::ontology::{ContextId, LayerId, MeaningId};
  if facts.is_empty() {
    return None;
  }
  let context = ContextId::from("Search.Open");
  let layer = LayerId::from("L4");
  // OWNER-LAW (2026-05-11): every evidence ContextualFact gets a stable
  // `MeaningId` so interpretation `fact_refs` and downstream
  // ontology evaluation / audit / replay can name the exact fact each
  // interpretation rests on. Without ids, `fact_refs` would be empty and
  // the lane-aware judge would degrade silently.
  let cfacts: Vec<ContextualFact> = facts
    .iter()
    .enumerate()
    .map(|(i, f)| {
      let mut cf = evidence_fact_to_contextual_fact(f, context.clone(), layer.clone());
      let id_str = format!("fact.search.evidence.{}.{}", sanitize_id_token(&f.subj), i);
      cf.id = Some(MeaningId::from(id_str));
      cf
    })
    .collect();
  // Build one interpretation per evidence fact so the selector can score
  // them against each other (deterministic tie-break in pnix-core). Each
  // interpretation references *its own* fact id so the selector can run a
  // real per-fact comparison rather than scoring identical fact bundles.
  let interpretations: Vec<Interpretation> = cfacts
    .iter()
    .enumerate()
    .map(|(i, cf)| {
      let interp_id = format!("interp.evidence.{i}");
      let fact_refs: Vec<String> = cf
        .id
        .as_ref()
        .map(|m| vec![m.0.clone()])
        .unwrap_or_default();
      interpretation_with_refs(interp_id, fact_refs)
    })
    .collect();
  let route_spec = QueryRouteSpec {
    query_context: "Search.Open".to_string(),
    ..Default::default()
  };
  ontology_query_decision_with_lane(
    utterance,
    &route_spec,
    interpretations,
    &cfacts,
    None,
    EvidenceLane::ExternalWebSearch,
  )
}

/// Lower a passage to evidence facts covering *every* `RequiredFact`.
///
/// OWNER-LAW (2026-05-11): when a `SearchPlan` has multiple required
/// facts (e.g. compound questions), the substrate must produce one
/// `EvidenceFact` per `(passage, required)` pair so coverage is computed
/// per-required, not just for the first one. Trust prior is shared per
/// passage; downstream audit can dedupe by source_url + covers_required_fact.
pub fn evidence_facts_from_passage_for_all_required(
  passage: &EvidencePassage,
  utterance: &str,
  policy: SourceTrustPolicy,
  required: &[RequiredFact],
) -> Vec<EvidenceFact> {
  if required.is_empty() {
    return vec![evidence_fact_from_passage(passage, utterance, policy, None)];
  }
  required
    .iter()
    .map(|r| evidence_fact_from_passage(passage, utterance, policy, Some(r)))
    .collect()
}

// ===========================================================================
// Connection: relation extraction (SimpleExtractor) + ExpressionProjection
// canonical-text generation (NLG). Owner-law: `pnix-core` only owns
// deterministic helpers — domain-specific extraction rules live in `.px`.
// ===========================================================================

/// Lift a passage to evidence facts using the substrate's deterministic
/// `SimpleExtractor` for noun / verb extraction, instead of a single broad
/// `pred="answer"` fact.
///
/// OWNER-LAW (2026-05-11): this is the relation-extraction connection from
/// the existing NLP carrier into the evidence pipeline. For each
/// (passage, required) pair we extract Korean / English noun-verb pairs
/// from the passage and emit one `EvidenceFact` per (noun, verb) tuple
/// matched against the required fact. If extraction yields nothing,
/// falls back to the broad-fact constructor so coverage is never empty.
pub fn evidence_facts_from_passage_extracted(
  passage: &EvidencePassage,
  utterance: &str,
  policy: SourceTrustPolicy,
  required: Option<&RequiredFact>,
) -> Vec<EvidenceFact> {
  use crate::nlp::schema_mapper::{NounExtractor, SimpleExtractor, VerbExtractor};

  let extractor = SimpleExtractor::default();
  let nouns = extractor.extract_nouns(&passage.passage);
  let verbs = extractor.extract_verbs(&passage.passage);

  if nouns.is_empty() && verbs.is_empty() {
    return vec![evidence_fact_from_passage(
      passage, utterance, policy, required,
    )];
  }

  let trust = source_kind_trust_prior(passage.source_kind, policy);
  let mut out = Vec::new();
  let provenance_refs_base = vec![
    passage.source_url.clone(),
    format!("retrieved-at:{}", passage.retrieved_at),
    format!("title:{}", passage.title),
    "extractor:SimpleExtractor".to_string(),
  ];

  // OWNER-LAW (2026-05-11): consult relation-extraction owner-law carriers
  // *before* emitting EvidenceFact triples. Each carrier mirrors the
  // matching `.px` owner-law file; the override records
  // `owner-law:<.px path>` + `marker:<text>` in provenance_refs so replay
  // can land on the canonical `.px` owner.
  //
  // Pred precedence (when multiple match): comparison > definition >
  // formula > causality > verb-default.
  //   - comparison: pred = "greater-than" / "less-than" / ...
  //   - definition: pred = "is-defined-as"
  //   - formula:    pred = "formula"     + dimension-check hint
  //   - causality:  pred = "causes"      + chain-eligible hint
  //
  // Negation is orthogonal — it flips polarity to Contradicts regardless
  // of which pred override (if any) is active.
  let negation = crate::nlp::relation_classifier::classify_negation(&passage.passage);
  let comparison = crate::nlp::relation_classifier::classify_comparison(&passage.passage);
  let definition = if comparison.is_none() && negation.is_none() {
    crate::nlp::relation_classifier::classify_definition(&passage.passage)
  } else {
    None
  };
  let formula = if comparison.is_none() && definition.is_none() {
    crate::nlp::relation_classifier::classify_formula(&passage.passage)
  } else {
    None
  };
  let causality = if comparison.is_none() && definition.is_none() && formula.is_none() {
    crate::nlp::relation_classifier::classify_causality(&passage.passage)
  } else {
    None
  };
  let base_polarity = if negation.is_some() {
    Polarity::Contradicts
  } else {
    Polarity::Supports
  };

  // Strategy: pair (verb, subject_noun, object_noun) when both are found.
  // Predicate is the verb's normalized form (or "is" if no verb), subj is
  // the first content noun, obj is the second noun (or the verb's object
  // when given). When `required` is set, the produced fact also carries
  // `covers_required_fact` so coverage is real.
  if !verbs.is_empty() {
    for (i, verb) in verbs.iter().enumerate() {
      let subj_noun = verb
        .subject
        .as_ref()
        .or_else(|| nouns.first().map(|n| &n.normalized))
        .cloned()
        .unwrap_or_else(|| utterance.trim().to_string());
      let obj_noun = verb
        .object
        .as_ref()
        .cloned()
        .or_else(|| {
          nouns
            .iter()
            .find(|n| n.normalized != subj_noun)
            .map(|n| n.normalized.clone())
        })
        .unwrap_or_else(|| format!("{:?}", passage.source_kind));
      let mut provenance_refs = provenance_refs_base.clone();
      provenance_refs.push(format!("relation-pair-index:{i}"));
      let pred = if let Some((kind, hit)) = comparison.as_ref() {
        provenance_refs.push(format!("owner-law:{}", hit.owner_law));
        provenance_refs.push(format!("relation-marker:{}", hit.matched_marker));
        kind.predicate().to_string()
      } else if let Some(hit) = definition.as_ref() {
        provenance_refs.push(format!("owner-law:{}", hit.owner_law));
        provenance_refs.push(format!("relation-marker:{}", hit.matched_marker));
        "is-defined-as".to_string()
      } else if let Some(hit) = formula.as_ref() {
        provenance_refs.push(format!("owner-law:{}", hit.owner_law));
        provenance_refs.push(format!("relation-marker:{}", hit.matched_marker));
        provenance_refs.push("formula-dimension-check-required".to_string());
        // OWNER-LAW (2026-05-11): if the substrate's dimension
        // inference engine can decide on this passage (split lhs/rhs
        // at `=`, look up symbols, compare MLT vectors), emit the
        // result marker into provenance so
        // `resolve_formula_dimension_check` can act on it. When the
        // passage doesn't contain a parseable formula equation, the
        // engine returns `None` and we just leave the required marker
        // standing (gate verdict will be `Missing`).
        if let Some(res) =
          crate::nlp::formula_dimension_inference::infer_passage_dimension_check(&passage.passage)
        {
          if let Some(marker) = crate::nlp::formula_dimension_inference::dimension_check_marker(res)
          {
            provenance_refs.push(marker.to_string());
          }
        }
        "formula".to_string()
      } else if let Some(hit) = causality.as_ref() {
        provenance_refs.push(format!("owner-law:{}", hit.owner_law));
        provenance_refs.push(format!("relation-marker:{}", hit.matched_marker));
        provenance_refs.push("chain-eligible:true".to_string());
        "causes".to_string()
      } else {
        verb.normalized.clone()
      };
      if let Some(hit) = negation.as_ref() {
        provenance_refs.push(format!("owner-law:{}", hit.owner_law));
        provenance_refs.push(format!("relation-marker:{}", hit.matched_marker));
      }
      out.push(EvidenceFact {
        subj: subj_noun,
        pred,
        obj: obj_noun,
        polarity: base_polarity,
        source_url: passage.source_url.clone(),
        source_kind: passage.source_kind,
        trust,
        passage: passage.passage.clone(),
        provenance_refs,
        covers_required_fact: required.map(|r| r.id.clone()),
      });
    }
  } else if let Some(noun) = nouns.first() {
    // No verb extracted. Default pred is "mentions" with Supports polarity,
    // but relation owner-laws (negation / comparison / definition) can
    // override pred and polarity even without a verb — they classify the
    // passage as a whole, not the verb-frame.
    let mut provenance_refs = provenance_refs_base.clone();
    provenance_refs.push("relation-pair-index:noun-only".to_string());
    let obj_noun = nouns
      .iter()
      .find(|n| n.normalized != noun.normalized)
      .map(|n| n.normalized.clone())
      .unwrap_or_else(|| utterance.trim().to_string());
    let pred = if let Some((kind, hit)) = comparison.as_ref() {
      provenance_refs.push(format!("owner-law:{}", hit.owner_law));
      provenance_refs.push(format!("relation-marker:{}", hit.matched_marker));
      kind.predicate().to_string()
    } else if let Some(hit) = definition.as_ref() {
      provenance_refs.push(format!("owner-law:{}", hit.owner_law));
      provenance_refs.push(format!("relation-marker:{}", hit.matched_marker));
      "is-defined-as".to_string()
    } else if let Some(hit) = formula.as_ref() {
      provenance_refs.push(format!("owner-law:{}", hit.owner_law));
      provenance_refs.push(format!("relation-marker:{}", hit.matched_marker));
      provenance_refs.push("formula-dimension-check-required".to_string());
      // OWNER-LAW (2026-05-11): same dimension inference wire-up as
      // the verb-branch above. See that branch for the rationale.
      if let Some(res) =
        crate::nlp::formula_dimension_inference::infer_passage_dimension_check(&passage.passage)
      {
        if let Some(marker) = crate::nlp::formula_dimension_inference::dimension_check_marker(res) {
          provenance_refs.push(marker.to_string());
        }
      }
      "formula".to_string()
    } else if let Some(hit) = causality.as_ref() {
      provenance_refs.push(format!("owner-law:{}", hit.owner_law));
      provenance_refs.push(format!("relation-marker:{}", hit.matched_marker));
      provenance_refs.push("chain-eligible:true".to_string());
      "causes".to_string()
    } else {
      "mentions".to_string()
    };
    if let Some(hit) = negation.as_ref() {
      provenance_refs.push(format!("owner-law:{}", hit.owner_law));
      provenance_refs.push(format!("relation-marker:{}", hit.matched_marker));
    }
    let final_obj =
      if comparison.is_some() || definition.is_some() || formula.is_some() || causality.is_some() {
        obj_noun
      } else {
        utterance.trim().to_string()
      };
    out.push(EvidenceFact {
      subj: noun.normalized.clone(),
      pred,
      obj: final_obj,
      polarity: base_polarity,
      source_url: passage.source_url.clone(),
      source_kind: passage.source_kind,
      trust,
      passage: passage.passage.clone(),
      provenance_refs,
      covers_required_fact: required.map(|r| r.id.clone()),
    });
  }

  if out.is_empty() {
    return vec![evidence_fact_from_passage(
      passage, utterance, policy, required,
    )];
  }
  out
}

/// Build an `ExpressionProjectionRecord` (canonical-text NLG) for an
/// evidence-aware ontology decision.
///
/// OWNER-LAW (2026-05-11): the substrate's *self-knowledge expression
/// surface*. Given the lane-aware judgement and accumulated evidence,
/// emit a `ExpressionProjectionRecord` with surface forms:
///
/// - `canonical-text`: a Korean sentence stating what the substrate
///   currently holds (Accepted / Candidate / Held / Contradicted), with
///   the lane name and a brief evidence summary.
/// - `mathml-content`: empty for now (filled by domain-specific generators).
/// - `openmath`: empty for now.
/// - `freecat-geometry`: empty for now.
///
/// The Korean sentence respects the meaning status:
///   Accepted → "확정적으로 X이다."
///   Candidate → "현재 외부 증거상 X일 가능성이 있으나 owner-law proof 전이라 Candidate다."
///   Held → "X에 대한 충분한 증거가 부족해 Held다. 다음 검색이 필요하다."
///   Contradicted → "현재 증거가 모순된다."
pub fn expression_projection_from_decision(
  decision: &OntologyQueryDecision,
  evidence: &[EvidenceFact],
) -> crate::ontology::ExpressionProjectionRecord {
  use crate::ontology::{ExpressionProjectionId, ExpressionProjectionRecord, MeaningStatus};

  let status = decision.promotion.target_status.clone();
  let lane_note = decision
    .promotion
    .reason
    .clone()
    .unwrap_or_else(|| "lane=unknown".to_string());

  // Build a brief subject snippet from the first evidence fact.
  let subject_summary = evidence
    .first()
    .map(|f| format!("{} {} {}", f.subj, f.pred, f.obj))
    .unwrap_or_else(|| "(no evidence)".to_string());

  let evidence_count = evidence.len();
  let source_count = evidence
    .iter()
    .map(|f| f.source_url.clone())
    .collect::<std::collections::BTreeSet<_>>()
    .len();

  let canonical_text = match status {
    MeaningStatus::Accepted => format!(
      "확정적으로 알고 있는 것: {} (evidence={}, sources={}, {})",
      subject_summary, evidence_count, source_count, lane_note
    ),
    MeaningStatus::Candidate => format!(
      "현재 외부 증거상 가능성: {} (evidence={}, sources={}, owner-law proof 전이라 Candidate. {})",
      subject_summary, evidence_count, source_count, lane_note
    ),
    MeaningStatus::Held => format!(
      "Held: {} 에 대한 충분한 증거가 부족함. 다음 검색이 필요. (evidence={}, sources={}, {})",
      subject_summary, evidence_count, source_count, lane_note
    ),
    MeaningStatus::Contradicted => format!(
      "Contradicted: 현재 증거가 모순됨. {} (evidence={}, sources={}, {})",
      subject_summary, evidence_count, source_count, lane_note
    ),
    MeaningStatus::Rejected => format!(
      "Rejected: {} (evidence={}, sources={}, {})",
      subject_summary, evidence_count, source_count, lane_note
    ),
    MeaningStatus::Deprecated => format!(
      "Deprecated: {} (이전 Accepted 였으나 지금은 owner-law deprecated)",
      subject_summary
    ),
    MeaningStatus::Deleted => format!("Deleted: {}", subject_summary),
  };

  let mut surface_forms = std::collections::BTreeMap::new();
  surface_forms.insert("canonical-text".to_string(), canonical_text.clone());
  surface_forms.insert("mathml-content".to_string(), String::new());
  surface_forms.insert("openmath".to_string(), String::new());
  surface_forms.insert("freecat-geometry".to_string(), String::new());

  let provenance_refs: Vec<String> = evidence
    .iter()
    .take(3)
    .map(|f| f.source_url.clone())
    .collect();

  ExpressionProjectionRecord {
    id: ExpressionProjectionId::from(format!("expr.search.{}", decision.selection.judgement.id)),
    context: crate::ontology::ContextId::from("Search.Open"),
    layer: crate::ontology::LayerId::from("L4"),
    subject: subject_summary,
    projection_family: "search-evidence-summary".to_string(),
    canonical_form: canonical_text,
    semantic_fact_refs: evidence
      .iter()
      .take(8)
      .map(|f| format!("{}|{}|{}", f.subj, f.pred, f.obj))
      .collect(),
    surface_forms,
    provenance_refs,
    artifact_refs: Vec::new(),
    notes: vec![format!(
      "promotion-status={:?} lane-note={}",
      status, lane_note
    )],
  }
}

/// Coverage check: which `RequiredFact`s have at least one supporting
/// `EvidenceFact` in the given collection.
pub fn required_facts_covered(needed: &[RequiredFact], evidence: &[EvidenceFact]) -> Vec<String> {
  needed
    .iter()
    .filter(|need| {
      evidence.iter().any(|ev| {
        ev.covers_required_fact.as_deref() == Some(need.id.as_str())
          && matches!(ev.polarity, Polarity::Supports)
      })
    })
    .map(|need| need.id.clone())
    .collect()
}

/// Extract `HeldPart`s from an utterance using the store's existing Held
/// facts as anchors.
///
/// OWNER-LAW: the substrate must know "what is missing" before it asks the
/// external sensor. This is the deterministic minimal extractor — it pulls
/// content tokens from the utterance and pairs them with the most-frequent
/// missing predicate observed in current Held facts (if the store provides
/// them). Concrete extractor logic owned by `.px` lives in stdlib;
/// callers that want richer extraction should pass an explicit list of
/// `HeldPart`s instead of calling this helper.
pub fn extract_held_parts_minimal(utterance: &str) -> Vec<HeldPart> {
  let trimmed = utterance.trim();
  if trimmed.is_empty() {
    return Vec::new();
  }
  // Minimal slice: one HeldPart per utterance with a concrete `RequiredFact`
  // so the Held-Reopen loop has a coverage target. Without a `RequiredFact`,
  // `HeldReopenState::is_covered` is vacuously true and the loop never runs.
  // The required fact uses `pred = "answer"` as the broad-question coverage
  // anchor; richer per-clause splits live in `.px` owner law.
  let cleaned = strip_search_particles(trimmed);
  let subject = if cleaned.is_empty() {
    trimmed.to_string()
  } else {
    cleaned
  };
  let need = RequiredFact {
    id: format!("need.{}.answer", sanitize_id_token(&subject)),
    subj: subject.clone(),
    pred: "answer".to_string(),
    obj: "?".to_string(),
    kind: FactNeed::Support,
    for_choice: None,
  };
  vec![HeldPart {
    subject: subject.clone(),
    missing_predicate: Some("answer".to_string()),
    missing_context: None,
    required_fact: Some(need),
  }]
}

/// Sanitize a string into a stable id token (alphanumeric + dashes).
fn sanitize_id_token(s: &str) -> String {
  let mut out = String::with_capacity(s.len());
  let mut last_dash = false;
  for c in s.chars() {
    if c.is_ascii_alphanumeric() || c.is_alphabetic() {
      out.push(c);
      last_dash = false;
    } else if !last_dash && !out.is_empty() {
      out.push('-');
      last_dash = true;
    }
  }
  while out.ends_with('-') {
    out.pop();
  }
  if out.is_empty() {
    "subject".to_string()
  } else {
    out
  }
}

/// Reformulate `RequiredFact`s into new `SearchQuery`s, deduplicating
/// against the previously-tried queries. Used by the Held-Reopen loop
/// to drive the next iteration when current evidence does not cover.
///
/// The reformulation strategy is deterministic and broad: prepend the
/// required predicate and append a definition / explanation marker so a
/// general-purpose search engine returns more focused results. Concrete
/// per-domain reformulation lives in `.px` owner law (future slice).
pub fn reformulate_queries_for_missing(
  missing: &[RequiredFact],
  previous: &[String],
) -> Vec<SearchQuery> {
  let mut out = Vec::new();
  let prev_set: std::collections::HashSet<String> = previous.iter().cloned().collect();
  for (i, need) in missing.iter().enumerate() {
    let bases = [
      format!("{} {} 정의", need.subj, need.pred),
      format!("{} {} 설명", need.subj, need.pred),
      format!("{} 뜻", need.subj),
    ];
    for (j, text) in bases.iter().enumerate() {
      if prev_set.contains(text) {
        continue;
      }
      out.push(SearchQuery {
        text: text.clone(),
        perspective: format!("reopen-{i}-{j}"),
        weight: 0.85,
      });
      // Cap at one reformulation per missing fact per round to avoid
      // combinatorial explosion; the loop iterates depth-bounded.
      break;
    }
  }
  out
}

/// Build a minimal `SearchPlan` from `HeldPart`s.
///
/// Each part contributes one query (using its missing_predicate + subject).
/// Missing context is appended when present. Caller may add follow-up
/// queries during the reopen loop.
pub fn search_plan_from_held_parts(parts: &[HeldPart], policy: SourceTrustPolicy) -> SearchPlan {
  let mut queries = Vec::new();
  let mut required_facts = Vec::new();
  for (idx, part) in parts.iter().enumerate() {
    let mut q = part.subject.clone();
    if let Some(ref pred) = part.missing_predicate {
      q.push(' ');
      q.push_str(pred);
    }
    if let Some(ref ctx) = part.missing_context {
      q.push(' ');
      q.push_str(ctx);
    }
    queries.push(SearchQuery {
      text: q,
      perspective: format!("held-part-{idx}"),
      weight: 1.0,
    });
    if let Some(ref need) = part.required_fact {
      required_facts.push(need.clone());
    }
  }
  SearchPlan {
    queries,
    required_facts,
    source_policy: policy,
    max_reopen_depth: 3,
  }
}

// ===========================================================================
// Legacy SearchSnippet (carrier for the open-book search path).
// New code: prefer the EvidencePointer / EvidencePassage / EvidenceFact
// pipeline above. SearchSnippet remains as the raw transport from a search
// adapter.
// ===========================================================================

/// 검색 스니펫 (pure data, IO 없음).
#[derive(Debug, Clone)]
pub struct SearchSnippet {
  pub title: String,
  pub text: String,
  pub url: String,
}

/// 검색 질의 (pure data).
#[derive(Debug, Clone)]
pub struct SearchQuery {
  pub text: String,
  pub perspective: String,
  pub weight: f64,
}

/// 검색 판단 결과.
#[derive(Debug, Clone)]
pub struct SearchAnswer {
  pub transcript: Vec<String>,
  pub sources: Vec<String>,
  pub confidence: f64,
  pub action: crate::ontology::JudgementAction,
  pub follow_up_hint: String,
}

/// 검색 쿼리 확장. 네트워크 IO 없이 순수하게 query plan만 만든다.
pub fn expand_search_queries(utterance: &str) -> Vec<SearchQuery> {
  let mut qs = Vec::new();
  let clean = strip_search_particles(utterance);

  qs.push(SearchQuery {
    text: clean.clone(),
    perspective: "original".into(),
    weight: 1.0,
  });

  let words = search_content_words(&clean);
  if words.len() >= 2 {
    qs.push(SearchQuery {
      text: words.join(" "),
      perspective: "terms".into(),
      weight: 0.9,
    });
  }

  let choices = extract_search_choices(utterance);
  if !choices.is_empty() {
    let stem = search_question_stem(utterance);
    for (label, text) in &choices {
      qs.push(SearchQuery {
        text: format!("{stem} {text}"),
        perspective: format!("choice-{label}"),
        weight: 0.75,
      });
    }
  }

  if has_hangul(&clean) {
    let en: Vec<String> = words
      .iter()
      .filter_map(|w| ko_science_term_to_en(w).map(String::from))
      .collect();
    if !en.is_empty() {
      qs.push(SearchQuery {
        text: en.join(" "),
        perspective: "english".into(),
        weight: 0.85,
      });
    }
  }

  qs
}

/// 검색 결과를 pnix ontology 판단으로 평가한다.
pub fn judge_search_results(
  utterance: &str,
  results: &[(String, Vec<SearchSnippet>)],
) -> SearchAnswer {
  let choices = extract_search_choices(utterance);
  if choices.is_empty() {
    judge_open_search(utterance, results)
  } else {
    judge_choices_search(utterance, &choices, results)
  }
}

/// 검색 결과 평가 정책.
pub fn search_evaluation_policy() -> crate::ontology::EvaluationPolicy {
  crate::ontology::EvaluationPolicy {
    id: "emergent-search".into(),
    accept_threshold: 0.55,
    hold_threshold: 0.30,
    ..Default::default()
  }
}

/// 평가 점수 → 판단 행동.
pub fn eval_to_judgement_action(
  policy: &crate::ontology::EvaluationPolicy,
  eval: &crate::ontology::EvaluationVector,
) -> crate::ontology::JudgementAction {
  use crate::ontology::JudgementAction;
  if eval.score >= policy.accept_threshold {
    JudgementAction::Accept
  } else if eval.score >= policy.hold_threshold {
    JudgementAction::Hold
  } else {
    JudgementAction::Reject
  }
}

/// 검색 스니펫 → ContextualFact + Interpretation 변환.
pub fn build_search_facts_and_interp(
  tag: &str,
  utterance: &str,
  snippets: &[&SearchSnippet],
) -> (Vec<ContextualFact>, Interpretation) {
  let facts: Vec<ContextualFact> = snippets
    .iter()
    .enumerate()
    .map(|(i, s)| ContextualFact {
      id: Some(crate::ontology::MeaningId::from(format!("{tag}-{i}"))),
      context: ContextId::from("emergent-search"),
      layer: crate::ontology::LayerId::from("L3"),
      subj: utterance.into(),
      pred: "search-evidence".into(),
      obj: s.text.clone(),
      status: crate::ontology::MeaningStatus::Candidate,
      confidence: search_term_overlap(utterance, &s.text),
      provenance_refs: if s.url.is_empty() {
        vec![]
      } else {
        vec![s.url.clone()]
      },
      proof_refs: vec![],
      contradiction_refs: vec![],
      loss: None,
      timestamp: None,
    })
    .collect();

  let fact_refs = facts
    .iter()
    .map(|f| f.id.as_ref().expect("search fact id").0.clone())
    .collect();

  let interp = Interpretation {
    id: crate::ontology::InterpretationId::from(format!("{tag}-interp")),
    observation_refs: vec!["utterance".into()],
    fact_refs,
    lift_refs: vec![],
    conflict_refs: vec![],
    loss: None,
  };

  (facts, interp)
}

/// 개방형 질문: 모든 검색 결과를 하나의 해석으로 평가한다.
pub fn judge_open_search(
  utterance: &str,
  results: &[(String, Vec<SearchSnippet>)],
) -> SearchAnswer {
  let all: Vec<&SearchSnippet> = results.iter().flat_map(|(_, ss)| ss).collect();
  let (facts, interp) = build_search_facts_and_interp("open", utterance, &all);
  let policy = search_evaluation_policy();
  let eval = ontology_evaluate(&policy, &interp, &facts);
  let action = eval_to_judgement_action(&policy, &eval);
  let top = top_search_snippets(&all, utterance, 3);

  let mut transcript = Vec::new();
  for s in &top {
    if !s.title.is_empty() {
      transcript.push(format!("[{}]", s.title));
    }
    transcript.push(s.text.clone());
  }
  transcript.push(String::new());
  transcript.push(format!(
    "({:.0}% confidence | coherence={:.2} coverage={:.2} safety={:.2})",
    eval.score * 100.0,
    eval.coherence,
    eval.coverage,
    eval.safety,
  ));

  SearchAnswer {
    transcript,
    sources: unique_search_sources(&all),
    confidence: eval.score,
    action: action.clone(),
    follow_up_hint: if matches!(action, crate::ontology::JudgementAction::Hold) {
      "검색 결과 불충분. 질문을 더 구체적으로 해주세요.".into()
    } else {
      String::new()
    },
  }
}

/// 객관식 질문: 보기별 해석을 만들고 ontology_select로 경쟁시킨다.
pub fn judge_choices_search(
  utterance: &str,
  choices: &[(String, String)],
  results: &[(String, Vec<SearchSnippet>)],
) -> SearchAnswer {
  let stem = search_question_stem(utterance);
  let policy = search_evaluation_policy();
  let all_snippets: Vec<&SearchSnippet> = results.iter().flat_map(|(_, ss)| ss).collect();

  let mut all_facts: Vec<ContextualFact> = Vec::new();
  let mut interps: Vec<Interpretation> = Vec::new();

  for (label, choice_text) in choices {
    let mut supporting: Vec<&SearchSnippet> = Vec::new();

    let choice_persp = format!("choice-{label}");
    for (persp, snippets) in results {
      if persp == &choice_persp {
        supporting.extend(snippets.iter());
      }
    }

    let choice_lower = choice_text.to_lowercase();
    for (persp, snippets) in results {
      if !persp.starts_with("choice-") {
        for s in snippets {
          if s.text.to_lowercase().contains(&choice_lower) {
            supporting.push(s);
          }
        }
      }
    }

    if supporting.is_empty() {
      continue;
    }

    let tag = format!("c-{label}");
    let base = all_facts.len();
    for (i, s) in supporting.iter().enumerate() {
      all_facts.push(ContextualFact {
        id: Some(crate::ontology::MeaningId::from(format!("{tag}-{i}"))),
        context: ContextId::from("emergent-search"),
        layer: crate::ontology::LayerId::from("L3"),
        subj: format!("{stem} -> {choice_text}"),
        pred: "supports-choice".into(),
        obj: s.text.clone(),
        status: crate::ontology::MeaningStatus::Candidate,
        confidence: search_term_overlap(&format!("{stem} {choice_text}"), &s.text),
        provenance_refs: if s.url.is_empty() {
          vec![]
        } else {
          vec![s.url.clone()]
        },
        proof_refs: vec![],
        contradiction_refs: vec![],
        loss: None,
        timestamp: None,
      });
    }

    let fact_refs: Vec<String> = (base..all_facts.len())
      .map(|i| all_facts[i].id.as_ref().expect("choice fact id").0.clone())
      .collect();

    interps.push(Interpretation {
      id: crate::ontology::InterpretationId::from(format!("choice-{label}")),
      observation_refs: vec!["utterance".into()],
      fact_refs,
      lift_refs: vec![],
      conflict_refs: vec![],
      loss: None,
    });
  }

  if interps.is_empty() {
    return judge_open_search(utterance, results);
  }

  match ontology_select(&policy, &interps, &all_facts) {
    Some(outcome) => {
      let winner_id = outcome
        .judgement
        .chosen_interpretation
        .as_ref()
        .map(|id| id.0.as_str())
        .unwrap_or("");
      let winner_label = winner_id.strip_prefix("choice-").unwrap_or(winner_id);
      let winner_text = choices
        .iter()
        .find(|(l, _)| l == winner_label)
        .map(|(_, t)| t.as_str())
        .unwrap_or("");

      let eval = &outcome.evaluation;
      let mut transcript = vec![format!(">>> {winner_label}) {winner_text}"), String::new()];

      let supporting: Vec<&&SearchSnippet> = all_snippets
        .iter()
        .filter(|s| s.text.to_lowercase().contains(&winner_text.to_lowercase()))
        .take(2)
        .collect();
      for s in &supporting {
        transcript.push(s.text.clone());
      }
      transcript.push(String::new());
      transcript.push(format!(
        "({:.0}% | coherence={:.2} coverage={:.2} safety={:.2} | {:?})",
        eval.score * 100.0,
        eval.coherence,
        eval.coverage,
        eval.safety,
        outcome.judgement.action,
      ));

      for interp in &interps {
        let e = ontology_evaluate(&policy, interp, &all_facts);
        let lbl = interp.id.0.strip_prefix("choice-").unwrap_or(&interp.id.0);
        let marker = if lbl == winner_label { " <--" } else { "" };
        transcript.push(format!(
          "  {lbl}) score={:.2} facts={}{}",
          e.score,
          interp.fact_refs.len(),
          marker,
        ));
      }

      SearchAnswer {
        transcript,
        sources: unique_search_sources(&all_snippets),
        confidence: eval.score,
        action: outcome.judgement.action.clone(),
        follow_up_hint: String::new(),
      }
    }
    None => judge_open_search(utterance, results),
  }
}

fn top_search_snippets<'a>(
  snippets: &[&'a SearchSnippet],
  utterance: &str,
  n: usize,
) -> Vec<&'a SearchSnippet> {
  let mut scored: Vec<(usize, f64)> = snippets
    .iter()
    .enumerate()
    .map(|(i, s)| (i, search_term_overlap(utterance, &s.text)))
    .collect();
  scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
  scored.iter().take(n).map(|(i, _)| snippets[*i]).collect()
}

fn unique_search_sources(snippets: &[&SearchSnippet]) -> Vec<String> {
  let mut seen = HashSet::new();
  snippets
    .iter()
    .filter_map(|s| {
      if s.url.is_empty() || !seen.insert(s.url.clone()) {
        None
      } else {
        Some(s.url.clone())
      }
    })
    .collect()
}

fn strip_search_particles(s: &str) -> String {
  let mut out = s.trim().to_string();
  for _ in 0..3 {
    let prev = out.clone();
    for suffix in &[
      "은 뭐야?",
      "는 뭐야?",
      "이 뭐야?",
      "가 뭐야?",
      "은 뭐야",
      "는 뭐야",
      "이 뭐야",
      "가 뭐야",
      "은?",
      "는?",
      "이?",
      "가?",
      "을?",
      "를?",
      "의?",
      "에서?",
      "에?",
      "으로?",
      "로?",
      "은",
      "는",
      "이",
      "가",
      "을",
      "를",
      "의",
      "에서",
      "에",
      "으로",
      "로",
      "뭐야",
      "무엇",
      "무엇인가",
      "어떻게",
      "인가요",
      "인가",
      "입니까",
      "인지",
      "?",
    ] {
      if let Some(stripped) = out.strip_suffix(suffix) {
        out = stripped.trim().to_string();
      }
    }
    if out == prev {
      break;
    }
  }
  out
}

fn search_content_words(s: &str) -> Vec<String> {
  let stops: HashSet<&str> = [
    "the", "a", "an", "is", "are", "was", "were", "of", "in", "to", "for", "and", "or", "that",
    "this", "which", "what", "how", "who", "when", "where", "why", "do", "does", "did", "not",
    "with", "by", "from", "at", "on", "it", "its", "be", "been", "being", "has", "have", "had",
  ]
  .into_iter()
  .collect();

  s.split(|c: char| c.is_whitespace() || c == '?' || c == '.' || c == ',' || c == ')')
    .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
    .filter(|w| w.len() > 1 && !stops.contains(*w))
    .map(String::from)
    .collect()
}

fn has_hangul(s: &str) -> bool {
  s.chars().any(|c| ('\u{AC00}'..='\u{D7AF}').contains(&c))
}

fn extract_search_choices(s: &str) -> Vec<(String, String)> {
  let mut choices = Vec::new();
  for label in &["A", "B", "C", "D", "E"] {
    let patterns = [
      format!("{label}) "),
      format!("{label}. "),
      format!("({label}) "),
    ];
    for pat in &patterns {
      if let Some(pos) = s.find(pat.as_str()) {
        let after = &s[pos + pat.len()..];
        let end = find_next_search_choice_start(after).unwrap_or(after.len());
        let text = after[..end].trim().to_string();
        if !text.is_empty() {
          choices.push((label.to_string(), text));
        }
        break;
      }
    }
  }
  choices
}

fn find_next_search_choice_start(s: &str) -> Option<usize> {
  for label in &["A", "B", "C", "D", "E"] {
    for pat in &[
      format!(" {label}) "),
      format!(" {label}. "),
      format!(" ({label}) "),
    ] {
      if let Some(pos) = s.find(pat.as_str()) {
        return Some(pos);
      }
    }
  }
  None
}

fn search_question_stem(s: &str) -> String {
  for label in &["A"] {
    for pat in &[
      format!("{label}) "),
      format!("{label}. "),
      format!("({label}) "),
    ] {
      if let Some(pos) = s.find(pat.as_str()) {
        return s[..pos].trim().to_string();
      }
    }
  }
  s.to_string()
}

fn search_term_overlap(a: &str, b: &str) -> f64 {
  let wa: HashSet<String> = search_content_words(&a.to_lowercase())
    .into_iter()
    .collect();
  let wb: HashSet<String> = search_content_words(&b.to_lowercase())
    .into_iter()
    .collect();
  if wa.is_empty() || wb.is_empty() {
    return 0.0;
  }
  let inter = wa.intersection(&wb).count() as f64;
  let union = wa.union(&wb).count() as f64;
  if union == 0.0 {
    0.0
  } else {
    inter / union
  }
}

fn ko_science_term_to_en(term: &str) -> Option<&'static str> {
  match term {
    "광합성" => Some("photosynthesis"),
    "세포" => Some("cell"),
    "미토콘드리아" => Some("mitochondria"),
    "DNA" | "디엔에이" => Some("DNA"),
    "유전자" | "유전" => Some("gene genetics"),
    "단백질" => Some("protein"),
    "효소" => Some("enzyme"),
    "화학식" => Some("chemical formula"),
    "원소" => Some("element"),
    "원자" => Some("atom"),
    "분자" => Some("molecule"),
    "화학결합" => Some("chemical bond"),
    "산화환원" | "산화" | "환원" => Some("redox reaction"),
    "주기율표" => Some("periodic table"),
    "몰" => Some("mole chemistry"),
    "pH" | "산도" => Some("pH"),
    "진화" => Some("evolution"),
    "생태계" => Some("ecosystem"),
    "항상성" => Some("homeostasis"),
    "면역" => Some("immunity"),
    "뉴런" | "신경" => Some("neuron"),
    "호르몬" => Some("hormone"),
    "속도" => Some("velocity"),
    "가속도" => Some("acceleration"),
    "힘" => Some("force"),
    "에너지" => Some("energy"),
    "운동량" => Some("momentum"),
    "파동" => Some("wave"),
    "전자기" => Some("electromagnetic"),
    "열역학" => Some("thermodynamics"),
    "엔트로피" => Some("entropy"),
    "미분" => Some("derivative calculus"),
    "적분" => Some("integral calculus"),
    "행렬" => Some("matrix linear algebra"),
    "벡터" => Some("vector"),
    "확률" => Some("probability"),
    "통계" => Some("statistics"),
    "삼각함수" => Some("trigonometry"),
    "집합" => Some("set theory"),
    "함수" => Some("function mathematics"),
    "극한" => Some("limit calculus"),
    _ => None,
  }
}

#[cfg(test)]
mod search_judgment_tests {
  use super::*;

  fn snippet(title: &str, text: &str, url: &str) -> SearchSnippet {
    SearchSnippet {
      title: title.to_string(),
      text: text.to_string(),
      url: url.to_string(),
    }
  }

  #[test]
  fn search_query_expansion_lifts_choices_and_korean_science_terms() {
    let choices = expand_search_queries("정답은? A) 세포 B) 에너지");
    assert!(choices.iter().any(|q| q.perspective == "choice-A"));
    assert!(choices.iter().any(|q| q.perspective == "choice-B"));

    let korean = expand_search_queries("광합성");
    assert!(korean
      .iter()
      .any(|q| q.perspective == "english" && q.text.contains("photosynthesis")));
  }

  #[test]
  fn open_search_judgement_builds_transcript_and_sources() {
    let results = vec![(
      "original".to_string(),
      vec![
        snippet(
          "Force",
          "force equals mass times acceleration",
          "https://example.test/force",
        ),
        snippet(
          "Energy",
          "energy and force are physics concepts",
          "https://example.test/energy",
        ),
      ],
    )];

    let answer = judge_search_results("force acceleration", &results);
    assert!(answer
      .transcript
      .iter()
      .any(|line| line.contains("force equals mass")));
    assert_eq!(answer.sources.len(), 2);
  }

  #[test]
  fn legal_blocked_source_families_have_zero_trust() {
    for url in &[
      "https://ko.wikipedia.org/wiki/중력파",
      "https://en.wikipedia.org/wiki/Gravity",
      "https://namu.wiki/w/중력",
    ] {
      let kind = classify_source_kind(url, "");
      assert_eq!(kind, SourceKind::Blocked, "{url} must be legally blocked");
      for policy in &[
        SourceTrustPolicy::Strict,
        SourceTrustPolicy::Normal,
        SourceTrustPolicy::Relaxed,
      ] {
        assert_eq!(source_kind_trust_prior(kind, *policy), 0.0);
      }
    }

    assert_eq!(
      classify_source_kind("https://www.britannica.com/science/gravity-physics", ""),
      SourceKind::Encyclopedia
    );
  }

  fn passage(text: &str) -> EvidencePassage {
    EvidencePassage {
      source_url: "https://example.test/p".to_string(),
      passage: text.to_string(),
      title: "Test".to_string(),
      source_kind: SourceKind::Encyclopedia,
      retrieved_at: "2026-05-11T00:00:00Z".to_string(),
    }
  }

  #[test]
  fn relation_owner_negation_flips_polarity_to_contradicts() {
    let p = passage("Light is not a single particle.");
    let facts =
      evidence_facts_from_passage_extracted(&p, "what is light", SourceTrustPolicy::Normal, None);
    assert!(!facts.is_empty(), "must emit at least one fact");
    assert!(
      facts
        .iter()
        .all(|f| matches!(f.polarity, Polarity::Contradicts)),
      "negation marker must flip polarity to Contradicts"
    );
    // owner-law reference is preserved in provenance
    assert!(facts
      .iter()
      .any(|f| f.provenance_refs.iter().any(|r| r.contains("negation.px"))));
  }

  #[test]
  fn relation_owner_comparison_overrides_predicate() {
    let p = passage("Proton mass is greater than electron mass.");
    let facts = evidence_facts_from_passage_extracted(
      &p,
      "compare proton electron",
      SourceTrustPolicy::Normal,
      None,
    );
    assert!(!facts.is_empty());
    // at least one fact should use the comparison predicate, not the raw verb
    assert!(
      facts.iter().any(|f| f.pred == "greater-than"),
      "comparison owner law must override pred to `greater-than`, got preds: {:?}",
      facts.iter().map(|f| f.pred.as_str()).collect::<Vec<_>>()
    );
    assert!(facts.iter().any(|f| f
      .provenance_refs
      .iter()
      .any(|r| r.contains("comparison.px"))));
  }

  #[test]
  fn relation_owner_definition_marks_pred_is_defined_as() {
    let p = passage("Ontology means the science of being.");
    let facts = evidence_facts_from_passage_extracted(
      &p,
      "what is ontology",
      SourceTrustPolicy::Normal,
      None,
    );
    assert!(!facts.is_empty());
    assert!(
      facts.iter().any(|f| f.pred == "is-defined-as"),
      "definition owner law must set pred to `is-defined-as`"
    );
    assert!(facts.iter().any(|f| f
      .provenance_refs
      .iter()
      .any(|r| r.contains("definition.px"))));
  }

  #[test]
  fn relation_owner_formula_sets_pred_and_dimension_check_flag() {
    let p = passage("F = ma is Newton's second law.");
    let facts =
      evidence_facts_from_passage_extracted(&p, "what is force", SourceTrustPolicy::Normal, None);
    assert!(!facts.is_empty());
    assert!(
      facts.iter().any(|f| f.pred == "formula"),
      "formula owner law must set pred to `formula`"
    );
    assert!(facts
      .iter()
      .any(|f| f.provenance_refs.iter().any(|r| r.contains("formula.px"))));
    assert!(
      facts.iter().any(|f| f
        .provenance_refs
        .iter()
        .any(|r| r == "formula-dimension-check-required")),
      "formula fact must carry dimension-check-required hint for downstream host"
    );
    // OWNER-LAW (2026-05-11) wire-up: the dimension inference engine
    // runs at fact-emission time for the passage `F = ma` and emits
    // `formula-dimension-check:passed`.
    assert!(
      facts.iter().any(|f| f
        .provenance_refs
        .iter()
        .any(|r| r == "formula-dimension-check:passed")),
      "F = ma must auto-emit formula-dimension-check:passed via inference engine"
    );
  }

  #[test]
  fn relation_owner_formula_emits_failed_marker_on_dimensionally_wrong_passage() {
    // `F = mv` is dimensionally wrong (mass·length·time⁻² vs
    // mass·length·time⁻¹). The inference engine should emit `:failed`.
    let p = passage("Newton wrote F = mv on the chalkboard.");
    let facts =
      evidence_facts_from_passage_extracted(&p, "what is force", SourceTrustPolicy::Normal, None);
    assert!(facts.iter().any(|f| f.pred == "formula"));
    assert!(
      facts.iter().any(|f| f
        .provenance_refs
        .iter()
        .any(|r| r == "formula-dimension-check:failed")),
      "F = mv must auto-emit formula-dimension-check:failed via inference engine"
    );
  }

  #[test]
  fn relation_owner_formula_omits_result_marker_when_passage_bounds_to_empty_rhs() {
    // `F = Qa` — Q is unknown, so the bounded extraction drops the
    // rhs entirely. The wire-up correctly emits *no* result marker —
    // only the original `formula-dimension-check-required` flag
    // stands. The downstream gate then reads this as `Missing` and
    // clamps to Candidate, which is the safe verdict. For strict
    // unknown-symbol Held semantics use the engine directly.
    let p = passage("Some textbook writes F = Qa as an alternate form.");
    let facts =
      evidence_facts_from_passage_extracted(&p, "what is force", SourceTrustPolicy::Normal, None);
    assert!(facts.iter().any(|f| f.pred == "formula"));
    // The required marker is present...
    assert!(facts.iter().any(|f| f
      .provenance_refs
      .iter()
      .any(|r| r == "formula-dimension-check-required")));
    // ...but no result marker (passed/failed/held) — bounding drained
    // the rhs, so the pipeline correctly stays silent rather than
    // guessing.
    for f in facts.iter().filter(|f| f.pred == "formula") {
      assert!(
        !f.provenance_refs
          .iter()
          .any(|r| r.starts_with("formula-dimension-check:")),
        "no result marker should be emitted when rhs bounds to empty; got: {:?}",
        f.provenance_refs
      );
    }
  }

  #[test]
  fn relation_owner_causality_sets_pred_and_chain_eligible() {
    let p = passage("Lower mass causes higher acceleration under the same force.");
    let facts = evidence_facts_from_passage_extracted(
      &p,
      "force acceleration",
      SourceTrustPolicy::Normal,
      None,
    );
    assert!(!facts.is_empty());
    assert!(
      facts.iter().any(|f| f.pred == "causes"),
      "causality owner law must set pred to `causes`"
    );
    assert!(facts
      .iter()
      .any(|f| f.provenance_refs.iter().any(|r| r.contains("causality.px"))));
    assert!(
      facts
        .iter()
        .any(|f| f.provenance_refs.iter().any(|r| r == "chain-eligible:true")),
      "causality fact must carry chain-eligible hint for 2-hop composer"
    );
  }

  #[test]
  fn relation_owner_comparison_wins_over_formula_when_both_match() {
    // "X is greater than Y = Z" — comparison takes precedence over formula
    // per the documented pred precedence ladder.
    let p = passage("proton mass is greater than electron mass = 9.1e-31");
    let facts =
      evidence_facts_from_passage_extracted(&p, "compare", SourceTrustPolicy::Normal, None);
    assert!(facts.iter().any(|f| f.pred == "greater-than"));
    assert!(
      !facts.iter().any(|f| f.pred == "formula"),
      "comparison must shadow formula when both match"
    );
  }

  #[test]
  fn relation_owner_supports_polarity_default_when_no_negation() {
    let p = passage("Force equals mass times acceleration.");
    let facts =
      evidence_facts_from_passage_extracted(&p, "what is force", SourceTrustPolicy::Normal, None);
    assert!(!facts.is_empty());
    assert!(
      facts
        .iter()
        .all(|f| matches!(f.polarity, Polarity::Supports)),
      "no negation marker means default Supports polarity"
    );
  }

  #[test]
  fn choice_search_judgement_uses_choice_path() {
    let results = vec![
      (
        "choice-A".to_string(),
        vec![snippet(
          "A",
          "Paris is the capital city of France",
          "https://example.test/paris",
        )],
      ),
      (
        "choice-B".to_string(),
        vec![snippet(
          "B",
          "Berlin is the capital city of Germany",
          "https://example.test/berlin",
        )],
      ),
    ];

    let answer = judge_search_results("프랑스의 수도는? A) Paris B) Berlin", &results);
    assert!(answer
      .transcript
      .first()
      .is_some_and(|line| line.starts_with(">>> ")));
    assert!(answer.transcript.iter().any(|line| line.contains("score=")));
  }
}
