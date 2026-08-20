use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/pnix-query-runtime/coding-project-adaptive-host-apply-execution-result-to-post-write-verification.px",
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
  let run = eval_file(&fixture_path())
    .expect("adaptive post-write verification smoke fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "proof")),
    "coding-project-adaptive-host-apply-execution-result-to-post-write-verification"
  );
}

#[test]
fn post_write_snapshot_is_verified_from_adaptive_host_execution_result() {
  let run = eval_file(&fixture_path()).unwrap();

  let result = get(&run, "host-apply-execution-result");
  assert_eq!(
    as_str(get(result, "outcome")),
    "coding-project-host-apply-execution-result-verified"
  );
  assert!(as_bool(get(result, "actual_write_executed")));
  assert!(as_bool(get(result, "post_write_snapshot_required")));

  let mirror = get(&run, "mirror-plan");
  assert_eq!(
    as_str(get(mirror, "next_action")),
    "build-coding-project-post-write-verification"
  );

  let snapshot = get(&run, "post-write-snapshot");
  assert_eq!(
    as_str(get(snapshot, "kind")),
    "coding-project-post-write-snapshot-v0"
  );
  assert!(as_bool(get(snapshot, "verified")));

  let verified = get(&run, "verified");
  assert_eq!(
    as_str(get(verified, "schema")),
    "puncheetah.code.post-write-verification.v0"
  );
  assert_eq!(
    as_str(get(verified, "outcome")),
    "coding-project-post-write-verification-passed"
  );
  assert!(as_bool(get(verified, "verified")));
  assert!(as_bool(get(verified, "post_write_verified")));
  assert!(as_bool(get(verified, "post_write_snapshot_verified")));
  assert!(as_bool(get(
    verified,
    "host_apply_execution_result_verified"
  )));
  assert!(as_bool(get(verified, "mirror_plan_consumed")));
  assert!(as_bool(get(verified, "actual_write_executed")));
  assert!(as_bool(get(verified, "all_target_hashes_match")));
  assert!(as_bool(get(verified, "all_new_text_at_target_anchors")));
  assert!(as_bool(get(
    verified,
    "all_old_text_removed_at_target_anchors"
  )));
  assert!(as_bool(get(verified, "rollback_handles_still_ready")));
  assert!(!as_bool(get(verified, "rollback_execution_allowed")));
  assert_eq!(
    as_str(get(verified, "transaction_id")),
    "coding-project-host-apply-plan:final-approval-adaptive-preview-demo"
  );
  assert_eq!(
    as_str(get(verified, "approved_preview_id")),
    "reopened-plan-preview-demo"
  );
  assert_eq!(as_i64(get(verified, "edit_count")), 1);
  assert_eq!(
    as_str(get(verified, "next_gate")),
    "coding-project-test-execution-receipt"
  );

  assert!(!as_bool(get(verified, "host_apply_allowed")));
  assert!(!as_bool(get(verified, "file_write_allowed")));
  assert!(!as_bool(get(verified, "host_execution_allowed")));
  assert!(!as_bool(get(verified, "apply_allowed")));
  assert!(!as_bool(get(verified, "raw_eval_allowed")));
  assert!(!as_bool(get(verified, "test_execution_allowed")));
  assert!(!as_bool(get(verified, "search_execution_allowed")));
  assert!(!as_bool(get(verified, "memory_write_allowed")));
  assert!(!as_bool(get(verified, "policy_persistence_allowed")));
  assert!(!as_bool(get(verified, "source_ingest_allowed")));
  assert!(!as_bool(get(verified, "accepted_fact_promotion_allowed")));
  assert!(!as_bool(get(verified, "route_update_allowed")));

  let checks = as_list(get(verified, "target_checks"));
  assert_eq!(checks.len(), 1);
  let check = &checks[0];
  assert_eq!(as_str(get(check, "path")), "client/src/request_flow.rs");
  assert!(as_bool(get(check, "snapshot_target_found")));
  assert!(as_bool(get(check, "target_identity_matches")));
  assert!(as_bool(get(check, "content_hash_matches")));
  assert!(as_bool(get(check, "new_text_at_target_anchor")));
  assert!(as_bool(get(check, "old_text_removed_at_target_anchor")));
  assert!(as_bool(get(check, "rollback_handle_still_ready")));
  assert!(as_bool(get(check, "ok")));
}

