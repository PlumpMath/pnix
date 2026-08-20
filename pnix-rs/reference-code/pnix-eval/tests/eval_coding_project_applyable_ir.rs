use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/coding-project-applyable-ir-receipt.px")
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
  let run = eval_file(&fixture_path()).expect("coding project applyable IR fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "proof")),
    "coding-project-applyable-ir-receipt"
  );

  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "puncheetah.contract.coding-project-applyable-ir.v0"
  );
  assert_eq!(as_str(get(meta, "base")), "coding-project-applyable-ir-v0");
}

#[test]
fn approved_preview_lowers_to_exact_text_file_edits_without_effects() {
  let run = eval_file(&fixture_path()).unwrap();
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
  assert!(!as_bool(get(applyable, "file_write_allowed")));
  assert!(!as_bool(get(applyable, "host_execution_allowed")));
  assert!(!as_bool(get(applyable, "host_apply_allowed")));
  assert!(!as_bool(get(applyable, "direct_apply_allowed")));
  assert!(!as_bool(get(applyable, "apply_allowed")));
  assert!(!as_bool(get(applyable, "raw_eval_allowed")));
  assert!(!as_bool(get(applyable, "test_execution_allowed")));

  let edits = as_list(get(applyable, "file_edits"));
  assert_eq!(edits.len(), 2);
  let first = &edits[0];
  assert_eq!(as_str(get(first, "path")), "src/module.ext");
  assert_eq!(as_str(get(first, "edit_kind")), "replace-exact-text");
  assert_eq!(
    as_str(get(first, "rollback_text")),
    as_str(get(first, "old_text"))
  );
  assert!(as_bool(get(first, "ready_for_source_anchor_check")));

  let test_plan = get(applyable, "test_plan");
  assert_eq!(
    as_str(get(test_plan, "baseline_test_command")),
    "project test command from manifest"
  );
  assert!(!as_bool(get(test_plan, "test_execution_allowed")));

  let receipt = get(applyable, "receipt");
  assert_eq!(
    as_str(get(receipt, "invariant")),
    "approved project patch preview is lowered to exact-text file edits with rollback text; no write/apply/host/test effect is allowed"
  );
}

#[test]
fn effect_requests_and_preview_approval_mismatch_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let mismatch = get(&run, "mismatch-held");
  assert!(as_bool(get(mismatch, "is_held")));
  assert_eq!(
    as_str(get(mismatch, "outcome")),
    "held-coding-project-applyable-ir-preview-approval-mismatch"
  );
  assert!(!as_bool(get(mismatch, "applyable_project_patch_ir_built")));

  let effect = get(&run, "effect-held");
  assert!(as_bool(get(effect, "is_held")));
  assert_eq!(
    as_str(get(effect, "outcome")),
    "held-coding-project-applyable-ir-effect-blocked"
  );
  assert!(!as_bool(get(effect, "file_write_allowed")));
}
