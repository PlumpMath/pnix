//! Candidate `.px` row proposal — Stage D-v0 of the evolution lane.
//!
//! OWNER-LAW (2026-05-12): mirror of
//! `stdlib/lib/gate/algorithm-synthesis/candidate-row-proposal.px`.
//! Reads accumulated ankh entries and emits typed proposals for
//! `.px` row candidates that have passed the FIRST of the 5
//! firewall gates (`intent-receipt`) only.
//!
//! Per the OWNER-LAW CONSTITUTION (CLAUDE.md), a proposal is NOT an
//! applied row and NOT executable code. It is a typed observation.
//! Four more gates (`macro-fold`, `axis-separation`,
//! `regression-proof`, `owner-law`) must close before any actual
//! `.px` file is edited.
//! v0 closes the first gate only; the remaining gates are future
//! Stage D-v1..v4 owners.

use serde::{Deserialize, Serialize};
use pnix_hash::{Digest, Sha256};
use std::collections::BTreeMap;

use super::ankh_retrieval_cache::{AnkhEntry, AnkhProvenanceSource, AnkhRetrievalKey, AnkhStore};

/// The 5 firewall gates from the constitution. Stays byte-identical
/// to `.px` `firewallGates`.
pub const FIREWALL_GATES: &[&str] = &[
  "intent-receipt",
  "macro-fold",
  "axis-separation",
  "regression-proof",
  "owner-law",
];

/// Candidate kinds this v0 owner can emit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CandidateKind {
  /// ≥ N ankh entries answer the same `(query_kind,
  /// provenance_source)` pair. Suggests `held-to-query` routing
  /// could be tuned.
  RecurringChannelSuccess,
  /// ≥ N ankh entries supply the same `import_spec` value across
  /// different target_paths. Suggests this import is common enough
  /// to be a default seed or to seed a `fact:*` cue.
  RecurringImportSpec,
  /// ≥ N ankh entries supply the same `equivalent_form` for the
  /// same `canonical_form` across different surrounding contexts.
  /// Suggests this algebraic identity is stable enough to register
  /// as a known identity. **Same firewall, different domain** —
  /// proves the algorithm-synthesis substrate is domain-agnostic.
  ///
  /// Triggering ankh shape:
  ///   - query_kind: `lookup-algebraic-equivalent`
  ///   - supplied_parameters: `canonical_form`, `equivalent_form`
  /// Grouping key: `(canonical_form, equivalent_form)` pair.
  MathExpressionLower,
  /// ≥ N ankh entries supply the same `products` for the same
  /// `(reactants, conditions)` triple across different surrounding
  /// contexts. Substrate-sharing **N=3 proof**: same firewall, third
  /// domain (chemistry).
  ///
  /// Triggering ankh shape:
  ///   - query_kind: `lookup-chemical-reaction`
  ///   - context_snapshot: `reactants`, `conditions`
  ///   - supplied_parameters: `products`
  /// Grouping key: `(reactants, products, conditions, language)` triple
  /// where `language` is the chemistry sub-domain (`inorganic` /
  /// `organic` / `biochem`).
  ChemicalReactionLower,
  /// **User-authored direct-injection lane** (2026-05-14). A user
  /// supplies a concrete `(cue, intent, weight)` row and an explicit
  /// owner-law approval. The 5-gate firewall evaluates the proposal
  /// identically to the ankh-derived variants — same fold, same
  /// axis separation, same regression-proof (uniqueness check on
  /// `cue`), same owner-law gate. The *target* is the separate
  /// `learned-intent-overlay.px` file rather than a static stdlib
  /// table — the substrate's `classifyWithLearnedOverlayAt`
  /// consumer auto-imports it on the next turn.
  ///
  /// Minimum evidence count = 1 (the user themselves is the
  /// evidence; Gate 5's `PromotionApproval` carries `actor_id +
  /// tenant_id`).
  LearnedIntentSignal,
  /// Same direct-injection pattern as `LearnedIntentSignal`, but
  /// for the operation-map surface. Row shape:
  ///   { intent, cues, transform, weight }
  /// where `cues` is a comma-separated flat string (the row table
  /// schema is `BTreeMap<String,String>`). Target =
  /// `learned-operation-overlay.px::overlayOperationRows`.
  /// Uniqueness on the `(intent, cues, transform)` triple.
  LearnedOperationMap,
  /// Same direct-injection pattern, parameter-resolution surface.
  /// Row shape: `{ operation_candidate, resolved_fields }`. The
  /// `resolved_fields` value is the JSON serialisation of the inner
  /// attrset (the row table is flat `BTreeMap<String,String>`).
  /// Target = `learned-parameter-overlay.px::overlayParameterRows`.
  /// Uniqueness on `operation_candidate`.
  LearnedParameterResolution,
  /// Same direct-injection pattern, fact-cue-registry surface.
  /// Row shape: `{ cue, markers }` where `markers` is a comma-
  /// separated flat string. Target =
  /// `learned-fact-cue-overlay.px::overlayPhrasePatterns`.
  /// Uniqueness on `cue`.
  LearnedFactCuePhrasePattern,
}

impl CandidateKind {
  pub const ALL: &'static [Self] = &[
    Self::RecurringChannelSuccess,
    Self::RecurringImportSpec,
    Self::MathExpressionLower,
    Self::ChemicalReactionLower,
    Self::LearnedIntentSignal,
    Self::LearnedOperationMap,
    Self::LearnedParameterResolution,
    Self::LearnedFactCuePhrasePattern,
  ];

  pub fn as_str(self) -> &'static str {
    match self {
      Self::RecurringChannelSuccess => "recurring-channel-success",
      Self::RecurringImportSpec => "recurring-import-spec",
      Self::MathExpressionLower => "math-expression-lower",
      Self::ChemicalReactionLower => "chemical-reaction-lower",
      Self::LearnedIntentSignal => "learned-intent-signal",
      Self::LearnedOperationMap => "learned-operation-map",
      Self::LearnedParameterResolution => "learned-parameter-resolution",
      Self::LearnedFactCuePhrasePattern => "learned-fact-cue-phrase-pattern",
    }
  }
}

/// Gate status values. v0 always emits `IntentReceiptOnly`. Other
/// variants reserved for future gate owners.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GateStatus {
  IntentReceiptOnly,
  MacroFoldAttempted,
  AxisSeparationAttempted,
  RegressionProofAttempted,
  OwnerLawAttempted,
  Promoted,
  Held,
  Rejected,
}

impl GateStatus {
  pub const ALL: &'static [Self] = &[
    Self::IntentReceiptOnly,
    Self::MacroFoldAttempted,
    Self::AxisSeparationAttempted,
    Self::RegressionProofAttempted,
    Self::OwnerLawAttempted,
    Self::Promoted,
    Self::Held,
    Self::Rejected,
  ];

  pub fn as_str(self) -> &'static str {
    match self {
      Self::IntentReceiptOnly => "intent-receipt-only",
      Self::MacroFoldAttempted => "macro-fold-attempted",
      Self::AxisSeparationAttempted => "axis-separation-attempted",
      Self::RegressionProofAttempted => "regression-proof-attempted",
      Self::OwnerLawAttempted => "owner-law-attempted",
      Self::Promoted => "promoted",
      Self::Held => "held",
      Self::Rejected => "rejected",
    }
  }
}

#[derive(Debug, Clone, Copy)]
pub struct EvidenceThresholdRow {
  pub kind: CandidateKind,
  pub minimum: usize,
}

pub const MINIMUM_EVIDENCE_COUNTS: &[EvidenceThresholdRow] = &[
  EvidenceThresholdRow {
    kind: CandidateKind::RecurringChannelSuccess,
    minimum: 2,
  },
  EvidenceThresholdRow {
    kind: CandidateKind::RecurringImportSpec,
    minimum: 2,
  },
  EvidenceThresholdRow {
    kind: CandidateKind::MathExpressionLower,
    minimum: 2,
  },
  EvidenceThresholdRow {
    kind: CandidateKind::ChemicalReactionLower,
    minimum: 2,
  },
  // User-authored direct-injection lane: one row is enough — the
  // user themselves is the supporting evidence, carried by the
  // owner-law Gate 5 `PromotionApproval`.
  EvidenceThresholdRow {
    kind: CandidateKind::LearnedIntentSignal,
    minimum: 1,
  },
  EvidenceThresholdRow {
    kind: CandidateKind::LearnedOperationMap,
    minimum: 1,
  },
  EvidenceThresholdRow {
    kind: CandidateKind::LearnedParameterResolution,
    minimum: 1,
  },
  EvidenceThresholdRow {
    kind: CandidateKind::LearnedFactCuePhrasePattern,
    minimum: 1,
  },
];

#[derive(Debug, Clone, Copy)]
pub struct CandidateTargetRow {
  pub kind: CandidateKind,
  pub target_owner: &'static str,
  pub target_table: &'static str,
}

