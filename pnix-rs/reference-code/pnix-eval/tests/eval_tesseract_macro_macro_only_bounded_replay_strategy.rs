//! Macro-only bounded replay strategy receipt.
//!
//! This pins the next bootstrap slice after the boot runner owner: a stdlib
//! `.px` owner validates the bounded full-graph replay strategy. It closes only
//! that strategy frontier. Replay execution, fresh p-puck, compare-after-boot,
//! regression corpus transfer, bootstrap audit update, host deletion, and boot
//! success remain unclaimed.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../..")
}

fn fixture_path() -> PathBuf {
  repo_root()
    .join("fixtures/tesseract-macro-legacy-probe/macro_only_bounded_replay_strategy_receipt.px")
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
fn bounded_replay_strategy_marker_and_owner_surfaces_are_pinned() {
  let run =
    eval_file(&fixture_path()).expect("macro-only bounded replay strategy receipt must eval");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-macro-only-bounded-replay-strategy"
  );
  assert_eq!(
    as_str(get(&run, "truth-owner")),
    "project-wiki/maps/tesseract-macro-ontology-discovery-ledger.md"
  );
  assert_eq!(
    as_str(get(&run, "constitution-owner")),
    "stdlib/lib/gate/tesseract-constitution.px"
  );
  assert!(repo_root()
    .join("stdlib/lib/gate/macro-only-boot-replay-strategy.px")
    .is_file());
  assert!(repo_root()
    .join("fixtures/pnix-query-runtime/macro-only-boot-replay-strategy-owner.px")
    .is_file());
}

#[test]
fn constitution_gate_blocks_strategy_overclaims() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "macro-only-bounded-replay-strategy"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "strategy-present-equals-replay-executed",
    "strategy-present-equals-boot-executed",
    "strategy-present-equals-new-engine-from-zero",
    "strategy-present-equals-p-puck-fresh",
    "strategy-present-equals-compare-after-boot",
    "strategy-present-equals-host-delete-proof",
    "full-import-overflow-retry-as-strategy-proof",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
}

#[test]
fn replay_strategy_contract_closes_strategy_frontier_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let contract = get(&run, "replay-strategy-contract");
  assert_eq!(
    as_str(get(contract, "id")),
    "contract.macro-only-bounded-replay-strategy.v1"
  );
  assert_eq!(
    as_str(get(contract, "owner")),
    "stdlib.lib.gate.macro-only-boot-replay-strategy"
  );
  assert_eq!(
    as_str(get(contract, "constructor")),
    "validateReplayStrategy"
  );
  assert_eq!(
    as_str(get(contract, "strategy-id")),
    "strategy.macro-only-boot.bounded-full-graph-replay.v1"
  );
  assert_eq!(as_i64(get(contract, "required-graph-node-count")), 11);
  assert_eq!(as_i64(get(contract, "required-bound-count")), 6);
  assert_eq!(as_i64(get(contract, "required-evidence-count")), 10);
  assert!(as_bool(get(
    contract,
    "closes-bounded-replay-strategy-frontier"
  )));
  for key in [
    "closes-replay-execution-proof",
    "closes-boot-execution-proof",
    "owns-p-puck",
    "owns-compare",
    "runtime-install",
    "global-ontology-runtime",
  ] {
    assert!(!as_bool(get(contract, key)), "`{key}` must stay false");
  }
}

#[test]
fn bounded_strategy_is_present_but_not_replay_or_boot() {
  let run = eval_file(&fixture_path()).unwrap();
  let strategy = get(&run, "bounded-strategy");
  assert_eq!(
    as_str(get(strategy, "status")),
    "bounded-replay-strategy-present"
  );
  assert!(as_bool(get(strategy, "bounded-replay-strategy-present")));
  assert_eq!(as_list(get(strategy, "missing")).len(), 0);
  assert_eq!(as_list(get(strategy, "graph-node-ids")).len(), 11);
  assert_eq!(as_list(get(strategy, "bound-ids")).len(), 6);
  for key in [
    "replay-executed",
    "boot-executed",
    "macro-only-runtime-owner-booted",
    "new-engine-from-zero",
    "fresh-p-puck-after-current-cut",
    "compare-after-boot",
    "runtime-install",
    "global-ontology-runtime",
    "host-code-removal-started",
  ] {
    assert!(!as_bool(get(strategy, key)), "`{key}` must stay false");
  }
}

