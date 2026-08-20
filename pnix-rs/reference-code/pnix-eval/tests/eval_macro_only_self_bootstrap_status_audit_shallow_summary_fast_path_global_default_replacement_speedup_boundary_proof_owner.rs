use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/pnix-query-runtime/macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-speedup-boundary-proof-owner.px",
  )
}

fn eval_fixture() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = fixture_path();
    let json = std::thread::Builder::new()
      .name(
        "bootstrap-shallow-summary-fast-path-global-default-speedup-boundary-owner-eval"
          .to_string(),
      )
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("global default replacement speedup boundary owner fixture")
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
fn fixture_imports_speedup_boundary_owner_and_measurement_source() {
  let run = eval_fixture();
  assert_eq!(
    as_str(get(run, "proof")),
    "macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-speedup-boundary-proof-owner"
  );
  assert!(as_bool(get(run, "imported-owner")));
  assert!(as_bool(get(run, "imported-measurement-owner")));
  assert!(as_bool(get(run, "imported-measurement-fixture")));
  assert!(as_bool(get(run, "used-px-owner")));
  assert_eq!(
    as_str(get(run, "expected-source-proof")),
    "proof.macro-only.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-measurement.v1"
  );
}

#[test]
fn owner_meta_accepts_bounded_warm_signal_without_global_speedup_claim() {
  let run = eval_fixture();
  let meta = get(run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.lib.gate.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-speedup-boundary-proof"
  );
  assert!(as_bool(get(
    meta,
    "global-default-replacement-speedup-boundary-proof"
  )));
  assert!(as_bool(get(meta, "bounded-warm-envelope-accepted")));
  assert!(as_bool(get(meta, "bounded-status-query-fast-path-signal")));
  assert!(as_bool(get(meta, "local-fast-path-signal")));
  assert!(as_bool(get(meta, "comparable-benchmark-required")));
  assert!(!as_bool(get(meta, "comparable-benchmark-present")));
  assert!(!as_bool(get(meta, "global-speedup-claimed")));
  assert!(!as_bool(get(meta, "cold-start-solved")));
  assert_eq!(
    as_i64(get(meta, "post-application-warm-min-duration-ms")),
    266
  );
  assert_eq!(
    as_i64(get(meta, "post-application-warm-max-duration-ms")),
    312
  );
  assert_eq!(
    as_str(get(meta, "speedup-boundary-verdict")),
    "bounded-warm-envelope-accepted-global-speedup-held-comparable-benchmark-required"
  );
}

#[test]
fn valid_proof_closes_boundary_and_opens_comparable_benchmark_frontier() {
  let run = eval_fixture();
  let valid = get(run, "valid-proof");
  assert_eq!(
    as_str(get(valid, "status")),
    "self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-speedup-boundary-proof-present"
  );
  assert!(get(valid, "held-id").is_null());
  assert!(as_bool(get(
    valid,
    "global-default-replacement-speedup-boundary-proof"
  )));
  assert!(as_bool(get(valid, "bounded-warm-envelope-accepted")));
  assert!(as_bool(get(valid, "comparable-benchmark-required")));
  assert!(!as_bool(get(valid, "global-speedup-claimed")));

  let closed = string_set(get(valid, "closes"));
  assert!(closed.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-speedup-boundary-proof"
  ));
  let open = string_set(get(valid, "next-open-frontiers"));
  assert!(open.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-comparable-benchmark-proof"
  ));
}

#[test]
fn held_failures_cover_boundary_inputs_comparability_and_overclaims() {
  let run = eval_fixture();
  for (field, held_id) in [
    (
      "wrong-proof",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-speedup-boundary.proof-id-mismatch",
    ),
    (
      "stale-stage",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-speedup-boundary.stale-current-stage",
    ),
    (
      "source-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-speedup-boundary.source-mismatch",
    ),
    (
      "measurement-evidence-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-speedup-boundary.measurement-evidence-missing",
    ),
    (
      "record-count-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-speedup-boundary.record-count-mismatch",
    ),
    (
      "measurement-envelope-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-speedup-boundary.measurement-envelope-mismatch",
    ),
    (
      "summary-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-speedup-boundary.summary-mismatch",
    ),
    (
      "comparability-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-speedup-boundary.comparability-mismatch",
    ),
    (
      "audit-fallback-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-speedup-boundary.audit-fallback-missing",
    ),
    (
      "missing-evidence",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-speedup-boundary.missing-required-evidence",
    ),
    (
      "frontier-shape-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-speedup-boundary.frontier-shape-mismatch",
    ),
    (
      "speedup-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-speedup-boundary.speedup-overclaim",
    ),
    (
      "runtime-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-speedup-boundary.runtime-overclaim",
    ),
    (
      "external-or-license-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-speedup-boundary.external-or-license-overclaim",
    ),
    (
      "authority-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-speedup-boundary.authority-overclaim",
    ),
  ] {
    let value = get(run, field);
    assert_eq!(as_str(get(value, "status")), "Held", "{field}");
    assert_eq!(as_str(get(value, "held-id")), held_id, "{field}");
  }
}

#[test]
fn boundary_summary_keeps_runtime_external_and_authority_false() {
  let run = eval_fixture();
  let summary = get(run, "boundary-summary");
  assert_eq!(
    as_str(get(summary, "speedup-boundary-verdict")),
    "bounded-warm-envelope-accepted-global-speedup-held-comparable-benchmark-required"
  );
  assert!(as_bool(get(summary, "bounded-warm-envelope-accepted")));
  assert!(as_bool(get(
    summary,
    "bounded-status-query-fast-path-signal"
  )));
  assert!(as_bool(get(summary, "comparable-benchmark-required")));
  assert!(!as_bool(get(
    summary,
    "apples-to-apples-global-speedup-comparison"
  )));
  assert_eq!(
    as_str(get(summary, "global-speedup-comparison-status")),
    "Held"
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
    assert!(!as_bool(get(summary, key)), "`{key}` must stay false");
  }
}
