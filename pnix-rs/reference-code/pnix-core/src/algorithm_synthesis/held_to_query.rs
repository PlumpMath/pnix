//! Held → retrieval-query owner.
//!
//! OWNER-LAW (2026-05-12): first owner of the **evolution lane**.
//! Mirror of `stdlib/lib/gate/algorithm-synthesis/held-to-query.px`.
//! Consumes a parameter-resolution `Held` verdict and emits a
//! typed retrieval-query candidate naming what evidence is needed
//! and which downstream channel should recover it.
//!
//! No retrieval is executed here. The downstream consumer is one of:
//!   - host symbol resolver (LSP / compiler / linter)
//!   - external knowledge search (docset / web / package registry)
//!   - ankh-retrieval (pnix's own accumulated knowledge)
//!   - operator follow-up surface (cockpit prompt)
//!   - not-recoverable (rejected, restructure required)
//!
//! Closed loop: NL → synthesis → Held → query → retrieval →
//! `KnowledgeRecord` / `FactCue` → re-enter synthesis as
//! `supplied_facts`.

use serde::{Deserialize, Serialize};
use pnix_hash::{Digest, Sha256};
use std::collections::BTreeMap;

use super::parameter_resolution::{ResolutionHeldKind, ResolutionVerdict};

/// Recovery channel that the downstream consumer should pick up the
/// query from. Sync test asserts parity against `.px`
/// `validRecoveryChannels`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HeldQueryRecoveryChannel {
  OperatorFollowup,
  HostSymbolResolver,
  ExternalKnowledgeSearch,
  AnkhRetrieval,
  NotRecoverable,
}

impl HeldQueryRecoveryChannel {
  pub const ALL: &'static [Self] = &[
    Self::OperatorFollowup,
    Self::HostSymbolResolver,
    Self::ExternalKnowledgeSearch,
    Self::AnkhRetrieval,
    Self::NotRecoverable,
  ];

  pub fn as_str(self) -> &'static str {
    match self {
      Self::OperatorFollowup => "operator-followup",
      Self::HostSymbolResolver => "host-symbol-resolver",
      Self::ExternalKnowledgeSearch => "external-knowledge-search",
      Self::AnkhRetrieval => "ankh-retrieval",
      Self::NotRecoverable => "not-recoverable",
    }
  }
}

/// Routing entry: per held-kind, the primary recovery channel and
/// optional fallback. ankh-retrieval is implicit *before* the
/// primary on every recoverable kind — the downstream owner is
/// expected to try ankh first and fall through to `primary` on miss.
#[derive(Debug, Clone, Copy)]
pub struct HeldRoutingEntry {
  pub held: ResolutionHeldKind,
  pub primary: HeldQueryRecoveryChannel,
  pub fallback: Option<HeldQueryRecoveryChannel>,
  pub query_kind: &'static str,
}

/// Per-query-kind message-template row. Mirror of `.px`
/// `queryMessageTemplates`. The template uses brace-style named
/// placeholders (`{path}`, `{language}`, `{transform}`) that
/// `render_query_text` substitutes generically.
#[derive(Debug, Clone, Copy)]
pub struct QueryMessageTemplate {
  pub query_kind: &'static str,
  pub template: &'static str,
}

pub const QUERY_MESSAGE_TEMPLATES: &[QueryMessageTemplate] = &[
  QueryMessageTemplate {
    query_kind: "operator-asks-old-symbol",
    template: "Which symbol does the operator want renamed in {path}?",
  },
  QueryMessageTemplate {
    query_kind: "operator-asks-new-symbol",
    template: "What new name does the operator want for the rename in {path}?",
  },
  QueryMessageTemplate {
    query_kind: "operator-asks-target-file",
    template: "Which file should the {transform} transform target?",
  },
  QueryMessageTemplate {
    query_kind: "operator-asks-language",
    template:
      "What language is the target file written in (its extension is not recognized)?",
  },
  QueryMessageTemplate {
    query_kind: "host-lints-unused-imports",
    template:
      "Run the host's symbol resolver on {path} and list unused imports for {language}.",
  },
  QueryMessageTemplate {
    query_kind: "operator-asks-test-name",
    template: "What should the new test in {path} be named?",
  },
  QueryMessageTemplate {
    query_kind: "lookup-module-providing-symbol",
    template: "Which {language} module provides the missing symbol referenced in {path}?",
  },
  QueryMessageTemplate {
    query_kind: "extend-resolver-implementation",
    template:
      "parameter-resolution does not yet support transform `{transform}`. Implement a resolver for it.",
  },
  QueryMessageTemplate {
    query_kind: "operator-rephrase-identifier",
    template:
      "Operator's input contained a non-identifier token; ask them to rephrase.",
  },
  QueryMessageTemplate {
    query_kind: "operator-rephrase-nontrivial-rename",
    template:
      "Operator asked to rename a symbol to itself; ask them for the actual target name.",
  },
  QueryMessageTemplate {
    query_kind: "lookup-algebraic-equivalent",
    template:
      "Find an algebraic equivalent for the given `canonical_form` in language `{language}`.",
  },
  QueryMessageTemplate {
    query_kind: "lookup-chemical-reaction",
    template:
      "Find the products of the given `reactants` under conditions `{conditions}` (lang `{language}`).",
  },
];

pub const HELD_ROUTING: &[HeldRoutingEntry] = &[
  HeldRoutingEntry {
    held: ResolutionHeldKind::MissingOldName,
    primary: HeldQueryRecoveryChannel::OperatorFollowup,
    fallback: None,
    query_kind: "operator-asks-old-symbol",
  },
  HeldRoutingEntry {
    held: ResolutionHeldKind::MissingNewName,
    primary: HeldQueryRecoveryChannel::OperatorFollowup,
    fallback: None,
    query_kind: "operator-asks-new-symbol",
  },
  HeldRoutingEntry {
    held: ResolutionHeldKind::MissingTargetPath,
    primary: HeldQueryRecoveryChannel::OperatorFollowup,
    fallback: Some(HeldQueryRecoveryChannel::HostSymbolResolver),
    query_kind: "operator-asks-target-file",
  },
  HeldRoutingEntry {
    held: ResolutionHeldKind::LanguageNotDerivable,
    primary: HeldQueryRecoveryChannel::OperatorFollowup,
    fallback: None,
    query_kind: "operator-asks-language",
  },
  HeldRoutingEntry {
    held: ResolutionHeldKind::MissingCandidateImports,
    primary: HeldQueryRecoveryChannel::HostSymbolResolver,
    fallback: Some(HeldQueryRecoveryChannel::OperatorFollowup),
    query_kind: "host-lints-unused-imports",
  },
  HeldRoutingEntry {
    held: ResolutionHeldKind::MissingTestName,
    primary: HeldQueryRecoveryChannel::OperatorFollowup,
    fallback: Some(HeldQueryRecoveryChannel::ExternalKnowledgeSearch),
    query_kind: "operator-asks-test-name",
  },
  HeldRoutingEntry {
    held: ResolutionHeldKind::MissingImportSpec,
    primary: HeldQueryRecoveryChannel::HostSymbolResolver,
    fallback: Some(HeldQueryRecoveryChannel::ExternalKnowledgeSearch),
    query_kind: "lookup-module-providing-symbol",
  },
  HeldRoutingEntry {
    held: ResolutionHeldKind::TransformNotSupportedByResolver,
    primary: HeldQueryRecoveryChannel::NotRecoverable,
    fallback: None,
    query_kind: "extend-resolver-implementation",
  },
  HeldRoutingEntry {
    held: ResolutionHeldKind::InvalidIdentifier,
    primary: HeldQueryRecoveryChannel::NotRecoverable,
    fallback: None,
    query_kind: "operator-rephrase-identifier",
  },
  HeldRoutingEntry {
    held: ResolutionHeldKind::OldEqualsNew,
    primary: HeldQueryRecoveryChannel::NotRecoverable,
    fallback: None,
    query_kind: "operator-rephrase-nontrivial-rename",
  },
  // Math-domain row. In v0, primary = HostSymbolResolver — the only
  // Implemented channel that consults caller-supplied host evidence.
  // The CAS adapter (sympy / Mathematica / etc.) lands as host
  // evidence under the `equivalent_form` slot, exactly the same
  // mechanism the coding lane uses for `import_spec`. When a true
  // ExternalKnowledgeSearch lane is wired (CAS adapter as separate
  // channel), this row moves; for now the substrate-sharing claim
  // is that the same dispatcher mechanism handles both domains.
  // Fallback = OperatorFollowup so the cockpit can ask the user.
  // `try_ankh_first` is implicit — math identities recur, so ankh
  // hits are the steady-state hot path.
  HeldRoutingEntry {
    held: ResolutionHeldKind::MissingAlgebraicEquivalent,
    primary: HeldQueryRecoveryChannel::HostSymbolResolver,
    fallback: Some(HeldQueryRecoveryChannel::OperatorFollowup),
    query_kind: "lookup-algebraic-equivalent",
  },
  // Chemistry row — same `HostSymbolResolver` stand-in pattern as
  // math. A CAS / chemistry-DB adapter would lower into the same
  // host-evidence injection mechanism.
  HeldRoutingEntry {
    held: ResolutionHeldKind::MissingChemistryProducts,
    primary: HeldQueryRecoveryChannel::HostSymbolResolver,
    fallback: Some(HeldQueryRecoveryChannel::OperatorFollowup),
    query_kind: "lookup-chemical-reaction",
  },
];

