use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/coding-project-working-tree-safety-check-receipt.px")
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
    eval_file(&fixture_path()).expect("coding project working tree safety fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "proof")),
    "coding-project-working-tree-safety-check"
  );

  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "puncheetah.contract.coding-project-working-tree-safety-check.v0"
  );
  assert_eq!(
    as_str(get(meta, "base")),
    "coding-project-working-tree-safety-check-v0"
  );
}

#[test]
fn mirror_planned_host_apply_plan_passes_working_tree_safety_without_effects() {
  let run = eval_file(&fixture_path()).unwrap();
  let passed = get(&run, "passed");

  assert_eq!(
    as_str(get(passed, "schema")),
    "puncheetah.code.working-tree-safety-check.v0"
  );
  assert_eq!(
    as_str(get(passed, "outcome")),
    "coding-project-working-tree-safety-check-passed"
  );
  assert!(as_bool(get(passed, "verified")));
  assert!(as_bool(get(passed, "working_tree_safety_check_passed")));
  assert!(as_bool(get(passed, "mirror_plan_consumed")));
  assert!(as_bool(get(passed, "host_apply_plan_verified")));
  assert!(as_bool(get(passed, "branch_allowed")));
  assert!(as_bool(get(passed, "working_tree_clean")));
  assert!(as_bool(get(passed, "all_target_files_safe")));
  assert!(as_bool(get(passed, "all_latest_content_hashes_match")));
  assert!(as_bool(get(passed, "file_lock_clear")));
  assert_eq!(as_i64(get(passed, "edit_count")), 1);
  assert_eq!(
    as_str(get(passed, "next_gate")),
    "coding-project-host-apply-transaction-envelope"
  );

  assert!(!as_bool(get(passed, "host_apply_allowed")));
  assert!(!as_bool(get(passed, "file_write_allowed")));
  assert!(!as_bool(get(passed, "host_execution_allowed")));
  assert!(!as_bool(get(passed, "apply_allowed")));
  assert!(!as_bool(get(passed, "raw_eval_allowed")));
  assert!(!as_bool(get(passed, "test_execution_allowed")));

  let checks = as_list(get(passed, "target_checks"));
  assert_eq!(checks.len(), 1);
  let first = &checks[0];
  assert!(as_bool(get(first, "ok")));
  assert!(as_bool(get(first, "file_found")));
  assert!(as_bool(get(first, "latest_content_hash_matches")));
  assert!(as_bool(get(first, "file_lock_clear")));

  let receipt = get(passed, "receipt");
  assert!(as_bool(get(receipt, "working_tree_safety_check_passed")));
  assert!(as_bool(get(receipt, "mirror_plan_consumed")));
  assert!(!as_bool(get(receipt, "host_apply_allowed")));
  assert_eq!(
    as_str(get(receipt, "invariant")),
    "host-provided working tree snapshot matches the mirror-planned host apply plan; actual host apply, file write, test execution, and raw eval remain locked"
  );
}

#[test]
fn missing_mirror_dirty_hash_mismatch_and_effect_requests_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let missing = get(&run, "missing-mirror");
  assert!(as_bool(get(missing, "is_held")));
  assert_eq!(
    as_str(get(missing, "outcome")),
    "held-coding-project-mirror-plan-required"
  );

  let dirty = get(&run, "dirty");
  assert!(as_bool(get(dirty, "is_held")));
  assert_eq!(
    as_str(get(dirty, "outcome")),
    "held-coding-project-working-tree-dirty"
  );

  let hash = get(&run, "hash-mismatch");
  assert!(as_bool(get(hash, "is_held")));
  assert_eq!(
    as_str(get(hash, "outcome")),
    "held-coding-project-working-tree-content-hash-mismatch"
  );

  let unsafe_target = get(&run, "unsafe-target");
  assert!(as_bool(get(unsafe_target, "is_held")));
  assert_eq!(
    as_str(get(unsafe_target, "outcome")),
    "held-coding-project-working-tree-target-file-unsafe"
  );

  let effect = get(&run, "effect-held");
  assert!(as_bool(get(effect, "is_held")));
  assert_eq!(
    as_str(get(effect, "outcome")),
    "held-coding-project-working-tree-effect-blocked"
  );
  assert!(!as_bool(get(effect, "host_apply_allowed")));
  assert!(!as_bool(get(effect, "file_write_allowed")));
  assert!(!as_bool(get(effect, "test_execution_allowed")));
}
