use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/mirror-self-observation-receipt.px")
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

#[test]
fn fixture_evaluates_with_pnix_eval_not_nix() {
  let run = eval_file(&fixture_path()).expect("mirror self-observation fixture must evaluate");
  assert_eq!(as_str(get(&run, "proof")), "mirror-self-observation");

  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.agent.mirror-self-observation"
  );
  assert_eq!(as_str(get(meta, "base")), "mirror-self-observation-v0");
}

#[test]
fn host_apply_plan_receipt_becomes_korean_self_plan_without_effects() {
  let run = eval_file(&fixture_path()).unwrap();
  let observed = get(&run, "observed");

  assert_eq!(
    as_str(get(observed, "schema")),
    "puncheetah.mirror.self-observation.v0"
  );
  assert_eq!(
    as_str(get(observed, "outcome")),
    "mirror-self-observation-plan-built"
  );
  assert!(as_bool(get(observed, "verified")));
  assert!(as_bool(get(observed, "mirror_self_observation_built")));
  assert!(as_bool(get(observed, "mirror_plan_built")));
  assert!(as_bool(get(observed, "mirror_plan_allowed")));
  assert!(!as_bool(get(observed, "host_apply_allowed")));
  assert!(!as_bool(get(observed, "file_write_allowed")));
  assert!(!as_bool(get(observed, "test_execution_allowed")));

  assert_eq!(
    as_str(get(observed, "next_gate")),
    "coding-project-working-tree-safety-check"
  );
  assert_eq!(
    as_str(get(observed, "next_action")),
    "build-coding-project-working-tree-safety-check"
  );

  let ko = get(observed, "ko_self_description");
  assert_eq!(as_str(get(ko, "kind")), "mirror-ko-self-description-v0");
  let sentences = as_list(get(ko, "sentences"));
  assert_eq!(sentences.len(), 3);
  assert!(as_str(get(ko, "text")).contains("호스트 적용 계획"));
  assert!(as_str(get(ko, "text")).contains("작업트리 안전성"));

  let atoms = as_list(get(observed, "canonical_self_meaning_atoms"));
  assert_eq!(atoms.len(), 4);
  assert_eq!(as_str(get(&atoms[0], "kind")), "self-observation");
  assert_eq!(as_str(get(&atoms[2], "kind")), "prohibition");
  assert_eq!(as_str(get(&atoms[3], "kind")), "next-plan");

  let plan = get(observed, "plan_receipt");
  assert_eq!(
    as_str(get(plan, "schema")),
    "puncheetah.mirror.plan-receipt.v0"
  );
  assert_eq!(as_str(get(plan, "outcome")), "mirror-plan-receipt-built");
  assert_eq!(as_str(get(plan, "plan_kind")), "next-gate-plan");
  assert!(!as_bool(get(plan, "host_apply_allowed")));
  assert!(!as_bool(get(plan, "file_write_allowed")));

  let dispatched = get(&run, "dispatched");
  assert_eq!(as_str(get(dispatched, "op")), "mirror-self-observe");
  let dispatch_result = get(dispatched, "result");
  assert_eq!(
    as_str(get(dispatch_result, "outcome")),
    "mirror-self-observation-plan-built"
  );
  assert_eq!(
    as_str(get(dispatch_result, "next_action")),
    "build-coding-project-working-tree-safety-check"
  );
}