/// A typed retrieval-query candidate emitted from a Held verdict.
/// Pure data — the downstream retrieval owner consumes this.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeldRetrievalQuery {
  /// `query_kind` from the routing table — the downstream consumer
  /// uses this to dispatch to the right retrieval implementation
  /// (e.g. `lookup-module-providing-symbol` routes to a docset /
  /// package-index retriever; `host-lints-unused-imports` routes
  /// to an LSP / lint adapter).
  pub query_kind: String,
  /// Held kind this query was emitted from. Audit trail.
  pub held_kind: ResolutionHeldKind,
  /// Transform whose resolution Held'd. Audit trail.
  pub transform: String,
  /// Primary recovery channel.
  pub primary_channel: HeldQueryRecoveryChannel,
  /// Optional fallback channel.
  pub fallback_channel: Option<HeldQueryRecoveryChannel>,
  /// Human-readable query string. Built by interpolating
  /// `partial_resolution` values into a per-kind template. Used by
  /// retrieval lanes that need a query string (web/docset search);
  /// `host-symbol-resolver` lanes ignore this and use
  /// `evidence_to_recover` + `context_fields` directly.
  pub query_text: String,
  /// Names of the missing fields the query is trying to recover.
  /// Mirrors `Held.missing_slots`. The downstream retrieval owner's
  /// output (a `FactCue` or `KnowledgeRecord`) is expected to fill
  /// these.
  pub evidence_to_recover: Vec<String>,
  /// Context to anchor the query — partial_resolution values from
  /// the Held verdict, used both for query interpolation and for
  /// host symbol resolver context.
  pub context_fields: BTreeMap<String, String>,
  /// When ankh has already retrieved an answer for the same
  /// (query_kind, context_fields) tuple, the consumer can short-
  /// circuit. v0: this hint is always `true` (try ankh first); v1
  /// could mark certain ephemeral query kinds as ankh-skipping.
  pub try_ankh_first: bool,
  /// Human-readable Held reason, carried verbatim for audit / UX.
  pub reason: String,
}

/// Look up the routing entry for a held kind. Returns `None` only
/// if the held kind is not yet wired into the routing table — which
/// the sync parity test catches.
fn routing_for(held: ResolutionHeldKind) -> Option<&'static HeldRoutingEntry> {
  HELD_ROUTING.iter().find(|e| e.held == held)
}

/// Resolve `{path}` from a context BTreeMap, accepting several
/// per-transform aliases. Keeps `render_query_text` generic — no
/// transform-specific branches.
fn context_path(context: &BTreeMap<String, String>) -> Option<&str> {
  for key in &["target_path", "target_paths", "target_module"] {
    if let Some(v) = context.get(*key) {
      return Some(v.as_str());
    }
  }
  None
}

/// Build a human-readable query string by looking up the
/// per-query-kind template in `QUERY_MESSAGE_TEMPLATES` and
/// substituting brace-style placeholders. The query string is
/// informational — host-symbol-resolver / ankh / operator-followup
/// lanes do NOT parse it; they use the structured fields. Only
/// external-knowledge-search uses the string as a search query.
///
/// Substituted placeholders:
///   - `{path}`      → context's target path (any of the
///                     transform-specific aliases) else `<unknown-path>`
///   - `{language}`  → context's language else `<unknown-language>`
///   - `{transform}` → the verdict's transform
fn render_query_text(
  query_kind: &str,
  context: &BTreeMap<String, String>,
  transform: &str,
) -> String {
  let path = context_path(context).unwrap_or("<unknown-path>");
  let language = context
    .get("language")
    .map(|s| s.as_str())
    .unwrap_or("<unknown-language>");
  let template = QUERY_MESSAGE_TEMPLATES
    .iter()
    .find(|t| t.query_kind == query_kind)
    .map(|t| t.template)
    .unwrap_or("Unrouted query kind `{query_kind}`");
  template
    .replace("{path}", path)
    .replace("{language}", language)
    .replace("{transform}", transform)
    .replace("{query_kind}", query_kind)
}

/// Project a Held / Rejected `ResolutionVerdict` into a typed
/// retrieval-query candidate. Returns `None` for `Ready` verdicts —
/// there is nothing to retrieve.
///
/// OWNER-LAW (2026-05-12): the **only** projection from synthesis
/// Held to retrieval input. Downstream retrieval lanes must consume
/// `HeldRetrievalQuery`, not raw Held verdicts.
pub fn build_query_from_held(verdict: &ResolutionVerdict) -> Option<HeldRetrievalQuery> {
  let (transform, held_kind, missing_slots, partial_resolution, reason) = match verdict {
    ResolutionVerdict::ResolutionReady { .. } => return None,
    ResolutionVerdict::ResolutionHeld {
      transform,
      held_kind,
      missing_slots,
      partial_resolution,
      reason,
    } => (
      transform.clone(),
      *held_kind,
      missing_slots.clone(),
      partial_resolution.clone(),
      reason.clone(),
    ),
    ResolutionVerdict::ResolutionRejected {
      transform,
      held_kind,
      reason,
    } => (
      transform.clone(),
      *held_kind,
      Vec::new(),
      BTreeMap::new(),
      reason.clone(),
    ),
  };

  let routing = routing_for(held_kind)?;
  let query_text = render_query_text(routing.query_kind, &partial_resolution, &transform);
  Some(HeldRetrievalQuery {
    query_kind: routing.query_kind.to_string(),
    held_kind,
    transform,
    primary_channel: routing.primary,
    fallback_channel: routing.fallback,
    query_text,
    evidence_to_recover: missing_slots,
    context_fields: partial_resolution,
    try_ankh_first: !matches!(routing.primary, HeldQueryRecoveryChannel::NotRecoverable),
    reason,
  })
}

