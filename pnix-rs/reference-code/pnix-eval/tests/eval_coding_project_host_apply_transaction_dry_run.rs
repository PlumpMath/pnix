use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/pnix-query-runtime/coding-project-host-apply-transaction-dry-run-receipt.px",
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
    .expect("coding project host apply transaction dry-run fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "proof")),
    "coding-project-host-apply-transaction-dry-run"
  );

  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "puncheetah.contract.coding-project-host-apply-transaction-dry-run.v0"
  );
  assert_eq!(
    as_str(get(meta, "base")),
    "coding-project-host-apply-transaction-dry-run-v0"
  );
}

#[test]
fn transaction_envelope_dry_run_forwards_and_rolls_back_without_effects() {
  let run = eval_file(&fixture_path()).unwrap();
  let passed = get(&run, "passed");

  assert_eq!(
    as_str(get(passed, "schema")),
    "puncheetah.code.host-apply-transaction-dry-run.v0"
  );
  assert_eq!(
    as_str(get(passed, "outcome")),
    "coding-project-host-apply-transaction-dry-run-passed"
  );
  assert!(as_bool(get(passed, "verified")));
  assert!(as_bool(get(passed, "transaction_dry_run_passed")));
  assert!(as_bool(get(passed, "transaction_envelope_verified")));
  assert!(as_bool(get(passed, "mirror_plan_consumed")));
  assert!(as_bool(get(passed, "dry_run_only")));
  assert!(as_bool(get(passed, "forward_apply_simulated")));
  assert!(as_bool(get(passed, "rollback_restored_original")));
  assert!(as_bool(get(passed, "rollback_handle_consumable")));
  assert!(as_bool(get(passed, "all_post_apply_hashes_match")));
  assert_eq!(as_i64(get(passed, "edit_count")), 1);
  assert_eq!(
    as_str(get(passed, "next_gate")),
    "coding-project-final-file-write-approval-gate"
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

  let dry_runs = as_list(get(passed, "dry_runs"));
  assert_eq!(dry_runs.len(), 1);
  let dry_run = &dry_runs[0];
  assert!(as_bool(get(dry_run, "edit_contract_ok")));
  assert!(as_bool(get(dry_run, "host_operation_matches_edit")));
  assert!(as_bool(get(dry_run, "rollback_handle_matches_edit")));
  assert!(as_bool(get(dry_run, "file_snapshot_matched")));
  assert!(as_bool(get(dry_run, "forward_apply_simulated")));
  assert!(as_bool(get(dry_run, "post_apply_hash_matches")));
  assert!(as_bool(get(dry_run, "rollback_apply_simulated")));
  assert!(as_bool(get(dry_run, "rollback_restored_original")));
  assert!(as_bool(get(dry_run, "rollback_handle_consumable")));
  assert!(as_bool(get(dry_run, "ok")));

  let receipt = get(passed, "receipt");
  assert!(as_bool(get(receipt, "transaction_dry_run_passed")));
  assert_eq!(
    as_str(get(receipt, "invariant")),
    "project host transaction envelope forwards and rolls back in memory; host apply, file write, test execution, search, and policy persistence remain locked"
  );
}

#[test]
fn missing_mirror_snapshot_mismatches_and_effect_requests_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let missing = get(&run, "missing-mirror");
  assert!(as_bool(get(missing, "is_held")));
  assert_eq!(
    as_str(get(missing, "outcome")),
    "held-coding-project-host-apply-transaction-dry-run-mirror-plan-required"
  );

  let content_hash = get(&run, "content-hash-mismatch");
  assert!(as_bool(get(content_hash, "is_held")));
  assert_eq!(
    as_str(get(content_hash, "outcome")),
    "held-coding-project-host-apply-transaction-dry-run-simulation-failed"
  );
  let content_hash_runs = as_list(get(content_hash, "dry_runs"));
  assert!(!as_bool(get(
    &content_hash_runs[0],
    "file_snapshot_matched"
  )));

  let forward = get(&run, "forward-missing");
  assert!(as_bool(get(forward, "is_held")));
  assert_eq!(
    as_str(get(forward, "outcome")),
    "held-coding-project-host-apply-transaction-dry-run-simulation-failed"
  );
  let forward_runs = as_list(get(forward, "dry_runs"));
  assert!(!as_bool(get(&forward_runs[0], "forward_apply_simulated")));

  let post = get(&run, "post-hash-mismatch");
  assert!(as_bool(get(post, "is_held")));
  assert_eq!(
    as_str(get(post, "outcome")),
    "held-coding-project-host-apply-transaction-dry-run-simulation-failed"
  );
  let post_runs = as_list(get(post, "dry_runs"));
  assert!(!as_bool(get(&post_runs[0], "post_apply_hash_matches")));

  let effect = get(&run, "effect-held");
  assert!(as_bool(get(effect, "is_held")));
  assert_eq!(
    as_str(get(effect, "outcome")),
    "held-coding-project-host-apply-transaction-dry-run-effect-blocked"
  );
  assert!(!as_bool(get(effect, "host_apply_allowed")));
  assert!(!as_bool(get(effect, "file_write_allowed")));
  assert!(!as_bool(get(effect, "test_execution_allowed")));
  assert!(!as_bool(get(effect, "search_execution_allowed")));
}
