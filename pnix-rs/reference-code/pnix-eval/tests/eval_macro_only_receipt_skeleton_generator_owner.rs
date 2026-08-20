use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/macro-only-receipt-skeleton-generator-owner.px")
}

fn eval_fixture() -> Value {
  let path = fixture_path();
  let json = std::thread::Builder::new()
    .name("receipt-skeleton-generator-owner-eval".to_string())
    .stack_size(32 * 1024 * 1024)
    .spawn(move || {
      eval_to_json(path.to_str().expect("utf-8 path"), true)
        .expect("receipt skeleton generator owner fixture")
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
fn fixture_imports_owner_and_source_emission() {
  let run = eval_fixture();
  assert_eq!(
    as_str(get(&run, "proof")),
    "macro-only-receipt-skeleton-generator-owner"
  );
  assert!(as_bool(get(&run, "imported-owner")));
  assert!(as_bool(get(&run, "used-px-owner")));
  assert_eq!(
    as_str(get(&run, "imported-source-receipt")),
    "tesseract-macro-ontology-macro-only-self-receipt-frontier-emission"
  );
  assert_eq!(
    as_str(get(&run, "source-receipt-status")),
    "self-receipt-frontier-emission-present-candidate-only"
  );
}

#[test]
fn owner_meta_declares_data_only_skeleton_generator() {
  let run = eval_fixture();
  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.lib.gate.macro-only-receipt-skeleton-generator"
  );
  assert_eq!(
    as_str(get(meta, "constructor")),
    "validateReceiptSkeletonGenerator"
  );
  assert!(as_bool(get(meta, "receipt-skeleton-generator")));
  assert!(as_bool(get(meta, "skeleton-data-only")));
  assert_eq!(as_i64(get(meta, "generated-skeleton-count")), 5);
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
    "new-engine-from-zero",
    "p-puck-is-semantic-owner",
    "old-host-authority",
    "gpl-family-dependencies",
  ] {
    assert!(!as_bool(get(meta, key)), "`{key}` must stay false");
  }
}

#[test]
fn expected_skeletons_preserve_candidates_and_sections() {
  let run = eval_fixture();
  let sections = string_set(get(&run, "required-skeleton-sections"));
  for expected in [
    "probe-marker",
    "truth-owner",
    "constitution-owner",
    "source-candidate",
    "contract",
    "trials",
    "six-layer-fold",
    "migration-delta",
    "discoveries",
    "negative-held-evidence",
    "hard-stops",
    "proof-placeholders",
  ] {
    assert!(sections.contains(expected), "missing section `{expected}`");
  }

  let skeletons = attrs_by_id(get(get(&run, "valid-proof"), "generated-receipt-skeletons"));
  assert_eq!(skeletons.len(), 5);
  let delete_ready =
    skeletons["skeleton.candidate.receipt.macro-only-host-removal-delete-ready-target-proof"];
  assert_eq!(
    as_str(get(delete_ready, "source-candidate-id")),
    "candidate.receipt.macro-only-host-removal-delete-ready-target-proof"
  );
  assert_eq!(
    as_str(get(delete_ready, "target-owner")),
    "stdlib/lib/gate/macro-only-host-removal-delete-ready-target-proof.px"
  );
  assert_eq!(
    as_str(get(delete_ready, "target-receipt")),
    "fixtures/tesseract-macro-legacy-probe/macro_only_host_removal_delete_ready_target_proof_receipt.px"
  );
  assert_eq!(as_str(get(delete_ready, "materialization")), "data-only");
  assert_eq!(as_str(get(delete_ready, "authority")), "skeleton-data-only");
  assert!(!as_bool(get(delete_ready, "receipt-file-created")));
  assert!(!as_bool(get(delete_ready, "receipt-content-written")));
  assert!(!as_bool(get(delete_ready, "receipt-auto-approved")));
}

#[test]
fn valid_proof_generates_skeleton_data_only() {
  let run = eval_fixture();
  let valid = get(&run, "valid-proof");
  assert_eq!(
    as_str(get(valid, "status")),
    "receipt-skeleton-generator-present"
  );
  assert!(get(valid, "held-id").is_null());
  assert!(as_bool(get(valid, "receipt-skeleton-generator")));
  assert!(as_bool(get(valid, "skeleton-data-only")));
  assert_eq!(as_i64(get(valid, "generated-skeleton-count")), 5);
  assert_eq!(as_i64(get(valid, "covered-candidate-count")), 5);
  assert_eq!(as_list(get(valid, "generated-receipt-skeletons")).len(), 5);
  assert!(string_set(get(valid, "closes"))
    .contains("need.self.receipt-skeleton-generator-after-frontier-emission"));
  assert!(string_set(get(valid, "next-open-frontiers"))
    .contains("receipt-skeleton-materialization-review-after-data-skeleton"));
  for key in [
    "receipt-file-created",
    "receipt-content-written",
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
fn source_candidate_and_skeleton_shape_failures_are_held() {
  let run = eval_fixture();
  for (key, held_id) in [
    (
      "wrong-proof",
      "held.macro-only-receipt-skeleton-generator.proof-id-mismatch",
    ),
    (
      "stale-stage",
      "held.macro-only-receipt-skeleton-generator.stale-current-stage",
    ),
    (
      "source-mismatch",
      "held.macro-only-receipt-skeleton-generator.source-mismatch",
    ),
    (
      "candidate-count-mismatch",
      "held.macro-only-receipt-skeleton-generator.candidate-count-mismatch",
    ),
    (
      "skeleton-count-mismatch",
      "held.macro-only-receipt-skeleton-generator.skeleton-count-mismatch",
    ),
    (
      "candidate-authority-overclaim",
      "held.macro-only-receipt-skeleton-generator.candidate-authority-overclaim",
    ),
    (
      "materialization-overclaim",
      "held.macro-only-receipt-skeleton-generator.materialization-overclaim",
    ),
    (
      "skeleton-authority-overclaim",
      "held.macro-only-receipt-skeleton-generator.skeleton-authority-overclaim",
    ),
    (
      "skeleton-shape-mismatch",
      "held.macro-only-receipt-skeleton-generator.skeleton-shape-mismatch",
    ),
    (
      "missing-section",
      "held.macro-only-receipt-skeleton-generator.skeleton-shape-mismatch",
    ),
  ] {
    let case = get(&run, key);
    assert_eq!(as_str(get(case, "status")), "Held", "{key}");
    assert_eq!(as_str(get(case, "held-id")), held_id, "{key}");
  }
}

#[test]
fn writer_approval_delete_runtime_owner_and_license_claims_are_held() {
  let run = eval_fixture();
  for (key, held_id) in [
    (
      "file-writer-claim",
      "held.macro-only-receipt-skeleton-generator.file-writer-overclaim",
    ),
    (
      "auto-approval-claim",
      "held.macro-only-receipt-skeleton-generator.auto-approval-overclaim",
    ),
    (
      "delete-claim",
      "held.macro-only-receipt-skeleton-generator.delete-or-command-overclaim",
    ),
    (
      "runtime-claim",
      "held.macro-only-receipt-skeleton-generator.runtime-overclaim",
    ),
    (
      "semantic-owner-claim",
      "held.macro-only-receipt-skeleton-generator.p-puck-semantic-owner",
    ),
    (
      "old-host-authority",
      "held.macro-only-receipt-skeleton-generator.old-host-authority",
    ),
    (
      "gpl-claim",
      "held.macro-only-receipt-skeleton-generator.gpl-family-dependency",
    ),
  ] {
    let case = get(&run, key);
    assert_eq!(as_str(get(case, "status")), "Held", "{key}");
    assert_eq!(as_str(get(case, "held-id")), held_id, "{key}");
  }
}

#[test]
fn required_evidence_records_no_file_or_runtime_shortcuts() {
  let run = eval_fixture();
  let evidence = string_set(get(&run, "required-evidence"));
  for expected in [
    "self-receipt-frontier-emission-present",
    "receipt-needed-detector-present",
    "source-candidates-present",
    "skeleton-count-matches-candidates",
    "one-skeleton-per-candidate",
    "skeleton-sections-complete",
    "skeletons-data-only",
    "no-receipt-file-created",
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
fn top_level_state_records_skeleton_generator_only() {
  let run = eval_fixture();
  assert!(as_bool(get(&run, "receipt-skeleton-generator")));
  assert!(as_bool(get(&run, "skeleton-data-only")));
  assert_eq!(as_i64(get(&run, "generated-skeleton-count")), 5);
  assert_eq!(as_i64(get(&run, "covered-candidate-count")), 5);
  assert_eq!(
    as_list(get(get(&run, "valid-proof"), "generated-receipt-skeletons")).len(),
    5
  );
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
    "gpl-family-dependencies",
  ] {
    assert!(!as_bool(get(&run, key)), "`{key}` must stay false");
  }
}
