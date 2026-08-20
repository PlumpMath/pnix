//! Retrieval-execution owner — Stage B of the evolution lane.
//!
//! OWNER-LAW (2026-05-12): mirror of
//! `stdlib/lib/gate/algorithm-synthesis/retrieval-execution.px`.
//! Consumes a `HeldRetrievalQuery` + caller-supplied `HostEvidence`
//! and produces a `RetrievalResult` describing what (if anything)
//! the retrieval recovered.
//!
//! v0 closes 3 channels and defers 2:
//!
//!   - **operator-followup**     — pass-through; the cockpit
//!                                 displays the query verbatim.
//!   - **host-symbol-resolver**  — stub adapter; caller injects
//!                                 a HostEvidence map (LSP / lint
//!                                 output) and the dispatcher fills
//!                                 evidence_to_recover slots by
//!                                 lookup.
//!   - **not-recoverable**       — pass-through Rejected.
//!   - **external-knowledge-search** — Deferred (future Stage).
//!   - **ankh-retrieval**        — Deferred (future Stage C).
//!
//! No new heuristic logic — all data is registry-driven (mirror of
//! `.px` tables). New slot → one row in `EVIDENCE_SLOT_TO_HOST_KEY`
//! and one row in `SLOT_TO_RESOLUTION_INPUT_FIELD`; no Rust branch.

use serde::{Deserialize, Serialize};
use pnix_hash::{Digest, Sha256};
use std::collections::BTreeMap;

use super::held_to_query::{HeldQueryRecoveryChannel, HeldRetrievalQuery};

/// Caller-supplied evidence pool. Keys are slot names (e.g.
/// `"import_spec"`); values are the answers the host's lint / LSP /
/// compiler discovered. The dispatcher looks up each slot in
/// `query.evidence_to_recover`.
pub type HostEvidence = BTreeMap<String, String>;

/// Channel implementation status — mirror of `.px`
/// `channelImplementationStatus`. Sync test asserts the per-channel
/// status matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChannelImplementationStatus {
  Implemented,
  Deferred,
}

impl ChannelImplementationStatus {
  pub fn as_str(self) -> &'static str {
    match self {
      Self::Implemented => "implemented",
      Self::Deferred => "deferred",
    }
  }
}

#[derive(Debug, Clone, Copy)]
pub struct ChannelImplementationRow {
  pub channel: HeldQueryRecoveryChannel,
  pub status: ChannelImplementationStatus,
}

/// Per-channel implementation status. Sync test asserts every
/// `HeldQueryRecoveryChannel::ALL` appears in this table.
pub const CHANNEL_IMPLEMENTATION_STATUS: &[ChannelImplementationRow] = &[
  ChannelImplementationRow {
    channel: HeldQueryRecoveryChannel::OperatorFollowup,
    status: ChannelImplementationStatus::Implemented,
  },
  ChannelImplementationRow {
    channel: HeldQueryRecoveryChannel::HostSymbolResolver,
    status: ChannelImplementationStatus::Implemented,
  },
  ChannelImplementationRow {
    channel: HeldQueryRecoveryChannel::ExternalKnowledgeSearch,
    status: ChannelImplementationStatus::Deferred,
  },
  ChannelImplementationRow {
    channel: HeldQueryRecoveryChannel::AnkhRetrieval,
    status: ChannelImplementationStatus::Deferred,
  },
  ChannelImplementationRow {
    channel: HeldQueryRecoveryChannel::NotRecoverable,
    status: ChannelImplementationStatus::Implemented,
  },
];

/// Slot-name → HostEvidence-key mapping. Currently a 1:1 identity
/// table; the indirection exists so future slot renames don't break
/// existing HostEvidence producers.
pub const EVIDENCE_SLOT_TO_HOST_KEY: &[(&str, &str)] = &[
  ("import_spec", "import_spec"),
  ("candidate_imports", "candidate_imports"),
  ("target_path", "target_path"),
  ("language", "language"),
  ("old_name", "old_name"),
  ("new_name", "new_name"),
  ("test_name", "test_name"),
  // Math-domain slot. The host (CAS adapter or operator UI) supplies
  // `equivalent_form` as the recovered evidence; ankh stores it under
  // the supplied_parameters key derived via SLOT_TO_RESOLUTION_INPUT_FIELD.
  ("equivalent_form", "equivalent_form"),
  // Chemistry-domain slot — substrate-sharing N=3.
  ("products", "products"),
];

