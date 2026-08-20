use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/macro-only-boot-fresh-p-puck-current-cut-owner.px")
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
fn fresh_puck_fixture_imports_owner_and_uses_bounded_report() {
  let run = eval_file(&fixture_path()).expect("fresh p-puck owner fixture");
  assert_eq!(
    as_str(get(&run, "proof")),
    "macro-only-boot-fresh-p-puck-current-cut-owner"
  );
  assert!(as_bool(get(&run, "imported-owner")));
  assert!(as_bool(get(&run, "used-px-owner")));
  assert!(!as_bool(get(&run, "imported-bootstrap-status")));
  assert_eq!(
    as_str(get(&run, "p-puck-proof-source")),
    "bounded p-puck pnixc preset report over latest current-cut receipt"
  );
}

#[test]
fn owner_meta_declares_puck_freshness_without_semantic_runtime_or_delete_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.lib.gate.macro-only-boot-fresh-p-puck-current-cut"
  );
  assert_eq!(
    as_str(get(meta, "constructor")),
    "validateFreshPuckCurrentCut"
  );
  assert_eq!(
    as_str(get(meta, "output-shape")),
    "fresh-p-puck-current-cut-present or Held"
  );
  assert!(as_bool(get(meta, "fresh-p-puck-after-current-cut")));
  assert!(as_bool(get(meta, "p-puck-wrapper-proof")));
  for key in [
    "p-puck-is-semantic-owner",
    "full-current-receipt-audit",
    "replay-executed",
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
  assert_eq!(as_i64(get(meta, "delete-ready-target-count")), 0);
}

#[test]
fn expected_report_command_receipt_and_counts_are_pinned() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(as_str(get(&run, "expected-report-kind")), "pnix-preset");
  assert_eq!(
    as_str(get(&run, "expected-report-name")),
    "macro-only-current-cut-target-delete-proof"
  );
  assert_eq!(as_str(get(&run, "expected-preset")), "pnixc");
  assert_eq!(as_str(get(&run, "expected-runner")), "cargo-bin");
  assert_eq!(as_str(get(&run, "expected-telemetry-source")), "p-puck");
  assert_eq!(
    as_str(get(&run, "expected-measurement")),
    "p-puck-internal-monotonic-ms"
  );
  assert_eq!(
    as_str(get(&run, "expected-audited-receipt")),
    "fixtures/tesseract-macro-legacy-probe/macro_only_target_delete_proof_receipt.px"
  );
  assert_eq!(
    as_str(get(&run, "expected-output-probe-marker")),
    "tesseract-macro-ontology-macro-only-target-specific-delete-proof"
  );
  assert_eq!(as_i64(get(&run, "previous-receipt-audit-count")), 38);
  assert_eq!(as_i64(get(&run, "current-tesseract-receipt-count")), 55);
}

