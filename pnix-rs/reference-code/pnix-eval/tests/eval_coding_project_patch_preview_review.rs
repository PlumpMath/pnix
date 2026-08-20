use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/project-patch-preview-review-receipt.px")
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
  let run = eval_file(&fixture_path()).expect("project patch preview review fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "proof")),
    "project-patch-preview-review-receipt"
  );

  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "puncheetah.contract.project-patch-preview-review.v0"
  );
  assert_eq!(as_str(get(meta, "base")), "project-patch-preview-review-v0");
}

#[test]
fn valid_preview_becomes_reviewable_approval_artifact_without_effects() {
  let run = eval_file(&fixture_path()).unwrap();
  let review = get(&run, "valid-review");

  assert_eq!(
    as_str(get(review, "outcome")),
    "coding-project-patch-preview-reviewed"
  );
  assert!(as_bool(get(review, "verified")));
  assert_eq!(as_str(get(review, "review_status")), "reviewable");
  assert_eq!(
    as_str(get(review, "next_gate")),
    "coding-project-apply-approval-gate"
  );
  assert!(as_bool(get(review, "approval_required")));
  assert_eq!(as_i64(get(review, "file_patch_count")), 2);
  assert!(!as_bool(get(review, "file_write_allowed")));
  assert!(!as_bool(get(review, "host_execution_allowed")));
  assert!(!as_bool(get(review, "direct_apply_allowed")));
  assert!(!as_bool(get(review, "raw_eval_allowed")));

  let requirements = get(review, "approval_requirements");
  assert_eq!(
    as_str(get(requirements, "approval_kind")),
    "coding-project-patch-approval-token-v0"
  );
  assert_eq!(
    as_str(get(requirements, "approved_preview_id")),
    "generic-project-patch-preview-demo"
  );
  assert_eq!(as_i64(get(requirements, "approved_file_count")), 2);

  let checks = as_list(get(review, "checks"));
  assert_eq!(checks.len(), 9);
  for check in checks {
    assert!(as_bool(get(check, "ok")), "failed check: {:?}", check);
  }

  let receipt = get(review, "receipt");
  assert_eq!(
    as_str(get(receipt, "invariant")),
    "project patch preview is stable, hash-addressed, generic-task-shaped, and effect-locked before approval"
  );
}

#[test]
fn effect_requests_and_bad_paths_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let effect = get(&run, "effect-held");
  assert!(as_bool(get(effect, "is_held")));
  assert_eq!(
    as_str(get(effect, "outcome")),
    "held-project-patch-preview-effect-blocked"
  );
  assert!(!as_bool(get(effect, "file_write_allowed")));

  let bad_path = get(&run, "bad-path-held");
  assert!(as_bool(get(bad_path, "is_held")));
  assert_eq!(
    as_str(get(bad_path, "outcome")),
    "held-project-patch-preview-not-reviewable"
  );
  assert!(!as_bool(get(bad_path, "direct_apply_allowed")));
}
