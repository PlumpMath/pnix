//! Tool-action materialization gate — the typed bridge between a
//! deterministic apply-receipt and the actual filesystem write.
//!
//! OWNER-LAW (2026-05-11): pnix is an LLM-independent deterministic AI
//! substrate. The `code_transform` module produces *pure-data* apply
//! receipts (`per_file_after` is in-memory, no I/O). This module owns
//! the **typed gate** that decides whether an apply-receipt may be
//! materialized to disk. The actual `std::fs::write` lives in a host
//! crate (doghouse-core or a downstream cockpit); this layer is the
//! gatekeeper.
//!
//! Canonical chain extension:
//!
//! ```text
//!   patch candidate
//!     → review receipt
//!     → apply receipt        (per_file_after is in-memory)
//!     → MATERIALIZATION REQUEST (this module)
//!     → classify → Ready / Held / Rejected
//!     → MATERIALIZATION PLAN (next slice)
//!     → host disk write     (effectful — host owns)
//!     → MATERIALIZATION RECEIPT (next slice)
//! ```
//!
//! The classifier in this module checks the **seven preconditions**
//! identified by the project roadmap:
//!
//!   1. `apply_receipt_artifact_id` non-empty + shape valid
//!   2. `repo_snapshot_ref` non-empty (git sha or equivalent —
//!      binds the materialization to a known repo state)
//!   3. `requested_by_actor_id` non-empty
//!   4. `requested_by_tenant_id` non-empty
//!   5. `capability` is one of the recognized kebab strings
//!      (authoritative here; a host-side typed mirror enum was
//!      absorbed into pnixc-meta and removed)
//!   6. `deployment_mode` is one of `dev` / `operator` /
//!      `customer-release`
//!   7. `content_policy` consistency with `deployment_mode` —
//!      `customer-release + include-content → Rejected`
//!
//! These are pure preconditions (no I/O). The pre-apply file hash
//! check (precondition 5 from the project plan) and the
//! target-paths-out-of-allowed check (precondition 6 from the plan)
//! happen in the materialization-plan layer (next slice) — they need
//! the apply receipt's per-file sha256 metadata and a
//! capability→path-policy translation that the host owns.

use serde::{Deserialize, Serialize};
use pnix_hash::{Digest, Sha256};

/// Recognized deployment modes — same kebab-case strings as
/// `doghouse-core::code_transform_artifact::DeploymentMode`.
///
/// This module keeps the value as a string so pnix-core doesn't have
/// to depend on doghouse-core. The host (which sits above both) is
/// responsible for serializing its enum into this string.
const DEPLOYMENT_MODES: &[&str] = &["dev", "operator", "customer-release"];

/// Recognized content-policy strings — same kebab-case as
/// `ApplyReceiptContentPolicy::as_str`.
const CONTENT_POLICIES: &[&str] = &["include-content", "omit-content"];

/// Recognized capability strings — kebab-case, authoritative here.
/// (A host-side typed `CodeEditCapability` mirror enum once shadowed
/// these strings; it was absorbed into pnixc-meta and removed.)
///
/// The closed enumeration of capability variants:
/// `read-only`, `edit-within-target-paths`, `edit-test-only`,
/// `edit-generated`, `edit-ci-config`, `edit-px-owner-law-substrate`,
/// `forbidden`. `read-only` and `forbidden` are listed so the
/// classifier can detect "this caller requested a capability that
/// exists but is wrong for materialization" (read-only never
/// materializes; forbidden is a deny-stub).
const CAPABILITIES: &[&str] = &[
  "read-only",
  "edit-within-target-paths",
  "edit-test-only",
  "edit-generated",
  "edit-ci-config",
  "edit-px-owner-law-substrate",
  "forbidden",
];

/// A request to materialize an apply-receipt's `per_file_after`
/// content onto the filesystem.
///
/// OWNER-LAW (2026-05-11): pure data. The classifier downstream
/// (`classify_tool_action_materialization_request`) decides whether
/// this request is well-formed; the plan + executor in later slices
/// decide whether the actual writes are safe.
///
/// Field semantics:
///
///   - `apply_receipt_artifact_id`: the replay-stable id from the
///     pnix-core code-transform apply-receipt artifact builder
///     (e.g. `apply-receipt.rename-symbol.<sha256>`). The host binds
///     this materialization to that exact apply event.
///   - `repo_snapshot_ref`: a git sha or equivalent identifier of the
///     repo state at apply time. The materialization-plan layer will
///     refuse to write when the current repo state has drifted.
///   - `capability`: kebab-case `CodeEditCapability` string. Host
///     interprets to decide which paths are writable.
///   - `requested_by_actor_id` / `_tenant_id`: who's asking. Empty =
///     refuse (no anonymous materialization).
///   - `requested_at_ms`: wall-clock timestamp at request time.
///   - `deployment_mode`: `dev` / `operator` / `customer-release`.
///   - `content_policy`: `include-content` / `omit-content`. Must be
///     consistent with `deployment_mode` — `customer-release` cannot
///     pair with `include-content` (customer artifact stores must not
///     embed file content).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolActionMaterializationRequest {
  pub apply_receipt_artifact_id: String,
  pub repo_snapshot_ref: String,
  pub capability: String,
  pub requested_by_actor_id: String,
  pub requested_by_tenant_id: String,
  pub requested_at_ms: u64,
  pub deployment_mode: String,
  pub content_policy: String,
}

/// Held / Rejected outcome kinds from
/// [`classify_tool_action_materialization_request`].
///
/// OWNER-LAW (2026-05-11): each variant maps 1:1 to a kebab-case
/// string. The Held vs Rejected distinction follows the same pattern
/// as `RenameVerdict` / `RemoveUnusedImportVerdict`:
///
///   - **Held**: the request is well-shaped but a precondition is
///     deferred (e.g. user hasn't supplied an actor yet; auditor
///     wants more evidence).
///   - **Rejected**: the request violates a hard rule and cannot
///     proceed (e.g. capability is `forbidden`; customer-release +
///     include-content).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ToolActionMaterializationHeldKind {
  /// `apply_receipt_artifact_id` is empty.
  MissingApplyReceiptArtifactId,
  /// `apply_receipt_artifact_id` doesn't look like a canonical
  /// pnix-core receipt id (must start with `apply-receipt.`).
  MalformedApplyReceiptArtifactId,
  /// `repo_snapshot_ref` is empty — must bind to a known repo state.
  MissingRepoSnapshotRef,
  /// `requested_by_actor_id` is empty.
  MissingRequestedByActor,
  /// `requested_by_tenant_id` is empty.
  MissingRequestedByTenant,
  /// `capability` is empty.
  MissingCapability,
  /// `capability` is not one of the recognized
  /// `CodeEditCapability` kebab-case strings.
  UnrecognizedCapability,
  /// `capability` is `read-only` — read-only never materializes.
  ReadOnlyCapabilityCannotMaterialize,
  /// `capability` is `forbidden` — deny-stub, never materializes.
  ForbiddenCapability,
  /// `deployment_mode` is empty.
  MissingDeploymentMode,
  /// `deployment_mode` is not one of `dev` / `operator` /
  /// `customer-release`.
  UnrecognizedDeploymentMode,
  /// `content_policy` is empty.
  MissingContentPolicy,
  /// `content_policy` is not one of `include-content` /
  /// `omit-content`.
  UnrecognizedContentPolicy,
  /// `customer-release` + `include-content` is forbidden — customer
  /// artifact stores must not embed file content.
  CustomerReleaseForbidsIncludeContent,
}

impl ToolActionMaterializationHeldKind {
  pub fn as_str(self) -> &'static str {
    match self {
      Self::MissingApplyReceiptArtifactId => "missing-apply-receipt-artifact-id",
      Self::MalformedApplyReceiptArtifactId => "malformed-apply-receipt-artifact-id",
      Self::MissingRepoSnapshotRef => "missing-repo-snapshot-ref",
      Self::MissingRequestedByActor => "missing-requested-by-actor",
      Self::MissingRequestedByTenant => "missing-requested-by-tenant",
      Self::MissingCapability => "missing-capability",
      Self::UnrecognizedCapability => "unrecognized-capability",
      Self::ReadOnlyCapabilityCannotMaterialize => "read-only-capability-cannot-materialize",
      Self::ForbiddenCapability => "forbidden-capability",
      Self::MissingDeploymentMode => "missing-deployment-mode",
      Self::UnrecognizedDeploymentMode => "unrecognized-deployment-mode",
      Self::MissingContentPolicy => "missing-content-policy",
      Self::UnrecognizedContentPolicy => "unrecognized-content-policy",
      Self::CustomerReleaseForbidsIncludeContent => "customer-release-forbids-include-content",
    }
  }
}

/// Verdict from [`classify_tool_action_materialization_request`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "verdict")]
pub enum ToolActionMaterializationVerdict {
  /// All preconditions pass — proceed to plan stage.
  Ready,
  /// Precondition deferred — caller can fix and retry.
  Held {
    held_kind: ToolActionMaterializationHeldKind,
    reason: String,
  },
  /// Hard rule violation — caller must restructure the request.
  Rejected {
    held_kind: ToolActionMaterializationHeldKind,
    reason: String,
  },
}

/// Pure classifier — applies the seven preconditions in order.
///
/// OWNER-LAW (2026-05-11): the ladder ordering is deterministic and
/// chosen so the most "fixable" preconditions surface first (empty
/// fields), graduating to "wrong value" preconditions (unrecognized
/// enum) and ending with "structural rule violation"
/// (customer-release + include-content) which is `Rejected` rather
/// than `Held`.
///
/// Ladder order:
///
///   1. `apply_receipt_artifact_id` empty? → Held
///   2. `apply_receipt_artifact_id` malformed? → Held
///   3. `repo_snapshot_ref` empty? → Held
///   4. `requested_by_actor_id` empty? → Held
///   5. `requested_by_tenant_id` empty? → Held
///   6. `capability` empty? → Held
///   7. `capability` unrecognized? → Held
///   8. `capability == read-only` → Held
///      (caller asked for a capability that never writes)
///   9. `capability == forbidden` → Rejected
///      (deny-stub — must restructure to a real capability)
///  10. `deployment_mode` empty? → Held
///  11. `deployment_mode` unrecognized? → Held
///  12. `content_policy` empty? → Held
///  13. `content_policy` unrecognized? → Held
///  14. `customer-release + include-content` → Rejected
///
/// Anything else → `Ready`.
pub fn classify_tool_action_materialization_request(
  req: &ToolActionMaterializationRequest,
) -> ToolActionMaterializationVerdict {
  if req.apply_receipt_artifact_id.is_empty() {
    return ToolActionMaterializationVerdict::Held {
      held_kind: ToolActionMaterializationHeldKind::MissingApplyReceiptArtifactId,
      reason:
        "apply_receipt_artifact_id required to bind the materialization to a specific apply event"
          .to_string(),
    };
  }
  if !req.apply_receipt_artifact_id.starts_with("apply-receipt.") {
    return ToolActionMaterializationVerdict::Held {
      held_kind: ToolActionMaterializationHeldKind::MalformedApplyReceiptArtifactId,
      reason: format!(
        "apply_receipt_artifact_id '{}' does not start with 'apply-receipt.' — must be a canonical pnix-core code-transform receipt id",
        req.apply_receipt_artifact_id
      ),
    };
  }
  if req.repo_snapshot_ref.is_empty() {
    return ToolActionMaterializationVerdict::Held {
      held_kind: ToolActionMaterializationHeldKind::MissingRepoSnapshotRef,
      reason: "repo_snapshot_ref required to bind the materialization to a known repo state (drift defense)".to_string(),
    };
  }
  if req.requested_by_actor_id.is_empty() {
    return ToolActionMaterializationVerdict::Held {
      held_kind: ToolActionMaterializationHeldKind::MissingRequestedByActor,
      reason: "requested_by_actor_id required — materialization is not anonymous".to_string(),
    };
  }
  if req.requested_by_tenant_id.is_empty() {
    return ToolActionMaterializationVerdict::Held {
      held_kind: ToolActionMaterializationHeldKind::MissingRequestedByTenant,
      reason: "requested_by_tenant_id required — materialization must be tenant-scoped".to_string(),
    };
  }
  if req.capability.is_empty() {
    return ToolActionMaterializationVerdict::Held {
      held_kind: ToolActionMaterializationHeldKind::MissingCapability,
      reason: "capability required — host gate cannot evaluate which paths are writable"
        .to_string(),
    };
  }
  if !CAPABILITIES.contains(&req.capability.as_str()) {
    return ToolActionMaterializationVerdict::Held {
      held_kind: ToolActionMaterializationHeldKind::UnrecognizedCapability,
      reason: format!(
        "capability '{}' is not one of the recognized CodeEditCapability strings ({:?})",
        req.capability, CAPABILITIES
      ),
    };
  }
  if req.capability == "read-only" {
    return ToolActionMaterializationVerdict::Held {
      held_kind: ToolActionMaterializationHeldKind::ReadOnlyCapabilityCannotMaterialize,
      reason: "capability=read-only cannot materialize — choose edit-within-target-paths / edit-test-only / edit-generated / edit-ci-config instead".to_string(),
    };
  }
  if req.capability == "forbidden" {
    return ToolActionMaterializationVerdict::Rejected {
      held_kind: ToolActionMaterializationHeldKind::ForbiddenCapability,
      reason: "capability=forbidden is a deny-stub — materialization request cannot use it"
        .to_string(),
    };
  }
  if req.deployment_mode.is_empty() {
    return ToolActionMaterializationVerdict::Held {
      held_kind: ToolActionMaterializationHeldKind::MissingDeploymentMode,
      reason: "deployment_mode required (dev / operator / customer-release)".to_string(),
    };
  }
  if !DEPLOYMENT_MODES.contains(&req.deployment_mode.as_str()) {
    return ToolActionMaterializationVerdict::Held {
      held_kind: ToolActionMaterializationHeldKind::UnrecognizedDeploymentMode,
      reason: format!(
        "deployment_mode '{}' is not one of {:?}",
        req.deployment_mode, DEPLOYMENT_MODES
      ),
    };
  }
  if req.content_policy.is_empty() {
    return ToolActionMaterializationVerdict::Held {
      held_kind: ToolActionMaterializationHeldKind::MissingContentPolicy,
      reason: "content_policy required (include-content / omit-content)".to_string(),
    };
  }
  if !CONTENT_POLICIES.contains(&req.content_policy.as_str()) {
    return ToolActionMaterializationVerdict::Held {
      held_kind: ToolActionMaterializationHeldKind::UnrecognizedContentPolicy,
      reason: format!(
        "content_policy '{}' is not one of {:?}",
        req.content_policy, CONTENT_POLICIES
      ),
    };
  }
  if req.deployment_mode == "customer-release" && req.content_policy == "include-content" {
    return ToolActionMaterializationVerdict::Rejected {
      held_kind: ToolActionMaterializationHeldKind::CustomerReleaseForbidsIncludeContent,
      reason: "customer-release deployment_mode forbids include-content — file content body must not embed in customer-facing artifact stores".to_string(),
    };
  }
  ToolActionMaterializationVerdict::Ready
}

