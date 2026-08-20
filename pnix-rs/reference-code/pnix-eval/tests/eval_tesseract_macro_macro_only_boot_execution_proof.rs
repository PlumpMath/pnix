//! Macro-only boot execution proof receipt.
//!
//! This pins the first positive macro-only boot trajectory state:
//! `boot-executed=true`. The receipt deliberately keeps runtime ownership,
//! semantic ownership, host removal, delete-ready targets, global runtime, and
//! new-engine-from-zero false.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../..")
}

fn fixture_path() -> PathBuf {
  repo_root()
    .join("fixtures/tesseract-macro-legacy-probe/macro_only_boot_execution_proof_receipt.px")
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

fn get_path<'a>(root: &'a Value, path: &[&str]) -> &'a Value {
  let mut cur = root;
  for key in path {
    cur = get(cur, key);
  }
  cur
}

fn string_set(v: &Value) -> BTreeSet<&str> {
  as_list(v).iter().map(as_str).collect()
}

fn attrs_by_id<'a>(items: &'a Value) -> BTreeMap<&'a str, &'a Value> {
  as_list(items)
    .iter()
    .map(|item| (as_str(get(item, "id")), item))
    .collect()
}

#[test]
fn marker_and_owner_surfaces_are_pinned() {
  let run = eval_file(&fixture_path()).expect("macro-only boot execution proof receipt");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-macro-only-boot-execution-proof"
  );
  assert_eq!(
    as_str(get(&run, "truth-owner")),
    "project-wiki/maps/tesseract-macro-ontology-discovery-ledger.md"
  );
  assert_eq!(
    as_str(get(&run, "constitution-owner")),
    "stdlib/lib/gate/tesseract-constitution.px"
  );
  for path in [
    "stdlib/lib/gate/macro-only-boot-execution-proof.px",
    "fixtures/pnix-query-runtime/macro-only-boot-execution-proof-owner.px",
    "fixtures/tesseract-macro-legacy-probe/macro_only_boot_execution_proof_receipt.px",
  ] {
    assert!(repo_root().join(path).is_file(), "missing `{path}`");
  }
}

#[test]
fn constitution_gate_blocks_boot_proof_overclaims() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "macro-only-boot-execution-proof"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));

  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "boot-executed-equals-macro-only-runtime-owner",
    "boot-executed-equals-new-engine-from-zero",
    "boot-executed-equals-runtime-install",
    "boot-executed-equals-global-ontology-runtime",
    "boot-executed-equals-host-removal",
    "boot-executed-equals-semantic-owner",
    "full-audit-green-equals-runtime-owner",
    "p-puck-telemetry-equals-semantic-owner",
    "source-parity-equals-host-delete",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
}

#[test]
fn contract_closes_boot_execution_proof_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let contract = get(&run, "macro-only-boot-execution-proof-contract");
  assert_eq!(
    as_str(get(contract, "id")),
    "contract.macro-only-boot-execution-proof.v1"
  );
  assert_eq!(
    as_str(get(contract, "owner")),
    "stdlib.lib.gate.macro-only-boot-execution-proof"
  );
  assert_eq!(
    as_str(get(contract, "current-status")),
    "macro-only-boot-execution-proof-present"
  );
  assert_eq!(as_i64(get(contract, "total-tests")), 931);
  assert_eq!(as_i64(get(contract, "source-tracked")), 18172);
  assert_eq!(as_i64(get(contract, "source-indexed")), 18172);
  assert_eq!(as_i64(get(contract, "p-puck-duration-ms")), 4934);
  assert!(as_bool(get(contract, "closes-boot-execution-proof")));
  for key in [
    "closes-macro-only-runtime-owner",
    "closes-new-engine-from-zero",
    "closes-runtime-install",
    "closes-global-ontology-runtime",
    "closes-host-removal",
    "closes-delete-ready-targets",
    "closes-semantic-owner-proof",
  ] {
    assert!(!as_bool(get(contract, key)), "`{key}` must stay false");
  }
}

