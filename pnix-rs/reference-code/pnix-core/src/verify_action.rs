//! Verify-action lane — bounded verifier execution receipts.
//!
//! OWNER-LAW (2026-05-11): pnix is an LLM-independent deterministic AI
//! substrate. After an apply-receipt is materialized to disk (the
//! `tool_action` lane), the host can run a *bounded verifier* —
//! pre-approved commands like `cargo test --lib` or `cargo clippy` —
//! against the post-materialization repo state, then emit a verify
//! receipt with one `VerifyCommandOutcome` per command and a derived
//! pass / fail / held verdict.
//!
//! Canonical chain (full 7 stages now):
//!
//! ```text
//!   patch candidate
//!     → review receipt
//!     → apply receipt
//!     → materialization receipt   (host wrote files to disk)
//!     → VERIFY RECEIPT (this lane)
//!     → pass  → promote (or just stop)
//!     → fail  → rollback handle → rollback receipt
//!     → held  → operator decision
//! ```
//!
//! This module owns the typed gate + canonical receipt shape. A host
//! crate (future slice) provides the bounded process runner that
//! produces the per-command outcomes by running each command under a
//! timeout + capture. The pnix-core layer stays I/O-free.
//!
//! Customer-release safety: command stdout / stderr **bodies are not
//! embedded** in the receipt by default — only byte lengths, exit
//! status, and duration. A future content-policy slice can opt the
//! bodies in for `dev` deployment mode. The same fail-closed
//! `CustomerRelease + IncludeContent → reject` discipline as the
//! apply-receipt will apply when that slice lands.

use serde::{Deserialize, Serialize};
use pnix_hash::{Digest, Sha256};

/// Hard upper bound on `timeout_seconds_per_command`. Bigger values
/// are Held — refuse to let a verifier hang forever (an operator
/// can split the work into multiple smaller commands).
///
/// One hour. Tunable in a future slice; the *.px owner law for verify
/// will own this value once it lands.
pub const VERIFY_TIMEOUT_MAX_SECONDS: u64 = 3600;

/// A request to verify a materialized apply-receipt by running
/// bounded commands.
///
/// OWNER-LAW (2026-05-11): pure data. The classifier downstream
/// (`classify_verify_target_request`) decides whether this request
/// is well-formed; the host bounded process runner consumes the
/// Ready'd request and produces the receipt with per-command
/// outcomes.
///
/// `commands` is a list of full command strings. The host is
/// responsible for tokenizing them safely (typical impl uses
/// `shell-words` or equivalent). The strings are **not** parsed
/// here — pnix-core only validates that they're non-empty.
///
/// `repo_snapshot_ref` is the post-materialization snapshot. It
/// should be a fresh git sha or equivalent (a new commit, a stash
/// id, or simply the same sha as the materialization request +
/// "+dirty" if uncommitted).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifyTargetRequest {
  pub materialization_receipt_artifact_id: String,
  pub commands: Vec<String>,
  pub repo_snapshot_ref: String,
  pub requested_by_actor_id: String,
  pub requested_by_tenant_id: String,
  pub requested_at_ms: u64,
  pub timeout_seconds_per_command: u64,
}

/// Held / Rejected outcome kinds from
/// [`classify_verify_target_request`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VerifyRequestHeldKind {
  MissingMaterializationReceiptArtifactId,
  MalformedMaterializationReceiptArtifactId,
  EmptyCommands,
  EmptyCommandString,
  DuplicateCommand,
  MissingRepoSnapshotRef,
  MissingRequestedByActor,
  MissingRequestedByTenant,
  TimeoutZero,
  TimeoutTooLarge,
}

impl VerifyRequestHeldKind {
  pub fn as_str(self) -> &'static str {
    match self {
      Self::MissingMaterializationReceiptArtifactId => {
        "missing-materialization-receipt-artifact-id"
      }
      Self::MalformedMaterializationReceiptArtifactId => {
        "malformed-materialization-receipt-artifact-id"
      }
      Self::EmptyCommands => "empty-commands",
      Self::EmptyCommandString => "empty-command-string",
      Self::DuplicateCommand => "duplicate-command",
      Self::MissingRepoSnapshotRef => "missing-repo-snapshot-ref",
      Self::MissingRequestedByActor => "missing-requested-by-actor",
      Self::MissingRequestedByTenant => "missing-requested-by-tenant",
      Self::TimeoutZero => "timeout-zero",
      Self::TimeoutTooLarge => "timeout-too-large",
    }
  }
}

/// Verdict from [`classify_verify_target_request`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "verdict")]
pub enum VerifyRequestVerdict {
  Ready,
  Held {
    held_kind: VerifyRequestHeldKind,
    reason: String,
  },
  Rejected {
    held_kind: VerifyRequestHeldKind,
    reason: String,
  },
}

