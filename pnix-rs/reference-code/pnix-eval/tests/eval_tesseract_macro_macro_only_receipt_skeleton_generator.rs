//! Macro-only receipt skeleton generator.
//!
//! This pins the data-only skeleton step after self receipt frontier emission:
//! candidate receipt names become structured skeleton data, while file writing,
//! approval, deletion, runtime install, API flattening, and meaning DB remain
//! false.

use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../..")
}

fn fixture_path() -> PathBuf {
  repo_root()
    .join("fixtures/tesseract-macro-legacy-probe/macro_only_receipt_skeleton_generator_receipt.px")
}

fn eval_fixture() -> Value {
  let path = fixture_path();
  let json = std::thread::Builder::new()
    .name("macro-only-receipt-skeleton-generator-eval".to_string())
    .stack_size(32 * 1024 * 1024)
    .spawn(move || {
      eval_to_json(path.to_str().expect("utf-8 path"), true)
        .expect("macro-only receipt skeleton generator receipt")
    })
    .expect("spawn eval thread")
    .join()
    .expect("eval thread panicked");
  serde_json::from_str(&json).expect("fixture JSON")
}

fn as_attrs(v: &Value) -> &Map<String, Value> {
  v.as_object()
    .unwrap_or_else(|| panic!("expected object, got {v:?}"))
}

fn as_list(v: &Value) -> &Vec<Value> {
  v.as_array()
    .unwrap_or_else(|| panic!("expected array, got {v:?}"))
}

fn as_str(v: &Value) -> &str {
  v.as_str()
    .unwrap_or_else(|| panic!("expected string, got {v:?}"))
}

fn as_bool(v: &Value) -> bool {
  v.as_bool()
    .unwrap_or_else(|| panic!("expected bool, got {v:?}"))
}

fn as_i64(v: &Value) -> i64 {
  v.as_i64()
    .unwrap_or_else(|| panic!("expected integer, got {v:?}"))
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
  let run = eval_fixture();
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-macro-only-receipt-skeleton-generator"
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
    "stdlib/lib/gate/macro-only-receipt-skeleton-generator.px",
    "fixtures/pnix-query-runtime/macro-only-receipt-skeleton-generator-owner.px",
    "fixtures/tesseract-macro-legacy-probe/macro_only_receipt_skeleton_generator_receipt.px",
  ] {
    assert!(repo_root().join(path).is_file(), "missing `{path}`");
  }
}