#[test]
fn patch_preview_review_to_applyable_ir_spine_receipts_plan_next_gates() {
  let run = eval_file(&fixture_path()).unwrap();

  let observed_review = get(&run, "observed-patch-preview-reviewed");
  assert_eq!(
    as_str(get(observed_review, "outcome")),
    "mirror-self-observation-plan-built"
  );
  assert!(as_bool(get(observed_review, "verified")));
  assert_eq!(
    as_str(get(observed_review, "next_gate")),
    "coding-project-apply-approval-gate"
  );
  assert_eq!(
    as_str(get(observed_review, "next_action")),
    "request-coding-project-apply-approval"
  );
  assert!(!as_bool(get(observed_review, "host_apply_allowed")));
  assert!(!as_bool(get(observed_review, "file_write_allowed")));

  let observed_approval = get(&run, "observed-apply-approval-gate");
  assert_eq!(
    as_str(get(observed_approval, "outcome")),
    "mirror-self-observation-plan-built"
  );
  assert!(as_bool(get(observed_approval, "verified")));
  assert_eq!(
    as_str(get(observed_approval, "next_gate")),
    "coding-project-applyable-ir"
  );
  assert_eq!(
    as_str(get(observed_approval, "next_action")),
    "build-coding-project-applyable-ir"
  );
  assert!(!as_bool(get(observed_approval, "host_apply_allowed")));
  assert!(!as_bool(get(observed_approval, "file_write_allowed")));

  let observed_applyable = get(&run, "observed-applyable-ir");
  assert_eq!(
    as_str(get(observed_applyable, "outcome")),
    "mirror-self-observation-plan-built"
  );
  assert!(as_bool(get(observed_applyable, "verified")));
  assert_eq!(
    as_str(get(observed_applyable, "next_gate")),
    "coding-project-source-anchor-check"
  );
  assert_eq!(
    as_str(get(observed_applyable, "next_action")),
    "build-coding-project-source-anchor-check"
  );
  assert!(!as_bool(get(observed_applyable, "host_apply_allowed")));
  assert!(!as_bool(get(observed_applyable, "file_write_allowed")));

  let observed_source_anchor = get(&run, "observed-source-anchor-check");
  assert_eq!(
    as_str(get(observed_source_anchor, "outcome")),
    "mirror-self-observation-plan-built"
  );
  assert!(as_bool(get(observed_source_anchor, "verified")));
  assert_eq!(
    as_str(get(observed_source_anchor, "next_gate")),
    "coding-project-apply-dry-run"
  );
  assert_eq!(
    as_str(get(observed_source_anchor, "next_action")),
    "build-coding-project-apply-dry-run"
  );
  assert!(!as_bool(get(observed_source_anchor, "host_apply_allowed")));
  assert!(!as_bool(get(observed_source_anchor, "file_write_allowed")));

  let observed_apply_dry_run = get(&run, "observed-apply-dry-run");
  assert_eq!(
    as_str(get(observed_apply_dry_run, "outcome")),
    "mirror-self-observation-plan-built"
  );
  assert!(as_bool(get(observed_apply_dry_run, "verified")));
  assert_eq!(
    as_str(get(observed_apply_dry_run, "next_gate")),
    "coding-project-test-plan-receipt"
  );
  assert_eq!(
    as_str(get(observed_apply_dry_run, "next_action")),
    "build-coding-project-test-plan-receipt"
  );
  assert!(!as_bool(get(observed_apply_dry_run, "host_apply_allowed")));
  assert!(!as_bool(get(observed_apply_dry_run, "file_write_allowed")));

  let observed_test_plan = get(&run, "observed-test-plan-receipt");
  assert_eq!(
    as_str(get(observed_test_plan, "outcome")),
    "mirror-self-observation-plan-built"
  );
  assert!(as_bool(get(observed_test_plan, "verified")));
  assert_eq!(
    as_str(get(observed_test_plan, "next_gate")),
    "coding-project-final-apply-approval-or-host-plan"
  );
  assert_eq!(
    as_str(get(observed_test_plan, "next_action")),
    "build-coding-project-final-apply-approval-or-host-plan"
  );
  assert!(!as_bool(get(observed_test_plan, "host_apply_allowed")));
  assert!(!as_bool(get(observed_test_plan, "file_write_allowed")));

  let observed_final_apply_policy = get(&run, "observed-final-apply-policy-receipt");
  assert_eq!(
    as_str(get(observed_final_apply_policy, "outcome")),
    "mirror-self-observation-plan-built"
  );
  assert!(as_bool(get(observed_final_apply_policy, "verified")));
  assert_eq!(
    as_str(get(observed_final_apply_policy, "next_gate")),
    "coding-project-final-apply-approval-gate"
  );
  assert_eq!(
    as_str(get(observed_final_apply_policy, "next_action")),
    "request-coding-project-final-apply-approval"
  );
  assert!(!as_bool(get(
    observed_final_apply_policy,
    "host_apply_allowed"
  )));
  assert!(!as_bool(get(
    observed_final_apply_policy,
    "file_write_allowed"
  )));

  let ko = get(observed_final_apply_policy, "ko_self_description");
  assert!(as_str(get(ko, "text")).contains("최종 적용 승인 정책"));
  assert!(as_str(get(ko, "text")).contains("최종 적용 승인"));
}

