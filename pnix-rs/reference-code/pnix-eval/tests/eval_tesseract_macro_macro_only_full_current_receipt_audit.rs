//! Full current receipt audit receipt.
//!
//! This pins the current receipt graph audit after bounded replay and
//! post-replay p-puck freshness. The receipt closes only full current receipt
//! audit; it does not boot the macro-only runtime or remove host code.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../..")
}

fn fixture_path() -> PathBuf {
  repo_root()
    .join("fixtures/tesseract-macro-legacy-probe/macro_only_full_current_receipt_audit_receipt.px")
}

fn as_attrs(v: &Value) -> &BTreeMap<String, Value> {
  match v {
    Value::AttrSet(m) => m,
    other => panic!("expected attrset, got {:?}", other),
  }
}

fn as_list(v: &Value) -> &Vec<Value> {
  match v {
    Value::List(items) => items,
    other => panic!("expected list, got {:?}", other),
  }
}

fn as_str(v: &Value) -> &str {
  match v {
    Value::String(s) => s,
    Value::StringContext { text, .. } => text,
    other => panic!("expected string, got {:?}", other),
  }
}

fn as_bool(v: &Value) -> bool {
  match v {
    Value::Bool(b) => *b,
    other => panic!("expected bool, got {:?}", other),
  }
}

fn as_i64(v: &Value) -> i64 {
  match v {
    Value::Int(i) => *i,
    other => panic!("expected int, got {:?}", other),
  }
}

fn get<'a>(v: &'a Value, key: &str) -> &'a Value {
  let attrs = as_attrs(v);
  attrs.get(key).unwrap_or_else(|| {
    panic!(
      "missing key `{}`; available: {:?}",
      key,
      attrs.keys().collect::<Vec<_>>()
    )
  })
}

fn get_path<'a>(root: &'a Value, path: &[&str]) -> &'a Value {
  let mut cur = root;
  for key in path {
    cur = get(cur, key);
  }
  cur
}

fn string_set(v: &Value) -> BTreeSet<&str> {
  as_list(v).iter().map(as_str).collect()
}

fn attrs_by_id<'a>(items: &'a Value) -> BTreeMap<&'a str, &'a Value> {
  as_list(items)
    .iter()
    .map(|item| (as_str(get(item, "id")), item))
    .collect()
}

#[test]
fn marker_and_owner_surfaces_are_pinned() {
  let run = eval_file(&fixture_path()).expect("full current receipt audit receipt");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-macro-only-full-current-receipt-audit"
  );
  assert_eq!(
    as_str(get(&run, "truth-owner")),
    "project-wiki/maps/tesseract-macro-ontology-discovery-ledger.md"
  );
  assert_eq!(
    as_str(get(&run, "constitution-owner")),
    "stdlib/lib/gate/tesseract-constitution.px"
  );
  for path in [
    "stdlib/lib/gate/macro-only-boot-full-current-receipt-audit.px",
    "fixtures/pnix-query-runtime/macro-only-boot-full-current-receipt-audit-owner.px",
    "fixtures/tesseract-macro-legacy-probe/macro_only_full_current_receipt_audit_receipt.px",
  ] {
    assert!(repo_root().join(path).is_file(), "missing `{path}`");
  }
}

#[test]
fn constitution_gate_blocks_full_audit_overclaims() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "macro-only-full-current-receipt-audit"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));

  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "full-current-receipt-audit-equals-boot-success",
    "full-current-receipt-audit-equals-runtime-owner",
    "full-current-receipt-audit-equals-semantic-owner",
    "full-current-receipt-audit-equals-host-removal",
    "compare-all-green-equals-host-delete",
    "wiki-smoke-green-equals-new-engine-from-zero",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
}

