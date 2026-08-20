use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/pnix-query-runtime/macro-only-host-removal-delete-patch-candidate-owner.px",
  )
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
fn fixture_imports_owner_and_prior_proofs() {
  let run = eval_file(&fixture_path()).expect("delete patch candidate owner fixture");
  assert_eq!(
    as_str(get(&run, "proof")),
    "macro-only-host-removal-delete-patch-candidate-owner"
  );
  assert!(as_bool(get(&run, "imported-owner")));
  assert!(as_bool(get(&run, "used-px-owner")));
  assert_eq!(
    as_str(get(&run, "imported-slow-path-repeat-proof")),
    "macro-only-host-removal-slow-path-repeat-proof-owner"
  );
  assert_eq!(
    as_str(get(&run, "imported-host-removal-execution-proof")),
    "macro-only-host-removal-execution-proof-owner"
  );
  assert_eq!(
    as_str(get(&run, "slow-path-repeat-status")),
    "host-removal-slow-path-repeat-within-threshold"
  );
}

#[test]
fn owner_meta_declares_candidate_not_delete_ready() {
  let run = eval_file(&fixture_path()).unwrap();
  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.lib.gate.macro-only-host-removal-delete-patch-candidate"
  );
  assert_eq!(
    as_str(get(meta, "constructor")),
    "validateHostRemovalDeletePatchCandidate"
  );
  assert!(as_bool(get(meta, "actual-host-removal-patch-candidate")));
  for key in [
    "actual-host-removal-patch-authorized",
    "delete-ready",
    "remove-now",
    "host-code-removal-started",
    "runtime-install",
    "global-ontology-runtime",
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
fn expected_counts_and_repeat_telemetry_are_pinned() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "expected-proof-id")),
    "proof.macro-only-host-removal.delete-patch-candidate.v1"
  );
  assert_eq!(
    as_str(get(&run, "expected-current-stage")),
    "host-removal-slow-path-repeat-closed-not-delete"
  );
  assert_eq!(as_i64(get(&run, "expected-total-tests")), 1035);
  assert_eq!(as_i64(get(&run, "expected-source-tracked")), 18202);
  assert_eq!(as_i64(get(&run, "expected-source-indexed")), 18202);
  assert_eq!(as_i64(get(&run, "expected-repeat-duration-ms")), 551);
  assert_eq!(as_i64(get(&run, "expected-slow-threshold-ms")), 5000);
  assert_eq!(
    as_str(get(&run, "expected-repeat-slow-path-status")),
    "within-threshold"
  );
}

#[test]
fn valid_proof_creates_patch_candidate_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let valid = get(&run, "valid-proof");
  assert_eq!(
    as_str(get(valid, "status")),
    "macro-only-host-removal-delete-patch-candidate-present"
  );
  assert!(matches!(get(valid, "held-id"), Value::Null));
  assert!(as_bool(get(valid, "actual-host-removal-patch-candidate")));
  assert!(as_bool(get(valid, "delete-patch-candidate-proof")));
  assert_eq!(as_list(get(valid, "targets")).len(), 5);
  assert_eq!(as_list(get(valid, "patch-candidate-targets")).len(), 5);
  assert_eq!(as_i64(get(valid, "delete-ready-target-count")), 0);
  for key in [
    "actual-host-removal-patch-authorized",
    "delete-ready",
    "remove-now",
    "host-code-removal-started",
    "host-removal-safe",
    "runtime-install",
    "global-ontology-runtime",
    "new-engine-from-zero",
    "p-puck-is-semantic-owner",
    "old-host-authority",
    "gpl-family-dependencies",
    "implementation-command",
  ] {
    assert!(!as_bool(get(valid, key)), "`{key}` must stay false");
  }
}

#[test]
fn target_hunks_are_candidate_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let targets = as_list(get(&run, "patch-candidate-targets"));
  assert_eq!(targets.len(), 5);
  let paths: BTreeSet<_> = targets.iter().map(|t| as_str(get(t, "path"))).collect();
  for expected in [
    "stdlib/lib/ontology.px",
    "crates/pnix-runtime-legacy/src/ssa_eval/builtins/mod.rs",
    "crates/pnix-runtime-legacy/src/ir/eval.rs",
    "crates/pnix-core/src/ontology.rs",
    "crates/pnix-eval/tests/ontology_builtins.rs",
  ] {
    assert!(paths.contains(expected), "missing target `{expected}`");
  }
  for target in targets {
    assert_eq!(
      as_str(get(target, "patch-action")),
      "candidate-delete-old-host-authority"
    );
    assert!(as_bool(get(target, "delete-candidate")));
    assert!(!as_bool(get(target, "delete-ready")));
    assert!(!as_bool(get(target, "remove-now")));
    assert!(!as_bool(get(target, "host-code-removal-started")));
    assert!(!as_bool(get(target, "implementation-command")));
  }
}