#[test]
fn host_transaction_dry_run_receipt_plans_final_file_write_approval() {
  let run = eval_file(&fixture_path()).unwrap();
  let observed = get(&run, "observed-dry-run");

  assert_eq!(
    as_str(get(observed, "outcome")),
    "mirror-self-observation-plan-built"
  );
  assert!(as_bool(get(observed, "verified")));
  assert_eq!(
    as_str(get(observed, "next_gate")),
    "coding-project-final-file-write-approval-gate"
  );
  assert_eq!(
    as_str(get(observed, "next_action")),
    "request-coding-project-final-file-write-approval"
  );
  assert!(!as_bool(get(observed, "host_apply_allowed")));
  assert!(!as_bool(get(observed, "file_write_allowed")));
  assert!(!as_bool(get(observed, "test_execution_allowed")));

  let ko = get(observed, "ko_self_description");
  assert!(as_str(get(ko, "text")).contains("transaction dry-run"));
  assert!(as_str(get(ko, "text")).contains("최종 파일 쓰기 승인"));

  let plan = get(observed, "plan_receipt");
  assert_eq!(
    as_str(get(plan, "current_observed_outcome")),
    "coding-project-host-apply-transaction-dry-run-passed"
  );
  assert_eq!(
    as_str(get(plan, "next_action")),
    "request-coding-project-final-file-write-approval"
  );
}

#[test]
fn final_write_approval_receipt_plans_host_apply_execution_gate() {
  let run = eval_file(&fixture_path()).unwrap();
  let observed = get(&run, "observed-final-write-approval");

  assert_eq!(
    as_str(get(observed, "outcome")),
    "mirror-self-observation-plan-built"
  );
  assert!(as_bool(get(observed, "verified")));
  assert_eq!(
    as_str(get(observed, "next_gate")),
    "coding-project-host-apply-execution-gate"
  );
  assert_eq!(
    as_str(get(observed, "next_action")),
    "build-coding-project-host-apply-execution-gate"
  );
  assert!(!as_bool(get(observed, "host_apply_allowed")));
  assert!(!as_bool(get(observed, "file_write_allowed")));
  assert!(!as_bool(get(observed, "test_execution_allowed")));

  let ko = get(observed, "ko_self_description");
  assert!(as_str(get(ko, "text")).contains("최종 파일 쓰기 승인"));
  assert!(as_str(get(ko, "text")).contains("host apply execution gate"));

  let plan = get(observed, "plan_receipt");
  assert_eq!(
    as_str(get(plan, "current_observed_outcome")),
    "coding-project-final-file-write-approval-gate-approved"
  );
  assert_eq!(
    as_str(get(plan, "next_action")),
    "build-coding-project-host-apply-execution-gate"
  );
}

