use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/pnix-query-runtime/macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-application-proof-owner.px",
  )
}

fn eval_fixture() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = fixture_path();
    let json = std::thread::Builder::new()
      .name(
        "bootstrap-shallow-summary-fast-path-callsite-widening-application-owner-eval".to_string(),
      )
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("shallow summary fast-path callsite widening application owner fixture")
      })
      .expect("spawn eval thread")
      .join()
      .expect("eval thread panicked");
    serde_json::from_str(&json).expect("fixture JSON")
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

fn string_set(v: &Value) -> BTreeSet<&str> {
  as_list(v).iter().map(as_str).collect()
}

fn attrs_by_callsite<'a>(items: &'a Value) -> BTreeMap<&'a str, &'a Value> {
  as_list(items)
    .iter()
    .map(|item| (as_str(get(item, "callsite-id")), item))
    .collect()
}

#[test]
fn fixture_imports_application_owner_and_policy_source() {
  let run = eval_fixture();
  assert_eq!(
    as_str(get(run, "proof")),
    "macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-application-proof-owner"
  );
  assert!(as_bool(get(run, "imported-owner")));
  assert!(as_bool(get(run, "imported-policy-owner")));
  assert!(as_bool(get(run, "imported-policy-fixture")));
  assert!(as_bool(get(run, "imported-binding-owner")));
  assert!(as_bool(get(run, "used-px-owner")));
  assert_eq!(
    as_str(get(run, "expected-source-proof")),
    "proof.macro-only.self.bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-policy.v1"
  );
}

#[test]
fn owner_meta_applies_two_new_callsites_without_global_replacement() {
  let run = eval_fixture();
  let meta = get(run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.lib.gate.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-application-proof"
  );
  assert!(as_bool(get(
    meta,
    "self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-application-proof"
  )));
  assert!(as_bool(get(meta, "callsite-widening-policy-approved")));
  assert!(as_bool(get(meta, "callsite-widening-applied")));
  assert!(as_bool(get(meta, "additional-callsites-applied")));
  assert_eq!(as_i64(get(meta, "applied-new-callsite-count")), 2);
  assert_eq!(as_i64(get(meta, "total-applied-callsite-count")), 3);
  assert!(as_bool(get(meta, "selected-callsite-remains-applied")));
  assert!(as_bool(get(meta, "measurement-required")));
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
  ] {
    assert!(!as_bool(get(meta, key)), "`{key}` must stay false");
  }
}

#[test]
fn application_results_cover_operator_panel_and_index_status() {
  let run = eval_fixture();
  let ids = string_set(get(run, "applied-new-callsite-ids"));
  assert_eq!(ids.len(), 2);
  assert!(ids.contains("callsite.bootstrap-status-audit.operator-panel.shallow-summary.v1"));
  assert!(ids.contains("callsite.bootstrap-status-audit.index-status.shallow-summary.v1"));

  let total_ids = string_set(get(run, "total-applied-callsite-ids"));
  assert_eq!(total_ids.len(), 3);
  assert!(total_ids.contains("callsite.bootstrap-status-audit.current-status.shallow-summary.v1"));

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
    assert!(as_bool(get(result, "full-audit-fallback-preserved")));
    assert_eq!(as_i64(get(result, "status-field-count")), 11);
    assert!(!as_bool(get(result, "global-default-callsite-replaced")));
  }
}

#[test]
fn valid_proof_closes_application_and_opens_measurement_frontier() {
  let run = eval_fixture();
  let valid = get(run, "valid-proof");
  assert_eq!(
    as_str(get(valid, "status")),
    "self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-application-proof-present"
  );
  assert!(get(valid, "held-id").is_null());
  assert!(as_bool(get(valid, "callsite-widening-applied")));
  assert!(as_bool(get(valid, "additional-callsites-applied")));
  assert_eq!(as_i64(get(valid, "applied-new-callsite-count")), 2);
  assert_eq!(as_i64(get(valid, "total-applied-callsite-count")), 3);

  let closed = string_set(get(valid, "closes"));
  assert!(closed.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-application-proof"
  ));
  let open = string_set(get(valid, "next-open-frontiers"));
  assert!(open.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement-proof"
  ));
}

#[test]
fn negative_callsite_applications_are_held() {
  let run = eval_fixture();
  for (field, held_id) in [
    (
      "negative-held-selected-reapply",
      "held.bootstrap-status-shallow-summary-callsite-widening-application.not-new-policy-target",
    ),
    (
      "negative-held-unlisted-callsite",
      "held.bootstrap-status-shallow-summary-callsite-widening-application.not-new-policy-target",
    ),
    (
      "negative-held-policy-approval-missing",
      "held.bootstrap-status-shallow-summary-callsite-widening-application.policy-approval-missing",
    ),
    (
      "negative-held-field-shape",
      "held.bootstrap-status-shallow-summary-callsite-widening-application.field-shape-mismatch",
    ),
    (
      "negative-held-fallback-missing",
      "held.bootstrap-status-shallow-summary-callsite-widening-application.fallback-missing",
    ),
    (
      "negative-held-route-result",
      "held.bootstrap-status-shallow-summary-callsite-widening-application.route-result-held",
    ),
  ] {
    let held = get(run, field);
    assert_eq!(as_str(get(held, "status")), "Held", "`{field}` status");
    assert_eq!(as_str(get(held, "held-id")), held_id, "`{field}` held id");
  }
}

#[test]
fn held_failures_cover_validator_overclaims() {
  let run = eval_fixture();
  for (field, held_id) in [
    (
      "wrong-proof",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-application.proof-id-mismatch",
    ),
    (
      "stale-stage",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-application.stale-current-stage",
    ),
    (
      "source-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-application.source-mismatch",
    ),
    (
      "policy-evidence-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-application.policy-evidence-missing",
    ),
    (
      "application-record-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-application.application-record-mismatch",
    ),
    (
      "result-shape-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-application.result-shape-mismatch",
    ),
    (
      "count-or-baseline-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-application.count-or-baseline-mismatch",
    ),
    (
      "negative-held-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-application.negative-held-missing",
    ),
    (
      "missing-evidence",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-application.missing-required-evidence",
    ),
    (
      "frontier-shape-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-application.frontier-shape-mismatch",
    ),
    (
      "global-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-application.global-overclaim",
    ),
    (
      "runtime-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-application.runtime-overclaim",
    ),
    (
      "external-or-license-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-application.external-or-license-overclaim",
    ),
    (
      "authority-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-application.authority-overclaim",
    ),
  ] {
    let held = get(run, field);
    assert_eq!(as_str(get(held, "status")), "Held", "`{field}` status");
    assert_eq!(as_str(get(held, "held-id")), held_id, "`{field}` held id");
  }
}
