//! Macro-only compare-after-boot receipt.
//!
//! This pins compare-after-boot as a `.px` owner output that the boot runner
//! can consume. It closes only that frontier: p-puck freshness, replay
//! execution, boot success, and target-specific host deletion remain open.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../..")
}

fn fixture_path() -> PathBuf {
  repo_root()
    .join("fixtures/tesseract-macro-legacy-probe/macro_only_boot_compare_after_boot_receipt.px")
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
fn compare_after_boot_marker_and_owner_surfaces_are_pinned() {
  let run = eval_file(&fixture_path()).expect("macro-only compare-after-boot receipt must eval");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-macro-only-compare-after-boot"
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
    "stdlib/lib/gate/macro-only-boot-compare-after-boot.px",
    "fixtures/pnix-query-runtime/macro-only-boot-compare-after-boot-owner.px",
    "fixtures/tesseract-macro-legacy-probe/macro_only_boot_compare_after_boot_receipt.px",
  ] {
    assert!(repo_root().join(path).is_file(), "missing `{path}`");
  }
}

#[test]
fn constitution_gate_blocks_compare_overclaims() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "macro-only-compare-after-boot"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "compare-present-equals-p-puck-fresh",
    "compare-present-equals-replay-executed",
    "compare-present-equals-boot-executed",
    "compare-present-equals-semantic-owner",
    "compare-present-equals-host-delete-proof",
    "stale-compare-count-equals-current-compare",
    "partial-compare-mode-equals-all-compare",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
}

#[test]
fn compare_contract_closes_compare_frontier_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let contract = get(&run, "compare-contract");
  assert_eq!(
    as_str(get(contract, "id")),
    "contract.macro-only-compare-after-boot.v1"
  );
  assert_eq!(
    as_str(get(contract, "owner")),
    "stdlib.lib.gate.macro-only-boot-compare-after-boot"
  );
  assert_eq!(
    as_str(get(contract, "constructor")),
    "validateCompareAfterBoot"
  );
  assert_eq!(
    as_str(get(contract, "expected-current-stage")),
    "macro-only-bootstrap-audit-update-present"
  );
  assert_eq!(as_i64(get(contract, "expected-total-tests")), 799);
  assert_eq!(as_i64(get(contract, "required-evidence-count")), 16);
  assert_eq!(as_i64(get(contract, "required-open-frontier-count")), 2);
  assert!(as_bool(get(contract, "closes-compare-after-boot")));
  for key in [
    "closes-p-puck-proof",
    "closes-replay-execution-proof",
    "closes-boot-execution-proof",
    "closes-host-delete-proof",
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
fn valid_compare_is_present_but_not_puck_replay_boot_or_delete() {
  let run = eval_file(&fixture_path()).unwrap();
  let compare = get(&run, "compare-after-boot-proof");
  assert_eq!(as_str(get(compare, "status")), "compare-after-boot-present");
  assert!(as_bool(get(compare, "compare-after-boot")));
  assert_eq!(as_i64(get(compare, "total-tests")), 799);
  assert_eq!(as_list(get(compare, "missing")).len(), 0);
  assert_eq!(as_list(get(compare, "required-evidence")).len(), 16);
  assert_eq!(as_list(get(compare, "required-open-frontiers")).len(), 2);
  for key in [
    "fresh-p-puck-after-current-cut",
    "replay-executed",
    "boot-executed",
    "macro-only-runtime-owner-booted",
    "new-engine-from-zero",
    "runtime-install",
    "global-ontology-runtime",
    "host-code-removal-started",
    "gpl-family-dependencies",
    "semantic-owner",
  ] {
    assert!(!as_bool(get(compare, key)), "`{key}` must stay false");
  }
}

#[test]
fn runner_after_compare_removes_strategy_corpus_audit_and_compare_missing_evidence() {
  let run = eval_file(&fixture_path()).unwrap();
  let runner = get(&run, "runner-after-strategy-corpus-audit-and-compare");
  assert_eq!(as_str(get(runner, "status")), "Held");
  assert_eq!(
    as_str(get(runner, "held-id")),
    "held.macro-only-boot-runner.missing-required-evidence"
  );
  assert!(!as_bool(get(runner, "ready-for-bounded-replay")));
  assert!(!as_bool(get(runner, "boot-executed")));
  let missing = string_set(get(runner, "missing"));
  for absent in [
    "bounded-replay-strategy-present",
    "regression-corpus-transfer-present",
    "bootstrap-status-audit-update-plan-present",
    "compare-after-boot",
  ] {
    assert!(
      !missing.contains(absent),
      "`{absent}` should not remain missing"
    );
  }
  for expected in [
    "fresh-p-puck-after-current-cut",
    "target-specific-delete-proof-present",
  ] {
    assert!(
      missing.contains(expected),
      "missing Held evidence `{expected}`"
    );
  }
  assert_eq!(missing.len(), 2);
}

#[test]
fn all_evidence_after_compare_is_replay_ready_but_still_not_boot() {
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
fn compare_trials_cover_valid_runner_and_blocked_overclaims() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "compare-trials"));
  assert_eq!(trials.len(), 12);

  let valid = trials.get("trial.A.valid-compare-after-boot").unwrap();
  assert_eq!(as_str(get(valid, "outcome")), "compare-after-boot-present");
  assert!(as_bool(get(valid, "compare-after-boot")));
  assert_eq!(as_i64(get(valid, "total-tests")), 799);
  assert!(!as_bool(get(valid, "boot-executed")));

  let runner = trials
    .get("trial.B.runner-after-strategy-corpus-audit-and-compare")
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
  assert!(!as_bool(get(runner, "compare-after-boot-still-missing")));
  assert!(!as_bool(get(runner, "boot-executed")));

  for expected in [
    "trial.C.wrong-command",
    "trial.D.wrong-total",
    "trial.E.compare-failed",
    "trial.F.missing-open-frontier",
    "trial.G.missing-required-evidence",
    "trial.H.compare-as-fresh-puck",
    "trial.I.compare-as-boot",
    "trial.J.compare-as-semantic-owner",
    "trial.K.compare-as-host-delete",
    "trial.L.gpl-family-dependency",
  ] {
    let trial = trials
      .get(expected)
      .unwrap_or_else(|| panic!("missing trial `{expected}`"));
    assert_eq!(as_str(get(trial, "outcome")), "Held");
  }
}

