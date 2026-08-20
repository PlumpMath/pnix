//! Macro-only boot execution attempt for the tesseract macro migration.
//!
//! The boot manifest is written, but that is not a boot. This test pins the
//! next honest state: the shallow manifest loads as attempt input, while boot
//! execution, macro-only runtime ownership, host deletion, and new-engine-from-
//! zero remain Held until runner, p-puck, compare, and replay gates exist.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../..")
}

fn fixture_path() -> PathBuf {
  repo_root()
    .join("fixtures/tesseract-macro-legacy-probe/macro_only_boot_execution_attempt_receipt.px")
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
fn macro_only_boot_attempt_marker_and_constitution_owner_are_pinned() {
  let run = eval_file(&fixture_path()).expect("macro-only boot attempt must evaluate");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-macro-only-boot-execution-attempt"
  );
  assert_eq!(
    as_str(get(&run, "truth-owner")),
    "project-wiki/maps/tesseract-macro-ontology-discovery-ledger.md"
  );
  assert_eq!(
    as_str(get(&run, "replacement-map")),
    "project-wiki/maps/tesseract-macro-ontology-replacement-map.md"
  );
  assert_eq!(
    as_str(get(&run, "migration-map")),
    "project-wiki/maps/tesseract-macro-ontology-migration-algorithm-map.md"
  );
  assert_eq!(
    as_str(get(&run, "constitution-owner")),
    "stdlib/lib/gate/tesseract-constitution.px"
  );
}

#[test]
fn manifest_input_loads_shallow_manifest_without_claiming_boot() {
  let run = eval_file(&fixture_path()).unwrap();
  let manifest_path = repo_root()
    .join("fixtures/tesseract-macro-legacy-probe/macro_only_ontology_boot_manifest_receipt.px");
  assert!(manifest_path.is_file());

  let input = get(&run, "manifest-input");
  assert_eq!(
    as_str(get(input, "marker")),
    "tesseract-macro-ontology-macro-only-boot-manifest"
  );
  assert!(as_bool(get(input, "manifest-written")));
  assert!(as_bool(get(input, "manifest-complete")));
  assert_eq!(
    as_str(get(input, "manifest-verdict")),
    "manifest-written-boot-held"
  );
  assert_eq!(as_i64(get(input, "boot-surface-count")), 8);
  assert_eq!(as_i64(get(input, "excluded-host-authority-count")), 5);
  assert!(!as_bool(get(input, "old-host-authority")));
  assert!(as_bool(get(input, "old-host-code-still-present")));
  assert!(!as_bool(get(input, "boot-executed")));
  assert!(!as_bool(get(input, "macro-only-runtime-owner-booted")));
  assert!(!as_bool(get(input, "new-engine-from-zero")));
  assert_eq!(as_i64(get(input, "delete-ready-target-count")), 0);
}

#[test]
fn constitution_gate_blocks_attempt_as_success_shortcuts() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "macro-only-boot-execution-attempt"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));

  let held_if = string_set(get(gate, "held-if"));
  for expected in [
    "manifest-missing",
    "manifest-treated-as-boot-success",
    "full-receipt-graph-import-treated-as-proof",
    "boot-runner-owner-missing",
    "fresh-p-puck-after-current-cut-missing",
    "compare-after-boot-missing",
    "regression-corpus-transfer-missing",
    "target-specific-delete-proof-missing",
    "old-host-code-deleted-from-attempt",
    "new-engine-from-zero-claimed-from-attempt",
  ] {
    assert!(held_if.contains(expected), "missing held-if `{expected}`");
  }

  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "boot-attempt-equals-boot-success",
    "manifest-load-equals-runtime-owner",
    "full-import-overflow-retry-as-proof",
    "stale-p-puck-audit-equals-fresh-current-cut",
    "missing-p-puck-command-is-ok",
    "compare-before-boot-equals-compare-after-boot",
    "delete-host-code-from-held-attempt",
    "claim-new-engine-from-zero-without-replay",
    "llm-prose-equals-boot-runner-owner",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
}