// ─── review + apply → materialization request bridge ─────────────────

/// Transform-agnostic inputs for
/// [`bridge_review_apply_to_materialization_request`].
///
/// OWNER-LAW (2026-05-11): the bridge is the typed gate between the
/// canonical chain's pure-data stages (review-receipt, apply-receipt)
/// and the host-side materialization lane. Per-transform wrappers
/// (e.g. `build_rename_materialization_request`) compute the
/// transform-specific id strings and forward them through this
/// struct.
///
/// All fields are `&str` slices (and one `u64`) so the caller decides
/// who owns the strings. The bridge never holds references past
/// return.
#[derive(Debug, Clone, Copy)]
pub struct MaterializationBridgeInputs<'a> {
  /// kebab-case review decision (`"approve"` / `"hold"` / `"reject"`).
  /// Bridge fails-closed unless this is `"approve"`.
  pub review_decision: &'a str,
  /// The candidate id the review pinned at review time.
  pub review_candidate_artifact_id: &'a str,
  /// The reviewer's tenant. Bridge requires
  /// `review_reviewer_tenant_id == apply_approval_tenant_id` —
  /// cross-tenant review/apply chains are suspicious. Actor may
  /// differ (a senior reviews; a junior applies).
  pub review_reviewer_tenant_id: &'a str,
  /// The candidate id the apply receipt was built from (re-derived
  /// by the caller from `apply.candidate`). Must match
  /// `review_candidate_artifact_id` — TOCTOU.
  pub apply_candidate_artifact_id: &'a str,
  /// The apply-receipt's own id. Becomes the materialization
  /// request's `apply_receipt_artifact_id` field after classifier
  /// validation.
  pub apply_receipt_artifact_id: &'a str,
  /// The approver of the apply event. Becomes the materialization
  /// request's `requested_by_actor_id`.
  pub apply_approval_actor_id: &'a str,
  /// The approver's tenant. Must match
  /// `review_reviewer_tenant_id`.
  pub apply_approval_tenant_id: &'a str,
  /// Materialization context — caller-provided.
  pub capability: &'a str,
  pub repo_snapshot_ref: &'a str,
  pub deployment_mode: &'a str,
  pub content_policy: &'a str,
  pub requested_at_ms: u64,
}

/// Errors from
/// [`bridge_review_apply_to_materialization_request`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaterializationBridgeError {
  /// The review decision was not `"approve"`. Carries the actual
  /// decision string so the caller can diagnose.
  ReviewNotApproved { decision: String },
  /// The review pinned a different candidate than the apply receipt
  /// was built from. TOCTOU break — fail-closed.
  ReviewCandidateMismatchesApplyCandidate {
    review_candidate_artifact_id: String,
    apply_candidate_artifact_id: String,
  },
  /// The reviewer and the apply approver are in different tenants.
  /// Cross-tenant review/apply chains aren't permitted.
  ReviewTenantMismatchesApplyTenant {
    review_tenant: String,
    apply_tenant: String,
  },
  /// The request was assembled but the classifier didn't return
  /// `Ready`. Carries the classifier's verdict (Held or Rejected)
  /// so the caller knows which precondition to fix.
  RequestNotReady(ToolActionMaterializationVerdict),
}

impl std::fmt::Display for MaterializationBridgeError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::ReviewNotApproved { decision } => write!(
        f,
        "review decision is '{decision}', not 'approve' — materialization request can only be built from approved reviews"
      ),
      Self::ReviewCandidateMismatchesApplyCandidate {
        review_candidate_artifact_id,
        apply_candidate_artifact_id,
      } => write!(
        f,
        "review pinned candidate '{review_candidate_artifact_id}' but apply receipt was built from '{apply_candidate_artifact_id}' — TOCTOU break"
      ),
      Self::ReviewTenantMismatchesApplyTenant {
        review_tenant,
        apply_tenant,
      } => write!(
        f,
        "reviewer tenant '{review_tenant}' does not match apply approver tenant '{apply_tenant}' — cross-tenant review/apply chains not permitted"
      ),
      Self::RequestNotReady(verdict) => write!(
        f,
        "assembled request did not classify as Ready: {:?}",
        verdict
      ),
    }
  }
}

impl std::error::Error for MaterializationBridgeError {}

/// Build a `ToolActionMaterializationRequest` from a review receipt
/// and an apply receipt's identity fields, verifying every TOCTOU
/// and policy gate.
///
/// OWNER-LAW (2026-05-11): preconditions verified in order:
///
///   1. `review_decision == "approve"` — Hold and Reject can't
///      authorize a write to disk.
///   2. `review_candidate_artifact_id == apply_candidate_artifact_id`
///      — TOCTOU: the apply MUST have been built from the EXACT
///      candidate the review approved.
///   3. `review_reviewer_tenant_id == apply_approval_tenant_id` —
///      same-tenant review and apply. Actor can differ.
///   4. The assembled request passes
///      [`classify_tool_action_materialization_request`] (which
///      enforces the 14-step ladder: apply_receipt_id shape +
///      repo_snapshot_ref + actor + tenant + capability +
///      deployment_mode + content_policy consistency).
///
/// On success the caller has a Ready'd request that can be fed to
/// [`build_tool_action_materialization_plan`] and then to the host
/// executor.
pub fn bridge_review_apply_to_materialization_request(
  inputs: &MaterializationBridgeInputs<'_>,
) -> Result<ToolActionMaterializationRequest, MaterializationBridgeError> {
  // 1. Review decision must be approve.
  if inputs.review_decision != "approve" {
    return Err(MaterializationBridgeError::ReviewNotApproved {
      decision: inputs.review_decision.to_string(),
    });
  }
  // 2. Candidate identity TOCTOU.
  if inputs.review_candidate_artifact_id != inputs.apply_candidate_artifact_id {
    return Err(
      MaterializationBridgeError::ReviewCandidateMismatchesApplyCandidate {
        review_candidate_artifact_id: inputs.review_candidate_artifact_id.to_string(),
        apply_candidate_artifact_id: inputs.apply_candidate_artifact_id.to_string(),
      },
    );
  }
  // 3. Tenant identity TOCTOU.
  if inputs.review_reviewer_tenant_id != inputs.apply_approval_tenant_id {
    return Err(
      MaterializationBridgeError::ReviewTenantMismatchesApplyTenant {
        review_tenant: inputs.review_reviewer_tenant_id.to_string(),
        apply_tenant: inputs.apply_approval_tenant_id.to_string(),
      },
    );
  }
  // 4. Build the request and classify.
  let request = ToolActionMaterializationRequest {
    apply_receipt_artifact_id: inputs.apply_receipt_artifact_id.to_string(),
    repo_snapshot_ref: inputs.repo_snapshot_ref.to_string(),
    capability: inputs.capability.to_string(),
    requested_by_actor_id: inputs.apply_approval_actor_id.to_string(),
    requested_by_tenant_id: inputs.apply_approval_tenant_id.to_string(),
    requested_at_ms: inputs.requested_at_ms,
    deployment_mode: inputs.deployment_mode.to_string(),
    content_policy: inputs.content_policy.to_string(),
  };
  match classify_tool_action_materialization_request(&request) {
    ToolActionMaterializationVerdict::Ready => Ok(request),
    other => Err(MaterializationBridgeError::RequestNotReady(other)),
  }
}

// ─── materialization plan ─────────────────────────────────────────────

/// Per-file state needed to plan a materialization write.
///
/// OWNER-LAW (2026-05-11): the host (which has filesystem access)
/// gathers these by:
///
///   1. Reading the current on-disk file → `pre_apply_sha256`. This
///      is the expected state at write time; the executor will refuse
///      to write if the actual disk file's sha256 has drifted away
///      from this value (drift defense).
///   2. Reading the apply-receipt's `per_file_after[*]` →
///      `post_apply_sha256` + `post_apply_byte_len` (always) and
///      `post_apply_content` (only when the deployment mode +
///      content policy permits embedding the body).
///
/// The plan stays transform-agnostic — it doesn't know about
/// `RenameApplyReceipt` or `RemoveUnusedImportApplyReceipt`. The host
/// extracts these fields from whichever apply-receipt shape and feeds
/// the result here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyReceiptFileState {
  pub path: String,
  /// SHA-256 of the file's content BEFORE apply (what the executor
  /// expects to see on disk when it runs). Empty = host couldn't
  /// compute it (rejected at plan time).
  pub pre_apply_sha256: String,
  /// SHA-256 of the file's content AFTER apply (the target the write
  /// must produce). Empty = malformed input (rejected at plan time).
  pub post_apply_sha256: String,
  /// Length in bytes of the post-apply content.
  pub post_apply_byte_len: usize,
  /// Full post-apply content. `Some(_)` when `content_policy ==
  /// include-content`; `None` when `content_policy == omit-content`.
  /// The executor must have *some* way to recover the body to write;
  /// when `None`, the host is expected to derive it from the apply
  /// receipt at write time (effectively re-deriving from the
  /// candidate's edits — keeps the plan small + customer-release
  /// safe).
  pub post_apply_content: Option<String>,
}

/// A plan to materialize an apply-receipt's per-file content onto
/// disk. Produced by [`build_tool_action_materialization_plan`] after
/// the request has been classified Ready and target-path /
/// content-policy consistency has been checked.
///
/// OWNER-LAW (2026-05-11): pure data. No I/O. The host executor
/// consumes this plan, runs the actual disk writes, and emits a
/// receipt in the next sub-slice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolActionMaterializationPlan {
  pub request: ToolActionMaterializationRequest,
  /// Re-stated from the request so the plan is self-contained for
  /// audit purposes (the host writes this plan into doghouse).
  pub apply_receipt_artifact_id: String,
  /// Per-file write targets. Already constrained to
  /// `allowed_target_paths` at plan time.
  pub file_states: Vec<ApplyReceiptFileState>,
}

