use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/tesseract-macro-legacy-probe/macro_only_receipt_content_write_proof_receipt.px",
  )
}

fn eval_fixture() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = fixture_path();
    let json = std::thread::Builder::new()
      .name("macro-only-receipt-content-write-proof-eval".to_string())
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("receipt content write proof receipt")
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
    "tesseract-macro-ontology-macro-only-receipt-content-write-proof"
  );
  assert_eq!(
    as_str(get(run, "constitution-owner")),
    "stdlib/lib/gate/tesseract-constitution.px"
  );
  assert_eq!(
    as_str(get(run, "source-receipt")),
    "tesseract-macro-ontology-macro-only-receipt-file-creation-proof"
  );
}

#[test]
fn constitution_gate_blocks_content_write_proof_collapse() {
  let run = eval_fixture();
  let gate = get(run, "constitutionGate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "macro-only-receipt-content-write-proof"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "content-write-proof-equals-receipt-content-written",
    "content-write-proof-equals-auto-write",
    "content-write-proof-equals-auto-approval",
    "content-write-proof-equals-target-frontier-closed",
    "content-write-proof-equals-delete-ready",
    "content-write-proof-equals-implementation-command",
    "content-write-proof-equals-global-runtime-install",
    "content-write-proof-equals-runtime-api-flattening",
    "content-write-proof-equals-meaning-db",
    "content-write-proof-equals-p-puck-semantic-owner",
    "old-host-code-authorizes-content-write",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
}

#[test]
fn proof_generates_five_content_write_records() {
  let run = eval_fixture();
  let proof = get(run, "receipt-content-write-proof");
  assert_eq!(
    as_str(get(proof, "status")),
    "receipt-content-write-proof-present"
  );
  assert!(as_bool(get(proof, "receipt-content-write-proof")));
  assert!(as_bool(get(proof, "content-write-proof-only")));
  assert_eq!(as_i64(get(proof, "content-write-proof-count")), 5);
  assert_eq!(as_i64(get(proof, "source-file-creation-proof-count")), 5);
  assert_eq!(as_list(get(proof, "content-write-proofs")).len(), 5);
  assert!(string_set(get(proof, "closes"))
    .contains("need.self.receipt-content-write-after-file-creation-proof"));
  assert!(string_set(get(proof, "next-open-frontiers"))
    .contains("receipt-auto-approval-after-content-write-proof"));
}

#[test]
fn content_write_records_preserve_paths_without_content_or_target_closure() {
  let run = eval_fixture();
  let records = attrs_by_id(get(run, "content-write-proofs"));
  assert_eq!(records.len(), 5);
  let record = records[
    "content-write.file-creation.disk-write.materialization.file.draft.review.skeleton.candidate.receipt.macro-only-host-removal-delete-ready-target-proof"
  ];
  assert_eq!(
    as_str(get(record, "target-receipt")),
    "fixtures/tesseract-macro-legacy-probe/macro_only_host_removal_delete_ready_target_proof_receipt.px"
  );
  assert_eq!(
    as_str(get(record, "file-path")),
    "fixtures/tesseract-macro-legacy-probe/macro_only_host_removal_delete_ready_target_proof_receipt.px"
  );
  assert!(as_bool(get(record, "receipt-content-write-proof")));
  assert!(as_bool(get(record, "content-write-proof-only")));
  assert!(as_bool(get(record, "content-template-proof")));
  assert!(as_bool(get(record, "content-section-proof")));
  assert!(as_bool(get(record, "content-nonempty-proof")));
  assert!(as_bool(get(record, "approval-deferred-proof")));
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
fn contract_closes_content_write_proof_only() {
  let run = eval_fixture();
  let contract = get(run, "receipt-content-write-contract");
  assert_eq!(
    as_str(get(contract, "proof-id")),
    "proof.macro-only.receipt-content-write.v1"
  );
  assert!(as_bool(get(contract, "closes-content-write-proof")));
  assert_eq!(as_i64(get(contract, "content-write-proof-count")), 5);
  for key in [
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
fn migration_delta_closes_only_content_write_proof_frontier() {
  let run = eval_fixture();
  let delta = get(run, "migrationDelta");
  let closes = string_set(get(delta, "closes"));
  assert!(closes.contains("need.self.receipt-content-write-after-file-creation-proof"));
  assert_eq!(closes.len(), 1);
  let not_closed = string_set(get(delta, "does-not-close"));
  for expected in [
    "need.self.receipt-auto-approval-after-content-write-proof",
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
  let trials = attrs_by_id(get(run, "receipt-content-write-trials"));
  assert_eq!(trials.len(), 20);
  assert_eq!(
    as_str(get(trials["trial.A.valid-content-write-proof"], "outcome")),
    "receipt-content-write-proof-present"
  );
  for (id, held) in [
    (
      "trial.C.wrong-proof-id",
      "held.macro-only-receipt-content-write.proof-id-mismatch",
    ),
    (
      "trial.D.stale-stage",
      "held.macro-only-receipt-content-write.stale-current-stage",
    ),
    (
      "trial.E.source-mismatch",
      "held.macro-only-receipt-content-write.source-mismatch",
    ),
    (
      "trial.F.file-creation-missing",
      "held.macro-only-receipt-content-write.file-creation-proof-missing",
    ),
    (
      "trial.G.file-creation-count-mismatch",
      "held.macro-only-receipt-content-write.file-creation-count-mismatch",
    ),
    (
      "trial.H.proof-count-mismatch",
      "held.macro-only-receipt-content-write.proof-count-mismatch",
    ),
    (
      "trial.I.source-file-creation-overclaim",
      "held.macro-only-receipt-content-write.source-file-creation-overclaim",
    ),
    (
      "trial.J.proof-authority-overclaim",
      "held.macro-only-receipt-content-write.proof-authority-overclaim",
    ),
    (
      "trial.K.proof-shape-mismatch",
      "held.macro-only-receipt-content-write.proof-shape-mismatch",
    ),
    (
      "trial.M.content-write-overclaim",
      "held.macro-only-receipt-content-write.content-write-overclaim",
    ),
    (
      "trial.N.auto-approval-overclaim",
      "held.macro-only-receipt-content-write.auto-approval-overclaim",
    ),
    (
      "trial.O.target-frontier-overclaim",
      "held.macro-only-receipt-content-write.target-frontier-overclaim",
    ),
    (
      "trial.P.delete-overclaim",
      "held.macro-only-receipt-content-write.delete-or-command-overclaim",
    ),
    (
      "trial.Q.runtime-overclaim",
      "held.macro-only-receipt-content-write.runtime-overclaim",
    ),
    (
      "trial.R.p-puck-semantic-owner",
      "held.macro-only-receipt-content-write.p-puck-semantic-owner",
    ),
    (
      "trial.S.old-host-authority",
      "held.macro-only-receipt-content-write.old-host-authority",
    ),
    (
      "trial.T.gpl-family-dependency",
      "held.macro-only-receipt-content-write.gpl-family-dependency",
    ),
  ] {
    assert_eq!(as_str(get(trials[id], "held-id")), held, "{id}");
  }
}

#[test]
fn discoveries_record_d588_through_d595() {
  let run = eval_fixture();
  let discoveries = attrs_by_id(get(run, "discoveries"));
  assert_eq!(discoveries.len(), 8);
  for expected in [
    "D588.content-write-proof-is-separate-from-written-receipt-content",
    "D589.content-write-proofs-preserve-file-creation-targets",
    "D590.one-content-write-proof-per-file-creation-keeps-five-lane-split",
    "D591.template-section-and-nonempty-proofs-precede-future-write",
    "D592.content-write-proof-is-not-auto-written-or-approved-content",
    "D593.content-write-proof-opens-auto-approval-frontier",
    "D594.content-write-hard-stops-block-runtime-delete-and-approval-collapse",
    "D595.content-write-proof-keeps-approval-and-target-frontier-separate",
  ] {
    assert!(discoveries.contains_key(expected), "missing {expected}");
  }
}

#[test]
fn top_level_flags_keep_actual_content_and_runtime_false() {
  let run = eval_fixture();
  assert!(as_bool(get(run, "receipt-content-write-proof-present")));
  assert!(as_bool(get(run, "content-write-proof-only")));
  assert_eq!(as_i64(get(run, "content-write-proof-count")), 5);
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
