use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/macro-only-boot-target-delete-preflight-owner.px")
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
fn preflight_fixture_imports_owner_and_uses_shallow_status_evidence() {
  let run = eval_file(&fixture_path()).expect("macro-only target delete preflight owner fixture");
  assert_eq!(
    as_str(get(&run, "proof")),
    "macro-only-boot-target-delete-preflight-owner"
  );
  assert!(as_bool(get(&run, "imported-owner")));
  assert!(as_bool(get(&run, "used-px-owner")));
  assert!(!as_bool(get(&run, "imported-bootstrap-status")));
  assert_eq!(
    as_str(get(&run, "bootstrap-status-source")),
    "shallow-status-snapshot"
  );
  assert!(!as_bool(get(&run, "imported-host-files")));
  assert_eq!(
    as_str(get(&run, "host-target-source")),
    "host-removal-map-target-list"
  );
}

#[test]
fn owner_meta_declares_preflight_without_delete_boot_or_semantic_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.lib.gate.macro-only-boot-target-delete-preflight"
  );
  assert_eq!(
    as_str(get(meta, "constructor")),
    "validateTargetDeletePreflight"
  );
  assert_eq!(
    as_str(get(meta, "output-shape")),
    "target-delete-preflight-present or Held"
  );
  assert!(as_bool(get(meta, "target-delete-preflight-present")));
  for key in [
    "target-specific-delete-proof-present",
    "fresh-p-puck-after-current-cut",
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
fn required_targets_evidence_and_frontiers_are_explicit() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "expected-current-stage")),
    "macro-only-compare-after-boot-present"
  );
  let targets = string_set(get(&run, "required-targets"));
  for expected in [
    "stdlib/lib/ontology.px",
    "crates/pnix-runtime-legacy/src/ssa_eval/builtins/mod.rs",
    "crates/pnix-runtime-legacy/src/ir/eval.rs",
    "crates/pnix-core/src/ontology.rs",
    "crates/pnix-eval/tests/ontology_builtins.rs",
  ] {
    assert!(targets.contains(expected), "missing target `{expected}`");
  }
  assert_eq!(targets.len(), 5);

  let evidence = string_set(get(&run, "required-evidence"));
  for expected in [
    "host-removal-map-written",
    "macro-only-boot-manifest-written",
    "macro-only-boot-execution-attempted",
    "macro-only-boot-runner-owner-present",
    "bounded-full-graph-replay-strategy-present",
    "regression-corpus-transfer-present",
    "bootstrap-status-audit-update-plan-present",
    "compare-after-boot",
    "target-list-recorded",
    "caller-usage-scan-deferred",
    "replacement-replay-corpus-deferred",
    "rollback-plan-deferred",
    "fresh-p-puck-still-false-recorded",
    "boot-executed-false-recorded",
    "delete-ready-target-count-zero",
    "host-code-removal-started-false-recorded",
    "gpl-family-dependencies-false",
  ] {
    assert!(evidence.contains(expected), "missing evidence `{expected}`");
  }
  assert_eq!(evidence.len(), 17);

  let frontiers = string_set(get(&run, "required-open-frontiers"));
  assert!(frontiers.contains("fresh-p-puck-after-current-cut"));
  assert!(frontiers.contains("target-specific-delete-proof-present"));
  assert!(frontiers.contains("bounded-replay-execution-proof-after-runner-ready"));
  assert_eq!(frontiers.len(), 3);
}

#[test]
fn valid_preflight_is_present_but_all_targets_remain_blocked() {
  let run = eval_file(&fixture_path()).unwrap();
  let valid = get(&run, "valid-preflight");
  assert_eq!(
    as_str(get(valid, "status")),
    "target-delete-preflight-present"
  );
  assert_eq!(as_str(get(valid, "preflight-status")), "present");
  assert!(matches!(get(valid, "held-id"), Value::Null));
  assert!(as_bool(get(valid, "target-delete-preflight-present")));
  assert!(!as_bool(get(valid, "target-specific-delete-proof-present")));
  assert_eq!(as_list(get(valid, "targets")).len(), 5);
  assert_eq!(as_list(get(valid, "blocked-targets")).len(), 5);
  assert_eq!(as_list(get(valid, "ready-targets")).len(), 0);
  assert_eq!(as_i64(get(valid, "delete-ready-target-count")), 0);
  assert_eq!(as_list(get(valid, "missing")).len(), 0);
}

