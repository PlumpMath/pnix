use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/macro-only-boot-bounded-replay-execution-owner.px")
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

fn string_set(v: &Value) -> BTreeSet<&str> {
  as_list(v).iter().map(as_str).collect()
}

#[test]
fn bounded_replay_fixture_imports_owner_and_fresh_puck_receipt() {
  let run = eval_file(&fixture_path()).expect("bounded replay owner fixture");
  assert_eq!(
    as_str(get(&run, "proof")),
    "macro-only-boot-bounded-replay-execution-owner"
  );
  assert!(as_bool(get(&run, "imported-owner")));
  assert!(as_bool(get(&run, "used-px-owner")));
  assert_eq!(
    as_str(get(&run, "imported-fresh-puck")),
    "literal-runner-ready-current-cut-input"
  );
}

#[test]
fn owner_meta_declares_replay_execution_without_boot_runtime_or_host_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.lib.gate.macro-only-boot-bounded-replay-execution"
  );
  assert_eq!(
    as_str(get(meta, "constructor")),
    "validateBoundedReplayExecution"
  );
  assert_eq!(
    as_str(get(meta, "output-shape")),
    "bounded-replay-executed or Held"
  );
  assert!(as_bool(get(meta, "bounded-replay-executed")));
  for key in [
    "boot-executed",
    "macro-only-runtime-owner-booted",
    "new-engine-from-zero",
    "runtime-install",
    "global-ontology-runtime",
    "host-code-removal-started",
    "semantic-owner",
  ] {
    assert!(!as_bool(get(meta, key)), "`{key}` must stay false");
  }
}

#[test]
fn expected_stage_schedule_bounds_and_evidence_are_pinned() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "expected-current-stage")),
    "macro-only-runner-ready-for-bounded-replay"
  );
  assert_eq!(
    as_str(get(&run, "expected-runner-status")),
    "runner-ready-for-bounded-replay"
  );
  assert_eq!(
    as_str(get(&run, "expected-semantic-delta-status")),
    "empty-or-held-only"
  );
  assert_eq!(as_i64(get(&run, "expected-node-count")), 11);
  assert_eq!(as_i64(get(&run, "max-edge-count")), 14);
  assert_eq!(as_i64(get(&run, "max-import-depth")), 4);
  assert_eq!(as_i64(get(&run, "max-stack-depth")), 6);

  let steps = string_set(get(&run, "expected-replay-step-ids"));
  for expected in [
    "replay.node.constitution-owner",
    "replay.node.promote-r7-compat",
    "replay.node.evaluate-select-ranking-owner",
    "replay.node.evaluate-select-route-adapter-owner",
    "replay.node.evaluate-select-scoped-adapter-install",
    "replay.node.lift-query-emit-r7-compat",
    "replay.node.internal-self-capability-map",
    "replay.node.host-removal-map",
    "replay.node.macro-only-boot-manifest",
    "replay.node.macro-only-boot-attempt",
    "replay.node.macro-only-boot-runner-owner",
  ] {
    assert!(steps.contains(expected), "missing replay step `{expected}`");
  }
  assert_eq!(steps.len(), 11);

  let evidence = string_set(get(&run, "required-evidence"));
  for expected in [
    "runner-ready-for-bounded-replay",
    "runner-missing-count-zero",
    "fresh-p-puck-current-cut-present",
    "bounded-node-schedule-present",
    "replay-step-trace-present",
    "cycle-policy-held-preserved",
    "stack-budget-respected",
    "negative-held-retained",
    "semantic-delta-empty-or-held",
    "boot-executed-false-recorded",
    "host-removal-started-false-recorded",
    "gpl-family-dependencies-false",
  ] {
    assert!(evidence.contains(expected), "missing evidence `{expected}`");
  }
}

