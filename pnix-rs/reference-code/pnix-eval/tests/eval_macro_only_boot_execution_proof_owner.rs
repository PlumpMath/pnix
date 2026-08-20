use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/macro-only-boot-execution-proof-owner.px")
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
fn fixture_imports_owner_full_audit_and_post_replay_puck() {
  let run = eval_file(&fixture_path()).expect("macro-only boot execution proof owner fixture");
  assert_eq!(
    as_str(get(&run, "proof")),
    "macro-only-boot-execution-proof-owner"
  );
  assert!(as_bool(get(&run, "imported-owner")));
  assert!(as_bool(get(&run, "used-px-owner")));
  assert!(as_bool(get(&run, "imported-full-current-receipt-audit")));
  assert!(as_bool(get(&run, "imported-post-replay-puck")));
}

#[test]
fn owner_meta_allows_boot_proof_without_runtime_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.lib.gate.macro-only-boot-execution-proof"
  );
  assert_eq!(
    as_str(get(meta, "constructor")),
    "validateBootExecutionProof"
  );
  assert!(as_bool(get(meta, "boot-execution-proof")));
  assert!(as_bool(get(meta, "may-emit-boot-executed")));
  assert!(as_bool(get(meta, "full-current-receipt-audit-required")));
  for key in [
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
fn expected_counts_telemetry_and_stage_are_pinned() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "expected-proof-id")),
    "proof.macro-only-boot.execution-after-full-current-audit.v1"
  );
  assert_eq!(
    as_str(get(&run, "expected-current-stage")),
    "full-current-receipt-audit-present"
  );
  assert_eq!(
    as_str(get(&run, "expected-runner-status")),
    "runner-ready-for-bounded-replay"
  );
  assert_eq!(
    as_str(get(&run, "expected-semantic-delta-status")),
    "empty-or-held-only"
  );
  assert_eq!(
    as_str(get(&run, "expected-compare-command")),
    "bash scripts/tesseract-macro-ontology-compare.sh --all"
  );
  assert_eq!(as_i64(get(&run, "expected-total-tests")), 931);
  assert_eq!(as_i64(get(&run, "expected-source-tracked")), 18172);
  assert_eq!(as_i64(get(&run, "expected-source-indexed")), 18172);
  assert_eq!(
    as_str(get(&run, "expected-puck-report-name")),
    "macro-only-current-cut-bounded-replay"
  );
  assert_eq!(as_i64(get(&run, "expected-puck-duration-ms")), 4934);
}

#[test]
fn valid_proof_sets_boot_executed_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let valid = get(&run, "valid-proof");
  assert_eq!(
    as_str(get(valid, "status")),
    "macro-only-boot-execution-proof-present"
  );
  assert_eq!(
    as_str(get(valid, "macro-only-boot-execution-proof-status")),
    "present"
  );
  assert!(matches!(get(valid, "held-id"), Value::Null));
  assert!(as_bool(get(valid, "boot-execution-proof")));
  assert!(as_bool(get(valid, "boot-executed")));
  assert!(as_bool(get(valid, "full-current-receipt-audit-input")));
  assert!(as_bool(get(valid, "bounded-replay-input")));
  assert!(as_bool(get(valid, "post-replay-p-puck-input")));
  assert_eq!(as_i64(get(valid, "total-tests")), 931);
  assert_eq!(as_i64(get(valid, "source-tracked")), 18172);
  assert_eq!(as_i64(get(valid, "source-indexed")), 18172);
  assert_eq!(as_i64(get(valid, "p-puck-duration-ms")), 4934);
  for key in [
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
    "full-current-receipt-audit-present",
    "bounded-replay-executed",
    "post-bounded-replay-p-puck-current-cut-present",
    "runner-ready-for-bounded-replay",
    "compare-all-931-ok",
    "source-inventory-tracked-equals-indexed",
    "p-puck-current-cut-within-threshold",
    "negative-held-retained",
    "semantic-delta-empty-or-held",
    "runtime-owner-false-before-proof",
    "host-code-removal-started-false-before-proof",
    "gpl-family-dependencies-false",
  ] {
    assert!(evidence.contains(expected), "missing evidence `{expected}`");
  }

  let frontiers = string_set(get(&run, "remaining-open-frontiers"));
  assert!(frontiers.contains("macro-only-runtime-owner-proof-after-boot-execution"));
  assert!(frontiers.contains("host-code-removal-execution-proof-after-successful-boot"));
  assert!(frontiers.contains("semantic-owner-proof-after-runtime-owner"));
  assert_eq!(frontiers.len(), 3);
}

#[test]
fn wrong_stale_missing_and_measurement_mismatches_are_held() {
  let run = eval_file(&fixture_path()).unwrap();
  for (key, held_id) in [
    (
      "missing-evidence",
      "held.macro-only-boot-proof.semantic-delta-or-held-loss",
    ),
    (
      "wrong-proof",
      "held.macro-only-boot-proof.proof-id-mismatch",
    ),
    (
      "stale-stage",
      "held.macro-only-boot-proof.stale-current-stage",
    ),
    (
      "full-audit-missing",
      "held.macro-only-boot-proof.full-audit-missing",
    ),
    (
      "replay-puck-missing",
      "held.macro-only-boot-proof.replay-or-puck-missing",
    ),
    (
      "runner-not-ready",
      "held.macro-only-boot-proof.runner-not-ready",
    ),
    (
      "compare-mismatch",
      "held.macro-only-boot-proof.compare-all-mismatch",
    ),
    (
      "source-parity-mismatch",
      "held.macro-only-boot-proof.source-parity-mismatch",
    ),
    (
      "puck-telemetry-mismatch",
      "held.macro-only-boot-proof.p-puck-telemetry-mismatch",
    ),
    (
      "semantic-delta-overclaim",
      "held.macro-only-boot-proof.semantic-delta-or-held-loss",
    ),
  ] {
    let output = get(&run, key);
    assert_eq!(as_str(get(output, "status")), "Held");
    assert_eq!(as_str(get(output, "held-id")), held_id);
    assert!(!as_bool(get(output, "boot-executed")));
  }
}

#[test]
fn boot_proof_blocks_runtime_host_semantic_old_host_and_gpl_overclaims() {
  let run = eval_file(&fixture_path()).unwrap();
  for (key, held_id) in [
    (
      "runtime-owner-claim",
      "held.macro-only-boot-proof.runtime-owner-overclaim",
    ),
    (
      "host-removal-claim",
      "held.macro-only-boot-proof.host-removal-overclaim",
    ),
    (
      "semantic-owner-claim",
      "held.macro-only-boot-proof.semantic-owner-overclaim",
    ),
    (
      "old-host-authority",
      "held.macro-only-boot-proof.old-host-authority",
    ),
    (
      "gpl-claim",
      "held.macro-only-boot-proof.gpl-family-dependency",
    ),
  ] {
    let output = get(&run, key);
    assert_eq!(as_str(get(output, "status")), "Held");
    assert_eq!(as_str(get(output, "held-id")), held_id);
    assert!(!as_bool(get(output, "boot-executed")));
    assert!(!as_bool(get(output, "macro-only-runtime-owner-booted")));
    assert!(!as_bool(get(output, "host-code-removal-started")));
    assert!(!as_bool(get(output, "semantic-owner")));
  }
}
