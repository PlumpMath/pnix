//! Macro-only target delete preflight receipt.
//!
//! This pins target-delete preflight as a `.px` owner output. It closes only
//! the preflight step: target-specific delete proof, fresh p-puck, replay
//! execution, boot success, runtime ownership, and host deletion remain open.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../..")
}

fn fixture_path() -> PathBuf {
  repo_root()
    .join("fixtures/tesseract-macro-legacy-probe/macro_only_target_delete_preflight_receipt.px")
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
fn target_delete_preflight_marker_and_owner_surfaces_are_pinned() {
  let run = eval_file(&fixture_path()).expect("macro-only target delete preflight receipt");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-macro-only-target-delete-preflight"
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
    "stdlib/lib/gate/macro-only-boot-target-delete-preflight.px",
    "fixtures/pnix-query-runtime/macro-only-boot-target-delete-preflight-owner.px",
    "fixtures/tesseract-macro-legacy-probe/macro_only_target_delete_preflight_receipt.px",
  ] {
    assert!(repo_root().join(path).is_file(), "missing `{path}`");
  }
}

#[test]
fn constitution_gate_blocks_preflight_overclaims() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "macro-only-target-delete-preflight"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "preflight-present-equals-target-delete-proof",
    "preflight-present-equals-host-removal-started",
    "preflight-present-equals-delete-ready-targets",
    "preflight-present-equals-p-puck-fresh",
    "preflight-present-equals-replay-executed",
    "preflight-present-equals-boot-executed",
    "preflight-present-equals-semantic-owner",
    "host-removal-map-target-list-equals-delete-proof",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
}

#[test]
fn preflight_contract_closes_preflight_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let contract = get(&run, "target-delete-preflight-contract");
  assert_eq!(
    as_str(get(contract, "id")),
    "contract.macro-only-target-delete-preflight.v1"
  );
  assert_eq!(
    as_str(get(contract, "owner")),
    "stdlib.lib.gate.macro-only-boot-target-delete-preflight"
  );
  assert_eq!(
    as_str(get(contract, "constructor")),
    "validateTargetDeletePreflight"
  );
  assert_eq!(
    as_str(get(contract, "expected-current-stage")),
    "macro-only-compare-after-boot-present"
  );
  assert_eq!(as_i64(get(contract, "required-target-count")), 5);
  assert_eq!(as_i64(get(contract, "required-evidence-count")), 17);
  assert_eq!(as_i64(get(contract, "required-open-frontier-count")), 3);
  assert!(as_bool(get(contract, "closes-target-delete-preflight")));
  for key in [
    "closes-target-specific-delete-proof",
    "closes-p-puck-proof",
    "closes-replay-execution-proof",
    "closes-boot-execution-proof",
    "closes-host-removal",
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
fn valid_preflight_lists_targets_but_does_not_make_them_delete_ready() {
  let run = eval_file(&fixture_path()).unwrap();
  let proof = get(&run, "target-delete-preflight-proof");
  assert_eq!(
    as_str(get(proof, "status")),
    "target-delete-preflight-present"
  );
  assert!(as_bool(get(proof, "target-delete-preflight-present")));
  assert!(!as_bool(get(proof, "target-specific-delete-proof-present")));
  assert_eq!(as_list(get(proof, "targets")).len(), 5);
  assert_eq!(as_list(get(proof, "blocked-targets")).len(), 5);
  assert_eq!(as_list(get(proof, "ready-targets")).len(), 0);
  assert_eq!(as_i64(get(proof, "delete-ready-target-count")), 0);
  for blocked in as_list(get(proof, "blocked-targets")) {
    assert!(!as_bool(get(blocked, "delete-ready")));
    assert!(!as_bool(get(blocked, "target-specific-proof-present")));
    assert!(!as_bool(get(blocked, "remove-now")));
  }
}

#[test]
fn runner_after_preflight_still_has_fresh_puck_and_target_delete_missing() {
  let run = eval_file(&fixture_path()).unwrap();
  let runner = get(&run, "runner-after-target-delete-preflight");
  assert_eq!(as_str(get(runner, "status")), "Held");
  assert_eq!(
    as_str(get(runner, "held-id")),
    "held.macro-only-boot-runner.missing-required-evidence"
  );
  assert!(!as_bool(get(runner, "ready-for-bounded-replay")));
  assert!(!as_bool(get(runner, "boot-executed")));
  let missing = string_set(get(runner, "missing"));
  assert!(missing.contains("fresh-p-puck-after-current-cut"));
  assert!(missing.contains("target-specific-delete-proof-present"));
  assert_eq!(missing.len(), 2);
}

#[test]
fn future_delete_proof_without_fresh_puck_is_still_held() {
  let run = eval_file(&fixture_path()).unwrap();
  let runner = get(&run, "runner-with-future-delete-proof-but-no-puck");
  assert_eq!(as_str(get(runner, "status")), "Held");
  assert!(!as_bool(get(runner, "ready-for-bounded-replay")));
  let missing = string_set(get(runner, "missing"));
  assert!(missing.contains("fresh-p-puck-after-current-cut"));
  assert!(!missing.contains("target-specific-delete-proof-present"));
  assert_eq!(missing.len(), 1);
}

#[test]
fn preflight_trials_cover_valid_runner_and_blocked_overclaims() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "target-delete-preflight-trials"));
  assert_eq!(trials.len(), 12);

  let valid = trials.get("trial.A.valid-target-delete-preflight").unwrap();
  assert_eq!(
    as_str(get(valid, "outcome")),
    "target-delete-preflight-present"
  );
  assert!(as_bool(get(valid, "target-delete-preflight-present")));
  assert!(!as_bool(get(valid, "target-specific-delete-proof-present")));
  assert_eq!(as_i64(get(valid, "delete-ready-target-count")), 0);

  let runner = trials
    .get("trial.B.runner-after-target-delete-preflight")
    .unwrap();
  assert_eq!(as_str(get(runner, "outcome")), "Held");
  assert!(as_bool(get(
    runner,
    "target-specific-delete-proof-still-missing"
  )));
  assert!(as_bool(get(runner, "fresh-p-puck-still-missing")));
  assert!(!as_bool(get(runner, "boot-executed")));

  let future = trials
    .get("trial.C.future-delete-proof-without-puck")
    .unwrap();
  assert_eq!(as_str(get(future, "outcome")), "Held");
  assert!(!as_bool(get(future, "ready-for-bounded-replay")));
  assert_eq!(string_set(get(future, "missing")).len(), 1);

  for expected in [
    "trial.D.missing-target",
    "trial.E.stale-stage",
    "trial.F.wrong-preflight-id",
    "trial.G.missing-required-evidence",
    "trial.H.preflight-as-fresh-puck",
    "trial.I.preflight-as-boot",
    "trial.J.preflight-as-delete-proof",
    "trial.K.preflight-as-semantic-owner",
    "trial.L.gpl-family-dependency",
  ] {
    let trial = trials
      .get(expected)
      .unwrap_or_else(|| panic!("missing trial `{expected}`"));
    assert_eq!(as_str(get(trial, "outcome")), "Held");
  }
}

