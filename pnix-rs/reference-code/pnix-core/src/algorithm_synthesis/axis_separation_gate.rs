//! Axis-separation gate — Stage D-v2 of the evolution lane (firewall
//! gate #3).
//!
//! OWNER-LAW (2026-05-12): mirror of
//! `stdlib/lib/gate/algorithm-synthesis/axis-separation-gate.px`.
//! Consumes a `MacroFoldedCandidate` (gate-2 output) and checks
//! whether the proposed row's key set lines up with the target
//! table's schema.
//!
//! Honest v0 outcome: current candidate kinds emit observer-shaped
//! rows (`query_kind`, `observed_primary_channel`, `import_spec`,
//! ...) but the target tables (`heldRoutingMap`,
//! `factPhrasePatterns`) have a *different* row shape (`held`,
//! `primary`, ... ; `cue`, `markers`). So this gate emits
//! `HeldMissingKeys` / `HeldExtraKeys` for every v0 candidate.
//! That is the intended firewall behavior — a candidate cannot
//! be promoted until its row matches the target schema.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use pnix_hash::{Digest, Sha256};

use crate::lang::pnix::parse_expr_to_ast_json;

use super::candidate_row_proposal::GateStatus;
use super::intent_recognition::VALID_INTENTS;
use super::macro_fold_gate::{MacroFoldOutcome, MacroFoldedCandidate};
use super::operation_candidate_mapping::VALID_TRANSFORMS;

/// Possible axis-separation outcomes. Stays byte-identical to
/// `.px` `validAxisOutcomes`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AxisSeparationOutcome {
  AxisVerified,
  HeldMissingKeys,
  HeldExtraKeys,
  HeldUnknownTable,
  HeldInvalidFieldValue,
}

impl AxisSeparationOutcome {
  pub const ALL: &'static [Self] = &[
    Self::AxisVerified,
    Self::HeldMissingKeys,
    Self::HeldExtraKeys,
    Self::HeldUnknownTable,
    Self::HeldInvalidFieldValue,
  ];

  pub fn as_str(self) -> &'static str {
    match self {
      Self::AxisVerified => "axis-verified",
      Self::HeldMissingKeys => "held-missing-keys",
      Self::HeldExtraKeys => "held-extra-keys",
      Self::HeldUnknownTable => "held-unknown-table",
      Self::HeldInvalidFieldValue => "held-invalid-field-value",
    }
  }
}

/// Row schema for a known `.px` target table. Mirror of `.px`
/// `targetTableSchemas` row shape.
#[derive(Debug, Clone, Copy)]
pub struct TargetTableSchema {
  pub target_owner: &'static str,
  pub target_table: &'static str,
  pub required_keys: &'static [&'static str],
  pub optional_keys: &'static [&'static str],
}

pub const TARGET_TABLE_SCHEMAS: &[TargetTableSchema] = &[
  TargetTableSchema {
    target_owner: "stdlib/lib/gate/algorithm-synthesis/held-to-query.px",
    target_table: "heldRoutingMap",
    required_keys: &["held", "primary"],
    optional_keys: &["fallback"],
  },
  TargetTableSchema {
    target_owner: "stdlib/lib/gate/algorithm-synthesis/fact-cue-registry.px",
    target_table: "factPhrasePatterns",
    required_keys: &["cue", "markers"],
    optional_keys: &[],
  },
  TargetTableSchema {
    target_owner: "stdlib/lib/gate/algorithm-synthesis/operation-candidate-mapping.px",
    target_table: "operationMap",
    required_keys: &["intent", "cues", "transform", "weight"],
    optional_keys: &[],
  },
  TargetTableSchema {
    target_owner: "stdlib/lib/gate/known-imports-by-language.px",
    target_table: "knownImportsByLanguage",
    required_keys: &["language", "import_spec"],
    optional_keys: &[],
  },
  TargetTableSchema {
    target_owner: "stdlib/lib/gate/known-algebraic-identities.px",
    target_table: "knownAlgebraicIdentities",
    required_keys: &["canonical_form", "equivalent_form", "language"],
    optional_keys: &[],
  },
  TargetTableSchema {
    target_owner: "stdlib/lib/gate/known-chemical-reactions.px",
    target_table: "knownChemicalReactions",
    required_keys: &["reactants", "products", "conditions", "language"],
    optional_keys: &[],
  },
  // Learned-intent-overlay (user-authored direct-injection lane).
  // Row shape mirrors `intent-recognition.px::mkSignalEntry`:
  // (cue, intent, weight). All required.
  TargetTableSchema {
    target_owner: "stdlib/lib/gate/algorithm-synthesis/learned-intent-overlay.px",
    target_table: "overlayIntentSignals",
    required_keys: &["cue", "intent", "weight"],
    optional_keys: &[],
  },
  // Learned-operation-overlay. Same fields as operationMap.
  // `cues` is the comma-separated flat string form.
  TargetTableSchema {
    target_owner: "stdlib/lib/gate/algorithm-synthesis/learned-operation-overlay.px",
    target_table: "overlayOperationRows",
    required_keys: &["intent", "cues", "transform", "weight"],
    optional_keys: &[],
  },
  // Learned-parameter-overlay. `resolved_fields` is a JSON string.
  TargetTableSchema {
    target_owner: "stdlib/lib/gate/algorithm-synthesis/learned-parameter-overlay.px",
    target_table: "overlayParameterRows",
    required_keys: &["operation_candidate", "resolved_fields"],
    optional_keys: &[],
  },
  // Learned-fact-cue-overlay. `markers` is a comma-separated flat
  // string.
  TargetTableSchema {
    target_owner: "stdlib/lib/gate/algorithm-synthesis/learned-fact-cue-overlay.px",
    target_table: "overlayPhrasePatterns",
    required_keys: &["cue", "markers"],
    optional_keys: &[],
  },
];

