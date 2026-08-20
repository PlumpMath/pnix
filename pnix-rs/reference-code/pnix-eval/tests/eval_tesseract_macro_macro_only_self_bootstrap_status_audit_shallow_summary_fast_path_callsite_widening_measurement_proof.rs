//! Macro-only self bootstrap status audit shallow summary fast-path callsite
//! widening measurement proof.
//!
//! This receipt consumes the callsite widening application proof, records
//! p-puck samples for the two newly widened callsites, inherits the selected
//! baseline measurement, and keeps global default replacement/readiness as a
//! separate proof.

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
    "fixtures/tesseract-macro-legacy-probe/macro_only_self_bootstrap_status_audit_shallow_summary_fast_path_callsite_widening_measurement_proof_receipt.px",
  )
}

fn eval_receipt() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = fixture_path();
    let json = std::thread::Builder::new()
      .name(
        "eval-bootstrap-shallow-summary-fast-path-callsite-widening-measurement-receipt"
          .to_string(),
      )
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("self bootstrap status fast-path callsite widening measurement receipt")
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
    "tesseract-macro-ontology-macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement-proof"
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
    "stdlib/lib/gate/macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement-proof.px",
    "fixtures/pnix-query-runtime/macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement-proof-owner.px",
    "fixtures/tesseract-macro-legacy-probe/macro_only_self_bootstrap_status_audit_shallow_summary_fast_path_callsite_widening_measurement_proof_receipt.px",
  ] {
    assert!(repo_root().join(path).is_file(), "missing `{path}`");
  }
}

#[test]
fn constitution_gate_blocks_widened_measurement_overclaims() {
  let run = eval_receipt();
  let gate = get(run, "constitutionGate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement-proof"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "widened-warm-repeat-within-threshold-equals-global-default-replacement",
    "widened-warm-repeat-within-threshold-equals-global-speedup",
    "widened-measurement-erases-cold-start-cost",
    "widened-measurement-equals-global-runtime",
    "widened-measurement-equals-runtime-api-flattening",
    "widened-measurement-equals-meaning-db",
    "widened-measurement-equals-external-solver-intake",
    "widened-measurement-equals-p-puck-semantic-owner",
    "widened-measurement-equals-self-modification",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
}

#[test]
fn contract_records_widened_measurement_envelope() {
  let run = eval_receipt();
  let contract = get(run, "measurement-contract");
  assert_eq!(
    as_str(get(contract, "id")),
    "contract.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement-proof.v1"
  );
  assert_eq!(
    as_str(get(contract, "current-status")),
    "self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement-proof-present"
  );
  assert!(as_bool(get(contract, "closes-measurement-frontier")));
  assert!(as_bool(get(
    contract,
    "opens-global-default-readiness-proof"
  )));
  assert!(as_bool(get(
    contract,
    "selected-baseline-measurement-inherited"
  )));
  assert_eq!(
    as_i64(get(contract, "selected-baseline-measurement-record-count")),
    3
  );
  assert_eq!(as_i64(get(contract, "measured-callsite-count")), 3);
  assert_eq!(as_i64(get(contract, "new-measurement-record-count")), 5);
  assert_eq!(
    as_i64(get(contract, "combined-measurement-record-count")),
    8
  );
  assert_eq!(
    as_i64(get(contract, "widened-cold-start-duration-ms")),
    10426
  );
  assert_eq!(
    as_i64(get(contract, "operator-panel-warm-one-duration-ms")),
    809
  );
  assert_eq!(
    as_i64(get(contract, "operator-panel-warm-two-duration-ms")),
    763
  );
  assert_eq!(
    as_i64(get(contract, "index-status-warm-one-duration-ms")),
    786
  );
  assert_eq!(
    as_i64(get(contract, "index-status-warm-two-duration-ms")),
    827
  );
  assert_eq!(
    as_i64(get(contract, "new-callsite-warm-min-duration-ms")),
    763
  );
  assert_eq!(
    as_i64(get(contract, "new-callsite-warm-max-duration-ms")),
    827
  );
  assert_eq!(
    as_i64(get(contract, "cold-to-new-warm-max-delta-ms")),
    -9599
  );
  assert!(as_bool(get(
    contract,
    "widened-cold-start-slow-path-candidate"
  )));
  assert!(as_bool(get(
    contract,
    "new-callsite-warm-repeats-within-threshold"
  )));
  assert!(!as_bool(get(contract, "global-default-readiness-proven")));
  assert!(!as_bool(get(contract, "global-default-callsite-replaced")));
  assert!(!as_bool(get(contract, "global-speedup-claimed")));
}

#[test]
fn measurement_proof_closes_only_widening_measurement_frontier() {
  let run = eval_receipt();
  let proof = get(run, "measurement-proof");
  assert_eq!(
    as_str(get(proof, "status")),
    "self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement-proof-present"
  );
  assert!(as_bool(get(
    proof,
    "self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement-proof"
  )));
  assert!(as_bool(get(proof, "widened-callsite-measured")));
  assert_eq!(as_i64(get(proof, "measured-callsite-count")), 3);
  assert_eq!(as_i64(get(proof, "new-measurement-record-count")), 5);
  assert_eq!(as_i64(get(proof, "combined-measurement-record-count")), 8);
  assert!(as_bool(get(
    proof,
    "new-callsite-warm-repeats-within-threshold"
  )));
  assert!(as_bool(get(
    proof,
    "global-default-readiness-proof-required"
  )));
  assert!(!as_bool(get(proof, "global-default-readiness-proven")));
  assert!(!as_bool(get(proof, "global-default-callsite-replaced")));

  let closed = string_set(get(proof, "closes"));
  assert!(closed.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement-proof"
  ));
  let open = string_set(get(proof, "next-open-frontiers"));
  assert!(open.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-readiness-proof"
  ));
}