#[test]
fn contract_closes_full_current_receipt_audit_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let contract = get(&run, "full-current-receipt-audit-contract");
  assert_eq!(
    as_str(get(contract, "id")),
    "contract.macro-only-full-current-receipt-audit.v1"
  );
  assert_eq!(
    as_str(get(contract, "owner")),
    "stdlib.lib.gate.macro-only-boot-full-current-receipt-audit"
  );
  assert_eq!(
    as_str(get(contract, "current-status")),
    "full-current-receipt-audit-present"
  );
  assert!(as_bool(get(contract, "closes-full-current-receipt-audit")));
  assert_eq!(as_i64(get(contract, "total-tests")), 915);
  assert_eq!(as_i64(get(contract, "source-tracked")), 18167);
  assert_eq!(as_i64(get(contract, "source-indexed")), 18167);
  for key in [
    "closes-boot-execution-proof",
    "closes-macro-only-runtime-owner",
    "closes-new-engine-from-zero",
    "closes-host-removal",
    "closes-delete-ready-targets",
    "closes-semantic-owner-proof",
  ] {
    assert!(!as_bool(get(contract, key)), "`{key}` must stay false");
  }
}

#[test]
fn proof_records_compare_smoke_diff_and_puck_evidence() {
  let run = eval_file(&fixture_path()).unwrap();
  let proof = get(&run, "full-current-receipt-audit-proof");
  assert_eq!(
    as_str(get(proof, "status")),
    "full-current-receipt-audit-present"
  );
  assert!(as_bool(get(proof, "full-current-receipt-audit")));
  assert!(as_bool(get(proof, "compare-all-proof")));
  assert!(as_bool(get(proof, "wiki-map-smoke-proof")));
  assert!(as_bool(get(proof, "diff-check-proof")));
  assert!(as_bool(get(
    proof,
    "post-bounded-replay-p-puck-current-cut-input"
  )));
  assert_eq!(as_i64(get(proof, "total-tests")), 915);
  assert_eq!(as_i64(get(proof, "focused-total-tests")), 16);
  assert_eq!(as_i64(get(proof, "source-tracked")), 18167);
  assert_eq!(as_i64(get(proof, "source-indexed")), 18167);
  assert_eq!(
    as_str(get(proof, "p-puck-report-name")),
    "macro-only-current-cut-bounded-replay"
  );
  assert_eq!(as_i64(get(proof, "p-puck-duration-ms")), 4934);
  assert!(!as_bool(get(proof, "boot-executed")));
  assert!(!as_bool(get(proof, "semantic-owner")));
  assert!(!as_bool(get(proof, "host-code-removal-started")));
}

#[test]
fn trials_cover_valid_audit_and_held_overclaims() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "full-current-receipt-audit-trials"));
  assert_eq!(trials.len(), 11);
  assert_eq!(
    as_str(get(
      trials["trial.A.valid-full-current-receipt-audit"],
      "outcome"
    )),
    "full-current-receipt-audit-present"
  );
  assert_eq!(
    as_str(get(trials["trial.B.post-replay-puck-input"], "outcome")),
    "tesseract-macro-ontology-macro-only-post-bounded-replay-p-puck-current-cut"
  );
  for (id, held) in [
    (
      "trial.C.post-replay-puck-missing",
      "held.macro-only-full-current-audit.post-replay-puck-missing",
    ),
    (
      "trial.D.compare-total-mismatch",
      "held.macro-only-full-current-audit.compare-all-mismatch",
    ),
    (
      "trial.E.focused-mode-mismatch",
      "held.macro-only-full-current-audit.focused-mode-mismatch",
    ),
    (
      "trial.F.wiki-smoke-mismatch",
      "held.macro-only-full-current-audit.wiki-smoke-mismatch",
    ),
    (
      "trial.G.diff-check-missing",
      "held.macro-only-full-current-audit.diff-check-missing",
    ),
    (
      "trial.H.boot-claim",
      "held.macro-only-full-current-audit.boot-or-runtime-claim",
    ),
    (
      "trial.I.host-removal-claim",
      "held.macro-only-full-current-audit.host-removal-claim",
    ),
    (
      "trial.J.semantic-owner-claim",
      "held.macro-only-full-current-audit.semantic-owner-claim",
    ),
    (
      "trial.K.gpl-family-dependency",
      "held.macro-only-full-current-audit.gpl-family-dependency",
    ),
  ] {
    assert_eq!(as_str(get(trials[id], "outcome")), "Held");
    assert_eq!(as_str(get(trials[id], "held-id")), held);
  }
}

