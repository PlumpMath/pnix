//! Macro-only self bootstrap status audit shallow summary fast-path global
//! default replacement readiness proof.
//!
//! This pins D739-D746: widened measurements can prove readiness for a later
//! bounded default replacement application, while replacement, global speedup,
//! cold-start solved, runtime install, meaning DB, external solver intake, and
//! authority claims stay false.

use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/tesseract-macro-legacy-probe/macro_only_self_bootstrap_status_audit_shallow_summary_fast_path_global_default_replacement_readiness_proof_receipt.px",
  )
}

fn eval_receipt() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = fixture_path();
    let json = std::thread::Builder::new()
      .name("eval-bootstrap-shallow-summary-fast-path-global-default-readiness-receipt".to_string())
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("global default replacement readiness receipt")
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
    "tesseract-macro-ontology-macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-readiness-proof"
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
fn constitution_gate_blocks_readiness_overclaims() {
  let run = eval_receipt();
  let gate = get(run, "constitutionGate");
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  let blocked = string_set(get(gate, "blocked-shortcuts"));
  for shortcut in [
    "readiness-equals-default-replacement",
    "readiness-equals-global-speedup",
    "readiness-erases-cold-start",
    "readiness-equals-global-runtime",
    "readiness-equals-runtime-api-flattening",
    "readiness-equals-meaning-db",
    "readiness-equals-external-solver-intake",
    "readiness-equals-p-puck-semantic-owner",
    "readiness-equals-self-modification",
  ] {
    assert!(blocked.contains(shortcut), "missing {shortcut}");
  }
}

#[test]
fn contract_records_readiness_without_replacement() {
  let run = eval_receipt();
  let contract = get(run, "readiness-contract");
  assert_eq!(
    as_str(get(contract, "current-status")),
    "self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-readiness-proof-present"
  );
  assert!(as_bool(get(contract, "closes-readiness-frontier")));
  assert!(as_bool(get(contract, "opens-application-frontier")));
  assert!(as_bool(get(contract, "global-default-readiness-proven")));
  assert!(as_bool(get(
    contract,
    "global-default-replacement-application-required"
  )));
  assert_eq!(as_i64(get(contract, "known-default-callsite-count")), 3);
  assert_eq!(
    as_i64(get(contract, "measured-known-default-callsite-count")),
    3
  );
  assert_eq!(
    as_i64(get(contract, "unmeasured-known-default-callsite-count")),
    0
  );
  assert_eq!(as_i64(get(contract, "measurement-record-count")), 8);
  assert_eq!(
    as_str(get(contract, "performance-envelope")),
    "widened-cold-start-slow-new-callsite-warm-repeats-within-threshold"
  );
  assert!(!as_bool(get(contract, "global-default-callsite-replaced")));
  assert!(!as_bool(get(contract, "global-speedup-claimed")));
  assert!(!as_bool(get(contract, "cold-start-solved")));
  assert!(!as_bool(get(contract, "runtime-install")));
  assert!(!as_bool(get(contract, "meaning-db")));
}

#[test]
fn readiness_proof_closes_only_readiness_frontier() {
  let run = eval_receipt();
  let proof = get(run, "readiness-proof");
  assert_eq!(
    as_str(get(proof, "readiness-verdict")),
    "ready-for-bounded-global-default-replacement-application"
  );
  assert!(as_bool(get(proof, "global-default-readiness-proven")));
  assert!(as_bool(get(
    proof,
    "global-default-replacement-application-required"
  )));
  assert!(!as_bool(get(proof, "global-default-callsite-replaced")));
  assert!(!as_bool(get(proof, "global-speedup-claimed")));
  assert!(!as_bool(get(proof, "cold-start-solved")));

  let closed = string_set(get(proof, "closes"));
  assert!(closed.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-readiness-proof"
  ));
  let open = string_set(get(proof, "next-open-frontiers"));
  assert!(open.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-application-proof"
  ));
}

