use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/coding-project-complete-or-rollback-policy.px")
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
    .expect("coding project complete-or-rollback policy fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "proof")),
    "coding-project-complete-or-rollback-policy"
  );

  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "puncheetah.contract.coding-project-complete-or-rollback-policy.v0"
  );
  assert_eq!(
    as_str(get(meta, "base")),
    "coding-project-complete-or-rollback-policy-v0"
  );
}

#[test]
fn passed_test_builds_complete_ready_policy_without_effects() {
  let run = eval_file(&fixture_path()).unwrap();
  let complete = get(&run, "complete-ready");

  assert_eq!(
    as_str(get(complete, "schema")),
    "puncheetah.code.complete-or-rollback-policy.v0"
  );
  assert_eq!(
    as_str(get(complete, "outcome")),
    "coding-project-transaction-complete-policy-built"
  );
  assert!(as_bool(get(complete, "verified")));
  assert!(as_bool(get(complete, "complete_or_rollback_policy_built")));
  assert_eq!(
    as_str(get(complete, "transaction_status")),
    "complete-ready"
  );
  assert!(as_bool(get(complete, "completion_ready")));
  assert!(!as_bool(get(complete, "rollback_available")));
  assert!(!as_bool(get(complete, "rollback_approval_required")));
  assert!(as_bool(get(complete, "rollback_ready")));
  assert!(!as_bool(get(complete, "rollback_execution_allowed")));
  assert!(as_bool(get(complete, "test_execution_receipt_verified")));
  assert!(as_bool(get(complete, "test_passed")));
  assert!(!as_bool(get(complete, "test_failed")));
  assert_eq!(as_i64(get(complete, "edit_count")), 1);
  assert_eq!(
    as_str(get(complete, "next_gate")),
    "coding-project-transaction-complete-receipt"
  );
  assert_effects_locked(complete);

  let policy = get(complete, "policy");
  assert_eq!(
    as_str(get(policy, "policy_kind")),
    "coding-project-complete-or-rollback-policy-v0"
  );
  assert_eq!(as_str(get(policy, "decision")), "complete-ready");
  assert_eq!(
    as_str(get(policy, "next_gate")),
    "coding-project-transaction-complete-receipt"
  );

  let receipt = get(complete, "receipt");
  assert_eq!(
    as_str(get(receipt, "invariant")),
    "verified test execution and rollback-ready evidence choose complete-ready or rollback-available policy without executing completion or rollback"
  );
}

#[test]
fn failed_test_builds_rollback_available_policy_without_executing_rollback() {
  let run = eval_file(&fixture_path()).unwrap();
  let rollback = get(&run, "rollback-available");

  assert_eq!(
    as_str(get(rollback, "outcome")),
    "coding-project-rollback-policy-built"
  );
  assert!(as_bool(get(rollback, "verified")));
  assert_eq!(
    as_str(get(rollback, "transaction_status")),
    "rollback-available"
  );
  assert!(!as_bool(get(rollback, "completion_ready")));
  assert!(as_bool(get(rollback, "rollback_available")));
  assert!(as_bool(get(rollback, "rollback_approval_required")));
  assert!(as_bool(get(rollback, "rollback_ready")));
  assert!(!as_bool(get(rollback, "rollback_execution_allowed")));
  assert!(!as_bool(get(rollback, "test_passed")));
  assert!(as_bool(get(rollback, "test_failed")));
  assert_eq!(
    as_str(get(rollback, "next_gate")),
    "coding-project-rollback-approval-or-execution-gate"
  );
  assert_effects_locked(rollback);

  let policy = get(rollback, "policy");
  assert_eq!(as_str(get(policy, "decision")), "rollback-available");
  assert_eq!(
    as_str(get(policy, "next_gate")),
    "coding-project-rollback-approval-or-execution-gate"
  );
}

#[test]
fn reasoning_dispatch_can_build_complete_or_rollback_policy() {
  let run = eval_file(&fixture_path()).unwrap();
  let dispatched = get(&run, "dispatched");
  assert_eq!(
    as_str(get(dispatched, "op")),
    "build-coding-project-complete-or-rollback-policy"
  );
  let result = get(dispatched, "result");
  assert_eq!(
    as_str(get(result, "outcome")),
    "coding-project-transaction-complete-policy-built"
  );
  assert_eq!(as_str(get(result, "transaction_status")), "complete-ready");
  assert!(!as_bool(get(result, "rollback_execution_allowed")));
}

#[test]
fn missing_mismatch_not_ready_invalid_targets_and_effect_requests_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let missing_ready = get(&run, "missing-rollback-ready");
  assert!(as_bool(get(missing_ready, "is_held")));
  assert_eq!(
    as_str(get(missing_ready, "outcome")),
    "held-coding-project-complete-or-rollback-policy-rollback-ready-required"
  );

  let missing_mirror = get(&run, "missing-mirror");
  assert!(as_bool(get(missing_mirror, "is_held")));
  assert_eq!(
    as_str(get(missing_mirror, "outcome")),
    "held-coding-project-complete-or-rollback-policy-mirror-plan-required"
  );

  let invalid_bool = get(&run, "invalid-test-bool");
  assert!(as_bool(get(invalid_bool, "is_held")));
  assert_eq!(
    as_str(get(invalid_bool, "outcome")),
    "held-coding-project-complete-or-rollback-policy-test-bool-invalid"
  );

  let not_ready = get(&run, "rollback-not-ready");
  assert!(as_bool(get(not_ready, "is_held")));
  assert_eq!(
    as_str(get(not_ready, "outcome")),
    "held-coding-project-complete-or-rollback-policy-rollback-not-ready"
  );

  let transaction = get(&run, "transaction-mismatch");
  assert!(as_bool(get(transaction, "is_held")));
  assert_eq!(
    as_str(get(transaction, "outcome")),
    "held-coding-project-complete-or-rollback-policy-transaction-mismatch"
  );

  let targets = get(&run, "target-checks-invalid");
  assert!(as_bool(get(targets, "is_held")));
  assert_eq!(
    as_str(get(targets, "outcome")),
    "held-coding-project-complete-or-rollback-policy-target-checks-invalid"
  );

  let effect = get(&run, "effect-held");
  assert!(as_bool(get(effect, "is_held")));
  assert_eq!(
    as_str(get(effect, "outcome")),
    "held-coding-project-complete-or-rollback-policy-effect-blocked"
  );
  assert_effects_locked(effect);
}
