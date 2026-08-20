//! Regression-proof gate — Stage D-v3 of the evolution lane
//! (firewall gate #4).
//!
//! OWNER-LAW (2026-05-12): mirror of
//! `stdlib/lib/gate/algorithm-synthesis/regression-proof-gate.px`.
//! Consumes an `AxisSeparatedCandidate` (gate-3 output) and checks
//! that adding the proposed row would not collide with an
//! existing row's uniqueness-key value.
//!
//! v0 does uniqueness-key conflict checking only. Test-corpus
//! replay (`cargo test` before/after the hypothetical row) is
//! deferred to v3.1.

use serde::{Deserialize, Serialize};
use pnix_hash::{Digest, Sha256};
use std::collections::BTreeMap;

use super::axis_separation_gate::{
  extract_string_values_from_folded_ast_json, AxisSeparatedCandidate, AxisSeparationOutcome,
};
use super::candidate_row_proposal::GateStatus;

/// Possible regression-proof outcomes. Stays byte-identical to
/// `.px` `validRegressionOutcomes`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RegressionOutcome {
  RegressionProven,
  HeldConflictWithExistingRow,
  HeldAxisNotVerified,
  HeldNoUniquenessPolicy,
}

impl RegressionOutcome {
  pub const ALL: &'static [Self] = &[
    Self::RegressionProven,
    Self::HeldConflictWithExistingRow,
    Self::HeldAxisNotVerified,
    Self::HeldNoUniquenessPolicy,
  ];

  pub fn as_str(self) -> &'static str {
    match self {
      Self::RegressionProven => "regression-proven",
      Self::HeldConflictWithExistingRow => "held-conflict-with-existing-row",
      Self::HeldAxisNotVerified => "held-axis-not-verified",
      Self::HeldNoUniquenessPolicy => "held-no-uniqueness-policy",
    }
  }
}

/// Uniqueness policy for a target table. Mirror of `.px`
/// `targetTableUniquenessKeys` row.
#[derive(Debug, Clone, Copy)]
pub struct TargetTableUniqueness {
  pub target_owner: &'static str,
  pub target_table: &'static str,
  pub uniqueness_keys: &'static [&'static str],
}

pub const TARGET_TABLE_UNIQUENESS_KEYS: &[TargetTableUniqueness] = &[
  TargetTableUniqueness {
    target_owner: "stdlib/lib/gate/algorithm-synthesis/held-to-query.px",
    target_table: "heldRoutingMap",
    uniqueness_keys: &["held"],
  },
  TargetTableUniqueness {
    target_owner: "stdlib/lib/gate/algorithm-synthesis/fact-cue-registry.px",
    target_table: "factPhrasePatterns",
    uniqueness_keys: &["cue"],
  },
  TargetTableUniqueness {
    target_owner: "stdlib/lib/gate/algorithm-synthesis/operation-candidate-mapping.px",
    target_table: "operationMap",
    uniqueness_keys: &["intent", "cues", "transform"],
  },
  TargetTableUniqueness {
    target_owner: "stdlib/lib/gate/known-imports-by-language.px",
    target_table: "knownImportsByLanguage",
    uniqueness_keys: &["language", "import_spec"],
  },
  TargetTableUniqueness {
    target_owner: "stdlib/lib/gate/known-algebraic-identities.px",
    target_table: "knownAlgebraicIdentities",
    uniqueness_keys: &["canonical_form", "equivalent_form", "language"],
  },
  TargetTableUniqueness {
    target_owner: "stdlib/lib/gate/known-chemical-reactions.px",
    target_table: "knownChemicalReactions",
    uniqueness_keys: &["reactants", "products", "conditions"],
  },
  // Learned-intent-overlay: user-authored direct-injection lane.
  // Uniqueness on `cue` (one cue maps to one intent; weight is
  // updated via supersede chain, not regression-proof).
  TargetTableUniqueness {
    target_owner: "stdlib/lib/gate/algorithm-synthesis/learned-intent-overlay.px",
    target_table: "overlayIntentSignals",
    uniqueness_keys: &["cue"],
  },
  // Learned-operation-overlay: same uniqueness as the static
  // operationMap — (intent, cues, transform) triple.
  TargetTableUniqueness {
    target_owner: "stdlib/lib/gate/algorithm-synthesis/learned-operation-overlay.px",
    target_table: "overlayOperationRows",
    uniqueness_keys: &["intent", "cues", "transform"],
  },
  // Learned-parameter-overlay: uniqueness on operation_candidate.
  TargetTableUniqueness {
    target_owner: "stdlib/lib/gate/algorithm-synthesis/learned-parameter-overlay.px",
    target_table: "overlayParameterRows",
    uniqueness_keys: &["operation_candidate"],
  },
  // Learned-fact-cue-overlay: uniqueness on cue.
  TargetTableUniqueness {
    target_owner: "stdlib/lib/gate/algorithm-synthesis/learned-fact-cue-overlay.px",
    target_table: "overlayPhrasePatterns",
    uniqueness_keys: &["cue"],
  },
];

