use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/macro-only-host-removal-fresh-delete-p-puck-current-cut-owner.px")
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
fn fixture_imports_owner_and_delete_candidate() {
  let run = eval_file(&fixture_path()).expect("fresh delete p-puck owner fixture");
  assert_eq!(
    as_str(get(&run, "proof")),
    "macro-only-host-removal-fresh-delete-p-puck-current-cut-owner"
  );
  assert!(as_bool(get(&run, "imported-owner")));
  assert!(as_bool(get(&run, "used-px-owner")));
  assert_eq!(
    as_str(get(&run, "imported-delete-patch-candidate")),
    "tesseract-macro-ontology-macro-only-host-removal-delete-patch-candidate"
  );
  assert_eq!(
    as_str(get(&run, "delete-patch-candidate-status")),
    "macro-only-host-removal-delete-patch-candidate-present"
  );
  assert_eq!(as_i64(get(&run, "delete-patch-candidate-target-count")), 5);
}

#[test]
fn owner_meta_declares_freshness_without_delete_ready() {
  let run = eval_file(&fixture_path()).unwrap();
  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.lib.gate.macro-only-host-removal-fresh-delete-p-puck-current-cut"
  );
  assert_eq!(
    as_str(get(meta, "constructor")),
    "validateHostRemovalFreshDeletePuckCurrentCut"
  );
  assert!(as_bool(get(meta, "fresh-puck-before-delete")));
  assert!(as_bool(get(
    meta,
    "fresh-puck-before-delete-as-delete-ready-frontier-closed"
  )));
  for key in [
    "actual-host-removal-patch-authorized",
    "delete-ready",
    "remove-now",
    "host-code-removal-started",
    "runtime-install",
    "global-ontology-runtime",
    "runtime-api-flattening",
    "meaning-db",
    "new-engine-from-zero",
    "p-puck-is-semantic-owner",
    "old-host-authority",
    "gpl-family-dependencies",
    "implementation-command",
  ] {
    assert!(!as_bool(get(meta, key)), "`{key}` must stay false");
  }
  assert_eq!(as_i64(get(meta, "delete-ready-target-count")), 0);
}

#[test]
fn expected_report_counts_and_telemetry_are_pinned() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "expected-proof-id")),
    "proof.macro-only-host-removal.fresh-delete-p-puck-current-cut.v1"
  );
  assert_eq!(
    as_str(get(&run, "expected-current-stage")),
    "host-removal-delete-patch-candidate-present-not-delete-ready"
  );
  assert_eq!(
    as_str(get(&run, "expected-report-name")),
    "macro-only-current-cut-host-removal-delete-patch-candidate"
  );
  assert_eq!(
    as_str(get(&run, "expected-audited-receipt")),
    "fixtures/tesseract-macro-legacy-probe/macro_only_host_removal_delete_patch_candidate_receipt.px"
  );
  assert_eq!(as_i64(get(&run, "expected-duration-ms")), 1318);
  assert_eq!(as_i64(get(&run, "slow-threshold-ms")), 5000);
  assert_eq!(
    as_str(get(&run, "expected-slow-path-status")),
    "within-threshold"
  );
  assert_eq!(as_i64(get(&run, "expected-total-tests")), 1053);
  assert_eq!(as_i64(get(&run, "expected-source-tracked")), 18207);
  assert_eq!(as_i64(get(&run, "expected-source-indexed")), 18207);
}

#[test]
fn valid_proof_closes_fresh_delete_puck_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let valid = get(&run, "valid-proof");
  assert_eq!(
    as_str(get(valid, "status")),
    "host-removal-fresh-delete-p-puck-current-cut-present"
  );
  assert!(matches!(get(valid, "held-id"), Value::Null));
  assert!(as_bool(get(
    valid,
    "host-removal-fresh-delete-p-puck-current-cut"
  )));
  assert!(as_bool(get(valid, "fresh-puck-before-delete")));
  assert!(as_bool(get(
    valid,
    "fresh-puck-before-delete-as-delete-ready-frontier-closed"
  )));
  assert!(as_bool(get(valid, "actual-host-removal-patch-candidate")));
  assert_eq!(as_i64(get(valid, "patch-candidate-target-count")), 5);
  assert_eq!(as_i64(get(valid, "delete-ready-target-count")), 0);
  for key in [
    "actual-host-removal-patch-authorized",
    "delete-ready",
    "remove-now",
    "host-code-removal-started",
    "host-removal-safe",
    "runtime-install",
    "global-ontology-runtime",
    "runtime-api-flattening",
    "meaning-db",
    "implementation-command",
  ] {
    assert!(!as_bool(get(valid, key)), "`{key}` must stay false");
  }
}

