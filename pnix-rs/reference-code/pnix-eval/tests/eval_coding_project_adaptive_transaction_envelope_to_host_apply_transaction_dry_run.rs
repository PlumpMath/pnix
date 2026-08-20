use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/pnix-query-runtime/coding-project-adaptive-transaction-envelope-to-host-apply-transaction-dry-run.px",
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
  let run = eval_file(&fixture_path()).expect("adaptive transaction dry-run fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "proof")),
    "coding-project-adaptive-transaction-envelope-to-host-apply-transaction-dry-run"
  );
}

#[test]
fn adaptive_transaction_envelope_dry_runs_forward_and_rollback_without_effects() {
  let run = eval_file(&fixture_path()).unwrap();

  let envelope = get(&run, "transaction-envelope");
  assert_eq!(
    as_str(get(envelope, "outcome")),
    "coding-project-host-apply-transaction-envelope-built"
  );
  assert_eq!(
    as_str(get(envelope, "next_gate")),
    "coding-project-host-apply-transaction-dry-run"
  );

  let mirror = get(&run, "mirror-plan");
  assert_eq!(
    as_str(get(mirror, "next_action")),
    "build-coding-project-host-apply-transaction-dry-run"
  );

  let dry_run = get(&run, "transaction-dry-run");
  assert_eq!(
    as_str(get(dry_run, "schema")),
    "puncheetah.code.host-apply-transaction-dry-run.v0"
  );
  assert_eq!(
    as_str(get(dry_run, "outcome")),
    "coding-project-host-apply-transaction-dry-run-passed"
  );
  assert!(as_bool(get(dry_run, "verified")));
  assert!(as_bool(get(dry_run, "transaction_dry_run_passed")));
  assert!(as_bool(get(dry_run, "transaction_envelope_verified")));
  assert!(as_bool(get(dry_run, "mirror_plan_consumed")));
  assert!(as_bool(get(dry_run, "dry_run_only")));
  assert!(as_bool(get(dry_run, "forward_apply_simulated")));
  assert!(as_bool(get(dry_run, "host_operations_verified")));
  assert!(as_bool(get(dry_run, "rollback_handles_verified")));
  assert!(as_bool(get(dry_run, "all_forward_operations_apply")));
  assert!(as_bool(get(dry_run, "rollback_restored_original")));
  assert!(as_bool(get(dry_run, "all_rollback_operations_restore")));
  assert!(as_bool(get(dry_run, "rollback_handle_consumable")));
  assert!(as_bool(get(dry_run, "all_post_apply_hashes_match")));
  assert!(as_bool(get(dry_run, "rollback_ready")));
  assert!(!as_bool(get(dry_run, "rollback_execution_allowed")));
  assert_eq!(
    as_str(get(dry_run, "transaction_id")),
    "coding-project-host-apply-plan:final-approval-adaptive-preview-demo"
  );
  assert_eq!(
    as_str(get(dry_run, "approved_preview_id")),
    "reopened-plan-preview-demo"
  );
  assert_eq!(as_i64(get(dry_run, "edit_count")), 1);
  assert_eq!(
    as_str(get(dry_run, "next_gate")),
    "coding-project-final-file-write-approval-gate"
  );

  assert!(!as_bool(get(dry_run, "host_apply_allowed")));
  assert!(!as_bool(get(dry_run, "file_write_allowed")));
  assert!(!as_bool(get(dry_run, "host_execution_allowed")));
  assert!(!as_bool(get(dry_run, "apply_allowed")));
  assert!(!as_bool(get(dry_run, "raw_eval_allowed")));
  assert!(!as_bool(get(dry_run, "test_execution_allowed")));
  assert!(!as_bool(get(dry_run, "search_execution_allowed")));
  assert!(!as_bool(get(dry_run, "memory_write_allowed")));
  assert!(!as_bool(get(dry_run, "policy_persistence_allowed")));
  assert!(!as_bool(get(dry_run, "source_ingest_allowed")));
  assert!(!as_bool(get(dry_run, "accepted_fact_promotion_allowed")));
  assert!(!as_bool(get(dry_run, "route_update_allowed")));

  let dry_runs = as_list(get(dry_run, "dry_runs"));
  assert_eq!(dry_runs.len(), 1);
  let edit_run = &dry_runs[0];
  assert_eq!(as_str(get(edit_run, "path")), "client/src/request_flow.rs");
  assert!(as_bool(get(edit_run, "edit_contract_ok")));
  assert!(as_bool(get(edit_run, "host_operation_matches_edit")));
  assert!(as_bool(get(edit_run, "rollback_handle_matches_edit")));
  assert!(as_bool(get(edit_run, "file_snapshot_matched")));
  assert!(as_bool(get(edit_run, "forward_apply_simulated")));
  assert!(as_bool(get(edit_run, "post_apply_hash_matches")));
  assert!(as_bool(get(edit_run, "rollback_apply_simulated")));
  assert!(as_bool(get(edit_run, "rollback_restored_original")));
  assert!(as_bool(get(edit_run, "rollback_handle_consumable")));
  assert!(as_bool(get(edit_run, "ok")));

  let receipt = get(dry_run, "receipt");
  assert!(as_bool(get(receipt, "transaction_dry_run_passed")));
  assert!(as_bool(get(receipt, "host_operations_verified")));
  assert!(as_bool(get(receipt, "rollback_handles_verified")));
  assert!(!as_bool(get(receipt, "accepted_fact_promotion_allowed")));
}