fn uniqueness_for(
  target_owner: &str,
  target_table: &str,
) -> Option<&'static TargetTableUniqueness> {
  TARGET_TABLE_UNIQUENESS_KEYS
    .iter()
    .find(|u| u.target_owner == target_owner && u.target_table == target_table)
}

/// Parse a folded `.px` attrset text into `(key, value)` pairs.
/// Same invariants as `extract_keys_from_folded_text` — lines look
/// like `  key = "value";` after `macro-fold-gate`'s
/// `multi-line-attrset` format.
///
/// Retained as a compatibility/debug fallback. Gate 4's main path
/// reads Gate 2's `folded_ast_json` via
/// `extract_string_values_from_folded_ast_json`.
pub fn extract_kv_from_folded_text(folded: &str) -> BTreeMap<String, String> {
  let mut out = BTreeMap::new();
  for line in folded.lines() {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed == "{" || trimmed == "}" {
      continue;
    }
    let Some(eq_idx) = trimmed.find(" = ") else {
      continue;
    };
    let key = trimmed[..eq_idx].trim().to_string();
    let after_eq = &trimmed[eq_idx + " = ".len()..];
    // Strip trailing `;` then surrounding `"`.
    let after_eq = after_eq.trim_end_matches(';').trim();
    let Some(unquoted) = after_eq.strip_prefix('"').and_then(|s| s.strip_suffix('"')) else {
      continue;
    };
    // Unescape `\"` and `\\` — inverse of `macro_fold_gate::escape_nix_string`.
    let mut unescaped = String::with_capacity(unquoted.len());
    let mut chars = unquoted.chars().peekable();
    while let Some(c) = chars.next() {
      if c == '\\' {
        if let Some(next) = chars.next() {
          unescaped.push(next);
        }
      } else {
        unescaped.push(c);
      }
    }
    out.insert(key, unescaped);
  }
  out
}

/// The output of the gate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegressionProvenCandidate {
  pub source: AxisSeparatedCandidate,
  pub outcome: RegressionOutcome,
  pub gate_status: GateStatus,
  /// On `HeldConflictWithExistingRow`: the uniqueness-key value
  /// triple that collided with an existing row. Empty otherwise.
  pub conflicting_uniqueness_value: BTreeMap<String, String>,
  /// On `HeldConflictWithExistingRow`: an opaque identifier for
  /// the existing row that collides (caller-supplied — typically
  /// the row's index in the existing-rows slice, or a fingerprint).
  pub conflicting_existing_row_ref: Option<String>,
  pub reason: String,
}

