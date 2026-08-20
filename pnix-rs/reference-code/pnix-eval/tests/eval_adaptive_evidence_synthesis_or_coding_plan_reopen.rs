use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/adaptive-evidence-synthesis-or-coding-plan-reopen.px")
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
  let run = eval_file(&fixture_path()).expect("adaptive evidence synthesis fixture evaluates");
  assert_eq!(
    as_str(get(&run, "proof")),
    "adaptive-evidence-synthesis-or-coding-plan-reopen"
  );

  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.agent.adaptive-evidence-synthesis-or-coding-plan-reopen"
  );
  assert_eq!(
    as_str(get(meta, "base")),
    "adaptive-evidence-synthesis-or-coding-plan-reopen-v0"
  );
}

#[test]
fn verified_candidate_evidence_reopens_coding_plan_without_promotion() {
  let run = eval_file(&fixture_path()).unwrap();
  let synthesis = get(&run, "synthesis");

  assert_eq!(
    as_str(get(synthesis, "schema")),
    "puncheetah.adaptive-evidence-synthesis-or-coding-plan-reopen.v0"
  );
  assert_eq!(
    as_str(get(synthesis, "outcome")),
    "adaptive-evidence-synthesis-or-coding-plan-reopen-built"
  );
  assert!(as_bool(get(synthesis, "verified")));
  assert!(as_bool(get(synthesis, "adaptive_evidence_synthesis_built")));
  assert!(as_bool(get(synthesis, "source_evidence_result_verified")));
  assert!(as_bool(get(synthesis, "candidate_evidence_summarized")));
  assert!(as_bool(get(synthesis, "candidate_evidence_preserved")));
  assert!(as_bool(get(synthesis, "coding_plan_reopen_allowed")));
  assert!(!as_bool(get(synthesis, "additional_evidence_required")));
  assert!(as_bool(get(synthesis, "promotion_candidate_gate_required")));
  assert!(as_bool(get(synthesis, "promotion_candidate_built")));
  assert!(as_bool(get(synthesis, "route_avoidance_candidates_built")));
  assert_eq!(as_str(get(synthesis, "decision")), "coding-plan-reopen");
  assert_eq!(
    as_str(get(synthesis, "next_gate")),
    "coding-expression-plan-reopen"
  );
  assert!(!as_bool(get(synthesis, "accepted_fact_allowed")));
  assert!(!as_bool(get(synthesis, "accepted_fact_promotion_allowed")));
  assert_effects_locked(synthesis);

  let summary = get(synthesis, "candidate_evidence_summary");
  assert_eq!(as_i64(get(summary, "docs_result_count")), 2);
  assert_eq!(as_i64(get(summary, "compiler_result_count")), 1);
  assert_eq!(as_i64(get(summary, "lsp_result_count")), 1);
  assert_eq!(as_i64(get(summary, "minimal_test_probe_result_count")), 1);
  assert_eq!(as_i64(get(summary, "negative_receipt_count")), 4);
  assert!(!as_bool(get(summary, "compiler_need_more")));
  assert!(!as_bool(get(summary, "test_need_more")));
  assert!(!as_bool(get(summary, "accepted_fact")));

  let reopen = get(synthesis, "coding_plan_reopen");
  assert_eq!(as_str(get(reopen, "status")), "reopen-ready");
  assert!(!as_bool(get(reopen, "code_write_allowed")));

  let promotion = get(synthesis, "promotion_candidate");
  assert_eq!(
    as_str(get(promotion, "status")),
    "requires-separate-promotion-gate"
  );
  assert!(!as_bool(get(promotion, "accepted_fact_promotion_allowed")));

  let steps = as_list(get(synthesis, "synthesis_steps"));
  assert_eq!(steps.len(), 4);
  assert_eq!(
    as_str(get(&steps[0], "outcome")),
    "adaptive-evidence-result-verified"
  );
}

