//! Bootstrap status audit for the tesseract macro ontology line.
//!
//! This is not a feature-claim test. It pins the correction boundary: current
//! receipts are evaluated macro substrate, but PNIX has not yet booted a fresh
//! macro-only ontology runtime with old host ontology code removed.

use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join(
    "../../fixtures/tesseract-macro-legacy-probe/macro_ontology_bootstrap_status_audit_receipt.px",
  )
}

fn eval_fixture() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = fixture_path();
    let json = std::thread::Builder::new()
      .name("bootstrap-status-audit-eval".to_string())
      .stack_size(64 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("bootstrap status audit must evaluate")
      })
      .expect("spawn eval thread")
      .join()
      .expect("eval thread panicked");
    serde_json::from_str(&json).expect("bootstrap status JSON")
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

fn get_path<'a>(root: &'a Value, path: &[&str]) -> &'a Value {
  let mut cur = root;
  for key in path {
    cur = get(cur, key);
  }
  cur
}

fn list_strings(v: &Value) -> Vec<&str> {
  as_list(v).iter().map(as_str).collect()
}

fn string_set(v: &Value) -> BTreeSet<&str> {
  list_strings(v).into_iter().collect()
}

fn attrs_by_key<'a>(items: &'a Value, key: &str) -> BTreeMap<&'a str, &'a Value> {
  as_list(items)
    .iter()
    .map(|item| (as_str(get(item, key)), item))
    .collect()
}

#[test]
fn bootstrap_status_marker_and_constitution_owner_are_pinned() {
  let run = eval_fixture();
  assert_eq!(
    as_str(get(run, "probe-marker")),
    "tesseract-macro-ontology-bootstrap-status-audit"
  );
  assert_eq!(
    as_str(get(run, "truth-owner")),
    "project-wiki/maps/tesseract-macro-ontology-discovery-ledger.md"
  );
  assert_eq!(
    as_str(get(run, "replacement-map")),
    "project-wiki/maps/tesseract-macro-ontology-replacement-map.md"
  );
  assert_eq!(
    as_str(get(run, "constitution-owner")),
    "stdlib/lib/gate/tesseract-constitution.px"
  );
}

#[test]
fn constitution_gate_blocks_bootstrap_overclaims() {
  let run = eval_fixture();
  let gate = get(run, "constitution-gate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "macro-ontology-bootstrap-status-audit"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));

  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "call-receipt-eval-a-fresh-runtime-boot",
    "hide-old-stdlib-ontology-imports",
    "hide-pnix-runtime-legacy-ontology-builtins",
    "treat-p-puck-wrapper-ok-as-owner-switch",
    "treat-lift-query-emit-r7-compat-as-runtime-install",
    "delete-lift-query-emit-without-r7-delete-proof",
    "treat-macro-only-boot-manifest-as-boot-execution",
    "treat-macro-only-boot-attempt-as-successful-boot",
    "treat-macro-only-boot-runner-owner-as-successful-boot",
    "treat-bounded-replay-strategy-as-successful-boot",
    "treat-regression-corpus-retention-as-successful-boot",
    "treat-regression-corpus-retention-as-fresh-puck-or-compare",
    "treat-bootstrap-audit-update-as-successful-boot",
    "treat-bootstrap-audit-update-as-fresh-puck-or-compare",
    "treat-compare-after-boot-as-successful-boot",
    "treat-compare-after-boot-as-fresh-puck-or-semantic-owner",
    "treat-target-delete-preflight-as-target-delete-proof",
    "treat-target-delete-preflight-as-host-removal",
    "treat-fresh-p-puck-as-full-current-receipt-audit",
    "treat-fresh-p-puck-as-replay-executed",
    "treat-fresh-p-puck-as-successful-boot",
    "treat-fresh-p-puck-as-host-removal",
    "treat-bounded-replay-execution-as-successful-boot",
    "treat-bounded-replay-execution-as-host-removal",
    "treat-bounded-replay-execution-as-semantic-owner",
    "treat-post-replay-p-puck-as-full-current-receipt-audit",
    "treat-post-replay-p-puck-as-successful-boot",
    "treat-post-replay-p-puck-as-host-removal",
    "treat-boot-execution-proof-as-runtime-owner",
    "treat-boot-execution-proof-as-new-engine-from-zero",
    "treat-boot-execution-proof-as-host-removal",
    "treat-boot-execution-proof-as-semantic-owner",
    "treat-runtime-owner-proof-as-new-engine-from-zero",
    "treat-runtime-owner-proof-as-global-runtime-install",
    "treat-runtime-owner-proof-as-host-removal",
    "treat-runtime-owner-proof-as-semantic-owner",
    "treat-semantic-owner-proof-as-new-engine-from-zero",
    "treat-semantic-owner-proof-as-global-runtime-install",
    "treat-semantic-owner-proof-as-host-removal",
    "treat-semantic-owner-proof-as-delete-ready",
    "treat-host-removal-fresh-puck-as-delete-ready",
    "treat-host-removal-fresh-puck-as-actual-delete",
    "treat-host-removal-fresh-puck-as-global-runtime-install",
    "treat-host-removal-fresh-puck-as-semantic-owner",
    "ignore-host-removal-fresh-puck-slow-path",
    "treat-host-removal-slow-path-repeat-as-delete-ready",
    "treat-host-removal-slow-path-repeat-as-actual-delete",
    "treat-host-removal-slow-path-repeat-as-global-runtime-install",
    "treat-host-removal-slow-path-repeat-as-semantic-owner",
    "treat-host-removal-delete-patch-candidate-as-delete-ready",
    "treat-host-removal-delete-patch-candidate-as-remove-now",
    "treat-host-removal-delete-patch-candidate-as-implementation-command",
    "treat-host-removal-delete-patch-candidate-as-global-runtime-install",
    "treat-host-removal-fresh-delete-puck-as-delete-ready",
    "treat-host-removal-fresh-delete-puck-as-remove-now",
    "treat-host-removal-fresh-delete-puck-as-implementation-command",
    "treat-host-removal-fresh-delete-puck-as-host-removal-started",
    "treat-host-removal-fresh-delete-puck-as-global-runtime-install",
    "treat-host-removal-fresh-delete-puck-as-runtime-api-flattening",
    "treat-host-removal-fresh-delete-puck-as-meaning-db",
    "treat-host-removal-fresh-delete-puck-as-semantic-owner",
    "treat-materialization-review-as-file-created",
    "treat-materialization-review-as-content-written",
    "treat-materialization-review-as-content-draft-generated",
    "treat-materialization-review-as-auto-approval",
    "treat-materialization-review-as-implementation-command",
    "treat-materialization-review-as-global-runtime",
    "treat-materialization-review-as-meaning-db",
    "treat-content-draft-as-file-created",
    "treat-content-draft-as-content-written",
    "treat-content-draft-as-auto-write",
    "treat-content-draft-as-auto-approval",
    "treat-content-draft-as-delete-ready",
    "treat-content-draft-as-implementation-command",
    "treat-content-draft-as-global-runtime",
    "treat-content-draft-as-meaning-db",
    "treat-file-writer-as-disk-write",
    "treat-file-writer-as-content-written",
    "treat-file-writer-as-auto-approval",
    "treat-file-writer-as-target-frontier-closed",
    "treat-file-writer-as-delete-ready",
    "treat-file-writer-as-implementation-command",
    "treat-file-writer-as-global-runtime",
    "treat-file-writer-as-meaning-db",
    "treat-materialization-proof-as-disk-write",
    "treat-materialization-proof-as-file-created",
    "treat-materialization-proof-as-content-written",
    "treat-materialization-proof-as-auto-approval",
    "treat-materialization-proof-as-target-frontier-closed",
    "treat-materialization-proof-as-delete-ready",
    "treat-materialization-proof-as-implementation-command",
    "treat-materialization-proof-as-global-runtime",
    "treat-materialization-proof-as-runtime-api-flattening",
    "treat-materialization-proof-as-meaning-db",
    "treat-materialization-proof-as-p-puck-semantic-owner",
    "let-old-host-code-authorize-fresh-delete-cut",
    "treat-scoped-adapter-as-global-ontology-runtime",
  ] {
    assert!(blocks.contains(expected), "missing gate block `{expected}`");
  }
}

