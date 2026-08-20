//! Ankh-retrieval cache — Stage C of the evolution lane.
//!
//! OWNER-LAW (2026-05-12): mirror of
//! `stdlib/lib/gate/algorithm-synthesis/ankh-retrieval-cache.px`.
//! Sits between the caller and `retrieval_execution::execute_retrieval`
//! to close the `ankh-retrieval` recovery channel — same-shape
//! queries that have already been answered get served from
//! accumulated knowledge instead of re-running the primary channel.
//!
//! Per the constitution, ankh is NOT a passive cache — every entry
//! carries provenance (recovery channel, contributing actor/tenant,
//! timestamp) so promotion gates (Stage D) can audit which fixtures
//! justify a candidate `.px` row. Drop the provenance and the cache
//! becomes a memory-corruption vector.

use serde::{Deserialize, Serialize};
use pnix_hash::{Digest, Sha256};
use std::collections::BTreeMap;

use super::held_to_query::{HeldQueryRecoveryChannel, HeldRetrievalQuery};
use super::retrieval_execution::{execute_retrieval, HostEvidence, RetrievalResult};

/// Where an ankh entry's evidence originally came from. Sync test
/// asserts parity against `.px` `validProvenanceSources`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AnkhProvenanceSource {
  HostSymbolResolver,
  ExternalKnowledgeSearch,
  OperatorFollowup,
}

impl AnkhProvenanceSource {
  pub const ALL: &'static [Self] = &[
    Self::HostSymbolResolver,
    Self::ExternalKnowledgeSearch,
    Self::OperatorFollowup,
  ];

  pub fn as_str(self) -> &'static str {
    match self {
      Self::HostSymbolResolver => "host-symbol-resolver",
      Self::ExternalKnowledgeSearch => "external-knowledge-search",
      Self::OperatorFollowup => "operator-followup",
    }
  }

  /// Project a `HeldQueryRecoveryChannel` into a provenance source.
  /// Returns `None` for channels that don't produce ankh-writable
  /// evidence (`AnkhRetrieval` itself — that would be a self-loop —
  /// and `NotRecoverable`).
  pub fn from_channel(channel: HeldQueryRecoveryChannel) -> Option<Self> {
    Some(match channel {
      HeldQueryRecoveryChannel::HostSymbolResolver => Self::HostSymbolResolver,
      HeldQueryRecoveryChannel::ExternalKnowledgeSearch => Self::ExternalKnowledgeSearch,
      HeldQueryRecoveryChannel::OperatorFollowup => Self::OperatorFollowup,
      HeldQueryRecoveryChannel::AnkhRetrieval | HeldQueryRecoveryChannel::NotRecoverable => {
        return None
      }
    })
  }
}

/// Fields in declared order from `.px` `ankhKeyFields`. Used by
/// `AnkhRetrievalKey::from_query` to build a canonical fingerprint.
pub const ANKH_KEY_FIELDS: &[&str] = &["query_kind", "target_path", "language"];

/// Replay-stable key into the ankh store. Two queries with the
/// same `(query_kind, target_path, language)` triple hit the same
/// entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct AnkhRetrievalKey {
  pub query_kind: String,
  pub target_path: String,
  pub language: String,
}

impl AnkhRetrievalKey {
  pub fn from_query(query: &HeldRetrievalQuery) -> Self {
    let get = |field: &str| {
      // `target_path` has transform-specific aliases.
      if field == "target_path" {
        for alias in &["target_path", "target_paths", "target_module"] {
          if let Some(v) = query.context_fields.get(*alias) {
            return v.clone();
          }
        }
        return String::new();
      }
      query.context_fields.get(field).cloned().unwrap_or_default()
    };
    Self {
      query_kind: query.query_kind.clone(),
      target_path: get("target_path"),
      language: get("language"),
    }
  }
}

/// A single ankh entry. ALL fields in `requiredProvenanceFields`
/// from the `.px` are non-Option here — the type system enforces
/// that an entry without provenance is unrepresentable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnkhEntry {
  /// The recovery channel that originally produced this evidence.
  pub provenance_source: AnkhProvenanceSource,
  /// Actor id of whoever supplied the host evidence (or operator).
  pub contributing_actor_id: String,
  /// Tenant scope. Cross-tenant reads are caller-policy in v0.
  pub contributing_tenant_id: String,
  /// Wall-clock ms when the entry was committed. Replay/supersede
  /// anchor.
  pub stored_at_ms: u64,
  /// The query_kind that this entry answers. Kept on the entry
  /// (not just the key) so audit dumps can reconstruct the lookup
  /// without re-deriving the key.
  pub query_kind: String,
  /// The recovered parameters, in the shape the next synthesis
  /// turn expects (keyed by `ResolutionInput` field name).
  pub supplied_parameters: BTreeMap<String, String>,
  /// Slot names this entry filled — subset of the original query's
  /// `evidence_to_recover`.
  pub filled_slots: Vec<String>,
  /// Snapshot of the originating query's `context_fields` at write
  /// time. Carries fields that were *already known* (lifted from
  /// the utterance / partial_resolution) but not part of the key
  /// — e.g. `canonical_form` for the math lane, where the
  /// candidate-row proposer needs both `canonical_form` (context)
  /// and `equivalent_form` (recovered) to group entries by
  /// identity. Default-empty for back-compat — coding-lane writes
  /// before this field landed populate via `Default`.
  #[serde(default)]
  pub context_snapshot: BTreeMap<String, String>,
}

/// Trait that any ankh store must implement. v0 ships
/// `InMemoryAnkhStore`; v1 will add a doghouse-redb impl under the
/// same trait.
pub trait AnkhStore {
  fn get(&self, key: &AnkhRetrievalKey) -> Option<AnkhEntry>;
  fn put(&mut self, key: AnkhRetrievalKey, entry: AnkhEntry);
  fn len(&self) -> usize;
  fn is_empty(&self) -> bool {
    self.len() == 0
  }
  /// Snapshot of every entry. Stage D consumers (candidate-row
  /// proposers) walk this to find recurring patterns. v0 returns
  /// owned clones; persistent-store impls can optimize this later
  /// by returning an iterator over borrowed entries.
  fn iter_entries(&self) -> Vec<(AnkhRetrievalKey, AnkhEntry)>;
}

