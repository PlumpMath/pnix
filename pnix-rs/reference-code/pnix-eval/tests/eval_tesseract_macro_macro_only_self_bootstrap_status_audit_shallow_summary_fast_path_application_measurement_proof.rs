//! Macro-only self bootstrap status audit shallow summary fast-path application
//! measurement proof.
//!
//! This receipt consumes the selected-callsite fast-path application proof and
//! records real p-puck timing samples for the applied status query. It separates
//! cold-start wrapper cost from warm selected-callsite behavior and opens a
//! callsite widening policy frontier instead of widening globally.

use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn repo_root() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../..")
}

fn fixture_path() -> PathBuf {
  repo_root().join(
    "fixtures/tesseract-macro-legacy-probe/macro_only_self_bootstrap_status_audit_shallow_summary_fast_path_application_measurement_proof_receipt.px",
  )
}

fn eval_receipt() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = fixture_path();
    let json = std::thread::Builder::new()
      .name("eval-bootstrap-shallow-summary-fast-path-application-measurement-receipt".to_string())
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("self bootstrap status fast-path application measurement receipt")
      })
      .expect("spawn evaluator thread")
      .join()
      .expect("evaluator thread");
    serde_json::from_str(&json).expect("receipt JSON")
  })
}

fn as_attrs(v: &Value) -> &Map<String, Value> {
  v.as_object()
    .unwrap_or_else(|| panic!("expected object, got {v:?}"))
}

fn as_list(v: &Value) -> &Vec<Value> {
  v.as_array()
    .unwrap_or_else(|| panic!("expected array, got {v:?}"))
}

fn as_str(v: &Value) -> &str {
  v.as_str()
    .unwrap_or_else(|| panic!("expected string, got {v:?}"))
}

fn as_bool(v: &Value) -> bool {
  v.as_bool()
    .unwrap_or_else(|| panic!("expected bool, got {v:?}"))
}

fn as_i64(v: &Value) -> i64 {
  v.as_i64()
    .unwrap_or_else(|| panic!("expected integer, got {v:?}"))
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
  let run = eval_receipt();
  assert_eq!(
    as_str(get(run, "probe-marker")),
    "tesseract-macro-ontology-macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application-measurement-proof"
  );
  assert_eq!(
    as_str(get(run, "truth-owner")),
    "project-wiki/maps/tesseract-macro-ontology-discovery-ledger.md"
  );
  assert_eq!(
    as_str(get(run, "constitution-owner")),
    "stdlib/lib/gate/tesseract-constitution.px"
  );
  for path in [
    "stdlib/lib/gate/macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application-measurement-proof.px",
    "fixtures/pnix-query-runtime/macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application-measurement-proof-owner.px",
    "fixtures/tesseract-macro-legacy-probe/macro_only_self_bootstrap_status_audit_shallow_summary_fast_path_application_measurement_proof_receipt.px",
  ] {
    assert!(repo_root().join(path).is_file(), "missing `{path}`");
  }
}

#[test]
fn constitution_gate_blocks_measurement_overclaims() {
  let run = eval_receipt();
  let gate = get(run, "constitutionGate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application-measurement-proof"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "warm-repeat-within-threshold-equals-global-speedup",
    "warm-repeat-within-threshold-erases-cold-start-cost",
    "measurement-equals-callsite-widening-approval",
    "measurement-equals-global-runtime",
    "measurement-equals-runtime-api-flattening",
    "measurement-equals-meaning-db",
    "measurement-equals-external-solver-intake",
    "measurement-equals-p-puck-semantic-owner",
    "measurement-equals-self-modification",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
}

#[test]
fn contract_records_cold_and_warm_measurement_envelope() {
  let run = eval_receipt();
  let contract = get(run, "measurement-contract");
  assert_eq!(
    as_str(get(contract, "id")),
    "contract.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application-measurement-proof.v1"
  );
  assert_eq!(
    as_str(get(contract, "current-status")),
    "self-bootstrap-status-audit-shallow-summary-fast-path-application-measurement-proof-present"
  );
  assert!(as_bool(get(contract, "closes-measurement-frontier")));
  assert!(as_bool(get(contract, "opens-callsite-widening-policy")));
  assert_eq!(as_i64(get(contract, "cold-start-duration-ms")), 10982);
  assert_eq!(as_i64(get(contract, "warm-repeat-one-duration-ms")), 358);
  assert_eq!(as_i64(get(contract, "warm-repeat-two-duration-ms")), 275);
  assert_eq!(as_i64(get(contract, "warm-repeat-min-duration-ms")), 275);
  assert_eq!(as_i64(get(contract, "warm-repeat-max-duration-ms")), 358);
  assert_eq!(as_i64(get(contract, "cold-to-warm-max-delta-ms")), -10624);
  assert!(as_bool(get(contract, "cold-start-slow-path-candidate")));
  assert!(as_bool(get(contract, "warm-repeats-within-threshold")));
  assert!(!as_bool(get(contract, "persistent-warm-slow-path")));
  assert!(!as_bool(get(contract, "global-speedup-claimed")));
  assert!(!as_bool(get(contract, "callsite-widening-approved")));
}

#[test]
fn measurement_proof_closes_only_measurement_frontier() {
  let run = eval_receipt();
  let proof = get(run, "measurement-proof");
  assert_eq!(
    as_str(get(proof, "status")),
    "self-bootstrap-status-audit-shallow-summary-fast-path-application-measurement-proof-present"
  );
  assert!(as_bool(get(
    proof,
    "self-bootstrap-status-audit-shallow-summary-fast-path-application-measurement-proof"
  )));
  assert!(as_bool(get(proof, "selected-callsite-measured")));
  assert_eq!(as_i64(get(proof, "measurement-record-count")), 3);
  assert!(as_bool(get(proof, "warm-repeats-within-threshold")));
  assert!(!as_bool(get(proof, "global-speedup-claimed")));
  assert!(!as_bool(get(proof, "callsite-widening-approved")));

  let closed = string_set(get(proof, "closes"));
  assert!(closed.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-application-measurement-proof"
  ));
  let open = string_set(get(proof, "next-open-frontiers"));
  assert!(open.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-policy-proof"
  ));
}

