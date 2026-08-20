use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/pnix-query-runtime/coding-project-adaptive-test-plan-to-final-apply-policy.px",
  )
}

fn as_attrs(v: &Value) -> &BTreeMap<String, Value> {
  match v {
    Value::AttrSet(m) => m,
    other => panic!("expected attrset, got {:?}", other),
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
  let run = eval_file(&fixture_path())
    .expect("adaptive test-plan to final apply policy fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "proof")),
    "coding-project-adaptive-test-plan-to-final-apply-policy"
  );
}

#[test]
fn adaptive_test_plan_requires_final_apply_approval_without_host_effects() {
  let run = eval_file(&fixture_path()).unwrap();

  let test_plan = get(&run, "test-plan-receipt");
  assert_eq!(
    as_str(get(test_plan, "outcome")),
    "coding-project-test-plan-receipt-built"
  );
  assert_eq!(
    as_str(get(test_plan, "next_gate")),
    "coding-project-final-apply-approval-or-host-plan"
  );
  assert!(!as_bool(get(test_plan, "test_execution_allowed")));

  let policy = get(&run, "final-apply-policy");
  assert_eq!(
    as_str(get(policy, "schema")),
    "puncheetah.code.final-apply-policy.v0"
  );
  assert_eq!(
    as_str(get(policy, "outcome")),
    "coding-project-final-apply-approval-required"
  );
  assert!(as_bool(get(policy, "verified")));
  assert!(as_bool(get(policy, "final_apply_policy_built")));
  assert!(as_bool(get(policy, "final_apply_approval_required")));
  assert!(as_bool(get(policy, "host_apply_plan_precondition_ready")));
  assert!(!as_bool(get(policy, "host_apply_plan_ready")));
  assert!(!as_bool(get(policy, "host_plan_bypass_allowed")));
  assert_eq!(
    as_str(get(policy, "approved_preview_id")),
    "reopened-plan-preview-demo"
  );
  assert_eq!(
    as_str(get(policy, "approved_preview_hash")),
    "sha256-reopened-plan-preview-demo"
  );
  assert_eq!(
    as_str(get(policy, "next_gate")),
    "coding-project-final-apply-approval-gate"
  );

  assert!(!as_bool(get(policy, "file_write_allowed")));
  assert!(!as_bool(get(policy, "host_execution_allowed")));
  assert!(!as_bool(get(policy, "host_apply_allowed")));
  assert!(!as_bool(get(policy, "direct_apply_allowed")));
  assert!(!as_bool(get(policy, "apply_allowed")));
  assert!(!as_bool(get(policy, "raw_eval_allowed")));
  assert!(!as_bool(get(policy, "test_execution_allowed")));

  let requirements = get(policy, "final_apply_approval_requirements");
  assert_eq!(
    as_str(get(requirements, "approval_kind")),
    "coding-project-final-apply-approval-token-v0"
  );
  assert_eq!(
    as_str(get(requirements, "approved_mode")),
    "project-patch-final-host-apply"
  );
  assert_eq!(
    as_str(get(requirements, "prior_preview_approval_id")),
    "approval-adaptive-preview-demo"
  );
  assert_eq!(
    as_str(get(requirements, "test_plan_receipt_outcome")),
    "coding-project-test-plan-receipt-built"
  );
  assert_eq!(
    as_str(get(requirements, "next_gate")),
    "coding-project-final-apply-approval-gate"
  );

  let summary = get(policy, "prior_gate_summary");
  assert_eq!(as_i64(get(summary, "edit_count")), 2);
  assert_eq!(as_i64(get(summary, "dry_run_count")), 2);
  assert_eq!(as_i64(get(summary, "anchor_count")), 2);
  assert_eq!(
    as_str(get(summary, "test_plan_receipt_outcome")),
    "coding-project-test-plan-receipt-built"
  );

  let receipt = get(policy, "receipt");
  assert!(as_bool(get(receipt, "final_apply_policy_built")));
  assert!(as_bool(get(receipt, "final_apply_approval_required")));
  assert!(!as_bool(get(receipt, "host_apply_plan_ready")));
  assert_eq!(
    as_str(get(receipt, "next_gate")),
    "coding-project-final-apply-approval-gate"
  );

  let safety = get(policy, "patch_safety_receipt");
  assert!(as_bool(get(safety, "verified")));
  assert_eq!(
    as_str(get(safety, "effect_contract")),
    "final-apply-policy-only-no-host-plan-no-write-no-test-no-raw-eval"
  );
}

#[test]
fn missing_test_plan_chain_mismatch_bypass_and_effects_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let missing = get(&run, "missing-test-plan");
  assert!(as_bool(get(missing, "is_held")));
  assert_eq!(
    as_str(get(missing, "outcome")),
    "held-coding-project-test-plan-receipt-required"
  );
  assert!(!as_bool(get(missing, "final_apply_policy_built")));

  let mismatch = get(&run, "chain-mismatch");
  assert!(as_bool(get(mismatch, "is_held")));
  assert_eq!(
    as_str(get(mismatch, "outcome")),
    "held-coding-project-final-apply-chain-mismatch"
  );
  assert!(!as_bool(get(mismatch, "final_apply_policy_built")));

  let bypass = get(&run, "host-plan-bypass-held");
  assert!(as_bool(get(bypass, "is_held")));
  assert_eq!(
    as_str(get(bypass, "outcome")),
    "held-coding-project-final-apply-host-plan-bypass-blocked"
  );
  assert!(!as_bool(get(bypass, "host_apply_plan_ready")));

  let effect = get(&run, "effect-held");
  assert!(as_bool(get(effect, "is_held")));
  assert_eq!(
    as_str(get(effect, "outcome")),
    "held-coding-project-final-apply-policy-effect-blocked"
  );
  assert!(!as_bool(get(effect, "file_write_allowed")));
  assert!(!as_bool(get(effect, "host_apply_allowed")));
  assert!(!as_bool(get(effect, "test_execution_allowed")));
}