/// Render a `HeldRetrievalQuery` as the canonical JSON payload of a
/// `coding.held-retrieval-query` artifact. Replay-stable id =
/// SHA-256 of intrinsic identity (held_kind + transform +
/// query_kind + primary channel + sorted evidence slots + sorted
/// context-field keys). `stored_at_ms` is extrinsic.
///
/// Content policy: every field is metadata. No source bodies, no
/// secrets — customer-release safe by default.
pub fn build_held_retrieval_query_artifact(
  query: &HeldRetrievalQuery,
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
) -> serde_json::Value {
  let mut h = Sha256::new();
  h.update(b"held-retrieval-query\x1f");
  h.update(query.held_kind.as_str().as_bytes());
  h.update(b"\x1f");
  h.update(query.transform.as_bytes());
  h.update(b"\x1f");
  h.update(query.query_kind.as_bytes());
  h.update(b"\x1f");
  h.update(query.primary_channel.as_str().as_bytes());
  h.update(b"\x1f");
  let mut sorted_evidence = query.evidence_to_recover.clone();
  sorted_evidence.sort();
  for slot in &sorted_evidence {
    h.update(slot.as_bytes());
    h.update(b"\x1e");
  }
  h.update(b"\x1f");
  // Sort context fields by key for replay stability.
  let mut keys: Vec<&String> = query.context_fields.keys().collect();
  keys.sort();
  for k in keys {
    h.update(k.as_bytes());
    h.update(b"\x1d");
    h.update(query.context_fields[k].as_bytes());
    h.update(b"\x1e");
  }
  let digest = h.finalize();
  let prefix = digest
    .iter()
    .take(16)
    .map(|b| format!("{b:02x}"))
    .collect::<String>();
  let id = format!("held-retrieval-query.{prefix}");

  let recoverable = !matches!(
    query.primary_channel,
    HeldQueryRecoveryChannel::NotRecoverable
  );

  let mut payload = serde_json::json!({
    "id": id,
    "artifact_family": "coding.held-retrieval-query",
    "source_surface": "algorithm-synthesis.held-to-query",
    "stored_at_ms": stored_at_ms,
    "held_kind": query.held_kind.as_str(),
    "transform": query.transform,
    "query_kind": query.query_kind,
    "primary_channel": query.primary_channel.as_str(),
    "fallback_channel": query.fallback_channel.map(|c| c.as_str()),
    "query_text": query.query_text,
    "evidence_to_recover": query.evidence_to_recover,
    "context_fields": query.context_fields,
    "try_ankh_first": query.try_ankh_first,
    "recoverable": recoverable,
    "reason": query.reason,
    "related_refs": serde_json::json!([
      format!("transform:{}", query.transform),
      format!("held-kind:{}", query.held_kind.as_str()),
      "owner-law:stdlib/lib/gate/algorithm-synthesis/held-to-query.px",
    ]),
    "target_paths": Vec::<String>::new(),
    "command_refs": Vec::<String>::new(),
  });
  if let Some(snap) = repo_snapshot_ref {
    payload["repo_snapshot_ref"] = serde_json::Value::String(snap.to_string());
  }
  payload
}

/// Outcome of a Held → Reopen transition. When a previously-Held
/// query is revisited (because new evidence arrived in a later
/// turn, or because the operator re-asked with more context), the
/// Reopen receipt records the *what changed*.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HeldReopenStatus {
  /// The new evidence filled the missing slot(s) — the previously
  /// Held verdict is now resolvable.
  Filled,
  /// New evidence arrived but the same Held kind persists (different
  /// slot still missing, or evidence didn't apply).
  StillHeld,
  /// The previous Held was *superseded* — operator deliberately
  /// chose a different transform / interpretation than the Held
  /// pointed to. Audit trail keeps the chain.
  Superseded,
}

impl HeldReopenStatus {
  pub fn as_str(self) -> &'static str {
    match self {
      Self::Filled => "filled",
      Self::StillHeld => "still-held",
      Self::Superseded => "superseded",
    }
  }
}

/// Receipt of a Held → Reopen event. Architecturally novel — the
/// first conversation-state primitive in the evolution lane.
/// Previous turns produce Held verdicts; this receipt links a new
/// turn's evidence back to the originating Held.
///
/// Carries:
///   - previous_query_id: SHA-prefix of the original held-retrieval-
///     query artifact (audit anchor)
///   - previous_held_kind / previous_transform: shape of the
///     originating Held
///   - new_supplied_parameters: what the operator added in this
///     turn that *would have* unblocked the original Held
///   - status: filled / still-held / superseded
///   - reopened_at_ms: extrinsic wall-clock
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeldReopenReceipt {
  pub previous_query_id: String,
  pub previous_held_kind: ResolutionHeldKind,
  pub previous_transform: String,
  pub previous_query_kind: String,
  pub status: HeldReopenStatus,
  pub new_supplied_parameters: BTreeMap<String, String>,
  pub reopened_at_ms: u64,
  pub reason: String,
}

/// Classify a Held → Reopen transition automatically. Given a
/// previously-emitted Held (its query + audit id) and the *new*
/// turn's resolution verdict + supplied parameters, decide whether
/// the new evidence:
///
///   - `Filled` the previous Held (same transform + new verdict is
///     Ready, AND at least one previously-missing slot has evidence
///     in the new turn's supplied parameters);
///   - `StillHeld` (same transform + same held_kind persists);
///   - `Superseded` (different transform → operator changed intent).
///
/// The substrate-level automation: cockpit doesn't need to ask
/// operator "did this resolve the previous Held?" — the
/// classification falls out of comparing the two verdicts.
///
/// Pure data; no I/O. `previous_query_id` is the audit anchor that
/// the caller computed by hashing the previous Held's artifact id.
pub fn classify_held_reopen(
  previous_query: &HeldRetrievalQuery,
  previous_query_id: String,
  new_resolution: &ResolutionVerdict,
  new_supplied_parameters: &BTreeMap<String, String>,
  reopened_at_ms: u64,
) -> HeldReopenReceipt {
  // 1. Determine new turn's transform.
  let new_transform = match new_resolution {
    ResolutionVerdict::ResolutionReady { transform, .. }
    | ResolutionVerdict::ResolutionHeld { transform, .. }
    | ResolutionVerdict::ResolutionRejected { transform, .. } => transform.clone(),
  };

  // 2. Different transform → Superseded.
  if new_transform != previous_query.transform {
    return HeldReopenReceipt {
      previous_query_id,
      previous_held_kind: previous_query.held_kind,
      previous_transform: previous_query.transform.clone(),
      previous_query_kind: previous_query.query_kind.clone(),
      status: HeldReopenStatus::Superseded,
      new_supplied_parameters: new_supplied_parameters.clone(),
      reopened_at_ms,
      reason: format!(
        "new turn switched transform `{}` → `{new_transform}` — operator changed intent",
        previous_query.transform
      ),
    };
  }

  // 3. Same transform — check verdict shape.
  match new_resolution {
    ResolutionVerdict::ResolutionReady { .. } => {
      // Resolution is Ready → previous Held got unblocked.
      // Sanity: at least one previously-missing slot should appear
      // in new_supplied_parameters (under either the slot name or
      // its `SLOT_TO_RESOLUTION_INPUT_FIELD` mapping). We don't
      // fail on absence — operator may have supplied evidence by
      // other means (NL extraction, host evidence) — but we surface
      // the matched slots in the receipt reason for audit.
      let mut matched: Vec<&String> = Vec::new();
      for slot in &previous_query.evidence_to_recover {
        if new_supplied_parameters.contains_key(slot) {
          matched.push(slot);
        }
      }
      let reason = if matched.is_empty() {
        format!(
          "transform `{}` now resolves Ready; new evidence supplied via NL extraction or host channel",
          previous_query.transform
        )
      } else {
        format!(
          "transform `{}` now resolves Ready; matched slot(s): {}",
          previous_query.transform,
          matched
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(", ")
        )
      };
      HeldReopenReceipt {
        previous_query_id,
        previous_held_kind: previous_query.held_kind,
        previous_transform: previous_query.transform.clone(),
        previous_query_kind: previous_query.query_kind.clone(),
        status: HeldReopenStatus::Filled,
        new_supplied_parameters: new_supplied_parameters.clone(),
        reopened_at_ms,
        reason,
      }
    }
    ResolutionVerdict::ResolutionHeld { held_kind, .. } => {
      // Same transform, still Held — operator's new evidence didn't
      // resolve the original missing slot (or resolved one and
      // moved on to a different missing slot, which is also
      // StillHeld semantically: we haven't moved past Held yet).
      let reason = if *held_kind == previous_query.held_kind {
        format!(
          "transform `{}` still Held on `{}` — new evidence didn't unblock",
          previous_query.transform,
          previous_query.held_kind.as_str()
        )
      } else {
        format!(
          "transform `{}` progressed `{}` → `{}` but still Held",
          previous_query.transform,
          previous_query.held_kind.as_str(),
          held_kind.as_str()
        )
      };
      HeldReopenReceipt {
        previous_query_id,
        previous_held_kind: previous_query.held_kind,
        previous_transform: previous_query.transform.clone(),
        previous_query_kind: previous_query.query_kind.clone(),
        status: HeldReopenStatus::StillHeld,
        new_supplied_parameters: new_supplied_parameters.clone(),
        reopened_at_ms,
        reason,
      }
    }
    ResolutionVerdict::ResolutionRejected { held_kind, .. } => {
      // Rejected on the same transform — operator's input is
      // structurally incompatible. Treat as StillHeld for the
      // purposes of the Reopen chain (the original Held is not
      // resolved; the new verdict explicitly closes the door
      // on this transform).
      HeldReopenReceipt {
        previous_query_id,
        previous_held_kind: previous_query.held_kind,
        previous_transform: previous_query.transform.clone(),
        previous_query_kind: previous_query.query_kind.clone(),
        status: HeldReopenStatus::StillHeld,
        new_supplied_parameters: new_supplied_parameters.clone(),
        reopened_at_ms,
        reason: format!(
          "transform `{}` Rejected on new evidence ({}); previous Held not resolved",
          previous_query.transform,
          held_kind.as_str()
        ),
      }
    }
  }
}

