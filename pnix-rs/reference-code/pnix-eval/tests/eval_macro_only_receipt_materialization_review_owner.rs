use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/macro-only-receipt-materialization-review-owner.px")
}

fn eval_fixture() -> Value {
  let path = fixture_path();
  let json = std::thread::Builder::new()
    .name("receipt-materialization-review-owner-eval".to_string())
    .stack_size(32 * 1024 * 1024)
    .spawn(move || {
      eval_to_json(path.to_str().expect("utf-8 path"), true)
        .expect("receipt materialization review owner fixture")
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
fn fixture_imports_owner_and_skeleton_source() {
  let run = eval_fixture();
  assert_eq!(
    as_str(get(&run, "proof")),
    "macro-only-receipt-materialization-review-owner"
  );
  assert!(as_bool(get(&run, "imported-owner")));
  assert!(as_bool(get(&run, "used-px-owner")));
  assert_eq!(
    as_str(get(&run, "imported-source-receipt")),
    "tesseract-macro-ontology-macro-only-receipt-skeleton-generator"
  );
  assert_eq!(
    as_str(get(&run, "source-receipt-status")),
    "receipt-skeleton-generator-present-data-only"
  );
}

#[test]
fn owner_meta_declares_review_only_materialization_gate() {
  let run = eval_fixture();
  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.lib.gate.macro-only-receipt-materialization-review"
  );
  assert_eq!(
    as_str(get(meta, "constructor")),
    "validateReceiptMaterializationReview"
  );
  assert!(as_bool(get(meta, "receipt-materialization-review")));
  assert!(as_bool(get(meta, "materialization-reviewed")));
  assert!(as_bool(get(meta, "materialization-review-only")));
  assert_eq!(as_i64(get(meta, "reviewed-skeleton-count")), 5);
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
    "runtime-install",
    "global-ontology-runtime",
    "runtime-api-flattening",
    "meaning-db",
    "new-engine-from-zero",
    "p-puck-is-semantic-owner",
    "old-host-authority",
    "gpl-family-dependencies",
  ] {
    assert!(!as_bool(get(meta, key)), "`{key}` must stay false");
  }
}

#[test]
fn valid_proof_reviews_five_skeletons_without_materializing_files() {
  let run = eval_fixture();
  let valid = get(&run, "valid-proof");
  assert_eq!(
    as_str(get(valid, "status")),
    "receipt-materialization-review-present"
  );
  assert!(get(valid, "held-id").is_null());
  assert!(as_bool(get(valid, "receipt-materialization-review")));
  assert!(as_bool(get(valid, "materialization-reviewed")));
  assert!(as_bool(get(valid, "materialization-review-only")));
  assert_eq!(as_i64(get(valid, "reviewed-skeleton-count")), 5);
  assert_eq!(as_i64(get(valid, "covered-skeleton-count")), 5);
  assert_eq!(as_list(get(valid, "materialization-reviews")).len(), 5);
  assert!(string_set(get(valid, "closes"))
    .contains("need.self.receipt-skeleton-materialization-review-after-data-skeleton"));
  assert!(string_set(get(valid, "next-open-frontiers"))
    .contains("receipt-content-draft-generation-after-materialization-review"));
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
    assert!(!as_bool(get(valid, key)), "`{key}` must stay false");
  }
}

#[test]
fn review_objects_preserve_skeleton_targets() {
  let run = eval_fixture();
  let reviews = attrs_by_id(get(get(&run, "valid-proof"), "materialization-reviews"));
  assert_eq!(reviews.len(), 5);
  let delete_ready =
    reviews["review.skeleton.candidate.receipt.macro-only-host-removal-delete-ready-target-proof"];
  assert_eq!(
    as_str(get(delete_ready, "source-skeleton-id")),
    "skeleton.candidate.receipt.macro-only-host-removal-delete-ready-target-proof"
  );
  assert_eq!(
    as_str(get(delete_ready, "target-owner")),
    "stdlib/lib/gate/macro-only-host-removal-delete-ready-target-proof.px"
  );
  assert_eq!(
    as_str(get(delete_ready, "target-receipt")),
    "fixtures/tesseract-macro-legacy-probe/macro_only_host_removal_delete_ready_target_proof_receipt.px"
  );
  assert_eq!(
    as_str(get(delete_ready, "review-status")),
    "materialization-review-ready"
  );
  assert_eq!(as_str(get(delete_ready, "materialization")), "review-only");
  assert_eq!(as_str(get(delete_ready, "authority")), "review-only");
  assert!(!as_bool(get(delete_ready, "receipt-file-created")));
  assert!(!as_bool(get(delete_ready, "receipt-content-written")));
  assert!(!as_bool(get(delete_ready, "content-draft-generated")));
  assert!(!as_bool(get(delete_ready, "receipt-auto-approved")));
}

