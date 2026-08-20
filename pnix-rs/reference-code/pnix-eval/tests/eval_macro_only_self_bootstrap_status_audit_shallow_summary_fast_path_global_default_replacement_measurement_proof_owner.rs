use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/pnix-query-runtime/macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-measurement-proof-owner.px",
  )
}

fn eval_fixture() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = fixture_path();
    let json = std::thread::Builder::new()
      .name("bootstrap-shallow-summary-fast-path-global-default-measurement-owner-eval".to_string())
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("global default replacement measurement owner fixture")
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

#[test]
fn fixture_imports_global_default_measurement_owner_and_application_source() {
  let run = eval_fixture();
  assert_eq!(
    as_str(get(run, "proof")),
    "macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-measurement-proof-owner"
  );
  assert!(as_bool(get(run, "imported-owner")));
  assert!(as_bool(get(run, "imported-application-owner")));
  assert!(as_bool(get(run, "imported-application-fixture")));
  assert!(as_bool(get(run, "used-px-owner")));
  assert_eq!(
    as_str(get(run, "expected-source-proof")),
    "proof.macro-only.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-application.v1"
  );
}

#[test]
fn owner_meta_records_post_application_measurement_without_speedup_claim() {
  let run = eval_fixture();
  let meta = get(run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.lib.gate.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-measurement-proof"
  );
  assert!(as_bool(get(
    meta,
    "global-default-replacement-measurement-proof"
  )));
  assert!(as_bool(get(meta, "post-application-measured")));
  assert!(as_bool(get(
    meta,
    "post-application-warm-repeats-within-threshold"
  )));
  assert!(as_bool(get(
    meta,
    "post-application-cold-start-slow-path-candidate"
  )));
  assert!(as_bool(get(meta, "global-speedup-boundary-proof-required")));
  assert_eq!(
    as_i64(get(meta, "pre-application-measurement-record-count")),
    8
  );
  assert_eq!(
    as_i64(get(meta, "post-application-measurement-record-count")),
    3
  );
  assert_eq!(as_i64(get(meta, "combined-measurement-record-count")), 11);
  assert_eq!(
    as_str(get(meta, "performance-envelope")),
    "global-default-replacement-post-application-cold-start-slow-warm-repeats-within-threshold"
  );

  for key in [
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
fn measurement_records_preserve_actual_post_application_p_puck_samples() {
  let run = eval_fixture();
  let records = as_list(get(run, "expected-post-application-measurement-records"));
  assert_eq!(records.len(), 3);

  let cold = &records[0];
  assert_eq!(
    as_str(get(cold, "id")),
    "measurement.1.global-default-replacement.application-status.cold-start"
  );
  assert_eq!(as_i64(get(cold, "duration-ms")), 10544);
  assert_eq!(as_str(get(cold, "status")), "slow-path-candidate");
  assert!(!as_bool(get(cold, "within-threshold")));

  let warm_one = &records[1];
  assert_eq!(as_i64(get(warm_one, "duration-ms")), 312);
  assert_eq!(as_str(get(warm_one, "status")), "within-threshold");
  assert!(as_bool(get(warm_one, "within-threshold")));

  let warm_two = &records[2];
  assert_eq!(as_i64(get(warm_two, "duration-ms")), 266);
  assert_eq!(as_str(get(warm_two, "status")), "within-threshold");
  assert!(as_bool(get(warm_two, "within-threshold")));

  for record in records {
    assert_eq!(
      as_str(get(record, "output-token")),
      "self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-application-proof-present"
    );
    assert!(as_bool(get(record, "application-proof-present")));
    assert!(as_bool(get(record, "bounded-global-default-replacement")));
    assert!(as_bool(get(record, "global-default-replacement-applied")));
    assert!(as_bool(get(record, "global-default-callsite-replaced")));
    assert!(!as_bool(get(record, "global-speedup-claimed")));
    assert!(!as_bool(get(record, "cold-start-solved")));
  }
}

#[test]
fn valid_proof_closes_measurement_and_opens_speedup_boundary_frontier() {
  let run = eval_fixture();
  let valid = get(run, "valid-proof");
  assert_eq!(
    as_str(get(valid, "status")),
    "self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-measurement-proof-present"
  );
  assert!(get(valid, "held-id").is_null());
  assert!(as_bool(get(
    valid,
    "global-default-replacement-measurement-proof"
  )));
  assert!(as_bool(get(valid, "post-application-measured")));
  assert!(as_bool(get(
    valid,
    "global-speedup-boundary-proof-required"
  )));
  assert!(!as_bool(get(valid, "global-speedup-claimed")));
  assert!(!as_bool(get(valid, "cold-start-solved")));

  let closed = string_set(get(valid, "closes"));
  assert!(closed.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-measurement-proof"
  ));
  let open = string_set(get(valid, "next-open-frontiers"));
  assert!(open.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-speedup-boundary-proof"
  ));
}