pub const CANDIDATE_TARGET_OWNERS: &[CandidateTargetRow] = &[
  CandidateTargetRow {
    kind: CandidateKind::RecurringChannelSuccess,
    target_owner: "stdlib/lib/gate/algorithm-synthesis/held-to-query.px",
    target_table: "heldRoutingMap",
  },
  CandidateTargetRow {
    kind: CandidateKind::RecurringImportSpec,
    target_owner: "stdlib/lib/gate/known-imports-by-language.px",
    target_table: "knownImportsByLanguage",
  },
  CandidateTargetRow {
    kind: CandidateKind::MathExpressionLower,
    target_owner: "stdlib/lib/gate/known-algebraic-identities.px",
    target_table: "knownAlgebraicIdentities",
  },
  CandidateTargetRow {
    kind: CandidateKind::ChemicalReactionLower,
    target_owner: "stdlib/lib/gate/known-chemical-reactions.px",
    target_table: "knownChemicalReactions",
  },
  // Learned-intent-overlay direct-injection lane targets a
  // *separate* `.px` file (not a static stdlib table). The
  // file is written by `apply_learned_overlay_write` and
  // consumed next turn via `classifyWithLearnedOverlayAt`.
  CandidateTargetRow {
    kind: CandidateKind::LearnedIntentSignal,
    target_owner: "stdlib/lib/gate/algorithm-synthesis/learned-intent-overlay.px",
    target_table: "overlayIntentSignals",
  },
  CandidateTargetRow {
    kind: CandidateKind::LearnedOperationMap,
    target_owner: "stdlib/lib/gate/algorithm-synthesis/learned-operation-overlay.px",
    target_table: "overlayOperationRows",
  },
  CandidateTargetRow {
    kind: CandidateKind::LearnedParameterResolution,
    target_owner: "stdlib/lib/gate/algorithm-synthesis/learned-parameter-overlay.px",
    target_table: "overlayParameterRows",
  },
  CandidateTargetRow {
    kind: CandidateKind::LearnedFactCuePhrasePattern,
    target_owner: "stdlib/lib/gate/algorithm-synthesis/learned-fact-cue-overlay.px",
    target_table: "overlayPhrasePatterns",
  },
];

/// Query kinds whose operator-followup entries can propose a
/// learned-intent overlay row. Mirror of `.px`
/// `learnedIntentSignalAnkhQueryKinds`.
pub const LEARNED_INTENT_SIGNAL_ANKH_QUERY_KINDS: &[&str] = &["operator-learned-intent-signal"];

/// Provenance values allowed to lower into a learned-intent overlay
/// row through ankh. Mirror of `.px`
/// `learnedIntentSignalAnkhAllowedProvenanceSources`.
pub const LEARNED_INTENT_SIGNAL_ANKH_ALLOWED_PROVENANCE: &[AnkhProvenanceSource] =
  &[AnkhProvenanceSource::OperatorFollowup];

/// Required `supplied_parameters` fields for an ankh-derived
/// learned-intent signal proposal. Mirror of `.px`
/// `learnedIntentSignalAnkhRequiredSuppliedParameters`.
pub const LEARNED_INTENT_SIGNAL_ANKH_REQUIRED_SUPPLIED_PARAMETERS: &[&str] =
  &["cue", "intent", "weight"];

pub const LEARNED_OPERATION_MAP_ANKH_QUERY_KINDS: &[&str] = &["operator-learned-operation-map"];
pub const LEARNED_OPERATION_MAP_ANKH_ALLOWED_PROVENANCE: &[AnkhProvenanceSource] =
  &[AnkhProvenanceSource::OperatorFollowup];
pub const LEARNED_OPERATION_MAP_ANKH_REQUIRED_SUPPLIED_PARAMETERS: &[&str] =
  &["intent", "cues", "transform", "weight"];

pub const LEARNED_PARAMETER_RESOLUTION_ANKH_QUERY_KINDS: &[&str] =
  &["operator-learned-parameter-resolution"];
pub const LEARNED_PARAMETER_RESOLUTION_ANKH_ALLOWED_PROVENANCE: &[AnkhProvenanceSource] =
  &[AnkhProvenanceSource::OperatorFollowup];
pub const LEARNED_PARAMETER_RESOLUTION_ANKH_REQUIRED_SUPPLIED_PARAMETERS: &[&str] =
  &["operation_candidate", "resolved_fields"];

pub const LEARNED_FACT_CUE_PHRASE_PATTERN_ANKH_QUERY_KINDS: &[&str] =
  &["operator-learned-fact-cue-phrase-pattern"];
pub const LEARNED_FACT_CUE_PHRASE_PATTERN_ANKH_ALLOWED_PROVENANCE: &[AnkhProvenanceSource] =
  &[AnkhProvenanceSource::OperatorFollowup];
pub const LEARNED_FACT_CUE_PHRASE_PATTERN_ANKH_REQUIRED_SUPPLIED_PARAMETERS: &[&str] =
  &["cue", "markers"];

fn minimum_for(kind: CandidateKind) -> usize {
  MINIMUM_EVIDENCE_COUNTS
    .iter()
    .find(|r| r.kind == kind)
    .map(|r| r.minimum)
    .unwrap_or(usize::MAX)
}

fn target_for(kind: CandidateKind) -> Option<&'static CandidateTargetRow> {
  CANDIDATE_TARGET_OWNERS.iter().find(|r| r.kind == kind)
}

/// A single typed proposal. Carries the structured observation
/// only — actual `.px` row text generation is the macro-fold gate's
/// job (Stage D-v1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandidateRowProposal {
  pub candidate_kind: CandidateKind,
  /// `.px` file the proposal targets (e.g. `held-to-query.px`).
  pub target_owner: String,
  /// Named table inside the target file (e.g. `heldRoutingMap`).
  pub target_table: String,
  /// Structured row data (key-value pairs). The macro-fold gate
  /// renders this into actual `.px` source text.
  pub proposed_row: BTreeMap<String, String>,
  /// Ankh entry keys (or fingerprints) that support this proposal.
  /// Audit trail — the human reviewer can walk back to the exact
  /// evidence that justified this candidate.
  pub supporting_evidence: Vec<String>,
  /// Number of ankh entries that contributed. Always
  /// `>= minimum_for(candidate_kind)`.
  pub evidence_count: usize,
  /// Gate status — v0 always emits `IntentReceiptOnly`.
  pub gate_status: GateStatus,
  /// Human-readable summary the cockpit can show alongside the
  /// proposal in a review queue.
  pub reason: String,
}

fn fingerprint(key: &AnkhRetrievalKey) -> String {
  format!("{}|{}|{}", key.query_kind, key.target_path, key.language)
}

fn can_lower_into_semantic_row(entry: &AnkhEntry) -> bool {
  // Paper-note quarantine: external text may prove that a recovery
  // channel recurs, but it must not directly synthesize static
  // knowledge rows. A separate owner-reviewed lowering lane must
  // convert external paper notes into host/operator evidence first.
  entry.provenance_source != AnkhProvenanceSource::ExternalKnowledgeSearch
}

fn can_lower_explicit_learned_overlay_row(
  entry: &AnkhEntry,
  query_kinds: &[&str],
  allowed_provenance: &[AnkhProvenanceSource],
  required_fields: &[&str],
) -> bool {
  query_kinds.contains(&entry.query_kind.as_str())
    && allowed_provenance.contains(&entry.provenance_source)
    && required_fields
      .iter()
      .all(|field| entry.supplied_parameters.contains_key(*field))
}

/// Walk every ankh entry, group by `(query_kind, provenance_source)`,
/// and emit a `RecurringChannelSuccess` proposal for each group
/// whose size meets the threshold.
fn propose_recurring_channel_success(
  entries: &[(AnkhRetrievalKey, AnkhEntry)],
) -> Vec<CandidateRowProposal> {
  let min = minimum_for(CandidateKind::RecurringChannelSuccess);
  let target = target_for(CandidateKind::RecurringChannelSuccess);
  let mut buckets: BTreeMap<(String, AnkhProvenanceSource), Vec<&AnkhRetrievalKey>> =
    BTreeMap::new();
  for (k, e) in entries {
    buckets
      .entry((e.query_kind.clone(), e.provenance_source))
      .or_default()
      .push(k);
  }
  let mut out = Vec::new();
  for ((query_kind, provenance), keys) in buckets {
    if keys.len() < min {
      continue;
    }
    let supporting: Vec<String> = keys.iter().map(|k| fingerprint(k)).collect();
    let mut row = BTreeMap::new();
    row.insert("query_kind".to_string(), query_kind.clone());
    row.insert(
      "observed_primary_channel".to_string(),
      provenance.as_str().to_string(),
    );
    let Some(t) = target else {
      continue;
    };
    out.push(CandidateRowProposal {
      candidate_kind: CandidateKind::RecurringChannelSuccess,
      target_owner: t.target_owner.to_string(),
      target_table: t.target_table.to_string(),
      proposed_row: row,
      supporting_evidence: supporting,
      evidence_count: keys.len(),
      gate_status: GateStatus::IntentReceiptOnly,
      reason: format!(
        "observed {} ankh entries answering `{query_kind}` via `{}`; consider tuning `{}`",
        keys.len(),
        provenance.as_str(),
        t.target_table,
      ),
    });
  }
  out
}

/// Walk every ankh entry, group by the value of
/// `supplied_parameters["candidate_import_spec"]`, and emit a
/// `RecurringImportSpec` proposal for each value that appears in
/// ≥ N entries across different target_paths.
fn propose_recurring_import_spec(
  entries: &[(AnkhRetrievalKey, AnkhEntry)],
) -> Vec<CandidateRowProposal> {
  let min = minimum_for(CandidateKind::RecurringImportSpec);
  let target = target_for(CandidateKind::RecurringImportSpec);
  // (import_spec_value, language) → set of distinct target_paths
  let mut buckets: BTreeMap<(String, String), Vec<&AnkhRetrievalKey>> = BTreeMap::new();
  for (k, e) in entries {
    if !can_lower_into_semantic_row(e) {
      continue;
    }
    let Some(spec) = e.supplied_parameters.get("candidate_import_spec") else {
      continue;
    };
    buckets
      .entry((spec.clone(), k.language.clone()))
      .or_default()
      .push(k);
  }
  let mut out = Vec::new();
  for ((spec, language), keys) in buckets {
    // Count distinct target_paths — same spec across N files is
    // the signal, not the same spec for the same file repeated.
    let mut distinct_paths: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for k in &keys {
      distinct_paths.insert(k.target_path.as_str());
    }
    if distinct_paths.len() < min {
      continue;
    }
    let supporting: Vec<String> = keys.iter().map(|k| fingerprint(k)).collect();
    let mut row = BTreeMap::new();
    row.insert("import_spec".to_string(), spec.clone());
    row.insert("language".to_string(), language.clone());
    row.insert(
      "distinct_target_paths".to_string(),
      distinct_paths.len().to_string(),
    );
    let Some(t) = target else { continue };
    out.push(CandidateRowProposal {
      candidate_kind: CandidateKind::RecurringImportSpec,
      target_owner: t.target_owner.to_string(),
      target_table: t.target_table.to_string(),
      proposed_row: row,
      supporting_evidence: supporting,
      evidence_count: keys.len(),
      gate_status: GateStatus::IntentReceiptOnly,
      reason: format!(
        "import spec `{spec}` for `{language}` recurred across {} distinct files; consider promoting as a known-import default or fact cue",
        distinct_paths.len(),
      ),
    });
  }
  out
}

