use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/pnix-query-runtime/coding-project-adaptive-dry-run-to-test-plan-receipt.px",
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
    eval_file(&fixture_path()).expect("adaptive dry-run to test plan fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "proof")),
    "coding-project-adaptive-dry-run-to-test-plan-receipt"
  );
}

#[test]
fn adaptive_dry_run_builds_test_plan_receipt_without_executing_tests() {
  let run = eval_file(&fixture_path()).unwrap();
  let dry_run = get(&run, "apply-dry-run");
  let receipt = get(&run, "test-plan-receipt");

  assert_eq!(
    as_str(get(dry_run, "outcome")),
    "coding-project-apply-dry-run-passed"
  );
  assert_eq!(
    as_str(get(dry_run, "next_gate")),
    "coding-project-test-plan-receipt"
  );

  assert_eq!(
    as_str(get(receipt, "schema")),
    "puncheetah.code.test-plan-receipt.v0"
  );
  assert_eq!(
    as_str(get(receipt, "outcome")),
    "coding-project-test-plan-receipt-built"
  );
  assert!(as_bool(get(receipt, "verified")));
  assert!(as_bool(get(receipt, "test_plan_verified")));
  assert!(as_bool(get(receipt, "baseline_test_command_allowed")));
  assert!(as_bool(get(receipt, "post_apply_test_command_allowed")));
  assert_eq!(
    as_str(get(receipt, "apply_dry_run_outcome")),
    "coding-project-apply-dry-run-passed"
  );
  assert_eq!(
    as_str(get(receipt, "applyable_ir_outcome")),
    "coding-project-applyable-ir-built"
  );
  assert_eq!(
    as_str(get(receipt, "approved_preview_id")),
    "reopened-plan-preview-demo"
  );
  assert_eq!(
    as_str(get(receipt, "manifest_test_command")),
    "cargo test -p client request_flow"
  );
  assert_eq!(
    as_str(get(receipt, "next_gate")),
    "coding-project-final-apply-approval-or-host-plan"
  );

  let commands = as_list(get(receipt, "planned_test_commands"));
  assert_eq!(commands.len(), 2);
  for command in commands {
    assert!(as_bool(get(command, "command_present")));
    assert!(as_bool(get(command, "command_matches_manifest")));
    assert!(as_bool(get(command, "command_allowlisted")));
    assert!(as_bool(get(command, "command_bounded")));
    assert!(!as_bool(get(command, "test_execution_allowed")));
  }

  let inner = get(receipt, "receipt");
  assert_eq!(as_i64(get(inner, "command_count")), 2);
  assert_eq!(
    as_str(get(inner, "next_gate")),
    "coding-project-final-apply-approval-or-host-plan"
  );

  assert!(!as_bool(get(receipt, "file_write_allowed")));
  assert!(!as_bool(get(receipt, "host_execution_allowed")));
  assert!(!as_bool(get(receipt, "host_apply_allowed")));
  assert!(!as_bool(get(receipt, "direct_apply_allowed")));
  assert!(!as_bool(get(receipt, "apply_allowed")));
  assert!(!as_bool(get(receipt, "raw_eval_allowed")));
  assert!(!as_bool(get(receipt, "test_execution_allowed")));
}

#[test]
fn missing_dry_run_command_drift_unsafe_command_and_effects_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let missing_dry_run = get(&run, "missing-dry-run");
  assert!(as_bool(get(missing_dry_run, "is_held")));
  assert_eq!(
    as_str(get(missing_dry_run, "outcome")),
    "held-coding-project-apply-dry-run-required"
  );

  let command_drift = get(&run, "command-drift");
  assert!(as_bool(get(command_drift, "is_held")));
  assert_eq!(
    as_str(get(command_drift, "outcome")),
    "held-coding-project-test-command-manifest-mismatch"
  );

  let unsafe_command = get(&run, "unsafe-command");
  assert!(as_bool(get(unsafe_command, "is_held")));
  assert_eq!(
    as_str(get(unsafe_command, "outcome")),
    "held-coding-project-test-command-unsafe"
  );

  let effect = get(&run, "effect-held");
  assert!(as_bool(get(effect, "is_held")));
  assert_eq!(
    as_str(get(effect, "outcome")),
    "held-coding-project-test-plan-effect-blocked"
  );
  assert!(!as_bool(get(effect, "test_execution_allowed")));
}
