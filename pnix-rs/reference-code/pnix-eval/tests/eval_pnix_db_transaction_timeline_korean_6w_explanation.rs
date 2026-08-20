use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/pnix-db-transaction-timeline-korean-6w-explanation.px")
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
  assert!(!as_bool(get(v, "memory_write_allowed")));
  assert!(!as_bool(get(v, "policy_persistence_allowed")));
}

#[test]
fn fixture_evaluates_with_pnix_eval_not_nix() {
  let run = eval_file(&fixture_path()).expect("Korean 6W timeline explanation fixture evaluates");
  assert_eq!(
    as_str(get(&run, "proof")),
    "pnix-db-transaction-timeline-korean-6w-explanation"
  );

  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.agent.pnix-db-transaction-timeline-korean-6w-explanation"
  );
  assert_eq!(
    as_str(get(meta, "base")),
    "pnix-db-transaction-timeline-korean-6w-explanation-v0"
  );
}

#[test]
fn complete_timeline_audit_becomes_korean_6w_status_answer() {
  let run = eval_file(&fixture_path()).unwrap();
  let explained = get(&run, "complete-status");

  assert_eq!(
    as_str(get(explained, "schema")),
    "puncheetah.pnix-db.transaction-timeline-korean-6w-explanation.v0"
  );
  assert_eq!(
    as_str(get(explained, "outcome")),
    "pnix-db-transaction-timeline-korean-audit-built"
  );
  assert!(as_bool(get(explained, "verified")));
  assert!(as_bool(get(explained, "timeline_query_answered")));
  assert!(as_bool(get(explained, "six_w_explanation_built")));
  assert_eq!(as_str(get(explained, "surface_language")), "korean");
  assert_eq!(as_str(get(explained, "query_kind")), "status");
  assert_eq!(as_str(get(explained, "terminal_status")), "complete");
  assert_eq!(
    as_str(get(explained, "terminal_receipt_kind")),
    "transaction-complete"
  );
  assert!(as_bool(get(explained, "evidence_chain_summarized")));
  assert_effects_locked(explained);

  let summary = get(explained, "ko_audit_summary");
  assert!(as_str(get(summary, "who")).contains("host bridge evidence"));
  assert!(as_str(get(summary, "what")).contains("테스트 통과"));
  assert!(as_str(get(summary, "why")).contains("complete-ready policy"));
  assert!(as_str(get(summary, "how")).contains("timeline seq"));
  assert!(as_str(get(summary, "result")).contains("complete"));
  assert!(as_str(get(explained, "focused_answer_ko")).contains("complete"));

  let atoms = as_list(get(explained, "canonical_audit_atoms"));
  assert_eq!(atoms.len(), 7);
  assert_eq!(as_str(get(&atoms[0], "slot")), "who");
  assert_eq!(as_str(get(&atoms[4], "slot")), "why");

  let steps = as_list(get(explained, "timeline_steps"));
  assert_eq!(steps.len(), 6);
  assert_eq!(
    as_str(get(&steps[5], "outcome")),
    "coding-project-transaction-complete-receipt-built"
  );
}

#[test]
fn rollback_timeline_audit_explains_failure_and_recovery_path() {
  let run = eval_file(&fixture_path()).unwrap();
  let why = get(&run, "rollback-why");
  let failed = get(&run, "rollback-failed");
  let evidence = get(&run, "rollback-evidence");

  assert_eq!(
    as_str(get(why, "outcome")),
    "pnix-db-transaction-timeline-korean-audit-built"
  );
  assert_eq!(as_str(get(why, "query_kind")), "why-rollback");
  assert_eq!(as_str(get(why, "terminal_status")), "rollback-complete");
  assert_eq!(
    as_str(get(why, "terminal_receipt_kind")),
    "rollback-complete"
  );
  assert!(as_str(get(why, "focused_answer_ko")).contains("테스트"));
  assert!(as_str(get(why, "focused_answer_ko")).contains("rollback"));
  assert_effects_locked(why);

  assert_eq!(as_str(get(failed, "query_kind")), "what-failed");
  assert!(as_str(get(failed, "focused_answer_ko")).contains("테스트 실패"));
  assert!(as_str(get(failed, "focused_answer_ko")).contains("rollback-complete"));

  assert_eq!(as_str(get(evidence, "query_kind")), "evidence-chain");
  assert!(as_str(get(evidence, "focused_answer_ko")).contains("rollback 실행 결과"));
  let steps = as_list(get(evidence, "timeline_steps"));
  assert_eq!(steps.len(), 9);
  assert_eq!(
    as_str(get(&steps[8], "outcome")),
    "coding-project-rollback-complete-receipt-built"
  );
}

#[test]
fn complete_branch_answers_why_rollback_negatively() {
  let run = eval_file(&fixture_path()).unwrap();
  let answer = get(&run, "complete-why-rollback");

  assert_eq!(
    as_str(get(answer, "outcome")),
    "pnix-db-transaction-timeline-korean-audit-built"
  );
  assert_eq!(as_str(get(answer, "query_kind")), "why-rollback");
  assert_eq!(as_str(get(answer, "terminal_status")), "complete");
  assert!(as_str(get(answer, "focused_answer_ko")).contains("rollback되지 않았다"));
  assert!(as_str(get(answer, "focused_answer_ko")).contains("테스트가 통과"));
}

#[test]
fn reasoning_dispatch_can_build_korean_6w_timeline_explanation() {
  let run = eval_file(&fixture_path()).unwrap();
  let dispatched = get(&run, "dispatched");
  assert_eq!(
    as_str(get(dispatched, "op")),
    "explain-pnix-db-transaction-timeline-korean-6w"
  );
  let result = get(dispatched, "result");
  assert_eq!(
    as_str(get(result, "outcome")),
    "pnix-db-transaction-timeline-korean-audit-built"
  );
  assert_eq!(as_str(get(result, "query_kind")), "gate-order");
  assert!(as_bool(get(result, "six_w_explanation_built")));
  assert!(as_str(get(result, "focused_answer_ko")).contains("event seq"));
  assert!(!as_bool(get(result, "memory_write_allowed")));
}

#[test]
fn missing_invalid_unsupported_and_effect_requests_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let missing = get(&run, "missing-audit");
  assert!(as_bool(get(missing, "is_held")));
  assert_eq!(
    as_str(get(missing, "outcome")),
    "held-pnix-db-transaction-timeline-korean-6w-audit-required"
  );

  let invalid = get(&run, "invalid-audit");
  assert!(as_bool(get(invalid, "is_held")));
  assert_eq!(
    as_str(get(invalid, "outcome")),
    "held-pnix-db-transaction-timeline-korean-6w-invalid-audit"
  );

  let missing_timeline = get(&run, "missing-timeline");
  assert!(as_bool(get(missing_timeline, "is_held")));
  assert_eq!(
    as_str(get(missing_timeline, "outcome")),
    "held-pnix-db-transaction-timeline-korean-6w-timeline-required"
  );

  let unsupported = get(&run, "unsupported-query");
  assert!(as_bool(get(unsupported, "is_held")));
  assert_eq!(
    as_str(get(unsupported, "outcome")),
    "held-pnix-db-transaction-timeline-korean-6w-unsupported-query"
  );

  let effect = get(&run, "effect-held");
  assert!(as_bool(get(effect, "is_held")));
  assert_eq!(
    as_str(get(effect, "outcome")),
    "held-pnix-db-transaction-timeline-korean-6w-effect-blocked"
  );
  assert_effects_locked(effect);
}
