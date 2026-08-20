use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/coding-project-rollback-complete-receipt.px")
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
    .expect("coding project rollback complete receipt fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "proof")),
    "coding-project-rollback-complete-receipt"
  );

  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "puncheetah.contract.coding-project-rollback-complete-receipt.v0"
  );
  assert_eq!(
    as_str(get(meta, "base")),
    "coding-project-rollback-complete-receipt-v0"
  );
}

#[test]
fn rollback_post_verification_seals_terminal_rollback_complete_without_effects() {
  let run = eval_file(&fixture_path()).unwrap();
  let complete = get(&run, "rollback-complete-receipt");

  assert_eq!(
    as_str(get(complete, "schema")),
    "puncheetah.code.rollback-complete-receipt.v0"
  );
  assert_eq!(
    as_str(get(complete, "outcome")),
    "coding-project-rollback-complete-receipt-built"
  );
  assert!(as_bool(get(complete, "verified")));
  assert!(as_bool(get(complete, "rollback_complete")));
  assert!(as_bool(get(complete, "transaction_closed")));
  assert_eq!(
    as_str(get(complete, "transaction_status")),
    "rollback-complete"
  );
  assert!(as_bool(get(
    complete,
    "rollback_post_verification_verified"
  )));
  assert!(as_bool(get(complete, "rollback_post_verified")));
  assert!(as_bool(get(complete, "rollback_post_snapshot_verified")));
  assert!(as_bool(get(complete, "actual_rollback_executed")));
  assert!(as_bool(get(complete, "rollback_executed")));
  assert!(as_bool(get(complete, "rollback_applied")));
  assert!(as_bool(get(complete, "all_restored_hashes_match")));
  assert!(as_bool(get(complete, "all_forward_text_removed")));
  assert!(as_bool(get(complete, "all_original_text_restored")));
  assert!(as_bool(get(complete, "rollback_handles_consumed")));
  assert!(!as_bool(get(complete, "rollback_execution_allowed")));
  assert_eq!(
    as_str(get(complete, "rollback_policy")),
    "closed-after-rollback"
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
    "rollback-post verification seals the failed transaction rollback-complete after restored state and consumed handles are verified"
  );
}

#[test]
fn reasoning_dispatch_can_build_rollback_complete_receipt() {
  let run = eval_file(&fixture_path()).unwrap();
  let dispatched = get(&run, "dispatched");
  assert_eq!(
    as_str(get(dispatched, "op")),
    "build-coding-project-rollback-complete-receipt"
  );
  let result = get(dispatched, "result");
  assert_eq!(
    as_str(get(result, "outcome")),
    "coding-project-rollback-complete-receipt-built"
  );
  assert!(as_bool(get(result, "rollback_complete")));
  assert!(!as_bool(get(result, "rollback_execution_allowed")));
}

#[test]
fn missing_complete_branch_mismatches_and_effects_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let missing_post = get(&run, "missing-post-verification");
  assert!(as_bool(get(missing_post, "is_held")));
  assert_eq!(
    as_str(get(missing_post, "outcome")),
    "held-coding-project-rollback-complete-receipt-post-verification-required"
  );

  let complete_branch = get(&run, "complete-branch");
  assert!(as_bool(get(complete_branch, "is_held")));
  assert_eq!(
    as_str(get(complete_branch, "outcome")),
    "held-coding-project-rollback-complete-receipt-complete-branch-not-allowed"
  );

  let missing_mirror = get(&run, "missing-mirror");
  assert!(as_bool(get(missing_mirror, "is_held")));
  assert_eq!(
    as_str(get(missing_mirror, "outcome")),
    "held-coding-project-rollback-complete-receipt-mirror-plan-required"
  );

  let not_verified = get(&run, "not-rollback-post-verified");
  assert!(as_bool(get(not_verified, "is_held")));
  assert_eq!(
    as_str(get(not_verified, "outcome")),
    "held-coding-project-rollback-complete-receipt-not-rollback-post-verified"
  );

  let transaction = get(&run, "transaction-mismatch");
  assert!(as_bool(get(transaction, "is_held")));
  assert_eq!(
    as_str(get(transaction, "outcome")),
    "held-coding-project-rollback-complete-receipt-transaction-mismatch"
  );

  let handle = get(&run, "handle-not-consumed");
  assert!(as_bool(get(handle, "is_held")));
  assert_eq!(
    as_str(get(handle, "outcome")),
    "held-coding-project-rollback-complete-receipt-handle-not-consumed"
  );

  let hash = get(&run, "restored-hash-mismatch");
  assert!(as_bool(get(hash, "is_held")));
  assert_eq!(
    as_str(get(hash, "outcome")),
    "held-coding-project-rollback-complete-receipt-restored-hash-mismatch"
  );

  let effect = get(&run, "effect-held");
  assert!(as_bool(get(effect, "is_held")));
  assert_eq!(
    as_str(get(effect, "outcome")),
    "held-coding-project-rollback-complete-receipt-effect-blocked"
  );
  assert_effects_locked(effect);
}
