use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/macro-only-boot-target-delete-proof-owner.px")
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
fn proof_fixture_imports_owner_and_uses_shallow_target_bindings() {
  let run = eval_file(&fixture_path()).expect("macro-only target delete proof owner fixture");
  assert_eq!(
    as_str(get(&run, "proof")),
    "macro-only-boot-target-delete-proof-owner"
  );
  assert!(as_bool(get(&run, "imported-owner")));
  assert!(as_bool(get(&run, "used-px-owner")));
  assert!(!as_bool(get(&run, "imported-bootstrap-status")));
  assert_eq!(
    as_str(get(&run, "target-proof-source")),
    "target-delete-preflight-plus-target-specific-bindings"
  );
}

#[test]
fn owner_meta_declares_target_proof_without_runtime_delete_or_semantic_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.lib.gate.macro-only-boot-target-delete-proof"
  );
  assert_eq!(
    as_str(get(meta, "constructor")),
    "validateTargetSpecificDeleteProof"
  );
  assert_eq!(
    as_str(get(meta, "output-shape")),
    "target-specific-delete-proof-present or Held"
  );
  assert!(as_bool(get(meta, "target-delete-preflight-present")));
  assert!(as_bool(get(meta, "target-specific-delete-proof-present")));
  for key in [
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
fn required_targets_global_evidence_target_evidence_and_frontiers_are_explicit() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "expected-current-stage")),
    "macro-only-target-delete-preflight-present"
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
    "target-delete-preflight-present",
    "host-removal-map-written",
    "compare-after-boot",
    "caller-usage-scan-recorded",
    "replacement-replay-corpus-bound",
    "rollback-plan-bound",
    "regression-corpus-bound",
    "fresh-p-puck-still-false-recorded",
    "delete-ready-target-count-zero",
    "host-code-removal-started-false-recorded",
    "gpl-family-dependencies-false",
  ] {
    assert!(evidence.contains(expected), "missing evidence `{expected}`");
  }

  let target_evidence = string_set(get(&run, "required-target-evidence"));
  for expected in [
    "target-listed-in-preflight",
    "caller-scan-present",
    "replacement-replay-binding-present",
    "rollback-binding-present",
    "regression-corpus-binding-present",
    "old-host-authority-false",
    "delete-ready-false",
    "remove-now-false",
  ] {
    assert!(
      target_evidence.contains(expected),
      "missing target evidence `{expected}`"
    );
  }

  let frontiers = string_set(get(&run, "remaining-open-frontiers"));
  assert!(frontiers.contains("fresh-p-puck-after-current-cut"));
  assert!(frontiers.contains("bounded-replay-execution-proof-after-runner-ready"));
  assert_eq!(frontiers.len(), 2);
}

#[test]
fn valid_proof_is_target_specific_but_no_target_is_delete_ready() {
  let run = eval_file(&fixture_path()).unwrap();
  let valid = get(&run, "valid-proof");
  assert_eq!(
    as_str(get(valid, "status")),
    "target-specific-delete-proof-present"
  );
  assert_eq!(
    as_str(get(valid, "target-specific-delete-proof-status")),
    "present"
  );
  assert!(matches!(get(valid, "held-id"), Value::Null));
  assert!(as_bool(get(valid, "target-delete-preflight-present")));
  assert!(as_bool(get(valid, "target-specific-delete-proof-present")));
  assert_eq!(as_list(get(valid, "targets")).len(), 5);
  assert_eq!(as_list(get(valid, "protected-targets")).len(), 5);
  assert_eq!(as_list(get(valid, "ready-targets")).len(), 0);
  assert_eq!(as_i64(get(valid, "delete-ready-target-count")), 0);
  for target in as_list(get(valid, "protected-targets")) {
    assert!(as_bool(get(target, "target-specific-proof-present")));
    assert!(!as_bool(get(target, "delete-ready")));
    assert!(!as_bool(get(target, "remove-now")));
    assert!(!as_bool(get(target, "host-code-removal-started")));
  }
}

#[test]
fn stale_wrong_missing_global_missing_target_and_delete_ready_target_are_held() {
  let run = eval_file(&fixture_path()).unwrap();
  for (key, held_id) in [
    (
      "stale-stage",
      "held.macro-only-target-delete-proof.stale-current-stage",
    ),
    (
      "wrong-proof",
      "held.macro-only-target-delete-proof.proof-id-mismatch",
    ),
    (
      "missing-target",
      "held.macro-only-target-delete-proof.missing-required-evidence",
    ),
    (
      "missing-target-evidence",
      "held.macro-only-target-delete-proof.missing-required-evidence",
    ),
    (
      "delete-ready-target",
      "held.macro-only-target-delete-proof.missing-required-evidence",
    ),
    (
      "missing-evidence",
      "held.macro-only-target-delete-proof.missing-required-evidence",
    ),
  ] {
    let output = get(&run, key);
    assert_eq!(as_str(get(output, "status")), "Held");
    assert_eq!(as_str(get(output, "held-id")), held_id);
    assert!(!as_bool(get(
      output,
      "target-specific-delete-proof-present"
    )));
    assert_eq!(as_i64(get(output, "delete-ready-target-count")), 0);
  }

  assert!(string_set(get(get(&run, "missing-target"), "missing"))
    .contains("crates/pnix-eval/tests/ontology_builtins.rs"));
  assert!(
    string_set(get(get(&run, "missing-target-evidence"), "missing"))
      .contains("crates/pnix-runtime-legacy/src/ssa_eval/builtins/mod.rs")
  );
  assert!(string_set(get(get(&run, "missing-evidence"), "missing"))
    .contains("caller-usage-scan-recorded"));
}

#[test]
fn proof_cannot_claim_puck_boot_host_removal_semantic_old_host_or_gpl() {
  let run = eval_file(&fixture_path()).unwrap();
  for (key, held_id) in [
    (
      "old-host-authority",
      "held.macro-only-target-delete-proof.old-host-authority",
    ),
    (
      "fresh-puck-claim",
      "held.macro-only-target-delete-proof.fresh-puck-claim",
    ),
    (
      "boot-claim",
      "held.macro-only-target-delete-proof.boot-claim",
    ),
    (
      "host-removal-claim",
      "held.macro-only-target-delete-proof.host-removal-claim",
    ),
    (
      "semantic-owner-claim",
      "held.macro-only-target-delete-proof.semantic-owner-claim",
    ),
    (
      "gpl-claim",
      "held.macro-only-target-delete-proof.gpl-family-dependency",
    ),
  ] {
    let output = get(&run, key);
    assert_eq!(as_str(get(output, "status")), "Held");
    assert_eq!(as_str(get(output, "held-id")), held_id);
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
    "valid-proof",
    "missing-target",
    "missing-target-evidence",
    "delete-ready-target",
    "missing-evidence",
    "stale-stage",
    "wrong-proof",
    "old-host-authority",
    "fresh-puck-claim",
    "boot-claim",
    "host-removal-claim",
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
fn top_level_state_records_target_proof_without_fresh_puck_or_deletion() {
  let run = eval_file(&fixture_path()).unwrap();
  assert!(as_bool(get(&run, "target-delete-preflight-present")));
  assert!(as_bool(get(&run, "target-specific-delete-proof-present")));
  for key in [
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
fn target_delete_proof_owner_file_exists_under_stdlib_gate() {
  let owner_path = Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../stdlib/lib/gate/macro-only-boot-target-delete-proof.px");
  assert!(owner_path.is_file());
}
