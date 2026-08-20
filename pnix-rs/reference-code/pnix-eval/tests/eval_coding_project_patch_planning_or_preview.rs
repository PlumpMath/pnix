use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/coding-project-patch-planning-or-preview.px")
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

fn assert_effects_locked(v: &Value) {
  assert!(!as_bool(get(v, "host_apply_allowed")));
  assert!(!as_bool(get(v, "file_write_allowed")));
  assert!(!as_bool(get(v, "host_execution_allowed")));
  assert!(!as_bool(get(v, "apply_allowed")));
  assert!(!as_bool(get(v, "raw_eval_allowed")));
  assert!(!as_bool(get(v, "test_execution_allowed")));
  assert!(!as_bool(get(v, "search_execution_allowed")));
  assert!(!as_bool(get(v, "compiler_execution_allowed")));
  assert!(!as_bool(get(v, "lsp_execution_allowed")));
  assert!(!as_bool(get(v, "memory_write_allowed")));
  assert!(!as_bool(get(v, "db_write_allowed")));
  assert!(!as_bool(get(v, "policy_persistence_allowed")));
  assert!(!as_bool(get(v, "source_ingest_allowed")));
  assert!(!as_bool(get(v, "search_evidence_accept_allowed")));
  assert!(!as_bool(get(v, "accepted_fact_promotion_allowed")));
  assert!(!as_bool(get(v, "learning_promotion_allowed")));
  assert!(!as_bool(get(v, "code_write_allowed")));
  assert!(!as_bool(get(v, "route_execution_allowed")));
  assert!(!as_bool(get(v, "route_policy_update_allowed")));
}

#[test]
fn fixture_evaluates_with_pnix_eval_not_nix() {
  let run = eval_file(&fixture_path()).expect("patch planning/preview fixture evaluates");
  assert_eq!(
    as_str(get(&run, "proof")),
    "coding-project-patch-planning-or-preview"
  );

  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "puncheetah.contract.coding-project-patch-planning-or-preview.v0"
  );
  assert_eq!(
    as_str(get(meta, "base")),
    "coding-project-patch-planning-or-preview-v0"
  );
}

#[test]
fn reopened_plan_and_candidate_build_reviewable_patch_preview_without_effects() {
  let run = eval_file(&fixture_path()).unwrap();
  let built = get(&run, "built-preview");

  assert_eq!(
    as_str(get(built, "outcome")),
    "coding-project-patch-planning-or-preview-built"
  );
  assert!(as_bool(get(built, "verified")));
  assert!(as_bool(get(built, "patch_planning_or_preview_built")));
  assert!(as_bool(get(
    built,
    "source_coding_expression_plan_reopen_verified"
  )));
  assert!(as_bool(get(built, "patch_preview_candidate_verified")));
  assert!(as_bool(get(built, "patch_preview_built")));
  assert!(as_bool(get(built, "candidate_evidence_preserved")));
  assert!(as_bool(get(built, "candidate_evidence_only")));
  assert!(!as_bool(get(built, "accepted_fact_allowed")));
  assert!(!as_bool(get(built, "accepted_fact_promotion_allowed")));
  assert!(!as_bool(get(built, "learning_promotion_allowed")));
  assert_eq!(
    as_str(get(built, "next_gate")),
    "coding-project-patch-preview-review"
  );
  assert_effects_locked(built);

  let preview = get(built, "patch_preview");
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
  assert!(as_bool(get(preview, "preview_available")));
  assert_eq!(as_list(get(preview, "file_patches")).len(), 2);

  let checks = as_list(get(built, "checks"));
  assert_eq!(checks.len(), 11);
  assert_eq!(as_list(get(built, "failed_checks")).len(), 0);
}