/// Check that adding `candidate` to its target table would not
/// collide with any existing row's uniqueness-key value.
///
/// `existing_rows` is the caller-supplied snapshot of the current
/// target table (each row a `(key → value)` map). v0 keeps this
/// caller-provided so the gate has no I/O — the caller can mock,
/// parse the live `.px`, or read a doghouse snapshot.
///
/// **OWNER-LAW (v0.6.6, 2026-05-15)**: this pure-Rust body is a
/// **byte-equivalent mirror** of the `.px` owner
/// `stdlib/lib/gate/algorithm-synthesis/regression-proof-gate.px::checkRegressionProofConflict`
/// (v0.6.4 migration). The `.px` is the **canonical owner**; this
/// Rust body exists as a cycle-free in-process fast-path because
/// `pnix-core` cannot depend on `pnix-eval`.
///
/// The byte-equivalent `.px`-delegating entry is
/// `doghouse_core::algorithm_synthesis_bridge::check_regression_proof_via_px_body`.
/// Drift between the two paths is prevented by the byte-equivalence
/// ratchet test
/// `crates/doghouse-core/tests/check_regression_proof_via_px_body_byte_equivalence.rs`
/// and the full-pipeline test
/// `crates/doghouse-core/tests/firewall_pipeline_dual_path_equivalence.rs`.
pub fn check_regression_proof(
  candidate: &AxisSeparatedCandidate,
  existing_rows: &[BTreeMap<String, String>],
) -> RegressionProvenCandidate {
  // Propagate gate-3 hold — regression-proof is meaningless on a
  // row whose schema didn't verify.
  if candidate.outcome != AxisSeparationOutcome::AxisVerified {
    return RegressionProvenCandidate {
      source: candidate.clone(),
      outcome: RegressionOutcome::HeldAxisNotVerified,
      gate_status: GateStatus::Held,
      conflicting_uniqueness_value: BTreeMap::new(),
      conflicting_existing_row_ref: None,
      reason: format!(
        "axis-separation gate produced `{}` — regression-proof cannot proceed without an axis-verified row",
        candidate.outcome.as_str()
      ),
    };
  }

  let target_owner = candidate.source.source.target_owner.as_str();
  let target_table = candidate.source.source.target_table.as_str();
  let Some(policy) = uniqueness_for(target_owner, target_table) else {
    return RegressionProvenCandidate {
      source: candidate.clone(),
      outcome: RegressionOutcome::HeldNoUniquenessPolicy,
      gate_status: GateStatus::Held,
      conflicting_uniqueness_value: BTreeMap::new(),
      conflicting_existing_row_ref: None,
      reason: format!(
        "no uniqueness policy registered for ({target_owner}, {target_table}) — register one in `targetTableUniquenessKeys` first"
      ),
    };
  };

  // Extract the candidate's uniqueness-key values from Gate 2's
  // AST projection. Fall back to text only for legacy/manually
  // constructed candidates that lack `folded_ast_json`.
  let candidate_kv = candidate
    .source
    .folded_ast_json
    .as_ref()
    .map(extract_string_values_from_folded_ast_json)
    .unwrap_or_else(|| extract_kv_from_folded_text(&candidate.source.folded_source_text));
  let candidate_uniqueness: BTreeMap<String, String> = policy
    .uniqueness_keys
    .iter()
    .filter_map(|k| candidate_kv.get(*k).map(|v| (k.to_string(), v.clone())))
    .collect();

  // The candidate must have ALL uniqueness keys (axis-separation
  // would have caught missing ones, but this is defense in depth).
  if candidate_uniqueness.len() != policy.uniqueness_keys.len() {
    return RegressionProvenCandidate {
      source: candidate.clone(),
      outcome: RegressionOutcome::HeldNoUniquenessPolicy,
      gate_status: GateStatus::Held,
      conflicting_uniqueness_value: candidate_uniqueness,
      conflicting_existing_row_ref: None,
      reason: format!(
        "candidate row is missing one or more uniqueness keys {:?}",
        policy.uniqueness_keys
      ),
    };
  }

  // Scan existing rows for any whose uniqueness-key tuple matches.
  for (idx, existing) in existing_rows.iter().enumerate() {
    let same = policy
      .uniqueness_keys
      .iter()
      .all(|k| existing.get(*k) == candidate_uniqueness.get(*k));
    if same {
      return RegressionProvenCandidate {
        source: candidate.clone(),
        outcome: RegressionOutcome::HeldConflictWithExistingRow,
        gate_status: GateStatus::Held,
        conflicting_uniqueness_value: candidate_uniqueness,
        conflicting_existing_row_ref: Some(format!("existing-row-index:{idx}")),
        reason: format!(
          "existing row at index {idx} already has uniqueness-key value {:?} — adding this candidate would create a duplicate",
          policy.uniqueness_keys
        ),
      };
    }
  }

  RegressionProvenCandidate {
    source: candidate.clone(),
    outcome: RegressionOutcome::RegressionProven,
    gate_status: GateStatus::RegressionProofAttempted,
    conflicting_uniqueness_value: BTreeMap::new(),
    conflicting_existing_row_ref: None,
    reason: format!(
      "no existing row in `{target_table}` shares the candidate's uniqueness-key tuple {:?}",
      candidate_uniqueness
    ),
  }
}