#[test]
fn six_layer_fold_keeps_compare_from_runtime_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-compare-fold");
  assert_eq!(as_str(get(fold, "mode")), "macro-only-compare-after-boot");
  for layer in [
    "surface", "ontology", "semantic", "gate", "runtime", "audit",
  ] {
    assert!(as_bool(get_path(fold, &[layer, "visible"])));
  }
  assert_eq!(
    as_str(get_path(fold, &["surface", "owner-path"])),
    "stdlib/lib/gate/macro-only-boot-compare-after-boot.px"
  );
  assert_eq!(
    as_str(get_path(fold, &["surface", "compare-command"])),
    "bash scripts/tesseract-macro-ontology-compare.sh --all"
  );
  assert_eq!(
    as_i64(get_path(fold, &["surface", "expected-total-tests"])),
    799
  );
  assert!(as_bool(get_path(
    fold,
    &["ontology", "compare-after-boot-present"]
  )));
  assert_eq!(
    as_i64(get_path(fold, &["ontology", "required-evidence-count"])),
    16
  );
  assert_eq!(
    as_i64(get_path(
      fold,
      &["gate", "runner-missing-after-compare-count"]
    )),
    2
  );
  assert!(as_bool(get_path(
    fold,
    &["semantic", "compare-is-runner-evidence-not-semantic-owner"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "compare-does-not-prove-puck-boot-or-delete"]
  )));
  assert!(!as_bool(get_path(fold, &["runtime", "replay-executed"])));
  assert!(!as_bool(get_path(fold, &["runtime", "boot-executed"])));
  assert!(as_bool(get_path(fold, &["audit", "compare-after-boot"])));
  assert!(!as_bool(get_path(
    fold,
    &["audit", "fresh-p-puck-after-current-cut"]
  )));
}

