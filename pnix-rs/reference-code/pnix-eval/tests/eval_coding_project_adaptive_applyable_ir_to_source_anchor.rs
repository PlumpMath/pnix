use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/pnix-query-runtime/coding-project-adaptive-applyable-ir-to-source-anchor.px",
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
  let run =
    eval_file(&fixture_path()).expect("adaptive applyable IR to source anchor fixture evaluates");
  assert_eq!(
    as_str(get(&run, "proof")),
    "coding-project-adaptive-applyable-ir-to-source-anchor"
  );
}

#[test]
fn adaptive_applyable_ir_anchors_against_host_snapshot_without_effects() {
  let run = eval_file(&fixture_path()).unwrap();

  let applyable = get(&run, "applyable-ir");
  assert_eq!(
    as_str(get(applyable, "outcome")),
    "coding-project-applyable-ir-built"
  );
  assert_eq!(
    as_str(get(applyable, "next_gate")),
    "coding-project-source-anchor-check"
  );
  assert_eq!(as_i64(get(applyable, "edit_count")), 2);

  let source_anchor = get(&run, "source-anchor-check");
  assert_eq!(
    as_str(get(source_anchor, "schema")),
    "puncheetah.code.source-anchor-check.v0"
  );
  assert_eq!(
    as_str(get(source_anchor, "outcome")),
    "coding-project-source-anchor-check-passed"
  );
  assert!(as_bool(get(source_anchor, "verified")));
  assert!(as_bool(get(source_anchor, "source_anchor_checked")));
  assert!(as_bool(get(source_anchor, "all_old_text_found")));
  assert!(as_bool(get(source_anchor, "all_old_text_unique")));
  assert!(as_bool(get(source_anchor, "all_pre_apply_hashes_match")));
  assert_eq!(as_i64(get(source_anchor, "anchor_count")), 2);
  assert_eq!(
    as_str(get(source_anchor, "next_gate")),
    "coding-project-apply-dry-run"
  );

  assert!(!as_bool(get(source_anchor, "file_write_allowed")));
  assert!(!as_bool(get(source_anchor, "host_execution_allowed")));
  assert!(!as_bool(get(source_anchor, "host_apply_allowed")));
  assert!(!as_bool(get(source_anchor, "direct_apply_allowed")));
  assert!(!as_bool(get(source_anchor, "apply_allowed")));
  assert!(!as_bool(get(source_anchor, "raw_eval_allowed")));
  assert!(!as_bool(get(source_anchor, "test_execution_allowed")));

  let checks = as_list(get(source_anchor, "anchor_checks"));
  assert_eq!(checks.len(), 2);
  let first = &checks[0];
  assert_eq!(as_str(get(first, "path")), "src/client.rs");
  assert!(as_bool(get(first, "snapshot_file_found")));
  assert!(as_bool(get(first, "pre_apply_hash_matches")));
  assert!(as_bool(get(first, "anchor_found")));
  assert!(as_bool(get(first, "anchor_unique")));
  assert_eq!(as_i64(get(first, "anchor_index")), 21);
}

#[test]
fn hash_drift_missing_anchor_ambiguous_anchor_and_effects_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let hash = get(&run, "hash-mismatch");
  assert!(as_bool(get(hash, "is_held")));
  assert_eq!(
    as_str(get(hash, "outcome")),
    "held-coding-project-source-hash-mismatch"
  );

  let missing = get(&run, "missing-anchor");
  assert!(as_bool(get(missing, "is_held")));
  assert_eq!(
    as_str(get(missing, "outcome")),
    "held-coding-project-source-anchor-missing"
  );

  let ambiguous = get(&run, "ambiguous-anchor");
  assert!(as_bool(get(ambiguous, "is_held")));
  assert_eq!(
    as_str(get(ambiguous, "outcome")),
    "held-coding-project-source-anchor-ambiguous"
  );

  let effect = get(&run, "effect-held");
  assert!(as_bool(get(effect, "is_held")));
  assert_eq!(
    as_str(get(effect, "outcome")),
    "held-coding-project-source-anchor-effect-blocked"
  );
  assert!(!as_bool(get(effect, "file_write_allowed")));
}
