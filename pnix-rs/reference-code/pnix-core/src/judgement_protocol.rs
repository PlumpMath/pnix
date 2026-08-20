//! Phase 7 P1: Judgement Protocol — pnix↔doghouse 경계의 정본 메시지 타입.
//!
//! 이 모듈은 pnix(판단 주체)와 doghouse(저장/서빙 gateway) 사이의 protocol을 정의한다.
//!
//! 설계 원칙 (convergence.md Phase 7):
//! - pnix가 판단을 내리고, doghouse는 event를 저장/인덱싱한다.
//! - doghouse가 event 의미를 재판단하면 안 된다.
//! - latest JSON은 projection이며 정본이 아니다. 정본은 append-only event log.
//! - 원격 peer result는 Accepted가 아니라 Candidate evidence로 들어온다.
//! - 같은 event log를 replay하면 같은 judgement projection이 나와야 한다.

use crate::ontology::{
  ContextId, ContextualFact, EvaluationVector, InterpretationId, JudgementAction, JudgementRecord,
  LayerId, MeaningId, MeaningStatus, PromotionDecision, SemanticEpisodeId,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// ID types
// ---------------------------------------------------------------------------

macro_rules! protocol_id {
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

protocol_id!(JudgementRequestId);
protocol_id!(JudgementEventId);
protocol_id!(PromotionEventId);
protocol_id!(EvidenceEnvelopeId);

// ---------------------------------------------------------------------------
// EvidenceSource: 증거의 출처 분류
// ---------------------------------------------------------------------------

/// 증거가 어디서 왔는지. doghouse가 저장할 때 출처를 기록하고,
/// pnix가 판단할 때 출처별 신뢰도/가중치를 적용한다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceSource {
  /// 사용자 입력 (utterance, follow-up)
  UserInput,
  /// 로컬 지식 저장소 (redb, concept vocabulary)
  LocalStore,
  /// 외부 검색 (DuckDuckGo, Wikipedia 등)
  WebSearch,
  /// docset API (프로그래밍 언어 reference)
  DocsetApi,
  /// 원격 peer의 응답 — 항상 Candidate로 진입
  RemotePeer {
    peer_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    latency_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    capability: Option<String>,
  },
  /// .px 규칙 평가 결과
  PxEvaluation,
  /// 내부 계산 (arithmetic, physics solver)
  Computation,
  /// 기타 (확장용)
  Other(String),
}

// ---------------------------------------------------------------------------
// EvidenceEnvelope: 판단 전 raw evidence 묶음
// ---------------------------------------------------------------------------

/// 판단 전에 수집된 raw evidence. pnix가 이것을 받아서 판단한다.
/// doghouse는 이것을 저장하고 pnix에 전달하는 역할만.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceEnvelope {
  pub id: EvidenceEnvelopeId,
  /// 이 evidence가 속한 episode (대화 turn)
  pub episode_id: SemanticEpisodeId,
  /// 수집된 fact들 (아직 판단 전 — status는 Candidate)
  #[serde(default)]
  pub facts: Vec<ContextualFact>,
  /// 각 fact의 출처
  #[serde(default)]
  pub sources: Vec<EvidenceSource>,
  /// 수집 시점 timestamp (ISO 8601)
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub collected_at: Option<String>,
  /// 자유 메타데이터 (검색 쿼리, docset lang 등)
  #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
  pub metadata: BTreeMap<String, String>,
}

// ---------------------------------------------------------------------------
// JudgementRequest: pnix에게 판단을 요청하는 메시지
// ---------------------------------------------------------------------------

/// doghouse(또는 다른 caller)가 pnix에게 판단을 요청할 때 보내는 메시지.
/// pnix는 이것을 받아서 JudgementEvent를 생성한다.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JudgementRequest {
  pub id: JudgementRequestId,
  /// 판단할 evidence
  pub evidence: EvidenceEnvelope,
  /// 원래 utterance (한국어 등)
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub utterance: Option<String>,
  /// 판단 제약 (allowed capability, context scope 등)
  #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
  pub constraints: BTreeMap<String, String>,
  /// request sequence (accept 시점 monotonic)
  #[serde(default)]
  pub seq: u64,
  /// 요청 시점 timestamp
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub requested_at: Option<String>,
}