/// Snapshot of a `MultiTurnSession` at a point in time. Replay-
/// stable and round-trippable through serde — supports process
/// restart resume and external session audit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionStateSnapshot {
  pub snapshot_at_ms: u64,
  pub pending_held: Option<SessionPendingHeld>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionPendingHeld {
  pub query: HeldRetrievalQuery,
  pub query_id: String,
}

/// Render a `SessionStateSnapshot` as the canonical JSON payload
/// of a `coding.multi-turn-session-state` artifact. Per-
/// session conversation-state receipt — operator gets "what is
/// the current conversation trying to resolve?" + the full audit
/// chain anchor.
///
/// Replay-stable id = SHA-256 of intrinsic identity
/// (has_pending_held + pending_held.query_id + pending_held query
/// fingerprint). `snapshot_at_ms` is extrinsic.
///
/// Customer-release safe — metadata + audit ids only.
pub fn build_session_state_artifact(
  snapshot: &SessionStateSnapshot,
  repo_snapshot_ref: Option<&str>,
) -> serde_json::Value {
  let mut h = Sha256::new();
  h.update(b"multi-turn-session-state\x1f");
  if let Some(p) = &snapshot.pending_held {
    h.update(b"pending\x1f");
    h.update(p.query_id.as_bytes());
    h.update(b"\x1f");
    h.update(p.query.transform.as_bytes());
    h.update(b"\x1e");
    h.update(p.query.held_kind.as_str().as_bytes());
    h.update(b"\x1e");
    h.update(p.query.query_kind.as_bytes());
  } else {
    h.update(b"empty");
  }
  let digest = h.finalize();
  let prefix = digest
    .iter()
    .take(16)
    .map(|b| format!("{b:02x}"))
    .collect::<String>();
  let id = format!("multi-turn-session-state.{prefix}");

  let mut payload = serde_json::json!({
    "id": id,
    "artifact_family": "coding.multi-turn-session-state",
    "source_surface": "algorithm-synthesis.held-to-query",
    "snapshot_at_ms": snapshot.snapshot_at_ms,
    "has_pending_held": snapshot.pending_held.is_some(),
  });

  if let Some(p) = &snapshot.pending_held {
    payload["pending_held_query_id"] = serde_json::Value::String(p.query_id.clone());
    payload["pending_held_transform"] = serde_json::Value::String(p.query.transform.clone());
    payload["pending_held_kind"] =
      serde_json::Value::String(p.query.held_kind.as_str().to_string());
    payload["pending_held_query_kind"] = serde_json::Value::String(p.query.query_kind.clone());
    // Full query payload for restore.
    payload["pending_held_query"] = serde_json::to_value(&p.query).unwrap_or_default();
  }

  payload["related_refs"] =
    serde_json::json!(["owner-law:stdlib/lib/gate/algorithm-synthesis/held-to-query.px",]);
  payload["target_paths"] = serde_json::json!(Vec::<String>::new());
  payload["command_refs"] = serde_json::json!(Vec::<String>::new());

  if let Some(snap) = repo_snapshot_ref {
    payload["repo_snapshot_ref"] = serde_json::Value::String(snap.to_string());
  }
  payload
}

/// Session-scoped state for multi-turn conversation tracking. Holds
/// the most recent Held verdict so the *next* turn's verdict can
/// be auto-classified against it via `classify_held_reopen`.
///
/// v0 tracks only the *most recent* Held (single in-flight Held).
/// Multi-Held conversations (operator juggles two intents in
/// parallel) are future work — for now, a new Held replaces any
/// existing tracked Held, and a Ready/Rejected clears it.
///
/// The substrate-level automation: caller hands every NL turn's
/// verdict to `register_turn`, receives an optional reopen receipt
/// when the turn relates to a prior Held. Caller does not maintain
/// audit ids or classification logic.
#[derive(Debug, Clone, Default)]
pub struct MultiTurnSession {
  /// (previous Held query, its artifact id). `None` when no Held
  /// is currently in flight.
  most_recent_held: Option<(HeldRetrievalQuery, String)>,
}

impl MultiTurnSession {
  pub fn new() -> Self {
    Self::default()
  }

  /// Whether the session currently has a tracked Held.
  pub fn has_pending_held(&self) -> bool {
    self.most_recent_held.is_some()
  }

  /// The transform of the currently-tracked Held, if any. Useful
  /// for cockpit "previous turn was…" surfaces.
  pub fn pending_held_transform(&self) -> Option<&str> {
    self
      .most_recent_held
      .as_ref()
      .map(|(q, _)| q.transform.as_str())
  }

  /// The pending Held's audit id (the SHA-prefix of the original
  /// `coding.held-retrieval-query` artifact). `None` when no
  /// Held is in flight. Operator uses this to walk back to the
  /// originating Held artifact.
  pub fn pending_held_query_id(&self) -> Option<&str> {
    self.most_recent_held.as_ref().map(|(_, id)| id.as_str())
  }

  /// Capture the current session state as a snapshot. Deterministic —
  /// same session → same snapshot (snapshot_at_ms is extrinsic).
  pub fn capture_snapshot(&self, snapshot_at_ms: u64) -> SessionStateSnapshot {
    SessionStateSnapshot {
      snapshot_at_ms,
      pending_held: self
        .most_recent_held
        .as_ref()
        .map(|(q, id)| SessionPendingHeld {
          query: q.clone(),
          query_id: id.clone(),
        }),
    }
  }

  /// Restore a session from a previously-captured snapshot. Used
  /// after process restart or when resuming a saved conversation.
  pub fn from_snapshot(snapshot: SessionStateSnapshot) -> Self {
    Self {
      most_recent_held: snapshot.pending_held.map(|p| (p.query, p.query_id)),
    }
  }

  /// Register a turn's resolution verdict. If there is a prior
  /// pending Held, emit a `HeldReopenReceipt` auto-classifying
  /// the new turn against it. Session state updates accordingly:
  ///
  ///   - New verdict is Held → it becomes the new pending Held
  ///     (previous one is *replaced*; reopen receipt records the
  ///     transition).
  ///   - New verdict is Ready/Rejected → pending Held is cleared.
  ///
  /// First call (no prior Held) returns `None` reopen receipt;
  /// state is updated if the first verdict is Held.
  pub fn register_turn(
    &mut self,
    verdict: &ResolutionVerdict,
    new_supplied_parameters: BTreeMap<String, String>,
    turn_at_ms: u64,
  ) -> Option<HeldReopenReceipt> {
    let reopen = if let Some((prev_query, prev_id)) = &self.most_recent_held {
      Some(classify_held_reopen(
        prev_query,
        prev_id.clone(),
        verdict,
        &new_supplied_parameters,
        turn_at_ms,
      ))
    } else {
      None
    };

    // Update state based on new verdict.
    match verdict {
      ResolutionVerdict::ResolutionHeld { .. } => {
        // New Held becomes the new pending state (regardless of
        // what the prior one was). Reopen receipt — if emitted —
        // already captured the transition.
        if let Some(new_query) = build_query_from_held(verdict) {
          let art = build_held_retrieval_query_artifact(&new_query, turn_at_ms, None);
          let new_id = art["id"].as_str().unwrap_or("").to_string();
          self.most_recent_held = Some((new_query, new_id));
        }
      }
      ResolutionVerdict::ResolutionReady { .. } | ResolutionVerdict::ResolutionRejected { .. } => {
        // Pending Held is resolved (or superseded) — clear it.
        self.most_recent_held = None;
      }
    }

    reopen
  }
}

