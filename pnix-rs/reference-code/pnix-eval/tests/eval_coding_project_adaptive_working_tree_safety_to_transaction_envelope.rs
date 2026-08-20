use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/pnix-query-runtime/coding-project-adaptive-working-tree-safety-to-transaction-envelope.px",
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
    eval_file(&fixture_path()).expect("adaptive transaction envelope fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "proof")),
    "coding-project-adaptive-working-tree-safety-to-transaction-envelope"
  );
}

#[test]
fn adaptive_working_tree_safety_builds_transaction_envelope_without_effects() {
  let run = eval_file(&fixture_path()).unwrap();

  let safety = get(&run, "working-tree-safety");
  assert_eq!(
    as_str(get(safety, "outcome")),
    "coding-project-working-tree-safety-check-passed"
  );
  assert_eq!(
    as_str(get(safety, "next_gate")),
    "coding-project-host-apply-transaction-envelope"
  );

  let mirror = get(&run, "mirror-plan");
  assert_eq!(
    as_str(get(mirror, "next_action")),
    "build-coding-project-host-apply-transaction-envelope"
  );

  let envelope = get(&run, "transaction-envelope");
  assert_eq!(
    as_str(get(envelope, "schema")),
    "puncheetah.code.host-apply-transaction-envelope.v0"
  );
  assert_eq!(
    as_str(get(envelope, "outcome")),
    "coding-project-host-apply-transaction-envelope-built"
  );
  assert!(as_bool(get(envelope, "verified")));
  assert!(as_bool(get(envelope, "transaction_envelope_built")));
  assert!(as_bool(get(envelope, "transaction_envelope_ready")));
  assert!(as_bool(get(envelope, "host_apply_plan_verified")));
  assert!(as_bool(get(envelope, "working_tree_safety_verified")));
  assert!(as_bool(get(envelope, "mirror_plan_consumed")));
  assert!(as_bool(get(envelope, "rollback_ready")));
  assert!(!as_bool(get(envelope, "rollback_execution_allowed")));
  assert_eq!(
    as_str(get(envelope, "transaction_envelope_kind")),
    "coding-project-host-apply-transaction-envelope-v0"
  );
  assert_eq!(
    as_str(get(envelope, "transaction_id")),
    "coding-project-host-apply-plan:final-approval-adaptive-preview-demo"
  );
  assert_eq!(
    as_str(get(envelope, "approved_preview_id")),
    "reopened-plan-preview-demo"
  );
  assert_eq!(
    as_str(get(envelope, "approved_preview_hash")),
    "sha256-reopened-plan-preview-demo"
  );
  assert_eq!(as_i64(get(envelope, "edit_count")), 1);
  assert_eq!(
    as_str(get(envelope, "next_gate")),
    "coding-project-host-apply-transaction-dry-run"
  );

  assert!(!as_bool(get(envelope, "host_apply_allowed")));
  assert!(!as_bool(get(envelope, "file_write_allowed")));
  assert!(!as_bool(get(envelope, "host_execution_allowed")));
  assert!(!as_bool(get(envelope, "apply_allowed")));
  assert!(!as_bool(get(envelope, "raw_eval_allowed")));
  assert!(!as_bool(get(envelope, "test_execution_allowed")));
  assert!(!as_bool(get(envelope, "search_execution_allowed")));
  assert!(!as_bool(get(envelope, "memory_write_allowed")));
  assert!(!as_bool(get(envelope, "policy_persistence_allowed")));
  assert!(!as_bool(get(envelope, "source_ingest_allowed")));
  assert!(!as_bool(get(envelope, "accepted_fact_promotion_allowed")));
  assert!(!as_bool(get(envelope, "route_update_allowed")));

  let operations = as_list(get(envelope, "host_operations"));
  assert_eq!(operations.len(), 1);
  let operation = &operations[0];
  assert_eq!(
    as_str(get(operation, "operation_kind")),
    "transactional-replace-exact-text"
  );
  assert_eq!(as_str(get(operation, "path")), "client/src/request_flow.rs");
  assert!(!as_bool(get(operation, "host_apply_execution_allowed")));
  assert!(!as_bool(get(operation, "file_write_allowed")));

  let handles = as_list(get(envelope, "rollback_handles"));
  assert_eq!(handles.len(), 1);
  let handle = &handles[0];
  assert_eq!(
    as_str(get(handle, "handle_kind")),
    "coding-project-transaction-rollback-handle-v0"
  );
  assert!(as_bool(get(handle, "materialized")));
  assert!(!as_bool(get(handle, "consumed")));
  assert_eq!(
    as_str(get(handle, "handle_id")),
    "coding-project-rollback:coding-project-host-plan:coding-project-applyable-edit:client/src/request_flow.rs"
  );

  let inner = get(envelope, "host_apply_transaction_envelope");
  assert_eq!(
    as_str(get(inner, "kind")),
    "coding-project-host-apply-transaction-envelope-v0"
  );
  assert!(as_bool(get(inner, "transaction_envelope_ready")));
  assert!(!as_bool(get(inner, "host_apply_allowed")));
  assert!(!as_bool(get(inner, "file_write_allowed")));
  assert!(!as_bool(get(inner, "accepted_fact_promotion_allowed")));

  let receipt = get(envelope, "receipt");
  assert!(as_bool(get(receipt, "transaction_envelope_ready")));
  assert!(as_bool(get(receipt, "rollback_ready")));
  assert!(!as_bool(get(receipt, "rollback_execution_allowed")));
  assert!(!as_bool(get(receipt, "route_update_allowed")));
  assert_eq!(
    as_str(get(receipt, "next_gate")),
    "coding-project-host-apply-transaction-dry-run"
  );

  let safety_receipt = get(envelope, "patch_safety_receipt");
  assert_eq!(
    as_str(get(safety_receipt, "effect_contract")),
    "project-host-transaction-envelope-only-no-write-no-apply-no-test-no-search-no-policy-persist"
  );
  assert!(!as_bool(get(safety_receipt, "source_ingest_allowed")));
}

#[test]
fn reasoning_dispatch_can_build_transaction_envelope() {
  let run = eval_file(&fixture_path()).unwrap();
  let dispatched = get(&run, "dispatched-transaction-envelope");
  assert_eq!(
    as_str(get(dispatched, "op")),
    "build-coding-project-host-apply-transaction-envelope"
  );

  let result = get(dispatched, "result");
  assert_eq!(
    as_str(get(result, "outcome")),
    "coding-project-host-apply-transaction-envelope-built"
  );
  assert!(as_bool(get(result, "transaction_envelope_ready")));
  assert_eq!(
    as_str(get(result, "next_gate")),
    "coding-project-host-apply-transaction-dry-run"
  );
}

#[test]
fn missing_chain_edit_safety_effect_and_promotion_requests_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let missing_safety = get(&run, "missing-safety");
  assert!(as_bool(get(missing_safety, "is_held")));
  assert_eq!(
    as_str(get(missing_safety, "outcome")),
    "held-coding-project-working-tree-safety-required"
  );

  let missing_mirror = get(&run, "missing-mirror");
  assert!(as_bool(get(missing_mirror, "is_held")));
  assert_eq!(
    as_str(get(missing_mirror, "outcome")),
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
  assert!(!as_bool(get(effect, "memory_write_allowed")));

  let promotion = get(&run, "promotion-held");
  assert!(as_bool(get(promotion, "is_held")));
  assert_eq!(
    as_str(get(promotion, "outcome")),
    "held-coding-project-host-apply-transaction-envelope-effect-blocked"
  );
  assert!(!as_bool(get(promotion, "accepted_fact_promotion_allowed")));
}
