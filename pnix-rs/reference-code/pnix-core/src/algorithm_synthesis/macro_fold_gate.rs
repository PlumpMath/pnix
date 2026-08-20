//! Macro-fold gate — Stage D-v1 of the evolution lane (firewall
//! gate #2).
//!
//! OWNER-LAW (2026-05-12): mirror of
//! `stdlib/lib/gate/algorithm-synthesis/macro-fold-gate.px`.
//! Consumes a `CandidateRowProposal` (gate 1 output) and produces
//! a `MacroFoldedCandidate` whose `folded_source_text` is a valid
//! Nix attrset literal ready for the next gate to validate against
//! a real `.px` table schema.
//!
//! Strictly syntactic — does NOT verify that the proposed row's
//! keys match the target table's schema. That is gate 3
//! (axis-separation).

use serde::{Deserialize, Serialize};
use pnix_hash::{Digest, Sha256};

use crate::lang::pnix::{parse_expr_to_ast_json, PNIX_AST_JSON_FORMAT};

use super::candidate_row_proposal::{CandidateRowProposal, GateStatus};

/// Possible fold outcomes. Stays byte-identical to `.px`
/// `validFoldOutcomes`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MacroFoldOutcome {
  Folded,
  HeldNotFoldable,
}

impl MacroFoldOutcome {
  pub const ALL: &'static [Self] = &[Self::Folded, Self::HeldNotFoldable];

  pub fn as_str(self) -> &'static str {
    match self {
      Self::Folded => "folded",
      Self::HeldNotFoldable => "held-not-foldable",
    }
  }
}

/// Attrset syntax rule row. Mirror of `.px` `attrsetSyntaxRules`.
#[derive(Debug, Clone, Copy)]
pub struct AttrsetSyntaxRule {
  pub format_id: &'static str,
  pub indent_spaces: usize,
  pub key_value_separator: &'static str,
  pub row_terminator: &'static str,
  pub open_brace: &'static str,
  pub close_brace: &'static str,
}

pub const ATTRSET_SYNTAX_RULES: &[AttrsetSyntaxRule] = &[AttrsetSyntaxRule {
  format_id: "multi-line-attrset",
  indent_spaces: 2,
  key_value_separator: " = ",
  row_terminator: ";",
  open_brace: "{",
  close_brace: "}",
}];

pub const DEFAULT_FORMAT: &str = "multi-line-attrset";

fn rule_for(format_id: &str) -> Option<&'static AttrsetSyntaxRule> {
  ATTRSET_SYNTAX_RULES
    .iter()
    .find(|r| r.format_id == format_id)
}

/// Escape a string for use as a Nix double-quoted string literal.
/// Escapes `\` and `"` only — newlines / tabs in values are
/// preserved verbatim (Nix double-quoted strings handle them).
fn escape_nix_string(s: &str) -> String {
  let mut out = String::with_capacity(s.len() + 2);
  for c in s.chars() {
    match c {
      '\\' => out.push_str("\\\\"),
      '"' => out.push_str("\\\""),
      c => out.push(c),
    }
  }
  out
}

/// The output of the gate. Carries the original proposal (audit)
/// and the syntactic fold result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MacroFoldedCandidate {
  /// Original proposal — preserved for downstream gates.
  pub source: CandidateRowProposal,
  /// `.px` source-text fragment. For `Folded`: a valid Nix attrset
  /// literal. For `HeldNotFoldable`: an empty string.
  pub folded_source_text: String,
  /// Stable AST projection of `folded_source_text`. Present only
  /// for `Folded`. This keeps Gate 2 from being text-only: the
  /// emitted row must parse back into first-class `.px` AST data
  /// before downstream gates can treat it as folded.
  pub folded_ast_json: Option<serde_json::Value>,
  /// Version tag for `folded_ast_json`.
  pub folded_ast_format: Option<String>,
  /// Which syntax rule produced `folded_source_text`. v0 always
  /// `"multi-line-attrset"`.
  pub format_id: String,
  /// Gate outcome.
  pub outcome: MacroFoldOutcome,
  /// Updated gate status — `MacroFoldAttempted` on `Folded`,
  /// `Held` on `HeldNotFoldable`.
  pub gate_status: GateStatus,
  /// Human-readable note for cockpit review.
  pub reason: String,
}

