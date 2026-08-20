//! Macro-only regression corpus retention receipt.
//!
//! This pins the regression corpus transfer/retention proof as a `.px` owner
//! surface. It closes only that bootstrap frontier. The corpus proof does not
//! run compare, p-puck, replay, boot, runtime install, or host deletion.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../..")
}

fn fixture_path() -> PathBuf {
  repo_root()
    .join("fixtures/tesseract-macro-legacy-probe/macro_only_regression_corpus_retention_receipt.px")
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
fn regression_corpus_marker_and_owner_surfaces_are_pinned() {
  let run = eval_file(&fixture_path()).expect("macro-only regression corpus receipt must eval");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-macro-only-regression-corpus-retention"
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
    .join("stdlib/lib/gate/macro-only-boot-regression-corpus.px")
    .is_file());
  assert!(repo_root()
    .join("fixtures/pnix-query-runtime/macro-only-boot-regression-corpus-owner.px")
    .is_file());
}

#[test]
fn constitution_gate_blocks_corpus_overclaims() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "macro-only-regression-corpus-retention"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "corpus-present-equals-compare-after-boot",
    "corpus-present-equals-p-puck-fresh",
    "corpus-present-equals-replay-executed",
    "corpus-present-equals-boot-executed",
    "corpus-present-equals-host-delete-proof",
    "old-host-specimen-equals-old-host-authority",
    "gpl-family-dependency-in-regression-corpus",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
}

#[test]
fn corpus_contract_closes_regression_corpus_frontier_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let contract = get(&run, "corpus-retention-contract");
  assert_eq!(
    as_str(get(contract, "id")),
    "contract.macro-only-regression-corpus-retention.v1"
  );
  assert_eq!(
    as_str(get(contract, "owner")),
    "stdlib.lib.gate.macro-only-boot-regression-corpus"
  );
  assert_eq!(
    as_str(get(contract, "constructor")),
    "validateRegressionCorpus"
  );
  assert_eq!(
    as_str(get(contract, "corpus-id")),
    "corpus.macro-only-boot.regression-retention.v1"
  );
  assert_eq!(as_i64(get(contract, "required-corpus-count")), 11);
  assert_eq!(as_i64(get(contract, "required-evidence-count")), 10);
  assert!(as_bool(get(contract, "closes-regression-corpus-frontier")));
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
fn retained_corpus_is_present_but_not_compare_puck_replay_or_boot() {
  let run = eval_file(&fixture_path()).unwrap();
  let corpus = get(&run, "retained-corpus");
  assert_eq!(
    as_str(get(corpus, "status")),
    "regression-corpus-transfer-present"
  );
  assert!(as_bool(get(corpus, "regression-corpus-transfer-present")));
  assert_eq!(as_list(get(corpus, "missing")).len(), 0);
  assert_eq!(as_list(get(corpus, "required-corpus-ids")).len(), 11);
  assert_eq!(as_list(get(corpus, "required-evidence")).len(), 10);
  for key in [
    "compare-after-boot",
    "fresh-p-puck-after-current-cut",
    "replay-executed",
    "boot-executed",
    "macro-only-runtime-owner-booted",
    "new-engine-from-zero",
    "runtime-install",
    "global-ontology-runtime",
    "host-code-removal-started",
    "gpl-family-dependencies",
  ] {
    assert!(!as_bool(get(corpus, key)), "`{key}` must stay false");
  }
}

#[test]
fn runner_after_corpus_removes_strategy_and_corpus_missing_evidence_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let runner = get(&run, "runner-after-strategy-and-corpus");
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
  for expected in [
    "fresh-p-puck-after-current-cut",
    "compare-after-boot",
    "bootstrap-status-audit-update-plan-present",
    "target-specific-delete-proof-present",
  ] {
    assert!(
      missing.contains(expected),
      "missing Held evidence `{expected}`"
    );
  }
  assert_eq!(missing.len(), 4);
}

#[test]
fn all_evidence_after_corpus_is_replay_ready_but_still_not_boot() {
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
fn corpus_trials_cover_valid_runner_and_blocked_overclaims() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "corpus-trials"));
  assert_eq!(trials.len(), 8);

  let valid = trials.get("trial.A.valid-retained-corpus").unwrap();
  assert_eq!(
    as_str(get(valid, "outcome")),
    "regression-corpus-transfer-present"
  );
  assert!(as_bool(get(valid, "regression-corpus-transfer-present")));
  assert!(!as_bool(get(valid, "boot-executed")));

  let runner = trials
    .get("trial.B.runner-after-strategy-and-corpus")
    .unwrap();
  assert_eq!(as_str(get(runner, "outcome")), "Held");
  assert!(!as_bool(get(
    runner,
    "bounded-replay-strategy-still-missing"
  )));
  assert!(!as_bool(get(runner, "regression-corpus-still-missing")));
  assert!(!as_bool(get(runner, "boot-executed")));

  for expected in [
    "trial.C.missing-required-corpus",
    "trial.D.missing-negative-held",
    "trial.E.corpus-as-boot",
    "trial.F.corpus-as-external-audit",
    "trial.G.corpus-as-host-delete",
    "trial.H.gpl-family-dependency",
  ] {
    let trial = trials
      .get(expected)
      .unwrap_or_else(|| panic!("missing trial `{expected}`"));
    assert_eq!(as_str(get(trial, "outcome")), "Held");
  }
}