/// Walk every ankh entry from the math-expression-lower lane (those
/// with `query_kind == "lookup-algebraic-equivalent"`) and emit a
/// `MathExpressionLower` proposal for each `(canonical_form,
/// equivalent_form)` pair that appears in ≥ N entries.
///
/// Substrate-sharing proof: this function reads from the *same*
/// ankh store as `propose_recurring_channel_success` /
/// `propose_recurring_import_spec`, runs through the *same* downstream
/// 5-gate firewall (macro-fold / axis-separation / regression-proof
/// / owner-law / hot-reload), but produces math-domain proposals.
/// One substrate, two domains.
fn propose_math_expression_lower(
  entries: &[(AnkhRetrievalKey, AnkhEntry)],
) -> Vec<CandidateRowProposal> {
  let min = minimum_for(CandidateKind::MathExpressionLower);
  let target = target_for(CandidateKind::MathExpressionLower);
  // (canonical_form, equivalent_form, language) → contributing keys
  //
  // `canonical_form` comes from the entry's `context_snapshot` (it
  // was already known *before* retrieval — lifted from the
  // utterance). `equivalent_form` comes from `supplied_parameters`
  // (it's what retrieval *recovered*). This split is the math-lane
  // shape: pnix asks "what's the equivalent of X?" knowing X, and
  // the answer fills equivalent_form.
  let mut buckets: BTreeMap<(String, String, String), Vec<&AnkhRetrievalKey>> = BTreeMap::new();
  for (k, e) in entries {
    if !can_lower_into_semantic_row(e) {
      continue;
    }
    if e.query_kind != "lookup-algebraic-equivalent" {
      continue;
    }
    let Some(canonical) = e.context_snapshot.get("canonical_form") else {
      continue;
    };
    let Some(equivalent) = e.supplied_parameters.get("equivalent_form") else {
      continue;
    };
    buckets
      .entry((canonical.clone(), equivalent.clone(), k.language.clone()))
      .or_default()
      .push(k);
  }
  let mut out = Vec::new();
  for ((canonical, equivalent, language), keys) in buckets {
    if keys.len() < min {
      continue;
    }
    let supporting: Vec<String> = keys.iter().map(|k| fingerprint(k)).collect();
    let mut row = BTreeMap::new();
    row.insert("canonical_form".to_string(), canonical.clone());
    row.insert("equivalent_form".to_string(), equivalent.clone());
    row.insert("language".to_string(), language.clone());
    let Some(t) = target else { continue };
    out.push(CandidateRowProposal {
      candidate_kind: CandidateKind::MathExpressionLower,
      target_owner: t.target_owner.to_string(),
      target_table: t.target_table.to_string(),
      proposed_row: row,
      supporting_evidence: supporting,
      evidence_count: keys.len(),
      gate_status: GateStatus::IntentReceiptOnly,
      reason: format!(
        "algebraic identity `{canonical}` ↔ `{equivalent}` (lang `{language}`) recurred across {} contexts; consider registering as a known identity",
        keys.len(),
      ),
    });
  }
  out
}

/// Walk every ankh entry from the chemistry lane (those with
/// `query_kind == "lookup-chemical-reaction"`) and emit a
/// `ChemicalReactionLower` proposal for each
/// `(reactants, products, conditions, language)` triple that
/// appears in ≥ N entries.
///
/// **Substrate-sharing N=3 proof:** this function reads from the
/// same ankh store as the coding-lane and math-lane proposers, runs
/// through the same downstream 5-gate firewall, but produces
/// chemistry-domain proposals. One substrate, three domains.
fn propose_chemical_reaction_lower(
  entries: &[(AnkhRetrievalKey, AnkhEntry)],
) -> Vec<CandidateRowProposal> {
  let min = minimum_for(CandidateKind::ChemicalReactionLower);
  let target = target_for(CandidateKind::ChemicalReactionLower);
  let mut buckets: BTreeMap<(String, String, String, String), Vec<&AnkhRetrievalKey>> =
    BTreeMap::new();
  for (k, e) in entries {
    if !can_lower_into_semantic_row(e) {
      continue;
    }
    if e.query_kind != "lookup-chemical-reaction" {
      continue;
    }
    let Some(reactants) = e.context_snapshot.get("reactants") else {
      continue;
    };
    let Some(conditions) = e.context_snapshot.get("conditions") else {
      continue;
    };
    let Some(products) = e.supplied_parameters.get("products") else {
      continue;
    };
    buckets
      .entry((
        reactants.clone(),
        products.clone(),
        conditions.clone(),
        k.language.clone(),
      ))
      .or_default()
      .push(k);
  }
  let mut out = Vec::new();
  for ((reactants, products, conditions, language), keys) in buckets {
    if keys.len() < min {
      continue;
    }
    let supporting: Vec<String> = keys.iter().map(|k| fingerprint(k)).collect();
    let mut row = BTreeMap::new();
    row.insert("reactants".to_string(), reactants.clone());
    row.insert("products".to_string(), products.clone());
    row.insert("conditions".to_string(), conditions.clone());
    row.insert("language".to_string(), language.clone());
    let Some(t) = target else { continue };
    out.push(CandidateRowProposal {
      candidate_kind: CandidateKind::ChemicalReactionLower,
      target_owner: t.target_owner.to_string(),
      target_table: t.target_table.to_string(),
      proposed_row: row,
      supporting_evidence: supporting,
      evidence_count: keys.len(),
      gate_status: GateStatus::IntentReceiptOnly,
      reason: format!(
        "chemical reaction `{reactants}` → `{products}` (cond `{conditions}`, lang `{language}`) recurred across {} contexts",
        keys.len(),
      ),
    });
  }
  out
}

/// Lower explicit operator-followup ankh evidence into
/// `LearnedIntentSignal` proposals.
///
/// This is narrower than autonomous row invention: the ankh entry
/// must already contain `cue`, `intent`, and `weight` as
/// operator-supplied fields, and external paper-note provenance is
/// refused. Gates 2-5 still own syntax, target schema/value checks,
/// regression proof, and approval before any overlay file changes.
fn propose_explicit_learned_overlay_rows_from_ankh(
  entries: &[(AnkhRetrievalKey, AnkhEntry)],
  kind: CandidateKind,
  query_kinds: &[&str],
  allowed_provenance: &[AnkhProvenanceSource],
  required_fields: &[&str],
) -> Vec<CandidateRowProposal> {
  let min = minimum_for(kind);
  let target = target_for(kind);
  let Some(t) = target else { return Vec::new() };

  // Same explicit row can be supplied from multiple follow-up
  // contexts; merge those into one proposal with multiple evidence
  // fingerprints. Conflicting rows remain distinct and are left for
  // Gate 4 / operator review.
  let mut buckets: BTreeMap<BTreeMap<String, String>, Vec<&AnkhRetrievalKey>> = BTreeMap::new();
  for (k, e) in entries {
    if !can_lower_explicit_learned_overlay_row(e, query_kinds, allowed_provenance, required_fields)
    {
      continue;
    }
    let mut row = BTreeMap::new();
    for field in required_fields {
      row.insert(
        (*field).to_string(),
        e.supplied_parameters
          .get(*field)
          .expect("checked required field")
          .clone(),
      );
    }
    buckets.entry(row).or_default().push(k);
  }

  let mut out = Vec::new();
  for (row, keys) in buckets {
    if keys.len() < min {
      continue;
    }
    let supporting: Vec<String> = keys.iter().map(|k| fingerprint(k)).collect();
    out.push(CandidateRowProposal {
      candidate_kind: kind,
      target_owner: t.target_owner.to_string(),
      target_table: t.target_table.to_string(),
      proposed_row: row,
      supporting_evidence: supporting,
      evidence_count: keys.len(),
      gate_status: GateStatus::IntentReceiptOnly,
      reason: format!(
        "operator-followup ankh evidence supplied `{}` row targeting `{}`",
        kind.as_str(),
        t.target_table,
      ),
    });
  }
  out
}

fn propose_learned_intent_signal_from_ankh(
  entries: &[(AnkhRetrievalKey, AnkhEntry)],
) -> Vec<CandidateRowProposal> {
  propose_explicit_learned_overlay_rows_from_ankh(
    entries,
    CandidateKind::LearnedIntentSignal,
    LEARNED_INTENT_SIGNAL_ANKH_QUERY_KINDS,
    LEARNED_INTENT_SIGNAL_ANKH_ALLOWED_PROVENANCE,
    LEARNED_INTENT_SIGNAL_ANKH_REQUIRED_SUPPLIED_PARAMETERS,
  )
}

fn propose_learned_operation_map_from_ankh(
  entries: &[(AnkhRetrievalKey, AnkhEntry)],
) -> Vec<CandidateRowProposal> {
  propose_explicit_learned_overlay_rows_from_ankh(
    entries,
    CandidateKind::LearnedOperationMap,
    LEARNED_OPERATION_MAP_ANKH_QUERY_KINDS,
    LEARNED_OPERATION_MAP_ANKH_ALLOWED_PROVENANCE,
    LEARNED_OPERATION_MAP_ANKH_REQUIRED_SUPPLIED_PARAMETERS,
  )
}