/// Test-grade session-scoped store. Entries live for the lifetime
/// of the `InMemoryAnkhStore` instance. v1 swaps this for a
/// doghouse-redb backend under the same trait.
#[derive(Debug, Clone, Default)]
pub struct InMemoryAnkhStore {
  entries: BTreeMap<AnkhRetrievalKey, AnkhEntry>,
}

impl InMemoryAnkhStore {
  pub fn new() -> Self {
    Self::default()
  }
}

impl AnkhStore for InMemoryAnkhStore {
  fn get(&self, key: &AnkhRetrievalKey) -> Option<AnkhEntry> {
    self.entries.get(key).cloned()
  }
  fn put(&mut self, key: AnkhRetrievalKey, entry: AnkhEntry) {
    self.entries.insert(key, entry);
  }
  fn len(&self) -> usize {
    self.entries.len()
  }
  fn iter_entries(&self) -> Vec<(AnkhRetrievalKey, AnkhEntry)> {
    self
      .entries
      .iter()
      .map(|(k, v)| (k.clone(), v.clone()))
      .collect()
  }
}

/// Context that wraps `execute_retrieval` with ankh-first lookup +
/// write-back. The contributing actor/tenant are required for any
/// entry the wrapper inserts — caller passes them so the audit
/// trail attributes the evidence to a real identity, not a system
/// pseudo-actor.
#[derive(Debug, Clone)]
pub struct AnkhRetrievalContext<'a> {
  pub contributing_actor_id: &'a str,
  pub contributing_tenant_id: &'a str,
  pub stored_at_ms: u64,
}

/// Try ankh first, then `execute_retrieval`. On a primary-channel
/// Ready, write the answer back to ankh under the provided
/// provenance. On ankh hit, return a `RetrievalReady` with channel
/// `AnkhRetrieval` and the cached `supplied_parameters`.
///
/// `query.try_ankh_first == false` (currently set for
/// `not-recoverable` primary) skips both the read and the write —
/// `not-recoverable` evidence has no shape we'd want to memoize.
pub fn execute_retrieval_with_ankh<S: AnkhStore>(
  query: &HeldRetrievalQuery,
  evidence: &HostEvidence,
  ankh: &mut S,
  ctx: &AnkhRetrievalContext<'_>,
) -> RetrievalResult {
  if !query.try_ankh_first {
    return execute_retrieval(query, evidence);
  }
  let key = AnkhRetrievalKey::from_query(query);

  // Read path: ankh-first.
  if let Some(entry) = ankh.get(&key) {
    return RetrievalResult::RetrievalReady {
      channel: HeldQueryRecoveryChannel::AnkhRetrieval,
      supplied_parameters: entry.supplied_parameters,
      filled_slots: entry.filled_slots,
    };
  }

  // Miss: run the standard dispatcher.
  let result = execute_retrieval(query, evidence);

  // Write path: record successful Ready answers back into ankh
  // under their original recovery channel's provenance. Held /
  // Rejected / Deferred do NOT write — partial / empty / unwired
  // evidence is not knowledge.
  if let RetrievalResult::RetrievalReady {
    channel,
    supplied_parameters,
    filled_slots,
  } = &result
  {
    if let Some(provenance) = AnkhProvenanceSource::from_channel(*channel) {
      ankh.put(
        key,
        AnkhEntry {
          provenance_source: provenance,
          contributing_actor_id: ctx.contributing_actor_id.to_string(),
          contributing_tenant_id: ctx.contributing_tenant_id.to_string(),
          stored_at_ms: ctx.stored_at_ms,
          query_kind: query.query_kind.clone(),
          supplied_parameters: supplied_parameters.clone(),
          filled_slots: filled_slots.clone(),
          context_snapshot: query.context_fields.clone(),
        },
      );
    }
  }
  result
}

/// Receipt status — describes how the entry surfaced in the current
/// turn. Same flat-family pattern as `coding.retrieval-result`:
/// one `coding.ankh-entry-receipt` family covers hit / write /
/// miss via the `status` field.
///
/// - `cache-hit`   — `ankh.get(key)` returned an existing entry.
/// - `cache-write` — primary channel produced a Ready answer and the
///                   wrapper wrote it back into ankh.
/// - `cache-miss`  — `ankh.get(key)` returned `None`. Receipt carries
///                   the key but no entry payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AnkhEntryReceiptStatus {
  CacheHit,
  CacheWrite,
  CacheMiss,
}

impl AnkhEntryReceiptStatus {
  pub fn as_str(self) -> &'static str {
    match self {
      Self::CacheHit => "cache-hit",
      Self::CacheWrite => "cache-write",
      Self::CacheMiss => "cache-miss",
    }
  }
}

