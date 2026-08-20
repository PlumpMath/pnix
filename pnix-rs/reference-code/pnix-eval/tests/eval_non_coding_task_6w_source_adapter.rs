use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/non-coding-task-6w-source-adapter.px")
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
  let run = eval_file(&fixture_path()).expect("non-coding task 6W fixture evaluates");
  assert_eq!(
    as_str(get(&run, "proof")),
    "non-coding-task-6w-source-adapter"
  );

  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.agent.non-coding-task-6w-source-adapter"
  );
  assert_eq!(
    as_str(get(meta, "base")),
    "non-coding-task-6w-source-adapter-v0"
  );
}

#[test]
fn math_proof_source_lifts_to_universal_reverse_view() {
  let run = eval_file(&fixture_path()).unwrap();
  let source = get(&run, "math-source");

  assert_eq!(
    as_str(get(source, "schema")),
    "puncheetah.non-coding-task-6w-source.v0"
  );
  assert_eq!(
    as_str(get(source, "outcome")),
    "non-coding-task-6w-source-built"
  );
  assert!(as_bool(get(source, "verified")));
  assert_eq!(as_str(get(source, "source_kind")), "math-proof");
  assert_eq!(
    as_str(get(source, "terminal_status")),
    "math-proof-grounded"
  );
  assert!(as_bool(get(source, "six_w_explanation_built")));
  assert_effects_locked(source);

  let six_w = get(source, "ko_audit_summary");
  assert!(as_str(get(six_w, "why")).contains("proof obligation"));
  assert!(as_str(get(six_w, "result")).contains("Mathlib"));

  let frame = get(&run, "math-frame");
  assert_eq!(
    as_str(get(frame, "outcome")),
    "universal-task-6w-frame-built"
  );
  assert_eq!(as_str(get(frame, "task_kind")), "math-proof");
  let chain = as_list(get(frame, "reverse_chain"));
  assert_eq!(chain.len(), 4);
  assert_eq!(
    as_str(get(&chain[0], "evidence_outcome")),
    "grounded-real-mathlib"
  );
  assert_eq!(
    as_str(get(&chain[3], "evidence_outcome")),
    "math-proof-claim-received"
  );
  assert!(as_str(get(frame, "reverse_view_text")).contains("결과에서 원인으로"));
}

#[test]
fn search_verification_source_explains_candidate_without_accepting_evidence() {
  let run = eval_file(&fixture_path()).unwrap();
  let source = get(&run, "search-source");

  assert_eq!(as_str(get(source, "source_kind")), "search-verification");
  assert_eq!(as_str(get(source, "terminal_status")), "search-verified");
  assert!(as_bool(get(source, "verified")));
  assert_effects_locked(source);
  assert!(!as_bool(get(source, "search_execution_allowed")));
  assert!(!as_bool(get(source, "memory_write_allowed")));

  let six_w = get(source, "ko_audit_summary");
  assert!(as_str(get(six_w, "why")).contains("candidate evidence"));
  assert!(as_str(get(six_w, "how")).contains("known verified facts"));

  let frame = get(&run, "search-frame");
  assert_eq!(
    as_str(get(frame, "outcome")),
    "universal-task-6w-frame-built"
  );
  assert_eq!(as_str(get(frame, "terminal_status")), "search-verified");
  let chain = as_list(get(frame, "reverse_chain"));
  assert_eq!(
    as_str(get(&chain[0], "evidence_outcome")),
    "verified-corroborated"
  );
  assert_eq!(
    as_str(get(&chain[3], "evidence_outcome")),
    "search-candidate-claim-received"
  );
}

#[test]
fn project_plan_source_builds_task_6w_and_reverse_view() {
  let run = eval_file(&fixture_path()).unwrap();
  let source = get(&run, "project-source");

  assert_eq!(as_str(get(source, "source_kind")), "project-plan");
  assert_eq!(as_str(get(source, "terminal_status")), "project-plan-ready");
  assert!(as_bool(get(source, "source_6w_adapter_verified")));
  assert_effects_locked(source);

  let frame = get(&run, "project-frame");
  assert_eq!(
    as_str(get(frame, "outcome")),
    "universal-task-6w-frame-built"
  );
  assert_eq!(as_str(get(frame, "task_kind")), "project-plan");
  let chain = as_list(get(frame, "reverse_chain"));
  assert_eq!(chain.len(), 4);
  assert_eq!(
    as_str(get(&chain[0], "evidence_outcome")),
    "project-plan-ready"
  );
}

