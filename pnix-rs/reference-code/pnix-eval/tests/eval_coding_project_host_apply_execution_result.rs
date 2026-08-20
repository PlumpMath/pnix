use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/coding-project-host-apply-execution-result-receipt.px")
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
    .expect("coding project host apply execution result fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "proof")),
    "coding-project-host-apply-execution-result"
  );

  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "puncheetah.contract.coding-project-host-apply-execution-result.v0"
  );
  assert_eq!(
    as_str(get(meta, "base")),
    "coding-project-host-apply-execution-result-v0"
  );
}

#[test]
fn host_bridge_result_is_verified_without_additional_effects() {
  let run = eval_file(&fixture_path()).unwrap();
  let passed = get(&run, "passed");

  assert_eq!(
    as_str(get(passed, "schema")),
    "puncheetah.code.host-apply-execution-result.v0"
  );
  assert_eq!(
    as_str(get(passed, "outcome")),
    "coding-project-host-apply-execution-result-verified"
  );
  assert!(as_bool(get(passed, "verified")));
  assert!(as_bool(get(
    passed,
    "host_bridge_execution_result_verified"
  )));
  assert!(as_bool(get(
    passed,
    "host_apply_execution_permission_verified"
  )));
  assert!(as_bool(get(passed, "mirror_plan_consumed")));
  assert!(as_bool(get(passed, "actual_write_executed")));
  assert!(as_bool(get(passed, "apply_executed")));
  assert!(as_bool(get(passed, "file_write_executed")));
  assert!(as_bool(get(passed, "transactional_write_verified")));
  assert!(as_bool(get(passed, "rollback_handles_ready")));
  assert!(as_bool(get(passed, "post_write_snapshot_required")));
  assert_eq!(as_i64(get(passed, "edit_count")), 1);
  assert_eq!(
    as_str(get(passed, "next_gate")),
    "coding-project-post-write-verification"
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

  let targets = as_list(get(passed, "targets"));
  assert_eq!(targets.len(), 1);
  let target = &targets[0];
  assert_eq!(as_str(get(target, "path")), "generic/subject.artifact");
  assert_eq!(
    as_str(get(target, "write_mode")),
    "transactional-replace-exact-text"
  );
  assert!(as_bool(get(target, "old_anchor_found")));
  assert!(as_bool(get(target, "replace_exact_text_applied")));
  assert!(as_bool(get(target, "rollback_handle_ready")));
  assert!(!as_bool(get(target, "rollback_handle_consumed")));

  let applied = get(target, "applied_edit");
  assert_eq!(as_str(get(applied, "edit_kind")), "replace-exact-text");
  let rollback = get(target, "rollback");
  assert_eq!(as_str(get(rollback, "old_text")), "generic after subject");
  assert_eq!(as_str(get(rollback, "new_text")), "generic before subject");

  let receipt = get(passed, "receipt");
  assert_eq!(
    as_str(get(receipt, "invariant")),
    "host bridge performed project transactional writes; result matches permission targets and rollback handles, while further effects remain gated"
  );
}

#[test]
fn reasoning_dispatch_can_verify_host_apply_execution_result() {
  let run = eval_file(&fixture_path()).unwrap();
  let dispatched = get(&run, "dispatched");
  assert_eq!(
    as_str(get(dispatched, "op")),
    "verify-coding-project-host-apply-execution-result"
  );
  let result = get(dispatched, "result");
  assert_eq!(
    as_str(get(result, "outcome")),
    "coding-project-host-apply-execution-result-verified"
  );
  assert!(as_bool(get(result, "actual_write_executed")));
  assert!(!as_bool(get(result, "host_execution_allowed")));
  assert!(!as_bool(get(result, "file_write_allowed")));
}

#[test]
fn missing_inputs_mismatches_and_effect_requests_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let missing_gate = get(&run, "missing-gate");
  assert!(as_bool(get(missing_gate, "is_held")));
  assert_eq!(
    as_str(get(missing_gate, "outcome")),
    "held-coding-project-host-apply-execution-gate-required"
  );

  let missing_mirror = get(&run, "missing-mirror");
  assert!(as_bool(get(missing_mirror, "is_held")));
  assert_eq!(
    as_str(get(missing_mirror, "outcome")),
    "held-coding-project-host-apply-execution-result-mirror-plan-required"
  );

  let missing_result = get(&run, "missing-result");
  assert!(as_bool(get(missing_result, "is_held")));
  assert_eq!(
    as_str(get(missing_result, "outcome")),
    "held-coding-project-host-apply-execution-result-required"
  );

  let transaction = get(&run, "transaction-mismatch");
  assert!(as_bool(get(transaction, "is_held")));
  assert_eq!(
    as_str(get(transaction, "outcome")),
    "held-coding-project-host-apply-execution-result-mismatch"
  );

  let target = get(&run, "target-hash-mismatch");
  assert!(as_bool(get(target, "is_held")));
  assert_eq!(
    as_str(get(target, "outcome")),
    "held-coding-project-host-apply-execution-result-mismatch"
  );

  let rollback = get(&run, "rollback-consumed");
  assert!(as_bool(get(rollback, "is_held")));
  assert_eq!(
    as_str(get(rollback, "outcome")),
    "held-coding-project-host-apply-execution-result-required"
  );

  let effect = get(&run, "effect-held");
  assert!(as_bool(get(effect, "is_held")));
  assert_eq!(
    as_str(get(effect, "outcome")),
    "held-coding-project-host-apply-execution-result-effect-blocked"
  );
  assert!(!as_bool(get(effect, "host_apply_allowed")));
  assert!(!as_bool(get(effect, "file_write_allowed")));
  assert!(!as_bool(get(effect, "host_execution_allowed")));
  assert!(!as_bool(get(effect, "test_execution_allowed")));
  assert!(!as_bool(get(effect, "search_execution_allowed")));

  let promotion = get(&run, "promotion-held");
  assert!(as_bool(get(promotion, "is_held")));
  assert_eq!(
    as_str(get(promotion, "outcome")),
    "held-coding-project-host-apply-execution-result-effect-blocked"
  );
  assert!(!as_bool(get(promotion, "accepted_fact_promotion_allowed")));
}
