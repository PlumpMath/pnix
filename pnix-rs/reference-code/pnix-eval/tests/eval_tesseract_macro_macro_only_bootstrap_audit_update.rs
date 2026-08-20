//! Macro-only bootstrap audit update receipt.
//!
//! This pins the bootstrap status audit update as a `.px` owner output that
//! the boot runner can consume. It closes only that frontier: p-puck freshness,
//! compare-after-boot, replay execution, boot success, and target-specific host
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
    .join("fixtures/tesseract-macro-legacy-probe/macro_only_bootstrap_audit_update_receipt.px")
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
fn bootstrap_audit_update_marker_and_owner_surfaces_are_pinned() {
  let run =
    eval_file(&fixture_path()).expect("macro-only bootstrap audit update receipt must eval");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-macro-only-bootstrap-audit-update"
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
    "stdlib/lib/gate/macro-only-boot-bootstrap-audit-update.px",
    "fixtures/pnix-query-runtime/macro-only-boot-bootstrap-audit-update-owner.px",
    "fixtures/tesseract-macro-legacy-probe/macro_only_bootstrap_audit_update_receipt.px",
  ] {
    assert!(repo_root().join(path).is_file(), "missing `{path}`");
  }
}

#[test]
fn constitution_gate_blocks_audit_update_overclaims() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "macro-only-bootstrap-audit-update"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "audit-update-present-equals-p-puck-fresh",
    "audit-update-present-equals-compare-after-boot",
    "audit-update-present-equals-replay-executed",
    "audit-update-present-equals-boot-executed",
    "audit-update-present-equals-host-delete-proof",
    "full-bootstrap-import-equals-audit-update-proof",
    "stale-bootstrap-stage-equals-current-audit",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
}

#[test]
fn audit_update_contract_closes_bootstrap_audit_frontier_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let contract = get(&run, "audit-update-contract");
  assert_eq!(
    as_str(get(contract, "id")),
    "contract.macro-only-bootstrap-audit-update.v1"
  );
  assert_eq!(
    as_str(get(contract, "owner")),
    "stdlib.lib.gate.macro-only-boot-bootstrap-audit-update"
  );
  assert_eq!(
    as_str(get(contract, "constructor")),
    "validateBootstrapAuditUpdate"
  );
  assert_eq!(
    as_str(get(contract, "expected-current-stage")),
    "macro-only-regression-corpus-retention-present"
  );
  assert_eq!(as_i64(get(contract, "required-evidence-count")), 14);
  assert_eq!(as_i64(get(contract, "required-open-frontier-count")), 3);
  assert!(as_bool(get(
    contract,
    "closes-bootstrap-audit-update-frontier"
  )));
  for key in [
    "closes-p-puck-proof",
    "closes-compare-after-boot",
    "closes-replay-execution-proof",
    "closes-boot-execution-proof",
    "closes-host-delete-proof",
    "owns-p-puck",
    "owns-compare",
    "runtime-install",
    "global-ontology-runtime",
  ] {
    assert!(!as_bool(get(contract, key)), "`{key}` must stay false");
  }
}

#[test]
fn valid_audit_update_is_present_but_not_puck_compare_replay_or_boot() {
  let run = eval_file(&fixture_path()).unwrap();
  let update = get(&run, "bootstrap-audit-update");
  assert_eq!(
    as_str(get(update, "status")),
    "bootstrap-status-audit-update-plan-present"
  );
  assert!(as_bool(get(
    update,
    "bootstrap-status-audit-update-plan-present"
  )));
  assert_eq!(as_list(get(update, "missing")).len(), 0);
  assert_eq!(as_list(get(update, "required-evidence")).len(), 14);
  assert_eq!(as_list(get(update, "required-open-frontiers")).len(), 3);
  for key in [
    "fresh-p-puck-after-current-cut",
    "compare-after-boot",
    "replay-executed",
    "boot-executed",
    "macro-only-runtime-owner-booted",
    "new-engine-from-zero",
    "runtime-install",
    "global-ontology-runtime",
    "host-code-removal-started",
    "gpl-family-dependencies",
  ] {
    assert!(!as_bool(get(update, key)), "`{key}` must stay false");
  }
}

#[test]
fn runner_after_audit_removes_strategy_corpus_and_audit_missing_evidence() {
  let run = eval_file(&fixture_path()).unwrap();
  let runner = get(&run, "runner-after-strategy-corpus-and-audit");
  assert_eq!(as_str(get(runner, "status")), "Held");
  assert_eq!(
    as_str(get(runner, "held-id")),
    "held.macro-only-boot-runner.missing-required-evidence"
  );
  assert!(!as_bool(get(runner, "ready-for-bounded-replay")));
  assert!(!as_bool(get(runner, "boot-executed")));
  let missing = string_set(get(runner, "missing"));
  assert!(!missing.contains("bounded-replay-strategy-present"));
  assert!(!missing.contains("regression-corpus-transfer-present"));
  assert!(!missing.contains("bootstrap-status-audit-update-plan-present"));
  for expected in [
    "fresh-p-puck-after-current-cut",
    "compare-after-boot",
    "target-specific-delete-proof-present",
  ] {
    assert!(
      missing.contains(expected),
      "missing Held evidence `{expected}`"
    );
  }
  assert_eq!(missing.len(), 3);
}

