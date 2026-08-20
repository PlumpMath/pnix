//! Self receipt frontier emission.
//!
//! This pins the receipt-needed detector: current open frontiers are mapped to
//! candidate receipt names, while writing, approval, deletion, runtime install,
//! API flattening, and meaning DB remain false.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../..")
}

fn fixture_path() -> PathBuf {
  repo_root().join(
    "fixtures/tesseract-macro-legacy-probe/macro_only_self_receipt_frontier_emission_receipt.px",
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
  let run = eval_file(&fixture_path()).expect("self receipt frontier emission receipt");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-macro-only-self-receipt-frontier-emission"
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
    "stdlib/lib/gate/macro-only-self-receipt-frontier-emission.px",
    "fixtures/pnix-query-runtime/macro-only-self-receipt-frontier-emission-owner.px",
    "fixtures/tesseract-macro-legacy-probe/macro_only_self_receipt_frontier_emission_receipt.px",
  ] {
    assert!(repo_root().join(path).is_file(), "missing `{path}`");
  }
}

#[test]
fn constitution_gate_blocks_receipt_writer_collapse() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitutionGate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "macro-only-self-receipt-frontier-emission"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "receipt-needed-detector-equals-receipt-writer",
    "candidate-name-equals-created-file",
    "candidate-name-equals-approval",
    "frontier-emission-equals-delete-ready",
    "frontier-emission-equals-implementation-command",
    "frontier-emission-equals-global-runtime-install",
    "frontier-emission-equals-runtime-api-flattening",
    "frontier-emission-equals-meaning-db",
    "frontier-emission-equals-p-puck-semantic-owner",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
}

#[test]
fn contract_closes_detector_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let contract = get(&run, "self-receipt-frontier-emission-contract");
  assert_eq!(
    as_str(get(contract, "id")),
    "contract.macro-only-self-receipt-frontier-emission.v1"
  );
  assert_eq!(
    as_str(get(contract, "current-status")),
    "self-receipt-frontier-emission-present"
  );
  assert_eq!(as_i64(get(contract, "open-frontier-count")), 5);
  assert_eq!(as_i64(get(contract, "emitted-candidate-count")), 5);
  assert!(as_bool(get(contract, "closes-receipt-needed-detector")));
  for key in [
    "closes-receipt-auto-writer",
    "closes-receipt-auto-approval",
    "closes-delete-ready-targets",
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
fn proof_emits_five_candidate_names_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let proof = get(&run, "self-receipt-frontier-emission-proof");
  assert_eq!(
    as_str(get(proof, "status")),
    "self-receipt-frontier-emission-present"
  );
  assert!(as_bool(get(proof, "self-receipt-frontier-emission")));
  assert!(as_bool(get(proof, "receipt-needed-detector")));
  assert_eq!(as_i64(get(proof, "emitted-candidate-count")), 5);
  let candidates = attrs_by_id(get(proof, "emitted-receipt-candidates"));
  for expected in [
    "candidate.receipt.macro-only-host-removal-delete-ready-target-proof",
    "candidate.receipt.macro-only-host-removal-implementation-command-proof",
    "candidate.receipt.global-runtime-install-proof-after-semantic-owner",
    "candidate.receipt.domain-runtime-api-flattening-map",
    "candidate.receipt.lift-query-emit-runtime-owner-or-host-removal-proof",
  ] {
    assert!(
      candidates.contains_key(expected),
      "missing candidate `{expected}`"
    );
    assert_eq!(
      as_str(get(candidates[expected], "authority")),
      "candidate-name-only"
    );
  }
  for key in [
    "receipt-auto-written",
    "receipt-auto-approved",
    "receipt-file-created",
    "delete-ready",
    "host-code-removal-started",
    "implementation-command",
    "global-ontology-runtime",
    "runtime-api-flattening",
    "meaning-db",
  ] {
    assert!(!as_bool(get(proof, key)), "`{key}` must stay false");
  }
}

