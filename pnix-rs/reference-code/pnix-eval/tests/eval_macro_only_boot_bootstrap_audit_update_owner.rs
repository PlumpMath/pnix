use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/macro-only-boot-bootstrap-audit-update-owner.px")
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
fn audit_update_fixture_imports_owner_and_uses_shallow_status_snapshots() {
  let run =
    eval_file(&fixture_path()).expect("macro-only boot bootstrap audit update fixture must eval");
  assert_eq!(
    as_str(get(&run, "proof")),
    "macro-only-boot-bootstrap-audit-update-owner"
  );
  assert!(as_bool(get(&run, "imported-owner")));
  assert!(as_bool(get(&run, "used-px-owner")));
  assert!(!as_bool(get(&run, "imported-corpus-retention")));
  assert_eq!(
    as_str(get(&run, "corpus-retention-source")),
    "shallow-status-snapshot"
  );
  assert_eq!(
    as_str(get(&run, "bootstrap-status-source")),
    "shallow-status-snapshot"
  );
  assert!(!as_bool(get(&run, "imported-bootstrap-status")));
}

#[test]
fn owner_meta_declares_audit_update_without_runtime_or_audit_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.lib.gate.macro-only-boot-bootstrap-audit-update"
  );
  assert_eq!(
    as_str(get(meta, "constructor")),
    "validateBootstrapAuditUpdate"
  );
  assert_eq!(
    as_str(get(meta, "output-shape")),
    "bootstrap-status-audit-update-plan-present or Held"
  );
  for key in [
    "fresh-p-puck-after-current-cut",
    "compare-after-boot",
    "replay-executed",
    "boot-executed",
    "macro-only-runtime-owner-booted",
    "new-engine-from-zero",
    "runtime-install",
    "global-ontology-runtime",
    "host-code-removal-started",
    "p-puck-owned-by-audit-update",
    "compare-owned-by-audit-update",
  ] {
    assert!(!as_bool(get(meta, key)), "`{key}` must stay false");
  }
}

#[test]
fn required_evidence_records_stage_false_states_and_remaining_frontiers() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "expected-current-stage")),
    "macro-only-regression-corpus-retention-present"
  );
  let evidence = string_set(get(&run, "required-evidence"));
  for expected in [
    "bounded-replay-strategy-present",
    "regression-corpus-transfer-present",
    "bootstrap-current-stage-recorded",
    "runner-missing-vector-after-corpus-recorded",
    "latest-unaudited-receipt-recorded",
    "fresh-p-puck-still-false-recorded",
    "compare-after-boot-still-false-recorded",
    "boot-executed-false-recorded",
    "macro-only-runtime-owner-booted-false-recorded",
    "new-engine-from-zero-false-recorded",
    "host-code-removal-started-false-recorded",
    "delete-ready-target-count-zero",
    "next-frontiers-recorded",
    "gpl-family-dependencies-false",
  ] {
    assert!(evidence.contains(expected), "missing evidence `{expected}`");
  }
  assert_eq!(evidence.len(), 14);

  let frontiers = string_set(get(&run, "required-open-frontiers"));
  assert!(frontiers.contains("fresh-p-puck-after-current-cut"));
  assert!(frontiers.contains("compare-after-boot"));
  assert!(frontiers.contains("target-specific-delete-proof-present"));
  assert_eq!(frontiers.len(), 3);
}

#[test]
fn valid_update_is_present_but_does_not_run_puck_compare_replay_or_boot() {
  let run = eval_file(&fixture_path()).unwrap();
  let valid = get(&run, "valid-update");
  assert_eq!(
    as_str(get(valid, "status")),
    "bootstrap-status-audit-update-plan-present"
  );
  assert_eq!(as_str(get(valid, "audit-update-status")), "present");
  assert!(matches!(get(valid, "held-id"), Value::Null));
  assert!(as_bool(get(
    valid,
    "bootstrap-status-audit-update-plan-present"
  )));
  assert_eq!(as_list(get(valid, "missing")).len(), 0);
  for key in [
    "fresh-p-puck-after-current-cut",
    "compare-after-boot",
    "replay-executed",
    "boot-executed",
    "macro-only-runtime-owner-booted",
    "new-engine-from-zero",
    "runtime-install",
    "global-ontology-runtime",
    "host-code-removal-started",
    "implementation-command",
  ] {
    assert!(!as_bool(get(valid, key)), "`{key}` must stay false");
  }
}

