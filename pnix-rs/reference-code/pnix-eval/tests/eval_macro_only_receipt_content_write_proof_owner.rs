use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/macro-only-receipt-content-write-proof-owner.px")
}

fn eval_fixture() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = fixture_path();
    let json = std::thread::Builder::new()
      .name("macro-only-receipt-content-write-proof-owner-eval".to_string())
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("receipt content write proof owner")
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
fn owner_fixture_uses_file_creation_source_and_stage() {
  let run = eval_fixture();
  assert!(as_bool(get(run, "imported-owner")));
  assert!(as_bool(get(run, "used-px-owner")));
  assert_eq!(
    as_str(get(run, "imported-source-receipt")),
    "tesseract-macro-ontology-macro-only-receipt-file-creation-proof"
  );
  assert_eq!(
    as_str(get(run, "source-receipt-status")),
    "receipt-file-creation-proof-present"
  );
  assert_eq!(
    as_str(get(run, "expected-current-stage")),
    "receipt-file-creation-proof-present"
  );
}

#[test]
fn valid_proof_emits_five_content_write_proof_records() {
  let run = eval_fixture();
  let proof = get(run, "valid-proof");
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
fn content_write_records_preserve_path_and_remain_proof_only() {
  let run = eval_fixture();
  let proof = get(run, "valid-proof");
  let records = attrs_by_id(get(proof, "content-write-proofs"));
  let record = records[
    "content-write.file-creation.disk-write.materialization.file.draft.review.skeleton.candidate.receipt.macro-only-host-removal-delete-ready-target-proof"
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
    "receipt-auto-approval-after-content-write-proof"
  );
  let plan = get(record, "content-plan");
  assert!(as_bool(get(plan, "dry-run-only")));
  assert!(as_bool(get(plan, "template-required")));
  assert!(as_bool(get(plan, "sections-required")));
  assert!(as_bool(get(plan, "nonempty-content-required")));
  assert!(as_bool(get(plan, "approval-required-after-content-write")));
  for key in [
    "receipt-file-created",
    "receipt-content-written",
    "receipt-auto-written",
    "receipt-auto-approved",
    "target-frontier-closed",
    "delete-ready",
    "implementation-command",
    "runtime-install",
    "meaning-db",
  ] {
    assert!(!as_bool(get(record, key)), "`{key}` must stay false");
  }
}

#[test]
fn required_sections_and_evidence_are_explicit() {
  let run = eval_fixture();
  let sections = string_set(get(run, "required-content-write-sections"));
  for expected in [
    "source-file-creation-proof",
    "target-path-proof",
    "content-template-proof",
    "content-section-proof",
    "content-nonempty-proof",
    "approval-deferred-proof",
    "negative-held-evidence",
  ] {
    assert!(sections.contains(expected), "missing section `{expected}`");
  }
  let evidence = string_set(get(run, "required-evidence"));
  for expected in [
    "receipt-file-creation-proof-present",
    "receipt-file-creation-proof",
    "one-content-write-proof-per-file-creation-proof",
    "receipt-content-write-proof",
    "content-write-proof-only",
    "content-template-proof",
    "content-section-proof",
    "content-nonempty-proof",
    "approval-deferred-proof",
    "no-receipt-content-written",
    "no-gpl-family-dependencies",
  ] {
    assert!(evidence.contains(expected), "missing evidence `{expected}`");
  }
}

#[test]
fn held_trials_cover_source_and_shape_failures() {
  let run = eval_fixture();
  for (key, held) in [
    (
      "wrong-proof",
      "held.macro-only-receipt-content-write.proof-id-mismatch",
    ),
    (
      "stale-stage",
      "held.macro-only-receipt-content-write.stale-current-stage",
    ),
    (
      "source-mismatch",
      "held.macro-only-receipt-content-write.source-mismatch",
    ),
    (
      "file-creation-missing",
      "held.macro-only-receipt-content-write.file-creation-proof-missing",
    ),
    (
      "file-creation-count-mismatch",
      "held.macro-only-receipt-content-write.file-creation-count-mismatch",
    ),
    (
      "proof-count-mismatch",
      "held.macro-only-receipt-content-write.proof-count-mismatch",
    ),
    (
      "source-file-creation-overclaim",
      "held.macro-only-receipt-content-write.source-file-creation-overclaim",
    ),
    (
      "proof-authority-overclaim",
      "held.macro-only-receipt-content-write.proof-authority-overclaim",
    ),
    (
      "proof-shape-mismatch",
      "held.macro-only-receipt-content-write.proof-shape-mismatch",
    ),
    (
      "missing-section",
      "held.macro-only-receipt-content-write.proof-shape-mismatch",
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
      "content-write-claim",
      "held.macro-only-receipt-content-write.content-write-overclaim",
    ),
    (
      "auto-approval-claim",
      "held.macro-only-receipt-content-write.auto-approval-overclaim",
    ),
    (
      "target-frontier-claim",
      "held.macro-only-receipt-content-write.target-frontier-overclaim",
    ),
    (
      "delete-claim",
      "held.macro-only-receipt-content-write.delete-or-command-overclaim",
    ),
    (
      "runtime-claim",
      "held.macro-only-receipt-content-write.runtime-overclaim",
    ),
    (
      "semantic-owner-claim",
      "held.macro-only-receipt-content-write.p-puck-semantic-owner",
    ),
    (
      "old-host-authority-case",
      "held.macro-only-receipt-content-write.old-host-authority",
    ),
    (
      "gpl-claim",
      "held.macro-only-receipt-content-write.gpl-family-dependency",
    ),
  ] {
    let trial = get(run, key);
    assert_eq!(as_str(get(trial, "status")), "Held", "{key}");
    assert_eq!(as_str(get(trial, "held-id")), held, "{key}");
  }
}

#[test]
fn meta_records_content_write_proof_boundary() {
  let run = eval_fixture();
  let meta = get(run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "constructor")),
    "validateReceiptContentWriteProof"
  );
  assert!(as_bool(get(meta, "receipt-content-write-proof")));
  assert!(as_bool(get(meta, "content-write-proof-only")));
  assert!(!as_bool(get(meta, "receipt-content-written")));
  assert!(!as_bool(get(meta, "receipt-auto-approved")));
  assert!(!as_bool(get(meta, "target-frontier-closed")));
  assert!(!as_bool(get(meta, "runtime-api-flattening")));
  assert!(!as_bool(get(meta, "meaning-db")));
}

#[test]
fn top_level_flags_keep_actual_content_and_runtime_false() {
  let run = eval_fixture();
  assert!(as_bool(get(run, "receipt-content-write-proof")));
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
