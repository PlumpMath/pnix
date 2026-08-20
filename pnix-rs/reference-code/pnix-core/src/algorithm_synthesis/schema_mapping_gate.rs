//! Schema-mapping gate — between Gate 1 and Gate 2 of the evolution
//! lane.
//!
//! OWNER-LAW (2026-05-12): mirror of
//! `stdlib/lib/gate/algorithm-synthesis/schema-mapping-gate.px`.
//! Converts observer-shaped proposals into schema-aware proposals
//! that can pass `axis-separation-gate` organically — closes the
//! Gate 3 hold that v0 candidates would otherwise always trigger.

use serde::{Deserialize, Serialize};
use pnix_hash::{Digest, Sha256};
use std::collections::BTreeMap;

use super::candidate_row_proposal::{CandidateKind, CandidateRowProposal};
use super::held_to_query::HeldQueryRecoveryChannel;
use super::parameter_resolution::ResolutionHeldKind;

/// Outcomes. Stays byte-identical to `.px` `validMappingOutcomes`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SchemaMappingOutcome {
  Mapped,
  HeldUnsupportedKindTablePair,
  HeldSourceFieldMissing,
  HeldUnknownQueryKind,
}

impl SchemaMappingOutcome {
  pub const ALL: &'static [Self] = &[
    Self::Mapped,
    Self::HeldUnsupportedKindTablePair,
    Self::HeldSourceFieldMissing,
    Self::HeldUnknownQueryKind,
  ];

  pub fn as_str(self) -> &'static str {
    match self {
      Self::Mapped => "mapped",
      Self::HeldUnsupportedKindTablePair => "held-unsupported-kind-table-pair",
      Self::HeldSourceFieldMissing => "held-source-field-missing",
      Self::HeldUnknownQueryKind => "held-unknown-query-kind",
    }
  }
}

/// Registered-mapping row. Stays byte-identical to `.px`
/// `registeredMappings`.
#[derive(Debug, Clone, Copy)]
pub struct RegisteredMapping {
  pub candidate_kind: CandidateKind,
  pub target_table: &'static str,
  pub mapper_id: &'static str,
}

pub const REGISTERED_MAPPINGS: &[RegisteredMapping] = &[
  RegisteredMapping {
    candidate_kind: CandidateKind::RecurringChannelSuccess,
    target_table: "heldRoutingMap",
    mapper_id: "channel-success-to-routing-row",
  },
  RegisteredMapping {
    candidate_kind: CandidateKind::RecurringImportSpec,
    target_table: "knownImportsByLanguage",
    mapper_id: "import-spec-to-known-imports-row",
  },
  RegisteredMapping {
    candidate_kind: CandidateKind::MathExpressionLower,
    target_table: "knownAlgebraicIdentities",
    mapper_id: "math-expression-to-algebraic-identity-row",
  },
  RegisteredMapping {
    candidate_kind: CandidateKind::ChemicalReactionLower,
    target_table: "knownChemicalReactions",
    mapper_id: "chemical-reaction-to-known-reaction-row",
  },
];

/// Reverse of `held-to-query.px::queryKindMap`. Mirror of `.px`
/// `queryKindToHeldKind`. Sync test asserts every entry here has a
/// matching forward entry in `HELD_ROUTING`.
pub const QUERY_KIND_TO_HELD_KIND: &[(&str, ResolutionHeldKind)] = &[
  (
    "operator-asks-old-symbol",
    ResolutionHeldKind::MissingOldName,
  ),
  (
    "operator-asks-new-symbol",
    ResolutionHeldKind::MissingNewName,
  ),
  (
    "operator-asks-target-file",
    ResolutionHeldKind::MissingTargetPath,
  ),
  (
    "operator-asks-language",
    ResolutionHeldKind::LanguageNotDerivable,
  ),
  (
    "host-lints-unused-imports",
    ResolutionHeldKind::MissingCandidateImports,
  ),
  (
    "operator-asks-test-name",
    ResolutionHeldKind::MissingTestName,
  ),
  (
    "lookup-module-providing-symbol",
    ResolutionHeldKind::MissingImportSpec,
  ),
  (
    "extend-resolver-implementation",
    ResolutionHeldKind::TransformNotSupportedByResolver,
  ),
  (
    "operator-rephrase-identifier",
    ResolutionHeldKind::InvalidIdentifier,
  ),
  (
    "operator-rephrase-nontrivial-rename",
    ResolutionHeldKind::OldEqualsNew,
  ),
  (
    "lookup-algebraic-equivalent",
    ResolutionHeldKind::MissingAlgebraicEquivalent,
  ),
  (
    "lookup-chemical-reaction",
    ResolutionHeldKind::MissingChemistryProducts,
  ),
];