/// Errors from [`build_tool_action_materialization_plan`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolActionMaterializationPlanError {
  /// Caller passed a request that didn't classify as Ready. Carries
  /// the verdict the classifier returned so the caller can fix it.
  RequestNotReady(ToolActionMaterializationVerdict),
  /// Caller's `apply_receipt_artifact_id` argument doesn't match
  /// `request.apply_receipt_artifact_id`. TOCTOU guard — the request
  /// pins one specific apply event, and the file states must belong
  /// to that apply.
  ApplyReceiptIdMismatch { expected: String, got: String },
  /// `file_states` is empty — nothing to materialize.
  EmptyFileStates,
  /// A file's path is not in `allowed_target_paths`. Capability gate
  /// — the caller asked to write outside the declared scope.
  TargetPathNotAllowed { path: String },
  /// A file has an empty `pre_apply_sha256`. Host failed to read the
  /// current disk state, drift defense can't proceed.
  EmptyPreApplySha256 { path: String },
  /// A file has an empty `post_apply_sha256`. Malformed input.
  EmptyPostApplySha256 { path: String },
  /// `content_policy == include-content` but a file has `None`
  /// content body. Plan can't materialize without the bytes.
  ContentPolicyIncludeContentMissingBody { path: String },
  /// `content_policy == omit-content` but a file has `Some(_)`
  /// content body. Caller leaked the body through when policy said
  /// drop it — fail-closed.
  ContentPolicyOmitContentSuppliedBody { path: String },
  /// `file_states` has duplicate paths (two entries for the same
  /// file). Plan must be unambiguous.
  DuplicateFilePath { path: String },
}

impl std::fmt::Display for ToolActionMaterializationPlanError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::RequestNotReady(verdict) => write!(
        f,
        "request did not classify as Ready: {:?}",
        verdict
      ),
      Self::ApplyReceiptIdMismatch { expected, got } => write!(
        f,
        "apply_receipt_artifact_id mismatch: request='{expected}', argument='{got}'"
      ),
      Self::EmptyFileStates => write!(f, "file_states is empty — nothing to materialize"),
      Self::TargetPathNotAllowed { path } => write!(
        f,
        "path '{path}' is not in allowed_target_paths (capability gate)"
      ),
      Self::EmptyPreApplySha256 { path } => {
        write!(f, "pre_apply_sha256 is empty for path '{path}' (drift defense input missing)")
      }
      Self::EmptyPostApplySha256 { path } => {
        write!(f, "post_apply_sha256 is empty for path '{path}' (write target missing)")
      }
      Self::ContentPolicyIncludeContentMissingBody { path } => write!(
        f,
        "content_policy=include-content but file '{path}' has no content body"
      ),
      Self::ContentPolicyOmitContentSuppliedBody { path } => write!(
        f,
        "content_policy=omit-content but file '{path}' was supplied with a content body (fail-closed: customer-release leak guard)"
      ),
      Self::DuplicateFilePath { path } => {
        write!(f, "file_states has duplicate entry for path '{path}'")
      }
    }
  }
}

impl std::error::Error for ToolActionMaterializationPlanError {}

/// Build a materialization plan from a classified-Ready request and
/// host-supplied per-file state.
///
/// OWNER-LAW (2026-05-11): the preconditions enforced here are
/// numbers 5 (pre-apply file hash recorded) and 6 (target_paths-out-
/// of-allowed check) from the project roadmap. Number 5 records
/// *expected* hash; the actual hash comparison happens at write time
/// in the executor (next sub-slice).
///
/// Preconditions checked in order:
///
///   1. Request classifies as `Ready` (else `RequestNotReady`).
///   2. `apply_receipt_artifact_id` arg matches the request's
///      (TOCTOU — the request pins one apply event).
///   3. `file_states` non-empty.
///   4. No duplicate `path` across `file_states`.
///   5. Every `file_states[i].path` is in `allowed_target_paths`.
///   6. Every `file_states[i].pre_apply_sha256` non-empty.
///   7. Every `file_states[i].post_apply_sha256` non-empty.
///   8. Content-policy consistency:
///      - `include-content` + `post_apply_content == None` → reject
///      - `omit-content` + `post_apply_content == Some(_)` → reject
///        (fail-closed — caller can't leak content past the policy)
pub fn build_tool_action_materialization_plan(
  request: &ToolActionMaterializationRequest,
  apply_receipt_artifact_id: &str,
  file_states: Vec<ApplyReceiptFileState>,
  allowed_target_paths: &[String],
) -> Result<ToolActionMaterializationPlan, ToolActionMaterializationPlanError> {
  // 1. Re-classify — defense in depth. Caller might pass a request
  //    they constructed without classifying.
  let verdict = classify_tool_action_materialization_request(request);
  if !matches!(verdict, ToolActionMaterializationVerdict::Ready) {
    return Err(ToolActionMaterializationPlanError::RequestNotReady(verdict));
  }
  // 2. TOCTOU: caller's arg must match the request.
  if request.apply_receipt_artifact_id != apply_receipt_artifact_id {
    return Err(ToolActionMaterializationPlanError::ApplyReceiptIdMismatch {
      expected: request.apply_receipt_artifact_id.clone(),
      got: apply_receipt_artifact_id.to_string(),
    });
  }
  // 3. Non-empty file list.
  if file_states.is_empty() {
    return Err(ToolActionMaterializationPlanError::EmptyFileStates);
  }
  // 4. No duplicate paths. Single linear scan with a BTreeSet — order
  //    is deterministic (returns the FIRST duplicate path).
  let mut seen: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
  for fs in &file_states {
    if !seen.insert(fs.path.as_str()) {
      return Err(ToolActionMaterializationPlanError::DuplicateFilePath {
        path: fs.path.clone(),
      });
    }
  }
  // 5. Path-allowed check + 6 + 7 + 8 — single pass per file.
  let allowed: std::collections::BTreeSet<&str> =
    allowed_target_paths.iter().map(|s| s.as_str()).collect();
  for fs in &file_states {
    if !allowed.contains(fs.path.as_str()) {
      return Err(ToolActionMaterializationPlanError::TargetPathNotAllowed {
        path: fs.path.clone(),
      });
    }
    if fs.pre_apply_sha256.is_empty() {
      return Err(ToolActionMaterializationPlanError::EmptyPreApplySha256 {
        path: fs.path.clone(),
      });
    }
    if fs.post_apply_sha256.is_empty() {
      return Err(ToolActionMaterializationPlanError::EmptyPostApplySha256 {
        path: fs.path.clone(),
      });
    }
    match (request.content_policy.as_str(), &fs.post_apply_content) {
      ("include-content", None) => {
        return Err(
          ToolActionMaterializationPlanError::ContentPolicyIncludeContentMissingBody {
            path: fs.path.clone(),
          },
        );
      }
      ("omit-content", Some(_)) => {
        return Err(
          ToolActionMaterializationPlanError::ContentPolicyOmitContentSuppliedBody {
            path: fs.path.clone(),
          },
        );
      }
      _ => {}
    }
  }
  Ok(ToolActionMaterializationPlan {
    request: request.clone(),
    apply_receipt_artifact_id: apply_receipt_artifact_id.to_string(),
    file_states,
  })
}

// ─── materialization receipt ──────────────────────────────────────────

/// Per-file outcome from the host executor's disk write.
///
/// OWNER-LAW (2026-05-11): the executor consumes a
/// [`ToolActionMaterializationPlan`], walks its `file_states`, and
/// emits one `DiskWriteOutcome` per file. Multiple outcomes per plan
/// — partial materialization is supported (e.g. 2 of 3 files written,
/// 1 drift-detected).
///
/// Variants:
///
///   - `Written` — happy path. Bytes written to disk; recompute
///     sha256 from the actual write so the receipt records what the
///     filesystem actually has.
///   - `PreApplyDriftDetected` — the file's on-disk sha256 didn't
///     match `pre_apply_sha256` in the plan. Someone hand-edited
///     between apply and materialize. Executor refused to write
///     (would clobber third-party edits).
///   - `WriteIoError` — std::fs::write returned an I/O error. The
///     `error_kind` is a kebab-case category string (`permission-
///     denied`, `not-found`, `would-overwrite-symlink`, etc.) and
///     `error_message` carries the OS-level detail.
///   - `TargetPathOutsideAllowed` — defense in depth. The plan stage
///     already enforced this; if the executor sees a path outside
///     `allowed_target_paths` it refused to write rather than trust
///     the plan blindly.
///   - `PathDoesNotExist` — the file the plan named has been
///     deleted between apply and materialize. Refuse to write
///     (would create a new file under an unexpected name).
///   - `PathIsNotARegularFile` — the path exists but is a directory,
///     symlink, device node, etc. Refuse to write.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "outcome")]
pub enum DiskWriteOutcome {
  Written {
    path: String,
    /// SHA-256 of the bytes the executor actually wrote. Recomputed
    /// from the write — should equal the plan's `post_apply_sha256`
    /// (audit can cross-check).
    written_sha256: String,
    byte_len: usize,
  },
  PreApplyDriftDetected {
    path: String,
    /// What the plan expected to find on disk before writing.
    expected_sha256: String,
    /// What the executor actually found on disk.
    found_sha256: String,
  },
  WriteIoError {
    path: String,
    /// Kebab-case category (`permission-denied`, `not-found`,
    /// `out-of-space`, `interrupted`, etc.).
    error_kind: String,
    /// OS-level detail for diagnostics.
    error_message: String,
  },
  TargetPathOutsideAllowed {
    path: String,
  },
  PathDoesNotExist {
    path: String,
  },
  PathIsNotARegularFile {
    path: String,
  },
  /// All-or-nothing mode (`MaterializationWriteMode::AllOrNothing`):
  /// this file's preflight passed but the batch was aborted before any
  /// (or after) a write was attempted. Two distinct cases share this
  /// outcome:
  ///
  ///   1. **Preflight-phase abort:** at least one OTHER file's preflight
  ///      would have failed, so the executor refused to write *any*
  ///      file (avoids a half-applied multi-file patch).
  ///
  ///   2. **Write-phase abort:** preflight passed for every file, but a
  ///      later file's `std::fs::write` failed (e.g. permission denied,
  ///      out-of-space). For files NOT yet attempted at that point, no
  ///      write was performed — they remain on disk untouched. For
  ///      files already written, see `RolledBackAfterWriteFailure` /
  ///      `RollbackAfterWriteFailureIoError`.
  ///
  /// In both cases the file on disk is unchanged. The distinction
  /// between cases 1 and 2 can be derived from the receipt as a whole
  /// (case 2 implies at least one `RolledBackAfterWriteFailure` or
  /// `RollbackAfterWriteFailureIoError` is also present).
  ///
  /// OWNER-LAW (2026-05-12): the conservative default. Service-grade
  /// deployments should not leave a repo half-edited.
  /// `MaterializationWriteMode::AllowPartial` opts back into per-file
  /// commit semantics (and skips rollback on write failure).
  SkippedAllOrNothingAbort {
    path: String,
  },
  /// All-or-nothing mode: this file's write phase succeeded, but a
  /// LATER file in the same batch failed to write. The executor read
  /// the original bytes during preflight (drift check) and used them
  /// to restore this file to its pre-apply state.
  ///
  /// `restored_sha256` is the hash of the file's on-disk content
  /// **read back after** the rollback write — not the hash of the
  /// bytes we intended to write. By contract it must equal
  /// `pre_apply_sha256` from the plan; if it doesn't, the executor
  /// emits `RollbackAfterWriteFailureIoError` instead (post-rollback
  /// sha256 mismatch). This protects against OS-level partial-write
  /// pathologies where `write(2)` returned success but the page
  /// cache / filesystem didn't actually commit the expected bytes.
  ///
  /// OWNER-LAW (2026-05-12): added to make AllOrNothing a journaled
  /// rollback guarantee rather than a preflight-only gate. AllOrNothing
  /// does NOT promise crash-consistent filesystem transactions — see
  /// `MaterializationWriteMode` for the bounded envelope.
  RolledBackAfterWriteFailure {
    path: String,
    restored_sha256: String,
  },
  /// All-or-nothing mode: this file was successfully written, but a
  /// later file failed AND the rollback write (restoring the original
  /// bytes) ALSO failed. The file remains in the post-apply state
  /// despite the batch abort — this is the partial-state worst case.
  ///
  /// `attempted_restore_sha256` is what the executor *intended* to
  /// restore (i.e. the original `pre_apply_sha256`). `error_kind` /
  /// `error_message` describe why the restore write failed.
  ///
  /// OWNER-LAW (2026-05-12): this outcome is the explicit audit signal
  /// that AllOrNothing was violated. Service-grade callers should
  /// treat a receipt containing this variant as a hard alarm.
  RollbackAfterWriteFailureIoError {
    path: String,
    attempted_restore_sha256: String,
    /// Kebab-case category of the rollback-write failure
    /// (`permission-denied`, `out-of-space`, etc.).
    error_kind: String,
    error_message: String,
  },
}