/// Pure classifier — applies the verify-request preconditions in
/// order.
///
/// OWNER-LAW (2026-05-11): ladder order:
///
///   1. `materialization_receipt_artifact_id` empty → Held
///   2. `materialization_receipt_artifact_id` malformed → Held
///      (must start with `tool-action-materialization-receipt.`)
///   3. `commands` empty → Held
///   4. any `commands[i]` empty string → Held
///   5. `commands` has duplicates → Held
///   6. `repo_snapshot_ref` empty → Held
///   7. `requested_by_actor_id` empty → Held
///   8. `requested_by_tenant_id` empty → Held
///   9. `timeout_seconds_per_command == 0` → Held
///  10. `timeout_seconds_per_command > VERIFY_TIMEOUT_MAX_SECONDS` →
///      Rejected (hard cap)
pub fn classify_verify_target_request(req: &VerifyTargetRequest) -> VerifyRequestVerdict {
  if req.materialization_receipt_artifact_id.is_empty() {
    return VerifyRequestVerdict::Held {
      held_kind: VerifyRequestHeldKind::MissingMaterializationReceiptArtifactId,
      reason: "materialization_receipt_artifact_id required to bind verify to a specific materialization event".to_string(),
    };
  }
  if !req
    .materialization_receipt_artifact_id
    .starts_with("tool-action-materialization-receipt.")
  {
    return VerifyRequestVerdict::Held {
      held_kind: VerifyRequestHeldKind::MalformedMaterializationReceiptArtifactId,
      reason: format!(
        "materialization_receipt_artifact_id '{}' does not start with 'tool-action-materialization-receipt.' — must be a canonical pnix-core materialization receipt id",
        req.materialization_receipt_artifact_id
      ),
    };
  }
  if req.commands.is_empty() {
    return VerifyRequestVerdict::Held {
      held_kind: VerifyRequestHeldKind::EmptyCommands,
      reason: "commands list is empty — at least one verifier command required".to_string(),
    };
  }
  for c in &req.commands {
    if c.is_empty() {
      return VerifyRequestVerdict::Held {
        held_kind: VerifyRequestHeldKind::EmptyCommandString,
        reason: "commands contains an empty string — every command must be non-empty".to_string(),
      };
    }
  }
  // Duplicate detection — BTreeSet preserves deterministic-failure
  // semantics (returns the first duplicate).
  let mut seen: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
  for c in &req.commands {
    if !seen.insert(c.as_str()) {
      return VerifyRequestVerdict::Held {
        held_kind: VerifyRequestHeldKind::DuplicateCommand,
        reason: format!("commands has duplicate entry '{c}' — each command must be unique"),
      };
    }
  }
  if req.repo_snapshot_ref.is_empty() {
    return VerifyRequestVerdict::Held {
      held_kind: VerifyRequestHeldKind::MissingRepoSnapshotRef,
      reason: "repo_snapshot_ref required — verify must bind to a specific repo state".to_string(),
    };
  }
  if req.requested_by_actor_id.is_empty() {
    return VerifyRequestVerdict::Held {
      held_kind: VerifyRequestHeldKind::MissingRequestedByActor,
      reason: "requested_by_actor_id required — verify is not anonymous".to_string(),
    };
  }
  if req.requested_by_tenant_id.is_empty() {
    return VerifyRequestVerdict::Held {
      held_kind: VerifyRequestHeldKind::MissingRequestedByTenant,
      reason: "requested_by_tenant_id required — verify must be tenant-scoped".to_string(),
    };
  }
  if req.timeout_seconds_per_command == 0 {
    return VerifyRequestVerdict::Held {
      held_kind: VerifyRequestHeldKind::TimeoutZero,
      reason: "timeout_seconds_per_command must be > 0".to_string(),
    };
  }
  if req.timeout_seconds_per_command > VERIFY_TIMEOUT_MAX_SECONDS {
    return VerifyRequestVerdict::Rejected {
      held_kind: VerifyRequestHeldKind::TimeoutTooLarge,
      reason: format!(
        "timeout_seconds_per_command={} exceeds hard cap of {}; split into smaller commands",
        req.timeout_seconds_per_command, VERIFY_TIMEOUT_MAX_SECONDS
      ),
    };
  }
  VerifyRequestVerdict::Ready
}

/// Per-command outcome from the host's bounded process runner.
///
/// OWNER-LAW (2026-05-11): the host runs each command under a
/// timeout, captures stdout/stderr, and emits one of these variants.
/// stdout/stderr **bodies are not in the receipt by default** (only
/// byte lengths) for customer-release safety; a future content
/// policy can opt them in for `dev` deployment mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "outcome")]
pub enum VerifyCommandOutcome {
  /// Command ran to completion with exit status 0.
  CompletedSuccess {
    command: String,
    stdout_byte_len: usize,
    stderr_byte_len: usize,
    duration_ms: u64,
  },
  /// Command ran to completion with a non-zero exit status.
  CompletedFailure {
    command: String,
    exit_status: i32,
    stdout_byte_len: usize,
    stderr_byte_len: usize,
    duration_ms: u64,
  },
  /// Command exceeded `timeout_seconds_per_command`. Process was
  /// killed. Partial stdout/stderr counts are what was captured
  /// before the kill.
  TimedOut {
    command: String,
    timeout_seconds: u64,
    partial_stdout_byte_len: usize,
    partial_stderr_byte_len: usize,
  },
  /// Host couldn't even spawn the command (binary not found,
  /// permission denied on the runner, etc.).
  SpawnFailed {
    command: String,
    error_kind: String,
    error_message: String,
  },
  /// Command was skipped — typically because an earlier command in
  /// the list failed and the runner is configured to stop on first
  /// failure. The `reason` documents why.
  Skipped { command: String, reason: String },
}