#[test]
fn constitution_gate_blocks_skeleton_to_file_collapse() {
  let run = eval_fixture();
  let gate = get(&run, "constitutionGate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "macro-only-receipt-skeleton-generator"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "receipt-skeleton-data-equals-file-created",
    "receipt-skeleton-data-equals-content-written",
    "receipt-skeleton-data-equals-auto-approval",
    "receipt-skeleton-data-equals-delete-ready",
    "receipt-skeleton-data-equals-implementation-command",
    "receipt-skeleton-data-equals-global-runtime-install",
    "receipt-skeleton-data-equals-runtime-api-flattening",
    "receipt-skeleton-data-equals-meaning-db",
    "receipt-skeleton-data-equals-p-puck-semantic-owner",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
}

#[test]
fn contract_closes_skeleton_generator_only() {
  let run = eval_fixture();
  let contract = get(&run, "receipt-skeleton-generator-contract");
  assert_eq!(
    as_str(get(contract, "id")),
    "contract.macro-only-receipt-skeleton-generator.v1"
  );
  assert_eq!(
    as_str(get(contract, "current-status")),
    "receipt-skeleton-generator-present"
  );
  assert_eq!(as_i64(get(contract, "source-candidate-count")), 5);
  assert_eq!(as_i64(get(contract, "generated-skeleton-count")), 5);
  assert!(as_bool(get(contract, "closes-skeleton-generator")));
  for key in [
    "closes-receipt-file-creation",
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
fn proof_generates_five_skeletons_data_only() {
  let run = eval_fixture();
  let proof = get(&run, "receipt-skeleton-generator-proof");
  assert_eq!(
    as_str(get(proof, "status")),
    "receipt-skeleton-generator-present"
  );
  assert!(as_bool(get(proof, "receipt-skeleton-generator")));
  assert!(as_bool(get(proof, "skeleton-data-only")));
  assert_eq!(as_i64(get(proof, "generated-skeleton-count")), 5);
  let skeletons = attrs_by_id(get(proof, "generated-receipt-skeletons"));
  assert_eq!(skeletons.len(), 5);
  for expected in [
    "skeleton.candidate.receipt.macro-only-host-removal-delete-ready-target-proof",
    "skeleton.candidate.receipt.macro-only-host-removal-implementation-command-proof",
    "skeleton.candidate.receipt.global-runtime-install-proof-after-semantic-owner",
    "skeleton.candidate.receipt.domain-runtime-api-flattening-map",
    "skeleton.candidate.receipt.lift-query-emit-runtime-owner-or-host-removal-proof",
  ] {
    assert!(
      skeletons.contains_key(expected),
      "missing skeleton `{expected}`"
    );
    assert_eq!(
      as_str(get(skeletons[expected], "materialization")),
      "data-only"
    );
    assert_eq!(
      as_str(get(skeletons[expected], "authority")),
      "skeleton-data-only"
    );
    assert!(!as_bool(get(skeletons[expected], "receipt-file-created")));
    assert!(!as_bool(get(
      skeletons[expected],
      "receipt-content-written"
    )));
    assert!(!as_bool(get(skeletons[expected], "receipt-auto-approved")));
  }
}

#[test]
fn trials_cover_valid_source_shape_and_held_boundaries() {
  let run = eval_fixture();
  let trials = attrs_by_id(get(&run, "receipt-skeleton-generator-trials"));
  assert_eq!(trials.len(), 19);
  assert_eq!(
    as_str(get(trials["trial.A.valid-skeleton-generation"], "outcome")),
    "receipt-skeleton-generator-present"
  );
  assert_eq!(
    as_str(get(trials["trial.B.source-emission"], "outcome")),
    "tesseract-macro-ontology-macro-only-self-receipt-frontier-emission"
  );
  for (id, held) in [
    (
      "trial.C.wrong-proof-id",
      "held.macro-only-receipt-skeleton-generator.proof-id-mismatch",
    ),
    (
      "trial.D.stale-stage",
      "held.macro-only-receipt-skeleton-generator.stale-current-stage",
    ),
    (
      "trial.E.source-mismatch",
      "held.macro-only-receipt-skeleton-generator.source-mismatch",
    ),
    (
      "trial.F.candidate-count-mismatch",
      "held.macro-only-receipt-skeleton-generator.candidate-count-mismatch",
    ),
    (
      "trial.G.skeleton-count-mismatch",
      "held.macro-only-receipt-skeleton-generator.skeleton-count-mismatch",
    ),
    (
      "trial.H.candidate-authority-overclaim",
      "held.macro-only-receipt-skeleton-generator.candidate-authority-overclaim",
    ),
    (
      "trial.I.materialization-overclaim",
      "held.macro-only-receipt-skeleton-generator.materialization-overclaim",
    ),
    (
      "trial.J.skeleton-authority-overclaim",
      "held.macro-only-receipt-skeleton-generator.skeleton-authority-overclaim",
    ),
    (
      "trial.K.skeleton-shape-mismatch",
      "held.macro-only-receipt-skeleton-generator.skeleton-shape-mismatch",
    ),
    (
      "trial.L.missing-section",
      "held.macro-only-receipt-skeleton-generator.skeleton-shape-mismatch",
    ),
    (
      "trial.M.file-writer-overclaim",
      "held.macro-only-receipt-skeleton-generator.file-writer-overclaim",
    ),
    (
      "trial.N.auto-approval-overclaim",
      "held.macro-only-receipt-skeleton-generator.auto-approval-overclaim",
    ),
    (
      "trial.O.delete-overclaim",
      "held.macro-only-receipt-skeleton-generator.delete-or-command-overclaim",
    ),
    (
      "trial.P.runtime-overclaim",
      "held.macro-only-receipt-skeleton-generator.runtime-overclaim",
    ),
    (
      "trial.Q.p-puck-semantic-owner",
      "held.macro-only-receipt-skeleton-generator.p-puck-semantic-owner",
    ),
    (
      "trial.R.old-host-authority",
      "held.macro-only-receipt-skeleton-generator.old-host-authority",
    ),
    (
      "trial.S.gpl-family-dependency",
      "held.macro-only-receipt-skeleton-generator.gpl-family-dependency",
    ),
  ] {
    assert_eq!(as_str(get(trials[id], "outcome")), "Held", "{id}");
    assert_eq!(as_str(get(trials[id], "held-id")), held, "{id}");
  }
}

#[test]
fn six_layer_fold_keeps_skeleton_data_separate_from_runtime() {
  let run = eval_fixture();
  let fold = get(&run, "six-layer-receipt-skeleton-generator-fold");
  assert_eq!(
    as_str(get(fold, "mode")),
    "macro-only-receipt-skeleton-generator"
  );
  assert_eq!(
    as_str(get_path(fold, &["gate", "constitution-verdict"])),
    "candidate-only"
  );
  assert!(as_bool(get_path(
    fold,
    &["semantic", "receipt-skeleton-generator"]
  )));
  assert!(as_bool(get_path(fold, &["semantic", "skeleton-data-only"])));
  assert!(!as_bool(get_path(
    fold,
    &["semantic", "receipt-file-created"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["semantic", "receipt-content-written"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["semantic", "receipt-auto-approved"]
  )));
  for key in [
    "receipt-file-created",
    "receipt-content-written",
    "receipt-auto-written",
    "receipt-auto-approved",
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
}

#[test]
fn migration_delta_closes_only_skeleton_generator() {
  let run = eval_fixture();
  let delta = get(&run, "migrationDelta");
  let closes = string_set(get(delta, "closes"));
  assert_eq!(closes.len(), 1);
  assert!(closes.contains("need.self.receipt-skeleton-generator-after-frontier-emission"));
  let not_closed = string_set(get(delta, "does-not-close"));
  for expected in [
    "need.self.receipt-file-writer",
    "need.self.receipt-auto-approval",
    "need.host-removal.delete-ready-targets",
    "need.host-removal.actual-host-removal-implementation-command",
    "need.runtime.global-ontology-install",
    "need.domain-runtime-api-flattening-after-semantic-owner",
    "need.stdlib.meaning-db",
  ] {
    assert!(
      not_closed.contains(expected),
      "missing does-not-close `{expected}`"
    );
  }
  let next = string_set(get(delta, "next-required"));
  assert!(next.contains("receipt-skeleton-materialization-review-after-data-skeleton"));
  assert!(next.contains("delete-ready-targets-after-fresh-delete-puck"));
}

#[test]
fn discoveries_record_d532_through_d539() {
  let run = eval_fixture();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 8);
  for expected in [
    "D532.receipt-candidate-names-can-fold-into-data-only-skeletons",
    "D533.skeleton-generation-is-not-materialization",
    "D534.one-skeleton-per-candidate-preserves-frontier-split",
    "D535.required-sections-become-machine-checkable-receipt-shape",
    "D536.skeleton-hard-stops-keep-self-authoring-from-self-approval",
    "D537.skeleton-output-is-next-work-input-not-implementation-command",
    "D538.skeleton-generator-makes-receipt-writing-flatter-but-still-gated",
    "D539.next-frontier-is-materialization-review-not-runtime",
  ] {
    assert!(
      discoveries.contains_key(expected),
      "missing discovery `{expected}`"
    );
    assert!(as_bool(get(discoveries[expected], "scenario-only")));
    assert_eq!(
      as_str(get(discoveries[expected], "decision-pressure")),
      "keep"
    );
  }
}

#[test]
fn top_level_state_records_data_only_no_runtime_or_db() {
  let run = eval_fixture();
  assert_eq!(
    as_str(get(&run, "replacement-readiness")),
    "receipt-skeleton-generator-present-data-only"
  );
  assert!(as_bool(get(&run, "receipt-skeleton-generator")));
  assert!(as_bool(get(&run, "skeleton-data-only")));
  assert_eq!(as_i64(get(&run, "generated-skeleton-count")), 5);
  assert_eq!(as_i64(get(&run, "covered-candidate-count")), 5);
  for key in [
    "receipt-file-created",
    "receipt-content-written",
    "receipt-auto-written",
    "receipt-auto-approved",
    "delete-ready",
    "remove-now",
    "host-code-removal-started",
    "implementation-command",
    "global-ontology-runtime",
    "runtime-install",
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
fn generated_skeletons_carry_hard_stops_and_placeholders() {
  let run = eval_fixture();
  let skeletons = as_list(get(&run, "generated-receipt-skeletons"));
  assert_eq!(skeletons.len(), 5);
  for skeleton in skeletons {
    let sections = string_set(get(skeleton, "sections"));
    let hard_stops = string_set(get(skeleton, "hard-stops"));
    let placeholders = string_set(get(skeleton, "proof-placeholders"));
    assert!(sections.contains("negative-held-evidence"));
    assert!(sections.contains("proof-placeholders"));
    assert!(hard_stops.contains("no-receipt-file-created"));
    assert!(hard_stops.contains("no-auto-approval"));
    assert!(hard_stops.contains("no-global-runtime"));
    assert!(hard_stops.contains("no-meaning-db"));
    assert!(placeholders.contains("constitutionGate"));
    assert!(placeholders.contains("migrationDelta"));
    assert!(placeholders.contains("bootstrap-update"));
  }
}