#[test]
fn six_layer_fold_keeps_preflight_from_runtime_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-target-delete-preflight-fold");
  assert_eq!(
    as_str(get(fold, "mode")),
    "macro-only-target-delete-preflight"
  );
  for layer in [
    "surface", "ontology", "semantic", "gate", "runtime", "audit",
  ] {
    assert!(as_bool(get_path(fold, &[layer, "visible"])));
  }
  assert_eq!(
    as_str(get_path(fold, &["surface", "owner-path"])),
    "stdlib/lib/gate/macro-only-boot-target-delete-preflight.px"
  );
  assert_eq!(as_i64(get_path(fold, &["surface", "host-target-count"])), 5);
  assert!(as_bool(get_path(
    fold,
    &["ontology", "target-delete-preflight-present"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["ontology", "target-specific-delete-proof-present"]
  )));
  assert_eq!(
    as_i64(get_path(fold, &["ontology", "ready-target-count"])),
    0
  );
  assert_eq!(
    as_i64(get_path(
      fold,
      &["gate", "runner-missing-after-preflight-count"]
    )),
    2
  );
  assert_eq!(
    as_i64(get_path(
      fold,
      &["gate", "future-delete-proof-without-puck-missing-count"]
    )),
    1
  );
  assert!(as_bool(get_path(
    fold,
    &["semantic", "preflight-is-obligation-map-not-delete-proof"]
  )));
  assert!(!as_bool(get_path(fold, &["runtime", "boot-executed"])));
  assert!(!as_bool(get_path(
    fold,
    &["runtime", "host-code-removal-started"]
  )));
}