// ---------------------------------------------------------------------------
// JudgementEvent: pnix가 내린 판단 결과 (append-only log의 단위)
// ---------------------------------------------------------------------------

/// pnix가 판단을 완료한 뒤 생성하는 event.
/// doghouse는 이것을 append-only로 저장하고 인덱싱한다.
/// doghouse가 이 event의 의미를 재판단하면 안 된다.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JudgementEvent {
  pub id: JudgementEventId,
  /// 어떤 request에 대한 판단인지
  pub request_id: JudgementRequestId,
  /// request sequence (latest projection ordering용)
  pub seq: u64,
  /// 판단 시점 timestamp
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub judged_at: Option<String>,
  /// 선택된 해석
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub selected: Option<InterpretationId>,
  /// 거부된 해석들
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub rejected: Vec<InterpretationId>,
  /// 보류된 해석들
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub held: Vec<InterpretationId>,
  /// 판단 action (Accept/Reject/Hold/Contradict)
  pub action: JudgementAction,
  /// 6축 evaluation score
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub evaluation: Option<EvaluationVector>,
  /// 기존 JudgementRecord와의 호환 참조
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub judgement_record: Option<JudgementRecord>,
  /// 판단 추적 (어떤 policy, 어떤 evidence가 결정에 영향)
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub trace: Vec<String>,
  /// provenance (evidence 출처, peer-id, route 등)
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub provenance: Vec<String>,
}

// ---------------------------------------------------------------------------
// PromotionEvent: Candidate → Accepted/Rejected/Held 전이 기록
// ---------------------------------------------------------------------------

/// fact의 상태 전이를 기록하는 event.
/// 같은 event log를 replay하면 같은 상태가 되어야 한다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromotionEvent {
  pub id: PromotionEventId,
  /// 어떤 judgement event에서 발생했는지
  pub judgement_event_id: JudgementEventId,
  /// 전이 대상 fact
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub fact_id: Option<MeaningId>,
  /// 전이 전 상태
  pub from_status: MeaningStatus,
  /// 전이 후 상태
  pub to_status: MeaningStatus,
  /// 전이 시점
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub promoted_at: Option<String>,
  /// 기존 PromotionDecision과의 호환
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub promotion_decision: Option<PromotionDecision>,
  /// 이유
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub reason: Option<String>,
}

// ---------------------------------------------------------------------------
// PeerEvidence: P2P 원격 peer 관련 최소 skeleton
// ---------------------------------------------------------------------------

/// P2P peer의 상태를 ContextualFact로 lift하기 위한 evidence.
/// Phase 7 최소 vertical slice: peer heartbeat → fact lift → judgement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerEvidence {
  /// peer 식별자
  pub peer_id: String,
  /// peer가 보유한 capability 도메인 목록
  #[serde(default)]
  pub capabilities: Vec<String>,
  /// 측정된 latency (ms)
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub latency_ms: Option<u64>,
  /// peer 상태
  pub status: PeerStatus,
  /// 관측 시점
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub observed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PeerStatus {
  Available,
  Busy,
  Unreachable,
  Unknown,
}

impl PeerEvidence {
  /// peer heartbeat를 ContextualFact로 lift한다.
  /// 원격 peer 결과는 항상 Candidate로 진입.
  pub fn to_contextual_fact(&self) -> ContextualFact {
    ContextualFact {
      id: Some(MeaningId::from(format!("peer.{}", self.peer_id))),
      context: ContextId::from("p2p"),
      layer: LayerId::from("L3"),
      subj: self.peer_id.clone(),
      pred: "peer-status".to_string(),
      obj: peer_status_name(&self.status).to_string(),
      status: MeaningStatus::Candidate, // 원격 = 항상 Candidate
      confidence: match self.status {
        PeerStatus::Available => 0.8,
        PeerStatus::Busy => 0.6,
        PeerStatus::Unreachable => 0.2,
        PeerStatus::Unknown => 0.1,
      },
      provenance_refs: peer_provenance_refs(self),
      proof_refs: vec![],
      contradiction_refs: vec![],
      loss: None,
      timestamp: self.observed_at.clone(),
    }
  }
}

