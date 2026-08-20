//! Macro-only runtime owner proof receipt.
//!
//! This pins the next bootstrap step after `boot-executed=true`:
//! `macro-only-runtime-owner-booted=true`. The receipt deliberately keeps
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
    .join("fixtures/tesseract-macro-legacy-probe/macro_only_runtime_owner_proof_receipt.px")
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
  let run = eval_file(&fixture_path()).expect("macro-only runtime owner proof receipt");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-macro-only-runtime-owner-proof"
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
    "stdlib/lib/gate/macro-only-runtime-owner-proof.px",
    "fixtures/pnix-query-runtime/macro-only-runtime-owner-proof-owner.px",
    "fixtures/tesseract-macro-legacy-probe/macro_only_runtime_owner_proof_receipt.px",
  ] {
    assert!(repo_root().join(path).is_file(), "missing `{path}`");
  }
}

#[test]
fn constitution_gate_blocks_runtime_owner_overclaims() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "macro-only-runtime-owner-proof"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));

  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "boot-executed-equals-runtime-owner-without-owner-proof",
    "runtime-owner-equals-new-engine-from-zero",
    "runtime-owner-equals-runtime-install",
    "runtime-owner-equals-global-ontology-runtime",
    "runtime-owner-equals-host-removal",
    "runtime-owner-equals-semantic-owner",
    "runtime-owner-equals-delete-ready-targets",
    "runtime-owner-erases-old-host-regression-corpus",
    "runtime-owner-reintroduces-gpl-family-dependency",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
}

#[test]
fn contract_closes_runtime_owner_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let contract = get(&run, "macro-only-runtime-owner-proof-contract");
  assert_eq!(
    as_str(get(contract, "id")),
    "contract.macro-only-runtime-owner-proof.v1"
  );
  assert_eq!(
    as_str(get(contract, "owner")),
    "stdlib.lib.gate.macro-only-runtime-owner-proof"
  );
  assert_eq!(
    as_str(get(contract, "current-status")),
    "macro-only-runtime-owner-proof-present"
  );
  assert_eq!(
    as_str(get(contract, "expected-runtime-owner-scope")),
    "bounded-receipt-trajectory-owner"
  );
  assert_eq!(as_i64(get(contract, "total-tests")), 947);
  assert_eq!(as_i64(get(contract, "source-tracked")), 18177);
  assert_eq!(as_i64(get(contract, "source-indexed")), 18177);
  assert!(as_bool(get(contract, "closes-macro-only-runtime-owner")));
  for key in [
    "closes-boot-execution-proof",
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
fn valid_proof_sets_runtime_owner_without_semantic_host_or_global_runtime() {
  let run = eval_file(&fixture_path()).unwrap();
  let proof = get(&run, "macro-only-runtime-owner-proof");
  assert_eq!(
    as_str(get(proof, "status")),
    "macro-only-runtime-owner-proof-present"
  );
  assert!(as_bool(get(proof, "runtime-owner-proof")));
  assert!(as_bool(get(proof, "boot-executed")));
  assert!(as_bool(get(proof, "macro-only-runtime-owner-booted")));
  assert_eq!(
    as_str(get(proof, "runtime-owner-scope")),
    "bounded-receipt-trajectory-owner"
  );
  assert_eq!(as_i64(get(proof, "total-tests")), 947);
  assert_eq!(as_i64(get(proof, "source-tracked")), 18177);
  assert_eq!(as_i64(get(proof, "source-indexed")), 18177);
  for key in [
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
fn trials_cover_valid_path_and_held_overclaims() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "macro-only-runtime-owner-proof-trials"));
  assert_eq!(trials.len(), 16);
  assert_eq!(
    as_str(get(trials["trial.A.valid-runtime-owner-proof"], "outcome")),
    "macro-only-runtime-owner-proof-present"
  );
  assert!(as_bool(get(
    trials["trial.A.valid-runtime-owner-proof"],
    "macro-only-runtime-owner-booted"
  )));
  assert_eq!(
    as_str(get(trials["trial.B.boot-proof-input"], "outcome")),
    "tesseract-macro-ontology-macro-only-boot-execution-proof"
  );
  assert!(!as_bool(get(
    trials["trial.B.boot-proof-input"],
    "macro-only-runtime-owner-before-proof"
  )));

  for (id, held) in [
    (
      "trial.C.wrong-proof-id",
      "held.macro-only-runtime-owner.proof-id-mismatch",
    ),
    (
      "trial.D.stale-stage",
      "held.macro-only-runtime-owner.stale-current-stage",
    ),
    (
      "trial.E.boot-proof-missing",
      "held.macro-only-runtime-owner.boot-proof-missing",
    ),
    (
      "trial.F.boot-not-executed",
      "held.macro-only-runtime-owner.boot-proof-missing",
    ),
    (
      "trial.G.audit-chain-missing",
      "held.macro-only-runtime-owner.replay-audit-chain-missing",
    ),
    (
      "trial.H.compare-mismatch",
      "held.macro-only-runtime-owner.compare-all-mismatch",
    ),
    (
      "trial.I.source-parity-mismatch",
      "held.macro-only-runtime-owner.source-parity-mismatch",
    ),
    (
      "trial.J.scope-mismatch",
      "held.macro-only-runtime-owner.scope-mismatch",
    ),
    (
      "trial.K.semantic-delta-overclaim",
      "held.macro-only-runtime-owner.semantic-delta-or-held-loss",
    ),
    (
      "trial.L.global-runtime-claim",
      "held.macro-only-runtime-owner.global-runtime-overclaim",
    ),
    (
      "trial.M.host-removal-claim",
      "held.macro-only-runtime-owner.host-removal-overclaim",
    ),
    (
      "trial.N.semantic-owner-claim",
      "held.macro-only-runtime-owner.semantic-owner-overclaim",
    ),
    (
      "trial.O.old-host-authority",
      "held.macro-only-runtime-owner.old-host-authority",
    ),
    (
      "trial.P.gpl-family-dependency",
      "held.macro-only-runtime-owner.gpl-family-dependency",
    ),
  ] {
    let trial = trials[id];
    assert_eq!(as_str(get(trial, "outcome")), "Held");
    assert_eq!(as_str(get(trial, "held-id")), held);
  }
}

