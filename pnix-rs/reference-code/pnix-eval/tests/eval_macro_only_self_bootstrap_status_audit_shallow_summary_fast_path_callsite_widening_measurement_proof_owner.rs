use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/pnix-query-runtime/macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement-proof-owner.px",
  )
}

fn eval_fixture() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = fixture_path();
    let json = std::thread::Builder::new()
      .name(
        "bootstrap-shallow-summary-fast-path-callsite-widening-measurement-owner-eval".to_string(),
      )
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("shallow summary fast-path callsite widening measurement owner fixture")
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
fn fixture_imports_widening_measurement_owner_and_sources() {
  let run = eval_fixture();
  assert_eq!(
    as_str(get(run, "proof")),
    "macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement-proof-owner"
  );
  assert!(as_bool(get(run, "imported-owner")));
  assert!(as_bool(get(run, "imported-application-owner")));
  assert!(as_bool(get(run, "imported-application-fixture")));
  assert!(as_bool(get(run, "imported-selected-measurement-owner")));
  assert!(as_bool(get(run, "imported-selected-measurement-fixture")));
  assert!(as_bool(get(run, "used-px-owner")));
  assert_eq!(
    as_str(get(run, "expected-source-proof")),
    "proof.macro-only.self.bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-application.v1"
  );
}

#[test]
fn owner_meta_records_widened_measurement_without_global_claims() {
  let run = eval_fixture();
  let meta = get(run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.lib.gate.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement-proof"
  );
  assert_eq!(
    as_str(get(meta, "constructor")),
    "validateSelfBootstrapStatusAuditShallowSummaryFastPathCallsiteWideningMeasurementProof"
  );
  assert!(as_bool(get(
    meta,
    "self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement-proof"
  )));
  assert!(as_bool(get(meta, "widened-callsite-measured")));
  assert!(as_bool(get(
    meta,
    "selected-baseline-measurement-inherited"
  )));
  assert_eq!(
    as_i64(get(meta, "selected-baseline-measurement-record-count")),
    3
  );
  assert_eq!(as_i64(get(meta, "measured-callsite-count")), 3);
  assert_eq!(as_i64(get(meta, "new-measured-callsite-count")), 2);
  assert_eq!(as_i64(get(meta, "new-measurement-record-count")), 5);
  assert_eq!(as_i64(get(meta, "combined-measurement-record-count")), 8);
  assert_eq!(as_i64(get(meta, "widened-cold-start-duration-ms")), 10426);
  assert_eq!(as_i64(get(meta, "new-callsite-warm-min-duration-ms")), 763);
  assert_eq!(as_i64(get(meta, "new-callsite-warm-max-duration-ms")), 827);
  assert!(as_bool(get(meta, "widened-cold-start-slow-path-candidate")));
  assert!(as_bool(get(
    meta,
    "new-callsite-warm-repeats-within-threshold"
  )));
  assert!(as_bool(get(
    meta,
    "global-default-readiness-proof-required"
  )));
  assert!(!as_bool(get(meta, "global-default-readiness-proven")));
  for key in [
    "global-default-callsite-replaced",
    "global-speedup-claimed",
    "cold-start-solved",
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
fn measurement_records_pin_actual_new_callsite_p_puck_samples() {
  let run = eval_fixture();
  let records = attrs_by_id(get(run, "expected-new-measurement-records"));
  assert_eq!(records.len(), 5);

  let operator_cold = records["measurement.1.callsite-widening.operator-panel.cold-start"];
  assert_eq!(as_i64(get(operator_cold, "duration-ms")), 10426);
  assert_eq!(as_str(get(operator_cold, "status")), "slow-path-candidate");
  assert_eq!(as_str(get(operator_cold, "sample-kind")), "cold-start");
  assert!(!as_bool(get(operator_cold, "within-threshold")));

  for (id, duration, callsite) in [
    (
      "measurement.2.callsite-widening.operator-panel.warm-repeat",
      809,
      "callsite.bootstrap-status-audit.operator-panel.shallow-summary.v1",
    ),
    (
      "measurement.3.callsite-widening.operator-panel.warm-repeat",
      763,
      "callsite.bootstrap-status-audit.operator-panel.shallow-summary.v1",
    ),
    (
      "measurement.4.callsite-widening.index-status.warm-repeat",
      786,
      "callsite.bootstrap-status-audit.index-status.shallow-summary.v1",
    ),
    (
      "measurement.5.callsite-widening.index-status.warm-repeat",
      827,
      "callsite.bootstrap-status-audit.index-status.shallow-summary.v1",
    ),
  ] {
    let record = records[id];
    assert_eq!(as_i64(get(record, "duration-ms")), duration);
    assert_eq!(as_str(get(record, "callsite-id")), callsite);
    assert_eq!(as_str(get(record, "status")), "within-threshold");
    assert!(as_bool(get(record, "within-threshold")));
    assert_eq!(
      as_str(get(record, "output-token")),
      "widened-callsite-fast-path-applied-shallow-summary-read"
    );
    assert_eq!(as_str(get(record, "telemetry-source")), "p-puck");
    assert!(!as_bool(get(record, "p-puck-is-semantic-owner")));
  }
}

#[test]
fn valid_proof_closes_measurement_and_opens_default_readiness() {
  let run = eval_fixture();
  let valid = get(run, "valid-proof");
  assert_eq!(
    as_str(get(valid, "status")),
    "self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement-proof-present"
  );
  assert!(get(valid, "held-id").is_null());
  assert!(as_bool(get(valid, "widened-callsite-measured")));
  assert!(as_bool(get(
    valid,
    "selected-baseline-measurement-inherited"
  )));
  assert_eq!(as_i64(get(valid, "measured-callsite-count")), 3);
  assert_eq!(as_i64(get(valid, "new-measurement-record-count")), 5);
  assert_eq!(as_i64(get(valid, "combined-measurement-record-count")), 8);
  assert!(as_bool(get(
    valid,
    "new-callsite-warm-repeats-within-threshold"
  )));
  assert!(as_bool(get(
    valid,
    "global-default-readiness-proof-required"
  )));
  assert!(!as_bool(get(valid, "global-default-readiness-proven")));
  assert!(!as_bool(get(valid, "global-default-callsite-replaced")));
  assert!(!as_bool(get(valid, "global-speedup-claimed")));

  let measured = string_set(get(valid, "measured-callsite-ids"));
  assert!(measured.contains("callsite.bootstrap-status-audit.current-status.shallow-summary.v1"));
  assert!(measured.contains("callsite.bootstrap-status-audit.operator-panel.shallow-summary.v1"));
  assert!(measured.contains("callsite.bootstrap-status-audit.index-status.shallow-summary.v1"));

  let closed = string_set(get(valid, "closes"));
  assert!(closed.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement-proof"
  ));
  let open = string_set(get(valid, "next-open-frontiers"));
  assert!(open.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-readiness-proof"
  ));
}

