use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/macro-only-self-bottleneck-attribution-proof-owner.px")
}

fn eval_fixture() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = fixture_path();
    let json = std::thread::Builder::new()
      .name("macro-only-self-bottleneck-attribution-proof-owner-eval".to_string())
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("self bottleneck attribution proof owner")
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
fn owner_fixture_imports_benchmark_execution_source() {
  let run = eval_fixture();
  assert!(as_bool(get(run, "imported-owner")));
  assert!(as_bool(get(run, "used-px-owner")));
  assert_eq!(
    as_str(get(run, "imported-source-proof")),
    "proof.macro-only.self-benchmark-execution.v1"
  );
  assert_eq!(
    as_str(get(run, "source-status")),
    "self-benchmark-execution-proof-present"
  );
  assert_eq!(
    as_str(get(run, "source-execution-owner")),
    "stdlib.lib.gate.macro-only-self-benchmark-execution-proof"
  );
}

#[test]
fn valid_proof_attributes_two_bottleneck_candidates_without_optimization() {
  let run = eval_fixture();
  let proof = get(run, "valid-proof");
  assert_eq!(
    as_str(get(proof, "status")),
    "self-bottleneck-attribution-proof-present"
  );
  assert!(as_bool(get(proof, "bottleneck-attributed")));
  assert!(as_bool(get(proof, "bottleneck-attribution-proof-only")));
  assert_eq!(as_i64(get(proof, "attributed-bottleneck-count")), 2);
  assert!(!as_bool(get(proof, "benchmark-map-surface-bottleneck")));
  assert!(as_bool(get(proof, "p-puck-wrapper-bottleneck-candidate")));
  assert!(as_bool(get(
    proof,
    "bootstrap-status-audit-bottleneck-candidate"
  )));
  assert!(string_set(get(proof, "closes"))
    .contains("need.self.bottleneck-attribution-proof-after-benchmark-map"));
  assert!(!as_bool(get(proof, "optimization-selected")));
  assert!(!as_bool(get(proof, "fast-path-promoted")));
}

#[test]
fn attribution_records_pin_surface_wrapper_and_bootstrap_evidence() {
  let run = eval_fixture();
  let records = attrs_by_id(get(run, "expected-attribution-records"));
  assert_eq!(records.len(), 3);

  let surface = records["attribution.benchmark-map-owner-surfaces.not-current-bottleneck"];
  assert_eq!(as_str(get(surface, "class")), "not-current-bottleneck");
  assert!(!as_bool(get(surface, "is-bottleneck")));
  assert_eq!(as_i64(get(surface, "max-observed-ms")), 275);
  assert_eq!(as_i64(get(surface, "slow-threshold-ms")), 5000);

  let wrapper = records["attribution.p-puck-wrapper-current-proof-status-query.slow-path"];
  assert_eq!(
    as_str(get(wrapper, "class")),
    "wrapper-cold-start-or-cargo-run-overhead"
  );
  assert!(as_bool(get(wrapper, "is-bottleneck")));
  assert_eq!(as_i64(get(wrapper, "duration-ms")), 9420);
  assert_eq!(
    as_str(get(wrapper, "evidence-status")),
    "slow-path-candidate"
  );
  assert!(as_str(get(wrapper, "attributed-to")).contains("p-puck cargo-run"));

  let bootstrap = records["attribution.all-mode-bootstrap-status-audit.long-running"];
  assert_eq!(
    as_str(get(bootstrap, "class")),
    "existing-bootstrap-status-audit-receipt-evaluation"
  );
  assert!(as_bool(get(bootstrap, "is-bottleneck")));
  assert_eq!(as_i64(get(bootstrap, "duration-lower-bound-ms")), 300000);
  assert_eq!(as_i64(get(bootstrap, "observed-cpu-percent")), 99);
  assert_eq!(
    as_str(get(bootstrap, "evidence-status")),
    "long-running-terminated"
  );

  for record in records.values() {
    assert!(as_bool(get(record, "attribution-recorded")));
    assert!(!as_bool(get(record, "semantic-owner")));
    assert!(!as_bool(get(record, "optimization-selected")));
    assert!(!as_bool(get(record, "fast-path-promoted")));
    assert!(!as_bool(get(record, "external-solver-selected")));
  }
}

#[test]
fn required_evidence_and_next_frontiers_are_explicit() {
  let run = eval_fixture();
  let evidence = string_set(get(run, "required-evidence"));
  for expected in [
    "benchmark-map-surface-not-current-bottleneck",
    "p-puck-wrapper-slow-path-candidate-present",
    "all-mode-bootstrap-audit-long-run-evidence-present",
    "bottleneck-attribution-is-classification-only",
    "optimization-deferred-after-attribution",
  ] {
    assert!(evidence.contains(expected), "missing evidence `{expected}`");
  }

  let frontiers = string_set(get(run, "remaining-open-frontiers"));
  assert!(!frontiers.contains("need.self.bottleneck-attribution-proof-after-benchmark-map"));
  assert!(frontiers.contains("need.self.p-puck-wrapper-cold-start-repeat-proof"));
  assert!(frontiers.contains("need.self.bootstrap-status-audit-profile-split-proof"));
  assert!(frontiers.contains("need.self.optimization-candidate-after-bottleneck-attribution"));
}

#[test]
fn held_trials_cover_source_shape_and_evidence_failures() {
  let run = eval_fixture();
  for (key, held) in [
    (
      "wrong-proof",
      "held.macro-only-self-bottleneck-attribution.proof-id-mismatch",
    ),
    (
      "stale-stage",
      "held.macro-only-self-bottleneck-attribution.stale-current-stage",
    ),
    (
      "source-mismatch",
      "held.macro-only-self-bottleneck-attribution.source-mismatch",
    ),
    (
      "execution-proof-missing",
      "held.macro-only-self-bottleneck-attribution.execution-proof-missing",
    ),
    (
      "record-count-mismatch",
      "held.macro-only-self-bottleneck-attribution.record-count-mismatch",
    ),
    (
      "record-shape-mismatch",
      "held.macro-only-self-bottleneck-attribution.shape-mismatch",
    ),
    (
      "missing-slow-wrapper-evidence",
      "held.macro-only-self-bottleneck-attribution.record-invalid",
    ),
    (
      "missing-bootstrap-evidence",
      "held.macro-only-self-bottleneck-attribution.record-invalid",
    ),
    (
      "semantic-surface-misattributed",
      "held.macro-only-self-bottleneck-attribution.record-invalid",
    ),
    (
      "missing-evidence",
      "held.macro-only-self-bottleneck-attribution.shape-mismatch",
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
      "held.macro-only-self-bottleneck-attribution.optimization-overclaim",
    ),
    (
      "authority-overclaim",
      "held.macro-only-self-bottleneck-attribution.authority-overclaim",
    ),
    (
      "runtime-overclaim",
      "held.macro-only-self-bottleneck-attribution.runtime-overclaim",
    ),
    (
      "gpl-claim",
      "held.macro-only-self-bottleneck-attribution.gpl-family-dependency",
    ),
  ] {
    let trial = get(run, key);
    assert_eq!(as_str(get(trial, "status")), "Held", "{key}");
    assert_eq!(as_str(get(trial, "held-id")), held, "{key}");
  }
}

#[test]
fn top_level_flags_keep_optimization_runtime_solver_and_self_modification_false() {
  let run = eval_fixture();
  assert!(as_bool(get(run, "bottleneck-attributed")));
  assert!(as_bool(get(run, "bottleneck-attribution-proof-only")));
  assert_eq!(as_i64(get(run, "attributed-bottleneck-count")), 2);
  for key in [
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
