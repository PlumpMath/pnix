//! Host-removal delete patch candidate.
//!
//! This pins the next host-removal migration step after slow-path repeat
//! clearance: the old-host deletion patch becomes an exact candidate object,
//! but not an applied diff, delete-ready state, or implementation command.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../..")
}

fn fixture_path() -> PathBuf {
  repo_root().join(
    "fixtures/tesseract-macro-legacy-probe/macro_only_host_removal_delete_patch_candidate_receipt.px",
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
  let run = eval_file(&fixture_path()).expect("delete patch candidate receipt");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-macro-only-host-removal-delete-patch-candidate"
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
    "stdlib/lib/gate/macro-only-host-removal-delete-patch-candidate.px",
    "fixtures/pnix-query-runtime/macro-only-host-removal-delete-patch-candidate-owner.px",
    "fixtures/tesseract-macro-legacy-probe/macro_only_host_removal_delete_patch_candidate_receipt.px",
  ] {
    assert!(repo_root().join(path).is_file(), "missing `{path}`");
  }
}

#[test]
fn constitution_gate_blocks_candidate_collapse() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitutionGate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "macro-only-host-removal-delete-patch-candidate"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "delete-patch-candidate-equals-delete-ready",
    "delete-patch-candidate-equals-remove-now",
    "delete-patch-candidate-equals-host-code-removal-started",
    "delete-patch-candidate-equals-implementation-command",
    "delete-patch-candidate-equals-global-runtime-install",
    "p-puck-audit-equals-semantic-owner",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
}

#[test]
fn contract_closes_candidate_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let contract = get(&run, "host-removal-delete-patch-candidate-contract");
  assert_eq!(
    as_str(get(contract, "id")),
    "contract.macro-only-host-removal-delete-patch-candidate.v1"
  );
  assert_eq!(
    as_str(get(contract, "current-status")),
    "macro-only-host-removal-delete-patch-candidate-present"
  );
  assert_eq!(as_i64(get(contract, "target-count")), 5);
  assert_eq!(as_i64(get(contract, "patch-candidate-target-count")), 5);
  assert_eq!(as_i64(get(contract, "total-tests")), 1035);
  assert!(as_bool(get(
    contract,
    "closes-actual-delete-patch-candidate"
  )));
  for key in [
    "closes-delete-ready",
    "closes-host-code-removal-started",
    "closes-implementation-command",
    "closes-global-runtime",
    "closes-runtime-api-flattening",
    "closes-meaning-db",
  ] {
    assert!(!as_bool(get(contract, key)), "`{key}` must stay false");
  }
}

#[test]
fn proof_records_patch_candidate_without_delete_ready() {
  let run = eval_file(&fixture_path()).unwrap();
  let proof = get(&run, "host-removal-delete-patch-candidate-proof");
  assert_eq!(
    as_str(get(proof, "status")),
    "macro-only-host-removal-delete-patch-candidate-present"
  );
  assert!(as_bool(get(proof, "actual-host-removal-patch-candidate")));
  assert!(as_bool(get(proof, "delete-patch-candidate-proof")));
  assert_eq!(as_list(get(proof, "patch-candidate-targets")).len(), 5);
  assert_eq!(as_i64(get(proof, "delete-ready-target-count")), 0);
  assert_eq!(as_i64(get(proof, "repeat-duration-ms")), 551);
  assert_eq!(as_i64(get(proof, "total-tests")), 1035);
  assert_eq!(as_i64(get(proof, "source-tracked")), 18202);
  assert_eq!(as_i64(get(proof, "source-indexed")), 18202);
  for key in [
    "actual-host-removal-patch-authorized",
    "delete-ready",
    "remove-now",
    "host-code-removal-started",
    "implementation-command",
    "global-ontology-runtime",
    "p-puck-is-semantic-owner",
  ] {
    assert!(!as_bool(get(proof, key)), "`{key}` must stay false");
  }
}

#[test]
fn trials_cover_candidate_and_all_held_boundaries() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "host-removal-delete-patch-candidate-trials"));
  assert_eq!(trials.len(), 14);
  assert_eq!(
    as_str(get(
      trials["trial.A.valid-delete-patch-candidate"],
      "outcome"
    )),
    "macro-only-host-removal-delete-patch-candidate-present"
  );
  assert_eq!(
    as_str(get(trials["trial.B.slow-path-repeat-input"], "outcome")),
    "tesseract-macro-ontology-macro-only-host-removal-slow-path-repeat-proof"
  );
  for (id, held) in [
    (
      "trial.C.wrong-proof-id",
      "held.macro-only-host-removal-delete-patch-candidate.proof-id-mismatch",
    ),
    (
      "trial.D.stale-stage",
      "held.macro-only-host-removal-delete-patch-candidate.stale-current-stage",
    ),
    (
      "trial.E.slow-path-missing",
      "held.macro-only-host-removal-delete-patch-candidate.slow-path-repeat-missing",
    ),
    (
      "trial.F.target-evidence-missing",
      "held.macro-only-host-removal-delete-patch-candidate.missing-required-evidence",
    ),
    (
      "trial.G.compare-mismatch",
      "held.macro-only-host-removal-delete-patch-candidate.compare-all-mismatch",
    ),
    (
      "trial.H.source-mismatch",
      "held.macro-only-host-removal-delete-patch-candidate.source-parity-mismatch",
    ),
    (
      "trial.I.host-code-lost",
      "held.macro-only-host-removal-delete-patch-candidate.host-code-or-held-loss",
    ),
    (
      "trial.J.delete-overclaim",
      "held.macro-only-host-removal-delete-patch-candidate.delete-overclaim",
    ),
    (
      "trial.K.runtime-overclaim",
      "held.macro-only-host-removal-delete-patch-candidate.runtime-overclaim",
    ),
    (
      "trial.L.p-puck-semantic-owner",
      "held.macro-only-host-removal-delete-patch-candidate.p-puck-semantic-owner",
    ),
    (
      "trial.M.old-host-authority",
      "held.macro-only-host-removal-delete-patch-candidate.old-host-authority",
    ),
    (
      "trial.N.gpl-family-dependency",
      "held.macro-only-host-removal-delete-patch-candidate.gpl-family-dependency",
    ),
  ] {
    assert_eq!(as_str(get(trials[id], "outcome")), "Held");
    assert_eq!(as_str(get(trials[id], "held-id")), held);
  }
}

