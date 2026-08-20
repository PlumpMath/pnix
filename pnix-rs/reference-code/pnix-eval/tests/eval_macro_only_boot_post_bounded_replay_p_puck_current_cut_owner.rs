use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/macro-only-boot-post-bounded-replay-p-puck-current-cut-owner.px")
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
fn fixture_imports_owner_and_uses_actual_p_puck_report() {
  let run = eval_file(&fixture_path()).expect("post replay p-puck owner fixture");
  assert_eq!(
    as_str(get(&run, "proof")),
    "macro-only-boot-post-bounded-replay-p-puck-current-cut-owner"
  );
  assert!(as_bool(get(&run, "imported-owner")));
  assert!(as_bool(get(&run, "used-px-owner")));
  assert_eq!(
    as_str(get(&run, "imported-bounded-replay")),
    "literal-bounded-replay-executed-input"
  );
  assert_eq!(
    as_str(get(&run, "p-puck-proof-source")),
    "actual p-puck pnixc report over bounded replay receipt"
  );
}

#[test]
fn owner_meta_declares_puck_freshness_without_boot_or_semantic_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.lib.gate.macro-only-boot-post-bounded-replay-p-puck-current-cut"
  );
  assert_eq!(
    as_str(get(meta, "constructor")),
    "validatePostReplayPuckCurrentCut"
  );
  assert_eq!(
    as_str(get(meta, "output-shape")),
    "post-bounded-replay-p-puck-current-cut-present or Held"
  );
  assert!(as_bool(get(meta, "post-bounded-replay-p-puck-current-cut")));
  assert!(as_bool(get(meta, "bounded-replay-executed-input")));
  assert!(as_bool(get(meta, "p-puck-wrapper-proof")));
  for key in [
    "p-puck-is-semantic-owner",
    "full-current-receipt-audit",
    "boot-executed",
    "macro-only-runtime-owner-booted",
    "new-engine-from-zero",
    "runtime-install",
    "global-ontology-runtime",
    "host-code-removal-started",
    "semantic-owner",
    "old-host-authority",
    "gpl-family-dependencies",
  ] {
    assert!(!as_bool(get(meta, key)), "`{key}` must stay false");
  }
}

#[test]
fn expected_report_command_receipt_and_telemetry_are_pinned() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(as_str(get(&run, "expected-report-kind")), "pnix-preset");
  assert_eq!(
    as_str(get(&run, "expected-report-name")),
    "macro-only-current-cut-bounded-replay"
  );
  assert_eq!(as_str(get(&run, "expected-status")), "ok");
  assert_eq!(as_str(get(&run, "expected-preset")), "pnixc");
  assert_eq!(as_str(get(&run, "expected-runner")), "cargo-bin");
  assert_eq!(as_str(get(&run, "expected-telemetry-source")), "p-puck");
  assert_eq!(
    as_str(get(&run, "expected-measurement")),
    "p-puck-internal-monotonic-ms"
  );
  assert_eq!(
    as_str(get(&run, "expected-audited-receipt")),
    "fixtures/tesseract-macro-legacy-probe/macro_only_bounded_replay_execution_receipt.px"
  );
  assert_eq!(
    as_str(get(&run, "expected-output-probe-marker")),
    "tesseract-macro-ontology-macro-only-bounded-replay-execution"
  );
  assert_eq!(as_i64(get(&run, "slow-threshold-ms")), 5000);
}