/// Render a `HeldReopenReceipt` as the canonical JSON payload of a
/// `coding.held-reopen-receipt` artifact. Architecturally
/// novel surface — first multi-turn conversation primitive.
///
/// Replay-stable id = SHA-256 of intrinsic identity
/// (previous_query_id + status + transform + held_kind + sorted
/// supplied_parameters). `reopened_at_ms` is extrinsic.
///
/// Content policy: metadata + supplied parameter pairs only. No
/// source bodies. Customer-release safe.
pub fn build_held_reopen_receipt_artifact(
  receipt: &HeldReopenReceipt,
  repo_snapshot_ref: Option<&str>,
) -> serde_json::Value {
  let mut h = Sha256::new();
  h.update(b"held-reopen-receipt\x1f");
  h.update(receipt.previous_query_id.as_bytes());
  h.update(b"\x1f");
  h.update(receipt.status.as_str().as_bytes());
  h.update(b"\x1f");
  h.update(receipt.previous_transform.as_bytes());
  h.update(b"\x1e");
  h.update(receipt.previous_held_kind.as_str().as_bytes());
  h.update(b"\x1e");
  h.update(receipt.previous_query_kind.as_bytes());
  h.update(b"\x1f");
  let mut keys: Vec<&String> = receipt.new_supplied_parameters.keys().collect();
  keys.sort();
  for k in keys {
    h.update(k.as_bytes());
    h.update(b"\x1d");
    h.update(receipt.new_supplied_parameters[k].as_bytes());
    h.update(b"\x1e");
  }
  let digest = h.finalize();
  let prefix = digest
    .iter()
    .take(16)
    .map(|b| format!("{b:02x}"))
    .collect::<String>();
  let id = format!("held-reopen-receipt.{prefix}");

  let mut payload = serde_json::json!({
    "id": id,
    "artifact_family": "coding.held-reopen-receipt",
    "source_surface": "algorithm-synthesis.held-to-query",
    "reopened_at_ms": receipt.reopened_at_ms,
    "status": receipt.status.as_str(),
    "previous_query_id": receipt.previous_query_id,
    "previous_held_kind": receipt.previous_held_kind.as_str(),
    "previous_transform": receipt.previous_transform,
    "previous_query_kind": receipt.previous_query_kind,
    "new_supplied_parameters": receipt.new_supplied_parameters,
    "reason": receipt.reason,
    "related_refs": serde_json::json!([
      format!("previous-query-id:{}", receipt.previous_query_id),
      format!("previous-held-kind:{}", receipt.previous_held_kind.as_str()),
      format!("previous-transform:{}", receipt.previous_transform),
      format!("reopen-status:{}", receipt.status.as_str()),
      "owner-law:stdlib/lib/gate/algorithm-synthesis/held-to-query.px",
    ]),
    "target_paths": Vec::<String>::new(),
    "command_refs": Vec::<String>::new(),
  });
  if let Some(snap) = repo_snapshot_ref {
    payload["repo_snapshot_ref"] = serde_json::Value::String(snap.to_string());
  }
  payload
}

#[cfg(test)]
mod tests {
  use super::*;

  fn held(
    transform: &str,
    kind: ResolutionHeldKind,
    missing: &[&str],
    partial: &[(&str, &str)],
  ) -> ResolutionVerdict {
    ResolutionVerdict::ResolutionHeld {
      transform: transform.to_string(),
      held_kind: kind,
      missing_slots: missing.iter().map(|s| s.to_string()).collect(),
      partial_resolution: partial
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect(),
      reason: "test reason".to_string(),
    }
  }

  // ─── routing coverage ──────────────────────────────────────────

  #[test]
  fn every_resolution_held_kind_has_a_routing_entry() {
    for kind in ResolutionHeldKind::ALL {
      let found = HELD_ROUTING.iter().any(|e| &e.held == kind);
      assert!(
        found,
        "ResolutionHeldKind `{}` has no entry in HELD_ROUTING — routing universe drift",
        kind.as_str()
      );
    }
  }

  #[test]
  fn every_query_kind_has_a_message_template() {
    for entry in HELD_ROUTING {
      let found = QUERY_MESSAGE_TEMPLATES
        .iter()
        .any(|t| t.query_kind == entry.query_kind);
      assert!(
        found,
        "query_kind `{}` has no message template — template universe drift",
        entry.query_kind
      );
    }
  }

  #[test]
  fn every_message_template_targets_a_known_query_kind() {
    for tmpl in QUERY_MESSAGE_TEMPLATES {
      let found = HELD_ROUTING.iter().any(|e| e.query_kind == tmpl.query_kind);
      assert!(
        found,
        "message template for `{}` has no routing row — dead template",
        tmpl.query_kind
      );
    }
  }

  #[test]
  fn no_duplicate_routing_entries_per_held_kind() {
    let mut seen = std::collections::HashSet::new();
    for e in HELD_ROUTING {
      assert!(
        seen.insert(e.held.as_str()),
        "duplicate routing entry for `{}`",
        e.held.as_str()
      );
    }
  }

  // ─── projection: Ready → None ──────────────────────────────────

  #[test]
  fn ready_verdict_returns_none() {
    let v = ResolutionVerdict::ResolutionReady {
      transform: "rename-symbol".to_string(),
      request: serde_json::json!({}),
      resolved_fields: BTreeMap::new(),
    };
    assert!(build_query_from_held(&v).is_none());
  }

  // ─── projection: typical Held cases ────────────────────────────

  #[test]
  fn missing_import_spec_routes_to_host_symbol_resolver() {
    let v = held(
      "add-import",
      ResolutionHeldKind::MissingImportSpec,
      &["import_spec"],
      &[("target_path", "src/util.py"), ("language", "python")],
    );
    let q = build_query_from_held(&v).expect("query");
    assert_eq!(q.query_kind, "lookup-module-providing-symbol");
    assert_eq!(
      q.primary_channel,
      HeldQueryRecoveryChannel::HostSymbolResolver
    );
    assert_eq!(
      q.fallback_channel,
      Some(HeldQueryRecoveryChannel::ExternalKnowledgeSearch)
    );
    assert!(q.evidence_to_recover.contains(&"import_spec".to_string()));
    assert_eq!(q.context_fields.get("target_path").unwrap(), "src/util.py");
    assert!(q.try_ankh_first);
    assert!(q.query_text.contains("python"));
    assert!(q.query_text.contains("src/util.py"));
  }

  #[test]
  fn missing_candidate_imports_routes_to_host_symbol_resolver() {
    let v = held(
      "remove-unused-import",
      ResolutionHeldKind::MissingCandidateImports,
      &["candidate_imports"],
      &[("target_path", "src/a.py"), ("language", "python")],
    );
    let q = build_query_from_held(&v).expect("query");
    assert_eq!(
      q.primary_channel,
      HeldQueryRecoveryChannel::HostSymbolResolver
    );
    assert_eq!(q.query_kind, "host-lints-unused-imports");
  }

  #[test]
  fn missing_old_name_routes_to_operator_followup() {
    let v = held(
      "rename-symbol",
      ResolutionHeldKind::MissingOldName,
      &["old_name", "new_name"],
      &[("target_paths", "src/a.rs"), ("language", "rust")],
    );
    let q = build_query_from_held(&v).expect("query");
    assert_eq!(
      q.primary_channel,
      HeldQueryRecoveryChannel::OperatorFollowup
    );
    assert!(q.fallback_channel.is_none());
    assert_eq!(q.query_kind, "operator-asks-old-symbol");
    assert!(q.try_ankh_first);
  }

  #[test]
  fn missing_test_name_falls_back_to_external_search() {
    let v = held(
      "add-test-stub",
      ResolutionHeldKind::MissingTestName,
      &["test_name"],
      &[("target_module", "tests/foo.rs"), ("language", "rust")],
    );
    let q = build_query_from_held(&v).expect("query");
    assert_eq!(
      q.primary_channel,
      HeldQueryRecoveryChannel::OperatorFollowup
    );
    assert_eq!(
      q.fallback_channel,
      Some(HeldQueryRecoveryChannel::ExternalKnowledgeSearch)
    );
  }

  // ─── projection: Rejected → not-recoverable ────────────────────

  #[test]
  fn rejected_invalid_identifier_is_not_recoverable() {
    let v = ResolutionVerdict::ResolutionRejected {
      transform: "rename-symbol".to_string(),
      held_kind: ResolutionHeldKind::InvalidIdentifier,
      reason: "non-ident".to_string(),
    };
    let q = build_query_from_held(&v).expect("query");
    assert_eq!(q.primary_channel, HeldQueryRecoveryChannel::NotRecoverable);
    assert!(
      !q.try_ankh_first,
      "not-recoverable disables ankh-first hint"
    );
    assert!(q.evidence_to_recover.is_empty());
  }

