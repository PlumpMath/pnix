//! Pure ontology-core IR types.
//!
//! These types intentionally stay free of runtime handles, host state,
//! executor policy, or backend-specific objects. They define the canonical
//! data family for contextual facts plus interpretation/evaluation/judgement/
//! promotion lifecycles described in `ontology.md`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

macro_rules! string_id {
  ($name:ident) => {
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct $name(pub String);

    impl From<&str> for $name {
      fn from(value: &str) -> Self {
        Self(value.to_string())
      }
    }

    impl From<String> for $name {
      fn from(value: String) -> Self {
        Self(value)
      }
    }
  };
}

string_id!(ContextId);
string_id!(LayerId);
string_id!(MeaningId);
string_id!(InterpretationId);
string_id!(SemanticRecordId);
string_id!(SemanticEpisodeId);
string_id!(KnowledgeRecordId);
string_id!(ToolCapabilityId);
string_id!(ToolActionPlanId);
string_id!(ToolExecutionResultId);
string_id!(ExpressionProjectionId);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MeaningStatus {
  Candidate,
  Accepted,
  Rejected,
  Contradicted,
  Held,
  Deprecated,
  Deleted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LossKind {
  Lossless,
  Lossy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LossPolicy {
  pub kind: LossKind,
  #[serde(default)]
  pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeaningLift {
  pub id: String,
  pub from_context: ContextId,
  pub to_context: ContextId,
  #[serde(default)]
  pub object_map: BTreeMap<String, String>,
  #[serde(default)]
  pub relation_map: BTreeMap<String, String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub loss: Option<LossPolicy>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluationWeights {
  pub coherence: f64,
  pub coverage: f64,
  pub loss: f64,
  pub cost: f64,
  pub replayability: f64,
  pub safety: f64,
}

impl Default for EvaluationWeights {
  fn default() -> Self {
    Self {
      coherence: 1.0,
      coverage: 1.0,
      loss: 1.0,
      cost: 0.5,
      replayability: 1.0,
      safety: 1.5,
    }
  }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluationPolicy {
  pub id: String,
  #[serde(default)]
  pub weights: EvaluationWeights,
  #[serde(default = "default_accept_threshold")]
  pub accept_threshold: f64,
  #[serde(default = "default_hold_threshold")]
  pub hold_threshold: f64,
  #[serde(default = "default_min_safety")]
  pub min_safety: f64,
  #[serde(default = "default_min_replayability")]
  pub min_replayability: f64,
}

impl Default for EvaluationPolicy {
  fn default() -> Self {
    Self {
      id: "ontology.default".to_string(),
      weights: EvaluationWeights::default(),
      accept_threshold: default_accept_threshold(),
      hold_threshold: default_hold_threshold(),
      min_safety: default_min_safety(),
      min_replayability: default_min_replayability(),
    }
  }
}

fn default_confidence() -> f64 {
  1.0
}

fn default_accept_threshold() -> f64 {
  0.70
}

fn default_hold_threshold() -> f64 {
  0.45
}

fn default_min_safety() -> f64 {
  0.50
}

fn default_min_replayability() -> f64 {
  0.30
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextualFact {
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub id: Option<MeaningId>,
  pub context: ContextId,
  pub layer: LayerId,
  pub subj: String,
  pub pred: String,
  pub obj: String,
  pub status: MeaningStatus,
  #[serde(default = "default_confidence")]
  pub confidence: f64,
  #[serde(default)]
  pub provenance_refs: Vec<String>,
  #[serde(default)]
  pub proof_refs: Vec<String>,
  #[serde(default)]
  pub contradiction_refs: Vec<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub loss: Option<LossPolicy>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub timestamp: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflictSet {
  pub id: String,
  #[serde(default)]
  pub fact_refs: Vec<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Interpretation {
  pub id: InterpretationId,
  #[serde(default)]
  pub observation_refs: Vec<String>,
  #[serde(default)]
  pub fact_refs: Vec<String>,
  #[serde(default)]
  pub lift_refs: Vec<String>,
  #[serde(default)]
  pub conflict_refs: Vec<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub loss: Option<LossPolicy>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluationVector {
  pub id: String,
  pub interpretation: InterpretationId,
  pub policy: String,
  pub coherence: f64,
  pub coverage: f64,
  pub loss_penalty: f64,
  pub cost: f64,
  pub replayability: f64,
  pub safety: f64,
  pub score: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum JudgementAction {
  Accept,
  Reject,
  Hold,
  Contradict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JudgementRecord {
  pub id: String,
  pub evaluation: String,
  pub action: JudgementAction,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub chosen_interpretation: Option<InterpretationId>,
  #[serde(default)]
  pub chosen_fact_refs: Vec<String>,
  #[serde(default)]
  pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromotionDecision {
  pub id: String,
  pub judgement: String,
  pub target_status: MeaningStatus,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub reason: Option<String>,
  #[serde(default)]
  pub artifact_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ToolTransportKind {
  Embedded,
  Plugin,
  Mcp,
  Http,
  Nrepl,
  Shell,
  StdIo,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCapabilityRecord {
  pub id: ToolCapabilityId,
  pub tool_name: String,
  pub adapter_runtime: String,
  pub transport: ToolTransportKind,
  #[serde(default)]
  pub executable_actions: Vec<String>,
  #[serde(default)]
  pub query_actions: Vec<String>,
  #[serde(default)]
  pub observable_events: Vec<String>,
  #[serde(default)]
  pub artifact_kinds: Vec<String>,
  #[serde(default)]
  pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolActionPlan {
  pub id: ToolActionPlanId,
  pub capability: ToolCapabilityId,
  pub judgement: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub chosen_interpretation: Option<InterpretationId>,
  pub action_name: String,
  #[serde(default)]
  pub args: BTreeMap<String, String>,
  #[serde(default)]
  pub provenance_refs: Vec<String>,
  #[serde(default)]
  pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExpressionProjectionRecord {
  pub id: ExpressionProjectionId,
  pub context: ContextId,
  pub layer: LayerId,
  pub subject: String,
  pub projection_family: String,
  pub canonical_form: String,
  #[serde(default)]
  pub semantic_fact_refs: Vec<String>,
  #[serde(default)]
  pub surface_forms: BTreeMap<String, String>,
  #[serde(default)]
  pub provenance_refs: Vec<String>,
  #[serde(default)]
  pub artifact_refs: Vec<String>,
  #[serde(default)]
  pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ToolExecutionStatus {
  Succeeded,
  Failed,
  Held,
  Rejected,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolExecutionResult {
  pub id: ToolExecutionResultId,
  pub plan: ToolActionPlanId,
  pub capability: ToolCapabilityId,
  pub status: ToolExecutionStatus,
  #[serde(default)]
  pub semantic_facts: Vec<ContextualFact>,
  #[serde(default)]
  pub expression_projections: Vec<ExpressionProjectionRecord>,
  #[serde(default)]
  pub artifact_refs: Vec<String>,
  #[serde(default)]
  pub provenance_refs: Vec<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub summary: Option<String>,
  #[serde(default)]
  pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SemanticRecordKind {
  ContextualFact,
  Interpretation,
  Evaluation,
  Judgement,
  Promotion,
  ToolCapability,
  ToolActionPlan,
  ToolExecutionResult,
  ExpressionProjection,
  Knowledge,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
pub enum SemanticRecordValue {
  ContextualFact(ContextualFact),
  Interpretation(Interpretation),
  Evaluation(EvaluationVector),
  Judgement(JudgementRecord),
  Promotion(PromotionDecision),
  ToolCapability(ToolCapabilityRecord),
  ToolActionPlan(ToolActionPlan),
  ToolExecutionResult(ToolExecutionResult),
  ExpressionProjection(ExpressionProjectionRecord),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticRecord {
  pub id: SemanticRecordId,
  pub episode: SemanticEpisodeId,
  pub record_kind: SemanticRecordKind,
  pub value: SemanticRecordValue,
  #[serde(default)]
  pub provenance_refs: Vec<String>,
  #[serde(default)]
  pub artifact_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticEpisode {
  pub id: SemanticEpisodeId,
  #[serde(default)]
  pub observation_refs: Vec<String>,
  #[serde(default)]
  pub record_refs: Vec<SemanticRecordId>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub chosen_interpretation: Option<InterpretationId>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub judgement_ref: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub promotion_ref: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeRecord {
  pub id: KnowledgeRecordId,
  pub episode: SemanticEpisodeId,
  pub target_status: MeaningStatus,
  #[serde(default)]
  pub fact_refs: Vec<String>,
  #[serde(default)]
  pub source_record_refs: Vec<SemanticRecordId>,
  #[serde(default)]
  pub provenance_refs: Vec<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticIngestEnvelope {
  #[serde(default)]
  pub observation_refs: Vec<String>,
  #[serde(default)]
  pub records: Vec<SemanticRecord>,
  pub episode: SemanticEpisode,
  #[serde(default)]
  pub knowledge_records: Vec<KnowledgeRecord>,
  #[serde(default)]
  pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectionOutcome {
  pub evaluation: EvaluationVector,
  pub judgement: JudgementRecord,
}

fn clamp01(value: f64) -> f64 {
  value.clamp(0.0, 1.0)
}

fn ratio(num: usize, denom: usize) -> f64 {
  if denom == 0 {
    0.0
  } else {
    num as f64 / denom as f64
  }
}

fn status_contributes_to_coherence(status: &MeaningStatus) -> bool {
  matches!(
    status,
    MeaningStatus::Candidate | MeaningStatus::Accepted | MeaningStatus::Held
  )
}

fn status_contributes_to_safety_risk(status: &MeaningStatus) -> bool {
  matches!(
    status,
    MeaningStatus::Rejected
      | MeaningStatus::Contradicted
      | MeaningStatus::Deprecated
      | MeaningStatus::Deleted
  )
}

fn loss_penalty(loss: Option<&LossPolicy>) -> f64 {
  match loss.map(|entry| &entry.kind) {
    Some(LossKind::Lossless) | None => 0.0,
    Some(LossKind::Lossy) => 1.0,
  }
}

impl EvaluationVector {
  pub fn compute_score(&self, policy: &EvaluationPolicy) -> f64 {
    let weights = &policy.weights;
    let weighted_sum = self.coherence * weights.coherence
      + self.coverage * weights.coverage
      + (1.0 - self.loss_penalty) * weights.loss
      + (1.0 - self.cost) * weights.cost
      + self.replayability * weights.replayability
      + self.safety * weights.safety;
    let total_weight = weights.coherence
      + weights.coverage
      + weights.loss
      + weights.cost
      + weights.replayability
      + weights.safety;
    if total_weight <= 0.0 {
      0.0
    } else {
      clamp01(weighted_sum / total_weight)
    }
  }
}

pub fn ontology_lift_fact(lift: &MeaningLift, fact: &ContextualFact) -> ContextualFact {
  let combined_loss = match (&fact.loss, &lift.loss) {
    (Some(existing), Some(new)) => Some(LossPolicy {
      kind: if matches!(existing.kind, LossKind::Lossy) || matches!(new.kind, LossKind::Lossy) {
        LossKind::Lossy
      } else {
        LossKind::Lossless
      },
      notes: existing
        .notes
        .iter()
        .chain(new.notes.iter())
        .cloned()
        .collect(),
    }),
    (Some(existing), None) => Some(existing.clone()),
    (None, Some(new)) => Some(new.clone()),
    (None, None) => None,
  };
  let lifted_id = fact
    .id
    .as_ref()
    .map(|id| MeaningId(format!("{}@{}", id.0, lift.to_context.0)));

  ContextualFact {
    id: lifted_id,
    context: lift.to_context.clone(),
    layer: fact.layer.clone(),
    subj: lift
      .object_map
      .get(&fact.subj)
      .cloned()
      .unwrap_or_else(|| fact.subj.clone()),
    pred: lift
      .relation_map
      .get(&fact.pred)
      .cloned()
      .unwrap_or_else(|| fact.pred.clone()),
    obj: lift
      .object_map
      .get(&fact.obj)
      .cloned()
      .unwrap_or_else(|| fact.obj.clone()),
    status: fact.status.clone(),
    confidence: fact.confidence,
    provenance_refs: fact.provenance_refs.clone(),
    proof_refs: fact.proof_refs.clone(),
    contradiction_refs: fact.contradiction_refs.clone(),
    loss: combined_loss,
    timestamp: fact.timestamp.clone(),
  }
}

pub fn ontology_evaluate(
  policy: &EvaluationPolicy,
  interpretation: &Interpretation,
  facts: &[ContextualFact],
) -> EvaluationVector {
  let referenced: Vec<&ContextualFact> = interpretation
    .fact_refs
    .iter()
    .filter_map(|fact_ref| {
      facts
        .iter()
        .find(|fact| fact.id.as_ref().is_some_and(|id| id.0 == *fact_ref))
    })
    .collect();
  let matched_fact_refs = referenced.len();
  let missing_fact_refs = interpretation
    .fact_refs
    .len()
    .saturating_sub(matched_fact_refs);
  let positive_count = referenced
    .iter()
    .filter(|fact| status_contributes_to_coherence(&fact.status))
    .count();
  let risky_count = referenced
    .iter()
    .filter(|fact| status_contributes_to_safety_risk(&fact.status))
    .count()
    + interpretation.conflict_refs.len();
  let replayable_count = referenced
    .iter()
    .filter(|fact| !fact.provenance_refs.is_empty() || !fact.proof_refs.is_empty())
    .count();

  let referenced_loss_sum: f64 = referenced
    .iter()
    .map(|fact| loss_penalty(fact.loss.as_ref()))
    .sum();
  let total_loss_inputs = referenced.len() + usize::from(interpretation.loss.is_some());
  let total_loss_sum = referenced_loss_sum + loss_penalty(interpretation.loss.as_ref());
  let loss_penalty = if total_loss_inputs == 0 {
    0.0
  } else {
    clamp01(total_loss_sum / total_loss_inputs as f64)
  };

  let coherence_denom = positive_count + risky_count;
  let coverage_denom = interpretation.fact_refs.len() + interpretation.observation_refs.len();
  let cost_denom = interpretation.fact_refs.len()
    + interpretation.lift_refs.len()
    + interpretation.conflict_refs.len()
    + 1;

  let mut evaluation = EvaluationVector {
    id: format!("eval.{}", interpretation.id.0),
    interpretation: interpretation.id.clone(),
    policy: policy.id.clone(),
    coherence: if coherence_denom == 0 {
      1.0
    } else {
      clamp01(ratio(positive_count, coherence_denom))
    },
    coverage: if coverage_denom == 0 {
      0.0
    } else {
      clamp01(matched_fact_refs as f64 / coverage_denom as f64)
    },
    loss_penalty,
    cost: clamp01(
      (missing_fact_refs + interpretation.lift_refs.len() + interpretation.conflict_refs.len())
        as f64
        / cost_denom as f64,
    ),
    replayability: if matched_fact_refs == 0 {
      0.0
    } else {
      clamp01(ratio(replayable_count, matched_fact_refs))
    },
    safety: if matched_fact_refs == 0 && interpretation.conflict_refs.is_empty() {
      1.0
    } else {
      clamp01(
        1.0
          - (risky_count as f64
            / (matched_fact_refs + interpretation.conflict_refs.len()).max(1) as f64),
      )
    },
    score: 0.0,
  };
  evaluation.score = evaluation.compute_score(policy);
  evaluation
}

fn preferred_evaluation(current: &EvaluationVector, candidate: &EvaluationVector) -> bool {
  candidate.score > current.score
    || (candidate.score == current.score && candidate.safety > current.safety)
    || (candidate.score == current.score
      && candidate.safety == current.safety
      && candidate.replayability > current.replayability)
    || (candidate.score == current.score
      && candidate.safety == current.safety
      && candidate.replayability == current.replayability
      && candidate.loss_penalty < current.loss_penalty)
    || (candidate.score == current.score
      && candidate.safety == current.safety
      && candidate.replayability == current.replayability
      && candidate.loss_penalty == current.loss_penalty
      && candidate.cost < current.cost)
    || (candidate.score == current.score
      && candidate.safety == current.safety
      && candidate.replayability == current.replayability
      && candidate.loss_penalty == current.loss_penalty
      && candidate.cost == current.cost
      && candidate.interpretation.0 < current.interpretation.0)
}

pub fn ontology_select(
  policy: &EvaluationPolicy,
  interpretations: &[Interpretation],
  facts: &[ContextualFact],
) -> Option<SelectionOutcome> {
  let mut best: Option<EvaluationVector> = None;

  for interpretation in interpretations {
    let evaluation = ontology_evaluate(policy, interpretation, facts);
    match &best {
      Some(current) if !preferred_evaluation(current, &evaluation) => {}
      _ => best = Some(evaluation),
    }
  }

  let evaluation = best?;
  let action = if evaluation.safety < policy.min_safety
    || evaluation.replayability < policy.min_replayability
  {
    JudgementAction::Hold
  } else if evaluation.score >= policy.accept_threshold {
    JudgementAction::Accept
  } else if evaluation.score >= policy.hold_threshold {
    JudgementAction::Hold
  } else {
    JudgementAction::Reject
  };

  Some(SelectionOutcome {
    judgement: JudgementRecord {
      id: format!("judge.{}", evaluation.interpretation.0),
      evaluation: evaluation.id.clone(),
      action,
      chosen_interpretation: Some(evaluation.interpretation.clone()),
      chosen_fact_refs: interpretations
        .iter()
        .find(|candidate| candidate.id == evaluation.interpretation)
        .map(|candidate| candidate.fact_refs.clone())
        .unwrap_or_default(),
      notes: vec![
        "deterministic tie-break: score -> safety -> replayability -> lower loss -> lower cost -> lexical interpretation id"
          .to_string(),
      ],
    },
    evaluation,
  })
}

/// Tesseract macro fold layer (carrier only — no fold law here).
///
/// OWNER-LAW (2026-05-11): pnix-core is the typed kernel for the tesseract
/// macro substrate but does NOT own the fold law. The macro itself lives in
/// `stdlib/lib/gate/tesseract-constitution.px` and the macro-generated
/// ontology lives in stdlib `.px` owners. This enum is a pure invariant
/// pin so receipts can name which layer they belong to without re-defining
/// the layer set in every consumer.
///
/// Six layers: `surface -> ontology -> semantic -> gate -> runtime -> audit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TesseractLayer {
  /// Raw form, path = value.
  Surface,
  /// Each path segment classified as a concept (role / capability / law).
  Ontology,
  /// Frame: object / relation / invariant / event / generator / property /
  /// measured-value / goal.
  Semantic,
  /// Constitution / safety / capability / proof / permission gates.
  Gate,
  /// Picked runtime route from candidates.
  Runtime,
  /// Fold log: ontology trace, semantic trace, gate trace, runtime trace,
  /// replay reference, failure reason if any.
  Audit,
}

/// Receipt carrier for one tesseract macro fold (six-layer envelope).
///
/// Pure data. Never auto-Accepted. The fold itself is owned by the macro
/// in stdlib `.px`; this struct only records *which refs the fold produced
/// at each layer* so downstream replay / audit / compare can address them.
#[derive(Debug, Clone)]
pub struct TesseractFoldReceipt {
  /// Surface-layer reference (raw form id).
  pub surface_ref: String,
  /// Ontology-layer references (role / concept / capability / law refs).
  pub ontology_refs: Vec<String>,
  /// Semantic-layer references (frame / role-binding refs).
  pub semantic_refs: Vec<String>,
  /// Gate-layer references (constitution / safety / capability / proof refs).
  pub gate_refs: Vec<String>,
  /// Runtime-layer references (selected route candidates).
  pub runtime_refs: Vec<String>,
  /// Audit-layer references (replay anchor, fold log, trace ids).
  pub audit_refs: Vec<String>,
  /// Held entries emitted during the fold (zero if all layers closed).
  pub held_refs: Vec<String>,
  /// OWNER-LAW: candidate_only = true means this receipt is not a promotion
  /// authority. Owner-law promotion still goes through the lane-aware gate.
  pub candidate_only: bool,
}

impl TesseractFoldReceipt {
  /// Empty receipt (no fold layer closed yet).
  pub fn empty(surface_ref: impl Into<String>) -> Self {
    Self {
      surface_ref: surface_ref.into(),
      ontology_refs: Vec::new(),
      semantic_refs: Vec::new(),
      gate_refs: Vec::new(),
      runtime_refs: Vec::new(),
      audit_refs: Vec::new(),
      held_refs: Vec::new(),
      candidate_only: true,
    }
  }

  /// Append a ref to a specific layer.
  pub fn append(&mut self, layer: TesseractLayer, r: impl Into<String>) {
    let s = r.into();
    match layer {
      TesseractLayer::Surface => self.surface_ref = s,
      TesseractLayer::Ontology => self.ontology_refs.push(s),
      TesseractLayer::Semantic => self.semantic_refs.push(s),
      TesseractLayer::Gate => self.gate_refs.push(s),
      TesseractLayer::Runtime => self.runtime_refs.push(s),
      TesseractLayer::Audit => self.audit_refs.push(s),
    }
  }

  /// Whether all six layers have at least one ref. This is *necessary but
  /// not sufficient* for closure — gate / audit refs must additionally pass
  /// owner-law proof. The macro fold is the law owner.
  pub fn has_all_layers(&self) -> bool {
    !self.surface_ref.is_empty()
      && !self.ontology_refs.is_empty()
      && !self.semantic_refs.is_empty()
      && !self.gate_refs.is_empty()
      && !self.runtime_refs.is_empty()
      && !self.audit_refs.is_empty()
  }
}

/// Evidence source lane for promotion gating.
///
/// OWNER-LAW (2026-05-10): pnix 는 LLM 없이 작동하는 deterministic AI substrate
/// (`CLAUDE.md` OWNER-LAW CONSTITUTION). External evidence (web search, API,
/// HTML, OCR, ASR, human prose, tool execution result) cannot promote directly
/// to `Accepted` — owner-law proof + replay + provenance + negative/Held proof
/// are required. Only internal owner-law lanes can yield `Accepted` directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceLane {
  /// `.px` owner-law surface or stdlib gate; deterministic + replayable.
  /// `Accept` → `Accepted` allowed.
  InternalOwnerLaw,
  /// Existing Accepted facts in store; replay/audit covered.
  /// `Accept` → `Accepted` allowed.
  InternalAcceptedMemory,
  /// Web search snippet / passage. Untrusted external prose.
  /// `Accept` → `Candidate` (cleared for owner-law review).
  ExternalWebSearch,
  /// External API / schema lift. Untrusted unless schema is owner-gated.
  /// `Accept` → `Candidate`.
  ExternalApi,
  /// Transducer output (OCR, ASR, vision, parser). Untrusted.
  /// `Accept` → `Candidate`.
  TransducerOutput,
  /// Human-provided prose (chat, comment, note). Untrusted.
  /// `Accept` → `Candidate` / `Held`.
  HumanProvidedProse,
  /// Tool execution result (subprocess, RPC). Replayable but not owner-law.
  /// `Accept` → `Candidate`.
  ToolExecutionResult,
  /// Peer evidence from another pnix node (P2P heartbeat / federated). Untrusted
  /// because the remote owner-law is not under this substrate's control.
  /// `Accept` → `Candidate`.
  PeerEvidence,
  /// **Internal derived reasoning** (2026-05-11).
  ///
  /// 2-hop / multi-hop composition of internal Accepted (or
  /// Accepted+Candidate) facts via a relation-composition owner law
  /// (`stdlib/lib/gate/reasoning/relation-composition.px`). Even when
  /// every parent fact is Accepted, the composed predicate is a *new
  /// semantic claim* and cannot bypass owner-law proof — so this lane
  /// is **untrusted by default**: `Accept` → `Candidate`.
  ///
  /// Composed `Accepted` requires a separate proof slice that fuses
  /// replay + negative / Held discipline + owner-law gate around the
  /// composition rule itself, not just around the parent facts. Until
  /// that slice exists, this lane never yields `Accepted`.
  InternalDerivedReasoning,
}

impl EvidenceLane {
  /// Whether this lane may promote `Accept` directly to `Accepted`.
  ///
  /// OWNER-LAW (2026-05-11): `InternalDerivedReasoning` is explicitly
  /// **not** here. Composed predicates are new semantic claims —
  /// having Accepted parents does not transfer Accepted to the
  /// composition. Proof of the composed predicate itself is required.
  pub fn allow_direct_accepted(self) -> bool {
    matches!(
      self,
      EvidenceLane::InternalOwnerLaw | EvidenceLane::InternalAcceptedMemory
    )
  }

  /// Map a `lane:<Variant>` provenance tag back to the enum variant.
  ///
  /// OWNER-LAW (2026-05-11): callers that emit `SemanticRecord` with a
  /// `provenance_refs` entry like `"lane:InternalDerivedReasoning"`
  /// promise that future judges will route the record through the
  /// matching lane gate. This parser is the *enforcement side* of that
  /// promise — it converts the tag back into a typed `EvidenceLane` so
  /// `ontology_promote_with_lane` can clamp `Accept` correctly.
  ///
  /// Returns `None` for unrecognized tags so callers can decide
  /// whether to fail-closed (treat as untrusted) or pick a safe
  /// default. **Failing closed is the correct default for unknown
  /// provenance lanes** — silently picking `InternalOwnerLaw` would
  /// let a malformed tag promote to Accepted.
  pub fn from_tag(tag: &str) -> Option<EvidenceLane> {
    match tag {
      "InternalOwnerLaw" => Some(EvidenceLane::InternalOwnerLaw),
      "InternalAcceptedMemory" => Some(EvidenceLane::InternalAcceptedMemory),
      "ExternalWebSearch" => Some(EvidenceLane::ExternalWebSearch),
      "ExternalApi" => Some(EvidenceLane::ExternalApi),
      "TransducerOutput" => Some(EvidenceLane::TransducerOutput),
      "HumanProvidedProse" => Some(EvidenceLane::HumanProvidedProse),
      "ToolExecutionResult" => Some(EvidenceLane::ToolExecutionResult),
      "PeerEvidence" => Some(EvidenceLane::PeerEvidence),
      "InternalDerivedReasoning" => Some(EvidenceLane::InternalDerivedReasoning),
      _ => None,
    }
  }

  /// Find the first `lane:<Variant>` entry in a `SemanticRecord`'s
  /// `provenance_refs` and return the matching enum variant.
  ///
  /// Returns `None` when no `lane:` entry exists *or* when the tag
  /// after `lane:` is unrecognized. **Callers must treat `None` as
  /// "untrusted — clamp Accept to Candidate"** rather than picking a
  /// permissive default. The convenience wrapper
  /// `promote_record_via_provenance_lane` does this.
  ///
  /// Note: this function does **not** detect conflicting lane tags
  /// (two distinct recognized tags on the same record). For that, use
  /// [`EvidenceLane::resolve_from_provenance_refs`], which returns a
  /// `LaneTagResolution` distinguishing None / Single / Unknown /
  /// Conflicting cases. Safety-critical promotion paths (e.g.
  /// `promote_record_via_provenance_lane`) use the strict resolver.
  pub fn from_provenance_refs(refs: &[String]) -> Option<EvidenceLane> {
    match Self::resolve_from_provenance_refs(refs) {
      LaneTagResolution::Single(lane) => Some(lane),
      _ => None,
    }
  }

  /// Conflict-aware lane tag resolver.
  ///
  /// OWNER-LAW (2026-05-11): collects every `lane:<X>` entry in
  /// `refs`, dedupes by string, and returns:
  ///
  /// - `None` — no lane tag at all
  /// - `Single(lane)` — exactly one distinct recognized tag (after
  ///   dedupe of identical strings)
  /// - `Unknown(tag)` — exactly one distinct tag, unrecognized
  /// - `Conflicting(tags)` — two or more distinct tags. Even if all
  ///   are recognized as "internal" lanes (e.g. `InternalOwnerLaw` +
  ///   `InternalAcceptedMemory`), this still fails closed because the
  ///   caller's upstream emitter has a bug — different layers
  ///   disagreed on what lane this record came from.
  ///
  /// `Conflicting` is deliberately a fail-closed signal, not silent
  /// first-match-wins: a record claiming two different lanes is a
  /// supply-chain / metadata-injection smell and should not promote.
  pub fn resolve_from_provenance_refs(refs: &[String]) -> LaneTagResolution {
    let mut tags: Vec<String> = Vec::new();
    for r in refs {
      if let Some(tag) = r.strip_prefix("lane:") {
        let owned = tag.to_string();
        if !tags.contains(&owned) {
          tags.push(owned);
        }
      }
    }
    match tags.len() {
      0 => LaneTagResolution::None,
      1 => {
        let tag = tags.into_iter().next().expect("len==1");
        match EvidenceLane::from_tag(&tag) {
          Some(lane) => LaneTagResolution::Single(lane),
          None => LaneTagResolution::Unknown(tag),
        }
      }
      _ => LaneTagResolution::Conflicting(tags),
    }
  }
}

/// Outcome of parsing a `SemanticRecord`'s `provenance_refs` for lane
/// tags. See [`EvidenceLane::resolve_from_provenance_refs`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaneTagResolution {
  /// No `lane:` entry in `refs`.
  None,
  /// Exactly one distinct recognized lane tag.
  Single(EvidenceLane),
  /// Exactly one distinct tag, but unrecognized.
  Unknown(String),
  /// Two or more distinct lane tags. Fail-closed: caller's emitter
  /// has a bug or is being tampered with.
  Conflicting(Vec<String>),
}

/// Promotion gate: maps `JudgementAction::Accept` to either `Accepted` (internal
/// lanes) or `Candidate` (external lanes). All other actions map directly.
///
/// OWNER-LAW: external `Accept` is downgraded to `Candidate` because external
/// prose cannot bypass owner-law proof. To promote a candidate to `Accepted`
/// later, the caller must pass through owner-law proof + replay + negative/Held
/// proof and call this function again with `InternalOwnerLaw` /
/// `InternalAcceptedMemory` lane.
pub fn ontology_promote_with_lane(
  _policy: &EvaluationPolicy,
  lane: EvidenceLane,
  judgement: &JudgementRecord,
) -> PromotionDecision {
  let target_status = match judgement.action {
    JudgementAction::Accept => {
      if lane.allow_direct_accepted() {
        MeaningStatus::Accepted
      } else {
        // OWNER-LAW gate: external lane Accept → Candidate.
        MeaningStatus::Candidate
      }
    }
    JudgementAction::Reject => MeaningStatus::Rejected,
    JudgementAction::Hold => MeaningStatus::Held,
    JudgementAction::Contradict => MeaningStatus::Contradicted,
  };

  let reason = if matches!(judgement.action, JudgementAction::Accept)
    && !lane.allow_direct_accepted()
  {
    if matches!(lane, EvidenceLane::InternalDerivedReasoning) {
      Some(format!(
        "derived lane {:?} Accept → Candidate (owner-law: composed predicates are new semantic claims; Accepted parents do not transfer)",
        lane
      ))
    } else {
      Some(format!(
        "external lane {:?} Accept → Candidate (owner-law: external prose needs owner-law proof for Accepted)",
        lane
      ))
    }
  } else {
    Some(format!(
      "promotion derived from {:?} via lane {:?}",
      judgement.action, lane
    ))
  };

  PromotionDecision {
    id: format!("promote.{}", judgement.id),
    judgement: judgement.id.clone(),
    target_status,
    reason,
    artifact_refs: judgement.chosen_fact_refs.clone(),
  }
}

/// Backward-compatible promotion entry point. Defaults to the
/// `InternalOwnerLaw` lane for callers that have not been migrated yet.
///
/// **DEPRECATED for external evidence (2026-05-10):** callers passing untrusted
/// external evidence (web search, API, OCR, human prose, tool result) must
/// migrate to `ontology_promote_with_lane` with the matching `EvidenceLane`.
/// Calling this function for external evidence violates owner-law because it
/// allows `Accept` → `Accepted` without lane gating.
pub fn ontology_promote(
  policy: &EvaluationPolicy,
  judgement: &JudgementRecord,
) -> PromotionDecision {
  ontology_promote_with_lane(policy, EvidenceLane::InternalOwnerLaw, judgement)
}

/// Promote a `SemanticRecord` through the lane its provenance declares.
///
/// OWNER-LAW (2026-05-11): bridges the gap between provenance metadata
/// (a `lane:<Variant>` tag) and the typed `EvidenceLane` gate. Callers
/// that have a `SemanticRecord` in hand can promote correctly without
/// having to thread the lane through every layer — the lane is read
/// from the record's own provenance.
///
/// **Fail-closed**: if the record has no recognized `lane:` tag, the
/// function clamps to `TransducerOutput`, which is the most restrictive
/// "untrusted external" lane. This is intentional: unknown lane =
/// untrusted, never optimistically `InternalOwnerLaw`. The reason text
/// makes the fallback explicit so audit can spot it.
pub fn promote_record_via_provenance_lane(
  policy: &EvaluationPolicy,
  record: &SemanticRecord,
  judgement: &JudgementRecord,
) -> PromotionDecision {
  let resolution = EvidenceLane::resolve_from_provenance_refs(&record.provenance_refs);
  let (resolved, fail_closed_note): (EvidenceLane, Option<String>) = match &resolution {
    LaneTagResolution::Single(lane) => (*lane, None),
    LaneTagResolution::None => (
      EvidenceLane::TransducerOutput,
      Some("no recognized `lane:` tag in provenance".to_string()),
    ),
    LaneTagResolution::Unknown(tag) => (
      EvidenceLane::TransducerOutput,
      Some(format!("unknown `lane:` tag `{tag}` in provenance")),
    ),
    LaneTagResolution::Conflicting(tags) => (
      // Conflicting lane tags — multiple layers disagree on what the
      // record's lane is. Treat as a supply-chain smell and fail
      // closed to the most restrictive lane. Conflicting still picks
      // TransducerOutput (Candidate-clamp), not InternalOwnerLaw.
      EvidenceLane::TransducerOutput,
      Some(format!(
        "conflicting `lane:` tags in provenance: {:?}",
        tags
      )),
    ),
  };
  let mut decision = ontology_promote_with_lane(policy, resolved, judgement);
  if let Some(note) = fail_closed_note {
    let extra = format!(" [fail-closed: {note}; defaulted to {:?}]", resolved);
    decision.reason = Some(match decision.reason {
      Some(r) => r + &extra,
      None => extra,
    });
  }
  decision
}

/// Promote a batch of derived `SemanticRecord`s through their declared
/// lanes.
///
/// OWNER-LAW (2026-05-11): convenience for the `derive_internal_two_hop_*`
/// path. For each record in `records`, builds a default
/// `JudgementAction::Accept` judgement and routes it through
/// [`promote_record_via_provenance_lane`]. The lane tag on each record
/// drives whether `Accept` clamps to `Candidate` (e.g. records tagged
/// `lane:InternalDerivedReasoning` always clamp) or passes through.
///
/// Records with **no** lane tag, **unknown** lane tags, or
/// **conflicting** lane tags all fail-closed to `Candidate` via the
/// underlying `promote_record_via_provenance_lane` policy. The caller
/// receives one `PromotionDecision` per record in input order.
pub fn judge_derived_records(
  policy: &EvaluationPolicy,
  records: &[SemanticRecord],
) -> Vec<PromotionDecision> {
  records
    .iter()
    .map(|rec| {
      // OWNER-LAW (2026-05-11): chosen_fact_refs should be a
      // `ContextualFact.id` for audit/replay precision. Fall back to a
      // `record:<id>` marker when the inner value isn't a ContextualFact
      // or the fact has no id — `record:` prefix tells audit readers
      // that this is a record-level reference, not a meaning-level one.
      let fact_ref = match &rec.value {
        SemanticRecordValue::ContextualFact(f) => f.id.as_ref().map(|m| m.0.clone()),
        _ => None,
      };
      let chosen_fact_refs = match fact_ref {
        Some(id) => vec![id],
        None => vec![format!("record:{}", rec.id.0)],
      };
      let judgement = JudgementRecord {
        id: format!("judgement.derived.{}", rec.id.0),
        evaluation: format!("evaluation.derived.{}", rec.id.0),
        action: JudgementAction::Accept,
        chosen_interpretation: None,
        chosen_fact_refs,
        notes: vec![format!(
          "default Accept for derived record; lane gate decides clamp"
        )],
      };
      promote_record_via_provenance_lane(policy, rec, &judgement)
    })
    .collect()
}

/// Resolution of a `ContextualFact`'s dimension-check status, derived
/// from its provenance markers.
///
/// OWNER-LAW (2026-05-11): a formula fact carries `pred == "formula"`
/// and is flagged by the `formula.px` relation owner with the
/// `formula-dimension-check-required` provenance marker. Before that
/// fact can be promoted to `Accepted`, the host must run a dimension
/// check and tag one of:
///
/// - `formula-dimension-check:passed` — lhs and rhs dimensions agree
/// - `formula-dimension-check:failed` — dimensions disagree
/// - `formula-dimension-check:held` — dimensions cannot be determined
///   (missing unit info, opaque symbols, etc.)
///
/// Facts whose `pred != "formula"` or that lack the required marker
/// return `NotApplicable` — they are not subject to this gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormulaDimensionCheckResolution {
  /// Fact is not a formula (i.e. `pred != "formula"`), OR is a formula
  /// but carries an explicit bypass marker
  /// `formula-dimension-check-bypassed:<reason>` (e.g.
  /// `formula-dimension-check-bypassed:owner-law-proofed`). The gate
  /// does not apply.
  ///
  /// **Tightened 2026-05-11 (review feedback)**: formula predicate
  /// alone is enough to fire the gate. Previously a `pred=="formula"`
  /// fact with no `formula-dimension-check-required` marker was also
  /// `NotApplicable`, which let any caller bypass the gate by simply
  /// omitting the marker. Now omitting the marker means `Missing` —
  /// the only way to bypass is the explicit bypass marker with a
  /// reason that audit can inspect.
  NotApplicable,
  /// Dimension check ran and lhs/rhs dimensions agree.
  Passed,
  /// Dimension check ran and lhs/rhs dimensions disagree.
  Failed,
  /// Dimension check could not be conclusively run (missing units,
  /// opaque symbols, etc.). Treat as not-yet-proven.
  Held,
  /// Formula fact with no result marker. The host has not run the
  /// dimension check yet, or the result was lost. Promotion must
  /// clamp to Candidate.
  Missing,
}

/// Inspect `f.provenance_refs` and decide the formula-dimension-check
/// resolution. Pure function — does not mutate the fact.
///
/// OWNER-LAW (2026-05-11, tightened): the gate fires on the formula
/// predicate itself, not on a `required` flag. Documented provenance
/// markers are:
///
///   - bypassed (explicit, with audit reason):
///       `formula-dimension-check-bypassed:<reason>`
///       e.g. `formula-dimension-check-bypassed:owner-law-proofed`
///   - required (informational; emitted by the relation classifier
///     but no longer load-bearing — the gate fires on `pred=="formula"`
///     regardless):
///       `formula-dimension-check-required`
///   - result markers (exactly one expected):
///       `formula-dimension-check:passed`
///       `formula-dimension-check:failed`
///       `formula-dimension-check:held`
///
/// **Decision tree**:
///   1. `pred != "formula"` → `NotApplicable`
///   2. `pred == "formula"` and carries any
///      `formula-dimension-check-bypassed:*` marker → `NotApplicable`
///   3. Otherwise inspect result markers:
///      - exactly one of `:passed`/`:failed`/`:held` → the
///        corresponding variant
///      - none → `Missing`
///      - any contradictory combination (multiple result markers) →
///        `Held` (fail-closed, same shape as
///        `LaneTagResolution::Conflicting`)
pub fn resolve_formula_dimension_check(f: &ContextualFact) -> FormulaDimensionCheckResolution {
  if f.pred != "formula" {
    return FormulaDimensionCheckResolution::NotApplicable;
  }
  // Explicit bypass — must carry a reason after the colon. Any
  // `formula-dimension-check-bypassed:<reason>` provenance ref is a
  // signal that a downstream owner-law has proven the formula by other
  // means (e.g. unitless mathematical identity). Audit can inspect
  // `<reason>` because the full ref is preserved in provenance.
  let has_bypass = f
    .provenance_refs
    .iter()
    .any(|p| p.starts_with("formula-dimension-check-bypassed:"));
  if has_bypass {
    return FormulaDimensionCheckResolution::NotApplicable;
  }
  let has_passed = f
    .provenance_refs
    .iter()
    .any(|p| p == "formula-dimension-check:passed");
  let has_failed = f
    .provenance_refs
    .iter()
    .any(|p| p == "formula-dimension-check:failed");
  let has_held = f
    .provenance_refs
    .iter()
    .any(|p| p == "formula-dimension-check:held");

  match (has_passed, has_failed, has_held) {
    (true, false, false) => FormulaDimensionCheckResolution::Passed,
    (false, true, false) => FormulaDimensionCheckResolution::Failed,
    (false, false, true) => FormulaDimensionCheckResolution::Held,
    (false, false, false) => FormulaDimensionCheckResolution::Missing,
    // Any combination of multiple result markers is contradictory ->
    // fail-closed to Held so promotion clamps.
    _ => FormulaDimensionCheckResolution::Held,
  }
}

/// Judge a batch of `SemanticRecord`s through the formula
/// dimension-check gate.
///
/// OWNER-LAW (2026-05-11): parallel to `judge_derived_records`, but
/// the gate here is a *structural fact predicate* rather than a
/// provenance lane. For each record:
///
/// 1. If the record's `ContextualFact` has
///    `resolve_formula_dimension_check == Passed`, build a default
///    `Accept` judgement and route it through
///    `promote_record_via_provenance_lane` (so any lane-tag clamp
///    *also* applies — formula and lane are stacking gates, not
///    alternative gates).
/// 2. Otherwise (Failed, Held, Missing, or non-formula records: keep
///    going through the lane gate without the formula clamp), the
///    decision is built with the same lane-aware promote but the
///    `reason` is appended with the formula-gate verdict.
/// 3. For `Failed` / `Missing`, the resulting decision is clamped to
///    `Candidate` regardless of lane outcome — Accept is never
///    permitted for an unproven formula.
///
/// Records that aren't formula facts return whatever the lane gate
/// decides, unchanged. Non-`ContextualFact` records (Interpretation,
/// Judgement, etc.) are not subject to the formula gate.
pub fn judge_formula_records(
  policy: &EvaluationPolicy,
  records: &[SemanticRecord],
) -> Vec<PromotionDecision> {
  records
    .iter()
    .map(|rec| {
      let formula_res = match &rec.value {
        SemanticRecordValue::ContextualFact(f) => resolve_formula_dimension_check(f),
        _ => FormulaDimensionCheckResolution::NotApplicable,
      };
      let fact_ref = match &rec.value {
        SemanticRecordValue::ContextualFact(f) => f.id.as_ref().map(|m| m.0.clone()),
        _ => None,
      };
      let chosen_fact_refs = match fact_ref {
        Some(id) => vec![id],
        None => vec![format!("record:{}", rec.id.0)],
      };
      let judgement = JudgementRecord {
        id: format!("judgement.formula.{}", rec.id.0),
        evaluation: format!("evaluation.formula.{}", rec.id.0),
        action: JudgementAction::Accept,
        chosen_interpretation: None,
        chosen_fact_refs,
        notes: vec![format!(
          "default Accept for formula record; dimension-check gate decides clamp"
        )],
      };
      let mut decision = promote_record_via_provenance_lane(policy, rec, &judgement);
      // Fail-closed: Failed / Missing / Held → clamp to Candidate even
      // if the lane gate would have allowed Accepted. Passed leaves the
      // lane decision intact.
      let needs_clamp = matches!(
        formula_res,
        FormulaDimensionCheckResolution::Failed
          | FormulaDimensionCheckResolution::Missing
          | FormulaDimensionCheckResolution::Held
      );
      if needs_clamp && matches!(decision.target_status, MeaningStatus::Accepted) {
        decision.target_status = MeaningStatus::Candidate;
      }
      // Always append the formula-gate verdict to the reason so audit
      // can see *why* the gate applied (or didn't).
      let gate_note = match formula_res {
        FormulaDimensionCheckResolution::NotApplicable => None,
        FormulaDimensionCheckResolution::Passed => Some("formula-dimension-check:passed"),
        FormulaDimensionCheckResolution::Failed => {
          Some("formula-dimension-check:failed [clamped to Candidate]")
        }
        FormulaDimensionCheckResolution::Held => {
          Some("formula-dimension-check:held [clamped to Candidate]")
        }
        FormulaDimensionCheckResolution::Missing => {
          Some("formula-dimension-check:missing [clamped to Candidate]")
        }
      };
      if let Some(note) = gate_note {
        let extra = format!(" [formula-gate: {note}]");
        decision.reason = Some(match decision.reason {
          Some(r) => r + &extra,
          None => extra,
        });
      }
      decision
    })
    .collect()
}

#[cfg(test)]
mod evidence_lane_tests {
  use super::*;

  fn accept_judgement() -> JudgementRecord {
    JudgementRecord {
      id: "j.test.1".to_string(),
      evaluation: "e.test.1".to_string(),
      action: JudgementAction::Accept,
      chosen_interpretation: None,
      chosen_fact_refs: vec![],
      notes: vec![],
    }
  }

  #[test]
  fn internal_owner_law_allows_direct_accepted() {
    assert!(EvidenceLane::InternalOwnerLaw.allow_direct_accepted());
    let policy = EvaluationPolicy::default();
    let promotion =
      ontology_promote_with_lane(&policy, EvidenceLane::InternalOwnerLaw, &accept_judgement());
    assert_eq!(promotion.target_status, MeaningStatus::Accepted);
  }

  #[test]
  fn internal_accepted_memory_allows_direct_accepted() {
    assert!(EvidenceLane::InternalAcceptedMemory.allow_direct_accepted());
    let policy = EvaluationPolicy::default();
    let promotion = ontology_promote_with_lane(
      &policy,
      EvidenceLane::InternalAcceptedMemory,
      &accept_judgement(),
    );
    assert_eq!(promotion.target_status, MeaningStatus::Accepted);
  }

  #[test]
  fn internal_derived_reasoning_clamps_to_candidate() {
    // OWNER-LAW invariant: composed predicates are new semantic claims.
    // Even when parent facts are Accepted, the composition Accept must
    // be clamped to Candidate until a proof of the composed predicate
    // exists.
    assert!(
      !EvidenceLane::InternalDerivedReasoning.allow_direct_accepted(),
      "InternalDerivedReasoning must NOT allow direct Accepted"
    );
    let policy = EvaluationPolicy::default();
    let promotion = ontology_promote_with_lane(
      &policy,
      EvidenceLane::InternalDerivedReasoning,
      &accept_judgement(),
    );
    assert_eq!(
      promotion.target_status,
      MeaningStatus::Candidate,
      "Accept on derived lane must clamp to Candidate"
    );
    // Reason text must say "derived" — distinguishes from external prose
    // clamping, so audit can tell which kind of untrusted lane fired.
    let reason = promotion.reason.expect("reason present");
    assert!(
      reason.contains("derived") || reason.contains("Derived"),
      "derived-lane reason must mention `derived`, got: {reason}"
    );
    assert!(
      reason.contains("composed"),
      "derived-lane reason must mention `composed`, got: {reason}"
    );
  }

  #[test]
  fn external_lanes_clamp_to_candidate() {
    let policy = EvaluationPolicy::default();
    for lane in [
      EvidenceLane::ExternalWebSearch,
      EvidenceLane::ExternalApi,
      EvidenceLane::TransducerOutput,
      EvidenceLane::HumanProvidedProse,
      EvidenceLane::ToolExecutionResult,
      EvidenceLane::PeerEvidence,
    ] {
      assert!(
        !lane.allow_direct_accepted(),
        "{lane:?} must NOT allow direct Accepted"
      );
      let promotion = ontology_promote_with_lane(&policy, lane, &accept_judgement());
      assert_eq!(
        promotion.target_status,
        MeaningStatus::Candidate,
        "{lane:?} Accept must clamp to Candidate"
      );
    }
  }

  #[test]
  fn lane_from_tag_recognizes_all_variants() {
    let all = [
      ("InternalOwnerLaw", EvidenceLane::InternalOwnerLaw),
      (
        "InternalAcceptedMemory",
        EvidenceLane::InternalAcceptedMemory,
      ),
      ("ExternalWebSearch", EvidenceLane::ExternalWebSearch),
      ("ExternalApi", EvidenceLane::ExternalApi),
      ("TransducerOutput", EvidenceLane::TransducerOutput),
      ("HumanProvidedProse", EvidenceLane::HumanProvidedProse),
      ("ToolExecutionResult", EvidenceLane::ToolExecutionResult),
      ("PeerEvidence", EvidenceLane::PeerEvidence),
      (
        "InternalDerivedReasoning",
        EvidenceLane::InternalDerivedReasoning,
      ),
    ];
    for (tag, expected) in all {
      assert_eq!(EvidenceLane::from_tag(tag), Some(expected), "tag={tag}");
    }
    assert_eq!(EvidenceLane::from_tag("UnknownLane"), None);
    assert_eq!(EvidenceLane::from_tag(""), None);
  }

  #[test]
  fn lane_from_provenance_refs_finds_first_tag() {
    let refs = vec![
      "owner-law:stdlib/lib/gate/reasoning/relation-composition.px".to_string(),
      "lane:InternalDerivedReasoning".to_string(),
      "derived-from-pred1:has-mass".to_string(),
    ];
    assert_eq!(
      EvidenceLane::from_provenance_refs(&refs),
      Some(EvidenceLane::InternalDerivedReasoning)
    );
  }

  #[test]
  fn lane_from_provenance_refs_returns_none_when_no_lane_tag() {
    let refs = vec![
      "owner-law:something.px".to_string(),
      "source-url:https://example.com".to_string(),
    ];
    assert_eq!(EvidenceLane::from_provenance_refs(&refs), None);
  }

  #[test]
  fn lane_from_provenance_refs_returns_none_for_unknown_tag() {
    let refs = vec!["lane:WhateverFancyName".to_string()];
    assert_eq!(EvidenceLane::from_provenance_refs(&refs), None);
  }

  fn make_record(lane_tag: Option<&str>) -> SemanticRecord {
    let mut refs = vec!["owner-law:test.px".to_string()];
    if let Some(t) = lane_tag {
      refs.push(format!("lane:{}", t));
    }
    SemanticRecord {
      id: SemanticRecordId::from("record.test.1".to_string()),
      episode: SemanticEpisodeId::from("episode.test.1".to_string()),
      record_kind: SemanticRecordKind::ContextualFact,
      value: SemanticRecordValue::ContextualFact(ContextualFact {
        id: Some(MeaningId::from("m.test.1".to_string())),
        context: ContextId::from("Test"),
        layer: LayerId::from("L1"),
        subj: "a".to_string(),
        pred: "p".to_string(),
        obj: "b".to_string(),
        status: MeaningStatus::Candidate,
        confidence: 0.5,
        provenance_refs: vec![],
        proof_refs: vec![],
        contradiction_refs: vec![],
        loss: None,
        timestamp: None,
      }),
      provenance_refs: refs,
      artifact_refs: vec![],
    }
  }

  #[test]
  fn promote_record_routes_derived_lane_and_clamps_to_candidate() {
    let policy = EvaluationPolicy::default();
    let record = make_record(Some("InternalDerivedReasoning"));
    let decision = promote_record_via_provenance_lane(&policy, &record, &accept_judgement());
    assert_eq!(
      decision.target_status,
      MeaningStatus::Candidate,
      "InternalDerivedReasoning-tagged record must clamp Accept to Candidate"
    );
    let reason = decision.reason.expect("reason present");
    assert!(reason.contains("composed"));
    assert!(
      !reason.contains("fail-closed"),
      "recognized lane must NOT trigger fail-closed reason; got: {reason}"
    );
  }

  #[test]
  fn promote_record_routes_internal_accepted_memory_to_accepted() {
    let policy = EvaluationPolicy::default();
    let record = make_record(Some("InternalAcceptedMemory"));
    let decision = promote_record_via_provenance_lane(&policy, &record, &accept_judgement());
    assert_eq!(decision.target_status, MeaningStatus::Accepted);
  }

  #[test]
  fn promote_record_fails_closed_when_lane_tag_missing() {
    let policy = EvaluationPolicy::default();
    let record = make_record(None);
    let decision = promote_record_via_provenance_lane(&policy, &record, &accept_judgement());
    // Fail-closed: defaults to TransducerOutput → Candidate, NOT Accepted.
    assert_eq!(
      decision.target_status,
      MeaningStatus::Candidate,
      "missing lane tag must fail-closed to Candidate, not Accepted"
    );
    let reason = decision.reason.expect("reason present");
    assert!(
      reason.contains("fail-closed"),
      "fail-closed fallback must be visible in reason; got: {reason}"
    );
    assert!(reason.contains("TransducerOutput"));
  }

  #[test]
  fn resolve_lane_tags_none() {
    let refs = vec!["owner-law:something.px".to_string()];
    assert_eq!(
      EvidenceLane::resolve_from_provenance_refs(&refs),
      LaneTagResolution::None
    );
  }

  #[test]
  fn resolve_lane_tags_single_recognized() {
    let refs = vec![
      "owner-law:x.px".to_string(),
      "lane:InternalDerivedReasoning".to_string(),
    ];
    assert_eq!(
      EvidenceLane::resolve_from_provenance_refs(&refs),
      LaneTagResolution::Single(EvidenceLane::InternalDerivedReasoning)
    );
  }

  #[test]
  fn resolve_lane_tags_duplicate_same_tag_is_single() {
    // `lane:X` twice → same tag, deduped, not a conflict.
    let refs = vec![
      "lane:InternalDerivedReasoning".to_string(),
      "lane:InternalDerivedReasoning".to_string(),
    ];
    assert_eq!(
      EvidenceLane::resolve_from_provenance_refs(&refs),
      LaneTagResolution::Single(EvidenceLane::InternalDerivedReasoning)
    );
  }

  #[test]
  fn resolve_lane_tags_unknown() {
    let refs = vec!["lane:WhateverFancy".to_string()];
    match EvidenceLane::resolve_from_provenance_refs(&refs) {
      LaneTagResolution::Unknown(tag) => assert_eq!(tag, "WhateverFancy"),
      other => panic!("expected Unknown, got {:?}", other),
    }
  }

  #[test]
  fn resolve_lane_tags_conflicting_distinct_recognized() {
    let refs = vec![
      "lane:InternalDerivedReasoning".to_string(),
      "lane:InternalAcceptedMemory".to_string(),
    ];
    match EvidenceLane::resolve_from_provenance_refs(&refs) {
      LaneTagResolution::Conflicting(tags) => {
        assert_eq!(tags.len(), 2);
        assert!(tags.contains(&"InternalDerivedReasoning".to_string()));
        assert!(tags.contains(&"InternalAcceptedMemory".to_string()));
      }
      other => panic!("expected Conflicting, got {:?}", other),
    }
  }

  #[test]
  fn resolve_lane_tags_conflicting_includes_unknown() {
    // Two distinct tags, one unknown — still Conflicting (fail-closed).
    let refs = vec![
      "lane:InternalDerivedReasoning".to_string(),
      "lane:Bogus".to_string(),
    ];
    assert!(matches!(
      EvidenceLane::resolve_from_provenance_refs(&refs),
      LaneTagResolution::Conflicting(_)
    ));
  }

  #[test]
  fn promote_record_fails_closed_on_conflicting_lane_tags() {
    let policy = EvaluationPolicy::default();
    let mut record = make_record(Some("InternalDerivedReasoning"));
    // Inject a second, conflicting lane tag.
    record
      .provenance_refs
      .push("lane:InternalAcceptedMemory".to_string());
    let decision = promote_record_via_provenance_lane(&policy, &record, &accept_judgement());
    assert_eq!(
      decision.target_status,
      MeaningStatus::Candidate,
      "conflicting lane tags must fail-closed to Candidate even when both tags are otherwise 'internal'"
    );
    let reason = decision.reason.expect("reason present");
    assert!(reason.contains("conflicting"));
    assert!(reason.contains("fail-closed"));
    assert!(reason.contains("TransducerOutput"));
  }

  #[test]
  fn judge_derived_records_clamps_each_to_candidate() {
    let policy = EvaluationPolicy::default();
    let records = vec![
      make_record(Some("InternalDerivedReasoning")),
      make_record(Some("InternalDerivedReasoning")),
    ];
    let decisions = judge_derived_records(&policy, &records);
    assert_eq!(decisions.len(), 2);
    for d in &decisions {
      assert_eq!(
        d.target_status,
        MeaningStatus::Candidate,
        "every derived-tagged record's Accept must clamp to Candidate"
      );
    }
  }

  #[test]
  fn judge_derived_records_fails_closed_on_missing_lane_tag() {
    let policy = EvaluationPolicy::default();
    let records = vec![make_record(None)];
    let decisions = judge_derived_records(&policy, &records);
    assert_eq!(decisions.len(), 1);
    assert_eq!(decisions[0].target_status, MeaningStatus::Candidate);
    let reason = decisions[0].reason.as_ref().expect("reason");
    assert!(reason.contains("fail-closed"));
  }

  #[test]
  fn promote_record_fails_closed_when_lane_tag_unknown() {
    let policy = EvaluationPolicy::default();
    let record = make_record(Some("MyNewFancyLane"));
    let decision = promote_record_via_provenance_lane(&policy, &record, &accept_judgement());
    assert_eq!(
      decision.target_status,
      MeaningStatus::Candidate,
      "unknown lane tag must fail-closed to Candidate"
    );
    let reason = decision.reason.expect("reason present");
    assert!(reason.contains("fail-closed"));
  }

  #[test]
  fn non_accept_actions_pass_through_unchanged_on_all_lanes() {
    let policy = EvaluationPolicy::default();
    let cases = [
      (JudgementAction::Reject, MeaningStatus::Rejected),
      (JudgementAction::Hold, MeaningStatus::Held),
      (JudgementAction::Contradict, MeaningStatus::Contradicted),
    ];
    for (action, expected) in cases {
      let j = JudgementRecord {
        action: action.clone(),
        ..accept_judgement()
      };
      // derived lane must not interfere with Reject / Hold / Contradict
      let promotion =
        ontology_promote_with_lane(&policy, EvidenceLane::InternalDerivedReasoning, &j);
      assert_eq!(
        promotion.target_status, expected,
        "{action:?} on derived lane must pass through unchanged"
      );
    }
  }

  // ─── formula dimension-check gate ────────────────────────────────────

  fn formula_record(formula_provenance: &[&str], lane_tag: Option<&str>) -> SemanticRecord {
    let mut rec_provenance = vec!["owner-law:test.px".to_string()];
    if let Some(t) = lane_tag {
      rec_provenance.push(format!("lane:{}", t));
    }
    let fact_provenance: Vec<String> = formula_provenance.iter().map(|s| s.to_string()).collect();
    SemanticRecord {
      id: SemanticRecordId::from("record.formula.1".to_string()),
      episode: SemanticEpisodeId::from("episode.formula.1".to_string()),
      record_kind: SemanticRecordKind::ContextualFact,
      value: SemanticRecordValue::ContextualFact(ContextualFact {
        id: Some(MeaningId::from("m.formula.1".to_string())),
        context: ContextId::from("Physics"),
        layer: LayerId::from("L2"),
        subj: "F".to_string(),
        pred: "formula".to_string(),
        obj: "m*a".to_string(),
        status: MeaningStatus::Candidate,
        confidence: 0.5,
        provenance_refs: fact_provenance,
        proof_refs: vec![],
        contradiction_refs: vec![],
        loss: None,
        timestamp: None,
      }),
      provenance_refs: rec_provenance,
      artifact_refs: vec![],
    }
  }

  fn extract_fact(rec: &SemanticRecord) -> &ContextualFact {
    match &rec.value {
      SemanticRecordValue::ContextualFact(f) => f,
      _ => panic!("not a ContextualFact record"),
    }
  }

  #[test]
  fn resolve_formula_dimension_check_returns_not_applicable_for_non_formula() {
    let rec = make_record(None);
    let f = extract_fact(&rec);
    assert_eq!(
      resolve_formula_dimension_check(f),
      FormulaDimensionCheckResolution::NotApplicable,
      "non-formula fact must not be subject to the gate"
    );
  }

  #[test]
  fn resolve_formula_dimension_check_returns_missing_without_any_marker() {
    // Tightened 2026-05-11 (review feedback): a `pred=="formula"`
    // fact with no markers at all must be `Missing`, not
    // `NotApplicable`. Otherwise a caller could bypass the gate by
    // simply omitting the `formula-dimension-check-required` marker.
    let rec = formula_record(&[], None);
    assert_eq!(
      resolve_formula_dimension_check(extract_fact(&rec)),
      FormulaDimensionCheckResolution::Missing,
      "formula predicate with no markers must be Missing (gate fires on pred alone)"
    );
  }

  #[test]
  fn resolve_formula_dimension_check_honors_explicit_bypass_marker() {
    // The only way to make a formula fact `NotApplicable` is the
    // explicit bypass marker with an audit reason. This is the
    // owner-law-proofed escape hatch.
    let rec = formula_record(
      &["formula-dimension-check-bypassed:owner-law-proofed"],
      None,
    );
    assert_eq!(
      resolve_formula_dimension_check(extract_fact(&rec)),
      FormulaDimensionCheckResolution::NotApplicable,
      "explicit bypass marker must make the gate NotApplicable"
    );
  }

  #[test]
  fn resolve_formula_dimension_check_bypass_takes_priority_over_result_markers() {
    // If both bypass and result markers are present, bypass wins —
    // a downstream owner-law has explicitly declared the gate doesn't
    // apply, and we trust that over any leftover marker noise.
    let rec = formula_record(
      &[
        "formula-dimension-check-bypassed:unitless-identity",
        "formula-dimension-check:failed",
      ],
      None,
    );
    assert_eq!(
      resolve_formula_dimension_check(extract_fact(&rec)),
      FormulaDimensionCheckResolution::NotApplicable
    );
  }

  #[test]
  fn resolve_formula_dimension_check_still_returns_missing_when_only_required_marker_present() {
    // The informational `formula-dimension-check-required` marker on
    // its own no longer changes the verdict — Missing is still
    // Missing because no result has been recorded. This documents the
    // marker as informational under the tightened semantics.
    let rec = formula_record(&["formula-dimension-check-required"], None);
    assert_eq!(
      resolve_formula_dimension_check(extract_fact(&rec)),
      FormulaDimensionCheckResolution::Missing
    );
  }

  #[test]
  fn resolve_formula_dimension_check_returns_passed_on_passed_marker() {
    let rec = formula_record(
      &[
        "formula-dimension-check-required",
        "formula-dimension-check:passed",
      ],
      None,
    );
    assert_eq!(
      resolve_formula_dimension_check(extract_fact(&rec)),
      FormulaDimensionCheckResolution::Passed
    );
  }

  #[test]
  fn resolve_formula_dimension_check_returns_failed_on_failed_marker() {
    let rec = formula_record(
      &[
        "formula-dimension-check-required",
        "formula-dimension-check:failed",
      ],
      None,
    );
    assert_eq!(
      resolve_formula_dimension_check(extract_fact(&rec)),
      FormulaDimensionCheckResolution::Failed
    );
  }

  #[test]
  fn resolve_formula_dimension_check_returns_held_on_held_marker() {
    let rec = formula_record(
      &[
        "formula-dimension-check-required",
        "formula-dimension-check:held",
      ],
      None,
    );
    assert_eq!(
      resolve_formula_dimension_check(extract_fact(&rec)),
      FormulaDimensionCheckResolution::Held
    );
  }

  #[test]
  fn resolve_formula_dimension_check_returns_held_on_contradictory_markers() {
    // Both passed and failed present → fail-closed to Held.
    let rec = formula_record(
      &[
        "formula-dimension-check-required",
        "formula-dimension-check:passed",
        "formula-dimension-check:failed",
      ],
      None,
    );
    assert_eq!(
      resolve_formula_dimension_check(extract_fact(&rec)),
      FormulaDimensionCheckResolution::Held
    );
  }

  #[test]
  fn judge_formula_records_clamps_missing_to_candidate_under_owner_law_lane() {
    // Record carries `lane:InternalOwnerLaw` (the only lane where Accept
    // would normally pass through). Without the formula gate, this
    // would promote to Accepted. With the gate, missing dimension check
    // forces Candidate.
    let policy = EvaluationPolicy::default();
    let rec = formula_record(
      &["formula-dimension-check-required"],
      Some("InternalOwnerLaw"),
    );
    let decisions = judge_formula_records(&policy, &[rec]);
    assert_eq!(decisions.len(), 1);
    assert_eq!(
      decisions[0].target_status,
      MeaningStatus::Candidate,
      "missing dimension check must clamp Accept to Candidate"
    );
    let reason = decisions[0].reason.as_ref().expect("reason present");
    assert!(
      reason.contains("formula-gate") && reason.contains("missing"),
      "audit reason must record the formula-gate verdict; got: {reason}"
    );
  }

  #[test]
  fn judge_formula_records_clamps_failed_to_candidate_under_owner_law_lane() {
    let policy = EvaluationPolicy::default();
    let rec = formula_record(
      &[
        "formula-dimension-check-required",
        "formula-dimension-check:failed",
      ],
      Some("InternalOwnerLaw"),
    );
    let decisions = judge_formula_records(&policy, &[rec]);
    assert_eq!(decisions[0].target_status, MeaningStatus::Candidate);
    assert!(decisions[0]
      .reason
      .as_ref()
      .expect("reason")
      .contains("failed"));
  }

  #[test]
  fn judge_formula_records_clamps_held_to_candidate_under_owner_law_lane() {
    let policy = EvaluationPolicy::default();
    let rec = formula_record(
      &[
        "formula-dimension-check-required",
        "formula-dimension-check:held",
      ],
      Some("InternalOwnerLaw"),
    );
    let decisions = judge_formula_records(&policy, &[rec]);
    assert_eq!(decisions[0].target_status, MeaningStatus::Candidate);
    assert!(decisions[0]
      .reason
      .as_ref()
      .expect("reason")
      .contains("held"));
  }

  #[test]
  fn judge_formula_records_passes_when_dimension_check_passed() {
    // Lane gates and formula gates stack. With `passed`, the formula
    // gate doesn't clamp; the lane gate (InternalOwnerLaw here) decides.
    let policy = EvaluationPolicy::default();
    let rec = formula_record(
      &[
        "formula-dimension-check-required",
        "formula-dimension-check:passed",
      ],
      Some("InternalOwnerLaw"),
    );
    let decisions = judge_formula_records(&policy, &[rec]);
    assert_eq!(
      decisions[0].target_status,
      MeaningStatus::Accepted,
      "InternalOwnerLaw + passed dimension check must promote to Accepted"
    );
    let reason = decisions[0].reason.as_ref().expect("reason");
    assert!(
      reason.contains("formula-gate") && reason.contains("passed"),
      "audit reason must still record the gate verdict on Passed; got: {reason}"
    );
  }

  #[test]
  fn judge_formula_records_stacks_with_derived_lane_clamp() {
    // Formula fact carrying `lane:InternalDerivedReasoning` — the lane
    // gate alone would clamp to Candidate. Even with a `passed` dim
    // check, the result is still Candidate because lane is the stricter
    // gate. This proves stacking: passing one gate does not bypass
    // another.
    let policy = EvaluationPolicy::default();
    let rec = formula_record(
      &[
        "formula-dimension-check-required",
        "formula-dimension-check:passed",
      ],
      Some("InternalDerivedReasoning"),
    );
    let decisions = judge_formula_records(&policy, &[rec]);
    assert_eq!(
      decisions[0].target_status,
      MeaningStatus::Candidate,
      "derived lane clamps even when formula gate passes — gates stack"
    );
  }

  #[test]
  fn judge_formula_records_clamps_unmarked_formula_under_owner_law_lane() {
    // Tightened 2026-05-11 (review feedback): a `pred=="formula"`
    // record under `InternalOwnerLaw` with ZERO formula markers must
    // still clamp to Candidate. Otherwise, omitting the
    // `formula-dimension-check-required` marker would silently bypass
    // the gate.
    let policy = EvaluationPolicy::default();
    let rec = formula_record(&[], Some("InternalOwnerLaw"));
    let decisions = judge_formula_records(&policy, &[rec]);
    assert_eq!(
      decisions[0].target_status,
      MeaningStatus::Candidate,
      "formula predicate with no markers must clamp even under InternalOwnerLaw"
    );
    let reason = decisions[0].reason.as_ref().expect("reason present");
    assert!(reason.contains("missing"));
  }

  #[test]
  fn judge_formula_records_allows_explicit_bypass_under_owner_law_lane() {
    // The escape hatch: explicit bypass with an audit reason lets the
    // formula gate stand down. The lane gate (InternalOwnerLaw) then
    // gets to promote to Accepted normally.
    let policy = EvaluationPolicy::default();
    let rec = formula_record(
      &["formula-dimension-check-bypassed:owner-law-proofed"],
      Some("InternalOwnerLaw"),
    );
    let decisions = judge_formula_records(&policy, &[rec]);
    assert_eq!(
      decisions[0].target_status,
      MeaningStatus::Accepted,
      "explicit bypass marker must let the formula gate stand down"
    );
  }

  #[test]
  fn judge_formula_records_does_not_affect_non_formula_records() {
    // A non-formula record under InternalOwnerLaw passes the formula
    // gate as NotApplicable, and the lane decides Accepted as usual.
    let policy = EvaluationPolicy::default();
    let rec = make_record(Some("InternalOwnerLaw"));
    let decisions = judge_formula_records(&policy, &[rec]);
    assert_eq!(decisions[0].target_status, MeaningStatus::Accepted);
  }
}

pub fn semantic_ingest_envelope_from_tool_execution(
  capability: Option<&ToolCapabilityRecord>,
  plan: &ToolActionPlan,
  result: &ToolExecutionResult,
) -> SemanticIngestEnvelope {
  let episode_id = SemanticEpisodeId::from(format!("episode.{}", result.id.0));
  let mut records = Vec::new();

  if let Some(capability) = capability {
    records.push(SemanticRecord {
      id: SemanticRecordId::from(format!("record.capability.{}", capability.id.0)),
      episode: episode_id.clone(),
      record_kind: SemanticRecordKind::ToolCapability,
      provenance_refs: vec![],
      artifact_refs: vec![],
      value: SemanticRecordValue::ToolCapability(capability.clone()),
    });
  }

  records.push(SemanticRecord {
    id: SemanticRecordId::from(format!("record.plan.{}", plan.id.0)),
    episode: episode_id.clone(),
    record_kind: SemanticRecordKind::ToolActionPlan,
    provenance_refs: plan.provenance_refs.clone(),
    artifact_refs: vec![],
    value: SemanticRecordValue::ToolActionPlan(plan.clone()),
  });

  records.push(SemanticRecord {
    id: SemanticRecordId::from(format!("record.result.{}", result.id.0)),
    episode: episode_id.clone(),
    record_kind: SemanticRecordKind::ToolExecutionResult,
    provenance_refs: result.provenance_refs.clone(),
    artifact_refs: result.artifact_refs.clone(),
    value: SemanticRecordValue::ToolExecutionResult(result.clone()),
  });

  for (index, projection) in result.expression_projections.iter().enumerate() {
    records.push(SemanticRecord {
      id: SemanticRecordId::from(format!("record.expression.{}.{}", result.id.0, index)),
      episode: episode_id.clone(),
      record_kind: SemanticRecordKind::ExpressionProjection,
      provenance_refs: projection.provenance_refs.clone(),
      artifact_refs: projection.artifact_refs.clone(),
      value: SemanticRecordValue::ExpressionProjection(projection.clone()),
    });
  }

  for (index, fact) in result.semantic_facts.iter().enumerate() {
    records.push(SemanticRecord {
      id: SemanticRecordId::from(format!("record.fact.{}.{}", result.id.0, index)),
      episode: episode_id.clone(),
      record_kind: SemanticRecordKind::ContextualFact,
      provenance_refs: fact.provenance_refs.clone(),
      artifact_refs: fact.proof_refs.clone(),
      value: SemanticRecordValue::ContextualFact(fact.clone()),
    });
  }

  let episode = SemanticEpisode {
    id: episode_id,
    observation_refs: result.provenance_refs.clone(),
    record_refs: records.iter().map(|record| record.id.clone()).collect(),
    chosen_interpretation: plan.chosen_interpretation.clone(),
    judgement_ref: Some(plan.judgement.clone()),
    promotion_ref: None,
    summary: result.summary.clone(),
  };

  SemanticIngestEnvelope {
    observation_refs: result.provenance_refs.clone(),
    records,
    episode,
    knowledge_records: vec![],
    notes: vec![
      "tool execution lowered into canonical semantic ingest carrier".to_string(),
      "tool adapter names remain private operational vocabulary".to_string(),
    ],
  }
}

pub fn dialogue_transcript_from_ingest(envelope: &SemanticIngestEnvelope) -> Vec<String> {
  let mut transcript = envelope
    .notes
    .iter()
    .filter_map(|note| note.strip_prefix("transcript:"))
    .map(str::trim)
    .filter(|line| !line.is_empty())
    .map(ToOwned::to_owned)
    .collect::<Vec<_>>();
  if !transcript.is_empty() {
    return transcript;
  }

  if let Some(utterance) = envelope
    .observation_refs
    .iter()
    .find_map(|reference| reference.strip_prefix("utterance:"))
    .map(str::trim)
    .filter(|utterance| !utterance.is_empty())
  {
    transcript.push(format!("user: {utterance}"));
  }

  if let Some(summary) = envelope
    .episode
    .summary
    .as_deref()
    .map(str::trim)
    .filter(|summary| !summary.is_empty())
  {
    transcript.push(format!("doghouse: {summary}"));
  }

  transcript
}

pub fn follow_up_hint_from_ingest(envelope: &SemanticIngestEnvelope) -> Option<String> {
  let contextual_facts: Vec<&ContextualFact> = envelope
    .records
    .iter()
    .filter_map(|record| match &record.value {
      SemanticRecordValue::ContextualFact(fact) => Some(fact),
      _ => None,
    })
    .collect();
  let concept_query_term = contextual_facts
    .iter()
    .find(|fact| fact.pred == "concept-query-term" && !fact.obj.trim().is_empty())
    .map(|fact| fact.obj.trim());

  let normalize_held_subject = |raw: &str| -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() || matches!(trimmed, "doghouse" | "user" | "question") {
      concept_query_term.unwrap_or(trimmed).to_string()
    } else {
      trimmed.to_string()
    }
  };

  let is_generic_context_flag = |raw: &str| matches!(raw.trim(), "" | "true" | "false" | "1" | "0");

  // P7: held에서 "무엇이 부족한지" 구체적으로 추출
  let requires_context = contextual_facts
    .iter()
    .find(|fact| fact.pred == "requires-context-before-commit");
  if let Some(fact) = requires_context {
    let missing = &fact.obj; // "experimental-setup", "domain", etc.
    if !is_generic_context_flag(missing) {
      let subject = normalize_held_subject(&fact.subj);
      let context = &fact.context.0;

      // missing context에 따라 구체적인 follow-up 질문 생성
      let hint = match missing.as_str() {
        "experimental-setup" | "experimental-context" =>
          format!("'{subject}'에 대해 어떤 실험/상황에서 보고 있는지 알려주면 더 정확히 답할 수 있다. [{context}]"),
        "domain" =>
          format!("'{subject}'이(가) 어떤 분야에서 쓰이는 건지 알려주면 held를 다시 열 수 있다. [{context}]"),
        _ =>
          format!("'{subject}'에 대한 '{missing}' 맥락이 필요하다. 더 구체적으로 물어보면 held judgement를 다시 열 수 있다. [{context}]"),
      };
      return Some(hint);
    }
  }

  // 일반 held — concept-held 패턴
  let held_concept = contextual_facts
    .iter()
    .find(|fact| fact.pred == "held-reason" && fact.status == MeaningStatus::Held);
  if let Some(fact) = held_concept {
    let subject = normalize_held_subject(&fact.subj);
    return Some(format!(
      "'{}'에 대한 엄밀한 정의가 아직 없다. 더 구체적인 맥락을 알려주면 held judgement를 다시 열 수 있다.",
      subject
    ));
  }

  if contextual_facts
    .iter()
    .any(|fact| fact.pred == "reopens-judgement")
  {
    return Some("follow-up가 이전 held judgement를 다시 열었다".to_string());
  }

  None
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn contextual_fact_round_trip_json() {
    let fact = ContextualFact {
      id: Some(MeaningId::from("fact.001")),
      context: ContextId::from("Ownership"),
      layer: LayerId::from("L3"),
      subj: "cheolsu".to_string(),
      pred: "owns".to_string(),
      obj: "baduk".to_string(),
      status: MeaningStatus::Candidate,
      confidence: 0.95,
      provenance_refs: vec!["obs.001".to_string()],
      proof_refs: vec![],
      contradiction_refs: vec![],
      loss: None,
      timestamp: Some("2026-04-04T00:00:00Z".to_string()),
    };

    let json = serde_json::to_string(&fact).expect("serialize contextual fact");
    let decoded: ContextualFact = serde_json::from_str(&json).expect("deserialize contextual fact");

    assert_eq!(decoded, fact);
  }

  #[test]
  fn evaluation_vector_serializes_canonical_axes() {
    let eval = EvaluationVector {
      id: "eval.001".to_string(),
      interpretation: InterpretationId::from("interp.001"),
      policy: "runtime.default".to_string(),
      coherence: 0.9,
      coverage: 0.8,
      loss_penalty: 0.1,
      cost: 0.2,
      replayability: 1.0,
      safety: 1.0,
      score: 0.0,
    };

    let value = serde_json::to_value(eval).expect("serialize evaluation");
    let object = value.as_object().expect("evaluation object");

    assert!(object.contains_key("coherence"));
    assert!(object.contains_key("coverage"));
    assert!(object.contains_key("loss_penalty"));
    assert!(object.contains_key("cost"));
    assert!(object.contains_key("replayability"));
    assert!(object.contains_key("safety"));
    assert!(object.contains_key("score"));
  }

  #[test]
  fn semantic_episode_and_knowledge_record_round_trip_json() {
    let episode = SemanticEpisode {
      id: SemanticEpisodeId::from("episode.demo.001"),
      observation_refs: vec!["utterance:user".to_string()],
      record_refs: vec![
        SemanticRecordId::from("record.fact.001"),
        SemanticRecordId::from("record.judgement.001"),
      ],
      chosen_interpretation: Some(InterpretationId::from("interp.demo.001")),
      judgement_ref: Some("judge.demo.001".to_string()),
      promotion_ref: Some("promote.demo.001".to_string()),
      summary: Some("demo episode".to_string()),
    };
    let knowledge = KnowledgeRecord {
      id: KnowledgeRecordId::from("knowledge.demo.001"),
      episode: episode.id.clone(),
      target_status: MeaningStatus::Accepted,
      fact_refs: vec!["fact.demo.001".to_string()],
      source_record_refs: vec![SemanticRecordId::from("record.fact.001")],
      provenance_refs: vec!["obs.demo".to_string()],
      summary: Some("stable promoted knowledge".to_string()),
    };

    let episode_json = serde_json::to_string(&episode).expect("serialize semantic episode");
    let decoded_episode: SemanticEpisode =
      serde_json::from_str(&episode_json).expect("deserialize semantic episode");
    let knowledge_json = serde_json::to_string(&knowledge).expect("serialize knowledge record");
    let decoded_knowledge: KnowledgeRecord =
      serde_json::from_str(&knowledge_json).expect("deserialize knowledge record");

    assert_eq!(decoded_episode, episode);
    assert_eq!(decoded_knowledge, knowledge);
  }

  #[test]
  fn semantic_ingest_envelope_round_trip_json() {
    let episode = SemanticEpisode {
      id: SemanticEpisodeId::from("episode.demo.002"),
      observation_refs: vec!["utterance:user".to_string()],
      record_refs: vec![SemanticRecordId::from("record.fact.002")],
      chosen_interpretation: Some(InterpretationId::from("interp.demo.002")),
      judgement_ref: Some("judge.demo.002".to_string()),
      promotion_ref: Some("promote.demo.002".to_string()),
      summary: Some("demo ingest episode".to_string()),
    };
    let record = SemanticRecord {
      id: SemanticRecordId::from("record.fact.002"),
      episode: episode.id.clone(),
      record_kind: SemanticRecordKind::ContextualFact,
      provenance_refs: vec!["obs.demo".to_string()],
      artifact_refs: vec![],
      value: SemanticRecordValue::ContextualFact(ContextualFact {
        id: Some(MeaningId::from("fact.demo.002")),
        context: ContextId::from("Career.KR"),
        layer: LayerId::from("L4"),
        subj: "user".to_string(),
        pred: "wants-career".to_string(),
        obj: "lawyer".to_string(),
        status: MeaningStatus::Candidate,
        confidence: 1.0,
        provenance_refs: vec!["utterance:demo".to_string()],
        proof_refs: vec![],
        contradiction_refs: vec![],
        loss: None,
        timestamp: None,
      }),
    };
    let knowledge = KnowledgeRecord {
      id: KnowledgeRecordId::from("knowledge.demo.002"),
      episode: episode.id.clone(),
      target_status: MeaningStatus::Accepted,
      fact_refs: vec!["fact.demo.002".to_string()],
      source_record_refs: vec![record.id.clone()],
      provenance_refs: vec!["obs.demo".to_string()],
      summary: Some("stable promoted knowledge".to_string()),
    };
    let envelope = SemanticIngestEnvelope {
      observation_refs: episode.observation_refs.clone(),
      records: vec![record],
      episode,
      knowledge_records: vec![knowledge],
      notes: vec!["canonical semantic ingest carrier".to_string()],
    };

    let json = serde_json::to_string(&envelope).expect("serialize semantic ingest envelope");
    let decoded: SemanticIngestEnvelope =
      serde_json::from_str(&json).expect("deserialize semantic ingest envelope");

    assert_eq!(decoded, envelope);
  }

  #[test]
  fn tool_adapter_records_round_trip_json() {
    let capability = ToolCapabilityRecord {
      id: ToolCapabilityId::from("tool.blender"),
      tool_name: "blender".to_string(),
      adapter_runtime: "python".to_string(),
      transport: ToolTransportKind::Plugin,
      executable_actions: vec!["scene.export_glb".to_string()],
      query_actions: vec!["scene.inspect".to_string()],
      observable_events: vec!["scene.changed".to_string()],
      artifact_kinds: vec!["blend".to_string(), "glb".to_string()],
      notes: vec!["private adapter vocabulary".to_string()],
    };
    let plan = ToolActionPlan {
      id: ToolActionPlanId::from("plan.blender.001"),
      capability: capability.id.clone(),
      judgement: "judge.demo.001".to_string(),
      chosen_interpretation: Some(InterpretationId::from("interp.demo.001")),
      action_name: "scene.export_glb".to_string(),
      args: BTreeMap::from([("scene".to_string(), "main".to_string())]),
      provenance_refs: vec!["judge.demo.001".to_string()],
      notes: vec!["hot path dispatch".to_string()],
    };
    let result = ToolExecutionResult {
      id: ToolExecutionResultId::from("result.blender.001"),
      plan: plan.id.clone(),
      capability: capability.id.clone(),
      status: ToolExecutionStatus::Succeeded,
      semantic_facts: vec![],
      expression_projections: vec![ExpressionProjectionRecord {
        id: ExpressionProjectionId::from("expr.blender.preview"),
        context: ContextId::from("ToolPreview"),
        layer: LayerId::from("L4"),
        subject: "scene.export_glb".to_string(),
        projection_family: "expmath".to_string(),
        canonical_form: "mesh_count(main) = 1".to_string(),
        semantic_fact_refs: vec![],
        surface_forms: BTreeMap::from([
          ("openmath".to_string(), "<OMOBJ><OMA><OMS cd=\"relation1\" name=\"eq\"/><OMV name=\"mesh_count(main)\"/><OMI>1</OMI></OMA></OMOBJ>".to_string()),
          ("mathml-content".to_string(), "<math><apply><eq/><ci>mesh_count(main)</ci><cn>1</cn></apply></math>".to_string()),
        ]),
        provenance_refs: vec!["runtime:preview".to_string()],
        artifact_refs: vec!["artifact://scene/main.glb".to_string()],
        notes: vec!["expression projection stays downstream from semantic meaning".to_string()],
      }],
      artifact_refs: vec!["artifact://scene/main.glb".to_string()],
      provenance_refs: vec!["runtime:dispatch".to_string()],
      summary: Some("export completed".to_string()),
      notes: vec!["cold path ingest".to_string()],
    };

    let capability_json = serde_json::to_string(&capability).expect("serialize capability");
    let decoded_capability: ToolCapabilityRecord =
      serde_json::from_str(&capability_json).expect("deserialize capability");
    let plan_json = serde_json::to_string(&plan).expect("serialize plan");
    let decoded_plan: ToolActionPlan = serde_json::from_str(&plan_json).expect("deserialize plan");
    let result_json = serde_json::to_string(&result).expect("serialize result");
    let decoded_result: ToolExecutionResult =
      serde_json::from_str(&result_json).expect("deserialize result");

    assert_eq!(decoded_capability, capability);
    assert_eq!(decoded_plan, plan);
    assert_eq!(decoded_result, result);
  }

  #[test]
  fn tool_execution_result_builds_canonical_semantic_ingest_envelope() {
    let capability = ToolCapabilityRecord {
      id: ToolCapabilityId::from("tool.python"),
      tool_name: "python".to_string(),
      adapter_runtime: "python".to_string(),
      transport: ToolTransportKind::StdIo,
      executable_actions: vec!["script.run".to_string()],
      query_actions: vec![],
      observable_events: vec!["script.completed".to_string()],
      artifact_kinds: vec!["stdout".to_string()],
      notes: vec![],
    };
    let plan = ToolActionPlan {
      id: ToolActionPlanId::from("plan.python.001"),
      capability: capability.id.clone(),
      judgement: "judge.tool.001".to_string(),
      chosen_interpretation: Some(InterpretationId::from("interp.tool.001")),
      action_name: "script.run".to_string(),
      args: BTreeMap::from([("entrypoint".to_string(), "demo.py".to_string())]),
      provenance_refs: vec!["judge.tool.001".to_string()],
      notes: vec![],
    };
    let fact = ContextualFact {
      id: Some(MeaningId::from("fact.tool.001")),
      context: ContextId::from("ToolRuntime"),
      layer: LayerId::from("L4"),
      subj: "python".to_string(),
      pred: "produced-artifact".to_string(),
      obj: "artifact://stdout/demo".to_string(),
      status: MeaningStatus::Accepted,
      confidence: 1.0,
      provenance_refs: vec!["runtime:tool.python".to_string()],
      proof_refs: vec!["artifact://stdout/demo".to_string()],
      contradiction_refs: vec![],
      loss: None,
      timestamp: None,
    };
    let result = ToolExecutionResult {
      id: ToolExecutionResultId::from("result.python.001"),
      plan: plan.id.clone(),
      capability: capability.id.clone(),
      status: ToolExecutionStatus::Succeeded,
      semantic_facts: vec![fact.clone()],
      expression_projections: vec![ExpressionProjectionRecord {
        id: ExpressionProjectionId::from("expr.python.001"),
        context: ContextId::from("ToolRuntime"),
        layer: LayerId::from("L4"),
        subject: "python.script".to_string(),
        projection_family: "expmath".to_string(),
        canonical_form: "stdout(demo.py) = artifact://stdout/demo".to_string(),
        semantic_fact_refs: vec!["fact.tool.001".to_string()],
        surface_forms: BTreeMap::from([(
          "canonical-text".to_string(),
          "stdout(demo.py) = artifact://stdout/demo".to_string(),
        )]),
        provenance_refs: vec!["runtime:tool.python".to_string()],
        artifact_refs: vec!["artifact://stdout/demo".to_string()],
        notes: vec![
          "projection record can exist even when the primary domain is not math".to_string(),
        ],
      }],
      artifact_refs: vec!["artifact://stdout/demo".to_string()],
      provenance_refs: vec!["runtime:dispatch".to_string()],
      summary: Some("script completed".to_string()),
      notes: vec!["normalized execution result".to_string()],
    };

    let envelope = semantic_ingest_envelope_from_tool_execution(Some(&capability), &plan, &result);

    assert_eq!(
      envelope.episode.chosen_interpretation,
      Some(InterpretationId::from("interp.tool.001"))
    );
    assert_eq!(
      envelope.episode.judgement_ref.as_deref(),
      Some("judge.tool.001")
    );
    assert_eq!(envelope.records.len(), 5);
    assert!(envelope
      .records
      .iter()
      .any(|record| { matches!(record.value, SemanticRecordValue::ToolCapability(_)) }));
    assert!(envelope
      .records
      .iter()
      .any(|record| { matches!(record.value, SemanticRecordValue::ToolActionPlan(_)) }));
    assert!(envelope
      .records
      .iter()
      .any(|record| { matches!(record.value, SemanticRecordValue::ToolExecutionResult(_)) }));
    assert!(envelope
      .records
      .iter()
      .any(|record| { matches!(record.value, SemanticRecordValue::ExpressionProjection(_)) }));
    assert!(envelope.records.iter().any(|record| {
      matches!(
        &record.value,
        SemanticRecordValue::ContextualFact(contextual_fact)
          if contextual_fact == &fact
      )
    }));
  }

  #[test]
  fn ontology_lift_fact_rewrites_context_and_preserves_loss_metadata() {
    let fact = ContextualFact {
      id: Some(MeaningId::from("fact.001")),
      context: ContextId::from("LocalPetCare"),
      layer: LayerId::from("L3"),
      subj: "cheolsu".to_string(),
      pred: "hasPet".to_string(),
      obj: "baduk".to_string(),
      status: MeaningStatus::Candidate,
      confidence: 1.0,
      provenance_refs: vec!["obs.001".to_string()],
      proof_refs: vec![],
      contradiction_refs: vec![],
      loss: Some(LossPolicy {
        kind: LossKind::Lossless,
        notes: vec!["local-role-binding".to_string()],
      }),
      timestamp: None,
    };
    let lift = MeaningLift {
      id: "lift.ownership".to_string(),
      from_context: ContextId::from("LocalPetCare"),
      to_context: ContextId::from("Ownership"),
      object_map: BTreeMap::from([
        ("cheolsu".to_string(), "owner:cheolsu".to_string()),
        ("baduk".to_string(), "pet:baduk".to_string()),
      ]),
      relation_map: BTreeMap::from([("hasPet".to_string(), "owns".to_string())]),
      loss: Some(LossPolicy {
        kind: LossKind::Lossy,
        notes: vec!["care-role collapsed into ownership".to_string()],
      }),
    };

    let lifted = ontology_lift_fact(&lift, &fact);

    assert_eq!(lifted.context, ContextId::from("Ownership"));
    assert_eq!(lifted.subj, "owner:cheolsu");
    assert_eq!(lifted.pred, "owns");
    assert_eq!(lifted.obj, "pet:baduk");
    assert_eq!(lifted.id, Some(MeaningId::from("fact.001@Ownership")));
    assert_eq!(
      lifted.loss.as_ref().map(|loss| &loss.kind),
      Some(&LossKind::Lossy)
    );
    assert!(lifted
      .loss
      .as_ref()
      .expect("combined loss")
      .notes
      .contains(&"care-role collapsed into ownership".to_string()));
  }

  #[test]
  fn ontology_select_prefers_high_coverage_replayable_interpretation_and_promotes_it() {
    let facts = vec![
      ContextualFact {
        id: Some(MeaningId::from("fact.a")),
        context: ContextId::from("Career.KR"),
        layer: LayerId::from("L4"),
        subj: "user".to_string(),
        pred: "wants-career".to_string(),
        obj: "lawyer".to_string(),
        status: MeaningStatus::Candidate,
        confidence: 1.0,
        provenance_refs: vec!["obs.want".to_string()],
        proof_refs: vec![],
        contradiction_refs: vec![],
        loss: None,
        timestamp: None,
      },
      ContextualFact {
        id: Some(MeaningId::from("fact.b")),
        context: ContextId::from("Career.KR"),
        layer: LayerId::from("L4"),
        subj: "lawyer".to_string(),
        pred: "requires".to_string(),
        obj: "bar-exam".to_string(),
        status: MeaningStatus::Accepted,
        confidence: 1.0,
        provenance_refs: vec!["schema.kr.bar".to_string()],
        proof_refs: vec!["proof.kr.bar".to_string()],
        contradiction_refs: vec![],
        loss: None,
        timestamp: None,
      },
      ContextualFact {
        id: Some(MeaningId::from("fact.c")),
        context: ContextId::from("Career.KR"),
        layer: LayerId::from("L4"),
        subj: "lawyer".to_string(),
        pred: "requires".to_string(),
        obj: "law-school".to_string(),
        status: MeaningStatus::Accepted,
        confidence: 1.0,
        provenance_refs: vec!["schema.kr.lawschool".to_string()],
        proof_refs: vec!["proof.kr.lawschool".to_string()],
        contradiction_refs: vec![],
        loss: None,
        timestamp: None,
      },
    ];
    let narrow = Interpretation {
      id: InterpretationId::from("interp.narrow"),
      observation_refs: vec!["obs.want".to_string()],
      fact_refs: vec!["fact.a".to_string()],
      lift_refs: vec![],
      conflict_refs: vec![],
      loss: None,
    };
    let rich = Interpretation {
      id: InterpretationId::from("interp.rich"),
      observation_refs: vec!["obs.want".to_string()],
      fact_refs: vec![
        "fact.a".to_string(),
        "fact.b".to_string(),
        "fact.c".to_string(),
      ],
      lift_refs: vec!["lift.career.kr".to_string()],
      conflict_refs: vec![],
      loss: None,
    };
    let policy = EvaluationPolicy::default();

    let selected = ontology_select(&policy, &[narrow, rich], &facts).expect("selection outcome");
    let promotion = ontology_promote(&policy, &selected.judgement);

    assert_eq!(
      selected.judgement.chosen_interpretation,
      Some(InterpretationId::from("interp.rich"))
    );
    assert_eq!(selected.judgement.action, JudgementAction::Accept);
    assert!(selected.evaluation.coverage > 0.7);
    assert!(selected.evaluation.replayability > 0.9);
    assert_eq!(promotion.target_status, MeaningStatus::Accepted);
  }

  #[test]
  fn ontology_select_holds_when_best_interpretation_fails_safety_floor() {
    let facts = vec![ContextualFact {
      id: Some(MeaningId::from("fact.risky")),
      context: ContextId::from("Repair"),
      layer: LayerId::from("L4"),
      subj: "system".to_string(),
      pred: "wants-self-modify".to_string(),
      obj: "runtime".to_string(),
      status: MeaningStatus::Contradicted,
      confidence: 1.0,
      provenance_refs: vec!["obs.runtime".to_string()],
      proof_refs: vec![],
      contradiction_refs: vec!["conflict.guardrail".to_string()],
      loss: None,
      timestamp: None,
    }];
    let interpretation = Interpretation {
      id: InterpretationId::from("interp.risky"),
      observation_refs: vec!["obs.runtime".to_string()],
      fact_refs: vec!["fact.risky".to_string()],
      lift_refs: vec![],
      conflict_refs: vec!["conflict.guardrail".to_string()],
      loss: None,
    };
    let selected = ontology_select(&EvaluationPolicy::default(), &[interpretation], &facts)
      .expect("selection outcome");

    assert_eq!(selected.judgement.action, JudgementAction::Hold);
  }

  #[test]
  fn dialogue_transcript_from_ingest_prefers_explicit_transcript_notes() {
    let envelope = SemanticIngestEnvelope {
      observation_refs: vec!["utterance:무시될 첫 턴".to_string()],
      records: vec![],
      episode: SemanticEpisode {
        id: SemanticEpisodeId::from("episode.demo"),
        observation_refs: vec![],
        record_refs: vec![],
        chosen_interpretation: None,
        judgement_ref: None,
        promotion_ref: None,
        summary: Some("무시될 summary".to_string()),
      },
      knowledge_records: vec![],
      notes: vec![
        "transcript:user: 빛은 뭐야?".to_string(),
        "transcript:doghouse: held".to_string(),
      ],
    };

    assert_eq!(
      dialogue_transcript_from_ingest(&envelope),
      vec!["user: 빛은 뭐야?".to_string(), "doghouse: held".to_string()]
    );
  }

  #[test]
  fn follow_up_hint_from_ingest_detects_context_narrowing_need() {
    let envelope = SemanticIngestEnvelope {
      observation_refs: vec!["utterance:빛은 뭐야?".to_string()],
      records: vec![SemanticRecord {
        id: SemanticRecordId::from("record.fact.000"),
        episode: SemanticEpisodeId::from("episode.demo"),
        record_kind: SemanticRecordKind::ContextualFact,
        provenance_refs: vec![],
        artifact_refs: vec![],
        value: SemanticRecordValue::ContextualFact(ContextualFact {
          id: None,
          context: ContextId::from("Physics.Light.General"),
          layer: LayerId::from("L4"),
          subj: "light".to_string(),
          pred: "requires-context-before-commit".to_string(),
          obj: "narrower experimental context".to_string(),
          status: MeaningStatus::Held,
          confidence: 1.0,
          provenance_refs: vec![],
          proof_refs: vec![],
          contradiction_refs: vec![],
          loss: None,
          timestamp: None,
        }),
      }],
      episode: SemanticEpisode {
        id: SemanticEpisodeId::from("episode.demo"),
        observation_refs: vec![],
        record_refs: vec![],
        chosen_interpretation: None,
        judgement_ref: None,
        promotion_ref: None,
        summary: Some("held".to_string()),
      },
      knowledge_records: vec![],
      notes: vec![],
    };

    assert!(follow_up_hint_from_ingest(&envelope)
      .unwrap_or_default()
      .contains("held judgement"));
  }

  #[test]
  fn follow_up_hint_from_ingest_ignores_generic_context_flags() {
    let envelope = SemanticIngestEnvelope {
      observation_refs: vec!["utterance:빛은 뭐야?".to_string()],
      records: vec![
        SemanticRecord {
          id: SemanticRecordId::from("record.fact.term"),
          episode: SemanticEpisodeId::from("episode.demo"),
          record_kind: SemanticRecordKind::ContextualFact,
          provenance_refs: vec![],
          artifact_refs: vec![],
          value: SemanticRecordValue::ContextualFact(ContextualFact {
            id: None,
            context: ContextId::from("Doghouse.ConceptQuery"),
            layer: LayerId::from("L4"),
            subj: "user".to_string(),
            pred: "concept-query-term".to_string(),
            obj: "빛".to_string(),
            status: MeaningStatus::Held,
            confidence: 1.0,
            provenance_refs: vec![],
            proof_refs: vec![],
            contradiction_refs: vec![],
            loss: None,
            timestamp: None,
          }),
        },
        SemanticRecord {
          id: SemanticRecordId::from("record.fact.requires"),
          episode: SemanticEpisodeId::from("episode.demo"),
          record_kind: SemanticRecordKind::ContextualFact,
          provenance_refs: vec![],
          artifact_refs: vec![],
          value: SemanticRecordValue::ContextualFact(ContextualFact {
            id: None,
            context: ContextId::from("Doghouse.ConceptQuery"),
            layer: LayerId::from("L4"),
            subj: "doghouse".to_string(),
            pred: "requires-context-before-commit".to_string(),
            obj: "true".to_string(),
            status: MeaningStatus::Held,
            confidence: 1.0,
            provenance_refs: vec![],
            proof_refs: vec![],
            contradiction_refs: vec![],
            loss: None,
            timestamp: None,
          }),
        },
        SemanticRecord {
          id: SemanticRecordId::from("record.fact.held"),
          episode: SemanticEpisodeId::from("episode.demo"),
          record_kind: SemanticRecordKind::ContextualFact,
          provenance_refs: vec![],
          artifact_refs: vec![],
          value: SemanticRecordValue::ContextualFact(ContextualFact {
            id: None,
            context: ContextId::from("Doghouse.ConceptQuery"),
            layer: LayerId::from("L4"),
            subj: "doghouse".to_string(),
            pred: "held-reason".to_string(),
            obj: "unknown-term".to_string(),
            status: MeaningStatus::Held,
            confidence: 1.0,
            provenance_refs: vec![],
            proof_refs: vec![],
            contradiction_refs: vec![],
            loss: None,
            timestamp: None,
          }),
        },
      ],
      episode: SemanticEpisode {
        id: SemanticEpisodeId::from("episode.demo"),
        observation_refs: vec![],
        record_refs: vec![],
        chosen_interpretation: None,
        judgement_ref: None,
        promotion_ref: None,
        summary: Some("held".to_string()),
      },
      knowledge_records: vec![],
      notes: vec![],
    };

    let hint = follow_up_hint_from_ingest(&envelope).unwrap_or_default();
    assert!(hint.contains("'빛'"));
    assert!(!hint.contains("'doghouse'"));
    assert!(!hint.contains("'true'"));
  }
}

// =========================================================================
// Introspection: ontology 자기참조 (convergence Phase 0)
// =========================================================================

/// Introspection 수준. 환경변수 `PNIX_INTROSPECTION_LEVEL`로 제어.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IntrospectionLevel {
  /// 순수 평가만. 성능 영향 0. (기본값 아님)
  Off,
  /// AST 구조에서 fact 자동 방출. +0.1ms.
  Structural,
  /// builtin/stdlib 메타데이터 방출. 부팅 시 1회.
  Catalog,
  /// CT commute 검증 + 타입 충돌 감지. +1~3ms.
  Verification,
  /// 평가 매 단계를 fact로 기록. +10~50ms. 디버그 전용.
  FullTrace,
}

impl Default for IntrospectionLevel {
  fn default() -> Self {
    Self::Structural
  }
}

impl IntrospectionLevel {
  /// 문자열에서 파싱. pnix-core 는 실행하지 않으므로 env 접근 금지.
  /// caller (runtime/doghouse) 가 환경변수를 읽어서 이 함수에 넘긴다.
  pub fn parse(s: &str) -> Self {
    match s.trim().to_lowercase().as_str() {
      "off" => Self::Off,
      "structural" => Self::Structural,
      "catalog" => Self::Catalog,
      "verification" => Self::Verification,
      "fulltrace" | "full-trace" | "full_trace" => Self::FullTrace,
      _ => Self::default(),
    }
  }

  pub fn emits_structural(&self) -> bool {
    *self >= Self::Structural
  }

  pub fn emits_catalog(&self) -> bool {
    *self >= Self::Catalog
  }

  pub fn emits_verification(&self) -> bool {
    *self >= Self::Verification
  }

  pub fn emits_full_trace(&self) -> bool {
    *self >= Self::FullTrace
  }
}

/// 단일 introspection fact 생성.
///
/// introspection fact는 관찰 사실이므로 즉시 Accepted, confidence=1.0.
pub fn introspection_fact(subj: &str, pred: &str, obj: &str) -> ContextualFact {
  ContextualFact {
    id: Some(MeaningId::from(format!(
      "introspection.{}.{}.{}",
      subj, pred, obj
    ))),
    context: ContextId::from("PnixIntrospection"),
    layer: LayerId::from("L1"),
    subj: subj.to_string(),
    pred: pred.to_string(),
    obj: obj.to_string(),
    status: MeaningStatus::Accepted,
    confidence: 1.0,
    provenance_refs: vec!["introspection.structural".to_string()],
    proof_refs: vec![],
    contradiction_refs: vec![],
    loss: None,
    timestamp: None,
  }
}

/// introspection fact 배치 생성.
pub fn introspection_facts(subj: &str, pairs: &[(&str, &str)]) -> Vec<ContextualFact> {
  pairs
    .iter()
    .map(|(pred, obj)| introspection_fact(subj, pred, obj))
    .collect()
}

/// introspection fact를 SemanticRecord로 래핑.
pub fn introspection_record(
  episode_id: &SemanticEpisodeId,
  index: usize,
  fact: ContextualFact,
) -> SemanticRecord {
  SemanticRecord {
    id: SemanticRecordId::from(format!("{}.introspection.{}", episode_id.0, index)),
    episode: episode_id.clone(),
    record_kind: SemanticRecordKind::ContextualFact,
    value: SemanticRecordValue::ContextualFact(fact),
    provenance_refs: vec!["introspection.structural".to_string()],
    artifact_refs: vec![],
  }
}

/// introspection facts를 SemanticIngestEnvelope로 조립.
pub fn introspection_envelope(
  episode_id_str: &str,
  source_path: &str,
  facts: Vec<ContextualFact>,
) -> SemanticIngestEnvelope {
  let episode_id = SemanticEpisodeId::from(episode_id_str.to_string());

  let records: Vec<SemanticRecord> = facts
    .iter()
    .enumerate()
    .map(|(i, fact)| introspection_record(&episode_id, i, fact.clone()))
    .collect();

  let record_refs = records.iter().map(|r| r.id.clone()).collect::<Vec<_>>();
  let fact_refs = records.iter().map(|r| r.id.0.clone()).collect::<Vec<_>>();

  SemanticIngestEnvelope {
    observation_refs: vec![format!("source:introspection:{}", source_path)],
    records,
    episode: SemanticEpisode {
      id: episode_id.clone(),
      observation_refs: vec![format!("source:introspection:{}", source_path)],
      record_refs,
      chosen_interpretation: None,
      judgement_ref: None,
      promotion_ref: None,
      summary: Some(format!("introspection: {}", source_path)),
    },
    knowledge_records: vec![KnowledgeRecord {
      id: KnowledgeRecordId::from(format!("knowledge.introspection.{}", episode_id_str)),
      episode: episode_id,
      target_status: MeaningStatus::Accepted,
      fact_refs,
      source_record_refs: vec![],
      provenance_refs: vec![format!("source:introspection:{}", source_path)],
      summary: Some(format!("introspection facts from {}", source_path)),
    }],
    notes: vec![format!("introspection:source:{}", source_path)],
  }
}

#[cfg(test)]
mod introspection_tests {
  use super::*;

  #[test]
  fn introspection_level_from_env_defaults_to_structural() {
    // 환경변수 없으면 Structural
    assert_eq!(
      IntrospectionLevel::default(),
      IntrospectionLevel::Structural
    );
  }

  #[test]
  fn introspection_level_ordering() {
    assert!(IntrospectionLevel::Off < IntrospectionLevel::Structural);
    assert!(IntrospectionLevel::Structural < IntrospectionLevel::Catalog);
    assert!(IntrospectionLevel::Catalog < IntrospectionLevel::Verification);
    assert!(IntrospectionLevel::Verification < IntrospectionLevel::FullTrace);
  }

  #[test]
  fn introspection_fact_has_correct_defaults() {
    let fact = introspection_fact("map", "is-a", "builtin");
    assert_eq!(fact.context.0, "PnixIntrospection");
    assert_eq!(fact.layer.0, "L1");
    assert_eq!(fact.status, MeaningStatus::Accepted);
    assert_eq!(fact.confidence, 1.0);
    assert_eq!(fact.subj, "map");
    assert_eq!(fact.pred, "is-a");
    assert_eq!(fact.obj, "builtin");
  }

  #[test]
  fn introspection_facts_batch() {
    let facts = introspection_facts(
      "builtins.map",
      &[("is-a", "builtin"), ("arity", "2"), ("category", "list")],
    );
    assert_eq!(facts.len(), 3);
    assert_eq!(facts[0].pred, "is-a");
    assert_eq!(facts[1].pred, "arity");
    assert_eq!(facts[2].pred, "category");
  }

  #[test]
  fn introspection_envelope_assembles_correctly() {
    let facts = vec![
      introspection_fact("map", "is-a", "builtin"),
      introspection_fact("map", "arity", "2"),
    ];
    let envelope = introspection_envelope("episode.test", "test.px", facts);
    assert_eq!(envelope.records.len(), 2);
    assert_eq!(envelope.episode.record_refs.len(), 2);
    assert_eq!(envelope.knowledge_records.len(), 1);
    assert!(envelope.notes[0].contains("introspection:source:test.px"));
  }
}
