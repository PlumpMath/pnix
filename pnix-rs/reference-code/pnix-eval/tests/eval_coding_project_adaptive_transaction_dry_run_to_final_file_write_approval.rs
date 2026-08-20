use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/pnix-query-runtime/coding-project-adaptive-transaction-dry-run-to-final-file-write-approval.px",
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
    eval_file(&fixture_path()).expect("adaptive final file write approval fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "proof")),
    "coding-project-adaptive-transaction-dry-run-to-final-file-write-approval"
  );
}

#[test]
fn adaptive_transaction_dry_run_grants_final_write_permission_without_writing() {
  let run = eval_file(&fixture_path()).unwrap();

  let dry_run = get(&run, "transaction-dry-run");
  assert_eq!(
    as_str(get(dry_run, "outcome")),
    "coding-project-host-apply-transaction-dry-run-passed"
  );
  assert!(as_bool(get(dry_run, "host_operations_verified")));
  assert!(as_bool(get(dry_run, "rollback_handles_verified")));
  assert!(as_bool(get(dry_run, "all_forward_operations_apply")));
  assert!(as_bool(get(dry_run, "all_rollback_operations_restore")));
  assert_eq!(
    as_str(get(dry_run, "next_gate")),
    "coding-project-final-file-write-approval-gate"
  );

  let mirror = get(&run, "mirror-plan");
  assert_eq!(
    as_str(get(mirror, "next_action")),
    "request-coding-project-final-file-write-approval"
  );

  let approved = get(&run, "approved");
  assert_eq!(
    as_str(get(approved, "schema")),
    "puncheetah.code.final-file-write-approval-gate.v0"
  );
  assert_eq!(
    as_str(get(approved, "outcome")),
    "coding-project-final-file-write-approval-gate-approved"
  );
  assert!(as_bool(get(approved, "verified")));
  assert!(as_bool(get(
    approved,
    "final_file_write_approval_gate_approved"
  )));
  assert!(as_bool(get(approved, "file_write_permission_granted")));
  assert!(as_bool(get(approved, "host_apply_execution_gate_allowed")));
  assert!(as_bool(get(approved, "transaction_dry_run_verified")));
  assert!(as_bool(get(approved, "mirror_plan_consumed")));
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
    "coding-project-host-apply-execution-gate"
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

  let permission = get(approved, "file_write_permission");
  assert_eq!(
    as_str(get(permission, "permission_kind")),
    "coding-project-file-write-permission-v0"
  );
  assert_eq!(
    as_str(get(permission, "permission_scope")),
    "host-execution-gate-only"
  );
  assert!(as_bool(get(permission, "file_write_permission_granted")));
  assert!(as_bool(get(
    permission,
    "host_apply_execution_gate_allowed"
  )));
  assert!(!as_bool(get(permission, "actual_write_executed")));
  assert!(!as_bool(get(permission, "file_write_allowed")));
  assert!(!as_bool(get(permission, "source_ingest_allowed")));
  assert!(!as_bool(get(permission, "accepted_fact_promotion_allowed")));
  assert!(!as_bool(get(permission, "route_update_allowed")));
  let targets = as_list(get(permission, "targets"));
  assert_eq!(targets.len(), 1);
  assert_eq!(
    as_str(get(&targets[0], "path")),
    "client/src/request_flow.rs"
  );

  let receipt = get(approved, "receipt");
  assert!(as_bool(get(receipt, "file_write_permission_granted")));
  assert!(as_bool(get(receipt, "host_apply_execution_gate_allowed")));
  assert!(!as_bool(get(receipt, "actual_write_executed")));
  assert!(!as_bool(get(receipt, "accepted_fact_promotion_allowed")));
  assert_eq!(
    as_str(get(receipt, "next_gate")),
    "coding-project-host-apply-execution-gate"
  );
}

#[test]
fn reasoning_dispatch_can_request_final_file_write_approval() {
  let run = eval_file(&fixture_path()).unwrap();
  let dispatched = get(&run, "dispatched-approval");
  assert_eq!(
    as_str(get(dispatched, "op")),
    "request-coding-project-final-file-write-approval"
  );

  let result = get(dispatched, "result");
  assert_eq!(
    as_str(get(result, "outcome")),
    "coding-project-final-file-write-approval-gate-approved"
  );
  assert!(as_bool(get(result, "file_write_permission_granted")));
  assert_eq!(
    as_str(get(result, "next_gate")),
    "coding-project-host-apply-execution-gate"
  );
}

#[test]
fn missing_inputs_mismatches_effects_and_promotions_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let missing_dry_run = get(&run, "missing-dry-run");
  assert!(as_bool(get(missing_dry_run, "is_held")));
  assert_eq!(
    as_str(get(missing_dry_run, "outcome")),
    "held-coding-project-host-apply-transaction-dry-run-required"
  );

  let missing_mirror = get(&run, "missing-mirror");
  assert!(as_bool(get(missing_mirror, "is_held")));
  assert_eq!(
    as_str(get(missing_mirror, "outcome")),
    "held-coding-project-final-file-write-approval-mirror-plan-required"
  );

  let missing_approval = get(&run, "missing-approval");
  assert!(as_bool(get(missing_approval, "is_held")));
  assert_eq!(
    as_str(get(missing_approval, "outcome")),
    "held-coding-project-final-file-write-approval-token-required"
  );

  let transaction = get(&run, "transaction-mismatch");
  assert!(as_bool(get(transaction, "is_held")));
  assert_eq!(
    as_str(get(transaction, "outcome")),
    "held-coding-project-final-file-write-approval-token-mismatch"
  );

  let target = get(&run, "target-hash-mismatch");
  assert!(as_bool(get(target, "is_held")));
  assert_eq!(
    as_str(get(target, "outcome")),
    "held-coding-project-final-file-write-approval-token-mismatch"
  );

  let effect = get(&run, "effect-held");
  assert!(as_bool(get(effect, "is_held")));
  assert_eq!(
    as_str(get(effect, "outcome")),
    "held-coding-project-final-file-write-approval-effect-blocked"
  );
  assert!(!as_bool(get(effect, "host_apply_allowed")));
  assert!(!as_bool(get(effect, "file_write_allowed")));
  assert!(!as_bool(get(effect, "source_ingest_allowed")));

  let promotion = get(&run, "promotion-held");
  assert!(as_bool(get(promotion, "is_held")));
  assert_eq!(
    as_str(get(promotion, "outcome")),
    "held-coding-project-final-file-write-approval-effect-blocked"
  );
  assert!(!as_bool(get(promotion, "accepted_fact_promotion_allowed")));
}