pub const DEFAULT_FALLBACK: HeldQueryRecoveryChannel = HeldQueryRecoveryChannel::OperatorFollowup;

fn registered_mapping_for(
  kind: CandidateKind,
  target_table: &str,
) -> Option<&'static RegisteredMapping> {
  REGISTERED_MAPPINGS
    .iter()
    .find(|r| r.candidate_kind == kind && r.target_table == target_table)
}

fn held_kind_for_query_kind(query_kind: &str) -> Option<ResolutionHeldKind> {
  QUERY_KIND_TO_HELD_KIND
    .iter()
    .find(|(qk, _)| *qk == query_kind)
    .map(|(_, h)| *h)
}

/// Output of the gate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchemaMappedCandidate {
  /// Original observer-shaped proposal. Audit-only.
  pub source: CandidateRowProposal,
  pub outcome: SchemaMappingOutcome,
  /// On `Mapped`: the new schema-aware proposal ready for Gate 2.
  /// On Held: `None`.
  pub schema_aware: Option<CandidateRowProposal>,
  pub reason: String,
}

/// Apply the registered schema mapping. Held outcomes preserve the
/// original proposal for audit.
pub fn map_proposal_to_schema(proposal: &CandidateRowProposal) -> SchemaMappedCandidate {
  let Some(registration) =
    registered_mapping_for(proposal.candidate_kind, proposal.target_table.as_str())
  else {
    return SchemaMappedCandidate {
      source: proposal.clone(),
      outcome: SchemaMappingOutcome::HeldUnsupportedKindTablePair,
      schema_aware: None,
      reason: format!(
        "no schema mapping registered for ({}, {}) — register one in `registeredMappings` first",
        proposal.candidate_kind.as_str(),
        proposal.target_table,
      ),
    };
  };

  match registration.mapper_id {
    "channel-success-to-routing-row" => map_channel_success_to_routing_row(proposal),
    "import-spec-to-known-imports-row" => map_import_spec_to_known_imports_row(proposal),
    "math-expression-to-algebraic-identity-row" => {
      map_math_expression_to_algebraic_identity_row(proposal)
    }
    "chemical-reaction-to-known-reaction-row" => {
      map_chemical_reaction_to_known_reaction_row(proposal)
    }
    other => SchemaMappedCandidate {
      source: proposal.clone(),
      outcome: SchemaMappingOutcome::HeldUnsupportedKindTablePair,
      schema_aware: None,
      reason: format!("mapper `{other}` is registered but not implemented in this Rust mirror"),
    },
  }
}

