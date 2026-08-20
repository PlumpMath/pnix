use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/pnix-db-receipt-append-gate-receipt.px")
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
  let run = eval_file(&fixture_path()).expect("pnix-db receipt append gate fixture must evaluate");
  assert_eq!(as_str(get(&run, "proof")), "pnix-db-receipt-append-gate");

  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.agent.pnix-db-receipt-append-gate"
  );
  assert_eq!(as_str(get(meta, "base")), "pnix-db-receipt-append-gate-v0");
}

#[test]
fn verified_receipt_becomes_append_permission_without_db_write() {
  let run = eval_file(&fixture_path()).unwrap();
  let passed = get(&run, "passed");

  assert_eq!(
    as_str(get(passed, "schema")),
    "puncheetah.pnix-db.receipt-append-gate.v0"
  );
  assert_eq!(
    as_str(get(passed, "outcome")),
    "pnix-db-receipt-append-gate-approved"
  );
  assert!(as_bool(get(passed, "verified")));
  assert!(as_bool(get(passed, "pnix_db_append_permission_granted")));
  assert!(as_bool(get(
    passed,
    "working_memory_append_permission_granted"
  )));
  assert_eq!(as_str(get(passed, "append_target")), "receipt");
  assert_eq!(as_str(get(passed, "append_table")), "wm_receipts");
  assert_eq!(as_str(get(passed, "append_mode")), "append-only");
  assert_eq!(
    as_str(get(passed, "append_conflict_policy")),
    "hold-on-different-bytes"
  );
  assert!(as_bool(get(passed, "working_memory_required")));
  assert!(!as_bool(get(passed, "db_write_executed")));
  assert_eq!(
    as_str(get(passed, "next_gate")),
    "pnix-db-receipt-append-result"
  );

  assert!(!as_bool(get(passed, "memory_write_allowed")));
  assert!(!as_bool(get(passed, "policy_persistence_allowed")));
  assert!(!as_bool(get(passed, "search_execution_allowed")));
  assert!(!as_bool(get(passed, "file_write_allowed")));
  assert!(!as_bool(get(passed, "host_execution_allowed")));
  assert!(!as_bool(get(passed, "host_apply_allowed")));

  let permission = get(passed, "working_memory_append_permission");
  assert_eq!(
    as_str(get(permission, "permission_kind")),
    "pnix-db-working-memory-append-permission-v0"
  );
  assert_eq!(
    as_str(get(permission, "permission_scope")),
    "working-memory-receipt-append-result-only"
  );
  assert!(as_bool(get(
    permission,
    "working_memory_append_permission_granted"
  )));
  assert!(!as_bool(get(permission, "db_write_executed")));
  assert!(!as_bool(get(permission, "memory_write_allowed")));
  assert_eq!(
    as_str(get(permission, "receipt_id")),
    "receipt:generic-host-apply-execution-gate"
  );
  assert_eq!(
    as_str(get(permission, "expected_content_hash")),
    "sha256-generic-receipt-content"
  );
  assert_eq!(as_i64(get(permission, "created_at_ms")), 0);
  assert_eq!(as_list(get(permission, "parent_event_ids")).len(), 0);
  assert_eq!(as_list(get(permission, "parent_receipt_ids")).len(), 1);

  let receipt = get(passed, "receipt");
  assert!(as_bool(get(receipt, "pnix_db_append_permission_granted")));
  assert!(!as_bool(get(receipt, "db_write_executed")));
  assert_eq!(
    as_str(get(receipt, "invariant")),
    "pnix-db receipt append gate grants append permission only; actual redb write must return pnix-db-receipt-append-result"
  );
}

#[test]
fn mirror_and_dispatch_route_append_gate_without_memory_effects() {
  let run = eval_file(&fixture_path()).unwrap();

  let observed = get(&run, "observed-passed");
  assert_eq!(
    as_str(get(observed, "outcome")),
    "mirror-self-observation-plan-built"
  );
  assert_eq!(
    as_str(get(observed, "next_gate")),
    "pnix-db-receipt-append-result"
  );
  assert_eq!(
    as_str(get(observed, "next_action")),
    "verify-pnix-db-receipt-append-result"
  );
  assert!(!as_bool(get(observed, "memory_write_allowed")));
  assert!(!as_bool(get(observed, "policy_persistence_allowed")));
  let ko = get(observed, "ko_self_description");
  assert!(as_str(get(ko, "text")).contains("pnix-db receipt append gate"));
  assert!(as_str(get(ko, "text")).contains("working memory append result receipt"));

  let dispatched = get(&run, "dispatched");
  assert_eq!(as_str(get(dispatched, "op")), "plan-pnix-db-receipt-append");
  let result = get(dispatched, "result");
  assert_eq!(
    as_str(get(result, "outcome")),
    "pnix-db-receipt-append-gate-approved"
  );
  assert!(as_bool(get(
    result,
    "working_memory_append_permission_granted"
  )));
  assert!(!as_bool(get(result, "memory_write_allowed")));
}

#[test]
fn invalid_receipts_contexts_and_effect_requests_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let missing = get(&run, "missing-receipt");
  assert!(as_bool(get(missing, "is_held")));
  assert_eq!(
    as_str(get(missing, "outcome")),
    "held-pnix-db-receipt-append-receipt-required"
  );

  let unverified = get(&run, "unverified-receipt");
  assert!(as_bool(get(unverified, "is_held")));
  assert_eq!(
    as_str(get(unverified, "outcome")),
    "held-pnix-db-receipt-append-unverified-receipt"
  );

  let hash = get(&run, "hash-mismatch");
  assert!(as_bool(get(hash, "is_held")));
  assert_eq!(
    as_str(get(hash, "outcome")),
    "held-pnix-db-receipt-append-context-mismatch"
  );

  let parent = get(&run, "parent-mismatch");
  assert!(as_bool(get(parent, "is_held")));
  assert_eq!(
    as_str(get(parent, "outcome")),
    "held-pnix-db-receipt-append-parent-mismatch"
  );

  let table = get(&run, "disallowed-table");
  assert!(as_bool(get(table, "is_held")));
  assert_eq!(
    as_str(get(table, "outcome")),
    "held-pnix-db-receipt-append-disallowed-table"
  );

  let policy = get(&run, "policy-persistence-held");
  assert!(as_bool(get(policy, "is_held")));
  assert_eq!(
    as_str(get(policy, "outcome")),
    "held-pnix-db-receipt-append-policy-persistence-blocked"
  );

  let effect = get(&run, "effect-held");
  assert!(as_bool(get(effect, "is_held")));
  assert_eq!(
    as_str(get(effect, "outcome")),
    "held-pnix-db-receipt-append-effect-blocked"
  );
  assert!(!as_bool(get(effect, "memory_write_allowed")));
  assert!(!as_bool(get(effect, "policy_persistence_allowed")));
  assert!(!as_bool(get(effect, "search_execution_allowed")));
  assert!(!as_bool(get(effect, "file_write_allowed")));
}