#[test]
fn six_layer_fold_keeps_corpus_from_runtime_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-corpus-retention-fold");
  assert_eq!(
    as_str(get(fold, "mode")),
    "macro-only-regression-corpus-retention"
  );
  for layer in [
    "surface", "ontology", "semantic", "gate", "runtime", "audit",
  ] {
    assert!(as_bool(get_path(fold, &[layer, "visible"])));
  }
  assert_eq!(
    as_str(get_path(fold, &["surface", "owner-path"])),
    "stdlib/lib/gate/macro-only-boot-regression-corpus.px"
  );
  assert!(as_bool(get_path(fold, &["ontology", "corpus-present"])));
  assert_eq!(
    as_i64(get_path(fold, &["ontology", "required-corpus-count"])),
    11
  );
  assert_eq!(
    as_str(get_path(fold, &["ontology", "runner-after-corpus-status"])),
    "Held"
  );
  assert!(as_bool(get_path(
    fold,
    &["semantic", "old-host-code-stays-specimen-not-authority"]
  )));
  assert!(as_bool(get_path(
    fold,
    &[
      "semantic",
      "corpus-removes-one-more-runner-missing-evidence"
    ]
  )));
  assert_eq!(
    as_i64(get_path(
      fold,
      &["gate", "runner-missing-after-corpus-count"]
    )),
    4
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
fn migration_delta_closes_regression_corpus_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let delta = get(&run, "migration-delta");
  assert_eq!(
    as_str(get(delta, "id")),
    "migration-delta.macro-only-regression-corpus-retention"
  );
  assert!(string_set(get(delta, "closes"))
    .contains("need.bootstrap.regression-corpus-transfer-or-retention-proof"));
  let not = string_set(get(delta, "does-not-close"));
  for expected in [
    "need.bootstrap.macro-only-boot-execution-proof",
    "need.bootstrap.macro-only-runtime-owner-boot",
    "need.bootstrap.new-engine-from-zero-proof",
    "need.bootstrap.fresh-p-puck-after-current-cut",
    "need.bootstrap.compare-after-boot",
    "need.bootstrap.bootstrap-status-audit-update-after-boot",
    "need.host-removal.target-specific-delete-proof",
  ] {
    assert!(not.contains(expected), "missing open frontier `{expected}`");
  }
  let next = string_set(get(delta, "next-required"));
  assert!(next.contains("fresh-p-puck-receipt-audit-after-current-cut"));
  assert!(next.contains("compare-after-boot"));
  assert!(next.contains("bootstrap-status-audit-update-after-corpus-proof"));
}

#[test]
fn discoveries_record_d381_through_d389() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 9);
  for expected in [
    "D381.regression-corpus-owner-is-px-owner-not-compare-or-replay",
    "D382.regression-corpus-binds-legacy-specimens-negative-held-and-rollback",
    "D383.old-host-code-stays-present-but-excluded-from-authority",
    "D384.corpus-proof-cannot-manufacture-fresh-puck-or-compare",
    "D385.corpus-proof-cannot-delete-host-code",
    "D386.gpl-family-dependency-is-held-inside-corpus-proof",
    "D387.strategy-plus-corpus-fed-runner-removes-two-missing-evidence-items",
    "D388.corpus-retention-keeps-future-host-deletion-reversible-and-auditable",
    "D389.next-frontier-is-fresh-puck-compare-bootstrap-audit-and-delete-proof",
  ] {
    let discovery = discoveries
      .get(expected)
      .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
    assert!(as_bool(get(discovery, "scenario-only")));
  }
}

#[test]
fn inherited_status_links_to_runner_and_strategy_without_booting() {
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
  assert_eq!(as_str(get(status, "runner-after-strategy-status")), "Held");
  assert_eq!(
    as_i64(get(status, "runner-after-strategy-missing-count")),
    5
  );
  assert!(!as_bool(get(status, "previous-boot-executed")));
  assert!(!as_bool(get(status, "previous-new-engine-from-zero")));
  assert_eq!(as_i64(get(status, "previous-delete-ready-target-count")), 0);
}

#[test]
fn top_level_state_records_corpus_without_boot_or_delete() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "replacement-readiness")),
    "macro-only-regression-corpus-retention-present"
  );
  assert!(as_bool(get(&run, "macro-only-boot-manifest-written")));
  assert!(as_bool(get(&run, "macro-only-boot-execution-attempted")));
  assert!(as_bool(get(&run, "macro-only-boot-runner-owner-present")));
  assert!(as_bool(get(
    &run,
    "bounded-full-graph-replay-strategy-present"
  )));
  assert!(as_bool(get(&run, "regression-corpus-transfer-present")));
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
fn negative_held_evidence_rejects_corpus_shortcuts() {
  let run = eval_file(&fixture_path()).unwrap();
  let rejects = string_set(get_path(&run, &["negative-held-evidence", "rejects"]));
  for expected in [
    "corpus-present-as-compare-after-boot",
    "corpus-present-as-p-puck-freshness",
    "corpus-present-as-replay-executed",
    "corpus-present-as-boot-executed",
    "corpus-present-as-host-delete-proof",
    "old-host-specimen-as-authority",
    "gpl-family-dependency-in-corpus",
    "corpus-proof-erases-negative-held",
  ] {
    assert!(rejects.contains(expected), "missing reject `{expected}`");
  }
}