// ---------------------------------------------------------------------------
// .px attrset 직렬화: protocol event → .px 형식 문자열
// pnix는 .px가 정본 형식. JSON이 아니라 .px attrset으로 기록한다.
// ---------------------------------------------------------------------------

fn px_string(value: &str) -> String {
  let mut out = String::with_capacity(value.len() + 2);
  out.push('"');
  for ch in value.chars() {
    match ch {
      '\\' => out.push_str("\\\\"),
      '"' => out.push_str("\\\""),
      '$' => out.push_str("\\$"),
      '\n' => out.push_str("\\n"),
      '\r' => out.push_str("\\r"),
      '\t' => out.push_str("\\t"),
      '\0' => out.push_str("\\0"),
      ch if ch.is_control() => out.push_str(&format!("\\u{{{:x}}}", ch as u32)),
      ch => out.push(ch),
    }
  }
  out.push('"');
  out
}

fn px_string_list<'a>(items: impl IntoIterator<Item = &'a str>) -> String {
  let encoded = items.into_iter().map(px_string).collect::<Vec<_>>();
  format!("[ {} ]", encoded.join(" "))
}

fn px_f64(value: f64) -> String {
  if value.is_finite() {
    format!("{value:.6}")
  } else {
    "0.000000".to_string()
  }
}

fn judgement_action_name(action: &JudgementAction) -> &'static str {
  match action {
    JudgementAction::Accept => "accept",
    JudgementAction::Reject => "reject",
    JudgementAction::Hold => "hold",
    JudgementAction::Contradict => "contradict",
  }
}

fn meaning_status_name(status: &MeaningStatus) -> &'static str {
  match status {
    MeaningStatus::Candidate => "candidate",
    MeaningStatus::Accepted => "accepted",
    MeaningStatus::Rejected => "rejected",
    MeaningStatus::Contradicted => "contradicted",
    MeaningStatus::Held => "held",
    MeaningStatus::Deprecated => "deprecated",
    MeaningStatus::Deleted => "deleted",
  }
}

fn peer_status_name(status: &PeerStatus) -> &'static str {
  match status {
    PeerStatus::Available => "available",
    PeerStatus::Busy => "busy",
    PeerStatus::Unreachable => "unreachable",
    PeerStatus::Unknown => "unknown",
  }
}

fn peer_provenance_refs(peer: &PeerEvidence) -> Vec<String> {
  let mut refs = vec![format!("source:p2p-heartbeat:{}", peer.peer_id)];
  if let Some(latency_ms) = peer.latency_ms {
    refs.push(format!("latency-ms:{latency_ms}"));
  }
  refs.extend(
    peer
      .capabilities
      .iter()
      .map(|capability| format!("capability:{capability}")),
  );
  refs
}

