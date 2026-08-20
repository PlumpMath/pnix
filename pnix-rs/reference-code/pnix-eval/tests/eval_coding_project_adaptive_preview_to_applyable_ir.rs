use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/coding-project-adaptive-preview-to-applyable-ir.px")
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
  let run = eval_file(&fixture_path()).expect("adaptive preview to applyable IR fixture evaluates");
  assert_eq!(
    as_str(get(&run, "proof")),
    "coding-project-adaptive-preview-to-applyable-ir"
  );
}

#[test]
fn p16_preview_reenters_review_approval_and_applyable_ir_spine() {
  let run = eval_file(&fixture_path()).unwrap();

  let source = get(&run, "source-patch-planning");
  assert_eq!(
    as_str(get(source, "outcome")),
    "coding-project-patch-planning-or-preview-built"
  );
  assert!(as_bool(get(source, "candidate_evidence_only")));
  assert!(!as_bool(get(source, "accepted_fact_promotion_allowed")));

  let preview = get(&run, "patch-preview");
  assert_eq!(
    as_str(get(preview, "schema")),
    "puncheetah.code.patch-preview.v0"
  );
  assert_eq!(
    as_str(get(preview, "outcome")),
    "coding-project-patch-preview-built"
  );
  assert_eq!(
    as_str(get(preview, "next_gate")),
    "coding-project-patch-preview-review"
  );

  let reviewed = get(&run, "reviewed");
  assert_eq!(
    as_str(get(reviewed, "outcome")),
    "coding-project-patch-preview-reviewed"
  );
  assert!(as_bool(get(reviewed, "verified")));
  assert_eq!(as_str(get(reviewed, "review_status")), "reviewable");
  assert_eq!(
    as_str(get(reviewed, "next_gate")),
    "coding-project-apply-approval-gate"
  );

  let approved = get(&run, "approved");
  assert_eq!(
    as_str(get(approved, "outcome")),
    "coding-project-apply-approval-gate-approved"
  );
  assert!(as_bool(get(approved, "verified")));
  assert!(as_bool(get(approved, "approval_token_verified")));
  assert_eq!(
    as_str(get(approved, "next_gate")),
    "coding-project-applyable-ir"
  );

  let applyable = get(&run, "applyable");
  assert_eq!(
    as_str(get(applyable, "schema")),
    "puncheetah.code.applyable-project-patch-ir.v0"
  );
  assert_eq!(
    as_str(get(applyable, "outcome")),
    "coding-project-applyable-ir-built"
  );
  assert!(as_bool(get(applyable, "verified")));
  assert!(as_bool(get(applyable, "applyable_project_patch_ir_built")));
  assert_eq!(
    as_str(get(applyable, "next_gate")),
    "coding-project-source-anchor-check"
  );
  assert_eq!(as_i64(get(applyable, "edit_count")), 2);
  assert_eq!(as_list(get(applyable, "file_edits")).len(), 2);
  assert!(!as_bool(get(applyable, "file_write_allowed")));
  assert!(!as_bool(get(applyable, "host_execution_allowed")));
  assert!(!as_bool(get(applyable, "host_apply_allowed")));
  assert!(!as_bool(get(applyable, "apply_allowed")));
  assert!(!as_bool(get(applyable, "test_execution_allowed")));
}

#[test]
fn mismatched_applyable_ir_and_effect_request_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let applyable_mismatch = get(&run, "applyable-mismatch-held");
  assert!(as_bool(get(applyable_mismatch, "is_held")));
  assert_eq!(
    as_str(get(applyable_mismatch, "outcome")),
    "held-coding-project-applyable-ir-preview-approval-mismatch"
  );

  let effect = get(&run, "applyable-effect-held");
  assert!(as_bool(get(effect, "is_held")));
  assert_eq!(
    as_str(get(effect, "outcome")),
    "held-coding-project-applyable-ir-effect-blocked"
  );
  assert!(!as_bool(get(effect, "file_write_allowed")));
  assert!(!as_bool(get(effect, "host_execution_allowed")));
}