#[test]
fn valid_proof_records_post_replay_puck_without_full_audit_or_boot() {
  let run = eval_file(&fixture_path()).unwrap();
  let valid = get(&run, "valid-proof");
  assert_eq!(
    as_str(get(valid, "status")),
    "post-bounded-replay-p-puck-current-cut-present"
  );
  assert_eq!(
    as_str(get(valid, "post-replay-p-puck-proof-status")),
    "present"
  );
  assert!(matches!(get(valid, "held-id"), Value::Null));
  assert!(as_bool(get(
    valid,
    "post-bounded-replay-p-puck-current-cut"
  )));
  assert!(as_bool(get(valid, "bounded-replay-executed-input")));
  assert!(as_bool(get(valid, "p-puck-wrapper-proof")));
  assert_eq!(as_i64(get(valid, "duration-ms")), 4934);
  assert_eq!(as_i64(get(valid, "slow-threshold-ms")), 5000);
  assert_eq!(as_str(get(valid, "slow-path-status")), "within-threshold");
  for key in [
    "full-current-receipt-audit",
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
fn required_evidence_and_remaining_frontiers_are_explicit() {
  let run = eval_file(&fixture_path()).unwrap();
  let evidence = string_set(get(&run, "required-evidence"));
  for expected in [
    "bounded-replay-executed",
    "bounded-replay-execution-proof-present",
    "p-puck-command-exit-zero",
    "p-puck-report-present",
    "audited-receipt-is-current-cut",
    "output-probe-marker-matches-current-cut",
    "duration-ms-recorded",
    "full-current-receipt-audit-false-recorded",
    "semantic-owner-false-recorded",
    "boot-executed-false-recorded",
    "host-code-removal-started-false-recorded",
    "gpl-family-dependencies-false",
  ] {
    assert!(evidence.contains(expected), "missing evidence `{expected}`");
  }

  let frontiers = string_set(get(&run, "remaining-open-frontiers"));
  assert!(frontiers.contains("full-current-receipt-audit-after-bounded-replay"));
  assert!(frontiers.contains("macro-only-boot-execution-proof-after-post-replay-p-puck"));
  assert!(frontiers.contains("host-code-removal-execution-proof-after-successful-boot"));
  assert_eq!(frontiers.len(), 3);
}

#[test]
fn stale_wrong_missing_report_telemetry_and_receipt_cases_are_held() {
  let run = eval_file(&fixture_path()).unwrap();
  for (key, held_id) in [
    (
      "missing-evidence",
      "held.macro-only-post-replay-p-puck.missing-required-evidence",
    ),
    (
      "wrong-proof",
      "held.macro-only-post-replay-p-puck.proof-id-mismatch",
    ),
    (
      "stale-stage",
      "held.macro-only-post-replay-p-puck.stale-current-stage",
    ),
    (
      "bounded-replay-missing",
      "held.macro-only-post-replay-p-puck.bounded-replay-missing",
    ),
    (
      "report-mismatch",
      "held.macro-only-post-replay-p-puck.report-mismatch",
    ),
    (
      "preset-mismatch",
      "held.macro-only-post-replay-p-puck.preset-or-runner-mismatch",
    ),
    (
      "telemetry-mismatch",
      "held.macro-only-post-replay-p-puck.telemetry-mismatch",
    ),
    (
      "receipt-mismatch",
      "held.macro-only-post-replay-p-puck.current-cut-receipt-mismatch",
    ),
    (
      "telemetry-missing",
      "held.macro-only-post-replay-p-puck.telemetry-missing",
    ),
  ] {
    let output = get(&run, key);
    assert_eq!(as_str(get(output, "status")), "Held");
    assert_eq!(as_str(get(output, "held-id")), held_id);
    assert!(!as_bool(get(
      output,
      "post-bounded-replay-p-puck-current-cut"
    )));
  }
}

#[test]
fn proof_blocks_full_audit_boot_host_semantic_and_gpl_overclaims() {
  let run = eval_file(&fixture_path()).unwrap();
  for (key, held_id) in [
    (
      "full-audit-overclaim",
      "held.macro-only-post-replay-p-puck.full-audit-overclaim",
    ),
    (
      "boot-claim",
      "held.macro-only-post-replay-p-puck.boot-or-runtime-claim",
    ),
    (
      "host-removal-claim",
      "held.macro-only-post-replay-p-puck.host-removal-claim",
    ),
    (
      "semantic-owner-claim",
      "held.macro-only-post-replay-p-puck.semantic-owner-claim",
    ),
    (
      "gpl-claim",
      "held.macro-only-post-replay-p-puck.gpl-family-dependency",
    ),
  ] {
    let output = get(&run, key);
    assert_eq!(as_str(get(output, "status")), "Held");
    assert_eq!(as_str(get(output, "held-id")), held_id);
    assert!(!as_bool(get(output, "p-puck-wrapper-proof")));
  }
}