fn schema_for(target_owner: &str, target_table: &str) -> Option<&'static TargetTableSchema> {
  TARGET_TABLE_SCHEMAS
    .iter()
    .find(|s| s.target_owner == target_owner && s.target_table == target_table)
}

/// Extract the key set from a folded `.px` attrset AST.
///
/// Gate 2 emits both source text and `folded_ast_json`; Gate 3 uses
/// the AST projection as its primary input so this check is not
/// load-bearing on line-oriented string parsing.
pub fn extract_keys_from_folded_ast_json(ast: &Value) -> Vec<String> {
  let Some(items) = ast
    .get("root")
    .and_then(|root| root.get("items"))
    .and_then(|items| items.as_array())
  else {
    return Vec::new();
  };

  let mut out = Vec::new();
  for item in items {
    if item.get("kind").and_then(|kind| kind.as_str()) != Some("assign") {
      continue;
    }
    let Some(key_path) = item.get("key_path").and_then(|path| path.as_array()) else {
      continue;
    };
    if key_path.len() != 1 {
      continue;
    }
    let Some(key) = key_path.first().and_then(|key| key.as_str()) else {
      continue;
    };
    out.push(key.to_string());
  }
  out
}

/// Extract the key set from a folded `.px` attrset literal text.
/// Reads each line, strips leading whitespace, takes the substring
/// before ` = `. Skips the surrounding `{` / `}` lines and any
/// blank lines.
///
/// This relies on `macro_fold_gate`'s `multi-line-attrset` format
/// invariants — same crate, same authority. Any folder that emits
/// a different format would need its own key-extractor.
///
/// Retained as a compatibility/debug fallback; new gate logic should
/// prefer `extract_keys_from_folded_ast_json`.
pub fn extract_keys_from_folded_text(folded: &str) -> Vec<String> {
  let mut out: Vec<String> = Vec::new();
  for line in folded.lines() {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed == "{" || trimmed == "}" {
      continue;
    }
    let Some(eq_idx) = trimmed.find(" = ") else {
      continue;
    };
    let key = trimmed[..eq_idx].trim();
    if !key.is_empty() {
      out.push(key.to_string());
    }
  }
  out
}

/// The output of the gate. Carries the gate-2 source verbatim plus
/// the axis-check verdict.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AxisSeparatedCandidate {
  pub source: MacroFoldedCandidate,
  pub outcome: AxisSeparationOutcome,
  /// Keys that appear both in the folded row and in
  /// `required_keys ++ optional_keys`. Empty for HeldUnknownTable.
  pub matched_keys: Vec<String>,
  /// Required keys that are NOT in the folded row. Empty unless
  /// outcome is `HeldMissingKeys`.
  pub missing_keys: Vec<String>,
  /// Folded-row keys that are NOT in
  /// `required_keys ++ optional_keys`. Empty unless outcome is
  /// `HeldExtraKeys`.
  pub extra_keys: Vec<String>,
  /// Folded-row fields whose value is outside the target owner's
  /// declared enum. Empty unless outcome is
  /// `HeldInvalidFieldValue`.
  pub invalid_field_values: Vec<FieldValueViolation>,
  pub gate_status: GateStatus,
  pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldValueViolation {
  pub field: String,
  pub value: String,
  pub expected: String,
}

