use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/coding-expression-plan-reopen.px")
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
  let run = eval_file(&fixture_path()).expect("coding expression plan reopen fixture evaluates");
  assert_eq!(as_str(get(&run, "proof")), "coding-expression-plan-reopen");

  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.agent.coding-expression-plan-reopen"
  );
  assert_eq!(
    as_str(get(meta, "base")),
    "coding-expression-plan-reopen-v0"
  );
}

#[test]
fn verified_synthesis_reopens_coding_expression_plan_without_promotion_or_write() {
  let run = eval_file(&fixture_path()).unwrap();
  let reopened = get(&run, "reopened");

  assert_eq!(
    as_str(get(reopened, "schema")),
    "puncheetah.coding-expression-plan-reopen.v0"
  );
  assert_eq!(
    as_str(get(reopened, "outcome")),
    "coding-expression-plan-reopened"
  );
  assert!(as_bool(get(reopened, "verified")));
  assert!(as_bool(get(reopened, "coding_expression_plan_reopened")));
  assert!(as_bool(get(reopened, "source_adaptive_synthesis_verified")));
  assert!(as_bool(get(
    reopened,
    "prior_coding_expression_plan_verified"
  )));
  assert!(as_bool(get(reopened, "candidate_evidence_consumed")));
  assert!(as_bool(get(reopened, "candidate_evidence_preserved")));
  assert!(as_bool(get(reopened, "candidate_evidence_only")));
  assert!(!as_bool(get(reopened, "accepted_fact_allowed")));
  assert!(!as_bool(get(reopened, "accepted_fact_promotion_allowed")));
  assert!(!as_bool(get(reopened, "learning_promotion_allowed")));
  assert_eq!(
    as_str(get(reopened, "next_gate")),
    "coding-project-patch-planning-or-preview"
  );
  assert_effects_locked(reopened);

  let resolved = as_list(get(reopened, "resolved_semantic_slots"));
  assert_eq!(resolved.len(), 2);
  assert_eq!(
    as_str(get(&resolved[0], "status")),
    "candidate-evidence-resolved-for-planning"
  );
  assert!(!as_bool(get(&resolved[0], "accepted_fact")));
  assert_eq!(as_list(get(reopened, "remaining_semantic_slots")).len(), 0);

  let api = as_list(get(reopened, "required_api_affordances"));
  assert_eq!(api.len(), 2);
  assert_eq!(
    as_str(get(&api[0], "status")),
    "candidate-evidence-verified-for-plan-reopen"
  );
  assert!(!as_bool(get(&api[0], "accepted_fact")));

  let patch_input = get(reopened, "patch_planning_input");
  assert_eq!(
    as_str(get(patch_input, "status")),
    "ready-for-patch-planning"
  );
  assert!(!as_bool(get(patch_input, "code_write_allowed")));
  assert!(!as_bool(get(patch_input, "accepted_fact_allowed")));

  let steps = as_list(get(reopened, "reopen_steps"));
  assert_eq!(steps.len(), 4);
  assert_eq!(
    as_str(get(&steps[0], "outcome")),
    "adaptive-evidence-synthesis-verified"
  );
}

#[test]
fn no_missing_slot_reopen_keeps_empty_resolved_and_avoidance_lists() {
  let run = eval_file(&fixture_path()).unwrap();
  let reopened = get(&run, "no-missing-reopened");

  assert_eq!(
    as_str(get(reopened, "outcome")),
    "coding-expression-plan-reopened"
  );
  assert_eq!(as_list(get(reopened, "resolved_semantic_slots")).len(), 0);
  assert_eq!(as_list(get(reopened, "required_api_affordances")).len(), 0);
  assert_eq!(
    as_list(get(reopened, "route_avoidance_candidates")).len(),
    0
  );
  assert!(!as_bool(get(reopened, "promotion_candidate_gate_required")));
  assert_effects_locked(reopened);
}

#[test]
fn non_reopenable_synthesis_and_missing_or_bad_prior_plan_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let missing_synthesis = get(&run, "missing-synthesis-held");
  assert!(as_bool(get(missing_synthesis, "is_held")));
  assert_eq!(
    as_str(get(missing_synthesis, "outcome")),
    "held-coding-expression-plan-reopen-synthesis-required"
  );

  let unverified_synthesis = get(&run, "unverified-synthesis-held");
  assert!(as_bool(get(unverified_synthesis, "is_held")));
  assert_eq!(
    as_str(get(unverified_synthesis, "outcome")),
    "held-coding-expression-plan-reopen-synthesis-not-reopenable"
  );

  let additional = get(&run, "additional-evidence-held");
  assert!(as_bool(get(additional, "is_held")));
  assert_eq!(
    as_str(get(additional, "outcome")),
    "held-coding-expression-plan-reopen-synthesis-not-reopenable"
  );

  let missing_prior = get(&run, "missing-prior-held");
  assert!(as_bool(get(missing_prior, "is_held")));
  assert_eq!(
    as_str(get(missing_prior, "outcome")),
    "held-coding-expression-plan-reopen-prior-plan-required"
  );

  let unverified_prior = get(&run, "unverified-prior-held");
  assert!(as_bool(get(unverified_prior, "is_held")));
  assert_eq!(
    as_str(get(unverified_prior, "outcome")),
    "held-coding-expression-plan-reopen-prior-plan-unverified"
  );
}

#[test]
fn effect_and_promotion_requests_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let effect = get(&run, "effect-held");
  assert!(as_bool(get(effect, "is_held")));
  assert_eq!(
    as_str(get(effect, "outcome")),
    "held-coding-expression-plan-reopen-effect-blocked"
  );
  assert_effects_locked(effect);

  let promotion = get(&run, "promotion-held");
  assert!(as_bool(get(promotion, "is_held")));
  assert_eq!(
    as_str(get(promotion, "outcome")),
    "held-coding-expression-plan-reopen-promotion-blocked"
  );
  assert_effects_locked(promotion);
}

#[test]
fn dispatch_and_mirror_connect_reopened_plan_to_patch_planning() {
  let run = eval_file(&fixture_path()).unwrap();

  let dispatched = get(&run, "dispatched");
  assert_eq!(
    as_str(get(dispatched, "op")),
    "reopen-coding-expression-plan"
  );
  let result = get(dispatched, "result");
  assert_eq!(
    as_str(get(result, "outcome")),
    "coding-expression-plan-reopened"
  );
  assert_eq!(
    as_str(get(result, "next_gate")),
    "coding-project-patch-planning-or-preview"
  );

  let observed = get(&run, "observed-reopen");
  assert_eq!(
    as_str(get(observed, "outcome")),
    "mirror-self-observation-plan-built"
  );
  let observation = get(observed, "observation");
  assert_eq!(
    as_str(get(observation, "observed_outcome")),
    "coding-expression-plan-reopened"
  );
  assert_eq!(
    as_str(get(observed, "next_action")),
    "build-coding-project-patch-planning-or-preview"
  );
}
