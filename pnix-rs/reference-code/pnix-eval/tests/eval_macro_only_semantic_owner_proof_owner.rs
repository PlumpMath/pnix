use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/macro-only-semantic-owner-proof-owner.px")
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
fn fixture_imports_owner_and_runtime_owner_proof() {
  let run = eval_file(&fixture_path()).expect("macro-only semantic owner proof fixture");
  assert_eq!(
    as_str(get(&run, "proof")),
    "macro-only-semantic-owner-proof-owner"
  );
  assert!(as_bool(get(&run, "imported-owner")));
  assert!(as_bool(get(&run, "used-px-owner")));
  assert_eq!(
    as_str(get(&run, "imported-runtime-owner-proof")),
    "macro-only-runtime-owner-proof-owner"
  );
  assert_eq!(
    as_str(get(&run, "runtime-owner-proof-status")),
    "macro-only-runtime-owner-proof-present"
  );
}

#[test]
fn owner_meta_allows_bounded_semantic_owner_without_runtime_install() {
  let run = eval_file(&fixture_path()).unwrap();
  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.lib.gate.macro-only-semantic-owner-proof"
  );
  assert_eq!(
    as_str(get(meta, "constructor")),
    "validateSemanticOwnerProof"
  );
  assert!(as_bool(get(meta, "semantic-owner-proof")));
  assert!(as_bool(get(meta, "requires-runtime-owner-proof")));
  assert!(as_bool(get(meta, "may-emit-semantic-owner")));
  assert_eq!(
    as_str(get(meta, "semantic-owner-scope")),
    "bounded-generated-ontology-semantic-owner"
  );
  for key in [
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
}

#[test]
fn expected_counts_scope_and_stage_are_pinned() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "expected-proof-id")),
    "proof.macro-only-semantic.owner-after-runtime-owner.v1"
  );
  assert_eq!(
    as_str(get(&run, "expected-current-stage")),
    "macro-only-runtime-owner-proof-present"
  );
  assert_eq!(
    as_str(get(&run, "expected-runtime-owner-scope")),
    "bounded-receipt-trajectory-owner"
  );
  assert_eq!(
    as_str(get(&run, "expected-semantic-owner-scope")),
    "bounded-generated-ontology-semantic-owner"
  );
  assert_eq!(
    as_str(get(&run, "expected-compare-command")),
    "bash scripts/tesseract-macro-ontology-compare.sh --all"
  );
  assert_eq!(as_i64(get(&run, "expected-total-tests")), 963);
  assert_eq!(as_i64(get(&run, "expected-source-tracked")), 18182);
  assert_eq!(as_i64(get(&run, "expected-source-indexed")), 18182);
  assert_eq!(
    as_str(get(&run, "expected-puck-report-name")),
    "macro-only-current-cut-bounded-replay"
  );
  assert_eq!(as_i64(get(&run, "expected-puck-duration-ms")), 4934);
}

