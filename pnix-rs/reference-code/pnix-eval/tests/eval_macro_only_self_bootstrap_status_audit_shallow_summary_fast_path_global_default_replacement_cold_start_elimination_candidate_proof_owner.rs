use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn combined_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../stdlib/lib/gate/macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-proof.px",
  )
}

fn eval_combined() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = combined_path();
    let json = std::thread::Builder::new()
      .name("cold-start-elimination-candidate-combined-eval".to_string())
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("cold-start elimination candidate combined eval")
      })
      .expect("spawn eval thread")
      .join()
      .expect("eval thread panicked");
    serde_json::from_str(&json).expect("combined JSON")
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

fn fixture() -> &'static Value {
  static F: OnceLock<&'static Value> = OnceLock::new();
  F.get_or_init(|| get(eval_combined(), "owner-fixture"))
}

fn string_set(v: &Value) -> BTreeSet<&str> {
  as_list(v).iter().map(as_str).collect()
}

#[test]
fn fixture_imports_elimination_candidate_owner_and_policy_source() {
  let run = fixture();
  assert_eq!(
    as_str(get(run, "proof")),
    "macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-proof-owner"
  );
  assert!(as_bool(get(run, "imported-owner")));
  assert!(as_bool(get(run, "imported-policy-owner")));
  assert!(as_bool(get(run, "used-px-owner")));
  assert_eq!(
    as_str(get(run, "expected-source-proof")),
    "proof.macro-only.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution-policy.v1"
  );
}

#[test]
fn owner_meta_marks_wrapper_bypass_selected_others_deferred() {
  let run = fixture();
  let meta = get(run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.lib.gate.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-proof"
  );
  assert!(as_bool(get(
    meta,
    "global-default-replacement-cold-start-elimination-candidate-proof"
  )));
  assert!(as_bool(get(meta, "elimination-candidate-only")));
  assert_eq!(
    as_str(get(meta, "selected-candidate-id")),
    "elimination.candidate.wrapper-bypass.bootstrap-status-audit-shallow-summary-status-query"
  );
  assert_eq!(
    as_str(get(meta, "selected-candidate-kind")),
    "wrapper-bypass"
  );
  assert!(as_bool(get(
    meta,
    "elimination-candidate-application-frontier-required"
  )));
  assert_eq!(
    as_str(get(meta, "candidate-verdict")),
    "wrapper-bypass-elimination-candidate-selected-not-applied"
  );
  assert!(!as_bool(get(meta, "cold-start-solved")));
  assert!(!as_bool(get(meta, "cold-start-eliminated")));
  assert!(!as_bool(get(meta, "wrapper-bypass-applied")));
  assert!(!as_bool(get(meta, "elimination-applied")));
}

#[test]
fn valid_proof_closes_elimination_candidate_and_opens_application() {
  let run = fixture();
  let valid = get(run, "valid-proof");
  assert_eq!(
    as_str(get(valid, "status")),
    "self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-proof-present"
  );
  assert!(get(valid, "held-id").is_null());
  assert!(as_bool(get(valid, "elimination-candidate-only")));
  assert_eq!(as_i64(get(valid, "elimination-candidate-count")), 3);
  assert_eq!(as_i64(get(valid, "selected-candidate-count")), 1);

  let closed = string_set(get(valid, "closes"));
  assert!(closed.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-proof"
  ));
  let open = string_set(get(valid, "next-open-frontiers"));
  assert!(open.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application-proof"
  ));
}

#[test]
fn candidate_registry_records_three_candidates_one_selected() {
  let run = fixture();
  let reg = get(run, "candidate-registry");
  assert_eq!(
    as_str(get(reg, "id")),
    "elimination-candidate-registry.fast-path.bootstrap-status-audit.shallow-summary.global-default-replacement.cold-start.v1"
  );
  assert_eq!(as_i64(get(reg, "candidate-count")), 3);
  assert_eq!(as_i64(get(reg, "selected-candidate-count")), 1);
  assert_eq!(as_i64(get(reg, "not-selected-candidate-count")), 2);
  assert_eq!(
    as_str(get(reg, "selected-candidate-kind")),
    "wrapper-bypass"
  );
  assert_eq!(
    as_str(get(
      reg,
      "selected-candidate-proposed-action-implementation-hint"
    )),
    "direct-evaluator-call-without-p-puck-cargo-spawn"
  );
  assert!(as_bool(get(reg, "elimination-candidate-only")));
  assert!(as_bool(get(
    reg,
    "elimination-candidate-application-frontier-required"
  )));
  assert!(!as_bool(get(reg, "cold-start-solved")));
  assert!(!as_bool(get(reg, "cold-start-eliminated")));
  assert!(!as_bool(get(reg, "wrapper-bypass-applied")));
  assert!(!as_bool(get(reg, "elimination-applied")));
}

#[test]
fn held_failures_cover_inputs_selection_registry_frontier_and_overclaims() {
  let run = fixture();
  for (field, held_id) in [
    (
      "wrong-proof",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate.proof-id-mismatch",
    ),
    (
      "stale-stage",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate.stale-current-stage",
    ),
    (
      "source-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate.source-mismatch",
    ),
    (
      "policy-source-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate.policy-source-missing",
    ),
    (
      "policy-input-shape-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate.policy-input-shape-mismatch",
    ),
    (
      "candidate-shape-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate.candidate-shape-mismatch",
    ),
    (
      "candidate-invalid",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate.candidate-invalid",
    ),
    (
      "selection-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate.selection-mismatch",
    ),
    (
      "candidate-status-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate.candidate-status-mismatch",
    ),
    (
      "scope-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate.scope-mismatch",
    ),
    (
      "registry-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate.registry-mismatch",
    ),
    (
      "application-frontier-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate.application-frontier-missing",
    ),
    (
      "audit-fallback-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate.audit-fallback-missing",
    ),
    (
      "missing-evidence",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate.missing-required-evidence",
    ),
    (
      "frontier-shape-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate.frontier-shape-mismatch",
    ),
    (
      "candidate-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate.candidate-overclaim",
    ),
    (
      "speedup-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate.speedup-overclaim",
    ),
    (
      "runtime-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate.runtime-overclaim",
    ),
    (
      "external-or-license-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate.external-or-license-overclaim",
    ),
    (
      "authority-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate.authority-overclaim",
    ),
  ] {
    let value = get(run, field);
    assert_eq!(as_str(get(value, "status")), "Held", "{field}");
    assert_eq!(as_str(get(value, "held-id")), held_id, "{field}");
  }
}

#[test]
fn hard_stops_remain_false_after_elimination_candidate_proof() {
  let run = fixture();
  for key in [
    "cold-start-solved",
    "cold-start-eliminated",
    "wrapper-bypass-applied",
    "elimination-applied",
    "global-speedup-claimed",
    "whole-system-speedup-claimed",
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
  ] {
    assert!(!as_bool(get(run, key)), "`{key}` must stay false");
  }
}