/// Batch.
pub fn check_all(
  candidates: &[AxisSeparatedCandidate],
  existing_rows: &[BTreeMap<String, String>],
) -> Vec<RegressionProvenCandidate> {
  candidates
    .iter()
    .map(|c| check_regression_proof(c, existing_rows))
    .collect()
}

/// Render a `RegressionProvenCandidate` as the canonical JSON
/// payload of a `coding.regression-proven-candidate` artifact.
/// Stage D-4 (Gate 4) cockpit surface — operator sees the
/// uniqueness-key policy, the candidate's uniqueness value, and
/// (on conflict) which existing row collides.
///
/// Single family for all 5 outcomes (`regression-proven` /
/// `held-axis-not-verified` / `held-no-uniqueness-policy` /
/// `held-conflict-with-existing-row` / `held-empty-uniqueness-key`).
/// On conflict, carries `conflicting_uniqueness_value` (the value
/// triple that collided) + `conflicting_existing_row_ref` (opaque
/// id for the existing row) so operator can supersede or rewrite.
///
/// Replay-stable id = SHA-256 of intrinsic identity (outcome +
/// target_owner + target_table + sorted uniqueness_policy +
/// sorted conflicting_uniqueness_value + conflicting_existing_row_ref).
/// `stored_at_ms` is extrinsic.
///
/// Content policy: metadata only (key names + value pairs +
/// opaque refs). No source bodies. Customer-release safe.
pub fn build_regression_proven_candidate_artifact(
  regression: &RegressionProvenCandidate,
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
) -> serde_json::Value {
  let folded = &regression.source.source;
  let proposal = &folded.source;
  let target_owner = proposal.target_owner.as_str();
  let target_table = proposal.target_table.as_str();

  // Look up uniqueness policy for cockpit display. Empty for
  // held-axis-not-verified (upstream propagation) when no policy
  // is registered for the target.
  let uniqueness_policy: Vec<String> = match uniqueness_for(target_owner, target_table) {
    Some(u) => u.uniqueness_keys.iter().map(|k| k.to_string()).collect(),
    None => Vec::new(),
  };

  let mut h = Sha256::new();
  h.update(b"regression-proven-candidate\x1f");
  h.update(regression.outcome.as_str().as_bytes());
  h.update(b"\x1f");
  h.update(target_owner.as_bytes());
  h.update(b"\x1e");
  h.update(target_table.as_bytes());
  h.update(b"\x1f");
  let mut sorted_policy = uniqueness_policy.clone();
  sorted_policy.sort();
  for k in &sorted_policy {
    h.update(k.as_bytes());
    h.update(b"\x1e");
  }
  h.update(b"\x1f");
  let mut conflict_keys: Vec<&String> = regression.conflicting_uniqueness_value.keys().collect();
  conflict_keys.sort();
  for k in conflict_keys {
    h.update(k.as_bytes());
    h.update(b"\x1d");
    h.update(regression.conflicting_uniqueness_value[k].as_bytes());
    h.update(b"\x1e");
  }
  h.update(b"\x1f");
  if let Some(ref r) = regression.conflicting_existing_row_ref {
    h.update(r.as_bytes());
  }
  let digest = h.finalize();
  let prefix = digest
    .iter()
    .take(16)
    .map(|b| format!("{b:02x}"))
    .collect::<String>();
  let id = format!("regression-proven-candidate.{prefix}");

  let mut payload = serde_json::json!({
    "id": id,
    "artifact_family": "coding.regression-proven-candidate",
    "source_surface": "algorithm-synthesis.regression-proof-gate",
    "stored_at_ms": stored_at_ms,
    "outcome": regression.outcome.as_str(),
    "gate_status": regression.gate_status.as_str(),
    "candidate_kind": proposal.candidate_kind.as_str(),
    "target_owner": target_owner,
    "target_table": target_table,
    "uniqueness_policy": uniqueness_policy,
    "conflicting_uniqueness_value": regression.conflicting_uniqueness_value,
    "reason": regression.reason,
    "related_refs": serde_json::json!([
      format!("candidate-kind:{}", proposal.candidate_kind.as_str()),
      format!("target-owner:{}", target_owner),
      format!("target-table:{}", target_table),
      "owner-law:stdlib/lib/gate/algorithm-synthesis/regression-proof-gate.px",
    ]),
    "target_paths": serde_json::json!([target_owner]),
    "command_refs": Vec::<String>::new(),
  });
  if let Some(ref r) = regression.conflicting_existing_row_ref {
    payload["conflicting_existing_row_ref"] = serde_json::Value::String(r.clone());
  }
  if let Some(snap) = repo_snapshot_ref {
    payload["repo_snapshot_ref"] = serde_json::Value::String(snap.to_string());
  }
  payload
}

