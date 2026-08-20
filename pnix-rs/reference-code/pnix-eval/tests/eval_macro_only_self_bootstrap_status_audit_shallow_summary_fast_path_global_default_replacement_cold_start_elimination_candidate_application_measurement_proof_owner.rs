use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn combined_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../stdlib/lib/gate/macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application-measurement-proof.px",
  )
}

fn eval_combined() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = combined_path();
    let json = std::thread::Builder::new()
      .name("cold-start-application-measurement-combined-eval".to_string())
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("cold-start application measurement combined eval")
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
fn fixture_imports_measurement_owner_and_application_source() {
  let run = fixture();
  assert!(as_bool(get(run, "imported-owner")));
  assert!(as_bool(get(run, "imported-application-owner")));
  assert!(as_bool(get(run, "used-px-owner")));
  assert_eq!(
    as_str(get(run, "expected-source-proof")),
    "proof.macro-only.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application.v1"
  );
}

#[test]
fn owner_meta_records_cold_delta_keeps_runtime_wired_false() {
  let run = fixture();
  let meta = get(run, "owner-meta");
  assert!(as_bool(get(
    meta,
    "global-default-replacement-cold-start-elimination-candidate-application-measurement-proof"
  )));
  assert_eq!(as_i64(get(meta, "pre-wrapper-cold-duration-ms")), 12457);
  assert_eq!(as_i64(get(meta, "post-direct-cold-duration-ms")), 2983);
  assert_eq!(as_i64(get(meta, "post-direct-warm-min-duration-ms")), 3024);
  assert_eq!(as_i64(get(meta, "post-direct-warm-max-duration-ms")), 3025);
  assert_eq!(as_i64(get(meta, "cold-delta-ms")), 9474);
  assert!(as_bool(get(
    meta,
    "bounded-status-query-cold-improvement-recorded"
  )));
  assert!(as_bool(get(
    meta,
    "application-speedup-boundary-frontier-required"
  )));
  assert!(as_bool(get(meta, "runtime-wiring-required")));
  assert!(!as_bool(get(meta, "runtime-wired")));
  assert!(!as_bool(get(meta, "cold-start-solved")));
  assert!(!as_bool(get(meta, "cold-start-eliminated")));
  assert!(!as_bool(get(meta, "cold-start-globally-bypassed")));
  assert!(!as_bool(get(meta, "elimination-applied-globally")));
  assert!(!as_bool(get(meta, "global-speedup-claimed")));
}

#[test]
fn valid_proof_closes_application_measurement_and_opens_speedup_boundary() {
  let run = fixture();
  let valid = get(run, "valid-proof");
  assert_eq!(
    as_str(get(valid, "status")),
    "self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application-measurement-proof-present"
  );
  assert!(get(valid, "held-id").is_null());
  assert_eq!(as_i64(get(valid, "cold-delta-ms")), 9474);
  assert_eq!(as_i64(get(valid, "measurement-record-count")), 4);
  assert!(!as_bool(get(valid, "runtime-wired")));

  let closed = string_set(get(valid, "closes"));
  assert!(closed.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application-measurement-proof"
  ));
  let open = string_set(get(valid, "next-open-frontiers"));
  assert!(open.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application-speedup-boundary-proof"
  ));
}

#[test]
fn measurement_registry_records_pre_post_durations() {
  let run = fixture();
  let reg = get(run, "measurement-registry");
  assert_eq!(
    as_str(get(reg, "id")),
    "elimination-application-measurement.fast-path.bootstrap-status-audit.shallow-summary.global-default-replacement.cold-start.v1"
  );
  assert_eq!(as_i64(get(reg, "measurement-record-count")), 4);
  assert_eq!(as_i64(get(reg, "pre-application-record-count")), 1);
  assert_eq!(as_i64(get(reg, "post-application-record-count")), 3);
  assert_eq!(as_i64(get(reg, "pre-wrapper-cold-duration-ms")), 12457);
  assert_eq!(as_i64(get(reg, "post-direct-cold-duration-ms")), 2983);
  assert_eq!(as_i64(get(reg, "cold-delta-ms")), 9474);
  assert!(as_bool(get(reg, "pre-application-slow-path")));
  assert!(as_bool(get(
    reg,
    "post-application-direct-within-threshold"
  )));
  assert!(as_bool(get(
    reg,
    "apples-to-apples-status-query-comparison"
  )));
  assert!(as_bool(get(reg, "runtime-wiring-required")));
  assert!(!as_bool(get(reg, "runtime-wired")));
  assert!(!as_bool(get(reg, "cold-start-solved")));
  assert!(!as_bool(get(reg, "cold-start-eliminated")));
  assert!(!as_bool(get(reg, "cold-start-globally-bypassed")));
  assert!(!as_bool(get(reg, "elimination-applied-globally")));
}

#[test]
fn held_failures_cover_all_branches() {
  let run = fixture();
  let prefix = "held.macro-only-self...cold-start-elimination-candidate-application-measurement";
  for (field, suffix) in [
    ("wrong-proof", "proof-id-mismatch"),
    ("stale-stage", "stale-current-stage"),
    ("source-mismatch", "source-mismatch"),
    ("application-source-missing", "application-source-missing"),
    (
      "application-input-shape-mismatch",
      "application-input-shape-mismatch",
    ),
    (
      "measurement-record-shape-mismatch",
      "measurement-record-shape-mismatch",
    ),
    ("measurement-record-invalid", "measurement-record-invalid"),
    ("measurement-status-mismatch", "measurement-status-mismatch"),
    ("duration-mismatch", "duration-mismatch"),
    ("delta-mismatch", "delta-mismatch"),
    ("scope-mismatch", "scope-mismatch"),
    ("registry-mismatch", "registry-mismatch"),
    ("boundary-frontier-missing", "boundary-frontier-missing"),
    ("audit-fallback-missing", "audit-fallback-missing"),
    ("missing-evidence", "missing-required-evidence"),
    ("frontier-shape-mismatch", "frontier-shape-mismatch"),
    ("measurement-overclaim", "measurement-overclaim"),
    ("speedup-overclaim", "speedup-overclaim"),
    ("runtime-overclaim", "runtime-overclaim"),
    (
      "external-or-license-overclaim",
      "external-or-license-overclaim",
    ),
    ("authority-overclaim", "authority-overclaim"),
  ] {
    let value = get(run, field);
    assert_eq!(as_str(get(value, "status")), "Held", "{field}");
    assert_eq!(
      as_str(get(value, "held-id")),
      &format!("{}.{}", prefix, suffix),
      "{field}"
    );
  }
}

#[test]
fn hard_stops_remain_false_after_application_measurement_proof() {
  let run = fixture();
  for key in [
    "cold-start-solved",
    "cold-start-eliminated",
    "cold-start-globally-bypassed",
    "elimination-applied-globally",
    "runtime-wired",
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
  assert!(as_bool(get(
    run,
    "bounded-status-query-cold-improvement-recorded"
  )));
  assert!(as_bool(get(run, "runtime-wiring-required")));
}
