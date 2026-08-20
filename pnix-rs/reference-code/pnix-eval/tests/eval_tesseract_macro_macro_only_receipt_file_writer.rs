use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/tesseract-macro-legacy-probe/macro_only_receipt_file_writer_receipt.px")
}

fn eval_fixture() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = fixture_path();
    let json = std::thread::Builder::new()
      .name("macro-only-receipt-file-writer-eval".to_string())
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true).expect("receipt file writer receipt")
      })
      .expect("spawn eval thread")
      .join()
      .expect("eval thread panicked");
    serde_json::from_str(&json).expect("fixture JSON")
  })
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
    as_str(get(run, "probe-marker")),
    "tesseract-macro-ontology-macro-only-receipt-file-writer"
  );
  assert_eq!(
    as_str(get(run, "constitution-owner")),
    "stdlib/lib/gate/tesseract-constitution.px"
  );
  assert_eq!(
    as_str(get(run, "source-receipt")),
    "tesseract-macro-ontology-macro-only-receipt-content-draft-generation"
  );
}

#[test]
fn constitution_gate_blocks_writer_candidate_collapse() {
  let run = eval_fixture();
  let gate = get(run, "constitutionGate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "macro-only-receipt-file-writer"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "file-artifact-candidate-equals-disk-write",
    "file-artifact-candidate-equals-receipt-file-created",
    "file-artifact-candidate-equals-receipt-content-written",
    "file-artifact-candidate-equals-auto-write",
    "file-artifact-candidate-equals-auto-approval",
    "file-artifact-candidate-equals-target-frontier-closed",
    "file-artifact-candidate-equals-delete-ready",
    "file-artifact-candidate-equals-implementation-command",
    "file-artifact-candidate-equals-global-runtime-install",
    "file-artifact-candidate-equals-runtime-api-flattening",
    "file-artifact-candidate-equals-meaning-db",
    "file-artifact-candidate-equals-p-puck-semantic-owner",
    "old-host-code-authorizes-file-artifact",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
}

#[test]
fn proof_generates_five_file_artifact_candidates() {
  let run = eval_fixture();
  let proof = get(run, "receipt-file-writer-proof");
  assert_eq!(
    as_str(get(proof, "status")),
    "receipt-file-writer-candidate-present"
  );
  assert!(as_bool(get(proof, "receipt-file-writer")));
  assert!(as_bool(get(proof, "receipt-file-artifact-generated")));
  assert!(as_bool(get(proof, "writer-candidate-only")));
  assert_eq!(as_i64(get(proof, "written-artifact-count")), 5);
  assert_eq!(as_i64(get(proof, "covered-draft-count")), 5);
  assert_eq!(as_list(get(proof, "file-artifacts")).len(), 5);
  assert!(string_set(get(proof, "closes"))
    .contains("need.self.receipt-file-writer-after-content-draft-generation"));
  assert!(string_set(get(proof, "next-open-frontiers"))
    .contains("receipt-file-materialization-proof-after-writer-candidate"));
}

#[test]
fn file_artifacts_preserve_paths_without_disk_write_or_target_closure() {
  let run = eval_fixture();
  let artifacts = attrs_by_id(get(run, "file-artifacts"));
  assert_eq!(artifacts.len(), 5);
  let artifact =
    artifacts["file.draft.review.skeleton.candidate.receipt.macro-only-host-removal-delete-ready-target-proof"];
  assert_eq!(
    as_str(get(artifact, "target-receipt")),
    "fixtures/tesseract-macro-legacy-probe/macro_only_host_removal_delete_ready_target_proof_receipt.px"
  );
  assert_eq!(
    as_str(get(artifact, "file-path")),
    "fixtures/tesseract-macro-legacy-probe/macro_only_host_removal_delete_ready_target_proof_receipt.px"
  );
  assert!(as_bool(get(artifact, "receipt-file-artifact-generated")));
  assert!(as_bool(get(artifact, "writer-candidate-only")));
  for key in [
    "receipt-file-created",
    "receipt-content-written",
    "receipt-auto-written",
    "receipt-auto-approved",
    "target-frontier-closed",
    "implementation-command",
    "runtime-install",
    "meaning-db",
    "old-host-authority",
    "gpl-family-dependencies",
  ] {
    assert!(!as_bool(get(artifact, key)), "`{key}` must stay false");
  }
}

#[test]
fn contract_closes_file_writer_only() {
  let run = eval_fixture();
  let contract = get(run, "receipt-file-writer-contract");
  assert_eq!(
    as_str(get(contract, "proof-id")),
    "proof.macro-only.receipt-file-writer.v1"
  );
  assert!(as_bool(get(contract, "closes-receipt-file-writer")));
  assert_eq!(as_i64(get(contract, "written-artifact-count")), 5);
  for key in [
    "closes-disk-write",
    "closes-receipt-file-creation",
    "closes-receipt-content-writing",
    "closes-receipt-auto-writer",
    "closes-receipt-auto-approval",
    "closes-target-frontier",
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
fn migration_delta_closes_only_writer_frontier() {
  let run = eval_fixture();
  let delta = get(run, "migrationDelta");
  let closes = string_set(get(delta, "closes"));
  assert!(closes.contains("need.self.receipt-file-writer-after-content-draft-generation"));
  assert_eq!(closes.len(), 1);
  let not_closed = string_set(get(delta, "does-not-close"));
  for expected in [
    "need.self.receipt-file-materialization-proof",
    "need.self.receipt-auto-approval",
    "need.host-removal.delete-ready-targets",
    "need.host-removal.actual-host-removal-implementation-command",
    "need.runtime.global-ontology-install",
    "need.domain-runtime-api-flattening-after-semantic-owner",
    "need.lift-query-emit.runtime-owner-or-host-removal-proof",
    "need.stdlib.meaning-db",
  ] {
    assert!(
      not_closed.contains(expected),
      "missing non-closure `{expected}`"
    );
  }
  assert!(string_set(get(delta, "next-required"))
    .contains("receipt-file-materialization-proof-after-writer-candidate"));
}

#[test]
fn trials_cover_valid_source_artifact_shape_and_held_boundaries() {
  let run = eval_fixture();
  let trials = attrs_by_id(get(run, "receipt-file-writer-trials"));
  assert_eq!(trials.len(), 20);
  assert_eq!(
    as_str(get(
      trials["trial.A.valid-file-writer-candidate"],
      "outcome"
    )),
    "receipt-file-writer-candidate-present"
  );
  for (id, held) in [
    (
      "trial.C.wrong-proof-id",
      "held.macro-only-receipt-file-writer.proof-id-mismatch",
    ),
    (
      "trial.D.stale-stage",
      "held.macro-only-receipt-file-writer.stale-current-stage",
    ),
    (
      "trial.E.source-mismatch",
      "held.macro-only-receipt-file-writer.source-mismatch",
    ),
    (
      "trial.F.content-draft-missing",
      "held.macro-only-receipt-file-writer.content-draft-missing",
    ),
    (
      "trial.G.draft-count-mismatch",
      "held.macro-only-receipt-file-writer.draft-count-mismatch",
    ),
    (
      "trial.H.artifact-count-mismatch",
      "held.macro-only-receipt-file-writer.artifact-count-mismatch",
    ),
    (
      "trial.I.source-draft-overclaim",
      "held.macro-only-receipt-file-writer.source-draft-overclaim",
    ),
    (
      "trial.J.artifact-authority-overclaim",
      "held.macro-only-receipt-file-writer.artifact-authority-overclaim",
    ),
    (
      "trial.K.artifact-shape-mismatch",
      "held.macro-only-receipt-file-writer.artifact-shape-mismatch",
    ),
    (
      "trial.M.disk-write-overclaim",
      "held.macro-only-receipt-file-writer.disk-write-overclaim",
    ),
    (
      "trial.N.auto-approval-overclaim",
      "held.macro-only-receipt-file-writer.auto-approval-overclaim",
    ),
    (
      "trial.O.target-frontier-overclaim",
      "held.macro-only-receipt-file-writer.target-frontier-overclaim",
    ),
    (
      "trial.P.delete-overclaim",
      "held.macro-only-receipt-file-writer.delete-or-command-overclaim",
    ),
    (
      "trial.Q.runtime-overclaim",
      "held.macro-only-receipt-file-writer.runtime-overclaim",
    ),
    (
      "trial.R.p-puck-semantic-owner",
      "held.macro-only-receipt-file-writer.p-puck-semantic-owner",
    ),
    (
      "trial.S.old-host-authority",
      "held.macro-only-receipt-file-writer.old-host-authority",
    ),
    (
      "trial.T.gpl-family-dependency",
      "held.macro-only-receipt-file-writer.gpl-family-dependency",
    ),
  ] {
    assert_eq!(as_str(get(trials[id], "held-id")), held, "{id}");
  }
}

#[test]
fn six_layer_fold_keeps_writer_artifact_separate_from_runtime() {
  let run = eval_fixture();
  let fold = get(run, "six-layer-receipt-file-writer-fold");
  assert_eq!(as_str(get(fold, "mode")), "macro-only-receipt-file-writer");
  assert!(as_bool(get(get(fold, "semantic"), "receipt-file-writer")));
  assert!(as_bool(get(
    get(fold, "semantic"),
    "receipt-file-artifact-generated"
  )));
  assert!(as_bool(get(get(fold, "semantic"), "writer-candidate-only")));
  assert!(!as_bool(get(get(fold, "semantic"), "receipt-file-created")));
  assert!(!as_bool(get(
    get(fold, "semantic"),
    "receipt-content-written"
  )));
  assert!(!as_bool(get(
    get(fold, "semantic"),
    "target-frontier-closed"
  )));
  let runtime = get(fold, "runtime");
  assert!(as_bool(get(runtime, "receipt-file-writer")));
  for key in [
    "receipt-file-created",
    "receipt-content-written",
    "receipt-auto-written",
    "receipt-auto-approved",
    "target-frontier-closed",
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
fn discoveries_record_d556_through_d563() {
  let run = eval_fixture();
  let discoveries = attrs_by_id(get(run, "discoveries"));
  assert_eq!(discoveries.len(), 8);
  for expected in [
    "D556.content-drafts-can-lower-to-file-artifact-candidates",
    "D557.file-artifacts-preserve-target-owner-and-path",
    "D558.file-writer-candidate-is-not-disk-materialization",
    "D559.rendered-body-is-structured-candidate-not-approval",
    "D560.writer-hard-stops-block-approval-target-runtime-collapse",
    "D561.one-file-artifact-per-draft-keeps-five-lane-split",
    "D562.writer-output-is-materialization-input-not-implementation-command",
    "D563.next-frontier-is-materialization-proof-not-approval-or-runtime",
  ] {
    assert!(
      discoveries.contains_key(expected),
      "missing discovery `{expected}`"
    );
  }
}

#[test]
fn top_level_state_records_writer_candidate_only_no_runtime_or_db() {
  let run = eval_fixture();
  assert_eq!(
    as_str(get(run, "replacement-readiness")),
    "receipt-file-writer-candidate-present"
  );
  assert!(as_bool(get(run, "receipt-file-writer")));
  assert!(as_bool(get(run, "receipt-file-artifact-generated")));
  assert!(as_bool(get(run, "writer-candidate-only")));
  assert_eq!(as_i64(get(run, "written-artifact-count")), 5);
  assert_eq!(as_i64(get(run, "covered-draft-count")), 5);
  for key in [
    "receipt-file-created",
    "receipt-content-written",
    "receipt-auto-written",
    "receipt-auto-approved",
    "target-frontier-closed",
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
    assert!(!as_bool(get(run, key)), "`{key}` must stay false");
  }
}
