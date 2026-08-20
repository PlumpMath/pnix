use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/pnix-query-runtime/macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-boundary-proof-owner.px",
  )
}

fn eval_fixture() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = fixture_path();
    let json = std::thread::Builder::new()
      .name(
        "bootstrap-shallow-summary-fast-path-global-default-cold-start-boundary-owner-eval"
          .to_string(),
      )
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("global default replacement cold-start boundary owner fixture")
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
fn fixture_imports_cold_start_boundary_owner_and_benchmark_source() {
  let run = eval_fixture();
  assert_eq!(
    as_str(get(run, "proof")),
    "macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-boundary-proof-owner"
  );
  assert!(as_bool(get(run, "imported-owner")));
  assert!(as_bool(get(run, "imported-benchmark-owner")));
  assert!(as_bool(get(run, "imported-benchmark-fixture")));
  assert!(as_bool(get(run, "used-px-owner")));
  assert_eq!(
    as_str(get(run, "expected-source-proof")),
    "proof.macro-only.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-comparable-benchmark.v1"
  );
}

#[test]
fn owner_meta_separates_cold_envelope_from_warm_envelope() {
  let run = eval_fixture();
  let meta = get(run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.lib.gate.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-boundary-proof"
  );
  assert!(as_bool(get(
    meta,
    "global-default-replacement-cold-start-boundary-proof"
  )));
  assert!(as_bool(get(meta, "cold-warm-envelopes-separated")));
  assert!(as_bool(get(meta, "cold-warm-gap-positive")));
  assert!(as_bool(get(
    meta,
    "cold-start-attribution-frontier-required"
  )));
  assert_eq!(as_i64(get(meta, "warm-envelope-min-duration-ms")), 266);
  assert_eq!(as_i64(get(meta, "warm-envelope-max-duration-ms")), 312);
  assert_eq!(as_i64(get(meta, "cold-envelope-min-duration-ms")), 9597);
  assert_eq!(as_i64(get(meta, "cold-envelope-max-duration-ms")), 10544);
  assert_eq!(as_i64(get(meta, "cold-warm-gap-min-ms")), 9285);
  assert_eq!(as_i64(get(meta, "cold-warm-gap-max-ms")), 10278);
  assert_eq!(as_i64(get(meta, "slow-threshold-ms")), 5000);
  assert_eq!(
    as_str(get(meta, "cold-warm-comparison-status")),
    "cold-warm-envelopes-separated-cold-start-solution-held"
  );
  assert!(!as_bool(get(meta, "cold-start-solved")));
  assert!(!as_bool(get(meta, "cold-start-eliminated")));
  assert!(!as_bool(get(meta, "cold-start-attributed")));
  assert!(!as_bool(get(meta, "global-speedup-claimed")));
  assert!(!as_bool(get(meta, "whole-system-speedup-claimed")));
}

#[test]
fn valid_proof_closes_cold_start_boundary_and_opens_cold_start_attribution() {
  let run = eval_fixture();
  let valid = get(run, "valid-proof");
  assert_eq!(
    as_str(get(valid, "status")),
    "self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-boundary-proof-present"
  );
  assert!(get(valid, "held-id").is_null());
  assert!(as_bool(get(
    valid,
    "global-default-replacement-cold-start-boundary-proof"
  )));
  assert!(as_bool(get(valid, "cold-warm-envelopes-separated")));
  assert!(as_bool(get(valid, "cold-warm-gap-positive")));
  assert!(!as_bool(get(valid, "cold-start-solved")));

  let closed = string_set(get(valid, "closes"));
  assert!(closed.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-boundary-proof"
  ));
  let open = string_set(get(valid, "next-open-frontiers"));
  assert!(open.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution-proof"
  ));
}

