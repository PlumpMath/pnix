use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/adaptive-evidence-acquisition-plan.px")
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
  let run = eval_file(&fixture_path()).expect("adaptive evidence fixture evaluates");
  assert_eq!(
    as_str(get(&run, "proof")),
    "adaptive-evidence-acquisition-plan"
  );

  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.agent.adaptive-evidence-acquisition-plan"
  );
  assert_eq!(
    as_str(get(meta, "base")),
    "adaptive-evidence-acquisition-plan-v0"
  );
}

#[test]
fn plan_builds_candidate_docs_compiler_lsp_test_and_negative_receipt_lanes() {
  let run = eval_file(&fixture_path()).unwrap();
  let plan = get(&run, "plan");

  assert_eq!(
    as_str(get(plan, "schema")),
    "puncheetah.adaptive-evidence-acquisition-plan.v0"
  );
  assert_eq!(
    as_str(get(plan, "outcome")),
    "adaptive-evidence-acquisition-plan-built"
  );
  assert!(as_bool(get(plan, "verified")));
  assert!(as_bool(get(
    plan,
    "adaptive_evidence_acquisition_plan_built"
  )));
  assert!(as_bool(get(plan, "source_coding_expression_plan_verified")));
  assert!(as_bool(get(plan, "docs_evidence_plans_built")));
  assert!(as_bool(get(plan, "compiler_feedback_plan_built")));
  assert!(as_bool(get(plan, "lsp_feedback_plan_built")));
  assert!(as_bool(get(plan, "minimal_test_probe_plan_built")));
  assert!(as_bool(get(plan, "negative_receipt_plan_built")));
  assert!(as_bool(get(plan, "evidence_candidate_only")));
  assert!(!as_bool(get(plan, "accepted_fact_allowed")));
  assert_eq!(
    as_str(get(plan, "next_gate")),
    "adaptive-evidence-acquisition-result"
  );
  assert_effects_locked(plan);

  assert_eq!(as_str(get(plan, "language")), "rust");
  assert!(list_contains_str(
    get(plan, "missing_semantic_slots"),
    "api-affordance"
  ));
  assert!(list_contains_str(
    get(plan, "code_quality_obligations"),
    "idiomatic-language-use"
  ));

  let docs = as_list(get(plan, "docs_evidence_plans"));
  assert_eq!(docs.len(), 2);
  assert_eq!(as_str(get(&docs[0], "api_ref")), "client.create_request");
  assert!(as_bool(get(&docs[0], "candidate_only")));
  assert!(as_bool(get(&docs[0], "verification_required")));
  assert!(!as_bool(get(&docs[0], "accepted_fact")));
  assert!(!as_bool(get(&docs[0], "search_execution_allowed")));

  let compiler = as_list(get(plan, "compiler_feedback_plans"));
  assert_eq!(compiler.len(), 1);
  assert!(as_bool(get(&compiler[0], "host_bridge_receipt_required")));
  assert!(!as_bool(get(&compiler[0], "compiler_execution_allowed")));

  let lsp = as_list(get(plan, "lsp_feedback_plans"));
  assert_eq!(lsp.len(), 1);
  assert!(as_bool(get(&lsp[0], "host_bridge_receipt_required")));
  assert!(!as_bool(get(&lsp[0], "lsp_execution_allowed")));

  let tests = as_list(get(plan, "minimal_test_probe_plans"));
  assert_eq!(tests.len(), 1);
  assert!(as_bool(get(&tests[0], "test_plan_receipt_required")));
  assert!(!as_bool(get(&tests[0], "test_execution_allowed")));

  let negative = as_list(get(plan, "negative_receipt_plans"));
  assert_eq!(negative.len(), 4);
  assert_eq!(as_str(get(&negative[0], "subject")), "api-affordance");

  let steps = as_list(get(plan, "evidence_acquisition_steps"));
  assert_eq!(steps.len(), 5);
  assert_eq!(
    as_str(get(&steps[0], "outcome")),
    "coding-expression-plan-verified"
  );
}

#[test]
fn no_api_plan_still_builds_compiler_lsp_test_and_quality_evidence_without_docs() {
  let run = eval_file(&fixture_path()).unwrap();
  let plan = get(&run, "no-api-plan");

  assert_eq!(
    as_str(get(plan, "outcome")),
    "adaptive-evidence-acquisition-plan-built"
  );
  assert!(!as_bool(get(plan, "docs_evidence_plans_built")));
  assert!(as_bool(get(plan, "compiler_feedback_plan_built")));
  assert!(as_bool(get(plan, "lsp_feedback_plan_built")));
  assert!(as_bool(get(plan, "minimal_test_probe_plan_built")));
  assert!(!as_bool(get(plan, "negative_receipt_plan_built")));
  assert_eq!(as_list(get(plan, "docs_evidence_plans")).len(), 0);
  assert_eq!(as_list(get(plan, "negative_receipt_plans")).len(), 0);
  assert!(as_list(get(plan, "code_quality_evidence_plans")).len() >= 8);
  assert_effects_locked(plan);
}

#[test]
fn missing_unverified_effect_and_promotion_requests_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let missing = get(&run, "missing-source-held");
  assert!(as_bool(get(missing, "is_held")));
  assert_eq!(
    as_str(get(missing, "outcome")),
    "held-adaptive-evidence-acquisition-plan-source-required"
  );

  let unverified = get(&run, "unverified-source-held");
  assert!(as_bool(get(unverified, "is_held")));
  assert_eq!(
    as_str(get(unverified, "outcome")),
    "held-adaptive-evidence-acquisition-plan-source-unverified"
  );

  let effect = get(&run, "effect-held");
  assert!(as_bool(get(effect, "is_held")));
  assert_eq!(
    as_str(get(effect, "outcome")),
    "held-adaptive-evidence-acquisition-plan-effect-blocked"
  );
  assert_effects_locked(effect);

  let promotion = get(&run, "promotion-held");
  assert!(as_bool(get(promotion, "is_held")));
  assert_eq!(
    as_str(get(promotion, "outcome")),
    "held-adaptive-evidence-acquisition-plan-promotion-blocked"
  );
  assert_effects_locked(promotion);
}

#[test]
fn dispatch_and_mirror_connect_plan_to_evidence_result_verification() {
  let run = eval_file(&fixture_path()).unwrap();

  let dispatched = get(&run, "dispatched");
  assert_eq!(
    as_str(get(dispatched, "op")),
    "build-adaptive-evidence-acquisition-plan"
  );
  let result = get(dispatched, "result");
  assert_eq!(
    as_str(get(result, "outcome")),
    "adaptive-evidence-acquisition-plan-built"
  );

  let observed = get(&run, "observed-plan");
  assert_eq!(
    as_str(get(observed, "outcome")),
    "mirror-self-observation-plan-built"
  );
  assert_eq!(
    as_str(get(observed, "next_action")),
    "verify-adaptive-evidence-acquisition-result"
  );
  let ko_self_description = get(observed, "ko_self_description");
  assert!(as_str(get(ko_self_description, "text")).contains("evidence acquisition"));
}