#[test]
fn host_apply_execution_gate_receipt_plans_host_execution_result_verification() {
  let run = eval_file(&fixture_path()).unwrap();
  let observed = get(&run, "observed-host-apply-execution-gate");

  assert_eq!(
    as_str(get(observed, "outcome")),
    "mirror-self-observation-plan-built"
  );
  assert!(as_bool(get(observed, "verified")));
  assert_eq!(
    as_str(get(observed, "next_gate")),
    "coding-project-host-apply-execution-result"
  );
  assert_eq!(
    as_str(get(observed, "next_action")),
    "verify-coding-project-host-apply-execution-result"
  );
  assert!(!as_bool(get(observed, "host_apply_allowed")));
  assert!(!as_bool(get(observed, "file_write_allowed")));
  assert!(!as_bool(get(observed, "host_execution_allowed")));
  assert!(!as_bool(get(observed, "test_execution_allowed")));

  let ko = get(observed, "ko_self_description");
  assert!(as_str(get(ko, "text")).contains("host apply execution gate"));
  assert!(as_str(get(ko, "text")).contains("execution result receipt"));

  let plan = get(observed, "plan_receipt");
  assert_eq!(
    as_str(get(plan, "current_observed_outcome")),
    "coding-project-host-apply-execution-gate-approved"
  );
  assert_eq!(
    as_str(get(plan, "next_action")),
    "verify-coding-project-host-apply-execution-result"
  );
}

#[test]
fn host_apply_execution_result_receipt_plans_post_write_verification() {
  let run = eval_file(&fixture_path()).unwrap();
  let observed = get(&run, "observed-host-apply-execution-result");

  assert_eq!(
    as_str(get(observed, "outcome")),
    "mirror-self-observation-plan-built"
  );
  assert!(as_bool(get(observed, "verified")));
  assert_eq!(
    as_str(get(observed, "next_gate")),
    "coding-project-post-write-verification"
  );
  assert_eq!(
    as_str(get(observed, "next_action")),
    "build-coding-project-post-write-verification"
  );
  assert!(!as_bool(get(observed, "host_apply_allowed")));
  assert!(!as_bool(get(observed, "file_write_allowed")));
  assert!(!as_bool(get(observed, "host_execution_allowed")));
  assert!(!as_bool(get(observed, "test_execution_allowed")));

  let ko = get(observed, "ko_self_description");
  assert!(as_str(get(ko, "text")).contains("host bridge 실행 결과"));
  assert!(as_str(get(ko, "text")).contains("post-write snapshot"));

  let plan = get(observed, "plan_receipt");
  assert_eq!(
    as_str(get(plan, "current_observed_outcome")),
    "coding-project-host-apply-execution-result-verified"
  );
  assert_eq!(
    as_str(get(plan, "next_action")),
    "build-coding-project-post-write-verification"
  );
}

#[test]
fn post_write_verification_receipt_plans_test_execution_receipt() {
  let run = eval_file(&fixture_path()).unwrap();
  let observed = get(&run, "observed-post-write-verification");

  assert_eq!(
    as_str(get(observed, "outcome")),
    "mirror-self-observation-plan-built"
  );
  assert!(as_bool(get(observed, "verified")));
  assert_eq!(
    as_str(get(observed, "next_gate")),
    "coding-project-test-execution-receipt"
  );
  assert_eq!(
    as_str(get(observed, "next_action")),
    "build-coding-project-test-execution-receipt"
  );
  assert!(!as_bool(get(observed, "host_apply_allowed")));
  assert!(!as_bool(get(observed, "file_write_allowed")));
  assert!(!as_bool(get(observed, "host_execution_allowed")));
  assert!(!as_bool(get(observed, "test_execution_allowed")));

  let ko = get(observed, "ko_self_description");
  assert!(as_str(get(ko, "text")).contains("post-write snapshot"));
  assert!(as_str(get(ko, "text")).contains("테스트 실행 receipt"));

  let plan = get(observed, "plan_receipt");
  assert_eq!(
    as_str(get(plan, "current_observed_outcome")),
    "coding-project-post-write-verification-passed"
  );
  assert_eq!(
    as_str(get(plan, "next_action")),
    "build-coding-project-test-execution-receipt"
  );
}

