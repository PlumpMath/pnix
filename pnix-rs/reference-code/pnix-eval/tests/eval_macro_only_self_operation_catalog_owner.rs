use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/macro-only-self-operation-catalog-owner.px")
}

fn eval_fixture() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = fixture_path();
    let json = std::thread::Builder::new()
      .name("macro-only-self-operation-catalog-owner-eval".to_string())
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("self operation catalog owner")
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
fn owner_fixture_imports_target_closure_source() {
  let run = eval_fixture();
  assert!(as_bool(get(run, "imported-owner")));
  assert!(as_bool(get(run, "used-px-owner")));
  assert_eq!(
    as_str(get(run, "imported-source-proof")),
    "proof.macro-only.target-frontier-closure.v1"
  );
  assert_eq!(
    as_str(get(run, "source-status")),
    "target-frontier-closure-proof-present"
  );
  assert_eq!(as_i64(get(run, "source-target-frontier-closure-count")), 5);
}

#[test]
fn valid_catalog_emits_ten_internal_operations() {
  let run = eval_fixture();
  let proof = get(run, "valid-catalog");
  assert_eq!(
    as_str(get(proof, "status")),
    "self-operation-catalog-present"
  );
  assert!(as_bool(get(proof, "operation-catalog-present")));
  assert!(as_bool(get(proof, "operation-catalog-proof-only")));
  assert_eq!(as_i64(get(proof, "operation-count")), 10);
  assert_eq!(as_list(get(proof, "operations")).len(), 10);
  assert!(string_set(get(proof, "closes")).contains("need.self.operation-catalog"));
}

#[test]
fn operation_catalog_contains_core_meta_interpret_operations() {
  let run = eval_fixture();
  let ops = attrs_by_id(get(run, "operations"));
  assert_eq!(ops.len(), 10);
  for id in [
    "op.surface-to-role-fold",
    "op.role-emission-verdict",
    "op.reverse-replay-delta",
    "op.fixture-local-mutation-loop",
    "op.held-reopen-taxonomy",
    "op.receipt-materialization-chain",
    "op.target-frontier-closure-proof",
    "op.p-puck-current-cut-audit",
    "op.scoped-fast-path-install",
    "op.benchmark-map-handoff",
  ] {
    assert!(ops.contains_key(id), "missing operation `{id}`");
  }
  let ppuck = ops["op.p-puck-current-cut-audit"];
  assert_eq!(as_str(get(ppuck, "class")), "audit");
  assert!(!as_bool(get(ppuck, "p-puck-semantic-owner")));
}

#[test]
fn required_fields_hard_stops_and_evidence_are_explicit() {
  let run = eval_fixture();
  let fields = string_set(get(run, "required-operation-fields"));
  for expected in [
    "id",
    "class",
    "input-shape",
    "output-shape",
    "owner-surface",
    "source-evidence",
    "measurement-hook",
    "hard-stops",
  ] {
    assert!(fields.contains(expected), "missing field `{expected}`");
  }
  let hard_stops = string_set(get(run, "required-operation-hard-stops"));
  for expected in [
    "no-global-runtime-install",
    "no-runtime-api-flattening",
    "no-meaning-db",
    "no-external-solver-intake",
    "no-llm-authority",
    "no-self-modification",
    "no-gpl-family-dependency",
  ] {
    assert!(
      hard_stops.contains(expected),
      "missing hard stop `{expected}`"
    );
  }
  let evidence = string_set(get(run, "required-evidence"));
  assert!(evidence.contains("operation-catalog-frontier-present"));
  assert!(evidence.contains("benchmark-map-deferred"));
}

#[test]
fn held_trials_cover_source_and_shape_failures() {
  let run = eval_fixture();
  for (key, held) in [
    (
      "wrong-proof",
      "held.macro-only-self-operation-catalog.proof-id-mismatch",
    ),
    (
      "stale-stage",
      "held.macro-only-self-operation-catalog.stale-current-stage",
    ),
    (
      "source-mismatch",
      "held.macro-only-self-operation-catalog.source-mismatch",
    ),
    (
      "source-frontier-missing",
      "held.macro-only-self-operation-catalog.source-frontier-missing",
    ),
    (
      "operation-count-mismatch",
      "held.macro-only-self-operation-catalog.operation-count-mismatch",
    ),
    (
      "operation-shape-mismatch",
      "held.macro-only-self-operation-catalog.operation-shape-mismatch",
    ),
    (
      "missing-field",
      "held.macro-only-self-operation-catalog.operation-shape-mismatch",
    ),
  ] {
    let trial = get(run, key);
    assert_eq!(as_str(get(trial, "status")), "Held", "{key}");
    assert_eq!(as_str(get(trial, "held-id")), held, "{key}");
  }
}

#[test]
fn held_trials_block_authority_and_runtime_overclaims() {
  let run = eval_fixture();
  for (key, held) in [
    (
      "operation-authority-overclaim",
      "held.macro-only-self-operation-catalog.operation-authority-overclaim",
    ),
    (
      "benchmark-overclaim",
      "held.macro-only-self-operation-catalog.benchmark-map-overclaim",
    ),
    (
      "runtime-overclaim",
      "held.macro-only-self-operation-catalog.runtime-overclaim",
    ),
    (
      "command-overclaim",
      "held.macro-only-self-operation-catalog.command-overclaim",
    ),
    (
      "authority-overclaim",
      "held.macro-only-self-operation-catalog.authority-overclaim",
    ),
    (
      "gpl-claim",
      "held.macro-only-self-operation-catalog.gpl-family-dependency",
    ),
  ] {
    let trial = get(run, key);
    assert_eq!(as_str(get(trial, "status")), "Held", "{key}");
    assert_eq!(as_str(get(trial, "held-id")), held, "{key}");
  }
}

#[test]
fn catalog_closes_operation_catalog_and_routes_to_benchmark_map() {
  let run = eval_fixture();
  let proof = get(run, "valid-catalog");
  let next = string_set(get(proof, "next-open-frontiers"));
  assert!(next.contains("need.self.benchmark-map"));
  assert!(!next.contains("need.self.operation-catalog"));
  assert!(!as_bool(get(proof, "benchmark-map")));
  assert!(as_bool(get(proof, "benchmark-map-deferred")));
}

#[test]
fn top_level_flags_keep_runtime_solver_and_self_modification_false() {
  let run = eval_fixture();
  assert!(as_bool(get(run, "operation-catalog-present")));
  assert!(as_bool(get(run, "operation-catalog-proof-only")));
  assert_eq!(as_i64(get(run, "operation-count")), 10);
  for key in [
    "benchmark-map",
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
