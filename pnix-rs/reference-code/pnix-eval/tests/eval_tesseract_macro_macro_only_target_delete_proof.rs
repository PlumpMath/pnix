//! Macro-only target-specific delete proof receipt.
//!
//! This pins target-specific host delete proof as a `.px` owner output after
//! target-delete preflight. It closes only the runner's delete-proof item:
//! fresh p-puck, replay execution, boot success, runtime ownership, and host
//! deletion remain open.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../..")
}

fn fixture_path() -> PathBuf {
  repo_root()
    .join("fixtures/tesseract-macro-legacy-probe/macro_only_target_delete_proof_receipt.px")
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

fn list_strings(v: &Value) -> Vec<&str> {
  as_list(v).iter().map(as_str).collect()
}

fn string_set(v: &Value) -> BTreeSet<&str> {
  list_strings(v).into_iter().collect()
}

fn attrs_by_id<'a>(items: &'a Value) -> BTreeMap<&'a str, &'a Value> {
  as_list(items)
    .iter()
    .map(|item| (as_str(get(item, "id")), item))
    .collect()
}

#[test]
fn target_specific_delete_proof_marker_and_owner_surfaces_are_pinned() {
  let run = eval_file(&fixture_path()).expect("macro-only target delete proof receipt");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-macro-only-target-specific-delete-proof"
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
    "stdlib/lib/gate/macro-only-boot-target-delete-proof.px",
    "fixtures/pnix-query-runtime/macro-only-boot-target-delete-proof-owner.px",
    "fixtures/tesseract-macro-legacy-probe/macro_only_target_delete_proof_receipt.px",
  ] {
    assert!(repo_root().join(path).is_file(), "missing `{path}`");
  }
}

#[test]
fn target_proof_targets_are_real_current_host_surfaces() {
  for path in [
    "stdlib/lib/ontology.px",
    "crates/pnix-runtime-legacy/src/ssa_eval/builtins/mod.rs",
    "crates/pnix-runtime-legacy/src/ir/eval.rs",
    "crates/pnix-core/src/ontology.rs",
    "crates/pnix-eval/tests/ontology_builtins.rs",
  ] {
    assert!(
      repo_root().join(path).is_file(),
      "target surface missing `{path}`"
    );
  }
}

#[test]
fn constitution_gate_blocks_target_proof_overclaims() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "macro-only-target-specific-delete-proof"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "target-specific-proof-equals-host-removal-started",
    "target-specific-proof-equals-delete-ready-targets",
    "target-specific-proof-equals-fresh-puck",
    "target-specific-proof-equals-replay-executed",
    "target-specific-proof-equals-boot-executed",
    "target-specific-proof-equals-runtime-owner",
    "target-specific-proof-equals-semantic-owner",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
}

#[test]
fn target_proof_contract_closes_delete_proof_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let contract = get(&run, "target-specific-delete-proof-contract");
  assert_eq!(
    as_str(get(contract, "id")),
    "contract.macro-only-target-specific-delete-proof.v1"
  );
  assert_eq!(
    as_str(get(contract, "owner")),
    "stdlib.lib.gate.macro-only-boot-target-delete-proof"
  );
  assert_eq!(
    as_str(get(contract, "constructor")),
    "validateTargetSpecificDeleteProof"
  );
  assert_eq!(
    as_str(get(contract, "expected-current-stage")),
    "macro-only-target-delete-preflight-present"
  );
  assert_eq!(as_i64(get(contract, "required-target-count")), 5);
  assert_eq!(as_i64(get(contract, "required-target-evidence-count")), 8);
  assert!(as_bool(get(
    contract,
    "closes-target-specific-delete-proof"
  )));
  for key in [
    "closes-fresh-puck-proof",
    "closes-replay-execution-proof",
    "closes-boot-execution-proof",
    "closes-host-removal",
    "closes-delete-ready-targets",
    "closes-semantic-owner-proof",
    "owns-p-puck",
    "owns-runtime",
    "owns-semantic-authority",
    "runtime-install",
    "global-ontology-runtime",
  ] {
    assert!(!as_bool(get(contract, key)), "`{key}` must stay false");
  }
}

#[test]
fn target_specific_proof_protects_targets_but_does_not_make_them_delete_ready() {
  let run = eval_file(&fixture_path()).unwrap();
  let proof = get(&run, "target-specific-delete-proof");
  assert_eq!(
    as_str(get(proof, "status")),
    "target-specific-delete-proof-present"
  );
  assert!(as_bool(get(proof, "target-delete-preflight-present")));
  assert!(as_bool(get(proof, "target-specific-delete-proof-present")));
  assert_eq!(as_list(get(proof, "targets")).len(), 5);
  assert_eq!(as_list(get(proof, "protected-targets")).len(), 5);
  assert_eq!(as_list(get(proof, "ready-targets")).len(), 0);
  assert_eq!(as_i64(get(proof, "delete-ready-target-count")), 0);
  for target in as_list(get(proof, "protected-targets")) {
    assert!(as_bool(get(target, "target-specific-proof-present")));
    assert!(as_bool(get(target, "caller-scan-present")));
    assert!(as_bool(get(target, "replacement-replay-binding-present")));
    assert!(as_bool(get(target, "rollback-binding-present")));
    assert!(as_bool(get(target, "regression-corpus-binding-present")));
    assert!(!as_bool(get(target, "delete-ready")));
    assert!(!as_bool(get(target, "remove-now")));
    assert!(!as_bool(get(target, "host-code-removal-started")));
  }
}