#[test]
fn test_execution_receipt_plans_rollback_ready_receipt() {
  let run = eval_file(&fixture_path()).unwrap();
  let observed = get(&run, "observed-test-execution-receipt");

  assert_eq!(
    as_str(get(observed, "outcome")),
    "mirror-self-observation-plan-built"
  );
  assert!(as_bool(get(observed, "verified")));
  assert_eq!(
    as_str(get(observed, "next_gate")),
    "coding-project-rollback-ready-receipt"
  );
  assert_eq!(
    as_str(get(observed, "next_action")),
    "build-coding-project-rollback-ready-receipt"
  );
  assert!(!as_bool(get(observed, "host_apply_allowed")));
  assert!(!as_bool(get(observed, "file_write_allowed")));
  assert!(!as_bool(get(observed, "host_execution_allowed")));
  assert!(!as_bool(get(observed, "test_execution_allowed")));

  let ko = get(observed, "ko_self_description");
  assert!(as_str(get(ko, "text")).contains("테스트 실행 receipt"));
  assert!(as_str(get(ko, "text")).contains("rollback handle"));

  let plan = get(observed, "plan_receipt");
  assert_eq!(
    as_str(get(plan, "current_observed_outcome")),
    "coding-project-test-execution-receipt-verified"
  );
  assert_eq!(
    as_str(get(plan, "next_action")),
    "build-coding-project-rollback-ready-receipt"
  );
}

#[test]
fn rollback_ready_receipt_plans_complete_or_rollback_policy() {
  let run = eval_file(&fixture_path()).unwrap();
  let observed = get(&run, "observed-rollback-ready-receipt");

  assert_eq!(
    as_str(get(observed, "outcome")),
    "mirror-self-observation-plan-built"
  );
  assert!(as_bool(get(observed, "verified")));
  assert_eq!(
    as_str(get(observed, "next_gate")),
    "coding-project-complete-or-rollback-policy"
  );
  assert_eq!(
    as_str(get(observed, "next_action")),
    "build-coding-project-complete-or-rollback-policy"
  );
  assert!(!as_bool(get(observed, "host_apply_allowed")));
  assert!(!as_bool(get(observed, "file_write_allowed")));
  assert!(!as_bool(get(observed, "host_execution_allowed")));
  assert!(!as_bool(get(observed, "test_execution_allowed")));

  let ko = get(observed, "ko_self_description");
  assert!(as_str(get(ko, "text")).contains("rollback handle"));
  assert!(as_str(get(ko, "text")).contains("정책"));

  let plan = get(observed, "plan_receipt");
  assert_eq!(
    as_str(get(plan, "current_observed_outcome")),
    "coding-project-rollback-ready-receipt-built"
  );
  assert_eq!(
    as_str(get(plan, "next_action")),
    "build-coding-project-complete-or-rollback-policy"
  );
}

#[test]
fn complete_policy_receipt_plans_transaction_complete_receipt() {
  let run = eval_file(&fixture_path()).unwrap();
  let observed = get(&run, "observed-complete-policy-receipt");

  assert_eq!(
    as_str(get(observed, "outcome")),
    "mirror-self-observation-plan-built"
  );
  assert!(as_bool(get(observed, "verified")));
  assert_eq!(
    as_str(get(observed, "next_gate")),
    "coding-project-transaction-complete-receipt"
  );
  assert_eq!(
    as_str(get(observed, "next_action")),
    "build-coding-project-transaction-complete-receipt"
  );
  assert!(!as_bool(get(observed, "host_apply_allowed")));
  assert!(!as_bool(get(observed, "file_write_allowed")));
  assert!(!as_bool(get(observed, "host_execution_allowed")));
  assert!(!as_bool(get(observed, "test_execution_allowed")));

  let ko = get(observed, "ko_self_description");
  assert!(as_str(get(ko, "text")).contains("complete-ready policy"));
  assert!(as_str(get(ko, "text")).contains("complete receipt"));

  let plan = get(observed, "plan_receipt");
  assert_eq!(
    as_str(get(plan, "current_observed_outcome")),
    "coding-project-transaction-complete-policy-built"
  );
  assert_eq!(
    as_str(get(plan, "next_action")),
    "build-coding-project-transaction-complete-receipt"
  );
}