#[test]
fn trials_cover_valid_source_and_held_boundaries() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "self-receipt-frontier-emission-trials"));
  assert_eq!(trials.len(), 16);
  assert_eq!(
    as_str(get(trials["trial.A.valid-frontier-emission"], "outcome")),
    "self-receipt-frontier-emission-present"
  );
  assert_eq!(
    as_str(get(trials["trial.B.frontier-source"], "outcome")),
    "tesseract-macro-ontology-macro-only-host-removal-fresh-delete-p-puck-current-cut"
  );
  for (id, held) in [
    (
      "trial.C.wrong-proof-id",
      "held.macro-only-self-receipt-frontier-emission.proof-id-mismatch",
    ),
    (
      "trial.D.stale-stage",
      "held.macro-only-self-receipt-frontier-emission.stale-current-stage",
    ),
    (
      "trial.E.missing-frontier-source",
      "held.macro-only-self-receipt-frontier-emission.missing-frontier-source",
    ),
    (
      "trial.F.missing-open-frontier",
      "held.macro-only-self-receipt-frontier-emission.frontier-or-candidate-mismatch",
    ),
    (
      "trial.G.candidate-count-mismatch",
      "held.macro-only-self-receipt-frontier-emission.candidate-count-mismatch",
    ),
    (
      "trial.H.unknown-candidate-frontier",
      "held.macro-only-self-receipt-frontier-emission.frontier-or-candidate-mismatch",
    ),
    (
      "trial.I.authority-overclaim",
      "held.macro-only-self-receipt-frontier-emission.authority-overclaim",
    ),
    (
      "trial.J.auto-writer-overclaim",
      "held.macro-only-self-receipt-frontier-emission.auto-writer-overclaim",
    ),
    (
      "trial.K.auto-approval-overclaim",
      "held.macro-only-self-receipt-frontier-emission.auto-approval-overclaim",
    ),
    (
      "trial.L.delete-overclaim",
      "held.macro-only-self-receipt-frontier-emission.delete-or-command-overclaim",
    ),
    (
      "trial.M.runtime-overclaim",
      "held.macro-only-self-receipt-frontier-emission.runtime-overclaim",
    ),
    (
      "trial.N.p-puck-semantic-owner",
      "held.macro-only-self-receipt-frontier-emission.p-puck-semantic-owner",
    ),
    (
      "trial.O.old-host-authority",
      "held.macro-only-self-receipt-frontier-emission.old-host-authority",
    ),
    (
      "trial.P.gpl-family-dependency",
      "held.macro-only-self-receipt-frontier-emission.gpl-family-dependency",
    ),
  ] {
    assert_eq!(as_str(get(trials[id], "outcome")), "Held", "{id}");
    assert_eq!(as_str(get(trials[id], "held-id")), held, "{id}");
  }
}