#[test]
fn bootstrap_verdict_says_receipts_are_real_but_new_engine_from_zero_is_not_proven() {
  let run = eval_fixture();
  let verdict = get(run, "bootstrap-verdict");
  assert!(!as_bool(get(verdict, "new-engine-from-zero")));
  assert!(as_bool(get(verdict, "receipt-evaluated-macro-substrate")));
  assert!(as_bool(get(verdict, "macro-only-runtime-owner-booted")));
  assert!(as_bool(get(
    verdict,
    "macro-only-runtime-owner-proof-present"
  )));
  assert!(as_bool(get(
    verdict,
    "macro-only-semantic-owner-proof-present"
  )));
  assert!(as_bool(get(
    verdict,
    "macro-only-host-removal-execution-proof-present"
  )));
  assert!(as_bool(get(
    verdict,
    "macro-only-host-removal-fresh-puck-current-cut-present"
  )));
  assert!(as_bool(get(
    verdict,
    "macro-only-host-removal-slow-path-repeat-proof-present"
  )));
  assert!(!as_bool(get(verdict, "old-engine-authority")));
  assert!(as_bool(get(verdict, "old-engine-specimen-import")));
  assert!(as_bool(get(
    verdict,
    "old-host-ontology-code-still-present"
  )));
  assert!(as_bool(get(verdict, "host-code-removal-map-written")));
  assert!(as_bool(get(verdict, "macro-only-boot-manifest-written")));
  assert!(as_bool(get(verdict, "macro-only-boot-execution-attempted")));
  assert!(as_bool(get(
    verdict,
    "macro-only-boot-execution-proof-present"
  )));
  assert!(as_bool(get(verdict, "macro-only-boot-execution-succeeded")));
  assert!(as_bool(get(verdict, "boot-executed")));
  assert!(as_bool(get(verdict, "boot-execution-attempt-held")));
  assert!(as_bool(get(
    verdict,
    "macro-only-boot-runner-owner-present"
  )));
  assert!(as_bool(get(
    verdict,
    "bounded-full-graph-replay-strategy-present"
  )));
  assert!(as_bool(get(verdict, "regression-corpus-transfer-present")));
  assert!(as_bool(get(
    verdict,
    "bootstrap-status-audit-update-plan-present"
  )));
  assert!(as_bool(get(verdict, "compare-after-boot")));
  assert!(as_bool(get(verdict, "target-delete-preflight-present")));
  assert!(as_bool(get(
    verdict,
    "target-specific-delete-proof-present"
  )));
  assert!(as_bool(get(verdict, "fresh-p-puck-after-current-cut")));
  assert!(as_bool(get(verdict, "full-current-receipt-audit")));
  assert!(as_bool(get(verdict, "ready-for-bounded-replay")));
  assert!(as_bool(get(verdict, "bounded-replay-executed")));
  assert!(as_bool(get(
    verdict,
    "post-bounded-replay-p-puck-current-cut"
  )));
  assert!(as_bool(get(verdict, "host-removal-execution-proof")));
  assert!(as_bool(get(verdict, "host-removal-execution-gate-present")));
  assert!(!as_bool(get(verdict, "host-removal-execution-authorized")));
  assert!(as_bool(get(verdict, "fresh-puck-before-delete-required")));
  assert!(as_bool(get(
    verdict,
    "host-removal-fresh-p-puck-current-cut"
  )));
  assert!(as_bool(get(
    verdict,
    "fresh-puck-before-host-removal-execution"
  )));
  assert!(as_bool(get(verdict, "slow-path-candidate")));
  assert!(as_bool(get(verdict, "slow-path-repeat-within-threshold")));
  assert!(as_bool(get(verdict, "slow-path-repeat-frontier-closed")));
  assert!(as_bool(get(
    verdict,
    "macro-only-host-removal-delete-patch-candidate-present"
  )));
  assert!(as_bool(get(
    verdict,
    "macro-only-host-removal-fresh-delete-puck-current-cut-present"
  )));
  assert!(as_bool(get(verdict, "actual-host-removal-patch-candidate")));
  assert!(!as_bool(get(
    verdict,
    "actual-host-removal-patch-authorized"
  )));
  assert_eq!(
    as_i64(get(verdict, "delete-patch-candidate-target-count")),
    5
  );
  assert!(as_bool(get(
    verdict,
    "fresh-puck-before-delete-as-delete-ready-frontier-closed"
  )));
  assert!(!as_bool(get(verdict, "persistent-slow-path")));
  assert!(!as_bool(get(verdict, "profile-required-from-repeat")));
  assert!(!as_bool(get(verdict, "self-optimization-candidate")));
  assert!(as_bool(get(verdict, "fresh-puck-before-delete")));
  assert!(!as_bool(get(verdict, "host-code-removal-started")));
  assert!(!as_bool(get(verdict, "host-removal-safe")));
  assert_eq!(as_i64(get(verdict, "delete-ready-target-count")), 0);
  assert!(as_bool(get(verdict, "macro-stage-addition-only")));
  assert!(as_bool(get(verdict, "p-puck-is-wrapper-proof")));
  assert!(!as_bool(get(verdict, "p-puck-is-semantic-owner")));
  assert!(!as_bool(get(verdict, "global-ontology-runtime")));
  assert!(as_bool(get(verdict, "semantic-owner")));
  assert!(as_bool(get(verdict, "semantic-owner-proof")));
  assert_eq!(
    as_str(get(verdict, "semantic-owner-scope")),
    "bounded-generated-ontology-semantic-owner"
  );
  assert!(as_bool(get(
    verdict,
    "macro-only-receipt-materialization-review-present"
  )));
  assert!(as_bool(get(
    verdict,
    "macro-only-receipt-content-draft-generation-present"
  )));
  assert!(as_bool(get(
    verdict,
    "macro-only-receipt-file-writer-present"
  )));
  assert!(as_bool(get(
    verdict,
    "macro-only-receipt-file-materialization-proof-present"
  )));
  assert!(as_bool(get(verdict, "receipt-materialization-review")));
  assert!(as_bool(get(verdict, "materialization-reviewed")));
  assert!(as_bool(get(verdict, "materialization-review-only")));
  assert_eq!(as_i64(get(verdict, "reviewed-receipt-skeleton-count")), 5);
  assert!(as_bool(get(verdict, "receipt-content-draft-generation")));
  assert!(as_bool(get(verdict, "content-draft-generated")));
  assert!(as_bool(get(verdict, "draft-data-only")));
  assert_eq!(as_i64(get(verdict, "drafted-receipt-review-count")), 5);
  assert!(as_bool(get(verdict, "receipt-file-writer")));
  assert!(as_bool(get(verdict, "receipt-file-artifact-generated")));
  assert!(as_bool(get(verdict, "writer-candidate-only")));
  assert_eq!(as_i64(get(verdict, "written-receipt-artifact-count")), 5);
  assert!(as_bool(get(verdict, "receipt-file-materialization-proof")));
  assert!(as_bool(get(verdict, "materialization-proof-only")));
  assert_eq!(as_i64(get(verdict, "materialization-proof-count")), 5);
  assert_eq!(as_i64(get(verdict, "source-artifact-count")), 5);
  assert!(!as_bool(get(verdict, "disk-write-executed")));
  assert!(!as_bool(get(verdict, "receipt-file-created")));
  assert!(!as_bool(get(verdict, "receipt-content-written")));
  assert_eq!(as_i64(get(verdict, "external-solver-dependency-count")), 0);
  assert!(!as_bool(get(verdict, "gpl-family-dependencies")));
}

