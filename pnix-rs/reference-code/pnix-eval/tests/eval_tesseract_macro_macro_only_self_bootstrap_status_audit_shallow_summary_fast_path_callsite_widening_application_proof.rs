//! Macro-only self bootstrap status audit shallow summary fast-path callsite
//! widening application proof.
//!
//! This receipt consumes the callsite widening policy proof and applies the
//! bounded shallow-summary fast path to two new callsites only. It keeps the
//! selected current-status callsite as baseline and opens measurement instead
//! of claiming global default replacement or global speedup.

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
    "fixtures/tesseract-macro-legacy-probe/macro_only_self_bootstrap_status_audit_shallow_summary_fast_path_callsite_widening_application_proof_receipt.px",
  )
}

fn eval_receipt() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = fixture_path();
    let json = std::thread::Builder::new()
      .name(
        "eval-bootstrap-shallow-summary-fast-path-callsite-widening-application-receipt"
          .to_string(),
      )
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("self bootstrap status fast-path callsite widening application receipt")
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

fn attrs_by_callsite<'a>(items: &'a Value) -> BTreeMap<&'a str, &'a Value> {
  as_list(items)
    .iter()
    .map(|item| (as_str(get(item, "callsite-id")), item))
    .collect()
}

#[test]
fn marker_and_owner_surfaces_are_pinned() {
  let run = eval_receipt();
  assert_eq!(
    as_str(get(run, "probe-marker")),
    "tesseract-macro-ontology-macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-application-proof"
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
    "stdlib/lib/gate/macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-application-proof.px",
    "fixtures/pnix-query-runtime/macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-application-proof-owner.px",
    "fixtures/tesseract-macro-legacy-probe/macro_only_self_bootstrap_status_audit_shallow_summary_fast_path_callsite_widening_application_proof_receipt.px",
  ] {
    assert!(repo_root().join(path).is_file(), "missing `{path}`");
  }
}

#[test]
fn constitution_gate_blocks_application_overclaims() {
  let run = eval_receipt();
  let gate = get(run, "constitutionGate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-application-proof"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "policy-approval-equals-application",
    "selected-callsite-reapply-equals-new-application",
    "two-callsite-application-equals-global-default-replacement",
    "application-equals-global-speedup",
    "application-equals-global-runtime",
    "application-equals-runtime-api-flattening",
    "application-equals-meaning-db",
    "application-equals-external-solver-intake",
    "application-equals-self-modification",
    "application-equals-llm-authority",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
}

#[test]
fn contract_records_two_new_callsite_application_without_global_claims() {
  let run = eval_receipt();
  let contract = get(run, "application-contract");
  assert_eq!(
    as_str(get(contract, "id")),
    "contract.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-application-proof.v1"
  );
  assert_eq!(
    as_str(get(contract, "current-status")),
    "self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-application-proof-present"
  );
  assert!(as_bool(get(contract, "closes-application-frontier")));
  assert!(as_bool(get(contract, "opens-measurement-frontier")));
  assert_eq!(as_i64(get(contract, "applied-new-callsite-count")), 2);
  assert_eq!(as_i64(get(contract, "total-applied-callsite-count")), 3);
  assert!(as_bool(get(contract, "selected-callsite-remains-applied")));
  assert!(as_bool(get(contract, "operator-panel-callsite-applied")));
  assert!(as_bool(get(contract, "index-status-callsite-applied")));
  assert!(as_bool(get(contract, "callsite-widening-policy-approved")));
  assert!(as_bool(get(contract, "callsite-widening-applied")));
  assert!(as_bool(get(contract, "additional-callsites-applied")));
  assert!(as_bool(get(contract, "measurement-required")));
  assert!(!as_bool(get(contract, "global-default-callsite-replaced")));
  assert!(!as_bool(get(contract, "global-speedup-claimed")));
}

#[test]
fn application_proof_closes_application_and_opens_measurement() {
  let run = eval_receipt();
  let proof = get(run, "application-proof");
  assert_eq!(
    as_str(get(proof, "status")),
    "self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-application-proof-present"
  );
  assert!(as_bool(get(
    proof,
    "self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-application-proof"
  )));
  assert!(as_bool(get(proof, "callsite-widening-policy-approved")));
  assert!(as_bool(get(proof, "callsite-widening-applied")));
  assert!(as_bool(get(proof, "additional-callsites-applied")));
  assert_eq!(as_i64(get(proof, "applied-new-callsite-count")), 2);
  assert_eq!(as_i64(get(proof, "total-applied-callsite-count")), 3);

  let closed = string_set(get(proof, "closes"));
  assert!(closed.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-application-proof"
  ));
  let open = string_set(get(proof, "next-open-frontiers"));
  assert!(open.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement-proof"
  ));
}

#[test]
fn applied_results_cover_operator_panel_and_index_status_only() {
  let run = eval_receipt();
  let results = attrs_by_callsite(get(run, "applied-new-callsite-results"));
  assert_eq!(results.len(), 2);
  for id in [
    "callsite.bootstrap-status-audit.operator-panel.shallow-summary.v1",
    "callsite.bootstrap-status-audit.index-status.shallow-summary.v1",
  ] {
    let result = results[id];
    assert_eq!(
      as_str(get(result, "status")),
      "widened-callsite-fast-path-applied-shallow-summary-read"
    );
    assert!(as_bool(get(result, "callsite-widening-applied")));
    assert!(as_bool(get(result, "additional-callsite-applied")));
    assert!(as_bool(get(result, "selected-callsite-remains-applied")));
    assert!(as_bool(get(result, "measurement-required")));
    assert!(as_bool(get(result, "full-audit-fallback-preserved")));
    assert_eq!(as_i64(get(result, "status-field-count")), 11);
    assert!(!as_bool(get(result, "global-default-callsite-replaced")));
    assert!(!as_bool(get(result, "global-speedup-claimed")));
  }
}

