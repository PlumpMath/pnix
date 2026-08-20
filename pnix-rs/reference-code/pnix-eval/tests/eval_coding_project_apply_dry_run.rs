use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/coding-project-apply-dry-run-receipt.px")
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
  let run = eval_file(&fixture_path()).expect("coding project apply dry-run fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "proof")),
    "coding-project-apply-dry-run-receipt"
  );

  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "puncheetah.contract.coding-project-apply-dry-run.v0"
  );
  assert_eq!(as_str(get(meta, "base")), "coding-project-apply-dry-run-v0");
}

#[test]
fn dry_run_applies_in_memory_checks_post_hashes_and_rolls_back() {
  let run = eval_file(&fixture_path()).unwrap();
  let passed = get(&run, "passed");

  assert_eq!(
    as_str(get(passed, "schema")),
    "puncheetah.code.apply-dry-run.v0"
  );
  assert_eq!(
    as_str(get(passed, "outcome")),
    "coding-project-apply-dry-run-passed"
  );
  assert!(as_bool(get(passed, "verified")));
  assert!(as_bool(get(passed, "dry_run_only")));
  assert!(as_bool(get(passed, "dry_run_applied")));
  assert!(as_bool(get(passed, "rollback_restored_original")));
  assert!(as_bool(get(passed, "all_post_apply_hashes_match")));
  assert_eq!(as_i64(get(passed, "dry_run_count")), 2);
  assert_eq!(
    as_str(get(passed, "next_gate")),
    "coding-project-test-plan-receipt"
  );

  assert!(!as_bool(get(passed, "file_write_allowed")));
  assert!(!as_bool(get(passed, "host_execution_allowed")));
  assert!(!as_bool(get(passed, "host_apply_allowed")));
  assert!(!as_bool(get(passed, "direct_apply_allowed")));
  assert!(!as_bool(get(passed, "apply_allowed")));
  assert!(!as_bool(get(passed, "raw_eval_allowed")));
  assert!(!as_bool(get(passed, "test_execution_allowed")));

  let dry_runs = as_list(get(passed, "dry_runs"));
  assert_eq!(dry_runs.len(), 2);
  let first = &dry_runs[0];
  assert_eq!(as_str(get(first, "path")), "src/module.ext");
  assert!(as_bool(get(first, "forward_applied")));
  assert!(as_bool(get(first, "post_apply_hash_matches")));
  assert!(as_bool(get(first, "rollback_restored_original")));
  assert_eq!(
    as_str(get(first, "computed_post_apply_sha256")),
    as_str(get(first, "expected_post_apply_sha256"))
  );

  let receipt = get(passed, "receipt");
  assert_eq!(
    as_str(get(receipt, "invariant")),
    "source-anchor-checked applyable project patch IR applies in memory, computed post hashes match, rollback restores original snapshot content, and write/apply/host/test effects stay locked"
  );
}

#[test]
fn bad_post_hash_stale_anchor_and_effect_requests_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let post_hash = get(&run, "post-hash-mismatch");
  assert!(as_bool(get(post_hash, "is_held")));
  assert_eq!(
    as_str(get(post_hash, "outcome")),
    "held-coding-project-dry-run-post-hash-mismatch"
  );
  assert!(!as_bool(get(post_hash, "all_post_apply_hashes_match")));

  let forward = get(&run, "forward-apply-failed");
  assert!(as_bool(get(forward, "is_held")));
  assert_eq!(
    as_str(get(forward, "outcome")),
    "held-coding-project-dry-run-forward-apply-failed"
  );
  assert!(!as_bool(get(forward, "dry_run_applied")));

  let effect = get(&run, "effect-held");
  assert!(as_bool(get(effect, "is_held")));
  assert_eq!(
    as_str(get(effect, "outcome")),
    "held-coding-project-apply-dry-run-effect-blocked"
  );
  assert!(!as_bool(get(effect, "file_write_allowed")));
}
