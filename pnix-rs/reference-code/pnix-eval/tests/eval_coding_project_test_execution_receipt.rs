use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/coding-project-test-execution-receipt.px")
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

#[test]
fn fixture_evaluates_with_pnix_eval_not_nix() {
  let run = eval_file(&fixture_path())
    .expect("coding project test execution receipt fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "proof")),
    "coding-project-test-execution-receipt"
  );

  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "puncheetah.contract.coding-project-test-execution-receipt.v0"
  );
  assert_eq!(
    as_str(get(meta, "base")),
    "coding-project-test-execution-receipt-v0"
  );
}

#[test]
fn passed_host_test_receipt_is_verified_without_running_tests_again() {
  let run = eval_file(&fixture_path()).unwrap();
  let passed = get(&run, "passed");

  assert_eq!(
    as_str(get(passed, "schema")),
    "puncheetah.code.test-execution-receipt.v0"
  );
  assert_eq!(
    as_str(get(passed, "outcome")),
    "coding-project-test-execution-receipt-verified"
  );
  assert!(as_bool(get(passed, "verified")));
  assert!(as_bool(get(passed, "test_execution_receipt_verified")));
  assert!(as_bool(get(passed, "test_executed")));
  assert!(as_bool(get(passed, "test_passed")));
  assert!(!as_bool(get(passed, "test_failed")));
  assert!(as_bool(get(passed, "post_write_verified")));
  assert!(as_bool(get(passed, "test_plan_verified")));
  assert!(as_bool(get(
    passed,
    "host_bridge_test_execution_result_verified"
  )));
  assert!(as_bool(get(passed, "test_command_approved")));
  assert!(as_bool(get(passed, "test_command_allowlisted")));
  assert!(as_bool(get(passed, "test_command_bounded")));
  assert!(as_bool(get(passed, "test_command_no_drift")));
  assert!(as_bool(get(passed, "rollback_ready_required")));
  assert_eq!(as_i64(get(passed, "exit_status")), 0);
  assert_eq!(as_i64(get(passed, "duration_ms")), 42);
  assert_eq!(
    as_str(get(passed, "next_gate")),
    "coding-project-rollback-ready-receipt"
  );

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
    "host-provided test execution receipt matches the verified post-write transaction and approved post-apply test command; rollback readiness remains required before completion"
  );
}

#[test]
fn failed_host_test_is_still_a_verified_test_receipt() {
  let run = eval_file(&fixture_path()).unwrap();
  let failed = get(&run, "failed-test");

  assert_eq!(
    as_str(get(failed, "outcome")),
    "coding-project-test-execution-receipt-verified"
  );
  assert!(as_bool(get(failed, "verified")));
  assert!(as_bool(get(failed, "test_execution_receipt_verified")));
  assert!(as_bool(get(failed, "test_executed")));
  assert!(!as_bool(get(failed, "test_passed")));
  assert!(as_bool(get(failed, "test_failed")));
  assert_eq!(as_i64(get(failed, "exit_status")), 1);
  assert!(as_bool(get(failed, "rollback_ready_required")));
  assert_eq!(
    as_str(get(failed, "next_gate")),
    "coding-project-rollback-ready-receipt"
  );
  assert!(!as_bool(get(failed, "test_execution_allowed")));
  assert!(!as_bool(get(failed, "host_execution_allowed")));
}

#[test]
fn reasoning_dispatch_can_build_test_execution_receipt() {
  let run = eval_file(&fixture_path()).unwrap();
  let dispatched = get(&run, "dispatched");
  assert_eq!(
    as_str(get(dispatched, "op")),
    "build-coding-project-test-execution-receipt"
  );
  let result = get(dispatched, "result");
  assert_eq!(
    as_str(get(result, "outcome")),
    "coding-project-test-execution-receipt-verified"
  );
  assert!(as_bool(get(result, "test_execution_receipt_verified")));
  assert!(as_bool(get(result, "test_passed")));
  assert!(!as_bool(get(result, "test_execution_allowed")));
}

#[test]
fn missing_mismatch_and_effect_requests_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let missing_post_write = get(&run, "missing-post-write");
  assert!(as_bool(get(missing_post_write, "is_held")));
  assert_eq!(
    as_str(get(missing_post_write, "outcome")),
    "held-coding-project-test-execution-post-write-required"
  );

  let missing_mirror = get(&run, "missing-mirror");
  assert!(as_bool(get(missing_mirror, "is_held")));
  assert_eq!(
    as_str(get(missing_mirror, "outcome")),
    "held-coding-project-test-execution-mirror-plan-required"
  );

  let missing_plan = get(&run, "missing-plan");
  assert!(as_bool(get(missing_plan, "is_held")));
  assert_eq!(
    as_str(get(missing_plan, "outcome")),
    "held-coding-project-test-execution-plan-required"
  );

  let missing_result = get(&run, "missing-result");
  assert!(as_bool(get(missing_result, "is_held")));
  assert_eq!(
    as_str(get(missing_result, "outcome")),
    "held-coding-project-test-execution-receipt-required"
  );

  let transaction = get(&run, "transaction-mismatch");
  assert!(as_bool(get(transaction, "is_held")));
  assert_eq!(
    as_str(get(transaction, "outcome")),
    "held-coding-project-test-result-transaction-mismatch"
  );

  let command_drift = get(&run, "command-drift");
  assert!(as_bool(get(command_drift, "is_held")));
  assert_eq!(
    as_str(get(command_drift, "outcome")),
    "held-coding-project-test-command-drift"
  );

  let not_allowlisted = get(&run, "command-not-allowlisted");
  assert!(as_bool(get(not_allowlisted, "is_held")));
  assert_eq!(
    as_str(get(not_allowlisted, "outcome")),
    "held-coding-project-test-command-not-allowlisted"
  );

  let missing_exit = get(&run, "missing-exit-status");
  assert!(as_bool(get(missing_exit, "is_held")));
  assert_eq!(
    as_str(get(missing_exit, "outcome")),
    "held-coding-project-test-result-missing-exit-status"
  );

  let missing_output = get(&run, "missing-output-summary");
  assert!(as_bool(get(missing_output, "is_held")));
  assert_eq!(
    as_str(get(missing_output, "outcome")),
    "held-coding-project-test-result-missing-output-summary"
  );

  let effect = get(&run, "effect-held");
  assert!(as_bool(get(effect, "is_held")));
  assert_eq!(
    as_str(get(effect, "outcome")),
    "held-coding-project-test-execution-effect-blocked"
  );
  assert!(!as_bool(get(effect, "host_apply_allowed")));
  assert!(!as_bool(get(effect, "file_write_allowed")));
  assert!(!as_bool(get(effect, "test_execution_allowed")));
  assert!(!as_bool(get(effect, "search_execution_allowed")));
}