/// Render an ankh entry (or miss) as the canonical JSON payload of a
/// `coding.ankh-entry-receipt` artifact. Replay-stable id =
/// SHA-256 of intrinsic identity (status + key + provenance source +
/// actor + tenant + sorted filled_slots + sorted supplied_parameters
/// key/value pairs). `observed_at_ms` is extrinsic.
///
/// `entry` is `None` for `cache-miss`. For `cache-hit` and
/// `cache-write`, the entry's own `stored_at_ms` is preserved as
/// part of the receipt (intrinsic to the entry, distinct from the
/// observation timestamp).
///
/// Content policy: every field is metadata or supplied_parameters
/// (caller-injected). No source bodies — customer-release safe.
pub fn build_ankh_entry_receipt_artifact(
  key: &AnkhRetrievalKey,
  entry: Option<&AnkhEntry>,
  status: AnkhEntryReceiptStatus,
  observed_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
) -> serde_json::Value {
  let mut h = Sha256::new();
  h.update(b"ankh-entry-receipt\x1f");
  h.update(status.as_str().as_bytes());
  h.update(b"\x1f");
  h.update(key.query_kind.as_bytes());
  h.update(b"\x1e");
  h.update(key.target_path.as_bytes());
  h.update(b"\x1e");
  h.update(key.language.as_bytes());
  h.update(b"\x1f");
  if let Some(e) = entry {
    h.update(e.provenance_source.as_str().as_bytes());
    h.update(b"\x1e");
    h.update(e.contributing_actor_id.as_bytes());
    h.update(b"\x1e");
    h.update(e.contributing_tenant_id.as_bytes());
    h.update(b"\x1e");
    h.update(e.query_kind.as_bytes());
    h.update(b"\x1f");
    let mut sorted_slots = e.filled_slots.clone();
    sorted_slots.sort();
    for slot in &sorted_slots {
      h.update(slot.as_bytes());
      h.update(b"\x1e");
    }
    h.update(b"\x1f");
    let mut keys: Vec<&String> = e.supplied_parameters.keys().collect();
    keys.sort();
    for k in keys {
      h.update(k.as_bytes());
      h.update(b"\x1d");
      h.update(e.supplied_parameters[k].as_bytes());
      h.update(b"\x1e");
    }
  }
  let digest = h.finalize();
  let prefix = digest
    .iter()
    .take(16)
    .map(|b| format!("{b:02x}"))
    .collect::<String>();
  let id = format!("ankh-entry-receipt.{prefix}");

  let mut payload = serde_json::json!({
    "id": id,
    "artifact_family": "coding.ankh-entry-receipt",
    "source_surface": "algorithm-synthesis.ankh-retrieval-cache",
    "observed_at_ms": observed_at_ms,
    "status": status.as_str(),
    "query_kind": key.query_kind,
    "target_path": key.target_path,
    "language": key.language,
    "related_refs": serde_json::json!([
      format!("query-kind:{}", key.query_kind),
      format!("target-path:{}", key.target_path),
      format!("language:{}", key.language),
      "owner-law:stdlib/lib/gate/algorithm-synthesis/ankh-retrieval-cache.px",
    ]),
    "target_paths": Vec::<String>::new(),
    "command_refs": Vec::<String>::new(),
  });

  if let Some(e) = entry {
    payload["provenance_source"] =
      serde_json::Value::String(e.provenance_source.as_str().to_string());
    payload["contributing_actor_id"] = serde_json::Value::String(e.contributing_actor_id.clone());
    payload["contributing_tenant_id"] = serde_json::Value::String(e.contributing_tenant_id.clone());
    payload["entry_stored_at_ms"] =
      serde_json::Value::Number(serde_json::Number::from(e.stored_at_ms));
    payload["filled_slots"] = serde_json::to_value(&e.filled_slots).unwrap_or_default();
    payload["supplied_parameters"] =
      serde_json::to_value(&e.supplied_parameters).unwrap_or_default();
  }

  if let Some(snap) = repo_snapshot_ref {
    payload["repo_snapshot_ref"] = serde_json::Value::String(snap.to_string());
  }
  payload
}

#[cfg(test)]
mod tests {
  use super::super::held_to_query::build_query_from_held;
  use super::super::parameter_resolution::{resolve_parameters, ResolutionInput};
  use super::*;

  fn ctx<'a>() -> AnkhRetrievalContext<'a> {
    AnkhRetrievalContext {
      contributing_actor_id: "actor.test",
      contributing_tenant_id: "tenant.test",
      stored_at_ms: 1700000000000,
    }
  }

  fn add_import_query() -> HeldRetrievalQuery {
    let v = resolve_parameters(&ResolutionInput {
      operation_candidate: "add-import".to_string(),
      utterance: "src/util.py 에 import 추가".to_string(),
      ..Default::default()
    });
    build_query_from_held(&v).expect("query")
  }

  fn host_evidence_import_os() -> HostEvidence {
    let mut e: HostEvidence = HostEvidence::new();
    e.insert("import_spec".to_string(), "import os".to_string());
    e
  }

  // ─── key derivation ────────────────────────────────────────────

  #[test]
  fn key_derived_from_query_kind_target_and_language() {
    let q = add_import_query();
    let k = AnkhRetrievalKey::from_query(&q);
    assert_eq!(k.query_kind, "lookup-module-providing-symbol");
    assert_eq!(k.target_path, "src/util.py");
    assert_eq!(k.language, "python");
  }

  #[test]
  fn same_query_shape_yields_same_key() {
    let q1 = add_import_query();
    let q2 = add_import_query();
    assert_eq!(
      AnkhRetrievalKey::from_query(&q1),
      AnkhRetrievalKey::from_query(&q2)
    );
  }

  // ─── miss → write → hit cycle ─────────────────────────────────

  #[test]
  fn first_call_misses_ankh_and_writes_back() {
    let q = add_import_query();
    let mut ankh = InMemoryAnkhStore::new();
    assert!(ankh.is_empty());
    let r1 = execute_retrieval_with_ankh(&q, &host_evidence_import_os(), &mut ankh, &ctx());
    match r1 {
      RetrievalResult::RetrievalReady { channel, .. } => {
        // First call resolves through host-symbol-resolver.
        assert_eq!(channel, HeldQueryRecoveryChannel::HostSymbolResolver);
      }
      other => panic!("expected Ready, got {other:?}"),
    }
    assert_eq!(ankh.len(), 1, "Ready answer must be written back to ankh");
  }

  #[test]
  fn second_call_hits_ankh_without_host_evidence() {
    let q = add_import_query();
    let mut ankh = InMemoryAnkhStore::new();
    // Seed ankh via first call.
    let _ = execute_retrieval_with_ankh(&q, &host_evidence_import_os(), &mut ankh, &ctx());
    // Second call with EMPTY host evidence still resolves — ankh
    // returns the cached answer.
    let r2 = execute_retrieval_with_ankh(&q, &HostEvidence::new(), &mut ankh, &ctx());
    match r2 {
      RetrievalResult::RetrievalReady {
        channel,
        supplied_parameters,
        ..
      } => {
        assert_eq!(channel, HeldQueryRecoveryChannel::AnkhRetrieval);
        assert_eq!(
          supplied_parameters.get("candidate_import_spec").unwrap(),
          "import os"
        );
      }
      other => panic!("expected ankh hit, got {other:?}"),
    }
  }

