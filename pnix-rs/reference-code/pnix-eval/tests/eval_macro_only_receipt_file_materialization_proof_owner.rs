use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/pnix-query-runtime/macro-only-receipt-file-materialization-proof-owner.px",
  )
}

fn eval_fixture() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = fixture_path();
    let json = std::thread::Builder::new()
      .name("macro-only-receipt-file-materialization-proof-owner-eval".to_string())
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("receipt file materialization proof owner")
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
fn owner_fixture_uses_writer_source_and_stage() {
  let run = eval_fixture();
  assert!(as_bool(get(run, "imported-owner")));
  assert!(as_bool(get(run, "used-px-owner")));
  assert_eq!(
    as_str(get(run, "imported-source-receipt")),
    "tesseract-macro-ontology-macro-only-receipt-file-writer"
  );
  assert_eq!(
    as_str(get(run, "source-receipt-status")),
    "receipt-file-writer-candidate-present"
  );
  assert_eq!(
    as_str(get(run, "expected-current-stage")),
    "receipt-file-writer-candidate-present"
  );
}

#[test]
fn valid_proof_emits_five_materialization_proof_records() {
  let run = eval_fixture();
  let proof = get(run, "valid-proof");
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
fn materialization_records_preserve_target_path_and_hard_stops() {
  let run = eval_fixture();
  let proof = get(run, "valid-proof");
  let records = attrs_by_id(get(proof, "materialization-proofs"));
  let record = records[
    "materialization.file.draft.review.skeleton.candidate.receipt.macro-only-host-removal-delete-ready-target-proof"
  ];
  assert_eq!(
    as_str(get(record, "target-owner")),
    "stdlib/lib/gate/macro-only-host-removal-delete-ready-target-proof.px"
  );
  assert_eq!(
    as_str(get(record, "target-receipt")),
    "fixtures/tesseract-macro-legacy-probe/macro_only_host_removal_delete_ready_target_proof_receipt.px"
  );
  assert_eq!(
    as_str(get(record, "file-path")),
    "fixtures/tesseract-macro-legacy-probe/macro_only_host_removal_delete_ready_target_proof_receipt.px"
  );
  assert_eq!(
    as_str(get(record, "next-action")),
    "receipt-file-disk-write-after-materialization-proof"
  );
  let hard_stops = string_set(get(record, "hard-stops"));
  for expected in [
    "no-disk-write-executed",
    "no-receipt-file-created",
    "no-receipt-content-written",
    "no-target-frontier-closed",
    "no-delete-ready",
    "no-implementation-command",
    "no-runtime-install",
    "no-meaning-db",
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
fn valid_proof_does_not_claim_disk_write_or_target_closure() {
  let run = eval_fixture();
  let proof = get(run, "valid-proof");
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
    assert!(!as_bool(get(proof, key)), "`{key}` must stay false");
  }
}

#[test]
fn held_trials_cover_source_and_shape_failures() {
  let run = eval_fixture();
  for (key, held) in [
    (
      "wrong-proof",
      "held.macro-only-receipt-file-materialization.proof-id-mismatch",
    ),
    (
      "stale-stage",
      "held.macro-only-receipt-file-materialization.stale-current-stage",
    ),
    (
      "source-mismatch",
      "held.macro-only-receipt-file-materialization.source-mismatch",
    ),
    (
      "writer-artifact-missing",
      "held.macro-only-receipt-file-materialization.writer-artifact-missing",
    ),
    (
      "artifact-count-mismatch",
      "held.macro-only-receipt-file-materialization.artifact-count-mismatch",
    ),
    (
      "proof-count-mismatch",
      "held.macro-only-receipt-file-materialization.proof-count-mismatch",
    ),
    (
      "source-artifact-overclaim",
      "held.macro-only-receipt-file-materialization.source-artifact-overclaim",
    ),
    (
      "proof-authority-overclaim",
      "held.macro-only-receipt-file-materialization.proof-authority-overclaim",
    ),
    (
      "proof-shape-mismatch",
      "held.macro-only-receipt-file-materialization.proof-shape-mismatch",
    ),
    (
      "missing-section",
      "held.macro-only-receipt-file-materialization.proof-shape-mismatch",
    ),
  ] {
    let trial = get(run, key);
    assert_eq!(as_str(get(trial, "status")), "Held", "{key}");
    assert_eq!(as_str(get(trial, "held-id")), held, "{key}");
  }
}

#[test]
fn held_trials_block_authority_overclaims() {
  let run = eval_fixture();
  for (key, held) in [
    (
      "disk-write-claim",
      "held.macro-only-receipt-file-materialization.disk-write-overclaim",
    ),
    (
      "auto-approval-claim",
      "held.macro-only-receipt-file-materialization.auto-approval-overclaim",
    ),
    (
      "target-frontier-claim",
      "held.macro-only-receipt-file-materialization.target-frontier-overclaim",
    ),
    (
      "delete-claim",
      "held.macro-only-receipt-file-materialization.delete-or-command-overclaim",
    ),
    (
      "runtime-claim",
      "held.macro-only-receipt-file-materialization.runtime-overclaim",
    ),
    (
      "semantic-owner-claim",
      "held.macro-only-receipt-file-materialization.p-puck-semantic-owner",
    ),
    (
      "old-host-authority-case",
      "held.macro-only-receipt-file-materialization.old-host-authority",
    ),
    (
      "gpl-claim",
      "held.macro-only-receipt-file-materialization.gpl-family-dependency",
    ),
  ] {
    let trial = get(run, key);
    assert_eq!(as_str(get(trial, "status")), "Held", "{key}");
    assert_eq!(as_str(get(trial, "held-id")), held, "{key}");
  }
}

#[test]
fn required_sections_and_evidence_are_explicit() {
  let run = eval_fixture();
  let sections = string_set(get(run, "required-proof-sections"));
  for expected in [
    "source-file-artifact",
    "target-path-proof",
    "repo-local-path-proof",
    "section-body-proof",
    "hard-stop-proof",
    "write-boundary-proof",
    "target-frontier-boundary",
    "next-frontier",
    "negative-held-evidence",
  ] {
    assert!(sections.contains(expected), "missing section `{expected}`");
  }
  let evidence = string_set(get(run, "required-evidence"));
  for expected in [
    "receipt-file-writer-candidate-present",
    "receipt-file-artifact-generated",
    "one-materialization-proof-per-artifact",
    "receipt-file-materialization-proof",
    "no-disk-write-executed",
    "no-receipt-file-created",
    "no-target-frontier-closed",
    "no-gpl-family-dependencies",
  ] {
    assert!(evidence.contains(expected), "missing evidence `{expected}`");
  }
}

#[test]
fn top_level_fixture_records_materialization_proof_only() {
  let run = eval_fixture();
  assert!(as_bool(get(run, "receipt-file-materialization-proof")));
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