impl JudgementEvent {
  /// .px attrset 형식으로 직렬화한다.
  pub fn to_px(&self) -> String {
    let mut lines = Vec::new();
    lines.push("{".to_string());
    lines.push(format!("  type = {};", px_string("judgement-event")));
    lines.push(format!("  id = {};", px_string(&self.id.0)));
    lines.push(format!("  request-id = {};", px_string(&self.request_id.0)));
    lines.push(format!("  seq = {};", self.seq));
    if let Some(ref t) = self.judged_at {
      lines.push(format!("  judged-at = {};", px_string(t)));
    }
    if let Some(ref s) = self.selected {
      lines.push(format!("  selected = {};", px_string(&s.0)));
    }
    if !self.rejected.is_empty() {
      lines.push(format!(
        "  rejected = {};",
        px_string_list(self.rejected.iter().map(|r| r.0.as_str()))
      ));
    }
    if !self.held.is_empty() {
      lines.push(format!(
        "  held = {};",
        px_string_list(self.held.iter().map(|h| h.0.as_str()))
      ));
    }
    lines.push(format!(
      "  action = {};",
      px_string(judgement_action_name(&self.action))
    ));
    if let Some(ref ev) = self.evaluation {
      lines.push("  evaluation = {".to_string());
      lines.push(format!("    id = {};", px_string(&ev.id)));
      lines.push(format!(
        "    interpretation = {};",
        px_string(&ev.interpretation.0)
      ));
      lines.push(format!("    policy = {};", px_string(&ev.policy)));
      lines.push(format!("    coherence = {};", px_f64(ev.coherence)));
      lines.push(format!("    coverage = {};", px_f64(ev.coverage)));
      lines.push(format!("    loss-penalty = {};", px_f64(ev.loss_penalty)));
      lines.push(format!("    cost = {};", px_f64(ev.cost)));
      lines.push(format!("    replayability = {};", px_f64(ev.replayability)));
      lines.push(format!("    safety = {};", px_f64(ev.safety)));
      lines.push(format!("    score = {};", px_f64(ev.score)));
      lines.push("  };".to_string());
    }
    if let Some(ref record) = self.judgement_record {
      lines.push("  judgement-record = {".to_string());
      lines.push(format!("    id = {};", px_string(&record.id)));
      lines.push(format!(
        "    evaluation = {};",
        px_string(&record.evaluation)
      ));
      lines.push(format!(
        "    action = {};",
        px_string(judgement_action_name(&record.action))
      ));
      if let Some(ref selected) = record.chosen_interpretation {
        lines.push(format!(
          "    chosen-interpretation = {};",
          px_string(&selected.0)
        ));
      }
      if !record.chosen_fact_refs.is_empty() {
        lines.push(format!(
          "    chosen-fact-refs = {};",
          px_string_list(record.chosen_fact_refs.iter().map(String::as_str))
        ));
      }
      if !record.notes.is_empty() {
        lines.push(format!(
          "    notes = {};",
          px_string_list(record.notes.iter().map(String::as_str))
        ));
      }
      lines.push("  };".to_string());
    }
    if !self.trace.is_empty() {
      lines.push(format!(
        "  trace = {};",
        px_string_list(self.trace.iter().map(String::as_str))
      ));
    }
    if !self.provenance.is_empty() {
      lines.push(format!(
        "  provenance = {};",
        px_string_list(self.provenance.iter().map(String::as_str))
      ));
    }
    lines.push("}".to_string());
    lines.join("\n")
  }
}

impl PromotionEvent {
  /// .px attrset 형식으로 직렬화한다.
  pub fn to_px(&self) -> String {
    let mut lines = Vec::new();
    lines.push("{".to_string());
    lines.push(format!("  type = {};", px_string("promotion-event")));
    lines.push(format!("  id = {};", px_string(&self.id.0)));
    lines.push(format!(
      "  judgement-event-id = {};",
      px_string(&self.judgement_event_id.0)
    ));
    if let Some(ref fid) = self.fact_id {
      lines.push(format!("  fact-id = {};", px_string(&fid.0)));
    }
    lines.push(format!(
      "  from-status = {};",
      px_string(meaning_status_name(&self.from_status))
    ));
    lines.push(format!(
      "  to-status = {};",
      px_string(meaning_status_name(&self.to_status))
    ));
    if let Some(ref t) = self.promoted_at {
      lines.push(format!("  promoted-at = {};", px_string(t)));
    }
    if let Some(ref decision) = self.promotion_decision {
      lines.push("  promotion-decision = {".to_string());
      lines.push(format!("    id = {};", px_string(&decision.id)));
      lines.push(format!(
        "    judgement = {};",
        px_string(&decision.judgement)
      ));
      lines.push(format!(
        "    target-status = {};",
        px_string(meaning_status_name(&decision.target_status))
      ));
      if let Some(ref reason) = decision.reason {
        lines.push(format!("    reason = {};", px_string(reason)));
      }
      if !decision.artifact_refs.is_empty() {
        lines.push(format!(
          "    artifact-refs = {};",
          px_string_list(decision.artifact_refs.iter().map(String::as_str))
        ));
      }
      lines.push("  };".to_string());
    }
    if let Some(ref r) = self.reason {
      lines.push(format!("  reason = {};", px_string(r)));
    }
    lines.push("}".to_string());
    lines.join("\n")
  }
}

// ---------------------------------------------------------------------------
// Bridge: 기존 ontology types → protocol events
// doghouse가 현재 ontology_evaluate/select/promote를 직접 호출하는 결과를
// append-only event log의 JudgementEvent/PromotionEvent로 변환한다.
// 판단 로직 이동 전 단계: event 기록만 추가.
// ---------------------------------------------------------------------------