/// Fourth v0 mapper — substrate-sharing N=3 proof for chemistry.
/// Observer row from `ChemicalReactionLower` carries `reactants` +
/// `products` + `conditions` + `language`. The knownChemicalReactions
/// schema needs the same four fields verbatim — identity transform
/// with field validation, same shape as the math mapper.
fn map_chemical_reaction_to_known_reaction_row(
  proposal: &CandidateRowProposal,
) -> SchemaMappedCandidate {
  let required = ["reactants", "products", "conditions", "language"];
  let mut schema_row: BTreeMap<String, String> = BTreeMap::new();
  for field in &required {
    let Some(value) = proposal.proposed_row.get(*field) else {
      return SchemaMappedCandidate {
        source: proposal.clone(),
        outcome: SchemaMappingOutcome::HeldSourceFieldMissing,
        schema_aware: None,
        reason: format!("observer row missing required field `{field}`"),
      };
    };
    schema_row.insert(field.to_string(), value.clone());
  }

  let mut new_proposal = proposal.clone();
  new_proposal.proposed_row = schema_row;
  new_proposal.reason = format!(
    "schema-mapped from observer-shape via `chemical-reaction-to-known-reaction-row` (4 fields)"
  );

  SchemaMappedCandidate {
    source: proposal.clone(),
    outcome: SchemaMappingOutcome::Mapped,
    schema_aware: Some(new_proposal),
    reason: "observer row mapped to schema-aware knownChemicalReactions row".to_string(),
  }
}

/// Third v0 mapper — substrate-sharing proof for math domain.
/// Observer row from `MathExpressionLower` carries `canonical_form`
/// + `equivalent_form` + `language`. The knownAlgebraicIdentities
/// schema needs the same three fields verbatim — observer and
/// schema-aware shapes coincide for this lane, so the mapper is
/// effectively an identity transform with field validation.
///
/// Identity-shape mapping is intentional: math identities are
/// already structured at the ankh layer, so no schema rewrite is
/// needed. This is the simplest possible mapper, by design — the
/// substrate-sharing claim is that the *same firewall* handles
/// both code and math, not that math requires a complex mapper.
fn map_math_expression_to_algebraic_identity_row(
  proposal: &CandidateRowProposal,
) -> SchemaMappedCandidate {
  let Some(canonical) = proposal.proposed_row.get("canonical_form") else {
    return SchemaMappedCandidate {
      source: proposal.clone(),
      outcome: SchemaMappingOutcome::HeldSourceFieldMissing,
      schema_aware: None,
      reason: "observer row missing required field `canonical_form`".to_string(),
    };
  };
  let Some(equivalent) = proposal.proposed_row.get("equivalent_form") else {
    return SchemaMappedCandidate {
      source: proposal.clone(),
      outcome: SchemaMappingOutcome::HeldSourceFieldMissing,
      schema_aware: None,
      reason: "observer row missing required field `equivalent_form`".to_string(),
    };
  };
  let Some(language) = proposal.proposed_row.get("language") else {
    return SchemaMappedCandidate {
      source: proposal.clone(),
      outcome: SchemaMappingOutcome::HeldSourceFieldMissing,
      schema_aware: None,
      reason: "observer row missing required field `language`".to_string(),
    };
  };

  let mut schema_row: BTreeMap<String, String> = BTreeMap::new();
  schema_row.insert("canonical_form".to_string(), canonical.clone());
  schema_row.insert("equivalent_form".to_string(), equivalent.clone());
  schema_row.insert("language".to_string(), language.clone());

  let mut new_proposal = proposal.clone();
  new_proposal.proposed_row = schema_row;
  new_proposal.reason = format!(
    "schema-mapped from observer-shape via `math-expression-to-algebraic-identity-row`: canonical={canonical} equivalent={equivalent} language={language}"
  );

  SchemaMappedCandidate {
    source: proposal.clone(),
    outcome: SchemaMappingOutcome::Mapped,
    schema_aware: Some(new_proposal),
    reason: format!(
      "observer row mapped to schema-aware knownAlgebraicIdentities row (canonical={canonical}, equivalent={equivalent})"
    ),
  }
}

