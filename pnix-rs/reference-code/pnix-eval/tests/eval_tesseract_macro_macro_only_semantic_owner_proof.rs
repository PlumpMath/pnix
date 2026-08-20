//! Macro-only semantic owner proof receipt.
//!
//! This pins the next bootstrap step after the bounded runtime owner proof:
//! `semantic-owner=true` for the generated ontology meaning surface. The
//! receipt deliberately keeps host removal, delete-ready targets, global
//! runtime, runtime install, and new-engine-from-zero false.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../..")
}

fn fixture_path() -> PathBuf {
  repo_root()
    .join("fixtures/tesseract-macro-legacy-probe/macro_only_semantic_owner_proof_receipt.px")
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
  let run = eval_file(&fixture_path()).expect("macro-only semantic owner proof receipt");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-macro-only-semantic-owner-proof"
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
    "stdlib/lib/gate/macro-only-semantic-owner-proof.px",
    "fixtures/pnix-query-runtime/macro-only-semantic-owner-proof-owner.px",
    "fixtures/tesseract-macro-legacy-probe/macro_only_semantic_owner_proof_receipt.px",
  ] {
    assert!(repo_root().join(path).is_file(), "missing `{path}`");
  }
}

#[test]
fn constitution_gate_blocks_semantic_owner_overclaims() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "macro-only-semantic-owner-proof"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));

  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "runtime-owner-equals-semantic-owner-without-proof",
    "semantic-owner-equals-new-engine-from-zero",
    "semantic-owner-equals-runtime-install",
    "semantic-owner-equals-global-ontology-runtime",
    "semantic-owner-equals-host-removal",
    "semantic-owner-equals-delete-ready-targets",
    "semantic-owner-erases-old-host-regression-corpus",
    "semantic-owner-reintroduces-gpl-family-dependency",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
}

#[test]
fn contract_closes_semantic_owner_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let contract = get(&run, "macro-only-semantic-owner-proof-contract");
  assert_eq!(
    as_str(get(contract, "id")),
    "contract.macro-only-semantic-owner-proof.v1"
  );
  assert_eq!(
    as_str(get(contract, "owner")),
    "stdlib.lib.gate.macro-only-semantic-owner-proof"
  );
  assert_eq!(
    as_str(get(contract, "current-status")),
    "macro-only-semantic-owner-proof-present"
  );
  assert_eq!(
    as_str(get(contract, "expected-semantic-owner-scope")),
    "bounded-generated-ontology-semantic-owner"
  );
  assert_eq!(as_i64(get(contract, "total-tests")), 963);
  assert_eq!(as_i64(get(contract, "source-tracked")), 18182);
  assert_eq!(as_i64(get(contract, "source-indexed")), 18182);
  assert!(as_bool(get(contract, "closes-semantic-owner")));
  for key in [
    "closes-runtime-owner-proof",
    "closes-new-engine-from-zero",
    "closes-runtime-install",
    "closes-global-ontology-runtime",
    "closes-host-removal",
    "closes-delete-ready-targets",
  ] {
    assert!(!as_bool(get(contract, key)), "`{key}` must stay false");
  }
}

#[test]
fn valid_proof_sets_semantic_owner_without_host_or_global_runtime() {
  let run = eval_file(&fixture_path()).unwrap();
  let proof = get(&run, "macro-only-semantic-owner-proof");
  assert_eq!(
    as_str(get(proof, "status")),
    "macro-only-semantic-owner-proof-present"
  );
  assert!(as_bool(get(proof, "semantic-owner-proof")));
  assert!(as_bool(get(proof, "semantic-owner")));
  assert_eq!(
    as_str(get(proof, "semantic-owner-scope")),
    "bounded-generated-ontology-semantic-owner"
  );
  assert!(as_bool(get(proof, "runtime-owner-proof")));
  assert!(as_bool(get(proof, "macro-only-runtime-owner-booted")));
  assert!(as_bool(get(proof, "boot-executed")));
  assert_eq!(as_i64(get(proof, "total-tests")), 963);
  assert_eq!(as_i64(get(proof, "source-tracked")), 18182);
  assert_eq!(as_i64(get(proof, "source-indexed")), 18182);
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
    assert!(!as_bool(get(proof, key)), "`{key}` must stay false");
  }
  assert_eq!(as_i64(get(proof, "delete-ready-target-count")), 0);
}

