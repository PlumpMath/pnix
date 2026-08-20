use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/pnix-query-runtime/coding-project-adaptive-final-apply-policy-to-final-approval-gate.px",
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
  let run =
    eval_file(&fixture_path()).expect("adaptive final apply approval fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "proof")),
    "coding-project-adaptive-final-apply-policy-to-final-approval-gate"
  );
}

#[test]
fn adaptive_final_approval_allows_host_apply_plan_only() {
  let run = eval_file(&fixture_path()).unwrap();

  let policy = get(&run, "final-apply-policy");
  assert_eq!(
    as_str(get(policy, "outcome")),
    "coding-project-final-apply-approval-required"
  );
  assert_eq!(
    as_str(get(policy, "next_gate")),
    "coding-project-final-apply-approval-gate"
  );
  assert!(as_bool(get(policy, "final_apply_approval_required")));
  assert!(!as_bool(get(policy, "host_apply_plan_ready")));

  let approval = get(&run, "final-apply-approval");
  assert_eq!(
    as_str(get(approval, "approval_kind")),
    "coding-project-final-apply-approval-token-v0"
  );
  assert_eq!(
    as_str(get(approval, "approved_mode")),
    "project-patch-final-host-apply"
  );
  assert!(as_bool(get(approval, "host_apply_plan_requested")));
  assert!(!as_bool(get(approval, "host_apply_requested")));

  let gate = get(&run, "final-approval-gate");
  assert_eq!(
    as_str(get(gate, "schema")),
    "puncheetah.code.final-apply-approval-gate.v0"
  );
  assert_eq!(
    as_str(get(gate, "outcome")),
    "coding-project-final-apply-approval-gate-approved"
  );
  assert!(as_bool(get(gate, "verified")));
  assert!(as_bool(get(gate, "final_apply_approval_verified")));
  assert!(!as_bool(get(gate, "final_apply_approval_token_consumed")));
  assert!(as_bool(get(gate, "host_apply_plan_allowed")));
  assert!(!as_bool(get(gate, "host_apply_plan_ready")));
  assert!(!as_bool(get(gate, "host_apply_allowed")));
  assert_eq!(
    as_str(get(gate, "approved_preview_id")),
    "reopened-plan-preview-demo"
  );
  assert_eq!(
    as_str(get(gate, "approved_preview_hash")),
    "sha256-reopened-plan-preview-demo"
  );
  assert_eq!(as_i64(get(gate, "approved_file_count")), 2);
  assert_eq!(
    as_str(get(gate, "prior_preview_approval_id")),
    "approval-adaptive-preview-demo"
  );
  assert_eq!(
    as_str(get(gate, "next_gate")),
    "coding-project-host-apply-plan"
  );

  assert!(!as_bool(get(gate, "file_write_allowed")));
  assert!(!as_bool(get(gate, "host_execution_allowed")));
  assert!(!as_bool(get(gate, "direct_apply_allowed")));
  assert!(!as_bool(get(gate, "apply_allowed")));
  assert!(!as_bool(get(gate, "raw_eval_allowed")));
  assert!(!as_bool(get(gate, "test_execution_allowed")));
  assert!(!as_bool(get(gate, "memory_write_allowed")));
  assert!(!as_bool(get(gate, "policy_persistence_allowed")));
  assert!(!as_bool(get(gate, "accepted_fact_promotion_allowed")));
  assert!(!as_bool(get(gate, "route_update_allowed")));

  let audit = get(gate, "final_apply_approval_audit");
  assert_eq!(
    as_str(get(audit, "policy_outcome")),
    "coding-project-final-apply-approval-required"
  );
  assert_eq!(
    as_str(get(audit, "test_plan_receipt_outcome")),
    "coding-project-test-plan-receipt-built"
  );
  assert_eq!(
    as_str(get(audit, "source_anchor_outcome")),
    "coding-project-source-anchor-check-passed"
  );
  assert_eq!(
    as_str(get(audit, "apply_dry_run_outcome")),
    "coding-project-apply-dry-run-passed"
  );

  let receipt = get(gate, "receipt");
  assert!(as_bool(get(receipt, "host_apply_plan_allowed")));
  assert!(!as_bool(get(receipt, "host_apply_allowed")));
  assert_eq!(
    as_str(get(receipt, "invariant")),
    "final human approval matches final apply requirements; only host apply plan construction is allowed next, no write/apply/test/host execution occurs here"
  );

  let safety = get(gate, "approval_safety_receipt");
  assert_eq!(
    as_str(get(safety, "effect_contract")),
    "final-approval-only-host-plan-next-no-write-no-apply-no-test-no-host-exec"
  );
  assert!(!as_bool(get(safety, "policy_persistence_allowed")));
}

#[test]
fn reasoning_dispatch_can_request_final_apply_approval() {
  let run = eval_file(&fixture_path()).unwrap();
  let dispatched = get(&run, "dispatched-final-approval");
  assert_eq!(
    as_str(get(dispatched, "op")),
    "request-coding-project-final-apply-approval"
  );

  let result = get(dispatched, "result");
  assert_eq!(
    as_str(get(result, "outcome")),
    "coding-project-final-apply-approval-gate-approved"
  );
  assert!(as_bool(get(result, "host_apply_plan_allowed")));
  assert_eq!(
    as_str(get(result, "next_gate")),
    "coding-project-host-apply-plan"
  );
}

#[test]
fn missing_mismatch_intent_policy_and_effects_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let missing = get(&run, "missing-approval");
  assert!(as_bool(get(missing, "is_held")));
  assert_eq!(
    as_str(get(missing, "outcome")),
    "held-coding-project-final-apply-approval-token-required"
  );
  assert!(!as_bool(get(missing, "host_apply_plan_allowed")));

  let hash = get(&run, "hash-mismatch");
  assert!(as_bool(get(hash, "is_held")));
  assert_eq!(
    as_str(get(hash, "outcome")),
    "held-coding-project-final-apply-approval-token-mismatch"
  );

  let prior = get(&run, "prior-approval-mismatch");
  assert!(as_bool(get(prior, "is_held")));
  assert_eq!(
    as_str(get(prior, "outcome")),
    "held-coding-project-final-apply-approval-token-mismatch"
  );

  let missing_intent = get(&run, "missing-host-plan-intent");
  assert!(as_bool(get(missing_intent, "is_held")));
  assert_eq!(
    as_str(get(missing_intent, "outcome")),
    "held-coding-project-final-apply-approval-token-required"
  );

  let bad_policy = get(&run, "bad-policy");
  assert!(as_bool(get(bad_policy, "is_held")));
  assert_eq!(
    as_str(get(bad_policy, "outcome")),
    "held-coding-project-final-apply-policy-required"
  );

  let effect = get(&run, "effect-held");
  assert!(as_bool(get(effect, "is_held")));
  assert_eq!(
    as_str(get(effect, "outcome")),
    "held-coding-project-final-apply-approval-effect-blocked"
  );
  assert!(!as_bool(get(effect, "host_apply_plan_allowed")));
  assert!(!as_bool(get(effect, "host_apply_allowed")));
  assert!(!as_bool(get(effect, "file_write_allowed")));
  assert!(!as_bool(get(effect, "test_execution_allowed")));
}