#[test]
fn valid_proof_sets_boot_executed_but_not_runtime_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  let proof = get(&run, "macro-only-boot-execution-proof");
  assert_eq!(
    as_str(get(proof, "status")),
    "macro-only-boot-execution-proof-present"
  );
  assert!(as_bool(get(proof, "boot-execution-proof")));
  assert!(as_bool(get(proof, "boot-executed")));
  assert!(as_bool(get(proof, "full-current-receipt-audit-input")));
  assert!(as_bool(get(proof, "bounded-replay-input")));
  assert!(as_bool(get(proof, "post-replay-p-puck-input")));
  assert_eq!(as_i64(get(proof, "total-tests")), 931);
  assert_eq!(as_i64(get(proof, "source-tracked")), 18172);
  assert_eq!(as_i64(get(proof, "source-indexed")), 18172);
  assert_eq!(
    as_str(get(proof, "semantic-delta-status")),
    "empty-or-held-only"
  );
  assert!(as_bool(get(proof, "negative-held-retained")));
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
    assert!(!as_bool(get(proof, key)), "`{key}` must stay false");
  }
  assert_eq!(as_i64(get(proof, "delete-ready-target-count")), 0);
}

#[test]
fn trials_cover_valid_path_and_all_held_overclaims() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "macro-only-boot-execution-proof-trials"));
  assert_eq!(trials.len(), 16);
  assert_eq!(
    as_str(get(trials["trial.A.valid-boot-execution-proof"], "outcome")),
    "macro-only-boot-execution-proof-present"
  );
  assert!(as_bool(get(
    trials["trial.A.valid-boot-execution-proof"],
    "boot-executed"
  )));
  assert_eq!(
    as_str(get(trials["trial.B.full-current-audit-input"], "outcome")),
    "tesseract-macro-ontology-macro-only-full-current-receipt-audit"
  );
  for (id, held) in [
    (
      "trial.C.wrong-proof-id",
      "held.macro-only-boot-proof.proof-id-mismatch",
    ),
    (
      "trial.D.stale-stage",
      "held.macro-only-boot-proof.stale-current-stage",
    ),
    (
      "trial.E.full-audit-missing",
      "held.macro-only-boot-proof.full-audit-missing",
    ),
    (
      "trial.F.replay-puck-missing",
      "held.macro-only-boot-proof.replay-or-puck-missing",
    ),
    (
      "trial.G.runner-not-ready",
      "held.macro-only-boot-proof.runner-not-ready",
    ),
    (
      "trial.H.compare-mismatch",
      "held.macro-only-boot-proof.compare-all-mismatch",
    ),
    (
      "trial.I.source-parity-mismatch",
      "held.macro-only-boot-proof.source-parity-mismatch",
    ),
    (
      "trial.J.puck-telemetry-mismatch",
      "held.macro-only-boot-proof.p-puck-telemetry-mismatch",
    ),
    (
      "trial.K.semantic-delta-overclaim",
      "held.macro-only-boot-proof.semantic-delta-or-held-loss",
    ),
    (
      "trial.L.runtime-owner-claim",
      "held.macro-only-boot-proof.runtime-owner-overclaim",
    ),
    (
      "trial.M.host-removal-claim",
      "held.macro-only-boot-proof.host-removal-overclaim",
    ),
    (
      "trial.N.semantic-owner-claim",
      "held.macro-only-boot-proof.semantic-owner-overclaim",
    ),
    (
      "trial.O.old-host-authority",
      "held.macro-only-boot-proof.old-host-authority",
    ),
    (
      "trial.P.gpl-family-dependency",
      "held.macro-only-boot-proof.gpl-family-dependency",
    ),
  ] {
    assert_eq!(as_str(get(trials[id], "outcome")), "Held");
    assert_eq!(as_str(get(trials[id], "held-id")), held);
  }
}