#[test]
fn trials_cover_valid_path_and_held_overclaims() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "macro-only-semantic-owner-proof-trials"));
  assert_eq!(trials.len(), 17);
  assert_eq!(
    as_str(get(trials["trial.A.valid-semantic-owner-proof"], "outcome")),
    "macro-only-semantic-owner-proof-present"
  );
  assert!(as_bool(get(
    trials["trial.A.valid-semantic-owner-proof"],
    "semantic-owner"
  )));
  assert_eq!(
    as_str(get(trials["trial.B.runtime-owner-input"], "outcome")),
    "tesseract-macro-ontology-macro-only-runtime-owner-proof"
  );
  assert!(!as_bool(get(
    trials["trial.B.runtime-owner-input"],
    "semantic-owner-before-proof"
  )));

  for (id, held) in [
    (
      "trial.C.wrong-proof-id",
      "held.macro-only-semantic-owner.proof-id-mismatch",
    ),
    (
      "trial.D.stale-stage",
      "held.macro-only-semantic-owner.stale-current-stage",
    ),
    (
      "trial.E.runtime-owner-missing",
      "held.macro-only-semantic-owner.runtime-owner-proof-missing",
    ),
    (
      "trial.F.runtime-owner-not-booted",
      "held.macro-only-semantic-owner.runtime-owner-proof-missing",
    ),
    (
      "trial.G.runtime-owner-scope-mismatch",
      "held.macro-only-semantic-owner.runtime-owner-scope-mismatch",
    ),
    (
      "trial.H.audit-chain-missing",
      "held.macro-only-semantic-owner.audit-chain-missing",
    ),
    (
      "trial.I.compare-mismatch",
      "held.macro-only-semantic-owner.compare-all-mismatch",
    ),
    (
      "trial.J.source-parity-mismatch",
      "held.macro-only-semantic-owner.source-parity-mismatch",
    ),
    (
      "trial.K.semantic-surface-missing",
      "held.macro-only-semantic-owner.semantic-surface-evidence-missing",
    ),
    (
      "trial.L.semantic-delta-loss",
      "held.macro-only-semantic-owner.semantic-delta-or-held-loss",
    ),
    (
      "trial.M.semantic-owner-scope-mismatch",
      "held.macro-only-semantic-owner.scope-mismatch",
    ),
    (
      "trial.N.global-runtime-claim",
      "held.macro-only-semantic-owner.global-runtime-overclaim",
    ),
    (
      "trial.O.host-removal-claim",
      "held.macro-only-semantic-owner.host-removal-overclaim",
    ),
    (
      "trial.P.old-host-authority",
      "held.macro-only-semantic-owner.old-host-authority",
    ),
    (
      "trial.Q.gpl-family-dependency",
      "held.macro-only-semantic-owner.gpl-family-dependency",
    ),
  ] {
    let trial = trials[id];
    assert_eq!(as_str(get(trial, "outcome")), "Held");
    assert_eq!(as_str(get(trial, "held-id")), held);
  }
}

#[test]
fn six_layer_fold_records_semantic_owner_without_collapsing_future_frontiers() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-macro-only-semantic-owner-fold");
  assert_eq!(as_str(get(fold, "mode")), "macro-only-semantic-owner-proof");
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
    &["ontology", "semantic-owner-proof"]
  )));
  assert!(as_bool(get_path(fold, &["ontology", "semantic-owner"])));
  assert_eq!(
    as_str(get_path(fold, &["semantic", "semantic-owner-scope"])),
    "bounded-generated-ontology-semantic-owner"
  );
  assert!(as_bool(get_path(fold, &["runtime", "semantic-owner"])));
  for path in [
    &["runtime", "new-engine-from-zero"][..],
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
fn migration_delta_closes_semantic_owner_and_opens_host_global_frontiers() {
  let run = eval_file(&fixture_path()).unwrap();
  let delta = get(&run, "migration-delta");
  let closes = string_set(get(delta, "closes"));
  assert!(closes.contains("need.semantic-owner.proof-after-runtime-owner"));
  assert!(closes.contains("need.semantic-owner.macro-ontology-runtime"));
  let not = string_set(get(delta, "does-not-close"));
  assert!(not.contains("need.bootstrap.new-engine-from-zero-proof"));
  assert!(not.contains("need.runtime.global-ontology-install"));
  assert!(not.contains("need.host-removal.host-code-removal-execution"));
  assert!(not.contains("need.host-removal.delete-ready-targets"));
  let next = string_set(get(delta, "next-required"));
  assert!(next.contains("host-code-removal-execution-proof-after-semantic-owner"));
  assert!(next.contains("global-runtime-install-proof-after-semantic-owner"));
  assert!(next.contains("domain-runtime-api-flattening-after-semantic-owner"));
}

#[test]
fn discoveries_record_d476_through_d483() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 8);
  for expected in [
    "D476.semantic-owner-proof-is-separate-from-runtime-owner",
    "D477.semantic-owner-is-bounded-generated-ontology-not-global-runtime",
    "D478.current-cut-compare-count-advances-to-963",
    "D479.source-parity-advances-to-18182",
    "D480.generated-ontology-surface-evidence-is-load-bearing",
    "D481.semantic-owner-preserves-negative-held",
    "D482.host-removal-waits-after-semantic-owner",
    "D483.semantic-owner-proof-blocks-old-host-and-gpl-authority",
  ] {
    let d = discoveries
      .get(expected)
      .unwrap_or_else(|| panic!("missing discovery {expected}"));
    assert_eq!(as_str(get(d, "decision-pressure")), "keep");
    assert!(as_bool(get(d, "scenario-only")));
  }
}

#[test]
fn top_level_state_is_semantic_owner_but_still_non_host_non_global_runtime() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "replacement-readiness")),
    "macro-only-semantic-owner-proof-present-not-host-removal"
  );
  assert!(as_bool(get(&run, "boot-executed")));
  assert!(as_bool(get(&run, "runtime-owner-proof")));
  assert!(as_bool(get(&run, "macro-only-runtime-owner-booted")));
  assert!(as_bool(get(&run, "semantic-owner-proof")));
  assert!(as_bool(get(&run, "semantic-owner")));
  assert_eq!(
    as_str(get(&run, "semantic-owner-scope")),
    "bounded-generated-ontology-semantic-owner"
  );
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