#[test]
fn boundary_summary_records_cold_and_warm_envelopes_and_positive_gap() {
  let run = eval_fixture();
  let summary = get(run, "boundary-summary");
  assert_eq!(
    as_str(get(summary, "id")),
    "boundary.fast-path.bootstrap-status-audit.shallow-summary.global-default-replacement.cold-start.v1"
  );
  assert_eq!(
    as_str(get(summary, "boundary-scope")),
    "bootstrap-status-audit-shallow-summary-status-query-family"
  );
  assert_eq!(as_i64(get(summary, "warm-envelope-min-duration-ms")), 266);
  assert_eq!(as_i64(get(summary, "warm-envelope-max-duration-ms")), 312);
  assert_eq!(as_i64(get(summary, "cold-envelope-min-duration-ms")), 9597);
  assert_eq!(as_i64(get(summary, "cold-envelope-max-duration-ms")), 10544);
  assert_eq!(as_i64(get(summary, "cold-warm-gap-min-ms")), 9285);
  assert_eq!(as_i64(get(summary, "cold-warm-gap-max-ms")), 10278);
  assert_eq!(as_i64(get(summary, "slow-threshold-ms")), 5000);
  assert_eq!(as_i64(get(summary, "cold-record-count")), 2);
  assert!(as_bool(get(summary, "cold-warm-envelopes-separated")));
  assert!(as_bool(get(summary, "cold-warm-gap-positive")));
  assert!(as_bool(get(
    summary,
    "cold-start-attribution-frontier-required"
  )));
  assert_eq!(
    as_str(get(summary, "boundary-verdict")),
    "cold-warm-separated-cold-start-attribution-required"
  );
  assert!(!as_bool(get(summary, "cold-start-solved")));
  assert!(!as_bool(get(summary, "cold-start-eliminated")));
  assert!(!as_bool(get(summary, "cold-start-attributed")));
}

#[test]
fn held_failures_cover_inputs_envelopes_separation_frontier_and_overclaims() {
  let run = eval_fixture();
  for (field, held_id) in [
    (
      "wrong-proof",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-boundary.proof-id-mismatch",
    ),
    (
      "stale-stage",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-boundary.stale-current-stage",
    ),
    (
      "source-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-boundary.source-mismatch",
    ),
    (
      "benchmark-source-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-boundary.benchmark-source-missing",
    ),
    (
      "warm-envelope-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-boundary.warm-envelope-mismatch",
    ),
    (
      "cold-envelope-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-boundary.cold-envelope-mismatch",
    ),
    (
      "cold-record-shape-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-boundary.cold-record-shape-mismatch",
    ),
    (
      "cold-record-invalid",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-boundary.cold-record-invalid",
    ),
    (
      "separation-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-boundary.separation-mismatch",
    ),
    (
      "scope-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-boundary.scope-mismatch",
    ),
    (
      "summary-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-boundary.summary-mismatch",
    ),
    (
      "attribution-frontier-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-boundary.attribution-frontier-missing",
    ),
    (
      "audit-fallback-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-boundary.audit-fallback-missing",
    ),
    (
      "missing-evidence",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-boundary.missing-required-evidence",
    ),
    (
      "frontier-shape-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-boundary.frontier-shape-mismatch",
    ),
    (
      "cold-start-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-boundary.cold-start-overclaim",
    ),
    (
      "speedup-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-boundary.speedup-overclaim",
    ),
    (
      "runtime-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-boundary.runtime-overclaim",
    ),
    (
      "external-or-license-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-boundary.external-or-license-overclaim",
    ),
    (
      "authority-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-boundary.authority-overclaim",
    ),
  ] {
    let value = get(run, field);
    assert_eq!(as_str(get(value, "status")), "Held", "{field}");
    assert_eq!(as_str(get(value, "held-id")), held_id, "{field}");
  }
}

#[test]
fn hard_stops_remain_false_after_cold_start_boundary_proof() {
  let run = eval_fixture();
  for key in [
    "cold-start-solved",
    "cold-start-eliminated",
    "cold-start-attributed",
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
