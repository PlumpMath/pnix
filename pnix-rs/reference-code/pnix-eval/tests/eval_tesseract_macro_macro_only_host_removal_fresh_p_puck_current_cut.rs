//! Host-removal fresh p-puck current-cut receipt.
//!
//! This pins the actual p-puck pnixc report over the host-removal execution
//! proof receipt. The proof closes freshness for that cut, records a slow-path
//! candidate, and keeps host deletion / runtime install false.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../..")
}

fn fixture_path() -> PathBuf {
  repo_root().join(
    "fixtures/tesseract-macro-legacy-probe/macro_only_host_removal_fresh_p_puck_current_cut_receipt.px",
  )
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
  let run = eval_file(&fixture_path()).expect("host removal fresh p-puck receipt");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-macro-only-host-removal-fresh-p-puck-current-cut"
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
    "stdlib/lib/gate/macro-only-host-removal-fresh-p-puck-current-cut.px",
    "fixtures/pnix-query-runtime/macro-only-host-removal-fresh-p-puck-current-cut-owner.px",
    "fixtures/tesseract-macro-legacy-probe/macro_only_host_removal_fresh_p_puck_current_cut_receipt.px",
  ] {
    assert!(repo_root().join(path).is_file(), "missing `{path}`");
  }
}

#[test]
fn constitution_gate_blocks_fresh_puck_overclaims() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitutionGate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "macro-only-host-removal-fresh-p-puck-current-cut"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));

  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "fresh-host-removal-puck-equals-delete-ready",
    "fresh-host-removal-puck-equals-host-code-removal",
    "fresh-host-removal-puck-equals-implementation-command",
    "fresh-host-removal-puck-equals-global-runtime-install",
    "fresh-host-removal-puck-equals-semantic-owner",
    "slow-path-candidate-equals-ignore-telemetry",
    "old-host-code-authorizes-fresh-delete-cut",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
}

#[test]
fn contract_closes_freshness_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let contract = get(&run, "host-removal-fresh-puck-contract");
  assert_eq!(
    as_str(get(contract, "id")),
    "contract.macro-only-host-removal-fresh-p-puck-current-cut.v1"
  );
  assert_eq!(
    as_str(get(contract, "current-status")),
    "host-removal-fresh-p-puck-current-cut-present"
  );
  assert!(as_bool(get(
    contract,
    "closes-fresh-puck-before-host-removal-execution"
  )));
  assert_eq!(as_i64(get(contract, "duration-ms")), 5389);
  assert_eq!(
    as_str(get(contract, "slow-path-status")),
    "slow-path-candidate"
  );
  for key in [
    "closes-slow-path-profile",
    "closes-actual-host-removal-patch",
    "closes-delete-ready-targets",
    "closes-global-runtime",
    "closes-new-engine-from-zero",
  ] {
    assert!(!as_bool(get(contract, key)), "`{key}` must stay false");
  }
}

#[test]
fn proof_records_actual_slow_p_puck_telemetry() {
  let run = eval_file(&fixture_path()).unwrap();
  let proof = get(&run, "host-removal-fresh-puck-proof");
  assert_eq!(
    as_str(get(proof, "status")),
    "host-removal-fresh-p-puck-current-cut-present"
  );
  assert_eq!(
    as_str(get(proof, "report-name")),
    "macro-only-current-cut-host-removal-execution-proof"
  );
  assert_eq!(
    as_str(get(proof, "audited-receipt")),
    "fixtures/tesseract-macro-legacy-probe/macro_only_host_removal_execution_proof_receipt.px"
  );
  assert_eq!(as_i64(get(proof, "duration-ms")), 5389);
  assert_eq!(as_i64(get(proof, "slow-threshold-ms")), 5000);
  assert_eq!(
    as_str(get(proof, "slow-path-status")),
    "slow-path-candidate"
  );
  assert!(as_bool(get(proof, "host-removal-fresh-p-puck-current-cut")));
  assert!(as_bool(get(
    proof,
    "fresh-puck-before-host-removal-execution"
  )));
  assert!(as_bool(get(proof, "slow-path-candidate")));
  assert!(as_bool(get(proof, "self-optimization-candidate")));
  assert!(!as_bool(get(proof, "actual-host-removal-patch-authorized")));
  assert!(!as_bool(get(proof, "host-code-removal-started")));
  assert_eq!(as_i64(get(proof, "delete-ready-target-count")), 0);
}

