use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/coding-project-host-apply-plan-receipt.px")
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
    eval_file(&fixture_path()).expect("coding project host apply plan fixture must evaluate");
  assert_eq!(as_str(get(&run, "proof")), "coding-project-host-apply-plan");

  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "puncheetah.contract.coding-project-host-apply-plan.v0"
  );
  assert_eq!(
    as_str(get(meta, "base")),
    "coding-project-host-apply-plan-v0"
  );
}

#[test]
fn final_approved_chain_builds_host_apply_plan_without_effects() {
  let run = eval_file(&fixture_path()).unwrap();
  let passed = get(&run, "passed");

  assert_eq!(
    as_str(get(passed, "schema")),
    "puncheetah.code.host-apply-plan.v0"
  );
  assert_eq!(
    as_str(get(passed, "outcome")),
    "coding-project-host-apply-plan-built"
  );
  assert!(as_bool(get(passed, "verified")));
  assert!(as_bool(get(passed, "host_apply_plan_built")));
  assert!(as_bool(get(passed, "host_apply_plan_ready")));
  assert!(as_bool(get(passed, "host_apply_plan_allowed")));
  assert!(!as_bool(get(passed, "host_apply_allowed")));
  assert_eq!(
    as_str(get(passed, "host_apply_plan_kind")),
    "coding-project-host-apply-plan-v0"
  );
  assert_eq!(
    as_str(get(passed, "next_gate")),
    "coding-project-working-tree-safety-check"
  );

  assert_eq!(as_i64(get(passed, "edit_count")), 1);
  assert_eq!(as_i64(get(passed, "approved_file_count")), 1);
  assert!(!as_bool(get(passed, "file_write_allowed")));
  assert!(!as_bool(get(passed, "host_execution_allowed")));
  assert!(!as_bool(get(passed, "direct_apply_allowed")));
  assert!(!as_bool(get(passed, "apply_allowed")));
  assert!(!as_bool(get(passed, "raw_eval_allowed")));
  assert!(!as_bool(get(passed, "test_execution_allowed")));

  let plan_edits = as_list(get(passed, "plan_edits"));
  assert_eq!(plan_edits.len(), 1);
  let first = &plan_edits[0];
  assert_eq!(as_str(get(first, "edit_kind")), "replace-exact-text");
  assert_eq!(as_i64(get(first, "anchor_index")), 7);
  assert!(as_bool(get(first, "rollback_restored_original")));

  let host_operation = get(first, "host_operation");
  assert_eq!(
    as_str(get(host_operation, "operation_kind")),
    "transactional-replace-exact-text"
  );

  let execution_policy = get(passed, "execution_policy");
  assert!(as_bool(get(
    execution_policy,
    "working_tree_safety_check_required"
  )));
  assert!(as_bool(get(
    execution_policy,
    "final_file_write_approval_required"
  )));
  assert!(as_bool(get(
    execution_policy,
    "rollback_ready_receipt_required"
  )));

  let receipt = get(passed, "receipt");
  assert!(as_bool(get(receipt, "host_apply_plan_ready")));
  assert!(!as_bool(get(receipt, "host_apply_allowed")));
  assert_eq!(
    as_str(get(receipt, "invariant")),
    "final approved project patch chain is frozen as a host apply plan; actual host apply, file write, test execution, and raw eval remain locked"
  );
}

#[test]
fn mismatches_missing_final_gate_and_effect_requests_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let chain = get(&run, "chain-mismatch");
  assert!(as_bool(get(chain, "is_held")));
  assert_eq!(
    as_str(get(chain, "outcome")),
    "held-coding-project-host-apply-plan-chain-mismatch"
  );
  assert!(!as_bool(get(chain, "host_apply_plan_ready")));

  let edit = get(&run, "edit-mismatch");
  assert!(as_bool(get(edit, "is_held")));
  assert_eq!(
    as_str(get(edit, "outcome")),
    "held-coding-project-host-apply-plan-edit-mismatch"
  );

  let missing = get(&run, "missing-final-gate");
  assert!(as_bool(get(missing, "is_held")));
  assert_eq!(
    as_str(get(missing, "outcome")),
    "held-coding-project-final-apply-approval-gate-required"
  );

  let effect = get(&run, "effect-held");
  assert!(as_bool(get(effect, "is_held")));
  assert_eq!(
    as_str(get(effect, "outcome")),
    "held-coding-project-host-apply-plan-effect-blocked"
  );
  assert!(!as_bool(get(effect, "file_write_allowed")));
  assert!(!as_bool(get(effect, "host_apply_allowed")));
  assert!(!as_bool(get(effect, "test_execution_allowed")));
}