use crate::ontology::SelectionOutcome;

impl JudgementEvent {
  /// 기존 `SelectionOutcome` (ontology_select + ontology_evaluate 결과)을
  /// append-only `JudgementEvent`로 변환한다.
  pub fn from_selection(
    request_id: &str,
    seq: u64,
    selection: &SelectionOutcome,
    trace: Vec<String>,
    provenance: Vec<String>,
  ) -> Self {
    Self::from_selection_with_event_id(
      format!("je.{request_id}.{seq}"),
      request_id,
      seq,
      selection,
      trace,
      provenance,
    )
  }

  /// 기존 `SelectionOutcome`을 명시적 event id로 변환한다.
  /// 한 request 안에 여러 judgement event가 생길 때 caller가 ordinal/suffix를 포함한
  /// append-only id를 부여할 수 있게 한다.
  pub fn from_selection_with_event_id(
    event_id: impl Into<String>,
    request_id: &str,
    seq: u64,
    selection: &SelectionOutcome,
    trace: Vec<String>,
    provenance: Vec<String>,
  ) -> Self {
    let chosen = selection.judgement.chosen_interpretation.clone();
    Self {
      id: JudgementEventId::from(event_id.into()),
      request_id: JudgementRequestId::from(request_id),
      seq,
      judged_at: None,
      selected: if selection.judgement.action == JudgementAction::Accept {
        chosen.clone()
      } else {
        None
      },
      rejected: match selection.judgement.action {
        JudgementAction::Reject | JudgementAction::Contradict => chosen.iter().cloned().collect(),
        _ => vec![],
      },
      held: match selection.judgement.action {
        JudgementAction::Hold => chosen.iter().cloned().collect(),
        _ => vec![],
      },
      action: selection.judgement.action.clone(),
      evaluation: Some(selection.evaluation.clone()),
      judgement_record: Some(selection.judgement.clone()),
      trace,
      provenance,
    }
  }
}

impl PromotionEvent {
  /// 기존 `PromotionDecision` (ontology_promote 결과)을
  /// append-only `PromotionEvent`로 변환한다.
  pub fn from_promotion(
    judgement_event_id: &JudgementEventId,
    promotion: &PromotionDecision,
  ) -> Self {
    // PromotionDecision.target_status가 전이 후 상태
    // 전이 전 상태는 항상 Candidate (judgement pipeline의 입력이 Candidate이므로)
    Self {
      id: PromotionEventId::from(format!("pe.{}.{}", judgement_event_id.0, promotion.id)),
      judgement_event_id: judgement_event_id.clone(),
      fact_id: None, // PromotionDecision은 개별 fact을 지정하지 않음
      from_status: MeaningStatus::Candidate,
      to_status: promotion.target_status.clone(),
      promoted_at: None,
      promotion_decision: Some(promotion.clone()),
      reason: promotion.reason.clone(),
    }
  }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn judgement_event_serde_roundtrip() {
    let event = JudgementEvent {
      id: JudgementEventId::from("je.001"),
      request_id: JudgementRequestId::from("jr.001"),
      seq: 42,
      judged_at: Some("2026-04-10T12:00:00Z".to_string()),
      selected: Some(InterpretationId::from("interp.force-equals-ma")),
      rejected: vec![InterpretationId::from("interp.unknown")],
      held: vec![],
      action: JudgementAction::Accept,
      evaluation: None,
      judgement_record: None,
      trace: vec![
        "policy:default".to_string(),
        "evidence:computation".to_string(),
      ],
      provenance: vec!["source:local-solver".to_string()],
    };
    let json = serde_json::to_string_pretty(&event).unwrap();
    let decoded: JudgementEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(event, decoded);
  }

  #[test]
  fn promotion_event_serde_roundtrip() {
    let event = PromotionEvent {
      id: PromotionEventId::from("pe.001"),
      judgement_event_id: JudgementEventId::from("je.001"),
      fact_id: Some(MeaningId::from("fact.force-result")),
      from_status: MeaningStatus::Candidate,
      to_status: MeaningStatus::Accepted,
      promoted_at: Some("2026-04-10T12:00:01Z".to_string()),
      promotion_decision: None,
      reason: Some("solver result verified".to_string()),
    };
    let json = serde_json::to_string_pretty(&event).unwrap();
    let decoded: PromotionEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(event, decoded);
  }

