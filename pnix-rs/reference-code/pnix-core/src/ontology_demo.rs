//! Minimal deterministic sentence -> ontology -> judgement demo.
//!
//! This stays intentionally small and controlled. It is not a general NLP
//! layer. The goal is to prove that a plain utterance can become a canonical
//! `ContextualFact`, multiple `Interpretation`s, deterministic evaluation,
//! deterministic judgement, and an explicit promotion decision.

use std::collections::BTreeMap;

use crate::lang::analyze_korean_text;
use crate::ontology::{
  ontology_promote, ontology_select, ContextId, ContextualFact, EvaluationPolicy,
  EvaluationWeights, ExpressionProjectionId, ExpressionProjectionRecord, Interpretation,
  InterpretationId, KnowledgeRecord, KnowledgeRecordId, LayerId, LossKind, LossPolicy, MeaningId,
  MeaningStatus, PromotionDecision, SelectionOutcome, SemanticEpisode, SemanticEpisodeId,
  SemanticIngestEnvelope, SemanticRecord, SemanticRecordId, SemanticRecordKind,
  SemanticRecordValue, ToolActionPlan, ToolActionPlanId, ToolCapabilityId, ToolCapabilityRecord,
  ToolExecutionResult, ToolExecutionResultId, ToolExecutionStatus, ToolTransportKind,
};