#[test]
fn verified_evidence_records_what_compare_and_p_puck_do_not_prove() {
  let run = eval_fixture();
  let compare = get_path(run, &["verified-evidence", "compare-harness"]);
  assert_eq!(as_str(get(compare, "status")), "ok");
  assert_eq!(as_i64(get(compare, "total-tests")), 1161);
  let compare_not = string_set(get(compare, "does-not-prove"));
  for expected in [
    "global-runtime-install",
    "host-legacy-code-removal",
    "all old functions replaced",
  ] {
    assert!(compare_not.contains(expected));
  }

  let puck = get_path(run, &["verified-evidence", "p-puck-receipt-audit"]);
  assert_eq!(as_str(get(puck, "status")), "ok");
  assert_eq!(as_i64(get(puck, "receipt-count")), 65);
  assert!(!as_bool(get(puck, "fresh-after-current-cut")));
  assert_eq!(
    as_str(get(puck, "latest-unaudited-receipt")),
    "fixtures/tesseract-macro-legacy-probe/macro_only_receipt_file_materialization_proof_receipt.px"
  );
  let unaudited = string_set(get(puck, "unaudited-receipts"));
  assert!(unaudited.contains(
    "fixtures/tesseract-macro-legacy-probe/compat_archive_lift_query_emit_surface_triple_receipt.px"
  ));
  assert!(
    unaudited.contains("fixtures/tesseract-macro-legacy-probe/host_code_removal_map_receipt.px")
  );
  assert!(unaudited.contains(
    "fixtures/tesseract-macro-legacy-probe/macro_only_ontology_boot_manifest_receipt.px"
  ));
  assert!(unaudited.contains(
    "fixtures/tesseract-macro-legacy-probe/macro_only_boot_execution_attempt_receipt.px"
  ));
  assert!(unaudited.contains("stdlib/lib/gate/macro-only-boot-runner.px"));
  assert!(unaudited.contains("fixtures/pnix-query-runtime/macro-only-boot-runner-owner.px"));
  assert!(unaudited
    .contains("fixtures/tesseract-macro-legacy-probe/macro_only_boot_runner_owner_receipt.px"));
  assert!(unaudited.contains("stdlib/lib/gate/macro-only-boot-full-current-receipt-audit.px"));
  assert!(unaudited
    .contains("fixtures/pnix-query-runtime/macro-only-boot-full-current-receipt-audit-owner.px"));
  assert!(unaudited.contains(
    "fixtures/tesseract-macro-legacy-probe/macro_only_full_current_receipt_audit_receipt.px"
  ));
  assert!(unaudited.contains("stdlib/lib/gate/macro-only-boot-execution-proof.px"));
  assert!(
    unaudited.contains("fixtures/pnix-query-runtime/macro-only-boot-execution-proof-owner.px")
  );
  assert!(unaudited
    .contains("fixtures/tesseract-macro-legacy-probe/macro_only_boot_execution_proof_receipt.px"));
  assert!(unaudited.contains("stdlib/lib/gate/macro-only-runtime-owner-proof.px"));
  assert!(unaudited.contains("fixtures/pnix-query-runtime/macro-only-runtime-owner-proof-owner.px"));
  assert!(unaudited
    .contains("fixtures/tesseract-macro-legacy-probe/macro_only_runtime_owner_proof_receipt.px"));
  assert!(unaudited.contains("stdlib/lib/gate/macro-only-semantic-owner-proof.px"));
  assert!(
    unaudited.contains("fixtures/pnix-query-runtime/macro-only-semantic-owner-proof-owner.px")
  );
  assert!(unaudited
    .contains("fixtures/tesseract-macro-legacy-probe/macro_only_semantic_owner_proof_receipt.px"));
  assert!(unaudited.contains("stdlib/lib/gate/macro-only-host-removal-execution-proof.px"));
  assert!(unaudited
    .contains("fixtures/pnix-query-runtime/macro-only-host-removal-execution-proof-owner.px"));
  assert!(unaudited.contains(
    "fixtures/tesseract-macro-legacy-probe/macro_only_host_removal_execution_proof_receipt.px"
  ));
  assert!(unaudited.contains("stdlib/lib/gate/macro-only-host-removal-fresh-p-puck-current-cut.px"));
  assert!(unaudited.contains(
    "fixtures/pnix-query-runtime/macro-only-host-removal-fresh-p-puck-current-cut-owner.px"
  ));
  assert!(unaudited.contains(
    "fixtures/tesseract-macro-legacy-probe/macro_only_host_removal_fresh_p_puck_current_cut_receipt.px"
  ));
  assert!(unaudited.contains("stdlib/lib/gate/macro-only-host-removal-slow-path-repeat-proof.px"));
  assert!(unaudited.contains(
    "fixtures/pnix-query-runtime/macro-only-host-removal-slow-path-repeat-proof-owner.px"
  ));
  assert!(unaudited.contains(
    "fixtures/tesseract-macro-legacy-probe/macro_only_host_removal_slow_path_repeat_proof_receipt.px"
  ));
  assert!(unaudited.contains("stdlib/lib/gate/macro-only-host-removal-delete-patch-candidate.px"));
  assert!(unaudited.contains(
    "fixtures/pnix-query-runtime/macro-only-host-removal-delete-patch-candidate-owner.px"
  ));
  assert!(unaudited.contains(
    "fixtures/tesseract-macro-legacy-probe/macro_only_host_removal_delete_patch_candidate_receipt.px"
  ));
  assert!(unaudited
    .contains("stdlib/lib/gate/macro-only-host-removal-fresh-delete-p-puck-current-cut.px"));
  assert!(unaudited.contains(
    "fixtures/pnix-query-runtime/macro-only-host-removal-fresh-delete-p-puck-current-cut-owner.px"
  ));
  assert!(unaudited.contains(
    "fixtures/tesseract-macro-legacy-probe/macro_only_host_removal_fresh_delete_p_puck_current_cut_receipt.px"
  ));
  assert!(unaudited.contains("stdlib/lib/gate/macro-only-self-receipt-frontier-emission.px"));
  assert!(unaudited
    .contains("fixtures/pnix-query-runtime/macro-only-self-receipt-frontier-emission-owner.px"));
  assert!(unaudited.contains(
    "fixtures/tesseract-macro-legacy-probe/macro_only_self_receipt_frontier_emission_receipt.px"
  ));
  assert!(unaudited.contains("stdlib/lib/gate/macro-only-receipt-skeleton-generator.px"));
  assert!(unaudited
    .contains("fixtures/pnix-query-runtime/macro-only-receipt-skeleton-generator-owner.px"));
  assert!(unaudited.contains(
    "fixtures/tesseract-macro-legacy-probe/macro_only_receipt_skeleton_generator_receipt.px"
  ));
  assert!(unaudited.contains("stdlib/lib/gate/macro-only-receipt-materialization-review.px"));
  assert!(unaudited
    .contains("fixtures/pnix-query-runtime/macro-only-receipt-materialization-review-owner.px"));
  assert!(unaudited.contains(
    "fixtures/tesseract-macro-legacy-probe/macro_only_receipt_materialization_review_receipt.px"
  ));
  assert!(unaudited.contains("stdlib/lib/gate/macro-only-receipt-content-draft-generator.px"));
  assert!(unaudited
    .contains("fixtures/pnix-query-runtime/macro-only-receipt-content-draft-generator-owner.px"));
  assert!(unaudited.contains(
    "fixtures/tesseract-macro-legacy-probe/macro_only_receipt_content_draft_generator_receipt.px"
  ));
  assert!(unaudited.contains("stdlib/lib/gate/macro-only-receipt-file-writer.px"));
  assert!(unaudited.contains("fixtures/pnix-query-runtime/macro-only-receipt-file-writer-owner.px"));
  assert!(unaudited
    .contains("fixtures/tesseract-macro-legacy-probe/macro_only_receipt_file_writer_receipt.px"));
  assert!(unaudited.contains("stdlib/lib/gate/macro-only-receipt-file-materialization-proof.px"));
  assert!(unaudited.contains(
    "fixtures/pnix-query-runtime/macro-only-receipt-file-materialization-proof-owner.px"
  ));
  assert!(unaudited.contains(
    "fixtures/tesseract-macro-legacy-probe/macro_only_receipt_file_materialization_proof_receipt.px"
  ));
  assert!(unaudited.contains("stdlib/lib/gate/macro-only-boot-replay-strategy.px"));
  assert!(
    unaudited.contains("fixtures/pnix-query-runtime/macro-only-boot-replay-strategy-owner.px")
  );
  assert!(unaudited.contains(
    "fixtures/tesseract-macro-legacy-probe/macro_only_bounded_replay_strategy_receipt.px"
  ));
  assert!(unaudited.contains("stdlib/lib/gate/macro-only-boot-regression-corpus.px"));
  assert!(
    unaudited.contains("fixtures/pnix-query-runtime/macro-only-boot-regression-corpus-owner.px")
  );
  assert!(unaudited.contains(
    "fixtures/tesseract-macro-legacy-probe/macro_only_regression_corpus_retention_receipt.px"
  ));
  assert!(unaudited.contains("stdlib/lib/gate/macro-only-boot-bootstrap-audit-update.px"));
  assert!(unaudited
    .contains("fixtures/pnix-query-runtime/macro-only-boot-bootstrap-audit-update-owner.px"));
  assert!(unaudited.contains(
    "fixtures/tesseract-macro-legacy-probe/macro_only_bootstrap_audit_update_receipt.px"
  ));
  assert!(unaudited.contains("stdlib/lib/gate/macro-only-boot-compare-after-boot.px"));
  assert!(
    unaudited.contains("fixtures/pnix-query-runtime/macro-only-boot-compare-after-boot-owner.px")
  );
  assert!(unaudited.contains(
    "fixtures/tesseract-macro-legacy-probe/macro_only_boot_compare_after_boot_receipt.px"
  ));
  assert!(unaudited.contains("stdlib/lib/gate/macro-only-boot-target-delete-preflight.px"));
  assert!(unaudited
    .contains("fixtures/pnix-query-runtime/macro-only-boot-target-delete-preflight-owner.px"));
  assert!(unaudited.contains(
    "fixtures/tesseract-macro-legacy-probe/macro_only_target_delete_preflight_receipt.px"
  ));
  assert!(unaudited.contains("stdlib/lib/gate/macro-only-boot-target-delete-proof.px"));
  assert!(
    unaudited.contains("fixtures/pnix-query-runtime/macro-only-boot-target-delete-proof-owner.px")
  );
  assert!(unaudited
    .contains("fixtures/tesseract-macro-legacy-probe/macro_only_target_delete_proof_receipt.px"));
  assert!(unaudited.contains("stdlib/lib/gate/macro-only-boot-fresh-p-puck-current-cut.px"));
  assert!(unaudited
    .contains("fixtures/pnix-query-runtime/macro-only-boot-fresh-p-puck-current-cut-owner.px"));
  assert!(unaudited.contains(
    "fixtures/tesseract-macro-legacy-probe/macro_only_fresh_p_puck_current_cut_receipt.px"
  ));
  assert!(unaudited.contains("stdlib/lib/gate/macro-only-boot-bounded-replay-execution.px"));
  assert!(unaudited
    .contains("fixtures/pnix-query-runtime/macro-only-boot-bounded-replay-execution-owner.px"));
  assert!(unaudited.contains(
    "fixtures/tesseract-macro-legacy-probe/macro_only_bounded_replay_execution_receipt.px"
  ));
  assert!(
    unaudited.contains("stdlib/lib/gate/macro-only-boot-post-bounded-replay-p-puck-current-cut.px")
  );
  assert!(unaudited.contains(
    "fixtures/pnix-query-runtime/macro-only-boot-post-bounded-replay-p-puck-current-cut-owner.px"
  ));
  assert!(unaudited.contains(
    "fixtures/tesseract-macro-legacy-probe/macro_only_post_bounded_replay_p_puck_current_cut_receipt.px"
  ));
  let puck_not = string_set(get(puck, "does-not-prove"));
  assert!(puck_not.contains("p-puck is semantic owner"));
  assert!(puck_not.contains("fresh macro ontology runtime is installed"));

  let materialization = get_path(
    run,
    &[
      "verified-evidence",
      "macro-only-receipt-file-materialization-proof",
    ],
  );
  assert_eq!(
    as_str(get(materialization, "status")),
    "receipt-file-materialization-proof-present"
  );
  assert!(as_bool(get(
    materialization,
    "receipt-file-materialization-proof"
  )));
  assert!(as_bool(get(materialization, "materialization-proof-only")));
  assert_eq!(as_i64(get(materialization, "source-artifact-count")), 5);
  assert_eq!(
    as_i64(get(materialization, "materialization-proof-count")),
    5
  );
  assert!(!as_bool(get(materialization, "disk-write-executed")));
  assert!(!as_bool(get(materialization, "receipt-file-created")));
  assert!(!as_bool(get(materialization, "receipt-content-written")));
  assert!(!as_bool(get(materialization, "target-frontier-closed")));
  assert!(!as_bool(get(materialization, "implementation-command")));
  assert!(!as_bool(get(materialization, "global-ontology-runtime")));
  assert!(!as_bool(get(materialization, "runtime-api-flattening")));
  assert!(!as_bool(get(materialization, "meaning-db")));
  let materialization_not = string_set(get(materialization, "does-not-prove"));
  assert!(materialization_not.contains("disk-write"));
  assert!(materialization_not.contains("receipt-file-created"));
  assert!(materialization_not.contains("target-frontier-closed"));
  assert!(materialization_not.contains("implementation-command"));

  let current_cut = get_path(run, &["verified-evidence", "p-puck-current-cut-proof"]);
  assert_eq!(
    as_str(get(current_cut, "status")),
    "fresh-p-puck-current-cut-present"
  );
  assert_eq!(as_str(get(current_cut, "report-kind")), "pnix-preset");
  assert_eq!(as_str(get(current_cut, "preset")), "pnixc");
  assert_eq!(as_str(get(current_cut, "runner")), "cargo-bin");
  assert_eq!(as_str(get(current_cut, "telemetry-source")), "p-puck");
  assert_eq!(as_i64(get(current_cut, "duration-ms")), 701);
  assert!(as_bool(get(current_cut, "fresh-p-puck-after-current-cut")));
  assert!(!as_bool(get(current_cut, "full-current-receipt-audit")));
  assert!(!as_bool(get(current_cut, "p-puck-is-semantic-owner")));
  let current_not = string_set(get(current_cut, "does-not-prove"));
  assert!(current_not.contains("full-current-receipt-audit"));
  assert!(current_not.contains("bounded-replay-executed"));
  assert!(current_not.contains("macro-only-runtime-owner-boot"));
  assert!(current_not.contains("host-code-removal"));

  let bounded = get_path(
    run,
    &["verified-evidence", "bounded-replay-execution-proof"],
  );
  assert_eq!(as_str(get(bounded, "status")), "bounded-replay-executed");
  assert!(as_bool(get(bounded, "runner-ready-input")));
  assert_eq!(as_i64(get(bounded, "runner-missing-count")), 0);
  assert_eq!(as_i64(get(bounded, "replay-step-count")), 11);
  assert_eq!(as_i64(get(bounded, "node-count")), 11);
  assert_eq!(
    as_str(get(bounded, "semantic-delta-status")),
    "empty-or-held-only"
  );
  assert!(as_bool(get(bounded, "bounded-replay-executed")));
  assert!(!as_bool(get(bounded, "boot-executed")));
  assert!(!as_bool(get(bounded, "macro-only-runtime-owner-booted")));
  assert!(!as_bool(get(bounded, "host-code-removal-started")));
  assert_eq!(as_i64(get(bounded, "delete-ready-target-count")), 0);
  let bounded_not = string_set(get(bounded, "does-not-prove"));
  assert!(bounded_not.contains("macro-only-runtime-owner-boot"));
  assert!(bounded_not.contains("new-engine-from-zero"));
  assert!(bounded_not.contains("host-code-removal"));
  assert!(bounded_not.contains("full-current-receipt-audit"));

  let post_puck = get_path(
    run,
    &[
      "verified-evidence",
      "post-bounded-replay-p-puck-current-cut-proof",
    ],
  );
  assert_eq!(
    as_str(get(post_puck, "status")),
    "post-bounded-replay-p-puck-current-cut-present"
  );
  assert_eq!(
    as_str(get(post_puck, "report-name")),
    "macro-only-current-cut-bounded-replay"
  );
  assert_eq!(as_i64(get(post_puck, "duration-ms")), 4934);
  assert_eq!(
    as_str(get(post_puck, "slow-path-status")),
    "within-threshold"
  );
  assert!(as_bool(get(
    post_puck,
    "post-bounded-replay-p-puck-current-cut"
  )));
  assert!(!as_bool(get(post_puck, "full-current-receipt-audit")));
  assert!(!as_bool(get(post_puck, "p-puck-is-semantic-owner")));
  assert!(!as_bool(get(post_puck, "boot-executed")));
  let post_not = string_set(get(post_puck, "does-not-prove"));
  assert!(post_not.contains("full-current-receipt-audit"));
  assert!(post_not.contains("macro-only-runtime-owner-boot"));
  assert!(post_not.contains("host-code-removal"));
  assert!(post_not.contains("semantic-owner"));

  let boot = get_path(
    run,
    &["verified-evidence", "macro-only-boot-execution-proof"],
  );
  assert_eq!(
    as_str(get(boot, "status")),
    "macro-only-boot-execution-proof-present"
  );
  assert_eq!(as_i64(get(boot, "total-tests")), 931);
  assert_eq!(as_i64(get(boot, "source-tracked")), 18172);
  assert_eq!(as_i64(get(boot, "source-indexed")), 18172);
  assert!(as_bool(get(boot, "full-current-receipt-audit-input")));
  assert!(as_bool(get(boot, "bounded-replay-input")));
  assert!(as_bool(get(boot, "post-replay-p-puck-input")));
  assert!(as_bool(get(boot, "boot-execution-proof")));
  assert!(as_bool(get(boot, "boot-executed")));
  assert!(!as_bool(get(boot, "macro-only-runtime-owner-booted")));
  assert!(!as_bool(get(boot, "semantic-owner")));
  assert!(!as_bool(get(boot, "host-code-removal-started")));
  assert_eq!(as_i64(get(boot, "delete-ready-target-count")), 0);

  let host_exec = get_path(
    run,
    &[
      "verified-evidence",
      "macro-only-host-removal-execution-proof",
    ],
  );
  assert_eq!(
    as_str(get(host_exec, "status")),
    "macro-only-host-removal-execution-proof-present"
  );
  assert_eq!(as_i64(get(host_exec, "total-tests")), 981);
  assert_eq!(as_i64(get(host_exec, "source-tracked")), 18187);
  assert_eq!(as_i64(get(host_exec, "source-indexed")), 18187);
  assert!(as_bool(get(host_exec, "host-removal-execution-proof")));
  assert!(as_bool(get(
    host_exec,
    "host-removal-execution-gate-present"
  )));
  assert!(!as_bool(get(
    host_exec,
    "host-removal-execution-authorized"
  )));
  assert!(as_bool(get(host_exec, "fresh-puck-before-delete-required")));
  assert!(!as_bool(get(host_exec, "fresh-puck-before-delete")));
  assert!(!as_bool(get(host_exec, "host-code-removal-started")));
  assert_eq!(as_i64(get(host_exec, "delete-ready-target-count")), 0);

  let host_fresh = get_path(
    run,
    &[
      "verified-evidence",
      "macro-only-host-removal-fresh-p-puck-current-cut-proof",
    ],
  );
  assert_eq!(
    as_str(get(host_fresh, "status")),
    "host-removal-fresh-p-puck-current-cut-present"
  );
  assert_eq!(
    as_str(get(host_fresh, "report-name")),
    "macro-only-current-cut-host-removal-execution-proof"
  );
  assert_eq!(
    as_str(get(host_fresh, "audited-receipt")),
    "fixtures/tesseract-macro-legacy-probe/macro_only_host_removal_execution_proof_receipt.px"
  );
  assert_eq!(as_i64(get(host_fresh, "duration-ms")), 5389);
  assert_eq!(as_i64(get(host_fresh, "slow-threshold-ms")), 5000);
  assert_eq!(
    as_str(get(host_fresh, "slow-path-status")),
    "slow-path-candidate"
  );
  assert!(as_bool(get(
    host_fresh,
    "host-removal-fresh-p-puck-current-cut"
  )));
  assert!(as_bool(get(
    host_fresh,
    "fresh-puck-before-host-removal-execution"
  )));
  assert!(as_bool(get(host_fresh, "p-puck-wrapper-proof")));
  assert!(as_bool(get(host_fresh, "slow-path-candidate")));
  assert!(as_bool(get(host_fresh, "self-optimization-candidate")));
  assert!(!as_bool(get(
    host_fresh,
    "actual-host-removal-patch-authorized"
  )));
  assert!(!as_bool(get(host_fresh, "host-code-removal-started")));
  assert!(!as_bool(get(host_fresh, "fresh-puck-before-delete")));
  assert_eq!(as_i64(get(host_fresh, "delete-ready-target-count")), 0);
  let host_fresh_not = string_set(get(host_fresh, "does-not-prove"));
  assert!(host_fresh_not.contains("actual-host-removal-patch"));
  assert!(host_fresh_not.contains("delete-ready-targets"));
  assert!(host_fresh_not.contains("global-runtime-install"));

  let host_repeat = get_path(
    run,
    &[
      "verified-evidence",
      "macro-only-host-removal-slow-path-repeat-proof",
    ],
  );
  assert_eq!(
    as_str(get(host_repeat, "status")),
    "host-removal-slow-path-repeat-within-threshold"
  );
  assert_eq!(
    as_str(get(host_repeat, "report-name")),
    "macro-only-current-cut-host-removal-execution-proof-repeat"
  );
  assert_eq!(
    as_str(get(host_repeat, "prior-report-name")),
    "macro-only-current-cut-host-removal-execution-proof"
  );
  assert_eq!(as_i64(get(host_repeat, "prior-gate-duration-ms")), 5389);
  assert_eq!(as_i64(get(host_repeat, "repeat-duration-ms")), 551);
  assert_eq!(as_i64(get(host_repeat, "puck-previous-duration-ms")), 5094);
  assert_eq!(as_i64(get(host_repeat, "duration-delta-ms")), -4543);
  assert_eq!(
    as_str(get(host_repeat, "repeat-slow-path-status")),
    "within-threshold"
  );
  assert_eq!(
    as_str(get(host_repeat, "duration-delta-status")),
    "faster-than-previous"
  );
  assert!(as_bool(get(
    host_repeat,
    "host-removal-slow-path-repeat-proof"
  )));
  assert!(as_bool(get(
    host_repeat,
    "slow-path-repeat-within-threshold"
  )));
  assert!(as_bool(get(
    host_repeat,
    "slow-path-repeat-frontier-closed"
  )));
  assert!(!as_bool(get(host_repeat, "persistent-slow-path")));
  assert!(!as_bool(get(host_repeat, "profile-required-from-repeat")));
  assert!(!as_bool(get(
    host_repeat,
    "actual-host-removal-patch-authorized"
  )));
  assert!(!as_bool(get(host_repeat, "host-code-removal-started")));
  assert_eq!(as_i64(get(host_repeat, "delete-ready-target-count")), 0);
  let host_repeat_not = string_set(get(host_repeat, "does-not-prove"));
  assert!(host_repeat_not.contains("actual-host-removal-patch"));
  assert!(host_repeat_not.contains("delete-ready-targets"));
  assert!(host_repeat_not.contains("global-runtime-install"));

  let fresh_delete = get_path(
    run,
    &[
      "verified-evidence",
      "macro-only-host-removal-fresh-delete-puck-current-cut",
    ],
  );
  assert_eq!(
    as_str(get(fresh_delete, "status")),
    "host-removal-fresh-delete-p-puck-current-cut-present"
  );
  assert_eq!(
    as_str(get(fresh_delete, "report-name")),
    "macro-only-current-cut-host-removal-delete-patch-candidate"
  );
  assert_eq!(
    as_str(get(fresh_delete, "audited-receipt")),
    "fixtures/tesseract-macro-legacy-probe/macro_only_host_removal_delete_patch_candidate_receipt.px"
  );
  assert_eq!(as_i64(get(fresh_delete, "duration-ms")), 1318);
  assert_eq!(as_i64(get(fresh_delete, "slow-threshold-ms")), 5000);
  assert_eq!(
    as_str(get(fresh_delete, "slow-path-status")),
    "within-threshold"
  );
  assert_eq!(
    as_str(get(fresh_delete, "duration-delta-status")),
    "no-previous-report"
  );
  assert_eq!(as_i64(get(fresh_delete, "total-tests")), 1053);
  assert_eq!(as_i64(get(fresh_delete, "source-tracked")), 18207);
  assert_eq!(as_i64(get(fresh_delete, "source-indexed")), 18207);
  assert!(as_bool(get(
    fresh_delete,
    "host-removal-fresh-delete-puck-current-cut"
  )));
  assert!(as_bool(get(fresh_delete, "fresh-puck-before-delete")));
  assert!(as_bool(get(
    fresh_delete,
    "fresh-puck-before-delete-as-delete-ready-frontier-closed"
  )));
  assert!(as_bool(get(
    fresh_delete,
    "actual-host-removal-patch-candidate"
  )));
  assert!(!as_bool(get(
    fresh_delete,
    "actual-host-removal-patch-authorized"
  )));
  assert!(!as_bool(get(fresh_delete, "delete-ready")));
  assert_eq!(as_i64(get(fresh_delete, "delete-ready-target-count")), 0);
  assert!(!as_bool(get(fresh_delete, "remove-now")));
  assert!(!as_bool(get(fresh_delete, "host-code-removal-started")));
  assert!(!as_bool(get(fresh_delete, "implementation-command")));
  assert!(!as_bool(get(fresh_delete, "runtime-api-flattening")));
  assert!(!as_bool(get(fresh_delete, "meaning-db")));
  assert!(as_bool(get(fresh_delete, "p-puck-wrapper-proof")));
  assert!(!as_bool(get(fresh_delete, "p-puck-is-semantic-owner")));
  let fresh_delete_not = string_set(get(fresh_delete, "does-not-prove"));
  assert!(fresh_delete_not.contains("delete-ready-targets"));
  assert!(fresh_delete_not.contains("implementation-command"));
  assert!(fresh_delete_not.contains("runtime-api-flattening"));
  assert!(fresh_delete_not.contains("meaning-db"));

  let runtime_owner = get_path(
    run,
    &["verified-evidence", "macro-only-runtime-owner-proof"],
  );
  assert_eq!(
    as_str(get(runtime_owner, "status")),
    "macro-only-runtime-owner-proof-present"
  );
  assert_eq!(
    as_str(get(runtime_owner, "runtime-owner-scope")),
    "bounded-receipt-trajectory-owner"
  );
  assert_eq!(as_i64(get(runtime_owner, "total-tests")), 947);
  assert_eq!(as_i64(get(runtime_owner, "source-tracked")), 18177);
  assert_eq!(as_i64(get(runtime_owner, "source-indexed")), 18177);
  assert!(as_bool(get(runtime_owner, "boot-executed")));
  assert!(as_bool(get(runtime_owner, "runtime-owner-proof")));
  assert!(as_bool(get(
    runtime_owner,
    "macro-only-runtime-owner-booted"
  )));
  assert!(!as_bool(get(runtime_owner, "new-engine-from-zero")));
  assert!(!as_bool(get(runtime_owner, "runtime-install")));
  assert!(!as_bool(get(runtime_owner, "global-ontology-runtime")));
  assert!(!as_bool(get(runtime_owner, "semantic-owner")));
  assert!(!as_bool(get(runtime_owner, "host-code-removal-started")));
  assert_eq!(as_i64(get(runtime_owner, "delete-ready-target-count")), 0);
  let runtime_not = string_set(get(runtime_owner, "does-not-prove"));
  assert!(runtime_not.contains("new-engine-from-zero"));
  assert!(runtime_not.contains("global-runtime-install"));
  assert!(runtime_not.contains("host-code-removal"));
  assert!(runtime_not.contains("semantic-owner"));

  let semantic_owner = get_path(
    run,
    &["verified-evidence", "macro-only-semantic-owner-proof"],
  );
  assert_eq!(
    as_str(get(semantic_owner, "status")),
    "macro-only-semantic-owner-proof-present"
  );
  assert_eq!(
    as_str(get(semantic_owner, "semantic-owner-scope")),
    "bounded-generated-ontology-semantic-owner"
  );
  assert_eq!(as_i64(get(semantic_owner, "total-tests")), 963);
  assert_eq!(as_i64(get(semantic_owner, "source-tracked")), 18182);
  assert_eq!(as_i64(get(semantic_owner, "source-indexed")), 18182);
  assert!(as_bool(get(semantic_owner, "runtime-owner-proof-input")));
  assert!(as_bool(get(semantic_owner, "runtime-owner-proof")));
  assert!(as_bool(get(
    semantic_owner,
    "macro-only-runtime-owner-booted"
  )));
  assert!(as_bool(get(semantic_owner, "boot-executed")));
  assert!(as_bool(get(semantic_owner, "semantic-owner-proof")));
  assert!(as_bool(get(semantic_owner, "semantic-owner")));
  assert!(!as_bool(get(semantic_owner, "new-engine-from-zero")));
  assert!(!as_bool(get(semantic_owner, "runtime-install")));
  assert!(!as_bool(get(semantic_owner, "global-ontology-runtime")));
  assert!(!as_bool(get(semantic_owner, "host-code-removal-started")));
  assert_eq!(as_i64(get(semantic_owner, "delete-ready-target-count")), 0);
  let semantic_not = string_set(get(semantic_owner, "does-not-prove"));
  assert!(semantic_not.contains("new-engine-from-zero"));
  assert!(semantic_not.contains("global-runtime-install"));
  assert!(semantic_not.contains("host-code-removal"));
  assert!(semantic_not.contains("delete-ready-targets"));
}

