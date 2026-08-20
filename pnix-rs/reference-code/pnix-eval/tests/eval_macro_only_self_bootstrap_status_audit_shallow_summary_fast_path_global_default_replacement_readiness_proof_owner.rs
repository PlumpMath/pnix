use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/pnix-query-runtime/macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-readiness-proof-owner.px",
  )
}

fn eval_fixture() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = fixture_path();
    let json = std::thread::Builder::new()
      .name("bootstrap-shallow-summary-fast-path-global-default-readiness-owner-eval".to_string())
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("global default replacement readiness owner fixture")
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
fn fixture_imports_global_default_readiness_owner_and_measurement_source() {
  let run = eval_fixture();
  assert_eq!(
    as_str(get(run, "proof")),
    "macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-readiness-proof-owner"
  );
  assert!(as_bool(get(run, "imported-owner")));
  assert!(as_bool(get(run, "imported-measurement-owner")));
  assert!(as_bool(get(run, "imported-measurement-fixture")));
  assert!(as_bool(get(run, "used-px-owner")));
  assert_eq!(
    as_str(get(run, "expected-source-proof")),
    "proof.macro-only.self.bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement.v1"
  );
}

#[test]
fn owner_meta_records_readiness_without_replacement_claims() {
  let run = eval_fixture();
  let meta = get(run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.lib.gate.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-readiness-proof"
  );
  assert_eq!(
    as_str(get(meta, "constructor")),
    "validateSelfBootstrapStatusAuditShallowSummaryFastPathGlobalDefaultReplacementReadinessProof"
  );
  assert!(as_bool(get(
    meta,
    "global-default-replacement-readiness-proof"
  )));
  assert!(as_bool(get(meta, "global-default-readiness-proven")));
  assert!(as_bool(get(
    meta,
    "global-default-replacement-application-required"
  )));
  assert_eq!(as_i64(get(meta, "known-default-callsite-count")), 3);
  assert_eq!(
    as_i64(get(meta, "measured-known-default-callsite-count")),
    3
  );
  assert_eq!(
    as_i64(get(meta, "unmeasured-known-default-callsite-count")),
    0
  );
  assert_eq!(as_i64(get(meta, "measurement-record-count")), 8);
  assert!(as_bool(get(
    meta,
    "new-callsite-warm-repeats-within-threshold"
  )));
  assert!(as_bool(get(meta, "widened-cold-start-slow-path-candidate")));

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
fn readiness_record_pins_known_default_callsite_coverage_and_boundaries() {
  let run = eval_fixture();
  let record = get(run, "expected-readiness-record");
  assert_eq!(
    as_str(get(record, "id")),
    "readiness.fast-path.bootstrap-status-audit.shallow-summary.global-default-replacement.v1"
  );
  assert_eq!(
    as_str(get(record, "readiness-verdict")),
    "ready-for-bounded-global-default-replacement-application"
  );
  assert_eq!(
    as_str(get(record, "default-replacement-mode")),
    "application-proof-required-before-replacement"
  );
  assert!(as_bool(get(record, "global-default-readiness-proven")));
  assert!(as_bool(get(
    record,
    "global-default-replacement-application-required"
  )));
  assert!(!as_bool(get(record, "global-default-callsite-replaced")));
  assert!(!as_bool(get(record, "global-speedup-claimed")));
  assert!(!as_bool(get(record, "cold-start-solved")));

  let callsites = string_set(get(record, "known-default-callsite-ids"));
  assert!(callsites.contains("callsite.bootstrap-status-audit.current-status.shallow-summary.v1"));
  assert!(callsites.contains("callsite.bootstrap-status-audit.operator-panel.shallow-summary.v1"));
  assert!(callsites.contains("callsite.bootstrap-status-audit.index-status.shallow-summary.v1"));
  assert_eq!(as_i64(get(record, "known-default-callsite-count")), 3);
  assert_eq!(
    as_i64(get(record, "measured-known-default-callsite-count")),
    3
  );
  assert_eq!(
    as_i64(get(record, "unmeasured-known-default-callsite-count")),
    0
  );
  assert_eq!(as_i64(get(record, "measurement-record-count")), 8);

  let held_ids = string_set(get(record, "negative-held-ids"));
  assert!(held_ids
    .contains("held.bootstrap-status-shallow-summary-callsite-widening-policy.full-json-shape"));
  assert!(held_ids
    .contains("held.bootstrap-status-shallow-summary-callsite-widening-policy.not-allowlisted"));
  assert!(held_ids.contains("held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-measurement.measurement-overclaim"));
}

#[test]
fn valid_proof_closes_readiness_and_opens_application_frontier() {
  let run = eval_fixture();
  let valid = get(run, "valid-proof");
  assert_eq!(
    as_str(get(valid, "status")),
    "self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-readiness-proof-present"
  );
  assert!(get(valid, "held-id").is_null());
  assert!(as_bool(get(
    valid,
    "global-default-replacement-readiness-proof"
  )));
  assert!(as_bool(get(valid, "global-default-readiness-proven")));
  assert!(as_bool(get(
    valid,
    "global-default-replacement-application-required"
  )));
  assert_eq!(as_i64(get(valid, "known-default-callsite-count")), 3);
  assert_eq!(
    as_i64(get(valid, "measured-known-default-callsite-count")),
    3
  );
  assert_eq!(
    as_i64(get(valid, "unmeasured-known-default-callsite-count")),
    0
  );
  assert!(!as_bool(get(valid, "global-default-callsite-replaced")));
  assert!(!as_bool(get(valid, "global-speedup-claimed")));
  assert!(!as_bool(get(valid, "cold-start-solved")));

  let closed = string_set(get(valid, "closes"));
  assert!(closed.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-readiness-proof"
  ));
  let open = string_set(get(valid, "next-open-frontiers"));
  assert!(open.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-application-proof"
  ));
}