#[test]
fn no_api_synthesis_reopens_plan_without_promotion_candidate_gate() {
  let run = eval_file(&fixture_path()).unwrap();
  let synthesis = get(&run, "no-api-synthesis");

  assert_eq!(
    as_str(get(synthesis, "outcome")),
    "adaptive-evidence-synthesis-or-coding-plan-reopen-built"
  );
  assert!(as_bool(get(synthesis, "coding_plan_reopen_allowed")));
  assert!(!as_bool(get(synthesis, "additional_evidence_required")));
  assert!(!as_bool(get(
    synthesis,
    "promotion_candidate_gate_required"
  )));
  assert!(!as_bool(get(synthesis, "route_avoidance_candidates_built")));
  assert_eq!(
    as_str(get(synthesis, "next_gate")),
    "coding-expression-plan-reopen"
  );
  assert_effects_locked(synthesis);

  let summary = get(synthesis, "candidate_evidence_summary");
  assert_eq!(as_i64(get(summary, "docs_result_count")), 0);
  assert_eq!(as_i64(get(summary, "negative_receipt_count")), 0);
}

#[test]
fn blocking_evidence_requests_additional_evidence_instead_of_reopening_plan() {
  let run = eval_file(&fixture_path()).unwrap();
  let synthesis = get(&run, "additional-evidence");

  assert_eq!(
    as_str(get(synthesis, "outcome")),
    "adaptive-evidence-synthesis-or-coding-plan-reopen-built"
  );
  assert_eq!(
    as_str(get(synthesis, "decision")),
    "additional-evidence-required"
  );
  assert!(!as_bool(get(synthesis, "coding_plan_reopen_allowed")));
  assert!(as_bool(get(synthesis, "additional_evidence_required")));
  assert!(!as_bool(get(
    synthesis,
    "promotion_candidate_gate_required"
  )));
  assert_eq!(
    as_str(get(synthesis, "next_gate")),
    "adaptive-evidence-acquisition-plan"
  );
  assert_effects_locked(synthesis);

  let summary = get(synthesis, "candidate_evidence_summary");
  assert!(as_bool(get(summary, "compiler_need_more")));
  assert!(as_bool(get(summary, "test_need_more")));

  let hint = get(synthesis, "additional_evidence_plan_hint");
  assert!(as_bool(get(hint, "required")));
  assert!(as_bool(get(hint, "compiler_need_more")));
  assert!(as_bool(get(hint, "test_need_more")));
}

#[test]
fn missing_unverified_candidate_loss_effect_and_promotion_requests_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let missing = get(&run, "missing-result-held");
  assert!(as_bool(get(missing, "is_held")));
  assert_eq!(
    as_str(get(missing, "outcome")),
    "held-adaptive-evidence-synthesis-result-required"
  );

  let unverified = get(&run, "unverified-result-held");
  assert!(as_bool(get(unverified, "is_held")));
  assert_eq!(
    as_str(get(unverified, "outcome")),
    "held-adaptive-evidence-synthesis-result-unverified"
  );

  let candidate = get(&run, "candidate-not-preserved-held");
  assert!(as_bool(get(candidate, "is_held")));
  assert_eq!(
    as_str(get(candidate, "outcome")),
    "held-adaptive-evidence-synthesis-result-unverified"
  );

  let effect = get(&run, "effect-held");
  assert!(as_bool(get(effect, "is_held")));
  assert_eq!(
    as_str(get(effect, "outcome")),
    "held-adaptive-evidence-synthesis-effect-blocked"
  );
  assert_effects_locked(effect);

  let promotion = get(&run, "promotion-held");
  assert!(as_bool(get(promotion, "is_held")));
  assert_eq!(
    as_str(get(promotion, "outcome")),
    "held-adaptive-evidence-synthesis-promotion-blocked"
  );
  assert_effects_locked(promotion);
}

#[test]
fn dispatch_and_mirror_connect_synthesis_to_plan_reopen() {
  let run = eval_file(&fixture_path()).unwrap();

  let dispatched = get(&run, "dispatched");
  assert_eq!(
    as_str(get(dispatched, "op")),
    "build-adaptive-evidence-synthesis-or-coding-plan-reopen"
  );
  let result = get(dispatched, "result");
  assert_eq!(
    as_str(get(result, "outcome")),
    "adaptive-evidence-synthesis-or-coding-plan-reopen-built"
  );
  assert_eq!(as_str(get(result, "decision")), "coding-plan-reopen");

  let observed = get(&run, "observed-synthesis");
  assert_eq!(
    as_str(get(observed, "outcome")),
    "mirror-self-observation-plan-built"
  );
  assert_eq!(
    as_str(get(observed, "next_action")),
    "reopen-coding-expression-plan"
  );
  let ko_self_description = get(observed, "ko_self_description");
  assert!(as_str(get(ko_self_description, "text")).contains("adaptive evidence synthesis"));
}
