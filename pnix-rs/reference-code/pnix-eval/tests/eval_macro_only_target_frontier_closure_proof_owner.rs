use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/macro-only-target-frontier-closure-proof-owner.px")
}

fn eval_fixture() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = fixture_path();
    let json = std::thread::Builder::new()
      .name("macro-only-target-frontier-closure-proof-owner-eval".to_string())
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("target frontier closure proof owner")
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
fn owner_fixture_uses_auto_approval_source_and_stage() {
  let run = eval_fixture();
  assert!(as_bool(get(run, "imported-owner")));
  assert!(as_bool(get(run, "used-px-owner")));
  assert_eq!(
    as_str(get(run, "imported-source-receipt")),
    "tesseract-macro-ontology-macro-only-receipt-auto-approval-proof"
  );
  assert_eq!(
    as_str(get(run, "source-receipt-status")),
    "receipt-auto-approval-proof-present"
  );
  assert_eq!(
    as_str(get(run, "expected-current-stage")),
    "receipt-auto-approval-proof-present"
  );
}

#[test]
fn valid_proof_emits_five_target_frontier_closure_records() {
  let run = eval_fixture();
  let proof = get(run, "valid-proof");
  assert_eq!(
    as_str(get(proof, "status")),
    "target-frontier-closure-proof-present"
  );
  assert!(as_bool(get(proof, "target-frontier-closure-proof")));
  assert!(as_bool(get(proof, "target-frontier-closure-proof-only")));
  assert_eq!(as_i64(get(proof, "target-frontier-closure-count")), 5);
  assert_eq!(as_i64(get(proof, "source-auto-approval-proof-count")), 5);
  assert_eq!(as_list(get(proof, "target-frontier-closures")).len(), 5);
  assert!(string_set(get(proof, "closes"))
    .contains("target-frontier-closure-after-receipt-auto-approval-proof"));
  assert!(!string_set(get(proof, "next-open-frontiers"))
    .contains("target-frontier-closure-after-receipt-auto-approval-proof"));
}

#[test]
fn closure_records_preserve_targets_and_close_only_receipt_target_frontier() {
  let run = eval_fixture();
  let proof = get(run, "valid-proof");
  let records = attrs_by_id(get(proof, "target-frontier-closures"));
  let record = records[
    "target-frontier-closure.auto-approval.content-write.file-creation.disk-write.materialization.file.draft.review.skeleton.candidate.receipt.macro-only-host-removal-delete-ready-target-proof"
  ];
  assert_eq!(
    as_str(get(record, "target-owner")),
    "stdlib/lib/gate/macro-only-host-removal-delete-ready-target-proof.px"
  );
  assert_eq!(
    as_str(get(record, "target-receipt")),
    "fixtures/tesseract-macro-legacy-probe/macro_only_host_removal_delete_ready_target_proof_receipt.px"
  );
  assert!(as_bool(get(record, "receipt-target-frontier-closed")));
  assert!(as_bool(get(record, "target-frontier-closed")));
  assert!(!as_bool(get(record, "underlying-work-frontier-closed")));
  let plan = get(record, "closure-plan");
  assert_eq!(
    as_str(get(plan, "closes-meta-frontier")),
    "target-frontier-closure-after-receipt-auto-approval-proof"
  );
  assert!(as_bool(get(plan, "underlying-work-deferred")));
  assert!(as_bool(get(plan, "delete-ready-still-separate")));
  assert!(as_bool(get(plan, "runtime-install-still-separate")));
}