#[test]
fn six_layer_fold_records_candidate_without_runtime_or_db() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-host-removal-delete-patch-candidate-fold");
  for layer in [
    "surface", "ontology", "semantic", "gate", "runtime", "audit",
  ] {
    assert!(
      as_bool(get(get(fold, layer), "visible")),
      "layer `{layer}` hidden"
    );
  }
  assert!(as_bool(get_path(
    fold,
    &["ontology", "actual-host-removal-patch-candidate"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "candidate-is-not-application"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "runtime-api-flattening-deferred"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "meaning-db-deferred"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["gate", "blocked-delete-ready-collapse"]
  )));
  assert_eq!(
    as_i64(get_path(fold, &["runtime", "delete-ready-target-count"])),
    0
  );
  assert!(!as_bool(get_path(
    fold,
    &["runtime", "host-code-removal-started"]
  )));
}

#[test]
fn migration_delta_closes_actual_patch_candidate_not_delete_ready() {
  let run = eval_file(&fixture_path()).unwrap();
  let delta = get(&run, "migrationDelta");
  let closes = string_set(get(delta, "closes"));
  assert!(closes.contains("need.host-removal.actual-delete-patch"));

  let not = string_set(get(delta, "does-not-close"));
  assert!(not.contains("need.host-removal.fresh-puck-before-delete-as-delete-ready"));
  assert!(not.contains("need.host-removal.delete-ready-targets"));
  assert!(not.contains("need.domain-runtime-api-flattening-after-semantic-owner"));
  assert!(not.contains("need.stdlib.meaning-db"));

  let next = string_set(get(delta, "next-required"));
  assert!(next.contains("fresh-puck-before-delete-as-delete-ready"));
  assert!(next.contains("delete-ready-targets-after-delete-candidate"));
}

#[test]
fn discoveries_record_d508_through_d515() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 8);
  for expected in [
    "D508.delete-patch-is-candidate-object-not-applied-diff",
    "D509.each-delete-target-requires-hunk-and-replay-rollback-regression-bindings",
    "D510.delete-candidate-and-delete-ready-are-separate-states",
    "D511.old-host-code-remains-present-through-candidate-stage",
    "D512.p-puck-audit-is-evidence-not-semantic-owner",
    "D513.compare-and-source-parity-become-delete-candidate-inputs",
    "D514.runtime-flattening-and-meaning-db-stay-deferred",
    "D515.host-removal-frontier-shifts-to-delete-ready-proof",
  ] {
    assert!(
      discoveries.contains_key(expected),
      "missing discovery `{expected}`"
    );
  }
}

#[test]
fn top_level_state_is_candidate_not_delete_ready() {
  let run = eval_file(&fixture_path()).unwrap();
  assert!(as_bool(get(
    &run,
    "host-removal-delete-patch-candidate-proof-present"
  )));
  assert!(as_bool(get(&run, "actual-host-removal-patch-candidate")));
  assert!(!as_bool(get(&run, "actual-host-removal-patch-authorized")));
  assert_eq!(as_i64(get(&run, "delete-ready-target-count")), 0);
  for key in [
    "delete-ready",
    "remove-now",
    "host-code-removal-started",
    "implementation-command",
    "runtime-install",
    "global-ontology-runtime",
    "new-engine-from-zero",
    "p-puck-is-semantic-owner",
    "old-host-authority",
    "gpl-family-dependencies",
    "runtime-api-flattening",
    "meaning-db",
  ] {
    assert!(!as_bool(get(&run, key)), "`{key}` must stay false");
  }
  assert_eq!(
    as_str(get(&run, "replacement-readiness")),
    "host-removal-delete-patch-candidate-present-not-delete-ready"
  );
}

#[test]
fn patch_candidate_targets_are_exact_and_inert() {
  let run = eval_file(&fixture_path()).unwrap();
  let targets = as_list(get(&run, "patch-candidate-targets"));
  assert_eq!(targets.len(), 5);
  for target in targets {
    assert!(as_bool(get(target, "delete-candidate")));
    assert_eq!(
      as_str(get(target, "patch-action")),
      "candidate-delete-old-host-authority"
    );
    assert!(!as_bool(get(target, "delete-ready")));
    assert!(!as_bool(get(target, "remove-now")));
    assert!(!as_bool(get(target, "host-code-removal-started")));
    assert!(!as_bool(get(target, "implementation-command")));
  }
}