impl VerifyCommandOutcome {
  pub fn kind_str(&self) -> &'static str {
    match self {
      Self::CompletedSuccess { .. } => "completed-success",
      Self::CompletedFailure { .. } => "completed-failure",
      Self::TimedOut { .. } => "timed-out",
      Self::SpawnFailed { .. } => "spawn-failed",
      Self::Skipped { .. } => "skipped",
    }
  }

  pub fn command(&self) -> &str {
    match self {
      Self::CompletedSuccess { command, .. }
      | Self::CompletedFailure { command, .. }
      | Self::TimedOut { command, .. }
      | Self::SpawnFailed { command, .. }
      | Self::Skipped { command, .. } => command,
    }
  }

  pub fn is_success(&self) -> bool {
    matches!(self, Self::CompletedSuccess { .. })
  }

  pub fn is_failure(&self) -> bool {
    matches!(
      self,
      Self::CompletedFailure { .. } | Self::TimedOut { .. } | Self::SpawnFailed { .. }
    )
  }

  pub fn is_held(&self) -> bool {
    matches!(self, Self::Skipped { .. })
  }
}

/// Derived verdict over a list of `VerifyCommandOutcome`.
///
/// OWNER-LAW (2026-05-11):
///   - `Pass`: all outcomes are `CompletedSuccess`.
///   - `Fail`: at least one `CompletedFailure` / `TimedOut` /
///     `SpawnFailed`. The reasoning is symmetric to the materialization
///     receipt: any executed failure means the verdict is fail.
///   - `Held`: only success + skipped outcomes (no failures, but
///     some commands didn't run). Operator decision needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VerifyOutcomeVerdict {
  Pass,
  Fail,
  Held,
}

impl VerifyOutcomeVerdict {
  pub fn as_str(self) -> &'static str {
    match self {
      Self::Pass => "pass",
      Self::Fail => "fail",
      Self::Held => "held",
    }
  }

  /// next_step prose for the cockpit panel + audit trail.
  pub fn next_step(self) -> &'static str {
    match self {
      Self::Pass => "promote-or-stop",
      Self::Fail => "open-rollback-handle",
      Self::Held => "operator-decision-or-rerun-skipped",
    }
  }
}

/// Counts of outcomes by kind. Computed once and embedded in the
/// receipt's canonical payload for fast cockpit rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct VerifyOutcomeSummary {
  pub total: usize,
  pub completed_success: usize,
  pub completed_failure: usize,
  pub timed_out: usize,
  pub spawn_failed: usize,
  pub skipped: usize,
}

/// The receipt of a verify-action run — pure data describing what
/// the bounded process runner did per command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifyReceipt {
  pub request: VerifyTargetRequest,
  pub executed_at_ms: u64,
  /// One outcome per command in the request, in request order.
  /// Length must equal `request.commands.len()`.
  pub command_outcomes: Vec<VerifyCommandOutcome>,
}

impl VerifyReceipt {
  /// Compute the per-kind summary counts.
  pub fn summary(&self) -> VerifyOutcomeSummary {
    let mut s = VerifyOutcomeSummary::default();
    for o in &self.command_outcomes {
      s.total += 1;
      match o {
        VerifyCommandOutcome::CompletedSuccess { .. } => s.completed_success += 1,
        VerifyCommandOutcome::CompletedFailure { .. } => s.completed_failure += 1,
        VerifyCommandOutcome::TimedOut { .. } => s.timed_out += 1,
        VerifyCommandOutcome::SpawnFailed { .. } => s.spawn_failed += 1,
        VerifyCommandOutcome::Skipped { .. } => s.skipped += 1,
      }
    }
    s
  }

  /// Derive the overall verdict from the outcomes.
  ///
  /// Pass iff every command succeeded AND there's at least one
  /// outcome AND the outcome count matches the request command
  /// count (partial receipts → Held).
  pub fn verdict(&self) -> VerifyOutcomeVerdict {
    if self.command_outcomes.is_empty() {
      // Empty receipt against a non-empty request → Held.
      return VerifyOutcomeVerdict::Held;
    }
    if self.command_outcomes.len() != self.request.commands.len() {
      // Partial receipt → Held (runner stopped without recording the rest).
      return VerifyOutcomeVerdict::Held;
    }
    let mut has_failure = false;
    let mut has_skip = false;
    let mut all_success = true;
    for o in &self.command_outcomes {
      if o.is_failure() {
        has_failure = true;
        all_success = false;
      } else if o.is_held() {
        has_skip = true;
        all_success = false;
      } else if !o.is_success() {
        all_success = false;
      }
    }
    if has_failure {
      return VerifyOutcomeVerdict::Fail;
    }
    if has_skip {
      return VerifyOutcomeVerdict::Held;
    }
    if all_success {
      return VerifyOutcomeVerdict::Pass;
    }
    VerifyOutcomeVerdict::Held
  }
}

// ─── canonical verify receipt artifact ────────────────────────────────