  // ─── provenance preservation ──────────────────────────────────

  #[test]
  fn stored_entry_carries_provenance_actor_tenant_and_timestamp() {
    let q = add_import_query();
    let mut ankh = InMemoryAnkhStore::new();
    let _ = execute_retrieval_with_ankh(&q, &host_evidence_import_os(), &mut ankh, &ctx());
    let entry = ankh.get(&AnkhRetrievalKey::from_query(&q)).expect("entry");
    assert_eq!(
      entry.provenance_source,
      AnkhProvenanceSource::HostSymbolResolver
    );
    assert_eq!(entry.contributing_actor_id, "actor.test");
    assert_eq!(entry.contributing_tenant_id, "tenant.test");
    assert_eq!(entry.stored_at_ms, 1700000000000);
    assert_eq!(entry.query_kind, "lookup-module-providing-symbol");
  }

  // ─── do-not-cache cases ───────────────────────────────────────

  #[test]
  fn held_outcome_does_not_pollute_ankh() {
    // operator-followup channel returns Held. Should NOT write.
    let v = resolve_parameters(&ResolutionInput {
      operation_candidate: "rename-symbol".to_string(),
      utterance: "이 함수 이름 바꿔줘 src/a.rs".to_string(),
      ..Default::default()
    });
    let q = build_query_from_held(&v).expect("query");
    let mut ankh = InMemoryAnkhStore::new();
    let _ = execute_retrieval_with_ankh(&q, &HostEvidence::new(), &mut ankh, &ctx());
    assert!(ankh.is_empty(), "Held outcomes must not write to ankh");
  }

  #[test]
  fn not_recoverable_query_skips_ankh_entirely() {
    use super::super::parameter_resolution::ResolutionHeldKind;
    let v = super::super::parameter_resolution::ResolutionVerdict::ResolutionRejected {
      transform: "rename-symbol".to_string(),
      held_kind: ResolutionHeldKind::InvalidIdentifier,
      reason: "non-ident".to_string(),
    };
    let q = build_query_from_held(&v).expect("query");
    assert!(!q.try_ankh_first);
    let mut ankh = InMemoryAnkhStore::new();
    let r = execute_retrieval_with_ankh(&q, &HostEvidence::new(), &mut ankh, &ctx());
    assert!(matches!(r, RetrievalResult::RetrievalRejected { .. }));
    assert!(ankh.is_empty(), "not-recoverable must skip ankh entirely");
  }

  // ─── different queries do not collide ─────────────────────────

  #[test]
  fn different_target_paths_yield_different_entries() {
    let mut ankh = InMemoryAnkhStore::new();
    // Query 1: src/util.py
    let q1 = add_import_query();
    let _ = execute_retrieval_with_ankh(&q1, &host_evidence_import_os(), &mut ankh, &ctx());
    // Query 2: src/other.py with different host answer
    let v2 = resolve_parameters(&ResolutionInput {
      operation_candidate: "add-import".to_string(),
      utterance: "src/other.py 에 import 추가".to_string(),
      ..Default::default()
    });
    let q2 = build_query_from_held(&v2).expect("query");
    let mut ev2 = HostEvidence::new();
    ev2.insert("import_spec".into(), "import sys".into());
    let _ = execute_retrieval_with_ankh(&q2, &ev2, &mut ankh, &ctx());
    assert_eq!(ankh.len(), 2);
    // Re-read q1: still gets `import os`, not `import sys`.
    let r1_redux = execute_retrieval_with_ankh(&q1, &HostEvidence::new(), &mut ankh, &ctx());
    match r1_redux {
      RetrievalResult::RetrievalReady {
        supplied_parameters,
        ..
      } => assert_eq!(
        supplied_parameters.get("candidate_import_spec").unwrap(),
        "import os"
      ),
      other => panic!("expected ankh hit for q1, got {other:?}"),
    }
  }

  // ─── ankh-entry-receipt artifact ──────────────────────────────

  fn write_one_entry() -> (InMemoryAnkhStore, AnkhRetrievalKey) {
    let q = add_import_query();
    let mut ankh = InMemoryAnkhStore::new();
    let _ = execute_retrieval_with_ankh(&q, &host_evidence_import_os(), &mut ankh, &ctx());
    let key = AnkhRetrievalKey::from_query(&q);
    (ankh, key)
  }

  #[test]
  fn receipt_hit_carries_provenance_actor_tenant_and_entry_timestamp() {
    let (ankh, key) = write_one_entry();
    let entry = ankh.get(&key).expect("entry");
    let art = build_ankh_entry_receipt_artifact(
      &key,
      Some(&entry),
      AnkhEntryReceiptStatus::CacheHit,
      1700000099999,
      None,
    );
    assert_eq!(art["artifact_family"], "coding.ankh-entry-receipt");
    assert_eq!(art["status"], "cache-hit");
    assert_eq!(art["provenance_source"], "host-symbol-resolver");
    assert_eq!(art["contributing_actor_id"], "actor.test");
    assert_eq!(art["contributing_tenant_id"], "tenant.test");
    assert_eq!(art["entry_stored_at_ms"], 1700000000000u64);
    assert_eq!(art["observed_at_ms"], 1700000099999u64);
    assert_eq!(art["query_kind"], "lookup-module-providing-symbol");
    assert_eq!(art["target_path"], "src/util.py");
    assert_eq!(art["language"], "python");
    // supplied_parameters preserved
    assert_eq!(
      art["supplied_parameters"]["candidate_import_spec"],
      "import os"
    );
  }

  #[test]
  fn receipt_write_status_uses_same_intrinsic_payload() {
    let (ankh, key) = write_one_entry();
    let entry = ankh.get(&key).expect("entry");
    let art = build_ankh_entry_receipt_artifact(
      &key,
      Some(&entry),
      AnkhEntryReceiptStatus::CacheWrite,
      1700000050000,
      None,
    );
    assert_eq!(art["status"], "cache-write");
    assert_eq!(art["provenance_source"], "host-symbol-resolver");
    assert_eq!(art["contributing_actor_id"], "actor.test");
    assert_eq!(art["entry_stored_at_ms"], 1700000000000u64);
  }