/// Check one folded candidate against its declared target table.
///
/// Outcome priority (first applicable wins):
///   1. `target_owner`/`target_table` not in schema registry →
///      `HeldUnknownTable`
///   2. folded text Held at gate 2 (empty) → propagate as
///      `HeldMissingKeys` with all required keys missing
///   3. any required key absent → `HeldMissingKeys`
///   4. any folded key outside required ∪ optional → `HeldExtraKeys`
///   5. else → `AxisVerified`
///
/// **OWNER-LAW (v0.6.6, 2026-05-15)**: this pure-Rust body is a
/// **byte-equivalent mirror** of the `.px` owner
/// `stdlib/lib/gate/algorithm-synthesis/axis-separation-gate.px::checkAxisSeparationSchema`
/// (v0.6.3.1-4 migration). The `.px` is the **canonical owner**;
/// this Rust body exists as a cycle-free in-process fast-path
/// because `pnix-core` cannot depend on `pnix-eval`.
///
/// The byte-equivalent `.px`-delegating entry is
/// `doghouse_core::algorithm_synthesis_bridge::check_axis_separation_via_px_body`.
/// Drift between the two paths is prevented by the byte-equivalence
/// ratchet test
/// `crates/doghouse-core/tests/check_axis_separation_via_px_body_byte_equivalence.rs`
/// and the full-pipeline test
/// `crates/doghouse-core/tests/firewall_pipeline_dual_path_equivalence.rs`.
pub fn check_axis_separation(folded: &MacroFoldedCandidate) -> AxisSeparatedCandidate {
  let target_owner = folded.source.target_owner.as_str();
  let target_table = folded.source.target_table.as_str();

  let Some(schema) = schema_for(target_owner, target_table) else {
    return AxisSeparatedCandidate {
      source: folded.clone(),
      outcome: AxisSeparationOutcome::HeldUnknownTable,
      matched_keys: Vec::new(),
      missing_keys: Vec::new(),
      extra_keys: Vec::new(),
      invalid_field_values: Vec::new(),
      gate_status: GateStatus::Held,
      reason: format!(
        "no schema registered for ({target_owner}, {target_table}) — register one in `targetTableSchemas` first"
      ),
    };
  };

  // Empty fold (held at gate 2) propagates: every required key is
  // missing.
  if folded.outcome == MacroFoldOutcome::HeldNotFoldable {
    return AxisSeparatedCandidate {
      source: folded.clone(),
      outcome: AxisSeparationOutcome::HeldMissingKeys,
      matched_keys: Vec::new(),
      missing_keys: schema.required_keys.iter().map(|s| s.to_string()).collect(),
      extra_keys: Vec::new(),
      invalid_field_values: Vec::new(),
      gate_status: GateStatus::Held,
      reason: "macro-fold held — propagating axis-separation hold".to_string(),
    };
  }

  let row_keys = folded
    .folded_ast_json
    .as_ref()
    .map(extract_keys_from_folded_ast_json)
    .unwrap_or_else(|| extract_keys_from_folded_text(&folded.folded_source_text));
  let row_set: std::collections::BTreeSet<&str> = row_keys.iter().map(|s| s.as_str()).collect();
  let required_set: std::collections::BTreeSet<&str> =
    schema.required_keys.iter().copied().collect();
  let optional_set: std::collections::BTreeSet<&str> =
    schema.optional_keys.iter().copied().collect();
  let allowed_set: std::collections::BTreeSet<&str> =
    required_set.union(&optional_set).copied().collect();

  let missing: Vec<String> = required_set
    .difference(&row_set)
    .map(|s| s.to_string())
    .collect();
  let extra: Vec<String> = row_set
    .difference(&allowed_set)
    .map(|s| s.to_string())
    .collect();
  let matched: Vec<String> = row_set
    .intersection(&allowed_set)
    .map(|s| s.to_string())
    .collect();

  if !missing.is_empty() {
    return AxisSeparatedCandidate {
      source: folded.clone(),
      outcome: AxisSeparationOutcome::HeldMissingKeys,
      matched_keys: matched,
      missing_keys: missing.clone(),
      extra_keys: extra,
      invalid_field_values: Vec::new(),
      gate_status: GateStatus::Held,
      reason: format!(
        "target `{target_table}` row schema requires keys {:?} which are absent from the folded row",
        missing
      ),
    };
  }
  if !extra.is_empty() {
    return AxisSeparatedCandidate {
      source: folded.clone(),
      outcome: AxisSeparationOutcome::HeldExtraKeys,
      matched_keys: matched,
      missing_keys: Vec::new(),
      extra_keys: extra.clone(),
      invalid_field_values: Vec::new(),
      gate_status: GateStatus::Held,
      reason: format!(
        "folded row contains keys {:?} that are not in target `{target_table}`'s schema",
        extra
      ),
    };
  }

  let invalid_field_values = invalid_field_values_for_target(target_owner, target_table, folded);
  if !invalid_field_values.is_empty() {
    return AxisSeparatedCandidate {
      source: folded.clone(),
      outcome: AxisSeparationOutcome::HeldInvalidFieldValue,
      matched_keys: matched,
      missing_keys: Vec::new(),
      extra_keys: Vec::new(),
      invalid_field_values: invalid_field_values.clone(),
      gate_status: GateStatus::Held,
      reason: format!(
        "folded row contains field values the target `{target_table}` consumer rejects: {:?}",
        invalid_field_values
      ),
    };
  }

  AxisSeparatedCandidate {
    source: folded.clone(),
    outcome: AxisSeparationOutcome::AxisVerified,
    matched_keys: matched,
    missing_keys: Vec::new(),
    extra_keys: Vec::new(),
    invalid_field_values: Vec::new(),
    gate_status: GateStatus::AxisSeparationAttempted,
    reason: format!("row matches schema of `{target_table}`"),
  }
}

fn invalid_field_values_for_target(
  target_owner: &str,
  target_table: &str,
  folded: &MacroFoldedCandidate,
) -> Vec<FieldValueViolation> {
  let is_intent_row = matches!(
    (target_owner, target_table),
    (
      "stdlib/lib/gate/algorithm-synthesis/learned-intent-overlay.px",
      "overlayIntentSignals"
    )
  );
  let is_operation_row = matches!(
    (target_owner, target_table),
    (
      "stdlib/lib/gate/algorithm-synthesis/operation-candidate-mapping.px",
      "operationMap"
    ) | (
      "stdlib/lib/gate/algorithm-synthesis/learned-operation-overlay.px",
      "overlayOperationRows"
    )
  );

  if !(is_intent_row || is_operation_row) {
    return Vec::new();
  }

  let values = extract_string_values_from_folded_candidate(folded);
  let mut violations = Vec::new();

  if is_intent_row {
    if let Some(intent) = values.get("intent") {
      if !VALID_INTENTS.contains(&intent.as_str()) {
        violations.push(FieldValueViolation {
          field: "intent".to_string(),
          value: intent.clone(),
          expected: "intent-recognition.validIntents".to_string(),
        });
      }
    }
  }

  if is_operation_row {
    if let Some(transform) = values.get("transform") {
      if !VALID_TRANSFORMS.contains(&transform.as_str()) {
        violations.push(FieldValueViolation {
          field: "transform".to_string(),
          value: transform.clone(),
          expected: "operation-candidate-mapping.validTransforms".to_string(),
        });
      }
    }
  }

  if let Some(weight) = values.get("weight") {
    if let Some(v) = weight_value_violation(weight) {
      violations.push(v);
    }
  }

  violations
}

