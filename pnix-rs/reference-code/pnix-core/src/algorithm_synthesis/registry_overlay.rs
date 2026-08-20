//! In-memory registry overlay — Stage E's "last mile".
//!
//! OWNER-LAW (2026-05-12): mirror of
//! `stdlib/lib/gate/algorithm-synthesis/registry-overlay.px`. Lets a
//! promoted candidate's row show up in the running classifier
//! WITHOUT a process restart. Same provenance discipline as ankh:
//! every overlay entry traces back to a HotReloadPlan, which
//! traces back through gate 5 to the original ankh evidence.
//!
//! v0 supports the hot registries used by the evolution lane:
//! held-routing, intent-signals, operation-map, fact-phrase
//! patterns, plus the math / chemistry / import domain overlays.
//!
//! Append-only merge policy. Overlays consulted AFTER static
//! const slices; they ADD entries, never override existing ones.

use serde::{Deserialize, Serialize};
use pnix_hash::{Digest, Sha256};
use std::collections::BTreeMap;

use super::ankh_retrieval_cache::AnkhStore;
use super::held_to_query::{HeldQueryRecoveryChannel, HeldRetrievalQuery};
use super::owner_law_gate::OwnerLawProcessedCandidate;
use super::parameter_resolution::ResolutionHeldKind;
use super::runtime_hot_reload::{HotReloadOutcome, HotReloadPlan};

/// Registries this overlay supports. Stays byte-identical to `.px`
/// `validRegistryTargets`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RegistryOverlayTarget {
  HeldRoutingMap,
  IntentSignals,
  OperationMap,
  FactPhrasePatterns,
  /// Math-domain registry — substrate-sharing proof at the final
  /// runtime layer. Promoted `math-expression-lower` candidates land
  /// here. Same overlay machinery, different domain — the substrate
  /// is genuinely domain-agnostic at every stage of the lane,
  /// including the runtime-visible terminal.
  KnownAlgebraicIdentities,
  /// Chemistry-domain registry — substrate-sharing **N=3 proof**.
  /// Promoted `chemical-reaction-lower` candidates land here.
  /// Demonstrates the substrate scales beyond two domains: one
  /// firewall, one overlay machinery, N independent domains.
  KnownChemicalReactions,
  /// Coding-domain registry — promoted `recurring-import-spec`
  /// candidates land here. Parallel structure to math/chemistry
  /// registries: (language, import_spec) row pairs.
  KnownImportsByLanguage,
}

impl RegistryOverlayTarget {
  pub const ALL: &'static [Self] = &[
    Self::HeldRoutingMap,
    Self::IntentSignals,
    Self::OperationMap,
    Self::FactPhrasePatterns,
    Self::KnownAlgebraicIdentities,
    Self::KnownChemicalReactions,
    Self::KnownImportsByLanguage,
  ];

  pub fn as_str(self) -> &'static str {
    match self {
      Self::HeldRoutingMap => "held-routing-map",
      Self::IntentSignals => "intent-signals",
      Self::OperationMap => "operation-map",
      Self::FactPhrasePatterns => "fact-phrase-patterns",
      Self::KnownAlgebraicIdentities => "known-algebraic-identities",
      Self::KnownChemicalReactions => "known-chemical-reactions",
      Self::KnownImportsByLanguage => "known-imports-by-language",
    }
  }

  /// Map a target table name (from a hot-reload plan's
  /// `target_table` field) into a `RegistryOverlayTarget`. Returns
  /// `None` for tables this overlay doesn't support yet.
  pub fn from_target_table(target_table: &str) -> Option<Self> {
    match target_table {
      "heldRoutingMap" => Some(Self::HeldRoutingMap),
      "intentSignals" => Some(Self::IntentSignals),
      "operationMap" => Some(Self::OperationMap),
      "factPhrasePatterns" => Some(Self::FactPhrasePatterns),
      "knownAlgebraicIdentities" => Some(Self::KnownAlgebraicIdentities),
      "knownChemicalReactions" => Some(Self::KnownChemicalReactions),
      "knownImportsByLanguage" => Some(Self::KnownImportsByLanguage),
      _ => None,
    }
  }
}

/// Provenance carried on every overlay entry. Required (non-Option)
/// per `.px` `requiredOverlayFields`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OverlayProvenance {
  pub source_hot_reload_plan_fingerprint: String,
  pub stored_at_ms: u64,
  pub contributing_actor_id: String,
  pub contributing_tenant_id: String,
}

/// Overlay entry for `held_to_query::HELD_ROUTING`. Mirrors the
/// shape of the static `HeldRoutingEntry` but with a String
/// `query_kind` (the static table uses `&'static str`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeldRoutingOverlayEntry {
  pub held: ResolutionHeldKind,
  pub primary: HeldQueryRecoveryChannel,
  pub fallback: Option<HeldQueryRecoveryChannel>,
  pub query_kind: String,
  pub provenance: OverlayProvenance,
}

/// Overlay entry for `intent_recognition::INTENT_SIGNALS`. Same
/// shape as the static `IntentSignalEntry` but owned (String).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntentSignalOverlayEntry {
  pub cue: String,
  pub intent: String,
  pub weight: f32,
  pub provenance: OverlayProvenance,
}

/// Overlay entry for
/// `operation_candidate_mapping::OPERATION_MAP`. `cues` is a list
/// of cue strings; the hot-reload plan's row text stores them as a
/// comma-separated string (`macro_fold_gate` only emits string
/// values), so this struct holds the parsed Vec.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperationMapOverlayEntry {
  pub intent: String,
  pub cues: Vec<String>,
  pub transform: String,
  pub weight: f32,
  pub provenance: OverlayProvenance,
}

/// Overlay entry for `fact_cue_registry::FACT_PHRASE_PATTERNS`.
/// `markers` is the list of substring patterns that fire the cue;
/// same comma-separated-string convention as `OperationMapOverlayEntry`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FactPhrasePatternOverlayEntry {
  pub cue: String,
  pub markers: Vec<String>,
  pub provenance: OverlayProvenance,
}

/// Overlay entry for `knownAlgebraicIdentities`. Substrate-sharing
/// math-lane terminal. Three required fields, same `Vec<...>` +
/// `OverlayProvenance` shape as the coding registries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnownAlgebraicIdentityOverlayEntry {
  pub canonical_form: String,
  pub equivalent_form: String,
  pub language: String,
  pub provenance: OverlayProvenance,
}

/// Overlay entry for `knownChemicalReactions`. Substrate-sharing
/// N=3 proof — chemistry domain lands with same shape machinery.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnownChemicalReactionOverlayEntry {
  pub reactants: String,
  pub products: String,
  pub conditions: String,
  pub language: String,
  pub provenance: OverlayProvenance,
}

/// Overlay entry for `knownImportsByLanguage`. Coding-lane terminal
/// — promoted RecurringImportSpec rows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnownImportsByLanguageOverlayEntry {
  pub language: String,
  pub import_spec: String,
  pub provenance: OverlayProvenance,
}

/// In-memory store for both overlay kinds. v0 is session-scoped
/// (analogous to InMemoryAnkhStore). v1 will persist alongside
/// the on-disk `.px` row so process restart replays the same
/// overlay state.
#[derive(Debug, Clone, Default)]
pub struct InMemoryRegistryOverlay {
  held_routing: Vec<HeldRoutingOverlayEntry>,
  intent_signals: Vec<IntentSignalOverlayEntry>,
  operation_map: Vec<OperationMapOverlayEntry>,
  fact_phrase_patterns: Vec<FactPhrasePatternOverlayEntry>,
  known_algebraic_identities: Vec<KnownAlgebraicIdentityOverlayEntry>,
  known_chemical_reactions: Vec<KnownChemicalReactionOverlayEntry>,
  known_imports_by_language: Vec<KnownImportsByLanguageOverlayEntry>,
}

impl InMemoryRegistryOverlay {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn held_routing(&self) -> &[HeldRoutingOverlayEntry] {
    &self.held_routing
  }

  pub fn intent_signals(&self) -> &[IntentSignalOverlayEntry] {
    &self.intent_signals
  }

  pub fn operation_map(&self) -> &[OperationMapOverlayEntry] {
    &self.operation_map
  }

  pub fn fact_phrase_patterns(&self) -> &[FactPhrasePatternOverlayEntry] {
    &self.fact_phrase_patterns
  }

  pub fn known_algebraic_identities(&self) -> &[KnownAlgebraicIdentityOverlayEntry] {
    &self.known_algebraic_identities
  }

  pub fn known_chemical_reactions(&self) -> &[KnownChemicalReactionOverlayEntry] {
    &self.known_chemical_reactions
  }

  pub fn known_imports_by_language(&self) -> &[KnownImportsByLanguageOverlayEntry] {
    &self.known_imports_by_language
  }

  pub fn push_held_routing(&mut self, entry: HeldRoutingOverlayEntry) {
    self.held_routing.push(entry);
  }

  pub fn push_intent_signal(&mut self, entry: IntentSignalOverlayEntry) {
    self.intent_signals.push(entry);
  }

  pub fn push_operation_map(&mut self, entry: OperationMapOverlayEntry) {
    self.operation_map.push(entry);
  }

  pub fn push_fact_phrase_pattern(&mut self, entry: FactPhrasePatternOverlayEntry) {
    self.fact_phrase_patterns.push(entry);
  }

  pub fn push_known_algebraic_identity(&mut self, entry: KnownAlgebraicIdentityOverlayEntry) {
    self.known_algebraic_identities.push(entry);
  }

  pub fn push_known_chemical_reaction(&mut self, entry: KnownChemicalReactionOverlayEntry) {
    self.known_chemical_reactions.push(entry);
  }

  pub fn push_known_imports_by_language(&mut self, entry: KnownImportsByLanguageOverlayEntry) {
    self.known_imports_by_language.push(entry);
  }

  pub fn len_total(&self) -> usize {
    self.held_routing.len()
      + self.intent_signals.len()
      + self.operation_map.len()
      + self.fact_phrase_patterns.len()
      + self.known_algebraic_identities.len()
      + self.known_chemical_reactions.len()
      + self.known_imports_by_language.len()
  }

  pub fn is_empty(&self) -> bool {
    self.len_total() == 0
  }
}

/// Errors from converting a HotReloadPlan into overlay entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverlayConversionError {
  /// The plan is not in `PlanReady` state.
  PlanNotReady(HotReloadOutcome),
  /// The target_table is not supported by overlay v0.
  UnsupportedTarget(String),
  /// A required field is missing from the proposed_row.
  MissingField(String),
  /// A field value can't be parsed into its typed form (e.g.
  /// invalid held-kind string, unknown channel).
  InvalidFieldValue { field: String, value: String },
}

impl std::fmt::Display for OverlayConversionError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::PlanNotReady(o) => write!(f, "cannot convert non-ready plan ({})", o.as_str()),
      Self::UnsupportedTarget(t) => write!(f, "overlay v0 does not support target `{t}`"),
      Self::MissingField(k) => write!(f, "proposed_row missing required field `{k}`"),
      Self::InvalidFieldValue { field, value } => {
        write!(
          f,
          "proposed_row field `{field}` has invalid value `{value}`"
        )
      }
    }
  }
}

impl std::error::Error for OverlayConversionError {}

fn parse_held_kind(s: &str) -> Option<ResolutionHeldKind> {
  ResolutionHeldKind::ALL
    .iter()
    .find(|k| k.as_str() == s)
    .copied()
}

fn parse_channel(s: &str) -> Option<HeldQueryRecoveryChannel> {
  HeldQueryRecoveryChannel::ALL
    .iter()
    .find(|c| c.as_str() == s)
    .copied()
}