  #[test]
  fn receipt_miss_carries_only_key_no_entry_fields() {
    let q = add_import_query();
    let key = AnkhRetrievalKey::from_query(&q);
    let art = build_ankh_entry_receipt_artifact(
      &key,
      None,
      AnkhEntryReceiptStatus::CacheMiss,
      1700000010000,
      None,
    );
    assert_eq!(art["status"], "cache-miss");
    assert_eq!(art["query_kind"], "lookup-module-providing-symbol");
    assert_eq!(art["target_path"], "src/util.py");
    assert_eq!(art["language"], "python");
    assert!(art.get("provenance_source").is_none());
    assert!(art.get("contributing_actor_id").is_none());
    assert!(art.get("contributing_tenant_id").is_none());
    assert!(art.get("entry_stored_at_ms").is_none());
    assert!(art.get("filled_slots").is_none());
    assert!(art.get("supplied_parameters").is_none());
  }

  #[test]
  fn receipt_id_is_replay_stable_across_observation_time() {
    let (ankh, key) = write_one_entry();
    let entry = ankh.get(&key).expect("entry");
    let a1 = build_ankh_entry_receipt_artifact(
      &key,
      Some(&entry),
      AnkhEntryReceiptStatus::CacheHit,
      1700000000000,
      None,
    );
    let a2 = build_ankh_entry_receipt_artifact(
      &key,
      Some(&entry),
      AnkhEntryReceiptStatus::CacheHit,
      1700000999999,
      None,
    );
    assert_eq!(a1["id"], a2["id"], "id must ignore observed_at_ms");
  }

  #[test]
  fn receipt_id_differs_between_hit_and_miss() {
    let (ankh, key) = write_one_entry();
    let entry = ankh.get(&key).expect("entry");
    let hit = build_ankh_entry_receipt_artifact(
      &key,
      Some(&entry),
      AnkhEntryReceiptStatus::CacheHit,
      0,
      None,
    );
    let miss =
      build_ankh_entry_receipt_artifact(&key, None, AnkhEntryReceiptStatus::CacheMiss, 0, None);
    assert_ne!(hit["id"], miss["id"]);
  }

  #[test]
  fn receipt_related_refs_walk_back_to_query_key() {
    let q = add_import_query();
    let key = AnkhRetrievalKey::from_query(&q);
    let art =
      build_ankh_entry_receipt_artifact(&key, None, AnkhEntryReceiptStatus::CacheMiss, 0, None);
    let refs: Vec<String> = serde_json::from_value(art["related_refs"].clone()).unwrap();
    assert!(refs
      .iter()
      .any(|r| r == "query-kind:lookup-module-providing-symbol"));
    assert!(refs.iter().any(|r| r == "target-path:src/util.py"));
    assert!(refs.iter().any(|r| r == "language:python"));
    assert!(refs.iter().any(|r| r.contains("ankh-retrieval-cache.px")));
  }

  // ─── math lane end-to-end (substrate-sharing proof) ───────────

  use super::super::candidate_row_proposal::{propose_candidates_from_ankh, CandidateKind};
  use super::super::parameter_resolution::{ResolutionHeldKind, ResolutionVerdict};

  /// Build a math-lane Held verdict carrying enough context for
  /// build_query_from_held to produce a `lookup-algebraic-equivalent`
  /// query. The canonical_form arrives via partial_resolution
  /// (would be lifted from the NL utterance in a real session;
  /// here we set it directly to keep the test about retrieval,
  /// not lift).
  fn math_held_verdict(canonical: &str, language: &str, target_path: &str) -> ResolutionVerdict {
    let mut partial: BTreeMap<String, String> = BTreeMap::new();
    partial.insert("canonical_form".to_string(), canonical.to_string());
    partial.insert("language".to_string(), language.to_string());
    partial.insert("target_path".to_string(), target_path.to_string());
    ResolutionVerdict::ResolutionHeld {
      transform: "lookup-algebraic-equivalent".to_string(),
      held_kind: ResolutionHeldKind::MissingAlgebraicEquivalent,
      missing_slots: vec!["equivalent_form".to_string()],
      partial_resolution: partial,
      reason: format!("user asked for an equivalent of `{canonical}`"),
    }
  }

  fn math_host_evidence(equivalent: &str) -> HostEvidence {
    let mut e = HostEvidence::new();
    e.insert("equivalent_form".to_string(), equivalent.to_string());
    e
  }

  /// Math Held verdict projects to a HeldRetrievalQuery with
  /// query_kind=lookup-algebraic-equivalent — the substrate's
  /// proof that the same projection works for a non-coding domain.
  #[test]
  fn math_held_projects_to_lookup_algebraic_equivalent_query() {
    let v = math_held_verdict("x^2 + 2*x*y + y^2", "polynomial", "math/exp-a.md");
    let q = super::super::held_to_query::build_query_from_held(&v).expect("math query");
    assert_eq!(q.query_kind, "lookup-algebraic-equivalent");
    assert_eq!(q.held_kind, ResolutionHeldKind::MissingAlgebraicEquivalent);
    // Primary = external-knowledge-search (deferred in v0), fallback
    // = operator-followup. try_ankh_first must be true because the
    // primary isn't NotRecoverable — math identities recur.
    assert!(q.try_ankh_first);
  }