/// Second v0 mapper. Observer row from `RecurringImportSpec`
/// already carries `import_spec` + `language` (plus
/// `distinct_target_paths` for audit). Strip the audit-only field;
/// retain the two schema-required fields verbatim.
fn map_import_spec_to_known_imports_row(proposal: &CandidateRowProposal) -> SchemaMappedCandidate {
  let Some(import_spec) = proposal.proposed_row.get("import_spec") else {
    return SchemaMappedCandidate {
      source: proposal.clone(),
      outcome: SchemaMappingOutcome::HeldSourceFieldMissing,
      schema_aware: None,
      reason: "observer row missing required field `import_spec`".to_string(),
    };
  };
  let Some(language) = proposal.proposed_row.get("language") else {
    return SchemaMappedCandidate {
      source: proposal.clone(),
      outcome: SchemaMappingOutcome::HeldSourceFieldMissing,
      schema_aware: None,
      reason: "observer row missing required field `language`".to_string(),
    };
  };

  let mut schema_row: BTreeMap<String, String> = BTreeMap::new();
  schema_row.insert("language".to_string(), language.clone());
  schema_row.insert("import_spec".to_string(), import_spec.clone());

  let mut new_proposal = proposal.clone();
  new_proposal.proposed_row = schema_row;
  new_proposal.reason = format!(
    "schema-mapped from observer-shape via `import-spec-to-known-imports-row`: language={language} import_spec={import_spec}"
  );

  SchemaMappedCandidate {
    source: proposal.clone(),
    outcome: SchemaMappingOutcome::Mapped,
    schema_aware: Some(new_proposal),
    reason: format!(
      "observer row mapped to schema-aware knownImportsByLanguage row (language={language}, import_spec={import_spec})"
    ),
  }
}

/// The one mapper v0 implements. Reads `query_kind` +
/// `observed_primary_channel` from the observer row, derives
/// `held` via reverse query-kind map, copies primary verbatim,
/// sets fallback to the conservative default.
fn map_channel_success_to_routing_row(proposal: &CandidateRowProposal) -> SchemaMappedCandidate {
  let Some(query_kind) = proposal.proposed_row.get("query_kind") else {
    return SchemaMappedCandidate {
      source: proposal.clone(),
      outcome: SchemaMappingOutcome::HeldSourceFieldMissing,
      schema_aware: None,
      reason: "observer row missing required field `query_kind`".to_string(),
    };
  };
  let Some(observed_primary) = proposal.proposed_row.get("observed_primary_channel") else {
    return SchemaMappedCandidate {
      source: proposal.clone(),
      outcome: SchemaMappingOutcome::HeldSourceFieldMissing,
      schema_aware: None,
      reason: "observer row missing required field `observed_primary_channel`".to_string(),
    };
  };
  let Some(held) = held_kind_for_query_kind(query_kind) else {
    return SchemaMappedCandidate {
      source: proposal.clone(),
      outcome: SchemaMappingOutcome::HeldUnknownQueryKind,
      schema_aware: None,
      reason: format!(
        "query_kind `{query_kind}` has no entry in `queryKindToHeldKind` — possibly a new kind not yet registered"
      ),
    };
  };

  let mut schema_row: BTreeMap<String, String> = BTreeMap::new();
  schema_row.insert("held".to_string(), held.as_str().to_string());
  schema_row.insert("primary".to_string(), observed_primary.clone());
  schema_row.insert(
    "fallback".to_string(),
    DEFAULT_FALLBACK.as_str().to_string(),
  );

  let mut new_proposal = proposal.clone();
  new_proposal.proposed_row = schema_row;
  new_proposal.reason = format!(
    "schema-mapped from observer-shape via `channel-success-to-routing-row`: held={} primary={} fallback={}",
    held.as_str(),
    observed_primary,
    DEFAULT_FALLBACK.as_str(),
  );

  SchemaMappedCandidate {
    source: proposal.clone(),
    outcome: SchemaMappingOutcome::Mapped,
    schema_aware: Some(new_proposal),
    reason: format!(
      "observer row mapped to schema-aware heldRoutingMap row (held={}, primary={})",
      held.as_str(),
      observed_primary,
    ),
  }
}