fn propose_learned_parameter_resolution_from_ankh(
  entries: &[(AnkhRetrievalKey, AnkhEntry)],
) -> Vec<CandidateRowProposal> {
  propose_explicit_learned_overlay_rows_from_ankh(
    entries,
    CandidateKind::LearnedParameterResolution,
    LEARNED_PARAMETER_RESOLUTION_ANKH_QUERY_KINDS,
    LEARNED_PARAMETER_RESOLUTION_ANKH_ALLOWED_PROVENANCE,
    LEARNED_PARAMETER_RESOLUTION_ANKH_REQUIRED_SUPPLIED_PARAMETERS,
  )
}

fn propose_learned_fact_cue_phrase_pattern_from_ankh(
  entries: &[(AnkhRetrievalKey, AnkhEntry)],
) -> Vec<CandidateRowProposal> {
  propose_explicit_learned_overlay_rows_from_ankh(
    entries,
    CandidateKind::LearnedFactCuePhrasePattern,
    LEARNED_FACT_CUE_PHRASE_PATTERN_ANKH_QUERY_KINDS,
    LEARNED_FACT_CUE_PHRASE_PATTERN_ANKH_ALLOWED_PROVENANCE,
    LEARNED_FACT_CUE_PHRASE_PATTERN_ANKH_REQUIRED_SUPPLIED_PARAMETERS,
  )
}

/// Main entry. Walks ankh, runs every candidate-kind proposer, and
/// returns the union of typed proposals. Result order: by kind
/// (`CandidateKind::ALL` order), within kind by candidate-content
/// ordering. Empty when no patterns reach their thresholds.
pub fn propose_candidates_from_ankh<S: AnkhStore>(ankh: &S) -> Vec<CandidateRowProposal> {
  let entries = ankh.iter_entries();
  let mut out = Vec::new();
  out.extend(propose_recurring_channel_success(&entries));
  out.extend(propose_recurring_import_spec(&entries));
  out.extend(propose_math_expression_lower(&entries));
  out.extend(propose_chemical_reaction_lower(&entries));
  out.extend(propose_learned_intent_signal_from_ankh(&entries));
  out.extend(propose_learned_operation_map_from_ankh(&entries));
  out.extend(propose_learned_parameter_resolution_from_ankh(&entries));
  out.extend(propose_learned_fact_cue_phrase_pattern_from_ankh(&entries));
  out
}

/// Render a `CandidateRowProposal` as the canonical JSON payload of
/// a `coding.candidate-row-proposal` artifact. Stage D-1
/// (observer / Gate 1) output — the operator-facing surface for
/// reviewing what pnix is *about to propose* before the firewall
/// gates run.
///
/// Replay-stable id = SHA-256 of intrinsic identity (candidate_kind
/// + target_owner + target_table + sorted proposed_row key/value
/// pairs + sorted supporting_evidence). `stored_at_ms` is
/// extrinsic.
///
/// Content policy: every field is observer metadata or row
/// key-value data (caller-injected). No source bodies — customer-
/// release safe.
pub fn build_candidate_row_proposal_artifact(
  proposal: &CandidateRowProposal,
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
) -> serde_json::Value {
  let mut h = Sha256::new();
  h.update(b"candidate-row-proposal\x1f");
  h.update(proposal.candidate_kind.as_str().as_bytes());
  h.update(b"\x1f");
  h.update(proposal.target_owner.as_bytes());
  h.update(b"\x1e");
  h.update(proposal.target_table.as_bytes());
  h.update(b"\x1f");
  let mut row_keys: Vec<&String> = proposal.proposed_row.keys().collect();
  row_keys.sort();
  for k in row_keys {
    h.update(k.as_bytes());
    h.update(b"\x1d");
    h.update(proposal.proposed_row[k].as_bytes());
    h.update(b"\x1e");
  }
  h.update(b"\x1f");
  let mut sorted_evidence = proposal.supporting_evidence.clone();
  sorted_evidence.sort();
  for e in &sorted_evidence {
    h.update(e.as_bytes());
    h.update(b"\x1e");
  }
  let digest = h.finalize();
  let prefix = digest
    .iter()
    .take(16)
    .map(|b| format!("{b:02x}"))
    .collect::<String>();
  let id = format!("candidate-row-proposal.{prefix}");

  let mut payload = serde_json::json!({
    "id": id,
    "artifact_family": "coding.candidate-row-proposal",
    "source_surface": "algorithm-synthesis.candidate-row-proposal",
    "stored_at_ms": stored_at_ms,
    "candidate_kind": proposal.candidate_kind.as_str(),
    "gate_status": proposal.gate_status.as_str(),
    "target_owner": proposal.target_owner,
    "target_table": proposal.target_table,
    "evidence_count": proposal.evidence_count,
    "proposed_row": proposal.proposed_row,
    "supporting_evidence": proposal.supporting_evidence,
    "reason": proposal.reason,
    "related_refs": serde_json::json!([
      format!("candidate-kind:{}", proposal.candidate_kind.as_str()),
      format!("target-owner:{}", proposal.target_owner),
      format!("target-table:{}", proposal.target_table),
      "owner-law:stdlib/lib/gate/algorithm-synthesis/candidate-row-proposal.px",
    ]),
    "target_paths": serde_json::json!([proposal.target_owner]),
    "command_refs": Vec::<String>::new(),
  });
  if let Some(snap) = repo_snapshot_ref {
    payload["repo_snapshot_ref"] = serde_json::Value::String(snap.to_string());
  }
  payload
}

#[cfg(test)]
mod tests {
  use super::super::ankh_retrieval_cache::{AnkhEntry, AnkhRetrievalKey, InMemoryAnkhStore};
  use super::*;

  fn seed_entry(
    ankh: &mut InMemoryAnkhStore,
    query_kind: &str,
    target_path: &str,
    language: &str,
    provenance: AnkhProvenanceSource,
    supplied_params: &[(&str, &str)],
  ) {
    use super::super::ankh_retrieval_cache::AnkhStore;
    let key = AnkhRetrievalKey {
      query_kind: query_kind.to_string(),
      target_path: target_path.to_string(),
      language: language.to_string(),
    };
    let entry = AnkhEntry {
      provenance_source: provenance,
      contributing_actor_id: "actor.seed".to_string(),
      contributing_tenant_id: "tenant.seed".to_string(),
      stored_at_ms: 1700000000000,
      query_kind: query_kind.to_string(),
      supplied_parameters: supplied_params
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect(),
      filled_slots: vec![],
      context_snapshot: BTreeMap::new(),
    };
    ankh.put(key, entry);
  }

  // ─── registry consistency ──────────────────────────────────────

  #[test]
  fn every_candidate_kind_has_a_threshold_row() {
    for k in CandidateKind::ALL {
      assert!(
        MINIMUM_EVIDENCE_COUNTS.iter().any(|r| &r.kind == k),
        "candidate kind `{}` has no threshold row",
        k.as_str()
      );
    }
  }

  #[test]
  fn every_candidate_kind_has_a_target_owner_row() {
    for k in CandidateKind::ALL {
      assert!(
        CANDIDATE_TARGET_OWNERS.iter().any(|r| &r.kind == k),
        "candidate kind `{}` has no target owner row",
        k.as_str()
      );
    }
  }

  // ─── empty / below-threshold ──────────────────────────────────

  #[test]
  fn empty_ankh_yields_no_proposals() {
    let ankh = InMemoryAnkhStore::new();
    let proposals = propose_candidates_from_ankh(&ankh);
    assert!(proposals.is_empty());
  }

  #[test]
  fn single_entry_does_not_meet_threshold() {
    let mut ankh = InMemoryAnkhStore::new();
    seed_entry(
      &mut ankh,
      "lookup-module-providing-symbol",
      "src/a.py",
      "python",
      AnkhProvenanceSource::HostSymbolResolver,
      &[("candidate_import_spec", "import os")],
    );
    assert!(propose_candidates_from_ankh(&ankh).is_empty());
  }

  // ─── recurring-channel-success ────────────────────────────────

  #[test]
  fn two_entries_same_query_kind_same_provenance_proposes_channel_success() {
    let mut ankh = InMemoryAnkhStore::new();
    seed_entry(
      &mut ankh,
      "lookup-module-providing-symbol",
      "src/a.py",
      "python",
      AnkhProvenanceSource::HostSymbolResolver,
      &[("candidate_import_spec", "import os")],
    );
    seed_entry(
      &mut ankh,
      "lookup-module-providing-symbol",
      "src/b.py",
      "python",
      AnkhProvenanceSource::HostSymbolResolver,
      &[("candidate_import_spec", "import sys")],
    );
    let proposals = propose_candidates_from_ankh(&ankh);
    let channel_proposals: Vec<&CandidateRowProposal> = proposals
      .iter()
      .filter(|p| p.candidate_kind == CandidateKind::RecurringChannelSuccess)
      .collect();
    assert_eq!(channel_proposals.len(), 1);
    let p = channel_proposals[0];
    assert_eq!(p.evidence_count, 2);
    assert_eq!(p.gate_status, GateStatus::IntentReceiptOnly);
    assert!(p.target_owner.contains("held-to-query.px"));
    assert_eq!(p.target_table, "heldRoutingMap");
    assert_eq!(
      p.proposed_row.get("observed_primary_channel").unwrap(),
      "host-symbol-resolver"
    );
  }

