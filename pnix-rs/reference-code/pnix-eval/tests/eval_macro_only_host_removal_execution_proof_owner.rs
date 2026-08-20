use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/macro-only-host-removal-execution-proof-owner.px")
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
fn fixture_imports_owner_semantic_owner_and_target_delete_proof() {
  let run = eval_file(&fixture_path()).expect("host-removal execution proof fixture");
  assert_eq!(
    as_str(get(&run, "proof")),
    "macro-only-host-removal-execution-proof-owner"
  );
  assert!(as_bool(get(&run, "imported-owner")));
  assert!(as_bool(get(&run, "used-px-owner")));
  assert_eq!(
    as_str(get(&run, "imported-semantic-owner-proof")),
    "macro-only-semantic-owner-proof-owner"
  );
  assert_eq!(
    as_str(get(&run, "imported-target-delete-proof")),
    "macro-only-boot-target-delete-proof-owner"
  );
  assert_eq!(
    as_str(get(&run, "semantic-owner-proof-status")),
    "macro-only-semantic-owner-proof-present"
  );
  assert_eq!(
    as_str(get(&run, "target-specific-delete-proof-status")),
    "target-specific-delete-proof-present"
  );
}

#[test]
fn owner_meta_closes_gate_shape_without_authorizing_deletion() {
  let run = eval_file(&fixture_path()).unwrap();
  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.lib.gate.macro-only-host-removal-execution-proof"
  );
  assert_eq!(
    as_str(get(meta, "constructor")),
    "validateHostRemovalExecutionProof"
  );
  assert!(as_bool(get(meta, "host-removal-execution-proof")));
  assert!(as_bool(get(meta, "requires-semantic-owner-proof")));
  assert!(as_bool(get(meta, "requires-target-specific-delete-proof")));
  assert!(as_bool(get(
    meta,
    "requires-fresh-puck-before-actual-delete"
  )));
  for key in [
    "host-removal-execution-authorized",
    "new-engine-from-zero",
    "runtime-install",
    "global-ontology-runtime",
    "host-code-removal-started",
    "old-host-authority",
    "gpl-family-dependencies",
    "implementation-command",
  ] {
    assert!(!as_bool(get(meta, key)), "`{key}` must stay false");
  }
  assert_eq!(as_i64(get(meta, "delete-ready-target-count")), 0);
}

#[test]
fn expected_stage_counts_and_measurements_are_pinned() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "expected-proof-id")),
    "proof.macro-only-host-removal.execution-after-semantic-owner.v1"
  );
  assert_eq!(
    as_str(get(&run, "expected-current-stage")),
    "macro-only-semantic-owner-proof-present"
  );
  assert_eq!(
    as_str(get(&run, "expected-semantic-owner-scope")),
    "bounded-generated-ontology-semantic-owner"
  );
  assert_eq!(
    as_str(get(&run, "expected-compare-command")),
    "bash scripts/tesseract-macro-ontology-compare.sh --all"
  );
  assert_eq!(as_i64(get(&run, "expected-total-tests")), 981);
  assert_eq!(as_i64(get(&run, "expected-source-tracked")), 18187);
  assert_eq!(as_i64(get(&run, "expected-source-indexed")), 18187);
  assert_eq!(
    as_str(get(&run, "expected-puck-report-name")),
    "macro-only-current-cut-bounded-replay"
  );
  assert_eq!(as_i64(get(&run, "expected-puck-duration-ms")), 4934);
}

#[test]
fn valid_proof_sets_execution_gate_but_keeps_delete_states_false() {
  let run = eval_file(&fixture_path()).unwrap();
  let valid = get(&run, "valid-proof");
  assert_eq!(
    as_str(get(valid, "status")),
    "macro-only-host-removal-execution-proof-present"
  );
  assert!(matches!(get(valid, "held-id"), Value::Null));
  assert!(as_bool(get(valid, "host-removal-execution-proof")));
  assert!(as_bool(get(valid, "host-removal-execution-gate-present")));
  assert!(!as_bool(get(valid, "host-removal-execution-authorized")));
  assert!(as_bool(get(valid, "semantic-owner")));
  assert_eq!(
    as_str(get(valid, "semantic-owner-scope")),
    "bounded-generated-ontology-semantic-owner"
  );
  assert_eq!(as_i64(get(valid, "total-tests")), 981);
  assert_eq!(as_i64(get(valid, "source-tracked")), 18187);
  assert_eq!(as_i64(get(valid, "source-indexed")), 18187);
  assert!(as_bool(get(valid, "fresh-puck-before-delete-required")));
  assert!(!as_bool(get(valid, "fresh-puck-before-delete")));
  assert!(as_bool(get(valid, "old-host-code-still-present")));
  assert_eq!(as_list(get(valid, "targets")).len(), 5);
  assert_eq!(as_list(get(valid, "execution-plan-targets")).len(), 5);
  for key in [
    "new-engine-from-zero",
    "runtime-install",
    "global-ontology-runtime",
    "old-host-authority",
    "host-code-removal-started",
    "host-removal-safe",
    "gpl-family-dependencies",
    "implementation-command",
  ] {
    assert!(!as_bool(get(valid, key)), "`{key}` must stay false");
  }
  assert_eq!(as_i64(get(valid, "delete-ready-target-count")), 0);
}

