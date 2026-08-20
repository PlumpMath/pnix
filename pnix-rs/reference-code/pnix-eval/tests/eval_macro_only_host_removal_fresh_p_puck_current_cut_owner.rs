use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/pnix-query-runtime/macro-only-host-removal-fresh-p-puck-current-cut-owner.px",
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
fn fixture_imports_owner_and_uses_actual_host_removal_p_puck_report() {
  let run = eval_file(&fixture_path()).expect("host removal fresh p-puck owner fixture");
  assert_eq!(
    as_str(get(&run, "proof")),
    "macro-only-host-removal-fresh-p-puck-current-cut-owner"
  );
  assert!(as_bool(get(&run, "imported-owner")));
  assert!(as_bool(get(&run, "used-px-owner")));
  assert_eq!(
    as_str(get(&run, "imported-host-removal-execution")),
    "literal-host-removal-execution-proof-input"
  );
  assert_eq!(
    as_str(get(&run, "p-puck-proof-source")),
    "actual p-puck pnixc report over host-removal execution proof receipt"
  );
}

#[test]
fn owner_meta_declares_freshness_and_slow_path_without_delete_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.lib.gate.macro-only-host-removal-fresh-p-puck-current-cut"
  );
  assert_eq!(
    as_str(get(meta, "constructor")),
    "validateHostRemovalFreshPuckCurrentCut"
  );
  assert!(as_bool(get(meta, "host-removal-fresh-p-puck-current-cut")));
  assert!(as_bool(get(
    meta,
    "fresh-puck-before-host-removal-execution"
  )));
  assert!(as_bool(get(meta, "p-puck-wrapper-proof")));
  assert!(as_bool(get(meta, "slow-path-candidate")));
  assert!(as_bool(get(meta, "self-optimization-candidate")));
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
fn expected_report_command_receipt_and_slow_telemetry_are_pinned() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "expected-report-name")),
    "macro-only-current-cut-host-removal-execution-proof"
  );
  assert_eq!(as_str(get(&run, "expected-status")), "ok");
  assert_eq!(as_str(get(&run, "expected-preset")), "pnixc");
  assert_eq!(as_str(get(&run, "expected-runner")), "cargo-bin");
  assert_eq!(
    as_str(get(&run, "expected-audited-receipt")),
    "fixtures/tesseract-macro-legacy-probe/macro_only_host_removal_execution_proof_receipt.px"
  );
  assert_eq!(
    as_str(get(&run, "expected-output-probe-marker")),
    "tesseract-macro-ontology-macro-only-host-removal-execution-proof"
  );
  assert_eq!(as_i64(get(&run, "expected-duration-ms")), 5389);
  assert_eq!(as_i64(get(&run, "slow-threshold-ms")), 5000);
  assert_eq!(
    as_str(get(&run, "expected-slow-path-status")),
    "slow-path-candidate"
  );
}