#[test]
fn reasoning_dispatch_can_build_adaptive_post_write_verification() {
  let run = eval_file(&fixture_path()).unwrap();
  let dispatched = get(&run, "dispatched-result");
  assert_eq!(
    as_str(get(dispatched, "op")),
    "build-coding-project-post-write-verification"
  );

  let result = get(dispatched, "result");
  assert_eq!(
    as_str(get(result, "outcome")),
    "coding-project-post-write-verification-passed"
  );
  assert!(as_bool(get(result, "post_write_verified")));
  assert_eq!(
    as_str(get(result, "next_gate")),
    "coding-project-test-execution-receipt"
  );
  assert!(!as_bool(get(result, "file_write_allowed")));
  assert!(!as_bool(get(result, "host_execution_allowed")));
  assert!(!as_bool(get(result, "test_execution_allowed")));
}

#[test]
fn missing_inputs_mismatches_effects_and_promotions_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let missing_result = get(&run, "missing-result");
  assert!(as_bool(get(missing_result, "is_held")));
  assert_eq!(
    as_str(get(missing_result, "outcome")),
    "held-coding-project-post-write-verification-result-required"
  );

  let missing_mirror = get(&run, "missing-mirror");
  assert!(as_bool(get(missing_mirror, "is_held")));
  assert_eq!(
    as_str(get(missing_mirror, "outcome")),
    "held-coding-project-post-write-verification-mirror-plan-required"
  );

  let missing_snapshot = get(&run, "missing-snapshot");
  assert!(as_bool(get(missing_snapshot, "is_held")));
  assert_eq!(
    as_str(get(missing_snapshot, "outcome")),
    "held-coding-project-post-write-verification-snapshot-required"
  );

  let transaction = get(&run, "transaction-mismatch");
  assert!(as_bool(get(transaction, "is_held")));
  assert_eq!(
    as_str(get(transaction, "outcome")),
    "held-coding-project-post-write-verification-transaction-mismatch"
  );

  let target_hash = get(&run, "target-hash-mismatch");
  assert!(as_bool(get(target_hash, "is_held")));
  assert_eq!(
    as_str(get(target_hash, "outcome")),
    "held-coding-project-post-write-verification-post-hash-mismatch"
  );

  let new_missing = get(&run, "new-text-missing");
  assert!(as_bool(get(new_missing, "is_held")));
  assert_eq!(
    as_str(get(new_missing, "outcome")),
    "held-coding-project-post-write-verification-new-text-missing"
  );

  let old_present = get(&run, "old-text-still-present");
  assert!(as_bool(get(old_present, "is_held")));
  assert_eq!(
    as_str(get(old_present, "outcome")),
    "held-coding-project-post-write-verification-old-text-still-present-at-target"
  );

  let rollback = get(&run, "rollback-consumed");
  assert!(as_bool(get(rollback, "is_held")));
  assert_eq!(
    as_str(get(rollback, "outcome")),
    "held-coding-project-post-write-verification-rollback-handle-consumed"
  );

  let effect = get(&run, "effect-held");
  assert!(as_bool(get(effect, "is_held")));
  assert_eq!(
    as_str(get(effect, "outcome")),
    "held-coding-project-post-write-verification-effect-blocked"
  );
  assert!(!as_bool(get(effect, "host_apply_allowed")));
  assert!(!as_bool(get(effect, "file_write_allowed")));
  assert!(!as_bool(get(effect, "host_execution_allowed")));
  assert!(!as_bool(get(effect, "source_ingest_allowed")));

  let promotion = get(&run, "promotion-held");
  assert!(as_bool(get(promotion, "is_held")));
  assert_eq!(
    as_str(get(promotion, "outcome")),
    "held-coding-project-post-write-verification-effect-blocked"
  );
  assert!(!as_bool(get(promotion, "accepted_fact_promotion_allowed")));
  assert!(!as_bool(get(promotion, "route_update_allowed")));
}