#[test]
fn held_failures_cover_measurement_inputs_boundaries_and_overclaims() {
  let run = eval_fixture();
  for (field, held_id) in [
    (
      "wrong-proof",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-measurement.proof-id-mismatch",
    ),
    (
      "stale-stage",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-measurement.stale-current-stage",
    ),
    (
      "source-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-measurement.source-mismatch",
    ),
    (
      "application-evidence-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-measurement.application-evidence-missing",
    ),
    (
      "callsite-set-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-measurement.callsite-set-mismatch",
    ),
    (
      "pre-application-envelope-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-measurement.pre-application-envelope-missing",
    ),
    (
      "record-shape-invalid",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-measurement.record-shape-invalid",
    ),
    (
      "sample-values-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-measurement.sample-values-mismatch",
    ),
    (
      "summary-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-measurement.summary-mismatch",
    ),
    (
      "envelope-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-measurement.envelope-mismatch",
    ),
    (
      "audit-fallback-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-measurement.audit-fallback-missing",
    ),
    (
      "negative-held-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-measurement.negative-held-boundary-missing",
    ),
    (
      "missing-evidence",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-measurement.missing-required-evidence",
    ),
    (
      "frontier-shape-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-measurement.frontier-shape-mismatch",
    ),
    (
      "measurement-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-measurement.measurement-overclaim",
    ),
    (
      "runtime-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-measurement.runtime-overclaim",
    ),
    (
      "external-or-license-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-measurement.external-or-license-overclaim",
    ),
    (
      "authority-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-measurement.authority-overclaim",
    ),
  ] {
    let value = get(run, field);
    assert_eq!(as_str(get(value, "status")), "Held", "{field}");
    assert_eq!(as_str(get(value, "held-id")), held_id, "{field}");
  }
}

#[test]
fn measurement_summary_keeps_speedup_cold_start_runtime_and_authority_false() {
  let run = eval_fixture();
  let summary = get(run, "expected-measurement-summary");
  assert_eq!(
    as_str(get(summary, "id")),
    "measurement.fast-path.bootstrap-status-audit.shallow-summary.global-default-replacement.post-application.v1"
  );
  assert_eq!(
    as_str(get(summary, "measurement-scope")),
    "bootstrap-status-audit-shallow-summary-global-default-application-status-query"
  );
  assert_eq!(
    as_i64(get(summary, "combined-measurement-record-count")),
    11
  );
  assert_eq!(
    as_i64(get(summary, "post-application-warm-min-duration-ms")),
    266
  );
  assert_eq!(
    as_i64(get(summary, "post-application-warm-max-duration-ms")),
    312
  );
  assert!(as_bool(get(
    summary,
    "post-application-warm-repeats-within-threshold"
  )));
  assert!(as_bool(get(
    summary,
    "global-speedup-boundary-proof-required"
  )));

  for key in [
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
    assert!(!as_bool(get(summary, key)), "`{key}` must stay false");
  }
}
