use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn combined_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../stdlib/lib/gate/macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application-comparable-benchmark-proof.px",
  )
}

fn eval_combined() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = combined_path();
    let json = std::thread::Builder::new()
      .name("cold-start-app-comparable-benchmark-combined-eval".to_string())
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("cold-start application comparable benchmark combined eval")
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
fn fixture_imports_benchmark_owner_and_speedup_boundary_source() {
  let run = fixture();
  assert!(as_bool(get(run, "imported-owner")));
  assert!(as_bool(get(run, "imported-speedup-boundary-owner")));
  assert!(as_bool(get(run, "used-px-owner")));
  assert_eq!(
    as_str(get(run, "expected-source-proof")),
    "proof.macro-only.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application-speedup-boundary.v1"
  );
}

#[test]
fn owner_meta_proves_apples_to_apples_cold_and_warm_speedup() {
  let run = fixture();
  let meta = get(run, "owner-meta");
  assert_eq!(as_i64(get(meta, "baseline-cold-duration-ms")), 12457);
  assert_eq!(as_i64(get(meta, "baseline-warm-duration-ms")), 10610);
  assert_eq!(as_i64(get(meta, "candidate-cold-duration-ms")), 2983);
  assert_eq!(as_i64(get(meta, "candidate-warm-min-duration-ms")), 3024);
  assert_eq!(as_i64(get(meta, "candidate-warm-max-duration-ms")), 3025);
  assert_eq!(as_i64(get(meta, "cold-delta-ms")), 9474);
  assert_eq!(as_i64(get(meta, "warm-delta-min-ms")), 7585);
  assert_eq!(as_i64(get(meta, "warm-delta-max-ms")), 7586);
  assert!(as_bool(get(
    meta,
    "apples-to-apples-same-expression-comparison"
  )));
  assert!(as_bool(get(meta, "apples-to-apples-cold-vs-cold")));
  assert!(as_bool(get(meta, "apples-to-apples-warm-vs-warm")));
  assert!(as_bool(get(
    meta,
    "bounded-status-query-cold-speedup-proven"
  )));
  assert!(as_bool(get(
    meta,
    "bounded-status-query-warm-speedup-proven"
  )));
  assert!(as_bool(get(meta, "bounded-status-query-speedup-proven")));
  assert!(as_bool(get(meta, "runtime-wiring-frontier-required")));
  assert!(!as_bool(get(meta, "runtime-wired")));
  assert!(!as_bool(get(meta, "cold-start-solved")));
  assert!(!as_bool(get(meta, "global-speedup-claimed")));
}

#[test]
fn valid_proof_closes_comparable_benchmark_and_opens_runtime_wiring() {
  let run = fixture();
  let valid = get(run, "valid-proof");
  assert_eq!(
    as_str(get(valid, "status")),
    "self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application-comparable-benchmark-proof-present"
  );
  assert!(get(valid, "held-id").is_null());
  assert!(as_bool(get(valid, "bounded-status-query-speedup-proven")));
  assert!(!as_bool(get(valid, "runtime-wired")));

  let closed = string_set(get(valid, "closes"));
  assert!(closed.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application-comparable-benchmark-proof"
  ));
  let open = string_set(get(valid, "next-open-frontiers"));
  assert!(open.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application-runtime-wiring-proof"
  ));
}

#[test]
fn benchmark_registry_records_five_records_with_both_deltas() {
  let run = fixture();
  let reg = get(run, "benchmark-registry");
  assert_eq!(
    as_str(get(reg, "id")),
    "elimination-application-comparable-benchmark.fast-path.bootstrap-status-audit.shallow-summary.global-default-replacement.cold-start.v1"
  );
  assert_eq!(as_i64(get(reg, "benchmark-record-count")), 5);
  assert_eq!(as_i64(get(reg, "cold-delta-ms")), 9474);
  assert_eq!(as_i64(get(reg, "warm-delta-min-ms")), 7585);
  assert_eq!(as_i64(get(reg, "warm-delta-max-ms")), 7586);
  assert!(as_bool(get(reg, "apples-to-apples-cold-vs-cold")));
  assert!(as_bool(get(reg, "apples-to-apples-warm-vs-warm")));
  assert!(as_bool(get(reg, "bounded-status-query-speedup-proven")));
  assert!(as_bool(get(reg, "runtime-wiring-frontier-required")));
  assert!(!as_bool(get(reg, "runtime-wired")));
  assert!(!as_bool(get(reg, "cold-start-solved")));
}

#[test]
fn held_failures_cover_all_branches() {
  let run = fixture();
  let prefix = "held.macro-only-self...cold-start-application-comparable-benchmark";
  for (field, suffix) in [
    ("wrong-proof", "proof-id-mismatch"),
    ("stale-stage", "stale-current-stage"),
    ("source-mismatch", "source-mismatch"),
    ("boundary-source-missing", "boundary-source-missing"),
    (
      "boundary-input-shape-mismatch",
      "boundary-input-shape-mismatch",
    ),
    (
      "benchmark-record-shape-mismatch",
      "benchmark-record-shape-mismatch",
    ),
    ("benchmark-record-invalid", "benchmark-record-invalid"),
    ("benchmark-status-mismatch", "benchmark-status-mismatch"),
    ("duration-mismatch", "duration-mismatch"),
    ("delta-mismatch", "delta-mismatch"),
    ("comparison-mismatch", "comparison-mismatch"),
    ("scope-mismatch", "scope-mismatch"),
    ("registry-mismatch", "registry-mismatch"),
    ("held-flags-missing", "held-flags-missing"),
    ("audit-fallback-missing", "audit-fallback-missing"),
    ("missing-evidence", "missing-required-evidence"),
    ("frontier-shape-mismatch", "frontier-shape-mismatch"),
    ("benchmark-overclaim", "benchmark-overclaim"),
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
fn hard_stops_remain_false_after_comparable_benchmark_proof() {
  let run = fixture();
  for key in [
    "runtime-wired",
    "cold-start-solved",
    "cold-start-eliminated",
    "cold-start-globally-bypassed",
    "elimination-applied-globally",
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
    "bounded-status-query-cold-speedup-proven"
  )));
  assert!(as_bool(get(
    run,
    "bounded-status-query-warm-speedup-proven"
  )));
  assert!(as_bool(get(run, "bounded-status-query-speedup-proven")));
  assert!(as_bool(get(run, "apples-to-apples-cold-vs-cold")));
  assert!(as_bool(get(run, "apples-to-apples-warm-vs-warm")));
  assert!(as_bool(get(run, "runtime-wiring-frontier-required")));
}
