use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/coding-project-rollback-approval-or-execution-gate.px")
}

fn as_attrs(v: &Value) -> &BTreeMap<String, Value> {
  match v {
    Value::AttrSet(m) => m,
    other => panic!("expected attrset, got {:?}", other),
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
  let run = eval_file(&fixture_path())
    .expect("coding project rollback approval/execution gate fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "proof")),
    "coding-project-rollback-approval-or-execution-gate"
  );

  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "puncheetah.contract.coding-project-rollback-approval-or-execution-gate.v0"
  );
  assert_eq!(
    as_str(get(meta, "base")),
    "coding-project-rollback-approval-or-execution-gate-v0"
  );
}

#[test]
fn rollback_available_policy_grants_host_rollback_permission_without_effects() {
  let run = eval_file(&fixture_path()).unwrap();
  let approved = get(&run, "approved");

  assert_eq!(
    as_str(get(approved, "schema")),
    "puncheetah.code.rollback-approval-or-execution-gate.v0"
  );
  assert_eq!(
    as_str(get(approved, "outcome")),
    "coding-project-rollback-approval-or-execution-gate-approved"
  );
  assert!(as_bool(get(approved, "verified")));
  assert!(as_bool(get(
    approved,
    "rollback_approval_or_execution_gate_approved"
  )));
  assert!(as_bool(get(approved, "rollback_approval_verified")));
  assert!(as_bool(get(
    approved,
    "rollback_execution_permission_granted"
  )));
  assert!(as_bool(get(approved, "host_rollback_execution_required")));
  assert!(as_bool(get(
    approved,
    "host_rollback_execution_result_required"
  )));
  assert!(!as_bool(get(approved, "actual_rollback_executed")));
  assert_eq!(
    as_str(get(approved, "transaction_status")),
    "rollback-execution-permission-ready"
  );
  assert!(!as_bool(get(approved, "test_passed")));
  assert!(as_bool(get(approved, "test_failed")));
  assert!(as_bool(get(approved, "rollback_ready")));
  assert!(as_bool(get(approved, "rollback_handles_ready")));
  assert!(!as_bool(get(approved, "rollback_handles_consumed")));
  assert!(!as_bool(get(approved, "rollback_execution_allowed")));
  assert_eq!(as_i64(get(approved, "edit_count")), 1);
  assert_eq!(
    as_str(get(approved, "next_gate")),
    "coding-project-rollback-execution-result"
  );
  assert_effects_locked(approved);

  let permission = get(approved, "rollback_execution_permission");
  assert_eq!(
    as_str(get(permission, "permission_kind")),
    "coding-project-rollback-execution-permission-v0"
  );
  assert_eq!(
    as_str(get(permission, "permission_scope")),
    "host-rollback-execution-result-only"
  );
  assert!(as_bool(get(
    permission,
    "rollback_execution_permission_granted"
  )));
  assert!(!as_bool(get(permission, "actual_rollback_executed")));
  assert_eq!(
    as_str(get(permission, "next_gate")),
    "coding-project-rollback-execution-result"
  );

  let receipt = get(approved, "receipt");
  assert_eq!(
    as_str(get(receipt, "invariant")),
    "rollback-available policy and explicit approval grant only a host rollback execution permission; actual rollback remains host-bridge evidence"
  );
}

#[test]
fn reasoning_dispatch_can_build_rollback_approval_gate() {
  let run = eval_file(&fixture_path()).unwrap();
  let dispatched = get(&run, "dispatched");
  assert_eq!(
    as_str(get(dispatched, "op")),
    "request-coding-project-rollback-approval-or-execution-gate"
  );
  let result = get(dispatched, "result");
  assert_eq!(
    as_str(get(result, "outcome")),
    "coding-project-rollback-approval-or-execution-gate-approved"
  );
  assert!(as_bool(get(
    result,
    "rollback_execution_permission_granted"
  )));
  assert!(!as_bool(get(result, "actual_rollback_executed")));
}

#[test]
fn complete_branch_missing_mismatch_and_effects_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let missing_policy = get(&run, "missing-policy");
  assert!(as_bool(get(missing_policy, "is_held")));
  assert_eq!(
    as_str(get(missing_policy, "outcome")),
    "held-coding-project-rollback-policy-required"
  );

  let complete_branch = get(&run, "complete-branch");
  assert!(as_bool(get(complete_branch, "is_held")));
  assert_eq!(
    as_str(get(complete_branch, "outcome")),
    "held-coding-project-rollback-complete-policy-not-rollbackable"
  );

  let missing_mirror = get(&run, "missing-mirror");
  assert!(as_bool(get(missing_mirror, "is_held")));
  assert_eq!(
    as_str(get(missing_mirror, "outcome")),
    "held-coding-project-rollback-approval-mirror-plan-required"
  );

  let missing_approval = get(&run, "missing-approval");
  assert!(as_bool(get(missing_approval, "is_held")));
  assert_eq!(
    as_str(get(missing_approval, "outcome")),
    "held-coding-project-rollback-approval-token-required"
  );

  let approval_mismatch = get(&run, "approval-mismatch");
  assert!(as_bool(get(approval_mismatch, "is_held")));
  assert_eq!(
    as_str(get(approval_mismatch, "outcome")),
    "held-coding-project-rollback-approval-token-mismatch"
  );

  let rollback_not_ready = get(&run, "rollback-not-ready");
  assert!(as_bool(get(rollback_not_ready, "is_held")));
  assert_eq!(
    as_str(get(rollback_not_ready, "outcome")),
    "held-coding-project-rollback-policy-not-ready"
  );

  let target_checks = get(&run, "target-checks-invalid");
  assert!(as_bool(get(target_checks, "is_held")));
  assert_eq!(
    as_str(get(target_checks, "outcome")),
    "held-coding-project-rollback-target-checks-invalid"
  );

  let effect = get(&run, "effect-held");
  assert!(as_bool(get(effect, "is_held")));
  assert_eq!(
    as_str(get(effect, "outcome")),
    "held-coding-project-rollback-approval-effect-blocked"
  );
  assert_effects_locked(effect);
}