#[test]
fn migration_delta_closes_preflight_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let delta = get(&run, "migration-delta");
  assert_eq!(
    as_str(get(delta, "id")),
    "migration-delta.macro-only-target-delete-preflight"
  );
  assert!(string_set(get(delta, "closes")).contains("need.host-removal.target-delete-preflight"));
  let not = string_set(get(delta, "does-not-close"));
  for expected in [
    "need.host-removal.target-specific-delete-proof",
    "need.bootstrap.fresh-p-puck-after-current-cut",
    "need.bootstrap.bounded-replay-execution-proof",
    "need.bootstrap.macro-only-boot-execution-proof",
    "need.bootstrap.macro-only-runtime-owner-boot",
    "need.bootstrap.new-engine-from-zero-proof",
  ] {
    assert!(not.contains(expected), "missing open frontier `{expected}`");
  }
  let next = string_set(get(delta, "next-required"));
  assert!(next.contains("fresh-p-puck-receipt-audit-after-current-cut"));
  assert!(next.contains("target-specific-host-delete-proof-after-successful-boot"));
  assert!(next.contains("caller-usage-scan-before-delete-proof"));
  assert!(next.contains("rollback-plan-before-delete-proof"));
}

#[test]
fn discoveries_record_d408_through_d416() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 9);
  for expected in [
    "D408.target-delete-preflight-owner-is-px-owner-not-delete-proof",
    "D409.all-five-old-host-targets-are-enumerated-before-delete-proof",
    "D410.host-removal-map-target-list-does-not-create-delete-ready-targets",
    "D411.delete-proof-obligations-stay-explicit-after-preflight",
    "D412.preflight-cannot-close-runner-target-specific-delete-proof",
    "D413.preflight-cannot-manufacture-puck-boot-runtime-or-new-engine",
    "D414.preflight-cannot-delete-host-code-or-add-gpl",
    "D415.runner-after-preflight-remains-held-with-two-missing-frontiers",
    "D416.next-frontier-remains-fresh-puck-and-target-delete-proof",
  ] {
    let discovery = discoveries
      .get(expected)
      .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
    assert!(as_bool(get(discovery, "scenario-only")));
  }
}

#[test]
fn inherited_status_links_to_compare_without_booting_or_deleting() {
  let run = eval_file(&fixture_path()).unwrap();
  let status = get(&run, "inherited-status");
  assert_eq!(
    as_str(get(status, "host-removal-map")),
    "tesseract-macro-ontology-host-code-removal-map"
  );
  assert!(as_bool(get(status, "host-removal-map-written")));
  assert_eq!(as_i64(get(status, "host-removal-map-target-count")), 5);
  assert_eq!(
    as_str(get(status, "compare-after-boot-proof")),
    "tesseract-macro-ontology-macro-only-compare-after-boot"
  );
  assert!(as_bool(get(status, "compare-after-boot")));
  assert_eq!(as_str(get(status, "runner-after-compare-status")), "Held");
  assert_eq!(as_i64(get(status, "runner-after-compare-missing-count")), 2);
  assert_eq!(as_str(get(status, "runner-after-preflight-status")), "Held");
  assert_eq!(
    as_i64(get(status, "runner-after-preflight-missing-count")),
    2
  );
  assert!(!as_bool(get(status, "previous-boot-executed")));
  assert!(!as_bool(get(status, "previous-new-engine-from-zero")));
  assert_eq!(as_i64(get(status, "previous-delete-ready-target-count")), 0);
}

#[test]
fn top_level_state_records_preflight_without_boot_or_delete() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "replacement-readiness")),
    "macro-only-target-delete-preflight-present"
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
  assert!(as_bool(get(&run, "target-delete-preflight-present")));
  for key in [
    "owner-switch",
    "target-specific-delete-proof-present",
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
fn negative_held_evidence_rejects_preflight_shortcuts() {
  let run = eval_file(&fixture_path()).unwrap();
  let rejects = string_set(get_path(&run, &["negative-held-evidence", "rejects"]));
  for expected in [
    "preflight-present-as-target-delete-proof",
    "preflight-present-as-host-removal-started",
    "preflight-present-as-delete-ready-targets",
    "preflight-present-as-p-puck-freshness",
    "preflight-present-as-replay-executed",
    "preflight-present-as-boot-executed",
    "preflight-present-as-runtime-owner",
    "preflight-present-as-new-engine-from-zero",
    "preflight-present-as-semantic-owner",
    "host-removal-map-target-list-as-delete-proof",
    "gpl-family-dependency-in-delete-preflight",
  ] {
    assert!(rejects.contains(expected), "missing reject `{expected}`");
  }
}