#[test]
fn source_skeleton_and_review_shape_failures_are_held() {
  let run = eval_fixture();
  for (key, held_id) in [
    (
      "wrong-proof",
      "held.macro-only-receipt-materialization-review.proof-id-mismatch",
    ),
    (
      "stale-stage",
      "held.macro-only-receipt-materialization-review.stale-current-stage",
    ),
    (
      "source-mismatch",
      "held.macro-only-receipt-materialization-review.source-mismatch",
    ),
    (
      "skeleton-generator-missing",
      "held.macro-only-receipt-materialization-review.skeleton-generator-missing",
    ),
    (
      "skeleton-count-mismatch",
      "held.macro-only-receipt-materialization-review.skeleton-count-mismatch",
    ),
    (
      "review-count-mismatch",
      "held.macro-only-receipt-materialization-review.review-count-mismatch",
    ),
    (
      "skeleton-materialization-overclaim",
      "held.macro-only-receipt-materialization-review.skeleton-materialization-overclaim",
    ),
    (
      "materialization-authority-overclaim",
      "held.macro-only-receipt-materialization-review.materialization-authority-overclaim",
    ),
    (
      "review-shape-mismatch",
      "held.macro-only-receipt-materialization-review.review-shape-mismatch",
    ),
    (
      "missing-section",
      "held.macro-only-receipt-materialization-review.review-shape-mismatch",
    ),
  ] {
    let case = get(&run, key);
    assert_eq!(as_str(get(case, "status")), "Held", "{key}");
    assert_eq!(as_str(get(case, "held-id")), held_id, "{key}");
  }
}

#[test]
fn file_approval_delete_runtime_owner_and_license_claims_are_held() {
  let run = eval_fixture();
  for (key, held_id) in [
    (
      "file-or-content-claim",
      "held.macro-only-receipt-materialization-review.file-or-content-overclaim",
    ),
    (
      "auto-approval-claim",
      "held.macro-only-receipt-materialization-review.auto-approval-overclaim",
    ),
    (
      "delete-claim",
      "held.macro-only-receipt-materialization-review.delete-or-command-overclaim",
    ),
    (
      "runtime-claim",
      "held.macro-only-receipt-materialization-review.runtime-overclaim",
    ),
    (
      "semantic-owner-claim",
      "held.macro-only-receipt-materialization-review.p-puck-semantic-owner",
    ),
    (
      "old-host-authority",
      "held.macro-only-receipt-materialization-review.old-host-authority",
    ),
    (
      "gpl-claim",
      "held.macro-only-receipt-materialization-review.gpl-family-dependency",
    ),
  ] {
    let case = get(&run, key);
    assert_eq!(as_str(get(case, "status")), "Held", "{key}");
    assert_eq!(as_str(get(case, "held-id")), held_id, "{key}");
  }
}

#[test]
fn required_evidence_records_review_without_file_or_runtime_shortcuts() {
  let run = eval_fixture();
  let evidence = string_set(get(&run, "required-evidence"));
  for expected in [
    "receipt-skeleton-generator-present",
    "skeleton-data-only",
    "five-generated-skeletons-present",
    "one-review-per-skeleton",
    "review-count-matches-skeletons",
    "review-targets-preserve-skeleton-targets",
    "review-sections-complete",
    "review-hard-stops-complete",
    "materialization-review-only",
    "no-receipt-file-created",
    "no-receipt-content-written",
    "no-content-draft-generated",
    "no-auto-approval",
    "no-host-removal",
    "no-implementation-command",
    "no-runtime-install",
    "no-runtime-api-flattening",
    "no-meaning-db",
    "no-gpl-family-dependencies",
  ] {
    assert!(evidence.contains(expected), "missing evidence `{expected}`");
  }
}

#[test]
fn top_level_state_records_materialization_review_only() {
  let run = eval_fixture();
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
    "runtime-install",
    "global-ontology-runtime",
    "runtime-api-flattening",
    "meaning-db",
    "p-puck-is-semantic-owner",
    "gpl-family-dependencies",
  ] {
    assert!(!as_bool(get(&run, key)), "`{key}` must stay false");
  }
}