/// Batch.
pub fn map_all(proposals: &[CandidateRowProposal]) -> Vec<SchemaMappedCandidate> {
  proposals.iter().map(map_proposal_to_schema).collect()
}

/// Render a `SchemaMappedCandidate` as the canonical JSON payload of
/// a `coding.schema-mapped-candidate` artifact. Stage D-1.5 —
/// the transformation receipt between observer-shape (Gate 1
/// output) and target-schema (Gate 2 input). On `Mapped`, carries
/// both the source row and the schema-aware row so the operator can
/// see *what the mapper changed*. On Held, carries the source row
/// only + the failure reason.
///
/// Single family for all 4 outcomes (status field: `mapped` /
/// `held-unsupported-kind-table-pair` / `held-source-field-missing`
/// / `held-unknown-query-kind`) — same flat-family pattern as
/// `retrieval-result`.
///
/// Replay-stable id = SHA-256 of intrinsic identity (outcome +
/// candidate_kind + target_owner + target_table + sorted source row
/// + sorted schema_aware row, if any). `stored_at_ms` is extrinsic.
///
/// Content policy: metadata + row key-value pairs (caller-injected
/// observer data). No source bodies — customer-release safe.
pub fn build_schema_mapped_candidate_artifact(
  mapped: &SchemaMappedCandidate,
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
) -> serde_json::Value {
  let mut h = Sha256::new();
  h.update(b"schema-mapped-candidate\x1f");
  h.update(mapped.outcome.as_str().as_bytes());
  h.update(b"\x1f");
  h.update(mapped.source.candidate_kind.as_str().as_bytes());
  h.update(b"\x1e");
  h.update(mapped.source.target_owner.as_bytes());
  h.update(b"\x1e");
  h.update(mapped.source.target_table.as_bytes());
  h.update(b"\x1f");
  let mut src_keys: Vec<&String> = mapped.source.proposed_row.keys().collect();
  src_keys.sort();
  for k in src_keys {
    h.update(k.as_bytes());
    h.update(b"\x1d");
    h.update(mapped.source.proposed_row[k].as_bytes());
    h.update(b"\x1e");
  }
  h.update(b"\x1f");
  if let Some(sa) = &mapped.schema_aware {
    let mut sa_keys: Vec<&String> = sa.proposed_row.keys().collect();
    sa_keys.sort();
    for k in sa_keys {
      h.update(k.as_bytes());
      h.update(b"\x1d");
      h.update(sa.proposed_row[k].as_bytes());
      h.update(b"\x1e");
    }
  }
  let digest = h.finalize();
  let prefix = digest
    .iter()
    .take(16)
    .map(|b| format!("{b:02x}"))
    .collect::<String>();
  let id = format!("schema-mapped-candidate.{prefix}");

  // Find the registered mapper id for the cockpit display (audit
  // trail — operator can map outcome → mapper rule).
  let mapper_id = registered_mapping_for(
    mapped.source.candidate_kind,
    mapped.source.target_table.as_str(),
  )
  .map(|r| r.mapper_id.to_string())
  .unwrap_or_else(|| "<unregistered>".to_string());

  let mut payload = serde_json::json!({
    "id": id,
    "artifact_family": "coding.schema-mapped-candidate",
    "source_surface": "algorithm-synthesis.schema-mapping-gate",
    "stored_at_ms": stored_at_ms,
    "outcome": mapped.outcome.as_str(),
    "candidate_kind": mapped.source.candidate_kind.as_str(),
    "target_owner": mapped.source.target_owner,
    "target_table": mapped.source.target_table,
    "mapper_id": mapper_id,
    "source_row": mapped.source.proposed_row,
    "reason": mapped.reason,
    "related_refs": serde_json::json!([
      format!("candidate-kind:{}", mapped.source.candidate_kind.as_str()),
      format!("target-owner:{}", mapped.source.target_owner),
      format!("target-table:{}", mapped.source.target_table),
      "owner-law:stdlib/lib/gate/algorithm-synthesis/schema-mapping-gate.px",
    ]),
    "target_paths": serde_json::json!([mapped.source.target_owner]),
    "command_refs": Vec::<String>::new(),
  });

  // On Mapped: surface schema_aware row + reveal the row diff at
  // the panel level. On Held: omit (None).
  if let Some(sa) = &mapped.schema_aware {
    payload["schema_aware_row"] = serde_json::to_value(&sa.proposed_row).unwrap_or_default();
    payload["schema_aware_target_owner"] = serde_json::Value::String(sa.target_owner.clone());
    payload["schema_aware_target_table"] = serde_json::Value::String(sa.target_table.clone());
  }

  if let Some(snap) = repo_snapshot_ref {
    payload["repo_snapshot_ref"] = serde_json::Value::String(snap.to_string());
  }
  payload
}