// v0.6.3.3 (2026-05-15): exposed `pub` so the doghouse-core
// byte-equivalence harness can compare it field-for-field with the
// `.px` body migration (`axis-separation-gate.px::weightValueViolation`).
// Other v0.6 carriers will follow this pattern; see
// `project-wiki/maps/non-mirror-px-pnixc-meta-migration-plan.md`.
pub fn weight_value_violation(value: &str) -> Option<FieldValueViolation> {
  let Ok(weight) = value.parse::<f32>() else {
    return Some(FieldValueViolation {
      field: "weight".to_string(),
      value: value.to_string(),
      expected: "numeric weight in [0,1]".to_string(),
    });
  };
  if weight.is_finite() && (0.0..=1.0).contains(&weight) {
    return None;
  }
  Some(FieldValueViolation {
    field: "weight".to_string(),
    value: value.to_string(),
    expected: "numeric weight in [0,1]".to_string(),
  })
}

fn extract_string_values_from_folded_candidate(
  folded: &MacroFoldedCandidate,
) -> std::collections::BTreeMap<String, String> {
  if let Some(ast) = folded.folded_ast_json.as_ref() {
    return extract_string_values_from_folded_ast_json(ast);
  }
  extract_string_values_from_folded_text_ast(&folded.folded_source_text)
}

fn extract_string_values_from_folded_text_ast(
  folded: &str,
) -> std::collections::BTreeMap<String, String> {
  let Ok(ast) = parse_expr_to_ast_json(folded) else {
    return std::collections::BTreeMap::new();
  };
  extract_string_values_from_folded_ast_json(&ast)
}

pub fn extract_string_values_from_folded_ast_json(
  ast: &Value,
) -> std::collections::BTreeMap<String, String> {
  let mut out = std::collections::BTreeMap::new();
  let Some(items) = ast
    .get("root")
    .and_then(|root| root.get("items"))
    .and_then(|items| items.as_array())
  else {
    return out;
  };

  for item in items {
    if item.get("kind").and_then(|kind| kind.as_str()) != Some("assign") {
      continue;
    }
    let Some(key_path) = item.get("key_path").and_then(|path| path.as_array()) else {
      continue;
    };
    if key_path.len() != 1 {
      continue;
    }
    let Some(key) = key_path.first().and_then(|key| key.as_str()) else {
      continue;
    };
    let Some(value) = item.get("value") else {
      continue;
    };
    if value.get("kind").and_then(|kind| kind.as_str()) != Some("string") {
      continue;
    }
    let Some(value) = value.get("value").and_then(|value| value.as_str()) else {
      continue;
    };
    out.insert(key.to_string(), value.to_string());
  }
  out
}

/// Batch: check every folded candidate. One result per input, in
/// input order.
pub fn check_all(folded_candidates: &[MacroFoldedCandidate]) -> Vec<AxisSeparatedCandidate> {
  folded_candidates
    .iter()
    .map(check_axis_separation)
    .collect()
}

