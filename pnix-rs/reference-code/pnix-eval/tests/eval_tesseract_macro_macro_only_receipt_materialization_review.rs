//! Macro-only receipt materialization review.
//!
//! This pins the review-only step after data-only receipt skeleton generation:
//! skeletons become materialization review objects, while file creation,
//! content writing, draft generation, approval, deletion, runtime install,
//! API flattening, and meaning DB remain false.

use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../..")
}

fn fixture_path() -> PathBuf {
  repo_root().join(
    "fixtures/tesseract-macro-legacy-probe/macro_only_receipt_materialization_review_receipt.px",
  )
}

fn eval_fixture() -> Value {
  let path = fixture_path();
  let json = std::thread::Builder::new()
    .name("macro-only-receipt-materialization-review-eval".to_string())
    .stack_size(32 * 1024 * 1024)
    .spawn(move || {
      eval_to_json(path.to_str().expect("utf-8 path"), true)
        .expect("macro-only receipt materialization review receipt")
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
    "tesseract-macro-ontology-macro-only-receipt-materialization-review"
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
    "stdlib/lib/gate/macro-only-receipt-materialization-review.px",
    "fixtures/pnix-query-runtime/macro-only-receipt-materialization-review-owner.px",
    "fixtures/tesseract-macro-legacy-probe/macro_only_receipt_materialization_review_receipt.px",
  ] {
    assert!(repo_root().join(path).is_file(), "missing `{path}`");
  }
}

#[test]
fn constitution_gate_blocks_review_to_file_collapse() {
  let run = eval_fixture();
  let gate = get(&run, "constitutionGate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "macro-only-receipt-materialization-review"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "materialization-review-equals-receipt-file-created",
    "materialization-review-equals-content-written",
    "materialization-review-equals-content-draft-generated",
    "materialization-review-equals-auto-approval",
    "materialization-review-equals-delete-ready",
    "materialization-review-equals-implementation-command",
    "materialization-review-equals-global-runtime-install",
    "materialization-review-equals-runtime-api-flattening",
    "materialization-review-equals-meaning-db",
    "materialization-review-equals-p-puck-semantic-owner",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
}

#[test]
fn contract_closes_materialization_review_only() {
  let run = eval_fixture();
  let contract = get(&run, "receipt-materialization-review-contract");
  assert_eq!(
    as_str(get(contract, "id")),
    "contract.macro-only-receipt-materialization-review.v1"
  );
  assert_eq!(
    as_str(get(contract, "current-status")),
    "receipt-materialization-review-present"
  );
  assert_eq!(as_i64(get(contract, "source-skeleton-count")), 5);
  assert_eq!(as_i64(get(contract, "reviewed-skeleton-count")), 5);
  assert!(as_bool(get(contract, "closes-materialization-review")));
  for key in [
    "closes-receipt-content-draft-generation",
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
fn proof_reviews_five_skeletons_review_only() {
  let run = eval_fixture();
  let proof = get(&run, "receipt-materialization-review-proof");
  assert_eq!(
    as_str(get(proof, "status")),
    "receipt-materialization-review-present"
  );
  assert!(as_bool(get(proof, "receipt-materialization-review")));
  assert!(as_bool(get(proof, "materialization-reviewed")));
  assert!(as_bool(get(proof, "materialization-review-only")));
  assert_eq!(as_i64(get(proof, "reviewed-skeleton-count")), 5);
  let reviews = attrs_by_id(get(proof, "materialization-reviews"));
  assert_eq!(reviews.len(), 5);
  for expected in [
    "review.skeleton.candidate.receipt.macro-only-host-removal-delete-ready-target-proof",
    "review.skeleton.candidate.receipt.macro-only-host-removal-implementation-command-proof",
    "review.skeleton.candidate.receipt.global-runtime-install-proof-after-semantic-owner",
    "review.skeleton.candidate.receipt.domain-runtime-api-flattening-map",
    "review.skeleton.candidate.receipt.lift-query-emit-runtime-owner-or-host-removal-proof",
  ] {
    assert!(
      reviews.contains_key(expected),
      "missing review `{expected}`"
    );
    assert_eq!(
      as_str(get(reviews[expected], "review-status")),
      "materialization-review-ready"
    );
    assert_eq!(
      as_str(get(reviews[expected], "materialization")),
      "review-only"
    );
    assert_eq!(as_str(get(reviews[expected], "authority")), "review-only");
    assert!(!as_bool(get(reviews[expected], "receipt-file-created")));
    assert!(!as_bool(get(reviews[expected], "receipt-content-written")));
    assert!(!as_bool(get(reviews[expected], "content-draft-generated")));
    assert!(!as_bool(get(reviews[expected], "receipt-auto-approved")));
  }
}

#[test]
fn trials_cover_valid_source_review_shape_and_held_boundaries() {
  let run = eval_fixture();
  let trials = attrs_by_id(get(&run, "receipt-materialization-review-trials"));
  assert_eq!(trials.len(), 19);
  assert_eq!(
    as_str(get(
      trials["trial.A.valid-materialization-review"],
      "outcome"
    )),
    "receipt-materialization-review-present"
  );
  assert_eq!(
    as_str(get(trials["trial.B.source-skeleton-generator"], "outcome")),
    "tesseract-macro-ontology-macro-only-receipt-skeleton-generator"
  );
  for (id, held) in [
    (
      "trial.C.wrong-proof-id",
      "held.macro-only-receipt-materialization-review.proof-id-mismatch",
    ),
    (
      "trial.D.stale-stage",
      "held.macro-only-receipt-materialization-review.stale-current-stage",
    ),
    (
      "trial.E.source-mismatch",
      "held.macro-only-receipt-materialization-review.source-mismatch",
    ),
    (
      "trial.F.skeleton-generator-missing",
      "held.macro-only-receipt-materialization-review.skeleton-generator-missing",
    ),
    (
      "trial.G.skeleton-count-mismatch",
      "held.macro-only-receipt-materialization-review.skeleton-count-mismatch",
    ),
    (
      "trial.H.review-count-mismatch",
      "held.macro-only-receipt-materialization-review.review-count-mismatch",
    ),
    (
      "trial.I.skeleton-materialization-overclaim",
      "held.macro-only-receipt-materialization-review.skeleton-materialization-overclaim",
    ),
    (
      "trial.J.materialization-authority-overclaim",
      "held.macro-only-receipt-materialization-review.materialization-authority-overclaim",
    ),
    (
      "trial.K.review-shape-mismatch",
      "held.macro-only-receipt-materialization-review.review-shape-mismatch",
    ),
    (
      "trial.L.missing-section",
      "held.macro-only-receipt-materialization-review.review-shape-mismatch",
    ),
    (
      "trial.M.file-or-content-overclaim",
      "held.macro-only-receipt-materialization-review.file-or-content-overclaim",
    ),
    (
      "trial.N.auto-approval-overclaim",
      "held.macro-only-receipt-materialization-review.auto-approval-overclaim",
    ),
    (
      "trial.O.delete-overclaim",
      "held.macro-only-receipt-materialization-review.delete-or-command-overclaim",
    ),
    (
      "trial.P.runtime-overclaim",
      "held.macro-only-receipt-materialization-review.runtime-overclaim",
    ),
    (
      "trial.Q.p-puck-semantic-owner",
      "held.macro-only-receipt-materialization-review.p-puck-semantic-owner",
    ),
    (
      "trial.R.old-host-authority",
      "held.macro-only-receipt-materialization-review.old-host-authority",
    ),
    (
      "trial.S.gpl-family-dependency",
      "held.macro-only-receipt-materialization-review.gpl-family-dependency",
    ),
  ] {
    assert_eq!(as_str(get(trials[id], "outcome")), "Held", "{id}");
    assert_eq!(as_str(get(trials[id], "held-id")), held, "{id}");
  }
}

#[test]
fn six_layer_fold_keeps_review_separate_from_content_and_runtime() {
  let run = eval_fixture();
  let fold = get(&run, "six-layer-receipt-materialization-review-fold");
  assert_eq!(
    as_str(get(fold, "mode")),
    "macro-only-receipt-materialization-review"
  );
  assert_eq!(
    as_str(get_path(fold, &["gate", "constitution-verdict"])),
    "candidate-only"
  );
  assert!(as_bool(get_path(
    fold,
    &["semantic", "receipt-materialization-review"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "materialization-reviewed"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "materialization-review-only"]
  )));
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
    &["semantic", "content-draft-generated"]
  )));
  for key in [
    "receipt-file-created",
    "receipt-content-written",
    "content-draft-generated",
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
fn migration_delta_closes_only_materialization_review() {
  let run = eval_fixture();
  let delta = get(&run, "migrationDelta");
  let closes = string_set(get(delta, "closes"));
  assert_eq!(closes.len(), 1);
  assert!(closes.contains("need.self.receipt-skeleton-materialization-review-after-data-skeleton"));
  let not_closed = string_set(get(delta, "does-not-close"));
  for expected in [
    "need.self.receipt-content-draft-generation",
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
  assert!(next.contains("receipt-content-draft-generation-after-materialization-review"));
  assert!(next.contains("delete-ready-targets-after-fresh-delete-puck"));
}

#[test]
fn discoveries_record_d540_through_d547() {
  let run = eval_fixture();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 8);
  for expected in [
    "D540.data-only-skeletons-can-be-reviewed-without-materialization",
    "D541.materialization-review-preserves-target-owner-and-receipt",
    "D542.review-only-is-a-distinct-authority-level",
    "D543.content-draft-generation-is-separate-from-review",
    "D544.review-hard-stops-prevent-file-approval-command-runtime-collapse",
    "D545.one-review-per-skeleton-keeps-frontier-split",
    "D546.review-output-is-draft-input-not-implementation-command",
    "D547.next-frontier-is-content-draft-generation-not-runtime",
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
fn top_level_state_records_review_only_no_runtime_or_db() {
  let run = eval_fixture();
  assert_eq!(
    as_str(get(&run, "replacement-readiness")),
    "receipt-materialization-review-present-review-only"
  );
  assert!(as_bool(get(&run, "receipt-materialization-review")));
  assert!(as_bool(get(&run, "materialization-reviewed")));
  assert!(as_bool(get(&run, "materialization-review-only")));
  assert_eq!(as_i64(get(&run, "reviewed-skeleton-count")), 5);
  assert_eq!(as_i64(get(&run, "covered-skeleton-count")), 5);
  for key in [
    "receipt-file-created",
    "receipt-content-written",
    "content-draft-generated",
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
fn materialization_reviews_carry_hard_stops_and_next_action() {
  let run = eval_fixture();
  let reviews = as_list(get(&run, "materialization-reviews"));
  assert_eq!(reviews.len(), 5);
  for review in reviews {
    let sections = string_set(get(review, "sections"));
    let hard_stops = string_set(get(review, "hard-stops"));
    assert!(sections.contains("source-skeleton"));
    assert!(sections.contains("materialization-target"));
    assert!(sections.contains("negative-held-evidence"));
    assert!(hard_stops.contains("no-receipt-file-created"));
    assert!(hard_stops.contains("no-receipt-content-written"));
    assert!(hard_stops.contains("no-content-draft-generated"));
    assert!(hard_stops.contains("no-auto-approval"));
    assert!(hard_stops.contains("no-global-runtime"));
    assert!(hard_stops.contains("no-meaning-db"));
    assert_eq!(
      as_str(get(review, "next-action")),
      "receipt-content-draft-generation-after-materialization-review"
    );
  }
}