/// Slot-name → `ResolutionInput` field name. Used to convert the
/// recovered evidence into the shape the next synthesis turn
/// expects. The dispatcher emits a `supplied_parameters` BTreeMap
/// keyed by these field names.
pub const SLOT_TO_RESOLUTION_INPUT_FIELD: &[(&str, &str)] = &[
  ("import_spec", "candidate_import_spec"),
  ("candidate_imports", "candidate_imports_for_unused"),
  ("target_path", "target_path"),
  ("language", "scope_hint"),
  ("old_name", "utterance_hint"),
  ("new_name", "utterance_hint"),
  ("test_name", "utterance_hint"),
  // Math-domain slot — substrate-sharing: same dispatcher mechanism,
  // different domain. `equivalent_form` is the key
  // `propose_math_expression_lower` looks for in ankh entries'
  // supplied_parameters.
  ("equivalent_form", "equivalent_form"),
  // Chemistry-domain slot — substrate-sharing N=3.
  ("products", "products"),
];

/// What retrieval recovered (or didn't).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "status")]
pub enum RetrievalResult {
  /// Evidence successfully recovered. `supplied_parameters` is
  /// indexed by `ResolutionInput` field name; caller can map these
  /// directly into a new `ResolutionInput` for the next synthesis
  /// turn.
  RetrievalReady {
    channel: HeldQueryRecoveryChannel,
    supplied_parameters: BTreeMap<String, String>,
    /// Slot names actually filled. Subset of
    /// `query.evidence_to_recover`. If shorter than the original,
    /// the next turn may Hold again on remaining missing slots.
    filled_slots: Vec<String>,
  },
  /// Channel was implemented but the lookup didn't produce
  /// evidence for any requested slot. Caller should fall back to
  /// `query.fallback_channel` (or operator-followup).
  RetrievalHeld {
    channel: HeldQueryRecoveryChannel,
    reason: String,
  },
  /// `not-recoverable` channel: the held kind cannot be recovered
  /// by retrieval. Caller must ask the operator to restructure.
  RetrievalRejected {
    channel: HeldQueryRecoveryChannel,
    reason: String,
  },
  /// Channel exists in routing but is not yet wired. Caller should
  /// try `query.fallback_channel` if any, else surface the
  /// deferred state to the operator.
  RetrievalDeferred {
    channel: HeldQueryRecoveryChannel,
    reason: String,
  },
}

fn channel_status(channel: HeldQueryRecoveryChannel) -> ChannelImplementationStatus {
  CHANNEL_IMPLEMENTATION_STATUS
    .iter()
    .find(|r| r.channel == channel)
    .map(|r| r.status)
    .unwrap_or(ChannelImplementationStatus::Deferred)
}

fn host_key_for_slot(slot: &str) -> Option<&'static str> {
  EVIDENCE_SLOT_TO_HOST_KEY
    .iter()
    .find(|(s, _)| *s == slot)
    .map(|(_, k)| *k)
}

fn resolution_input_field_for_slot(slot: &str) -> Option<&'static str> {
  SLOT_TO_RESOLUTION_INPUT_FIELD
    .iter()
    .find(|(s, _)| *s == slot)
    .map(|(_, f)| *f)
}