/// Fold a `CandidateRowProposal` into a syntactic `.px` source-text
/// fragment using `DEFAULT_FORMAT`.
pub fn fold_proposal(proposal: &CandidateRowProposal) -> MacroFoldedCandidate {
  fold_proposal_with_format(proposal, DEFAULT_FORMAT)
}

/// Fold using a named syntax rule. Returns a `HeldNotFoldable`
/// candidate if the rule isn't registered (caller bug).
///
/// **OWNER-LAW (v0.6.6, 2026-05-15)**: this pure-Rust body is a
/// **byte-equivalent mirror** of the `.px` owner
/// `stdlib/lib/gate/algorithm-synthesis/macro-fold-gate.px::foldAttrsetTextWithDefaultFormat`
/// (v0.6.1 migration). The `.px` is the **canonical owner**; this
/// Rust body exists as a cycle-free in-process fast-path because
/// `pnix-core` cannot depend on `pnix-eval` (which evaluates `.px`).
///
/// The byte-equivalent `.px`-delegating entry is
/// `doghouse_core::algorithm_synthesis_bridge::macro_fold_gate_via_px_body`.
/// Drift between the two paths is prevented by the byte-equivalence
/// ratchet test
/// `crates/doghouse-core/tests/macro_fold_gate_via_px_body_byte_equivalence.rs`
/// and the full-pipeline test
/// `crates/doghouse-core/tests/firewall_pipeline_dual_path_equivalence.rs`.
///
/// Any change to this body MUST come with a corresponding change
/// to the `.px` owner, or the byte-equivalence test will fail.
pub fn fold_proposal_with_format(
  proposal: &CandidateRowProposal,
  format_id: &str,
) -> MacroFoldedCandidate {
  let Some(rule) = rule_for(format_id) else {
    return MacroFoldedCandidate {
      source: proposal.clone(),
      folded_source_text: String::new(),
      folded_ast_json: None,
      folded_ast_format: None,
      format_id: format_id.to_string(),
      outcome: MacroFoldOutcome::HeldNotFoldable,
      gate_status: GateStatus::Held,
      reason: format!("unknown attrset format `{format_id}`"),
    };
  };

  if proposal.proposed_row.is_empty() {
    return MacroFoldedCandidate {
      source: proposal.clone(),
      folded_source_text: String::new(),
      folded_ast_json: None,
      folded_ast_format: None,
      format_id: format_id.to_string(),
      outcome: MacroFoldOutcome::HeldNotFoldable,
      gate_status: GateStatus::Held,
      reason: "proposed_row is empty — nothing to fold".to_string(),
    };
  }

  // BTreeMap iter order is alphabetical — deterministic across
  // runs and across platforms.
  let indent = " ".repeat(rule.indent_spaces);
  let mut text = String::new();
  text.push_str(rule.open_brace);
  text.push('\n');
  for (k, v) in &proposal.proposed_row {
    text.push_str(&indent);
    text.push_str(k);
    text.push_str(rule.key_value_separator);
    text.push('"');
    text.push_str(&escape_nix_string(v));
    text.push('"');
    text.push_str(rule.row_terminator);
    text.push('\n');
  }
  text.push_str(rule.close_brace);

  let ast = match parse_expr_to_ast_json(&text) {
    Ok(ast) => ast,
    Err(err) => {
      return MacroFoldedCandidate {
        source: proposal.clone(),
        folded_source_text: String::new(),
        folded_ast_json: None,
        folded_ast_format: None,
        format_id: format_id.to_string(),
        outcome: MacroFoldOutcome::HeldNotFoldable,
        gate_status: GateStatus::Held,
        reason: format!("folded text did not parse as `.px` attrset: {err}"),
      };
    }
  };

  MacroFoldedCandidate {
    source: proposal.clone(),
    folded_source_text: text,
    folded_ast_json: Some(ast),
    folded_ast_format: Some(PNIX_AST_JSON_FORMAT.to_string()),
    format_id: format_id.to_string(),
    outcome: MacroFoldOutcome::Folded,
    gate_status: GateStatus::MacroFoldAttempted,
    reason: format!(
      "folded {} key(s) into `{format_id}` syntax",
      proposal.proposed_row.len()
    ),
  }
}

