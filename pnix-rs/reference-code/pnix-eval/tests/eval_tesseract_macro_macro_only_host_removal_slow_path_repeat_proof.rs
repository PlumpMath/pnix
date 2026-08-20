//! Host-removal slow-path repeat proof.
//!
//! This pins the actual repeat p-puck report after the host-removal fresh
//! p-puck current-cut proof recorded a slow-path candidate. The repeat run is
//! within threshold, so the slow-path repeat/profile frontier closes without
//! authorizing deletion, runtime install, or semantic ownership.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../..")
}

fn fixture_path() -> PathBuf {
  repo_root().join(
    "fixtures/tesseract-macro-legacy-probe/macro_only_host_removal_slow_path_repeat_proof_receipt.px",
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
  let run = eval_file(&fixture_path()).expect("host removal slow-path repeat receipt");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-macro-only-host-removal-slow-path-repeat-proof"
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
    "stdlib/lib/gate/macro-only-host-removal-slow-path-repeat-proof.px",
    "fixtures/pnix-query-runtime/macro-only-host-removal-slow-path-repeat-proof-owner.px",
    "fixtures/tesseract-macro-legacy-probe/macro_only_host_removal_slow_path_repeat_proof_receipt.px",
  ] {
    assert!(repo_root().join(path).is_file(), "missing `{path}`");
  }
}

#[test]
fn constitution_gate_blocks_repeat_overclaims() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitutionGate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "macro-only-host-removal-slow-path-repeat-proof"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));

  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "slow-path-repeat-equals-delete-ready",
    "slow-path-repeat-equals-host-code-removal",
    "slow-path-repeat-equals-implementation-command",
    "slow-path-repeat-equals-global-runtime-install",
    "slow-path-repeat-equals-semantic-owner",
    "within-threshold-repeat-erases-future-telemetry-gates",
    "old-host-code-authorizes-slow-path-clearance",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
}

#[test]
fn contract_closes_slow_path_repeat_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let contract = get(&run, "host-removal-slow-path-repeat-contract");
  assert_eq!(
    as_str(get(contract, "id")),
    "contract.macro-only-host-removal-slow-path-repeat-proof.v1"
  );
  assert_eq!(
    as_str(get(contract, "current-status")),
    "host-removal-slow-path-repeat-within-threshold"
  );
  assert_eq!(as_i64(get(contract, "prior-gate-duration-ms")), 5389);
  assert_eq!(as_i64(get(contract, "repeat-duration-ms")), 551);
  assert_eq!(as_i64(get(contract, "puck-previous-duration-ms")), 5094);
  assert_eq!(as_i64(get(contract, "duration-delta-ms")), -4543);
  assert_eq!(
    as_str(get(contract, "repeat-slow-path-status")),
    "within-threshold"
  );
  assert!(as_bool(get(contract, "closes-slow-path-repeat-or-profile")));
  for key in [
    "closes-actual-host-removal-patch",
    "closes-delete-ready-targets",
    "closes-global-runtime",
    "closes-new-engine-from-zero",
  ] {
    assert!(!as_bool(get(contract, key)), "`{key}` must stay false");
  }
}

#[test]
fn proof_records_actual_repeat_p_puck_telemetry() {
  let run = eval_file(&fixture_path()).unwrap();
  let proof = get(&run, "host-removal-slow-path-repeat-proof");
  assert_eq!(
    as_str(get(proof, "status")),
    "host-removal-slow-path-repeat-within-threshold"
  );
  assert_eq!(
    as_str(get(proof, "report-name")),
    "macro-only-current-cut-host-removal-execution-proof-repeat"
  );
  assert_eq!(
    as_str(get(proof, "audited-receipt")),
    "fixtures/tesseract-macro-legacy-probe/macro_only_host_removal_execution_proof_receipt.px"
  );
  assert_eq!(as_i64(get(proof, "prior-gate-duration-ms")), 5389);
  assert_eq!(as_i64(get(proof, "repeat-duration-ms")), 551);
  assert_eq!(as_i64(get(proof, "puck-previous-duration-ms")), 5094);
  assert_eq!(as_i64(get(proof, "duration-delta-ms")), -4543);
  assert_eq!(as_i64(get(proof, "slow-threshold-ms")), 5000);
  assert_eq!(
    as_str(get(proof, "repeat-slow-path-status")),
    "within-threshold"
  );
  assert_eq!(
    as_str(get(proof, "duration-delta-status")),
    "faster-than-previous"
  );
  assert!(as_bool(get(proof, "slow-path-repeat-frontier-closed")));
  assert!(!as_bool(get(proof, "persistent-slow-path")));
  assert!(!as_bool(get(proof, "actual-host-removal-patch-authorized")));
  assert_eq!(as_i64(get(proof, "delete-ready-target-count")), 0);
}

