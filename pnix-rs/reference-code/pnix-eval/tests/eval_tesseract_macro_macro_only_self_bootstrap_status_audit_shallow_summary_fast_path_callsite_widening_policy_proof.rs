//! Macro-only self bootstrap status audit shallow summary fast-path callsite
//! widening policy proof.
//!
//! This receipt consumes the selected-callsite measurement proof and approves a
//! narrow allowlist policy. It does not apply any additional callsites; that
//! remains a separate application proof frontier.

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
    "fixtures/tesseract-macro-legacy-probe/macro_only_self_bootstrap_status_audit_shallow_summary_fast_path_callsite_widening_policy_proof_receipt.px",
  )
}

fn eval_receipt() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = fixture_path();
    let json = std::thread::Builder::new()
      .name("eval-bootstrap-shallow-summary-fast-path-callsite-widening-policy-receipt".to_string())
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("self bootstrap status fast-path callsite widening policy receipt")
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
    "tesseract-macro-ontology-macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-policy-proof"
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
    "stdlib/lib/gate/macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-policy-proof.px",
    "fixtures/pnix-query-runtime/macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-policy-proof-owner.px",
    "fixtures/tesseract-macro-legacy-probe/macro_only_self_bootstrap_status_audit_shallow_summary_fast_path_callsite_widening_policy_proof_receipt.px",
  ] {
    assert!(repo_root().join(path).is_file(), "missing `{path}`");
  }
}

#[test]
fn constitution_gate_blocks_policy_overclaims() {
  let run = eval_receipt();
  let gate = get(run, "constitutionGate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-policy-proof"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "measurement-equals-global-callsite-widening",
    "policy-approval-equals-application",
    "allowlist-equals-global-default-callsite-replacement",
    "warm-repeat-within-threshold-equals-global-speedup",
    "policy-equals-global-runtime",
    "policy-equals-runtime-api-flattening",
    "policy-equals-meaning-db",
    "policy-equals-external-solver-intake",
    "policy-equals-self-modification",
    "policy-equals-llm-authority",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
}

#[test]
fn contract_records_allowlist_policy_without_application() {
  let run = eval_receipt();
  let contract = get(run, "policy-contract");
  assert_eq!(
    as_str(get(contract, "id")),
    "contract.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-policy-proof.v1"
  );
  assert_eq!(
    as_str(get(contract, "current-status")),
    "self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-policy-proof-present"
  );
  assert!(as_bool(get(contract, "closes-policy-frontier")));
  assert!(as_bool(get(contract, "opens-application-frontier")));
  assert_eq!(as_i64(get(contract, "allowed-callsite-count")), 3);
  assert_eq!(as_i64(get(contract, "eligible-new-callsite-count")), 2);
  assert!(as_bool(get(contract, "callsite-widening-policy-approved")));
  assert!(!as_bool(get(contract, "callsite-widening-applied")));
  assert!(!as_bool(get(contract, "additional-callsites-applied")));
  assert!(!as_bool(get(contract, "global-default-callsite-replaced")));
  assert!(!as_bool(get(contract, "global-speedup-claimed")));
}

#[test]
fn policy_proof_closes_only_policy_frontier() {
  let run = eval_receipt();
  let proof = get(run, "policy-proof");
  assert_eq!(
    as_str(get(proof, "status")),
    "self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-policy-proof-present"
  );
  assert!(as_bool(get(
    proof,
    "self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-policy-proof"
  )));
  assert!(as_bool(get(proof, "callsite-widening-policy-approved")));
  assert!(!as_bool(get(proof, "callsite-widening-applied")));
  assert!(!as_bool(get(proof, "additional-callsites-applied")));
  assert_eq!(as_i64(get(proof, "allowed-callsite-count")), 3);
  assert_eq!(as_i64(get(proof, "eligible-new-callsite-count")), 2);

  let closed = string_set(get(proof, "closes"));
  assert!(closed.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-policy-proof"
  ));
  let open = string_set(get(proof, "next-open-frontiers"));
  assert!(open.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-application-proof"
  ));
}