  #[test]
  fn evidence_envelope_serde_roundtrip() {
    let envelope = EvidenceEnvelope {
      id: EvidenceEnvelopeId::from("ev.001"),
      episode_id: SemanticEpisodeId::from("ep.001"),
      facts: vec![ContextualFact {
        id: Some(MeaningId::from("fact.test")),
        context: ContextId::from("test"),
        layer: LayerId::from("L1"),
        subj: "힘".to_string(),
        pred: "definition-ko".to_string(),
        obj: "물체의 운동 상태를 변화시키는 원인".to_string(),
        status: MeaningStatus::Candidate,
        confidence: 0.9,
        provenance_refs: vec!["source:user-input".to_string()],
        proof_refs: vec![],
        contradiction_refs: vec![],
        loss: None,
        timestamp: None,
      }],
      sources: vec![EvidenceSource::UserInput],
      collected_at: Some("2026-04-10T12:00:00Z".to_string()),
      metadata: BTreeMap::new(),
    };
    let json = serde_json::to_string_pretty(&envelope).unwrap();
    let decoded: EvidenceEnvelope = serde_json::from_str(&json).unwrap();
    assert_eq!(envelope, decoded);
  }

  #[test]
  fn judgement_request_serde_roundtrip() {
    let request = JudgementRequest {
      id: JudgementRequestId::from("jr.001"),
      evidence: EvidenceEnvelope {
        id: EvidenceEnvelopeId::from("ev.001"),
        episode_id: SemanticEpisodeId::from("ep.001"),
        facts: vec![],
        sources: vec![],
        collected_at: None,
        metadata: BTreeMap::new(),
      },
      utterance: Some("힘이 뭐야?".to_string()),
      constraints: BTreeMap::new(),
      seq: 1,
      requested_at: None,
    };
    let json = serde_json::to_string_pretty(&request).unwrap();
    let decoded: JudgementRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(request, decoded);
  }

  #[test]
  fn peer_evidence_lifts_to_candidate_fact() {
    let peer = PeerEvidence {
      peer_id: "node-alpha".to_string(),
      capabilities: vec!["physics".to_string(), "math".to_string()],
      latency_ms: Some(12),
      status: PeerStatus::Available,
      observed_at: Some("2026-04-10T12:00:00Z".to_string()),
    };
    let fact = peer.to_contextual_fact();
    assert_eq!(fact.status, MeaningStatus::Candidate); // 원격 = 항상 Candidate
    assert_eq!(fact.context.0, "p2p");
    assert_eq!(fact.layer.0, "L3");
    assert_eq!(fact.subj, "node-alpha");
    assert_eq!(fact.pred, "peer-status");
    assert_eq!(fact.obj, "available");
    assert!(fact.confidence > 0.0);
    assert!(fact.provenance_refs.iter().any(|p| p == "latency-ms:12"));
    assert!(fact
      .provenance_refs
      .iter()
      .any(|p| p == "capability:physics"));
  }

  #[test]
  fn remote_peer_evidence_source_roundtrip() {
    let source = EvidenceSource::RemotePeer {
      peer_id: "node-beta".to_string(),
      latency_ms: Some(45),
      capability: Some("korean-nlp".to_string()),
    };
    let json = serde_json::to_string(&source).unwrap();
    let decoded: EvidenceSource = serde_json::from_str(&json).unwrap();
    assert_eq!(source, decoded);
  }