#[test]
fn required_targets_evidence_and_frontiers_are_explicit() {
  let run = eval_file(&fixture_path()).unwrap();
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

  let evidence = string_set(get(&run, "required-evidence"));
  for expected in [
    "semantic-owner-proof-present",
    "semantic-owner-bounded",
    "target-specific-delete-proof-present",
    "host-removal-map-written",
    "all-five-host-targets-protected",
    "compare-all-981-ok",
    "source-inventory-tracked-equals-indexed",
    "fresh-puck-before-delete-required",
    "delete-ready-target-count-zero",
    "gpl-family-dependencies-false",
  ] {
    assert!(evidence.contains(expected), "missing evidence `{expected}`");
  }

  let target_evidence = string_set(get(&run, "required-target-evidence"));
  assert!(target_evidence.contains("semantic-replacement-owner-present"));
  assert!(target_evidence.contains("protected-before-delete-execution"));
  assert!(target_evidence.contains("remove-now-false"));

  let frontiers = string_set(get(&run, "remaining-open-frontiers"));
  assert!(frontiers.contains("fresh-puck-before-host-removal-execution"));
  assert!(frontiers.contains("actual-host-removal-patch-after-fresh-puck"));
  assert!(frontiers.contains("global-runtime-install-proof-after-semantic-owner"));
  assert_eq!(frontiers.len(), 5);
}

#[test]
fn held_cases_block_stale_missing_measurement_and_target_failures() {
  let run = eval_file(&fixture_path()).unwrap();
  for (key, held_id) in [
    (
      "wrong-proof",
      "held.macro-only-host-removal-execution.proof-id-mismatch",
    ),
    (
      "stale-stage",
      "held.macro-only-host-removal-execution.stale-current-stage",
    ),
    (
      "semantic-owner-missing",
      "held.macro-only-host-removal-execution.semantic-owner-missing",
    ),
    (
      "semantic-scope-mismatch",
      "held.macro-only-host-removal-execution.semantic-owner-missing",
    ),
    (
      "audit-chain-missing",
      "held.macro-only-host-removal-execution.audit-chain-missing",
    ),
    (
      "target-proof-missing",
      "held.macro-only-host-removal-execution.host-removal-map-or-target-proof-missing",
    ),
    (
      "compare-mismatch",
      "held.macro-only-host-removal-execution.compare-all-mismatch",
    ),
    (
      "source-parity-mismatch",
      "held.macro-only-host-removal-execution.source-parity-mismatch",
    ),
    (
      "puck-telemetry-mismatch",
      "held.macro-only-host-removal-execution.p-puck-telemetry-mismatch",
    ),
    (
      "target-evidence-missing",
      "held.macro-only-host-removal-execution.missing-required-evidence",
    ),
    (
      "missing-target",
      "held.macro-only-host-removal-execution.missing-required-evidence",
    ),
  ] {
    let output = get(&run, key);
    assert_eq!(as_str(get(output, "status")), "Held");
    assert_eq!(as_str(get(output, "held-id")), held_id);
    assert!(!as_bool(get(output, "host-removal-execution-proof")));
    assert!(!as_bool(get(output, "host-code-removal-started")));
  }
}

#[test]
fn boundary_cases_block_delete_fresh_puck_or_host_code_loss() {
  let run = eval_file(&fixture_path()).unwrap();
  for (key, held_id) in [
    (
      "fresh-puck-boundary",
      "held.macro-only-host-removal-execution.fresh-puck-boundary",
    ),
    (
      "host-code-lost",
      "held.macro-only-host-removal-execution.host-code-or-held-loss",
    ),
    (
      "deletion-overclaim",
      "held.macro-only-host-removal-execution.deletion-overclaim",
    ),
  ] {
    let output = get(&run, key);
    assert_eq!(as_str(get(output, "status")), "Held");
    assert_eq!(as_str(get(output, "held-id")), held_id);
    assert!(!as_bool(get(output, "host-removal-execution-authorized")));
    assert_eq!(as_i64(get(output, "delete-ready-target-count")), 0);
  }
}

#[test]
fn overclaims_are_held_before_global_old_host_or_gpl_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  for (key, held_id) in [
    (
      "global-runtime-claim",
      "held.macro-only-host-removal-execution.global-runtime-overclaim",
    ),
    (
      "old-host-authority",
      "held.macro-only-host-removal-execution.old-host-authority",
    ),
    (
      "gpl-claim",
      "held.macro-only-host-removal-execution.gpl-family-dependency",
    ),
  ] {
    let output = get(&run, key);
    assert_eq!(as_str(get(output, "status")), "Held");
    assert_eq!(as_str(get(output, "held-id")), held_id);
    assert!(!as_bool(get(output, "host-removal-execution-proof")));
    assert!(!as_bool(get(output, "host-code-removal-started")));
  }
}