#[test]
fn trials_cover_valid_repeat_and_held_overclaims() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "host-removal-slow-path-repeat-trials"));
  assert_eq!(trials.len(), 13);
  assert_eq!(
    as_str(get(
      trials["trial.A.valid-host-removal-slow-path-repeat"],
      "outcome"
    )),
    "host-removal-slow-path-repeat-within-threshold"
  );
  assert_eq!(
    as_str(get(
      trials["trial.B.host-removal-fresh-puck-input"],
      "outcome"
    )),
    "tesseract-macro-ontology-macro-only-host-removal-fresh-p-puck-current-cut"
  );
  for (id, held) in [
    (
      "trial.C.prior-slow-path-missing",
      "held.macro-only-host-removal-slow-path-repeat.prior-slow-path-missing",
    ),
    (
      "trial.D.report-mismatch",
      "held.macro-only-host-removal-slow-path-repeat.report-mismatch",
    ),
    (
      "trial.E.receipt-mismatch",
      "held.macro-only-host-removal-slow-path-repeat.current-cut-receipt-mismatch",
    ),
    (
      "trial.F.telemetry-number-drift",
      "held.macro-only-host-removal-slow-path-repeat.telemetry-number-drift",
    ),
    (
      "trial.G.telemetry-status-drift",
      "held.macro-only-host-removal-slow-path-repeat.telemetry-status-drift",
    ),
    (
      "trial.H.profile-overclaim",
      "held.macro-only-host-removal-slow-path-repeat.profile-overclaim",
    ),
    (
      "trial.I.delete-claim",
      "held.macro-only-host-removal-slow-path-repeat.delete-overclaim",
    ),
    (
      "trial.J.runtime-claim",
      "held.macro-only-host-removal-slow-path-repeat.runtime-overclaim",
    ),
    (
      "trial.K.semantic-owner-claim",
      "held.macro-only-host-removal-slow-path-repeat.semantic-owner-claim",
    ),
    (
      "trial.L.old-host-authority",
      "held.macro-only-host-removal-slow-path-repeat.old-host-authority",
    ),
    (
      "trial.M.gpl-family-dependency",
      "held.macro-only-host-removal-slow-path-repeat.gpl-family-dependency",
    ),
  ] {
    assert_eq!(as_str(get(trials[id], "outcome")), "Held");
    assert_eq!(as_str(get(trials[id], "held-id")), held);
  }
}

#[test]
fn six_layer_fold_records_repeat_clearance_without_delete_collapse() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-host-removal-slow-path-repeat-fold");
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
    &["ontology", "prior-slow-path-candidate"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "repeat-within-threshold"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["semantic", "persistent-slow-path"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["gate", "slow-path-repeat-frontier-closed"]
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
fn migration_delta_closes_slow_path_repeat_and_keeps_flattening_deferred() {
  let run = eval_file(&fixture_path()).unwrap();
  let delta = get(&run, "migrationDelta");
  let closes = string_set(get(delta, "closes"));
  assert!(closes.contains("need.host-removal.slow-path-repeat-or-profile-before-delete"));

  let not = string_set(get(delta, "does-not-close"));
  assert!(not.contains("need.host-removal.fresh-puck-before-delete-as-delete-ready"));
  assert!(not.contains("need.host-removal.actual-delete-patch-after-fresh-puck"));
  assert!(not.contains("need.runtime.global-ontology-install"));

  let next = string_set(get(delta, "next-required"));
  assert!(!next.contains("host-removal-slow-path-repeat-or-profile-before-delete"));
  assert!(next.contains("actual-host-removal-patch-after-fresh-puck"));
  assert!(next.contains("domain-runtime-api-flattening-after-semantic-owner"));
  assert_eq!(next.len(), 4);
}

#[test]
fn discoveries_record_d500_through_d507() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 8);
  for expected in [
    "D500.slow-path-repeat-is-separate-after-fresh-puck",
    "D501.repeat-run-within-threshold-clears-persistent-slow-path",
    "D502.prior-gate-duration-and-puck-previous-duration-are-distinct",
    "D503.faster-than-previous-delta-is-evidence-not-authority",
    "D504.no-slow-steps-blocks-profile-patch-theater",
    "D505.slow-path-closure-does-not-authorize-host-removal",
    "D506.repeat-puck-wrapper-remains-non-semantic-owner",
    "D507.host-removal-frontier-shrinks-with-flattening-deferred",
  ] {
    let item = discoveries
      .get(expected)
      .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
    assert_eq!(as_str(get(item, "decision-pressure")), "keep");
    assert!(as_bool(get(item, "scenario-only")));
  }
}

#[test]
fn top_level_state_is_repeat_closed_but_still_non_delete() {
  let run = eval_file(&fixture_path()).unwrap();
  assert!(as_bool(get(
    &run,
    "host-removal-slow-path-repeat-proof-present"
  )));
  assert!(as_bool(get(&run, "slow-path-repeat-within-threshold")));
  assert!(as_bool(get(&run, "slow-path-repeat-frontier-closed")));
  assert!(!as_bool(get(&run, "persistent-slow-path")));
  assert!(!as_bool(get(&run, "profile-required-from-repeat")));
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
    "host-removal-slow-path-repeat-closed-not-delete"
  );
}