/// Try one implemented channel. Returns the typed result for that
/// channel; the outer dispatcher decides whether to fall back.
fn try_channel(
  channel: HeldQueryRecoveryChannel,
  query: &HeldRetrievalQuery,
  evidence: &HostEvidence,
) -> RetrievalResult {
  match channel_status(channel) {
    ChannelImplementationStatus::Deferred => {
      return RetrievalResult::RetrievalDeferred {
        channel,
        reason: format!(
          "channel `{}` is not wired yet (Stage B v0 defers external-knowledge-search and ankh-retrieval)",
          channel.as_str()
        ),
      };
    }
    ChannelImplementationStatus::Implemented => {}
  }

  match channel {
    HeldQueryRecoveryChannel::NotRecoverable => RetrievalResult::RetrievalRejected {
      channel,
      reason: format!(
        "held kind `{}` is not recoverable by retrieval — operator must restructure the input",
        query.held_kind.as_str()
      ),
    },
    HeldQueryRecoveryChannel::OperatorFollowup => {
      // Pass-through: the cockpit consumes query.query_text +
      // evidence_to_recover verbatim. We record `Held` because no
      // evidence was *retrieved* — only the prompt was surfaced.
      // The cockpit's operator response feeds a fresh synthesis
      // turn through the normal NL path.
      RetrievalResult::RetrievalHeld {
        channel,
        reason: "operator-followup is the prompt itself; awaiting human response".to_string(),
      }
    }
    HeldQueryRecoveryChannel::HostSymbolResolver => {
      // Look up each evidence_to_recover slot in the host
      // evidence pool. Map matched slots to ResolutionInput field
      // names via SLOT_TO_RESOLUTION_INPUT_FIELD.
      let mut supplied: BTreeMap<String, String> = BTreeMap::new();
      let mut filled: Vec<String> = Vec::new();
      for slot in &query.evidence_to_recover {
        let Some(host_key) = host_key_for_slot(slot) else {
          continue;
        };
        let Some(value) = evidence.get(host_key) else {
          continue;
        };
        let Some(field) = resolution_input_field_for_slot(slot) else {
          continue;
        };
        supplied.insert(field.to_string(), value.clone());
        filled.push(slot.clone());
      }
      if filled.is_empty() {
        RetrievalResult::RetrievalHeld {
          channel,
          reason: format!(
            "host symbol resolver pool has no entries for any of {:?}",
            query.evidence_to_recover
          ),
        }
      } else {
        RetrievalResult::RetrievalReady {
          channel,
          supplied_parameters: supplied,
          filled_slots: filled,
        }
      }
    }
    HeldQueryRecoveryChannel::ExternalKnowledgeSearch | HeldQueryRecoveryChannel::AnkhRetrieval => {
      // Status-table guard above already returned Deferred; this
      // arm is unreachable but typed-exhaustive.
      RetrievalResult::RetrievalDeferred {
        channel,
        reason: format!("channel `{}` deferred", channel.as_str()),
      }
    }
  }
}

/// Main entry: execute a retrieval against the query's primary
/// channel; on Held / Deferred, try the fallback channel if any.
/// `Rejected` and `Ready` are terminal (no fallback).
pub fn execute_retrieval(query: &HeldRetrievalQuery, evidence: &HostEvidence) -> RetrievalResult {
  let primary = try_channel(query.primary_channel, query, evidence);
  match primary {
    RetrievalResult::RetrievalReady { .. } | RetrievalResult::RetrievalRejected { .. } => primary,
    RetrievalResult::RetrievalHeld { .. } | RetrievalResult::RetrievalDeferred { .. } => {
      let Some(fallback) = query.fallback_channel else {
        return primary;
      };
      try_channel(fallback, query, evidence)
    }
  }
}

/// One-word status string for the outcome — used in artifact JSON
/// and panel display. Keeps the family flat (one
/// `coding.retrieval-result` family covers all 4 outcomes
/// via the `status` field, same shape as
/// `tool-action-materialization-receipt`).
fn result_status_str(result: &RetrievalResult) -> &'static str {
  match result {
    RetrievalResult::RetrievalReady { .. } => "ready",
    RetrievalResult::RetrievalHeld { .. } => "held",
    RetrievalResult::RetrievalRejected { .. } => "rejected",
    RetrievalResult::RetrievalDeferred { .. } => "deferred",
  }
}

fn result_channel(result: &RetrievalResult) -> HeldQueryRecoveryChannel {
  match result {
    RetrievalResult::RetrievalReady { channel, .. }
    | RetrievalResult::RetrievalHeld { channel, .. }
    | RetrievalResult::RetrievalRejected { channel, .. }
    | RetrievalResult::RetrievalDeferred { channel, .. } => *channel,
  }
}

fn result_reason(result: &RetrievalResult) -> Option<&str> {
  match result {
    RetrievalResult::RetrievalReady { .. } => None,
    RetrievalResult::RetrievalHeld { reason, .. }
    | RetrievalResult::RetrievalRejected { reason, .. }
    | RetrievalResult::RetrievalDeferred { reason, .. } => Some(reason),
  }
}

