//! Algorithm-sentence-sequence artifact builder.
//!
//! OWNER-LAW (2026-05-12): mirror of
//! `stdlib/lib/gate/algorithm-synthesis/algorithm-sentence-sequence.px`.
//! Builds the typed receipt artifact that captures one full
//! synthesis chain pass: temporal shape (from time:* cues) +
//! ordered steps (from operation candidate + resolution verdict) +
//! provenance.
//!
//! This is not a classifier — it's a deterministic projection over
//! the upstream verdicts. Its job is to:
//!
//!   1. Pick a *temporal shape* for the generated code based on
//!      which time:* cue won upstream
//!   2. Render the synthesis decisions as an ordered list of
//!      semantic steps (each step has an NL description + a step
//!      kind + provenance)
//!   3. Surface Held outcomes as a final `hold` step rather than
//!      pretending the chain closed
//!
//! The result is meant to be stored in doghouse under the
//! `coding.algorithm-sentence-sequence-{ready,held}` family
//! (wrapper lives in `doghouse-core::code_transform_artifact`).

use serde::{Deserialize, Serialize};
use pnix_hash::{Digest, Sha256};

use crate::algorithm_synthesis::operation_candidate_mapping::OperationCandidate;
use crate::algorithm_synthesis::parameter_resolution::ResolutionVerdict;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TemporalShape {
  /// Explicit step-by-step request — `time:process-step` fired.
  Ordered,
  /// Conditional branch request — `time:conditional` fired.
  Conditional,
  /// Diagnose-then-fix request — `time:causal` fired.
  CausalRepair,
  /// Default — single direct action, no temporal cue.
  ImmediateEdit,
}

impl TemporalShape {
  pub const ALL: &'static [Self] = &[
    Self::Ordered,
    Self::Conditional,
    Self::CausalRepair,
    Self::ImmediateEdit,
  ];

  pub fn as_str(self) -> &'static str {
    match self {
      Self::Ordered => "ordered",
      Self::Conditional => "conditional",
      Self::CausalRepair => "causal-repair",
      Self::ImmediateEdit => "immediate-edit",
    }
  }
}

/// Each step in the sequence — one *semantic sentence* with provenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StepKind {
  Inspect,
  Resolve,
  Propose,
  Hold,
}

impl StepKind {
  pub const ALL: &'static [Self] = &[Self::Inspect, Self::Resolve, Self::Propose, Self::Hold];

