use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/pnix-query-runtime/coding-project-final-apply-approval-or-host-plan-receipt.px",
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
  let run = eval_file(&fixture_path()).expect("final apply policy fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "proof")),
    "coding-project-final-apply-approval-or-host-plan"
  );

  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "puncheetah.contract.coding-project-final-apply-approval-or-host-plan.v0"
  );
  assert_eq!(
    as_str(get(meta, "base")),
    "coding-project-final-apply-approval-or-host-plan-v0"
  );
}

#[test]
fn coherent_chain_requires_final_apply_approval_without_effects() {
  let run = eval_file(&fixture_path()).unwrap();
  let passed = get(&run, "passed");

  assert_eq!(
    as_str(get(passed, "schema")),
    "puncheetah.code.final-apply-policy.v0"
  );
  assert_eq!(
    as_str(get(passed, "outcome")),
    "coding-project-final-apply-approval-required"
  );
  assert!(as_bool(get(passed, "verified")));
  assert!(as_bool(get(passed, "final_apply_policy_built")));
  assert!(as_bool(get(passed, "final_apply_approval_required")));
  assert!(as_bool(get(passed, "host_apply_plan_precondition_ready")));
  assert!(!as_bool(get(passed, "host_apply_plan_ready")));
  assert!(!as_bool(get(passed, "host_plan_bypass_allowed")));
  assert_eq!(
    as_str(get(passed, "next_gate")),
    "coding-project-final-apply-approval-gate"
  );

  assert!(!as_bool(get(passed, "file_write_allowed")));
  assert!(!as_bool(get(passed, "host_execution_allowed")));
  assert!(!as_bool(get(passed, "host_apply_allowed")));
  assert!(!as_bool(get(passed, "direct_apply_allowed")));
  assert!(!as_bool(get(passed, "apply_allowed")));
  assert!(!as_bool(get(passed, "raw_eval_allowed")));
  assert!(!as_bool(get(passed, "test_execution_allowed")));

  let requirements = get(passed, "final_apply_approval_requirements");
  assert_eq!(
    as_str(get(requirements, "approval_kind")),
    "coding-project-final-apply-approval-token-v0"
  );
  assert_eq!(
    as_str(get(requirements, "approved_mode")),
    "project-patch-final-host-apply"
  );
  assert_eq!(
    as_str(get(requirements, "next_gate")),
    "coding-project-final-apply-approval-gate"
  );

  let summary = get(passed, "prior_gate_summary");
  assert_eq!(as_i64(get(summary, "edit_count")), 2);
  assert_eq!(as_i64(get(summary, "dry_run_count")), 2);
  assert_eq!(as_i64(get(summary, "anchor_count")), 2);

  let receipt = get(passed, "receipt");
  assert_eq!(
    as_str(get(receipt, "invariant")),
    "source anchor, dry-run, rollback simulation, and test plan receipt are coherent; final apply approval is still required and no write/test/host effect is allowed"
  );
}

#[test]
fn absent_policy_defaults_to_safe_final_approval_required() {
  let run = eval_file(&fixture_path()).unwrap();
  let passed = get(&run, "default-policy-passed");

  assert_eq!(
    as_str(get(passed, "outcome")),
    "coding-project-final-apply-approval-required"
  );
  assert!(as_bool(get(passed, "final_apply_approval_required")));
  assert!(!as_bool(get(passed, "host_apply_plan_ready")));
  assert_eq!(
    as_str(get(passed, "next_gate")),
    "coding-project-final-apply-approval-gate"
  );
}

#[test]
fn mismatch_bypass_and_effects_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

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
  assert!(!as_bool(get(effect, "test_execution_allowed")));
}