#[test]
fn missing_frontier_and_false_state_are_held_before_presence() {
  let run = eval_file(&fixture_path()).unwrap();
  let missing_frontier = get(&run, "missing-frontier");
  assert_eq!(as_str(get(missing_frontier, "status")), "Held");
  assert_eq!(
    as_str(get(missing_frontier, "held-id")),
    "held.macro-only-boot-bootstrap-audit-update.missing-required-evidence"
  );
  assert!(
    string_set(get(missing_frontier, "missing")).contains("target-specific-delete-proof-present")
  );

  let missing_false = get(&run, "missing-false-state");
  assert_eq!(as_str(get(missing_false, "status")), "Held");
  assert!(string_set(get(missing_false, "missing")).contains("boot-executed-false-recorded"));
}

#[test]
fn stale_stage_wrong_update_and_old_host_authority_are_held() {
  let run = eval_file(&fixture_path()).unwrap();
  let stale = get(&run, "stale-stage");
  assert_eq!(as_str(get(stale, "status")), "Held");
  assert_eq!(
    as_str(get(stale, "held-id")),
    "held.macro-only-boot-bootstrap-audit-update.stale-current-stage"
  );

  let wrong = get(&run, "wrong-update");
  assert_eq!(as_str(get(wrong, "status")), "Held");
  assert_eq!(
    as_str(get(wrong, "held-id")),
    "held.macro-only-boot-bootstrap-audit-update.update-id-mismatch"
  );

  let old_host = get(&run, "old-host-authority");
  assert_eq!(as_str(get(old_host, "status")), "Held");
  assert_eq!(
    as_str(get(old_host, "held-id")),
    "held.macro-only-boot-bootstrap-audit-update.old-host-authority"
  );
  assert!(!as_bool(get(old_host, "old-host-authority")));
}

#[test]
fn audit_update_cannot_claim_external_audit_boot_delete_or_gpl_dependency() {
  let run = eval_file(&fixture_path()).unwrap();
  let audit = get(&run, "external-audit-claim");
  assert_eq!(as_str(get(audit, "status")), "Held");
  assert_eq!(
    as_str(get(audit, "held-id")),
    "held.macro-only-boot-bootstrap-audit-update.external-audit-claim"
  );

  let boot = get(&run, "boot-claim");
  assert_eq!(as_str(get(boot, "status")), "Held");
  assert_eq!(
    as_str(get(boot, "held-id")),
    "held.macro-only-boot-bootstrap-audit-update.boot-claim"
  );

  let delete = get(&run, "delete-claim");
  assert_eq!(as_str(get(delete, "status")), "Held");
  assert_eq!(
    as_str(get(delete, "held-id")),
    "held.macro-only-boot-bootstrap-audit-update.delete-before-boot"
  );

  let gpl = get(&run, "gpl-claim");
  assert_eq!(as_str(get(gpl, "status")), "Held");
  assert_eq!(
    as_str(get(gpl, "held-id")),
    "held.macro-only-boot-bootstrap-audit-update.gpl-family-dependency"
  );
}

#[test]
fn all_outputs_preserve_no_runtime_or_host_delete_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  for key in [
    "valid-update",
    "missing-frontier",
    "missing-false-state",
    "stale-stage",
    "wrong-update",
    "old-host-authority",
    "external-audit-claim",
    "boot-claim",
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
      !as_bool(get(value, "compare-after-boot")),
      "`{key}` claimed compare"
    );
    assert!(
      !as_bool(get(value, "fresh-p-puck-after-current-cut")),
      "`{key}` claimed p-puck"
    );
  }
}

#[test]
fn top_level_state_records_audit_update_owner_without_runtime_install() {
  let run = eval_file(&fixture_path()).unwrap();
  assert!(as_bool(get(
    &run,
    "bootstrap-status-audit-update-plan-present"
  )));
  for key in [
    "fresh-p-puck-after-current-cut",
    "compare-after-boot",
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
