use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/tesseract-macro-legacy-probe/macro_only_receipt_file_materialization_proof_receipt.px",
  )
}

fn eval_fixture() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = fixture_path();
    let json = std::thread::Builder::new()
      .name("macro-only-receipt-file-materialization-proof-eval".to_string())
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("receipt file materialization proof receipt")
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
    "tesseract-macro-ontology-macro-only-receipt-file-materialization-proof"
  );
  assert_eq!(
    as_str(get(run, "constitution-owner")),
    "stdlib/lib/gate/tesseract-constitution.px"
  );
  assert_eq!(
    as_str(get(run, "source-receipt")),
    "tesseract-macro-ontology-macro-only-receipt-file-writer"
  );
}

#[test]
fn constitution_gate_blocks_materialization_proof_collapse() {
  let run = eval_fixture();
  let gate = get(run, "constitutionGate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "macro-only-receipt-file-materialization-proof"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "materialization-proof-equals-disk-write",
    "materialization-proof-equals-receipt-file-created",
    "materialization-proof-equals-receipt-content-written",
    "materialization-proof-equals-auto-write",
    "materialization-proof-equals-auto-approval",
    "materialization-proof-equals-target-frontier-closed",
    "materialization-proof-equals-delete-ready",
    "materialization-proof-equals-implementation-command",
    "materialization-proof-equals-global-runtime-install",
    "materialization-proof-equals-runtime-api-flattening",
    "materialization-proof-equals-meaning-db",
    "materialization-proof-equals-p-puck-semantic-owner",
    "old-host-code-authorizes-materialization",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
}

#[test]
fn proof_generates_five_materialization_records() {
  let run = eval_fixture();
  let proof = get(run, "receipt-file-materialization-proof");
  assert_eq!(
    as_str(get(proof, "status")),
    "receipt-file-materialization-proof-present"
  );
  assert!(as_bool(get(proof, "receipt-file-materialization-proof")));
  assert!(as_bool(get(proof, "materialization-proof-only")));
  assert_eq!(as_i64(get(proof, "materialization-proof-count")), 5);
  assert_eq!(as_i64(get(proof, "source-artifact-count")), 5);
  assert_eq!(as_list(get(proof, "materialization-proofs")).len(), 5);
  assert!(string_set(get(proof, "closes"))
    .contains("need.self.receipt-file-materialization-proof-after-writer-candidate"));
  assert!(string_set(get(proof, "next-open-frontiers"))
    .contains("receipt-file-disk-write-after-materialization-proof"));
}

#[test]
fn materialization_records_preserve_paths_without_disk_write_or_target_closure() {
  let run = eval_fixture();
  let records = attrs_by_id(get(run, "materialization-proofs"));
  assert_eq!(records.len(), 5);
  let record = records[
    "materialization.file.draft.review.skeleton.candidate.receipt.macro-only-host-removal-delete-ready-target-proof"
  ];
  assert_eq!(
    as_str(get(record, "target-receipt")),
    "fixtures/tesseract-macro-legacy-probe/macro_only_host_removal_delete_ready_target_proof_receipt.px"
  );
  assert_eq!(
    as_str(get(record, "file-path")),
    "fixtures/tesseract-macro-legacy-probe/macro_only_host_removal_delete_ready_target_proof_receipt.px"
  );
  assert!(as_bool(get(record, "receipt-file-materialization-proof")));
  assert!(as_bool(get(record, "materialization-proof-only")));
  for key in [
    "disk-write-executed",
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
    assert!(!as_bool(get(record, key)), "`{key}` must stay false");
  }
}

#[test]
fn contract_closes_materialization_proof_only() {
  let run = eval_fixture();
  let contract = get(run, "receipt-file-materialization-contract");
  assert_eq!(
    as_str(get(contract, "proof-id")),
    "proof.macro-only.receipt-file-materialization.v1"
  );
  assert!(as_bool(get(contract, "closes-materialization-proof")));
  assert_eq!(as_i64(get(contract, "materialization-proof-count")), 5);
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
fn migration_delta_closes_only_materialization_proof_frontier() {
  let run = eval_fixture();
  let delta = get(run, "migrationDelta");
  let closes = string_set(get(delta, "closes"));
  assert!(closes.contains("need.self.receipt-file-materialization-proof-after-writer-candidate"));
  assert_eq!(closes.len(), 1);
  let not_closed = string_set(get(delta, "does-not-close"));
  for expected in [
    "need.self.receipt-file-disk-write-after-materialization-proof",
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
    .contains("receipt-file-disk-write-after-materialization-proof"));
}

#[test]
fn trials_cover_valid_source_proof_shape_and_held_boundaries() {
  let run = eval_fixture();
  let trials = attrs_by_id(get(run, "receipt-file-materialization-trials"));
  assert_eq!(trials.len(), 20);
  assert_eq!(
    as_str(get(
      trials["trial.A.valid-materialization-proof"],
      "outcome"
    )),
    "receipt-file-materialization-proof-present"
  );
  for (id, held) in [
    (
      "trial.C.wrong-proof-id",
      "held.macro-only-receipt-file-materialization.proof-id-mismatch",
    ),
    (
      "trial.D.stale-stage",
      "held.macro-only-receipt-file-materialization.stale-current-stage",
    ),
    (
      "trial.E.source-mismatch",
      "held.macro-only-receipt-file-materialization.source-mismatch",
    ),
    (
      "trial.F.writer-artifact-missing",
      "held.macro-only-receipt-file-materialization.writer-artifact-missing",
    ),
    (
      "trial.G.artifact-count-mismatch",
      "held.macro-only-receipt-file-materialization.artifact-count-mismatch",
    ),
    (
      "trial.H.proof-count-mismatch",
      "held.macro-only-receipt-file-materialization.proof-count-mismatch",
    ),
    (
      "trial.I.source-artifact-overclaim",
      "held.macro-only-receipt-file-materialization.source-artifact-overclaim",
    ),
    (
      "trial.J.proof-authority-overclaim",
      "held.macro-only-receipt-file-materialization.proof-authority-overclaim",
    ),
    (
      "trial.K.proof-shape-mismatch",
      "held.macro-only-receipt-file-materialization.proof-shape-mismatch",
    ),
    (
      "trial.M.disk-write-overclaim",
      "held.macro-only-receipt-file-materialization.disk-write-overclaim",
    ),
    (
      "trial.N.auto-approval-overclaim",
      "held.macro-only-receipt-file-materialization.auto-approval-overclaim",
    ),
    (
      "trial.O.target-frontier-overclaim",
      "held.macro-only-receipt-file-materialization.target-frontier-overclaim",
    ),
    (
      "trial.P.delete-overclaim",
      "held.macro-only-receipt-file-materialization.delete-or-command-overclaim",
    ),
    (
      "trial.Q.runtime-overclaim",
      "held.macro-only-receipt-file-materialization.runtime-overclaim",
    ),
    (
      "trial.R.p-puck-semantic-owner",
      "held.macro-only-receipt-file-materialization.p-puck-semantic-owner",
    ),
    (
      "trial.S.old-host-authority",
      "held.macro-only-receipt-file-materialization.old-host-authority",
    ),
    (
      "trial.T.gpl-family-dependency",
      "held.macro-only-receipt-file-materialization.gpl-family-dependency",
    ),
  ] {
    assert_eq!(as_str(get(trials[id], "held-id")), held, "{id}");
  }
}

#[test]
fn six_layer_fold_keeps_materialization_proof_separate_from_runtime() {
  let run = eval_fixture();
  let fold = get(run, "six-layer-receipt-file-materialization-fold");
  assert_eq!(
    as_str(get(fold, "mode")),
    "macro-only-receipt-file-materialization-proof"
  );
  assert!(as_bool(get(
    get(fold, "semantic"),
    "receipt-file-materialization-proof"
  )));
  assert!(as_bool(get(
    get(fold, "semantic"),
    "materialization-proof-only"
  )));
  assert!(!as_bool(get(get(fold, "semantic"), "disk-write-executed")));
  assert!(!as_bool(get(get(fold, "semantic"), "receipt-file-created")));
  assert!(!as_bool(get(
    get(fold, "semantic"),
    "target-frontier-closed"
  )));
  let runtime = get(fold, "runtime");
  assert!(as_bool(get(runtime, "receipt-file-materialization-proof")));
  for key in [
    "disk-write-executed",
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
fn discoveries_record_d564_through_d571() {
  let run = eval_fixture();
  let discoveries = attrs_by_id(get(run, "discoveries"));
  assert_eq!(discoveries.len(), 8);
  for expected in [
    "D564.materialization-proof-is-separate-from-disk-write",
    "D565.materialization-proofs-preserve-artifact-targets",
    "D566.one-proof-per-artifact-keeps-five-lane-split",
    "D567.repo-local-path-and-section-proof-precede-disk-write",
    "D568.materialization-proof-record-is-not-approved-target-receipt",
    "D569.materialization-proof-opens-disk-write-frontier",
    "D570.materialization-hard-stops-block-target-runtime-collapse",
    "D571.proof-only-keeps-auto-approval-and-runtime-separate",
  ] {
    assert!(
      discoveries.contains_key(expected),
      "missing discovery `{expected}`"
    );
  }
}

#[test]
fn top_level_state_records_materialization_proof_only_no_runtime_or_db() {
  let run = eval_fixture();
  assert_eq!(
    as_str(get(run, "replacement-readiness")),
    "receipt-file-materialization-proof-present"
  );
  assert!(as_bool(get(
    run,
    "receipt-file-materialization-proof-present"
  )));
  assert!(as_bool(get(run, "materialization-proof-only")));
  assert_eq!(as_i64(get(run, "materialization-proof-count")), 5);
  assert_eq!(as_i64(get(run, "source-artifact-count")), 5);
  for key in [
    "disk-write-executed",
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