#[test]
fn migration_delta_closes_compare_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let delta = get(&run, "migration-delta");
  assert_eq!(
    as_str(get(delta, "id")),
    "migration-delta.macro-only-compare-after-boot"
  );
  assert!(string_set(get(delta, "closes")).contains("need.bootstrap.compare-after-boot"));
  let not = string_set(get(delta, "does-not-close"));
  for expected in [
    "need.bootstrap.macro-only-boot-execution-proof",
    "need.bootstrap.macro-only-runtime-owner-boot",
    "need.bootstrap.new-engine-from-zero-proof",
    "need.bootstrap.fresh-p-puck-after-current-cut",
    "need.host-removal.target-specific-delete-proof",
  ] {
    assert!(not.contains(expected), "missing open frontier `{expected}`");
  }
  let next = string_set(get(delta, "next-required"));
  assert!(next.contains("fresh-p-puck-receipt-audit-after-current-cut"));
  assert!(next.contains("target-specific-host-delete-proof-after-successful-boot"));
  assert!(next.contains("bounded-replay-execution-proof-after-runner-ready"));
}

#[test]
fn discoveries_record_d399_through_d407() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 9);
  for expected in [
    "D399.compare-after-boot-owner-is-px-owner-not-puck-or-boot",
    "D400.compare-proof-lowers-all-harness-result-into-runner-evidence",
    "D401.compare-proof-is-stage-command-count-and-status-bound",
    "D402.compare-cannot-manufacture-fresh-puck",
    "D403.compare-cannot-manufacture-replay-boot-runtime-or-new-engine",
    "D404.compare-cannot-delete-host-code-or-add-gpl",
    "D405.strategy-corpus-audit-compare-fed-runner-removes-four-missing-evidence-items",
    "D406.compare-proof-is-not-semantic-owner",
    "D407.next-frontier-is-fresh-puck-and-target-delete-proof",
  ] {
    let discovery = discoveries
      .get(expected)
      .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
    assert!(as_bool(get(discovery, "scenario-only")));
  }
}

#[test]
fn inherited_status_links_to_audit_update_without_booting() {
  let run = eval_file(&fixture_path()).unwrap();
  let status = get(&run, "inherited-status");
  assert_eq!(
    as_str(get(status, "macro-only-boot-runner-owner")),
    "tesseract-macro-ontology-macro-only-boot-runner-owner"
  );
  assert!(as_bool(get(status, "macro-only-boot-runner-owner-present")));
  assert_eq!(
    as_str(get(status, "bootstrap-audit-update")),
    "tesseract-macro-ontology-macro-only-bootstrap-audit-update"
  );
  assert!(as_bool(get(
    status,
    "bootstrap-status-audit-update-plan-present"
  )));
  assert_eq!(as_str(get(status, "runner-after-audit-status")), "Held");
  assert_eq!(as_i64(get(status, "runner-after-audit-missing-count")), 3);
  assert!(!as_bool(get(status, "previous-boot-executed")));
  assert!(!as_bool(get(status, "previous-new-engine-from-zero")));
  assert_eq!(as_i64(get(status, "previous-delete-ready-target-count")), 0);
}

#[test]
fn top_level_state_records_compare_without_boot_or_delete() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "replacement-readiness")),
    "macro-only-compare-after-boot-present"
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
  assert!(as_bool(get(&run, "compare-after-boot")));
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
fn negative_held_evidence_rejects_compare_shortcuts() {
  let run = eval_file(&fixture_path()).unwrap();
  let rejects = string_set(get_path(&run, &["negative-held-evidence", "rejects"]));
  for expected in [
    "compare-present-as-p-puck-freshness",
    "compare-present-as-replay-executed",
    "compare-present-as-boot-executed",
    "compare-present-as-runtime-owner",
    "compare-present-as-new-engine-from-zero",
    "compare-present-as-semantic-owner",
    "compare-present-as-host-delete-proof",
    "stale-compare-stage-command-or-total",
    "failed-compare-as-ok",
    "gpl-family-dependency-in-compare-proof",
  ] {
    assert!(rejects.contains(expected), "missing reject `{expected}`");
  }
}