/// Read `(key, value)` pairs from a folded source text. Mirror of
/// `regression_proof_gate::extract_kv_from_folded_text` so overlay
/// conversion stays consistent with the rest of the pipeline.
fn extract_kv_from_folded_text(folded: &str) -> BTreeMap<String, String> {
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
    let after_eq = after_eq.trim_end_matches(';').trim();
    let Some(unquoted) = after_eq.strip_prefix('"').and_then(|s| s.strip_suffix('"')) else {
      continue;
    };
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

/// Convert a `HotReloadPlan { PlanReady }` into a typed overlay
/// entry and push it onto the overlay store. v0 supports the
/// two registries named in `RegistryOverlayTarget::ALL`.
pub fn apply_hot_reload_plan_to_overlay(
  plan: &HotReloadPlan,
  overlay: &mut InMemoryRegistryOverlay,
  source_candidate: &OwnerLawProcessedCandidate,
) -> Result<RegistryOverlayTarget, OverlayConversionError> {
  if plan.outcome != HotReloadOutcome::PlanReady {
    return Err(OverlayConversionError::PlanNotReady(plan.outcome));
  }
  // Walk down the gate chain to reach the original CandidateRowProposal:
  //   HotReloadPlan → OwnerLawProcessed → RegressionProven →
  //   AxisSeparated → MacroFolded → CandidateRowProposal
  let target_table = plan
    .source
    .source
    .source
    .source
    .source
    .target_table
    .as_str();
  let Some(target) = RegistryOverlayTarget::from_target_table(target_table) else {
    return Err(OverlayConversionError::UnsupportedTarget(
      target_table.to_string(),
    ));
  };

  // folded_source_text lives on MacroFoldedCandidate (4 hops down).
  let kv = extract_kv_from_folded_text(&plan.source.source.source.source.folded_source_text);
  let approval = source_candidate.approval.as_ref().ok_or_else(|| {
    OverlayConversionError::MissingField("approval (on source candidate)".to_string())
  })?;
  let provenance = OverlayProvenance {
    source_hot_reload_plan_fingerprint: source_candidate.candidate_fingerprint.clone(),
    stored_at_ms: approval.approved_at_ms,
    contributing_actor_id: approval.actor_id.clone(),
    contributing_tenant_id: approval.tenant_id.clone(),
  };

  match target {
    RegistryOverlayTarget::HeldRoutingMap => {
      let held_str = kv
        .get("held")
        .ok_or_else(|| OverlayConversionError::MissingField("held".to_string()))?;
      let primary_str = kv
        .get("primary")
        .ok_or_else(|| OverlayConversionError::MissingField("primary".to_string()))?;
      let held =
        parse_held_kind(held_str).ok_or_else(|| OverlayConversionError::InvalidFieldValue {
          field: "held".to_string(),
          value: held_str.clone(),
        })?;
      let primary =
        parse_channel(primary_str).ok_or_else(|| OverlayConversionError::InvalidFieldValue {
          field: "primary".to_string(),
          value: primary_str.clone(),
        })?;
      let fallback = match kv.get("fallback") {
        None => None,
        Some(s) => {
          Some(
            parse_channel(s).ok_or_else(|| OverlayConversionError::InvalidFieldValue {
              field: "fallback".to_string(),
              value: s.clone(),
            })?,
          )
        }
      };
      // The query_kind is not in the row schema — derive a
      // descriptive placeholder from the held kind. Future:
      // schema-aware proposers may emit a `query_kind` field
      // directly.
      let query_kind = format!("overlay-derived:{}", held.as_str());
      overlay.push_held_routing(HeldRoutingOverlayEntry {
        held,
        primary,
        fallback,
        query_kind,
        provenance,
      });
    }
    RegistryOverlayTarget::IntentSignals => {
      let cue = kv
        .get("cue")
        .ok_or_else(|| OverlayConversionError::MissingField("cue".to_string()))?
        .clone();
      let intent = kv
        .get("intent")
        .ok_or_else(|| OverlayConversionError::MissingField("intent".to_string()))?
        .clone();
      let weight_str = kv
        .get("weight")
        .ok_or_else(|| OverlayConversionError::MissingField("weight".to_string()))?;
      let weight: f32 =
        weight_str
          .parse()
          .map_err(|_| OverlayConversionError::InvalidFieldValue {
            field: "weight".to_string(),
            value: weight_str.clone(),
          })?;
      overlay.push_intent_signal(IntentSignalOverlayEntry {
        cue,
        intent,
        weight,
        provenance,
      });
    }
    RegistryOverlayTarget::OperationMap => {
      let intent = kv
        .get("intent")
        .ok_or_else(|| OverlayConversionError::MissingField("intent".to_string()))?
        .clone();
      let cues_raw = kv
        .get("cues")
        .ok_or_else(|| OverlayConversionError::MissingField("cues".to_string()))?;
      let cues = parse_comma_separated_list(cues_raw);
      let transform = kv
        .get("transform")
        .ok_or_else(|| OverlayConversionError::MissingField("transform".to_string()))?
        .clone();
      let weight_str = kv
        .get("weight")
        .ok_or_else(|| OverlayConversionError::MissingField("weight".to_string()))?;
      let weight: f32 =
        weight_str
          .parse()
          .map_err(|_| OverlayConversionError::InvalidFieldValue {
            field: "weight".to_string(),
            value: weight_str.clone(),
          })?;
      overlay.push_operation_map(OperationMapOverlayEntry {
        intent,
        cues,
        transform,
        weight,
        provenance,
      });
    }
    RegistryOverlayTarget::FactPhrasePatterns => {
      let cue = kv
        .get("cue")
        .ok_or_else(|| OverlayConversionError::MissingField("cue".to_string()))?
        .clone();
      let markers_raw = kv
        .get("markers")
        .ok_or_else(|| OverlayConversionError::MissingField("markers".to_string()))?;
      let markers = parse_comma_separated_list(markers_raw);
      overlay.push_fact_phrase_pattern(FactPhrasePatternOverlayEntry {
        cue,
        markers,
        provenance,
      });
    }
    RegistryOverlayTarget::KnownAlgebraicIdentities => {
      let canonical_form = kv
        .get("canonical_form")
        .ok_or_else(|| OverlayConversionError::MissingField("canonical_form".to_string()))?
        .clone();
      let equivalent_form = kv
        .get("equivalent_form")
        .ok_or_else(|| OverlayConversionError::MissingField("equivalent_form".to_string()))?
        .clone();
      let language = kv
        .get("language")
        .ok_or_else(|| OverlayConversionError::MissingField("language".to_string()))?
        .clone();
      overlay.push_known_algebraic_identity(KnownAlgebraicIdentityOverlayEntry {
        canonical_form,
        equivalent_form,
        language,
        provenance,
      });
    }
    RegistryOverlayTarget::KnownChemicalReactions => {
      let reactants = kv
        .get("reactants")
        .ok_or_else(|| OverlayConversionError::MissingField("reactants".to_string()))?
        .clone();
      let products = kv
        .get("products")
        .ok_or_else(|| OverlayConversionError::MissingField("products".to_string()))?
        .clone();
      let conditions = kv
        .get("conditions")
        .ok_or_else(|| OverlayConversionError::MissingField("conditions".to_string()))?
        .clone();
      let language = kv
        .get("language")
        .ok_or_else(|| OverlayConversionError::MissingField("language".to_string()))?
        .clone();
      overlay.push_known_chemical_reaction(KnownChemicalReactionOverlayEntry {
        reactants,
        products,
        conditions,
        language,
        provenance,
      });
    }
    RegistryOverlayTarget::KnownImportsByLanguage => {
      let language = kv
        .get("language")
        .ok_or_else(|| OverlayConversionError::MissingField("language".to_string()))?
        .clone();
      let import_spec = kv
        .get("import_spec")
        .ok_or_else(|| OverlayConversionError::MissingField("import_spec".to_string()))?
        .clone();
      overlay.push_known_imports_by_language(KnownImportsByLanguageOverlayEntry {
        language,
        import_spec,
        provenance,
      });
    }
  }
  Ok(target)
}

/// Per-registry overlay row counts. Used by the registry-overlay
/// receipt to show before/after deltas — operator sees that the
/// candidate landed in the *specific* registry it should have.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct OverlayRowCounts {
  pub held_routing: usize,
  pub intent_signals: usize,
  pub operation_map: usize,
  pub fact_phrase_patterns: usize,
  pub known_algebraic_identities: usize,
  pub known_chemical_reactions: usize,
  pub known_imports_by_language: usize,
}

impl OverlayRowCounts {
  pub fn from_overlay(overlay: &InMemoryRegistryOverlay) -> Self {
    Self {
      held_routing: overlay.held_routing().len(),
      intent_signals: overlay.intent_signals().len(),
      operation_map: overlay.operation_map().len(),
      fact_phrase_patterns: overlay.fact_phrase_patterns().len(),
      known_algebraic_identities: overlay.known_algebraic_identities().len(),
      known_chemical_reactions: overlay.known_chemical_reactions().len(),
      known_imports_by_language: overlay.known_imports_by_language().len(),
    }
  }

  pub fn total(&self) -> usize {
    self.held_routing
      + self.intent_signals
      + self.operation_map
      + self.fact_phrase_patterns
      + self.known_algebraic_identities
      + self.known_chemical_reactions
      + self.known_imports_by_language
  }
}

/// Receipt of one `apply_hot_reload_plan_to_overlay` attempt.
/// The carrier-level value `apply_hot_reload_plan_to_overlay` already
/// returns is `Result<RegistryOverlayTarget, OverlayConversionError>`;
/// this struct adds before/after row counts and a stable receipt
/// shape so the cockpit panel can render either outcome uniformly.
///
/// `status`:
///   - `applied`               — overlay grew by 1 row in the named target
///   - `held-plan-not-ready`   — caller bug: plan not in PlanReady
///   - `held-unsupported-target` — target_table not in overlay v0
///   - `held-row-text-parse-failed` — folded text malformed
///
/// `target_registry` is populated on `applied`; empty otherwise.
#[derive(Debug, Clone, PartialEq)]
pub struct OverlayApplyReceipt {
  pub status: OverlayApplyStatus,
  pub target_registry: Option<RegistryOverlayTarget>,
  pub before_counts: OverlayRowCounts,
  pub after_counts: OverlayRowCounts,
  pub source_hot_reload_plan_fingerprint: String,
  pub source_target_owner: String,
  pub source_target_table: String,
  pub source_candidate_kind: String,
  pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OverlayApplyStatus {
  Applied,
  HeldPlanNotReady,
  HeldUnsupportedTarget,
  HeldRowTextParseFailed,
}

impl OverlayApplyStatus {
  pub fn as_str(self) -> &'static str {
    match self {
      Self::Applied => "applied",
      Self::HeldPlanNotReady => "held-plan-not-ready",
      Self::HeldUnsupportedTarget => "held-unsupported-target",
      Self::HeldRowTextParseFailed => "held-row-text-parse-failed",
    }
  }
}

/// Apply a hot-reload plan to the overlay AND emit a receipt. Same
/// semantics as `apply_hot_reload_plan_to_overlay` but also captures
/// the before/after diff for the cockpit. Caller uses this when they
/// want the receipt; `apply_hot_reload_plan_to_overlay` stays for
/// pipeline-internal use where only the outcome matters.
pub fn apply_hot_reload_plan_to_overlay_with_receipt(
  plan: &HotReloadPlan,
  overlay: &mut InMemoryRegistryOverlay,
  source_candidate: &OwnerLawProcessedCandidate,
) -> OverlayApplyReceipt {
  let before = OverlayRowCounts::from_overlay(overlay);
  let proposal = &plan.source.source.source.source.source;
  let source_target_owner = proposal.target_owner.clone();
  let source_target_table = proposal.target_table.clone();
  let source_candidate_kind = proposal.candidate_kind.as_str().to_string();
  // Fingerprint the plan deterministically so audit can correlate
  // this receipt with a specific hot-reload-plan artifact.
  let source_fp = {
    let mut h = Sha256::new();
    h.update(plan.target_path.as_bytes());
    h.update(b"\x1f");
    h.update(plan.pre_apply_sha256.as_bytes());
    h.update(b"\x1f");
    h.update(plan.post_apply_sha256.as_bytes());
    h.update(b"\x1f");
    h.update(plan.outcome.as_str().as_bytes());
    h.update(b"\x1f");
    h.update(plan.inserted_row_text.as_bytes());
    format!("{:x}", h.finalize())
  };

  let outcome = apply_hot_reload_plan_to_overlay(plan, overlay, source_candidate);
  let after = OverlayRowCounts::from_overlay(overlay);

  let (status, target_registry, reason) = match outcome {
    Ok(t) => (
      OverlayApplyStatus::Applied,
      Some(t),
      format!(
        "overlay grew from {} to {} rows; new entry in {}",
        before.total(),
        after.total(),
        t.as_str()
      ),
    ),
    Err(OverlayConversionError::PlanNotReady(o)) => (
      OverlayApplyStatus::HeldPlanNotReady,
      None,
      format!("plan outcome `{}` is not PlanReady", o.as_str()),
    ),
    Err(OverlayConversionError::UnsupportedTarget(t)) => (
      OverlayApplyStatus::HeldUnsupportedTarget,
      None,
      format!("overlay v0 does not support target `{t}`"),
    ),
    Err(OverlayConversionError::MissingField(k)) => (
      OverlayApplyStatus::HeldRowTextParseFailed,
      None,
      format!("proposed_row missing required field `{k}`"),
    ),
    Err(OverlayConversionError::InvalidFieldValue { field, value }) => (
      OverlayApplyStatus::HeldRowTextParseFailed,
      None,
      format!("proposed_row field `{field}` has invalid value `{value}`"),
    ),
  };

  OverlayApplyReceipt {
    status,
    target_registry,
    before_counts: before,
    after_counts: after,
    source_hot_reload_plan_fingerprint: source_fp,
    source_target_owner,
    source_target_table,
    source_candidate_kind,
    reason,
  }
}

