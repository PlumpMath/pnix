use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/macro-only-self-benchmark-map-owner.px")
}

fn eval_fixture() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = fixture_path();
    let json = std::thread::Builder::new()
      .name("macro-only-self-benchmark-map-owner-eval".to_string())
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true).expect("self benchmark map owner")
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
fn owner_fixture_imports_operation_catalog_source() {
  let run = eval_fixture();
  assert!(as_bool(get(run, "imported-owner")));
  assert!(as_bool(get(run, "used-px-owner")));
  assert_eq!(
    as_str(get(run, "imported-source-proof")),
    "proof.macro-only.self-operation-catalog.v1"
  );
  assert_eq!(
    as_str(get(run, "source-status")),
    "self-operation-catalog-present"
  );
  assert_eq!(as_i64(get(run, "source-operation-count")), 10);
}

#[test]
fn valid_map_emits_ten_benchmark_targets() {
  let run = eval_fixture();
  let proof = get(run, "valid-map");
  assert_eq!(as_str(get(proof, "status")), "self-benchmark-map-present");
  assert!(as_bool(get(proof, "benchmark-map-present")));
  assert!(as_bool(get(proof, "benchmark-map-proof-only")));
  assert_eq!(as_i64(get(proof, "operation-count")), 10);
  assert_eq!(as_i64(get(proof, "measurement-algorithm-count")), 10);
  assert_eq!(as_i64(get(proof, "benchmark-target-count")), 10);
  assert!(string_set(get(proof, "closes")).contains("need.self.benchmark-map"));
}

#[test]
fn benchmark_targets_preserve_operation_ids_and_measurement_hooks() {
  let run = eval_fixture();
  let targets = attrs_by_id(get(run, "benchmark-targets"));
  assert_eq!(targets.len(), 10);
  for id in [
    "benchmark-target.op.surface-to-role-fold",
    "benchmark-target.op.role-emission-verdict",
    "benchmark-target.op.reverse-replay-delta",
    "benchmark-target.op.fixture-local-mutation-loop",
    "benchmark-target.op.held-reopen-taxonomy",
    "benchmark-target.op.receipt-materialization-chain",
    "benchmark-target.op.target-frontier-closure-proof",
    "benchmark-target.op.p-puck-current-cut-audit",
    "benchmark-target.op.scoped-fast-path-install",
    "benchmark-target.op.benchmark-map-handoff",
  ] {
    assert!(targets.contains_key(id), "missing target `{id}`");
  }
  let ppuck = targets["benchmark-target.op.p-puck-current-cut-audit"];
  assert_eq!(
    as_str(get(ppuck, "measurement-hook")),
    "measure.wall-clock-distribution"
  );
  assert!(as_bool(get(ppuck, "p-puck-wrapper-telemetry-accepted")));
}

#[test]
fn measurement_algorithm_registry_and_comparison_shape_are_explicit() {
  let run = eval_fixture();
  let algorithms = attrs_by_id(get(run, "expected-measurement-algorithms"));
  assert_eq!(algorithms.len(), 10);
  for id in [
    "measure.wall-clock-distribution",
    "measure.step-count-and-fold-depth",
    "measure.operation-count",
    "measure.memory-and-allocation-pressure",
    "measure.cache-and-reuse-rate",
    "measure.replay-determinism-hash",
    "measure.held-and-loss-delta",
    "measure.regression-corpus-throughput",
    "measure.bottleneck-attribution",
    "measure.asymptotic-shape-hypothesis",
  ] {
    assert!(algorithms.contains_key(id), "missing algorithm `{id}`");
  }
  let shape = string_set(get(run, "required-comparison-shape"));
  for expected in [
    "baseline-run",
    "candidate-run",
    "repeated-run-distribution",
    "semantic-equivalence-or-held-delta",
    "before-after-replay",
    "bottleneck-attribution",
  ] {
    assert!(shape.contains(expected), "missing shape `{expected}`");
  }
}

