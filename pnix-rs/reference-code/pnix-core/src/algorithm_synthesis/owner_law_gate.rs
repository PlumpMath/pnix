//! Owner-law gate — Stage D-v4 of the evolution lane (firewall
//! gate #5, the final one).
//!
//! OWNER-LAW (2026-05-12): mirror of
//! `stdlib/lib/gate/algorithm-synthesis/owner-law-gate.px`. The
//! only gate that can produce `GateStatus::Promoted`.
//!
//! Per the OWNER-LAW CONSTITUTION, promotion requires:
//!   - explicit `PromotionApproval` from a namespaced actor + tenant
//!     principal that satisfies owner-law identity-shape policy
//!   - fingerprint match between approval and candidate (TOCTOU)
//!   - optional TTL not yet expired
//!   - all 4 upstream gates passed
//!
//! All three approval decisions (Approve / Reject / HoldForMoreEvidence)
//! preserve an audit-ref in the resulting `OwnerLawProcessedCandidate`.
//! `Rejected` outcomes feed the supersede chain — they are evidence
//! the same shape should not be re-proposed without new evidence.

use serde::{Deserialize, Serialize};
use pnix_hash::{Digest, Sha256};

use super::candidate_row_proposal::GateStatus;
use super::regression_proof_gate::{RegressionOutcome, RegressionProvenCandidate};

/// Possible owner-law outcomes. Stays byte-identical to `.px`
/// `validOwnerLawOutcomes`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OwnerLawOutcome {
  Promoted,
  Rejected,
  HeldAwaitingApproval,
  HeldForMoreEvidence,
  HeldUpstreamGate,
  HeldStaleApproval,
  HeldExpiredApproval,
  HeldInvalidApprovalIdentity,
}

impl OwnerLawOutcome {
  pub const ALL: &'static [Self] = &[
    Self::Promoted,
    Self::Rejected,
    Self::HeldAwaitingApproval,
    Self::HeldForMoreEvidence,
    Self::HeldUpstreamGate,
    Self::HeldStaleApproval,
    Self::HeldExpiredApproval,
    Self::HeldInvalidApprovalIdentity,
  ];

  pub fn as_str(self) -> &'static str {
    match self {
      Self::Promoted => "promoted",
      Self::Rejected => "rejected",
      Self::HeldAwaitingApproval => "held-awaiting-approval",
      Self::HeldForMoreEvidence => "held-for-more-evidence",
      Self::HeldUpstreamGate => "held-upstream-gate",
      Self::HeldStaleApproval => "held-stale-approval",
      Self::HeldExpiredApproval => "held-expired-approval",
      Self::HeldInvalidApprovalIdentity => "held-invalid-approval-identity",
    }
  }
}

/// Approval identity-shape policy mirrored from
/// `owner-law-gate.px::approvalIdentityFieldPolicies`.
///
/// This is not external authentication. It is the deterministic
/// owner-law floor: Gate 5 refuses anonymous or unscoped principals
/// before it considers fingerprint/TTL/decision.
pub const APPROVAL_ACTOR_ID_VALID_PREFIXES: &[&str] = &["actor:", "actor."];
pub const APPROVAL_TENANT_ID_VALID_PREFIXES: &[&str] = &["tenant:", "tenant."];
pub const APPROVAL_IDENTITY_FORBIDDEN_SUBSTRINGS: &[&str] = &[" ", "\t", "\n", "\r", "/", "\\"];

fn principal_identity_error(field: &str, value: &str, prefixes: &[&str]) -> Option<String> {
  if value.is_empty() {
    return Some(format!("{field} must be non-empty"));
  }
  if value.trim() != value {
    return Some(format!("{field} must not have leading/trailing whitespace"));
  }
  for forbidden in APPROVAL_IDENTITY_FORBIDDEN_SUBSTRINGS {
    if value.contains(forbidden) {
      return Some(format!(
        "{field} contains forbidden substring {forbidden:?}"
      ));
    }
  }
  let Some(prefix) = prefixes.iter().find(|p| value.starts_with(**p)) else {
    return Some(format!(
      "{field} must start with one of [{}]",
      prefixes.join(", ")
    ));
  };
  if value.len() == prefix.len() {
    return Some(format!(
      "{field} must include a principal name after `{prefix}`"
    ));
  }
  if !value
    .chars()
    .all(|c| c.is_ascii_alphanumeric() || matches!(c, ':' | '.' | '_' | '-'))
  {
    return Some(format!(
      "{field} may contain only ASCII alphanumeric plus ':', '.', '_' and '-'"
    ));
  }
  None
}