#[test]
fn trials_cover_measurement_values_and_held_overclaims() {
  let run = eval_receipt();
  let trials = attrs_by_id(get(run, "measurement-trials"));
  assert_eq!(trials.len(), 18);
  assert_eq!(
    as_str(get(trials["trial.A.valid-measurement-proof"], "outcome")),
    "self-bootstrap-status-audit-shallow-summary-fast-path-application-measurement-proof-present"
  );
  assert_eq!(
    as_i64(get(trials["trial.B.cold-start-sample"], "duration-ms")),
    10982
  );
  assert_eq!(
    as_i64(get(trials["trial.C.warm-repeat-one"], "duration-ms")),
    358
  );
  assert_eq!(
    as_i64(get(trials["trial.D.warm-repeat-two"], "duration-ms")),
    275
  );

  for (trial, held_id) in [
    (
      "trial.F.wrong-proof-id",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application-measurement.proof-id-mismatch",
    ),
    (
      "trial.J.record-shape-invalid",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application-measurement.record-shape-invalid",
    ),
    (
      "trial.K.sample-values-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application-measurement.sample-values-mismatch",
    ),
    (
      "trial.O.measurement-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application-measurement.measurement-overclaim",
    ),
    (
      "trial.P.runtime-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application-measurement.runtime-overclaim",
    ),
    (
      "trial.Q.external-or-license-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application-measurement.external-or-license-overclaim",
    ),
    (
      "trial.R.authority-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application-measurement.authority-overclaim",
    ),
  ] {
    assert_eq!(as_str(get(trials[trial], "held-id")), held_id);
  }
}

#[test]
fn six_layer_fold_separates_measurement_from_runtime_install() {
  let run = eval_receipt();
  assert!(as_bool(get_path(
    run,
    &["six-layer-measurement-fold", "surface", "visible"]
  )));
  assert_eq!(
    as_i64(get_path(
      run,
      &[
        "six-layer-measurement-fold",
        "ontology",
        "measurement-record-count"
      ]
    )),
    3
  );
  assert_eq!(
    as_str(get_path(
      run,
      &[
        "six-layer-measurement-fold",
        "semantic",
        "performance-envelope"
      ]
    )),
    "cold-start-slow-warm-repeats-within-threshold"
  );
  assert!(as_bool(get_path(
    run,
    &[
      "six-layer-measurement-fold",
      "gate",
      "blocked-measurement-overclaim"
    ]
  )));
  assert!(!as_bool(get_path(
    run,
    &["six-layer-measurement-fold", "runtime", "runtime-install"]
  )));
  assert!(!as_bool(get_path(
    run,
    &["six-layer-measurement-fold", "runtime", "meaning-db"]
  )));
}

#[test]
fn discoveries_record_d707_through_d714() {
  let run = eval_receipt();
  let discoveries = attrs_by_id(get(run, "discoveries"));
  assert_eq!(discoveries.len(), 8);
  for expected in [
    "D707.application-measurement-records-cold-and-warm-samples",
    "D708.warm-selected-callsite-repeats-are-within-threshold",
    "D709.cold-start-cost-remains-separate-from-fast-path",
    "D710.measurement-does-not-prove-global-speedup",
    "D711.measurement-keeps-fallback-and-rollback",
    "D712.callsite-widening-requires-separate-policy",
    "D713.measurement-preserves-external-and-authority-boundaries",
    "D714.next-frontier-is-callsite-widening-policy-proof",
  ] {
    let d = discoveries
      .get(expected)
      .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
    assert!(as_bool(get(d, "scenario-only")));
    assert_eq!(as_str(get(d, "decision-pressure")), "keep");
  }
}

#[test]
fn final_flags_keep_receipt_candidate_only() {
  let run = eval_receipt();
  assert!(as_bool(get(
    run,
    "self-bootstrap-status-audit-shallow-summary-fast-path-application-measurement-proof"
  )));
  assert!(as_bool(get(run, "selected-callsite-measured")));
  assert!(as_bool(get(run, "warm-repeats-within-threshold")));
  assert!(as_bool(get(run, "cold-start-slow-path-candidate")));
  assert!(!as_bool(get(run, "global-speedup-claimed")));
  assert!(!as_bool(get(run, "cold-start-solved")));
  assert!(!as_bool(get(run, "callsite-widening-approved")));
  assert_eq!(as_str(get(run, "replacement-readiness")), "not-proven");
  for key in [
    "external-solver-installed",
    "runtime-install",
    "global-ontology-runtime",
    "runtime-api-flattening",
    "meaning-db",
    "self-modification",
    "llm-authority",
    "p-puck-is-semantic-owner",
    "old-host-authority",
    "gpl-family-dependencies",
    "implementation-command",
    "owner-switch",
  ] {
    assert!(!as_bool(get(run, key)), "`{key}` must stay false");
  }
}