#[test]
fn trials_cover_allowlist_and_negative_held_cases() {
  let run = eval_receipt();
  let trials = attrs_by_id(get(run, "policy-trials"));
  assert_eq!(trials.len(), 19);
  assert_eq!(
    as_str(get(trials["trial.A.valid-policy-proof"], "outcome")),
    "self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-policy-proof-present"
  );
  assert_eq!(
    as_str(get(
      trials["trial.B.selected-callsite-policy-eligible"],
      "outcome"
    )),
    "policy-eligible"
  );
  assert_eq!(
    as_str(get(
      trials["trial.C.operator-panel-policy-eligible"],
      "callsite-id"
    )),
    "callsite.bootstrap-status-audit.operator-panel.shallow-summary.v1"
  );
  assert_eq!(
    as_str(get(
      trials["trial.D.index-status-policy-eligible"],
      "callsite-id"
    )),
    "callsite.bootstrap-status-audit.index-status.shallow-summary.v1"
  );

  for (trial, held_id) in [
    (
      "trial.E.full-json-held",
      "held.bootstrap-status-shallow-summary-callsite-widening-policy.full-json-shape",
    ),
    (
      "trial.F.not-allowlisted-held",
      "held.bootstrap-status-shallow-summary-callsite-widening-policy.not-allowlisted",
    ),
    (
      "trial.G.fallback-missing-held",
      "held.bootstrap-status-shallow-summary-callsite-widening-policy.fallback-missing",
    ),
    (
      "trial.H.global-overclaim-held",
      "held.bootstrap-status-shallow-summary-callsite-widening-policy.global-overclaim",
    ),
    (
      "trial.I.field-shape-held",
      "held.bootstrap-status-shallow-summary-callsite-widening-policy.field-shape",
    ),
    (
      "trial.J.domain-mismatch-held",
      "held.bootstrap-status-shallow-summary-callsite-widening-policy.domain-mismatch",
    ),
    (
      "trial.P.application-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-policy.application-overclaim",
    ),
  ] {
    assert_eq!(as_str(get(trials[trial], "outcome")), "Held", "`{trial}` status");
    assert_eq!(as_str(get(trials[trial], "held-id")), held_id, "`{trial}` held id");
  }
}

#[test]
fn six_layer_fold_preserves_policy_runtime_and_audit_boundaries() {
  let run = eval_receipt();
  assert_eq!(
    as_str(get_path(run, &["six-layer-policy-fold", "surface", "source-measurement-proof"])),
    "proof.macro-only.self.bootstrap-status-audit-shallow-summary-fast-path-application-measurement.v1"
  );
  assert_eq!(
    as_str(get_path(
      run,
      &["six-layer-policy-fold", "semantic", "measurement-envelope"]
    )),
    "cold-start-slow-warm-repeats-within-threshold"
  );
  assert!(as_bool(get_path(
    run,
    &[
      "six-layer-policy-fold",
      "semantic",
      "callsite-widening-policy-approved"
    ]
  )));
  assert!(!as_bool(get_path(
    run,
    &[
      "six-layer-policy-fold",
      "semantic",
      "callsite-widening-applied"
    ]
  )));
  assert_eq!(
    as_i64(get_path(
      run,
      &["six-layer-policy-fold", "gate", "negative-held-rerun-count"]
    )),
    6
  );
  for key in [
    "additional-callsites-applied",
    "global-default-callsite-replaced",
    "runtime-install",
    "global-ontology-runtime",
    "runtime-api-flattening",
    "meaning-db",
    "external-solver-installed",
    "self-modification",
  ] {
    assert!(!as_bool(get_path(
      run,
      &["six-layer-policy-fold", "runtime", key]
    )));
  }
}

#[test]
fn discoveries_record_d715_through_d722() {
  let run = eval_receipt();
  let discoveries = attrs_by_id(get(run, "discoveries"));
  assert_eq!(discoveries.len(), 8);
  for expected in [
    "D715.callsite-widening-policy-consumes-measured-fast-path-envelope",
    "D716.policy-allowlist-is-three-callsites-not-global-default",
    "D717.policy-classification-requires-exact-shallow-field-shape-and-fallback",
    "D718.negative-held-reruns-block-full-json-unlisted-fallback-global-field-and-domain-mismatch",
    "D719.policy-approval-is-not-widening-application",
    "D720.selected-callsite-remains-the-only-applied-route",
    "D721.policy-preserves-runtime-external-and-authority-boundaries",
    "D722.next-frontier-is-callsite-widening-application-proof",
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
    "self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-policy-proof"
  )));
  assert!(as_bool(get(run, "callsite-widening-policy-approved")));
  assert_eq!(as_i64(get(run, "allowed-callsite-count")), 3);
  assert_eq!(as_i64(get(run, "eligible-new-callsite-count")), 2);
  assert!(as_bool(get(run, "application-proof-required")));
  for key in [
    "callsite-widening-applied",
    "additional-callsites-applied",
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