/// Validate the deterministic owner-law identity-shape policy for an
/// approval. This is intentionally public so higher-level write tails
/// can preflight and render the same policy result the gate enforces.
pub fn validate_approval_identity(approval: &PromotionApproval) -> Result<(), String> {
  if let Some(e) = principal_identity_error(
    "actor_id",
    &approval.actor_id,
    APPROVAL_ACTOR_ID_VALID_PREFIXES,
  ) {
    return Err(e);
  }
  if let Some(e) = principal_identity_error(
    "tenant_id",
    &approval.tenant_id,
    APPROVAL_TENANT_ID_VALID_PREFIXES,
  ) {
    return Err(e);
  }
  Ok(())
}

/// Decision an explicit `PromotionApproval` may carry. Stays
/// byte-identical to `.px` `validApprovalDecisions`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PromotionApprovalDecision {
  Approve,
  Reject,
  HoldForMoreEvidence,
}

impl PromotionApprovalDecision {
  pub const ALL: &'static [Self] = &[Self::Approve, Self::Reject, Self::HoldForMoreEvidence];

  pub fn as_str(self) -> &'static str {
    match self {
      Self::Approve => "approve",
      Self::Reject => "reject",
      Self::HoldForMoreEvidence => "hold-for-more-evidence",
    }
  }
}

/// Explicit approval referencing a candidate by deterministic
/// fingerprint. All fields are required (non-Option) except
/// `ttl_ms` and `reason` per the `.px` `requiredApprovalFields`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromotionApproval {
  pub actor_id: String,
  pub tenant_id: String,
  pub approved_at_ms: u64,
  pub decision: PromotionApprovalDecision,
  /// SHA-256 fingerprint of the candidate being approved. The gate
  /// verifies this matches the actual candidate's fingerprint.
  pub candidate_fingerprint: String,
  /// Optional time-to-live in ms. `None` = no expiry.
  pub ttl_ms: Option<u64>,
  /// Optional human-readable reason. Audit-only.
  pub reason: Option<String>,
}

/// Compute the deterministic fingerprint of a regression-proven
/// candidate. SHA-256 of `target_owner | target_table |
/// folded_source_text`. Replay-stable across processes and
/// platforms.
pub fn candidate_fingerprint(candidate: &RegressionProvenCandidate) -> String {
  let mut h = Sha256::new();
  h.update(candidate.source.source.source.target_owner.as_bytes());
  h.update(b"\x1f");
  h.update(candidate.source.source.source.target_table.as_bytes());
  h.update(b"\x1f");
  h.update(candidate.source.source.folded_source_text.as_bytes());
  let digest = h.finalize();
  format!("{:x}", digest)
}

/// The output of the gate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OwnerLawProcessedCandidate {
  pub source: RegressionProvenCandidate,
  pub outcome: OwnerLawOutcome,
  pub gate_status: GateStatus,
  /// The fingerprint computed from the candidate. Audit-ref
  /// callers can compare to the approval's claimed fingerprint.
  pub candidate_fingerprint: String,
  /// The approval that was consulted (or `None` if no approval was
  /// supplied). Audit-only.
  pub approval: Option<PromotionApproval>,
  pub reason: String,
}

