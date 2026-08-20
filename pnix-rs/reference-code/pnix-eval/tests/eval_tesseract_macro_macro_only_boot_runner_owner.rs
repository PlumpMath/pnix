//! Macro-only boot runner owner receipt.
//!
//! This pins the next bootstrap slice after the boot execution attempt: a
//! stdlib `.px` runner owner exists and evaluates boot evidence, but it only
//! closes the runner-owner frontier. Boot execution, new-engine-from-zero, p-
//! puck freshness, compare-after-boot, and host deletion remain open.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../..")
}

fn fixture_path() -> PathBuf {
  repo_root().join("fixtures/tesseract-macro-legacy-probe/macro_only_boot_runner_owner_receipt.px")
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
fn runner_owner_receipt_marker_and_owner_surfaces_are_pinned() {
  let run = eval_file(&fixture_path()).expect("macro-only boot runner owner receipt must eval");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-macro-only-boot-runner-owner"
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
    .join("stdlib/lib/gate/macro-only-boot-runner.px")
    .is_file());
  assert!(repo_root()
    .join("fixtures/pnix-query-runtime/macro-only-boot-runner-owner.px")
    .is_file());
}

#[test]
fn constitution_gate_blocks_runner_owner_overclaims() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "macro-only-boot-runner-owner"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "runner-owner-equals-runtime-boot",
    "runner-ready-equals-new-engine-from-zero",
    "runner-owner-equals-p-puck-fresh",
    "runner-owner-equals-compare-after-boot",
    "runner-owner-equals-host-delete-proof",
    "old-host-authority-through-runner",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
}

#[test]
fn runner_owner_contract_closes_runner_frontier_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let contract = get(&run, "runner-owner-contract");
  assert_eq!(
    as_str(get(contract, "id")),
    "contract.macro-only-boot-runner-owner.v1"
  );
  assert_eq!(
    as_str(get(contract, "owner")),
    "stdlib.lib.gate.macro-only-boot-runner"
  );
  assert_eq!(as_str(get(contract, "constructor")), "runBootAttempt");
  assert_eq!(
    as_str(get(contract, "runner-id")),
    "runner.macro-only-boot.v1"
  );
  assert!(as_bool(get(contract, "closes-runner-owner-frontier")));
  for key in [
    "closes-boot-execution-proof",
    "owns-p-puck",
    "owns-compare",
    "runtime-install",
    "global-ontology-runtime",
  ] {
    assert!(!as_bool(get(contract, key)), "`{key}` must stay false");
  }
  let required = string_set(get(contract, "required-evidence"));
  assert!(required.contains("bounded-replay-strategy-present"));
  assert!(required.contains("fresh-p-puck-after-current-cut"));
  assert!(required.contains("compare-after-boot"));
  assert!(required.contains("target-specific-delete-proof-present"));
}

#[test]
fn current_runner_evaluation_is_held_with_missing_evidence_vector() {
  let run = eval_file(&fixture_path()).unwrap();
  let current = get(&run, "current-runner-evaluation");
  assert_eq!(as_str(get(current, "status")), "Held");
  assert_eq!(
    as_str(get(current, "held-id")),
    "held.macro-only-boot-runner.missing-required-evidence"
  );
  assert!(as_bool(get(current, "boot-runner-owner-present")));
  assert!(!as_bool(get(current, "ready-for-bounded-replay")));
  let missing = string_set(get(current, "missing"));
  for expected in [
    "bounded-replay-strategy-present",
    "fresh-p-puck-after-current-cut",
    "compare-after-boot",
    "regression-corpus-transfer-present",
    "bootstrap-status-audit-update-plan-present",
    "target-specific-delete-proof-present",
  ] {
    assert!(missing.contains(expected), "missing `{expected}`");
  }
}

#[test]
fn replay_ready_candidate_still_does_not_execute_boot() {
  let run = eval_file(&fixture_path()).unwrap();
  let ready = get(&run, "replay-ready-candidate");
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
fn runner_trials_cover_current_ready_and_overclaim_cases() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "runner-trials"));
  assert_eq!(trials.len(), 5);

  let current = trials.get("trial.A.current-attempt").unwrap();
  assert_eq!(as_str(get(current, "outcome")), "Held");
  assert_eq!(
    as_str(get(current, "held-id")),
    "held.macro-only-boot-runner.missing-required-evidence"
  );
  assert!(as_bool(get(current, "boot-runner-owner-present")));
  assert!(!as_bool(get(current, "boot-executed")));

  let ready = trials.get("trial.B.replay-ready-candidate").unwrap();
  assert_eq!(
    as_str(get(ready, "outcome")),
    "runner-ready-for-bounded-replay"
  );
  assert!(as_bool(get(ready, "ready-for-bounded-replay")));
  assert!(!as_bool(get(ready, "boot-executed")));
  assert!(!as_bool(get(ready, "new-engine-from-zero")));

  for expected in [
    "trial.C.runner-owner-as-boot-success",
    "trial.D.runner-ready-as-new-engine",
    "trial.E.old-host-authority-through-runner",
  ] {
    let trial = trials
      .get(expected)
      .unwrap_or_else(|| panic!("missing trial `{expected}`"));
    assert_eq!(as_str(get(trial, "outcome")), "Held");
  }
}

