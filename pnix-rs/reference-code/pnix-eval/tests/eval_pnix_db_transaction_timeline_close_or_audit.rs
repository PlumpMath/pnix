use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/pnix-db-transaction-timeline-close-or-audit.px")
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
  let run =
    eval_file(&fixture_path()).expect("pnix-db transaction timeline audit fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "proof")),
    "pnix-db-transaction-timeline-close-or-audit"
  );

  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.agent.pnix-db-transaction-timeline-close-or-audit"
  );
  assert_eq!(
    as_str(get(meta, "base")),
    "pnix-db-transaction-timeline-close-or-audit-v0"
  );
}

#[test]
fn complete_terminal_timeline_is_audited_without_writing_memory() {
  let run = eval_file(&fixture_path()).unwrap();
  let audit = get(&run, "complete-audit");

  assert_eq!(
    as_str(get(audit, "schema")),
    "puncheetah.pnix-db.transaction-timeline-close-or-audit.v0"
  );
  assert_eq!(
    as_str(get(audit, "outcome")),
    "pnix-db-transaction-timeline-audit-passed"
  );
  assert!(as_bool(get(audit, "verified")));
  assert!(as_bool(get(audit, "transaction_timeline_closed")));
  assert!(as_bool(get(audit, "transaction_audited")));
  assert_eq!(
    as_str(get(audit, "terminal_receipt_kind")),
    "transaction-complete"
  );
  assert_eq!(as_str(get(audit, "final_status")), "complete");
  assert!(as_bool(get(audit, "all_parent_receipts_present")));
  assert!(as_bool(get(audit, "all_expected_gates_present")));
  assert!(as_bool(get(audit, "timeline_order_verified")));
  assert!(as_bool(get(audit, "terminal_event_present")));
  assert_eq!(as_i64(get(audit, "timeline_event_count")), 6);
  assert_eq!(as_str(get(audit, "next_gate")), "end");
  assert_effects_locked(audit);

  let expected = as_list(get(audit, "expected_outcomes"));
  assert_eq!(expected.len(), 6);
  assert_eq!(
    as_str(&expected[5]),
    "coding-project-transaction-complete-receipt-built"
  );
}

#[test]
fn rollback_terminal_timeline_is_audited_without_writing_memory() {
  let run = eval_file(&fixture_path()).unwrap();
  let audit = get(&run, "rollback-audit");

  assert_eq!(
    as_str(get(audit, "outcome")),
    "pnix-db-transaction-timeline-audit-passed"
  );
  assert!(as_bool(get(audit, "transaction_timeline_closed")));
  assert!(as_bool(get(audit, "transaction_audited")));
  assert_eq!(
    as_str(get(audit, "terminal_receipt_kind")),
    "rollback-complete"
  );
  assert_eq!(as_str(get(audit, "final_status")), "rollback-complete");
  assert!(as_bool(get(audit, "all_expected_gates_present")));
  assert!(as_bool(get(audit, "timeline_order_verified")));
  assert_eq!(as_i64(get(audit, "timeline_event_count")), 9);
  assert_effects_locked(audit);

  let expected = as_list(get(audit, "expected_outcomes"));
  assert_eq!(expected.len(), 9);
  assert_eq!(
    as_str(&expected[8]),
    "coding-project-rollback-complete-receipt-built"
  );
}

#[test]
fn reasoning_dispatch_can_audit_transaction_timeline() {
  let run = eval_file(&fixture_path()).unwrap();
  let dispatched = get(&run, "dispatched");
  assert_eq!(
    as_str(get(dispatched, "op")),
    "build-pnix-db-transaction-timeline-close-or-audit"
  );
  let result = get(dispatched, "result");
  assert_eq!(
    as_str(get(result, "outcome")),
    "pnix-db-transaction-timeline-audit-passed"
  );
  assert!(as_bool(get(result, "transaction_audited")));
  assert_eq!(
    as_str(get(result, "terminal_receipt_kind")),
    "rollback-complete"
  );
  assert!(!as_bool(get(result, "memory_write_allowed")));
}

#[test]
fn missing_mismatch_order_and_effect_requests_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let missing_terminal = get(&run, "missing-terminal");
  assert!(as_bool(get(missing_terminal, "is_held")));
  assert_eq!(
    as_str(get(missing_terminal, "outcome")),
    "held-pnix-db-transaction-timeline-audit-terminal-receipt-required"
  );

  let missing_mirror = get(&run, "missing-mirror");
  assert!(as_bool(get(missing_mirror, "is_held")));
  assert_eq!(
    as_str(get(missing_mirror, "outcome")),
    "held-pnix-db-transaction-timeline-audit-mirror-plan-required"
  );

  let missing_timeline = get(&run, "missing-timeline");
  assert!(as_bool(get(missing_timeline, "is_held")));
  assert_eq!(
    as_str(get(missing_timeline, "outcome")),
    "held-pnix-db-transaction-timeline-audit-snapshot-required"
  );

  let terminal_mismatch = get(&run, "terminal-mismatch");
  assert!(as_bool(get(terminal_mismatch, "is_held")));
  assert_eq!(
    as_str(get(terminal_mismatch, "outcome")),
    "held-pnix-db-transaction-timeline-audit-snapshot-mismatch"
  );

  let missing_expected = get(&run, "missing-expected-gate");
  assert!(as_bool(get(missing_expected, "is_held")));
  assert_eq!(
    as_str(get(missing_expected, "outcome")),
    "held-pnix-db-transaction-timeline-audit-expected-gates-missing"
  );

  let order = get(&run, "order-invalid");
  assert!(as_bool(get(order, "is_held")));
  assert_eq!(
    as_str(get(order, "outcome")),
    "held-pnix-db-transaction-timeline-audit-order-invalid"
  );

  let effect = get(&run, "effect-held");
  assert!(as_bool(get(effect, "is_held")));
  assert_eq!(
    as_str(get(effect, "outcome")),
    "held-pnix-db-transaction-timeline-audit-effect-blocked"
  );
  assert_effects_locked(effect);
}