#[test]
fn boot_execution_gate_records_attempt_but_keeps_success_false() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "boot-execution-gate");
  assert_eq!(
    as_str(get(gate, "id")),
    "gate.macro-only-boot.execution-attempt.v1"
  );
  assert!(as_bool(get(gate, "attempted")));
  assert!(as_bool(get(gate, "manifest-loaded")));
  assert!(as_bool(get(gate, "shallow-manifest-evaluated")));
  assert_eq!(
    as_str(get(gate, "manifest-marker")),
    "tesseract-macro-ontology-macro-only-boot-manifest"
  );
  assert_eq!(as_str(get(gate, "verdict")), "boot-execution-held");
  assert!(as_str(get(gate, "observed-full-graph-import-result")).contains("overflowed"));
  for key in [
    "full-receipt-graph-import-replayed",
    "full-receipt-graph-import-allowed-in-attempt",
    "boot-runner-owner-present",
    "p-puck-command-present-in-current-path",
    "fresh-p-puck-after-current-cut",
    "compare-after-boot",
    "regression-corpus-transfer-present",
    "target-specific-delete-proof-present",
    "old-host-authority",
    "boot-executed",
    "macro-only-runtime-owner-booted",
    "new-engine-from-zero",
  ] {
    assert!(!as_bool(get(gate, key)), "`{key}` must stay false");
  }
  assert!(as_bool(get(gate, "old-host-code-still-present")));
  assert_eq!(as_i64(get(gate, "delete-ready-target-count")), 0);

  let reasons = string_set(get(gate, "held-reasons"));
  for expected in [
    "boot-runner-owner-missing",
    "full-graph-replay-strategy-missing",
    "fresh-p-puck-after-current-cut-missing",
    "compare-after-boot-missing",
    "regression-corpus-transfer-missing",
    "target-specific-delete-proof-missing",
  ] {
    assert!(
      reasons.contains(expected),
      "missing held reason `{expected}`"
    );
  }
}

#[test]
fn boot_execution_trials_hold_each_missing_success_gate() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "boot-execution-trials"));
  assert_eq!(trials.len(), 7);
  for expected in [
    "trial.A.no-manifest-input",
    "trial.B.full-graph-import-as-boot-proof",
    "trial.C.boot-runner-owner-missing",
    "trial.D.p-puck-freshness-missing",
    "trial.E.compare-after-boot-missing",
    "trial.F.host-delete-from-held-attempt",
  ] {
    let trial = trials
      .get(expected)
      .unwrap_or_else(|| panic!("missing trial `{expected}`"));
    assert_eq!(as_str(get(trial, "outcome")), "Held");
    assert!(!as_bool(get(trial, "boot-executed")));
  }

  let loaded = trials
    .get("trial.G.shallow-manifest-loaded")
    .expect("manifest-loaded trial");
  assert_eq!(as_str(get(loaded, "outcome")), "attempt-recorded-boot-held");
  assert!(as_bool(get(loaded, "manifest-loaded")));
  assert!(as_bool(get(loaded, "attempted")));
  assert!(!as_bool(get(loaded, "boot-executed")));
  assert!(!as_bool(get(loaded, "macro-only-runtime-owner-booted")));
  assert!(!as_bool(get(loaded, "new-engine-from-zero")));
}