#[test]
fn required_evidence_and_remaining_frontiers_are_explicit() {
  let run = eval_file(&fixture_path()).unwrap();
  let evidence = string_set(get(&run, "required-evidence"));
  for expected in [
    "delete-patch-candidate-present",
    "patch-candidate-target-count-five",
    "audited-receipt-is-delete-patch-candidate",
    "slow-path-within-threshold-recorded",
    "delete-ready-target-count-zero",
    "compare-all-1053-ok",
    "source-inventory-18207-parity",
  ] {
    assert!(evidence.contains(expected), "missing evidence `{expected}`");
  }

  let frontiers = string_set(get(&run, "remaining-open-frontiers"));
  assert!(frontiers.contains("delete-ready-targets-after-fresh-delete-puck"));
  assert!(frontiers.contains("actual-host-removal-implementation-command"));
  assert!(frontiers.contains("domain-runtime-api-flattening-after-semantic-owner"));
  assert_eq!(frontiers.len(), 5);
}

#[test]
fn stale_report_candidate_compare_and_source_cases_are_held() {
  let run = eval_file(&fixture_path()).unwrap();
  for (key, held_id) in [
    (
      "wrong-proof",
      "held.macro-only-host-removal-fresh-delete-puck.proof-id-mismatch",
    ),
    (
      "stale-stage",
      "held.macro-only-host-removal-fresh-delete-puck.stale-current-stage",
    ),
    (
      "delete-candidate-missing",
      "held.macro-only-host-removal-fresh-delete-puck.delete-candidate-missing",
    ),
    (
      "report-mismatch",
      "held.macro-only-host-removal-fresh-delete-puck.report-mismatch",
    ),
    (
      "telemetry-mismatch",
      "held.macro-only-host-removal-fresh-delete-puck.telemetry-mismatch",
    ),
    (
      "receipt-mismatch",
      "held.macro-only-host-removal-fresh-delete-puck.current-cut-receipt-mismatch",
    ),
    (
      "telemetry-drift",
      "held.macro-only-host-removal-fresh-delete-puck.telemetry-missing-or-drifted",
    ),
    (
      "compare-mismatch",
      "held.macro-only-host-removal-fresh-delete-puck.compare-all-mismatch",
    ),
    (
      "source-mismatch",
      "held.macro-only-host-removal-fresh-delete-puck.source-parity-mismatch",
    ),
    (
      "missing-evidence",
      "held.macro-only-host-removal-fresh-delete-puck.missing-required-evidence",
    ),
  ] {
    let case = get(&run, key);
    assert_eq!(as_str(get(case, "status")), "Held", "{key}");
    assert_eq!(as_str(get(case, "held-id")), held_id, "{key}");
  }
}

#[test]
fn delete_runtime_puck_old_host_and_gpl_overclaims_are_held() {
  let run = eval_file(&fixture_path()).unwrap();
  for (key, held_id) in [
    (
      "delete-claim",
      "held.macro-only-host-removal-fresh-delete-puck.delete-overclaim",
    ),
    (
      "runtime-claim",
      "held.macro-only-host-removal-fresh-delete-puck.runtime-overclaim",
    ),
    (
      "semantic-owner-claim",
      "held.macro-only-host-removal-fresh-delete-puck.p-puck-semantic-owner",
    ),
    (
      "old-host-authority",
      "held.macro-only-host-removal-fresh-delete-puck.old-host-authority",
    ),
    (
      "gpl-claim",
      "held.macro-only-host-removal-fresh-delete-puck.gpl-family-dependency",
    ),
  ] {
    let case = get(&run, key);
    assert_eq!(as_str(get(case, "status")), "Held", "{key}");
    assert_eq!(as_str(get(case, "held-id")), held_id, "{key}");
  }
}

#[test]
fn top_level_state_records_fresh_delete_cut_without_install_or_db() {
  let run = eval_file(&fixture_path()).unwrap();
  assert!(as_bool(get(
    &run,
    "host-removal-fresh-delete-p-puck-current-cut"
  )));
  assert!(as_bool(get(&run, "fresh-puck-before-delete")));
  assert!(as_bool(get(
    &run,
    "fresh-puck-before-delete-as-delete-ready-frontier-closed"
  )));
  assert!(!as_bool(get(&run, "delete-ready")));
  assert_eq!(as_i64(get(&run, "delete-ready-target-count")), 0);
  assert!(!as_bool(get(&run, "remove-now")));
  assert!(!as_bool(get(&run, "host-code-removal-started")));
  assert!(!as_bool(get(&run, "runtime-api-flattening")));
  assert!(!as_bool(get(&run, "meaning-db")));
  assert!(!as_bool(get(&run, "implementation-command")));
}
