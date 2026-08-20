use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/pnix-query-runtime/macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-application-proof-owner.px",
  )
}

fn eval_fixture() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = fixture_path();
    let json = std::thread::Builder::new()
      .name("bootstrap-shallow-summary-fast-path-global-default-application-owner-eval".to_string())
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("global default replacement application owner fixture")
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
fn fixture_imports_global_default_application_owner_and_readiness_source() {
  let run = eval_fixture();
  assert_eq!(
    as_str(get(run, "proof")),
    "macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-application-proof-owner"
  );
  assert!(as_bool(get(run, "imported-owner")));
  assert!(as_bool(get(run, "imported-readiness-owner")));
  assert!(as_bool(get(run, "imported-readiness-fixture")));
  assert!(as_bool(get(run, "used-px-owner")));
  assert_eq!(
    as_str(get(run, "expected-source-proof")),
    "proof.macro-only.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-readiness.v1"
  );
}

#[test]
fn owner_meta_records_bounded_default_replacement_without_speedup_or_runtime() {
  let run = eval_fixture();
  let meta = get(run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.lib.gate.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-application-proof"
  );
  assert!(as_bool(get(
    meta,
    "global-default-replacement-application-proof"
  )));
  assert!(as_bool(get(meta, "global-default-replacement-applied")));
  assert!(as_bool(get(meta, "bounded-global-default-replacement")));
  assert!(as_bool(get(meta, "global-default-callsite-replaced")));
  assert_eq!(as_i64(get(meta, "known-default-callsite-count")), 3);
  assert_eq!(as_i64(get(meta, "replaced-default-callsite-count")), 3);
  assert_eq!(
    as_i64(get(meta, "unmeasured-callsite-replacement-count")),
    0
  );
  assert!(as_bool(get(meta, "post-application-measurement-required")));

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
fn replacement_record_pins_exact_known_default_callsite_set() {
  let run = eval_fixture();
  let record = get(run, "expected-replacement-record");
  assert_eq!(
    as_str(get(record, "id")),
    "application.fast-path.bootstrap-status-audit.shallow-summary.global-default-replacement.v1"
  );
  assert_eq!(
    as_str(get(record, "replacement-verdict")),
    "bounded-global-default-replacement-applied"
  );
  assert!(as_bool(get(record, "global-default-callsite-replaced")));
  assert!(as_bool(get(record, "global-default-replacement-applied")));
  assert!(as_bool(get(record, "bounded-global-default-replacement")));
  assert!(!as_bool(get(record, "global-speedup-claimed")));
  assert!(!as_bool(get(record, "cold-start-solved")));

  let known = string_set(get(record, "known-default-callsite-ids"));
  let replaced = string_set(get(record, "replaced-default-callsite-ids"));
  for id in [
    "callsite.bootstrap-status-audit.current-status.shallow-summary.v1",
    "callsite.bootstrap-status-audit.operator-panel.shallow-summary.v1",
    "callsite.bootstrap-status-audit.index-status.shallow-summary.v1",
  ] {
    assert!(known.contains(id));
    assert!(replaced.contains(id));
  }
  assert_eq!(as_i64(get(record, "known-default-callsite-count")), 3);
  assert_eq!(as_i64(get(record, "replaced-default-callsite-count")), 3);
  assert_eq!(
    as_i64(get(record, "unmeasured-callsite-replacement-count")),
    0
  );
}

#[test]
fn valid_proof_closes_application_and_opens_measurement_frontier() {
  let run = eval_fixture();
  let valid = get(run, "valid-proof");
  assert_eq!(
    as_str(get(valid, "status")),
    "self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-application-proof-present"
  );
  assert!(get(valid, "held-id").is_null());
  assert!(as_bool(get(
    valid,
    "global-default-replacement-application-proof"
  )));
  assert!(as_bool(get(valid, "global-default-replacement-applied")));
  assert!(as_bool(get(valid, "global-default-callsite-replaced")));
  assert!(as_bool(get(valid, "post-application-measurement-required")));
  assert!(!as_bool(get(valid, "global-speedup-claimed")));
  assert!(!as_bool(get(valid, "cold-start-solved")));

  let closed = string_set(get(valid, "closes"));
  assert!(closed.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-application-proof"
  ));
  let open = string_set(get(valid, "next-open-frontiers"));
  assert!(open.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-measurement-proof"
  ));
}

#[test]
fn held_failures_cover_application_inputs_boundaries_and_overclaims() {
  let run = eval_fixture();
  for (field, held_id) in [
    (
      "wrong-proof",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-application.proof-id-mismatch",
    ),
    (
      "stale-stage",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-application.stale-current-stage",
    ),
    (
      "source-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-application.source-mismatch",
    ),
    (
      "readiness-evidence-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-application.readiness-evidence-missing",
    ),
    (
      "replacement-record-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-application.replacement-record-mismatch",
    ),
    (
      "callsite-coverage-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-application.callsite-coverage-mismatch",
    ),
    (
      "field-shape-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-application.field-shape-mismatch",
    ),
    (
      "audit-fallback-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-application.audit-fallback-missing",
    ),
    (
      "negative-held-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-application.negative-held-boundary-missing",
    ),
    (
      "missing-evidence",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-application.missing-required-evidence",
    ),
    (
      "frontier-shape-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-application.frontier-shape-mismatch",
    ),
    (
      "replacement-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-application.replacement-overclaim",
    ),
    (
      "runtime-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-application.runtime-overclaim",
    ),
    (
      "external-or-license-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-application.external-or-license-overclaim",
    ),
    (
      "authority-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-application.authority-overclaim",
    ),
  ] {
    let value = get(run, field);
    assert_eq!(as_str(get(value, "status")), "Held", "{field}");
    assert_eq!(as_str(get(value, "held-id")), held_id, "{field}");
    assert!(!as_bool(get(value, "global-default-callsite-replaced")));
    assert!(!as_bool(get(value, "global-speedup-claimed")));
    assert!(!as_bool(get(value, "cold-start-solved")));
  }
}

#[test]
fn final_fixture_flags_keep_application_bounded() {
  let run = eval_fixture();
  assert!(as_bool(get(
    run,
    "global-default-replacement-application-proof"
  )));
  assert!(as_bool(get(run, "global-default-replacement-applied")));
  assert!(as_bool(get(run, "bounded-global-default-replacement")));
  assert!(as_bool(get(run, "global-default-callsite-replaced")));
  assert_eq!(as_i64(get(run, "known-default-callsite-count")), 3);
  assert_eq!(as_i64(get(run, "replaced-default-callsite-count")), 3);
  assert_eq!(as_i64(get(run, "unmeasured-callsite-replacement-count")), 0);
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
