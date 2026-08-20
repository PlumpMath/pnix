use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/pnix-query-runtime/coding-project-host-apply-transaction-envelope-receipt.px",
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
    .expect("coding project host apply transaction envelope fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "proof")),
    "coding-project-host-apply-transaction-envelope"
  );

  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "puncheetah.contract.coding-project-host-apply-transaction-envelope.v0"
  );
  assert_eq!(
    as_str(get(meta, "base")),
    "coding-project-host-apply-transaction-envelope-v0"
  );
}

#[test]
fn mirror_planned_working_tree_safety_builds_transaction_envelope_without_effects() {
  let run = eval_file(&fixture_path()).unwrap();
  let passed = get(&run, "passed");

  assert_eq!(
    as_str(get(passed, "schema")),
    "puncheetah.code.host-apply-transaction-envelope.v0"
  );
  assert_eq!(
    as_str(get(passed, "outcome")),
    "coding-project-host-apply-transaction-envelope-built"
  );
  assert!(as_bool(get(passed, "verified")));
  assert!(as_bool(get(passed, "transaction_envelope_built")));
  assert!(as_bool(get(passed, "transaction_envelope_ready")));
  assert!(as_bool(get(passed, "host_apply_plan_verified")));
  assert!(as_bool(get(passed, "working_tree_safety_verified")));
  assert!(as_bool(get(passed, "mirror_plan_consumed")));
  assert_eq!(
    as_str(get(passed, "transaction_envelope_kind")),
    "coding-project-host-apply-transaction-envelope-v0"
  );
  assert_eq!(as_i64(get(passed, "edit_count")), 1);
  assert_eq!(
    as_str(get(passed, "next_gate")),
    "coding-project-host-apply-transaction-dry-run"
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

  let operations = as_list(get(passed, "host_operations"));
  assert_eq!(operations.len(), 1);
  let operation = &operations[0];
  assert_eq!(
    as_str(get(operation, "operation_kind")),
    "transactional-replace-exact-text"
  );
  assert!(!as_bool(get(operation, "host_apply_execution_allowed")));
  assert!(!as_bool(get(operation, "file_write_allowed")));

  let handles = as_list(get(passed, "rollback_handles"));
  assert_eq!(handles.len(), 1);
  let handle = &handles[0];
  assert_eq!(
    as_str(get(handle, "handle_kind")),
    "coding-project-transaction-rollback-handle-v0"
  );
  assert!(as_bool(get(handle, "materialized")));
  assert!(!as_bool(get(handle, "consumed")));

  let rollback = get(handle, "rollback");
  assert!(as_bool(get(rollback, "restores_forward_old_text")));

  let envelope = get(passed, "host_apply_transaction_envelope");
  assert_eq!(
    as_str(get(envelope, "kind")),
    "coding-project-host-apply-transaction-envelope-v0"
  );
  assert!(!as_bool(get(envelope, "host_apply_allowed")));
  assert!(!as_bool(get(envelope, "file_write_allowed")));

  let receipt = get(passed, "receipt");
  assert!(as_bool(get(receipt, "transaction_envelope_ready")));
  assert!(as_bool(get(receipt, "rollback_ready")));
  assert!(!as_bool(get(receipt, "rollback_execution_allowed")));
  assert_eq!(
    as_str(get(receipt, "invariant")),
    "working-tree verified project host apply plan is packaged as a transaction envelope; host apply, file write, test execution, search, and policy persistence remain locked"
  );
}

#[test]
fn missing_mirror_chain_edit_safety_mismatch_and_effect_requests_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let missing = get(&run, "missing-mirror");
  assert!(as_bool(get(missing, "is_held")));
  assert_eq!(
    as_str(get(missing, "outcome")),
    "held-coding-project-transaction-envelope-mirror-plan-required"
  );

  let chain = get(&run, "chain-mismatch");
  assert!(as_bool(get(chain, "is_held")));
  assert_eq!(
    as_str(get(chain, "outcome")),
    "held-coding-project-host-apply-transaction-envelope-chain-mismatch"
  );

  let edit = get(&run, "edit-safety-mismatch");
  assert!(as_bool(get(edit, "is_held")));
  assert_eq!(
    as_str(get(edit, "outcome")),
    "held-coding-project-host-apply-transaction-envelope-edit-safety-mismatch"
  );

  let effect = get(&run, "effect-held");
  assert!(as_bool(get(effect, "is_held")));
  assert_eq!(
    as_str(get(effect, "outcome")),
    "held-coding-project-host-apply-transaction-envelope-effect-blocked"
  );
  assert!(!as_bool(get(effect, "host_apply_allowed")));
  assert!(!as_bool(get(effect, "file_write_allowed")));
  assert!(!as_bool(get(effect, "test_execution_allowed")));
  assert!(!as_bool(get(effect, "search_execution_allowed")));
}