#[cfg(test)]
mod tests {
  use super::super::axis_separation_gate::check_axis_separation;
  use super::super::candidate_row_proposal::{CandidateKind, CandidateRowProposal, GateStatus};
  use super::super::macro_fold_gate::fold_proposal;
  use super::*;

  fn proposal(
    kind: CandidateKind,
    target_owner: &str,
    target_table: &str,
    row: &[(&str, &str)],
  ) -> CandidateRowProposal {
    let mut proposed = BTreeMap::new();
    for (k, v) in row {
      proposed.insert(k.to_string(), v.to_string());
    }
    CandidateRowProposal {
      candidate_kind: kind,
      target_owner: target_owner.to_string(),
      target_table: target_table.to_string(),
      proposed_row: proposed,
      supporting_evidence: vec!["evidence-1".to_string()],
      evidence_count: 2,
      gate_status: GateStatus::IntentReceiptOnly,
      reason: "test".to_string(),
    }
  }

  fn schema_aware_held_routing_candidate(
    held: &str,
    primary: &str,
    fallback: Option<&str>,
  ) -> AxisSeparatedCandidate {
    let mut row = BTreeMap::new();
    row.insert("held".to_string(), held.to_string());
    row.insert("primary".to_string(), primary.to_string());
    if let Some(f) = fallback {
      row.insert("fallback".to_string(), f.to_string());
    }
    let proposal = CandidateRowProposal {
      candidate_kind: CandidateKind::RecurringChannelSuccess,
      target_owner: "stdlib/lib/gate/algorithm-synthesis/held-to-query.px".to_string(),
      target_table: "heldRoutingMap".to_string(),
      proposed_row: row,
      supporting_evidence: vec!["test".to_string()],
      evidence_count: 2,
      gate_status: GateStatus::IntentReceiptOnly,
      reason: "fixture".to_string(),
    };
    check_axis_separation(&fold_proposal(&proposal))
  }

  fn existing_row(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
      .iter()
      .map(|(k, v)| (k.to_string(), v.to_string()))
      .collect()
  }

  // ─── registry consistency ──────────────────────────────────────

  #[test]
  fn every_outcome_has_a_string_form() {
    for o in RegressionOutcome::ALL {
      assert!(!o.as_str().is_empty());
    }
  }

  #[test]
  fn uniqueness_table_has_no_duplicate_target_pairs() {
    let mut seen = std::collections::HashSet::new();
    for u in TARGET_TABLE_UNIQUENESS_KEYS {
      let key = (u.target_owner, u.target_table);
      assert!(seen.insert(key), "duplicate uniqueness entry: {key:?}");
    }
  }

  // ─── propagation from axis-separation ──────────────────────────

  #[test]
  fn axis_not_verified_propagates_as_held() {
    use super::super::axis_separation_gate::AxisSeparationOutcome;
    // Hand-build an axis-held candidate.
    let proposal = CandidateRowProposal {
      candidate_kind: CandidateKind::RecurringImportSpec,
      target_owner: "stdlib/lib/gate/algorithm-synthesis/fact-cue-registry.px".to_string(),
      target_table: "factPhrasePatterns".to_string(),
      proposed_row: {
        let mut r = BTreeMap::new();
        r.insert("import_spec".to_string(), "import os".to_string());
        r
      },
      supporting_evidence: vec![],
      evidence_count: 2,
      gate_status: GateStatus::IntentReceiptOnly,
      reason: "fixture".to_string(),
    };
    let axis = check_axis_separation(&fold_proposal(&proposal));
    assert_eq!(axis.outcome, AxisSeparationOutcome::HeldMissingKeys);
    let r = check_regression_proof(&axis, &[]);
    assert_eq!(r.outcome, RegressionOutcome::HeldAxisNotVerified);
    assert_eq!(r.gate_status, GateStatus::Held);
  }