  #[test]
  fn judgement_event_from_selection_outcome() {
    use crate::ontology::{EvaluationVector, JudgementRecord, SelectionOutcome};
    let selection = SelectionOutcome {
      evaluation: EvaluationVector {
        id: "eval.001".to_string(),
        interpretation: InterpretationId::from("interp.force"),
        policy: "default".to_string(),
        coherence: 0.9,
        coverage: 0.8,
        loss_penalty: 0.0,
        cost: 0.1,
        replayability: 1.0,
        safety: 1.0,
        score: 0.85,
      },
      judgement: JudgementRecord {
        id: "jud.001".to_string(),
        evaluation: "eval.001".to_string(),
        action: JudgementAction::Accept,
        chosen_interpretation: Some(InterpretationId::from("interp.force")),
        chosen_fact_refs: vec!["fact.001".to_string()],
        notes: vec![],
      },
    };
    let event = JudgementEvent::from_selection("req.001", 5, &selection, vec![], vec![]);
    assert_eq!(event.action, JudgementAction::Accept);
    assert_eq!(event.seq, 5);
    assert_eq!(event.selected, Some(InterpretationId::from("interp.force")));
    assert!(event.evaluation.is_some());
    assert!(event.judgement_record.is_some());
  }

  #[test]
  fn judgement_event_from_selection_accepts_explicit_event_id() {
    use crate::ontology::{EvaluationVector, JudgementRecord, SelectionOutcome};
    let selection = SelectionOutcome {
      evaluation: EvaluationVector {
        id: "eval.002".to_string(),
        interpretation: InterpretationId::from("interp.force"),
        policy: "default".to_string(),
        coherence: 0.9,
        coverage: 0.8,
        loss_penalty: 0.0,
        cost: 0.1,
        replayability: 1.0,
        safety: 1.0,
        score: 0.85,
      },
      judgement: JudgementRecord {
        id: "jud.002".to_string(),
        evaluation: "eval.002".to_string(),
        action: JudgementAction::Accept,
        chosen_interpretation: Some(InterpretationId::from("interp.force")),
        chosen_fact_refs: vec![],
        notes: vec![],
      },
    };
    let event = JudgementEvent::from_selection_with_event_id(
      "je.req.002.9.0.route",
      "req.002",
      9,
      &selection,
      vec![],
      vec![],
    );
    assert_eq!(event.id, JudgementEventId::from("je.req.002.9.0.route"));
    assert_eq!(event.request_id, JudgementRequestId::from("req.002"));
    assert_eq!(event.seq, 9);
  }

  #[test]
  fn judgement_event_to_px_format() {
    use crate::ontology::{EvaluationVector, JudgementRecord, SelectionOutcome};
    let selection = SelectionOutcome {
      evaluation: EvaluationVector {
        id: "eval.001".to_string(),
        interpretation: InterpretationId::from("interp.force"),
        policy: "default".to_string(),
        coherence: 0.9,
        coverage: 0.8,
        loss_penalty: 0.0,
        cost: 0.1,
        replayability: 1.0,
        safety: 1.0,
        score: 0.85,
      },
      judgement: JudgementRecord {
        id: "jud.001".to_string(),
        evaluation: "eval.001".to_string(),
        action: JudgementAction::Accept,
        chosen_interpretation: Some(InterpretationId::from("interp.force")),
        chosen_fact_refs: vec![],
        notes: vec![],
      },
    };
    let event = JudgementEvent::from_selection("test", 1, &selection, vec![], vec![]);
    let px = event.to_px();
    assert!(px.contains("type = \"judgement-event\""));
    assert!(px.contains("action = \"accept\""));
    assert!(px.contains("selected = \"interp.force\""));
    assert!(px.contains("evaluation = {"));
    assert!(px.contains("score = 0.850000"));
    assert!(px.contains("judgement-record = {"));
    assert!(px.starts_with('{'));
    assert!(px.ends_with('}'));
    crate::lang::pnix::parse_expr(&px).expect("generated judgement .px should parse");
  }

  #[test]
  fn promotion_event_to_px_format() {
    let decision = PromotionDecision {
      id: "prom.001".to_string(),
      judgement: "jud.001".to_string(),
      target_status: MeaningStatus::Accepted,
      reason: Some("verified".to_string()),
      artifact_refs: vec![],
    };
    let je_id = JudgementEventId::from("je.test.1");
    let event = PromotionEvent::from_promotion(&je_id, &decision);
    let px = event.to_px();
    assert!(px.contains("type = \"promotion-event\""));
    assert!(px.contains("from-status = \"candidate\""));
    assert!(px.contains("to-status = \"accepted\""));
    assert!(px.contains("promotion-decision = {"));
    assert!(px.contains("reason = \"verified\""));
    crate::lang::pnix::parse_expr(&px).expect("generated promotion .px should parse");
  }