/// Process a `RegressionProvenCandidate` with an optional explicit
/// approval and the current wall-clock time.
///
/// Outcome priority:
///   1. Upstream gate did not produce `RegressionProven` →
///      `HeldUpstreamGate`
///   2. No approval supplied → `HeldAwaitingApproval`
///   3. Approval actor/tenant id violates owner-law identity policy →
///      `HeldInvalidApprovalIdentity`
///   4. Approval fingerprint mismatches candidate → `HeldStaleApproval`
///   5. Approval has `ttl_ms` and `current_time_ms` past expiry →
///      `HeldExpiredApproval`
///   6. Approval decision = `HoldForMoreEvidence` → `HeldForMoreEvidence`
///   7. Approval decision = `Reject` → `Rejected`
///   8. Approval decision = `Approve` → `Promoted`
///
/// **OWNER-LAW (v0.6.6, 2026-05-15)**: this pure-Rust body is a
/// **byte-equivalent mirror** of the `.px` owner
/// `stdlib/lib/gate/algorithm-synthesis/owner-law-gate.px::checkOwnerLawApproval`
/// (v0.6.5 migration). The `.px` is the **canonical owner**; this
/// Rust body exists as a cycle-free in-process fast-path because
/// `pnix-core` cannot depend on `pnix-eval`.
///
/// The byte-equivalent `.px`-delegating entry is
/// `doghouse_core::algorithm_synthesis_bridge::process_owner_law_via_px_body`.
/// Drift between the two paths is prevented by the byte-equivalence
/// ratchet test
/// `crates/doghouse-core/tests/process_owner_law_via_px_body_byte_equivalence.rs`
/// and the full-pipeline test
/// `crates/doghouse-core/tests/firewall_pipeline_dual_path_equivalence.rs`.
pub fn process_owner_law(
  candidate: &RegressionProvenCandidate,
  approval: Option<&PromotionApproval>,
  current_time_ms: u64,
) -> OwnerLawProcessedCandidate {
  let fingerprint = candidate_fingerprint(candidate);

  // (1) Upstream-gate hold propagates.
  if candidate.outcome != RegressionOutcome::RegressionProven {
    return OwnerLawProcessedCandidate {
      source: candidate.clone(),
      outcome: OwnerLawOutcome::HeldUpstreamGate,
      gate_status: GateStatus::Held,
      candidate_fingerprint: fingerprint,
      approval: approval.cloned(),
      reason: format!(
        "regression-proof gate produced `{}` — owner-law cannot proceed without a regression-proven row",
        candidate.outcome.as_str()
      ),
    };
  }

  // (2) No approval supplied.
  let Some(a) = approval else {
    return OwnerLawProcessedCandidate {
      source: candidate.clone(),
      outcome: OwnerLawOutcome::HeldAwaitingApproval,
      gate_status: GateStatus::Held,
      candidate_fingerprint: fingerprint,
      approval: None,
      reason: "no PromotionApproval supplied; candidate is queued for operator review".to_string(),
    };
  };

  // (3) Approval identity-shape policy.
  if let Err(identity_error) = validate_approval_identity(a) {
    return OwnerLawProcessedCandidate {
      source: candidate.clone(),
      outcome: OwnerLawOutcome::HeldInvalidApprovalIdentity,
      gate_status: GateStatus::Held,
      candidate_fingerprint: fingerprint,
      approval: Some(a.clone()),
      reason: format!(
        "approval identity rejected by owner-law policy: {identity_error}; this is not external auth, but anonymous/unscoped approvals cannot promote"
      ),
    };
  }

  // (4) Fingerprint mismatch (TOCTOU).
  if a.candidate_fingerprint != fingerprint {
    return OwnerLawProcessedCandidate {
      source: candidate.clone(),
      outcome: OwnerLawOutcome::HeldStaleApproval,
      gate_status: GateStatus::Held,
      candidate_fingerprint: fingerprint.clone(),
      approval: Some(a.clone()),
      reason: format!(
        "approval references candidate fingerprint `{}` but actual candidate fingerprints to `{fingerprint}` — candidate changed since approval was issued",
        a.candidate_fingerprint
      ),
    };
  }

  // (5) TTL expiry.
  if let Some(ttl) = a.ttl_ms {
    let expires_at = a.approved_at_ms.saturating_add(ttl);
    if current_time_ms > expires_at {
      return OwnerLawProcessedCandidate {
        source: candidate.clone(),
        outcome: OwnerLawOutcome::HeldExpiredApproval,
        gate_status: GateStatus::Held,
        candidate_fingerprint: fingerprint,
        approval: Some(a.clone()),
        reason: format!(
          "approval expired at {expires_at}ms; current time is {current_time_ms}ms — operator must re-approve"
        ),
      };
    }
  }

  // (6)–(8) Decision.
  match a.decision {
    PromotionApprovalDecision::HoldForMoreEvidence => OwnerLawProcessedCandidate {
      source: candidate.clone(),
      outcome: OwnerLawOutcome::HeldForMoreEvidence,
      gate_status: GateStatus::Held,
      candidate_fingerprint: fingerprint,
      approval: Some(a.clone()),
      reason: "operator chose hold-for-more-evidence; candidate stays in pending queue".to_string(),
    },
    PromotionApprovalDecision::Reject => OwnerLawProcessedCandidate {
      source: candidate.clone(),
      outcome: OwnerLawOutcome::Rejected,
      gate_status: GateStatus::Rejected,
      candidate_fingerprint: fingerprint,
      approval: Some(a.clone()),
      reason: format!(
        "operator rejected candidate; supersede-chain entry recorded.{}",
        a.reason.as_deref().map(|r| format!(" reason: {r}")).unwrap_or_default()
      ),
    },
    PromotionApprovalDecision::Approve => OwnerLawProcessedCandidate {
      source: candidate.clone(),
      outcome: OwnerLawOutcome::Promoted,
      gate_status: GateStatus::Promoted,
      candidate_fingerprint: fingerprint,
      approval: Some(a.clone()),
      reason:
        "operator authorized PNIX to fold this candidate into the owner-law substrate (Stage E eligible)"
          .to_string(),
    },
  }
}