  #[test]
  fn different_provenance_does_not_merge_into_one_proposal() {
    let mut ankh = InMemoryAnkhStore::new();
    seed_entry(
      &mut ankh,
      "lookup-module-providing-symbol",
      "src/a.py",
      "python",
      AnkhProvenanceSource::HostSymbolResolver,
      &[("candidate_import_spec", "import os")],
    );
    seed_entry(
      &mut ankh,
      "lookup-module-providing-symbol",
      "src/b.py",
      "python",
      AnkhProvenanceSource::OperatorFollowup,
      &[("candidate_import_spec", "import sys")],
    );
    // Each provenance has only 1 entry → neither meets the
    // threshold → no channel proposal.
    let proposals = propose_candidates_from_ankh(&ankh);
    let channel_proposals: Vec<&CandidateRowProposal> = proposals
      .iter()
      .filter(|p| p.candidate_kind == CandidateKind::RecurringChannelSuccess)
      .collect();
    assert!(channel_proposals.is_empty());
  }

  // ─── recurring-import-spec ────────────────────────────────────

  #[test]
  fn same_import_spec_across_two_files_proposes_recurring_import() {
    let mut ankh = InMemoryAnkhStore::new();
    seed_entry(
      &mut ankh,
      "lookup-module-providing-symbol",
      "src/a.py",
      "python",
      AnkhProvenanceSource::HostSymbolResolver,
      &[("candidate_import_spec", "import os")],
    );
    seed_entry(
      &mut ankh,
      "lookup-module-providing-symbol",
      "src/b.py",
      "python",
      AnkhProvenanceSource::HostSymbolResolver,
      &[("candidate_import_spec", "import os")],
    );
    let proposals = propose_candidates_from_ankh(&ankh);
    let import_proposals: Vec<&CandidateRowProposal> = proposals
      .iter()
      .filter(|p| p.candidate_kind == CandidateKind::RecurringImportSpec)
      .collect();
    assert_eq!(import_proposals.len(), 1);
    let p = import_proposals[0];
    assert_eq!(p.proposed_row.get("import_spec").unwrap(), "import os");
    assert_eq!(p.proposed_row.get("language").unwrap(), "python");
    assert_eq!(p.proposed_row.get("distinct_target_paths").unwrap(), "2");
    assert!(p.target_owner.contains("known-imports-by-language.px"));
  }

  #[test]
  fn external_import_specs_do_not_lower_directly_to_static_import_rows() {
    let mut ankh = InMemoryAnkhStore::new();
    seed_entry(
      &mut ankh,
      "lookup-module-providing-symbol",
      "https://example.test/a",
      "python",
      AnkhProvenanceSource::ExternalKnowledgeSearch,
      &[("candidate_import_spec", "import os")],
    );
    seed_entry(
      &mut ankh,
      "lookup-module-providing-symbol",
      "https://example.test/b",
      "python",
      AnkhProvenanceSource::ExternalKnowledgeSearch,
      &[("candidate_import_spec", "import os")],
    );

    let proposals = propose_candidates_from_ankh(&ankh);
    assert!(
      proposals
        .iter()
        .all(|p| p.candidate_kind != CandidateKind::RecurringImportSpec),
      "external paper notes must not synthesize static import rows: {proposals:#?}"
    );
    assert!(
      proposals.iter().any(|p| {
        p.candidate_kind == CandidateKind::RecurringChannelSuccess
          && p
            .proposed_row
            .get("observed_primary_channel")
            .map(|s| s.as_str())
            == Some("external-knowledge-search")
      }),
      "external recurrence is still observable as channel evidence"
    );
  }

  #[test]
  fn same_import_spec_same_file_repeated_does_not_propose() {
    // Same file, same spec — distinct_target_paths == 1 < threshold.
    let mut ankh = InMemoryAnkhStore::new();
    seed_entry(
      &mut ankh,
      "lookup-module-providing-symbol",
      "src/a.py",
      "python",
      AnkhProvenanceSource::HostSymbolResolver,
      &[("candidate_import_spec", "import os")],
    );
    // Same key (overwrites), but iter_entries reflects current map.
    // Adding a second entry with same key just overwrites — count
    // stays at 1.
    let proposals = propose_candidates_from_ankh(&ankh);
    let import_proposals: Vec<&CandidateRowProposal> = proposals
      .iter()
      .filter(|p| p.candidate_kind == CandidateKind::RecurringImportSpec)
      .collect();
    assert!(import_proposals.is_empty());
  }

  // ─── audit trail ──────────────────────────────────────────────

  #[test]
  fn proposal_carries_supporting_evidence_fingerprints() {
    let mut ankh = InMemoryAnkhStore::new();
    seed_entry(
      &mut ankh,
      "lookup-module-providing-symbol",
      "src/a.py",
      "python",
      AnkhProvenanceSource::HostSymbolResolver,
      &[("candidate_import_spec", "import os")],
    );
    seed_entry(
      &mut ankh,
      "lookup-module-providing-symbol",
      "src/b.py",
      "python",
      AnkhProvenanceSource::HostSymbolResolver,
      &[("candidate_import_spec", "import os")],
    );
    let proposals = propose_candidates_from_ankh(&ankh);
    let p = &proposals[0];
    assert_eq!(p.supporting_evidence.len(), 2);
    for fp in &p.supporting_evidence {
      assert!(fp.contains("lookup-module-providing-symbol"));
      assert!(fp.contains("python"));
    }
  }

  #[test]
  fn proposal_gate_status_is_always_intent_receipt_only_in_v0() {
    let mut ankh = InMemoryAnkhStore::new();
    seed_entry(
      &mut ankh,
      "lookup-module-providing-symbol",
      "src/a.py",
      "python",
      AnkhProvenanceSource::HostSymbolResolver,
      &[("candidate_import_spec", "import os")],
    );
    seed_entry(
      &mut ankh,
      "lookup-module-providing-symbol",
      "src/b.py",
      "python",
      AnkhProvenanceSource::HostSymbolResolver,
      &[("candidate_import_spec", "import os")],
    );
    for p in propose_candidates_from_ankh(&ankh) {
      assert_eq!(p.gate_status, GateStatus::IntentReceiptOnly);
    }
  }

  // ─── learned-intent signal from operator-followup ankh ───────

  fn seed_learned_intent_signal_with_provenance(
    ankh: &mut InMemoryAnkhStore,
    target_path: &str,
    cue: &str,
    intent: &str,
    weight: &str,
    provenance: AnkhProvenanceSource,
  ) {
    use super::super::ankh_retrieval_cache::AnkhStore;
    let query_kind = LEARNED_INTENT_SIGNAL_ANKH_QUERY_KINDS[0];
    let key = AnkhRetrievalKey {
      query_kind: query_kind.to_string(),
      target_path: target_path.to_string(),
      language: "pnix".to_string(),
    };
    let mut supplied = BTreeMap::new();
    supplied.insert("cue".to_string(), cue.to_string());
    supplied.insert("intent".to_string(), intent.to_string());
    supplied.insert("weight".to_string(), weight.to_string());
    let entry = AnkhEntry {
      provenance_source: provenance,
      contributing_actor_id: "actor.operator".to_string(),
      contributing_tenant_id: "tenant.default".to_string(),
      stored_at_ms: 1700000000000,
      query_kind: query_kind.to_string(),
      supplied_parameters: supplied,
      filled_slots: LEARNED_INTENT_SIGNAL_ANKH_REQUIRED_SUPPLIED_PARAMETERS
        .iter()
        .map(|s| s.to_string())
        .collect(),
      context_snapshot: BTreeMap::new(),
    };
    ankh.put(key, entry);
  }

  fn seed_explicit_learned_overlay_row_with_provenance(
    ankh: &mut InMemoryAnkhStore,
    query_kind: &str,
    target_path: &str,
    supplied_params: &[(&str, &str)],
    provenance: AnkhProvenanceSource,
  ) {
    use super::super::ankh_retrieval_cache::AnkhStore;
    let key = AnkhRetrievalKey {
      query_kind: query_kind.to_string(),
      target_path: target_path.to_string(),
      language: "pnix".to_string(),
    };
    let entry = AnkhEntry {
      provenance_source: provenance,
      contributing_actor_id: "actor.operator".to_string(),
      contributing_tenant_id: "tenant.default".to_string(),
      stored_at_ms: 1700000000000,
      query_kind: query_kind.to_string(),
      supplied_parameters: supplied_params
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect(),
      filled_slots: supplied_params.iter().map(|(k, _)| k.to_string()).collect(),
      context_snapshot: BTreeMap::new(),
    };
    ankh.put(key, entry);
  }

  #[test]
  fn operator_followup_ankh_entry_proposes_learned_intent_signal() {
    let mut ankh = InMemoryAnkhStore::new();
    seed_learned_intent_signal_with_provenance(
      &mut ankh,
      "turns/held-001",
      "fact:operator-confirmed-refactor",
      "refactor",
      "0.91",
      AnkhProvenanceSource::OperatorFollowup,
    );

    let proposals = propose_candidates_from_ankh(&ankh);
    let learned: Vec<&CandidateRowProposal> = proposals
      .iter()
      .filter(|p| p.candidate_kind == CandidateKind::LearnedIntentSignal)
      .collect();
    assert_eq!(learned.len(), 1);
    let p = learned[0];
    assert_eq!(p.evidence_count, 1);
    assert_eq!(p.target_table, "overlayIntentSignals");
    assert_eq!(
      p.target_owner,
      "stdlib/lib/gate/algorithm-synthesis/learned-intent-overlay.px"
    );
    assert_eq!(
      p.proposed_row.get("cue").map(String::as_str),
      Some("fact:operator-confirmed-refactor")
    );
    assert_eq!(
      p.proposed_row.get("intent").map(String::as_str),
      Some("refactor")
    );
    assert_eq!(
      p.proposed_row.get("weight").map(String::as_str),
      Some("0.91")
    );
    assert!(
      p.reason.contains("operator-followup ankh evidence"),
      "reason should expose that this is evidence lowering, not direct row injection: {}",
      p.reason
    );
  }