#[test]
fn six_layer_fold_records_bounded_runtime_owner_without_collapsing_future_frontiers() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-macro-only-runtime-owner-fold");
  assert_eq!(as_str(get(fold, "mode")), "macro-only-runtime-owner-proof");
  for layer in [
    "surface", "ontology", "semantic", "gate", "runtime", "audit",
  ] {
    assert!(
      as_bool(get_path(fold, &[layer, "visible"])),
      "{layer} hidden"
    );
  }
  assert!(as_bool(get_path(
    fold,
    &["ontology", "runtime-owner-proof"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["ontology", "macro-only-runtime-owner-booted"]
  )));
  assert_eq!(
    as_str(get_path(fold, &["runtime", "runtime-owner-scope"])),
    "bounded-receipt-trajectory-owner"
  );
  for path in [
    &["runtime", "new-engine-from-zero"][..],
    &["runtime", "runtime-install"][..],
    &["runtime", "global-ontology-runtime"][..],
    &["runtime", "host-code-removal-started"][..],
    &["runtime", "host-removal-safe"][..],
    &["semantic", "semantic-owner"][..],
    &["semantic", "old-host-authority"][..],
  ] {
    assert!(!as_bool(get_path(fold, path)), "{path:?} must stay false");
  }
  assert_eq!(
    as_i64(get_path(fold, &["runtime", "delete-ready-target-count"])),
    0
  );
}

#[test]
fn migration_delta_closes_runtime_owner_and_opens_host_semantic_global_frontiers() {
  let run = eval_file(&fixture_path()).unwrap();
  let delta = get(&run, "migration-delta");
  let closes = string_set(get(delta, "closes"));
  assert!(closes.contains("need.bootstrap.macro-only-runtime-owner-proof-after-boot-execution"));
  assert!(closes.contains("need.bootstrap.macro-only-runtime-owner-boot"));
  let not = string_set(get(delta, "does-not-close"));
  assert!(not.contains("need.bootstrap.new-engine-from-zero-proof"));
  assert!(not.contains("need.runtime.global-ontology-install"));
  assert!(not.contains("need.host-removal.host-code-removal-execution"));
  assert!(not.contains("need.semantic-owner.macro-ontology-runtime"));
  let next = string_set(get(delta, "next-required"));
  assert!(next.contains("host-code-removal-execution-proof-after-runtime-owner"));
  assert!(next.contains("semantic-owner-proof-after-runtime-owner"));
  assert!(next.contains("global-runtime-install-proof-after-runtime-owner"));
}

#[test]
fn discoveries_record_d468_through_d475() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 8);
  for expected in [
    "D468.runtime-owner-proof-is-separate-from-boot-execution",
    "D469.runtime-owner-is-bounded-receipt-trajectory-not-semantic-owner",
    "D470.current-cut-compare-count-advances-to-947",
    "D471.source-parity-advances-to-18177",
    "D472.bounded-runtime-owner-scope-blocks-global-runtime-collapse",
    "D473.host-removal-waits-for-runtime-owner-followup-proof",
    "D474.semantic-owner-waits-after-runtime-owner",
    "D475.runtime-owner-proof-blocks-old-host-and-gpl-authority",
  ] {
    let d = discoveries
      .get(expected)
      .unwrap_or_else(|| panic!("missing discovery {expected}"));
    assert_eq!(as_str(get(d, "decision-pressure")), "keep");
    assert!(as_bool(get(d, "scenario-only")));
  }
}

#[test]
fn top_level_state_is_runtime_owner_but_still_non_host_non_semantic() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "replacement-readiness")),
    "macro-only-runtime-owner-proof-present-not-semantic-owner"
  );
  assert!(as_bool(get(&run, "boot-executed")));
  assert!(as_bool(get(&run, "runtime-owner-proof")));
  assert!(as_bool(get(&run, "macro-only-runtime-owner-booted")));
  for key in [
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