  /// First math retrieval: ankh miss → host-symbol-resolver returns
  /// Ready with equivalent_form → ankh writes the entry back.
  /// (host-symbol-resolver becomes the de-facto resolver because
  /// external-knowledge-search is Deferred in v0 — host evidence
  /// can supply any slot listed in EVIDENCE_SLOT_TO_HOST_KEY.)
  #[test]
  fn math_first_retrieval_writes_back_to_ankh() {
    let v = math_held_verdict("x^2 + 2*x*y + y^2", "polynomial", "math/exp-a.md");
    let q = super::super::held_to_query::build_query_from_held(&v).expect("q");
    let mut ankh = InMemoryAnkhStore::new();
    let host = math_host_evidence("(x+y)^2");
    let r = execute_retrieval_with_ankh(&q, &host, &mut ankh, &ctx());
    match r {
      RetrievalResult::RetrievalReady {
        supplied_parameters,
        ..
      } => {
        assert_eq!(
          supplied_parameters.get("equivalent_form").unwrap(),
          "(x+y)^2"
        );
      }
      other => panic!("expected Ready, got {other:?}"),
    }
    assert_eq!(ankh.len(), 1, "math Ready must write back to ankh");
    let entry = ankh
      .get(&AnkhRetrievalKey::from_query(&q))
      .expect("math entry");
    assert_eq!(entry.query_kind, "lookup-algebraic-equivalent");
    assert_eq!(
      entry.supplied_parameters.get("equivalent_form").unwrap(),
      "(x+y)^2"
    );
  }

  /// Two distinct math contexts (different target_paths) supplying
  /// the same equivalent for the same canonical → ankh accumulates
  /// two entries → `propose_candidates_from_ankh` emits a
  /// MathExpressionLower proposal targeting knownAlgebraicIdentities.
  /// This is the full substrate-sharing roundtrip executed (NOT a
  /// proof-shape receipt — actual evidence flows through the same
  /// channels that handle coding-lane queries).
  #[test]
  fn math_two_writes_then_proposal_emerges_through_same_substrate() {
    let mut ankh = InMemoryAnkhStore::new();
    // First context
    let v1 = math_held_verdict("x^2 + 2*x*y + y^2", "polynomial", "math/exp-a.md");
    let q1 = super::super::held_to_query::build_query_from_held(&v1).expect("q1");
    let _ = execute_retrieval_with_ankh(&q1, &math_host_evidence("(x+y)^2"), &mut ankh, &ctx());
    // Second context — different target_path so it's a fresh ankh
    // key, but produces the same (canonical, equivalent, language)
    // triple. propose_math_expression_lower groups by that triple.
    let v2 = math_held_verdict("x^2 + 2*x*y + y^2", "polynomial", "math/exp-b.md");
    let q2 = super::super::held_to_query::build_query_from_held(&v2).expect("q2");
    let _ = execute_retrieval_with_ankh(&q2, &math_host_evidence("(x+y)^2"), &mut ankh, &ctx());
    assert_eq!(
      ankh.len(),
      2,
      "two distinct math contexts → two ankh entries"
    );

    // Now run the same propose function that handles coding lanes.
    let proposals = propose_candidates_from_ankh(&ankh);
    let math: Vec<_> = proposals
      .iter()
      .filter(|p| p.candidate_kind == CandidateKind::MathExpressionLower)
      .collect();
    assert_eq!(math.len(), 1, "exactly one math proposal emerges");
    let p = math[0];
    assert_eq!(p.target_table, "knownAlgebraicIdentities");
    assert_eq!(
      p.proposed_row.get("canonical_form").unwrap(),
      "x^2 + 2*x*y + y^2"
    );
    assert_eq!(p.proposed_row.get("equivalent_form").unwrap(), "(x+y)^2");
    assert_eq!(p.evidence_count, 2);
  }

  /// **Full Korean NL → ankh end-to-end roundtrip.** Operator types
  /// a Korean question with embedded math expression →
  /// `resolve_parameters` lifts it to Held(MissingAlgebraicEquivalent)
  /// with canonical_form in partial_resolution → `build_query_from_held`
  /// projects to a retrieval query → `execute_retrieval_with_ankh`
  /// asks host (CAS adapter stand-in) for equivalent_form → ankh
  /// stores entry. Next call ankh-hits.
  ///
  /// This is the closure proof that the **user-facing entry point
  /// (한국어 발화)** lands evidence in the same substrate that
  /// coding-lane utterances do, and that the substrate behaves
  /// uniformly across domains — same dispatcher, same channels, same
  /// write-back. No proof-shape receipt; executed `cargo test`
  /// evidence.
  #[test]
  fn korean_math_utterance_lifts_to_held_query_writes_to_ankh_hits_on_replay() {
    use super::super::parameter_resolution::{
      resolve_parameters, ResolutionHeldKind, ResolutionInput, ResolutionVerdict,
    };

    // Step 1: 한국어 발화 → resolver lifts → Held verdict.
    let verdict = resolve_parameters(&ResolutionInput {
      operation_candidate: "lookup-algebraic-equivalent".to_string(),
      utterance: "x^2 + 2*x*y + y^2 는 뭐야?".to_string(),
      ..Default::default()
    });
    let (transform, partial) = match &verdict {
      ResolutionVerdict::ResolutionHeld {
        transform,
        held_kind,
        partial_resolution,
        ..
      } => {
        assert_eq!(transform, "lookup-algebraic-equivalent");
        assert_eq!(*held_kind, ResolutionHeldKind::MissingAlgebraicEquivalent);
        (transform.clone(), partial_resolution.clone())
      }
      other => panic!("expected Held from korean math utterance, got {other:?}"),
    };
    assert_eq!(partial.get("canonical_form").unwrap(), "x^2 + 2*x*y + y^2");
    assert_eq!(partial.get("language").unwrap(), "polynomial");
    let _ = transform;

    // Step 2: Held verdict → retrieval query (same projection
    // function that handles coding-lane Helds).
    let query = super::super::held_to_query::build_query_from_held(&verdict)
      .expect("math Held projects to retrieval query");
    assert_eq!(query.query_kind, "lookup-algebraic-equivalent");
    assert_eq!(
      query.context_fields.get("canonical_form").unwrap(),
      "x^2 + 2*x*y + y^2"
    );
    assert!(query.try_ankh_first);

    // Step 3: First retrieval — ankh miss → host supplies
    // equivalent_form → ankh writes back.
    let mut ankh = InMemoryAnkhStore::new();
    let host = {
      let mut h = HostEvidence::new();
      h.insert("equivalent_form".to_string(), "(x+y)^2".to_string());
      h
    };
    let r1 = execute_retrieval_with_ankh(&query, &host, &mut ankh, &ctx());
    match &r1 {
      RetrievalResult::RetrievalReady { channel, .. } => {
        assert_eq!(*channel, HeldQueryRecoveryChannel::HostSymbolResolver);
      }
      other => panic!("expected Ready on first retrieval, got {other:?}"),
    }
    assert_eq!(
      ankh.len(),
      1,
      "korean math utterance must write back to ankh"
    );

    // Step 4: Replay with empty host evidence → ankh hits.
    let r2 = execute_retrieval_with_ankh(&query, &HostEvidence::new(), &mut ankh, &ctx());
    match r2 {
      RetrievalResult::RetrievalReady {
        channel,
        supplied_parameters,
        ..
      } => {
        assert_eq!(channel, HeldQueryRecoveryChannel::AnkhRetrieval);
        assert_eq!(
          supplied_parameters.get("equivalent_form").unwrap(),
          "(x+y)^2"
        );
      }
      other => panic!("expected ankh hit on replay, got {other:?}"),
    }

    // Step 5: Verify the stored entry preserves both canonical (from
    // context_snapshot) and equivalent (from supplied_parameters) —
    // the exact shape `propose_math_expression_lower` reads.
    let entry = ankh
      .get(&AnkhRetrievalKey::from_query(&query))
      .expect("entry present after replay");
    assert_eq!(
      entry.context_snapshot.get("canonical_form").unwrap(),
      "x^2 + 2*x*y + y^2"
    );
    assert_eq!(
      entry.supplied_parameters.get("equivalent_form").unwrap(),
      "(x+y)^2"
    );
    assert_eq!(entry.query_kind, "lookup-algebraic-equivalent");
  }