  #[test]
  fn external_paper_note_does_not_propose_learned_intent_signal() {
    let mut ankh = InMemoryAnkhStore::new();
    seed_learned_intent_signal_with_provenance(
      &mut ankh,
      "https://example.test/intent-a",
      "fact:web-claimed-refactor",
      "refactor",
      "0.91",
      AnkhProvenanceSource::ExternalKnowledgeSearch,
    );
    seed_learned_intent_signal_with_provenance(
      &mut ankh,
      "https://example.test/intent-b",
      "fact:web-claimed-refactor",
      "refactor",
      "0.91",
      AnkhProvenanceSource::ExternalKnowledgeSearch,
    );

    let proposals = propose_candidates_from_ankh(&ankh);
    assert!(
      proposals
        .iter()
        .all(|p| p.candidate_kind != CandidateKind::LearnedIntentSignal),
      "external paper notes must not lower directly into learned overlay rows: {proposals:#?}"
    );
  }

  #[test]
  fn operator_followup_ankh_entries_propose_all_learned_overlay_surfaces() {
    let mut ankh = InMemoryAnkhStore::new();
    seed_learned_intent_signal_with_provenance(
      &mut ankh,
      "turns/all-surface-intent",
      "fact:operator-all-surface",
      "refactor",
      "0.91",
      AnkhProvenanceSource::OperatorFollowup,
    );
    seed_explicit_learned_overlay_row_with_provenance(
      &mut ankh,
      LEARNED_OPERATION_MAP_ANKH_QUERY_KINDS[0],
      "turns/all-surface-operation",
      &[
        ("intent", "refactor"),
        ("cues", "fact:operator-all-surface"),
        ("transform", "rename-symbol"),
        ("weight", "0.77"),
      ],
      AnkhProvenanceSource::OperatorFollowup,
    );
    seed_explicit_learned_overlay_row_with_provenance(
      &mut ankh,
      LEARNED_PARAMETER_RESOLUTION_ANKH_QUERY_KINDS[0],
      "turns/all-surface-parameter",
      &[
        ("operation_candidate", "rename-symbol"),
        ("resolved_fields", "{\"new_name\":\"tidy_name\"}"),
      ],
      AnkhProvenanceSource::OperatorFollowup,
    );
    seed_explicit_learned_overlay_row_with_provenance(
      &mut ankh,
      LEARNED_FACT_CUE_PHRASE_PATTERN_ANKH_QUERY_KINDS[0],
      "turns/all-surface-fact-cue",
      &[
        ("cue", "fact:operator-all-surface"),
        ("markers", "다듬,tidy"),
      ],
      AnkhProvenanceSource::OperatorFollowup,
    );

    let proposals = propose_candidates_from_ankh(&ankh);
    let kinds: Vec<CandidateKind> = proposals.iter().map(|p| p.candidate_kind).collect();
    for kind in [
      CandidateKind::LearnedIntentSignal,
      CandidateKind::LearnedOperationMap,
      CandidateKind::LearnedParameterResolution,
      CandidateKind::LearnedFactCuePhrasePattern,
    ] {
      assert!(
        kinds.contains(&kind),
        "operator follow-up evidence should propose {kind:?}; got {proposals:#?}"
      );
    }
  }

  #[test]
  fn external_paper_notes_do_not_propose_any_learned_overlay_surface() {
    let mut ankh = InMemoryAnkhStore::new();
    seed_learned_intent_signal_with_provenance(
      &mut ankh,
      "https://example.test/all-surface-intent",
      "fact:web-all-surface",
      "refactor",
      "0.91",
      AnkhProvenanceSource::ExternalKnowledgeSearch,
    );
    seed_explicit_learned_overlay_row_with_provenance(
      &mut ankh,
      LEARNED_OPERATION_MAP_ANKH_QUERY_KINDS[0],
      "https://example.test/all-surface-operation",
      &[
        ("intent", "refactor"),
        ("cues", "fact:web-all-surface"),
        ("transform", "rename-symbol"),
        ("weight", "0.77"),
      ],
      AnkhProvenanceSource::ExternalKnowledgeSearch,
    );
    seed_explicit_learned_overlay_row_with_provenance(
      &mut ankh,
      LEARNED_PARAMETER_RESOLUTION_ANKH_QUERY_KINDS[0],
      "https://example.test/all-surface-parameter",
      &[
        ("operation_candidate", "rename-symbol"),
        ("resolved_fields", "{\"new_name\":\"tidy_name\"}"),
      ],
      AnkhProvenanceSource::ExternalKnowledgeSearch,
    );
    seed_explicit_learned_overlay_row_with_provenance(
      &mut ankh,
      LEARNED_FACT_CUE_PHRASE_PATTERN_ANKH_QUERY_KINDS[0],
      "https://example.test/all-surface-fact-cue",
      &[("cue", "fact:web-all-surface"), ("markers", "tidy")],
      AnkhProvenanceSource::ExternalKnowledgeSearch,
    );

    let proposals = propose_candidates_from_ankh(&ankh);
    for forbidden in [
      CandidateKind::LearnedIntentSignal,
      CandidateKind::LearnedOperationMap,
      CandidateKind::LearnedParameterResolution,
      CandidateKind::LearnedFactCuePhrasePattern,
    ] {
      assert!(
        proposals.iter().all(|p| p.candidate_kind != forbidden),
        "external paper notes must not lower directly into {forbidden:?}: {proposals:#?}"
      );
    }
  }

  #[test]
  fn ankh_learned_intent_proposal_walks_gate2_and_gate3() {
    let mut ankh = InMemoryAnkhStore::new();
    seed_learned_intent_signal_with_provenance(
      &mut ankh,
      "turns/held-002",
      "fact:operator-confirmed-explain",
      "explain",
      "0.8",
      AnkhProvenanceSource::OperatorFollowup,
    );
    let proposal = propose_candidates_from_ankh(&ankh)
      .into_iter()
      .find(|p| p.candidate_kind == CandidateKind::LearnedIntentSignal)
      .expect("learned intent proposal");
    let folded = super::super::macro_fold_gate::fold_proposal(&proposal);
    assert_eq!(
      folded.outcome,
      super::super::macro_fold_gate::MacroFoldOutcome::Folded
    );
    assert!(
      folded.folded_ast_json.is_some(),
      "Gate 2 must produce first-class AST evidence"
    );
    let separated = super::super::axis_separation_gate::check_axis_separation(&folded);
    assert_eq!(
      separated.outcome,
      super::super::axis_separation_gate::AxisSeparationOutcome::AxisVerified
    );
  }

  #[test]
  fn invalid_ankh_learned_intent_weight_reaches_gate3_hold() {
    let mut ankh = InMemoryAnkhStore::new();
    seed_learned_intent_signal_with_provenance(
      &mut ankh,
      "turns/held-003",
      "fact:operator-confirmed-test",
      "test",
      "heavy",
      AnkhProvenanceSource::OperatorFollowup,
    );
    let proposal = propose_candidates_from_ankh(&ankh)
      .into_iter()
      .find(|p| p.candidate_kind == CandidateKind::LearnedIntentSignal)
      .expect("learned intent proposal");
    let folded = super::super::macro_fold_gate::fold_proposal(&proposal);
    let separated = super::super::axis_separation_gate::check_axis_separation(&folded);
    assert_eq!(
      separated.outcome,
      super::super::axis_separation_gate::AxisSeparationOutcome::HeldInvalidFieldValue
    );
    assert_eq!(separated.invalid_field_values.len(), 1);
    assert_eq!(separated.invalid_field_values[0].field, "weight");
    assert_eq!(separated.invalid_field_values[0].value, "heavy");
  }

  // ─── candidate-row-proposal artifact (Stage D-1 panel) ───────

  fn seed_two_channel_success(ankh: &mut InMemoryAnkhStore) {
    seed_entry(
      ankh,
      "lookup-module-providing-symbol",
      "src/a.py",
      "python",
      AnkhProvenanceSource::HostSymbolResolver,
      &[("candidate_import_spec", "import os")],
    );
    seed_entry(
      ankh,
      "lookup-module-providing-symbol",
      "src/b.py",
      "python",
      AnkhProvenanceSource::HostSymbolResolver,
      &[("candidate_import_spec", "import os")],
    );
  }

  fn first_proposal(ankh: &InMemoryAnkhStore) -> CandidateRowProposal {
    propose_candidates_from_ankh(ankh)
      .into_iter()
      .next()
      .expect("at least one proposal")
  }

  #[test]
  fn artifact_carries_candidate_kind_and_target() {
    let mut ankh = InMemoryAnkhStore::new();
    seed_two_channel_success(&mut ankh);
    let p = first_proposal(&ankh);
    let art = build_candidate_row_proposal_artifact(&p, 1700000000000, None);
    assert_eq!(art["artifact_family"], "coding.candidate-row-proposal");
    assert_eq!(art["candidate_kind"], p.candidate_kind.as_str());
    assert_eq!(art["target_owner"], p.target_owner);
    assert_eq!(art["target_table"], p.target_table);
    assert_eq!(art["evidence_count"], p.evidence_count);
    assert_eq!(art["gate_status"], "intent-receipt-only");
  }

  #[test]
  fn artifact_carries_proposed_row_kv_and_supporting_evidence() {
    let mut ankh = InMemoryAnkhStore::new();
    seed_two_channel_success(&mut ankh);
    let p = first_proposal(&ankh);
    let art = build_candidate_row_proposal_artifact(&p, 0, None);
    let row = art["proposed_row"].as_object().expect("proposed_row");
    assert!(!row.is_empty(), "proposed_row must be non-empty");
    let evidence = art["supporting_evidence"]
      .as_array()
      .expect("evidence array");
    assert_eq!(evidence.len(), p.supporting_evidence.len());
  }