#[test]
fn rollback_policy_receipt_plans_rollback_approval_or_execution_gate() {
  let run = eval_file(&fixture_path()).unwrap();
  let observed = get(&run, "observed-rollback-policy-receipt");

  assert_eq!(
    as_str(get(observed, "outcome")),
    "mirror-self-observation-plan-built"
  );
  assert!(as_bool(get(observed, "verified")));
  assert_eq!(
    as_str(get(observed, "next_gate")),
    "coding-project-rollback-approval-or-execution-gate"
  );
  assert_eq!(
    as_str(get(observed, "next_action")),
    "request-coding-project-rollback-approval-or-execution-gate"
  );
  assert!(!as_bool(get(observed, "host_apply_allowed")));
  assert!(!as_bool(get(observed, "file_write_allowed")));
  assert!(!as_bool(get(observed, "host_execution_allowed")));
  assert!(!as_bool(get(observed, "test_execution_allowed")));

  let ko = get(observed, "ko_self_description");
  assert!(as_str(get(ko, "text")).contains("rollback-available policy"));
  assert!(as_str(get(ko, "text")).contains("rollback 승인"));

  let plan = get(observed, "plan_receipt");
  assert_eq!(
    as_str(get(plan, "current_observed_outcome")),
    "coding-project-rollback-policy-built"
  );
  assert_eq!(
    as_str(get(plan, "next_action")),
    "request-coding-project-rollback-approval-or-execution-gate"
  );
}

#[test]
fn transaction_complete_receipt_plans_pnix_db_timeline_close_or_audit() {
  let run = eval_file(&fixture_path()).unwrap();
  let observed = get(&run, "observed-transaction-complete-receipt");

  assert_eq!(
    as_str(get(observed, "outcome")),
    "mirror-self-observation-plan-built"
  );
  assert!(as_bool(get(observed, "verified")));
  assert_eq!(
    as_str(get(observed, "next_gate")),
    "pnix-db-transaction-timeline-close-or-audit"
  );
  assert_eq!(
    as_str(get(observed, "next_action")),
    "build-pnix-db-transaction-timeline-close-or-audit"
  );
  assert!(!as_bool(get(observed, "host_apply_allowed")));
  assert!(!as_bool(get(observed, "file_write_allowed")));
  assert!(!as_bool(get(observed, "host_execution_allowed")));
  assert!(!as_bool(get(observed, "test_execution_allowed")));

  let ko = get(observed, "ko_self_description");
  assert!(as_str(get(ko, "text")).contains("transaction complete receipt"));
  assert!(as_str(get(ko, "text")).contains("pnix-db transaction timeline"));

  let plan = get(observed, "plan_receipt");
  assert_eq!(
    as_str(get(plan, "current_observed_outcome")),
    "coding-project-transaction-complete-receipt-built"
  );
  assert_eq!(
    as_str(get(plan, "next_action")),
    "build-pnix-db-transaction-timeline-close-or-audit"
  );
}

