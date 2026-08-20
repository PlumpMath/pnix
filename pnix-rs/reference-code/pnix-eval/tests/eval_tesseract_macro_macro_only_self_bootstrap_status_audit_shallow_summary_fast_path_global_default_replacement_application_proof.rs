//! Macro-only self bootstrap status audit shallow summary fast-path global
//! default replacement application proof.
//!
//! This pins D747-D754: bounded default replacement can apply to the known
//! bootstrap-status audit shallow-summary callsites, while global speedup,
//! cold-start solved, runtime install, meaning DB, external solver intake, and
//! authority claims remain blocked until later proofs.

use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/tesseract-macro-legacy-probe/macro_only_self_bootstrap_status_audit_shallow_summary_fast_path_global_default_replacement_application_proof_receipt.px",
  )
}

fn eval_receipt() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = fixture_path();
    let json = std::thread::Builder::new()
      .name(
        "eval-bootstrap-shallow-summary-fast-path-global-default-application-receipt".to_string(),
      )
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("global default replacement application receipt")
      })
      .expect("spawn eval thread")
      .join()
      .expect("eval thread panicked");
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

fn get_path<'a>(mut v: &'a Value, path: &[&str]) -> &'a Value {
  for key in path {
    v = get(v, key);
  }
  v
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
    "tesseract-macro-ontology-macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-application-proof"
  );
  assert_eq!(
    as_str(get(run, "constitution-owner")),
    "stdlib/lib/gate/tesseract-constitution.px"
  );
  assert_eq!(
    as_str(get(run, "truth-owner")),
    "project-wiki/maps/tesseract-macro-ontology-discovery-ledger.md"
  );
}

#[test]
fn constitution_gate_blocks_application_overclaims() {
  let run = eval_receipt();
  let gate = get(run, "constitutionGate");
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  let blocked = string_set(get(gate, "blocked-shortcuts"));
  for shortcut in [
    "application-equals-global-speedup",
    "application-erases-cold-start",
    "application-equals-global-runtime",
    "application-equals-runtime-api-flattening",
    "application-equals-meaning-db",
    "application-equals-external-solver-intake",
    "application-equals-p-puck-semantic-owner",
    "application-equals-self-modification",
    "application-replaces-unmeasured-callsites",
    "application-replaces-full-json-shape",
  ] {
    assert!(blocked.contains(shortcut), "missing {shortcut}");
  }
}

#[test]
fn contract_records_bounded_default_replacement_without_speedup() {
  let run = eval_receipt();
  let contract = get(run, "application-contract");
  assert_eq!(
    as_str(get(contract, "current-status")),
    "self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-application-proof-present"
  );
  assert!(as_bool(get(contract, "closes-application-frontier")));
  assert!(as_bool(get(contract, "opens-measurement-frontier")));
  assert!(as_bool(get(contract, "bounded-global-default-replacement")));
  assert!(as_bool(get(contract, "global-default-replacement-applied")));
  assert!(as_bool(get(contract, "global-default-callsite-replaced")));
  assert_eq!(as_i64(get(contract, "known-default-callsite-count")), 3);
  assert_eq!(as_i64(get(contract, "replaced-default-callsite-count")), 3);
  assert_eq!(
    as_i64(get(contract, "unmeasured-callsite-replacement-count")),
    0
  );
  assert!(as_bool(get(
    contract,
    "post-application-measurement-required"
  )));
  assert!(!as_bool(get(contract, "global-speedup-claimed")));
  assert!(!as_bool(get(contract, "cold-start-solved")));
  assert!(!as_bool(get(contract, "runtime-install")));
  assert!(!as_bool(get(contract, "meaning-db")));
}

#[test]
fn application_proof_closes_only_application_frontier() {
  let run = eval_receipt();
  let proof = get(run, "application-proof");
  assert_eq!(
    as_str(get(proof, "replacement-verdict")),
    "bounded-global-default-replacement-applied"
  );
  assert!(as_bool(get(proof, "global-default-replacement-applied")));
  assert!(as_bool(get(proof, "global-default-callsite-replaced")));
  assert!(as_bool(get(proof, "post-application-measurement-required")));
  assert!(!as_bool(get(proof, "global-speedup-claimed")));
  assert!(!as_bool(get(proof, "cold-start-solved")));

  let closed = string_set(get(proof, "closes"));
  assert!(closed.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-application-proof"
  ));
  let open = string_set(get(proof, "next-open-frontiers"));
  assert!(open.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-measurement-proof"
  ));
}

