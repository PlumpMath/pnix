//! Macro-only host-removal execution proof receipt.
//!
//! This pins the next bootstrap step after bounded semantic owner proof:
//! host-removal execution is now an explicit gate shape, while actual deletion,
//! delete-ready targets, global runtime, runtime install, and new-engine-from-zero
//! remain false.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../..")
}

fn fixture_path() -> PathBuf {
  repo_root().join(
    "fixtures/tesseract-macro-legacy-probe/macro_only_host_removal_execution_proof_receipt.px",
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
  let run = eval_file(&fixture_path()).expect("host-removal execution proof receipt");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-macro-only-host-removal-execution-proof"
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
    "stdlib/lib/gate/macro-only-host-removal-execution-proof.px",
    "fixtures/pnix-query-runtime/macro-only-host-removal-execution-proof-owner.px",
    "fixtures/tesseract-macro-legacy-probe/macro_only_host_removal_execution_proof_receipt.px",
  ] {
    assert!(repo_root().join(path).is_file(), "missing `{path}`");
  }
}

#[test]
fn constitution_gate_blocks_host_removal_execution_overclaims() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "macro-only-host-removal-execution-proof"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "semantic-owner-equals-host-code-deletion",
    "target-specific-delete-proof-equals-delete-ready",
    "host-removal-execution-proof-equals-implementation-command",
    "host-removal-execution-proof-equals-global-runtime-install",
    "host-removal-execution-proof-equals-new-engine-from-zero",
    "old-host-code-authorizes-its-own-removal",
    "stale-p-puck-report-authorizes-delete-patch",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
}

#[test]
fn contract_closes_execution_gate_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let contract = get(&run, "host-removal-execution-proof-contract");
  assert_eq!(
    as_str(get(contract, "id")),
    "contract.macro-only-host-removal-execution-proof.v1"
  );
  assert_eq!(
    as_str(get(contract, "owner")),
    "stdlib.lib.gate.macro-only-host-removal-execution-proof"
  );
  assert_eq!(
    as_str(get(contract, "current-status")),
    "macro-only-host-removal-execution-proof-present"
  );
  assert_eq!(as_i64(get(contract, "target-count")), 5);
  assert_eq!(as_i64(get(contract, "execution-plan-target-count")), 5);
  assert_eq!(as_i64(get(contract, "total-tests")), 981);
  assert_eq!(as_i64(get(contract, "source-tracked")), 18187);
  assert_eq!(as_i64(get(contract, "source-indexed")), 18187);
  assert!(as_bool(get(
    contract,
    "closes-host-removal-execution-proof"
  )));
  for key in [
    "closes-host-removal-authorization",
    "closes-actual-host-removal-patch",
    "closes-delete-ready-targets",
    "closes-global-runtime",
    "closes-new-engine-from-zero",
  ] {
    assert!(!as_bool(get(contract, key)), "`{key}` must stay false");
  }
}

#[test]
fn valid_proof_sets_execution_gate_without_delete_or_global_runtime() {
  let run = eval_file(&fixture_path()).unwrap();
  let proof = get(&run, "macro-only-host-removal-execution-proof");
  assert_eq!(
    as_str(get(proof, "status")),
    "macro-only-host-removal-execution-proof-present"
  );
  assert!(as_bool(get(proof, "host-removal-execution-proof")));
  assert!(as_bool(get(proof, "host-removal-execution-gate-present")));
  assert!(!as_bool(get(proof, "host-removal-execution-authorized")));
  assert!(as_bool(get(proof, "semantic-owner")));
  assert_eq!(
    as_str(get(proof, "semantic-owner-scope")),
    "bounded-generated-ontology-semantic-owner"
  );
  assert!(as_bool(get(proof, "fresh-puck-before-delete-required")));
  assert!(!as_bool(get(proof, "fresh-puck-before-delete")));
  assert!(as_bool(get(proof, "old-host-code-still-present")));
  assert_eq!(as_i64(get(proof, "delete-ready-target-count")), 0);
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
    assert!(!as_bool(get(proof, key)), "`{key}` must stay false");
  }
}

#[test]
fn trials_cover_valid_path_inputs_and_held_overclaims() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "host-removal-execution-proof-trials"));
  assert_eq!(trials.len(), 15);
  assert_eq!(
    as_str(get(
      trials["trial.A.valid-host-removal-execution-proof"],
      "outcome"
    )),
    "macro-only-host-removal-execution-proof-present"
  );
  assert!(!as_bool(get(
    trials["trial.A.valid-host-removal-execution-proof"],
    "host-removal-execution-authorized"
  )));
  assert_eq!(
    as_str(get(trials["trial.B.semantic-owner-input"], "outcome")),
    "tesseract-macro-ontology-macro-only-semantic-owner-proof"
  );
  assert_eq!(
    as_str(get(trials["trial.C.target-delete-input"], "outcome")),
    "tesseract-macro-ontology-macro-only-target-specific-delete-proof"
  );

  for (id, held) in [
    (
      "trial.D.wrong-proof-id",
      "held.macro-only-host-removal-execution.proof-id-mismatch",
    ),
    (
      "trial.E.stale-stage",
      "held.macro-only-host-removal-execution.stale-current-stage",
    ),
    (
      "trial.F.semantic-owner-missing",
      "held.macro-only-host-removal-execution.semantic-owner-missing",
    ),
    (
      "trial.G.target-proof-missing",
      "held.macro-only-host-removal-execution.host-removal-map-or-target-proof-missing",
    ),
    (
      "trial.H.target-evidence-missing",
      "held.macro-only-host-removal-execution.missing-required-evidence",
    ),
    (
      "trial.I.compare-mismatch",
      "held.macro-only-host-removal-execution.compare-all-mismatch",
    ),
    (
      "trial.J.fresh-puck-boundary",
      "held.macro-only-host-removal-execution.fresh-puck-boundary",
    ),
    (
      "trial.K.host-code-lost",
      "held.macro-only-host-removal-execution.host-code-or-held-loss",
    ),
    (
      "trial.L.global-runtime-claim",
      "held.macro-only-host-removal-execution.global-runtime-overclaim",
    ),
    (
      "trial.M.deletion-overclaim",
      "held.macro-only-host-removal-execution.deletion-overclaim",
    ),
    (
      "trial.N.old-host-authority",
      "held.macro-only-host-removal-execution.old-host-authority",
    ),
    (
      "trial.O.gpl-family-dependency",
      "held.macro-only-host-removal-execution.gpl-family-dependency",
    ),
  ] {
    let trial = trials[id];
    assert_eq!(as_str(get(trial, "outcome")), "Held");
    assert_eq!(as_str(get(trial, "held-id")), held);
  }
}