#[test]
fn all_evidence_after_audit_is_replay_ready_but_still_not_boot() {
  let run = eval_file(&fixture_path()).unwrap();
  let ready = get(&run, "replay-ready-after-all-evidence");
  assert_eq!(
    as_str(get(ready, "status")),
    "runner-ready-for-bounded-replay"
  );
  assert!(as_bool(get(ready, "ready-for-bounded-replay")));
  assert!(matches!(get(ready, "held-id"), Value::Null));
  for key in [
    "boot-executed",
    "macro-only-runtime-owner-booted",
    "new-engine-from-zero",
    "runtime-install",
    "global-ontology-runtime",
    "host-code-removal-started",
  ] {
    assert!(!as_bool(get(ready, key)), "`{key}` must stay false");
  }
}

#[test]
fn audit_trials_cover_valid_runner_and_blocked_overclaims() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "audit-trials"));
  assert_eq!(trials.len(), 9);

  let valid = trials.get("trial.A.valid-bootstrap-audit-update").unwrap();
  assert_eq!(
    as_str(get(valid, "outcome")),
    "bootstrap-status-audit-update-plan-present"
  );
  assert!(as_bool(get(
    valid,
    "bootstrap-status-audit-update-plan-present"
  )));
  assert!(!as_bool(get(valid, "boot-executed")));

  let runner = trials
    .get("trial.B.runner-after-strategy-corpus-and-audit")
    .unwrap();
  assert_eq!(as_str(get(runner, "outcome")), "Held");
  assert!(!as_bool(get(
    runner,
    "bounded-replay-strategy-still-missing"
  )));
  assert!(!as_bool(get(runner, "regression-corpus-still-missing")));
  assert!(!as_bool(get(
    runner,
    "bootstrap-audit-update-still-missing"
  )));
  assert!(!as_bool(get(runner, "boot-executed")));

  for expected in [
    "trial.C.missing-open-frontier",
    "trial.D.stale-current-stage",
    "trial.E.missing-false-state",
    "trial.F.audit-as-external-audit",
    "trial.G.audit-as-boot",
    "trial.H.audit-as-host-delete",
    "trial.I.gpl-family-dependency",
  ] {
    let trial = trials
      .get(expected)
      .unwrap_or_else(|| panic!("missing trial `{expected}`"));
    assert_eq!(as_str(get(trial, "outcome")), "Held");
  }
}

#[test]
fn six_layer_fold_keeps_audit_update_from_runtime_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-audit-update-fold");
  assert_eq!(
    as_str(get(fold, "mode")),
    "macro-only-bootstrap-audit-update"
  );
  for layer in [
    "surface", "ontology", "semantic", "gate", "runtime", "audit",
  ] {
    assert!(as_bool(get_path(fold, &[layer, "visible"])));
  }
  assert_eq!(
    as_str(get_path(fold, &["surface", "owner-path"])),
    "stdlib/lib/gate/macro-only-boot-bootstrap-audit-update.px"
  );
  assert!(as_bool(get_path(
    fold,
    &["surface", "shallow-status-snapshot"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["ontology", "audit-update-present"]
  )));
  assert_eq!(
    as_i64(get_path(fold, &["ontology", "required-evidence-count"])),
    14
  );
  assert_eq!(
    as_str(get_path(fold, &["ontology", "runner-after-audit-status"])),
    "Held"
  );
  assert!(as_bool(get_path(
    fold,
    &["semantic", "audit-update-is-runner-evidence-not-boot-proof"]
  )));
  assert!(as_bool(get_path(
    fold,
    &[
      "semantic",
      "full-bootstrap-graph-import-remains-overflow-risk"
    ]
  )));
  assert_eq!(
    as_i64(get_path(
      fold,
      &["gate", "runner-missing-after-audit-count"]
    )),
    3
  );
  assert!(!as_bool(get_path(fold, &["runtime", "replay-executed"])));
  assert!(!as_bool(get_path(fold, &["runtime", "boot-executed"])));
  assert!(!as_bool(get_path(
    fold,
    &["audit", "fresh-p-puck-after-current-cut"]
  )));
  assert!(!as_bool(get_path(fold, &["audit", "compare-after-boot"])));
}