#[test]
fn surface_state_distinguishes_promote_eval_select_and_lift_query_emit_phases() {
  let run = eval_fixture();
  let rows = attrs_by_key(get(run, "surface-state"), "surface");

  let promote = rows
    .get("builtins.ontologyPromote / ontology.promote")
    .expect("promote row");
  assert_eq!(as_str(get(promote, "phase")), "R7");
  assert_eq!(
    as_str(get(promote, "macro-owner")),
    "macro-native.promote.surface-owner"
  );
  assert!(!as_bool(get(promote, "legacy-authority")));
  assert!(!as_bool(get(promote, "removal-safe")));

  let eval_select = rows
    .get("builtins.ontologyEvaluate / builtins.ontologySelect")
    .expect("eval/select row");
  assert_eq!(
    as_str(get(eval_select, "phase")),
    "scoped-runtime-adapter-install"
  );
  assert!(as_bool(get(eval_select, "scoped-adapter-installed")));
  assert!(!as_bool(get(eval_select, "global-runtime-install")));
  assert!(!as_bool(get(eval_select, "removal-safe")));

  let lqe = rows
    .get("builtins.ontologyLift / builtins.ontologyQuery / builtins.ontologyEmit")
    .expect("lift/query/emit row");
  assert_eq!(as_str(get(lqe, "phase")), "R7");
  assert_eq!(
    as_str(get(lqe, "status")),
    "compat-retained-for-lift-query-emit-surface-triple"
  );
  assert_eq!(
    as_str(get(lqe, "macro-owner")),
    "macro-native.lift-query-emit.surface-triple-owner"
  );
  assert!(as_bool(get(lqe, "owner-switch")));
  assert!(as_bool(get(lqe, "compat-retained")));
  assert!(!as_bool(get(lqe, "query-runtime-install")));
  assert!(!as_bool(get(lqe, "fact-store-install")));
  assert!(!as_bool(get(lqe, "audit-event-log-install")));
  assert!(!as_bool(get(lqe, "expression-projection-owner")));
  assert!(!as_bool(get(lqe, "removal-safe")));
}