#[test]
fn runner_after_strategy_removes_only_bounded_replay_missing_evidence() {
  let run = eval_file(&fixture_path()).unwrap();
  let runner = get(&run, "runner-after-strategy-only");
  assert_eq!(as_str(get(runner, "status")), "Held");
  assert_eq!(
    as_str(get(runner, "held-id")),
    "held.macro-only-boot-runner.missing-required-evidence"
  );
  assert!(!as_bool(get(runner, "ready-for-bounded-replay")));
  assert!(!as_bool(get(runner, "boot-executed")));
  let missing = string_set(get(runner, "missing"));
  assert!(!missing.contains("bounded-replay-strategy-present"));
  for expected in [
    "fresh-p-puck-after-current-cut",
    "compare-after-boot",
    "regression-corpus-transfer-present",
    "bootstrap-status-audit-update-plan-present",
    "target-specific-delete-proof-present",
  ] {
    assert!(
      missing.contains(expected),
      "missing Held evidence `{expected}`"
    );
  }
  assert_eq!(missing.len(), 5);
}

#[test]
fn all_evidence_after_strategy_is_replay_ready_but_still_not_boot() {
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
fn strategy_trials_cover_valid_runner_and_blocked_overclaims() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "strategy-trials"));
  assert_eq!(trials.len(), 6);

  let valid = trials.get("trial.A.valid-bounded-strategy").unwrap();
  assert_eq!(
    as_str(get(valid, "outcome")),
    "bounded-replay-strategy-present"
  );
  assert!(as_bool(get(valid, "bounded-replay-strategy-present")));
  assert!(!as_bool(get(valid, "replay-executed")));
  assert!(!as_bool(get(valid, "boot-executed")));

  let runner = trials.get("trial.B.runner-after-strategy-only").unwrap();
  assert_eq!(as_str(get(runner, "outcome")), "Held");
  assert!(!as_bool(get(
    runner,
    "bounded-replay-strategy-still-missing"
  )));
  assert!(!as_bool(get(runner, "boot-executed")));

  for expected in [
    "trial.C.unbounded-or-incomplete-graph",
    "trial.D.missing-cycle-guard",
    "trial.E.strategy-as-boot",
    "trial.F.strategy-as-external-audit",
  ] {
    let trial = trials
      .get(expected)
      .unwrap_or_else(|| panic!("missing trial `{expected}`"));
    assert_eq!(as_str(get(trial, "outcome")), "Held");
  }
}

#[test]
fn six_layer_fold_keeps_strategy_from_runtime_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-replay-strategy-fold");
  assert_eq!(
    as_str(get(fold, "mode")),
    "macro-only-bounded-replay-strategy"
  );
  for layer in [
    "surface", "ontology", "semantic", "gate", "runtime", "audit",
  ] {
    assert!(as_bool(get_path(fold, &[layer, "visible"])));
  }
  assert_eq!(
    as_str(get_path(fold, &["surface", "owner-path"])),
    "stdlib/lib/gate/macro-only-boot-replay-strategy.px"
  );
  assert!(as_bool(get_path(fold, &["ontology", "strategy-present"])));
  assert_eq!(
    as_i64(get_path(fold, &["ontology", "graph-node-count"])),
    11
  );
  assert_eq!(
    as_str(get_path(
      fold,
      &["ontology", "runner-after-strategy-status"]
    )),
    "Held"
  );
  assert!(as_bool(get_path(
    fold,
    &["semantic", "strategy-is-shape-not-execution"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "strategy-removes-one-runner-missing-evidence"]
  )));
  assert_eq!(
    as_i64(get_path(
      fold,
      &["gate", "runner-missing-after-strategy-count"]
    )),
    5
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
fn migration_delta_closes_bounded_strategy_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let delta = get(&run, "migration-delta");
  assert_eq!(
    as_str(get(delta, "id")),
    "migration-delta.macro-only-bounded-replay-strategy"
  );
  assert!(
    string_set(get(delta, "closes")).contains("need.bootstrap.bounded-full-graph-replay-strategy")
  );
  let not = string_set(get(delta, "does-not-close"));
  for expected in [
    "need.bootstrap.macro-only-boot-execution-proof",
    "need.bootstrap.macro-only-runtime-owner-boot",
    "need.bootstrap.new-engine-from-zero-proof",
    "need.bootstrap.fresh-p-puck-after-current-cut",
    "need.bootstrap.compare-after-boot",
    "need.bootstrap.regression-corpus-transfer-or-retention-proof",
    "need.bootstrap.bootstrap-status-audit-update-after-boot",
    "need.host-removal.target-specific-delete-proof",
  ] {
    assert!(not.contains(expected), "missing open frontier `{expected}`");
  }
  let next = string_set(get(delta, "next-required"));
  assert!(next.contains("fresh-p-puck-receipt-audit-after-current-cut"));
  assert!(next.contains("compare-after-boot"));
  assert!(next.contains("regression-corpus-transfer-or-retention-proof"));
}

