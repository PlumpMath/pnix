use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/pnix-query-runtime/coding-project-adaptive-final-file-write-approval-to-host-apply-execution-gate.px",
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
    eval_file(&fixture_path()).expect("adaptive host apply execution gate fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "proof")),
    "coding-project-adaptive-final-file-write-approval-to-host-apply-execution-gate"
  );
}

#[test]
fn adaptive_final_write_approval_grants_host_execution_permission_without_writing() {
  let run = eval_file(&fixture_path()).unwrap();

  let final_write = get(&run, "final-write-approval");
  assert_eq!(
    as_str(get(final_write, "outcome")),
    "coding-project-final-file-write-approval-gate-approved"
  );
  assert!(as_bool(get(final_write, "file_write_permission_granted")));
  assert!(as_bool(get(
    final_write,
    "host_apply_execution_gate_allowed"
  )));
  assert!(!as_bool(get(final_write, "actual_write_executed")));

  let mirror = get(&run, "mirror-plan");
  assert_eq!(
    as_str(get(mirror, "next_action")),
    "build-coding-project-host-apply-execution-gate"
  );

  let approved = get(&run, "approved");
  assert_eq!(
    as_str(get(approved, "schema")),
    "puncheetah.code.host-apply-execution-gate.v0"
  );
  assert_eq!(
    as_str(get(approved, "outcome")),
    "coding-project-host-apply-execution-gate-approved"
  );
  assert!(as_bool(get(approved, "verified")));
  assert!(as_bool(get(approved, "host_apply_execution_gate_approved")));
  assert!(as_bool(get(
    approved,
    "host_apply_execution_permission_granted"
  )));
  assert!(as_bool(get(approved, "file_write_permission_verified")));
  assert!(as_bool(get(approved, "mirror_plan_consumed")));
  assert!(as_bool(get(approved, "host_bridge_execution_required")));
  assert!(as_bool(get(
    approved,
    "host_bridge_execution_result_required"
  )));
  assert!(!as_bool(get(approved, "actual_write_executed")));
  assert_eq!(
    as_str(get(approved, "transaction_id")),
    "coding-project-host-apply-plan:final-approval-adaptive-preview-demo"
  );
  assert_eq!(
    as_str(get(approved, "approved_preview_id")),
    "reopened-plan-preview-demo"
  );
  assert_eq!(as_i64(get(approved, "edit_count")), 1);
  assert_eq!(
    as_str(get(approved, "next_gate")),
    "coding-project-host-apply-execution-result"
  );

  assert!(!as_bool(get(approved, "host_apply_allowed")));
  assert!(!as_bool(get(approved, "file_write_allowed")));
  assert!(!as_bool(get(approved, "host_execution_allowed")));
  assert!(!as_bool(get(approved, "apply_allowed")));
  assert!(!as_bool(get(approved, "raw_eval_allowed")));
  assert!(!as_bool(get(approved, "test_execution_allowed")));
  assert!(!as_bool(get(approved, "search_execution_allowed")));
  assert!(!as_bool(get(approved, "memory_write_allowed")));
  assert!(!as_bool(get(approved, "policy_persistence_allowed")));
  assert!(!as_bool(get(approved, "source_ingest_allowed")));
  assert!(!as_bool(get(approved, "accepted_fact_promotion_allowed")));
  assert!(!as_bool(get(approved, "route_update_allowed")));

  let permission = get(approved, "host_apply_execution_permission");
  assert_eq!(
    as_str(get(permission, "permission_kind")),
    "coding-project-host-apply-execution-permission-v0"
  );
  assert_eq!(
    as_str(get(permission, "permission_scope")),
    "host-bridge-execution-result-only"
  );
  assert!(as_bool(get(
    permission,
    "host_apply_execution_permission_granted"
  )));
  assert!(as_bool(get(permission, "host_bridge_execution_required")));
  assert!(as_bool(get(
    permission,
    "host_bridge_execution_result_required"
  )));
  assert!(!as_bool(get(permission, "actual_write_executed")));
  assert!(!as_bool(get(permission, "host_apply_allowed")));
  assert!(!as_bool(get(permission, "file_write_allowed")));
  assert!(!as_bool(get(permission, "source_ingest_allowed")));
  assert!(!as_bool(get(permission, "accepted_fact_promotion_allowed")));
  assert!(!as_bool(get(permission, "route_update_allowed")));
  assert_eq!(
    as_str(get(permission, "next_gate")),
    "coding-project-host-apply-execution-result"
  );

  let targets = as_list(get(permission, "targets"));
  assert_eq!(targets.len(), 1);
  let target = &targets[0];
  assert_eq!(as_str(get(target, "path")), "client/src/request_flow.rs");
  assert_eq!(
    as_str(get(target, "write_mode")),
    "transactional-replace-exact-text"
  );
}