#[test]
fn geometry_adapter_source_preserves_coordinate_units_loss_and_reverse_view() {
  let run = eval_file(&fixture_path()).unwrap();
  let source = get(&run, "geometry-source");

  assert_eq!(as_str(get(source, "source_kind")), "geometry-adapter");
  assert_eq!(
    as_str(get(source, "terminal_status")),
    "geometry-transform-verified"
  );
  assert!(as_bool(get(source, "verified")));
  assert!(as_bool(get(source, "six_w_explanation_built")));
  assert_effects_locked(source);

  let geometry = get(source, "geometry");
  assert_eq!(
    as_str(get(geometry, "coordinate_system")),
    "right-handed-y-up"
  );
  assert_eq!(as_str(get(geometry, "units")), "meter");
  assert_eq!(as_str(get(geometry, "source_representation")), "x3d-scene");
  assert_eq!(as_str(get(geometry, "target_representation")), "freecat-ir");
  assert_eq!(
    as_str(get(geometry, "transform_kind")),
    "affine-translation"
  );
  assert_eq!(
    as_str(get(geometry, "transform_params")),
    "translate(1, 2, 3)"
  );
  assert_eq!(as_list(get(geometry, "preserved_invariants")).len(), 3);
  assert_eq!(as_list(get(geometry, "lossy_fields")).len(), 0);

  let six_w = get(source, "ko_audit_summary");
  assert!(as_str(get(six_w, "why")).contains("좌표계"));
  assert!(as_str(get(six_w, "why")).contains("loss policy"));
  assert!(as_str(get(six_w, "how")).contains("x3d-scene"));
  assert!(as_str(get(six_w, "how")).contains("freecat-ir"));

  let frame = get(&run, "geometry-frame");
  assert_eq!(
    as_str(get(frame, "outcome")),
    "universal-task-6w-frame-built"
  );
  assert_eq!(as_str(get(frame, "task_kind")), "geometry-adapter");
  assert_eq!(
    as_str(get(frame, "terminal_status")),
    "geometry-transform-verified"
  );
  let chain = as_list(get(frame, "reverse_chain"));
  assert_eq!(chain.len(), 5);
  assert_eq!(
    as_str(get(&chain[0], "evidence_outcome")),
    "geometry-transform-verified"
  );
  assert_eq!(
    as_str(get(&chain[4], "evidence_outcome")),
    "geometry-transform-receipt-received"
  );
  assert!(as_str(get(frame, "reverse_view_text")).contains("결과에서 원인으로"));
}

#[test]
fn dispatch_and_mirror_connect_non_coding_source_to_universal_frame() {
  let run = eval_file(&fixture_path()).unwrap();

  let dispatched = get(&run, "dispatched");
  assert_eq!(
    as_str(get(dispatched, "op")),
    "build-non-coding-task-6w-source"
  );
  let result = get(dispatched, "result");
  assert_eq!(
    as_str(get(result, "outcome")),
    "non-coding-task-6w-source-built"
  );
  assert_eq!(as_str(get(result, "next_gate")), "universal-task-6w-frame");

  let observed = get(&run, "observed-math-source");
  assert_eq!(
    as_str(get(observed, "outcome")),
    "mirror-self-observation-plan-built"
  );
  assert_eq!(
    as_str(get(observed, "next_action")),
    "build-universal-task-6w-frame"
  );
  let ko_self_description = get(observed, "ko_self_description");
  assert!(as_str(get(ko_self_description, "text")).contains("non-coding task 6W source"));
}

#[test]
fn missing_unsupported_incomplete_and_effect_requests_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let missing = get(&run, "missing-source");
  assert!(as_bool(get(missing, "is_held")));
  assert_eq!(
    as_str(get(missing, "outcome")),
    "held-non-coding-task-6w-source-required"
  );

  let unsupported = get(&run, "unsupported-kind");
  assert!(as_bool(get(unsupported, "is_held")));
  assert_eq!(
    as_str(get(unsupported, "outcome")),
    "held-non-coding-task-6w-source-kind-unsupported"
  );

  let missing_math = get(&run, "missing-math-theorem");
  assert!(as_bool(get(missing_math, "is_held")));
  assert_eq!(
    as_str(get(missing_math, "outcome")),
    "held-non-coding-task-6w-math-theorem-required"
  );

  let missing_geometry_coordinate = get(&run, "missing-geometry-coordinate-system");
  assert!(as_bool(get(missing_geometry_coordinate, "is_held")));
  assert_eq!(
    as_str(get(missing_geometry_coordinate, "outcome")),
    "held-geometry-task-6w-source-missing-coordinate-system"
  );

  let missing_geometry_target = get(&run, "missing-geometry-target-representation");
  assert!(as_bool(get(missing_geometry_target, "is_held")));
  assert_eq!(
    as_str(get(missing_geometry_target, "outcome")),
    "held-geometry-task-6w-source-target-representation-mismatch"
  );

  let geometry_transform_unverified = get(&run, "geometry-transform-not-verified");
  assert!(as_bool(get(geometry_transform_unverified, "is_held")));
  assert_eq!(
    as_str(get(geometry_transform_unverified, "outcome")),
    "held-geometry-task-6w-source-transform-not-verified"
  );

  let missing_geometry_loss = get(&run, "missing-geometry-loss-policy");
  assert!(as_bool(get(missing_geometry_loss, "is_held")));
  assert_eq!(
    as_str(get(missing_geometry_loss, "outcome")),
    "held-geometry-task-6w-source-loss-policy-required"
  );

  let effect = get(&run, "effect-held");
  assert!(as_bool(get(effect, "is_held")));
  assert_eq!(
    as_str(get(effect, "outcome")),
    "held-non-coding-task-6w-source-effect-blocked"
  );
  assert_effects_locked(effect);

  let geometry_effect = get(&run, "geometry-effect-held");
  assert!(as_bool(get(geometry_effect, "is_held")));
  assert_eq!(
    as_str(get(geometry_effect, "outcome")),
    "held-non-coding-task-6w-source-effect-blocked"
  );
  assert_effects_locked(geometry_effect);
}