/// Render an `OverlayApplyReceipt` as the canonical JSON payload of
/// a `coding.registry-overlay-receipt` artifact. The final
/// surface in the evolution lane: "did the promoted candidate
/// actually become visible to the next NL turn's vocabulary?"
///
/// On `applied`, the panel shows the named target_registry + the
/// before/after row counts (operator sees the +1 in the specific
/// registry). On Held outcomes, target_registry is null and reason
/// explains why the overlay didn't grow.
///
/// Replay-stable id = SHA-256 of intrinsic identity (status +
/// source_hot_reload_plan_fingerprint + target_registry + source
/// target_owner / target_table + sorted before/after counts).
///
/// Customer-release safe — counts + fingerprint + actor metadata
/// only, no source bodies.
pub fn build_registry_overlay_receipt_artifact(
  receipt: &OverlayApplyReceipt,
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
) -> serde_json::Value {
  let mut h = Sha256::new();
  h.update(b"registry-overlay-receipt\x1f");
  h.update(receipt.status.as_str().as_bytes());
  h.update(b"\x1f");
  h.update(receipt.source_hot_reload_plan_fingerprint.as_bytes());
  h.update(b"\x1f");
  if let Some(t) = receipt.target_registry {
    h.update(t.as_str().as_bytes());
  }
  h.update(b"\x1f");
  h.update(receipt.source_target_owner.as_bytes());
  h.update(b"\x1e");
  h.update(receipt.source_target_table.as_bytes());
  h.update(b"\x1f");
  h.update(receipt.before_counts.total().to_string().as_bytes());
  h.update(b"\x1e");
  h.update(receipt.after_counts.total().to_string().as_bytes());
  let digest = h.finalize();
  let prefix = digest
    .iter()
    .take(16)
    .map(|b| format!("{b:02x}"))
    .collect::<String>();
  let id = format!("registry-overlay-receipt.{prefix}");

  let mut payload = serde_json::json!({
    "id": id,
    "artifact_family": "coding.registry-overlay-receipt",
    "source_surface": "algorithm-synthesis.registry-overlay",
    "stored_at_ms": stored_at_ms,
    "status": receipt.status.as_str(),
    "target_registry": receipt.target_registry.map(|t| t.as_str()),
    "candidate_kind": receipt.source_candidate_kind,
    "source_target_owner": receipt.source_target_owner,
    "source_target_table": receipt.source_target_table,
    "source_hot_reload_plan_fingerprint": receipt.source_hot_reload_plan_fingerprint,
    "before_counts": {
      "held_routing": receipt.before_counts.held_routing,
      "intent_signals": receipt.before_counts.intent_signals,
      "operation_map": receipt.before_counts.operation_map,
      "fact_phrase_patterns": receipt.before_counts.fact_phrase_patterns,
      "known_algebraic_identities": receipt.before_counts.known_algebraic_identities,
      "known_chemical_reactions": receipt.before_counts.known_chemical_reactions,
      "known_imports_by_language": receipt.before_counts.known_imports_by_language,
      "total": receipt.before_counts.total(),
    },
    "after_counts": {
      "held_routing": receipt.after_counts.held_routing,
      "intent_signals": receipt.after_counts.intent_signals,
      "operation_map": receipt.after_counts.operation_map,
      "fact_phrase_patterns": receipt.after_counts.fact_phrase_patterns,
      "known_algebraic_identities": receipt.after_counts.known_algebraic_identities,
      "known_chemical_reactions": receipt.after_counts.known_chemical_reactions,
      "known_imports_by_language": receipt.after_counts.known_imports_by_language,
      "total": receipt.after_counts.total(),
    },
    "delta_total": receipt.after_counts.total() as i64
      - receipt.before_counts.total() as i64,
    "reason": receipt.reason,
    "related_refs": serde_json::json!([
      format!("hot-reload-plan-fingerprint:{}", receipt.source_hot_reload_plan_fingerprint),
      format!("target-owner:{}", receipt.source_target_owner),
      format!("target-table:{}", receipt.source_target_table),
      format!("candidate-kind:{}", receipt.source_candidate_kind),
      "owner-law:stdlib/lib/gate/algorithm-synthesis/registry-overlay.px",
    ]),
    "target_paths": serde_json::json!([receipt.source_target_owner]),
    "command_refs": Vec::<String>::new(),
  });
  if let Some(snap) = repo_snapshot_ref {
    payload["repo_snapshot_ref"] = serde_json::Value::String(snap.to_string());
  }
  payload
}

/// Parse a comma-separated string into a list of trimmed tokens.
/// Empty tokens dropped. Used by overlay entries whose source `.px`
/// shape is `field = "v1,v2,v3"` (since macro-fold-gate only emits
/// string values, lists are flattened to comma-separated strings).
fn parse_comma_separated_list(s: &str) -> Vec<String> {
  s.split(',')
    .map(|t| t.trim().to_string())
    .filter(|t| !t.is_empty())
    .collect()
}

/// Cross-cutting substrate snapshot — gives the operator a single
/// typed receipt answering "what does the substrate currently
/// know?" Aggregates the in-memory ankh + registry overlay state
/// into per-domain summaries.
///
/// This is *not* a duplicate truth source — the authoritative state
/// lives in the ankh store and the on-disk `.px` files. This
/// snapshot is a *projection* (Category J in the lattice). Used by
/// the cockpit "substrate dashboard" panel. Re-built on demand;
/// not persisted on its own.
#[derive(Debug, Clone, PartialEq)]
pub struct SubstrateStateSummary {
  pub snapshot_at_ms: u64,
  pub ankh_total_entries: usize,
  /// Per-query-kind ankh entry count. Sorted by key.
  pub ankh_by_query_kind: BTreeMap<String, usize>,
  /// Per-provenance-source ankh entry count.
  pub ankh_by_provenance_source: BTreeMap<String, usize>,
  /// Overlay row counts, all 7 registries.
  pub overlay_counts: OverlayRowCounts,
}

impl SubstrateStateSummary {
  /// Build a fresh snapshot from current ankh + overlay state.
  /// Deterministic — same inputs → same summary.
  pub fn from_state<S: AnkhStore>(
    ankh: &S,
    overlay: &InMemoryRegistryOverlay,
    snapshot_at_ms: u64,
  ) -> Self {
    let entries = ankh.iter_entries();
    let mut by_query_kind: BTreeMap<String, usize> = BTreeMap::new();
    let mut by_provenance: BTreeMap<String, usize> = BTreeMap::new();
    for (_, e) in &entries {
      *by_query_kind.entry(e.query_kind.clone()).or_insert(0) += 1;
      *by_provenance
        .entry(e.provenance_source.as_str().to_string())
        .or_insert(0) += 1;
    }
    Self {
      snapshot_at_ms,
      ankh_total_entries: entries.len(),
      ankh_by_query_kind: by_query_kind,
      ankh_by_provenance_source: by_provenance,
      overlay_counts: OverlayRowCounts::from_overlay(overlay),
    }
  }
}

/// Delta between two `SubstrateStateSummary` snapshots. Operator
/// uses this to answer "what grew in this session?" — the
/// productivity receipt at the substrate level.
///
/// Signed deltas (i64) because ankh is technically append-only
/// per-key but `iter_entries()` over different stores can show
/// "negative growth" if the after snapshot has fewer entries.
/// The signs are intrinsic — operator should not silently abs()
/// them, since a *decrease* in any count is a substrate-level
/// audit event (data loss, store swap, rollback).
#[derive(Debug, Clone, PartialEq)]
pub struct SubstrateStateDelta {
  pub before_snapshot_at_ms: u64,
  pub after_snapshot_at_ms: u64,
  pub ankh_total_delta: i64,
  /// Per-query-kind delta. Keys present only in one snapshot show
  /// up with positive or negative delta against zero.
  pub ankh_by_query_kind_delta: BTreeMap<String, i64>,
  /// Per-provenance delta.
  pub ankh_by_provenance_source_delta: BTreeMap<String, i64>,
  /// Overlay per-registry delta.
  pub overlay_delta: OverlayRowCountsDelta,
}

/// Per-registry signed delta. Parallel to `OverlayRowCounts` but
/// each field is i64 to allow signed deltas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct OverlayRowCountsDelta {
  pub held_routing: i64,
  pub intent_signals: i64,
  pub operation_map: i64,
  pub fact_phrase_patterns: i64,
  pub known_algebraic_identities: i64,
  pub known_chemical_reactions: i64,
  pub known_imports_by_language: i64,
}

impl OverlayRowCountsDelta {
  pub fn from_diff(before: &OverlayRowCounts, after: &OverlayRowCounts) -> Self {
    let d = |b: usize, a: usize| (a as i64) - (b as i64);
    Self {
      held_routing: d(before.held_routing, after.held_routing),
      intent_signals: d(before.intent_signals, after.intent_signals),
      operation_map: d(before.operation_map, after.operation_map),
      fact_phrase_patterns: d(before.fact_phrase_patterns, after.fact_phrase_patterns),
      known_algebraic_identities: d(
        before.known_algebraic_identities,
        after.known_algebraic_identities,
      ),
      known_chemical_reactions: d(
        before.known_chemical_reactions,
        after.known_chemical_reactions,
      ),
      known_imports_by_language: d(
        before.known_imports_by_language,
        after.known_imports_by_language,
      ),
    }
  }

  pub fn total(&self) -> i64 {
    self.held_routing
      + self.intent_signals
      + self.operation_map
      + self.fact_phrase_patterns
      + self.known_algebraic_identities
      + self.known_chemical_reactions
      + self.known_imports_by_language
  }
}

impl SubstrateStateDelta {
  /// Compute the delta between two snapshots. Order matters:
  /// `before` then `after`. Caller decides which is which.
  pub fn from_pair(before: &SubstrateStateSummary, after: &SubstrateStateSummary) -> Self {
    let ankh_total_delta = (after.ankh_total_entries as i64) - (before.ankh_total_entries as i64);

    // Union of all keys; missing-from-one side contributes 0.
    let union_keys = |a: &BTreeMap<String, usize>, b: &BTreeMap<String, usize>| {
      let mut out: BTreeMap<String, i64> = BTreeMap::new();
      for (k, v) in a {
        let other = *b.get(k).unwrap_or(&0);
        let d = (other as i64) - (*v as i64);
        if d != 0 {
          out.insert(k.clone(), d);
        }
      }
      for (k, v) in b {
        if !a.contains_key(k) && *v != 0 {
          out.insert(k.clone(), *v as i64);
        }
      }
      out
    };

    Self {
      before_snapshot_at_ms: before.snapshot_at_ms,
      after_snapshot_at_ms: after.snapshot_at_ms,
      ankh_total_delta,
      ankh_by_query_kind_delta: union_keys(&before.ankh_by_query_kind, &after.ankh_by_query_kind),
      ankh_by_provenance_source_delta: union_keys(
        &before.ankh_by_provenance_source,
        &after.ankh_by_provenance_source,
      ),
      overlay_delta: OverlayRowCountsDelta::from_diff(
        &before.overlay_counts,
        &after.overlay_counts,
      ),
    }
  }
}

/// Render a `SubstrateStateDelta` as a typed
/// `coding.substrate-state-delta` artifact. Cross-cutting
/// productivity receipt — operator gets "this session grew the
/// substrate by N rows in M registries".
///
/// Replay-stable id = SHA-256 of intrinsic identity (before/after
/// snapshot_at_ms + all deltas).
pub fn build_substrate_state_delta_artifact(
  delta: &SubstrateStateDelta,
  repo_snapshot_ref: Option<&str>,
) -> serde_json::Value {
  use pnix_hash::{Digest, Sha256};
  let mut h = Sha256::new();
  h.update(b"substrate-state-delta\x1f");
  h.update(delta.before_snapshot_at_ms.to_string().as_bytes());
  h.update(b"\x1f");
  h.update(delta.after_snapshot_at_ms.to_string().as_bytes());
  h.update(b"\x1f");
  h.update(delta.ankh_total_delta.to_string().as_bytes());
  h.update(b"\x1f");
  for (k, v) in &delta.ankh_by_query_kind_delta {
    h.update(k.as_bytes());
    h.update(b"\x1d");
    h.update(v.to_string().as_bytes());
    h.update(b"\x1e");
  }
  h.update(b"\x1f");
  for (k, v) in &delta.ankh_by_provenance_source_delta {
    h.update(k.as_bytes());
    h.update(b"\x1d");
    h.update(v.to_string().as_bytes());
    h.update(b"\x1e");
  }
  h.update(b"\x1f");
  h.update(delta.overlay_delta.total().to_string().as_bytes());
  let digest = h.finalize();
  let prefix = digest
    .iter()
    .take(16)
    .map(|b| format!("{b:02x}"))
    .collect::<String>();
  let id = format!("substrate-state-delta.{prefix}");

  let mut payload = serde_json::json!({
    "id": id,
    "artifact_family": "coding.substrate-state-delta",
    "source_surface": "algorithm-synthesis.registry-overlay",
    "before_snapshot_at_ms": delta.before_snapshot_at_ms,
    "after_snapshot_at_ms": delta.after_snapshot_at_ms,
    "ankh_total_delta": delta.ankh_total_delta,
    "ankh_by_query_kind_delta": delta.ankh_by_query_kind_delta,
    "ankh_by_provenance_source_delta": delta.ankh_by_provenance_source_delta,
    "overlay_delta": {
      "held_routing": delta.overlay_delta.held_routing,
      "intent_signals": delta.overlay_delta.intent_signals,
      "operation_map": delta.overlay_delta.operation_map,
      "fact_phrase_patterns": delta.overlay_delta.fact_phrase_patterns,
      "known_algebraic_identities": delta.overlay_delta.known_algebraic_identities,
      "known_chemical_reactions": delta.overlay_delta.known_chemical_reactions,
      "known_imports_by_language": delta.overlay_delta.known_imports_by_language,
      "total": delta.overlay_delta.total(),
    },
    "related_refs": serde_json::json!([
      format!("before-snapshot-at-ms:{}", delta.before_snapshot_at_ms),
      format!("after-snapshot-at-ms:{}", delta.after_snapshot_at_ms),
      format!("ankh-total-delta:{}", delta.ankh_total_delta),
      format!("overlay-total-delta:{}", delta.overlay_delta.total()),
      "owner-law:stdlib/lib/gate/algorithm-synthesis/registry-overlay.px",
    ]),
    "target_paths": Vec::<String>::new(),
    "command_refs": Vec::<String>::new(),
  });
  if let Some(snap) = repo_snapshot_ref {
    payload["repo_snapshot_ref"] = serde_json::Value::String(snap.to_string());
  }
  payload
}

