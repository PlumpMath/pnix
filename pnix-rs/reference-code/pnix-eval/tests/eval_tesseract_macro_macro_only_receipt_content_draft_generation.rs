use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/tesseract-macro-legacy-probe/macro_only_receipt_content_draft_generator_receipt.px",
  )
}

fn eval_fixture() -> Value {
  let path = fixture_path();
  let json = std::thread::Builder::new()
    .name("macro-only-receipt-content-draft-generation-eval".to_string())
    .stack_size(32 * 1024 * 1024)
    .spawn(move || {
      eval_to_json(path.to_str().expect("utf-8 path"), true)
        .expect("receipt content draft generation receipt")
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
    "tesseract-macro-ontology-macro-only-receipt-content-draft-generation"
  );
  assert_eq!(
    as_str(get(&run, "constitution-owner")),
    "stdlib/lib/gate/tesseract-constitution.px"
  );
  assert_eq!(
    as_str(get(&run, "source-receipt")),
    "tesseract-macro-ontology-macro-only-receipt-materialization-review"
  );
}

#[test]
fn constitution_gate_blocks_draft_to_writer_collapse() {
  let run = eval_fixture();
  let gate = get(&run, "constitutionGate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "macro-only-receipt-content-draft-generation"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "content-draft-equals-receipt-file-created",
    "content-draft-equals-receipt-content-written",
    "content-draft-equals-auto-write",
    "content-draft-equals-auto-approval",
    "content-draft-equals-delete-ready",
    "content-draft-equals-implementation-command",
    "content-draft-equals-global-runtime-install",
    "content-draft-equals-runtime-api-flattening",
    "content-draft-equals-meaning-db",
    "content-draft-equals-p-puck-semantic-owner",
    "old-host-code-authorizes-content-draft",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
}

#[test]
fn proof_generates_five_drafts_data_only() {
  let run = eval_fixture();
  let proof = get(&run, "receipt-content-draft-generation-proof");
  assert_eq!(
    as_str(get(proof, "status")),
    "receipt-content-draft-generation-present"
  );
  assert!(as_bool(get(proof, "receipt-content-draft-generation")));
  assert!(as_bool(get(proof, "content-draft-generated")));
  assert!(as_bool(get(proof, "draft-data-only")));
  assert_eq!(as_i64(get(proof, "drafted-review-count")), 5);
  assert_eq!(as_i64(get(proof, "covered-review-count")), 5);
  assert_eq!(as_list(get(proof, "content-drafts")).len(), 5);
  assert!(string_set(get(proof, "closes"))
    .contains("need.self.receipt-content-draft-generation-after-materialization-review"));
  assert!(string_set(get(proof, "next-open-frontiers"))
    .contains("receipt-file-writer-after-content-draft-generation"));
}

#[test]
fn content_drafts_carry_hard_stops_and_next_action() {
  let run = eval_fixture();
  let drafts = attrs_by_id(get(&run, "content-drafts"));
  assert_eq!(drafts.len(), 5);
  let draft = drafts
    ["draft.review.skeleton.candidate.receipt.macro-only-host-removal-delete-ready-target-proof"];
  assert_eq!(as_str(get(draft, "draft-status")), "content-draft-ready");
  assert_eq!(as_str(get(draft, "authority")), "draft-only");
  assert_eq!(
    as_str(get(draft, "next-action")),
    "receipt-file-writer-after-content-draft-generation"
  );
  let sections = string_set(get(draft, "sections"));
  for expected in [
    "probe-marker",
    "imports",
    "constitution-gate",
    "contract",
    "source-review",
    "draft-body",
    "trials",
    "six-layer-fold",
    "migration-delta",
    "discoveries",
    "negative-held-evidence",
    "hard-stops",
    "tests",
    "compare-mode",
    "bootstrap-update",
  ] {
    assert!(sections.contains(expected), "missing section `{expected}`");
  }
  let hard_stops = string_set(get(draft, "hard-stops"));
  for expected in [
    "no-receipt-file-created",
    "no-receipt-content-written",
    "no-auto-write",
    "no-auto-approval",
    "no-host-code-removal",
    "no-implementation-command",
    "no-runtime-install",
    "no-global-runtime",
    "no-runtime-api-flattening",
    "no-meaning-db",
    "no-p-puck-semantic-owner",
    "no-old-host-authority",
    "no-gpl-family-dependency",
  ] {
    assert!(
      hard_stops.contains(expected),
      "missing hard stop `{expected}`"
    );
  }
}

#[test]
fn contract_closes_content_draft_generation_only() {
  let run = eval_fixture();
  let contract = get(&run, "receipt-content-draft-generation-contract");
  assert_eq!(
    as_str(get(contract, "proof-id")),
    "proof.macro-only.receipt-content-draft-generation.v1"
  );
  assert!(as_bool(get(contract, "closes-content-draft-generation")));
  for key in [
    "closes-receipt-file-creation",
    "closes-receipt-content-writing",
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
fn migration_delta_closes_only_content_draft_generation() {
  let run = eval_fixture();
  let delta = get(&run, "migrationDelta");
  let closes = string_set(get(delta, "closes"));
  assert!(
    closes.contains("need.self.receipt-content-draft-generation-after-materialization-review")
  );
  assert_eq!(closes.len(), 1);
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
      "missing non-closure `{expected}`"
    );
  }
  assert!(string_set(get(delta, "next-required"))
    .contains("receipt-file-writer-after-content-draft-generation"));
}

#[test]
fn trials_cover_valid_source_draft_shape_and_held_boundaries() {
  let run = eval_fixture();
  let trials = attrs_by_id(get(&run, "receipt-content-draft-generation-trials"));
  assert_eq!(trials.len(), 19);
  assert_eq!(
    as_str(get(
      trials["trial.A.valid-content-draft-generation"],
      "outcome"
    )),
    "receipt-content-draft-generation-present"
  );
  for (id, held) in [
    (
      "trial.C.wrong-proof-id",
      "held.macro-only-receipt-content-draft-generation.proof-id-mismatch",
    ),
    (
      "trial.D.stale-stage",
      "held.macro-only-receipt-content-draft-generation.stale-current-stage",
    ),
    (
      "trial.E.source-mismatch",
      "held.macro-only-receipt-content-draft-generation.source-mismatch",
    ),
    (
      "trial.F.materialization-review-missing",
      "held.macro-only-receipt-content-draft-generation.materialization-review-missing",
    ),
    (
      "trial.G.review-count-mismatch",
      "held.macro-only-receipt-content-draft-generation.review-count-mismatch",
    ),
    (
      "trial.H.draft-count-mismatch",
      "held.macro-only-receipt-content-draft-generation.draft-count-mismatch",
    ),
    (
      "trial.I.source-review-overclaim",
      "held.macro-only-receipt-content-draft-generation.source-review-overclaim",
    ),
    (
      "trial.J.draft-authority-overclaim",
      "held.macro-only-receipt-content-draft-generation.draft-authority-overclaim",
    ),
    (
      "trial.K.draft-shape-mismatch",
      "held.macro-only-receipt-content-draft-generation.draft-shape-mismatch",
    ),
    (
      "trial.M.file-or-content-overclaim",
      "held.macro-only-receipt-content-draft-generation.file-or-content-overclaim",
    ),
    (
      "trial.N.auto-approval-overclaim",
      "held.macro-only-receipt-content-draft-generation.auto-approval-overclaim",
    ),
    (
      "trial.O.delete-overclaim",
      "held.macro-only-receipt-content-draft-generation.delete-or-command-overclaim",
    ),
    (
      "trial.P.runtime-overclaim",
      "held.macro-only-receipt-content-draft-generation.runtime-overclaim",
    ),
    (
      "trial.Q.p-puck-semantic-owner",
      "held.macro-only-receipt-content-draft-generation.p-puck-semantic-owner",
    ),
    (
      "trial.R.old-host-authority",
      "held.macro-only-receipt-content-draft-generation.old-host-authority",
    ),
    (
      "trial.S.gpl-family-dependency",
      "held.macro-only-receipt-content-draft-generation.gpl-family-dependency",
    ),
  ] {
    assert_eq!(as_str(get(trials[id], "held-id")), held, "{id}");
  }
}

#[test]
fn six_layer_fold_keeps_draft_separate_from_file_and_runtime() {
  let run = eval_fixture();
  let fold = get(&run, "six-layer-receipt-content-draft-generation-fold");
  assert_eq!(
    as_str(get(fold, "mode")),
    "macro-only-receipt-content-draft-generation"
  );
  assert!(as_bool(get(
    get(fold, "semantic"),
    "content-draft-generated"
  )));
  assert!(as_bool(get(get(fold, "semantic"), "draft-data-only")));
  assert!(!as_bool(get(get(fold, "semantic"), "receipt-file-created")));
  assert!(!as_bool(get(
    get(fold, "semantic"),
    "receipt-content-written"
  )));
  let runtime = get(fold, "runtime");
  assert!(as_bool(get(runtime, "content-draft-generated")));
  for key in [
    "receipt-file-created",
    "receipt-content-written",
    "receipt-auto-written",
    "receipt-auto-approved",
    "delete-ready",
    "implementation-command",
    "runtime-install",
    "global-ontology-runtime",
    "runtime-api-flattening",
    "meaning-db",
  ] {
    assert!(!as_bool(get(runtime, key)), "`{key}` must stay false");
  }
}

#[test]
fn discoveries_record_d548_through_d555() {
  let run = eval_fixture();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 8);
  for expected in [
    "D548.review-objects-can-lower-to-structured-draft-data",
    "D549.draft-data-preserves-review-targets",
    "D550.content-draft-is-not-written-content",
    "D551.draft-body-is-template-plan-not-file",
    "D552.draft-hard-stops-block-writer-approval-command-runtime-collapse",
    "D553.one-draft-per-review-keeps-frontier-split",
    "D554.draft-output-is-writer-input-not-implementation-command",
    "D555.next-frontier-is-file-writer-not-approval-or-runtime",
  ] {
    assert!(
      discoveries.contains_key(expected),
      "missing discovery `{expected}`"
    );
  }
}

#[test]
fn top_level_state_records_draft_data_only_no_writer_runtime_or_db() {
  let run = eval_fixture();
  assert_eq!(
    as_str(get(&run, "replacement-readiness")),
    "receipt-content-draft-generation-present-data-only"
  );
  assert!(as_bool(get(&run, "receipt-content-draft-generation")));
  assert!(as_bool(get(&run, "content-draft-generated")));
  assert!(as_bool(get(&run, "draft-data-only")));
  assert_eq!(as_i64(get(&run, "drafted-review-count")), 5);
  assert_eq!(as_i64(get(&run, "covered-review-count")), 5);
  for key in [
    "receipt-file-created",
    "receipt-content-written",
    "receipt-auto-written",
    "receipt-auto-approved",
    "delete-ready",
    "remove-now",
    "host-code-removal-started",
    "implementation-command",
    "runtime-install",
    "global-ontology-runtime",
    "runtime-api-flattening",
    "meaning-db",
    "p-puck-is-semantic-owner",
    "old-host-authority",
    "gpl-family-dependencies",
  ] {
    assert!(!as_bool(get(&run, key)), "`{key}` must stay false");
  }
}