  // ─── conflict detection ────────────────────────────────────────

  #[test]
  fn candidate_with_same_uniqueness_value_conflicts() {
    let candidate = schema_aware_held_routing_candidate(
      "missing-import-spec",
      "host-symbol-resolver",
      Some("external-knowledge-search"),
    );
    let existing = vec![existing_row(&[
      ("held", "missing-import-spec"),
      ("primary", "host-symbol-resolver"),
      ("fallback", "external-knowledge-search"),
    ])];
    let r = check_regression_proof(&candidate, &existing);
    assert_eq!(r.outcome, RegressionOutcome::HeldConflictWithExistingRow);
    assert_eq!(r.gate_status, GateStatus::Held);
    assert_eq!(
      r.conflicting_uniqueness_value.get("held").unwrap(),
      "missing-import-spec"
    );
    assert!(r.conflicting_existing_row_ref.is_some());
  }

  #[test]
  fn candidate_with_new_uniqueness_value_passes() {
    let candidate =
      schema_aware_held_routing_candidate("new-fictional-held-kind", "host-symbol-resolver", None);
    let existing = vec![existing_row(&[
      ("held", "missing-import-spec"),
      ("primary", "host-symbol-resolver"),
    ])];
    let r = check_regression_proof(&candidate, &existing);
    assert_eq!(r.outcome, RegressionOutcome::RegressionProven);
    assert_eq!(r.gate_status, GateStatus::RegressionProofAttempted);
    assert!(r.conflicting_existing_row_ref.is_none());
  }

  #[test]
  fn candidate_with_no_existing_rows_always_passes() {
    let candidate =
      schema_aware_held_routing_candidate("first-ever-held-kind", "host-symbol-resolver", None);
    let r = check_regression_proof(&candidate, &[]);
    assert_eq!(r.outcome, RegressionOutcome::RegressionProven);
  }

  #[test]
  fn regression_proof_reads_uniqueness_values_from_folded_ast_json() {
    use super::super::axis_separation_gate::AxisSeparationOutcome;

    let proposal = proposal(
      CandidateKind::RecurringImportSpec,
      "stdlib/lib/gate/known-imports-by-language.px",
      "knownImportsByLanguage",
      &[("language", "python"), ("import_spec", "import os")],
    );
    let mut folded = fold_proposal(&proposal);
    assert!(
      folded.folded_ast_json.is_some(),
      "Gate 2 must emit AST evidence"
    );
    folded.folded_source_text = "{\n}".to_string();

    let axis = check_axis_separation(&folded);
    assert_eq!(axis.outcome, AxisSeparationOutcome::AxisVerified);
    let r = check_regression_proof(&axis, &[]);
    assert_eq!(r.outcome, RegressionOutcome::RegressionProven);
  }

  // ─── multi-key uniqueness (operationMap) ───────────────────────

  #[test]
  fn operation_map_uniqueness_compares_full_triple() {
    use super::super::axis_separation_gate::AxisSeparationOutcome;
    // Hand-build an operationMap-shaped candidate row.
    let mut row = BTreeMap::new();
    row.insert("intent".to_string(), "fix-bug".to_string());
    row.insert("cues".to_string(), "fact:missing-import".to_string());
    row.insert("transform".to_string(), "add-import".to_string());
    row.insert("weight".to_string(), "0.95".to_string());
    let proposal = CandidateRowProposal {
      candidate_kind: CandidateKind::RecurringChannelSuccess,
      target_owner: "stdlib/lib/gate/algorithm-synthesis/operation-candidate-mapping.px"
        .to_string(),
      target_table: "operationMap".to_string(),
      proposed_row: row,
      supporting_evidence: vec![],
      evidence_count: 2,
      gate_status: GateStatus::IntentReceiptOnly,
      reason: "fixture".to_string(),
    };
    let axis = check_axis_separation(&fold_proposal(&proposal));
    assert_eq!(axis.outcome, AxisSeparationOutcome::AxisVerified);
    // existing rows with DIFFERENT cues or DIFFERENT transform must
    // NOT collide.
    let existing = vec![
      existing_row(&[
        ("intent", "fix-bug"),
        ("cues", "fact:missing-import"),
        ("transform", "rename-symbol"), // different transform
        ("weight", "0.40"),
      ]),
      existing_row(&[
        ("intent", "fix-bug"),
        ("cues", "metaphor:brokenness"), // different cues
        ("transform", "add-import"),
        ("weight", "0.40"),
      ]),
    ];
    let r = check_regression_proof(&axis, &existing);
    assert_eq!(r.outcome, RegressionOutcome::RegressionProven);
  }