#[test]
fn rollback_approval_gate_receipt_plans_rollback_execution_result_verification() {
  let run = eval_file(&fixture_path()).unwrap();
  let observed = get(&run, "observed-rollback-approval-gate-receipt");

  assert_eq!(
    as_str(get(observed, "outcome")),
    "mirror-self-observation-plan-built"
  );
  assert!(as_bool(get(observed, "verified")));
  assert_eq!(
    as_str(get(observed, "next_gate")),
    "coding-project-rollback-execution-result"
  );
  assert_eq!(
    as_str(get(observed, "next_action")),
    "verify-coding-project-rollback-execution-result"
  );
  assert!(!as_bool(get(observed, "host_apply_allowed")));
  assert!(!as_bool(get(observed, "file_write_allowed")));
  assert!(!as_bool(get(observed, "host_execution_allowed")));
  assert!(!as_bool(get(observed, "test_execution_allowed")));

  let ko = get(observed, "ko_self_description");
  assert!(as_str(get(ko, "text")).contains("rollback approval/execution gate"));
  assert!(as_str(get(ko, "text")).contains("rollback execution result receipt"));

  let plan = get(observed, "plan_receipt");
  assert_eq!(
    as_str(get(plan, "current_observed_outcome")),
    "coding-project-rollback-approval-or-execution-gate-approved"
  );
  assert_eq!(
    as_str(get(plan, "next_action")),
    "verify-coding-project-rollback-execution-result"
  );
}

#[test]
fn rollback_execution_result_receipt_plans_rollback_post_verification() {
  let run = eval_file(&fixture_path()).unwrap();
  let observed = get(&run, "observed-rollback-execution-result-receipt");

  assert_eq!(
    as_str(get(observed, "outcome")),
    "mirror-self-observation-plan-built"
  );
  assert!(as_bool(get(observed, "verified")));
  assert_eq!(
    as_str(get(observed, "next_gate")),
    "coding-project-rollback-post-verification"
  );
  assert_eq!(
    as_str(get(observed, "next_action")),
    "build-coding-project-rollback-post-verification"
  );
  assert!(!as_bool(get(observed, "host_apply_allowed")));
  assert!(!as_bool(get(observed, "file_write_allowed")));
  assert!(!as_bool(get(observed, "host_execution_allowed")));
  assert!(!as_bool(get(observed, "test_execution_allowed")));

  let ko = get(observed, "ko_self_description");
  assert!(as_str(get(ko, "text")).contains("rollback 실행 결과 receipt"));
  assert!(as_str(get(ko, "text")).contains("rollback 이후 snapshot"));

  let plan = get(observed, "plan_receipt");
  assert_eq!(
    as_str(get(plan, "current_observed_outcome")),
    "coding-project-rollback-execution-result-verified"
  );
  assert_eq!(
    as_str(get(plan, "next_action")),
    "build-coding-project-rollback-post-verification"
  );
}

#[test]
fn rollback_post_verification_receipt_plans_rollback_complete_receipt() {
  let run = eval_file(&fixture_path()).unwrap();
  let observed = get(&run, "observed-rollback-post-verification-receipt");

  assert_eq!(
    as_str(get(observed, "outcome")),
    "mirror-self-observation-plan-built"
  );
  assert!(as_bool(get(observed, "verified")));
  assert_eq!(
    as_str(get(observed, "next_gate")),
    "coding-project-rollback-complete-receipt"
  );
  assert_eq!(
    as_str(get(observed, "next_action")),
    "build-coding-project-rollback-complete-receipt"
  );
  assert!(!as_bool(get(observed, "host_apply_allowed")));
  assert!(!as_bool(get(observed, "file_write_allowed")));
  assert!(!as_bool(get(observed, "host_execution_allowed")));
  assert!(!as_bool(get(observed, "test_execution_allowed")));

  let ko = get(observed, "ko_self_description");
  assert!(as_str(get(ko, "text")).contains("rollback 이후 snapshot"));
  assert!(as_str(get(ko, "text")).contains("rollback complete receipt"));

  let plan = get(observed, "plan_receipt");
  assert_eq!(
    as_str(get(plan, "current_observed_outcome")),
    "coding-project-rollback-post-verification-passed"
  );
  assert_eq!(
    as_str(get(plan, "next_action")),
    "build-coding-project-rollback-complete-receipt"
  );
}