#[cfg(test)]
mod tests {
  use super::super::candidate_row_proposal::GateStatus;
  use super::*;

  fn observer_proposal(
    kind: CandidateKind,
    target_table: &str,
    row: &[(&str, &str)],
  ) -> CandidateRowProposal {
    let mut proposed = BTreeMap::new();
    for (k, v) in row {
      proposed.insert(k.to_string(), v.to_string());
    }
    CandidateRowProposal {
      candidate_kind: kind,
      target_owner: "stdlib/lib/gate/test.px".to_string(),
      target_table: target_table.to_string(),
      proposed_row: proposed,
      supporting_evidence: vec!["e1".to_string()],
      evidence_count: 2,
      gate_status: GateStatus::IntentReceiptOnly,
      reason: "test".to_string(),
    }
  }

  // ─── outcome universe ──────────────────────────────────────────

  #[test]
  fn every_outcome_has_string_form() {
    for o in SchemaMappingOutcome::ALL {
      assert!(!o.as_str().is_empty());
    }
  }

  // ─── happy path ───────────────────────────────────────────────

  #[test]
  fn channel_success_to_routing_row_maps_correctly() {
    let p = observer_proposal(
      CandidateKind::RecurringChannelSuccess,
      "heldRoutingMap",
      &[
        ("query_kind", "lookup-module-providing-symbol"),
        ("observed_primary_channel", "host-symbol-resolver"),
      ],
    );
    let r = map_proposal_to_schema(&p);
    assert_eq!(r.outcome, SchemaMappingOutcome::Mapped);
    let aware = r.schema_aware.expect("schema_aware");
    assert_eq!(
      aware.proposed_row.get("held").unwrap(),
      "missing-import-spec"
    );
    assert_eq!(
      aware.proposed_row.get("primary").unwrap(),
      "host-symbol-resolver"
    );
    assert_eq!(
      aware.proposed_row.get("fallback").unwrap(),
      "operator-followup"
    );
    // Audit: candidate_kind + target_table + evidence preserved.
    assert_eq!(aware.candidate_kind, CandidateKind::RecurringChannelSuccess);
    assert_eq!(aware.target_table, "heldRoutingMap");
    assert_eq!(aware.evidence_count, p.evidence_count);
    assert_eq!(aware.supporting_evidence, p.supporting_evidence);
  }

  #[test]
  fn maps_all_query_kinds_in_reverse_table() {
    // Every (query_kind, held_kind) row must map cleanly.
    for (qk, expected_held) in QUERY_KIND_TO_HELD_KIND {
      let p = observer_proposal(
        CandidateKind::RecurringChannelSuccess,
        "heldRoutingMap",
        &[
          ("query_kind", qk),
          ("observed_primary_channel", "host-symbol-resolver"),
        ],
      );
      let r = map_proposal_to_schema(&p);
      assert_eq!(
        r.outcome,
        SchemaMappingOutcome::Mapped,
        "query_kind `{qk}` failed to map"
      );
      assert_eq!(
        r.schema_aware.unwrap().proposed_row.get("held").unwrap(),
        expected_held.as_str()
      );
    }
  }

