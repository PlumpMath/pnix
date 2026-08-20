use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/tesseract-macro-legacy-probe/macro_only_self_benchmark_map_receipt.px")
}

fn eval_fixture() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = fixture_path();
    let json = std::thread::Builder::new()
      .name("macro-only-self-benchmark-map-eval".to_string())
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true).expect("self benchmark map receipt")
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
fn marker_and_source_receipts_are_pinned() {
  let run = eval_fixture();
  assert_eq!(
    as_str(get(run, "probe-marker")),
    "tesseract-macro-ontology-macro-only-self-benchmark-map"
  );
  assert_eq!(
    as_str(get(run, "constitution-owner")),
    "stdlib/lib/gate/tesseract-constitution.px"
  );
  assert_eq!(
    as_str(get(run, "source-operation-catalog")),
    "tesseract-macro-ontology-macro-only-self-operation-catalog"
  );
  assert_eq!(
    as_str(get(run, "measurement-registry-source")),
    "registry.performance-measurement-algorithms.v1"
  );
}

#[test]
fn constitution_gate_blocks_benchmark_map_collapse_modes() {
  let run = eval_fixture();
  let gate = get(run, "constitutionGate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "macro-only-self-benchmark-map"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "benchmark-map-equals-benchmark-execution",
    "benchmark-map-equals-bottleneck-attribution",
    "benchmark-map-equals-fast-path-promotion",
    "single-run-equals-speed-proof",
    "green-test-equals-performance-proof",
    "speed-equals-held-erasure",
    "p-puck-telemetry-equals-semantic-owner",
    "benchmark-map-equals-external-solver-intake",
    "benchmark-map-equals-runtime-api-flattening",
    "benchmark-map-equals-meaning-db",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
}

#[test]
fn benchmark_map_contract_closes_only_map_frontier() {
  let run = eval_fixture();
  let contract = get(run, "benchmark-map-contract");
  assert_eq!(
    as_str(get(contract, "proof-id")),
    "proof.macro-only.self-benchmark-map.v1"
  );
  assert!(as_bool(get(contract, "closes-benchmark-map-frontier")));
  assert_eq!(as_i64(get(contract, "operation-count")), 10);
  assert_eq!(as_i64(get(contract, "measurement-algorithm-count")), 10);
  assert_eq!(as_i64(get(contract, "benchmark-target-count")), 10);
  for key in [
    "closes-benchmark-execution",
    "closes-bottleneck-attribution",
    "promotes-fast-path",
    "installs-external-solver",
    "closes-global-runtime",
    "closes-runtime-api-flattening",
    "closes-meaning-db",
    "grants-llm-authority",
    "self-modification",
  ] {
    assert!(!as_bool(get(contract, key)), "`{key}` must stay false");
  }
}

#[test]
fn benchmark_map_contains_ten_operation_measurement_targets() {
  let run = eval_fixture();
  assert!(as_bool(get(run, "benchmark-map-present")));
  assert!(as_bool(get(run, "benchmark-map-proof-only")));
  assert_eq!(as_i64(get(run, "operation-count")), 10);
  assert_eq!(as_i64(get(run, "measurement-algorithm-count")), 10);
  assert_eq!(as_i64(get(run, "benchmark-target-count")), 10);
  let targets = attrs_by_id(get(run, "benchmark-targets"));
  let fast = targets["benchmark-target.op.scoped-fast-path-install"];
  assert_eq!(
    as_str(get(fast, "measurement-hook")),
    "measure.cache-and-reuse-rate"
  );
  assert!(!as_bool(get(fast, "fast-path-promoted")));
  assert!(!as_bool(get(fast, "runtime-api")));
}

#[test]
fn six_layer_fold_keeps_measurement_runtime_and_audit_separate() {
  let run = eval_fixture();
  let fold = get(run, "six-layer-benchmark-map-fold");
  assert!(as_bool(get(get(fold, "surface"), "visible")));
  assert_eq!(
    as_i64(get(get(fold, "ontology"), "benchmark-target-count")),
    10
  );
  assert!(as_bool(get(get(fold, "semantic"), "benchmark-map-present")));
  assert!(!as_bool(get(get(fold, "semantic"), "benchmark-executed")));
  assert!(as_bool(get(get(fold, "gate"), "blocked-single-run-proof")));
  let runtime = get(fold, "runtime");
  assert!(as_bool(get(runtime, "benchmark-map-present")));
  assert!(!as_bool(get(runtime, "benchmark-executed")));
  assert!(!as_bool(get(runtime, "bottleneck-attributed")));
  assert!(!as_bool(get(runtime, "runtime-api-flattening")));
  assert_eq!(
    as_i64(get(get(fold, "audit"), "benchmark-target-count")),
    10
  );
}

#[test]
fn trials_cover_valid_sources_and_held_boundaries() {
  let run = eval_fixture();
  let trials = attrs_by_id(get(run, "benchmark-map-trials"));
  assert_eq!(trials.len(), 17);
  assert_eq!(
    as_str(get(trials["trial.A.valid-benchmark-map"], "outcome")),
    "self-benchmark-map-present"
  );
  for (id, held) in [
    (
      "trial.D.wrong-proof-id",
      "held.macro-only-self-benchmark-map.proof-id-mismatch",
    ),
    (
      "trial.E.stale-stage",
      "held.macro-only-self-benchmark-map.stale-current-stage",
    ),
    (
      "trial.F.source-mismatch",
      "held.macro-only-self-benchmark-map.source-mismatch",
    ),
    (
      "trial.G.source-frontier-missing",
      "held.macro-only-self-benchmark-map.source-frontier-missing",
    ),
    (
      "trial.H.operation-count-mismatch",
      "held.macro-only-self-benchmark-map.operation-count-mismatch",
    ),
    (
      "trial.I.algorithm-count-mismatch",
      "held.macro-only-self-benchmark-map.algorithm-count-mismatch",
    ),
    (
      "trial.J.target-count-mismatch",
      "held.macro-only-self-benchmark-map.target-count-mismatch",
    ),
    (
      "trial.K.target-authority-overclaim",
      "held.macro-only-self-benchmark-map.target-authority-overclaim",
    ),
    (
      "trial.L.target-shape-mismatch",
      "held.macro-only-self-benchmark-map.shape-mismatch",
    ),
    (
      "trial.N.execution-overclaim",
      "held.macro-only-self-benchmark-map.execution-overclaim",
    ),
    (
      "trial.O.authority-overclaim",
      "held.macro-only-self-benchmark-map.authority-overclaim",
    ),
    (
      "trial.P.runtime-overclaim",
      "held.macro-only-self-benchmark-map.runtime-overclaim",
    ),
    (
      "trial.Q.gpl-family-dependency",
      "held.macro-only-self-benchmark-map.gpl-family-dependency",
    ),
  ] {
    assert_eq!(as_str(get(trials[id], "held-id")), held, "{id}");
  }
}

#[test]
fn migration_delta_closes_benchmark_map_but_leaves_execution_open() {
  let run = eval_fixture();
  let delta = get(run, "migrationDelta");
  let closes = string_set(get(delta, "closes"));
  assert_eq!(closes.len(), 1);
  assert!(closes.contains("need.self.benchmark-map"));
  let not_closed = string_set(get(delta, "does-not-close"));
  assert!(not_closed.contains("need.self.benchmark-execution-proof"));
  assert!(not_closed.contains("need.self.bottleneck-attribution-proof-after-benchmark-map"));
  assert!(not_closed.contains("need.domain-runtime-api-flattening-after-semantic-owner"));
  assert!(not_closed.contains("need.stdlib.meaning-db"));
}

#[test]
fn discoveries_record_d619_through_d626() {
  let run = eval_fixture();
  let discoveries = attrs_by_id(get(run, "discoveries"));
  assert_eq!(discoveries.len(), 8);
  for expected in [
    "D619.operation-catalog-lowers-into-benchmark-obligation-map",
    "D620.benchmark-map-is-not-benchmark-execution",
    "D621.single-run-and-green-test-only-speed-claims-are-held",
    "D622.fast-path-promotion-requires-held-loss-and-replay-preservation",
    "D623.p-puck-telemetry-is-measurement-input-not-semantic-owner",
    "D624.external-solver-intake-stays-deferred-until-bottleneck-proof",
    "D625.benchmark-map-routes-to-execution-and-bottleneck-attribution",
    "D626.runtime-flattening-and-global-runtime-remain-blocked-until-measured-proof",
  ] {
    assert!(discoveries.contains_key(expected), "missing {expected}");
  }
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
    "optimization-selected",
    "fast-path-promoted",
    "external-solver-installed",
    "runtime-install",
    "global-ontology-runtime",
    "runtime-api-flattening",
    "meaning-db",
    "host-code-removal-started",
    "implementation-command",
    "llm-authority",
    "self-modification",
    "p-puck-is-semantic-owner",
    "old-host-authority",
    "gpl-family-dependencies",
  ] {
    assert!(!as_bool(get(run, key)), "`{key}` must stay false");
  }
}
