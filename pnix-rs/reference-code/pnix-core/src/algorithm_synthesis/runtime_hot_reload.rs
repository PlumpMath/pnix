//! Runtime hot-reload — Stage E of the evolution lane.
//!
//! OWNER-LAW (2026-05-12): mirror of
//! `stdlib/lib/gate/algorithm-synthesis/runtime-hot-reload.px`.
//! Consumes an `OwnerLawProcessedCandidate { outcome: Promoted }`
//! and produces a `HotReloadPlan` describing where the row text
//! should be inserted into the target `.px` file.
//!
//! v0 is a *dry-run plan generator* — it does NOT touch disk. The
//! caller hands the plan to
//! `tool-action-runtime::execute_materialization_plan` with
//! AllOrNothing semantics, the same substrate proven safe by the
//! rename-symbol and remove-unused-import roundtrip tests.

use serde::{Deserialize, Serialize};
use pnix_hash::{Digest, Sha256};

use super::candidate_row_proposal::GateStatus;
use super::owner_law_gate::{OwnerLawOutcome, OwnerLawProcessedCandidate};
use crate::tool_action::{
  ApplyReceiptFileState, ToolActionMaterializationPlan, ToolActionMaterializationPlanError,
  ToolActionMaterializationRequest,
};

/// Hot-reload plan outcomes. Stays byte-identical to `.px`
/// `validHotReloadOutcomes`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HotReloadOutcome {
  PlanReady,
  HeldNotPromoted,
  HeldAnchorNotFound,
  HeldSyntaxImbalanced,
  HeldTargetFileUnknown,
}

impl HotReloadOutcome {
  pub const ALL: &'static [Self] = &[
    Self::PlanReady,
    Self::HeldNotPromoted,
    Self::HeldAnchorNotFound,
    Self::HeldSyntaxImbalanced,
    Self::HeldTargetFileUnknown,
  ];

  pub fn as_str(self) -> &'static str {
    match self {
      Self::PlanReady => "plan-ready",
      Self::HeldNotPromoted => "held-not-promoted",
      Self::HeldAnchorNotFound => "held-anchor-not-found",
      Self::HeldSyntaxImbalanced => "held-syntax-imbalanced",
      Self::HeldTargetFileUnknown => "held-target-file-unknown",
    }
  }
}

#[derive(Debug, Clone, Copy)]
pub struct InsertionAnchor {
  pub target_owner: &'static str,
  pub target_table: &'static str,
  pub table_declaration: &'static str,
  pub anchor_pattern: &'static str,
}

pub const INSERTION_ANCHORS: &[InsertionAnchor] = &[
  InsertionAnchor {
    target_owner: "stdlib/lib/gate/algorithm-synthesis/held-to-query.px",
    target_table: "heldRoutingMap",
    table_declaration: "heldRoutingMap = [",
    anchor_pattern: "];",
  },
  InsertionAnchor {
    target_owner: "stdlib/lib/gate/algorithm-synthesis/fact-cue-registry.px",
    target_table: "factPhrasePatterns",
    table_declaration: "factPhrasePatterns = [",
    anchor_pattern: "];",
  },
  InsertionAnchor {
    target_owner: "stdlib/lib/gate/algorithm-synthesis/operation-candidate-mapping.px",
    target_table: "operationMap",
    table_declaration: "operationMap = [",
    anchor_pattern: "];",
  },
  InsertionAnchor {
    target_owner: "stdlib/lib/gate/known-imports-by-language.px",
    target_table: "knownImportsByLanguage",
    table_declaration: "knownImportsByLanguage = [",
    anchor_pattern: "];",
  },
  InsertionAnchor {
    target_owner: "stdlib/lib/gate/known-algebraic-identities.px",
    target_table: "knownAlgebraicIdentities",
    table_declaration: "knownAlgebraicIdentities = [",
    anchor_pattern: "];",
  },
  InsertionAnchor {
    target_owner: "stdlib/lib/gate/known-chemical-reactions.px",
    target_table: "knownChemicalReactions",
    table_declaration: "knownChemicalReactions = [",
    anchor_pattern: "];",
  },
];

fn anchor_for(target_owner: &str, target_table: &str) -> Option<&'static InsertionAnchor> {
  INSERTION_ANCHORS
    .iter()
    .find(|a| a.target_owner == target_owner && a.target_table == target_table)
}

fn sha256_hex(s: &str) -> String {
  let mut h = Sha256::new();
  h.update(s.as_bytes());
  format!("{:x}", h.finalize())
}