#[test]
fn host_removal_targets_are_explicitly_not_removable_yet() {
  let run = eval_fixture();
  let targets = attrs_by_key(get(run, "host-removal-targets"), "path");
  for expected in [
    "crates/pnix-runtime-legacy/src/ssa_eval/builtins/mod.rs",
    "crates/pnix-runtime-legacy/src/ir/eval.rs",
    "stdlib/lib/ontology.px",
    "crates/pnix-core/src/ontology.rs",
    "crates/pnix-eval/tests/ontology_builtins.rs",
  ] {
    let target = targets
      .get(expected)
      .unwrap_or_else(|| panic!("missing host removal target `{expected}`"));
    assert!(!as_bool(get(target, "delete-ready")));
    assert!(!as_bool(get(target, "remove-now")));
    assert!(!as_list(get(target, "observed-symbols")).is_empty());
  }
}

#[test]
fn start_again_protocol_puts_macro_only_boot_before_more_claims() {
  let run = eval_fixture();
  let steps = attrs_by_key(get(run, "start-again-protocol"), "id");
  assert_eq!(steps.len(), 4);
  for id in ["boot.step.1", "boot.step.2", "boot.step.3", "boot.step.4"] {
    assert!(as_bool(get(steps.get(id).expect("step"), "required")));
  }
  assert!(as_str(get(steps.get("boot.step.2").unwrap(), "action"))
    .contains("macro-only ontology boot manifest"));
  assert!(
    as_str(get(steps.get("boot.step.3").unwrap(), "action")).contains("host-code removal map")
  );
}

#[test]
fn migration_pipeline_starts_discovery_absorption_to_macro_only_boot_path() {
  let run = eval_fixture();
  let pipeline = get(run, "migration-pipeline");
  assert_eq!(
    as_str(get(pipeline, "id")),
    "pipeline.discovery-absorption-to-macro-ontology-migration.v1"
  );
  assert_eq!(
    as_str(get(pipeline, "current-stage")),
    "receipt-file-materialization-proof-present"
  );
  assert!(as_bool(get(pipeline, "migration-started")));
  assert!(!as_bool(get(pipeline, "migration-complete")));
  assert!(as_str(get(pipeline, "host-code-growth-policy"))
    .contains("semantic capability growth belongs in .px macro-stage owners"));

  let direction = string_set(get(pipeline, "direction"));
  for expected in [
    "discover-old-surface-behavior-as-specimen",
    "write-macro-native-rewrite-candidate",
    "reverse-replay-against-old-specimen",
    "switch-surface-owner",
    "audit-bootstrap-status",
    "write-host-removal-map",
    "write-macro-only-boot-manifest",
    "record-macro-only-boot-execution-attempt",
    "write-macro-only-boot-runner-owner",
    "write-bounded-full-graph-replay-strategy",
    "write-regression-corpus-retention-proof",
    "write-bootstrap-status-audit-update-proof",
    "write-compare-after-boot-proof",
    "write-target-delete-preflight-proof",
    "write-target-specific-delete-proof",
    "prove-fresh-p-puck-current-cut",
    "execute-bounded-replay",
    "prove-post-bounded-replay-p-puck-current-cut",
    "prove-full-current-receipt-audit",
    "prove-macro-only-boot",
    "prove-macro-only-runtime-owner-after-boot",
    "prove-macro-only-semantic-owner-after-runtime-owner",
    "prove-host-removal-execution-gate-after-semantic-owner",
    "prove-host-removal-fresh-p-puck-current-cut",
    "prove-host-removal-slow-path-repeat-before-delete",
    "write-host-removal-delete-patch-candidate",
    "prove-host-removal-fresh-delete-puck-current-cut",
    "emit-next-receipt-frontier-candidates",
    "generate-data-only-receipt-skeletons",
    "review-data-only-receipt-skeletons",
    "generate-structured-receipt-content-drafts",
    "generate-receipt-file-artifact-candidates",
    "prove-receipt-file-materialization-records",
    "remove-or-archive-old-host-code",
  ] {
    assert!(
      direction.contains(expected),
      "missing migration step `{expected}`"
    );
  }

  let next = string_set(get(pipeline, "next-required"));
  assert!(next.contains("receipt-file-disk-write-after-materialization-proof"));
  assert!(!next.contains("receipt-file-materialization-proof-after-writer-candidate"));
  assert!(!next.contains("receipt-file-writer-after-content-draft-generation"));
  assert!(!next.contains("receipt-content-draft-generation-after-materialization-review"));
  assert!(!next.contains("receipt-skeleton-materialization-review-after-data-skeleton"));
  assert!(!next.contains("receipt-skeleton-generator-after-frontier-emission"));
  assert!(!next.contains("bounded-replay-execution-proof-after-runner-ready"));
  assert!(!next.contains("macro-only-boot-execution-proof-after-full-current-receipt-audit"));
  assert!(!next.contains("macro-only-runtime-owner-proof-after-boot-execution"));
  assert!(!next.contains("semantic-owner-proof-after-runtime-owner"));
  assert!(!next.contains("host-code-removal-execution-proof-after-semantic-owner"));
  assert!(!next.contains("fresh-puck-before-host-removal-execution"));
  assert!(!next.contains("host-removal-slow-path-repeat-or-profile-before-delete"));
  assert!(!next.contains("actual-host-removal-patch-after-fresh-puck"));
  assert!(!next.contains("fresh-puck-before-delete-as-delete-ready"));
  assert!(!next.contains("delete-ready-targets-after-delete-candidate"));
  assert!(next.contains("delete-ready-targets-after-fresh-delete-puck"));
  assert!(next.contains("actual-host-removal-implementation-command"));
  assert!(next.contains("global-runtime-install-proof-after-semantic-owner"));
  assert!(next.contains("domain-runtime-api-flattening-after-semantic-owner"));
  assert!(!next.contains("full-current-receipt-audit-after-bounded-replay"));
  assert!(!next.contains("bootstrap-status-audit-update-after-corpus-proof"));
  assert!(!next.contains("fresh-p-puck-and-compare-proof-after-each-cut"));
  assert!(!next.contains("target-specific-host-delete-proof"));
  assert!(next.contains("lift-query-emit-runtime-owner-or-host-removal-proof"));
  assert!(!next.contains("fresh-p-puck-proof-after-current-cut"));
}

#[test]
fn overclaim_corrections_hold_false_discovery_language() {
  let run = eval_fixture();
  let corrections = attrs_by_key(get(run, "overclaim-corrections"), "claim");
  for claim in [
    "new ontology engine starts from zero",
    "all discovered functions are installed capabilities",
    "host-code removal map means old host code can now be deleted",
    "macro-only boot manifest means macro-only runtime booted",
    "macro-only boot execution attempt means macro-only runtime booted",
    "macro-only boot runner owner means macro-only runtime booted",
    "bounded replay strategy means macro-only runtime booted",
    "regression corpus retention means macro-only runtime booted",
    "bootstrap audit update means macro-only runtime booted",
    "compare-after-boot proof means macro-only runtime booted",
    "target-delete preflight means old host code can be deleted",
    "fresh p-puck current-cut proof means macro-only runtime booted",
    "bounded replay execution means macro-only runtime booted",
    "post bounded replay p-puck current-cut proof means macro-only runtime booted",
    "post bounded replay p-puck current-cut proof means full current receipt audit",
    "macro-only boot execution proof means macro-only runtime owner booted",
    "macro-only boot execution proof means new engine from zero",
    "macro-only boot execution proof means old host code can be removed",
    "macro-only boot execution proof means semantic owner",
    "macro-only runtime owner proof means new engine from zero",
    "macro-only runtime owner proof means global runtime install",
    "macro-only runtime owner proof means old host code can be removed",
    "macro-only runtime owner proof means semantic owner",
    "macro-only semantic owner proof means new engine from zero",
    "macro-only semantic owner proof means global runtime install",
    "macro-only semantic owner proof means old host code can be removed",
    "macro-only host-removal execution proof means old host code can be deleted",
    "macro-only host-removal fresh p-puck current-cut means old host code can be deleted",
    "host-removal slow-path candidate can be ignored before deletion",
    "host-removal slow-path repeat proof means old host code can be deleted",
    "host-removal delete patch candidate means old host code can be deleted",
    "host-removal fresh delete p-puck current-cut means old host code can be deleted",
    "self-receipt frontier emission means PNIX can write and approve receipts automatically",
    "receipt skeleton generation means receipt files were written and approved",
    "receipt materialization review means receipt content was drafted or files were written",
    "receipt content draft generation means receipt files were written or approved",
    "receipt file materialization proof means receipt files were written or approved",
    "p-puck proof closes semantic authority",
  ] {
    assert_eq!(
      as_str(get(corrections.get(claim).expect("correction"), "status")),
      "Held"
    );
  }
  assert_eq!(
    as_str(get(
      corrections
        .get("host code should grow for every discovery")
        .expect("host code correction"),
      "status"
    )),
    "Keep"
  );
}