#[test]
fn held_failures_cover_measurement_shape_coverage_and_overclaims() {
  let run = eval_fixture();
  for (field, held_id) in [
    (
      "wrong-proof",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement.proof-id-mismatch",
    ),
    (
      "stale-stage",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement.stale-current-stage",
    ),
    (
      "source-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement.source-mismatch",
    ),
    (
      "application-evidence-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement.application-evidence-missing",
    ),
    (
      "selected-baseline-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement.selected-baseline-missing",
    ),
    (
      "record-shape-invalid",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement.record-shape-invalid",
    ),
    (
      "sample-values-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement.sample-values-mismatch",
    ),
    (
      "coverage-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement.coverage-mismatch",
    ),
    (
      "envelope-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement.envelope-mismatch",
    ),
    (
      "audit-fallback-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement.audit-fallback-missing",
    ),
    (
      "missing-evidence",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement.missing-required-evidence",
    ),
    (
      "frontier-shape-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement.frontier-shape-mismatch",
    ),
    (
      "measurement-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement.measurement-overclaim",
    ),
    (
      "runtime-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement.runtime-overclaim",
    ),
    (
      "external-or-license-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement.external-or-license-overclaim",
    ),
    (
      "authority-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement.authority-overclaim",
    ),
  ] {
    let result = get(run, field);
    assert_eq!(as_str(get(result, "status")), "Held");
    assert_eq!(as_str(get(result, "held-id")), held_id);
    assert!(!as_bool(get(
      result,
      "self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement-proof"
    )));
  }
}

#[test]
fn final_fixture_flags_keep_measurement_scoped() {
  let run = eval_fixture();
  assert!(as_bool(get(
    run,
    "self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement-proof"
  )));
  assert!(as_bool(get(run, "widened-callsite-measured")));
  assert!(as_bool(get(run, "selected-baseline-measurement-inherited")));
  assert!(as_bool(get(
    run,
    "new-callsite-warm-repeats-within-threshold"
  )));
  assert!(as_bool(get(run, "widened-cold-start-slow-path-candidate")));
  assert!(!as_bool(get(run, "persistent-warm-slow-path")));
  assert!(!as_bool(get(run, "global-default-callsite-replaced")));
  assert!(!as_bool(get(run, "global-speedup-claimed")));
  assert!(!as_bool(get(run, "cold-start-solved")));
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