  pub fn as_str(self) -> &'static str {
    match self {
      Self::Inspect => "inspect",
      Self::Resolve => "resolve",
      Self::Propose => "propose",
      Self::Hold => "hold",
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlgorithmStep {
  pub index: u32,
  pub description: String,
  pub kind: StepKind,
  pub transform: Option<String>,
  pub signals: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlgorithmSentenceSequence {
  pub temporal_shape: TemporalShape,
  pub source_intent: String,
  pub source_transform: String,
  pub steps: Vec<AlgorithmStep>,
  /// True when the chain closed at a `Propose` step (resolution
  /// Ready). False when the last step is `Hold`.
  pub ready: bool,
  /// Cues that contributed to this sequence — all fired_signals
  /// passed through the chain.
  pub provenance_signals: Vec<String>,
}

/// Decide the temporal shape from which time:* cue won upstream.
/// Priority: ordered > causal-repair > conditional > immediate-edit.
pub fn derive_temporal_shape(fired_signals: &[String]) -> TemporalShape {
  let has = |needle: &str| fired_signals.iter().any(|s| s == needle);
  if has("time:process-step") {
    TemporalShape::Ordered
  } else if has("time:causal") {
    TemporalShape::CausalRepair
  } else if has("time:conditional") {
    TemporalShape::Conditional
  } else {
    TemporalShape::ImmediateEdit
  }
}

/// Build the algorithm-sentence-sequence from the synthesis chain's
/// outputs. Always succeeds — incomplete chains produce a final
/// `Hold` step instead of erroring.
pub fn build_algorithm_sentence_sequence(
  operation: &OperationCandidate,
  resolution: &ResolutionVerdict,
  fired_signals: &[String],
) -> AlgorithmSentenceSequence {
  let shape = derive_temporal_shape(fired_signals);
  let mut steps: Vec<AlgorithmStep> = Vec::new();

  // Step 1: inspect — every chain begins by naming the target.
  let inspect_desc = match shape {
    TemporalShape::Ordered => format!(
      "Inspect the target code for `{}` (ordered: step-by-step proceed)",
      operation.transform
    ),
    TemporalShape::Conditional => format!(
      "Inspect the target code for `{}` (conditional: branch on supplied condition)",
      operation.transform
    ),
    TemporalShape::CausalRepair => format!(
      "Inspect the target code for `{}` (causal-repair: diagnose root cause first)",
      operation.transform
    ),
    TemporalShape::ImmediateEdit => format!(
      "Inspect the target code for `{}` (immediate edit)",
      operation.transform
    ),
  };
  steps.push(AlgorithmStep {
    index: 0,
    description: inspect_desc,
    kind: StepKind::Inspect,
    transform: Some(operation.transform.clone()),
    signals: fired_signals.to_vec(),
  });

  // Steps 2+: based on the resolution verdict.
  let mut ready = false;
  match resolution {
    ResolutionVerdict::ResolutionReady {
      transform,
      resolved_fields,
      ..
    } => {
      // Resolve step — name each field that got resolved.
      let resolved_summary = if resolved_fields.is_empty() {
        "(no fields)".to_string()
      } else {
        resolved_fields
          .iter()
          .map(|(k, v)| format!("{k}={v}"))
          .collect::<Vec<_>>()
          .join(", ")
      };
      steps.push(AlgorithmStep {
        index: 1,
        description: format!("Resolve `{transform}` request fields: {resolved_summary}"),
        kind: StepKind::Resolve,
        transform: Some(transform.clone()),
        signals: vec![],
      });
      // Propose step — final closure.
      steps.push(AlgorithmStep {
        index: 2,
        description: format!(
          "Propose typed `{transform}` request for downstream `classify_{}` carrier",
          transform.replace('-', "_")
        ),
        kind: StepKind::Propose,
        transform: Some(transform.clone()),
        signals: vec![],
      });
      ready = true;
    }
    ResolutionVerdict::ResolutionHeld {
      transform,
      held_kind,
      missing_slots,
      partial_resolution,
      reason,
    } => {
      // Surface partial resolution as a Resolve step (what DID
      // close), then a Hold step (what's missing).
      if !partial_resolution.is_empty() {
        let partial_summary = partial_resolution
          .iter()
          .map(|(k, v)| format!("{k}={v}"))
          .collect::<Vec<_>>()
          .join(", ");
        steps.push(AlgorithmStep {
          index: 1,
          description: format!("Partial resolve for `{transform}`: {partial_summary}"),
          kind: StepKind::Resolve,
          transform: Some(transform.clone()),
          signals: vec![],
        });
      }
      let missing_summary = if missing_slots.is_empty() {
        "(no slots — held on other grounds)".to_string()
      } else {
        missing_slots.join(", ")
      };
      steps.push(AlgorithmStep {
        index: steps.len() as u32,
        description: format!(
          "Hold `{transform}` at `{}`: missing [{missing_summary}] — {reason}",
          held_kind.as_str()
        ),
        kind: StepKind::Hold,
        transform: Some(transform.clone()),
        signals: vec![],
      });
    }
    ResolutionVerdict::ResolutionRejected {
      transform,
      held_kind,
      reason,
    } => {
      // Rejected = also surfaces as Hold step but with hard-rejection
      // semantics in the kind string. We could add a separate
      // `Reject` StepKind later; for now Hold captures "do not
      // proceed".
      steps.push(AlgorithmStep {
        index: steps.len() as u32,
        description: format!("Reject `{transform}` at `{}`: {reason}", held_kind.as_str()),
        kind: StepKind::Hold,
        transform: Some(transform.clone()),
        signals: vec![],
      });
    }
  }

  AlgorithmSentenceSequence {
    temporal_shape: shape,
    source_intent: operation.source_intent.clone(),
    source_transform: operation.transform.clone(),
    steps,
    ready,
    provenance_signals: fired_signals.to_vec(),
  }
}

/// Wrap an `AlgorithmSentenceSequence` into the canonical artifact
/// envelope (id + family + source_surface + target_paths +
/// command_refs + related_refs + payload + repo_snapshot_ref). The
/// id is replay-stable SHA-256 of intrinsic identity:
///
///   1. temporal_shape
///   2. source_intent
///   3. source_transform
///   4. ready flag
///   5. each step's (index, kind, transform-option, description)
///   6. provenance_signals (sorted for determinism)
///
/// `stored_at_ms` and `repo_snapshot_ref` are extrinsic; they don't
/// participate in the hash.
pub fn build_sequence_artifact(
  seq: &AlgorithmSentenceSequence,
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
) -> serde_json::Value {
  let payload = build_sequence_payload(seq);

  let mut hasher = Sha256::new();
  hasher.update(b"algorithm-sentence-sequence\x1f");
  hasher.update(seq.temporal_shape.as_str().as_bytes());
  hasher.update(b"\x1f");
  hasher.update(seq.source_intent.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(seq.source_transform.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(&[seq.ready as u8]);
  hasher.update(b"\x1f");
  for s in &seq.steps {
    hasher.update(s.index.to_le_bytes());
    hasher.update(b"\x1e");
    hasher.update(s.kind.as_str().as_bytes());
    hasher.update(b"\x1e");
    hasher.update(s.transform.as_deref().unwrap_or("").as_bytes());
    hasher.update(b"\x1e");
    hasher.update(s.description.as_bytes());
    hasher.update(b"\x1d");
  }
  // Sort provenance signals so the same input set yields the same
  // hash regardless of insertion order upstream.
  let mut sorted_sig = seq.provenance_signals.clone();
  sorted_sig.sort();
  for sig in &sorted_sig {
    hasher.update(sig.as_bytes());
    hasher.update(b"\x1c");
  }
  let digest = hasher.finalize();
  // 16-byte (128-bit) prefix → 32 hex chars. Long enough for
  // collision-free audit ids in long-lived doghouse storage; matches
  // the family-wide convention. Extending past 16 bytes is unlikely
  // to add value but is also free.
  let prefix = digest
    .iter()
    .take(16)
    .map(|b| format!("{b:02x}"))
    .collect::<String>();
  let id = format!("algorithm-sentence-sequence.{prefix}");

  let artifact_family = if seq.ready {
    "coding.algorithm-sentence-sequence-ready"
  } else {
    "coding.algorithm-sentence-sequence-held"
  };

  let mut artifact = serde_json::json!({
    "id": id,
    "artifact_family": artifact_family,
    "source_surface": "algorithm-synthesis.algorithm-sentence-sequence",
    "stored_at_ms": stored_at_ms,
    "target_paths": Vec::<String>::new(),  // synthesis receipt is not per-file
    "command_refs": Vec::<String>::new(),
    "related_refs": serde_json::json!([
      "owner-law:stdlib/lib/gate/algorithm-synthesis/algorithm-sentence-sequence.px"
    ]),
    "payload": payload,
  });
  if let Some(snap) = repo_snapshot_ref {
    artifact["repo_snapshot_ref"] = serde_json::Value::String(snap.to_string());
  }
  artifact
}

/// Render the sequence as a canonical JSON value suitable for
/// doghouse storage under
/// `coding.algorithm-sentence-sequence-{ready,held}`.
pub fn build_sequence_payload(seq: &AlgorithmSentenceSequence) -> serde_json::Value {
  serde_json::json!({
    "transform": "algorithm-synthesis.algorithm-sentence-sequence",
    "owner_law": "stdlib/lib/gate/algorithm-synthesis/algorithm-sentence-sequence.px",
    "temporal_shape": seq.temporal_shape.as_str(),
    "source_intent": seq.source_intent,
    "source_transform": seq.source_transform,
    "ready": seq.ready,
    "steps": seq.steps.iter().map(|s| serde_json::json!({
      "index": s.index,
      "description": s.description,
      "kind": s.kind.as_str(),
      "transform": s.transform,
      "signals": s.signals,
    })).collect::<Vec<_>>(),
    "provenance_signals": seq.provenance_signals,
    "artifact_family": if seq.ready {
      "coding.algorithm-sentence-sequence-ready"
    } else {
      "coding.algorithm-sentence-sequence-held"
    },
    "candidate_only": true,
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::algorithm_synthesis::parameter_resolution::ResolutionHeldKind;
  use std::collections::BTreeMap;

  fn op_candidate(transform: &str, intent: &str) -> OperationCandidate {
    OperationCandidate {
      transform: transform.to_string(),
      confidence: 0.95,
      firing_cue_sets: vec![vec!["verb:rename".to_string()]],
      source_intent: intent.to_string(),
    }
  }

  fn ready_resolution(transform: &str) -> ResolutionVerdict {
    let mut resolved = BTreeMap::new();
    resolved.insert("old_name".to_string(), "foo".to_string());
    resolved.insert("new_name".to_string(), "bar".to_string());
    resolved.insert("target_paths".to_string(), "src/a.rs".to_string());
    resolved.insert("language".to_string(), "rust".to_string());
    resolved.insert("scope".to_string(), "local-target-paths".to_string());
    ResolutionVerdict::ResolutionReady {
      transform: transform.to_string(),
      request: serde_json::json!({
        "old_name": "foo",
        "new_name": "bar",
        "target_paths": ["src/a.rs"],
        "language": "rust",
        "scope": "local-target-paths",
      }),
      resolved_fields: resolved,
    }
  }

  fn held_resolution(transform: &str) -> ResolutionVerdict {
    let mut partial = BTreeMap::new();
    partial.insert("language".to_string(), "rust".to_string());
    partial.insert("scope".to_string(), "local-target-paths".to_string());
    ResolutionVerdict::ResolutionHeld {
      transform: transform.to_string(),
      held_kind: ResolutionHeldKind::MissingTargetPath,
      missing_slots: vec!["target_path".to_string()],
      partial_resolution: partial,
      reason: "no file path".to_string(),
    }
  }

  #[test]
  fn temporal_shape_ordered_when_process_step_fires() {
    let s = derive_temporal_shape(&["time:process-step".to_string()]);
    assert_eq!(s, TemporalShape::Ordered);
  }

  #[test]
  fn temporal_shape_causal_repair_when_causal_fires() {
    let s = derive_temporal_shape(&["time:causal".to_string(), "verb:fix".to_string()]);
    assert_eq!(s, TemporalShape::CausalRepair);
  }

  #[test]
  fn temporal_shape_conditional_when_conditional_fires() {
    let s = derive_temporal_shape(&["time:conditional".to_string()]);
    assert_eq!(s, TemporalShape::Conditional);
  }

  #[test]
  fn temporal_shape_immediate_when_no_time_cue() {
    let s = derive_temporal_shape(&["verb:rename".to_string()]);
    assert_eq!(s, TemporalShape::ImmediateEdit);
  }

  #[test]
  fn priority_order_ordered_beats_causal() {
    // Both fire — ordered wins.
    let s = derive_temporal_shape(&["time:causal".to_string(), "time:process-step".to_string()]);
    assert_eq!(s, TemporalShape::Ordered);
  }

  #[test]
  fn ready_sequence_has_three_steps() {
    let op = op_candidate("rename-symbol", "refactor");
    let res = ready_resolution("rename-symbol");
    let fired = vec!["verb:rename".to_string(), "time:imperative".to_string()];
    let seq = build_algorithm_sentence_sequence(&op, &res, &fired);
    assert!(seq.ready);
    assert_eq!(seq.steps.len(), 3);
    assert_eq!(seq.steps[0].kind, StepKind::Inspect);
    assert_eq!(seq.steps[1].kind, StepKind::Resolve);
    assert_eq!(seq.steps[2].kind, StepKind::Propose);
    assert_eq!(seq.temporal_shape, TemporalShape::ImmediateEdit);
  }

  #[test]
  fn held_sequence_terminates_in_hold_step() {
    let op = op_candidate("rename-symbol", "refactor");
    let res = held_resolution("rename-symbol");
    let fired = vec!["verb:rename".to_string(), "time:imperative".to_string()];
    let seq = build_algorithm_sentence_sequence(&op, &res, &fired);
    assert!(!seq.ready);
    // Inspect + partial Resolve + Hold = 3 steps.
    assert_eq!(seq.steps.len(), 3);
    assert_eq!(seq.steps.last().unwrap().kind, StepKind::Hold);
    assert!(seq
      .steps
      .last()
      .unwrap()
      .description
      .contains("missing-target-path"));
  }

  #[test]
  fn ordered_shape_propagates_to_inspect_description() {
    let op = op_candidate("rename-symbol", "refactor");
    let res = ready_resolution("rename-symbol");
    let fired = vec!["verb:rename".to_string(), "time:process-step".to_string()];
    let seq = build_algorithm_sentence_sequence(&op, &res, &fired);
    assert_eq!(seq.temporal_shape, TemporalShape::Ordered);
    assert!(seq.steps[0].description.contains("ordered"));
  }

  #[test]
  fn payload_serialization_carries_canonical_fields() {
    let op = op_candidate("rename-symbol", "refactor");
    let res = ready_resolution("rename-symbol");
    let fired = vec!["verb:rename".to_string()];
    let seq = build_algorithm_sentence_sequence(&op, &res, &fired);
    let p = build_sequence_payload(&seq);
    assert_eq!(
      p["transform"].as_str(),
      Some("algorithm-synthesis.algorithm-sentence-sequence")
    );
    assert_eq!(p["temporal_shape"].as_str(), Some("immediate-edit"));
    assert_eq!(p["source_intent"].as_str(), Some("refactor"));
    assert_eq!(p["source_transform"].as_str(), Some("rename-symbol"));
    assert_eq!(p["ready"].as_bool(), Some(true));
    assert_eq!(
      p["artifact_family"].as_str(),
      Some("coding.algorithm-sentence-sequence-ready")
    );
    assert_eq!(p["steps"].as_array().unwrap().len(), 3);
    assert_eq!(p["candidate_only"].as_bool(), Some(true));
  }

  #[test]
  fn held_payload_has_held_artifact_family() {
    let op = op_candidate("rename-symbol", "refactor");
    let res = held_resolution("rename-symbol");
    let fired = vec!["verb:rename".to_string()];
    let seq = build_algorithm_sentence_sequence(&op, &res, &fired);
    let p = build_sequence_payload(&seq);
    assert_eq!(p["ready"].as_bool(), Some(false));
    assert_eq!(
      p["artifact_family"].as_str(),
      Some("coding.algorithm-sentence-sequence-held")
    );
  }

  #[test]
  fn rejected_resolution_surfaces_as_hold_step_with_reject_text() {
    let op = op_candidate("rename-symbol", "refactor");
    let res = ResolutionVerdict::ResolutionRejected {
      transform: "rename-symbol".to_string(),
      held_kind: ResolutionHeldKind::OldEqualsNew,
      reason: "old == new".to_string(),
    };
    let fired = vec!["verb:rename".to_string()];
    let seq = build_algorithm_sentence_sequence(&op, &res, &fired);
    assert!(!seq.ready);
    assert_eq!(seq.steps.last().unwrap().kind, StepKind::Hold);
    assert!(seq.steps.last().unwrap().description.starts_with("Reject"));
  }

  #[test]
  fn temporal_shape_all_slice_has_no_duplicates() {
    let mut seen: Vec<&str> = Vec::new();
    for v in TemporalShape::ALL {
      let s = v.as_str();
      assert!(!seen.contains(&s), "duplicate temporal shape: {s}");
      seen.push(s);
    }
  }

  // ─── artifact builder tests ──────────────────────────────────

  fn ready_seq() -> AlgorithmSentenceSequence {
    let op = op_candidate("rename-symbol", "refactor");
    let res = ready_resolution("rename-symbol");
    let fired = vec!["verb:rename".to_string(), "time:imperative".to_string()];
    build_algorithm_sentence_sequence(&op, &res, &fired)
  }

  #[test]
  fn artifact_envelope_shape_ready() {
    let a = build_sequence_artifact(&ready_seq(), 1700000000000, None);
    assert!(a["id"]
      .as_str()
      .unwrap()
      .starts_with("algorithm-sentence-sequence."));
    assert_eq!(
      a["artifact_family"].as_str(),
      Some("coding.algorithm-sentence-sequence-ready")
    );
    assert_eq!(
      a["source_surface"].as_str(),
      Some("algorithm-synthesis.algorithm-sentence-sequence")
    );
    assert_eq!(a["stored_at_ms"].as_u64(), Some(1700000000000));
    assert!(a["target_paths"].as_array().unwrap().is_empty());
    assert!(a["related_refs"].as_array().unwrap().iter().any(|v| v
      .as_str()
      .unwrap()
      .contains("algorithm-sentence-sequence.px")));
    assert!(a.get("repo_snapshot_ref").is_none());
  }

  #[test]
  fn artifact_id_prefix_is_32_hex_chars() {
    // 16-byte (128-bit) digest prefix → 32 hex chars after the
    // `algorithm-sentence-sequence.` family tag. Audit ids are
    // long-lived in doghouse, so the prefix needs collision room.
    let a = build_sequence_artifact(&ready_seq(), 0, None);
    let id = a["id"].as_str().unwrap();
    let suffix = id
      .strip_prefix("algorithm-sentence-sequence.")
      .expect("id has expected family prefix");
    assert_eq!(
      suffix.len(),
      32,
      "id `{id}` digest prefix must be 32 hex chars"
    );
    assert!(suffix.chars().all(|c| c.is_ascii_hexdigit()));
  }

  #[test]
  fn artifact_id_is_replay_stable_across_stored_at_ms() {
    let s = ready_seq();
    let a1 = build_sequence_artifact(&s, 1000, None);
    let a2 = build_sequence_artifact(&s, 9999999, None);
    assert_eq!(a1["id"], a2["id"]);
  }

  #[test]
  fn artifact_id_differs_when_temporal_shape_differs() {
    let s_now = ready_seq();
    let mut s_ordered = ready_seq();
    s_ordered.temporal_shape = TemporalShape::Ordered;
    let a1 = build_sequence_artifact(&s_now, 0, None);
    let a2 = build_sequence_artifact(&s_ordered, 0, None);
    assert_ne!(a1["id"], a2["id"]);
  }

  #[test]
  fn artifact_id_invariant_to_provenance_signal_order() {
    let mut s_a = ready_seq();
    let mut s_b = ready_seq();
    s_a.provenance_signals = vec!["b".into(), "a".into(), "c".into()];
    s_b.provenance_signals = vec!["c".into(), "a".into(), "b".into()];
    let a1 = build_sequence_artifact(&s_a, 0, None);
    let a2 = build_sequence_artifact(&s_b, 0, None);
    assert_eq!(
      a1["id"], a2["id"],
      "provenance signal order should not affect id"
    );
  }

  #[test]
  fn artifact_carries_repo_snapshot_ref_when_provided() {
    let a = build_sequence_artifact(&ready_seq(), 0, Some("git:abc123"));
    assert_eq!(a["repo_snapshot_ref"].as_str(), Some("git:abc123"));
  }

  #[test]
  fn held_artifact_carries_held_family() {
    let op = op_candidate("rename-symbol", "refactor");
    let res = held_resolution("rename-symbol");
    let fired = vec!["verb:rename".to_string()];
    let seq = build_algorithm_sentence_sequence(&op, &res, &fired);
    let a = build_sequence_artifact(&seq, 0, None);
    assert_eq!(
      a["artifact_family"].as_str(),
      Some("coding.algorithm-sentence-sequence-held")
    );
  }
}
