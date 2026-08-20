use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/coding-project-source-anchor-check-receipt.px")
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
  let run = eval_file(&fixture_path()).expect("coding project source anchor fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "proof")),
    "coding-project-source-anchor-check-receipt"
  );

  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "puncheetah.contract.coding-project-source-anchor-check.v0"
  );
  assert_eq!(
    as_str(get(meta, "base")),
    "coding-project-source-anchor-check-v0"
  );
}

#[test]
fn source_snapshot_anchors_applyable_ir_without_effects() {
  let run = eval_file(&fixture_path()).unwrap();
  let passed = get(&run, "passed");

  assert_eq!(
    as_str(get(passed, "schema")),
    "puncheetah.code.source-anchor-check.v0"
  );
  assert_eq!(
    as_str(get(passed, "outcome")),
    "coding-project-source-anchor-check-passed"
  );
  assert!(as_bool(get(passed, "verified")));
  assert!(as_bool(get(passed, "source_anchor_checked")));
  assert!(as_bool(get(passed, "all_old_text_found")));
  assert!(as_bool(get(passed, "all_old_text_unique")));
  assert!(as_bool(get(passed, "all_pre_apply_hashes_match")));
  assert_eq!(as_i64(get(passed, "anchor_count")), 2);
  assert_eq!(
    as_str(get(passed, "next_gate")),
    "coding-project-apply-dry-run"
  );

  assert!(!as_bool(get(passed, "file_write_allowed")));
  assert!(!as_bool(get(passed, "host_execution_allowed")));
  assert!(!as_bool(get(passed, "host_apply_allowed")));
  assert!(!as_bool(get(passed, "direct_apply_allowed")));
  assert!(!as_bool(get(passed, "apply_allowed")));
  assert!(!as_bool(get(passed, "raw_eval_allowed")));
  assert!(!as_bool(get(passed, "test_execution_allowed")));

  let checks = as_list(get(passed, "anchor_checks"));
  assert_eq!(checks.len(), 2);
  let first = &checks[0];
  assert_eq!(as_str(get(first, "path")), "src/module.ext");
  assert!(as_bool(get(first, "snapshot_file_found")));
  assert!(as_bool(get(first, "pre_apply_hash_matches")));
  assert!(as_bool(get(first, "anchor_found")));
  assert!(as_bool(get(first, "anchor_unique")));

  let receipt = get(passed, "receipt");
  assert_eq!(
    as_str(get(receipt, "invariant")),
    "host source snapshot matches applyable project patch IR hashes and every old_text anchor is present exactly once; no write/apply/host/test effect is allowed"
  );
}

#[test]
fn stale_hash_missing_anchor_ambiguous_anchor_and_effects_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let hash = get(&run, "hash-mismatch");
  assert!(as_bool(get(hash, "is_held")));
  assert_eq!(
    as_str(get(hash, "outcome")),
    "held-coding-project-source-hash-mismatch"
  );
  assert!(!as_bool(get(hash, "all_pre_apply_hashes_match")));

  let missing = get(&run, "missing-anchor");
  assert!(as_bool(get(missing, "is_held")));
  assert_eq!(
    as_str(get(missing, "outcome")),
    "held-coding-project-source-anchor-missing"
  );
  assert!(!as_bool(get(missing, "all_old_text_found")));

  let ambiguous = get(&run, "ambiguous-anchor");
  assert!(as_bool(get(ambiguous, "is_held")));
  assert_eq!(
    as_str(get(ambiguous, "outcome")),
    "held-coding-project-source-anchor-ambiguous"
  );
  assert!(!as_bool(get(ambiguous, "all_old_text_unique")));

  let effect = get(&run, "effect-held");
  assert!(as_bool(get(effect, "is_held")));
  assert_eq!(
    as_str(get(effect, "outcome")),
    "held-coding-project-source-anchor-effect-blocked"
  );
  assert!(!as_bool(get(effect, "file_write_allowed")));
}
