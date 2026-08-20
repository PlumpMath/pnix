use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/coding-project-host-apply-execution-gate-receipt.px")
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
    .expect("coding project host apply execution gate fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "proof")),
    "coding-project-host-apply-execution-gate"
  );

  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "puncheetah.contract.coding-project-host-apply-execution-gate.v0"
  );
  assert_eq!(
    as_str(get(meta, "base")),
    "coding-project-host-apply-execution-gate-v0"
  );
}

#[test]
fn explicit_host_execution_approval_grants_permission_without_writing() {
  let run = eval_file(&fixture_path()).unwrap();
  let passed = get(&run, "passed");

  assert_eq!(
    as_str(get(passed, "schema")),
    "puncheetah.code.host-apply-execution-gate.v0"
  );
  assert_eq!(
    as_str(get(passed, "outcome")),
    "coding-project-host-apply-execution-gate-approved"
  );
  assert!(as_bool(get(passed, "verified")));
  assert!(as_bool(get(passed, "host_apply_execution_gate_approved")));
  assert!(as_bool(get(
    passed,
    "host_apply_execution_permission_granted"
  )));
  assert!(as_bool(get(passed, "file_write_permission_verified")));
  assert!(as_bool(get(passed, "mirror_plan_consumed")));
  assert!(as_bool(get(passed, "host_bridge_execution_required")));
  assert!(as_bool(get(
    passed,
    "host_bridge_execution_result_required"
  )));
  assert!(!as_bool(get(passed, "actual_write_executed")));
  assert_eq!(as_i64(get(passed, "edit_count")), 1);
  assert_eq!(
    as_str(get(passed, "next_gate")),
    "coding-project-host-apply-execution-result"
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

  let permission = get(passed, "host_apply_execution_permission");
  assert_eq!(
    as_str(get(permission, "permission_kind")),
    "coding-project-host-apply-execution-permission-v0"
  );
  assert_eq!(
    as_str(get(permission, "permission_scope")),
    "host-bridge-execution-result-only"
  );
  assert!(as_bool(get(
    permission,
    "host_apply_execution_permission_granted"
  )));
  assert!(as_bool(get(permission, "host_bridge_execution_required")));
  assert!(as_bool(get(
    permission,
    "host_bridge_execution_result_required"
  )));
  assert!(!as_bool(get(permission, "actual_write_executed")));
  assert!(!as_bool(get(permission, "host_apply_allowed")));
  assert!(!as_bool(get(permission, "file_write_allowed")));
  assert!(!as_bool(get(permission, "source_ingest_allowed")));
  assert!(!as_bool(get(permission, "accepted_fact_promotion_allowed")));
  assert!(!as_bool(get(permission, "route_update_allowed")));
  assert_eq!(
    as_str(get(permission, "next_gate")),
    "coding-project-host-apply-execution-result"
  );

  let targets = as_list(get(permission, "targets"));
  assert_eq!(targets.len(), 1);
  let target = &targets[0];
  assert_eq!(as_str(get(target, "path")), "generic/subject.artifact");
  assert_eq!(
    as_str(get(target, "write_mode")),
    "transactional-replace-exact-text"
  );
  assert_eq!(
    as_str(get(target, "rollback_hash")),
    "sha256-generic-before-content"
  );

  let receipt = get(passed, "receipt");
  assert!(as_bool(get(
    receipt,
    "host_apply_execution_permission_granted"
  )));
  assert!(as_bool(get(receipt, "file_write_permission_verified")));
  assert!(!as_bool(get(receipt, "actual_write_executed")));
  assert_eq!(
    as_str(get(receipt, "invariant")),
    "project host apply execution gate validates final write permission and host execution approval; actual file write remains delegated to host bridge execution result receipt"
  );
}

#[test]
fn missing_inputs_mismatches_and_effect_requests_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let missing_final = get(&run, "missing-final-write-approval");
  assert!(as_bool(get(missing_final, "is_held")));
  assert_eq!(
    as_str(get(missing_final, "outcome")),
    "held-coding-project-final-file-write-approval-required"
  );

  let missing_mirror = get(&run, "missing-mirror");
  assert!(as_bool(get(missing_mirror, "is_held")));
  assert_eq!(
    as_str(get(missing_mirror, "outcome")),
    "held-coding-project-host-apply-execution-gate-mirror-plan-required"
  );

  let missing_approval = get(&run, "missing-approval");
  assert!(as_bool(get(missing_approval, "is_held")));
  assert_eq!(
    as_str(get(missing_approval, "outcome")),
    "held-coding-project-host-apply-execution-approval-token-required"
  );

  let transaction = get(&run, "transaction-mismatch");
  assert!(as_bool(get(transaction, "is_held")));
  assert_eq!(
    as_str(get(transaction, "outcome")),
    "held-coding-project-host-apply-execution-approval-token-mismatch"
  );

  let target = get(&run, "target-hash-mismatch");
  assert!(as_bool(get(target, "is_held")));
  assert_eq!(
    as_str(get(target, "outcome")),
    "held-coding-project-host-apply-execution-approval-token-mismatch"
  );

  let rollback = get(&run, "rollback-mismatch");
  assert!(as_bool(get(rollback, "is_held")));
  assert_eq!(
    as_str(get(rollback, "outcome")),
    "held-coding-project-host-apply-execution-approval-token-mismatch"
  );

  let effect = get(&run, "effect-held");
  assert!(as_bool(get(effect, "is_held")));
  assert_eq!(
    as_str(get(effect, "outcome")),
    "held-coding-project-host-apply-execution-gate-effect-blocked"
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
    "held-coding-project-host-apply-execution-gate-effect-blocked"
  );
  assert!(!as_bool(get(promotion, "accepted_fact_promotion_allowed")));
}