/// Brace-balance check (v0 fallback). Counts `{ } [ ]`
/// occurrences; balanced means counts match per pair. Generic —
/// does not understand strings/comments. Kept available for
/// callers who don't want to pay the parser cost (rare).
fn brace_balanced(text: &str) -> bool {
  let mut curly = 0i64;
  let mut square = 0i64;
  for c in text.chars() {
    match c {
      '{' => curly += 1,
      '}' => curly -= 1,
      '[' => square += 1,
      ']' => square -= 1,
      _ => {}
    }
    if curly < 0 || square < 0 {
      return false;
    }
  }
  curly == 0 && square == 0
}

/// **Real Nix parse check (v1).** Calls
/// `pnix_core::lang::pnix::parser::parse_expr` on the post-apply
/// content. Stricter than brace-balance — rejects content that is
/// brace-balanced but not syntactically valid Nix (e.g.
/// `{ key value }` without `=`).
///
/// Returns `Ok(())` on parse success, `Err(reason)` on parse
/// failure with the parser's own error message embedded.
fn nix_parse_check(text: &str) -> Result<(), String> {
  match crate::lang::pnix::parser::parse_expr(text) {
    Ok(_) => Ok(()),
    Err(e) => Err(format!(
      "pnix-eval parser rejected post-apply content: {e:?}"
    )),
  }
}

/// The plan produced by Stage E.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HotReloadPlan {
  pub source: OwnerLawProcessedCandidate,
  pub outcome: HotReloadOutcome,
  pub gate_status: GateStatus,
  /// Repo-relative path of the file the plan would modify. Empty
  /// on `HeldTargetFileUnknown`.
  pub target_path: String,
  /// sha256 of the file's current bytes (i.e. before the row is
  /// inserted). Empty on Held outcomes.
  pub pre_apply_sha256: String,
  /// sha256 of the file's bytes after the row is inserted. Empty
  /// on Held outcomes.
  pub post_apply_sha256: String,
  /// The full post-apply file contents. Empty on Held outcomes.
  /// Caller hands this to AllOrNothing materialization via
  /// `ApplyReceiptFileState { post_apply_content: Some(...) }`.
  pub post_apply_content: String,
  /// The exact text inserted into the file. Useful for audit and
  /// for the AllOrNothing rollback to know what to remove. Empty
  /// on Held outcomes.
  pub inserted_row_text: String,
  pub reason: String,
}

/// Render a `HotReloadPlan` as the canonical JSON payload of a
/// `coding.hot-reload-plan-{ready,held}` artifact. The
/// artifact id is SHA-256 of intrinsic identity (target_path +
/// pre/post sha + outcome + inserted text). `stored_at_ms` is
/// extrinsic (not part of the hash).
///
/// Content policy: the full `post_apply_content` is omitted —
/// callers who need it consult the plan directly; the artifact is
/// the metadata projection for cockpit / audit purposes. This
/// keeps `coding.hot-reload-plan-*` customer-release-safe by
/// default (no source bodies in the artifact).
pub fn build_hot_reload_plan_artifact(
  plan: &HotReloadPlan,
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
) -> serde_json::Value {
  let mut h = Sha256::new();
  h.update(b"hot-reload-plan\x1f");
  h.update(plan.target_path.as_bytes());
  h.update(b"\x1f");
  h.update(plan.pre_apply_sha256.as_bytes());
  h.update(b"\x1f");
  h.update(plan.post_apply_sha256.as_bytes());
  h.update(b"\x1f");
  h.update(plan.outcome.as_str().as_bytes());
  h.update(b"\x1f");
  h.update(plan.inserted_row_text.as_bytes());
  let digest = h.finalize();
  let prefix = digest
    .iter()
    .take(16)
    .map(|b| format!("{b:02x}"))
    .collect::<String>();
  let id = format!("hot-reload-plan.{prefix}");

  let family = if plan.outcome == HotReloadOutcome::PlanReady {
    "coding.hot-reload-plan-ready"
  } else {
    "coding.hot-reload-plan-held"
  };

  // Source-chain back-refs for audit walk. The full chain is in
  // the OwnerLawProcessedCandidate inside `plan.source`; the
  // artifact surfaces the candidate's fingerprint as the entry
  // point.
  let mut payload = serde_json::json!({
    "id": id,
    "artifact_family": family,
    "source_surface": "algorithm-synthesis.runtime-hot-reload",
    "stored_at_ms": stored_at_ms,
    "outcome": plan.outcome.as_str(),
    "gate_status": match plan.gate_status {
      GateStatus::Promoted => "promoted",
      GateStatus::Held => "held",
      GateStatus::Rejected => "rejected",
      GateStatus::IntentReceiptOnly => "intent-receipt-only",
      GateStatus::MacroFoldAttempted => "macro-fold-attempted",
      GateStatus::AxisSeparationAttempted => "axis-separation-attempted",
      GateStatus::RegressionProofAttempted => "regression-proof-attempted",
      GateStatus::OwnerLawAttempted => "owner-law-attempted",
    },
    "target_path": plan.target_path,
    "pre_apply_sha256": plan.pre_apply_sha256,
    "post_apply_sha256": plan.post_apply_sha256,
    "inserted_row_byte_len": plan.inserted_row_text.len(),
    "reason": plan.reason,
    "gate_chain": build_gate_chain_array(plan),
    "related_refs": serde_json::json!([
      format!("owner-law-candidate-fingerprint:{}", plan.source.candidate_fingerprint),
      format!("target-table:{}", plan.source.source.source.source.source.target_table),
      "owner-law:stdlib/lib/gate/algorithm-synthesis/runtime-hot-reload.px",
    ]),
    "target_paths": serde_json::json!([plan.target_path]),
    "command_refs": Vec::<String>::new(),
  });
  if let Some(snap) = repo_snapshot_ref {
    payload["repo_snapshot_ref"] = serde_json::Value::String(snap.to_string());
  }
  payload
}