#[test]
fn six_layer_fold_records_graph_audit_without_runtime_install() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-full-current-receipt-audit-fold");
  for layer in [
    "surface", "ontology", "semantic", "gate", "runtime", "audit",
  ] {
    assert!(
      as_bool(get(get(fold, layer), "visible")),
      "layer `{layer}` invisible"
    );
  }
  assert!(as_bool(get_path(
    fold,
    &["ontology", "full-current-receipt-audit"]
  )));
  assert_eq!(
    as_i64(get_path(fold, &["ontology", "compare-all-total-tests"])),
    915
  );
  assert!(as_bool(get_path(
    fold,
    &["ontology", "source-inventory-parity"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "graph-audit-is-not-semantic-owner"]
  )));
  assert!(!as_bool(get_path(fold, &["runtime", "boot-executed"])));
  assert_eq!(
    as_i64(get_path(fold, &["audit", "p-puck-duration-ms"])),
    4934
  );
}

#[test]
fn migration_delta_closes_full_audit_and_keeps_boot_open() {
  let run = eval_file(&fixture_path()).unwrap();
  let delta = get(&run, "migration-delta");
  let closes = string_set(get(delta, "closes"));
  assert!(closes.contains("need.bootstrap.full-current-receipt-audit-after-bounded-replay"));
  let not = string_set(get(delta, "does-not-close"));
  assert!(not.contains("need.bootstrap.macro-only-boot-execution-proof"));
  assert!(not.contains("need.bootstrap.macro-only-runtime-owner-boot"));
  assert!(not.contains("need.host-removal.host-code-removal-execution"));
  let next = string_set(get(delta, "next-required"));
  assert!(next.contains("macro-only-boot-execution-proof-after-full-current-receipt-audit"));
  assert!(next.contains("host-code-removal-execution-proof-after-successful-boot"));
}

#[test]
fn discoveries_record_d452_through_d459() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 8);
  for expected in [
    "D452.full-current-receipt-audit-follows-post-replay-puck",
    "D453.full-audit-binds-compare-all-total",
    "D454.full-audit-binds-focused-post-replay-mode",
    "D455.wiki-smoke-and-source-parity-become-audit-input",
    "D456.diff-check-becomes-current-graph-cleanliness-input",
    "D457.full-audit-is-not-boot-or-semantic-owner",
    "D458.full-audit-gives-next-boot-proof-floor",
    "D459.audit-metrics-become-self-optimization-baseline",
  ] {
    let d = discoveries
      .get(expected)
      .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
    assert!(as_bool(get(d, "scenario-only")));
    assert_eq!(as_str(get(d, "decision-pressure")), "keep");
  }
}

#[test]
fn top_level_state_stays_non_boot_non_delete_non_semantic() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "replacement-readiness")),
    "full-current-receipt-audit-present-not-booted"
  );
  assert!(as_bool(get(&run, "full-current-receipt-audit")));
  assert!(as_bool(get(
    &run,
    "current-receipt-audit-after-bounded-replay"
  )));
  assert!(as_bool(get(&run, "compare-all-proof")));
  assert!(as_bool(get(&run, "wiki-map-smoke-proof")));
  assert!(as_bool(get(&run, "diff-check-proof")));
  for key in [
    "boot-executed",
    "macro-only-runtime-owner-booted",
    "new-engine-from-zero",
    "runtime-install",
    "global-ontology-runtime",
    "host-code-removal-started",
    "host-removal-safe",
    "semantic-owner",
    "old-host-authority",
    "gpl-family-dependencies",
    "implementation-command",
  ] {
    assert!(!as_bool(get(&run, key)), "`{key}` must stay false");
  }
  assert_eq!(as_i64(get(&run, "delete-ready-target-count")), 0);
}