/// Render a `VerifyReceipt` as the canonical JSON payload of a
/// `coding.verify-receipt` artifact.
///
/// OWNER-LAW (2026-05-11): the receipt embeds the full request +
/// summary + per-command outcomes + derived verdict so the artifact
/// is self-contained for audit. `next_step` is verdict-determined
/// (`promote-or-stop` / `open-rollback-handle` /
/// `operator-decision-or-rerun-skipped`).
pub fn build_verify_receipt_payload(receipt: &VerifyReceipt) -> serde_json::Value {
  let summary = receipt.summary();
  let verdict = receipt.verdict();
  serde_json::json!({
    "transform": "verify-action",
    "materialization_receipt_artifact_id": receipt.request.materialization_receipt_artifact_id,
    "request": receipt.request,
    "executed_at_ms": receipt.executed_at_ms,
    "summary": {
      "total": summary.total,
      "completed_success": summary.completed_success,
      "completed_failure": summary.completed_failure,
      "timed_out": summary.timed_out,
      "spawn_failed": summary.spawn_failed,
      "skipped": summary.skipped,
    },
    "verdict": verdict.as_str(),
    "next_step": verdict.next_step(),
    "command_outcomes": receipt.command_outcomes,
  })
}

/// Wrap a `VerifyReceipt` into a full `coding.verify-receipt`
/// artifact value with a replay-stable id.
///
/// OWNER-LAW (2026-05-11): id hash binds intrinsic verify identity:
///
///   1. `materialization_receipt_artifact_id` (which materialization
///      this verifies)
///   2. `repo_snapshot_ref` + `requested_by_actor_id` /
///      `_tenant_id` + `requested_at_ms`
///   3. command list (canonicalized order — order matters for the
///      receipt because the runner runs in order)
///   4. `executed_at_ms`
///   5. per-outcome: kind + command + per-variant identifying
///      field (exit_status / duration_ms / timeout_seconds /
///      error_kind / skipped-reason)
///
/// `stored_at_ms` is extrinsic and not in the hash. `related_refs`
/// carries `materialization-receipt-artifact:<id>` so audit can
/// walk the chain.
pub fn build_verify_receipt_artifact(
  receipt: &VerifyReceipt,
  stored_at_ms: u64,
) -> serde_json::Value {
  let payload = build_verify_receipt_payload(receipt);
  let request = &receipt.request;

  let mut hasher = Sha256::new();
  hasher.update(b"verify-receipt\x1f");
  hasher.update(request.materialization_receipt_artifact_id.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(request.repo_snapshot_ref.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(request.requested_by_actor_id.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(request.requested_by_tenant_id.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(request.requested_at_ms.to_le_bytes());
  hasher.update(b"\x1f");
  hasher.update(request.timeout_seconds_per_command.to_le_bytes());
  hasher.update(b"\x1f");
  for c in &request.commands {
    hasher.update(c.as_bytes());
    hasher.update(b"\x1e");
  }
  hasher.update(b"\x1f");
  hasher.update(receipt.executed_at_ms.to_le_bytes());
  hasher.update(b"\x1f");
  for o in &receipt.command_outcomes {
    hasher.update(o.kind_str().as_bytes());
    hasher.update(b"\x1e");
    hasher.update(o.command().as_bytes());
    hasher.update(b"\x1d");
    match o {
      VerifyCommandOutcome::CompletedSuccess { duration_ms, .. } => {
        hasher.update(duration_ms.to_le_bytes());
      }
      VerifyCommandOutcome::CompletedFailure {
        exit_status,
        duration_ms,
        ..
      } => {
        hasher.update(exit_status.to_le_bytes());
        hasher.update(b"\x1c");
        hasher.update(duration_ms.to_le_bytes());
      }
      VerifyCommandOutcome::TimedOut {
        timeout_seconds, ..
      } => {
        hasher.update(timeout_seconds.to_le_bytes());
      }
      VerifyCommandOutcome::SpawnFailed { error_kind, .. } => {
        hasher.update(error_kind.as_bytes());
      }
      VerifyCommandOutcome::Skipped { reason, .. } => {
        hasher.update(reason.as_bytes());
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
  let id = format!("verify-receipt.{prefix}");

  serde_json::json!({
    "id": id,
    "artifact_family": "coding.verify-receipt",
    "source_surface": "verify-action.bounded-runner",
    "stored_at_ms": stored_at_ms,
    "target_paths": serde_json::Value::Array(Vec::new()),
    "command_refs": request.commands,
    "related_refs": [
      "owner-law:crates/pnix-core/src/verify_action.rs",
      format!(
        "materialization-receipt-artifact:{}",
        request.materialization_receipt_artifact_id
      ),
    ],
    "repo_snapshot_ref": request.repo_snapshot_ref,
    "payload": payload,
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  fn req() -> VerifyTargetRequest {
    VerifyTargetRequest {
      materialization_receipt_artifact_id: "tool-action-materialization-receipt.deadbeef"
        .to_string(),
      commands: vec![
        "cargo test --lib".to_string(),
        "cargo clippy -- -D warnings".to_string(),
      ],
      repo_snapshot_ref: "git:abc123".to_string(),
      requested_by_actor_id: "actor.user.1".to_string(),
      requested_by_tenant_id: "tenant.alpha".to_string(),
      requested_at_ms: 1700000000000,
      timeout_seconds_per_command: 300,
    }
  }

  // ─── classifier ──────────────────────────────────────────────────

  #[test]
  fn ready_when_all_preconditions_pass() {
    assert!(matches!(
      classify_verify_target_request(&req()),
      VerifyRequestVerdict::Ready
    ));
  }

  #[test]
  fn held_when_materialization_receipt_id_empty() {
    let mut r = req();
    r.materialization_receipt_artifact_id = String::new();
    assert!(matches!(
      classify_verify_target_request(&r),
      VerifyRequestVerdict::Held {
        held_kind: VerifyRequestHeldKind::MissingMaterializationReceiptArtifactId,
        ..
      }
    ));
  }

  #[test]
  fn held_when_materialization_receipt_id_malformed() {
    let mut r = req();
    r.materialization_receipt_artifact_id = "apply-receipt.x.y".to_string();
    assert!(matches!(
      classify_verify_target_request(&r),
      VerifyRequestVerdict::Held {
        held_kind: VerifyRequestHeldKind::MalformedMaterializationReceiptArtifactId,
        ..
      }
    ));
  }

  #[test]
  fn held_when_commands_empty() {
    let mut r = req();
    r.commands.clear();
    assert!(matches!(
      classify_verify_target_request(&r),
      VerifyRequestVerdict::Held {
        held_kind: VerifyRequestHeldKind::EmptyCommands,
        ..
      }
    ));
  }

  #[test]
  fn held_when_command_string_empty() {
    let mut r = req();
    r.commands.push(String::new());
    assert!(matches!(
      classify_verify_target_request(&r),
      VerifyRequestVerdict::Held {
        held_kind: VerifyRequestHeldKind::EmptyCommandString,
        ..
      }
    ));
  }

  #[test]
  fn held_when_duplicate_command() {
    let mut r = req();
    r.commands.push("cargo test --lib".to_string()); // duplicate
    assert!(matches!(
      classify_verify_target_request(&r),
      VerifyRequestVerdict::Held {
        held_kind: VerifyRequestHeldKind::DuplicateCommand,
        ..
      }
    ));
  }

  #[test]
  fn held_when_repo_snapshot_ref_empty() {
    let mut r = req();
    r.repo_snapshot_ref = String::new();
    assert!(matches!(
      classify_verify_target_request(&r),
      VerifyRequestVerdict::Held {
        held_kind: VerifyRequestHeldKind::MissingRepoSnapshotRef,
        ..
      }
    ));
  }

  #[test]
  fn held_when_actor_or_tenant_empty() {
    let mut r = req();
    r.requested_by_actor_id = String::new();
    assert!(matches!(
      classify_verify_target_request(&r),
      VerifyRequestVerdict::Held {
        held_kind: VerifyRequestHeldKind::MissingRequestedByActor,
        ..
      }
    ));
    let mut r = req();
    r.requested_by_tenant_id = String::new();
    assert!(matches!(
      classify_verify_target_request(&r),
      VerifyRequestVerdict::Held {
        held_kind: VerifyRequestHeldKind::MissingRequestedByTenant,
        ..
      }
    ));
  }

  #[test]
  fn held_when_timeout_zero() {
    let mut r = req();
    r.timeout_seconds_per_command = 0;
    assert!(matches!(
      classify_verify_target_request(&r),
      VerifyRequestVerdict::Held {
        held_kind: VerifyRequestHeldKind::TimeoutZero,
        ..
      }
    ));
  }

  #[test]
  fn rejected_when_timeout_too_large() {
    let mut r = req();
    r.timeout_seconds_per_command = VERIFY_TIMEOUT_MAX_SECONDS + 1;
    match classify_verify_target_request(&r) {
      VerifyRequestVerdict::Rejected { held_kind, reason } => {
        assert_eq!(held_kind, VerifyRequestHeldKind::TimeoutTooLarge);
        assert!(reason.contains("3600"));
      }
      other => panic!("expected TimeoutTooLarge rejection, got {:?}", other),
    }
  }

  #[test]
  fn ready_at_timeout_max_boundary() {
    // Boundary: exactly == MAX is allowed; > MAX rejected.
    let mut r = req();
    r.timeout_seconds_per_command = VERIFY_TIMEOUT_MAX_SECONDS;
    assert!(matches!(
      classify_verify_target_request(&r),
      VerifyRequestVerdict::Ready
    ));
  }

  #[test]
  fn ladder_order_materialization_id_wins() {
    let mut r = req();
    r.materialization_receipt_artifact_id = String::new();
    r.commands.clear();
    r.repo_snapshot_ref = String::new();
    assert!(matches!(
      classify_verify_target_request(&r),
      VerifyRequestVerdict::Held {
        held_kind: VerifyRequestHeldKind::MissingMaterializationReceiptArtifactId,
        ..
      }
    ));
  }

  // ─── command outcome enum ────────────────────────────────────────

  #[test]
  fn outcome_kind_str_and_command_helpers() {
    let cases: Vec<(VerifyCommandOutcome, &str)> = vec![
      (
        VerifyCommandOutcome::CompletedSuccess {
          command: "cargo test".into(),
          stdout_byte_len: 100,
          stderr_byte_len: 0,
          duration_ms: 5000,
        },
        "completed-success",
      ),
      (
        VerifyCommandOutcome::CompletedFailure {
          command: "cargo clippy".into(),
          exit_status: 1,
          stdout_byte_len: 50,
          stderr_byte_len: 200,
          duration_ms: 4000,
        },
        "completed-failure",
      ),
      (
        VerifyCommandOutcome::TimedOut {
          command: "cargo bench".into(),
          timeout_seconds: 60,
          partial_stdout_byte_len: 0,
          partial_stderr_byte_len: 0,
        },
        "timed-out",
      ),
      (
        VerifyCommandOutcome::SpawnFailed {
          command: "nonexistent-tool".into(),
          error_kind: "not-found".into(),
          error_message: "binary not in $PATH".into(),
        },
        "spawn-failed",
      ),
      (
        VerifyCommandOutcome::Skipped {
          command: "cargo doc".into(),
          reason: "earlier command failed".into(),
        },
        "skipped",
      ),
    ];
    for (outcome, expected_kind) in cases {
      assert_eq!(outcome.kind_str(), expected_kind);
      assert!(!outcome.command().is_empty());
    }
  }

  #[test]
  fn outcome_is_success_only_for_completed_success() {
    let s = VerifyCommandOutcome::CompletedSuccess {
      command: "x".into(),
      stdout_byte_len: 0,
      stderr_byte_len: 0,
      duration_ms: 1,
    };
    let f = VerifyCommandOutcome::CompletedFailure {
      command: "x".into(),
      exit_status: 1,
      stdout_byte_len: 0,
      stderr_byte_len: 0,
      duration_ms: 1,
    };
    let t = VerifyCommandOutcome::TimedOut {
      command: "x".into(),
      timeout_seconds: 60,
      partial_stdout_byte_len: 0,
      partial_stderr_byte_len: 0,
    };
    let sp = VerifyCommandOutcome::SpawnFailed {
      command: "x".into(),
      error_kind: "k".into(),
      error_message: "m".into(),
    };
    let sk = VerifyCommandOutcome::Skipped {
      command: "x".into(),
      reason: "r".into(),
    };
    assert!(s.is_success());
    assert!(!f.is_success() && f.is_failure());
    assert!(!t.is_success() && t.is_failure());
    assert!(!sp.is_success() && sp.is_failure());
    assert!(!sk.is_success() && !sk.is_failure() && sk.is_held());
  }

  // ─── verdict derivation ──────────────────────────────────────────

  fn receipt_with_outcomes(outcomes: Vec<VerifyCommandOutcome>) -> VerifyReceipt {
    let mut request = req();
    // Match commands count to outcomes count so verdict() doesn't
    // bail on partial-receipt detection.
    request.commands = outcomes.iter().map(|o| o.command().to_string()).collect();
    VerifyReceipt {
      request,
      executed_at_ms: 1800000000000,
      command_outcomes: outcomes,
    }
  }

  #[test]
  fn verdict_pass_when_all_success() {
    let r = receipt_with_outcomes(vec![
      VerifyCommandOutcome::CompletedSuccess {
        command: "cargo test --lib".into(),
        stdout_byte_len: 0,
        stderr_byte_len: 0,
        duration_ms: 5000,
      },
      VerifyCommandOutcome::CompletedSuccess {
        command: "cargo clippy".into(),
        stdout_byte_len: 0,
        stderr_byte_len: 0,
        duration_ms: 3000,
      },
    ]);
    assert_eq!(r.verdict(), VerifyOutcomeVerdict::Pass);
    assert_eq!(r.verdict().next_step(), "promote-or-stop");
  }

  #[test]
  fn verdict_fail_when_any_failure() {
    let r = receipt_with_outcomes(vec![
      VerifyCommandOutcome::CompletedSuccess {
        command: "cargo test --lib".into(),
        stdout_byte_len: 0,
        stderr_byte_len: 0,
        duration_ms: 5000,
      },
      VerifyCommandOutcome::CompletedFailure {
        command: "cargo clippy".into(),
        exit_status: 101,
        stdout_byte_len: 0,
        stderr_byte_len: 200,
        duration_ms: 3000,
      },
    ]);
    assert_eq!(r.verdict(), VerifyOutcomeVerdict::Fail);
    assert_eq!(r.verdict().next_step(), "open-rollback-handle");
  }

  #[test]
  fn verdict_fail_when_timed_out() {
    let r = receipt_with_outcomes(vec![VerifyCommandOutcome::TimedOut {
      command: "cargo bench".into(),
      timeout_seconds: 60,
      partial_stdout_byte_len: 0,
      partial_stderr_byte_len: 0,
    }]);
    assert_eq!(r.verdict(), VerifyOutcomeVerdict::Fail);
  }

  #[test]
  fn verdict_fail_when_spawn_failed() {
    let r = receipt_with_outcomes(vec![VerifyCommandOutcome::SpawnFailed {
      command: "missing-tool".into(),
      error_kind: "not-found".into(),
      error_message: "x".into(),
    }]);
    assert_eq!(r.verdict(), VerifyOutcomeVerdict::Fail);
  }

  #[test]
  fn verdict_held_when_skipped_present_and_no_failures() {
    let r = receipt_with_outcomes(vec![
      VerifyCommandOutcome::CompletedSuccess {
        command: "cargo test --lib".into(),
        stdout_byte_len: 0,
        stderr_byte_len: 0,
        duration_ms: 1000,
      },
      VerifyCommandOutcome::Skipped {
        command: "cargo doc".into(),
        reason: "operator paused".into(),
      },
    ]);
    assert_eq!(r.verdict(), VerifyOutcomeVerdict::Held);
    assert_eq!(
      r.verdict().next_step(),
      "operator-decision-or-rerun-skipped"
    );
  }

  #[test]
  fn verdict_held_when_empty_outcomes() {
    let mut r = receipt_with_outcomes(vec![]);
    r.request.commands = vec!["cargo test".to_string()];
    assert_eq!(r.verdict(), VerifyOutcomeVerdict::Held);
  }

  #[test]
  fn verdict_held_when_partial_receipt() {
    let mut r = receipt_with_outcomes(vec![VerifyCommandOutcome::CompletedSuccess {
      command: "cargo test --lib".into(),
      stdout_byte_len: 0,
      stderr_byte_len: 0,
      duration_ms: 1,
    }]);
    // Request claims 2 commands; receipt has only 1 outcome → Held.
    r.request.commands = vec!["cargo test --lib".to_string(), "cargo clippy".to_string()];
    assert_eq!(r.verdict(), VerifyOutcomeVerdict::Held);
  }

  #[test]
  fn verdict_fail_dominates_skipped() {
    // A failure plus a skipped: verdict is Fail (not Held), because
    // the failure forces a rollback decision regardless of skipped.
    let r = receipt_with_outcomes(vec![
      VerifyCommandOutcome::CompletedFailure {
        command: "cargo test --lib".into(),
        exit_status: 1,
        stdout_byte_len: 0,
        stderr_byte_len: 0,
        duration_ms: 5000,
      },
      VerifyCommandOutcome::Skipped {
        command: "cargo clippy".into(),
        reason: "earlier command failed".into(),
      },
    ]);
    assert_eq!(r.verdict(), VerifyOutcomeVerdict::Fail);
  }

  // ─── summary ─────────────────────────────────────────────────────

  #[test]
  fn summary_counts_all_kinds() {
    let r = receipt_with_outcomes(vec![
      VerifyCommandOutcome::CompletedSuccess {
        command: "a".into(),
        stdout_byte_len: 0,
        stderr_byte_len: 0,
        duration_ms: 1,
      },
      VerifyCommandOutcome::CompletedFailure {
        command: "b".into(),
        exit_status: 1,
        stdout_byte_len: 0,
        stderr_byte_len: 0,
        duration_ms: 1,
      },
      VerifyCommandOutcome::TimedOut {
        command: "c".into(),
        timeout_seconds: 60,
        partial_stdout_byte_len: 0,
        partial_stderr_byte_len: 0,
      },
      VerifyCommandOutcome::SpawnFailed {
        command: "d".into(),
        error_kind: "k".into(),
        error_message: "m".into(),
      },
      VerifyCommandOutcome::Skipped {
        command: "e".into(),
        reason: "r".into(),
      },
    ]);
    let s = r.summary();
    assert_eq!(s.total, 5);
    assert_eq!(s.completed_success, 1);
    assert_eq!(s.completed_failure, 1);
    assert_eq!(s.timed_out, 1);
    assert_eq!(s.spawn_failed, 1);
    assert_eq!(s.skipped, 1);
  }

  // ─── canonical artifact ──────────────────────────────────────────

  fn fixture_pass_receipt() -> VerifyReceipt {
    receipt_with_outcomes(vec![
      VerifyCommandOutcome::CompletedSuccess {
        command: "cargo test --lib".into(),
        stdout_byte_len: 1024,
        stderr_byte_len: 0,
        duration_ms: 5000,
      },
      VerifyCommandOutcome::CompletedSuccess {
        command: "cargo clippy -- -D warnings".into(),
        stdout_byte_len: 0,
        stderr_byte_len: 0,
        duration_ms: 3000,
      },
    ])
  }

  fn fixture_fail_receipt() -> VerifyReceipt {
    receipt_with_outcomes(vec![
      VerifyCommandOutcome::CompletedFailure {
        command: "cargo test --lib".into(),
        exit_status: 101,
        stdout_byte_len: 100,
        stderr_byte_len: 500,
        duration_ms: 4500,
      },
      VerifyCommandOutcome::Skipped {
        command: "cargo clippy -- -D warnings".into(),
        reason: "earlier command failed".into(),
      },
    ])
  }

  #[test]
  fn payload_canonical_fields_for_pass() {
    let r = fixture_pass_receipt();
    let payload = build_verify_receipt_payload(&r);
    assert_eq!(payload["transform"].as_str(), Some("verify-action"));
    assert_eq!(payload["verdict"].as_str(), Some("pass"));
    assert_eq!(payload["next_step"].as_str(), Some("promote-or-stop"));
    assert_eq!(payload["summary"]["total"].as_u64(), Some(2));
    assert_eq!(payload["summary"]["completed_success"].as_u64(), Some(2));
    assert_eq!(payload["summary"]["completed_failure"].as_u64(), Some(0));
    // Request embedded for audit.
    assert_eq!(
      payload["request"]["timeout_seconds_per_command"].as_u64(),
      Some(300)
    );
    // command_outcomes preserves kebab-case `outcome` tag.
    let outcomes = payload["command_outcomes"].as_array().unwrap();
    assert_eq!(outcomes[0]["outcome"].as_str(), Some("completed-success"));
    assert_eq!(outcomes[0]["duration_ms"].as_u64(), Some(5000));
  }

  #[test]
  fn payload_canonical_fields_for_fail() {
    let r = fixture_fail_receipt();
    let payload = build_verify_receipt_payload(&r);
    assert_eq!(payload["verdict"].as_str(), Some("fail"));
    assert_eq!(payload["next_step"].as_str(), Some("open-rollback-handle"));
    assert_eq!(payload["summary"]["completed_failure"].as_u64(), Some(1));
    assert_eq!(payload["summary"]["skipped"].as_u64(), Some(1));
    let outcomes = payload["command_outcomes"].as_array().unwrap();
    assert_eq!(outcomes[0]["outcome"].as_str(), Some("completed-failure"));
    assert_eq!(outcomes[0]["exit_status"].as_i64(), Some(101));
    assert_eq!(outcomes[1]["outcome"].as_str(), Some("skipped"));
    assert_eq!(
      outcomes[1]["reason"].as_str(),
      Some("earlier command failed")
    );
  }

  #[test]
  fn artifact_envelope_shape() {
    let r = fixture_pass_receipt();
    let art = build_verify_receipt_artifact(&r, 1900000000000);
    assert_eq!(
      art["artifact_family"].as_str(),
      Some("coding.verify-receipt")
    );
    assert_eq!(
      art["source_surface"].as_str(),
      Some("verify-action.bounded-runner")
    );
    assert_eq!(art["stored_at_ms"].as_u64(), Some(1900000000000));
    let id = art["id"].as_str().expect("id");
    assert!(id.starts_with("verify-receipt."));
    // command_refs surfaces the verifier commands at envelope level.
    let cmd_refs = art["command_refs"].as_array().expect("command_refs");
    assert_eq!(cmd_refs.len(), 2);
    assert_eq!(cmd_refs[0].as_str(), Some("cargo test --lib"));
    // related_refs has owner-law + materialization-receipt back-ref.
    let related = art["related_refs"].as_array().unwrap();
    assert!(related.iter().any(|v| v
      .as_str()
      .map(|s| s == "owner-law:crates/pnix-core/src/verify_action.rs")
      .unwrap_or(false)));
    assert!(related.iter().any(|v| v
      .as_str()
      .map(
        |s| s.starts_with("materialization-receipt-artifact:tool-action-materialization-receipt.")
      )
      .unwrap_or(false)));
    assert_eq!(art["repo_snapshot_ref"].as_str(), Some("git:abc123"));
  }

  #[test]
  fn artifact_id_replay_stable_across_stored_at_ms() {
    let r = fixture_pass_receipt();
    let a = build_verify_receipt_artifact(&r, 1000);
    let b = build_verify_receipt_artifact(&r, 9999);
    assert_eq!(a["id"], b["id"]);
    assert_ne!(a["stored_at_ms"], b["stored_at_ms"]);
  }

  #[test]
  fn artifact_id_differs_per_outcome_set() {
    let pass = fixture_pass_receipt();
    let fail = fixture_fail_receipt();
    let a = build_verify_receipt_artifact(&pass, 0);
    let b = build_verify_receipt_artifact(&fail, 0);
    assert_ne!(a["id"], b["id"]);
  }

  #[test]
  fn artifact_id_differs_per_executed_at_ms() {
    let mut a = fixture_pass_receipt();
    a.executed_at_ms = 1700000000000;
    let mut b = fixture_pass_receipt();
    b.executed_at_ms = 1800000000000;
    let art_a = build_verify_receipt_artifact(&a, 0);
    let art_b = build_verify_receipt_artifact(&b, 0);
    assert_ne!(art_a["id"], art_b["id"]);
  }

  #[test]
  fn artifact_id_differs_when_command_list_differs() {
    // Same outcomes count + kinds, different commands → distinct
    // receipt ids. Audit distinguishes "ran cargo test" vs "ran
    // cargo clippy" even when both passed.
    let a = receipt_with_outcomes(vec![VerifyCommandOutcome::CompletedSuccess {
      command: "cargo test --lib".into(),
      stdout_byte_len: 0,
      stderr_byte_len: 0,
      duration_ms: 1,
    }]);
    let b = receipt_with_outcomes(vec![VerifyCommandOutcome::CompletedSuccess {
      command: "cargo build".into(),
      stdout_byte_len: 0,
      stderr_byte_len: 0,
      duration_ms: 1,
    }]);
    let art_a = build_verify_receipt_artifact(&a, 0);
    let art_b = build_verify_receipt_artifact(&b, 0);
    assert_ne!(art_a["id"], art_b["id"]);
  }

  #[test]
  fn serde_round_trip_preserves_outcome_kinds() {
    let r = fixture_fail_receipt();
    let json = serde_json::to_string(&r).expect("serialize");
    assert!(json.contains("\"outcome\":\"completed-failure\""));
    assert!(json.contains("\"outcome\":\"skipped\""));
    let back: VerifyReceipt = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back, r);
  }
}