  /// **Zero-config Korean NL → ankh end-to-end.** Same as the
  /// previous test but the caller does NOT pre-set
  /// `operation_candidate`. The intent classifier + operation map
  /// + resolver chain auto-routes the utterance to the math
  /// transform purely from cue extraction.
  ///
  /// This is the *full* closure: 사용자가 한국어 한 줄 입력만 하면
  /// substrate 가 알아서 routing → ankh 까지 evidence 가 누적된다.
  /// The Jarvis-form behavior — operator types, pnix figures out
  /// which lane, no hand-config.
  #[test]
  fn zero_config_korean_math_utterance_routes_through_intent_to_ankh() {
    use super::super::fact_cue_registry::extract_fact_signals;
    use super::super::intent_recognition::{
      classify_intent_recognition, IntentVerdict, SynthesisIntentInput,
    };
    use super::super::operation_candidate_mapping::{
      classify_operation_candidates, OperationMappingVerdict,
    };
    use super::super::parameter_resolution::{
      resolve_parameters, ResolutionHeldKind, ResolutionInput, ResolutionVerdict,
    };

    let utterance = "x^2 + 2*x*y + y^2 는 뭐야?";

    // Step 1: cue extraction — fact:math-question fires structurally.
    let cues = extract_fact_signals(utterance, &[]);
    assert!(
      cues.iter().any(|c| c == "fact:math-question"),
      "fact:math-question must fire, got {cues:?}"
    );

    // Step 2: intent recognition → explain wins.
    let intent_verdict = classify_intent_recognition(&SynthesisIntentInput {
      utterance: utterance.to_string(),
      fired_signals: cues.clone(),
      ..Default::default()
    });
    let top_intent = match intent_verdict {
      IntentVerdict::IntentRecognitionReady { ranked_intents } => ranked_intents
        .first()
        .expect("at least one ranked intent")
        .intent
        .clone(),
      other => panic!("expected IntentRecognitionReady, got {other:?}"),
    };
    assert_eq!(
      top_intent, "explain",
      "math-question must route to explain intent"
    );

    // Step 3: intent + cues → operation candidate.
    let op_verdict = classify_operation_candidates(&top_intent, &cues);
    let top_transform = match op_verdict {
      OperationMappingVerdict::OperationMappingReady { ranked_operations } => ranked_operations
        .first()
        .expect("at least one operation")
        .transform
        .clone(),
      other => panic!("expected OperationMappingReady, got {other:?}"),
    };
    assert_eq!(top_transform, "lookup-algebraic-equivalent");

    // Step 4: resolver → Held(MissingAlgebraicEquivalent).
    let resolution = resolve_parameters(&ResolutionInput {
      operation_candidate: top_transform,
      utterance: utterance.to_string(),
      ..Default::default()
    });
    let canonical = match &resolution {
      ResolutionVerdict::ResolutionHeld {
        held_kind,
        partial_resolution,
        ..
      } => {
        assert_eq!(*held_kind, ResolutionHeldKind::MissingAlgebraicEquivalent);
        partial_resolution.get("canonical_form").unwrap().clone()
      }
      other => panic!("expected Held, got {other:?}"),
    };
    assert_eq!(canonical, "x^2 + 2*x*y + y^2");

    // Step 5: Held → retrieval query → ankh write.
    let query = super::super::held_to_query::build_query_from_held(&resolution)
      .expect("math Held projects to query");
    let mut ankh = InMemoryAnkhStore::new();
    let host = {
      let mut h = HostEvidence::new();
      h.insert("equivalent_form".to_string(), "(x+y)^2".to_string());
      h
    };
    let r = execute_retrieval_with_ankh(&query, &host, &mut ankh, &ctx());
    assert!(
      matches!(r, RetrievalResult::RetrievalReady { .. }),
      "host evidence must yield Ready, got {r:?}"
    );
    assert_eq!(ankh.len(), 1, "zero-config utterance writes to ankh");

    // Step 6: replay with empty host evidence → ankh hits.
    let r2 = execute_retrieval_with_ankh(&query, &HostEvidence::new(), &mut ankh, &ctx());
    match r2 {
      RetrievalResult::RetrievalReady {
        channel,
        supplied_parameters,
        ..
      } => {
        assert_eq!(channel, HeldQueryRecoveryChannel::AnkhRetrieval);
        assert_eq!(
          supplied_parameters.get("equivalent_form").unwrap(),
          "(x+y)^2"
        );
      }
      other => panic!("expected ankh hit, got {other:?}"),
    }
  }