#[test]
fn stale_stage_wrong_preflight_missing_target_evidence_and_frontier_are_held() {
  let run = eval_file(&fixture_path()).unwrap();
  for (key, held_id) in [
    (
      "stale-stage",
      "held.macro-only-target-delete-preflight.stale-current-stage",
    ),
    (
      "wrong-preflight",
      "held.macro-only-target-delete-preflight.preflight-id-mismatch",
    ),
    (
      "missing-target",
      "held.macro-only-target-delete-preflight.missing-required-evidence",
    ),
    (
      "missing-evidence",
      "held.macro-only-target-delete-preflight.missing-required-evidence",
    ),
    (
      "missing-frontier",
      "held.macro-only-target-delete-preflight.missing-required-evidence",
    ),
  ] {
    let output = get(&run, key);
    assert_eq!(as_str(get(output, "status")), "Held");
    assert_eq!(as_str(get(output, "held-id")), held_id);
    assert!(!as_bool(get(output, "target-delete-preflight-present")));
    assert_eq!(as_i64(get(output, "delete-ready-target-count")), 0);
  }

  assert!(string_set(get(get(&run, "missing-target"), "missing"))
    .contains("crates/pnix-eval/tests/ontology_builtins.rs"));
  assert!(
    string_set(get(get(&run, "missing-evidence"), "missing")).contains("rollback-plan-deferred")
  );
  assert!(string_set(get(get(&run, "missing-frontier"), "missing"))
    .contains("target-specific-delete-proof-present"));
}

#[test]
fn preflight_cannot_claim_puck_boot_delete_semantic_old_host_or_gpl() {
  let run = eval_file(&fixture_path()).unwrap();
  for (key, held_id) in [
    (
      "old-host-authority",
      "held.macro-only-target-delete-preflight.old-host-authority",
    ),
    (
      "fresh-puck-claim",
      "held.macro-only-target-delete-preflight.fresh-puck-claim",
    ),
    (
      "boot-claim",
      "held.macro-only-target-delete-preflight.boot-claim",
    ),
    (
      "delete-claim",
      "held.macro-only-target-delete-preflight.delete-proof-claim",
    ),
    (
      "semantic-owner-claim",
      "held.macro-only-target-delete-preflight.semantic-owner-claim",
    ),
    (
      "gpl-claim",
      "held.macro-only-target-delete-preflight.gpl-family-dependency",
    ),
  ] {
    let output = get(&run, key);
    assert_eq!(as_str(get(output, "status")), "Held");
    assert_eq!(as_str(get(output, "held-id")), held_id);
    assert!(!as_bool(get(output, "target-delete-preflight-present")));
    assert!(!as_bool(get(
      output,
      "target-specific-delete-proof-present"
    )));
    assert!(!as_bool(get(output, "host-code-removal-started")));
  }
}

#[test]
fn every_output_preserves_non_runtime_and_non_delete_boundaries() {
  let run = eval_file(&fixture_path()).unwrap();
  for key in [
    "valid-preflight",
    "missing-target",
    "missing-frontier",
    "missing-evidence",
    "stale-stage",
    "wrong-preflight",
    "old-host-authority",
    "fresh-puck-claim",
    "boot-claim",
    "delete-claim",
    "semantic-owner-claim",
    "gpl-claim",
  ] {
    let output = get(&run, key);
    for flag in [
      "fresh-p-puck-after-current-cut",
      "replay-executed",
      "boot-executed",
      "macro-only-runtime-owner-booted",
      "new-engine-from-zero",
      "runtime-install",
      "global-ontology-runtime",
      "host-code-removal-started",
      "semantic-owner",
      "implementation-command",
    ] {
      assert!(
        !as_bool(get(output, flag)),
        "`{key}.{flag}` must stay false"
      );
    }
    assert_eq!(as_i64(get(output, "delete-ready-target-count")), 0);
  }
}

#[test]
fn top_level_state_records_preflight_without_target_delete_proof() {
  let run = eval_file(&fixture_path()).unwrap();
  assert!(as_bool(get(&run, "target-delete-preflight-present")));
  for key in [
    "target-specific-delete-proof-present",
    "fresh-p-puck-after-current-cut",
    "replay-executed",
    "boot-executed",
    "macro-only-runtime-owner-booted",
    "new-engine-from-zero",
    "runtime-install",
    "global-ontology-runtime",
    "host-code-removal-started",
    "gpl-family-dependencies",
    "implementation-command",
  ] {
    assert!(!as_bool(get(&run, key)), "`{key}` must stay false");
  }
  assert_eq!(as_i64(get(&run, "delete-ready-target-count")), 0);
}

#[test]
fn preflight_owner_file_exists_under_stdlib_gate() {
  let owner_path = Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../stdlib/lib/gate/macro-only-boot-target-delete-preflight.px");
  assert!(owner_path.is_file());
}
