use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/pnix-query-runtime/macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application-measurement-proof-owner.px",
  )
}

fn eval_fixture() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = fixture_path();
    let json = std::thread::Builder::new()
      .name("bootstrap-shallow-summary-fast-path-application-measurement-owner-eval".to_string())
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("shallow summary fast-path application measurement owner fixture")
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

fn attrs_by_id<'a>(items: &'a Value) -> BTreeMap<&'a str, &'a Value> {
  as_list(items)
    .iter()
    .map(|item| (as_str(get(item, "id")), item))
    .collect()
}

#[test]
fn fixture_imports_measurement_owner_and_application_source() {
  let run = eval_fixture();
  assert_eq!(
    as_str(get(run, "proof")),
    "macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application-measurement-proof-owner"
  );
  assert!(as_bool(get(run, "imported-owner")));
  assert!(as_bool(get(run, "imported-application-owner")));
  assert!(as_bool(get(run, "imported-application-fixture")));
  assert!(as_bool(get(run, "used-px-owner")));
  assert_eq!(
    as_str(get(run, "expected-source-proof")),
    "proof.macro-only.self.bootstrap-status-audit-shallow-summary-fast-path-application.v1"
  );
}

#[test]
fn owner_meta_records_measurement_without_global_claims() {
  let run = eval_fixture();
  let meta = get(run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.lib.gate.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application-measurement-proof"
  );
  assert_eq!(
    as_str(get(meta, "constructor")),
    "validateSelfBootstrapStatusAuditShallowSummaryFastPathApplicationMeasurementProof"
  );
  assert!(as_bool(get(
    meta,
    "self-bootstrap-status-audit-shallow-summary-fast-path-application-measurement-proof"
  )));
  assert!(as_bool(get(meta, "selected-callsite-measured")));
  assert_eq!(as_i64(get(meta, "measurement-record-count")), 3);
  assert_eq!(as_i64(get(meta, "cold-start-duration-ms")), 10982);
  assert_eq!(as_i64(get(meta, "warm-repeat-min-duration-ms")), 275);
  assert_eq!(as_i64(get(meta, "warm-repeat-max-duration-ms")), 358);
  assert!(as_bool(get(meta, "cold-start-slow-path-candidate")));
  assert!(as_bool(get(meta, "warm-repeats-within-threshold")));
  assert!(!as_bool(get(meta, "persistent-warm-slow-path")));
  assert_eq!(
    as_str(get(meta, "performance-envelope")),
    "cold-start-slow-warm-repeats-within-threshold"
  );
  for key in [
    "global-speedup-claimed",
    "cold-start-solved",
    "callsite-widening-approved",
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
fn measurement_records_pin_actual_p_puck_samples() {
  let run = eval_fixture();
  let records = attrs_by_id(get(run, "expected-measurement-records"));
  assert_eq!(records.len(), 3);

  let cold = records["measurement.1.application-status-query.cold-start"];
  assert_eq!(as_i64(get(cold, "duration-ms")), 10982);
  assert_eq!(as_str(get(cold, "status")), "slow-path-candidate");
  assert_eq!(as_str(get(cold, "sample-kind")), "cold-start");
  assert!(!as_bool(get(cold, "within-threshold")));

  let warm_one = records["measurement.2.application-status-query.warm-repeat"];
  assert_eq!(as_i64(get(warm_one, "duration-ms")), 358);
  assert_eq!(as_str(get(warm_one, "status")), "within-threshold");
  assert!(as_bool(get(warm_one, "within-threshold")));

  let warm_two = records["measurement.3.application-status-query.warm-repeat"];
  assert_eq!(as_i64(get(warm_two, "duration-ms")), 275);
  assert_eq!(as_str(get(warm_two, "status")), "within-threshold");
  assert!(as_bool(get(warm_two, "within-threshold")));

  for record in records.values() {
    assert_eq!(
      as_str(get(record, "output-token")),
      "self-bootstrap-status-audit-shallow-summary-fast-path-application-proof-present"
    );
    assert_eq!(as_str(get(record, "telemetry-source")), "p-puck");
    assert!(as_bool(get(record, "exit-zero")));
    assert!(as_bool(get(record, "output-stable")));
    assert!(!as_bool(get(record, "p-puck-is-semantic-owner")));
  }
}

#[test]
fn valid_proof_closes_measurement_and_opens_widening_policy() {
  let run = eval_fixture();
  let valid = get(run, "valid-proof");
  assert_eq!(
    as_str(get(valid, "status")),
    "self-bootstrap-status-audit-shallow-summary-fast-path-application-measurement-proof-present"
  );
  assert!(get(valid, "held-id").is_null());
  assert!(as_bool(get(valid, "selected-callsite-measured")));
  assert_eq!(as_i64(get(valid, "measurement-record-count")), 3);
  assert_eq!(as_i64(get(valid, "cold-start-duration-ms")), 10982);
  assert_eq!(as_i64(get(valid, "warm-repeat-one-duration-ms")), 358);
  assert_eq!(as_i64(get(valid, "warm-repeat-two-duration-ms")), 275);
  assert_eq!(as_i64(get(valid, "cold-to-warm-max-delta-ms")), -10624);
  assert!(as_bool(get(valid, "cold-start-slow-path-candidate")));
  assert!(as_bool(get(valid, "warm-repeats-within-threshold")));
  assert!(!as_bool(get(valid, "global-speedup-claimed")));
  assert!(!as_bool(get(valid, "callsite-widening-approved")));
  assert!(as_bool(get(valid, "callsite-widening-policy-required")));

  let closed = string_set(get(valid, "closes"));
  assert!(closed.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-application-measurement-proof"
  ));
  let open = string_set(get(valid, "next-open-frontiers"));
  assert!(open.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-policy-proof"
  ));
}

