use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/tesseract-macro-legacy-probe/macro_only_self_benchmark_execution_proof_receipt.px",
  )
}

fn eval_fixture() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = fixture_path();
    let json = std::thread::Builder::new()
      .name("macro-only-self-benchmark-execution-proof-eval".to_string())
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("self benchmark execution proof receipt")
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
fn marker_and_source_are_pinned() {
  let run = eval_fixture();
  assert_eq!(
    as_str(get(run, "probe-marker")),
    "tesseract-macro-ontology-macro-only-self-benchmark-execution-proof"
  );
  assert_eq!(
    as_str(get(run, "constitution-owner")),
    "stdlib/lib/gate/tesseract-constitution.px"
  );
  assert_eq!(
    as_str(get(run, "source-benchmark-map")),
    "tesseract-macro-ontology-macro-only-self-benchmark-map"
  );
}

#[test]
fn constitution_gate_blocks_execution_collapse_modes() {
  let run = eval_fixture();
  let gate = get(run, "constitutionGate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "macro-only-self-benchmark-execution-proof"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "benchmark-execution-equals-bottleneck-attribution",
    "benchmark-execution-equals-fast-path-promotion",
    "within-threshold-equals-no-future-profile-needed",
    "slow-warmup-equals-persistent-bottleneck",
    "fixture-direct-import-failure-equals-runtime-failure",
    "p-puck-telemetry-equals-semantic-owner",
    "benchmark-execution-equals-runtime-api-flattening",
    "benchmark-execution-equals-meaning-db",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
}

#[test]
fn execution_contract_closes_only_execution_frontier() {
  let run = eval_fixture();
  let contract = get(run, "benchmark-execution-contract");
  assert_eq!(
    as_str(get(contract, "proof-id")),
    "proof.macro-only.self-benchmark-execution.v1"
  );
  assert!(as_bool(get(
    contract,
    "closes-benchmark-execution-frontier"
  )));
  assert_eq!(as_i64(get(contract, "benchmark-target-count")), 10);
  assert_eq!(as_i64(get(contract, "executed-benchmark-record-count")), 3);
  assert_eq!(as_i64(get(contract, "successful-p-puck-run-count")), 9);
  for key in [
    "closes-bottleneck-attribution",
    "selects-optimization",
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
fn execution_records_and_negative_boundary_are_visible() {
  let run = eval_fixture();
  assert!(as_bool(get(run, "benchmark-executed")));
  assert!(as_bool(get(run, "benchmark-execution-proof-only")));
  assert_eq!(as_i64(get(run, "executed-benchmark-record-count")), 3);
  assert_eq!(as_i64(get(run, "successful-p-puck-run-count")), 9);
  assert!(as_bool(get(run, "repeated-run-distribution-captured")));
  assert!(as_bool(get(run, "direct-fixture-execution-held")));
  let records = attrs_by_id(get(run, "execution-records"));
  assert!(records.contains_key("execution.status.expected-status"));
  assert!(records.contains_key("execution.target-count.materialized"));
  assert!(records.contains_key("execution.target-id-list.materialized"));
  let negative = get(run, "negative-execution-record");
  assert_eq!(as_str(get(negative, "error-kind")), "path-traversal");
  assert!(as_bool(get(negative, "held-boundary")));
}

#[test]
fn six_layer_fold_keeps_execution_and_attribution_separate() {
  let run = eval_fixture();
  let fold = get(run, "six-layer-benchmark-execution-fold");
  assert!(as_bool(get(get(fold, "surface"), "visible")));
  assert_eq!(
    as_i64(get(get(fold, "ontology"), "benchmark-target-count")),
    10
  );
  assert!(as_bool(get(get(fold, "semantic"), "benchmark-executed")));
  assert!(!as_bool(get(
    get(fold, "semantic"),
    "bottleneck-attributed"
  )));
  assert!(as_bool(get(
    get(fold, "gate"),
    "blocked-optimization-overclaim"
  )));
  let runtime = get(fold, "runtime");
  assert!(as_bool(get(runtime, "benchmark-executed")));
  assert!(!as_bool(get(runtime, "bottleneck-attributed")));
  assert!(!as_bool(get(runtime, "runtime-api-flattening")));
  assert_eq!(
    as_i64(get(get(fold, "audit"), "successful-p-puck-run-count")),
    9
  );
}

#[test]
fn trials_cover_valid_sources_and_held_boundaries() {
  let run = eval_fixture();
  let trials = attrs_by_id(get(run, "benchmark-execution-trials"));
  assert_eq!(trials.len(), 17);
  assert_eq!(
    as_str(get(
      trials["trial.A.valid-benchmark-execution-proof"],
      "outcome"
    )),
    "self-benchmark-execution-proof-present"
  );
  for (id, held) in [
    (
      "trial.D.wrong-proof-id",
      "held.macro-only-self-benchmark-execution.proof-id-mismatch",
    ),
    (
      "trial.E.stale-stage",
      "held.macro-only-self-benchmark-execution.stale-current-stage",
    ),
    (
      "trial.F.source-mismatch",
      "held.macro-only-self-benchmark-execution.source-mismatch",
    ),
    (
      "trial.G.map-missing",
      "held.macro-only-self-benchmark-execution.benchmark-map-missing",
    ),
    (
      "trial.H.record-count-mismatch",
      "held.macro-only-self-benchmark-execution.record-count-mismatch",
    ),
    (
      "trial.I.record-shape-mismatch",
      "held.macro-only-self-benchmark-execution.shape-mismatch",
    ),
    (
      "trial.J.insufficient-samples",
      "held.macro-only-self-benchmark-execution.execution-record-invalid",
    ),
    (
      "trial.K.slow-or-unstable-output",
      "held.macro-only-self-benchmark-execution.execution-record-invalid",
    ),
    (
      "trial.L.negative-boundary-missing",
      "held.macro-only-self-benchmark-execution.negative-boundary-missing",
    ),
    (
      "trial.N.optimization-overclaim",
      "held.macro-only-self-benchmark-execution.optimization-overclaim",
    ),
    (
      "trial.O.authority-overclaim",
      "held.macro-only-self-benchmark-execution.authority-overclaim",
    ),
    (
      "trial.P.runtime-overclaim",
      "held.macro-only-self-benchmark-execution.runtime-overclaim",
    ),
    (
      "trial.Q.gpl-family-dependency",
      "held.macro-only-self-benchmark-execution.gpl-family-dependency",
    ),
  ] {
    assert_eq!(as_str(get(trials[id], "held-id")), held, "{id}");
  }
}

#[test]
fn migration_delta_closes_execution_but_leaves_attribution_open() {
  let run = eval_fixture();
  let delta = get(run, "migrationDelta");
  let closes = string_set(get(delta, "closes"));
  assert_eq!(closes.len(), 1);
  assert!(closes.contains("need.self.benchmark-execution-proof"));
  let not_closed = string_set(get(delta, "does-not-close"));
  assert!(not_closed.contains("need.self.bottleneck-attribution-proof-after-benchmark-map"));
  assert!(not_closed.contains("need.domain-runtime-api-flattening-after-semantic-owner"));
  assert!(not_closed.contains("need.stdlib.meaning-db"));
}

#[test]
fn discoveries_record_d627_through_d634() {
  let run = eval_fixture();
  let discoveries = attrs_by_id(get(run, "discoveries"));
  assert_eq!(discoveries.len(), 8);
  for expected in [
    "D627.benchmark-execution-captures-repeated-p-puck-distributions",
    "D628.stdlib-owner-path-is-executable-while-fixture-path-traversal-is-held",
    "D629.target-obligation-materialization-is-proven-before-attribution",
    "D630.warmup-slow-path-does-not-equal-persistent-bottleneck",
    "D631.benchmark-execution-is-not-bottleneck-attribution",
    "D632.benchmark-execution-does-not-flatten-runtime",
    "D633.p-puck-telemetry-remains-measurement-not-semantic-owner",
    "D634.external-solver-and-fast-path-remain-blocked-until-attribution",
  ] {
    assert!(discoveries.contains_key(expected), "missing {expected}");
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
