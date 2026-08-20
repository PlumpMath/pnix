use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/tesseract-macro-legacy-probe/macro_only_receipt_file_creation_proof_receipt.px",
  )
}

fn eval_fixture() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = fixture_path();
    let json = std::thread::Builder::new()
      .name("macro-only-receipt-file-creation-proof-eval".to_string())
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("receipt file creation proof receipt")
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
    "tesseract-macro-ontology-macro-only-receipt-file-creation-proof"
  );
  assert_eq!(
    as_str(get(run, "constitution-owner")),
    "stdlib/lib/gate/tesseract-constitution.px"
  );
  assert_eq!(
    as_str(get(run, "source-receipt")),
    "tesseract-macro-ontology-macro-only-receipt-file-disk-write-proof"
  );
}

#[test]
fn constitution_gate_blocks_file_creation_proof_collapse() {
  let run = eval_fixture();
  let gate = get(run, "constitutionGate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "macro-only-receipt-file-creation-proof"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "file-creation-proof-equals-receipt-file-created",
    "file-creation-proof-equals-receipt-content-written",
    "file-creation-proof-equals-auto-write",
    "file-creation-proof-equals-auto-approval",
    "file-creation-proof-equals-target-frontier-closed",
    "file-creation-proof-equals-delete-ready",
    "file-creation-proof-equals-implementation-command",
    "file-creation-proof-equals-global-runtime-install",
    "file-creation-proof-equals-runtime-api-flattening",
    "file-creation-proof-equals-meaning-db",
    "file-creation-proof-equals-p-puck-semantic-owner",
    "old-host-code-authorizes-file-creation",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
}

#[test]
fn proof_generates_five_file_creation_records() {
  let run = eval_fixture();
  let proof = get(run, "receipt-file-creation-proof");
  assert_eq!(
    as_str(get(proof, "status")),
    "receipt-file-creation-proof-present"
  );
  assert!(as_bool(get(proof, "receipt-file-creation-proof")));
  assert!(as_bool(get(proof, "file-creation-proof-only")));
  assert_eq!(as_i64(get(proof, "file-creation-proof-count")), 5);
  assert_eq!(as_i64(get(proof, "source-disk-write-proof-count")), 5);
  assert_eq!(as_list(get(proof, "file-creation-proofs")).len(), 5);
  assert!(string_set(get(proof, "closes"))
    .contains("need.self.receipt-file-creation-after-disk-write-proof"));
  assert!(string_set(get(proof, "next-open-frontiers"))
    .contains("receipt-content-write-after-file-creation-proof"));
}

#[test]
fn file_creation_records_preserve_paths_without_content_or_target_closure() {
  let run = eval_fixture();
  let records = attrs_by_id(get(run, "file-creation-proofs"));
  assert_eq!(records.len(), 5);
  let record = records[
    "file-creation.disk-write.materialization.file.draft.review.skeleton.candidate.receipt.macro-only-host-removal-delete-ready-target-proof"
  ];
  assert_eq!(
    as_str(get(record, "target-receipt")),
    "fixtures/tesseract-macro-legacy-probe/macro_only_host_removal_delete_ready_target_proof_receipt.px"
  );
  assert_eq!(
    as_str(get(record, "file-path")),
    "fixtures/tesseract-macro-legacy-probe/macro_only_host_removal_delete_ready_target_proof_receipt.px"
  );
  assert!(as_bool(get(record, "receipt-file-creation-proof")));
  assert!(as_bool(get(record, "file-creation-proof-only")));
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
    assert!(!as_bool(get(record, key)), "`{key}` must stay false");
  }
}