/// Unfold the gate chain from Gate 1 (candidate-row) through Stage E
/// (hot-reload-plan) as one ordered array. Cockpit timeline panel
/// walks this in order. Each entry intrinsic to the plan — does NOT
/// affect the artifact id (id only covers target_path + pre/post sha
/// + outcome + inserted_row_text, which already pin the chain via
/// the upstream gates).
///
/// Stage E gates that never ran (because an earlier gate Held) still
/// surface, but with the propagated Held reason from the upstream
/// gate — this is intentional. The operator sees exactly which gate
/// was the cause, and which downstream gates were short-circuited.
fn build_gate_chain_array(plan: &HotReloadPlan) -> serde_json::Value {
  let g5_owner = &plan.source;
  let g4_regr = &g5_owner.source;
  let g3_axis = &g4_regr.source;
  let g2_fold = &g3_axis.source;
  let g1_row = &g2_fold.source;

  serde_json::json!([
    {
      "index": 1,
      "name": "candidate-row-proposal",
      "outcome": "proposed",
      "reason": g1_row.reason.clone(),
      "extra": {
        "candidate_kind": g1_row.candidate_kind.as_str(),
        "evidence_count": g1_row.evidence_count,
        "target_owner": g1_row.target_owner.clone(),
        "target_table": g1_row.target_table.clone(),
      },
    },
    {
      "index": 2,
      "name": "macro-fold",
      "outcome": g2_fold.outcome.as_str(),
      "reason": g2_fold.reason.clone(),
    },
    {
      "index": 3,
      "name": "axis-separation",
      "outcome": g3_axis.outcome.as_str(),
      "reason": g3_axis.reason.clone(),
    },
    {
      "index": 4,
      "name": "regression-proof",
      "outcome": g4_regr.outcome.as_str(),
      "reason": g4_regr.reason.clone(),
    },
    {
      "index": 5,
      "name": "owner-law",
      "outcome": g5_owner.outcome.as_str(),
      "reason": g5_owner.reason.clone(),
    },
    {
      "index": 6,
      "name": "stage-e-hot-reload-plan",
      "outcome": plan.outcome.as_str(),
      "reason": plan.reason.clone(),
    },
  ])
}