#[test]
fn reasoning_dispatch_can_build_transaction_dry_run() {
  let run = eval_file(&fixture_path()).unwrap();
  let dispatched = get(&run, "dispatched-transaction-dry-run");
  assert_eq!(
    as_str(get(dispatched, "op")),
    "build-coding-project-host-apply-transaction-dry-run"
  );

  let result = get(dispatched, "result");
  assert_eq!(
    as_str(get(result, "outcome")),
    "coding-project-host-apply-transaction-dry-run-passed"
  );
  assert!(as_bool(get(result, "transaction_dry_run_passed")));
  assert_eq!(
    as_str(get(result, "next_gate")),
    "coding-project-final-file-write-approval-gate"
  );
}

#[test]
fn missing_snapshot_mismatches_consumed_handle_effect_and_promotion_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let missing_envelope = get(&run, "missing-envelope");
  assert!(as_bool(get(missing_envelope, "is_held")));
  assert_eq!(
    as_str(get(missing_envelope, "outcome")),
    "held-coding-project-host-apply-transaction-envelope-required"
  );

  let missing_snapshot = get(&run, "missing-snapshot");
  assert!(as_bool(get(missing_snapshot, "is_held")));
  assert_eq!(
    as_str(get(missing_snapshot, "outcome")),
    "held-coding-project-host-apply-transaction-dry-run-content-snapshot-required"
  );

  let content_hash = get(&run, "content-hash-mismatch");
  assert!(as_bool(get(content_hash, "is_held")));
  assert_eq!(
    as_str(get(content_hash, "outcome")),
    "held-coding-project-host-apply-transaction-dry-run-simulation-failed"
  );
  let content_hash_runs = as_list(get(content_hash, "dry_runs"));
  assert!(!as_bool(get(
    &content_hash_runs[0],
    "file_snapshot_matched"
  )));

  let forward = get(&run, "forward-missing");
  assert!(as_bool(get(forward, "is_held")));
  assert_eq!(
    as_str(get(forward, "outcome")),
    "held-coding-project-host-apply-transaction-dry-run-simulation-failed"
  );
  let forward_runs = as_list(get(forward, "dry_runs"));
  assert!(!as_bool(get(&forward_runs[0], "forward_apply_simulated")));

  let consumed = get(&run, "consumed-rollback-handle");
  assert!(as_bool(get(consumed, "is_held")));
  assert_eq!(
    as_str(get(consumed, "outcome")),
    "held-coding-project-host-apply-transaction-dry-run-simulation-failed"
  );
  let consumed_runs = as_list(get(consumed, "dry_runs"));
  assert!(!as_bool(get(
    &consumed_runs[0],
    "rollback_handle_matches_edit"
  )));

  let effect = get(&run, "effect-held");
  assert!(as_bool(get(effect, "is_held")));
  assert_eq!(
    as_str(get(effect, "outcome")),
    "held-coding-project-host-apply-transaction-dry-run-effect-blocked"
  );
  assert!(!as_bool(get(effect, "host_apply_allowed")));
  assert!(!as_bool(get(effect, "file_write_allowed")));
  assert!(!as_bool(get(effect, "memory_write_allowed")));

  let promotion = get(&run, "promotion-held");
  assert!(as_bool(get(promotion, "is_held")));
  assert_eq!(
    as_str(get(promotion, "outcome")),
    "held-coding-project-host-apply-transaction-dry-run-effect-blocked"
  );
  assert!(!as_bool(get(promotion, "accepted_fact_promotion_allowed")));
}