  #[test]
  fn artifact_id_is_replay_stable_across_stored_at() {
    let mut ankh = InMemoryAnkhStore::new();
    seed_two_channel_success(&mut ankh);
    let p = first_proposal(&ankh);
    let a1 = build_candidate_row_proposal_artifact(&p, 1, None);
    let a2 = build_candidate_row_proposal_artifact(&p, 999999, None);
    assert_eq!(a1["id"], a2["id"], "id must ignore stored_at_ms");
  }

  #[test]
  fn artifact_id_differs_when_target_table_differs() {
    let mut ankh = InMemoryAnkhStore::new();
    seed_two_channel_success(&mut ankh);
    let p1 = first_proposal(&ankh);
    let mut p2 = p1.clone();
    p2.target_table = "differentTable".to_string();
    let a1 = build_candidate_row_proposal_artifact(&p1, 0, None);
    let a2 = build_candidate_row_proposal_artifact(&p2, 0, None);
    assert_ne!(a1["id"], a2["id"]);
  }

  #[test]
  fn artifact_related_refs_walk_back_to_kind_owner_table() {
    let mut ankh = InMemoryAnkhStore::new();
    seed_two_channel_success(&mut ankh);
    let p = first_proposal(&ankh);
    let art = build_candidate_row_proposal_artifact(&p, 0, None);
    let refs: Vec<String> = serde_json::from_value(art["related_refs"].clone()).unwrap();
    assert!(refs.iter().any(|r| r.starts_with("candidate-kind:")));
    assert!(refs.iter().any(|r| r.starts_with("target-owner:")));
    assert!(refs.iter().any(|r| r.starts_with("target-table:")));
    assert!(refs.iter().any(|r| r.contains("candidate-row-proposal.px")));
  }

  // ─── substrate-sharing: math-expression-lower lane ───────────

  fn seed_math_identity(
    ankh: &mut InMemoryAnkhStore,
    target_path: &str,
    language: &str,
    canonical: &str,
    equivalent: &str,
  ) {
    seed_math_identity_with_provenance(
      ankh,
      target_path,
      language,
      canonical,
      equivalent,
      AnkhProvenanceSource::OperatorFollowup,
    );
  }

  fn seed_math_identity_with_provenance(
    ankh: &mut InMemoryAnkhStore,
    target_path: &str,
    language: &str,
    canonical: &str,
    equivalent: &str,
    provenance: AnkhProvenanceSource,
  ) {
    use super::super::ankh_retrieval_cache::AnkhStore;
    let key = AnkhRetrievalKey {
      query_kind: "lookup-algebraic-equivalent".to_string(),
      target_path: target_path.to_string(),
      language: language.to_string(),
    };
    // Math lane split: canonical_form is *context* (lifted from
    // utterance pre-retrieval), equivalent_form is *recovered*
    // evidence. propose_math_expression_lower groups by both.
    let mut supplied = BTreeMap::new();
    supplied.insert("equivalent_form".to_string(), equivalent.to_string());
    let mut context = BTreeMap::new();
    context.insert("canonical_form".to_string(), canonical.to_string());
    context.insert("language".to_string(), language.to_string());
    let entry = AnkhEntry {
      provenance_source: provenance,
      contributing_actor_id: "actor.math-seed".to_string(),
      contributing_tenant_id: "tenant.math-seed".to_string(),
      stored_at_ms: 1700000000000,
      query_kind: "lookup-algebraic-equivalent".to_string(),
      supplied_parameters: supplied,
      filled_slots: vec!["equivalent_form".to_string()],
      context_snapshot: context,
    };
    ankh.put(key, entry);
  }

  #[test]
  fn math_kind_registered_in_all_and_target_owner_tables() {
    // Constitution: a new kind must appear in CandidateKind::ALL,
    // MINIMUM_EVIDENCE_COUNTS, and CANDIDATE_TARGET_OWNERS — else
    // it can be proposed but never lands. Sync test.
    assert!(CandidateKind::ALL.contains(&CandidateKind::MathExpressionLower));
    assert_eq!(minimum_for(CandidateKind::MathExpressionLower), 2);
    let t =
      target_for(CandidateKind::MathExpressionLower).expect("math kind must have a target row");
    assert_eq!(t.target_table, "knownAlgebraicIdentities");
    assert_eq!(
      t.target_owner,
      "stdlib/lib/gate/known-algebraic-identities.px"
    );
  }

  #[test]
  fn two_ankh_entries_same_identity_propose_math_expression_lower() {
    let mut ankh = InMemoryAnkhStore::new();
    // Two operators ask about the same identity in two different
    // surrounding contexts (different target_paths). pnix should
    // notice the recurring `(canonical, equivalent)` pair.
    seed_math_identity(
      &mut ankh,
      "math/expand-square-of-sum-a.md",
      "polynomial",
      "x^2 + 2*x*y + y^2",
      "(x+y)^2",
    );
    seed_math_identity(
      &mut ankh,
      "math/expand-square-of-sum-b.md",
      "polynomial",
      "x^2 + 2*x*y + y^2",
      "(x+y)^2",
    );
    let proposals = propose_candidates_from_ankh(&ankh);
    let math: Vec<&CandidateRowProposal> = proposals
      .iter()
      .filter(|p| p.candidate_kind == CandidateKind::MathExpressionLower)
      .collect();
    assert_eq!(math.len(), 1, "exactly one math proposal");
    let p = math[0];
    assert_eq!(p.target_table, "knownAlgebraicIdentities");
    assert_eq!(p.evidence_count, 2);
    assert_eq!(
      p.proposed_row.get("canonical_form").unwrap(),
      "x^2 + 2*x*y + y^2"
    );
    assert_eq!(p.proposed_row.get("equivalent_form").unwrap(), "(x+y)^2");
    assert_eq!(p.proposed_row.get("language").unwrap(), "polynomial");
  }

  #[test]
  fn external_math_entries_do_not_lower_directly_to_known_identities() {
    let mut ankh = InMemoryAnkhStore::new();
    seed_math_identity_with_provenance(
      &mut ankh,
      "https://example.test/math-a",
      "polynomial",
      "x^2 + 2*x*y + y^2",
      "(x+y)^2",
      AnkhProvenanceSource::ExternalKnowledgeSearch,
    );
    seed_math_identity_with_provenance(
      &mut ankh,
      "https://example.test/math-b",
      "polynomial",
      "x^2 + 2*x*y + y^2",
      "(x+y)^2",
      AnkhProvenanceSource::ExternalKnowledgeSearch,
    );

    let proposals = propose_candidates_from_ankh(&ankh);
    assert!(
      proposals
        .iter()
        .all(|p| p.candidate_kind != CandidateKind::MathExpressionLower),
      "external paper notes must not synthesize known algebraic identities: {proposals:#?}"
    );
  }

  #[test]
  fn single_math_entry_does_not_meet_threshold() {
    let mut ankh = InMemoryAnkhStore::new();
    seed_math_identity(
      &mut ankh,
      "math/x.md",
      "polynomial",
      "a^2 - b^2",
      "(a+b)*(a-b)",
    );
    let proposals = propose_candidates_from_ankh(&ankh);
    let math: Vec<_> = proposals
      .iter()
      .filter(|p| p.candidate_kind == CandidateKind::MathExpressionLower)
      .collect();
    assert!(math.is_empty(), "single evidence must not propose");
  }

  #[test]
  fn different_equivalent_forms_do_not_collapse() {
    let mut ankh = InMemoryAnkhStore::new();
    // Same canonical_form but different equivalent_forms — should
    // emerge as TWO distinct proposals, not one merged row. pnix
    // does not assume "the right answer" when operators disagree.
    seed_math_identity(
      &mut ankh,
      "math/a.md",
      "polynomial",
      "x^2 + 2*x*y + y^2",
      "(x+y)^2",
    );
    seed_math_identity(
      &mut ankh,
      "math/b.md",
      "polynomial",
      "x^2 + 2*x*y + y^2",
      "(x+y)^2",
    );
    seed_math_identity(
      &mut ankh,
      "math/c.md",
      "polynomial",
      "x^2 + 2*x*y + y^2",
      "(y+x)^2", // intentional non-canonical variant
    );
    seed_math_identity(
      &mut ankh,
      "math/d.md",
      "polynomial",
      "x^2 + 2*x*y + y^2",
      "(y+x)^2",
    );
    let proposals = propose_candidates_from_ankh(&ankh);
    let math: Vec<&CandidateRowProposal> = proposals
      .iter()
      .filter(|p| p.candidate_kind == CandidateKind::MathExpressionLower)
      .collect();
    assert_eq!(
      math.len(),
      2,
      "two distinct equivalent_forms → two proposals"
    );
  }

  #[test]
  fn different_languages_do_not_collapse() {
    // `(p ∧ q) ∨ (p ∧ r)` ↔ `p ∧ (q ∨ r)` is true in boolean-algebra
    // but the *same string form* under "polynomial" is meaningless.
    // pnix scopes by language so boolean and polynomial identities
    // never merge into one row.
    let mut ankh = InMemoryAnkhStore::new();
    seed_math_identity(
      &mut ankh,
      "math/bool-a.md",
      "boolean-algebra",
      "(p ∧ q) ∨ (p ∧ r)",
      "p ∧ (q ∨ r)",
    );
    seed_math_identity(
      &mut ankh,
      "math/bool-b.md",
      "boolean-algebra",
      "(p ∧ q) ∨ (p ∧ r)",
      "p ∧ (q ∨ r)",
    );
    seed_math_identity(
      &mut ankh,
      "math/poly-a.md",
      "polynomial",
      "x*y + x*z",
      "x*(y+z)",
    );
    seed_math_identity(
      &mut ankh,
      "math/poly-b.md",
      "polynomial",
      "x*y + x*z",
      "x*(y+z)",
    );
    let proposals = propose_candidates_from_ankh(&ankh);
    let math: Vec<&CandidateRowProposal> = proposals
      .iter()
      .filter(|p| p.candidate_kind == CandidateKind::MathExpressionLower)
      .collect();
    assert_eq!(
      math.len(),
      2,
      "two languages → two proposals (no cross-domain bleed)"
    );
  }