/// Build a hot-reload plan from a Promoted owner-law candidate.
///
/// `current_file_content` is the current text of the target `.px`
/// file (caller-supplied — Stage E does no I/O so it can be unit-
/// tested with synthetic inputs).
pub fn plan_hot_reload(
  candidate: &OwnerLawProcessedCandidate,
  current_file_content: &str,
) -> HotReloadPlan {
  let target_owner = candidate.source.source.source.source.target_owner.clone();
  let target_table = candidate.source.source.source.source.target_table.clone();

  // (1) Must be Promoted.
  if candidate.outcome != OwnerLawOutcome::Promoted {
    return HotReloadPlan {
      source: candidate.clone(),
      outcome: HotReloadOutcome::HeldNotPromoted,
      gate_status: GateStatus::Held,
      target_path: target_owner,
      pre_apply_sha256: String::new(),
      post_apply_sha256: String::new(),
      post_apply_content: String::new(),
      inserted_row_text: String::new(),
      reason: format!(
        "owner-law gate produced `{}` — hot-reload requires Promoted",
        candidate.outcome.as_str()
      ),
    };
  }

  // (2) Anchor must be registered.
  let Some(anchor) = anchor_for(&target_owner, &target_table) else {
    return HotReloadPlan {
      source: candidate.clone(),
      outcome: HotReloadOutcome::HeldTargetFileUnknown,
      gate_status: GateStatus::Held,
      target_path: target_owner,
      pre_apply_sha256: String::new(),
      post_apply_sha256: String::new(),
      post_apply_content: String::new(),
      inserted_row_text: String::new(),
      reason: format!(
        "no insertion anchor registered for ({target_table}) — register one in `insertionAnchors`"
      ),
    };
  };

  // (3) Anchor must be locatable in the current file.
  let Some(decl_idx) = current_file_content.find(anchor.table_declaration) else {
    return HotReloadPlan {
      source: candidate.clone(),
      outcome: HotReloadOutcome::HeldAnchorNotFound,
      gate_status: GateStatus::Held,
      target_path: target_owner,
      pre_apply_sha256: sha256_hex(current_file_content),
      post_apply_sha256: String::new(),
      post_apply_content: String::new(),
      inserted_row_text: String::new(),
      reason: format!(
        "table declaration `{}` not found in current file content",
        anchor.table_declaration
      ),
    };
  };
  let after_decl = &current_file_content[decl_idx + anchor.table_declaration.len()..];
  let Some(close_rel) = after_decl.find(anchor.anchor_pattern) else {
    return HotReloadPlan {
      source: candidate.clone(),
      outcome: HotReloadOutcome::HeldAnchorNotFound,
      gate_status: GateStatus::Held,
      target_path: target_owner,
      pre_apply_sha256: sha256_hex(current_file_content),
      post_apply_sha256: String::new(),
      post_apply_content: String::new(),
      inserted_row_text: String::new(),
      reason: format!(
        "anchor pattern `{}` not found after table declaration `{}`",
        anchor.anchor_pattern, anchor.table_declaration
      ),
    };
  };
  let close_idx = decl_idx + anchor.table_declaration.len() + close_rel;

  // (4) Build the inserted row text. The folded source text is
  // already a valid Nix attrset literal; we indent it 4 spaces
  // (matching existing rows' indent inside `[ ... ]`) and ensure a
  // trailing newline before the closing `];`.
  let folded = &candidate.source.source.source.folded_source_text;
  let indented: String = folded
    .lines()
    .map(|l| format!("    {l}"))
    .collect::<Vec<_>>()
    .join("\n");
  let inserted_row_text = format!("{indented}\n  ");

  // (5) Compose post-apply content: pre[..close_idx] + inserted + pre[close_idx..]
  let mut post_content =
    String::with_capacity(current_file_content.len() + inserted_row_text.len());
  post_content.push_str(&current_file_content[..close_idx]);
  post_content.push_str(&inserted_row_text);
  post_content.push_str(&current_file_content[close_idx..]);

  // (6) Two-step syntax verification:
  //   (a) brace-balance — cheap structural shape check.
  //   (b) full pnix-eval parser — catches malformed Nix that
  //       happens to brace-balance (e.g. `{ key value }` without
  //       `=`). v1 upgrade from brace-balance-only.
  if !brace_balanced(&post_content) {
    return HotReloadPlan {
      source: candidate.clone(),
      outcome: HotReloadOutcome::HeldSyntaxImbalanced,
      gate_status: GateStatus::Held,
      target_path: target_owner,
      pre_apply_sha256: sha256_hex(current_file_content),
      post_apply_sha256: String::new(),
      post_apply_content: String::new(),
      inserted_row_text,
      reason: "post-apply content has imbalanced `{}` / `[]` — fold step produced malformed text"
        .to_string(),
    };
  }
  if let Err(parse_err) = nix_parse_check(&post_content) {
    return HotReloadPlan {
      source: candidate.clone(),
      outcome: HotReloadOutcome::HeldSyntaxImbalanced,
      gate_status: GateStatus::Held,
      target_path: target_owner,
      pre_apply_sha256: sha256_hex(current_file_content),
      post_apply_sha256: String::new(),
      post_apply_content: String::new(),
      inserted_row_text,
      reason: parse_err,
    };
  }

  HotReloadPlan {
    source: candidate.clone(),
    outcome: HotReloadOutcome::PlanReady,
    gate_status: GateStatus::Promoted,
    target_path: target_owner,
    pre_apply_sha256: sha256_hex(current_file_content),
    post_apply_sha256: sha256_hex(&post_content),
    post_apply_content: post_content,
    inserted_row_text,
    reason: format!(
      "hot-reload plan ready: row inserted before `{}` in `{target_table}` declaration",
      anchor.anchor_pattern
    ),
  }
}

/// Context the caller supplies when lowering a HotReloadPlan to a
/// materialization plan. Same identity fields as
/// `ToolActionMaterializationRequest` requires.
#[derive(Debug, Clone)]
pub struct HotReloadMaterializationContext<'a> {
  pub apply_receipt_artifact_id: &'a str,
  pub repo_snapshot_ref: &'a str,
  /// Required `.px` editing capability. Distinct from
  /// `edit-within-target-paths` (used by code-transform writes)
  /// because owner-law substrate edits are higher-privilege.
  pub capability: &'a str,
  pub requested_by_actor_id: &'a str,
  pub requested_by_tenant_id: &'a str,
  pub requested_at_ms: u64,
  pub deployment_mode: &'a str,
}