/// Render a `RetrievalResult` as the canonical JSON payload of a
/// `coding.retrieval-result` artifact. Replay-stable id =
/// SHA-256 of intrinsic identity (status + channel + sorted
/// filled_slots + sorted supplied_parameters key/value pairs).
/// `stored_at_ms` is extrinsic.
///
/// Carries the originating `HeldRetrievalQuery`'s fingerprint
/// (query_kind + held_kind + transform) as `related_refs` so the
/// cockpit can walk back to the Stage A artifact that produced
/// the query.
///
/// Content policy: every field is metadata or supplied_parameters
/// (caller-injected). No source bodies — customer-release safe.
pub fn build_retrieval_result_artifact(
  query: &HeldRetrievalQuery,
  result: &RetrievalResult,
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
) -> serde_json::Value {
  let status = result_status_str(result);
  let channel = result_channel(result);

  let mut h = Sha256::new();
  h.update(b"retrieval-result\x1f");
  h.update(status.as_bytes());
  h.update(b"\x1f");
  h.update(channel.as_str().as_bytes());
  h.update(b"\x1f");
  h.update(query.held_kind.as_str().as_bytes());
  h.update(b"\x1f");
  h.update(query.transform.as_bytes());
  h.update(b"\x1f");
  if let RetrievalResult::RetrievalReady {
    supplied_parameters,
    filled_slots,
    ..
  } = result
  {
    let mut sorted_slots = filled_slots.clone();
    sorted_slots.sort();
    for slot in &sorted_slots {
      h.update(slot.as_bytes());
      h.update(b"\x1e");
    }
    h.update(b"\x1f");
    let mut keys: Vec<&String> = supplied_parameters.keys().collect();
    keys.sort();
    for k in keys {
      h.update(k.as_bytes());
      h.update(b"\x1d");
      h.update(supplied_parameters[k].as_bytes());
      h.update(b"\x1e");
    }
  }
  if let Some(reason) = result_reason(result) {
    h.update(reason.as_bytes());
  }
  let digest = h.finalize();
  let prefix = digest
    .iter()
    .take(16)
    .map(|b| format!("{b:02x}"))
    .collect::<String>();
  let id = format!("retrieval-result.{prefix}");

  let mut payload = serde_json::json!({
    "id": id,
    "artifact_family": "coding.retrieval-result",
    "source_surface": "algorithm-synthesis.retrieval-execution",
    "stored_at_ms": stored_at_ms,
    "status": status,
    "channel": channel.as_str(),
    "held_kind": query.held_kind.as_str(),
    "transform": query.transform,
    "query_kind": query.query_kind,
    "related_refs": serde_json::json!([
      format!("transform:{}", query.transform),
      format!("held-kind:{}", query.held_kind.as_str()),
      format!("query-kind:{}", query.query_kind),
      "owner-law:stdlib/lib/gate/algorithm-synthesis/retrieval-execution.px",
    ]),
    "target_paths": Vec::<String>::new(),
    "command_refs": Vec::<String>::new(),
  });

  // Per-outcome fields.
  match result {
    RetrievalResult::RetrievalReady {
      supplied_parameters,
      filled_slots,
      ..
    } => {
      payload["filled_slots"] = serde_json::to_value(filled_slots).unwrap_or_default();
      payload["supplied_parameters"] =
        serde_json::to_value(supplied_parameters).unwrap_or_default();
    }
    RetrievalResult::RetrievalHeld { reason, .. }
    | RetrievalResult::RetrievalRejected { reason, .. }
    | RetrievalResult::RetrievalDeferred { reason, .. } => {
      payload["reason"] = serde_json::Value::String(reason.clone());
    }
  }

  if let Some(snap) = repo_snapshot_ref {
    payload["repo_snapshot_ref"] = serde_json::Value::String(snap.to_string());
  }
  payload
}

#[cfg(test)]
mod tests {
  use super::super::held_to_query::build_query_from_held;
  use super::super::parameter_resolution::{
    resolve_parameters, ResolutionHeldKind, ResolutionInput, ResolutionVerdict,
  };
  use super::*;

  fn make_evidence(pairs: &[(&str, &str)]) -> HostEvidence {
    pairs
      .iter()
      .map(|(k, v)| (k.to_string(), v.to_string()))
      .collect()
  }

  // ─── registry consistency ──────────────────────────────────────

  #[test]
  fn every_recovery_channel_has_a_status_row() {
    for c in HeldQueryRecoveryChannel::ALL {
      let found = CHANNEL_IMPLEMENTATION_STATUS
        .iter()
        .any(|r| &r.channel == c);
      assert!(
        found,
        "channel `{}` has no status row — universe drift",
        c.as_str()
      );
    }
  }

  #[test]
  fn no_duplicate_channel_status_rows() {
    let mut seen = std::collections::HashSet::new();
    for r in CHANNEL_IMPLEMENTATION_STATUS {
      assert!(
        seen.insert(r.channel.as_str()),
        "duplicate channel status row: `{}`",
        r.channel.as_str()
      );
    }
  }

  // ─── operator-followup channel ─────────────────────────────────

