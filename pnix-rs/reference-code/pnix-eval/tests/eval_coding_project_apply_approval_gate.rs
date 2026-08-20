use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/coding-project-apply-approval-gate-receipt.px")
}

fn as_attrs(v: &Value) -> &BTreeMap<String, Value> {
  match v {
    Value::AttrSet(m) => m,
    other => panic!("expected attrset, got {:?}", other),
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
    eval_file(&fixture_path()).expect("coding project apply approval fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "proof")),
    "coding-project-apply-approval-gate-receipt"
  );

  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "puncheetah.contract.coding-project-apply-approval-gate.v0"
  );
  assert_eq!(
    as_str(get(meta, "base")),
    "coding-project-apply-approval-gate-v0"
  );
}

#[test]
fn matching_approval_token_opens_applyable_ir_gate_without_effects() {
  let run = eval_file(&fixture_path()).unwrap();
  let approved = get(&run, "approved");

  assert_eq!(
    as_str(get(approved, "outcome")),
    "coding-project-apply-approval-gate-approved"
  );
  assert!(as_bool(get(approved, "verified")));
  assert!(as_bool(get(approved, "approval_token_verified")));
  assert!(as_bool(get(approved, "project_patch_apply_approved")));
  assert_eq!(
    as_str(get(approved, "next_gate")),
    "coding-project-applyable-ir"
  );
  assert_eq!(
    as_str(get(approved, "approved_preview_id")),
    "generic-project-patch-preview-demo"
  );
  assert_eq!(
    as_str(get(approved, "approved_preview_hash")),
    "sha256-generic-preview-demo"
  );
  assert_eq!(as_i64(get(approved, "approved_file_count")), 2);
  assert!(!as_bool(get(approved, "file_write_allowed")));
  assert!(!as_bool(get(approved, "host_execution_allowed")));
  assert!(!as_bool(get(approved, "host_apply_allowed")));
  assert!(!as_bool(get(approved, "direct_apply_allowed")));
  assert!(!as_bool(get(approved, "apply_allowed")));
  assert!(!as_bool(get(approved, "raw_eval_allowed")));

  let audit = get(approved, "approval_audit");
  assert_eq!(as_str(get(audit, "approval_id")), "approval-generic-demo");
  assert_eq!(
    as_str(get(audit, "review_outcome")),
    "coding-project-patch-preview-reviewed"
  );

  let receipt = get(approved, "receipt");
  assert_eq!(
    as_str(get(receipt, "invariant")),
    "human approval matches reviewed project patch preview; no apply, write, host execution, or raw eval allowed"
  );
}

#[test]
fn effect_requests_and_approval_mismatch_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let mismatch = get(&run, "mismatch-held");
  assert!(as_bool(get(mismatch, "is_held")));
  assert_eq!(
    as_str(get(mismatch, "outcome")),
    "held-project-patch-approval-token-mismatch"
  );
  assert!(!as_bool(get(mismatch, "project_patch_apply_approved")));
  assert!(!as_bool(get(mismatch, "file_write_allowed")));

  let effect = get(&run, "effect-held");
  assert!(as_bool(get(effect, "is_held")));
  assert_eq!(
    as_str(get(effect, "outcome")),
    "held-coding-project-apply-approval-effect-blocked"
  );
  assert!(!as_bool(get(effect, "apply_allowed")));
}