  #[test]
  fn operation_map_full_triple_match_conflicts() {
    use super::super::axis_separation_gate::AxisSeparationOutcome;
    let mut row = BTreeMap::new();
    row.insert("intent".to_string(), "fix-bug".to_string());
    row.insert("cues".to_string(), "fact:missing-import".to_string());
    row.insert("transform".to_string(), "add-import".to_string());
    row.insert("weight".to_string(), "0.99".to_string());
    let proposal = CandidateRowProposal {
      candidate_kind: CandidateKind::RecurringChannelSuccess,
      target_owner: "stdlib/lib/gate/algorithm-synthesis/operation-candidate-mapping.px"
        .to_string(),
      target_table: "operationMap".to_string(),
      proposed_row: row,
      supporting_evidence: vec![],
      evidence_count: 2,
      gate_status: GateStatus::IntentReceiptOnly,
      reason: "fixture".to_string(),
    };
    let axis = check_axis_separation(&fold_proposal(&proposal));
    assert_eq!(axis.outcome, AxisSeparationOutcome::AxisVerified);
    let existing = vec![existing_row(&[
      ("intent", "fix-bug"),
      ("cues", "fact:missing-import"),
      ("transform", "add-import"),
      ("weight", "0.95"), // different weight — but weight is NOT in uniqueness keys
    ])];
    let r = check_regression_proof(&axis, &existing);
    assert_eq!(r.outcome, RegressionOutcome::HeldConflictWithExistingRow);
  }

  // ─── kv extraction ─────────────────────────────────────────────

  #[test]
  fn kv_extraction_handles_escaped_quote() {
    let text = "{\n  k = \"value with \\\"quote\\\"\";\n}";
    let kv = extract_kv_from_folded_text(text);
    assert_eq!(kv.get("k").unwrap(), "value with \"quote\"");
  }

  #[test]
  fn kv_extraction_handles_escaped_backslash() {
    let text = "{\n  path = \"C:\\\\Foo\";\n}";
    let kv = extract_kv_from_folded_text(text);
    assert_eq!(kv.get("path").unwrap(), "C:\\Foo");
  }

  #[test]
  fn kv_extraction_preserves_korean() {
    let text = "{\n  msg = \"이 함수\";\n}";
    let kv = extract_kv_from_folded_text(text);
    assert_eq!(kv.get("msg").unwrap(), "이 함수");
  }

  // ─── regression-proven-candidate artifact (Stage D-4 panel) ──

  fn proven_known_imports() -> RegressionProvenCandidate {
    let p = proposal(
      CandidateKind::RecurringImportSpec,
      "stdlib/lib/gate/known-imports-by-language.px",
      "knownImportsByLanguage",
      &[("language", "python"), ("import_spec", "import os")],
    );
    let f = fold_proposal(&p);
    let s = check_axis_separation(&f);
    check_regression_proof(&s, &[])
  }

  fn conflicting_known_imports() -> RegressionProvenCandidate {
    let p = proposal(
      CandidateKind::RecurringImportSpec,
      "stdlib/lib/gate/known-imports-by-language.px",
      "knownImportsByLanguage",
      &[("language", "python"), ("import_spec", "import os")],
    );
    let f = fold_proposal(&p);
    let s = check_axis_separation(&f);
    // Existing row with same (language, import_spec) → collision.
    let mut existing = BTreeMap::new();
    existing.insert("language".to_string(), "python".to_string());
    existing.insert("import_spec".to_string(), "import os".to_string());
    check_regression_proof(&s, std::slice::from_ref(&existing))
  }