#[test]
fn six_layer_fold_records_execution_gate_without_collapsing_delete() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-host-removal-execution-fold");
  assert_eq!(
    as_str(get(fold, "mode")),
    "macro-only-host-removal-execution-proof"
  );
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
    &["ontology", "host-removal-execution-proof"]
  )));
  assert_eq!(
    as_i64(get_path(fold, &["ontology", "execution-plan-target-count"])),
    5
  );
  assert!(as_bool(get_path(
    fold,
    &[
      "semantic",
      "host-removal-is-gated-execution-not-delete-command"
    ]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["runtime", "host-removal-execution-authorized"]
  )));
  for path in [
    &["runtime", "runtime-install"][..],
    &["runtime", "global-ontology-runtime"][..],
    &["runtime", "host-code-removal-started"][..],
    &["runtime", "host-removal-safe"][..],
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
fn execution_plan_targets_stay_protected_not_ready() {
  let run = eval_file(&fixture_path()).unwrap();
  let proof = get(&run, "macro-only-host-removal-execution-proof");
  let targets = as_list(get(proof, "execution-plan-targets"));
  assert_eq!(targets.len(), 5);
  for target in targets {
    assert!(as_bool(get(target, "target-specific-proof-present")));
    assert!(as_bool(get(target, "semantic-replacement-owner-present")));
    assert!(as_bool(get(target, "protected-before-delete-execution")));
    assert_eq!(
      as_str(get(target, "execution-gate")),
      "fresh-puck-before-host-removal-execution"
    );
    assert!(!as_bool(get(target, "delete-ready")));
    assert!(!as_bool(get(target, "remove-now")));
    assert!(!as_bool(get(target, "host-code-removal-started")));
  }
}

#[test]
fn migration_delta_closes_execution_proof_and_opens_delete_patch_frontier() {
  let run = eval_file(&fixture_path()).unwrap();
  let delta = get(&run, "migration-delta");
  let closes = string_set(get(delta, "closes"));
  assert!(closes.contains("need.host-removal.host-code-removal-execution"));
  assert!(closes.contains("host-code-removal-execution-proof-after-semantic-owner"));
  let not = string_set(get(delta, "does-not-close"));
  assert!(not.contains("need.host-removal.fresh-puck-before-delete"));
  assert!(not.contains("need.host-removal.actual-delete-patch"));
  assert!(not.contains("need.host-removal.delete-ready-targets"));
  assert!(not.contains("need.runtime.global-ontology-install"));
  let next = string_set(get(delta, "next-required"));
  assert!(next.contains("fresh-puck-before-host-removal-execution"));
  assert!(next.contains("actual-host-removal-patch-after-fresh-puck"));
  assert!(next.contains("global-runtime-install-proof-after-semantic-owner"));
}

#[test]
fn discoveries_record_d484_through_d491() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 8);
  for expected in [
    "D484.host-removal-execution-proof-is-a-gate-not-delete",
    "D485.semantic-owner-is-load-bearing-for-host-removal-execution",
    "D486.target-specific-delete-proof-becomes-execution-input-not-delete-ready",
    "D487.current-cut-measurement-advances-to-981-and-18187",
    "D488.fresh-puck-before-delete-is-required-but-not-manufactured",
    "D489.old-host-code-and-negative-held-stay-until-delete-patch",
    "D490.host-removal-execution-blocks-global-runtime-and-old-host-authority",
    "D491.host-removal-execution-keeps-gpl-and-implementation-command-false",
  ] {
    let d = discoveries
      .get(expected)
      .unwrap_or_else(|| panic!("missing discovery {expected}"));
    assert_eq!(as_str(get(d, "decision-pressure")), "keep");
    assert!(as_bool(get(d, "scenario-only")));
  }
}

#[test]
fn top_level_state_is_execution_proof_but_still_non_delete() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "replacement-readiness")),
    "macro-only-host-removal-execution-proof-present-not-delete"
  );
  assert!(as_bool(get(&run, "semantic-owner")));
  assert!(as_bool(get(&run, "target-specific-delete-proof-present")));
  assert!(as_bool(get(&run, "host-removal-execution-proof-present")));
  assert!(as_bool(get(&run, "host-removal-execution-proof")));
  assert!(as_bool(get(&run, "host-removal-execution-gate-present")));
  assert!(!as_bool(get(&run, "host-removal-execution-authorized")));
  assert!(as_bool(get(&run, "old-host-code-still-present")));
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
    assert!(!as_bool(get(&run, key)), "`{key}` must stay false");
  }
  assert_eq!(as_i64(get(&run, "delete-ready-target-count")), 0);
}