#[test]
fn trials_cover_application_positive_and_held_cases() {
  let run = eval_receipt();
  let trials = attrs_by_id(get(run, "application-trials"));
  assert_eq!(
    as_str(get(trials["trial.A.valid-application-proof"], "outcome")),
    "self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-application-proof-present"
  );
  assert_eq!(
    as_i64(get(
      trials["trial.B.known-default-replacement-coverage"],
      "count"
    )),
    3
  );
  assert_eq!(
    as_str(get(trials["trial.D.next-frontier"], "outcome")),
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-measurement-proof"
  );

  for (trial, held_id) in [
    (
      "trial.E.wrong-proof-id",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-application.proof-id-mismatch",
    ),
    (
      "trial.F.stale-stage",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-application.stale-current-stage",
    ),
    (
      "trial.G.source-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-application.source-mismatch",
    ),
    (
      "trial.H.readiness-evidence-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-application.readiness-evidence-missing",
    ),
    (
      "trial.I.replacement-record-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-application.replacement-record-mismatch",
    ),
    (
      "trial.J.callsite-coverage-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-application.callsite-coverage-mismatch",
    ),
    (
      "trial.K.field-shape-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-application.field-shape-mismatch",
    ),
    (
      "trial.L.audit-fallback-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-application.audit-fallback-missing",
    ),
    (
      "trial.M.negative-held-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-application.negative-held-boundary-missing",
    ),
    (
      "trial.N.frontier-shape-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-application.frontier-shape-mismatch",
    ),
    (
      "trial.O.replacement-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-application.replacement-overclaim",
    ),
    (
      "trial.P.runtime-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-application.runtime-overclaim",
    ),
    (
      "trial.Q.external-or-license-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-application.external-or-license-overclaim",
    ),
    (
      "trial.R.authority-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-application.authority-overclaim",
    ),
  ] {
    assert_eq!(as_str(get(trials[trial], "outcome")), "Held", "{trial}");
    assert_eq!(as_str(get(trials[trial], "held-id")), held_id, "{trial}");
  }
}

#[test]
fn six_layer_fold_separates_application_from_measurement_and_runtime() {
  let run = eval_receipt();
  assert!(as_bool(get_path(
    run,
    &["six-layer-application-fold", "surface", "visible"]
  )));
  assert_eq!(
    as_i64(get_path(
      run,
      &[
        "six-layer-application-fold",
        "ontology",
        "unmeasured-callsite-replacement-count"
      ]
    )),
    0
  );
  assert!(as_bool(get_path(
    run,
    &[
      "six-layer-application-fold",
      "semantic",
      "bounded-global-default-replacement"
    ]
  )));
  assert!(as_bool(get_path(
    run,
    &[
      "six-layer-application-fold",
      "semantic",
      "global-default-callsite-replaced"
    ]
  )));
  assert!(!as_bool(get_path(
    run,
    &[
      "six-layer-application-fold",
      "semantic",
      "global-speedup-claimed"
    ]
  )));
  assert!(as_bool(get_path(
    run,
    &[
      "six-layer-application-fold",
      "runtime",
      "post-application-measurement-required"
    ]
  )));
  assert!(!as_bool(get_path(
    run,
    &["six-layer-application-fold", "runtime", "runtime-install"]
  )));
}

#[test]
fn discoveries_record_d747_through_d754() {
  let run = eval_receipt();
  let discoveries = attrs_by_id(get(run, "discoveries"));
  for id in [
    "D747.application-consumes-readiness-not-runtime",
    "D748.bounded-default-replacement-covers-exactly-three-known-callsites",
    "D749.global-default-callsite-replaced-is-bounded-route-state",
    "D750.application-opens-measurement-before-speedup-or-cold-start-claims",
    "D751.unmeasured-or-full-json-default-replacement-stays-held",
    "D752.negative-held-boundaries-survive-default-replacement-application",
    "D753.application-preserves-runtime-external-license-and-authority-boundaries",
    "D754.next-frontier-is-global-default-replacement-measurement-proof",
  ] {
    let discovery = discoveries[id];
    assert!(as_bool(get(discovery, "scenario-only")));
    assert_eq!(as_str(get(discovery, "decision-pressure")), "keep");
  }
}

#[test]
fn final_flags_keep_receipt_candidate_only() {
  let run = eval_receipt();
  assert!(as_bool(get(
    run,
    "global-default-replacement-application-proof"
  )));
  assert!(as_bool(get(run, "global-default-replacement-applied")));
  assert!(as_bool(get(run, "bounded-global-default-replacement")));
  assert!(as_bool(get(run, "global-default-callsite-replaced")));
  assert!(as_bool(get(run, "post-application-measurement-required")));
  assert!(!as_bool(get(run, "global-speedup-claimed")));
  assert!(!as_bool(get(run, "cold-start-solved")));
  assert!(!as_bool(get(run, "runtime-install")));
  assert!(!as_bool(get(run, "global-ontology-runtime")));
  assert!(!as_bool(get(run, "runtime-api-flattening")));
  assert!(!as_bool(get(run, "meaning-db")));
  assert!(!as_bool(get(run, "external-solver-installed")));
  assert!(!as_bool(get(run, "self-modification")));
  assert!(!as_bool(get(run, "llm-authority")));
  assert!(!as_bool(get(run, "p-puck-is-semantic-owner")));
  assert!(!as_bool(get(run, "old-host-authority")));
  assert!(!as_bool(get(run, "gpl-family-dependencies")));
  assert!(!as_bool(get(run, "implementation-command")));
}