/// Errors specific to building a materialization plan from a
/// hot-reload plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HotReloadMaterializationError {
  /// The hot-reload plan is not in `PlanReady` state. Cannot
  /// materialize a held plan.
  PlanNotReady(HotReloadOutcome),
  /// The underlying `tool_action` plan-builder rejected the
  /// request (e.g. drift, duplicate path, missing sha).
  ToolActionPlan(ToolActionMaterializationPlanError),
}

impl std::fmt::Display for HotReloadMaterializationError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::PlanNotReady(o) => write!(f, "hot-reload plan is not ready (outcome: {})", o.as_str()),
      Self::ToolActionPlan(e) => write!(f, "tool-action plan rejected: {e}"),
    }
  }
}

impl std::error::Error for HotReloadMaterializationError {}

/// Lower a `HotReloadPlan { PlanReady }` into a
/// `ToolActionMaterializationPlan` ready for
/// `tool_action_runtime::execute_materialization_plan`. The
/// resulting plan uses `content-policy=include-content` so the
/// AllOrNothing materializer has the bytes to write directly from
/// the plan (no separate `ContentSource::Lookup` needed).
///
/// `allowed_target_paths` MUST contain `plan.target_path` — the
/// caller asserts which paths the materializer is authorized to
/// touch. v0 callers typically pass a single-element slice
/// `&[plan.target_path.clone()]`.
pub fn build_materialization_plan(
  plan: &HotReloadPlan,
  ctx: &HotReloadMaterializationContext<'_>,
  allowed_target_paths: &[String],
) -> Result<ToolActionMaterializationPlan, HotReloadMaterializationError> {
  if plan.outcome != HotReloadOutcome::PlanReady {
    return Err(HotReloadMaterializationError::PlanNotReady(plan.outcome));
  }
  let request = ToolActionMaterializationRequest {
    apply_receipt_artifact_id: ctx.apply_receipt_artifact_id.to_string(),
    repo_snapshot_ref: ctx.repo_snapshot_ref.to_string(),
    capability: ctx.capability.to_string(),
    requested_by_actor_id: ctx.requested_by_actor_id.to_string(),
    requested_by_tenant_id: ctx.requested_by_tenant_id.to_string(),
    requested_at_ms: ctx.requested_at_ms,
    deployment_mode: ctx.deployment_mode.to_string(),
    content_policy: "include-content".to_string(),
  };
  let file_state = ApplyReceiptFileState {
    path: plan.target_path.clone(),
    pre_apply_sha256: plan.pre_apply_sha256.clone(),
    post_apply_sha256: plan.post_apply_sha256.clone(),
    post_apply_byte_len: plan.post_apply_content.len(),
    post_apply_content: Some(plan.post_apply_content.clone()),
  };
  crate::tool_action::build_tool_action_materialization_plan(
    &request,
    ctx.apply_receipt_artifact_id,
    vec![file_state],
    allowed_target_paths,
  )
  .map_err(HotReloadMaterializationError::ToolActionPlan)
}

/// Build an inverse-materialization plan that undoes a successful
/// hot-reload. The inverse swaps `pre_apply_sha256` and
/// `post_apply_sha256` and uses the original pre-apply content
/// as the inverse's post-apply payload — same AllOrNothing
/// semantics as rename-symbol's `nl_rename_then_rollback_*`
/// roundtrip test.
///
/// `pre_apply_content` is the caller-supplied original bytes
/// (typically read back from `plan.target_path` before the
/// hot-reload was applied). v0 keeps this caller-provided so the
/// rollback path has no I/O concerns at the owner level.
pub fn build_rollback_materialization_plan(
  plan: &HotReloadPlan,
  pre_apply_content: &str,
  ctx: &HotReloadMaterializationContext<'_>,
  allowed_target_paths: &[String],
) -> Result<ToolActionMaterializationPlan, HotReloadMaterializationError> {
  if plan.outcome != HotReloadOutcome::PlanReady {
    return Err(HotReloadMaterializationError::PlanNotReady(plan.outcome));
  }
  // Sanity: caller's pre_apply_content must hash to plan.pre_apply_sha256.
  let computed = sha256_hex(pre_apply_content);
  if computed != plan.pre_apply_sha256 {
    return Err(HotReloadMaterializationError::ToolActionPlan(
      ToolActionMaterializationPlanError::EmptyPreApplySha256 {
        path: plan.target_path.clone(),
      },
    ));
  }
  let request = ToolActionMaterializationRequest {
    apply_receipt_artifact_id: ctx.apply_receipt_artifact_id.to_string(),
    repo_snapshot_ref: ctx.repo_snapshot_ref.to_string(),
    capability: ctx.capability.to_string(),
    requested_by_actor_id: ctx.requested_by_actor_id.to_string(),
    requested_by_tenant_id: ctx.requested_by_tenant_id.to_string(),
    requested_at_ms: ctx.requested_at_ms,
    deployment_mode: ctx.deployment_mode.to_string(),
    content_policy: "include-content".to_string(),
  };
  let file_state = ApplyReceiptFileState {
    path: plan.target_path.clone(),
    pre_apply_sha256: plan.post_apply_sha256.clone(),
    post_apply_sha256: plan.pre_apply_sha256.clone(),
    post_apply_byte_len: pre_apply_content.len(),
    post_apply_content: Some(pre_apply_content.to_string()),
  };
  crate::tool_action::build_tool_action_materialization_plan(
    &request,
    ctx.apply_receipt_artifact_id,
    vec![file_state],
    allowed_target_paths,
  )
  .map_err(HotReloadMaterializationError::ToolActionPlan)
}