#[test]
fn valid_proof_records_fresh_puck_but_blocks_delete_and_runtime() {
  let run = eval_file(&fixture_path()).unwrap();
  let valid = get(&run, "valid-proof");
  assert_eq!(
    as_str(get(valid, "status")),
    "host-removal-fresh-p-puck-current-cut-present"
  );
  assert!(matches!(get(valid, "held-id"), Value::Null));
  assert!(as_bool(get(valid, "host-removal-fresh-p-puck-current-cut")));
  assert!(as_bool(get(
    valid,
    "fresh-puck-before-host-removal-execution"
  )));
  assert!(as_bool(get(valid, "host-removal-execution-proof-input")));
  assert!(as_bool(get(valid, "p-puck-wrapper-proof")));
  assert!(as_bool(get(valid, "slow-path-candidate")));
  assert!(as_bool(get(valid, "self-optimization-candidate")));
  assert_eq!(as_i64(get(valid, "duration-ms")), 5389);
  assert_eq!(as_i64(get(valid, "slow-threshold-ms")), 5000);
  assert_eq!(
    as_str(get(valid, "slow-path-status")),
    "slow-path-candidate"
  );
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
fn required_evidence_and_new_frontiers_are_explicit() {
  let run = eval_file(&fixture_path()).unwrap();
  let evidence = string_set(get(&run, "required-evidence"));
  for expected in [
    "host-removal-execution-proof-present",
    "p-puck-report-present",
    "audited-receipt-is-host-removal-execution-proof",
    "output-probe-marker-matches-host-removal-execution-proof",
    "slow-path-candidate-recorded",
    "actual-delete-patch-false-recorded",
    "gpl-family-dependencies-false",
  ] {
    assert!(evidence.contains(expected), "missing evidence `{expected}`");
  }

  let frontiers = string_set(get(&run, "remaining-open-frontiers"));
  assert!(frontiers.contains("host-removal-slow-path-repeat-or-profile-before-delete"));
  assert!(frontiers.contains("actual-host-removal-patch-after-fresh-puck"));
  assert!(frontiers.contains("global-runtime-install-proof-after-semantic-owner"));
  assert_eq!(frontiers.len(), 5);
}

#[test]
fn stale_wrong_missing_report_and_telemetry_cases_are_held() {
  let run = eval_file(&fixture_path()).unwrap();
  for (key, held_id) in [
    (
      "wrong-proof",
      "held.macro-only-host-removal-fresh-puck.proof-id-mismatch",
    ),
    (
      "stale-stage",
      "held.macro-only-host-removal-fresh-puck.stale-current-stage",
    ),
    (
      "execution-proof-missing",
      "held.macro-only-host-removal-fresh-puck.execution-proof-missing",
    ),
    (
      "report-mismatch",
      "held.macro-only-host-removal-fresh-puck.report-mismatch",
    ),
    (
      "telemetry-mismatch",
      "held.macro-only-host-removal-fresh-puck.telemetry-mismatch",
    ),
    (
      "receipt-mismatch",
      "held.macro-only-host-removal-fresh-puck.current-cut-receipt-mismatch",
    ),
    (
      "telemetry-drift",
      "held.macro-only-host-removal-fresh-puck.telemetry-missing-or-drifted",
    ),
    (
      "missing-evidence",
      "held.macro-only-host-removal-fresh-puck.missing-required-evidence",
    ),
  ] {
    let output = get(&run, key);
    assert_eq!(as_str(get(output, "status")), "Held");
    assert_eq!(as_str(get(output, "held-id")), held_id);
    assert!(!as_bool(get(
      output,
      "host-removal-fresh-p-puck-current-cut"
    )));
  }
}

#[test]
fn delete_runtime_semantic_old_host_and_gpl_overclaims_are_held() {
  let run = eval_file(&fixture_path()).unwrap();
  for (key, held_id) in [
    (
      "delete-claim",
      "held.macro-only-host-removal-fresh-puck.delete-overclaim",
    ),
    (
      "runtime-claim",
      "held.macro-only-host-removal-fresh-puck.runtime-overclaim",
    ),
    (
      "semantic-owner-claim",
      "held.macro-only-host-removal-fresh-puck.semantic-owner-claim",
    ),
    (
      "old-host-authority",
      "held.macro-only-host-removal-fresh-puck.old-host-authority",
    ),
    (
      "gpl-claim",
      "held.macro-only-host-removal-fresh-puck.gpl-family-dependency",
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
fn top_level_state_records_candidate_only_freshness() {
  let run = eval_file(&fixture_path()).unwrap();
  assert!(as_bool(get(&run, "host-removal-fresh-p-puck-current-cut")));
  assert!(as_bool(get(
    &run,
    "fresh-puck-before-host-removal-execution"
  )));
  assert!(as_bool(get(&run, "p-puck-wrapper-proof")));
  assert!(as_bool(get(&run, "slow-path-candidate")));
  assert!(as_bool(get(&run, "self-optimization-candidate")));
  assert!(!as_bool(get(&run, "actual-host-removal-patch-authorized")));
  assert!(!as_bool(get(&run, "host-code-removal-started")));
  assert!(!as_bool(get(&run, "global-ontology-runtime")));
  assert!(!as_bool(get(&run, "gpl-family-dependencies")));
  assert!(!as_bool(get(&run, "implementation-command")));
  assert_eq!(as_i64(get(&run, "delete-ready-target-count")), 0);
}
