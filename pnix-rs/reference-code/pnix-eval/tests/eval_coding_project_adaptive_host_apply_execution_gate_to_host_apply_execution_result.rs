use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/pnix-query-runtime/coding-project-adaptive-host-apply-execution-gate-to-host-apply-execution-result.px",
  )
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

fn as_i64(v: &Value) -> i64 {
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

#[test]
fn fixture_evaluates_with_pnix_eval_not_nix() {
  let run =
    eval_file(&fixture_path()).expect("adaptive host apply execution result fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "proof")),
    "coding-project-adaptive-host-apply-execution-gate-to-host-apply-execution-result"
  );
}

#[test]
fn host_bridge_execution_result_is_verified_without_additional_effects() {
  let run = eval_file(&fixture_path()).unwrap();

  let gate = get(&run, "host-apply-execution-gate");
  assert_eq!(
    as_str(get(gate, "outcome")),
    "coding-project-host-apply-execution-gate-approved"
  );
  assert!(as_bool(get(
    gate,
    "host_apply_execution_permission_granted"
  )));
  assert!(!as_bool(get(gate, "actual_write_executed")));

  let mirror = get(&run, "mirror-plan");
  assert_eq!(
    as_str(get(mirror, "next_action")),
    "verify-coding-project-host-apply-execution-result"
  );

  let verified = get(&run, "verified");
  assert_eq!(
    as_str(get(verified, "schema")),
    "puncheetah.code.host-apply-execution-result.v0"
  );
  assert_eq!(
    as_str(get(verified, "outcome")),
    "coding-project-host-apply-execution-result-verified"
  );
  assert!(as_bool(get(verified, "verified")));
  assert!(as_bool(get(
    verified,
    "host_bridge_execution_result_verified"
  )));
  assert!(as_bool(get(
    verified,
    "host_apply_execution_permission_verified"
  )));
  assert!(as_bool(get(verified, "mirror_plan_consumed")));
  assert!(as_bool(get(verified, "actual_write_executed")));
  assert!(as_bool(get(verified, "apply_executed")));
  assert!(as_bool(get(verified, "file_write_executed")));
  assert!(as_bool(get(verified, "transactional_write_verified")));
  assert!(as_bool(get(verified, "rollback_handles_ready")));
  assert!(!as_bool(get(verified, "rollback_handle_consumed")));
  assert!(as_bool(get(verified, "post_write_snapshot_required")));
  assert_eq!(
    as_str(get(verified, "transaction_id")),
    "coding-project-host-apply-plan:final-approval-adaptive-preview-demo"
  );
  assert_eq!(
    as_str(get(verified, "approved_preview_id")),
    "reopened-plan-preview-demo"
  );
  assert_eq!(as_i64(get(verified, "edit_count")), 1);
  assert_eq!(
    as_str(get(verified, "next_gate")),
    "coding-project-post-write-verification"
  );

  assert!(!as_bool(get(verified, "host_apply_allowed")));
  assert!(!as_bool(get(verified, "file_write_allowed")));
  assert!(!as_bool(get(verified, "host_execution_allowed")));
  assert!(!as_bool(get(verified, "apply_allowed")));
  assert!(!as_bool(get(verified, "raw_eval_allowed")));
  assert!(!as_bool(get(verified, "test_execution_allowed")));
  assert!(!as_bool(get(verified, "search_execution_allowed")));
  assert!(!as_bool(get(verified, "memory_write_allowed")));
  assert!(!as_bool(get(verified, "policy_persistence_allowed")));
  assert!(!as_bool(get(verified, "source_ingest_allowed")));
  assert!(!as_bool(get(verified, "accepted_fact_promotion_allowed")));
  assert!(!as_bool(get(verified, "route_update_allowed")));

  let targets = as_list(get(verified, "targets"));
  assert_eq!(targets.len(), 1);
  let target = &targets[0];
  assert_eq!(as_str(get(target, "path")), "client/src/request_flow.rs");
  assert_eq!(
    as_str(get(target, "write_mode")),
    "transactional-replace-exact-text"
  );
  assert!(as_bool(get(target, "old_anchor_found")));
  assert!(as_bool(get(target, "replace_exact_text_applied")));
  assert!(as_bool(get(target, "rollback_handle_ready")));
  assert!(!as_bool(get(target, "rollback_handle_consumed")));

  let applied = get(target, "applied_edit");
  assert_eq!(
    as_str(get(applied, "old_text")),
    "let response = client.send(request);"
  );
  assert_eq!(
    as_str(get(applied, "new_text")),
    "let response = client.send(request)?;"
  );
}

#[test]
fn reasoning_dispatch_can_verify_adaptive_host_apply_execution_result() {
  let run = eval_file(&fixture_path()).unwrap();
  let dispatched = get(&run, "dispatched-result");
  assert_eq!(
    as_str(get(dispatched, "op")),
    "verify-coding-project-host-apply-execution-result"
  );

  let result = get(dispatched, "result");
  assert_eq!(
    as_str(get(result, "outcome")),
    "coding-project-host-apply-execution-result-verified"
  );
  assert!(as_bool(get(result, "actual_write_executed")));
  assert_eq!(
    as_str(get(result, "next_gate")),
    "coding-project-post-write-verification"
  );
  assert!(!as_bool(get(result, "file_write_allowed")));
  assert!(!as_bool(get(result, "host_execution_allowed")));
}

#[test]
fn missing_inputs_mismatches_effects_and_promotions_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let missing_gate = get(&run, "missing-gate");
  assert!(as_bool(get(missing_gate, "is_held")));
  assert_eq!(
    as_str(get(missing_gate, "outcome")),
    "held-coding-project-host-apply-execution-gate-required"
  );

  let missing_mirror = get(&run, "missing-mirror");
  assert!(as_bool(get(missing_mirror, "is_held")));
  assert_eq!(
    as_str(get(missing_mirror, "outcome")),
    "held-coding-project-host-apply-execution-result-mirror-plan-required"
  );

  let missing_result = get(&run, "missing-result");
  assert!(as_bool(get(missing_result, "is_held")));
  assert_eq!(
    as_str(get(missing_result, "outcome")),
    "held-coding-project-host-apply-execution-result-required"
  );

  let transaction = get(&run, "transaction-mismatch");
  assert!(as_bool(get(transaction, "is_held")));
  assert_eq!(
    as_str(get(transaction, "outcome")),
    "held-coding-project-host-apply-execution-result-mismatch"
  );

  let target = get(&run, "target-hash-mismatch");
  assert!(as_bool(get(target, "is_held")));
  assert_eq!(
    as_str(get(target, "outcome")),
    "held-coding-project-host-apply-execution-result-mismatch"
  );

  let rollback = get(&run, "rollback-consumed");
  assert!(as_bool(get(rollback, "is_held")));
  assert_eq!(
    as_str(get(rollback, "outcome")),
    "held-coding-project-host-apply-execution-result-required"
  );

  let effect = get(&run, "effect-held");
  assert!(as_bool(get(effect, "is_held")));
  assert_eq!(
    as_str(get(effect, "outcome")),
    "held-coding-project-host-apply-execution-result-effect-blocked"
  );
  assert!(!as_bool(get(effect, "host_apply_allowed")));
  assert!(!as_bool(get(effect, "file_write_allowed")));
  assert!(!as_bool(get(effect, "host_execution_allowed")));
  assert!(!as_bool(get(effect, "source_ingest_allowed")));

  let promotion = get(&run, "promotion-held");
  assert!(as_bool(get(promotion, "is_held")));
  assert_eq!(
    as_str(get(promotion, "outcome")),
    "held-coding-project-host-apply-execution-result-effect-blocked"
  );
  assert!(!as_bool(get(promotion, "accepted_fact_promotion_allowed")));
}