#[test]
fn discoveries_record_d372_through_d380() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 9);
  for expected in [
    "D372.bounded-replay-strategy-owner-is-px-owner-not-replay-execution",
    "D373.replay-strategy-names-macro-boot-graph-nodes",
    "D374.replay-strategy-requires-node-edge-depth-cycle-stack-bounds",
    "D375.strategy-excludes-old-host-authority-from-replay-owner",
    "D376.strategy-cannot-manufacture-fresh-puck-compare-corpus-audit-or-delete-proof",
    "D377.strategy-fed-runner-removes-one-missing-evidence-only",
    "D378.full-import-overflow-does-not-become-replay-strategy",
    "D379.strategy-order-separates-owner-surfaces-from-regression-specimens",
    "D380.next-frontier-is-fresh-puck-compare-corpus-audit-delete",
  ] {
    let discovery = discoveries
      .get(expected)
      .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
    assert!(as_bool(get(discovery, "scenario-only")));
  }
}

#[test]
fn inherited_status_links_back_to_runner_without_changing_boot_state() {
  let run = eval_file(&fixture_path()).unwrap();
  let status = get(&run, "inherited-status");
  assert_eq!(
    as_str(get(status, "macro-only-boot-runner-owner")),
    "tesseract-macro-ontology-macro-only-boot-runner-owner"
  );
  assert!(as_bool(get(status, "macro-only-boot-runner-owner-present")));
  assert_eq!(
    as_str(get(status, "previous-runner-current-status")),
    "Held"
  );
  assert_eq!(as_i64(get(status, "previous-runner-missing-count")), 6);
  assert!(!as_bool(get(status, "previous-boot-executed")));
  assert!(!as_bool(get(status, "previous-new-engine-from-zero")));
  assert_eq!(as_i64(get(status, "previous-delete-ready-target-count")), 0);
}

#[test]
fn top_level_state_records_strategy_without_boot_or_delete() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "replacement-readiness")),
    "macro-only-bounded-replay-strategy-present"
  );
  assert!(as_bool(get(&run, "macro-only-boot-manifest-written")));
  assert!(as_bool(get(&run, "macro-only-boot-execution-attempted")));
  assert!(as_bool(get(&run, "macro-only-boot-runner-owner-present")));
  assert!(as_bool(get(
    &run,
    "bounded-full-graph-replay-strategy-present"
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
fn negative_held_evidence_rejects_strategy_shortcuts() {
  let run = eval_file(&fixture_path()).unwrap();
  let rejects = string_set(get_path(&run, &["negative-held-evidence", "rejects"]));
  for expected in [
    "strategy-present-as-replay-executed",
    "strategy-present-as-boot-executed",
    "strategy-present-as-new-engine-from-zero",
    "strategy-present-as-p-puck-freshness",
    "strategy-present-as-compare-after-boot",
    "strategy-present-as-host-delete-proof",
    "unbounded-full-import-as-strategy-proof",
  ] {
    assert!(rejects.contains(expected), "missing reject `{expected}`");
  }
}