/// Convenience: fold every proposal in a batch. Returns one result
/// per input in input order.
pub fn fold_all(proposals: &[CandidateRowProposal]) -> Vec<MacroFoldedCandidate> {
  proposals.iter().map(fold_proposal).collect()
}

// v0.6.2 (2026-05-14): the `.px`-body-delegating entry
// `fold_proposal_with_format_via_px_body` cannot live in this crate —
// pnix-eval depends on pnix-core, so the delegation must live one
// layer up. The actual delegating function is at
// `crates/doghouse-core/src/algorithm_synthesis_bridge.rs::macro_fold_gate_via_px_body`,
// and its byte-equivalence proof (against this file's
// `fold_proposal_with_format`) lives at
// `crates/doghouse-core/tests/macro_fold_gate_via_px_body_byte_equivalence.rs`.
// See `project-wiki/maps/non-mirror-px-pnixc-meta-migration-plan.md`
// v0.6.2 row for the honest crate-layering note.

/// Render a `MacroFoldedCandidate` as the canonical JSON payload of
/// a `coding.macro-folded-candidate` artifact. Stage D-2
/// (Gate 2 output) cockpit surface — the operator sees the actual
/// `.px` source-text fragment pnix would insert, in the syntax it
/// would emit, *before* axis-separation runs.
///
/// Single family for both outcomes (`folded` / `held-not-foldable`),
/// status field discriminates. On Folded, carries the full
/// `folded_source_text` + format_id + the row that was folded. On
/// HeldNotFoldable, carries the source row + reason (empty text).
///
/// Replay-stable id = SHA-256 of intrinsic identity (outcome +
/// format_id + folded_source_text + candidate_kind + target_owner +
/// target_table + sorted source row). `stored_at_ms` is extrinsic.
///
/// Content policy: `folded_source_text` is the row's key-value
/// data rendered as Nix attrset syntax — caller-injected observer
/// data only, no source bodies. Customer-release safe.
pub fn build_macro_folded_candidate_artifact(
  folded: &MacroFoldedCandidate,
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
) -> serde_json::Value {
  let mut h = Sha256::new();
  h.update(b"macro-folded-candidate\x1f");
  h.update(folded.outcome.as_str().as_bytes());
  h.update(b"\x1f");
  h.update(folded.format_id.as_bytes());
  h.update(b"\x1f");
  h.update(folded.folded_source_text.as_bytes());
  h.update(b"\x1f");
  h.update(folded.source.candidate_kind.as_str().as_bytes());
  h.update(b"\x1e");
  h.update(folded.source.target_owner.as_bytes());
  h.update(b"\x1e");
  h.update(folded.source.target_table.as_bytes());
  h.update(b"\x1f");
  let mut row_keys: Vec<&String> = folded.source.proposed_row.keys().collect();
  row_keys.sort();
  for k in row_keys {
    h.update(k.as_bytes());
    h.update(b"\x1d");
    h.update(folded.source.proposed_row[k].as_bytes());
    h.update(b"\x1e");
  }
  let digest = h.finalize();
  let prefix = digest
    .iter()
    .take(16)
    .map(|b| format!("{b:02x}"))
    .collect::<String>();
  let id = format!("macro-folded-candidate.{prefix}");

  let mut payload = serde_json::json!({
    "id": id,
    "artifact_family": "coding.macro-folded-candidate",
    "source_surface": "algorithm-synthesis.macro-fold-gate",
    "stored_at_ms": stored_at_ms,
    "outcome": folded.outcome.as_str(),
    "gate_status": folded.gate_status.as_str(),
    "format_id": folded.format_id,
    "candidate_kind": folded.source.candidate_kind.as_str(),
    "target_owner": folded.source.target_owner,
    "target_table": folded.source.target_table,
    "source_row": folded.source.proposed_row,
    "folded_source_text": folded.folded_source_text,
    "folded_byte_len": folded.folded_source_text.len(),
    "folded_ast_format": folded.folded_ast_format,
    "folded_ast_json": folded.folded_ast_json,
    "reason": folded.reason,
    "related_refs": serde_json::json!([
      format!("candidate-kind:{}", folded.source.candidate_kind.as_str()),
      format!("target-owner:{}", folded.source.target_owner),
      format!("target-table:{}", folded.source.target_table),
      format!("format-id:{}", folded.format_id),
      "owner-law:stdlib/lib/gate/algorithm-synthesis/macro-fold-gate.px",
    ]),
    "target_paths": serde_json::json!([folded.source.target_owner]),
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
  use super::*;
  use std::collections::BTreeMap;

  fn proposal(kind: CandidateKind, row: &[(&str, &str)]) -> CandidateRowProposal {
    let mut proposed = BTreeMap::new();
    for (k, v) in row {
      proposed.insert(k.to_string(), v.to_string());
    }
    CandidateRowProposal {
      candidate_kind: kind,
      target_owner: "stdlib/lib/gate/test.px".to_string(),
      target_table: "testTable".to_string(),
      proposed_row: proposed,
      supporting_evidence: vec!["evidence-1".to_string()],
      evidence_count: 2,
      gate_status: GateStatus::IntentReceiptOnly,
      reason: "test".to_string(),
    }
  }

  #[test]
  fn folds_simple_two_key_proposal_into_multi_line_attrset() {
    let p = proposal(
      CandidateKind::RecurringImportSpec,
      &[("import_spec", "import os"), ("language", "python")],
    );
    let r = fold_proposal(&p);
    assert_eq!(r.outcome, MacroFoldOutcome::Folded);
    assert_eq!(r.gate_status, GateStatus::MacroFoldAttempted);
    // BTreeMap → alphabetical key order.
    let expected = "{\n  import_spec = \"import os\";\n  language = \"python\";\n}";
    assert_eq!(r.folded_source_text, expected);
  }

  #[test]
  fn deterministic_key_order_across_runs() {
    // Insertion order differs from sorted order — output must use
    // sorted order so two runs of the same proposal produce
    // identical text.
    let mut row1 = BTreeMap::new();
    row1.insert("z_key".to_string(), "z".to_string());
    row1.insert("a_key".to_string(), "a".to_string());
    let p1 = CandidateRowProposal {
      candidate_kind: CandidateKind::RecurringImportSpec,
      target_owner: "t".to_string(),
      target_table: "t".to_string(),
      proposed_row: row1,
      supporting_evidence: vec![],
      evidence_count: 2,
      gate_status: GateStatus::IntentReceiptOnly,
      reason: "".to_string(),
    };
    let mut row2 = BTreeMap::new();
    row2.insert("a_key".to_string(), "a".to_string());
    row2.insert("z_key".to_string(), "z".to_string());
    let p2 = CandidateRowProposal {
      proposed_row: row2,
      ..p1.clone()
    };
    let r1 = fold_proposal(&p1);
    let r2 = fold_proposal(&p2);
    assert_eq!(r1.folded_source_text, r2.folded_source_text);
    assert!(r1.folded_source_text.contains("a_key"));
    assert!(r1.folded_source_text.find("a_key") < r1.folded_source_text.find("z_key"));
  }

  // ─── escaping ─────────────────────────────────────────────────

  #[test]
  fn escapes_double_quote_in_value() {
    let p = proposal(
      CandidateKind::RecurringImportSpec,
      &[("import_spec", "use std::path::Path; // \"safe\"")],
    );
    let r = fold_proposal(&p);
    assert_eq!(r.outcome, MacroFoldOutcome::Folded);
    assert!(r.folded_source_text.contains(r#"\"safe\""#));
    // The output must still parse as a Nix double-quoted string —
    // a naive caller could re-parse `r.folded_source_text` and get
    // back the original value modulo escapes.
  }

  #[test]
  fn escapes_backslash_in_value() {
    let p = proposal(
      CandidateKind::RecurringImportSpec,
      &[("import_spec", "C:\\Path\\To\\Foo")],
    );
    let r = fold_proposal(&p);
    assert_eq!(r.outcome, MacroFoldOutcome::Folded);
    assert!(r.folded_source_text.contains("C:\\\\Path\\\\To\\\\Foo"));
  }

  #[test]
  fn preserves_korean_characters_unescaped() {
    let p = proposal(
      CandidateKind::RecurringImportSpec,
      &[("description", "이 함수 이름 바꿔줘")],
    );
    let r = fold_proposal(&p);
    assert_eq!(r.outcome, MacroFoldOutcome::Folded);
    // Korean chars pass through Nix double-quoted strings verbatim;
    // we should NOT have transformed them in any way.
    assert!(r.folded_source_text.contains("이 함수 이름 바꿔줘"));
  }

  // ─── held cases ───────────────────────────────────────────────

  #[test]
  fn empty_proposed_row_holds_not_foldable() {
    let mut p = proposal(CandidateKind::RecurringImportSpec, &[]);
    p.proposed_row.clear();
    let r = fold_proposal(&p);
    assert_eq!(r.outcome, MacroFoldOutcome::HeldNotFoldable);
    assert_eq!(r.gate_status, GateStatus::Held);
    assert!(r.folded_source_text.is_empty());
    assert!(r.reason.contains("empty"));
  }

  #[test]
  fn unknown_format_id_holds_not_foldable() {
    let p = proposal(
      CandidateKind::RecurringImportSpec,
      &[("import_spec", "import os")],
    );
    let r = fold_proposal_with_format(&p, "bogus-format");
    assert_eq!(r.outcome, MacroFoldOutcome::HeldNotFoldable);
    assert_eq!(r.gate_status, GateStatus::Held);
    assert!(r.folded_source_text.is_empty());
  }

  // ─── original proposal preserved ──────────────────────────────

  #[test]
  fn original_proposal_carried_verbatim_for_audit() {
    let p = proposal(
      CandidateKind::RecurringChannelSuccess,
      &[
        ("query_kind", "lookup-module-providing-symbol"),
        ("observed_primary_channel", "host-symbol-resolver"),
      ],
    );
    let r = fold_proposal(&p);
    assert_eq!(r.source, p, "source must be preserved verbatim");
    assert_eq!(r.source.evidence_count, 2);
  }

  // ─── batch fold ───────────────────────────────────────────────

  #[test]
  fn fold_all_returns_one_per_input() {
    let proposals = vec![
      proposal(
        CandidateKind::RecurringImportSpec,
        &[("import_spec", "import os")],
      ),
      proposal(
        CandidateKind::RecurringChannelSuccess,
        &[("query_kind", "lookup-module-providing-symbol")],
      ),
    ];
    let results = fold_all(&proposals);
    assert_eq!(results.len(), 2);
    for r in &results {
      assert_eq!(r.outcome, MacroFoldOutcome::Folded);
    }
  }

  // ─── output is valid Nix attrset shape ────────────────────────

  #[test]
  fn folded_text_starts_with_open_brace_and_ends_with_close() {
    let p = proposal(
      CandidateKind::RecurringImportSpec,
      &[("import_spec", "import os")],
    );
    let r = fold_proposal(&p);
    assert!(r.folded_source_text.starts_with('{'));
    assert!(r.folded_source_text.ends_with('}'));
  }

  #[test]
  fn folded_candidate_carries_stable_ast_projection() {
    let p = proposal(
      CandidateKind::RecurringImportSpec,
      &[("import_spec", "import os"), ("language", "python")],
    );
    let r = fold_proposal(&p);
    assert_eq!(r.outcome, MacroFoldOutcome::Folded);
    assert_eq!(r.folded_ast_format.as_deref(), Some(PNIX_AST_JSON_FORMAT));
    let ast = r.folded_ast_json.as_ref().expect("folded AST");
    assert_eq!(ast["format"], PNIX_AST_JSON_FORMAT);
    assert_eq!(ast["root"]["kind"], "attr_set");
    let items = ast["root"]["items"].as_array().expect("attr items");
    assert_eq!(items.len(), 2);
  }

  // ─── macro-folded-candidate artifact (Stage D-2 panel) ───────

  fn folded_polynomial_identity() -> MacroFoldedCandidate {
    fold_proposal(&proposal(
      CandidateKind::MathExpressionLower,
      &[
        ("canonical_form", "x^2 + 2*x*y + y^2"),
        ("equivalent_form", "(x+y)^2"),
        ("language", "polynomial"),
      ],
    ))
  }

  #[test]
  fn artifact_carries_outcome_format_and_folded_text() {
    let f = folded_polynomial_identity();
    let art = build_macro_folded_candidate_artifact(&f, 1700000000000, None);
    assert_eq!(art["artifact_family"], "coding.macro-folded-candidate");
    assert_eq!(art["outcome"], "folded");
    assert_eq!(art["format_id"], "multi-line-attrset");
    assert_eq!(art["candidate_kind"], "math-expression-lower");
    let text = art["folded_source_text"].as_str().unwrap();
    assert!(text.contains("canonical_form"));
    assert!(text.contains("equivalent_form"));
    assert!(text.contains("language"));
    assert_eq!(art["folded_byte_len"], text.len());
    assert_eq!(art["folded_ast_format"], PNIX_AST_JSON_FORMAT);
    assert_eq!(art["folded_ast_json"]["root"]["kind"], "attr_set");
  }

  #[test]
  fn artifact_carries_source_row_for_walk_back() {
    let f = folded_polynomial_identity();
    let art = build_macro_folded_candidate_artifact(&f, 0, None);
    let src = art["source_row"].as_object().unwrap();
    assert_eq!(src.get("canonical_form").unwrap(), "x^2 + 2*x*y + y^2");
    assert_eq!(src.get("equivalent_form").unwrap(), "(x+y)^2");
  }

  #[test]
  fn artifact_held_carries_empty_folded_text_and_reason() {
    // proposal with empty row → HeldNotFoldable.
    let mut p = proposal(CandidateKind::RecurringImportSpec, &[]);
    p.proposed_row.clear();
    let f = fold_proposal(&p);
    assert_eq!(f.outcome, MacroFoldOutcome::HeldNotFoldable);
    let art = build_macro_folded_candidate_artifact(&f, 0, None);
    assert_eq!(art["outcome"], "held-not-foldable");
    assert_eq!(art["folded_source_text"], "");
    assert_eq!(art["folded_byte_len"], 0);
    let reason = art["reason"].as_str().unwrap();
    assert!(!reason.is_empty(), "Held outcomes must surface a reason");
  }

  #[test]
  fn artifact_id_is_replay_stable_across_stored_at() {
    let f = folded_polynomial_identity();
    let a1 = build_macro_folded_candidate_artifact(&f, 1, None);
    let a2 = build_macro_folded_candidate_artifact(&f, 999999, None);
    assert_eq!(a1["id"], a2["id"]);
  }

  #[test]
  fn artifact_id_differs_for_different_folded_text() {
    let f1 = folded_polynomial_identity();
    // Same shape but different equivalent_form → different folded text.
    let f2 = fold_proposal(&proposal(
      CandidateKind::MathExpressionLower,
      &[
        ("canonical_form", "x^2 + 2*x*y + y^2"),
        ("equivalent_form", "(y+x)^2"),
        ("language", "polynomial"),
      ],
    ));
    let a1 = build_macro_folded_candidate_artifact(&f1, 0, None);
    let a2 = build_macro_folded_candidate_artifact(&f2, 0, None);
    assert_ne!(a1["id"], a2["id"]);
  }

  #[test]
  fn artifact_related_refs_carry_format_and_owner_law() {
    let f = folded_polynomial_identity();
    let art = build_macro_folded_candidate_artifact(&f, 0, None);
    let refs: Vec<String> = serde_json::from_value(art["related_refs"].clone()).unwrap();
    assert!(refs.iter().any(|r| r == "format-id:multi-line-attrset"));
    assert!(refs.iter().any(|r| r.contains("macro-fold-gate.px")));
  }
}