#[test]
fn valid_proof_sets_semantic_owner_but_not_global_or_host_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  let valid = get(&run, "valid-proof");
  assert_eq!(
    as_str(get(valid, "status")),
    "macro-only-semantic-owner-proof-present"
  );
  assert_eq!(
    as_str(get(valid, "macro-only-semantic-owner-proof-status")),
    "present"
  );
  assert!(matches!(get(valid, "held-id"), Value::Null));
  assert!(as_bool(get(valid, "semantic-owner-proof")));
  assert!(as_bool(get(valid, "semantic-owner")));
  assert_eq!(
    as_str(get(valid, "semantic-owner-scope")),
    "bounded-generated-ontology-semantic-owner"
  );
  assert!(as_bool(get(valid, "runtime-owner-proof")));
  assert!(as_bool(get(valid, "macro-only-runtime-owner-booted")));
  assert!(as_bool(get(valid, "boot-executed")));
  assert_eq!(as_i64(get(valid, "total-tests")), 963);
  assert_eq!(as_i64(get(valid, "source-tracked")), 18182);
  assert_eq!(as_i64(get(valid, "source-indexed")), 18182);
  for key in [
    "new-engine-from-zero",
    "runtime-install",
    "global-ontology-runtime",
    "host-code-removal-started",
    "host-removal-safe",
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
    "macro-only-runtime-owner-proof-present",
    "macro-only-runtime-owner-booted",
    "runtime-owner-scope-bounded",
    "compare-all-963-ok",
    "source-inventory-tracked-equals-indexed",
    "generated-ontology-surface-present",
    "old-ontology-surfaces-demoted",
    "legacy-externs-classified",
    "semantic-owner-scope-bounded",
    "host-code-removal-started-false-recorded",
    "global-runtime-false-recorded",
    "gpl-family-dependencies-false",
  ] {
    assert!(evidence.contains(expected), "missing evidence `{expected}`");
  }

  let frontiers = string_set(get(&run, "remaining-open-frontiers"));
  assert!(frontiers.contains("host-code-removal-execution-proof-after-semantic-owner"));
  assert!(frontiers.contains("global-runtime-install-proof-after-semantic-owner"));
  assert!(frontiers.contains("domain-runtime-api-flattening-after-semantic-owner"));
  assert_eq!(frontiers.len(), 3);
}

#[test]
fn held_cases_block_stale_runtime_measurement_surface_and_scope_failures() {
  let run = eval_file(&fixture_path()).unwrap();
  for (key, held_id) in [
    (
      "wrong-proof",
      "held.macro-only-semantic-owner.proof-id-mismatch",
    ),
    (
      "stale-stage",
      "held.macro-only-semantic-owner.stale-current-stage",
    ),
    (
      "runtime-owner-missing",
      "held.macro-only-semantic-owner.runtime-owner-proof-missing",
    ),
    (
      "runtime-owner-not-booted",
      "held.macro-only-semantic-owner.runtime-owner-proof-missing",
    ),
    (
      "runtime-owner-scope-mismatch",
      "held.macro-only-semantic-owner.runtime-owner-scope-mismatch",
    ),
    (
      "audit-chain-missing",
      "held.macro-only-semantic-owner.audit-chain-missing",
    ),
    (
      "compare-mismatch",
      "held.macro-only-semantic-owner.compare-all-mismatch",
    ),
    (
      "source-parity-mismatch",
      "held.macro-only-semantic-owner.source-parity-mismatch",
    ),
    (
      "puck-telemetry-mismatch",
      "held.macro-only-semantic-owner.p-puck-telemetry-mismatch",
    ),
    (
      "semantic-surface-missing",
      "held.macro-only-semantic-owner.semantic-surface-evidence-missing",
    ),
    (
      "semantic-delta-loss",
      "held.macro-only-semantic-owner.semantic-delta-or-held-loss",
    ),
    (
      "semantic-owner-scope-mismatch",
      "held.macro-only-semantic-owner.scope-mismatch",
    ),
  ] {
    let output = get(&run, key);
    assert_eq!(as_str(get(output, "status")), "Held");
    assert_eq!(as_str(get(output, "held-id")), held_id);
    assert!(!as_bool(get(output, "semantic-owner")));
  }
}

#[test]
fn overclaims_are_held_before_global_host_old_host_or_gpl_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  for (key, held_id) in [
    (
      "global-runtime-claim",
      "held.macro-only-semantic-owner.global-runtime-overclaim",
    ),
    (
      "host-removal-claim",
      "held.macro-only-semantic-owner.host-removal-overclaim",
    ),
    (
      "old-host-authority",
      "held.macro-only-semantic-owner.old-host-authority",
    ),
    (
      "gpl-claim",
      "held.macro-only-semantic-owner.gpl-family-dependency",
    ),
  ] {
    let output = get(&run, key);
    assert_eq!(as_str(get(output, "status")), "Held");
    assert_eq!(as_str(get(output, "held-id")), held_id);
    assert!(!as_bool(get(output, "semantic-owner-proof")));
    assert!(!as_bool(get(output, "semantic-owner")));
  }
}