  #[test]
  fn operator_followup_returns_held_awaiting_human() {
    let v = resolve_parameters(&ResolutionInput {
      operation_candidate: "rename-symbol".to_string(),
      utterance: "이 함수 이름 바꿔줘 src/a.rs".to_string(),
      ..Default::default()
    });
    let q = build_query_from_held(&v).expect("query");
    let r = execute_retrieval(&q, &HostEvidence::new());
    match r {
      RetrievalResult::RetrievalHeld { channel, .. } => {
        assert_eq!(channel, HeldQueryRecoveryChannel::OperatorFollowup);
      }
      other => panic!("expected Held(OperatorFollowup), got {other:?}"),
    }
  }

  // ─── host-symbol-resolver channel ──────────────────────────────

  #[test]
  fn host_symbol_resolver_fills_import_spec() {
    let v = resolve_parameters(&ResolutionInput {
      operation_candidate: "add-import".to_string(),
      utterance: "src/util.py 에 import 추가".to_string(),
      ..Default::default()
    });
    let q = build_query_from_held(&v).expect("query");
    assert_eq!(
      q.primary_channel,
      HeldQueryRecoveryChannel::HostSymbolResolver
    );
    let evidence = make_evidence(&[("import_spec", "import os")]);
    let r = execute_retrieval(&q, &evidence);
    match r {
      RetrievalResult::RetrievalReady {
        channel,
        supplied_parameters,
        filled_slots,
      } => {
        assert_eq!(channel, HeldQueryRecoveryChannel::HostSymbolResolver);
        assert_eq!(
          supplied_parameters.get("candidate_import_spec").unwrap(),
          "import os"
        );
        assert!(filled_slots.contains(&"import_spec".to_string()));
      }
      other => panic!("expected Ready, got {other:?}"),
    }
  }

  #[test]
  fn host_symbol_resolver_held_when_evidence_pool_empty_falls_back_to_external() {
    let v = resolve_parameters(&ResolutionInput {
      operation_candidate: "add-import".to_string(),
      utterance: "src/util.py 에 import 추가".to_string(),
      ..Default::default()
    });
    let q = build_query_from_held(&v).expect("query");
    // Empty evidence pool → host-symbol-resolver Holds → fallback
    // is external-knowledge-search (Deferred for v0).
    let r = execute_retrieval(&q, &HostEvidence::new());
    match r {
      RetrievalResult::RetrievalDeferred { channel, .. } => {
        assert_eq!(channel, HeldQueryRecoveryChannel::ExternalKnowledgeSearch);
      }
      other => panic!("expected Deferred(ExternalKnowledgeSearch), got {other:?}"),
    }
  }

  #[test]
  fn host_symbol_resolver_fills_candidate_imports_list() {
    let v = resolve_parameters(&ResolutionInput {
      operation_candidate: "remove-unused-import".to_string(),
      utterance: "src/a.py 에서 안 쓰는 import 지워줘".to_string(),
      ..Default::default()
    });
    let q = build_query_from_held(&v).expect("query");
    let evidence = make_evidence(&[("candidate_imports", "os,sys")]);
    let r = execute_retrieval(&q, &evidence);
    match r {
      RetrievalResult::RetrievalReady {
        supplied_parameters,
        ..
      } => {
        // Value is forwarded verbatim — the synthesis turn caller
        // parses the list shape (currently host-supplied as a
        // single value; future evidence types can carry richer
        // shapes).
        assert_eq!(
          supplied_parameters
            .get("candidate_imports_for_unused")
            .unwrap(),
          "os,sys"
        );
      }
      other => panic!("expected Ready, got {other:?}"),
    }
  }

  // ─── not-recoverable channel ───────────────────────────────────

  #[test]
  fn not_recoverable_returns_rejected() {
    let v = ResolutionVerdict::ResolutionRejected {
      transform: "rename-symbol".to_string(),
      held_kind: ResolutionHeldKind::InvalidIdentifier,
      reason: "non-ident".to_string(),
    };
    let q = build_query_from_held(&v).expect("query");
    let r = execute_retrieval(&q, &HostEvidence::new());
    assert!(matches!(r, RetrievalResult::RetrievalRejected { .. }));
  }

  // ─── deferred channels ─────────────────────────────────────────

