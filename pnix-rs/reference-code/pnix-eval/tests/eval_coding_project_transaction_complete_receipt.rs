use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/coding-project-transaction-complete-receipt.px")
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
    .expect("coding project transaction complete receipt fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "proof")),
    "coding-project-transaction-complete-receipt"
  );

  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "puncheetah.contract.coding-project-transaction-complete-receipt.v0"
  );
  assert_eq!(
    as_str(get(meta, "base")),
    "coding-project-transaction-complete-receipt-v0"
  );
}

#[test]
fn complete_ready_policy_builds_terminal_complete_receipt_without_effects() {
  let run = eval_file(&fixture_path()).unwrap();
  let complete = get(&run, "complete-receipt");

  assert_eq!(
    as_str(get(complete, "schema")),
    "puncheetah.code.transaction-complete-receipt.v0"
  );
  assert_eq!(
    as_str(get(complete, "outcome")),
    "coding-project-transaction-complete-receipt-built"
  );
  assert!(as_bool(get(complete, "verified")));
  assert!(as_bool(get(complete, "transaction_complete")));
  assert!(as_bool(get(complete, "transaction_closed")));
  assert_eq!(as_str(get(complete, "transaction_status")), "complete");
  assert!(as_bool(get(complete, "completion_ready_policy_verified")));
  assert!(as_bool(get(complete, "test_passed")));
  assert!(!as_bool(get(complete, "test_failed")));
  assert!(as_bool(get(complete, "rollback_ready_at_completion")));
  assert!(as_bool(get(complete, "rollback_ready")));
  assert!(as_bool(get(complete, "rollback_handles_ready")));
  assert!(!as_bool(get(complete, "rollback_handles_consumed")));
  assert!(!as_bool(get(complete, "rollback_execution_allowed")));
  assert_eq!(
    as_str(get(complete, "rollback_policy")),
    "locked-after-complete"
  );
  assert_eq!(as_i64(get(complete, "edit_count")), 1);
  assert_eq!(
    as_str(get(complete, "next_gate")),
    "pnix-db-transaction-timeline-close-or-audit"
  );
  assert_effects_locked(complete);

  let receipt = get(complete, "receipt");
  assert_eq!(
    as_str(get(receipt, "invariant")),
    "complete-ready policy with passed test evidence seals the transaction complete while rollback remains ready but policy-locked"
  );
}

#[test]
fn reasoning_dispatch_can_build_transaction_complete_receipt() {
  let run = eval_file(&fixture_path()).unwrap();
  let dispatched = get(&run, "dispatched");
  assert_eq!(
    as_str(get(dispatched, "op")),
    "build-coding-project-transaction-complete-receipt"
  );
  let result = get(dispatched, "result");
  assert_eq!(
    as_str(get(result, "outcome")),
    "coding-project-transaction-complete-receipt-built"
  );
  assert!(as_bool(get(result, "transaction_complete")));
  assert!(!as_bool(get(result, "rollback_execution_allowed")));
}

#[test]
fn rollback_policy_and_invalid_complete_evidence_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let missing_policy = get(&run, "missing-policy");
  assert!(as_bool(get(missing_policy, "is_held")));
  assert_eq!(
    as_str(get(missing_policy, "outcome")),
    "held-coding-project-transaction-complete-policy-required"
  );

  let rollback_branch = get(&run, "rollback-branch");
  assert!(as_bool(get(rollback_branch, "is_held")));
  assert_eq!(
    as_str(get(rollback_branch, "outcome")),
    "held-coding-project-transaction-complete-not-complete-policy"
  );

  let missing_mirror = get(&run, "missing-mirror");
  assert!(as_bool(get(missing_mirror, "is_held")));
  assert_eq!(
    as_str(get(missing_mirror, "outcome")),
    "held-coding-project-transaction-complete-mirror-plan-required"
  );

  let test_not_passed = get(&run, "test-not-passed");
  assert!(as_bool(get(test_not_passed, "is_held")));
  assert_eq!(
    as_str(get(test_not_passed, "outcome")),
    "held-coding-project-transaction-complete-test-not-passed"
  );

  let rollback_not_ready = get(&run, "rollback-not-ready");
  assert!(as_bool(get(rollback_not_ready, "is_held")));
  assert_eq!(
    as_str(get(rollback_not_ready, "outcome")),
    "held-coding-project-transaction-complete-rollback-not-ready"
  );

  let policy_mismatch = get(&run, "policy-mismatch");
  assert!(as_bool(get(policy_mismatch, "is_held")));
  assert_eq!(
    as_str(get(policy_mismatch, "outcome")),
    "held-coding-project-transaction-complete-policy-mismatch"
  );

  let target_checks = get(&run, "target-checks-invalid");
  assert!(as_bool(get(target_checks, "is_held")));
  assert_eq!(
    as_str(get(target_checks, "outcome")),
    "held-coding-project-transaction-complete-target-checks-invalid"
  );

  let effect = get(&run, "effect-held");
  assert!(as_bool(get(effect, "is_held")));
  assert_eq!(
    as_str(get(effect, "outcome")),
    "held-coding-project-transaction-complete-effect-blocked"
  );
  assert_effects_locked(effect);
}