#[test]
fn migration_delta_closes_audit_update_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let delta = get(&run, "migration-delta");
  assert_eq!(
    as_str(get(delta, "id")),
    "migration-delta.macro-only-bootstrap-audit-update"
  );
  assert!(string_set(get(delta, "closes"))
    .contains("need.bootstrap.bootstrap-status-audit-update-after-boot"));
  let not = string_set(get(delta, "does-not-close"));
  for expected in [
    "need.bootstrap.macro-only-boot-execution-proof",
    "need.bootstrap.macro-only-runtime-owner-boot",
    "need.bootstrap.new-engine-from-zero-proof",
    "need.bootstrap.fresh-p-puck-after-current-cut",
    "need.bootstrap.compare-after-boot",
    "need.host-removal.target-specific-delete-proof",
  ] {
    assert!(not.contains(expected), "missing open frontier `{expected}`");
  }
  let next = string_set(get(delta, "next-required"));
  assert!(next.contains("fresh-p-puck-receipt-audit-after-current-cut"));
  assert!(next.contains("compare-after-boot"));
  assert!(next.contains("target-specific-host-delete-proof-after-successful-boot"));
}

#[test]
fn discoveries_record_d390_through_d398() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 9);
  for expected in [
    "D390.bootstrap-audit-update-owner-is-px-owner-not-boot-proof",
    "D391.bootstrap-audit-update-uses-shallow-status-snapshot",
    "D392.full-bootstrap-graph-import-overflow-stays-negative-evidence",
    "D393.audit-update-records-false-runtime-and-delete-states",
    "D394.audit-update-cannot-manufacture-fresh-puck-or-compare",
    "D395.stale-bootstrap-stage-is-held-before-runner-evidence",
    "D396.strategy-corpus-audit-fed-runner-removes-three-missing-evidence-items",
    "D397.audit-update-turns-wiki-status-sync-into-consumable-runner-evidence",
    "D398.next-frontier-is-fresh-puck-compare-and-target-delete-proof",
  ] {
    let discovery = discoveries
      .get(expected)
      .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
    assert!(as_bool(get(discovery, "scenario-only")));
  }
}

#[test]
fn inherited_status_links_to_corpus_without_booting() {
  let run = eval_file(&fixture_path()).unwrap();
  let status = get(&run, "inherited-status");
  assert_eq!(
    as_str(get(status, "macro-only-boot-runner-owner")),
    "tesseract-macro-ontology-macro-only-boot-runner-owner"
  );
  assert!(as_bool(get(status, "macro-only-boot-runner-owner-present")));
  assert_eq!(
    as_str(get(status, "bounded-replay-strategy")),
    "tesseract-macro-ontology-macro-only-bounded-replay-strategy"
  );
  assert!(as_bool(get(
    status,
    "bounded-full-graph-replay-strategy-present"
  )));
  assert_eq!(
    as_str(get(status, "regression-corpus-retention")),
    "tesseract-macro-ontology-macro-only-regression-corpus-retention"
  );
  assert!(as_bool(get(status, "regression-corpus-transfer-present")));
  assert_eq!(as_str(get(status, "runner-after-corpus-status")), "Held");
  assert_eq!(as_i64(get(status, "runner-after-corpus-missing-count")), 4);
  assert!(!as_bool(get(status, "previous-boot-executed")));
  assert!(!as_bool(get(status, "previous-new-engine-from-zero")));
  assert_eq!(as_i64(get(status, "previous-delete-ready-target-count")), 0);
}

#[test]
fn top_level_state_records_audit_update_without_boot_or_delete() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "replacement-readiness")),
    "macro-only-bootstrap-audit-update-present"
  );
  assert!(as_bool(get(&run, "macro-only-boot-manifest-written")));
  assert!(as_bool(get(&run, "macro-only-boot-execution-attempted")));
  assert!(as_bool(get(&run, "macro-only-boot-runner-owner-present")));
  assert!(as_bool(get(
    &run,
    "bounded-full-graph-replay-strategy-present"
  )));
  assert!(as_bool(get(&run, "regression-corpus-transfer-present")));
  assert!(as_bool(get(
    &run,
    "bootstrap-status-audit-update-plan-present"
  )));
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
    "compare-after-boot",
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
fn negative_held_evidence_rejects_audit_update_shortcuts() {
  let run = eval_file(&fixture_path()).unwrap();
  let rejects = string_set(get_path(&run, &["negative-held-evidence", "rejects"]));
  for expected in [
    "audit-update-present-as-p-puck-freshness",
    "audit-update-present-as-compare-after-boot",
    "audit-update-present-as-replay-executed",
    "audit-update-present-as-boot-executed",
    "audit-update-present-as-host-delete-proof",
    "full-bootstrap-import-as-audit-update-proof",
    "stale-bootstrap-stage-as-current",
    "missing-false-runtime-state-as-current",
    "gpl-family-dependency-in-audit-update",
  ] {
    assert!(rejects.contains(expected), "missing reject `{expected}`");
  }
}