#[test]
fn reasoning_dispatch_can_build_host_apply_execution_gate() {
  let run = eval_file(&fixture_path()).unwrap();
  let dispatched = get(&run, "dispatched-gate");
  assert_eq!(
    as_str(get(dispatched, "op")),
    "build-coding-project-host-apply-execution-gate"
  );

  let result = get(dispatched, "result");
  assert_eq!(
    as_str(get(result, "outcome")),
    "coding-project-host-apply-execution-gate-approved"
  );
  assert!(as_bool(get(
    result,
    "host_apply_execution_permission_granted"
  )));
  assert_eq!(
    as_str(get(result, "next_gate")),
    "coding-project-host-apply-execution-result"
  );
}

#[test]
fn missing_inputs_mismatches_effects_and_promotions_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let missing_final = get(&run, "missing-final-write-approval");
  assert!(as_bool(get(missing_final, "is_held")));
  assert_eq!(
    as_str(get(missing_final, "outcome")),
    "held-coding-project-final-file-write-approval-required"
  );

  let missing_mirror = get(&run, "missing-mirror");
  assert!(as_bool(get(missing_mirror, "is_held")));
  assert_eq!(
    as_str(get(missing_mirror, "outcome")),
    "held-coding-project-host-apply-execution-gate-mirror-plan-required"
  );

  let missing_approval = get(&run, "missing-approval");
  assert!(as_bool(get(missing_approval, "is_held")));
  assert_eq!(
    as_str(get(missing_approval, "outcome")),
    "held-coding-project-host-apply-execution-approval-token-required"
  );

  let transaction = get(&run, "transaction-mismatch");
  assert!(as_bool(get(transaction, "is_held")));
  assert_eq!(
    as_str(get(transaction, "outcome")),
    "held-coding-project-host-apply-execution-approval-token-mismatch"
  );

  let target = get(&run, "target-hash-mismatch");
  assert!(as_bool(get(target, "is_held")));
  assert_eq!(
    as_str(get(target, "outcome")),
    "held-coding-project-host-apply-execution-approval-token-mismatch"
  );

  let rollback = get(&run, "rollback-mismatch");
  assert!(as_bool(get(rollback, "is_held")));
  assert_eq!(
    as_str(get(rollback, "outcome")),
    "held-coding-project-host-apply-execution-approval-token-mismatch"
  );

  let effect = get(&run, "effect-held");
  assert!(as_bool(get(effect, "is_held")));
  assert_eq!(
    as_str(get(effect, "outcome")),
    "held-coding-project-host-apply-execution-gate-effect-blocked"
  );
  assert!(!as_bool(get(effect, "host_apply_allowed")));
  assert!(!as_bool(get(effect, "file_write_allowed")));
  assert!(!as_bool(get(effect, "host_execution_allowed")));
  assert!(!as_bool(get(effect, "source_ingest_allowed")));

  let promotion = get(&run, "promotion-held");
  assert!(as_bool(get(promotion, "is_held")));
  assert_eq!(
    as_str(get(promotion, "outcome")),
    "held-coding-project-host-apply-execution-gate-effect-blocked"
  );
  assert!(!as_bool(get(promotion, "accepted_fact_promotion_allowed")));
}