#[test]
fn runner_after_target_proof_is_held_only_on_fresh_puck() {
  let run = eval_file(&fixture_path()).unwrap();
  let runner = get(&run, "runner-after-target-specific-delete-proof");
  assert_eq!(as_str(get(runner, "status")), "Held");
  assert_eq!(
    as_str(get(runner, "held-id")),
    "held.macro-only-boot-runner.missing-required-evidence"
  );
  assert!(!as_bool(get(runner, "ready-for-bounded-replay")));
  assert!(!as_bool(get(runner, "boot-executed")));
  let missing = string_set(get(runner, "missing"));
  assert!(missing.contains("fresh-p-puck-after-current-cut"));
  assert!(!missing.contains("target-specific-delete-proof-present"));
  assert_eq!(missing.len(), 1);
}

#[test]
fn target_proof_trials_cover_valid_runner_and_blocked_overclaims() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "target-specific-delete-proof-trials"));
  assert_eq!(trials.len(), 13);

  let valid = trials
    .get("trial.A.valid-target-specific-delete-proof")
    .unwrap();
  assert_eq!(
    as_str(get(valid, "outcome")),
    "target-specific-delete-proof-present"
  );
  assert!(as_bool(get(valid, "target-specific-delete-proof-present")));
  assert_eq!(as_i64(get(valid, "delete-ready-target-count")), 0);
  assert_eq!(as_i64(get(valid, "protected-target-count")), 5);

  let runner = trials
    .get("trial.B.runner-after-target-specific-delete-proof")
    .unwrap();
  assert_eq!(as_str(get(runner, "outcome")), "Held");
  assert!(!as_bool(get(
    runner,
    "target-specific-delete-proof-still-missing"
  )));
  assert!(as_bool(get(runner, "fresh-p-puck-still-missing")));
  assert!(!as_bool(get(runner, "boot-executed")));

  for expected in [
    "trial.C.missing-target",
    "trial.D.missing-target-evidence",
    "trial.E.delete-ready-target",
    "trial.F.missing-global-evidence",
    "trial.G.stale-stage",
    "trial.H.wrong-proof-id",
    "trial.I.target-proof-as-fresh-puck",
    "trial.J.target-proof-as-boot",
    "trial.K.target-proof-as-host-removal",
    "trial.L.target-proof-as-semantic-owner",
    "trial.M.gpl-family-dependency",
  ] {
    let trial = trials
      .get(expected)
      .unwrap_or_else(|| panic!("missing trial `{expected}`"));
    assert_eq!(as_str(get(trial, "outcome")), "Held");
  }
}

#[test]
fn six_layer_fold_keeps_target_proof_from_runtime_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-target-specific-delete-proof-fold");
  assert_eq!(
    as_str(get(fold, "mode")),
    "macro-only-target-specific-delete-proof"
  );
  for layer in [
    "surface", "ontology", "semantic", "gate", "runtime", "audit",
  ] {
    assert!(as_bool(get_path(fold, &[layer, "visible"])));
  }
  assert_eq!(
    as_str(get_path(fold, &["surface", "owner-path"])),
    "stdlib/lib/gate/macro-only-boot-target-delete-proof.px"
  );
  assert_eq!(as_i64(get_path(fold, &["surface", "host-target-count"])), 5);
  assert!(as_bool(get_path(
    fold,
    &["ontology", "target-specific-delete-proof-present"]
  )));
  assert_eq!(
    as_i64(get_path(fold, &["ontology", "protected-target-count"])),
    5
  );
  assert_eq!(
    as_i64(get_path(fold, &["ontology", "ready-target-count"])),
    0
  );
  assert_eq!(
    as_i64(get_path(
      fold,
      &["gate", "runner-missing-after-target-proof-count"]
    )),
    1
  );
  assert!(as_bool(get_path(
    fold,
    &[
      "semantic",
      "target-proof-is-runner-evidence-not-host-removal"
    ]
  )));
  assert!(!as_bool(get_path(fold, &["runtime", "boot-executed"])));
  assert!(!as_bool(get_path(
    fold,
    &["runtime", "host-code-removal-started"]
  )));
}

