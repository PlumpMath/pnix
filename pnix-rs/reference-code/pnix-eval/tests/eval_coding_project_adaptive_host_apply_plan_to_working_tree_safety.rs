use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/pnix-query-runtime/coding-project-adaptive-host-apply-plan-to-working-tree-safety.px",
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
  let run = eval_file(&fixture_path()).expect("adaptive working tree safety fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "proof")),
    "coding-project-adaptive-host-apply-plan-to-working-tree-safety"
  );
}

#[test]
fn adaptive_host_apply_plan_passes_working_tree_safety_without_effects() {
  let run = eval_file(&fixture_path()).unwrap();

  let host_plan = get(&run, "host-apply-plan");
  assert_eq!(
    as_str(get(host_plan, "outcome")),
    "coding-project-host-apply-plan-built"
  );
  assert_eq!(
    as_str(get(host_plan, "next_gate")),
    "coding-project-working-tree-safety-check"
  );
  assert!(as_bool(get(host_plan, "host_apply_plan_ready")));
  assert!(!as_bool(get(host_plan, "host_apply_allowed")));

  let mirror = get(&run, "mirror-plan");
  assert_eq!(
    as_str(get(mirror, "next_action")),
    "build-coding-project-working-tree-safety-check"
  );

  let safety = get(&run, "working-tree-safety");
  assert_eq!(
    as_str(get(safety, "schema")),
    "puncheetah.code.working-tree-safety-check.v0"
  );
  assert_eq!(
    as_str(get(safety, "outcome")),
    "coding-project-working-tree-safety-check-passed"
  );
  assert!(as_bool(get(safety, "verified")));
  assert!(as_bool(get(safety, "working_tree_safety_check_passed")));
  assert!(as_bool(get(safety, "mirror_plan_consumed")));
  assert!(as_bool(get(safety, "host_apply_plan_verified")));
  assert!(as_bool(get(safety, "branch_allowed")));
  assert!(as_bool(get(safety, "working_tree_clean")));
  assert!(as_bool(get(safety, "all_target_files_safe")));
  assert!(as_bool(get(safety, "all_latest_content_hashes_match")));
  assert!(as_bool(get(safety, "file_lock_clear")));
  assert_eq!(
    as_str(get(safety, "transaction_id")),
    "coding-project-host-apply-plan:final-approval-adaptive-preview-demo"
  );
  assert_eq!(
    as_str(get(safety, "approved_preview_id")),
    "reopened-plan-preview-demo"
  );
  assert_eq!(
    as_str(get(safety, "approved_preview_hash")),
    "sha256-reopened-plan-preview-demo"
  );
  assert_eq!(as_i64(get(safety, "edit_count")), 1);
  assert_eq!(
    as_str(get(safety, "workspace_ref")),
    "adaptive-demo-workspace"
  );
  assert_eq!(as_str(get(safety, "current_branch")), "main");
  assert_eq!(
    as_str(get(safety, "next_gate")),
    "coding-project-host-apply-transaction-envelope"
  );

  assert!(!as_bool(get(safety, "host_apply_allowed")));
  assert!(!as_bool(get(safety, "file_write_allowed")));
  assert!(!as_bool(get(safety, "host_execution_allowed")));
  assert!(!as_bool(get(safety, "apply_allowed")));
  assert!(!as_bool(get(safety, "raw_eval_allowed")));
  assert!(!as_bool(get(safety, "test_execution_allowed")));
  assert!(!as_bool(get(safety, "memory_write_allowed")));
  assert!(!as_bool(get(safety, "policy_persistence_allowed")));
  assert!(!as_bool(get(safety, "accepted_fact_promotion_allowed")));
  assert!(!as_bool(get(safety, "route_update_allowed")));

  let checks = as_list(get(safety, "target_checks"));
  assert_eq!(checks.len(), 1);
  let check = &checks[0];
  assert_eq!(
    as_str(get(check, "plan_edit_id")),
    "coding-project-host-plan:coding-project-applyable-edit:client/src/request_flow.rs"
  );
  assert_eq!(as_str(get(check, "path")), "client/src/request_flow.rs");
  assert!(as_bool(get(check, "file_found")));
  assert!(as_bool(get(check, "file_snapshot_ok")));
  assert!(as_bool(get(check, "latest_content_hash_matches")));
  assert!(as_bool(get(check, "file_lock_clear")));
  assert!(as_bool(get(check, "symlink_clear")));
  assert!(as_bool(get(check, "path_traversal_clear")));
  assert!(as_bool(get(check, "ok")));

  let receipt = get(safety, "receipt");
  assert!(as_bool(get(receipt, "working_tree_safety_check_passed")));
  assert!(as_bool(get(receipt, "host_apply_plan_verified")));
  assert!(!as_bool(get(receipt, "host_apply_allowed")));
  assert_eq!(
    as_str(get(receipt, "next_gate")),
    "coding-project-host-apply-transaction-envelope"
  );

  let safety_receipt = get(safety, "patch_safety_receipt");
  assert_eq!(
    as_str(get(safety_receipt, "effect_contract")),
    "working-tree-snapshot-check-only-no-write-no-apply-no-test-no-host-exec"
  );
  assert!(!as_bool(get(safety_receipt, "memory_write_allowed")));
}

#[test]
fn reasoning_dispatch_can_build_working_tree_safety_check() {
  let run = eval_file(&fixture_path()).unwrap();
  let dispatched = get(&run, "dispatched-working-tree-safety");
  assert_eq!(
    as_str(get(dispatched, "op")),
    "build-coding-project-working-tree-safety-check"
  );

  let result = get(dispatched, "result");
  assert_eq!(
    as_str(get(result, "outcome")),
    "coding-project-working-tree-safety-check-passed"
  );
  assert!(as_bool(get(result, "working_tree_safety_check_passed")));
  assert_eq!(
    as_str(get(result, "next_gate")),
    "coding-project-host-apply-transaction-envelope"
  );
}

#[test]
fn missing_dirty_hash_lock_unsafe_and_effects_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let missing = get(&run, "missing-host-plan");
  assert!(as_bool(get(missing, "is_held")));
  assert_eq!(
    as_str(get(missing, "outcome")),
    "held-coding-project-host-apply-plan-required"
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

  let lock = get(&run, "lock-active");
  assert!(as_bool(get(lock, "is_held")));
  assert_eq!(
    as_str(get(lock, "outcome")),
    "held-coding-project-working-tree-file-lock-active"
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
  assert!(!as_bool(get(effect, "memory_write_allowed")));
}