#[test]
fn six_layer_fold_keeps_runner_owner_from_runtime_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-runner-owner-fold");
  assert_eq!(as_str(get(fold, "mode")), "macro-only-boot-runner-owner");
  for layer in [
    "surface", "ontology", "semantic", "gate", "runtime", "audit",
  ] {
    assert!(as_bool(get_path(fold, &[layer, "visible"])));
  }
  assert_eq!(
    as_str(get_path(fold, &["surface", "owner-path"])),
    "stdlib/lib/gate/macro-only-boot-runner.px"
  );
  assert!(as_bool(get_path(
    fold,
    &["ontology", "runner-owner-present"]
  )));
  assert_eq!(
    as_str(get_path(fold, &["ontology", "current-attempt-status"])),
    "Held"
  );
  assert_eq!(
    as_str(get_path(
      fold,
      &["ontology", "replay-ready-candidate-status"]
    )),
    "runner-ready-for-bounded-replay"
  );
  assert!(as_bool(get_path(
    fold,
    &["semantic", "runner-owner-closes-only-runner-frontier"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "replay-ready-is-not-boot-executed"]
  )));
  assert!(!as_bool(get_path(fold, &["runtime", "boot-executed"])));
  assert!(!as_bool(get_path(
    fold,
    &["runtime", "macro-only-runtime-owner-booted"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["audit", "fresh-p-puck-after-current-cut"]
  )));
  assert!(!as_bool(get_path(fold, &["audit", "compare-after-boot"])));
}

#[test]
fn migration_delta_closes_runner_owner_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let delta = get(&run, "migration-delta");
  assert_eq!(
    as_str(get(delta, "id")),
    "migration-delta.macro-only-boot-runner-owner"
  );
  assert!(string_set(get(delta, "closes")).contains("need.bootstrap.macro-only-boot-runner-owner"));
  let not = string_set(get(delta, "does-not-close"));
  for expected in [
    "need.bootstrap.macro-only-boot-execution-proof",
    "need.bootstrap.macro-only-runtime-owner-boot",
    "need.bootstrap.new-engine-from-zero-proof",
    "need.bootstrap.bounded-full-graph-replay-strategy",
    "need.bootstrap.fresh-p-puck-after-current-cut",
    "need.bootstrap.compare-after-boot",
    "need.host-removal.target-specific-delete-proof",
  ] {
    assert!(not.contains(expected), "missing open frontier `{expected}`");
  }
  let next = string_set(get(delta, "next-required"));
  assert!(next.contains("bounded-full-graph-replay-strategy"));
  assert!(next.contains("fresh-p-puck-receipt-audit-after-current-cut"));
  assert!(next.contains("compare-after-boot"));
}

#[test]
fn discoveries_record_d363_through_d371() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 9);
  for expected in [
    "D363.macro-only-boot-runner-owner-is-px-owner-not-runtime-boot",
    "D364.runner-consumes-shallow-manifest-and-attempt-evidence",
    "D365.current-attempt-is-held-by-runner-with-evidence-vector",
    "D366.all-evidence-payload-is-replay-ready-not-boot-executed",
    "D367.runner-blocks-old-host-authority-reentry",
    "D368.runner-does-not-own-p-puck-or-compare",
    "D369.runner-owner-does-not-create-host-delete-targets",
    "D370.runner-owner-closes-only-one-bootstrap-frontier",
    "D371.next-frontier-is-bounded-replay-strategy-before-boot-success",
  ] {
    let discovery = discoveries
      .get(expected)
      .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
    assert!(as_bool(get(discovery, "scenario-only")));
  }
}

#[test]
fn inherited_status_links_back_to_boot_attempt_without_changing_boot_state() {
  let run = eval_file(&fixture_path()).unwrap();
  let status = get(&run, "inherited-status");
  assert_eq!(
    as_str(get(status, "macro-only-boot-attempt")),
    "tesseract-macro-ontology-macro-only-boot-execution-attempt"
  );
  assert!(as_bool(get(status, "macro-only-boot-execution-attempted")));
  assert!(!as_bool(get(status, "previous-boot-executed")));
  assert!(!as_bool(get(
    status,
    "previous-macro-only-runtime-owner-booted"
  )));
  assert!(!as_bool(get(status, "previous-new-engine-from-zero")));
  assert_eq!(as_i64(get(status, "previous-delete-ready-target-count")), 0);
}

#[test]
fn top_level_state_records_runner_owner_without_boot_or_delete() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "replacement-readiness")),
    "macro-only-boot-runner-owner-present"
  );
  assert!(as_bool(get(&run, "macro-only-boot-manifest-written")));
  assert!(as_bool(get(&run, "macro-only-boot-execution-attempted")));
  assert!(as_bool(get(&run, "macro-only-boot-runner-owner-present")));
  for key in [
    "owner-switch",
    "boot-executed",
    "macro-only-runtime-owner-booted",
    "new-engine-from-zero",
    "old-host-authority",
    "host-code-removal-started",
    "host-removal-safe",
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
fn negative_held_evidence_rejects_runner_owner_shortcuts() {
  let run = eval_file(&fixture_path()).unwrap();
  let rejects = string_set(get_path(&run, &["negative-held-evidence", "rejects"]));
  for expected in [
    "runner-owner-as-runtime-boot",
    "runner-ready-as-new-engine-from-zero",
    "runner-owner-as-p-puck-freshness",
    "runner-owner-as-compare-after-boot",
    "runner-owner-as-host-delete-proof",
    "old-host-authority-through-runner",
  ] {
    assert!(rejects.contains(expected), "missing reject `{expected}`");
  }
}