/// Render an `AxisSeparatedCandidate` as the canonical JSON payload
/// of a `coding.axis-separated-candidate` artifact. Stage D-3
/// (Gate 3) cockpit surface — operator sees the exact key-set
/// verdict (matched / missing / extra) plus the schema reference
/// so they can debug *why* a folded row failed axis-separation.
///
/// Single family for all 5 outcomes (`axis-verified` /
/// `held-unknown-table` / `held-missing-keys` / `held-extra-keys` /
/// `held-invalid-field-value`). Key vectors and value violations are
/// always present for the operator to read at a glance — empty
/// vectors signal "this category didn't fire."
///
/// Replay-stable id = SHA-256 of intrinsic identity (outcome +
/// target_owner + target_table + sorted matched_keys + sorted
/// missing_keys + sorted extra_keys + sorted invalid field values).
/// `stored_at_ms` is extrinsic.
///
/// Content policy: metadata only (key names, schema rows). No
/// folded source text bodies — those live in the upstream
/// `macro-folded-candidate` artifact. Customer-release safe.
pub fn build_axis_separated_candidate_artifact(
  separated: &AxisSeparatedCandidate,
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
) -> serde_json::Value {
  let mut h = Sha256::new();
  h.update(b"axis-separated-candidate\x1f");
  h.update(separated.outcome.as_str().as_bytes());
  h.update(b"\x1f");
  h.update(separated.source.source.target_owner.as_bytes());
  h.update(b"\x1e");
  h.update(separated.source.source.target_table.as_bytes());
  h.update(b"\x1f");
  let mut sorted_matched = separated.matched_keys.clone();
  sorted_matched.sort();
  for k in &sorted_matched {
    h.update(k.as_bytes());
    h.update(b"\x1e");
  }
  h.update(b"\x1f");
  let mut sorted_missing = separated.missing_keys.clone();
  sorted_missing.sort();
  for k in &sorted_missing {
    h.update(k.as_bytes());
    h.update(b"\x1e");
  }
  h.update(b"\x1f");
  let mut sorted_extra = separated.extra_keys.clone();
  sorted_extra.sort();
  for k in &sorted_extra {
    h.update(k.as_bytes());
    h.update(b"\x1e");
  }
  h.update(b"\x1f");
  let mut sorted_invalid = separated.invalid_field_values.clone();
  sorted_invalid
    .sort_by(|a, b| (&a.field, &a.value, &a.expected).cmp(&(&b.field, &b.value, &b.expected)));
  for violation in &sorted_invalid {
    h.update(violation.field.as_bytes());
    h.update(b"\x1d");
    h.update(violation.value.as_bytes());
    h.update(b"\x1d");
    h.update(violation.expected.as_bytes());
    h.update(b"\x1e");
  }
  let digest = h.finalize();
  let prefix = digest
    .iter()
    .take(16)
    .map(|b| format!("{b:02x}"))
    .collect::<String>();
  let id = format!("axis-separated-candidate.{prefix}");

  // Look up the schema for cockpit display so operator sees what
  // the gate *expected*. Empty for HeldUnknownTable (no schema row
  // matches the target).
  let target_owner = separated.source.source.target_owner.as_str();
  let target_table = separated.source.source.target_table.as_str();
  let (required_keys, optional_keys): (Vec<String>, Vec<String>) =
    match schema_for(target_owner, target_table) {
      Some(s) => (
        s.required_keys.iter().map(|k| k.to_string()).collect(),
        s.optional_keys.iter().map(|k| k.to_string()).collect(),
      ),
      None => (Vec::new(), Vec::new()),
    };

  let mut payload = serde_json::json!({
    "id": id,
    "artifact_family": "coding.axis-separated-candidate",
    "source_surface": "algorithm-synthesis.axis-separation-gate",
    "stored_at_ms": stored_at_ms,
    "outcome": separated.outcome.as_str(),
    "gate_status": separated.gate_status.as_str(),
    "candidate_kind": separated.source.source.candidate_kind.as_str(),
    "target_owner": target_owner,
    "target_table": target_table,
    "matched_keys": separated.matched_keys,
    "missing_keys": separated.missing_keys,
    "extra_keys": separated.extra_keys,
    "invalid_field_values": separated.invalid_field_values,
    "schema_required_keys": required_keys,
    "schema_optional_keys": optional_keys,
    "reason": separated.reason,
    "related_refs": serde_json::json!([
      format!("candidate-kind:{}", separated.source.source.candidate_kind.as_str()),
      format!("target-owner:{}", target_owner),
      format!("target-table:{}", target_table),
      "owner-law:stdlib/lib/gate/algorithm-synthesis/axis-separation-gate.px",
    ]),
    "target_paths": serde_json::json!([target_owner]),
    "command_refs": Vec::<String>::new(),
  });
  if let Some(snap) = repo_snapshot_ref {
    payload["repo_snapshot_ref"] = serde_json::Value::String(snap.to_string());
  }
  payload
}

#[cfg(test)]
mod tests {
  use super::super::candidate_row_proposal::{CandidateKind, CandidateRowProposal, GateStatus};
  use super::super::macro_fold_gate::{fold_proposal, MacroFoldedCandidate};
  use super::*;
  use std::collections::BTreeMap;

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

  fn fold(p: CandidateRowProposal) -> MacroFoldedCandidate {
    fold_proposal(&p)
  }

  // ─── registry consistency ──────────────────────────────────────

  #[test]
  fn every_axis_outcome_is_reachable_via_as_str() {
    for o in AxisSeparationOutcome::ALL {
      assert!(!o.as_str().is_empty());
    }
  }

  #[test]
  fn schema_table_has_no_duplicate_target_pairs() {
    let mut seen = std::collections::HashSet::new();
    for s in TARGET_TABLE_SCHEMAS {
      let key = (s.target_owner, s.target_table);
      assert!(seen.insert(key), "duplicate schema entry: {:?}", key);
    }
  }

  // ─── key extraction ────────────────────────────────────────────

  #[test]
  fn extract_keys_handles_multi_line_attrset() {
    let text = "{\n  alpha = \"a\";\n  beta = \"b\";\n}";
    let keys = extract_keys_from_folded_text(text);
    assert_eq!(keys, vec!["alpha", "beta"]);
  }

  #[test]
  fn extract_keys_handles_empty_attrset() {
    let text = "{\n}";
    let keys = extract_keys_from_folded_text(text);
    assert!(keys.is_empty());
  }

  // ─── unknown table ────────────────────────────────────────────

  #[test]
  fn unknown_target_owner_holds() {
    let p = proposal(
      CandidateKind::RecurringImportSpec,
      "stdlib/lib/gate/does-not-exist.px",
      "noSuchTable",
      &[("a", "1"), ("b", "2")],
    );
    let r = check_axis_separation(&fold(p));
    assert_eq!(r.outcome, AxisSeparationOutcome::HeldUnknownTable);
    assert_eq!(r.gate_status, GateStatus::Held);
  }

  // ─── missing keys (the v0 candidates' real outcome) ────────────

