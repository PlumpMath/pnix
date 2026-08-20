use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/coding-expression-adaptation-resilience-plan.px")
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

fn list_contains_str(v: &Value, needle: &str) -> bool {
  as_list(v).iter().any(|item| as_str(item) == needle)
}

fn assert_effects_locked(v: &Value) {
  assert!(!as_bool(get(v, "host_apply_allowed")));
  assert!(!as_bool(get(v, "file_write_allowed")));
  assert!(!as_bool(get(v, "host_execution_allowed")));
  assert!(!as_bool(get(v, "apply_allowed")));
  assert!(!as_bool(get(v, "raw_eval_allowed")));
  assert!(!as_bool(get(v, "test_execution_allowed")));
  assert!(!as_bool(get(v, "search_execution_allowed")));
  assert!(!as_bool(get(v, "memory_write_allowed")));
  assert!(!as_bool(get(v, "db_write_allowed")));
  assert!(!as_bool(get(v, "policy_persistence_allowed")));
  assert!(!as_bool(get(v, "source_ingest_allowed")));
  assert!(!as_bool(get(v, "search_evidence_accept_allowed")));
  assert!(!as_bool(get(v, "code_write_allowed")));
  assert!(!as_bool(get(v, "route_execution_allowed")));
}

#[test]
fn fixture_evaluates_with_pnix_eval_not_nix() {
  let run = eval_file(&fixture_path()).expect("coding expression adaptation fixture evaluates");
  assert_eq!(
    as_str(get(&run, "proof")),
    "coding-expression-adaptation-resilience-plan"
  );

  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.agent.coding-expression-adaptation-resilience-plan"
  );
  assert_eq!(
    as_str(get(meta, "base")),
    "coding-expression-adaptation-resilience-plan-v0"
  );
}

#[test]
fn plan_builds_code_quality_language_api_and_resilience_slots() {
  let run = eval_file(&fixture_path()).unwrap();
  let plan = get(&run, "plan");

  assert_eq!(
    as_str(get(plan, "schema")),
    "puncheetah.coding-expression-adaptation-resilience-plan.v0"
  );
  assert_eq!(
    as_str(get(plan, "outcome")),
    "coding-expression-adaptation-resilience-plan-built"
  );
  assert!(as_bool(get(plan, "verified")));
  assert!(as_bool(get(plan, "adaptation_plan_built")));
  assert!(as_bool(get(plan, "coding_expression_plan_built")));
  assert!(as_bool(get(plan, "code_quality_plan_built")));
  assert!(as_bool(get(plan, "api_affordance_learning_planned")));
  assert!(as_bool(get(plan, "resilience_plan_built")));
  assert_eq!(
    as_str(get(plan, "next_gate")),
    "adaptive-evidence-acquisition-plan"
  );
  assert_effects_locked(plan);

  assert_eq!(as_str(get(plan, "language")), "rust");
  assert!(list_contains_str(
    get(plan, "missing_semantic_slots"),
    "api-affordance"
  ));
  assert!(list_contains_str(
    get(plan, "required_language_concepts"),
    "binding-scope-meaning"
  ));
  assert!(list_contains_str(
    get(plan, "required_language_concepts"),
    "effect-io-boundary"
  ));
  assert!(list_contains_str(
    get(plan, "code_quality_obligations"),
    "idiomatic-language-use"
  ));
  assert!(list_contains_str(
    get(plan, "code_quality_obligations"),
    "testable-behavior"
  ));
  assert!(list_contains_str(
    get(plan, "promotion_blockers"),
    "unverified-search-evidence"
  ));

  let api_needs = as_list(get(plan, "required_api_affordances"));
  assert_eq!(api_needs.len(), 2);
  assert_eq!(
    as_str(get(&api_needs[0], "status")),
    "candidate-needs-verification"
  );
  assert!(!as_bool(get(&api_needs[0], "accepted_fact")));

  let search_plans = as_list(get(plan, "search_need_plans"));
  assert_eq!(search_plans.len(), 1);
  assert!(!as_bool(get(&search_plans[0], "search_execution_allowed")));
  assert!(as_bool(get(&search_plans[0], "candidate_only")));
  assert!(as_bool(get(&search_plans[0], "verification_required")));
}

#[test]
fn no_api_gap_plan_still_requires_code_quality_and_test_obligation() {
  let run = eval_file(&fixture_path()).unwrap();
  let plan = get(&run, "no-api-gap-plan");

  assert_eq!(
    as_str(get(plan, "outcome")),
    "coding-expression-adaptation-resilience-plan-built"
  );
  assert!(as_bool(get(plan, "code_quality_plan_built")));
  assert!(!as_bool(get(plan, "api_affordance_learning_planned")));
  assert!(list_contains_str(
    get(plan, "code_quality_obligations"),
    "correctness-against-user-requirement"
  ));
  assert!(list_contains_str(
    get(plan, "required_language_concepts"),
    "test-obligation"
  ));
  assert_effects_locked(plan);
}

#[test]
fn missing_task_context_effect_and_promotion_requests_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let missing_task = get(&run, "missing-task-held");
  assert!(as_bool(get(missing_task, "is_held")));
  assert_eq!(
    as_str(get(missing_task, "outcome")),
    "held-coding-expression-adaptation-resilience-task-required"
  );

  let missing_project = get(&run, "missing-project-held");
  assert!(as_bool(get(missing_project, "is_held")));
  assert_eq!(
    as_str(get(missing_project, "outcome")),
    "held-coding-expression-adaptation-resilience-project-context-required"
  );

  let effect = get(&run, "effect-held");
  assert!(as_bool(get(effect, "is_held")));
  assert_eq!(
    as_str(get(effect, "outcome")),
    "held-coding-expression-adaptation-resilience-effect-blocked"
  );
  assert_effects_locked(effect);

  let promotion = get(&run, "promotion-held");
  assert!(as_bool(get(promotion, "is_held")));
  assert_eq!(
    as_str(get(promotion, "outcome")),
    "held-coding-expression-adaptation-resilience-promotion-blocked"
  );
  assert_effects_locked(promotion);
}

#[test]
fn dispatch_and_mirror_connect_plan_to_adaptive_evidence_acquisition() {
  let run = eval_file(&fixture_path()).unwrap();

  let dispatched = get(&run, "dispatched");
  assert_eq!(
    as_str(get(dispatched, "op")),
    "plan-coding-expression-adaptation-resilience"
  );
  let result = get(dispatched, "result");
  assert_eq!(
    as_str(get(result, "outcome")),
    "coding-expression-adaptation-resilience-plan-built"
  );

  let observed = get(&run, "observed-plan");
  assert_eq!(
    as_str(get(observed, "outcome")),
    "mirror-self-observation-plan-built"
  );
  assert_eq!(
    as_str(get(observed, "next_action")),
    "build-adaptive-evidence-acquisition-plan"
  );
  let ko_self_description = get(observed, "ko_self_description");
  assert!(as_str(get(ko_self_description, "text")).contains("code quality"));
}