  // ─── held paths ───────────────────────────────────────────────

  #[test]
  fn unsupported_kind_table_pair_holds() {
    let p = observer_proposal(
      CandidateKind::RecurringImportSpec,
      "factPhrasePatterns",
      &[("import_spec", "import os"), ("language", "python")],
    );
    let r = map_proposal_to_schema(&p);
    assert_eq!(
      r.outcome,
      SchemaMappingOutcome::HeldUnsupportedKindTablePair
    );
    assert!(r.schema_aware.is_none());
  }

  #[test]
  fn missing_query_kind_field_holds() {
    let p = observer_proposal(
      CandidateKind::RecurringChannelSuccess,
      "heldRoutingMap",
      &[("observed_primary_channel", "host-symbol-resolver")],
    );
    let r = map_proposal_to_schema(&p);
    assert_eq!(r.outcome, SchemaMappingOutcome::HeldSourceFieldMissing);
  }

  #[test]
  fn missing_observed_primary_channel_field_holds() {
    let p = observer_proposal(
      CandidateKind::RecurringChannelSuccess,
      "heldRoutingMap",
      &[("query_kind", "lookup-module-providing-symbol")],
    );
    let r = map_proposal_to_schema(&p);
    assert_eq!(r.outcome, SchemaMappingOutcome::HeldSourceFieldMissing);
  }

  #[test]
  fn unknown_query_kind_holds() {
    let p = observer_proposal(
      CandidateKind::RecurringChannelSuccess,
      "heldRoutingMap",
      &[
        ("query_kind", "novel-future-query-kind-not-yet-registered"),
        ("observed_primary_channel", "host-symbol-resolver"),
      ],
    );
    let r = map_proposal_to_schema(&p);
    assert_eq!(r.outcome, SchemaMappingOutcome::HeldUnknownQueryKind);
  }

  // ─── registry consistency ─────────────────────────────────────

  #[test]
  fn every_query_kind_has_distinct_held_kind() {
    // QUERY_KIND_TO_HELD_KIND is a function (no duplicate
    // query_kind keys, no duplicate held values either since
    // ResolutionHeldKind is closed).
    let mut seen_q = std::collections::HashSet::new();
    let mut seen_h = std::collections::HashSet::new();
    for (qk, h) in QUERY_KIND_TO_HELD_KIND {
      assert!(seen_q.insert(*qk), "duplicate query_kind: {qk}");
      assert!(
        seen_h.insert(h.as_str()),
        "duplicate held kind: {}",
        h.as_str()
      );
    }
  }

  // ─── batch ─────────────────────────────────────────────────────