  #[test]
  fn recurring_channel_success_proposed_row_is_missing_held_to_query_keys() {
    // RecurringChannelSuccess emits `{query_kind, observed_primary_channel}`
    // but heldRoutingMap requires `{held, primary}`.
    let p = proposal(
      CandidateKind::RecurringChannelSuccess,
      "stdlib/lib/gate/algorithm-synthesis/held-to-query.px",
      "heldRoutingMap",
      &[
        ("query_kind", "lookup-module-providing-symbol"),
        ("observed_primary_channel", "host-symbol-resolver"),
      ],
    );
    let r = check_axis_separation(&fold(p));
    assert_eq!(r.outcome, AxisSeparationOutcome::HeldMissingKeys);
    assert!(r.missing_keys.contains(&"held".to_string()));
    assert!(r.missing_keys.contains(&"primary".to_string()));
  }

  #[test]
  fn recurring_import_spec_proposed_row_is_missing_fact_cue_keys() {
    // RecurringImportSpec emits `{import_spec, language, distinct_target_paths}`
    // but factPhrasePatterns requires `{cue, markers}`.
    let p = proposal(
      CandidateKind::RecurringImportSpec,
      "stdlib/lib/gate/algorithm-synthesis/fact-cue-registry.px",
      "factPhrasePatterns",
      &[
        ("import_spec", "import os"),
        ("language", "python"),
        ("distinct_target_paths", "2"),
      ],
    );
    let r = check_axis_separation(&fold(p));
    assert_eq!(r.outcome, AxisSeparationOutcome::HeldMissingKeys);
    assert!(r.missing_keys.contains(&"cue".to_string()));
    assert!(r.missing_keys.contains(&"markers".to_string()));
  }

  // ─── verified (when proposal happens to match schema) ─────────

  #[test]
  fn schema_aware_held_routing_row_verifies() {
    let p = proposal(
      CandidateKind::RecurringChannelSuccess,
      "stdlib/lib/gate/algorithm-synthesis/held-to-query.px",
      "heldRoutingMap",
      &[
        ("held", "missing-import-spec"),
        ("primary", "host-symbol-resolver"),
        ("fallback", "external-knowledge-search"),
      ],
    );
    let r = check_axis_separation(&fold(p));
    assert_eq!(r.outcome, AxisSeparationOutcome::AxisVerified);
    assert_eq!(r.gate_status, GateStatus::AxisSeparationAttempted);
    assert!(r.matched_keys.contains(&"held".to_string()));
    assert!(r.matched_keys.contains(&"primary".to_string()));
    assert!(r.matched_keys.contains(&"fallback".to_string()));
  }

  #[test]
  fn schema_aware_row_without_optional_key_still_verifies() {
    // `fallback` is optional in heldRoutingMap; omitting it is OK.
    let p = proposal(
      CandidateKind::RecurringChannelSuccess,
      "stdlib/lib/gate/algorithm-synthesis/held-to-query.px",
      "heldRoutingMap",
      &[
        ("held", "missing-import-spec"),
        ("primary", "host-symbol-resolver"),
      ],
    );
    let r = check_axis_separation(&fold(p));
    assert_eq!(r.outcome, AxisSeparationOutcome::AxisVerified);
  }

  #[test]
  fn axis_separation_reads_keys_from_folded_ast_json() {
    let p = proposal(
      CandidateKind::RecurringImportSpec,
      "stdlib/lib/gate/known-imports-by-language.px",
      "knownImportsByLanguage",
      &[("language", "python"), ("import_spec", "import os")],
    );
    let mut folded = fold(p);
    assert!(
      folded.folded_ast_json.is_some(),
      "Gate 2 must emit AST evidence"
    );
    folded.folded_source_text = "{\n}".to_string();

    let r = check_axis_separation(&folded);
    assert_eq!(r.outcome, AxisSeparationOutcome::AxisVerified);
    assert!(r.matched_keys.contains(&"language".to_string()));
    assert!(r.matched_keys.contains(&"import_spec".to_string()));
  }

  // ─── extra keys ────────────────────────────────────────────────

  #[test]
  fn row_with_unknown_key_holds_extra_keys() {
    let p = proposal(
      CandidateKind::RecurringChannelSuccess,
      "stdlib/lib/gate/algorithm-synthesis/held-to-query.px",
      "heldRoutingMap",
      &[
        ("held", "missing-import-spec"),
        ("primary", "host-symbol-resolver"),
        ("unknown_extra_field", "value"),
      ],
    );
    let r = check_axis_separation(&fold(p));
    assert_eq!(r.outcome, AxisSeparationOutcome::HeldExtraKeys);
    assert!(r.extra_keys.contains(&"unknown_extra_field".to_string()));
  }

  #[test]
  fn learned_intent_row_with_unknown_intent_holds_invalid_value() {
    let p = proposal(
      CandidateKind::LearnedIntentSignal,
      "stdlib/lib/gate/algorithm-synthesis/learned-intent-overlay.px",
      "overlayIntentSignals",
      &[
        ("cue", "fact:bad-intent-cue"),
        ("intent", "remix"),
        ("weight", "0.92"),
      ],
    );
    let r = check_axis_separation(&fold(p));
    assert_eq!(r.outcome, AxisSeparationOutcome::HeldInvalidFieldValue);
    assert_eq!(r.gate_status, GateStatus::Held);
    assert_eq!(r.invalid_field_values.len(), 1);
    assert_eq!(r.invalid_field_values[0].field, "intent");
    assert_eq!(r.invalid_field_values[0].value, "remix");
    assert_eq!(
      r.invalid_field_values[0].expected,
      "intent-recognition.validIntents"
    );
  }

