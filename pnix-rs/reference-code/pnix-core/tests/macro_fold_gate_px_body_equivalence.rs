//! v0.6 first slice (2026-05-14): byte-equivalence between the
//! Rust `fold_proposal_with_format` body and the new
//! `macro-fold-gate.px::foldAttrsetText` `.px`-side body.
//!
//! Per
//! `project-wiki/maps/non-mirror-px-pnixc-meta-migration-plan.md::Stage 1 P0`,
//! Stage 1 P0 closed `.px`-side data + entry functions for the
//! 5-gate firewall. v0.6 begins migrating the actual *gate body
//! logic* from Rust into `.px`. macro-fold-gate is the first
//! carrier because it has the smallest pure-string body.
//!
//! This test pins the byte-equivalence claim. If the Rust body and
//! the `.px` body diverge on any input, this test fails — the
//! migration cannot drift silently.

use std::path::PathBuf;
use std::process::Command;

use pnix_core::algorithm_synthesis::candidate_row_proposal::{
  CandidateKind, CandidateRowProposal, GateStatus,
};
use pnix_core::algorithm_synthesis::macro_fold_gate::{
  fold_proposal_with_format, MacroFoldOutcome,
};

fn workspace_root() -> PathBuf {
  let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
  p.push("../..");
  p.canonicalize().expect("canonicalise workspace root")
}

fn pnixc_meta_binary() -> PathBuf {
  workspace_root().join("target/debug/pnixc-meta")
}

fn macro_fold_gate_px() -> PathBuf {
  workspace_root().join("stdlib/lib/gate/algorithm-synthesis/macro-fold-gate.px")
}

fn run_pnixc_meta(name: &str, source: &str) -> String {
  let tmp_dir = workspace_root().join("target/tmp-pnixc-meta-px-body-tests");
  std::fs::create_dir_all(&tmp_dir).expect("mk tmp dir");
  let path = tmp_dir.join(format!("{name}.px"));
  std::fs::write(&path, source).expect("write tmp .px");
  let bin = pnixc_meta_binary();
  assert!(
    bin.exists(),
    "pnixc-meta binary missing — `cargo build -p pnixc-meta` first: {}",
    bin.display()
  );
  let out = Command::new(&bin)
    .arg(&path)
    .env("PNIXC_META_STACK_BYTES", (256 * 1024 * 1024).to_string())
    .output()
    .expect("invoke pnixc-meta");
  let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
  let stderr = String::from_utf8_lossy(&out.stderr).to_string();
  let _ = std::fs::remove_file(&path);
  assert!(
    out.status.success(),
    "pnixc-meta failed: stdout={stdout}, stderr={stderr}"
  );
  stdout
}

fn proposal(row: &[(&str, &str)]) -> CandidateRowProposal {
  let mut crp = CandidateRowProposal {
    candidate_kind: CandidateKind::RecurringImportSpec,
    target_owner: "test-owner".to_string(),
    target_table: "test-table".to_string(),
    proposed_row: Default::default(),
    supporting_evidence: vec![],
    evidence_count: 0,
    gate_status: GateStatus::IntentReceiptOnly,
    reason: "v0.6 byte-equivalence smoke".to_string(),
  };
  for (k, v) in row {
    crp.proposed_row.insert((*k).to_string(), (*v).to_string());
  }
  crp
}

fn px_pairs_literal(row: &[(&str, &str)]) -> String {
  // Build `.px` `[ {key=...; value=...; } ... ]` literal. Rust's
  // BTreeMap iter order is alphabetical; we sort here to match.
  let mut sorted: Vec<(&str, &str)> = row.iter().copied().collect();
  sorted.sort_by_key(|p| p.0);
  let mut s = String::from("[");
  for (k, v) in &sorted {
    s.push_str(" { key = \"");
    s.push_str(k);
    s.push_str("\"; value = \"");
    // Escape the value the same way Nix string literals need:
    // backslash and double-quote get backslash-escaped.
    for c in v.chars() {
      match c {
        '\\' => s.push_str("\\\\"),
        '"' => s.push_str("\\\""),
        _ => s.push(c),
      }
    }
    s.push_str("\"; }");
  }
  s.push_str(" ]");
  s
}

fn assert_byte_equivalent(name: &str, row: &[(&str, &str)]) {
  let p = proposal(row);
  let rust_folded = fold_proposal_with_format(&p, "multi-line-attrset");
  assert_eq!(
    rust_folded.outcome,
    MacroFoldOutcome::Folded,
    "Rust folder must succeed on this input (rust reason: {})",
    rust_folded.reason
  );
  let rust_text = rust_folded.folded_source_text;

  let pairs_lit = px_pairs_literal(row);
  let source = format!(
    "let mfg = import {gate}; pairs = {pairs}; in mfg.foldAttrsetTextWithDefaultFormat pairs",
    gate = macro_fold_gate_px().to_str().expect("path utf-8"),
    pairs = pairs_lit
  );
  let pnixc_meta_json = run_pnixc_meta(name, &source);
  // pnixc-meta emits the string JSON-encoded; decode.
  let px_text: String =
    serde_json::from_str(&pnixc_meta_json).expect("pnixc-meta stdout is JSON string");

  assert_eq!(
    rust_text, px_text,
    "macro-fold-gate body output diverged between Rust and .px:\nRust=<<<{rust_text}>>>\n.px=<<<{px_text}>>>"
  );
}

#[test]
fn macro_fold_gate_px_body_byte_equivalent_3_pair_input() {
  assert_byte_equivalent(
    "macro_fold_3pair",
    &[
      ("cue", "verb:rename"),
      ("intent", "refactor"),
      ("weight", "0.8"),
    ],
  );
}

#[test]
fn macro_fold_gate_px_body_byte_equivalent_with_special_chars() {
  // Backslashes and double-quotes inside values must be escaped
  // identically by Rust and `.px` bodies.
  assert_byte_equivalent(
    "macro_fold_special_chars",
    &[
      ("path", "C:\\Users\\demo"),
      ("quote", "she said \"hi\""),
      ("plain", "normal value"),
    ],
  );
}

#[test]
fn macro_fold_gate_px_body_byte_equivalent_single_pair() {
  assert_byte_equivalent("macro_fold_single", &[("only", "value")]);
}

#[test]
fn macro_fold_gate_px_body_byte_equivalent_six_pair_input() {
  assert_byte_equivalent(
    "macro_fold_6pair",
    &[
      ("a", "alpha"),
      ("b", "beta"),
      ("c", "gamma"),
      ("d", "delta"),
      ("e", "epsilon"),
      ("f", "zeta"),
    ],
  );
}
