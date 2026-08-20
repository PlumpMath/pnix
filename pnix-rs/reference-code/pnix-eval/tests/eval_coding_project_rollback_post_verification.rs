use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/coding-project-rollback-post-verification.px")
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
    .expect("coding project rollback-post verification fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "proof")),
    "coding-project-rollback-post-verification"
  );

  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "puncheetah.contract.coding-project-rollback-post-verification.v0"
  );
  assert_eq!(
    as_str(get(meta, "base")),
    "coding-project-rollback-post-verification-v0"
  );
}

#[test]
fn rollback_post_snapshot_is_verified_without_additional_effects() {
  let run = eval_file(&fixture_path()).unwrap();
  let passed = get(&run, "passed");

  assert_eq!(
    as_str(get(passed, "schema")),
    "puncheetah.code.rollback-post-verification.v0"
  );
  assert_eq!(
    as_str(get(passed, "outcome")),
    "coding-project-rollback-post-verification-passed"
  );
  assert!(as_bool(get(passed, "verified")));
  assert!(as_bool(get(passed, "rollback_post_verified")));
  assert!(as_bool(get(passed, "rollback_post_snapshot_verified")));
  assert!(as_bool(get(
    passed,
    "host_rollback_execution_result_verified"
  )));
  assert!(as_bool(get(passed, "mirror_plan_consumed")));
  assert!(as_bool(get(passed, "actual_rollback_executed")));
  assert!(as_bool(get(passed, "rollback_executed")));
  assert!(as_bool(get(passed, "rollback_applied")));
  assert!(as_bool(get(passed, "all_restored_hashes_match")));
  assert!(as_bool(get(passed, "all_forward_text_removed")));
  assert!(as_bool(get(passed, "all_original_text_restored")));
  assert!(as_bool(get(passed, "rollback_handles_consumed")));
  assert_eq!(
    as_str(get(passed, "transaction_status")),
    "rollback-post-verified"
  );
  assert_eq!(as_i64(get(passed, "edit_count")), 1);
  assert_eq!(
    as_str(get(passed, "next_gate")),
    "coding-project-rollback-complete-receipt"
  );
  assert_effects_locked(passed);

  let checks = as_list(get(passed, "target_checks"));
  assert_eq!(checks.len(), 1);
  let check = &checks[0];
  assert_eq!(as_str(get(check, "path")), "generic/subject.artifact");
  assert!(as_bool(get(check, "snapshot_target_found")));
  assert!(as_bool(get(check, "target_identity_matches")));
  assert!(as_bool(get(check, "restored_hash_matches")));
  assert!(as_bool(get(check, "original_text_restored_at_anchor")));
  assert!(as_bool(get(check, "forward_text_removed_at_anchor")));
  assert!(as_bool(get(check, "rollback_handle_consumed")));
  assert!(as_bool(get(check, "ok")));

  let receipt = get(passed, "receipt");
  assert_eq!(
    as_str(get(receipt, "invariant")),
    "host-provided rollback-post snapshot matches verified rollback execution result targets, restored hashes, original anchors, and consumed rollback handles; rollback-complete receipt remains gated"
  );
}

#[test]
fn reasoning_dispatch_can_build_rollback_post_verification() {
  let run = eval_file(&fixture_path()).unwrap();
  let dispatched = get(&run, "dispatched");
  assert_eq!(
    as_str(get(dispatched, "op")),
    "build-coding-project-rollback-post-verification"
  );
  let result = get(dispatched, "result");
  assert_eq!(
    as_str(get(result, "outcome")),
    "coding-project-rollback-post-verification-passed"
  );
  assert!(as_bool(get(result, "rollback_post_verified")));
  assert!(!as_bool(get(result, "host_execution_allowed")));
  assert!(!as_bool(get(result, "file_write_allowed")));
  assert!(!as_bool(get(result, "test_execution_allowed")));
}

#[test]
fn missing_inputs_mismatches_and_effect_requests_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let missing_result = get(&run, "missing-result");
  assert!(as_bool(get(missing_result, "is_held")));
  assert_eq!(
    as_str(get(missing_result, "outcome")),
    "held-coding-project-rollback-post-verification-execution-result-required"
  );

  let missing_mirror = get(&run, "missing-mirror");
  assert!(as_bool(get(missing_mirror, "is_held")));
  assert_eq!(
    as_str(get(missing_mirror, "outcome")),
    "held-coding-project-rollback-post-verification-mirror-plan-required"
  );

  let missing_snapshot = get(&run, "missing-snapshot");
  assert!(as_bool(get(missing_snapshot, "is_held")));
  assert_eq!(
    as_str(get(missing_snapshot, "outcome")),
    "held-coding-project-rollback-post-verification-snapshot-required"
  );

  let transaction = get(&run, "transaction-mismatch");
  assert!(as_bool(get(transaction, "is_held")));
  assert_eq!(
    as_str(get(transaction, "outcome")),
    "held-coding-project-rollback-post-verification-transaction-mismatch"
  );

  let restore_hash = get(&run, "restore-hash-mismatch");
  assert!(as_bool(get(restore_hash, "is_held")));
  assert_eq!(
    as_str(get(restore_hash, "outcome")),
    "held-coding-project-rollback-post-verification-restore-hash-mismatch"
  );

  let forward = get(&run, "forward-text-still-present");
  assert!(as_bool(get(forward, "is_held")));
  assert_eq!(
    as_str(get(forward, "outcome")),
    "held-coding-project-rollback-post-verification-forward-text-still-present"
  );

  let original = get(&run, "original-text-not-restored");
  assert!(as_bool(get(original, "is_held")));
  assert_eq!(
    as_str(get(original, "outcome")),
    "held-coding-project-rollback-post-verification-original-text-not-restored"
  );

  let handle = get(&run, "handle-not-consumed");
  assert!(as_bool(get(handle, "is_held")));
  assert_eq!(
    as_str(get(handle, "outcome")),
    "held-coding-project-rollback-post-verification-handle-not-consumed"
  );

  let effect = get(&run, "effect-held");
  assert!(as_bool(get(effect, "is_held")));
  assert_eq!(
    as_str(get(effect, "outcome")),
    "held-coding-project-rollback-post-verification-effect-blocked"
  );
  assert_effects_locked(effect);
}