  fn seed_chemistry_reaction(
    ankh: &mut InMemoryAnkhStore,
    target_path: &str,
    language: &str,
    reactants: &str,
    products: &str,
    conditions: &str,
  ) {
    seed_chemistry_reaction_with_provenance(
      ankh,
      target_path,
      language,
      reactants,
      products,
      conditions,
      AnkhProvenanceSource::OperatorFollowup,
    );
  }

  fn seed_chemistry_reaction_with_provenance(
    ankh: &mut InMemoryAnkhStore,
    target_path: &str,
    language: &str,
    reactants: &str,
    products: &str,
    conditions: &str,
    provenance: AnkhProvenanceSource,
  ) {
    use super::super::ankh_retrieval_cache::AnkhStore;
    let key = AnkhRetrievalKey {
      query_kind: "lookup-chemical-reaction".to_string(),
      target_path: target_path.to_string(),
      language: language.to_string(),
    };
    let mut supplied = BTreeMap::new();
    supplied.insert("products".to_string(), products.to_string());
    let mut context = BTreeMap::new();
    context.insert("reactants".to_string(), reactants.to_string());
    context.insert("conditions".to_string(), conditions.to_string());
    context.insert("language".to_string(), language.to_string());
    let entry = AnkhEntry {
      provenance_source: provenance,
      contributing_actor_id: "actor.chem-seed".to_string(),
      contributing_tenant_id: "tenant.chem-seed".to_string(),
      stored_at_ms: 1700000000000,
      query_kind: "lookup-chemical-reaction".to_string(),
      supplied_parameters: supplied,
      filled_slots: vec!["products".to_string()],
      context_snapshot: context,
    };
    ankh.put(key, entry);
  }

  #[test]
  fn chemistry_kind_registered_in_all_threshold_and_target_owner_tables() {
    assert!(CandidateKind::ALL.contains(&CandidateKind::ChemicalReactionLower));
    assert_eq!(minimum_for(CandidateKind::ChemicalReactionLower), 2);
    let t =
      target_for(CandidateKind::ChemicalReactionLower).expect("chemistry kind must have target");
    assert_eq!(t.target_table, "knownChemicalReactions");
    assert_eq!(
      t.target_owner,
      "stdlib/lib/gate/known-chemical-reactions.px"
    );
  }

  #[test]
  fn two_chemistry_entries_same_reaction_propose_chemical_reaction_lower() {
    let mut ankh = InMemoryAnkhStore::new();
    seed_chemistry_reaction(
      &mut ankh,
      "chem/lesson-a.md",
      "inorganic",
      "2 H2 + O2",
      "2 H2O",
      "spark, 25C",
    );
    seed_chemistry_reaction(
      &mut ankh,
      "chem/lesson-b.md",
      "inorganic",
      "2 H2 + O2",
      "2 H2O",
      "spark, 25C",
    );
    let proposals = propose_candidates_from_ankh(&ankh);
    let chem: Vec<&CandidateRowProposal> = proposals
      .iter()
      .filter(|p| p.candidate_kind == CandidateKind::ChemicalReactionLower)
      .collect();
    assert_eq!(chem.len(), 1);
    let p = chem[0];
    assert_eq!(p.target_table, "knownChemicalReactions");
    assert_eq!(p.evidence_count, 2);
    assert_eq!(p.proposed_row.get("reactants").unwrap(), "2 H2 + O2");
    assert_eq!(p.proposed_row.get("products").unwrap(), "2 H2O");
    assert_eq!(p.proposed_row.get("conditions").unwrap(), "spark, 25C");
    assert_eq!(p.proposed_row.get("language").unwrap(), "inorganic");
  }

  #[test]
  fn external_chemistry_entries_do_not_lower_directly_to_known_reactions() {
    let mut ankh = InMemoryAnkhStore::new();
    seed_chemistry_reaction_with_provenance(
      &mut ankh,
      "https://example.test/chem-a",
      "inorganic",
      "2 H2 + O2",
      "2 H2O",
      "spark, 25C",
      AnkhProvenanceSource::ExternalKnowledgeSearch,
    );
    seed_chemistry_reaction_with_provenance(
      &mut ankh,
      "https://example.test/chem-b",
      "inorganic",
      "2 H2 + O2",
      "2 H2O",
      "spark, 25C",
      AnkhProvenanceSource::ExternalKnowledgeSearch,
    );

    let proposals = propose_candidates_from_ankh(&ankh);
    assert!(
      proposals
        .iter()
        .all(|p| p.candidate_kind != CandidateKind::ChemicalReactionLower),
      "external paper notes must not synthesize known chemical reactions: {proposals:#?}"
    );
  }

  #[test]
  fn different_chemistry_conditions_do_not_collapse() {
    // Same reactants + products but different conditions = different
    // proposals. pnix does not assume conditions are interchangeable.
    let mut ankh = InMemoryAnkhStore::new();
    for (path, cond) in &[
      ("chem/a.md", "spark, 25C"),
      ("chem/b.md", "spark, 25C"),
      ("chem/c.md", "Pt catalyst, 25C"),
      ("chem/d.md", "Pt catalyst, 25C"),
    ] {
      seed_chemistry_reaction(&mut ankh, path, "inorganic", "2 H2 + O2", "2 H2O", cond);
    }
    let all = propose_candidates_from_ankh(&ankh);
    let chem_count = all
      .iter()
      .filter(|p| p.candidate_kind == CandidateKind::ChemicalReactionLower)
      .count();
    assert_eq!(chem_count, 2, "two distinct conditions → two proposals");
  }

  /// **Substrate-sharing N=3 proof.** One ankh + one
  /// `propose_candidates_from_ankh` call lands three distinct
  /// domain proposals: coding (RecurringImportSpec) + math
  /// (MathExpressionLower) + chemistry (ChemicalReactionLower).
  /// Same firewall entry point, three domains.
  #[test]
  fn substrate_sharing_proof_n_equals_three_domains() {
    let mut ankh = InMemoryAnkhStore::new();
    // Coding evidence
    seed_entry(
      &mut ankh,
      "lookup-module-providing-symbol",
      "src/a.py",
      "python",
      AnkhProvenanceSource::HostSymbolResolver,
      &[("candidate_import_spec", "import os")],
    );
    seed_entry(
      &mut ankh,
      "lookup-module-providing-symbol",
      "src/b.py",
      "python",
      AnkhProvenanceSource::HostSymbolResolver,
      &[("candidate_import_spec", "import os")],
    );
    // Math evidence
    seed_math_identity(
      &mut ankh,
      "math/a.md",
      "polynomial",
      "(x+y)^2",
      "x^2 + 2*x*y + y^2",
    );
    seed_math_identity(
      &mut ankh,
      "math/b.md",
      "polynomial",
      "(x+y)^2",
      "x^2 + 2*x*y + y^2",
    );
    // Chemistry evidence
    seed_chemistry_reaction(
      &mut ankh,
      "chem/a.md",
      "inorganic",
      "2 H2 + O2",
      "2 H2O",
      "spark, 25C",
    );
    seed_chemistry_reaction(
      &mut ankh,
      "chem/b.md",
      "inorganic",
      "2 H2 + O2",
      "2 H2O",
      "spark, 25C",
    );

    let proposals = propose_candidates_from_ankh(&ankh);
    let kinds: Vec<CandidateKind> = proposals.iter().map(|p| p.candidate_kind).collect();
    assert!(
      kinds.contains(&CandidateKind::RecurringImportSpec),
      "coding lane proposal must emerge"
    );
    assert!(
      kinds.contains(&CandidateKind::MathExpressionLower),
      "math lane proposal must emerge"
    );
    assert!(
      kinds.contains(&CandidateKind::ChemicalReactionLower),
      "chemistry lane proposal must emerge"
    );
  }

  #[test]
  fn substrate_sharing_proof_same_propose_function_handles_coding_and_math_jointly() {
    // The substrate-sharing claim: one ankh + one
    // `propose_candidates_from_ankh` call lands BOTH a coding
    // proposal (RecurringImportSpec) AND a math proposal
    // (MathExpressionLower) — same firewall entry point.
    let mut ankh = InMemoryAnkhStore::new();

    // Coding lane evidence
    seed_entry(
      &mut ankh,
      "lookup-module-providing-symbol",
      "src/a.py",
      "python",
      AnkhProvenanceSource::HostSymbolResolver,
      &[("candidate_import_spec", "import os")],
    );
    seed_entry(
      &mut ankh,
      "lookup-module-providing-symbol",
      "src/b.py",
      "python",
      AnkhProvenanceSource::HostSymbolResolver,
      &[("candidate_import_spec", "import os")],
    );

    // Math lane evidence
    seed_math_identity(
      &mut ankh,
      "math/a.md",
      "polynomial",
      "(x+y)^2",
      "x^2 + 2*x*y + y^2",
    );
    seed_math_identity(
      &mut ankh,
      "math/b.md",
      "polynomial",
      "(x+y)^2",
      "x^2 + 2*x*y + y^2",
    );

    let proposals = propose_candidates_from_ankh(&ankh);
    let kinds: Vec<CandidateKind> = proposals.iter().map(|p| p.candidate_kind).collect();
    assert!(
      kinds.contains(&CandidateKind::RecurringImportSpec),
      "coding lane must produce a proposal"
    );
    assert!(
      kinds.contains(&CandidateKind::MathExpressionLower),
      "math lane must produce a proposal"
    );
    // Also recurring-channel-success may fire (two host-symbol-resolver
    // entries answering the same query_kind) — that's fine.
  }
}