/// Render an `OwnerLawProcessedCandidate` as the canonical JSON
/// payload of a `coding.owner-law-processed-candidate`
/// artifact. Stage D-5 (Gate 5, *the* firewall gate) cockpit
/// surface — operator sees the cryptographic fingerprint, the
/// approval decision (or absence), TTL, actor/tenant, plus the
/// final gate-5 verdict.
///
/// 8 outcomes covered (`promoted` / `rejected` / `held-upstream-gate`
/// / `held-awaiting-approval` / `held-stale-approval` /
/// `held-expired-approval` / `held-for-more-evidence` /
/// `held-invalid-approval-identity`). On any outcome, the candidate
/// fingerprint is included so audit can verify the approval was
/// bound to *this exact* candidate (TOCTOU guard). On non-promoted
/// outcomes, the approval block is still surfaced when present so
/// operator sees *what was attempted*.
///
/// Replay-stable id = SHA-256 of intrinsic identity (outcome +
/// candidate_fingerprint + approval.actor_id + approval.decision +
/// approval.candidate_fingerprint + approval.ttl_ms +
/// approval.approved_at_ms). `stored_at_ms` is extrinsic.
///
/// Content policy: metadata + cryptographic refs + actor ids
/// only. No source bodies. Customer-release safe.
pub fn build_owner_law_processed_candidate_artifact(
  processed: &OwnerLawProcessedCandidate,
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
) -> serde_json::Value {
  let proposal = &processed.source.source.source.source;

  let mut h = Sha256::new();
  h.update(b"owner-law-processed-candidate\x1f");
  h.update(processed.outcome.as_str().as_bytes());
  h.update(b"\x1f");
  h.update(processed.candidate_fingerprint.as_bytes());
  h.update(b"\x1f");
  if let Some(a) = &processed.approval {
    h.update(a.actor_id.as_bytes());
    h.update(b"\x1e");
    h.update(a.tenant_id.as_bytes());
    h.update(b"\x1e");
    h.update(a.decision.as_str().as_bytes());
    h.update(b"\x1e");
    h.update(a.candidate_fingerprint.as_bytes());
    h.update(b"\x1e");
    h.update(a.approved_at_ms.to_string().as_bytes());
    h.update(b"\x1e");
    if let Some(ttl) = a.ttl_ms {
      h.update(ttl.to_string().as_bytes());
    }
  }
  let digest = h.finalize();
  let prefix = digest
    .iter()
    .take(16)
    .map(|b| format!("{b:02x}"))
    .collect::<String>();
  let id = format!("owner-law-processed-candidate.{prefix}");

  let mut payload = serde_json::json!({
    "id": id,
    "artifact_family": "coding.owner-law-processed-candidate",
    "source_surface": "algorithm-synthesis.owner-law-gate",
    "stored_at_ms": stored_at_ms,
    "outcome": processed.outcome.as_str(),
    "gate_status": processed.gate_status.as_str(),
    "candidate_kind": proposal.candidate_kind.as_str(),
    "target_owner": proposal.target_owner,
    "target_table": proposal.target_table,
    "candidate_fingerprint": processed.candidate_fingerprint,
    "reason": processed.reason,
    "related_refs": serde_json::json!([
      format!("candidate-kind:{}", proposal.candidate_kind.as_str()),
      format!("target-owner:{}", proposal.target_owner),
      format!("target-table:{}", proposal.target_table),
      format!("candidate-fingerprint:{}", processed.candidate_fingerprint),
      "owner-law:stdlib/lib/gate/algorithm-synthesis/owner-law-gate.px",
    ]),
    "target_paths": serde_json::json!([proposal.target_owner]),
    "command_refs": Vec::<String>::new(),
  });

  // Approval block — present when an approval was supplied, even
  // on non-promoted outcomes (operator sees what was attempted).
  if let Some(a) = &processed.approval {
    let mut approval_obj = serde_json::json!({
      "actor_id": a.actor_id,
      "tenant_id": a.tenant_id,
      "approved_at_ms": a.approved_at_ms,
      "decision": a.decision.as_str(),
      "candidate_fingerprint": a.candidate_fingerprint,
      "fingerprint_matches_candidate":
        a.candidate_fingerprint == processed.candidate_fingerprint,
      "identity_policy_matches": validate_approval_identity(a).is_ok(),
    });
    if let Some(ttl) = a.ttl_ms {
      approval_obj["ttl_ms"] = serde_json::Value::Number(serde_json::Number::from(ttl));
    }
    if let Some(reason) = &a.reason {
      approval_obj["reason"] = serde_json::Value::String(reason.clone());
    }
    payload["approval"] = approval_obj;
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
  use super::super::regression_proof_gate::check_regression_proof;
  use super::*;
  use std::collections::BTreeMap;

  fn schema_aware_regression_proven_candidate(
    held: &str,
    primary: &str,
  ) -> RegressionProvenCandidate {
    let mut row = BTreeMap::new();
    row.insert("held".to_string(), held.to_string());
    row.insert("primary".to_string(), primary.to_string());
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
    let axis = check_axis_separation(&fold_proposal(&proposal));
    check_regression_proof(&axis, &[])
  }

  fn approval_for(
    candidate: &RegressionProvenCandidate,
    decision: PromotionApprovalDecision,
  ) -> PromotionApproval {
    PromotionApproval {
      actor_id: "actor.operator".into(),
      tenant_id: "tenant.test".into(),
      approved_at_ms: 1700000000000,
      decision,
      candidate_fingerprint: candidate_fingerprint(candidate),
      ttl_ms: None,
      reason: None,
    }
  }

  // ─── fingerprint stability ─────────────────────────────────────

  #[test]
  fn fingerprint_is_deterministic_across_runs() {
    let c1 = schema_aware_regression_proven_candidate("h", "p");
    let c2 = schema_aware_regression_proven_candidate("h", "p");
    assert_eq!(candidate_fingerprint(&c1), candidate_fingerprint(&c2));
  }

  #[test]
  fn fingerprint_differs_for_different_rows() {
    let c1 = schema_aware_regression_proven_candidate("h1", "p");
    let c2 = schema_aware_regression_proven_candidate("h2", "p");
    assert_ne!(candidate_fingerprint(&c1), candidate_fingerprint(&c2));
  }

  // ─── upstream-gate propagation ─────────────────────────────────

  #[test]
  fn upstream_held_propagates_as_held_upstream_gate() {
    // Build a regression-held candidate.
    let mut row = BTreeMap::new();
    row.insert("import_spec".to_string(), "import os".to_string());
    let proposal = CandidateRowProposal {
      candidate_kind: CandidateKind::RecurringImportSpec,
      target_owner: "stdlib/lib/gate/algorithm-synthesis/fact-cue-registry.px".to_string(),
      target_table: "factPhrasePatterns".to_string(),
      proposed_row: row,
      supporting_evidence: vec![],
      evidence_count: 2,
      gate_status: GateStatus::IntentReceiptOnly,
      reason: "fixture".to_string(),
    };
    let axis = check_axis_separation(&fold_proposal(&proposal));
    let reg = check_regression_proof(&axis, &[]);
    // reg.outcome is HeldAxisNotVerified (axis hold propagated).
    let r = process_owner_law(&reg, None, 1700000000000);
    assert_eq!(r.outcome, OwnerLawOutcome::HeldUpstreamGate);
    assert_eq!(r.gate_status, GateStatus::Held);
  }

  // ─── awaiting approval ─────────────────────────────────────────

  #[test]
  fn no_approval_holds_awaiting() {
    let c = schema_aware_regression_proven_candidate("future-kind", "host-symbol-resolver");
    let r = process_owner_law(&c, None, 1700000000000);
    assert_eq!(r.outcome, OwnerLawOutcome::HeldAwaitingApproval);
    assert_eq!(r.gate_status, GateStatus::Held);
    assert!(r.approval.is_none());
  }

  // ─── approval identity policy ────────────────────────────────

  #[test]
  fn malformed_actor_identity_holds_before_promotion() {
    let c = schema_aware_regression_proven_candidate("future-kind", "host-symbol-resolver");
    let mut a = approval_for(&c, PromotionApprovalDecision::Approve);
    a.actor_id = "literally-anything".to_string();
    let r = process_owner_law(&c, Some(&a), 1700000000000);
    assert_eq!(r.outcome, OwnerLawOutcome::HeldInvalidApprovalIdentity);
    assert_eq!(r.gate_status, GateStatus::Held);
    assert!(r.reason.contains("actor_id must start"));
  }

  #[test]
  fn malformed_tenant_identity_holds_before_promotion() {
    let c = schema_aware_regression_proven_candidate("future-kind", "host-symbol-resolver");
    let mut a = approval_for(&c, PromotionApprovalDecision::Approve);
    a.tenant_id = "tenant/breakout".to_string();
    let r = process_owner_law(&c, Some(&a), 1700000000000);
    assert_eq!(r.outcome, OwnerLawOutcome::HeldInvalidApprovalIdentity);
    assert_eq!(r.gate_status, GateStatus::Held);
    assert!(r.reason.contains("tenant_id contains forbidden substring"));
  }

  // ─── TOCTOU fingerprint guard ─────────────────────────────────

  #[test]
  fn stale_approval_with_wrong_fingerprint_holds() {
    let c = schema_aware_regression_proven_candidate("future-kind", "host-symbol-resolver");
    let mut a = approval_for(&c, PromotionApprovalDecision::Approve);
    a.candidate_fingerprint = "deadbeef".to_string();
    let r = process_owner_law(&c, Some(&a), 1700000000000);
    assert_eq!(r.outcome, OwnerLawOutcome::HeldStaleApproval);
    assert_eq!(r.gate_status, GateStatus::Held);
  }

  // ─── TTL expiry ────────────────────────────────────────────────

  #[test]
  fn expired_approval_holds() {
    let c = schema_aware_regression_proven_candidate("future-kind", "host-symbol-resolver");
    let mut a = approval_for(&c, PromotionApprovalDecision::Approve);
    a.ttl_ms = Some(1000); // 1 second
                           // approved_at = 1700000000000, ttl = 1s, current = +2s
    let r = process_owner_law(&c, Some(&a), 1700000002000);
    assert_eq!(r.outcome, OwnerLawOutcome::HeldExpiredApproval);
    assert_eq!(r.gate_status, GateStatus::Held);
  }

  #[test]
  fn unexpired_approval_within_ttl_promotes() {
    let c = schema_aware_regression_proven_candidate("future-kind", "host-symbol-resolver");
    let mut a = approval_for(&c, PromotionApprovalDecision::Approve);
    a.ttl_ms = Some(60_000); // 60 seconds
    let r = process_owner_law(&c, Some(&a), 1700000000500);
    assert_eq!(r.outcome, OwnerLawOutcome::Promoted);
  }

  #[test]
  fn no_ttl_never_expires() {
    let c = schema_aware_regression_proven_candidate("future-kind", "host-symbol-resolver");
    let a = approval_for(&c, PromotionApprovalDecision::Approve);
    // current_time far in the future — no ttl set, no expiry.
    let r = process_owner_law(&c, Some(&a), u64::MAX / 2);
    assert_eq!(r.outcome, OwnerLawOutcome::Promoted);
  }

  // ─── decisions ─────────────────────────────────────────────────

  #[test]
  fn approve_decision_promotes() {
    let c = schema_aware_regression_proven_candidate("future-kind", "host-symbol-resolver");
    let a = approval_for(&c, PromotionApprovalDecision::Approve);
    let r = process_owner_law(&c, Some(&a), 1700000000000);
    assert_eq!(r.outcome, OwnerLawOutcome::Promoted);
    assert_eq!(r.gate_status, GateStatus::Promoted);
  }

  #[test]
  fn reject_decision_records_supersede_audit() {
    let c = schema_aware_regression_proven_candidate("future-kind", "host-symbol-resolver");
    let mut a = approval_for(&c, PromotionApprovalDecision::Reject);
    a.reason = Some("not aligned with current routing policy".to_string());
    let r = process_owner_law(&c, Some(&a), 1700000000000);
    assert_eq!(r.outcome, OwnerLawOutcome::Rejected);
    assert_eq!(r.gate_status, GateStatus::Rejected);
    assert!(r.reason.contains("not aligned with current routing policy"));
    // Audit-ref: approval is preserved.
    assert!(r.approval.is_some());
  }

  #[test]
  fn hold_for_more_evidence_keeps_candidate_pending() {
    let c = schema_aware_regression_proven_candidate("future-kind", "host-symbol-resolver");
    let a = approval_for(&c, PromotionApprovalDecision::HoldForMoreEvidence);
    let r = process_owner_law(&c, Some(&a), 1700000000000);
    assert_eq!(r.outcome, OwnerLawOutcome::HeldForMoreEvidence);
    assert_eq!(r.gate_status, GateStatus::Held);
  }

  // ─── audit preservation ───────────────────────────────────────

  #[test]
  fn promoted_outcome_preserves_full_source_chain() {
    let c = schema_aware_regression_proven_candidate("future-kind", "host-symbol-resolver");
    let a = approval_for(&c, PromotionApprovalDecision::Approve);
    let r = process_owner_law(&c, Some(&a), 1700000000000);
    // Walk back through every gate's source.
    assert_eq!(r.source, c);
    assert_eq!(r.source.source.matched_keys.len() > 0, true);
    assert_eq!(
      r.source.source.source.outcome,
      super::super::macro_fold_gate::MacroFoldOutcome::Folded
    );
    // The original proposal must be reachable.
    let original_proposal = &r.source.source.source.source;
    assert_eq!(original_proposal.evidence_count, 2);
    assert_eq!(r.candidate_fingerprint, candidate_fingerprint(&c));
  }

  // ─── owner-law-processed-candidate artifact (Stage D-5) ───────

  fn promoted_processed() -> OwnerLawProcessedCandidate {
    let c = schema_aware_regression_proven_candidate("missing-import-spec", "host-symbol-resolver");
    let a = approval_for(&c, PromotionApprovalDecision::Approve);
    process_owner_law(&c, Some(&a), 1700000000500)
  }

  #[test]
  fn artifact_promoted_carries_fingerprint_and_matching_approval() {
    let p = promoted_processed();
    assert_eq!(p.outcome, OwnerLawOutcome::Promoted);
    let art = build_owner_law_processed_candidate_artifact(&p, 1700000001000, None);
    assert_eq!(
      art["artifact_family"],
      "coding.owner-law-processed-candidate"
    );
    assert_eq!(art["outcome"], "promoted");
    let fp = art["candidate_fingerprint"].as_str().unwrap();
    assert_eq!(fp.len(), 64, "SHA-256 hex = 64 chars");
    let approval = art["approval"].as_object().expect("approval present");
    assert_eq!(approval.get("decision").unwrap(), "approve");
    assert_eq!(
      approval.get("fingerprint_matches_candidate").unwrap(),
      true,
      "TOCTOU guard: approval fp must match candidate fp"
    );
  }

  #[test]
  fn artifact_held_awaiting_approval_has_no_approval_block() {
    let c = schema_aware_regression_proven_candidate("h", "p");
    let p = process_owner_law(&c, None, 1700000000500);
    assert_eq!(p.outcome, OwnerLawOutcome::HeldAwaitingApproval);
    let art = build_owner_law_processed_candidate_artifact(&p, 0, None);
    assert_eq!(art["outcome"], "held-awaiting-approval");
    // Fingerprint still surfaced — operator sees what *would* need
    // approval.
    assert_eq!(art["candidate_fingerprint"].as_str().unwrap().len(), 64);
    assert!(art.get("approval").is_none(), "no approval → no block");
  }

  #[test]
  fn artifact_held_stale_approval_surfaces_mismatch() {
    let c = schema_aware_regression_proven_candidate("h", "p");
    let mut a = approval_for(&c, PromotionApprovalDecision::Approve);
    // Tamper: approval claims a different fingerprint.
    a.candidate_fingerprint =
      "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef".to_string();
    let p = process_owner_law(&c, Some(&a), 1700000000500);
    assert_eq!(p.outcome, OwnerLawOutcome::HeldStaleApproval);
    let art = build_owner_law_processed_candidate_artifact(&p, 0, None);
    assert_eq!(art["outcome"], "held-stale-approval");
    let approval = art["approval"].as_object().expect("approval present");
    assert_eq!(
      approval.get("fingerprint_matches_candidate").unwrap(),
      false,
      "stale approval → fingerprint mismatch surfaced"
    );
  }

  #[test]
  fn artifact_held_invalid_identity_surfaces_policy_result() {
    let c = schema_aware_regression_proven_candidate("h", "p");
    let mut a = approval_for(&c, PromotionApprovalDecision::Approve);
    a.actor_id = "anonymous".to_string();
    let p = process_owner_law(&c, Some(&a), 1700000000500);
    assert_eq!(p.outcome, OwnerLawOutcome::HeldInvalidApprovalIdentity);
    let art = build_owner_law_processed_candidate_artifact(&p, 0, None);
    assert_eq!(art["outcome"], "held-invalid-approval-identity");
    let approval = art["approval"].as_object().expect("approval present");
    assert_eq!(
      approval.get("identity_policy_matches").unwrap(),
      false,
      "invalid actor/tenant principal shape is surfaced to cockpit/audit"
    );
  }

  #[test]
  fn artifact_rejected_carries_reject_decision() {
    let c = schema_aware_regression_proven_candidate("h", "p");
    let a = approval_for(&c, PromotionApprovalDecision::Reject);
    let p = process_owner_law(&c, Some(&a), 1700000000500);
    assert_eq!(p.outcome, OwnerLawOutcome::Rejected);
    let art = build_owner_law_processed_candidate_artifact(&p, 0, None);
    assert_eq!(art["outcome"], "rejected");
    assert_eq!(
      art["approval"]
        .as_object()
        .unwrap()
        .get("decision")
        .unwrap(),
      "reject"
    );
  }

  #[test]
  fn artifact_held_expired_approval_when_ttl_passed() {
    let c = schema_aware_regression_proven_candidate("h", "p");
    let mut a = approval_for(&c, PromotionApprovalDecision::Approve);
    a.approved_at_ms = 1700000000000;
    a.ttl_ms = Some(100);
    let p = process_owner_law(&c, Some(&a), 1700000001000);
    assert_eq!(p.outcome, OwnerLawOutcome::HeldExpiredApproval);
    let art = build_owner_law_processed_candidate_artifact(&p, 0, None);
    assert_eq!(art["outcome"], "held-expired-approval");
    let approval = art["approval"].as_object().unwrap();
    assert_eq!(approval.get("ttl_ms").unwrap(), 100);
  }

  #[test]
  fn artifact_id_is_replay_stable_across_stored_at() {
    let p = promoted_processed();
    let a1 = build_owner_law_processed_candidate_artifact(&p, 1, None);
    let a2 = build_owner_law_processed_candidate_artifact(&p, 999999, None);
    assert_eq!(a1["id"], a2["id"]);
  }

  #[test]
  fn artifact_id_differs_between_promoted_and_rejected() {
    let c = schema_aware_regression_proven_candidate("h", "p");
    let p_approve = process_owner_law(
      &c,
      Some(&approval_for(&c, PromotionApprovalDecision::Approve)),
      1700000000500,
    );
    let p_reject = process_owner_law(
      &c,
      Some(&approval_for(&c, PromotionApprovalDecision::Reject)),
      1700000000500,
    );
    let a1 = build_owner_law_processed_candidate_artifact(&p_approve, 0, None);
    let a2 = build_owner_law_processed_candidate_artifact(&p_reject, 0, None);
    assert_ne!(a1["id"], a2["id"]);
  }
}