#[test]
fn trials_cover_application_and_negative_held_cases() {
  let run = eval_receipt();
  let trials = attrs_by_id(get(run, "application-trials"));
  assert_eq!(trials.len(), 21);
  assert_eq!(
    as_str(get(trials["trial.A.valid-application-proof"], "outcome")),
    "self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-application-proof-present"
  );
  assert_eq!(
    as_str(get(trials["trial.B.operator-panel-applied"], "outcome")),
    "widened-callsite-fast-path-applied-shallow-summary-read"
  );
  assert_eq!(
    as_str(get(trials["trial.C.index-status-applied"], "callsite-id")),
    "callsite.bootstrap-status-audit.index-status.shallow-summary.v1"
  );

  for (trial, held_id) in [
    (
      "trial.D.selected-reapply-held",
      "held.bootstrap-status-shallow-summary-callsite-widening-application.not-new-policy-target",
    ),
    (
      "trial.E.unlisted-callsite-held",
      "held.bootstrap-status-shallow-summary-callsite-widening-application.not-new-policy-target",
    ),
    (
      "trial.F.policy-approval-missing-held",
      "held.bootstrap-status-shallow-summary-callsite-widening-application.policy-approval-missing",
    ),
    (
      "trial.G.field-shape-held",
      "held.bootstrap-status-shallow-summary-callsite-widening-application.field-shape-mismatch",
    ),
    (
      "trial.H.fallback-missing-held",
      "held.bootstrap-status-shallow-summary-callsite-widening-application.fallback-missing",
    ),
    (
      "trial.I.route-result-held",
      "held.bootstrap-status-shallow-summary-callsite-widening-application.route-result-held",
    ),
    (
      "trial.R.global-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-application.global-overclaim",
    ),
  ] {
    assert_eq!(as_str(get(trials[trial], "outcome")), "Held", "`{trial}` status");
    assert_eq!(as_str(get(trials[trial], "held-id")), held_id, "`{trial}` held id");
  }
}

#[test]
fn six_layer_fold_preserves_application_runtime_and_audit_boundaries() {
  let run = eval_receipt();
  assert_eq!(
    as_str(get_path(
      run,
      &["six-layer-application-fold", "surface", "source-policy-proof"]
    )),
    "proof.macro-only.self.bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-policy.v1"
  );
  assert_eq!(
    as_i64(get_path(
      run,
      &[
        "six-layer-application-fold",
        "runtime",
        "applied-new-callsite-count"
      ]
    )),
    2
  );
  assert_eq!(
    as_i64(get_path(
      run,
      &[
        "six-layer-application-fold",
        "runtime",
        "total-applied-callsite-count"
      ]
    )),
    3
  );
  assert!(as_bool(get_path(
    run,
    &[
      "six-layer-application-fold",
      "semantic",
      "callsite-widening-applied"
    ]
  )));
  assert!(as_bool(get_path(
    run,
    &[
      "six-layer-application-fold",
      "semantic",
      "measurement-required"
    ]
  )));
  assert_eq!(
    as_i64(get_path(
      run,
      &[
        "six-layer-application-fold",
        "gate",
        "negative-held-rerun-count"
      ]
    )),
    6
  );
  for key in [
    "global-default-callsite-replaced",
    "global-speedup-claimed",
    "runtime-install",
    "global-ontology-runtime",
    "runtime-api-flattening",
    "meaning-db",
    "external-solver-installed",
    "self-modification",
  ] {
    assert!(!as_bool(get_path(
      run,
      &["six-layer-application-fold", "runtime", key]
    )));
  }
}

#[test]
fn discoveries_record_d723_through_d730() {
  let run = eval_receipt();
  let discoveries = attrs_by_id(get(run, "discoveries"));
  assert_eq!(discoveries.len(), 8);
  for expected in [
    "D723.callsite-widening-application-consumes-policy-proof",
    "D724.application-applies-exactly-two-new-callsites",
    "D725.selected-current-status-callsite-remains-baseline-not-reapplied",
    "D726.application-preserves-exact-field-shape-fallback-and-rollback",
    "D727.application-negative-helds-block-unlisted-policy-field-fallback-and-route-mismatch",
    "D728.application-is-not-global-default-replacement-or-global-speedup",
    "D729.application-preserves-runtime-external-license-and-authority-boundaries",
    "D730.next-frontier-is-widened-callsite-measurement-proof",
  ] {
    let d = discoveries
      .get(expected)
      .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
    assert!(as_bool(get(d, "scenario-only")));
    assert_eq!(as_str(get(d, "decision-pressure")), "keep");
  }
}

#[test]
fn top_level_flags_remain_candidate_only() {
  let run = eval_receipt();
  assert!(as_bool(get(
    run,
    "self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-application-proof"
  )));
  assert!(as_bool(get(run, "callsite-widening-policy-approved")));
  assert!(as_bool(get(run, "callsite-widening-applied")));
  assert!(as_bool(get(run, "additional-callsites-applied")));
  assert_eq!(as_i64(get(run, "applied-new-callsite-count")), 2);
  assert_eq!(as_i64(get(run, "total-applied-callsite-count")), 3);
  assert!(as_bool(get(run, "measurement-required")));
  for key in [
    "global-default-callsite-replaced",
    "global-speedup-claimed",
    "runtime-install",
    "global-ontology-runtime",
    "runtime-api-flattening",
    "meaning-db",
    "external-solver-installed",
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
  assert_eq!(as_str(get(run, "replacement-readiness")), "not-proven");
}
