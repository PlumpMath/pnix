use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/macro-only-self-benchmark-execution-proof-owner.px")
}

fn eval_fixture() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = fixture_path();
    let json = std::thread::Builder::new()
      .name("macro-only-self-benchmark-execution-proof-owner-eval".to_string())
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("self benchmark execution proof owner")
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
fn owner_fixture_imports_benchmark_map_source() {
  let run = eval_fixture();
  assert!(as_bool(get(run, "imported-owner")));
  assert!(as_bool(get(run, "used-px-owner")));
  assert_eq!(
    as_str(get(run, "imported-source-proof")),
    "proof.macro-only.self-benchmark-map.v1"
  );
  assert_eq!(
    as_str(get(run, "source-status")),
    "self-benchmark-map-present"
  );
  assert_eq!(
    as_str(get(run, "source-benchmark-map-owner")),
    "stdlib.lib.gate.macro-only-self-benchmark-map"
  );
}

#[test]
fn valid_proof_records_three_execution_families_and_nine_runs() {
  let run = eval_fixture();
  let proof = get(run, "valid-proof");
  assert_eq!(
    as_str(get(proof, "status")),
    "self-benchmark-execution-proof-present"
  );
  assert!(as_bool(get(proof, "benchmark-executed")));
  assert!(as_bool(get(proof, "benchmark-execution-proof-only")));
  assert_eq!(as_i64(get(proof, "executed-benchmark-record-count")), 3);
  assert_eq!(as_i64(get(proof, "successful-p-puck-run-count")), 9);
  assert!(as_bool(get(proof, "repeated-run-distribution-captured")));
  assert!(string_set(get(proof, "closes")).contains("need.self.benchmark-execution-proof"));
}

#[test]
fn execution_records_pin_actual_p_puck_samples() {
  let run = eval_fixture();
  let records = attrs_by_id(get(run, "expected-execution-records"));
  assert_eq!(records.len(), 3);

  let status = records["execution.status.expected-status"];
  assert_eq!(
    as_str(get(status, "expected-output-token")),
    "self-benchmark-map-present"
  );
  assert_eq!(as_i64(get(status, "duration-min-ms")), 219);
  assert_eq!(as_i64(get(status, "duration-p50-ms")), 223);
  assert_eq!(as_i64(get(status, "duration-p95-ms")), 224);
  assert_eq!(as_i64(get(status, "duration-max-ms")), 224);

  let count = records["execution.target-count.materialized"];
  assert_eq!(as_str(get(count, "expected-output-token")), "10");
  assert_eq!(as_i64(get(count, "duration-min-ms")), 266);
  assert_eq!(as_i64(get(count, "duration-p50-ms")), 271);
  assert_eq!(as_i64(get(count, "duration-p95-ms")), 275);
  assert_eq!(as_i64(get(count, "duration-max-ms")), 275);

  let ids = records["execution.target-id-list.materialized"];
  assert!(
    as_str(get(ids, "expected-output-token")).contains("benchmark-target.op.surface-to-role-fold")
  );
  assert_eq!(as_i64(get(ids, "duration-min-ms")), 267);
  assert_eq!(as_i64(get(ids, "duration-p50-ms")), 273);
  assert_eq!(as_i64(get(ids, "duration-p95-ms")), 273);
  assert_eq!(as_i64(get(ids, "duration-max-ms")), 273);

  for record in records.values() {
    assert_eq!(as_i64(get(record, "sample-count")), 3);
    assert_eq!(as_str(get(record, "status")), "within-threshold");
    assert!(as_bool(get(record, "command-exit-zero")));
    assert!(as_bool(get(record, "output-stable")));
    assert!(!as_bool(get(record, "bottleneck-attributed")));
  }
}

#[test]
fn negative_fixture_direct_import_boundary_is_preserved() {
  let run = eval_fixture();
  let negative = get(run, "expected-negative-execution-record");
  assert_eq!(
    as_str(get(negative, "id")),
    "negative.fixture-direct-import.path-traversal-held"
  );
  assert_eq!(as_str(get(negative, "status")), "failed-run-debug-required");
  assert_eq!(as_i64(get(negative, "duration-ms")), 189);
  assert!(!as_bool(get(negative, "command-exit-zero")));
  assert_eq!(as_str(get(negative, "error-kind")), "path-traversal");
  assert!(as_bool(get(negative, "held-boundary")));
  assert!(as_bool(get(negative, "accepted-as-negative-evidence")));
}

