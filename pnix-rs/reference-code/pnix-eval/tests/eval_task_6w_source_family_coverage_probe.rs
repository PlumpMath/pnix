use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/task-6w-source-family-coverage-probe.px")
}

fn as_attrs(v: &Value) -> &BTreeMap<String, Value> {
  match v {
    Value::AttrSet(m) => m,
    other => panic!("expected attrset, got {:?}", other),
  }
}

fn as_list(v: &Value) -> &Vec<Value> {
  match v {
    Value::List(items) => items,
    other => panic!("expected list, got {:?}", other),
  }
}

fn as_str(v: &Value) -> &str {
  match v {
    Value::String(s) => s,
    Value::StringContext { text, .. } => text,
    other => panic!("expected string, got {:?}", other),
  }
}

fn as_bool(v: &Value) -> bool {
  match v {
    Value::Bool(b) => *b,
    other => panic!("expected bool, got {:?}", other),
  }
}

fn as_int(v: &Value) -> i64 {
  match v {
    Value::Int(i) => *i,
    other => panic!("expected int, got {:?}", other),
  }
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

fn list_contains_str(v: &Value, needle: &str) -> bool {
  as_list(v).iter().any(|item| as_str(item) == needle)
}

fn row_for_kind<'a>(rows: &'a Value, kind: &str) -> &'a Value {
  as_list(rows)
    .iter()
    .find(|row| as_str(get(row, "source_kind")) == kind)
    .unwrap_or_else(|| panic!("missing coverage row for `{}`", kind))
}

fn assert_effects_locked(v: &Value) {
  assert!(!as_bool(get(v, "host_apply_allowed")));
  assert!(!as_bool(get(v, "file_write_allowed")));
  assert!(!as_bool(get(v, "host_execution_allowed")));
  assert!(!as_bool(get(v, "apply_allowed")));
  assert!(!as_bool(get(v, "raw_eval_allowed")));
  assert!(!as_bool(get(v, "test_execution_allowed")));
  assert!(!as_bool(get(v, "search_execution_allowed")));
  assert!(!as_bool(get(v, "memory_write_allowed")));
  assert!(!as_bool(get(v, "policy_persistence_allowed")));
  assert!(!as_bool(get(v, "source_ingest_allowed")));
  assert!(!as_bool(get(v, "search_evidence_accept_allowed")));
  assert!(!as_bool(get(v, "geometry_execution_allowed")));
  assert!(!as_bool(get(v, "scene_mutation_allowed")));
  assert!(!as_bool(get(v, "blender_execution_allowed")));
  assert!(!as_bool(get(v, "rhino_execution_allowed")));
  assert!(!as_bool(get(v, "freecad_execution_allowed")));
}

#[test]
fn fixture_evaluates_with_pnix_eval_not_nix() {
  let run = eval_file(&fixture_path()).expect("task 6W source family coverage fixture evaluates");
  assert_eq!(
    as_str(get(&run, "proof")),
    "task-6w-source-family-coverage-probe"
  );

  let registry_meta = get(&run, "registry-meta");
  assert_eq!(
    as_str(get(registry_meta, "owner")),
    "stdlib.agent.task-6w-source-family-registry"
  );
  assert_eq!(
    as_str(get(registry_meta, "base")),
    "task-6w-source-family-registry-v0"
  );

  let probe_meta = get(&run, "probe-meta");
  assert_eq!(
    as_str(get(probe_meta, "owner")),
    "stdlib.agent.task-6w-source-family-coverage-probe"
  );
  assert_eq!(
    as_str(get(probe_meta, "base")),
    "task-6w-source-family-coverage-probe-v0"
  );
}

#[test]
fn registry_declares_math_search_project_geometry_families() {
  let run = eval_file(&fixture_path()).unwrap();
  let families = get(&run, "default-source-families");
  assert_eq!(as_list(families).len(), 4);

  let coverage = get(&run, "coverage");
  let kinds = get(coverage, "registered_source_kinds");
  assert!(list_contains_str(kinds, "math-proof"));
  assert!(list_contains_str(kinds, "search-verification"));
  assert!(list_contains_str(kinds, "project-plan"));
  assert!(list_contains_str(kinds, "geometry-adapter"));

  let rows = get(coverage, "coverage_by_source_kind");
  let geometry = row_for_kind(rows, "geometry-adapter");
  assert!(as_bool(get(geometry, "family_contract_valid")));
  assert!(list_contains_str(
    get(geometry, "required_fields"),
    "coordinate_system"
  ));
  assert!(list_contains_str(get(geometry, "required_fields"), "units"));
  assert!(list_contains_str(
    get(geometry, "required_fields"),
    "source_representation"
  ));
  assert!(list_contains_str(
    get(geometry, "held_reasons"),
    "held-geometry-task-6w-source-loss-policy-required"
  ));
  assert!(list_contains_str(
    get(geometry, "terminal_statuses"),
    "geometry-transform-verified"
  ));
  assert!(list_contains_str(
    get(geometry, "evidence_step_outcomes"),
    "geometry-coordinate-system-checked"
  ));
}

