use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/coding-project-test-plan-receipt.px")
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
  let run = eval_file(&fixture_path()).expect("coding project test plan fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "proof")),
    "coding-project-test-plan-receipt"
  );

  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "puncheetah.contract.coding-project-test-plan-receipt.v0"
  );
  assert_eq!(
    as_str(get(meta, "base")),
    "coding-project-test-plan-receipt-v0"
  );
}

#[test]
fn dry_run_and_manifest_build_test_plan_receipt_without_execution() {
  let run = eval_file(&fixture_path()).unwrap();
  let passed = get(&run, "passed");

  assert_eq!(
    as_str(get(passed, "schema")),
    "puncheetah.code.test-plan-receipt.v0"
  );
  assert_eq!(
    as_str(get(passed, "outcome")),
    "coding-project-test-plan-receipt-built"
  );
  assert!(as_bool(get(passed, "verified")));
  assert!(as_bool(get(passed, "test_plan_verified")));
  assert!(as_bool(get(passed, "baseline_test_command_allowed")));
  assert!(as_bool(get(passed, "post_apply_test_command_allowed")));
  assert_eq!(
    as_str(get(passed, "next_gate")),
    "coding-project-final-apply-approval-or-host-plan"
  );

  assert!(!as_bool(get(passed, "file_write_allowed")));
  assert!(!as_bool(get(passed, "host_execution_allowed")));
  assert!(!as_bool(get(passed, "host_apply_allowed")));
  assert!(!as_bool(get(passed, "direct_apply_allowed")));
  assert!(!as_bool(get(passed, "apply_allowed")));
  assert!(!as_bool(get(passed, "raw_eval_allowed")));
  assert!(!as_bool(get(passed, "test_execution_allowed")));

  let commands = as_list(get(passed, "planned_test_commands"));
  assert_eq!(commands.len(), 2);
  let baseline = &commands[0];
  assert_eq!(as_str(get(baseline, "phase")), "baseline");
  assert!(as_bool(get(baseline, "command_matches_manifest")));
  assert!(as_bool(get(baseline, "command_allowlisted")));
  assert!(as_bool(get(baseline, "command_bounded")));
  assert!(!as_bool(get(baseline, "test_execution_allowed")));

  let receipt = get(passed, "receipt");
  assert_eq!(as_i64(get(receipt, "command_count")), 2);
  assert_eq!(
    as_str(get(receipt, "invariant")),
    "dry-run passed and baseline/post-apply test commands match the project manifest allowlist; no test execution, file write, apply, host, or raw eval effect is allowed"
  );
}

#[test]
fn manifest_mismatch_not_allowlisted_unsafe_and_effects_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let mismatch = get(&run, "manifest-mismatch");
  assert!(as_bool(get(mismatch, "is_held")));
  assert_eq!(
    as_str(get(mismatch, "outcome")),
    "held-coding-project-test-command-manifest-mismatch"
  );
  assert!(!as_bool(get(mismatch, "test_plan_verified")));

  let not_allowlisted = get(&run, "not-allowlisted");
  assert!(as_bool(get(not_allowlisted, "is_held")));
  assert_eq!(
    as_str(get(not_allowlisted, "outcome")),
    "held-coding-project-test-command-not-allowlisted"
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