#[test]
fn trials_cover_valid_report_and_held_overclaims() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "host-removal-fresh-puck-trials"));
  assert_eq!(trials.len(), 11);
  assert_eq!(
    as_str(get(
      trials["trial.A.valid-host-removal-fresh-puck-current-cut"],
      "outcome"
    )),
    "host-removal-fresh-p-puck-current-cut-present"
  );
  assert_eq!(
    as_str(get(
      trials["trial.B.host-removal-execution-input"],
      "outcome"
    )),
    "tesseract-macro-ontology-macro-only-host-removal-execution-proof"
  );
  for (id, held) in [
    (
      "trial.C.execution-proof-missing",
      "held.macro-only-host-removal-fresh-puck.execution-proof-missing",
    ),
    (
      "trial.D.report-mismatch",
      "held.macro-only-host-removal-fresh-puck.report-mismatch",
    ),
    (
      "trial.E.receipt-mismatch",
      "held.macro-only-host-removal-fresh-puck.current-cut-receipt-mismatch",
    ),
    (
      "trial.F.telemetry-drift",
      "held.macro-only-host-removal-fresh-puck.telemetry-missing-or-drifted",
    ),
    (
      "trial.G.delete-claim",
      "held.macro-only-host-removal-fresh-puck.delete-overclaim",
    ),
    (
      "trial.H.runtime-claim",
      "held.macro-only-host-removal-fresh-puck.runtime-overclaim",
    ),
    (
      "trial.I.semantic-owner-claim",
      "held.macro-only-host-removal-fresh-puck.semantic-owner-claim",
    ),
    (
      "trial.J.old-host-authority",
      "held.macro-only-host-removal-fresh-puck.old-host-authority",
    ),
    (
      "trial.K.gpl-family-dependency",
      "held.macro-only-host-removal-fresh-puck.gpl-family-dependency",
    ),
  ] {
    assert_eq!(as_str(get(trials[id], "outcome")), "Held");
    assert_eq!(as_str(get(trials[id], "held-id")), held);
  }
}

#[test]
fn six_layer_fold_records_freshness_without_delete_collapse() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-host-removal-fresh-puck-fold");
  for layer in [
    "surface", "ontology", "semantic", "gate", "runtime", "audit",
  ] {
    assert!(
      as_bool(get(get(fold, layer), "visible")),
      "layer `{layer}` hidden"
    );
  }
  assert!(as_bool(get_path(
    fold,
    &["ontology", "host-removal-execution-proof-input"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "slow-path-candidate"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "self-optimization-candidate"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["gate", "blocked-delete-overclaim"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["runtime", "actual-host-removal-patch-authorized"]
  )));
  assert_eq!(
    as_i64(get_path(fold, &["runtime", "delete-ready-target-count"])),
    0
  );
}

#[test]
fn migration_delta_closes_freshness_and_opens_slow_path_profile() {
  let run = eval_file(&fixture_path()).unwrap();
  let delta = get(&run, "migrationDelta");
  let closes = string_set(get(delta, "closes"));
  assert!(closes.contains("fresh-puck-before-host-removal-execution"));

  let not = string_set(get(delta, "does-not-close"));
  assert!(not.contains("need.host-removal.fresh-puck-before-delete-as-delete-ready"));
  assert!(not.contains("need.host-removal.slow-path-repeat-or-profile-before-delete"));
  assert!(not.contains("need.host-removal.actual-delete-patch-after-fresh-puck"));
  assert!(not.contains("need.runtime.global-ontology-install"));

  let next = string_set(get(delta, "next-required"));
  assert!(next.contains("host-removal-slow-path-repeat-or-profile-before-delete"));
  assert!(next.contains("actual-host-removal-patch-after-fresh-puck"));
}

#[test]
fn discoveries_record_d492_through_d499() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 8);
  for expected in [
    "D492.host-removal-fresh-puck-must-follow-execution-proof",
    "D493.host-removal-puck-current-cut-is-per-receipt",
    "D494.slow-path-telemetry-is-load-bearing",
    "D495.slow-path-candidate-opens-self-optimization-not-delete",
    "D496.fresh-puck-does-not-authorize-host-removal",
    "D497.p-puck-wrapper-remains-non-semantic-owner",
    "D498.old-host-authority-cannot-authorize-fresh-delete-cut",
    "D499.host-removal-fresh-puck-keeps-gpl-and-command-false",
  ] {
    let item = discoveries
      .get(expected)
      .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
    assert_eq!(as_str(get(item, "decision-pressure")), "keep");
    assert!(as_bool(get(item, "scenario-only")));
  }
}

#[test]
fn top_level_state_is_fresh_but_still_non_delete() {
  let run = eval_file(&fixture_path()).unwrap();
  assert!(as_bool(get(&run, "host-removal-fresh-p-puck-current-cut")));
  assert!(as_bool(get(
    &run,
    "fresh-puck-before-host-removal-execution"
  )));
  assert!(as_bool(get(&run, "p-puck-wrapper-proof")));
  assert!(as_bool(get(&run, "slow-path-candidate")));
  assert!(as_bool(get(&run, "self-optimization-candidate")));
  for key in [
    "p-puck-is-semantic-owner",
    "actual-host-removal-patch-authorized",
    "host-code-removal-started",
    "host-removal-safe",
    "fresh-puck-before-delete",
    "global-ontology-runtime",
    "runtime-install",
    "new-engine-from-zero",
    "old-host-authority",
    "gpl-family-dependencies",
    "implementation-command",
  ] {
    assert!(!as_bool(get(&run, key)), "`{key}` must stay false");
  }
  assert_eq!(as_i64(get(&run, "delete-ready-target-count")), 0);
}

#[test]
fn replacement_readiness_says_not_delete() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "replacement-readiness")),
    "host-removal-fresh-p-puck-current-cut-present-not-delete"
  );
}