impl DiskWriteOutcome {
  /// Return the kebab-case outcome kind. Useful for receipt summary
  /// counters and freecat-cli panel labels.
  pub fn kind_str(&self) -> &'static str {
    match self {
      Self::Written { .. } => "written",
      Self::PreApplyDriftDetected { .. } => "pre-apply-drift-detected",
      Self::WriteIoError { .. } => "write-io-error",
      Self::TargetPathOutsideAllowed { .. } => "target-path-outside-allowed",
      Self::PathDoesNotExist { .. } => "path-does-not-exist",
      Self::PathIsNotARegularFile { .. } => "path-is-not-a-regular-file",
      Self::SkippedAllOrNothingAbort { .. } => "skipped-all-or-nothing-abort",
      Self::RolledBackAfterWriteFailure { .. } => "rolled-back-after-write-failure",
      Self::RollbackAfterWriteFailureIoError { .. } => "rollback-after-write-failure-io-error",
    }
  }

  /// Path the outcome relates to. All variants carry a path.
  pub fn path(&self) -> &str {
    match self {
      Self::Written { path, .. }
      | Self::PreApplyDriftDetected { path, .. }
      | Self::WriteIoError { path, .. }
      | Self::TargetPathOutsideAllowed { path, .. }
      | Self::PathDoesNotExist { path, .. }
      | Self::PathIsNotARegularFile { path, .. }
      | Self::SkippedAllOrNothingAbort { path, .. }
      | Self::RolledBackAfterWriteFailure { path, .. }
      | Self::RollbackAfterWriteFailureIoError { path, .. } => path,
    }
  }

  /// `true` only for `Written`. Used by
  /// [`ToolActionMaterializationReceipt::all_writes_succeeded`].
  pub fn is_success(&self) -> bool {
    matches!(self, Self::Written { .. })
  }

  /// `true` for outcomes that explicitly say "this file's preflight
  /// passed but no write occurred because another file's preflight
  /// failed under AllOrNothing mode". Used by audit / cockpit to
  /// distinguish "would have succeeded" from "actually failed".
  pub fn is_all_or_nothing_skip(&self) -> bool {
    matches!(self, Self::SkippedAllOrNothingAbort { .. })
  }

  /// `true` for outcomes where the file was successfully written but
  /// then either restored or left in an unrestored half-applied state
  /// because the batch aborted. Audit / cockpit can use this to flag
  /// "this batch went through the write phase before aborting".
  pub fn is_rollback_lane(&self) -> bool {
    matches!(
      self,
      Self::RolledBackAfterWriteFailure { .. } | Self::RollbackAfterWriteFailureIoError { .. }
    )
  }

  /// `true` only for the worst-case partial-state outcome: the file
  /// was written, the batch aborted, and the rollback restore ALSO
  /// failed. Service-grade callers should treat a receipt containing
  /// this as a hard alarm — the repo is in a non-canonical state.
  pub fn is_unrestored_partial_state(&self) -> bool {
    matches!(self, Self::RollbackAfterWriteFailureIoError { .. })
  }
}

/// The receipt of a materialization run — pure data describing what
/// the executor did per file.
///
/// OWNER-LAW (2026-05-11): the executor (host crate, future slice)
/// runs `std::fs::write` per file in the plan and emits this receipt.
/// `executed_at_ms` is wall-clock at execution start; per-file
/// timing isn't recorded here (the receipt is decision-level, not
/// profiling-level).
///
/// The receipt is what gets stored in doghouse as
/// `coding.tool-action-materialization-receipt`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolActionMaterializationReceipt {
  pub plan: ToolActionMaterializationPlan,
  pub executed_at_ms: u64,
  /// One outcome per file in the plan, in plan order. Length must
  /// equal `plan.file_states.len()` — partial receipts are not
  /// allowed (every file gets an outcome, even if the executor
  /// stopped after the first drift).
  pub disk_write_outcomes: Vec<DiskWriteOutcome>,
}

/// Counts of outcomes by kind. Computed once and cached in the
/// receipt's canonical payload for fast cockpit rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MaterializationOutcomeSummary {
  pub total: usize,
  pub written: usize,
  pub pre_apply_drift_detected: usize,
  pub write_io_error: usize,
  pub target_path_outside_allowed: usize,
  pub path_does_not_exist: usize,
  pub path_is_not_a_regular_file: usize,
  /// Files whose preflight passed but were skipped because another
  /// file in the plan would have failed under
  /// `MaterializationWriteMode::AllOrNothing`. Distinct from
  /// `write_io_error` — these files are *fine* on disk; the executor
  /// just refused to write them to avoid a half-applied multi-file
  /// patch.
  pub skipped_all_or_nothing_abort: usize,
  /// Files that were successfully written but later restored to their
  /// pre-apply state because a sibling file's write failed under
  /// `MaterializationWriteMode::AllOrNothing`. These files are *fine*
  /// on disk (restored) but the audit trail records the touch.
  pub rolled_back_after_write_failure: usize,
  /// Worst-case partial state: files that were written, the batch
  /// aborted, and the rollback restore *also* failed. Repo is in a
  /// non-canonical state for these paths — service-grade callers must
  /// treat this as a hard alarm.
  pub rollback_after_write_failure_io_error: usize,
}

impl ToolActionMaterializationReceipt {
  /// Build a summary of outcome counts. Used by the canonical
  /// payload renderer + the freecat-cli cockpit panel.
  pub fn summary(&self) -> MaterializationOutcomeSummary {
    let mut s = MaterializationOutcomeSummary::default();
    for o in &self.disk_write_outcomes {
      s.total += 1;
      match o {
        DiskWriteOutcome::Written { .. } => s.written += 1,
        DiskWriteOutcome::PreApplyDriftDetected { .. } => s.pre_apply_drift_detected += 1,
        DiskWriteOutcome::WriteIoError { .. } => s.write_io_error += 1,
        DiskWriteOutcome::TargetPathOutsideAllowed { .. } => s.target_path_outside_allowed += 1,
        DiskWriteOutcome::PathDoesNotExist { .. } => s.path_does_not_exist += 1,
        DiskWriteOutcome::PathIsNotARegularFile { .. } => s.path_is_not_a_regular_file += 1,
        DiskWriteOutcome::SkippedAllOrNothingAbort { .. } => s.skipped_all_or_nothing_abort += 1,
        DiskWriteOutcome::RolledBackAfterWriteFailure { .. } => {
          s.rolled_back_after_write_failure += 1
        }
        DiskWriteOutcome::RollbackAfterWriteFailureIoError { .. } => {
          s.rollback_after_write_failure_io_error += 1
        }
      }
    }
    s
  }

  /// True iff every outcome is `Written` and the count matches the
  /// plan's file count.
  pub fn all_writes_succeeded(&self) -> bool {
    self.disk_write_outcomes.len() == self.plan.file_states.len()
      && self.disk_write_outcomes.iter().all(|o| o.is_success())
  }

  /// True iff at least one outcome is *not* `Written`. Indicates the
  /// caller should consult `summary()` for details.
  pub fn has_failures(&self) -> bool {
    !self.all_writes_succeeded()
  }
}

// ─── canonical receipt artifact ───────────────────────────────────────

/// Render a `ToolActionMaterializationReceipt` as the canonical JSON
/// payload of a `coding.tool-action-materialization-receipt`
/// artifact.
///
/// OWNER-LAW (2026-05-11): payload shape:
///
/// ```json
/// {
///   "transform": "tool-action-materialization",
///   "apply_receipt_artifact_id": "apply-receipt.<transform>.<hex>",
///   "request": { ... },
///   "executed_at_ms": ...,
///   "summary": { "total": .., "written": .., "pre_apply_drift_detected": .., ... },
///   "all_writes_succeeded": true | false,
///   "disk_write_outcomes": [
///     { "outcome": "written", "path": "...", "written_sha256": "...", "byte_len": ... },
///     { "outcome": "pre-apply-drift-detected", "path": "...", "expected_sha256": "...", "found_sha256": "..." },
///     ...
///   ],
///   "next_step": "verify-or-rollback-or-promote"
/// }
/// ```
///
/// `request` carries the full materialization request so the receipt
/// is self-contained for audit (capability, deployment_mode,
/// content_policy, repo_snapshot_ref, actor, tenant). The
/// `disk_write_outcomes` list uses the serde-derived kebab-case
/// `outcome` tag.
pub fn build_tool_action_materialization_receipt_payload(
  receipt: &ToolActionMaterializationReceipt,
) -> serde_json::Value {
  let summary = receipt.summary();
  serde_json::json!({
    "transform": "tool-action-materialization",
    "apply_receipt_artifact_id": receipt.plan.apply_receipt_artifact_id,
    "request": receipt.plan.request,
    "executed_at_ms": receipt.executed_at_ms,
    "summary": {
      "total": summary.total,
      "written": summary.written,
      "pre_apply_drift_detected": summary.pre_apply_drift_detected,
      "write_io_error": summary.write_io_error,
      "target_path_outside_allowed": summary.target_path_outside_allowed,
      "path_does_not_exist": summary.path_does_not_exist,
      "path_is_not_a_regular_file": summary.path_is_not_a_regular_file,
      "skipped_all_or_nothing_abort": summary.skipped_all_or_nothing_abort,
      "rolled_back_after_write_failure": summary.rolled_back_after_write_failure,
      "rollback_after_write_failure_io_error": summary.rollback_after_write_failure_io_error,
    },
    "all_writes_succeeded": receipt.all_writes_succeeded(),
    "unrestored_partial_state_count": summary.rollback_after_write_failure_io_error,
    "disk_write_outcomes": receipt.disk_write_outcomes,
    "next_step": "verify-or-rollback-or-promote",
  })
}