  #[test]
  fn learned_intent_row_with_valid_intent_verifies() {
    let p = proposal(
      CandidateKind::LearnedIntentSignal,
      "stdlib/lib/gate/algorithm-synthesis/learned-intent-overlay.px",
      "overlayIntentSignals",
      &[
        ("cue", "fact:valid-intent-cue"),
        ("intent", "refactor"),
        ("weight", "0.92"),
      ],
    );
    let r = check_axis_separation(&fold(p));
    assert_eq!(r.outcome, AxisSeparationOutcome::AxisVerified);
    assert!(r.invalid_field_values.is_empty());
  }

  #[test]
  fn learned_intent_row_with_non_numeric_weight_holds_invalid_value() {
    let p = proposal(
      CandidateKind::LearnedIntentSignal,
      "stdlib/lib/gate/algorithm-synthesis/learned-intent-overlay.px",
      "overlayIntentSignals",
      &[
        ("cue", "fact:bad-weight-cue"),
        ("intent", "refactor"),
        ("weight", "heavy"),
      ],
    );
    let r = check_axis_separation(&fold(p));
    assert_eq!(r.outcome, AxisSeparationOutcome::HeldInvalidFieldValue);
    assert_eq!(r.invalid_field_values.len(), 1);
    assert_eq!(r.invalid_field_values[0].field, "weight");
    assert_eq!(r.invalid_field_values[0].value, "heavy");
    assert_eq!(
      r.invalid_field_values[0].expected,
      "numeric weight in [0,1]"
    );
  }

  #[test]
  fn learned_operation_row_with_unknown_transform_holds_invalid_value() {
    let p = proposal(
      CandidateKind::LearnedOperationMap,
      "stdlib/lib/gate/algorithm-synthesis/learned-operation-overlay.px",
      "overlayOperationRows",
      &[
        ("intent", "refactor"),
        ("cues", "fact:op-cycle-test-cue"),
        ("transform", "fn-op-cycle-test-transform"),
        ("weight", "0.92"),
      ],
    );
    let r = check_axis_separation(&fold(p));
    assert_eq!(r.outcome, AxisSeparationOutcome::HeldInvalidFieldValue);
    assert_eq!(r.gate_status, GateStatus::Held);
    assert_eq!(r.invalid_field_values.len(), 1);
    assert_eq!(r.invalid_field_values[0].field, "transform");
    assert_eq!(
      r.invalid_field_values[0].value,
      "fn-op-cycle-test-transform"
    );
  }

  #[test]
  fn learned_operation_row_with_valid_transform_verifies() {
    let p = proposal(
      CandidateKind::LearnedOperationMap,
      "stdlib/lib/gate/algorithm-synthesis/learned-operation-overlay.px",
      "overlayOperationRows",
      &[
        ("intent", "refactor"),
        ("cues", "fact:op-cycle-test-cue"),
        ("transform", "change-signature"),
        ("weight", "0.92"),
      ],
    );
    let r = check_axis_separation(&fold(p));
    assert_eq!(r.outcome, AxisSeparationOutcome::AxisVerified);
    assert!(r.invalid_field_values.is_empty());
  }

  #[test]
  fn learned_operation_row_with_out_of_range_weight_holds_invalid_value() {
    let p = proposal(
      CandidateKind::LearnedOperationMap,
      "stdlib/lib/gate/algorithm-synthesis/learned-operation-overlay.px",
      "overlayOperationRows",
      &[
        ("intent", "refactor"),
        ("cues", "fact:op-cycle-test-cue"),
        ("transform", "change-signature"),
        ("weight", "1.7"),
      ],
    );
    let r = check_axis_separation(&fold(p));
    assert_eq!(r.outcome, AxisSeparationOutcome::HeldInvalidFieldValue);
    assert_eq!(r.invalid_field_values.len(), 1);
    assert_eq!(r.invalid_field_values[0].field, "weight");
    assert_eq!(r.invalid_field_values[0].value, "1.7");
    assert_eq!(
      r.invalid_field_values[0].expected,
      "numeric weight in [0,1]"
    );
  }

  // ─── batch ─────────────────────────────────────────────────────

