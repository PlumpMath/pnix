use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/macro-only-boot-compare-after-boot-owner.px")
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
fn compare_fixture_imports_owner_and_uses_recorded_compare_evidence() {
  let run = eval_file(&fixture_path()).expect("macro-only boot compare owner fixture must eval");
  assert_eq!(
    as_str(get(&run, "proof")),
    "macro-only-boot-compare-after-boot-owner"
  );
  assert!(as_bool(get(&run, "imported-owner")));
  assert!(as_bool(get(&run, "used-px-owner")));
  assert!(!as_bool(get(&run, "imported-bootstrap-status")));
  assert_eq!(
    as_str(get(&run, "bootstrap-status-source")),
    "shallow-status-snapshot"
  );
  assert!(!as_bool(get(&run, "imported-compare-log")));
  assert_eq!(
    as_str(get(&run, "compare-log-source")),
    "recorded-runner-evidence"
  );
}

#[test]
fn owner_meta_declares_compare_evidence_without_runtime_or_semantic_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.lib.gate.macro-only-boot-compare-after-boot"
  );
  assert_eq!(as_str(get(meta, "constructor")), "validateCompareAfterBoot");
  assert_eq!(
    as_str(get(meta, "output-shape")),
    "compare-after-boot-present or Held"
  );
  assert!(as_bool(get(meta, "compare-after-boot")));
  assert_eq!(as_i64(get(meta, "expected-total-tests")), 799);
  for key in [
    "fresh-p-puck-after-current-cut",
    "replay-executed",
    "boot-executed",
    "macro-only-runtime-owner-booted",
    "new-engine-from-zero",
    "runtime-install",
    "global-ontology-runtime",
    "host-code-removal-started",
    "compare-owned-by-boot-proof",
    "p-puck-owned-by-compare",
    "semantic-owner",
  ] {
    assert!(!as_bool(get(meta, key)), "`{key}` must stay false");
  }
}

#[test]
fn required_evidence_records_compare_command_count_status_and_remaining_frontiers() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "expected-current-stage")),
    "macro-only-bootstrap-audit-update-present"
  );
  assert_eq!(
    as_str(get(&run, "expected-command")),
    "bash scripts/tesseract-macro-ontology-compare.sh --all"
  );
  assert_eq!(as_i64(get(&run, "expected-total-tests")), 799);

  let evidence = string_set(get(&run, "required-evidence"));
  for expected in [
    "bounded-replay-strategy-present",
    "regression-corpus-transfer-present",
    "bootstrap-status-audit-update-plan-present",
    "runner-missing-vector-after-audit-recorded",
    "compare-current-stage-recorded",
    "compare-run-after-audit-update-recorded",
    "compare-command-recorded",
    "compare-total-tests-recorded",
    "compare-status-ok",
    "fresh-p-puck-still-false-recorded",
    "boot-executed-false-recorded",
    "macro-only-runtime-owner-booted-false-recorded",
    "new-engine-from-zero-false-recorded",
    "host-code-removal-started-false-recorded",
    "delete-ready-target-count-zero",
    "gpl-family-dependencies-false",
  ] {
    assert!(evidence.contains(expected), "missing evidence `{expected}`");
  }
  assert_eq!(evidence.len(), 16);

  let frontiers = string_set(get(&run, "required-open-frontiers"));
  assert!(frontiers.contains("fresh-p-puck-after-current-cut"));
  assert!(frontiers.contains("target-specific-delete-proof-present"));
  assert_eq!(frontiers.len(), 2);
}

#[test]
fn valid_compare_is_present_but_does_not_run_puck_replay_boot_or_delete() {
  let run = eval_file(&fixture_path()).unwrap();
  let valid = get(&run, "valid-compare");
  assert_eq!(as_str(get(valid, "status")), "compare-after-boot-present");
  assert_eq!(as_str(get(valid, "compare-proof-status")), "present");
  assert!(matches!(get(valid, "held-id"), Value::Null));
  assert!(as_bool(get(valid, "compare-after-boot")));
  assert_eq!(as_i64(get(valid, "total-tests")), 799);
  assert_eq!(as_list(get(valid, "missing")).len(), 0);
  for key in [
    "fresh-p-puck-after-current-cut",
    "replay-executed",
    "boot-executed",
    "macro-only-runtime-owner-booted",
    "new-engine-from-zero",
    "runtime-install",
    "global-ontology-runtime",
    "host-code-removal-started",
    "compare-owned-by-boot-proof",
    "p-puck-owned-by-compare",
    "semantic-owner",
    "implementation-command",
  ] {
    assert!(!as_bool(get(valid, key)), "`{key}` must stay false");
  }
}