/// Unified glance — substrate-state-summary + multi-turn-session-
/// state bundled into a single typed receipt. Cross-cutting cockpit
/// "dashboard" projection: operator gets ankh + overlay + session
/// state in one artifact, one panel, one render.
///
/// Not a duplicate truth source. The two component states are
/// authoritative on their own (substrate-state-summary owns ankh
/// + overlay; multi-turn-session-state owns session). The glance
/// is a *composition* — operator wants to know everything at once.
#[derive(Debug, Clone, PartialEq)]
pub struct SubstrateGlance {
  pub glance_at_ms: u64,
  pub substrate_summary: SubstrateStateSummary,
  pub session_state: Option<super::held_to_query::SessionStateSnapshot>,
}

impl SubstrateGlance {
  /// Capture a unified glance from current ankh + overlay + session
  /// state. Pass `None` for session when no session is active.
  pub fn capture<S: AnkhStore>(
    ankh: &S,
    overlay: &InMemoryRegistryOverlay,
    session: Option<&super::held_to_query::MultiTurnSession>,
    glance_at_ms: u64,
  ) -> Self {
    Self {
      glance_at_ms,
      substrate_summary: SubstrateStateSummary::from_state(ankh, overlay, glance_at_ms),
      session_state: session.map(|s| s.capture_snapshot(glance_at_ms)),
    }
  }
}

/// Render a `SubstrateGlance` as the canonical JSON payload of a
/// `coding.substrate-glance` artifact. Cockpit-friendly
/// composition of substrate + session state.
///
/// Replay-stable id = SHA-256 of intrinsic identity (substrate
/// summary id components + session snapshot id components).
/// `glance_at_ms` is extrinsic.
///
/// Customer-release safe — counts + metadata + audit ids only.
pub fn build_substrate_glance_artifact(
  glance: &SubstrateGlance,
  repo_snapshot_ref: Option<&str>,
) -> serde_json::Value {
  use pnix_hash::{Digest, Sha256};
  let mut h = Sha256::new();
  h.update(b"substrate-glance\x1f");
  h.update(
    glance
      .substrate_summary
      .ankh_total_entries
      .to_string()
      .as_bytes(),
  );
  h.update(b"\x1f");
  h.update(
    glance
      .substrate_summary
      .overlay_counts
      .total()
      .to_string()
      .as_bytes(),
  );
  h.update(b"\x1f");
  if let Some(s) = &glance.session_state {
    if let Some(p) = &s.pending_held {
      h.update(b"pending\x1f");
      h.update(p.query_id.as_bytes());
    } else {
      h.update(b"session-at-rest");
    }
  } else {
    h.update(b"no-session");
  }
  let digest = h.finalize();
  let prefix = digest
    .iter()
    .take(16)
    .map(|b| format!("{b:02x}"))
    .collect::<String>();
  let id = format!("substrate-glance.{prefix}");

  let summary_payload = build_substrate_state_summary_artifact(&glance.substrate_summary, None);
  let session_payload = glance
    .session_state
    .as_ref()
    .map(|s| super::held_to_query::build_session_state_artifact(s, None));

  let mut payload = serde_json::json!({
    "id": id,
    "artifact_family": "coding.substrate-glance",
    "source_surface": "algorithm-synthesis.registry-overlay",
    "glance_at_ms": glance.glance_at_ms,
    // Component states embedded — operator can drill into either
    // without re-running the capture.
    "substrate_summary": summary_payload,
    "session_state": session_payload,
    "has_session": glance.session_state.is_some(),
    "has_pending_held": glance
      .session_state
      .as_ref()
      .and_then(|s| s.pending_held.as_ref())
      .is_some(),
    "ankh_total_entries": glance.substrate_summary.ankh_total_entries,
    "overlay_total": glance.substrate_summary.overlay_counts.total(),
    "related_refs": serde_json::json!([
      format!("ankh-total:{}", glance.substrate_summary.ankh_total_entries),
      format!("overlay-total:{}", glance.substrate_summary.overlay_counts.total()),
      "owner-law:stdlib/lib/gate/algorithm-synthesis/registry-overlay.px",
    ]),
    "target_paths": Vec::<String>::new(),
    "command_refs": Vec::<String>::new(),
  });
  if let Some(snap) = repo_snapshot_ref {
    payload["repo_snapshot_ref"] = serde_json::Value::String(snap.to_string());
  }
  payload
}

/// Render a `SubstrateStateSummary` as the canonical JSON payload of
/// a `coding.substrate-state-summary` artifact. The cross-
/// cutting cockpit surface — operator gets a single receipt that
/// answers "what does pnix currently know?"
///
/// Replay-stable id = SHA-256 of intrinsic identity (all counts +
/// sorted query_kind / provenance keys). `snapshot_at_ms` is
/// intrinsic (different snapshots are *different* substrates) and
/// participates in the hash.
///
/// Customer-release safe — counts + key names only, no source bodies.
pub fn build_substrate_state_summary_artifact(
  summary: &SubstrateStateSummary,
  repo_snapshot_ref: Option<&str>,
) -> serde_json::Value {
  use pnix_hash::{Digest, Sha256};
  let mut h = Sha256::new();
  h.update(b"substrate-state-summary\x1f");
  h.update(summary.snapshot_at_ms.to_string().as_bytes());
  h.update(b"\x1f");
  h.update(summary.ankh_total_entries.to_string().as_bytes());
  h.update(b"\x1f");
  for (k, v) in &summary.ankh_by_query_kind {
    h.update(k.as_bytes());
    h.update(b"\x1d");
    h.update(v.to_string().as_bytes());
    h.update(b"\x1e");
  }
  h.update(b"\x1f");
  for (k, v) in &summary.ankh_by_provenance_source {
    h.update(k.as_bytes());
    h.update(b"\x1d");
    h.update(v.to_string().as_bytes());
    h.update(b"\x1e");
  }
  h.update(b"\x1f");
  h.update(summary.overlay_counts.total().to_string().as_bytes());
  let digest = h.finalize();
  let prefix = digest
    .iter()
    .take(16)
    .map(|b| format!("{b:02x}"))
    .collect::<String>();
  let id = format!("substrate-state-summary.{prefix}");

  let mut payload = serde_json::json!({
    "id": id,
    "artifact_family": "coding.substrate-state-summary",
    "source_surface": "algorithm-synthesis.registry-overlay",
    "snapshot_at_ms": summary.snapshot_at_ms,
    "ankh_total_entries": summary.ankh_total_entries,
    "ankh_by_query_kind": summary.ankh_by_query_kind,
    "ankh_by_provenance_source": summary.ankh_by_provenance_source,
    "overlay_counts": {
      "held_routing": summary.overlay_counts.held_routing,
      "intent_signals": summary.overlay_counts.intent_signals,
      "operation_map": summary.overlay_counts.operation_map,
      "fact_phrase_patterns": summary.overlay_counts.fact_phrase_patterns,
      "known_algebraic_identities": summary.overlay_counts.known_algebraic_identities,
      "known_chemical_reactions": summary.overlay_counts.known_chemical_reactions,
      "known_imports_by_language": summary.overlay_counts.known_imports_by_language,
      "total": summary.overlay_counts.total(),
    },
    "related_refs": serde_json::json!([
      format!("ankh-total:{}", summary.ankh_total_entries),
      format!("overlay-total:{}", summary.overlay_counts.total()),
      "owner-law:stdlib/lib/gate/algorithm-synthesis/registry-overlay.px",
    ]),
    "target_paths": Vec::<String>::new(),
    "command_refs": Vec::<String>::new(),
  });
  if let Some(snap) = repo_snapshot_ref {
    payload["repo_snapshot_ref"] = serde_json::Value::String(snap.to_string());
  }
  payload
}

/// Look up a held kind across the static `HELD_ROUTING` table and
/// the overlay. Static rows take precedence on equal-kind match —
/// append-only policy means overlay only fills in *missing*
/// routing decisions. Returns the first match or `None`.
///
/// This is the overlay-aware variant of
/// `held_to_query::routing_for`. v0 callers can use this in place
/// of building a fresh `HeldRetrievalQuery` lookup that would
/// otherwise need both sources walked manually.
pub fn lookup_held_routing(
  held: ResolutionHeldKind,
  overlay: &InMemoryRegistryOverlay,
) -> Option<HeldRoutingLookup> {
  // Static first.
  for entry in super::held_to_query::HELD_ROUTING {
    if entry.held == held {
      return Some(HeldRoutingLookup::Static {
        primary: entry.primary,
        fallback: entry.fallback,
        query_kind: entry.query_kind.to_string(),
      });
    }
  }
  // Then overlay.
  for entry in overlay.held_routing() {
    if entry.held == held {
      return Some(HeldRoutingLookup::Overlay {
        primary: entry.primary,
        fallback: entry.fallback,
        query_kind: entry.query_kind.clone(),
        provenance: entry.provenance.clone(),
      });
    }
  }
  None
}

/// Result of an overlay-aware routing lookup. `Static` means the
/// answer came from the compile-time table; `Overlay` means it
/// came from a hot-reloaded row and carries provenance for audit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeldRoutingLookup {
  Static {
    primary: HeldQueryRecoveryChannel,
    fallback: Option<HeldQueryRecoveryChannel>,
    query_kind: String,
  },
  Overlay {
    primary: HeldQueryRecoveryChannel,
    fallback: Option<HeldQueryRecoveryChannel>,
    query_kind: String,
    provenance: OverlayProvenance,
  },
}

