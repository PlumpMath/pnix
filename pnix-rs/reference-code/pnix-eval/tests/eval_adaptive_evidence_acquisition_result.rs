use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/adaptive-evidence-acquisition-result.px")
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
  let run = eval_file(&fixture_path()).expect("adaptive evidence result fixture evaluates");
  assert_eq!(
    as_str(get(&run, "proof")),
    "adaptive-evidence-acquisition-result"
  );

  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.agent.adaptive-evidence-acquisition-result"
  );
  assert_eq!(
    as_str(get(meta, "base")),
    "adaptive-evidence-acquisition-result-v0"
  );
}

#[test]
fn result_verifies_all_planned_candidate_evidence_lanes_without_promotion() {
  let run = eval_file(&fixture_path()).unwrap();
  let result = get(&run, "result");

  assert_eq!(
    as_str(get(result, "schema")),
    "puncheetah.adaptive-evidence-acquisition-result.v0"
  );
  assert_eq!(
    as_str(get(result, "outcome")),
    "adaptive-evidence-acquisition-result-verified"
  );
  assert!(as_bool(get(result, "verified")));
  assert!(as_bool(get(
    result,
    "adaptive_evidence_acquisition_result_verified"
  )));
  assert!(as_bool(get(
    result,
    "source_adaptive_evidence_acquisition_plan_verified"
  )));
  assert!(as_bool(get(
    result,
    "all_required_evidence_lanes_satisfied"
  )));
  assert!(as_bool(get(result, "docs_evidence_results_verified")));
  assert!(as_bool(get(result, "compiler_feedback_result_verified")));
  assert!(as_bool(get(result, "lsp_feedback_result_verified")));
  assert!(as_bool(get(result, "minimal_test_probe_result_verified")));
  assert!(as_bool(get(result, "negative_receipt_result_verified")));
  assert!(as_bool(get(result, "candidate_evidence_preserved")));
  assert!(!as_bool(get(result, "accepted_fact_allowed")));
  assert!(!as_bool(get(result, "accepted_fact_promotion_allowed")));
  assert_eq!(
    as_str(get(result, "next_gate")),
    "adaptive-evidence-synthesis-or-coding-plan-reopen"
  );
  assert_effects_locked(result);

  let docs = as_list(get(result, "docs_evidence_results"));
  assert_eq!(docs.len(), 2);
  assert_eq!(as_str(get(&docs[0], "api_ref")), "client.create_request");
  assert!(as_bool(get(&docs[0], "candidate_only")));
  assert!(!as_bool(get(&docs[0], "accepted_fact")));

  let compiler = as_list(get(result, "compiler_feedback_results"));
  assert_eq!(compiler.len(), 1);
  assert!(as_bool(get(&compiler[0], "host_bridge_receipt_verified")));
  assert_eq!(as_str(get(&compiler[0], "language")), "rust");

  let statuses = as_list(get(result, "evidence_lane_statuses"));
  assert_eq!(statuses.len(), 5);
  assert_eq!(as_str(get(&statuses[0], "lane")), "docs-or-type-signature");
  assert!(as_bool(get(&statuses[0], "candidate_only")));

  let steps = as_list(get(result, "evidence_result_steps"));
  assert_eq!(steps.len(), 5);
  assert_eq!(
    as_str(get(&steps[0], "outcome")),
    "adaptive-evidence-plan-verified"
  );
}

#[test]
fn no_api_result_can_verify_without_docs_or_negative_receipts() {
  let run = eval_file(&fixture_path()).unwrap();
  let result = get(&run, "no-api-result");

  assert_eq!(
    as_str(get(result, "outcome")),
    "adaptive-evidence-acquisition-result-verified"
  );
  assert!(as_bool(get(
    result,
    "all_required_evidence_lanes_satisfied"
  )));
  assert!(!as_bool(get(result, "docs_evidence_results_verified")));
  assert!(!as_bool(get(result, "negative_receipt_result_verified")));
  assert_eq!(as_list(get(result, "docs_evidence_results")).len(), 0);
  assert_eq!(as_list(get(result, "negative_receipts")).len(), 0);
  assert_effects_locked(result);
}

#[test]
fn missing_mismatch_contamination_and_effect_requests_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let missing_plan = get(&run, "missing-plan-held");
  assert!(as_bool(get(missing_plan, "is_held")));
  assert_eq!(
    as_str(get(missing_plan, "outcome")),
    "held-adaptive-evidence-acquisition-result-plan-required"
  );

  let unverified_plan = get(&run, "unverified-plan-held");
  assert!(as_bool(get(unverified_plan, "is_held")));
  assert_eq!(
    as_str(get(unverified_plan, "outcome")),
    "held-adaptive-evidence-acquisition-result-plan-unverified"
  );

  let docs_missing = get(&run, "docs-missing-held");
  assert!(as_bool(get(docs_missing, "is_held")));
  assert_eq!(
    as_str(get(docs_missing, "outcome")),
    "held-adaptive-evidence-acquisition-result-docs-missing"
  );

  let docs_mismatch = get(&run, "docs-mismatch-held");
  assert!(as_bool(get(docs_mismatch, "is_held")));
  assert_eq!(
    as_str(get(docs_mismatch, "outcome")),
    "held-adaptive-evidence-acquisition-result-docs-plan-mismatch"
  );

  let promoted = get(&run, "candidate-promoted-held");
  assert!(as_bool(get(promoted, "is_held")));
  assert_eq!(
    as_str(get(promoted, "outcome")),
    "held-adaptive-evidence-acquisition-result-docs-invalid"
  );

  let compiler_missing = get(&run, "compiler-missing-held");
  assert!(as_bool(get(compiler_missing, "is_held")));
  assert_eq!(
    as_str(get(compiler_missing, "outcome")),
    "held-adaptive-evidence-acquisition-result-compiler-missing"
  );

  let negative_missing = get(&run, "negative-missing-held");
  assert!(as_bool(get(negative_missing, "is_held")));
  assert_eq!(
    as_str(get(negative_missing, "outcome")),
    "held-adaptive-evidence-acquisition-result-negative-receipt-missing"
  );

  let effect = get(&run, "effect-held");
  assert!(as_bool(get(effect, "is_held")));
  assert_eq!(
    as_str(get(effect, "outcome")),
    "held-adaptive-evidence-acquisition-result-effect-blocked"
  );
  assert_effects_locked(effect);

  let promotion = get(&run, "promotion-held");
  assert!(as_bool(get(promotion, "is_held")));
  assert_eq!(
    as_str(get(promotion, "outcome")),
    "held-adaptive-evidence-acquisition-result-promotion-blocked"
  );
  assert_effects_locked(promotion);
}

#[test]
fn dispatch_and_mirror_connect_result_to_synthesis_or_plan_reopen() {
  let run = eval_file(&fixture_path()).unwrap();

  let dispatched = get(&run, "dispatched");
  assert_eq!(
    as_str(get(dispatched, "op")),
    "verify-adaptive-evidence-acquisition-result"
  );
  let result = get(dispatched, "result");
  assert_eq!(
    as_str(get(result, "outcome")),
    "adaptive-evidence-acquisition-result-verified"
  );

  let observed = get(&run, "observed-result");
  assert_eq!(
    as_str(get(observed, "outcome")),
    "mirror-self-observation-plan-built"
  );
  assert_eq!(
    as_str(get(observed, "next_action")),
    "build-adaptive-evidence-synthesis-or-coding-plan-reopen"
  );
  let ko_self_description = get(observed, "ko_self_description");
  assert!(as_str(get(ko_self_description, "text")).contains("evidence acquisition result"));
}