#[test]
fn built_preview_passes_existing_project_patch_preview_review_gate() {
  let run = eval_file(&fixture_path()).unwrap();
  let review = get(&run, "downstream-review");

  assert_eq!(
    as_str(get(review, "outcome")),
    "coding-project-patch-preview-reviewed"
  );
  assert!(as_bool(get(review, "verified")));
  assert_eq!(as_str(get(review, "review_status")), "reviewable");
  assert!(as_bool(get(review, "approval_required")));
  assert_eq!(
    as_str(get(review, "next_gate")),
    "coding-project-apply-approval-gate"
  );
  assert!(!as_bool(get(review, "file_write_allowed")));
  assert!(!as_bool(get(review, "host_execution_allowed")));
}

#[test]
fn missing_bad_or_mismatched_sources_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let missing_plan = get(&run, "missing-plan-held");
  assert!(as_bool(get(missing_plan, "is_held")));
  assert_eq!(
    as_str(get(missing_plan, "outcome")),
    "held-coding-project-patch-planning-reopened-plan-required"
  );

  let unverified_plan = get(&run, "unverified-plan-held");
  assert!(as_bool(get(unverified_plan, "is_held")));
  assert_eq!(
    as_str(get(unverified_plan, "outcome")),
    "held-coding-project-patch-planning-reopened-plan-unverified"
  );

  let missing_candidate = get(&run, "missing-candidate-held");
  assert!(as_bool(get(missing_candidate, "is_held")));
  assert_eq!(
    as_str(get(missing_candidate, "outcome")),
    "held-coding-project-patch-planning-preview-candidate-required"
  );

  let mismatch = get(&run, "mismatch-held");
  assert!(as_bool(get(mismatch, "is_held")));
  assert_eq!(
    as_str(get(mismatch, "outcome")),
    "held-coding-project-patch-planning-context-mismatch"
  );

  let unsafe_path = get(&run, "unsafe-path-held");
  assert!(as_bool(get(unsafe_path, "is_held")));
  assert_eq!(
    as_str(get(unsafe_path, "outcome")),
    "held-coding-project-patch-planning-unsafe-path"
  );

  let invalid_candidate = get(&run, "invalid-candidate-held");
  assert!(as_bool(get(invalid_candidate, "is_held")));
  assert_eq!(
    as_str(get(invalid_candidate, "outcome")),
    "held-coding-project-patch-planning-preview-candidate-invalid"
  );
}

#[test]
fn effect_and_promotion_requests_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let effect = get(&run, "effect-held");
  assert!(as_bool(get(effect, "is_held")));
  assert_eq!(
    as_str(get(effect, "outcome")),
    "held-coding-project-patch-planning-effect-blocked"
  );
  assert_effects_locked(effect);

  let promotion = get(&run, "promotion-held");
  assert!(as_bool(get(promotion, "is_held")));
  assert_eq!(
    as_str(get(promotion, "outcome")),
    "held-coding-project-patch-planning-promotion-blocked"
  );
  assert_effects_locked(promotion);
}

#[test]
fn dispatch_and_mirror_connect_patch_preview_to_review_gate() {
  let run = eval_file(&fixture_path()).unwrap();

  let dispatched = get(&run, "dispatched");
  assert_eq!(
    as_str(get(dispatched, "op")),
    "build-coding-project-patch-planning-or-preview"
  );
  let result = get(dispatched, "result");
  assert_eq!(
    as_str(get(result, "outcome")),
    "coding-project-patch-planning-or-preview-built"
  );
  assert_eq!(
    as_str(get(result, "next_gate")),
    "coding-project-patch-preview-review"
  );

  let observed = get(&run, "observed-built");
  assert_eq!(
    as_str(get(observed, "outcome")),
    "mirror-self-observation-plan-built"
  );
  let observation = get(observed, "observation");
  assert_eq!(
    as_str(get(observation, "observed_outcome")),
    "coding-project-patch-planning-or-preview-built"
  );
  assert_eq!(
    as_str(get(observed, "next_action")),
    "build-coding-project-patch-preview-review"
  );
}