  /// **Zero-config Korean chemistry NL → ankh end-to-end.**
  /// Parallel to the math zero-config test — substrate-sharing N=3
  /// proof at the *Korean NL → ankh* level. Operator types a
  /// chemistry question, no `operation_candidate` set, no intent
  /// hint. Cue extractor + intent classifier + operation map all
  /// auto-route to chemistry transform.
  #[test]
  fn zero_config_korean_chemistry_utterance_routes_through_intent_to_ankh() {
    use super::super::fact_cue_registry::extract_fact_signals;
    use super::super::intent_recognition::{
      classify_intent_recognition, IntentVerdict, SynthesisIntentInput,
    };
    use super::super::operation_candidate_mapping::{
      classify_operation_candidates, OperationMappingVerdict,
    };
    use super::super::parameter_resolution::{
      resolve_parameters, ResolutionHeldKind, ResolutionInput, ResolutionVerdict,
    };

    let utterance = "2 H2 + O2 가 어떻게 반응해?";

    // Step 1: cue extraction — fact:chemistry-question fires.
    let cues = extract_fact_signals(utterance, &[]);
    assert!(
      cues.iter().any(|c| c == "fact:chemistry-question"),
      "fact:chemistry-question must fire, got {cues:?}"
    );

    // Step 2: intent → explain (chemistry shares math route).
    let intent_v = classify_intent_recognition(&SynthesisIntentInput {
      utterance: utterance.to_string(),
      fired_signals: cues.clone(),
      ..Default::default()
    });
    let top_intent = match intent_v {
      IntentVerdict::IntentRecognitionReady { ranked_intents } => ranked_intents
        .first()
        .expect("at least one intent")
        .intent
        .clone(),
      other => panic!("expected Ready, got {other:?}"),
    };
    assert_eq!(top_intent, "explain");

    // Step 3: operation routing → lookup-chemical-reaction.
    let op_v = classify_operation_candidates(&top_intent, &cues);
    let top_transform = match op_v {
      OperationMappingVerdict::OperationMappingReady { ranked_operations } => ranked_operations
        .first()
        .expect("at least one operation")
        .transform
        .clone(),
      other => panic!("expected OperationMappingReady, got {other:?}"),
    };
    assert_eq!(top_transform, "lookup-chemical-reaction");

    // Step 4: resolver → Held(MissingChemistryProducts).
    let resolution = resolve_parameters(&ResolutionInput {
      operation_candidate: top_transform,
      utterance: utterance.to_string(),
      ..Default::default()
    });
    let reactants = match &resolution {
      ResolutionVerdict::ResolutionHeld {
        held_kind,
        partial_resolution,
        ..
      } => {
        assert_eq!(*held_kind, ResolutionHeldKind::MissingChemistryProducts);
        partial_resolution.get("reactants").unwrap().clone()
      }
      other => panic!("expected Held, got {other:?}"),
    };
    assert_eq!(reactants, "2 H2 + O2");

    // Step 5: Held → retrieval query → ankh write.
    let query = super::super::held_to_query::build_query_from_held(&resolution)
      .expect("chemistry Held projects to query");
    assert_eq!(query.query_kind, "lookup-chemical-reaction");
    let mut ankh = InMemoryAnkhStore::new();
    let host = {
      let mut h = HostEvidence::new();
      h.insert("products".to_string(), "2 H2O".to_string());
      h
    };
    let r = execute_retrieval_with_ankh(&query, &host, &mut ankh, &ctx());
    assert!(
      matches!(r, RetrievalResult::RetrievalReady { .. }),
      "host evidence yields Ready, got {r:?}"
    );
    assert_eq!(ankh.len(), 1, "zero-config chemistry utterance writes ankh");

    // Step 6: replay → ankh hit.
    let r2 = execute_retrieval_with_ankh(&query, &HostEvidence::new(), &mut ankh, &ctx());
    match r2 {
      RetrievalResult::RetrievalReady {
        channel,
        supplied_parameters,
        ..
      } => {
        assert_eq!(channel, HeldQueryRecoveryChannel::AnkhRetrieval);
        assert_eq!(supplied_parameters.get("products").unwrap(), "2 H2O");
      }
      other => panic!("expected ankh hit, got {other:?}"),
    }
  }

  /// Same canonical, *different* equivalents (two operators
  /// disagree) → ankh stores both separately → propose emits two
  /// distinct candidate proposals. pnix does not silently merge
  /// disagreement. Substrate honors held/conflict shape uniformly.
  #[test]
  fn math_disagreement_does_not_silently_collapse() {
    let mut ankh = InMemoryAnkhStore::new();
    // Two say (x+y)^2
    for path in &["math/a.md", "math/b.md"] {
      let v = math_held_verdict("x^2 + 2*x*y + y^2", "polynomial", path);
      let q = super::super::held_to_query::build_query_from_held(&v).unwrap();
      let _ = execute_retrieval_with_ankh(&q, &math_host_evidence("(x+y)^2"), &mut ankh, &ctx());
    }
    // Two say (y+x)^2
    for path in &["math/c.md", "math/d.md"] {
      let v = math_held_verdict("x^2 + 2*x*y + y^2", "polynomial", path);
      let q = super::super::held_to_query::build_query_from_held(&v).unwrap();
      let _ = execute_retrieval_with_ankh(&q, &math_host_evidence("(y+x)^2"), &mut ankh, &ctx());
    }
    let proposals = propose_candidates_from_ankh(&ankh);
    let math: Vec<_> = proposals
      .iter()
      .filter(|p| p.candidate_kind == CandidateKind::MathExpressionLower)
      .collect();
    assert_eq!(
      math.len(),
      2,
      "two distinct equivalents → two proposals (no silent collapse)"
    );
  }
}