#[test]
fn trials_cover_measurement_values_and_held_overclaims() {
  let run = eval_receipt();
  let trials = attrs_by_id(get(run, "measurement-trials"));
  assert_eq!(trials.len(), 23);
  assert_eq!(
    as_str(get(trials["trial.A.valid-widening-measurement-proof"], "outcome")),
    "self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement-proof-present"
  );
  assert_eq!(
    as_i64(get(trials["trial.C.operator-cold-start"], "duration-ms")),
    10426
  );
  assert_eq!(
    as_i64(get(trials["trial.D.operator-warm-one"], "duration-ms")),
    809
  );
  assert_eq!(
    as_i64(get(trials["trial.E.operator-warm-two"], "duration-ms")),
    763
  );
  assert_eq!(
    as_i64(get(trials["trial.F.index-warm-one"], "duration-ms")),
    786
  );
  assert_eq!(
    as_i64(get(trials["trial.G.index-warm-two"], "duration-ms")),
    827
  );

  for (trial, held_id) in [
    (
      "trial.I.wrong-proof-id",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement.proof-id-mismatch",
    ),
    (
      "trial.N.record-shape-invalid",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement.record-shape-invalid",
    ),
    (
      "trial.O.sample-values-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement.sample-values-mismatch",
    ),
    (
      "trial.P.coverage-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement.coverage-mismatch",
    ),
    (
      "trial.T.measurement-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement.measurement-overclaim",
    ),
    (
      "trial.U.runtime-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement.runtime-overclaim",
    ),
    (
      "trial.V.external-or-license-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement.external-or-license-overclaim",
    ),
    (
      "trial.W.authority-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement.authority-overclaim",
    ),
  ] {
    assert_eq!(as_str(get(trials[trial], "held-id")), held_id);
  }
}

#[test]
fn six_layer_fold_separates_measurement_from_global_default() {
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
        "measured-callsite-count"
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
    "widened-cold-start-slow-new-callsite-warm-repeats-within-threshold"
  );
  assert!(!as_bool(get_path(
    run,
    &[
      "six-layer-measurement-fold",
      "semantic",
      "global-default-callsite-replaced"
    ]
  )));
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
fn discoveries_record_d731_through_d738() {
  let run = eval_receipt();
  let discoveries = attrs_by_id(get(run, "discoveries"));
  assert_eq!(discoveries.len(), 8);
  for expected in [
    "D731.widening-measurement-records-two-new-callsite-samples",
    "D732.selected-baseline-measurement-is-inherited-not-erased",
    "D733.widened-new-callsite-warm-repeats-are-within-threshold",
    "D734.widened-measurement-coverage-is-exactly-three-callsites",
    "D735.widened-measurement-does-not-prove-global-default-replacement",
    "D736.widened-measurement-keeps-fallback-and-rollback",
    "D737.widened-measurement-preserves-external-and-authority-boundaries",
    "D738.next-frontier-is-global-default-replacement-readiness-proof",
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
    "self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement-proof"
  )));
  assert!(as_bool(get(run, "widened-callsite-measured")));
  assert!(as_bool(get(run, "selected-baseline-measurement-inherited")));
  assert_eq!(as_i64(get(run, "new-callsite-warm-max-duration-ms")), 827);
  assert!(as_bool(get(run, "global-default-readiness-proof-required")));
  assert!(!as_bool(get(run, "global-default-readiness-proven")));
  assert!(!as_bool(get(run, "global-default-callsite-replaced")));
  assert!(!as_bool(get(run, "global-speedup-claimed")));
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