#[test]
fn inherited_status_counts_current_old_function_challenge_state() {
  let run = eval_fixture();
  let status = get(run, "inherited-status");
  assert_eq!(as_i64(get(status, "legacy-extern-count")), 12);
  assert_eq!(as_i64(get(status, "legacy-externs-classified")), 12);
  assert_eq!(as_str(get(status, "promote-phase")), "R7");
  assert_eq!(
    as_str(get(status, "evaluate-select-phase")),
    "scoped-runtime-adapter-install"
  );
  assert_eq!(as_str(get(status, "lift-query-emit-phase")), "R7");
  assert_eq!(
    as_str(get(status, "lift-query-emit-compat")),
    "tesseract-macro-ontology-r7-compat-archive-lift-query-emit-surface-triple"
  );
  assert_eq!(
    as_str(get(status, "lift-query-emit-compat-status")),
    "compat-retained-for-lift-query-emit-surface-triple"
  );
  assert_eq!(
    as_str(get(status, "host-removal-map")),
    "tesseract-macro-ontology-host-code-removal-map"
  );
  assert!(as_bool(get(status, "host-removal-map-written")));
  assert_eq!(
    as_str(get(status, "macro-only-boot-manifest")),
    "tesseract-macro-ontology-macro-only-boot-manifest"
  );
  assert!(as_bool(get(status, "macro-only-boot-manifest-written")));
  assert_eq!(
    as_str(get(status, "macro-only-boot-execution-attempt")),
    "tesseract-macro-ontology-macro-only-boot-execution-attempt"
  );
  assert!(as_bool(get(status, "macro-only-boot-execution-attempted")));
  assert!(!as_bool(get(
    status,
    "boot-execution-attempt-boot-executed"
  )));
  assert!(as_bool(get(status, "boot-execution-attempt-held")));
  assert_eq!(
    as_str(get(status, "macro-only-boot-runner-owner")),
    "tesseract-macro-ontology-macro-only-boot-runner-owner"
  );
  assert!(as_bool(get(status, "macro-only-boot-runner-owner-present")));
  assert_eq!(
    as_str(get(status, "bounded-replay-strategy")),
    "tesseract-macro-ontology-macro-only-bounded-replay-strategy"
  );
  assert!(as_bool(get(
    status,
    "bounded-full-graph-replay-strategy-present"
  )));
  assert_eq!(
    as_str(get(status, "regression-corpus-retention")),
    "tesseract-macro-ontology-macro-only-regression-corpus-retention"
  );
  assert!(as_bool(get(status, "regression-corpus-transfer-present")));
  assert_eq!(
    as_str(get(status, "bootstrap-audit-update")),
    "tesseract-macro-ontology-macro-only-bootstrap-audit-update"
  );
  assert!(as_bool(get(
    status,
    "bootstrap-status-audit-update-plan-present"
  )));
  assert_eq!(as_str(get(status, "runner-after-audit-status")), "Held");
  assert_eq!(as_i64(get(status, "runner-after-audit-missing-count")), 3);
  assert_eq!(
    as_str(get(status, "compare-after-boot-proof")),
    "tesseract-macro-ontology-macro-only-compare-after-boot"
  );
  assert!(as_bool(get(status, "compare-after-boot")));
  assert_eq!(as_str(get(status, "runner-after-compare-status")), "Held");
  assert_eq!(as_i64(get(status, "runner-after-compare-missing-count")), 2);
  assert_eq!(
    as_str(get(status, "target-delete-preflight")),
    "tesseract-macro-ontology-macro-only-target-delete-preflight"
  );
  assert!(as_bool(get(status, "target-delete-preflight-present")));
  assert_eq!(as_str(get(status, "runner-after-preflight-status")), "Held");
  assert_eq!(
    as_i64(get(status, "runner-after-preflight-missing-count")),
    2
  );
  assert_eq!(
    as_str(get(status, "target-specific-delete-proof")),
    "tesseract-macro-ontology-macro-only-target-specific-delete-proof"
  );
  assert!(as_bool(get(status, "target-specific-delete-proof-present")));
  assert_eq!(
    as_str(get(status, "runner-after-target-proof-status")),
    "Held"
  );
  assert_eq!(
    as_i64(get(status, "runner-after-target-proof-missing-count")),
    1
  );
  assert_eq!(
    as_str(get(status, "fresh-p-puck-current-cut")),
    "tesseract-macro-ontology-macro-only-fresh-p-puck-current-cut"
  );
  assert!(as_bool(get(status, "fresh-p-puck-after-current-cut")));
  assert_eq!(
    as_str(get(status, "runner-after-fresh-puck-status")),
    "runner-ready-for-bounded-replay"
  );
  assert_eq!(
    as_i64(get(status, "runner-after-fresh-puck-missing-count")),
    0
  );
  assert!(as_bool(get(status, "ready-for-bounded-replay")));
  assert_eq!(
    as_str(get(status, "bounded-replay-execution")),
    "tesseract-macro-ontology-macro-only-bounded-replay-execution"
  );
  assert!(as_bool(get(status, "bounded-replay-executed")));
  assert_eq!(as_i64(get(status, "bounded-replay-step-count")), 11);
  assert_eq!(
    as_str(get(status, "bounded-replay-semantic-delta-status")),
    "empty-or-held-only"
  );
  assert_eq!(
    as_str(get(status, "post-bounded-replay-p-puck-current-cut")),
    "tesseract-macro-ontology-macro-only-post-bounded-replay-p-puck-current-cut"
  );
  assert!(as_bool(get(
    status,
    "post-bounded-replay-p-puck-current-cut-present"
  )));
  assert_eq!(
    as_i64(get(status, "post-bounded-replay-p-puck-duration-ms")),
    4934
  );
  assert_eq!(
    as_str(get(status, "post-bounded-replay-p-puck-slow-path-status")),
    "within-threshold"
  );
  assert_eq!(
    as_str(get(status, "full-current-receipt-audit")),
    "tesseract-macro-ontology-macro-only-full-current-receipt-audit"
  );
  assert!(as_bool(get(status, "full-current-receipt-audit-present")));
  assert_eq!(
    as_i64(get(status, "full-current-receipt-audit-total-tests")),
    915
  );
  assert_eq!(
    as_i64(get(status, "full-current-receipt-audit-source-tracked")),
    18167
  );
  assert_eq!(
    as_i64(get(status, "full-current-receipt-audit-source-indexed")),
    18167
  );
  assert_eq!(
    as_str(get(status, "macro-only-boot-execution-proof")),
    "tesseract-macro-ontology-macro-only-boot-execution-proof"
  );
  assert!(as_bool(get(
    status,
    "macro-only-boot-execution-proof-present"
  )));
  assert_eq!(
    as_i64(get(status, "macro-only-boot-proof-total-tests")),
    931
  );
  assert_eq!(
    as_i64(get(status, "macro-only-boot-proof-source-tracked")),
    18172
  );
  assert_eq!(
    as_i64(get(status, "macro-only-boot-proof-source-indexed")),
    18172
  );
  assert!(as_bool(get(status, "boot-executed")));
  assert_eq!(
    as_str(get(status, "macro-only-runtime-owner-proof")),
    "tesseract-macro-ontology-macro-only-runtime-owner-proof"
  );
  assert!(as_bool(get(
    status,
    "macro-only-runtime-owner-proof-present"
  )));
  assert_eq!(
    as_i64(get(status, "macro-only-runtime-owner-proof-total-tests")),
    947
  );
  assert_eq!(
    as_i64(get(status, "macro-only-runtime-owner-proof-source-tracked")),
    18177
  );
  assert_eq!(
    as_i64(get(status, "macro-only-runtime-owner-proof-source-indexed")),
    18177
  );
  assert_eq!(
    as_str(get(status, "macro-only-runtime-owner-scope")),
    "bounded-receipt-trajectory-owner"
  );
  assert!(as_bool(get(status, "macro-only-runtime-owner-booted")));
  assert_eq!(
    as_str(get(status, "macro-only-semantic-owner-proof")),
    "tesseract-macro-ontology-macro-only-semantic-owner-proof"
  );
  assert!(as_bool(get(
    status,
    "macro-only-semantic-owner-proof-present"
  )));
  assert_eq!(
    as_i64(get(status, "macro-only-semantic-owner-proof-total-tests")),
    963
  );
  assert_eq!(
    as_i64(get(
      status,
      "macro-only-semantic-owner-proof-source-tracked"
    )),
    18182
  );
  assert_eq!(
    as_i64(get(
      status,
      "macro-only-semantic-owner-proof-source-indexed"
    )),
    18182
  );
  assert_eq!(
    as_str(get(status, "macro-only-semantic-owner-scope")),
    "bounded-generated-ontology-semantic-owner"
  );
  assert!(as_bool(get(status, "semantic-owner")));
  assert_eq!(
    as_str(get(status, "macro-only-host-removal-execution-proof")),
    "tesseract-macro-ontology-macro-only-host-removal-execution-proof"
  );
  assert!(as_bool(get(
    status,
    "macro-only-host-removal-execution-proof-present"
  )));
  assert_eq!(
    as_i64(get(
      status,
      "macro-only-host-removal-execution-proof-total-tests"
    )),
    981
  );
  assert_eq!(
    as_i64(get(
      status,
      "macro-only-host-removal-execution-proof-source-tracked"
    )),
    18187
  );
  assert_eq!(
    as_i64(get(
      status,
      "macro-only-host-removal-execution-proof-source-indexed"
    )),
    18187
  );
  assert!(as_bool(get(status, "host-removal-execution-gate-present")));
  assert!(!as_bool(get(status, "host-removal-execution-authorized")));
  assert!(as_bool(get(status, "fresh-puck-before-delete-required")));
  assert_eq!(
    as_str(get(
      status,
      "macro-only-host-removal-fresh-puck-current-cut"
    )),
    "tesseract-macro-ontology-macro-only-host-removal-fresh-p-puck-current-cut"
  );
  assert!(as_bool(get(
    status,
    "macro-only-host-removal-fresh-puck-current-cut-present"
  )));
  assert_eq!(
    as_i64(get(
      status,
      "macro-only-host-removal-fresh-puck-duration-ms"
    )),
    5389
  );
  assert_eq!(
    as_str(get(
      status,
      "macro-only-host-removal-fresh-puck-slow-path-status"
    )),
    "slow-path-candidate"
  );
  assert!(as_bool(get(
    status,
    "fresh-puck-before-host-removal-execution"
  )));
  assert!(as_bool(get(status, "host-removal-slow-path-candidate")));
  assert_eq!(
    as_str(get(
      status,
      "macro-only-host-removal-slow-path-repeat-proof"
    )),
    "tesseract-macro-ontology-macro-only-host-removal-slow-path-repeat-proof"
  );
  assert!(as_bool(get(
    status,
    "macro-only-host-removal-slow-path-repeat-proof-present"
  )));
  assert_eq!(
    as_i64(get(
      status,
      "macro-only-host-removal-slow-path-repeat-duration-ms"
    )),
    551
  );
  assert_eq!(
    as_str(get(
      status,
      "macro-only-host-removal-slow-path-repeat-status"
    )),
    "within-threshold"
  );
  assert!(as_bool(get(
    status,
    "host-removal-slow-path-repeat-frontier-closed"
  )));
  assert!(!as_bool(get(status, "host-removal-persistent-slow-path")));
  assert!(!as_bool(get(
    status,
    "host-removal-self-optimization-candidate"
  )));
  assert_eq!(
    as_str(get(
      status,
      "macro-only-host-removal-delete-patch-candidate"
    )),
    "tesseract-macro-ontology-macro-only-host-removal-delete-patch-candidate"
  );
  assert!(as_bool(get(
    status,
    "macro-only-host-removal-delete-patch-candidate-present"
  )));
  assert!(as_bool(get(status, "actual-host-removal-patch-candidate")));
  assert!(!as_bool(get(
    status,
    "actual-host-removal-patch-authorized"
  )));
  assert_eq!(
    as_i64(get(
      status,
      "host-removal-delete-patch-candidate-target-count"
    )),
    5
  );
  assert_eq!(
    as_i64(get(
      status,
      "host-removal-delete-patch-candidate-total-tests"
    )),
    1035
  );
  assert_eq!(
    as_i64(get(
      status,
      "host-removal-delete-patch-candidate-source-tracked"
    )),
    18202
  );
  assert_eq!(
    as_i64(get(
      status,
      "host-removal-delete-patch-candidate-source-indexed"
    )),
    18202
  );
  assert_eq!(
    as_str(get(
      status,
      "macro-only-host-removal-fresh-delete-puck-current-cut"
    )),
    "tesseract-macro-ontology-macro-only-host-removal-fresh-delete-p-puck-current-cut"
  );
  assert!(as_bool(get(
    status,
    "macro-only-host-removal-fresh-delete-puck-current-cut-present"
  )));
  assert_eq!(
    as_i64(get(
      status,
      "macro-only-host-removal-fresh-delete-puck-duration-ms"
    )),
    1318
  );
  assert_eq!(
    as_str(get(
      status,
      "macro-only-host-removal-fresh-delete-puck-slow-path-status"
    )),
    "within-threshold"
  );
  assert_eq!(
    as_str(get(
      status,
      "macro-only-host-removal-fresh-delete-puck-duration-delta-status"
    )),
    "no-previous-report"
  );
  assert!(as_bool(get(status, "fresh-puck-before-delete")));
  assert!(as_bool(get(
    status,
    "fresh-puck-before-delete-as-delete-ready-frontier-closed"
  )));
  assert_eq!(
    as_i64(get(status, "host-removal-fresh-delete-puck-total-tests")),
    1053
  );
  assert_eq!(
    as_i64(get(status, "host-removal-fresh-delete-puck-source-tracked")),
    18207
  );
  assert_eq!(
    as_i64(get(status, "host-removal-fresh-delete-puck-source-indexed")),
    18207
  );
  assert_eq!(
    as_str(get(status, "macro-only-self-receipt-frontier-emission")),
    "tesseract-macro-ontology-macro-only-self-receipt-frontier-emission"
  );
  assert!(as_bool(get(
    status,
    "macro-only-self-receipt-frontier-emission-present"
  )));
  assert!(as_bool(get(status, "receipt-needed-detector")));
  assert_eq!(as_i64(get(status, "emitted-receipt-candidate-count")), 5);
  assert_eq!(
    as_str(get(status, "macro-only-receipt-skeleton-generator")),
    "tesseract-macro-ontology-macro-only-receipt-skeleton-generator"
  );
  assert!(as_bool(get(
    status,
    "macro-only-receipt-skeleton-generator-present"
  )));
  assert!(as_bool(get(status, "receipt-skeleton-generator")));
  assert!(as_bool(get(status, "skeleton-data-only")));
  assert_eq!(as_i64(get(status, "generated-receipt-skeleton-count")), 5);
  assert_eq!(
    as_str(get(status, "macro-only-receipt-materialization-review")),
    "tesseract-macro-ontology-macro-only-receipt-materialization-review"
  );
  assert!(as_bool(get(
    status,
    "macro-only-receipt-materialization-review-present"
  )));
  assert!(as_bool(get(status, "receipt-materialization-review")));
  assert!(as_bool(get(status, "materialization-reviewed")));
  assert!(as_bool(get(status, "materialization-review-only")));
  assert_eq!(as_i64(get(status, "reviewed-receipt-skeleton-count")), 5);
  assert_eq!(
    as_str(get(status, "macro-only-receipt-content-draft-generation")),
    "tesseract-macro-ontology-macro-only-receipt-content-draft-generation"
  );
  assert!(as_bool(get(
    status,
    "macro-only-receipt-content-draft-generation-present"
  )));
  assert!(as_bool(get(status, "receipt-content-draft-generation")));
  assert!(as_bool(get(status, "content-draft-generated")));
  assert!(as_bool(get(status, "draft-data-only")));
  assert_eq!(as_i64(get(status, "drafted-receipt-review-count")), 5);
  assert_eq!(
    as_str(get(status, "macro-only-receipt-file-writer")),
    "tesseract-macro-ontology-macro-only-receipt-file-writer"
  );
  assert!(as_bool(get(
    status,
    "macro-only-receipt-file-writer-present"
  )));
  assert!(as_bool(get(status, "receipt-file-writer")));
  assert!(as_bool(get(status, "receipt-file-artifact-generated")));
  assert!(as_bool(get(status, "writer-candidate-only")));
  assert_eq!(as_i64(get(status, "written-receipt-artifact-count")), 5);
  assert_eq!(
    as_str(get(status, "macro-only-receipt-file-materialization-proof")),
    "tesseract-macro-ontology-macro-only-receipt-file-materialization-proof"
  );
  assert!(as_bool(get(
    status,
    "macro-only-receipt-file-materialization-proof-present"
  )));
  assert!(as_bool(get(status, "receipt-file-materialization-proof")));
  assert!(as_bool(get(status, "materialization-proof-only")));
  assert_eq!(as_i64(get(status, "materialization-proof-count")), 5);
  assert_eq!(as_i64(get(status, "source-artifact-count")), 5);
  assert!(!as_bool(get(status, "disk-write-executed")));
  assert!(!as_bool(get(status, "receipt-auto-written")));
  assert!(!as_bool(get(status, "receipt-auto-approved")));
  assert!(!as_bool(get(status, "receipt-file-created")));
  assert!(!as_bool(get(status, "receipt-content-written")));
  assert_eq!(as_i64(get(status, "delete-ready-target-count")), 0);
  assert_eq!(as_i64(get(status, "external-solver-dependency-count")), 0);
  assert!(!as_bool(get(status, "llm-main-system")));
}