#[test]
fn trials_cover_readiness_positive_and_held_cases() {
  let run = eval_receipt();
  let trials = attrs_by_id(get(run, "readiness-trials"));
  assert_eq!(
    as_str(get(trials["trial.A.valid-readiness-proof"], "outcome")),
    "self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-readiness-proof-present"
  );
  assert_eq!(
    as_i64(get(
      trials["trial.B.known-default-callsite-coverage"],
      "count"
    )),
    3
  );
  assert_eq!(
    as_str(get(trials["trial.D.next-frontier"], "outcome")),
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-application-proof"
  );

  for (trial, held_id) in [
    (
      "trial.E.wrong-proof-id",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-readiness.proof-id-mismatch",
    ),
    (
      "trial.F.stale-stage",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-readiness.stale-current-stage",
    ),
    (
      "trial.G.source-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-readiness.source-mismatch",
    ),
    (
      "trial.H.measurement-evidence-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-readiness.measurement-evidence-missing",
    ),
    (
      "trial.I.readiness-record-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-readiness.readiness-record-mismatch",
    ),
    (
      "trial.J.callsite-coverage-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-readiness.callsite-coverage-mismatch",
    ),
    (
      "trial.K.measurement-envelope-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-readiness.measurement-envelope-mismatch",
    ),
    (
      "trial.L.field-shape-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-readiness.field-shape-mismatch",
    ),
    (
      "trial.M.audit-fallback-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-readiness.audit-fallback-missing",
    ),
    (
      "trial.N.negative-held-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-readiness.negative-held-boundary-missing",
    ),
    (
      "trial.O.frontier-shape-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-readiness.frontier-shape-mismatch",
    ),
    (
      "trial.P.replacement-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-readiness.replacement-overclaim",
    ),
    (
      "trial.Q.runtime-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-readiness.runtime-overclaim",
    ),
    (
      "trial.R.external-or-license-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-readiness.external-or-license-overclaim",
    ),
    (
      "trial.S.authority-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-readiness.authority-overclaim",
    ),
  ] {
    assert_eq!(as_str(get(trials[trial], "outcome")), "Held", "{trial}");
    assert_eq!(as_str(get(trials[trial], "held-id")), held_id, "{trial}");
  }
}

#[test]
fn six_layer_fold_separates_readiness_from_application() {
  let run = eval_receipt();
  assert!(as_bool(get_path(
    run,
    &["six-layer-readiness-fold", "surface", "visible"]
  )));
  assert_eq!(
    as_i64(get_path(
      run,
      &[
        "six-layer-readiness-fold",
        "ontology",
        "known-default-callsite-count"
      ]
    )),
    3
  );
  assert_eq!(
    as_i64(get_path(
      run,
      &[
        "six-layer-readiness-fold",
        "ontology",
        "unmeasured-known-default-callsite-count"
      ]
    )),
    0
  );
  assert!(as_bool(get_path(
    run,
    &[
      "six-layer-readiness-fold",
      "semantic",
      "global-default-readiness-proven"
    ]
  )));
  assert!(as_bool(get_path(
    run,
    &[
      "six-layer-readiness-fold",
      "semantic",
      "global-default-replacement-application-required"
    ]
  )));
  assert!(!as_bool(get_path(
    run,
    &[
      "six-layer-readiness-fold",
      "semantic",
      "global-default-callsite-replaced"
    ]
  )));
  assert!(as_bool(get_path(
    run,
    &[
      "six-layer-readiness-fold",
      "gate",
      "blocked-replacement-overclaim"
    ]
  )));
  assert!(!as_bool(get_path(
    run,
    &["six-layer-readiness-fold", "runtime", "runtime-install"]
  )));
}

#[test]
fn discoveries_record_d739_through_d746() {
  let run = eval_receipt();
  let discoveries = attrs_by_id(get(run, "discoveries"));
  for id in [
    "D739.global-default-readiness-consumes-widened-measurement-not-application",
    "D740.known-default-callsite-set-is-exactly-three-and-all-measured",
    "D741.readiness-proven-opens-application-frontier-only",
    "D742.cold-start-remains-separate-from-readiness",
    "D743.negative-held-boundaries-survive-readiness",
    "D744.readiness-keeps-exact-field-shape-fallback-and-rollback",
    "D745.readiness-preserves-runtime-external-license-and-authority-boundaries",
    "D746.next-frontier-is-global-default-replacement-application-proof",
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
    "global-default-replacement-readiness-proof"
  )));
  assert!(as_bool(get(run, "global-default-readiness-proven")));
  assert!(as_bool(get(
    run,
    "global-default-replacement-application-required"
  )));
  assert!(!as_bool(get(run, "global-default-callsite-replaced")));
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