  #[test]
  fn artifact_proven_carries_uniqueness_policy_and_empty_conflict() {
    let r = proven_known_imports();
    assert_eq!(r.outcome, RegressionOutcome::RegressionProven);
    let art = build_regression_proven_candidate_artifact(&r, 1700000000000, None);
    assert_eq!(art["artifact_family"], "coding.regression-proven-candidate");
    assert_eq!(art["outcome"], "regression-proven");
    let policy: Vec<String> = serde_json::from_value(art["uniqueness_policy"].clone()).unwrap();
    assert!(policy.iter().any(|k| k == "language"));
    assert!(policy.iter().any(|k| k == "import_spec"));
    let conflict = art["conflicting_uniqueness_value"].as_object().unwrap();
    assert!(conflict.is_empty(), "no conflict on proven");
    assert!(art.get("conflicting_existing_row_ref").is_none());
  }

  #[test]
  fn artifact_conflict_carries_collision_value_and_existing_ref() {
    let r = conflicting_known_imports();
    assert_eq!(r.outcome, RegressionOutcome::HeldConflictWithExistingRow);
    let art = build_regression_proven_candidate_artifact(&r, 0, None);
    assert_eq!(art["outcome"], "held-conflict-with-existing-row");
    let conflict = art["conflicting_uniqueness_value"].as_object().unwrap();
    assert_eq!(conflict.get("language").unwrap(), "python");
    assert_eq!(conflict.get("import_spec").unwrap(), "import os");
    // Existing row ref populated (caller convention: row index).
    assert!(art.get("conflicting_existing_row_ref").is_some());
  }

  #[test]
  fn artifact_held_axis_not_verified_propagates_with_empty_policy_listing() {
    // Build a candidate that holds at Gate 3 (missing key).
    let p = proposal(
      CandidateKind::RecurringImportSpec,
      "stdlib/lib/gate/known-imports-by-language.px",
      "knownImportsByLanguage",
      &[("language", "python")], // import_spec missing
    );
    let f = fold_proposal(&p);
    let s = check_axis_separation(&f);
    let r = check_regression_proof(&s, &[]);
    assert_eq!(r.outcome, RegressionOutcome::HeldAxisNotVerified);
    let art = build_regression_proven_candidate_artifact(&r, 0, None);
    assert_eq!(art["outcome"], "held-axis-not-verified");
    // Policy lookup still succeeds (target registered) — operator
    // still sees what uniqueness *would* enforce if Gate 3 passes.
    let policy: Vec<String> = serde_json::from_value(art["uniqueness_policy"].clone()).unwrap();
    assert!(!policy.is_empty());
  }

  #[test]
  fn artifact_id_is_replay_stable_across_stored_at() {
    let r = proven_known_imports();
    let a1 = build_regression_proven_candidate_artifact(&r, 1, None);
    let a2 = build_regression_proven_candidate_artifact(&r, 999999, None);
    assert_eq!(a1["id"], a2["id"]);
  }

  #[test]
  fn artifact_id_differs_between_proven_and_conflict() {
    let p = proven_known_imports();
    let c = conflicting_known_imports();
    let a_p = build_regression_proven_candidate_artifact(&p, 0, None);
    let a_c = build_regression_proven_candidate_artifact(&c, 0, None);
    assert_ne!(a_p["id"], a_c["id"]);
  }

  #[test]
  fn artifact_math_lane_proven_carries_three_field_uniqueness() {
    let p = proposal(
      CandidateKind::MathExpressionLower,
      "stdlib/lib/gate/known-algebraic-identities.px",
      "knownAlgebraicIdentities",
      &[
        ("canonical_form", "x^2 + 2*x*y + y^2"),
        ("equivalent_form", "(x+y)^2"),
        ("language", "polynomial"),
      ],
    );
    let f = fold_proposal(&p);
    let s = check_axis_separation(&f);
    let r = check_regression_proof(&s, &[]);
    assert_eq!(r.outcome, RegressionOutcome::RegressionProven);
    let art = build_regression_proven_candidate_artifact(&r, 0, None);
    let policy: Vec<String> = serde_json::from_value(art["uniqueness_policy"].clone()).unwrap();
    assert!(policy.iter().any(|k| k == "canonical_form"));
    assert!(policy.iter().any(|k| k == "equivalent_form"));
    assert!(policy.iter().any(|k| k == "language"));
  }
}
