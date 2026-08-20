use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/macro-only-boot-full-current-receipt-audit-owner.px")
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
fn fixture_imports_owner_and_post_replay_puck_input() {
  let run = eval_file(&fixture_path()).expect("full current receipt audit owner fixture");
  assert_eq!(
    as_str(get(&run, "proof")),
    "macro-only-boot-full-current-receipt-audit-owner"
  );
  assert!(as_bool(get(&run, "imported-owner")));
  assert!(as_bool(get(&run, "used-px-owner")));
  assert!(as_bool(get(&run, "imported-post-replay-puck")));
  assert_eq!(
    as_str(get(&run, "post-replay-puck-proof-status")),
    "post-bounded-replay-p-puck-current-cut-present"
  );
}

#[test]
fn owner_meta_declares_full_audit_without_runtime_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.lib.gate.macro-only-boot-full-current-receipt-audit"
  );
  assert_eq!(
    as_str(get(meta, "constructor")),
    "validateFullCurrentReceiptAudit"
  );
  assert!(as_bool(get(meta, "full-current-receipt-audit")));
  assert!(as_bool(get(
    meta,
    "current-receipt-audit-after-bounded-replay"
  )));
  assert!(as_bool(get(meta, "compare-all-proof")));
  assert!(as_bool(get(meta, "wiki-map-smoke-proof")));
  assert!(as_bool(get(meta, "diff-check-proof")));
  assert!(as_bool(get(
    meta,
    "post-bounded-replay-p-puck-current-cut-input"
  )));
  for key in [
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
    assert!(!as_bool(get(meta, key)), "`{key}` must stay false");
  }
}

#[test]
fn expected_commands_counts_and_telemetry_are_pinned() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "expected-compare-command")),
    "bash scripts/tesseract-macro-ontology-compare.sh --all"
  );
  assert_eq!(
    as_str(get(&run, "expected-focused-command")),
    "bash scripts/tesseract-macro-ontology-compare.sh --macro-only-boot-post-replay-puck"
  );
  assert_eq!(
    as_str(get(&run, "expected-smoke-command")),
    "bash scripts/check-project-wiki-map-smoke.sh"
  );
  assert_eq!(
    as_str(get(&run, "expected-diff-check-command")),
    "git diff --check"
  );
  assert_eq!(as_i64(get(&run, "expected-total-tests")), 915);
  assert_eq!(as_i64(get(&run, "expected-focused-tests")), 16);
  assert_eq!(as_i64(get(&run, "expected-source-tracked")), 18167);
  assert_eq!(as_i64(get(&run, "expected-source-indexed")), 18167);
  assert_eq!(
    as_str(get(&run, "expected-puck-report-name")),
    "macro-only-current-cut-bounded-replay"
  );
  assert_eq!(as_i64(get(&run, "expected-puck-duration-ms")), 4934);
}

#[test]
fn valid_proof_closes_full_current_receipt_audit_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let valid = get(&run, "valid-proof");
  assert_eq!(
    as_str(get(valid, "status")),
    "full-current-receipt-audit-present"
  );
  assert_eq!(
    as_str(get(valid, "full-current-receipt-audit-status")),
    "present"
  );
  assert!(matches!(get(valid, "held-id"), Value::Null));
  assert!(as_bool(get(valid, "full-current-receipt-audit")));
  assert!(as_bool(get(
    valid,
    "current-receipt-audit-after-bounded-replay"
  )));
  assert!(as_bool(get(valid, "compare-all-proof")));
  assert!(as_bool(get(valid, "wiki-map-smoke-proof")));
  assert!(as_bool(get(valid, "diff-check-proof")));
  assert_eq!(as_i64(get(valid, "total-tests")), 915);
  assert_eq!(as_i64(get(valid, "source-tracked")), 18167);
  assert_eq!(as_i64(get(valid, "source-indexed")), 18167);
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
fn required_evidence_and_remaining_frontiers_are_explicit() {
  let run = eval_file(&fixture_path()).unwrap();
  let evidence = string_set(get(&run, "required-evidence"));
  for expected in [
    "post-bounded-replay-p-puck-current-cut-present",
    "bounded-replay-executed",
    "compare-all-total-tests-915",
    "focused-post-replay-puck-mode-ok",
    "wiki-map-smoke-ok",
    "source-inventory-tracked-equals-indexed",
    "git-diff-check-ok",
    "boot-executed-false-recorded",
    "semantic-owner-false-recorded",
    "host-code-removal-started-false-recorded",
    "gpl-family-dependencies-false",
  ] {
    assert!(evidence.contains(expected), "missing evidence `{expected}`");
  }

  let frontiers = string_set(get(&run, "remaining-open-frontiers"));
  assert!(frontiers.contains("macro-only-boot-execution-proof-after-full-current-receipt-audit"));
  assert!(frontiers.contains("host-code-removal-execution-proof-after-successful-boot"));
  assert_eq!(frontiers.len(), 2);
}

#[test]
fn stale_wrong_missing_and_measurement_mismatches_are_held() {
  let run = eval_file(&fixture_path()).unwrap();
  for (key, held_id) in [
    (
      "missing-evidence",
      "held.macro-only-full-current-audit.missing-required-evidence",
    ),
    (
      "wrong-proof",
      "held.macro-only-full-current-audit.proof-id-mismatch",
    ),
    (
      "stale-stage",
      "held.macro-only-full-current-audit.stale-current-stage",
    ),
    (
      "post-replay-puck-missing",
      "held.macro-only-full-current-audit.post-replay-puck-missing",
    ),
    (
      "puck-report-mismatch",
      "held.macro-only-full-current-audit.p-puck-report-mismatch",
    ),
    (
      "compare-mismatch",
      "held.macro-only-full-current-audit.compare-all-mismatch",
    ),
    (
      "focused-mismatch",
      "held.macro-only-full-current-audit.focused-mode-mismatch",
    ),
    (
      "smoke-mismatch",
      "held.macro-only-full-current-audit.wiki-smoke-mismatch",
    ),
    (
      "diff-check-missing",
      "held.macro-only-full-current-audit.diff-check-missing",
    ),
  ] {
    let output = get(&run, key);
    assert_eq!(as_str(get(output, "status")), "Held");
    assert_eq!(as_str(get(output, "held-id")), held_id);
    assert!(!as_bool(get(output, "full-current-receipt-audit")));
  }
}

#[test]
fn audit_blocks_boot_host_semantic_and_gpl_overclaims() {
  let run = eval_file(&fixture_path()).unwrap();
  for (key, held_id) in [
    (
      "boot-claim",
      "held.macro-only-full-current-audit.boot-or-runtime-claim",
    ),
    (
      "host-removal-claim",
      "held.macro-only-full-current-audit.host-removal-claim",
    ),
    (
      "semantic-owner-claim",
      "held.macro-only-full-current-audit.semantic-owner-claim",
    ),
    (
      "gpl-claim",
      "held.macro-only-full-current-audit.gpl-family-dependency",
    ),
  ] {
    let output = get(&run, key);
    assert_eq!(as_str(get(output, "status")), "Held");
    assert_eq!(as_str(get(output, "held-id")), held_id);
    assert!(!as_bool(get(output, "full-current-receipt-audit")));
    assert!(!as_bool(get(output, "boot-executed")));
    assert!(!as_bool(get(output, "host-code-removal-started")));
    assert!(!as_bool(get(output, "semantic-owner")));
  }
}
