use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/coding-project-final-apply-approval-gate-receipt.px")
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
  let run = eval_file(&fixture_path()).expect("final apply approval fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "proof")),
    "coding-project-final-apply-approval-gate"
  );

  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "puncheetah.contract.coding-project-final-apply-approval-gate.v0"
  );
  assert_eq!(
    as_str(get(meta, "base")),
    "coding-project-final-apply-approval-gate-v0"
  );
}

#[test]
fn final_approval_token_allows_host_apply_plan_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let passed = get(&run, "passed");

  assert_eq!(
    as_str(get(passed, "schema")),
    "puncheetah.code.final-apply-approval-gate.v0"
  );
  assert_eq!(
    as_str(get(passed, "outcome")),
    "coding-project-final-apply-approval-gate-approved"
  );
  assert!(as_bool(get(passed, "verified")));
  assert!(as_bool(get(passed, "final_apply_approval_verified")));
  assert!(as_bool(get(passed, "host_apply_plan_allowed")));
  assert!(!as_bool(get(passed, "host_apply_plan_ready")));
  assert!(!as_bool(get(passed, "host_apply_allowed")));
  assert_eq!(
    as_str(get(passed, "next_gate")),
    "coding-project-host-apply-plan"
  );

  assert!(!as_bool(get(passed, "file_write_allowed")));
  assert!(!as_bool(get(passed, "host_execution_allowed")));
  assert!(!as_bool(get(passed, "direct_apply_allowed")));
  assert!(!as_bool(get(passed, "apply_allowed")));
  assert!(!as_bool(get(passed, "raw_eval_allowed")));
  assert!(!as_bool(get(passed, "test_execution_allowed")));

  assert_eq!(as_i64(get(passed, "approved_file_count")), 2);

  let audit = get(passed, "final_apply_approval_audit");
  assert_eq!(
    as_str(get(audit, "approved_mode")),
    "project-patch-final-host-apply"
  );
  assert_eq!(
    as_str(get(audit, "policy_outcome")),
    "coding-project-final-apply-approval-required"
  );

  let receipt = get(passed, "receipt");
  assert!(as_bool(get(receipt, "host_apply_plan_allowed")));
  assert!(!as_bool(get(receipt, "host_apply_allowed")));
  assert_eq!(
    as_str(get(receipt, "invariant")),
    "final human approval matches final apply requirements; only host apply plan construction is allowed next, no write/apply/test/host execution occurs here"
  );
}

#[test]
fn token_mismatch_policy_failure_and_effects_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let hash = get(&run, "hash-mismatch");
  assert!(as_bool(get(hash, "is_held")));
  assert_eq!(
    as_str(get(hash, "outcome")),
    "held-coding-project-final-apply-approval-token-mismatch"
  );
  assert!(!as_bool(get(hash, "host_apply_plan_allowed")));

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
  assert!(!as_bool(get(effect, "file_write_allowed")));
  assert!(!as_bool(get(effect, "test_execution_allowed")));
}