/// Wrap a `ToolActionMaterializationReceipt` into a full
/// `coding.tool-action-materialization-receipt` artifact value
/// with a replay-stable id.
///
/// OWNER-LAW (2026-05-11): id hash binds intrinsic execution
/// identity:
///
///   1. `apply_receipt_artifact_id` (which apply event this
///      materializes)
///   2. `repo_snapshot_ref` (repo state at apply time)
///   3. `requested_by_actor_id` / `_tenant_id` / `requested_at_ms`
///   4. `executed_at_ms` (event identity — each materialization
///      execution is distinct)
///   5. For each outcome in order: `kind_str` + path + per-variant
///      identifying field (written: written_sha256; drift: expected +
///      found sha256; io-error: error_kind; others: just path).
///
/// `stored_at_ms` is extrinsic and not in the hash. `related_refs`
/// carries `apply-receipt-artifact:<id>` so audit can walk from the
/// apply-receipt to its materialization outcomes.
pub fn build_tool_action_materialization_receipt_artifact(
  receipt: &ToolActionMaterializationReceipt,
  stored_at_ms: u64,
) -> serde_json::Value {
  let payload = build_tool_action_materialization_receipt_payload(receipt);
  let request = &receipt.plan.request;

  let mut hasher = Sha256::new();
  hasher.update(b"tool-action-materialization-receipt\x1f");
  hasher.update(receipt.plan.apply_receipt_artifact_id.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(request.repo_snapshot_ref.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(request.requested_by_actor_id.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(request.requested_by_tenant_id.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(request.requested_at_ms.to_le_bytes());
  hasher.update(b"\x1f");
  hasher.update(receipt.executed_at_ms.to_le_bytes());
  hasher.update(b"\x1f");
  for outcome in &receipt.disk_write_outcomes {
    hasher.update(outcome.kind_str().as_bytes());
    hasher.update(b"\x1e");
    hasher.update(outcome.path().as_bytes());
    hasher.update(b"\x1d");
    match outcome {
      DiskWriteOutcome::Written {
        written_sha256,
        byte_len,
        ..
      } => {
        hasher.update(written_sha256.as_bytes());
        hasher.update(byte_len.to_le_bytes());
      }
      DiskWriteOutcome::PreApplyDriftDetected {
        expected_sha256,
        found_sha256,
        ..
      } => {
        hasher.update(expected_sha256.as_bytes());
        hasher.update(b"\x1c");
        hasher.update(found_sha256.as_bytes());
      }
      DiskWriteOutcome::WriteIoError { error_kind, .. } => {
        hasher.update(error_kind.as_bytes());
      }
      DiskWriteOutcome::RolledBackAfterWriteFailure {
        restored_sha256, ..
      } => {
        hasher.update(restored_sha256.as_bytes());
      }
      DiskWriteOutcome::RollbackAfterWriteFailureIoError {
        attempted_restore_sha256,
        error_kind,
        ..
      } => {
        hasher.update(attempted_restore_sha256.as_bytes());
        hasher.update(b"\x1c");
        hasher.update(error_kind.as_bytes());
      }
      DiskWriteOutcome::TargetPathOutsideAllowed { .. }
      | DiskWriteOutcome::PathDoesNotExist { .. }
      | DiskWriteOutcome::PathIsNotARegularFile { .. }
      | DiskWriteOutcome::SkippedAllOrNothingAbort { .. } => {
        // path already hashed above — no per-variant extra. The
        // kind_str discriminator already differentiates these from
        // each other in the hash stream.
      }
    }
    hasher.update(b"\x1b");
  }
  let digest = hasher.finalize();
  let prefix = digest
    .iter()
    .take(8)
    .map(|b| format!("{b:02x}"))
    .collect::<String>();
  let id = format!("tool-action-materialization-receipt.{prefix}");

  // target_paths: the paths that the materialization touched, in
  // outcome order. Allows the doghouse `coding_memory_artifacts_by_*`
  // index to surface this receipt under each affected path.
  let target_paths: Vec<String> = receipt
    .disk_write_outcomes
    .iter()
    .map(|o| o.path().to_string())
    .collect();

  serde_json::json!({
    "id": id,
    "artifact_family": "coding.tool-action-materialization-receipt",
    "source_surface": "tool-action.materialization",
    "stored_at_ms": stored_at_ms,
    "target_paths": target_paths,
    "command_refs": [],
    "related_refs": [
      "owner-law:crates/pnix-core/src/tool_action.rs",
      format!("apply-receipt-artifact:{}", receipt.plan.apply_receipt_artifact_id),
    ],
    "repo_snapshot_ref": request.repo_snapshot_ref,
    "payload": payload,
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  fn req() -> ToolActionMaterializationRequest {
    ToolActionMaterializationRequest {
      apply_receipt_artifact_id: "apply-receipt.rename-symbol.deadbeef".to_string(),
      repo_snapshot_ref: "git:abc123".to_string(),
      capability: "edit-within-target-paths".to_string(),
      requested_by_actor_id: "actor.user.1".to_string(),
      requested_by_tenant_id: "tenant.alpha".to_string(),
      requested_at_ms: 1700000000000,
      deployment_mode: "dev".to_string(),
      content_policy: "include-content".to_string(),
    }
  }

  #[test]
  fn ready_when_all_preconditions_pass() {
    let r = req();
    assert!(matches!(
      classify_tool_action_materialization_request(&r),
      ToolActionMaterializationVerdict::Ready
    ));
  }

  #[test]
  fn ready_for_customer_release_with_omit_content() {
    let mut r = req();
    r.deployment_mode = "customer-release".to_string();
    r.content_policy = "omit-content".to_string();
    assert!(matches!(
      classify_tool_action_materialization_request(&r),
      ToolActionMaterializationVerdict::Ready
    ));
  }

  #[test]
  fn ready_for_each_recognized_capability_except_readonly_and_forbidden() {
    for cap in &[
      "edit-within-target-paths",
      "edit-test-only",
      "edit-generated",
      "edit-ci-config",
      "edit-px-owner-law-substrate",
    ] {
      let mut r = req();
      r.capability = cap.to_string();
      assert!(
        matches!(
          classify_tool_action_materialization_request(&r),
          ToolActionMaterializationVerdict::Ready
        ),
        "capability {cap} should yield Ready"
      );
    }
  }

  #[test]
  fn held_when_apply_receipt_id_empty() {
    let mut r = req();
    r.apply_receipt_artifact_id = String::new();
    match classify_tool_action_materialization_request(&r) {
      ToolActionMaterializationVerdict::Held { held_kind, .. } => {
        assert_eq!(
          held_kind,
          ToolActionMaterializationHeldKind::MissingApplyReceiptArtifactId
        );
      }
      other => panic!("expected MissingApplyReceiptArtifactId, got {:?}", other),
    }
  }

  #[test]
  fn held_when_apply_receipt_id_malformed() {
    let mut r = req();
    r.apply_receipt_artifact_id = "not-an-apply-receipt".to_string();
    match classify_tool_action_materialization_request(&r) {
      ToolActionMaterializationVerdict::Held { held_kind, .. } => {
        assert_eq!(
          held_kind,
          ToolActionMaterializationHeldKind::MalformedApplyReceiptArtifactId
        );
      }
      other => panic!("expected MalformedApplyReceiptArtifactId, got {:?}", other),
    }
  }

  #[test]
  fn held_when_repo_snapshot_ref_empty() {
    let mut r = req();
    r.repo_snapshot_ref = String::new();
    match classify_tool_action_materialization_request(&r) {
      ToolActionMaterializationVerdict::Held { held_kind, .. } => {
        assert_eq!(
          held_kind,
          ToolActionMaterializationHeldKind::MissingRepoSnapshotRef
        );
      }
      other => panic!("expected MissingRepoSnapshotRef, got {:?}", other),
    }
  }

  #[test]
  fn held_when_actor_empty() {
    let mut r = req();
    r.requested_by_actor_id = String::new();
    match classify_tool_action_materialization_request(&r) {
      ToolActionMaterializationVerdict::Held { held_kind, .. } => {
        assert_eq!(
          held_kind,
          ToolActionMaterializationHeldKind::MissingRequestedByActor
        );
      }
      other => panic!("expected MissingRequestedByActor, got {:?}", other),
    }
  }

  #[test]
  fn held_when_tenant_empty() {
    let mut r = req();
    r.requested_by_tenant_id = String::new();
    match classify_tool_action_materialization_request(&r) {
      ToolActionMaterializationVerdict::Held { held_kind, .. } => {
        assert_eq!(
          held_kind,
          ToolActionMaterializationHeldKind::MissingRequestedByTenant
        );
      }
      other => panic!("expected MissingRequestedByTenant, got {:?}", other),
    }
  }

  #[test]
  fn held_when_capability_empty() {
    let mut r = req();
    r.capability = String::new();
    match classify_tool_action_materialization_request(&r) {
      ToolActionMaterializationVerdict::Held { held_kind, .. } => {
        assert_eq!(
          held_kind,
          ToolActionMaterializationHeldKind::MissingCapability
        );
      }
      other => panic!("expected MissingCapability, got {:?}", other),
    }
  }

  #[test]
  fn held_when_capability_unrecognized() {
    let mut r = req();
    r.capability = "edit-everything-anywhere".to_string();
    match classify_tool_action_materialization_request(&r) {
      ToolActionMaterializationVerdict::Held { held_kind, .. } => {
        assert_eq!(
          held_kind,
          ToolActionMaterializationHeldKind::UnrecognizedCapability
        );
      }
      other => panic!("expected UnrecognizedCapability, got {:?}", other),
    }
  }

  #[test]
  fn held_when_capability_read_only() {
    let mut r = req();
    r.capability = "read-only".to_string();
    match classify_tool_action_materialization_request(&r) {
      ToolActionMaterializationVerdict::Held { held_kind, .. } => {
        assert_eq!(
          held_kind,
          ToolActionMaterializationHeldKind::ReadOnlyCapabilityCannotMaterialize
        );
      }
      other => panic!(
        "expected ReadOnlyCapabilityCannotMaterialize, got {:?}",
        other
      ),
    }
  }

  #[test]
  fn rejected_when_capability_forbidden() {
    let mut r = req();
    r.capability = "forbidden".to_string();
    match classify_tool_action_materialization_request(&r) {
      ToolActionMaterializationVerdict::Rejected { held_kind, .. } => {
        assert_eq!(
          held_kind,
          ToolActionMaterializationHeldKind::ForbiddenCapability
        );
      }
      other => panic!("expected ForbiddenCapability rejection, got {:?}", other),
    }
  }

  #[test]
  fn held_when_deployment_mode_empty_or_unrecognized() {
    let mut r = req();
    r.deployment_mode = String::new();
    assert!(matches!(
      classify_tool_action_materialization_request(&r),
      ToolActionMaterializationVerdict::Held {
        held_kind: ToolActionMaterializationHeldKind::MissingDeploymentMode,
        ..
      }
    ));
    r.deployment_mode = "production-prime".to_string();
    assert!(matches!(
      classify_tool_action_materialization_request(&r),
      ToolActionMaterializationVerdict::Held {
        held_kind: ToolActionMaterializationHeldKind::UnrecognizedDeploymentMode,
        ..
      }
    ));
  }

  #[test]
  fn held_when_content_policy_empty_or_unrecognized() {
    let mut r = req();
    r.content_policy = String::new();
    assert!(matches!(
      classify_tool_action_materialization_request(&r),
      ToolActionMaterializationVerdict::Held {
        held_kind: ToolActionMaterializationHeldKind::MissingContentPolicy,
        ..
      }
    ));
    r.content_policy = "stream-content".to_string();
    assert!(matches!(
      classify_tool_action_materialization_request(&r),
      ToolActionMaterializationVerdict::Held {
        held_kind: ToolActionMaterializationHeldKind::UnrecognizedContentPolicy,
        ..
      }
    ));
  }

  #[test]
  fn rejected_when_customer_release_combined_with_include_content() {
    let mut r = req();
    r.deployment_mode = "customer-release".to_string();
    r.content_policy = "include-content".to_string();
    match classify_tool_action_materialization_request(&r) {
      ToolActionMaterializationVerdict::Rejected { held_kind, reason } => {
        assert_eq!(
          held_kind,
          ToolActionMaterializationHeldKind::CustomerReleaseForbidsIncludeContent
        );
        assert!(reason.contains("customer-release"));
        assert!(reason.contains("include-content"));
      }
      other => panic!(
        "expected CustomerReleaseForbidsIncludeContent rejection, got {:?}",
        other
      ),
    }
  }

  #[test]
  fn ladder_order_apply_receipt_id_wins_over_other_empties() {
    // Multiple fields empty — the classifier returns the FIRST
    // failure in ladder order so the caller knows what to fix first.
    let mut r = req();
    r.apply_receipt_artifact_id = String::new();
    r.repo_snapshot_ref = String::new();
    r.requested_by_actor_id = String::new();
    match classify_tool_action_materialization_request(&r) {
      ToolActionMaterializationVerdict::Held { held_kind, .. } => {
        // Apply receipt id is checked first.
        assert_eq!(
          held_kind,
          ToolActionMaterializationHeldKind::MissingApplyReceiptArtifactId
        );
      }
      other => panic!(
        "expected MissingApplyReceiptArtifactId first, got {:?}",
        other
      ),
    }
  }

  // ─── materialization plan ────────────────────────────────────────

  fn plan_req() -> ToolActionMaterializationRequest {
    let mut r = req();
    // Plan tests use omit-content by default (customer-release safe).
    r.content_policy = "omit-content".to_string();
    r
  }

  fn file_state(path: &str) -> ApplyReceiptFileState {
    ApplyReceiptFileState {
      path: path.to_string(),
      pre_apply_sha256: "abc123pre".to_string(),
      post_apply_sha256: "def456post".to_string(),
      post_apply_byte_len: 42,
      post_apply_content: None,
    }
  }

  #[test]
  fn plan_ready_omit_content_path() {
    let request = plan_req();
    let states = vec![file_state("src/a.rs")];
    let plan = build_tool_action_materialization_plan(
      &request,
      &request.apply_receipt_artifact_id.clone(),
      states,
      &["src/a.rs".to_string()],
    )
    .expect("plan succeeds");
    assert_eq!(plan.file_states.len(), 1);
    assert_eq!(plan.file_states[0].path, "src/a.rs");
    assert!(plan.file_states[0].post_apply_content.is_none());
    assert_eq!(
      plan.apply_receipt_artifact_id,
      plan.request.apply_receipt_artifact_id
    );
  }

  #[test]
  fn plan_ready_include_content_path() {
    let mut request = plan_req();
    request.content_policy = "include-content".to_string();
    let mut fs = file_state("src/a.rs");
    fs.post_apply_content = Some("fn bar() {}\n".to_string());
    let plan = build_tool_action_materialization_plan(
      &request,
      &request.apply_receipt_artifact_id.clone(),
      vec![fs],
      &["src/a.rs".to_string()],
    )
    .expect("plan succeeds");
    assert_eq!(
      plan.file_states[0].post_apply_content.as_deref(),
      Some("fn bar() {}\n")
    );
  }

  #[test]
  fn plan_rejects_request_not_ready() {
    let mut request = plan_req();
    request.apply_receipt_artifact_id = String::new();
    let states = vec![file_state("src/a.rs")];
    match build_tool_action_materialization_plan(
      &request,
      "apply-receipt.rename-symbol.x",
      states,
      &["src/a.rs".to_string()],
    ) {
      Err(ToolActionMaterializationPlanError::RequestNotReady(v)) => {
        // The underlying request is Held on MissingApplyReceiptArtifactId.
        assert!(matches!(v, ToolActionMaterializationVerdict::Held { .. }));
      }
      other => panic!("expected RequestNotReady, got {:?}", other),
    }
  }

  #[test]
  fn plan_rejects_apply_receipt_id_mismatch() {
    let request = plan_req();
    let states = vec![file_state("src/a.rs")];
    let result = build_tool_action_materialization_plan(
      &request,
      "apply-receipt.rename-symbol.different",
      states,
      &["src/a.rs".to_string()],
    );
    match result {
      Err(ToolActionMaterializationPlanError::ApplyReceiptIdMismatch { expected, got }) => {
        assert_eq!(expected, request.apply_receipt_artifact_id);
        assert_eq!(got, "apply-receipt.rename-symbol.different");
      }
      other => panic!("expected ApplyReceiptIdMismatch, got {:?}", other),
    }
  }

  #[test]
  fn plan_rejects_empty_file_states() {
    let request = plan_req();
    let result = build_tool_action_materialization_plan(
      &request,
      &request.apply_receipt_artifact_id.clone(),
      Vec::new(),
      &["src/a.rs".to_string()],
    );
    assert!(matches!(
      result,
      Err(ToolActionMaterializationPlanError::EmptyFileStates)
    ));
  }

  #[test]
  fn plan_rejects_path_outside_allowed() {
    let request = plan_req();
    let states = vec![file_state("src/secret.rs")];
    let result = build_tool_action_materialization_plan(
      &request,
      &request.apply_receipt_artifact_id.clone(),
      states,
      &["src/a.rs".to_string()], // secret.rs not in allowed
    );
    match result {
      Err(ToolActionMaterializationPlanError::TargetPathNotAllowed { path }) => {
        assert_eq!(path, "src/secret.rs");
      }
      other => panic!("expected TargetPathNotAllowed, got {:?}", other),
    }
  }

  #[test]
  fn plan_rejects_duplicate_file_path() {
    let request = plan_req();
    let states = vec![file_state("src/a.rs"), file_state("src/a.rs")];
    let result = build_tool_action_materialization_plan(
      &request,
      &request.apply_receipt_artifact_id.clone(),
      states,
      &["src/a.rs".to_string()],
    );
    match result {
      Err(ToolActionMaterializationPlanError::DuplicateFilePath { path }) => {
        assert_eq!(path, "src/a.rs");
      }
      other => panic!("expected DuplicateFilePath, got {:?}", other),
    }
  }

  #[test]
  fn plan_rejects_empty_pre_apply_sha256() {
    let request = plan_req();
    let mut fs = file_state("src/a.rs");
    fs.pre_apply_sha256 = String::new();
    let result = build_tool_action_materialization_plan(
      &request,
      &request.apply_receipt_artifact_id.clone(),
      vec![fs],
      &["src/a.rs".to_string()],
    );
    match result {
      Err(ToolActionMaterializationPlanError::EmptyPreApplySha256 { path }) => {
        assert_eq!(path, "src/a.rs");
      }
      other => panic!("expected EmptyPreApplySha256, got {:?}", other),
    }
  }

  #[test]
  fn plan_rejects_empty_post_apply_sha256() {
    let request = plan_req();
    let mut fs = file_state("src/a.rs");
    fs.post_apply_sha256 = String::new();
    let result = build_tool_action_materialization_plan(
      &request,
      &request.apply_receipt_artifact_id.clone(),
      vec![fs],
      &["src/a.rs".to_string()],
    );
    match result {
      Err(ToolActionMaterializationPlanError::EmptyPostApplySha256 { path }) => {
        assert_eq!(path, "src/a.rs");
      }
      other => panic!("expected EmptyPostApplySha256, got {:?}", other),
    }
  }

  #[test]
  fn plan_rejects_include_content_missing_body() {
    let mut request = plan_req();
    request.content_policy = "include-content".to_string();
    // body is None — caller violated the policy.
    let result = build_tool_action_materialization_plan(
      &request,
      &request.apply_receipt_artifact_id.clone(),
      vec![file_state("src/a.rs")],
      &["src/a.rs".to_string()],
    );
    match result {
      Err(ToolActionMaterializationPlanError::ContentPolicyIncludeContentMissingBody { path }) => {
        assert_eq!(path, "src/a.rs");
      }
      other => panic!(
        "expected ContentPolicyIncludeContentMissingBody, got {:?}",
        other
      ),
    }
  }

  #[test]
  fn plan_rejects_omit_content_supplied_body() {
    // omit-content + Some(body) → fail-closed (customer-release leak guard).
    let request = plan_req(); // omit-content already
    let mut fs = file_state("src/a.rs");
    fs.post_apply_content = Some("smuggled body".to_string());
    let result = build_tool_action_materialization_plan(
      &request,
      &request.apply_receipt_artifact_id.clone(),
      vec![fs],
      &["src/a.rs".to_string()],
    );
    match result {
      Err(ToolActionMaterializationPlanError::ContentPolicyOmitContentSuppliedBody { path }) => {
        assert_eq!(path, "src/a.rs");
      }
      other => panic!(
        "expected ContentPolicyOmitContentSuppliedBody, got {:?}",
        other
      ),
    }
  }

  #[test]
  fn plan_ladder_order_request_classification_wins_over_path() {
    // Both the request and the path-allowed list are broken. The
    // request-classification check runs FIRST so the caller sees the
    // root cause (their request is malformed) rather than a downstream
    // symptom (their path isn't allowed).
    let mut request = plan_req();
    request.apply_receipt_artifact_id = String::new();
    let states = vec![file_state("src/secret.rs")];
    let result =
      build_tool_action_materialization_plan(&request, "x", states, &["src/a.rs".to_string()]);
    assert!(matches!(
      result,
      Err(ToolActionMaterializationPlanError::RequestNotReady(_))
    ));
  }

  #[test]
  fn plan_ladder_order_duplicate_wins_over_path_allowed() {
    // Duplicate paths AND wrong allowed list — duplicate check runs
    // first.
    let request = plan_req();
    let states = vec![file_state("src/secret.rs"), file_state("src/secret.rs")];
    let result = build_tool_action_materialization_plan(
      &request,
      &request.apply_receipt_artifact_id.clone(),
      states,
      &["src/a.rs".to_string()],
    );
    assert!(matches!(
      result,
      Err(ToolActionMaterializationPlanError::DuplicateFilePath { .. })
    ));
  }

  #[test]
  fn plan_handles_multiple_files_in_allowed_set() {
    let request = plan_req();
    let states = vec![
      file_state("src/a.rs"),
      file_state("src/b.rs"),
      file_state("src/c.rs"),
    ];
    let plan = build_tool_action_materialization_plan(
      &request,
      &request.apply_receipt_artifact_id.clone(),
      states,
      &[
        "src/a.rs".to_string(),
        "src/b.rs".to_string(),
        "src/c.rs".to_string(),
        "src/d.rs".to_string(), // extra allowed entry — fine
      ],
    )
    .expect("plan succeeds with multiple files");
    assert_eq!(plan.file_states.len(), 3);
  }

  #[test]
  fn plan_error_messages_include_path_for_diagnostics() {
    // Sanity: error Display impls actually surface the path so the
    // caller's log message identifies the failing file.
    let request = plan_req();
    let states = vec![file_state("src/leaky.rs")];
    let err = build_tool_action_materialization_plan(
      &request,
      &request.apply_receipt_artifact_id.clone(),
      states,
      &["src/other.rs".to_string()],
    )
    .expect_err("path-not-allowed");
    let msg = err.to_string();
    assert!(
      msg.contains("src/leaky.rs"),
      "error msg should include path"
    );
  }

  // ─── materialization receipt ─────────────────────────────────────

  fn fixture_plan_one_file() -> ToolActionMaterializationPlan {
    let request = plan_req();
    build_tool_action_materialization_plan(
      &request,
      &request.apply_receipt_artifact_id.clone(),
      vec![file_state("src/a.rs")],
      &["src/a.rs".to_string()],
    )
    .expect("plan succeeds")
  }

  fn fixture_plan_three_files() -> ToolActionMaterializationPlan {
    let request = plan_req();
    build_tool_action_materialization_plan(
      &request,
      &request.apply_receipt_artifact_id.clone(),
      vec![
        file_state("src/a.rs"),
        file_state("src/b.rs"),
        file_state("src/c.rs"),
      ],
      &[
        "src/a.rs".to_string(),
        "src/b.rs".to_string(),
        "src/c.rs".to_string(),
      ],
    )
    .expect("plan succeeds with three files")
  }

  fn written_outcome(path: &str, sha: &str) -> DiskWriteOutcome {
    DiskWriteOutcome::Written {
      path: path.to_string(),
      written_sha256: sha.to_string(),
      byte_len: 12,
    }
  }

  #[test]
  fn outcome_kind_str_matches_all_variants() {
    let cases = [
      (
        DiskWriteOutcome::Written {
          path: "p".into(),
          written_sha256: "s".into(),
          byte_len: 1,
        },
        "written",
      ),
      (
        DiskWriteOutcome::PreApplyDriftDetected {
          path: "p".into(),
          expected_sha256: "e".into(),
          found_sha256: "f".into(),
        },
        "pre-apply-drift-detected",
      ),
      (
        DiskWriteOutcome::WriteIoError {
          path: "p".into(),
          error_kind: "permission-denied".into(),
          error_message: "denied".into(),
        },
        "write-io-error",
      ),
      (
        DiskWriteOutcome::TargetPathOutsideAllowed { path: "p".into() },
        "target-path-outside-allowed",
      ),
      (
        DiskWriteOutcome::PathDoesNotExist { path: "p".into() },
        "path-does-not-exist",
      ),
      (
        DiskWriteOutcome::PathIsNotARegularFile { path: "p".into() },
        "path-is-not-a-regular-file",
      ),
      (
        DiskWriteOutcome::SkippedAllOrNothingAbort { path: "p".into() },
        "skipped-all-or-nothing-abort",
      ),
      (
        DiskWriteOutcome::RolledBackAfterWriteFailure {
          path: "p".into(),
          restored_sha256: "abc".into(),
        },
        "rolled-back-after-write-failure",
      ),
      (
        DiskWriteOutcome::RollbackAfterWriteFailureIoError {
          path: "p".into(),
          attempted_restore_sha256: "abc".into(),
          error_kind: "permission-denied".into(),
          error_message: "denied".into(),
        },
        "rollback-after-write-failure-io-error",
      ),
    ];
    for (outcome, expected) in cases {
      assert_eq!(outcome.kind_str(), expected);
      assert_eq!(outcome.path(), "p");
    }
  }

  #[test]
  fn rollback_outcomes_audit_helpers() {
    let rb = DiskWriteOutcome::RolledBackAfterWriteFailure {
      path: "src/a.rs".into(),
      restored_sha256: "abc".into(),
    };
    assert!(rb.is_rollback_lane());
    assert!(!rb.is_unrestored_partial_state());
    assert!(!rb.is_success());
    assert!(!rb.is_all_or_nothing_skip());

    let rbio = DiskWriteOutcome::RollbackAfterWriteFailureIoError {
      path: "src/b.rs".into(),
      attempted_restore_sha256: "abc".into(),
      error_kind: "permission-denied".into(),
      error_message: "denied".into(),
    };
    assert!(rbio.is_rollback_lane());
    assert!(rbio.is_unrestored_partial_state());
    assert!(!rbio.is_success());
    assert!(!rbio.is_all_or_nothing_skip());
  }

  #[test]
  fn outcome_is_success_only_for_written() {
    assert!(written_outcome("src/a.rs", "abc").is_success());
    assert!(!DiskWriteOutcome::PreApplyDriftDetected {
      path: "src/a.rs".into(),
      expected_sha256: "abc".into(),
      found_sha256: "def".into(),
    }
    .is_success());
    assert!(!DiskWriteOutcome::WriteIoError {
      path: "src/a.rs".into(),
      error_kind: "permission-denied".into(),
      error_message: "x".into(),
    }
    .is_success());
    assert!(!DiskWriteOutcome::TargetPathOutsideAllowed {
      path: "src/a.rs".into(),
    }
    .is_success());
    assert!(!DiskWriteOutcome::PathDoesNotExist {
      path: "src/a.rs".into(),
    }
    .is_success());
    assert!(!DiskWriteOutcome::PathIsNotARegularFile {
      path: "src/a.rs".into(),
    }
    .is_success());
    assert!(!DiskWriteOutcome::SkippedAllOrNothingAbort {
      path: "src/a.rs".into(),
    }
    .is_success());
    assert!(!DiskWriteOutcome::RolledBackAfterWriteFailure {
      path: "src/a.rs".into(),
      restored_sha256: "abc".into(),
    }
    .is_success());
    assert!(!DiskWriteOutcome::RollbackAfterWriteFailureIoError {
      path: "src/a.rs".into(),
      attempted_restore_sha256: "abc".into(),
      error_kind: "permission-denied".into(),
      error_message: "denied".into(),
    }
    .is_success());
  }

  // ─── review + apply → request bridge ─────────────────────────────

  fn bridge_inputs_ready<'a>() -> MaterializationBridgeInputs<'a> {
    MaterializationBridgeInputs {
      review_decision: "approve",
      review_candidate_artifact_id: "generated-patch.rename-symbol.cafef00d",
      review_reviewer_tenant_id: "tenant.alpha",
      apply_candidate_artifact_id: "generated-patch.rename-symbol.cafef00d",
      apply_receipt_artifact_id: "apply-receipt.rename-symbol.deadbeef",
      apply_approval_actor_id: "actor.user.1",
      apply_approval_tenant_id: "tenant.alpha",
      capability: "edit-within-target-paths",
      repo_snapshot_ref: "git:abc123",
      deployment_mode: "dev",
      content_policy: "include-content",
      requested_at_ms: 1700000000000,
    }
  }

  #[test]
  fn bridge_yields_ready_request_on_clean_inputs() {
    let inputs = bridge_inputs_ready();
    let req = bridge_review_apply_to_materialization_request(&inputs).expect("ready");
    assert_eq!(
      req.apply_receipt_artifact_id,
      "apply-receipt.rename-symbol.deadbeef"
    );
    assert_eq!(req.requested_by_actor_id, "actor.user.1");
    assert_eq!(req.requested_by_tenant_id, "tenant.alpha");
    assert_eq!(req.capability, "edit-within-target-paths");
    assert_eq!(req.repo_snapshot_ref, "git:abc123");
    assert_eq!(req.requested_at_ms, 1700000000000);
    // Verify the assembled request itself classifies Ready (sanity).
    assert!(matches!(
      classify_tool_action_materialization_request(&req),
      ToolActionMaterializationVerdict::Ready
    ));
  }

  #[test]
  fn bridge_rejects_hold_review_decision() {
    let mut inputs = bridge_inputs_ready();
    inputs.review_decision = "hold";
    match bridge_review_apply_to_materialization_request(&inputs) {
      Err(MaterializationBridgeError::ReviewNotApproved { decision }) => {
        assert_eq!(decision, "hold");
      }
      other => panic!("expected ReviewNotApproved, got {:?}", other),
    }
  }

  #[test]
  fn bridge_rejects_reject_review_decision() {
    let mut inputs = bridge_inputs_ready();
    inputs.review_decision = "reject";
    assert!(matches!(
      bridge_review_apply_to_materialization_request(&inputs),
      Err(MaterializationBridgeError::ReviewNotApproved { .. })
    ));
  }

  #[test]
  fn bridge_rejects_candidate_id_mismatch() {
    let mut inputs = bridge_inputs_ready();
    inputs.apply_candidate_artifact_id = "generated-patch.rename-symbol.different";
    match bridge_review_apply_to_materialization_request(&inputs) {
      Err(MaterializationBridgeError::ReviewCandidateMismatchesApplyCandidate {
        review_candidate_artifact_id,
        apply_candidate_artifact_id,
      }) => {
        assert_eq!(
          review_candidate_artifact_id,
          "generated-patch.rename-symbol.cafef00d"
        );
        assert_eq!(
          apply_candidate_artifact_id,
          "generated-patch.rename-symbol.different"
        );
      }
      other => panic!(
        "expected ReviewCandidateMismatchesApplyCandidate, got {:?}",
        other
      ),
    }
  }

  #[test]
  fn bridge_rejects_cross_tenant_review_and_apply() {
    let mut inputs = bridge_inputs_ready();
    inputs.review_reviewer_tenant_id = "tenant.alpha";
    inputs.apply_approval_tenant_id = "tenant.beta";
    match bridge_review_apply_to_materialization_request(&inputs) {
      Err(MaterializationBridgeError::ReviewTenantMismatchesApplyTenant {
        review_tenant,
        apply_tenant,
      }) => {
        assert_eq!(review_tenant, "tenant.alpha");
        assert_eq!(apply_tenant, "tenant.beta");
      }
      other => panic!(
        "expected ReviewTenantMismatchesApplyTenant, got {:?}",
        other
      ),
    }
  }

  #[test]
  fn bridge_accepts_different_actors_in_same_tenant() {
    // Different actor on review vs apply is allowed — senior
    // reviews, junior applies. Same tenant is required.
    let mut inputs = bridge_inputs_ready();
    // Caller's apply_approval_actor_id is "actor.user.1"; the
    // review's reviewer is not in `MaterializationBridgeInputs`
    // directly (we only carry the tenant), so this is equivalent to
    // "different actor in same tenant". Should pass.
    inputs.apply_approval_actor_id = "actor.junior.5";
    let req = bridge_review_apply_to_materialization_request(&inputs).expect("ready");
    assert_eq!(req.requested_by_actor_id, "actor.junior.5");
  }

  #[test]
  fn bridge_forwards_classifier_held_for_missing_capability() {
    let mut inputs = bridge_inputs_ready();
    inputs.capability = "";
    match bridge_review_apply_to_materialization_request(&inputs) {
      Err(MaterializationBridgeError::RequestNotReady(verdict)) => {
        assert!(matches!(
          verdict,
          ToolActionMaterializationVerdict::Held {
            held_kind: ToolActionMaterializationHeldKind::MissingCapability,
            ..
          }
        ));
      }
      other => panic!(
        "expected RequestNotReady(MissingCapability), got {:?}",
        other
      ),
    }
  }

  #[test]
  fn bridge_forwards_classifier_held_for_read_only_capability() {
    let mut inputs = bridge_inputs_ready();
    inputs.capability = "read-only";
    match bridge_review_apply_to_materialization_request(&inputs) {
      Err(MaterializationBridgeError::RequestNotReady(verdict)) => {
        assert!(matches!(
          verdict,
          ToolActionMaterializationVerdict::Held {
            held_kind: ToolActionMaterializationHeldKind::ReadOnlyCapabilityCannotMaterialize,
            ..
          }
        ));
      }
      other => panic!("expected RequestNotReady(read-only), got {:?}", other),
    }
  }

  #[test]
  fn bridge_forwards_classifier_rejected_for_forbidden_capability() {
    let mut inputs = bridge_inputs_ready();
    inputs.capability = "forbidden";
    match bridge_review_apply_to_materialization_request(&inputs) {
      Err(MaterializationBridgeError::RequestNotReady(verdict)) => {
        assert!(matches!(
          verdict,
          ToolActionMaterializationVerdict::Rejected {
            held_kind: ToolActionMaterializationHeldKind::ForbiddenCapability,
            ..
          }
        ));
      }
      other => panic!("expected RequestNotReady(forbidden), got {:?}", other),
    }
  }

  #[test]
  fn bridge_forwards_classifier_rejected_for_customer_release_with_include_content() {
    let mut inputs = bridge_inputs_ready();
    inputs.deployment_mode = "customer-release";
    inputs.content_policy = "include-content";
    match bridge_review_apply_to_materialization_request(&inputs) {
      Err(MaterializationBridgeError::RequestNotReady(verdict)) => {
        assert!(matches!(
          verdict,
          ToolActionMaterializationVerdict::Rejected {
            held_kind: ToolActionMaterializationHeldKind::CustomerReleaseForbidsIncludeContent,
            ..
          }
        ));
      }
      other => panic!(
        "expected RequestNotReady(customer-release leak), got {:?}",
        other
      ),
    }
  }

  #[test]
  fn bridge_ladder_review_decision_wins_over_candidate_mismatch() {
    // Even with a candidate mismatch AND no approve, the bridge
    // returns ReviewNotApproved first — it's the simpler precondition
    // and lets the caller fix the root issue first.
    let mut inputs = bridge_inputs_ready();
    inputs.review_decision = "hold";
    inputs.apply_candidate_artifact_id = "generated-patch.rename-symbol.different";
    assert!(matches!(
      bridge_review_apply_to_materialization_request(&inputs),
      Err(MaterializationBridgeError::ReviewNotApproved { .. })
    ));
  }

  #[test]
  fn bridge_ladder_candidate_mismatch_wins_over_tenant_mismatch() {
    let mut inputs = bridge_inputs_ready();
    inputs.apply_candidate_artifact_id = "different";
    inputs.apply_approval_tenant_id = "tenant.beta";
    assert!(matches!(
      bridge_review_apply_to_materialization_request(&inputs),
      Err(MaterializationBridgeError::ReviewCandidateMismatchesApplyCandidate { .. })
    ));
  }

  #[test]
  fn bridge_error_display_messages_include_diagnostic_strings() {
    // Sanity: Display impls surface actor/tenant/decision/etc. so
    // operator logs show the right cause.
    let mut inputs = bridge_inputs_ready();
    inputs.review_decision = "reject";
    let err = bridge_review_apply_to_materialization_request(&inputs).expect_err("reject");
    let msg = err.to_string();
    assert!(msg.contains("reject"));
    assert!(msg.contains("approve"));
  }

  #[test]
  fn outcome_is_all_or_nothing_skip_only_for_that_variant() {
    assert!(DiskWriteOutcome::SkippedAllOrNothingAbort {
      path: "src/a.rs".into(),
    }
    .is_all_or_nothing_skip());
    assert!(!written_outcome("src/a.rs", "abc").is_all_or_nothing_skip());
    assert!(!DiskWriteOutcome::PreApplyDriftDetected {
      path: "src/a.rs".into(),
      expected_sha256: "e".into(),
      found_sha256: "f".into(),
    }
    .is_all_or_nothing_skip());
  }

  #[test]
  fn receipt_summary_counts_all_kinds() {
    let plan = fixture_plan_three_files();
    let receipt = ToolActionMaterializationReceipt {
      plan,
      executed_at_ms: 1800000000000,
      disk_write_outcomes: vec![
        written_outcome("src/a.rs", "abc"),
        DiskWriteOutcome::PreApplyDriftDetected {
          path: "src/b.rs".into(),
          expected_sha256: "pre".into(),
          found_sha256: "drifted".into(),
        },
        DiskWriteOutcome::WriteIoError {
          path: "src/c.rs".into(),
          error_kind: "permission-denied".into(),
          error_message: "denied".into(),
        },
      ],
    };
    let s = receipt.summary();
    assert_eq!(s.total, 3);
    assert_eq!(s.written, 1);
    assert_eq!(s.pre_apply_drift_detected, 1);
    assert_eq!(s.write_io_error, 1);
    assert_eq!(s.target_path_outside_allowed, 0);
  }

  #[test]
  fn receipt_all_writes_succeeded_requires_every_file_written() {
    let plan = fixture_plan_three_files();
    let plan_len = plan.file_states.len();

    // 3 written, plan has 3 → success.
    let all_written = ToolActionMaterializationReceipt {
      plan: plan.clone(),
      executed_at_ms: 0,
      disk_write_outcomes: vec![
        written_outcome("src/a.rs", "a"),
        written_outcome("src/b.rs", "b"),
        written_outcome("src/c.rs", "c"),
      ],
    };
    assert!(all_written.all_writes_succeeded());
    assert!(!all_written.has_failures());

    // 2 written + 1 drift → has_failures.
    let mixed = ToolActionMaterializationReceipt {
      plan: plan.clone(),
      executed_at_ms: 0,
      disk_write_outcomes: vec![
        written_outcome("src/a.rs", "a"),
        DiskWriteOutcome::PreApplyDriftDetected {
          path: "src/b.rs".into(),
          expected_sha256: "x".into(),
          found_sha256: "y".into(),
        },
        written_outcome("src/c.rs", "c"),
      ],
    };
    assert!(!mixed.all_writes_succeeded());
    assert!(mixed.has_failures());

    // Length mismatch (partial receipt) → !all_writes_succeeded
    // even if every recorded outcome is Written. Guard against
    // executor that stopped early without recording the rest.
    let partial = ToolActionMaterializationReceipt {
      plan: plan.clone(),
      executed_at_ms: 0,
      disk_write_outcomes: vec![written_outcome("src/a.rs", "a")],
    };
    assert_eq!(plan_len, 3);
    assert!(
      !partial.all_writes_succeeded(),
      "length-mismatch must not succeed"
    );
    assert!(partial.has_failures());
  }

  #[test]
  fn receipt_summary_empty_outcomes() {
    let plan = fixture_plan_one_file();
    let receipt = ToolActionMaterializationReceipt {
      plan,
      executed_at_ms: 0,
      disk_write_outcomes: Vec::new(),
    };
    let s = receipt.summary();
    assert_eq!(s.total, 0);
    // Empty outcomes against a 1-file plan = not success.
    assert!(!receipt.all_writes_succeeded());
  }

  #[test]
  fn receipt_serde_round_trip_preserves_outcome_kinds() {
    // Sanity: serde tag = "outcome" preserves the kebab-case
    // discriminant through a serialize/deserialize round-trip.
    let plan = fixture_plan_one_file();
    let receipt = ToolActionMaterializationReceipt {
      plan,
      executed_at_ms: 1800000000000,
      disk_write_outcomes: vec![DiskWriteOutcome::PreApplyDriftDetected {
        path: "src/a.rs".into(),
        expected_sha256: "abc".into(),
        found_sha256: "def".into(),
      }],
    };
    let json = serde_json::to_string(&receipt).expect("serialize");
    assert!(json.contains("\"outcome\":\"pre-apply-drift-detected\""));
    let back: ToolActionMaterializationReceipt = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(
      back.disk_write_outcomes[0].kind_str(),
      "pre-apply-drift-detected"
    );
    assert_eq!(back, receipt);
  }

  // ─── canonical receipt artifact ──────────────────────────────────

  fn fixture_receipt_all_written() -> ToolActionMaterializationReceipt {
    let plan = fixture_plan_three_files();
    ToolActionMaterializationReceipt {
      plan,
      executed_at_ms: 1800000000000,
      disk_write_outcomes: vec![
        written_outcome("src/a.rs", "shaA"),
        written_outcome("src/b.rs", "shaB"),
        written_outcome("src/c.rs", "shaC"),
      ],
    }
  }

  fn fixture_receipt_mixed_outcomes() -> ToolActionMaterializationReceipt {
    let plan = fixture_plan_three_files();
    ToolActionMaterializationReceipt {
      plan,
      executed_at_ms: 1800000000000,
      disk_write_outcomes: vec![
        written_outcome("src/a.rs", "shaA"),
        DiskWriteOutcome::PreApplyDriftDetected {
          path: "src/b.rs".into(),
          expected_sha256: "expected".into(),
          found_sha256: "drifted".into(),
        },
        DiskWriteOutcome::WriteIoError {
          path: "src/c.rs".into(),
          error_kind: "permission-denied".into(),
          error_message: "denied by os".into(),
        },
      ],
    }
  }

  #[test]
  fn receipt_payload_canonical_fields() {
    let receipt = fixture_receipt_all_written();
    let payload = build_tool_action_materialization_receipt_payload(&receipt);
    assert_eq!(
      payload["transform"].as_str(),
      Some("tool-action-materialization")
    );
    assert_eq!(
      payload["apply_receipt_artifact_id"].as_str(),
      Some(receipt.plan.apply_receipt_artifact_id.as_str())
    );
    assert_eq!(payload["executed_at_ms"].as_u64(), Some(1800000000000));
    assert_eq!(payload["all_writes_succeeded"].as_bool(), Some(true));
    assert_eq!(
      payload["next_step"].as_str(),
      Some("verify-or-rollback-or-promote")
    );
    // request is embedded for audit self-containment.
    assert_eq!(
      payload["request"]["capability"].as_str(),
      Some("edit-within-target-paths")
    );
    assert_eq!(
      payload["request"]["requested_by_actor_id"].as_str(),
      Some("actor.user.1")
    );
    // summary counts.
    assert_eq!(payload["summary"]["total"].as_u64(), Some(3));
    assert_eq!(payload["summary"]["written"].as_u64(), Some(3));
    assert_eq!(
      payload["summary"]["pre_apply_drift_detected"].as_u64(),
      Some(0)
    );
    // disk_write_outcomes preserves serde kebab-case `outcome` tag.
    let outcomes = payload["disk_write_outcomes"].as_array().unwrap();
    assert_eq!(outcomes.len(), 3);
    assert_eq!(outcomes[0]["outcome"].as_str(), Some("written"));
    assert_eq!(outcomes[0]["path"].as_str(), Some("src/a.rs"));
  }

  #[test]
  fn receipt_payload_mixed_outcomes_surface_correctly() {
    let receipt = fixture_receipt_mixed_outcomes();
    let payload = build_tool_action_materialization_receipt_payload(&receipt);
    assert_eq!(payload["all_writes_succeeded"].as_bool(), Some(false));
    assert_eq!(payload["summary"]["written"].as_u64(), Some(1));
    assert_eq!(
      payload["summary"]["pre_apply_drift_detected"].as_u64(),
      Some(1)
    );
    assert_eq!(payload["summary"]["write_io_error"].as_u64(), Some(1));
    let outcomes = payload["disk_write_outcomes"].as_array().unwrap();
    assert_eq!(
      outcomes[1]["outcome"].as_str(),
      Some("pre-apply-drift-detected")
    );
    assert_eq!(outcomes[1]["expected_sha256"].as_str(), Some("expected"));
    assert_eq!(outcomes[1]["found_sha256"].as_str(), Some("drifted"));
    assert_eq!(outcomes[2]["outcome"].as_str(), Some("write-io-error"));
    assert_eq!(
      outcomes[2]["error_kind"].as_str(),
      Some("permission-denied")
    );
  }

  #[test]
  fn receipt_artifact_envelope_shape() {
    let receipt = fixture_receipt_all_written();
    let art = build_tool_action_materialization_receipt_artifact(&receipt, 1900000000000);
    assert_eq!(
      art["artifact_family"].as_str(),
      Some("coding.tool-action-materialization-receipt")
    );
    assert_eq!(
      art["source_surface"].as_str(),
      Some("tool-action.materialization")
    );
    assert_eq!(art["stored_at_ms"].as_u64(), Some(1900000000000));
    let id = art["id"].as_str().expect("id");
    assert!(
      id.starts_with("tool-action-materialization-receipt."),
      "id should start with the family prefix, got {id}"
    );
    // target_paths reflects the outcome paths in order.
    let target_paths = art["target_paths"].as_array().unwrap();
    assert_eq!(target_paths.len(), 3);
    assert_eq!(target_paths[0].as_str(), Some("src/a.rs"));
    // related_refs has owner-law + apply-receipt back-ref.
    let related = art["related_refs"].as_array().unwrap();
    assert!(related.iter().any(|v| v
      .as_str()
      .map(|s| s == "owner-law:crates/pnix-core/src/tool_action.rs")
      .unwrap_or(false)));
    assert!(related.iter().any(|v| v
      .as_str()
      .map(|s| s.starts_with("apply-receipt-artifact:apply-receipt."))
      .unwrap_or(false)));
    // repo_snapshot_ref carries through.
    assert_eq!(art["repo_snapshot_ref"].as_str(), Some("git:abc123"));
  }

  #[test]
  fn receipt_artifact_id_replay_stable_across_stored_at_ms() {
    let receipt = fixture_receipt_all_written();
    let a = build_tool_action_materialization_receipt_artifact(&receipt, 1000);
    let b = build_tool_action_materialization_receipt_artifact(&receipt, 9999);
    assert_eq!(a["id"], b["id"], "stored_at_ms is extrinsic to receipt id");
    assert_ne!(a["stored_at_ms"], b["stored_at_ms"]);
  }

  #[test]
  fn receipt_artifact_id_differs_per_outcome_set() {
    // Same plan + request + execution timestamp, different outcomes →
    // different ids (audit can distinguish "all written" from "drift
    // detected on file b").
    let all_written = fixture_receipt_all_written();
    let mixed = fixture_receipt_mixed_outcomes();
    let a = build_tool_action_materialization_receipt_artifact(&all_written, 0);
    let m = build_tool_action_materialization_receipt_artifact(&mixed, 0);
    assert_ne!(a["id"], m["id"]);
  }

  #[test]
  fn receipt_artifact_id_differs_per_executed_at_ms() {
    // Same plan + outcomes, different execution timestamps → distinct
    // ids (each materialization run is a distinct event).
    let mut early = fixture_receipt_all_written();
    early.executed_at_ms = 1700000000000;
    let mut late = fixture_receipt_all_written();
    late.executed_at_ms = 1700000099999;
    let a = build_tool_action_materialization_receipt_artifact(&early, 0);
    let b = build_tool_action_materialization_receipt_artifact(&late, 0);
    assert_ne!(a["id"], b["id"]);
  }

  #[test]
  fn receipt_artifact_includes_drift_sha256_in_hash() {
    // Two receipts with the same shape but different drift sha256s
    // must produce distinct ids — the executor's audit evidence
    // distinguishes "drift to X" from "drift to Y".
    let plan = fixture_plan_one_file();
    let make = |found: &str| ToolActionMaterializationReceipt {
      plan: plan.clone(),
      executed_at_ms: 1800000000000,
      disk_write_outcomes: vec![DiskWriteOutcome::PreApplyDriftDetected {
        path: "src/a.rs".into(),
        expected_sha256: "expected".into(),
        found_sha256: found.into(),
      }],
    };
    let r1 = make("drifted-to-foo");
    let r2 = make("drifted-to-bar");
    let a = build_tool_action_materialization_receipt_artifact(&r1, 0);
    let b = build_tool_action_materialization_receipt_artifact(&r2, 0);
    assert_ne!(a["id"], b["id"]);
  }

  #[test]
  fn held_kind_as_str_round_trip() {
    // Sanity: every variant has a non-empty kebab-case string and the
    // strings are all distinct.
    let kinds = [
      ToolActionMaterializationHeldKind::MissingApplyReceiptArtifactId,
      ToolActionMaterializationHeldKind::MalformedApplyReceiptArtifactId,
      ToolActionMaterializationHeldKind::MissingRepoSnapshotRef,
      ToolActionMaterializationHeldKind::MissingRequestedByActor,
      ToolActionMaterializationHeldKind::MissingRequestedByTenant,
      ToolActionMaterializationHeldKind::MissingCapability,
      ToolActionMaterializationHeldKind::UnrecognizedCapability,
      ToolActionMaterializationHeldKind::ReadOnlyCapabilityCannotMaterialize,
      ToolActionMaterializationHeldKind::ForbiddenCapability,
      ToolActionMaterializationHeldKind::MissingDeploymentMode,
      ToolActionMaterializationHeldKind::UnrecognizedDeploymentMode,
      ToolActionMaterializationHeldKind::MissingContentPolicy,
      ToolActionMaterializationHeldKind::UnrecognizedContentPolicy,
      ToolActionMaterializationHeldKind::CustomerReleaseForbidsIncludeContent,
    ];
    let strings: std::collections::BTreeSet<&str> = kinds.iter().map(|k| k.as_str()).collect();
    assert_eq!(
      strings.len(),
      kinds.len(),
      "all held_kinds must have distinct strings"
    );
    for s in &strings {
      assert!(!s.is_empty());
      // All should be kebab-case.
      assert!(s.chars().all(|c| c.is_ascii_lowercase() || c == '-'));
    }
  }
}