#[test]
fn six_layer_fold_keeps_candidate_names_separate_from_runtime() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-self-receipt-frontier-emission-fold");
  assert_eq!(
    as_str(get(fold, "mode")),
    "macro-only-self-receipt-frontier-emission"
  );
  assert_eq!(
    as_str(get_path(fold, &["gate", "constitution-verdict"])),
    "candidate-only"
  );
  assert!(as_bool(get_path(
    fold,
    &["semantic", "receipt-needed-detector"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "candidate-name-only"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["semantic", "receipt-auto-written"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["semantic", "receipt-auto-approved"]
  )));
  for key in [
    "receipt-file-created",
    "delete-ready",
    "host-code-removal-started",
    "implementation-command",
    "runtime-install",
    "global-ontology-runtime",
    "runtime-api-flattening",
    "meaning-db",
  ] {
    assert!(
      !as_bool(get_path(fold, &["runtime", key])),
      "`{key}` must stay false"
    );
  }
  assert_eq!(
    as_i64(get_path(fold, &["audit", "emitted-candidate-count"])),
    5
  );
}

#[test]
fn migration_delta_closes_only_receipt_needed_detector() {
  let run = eval_file(&fixture_path()).unwrap();
  let delta = get(&run, "migrationDelta");
  assert_eq!(
    as_str(get(delta, "id")),
    "migration-delta.macro-only-self-receipt-frontier-emission"
  );
  assert!(string_set(get(delta, "closes")).contains("need.self.receipt-needed-detector"));
  let not_closed = string_set(get(delta, "does-not-close"));
  for expected in [
    "need.self.receipt-auto-writer",
    "need.self.receipt-auto-approval",
    "need.host-removal.delete-ready-targets",
    "need.host-removal.actual-host-removal-implementation-command",
    "need.runtime.global-ontology-install",
    "need.domain-runtime-api-flattening-after-semantic-owner",
    "need.stdlib.meaning-db",
  ] {
    assert!(
      not_closed.contains(expected),
      "missing non-close `{expected}`"
    );
  }
  let next = string_set(get(delta, "next-required"));
  assert!(next.contains("receipt-skeleton-generator-after-frontier-emission"));
  assert!(next.contains("delete-ready-targets-after-fresh-delete-puck"));
}

#[test]
fn discoveries_record_d524_through_d531() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 8);
  for expected in [
    "D524.current-frontier-list-is-receipt-needed-input",
    "D525.self-receipt-emission-names-candidates-without-writing-files",
    "D526.one-receipt-candidate-per-open-frontier",
    "D527.auto-writer-and-auto-approval-remain-separate-frontiers",
    "D528.host-removal-remains-false-after-receipt-frontier-emission",
    "D529.global-runtime-flattening-and-meaning-db-remain-false",
    "D530.p-puck-remains-non-semantic-owner-in-self-receipt-loop",
    "D531.next-frontier-is-skeleton-generator-not-authority",
  ] {
    let d = discoveries
      .get(expected)
      .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
    assert!(as_bool(get(d, "scenario-only")));
    assert_eq!(as_str(get(d, "decision-pressure")), "keep");
  }
}

#[test]
fn top_level_state_records_detector_only() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "replacement-readiness")),
    "self-receipt-frontier-emission-present-candidate-only"
  );
  assert!(as_bool(get(&run, "self-receipt-frontier-emission")));
  assert!(as_bool(get(&run, "receipt-needed-detector")));
  assert_eq!(as_i64(get(&run, "emitted-candidate-count")), 5);
  assert_eq!(as_i64(get(&run, "covered-frontier-count")), 5);
  for key in [
    "receipt-auto-written",
    "receipt-auto-approved",
    "receipt-file-created",
    "delete-ready",
    "remove-now",
    "host-code-removal-started",
    "implementation-command",
    "global-ontology-runtime",
    "runtime-api-flattening",
    "meaning-db",
    "new-engine-from-zero",
    "p-puck-is-semantic-owner",
    "old-host-authority",
    "gpl-family-dependencies",
  ] {
    assert!(!as_bool(get(&run, key)), "`{key}` must stay false");
  }
}

#[test]
fn source_open_frontiers_are_preserved_as_next_work() {
  let run = eval_file(&fixture_path()).unwrap();
  let open = string_set(get(&run, "open-frontiers"));
  let next = string_set(get_path(&run, &["migrationDelta", "next-required"]));
  for expected in [
    "delete-ready-targets-after-fresh-delete-puck",
    "actual-host-removal-implementation-command",
    "global-runtime-install-proof-after-semantic-owner",
    "domain-runtime-api-flattening-after-semantic-owner",
    "lift-query-emit-runtime-owner-or-host-removal-proof",
  ] {
    assert!(
      open.contains(expected),
      "missing open frontier `{expected}`"
    );
    assert!(
      next.contains(expected),
      "missing next frontier `{expected}`"
    );
  }
}