#[test]
fn required_sections_and_evidence_are_explicit() {
  let run = eval_fixture();
  let sections = string_set(get(run, "required-target-closure-sections"));
  for expected in [
    "source-auto-approval-proof",
    "target-path-proof",
    "target-frontier-identity-proof",
    "approval-proof-preserved",
    "closed-target-frontier-proof",
    "underlying-work-deferred-proof",
    "negative-held-evidence",
  ] {
    assert!(sections.contains(expected), "missing section `{expected}`");
  }
  let evidence = string_set(get(run, "required-evidence"));
  for expected in [
    "receipt-auto-approval-proof-present",
    "receipt-auto-approval-proof",
    "one-target-frontier-closure-per-auto-approval-proof",
    "target-frontier-closure-proof",
    "target-frontier-closure-proof-only",
    "closed-target-frontier-proof",
    "underlying-work-deferred-proof",
    "no-auto-approval",
    "no-delete-ready",
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
      "held.macro-only-target-frontier-closure.proof-id-mismatch",
    ),
    (
      "stale-stage",
      "held.macro-only-target-frontier-closure.stale-current-stage",
    ),
    (
      "source-mismatch",
      "held.macro-only-target-frontier-closure.source-mismatch",
    ),
    (
      "auto-approval-missing",
      "held.macro-only-target-frontier-closure.auto-approval-proof-missing",
    ),
    (
      "auto-approval-count-mismatch",
      "held.macro-only-target-frontier-closure.auto-approval-count-mismatch",
    ),
    (
      "closure-count-mismatch",
      "held.macro-only-target-frontier-closure.closure-count-mismatch",
    ),
    (
      "source-auto-approval-overclaim",
      "held.macro-only-target-frontier-closure.source-auto-approval-overclaim",
    ),
    (
      "closure-authority-overclaim",
      "held.macro-only-target-frontier-closure.closure-authority-overclaim",
    ),
    (
      "closure-shape-mismatch",
      "held.macro-only-target-frontier-closure.proof-shape-mismatch",
    ),
    (
      "missing-section",
      "held.macro-only-target-frontier-closure.proof-shape-mismatch",
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
      "content-or-approval-claim",
      "held.macro-only-target-frontier-closure.content-or-approval-overclaim",
    ),
    (
      "target-frontier-missing",
      "held.macro-only-target-frontier-closure.target-frontier-not-closed",
    ),
    (
      "underlying-frontier-claim",
      "held.macro-only-target-frontier-closure.underlying-frontier-overclaim",
    ),
    (
      "delete-claim",
      "held.macro-only-target-frontier-closure.delete-or-command-overclaim",
    ),
    (
      "runtime-claim",
      "held.macro-only-target-frontier-closure.runtime-overclaim",
    ),
    (
      "semantic-owner-claim",
      "held.macro-only-target-frontier-closure.p-puck-semantic-owner",
    ),
    (
      "old-host-authority-case",
      "held.macro-only-target-frontier-closure.old-host-authority",
    ),
    (
      "gpl-claim",
      "held.macro-only-target-frontier-closure.gpl-family-dependency",
    ),
  ] {
    let trial = get(run, key);
    assert_eq!(as_str(get(trial, "status")), "Held", "{key}");
    assert_eq!(as_str(get(trial, "held-id")), held, "{key}");
  }
}

#[test]
fn top_level_flags_close_target_frontier_but_keep_runtime_false() {
  let run = eval_fixture();
  assert!(as_bool(get(run, "target-frontier-closure-proof")));
  assert!(as_bool(get(run, "target-frontier-closure-proof-only")));
  assert_eq!(as_i64(get(run, "target-frontier-closure-count")), 5);
  assert!(!as_bool(get(run, "receipt-auto-approved")));
  assert!(as_bool(get(run, "target-frontier-closed")));
  assert!(!as_bool(get(run, "underlying-work-frontier-closed")));
  for key in [
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

#[test]
fn owner_meta_records_scope_and_remaining_frontiers() {
  let run = eval_fixture();
  let meta = get(run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.lib.gate.macro-only-target-frontier-closure-proof"
  );
  assert!(as_bool(get(meta, "target-frontier-closure-proof")));
  assert!(as_bool(get(meta, "target-frontier-closed")));
  assert!(!as_bool(get(meta, "underlying-work-frontier-closed")));
  assert!(string_set(get(meta, "next-open-frontiers"))
    .contains("actual-host-removal-implementation-command"));
}