#[test]
fn required_evidence_and_remaining_frontiers_are_explicit() {
  let run = eval_file(&fixture_path()).unwrap();
  let evidence = string_set(get(&run, "required-evidence"));
  for expected in [
    "p-puck-binary-present",
    "p-puck-command-exit-zero",
    "p-puck-report-present",
    "report-kind-pnix-preset",
    "preset-is-pnixc",
    "runner-is-cargo-bin",
    "telemetry-source-p-puck",
    "audited-receipt-is-current-cut",
    "output-probe-marker-matches-current-cut",
    "upstream-pnixc-exit-zero",
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
  assert!(frontiers.contains("bounded-replay-execution-proof-after-runner-ready"));
  assert!(frontiers.contains("macro-only-boot-execution-proof-after-bounded-replay"));
  assert!(frontiers.contains("host-code-removal-execution-proof-after-successful-boot"));
  assert_eq!(frontiers.len(), 3);
}

#[test]
fn valid_proof_records_fresh_puck_but_not_full_receipt_audit_or_boot() {
  let run = eval_file(&fixture_path()).unwrap();
  let valid = get(&run, "valid-proof");
  assert_eq!(
    as_str(get(valid, "status")),
    "fresh-p-puck-current-cut-present"
  );
  assert_eq!(as_str(get(valid, "fresh-p-puck-proof-status")), "present");
  assert!(matches!(get(valid, "held-id"), Value::Null));
  assert!(as_bool(get(valid, "fresh-p-puck-after-current-cut")));
  assert!(as_bool(get(valid, "p-puck-wrapper-proof")));
  assert!(!as_bool(get(valid, "p-puck-is-semantic-owner")));
  assert!(!as_bool(get(valid, "full-current-receipt-audit")));
  assert_eq!(as_i64(get(valid, "duration-ms")), 701);
  assert_eq!(as_i64(get(valid, "slow-threshold-ms")), 5000);
  assert_eq!(as_str(get(valid, "slow-path-status")), "within-threshold");
  for key in [
    "replay-executed",
    "boot-executed",
    "macro-only-runtime-owner-booted",
    "new-engine-from-zero",
    "runtime-install",
    "global-ontology-runtime",
    "host-code-removal-started",
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
fn stale_wrong_missing_report_preset_telemetry_receipt_and_duration_are_held() {
  let run = eval_file(&fixture_path()).unwrap();
  for (key, held_id) in [
    (
      "missing-evidence",
      "held.macro-only-fresh-p-puck.missing-required-evidence",
    ),
    (
      "stale-stage",
      "held.macro-only-fresh-p-puck.stale-current-stage",
    ),
    (
      "wrong-proof",
      "held.macro-only-fresh-p-puck.proof-id-mismatch",
    ),
    (
      "report-mismatch",
      "held.macro-only-fresh-p-puck.report-mismatch",
    ),
    (
      "preset-mismatch",
      "held.macro-only-fresh-p-puck.preset-or-runner-mismatch",
    ),
    (
      "telemetry-mismatch",
      "held.macro-only-fresh-p-puck.telemetry-mismatch",
    ),
    (
      "receipt-mismatch",
      "held.macro-only-fresh-p-puck.current-cut-receipt-mismatch",
    ),
    (
      "telemetry-missing",
      "held.macro-only-fresh-p-puck.telemetry-missing",
    ),
  ] {
    let output = get(&run, key);
    assert_eq!(as_str(get(output, "status")), "Held");
    assert_eq!(as_str(get(output, "held-id")), held_id);
    assert!(!as_bool(get(output, "fresh-p-puck-after-current-cut")));
  }
}

#[test]
fn proof_blocks_full_audit_boot_host_semantic_and_gpl_overclaims() {
  let run = eval_file(&fixture_path()).unwrap();
  for (key, held_id) in [
    (
      "full-audit-overclaim",
      "held.macro-only-fresh-p-puck.full-audit-overclaim",
    ),
    ("boot-claim", "held.macro-only-fresh-p-puck.boot-claim"),
    (
      "host-removal-claim",
      "held.macro-only-fresh-p-puck.host-removal-claim",
    ),
    (
      "semantic-owner-claim",
      "held.macro-only-fresh-p-puck.semantic-owner-claim",
    ),
    (
      "gpl-claim",
      "held.macro-only-fresh-p-puck.gpl-family-dependency",
    ),
  ] {
    let output = get(&run, key);
    assert_eq!(as_str(get(output, "status")), "Held");
    assert_eq!(as_str(get(output, "held-id")), held_id);
    assert!(!as_bool(get(output, "fresh-p-puck-after-current-cut")));
  }
}

#[test]
fn top_level_state_records_fresh_puck_without_replay_boot_or_delete() {
  let run = eval_file(&fixture_path()).unwrap();
  assert!(as_bool(get(&run, "fresh-p-puck-after-current-cut")));
  assert!(as_bool(get(&run, "p-puck-wrapper-proof")));
  assert!(!as_bool(get(&run, "full-current-receipt-audit")));
  assert!(!as_bool(get(&run, "replay-executed")));
  assert!(!as_bool(get(&run, "boot-executed")));
  assert!(!as_bool(get(&run, "macro-only-runtime-owner-booted")));
  assert!(!as_bool(get(&run, "new-engine-from-zero")));
  assert!(!as_bool(get(&run, "runtime-install")));
  assert!(!as_bool(get(&run, "global-ontology-runtime")));
  assert!(!as_bool(get(&run, "host-code-removal-started")));
  assert_eq!(as_i64(get(&run, "delete-ready-target-count")), 0);
  assert!(!as_bool(get(&run, "gpl-family-dependencies")));
  assert!(!as_bool(get(&run, "implementation-command")));
}
