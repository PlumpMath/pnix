use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/macro-only-receipt-auto-approval-proof-owner.px")
}

fn eval_fixture() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = fixture_path();
    let json = std::thread::Builder::new()
      .name("macro-only-receipt-auto-approval-proof-owner-eval".to_string())
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("receipt auto-approval proof owner")
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
fn owner_fixture_uses_content_write_source_and_stage() {
  let run = eval_fixture();
  assert!(as_bool(get(run, "imported-owner")));
  assert!(as_bool(get(run, "used-px-owner")));
  assert_eq!(
    as_str(get(run, "imported-source-receipt")),
    "tesseract-macro-ontology-macro-only-receipt-content-write-proof"
  );
  assert_eq!(
    as_str(get(run, "source-receipt-status")),
    "receipt-content-write-proof-present"
  );
  assert_eq!(
    as_str(get(run, "expected-current-stage")),
    "receipt-content-write-proof-present"
  );
}

#[test]
fn valid_proof_emits_five_auto_approval_proof_records() {
  let run = eval_fixture();
  let proof = get(run, "valid-proof");
  assert_eq!(
    as_str(get(proof, "status")),
    "receipt-auto-approval-proof-present"
  );
  assert!(as_bool(get(proof, "receipt-auto-approval-proof")));
  assert!(as_bool(get(proof, "auto-approval-proof-only")));
  assert_eq!(as_i64(get(proof, "auto-approval-proof-count")), 5);
  assert_eq!(as_i64(get(proof, "source-content-write-proof-count")), 5);
  assert_eq!(as_list(get(proof, "auto-approval-proofs")).len(), 5);
  assert!(string_set(get(proof, "closes"))
    .contains("need.self.receipt-auto-approval-after-content-write-proof"));
  assert!(string_set(get(proof, "next-open-frontiers"))
    .contains("target-frontier-closure-after-receipt-auto-approval-proof"));
}

#[test]
fn auto_approval_records_preserve_path_and_remain_proof_only() {
  let run = eval_fixture();
  let proof = get(run, "valid-proof");
  let records = attrs_by_id(get(proof, "auto-approval-proofs"));
  let record = records[
    "auto-approval.content-write.file-creation.disk-write.materialization.file.draft.review.skeleton.candidate.receipt.macro-only-host-removal-delete-ready-target-proof"
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
    "target-frontier-closure-after-receipt-auto-approval-proof"
  );
  let plan = get(record, "approval-plan");
  assert!(as_bool(get(plan, "dry-run-only")));
  assert!(as_bool(get(plan, "content-write-proof-required")));
  assert!(as_bool(get(plan, "approval-scope-required")));
  assert!(as_bool(get(
    plan,
    "target-frontier-closure-required-after-approval"
  )));
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
  let sections = string_set(get(run, "required-auto-approval-sections"));
  for expected in [
    "source-content-write-proof",
    "target-path-proof",
    "approval-preflight-proof",
    "approval-scope-proof",
    "approval-non-execution-proof",
    "target-frontier-deferred-proof",
    "negative-held-evidence",
  ] {
    assert!(sections.contains(expected), "missing section `{expected}`");
  }
  let evidence = string_set(get(run, "required-evidence"));
  for expected in [
    "receipt-content-write-proof-present",
    "receipt-content-write-proof",
    "one-auto-approval-proof-per-content-write-proof",
    "receipt-auto-approval-proof",
    "auto-approval-proof-only",
    "approval-preflight-proof",
    "approval-scope-proof",
    "approval-non-execution-proof",
    "target-frontier-deferred-proof",
    "no-auto-approval",
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
      "held.macro-only-receipt-auto-approval.proof-id-mismatch",
    ),
    (
      "stale-stage",
      "held.macro-only-receipt-auto-approval.stale-current-stage",
    ),
    (
      "source-mismatch",
      "held.macro-only-receipt-auto-approval.source-mismatch",
    ),
    (
      "content-write-missing",
      "held.macro-only-receipt-auto-approval.content-write-proof-missing",
    ),
    (
      "content-write-count-mismatch",
      "held.macro-only-receipt-auto-approval.content-write-count-mismatch",
    ),
    (
      "proof-count-mismatch",
      "held.macro-only-receipt-auto-approval.proof-count-mismatch",
    ),
    (
      "source-content-write-overclaim",
      "held.macro-only-receipt-auto-approval.source-content-write-overclaim",
    ),
    (
      "proof-authority-overclaim",
      "held.macro-only-receipt-auto-approval.proof-authority-overclaim",
    ),
    (
      "proof-shape-mismatch",
      "held.macro-only-receipt-auto-approval.proof-shape-mismatch",
    ),
    (
      "missing-section",
      "held.macro-only-receipt-auto-approval.proof-shape-mismatch",
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
      "held.macro-only-receipt-auto-approval.content-or-write-overclaim",
    ),
    (
      "auto-approval-claim",
      "held.macro-only-receipt-auto-approval.actual-approval-overclaim",
    ),
    (
      "target-frontier-claim",
      "held.macro-only-receipt-auto-approval.target-frontier-overclaim",
    ),
    (
      "delete-claim",
      "held.macro-only-receipt-auto-approval.delete-or-command-overclaim",
    ),
    (
      "runtime-claim",
      "held.macro-only-receipt-auto-approval.runtime-overclaim",
    ),
    (
      "semantic-owner-claim",
      "held.macro-only-receipt-auto-approval.p-puck-semantic-owner",
    ),
    (
      "old-host-authority-case",
      "held.macro-only-receipt-auto-approval.old-host-authority",
    ),
    (
      "gpl-claim",
      "held.macro-only-receipt-auto-approval.gpl-family-dependency",
    ),
  ] {
    let trial = get(run, key);
    assert_eq!(as_str(get(trial, "status")), "Held", "{key}");
    assert_eq!(as_str(get(trial, "held-id")), held, "{key}");
  }
}

#[test]
fn meta_records_auto_approval_proof_boundary() {
  let run = eval_fixture();
  let meta = get(run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "constructor")),
    "validateReceiptAutoApprovalProof"
  );
  assert!(as_bool(get(meta, "receipt-auto-approval-proof")));
  assert!(as_bool(get(meta, "auto-approval-proof-only")));
  assert!(!as_bool(get(meta, "receipt-auto-approved")));
  assert!(!as_bool(get(meta, "target-frontier-closed")));
  assert!(!as_bool(get(meta, "runtime-api-flattening")));
  assert!(!as_bool(get(meta, "meaning-db")));
}

#[test]
fn top_level_flags_keep_actual_approval_and_runtime_false() {
  let run = eval_fixture();
  assert!(as_bool(get(run, "receipt-auto-approval-proof")));
  assert!(as_bool(get(run, "auto-approval-proof-only")));
  assert_eq!(as_i64(get(run, "auto-approval-proof-count")), 5);
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