#[test]
fn valid_proof_executes_bounded_replay_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let valid = get(&run, "valid-proof");
  assert_eq!(as_str(get(valid, "status")), "bounded-replay-executed");
  assert_eq!(
    as_str(get(valid, "bounded-replay-execution-status")),
    "executed"
  );
  assert!(matches!(get(valid, "held-id"), Value::Null));
  assert!(as_bool(get(valid, "ready-for-bounded-replay-input")));
  assert_eq!(as_i64(get(valid, "runner-missing-count")), 0);
  assert!(as_bool(get(valid, "bounded-replay-executed")));
  assert!(as_bool(get(valid, "bounded-replay-execution-proof")));
  assert_eq!(
    as_str(get(valid, "semantic-delta-status")),
    "empty-or-held-only"
  );
  for key in [
    "boot-executed",
    "macro-only-runtime-owner-booted",
    "new-engine-from-zero",
    "runtime-install",
    "global-ontology-runtime",
    "host-code-removal-started",
    "host-removal-safe",
    "semantic-owner",
    "old-host-authority",
    "gpl-family-dependencies",
    "implementation-command",
  ] {
    assert!(!as_bool(get(valid, key)), "`{key}` must stay false");
  }
  assert_eq!(as_i64(get(valid, "delete-ready-target-count")), 0);
}

#[test]
fn wrong_stage_runner_schedule_missing_bound_negative_held_and_semantic_delta_are_held() {
  let run = eval_file(&fixture_path()).unwrap();
  for (key, held_id) in [
    (
      "wrong-proof",
      "held.macro-only-bounded-replay.proof-id-mismatch",
    ),
    (
      "stale-stage",
      "held.macro-only-bounded-replay.stale-current-stage",
    ),
    (
      "runner-not-ready",
      "held.macro-only-bounded-replay.runner-not-ready",
    ),
    (
      "missing-evidence",
      "held.macro-only-bounded-replay.missing-required-evidence",
    ),
    (
      "schedule-mismatch",
      "held.macro-only-bounded-replay.schedule-mismatch",
    ),
    (
      "bound-violation",
      "held.macro-only-bounded-replay.bound-violation",
    ),
    (
      "negative-held-lost",
      "held.macro-only-bounded-replay.missing-required-evidence",
    ),
    (
      "semantic-delta-overclaim",
      "held.macro-only-bounded-replay.semantic-delta-overclaim",
    ),
  ] {
    let output = get(&run, key);
    assert_eq!(as_str(get(output, "status")), "Held");
    assert_eq!(as_str(get(output, "held-id")), held_id);
    assert!(!as_bool(get(output, "bounded-replay-executed")));
  }
}

#[test]
fn proof_blocks_old_host_boot_host_removal_semantic_owner_and_gpl_claims() {
  let run = eval_file(&fixture_path()).unwrap();
  for (key, held_id) in [
    (
      "old-host-authority",
      "held.macro-only-bounded-replay.old-host-authority",
    ),
    (
      "boot-claim",
      "held.macro-only-bounded-replay.boot-or-runtime-claim",
    ),
    (
      "host-removal-claim",
      "held.macro-only-bounded-replay.host-removal-claim",
    ),
    (
      "semantic-owner-claim",
      "held.macro-only-bounded-replay.semantic-owner-claim",
    ),
    (
      "gpl-claim",
      "held.macro-only-bounded-replay.gpl-family-dependency",
    ),
  ] {
    let output = get(&run, key);
    assert_eq!(as_str(get(output, "status")), "Held");
    assert_eq!(as_str(get(output, "held-id")), held_id);
    assert!(!as_bool(get(output, "bounded-replay-executed")));
  }
}

#[test]
fn remaining_frontiers_are_boot_host_removal_and_full_audit_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let frontiers = string_set(get(&run, "remaining-open-frontiers"));
  assert!(frontiers.contains("macro-only-boot-execution-proof-after-bounded-replay"));
  assert!(frontiers.contains("host-code-removal-execution-proof-after-successful-boot"));
  assert!(frontiers.contains("full-current-receipt-audit-after-bounded-replay"));
  assert_eq!(frontiers.len(), 3);
}
