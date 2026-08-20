use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/pnix-query-runtime/macro-only-host-removal-slow-path-repeat-proof-owner.px",
  )
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
fn fixture_imports_owner_and_uses_actual_repeat_report() {
  let run = eval_file(&fixture_path()).expect("host removal slow-path repeat owner fixture");
  assert_eq!(
    as_str(get(&run, "proof")),
    "macro-only-host-removal-slow-path-repeat-proof-owner"
  );
  assert!(as_bool(get(&run, "imported-owner")));
  assert!(as_bool(get(&run, "used-px-owner")));
  assert_eq!(
    as_str(get(&run, "imported-host-removal-fresh-puck")),
    "literal-host-removal-fresh-puck-slow-path-input"
  );
  assert_eq!(
    as_str(get(&run, "p-puck-proof-source")),
    "actual repeat p-puck pnixc report over host-removal execution proof receipt"
  );
}

#[test]
fn owner_meta_declares_repeat_frontier_closed_without_delete() {
  let run = eval_file(&fixture_path()).unwrap();
  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.lib.gate.macro-only-host-removal-slow-path-repeat-proof"
  );
  assert_eq!(
    as_str(get(meta, "constructor")),
    "validateHostRemovalSlowPathRepeatProof"
  );
  assert!(as_bool(get(meta, "host-removal-slow-path-repeat-proof")));
  assert!(as_bool(get(meta, "slow-path-repeat-within-threshold")));
  assert!(as_bool(get(meta, "slow-path-repeat-frontier-closed")));
  assert!(!as_bool(get(meta, "persistent-slow-path")));
  assert!(!as_bool(get(meta, "profile-required-from-repeat")));
  for key in [
    "p-puck-is-semantic-owner",
    "actual-host-removal-patch-authorized",
    "host-code-removal-started",
    "runtime-install",
    "global-ontology-runtime",
    "new-engine-from-zero",
    "old-host-authority",
    "gpl-family-dependencies",
    "implementation-command",
  ] {
    assert!(!as_bool(get(meta, key)), "`{key}` must stay false");
  }
  assert_eq!(as_i64(get(meta, "delete-ready-target-count")), 0);
}

#[test]
fn repeat_report_telemetry_is_pinned() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "expected-repeat-report-name")),
    "macro-only-current-cut-host-removal-execution-proof-repeat"
  );
  assert_eq!(
    as_str(get(&run, "expected-prior-report-name")),
    "macro-only-current-cut-host-removal-execution-proof"
  );
  assert_eq!(
    as_str(get(&run, "expected-audited-receipt")),
    "fixtures/tesseract-macro-legacy-probe/macro_only_host_removal_execution_proof_receipt.px"
  );
  assert_eq!(as_i64(get(&run, "prior-gate-duration-ms")), 5389);
  assert_eq!(as_i64(get(&run, "expected-repeat-duration-ms")), 551);
  assert_eq!(
    as_i64(get(&run, "expected-puck-previous-duration-ms")),
    5094
  );
  assert_eq!(as_i64(get(&run, "expected-duration-delta-ms")), -4543);
  assert_eq!(as_i64(get(&run, "slow-threshold-ms")), 5000);
  assert_eq!(
    as_str(get(&run, "expected-prior-slow-path-status")),
    "slow-path-candidate"
  );
  assert_eq!(
    as_str(get(&run, "expected-repeat-slow-path-status")),
    "within-threshold"
  );
  assert_eq!(
    as_str(get(&run, "expected-duration-delta-status")),
    "faster-than-previous"
  );
}

#[test]
fn valid_proof_closes_repeat_frontier_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let valid = get(&run, "valid-proof");
  assert_eq!(
    as_str(get(valid, "status")),
    "host-removal-slow-path-repeat-within-threshold"
  );
  assert!(matches!(get(valid, "held-id"), Value::Null));
  assert!(as_bool(get(valid, "host-removal-slow-path-repeat-proof")));
  assert!(as_bool(get(valid, "slow-path-repeat-within-threshold")));
  assert!(as_bool(get(valid, "slow-path-repeat-frontier-closed")));
  assert!(!as_bool(get(valid, "persistent-slow-path")));
  assert!(!as_bool(get(valid, "profile-required-from-repeat")));
  assert_eq!(as_i64(get(valid, "repeat-duration-ms")), 551);
  assert_eq!(as_i64(get(valid, "slow-steps-count")), 0);
  assert_eq!(as_i64(get(valid, "slowest-steps-count")), 0);
  for key in [
    "actual-host-removal-patch-authorized",
    "host-code-removal-started",
    "fresh-puck-before-delete",
    "host-removal-safe",
    "runtime-install",
    "global-ontology-runtime",
    "new-engine-from-zero",
    "old-host-authority",
    "gpl-family-dependencies",
    "implementation-command",
  ] {
    assert!(!as_bool(get(valid, key)), "`{key}` must stay false");
  }
  assert_eq!(as_i64(get(valid, "delete-ready-target-count")), 0);
}