#[test]
fn stale_stage_wrong_proof_command_total_and_failed_compare_are_held() {
  let run = eval_file(&fixture_path()).unwrap();
  for (key, held_id) in [
    (
      "stale-stage",
      "held.macro-only-boot-compare-after-boot.stale-current-stage",
    ),
    (
      "wrong-proof",
      "held.macro-only-boot-compare-after-boot.proof-id-mismatch",
    ),
    (
      "wrong-command",
      "held.macro-only-boot-compare-after-boot.command-mismatch",
    ),
    (
      "wrong-total",
      "held.macro-only-boot-compare-after-boot.total-tests-mismatch",
    ),
    (
      "compare-failed",
      "held.macro-only-boot-compare-after-boot.compare-not-ok",
    ),
  ] {
    let output = get(&run, key);
    assert_eq!(as_str(get(output, "status")), "Held");
    assert_eq!(as_str(get(output, "held-id")), held_id);
    assert!(!as_bool(get(output, "compare-after-boot")));
  }
}

#[test]
fn missing_frontier_and_required_evidence_are_held_before_presence() {
  let run = eval_file(&fixture_path()).unwrap();
  let missing_frontier = get(&run, "missing-frontier");
  assert_eq!(as_str(get(missing_frontier, "status")), "Held");
  assert_eq!(
    as_str(get(missing_frontier, "held-id")),
    "held.macro-only-boot-compare-after-boot.missing-required-evidence"
  );
  assert!(
    string_set(get(missing_frontier, "missing")).contains("target-specific-delete-proof-present")
  );

  let missing_evidence = get(&run, "missing-evidence");
  assert_eq!(as_str(get(missing_evidence, "status")), "Held");
  assert!(string_set(get(missing_evidence, "missing")).contains("compare-total-tests-recorded"));
}

#[test]
fn compare_proof_cannot_claim_puck_boot_semantic_owner_delete_or_gpl_dependency() {
  let run = eval_file(&fixture_path()).unwrap();
  for (key, held_id) in [
    (
      "fresh-puck-claim",
      "held.macro-only-boot-compare-after-boot.fresh-puck-claim",
    ),
    (
      "boot-claim",
      "held.macro-only-boot-compare-after-boot.boot-claim",
    ),
    (
      "semantic-owner-claim",
      "held.macro-only-boot-compare-after-boot.semantic-owner-claim",
    ),
    (
      "delete-claim",
      "held.macro-only-boot-compare-after-boot.delete-before-boot",
    ),
    (
      "gpl-claim",
      "held.macro-only-boot-compare-after-boot.gpl-family-dependency",
    ),
  ] {
    let output = get(&run, key);
    assert_eq!(as_str(get(output, "status")), "Held");
    assert_eq!(as_str(get(output, "held-id")), held_id);
    assert!(!as_bool(get(output, "compare-after-boot")));
  }
}

#[test]
fn all_outputs_preserve_no_runtime_or_host_delete_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  for key in [
    "valid-compare",
    "missing-frontier",
    "missing-evidence",
    "stale-stage",
    "wrong-proof",
    "wrong-command",
    "wrong-total",
    "compare-failed",
    "fresh-puck-claim",
    "boot-claim",
    "semantic-owner-claim",
    "delete-claim",
    "gpl-claim",
  ] {
    let value = get(&run, key);
    assert!(!as_bool(get(value, "boot-executed")), "`{key}` booted");
    assert!(
      !as_bool(get(value, "new-engine-from-zero")),
      "`{key}` claimed zero boot"
    );
    assert!(
      !as_bool(get(value, "runtime-install")),
      "`{key}` installed runtime"
    );
    assert!(
      !as_bool(get(value, "global-ontology-runtime")),
      "`{key}` claimed global runtime"
    );
    assert!(
      !as_bool(get(value, "host-code-removal-started")),
      "`{key}` removed host code"
    );
    assert!(
      !as_bool(get(value, "fresh-p-puck-after-current-cut")),
      "`{key}` claimed p-puck"
    );
    assert!(
      !as_bool(get(value, "semantic-owner")),
      "`{key}` claimed semantic owner"
    );
  }
}

#[test]
fn top_level_state_records_compare_without_runtime_install() {
  let run = eval_file(&fixture_path()).unwrap();
  assert!(as_bool(get(&run, "compare-after-boot")));
  for key in [
    "fresh-p-puck-after-current-cut",
    "replay-executed",
    "boot-executed",
    "macro-only-runtime-owner-booted",
    "new-engine-from-zero",
    "runtime-install",
    "global-ontology-runtime",
    "host-code-removal-started",
    "implementation-command",
  ] {
    assert!(!as_bool(get(&run, key)), "`{key}` must stay false");
  }
}