  #[test]
  fn rejected_old_equals_new_is_not_recoverable() {
    let v = ResolutionVerdict::ResolutionRejected {
      transform: "rename-symbol".to_string(),
      held_kind: ResolutionHeldKind::OldEqualsNew,
      reason: "identical".to_string(),
    };
    let q = build_query_from_held(&v).expect("query");
    assert_eq!(q.primary_channel, HeldQueryRecoveryChannel::NotRecoverable);
  }

  // ─── transform-not-supported-by-resolver routes to extension ───

  #[test]
  fn unsupported_transform_routes_to_extend_resolver() {
    let v = held(
      "extract-function",
      ResolutionHeldKind::TransformNotSupportedByResolver,
      &[],
      &[],
    );
    let q = build_query_from_held(&v).expect("query");
    assert_eq!(q.primary_channel, HeldQueryRecoveryChannel::NotRecoverable);
    assert_eq!(q.query_kind, "extend-resolver-implementation");
    assert!(q.query_text.contains("extract-function"));
  }

  // ─── round-trip with real parameter-resolution ─────────────────

  #[test]
  fn integrates_with_parameter_resolution_held() {
    use super::super::parameter_resolution::{resolve_parameters, ResolutionInput};
    // Abstract NL → MissingTargetPath Held.
    let v = resolve_parameters(&ResolutionInput {
      operation_candidate: "rename-symbol".to_string(),
      utterance: "이 함수 이름 바꿔줘".to_string(),
      ..Default::default()
    });
    let q = build_query_from_held(&v).expect("Held produces a query");
    assert_eq!(q.transform, "rename-symbol");
    assert_eq!(q.held_kind, ResolutionHeldKind::MissingTargetPath);
    assert_eq!(
      q.primary_channel,
      HeldQueryRecoveryChannel::OperatorFollowup
    );
    assert_eq!(
      q.fallback_channel,
      Some(HeldQueryRecoveryChannel::HostSymbolResolver)
    );
    assert!(q.evidence_to_recover.contains(&"target_path".to_string()));
  }

  // ─── artifact builder ─────────────────────────────────────────

  fn add_import_held_query() -> HeldRetrievalQuery {
    use super::super::parameter_resolution::{resolve_parameters, ResolutionInput};
    let v = resolve_parameters(&ResolutionInput {
      operation_candidate: "add-import".to_string(),
      utterance: "src/util.py 에 import 추가".to_string(),
      ..Default::default()
    });
    build_query_from_held(&v).expect("query")
  }

  #[test]
  fn artifact_id_is_replay_stable() {
    let q = add_import_held_query();
    let a1 = build_held_retrieval_query_artifact(&q, 1000, None);
    let a2 = build_held_retrieval_query_artifact(&q, 9999999, None);
    assert_eq!(a1["id"], a2["id"]);
    assert!(a1["id"]
      .as_str()
      .unwrap()
      .starts_with("held-retrieval-query."));
  }

  #[test]
  fn artifact_payload_surfaces_recovery_routing() {
    let q = add_import_held_query();
    let a = build_held_retrieval_query_artifact(&q, 1700000000000, None);
    assert_eq!(a["artifact_family"], "coding.held-retrieval-query");
    assert_eq!(a["held_kind"], "missing-import-spec");
    assert_eq!(a["transform"], "add-import");
    assert_eq!(a["query_kind"], "lookup-module-providing-symbol");
    assert_eq!(a["primary_channel"], "host-symbol-resolver");
    assert_eq!(a["fallback_channel"], "external-knowledge-search");
    assert_eq!(a["recoverable"], true);
    assert_eq!(a["try_ankh_first"], true);
    assert!(a["evidence_to_recover"]
      .as_array()
      .unwrap()
      .iter()
      .any(|v| v == "import_spec"));
  }

  #[test]
  fn artifact_marks_not_recoverable_for_rejected() {
    let v = ResolutionVerdict::ResolutionRejected {
      transform: "rename-symbol".to_string(),
      held_kind: ResolutionHeldKind::InvalidIdentifier,
      reason: "non-ident".to_string(),
    };
    let q = build_query_from_held(&v).expect("query");
    let a = build_held_retrieval_query_artifact(&q, 0, None);
    assert_eq!(a["recoverable"], false);
    assert_eq!(a["try_ankh_first"], false);
    assert_eq!(a["primary_channel"], "not-recoverable");
  }

  #[test]
  fn artifact_includes_audit_back_refs() {
    let q = add_import_held_query();
    let a = build_held_retrieval_query_artifact(&q, 0, None);
    let refs = a["related_refs"].as_array().unwrap();
    assert!(refs
      .iter()
      .any(|v| v.as_str().unwrap().contains("transform:add-import")));
    assert!(refs.iter().any(|v| v
      .as_str()
      .unwrap()
      .contains("held-kind:missing-import-spec")));
    assert!(refs
      .iter()
      .any(|v| v.as_str().unwrap().contains("held-to-query.px")));
  }

  // ─── held-reopen-receipt (multi-turn primitive) ───────────────

  fn sample_reopen_receipt(status: HeldReopenStatus) -> HeldReopenReceipt {
    let mut sp = BTreeMap::new();
    sp.insert("equivalent_form".to_string(), "(x+y)^2".to_string());
    HeldReopenReceipt {
      previous_query_id: "held-retrieval-query.aabbccdd00112233".to_string(),
      previous_held_kind: ResolutionHeldKind::MissingAlgebraicEquivalent,
      previous_transform: "lookup-algebraic-equivalent".to_string(),
      previous_query_kind: "lookup-algebraic-equivalent".to_string(),
      status,
      new_supplied_parameters: sp,
      reopened_at_ms: 1700000005000,
      reason: "operator supplied equivalent_form in follow-up turn".to_string(),
    }
  }

  #[test]
  fn reopen_receipt_filled_carries_status_and_back_ref() {
    let r = sample_reopen_receipt(HeldReopenStatus::Filled);
    let art = build_held_reopen_receipt_artifact(&r, None);
    assert_eq!(art["artifact_family"], "coding.held-reopen-receipt");
    assert_eq!(art["status"], "filled");
    assert_eq!(
      art["previous_query_id"],
      "held-retrieval-query.aabbccdd00112233"
    );
    assert_eq!(art["previous_held_kind"], "missing-algebraic-equivalent");
    assert_eq!(art["previous_transform"], "lookup-algebraic-equivalent");
    let sp = art["new_supplied_parameters"].as_object().unwrap();
    assert_eq!(sp.get("equivalent_form").unwrap(), "(x+y)^2");
  }

  #[test]
  fn reopen_receipt_still_held_distinct_status() {
    let r = sample_reopen_receipt(HeldReopenStatus::StillHeld);
    let art = build_held_reopen_receipt_artifact(&r, None);
    assert_eq!(art["status"], "still-held");
  }

  #[test]
  fn reopen_receipt_superseded_distinct_status() {
    let r = sample_reopen_receipt(HeldReopenStatus::Superseded);
    let art = build_held_reopen_receipt_artifact(&r, None);
    assert_eq!(art["status"], "superseded");
  }

  #[test]
  fn reopen_receipt_id_is_replay_stable_across_reopened_at() {
    let mut r1 = sample_reopen_receipt(HeldReopenStatus::Filled);
    let mut r2 = sample_reopen_receipt(HeldReopenStatus::Filled);
    r1.reopened_at_ms = 1;
    r2.reopened_at_ms = 999999;
    let a1 = build_held_reopen_receipt_artifact(&r1, None);
    let a2 = build_held_reopen_receipt_artifact(&r2, None);
    assert_eq!(a1["id"], a2["id"], "id must ignore reopened_at_ms");
  }

  #[test]
  fn reopen_receipt_id_differs_when_status_differs() {
    let r_filled = sample_reopen_receipt(HeldReopenStatus::Filled);
    let r_super = sample_reopen_receipt(HeldReopenStatus::Superseded);
    let a_f = build_held_reopen_receipt_artifact(&r_filled, None);
    let a_s = build_held_reopen_receipt_artifact(&r_super, None);
    assert_ne!(a_f["id"], a_s["id"]);
  }