#[test]
fn held_failures_cover_readiness_inputs_boundaries_and_overclaims() {
  let run = eval_fixture();
  for (field, held_id) in [
    (
      "wrong-proof",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-readiness.proof-id-mismatch",
    ),
    (
      "stale-stage",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-readiness.stale-current-stage",
    ),
    (
      "source-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-readiness.source-mismatch",
    ),
    (
      "measurement-evidence-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-readiness.measurement-evidence-missing",
    ),
    (
      "readiness-record-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-readiness.readiness-record-mismatch",
    ),
    (
      "callsite-coverage-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-readiness.callsite-coverage-mismatch",
    ),
    (
      "measurement-envelope-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-readiness.measurement-envelope-mismatch",
    ),
    (
      "field-shape-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-readiness.field-shape-mismatch",
    ),
    (
      "audit-fallback-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-readiness.audit-fallback-missing",
    ),
    (
      "negative-held-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-readiness.negative-held-boundary-missing",
    ),
    (
      "missing-evidence",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-readiness.missing-required-evidence",
    ),
    (
      "frontier-shape-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-readiness.frontier-shape-mismatch",
    ),
    (
      "replacement-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-readiness.replacement-overclaim",
    ),
    (
      "runtime-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-readiness.runtime-overclaim",
    ),
    (
      "external-or-license-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-readiness.external-or-license-overclaim",
    ),
    (
      "authority-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-readiness.authority-overclaim",
    ),
  ] {
    let value = get(run, field);
    assert_eq!(as_str(get(value, "status")), "Held", "{field}");
    assert_eq!(as_str(get(value, "held-id")), held_id, "{field}");
    assert!(!as_bool(get(value, "global-default-readiness-proven")));
    assert!(!as_bool(get(value, "global-default-callsite-replaced")));
    assert!(!as_bool(get(value, "global-speedup-claimed")));
  }
}

#[test]
fn final_fixture_flags_keep_readiness_scoped() {
  let run = eval_fixture();
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