#[test]
fn six_layer_fold_keeps_attempt_runtime_and_audit_gates_separate() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-boot-execution-attempt-fold");
  assert_eq!(
    as_str(get(fold, "mode")),
    "macro-only-boot-execution-attempt"
  );
  for layer in [
    "surface", "ontology", "semantic", "gate", "runtime", "audit",
  ] {
    assert!(
      as_bool(get_path(fold, &[layer, "visible"])),
      "layer `{layer}` must be visible"
    );
  }
  assert!(as_bool(get_path(fold, &["surface", "manifest-loaded"])));
  assert!(!as_bool(get_path(
    fold,
    &["surface", "p-puck-command-present-in-current-path"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["ontology", "manifest-composition-known"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["ontology", "old-host-authority"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["ontology", "boot-runner-owner-present"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "attempt-is-not-success"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "full-graph-import-is-not-proof"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["semantic", "boot-execution-proven"]
  )));
  assert_eq!(
    as_str(get_path(fold, &["gate", "execution-verdict"])),
    "boot-execution-held"
  );
  assert_eq!(as_i64(get_path(fold, &["gate", "held-reason-count"])), 6);
  assert!(!as_bool(get_path(fold, &["runtime", "boot-executed"])));
  assert!(!as_bool(get_path(
    fold,
    &["runtime", "macro-only-runtime-owner-booted"]
  )));
  assert!(!as_bool(get_path(fold, &["runtime", "runtime-install"])));
  assert!(as_bool(get_path(fold, &["audit", "p-puck-fresh-required"])));
  assert!(!as_bool(get_path(
    fold,
    &["audit", "p-puck-fresh-after-current-cut"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["audit", "compare-after-boot-required"]
  )));
}

#[test]
fn migration_delta_closes_attempt_record_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let delta = get(&run, "migration-delta");
  assert_eq!(
    as_str(get(delta, "id")),
    "migration-delta.macro-only-boot-execution-attempt"
  );
  let closes = string_set(get(delta, "closes"));
  assert!(closes.contains("need.bootstrap.macro-only-boot-execution-attempt-record"));

  let not = string_set(get(delta, "does-not-close"));
  for expected in [
    "need.bootstrap.macro-only-boot-execution-proof",
    "need.bootstrap.macro-only-runtime-owner-boot",
    "need.bootstrap.new-engine-from-zero-proof",
    "need.bootstrap.fresh-p-puck-after-current-cut",
    "need.bootstrap.compare-after-boot",
    "need.bootstrap.full-graph-replay-strategy",
    "need.host-removal.target-specific-delete-proof",
  ] {
    assert!(not.contains(expected), "missing open frontier `{expected}`");
  }

  let next = string_set(get(delta, "next-required"));
  assert!(next.contains("macro-only-boot-runner-owner"));
  assert!(next.contains("bounded-full-graph-replay-strategy"));
  assert!(next.contains("fresh-p-puck-receipt-audit-after-current-cut"));
  assert!(next.contains("compare-after-boot"));
  assert!(next.contains("bootstrap-status-audit-after-boot-attempt"));
}

#[test]
fn discoveries_record_d354_through_d362() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 9);
  for expected in [
    "D354.macro-only-boot-execution-attempt-separates-attempt-from-success",
    "D355.shallow-manifest-is-valid-boot-attempt-input",
    "D356.full-receipt-graph-import-is-not-the-boot-proof-path",
    "D357.boot-runner-owner-is-open-frontier",
    "D358.p-puck-freshness-is-an-execution-gate",
    "D359.compare-after-boot-is-required-before-new-engine-from-zero",
    "D360.old-host-authority-remains-excluded-during-boot-attempt",
    "D361.host-delete-targets-remain-zero-after-held-boot-attempt",
    "D362.held-boot-attempt-narrows-next-proof-obligations",
  ] {
    let discovery = discoveries
      .get(expected)
      .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
    assert!(as_bool(get(discovery, "scenario-only")));
  }
}

#[test]
fn inherited_status_keeps_manifest_and_external_dependency_boundaries() {
  let run = eval_file(&fixture_path()).unwrap();
  let status = get(&run, "inherited-status");
  assert_eq!(
    as_str(get(status, "macro-only-boot-manifest")),
    "tesseract-macro-ontology-macro-only-boot-manifest"
  );
  assert!(as_bool(get(status, "manifest-written")));
  assert!(as_bool(get(status, "manifest-complete")));
  assert_eq!(
    as_str(get(status, "manifest-verdict")),
    "manifest-written-boot-held"
  );
  assert_eq!(as_i64(get(status, "previous-delete-ready-target-count")), 0);
  assert!(!as_bool(get(
    status,
    "previous-macro-only-runtime-owner-booted"
  )));
  assert!(!as_bool(get(status, "previous-new-engine-from-zero")));
  assert_eq!(as_i64(get(status, "external-solver-dependency-count")), 0);
  assert!(!as_bool(get(status, "gpl-family-dependencies")));
}

#[test]
fn top_level_state_is_attempted_but_not_booted() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "replacement-readiness")),
    "macro-only-boot-execution-attempt-held"
  );
  assert!(!as_bool(get(&run, "owner-switch")));
  assert!(as_bool(get(&run, "macro-only-boot-manifest-written")));
  assert!(as_bool(get(&run, "macro-only-boot-execution-attempted")));
  for key in [
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
fn negative_evidence_rejects_boot_success_and_host_delete_shortcuts() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get_path(&run, &["negative-held-evidence", "status"])),
    "present"
  );
  let rejects = string_set(get_path(&run, &["negative-held-evidence", "rejects"]));
  for expected in [
    "boot-attempt-as-boot-success",
    "manifest-load-as-runtime-owner",
    "full-graph-import-overflow-as-proof",
    "stale-p-puck-as-current-cut-proof",
    "missing-p-puck-command-as-fresh-audit",
    "compare-before-boot-as-compare-after-boot",
    "held-attempt-as-host-delete-proof",
    "llm-prose-as-boot-runner-owner",
  ] {
    assert!(rejects.contains(expected), "missing reject `{expected}`");
  }
}