#[test]
fn top_level_state_keeps_bootstrap_audit_as_non_runtime_non_command() {
  let run = eval_fixture();
  assert_eq!(
    as_str(get(run, "replacement-readiness")),
    "receipt-file-materialization-proof-present"
  );
  assert_eq!(
    as_str(get(run, "owner-switch")),
    "promote-evaluate-select-and-lift-query-emit"
  );
  assert!(as_bool(get(run, "host-removal-map-written")));
  assert!(as_bool(get(run, "macro-only-boot-manifest-written")));
  assert!(as_bool(get(run, "macro-only-boot-execution-attempted")));
  assert!(as_bool(get(run, "macro-only-boot-runner-owner-present")));
  assert!(as_bool(get(
    run,
    "bounded-full-graph-replay-strategy-present"
  )));
  assert!(as_bool(get(run, "regression-corpus-transfer-present")));
  assert!(as_bool(get(
    run,
    "bootstrap-status-audit-update-plan-present"
  )));
  assert!(as_bool(get(run, "compare-after-boot")));
  assert!(as_bool(get(run, "target-delete-preflight-present")));
  assert!(as_bool(get(run, "target-specific-delete-proof-present")));
  assert!(as_bool(get(run, "fresh-p-puck-after-current-cut")));
  assert!(as_bool(get(run, "full-current-receipt-audit")));
  assert!(as_bool(get(run, "macro-only-boot-execution-proof-present")));
  assert!(as_bool(get(run, "macro-only-runtime-owner-proof-present")));
  assert!(as_bool(get(run, "macro-only-semantic-owner-proof-present")));
  assert!(as_bool(get(
    run,
    "macro-only-host-removal-execution-proof-present"
  )));
  assert!(as_bool(get(
    run,
    "macro-only-host-removal-fresh-puck-current-cut-present"
  )));
  assert!(as_bool(get(
    run,
    "macro-only-host-removal-slow-path-repeat-proof-present"
  )));
  assert!(as_bool(get(
    run,
    "macro-only-host-removal-delete-patch-candidate-present"
  )));
  assert!(as_bool(get(
    run,
    "macro-only-host-removal-fresh-delete-puck-current-cut-present"
  )));
  assert!(as_bool(get(
    run,
    "macro-only-self-receipt-frontier-emission-present"
  )));
  assert!(as_bool(get(
    run,
    "macro-only-receipt-skeleton-generator-present"
  )));
  assert!(as_bool(get(
    run,
    "macro-only-receipt-materialization-review-present"
  )));
  assert!(as_bool(get(
    run,
    "macro-only-receipt-content-draft-generation-present"
  )));
  assert!(as_bool(get(run, "macro-only-receipt-file-writer-present")));
  assert!(as_bool(get(
    run,
    "macro-only-receipt-file-materialization-proof-present"
  )));
  assert!(as_bool(get(run, "receipt-needed-detector")));
  assert_eq!(as_i64(get(run, "emitted-receipt-candidate-count")), 5);
  assert!(as_bool(get(run, "receipt-skeleton-generator")));
  assert!(as_bool(get(run, "skeleton-data-only")));
  assert_eq!(as_i64(get(run, "generated-receipt-skeleton-count")), 5);
  assert!(as_bool(get(run, "receipt-materialization-review")));
  assert!(as_bool(get(run, "materialization-reviewed")));
  assert!(as_bool(get(run, "materialization-review-only")));
  assert_eq!(as_i64(get(run, "reviewed-receipt-skeleton-count")), 5);
  assert!(as_bool(get(run, "receipt-content-draft-generation")));
  assert!(as_bool(get(run, "content-draft-generated")));
  assert!(as_bool(get(run, "draft-data-only")));
  assert_eq!(as_i64(get(run, "drafted-receipt-review-count")), 5);
  assert!(as_bool(get(run, "receipt-file-writer")));
  assert!(as_bool(get(run, "receipt-file-artifact-generated")));
  assert!(as_bool(get(run, "writer-candidate-only")));
  assert_eq!(as_i64(get(run, "written-receipt-artifact-count")), 5);
  assert!(as_bool(get(run, "receipt-file-materialization-proof")));
  assert!(as_bool(get(run, "materialization-proof-only")));
  assert_eq!(as_i64(get(run, "materialization-proof-count")), 5);
  assert_eq!(as_i64(get(run, "source-artifact-count")), 5);
  assert!(!as_bool(get(run, "disk-write-executed")));
  assert!(!as_bool(get(run, "receipt-auto-written")));
  assert!(!as_bool(get(run, "receipt-auto-approved")));
  assert!(!as_bool(get(run, "receipt-file-created")));
  assert!(!as_bool(get(run, "receipt-content-written")));
  assert!(as_bool(get(run, "ready-for-bounded-replay")));
  assert!(as_bool(get(run, "bounded-replay-executed")));
  assert!(as_bool(get(run, "post-bounded-replay-p-puck-current-cut")));
  assert!(as_bool(get(run, "boot-executed")));
  assert!(as_bool(get(run, "macro-only-runtime-owner-booted")));
  assert!(as_bool(get(run, "semantic-owner")));
  assert!(as_bool(get(run, "semantic-owner-proof")));
  assert_eq!(
    as_str(get(run, "semantic-owner-scope")),
    "bounded-generated-ontology-semantic-owner"
  );
  assert!(as_bool(get(run, "host-removal-execution-proof")));
  assert!(as_bool(get(run, "host-removal-execution-gate-present")));
  assert!(!as_bool(get(run, "host-removal-execution-authorized")));
  assert!(as_bool(get(run, "fresh-puck-before-delete-required")));
  assert!(as_bool(get(run, "host-removal-fresh-p-puck-current-cut")));
  assert!(as_bool(get(
    run,
    "fresh-puck-before-host-removal-execution"
  )));
  assert!(as_bool(get(run, "slow-path-candidate")));
  assert!(as_bool(get(run, "slow-path-repeat-within-threshold")));
  assert!(as_bool(get(run, "slow-path-repeat-frontier-closed")));
  assert!(!as_bool(get(run, "persistent-slow-path")));
  assert!(!as_bool(get(run, "profile-required-from-repeat")));
  assert!(!as_bool(get(run, "self-optimization-candidate")));
  assert!(as_bool(get(run, "actual-host-removal-patch-candidate")));
  assert!(!as_bool(get(run, "actual-host-removal-patch-authorized")));
  assert_eq!(as_i64(get(run, "delete-patch-candidate-target-count")), 5);
  assert!(as_bool(get(
    run,
    "fresh-puck-before-delete-as-delete-ready-frontier-closed"
  )));
  assert_eq!(as_i64(get(run, "delete-ready-target-count")), 0);
  assert!(!as_bool(get(run, "delete-ready")));
  assert!(!as_bool(get(run, "remove-now")));
  assert!(as_bool(get(run, "fresh-puck-before-delete")));
  assert!(!as_bool(get(run, "new-engine-from-zero")));
  assert!(!as_bool(get(run, "host-code-removal-started")));
  assert!(!as_bool(get(run, "host-removal-safe")));
  assert!(!as_bool(get(run, "runtime-install")));
  assert!(!as_bool(get(run, "global-ontology-runtime")));
  assert!(!as_bool(get(run, "implementation-command")));

  let rejects = string_set(get_path(run, &["negative-held-evidence", "rejects"]));
  assert!(rejects.contains("new-engine-from-zero-before-macro-runtime-owner"));
  assert!(rejects.contains("lift-query-emit-r7-compat-as-runtime-install"));
  assert!(rejects.contains("lift-query-emit-delete-without-r7-delete-proof"));
  assert!(rejects.contains("host-removal-map-as-delete-proof"));
  assert!(rejects.contains("macro-only-boot-manifest-as-runtime-boot"));
  assert!(rejects.contains("macro-only-boot-attempt-as-runtime-boot"));
  assert!(rejects.contains("macro-only-boot-runner-owner-as-runtime-boot"));
  assert!(rejects.contains("bounded-replay-strategy-as-runtime-boot"));
  assert!(rejects.contains("regression-corpus-retention-as-runtime-boot"));
  assert!(rejects.contains("regression-corpus-retention-as-fresh-puck-or-compare"));
  assert!(rejects.contains("bootstrap-audit-update-as-runtime-boot"));
  assert!(rejects.contains("bootstrap-audit-update-as-fresh-puck-or-compare"));
  assert!(rejects.contains("compare-after-boot-as-runtime-boot"));
  assert!(rejects.contains("compare-after-boot-as-fresh-puck-or-semantic-owner"));
  assert!(rejects.contains("target-delete-preflight-as-delete-proof"));
  assert!(rejects.contains("target-delete-preflight-as-host-removal"));
  assert!(rejects.contains("fresh-puck-as-full-current-receipt-audit"));
  assert!(rejects.contains("fresh-puck-as-replay-executed"));
  assert!(rejects.contains("fresh-puck-as-boot-executed"));
  assert!(rejects.contains("fresh-puck-as-runtime-owner"));
  assert!(rejects.contains("fresh-puck-as-host-removal-started"));
  assert!(rejects.contains("bounded-replay-execution-as-runtime-boot"));
  assert!(rejects.contains("bounded-replay-execution-as-runtime-owner"));
  assert!(rejects.contains("bounded-replay-execution-as-host-removal"));
  assert!(rejects.contains("bounded-replay-execution-as-semantic-owner"));
  assert!(rejects.contains("post-replay-puck-as-full-current-receipt-audit"));
  assert!(rejects.contains("post-replay-puck-as-runtime-boot"));
  assert!(rejects.contains("post-replay-puck-as-host-removal"));
  assert!(rejects.contains("post-replay-puck-as-semantic-owner"));
  assert!(rejects.contains("full-current-receipt-audit-as-runtime-boot"));
  assert!(rejects.contains("full-current-receipt-audit-as-host-removal"));
  assert!(rejects.contains("full-current-receipt-audit-as-semantic-owner"));
  assert!(rejects.contains("boot-execution-proof-as-runtime-owner"));
  assert!(rejects.contains("boot-execution-proof-as-new-engine-from-zero"));
  assert!(rejects.contains("boot-execution-proof-as-host-removal"));
  assert!(rejects.contains("boot-execution-proof-as-semantic-owner"));
  assert!(rejects.contains("runtime-owner-proof-as-new-engine-from-zero"));
  assert!(rejects.contains("runtime-owner-proof-as-global-runtime-install"));
  assert!(rejects.contains("runtime-owner-proof-as-host-removal"));
  assert!(rejects.contains("runtime-owner-proof-as-semantic-owner"));
  assert!(rejects.contains("semantic-owner-proof-as-new-engine-from-zero"));
  assert!(rejects.contains("semantic-owner-proof-as-global-runtime-install"));
  assert!(rejects.contains("semantic-owner-proof-as-host-removal"));
  assert!(rejects.contains("semantic-owner-proof-as-delete-ready"));
  assert!(rejects.contains("host-removal-execution-proof-as-delete-command"));
  assert!(rejects.contains("host-removal-execution-proof-as-delete-ready"));
  assert!(rejects.contains("host-removal-execution-proof-as-global-runtime-install"));
  assert!(rejects.contains("host-removal-fresh-puck-as-delete-ready"));
  assert!(rejects.contains("host-removal-fresh-puck-as-actual-delete"));
  assert!(rejects.contains("host-removal-fresh-puck-as-global-runtime-install"));
  assert!(rejects.contains("host-removal-fresh-puck-as-semantic-owner"));
  assert!(rejects.contains("host-removal-fresh-puck-slow-path-ignored"));
  assert!(rejects.contains("host-removal-slow-path-repeat-as-delete-ready"));
  assert!(rejects.contains("host-removal-slow-path-repeat-as-actual-delete"));
  assert!(rejects.contains("host-removal-slow-path-repeat-as-global-runtime-install"));
  assert!(rejects.contains("host-removal-slow-path-repeat-as-semantic-owner"));
  assert!(rejects.contains("host-removal-delete-patch-candidate-as-delete-ready"));
  assert!(rejects.contains("host-removal-delete-patch-candidate-as-remove-now"));
  assert!(rejects.contains("host-removal-delete-patch-candidate-as-implementation-command"));
  assert!(rejects.contains("host-removal-delete-patch-candidate-as-global-runtime-install"));
  assert!(rejects.contains("host-removal-fresh-delete-puck-as-delete-ready"));
  assert!(rejects.contains("host-removal-fresh-delete-puck-as-remove-now"));
  assert!(rejects.contains("host-removal-fresh-delete-puck-as-implementation-command"));
  assert!(rejects.contains("host-removal-fresh-delete-puck-as-host-removal-started"));
  assert!(rejects.contains("host-removal-fresh-delete-puck-as-global-runtime-install"));
  assert!(rejects.contains("host-removal-fresh-delete-puck-as-runtime-api-flattening"));
  assert!(rejects.contains("host-removal-fresh-delete-puck-as-meaning-db"));
  assert!(rejects.contains("host-removal-fresh-delete-puck-as-semantic-owner"));
  assert!(rejects.contains("self-receipt-emission-as-auto-writer"));
  assert!(rejects.contains("self-receipt-emission-as-auto-approval"));
  assert!(rejects.contains("self-receipt-emission-as-implementation-command"));
  assert!(rejects.contains("self-receipt-emission-as-global-runtime"));
  assert!(rejects.contains("self-receipt-emission-as-meaning-db"));
  assert!(rejects.contains("receipt-skeleton-as-file-created"));
  assert!(rejects.contains("receipt-skeleton-as-auto-approval"));
  assert!(rejects.contains("receipt-skeleton-as-implementation-command"));
  assert!(rejects.contains("receipt-skeleton-as-global-runtime"));
  assert!(rejects.contains("receipt-skeleton-as-meaning-db"));
  assert!(rejects.contains("materialization-review-as-file-created"));
  assert!(rejects.contains("materialization-review-as-content-written"));
  assert!(rejects.contains("materialization-review-as-content-draft-generated"));
  assert!(rejects.contains("materialization-review-as-auto-approval"));
  assert!(rejects.contains("materialization-review-as-implementation-command"));
  assert!(rejects.contains("materialization-review-as-global-runtime"));
  assert!(rejects.contains("materialization-review-as-meaning-db"));
  assert!(rejects.contains("content-draft-as-file-created"));
  assert!(rejects.contains("content-draft-as-content-written"));
  assert!(rejects.contains("content-draft-as-auto-write"));
  assert!(rejects.contains("content-draft-as-auto-approval"));
  assert!(rejects.contains("content-draft-as-delete-ready"));
  assert!(rejects.contains("content-draft-as-implementation-command"));
  assert!(rejects.contains("content-draft-as-global-runtime"));
  assert!(rejects.contains("content-draft-as-meaning-db"));
  assert!(rejects.contains("file-writer-as-disk-write"));
  assert!(rejects.contains("file-writer-as-content-written"));
  assert!(rejects.contains("file-writer-as-auto-approval"));
  assert!(rejects.contains("file-writer-as-target-frontier-closed"));
  assert!(rejects.contains("file-writer-as-delete-ready"));
  assert!(rejects.contains("file-writer-as-implementation-command"));
  assert!(rejects.contains("file-writer-as-global-runtime"));
  assert!(rejects.contains("file-writer-as-meaning-db"));
  assert!(rejects.contains("materialization-proof-as-disk-write"));
  assert!(rejects.contains("materialization-proof-as-file-created"));
  assert!(rejects.contains("materialization-proof-as-content-written"));
  assert!(rejects.contains("materialization-proof-as-auto-approval"));
  assert!(rejects.contains("materialization-proof-as-target-frontier-closed"));
  assert!(rejects.contains("materialization-proof-as-delete-ready"));
  assert!(rejects.contains("materialization-proof-as-implementation-command"));
  assert!(rejects.contains("materialization-proof-as-global-runtime"));
  assert!(rejects.contains("materialization-proof-as-runtime-api-flattening"));
  assert!(rejects.contains("materialization-proof-as-meaning-db"));
  assert!(rejects.contains("materialization-proof-as-p-puck-semantic-owner"));
  assert!(rejects.contains("old-host-authority-for-fresh-delete-cut"));
  assert!(rejects.contains("receipt-eval-as-runtime-boot"));
}