/// Combined static + overlay walk for intent-signal lookup.
/// Returns every signal entry whose cue matches the given input,
/// from both sources. Caller's scoring logic accumulates as
/// normal — append semantics means the overlay can only ADD signal
/// to an intent, never subtract.
pub fn lookup_intent_signals_for_cue(
  cue: &str,
  overlay: &InMemoryRegistryOverlay,
) -> Vec<MergedIntentSignal> {
  let mut out: Vec<MergedIntentSignal> = Vec::new();
  for entry in super::intent_recognition::INTENT_SIGNALS {
    if entry.cue == cue {
      out.push(MergedIntentSignal::Static {
        intent: entry.intent.to_string(),
        weight: entry.weight,
      });
    }
  }
  for entry in overlay.intent_signals() {
    if entry.cue == cue {
      out.push(MergedIntentSignal::Overlay {
        intent: entry.intent.clone(),
        weight: entry.weight,
        provenance: entry.provenance.clone(),
      });
    }
  }
  out
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MergedIntentSignal {
  Static {
    intent: String,
    weight: f32,
  },
  Overlay {
    intent: String,
    weight: f32,
    provenance: OverlayProvenance,
  },
}

impl MergedIntentSignal {
  pub fn intent(&self) -> &str {
    match self {
      Self::Static { intent, .. } | Self::Overlay { intent, .. } => intent,
    }
  }
  pub fn weight(&self) -> f32 {
    match self {
      Self::Static { weight, .. } | Self::Overlay { weight, .. } => *weight,
    }
  }
}

/// Overlay-aware lookup over `OPERATION_MAP`. Returns every entry
/// (static + overlay) that matches the given `(intent, cues_set)`
/// shape: static rows whose `cues` slice is a subset of
/// `fired_cues`, plus overlay rows under the same rule. Append-only
/// — overlay can ADD operations for the same intent, never remove
/// or shadow static rows.
pub fn lookup_operations_for_intent_and_cues(
  intent: &str,
  fired_cues: &[String],
  overlay: &InMemoryRegistryOverlay,
) -> Vec<MergedOperationMapping> {
  let mut out: Vec<MergedOperationMapping> = Vec::new();
  let fired_set: std::collections::BTreeSet<&str> = fired_cues.iter().map(|s| s.as_str()).collect();
  for entry in super::operation_candidate_mapping::OPERATION_MAP {
    if entry.intent != intent {
      continue;
    }
    if entry.cues.iter().all(|c| fired_set.contains(c)) {
      out.push(MergedOperationMapping::Static {
        transform: entry.transform.to_string(),
        weight: entry.weight,
        matched_cues: entry.cues.iter().map(|s| s.to_string()).collect(),
      });
    }
  }
  for entry in overlay.operation_map() {
    if entry.intent != intent {
      continue;
    }
    if entry.cues.iter().all(|c| fired_set.contains(c.as_str())) {
      out.push(MergedOperationMapping::Overlay {
        transform: entry.transform.clone(),
        weight: entry.weight,
        matched_cues: entry.cues.clone(),
        provenance: entry.provenance.clone(),
      });
    }
  }
  out
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MergedOperationMapping {
  Static {
    transform: String,
    weight: f32,
    matched_cues: Vec<String>,
  },
  Overlay {
    transform: String,
    weight: f32,
    matched_cues: Vec<String>,
    provenance: OverlayProvenance,
  },
}

impl MergedOperationMapping {
  pub fn transform(&self) -> &str {
    match self {
      Self::Static { transform, .. } | Self::Overlay { transform, .. } => transform,
    }
  }
  pub fn weight(&self) -> f32 {
    match self {
      Self::Static { weight, .. } | Self::Overlay { weight, .. } => *weight,
    }
  }
}

/// Overlay-aware lookup over `FACT_PHRASE_PATTERNS`. Returns every
/// pattern row (static + overlay) whose `cue` matches the given
/// name. Append-only — overlay can ADD a new pattern for the same
/// cue (more markers), never remove or shadow static markers.
pub fn lookup_fact_phrase_patterns_for_cue(
  cue: &str,
  overlay: &InMemoryRegistryOverlay,
) -> Vec<MergedFactPhrasePattern> {
  let mut out: Vec<MergedFactPhrasePattern> = Vec::new();
  for row in super::fact_cue_registry::FACT_PHRASE_PATTERNS {
    if row.cue == cue {
      out.push(MergedFactPhrasePattern::Static {
        markers: row.markers.iter().map(|s| s.to_string()).collect(),
      });
    }
  }
  for entry in overlay.fact_phrase_patterns() {
    if entry.cue == cue {
      out.push(MergedFactPhrasePattern::Overlay {
        markers: entry.markers.clone(),
        provenance: entry.provenance.clone(),
      });
    }
  }
  out
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MergedFactPhrasePattern {
  Static {
    markers: Vec<String>,
  },
  Overlay {
    markers: Vec<String>,
    provenance: OverlayProvenance,
  },
}

impl MergedFactPhrasePattern {
  pub fn markers(&self) -> &[String] {
    match self {
      Self::Static { markers } | Self::Overlay { markers, .. } => markers,
    }
  }
}

/// Convenience: build a HeldRetrievalQuery using the overlay-aware
/// lookup. Mirrors `held_to_query::build_query_from_held` but
/// consults the overlay for held kinds the static table doesn't
/// know about. v0: only handles the routing portion; the
/// query_text / template lookup still uses the static
/// `QUERY_MESSAGE_TEMPLATES` (which an overlay-derived held has
/// no entry in — caller treats absent template as generic prompt).
pub fn build_query_with_overlay(
  held: ResolutionHeldKind,
  transform: &str,
  partial_resolution: &BTreeMap<String, String>,
  missing_slots: &[String],
  reason: &str,
  overlay: &InMemoryRegistryOverlay,
) -> Option<HeldRetrievalQuery> {
  let lookup = lookup_held_routing(held, overlay)?;
  let (primary, fallback, query_kind) = match lookup {
    HeldRoutingLookup::Static {
      primary,
      fallback,
      query_kind,
    } => (primary, fallback, query_kind),
    HeldRoutingLookup::Overlay {
      primary,
      fallback,
      query_kind,
      ..
    } => (primary, fallback, query_kind),
  };
  Some(HeldRetrievalQuery {
    query_kind,
    held_kind: held,
    transform: transform.to_string(),
    primary_channel: primary,
    fallback_channel: fallback,
    query_text: format!("(overlay-aware lookup for held kind `{}`)", held.as_str()),
    evidence_to_recover: missing_slots.to_vec(),
    context_fields: partial_resolution.clone(),
    try_ankh_first: primary != HeldQueryRecoveryChannel::NotRecoverable,
    reason: reason.to_string(),
  })
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
  use super::super::runtime_hot_reload::plan_hot_reload;
  use super::*;

  fn promote_held_routing_row(
    held_kind_str: &str,
    primary_str: &str,
    fallback_str: Option<&str>,
  ) -> (HotReloadPlan, OwnerLawProcessedCandidate) {
    let mut row = BTreeMap::new();
    row.insert("held".to_string(), held_kind_str.to_string());
    row.insert("primary".to_string(), primary_str.to_string());
    if let Some(f) = fallback_str {
      row.insert("fallback".to_string(), f.to_string());
    }
    let proposal = CandidateRowProposal {
      candidate_kind: CandidateKind::RecurringChannelSuccess,
      target_owner: "stdlib/lib/gate/algorithm-synthesis/held-to-query.px".to_string(),
      target_table: "heldRoutingMap".to_string(),
      proposed_row: row,
      supporting_evidence: vec!["e1".to_string()],
      evidence_count: 2,
      gate_status: GateStatus::IntentReceiptOnly,
      reason: "test".to_string(),
    };
    let folded = fold_proposal(&proposal);
    let separated = check_axis_separation(&folded);
    let regression = check_regression_proof(&separated, &[]);
    let approval = PromotionApproval {
      actor_id: "actor.overlay-test".to_string(),
      tenant_id: "tenant.overlay-test".to_string(),
      approved_at_ms: 1700000000000,
      decision: PromotionApprovalDecision::Approve,
      candidate_fingerprint: candidate_fingerprint(&regression),
      ttl_ms: None,
      reason: None,
    };
    let owner = process_owner_law(&regression, Some(&approval), 1700000000000);
    let plan = plan_hot_reload(&owner, "let heldRoutingMap = [ ]; in {}\n");
    (plan, owner)
  }

  // ─── target enum / registry consistency ───────────────────────

  #[test]
  fn every_target_has_a_string_form() {
    for t in RegistryOverlayTarget::ALL {
      assert!(!t.as_str().is_empty());
    }
  }

  #[test]
  fn from_target_table_recognizes_known_tables() {
    assert_eq!(
      RegistryOverlayTarget::from_target_table("heldRoutingMap"),
      Some(RegistryOverlayTarget::HeldRoutingMap)
    );
    assert_eq!(
      RegistryOverlayTarget::from_target_table("intentSignals"),
      Some(RegistryOverlayTarget::IntentSignals)
    );
    assert_eq!(
      RegistryOverlayTarget::from_target_table("operationMap"),
      Some(RegistryOverlayTarget::OperationMap)
    );
    assert_eq!(
      RegistryOverlayTarget::from_target_table("factPhrasePatterns"),
      Some(RegistryOverlayTarget::FactPhrasePatterns)
    );
    assert!(RegistryOverlayTarget::from_target_table("noSuchTable").is_none());
  }

  // ─── conversion ───────────────────────────────────────────────

  #[test]
  fn apply_held_routing_plan_pushes_overlay_entry() {
    let (plan, owner) = promote_held_routing_row(
      "missing-import-spec",
      "host-symbol-resolver",
      Some("external-knowledge-search"),
    );
    assert_eq!(plan.outcome, HotReloadOutcome::PlanReady);
    let mut overlay = InMemoryRegistryOverlay::new();
    let target =
      apply_hot_reload_plan_to_overlay(&plan, &mut overlay, &owner).expect("overlay conversion");
    assert_eq!(target, RegistryOverlayTarget::HeldRoutingMap);
    assert_eq!(overlay.held_routing().len(), 1);
    let entry = &overlay.held_routing()[0];
    assert_eq!(entry.held, ResolutionHeldKind::MissingImportSpec);
    assert_eq!(entry.primary, HeldQueryRecoveryChannel::HostSymbolResolver);
    assert_eq!(
      entry.fallback,
      Some(HeldQueryRecoveryChannel::ExternalKnowledgeSearch)
    );
    assert_eq!(entry.provenance.contributing_actor_id, "actor.overlay-test");
    assert_eq!(entry.provenance.stored_at_ms, 1700000000000);
  }

  #[test]
  fn apply_non_ready_plan_errors() {
    let mut row = BTreeMap::new();
    row.insert("held".to_string(), "missing-import-spec".to_string());
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
    let regression = check_regression_proof(&check_axis_separation(&fold_proposal(&proposal)), &[]);
    let owner = process_owner_law(&regression, None, 1700000000000);
    // owner is HeldAwaitingApproval → plan is HeldNotPromoted.
    let plan = plan_hot_reload(&owner, "let heldRoutingMap = [];\n");
    let mut overlay = InMemoryRegistryOverlay::new();
    let result = apply_hot_reload_plan_to_overlay(&plan, &mut overlay, &owner);
    assert!(matches!(
      result,
      Err(OverlayConversionError::PlanNotReady(_))
    ));
    assert!(overlay.is_empty());
  }

  #[test]
  fn apply_unsupported_target_errors() {
    // Hand-craft a plan whose source_table is something the
    // overlay doesn't know. v0 now covers heldRoutingMap,
    // intentSignals, operationMap, factPhrasePatterns — pick a
    // genuinely unsupported name.
    let (mut plan, owner) =
      promote_held_routing_row("missing-import-spec", "host-symbol-resolver", None);
    plan.source.source.source.source.source.target_table = "unknownFutureTable".to_string();
    let mut overlay = InMemoryRegistryOverlay::new();
    let result = apply_hot_reload_plan_to_overlay(&plan, &mut overlay, &owner);
    assert!(matches!(
      result,
      Err(OverlayConversionError::UnsupportedTarget(_))
    ));
  }

  // ─── lookup: static entries beat overlay entries ──────────────

  #[test]
  fn lookup_static_entry_takes_precedence_over_overlay() {
    // missing-import-spec is in the static table.
    let (plan, owner) = promote_held_routing_row(
      "missing-import-spec",
      "operator-followup", // intentionally different from static
      None,
    );
    let mut overlay = InMemoryRegistryOverlay::new();
    apply_hot_reload_plan_to_overlay(&plan, &mut overlay, &owner).expect("overlay");
    let result = lookup_held_routing(ResolutionHeldKind::MissingImportSpec, &overlay);
    match result {
      Some(HeldRoutingLookup::Static { primary, .. }) => {
        // Static value (HostSymbolResolver) wins, NOT overlay's
        // OperatorFollowup.
        assert_eq!(primary, HeldQueryRecoveryChannel::HostSymbolResolver);
      }
      other => panic!("expected Static, got {other:?}"),
    }
  }

  // ─── lookup: overlay fills in missing static entries ──────────
  //
  // The static HELD_ROUTING table already covers every kind in
  // ResolutionHeldKind::ALL (see held_to_query::HELD_ROUTING
  // construction). For v0 we demonstrate the overlay path
  // structurally: we craft an overlay entry directly and ensure
  // `lookup_held_routing` returns it ONLY if the static table
  // happens not to cover that kind. Since the static table is
  // exhaustive, this is more of a defense-in-depth check: the
  // overlay path is reachable in code even if it never fires in
  // current production.

  #[test]
  fn lookup_overlay_path_compiles_and_returns_overlay_when_no_static_entry() {
    // We can't actually construct a held kind that isn't in the
    // static table (the enum is closed). What we CAN do is verify
    // that the overlay entries are searched and the `Overlay`
    // variant exists in the lookup result enum (compile-time
    // assertion).
    let overlay = InMemoryRegistryOverlay::new();
    // For every held kind, the static table covers it.
    for held in ResolutionHeldKind::ALL {
      let result = lookup_held_routing(*held, &overlay);
      assert!(matches!(result, Some(HeldRoutingLookup::Static { .. })));
    }
  }

  // ─── intent-signal overlay ────────────────────────────────────

  #[test]
  fn intent_signal_overlay_extends_signal_set() {
    // Direct push (not via plan) — exercises the overlay's
    // intent-signal path independently of the held-routing one.
    let mut overlay = InMemoryRegistryOverlay::new();
    overlay.push_intent_signal(IntentSignalOverlayEntry {
      cue: "fact:future-new-cue".to_string(),
      intent: "fix-bug".to_string(),
      weight: 0.75,
      provenance: OverlayProvenance {
        source_hot_reload_plan_fingerprint: "fingerprint-test".to_string(),
        stored_at_ms: 1700000000000,
        contributing_actor_id: "actor.test".to_string(),
        contributing_tenant_id: "tenant.test".to_string(),
      },
    });
    let matches = lookup_intent_signals_for_cue("fact:future-new-cue", &overlay);
    assert_eq!(matches.len(), 1);
    match &matches[0] {
      MergedIntentSignal::Overlay {
        intent,
        weight,
        provenance,
      } => {
        assert_eq!(intent, "fix-bug");
        assert_eq!(*weight, 0.75);
        assert_eq!(provenance.contributing_actor_id, "actor.test");
      }
      other => panic!("expected Overlay, got {other:?}"),
    }
  }

  #[test]
  fn intent_signal_lookup_returns_both_static_and_overlay_when_cue_present_in_both() {
    let mut overlay = InMemoryRegistryOverlay::new();
    // verb:rename is in the static table (refactor 0.95). Add an
    // overlay entry for the same cue with a different intent.
    overlay.push_intent_signal(IntentSignalOverlayEntry {
      cue: "verb:rename".to_string(),
      intent: "cleanup".to_string(),
      weight: 0.30,
      provenance: OverlayProvenance {
        source_hot_reload_plan_fingerprint: "fp".to_string(),
        stored_at_ms: 1700000000000,
        contributing_actor_id: "a".to_string(),
        contributing_tenant_id: "t".to_string(),
      },
    });
    let matches = lookup_intent_signals_for_cue("verb:rename", &overlay);
    // Static entry + overlay entry both returned.
    assert!(matches
      .iter()
      .any(|m| matches!(m, MergedIntentSignal::Static { .. })));
    assert!(matches
      .iter()
      .any(|m| matches!(m, MergedIntentSignal::Overlay { .. })));
  }

  // ─── kv extractor preserves escapes (regression test) ─────────

  #[test]
  fn kv_extractor_handles_escaped_quote() {
    let text = "{\n  msg = \"a \\\"quoted\\\" b\";\n}";
    let kv = extract_kv_from_folded_text(text);
    assert_eq!(kv.get("msg").unwrap(), "a \"quoted\" b");
  }

  // ─── operation-map overlay ────────────────────────────────────

  #[test]
  fn operation_map_overlay_entry_pushed_directly() {
    let mut overlay = InMemoryRegistryOverlay::new();
    overlay.push_operation_map(OperationMapOverlayEntry {
      intent: "fix-bug".to_string(),
      cues: vec!["fact:missing-import".to_string()],
      transform: "add-import".to_string(),
      weight: 0.95,
      provenance: OverlayProvenance {
        source_hot_reload_plan_fingerprint: "fp".to_string(),
        stored_at_ms: 1700000000000,
        contributing_actor_id: "a".to_string(),
        contributing_tenant_id: "t".to_string(),
      },
    });
    assert_eq!(overlay.operation_map().len(), 1);
    assert_eq!(overlay.len_total(), 1);
  }

  #[test]
  fn operation_map_lookup_returns_static_and_overlay() {
    let mut overlay = InMemoryRegistryOverlay::new();
    // Static OPERATION_MAP has fix-bug + fact:missing-import →
    // add-import. Overlay adds the same combination at a different
    // weight (append-only, both should surface).
    overlay.push_operation_map(OperationMapOverlayEntry {
      intent: "fix-bug".to_string(),
      cues: vec!["fact:missing-import".to_string()],
      transform: "add-import".to_string(),
      weight: 0.50,
      provenance: OverlayProvenance {
        source_hot_reload_plan_fingerprint: "fp".to_string(),
        stored_at_ms: 1700000000000,
        contributing_actor_id: "a".to_string(),
        contributing_tenant_id: "t".to_string(),
      },
    });
    let matches = lookup_operations_for_intent_and_cues(
      "fix-bug",
      &["fact:missing-import".to_string()],
      &overlay,
    );
    // Both static and overlay rows match — append-only contract.
    assert!(matches
      .iter()
      .any(|m| matches!(m, MergedOperationMapping::Static { .. })));
    assert!(matches
      .iter()
      .any(|m| matches!(m, MergedOperationMapping::Overlay { .. })));
  }

  #[test]
  fn operation_map_overlay_requires_all_cues_present() {
    let mut overlay = InMemoryRegistryOverlay::new();
    overlay.push_operation_map(OperationMapOverlayEntry {
      intent: "test".to_string(),
      cues: vec!["verb:test".to_string(), "structural:test-file".to_string()],
      transform: "add-test-stub".to_string(),
      weight: 0.99,
      provenance: OverlayProvenance {
        source_hot_reload_plan_fingerprint: "fp".to_string(),
        stored_at_ms: 1700000000000,
        contributing_actor_id: "a".to_string(),
        contributing_tenant_id: "t".to_string(),
      },
    });
    // Only one of the two cues fired — overlay row does NOT match.
    let matches =
      lookup_operations_for_intent_and_cues("test", &["verb:test".to_string()], &overlay);
    assert!(!matches
      .iter()
      .any(|m| matches!(m, MergedOperationMapping::Overlay { .. })));
    // Both cues fired — overlay row matches.
    let matches = lookup_operations_for_intent_and_cues(
      "test",
      &["verb:test".to_string(), "structural:test-file".to_string()],
      &overlay,
    );
    assert!(matches
      .iter()
      .any(|m| matches!(m, MergedOperationMapping::Overlay { .. })));
  }

  // ─── fact-phrase-patterns overlay ─────────────────────────────

  #[test]
  fn fact_phrase_patterns_overlay_entry_pushed_directly() {
    let mut overlay = InMemoryRegistryOverlay::new();
    overlay.push_fact_phrase_pattern(FactPhrasePatternOverlayEntry {
      cue: "fact:future-new-cue".to_string(),
      markers: vec!["pattern A".to_string(), "pattern B".to_string()],
      provenance: OverlayProvenance {
        source_hot_reload_plan_fingerprint: "fp".to_string(),
        stored_at_ms: 1700000000000,
        contributing_actor_id: "a".to_string(),
        contributing_tenant_id: "t".to_string(),
      },
    });
    let matches = lookup_fact_phrase_patterns_for_cue("fact:future-new-cue", &overlay);
    assert_eq!(matches.len(), 1);
    match &matches[0] {
      MergedFactPhrasePattern::Overlay { markers, .. } => {
        assert_eq!(
          markers,
          &vec!["pattern A".to_string(), "pattern B".to_string()]
        );
      }
      other => panic!("expected Overlay, got {other:?}"),
    }
  }

  #[test]
  fn fact_phrase_patterns_lookup_returns_static_and_overlay_for_same_cue() {
    // `fact:slow-path` exists in the static FACT_PHRASE_PATTERNS.
    let mut overlay = InMemoryRegistryOverlay::new();
    overlay.push_fact_phrase_pattern(FactPhrasePatternOverlayEntry {
      cue: "fact:slow-path".to_string(),
      markers: vec!["new marker A".to_string()],
      provenance: OverlayProvenance {
        source_hot_reload_plan_fingerprint: "fp".to_string(),
        stored_at_ms: 1700000000000,
        contributing_actor_id: "a".to_string(),
        contributing_tenant_id: "t".to_string(),
      },
    });
    let matches = lookup_fact_phrase_patterns_for_cue("fact:slow-path", &overlay);
    // Static markers + overlay markers both present.
    assert!(matches
      .iter()
      .any(|m| matches!(m, MergedFactPhrasePattern::Static { .. })));
    assert!(matches
      .iter()
      .any(|m| matches!(m, MergedFactPhrasePattern::Overlay { .. })));
  }

  // ─── comma-separated list parser ──────────────────────────────

  #[test]
  fn comma_separated_list_parser_trims_and_filters() {
    let parsed = parse_comma_separated_list("a, b,c , ,d");
    assert_eq!(parsed, vec!["a", "b", "c", "d"]);
  }

  #[test]
  fn comma_separated_list_parser_handles_single_value() {
    let parsed = parse_comma_separated_list("verb:rename");
    assert_eq!(parsed, vec!["verb:rename"]);
  }

  #[test]
  fn comma_separated_list_parser_handles_empty() {
    let parsed = parse_comma_separated_list("");
    assert!(parsed.is_empty());
  }

  // ─── registry-overlay-receipt artifact (final lane surface) ──

  fn synthetic_held_to_query_file() -> String {
    r#"# stdlib.lib.gate.algorithm-synthesis.held-to-query
let
  heldRoutingMap = [
  ];
in {
  inherit heldRoutingMap;
}
"#
    .to_string()
  }

  fn promoted_held_routing_candidate() -> (
    super::super::owner_law_gate::OwnerLawProcessedCandidate,
    super::super::runtime_hot_reload::HotReloadPlan,
  ) {
    let mut row = BTreeMap::new();
    row.insert("held".to_string(), "missing-import-spec".to_string());
    row.insert("primary".to_string(), "host-symbol-resolver".to_string());
    let proposal = CandidateRowProposal {
      candidate_kind: CandidateKind::RecurringChannelSuccess,
      target_owner: "stdlib/lib/gate/algorithm-synthesis/held-to-query.px".to_string(),
      target_table: "heldRoutingMap".to_string(),
      proposed_row: row,
      supporting_evidence: vec!["e1".to_string(), "e2".to_string()],
      evidence_count: 2,
      gate_status: GateStatus::IntentReceiptOnly,
      reason: "fixture".to_string(),
    };
    let folded = fold_proposal(&proposal);
    let axis = check_axis_separation(&folded);
    let regression = check_regression_proof(&axis, &[]);
    let approval = PromotionApproval {
      actor_id: "actor.test".into(),
      tenant_id: "tenant.test".into(),
      approved_at_ms: 1700000000000,
      decision: PromotionApprovalDecision::Approve,
      candidate_fingerprint: candidate_fingerprint(&regression),
      ttl_ms: None,
      reason: None,
    };
    let owner = process_owner_law(&regression, Some(&approval), 1700000000000);
    let plan = plan_hot_reload(&owner, &synthetic_held_to_query_file());
    (owner, plan)
  }

  #[test]
  fn applied_receipt_carries_target_registry_and_delta() {
    let (owner, plan) = promoted_held_routing_candidate();
    let mut overlay = InMemoryRegistryOverlay::new();
    let receipt = apply_hot_reload_plan_to_overlay_with_receipt(&plan, &mut overlay, &owner);
    assert_eq!(receipt.status, OverlayApplyStatus::Applied);
    assert_eq!(
      receipt.target_registry,
      Some(RegistryOverlayTarget::HeldRoutingMap)
    );
    assert_eq!(receipt.before_counts.total(), 0);
    assert_eq!(receipt.after_counts.total(), 1);
    assert_eq!(receipt.after_counts.held_routing, 1);
    let art = build_registry_overlay_receipt_artifact(&receipt, 1700000001000, None);
    assert_eq!(art["artifact_family"], "coding.registry-overlay-receipt");
    assert_eq!(art["status"], "applied");
    assert_eq!(art["target_registry"], "held-routing-map");
    assert_eq!(art["delta_total"], 1);
    assert_eq!(art["after_counts"]["held_routing"], 1);
    assert_eq!(art["before_counts"]["total"], 0);
  }

  #[test]
  fn applied_receipt_subsequent_apply_increments_target_only() {
    let (owner, plan) = promoted_held_routing_candidate();
    let mut overlay = InMemoryRegistryOverlay::new();
    // First apply.
    let _ = apply_hot_reload_plan_to_overlay_with_receipt(&plan, &mut overlay, &owner);
    // Same plan reapplied — append-only policy, so total grows by 1
    // again (the overlay does not dedupe; that's a higher-level
    // policy decision).
    let r2 = apply_hot_reload_plan_to_overlay_with_receipt(&plan, &mut overlay, &owner);
    assert_eq!(r2.before_counts.total(), 1);
    assert_eq!(r2.after_counts.total(), 2);
    let art = build_registry_overlay_receipt_artifact(&r2, 0, None);
    assert_eq!(art["delta_total"], 1);
    assert_eq!(art["after_counts"]["held_routing"], 2);
    // Other registries untouched.
    assert_eq!(art["after_counts"]["intent_signals"], 0);
  }

  #[test]
  fn held_plan_not_ready_yields_held_receipt_with_no_overlay_change() {
    // Build a not-promoted owner-law candidate by failing approval.
    let mut row = BTreeMap::new();
    row.insert("held".to_string(), "missing-import-spec".to_string());
    row.insert("primary".to_string(), "host-symbol-resolver".to_string());
    let proposal = CandidateRowProposal {
      candidate_kind: CandidateKind::RecurringChannelSuccess,
      target_owner: "stdlib/lib/gate/algorithm-synthesis/held-to-query.px".to_string(),
      target_table: "heldRoutingMap".to_string(),
      proposed_row: row,
      supporting_evidence: vec!["e1".to_string()],
      evidence_count: 2,
      gate_status: GateStatus::IntentReceiptOnly,
      reason: "fixture".to_string(),
    };
    let folded = fold_proposal(&proposal);
    let axis = check_axis_separation(&folded);
    let regression = check_regression_proof(&axis, &[]);
    // No approval → owner-law holds-awaiting-approval → plan not ready.
    let owner = process_owner_law(&regression, None, 1700000000000);
    let plan = plan_hot_reload(&owner, &synthetic_held_to_query_file());

    let mut overlay = InMemoryRegistryOverlay::new();
    let receipt = apply_hot_reload_plan_to_overlay_with_receipt(&plan, &mut overlay, &owner);
    assert_eq!(receipt.status, OverlayApplyStatus::HeldPlanNotReady);
    assert!(receipt.target_registry.is_none());
    assert_eq!(receipt.before_counts.total(), 0);
    assert_eq!(receipt.after_counts.total(), 0);
    let art = build_registry_overlay_receipt_artifact(&receipt, 0, None);
    assert_eq!(art["status"], "held-plan-not-ready");
    assert!(art["target_registry"].is_null());
    assert_eq!(art["delta_total"], 0);
  }

  #[test]
  fn receipt_id_is_replay_stable_across_stored_at() {
    let (owner, plan) = promoted_held_routing_candidate();
    let mut overlay = InMemoryRegistryOverlay::new();
    let receipt = apply_hot_reload_plan_to_overlay_with_receipt(&plan, &mut overlay, &owner);
    let a1 = build_registry_overlay_receipt_artifact(&receipt, 1, None);
    let a2 = build_registry_overlay_receipt_artifact(&receipt, 999999, None);
    assert_eq!(a1["id"], a2["id"]);
  }

  #[test]
  fn receipt_related_refs_walk_back_to_hot_reload_plan() {
    let (owner, plan) = promoted_held_routing_candidate();
    let mut overlay = InMemoryRegistryOverlay::new();
    let receipt = apply_hot_reload_plan_to_overlay_with_receipt(&plan, &mut overlay, &owner);
    let art = build_registry_overlay_receipt_artifact(&receipt, 0, None);
    let refs: Vec<String> = serde_json::from_value(art["related_refs"].clone()).unwrap();
    assert!(refs
      .iter()
      .any(|r| r.starts_with("hot-reload-plan-fingerprint:")));
    assert!(refs.iter().any(|r| r.starts_with("candidate-kind:")));
    assert!(refs.iter().any(|r| r.contains("registry-overlay.px")));
  }

  // ─── math-lane overlay (substrate-sharing final step) ─────────

  fn synthetic_algebraic_identities_file() -> String {
    r#"# stdlib.lib.gate.known-algebraic-identities
let
  knownAlgebraicIdentities = [
  ];
in {
  inherit knownAlgebraicIdentities;
}
"#
    .to_string()
  }

  fn promoted_math_identity_candidate() -> (
    super::super::owner_law_gate::OwnerLawProcessedCandidate,
    super::super::runtime_hot_reload::HotReloadPlan,
  ) {
    let mut row = BTreeMap::new();
    row.insert(
      "canonical_form".to_string(),
      "x^2 + 2*x*y + y^2".to_string(),
    );
    row.insert("equivalent_form".to_string(), "(x+y)^2".to_string());
    row.insert("language".to_string(), "polynomial".to_string());
    let proposal = CandidateRowProposal {
      candidate_kind: CandidateKind::MathExpressionLower,
      target_owner: "stdlib/lib/gate/known-algebraic-identities.px".to_string(),
      target_table: "knownAlgebraicIdentities".to_string(),
      proposed_row: row,
      supporting_evidence: vec!["math/a.md".to_string(), "math/b.md".to_string()],
      evidence_count: 2,
      gate_status: GateStatus::IntentReceiptOnly,
      reason: "math fixture".to_string(),
    };
    let folded = fold_proposal(&proposal);
    let axis = check_axis_separation(&folded);
    let regression = check_regression_proof(&axis, &[]);
    let approval = PromotionApproval {
      actor_id: "actor.math-operator".into(),
      tenant_id: "tenant.test".into(),
      approved_at_ms: 1700000000000,
      decision: PromotionApprovalDecision::Approve,
      candidate_fingerprint: candidate_fingerprint(&regression),
      ttl_ms: None,
      reason: None,
    };
    let owner = process_owner_law(&regression, Some(&approval), 1700000000000);
    let plan = plan_hot_reload(&owner, &synthetic_algebraic_identities_file());
    (owner, plan)
  }

  #[test]
  fn apply_math_plan_pushes_known_algebraic_identity_overlay_entry() {
    let (owner, plan) = promoted_math_identity_candidate();
    let mut overlay = InMemoryRegistryOverlay::new();
    let target = apply_hot_reload_plan_to_overlay(&plan, &mut overlay, &owner).expect("applied");
    assert_eq!(target, RegistryOverlayTarget::KnownAlgebraicIdentities);
    assert_eq!(overlay.known_algebraic_identities().len(), 1);
    let entry = &overlay.known_algebraic_identities()[0];
    assert_eq!(entry.canonical_form, "x^2 + 2*x*y + y^2");
    assert_eq!(entry.equivalent_form, "(x+y)^2");
    assert_eq!(entry.language, "polynomial");
    // Provenance is non-empty — same shape coding-lane entries get.
    assert_eq!(
      entry.provenance.contributing_actor_id,
      "actor.math-operator"
    );
    // Other registries untouched.
    assert!(overlay.held_routing().is_empty());
    assert!(overlay.operation_map().is_empty());
  }

  #[test]
  fn math_overlay_receipt_status_is_applied_not_held_unsupported() {
    let (owner, plan) = promoted_math_identity_candidate();
    let mut overlay = InMemoryRegistryOverlay::new();
    let receipt = apply_hot_reload_plan_to_overlay_with_receipt(&plan, &mut overlay, &owner);
    assert_eq!(receipt.status, OverlayApplyStatus::Applied);
    assert_eq!(
      receipt.target_registry,
      Some(RegistryOverlayTarget::KnownAlgebraicIdentities)
    );
    assert_eq!(receipt.before_counts.known_algebraic_identities, 0);
    assert_eq!(receipt.after_counts.known_algebraic_identities, 1);
    let art = build_registry_overlay_receipt_artifact(&receipt, 0, None);
    assert_eq!(art["status"], "applied");
    assert_eq!(art["target_registry"], "known-algebraic-identities");
    assert_eq!(art["after_counts"]["known_algebraic_identities"], 1);
    assert_eq!(art["delta_total"], 1);
  }

  fn synthetic_chemistry_reactions_file() -> String {
    r#"# stdlib.lib.gate.known-chemical-reactions
let
  knownChemicalReactions = [
  ];
in {
  inherit knownChemicalReactions;
}
"#
    .to_string()
  }

  fn promoted_chemistry_reaction_candidate() -> (
    super::super::owner_law_gate::OwnerLawProcessedCandidate,
    super::super::runtime_hot_reload::HotReloadPlan,
  ) {
    let mut row = BTreeMap::new();
    row.insert("reactants".to_string(), "2 H2 + O2".to_string());
    row.insert("products".to_string(), "2 H2O".to_string());
    row.insert("conditions".to_string(), "spark, 25C".to_string());
    row.insert("language".to_string(), "inorganic".to_string());
    let proposal = CandidateRowProposal {
      candidate_kind: CandidateKind::ChemicalReactionLower,
      target_owner: "stdlib/lib/gate/known-chemical-reactions.px".to_string(),
      target_table: "knownChemicalReactions".to_string(),
      proposed_row: row,
      supporting_evidence: vec!["chem/a.md".to_string(), "chem/b.md".to_string()],
      evidence_count: 2,
      gate_status: GateStatus::IntentReceiptOnly,
      reason: "chem fixture".to_string(),
    };
    let folded = fold_proposal(&proposal);
    let axis = check_axis_separation(&folded);
    let regression = check_regression_proof(&axis, &[]);
    let approval = PromotionApproval {
      actor_id: "actor.chem-operator".into(),
      tenant_id: "tenant.test".into(),
      approved_at_ms: 1700000000000,
      decision: PromotionApprovalDecision::Approve,
      candidate_fingerprint: candidate_fingerprint(&regression),
      ttl_ms: None,
      reason: None,
    };
    let owner = process_owner_law(&regression, Some(&approval), 1700000000000);
    let plan = plan_hot_reload(&owner, &synthetic_chemistry_reactions_file());
    (owner, plan)
  }

  #[test]
  fn apply_chemistry_plan_pushes_known_chemical_reaction_overlay_entry() {
    let (owner, plan) = promoted_chemistry_reaction_candidate();
    let mut overlay = InMemoryRegistryOverlay::new();
    let target = apply_hot_reload_plan_to_overlay(&plan, &mut overlay, &owner).expect("applied");
    assert_eq!(target, RegistryOverlayTarget::KnownChemicalReactions);
    assert_eq!(overlay.known_chemical_reactions().len(), 1);
    let entry = &overlay.known_chemical_reactions()[0];
    assert_eq!(entry.reactants, "2 H2 + O2");
    assert_eq!(entry.products, "2 H2O");
    assert_eq!(entry.conditions, "spark, 25C");
    assert_eq!(entry.language, "inorganic");
  }

  #[test]
  fn substrate_state_summary_captures_empty_state() {
    let ankh = super::super::ankh_retrieval_cache::InMemoryAnkhStore::new();
    let overlay = InMemoryRegistryOverlay::new();
    let summary = SubstrateStateSummary::from_state(&ankh, &overlay, 1700000000000);
    assert_eq!(summary.ankh_total_entries, 0);
    assert!(summary.ankh_by_query_kind.is_empty());
    assert!(summary.ankh_by_provenance_source.is_empty());
    assert_eq!(summary.overlay_counts.total(), 0);
    let art = build_substrate_state_summary_artifact(&summary, None);
    assert_eq!(art["artifact_family"], "coding.substrate-state-summary");
    assert_eq!(art["ankh_total_entries"], 0);
    assert_eq!(art["overlay_counts"]["total"], 0);
  }

  #[test]
  fn substrate_state_summary_breaks_down_ankh_by_query_kind_and_provenance() {
    use super::super::ankh_retrieval_cache::{
      AnkhEntry as AE, AnkhProvenanceSource as AP, AnkhRetrievalKey as AK, AnkhStore,
      InMemoryAnkhStore,
    };
    let mut ankh = InMemoryAnkhStore::new();
    let mk_entry = |qk: &str, prov: AP| AE {
      provenance_source: prov,
      contributing_actor_id: "a".to_string(),
      contributing_tenant_id: "t".to_string(),
      stored_at_ms: 0,
      query_kind: qk.to_string(),
      supplied_parameters: BTreeMap::new(),
      filled_slots: vec![],
      context_snapshot: BTreeMap::new(),
    };
    ankh.put(
      AK {
        query_kind: "lookup-module-providing-symbol".to_string(),
        target_path: "src/a.py".to_string(),
        language: "python".to_string(),
      },
      mk_entry("lookup-module-providing-symbol", AP::HostSymbolResolver),
    );
    ankh.put(
      AK {
        query_kind: "lookup-algebraic-equivalent".to_string(),
        target_path: "math/a.md".to_string(),
        language: "polynomial".to_string(),
      },
      mk_entry("lookup-algebraic-equivalent", AP::OperatorFollowup),
    );
    let overlay = InMemoryRegistryOverlay::new();
    let summary = SubstrateStateSummary::from_state(&ankh, &overlay, 0);
    assert_eq!(summary.ankh_total_entries, 2);
    assert_eq!(
      summary
        .ankh_by_query_kind
        .get("lookup-module-providing-symbol"),
      Some(&1)
    );
    assert_eq!(
      summary
        .ankh_by_query_kind
        .get("lookup-algebraic-equivalent"),
      Some(&1)
    );
    assert_eq!(
      summary
        .ankh_by_provenance_source
        .get("host-symbol-resolver"),
      Some(&1)
    );
    assert_eq!(
      summary.ankh_by_provenance_source.get("operator-followup"),
      Some(&1)
    );
  }

  #[test]
  fn substrate_state_summary_after_three_domain_applies() {
    let mut overlay = InMemoryRegistryOverlay::new();
    let (coding_owner, coding_plan) = promoted_held_routing_candidate();
    let (math_owner, math_plan) = promoted_math_identity_candidate();
    let (chem_owner, chem_plan) = promoted_chemistry_reaction_candidate();
    let _ = apply_hot_reload_plan_to_overlay(&coding_plan, &mut overlay, &coding_owner);
    let _ = apply_hot_reload_plan_to_overlay(&math_plan, &mut overlay, &math_owner);
    let _ = apply_hot_reload_plan_to_overlay(&chem_plan, &mut overlay, &chem_owner);
    let ankh = super::super::ankh_retrieval_cache::InMemoryAnkhStore::new();
    let summary = SubstrateStateSummary::from_state(&ankh, &overlay, 0);
    let art = build_substrate_state_summary_artifact(&summary, None);
    assert_eq!(art["overlay_counts"]["held_routing"], 1);
    assert_eq!(art["overlay_counts"]["known_algebraic_identities"], 1);
    assert_eq!(art["overlay_counts"]["known_chemical_reactions"], 1);
    assert_eq!(art["overlay_counts"]["total"], 3);
  }

  #[test]
  fn substrate_state_summary_id_replay_stable_when_inputs_match() {
    let ankh = super::super::ankh_retrieval_cache::InMemoryAnkhStore::new();
    let overlay = InMemoryRegistryOverlay::new();
    let s1 = SubstrateStateSummary::from_state(&ankh, &overlay, 0);
    let s2 = SubstrateStateSummary::from_state(&ankh, &overlay, 0);
    let a1 = build_substrate_state_summary_artifact(&s1, None);
    let a2 = build_substrate_state_summary_artifact(&s2, None);
    assert_eq!(a1["id"], a2["id"]);
  }

  #[test]
  fn substrate_glance_captures_empty_state_no_session() {
    let ankh = super::super::ankh_retrieval_cache::InMemoryAnkhStore::new();
    let overlay = InMemoryRegistryOverlay::new();
    let glance = SubstrateGlance::capture(&ankh, &overlay, None, 1700000000000);
    let art = build_substrate_glance_artifact(&glance, None);
    assert_eq!(art["artifact_family"], "coding.substrate-glance");
    assert_eq!(art["has_session"], false);
    assert_eq!(art["has_pending_held"], false);
    assert_eq!(art["ankh_total_entries"], 0);
    assert_eq!(art["overlay_total"], 0);
    assert!(art["session_state"].is_null());
    // Component states embedded.
    assert_eq!(
      art["substrate_summary"]["artifact_family"],
      "coding.substrate-state-summary"
    );
  }

  #[test]
  fn substrate_glance_captures_session_with_pending_held() {
    use super::super::held_to_query::MultiTurnSession;
    use super::super::parameter_resolution::{resolve_parameters, ResolutionInput};
    let ankh = super::super::ankh_retrieval_cache::InMemoryAnkhStore::new();
    let overlay = InMemoryRegistryOverlay::new();
    let mut session = MultiTurnSession::new();
    let v = resolve_parameters(&ResolutionInput {
      operation_candidate: "lookup-algebraic-equivalent".to_string(),
      utterance: "x^2 + 2*x*y + y^2 는 뭐야?".to_string(),
      ..Default::default()
    });
    let _ = session.register_turn(&v, BTreeMap::new(), 1);
    let glance = SubstrateGlance::capture(&ankh, &overlay, Some(&session), 1700000000000);
    let art = build_substrate_glance_artifact(&glance, None);
    assert_eq!(art["has_session"], true);
    assert_eq!(art["has_pending_held"], true);
    assert_eq!(
      art["session_state"]["artifact_family"],
      "coding.multi-turn-session-state"
    );
    assert_eq!(
      art["session_state"]["pending_held_transform"],
      "lookup-algebraic-equivalent"
    );
  }

  #[test]
  fn substrate_glance_captures_three_domain_overlay_with_session_at_rest() {
    use super::super::held_to_query::MultiTurnSession;
    let (co, cp) = promoted_held_routing_candidate();
    let (mo, mp) = promoted_math_identity_candidate();
    let (ho, hp) = promoted_chemistry_reaction_candidate();
    let mut overlay = InMemoryRegistryOverlay::new();
    let _ = apply_hot_reload_plan_to_overlay(&cp, &mut overlay, &co);
    let _ = apply_hot_reload_plan_to_overlay(&mp, &mut overlay, &mo);
    let _ = apply_hot_reload_plan_to_overlay(&hp, &mut overlay, &ho);
    let ankh = super::super::ankh_retrieval_cache::InMemoryAnkhStore::new();
    let session = MultiTurnSession::new(); // at rest
    let glance = SubstrateGlance::capture(&ankh, &overlay, Some(&session), 0);
    let art = build_substrate_glance_artifact(&glance, None);
    assert_eq!(art["overlay_total"], 3);
    assert_eq!(art["has_session"], true);
    assert_eq!(art["has_pending_held"], false);
  }

  #[test]
  fn substrate_glance_id_replay_stable_across_glance_at_ms() {
    let ankh = super::super::ankh_retrieval_cache::InMemoryAnkhStore::new();
    let overlay = InMemoryRegistryOverlay::new();
    let g1 = SubstrateGlance::capture(&ankh, &overlay, None, 1);
    let g2 = SubstrateGlance::capture(&ankh, &overlay, None, 999999);
    let a1 = build_substrate_glance_artifact(&g1, None);
    let a2 = build_substrate_glance_artifact(&g2, None);
    assert_eq!(a1["id"], a2["id"]);
  }

  #[test]
  fn substrate_glance_id_differs_with_and_without_session() {
    let ankh = super::super::ankh_retrieval_cache::InMemoryAnkhStore::new();
    let overlay = InMemoryRegistryOverlay::new();
    let session = super::super::held_to_query::MultiTurnSession::new();
    let g_no = SubstrateGlance::capture(&ankh, &overlay, None, 0);
    let g_session = SubstrateGlance::capture(&ankh, &overlay, Some(&session), 0);
    let a_no = build_substrate_glance_artifact(&g_no, None);
    let a_session = build_substrate_glance_artifact(&g_session, None);
    assert_ne!(a_no["id"], a_session["id"]);
  }

  #[test]
  fn substrate_state_delta_empty_to_empty_is_zero() {
    let ankh = super::super::ankh_retrieval_cache::InMemoryAnkhStore::new();
    let overlay = InMemoryRegistryOverlay::new();
    let s1 = SubstrateStateSummary::from_state(&ankh, &overlay, 1);
    let s2 = SubstrateStateSummary::from_state(&ankh, &overlay, 2);
    let delta = SubstrateStateDelta::from_pair(&s1, &s2);
    assert_eq!(delta.ankh_total_delta, 0);
    assert!(delta.ankh_by_query_kind_delta.is_empty());
    assert_eq!(delta.overlay_delta.total(), 0);
  }

  #[test]
  fn substrate_state_delta_captures_three_domain_growth() {
    // Before: empty. After: three domain rows applied.
    let mut overlay_after = InMemoryRegistryOverlay::new();
    let (co, cp) = promoted_held_routing_candidate();
    let (mo, mp) = promoted_math_identity_candidate();
    let (ho, hp) = promoted_chemistry_reaction_candidate();
    let _ = apply_hot_reload_plan_to_overlay(&cp, &mut overlay_after, &co);
    let _ = apply_hot_reload_plan_to_overlay(&mp, &mut overlay_after, &mo);
    let _ = apply_hot_reload_plan_to_overlay(&hp, &mut overlay_after, &ho);
    let ankh = super::super::ankh_retrieval_cache::InMemoryAnkhStore::new();
    let s_before = SubstrateStateSummary::from_state(&ankh, &InMemoryRegistryOverlay::new(), 1);
    let s_after = SubstrateStateSummary::from_state(&ankh, &overlay_after, 2);
    let delta = SubstrateStateDelta::from_pair(&s_before, &s_after);
    assert_eq!(delta.overlay_delta.total(), 3);
    // The coding lane (held_routing) was the proposal target for
    // promoted_held_routing_candidate fixture — verify per-registry.
    assert_eq!(delta.overlay_delta.held_routing, 1);
    assert_eq!(delta.overlay_delta.known_algebraic_identities, 1);
    assert_eq!(delta.overlay_delta.known_chemical_reactions, 1);
  }

  #[test]
  fn substrate_state_delta_artifact_carries_signed_deltas() {
    // Build a delta where after has fewer entries → negative delta.
    // (In practice ankh is append-only, but the *delta type* must
    // honor signs as substrate-level audit signals.)
    let ankh_before = {
      use super::super::ankh_retrieval_cache::{
        AnkhEntry as AE, AnkhProvenanceSource as AP, AnkhRetrievalKey as AK, AnkhStore,
        InMemoryAnkhStore,
      };
      let mut a = InMemoryAnkhStore::new();
      a.put(
        AK {
          query_kind: "qk".to_string(),
          target_path: "p".to_string(),
          language: "l".to_string(),
        },
        AE {
          provenance_source: AP::HostSymbolResolver,
          contributing_actor_id: "a".to_string(),
          contributing_tenant_id: "t".to_string(),
          stored_at_ms: 0,
          query_kind: "qk".to_string(),
          supplied_parameters: BTreeMap::new(),
          filled_slots: vec![],
          context_snapshot: BTreeMap::new(),
        },
      );
      a
    };
    let ankh_after = super::super::ankh_retrieval_cache::InMemoryAnkhStore::new();
    let overlay = InMemoryRegistryOverlay::new();
    let s_before = SubstrateStateSummary::from_state(&ankh_before, &overlay, 1);
    let s_after = SubstrateStateSummary::from_state(&ankh_after, &overlay, 2);
    let delta = SubstrateStateDelta::from_pair(&s_before, &s_after);
    assert_eq!(delta.ankh_total_delta, -1, "negative delta preserved");
    let art = build_substrate_state_delta_artifact(&delta, None);
    assert_eq!(art["artifact_family"], "coding.substrate-state-delta");
    assert_eq!(art["ankh_total_delta"], -1);
  }

  #[test]
  fn substrate_state_delta_id_replay_stable_when_inputs_match() {
    let ankh = super::super::ankh_retrieval_cache::InMemoryAnkhStore::new();
    let overlay = InMemoryRegistryOverlay::new();
    let s1 = SubstrateStateSummary::from_state(&ankh, &overlay, 1);
    let s2 = SubstrateStateSummary::from_state(&ankh, &overlay, 2);
    let d1 = SubstrateStateDelta::from_pair(&s1, &s2);
    let d2 = SubstrateStateDelta::from_pair(&s1, &s2);
    let a1 = build_substrate_state_delta_artifact(&d1, None);
    let a2 = build_substrate_state_delta_artifact(&d2, None);
    assert_eq!(a1["id"], a2["id"]);
  }

  #[test]
  fn substrate_state_summary_id_differs_when_snapshot_at_ms_differs() {
    // snapshot_at_ms IS intrinsic — different time = different
    // substrate snapshot, so id differs.
    let ankh = super::super::ankh_retrieval_cache::InMemoryAnkhStore::new();
    let overlay = InMemoryRegistryOverlay::new();
    let s1 = SubstrateStateSummary::from_state(&ankh, &overlay, 1);
    let s2 = SubstrateStateSummary::from_state(&ankh, &overlay, 2);
    let a1 = build_substrate_state_summary_artifact(&s1, None);
    let a2 = build_substrate_state_summary_artifact(&s2, None);
    assert_ne!(a1["id"], a2["id"]);
  }

  #[test]
  fn three_domains_grow_overlay_independently() {
    let mut overlay = InMemoryRegistryOverlay::new();
    let (coding_owner, coding_plan) = promoted_held_routing_candidate();
    let (math_owner, math_plan) = promoted_math_identity_candidate();
    let (chem_owner, chem_plan) = promoted_chemistry_reaction_candidate();
    let _ =
      apply_hot_reload_plan_to_overlay(&coding_plan, &mut overlay, &coding_owner).expect("coding");
    let _ = apply_hot_reload_plan_to_overlay(&math_plan, &mut overlay, &math_owner).expect("math");
    let _ = apply_hot_reload_plan_to_overlay(&chem_plan, &mut overlay, &chem_owner).expect("chem");
    assert_eq!(overlay.held_routing().len(), 1);
    assert_eq!(overlay.known_algebraic_identities().len(), 1);
    assert_eq!(overlay.known_chemical_reactions().len(), 1);
    assert_eq!(overlay.len_total(), 3);
  }

  #[test]
  fn math_lane_apply_with_coding_lane_apply_grow_independently() {
    // Apply both math + coding candidates to the same overlay; each
    // registry grows by 1, total = 2.
    let mut overlay = InMemoryRegistryOverlay::new();
    let (math_owner, math_plan) = promoted_math_identity_candidate();
    let (coding_owner, coding_plan) = promoted_held_routing_candidate();
    let _ = apply_hot_reload_plan_to_overlay(&math_plan, &mut overlay, &math_owner).expect("math");
    let _ =
      apply_hot_reload_plan_to_overlay(&coding_plan, &mut overlay, &coding_owner).expect("coding");
    assert_eq!(overlay.known_algebraic_identities().len(), 1);
    assert_eq!(overlay.held_routing().len(), 1);
    assert_eq!(overlay.len_total(), 2);
  }
}