#[test]
fn required_fields_and_hard_stops_block_speed_theater() {
  let run = eval_fixture();
  let fields = string_set(get(run, "required-benchmark-target-fields"));
  for expected in [
    "id",
    "operation-id",
    "operation-class",
    "measurement-hook",
    "baseline-run-required",
    "candidate-run-required",
    "repeated-run-distribution-required",
    "semantic-equivalence-or-held-delta-required",
    "before-after-replay-required",
    "bottleneck-attribution-required",
    "hard-stops",
  ] {
    assert!(fields.contains(expected), "missing field `{expected}`");
  }
  let stops = string_set(get(run, "required-benchmark-hard-stops"));
  for expected in [
    "no-single-run-proof",
    "no-green-test-only-proof",
    "no-held-erasure-for-speed",
    "no-owner-law-bypass",
    "no-fast-path-promotion",
    "no-external-solver-intake",
    "no-runtime-api-flattening",
    "no-meaning-db",
    "no-self-modification",
    "no-llm-authority",
    "no-gpl-family-dependency",
  ] {
    assert!(stops.contains(expected), "missing hard stop `{expected}`");
  }
}

#[test]
fn held_trials_cover_source_shape_and_authority_failures() {
  let run = eval_fixture();
  for (key, held) in [
    (
      "wrong-proof",
      "held.macro-only-self-benchmark-map.proof-id-mismatch",
    ),
    (
      "stale-stage",
      "held.macro-only-self-benchmark-map.stale-current-stage",
    ),
    (
      "source-mismatch",
      "held.macro-only-self-benchmark-map.source-mismatch",
    ),
    (
      "source-frontier-missing",
      "held.macro-only-self-benchmark-map.source-frontier-missing",
    ),
    (
      "operation-count-mismatch",
      "held.macro-only-self-benchmark-map.operation-count-mismatch",
    ),
    (
      "algorithm-count-mismatch",
      "held.macro-only-self-benchmark-map.algorithm-count-mismatch",
    ),
    (
      "target-count-mismatch",
      "held.macro-only-self-benchmark-map.target-count-mismatch",
    ),
    (
      "target-authority-overclaim",
      "held.macro-only-self-benchmark-map.target-authority-overclaim",
    ),
    (
      "target-shape-mismatch",
      "held.macro-only-self-benchmark-map.shape-mismatch",
    ),
    (
      "missing-field",
      "held.macro-only-self-benchmark-map.shape-mismatch",
    ),
    (
      "execution-overclaim",
      "held.macro-only-self-benchmark-map.execution-overclaim",
    ),
    (
      "authority-overclaim",
      "held.macro-only-self-benchmark-map.authority-overclaim",
    ),
    (
      "runtime-overclaim",
      "held.macro-only-self-benchmark-map.runtime-overclaim",
    ),
    (
      "gpl-claim",
      "held.macro-only-self-benchmark-map.gpl-family-dependency",
    ),
  ] {
    let trial = get(run, key);
    assert_eq!(as_str(get(trial, "status")), "Held", "{key}");
    assert_eq!(as_str(get(trial, "held-id")), held, "{key}");
  }
}

#[test]
fn benchmark_map_closes_map_only_and_routes_to_execution_proofs() {
  let run = eval_fixture();
  let proof = get(run, "valid-map");
  let next = string_set(get(proof, "next-open-frontiers"));
  assert!(next.contains("need.self.benchmark-execution-proof"));
  assert!(next.contains("need.self.bottleneck-attribution-proof-after-benchmark-map"));
  assert!(!next.contains("need.self.benchmark-map"));
  assert!(!as_bool(get(proof, "benchmark-executed")));
  assert!(!as_bool(get(proof, "bottleneck-attributed")));
  assert!(!as_bool(get(proof, "fast-path-promoted")));
}

#[test]
fn top_level_flags_keep_runtime_solver_and_self_modification_false() {
  let run = eval_fixture();
  assert!(as_bool(get(run, "benchmark-map-present")));
  assert!(as_bool(get(run, "benchmark-map-proof-only")));
  assert_eq!(as_i64(get(run, "benchmark-target-count")), 10);
  for key in [
    "benchmark-executed",
    "bottleneck-attributed",
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