  #[test]
  fn map_all_returns_one_per_input() {
    let proposals = vec![
      observer_proposal(
        CandidateKind::RecurringChannelSuccess,
        "heldRoutingMap",
        &[
          ("query_kind", "lookup-module-providing-symbol"),
          ("observed_primary_channel", "host-symbol-resolver"),
        ],
      ),
      observer_proposal(
        CandidateKind::RecurringImportSpec,
        "factPhrasePatterns",
        &[("import_spec", "import os")],
      ),
    ];
    let results = map_all(&proposals);
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].outcome, SchemaMappingOutcome::Mapped);
    assert_eq!(
      results[1].outcome,
      SchemaMappingOutcome::HeldUnsupportedKindTablePair
    );
  }

  // ─── schema-mapped-candidate artifact (Stage D-1.5 panel) ────

  fn mapped_channel_success() -> SchemaMappedCandidate {
    map_proposal_to_schema(&observer_proposal(
      CandidateKind::RecurringChannelSuccess,
      "heldRoutingMap",
      &[
        ("query_kind", "lookup-module-providing-symbol"),
        ("observed_primary_channel", "host-symbol-resolver"),
      ],
    ))
  }

  fn held_unsupported_pair() -> SchemaMappedCandidate {
    map_proposal_to_schema(&observer_proposal(
      CandidateKind::RecurringImportSpec,
      "factPhrasePatterns",
      &[("import_spec", "import os")],
    ))
  }

  fn held_missing_field() -> SchemaMappedCandidate {
    map_proposal_to_schema(&observer_proposal(
      CandidateKind::RecurringImportSpec,
      "knownImportsByLanguage",
      &[("language", "python")],
    ))
  }

  #[test]
  fn artifact_mapped_carries_source_and_schema_aware_rows() {
    let m = mapped_channel_success();
    assert_eq!(m.outcome, SchemaMappingOutcome::Mapped);
    let art = build_schema_mapped_candidate_artifact(&m, 1700000000000, None);
    assert_eq!(art["artifact_family"], "coding.schema-mapped-candidate");
    assert_eq!(art["outcome"], "mapped");
    assert_eq!(art["candidate_kind"], "recurring-channel-success");
    assert_eq!(art["target_table"], "heldRoutingMap");
    assert_eq!(art["mapper_id"], "channel-success-to-routing-row");
    // source row preserved
    let src = art["source_row"].as_object().unwrap();
    assert!(src.contains_key("query_kind"));
    // schema_aware row materialized
    let sa = art["schema_aware_row"]
      .as_object()
      .expect("schema_aware_row");
    assert!(!sa.is_empty(), "Mapped must carry schema_aware row");
  }

  #[test]
  fn artifact_held_carries_source_only_no_schema_aware() {
    let m = held_unsupported_pair();
    assert_eq!(
      m.outcome,
      SchemaMappingOutcome::HeldUnsupportedKindTablePair
    );
    let art = build_schema_mapped_candidate_artifact(&m, 0, None);
    assert_eq!(art["outcome"], "held-unsupported-kind-table-pair");
    assert!(art["source_row"].is_object());
    assert!(
      art.get("schema_aware_row").is_none(),
      "Held outcomes must not carry schema_aware_row"
    );
    assert_eq!(art["mapper_id"], "<unregistered>");
  }

  #[test]
  fn artifact_held_missing_field_preserves_reason_and_pair() {
    let m = held_missing_field();
    assert_eq!(m.outcome, SchemaMappingOutcome::HeldSourceFieldMissing);
    let art = build_schema_mapped_candidate_artifact(&m, 0, None);
    assert_eq!(art["outcome"], "held-source-field-missing");
    assert_eq!(art["mapper_id"], "import-spec-to-known-imports-row");
    assert!(art.get("schema_aware_row").is_none());
    let reason = art["reason"].as_str().unwrap_or("");
    assert!(
      !reason.is_empty(),
      "Held outcomes must surface why the mapper held"
    );
  }

  #[test]
  fn artifact_id_is_replay_stable_across_stored_at() {
    let m = mapped_channel_success();
    let a1 = build_schema_mapped_candidate_artifact(&m, 1, None);
    let a2 = build_schema_mapped_candidate_artifact(&m, 999999, None);
    assert_eq!(a1["id"], a2["id"]);
  }

  #[test]
  fn artifact_id_differs_between_mapped_and_held() {
    let mapped = mapped_channel_success();
    let held = held_unsupported_pair();
    let a_m = build_schema_mapped_candidate_artifact(&mapped, 0, None);
    let a_h = build_schema_mapped_candidate_artifact(&held, 0, None);
    assert_ne!(a_m["id"], a_h["id"]);
  }

  #[test]
  fn artifact_related_refs_walk_back_to_kind_owner_table() {
    let m = mapped_channel_success();
    let art = build_schema_mapped_candidate_artifact(&m, 0, None);
    let refs: Vec<String> = serde_json::from_value(art["related_refs"].clone()).unwrap();
    assert!(refs.iter().any(|r| r.starts_with("candidate-kind:")));
    assert!(refs.iter().any(|r| r.starts_with("target-table:")));
    assert!(refs.iter().any(|r| r.contains("schema-mapping-gate.px")));
  }
}
