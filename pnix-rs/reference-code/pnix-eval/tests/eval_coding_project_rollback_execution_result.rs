use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/coding-project-rollback-execution-result.px")
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
    .expect("coding project rollback execution result fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "proof")),
    "coding-project-rollback-execution-result"
  );

  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "puncheetah.contract.coding-project-rollback-execution-result.v0"
  );
  assert_eq!(
    as_str(get(meta, "base")),
    "coding-project-rollback-execution-result-v0"
  );
}

#[test]
fn host_rollback_result_is_verified_without_additional_effects() {
  let run = eval_file(&fixture_path()).unwrap();
  let verified = get(&run, "verified");

  assert_eq!(
    as_str(get(verified, "schema")),
    "puncheetah.code.rollback-execution-result.v0"
  );
  assert_eq!(
    as_str(get(verified, "outcome")),
    "coding-project-rollback-execution-result-verified"
  );
  assert!(as_bool(get(verified, "verified")));
  assert!(as_bool(get(
    verified,
    "host_rollback_execution_result_verified"
  )));
  assert!(as_bool(get(
    verified,
    "rollback_execution_permission_verified"
  )));
  assert!(as_bool(get(verified, "mirror_plan_consumed")));
  assert!(as_bool(get(verified, "actual_rollback_executed")));
  assert!(as_bool(get(verified, "rollback_executed")));
  assert!(as_bool(get(verified, "rollback_applied")));
  assert!(as_bool(get(verified, "transactional_rollback_verified")));
  assert!(as_bool(get(verified, "rollback_restored_pre_hash")));
  assert!(as_bool(get(verified, "rollback_handles_consumed")));
  assert!(as_bool(get(
    verified,
    "rollback_post_verification_required"
  )));
  assert_eq!(
    as_str(get(verified, "transaction_status")),
    "rollback-executed-pending-post-verification"
  );
  assert_eq!(as_i64(get(verified, "edit_count")), 1);
  assert_eq!(
    as_str(get(verified, "next_gate")),
    "coding-project-rollback-post-verification"
  );
  assert_effects_locked(verified);

  let targets = as_list(get(verified, "targets"));
  assert_eq!(targets.len(), 1);
  let target = &targets[0];
  assert_eq!(as_str(get(target, "path")), "generic/subject.artifact");
  assert_eq!(
    as_str(get(target, "rollback_mode")),
    "host-bridge-transactional-rollback"
  );
  assert!(as_bool(get(target, "post_apply_anchor_found")));
  assert!(as_bool(get(target, "rollback_exact_text_applied")));
  assert!(as_bool(get(target, "rollback_restored_pre_hash")));
  assert!(as_bool(get(target, "restore_content_hash_matches")));
  assert!(as_bool(get(target, "rollback_handle_consumed")));

  let applied = get(target, "rollback_applied_edit");
  assert_eq!(as_str(get(applied, "edit_kind")), "replace-exact-text");
  assert_eq!(as_str(get(applied, "old_text")), "generic after subject");
  assert_eq!(as_str(get(applied, "new_text")), "generic before subject");
  let forward = get(target, "forward");
  assert_eq!(as_str(get(forward, "old_text")), "generic before subject");
  assert_eq!(as_str(get(forward, "new_text")), "generic after subject");

  let receipt = get(verified, "receipt");
  assert_eq!(
    as_str(get(receipt, "invariant")),
    "host bridge performed project rollback; result matches rollback execution permission targets and consumed handles, while post-rollback verification remains gated"
  );
}

#[test]
fn reasoning_dispatch_can_verify_rollback_execution_result() {
  let run = eval_file(&fixture_path()).unwrap();
  let dispatched = get(&run, "dispatched");
  assert_eq!(
    as_str(get(dispatched, "op")),
    "verify-coding-project-rollback-execution-result"
  );
  let result = get(dispatched, "result");
  assert_eq!(
    as_str(get(result, "outcome")),
    "coding-project-rollback-execution-result-verified"
  );
  assert!(as_bool(get(result, "actual_rollback_executed")));
  assert!(as_bool(get(result, "rollback_handles_consumed")));
  assert!(!as_bool(get(result, "host_execution_allowed")));
  assert!(!as_bool(get(result, "file_write_allowed")));
}

#[test]
fn missing_inputs_mismatches_not_applied_and_effect_requests_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let missing_gate = get(&run, "missing-gate");
  assert!(as_bool(get(missing_gate, "is_held")));
  assert_eq!(
    as_str(get(missing_gate, "outcome")),
    "held-coding-project-rollback-execution-result-permission-required"
  );

  let missing_mirror = get(&run, "missing-mirror");
  assert!(as_bool(get(missing_mirror, "is_held")));
  assert_eq!(
    as_str(get(missing_mirror, "outcome")),
    "held-coding-project-rollback-execution-result-mirror-plan-required"
  );

  let missing_result = get(&run, "missing-result");
  assert!(as_bool(get(missing_result, "is_held")));
  assert_eq!(
    as_str(get(missing_result, "outcome")),
    "held-coding-project-rollback-execution-result-required"
  );

  let transaction = get(&run, "transaction-mismatch");
  assert!(as_bool(get(transaction, "is_held")));
  assert_eq!(
    as_str(get(transaction, "outcome")),
    "held-coding-project-rollback-execution-result-mismatch"
  );

  let restore_hash = get(&run, "restore-hash-mismatch");
  assert!(as_bool(get(restore_hash, "is_held")));
  assert_eq!(
    as_str(get(restore_hash, "outcome")),
    "held-coding-project-rollback-execution-result-required"
  );

  let not_applied = get(&run, "not-applied");
  assert!(as_bool(get(not_applied, "is_held")));
  assert_eq!(
    as_str(get(not_applied, "outcome")),
    "held-coding-project-rollback-execution-result-required"
  );

  let handle = get(&run, "handle-not-consumed");
  assert!(as_bool(get(handle, "is_held")));
  assert_eq!(
    as_str(get(handle, "outcome")),
    "held-coding-project-rollback-execution-result-required"
  );

  let effect = get(&run, "effect-held");
  assert!(as_bool(get(effect, "is_held")));
  assert_eq!(
    as_str(get(effect, "outcome")),
    "held-coding-project-rollback-execution-result-effect-blocked"
  );
  assert_effects_locked(effect);
}