  #[test]
  fn deferred_channels_return_deferred_status() {
    let r = try_channel(
      HeldQueryRecoveryChannel::ExternalKnowledgeSearch,
      // Dummy query; the channel-status guard short-circuits.
      &HeldRetrievalQuery {
        query_kind: "lookup-module-providing-symbol".into(),
        held_kind: ResolutionHeldKind::MissingImportSpec,
        transform: "add-import".into(),
        primary_channel: HeldQueryRecoveryChannel::HostSymbolResolver,
        fallback_channel: Some(HeldQueryRecoveryChannel::ExternalKnowledgeSearch),
        query_text: "".into(),
        evidence_to_recover: vec![],
        context_fields: BTreeMap::new(),
        try_ankh_first: true,
        reason: "".into(),
      },
      &HostEvidence::new(),
    );
    assert!(matches!(r, RetrievalResult::RetrievalDeferred { .. }));
  }

  // ─── artifact builder ─────────────────────────────────────────

  fn add_import_query() -> HeldRetrievalQuery {
    let v = resolve_parameters(&ResolutionInput {
      operation_candidate: "add-import".to_string(),
      utterance: "src/util.py 에 import 추가".to_string(),
      ..Default::default()
    });
    build_query_from_held(&v).expect("query")
  }

  #[test]
  fn artifact_ready_carries_filled_slots_and_parameters() {
    let q = add_import_query();
    let evidence = make_evidence(&[("import_spec", "import os")]);
    let result = execute_retrieval(&q, &evidence);
    let art = build_retrieval_result_artifact(&q, &result, 1700000000000, None);
    assert_eq!(art["artifact_family"], "coding.retrieval-result");
    assert_eq!(art["status"], "ready");
    assert_eq!(art["channel"], "host-symbol-resolver");
    assert_eq!(art["held_kind"], "missing-import-spec");
    assert_eq!(art["transform"], "add-import");
    assert!(art["filled_slots"]
      .as_array()
      .unwrap()
      .iter()
      .any(|v| v == "import_spec"));
    assert_eq!(
      art["supplied_parameters"]["candidate_import_spec"],
      "import os"
    );
  }

  #[test]
  fn artifact_held_carries_reason() {
    let v = resolve_parameters(&ResolutionInput {
      operation_candidate: "rename-symbol".to_string(),
      utterance: "이 함수 이름 바꿔줘 src/a.rs".to_string(),
      ..Default::default()
    });
    let q = build_query_from_held(&v).expect("query");
    let result = execute_retrieval(&q, &HostEvidence::new());
    let art = build_retrieval_result_artifact(&q, &result, 0, None);
    assert_eq!(art["status"], "held");
    assert_eq!(art["channel"], "operator-followup");
    assert!(art["reason"]
      .as_str()
      .unwrap()
      .contains("operator-followup"));
    // Held outcome → no filled_slots / supplied_parameters fields.
    assert!(art.get("filled_slots").is_none());
    assert!(art.get("supplied_parameters").is_none());
  }

  #[test]
  fn artifact_rejected_for_not_recoverable() {
    let v = ResolutionVerdict::ResolutionRejected {
      transform: "rename-symbol".to_string(),
      held_kind: ResolutionHeldKind::InvalidIdentifier,
      reason: "non-ident".to_string(),
    };
    let q = build_query_from_held(&v).expect("query");
    let result = execute_retrieval(&q, &HostEvidence::new());
    let art = build_retrieval_result_artifact(&q, &result, 0, None);
    assert_eq!(art["status"], "rejected");
    assert_eq!(art["channel"], "not-recoverable");
  }

  #[test]
  fn artifact_id_is_replay_stable() {
    let q = add_import_query();
    let evidence = make_evidence(&[("import_spec", "import os")]);
    let result = execute_retrieval(&q, &evidence);
    let a1 = build_retrieval_result_artifact(&q, &result, 1000, None);
    let a2 = build_retrieval_result_artifact(&q, &result, 99999999, None);
    assert_eq!(a1["id"], a2["id"]);
    assert!(a1["id"].as_str().unwrap().starts_with("retrieval-result."));
  }

  #[test]
  fn artifact_related_refs_walk_back_to_query() {
    let q = add_import_query();
    let result = execute_retrieval(&q, &make_evidence(&[("import_spec", "import os")]));
    let art = build_retrieval_result_artifact(&q, &result, 0, None);
    let refs = art["related_refs"].as_array().unwrap();
    assert!(refs
      .iter()
      .any(|v| v.as_str().unwrap().contains("transform:add-import")));
    assert!(refs.iter().any(|v| v
      .as_str()
      .unwrap()
      .contains("held-kind:missing-import-spec")));
    assert!(refs.iter().any(|v| v
      .as_str()
      .unwrap()
      .contains("query-kind:lookup-module-providing-symbol")));
  }
}