#[test]
fn required_evidence_and_remaining_frontiers_are_explicit() {
  let run = eval_file(&fixture_path()).unwrap();
  let evidence = string_set(get(&run, "required-evidence"));
  for expected in [
    "host-removal-fresh-puck-current-cut-present",
    "prior-slow-path-candidate-recorded",
    "p-puck-repeat-report-present",
    "repeat-slow-path-status-within-threshold",
    "duration-delta-status-faster-than-previous",
    "no-slow-steps-recorded",
    "actual-delete-patch-false-recorded",
  ] {
    assert!(evidence.contains(expected), "missing evidence `{expected}`");
  }

  let frontiers = string_set(get(&run, "remaining-open-frontiers"));
  assert!(!frontiers.contains("host-removal-slow-path-repeat-or-profile-before-delete"));
  assert!(frontiers.contains("actual-host-removal-patch-after-fresh-puck"));
  assert!(frontiers.contains("global-runtime-install-proof-after-semantic-owner"));
  assert!(frontiers.contains("domain-runtime-api-flattening-after-semantic-owner"));
  assert_eq!(frontiers.len(), 4);
}

#[test]
fn stale_wrong_missing_report_receipt_and_telemetry_cases_are_held() {
  let run = eval_file(&fixture_path()).unwrap();
  for (key, held_id) in [
    (
      "wrong-proof",
      "held.macro-only-host-removal-slow-path-repeat.proof-id-mismatch",
    ),
    (
      "stale-stage",
      "held.macro-only-host-removal-slow-path-repeat.stale-current-stage",
    ),
    (
      "prior-slow-path-missing",
      "held.macro-only-host-removal-slow-path-repeat.prior-slow-path-missing",
    ),
    (
      "report-mismatch",
      "held.macro-only-host-removal-slow-path-repeat.report-mismatch",
    ),
    (
      "telemetry-mismatch",
      "held.macro-only-host-removal-slow-path-repeat.telemetry-mismatch",
    ),
    (
      "receipt-mismatch",
      "held.macro-only-host-removal-slow-path-repeat.current-cut-receipt-mismatch",
    ),
    (
      "telemetry-number-drift",
      "held.macro-only-host-removal-slow-path-repeat.telemetry-number-drift",
    ),
    (
      "telemetry-status-drift",
      "held.macro-only-host-removal-slow-path-repeat.telemetry-status-drift",
    ),
    (
      "profile-overclaim",
      "held.macro-only-host-removal-slow-path-repeat.profile-overclaim",
    ),
    (
      "missing-evidence",
      "held.macro-only-host-removal-slow-path-repeat.missing-required-evidence",
    ),
  ] {
    let output = get(&run, key);
    assert_eq!(as_str(get(output, "status")), "Held");
    assert_eq!(as_str(get(output, "held-id")), held_id);
    assert!(!as_bool(get(output, "host-removal-slow-path-repeat-proof")));
  }
}

#[test]
fn delete_runtime_semantic_old_host_and_gpl_overclaims_are_held() {
  let run = eval_file(&fixture_path()).unwrap();
  for (key, held_id) in [
    (
      "delete-claim",
      "held.macro-only-host-removal-slow-path-repeat.delete-overclaim",
    ),
    (
      "runtime-claim",
      "held.macro-only-host-removal-slow-path-repeat.runtime-overclaim",
    ),
    (
      "semantic-owner-claim",
      "held.macro-only-host-removal-slow-path-repeat.semantic-owner-claim",
    ),
    (
      "old-host-authority",
      "held.macro-only-host-removal-slow-path-repeat.old-host-authority",
    ),
    (
      "gpl-claim",
      "held.macro-only-host-removal-slow-path-repeat.gpl-family-dependency",
    ),
  ] {
    let output = get(&run, key);
    assert_eq!(as_str(get(output, "status")), "Held");
    assert_eq!(as_str(get(output, "held-id")), held_id);
    assert!(!as_bool(get(output, "host-code-removal-started")));
    assert_eq!(as_i64(get(output, "delete-ready-target-count")), 0);
  }
}

#[test]
fn top_level_state_records_repeat_clearance_without_runtime_or_delete() {
  let run = eval_file(&fixture_path()).unwrap();
  assert!(as_bool(get(&run, "host-removal-slow-path-repeat-proof")));
  assert!(as_bool(get(&run, "slow-path-repeat-within-threshold")));
  assert!(as_bool(get(&run, "slow-path-repeat-frontier-closed")));
  assert!(!as_bool(get(&run, "persistent-slow-path")));
  assert!(!as_bool(get(&run, "profile-required-from-repeat")));
  assert!(as_bool(get(&run, "p-puck-wrapper-proof")));
  assert!(!as_bool(get(&run, "actual-host-removal-patch-authorized")));
  assert!(!as_bool(get(&run, "host-code-removal-started")));
  assert!(!as_bool(get(&run, "global-ontology-runtime")));
  assert!(!as_bool(get(&run, "gpl-family-dependencies")));
  assert!(!as_bool(get(&run, "implementation-command")));
  assert_eq!(as_i64(get(&run, "delete-ready-target-count")), 0);
}