#[test]
fn coverage_probe_validates_registry_samples_effect_locks_and_reverse_view() {
  let run = eval_file(&fixture_path()).unwrap();
  let coverage = get(&run, "coverage");

  assert_eq!(
    as_str(get(coverage, "schema")),
    "puncheetah.task-6w-source-family-coverage.v0"
  );
  assert_eq!(
    as_str(get(coverage, "outcome")),
    "task-6w-source-family-coverage-built"
  );
  assert!(as_bool(get(coverage, "verified")));
  assert!(as_bool(get(coverage, "source_family_coverage_built")));
  assert!(as_bool(get(
    coverage,
    "task_6w_source_family_registry_verified"
  )));
  assert_eq!(as_int(get(coverage, "source_family_count")), 4);
  assert!(as_bool(get(coverage, "all_required_fields_declared")));
  assert!(as_bool(get(coverage, "all_held_reasons_declared")));
  assert!(as_bool(get(coverage, "all_effect_locks_declared")));
  assert!(as_bool(get(coverage, "all_reverse_view_shapes_declared")));
  assert!(as_bool(get(coverage, "all_terminal_statuses_declared")));
  assert!(as_bool(get(coverage, "sample_source_coverage_complete")));
  assert!(as_bool(get(
    coverage,
    "reverse_view_sample_coverage_complete"
  )));
  assert_eq!(
    as_str(get(coverage, "next_gate")),
    "coding-expression-adaptation-resilience-plan"
  );
  assert_effects_locked(coverage);

  for row in as_list(get(coverage, "coverage_by_source_kind")) {
    assert!(as_bool(get(row, "sample_source_valid")));
    assert!(as_bool(get(row, "reverse_view_frame_valid")));
    assert!(as_bool(get(row, "effect_locks_declared")));
  }
}

#[test]
fn registry_only_probe_can_audit_declarations_without_samples() {
  let run = eval_file(&fixture_path()).unwrap();
  let coverage = get(&run, "registry-only-coverage");

  assert_eq!(
    as_str(get(coverage, "outcome")),
    "task-6w-source-family-coverage-built"
  );
  assert!(as_bool(get(coverage, "all_required_fields_declared")));
  assert!(!as_bool(get(coverage, "sample_sources_checked")));
  assert!(!as_bool(get(coverage, "universal_frames_checked")));
  assert_eq!(as_int(get(coverage, "source_family_count")), 4);
}

#[test]
fn incomplete_missing_sample_reverse_and_effect_requests_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let missing_kind = get(&run, "missing-kind-held");
  assert!(as_bool(get(missing_kind, "is_held")));
  assert_eq!(
    as_str(get(missing_kind, "outcome")),
    "held-task-6w-source-family-coverage-missing-source-kind"
  );

  let incomplete = get(&run, "incomplete-registry-held");
  assert!(as_bool(get(incomplete, "is_held")));
  assert_eq!(
    as_str(get(incomplete, "outcome")),
    "held-task-6w-source-family-coverage-registry-incomplete"
  );

  let missing_sample = get(&run, "missing-sample-held");
  assert!(as_bool(get(missing_sample, "is_held")));
  assert_eq!(
    as_str(get(missing_sample, "outcome")),
    "held-task-6w-source-family-coverage-sample-source-incomplete"
  );

  let missing_reverse = get(&run, "missing-reverse-held");
  assert!(as_bool(get(missing_reverse, "is_held")));
  assert_eq!(
    as_str(get(missing_reverse, "outcome")),
    "held-task-6w-source-family-coverage-reverse-view-incomplete"
  );

  let effect = get(&run, "effect-held");
  assert!(as_bool(get(effect, "is_held")));
  assert_eq!(
    as_str(get(effect, "outcome")),
    "held-task-6w-source-family-coverage-effect-blocked"
  );
  assert_effects_locked(effect);
}

#[test]
fn dispatch_and_mirror_connect_coverage_to_coding_expression_planning() {
  let run = eval_file(&fixture_path()).unwrap();

  let dispatched = get(&run, "dispatched");
  assert_eq!(
    as_str(get(dispatched, "op")),
    "build-task-6w-source-family-coverage"
  );
  let result = get(dispatched, "result");
  assert_eq!(
    as_str(get(result, "outcome")),
    "task-6w-source-family-coverage-built"
  );

  let observed = get(&run, "observed-coverage");
  assert_eq!(
    as_str(get(observed, "outcome")),
    "mirror-self-observation-plan-built"
  );
  assert_eq!(
    as_str(get(observed, "next_action")),
    "plan-coding-expression-adaptation-resilience"
  );
  let ko_self_description = get(observed, "ko_self_description");
  assert!(as_str(get(ko_self_description, "text")).contains("task-6W source family coverage"));
}
