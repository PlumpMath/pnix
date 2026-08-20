use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/tesseract-macro-legacy-probe/macro_only_self_bottleneck_attribution_proof_receipt.px",
  )
}

fn eval_fixture() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = fixture_path();
    let json = std::thread::Builder::new()
      .name("macro-only-self-bottleneck-attribution-proof-eval".to_string())
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("self bottleneck attribution proof receipt")
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
    "tesseract-macro-ontology-macro-only-self-bottleneck-attribution-proof"
  );
  assert_eq!(
    as_str(get(run, "constitution-owner")),
    "stdlib/lib/gate/tesseract-constitution.px"
  );
  assert_eq!(
    as_str(get(run, "source-execution-proof")),
    "tesseract-macro-ontology-macro-only-self-benchmark-execution-proof"
  );
}

#[test]
fn constitution_gate_blocks_attribution_collapse_modes() {
  let run = eval_fixture();
  let gate = get(run, "constitutionGate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "macro-only-self-bottleneck-attribution-proof"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "bottleneck-attribution-equals-fast-path-promotion",
    "bottleneck-attribution-equals-external-solver-intake",
    "wrapper-slow-path-equals-semantic-fold-slow",
    "all-mode-bootstrap-audit-slow-equals-new-proof-regression",
    "p-puck-telemetry-equals-semantic-owner",
    "bottleneck-attribution-equals-runtime-api-flattening",
    "bottleneck-attribution-equals-meaning-db",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
}

#[test]
fn attribution_contract_closes_only_attribution_frontier() {
  let run = eval_fixture();
  let contract = get(run, "bottleneck-attribution-contract");
  assert_eq!(
    as_str(get(contract, "proof-id")),
    "proof.macro-only.self-bottleneck-attribution.v1"
  );
  assert!(as_bool(get(
    contract,
    "closes-bottleneck-attribution-frontier"
  )));
  assert!(!as_bool(get(contract, "benchmark-map-surface-bottleneck")));
  assert!(as_bool(get(
    contract,
    "p-puck-wrapper-bottleneck-candidate"
  )));
  assert!(as_bool(get(
    contract,
    "bootstrap-status-audit-bottleneck-candidate"
  )));
  assert_eq!(as_i64(get(contract, "attributed-bottleneck-count")), 2);
  for key in [
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
fn attribution_records_are_visible_and_separated() {
  let run = eval_fixture();
  assert!(as_bool(get(run, "bottleneck-attributed")));
  assert!(as_bool(get(run, "bottleneck-attribution-proof-only")));
  assert_eq!(as_i64(get(run, "attributed-bottleneck-count")), 2);
  let records = attrs_by_id(get(run, "attribution-records"));
  assert_eq!(records.len(), 3);
  assert!(!as_bool(get(
    records["attribution.benchmark-map-owner-surfaces.not-current-bottleneck"],
    "is-bottleneck"
  )));
  assert!(as_bool(get(
    records["attribution.p-puck-wrapper-current-proof-status-query.slow-path"],
    "is-bottleneck"
  )));
  assert!(as_bool(get(
    records["attribution.all-mode-bootstrap-status-audit.long-running"],
    "is-bottleneck"
  )));
}

#[test]
fn six_layer_fold_keeps_attribution_and_optimization_separate() {
  let run = eval_fixture();
  let fold = get(run, "six-layer-bottleneck-attribution-fold");
  assert!(as_bool(get(get(fold, "surface"), "visible")));
  assert_eq!(
    as_i64(get(get(fold, "ontology"), "attributed-bottleneck-count")),
    2
  );
  let semantic = get(fold, "semantic");
  assert!(as_bool(get(semantic, "bottleneck-attributed")));
  assert!(as_bool(get(semantic, "bottleneck-attribution-proof-only")));
  assert!(!as_bool(get(semantic, "benchmark-map-surface-bottleneck")));
  assert!(!as_bool(get(semantic, "optimization-selected")));
  assert!(as_bool(get(
    get(fold, "gate"),
    "blocked-fast-path-promotion"
  )));
  let runtime = get(fold, "runtime");
  assert!(as_bool(get(runtime, "bottleneck-attributed")));
  assert!(!as_bool(get(runtime, "fast-path-promoted")));
  assert!(!as_bool(get(runtime, "runtime-api-flattening")));
}

#[test]
fn trials_cover_valid_sources_slow_evidence_and_held_boundaries() {
  let run = eval_fixture();
  let trials = attrs_by_id(get(run, "bottleneck-attribution-trials"));
  assert_eq!(trials.len(), 18);
  assert_eq!(
    as_str(get(
      trials["trial.A.valid-bottleneck-attribution-proof"],
      "outcome"
    )),
    "self-bottleneck-attribution-proof-present"
  );
  assert_eq!(
    as_i64(get(trials["trial.D.wrapper-slow-path"], "duration-ms")),
    9420
  );
  assert_eq!(
    as_i64(get(
      trials["trial.E.bootstrap-audit-long-running"],
      "duration-lower-bound-ms"
    )),
    300000
  );
  for (id, held) in [
    (
      "trial.F.wrong-proof-id",
      "held.macro-only-self-bottleneck-attribution.proof-id-mismatch",
    ),
    (
      "trial.G.stale-stage",
      "held.macro-only-self-bottleneck-attribution.stale-current-stage",
    ),
    (
      "trial.H.source-mismatch",
      "held.macro-only-self-bottleneck-attribution.source-mismatch",
    ),
    (
      "trial.I.execution-proof-missing",
      "held.macro-only-self-bottleneck-attribution.execution-proof-missing",
    ),
    (
      "trial.J.record-count-mismatch",
      "held.macro-only-self-bottleneck-attribution.record-count-mismatch",
    ),
    (
      "trial.K.record-shape-mismatch",
      "held.macro-only-self-bottleneck-attribution.shape-mismatch",
    ),
    (
      "trial.L.wrapper-evidence-missing",
      "held.macro-only-self-bottleneck-attribution.record-invalid",
    ),
    (
      "trial.M.bootstrap-evidence-missing",
      "held.macro-only-self-bottleneck-attribution.record-invalid",
    ),
    (
      "trial.N.semantic-surface-misattributed",
      "held.macro-only-self-bottleneck-attribution.record-invalid",
    ),
    (
      "trial.O.optimization-overclaim",
      "held.macro-only-self-bottleneck-attribution.optimization-overclaim",
    ),
    (
      "trial.P.authority-overclaim",
      "held.macro-only-self-bottleneck-attribution.authority-overclaim",
    ),
    (
      "trial.Q.runtime-overclaim",
      "held.macro-only-self-bottleneck-attribution.runtime-overclaim",
    ),
    (
      "trial.R.gpl-family-dependency",
      "held.macro-only-self-bottleneck-attribution.gpl-family-dependency",
    ),
  ] {
    assert_eq!(as_str(get(trials[id], "held-id")), held, "{id}");
  }
}

#[test]
fn migration_delta_closes_attribution_but_leaves_optimization_open() {
  let run = eval_fixture();
  let delta = get(run, "migrationDelta");
  let closes = string_set(get(delta, "closes"));
  assert_eq!(closes.len(), 1);
  assert!(closes.contains("need.self.bottleneck-attribution-proof-after-benchmark-map"));
  let not_closed = string_set(get(delta, "does-not-close"));
  assert!(not_closed.contains("need.self.optimization-candidate-after-bottleneck-attribution"));
  assert!(not_closed.contains("need.self.p-puck-wrapper-cold-start-repeat-proof"));
  assert!(not_closed.contains("need.self.bootstrap-status-audit-profile-split-proof"));
  assert!(not_closed.contains("need.domain-runtime-api-flattening-after-semantic-owner"));
  assert!(not_closed.contains("need.stdlib.meaning-db"));
}

#[test]
fn discoveries_record_d635_through_d642() {
  let run = eval_fixture();
  let discoveries = attrs_by_id(get(run, "discoveries"));
  assert_eq!(discoveries.len(), 8);
  for expected in [
    "D635.benchmark-map-owner-surfaces-are-not-current-bottleneck",
    "D636.p-puck-wrapper-query-is-current-slow-path-candidate",
    "D637.all-mode-bootstrap-status-audit-is-existing-long-run-bottleneck",
    "D638.bottleneck-attribution-is-not-optimization-selection",
    "D639.semantic-fold-and-wrapper-cost-stay-separated",
    "D640.future-optimization-targets-wrapper-and-bootstrap-audit-first",
    "D641.external-solver-intake-is-not-justified-by-harness-bottleneck",
    "D642.runtime-flattening-and-meaning-db-remain-open-after-attribution",
  ] {
    assert!(discoveries.contains_key(expected), "missing {expected}");
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