#[test]
fn migration_delta_closes_target_specific_proof_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let delta = get(&run, "migration-delta");
  assert_eq!(
    as_str(get(delta, "id")),
    "migration-delta.macro-only-target-specific-delete-proof"
  );
  assert!(
    string_set(get(delta, "closes")).contains("need.host-removal.target-specific-delete-proof")
  );
  let not = string_set(get(delta, "does-not-close"));
  for expected in [
    "need.bootstrap.fresh-p-puck-after-current-cut",
    "need.bootstrap.bounded-replay-execution-proof",
    "need.bootstrap.macro-only-boot-execution-proof",
    "need.bootstrap.macro-only-runtime-owner-boot",
    "need.bootstrap.new-engine-from-zero-proof",
    "need.host-removal.host-code-removal-execution",
  ] {
    assert!(not.contains(expected), "missing open frontier `{expected}`");
  }
  let next = string_set(get(delta, "next-required"));
  assert!(next.contains("fresh-p-puck-receipt-audit-after-current-cut"));
  assert!(next.contains("bounded-replay-execution-proof-after-fresh-puck"));
}

#[test]
fn discoveries_record_d417_through_d425() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 9);
  for expected in [
    "D417.target-specific-delete-proof-owner-is-px-owner-not-removal",
    "D418.target-proof-requires-per-target-caller-replay-rollback-regression-bindings",
    "D419.target-specific-proof-protects-all-five-host-targets",
    "D420.target-specific-proof-removes-runner-delete-missing-only",
    "D421.target-proof-cannot-manufacture-fresh-puck",
    "D422.target-proof-cannot-manufacture-replay-boot-runtime-or-new-engine",
    "D423.target-proof-cannot-create-delete-ready-targets",
    "D424.target-proof-cannot-be-semantic-owner-or-gpl-intake",
    "D425.next-frontier-is-fresh-puck-before-bounded-replay",
  ] {
    let discovery = discoveries
      .get(expected)
      .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
    assert!(as_bool(get(discovery, "scenario-only")));
  }
}

#[test]
fn inherited_status_links_to_preflight_and_reduces_runner_missing_count_to_one() {
  let run = eval_file(&fixture_path()).unwrap();
  let status = get(&run, "inherited-status");
  assert_eq!(
    as_str(get(status, "target-delete-preflight")),
    "tesseract-macro-ontology-macro-only-target-delete-preflight"
  );
  assert!(as_bool(get(status, "target-delete-preflight-present")));
  assert_eq!(as_str(get(status, "runner-after-preflight-status")), "Held");
  assert_eq!(
    as_i64(get(status, "runner-after-preflight-missing-count")),
    2
  );
  assert_eq!(
    as_str(get(status, "runner-after-target-proof-status")),
    "Held"
  );
  assert_eq!(
    as_i64(get(status, "runner-after-target-proof-missing-count")),
    1
  );
  assert!(!as_bool(get(status, "previous-boot-executed")));
  assert!(!as_bool(get(status, "previous-new-engine-from-zero")));
  assert_eq!(as_i64(get(status, "previous-delete-ready-target-count")), 0);
}

#[test]
fn top_level_state_records_target_proof_without_boot_or_delete() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "replacement-readiness")),
    "macro-only-target-specific-delete-proof-present"
  );
  assert!(as_bool(get(&run, "target-delete-preflight-present")));
  assert!(as_bool(get(&run, "target-specific-delete-proof-present")));
  for key in [
    "owner-switch",
    "replay-executed",
    "boot-executed",
    "macro-only-runtime-owner-booted",
    "new-engine-from-zero",
    "old-host-authority",
    "host-code-removal-started",
    "host-removal-safe",
    "fresh-p-puck-after-current-cut",
    "runtime-install",
    "global-ontology-runtime",
    "implementation-command",
    "gpl-family-dependencies",
  ] {
    assert!(!as_bool(get(&run, key)), "`{key}` must stay false");
  }
  assert!(as_bool(get(&run, "old-host-code-still-present")));
  assert_eq!(as_i64(get(&run, "delete-ready-target-count")), 0);
  assert_eq!(as_i64(get(&run, "external-solver-dependency-count")), 0);
}

#[test]
fn negative_held_evidence_rejects_target_proof_shortcuts() {
  let run = eval_file(&fixture_path()).unwrap();
  let rejects = string_set(get_path(&run, &["negative-held-evidence", "rejects"]));
  for expected in [
    "target-specific-proof-as-host-removal-started",
    "target-specific-proof-as-delete-ready-targets",
    "target-specific-proof-as-p-puck-freshness",
    "target-specific-proof-as-replay-executed",
    "target-specific-proof-as-boot-executed",
    "target-specific-proof-as-runtime-owner",
    "target-specific-proof-as-new-engine-from-zero",
    "target-specific-proof-as-semantic-owner",
    "target-specific-proof-with-gpl-family-dependency",
  ] {
    assert!(rejects.contains(expected), "missing reject `{expected}`");
  }
}