#[cfg(test)]
mod tests {
  use super::super::axis_separation_gate::check_axis_separation;
  use super::super::candidate_row_proposal::{CandidateKind, CandidateRowProposal, GateStatus};
  use super::super::macro_fold_gate::fold_proposal;
  use super::super::owner_law_gate::{
    candidate_fingerprint, process_owner_law, PromotionApproval, PromotionApprovalDecision,
  };
  use super::super::regression_proof_gate::check_regression_proof;
  use super::*;
  use std::collections::BTreeMap;

  fn promoted_candidate(
    held: &str,
    primary: &str,
    fallback: Option<&str>,
  ) -> OwnerLawProcessedCandidate {
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
    let folded = fold_proposal(&proposal);
    let axis = check_axis_separation(&folded);
    let reg = check_regression_proof(&axis, &[]);
    let approval = PromotionApproval {
      actor_id: "actor.operator".to_string(),
      tenant_id: "tenant.test".to_string(),
      approved_at_ms: 1700000000000,
      decision: PromotionApprovalDecision::Approve,
      candidate_fingerprint: candidate_fingerprint(&reg),
      ttl_ms: None,
      reason: None,
    };
    process_owner_law(&reg, Some(&approval), 1700000000000)
  }

  fn synthetic_held_to_query_file() -> String {
    // Minimal stand-in for the real `.px` file. Only the bits the
    // plan needs to recognize: the table declaration and the
    // closing `];`. Real-file integration is the responsibility of
    // a downstream caller (puck operator override) — Stage E itself
    // is pure-data and tested against synthetic content.
    r#"# header
let
  heldRoutingMap = [
    {
      held = "missing-import-spec";
      primary = "host-symbol-resolver";
      fallback = "external-knowledge-search";
    }
  ];
in {}
"#
    .to_string()
  }

  // ─── outcome universe ──────────────────────────────────────────

  #[test]
  fn every_outcome_has_a_string_form() {
    for o in HotReloadOutcome::ALL {
      assert!(!o.as_str().is_empty());
    }
  }

  // ─── not-promoted propagation ──────────────────────────────────

  #[test]
  fn not_promoted_holds() {
    // Build an OwnerLawProcessedCandidate with HeldAwaitingApproval.
    let mut row = BTreeMap::new();
    row.insert("held".to_string(), "h".to_string());
    row.insert("primary".to_string(), "host-symbol-resolver".to_string());
    let proposal = CandidateRowProposal {
      candidate_kind: CandidateKind::RecurringChannelSuccess,
      target_owner: "stdlib/lib/gate/algorithm-synthesis/held-to-query.px".to_string(),
      target_table: "heldRoutingMap".to_string(),
      proposed_row: row,
      supporting_evidence: vec![],
      evidence_count: 2,
      gate_status: GateStatus::IntentReceiptOnly,
      reason: "fixture".to_string(),
    };
    let reg = check_regression_proof(&check_axis_separation(&fold_proposal(&proposal)), &[]);
    // No approval supplied.
    let owner = process_owner_law(&reg, None, 1700000000000);
    let plan = plan_hot_reload(&owner, &synthetic_held_to_query_file());
    assert_eq!(plan.outcome, HotReloadOutcome::HeldNotPromoted);
    assert_eq!(plan.gate_status, GateStatus::Held);
    assert!(plan.post_apply_content.is_empty());
  }

  // ─── unknown target ───────────────────────────────────────────