#[test]
fn required_evidence_and_remaining_frontiers_are_explicit() {
  let run = eval_file(&fixture_path()).unwrap();
  let evidence = string_set(get(&run, "required-evidence"));
  for expected in [
    "host-removal-slow-path-repeat-proof-present",
    "candidate-patch-hunks-recorded",
    "per-target-caller-scan-recorded",
    "per-target-replacement-replay-binding-recorded",
    "per-target-rollback-binding-recorded",
    "per-target-regression-corpus-binding-recorded",
    "compare-all-1035-ok",
    "source-inventory-18202-parity",
  ] {
    assert!(evidence.contains(expected), "missing evidence `{expected}`");
  }

  let frontiers = string_set(get(&run, "remaining-open-frontiers"));
  assert!(frontiers.contains("fresh-puck-before-delete-as-delete-ready"));
  assert!(frontiers.contains("delete-ready-targets-after-delete-candidate"));
  assert!(frontiers.contains("actual-host-removal-implementation-command"));
  assert!(frontiers.contains("domain-runtime-api-flattening-after-semantic-owner"));
  assert_eq!(frontiers.len(), 6);
}

#[test]
fn stale_missing_target_compare_and_source_cases_are_held() {
  let run = eval_file(&fixture_path()).unwrap();
  for (key, held_id) in [
    (
      "wrong-proof",
      "held.macro-only-host-removal-delete-patch-candidate.proof-id-mismatch",
    ),
    (
      "stale-stage",
      "held.macro-only-host-removal-delete-patch-candidate.stale-current-stage",
    ),
    (
      "slow-path-missing",
      "held.macro-only-host-removal-delete-patch-candidate.slow-path-repeat-missing",
    ),
    (
      "target-evidence-missing",
      "held.macro-only-host-removal-delete-patch-candidate.missing-required-evidence",
    ),
    (
      "missing-target",
      "held.macro-only-host-removal-delete-patch-candidate.missing-required-evidence",
    ),
    (
      "compare-mismatch",
      "held.macro-only-host-removal-delete-patch-candidate.compare-all-mismatch",
    ),
    (
      "source-parity-mismatch",
      "held.macro-only-host-removal-delete-patch-candidate.source-parity-mismatch",
    ),
    (
      "host-code-lost",
      "held.macro-only-host-removal-delete-patch-candidate.host-code-or-held-loss",
    ),
    (
      "missing-evidence",
      "held.macro-only-host-removal-delete-patch-candidate.missing-required-evidence",
    ),
  ] {
    let output = get(&run, key);
    assert_eq!(as_str(get(output, "status")), "Held");
    assert_eq!(as_str(get(output, "held-id")), held_id);
    assert!(!as_bool(get(output, "actual-host-removal-patch-candidate")));
  }
}

#[test]
fn delete_runtime_puck_old_host_and_gpl_overclaims_are_held() {
  let run = eval_file(&fixture_path()).unwrap();
  for (key, held_id) in [
    (
      "delete-overclaim",
      "held.macro-only-host-removal-delete-patch-candidate.delete-overclaim",
    ),
    (
      "runtime-overclaim",
      "held.macro-only-host-removal-delete-patch-candidate.runtime-overclaim",
    ),
    (
      "puck-semantic-owner",
      "held.macro-only-host-removal-delete-patch-candidate.p-puck-semantic-owner",
    ),
    (
      "old-host-authority-held",
      "held.macro-only-host-removal-delete-patch-candidate.old-host-authority",
    ),
    (
      "gpl-claim",
      "held.macro-only-host-removal-delete-patch-candidate.gpl-family-dependency",
    ),
  ] {
    let output = get(&run, key);
    assert_eq!(as_str(get(output, "status")), "Held");
    assert_eq!(as_str(get(output, "held-id")), held_id);
    assert_eq!(as_i64(get(output, "delete-ready-target-count")), 0);
    assert!(!as_bool(get(output, "implementation-command")));
  }
}