  #[test]
  fn generated_px_escapes_strings_and_stays_parseable() {
    let event = JudgementEvent {
      id: JudgementEventId::from("je.quote\"slash\\dollar$"),
      request_id: JudgementRequestId::from("jr.${not_interp}"),
      seq: 7,
      judged_at: None,
      selected: Some(InterpretationId::from("interp.\"quoted\"")),
      rejected: vec![],
      held: vec![],
      action: JudgementAction::Accept,
      evaluation: None,
      judgement_record: None,
      trace: vec!["line1\nline2 with \"quote\" and ${dollar}".to_string()],
      provenance: vec!["source\\path".to_string()],
    };
    let px = event.to_px();
    assert!(px.contains("\\\""));
    assert!(px.contains("\\\\"));
    assert!(px.contains("\\$"));
    crate::lang::pnix::parse_expr(&px).expect("escaped judgement .px should parse");
  }

  #[test]
  fn hold_selection_is_held_not_selected() {
    use crate::ontology::{EvaluationVector, JudgementRecord, SelectionOutcome};
    let selection = SelectionOutcome {
      evaluation: EvaluationVector {
        id: "eval.held".to_string(),
        interpretation: InterpretationId::from("interp.held"),
        policy: "default".to_string(),
        coherence: 0.2,
        coverage: 0.2,
        loss_penalty: 0.0,
        cost: 0.1,
        replayability: 0.1,
        safety: 0.1,
        score: 0.2,
      },
      judgement: JudgementRecord {
        id: "jud.held".to_string(),
        evaluation: "eval.held".to_string(),
        action: JudgementAction::Hold,
        chosen_interpretation: Some(InterpretationId::from("interp.held")),
        chosen_fact_refs: vec![],
        notes: vec![],
      },
    };
    let event = JudgementEvent::from_selection("req.held", 8, &selection, vec![], vec![]);
    assert_eq!(event.selected, None);
    assert_eq!(event.held, vec![InterpretationId::from("interp.held")]);
    assert!(event.rejected.is_empty());
  }

  #[test]
  fn promotion_event_from_promotion_decision() {
    let decision = PromotionDecision {
      id: "prom.001".to_string(),
      judgement: "jud.001".to_string(),
      target_status: MeaningStatus::Accepted,
      reason: Some("solver verified".to_string()),
      artifact_refs: vec![],
    };
    let je_id = JudgementEventId::from("je.req.001.5");
    let event = PromotionEvent::from_promotion(&je_id, &decision);
    assert_eq!(event.from_status, MeaningStatus::Candidate);
    assert_eq!(event.to_status, MeaningStatus::Accepted);
    assert_eq!(event.reason, Some("solver verified".to_string()));
    assert_eq!(event.judgement_event_id, je_id);
  }

  #[test]
  fn promotion_event_id_is_scoped_to_judgement_event_id() {
    let decision = PromotionDecision {
      id: "prom.same".to_string(),
      judgement: "jud.same".to_string(),
      target_status: MeaningStatus::Accepted,
      reason: None,
      artifact_refs: vec![],
    };
    let first = PromotionEvent::from_promotion(&JudgementEventId::from("je.req.1.0"), &decision);
    let second = PromotionEvent::from_promotion(&JudgementEventId::from("je.req.1.1"), &decision);
    assert_ne!(first.id, second.id);
    assert!(first.id.0.contains("je.req.1.0"));
    assert!(second.id.0.contains("je.req.1.1"));
  }

  // 2026-05-06: `px_protocol_spec_fixture_exists` 함수는
  // `crates/pnix-core/tests/judgement_protocol_fixture.rs` 로
  // 이동. inline `#[cfg(test)]` 안에서 fixture 파일을
  // 읽으면 `forbid_runtime_symbols_in_src` 의 text-scan 이
  // src/ 에 forbidden runtime symbol 검출. 의미는 동일하게
  // 유지되며 외부 tests crate 에서 `pnix_core::lang::pnix::parse_expr`
  // 의 pub API 만 호출.
}