  // ─── classify_held_reopen (auto-classifier) ───────────────────

  fn sample_math_held_query() -> HeldRetrievalQuery {
    let v = super::super::parameter_resolution::resolve_parameters(
      &super::super::parameter_resolution::ResolutionInput {
        operation_candidate: "lookup-algebraic-equivalent".to_string(),
        utterance: "x^2 + 2*x*y + y^2 는 뭐야?".to_string(),
        ..Default::default()
      },
    );
    build_query_from_held(&v).expect("math query")
  }

  #[test]
  fn classify_filled_when_new_resolution_is_ready_with_matching_slot() {
    let prev = sample_math_held_query();
    // Simulate the next turn: Ready resolution from a synthetic
    // ResolutionVerdict (operator supplied the equivalent).
    let new = super::super::parameter_resolution::ResolutionVerdict::ResolutionReady {
      transform: "lookup-algebraic-equivalent".to_string(),
      request: serde_json::json!({}),
      resolved_fields: BTreeMap::new(),
    };
    let mut sp = BTreeMap::new();
    sp.insert("equivalent_form".to_string(), "(x+y)^2".to_string());
    let r = classify_held_reopen(
      &prev,
      "held-retrieval-query.aabb".to_string(),
      &new,
      &sp,
      1700000005000,
    );
    assert_eq!(r.status, HeldReopenStatus::Filled);
    assert!(r.reason.contains("equivalent_form"));
    assert_eq!(
      r.new_supplied_parameters.get("equivalent_form").unwrap(),
      "(x+y)^2"
    );
  }

  #[test]
  fn classify_still_held_when_same_transform_still_held() {
    let prev = sample_math_held_query();
    // Same transform, still Held on same held_kind.
    let new = super::super::parameter_resolution::ResolutionVerdict::ResolutionHeld {
      transform: "lookup-algebraic-equivalent".to_string(),
      held_kind: ResolutionHeldKind::MissingAlgebraicEquivalent,
      missing_slots: vec!["equivalent_form".to_string()],
      partial_resolution: BTreeMap::new(),
      reason: "still missing".to_string(),
    };
    let r = classify_held_reopen(
      &prev,
      "held-retrieval-query.aabb".to_string(),
      &new,
      &BTreeMap::new(),
      0,
    );
    assert_eq!(r.status, HeldReopenStatus::StillHeld);
    assert!(r.reason.contains("still Held"));
  }

  #[test]
  fn classify_superseded_when_new_transform_differs() {
    let prev = sample_math_held_query();
    // Operator switched intent to a different transform.
    let new = super::super::parameter_resolution::ResolutionVerdict::ResolutionHeld {
      transform: "rename-symbol".to_string(),
      held_kind: ResolutionHeldKind::MissingTargetPath,
      missing_slots: vec!["target_path".to_string()],
      partial_resolution: BTreeMap::new(),
      reason: "different intent".to_string(),
    };
    let r = classify_held_reopen(
      &prev,
      "held-retrieval-query.aabb".to_string(),
      &new,
      &BTreeMap::new(),
      0,
    );
    assert_eq!(r.status, HeldReopenStatus::Superseded);
    assert!(r.reason.contains("switched transform"));
  }

  #[test]
  fn classify_still_held_when_same_transform_progressed_but_not_done() {
    // Same transform, but Held kind progressed to a different
    // missing slot — still StillHeld, but reason notes progression.
    let prev = sample_math_held_query();
    let new = super::super::parameter_resolution::ResolutionVerdict::ResolutionHeld {
      transform: "lookup-algebraic-equivalent".to_string(),
      held_kind: ResolutionHeldKind::InvalidIdentifier,
      missing_slots: vec![],
      partial_resolution: BTreeMap::new(),
      reason: "invalid".to_string(),
    };
    let r = classify_held_reopen(
      &prev,
      "held-retrieval-query.aabb".to_string(),
      &new,
      &BTreeMap::new(),
      0,
    );
    assert_eq!(r.status, HeldReopenStatus::StillHeld);
    assert!(r.reason.contains("progressed"));
  }

  #[test]
  fn classify_still_held_when_rejected_on_same_transform() {
    let prev = sample_math_held_query();
    let new = super::super::parameter_resolution::ResolutionVerdict::ResolutionRejected {
      transform: "lookup-algebraic-equivalent".to_string(),
      held_kind: ResolutionHeldKind::InvalidIdentifier,
      reason: "not a math expression".to_string(),
    };
    let r = classify_held_reopen(
      &prev,
      "held-retrieval-query.aabb".to_string(),
      &new,
      &BTreeMap::new(),
      0,
    );
    assert_eq!(r.status, HeldReopenStatus::StillHeld);
    assert!(r.reason.contains("Rejected"));
  }

  #[test]
  fn classify_filled_without_explicit_slot_match_still_flags_audit() {
    // Caller supplied empty new_supplied_parameters but the new
    // verdict is Ready — operator might have supplied evidence via
    // NL extraction. Status is Filled with audit note.
    let prev = sample_math_held_query();
    let new = super::super::parameter_resolution::ResolutionVerdict::ResolutionReady {
      transform: "lookup-algebraic-equivalent".to_string(),
      request: serde_json::json!({}),
      resolved_fields: BTreeMap::new(),
    };
    let r = classify_held_reopen(
      &prev,
      "held-retrieval-query.aabb".to_string(),
      &new,
      &BTreeMap::new(),
      0,
    );
    assert_eq!(r.status, HeldReopenStatus::Filled);
    assert!(r.reason.contains("NL extraction or host channel"));
  }

  // ─── MultiTurnSession (auto-tracking) ─────────────────────────

  fn math_held_input() -> super::super::parameter_resolution::ResolutionInput {
    super::super::parameter_resolution::ResolutionInput {
      operation_candidate: "lookup-algebraic-equivalent".to_string(),
      utterance: "x^2 + 2*x*y + y^2 는 뭐야?".to_string(),
      ..Default::default()
    }
  }

  fn synthetic_ready(transform: &str) -> super::super::parameter_resolution::ResolutionVerdict {
    super::super::parameter_resolution::ResolutionVerdict::ResolutionReady {
      transform: transform.to_string(),
      request: serde_json::json!({}),
      resolved_fields: BTreeMap::new(),
    }
  }

  fn synthetic_held(
    transform: &str,
    kind: ResolutionHeldKind,
  ) -> super::super::parameter_resolution::ResolutionVerdict {
    super::super::parameter_resolution::ResolutionVerdict::ResolutionHeld {
      transform: transform.to_string(),
      held_kind: kind,
      missing_slots: vec![],
      partial_resolution: BTreeMap::new(),
      reason: "synthetic".to_string(),
    }
  }

  #[test]
  fn session_first_held_no_reopen_state_updates() {
    let mut session = MultiTurnSession::new();
    let v = super::super::parameter_resolution::resolve_parameters(&math_held_input());
    let reopen = session.register_turn(&v, BTreeMap::new(), 1700000000000);
    assert!(reopen.is_none(), "first turn has no prior Held");
    assert!(session.has_pending_held());
    assert_eq!(
      session.pending_held_transform(),
      Some("lookup-algebraic-equivalent")
    );
  }

  #[test]
  fn session_filled_path_clears_pending_held() {
    let mut session = MultiTurnSession::new();
    let v1 = super::super::parameter_resolution::resolve_parameters(&math_held_input());
    let _ = session.register_turn(&v1, BTreeMap::new(), 1);
    assert!(session.has_pending_held());

    // Turn 2: same transform Ready → Filled, pending cleared.
    let mut sp = BTreeMap::new();
    sp.insert("equivalent_form".to_string(), "(x+y)^2".to_string());
    let v2 = synthetic_ready("lookup-algebraic-equivalent");
    let reopen = session
      .register_turn(&v2, sp, 2)
      .expect("reopen emitted because prior Held existed");
    assert_eq!(reopen.status, HeldReopenStatus::Filled);
    assert!(!session.has_pending_held(), "Filled clears pending Held");
  }

