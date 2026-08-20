use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/pnix-query-runtime/macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-comparable-benchmark-proof-owner.px",
  )
}

fn eval_fixture() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = fixture_path();
    let json = std::thread::Builder::new()
      .name("bootstrap-shallow-summary-fast-path-global-default-comparable-owner-eval".to_string())
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("global default replacement comparable benchmark owner fixture")
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
fn fixture_imports_comparable_benchmark_owner_and_speedup_source() {
  let run = eval_fixture();
  assert_eq!(
    as_str(get(run, "proof")),
    "macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-comparable-benchmark-proof-owner"
  );
  assert!(as_bool(get(run, "imported-owner")));
  assert!(as_bool(get(run, "imported-speedup-owner")));
  assert!(as_bool(get(run, "imported-speedup-fixture")));
  assert!(as_bool(get(run, "used-px-owner")));
  assert_eq!(
    as_str(get(run, "expected-source-proof")),
    "proof.macro-only.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-speedup-boundary.v1"
  );
}

#[test]
fn owner_meta_proves_bounded_status_query_speedup_without_global_speedup() {
  let run = eval_fixture();
  let meta = get(run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.lib.gate.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-comparable-benchmark-proof"
  );
  assert!(as_bool(get(
    meta,
    "global-default-replacement-comparable-benchmark-proof"
  )));
  assert!(as_bool(get(meta, "comparable-benchmark-present")));
  assert!(as_bool(get(
    meta,
    "apples-to-apples-status-query-comparison"
  )));
  assert!(!as_bool(get(
    meta,
    "apples-to-apples-global-speedup-comparison"
  )));
  assert!(as_bool(get(meta, "bounded-status-query-speedup-proven")));
  assert!(as_bool(get(
    meta,
    "local-global-default-replacement-warm-speedup"
  )));
  assert_eq!(as_i64(get(meta, "baseline-warm-min-duration-ms")), 763);
  assert_eq!(as_i64(get(meta, "baseline-warm-max-duration-ms")), 827);
  assert_eq!(as_i64(get(meta, "candidate-warm-min-duration-ms")), 266);
  assert_eq!(as_i64(get(meta, "candidate-warm-max-duration-ms")), 312);
  assert_eq!(as_i64(get(meta, "warm-max-improvement-ms")), 515);
  assert_eq!(
    as_str(get(meta, "global-speedup-comparison-status")),
    "bounded-local-status-query-speedup-proven-global-speedup-held"
  );
  assert!(!as_bool(get(meta, "global-speedup-claimed")));
  assert!(!as_bool(get(meta, "whole-system-speedup-claimed")));
  assert!(!as_bool(get(meta, "cold-start-solved")));
}

#[test]
fn valid_proof_closes_comparable_benchmark_and_opens_cold_start_boundary() {
  let run = eval_fixture();
  let valid = get(run, "valid-proof");
  assert_eq!(
    as_str(get(valid, "status")),
    "self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-comparable-benchmark-proof-present"
  );
  assert!(get(valid, "held-id").is_null());
  assert!(as_bool(get(
    valid,
    "global-default-replacement-comparable-benchmark-proof"
  )));
  assert!(as_bool(get(valid, "bounded-status-query-speedup-proven")));
  assert!(!as_bool(get(valid, "global-speedup-claimed")));

  let closed = string_set(get(valid, "closes"));
  assert!(closed.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-comparable-benchmark-proof"
  ));
  let open = string_set(get(valid, "next-open-frontiers"));
  assert!(open.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-boundary-proof"
  ));
}

#[test]
fn benchmark_summary_records_pre_and_post_warm_envelopes() {
  let run = eval_fixture();
  let summary = get(run, "benchmark-summary");
  assert_eq!(
    as_str(get(summary, "id")),
    "benchmark.fast-path.bootstrap-status-audit.shallow-summary.global-default-replacement.comparable.v1"
  );
  assert_eq!(
    as_str(get(summary, "benchmark-scope")),
    "bootstrap-status-audit-shallow-summary-status-query-family"
  );
  assert_eq!(as_i64(get(summary, "baseline-warm-min-duration-ms")), 763);
  assert_eq!(as_i64(get(summary, "baseline-warm-max-duration-ms")), 827);
  assert_eq!(as_i64(get(summary, "candidate-warm-min-duration-ms")), 266);
  assert_eq!(as_i64(get(summary, "candidate-warm-max-duration-ms")), 312);
  assert_eq!(as_i64(get(summary, "warm-min-improvement-ms")), 497);
  assert_eq!(as_i64(get(summary, "warm-max-improvement-ms")), 515);
  assert_eq!(as_str(get(summary, "warm-max-speedup-ratio")), "2.65x");
  assert!(as_bool(get(
    summary,
    "apples-to-apples-status-query-comparison"
  )));
  assert!(!as_bool(get(
    summary,
    "apples-to-apples-global-speedup-comparison"
  )));
  assert_eq!(
    as_str(get(summary, "benchmark-verdict")),
    "bounded-status-query-speedup-proven-whole-system-speedup-held"
  );
}

#[test]
fn held_failures_cover_inputs_comparability_frontier_and_overclaims() {
  let run = eval_fixture();
  for (field, held_id) in [
    (
      "wrong-proof",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-comparable-benchmark.proof-id-mismatch",
    ),
    (
      "stale-stage",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-comparable-benchmark.stale-current-stage",
    ),
    (
      "source-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-comparable-benchmark.source-mismatch",
    ),
    (
      "speedup-boundary-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-comparable-benchmark.speedup-boundary-missing",
    ),
    (
      "baseline-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-comparable-benchmark.baseline-missing",
    ),
    (
      "value-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-comparable-benchmark.value-mismatch",
    ),
    (
      "summary-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-comparable-benchmark.summary-mismatch",
    ),
    (
      "comparability-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-comparable-benchmark.comparability-mismatch",
    ),
    (
      "audit-fallback-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-comparable-benchmark.audit-fallback-missing",
    ),
    (
      "missing-evidence",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-comparable-benchmark.missing-required-evidence",
    ),
    (
      "frontier-shape-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-comparable-benchmark.frontier-shape-mismatch",
    ),
    (
      "speedup-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-comparable-benchmark.speedup-overclaim",
    ),
    (
      "runtime-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-comparable-benchmark.runtime-overclaim",
    ),
    (
      "external-or-license-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-comparable-benchmark.external-or-license-overclaim",
    ),
    (
      "authority-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-comparable-benchmark.authority-overclaim",
    ),
  ] {
    let value = get(run, field);
    assert_eq!(as_str(get(value, "status")), "Held", "{field}");
    assert_eq!(as_str(get(value, "held-id")), held_id, "{field}");
  }
}

#[test]
fn hard_stops_remain_false_after_bounded_comparable_benchmark() {
  let run = eval_fixture();
  for key in [
    "global-speedup-claimed",
    "whole-system-speedup-claimed",
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
    "implementation-command",
  ] {
    assert!(!as_bool(get(run, key)), "`{key}` must stay false");
  }
}
