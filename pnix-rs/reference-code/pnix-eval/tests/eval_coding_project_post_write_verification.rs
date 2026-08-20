use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/coding-project-post-write-verification-receipt.px")
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
    .expect("coding project post-write verification fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "proof")),
    "coding-project-post-write-verification"
  );

  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "puncheetah.contract.coding-project-post-write-verification.v0"
  );
  assert_eq!(
    as_str(get(meta, "base")),
    "coding-project-post-write-verification-v0"
  );
}

#[test]
fn post_write_snapshot_is_verified_without_additional_effects() {
  let run = eval_file(&fixture_path()).unwrap();
  let passed = get(&run, "passed");

  assert_eq!(
    as_str(get(passed, "schema")),
    "puncheetah.code.post-write-verification.v0"
  );
  assert_eq!(
    as_str(get(passed, "outcome")),
    "coding-project-post-write-verification-passed"
  );
  assert!(as_bool(get(passed, "verified")));
  assert!(as_bool(get(passed, "post_write_verified")));
  assert!(as_bool(get(passed, "post_write_snapshot_verified")));
  assert!(as_bool(get(passed, "host_apply_execution_result_verified")));
  assert!(as_bool(get(passed, "mirror_plan_consumed")));
  assert!(as_bool(get(passed, "actual_write_executed")));
  assert!(as_bool(get(passed, "all_target_hashes_match")));
  assert!(as_bool(get(passed, "all_new_text_at_target_anchors")));
  assert!(as_bool(get(
    passed,
    "all_old_text_removed_at_target_anchors"
  )));
  assert!(as_bool(get(passed, "rollback_handles_still_ready")));
  assert!(!as_bool(get(passed, "rollback_execution_allowed")));
  assert_eq!(as_i64(get(passed, "edit_count")), 1);
  assert_eq!(
    as_str(get(passed, "next_gate")),
    "coding-project-test-execution-receipt"
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
  assert!(!as_bool(get(passed, "source_ingest_allowed")));
  assert!(!as_bool(get(passed, "accepted_fact_promotion_allowed")));
  assert!(!as_bool(get(passed, "route_update_allowed")));

  let checks = as_list(get(passed, "target_checks"));
  assert_eq!(checks.len(), 1);
  let check = &checks[0];
  assert_eq!(as_str(get(check, "path")), "generic/subject.artifact");
  assert!(as_bool(get(check, "snapshot_target_found")));
  assert!(as_bool(get(check, "target_identity_matches")));
  assert!(as_bool(get(check, "content_hash_matches")));
  assert!(as_bool(get(check, "new_text_at_target_anchor")));
  assert!(as_bool(get(check, "old_text_removed_at_target_anchor")));
  assert!(as_bool(get(check, "rollback_handle_still_ready")));
  assert!(as_bool(get(check, "ok")));

  let receipt = get(passed, "receipt");
  assert_eq!(
    as_str(get(receipt, "invariant")),
    "host-provided post-write snapshot matches verified execution result targets, hashes, anchors, and rollback readiness; test execution remains gated"
  );
}

#[test]
fn reasoning_dispatch_can_build_post_write_verification() {
  let run = eval_file(&fixture_path()).unwrap();
  let dispatched = get(&run, "dispatched");
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
  assert!(!as_bool(get(effect, "test_execution_allowed")));
  assert!(!as_bool(get(effect, "search_execution_allowed")));

  let promotion = get(&run, "promotion-held");
  assert!(as_bool(get(promotion, "is_held")));
  assert_eq!(
    as_str(get(promotion, "outcome")),
    "held-coding-project-post-write-verification-effect-blocked"
  );
  assert!(!as_bool(get(promotion, "source_ingest_allowed")));
  assert!(!as_bool(get(promotion, "accepted_fact_promotion_allowed")));
  assert!(!as_bool(get(promotion, "route_update_allowed")));
}
