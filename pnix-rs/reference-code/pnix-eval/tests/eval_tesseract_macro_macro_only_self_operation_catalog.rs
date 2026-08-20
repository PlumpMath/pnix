use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/tesseract-macro-legacy-probe/macro_only_self_operation_catalog_receipt.px",
  )
}

fn eval_fixture() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = fixture_path();
    let json = std::thread::Builder::new()
      .name("macro-only-self-operation-catalog-eval".to_string())
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("self operation catalog receipt")
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
    "tesseract-macro-ontology-macro-only-self-operation-catalog"
  );
  assert_eq!(
    as_str(get(run, "constitution-owner")),
    "stdlib/lib/gate/tesseract-constitution.px"
  );
  assert_eq!(
    as_str(get(run, "source-target-closure-receipt")),
    "tesseract-macro-ontology-macro-only-target-frontier-closure-proof"
  );
  assert_eq!(
    as_str(get(run, "source-self-capability-map")),
    "tesseract-macro-ontology-internal-self-capability-map"
  );
}

#[test]
fn constitution_gate_blocks_catalog_collapse_modes() {
  let run = eval_fixture();
  let gate = get(run, "constitutionGate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "macro-only-self-operation-catalog"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "operation-catalog-equals-global-runtime",
    "operation-catalog-equals-runtime-api-flattening",
    "operation-catalog-equals-meaning-db",
    "operation-catalog-equals-benchmark-map",
    "operation-catalog-equals-external-solver-intake",
    "operation-catalog-equals-llm-authority",
    "operation-catalog-equals-self-modification",
    "old-host-code-authorizes-operation-catalog",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
}

#[test]
fn operation_catalog_contract_closes_only_operation_catalog_frontier() {
  let run = eval_fixture();
  let contract = get(run, "operation-catalog-contract");
  assert_eq!(
    as_str(get(contract, "proof-id")),
    "proof.macro-only.self-operation-catalog.v1"
  );
  assert!(as_bool(get(contract, "closes-operation-catalog-frontier")));
  assert_eq!(as_i64(get(contract, "operation-count")), 10);
  for key in [
    "closes-benchmark-map",
    "closes-global-runtime",
    "closes-runtime-api-flattening",
    "closes-meaning-db",
    "closes-host-code-removal",
    "imports-external-solver",
    "grants-llm-authority",
    "self-modification",
  ] {
    assert!(!as_bool(get(contract, key)), "`{key}` must stay false");
  }
}

#[test]
fn operation_catalog_contains_ten_reusable_operation_entries() {
  let run = eval_fixture();
  assert!(as_bool(get(run, "operation-catalog-present")));
  assert!(as_bool(get(run, "operation-catalog-proof-only")));
  assert_eq!(as_i64(get(run, "operation-count")), 10);
  let ops = attrs_by_id(get(run, "operations"));
  assert_eq!(ops.len(), 10);
  let fast = ops["op.scoped-fast-path-install"];
  assert_eq!(
    as_str(get(fast, "toolization-state")),
    "scoped-installed-surface-pair"
  );
  assert!(as_bool(get(fast, "installed")));
  assert!(!as_bool(get(fast, "global-runtime")));
  let handoff = ops["op.benchmark-map-handoff"];
  assert_eq!(as_str(get(handoff, "class")), "measurement-frontier");
}

#[test]
fn six_layer_fold_keeps_catalog_runtime_and_audit_separate() {
  let run = eval_fixture();
  let fold = get(run, "six-layer-operation-catalog-fold");
  assert!(as_bool(get(get(fold, "surface"), "visible")));
  assert_eq!(as_i64(get(get(fold, "ontology"), "operation-count")), 10);
  assert!(as_bool(get(
    get(fold, "semantic"),
    "operation-catalog-present"
  )));
  assert!(as_bool(get(get(fold, "gate"), "blocked-self-modification")));
  let runtime = get(fold, "runtime");
  assert!(as_bool(get(runtime, "operation-catalog-present")));
  assert!(!as_bool(get(runtime, "runtime-api-flattening")));
  assert!(!as_bool(get(runtime, "external-solver-installed")));
  assert_eq!(as_i64(get(get(fold, "audit"), "operation-count")), 10);
}

#[test]
fn trials_cover_valid_sources_and_held_boundaries() {
  let run = eval_fixture();
  let trials = attrs_by_id(get(run, "operation-catalog-trials"));
  assert_eq!(trials.len(), 17);
  assert_eq!(
    as_str(get(trials["trial.A.valid-operation-catalog"], "outcome")),
    "self-operation-catalog-present"
  );
  for (id, held) in [
    (
      "trial.E.wrong-proof-id",
      "held.macro-only-self-operation-catalog.proof-id-mismatch",
    ),
    (
      "trial.F.stale-stage",
      "held.macro-only-self-operation-catalog.stale-current-stage",
    ),
    (
      "trial.G.source-mismatch",
      "held.macro-only-self-operation-catalog.source-mismatch",
    ),
    (
      "trial.H.source-frontier-missing",
      "held.macro-only-self-operation-catalog.source-frontier-missing",
    ),
    (
      "trial.I.operation-count-mismatch",
      "held.macro-only-self-operation-catalog.operation-count-mismatch",
    ),
    (
      "trial.J.operation-authority-overclaim",
      "held.macro-only-self-operation-catalog.operation-authority-overclaim",
    ),
    (
      "trial.K.operation-shape-mismatch",
      "held.macro-only-self-operation-catalog.operation-shape-mismatch",
    ),
    (
      "trial.M.benchmark-overclaim",
      "held.macro-only-self-operation-catalog.benchmark-map-overclaim",
    ),
    (
      "trial.N.runtime-overclaim",
      "held.macro-only-self-operation-catalog.runtime-overclaim",
    ),
    (
      "trial.O.command-overclaim",
      "held.macro-only-self-operation-catalog.command-overclaim",
    ),
    (
      "trial.P.authority-overclaim",
      "held.macro-only-self-operation-catalog.authority-overclaim",
    ),
    (
      "trial.Q.gpl-family-dependency",
      "held.macro-only-self-operation-catalog.gpl-family-dependency",
    ),
  ] {
    assert_eq!(as_str(get(trials[id], "held-id")), held, "{id}");
  }
}

#[test]
fn migration_delta_closes_operation_catalog_but_leaves_benchmark_open() {
  let run = eval_fixture();
  let delta = get(run, "migrationDelta");
  let closes = string_set(get(delta, "closes"));
  assert_eq!(closes.len(), 1);
  assert!(closes.contains("need.self.operation-catalog"));
  let not_closed = string_set(get(delta, "does-not-close"));
  assert!(not_closed.contains("need.self.benchmark-map"));
  assert!(not_closed.contains("need.domain-runtime-api-flattening-after-semantic-owner"));
  assert!(not_closed.contains("need.stdlib.meaning-db"));
}

#[test]
fn discoveries_record_d611_through_d618() {
  let run = eval_fixture();
  let discoveries = attrs_by_id(get(run, "discoveries"));
  assert_eq!(discoveries.len(), 8);
  for expected in [
    "D611.self-operation-catalog-lowers-discovered-capabilities-into-reusable-ops",
    "D612.operation-catalog-is-not-runtime-api-flattening",
    "D613.operation-entries-preserve-negative-evidence-and-held-output",
    "D614.operation-catalog-bridges-self-capability-map-to-benchmark-map",
    "D615.p-puck-current-cut-audit-is-cataloged-as-audit-op-not-semantic-owner",
    "D616.scoped-fast-path-install-is-one-operation-not-global-authority",
    "D617.operation-catalog-blocks-external-solver-and-llm-authority-collapse",
    "D618.operation-catalog-keeps-self-extension-mechanical-not-self-modification",
  ] {
    assert!(discoveries.contains_key(expected), "missing {expected}");
  }
}

#[test]
fn top_level_flags_keep_runtime_solver_and_self_modification_false() {
  let run = eval_fixture();
  assert!(as_bool(get(run, "operation-catalog-present")));
  assert!(as_bool(get(run, "operation-catalog-proof-only")));
  assert_eq!(as_i64(get(run, "operation-count")), 10);
  assert!(!as_bool(get(run, "benchmark-map")));
  assert!(as_bool(get(run, "benchmark-map-deferred")));
  for key in [
    "runtime-install",
    "global-ontology-runtime",
    "runtime-api-flattening",
    "meaning-db",
    "host-code-removal-started",
    "implementation-command",
    "external-solver-installed",
    "llm-authority",
    "self-modification",
    "p-puck-is-semantic-owner",
    "old-host-authority",
    "gpl-family-dependencies",
  ] {
    assert!(!as_bool(get(run, key)), "`{key}` must stay false");
  }
}