#[derive(Debug, Clone, PartialEq)]
pub struct SentenceJudgementDemo {
  pub utterance: String,
  pub observation: ContextualFact,
  pub facts: Vec<ContextualFact>,
  pub interpretations: Vec<Interpretation>,
  pub selected: SelectionOutcome,
  pub promotion: PromotionDecision,
  pub answer: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KoreanToolDialogueDemo {
  pub utterance: String,
  pub observation: ContextualFact,
  pub facts: Vec<ContextualFact>,
  pub interpretations: Vec<Interpretation>,
  pub selected: SelectionOutcome,
  pub promotion: PromotionDecision,
  pub capability: ToolCapabilityRecord,
  pub plan: ToolActionPlan,
  pub result: ToolExecutionResult,
  pub answer: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KoreanToolDialogueFollowupDemo {
  pub initial: KoreanToolDialogueDemo,
  pub follow_up: KoreanToolDialogueDemo,
}

impl SentenceJudgementDemo {
  pub fn semantic_records(&self) -> Vec<SemanticRecord> {
    let mut records = Vec::new();
    let episode = SemanticEpisodeId::from("episode.demo.sentence-judgement");

    for (index, fact) in self.facts.iter().enumerate() {
      records.push(SemanticRecord {
        id: SemanticRecordId::from(format!("record.fact.{index:03}")),
        episode: episode.clone(),
        record_kind: SemanticRecordKind::ContextualFact,
        provenance_refs: fact.provenance_refs.clone(),
        artifact_refs: fact.proof_refs.clone(),
        value: SemanticRecordValue::ContextualFact(fact.clone()),
      });
    }

    for (index, interpretation) in self.interpretations.iter().enumerate() {
      records.push(SemanticRecord {
        id: SemanticRecordId::from(format!("record.interpretation.{index:03}")),
        episode: episode.clone(),
        record_kind: SemanticRecordKind::Interpretation,
        provenance_refs: interpretation.observation_refs.clone(),
        artifact_refs: interpretation.fact_refs.clone(),
        value: SemanticRecordValue::Interpretation(interpretation.clone()),
      });
    }

    records.push(SemanticRecord {
      id: SemanticRecordId::from("record.evaluation.000"),
      episode: episode.clone(),
      record_kind: SemanticRecordKind::Evaluation,
      provenance_refs: vec![self.selected.evaluation.policy.clone()],
      artifact_refs: self.selected.judgement.chosen_fact_refs.clone(),
      value: SemanticRecordValue::Evaluation(self.selected.evaluation.clone()),
    });

    records.push(SemanticRecord {
      id: SemanticRecordId::from("record.judgement.000"),
      episode: episode.clone(),
      record_kind: SemanticRecordKind::Judgement,
      provenance_refs: self.selected.judgement.notes.clone(),
      artifact_refs: self.selected.judgement.chosen_fact_refs.clone(),
      value: SemanticRecordValue::Judgement(self.selected.judgement.clone()),
    });

    records.push(SemanticRecord {
      id: SemanticRecordId::from("record.promotion.000"),
      episode,
      record_kind: SemanticRecordKind::Promotion,
      provenance_refs: self.promotion.reason.clone().into_iter().collect(),
      artifact_refs: self.promotion.artifact_refs.clone(),
      value: SemanticRecordValue::Promotion(self.promotion.clone()),
    });

    records
  }

  pub fn semantic_episode(&self) -> SemanticEpisode {
    let records = self.semantic_records();

    SemanticEpisode {
      id: SemanticEpisodeId::from("episode.demo.sentence-judgement"),
      observation_refs: self.observation.provenance_refs.clone(),
      record_refs: records.iter().map(|record| record.id.clone()).collect(),
      chosen_interpretation: self.selected.judgement.chosen_interpretation.clone(),
      judgement_ref: Some(self.selected.judgement.id.clone()),
      promotion_ref: Some(self.promotion.id.clone()),
      summary: Some(self.answer.clone()),
    }
  }

  pub fn knowledge_record(&self) -> KnowledgeRecord {
    let episode = self.semantic_episode();
    let records = self.semantic_records();

    KnowledgeRecord {
      id: KnowledgeRecordId::from("knowledge.demo.sentence-judgement"),
      episode: episode.id,
      target_status: self.promotion.target_status.clone(),
      fact_refs: self.selected.judgement.chosen_fact_refs.clone(),
      source_record_refs: records.iter().map(|record| record.id.clone()).collect(),
      provenance_refs: self.observation.provenance_refs.clone(),
      summary: Some(self.answer.clone()),
    }
  }

  pub fn semantic_ingest_envelope(&self) -> SemanticIngestEnvelope {
    let records = self.semantic_records();
    let episode = self.semantic_episode();
    let knowledge = self.knowledge_record();

    SemanticIngestEnvelope {
      observation_refs: episode.observation_refs.clone(),
      records,
      episode,
      knowledge_records: vec![knowledge],
      notes: vec![
        "canonical semantic ingest carrier".to_string(),
        "external clients do not need internal persistence names".to_string(),
      ],
    }
  }
}

impl KoreanToolDialogueDemo {
  pub fn transcript_lines(&self) -> Vec<String> {
    vec![
      format!("user: {}", self.utterance),
      format!("doghouse: {}", self.answer),
    ]
  }

  pub fn semantic_records(&self) -> Vec<SemanticRecord> {
    let mut records = Vec::new();
    let episode = SemanticEpisodeId::from("episode.demo.korean-tool-dialogue");

    for (index, fact) in self.facts.iter().enumerate() {
      records.push(SemanticRecord {
        id: SemanticRecordId::from(format!("record.dialogue.fact.{index:03}")),
        episode: episode.clone(),
        record_kind: SemanticRecordKind::ContextualFact,
        provenance_refs: fact.provenance_refs.clone(),
        artifact_refs: fact.proof_refs.clone(),
        value: SemanticRecordValue::ContextualFact(fact.clone()),
      });
    }

    for (index, fact) in self.result.semantic_facts.iter().enumerate() {
      records.push(SemanticRecord {
        id: SemanticRecordId::from(format!("record.dialogue.effect.{index:03}")),
        episode: episode.clone(),
        record_kind: SemanticRecordKind::ContextualFact,
        provenance_refs: fact.provenance_refs.clone(),
        artifact_refs: fact.proof_refs.clone(),
        value: SemanticRecordValue::ContextualFact(fact.clone()),
      });
    }

    for (index, interpretation) in self.interpretations.iter().enumerate() {
      records.push(SemanticRecord {
        id: SemanticRecordId::from(format!("record.dialogue.interpretation.{index:03}")),
        episode: episode.clone(),
        record_kind: SemanticRecordKind::Interpretation,
        provenance_refs: interpretation.observation_refs.clone(),
        artifact_refs: interpretation.fact_refs.clone(),
        value: SemanticRecordValue::Interpretation(interpretation.clone()),
      });
    }

    records.push(SemanticRecord {
      id: SemanticRecordId::from("record.dialogue.evaluation.000"),
      episode: episode.clone(),
      record_kind: SemanticRecordKind::Evaluation,
      provenance_refs: vec![self.selected.evaluation.policy.clone()],
      artifact_refs: self.selected.judgement.chosen_fact_refs.clone(),
      value: SemanticRecordValue::Evaluation(self.selected.evaluation.clone()),
    });

    records.push(SemanticRecord {
      id: SemanticRecordId::from("record.dialogue.judgement.000"),
      episode: episode.clone(),
      record_kind: SemanticRecordKind::Judgement,
      provenance_refs: self.selected.judgement.notes.clone(),
      artifact_refs: self.selected.judgement.chosen_fact_refs.clone(),
      value: SemanticRecordValue::Judgement(self.selected.judgement.clone()),
    });

    records.push(SemanticRecord {
      id: SemanticRecordId::from("record.dialogue.promotion.000"),
      episode: episode.clone(),
      record_kind: SemanticRecordKind::Promotion,
      provenance_refs: self.promotion.reason.clone().into_iter().collect(),
      artifact_refs: self.promotion.artifact_refs.clone(),
      value: SemanticRecordValue::Promotion(self.promotion.clone()),
    });

    records.push(SemanticRecord {
      id: SemanticRecordId::from("record.dialogue.capability.000"),
      episode: episode.clone(),
      record_kind: SemanticRecordKind::ToolCapability,
      provenance_refs: self.capability.notes.clone(),
      artifact_refs: vec![],
      value: SemanticRecordValue::ToolCapability(self.capability.clone()),
    });

    records.push(SemanticRecord {
      id: SemanticRecordId::from("record.dialogue.plan.000"),
      episode: episode.clone(),
      record_kind: SemanticRecordKind::ToolActionPlan,
      provenance_refs: self.plan.provenance_refs.clone(),
      artifact_refs: vec![],
      value: SemanticRecordValue::ToolActionPlan(self.plan.clone()),
    });

    records.push(SemanticRecord {
      id: SemanticRecordId::from("record.dialogue.result.000"),
      episode,
      record_kind: SemanticRecordKind::ToolExecutionResult,
      provenance_refs: self.result.provenance_refs.clone(),
      artifact_refs: self.result.artifact_refs.clone(),
      value: SemanticRecordValue::ToolExecutionResult(self.result.clone()),
    });

    for (index, projection) in self.result.expression_projections.iter().enumerate() {
      records.push(SemanticRecord {
        id: SemanticRecordId::from(format!("record.dialogue.expression.{index:03}")),
        episode: SemanticEpisodeId::from("episode.demo.korean-tool-dialogue"),
        record_kind: SemanticRecordKind::ExpressionProjection,
        provenance_refs: projection.provenance_refs.clone(),
        artifact_refs: projection.artifact_refs.clone(),
        value: SemanticRecordValue::ExpressionProjection(projection.clone()),
      });
    }

    records
  }

  pub fn semantic_episode(&self) -> SemanticEpisode {
    let records = self.semantic_records();

    SemanticEpisode {
      id: SemanticEpisodeId::from("episode.demo.korean-tool-dialogue"),
      observation_refs: self.observation.provenance_refs.clone(),
      record_refs: records.iter().map(|record| record.id.clone()).collect(),
      chosen_interpretation: self.selected.judgement.chosen_interpretation.clone(),
      judgement_ref: Some(self.selected.judgement.id.clone()),
      promotion_ref: Some(self.promotion.id.clone()),
      summary: Some(self.answer.clone()),
    }
  }

  pub fn knowledge_record(&self) -> KnowledgeRecord {
    let episode = self.semantic_episode();
    let records = self.semantic_records();
    let mut fact_refs = self
      .facts
      .iter()
      .filter_map(|fact| fact.id.as_ref().cloned())
      .collect::<Vec<_>>();
    fact_refs.extend(
      self
        .result
        .semantic_facts
        .iter()
        .filter_map(|fact| fact.id.as_ref().cloned()),
    );

    KnowledgeRecord {
      id: KnowledgeRecordId::from("knowledge.demo.korean-tool-dialogue"),
      episode: episode.id,
      target_status: self.promotion.target_status.clone(),
      fact_refs: fact_refs.into_iter().map(|fact| fact.0).collect(),
      source_record_refs: records.iter().map(|record| record.id.clone()).collect(),
      provenance_refs: self.observation.provenance_refs.clone(),
      summary: Some(self.answer.clone()),
    }
  }

  pub fn semantic_ingest_envelope(&self) -> SemanticIngestEnvelope {
    let records = self.semantic_records();
    let episode = self.semantic_episode();
    let knowledge = self.knowledge_record();
    let mut notes = vec![
      "korean tool dialogue semantic ingest carrier".to_string(),
      "doghouse persists korean dialogue/tool intent as semantic records".to_string(),
    ];
    notes.extend(
      self
        .transcript_lines()
        .into_iter()
        .map(|line| format!("transcript:{line}")),
    );

    SemanticIngestEnvelope {
      observation_refs: episode.observation_refs.clone(),
      records,
      episode,
      knowledge_records: vec![knowledge],
      notes,
    }
  }
}

impl KoreanToolDialogueFollowupDemo {
  pub fn transcript_lines(&self) -> Vec<String> {
    vec![
      format!("user: {}", self.initial.utterance),
      format!("doghouse: {}", self.initial.answer),
      format!("user: {}", self.follow_up.utterance),
      format!("doghouse: {}", self.follow_up.answer),
    ]
  }

  pub fn semantic_ingest_envelope(&self) -> SemanticIngestEnvelope {
    let mut envelope = self.follow_up.semantic_ingest_envelope();
    envelope
      .notes
      .retain(|note| !note.starts_with("transcript:"));
    envelope.notes.extend(
      self
        .transcript_lines()
        .into_iter()
        .map(|line| format!("transcript:{line}")),
    );
    envelope
  }
}

fn normalized_lawyer_goal(utterance: &str) -> bool {
  let normalized = utterance.trim().to_ascii_lowercase();
  matches!(
    normalized.as_str(),
    "i want to be a lawyer"
      | "i want to become a lawyer"
      | "i want to become lawyer"
      | "나 변호사 하고 싶어"
      | "나 변호사가 되고 싶어"
      | "변호사가 되고 싶어"
  )
}

fn career_schema_facts(jurisdiction: &str) -> (String, Vec<ContextualFact>) {
  let jurisdiction = match jurisdiction.trim() {
    "" => "Career.KR",
    other => other,
  };
  let law_school = match jurisdiction {
    "Career.US" => "jd-program",
    _ => "law-school",
  };
  let bar_exam = match jurisdiction {
    "Career.US" => "state-bar-exam",
    _ => "bar-exam",
  };

  (
    jurisdiction.to_string(),
    vec![
      ContextualFact {
        id: Some(MeaningId::from("fact.schema.001")),
        context: ContextId::from(jurisdiction),
        layer: LayerId::from("L4"),
        subj: "lawyer".to_string(),
        pred: "requires".to_string(),
        obj: law_school.to_string(),
        status: MeaningStatus::Accepted,
        confidence: 1.0,
        provenance_refs: vec![format!("schema:{jurisdiction}:law-school")],
        proof_refs: vec![format!("proof:{jurisdiction}:law-school")],
        contradiction_refs: vec![],
        loss: None,
        timestamp: None,
      },
      ContextualFact {
        id: Some(MeaningId::from("fact.schema.002")),
        context: ContextId::from(jurisdiction),
        layer: LayerId::from("L4"),
        subj: "lawyer".to_string(),
        pred: "requires".to_string(),
        obj: bar_exam.to_string(),
        status: MeaningStatus::Accepted,
        confidence: 1.0,
        provenance_refs: vec![format!("schema:{jurisdiction}:bar-exam")],
        proof_refs: vec![format!("proof:{jurisdiction}:bar-exam")],
        contradiction_refs: vec![],
        loss: None,
        timestamp: None,
      },
    ],
  )
}

fn answer_for_demo(
  jurisdiction: &str,
  selected: &SelectionOutcome,
  facts: &[ContextualFact],
) -> String {
  let chosen_refs = &selected.judgement.chosen_fact_refs;
  let requirements = facts
    .iter()
    .filter(|fact| {
      chosen_refs
        .iter()
        .any(|fact_ref| fact.id.as_ref().is_some_and(|id| id.0 == *fact_ref))
        && fact.subj == "lawyer"
        && fact.pred == "requires"
    })
    .map(|fact| fact.obj.clone())
    .collect::<Vec<_>>();
  let requirement_text = if requirements.is_empty() {
    "requirements unavailable".to_string()
  } else {
    requirements.join(", ")
  };

  format!(
    "judgement={:?}; jurisdiction={}; requirements={}; provenance={}; score={:.2}",
    selected.judgement.action,
    jurisdiction,
    requirement_text,
    chosen_refs.join("|"),
    selected.evaluation.score
  )
}

struct ToolDialogueSpec {
  kind: ToolDialogueKind,
  tool_name: &'static str,
  runtime: &'static str,
  context: &'static str,
  action_name: &'static str,
  args: &'static [(&'static str, &'static str)],
  object: &'static str,
  summary: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToolDialogueKind {
  ExternalTool,
  OntologyRuntime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ZebraPuzzleSolution {
  fish_owner_ko: &'static str,
  water_drinker_ko: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ArithmeticSolution {
  expression: &'static str,
  result: i64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct NewtonForceSolution {
  mass_kg: f64,
  acceleration_mps2: f64,
  force_n: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LightContextSolution {
  context_ko: &'static str,
  manifestation: &'static str,
  phenomenon: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LightAmbiguitySolution {
  reason: &'static str,
  wave_context: &'static str,
  wave_manifestation: &'static str,
  particle_context: &'static str,
  particle_manifestation: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum RuntimeDialogueResult {
  Zebra(ZebraPuzzleSolution),
  Arithmetic(ArithmeticSolution),
  NewtonForce(NewtonForceSolution),
  LightContext(LightContextSolution),
  LightAmbiguity(LightAmbiguitySolution),
}

fn korean_language_loss(kind: LossKind, note: &str) -> Option<LossPolicy> {
  Some(LossPolicy {
    kind,
    notes: vec![note.to_string()],
  })
}

fn korean_language_facts(utterance: &str, context: &str) -> Vec<ContextualFact> {
  let analysis = analyze_korean_text(utterance);
  let context = ContextId::from(context);
  let mut facts = vec![
    ContextualFact {
      id: Some(MeaningId::from("fact.dialogue.ko.001")),
      context: context.clone(),
      layer: LayerId::from("L1"),
      subj: "utterance".to_string(),
      pred: "language".to_string(),
      obj: "ko".to_string(),
      status: MeaningStatus::Accepted,
      confidence: 1.0,
      provenance_refs: vec!["lang.ko:detected".to_string()],
      proof_refs: vec!["lang.ko:grammar".to_string()],
      contradiction_refs: vec![],
      loss: korean_language_loss(LossKind::Lossless, "direct language detection"),
      timestamp: None,
    },
    ContextualFact {
      id: Some(MeaningId::from("fact.dialogue.ko.002")),
      context: context.clone(),
      layer: LayerId::from("L1"),
      subj: "utterance".to_string(),
      pred: "script".to_string(),
      obj: "hangul".to_string(),
      status: MeaningStatus::Accepted,
      confidence: 1.0,
      provenance_refs: vec!["lang.ko:script".to_string()],
      proof_refs: vec!["lang.ko:syllable".to_string()],
      contradiction_refs: vec![],
      loss: korean_language_loss(LossKind::Lossless, "hangul syllable decomposition"),
      timestamp: None,
    },
    ContextualFact {
      id: Some(MeaningId::from("fact.dialogue.ko.003")),
      context: context.clone(),
      layer: LayerId::from("L1"),
      subj: "utterance".to_string(),
      pred: "token-count".to_string(),
      obj: analysis.tokens.len().to_string(),
      status: MeaningStatus::Accepted,
      confidence: 1.0,
      provenance_refs: vec!["lang.ko:tokenization".to_string()],
      proof_refs: vec![],
      contradiction_refs: vec![],
      loss: korean_language_loss(LossKind::Lossless, "normalized Korean token count"),
      timestamp: None,
    },
    ContextualFact {
      id: Some(MeaningId::from("fact.dialogue.ko.004")),
      context: context.clone(),
      layer: LayerId::from("L1"),
      subj: "utterance".to_string(),
      pred: "hangul-syllable-count".to_string(),
      obj: analysis.hangul_syllables.len().to_string(),
      status: MeaningStatus::Accepted,
      confidence: 1.0,
      provenance_refs: vec!["lang.ko:syllable-count".to_string()],
      proof_refs: vec![],
      contradiction_refs: vec![],
      loss: korean_language_loss(LossKind::Lossless, "hangul syllable counting"),
      timestamp: None,
    },
    ContextualFact {
      id: Some(MeaningId::from("fact.dialogue.ko.005")),
      context: context.clone(),
      layer: LayerId::from("L2"),
      subj: "utterance".to_string(),
      pred: "sentence-mood".to_string(),
      obj: analysis.sentence_mood.as_str().to_string(),
      status: MeaningStatus::Accepted,
      confidence: 0.85,
      provenance_refs: vec!["lang.ko:mood".to_string()],
      proof_refs: vec![],
      contradiction_refs: vec![],
      loss: korean_language_loss(
        LossKind::Lossy,
        "ending-based Korean sentence mood heuristic",
      ),
      timestamp: None,
    },
  ];

  if let Some(final_token) = analysis.final_token {
    facts.push(ContextualFact {
      id: Some(MeaningId::from("fact.dialogue.ko.006")),
      context: context.clone(),
      layer: LayerId::from("L2"),
      subj: "utterance".to_string(),
      pred: "final-token".to_string(),
      obj: final_token,
      status: MeaningStatus::Accepted,
      confidence: 1.0,
      provenance_refs: vec!["lang.ko:final-token".to_string()],
      proof_refs: vec![],
      contradiction_refs: vec![],
      loss: korean_language_loss(LossKind::Lossless, "normalized final token capture"),
      timestamp: None,
    });
  }

  if let Some(first_syllable) = analysis.hangul_syllables.first() {
    facts.push(ContextualFact {
      id: Some(MeaningId::from("fact.dialogue.ko.007")),
      context: context.clone(),
      layer: LayerId::from("L1"),
      subj: "utterance".to_string(),
      pred: "first-syllable-jamo".to_string(),
      obj: first_syllable.jamo_string(),
      status: MeaningStatus::Accepted,
      confidence: 1.0,
      provenance_refs: vec!["lang.ko:jamo".to_string()],
      proof_refs: vec![],
      contradiction_refs: vec![],
      loss: korean_language_loss(LossKind::Lossless, "first hangul syllable decomposition"),
      timestamp: None,
    });
  }

  for (index, particle) in analysis.particles.iter().enumerate() {
    facts.push(ContextualFact {
      id: Some(MeaningId::from(format!(
        "fact.dialogue.ko.{:03}",
        100 + index
      ))),
      context: context.clone(),
      layer: LayerId::from("L2"),
      subj: "utterance".to_string(),
      pred: "has-particle".to_string(),
      obj: particle.kind.as_str().to_string(),
      status: MeaningStatus::Accepted,
      confidence: 0.80,
      provenance_refs: vec![
        "lang.ko:particle".to_string(),
        format!("token:{}", particle.token),
        format!("surface:{}", particle.surface),
      ],
      proof_refs: vec![],
      contradiction_refs: vec![],
      loss: korean_language_loss(LossKind::Lossy, "suffix-based Korean particle detection"),
      timestamp: None,
    });
  }

  facts
}

fn arithmetic_expression_projection(
  plan: &ToolActionPlan,
  solution: ArithmeticSolution,
) -> ExpressionProjectionRecord {
  ExpressionProjectionRecord {
    id: ExpressionProjectionId::from("expr.dialogue.arithmetic.001"),
    context: ContextId::from("Math.Arithmetic.Basic"),
    layer: LayerId::from("L4"),
    subject: "산술-식".to_string(),
    projection_family: "expmath".to_string(),
    canonical_form: "2 + 2 = 4".to_string(),
    semantic_fact_refs: vec![
      "fact.dialogue.result.001".to_string(),
      "fact.dialogue.result.002".to_string(),
      "fact.dialogue.result.003".to_string(),
      "fact.dialogue.result.004".to_string(),
    ],
    surface_forms: BTreeMap::from([
      ("canonical-text".to_string(), format!("{} = {}", solution.expression, solution.result)),
      (
        "openmath".to_string(),
        "<OMOBJ><OMA><OMS cd=\"relation1\" name=\"eq\"/><OMA><OMS cd=\"arith1\" name=\"plus\"/><OMI>2</OMI><OMI>2</OMI></OMA><OMI>4</OMI></OMA></OMOBJ>".to_string(),
      ),
      (
        "mathml-content".to_string(),
        "<math><apply><eq/><apply><plus/><cn>2</cn><cn>2</cn></apply><cn>4</cn></apply></math>"
          .to_string(),
      ),
      (
        "freecat-geometry".to_string(),
        "surface.fromOpenMathToGeometry(OMOBJ(arith1.plus(2,2)=4))".to_string(),
      ),
    ]),
    provenance_refs: vec![format!("plan:{}", plan.id.0)],
    artifact_refs: vec!["artifact://doghouse/expression/arithmetic-2-plus-2".to_string()],
    notes: vec![
      "historical freecat path: math meaning -> OpenMath/MathML -> freecat geometry".to_string(),
      "projection family is neutral expmath; OpenMath is one downstream formal surface".to_string(),
    ],
  }
}

fn newton_force_expression_projection(
  plan: &ToolActionPlan,
  solution: NewtonForceSolution,
) -> ExpressionProjectionRecord {
  ExpressionProjectionRecord {
    id: ExpressionProjectionId::from("expr.dialogue.physics.001"),
    context: ContextId::from("Physics.Newtonian.Force"),
    layer: LayerId::from("L4"),
    subject: "뉴턴-제2법칙".to_string(),
    projection_family: "expmath".to_string(),
    canonical_form: "F = m * a = 6 N".to_string(),
    semantic_fact_refs: vec![
      "fact.dialogue.result.001".to_string(),
      "fact.dialogue.result.002".to_string(),
      "fact.dialogue.result.003".to_string(),
      "fact.dialogue.result.004".to_string(),
      "fact.dialogue.result.005".to_string(),
    ],
    surface_forms: BTreeMap::from([
      (
        "canonical-text".to_string(),
        format!("F = m * a = {} * {} = {} N", solution.mass_kg, solution.acceleration_mps2, solution.force_n),
      ),
      (
        "openmath".to_string(),
        "<OMOBJ><OMA><OMS cd=\"relation1\" name=\"eq\"/><OMV name=\"F\"/><OMA><OMS cd=\"arith1\" name=\"times\"/><OMI>2</OMI><OMI>3</OMI></OMA></OMA></OMOBJ>".to_string(),
      ),
      (
        "mathml-content".to_string(),
        "<math><apply><eq/><ci>F</ci><apply><times/><cn>2</cn><cn>3</cn></apply></apply></math>"
          .to_string(),
      ),
      (
        "freecat-geometry".to_string(),
        "surface.fromOpenMathToGeometry(OMOBJ(F = arith1.times(2,3)))".to_string(),
      ),
    ]),
    provenance_refs: vec![format!("plan:{}", plan.id.0)],
    artifact_refs: vec!["artifact://doghouse/expression/newton-force".to_string()],
    notes: vec![
      "units stay in semantic facts; formal projection keeps the calculable relation".to_string(),
      "projection family is neutral expmath and can later emit OpenMath/MathML/freecat render surfaces".to_string(),
    ],
  }
}

fn tool_dialogue_spec(utterance: &str) -> Option<ToolDialogueSpec> {
  match utterance.trim() {
    "블렌더에서 큐브 생성해"
    | "블렌더에서 큐브 만들어"
    | "블렌더에서 큐브 만들어줘"
    | "Blender에서 큐브 생성해" => Some(ToolDialogueSpec {
      kind: ToolDialogueKind::ExternalTool,
      tool_name: "blender",
      runtime: "puckBlend",
      context: "Tool.Blender",
      action_name: "mesh.create-cube",
      args: &[("primitive", "cube"), ("size", "2")],
      object: "cube",
      summary: "블렌더 큐브 생성 계획을 doghouse에 기록하고 puck adaptor로 보냅니다.",
    }),
    "라이노에서 선 그어" | "라이노에서 line 그어" | "Rhino에서 선 그어" => {
      Some(ToolDialogueSpec {
        kind: ToolDialogueKind::ExternalTool,
        tool_name: "rhino",
        runtime: "puckRH",
        context: "Tool.Rhino",
        action_name: "command.run",
        args: &[("command", "Line")],
        object: "line",
        summary: "라이노 Line 명령 계획을 doghouse에 기록하고 puck adaptor로 보냅니다.",
      })
    }
    "아인슈타인 퍼즐 풀어줘"
    | "얼룩말 퍼즐 풀어줘"
    | "아인슈타인 퍼즐에서 물고기를 기르는 사람은 누구야?"
    | "아인슈타인 퍼즐에서 물고기 주인은 누구야?"
    | "얼룩말 퍼즐에서 물고기를 기르는 사람은 누구야?" => {
      Some(ToolDialogueSpec {
        kind: ToolDialogueKind::OntologyRuntime,
        tool_name: "ontology-runtime",
        runtime: "doghouse",
        context: "Logic.Zebra",
        action_name: "logic.solve-zebra",
        args: &[("puzzle", "einstein-zebra"), ("question", "fish-owner")],
        object: "einstein-puzzle",
        summary: "아인슈타인 퍼즐 제약을 ontology runtime에서 계산하고 doghouse에 기록합니다.",
      })
    }
    "2 더하기 2는 뭐야?" | "2+2는 뭐야?" | "2 더하기 2 계산해" | "2 더하기 2 알려줘" => {
      Some(ToolDialogueSpec {
        kind: ToolDialogueKind::OntologyRuntime,
        tool_name: "ontology-runtime",
        runtime: "doghouse",
        context: "Math.Arithmetic",
        action_name: "math.solve-arithmetic",
        args: &[("expression", "2+2"), ("result-domain", "integer")],
        object: "arithmetic-expression",
        summary: "정수 산술 식을 ontology runtime에서 계산하고 doghouse에 기록합니다.",
      })
    }
    "질량 2kg이고 가속도 3m/s^2이면 힘은 얼마야?"
    | "질량 2kg, 가속도 3m/s^2일 때 힘은?"
    | "뉴턴 제2법칙으로 질량 2kg 가속도 3m/s^2면 힘은 얼마야?" => {
      Some(ToolDialogueSpec {
        kind: ToolDialogueKind::OntologyRuntime,
        tool_name: "ontology-runtime",
        runtime: "doghouse",
        context: "Physics.Newtonian",
        action_name: "physics.compute-force",
        args: &[("mass-kg", "2"), ("acceleration-mps2", "3")],
        object: "force-equation",
        summary: "뉴턴 제2법칙 F=ma를 ontology runtime에서 계산하고 doghouse에 기록합니다.",
      })
    }
    "빛은 뭐야?" | "빛은 파동이야 입자야?" | "빛은 파동이야, 입자야?" | "빛의 정체는 뭐야?" => {
      Some(ToolDialogueSpec {
        kind: ToolDialogueKind::OntologyRuntime,
        tool_name: "ontology-runtime",
        runtime: "doghouse",
        context: "Physics.Light.General",
        action_name: "physics.interpret-light-general",
        args: &[("target", "light"), ("context-required", "true")],
        object: "light",
        summary:
          "빛 해석은 실험 context가 필요하므로 held judgement와 대체 context들을 기록합니다.",
      })
    }
    "이중슬릿 실험에서 빛은 뭐야?"
    | "이중슬릿에서 빛은 뭐야?"
    | "이중슬릿 실험에서 빛은 파동이야?" => Some(ToolDialogueSpec {
      kind: ToolDialogueKind::OntologyRuntime,
      tool_name: "ontology-runtime",
      runtime: "doghouse",
      context: "Physics.Light.DoubleSlit",
      action_name: "physics.interpret-light",
      args: &[("target", "light"), ("experiment", "double-slit")],
      object: "light",
      summary: "이중슬릿 context에서 빛의 파동성 해석을 ontology runtime에서 계산합니다.",
    }),
    "광전효과에서 빛은 뭐야?" | "광전효과에서 빛은 입자야?" | "광전효과로 보면 빛은 뭐야?" => {
      Some(ToolDialogueSpec {
        kind: ToolDialogueKind::OntologyRuntime,
        tool_name: "ontology-runtime",
        runtime: "doghouse",
        context: "Physics.Light.Photoelectric",
        action_name: "physics.interpret-light",
        args: &[("target", "light"), ("experiment", "photoelectric")],
        object: "light",
        summary: "광전효과 context에서 빛의 입자성 해석을 ontology runtime에서 계산합니다.",
      })
    }
    _ => None,
  }
}

fn house_permutations() -> Vec<[usize; 5]> {
  fn recurse(
    depth: usize,
    used: &mut [bool; 5],
    current: &mut [usize; 5],
    out: &mut Vec<[usize; 5]>,
  ) {
    if depth == 5 {
      out.push(*current);
      return;
    }

    for value in 0..5 {
      if used[value] {
        continue;
      }
      used[value] = true;
      current[depth] = value;
      recurse(depth + 1, used, current, out);
      used[value] = false;
    }
  }

  let mut out = Vec::with_capacity(120);
  recurse(0, &mut [false; 5], &mut [0; 5], &mut out);
  out
}

fn is_neighbor(left: usize, right: usize) -> bool {
  left.abs_diff(right) == 1
}

fn nationality_ko_at_house(nationalities: &[usize; 5], house: usize) -> &'static str {
  if nationalities[0] == house {
    "영국인"
  } else if nationalities[1] == house {
    "스웨덴인"
  } else if nationalities[2] == house {
    "덴마크인"
  } else if nationalities[3] == house {
    "노르웨이인"
  } else {
    "독일인"
  }
}

fn solve_standard_zebra_puzzle() -> ZebraPuzzleSolution {
  const RED: usize = 0;
  const GREEN: usize = 1;
  const WHITE: usize = 2;
  const YELLOW: usize = 3;
  const BLUE: usize = 4;

  const BRIT: usize = 0;
  const SWEDE: usize = 1;
  const DANE: usize = 2;
  const NORWEGIAN: usize = 3;
  const GERMAN: usize = 4;

  const TEA: usize = 0;
  const COFFEE: usize = 1;
  const MILK: usize = 2;
  const BEER: usize = 3;
  const WATER: usize = 4;

  const PALL_MALL: usize = 0;
  const DUNHILL: usize = 1;
  const BLEND: usize = 2;
  const BLUE_MASTER: usize = 3;
  const PRINCE: usize = 4;

  const DOG: usize = 0;
  const BIRD: usize = 1;
  const CAT: usize = 2;
  const HORSE: usize = 3;
  const FISH: usize = 4;

  let permutations = house_permutations();

  for colors in &permutations {
    if colors[GREEN] + 1 != colors[WHITE] {
      continue;
    }

    for nationalities in &permutations {
      if nationalities[BRIT] != colors[RED] {
        continue;
      }
      if nationalities[NORWEGIAN] != 0 {
        continue;
      }
      if !is_neighbor(nationalities[NORWEGIAN], colors[BLUE]) {
        continue;
      }

      for drinks in &permutations {
        if nationalities[DANE] != drinks[TEA] {
          continue;
        }
        if colors[GREEN] != drinks[COFFEE] {
          continue;
        }
        if drinks[MILK] != 2 {
          continue;
        }

        for smokes in &permutations {
          if smokes[DUNHILL] != colors[YELLOW] {
            continue;
          }
          if smokes[BLUE_MASTER] != drinks[BEER] {
            continue;
          }
          if smokes[PRINCE] != nationalities[GERMAN] {
            continue;
          }
          if !is_neighbor(smokes[BLEND], drinks[WATER]) {
            continue;
          }

          for pets in &permutations {
            if nationalities[SWEDE] != pets[DOG] {
              continue;
            }
            if smokes[PALL_MALL] != pets[BIRD] {
              continue;
            }
            if !is_neighbor(smokes[BLEND], pets[CAT]) {
              continue;
            }
            if !is_neighbor(pets[HORSE], smokes[DUNHILL]) {
              continue;
            }

            return ZebraPuzzleSolution {
              fish_owner_ko: nationality_ko_at_house(nationalities, pets[FISH]),
              water_drinker_ko: nationality_ko_at_house(nationalities, drinks[WATER]),
            };
          }
        }
      }
    }
  }

  unreachable!("standard zebra puzzle has one deterministic solution")
}

fn solve_demo_arithmetic() -> ArithmeticSolution {
  ArithmeticSolution {
    expression: "2+2",
    result: 4,
  }
}

fn solve_demo_newton_force() -> NewtonForceSolution {
  let mass_kg = 2.0;
  let acceleration_mps2 = 3.0;
  NewtonForceSolution {
    mass_kg,
    acceleration_mps2,
    force_n: mass_kg * acceleration_mps2,
  }
}

fn solve_demo_light_context(context: &str) -> Option<LightContextSolution> {
  match context {
    "Physics.Light.DoubleSlit" => Some(LightContextSolution {
      context_ko: "이중슬릿 실험",
      manifestation: "wave-like-interference",
      phenomenon: "간섭무늬",
    }),
    "Physics.Light.Photoelectric" => Some(LightContextSolution {
      context_ko: "광전효과",
      manifestation: "particle-like-quanta",
      phenomenon: "광전자 방출",
    }),
    _ => None,
  }
}

fn solve_demo_light_ambiguity() -> LightAmbiguitySolution {
  LightAmbiguitySolution {
    reason: "빛은 context 없이 하나의 manifestation으로 수렴하지 않는다",
    wave_context: "이중슬릿 실험",
    wave_manifestation: "wave-like-interference",
    particle_context: "광전효과",
    particle_manifestation: "particle-like-quanta",
  }
}

fn tool_dialogue_domain_facts(spec: &ToolDialogueSpec) -> Vec<ContextualFact> {
  match spec.action_name {
    "physics.interpret-light-general" => vec![
      ContextualFact {
        id: Some(MeaningId::from("fact.dialogue.domain.001")),
        context: ContextId::from("Physics.Light.General"),
        layer: LayerId::from("L3"),
        subj: "light".to_string(),
        pred: "requires-context".to_string(),
        obj: "experimental-setup".to_string(),
        status: MeaningStatus::Accepted,
        confidence: 1.0,
        provenance_refs: vec!["physics:light:context-required".to_string()],
        proof_refs: vec!["artifact://doghouse/physics/light-context-map".to_string()],
        contradiction_refs: vec![],
        loss: Some(LossPolicy {
          kind: LossKind::Lossless,
          notes: vec!["light interpretation depends on experimental context".to_string()],
        }),
        timestamp: None,
      },
      ContextualFact {
        id: Some(MeaningId::from("fact.dialogue.domain.002")),
        context: ContextId::from("Physics.Light.DoubleSlit"),
        layer: LayerId::from("L3"),
        subj: "light".to_string(),
        pred: "manifests-as".to_string(),
        obj: "wave-like-interference".to_string(),
        status: MeaningStatus::Accepted,
        confidence: 1.0,
        provenance_refs: vec!["physics:light:double-slit".to_string()],
        proof_refs: vec!["artifact://doghouse/physics/light-context-map".to_string()],
        contradiction_refs: vec![],
        loss: Some(LossPolicy {
          kind: LossKind::Lossless,
          notes: vec!["wave manifestation is valid under double-slit context".to_string()],
        }),
        timestamp: None,
      },
      ContextualFact {
        id: Some(MeaningId::from("fact.dialogue.domain.003")),
        context: ContextId::from("Physics.Light.Photoelectric"),
        layer: LayerId::from("L3"),
        subj: "light".to_string(),
        pred: "manifests-as".to_string(),
        obj: "particle-like-quanta".to_string(),
        status: MeaningStatus::Accepted,
        confidence: 1.0,
        provenance_refs: vec!["physics:light:photoelectric".to_string()],
        proof_refs: vec!["artifact://doghouse/physics/light-context-map".to_string()],
        contradiction_refs: vec![],
        loss: Some(LossPolicy {
          kind: LossKind::Lossless,
          notes: vec!["particle manifestation is valid under photoelectric context".to_string()],
        }),
        timestamp: None,
      },
    ],
    "physics.interpret-light" => solve_demo_light_context(spec.context)
      .map(|solution| {
        vec![
          ContextualFact {
            id: Some(MeaningId::from("fact.dialogue.domain.001")),
            context: ContextId::from(spec.context),
            layer: LayerId::from("L3"),
            subj: "experiment".to_string(),
            pred: "kind".to_string(),
            obj: solution.context_ko.to_string(),
            status: MeaningStatus::Accepted,
            confidence: 1.0,
            provenance_refs: vec![format!("physics:light:{}", spec.context)],
            proof_refs: vec!["artifact://doghouse/physics/light-context-map".to_string()],
            contradiction_refs: vec![],
            loss: Some(LossPolicy {
              kind: LossKind::Lossless,
              notes: vec!["explicit experiment context supplied".to_string()],
            }),
            timestamp: None,
          },
          ContextualFact {
            id: Some(MeaningId::from("fact.dialogue.domain.002")),
            context: ContextId::from(spec.context),
            layer: LayerId::from("L3"),
            subj: "light".to_string(),
            pred: "manifests-as".to_string(),
            obj: solution.manifestation.to_string(),
            status: MeaningStatus::Accepted,
            confidence: 1.0,
            provenance_refs: vec![format!("physics:light:{}", spec.context)],
            proof_refs: vec!["artifact://doghouse/physics/light-context-map".to_string()],
            contradiction_refs: vec![],
            loss: Some(LossPolicy {
              kind: LossKind::Lossless,
              notes: vec!["context-specific light interpretation".to_string()],
            }),
            timestamp: None,
          },
        ]
      })
      .unwrap_or_default(),
    _ => Vec::new(),
  }
}

fn tool_dialogue_policy() -> EvaluationPolicy {
  EvaluationPolicy {
    id: "ontology.demo.korean-tool-dialogue".to_string(),
    weights: EvaluationWeights {
      coherence: 1.0,
      coverage: 2.2,
      loss: 0.4,
      cost: 0.3,
      replayability: 1.3,
      safety: 1.4,
    },
    ..EvaluationPolicy::default()
  }
}

fn tool_dialogue_policy_for(spec: &ToolDialogueSpec) -> EvaluationPolicy {
  let mut policy = tool_dialogue_policy();
  if spec.action_name == "physics.interpret-light-general" {
    policy.id = "ontology.demo.korean-tool-dialogue.light-general".to_string();
    policy.accept_threshold = 0.99;
    policy.hold_threshold = 0.60;
  }
  policy
}

fn tool_dialogue_answer(
  spec: &ToolDialogueSpec,
  selected: &SelectionOutcome,
  runtime_result: Option<RuntimeDialogueResult>,
) -> String {
  if let Some(result) = runtime_result {
    return match result {
      RuntimeDialogueResult::Zebra(solution) => format!(
        "runtime ontology 계산 결과: 물고기를 기르는 사람은 {}; 물을 마시는 사람은 {}; judgement={:?}; score={:.2}",
        solution.fish_owner_ko,
        solution.water_drinker_ko,
        selected.judgement.action,
        selected.evaluation.score
      ),
      RuntimeDialogueResult::Arithmetic(solution) => format!(
        "runtime ontology 계산 결과: {} = {}; judgement={:?}; score={:.2}",
        solution.expression,
        solution.result,
        selected.judgement.action,
        selected.evaluation.score
      ),
      RuntimeDialogueResult::NewtonForce(solution) => format!(
        "runtime ontology 계산 결과: F = m*a = {}N (m={}kg, a={}m/s^2); judgement={:?}; score={:.2}",
        solution.force_n,
        solution.mass_kg,
        solution.acceleration_mps2,
        selected.judgement.action,
        selected.evaluation.score
      ),
      RuntimeDialogueResult::LightContext(solution) => format!(
        "runtime ontology 계산 결과: {} context에서 빛은 {}로 해석되고 {}를 보인다; judgement={:?}; score={:.2}",
        solution.context_ko,
        solution.manifestation,
        solution.phenomenon,
        selected.judgement.action,
        selected.evaluation.score
      ),
      RuntimeDialogueResult::LightAmbiguity(solution) => format!(
        "runtime ontology 판단: {}; {}에서는 {}, {}에서는 {}; judgement={:?}; score={:.2}",
        solution.reason,
        solution.wave_context,
        solution.wave_manifestation,
        solution.particle_context,
        solution.particle_manifestation,
        selected.judgement.action,
        selected.evaluation.score
      ),
    };
  }

  format!(
    "judgement={:?}; tool={}; action={}; summary={}; score={:.2}",
    selected.judgement.action,
    spec.tool_name,
    spec.action_name,
    spec.summary,
    selected.evaluation.score
  )
}

pub fn run_sentence_judgement_demo(
  utterance: &str,
  jurisdiction: &str,
) -> Option<SentenceJudgementDemo> {
  if !normalized_lawyer_goal(utterance) {
    return None;
  }

  let (jurisdiction, mut facts) = career_schema_facts(jurisdiction);
  let observation = ContextualFact {
    id: Some(MeaningId::from("fact.obs.001")),
    context: ContextId::from(jurisdiction.clone()),
    layer: LayerId::from("L4"),
    subj: "user".to_string(),
    pred: "wants-career".to_string(),
    obj: "lawyer".to_string(),
    status: MeaningStatus::Candidate,
    confidence: 1.0,
    provenance_refs: vec![format!("utterance:{}", utterance.trim())],
    proof_refs: vec![],
    contradiction_refs: vec![],
    loss: Some(LossPolicy {
      kind: LossKind::Lossless,
      notes: vec!["controlled utterance normalization".to_string()],
    }),
    timestamp: None,
  };
  facts.insert(0, observation.clone());

  let interpretations = vec![
    Interpretation {
      id: InterpretationId::from("interp.career.intent-only"),
      observation_refs: vec!["utterance".to_string()],
      fact_refs: vec!["fact.obs.001".to_string()],
      lift_refs: vec![],
      conflict_refs: vec![],
      loss: None,
    },
    Interpretation {
      id: InterpretationId::from("interp.career.path.kr"),
      observation_refs: vec!["utterance".to_string()],
      fact_refs: vec![
        "fact.obs.001".to_string(),
        "fact.schema.001".to_string(),
        "fact.schema.002".to_string(),
      ],
      lift_refs: vec![format!("lift:{jurisdiction}:career-path")],
      conflict_refs: vec![],
      loss: Some(LossPolicy {
        kind: LossKind::Lossy,
        notes: vec!["personal aptitude and finances omitted".to_string()],
      }),
    },
  ];
  let policy = demo_policy();
  let selected = ontology_select(&policy, &interpretations, &facts)?;
  let promotion = ontology_promote(&policy, &selected.judgement);
  let answer = answer_for_demo(&jurisdiction, &selected, &facts);

  Some(SentenceJudgementDemo {
    utterance: utterance.to_string(),
    observation,
    facts,
    interpretations,
    selected,
    promotion,
    answer,
  })
}

pub fn run_korean_tool_dialogue_demo(utterance: &str) -> Option<KoreanToolDialogueDemo> {
  let spec = tool_dialogue_spec(utterance)?;
  let korean_facts = korean_language_facts(utterance, spec.context);
  let korean_fact_refs = korean_facts
    .iter()
    .filter_map(|fact| fact.id.as_ref().map(|id| id.0.clone()))
    .collect::<Vec<_>>();
  let domain_facts = tool_dialogue_domain_facts(&spec);
  let domain_fact_refs = domain_facts
    .iter()
    .filter_map(|fact| fact.id.as_ref().map(|id| id.0.clone()))
    .collect::<Vec<_>>();

  let observation = ContextualFact {
    id: Some(MeaningId::from("fact.dialogue.obs.001")),
    context: ContextId::from(spec.context),
    layer: LayerId::from("L4"),
    subj: "user".to_string(),
    pred: "requests-tool-action".to_string(),
    obj: format!("{}:{}", spec.tool_name, spec.action_name),
    status: MeaningStatus::Candidate,
    confidence: 1.0,
    provenance_refs: vec![format!("utterance:{}", utterance.trim())],
    proof_refs: vec![],
    contradiction_refs: vec![],
    loss: Some(LossPolicy {
      kind: LossKind::Lossless,
      notes: vec!["ontology-native Korean normalization".to_string()],
    }),
    timestamp: None,
  };

  let tool_capability_fact = ContextualFact {
    id: Some(MeaningId::from("fact.dialogue.capability.001")),
    context: ContextId::from(spec.context),
    layer: LayerId::from("L4"),
    subj: spec.tool_name.to_string(),
    pred: "supports-action".to_string(),
    obj: spec.action_name.to_string(),
    status: MeaningStatus::Accepted,
    confidence: 1.0,
    provenance_refs: vec![format!("capability:{}:{}", spec.runtime, spec.action_name)],
    proof_refs: vec![format!("tool://{}/{}", spec.tool_name, spec.action_name)],
    contradiction_refs: vec![],
    loss: None,
    timestamp: None,
  };

  let tool_object_fact = ContextualFact {
    id: Some(MeaningId::from("fact.dialogue.capability.002")),
    context: ContextId::from(spec.context),
    layer: LayerId::from("L4"),
    subj: spec.tool_name.to_string(),
    pred: "acts-on".to_string(),
    obj: spec.object.to_string(),
    status: MeaningStatus::Accepted,
    confidence: 1.0,
    provenance_refs: vec![format!("schema:{}:{}", spec.tool_name, spec.object)],
    proof_refs: vec![],
    contradiction_refs: vec![],
    loss: None,
    timestamp: None,
  };

  let mut facts = vec![observation.clone()];
  facts.extend(korean_facts);
  facts.extend(domain_facts);
  facts.push(tool_capability_fact.clone());
  facts.push(tool_object_fact.clone());

  let mut dispatch_fact_refs = vec![
    "fact.dialogue.obs.001".to_string(),
    "fact.dialogue.capability.001".to_string(),
    "fact.dialogue.capability.002".to_string(),
  ];
  dispatch_fact_refs.extend(korean_fact_refs);
  dispatch_fact_refs.extend(domain_fact_refs);

  let interpretations = vec![
    Interpretation {
      id: InterpretationId::from("interp.tool.intent-only"),
      observation_refs: vec!["utterance".to_string()],
      fact_refs: vec!["fact.dialogue.obs.001".to_string()],
      lift_refs: vec![],
      conflict_refs: vec![],
      loss: Some(LossPolicy {
        kind: LossKind::Lossy,
        notes: vec!["tool runtime not chosen yet".to_string()],
      }),
    },
    Interpretation {
      id: InterpretationId::from(format!("interp.tool.dispatch.{}", spec.tool_name)),
      observation_refs: vec!["utterance".to_string()],
      fact_refs: dispatch_fact_refs,
      lift_refs: vec![format!("lift:{}:dispatch", spec.tool_name)],
      conflict_refs: vec![],
      loss: Some(if spec.action_name == "physics.interpret-light-general" {
        LossPolicy {
          kind: LossKind::Lossy,
          notes: vec!["general light question omitted the experimental context".to_string()],
        }
      } else {
        LossPolicy {
          kind: LossKind::Lossless,
          notes: vec!["tool action fully grounded".to_string()],
        }
      }),
    },
  ];

  let policy = tool_dialogue_policy_for(&spec);
  let selected = ontology_select(&policy, &interpretations, &facts)?;
  let promotion = ontology_promote(&policy, &selected.judgement);
  let transport = match spec.kind {
    ToolDialogueKind::ExternalTool => ToolTransportKind::Plugin,
    ToolDialogueKind::OntologyRuntime => ToolTransportKind::Embedded,
  };
  let capability = ToolCapabilityRecord {
    id: ToolCapabilityId::from(format!("tool.{}.{}", spec.tool_name, spec.runtime)),
    tool_name: spec.tool_name.to_string(),
    adapter_runtime: spec.runtime.to_string(),
    transport,
    executable_actions: vec![spec.action_name.to_string()],
    query_actions: vec!["health.check".to_string()],
    observable_events: vec!["tool.action.completed".to_string()],
    artifact_kinds: vec!["semantic-record".to_string(), "tool-artifact".to_string()],
    notes: match spec.kind {
      ToolDialogueKind::ExternalTool => vec![
        "korean dialogue demo capability".to_string(),
        "puck dispatches after doghouse judgement".to_string(),
      ],
      ToolDialogueKind::OntologyRuntime => vec![
        "korean runtime ontology dialogue capability".to_string(),
        "doghouse computes the answer before puck surfaces it".to_string(),
      ],
    },
  };
  let plan = ToolActionPlan {
    id: ToolActionPlanId::from(format!("plan.dialogue.{}", spec.tool_name)),
    capability: capability.id.clone(),
    judgement: selected.judgement.id.clone(),
    chosen_interpretation: selected.judgement.chosen_interpretation.clone(),
    action_name: spec.action_name.to_string(),
    args: spec
      .args
      .iter()
      .map(|(key, value)| (key.to_string(), value.to_string()))
      .collect(),
    provenance_refs: observation.provenance_refs.clone(),
    notes: vec![spec.summary.to_string()],
  };
  let runtime_result = match spec.kind {
    ToolDialogueKind::ExternalTool => None,
    ToolDialogueKind::OntologyRuntime => match spec.action_name {
      "logic.solve-zebra" => Some(RuntimeDialogueResult::Zebra(solve_standard_zebra_puzzle())),
      "math.solve-arithmetic" => Some(RuntimeDialogueResult::Arithmetic(solve_demo_arithmetic())),
      "physics.compute-force" => {
        Some(RuntimeDialogueResult::NewtonForce(solve_demo_newton_force()))
      }
      "physics.interpret-light-general" => Some(RuntimeDialogueResult::LightAmbiguity(
        solve_demo_light_ambiguity(),
      )),
      "physics.interpret-light" => {
        solve_demo_light_context(spec.context).map(RuntimeDialogueResult::LightContext)
      }
      _ => None,
    },
  };
  let (semantic_facts, expression_projections, artifact_refs, result_summary, result_notes) =
    if let Some(result) = runtime_result {
      match result {
        RuntimeDialogueResult::Zebra(solution) => (
          vec![
            ContextualFact {
              id: Some(MeaningId::from("fact.dialogue.result.001")),
              context: ContextId::from(spec.context),
              layer: LayerId::from("L4"),
              subj: "아인슈타인-퍼즐".to_string(),
              pred: "fish-owner".to_string(),
              obj: solution.fish_owner_ko.to_string(),
              status: MeaningStatus::Accepted,
              confidence: 1.0,
              provenance_refs: vec![format!("plan:{}", plan.id.0)],
              proof_refs: vec!["artifact://doghouse/ontology-runtime/zebra-solution".to_string()],
              contradiction_refs: vec![],
              loss: Some(LossPolicy {
                kind: LossKind::Lossless,
                notes: vec!["standard zebra puzzle constraints solved at runtime".to_string()],
              }),
              timestamp: None,
            },
            ContextualFact {
              id: Some(MeaningId::from("fact.dialogue.result.002")),
              context: ContextId::from(spec.context),
              layer: LayerId::from("L4"),
              subj: "아인슈타인-퍼즐".to_string(),
              pred: "water-drinker".to_string(),
              obj: solution.water_drinker_ko.to_string(),
              status: MeaningStatus::Accepted,
              confidence: 1.0,
              provenance_refs: vec![format!("plan:{}", plan.id.0)],
              proof_refs: vec!["artifact://doghouse/ontology-runtime/zebra-solution".to_string()],
              contradiction_refs: vec![],
              loss: Some(LossPolicy {
                kind: LossKind::Lossless,
                notes: vec!["standard zebra puzzle constraints solved at runtime".to_string()],
              }),
              timestamp: None,
            },
          ],
          vec![],
          vec!["artifact://doghouse/ontology-runtime/zebra-solution".to_string()],
          format!(
            "ontology runtime solved zebra puzzle; fish-owner={}; water-drinker={}",
            solution.fish_owner_ko, solution.water_drinker_ko
          ),
          vec![
            "runtime ontology result for korean dialogue".to_string(),
            "constraint satisfaction closed loop executed inside doghouse".to_string(),
          ],
        ),
        RuntimeDialogueResult::Arithmetic(solution) => (
          vec![
            ContextualFact {
              id: Some(MeaningId::from("fact.dialogue.result.001")),
              context: ContextId::from(spec.context),
              layer: LayerId::from("L4"),
              subj: "산술-식".to_string(),
              pred: "expression".to_string(),
              obj: solution.expression.to_string(),
              status: MeaningStatus::Accepted,
              confidence: 1.0,
              provenance_refs: vec![format!("plan:{}", plan.id.0)],
              proof_refs: vec![
                "artifact://doghouse/ontology-runtime/arithmetic-solution".to_string()
              ],
              contradiction_refs: vec![],
              loss: Some(LossPolicy {
                kind: LossKind::Lossless,
                notes: vec!["integer arithmetic solved at runtime".to_string()],
              }),
              timestamp: None,
            },
            ContextualFact {
              id: Some(MeaningId::from("fact.dialogue.result.002")),
              context: ContextId::from(spec.context),
              layer: LayerId::from("L4"),
              subj: "산술-식".to_string(),
              pred: "result".to_string(),
              obj: solution.result.to_string(),
              status: MeaningStatus::Accepted,
              confidence: 1.0,
              provenance_refs: vec![format!("plan:{}", plan.id.0)],
              proof_refs: vec![
                "artifact://doghouse/ontology-runtime/arithmetic-solution".to_string()
              ],
              contradiction_refs: vec![],
              loss: Some(LossPolicy {
                kind: LossKind::Lossless,
                notes: vec!["integer arithmetic solved at runtime".to_string()],
              }),
              timestamp: None,
            },
            ContextualFact {
              id: Some(MeaningId::from("fact.dialogue.result.003")),
              context: ContextId::from(spec.context),
              layer: LayerId::from("L4"),
              subj: "산술-식".to_string(),
              pred: "projection-surface".to_string(),
              obj: "math-expression".to_string(),
              status: MeaningStatus::Accepted,
              confidence: 1.0,
              provenance_refs: vec![format!("plan:{}", plan.id.0)],
              proof_refs: vec!["artifact://doghouse/expression/arithmetic-2-plus-2".to_string()],
              contradiction_refs: vec![],
              loss: Some(LossPolicy {
                kind: LossKind::Lossless,
                notes: vec![
                  "computed arithmetic can be projected into a formal math surface".to_string(),
                ],
              }),
              timestamp: None,
            },
            ContextualFact {
              id: Some(MeaningId::from("fact.dialogue.result.004")),
              context: ContextId::from(spec.context),
              layer: LayerId::from("L4"),
              subj: "산술-식".to_string(),
              pred: "expression-normal-form".to_string(),
              obj: "2 + 2 = 4".to_string(),
              status: MeaningStatus::Accepted,
              confidence: 1.0,
              provenance_refs: vec![format!("plan:{}", plan.id.0)],
              proof_refs: vec!["artifact://doghouse/expression/arithmetic-2-plus-2".to_string()],
              contradiction_refs: vec![],
              loss: Some(LossPolicy {
                kind: LossKind::Lossless,
                notes: vec!["render-ready arithmetic expression surface".to_string()],
              }),
              timestamp: None,
            },
          ],
          vec![arithmetic_expression_projection(&plan, solution)],
          vec![
            "artifact://doghouse/ontology-runtime/arithmetic-solution".to_string(),
            "artifact://doghouse/expression/arithmetic-2-plus-2".to_string(),
          ],
          format!(
            "ontology runtime solved arithmetic; expression={}; result={}",
            solution.expression, solution.result
          ),
          vec![
            "runtime ontology result for korean math dialogue".to_string(),
            "deterministic arithmetic closed loop executed inside doghouse".to_string(),
            "math meaning is primary; expression projection is a downstream surface".to_string(),
          ],
        ),
        RuntimeDialogueResult::NewtonForce(solution) => (
          vec![
            ContextualFact {
              id: Some(MeaningId::from("fact.dialogue.result.001")),
              context: ContextId::from(spec.context),
              layer: LayerId::from("L4"),
              subj: "뉴턴-제2법칙".to_string(),
              pred: "mass-kg".to_string(),
              obj: solution.mass_kg.to_string(),
              status: MeaningStatus::Accepted,
              confidence: 1.0,
              provenance_refs: vec![format!("plan:{}", plan.id.0)],
              proof_refs: vec!["artifact://doghouse/ontology-runtime/newton-force".to_string()],
              contradiction_refs: vec![],
              loss: Some(LossPolicy {
                kind: LossKind::Lossless,
                notes: vec!["deterministic Newtonian computation".to_string()],
              }),
              timestamp: None,
            },
            ContextualFact {
              id: Some(MeaningId::from("fact.dialogue.result.002")),
              context: ContextId::from(spec.context),
              layer: LayerId::from("L4"),
              subj: "뉴턴-제2법칙".to_string(),
              pred: "acceleration-mps2".to_string(),
              obj: solution.acceleration_mps2.to_string(),
              status: MeaningStatus::Accepted,
              confidence: 1.0,
              provenance_refs: vec![format!("plan:{}", plan.id.0)],
              proof_refs: vec!["artifact://doghouse/ontology-runtime/newton-force".to_string()],
              contradiction_refs: vec![],
              loss: Some(LossPolicy {
                kind: LossKind::Lossless,
                notes: vec!["deterministic Newtonian computation".to_string()],
              }),
              timestamp: None,
            },
            ContextualFact {
              id: Some(MeaningId::from("fact.dialogue.result.003")),
              context: ContextId::from(spec.context),
              layer: LayerId::from("L4"),
              subj: "뉴턴-제2법칙".to_string(),
              pred: "force-n".to_string(),
              obj: solution.force_n.to_string(),
              status: MeaningStatus::Accepted,
              confidence: 1.0,
              provenance_refs: vec![format!("plan:{}", plan.id.0)],
              proof_refs: vec!["artifact://doghouse/ontology-runtime/newton-force".to_string()],
              contradiction_refs: vec![],
              loss: Some(LossPolicy {
                kind: LossKind::Lossless,
                notes: vec!["deterministic Newtonian computation".to_string()],
              }),
              timestamp: None,
            },
            ContextualFact {
              id: Some(MeaningId::from("fact.dialogue.result.004")),
              context: ContextId::from(spec.context),
              layer: LayerId::from("L4"),
              subj: "뉴턴-제2법칙".to_string(),
              pred: "projection-surface".to_string(),
              obj: "math-expression".to_string(),
              status: MeaningStatus::Accepted,
              confidence: 1.0,
              provenance_refs: vec![format!("plan:{}", plan.id.0)],
              proof_refs: vec!["artifact://doghouse/expression/newton-force".to_string()],
              contradiction_refs: vec![],
              loss: Some(LossPolicy {
                kind: LossKind::Lossless,
                notes: vec![
                  "computed physics law can be projected into a formal math surface".to_string(),
                ],
              }),
              timestamp: None,
            },
            ContextualFact {
              id: Some(MeaningId::from("fact.dialogue.result.005")),
              context: ContextId::from(spec.context),
              layer: LayerId::from("L4"),
              subj: "뉴턴-제2법칙".to_string(),
              pred: "expression-normal-form".to_string(),
              obj: "F = m * a = 6 N".to_string(),
              status: MeaningStatus::Accepted,
              confidence: 1.0,
              provenance_refs: vec![format!("plan:{}", plan.id.0)],
              proof_refs: vec!["artifact://doghouse/expression/newton-force".to_string()],
              contradiction_refs: vec![],
              loss: Some(LossPolicy {
                kind: LossKind::Lossless,
                notes: vec!["render-ready physics expression surface".to_string()],
              }),
              timestamp: None,
            },
          ],
          vec![newton_force_expression_projection(&plan, solution)],
          vec![
            "artifact://doghouse/ontology-runtime/newton-force".to_string(),
            "artifact://doghouse/expression/newton-force".to_string(),
          ],
          format!(
            "ontology runtime solved newton force; mass={}; acceleration={}; force={}",
            solution.mass_kg, solution.acceleration_mps2, solution.force_n
          ),
          vec![
            "runtime ontology result for korean physics dialogue".to_string(),
            "deterministic Newtonian closed loop executed inside doghouse".to_string(),
            "physics meaning is primary; expression projection is a downstream surface".to_string(),
          ],
        ),
        RuntimeDialogueResult::LightContext(solution) => (
          vec![
            ContextualFact {
              id: Some(MeaningId::from("fact.dialogue.result.001")),
              context: ContextId::from(spec.context),
              layer: LayerId::from("L4"),
              subj: "light".to_string(),
              pred: "interpreted-in-context".to_string(),
              obj: solution.context_ko.to_string(),
              status: MeaningStatus::Accepted,
              confidence: 1.0,
              provenance_refs: vec![format!("plan:{}", plan.id.0)],
              proof_refs: vec!["artifact://doghouse/physics/light-context-map".to_string()],
              contradiction_refs: vec![],
              loss: Some(LossPolicy {
                kind: LossKind::Lossless,
                notes: vec!["context-specific light interpretation".to_string()],
              }),
              timestamp: None,
            },
            ContextualFact {
              id: Some(MeaningId::from("fact.dialogue.result.002")),
              context: ContextId::from(spec.context),
              layer: LayerId::from("L4"),
              subj: "light".to_string(),
              pred: "manifests-as".to_string(),
              obj: solution.manifestation.to_string(),
              status: MeaningStatus::Accepted,
              confidence: 1.0,
              provenance_refs: vec![format!("plan:{}", plan.id.0)],
              proof_refs: vec!["artifact://doghouse/physics/light-context-map".to_string()],
              contradiction_refs: vec![],
              loss: Some(LossPolicy {
                kind: LossKind::Lossless,
                notes: vec!["context-specific light interpretation".to_string()],
              }),
              timestamp: None,
            },
          ],
          vec![],
          vec!["artifact://doghouse/physics/light-context-map".to_string()],
          format!(
            "ontology runtime interpreted light in {}; manifestation={}",
            solution.context_ko, solution.manifestation
          ),
          vec![
            "runtime ontology result for context-specific korean physics dialogue".to_string(),
            "same target is reinterpreted under a narrower context".to_string(),
          ],
        ),
        RuntimeDialogueResult::LightAmbiguity(solution) => (
          vec![
            ContextualFact {
              id: Some(MeaningId::from("fact.dialogue.result.001")),
              context: ContextId::from("Physics.Light.General"),
              layer: LayerId::from("L4"),
              subj: "light".to_string(),
              pred: "requires-context-before-commit".to_string(),
              obj: "experimental-setup".to_string(),
              status: MeaningStatus::Held,
              confidence: 1.0,
              provenance_refs: vec![format!("plan:{}", plan.id.0)],
              proof_refs: vec!["artifact://doghouse/physics/light-context-map".to_string()],
              contradiction_refs: vec![],
              loss: Some(LossPolicy {
                kind: LossKind::Lossless,
                notes: vec![
                  "general light question stays open until context is specified".to_string(),
                ],
              }),
              timestamp: None,
            },
            ContextualFact {
              id: Some(MeaningId::from("fact.dialogue.result.002")),
              context: ContextId::from("Physics.Light.DoubleSlit"),
              layer: LayerId::from("L4"),
              subj: "light".to_string(),
              pred: "manifests-as".to_string(),
              obj: solution.wave_manifestation.to_string(),
              status: MeaningStatus::Accepted,
              confidence: 1.0,
              provenance_refs: vec![format!("plan:{}", plan.id.0)],
              proof_refs: vec!["artifact://doghouse/physics/light-context-map".to_string()],
              contradiction_refs: vec![],
              loss: Some(LossPolicy {
                kind: LossKind::Lossless,
                notes: vec![
                  "wave interpretation remains valid under double-slit context".to_string(),
                ],
              }),
              timestamp: None,
            },
            ContextualFact {
              id: Some(MeaningId::from("fact.dialogue.result.003")),
              context: ContextId::from("Physics.Light.Photoelectric"),
              layer: LayerId::from("L4"),
              subj: "light".to_string(),
              pred: "manifests-as".to_string(),
              obj: solution.particle_manifestation.to_string(),
              status: MeaningStatus::Accepted,
              confidence: 1.0,
              provenance_refs: vec![format!("plan:{}", plan.id.0)],
              proof_refs: vec!["artifact://doghouse/physics/light-context-map".to_string()],
              contradiction_refs: vec![],
              loss: Some(LossPolicy {
                kind: LossKind::Lossless,
                notes: vec![
                  "particle interpretation remains valid under photoelectric context".to_string(),
                ],
              }),
              timestamp: None,
            },
          ],
          vec![],
          vec!["artifact://doghouse/physics/light-context-map".to_string()],
          format!(
            "ontology runtime held general light question; alternatives={}=>{}, {}=>{}",
            solution.wave_context,
            solution.wave_manifestation,
            solution.particle_context,
            solution.particle_manifestation
          ),
          vec![
            "runtime ontology result for divergent korean physics dialogue".to_string(),
            "general question held until the user chooses a narrower experimental context"
              .to_string(),
          ],
        ),
      }
    } else {
      (
        vec![ContextualFact {
          id: Some(MeaningId::from("fact.dialogue.result.001")),
          context: ContextId::from(spec.context),
          layer: LayerId::from("L4"),
          subj: spec.tool_name.to_string(),
          pred: "scheduled-action".to_string(),
          obj: spec.action_name.to_string(),
          status: MeaningStatus::Accepted,
          confidence: 1.0,
          provenance_refs: vec![format!("plan:{}", plan.id.0)],
          proof_refs: vec![format!("artifact://{}/{}", spec.tool_name, spec.object)],
          contradiction_refs: vec![],
          loss: None,
          timestamp: None,
        }],
        vec![],
        vec![format!("artifact://{}/{}", spec.tool_name, spec.object)],
        format!("{} {} dispatched", spec.tool_name, spec.action_name),
        vec!["demo execution result for puck<->doghouse dialogue".to_string()],
      )
    };
  let result = ToolExecutionResult {
    id: ToolExecutionResultId::from(format!("result.dialogue.{}", spec.tool_name)),
    plan: plan.id.clone(),
    capability: capability.id.clone(),
    status: match promotion.target_status {
      MeaningStatus::Accepted => ToolExecutionStatus::Succeeded,
      MeaningStatus::Held => ToolExecutionStatus::Held,
      MeaningStatus::Rejected | MeaningStatus::Contradicted => ToolExecutionStatus::Rejected,
      _ => ToolExecutionStatus::Succeeded,
    },
    semantic_facts,
    expression_projections,
    artifact_refs,
    provenance_refs: vec![
      format!("judgement:{}", selected.judgement.id),
      format!("capability:{}", capability.id.0),
    ],
    summary: Some(result_summary),
    notes: result_notes,
  };
  let answer = tool_dialogue_answer(&spec, &selected, runtime_result);

  Some(KoreanToolDialogueDemo {
    utterance: utterance.to_string(),
    observation,
    facts,
    interpretations,
    selected,
    promotion,
    capability,
    plan,
    result,
    answer,
  })
}

pub fn run_korean_tool_dialogue_followup_demo(
  initial_utterance: &str,
  follow_up_utterance: &str,
) -> Option<KoreanToolDialogueFollowupDemo> {
  let initial = run_korean_tool_dialogue_demo(initial_utterance)?;
  let mut follow_up = run_korean_tool_dialogue_demo(follow_up_utterance)?;

  follow_up
    .observation
    .provenance_refs
    .push(format!("follow-up-to:{}", initial.selected.judgement.id));
  follow_up
    .observation
    .provenance_refs
    .push(format!("reopens-from:{}", initial.promotion.id));
  follow_up.facts.push(ContextualFact {
    id: Some(MeaningId::from("fact.dialogue.followup.001")),
    context: follow_up.observation.context.clone(),
    layer: LayerId::from("L4"),
    subj: "dialogue".to_string(),
    pred: "reopens-judgement".to_string(),
    obj: initial.selected.judgement.id.clone(),
    status: MeaningStatus::Accepted,
    confidence: 1.0,
    provenance_refs: vec![
      format!("follow-up:{}", initial.utterance),
      format!("follow-up:{}", follow_up.utterance),
    ],
    proof_refs: vec![],
    contradiction_refs: vec![],
    loss: Some(LossPolicy {
      kind: LossKind::Lossless,
      notes: vec!["same dialogue reopened a previously held judgement".to_string()],
    }),
    timestamp: None,
  });
  follow_up.facts.push(ContextualFact {
    id: Some(MeaningId::from("fact.dialogue.followup.002")),
    context: follow_up.observation.context.clone(),
    layer: LayerId::from("L4"),
    subj: "dialogue".to_string(),
    pred: "narrows-context-from".to_string(),
    obj: initial.observation.context.0.clone(),
    status: MeaningStatus::Accepted,
    confidence: 1.0,
    provenance_refs: vec![format!("follow-up-to:{}", initial.selected.judgement.id)],
    proof_refs: vec![],
    contradiction_refs: vec![],
    loss: Some(LossPolicy {
      kind: LossKind::Lossless,
      notes: vec!["follow-up supplied a narrower experimental context".to_string()],
    }),
    timestamp: None,
  });
  follow_up
    .plan
    .notes
    .push(format!("reopened from {}", initial.selected.judgement.id));
  follow_up
    .result
    .provenance_refs
    .push(format!("reopened-from:{}", initial.selected.judgement.id));
  follow_up.result.notes.push(
    "follow-up utterance reopened the previous held judgement under a narrower context".to_string(),
  );
  follow_up.answer = format!("follow-up reopen: {}", follow_up.answer);

  Some(KoreanToolDialogueFollowupDemo { initial, follow_up })
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::ontology::{JudgementAction, MeaningStatus};

  #[test]
  fn sentence_demo_promotes_lawyer_goal_with_provenance() {
    let demo =
      run_sentence_judgement_demo("나 변호사 하고 싶어", "Career.KR").expect("demo result");

    assert_eq!(
      demo.selected.judgement.chosen_interpretation,
      Some(InterpretationId::from("interp.career.path.kr"))
    );
    assert_eq!(demo.selected.judgement.action, JudgementAction::Accept);
    assert_eq!(demo.promotion.target_status, MeaningStatus::Accepted);
    assert!(demo.answer.contains("requirements=law-school, bar-exam"));
    assert!(demo
      .answer
      .contains("fact.obs.001|fact.schema.001|fact.schema.002"));
  }

  #[test]
  fn sentence_demo_supports_english_surface() {
    let demo = run_sentence_judgement_demo("I want to be a lawyer", "Career.US")
      .expect("english demo result");

    assert_eq!(demo.promotion.target_status, MeaningStatus::Accepted);
    assert!(demo
      .answer
      .contains("requirements=jd-program, state-bar-exam"));
    assert!(demo.answer.contains("jurisdiction=Career.US"));
  }

  #[test]
  fn sentence_demo_rejects_unknown_utterance() {
    assert!(run_sentence_judgement_demo("I want coffee", "Career.KR").is_none());
  }

  #[test]
  fn sentence_demo_exports_semantic_records_episode_and_knowledge() {
    let demo =
      run_sentence_judgement_demo("나 변호사 하고 싶어", "Career.KR").expect("demo result");

    let records = demo.semantic_records();
    let episode = demo.semantic_episode();
    let knowledge = demo.knowledge_record();

    assert!(records
      .iter()
      .any(|record| matches!(record.value, SemanticRecordValue::Judgement(_))));
    assert_eq!(
      episode.chosen_interpretation,
      Some(InterpretationId::from("interp.career.path.kr"))
    );
    assert_eq!(knowledge.target_status, MeaningStatus::Accepted);
    assert!(knowledge
      .source_record_refs
      .iter()
      .any(|record_id| record_id.0 == "record.promotion.000"));
  }

  #[test]
  fn sentence_demo_exports_semantic_ingest_envelope() {
    let demo =
      run_sentence_judgement_demo("나 변호사 하고 싶어", "Career.KR").expect("demo result");

    let envelope = demo.semantic_ingest_envelope();

    assert_eq!(envelope.records.len(), demo.semantic_records().len());
    assert_eq!(envelope.knowledge_records.len(), 1);
    assert_eq!(
      envelope.episode.chosen_interpretation,
      Some(InterpretationId::from("interp.career.path.kr"))
    );
  }

  #[test]
  fn korean_tool_dialogue_demo_exports_semantic_ingest() {
    let demo = run_korean_tool_dialogue_demo("블렌더에서 큐브 생성해").expect("tool dialogue demo");

    assert_eq!(demo.selected.judgement.action, JudgementAction::Accept);
    assert_eq!(demo.promotion.target_status, MeaningStatus::Accepted);
    assert_eq!(demo.capability.tool_name, "blender");
    assert_eq!(demo.plan.action_name, "mesh.create-cube");
    assert_eq!(demo.result.status, ToolExecutionStatus::Succeeded);

    let envelope = demo.semantic_ingest_envelope();
    assert!(envelope
      .records
      .iter()
      .any(|record| matches!(record.value, SemanticRecordValue::ToolCapability(_))));
    assert!(envelope
      .records
      .iter()
      .any(|record| matches!(record.value, SemanticRecordValue::ToolActionPlan(_))));
    assert!(envelope
      .records
      .iter()
      .any(|record| matches!(record.value, SemanticRecordValue::ToolExecutionResult(_))));
    assert!(envelope.records.iter().any(|record| {
      matches!(
        &record.value,
        SemanticRecordValue::ContextualFact(fact)
          if fact.pred == "language" && fact.obj == "ko"
      )
    }));
    assert!(envelope.records.iter().any(|record| {
      matches!(
        &record.value,
        SemanticRecordValue::ContextualFact(fact)
          if fact.pred == "sentence-mood" && fact.obj == "imperative"
      )
    }));
    assert!(envelope
      .episode
      .summary
      .as_deref()
      .unwrap_or_default()
      .contains("blender"));
  }

  #[test]
  fn korean_tool_dialogue_demo_supports_rhino_surface() {
    let demo = run_korean_tool_dialogue_demo("라이노에서 선 그어").expect("rhino dialogue demo");

    assert_eq!(demo.capability.tool_name, "rhino");
    assert_eq!(demo.plan.action_name, "command.run");
    assert_eq!(
      demo.plan.args.get("command").map(String::as_str),
      Some("Line")
    );
  }

  #[test]
  fn korean_tool_dialogue_demo_supports_runtime_zebra_surface() {
    let demo = run_korean_tool_dialogue_demo("아인슈타인 퍼즐에서 물고기를 기르는 사람은 누구야?")
      .expect("zebra runtime dialogue demo");

    assert_eq!(demo.capability.tool_name, "ontology-runtime");
    assert_eq!(demo.capability.adapter_runtime, "doghouse");
    assert_eq!(demo.capability.transport, ToolTransportKind::Embedded);
    assert_eq!(demo.plan.action_name, "logic.solve-zebra");
    assert_eq!(
      demo
        .result
        .semantic_facts
        .iter()
        .find(|fact| fact.pred == "fish-owner")
        .map(|fact| fact.obj.as_str()),
      Some("독일인")
    );
    assert!(demo
      .facts
      .iter()
      .any(|fact| { fact.pred == "sentence-mood" && fact.obj == "interrogative" }));
    assert!(demo
      .facts
      .iter()
      .any(|fact| fact.pred == "has-particle" && fact.obj == "object"));
    assert!(demo.answer.contains("독일인"));
  }

  #[test]
  fn korean_tool_dialogue_demo_supports_math_runtime_surface() {
    let demo = run_korean_tool_dialogue_demo("2 더하기 2는 뭐야?").expect("math runtime dialogue");

    assert_eq!(demo.capability.tool_name, "ontology-runtime");
    assert_eq!(demo.plan.action_name, "math.solve-arithmetic");
    assert!(demo
      .facts
      .iter()
      .any(|fact| { fact.pred == "sentence-mood" && fact.obj == "interrogative" }));
    assert_eq!(
      demo
        .result
        .semantic_facts
        .iter()
        .find(|fact| fact.pred == "result")
        .map(|fact| fact.obj.as_str()),
      Some("4")
    );
    assert!(demo.answer.contains("2+2 = 4"));
  }

  #[test]
  fn korean_tool_dialogue_demo_supports_physics_runtime_surface() {
    let demo = run_korean_tool_dialogue_demo("질량 2kg이고 가속도 3m/s^2이면 힘은 얼마야?")
      .expect("physics runtime dialogue");

    assert_eq!(demo.capability.tool_name, "ontology-runtime");
    assert_eq!(demo.plan.action_name, "physics.compute-force");
    assert_eq!(
      demo
        .result
        .semantic_facts
        .iter()
        .find(|fact| fact.pred == "force-n")
        .map(|fact| fact.obj.as_str()),
      Some("6")
    );
    assert!(demo.answer.contains("6N"));
  }

  #[test]
  fn korean_tool_dialogue_demo_holds_general_light_question_until_context_is_specified() {
    let demo = run_korean_tool_dialogue_demo("빛은 뭐야?").expect("general light dialogue");

    assert_eq!(demo.capability.tool_name, "ontology-runtime");
    assert_eq!(demo.plan.action_name, "physics.interpret-light-general");
    assert_eq!(demo.selected.judgement.action, JudgementAction::Hold);
    assert_eq!(demo.promotion.target_status, MeaningStatus::Held);
    assert_eq!(demo.result.status, ToolExecutionStatus::Held);
    assert!(demo
      .facts
      .iter()
      .any(|fact| fact.pred == "requires-context" && fact.obj == "experimental-setup"));
    assert!(demo.answer.contains("수렴하지 않는다"));
  }

  #[test]
  fn korean_tool_dialogue_demo_reinterprets_light_in_double_slit_context() {
    let demo = run_korean_tool_dialogue_demo("이중슬릿 실험에서 빛은 뭐야?")
      .expect("double slit light dialogue");

    assert_eq!(demo.plan.action_name, "physics.interpret-light");
    assert_eq!(demo.selected.judgement.action, JudgementAction::Accept);
    assert_eq!(demo.result.status, ToolExecutionStatus::Succeeded);
    assert_eq!(
      demo
        .result
        .semantic_facts
        .iter()
        .find(|fact| fact.pred == "manifests-as")
        .map(|fact| fact.obj.as_str()),
      Some("wave-like-interference")
    );
    assert!(demo.answer.contains("이중슬릿 실험"));
  }

  #[test]
  fn korean_tool_dialogue_demo_reinterprets_light_in_photoelectric_context() {
    let demo = run_korean_tool_dialogue_demo("광전효과에서 빛은 뭐야?")
      .expect("photoelectric light dialogue");

    assert_eq!(demo.plan.action_name, "physics.interpret-light");
    assert_eq!(demo.selected.judgement.action, JudgementAction::Accept);
    assert_eq!(
      demo
        .result
        .semantic_facts
        .iter()
        .find(|fact| fact.pred == "manifests-as")
        .map(|fact| fact.obj.as_str()),
      Some("particle-like-quanta")
    );
    assert!(demo.answer.contains("광전효과"));
  }

  #[test]
  fn korean_tool_dialogue_demo_math_runtime_exports_expression_projection_facts() {
    let demo = run_korean_tool_dialogue_demo("2 더하기 2는 뭐야?").expect("math runtime dialogue");

    assert!(demo
      .result
      .semantic_facts
      .iter()
      .any(|fact| fact.pred == "projection-surface" && fact.obj == "math-expression"));
    assert!(demo
      .result
      .semantic_facts
      .iter()
      .any(|fact| fact.pred == "expression-normal-form" && fact.obj == "2 + 2 = 4"));
    assert!(demo
      .result
      .artifact_refs
      .iter()
      .any(|artifact| artifact == "artifact://doghouse/expression/arithmetic-2-plus-2"));
    assert_eq!(demo.result.expression_projections.len(), 1);
    assert_eq!(
      demo.result.expression_projections[0].projection_family,
      "expmath"
    );
    assert_eq!(
      demo.result.expression_projections[0]
        .surface_forms
        .get("openmath")
        .map(String::as_str),
      Some("<OMOBJ><OMA><OMS cd=\"relation1\" name=\"eq\"/><OMA><OMS cd=\"arith1\" name=\"plus\"/><OMI>2</OMI><OMI>2</OMI></OMA><OMI>4</OMI></OMA></OMOBJ>")
    );
  }

  #[test]
  fn korean_tool_dialogue_demo_physics_runtime_exports_expression_projection_facts() {
    let demo = run_korean_tool_dialogue_demo("질량 2kg이고 가속도 3m/s^2이면 힘은 얼마야?")
      .expect("physics runtime dialogue");

    assert!(demo
      .result
      .semantic_facts
      .iter()
      .any(|fact| fact.pred == "projection-surface" && fact.obj == "math-expression"));
    assert!(demo
      .result
      .semantic_facts
      .iter()
      .any(|fact| fact.pred == "expression-normal-form" && fact.obj == "F = m * a = 6 N"));
    assert!(demo
      .result
      .artifact_refs
      .iter()
      .any(|artifact| artifact == "artifact://doghouse/expression/newton-force"));
    assert_eq!(demo.result.expression_projections.len(), 1);
    assert_eq!(
      demo.result.expression_projections[0].projection_family,
      "expmath"
    );
    assert!(demo.result.expression_projections[0]
      .surface_forms
      .contains_key("freecat-geometry"));
  }

  #[test]
  fn korean_tool_dialogue_followup_demo_reopens_held_light_question() {
    let demo = run_korean_tool_dialogue_followup_demo("빛은 뭐야?", "이중슬릿 실험에서 빛은 뭐야?")
      .expect("light follow-up demo");

    assert_eq!(
      demo.initial.selected.judgement.action,
      JudgementAction::Hold
    );
    assert_eq!(
      demo.follow_up.selected.judgement.action,
      JudgementAction::Accept
    );
    assert!(demo
      .follow_up
      .facts
      .iter()
      .any(|fact| fact.pred == "reopens-judgement"));
    assert!(demo
      .follow_up
      .answer
      .contains("follow-up reopen: runtime ontology 계산 결과"));
    assert_eq!(demo.transcript_lines().len(), 4);
    assert_eq!(
      demo
        .semantic_ingest_envelope()
        .notes
        .iter()
        .filter(|note| note.starts_with("transcript:"))
        .count(),
      4
    );
  }

  #[test]
  fn korean_tool_dialogue_demo_rejects_unknown_utterance() {
    assert!(run_korean_tool_dialogue_demo("오늘 날씨 어때").is_none());
  }
}
fn demo_policy() -> EvaluationPolicy {
  EvaluationPolicy {
    id: "ontology.demo.career".to_string(),
    weights: EvaluationWeights {
      coherence: 1.0,
      coverage: 2.0,
      loss: 0.4,
      cost: 0.2,
      replayability: 1.2,
      safety: 1.2,
    },
    ..EvaluationPolicy::default()
  }
}
