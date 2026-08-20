use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/universal-task-6w-frame.px")
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
}

#[test]
fn fixture_evaluates_with_pnix_eval_not_nix() {
  let run = eval_file(&fixture_path()).expect("universal task 6W fixture evaluates");
  assert_eq!(as_str(get(&run, "proof")), "universal-task-6w-frame");

  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.agent.universal-task-6w-frame"
  );
  assert_eq!(as_str(get(meta, "base")), "universal-task-6w-frame-v0");
}

#[test]
fn complete_korean_6w_explanation_lifts_to_universal_task_frame() {
  let run = eval_file(&fixture_path()).unwrap();
  let frame = get(&run, "complete-frame");

  assert_eq!(
    as_str(get(frame, "schema")),
    "puncheetah.universal-task-6w-frame.v0"
  );
  assert_eq!(
    as_str(get(frame, "outcome")),
    "universal-task-6w-frame-built"
  );
  assert!(as_bool(get(frame, "verified")));
  assert!(as_bool(get(frame, "universal_task_6w_built")));
  assert!(as_bool(get(frame, "reverse_view_grounded")));
  assert!(as_bool(get(frame, "source_6w_explanation_verified")));
  assert_eq!(as_str(get(frame, "task_kind")), "coding-transaction");
  assert_eq!(as_str(get(frame, "terminal_status")), "complete");
  assert_effects_locked(frame);

  let task_6w = get(frame, "task_6w");
  assert!(as_str(get(task_6w, "what")).contains("테스트 통과"));
  assert!(as_str(get(task_6w, "why")).contains("complete-ready policy"));
  assert!(as_str(get(task_6w, "result")).contains("complete"));

  let chain = as_list(get(frame, "reverse_chain"));
  assert_eq!(chain.len(), 6);
  assert_eq!(
    as_str(get(&chain[0], "evidence_outcome")),
    "coding-project-transaction-complete-receipt-built"
  );
  assert_eq!(
    as_str(get(&chain[5], "evidence_outcome")),
    "coding-project-host-apply-execution-result-verified"
  );
  assert!(as_str(get(frame, "reverse_view_text")).contains("결과에서 원인으로"));

  let atoms = as_list(get(frame, "canonical_task_atoms"));
  assert!(atoms.len() >= 7);
  assert_eq!(as_str(get(&atoms[0], "kind")), "task-6w");
  assert_eq!(as_str(get(&atoms[4], "slot")), "why");
}

#[test]
fn rollback_task_frame_reverse_view_walks_from_terminal_result_to_test_evidence() {
  let run = eval_file(&fixture_path()).unwrap();
  let frame = get(&run, "rollback-frame");

  assert_eq!(
    as_str(get(frame, "outcome")),
    "universal-task-6w-frame-built"
  );
  assert_eq!(as_str(get(frame, "terminal_status")), "rollback-complete");
  assert!(as_bool(get(frame, "reverse_view_grounded")));
  assert_effects_locked(frame);

  let chain = as_list(get(frame, "reverse_chain"));
  assert_eq!(chain.len(), 9);
  assert_eq!(
    as_str(get(&chain[0], "evidence_outcome")),
    "coding-project-rollback-complete-receipt-built"
  );
  assert_eq!(
    as_str(get(&chain[1], "evidence_outcome")),
    "coding-project-rollback-post-verification-passed"
  );
  assert_eq!(
    as_str(get(&chain[4], "evidence_outcome")),
    "coding-project-rollback-policy-built"
  );
  assert_eq!(
    as_str(get(&chain[6], "evidence_outcome")),
    "coding-project-test-execution-receipt-verified"
  );
  assert!(as_str(get(&chain[0], "because")).contains("rollback 이후 snapshot"));
  assert!(as_str(get(frame, "focused_answer_ko")).contains("reverseView"));
}

#[test]
fn reasoning_dispatch_can_build_universal_task_6w_frame() {
  let run = eval_file(&fixture_path()).unwrap();
  let dispatched = get(&run, "dispatched");
  assert_eq!(
    as_str(get(dispatched, "op")),
    "build-universal-task-6w-frame"
  );

  let result = get(dispatched, "result");
  assert_eq!(
    as_str(get(result, "outcome")),
    "universal-task-6w-frame-built"
  );
  assert!(as_bool(get(result, "universal_task_6w_built")));
  assert!(as_bool(get(result, "reverse_view_grounded")));
  assert_eq!(as_str(get(result, "terminal_status")), "rollback-complete");
}

#[test]
fn missing_invalid_incomplete_and_effect_requests_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let missing = get(&run, "missing-source");
  assert!(as_bool(get(missing, "is_held")));
  assert_eq!(
    as_str(get(missing, "outcome")),
    "held-universal-task-6w-source-required"
  );

  let invalid = get(&run, "invalid-source");
  assert!(as_bool(get(invalid, "is_held")));
  assert_eq!(
    as_str(get(invalid, "outcome")),
    "held-universal-task-6w-invalid-source"
  );

  let missing_slots = get(&run, "missing-slots");
  assert!(as_bool(get(missing_slots, "is_held")));
  assert_eq!(
    as_str(get(missing_slots, "outcome")),
    "held-universal-task-6w-slots-required"
  );

  let missing_steps = get(&run, "missing-evidence-steps");
  assert!(as_bool(get(missing_steps, "is_held")));
  assert_eq!(
    as_str(get(missing_steps, "outcome")),
    "held-universal-task-6w-evidence-steps-required"
  );

  let effect = get(&run, "effect-held");
  assert!(as_bool(get(effect, "is_held")));
  assert_eq!(
    as_str(get(effect, "outcome")),
    "held-universal-task-6w-effect-blocked"
  );
  assert_effects_locked(effect);
}