#[test]
fn held_failures_cover_measurement_shape_and_overclaims() {
  let run = eval_fixture();
  for (field, held_id) in [
    (
      "wrong-proof",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application-measurement.proof-id-mismatch",
    ),
    (
      "stale-stage",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application-measurement.stale-current-stage",
    ),
    (
      "source-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application-measurement.source-mismatch",
    ),
    (
      "application-evidence-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application-measurement.application-evidence-missing",
    ),
    (
      "record-shape-invalid",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application-measurement.record-shape-invalid",
    ),
    (
      "sample-values-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application-measurement.sample-values-mismatch",
    ),
    (
      "envelope-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application-measurement.envelope-mismatch",
    ),
    (
      "audit-fallback-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application-measurement.audit-fallback-missing",
    ),
    (
      "missing-evidence",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application-measurement.missing-required-evidence",
    ),
    (
      "frontier-shape-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application-measurement.frontier-shape-mismatch",
    ),
    (
      "measurement-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application-measurement.measurement-overclaim",
    ),
    (
      "runtime-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application-measurement.runtime-overclaim",
    ),
    (
      "external-or-license-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application-measurement.external-or-license-overclaim",
    ),
    (
      "authority-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application-measurement.authority-overclaim",
    ),
  ] {
    let result = get(run, field);
    assert_eq!(as_str(get(result, "status")), "Held");
    assert_eq!(as_str(get(result, "held-id")), held_id);
    assert!(!as_bool(get(
      result,
      "self-bootstrap-status-audit-shallow-summary-fast-path-application-measurement-proof"
    )));
  }
}

#[test]
fn final_fixture_flags_keep_measurement_scoped() {
  let run = eval_fixture();
  assert!(as_bool(get(
    run,
    "self-bootstrap-status-audit-shallow-summary-fast-path-application-measurement-proof"
  )));
  assert!(as_bool(get(run, "selected-callsite-measured")));
  assert!(as_bool(get(run, "warm-repeats-within-threshold")));
  assert!(as_bool(get(run, "cold-start-slow-path-candidate")));
  assert!(!as_bool(get(run, "persistent-warm-slow-path")));
  assert!(!as_bool(get(run, "global-speedup-claimed")));
  assert!(!as_bool(get(run, "cold-start-solved")));
  assert!(!as_bool(get(run, "callsite-widening-approved")));
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
  ] {
    assert!(!as_bool(get(run, key)), "`{key}` must stay false");
  }
}