#[test]
fn six_layer_fold_records_boot_without_owner_or_delete_collapse() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-macro-only-boot-proof-fold");
  for layer in [
    "surface", "ontology", "semantic", "gate", "runtime", "audit",
  ] {
    assert!(
      as_bool(get(get(fold, layer), "visible")),
      "layer `{layer}` invisible"
    );
  }
  assert!(as_bool(get_path(
    fold,
    &["ontology", "boot-execution-proof"]
  )));
  assert!(as_bool(get_path(fold, &["ontology", "boot-executed"])));
  assert!(as_bool(get_path(
    fold,
    &["ontology", "full-current-receipt-audit-input"]
  )));
  assert_eq!(
    as_i64(get_path(fold, &["ontology", "compare-all-total-tests"])),
    931
  );
  assert!(!as_bool(get_path(fold, &["semantic", "semantic-owner"])));
  assert!(!as_bool(get_path(
    fold,
    &["runtime", "macro-only-runtime-owner-booted"]
  )));
  assert_eq!(
    as_i64(get_path(fold, &["runtime", "delete-ready-target-count"])),
    0
  );
  assert_eq!(
    as_i64(get_path(fold, &["audit", "p-puck-duration-ms"])),
    4934
  );
}

#[test]
fn migration_delta_closes_boot_proof_and_opens_next_frontiers() {
  let run = eval_file(&fixture_path()).unwrap();
  let delta = get(&run, "migration-delta");
  let closes = string_set(get(delta, "closes"));
  assert!(closes
    .contains("need.bootstrap.macro-only-boot-execution-proof-after-full-current-receipt-audit"));
  let not = string_set(get(delta, "does-not-close"));
  assert!(not.contains("need.bootstrap.macro-only-runtime-owner-boot"));
  assert!(not.contains("need.bootstrap.new-engine-from-zero-proof"));
  assert!(not.contains("need.host-removal.host-code-removal-execution"));
  assert!(not.contains("need.semantic-owner.macro-ontology-runtime"));
  let next = string_set(get(delta, "next-required"));
  assert!(next.contains("macro-only-runtime-owner-proof-after-boot-execution"));
  assert!(next.contains("host-code-removal-execution-proof-after-successful-boot"));
  assert!(next.contains("semantic-owner-proof-after-runtime-owner"));
  assert!(next.contains("lift-query-emit-runtime-owner-or-host-removal-proof"));
}

#[test]
fn discoveries_record_d460_through_d467() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 8);
  for expected in [
    "D460.boot-execution-proof-is-separate-from-full-audit",
    "D461.boot-executed-is-receipt-trajectory-not-runtime-owner",
    "D462.current-cut-compare-count-advances-to-931",
    "D463.source-parity-advances-to-18172",
    "D464.boot-proof-preserves-negative-held-and-empty-semantic-delta",
    "D465.boot-proof-opens-runtime-owner-and-host-removal-frontiers",
    "D466.boot-proof-blocks-old-host-and-gpl-authority",
    "D467.boot-proof-is-first-positive-boot-state-with-zero-delete-targets",
  ] {
    let d = discoveries
      .get(expected)
      .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
    assert!(as_bool(get(d, "scenario-only")));
    assert_eq!(as_str(get(d, "decision-pressure")), "keep");
  }
}

#[test]
fn top_level_state_is_boot_executed_but_still_non_runtime_non_delete() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "replacement-readiness")),
    "macro-only-boot-execution-proof-present-not-runtime-owner"
  );
  assert!(as_bool(get(&run, "full-current-receipt-audit")));
  assert!(as_bool(get(
    &run,
    "macro-only-boot-execution-proof-present"
  )));
  assert!(as_bool(get(&run, "boot-execution-proof")));
  assert!(as_bool(get(&run, "boot-executed")));
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
    assert!(!as_bool(get(&run, key)), "`{key}` must stay false");
  }
  assert_eq!(as_i64(get(&run, "delete-ready-target-count")), 0);
}