  #[test]
  fn session_superseded_path_replaces_pending_held() {
    let mut session = MultiTurnSession::new();
    let v1 = super::super::parameter_resolution::resolve_parameters(&math_held_input());
    let _ = session.register_turn(&v1, BTreeMap::new(), 1);

    // Turn 2: different transform Held → Superseded, NEW Held
    // becomes pending.
    let v2 = synthetic_held("rename-symbol", ResolutionHeldKind::MissingTargetPath);
    let reopen = session
      .register_turn(&v2, BTreeMap::new(), 2)
      .expect("reopen emitted");
    assert_eq!(reopen.status, HeldReopenStatus::Superseded);
    assert!(session.has_pending_held(), "new Held replaces previous");
    assert_eq!(session.pending_held_transform(), Some("rename-symbol"));
  }

  #[test]
  fn session_still_held_replaces_pending_with_new_held() {
    let mut session = MultiTurnSession::new();
    let v1 = super::super::parameter_resolution::resolve_parameters(&math_held_input());
    let _ = session.register_turn(&v1, BTreeMap::new(), 1);

    // Turn 2: same transform Held again on same held_kind → StillHeld.
    let v2 = synthetic_held(
      "lookup-algebraic-equivalent",
      ResolutionHeldKind::MissingAlgebraicEquivalent,
    );
    let reopen = session
      .register_turn(&v2, BTreeMap::new(), 2)
      .expect("reopen emitted");
    assert_eq!(reopen.status, HeldReopenStatus::StillHeld);
    // Pending Held is replaced (with the new identical-shape Held —
    // most recent semantics).
    assert!(session.has_pending_held());
  }

  #[test]
  fn session_ready_without_prior_held_no_reopen() {
    let mut session = MultiTurnSession::new();
    let v = synthetic_ready("rename-symbol");
    let reopen = session.register_turn(&v, BTreeMap::new(), 1);
    assert!(
      reopen.is_none(),
      "no reopen emitted when no prior Held existed"
    );
    assert!(!session.has_pending_held());
  }

  #[test]
  fn session_handles_three_turn_sequence_with_audit_chain() {
    let mut session = MultiTurnSession::new();
    // Turn 1: math Held.
    let v1 = super::super::parameter_resolution::resolve_parameters(&math_held_input());
    let r1 = session.register_turn(&v1, BTreeMap::new(), 1);
    assert!(r1.is_none());
    let turn1_transform = session.pending_held_transform().map(|s| s.to_string());

    // Turn 2: same transform, still Held (operator's follow-up
    // didn't help).
    let v2 = synthetic_held(
      "lookup-algebraic-equivalent",
      ResolutionHeldKind::MissingAlgebraicEquivalent,
    );
    let r2 = session.register_turn(&v2, BTreeMap::new(), 2).expect("r2");
    assert_eq!(r2.status, HeldReopenStatus::StillHeld);

    // Turn 3: operator finally supplied evidence → Ready, Filled.
    let mut sp = BTreeMap::new();
    sp.insert("equivalent_form".to_string(), "(x+y)^2".to_string());
    let v3 = synthetic_ready("lookup-algebraic-equivalent");
    let r3 = session.register_turn(&v3, sp, 3).expect("r3");
    assert_eq!(r3.status, HeldReopenStatus::Filled);
    assert!(!session.has_pending_held());

    // Audit: all three reopen receipts reference the previous turn's
    // transform consistently — the chain is walkable.
    assert_eq!(
      turn1_transform.as_deref(),
      Some("lookup-algebraic-equivalent")
    );
    assert_eq!(r2.previous_transform, "lookup-algebraic-equivalent");
    assert_eq!(r3.previous_transform, "lookup-algebraic-equivalent");
  }

  // ─── session-state snapshot artifact + restore ────────────────

  #[test]
  fn session_snapshot_empty_session_carries_no_pending() {
    let session = MultiTurnSession::new();
    let snap = session.capture_snapshot(1700000000000);
    assert!(snap.pending_held.is_none());
    let art = build_session_state_artifact(&snap, None);
    assert_eq!(art["artifact_family"], "coding.multi-turn-session-state");
    assert_eq!(art["has_pending_held"], false);
    assert!(art.get("pending_held_transform").is_none());
  }

  #[test]
  fn session_snapshot_populated_session_carries_query_id_and_metadata() {
    let mut session = MultiTurnSession::new();
    let v = super::super::parameter_resolution::resolve_parameters(&math_held_input());
    let _ = session.register_turn(&v, BTreeMap::new(), 1);
    let snap = session.capture_snapshot(2);
    let art = build_session_state_artifact(&snap, None);
    assert_eq!(art["has_pending_held"], true);
    assert_eq!(art["pending_held_transform"], "lookup-algebraic-equivalent");
    assert_eq!(art["pending_held_kind"], "missing-algebraic-equivalent");
    assert!(art["pending_held_query_id"]
      .as_str()
      .unwrap()
      .starts_with("held-retrieval-query."));
    // Full query payload present for restore.
    assert!(art["pending_held_query"].is_object());
  }

  #[test]
  fn session_snapshot_round_trips_through_from_snapshot() {
    let mut session = MultiTurnSession::new();
    let v = super::super::parameter_resolution::resolve_parameters(&math_held_input());
    let _ = session.register_turn(&v, BTreeMap::new(), 1);
    let snap = session.capture_snapshot(2);

    // Restore into a fresh session.
    let restored = MultiTurnSession::from_snapshot(snap.clone());
    assert!(restored.has_pending_held());
    assert_eq!(
      restored.pending_held_transform(),
      Some("lookup-algebraic-equivalent")
    );
    // The audit id survives restore.
    assert_eq!(
      restored.pending_held_query_id(),
      session.pending_held_query_id()
    );
  }

  #[test]
  fn session_snapshot_restore_lets_next_turn_classify_correctly() {
    // End-to-end: snapshot → restore → register Filled turn →
    // reopen receipt links back to the original query id.
    let mut session = MultiTurnSession::new();
    let v1 = super::super::parameter_resolution::resolve_parameters(&math_held_input());
    let _ = session.register_turn(&v1, BTreeMap::new(), 1);
    let original_id = session.pending_held_query_id().unwrap().to_string();

    let snap = session.capture_snapshot(2);
    let mut restored = MultiTurnSession::from_snapshot(snap);

    // After restore, a Ready turn on same transform should produce
    // a Filled reopen referencing the *original* query id.
    let mut sp = BTreeMap::new();
    sp.insert("equivalent_form".to_string(), "(x+y)^2".to_string());
    let v2 = synthetic_ready("lookup-algebraic-equivalent");
    let reopen = restored
      .register_turn(&v2, sp, 3)
      .expect("reopen emitted after restore");
    assert_eq!(reopen.status, HeldReopenStatus::Filled);
    assert_eq!(reopen.previous_query_id, original_id);
  }

  #[test]
  fn session_snapshot_id_replay_stable_across_snapshot_at_ms() {
    let mut session = MultiTurnSession::new();
    let v = super::super::parameter_resolution::resolve_parameters(&math_held_input());
    let _ = session.register_turn(&v, BTreeMap::new(), 1);
    let s1 = session.capture_snapshot(1);
    let s2 = session.capture_snapshot(999999);
    let a1 = build_session_state_artifact(&s1, None);
    let a2 = build_session_state_artifact(&s2, None);
    assert_eq!(a1["id"], a2["id"], "id must ignore snapshot_at_ms");
  }

  #[test]
  fn session_snapshot_id_differs_between_empty_and_populated() {
    let empty = SessionStateSnapshot {
      snapshot_at_ms: 0,
      pending_held: None,
    };
    let mut session = MultiTurnSession::new();
    let v = super::super::parameter_resolution::resolve_parameters(&math_held_input());
    let _ = session.register_turn(&v, BTreeMap::new(), 1);
    let populated = session.capture_snapshot(0);
    let a_e = build_session_state_artifact(&empty, None);
    let a_p = build_session_state_artifact(&populated, None);
    assert_ne!(a_e["id"], a_p["id"]);
  }

  #[test]
  fn reopen_receipt_related_refs_walk_back_to_previous_query() {
    let r = sample_reopen_receipt(HeldReopenStatus::Filled);
    let art = build_held_reopen_receipt_artifact(&r, None);
    let refs: Vec<String> = serde_json::from_value(art["related_refs"].clone()).unwrap();
    assert!(refs
      .iter()
      .any(|r| r.starts_with("previous-query-id:held-retrieval-query.")));
    assert!(refs.iter().any(|r| r == "reopen-status:filled"));
  }
}
