use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/coding-project-rollback-ready-receipt.px")
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
  let run = eval_file(&fixture_path())
    .expect("coding project rollback-ready receipt fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "proof")),
    "coding-project-rollback-ready-receipt"
  );

  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "puncheetah.contract.coding-project-rollback-ready-receipt.v0"
  );
  assert_eq!(
    as_str(get(meta, "base")),
    "coding-project-rollback-ready-receipt-v0"
  );
}

#[test]
fn passed_test_receipt_builds_rollback_ready_without_executing_rollback() {
  let run = eval_file(&fixture_path()).unwrap();
  let passed = get(&run, "passed");

  assert_eq!(
    as_str(get(passed, "schema")),
    "puncheetah.code.rollback-ready-receipt.v0"
  );
  assert_eq!(
    as_str(get(passed, "outcome")),
    "coding-project-rollback-ready-receipt-built"
  );
  assert!(as_bool(get(passed, "verified")));
  assert!(as_bool(get(passed, "rollback_ready")));
  assert!(as_bool(get(passed, "rollback_handles_ready")));
  assert!(!as_bool(get(passed, "rollback_handles_consumed")));
  assert!(!as_bool(get(passed, "rollback_execution_allowed")));
  assert!(as_bool(get(passed, "test_execution_receipt_verified")));
  assert!(as_bool(get(passed, "test_passed")));
  assert!(!as_bool(get(passed, "test_failed")));
  assert_eq!(as_i64(get(passed, "edit_count")), 1);
  assert_eq!(
    as_str(get(passed, "next_gate")),
    "coding-project-complete-or-rollback-policy"
  );

  let checks = as_list(get(passed, "target_checks"));
  assert_eq!(checks.len(), 1);
  let check = &checks[0];
  assert!(as_bool(get(check, "target_identity_matches")));
  assert!(as_bool(get(check, "rollback_handle_present")));
  assert!(as_bool(get(check, "rollback_handle_still_ready")));
  assert!(as_bool(get(check, "content_hash_matches")));
  assert!(as_bool(get(check, "rollback_contract_reverse_of_forward")));
  assert!(as_bool(get(check, "ok")));

  assert!(!as_bool(get(passed, "host_apply_allowed")));
  assert!(!as_bool(get(passed, "file_write_allowed")));
  assert!(!as_bool(get(passed, "host_execution_allowed")));
  assert!(!as_bool(get(passed, "apply_allowed")));
  assert!(!as_bool(get(passed, "raw_eval_allowed")));
  assert!(!as_bool(get(passed, "test_execution_allowed")));
  assert!(!as_bool(get(passed, "search_execution_allowed")));
  assert!(!as_bool(get(passed, "memory_write_allowed")));
  assert!(!as_bool(get(passed, "policy_persistence_allowed")));

  let receipt = get(passed, "receipt");
  assert_eq!(
    as_str(get(receipt, "invariant")),
    "host-provided rollback readiness snapshot proves rollback handles are still ready after test evidence; complete-or-rollback policy remains gated"
  );
}

#[test]
fn failed_test_still_builds_rollback_ready_for_policy_decision() {
  let run = eval_file(&fixture_path()).unwrap();
  let failed = get(&run, "failed-test");

  assert_eq!(
    as_str(get(failed, "outcome")),
    "coding-project-rollback-ready-receipt-built"
  );
  assert!(as_bool(get(failed, "verified")));
  assert!(as_bool(get(failed, "rollback_ready")));
  assert!(as_bool(get(failed, "rollback_handles_ready")));
  assert!(!as_bool(get(failed, "rollback_execution_allowed")));
  assert!(!as_bool(get(failed, "test_passed")));
  assert!(as_bool(get(failed, "test_failed")));
  assert_eq!(
    as_str(get(failed, "next_gate")),
    "coding-project-complete-or-rollback-policy"
  );
}

#[test]
fn reasoning_dispatch_can_build_rollback_ready_receipt() {
  let run = eval_file(&fixture_path()).unwrap();
  let dispatched = get(&run, "dispatched");
  assert_eq!(
    as_str(get(dispatched, "op")),
    "build-coding-project-rollback-ready-receipt"
  );
  let result = get(dispatched, "result");
  assert_eq!(
    as_str(get(result, "outcome")),
    "coding-project-rollback-ready-receipt-built"
  );
  assert!(as_bool(get(result, "rollback_ready")));
  assert!(!as_bool(get(result, "rollback_execution_allowed")));
}

#[test]
fn missing_mismatch_consumed_hash_contract_and_effect_requests_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let missing_test = get(&run, "missing-test-receipt");
  assert!(as_bool(get(missing_test, "is_held")));
  assert_eq!(
    as_str(get(missing_test, "outcome")),
    "held-coding-project-rollback-ready-test-receipt-required"
  );

  let missing_mirror = get(&run, "missing-mirror");
  assert!(as_bool(get(missing_mirror, "is_held")));
  assert_eq!(
    as_str(get(missing_mirror, "outcome")),
    "held-coding-project-rollback-ready-mirror-plan-required"
  );

  let missing_snapshot = get(&run, "missing-snapshot");
  assert!(as_bool(get(missing_snapshot, "is_held")));
  assert_eq!(
    as_str(get(missing_snapshot, "outcome")),
    "held-coding-project-rollback-ready-snapshot-required"
  );

  let transaction = get(&run, "transaction-mismatch");
  assert!(as_bool(get(transaction, "is_held")));
  assert_eq!(
    as_str(get(transaction, "outcome")),
    "held-coding-project-rollback-ready-transaction-mismatch"
  );

  let target = get(&run, "target-mismatch");
  assert!(as_bool(get(target, "is_held")));
  assert_eq!(
    as_str(get(target, "outcome")),
    "held-coding-project-rollback-ready-target-mismatch"
  );

  let missing_handle = get(&run, "missing-handle");
  assert!(as_bool(get(missing_handle, "is_held")));
  assert_eq!(
    as_str(get(missing_handle, "outcome")),
    "held-coding-project-rollback-ready-rollback-handle-missing"
  );

  let consumed = get(&run, "consumed-handle");
  assert!(as_bool(get(consumed, "is_held")));
  assert_eq!(
    as_str(get(consumed, "outcome")),
    "held-coding-project-rollback-ready-rollback-handle-consumed"
  );

  let hash = get(&run, "hash-mismatch");
  assert!(as_bool(get(hash, "is_held")));
  assert_eq!(
    as_str(get(hash, "outcome")),
    "held-coding-project-rollback-ready-rollback-hash-mismatch"
  );

  let contract = get(&run, "contract-mismatch");
  assert!(as_bool(get(contract, "is_held")));
  assert_eq!(
    as_str(get(contract, "outcome")),
    "held-coding-project-rollback-ready-contract-mismatch"
  );

  let effect = get(&run, "effect-held");
  assert!(as_bool(get(effect, "is_held")));
  assert_eq!(
    as_str(get(effect, "outcome")),
    "held-coding-project-rollback-ready-effect-blocked"
  );
  assert!(!as_bool(get(effect, "host_apply_allowed")));
  assert!(!as_bool(get(effect, "file_write_allowed")));
  assert!(!as_bool(get(effect, "host_execution_allowed")));
  assert!(!as_bool(get(effect, "test_execution_allowed")));
}