#[test]
fn rollback_complete_receipt_plans_pnix_db_timeline_close_or_audit() {
  let run = eval_file(&fixture_path()).unwrap();
  let observed = get(&run, "observed-rollback-complete-receipt");

  assert_eq!(
    as_str(get(observed, "outcome")),
    "mirror-self-observation-plan-built"
  );
  assert!(as_bool(get(observed, "verified")));
  assert_eq!(
    as_str(get(observed, "next_gate")),
    "pnix-db-transaction-timeline-close-or-audit"
  );
  assert_eq!(
    as_str(get(observed, "next_action")),
    "build-pnix-db-transaction-timeline-close-or-audit"
  );
  assert!(!as_bool(get(observed, "host_apply_allowed")));
  assert!(!as_bool(get(observed, "file_write_allowed")));
  assert!(!as_bool(get(observed, "host_execution_allowed")));
  assert!(!as_bool(get(observed, "test_execution_allowed")));

  let ko = get(observed, "ko_self_description");
  assert!(as_str(get(ko, "text")).contains("rollback complete receipt"));
  assert!(as_str(get(ko, "text")).contains("pnix-db transaction timeline"));

  let plan = get(observed, "plan_receipt");
  assert_eq!(
    as_str(get(plan, "current_observed_outcome")),
    "coding-project-rollback-complete-receipt-built"
  );
  assert_eq!(
    as_str(get(plan, "next_action")),
    "build-pnix-db-transaction-timeline-close-or-audit"
  );
}

#[test]
fn pnix_db_timeline_audit_receipt_closes_terminal_audit_lane() {
  let run = eval_file(&fixture_path()).unwrap();
  let observed = get(&run, "observed-pnix-db-timeline-audit-receipt");

  assert_eq!(
    as_str(get(observed, "outcome")),
    "mirror-self-observation-plan-built"
  );
  assert!(as_bool(get(observed, "verified")));
  assert_eq!(as_str(get(observed, "next_gate")), "end");
  assert_eq!(
    as_str(get(observed, "next_action")),
    "complete-pnix-db-transaction-timeline-audit"
  );
  assert!(!as_bool(get(observed, "host_apply_allowed")));
  assert!(!as_bool(get(observed, "file_write_allowed")));
  assert!(!as_bool(get(observed, "host_execution_allowed")));
  assert!(!as_bool(get(observed, "test_execution_allowed")));

  let ko = get(observed, "ko_self_description");
  assert!(as_str(get(ko, "text")).contains("transaction timeline audit"));
  assert!(as_str(get(ko, "text")).contains("audit lane"));

  let plan = get(observed, "plan_receipt");
  assert_eq!(
    as_str(get(plan, "current_observed_outcome")),
    "pnix-db-transaction-timeline-audit-passed"
  );
  assert_eq!(
    as_str(get(plan, "next_action")),
    "complete-pnix-db-transaction-timeline-audit"
  );
}

#[test]
fn missing_next_gate_and_effects_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let missing = get(&run, "missing-next-gate");
  assert!(as_bool(get(missing, "is_held")));
  assert_eq!(
    as_str(get(missing, "outcome")),
    "held-mirror-self-observation-next-gate-required"
  );
  assert!(!as_bool(get(missing, "mirror_plan_built")));

  let request = get(&run, "effect-request");
  assert!(as_bool(get(request, "is_held")));
  assert_eq!(
    as_str(get(request, "outcome")),
    "held-mirror-self-observation-effect-blocked"
  );

  let allowed = get(&run, "effect-allowed-receipt");
  assert!(as_bool(get(allowed, "is_held")));
  assert_eq!(
    as_str(get(allowed, "outcome")),
    "held-mirror-self-observation-receipt-effect-allowed"
  );
  assert!(!as_bool(get(allowed, "host_apply_allowed")));
  assert!(!as_bool(get(allowed, "file_write_allowed")));
}