  #[test]
  fn check_all_returns_one_per_input() {
    let candidates = vec![
      fold(proposal(
        CandidateKind::RecurringImportSpec,
        "stdlib/lib/gate/algorithm-synthesis/fact-cue-registry.px",
        "factPhrasePatterns",
        &[("import_spec", "import os")],
      )),
      fold(proposal(
        CandidateKind::RecurringChannelSuccess,
        "stdlib/lib/gate/algorithm-synthesis/held-to-query.px",
        "heldRoutingMap",
        &[("held", "x"), ("primary", "y")],
      )),
    ];
    let results = check_all(&candidates);
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].outcome, AxisSeparationOutcome::HeldMissingKeys);
    assert_eq!(results[1].outcome, AxisSeparationOutcome::AxisVerified);
  }

  // ─── axis-separated-candidate artifact (Stage D-3 panel) ─────

  fn verified_known_imports() -> AxisSeparatedCandidate {
    let p = proposal(
      CandidateKind::RecurringImportSpec,
      "stdlib/lib/gate/known-imports-by-language.px",
      "knownImportsByLanguage",
      &[("language", "python"), ("import_spec", "import os")],
    );
    let f = fold_proposal(&p);
    check_axis_separation(&f)
  }

  fn missing_key_candidate() -> AxisSeparatedCandidate {
    let p = proposal(
      CandidateKind::RecurringImportSpec,
      "stdlib/lib/gate/known-imports-by-language.px",
      "knownImportsByLanguage",
      &[("language", "python")], // import_spec missing
    );
    let f = fold_proposal(&p);
    check_axis_separation(&f)
  }

  fn extra_key_candidate() -> AxisSeparatedCandidate {
    let p = proposal(
      CandidateKind::RecurringImportSpec,
      "stdlib/lib/gate/known-imports-by-language.px",
      "knownImportsByLanguage",
      &[
        ("language", "python"),
        ("import_spec", "import os"),
        ("extraneous_field", "noise"),
      ],
    );
    let f = fold_proposal(&p);
    check_axis_separation(&f)
  }

  #[test]
  fn artifact_verified_carries_matched_keys_and_schema_reference() {
    let s = verified_known_imports();
    assert_eq!(s.outcome, AxisSeparationOutcome::AxisVerified);
    let art = build_axis_separated_candidate_artifact(&s, 1700000000000, None);
    assert_eq!(art["artifact_family"], "coding.axis-separated-candidate");
    assert_eq!(art["outcome"], "axis-verified");
    let matched: Vec<String> = serde_json::from_value(art["matched_keys"].clone()).unwrap();
    assert!(matched.iter().any(|k| k == "language"));
    assert!(matched.iter().any(|k| k == "import_spec"));
    let required: Vec<String> =
      serde_json::from_value(art["schema_required_keys"].clone()).unwrap();
    assert!(required.iter().any(|k| k == "language"));
    assert!(required.iter().any(|k| k == "import_spec"));
  }

  #[test]
  fn artifact_held_missing_keys_surfaces_exact_missing_key() {
    let s = missing_key_candidate();
    assert_eq!(s.outcome, AxisSeparationOutcome::HeldMissingKeys);
    let art = build_axis_separated_candidate_artifact(&s, 0, None);
    assert_eq!(art["outcome"], "held-missing-keys");
    let missing: Vec<String> = serde_json::from_value(art["missing_keys"].clone()).unwrap();
    assert!(missing.iter().any(|k| k == "import_spec"));
  }

  #[test]
  fn artifact_held_extra_keys_surfaces_exact_extra_key() {
    let s = extra_key_candidate();
    assert_eq!(s.outcome, AxisSeparationOutcome::HeldExtraKeys);
    let art = build_axis_separated_candidate_artifact(&s, 0, None);
    assert_eq!(art["outcome"], "held-extra-keys");
    let extra: Vec<String> = serde_json::from_value(art["extra_keys"].clone()).unwrap();
    assert!(extra.iter().any(|k| k == "extraneous_field"));
  }

  #[test]
  fn artifact_held_unknown_table_has_empty_schema_lists() {
    let p = proposal(
      CandidateKind::RecurringImportSpec,
      "stdlib/lib/gate/no-such-owner.px",
      "noSuchTable",
      &[("k", "v")],
    );
    let f = fold_proposal(&p);
    let s = check_axis_separation(&f);
    assert_eq!(s.outcome, AxisSeparationOutcome::HeldUnknownTable);
    let art = build_axis_separated_candidate_artifact(&s, 0, None);
    assert_eq!(art["outcome"], "held-unknown-table");
    let required: Vec<String> =
      serde_json::from_value(art["schema_required_keys"].clone()).unwrap();
    let optional: Vec<String> =
      serde_json::from_value(art["schema_optional_keys"].clone()).unwrap();
    assert!(required.is_empty(), "no schema → empty required list");
    assert!(optional.is_empty(), "no schema → empty optional list");
  }

  #[test]
  fn artifact_id_is_replay_stable_across_stored_at() {
    let s = verified_known_imports();
    let a1 = build_axis_separated_candidate_artifact(&s, 1, None);
    let a2 = build_axis_separated_candidate_artifact(&s, 999999, None);
    assert_eq!(a1["id"], a2["id"]);
  }

  #[test]
  fn artifact_id_differs_between_verified_and_missing() {
    let v = verified_known_imports();
    let m = missing_key_candidate();
    let a_v = build_axis_separated_candidate_artifact(&v, 0, None);
    let a_m = build_axis_separated_candidate_artifact(&m, 0, None);
    assert_ne!(a_v["id"], a_m["id"]);
  }

  #[test]
  fn artifact_math_lane_verified_renders_with_math_schema() {
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
    assert_eq!(s.outcome, AxisSeparationOutcome::AxisVerified);
    let art = build_axis_separated_candidate_artifact(&s, 0, None);
    assert_eq!(art["candidate_kind"], "math-expression-lower");
    assert_eq!(art["target_table"], "knownAlgebraicIdentities");
    let required: Vec<String> =
      serde_json::from_value(art["schema_required_keys"].clone()).unwrap();
    assert!(required.iter().any(|k| k == "canonical_form"));
    assert!(required.iter().any(|k| k == "equivalent_form"));
    assert!(required.iter().any(|k| k == "language"));
  }
}