  #[test]
  fn unknown_target_owner_holds() {
    let mut owner = promoted_candidate("future-kind", "host-symbol-resolver", None);
    // Mutate the target_owner to one we have no anchor for.
    owner.source.source.source.source.target_owner =
      "stdlib/lib/gate/does-not-exist.px".to_string();
    let plan = plan_hot_reload(&owner, &synthetic_held_to_query_file());
    assert_eq!(plan.outcome, HotReloadOutcome::HeldTargetFileUnknown);
  }

  // ─── anchor missing in file ───────────────────────────────────

  #[test]
  fn anchor_not_found_in_file_holds() {
    let owner = promoted_candidate("future-kind", "host-symbol-resolver", None);
    // File doesn't contain the table declaration at all.
    let plan = plan_hot_reload(&owner, "let in {}\n");
    assert_eq!(plan.outcome, HotReloadOutcome::HeldAnchorNotFound);
  }

  // ─── happy path ───────────────────────────────────────────────

  #[test]
  fn promoted_candidate_yields_plan_ready() {
    let owner = promoted_candidate(
      "future-kind",
      "host-symbol-resolver",
      Some("operator-followup"),
    );
    let pre = synthetic_held_to_query_file();
    let plan = plan_hot_reload(&owner, &pre);
    assert_eq!(plan.outcome, HotReloadOutcome::PlanReady);
    assert_eq!(plan.gate_status, GateStatus::Promoted);
    assert!(!plan.post_apply_content.is_empty());
    assert_ne!(plan.pre_apply_sha256, plan.post_apply_sha256);

    // Inserted row text appears in the post-apply content exactly once.
    assert!(plan.post_apply_content.contains(&plan.inserted_row_text));
    // The newly inserted row contains the candidate values.
    assert!(plan.post_apply_content.contains("future-kind"));
    assert!(plan.post_apply_content.contains("operator-followup"));
    // The pre-existing row is preserved verbatim.
    assert!(plan.post_apply_content.contains("missing-import-spec"));
  }

  #[test]
  fn post_apply_content_remains_brace_balanced() {
    let owner = promoted_candidate(
      "future-kind",
      "host-symbol-resolver",
      Some("operator-followup"),
    );
    let plan = plan_hot_reload(&owner, &synthetic_held_to_query_file());
    assert_eq!(plan.outcome, HotReloadOutcome::PlanReady);
    assert!(brace_balanced(&plan.post_apply_content));
  }

  #[test]
  fn pre_apply_sha256_matches_input_file() {
    let owner = promoted_candidate("future-kind", "host-symbol-resolver", None);
    let pre = synthetic_held_to_query_file();
    let plan = plan_hot_reload(&owner, &pre);
    assert_eq!(plan.pre_apply_sha256, sha256_hex(&pre));
  }

  #[test]
  fn post_apply_sha256_matches_planned_content() {
    let owner = promoted_candidate("future-kind", "host-symbol-resolver", None);
    let plan = plan_hot_reload(&owner, &synthetic_held_to_query_file());
    assert_eq!(plan.outcome, HotReloadOutcome::PlanReady);
    assert_eq!(plan.post_apply_sha256, sha256_hex(&plan.post_apply_content));
  }

  // ─── audit chain preservation ─────────────────────────────────

  #[test]
  fn plan_carries_full_gate_chain_for_audit() {
    let owner = promoted_candidate("future-kind", "host-symbol-resolver", None);
    let plan = plan_hot_reload(&owner, &synthetic_held_to_query_file());
    assert_eq!(plan.outcome, HotReloadOutcome::PlanReady);
    // Walk back from plan all the way to the original proposal.
    assert_eq!(plan.source.outcome, OwnerLawOutcome::Promoted);
    let original_proposal = &plan.source.source.source.source.source;
    assert_eq!(
      original_proposal.candidate_kind,
      CandidateKind::RecurringChannelSuccess
    );
    assert_eq!(original_proposal.evidence_count, 2);
  }

  // ─── brace-balance helper ────────────────────────────────────

  #[test]
  fn brace_balanced_accepts_balanced_input() {
    assert!(brace_balanced("{ [ ] }"));
    assert!(brace_balanced("let x = [ 1 2 3 ]; in { y = x; }"));
  }

  #[test]
  fn brace_balanced_rejects_unbalanced() {
    assert!(!brace_balanced("{ ["));
    assert!(!brace_balanced("} {"));
  }

  // ─── v1: real Nix parse check ─────────────────────────────────

  #[test]
  fn nix_parse_check_accepts_valid_attrset() {
    assert!(nix_parse_check("{ a = 1; b = 2; }").is_ok());
  }

  #[test]
  fn nix_parse_check_accepts_let_in_with_attrset() {
    // The synthetic `.px` shape that Stage E plans for.
    let text = r#"let
  heldRoutingMap = [
    { held = "x"; primary = "y"; }
  ];
in {}
"#;
    assert!(nix_parse_check(text).is_ok());
  }