#[test]
fn contract_closes_file_creation_proof_only() {
  let run = eval_fixture();
  let contract = get(run, "receipt-file-creation-contract");
  assert_eq!(
    as_str(get(contract, "proof-id")),
    "proof.macro-only.receipt-file-creation.v1"
  );
  assert!(as_bool(get(contract, "closes-file-creation-proof")));
  assert_eq!(as_i64(get(contract, "file-creation-proof-count")), 5);
  for key in [
    "closes-actual-file-creation",
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
fn migration_delta_closes_only_file_creation_proof_frontier() {
  let run = eval_fixture();
  let delta = get(run, "migrationDelta");
  let closes = string_set(get(delta, "closes"));
  assert!(closes.contains("need.self.receipt-file-creation-after-disk-write-proof"));
  assert_eq!(closes.len(), 1);
  let not_closed = string_set(get(delta, "does-not-close"));
  for expected in [
    "need.self.receipt-content-write-after-file-creation-proof",
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
}

#[test]
fn trials_cover_valid_source_proof_shape_and_held_boundaries() {
  let run = eval_fixture();
  let trials = attrs_by_id(get(run, "receipt-file-creation-trials"));
  assert_eq!(trials.len(), 20);
  assert_eq!(
    as_str(get(trials["trial.A.valid-file-creation-proof"], "outcome")),
    "receipt-file-creation-proof-present"
  );
  for (id, held) in [
    (
      "trial.C.wrong-proof-id",
      "held.macro-only-receipt-file-creation.proof-id-mismatch",
    ),
    (
      "trial.D.stale-stage",
      "held.macro-only-receipt-file-creation.stale-current-stage",
    ),
    (
      "trial.E.source-mismatch",
      "held.macro-only-receipt-file-creation.source-mismatch",
    ),
    (
      "trial.F.disk-write-missing",
      "held.macro-only-receipt-file-creation.disk-write-proof-missing",
    ),
    (
      "trial.G.disk-write-count-mismatch",
      "held.macro-only-receipt-file-creation.disk-write-count-mismatch",
    ),
    (
      "trial.H.proof-count-mismatch",
      "held.macro-only-receipt-file-creation.proof-count-mismatch",
    ),
    (
      "trial.I.source-disk-write-overclaim",
      "held.macro-only-receipt-file-creation.source-disk-write-overclaim",
    ),
    (
      "trial.J.proof-authority-overclaim",
      "held.macro-only-receipt-file-creation.proof-authority-overclaim",
    ),
    (
      "trial.K.proof-shape-mismatch",
      "held.macro-only-receipt-file-creation.proof-shape-mismatch",
    ),
    (
      "trial.M.file-creation-overclaim",
      "held.macro-only-receipt-file-creation.file-creation-overclaim",
    ),
    (
      "trial.N.auto-approval-overclaim",
      "held.macro-only-receipt-file-creation.auto-approval-overclaim",
    ),
    (
      "trial.O.target-frontier-overclaim",
      "held.macro-only-receipt-file-creation.target-frontier-overclaim",
    ),
    (
      "trial.P.delete-overclaim",
      "held.macro-only-receipt-file-creation.delete-or-command-overclaim",
    ),
    (
      "trial.Q.runtime-overclaim",
      "held.macro-only-receipt-file-creation.runtime-overclaim",
    ),
    (
      "trial.R.p-puck-semantic-owner",
      "held.macro-only-receipt-file-creation.p-puck-semantic-owner",
    ),
    (
      "trial.S.old-host-authority",
      "held.macro-only-receipt-file-creation.old-host-authority",
    ),
    (
      "trial.T.gpl-family-dependency",
      "held.macro-only-receipt-file-creation.gpl-family-dependency",
    ),
  ] {
    assert_eq!(as_str(get(trials[id], "held-id")), held, "{id}");
  }
}

#[test]
fn discoveries_record_d580_through_d587() {
  let run = eval_fixture();
  let discoveries = attrs_by_id(get(run, "discoveries"));
  assert_eq!(discoveries.len(), 8);
  for expected in [
    "D580.file-creation-proof-is-separate-from-actual-file-creation",
    "D581.file-creation-proofs-preserve-disk-write-targets",
    "D582.one-file-creation-proof-per-disk-write-keeps-five-lane-split",
    "D583.parent-directory-and-empty-file-forbidden-precede-creation",
    "D584.file-creation-proof-is-not-written-or-approved-receipt-content",
    "D585.file-creation-proof-opens-content-write-frontier",
    "D586.file-creation-hard-stops-block-runtime-delete-and-write-collapse",
    "D587.file-creation-proof-keeps-approval-and-target-frontier-separate",
  ] {
    assert!(discoveries.contains_key(expected), "missing {expected}");
  }
}

#[test]
fn top_level_flags_keep_actual_file_creation_and_runtime_false() {
  let run = eval_fixture();
  assert!(as_bool(get(run, "receipt-file-creation-proof-present")));
  assert!(as_bool(get(run, "file-creation-proof-only")));
  assert_eq!(as_i64(get(run, "file-creation-proof-count")), 5);
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
    "new-engine-from-zero",
    "p-puck-is-semantic-owner",
    "old-host-authority",
    "gpl-family-dependencies",
  ] {
    assert!(!as_bool(get(run, key)), "`{key}` must stay false");
  }
}