#[test]
fn required_evidence_and_remaining_frontiers_are_explicit() {
  let run = eval_fixture();
  let evidence = string_set(get(run, "required-evidence"));
  for expected in [
    "p-puck-status-expression-run-distribution-present",
    "p-puck-target-count-run-distribution-present",
    "p-puck-target-id-list-run-distribution-present",
    "three-samples-per-successful-record",
    "successful-run-count-is-nine",
    "path-traversal-held-boundary-recorded",
    "bottleneck-attribution-deferred",
  ] {
    assert!(evidence.contains(expected), "missing evidence `{expected}`");
  }
  let frontiers = string_set(get(run, "remaining-open-frontiers"));
  assert!(!frontiers.contains("need.self.benchmark-execution-proof"));
  assert!(frontiers.contains("need.self.bottleneck-attribution-proof-after-benchmark-map"));
}

#[test]
fn held_trials_cover_source_shape_sample_and_negative_failures() {
  let run = eval_fixture();
  for (key, held) in [
    (
      "wrong-proof",
      "held.macro-only-self-benchmark-execution.proof-id-mismatch",
    ),
    (
      "stale-stage",
      "held.macro-only-self-benchmark-execution.stale-current-stage",
    ),
    (
      "source-mismatch",
      "held.macro-only-self-benchmark-execution.source-mismatch",
    ),
    (
      "map-missing",
      "held.macro-only-self-benchmark-execution.benchmark-map-missing",
    ),
    (
      "record-count-mismatch",
      "held.macro-only-self-benchmark-execution.record-count-mismatch",
    ),
    (
      "record-shape-mismatch",
      "held.macro-only-self-benchmark-execution.shape-mismatch",
    ),
    (
      "insufficient-samples",
      "held.macro-only-self-benchmark-execution.execution-record-invalid",
    ),
    (
      "slow-or-unstable-output",
      "held.macro-only-self-benchmark-execution.execution-record-invalid",
    ),
    (
      "negative-boundary-missing",
      "held.macro-only-self-benchmark-execution.negative-boundary-missing",
    ),
    (
      "missing-evidence",
      "held.macro-only-self-benchmark-execution.shape-mismatch",
    ),
  ] {
    let trial = get(run, key);
    assert_eq!(as_str(get(trial, "status")), "Held", "{key}");
    assert_eq!(as_str(get(trial, "held-id")), held, "{key}");
  }
}

#[test]
fn overclaims_are_held() {
  let run = eval_fixture();
  for (key, held) in [
    (
      "optimization-overclaim",
      "held.macro-only-self-benchmark-execution.optimization-overclaim",
    ),
    (
      "authority-overclaim",
      "held.macro-only-self-benchmark-execution.authority-overclaim",
    ),
    (
      "runtime-overclaim",
      "held.macro-only-self-benchmark-execution.runtime-overclaim",
    ),
    (
      "gpl-claim",
      "held.macro-only-self-benchmark-execution.gpl-family-dependency",
    ),
  ] {
    let trial = get(run, key);
    assert_eq!(as_str(get(trial, "status")), "Held", "{key}");
    assert_eq!(as_str(get(trial, "held-id")), held, "{key}");
  }
}

#[test]
fn top_level_flags_keep_attribution_runtime_solver_and_self_modification_false() {
  let run = eval_fixture();
  assert!(as_bool(get(run, "benchmark-executed")));
  assert!(as_bool(get(run, "benchmark-execution-proof-only")));
  assert_eq!(as_i64(get(run, "successful-p-puck-run-count")), 9);
  for key in [
    "bottleneck-attributed",
    "optimization-selected",
    "fast-path-promoted",
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
  ] {
    assert!(!as_bool(get(run, key)), "`{key}` must stay false");
  }
}