  #[test]
  fn nix_parse_check_rejects_brace_balanced_but_invalid_nix() {
    // brace-balanced but missing `=` between key and value —
    // not valid Nix. v0 brace_balanced returns true; v1
    // nix_parse_check rejects.
    let bad = "{ key value }";
    assert!(brace_balanced(bad), "brace-balanced (v0 would accept)");
    assert!(nix_parse_check(bad).is_err(), "real parser rejects");
  }

  #[test]
  fn nix_parse_check_rejects_unterminated_string() {
    // brace-balanced but unterminated string literal.
    let bad = "{ x = \"unterminated; }";
    assert!(nix_parse_check(bad).is_err());
  }

  // ─── gate-chain projection (cockpit timeline) ────────────────

  #[test]
  fn artifact_carries_gate_chain_six_entries_in_order() {
    let owner = promoted_candidate("future-kind", "host-symbol-resolver", None);
    let plan = plan_hot_reload(&owner, &synthetic_held_to_query_file());
    let art = build_hot_reload_plan_artifact(&plan, 1700000000000, None);
    let chain = art["gate_chain"].as_array().expect("gate_chain array");
    assert_eq!(chain.len(), 6, "exactly 6 entries: G1..G5 + Stage E");
    let names: Vec<&str> = chain
      .iter()
      .map(|e| e["name"].as_str().unwrap_or(""))
      .collect();
    assert_eq!(
      names,
      vec![
        "candidate-row-proposal",
        "macro-fold",
        "axis-separation",
        "regression-proof",
        "owner-law",
        "stage-e-hot-reload-plan",
      ]
    );
    let indices: Vec<u64> = chain
      .iter()
      .map(|e| e["index"].as_u64().unwrap_or(0))
      .collect();
    assert_eq!(indices, vec![1, 2, 3, 4, 5, 6]);
  }

  #[test]
  fn artifact_gate_chain_promoted_plan_marks_all_six_green() {
    let owner = promoted_candidate("future-kind", "host-symbol-resolver", None);
    let plan = plan_hot_reload(&owner, &synthetic_held_to_query_file());
    assert_eq!(plan.outcome, HotReloadOutcome::PlanReady);
    let art = build_hot_reload_plan_artifact(&plan, 0, None);
    let chain = art["gate_chain"].as_array().unwrap();
    assert_eq!(chain[0]["outcome"], "proposed");
    assert_eq!(chain[1]["outcome"], "folded");
    assert_eq!(chain[2]["outcome"], "axis-verified");
    assert_eq!(chain[3]["outcome"], "regression-proven");
    assert_eq!(chain[4]["outcome"], "promoted");
    assert_eq!(chain[5]["outcome"], "plan-ready");
  }

  #[test]
  fn artifact_gate_chain_g1_carries_candidate_kind_and_evidence_count() {
    let owner = promoted_candidate("future-kind", "host-symbol-resolver", None);
    let plan = plan_hot_reload(&owner, &synthetic_held_to_query_file());
    let art = build_hot_reload_plan_artifact(&plan, 0, None);
    let g1 = &art["gate_chain"][0];
    assert_eq!(g1["extra"]["candidate_kind"], "recurring-channel-success");
    assert_eq!(g1["extra"]["evidence_count"], 2);
    assert_eq!(g1["extra"]["target_table"], "heldRoutingMap");
  }

  #[test]
  fn artifact_id_unchanged_by_gate_chain_addition() {
    // Adding `gate_chain` to the payload must NOT shift the
    // replay-stable id — id covers intrinsic identity (target_path
    // + pre/post sha + outcome + inserted_row_text) only.
    let owner = promoted_candidate("future-kind", "host-symbol-resolver", None);
    let plan = plan_hot_reload(&owner, &synthetic_held_to_query_file());
    let art = build_hot_reload_plan_artifact(&plan, 1, None);

    // Re-derive the expected id locally with the same intrinsic
    // recipe used inside build_hot_reload_plan_artifact.
    let mut h = Sha256::new();
    h.update(b"hot-reload-plan\x1f");
    h.update(plan.target_path.as_bytes());
    h.update(b"\x1f");
    h.update(plan.pre_apply_sha256.as_bytes());
    h.update(b"\x1f");
    h.update(plan.post_apply_sha256.as_bytes());
    h.update(b"\x1f");
    h.update(plan.outcome.as_str().as_bytes());
    h.update(b"\x1f");
    h.update(plan.inserted_row_text.as_bytes());
    let digest = h.finalize();
    let prefix = digest
      .iter()
      .take(16)
      .map(|b| format!("{b:02x}"))
      .collect::<String>();
    let expected = format!("hot-reload-plan.{prefix}");
    assert_eq!(art["id"].as_str().unwrap(), expected);
  }
}
