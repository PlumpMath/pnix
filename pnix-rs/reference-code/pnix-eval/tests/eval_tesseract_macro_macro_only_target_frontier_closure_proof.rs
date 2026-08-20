use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/tesseract-macro-legacy-probe/macro_only_target_frontier_closure_proof_receipt.px",
  )
}

fn eval_fixture() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = fixture_path();
    let json = std::thread::Builder::new()
      .name("macro-only-target-frontier-closure-proof-eval".to_string())
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("target frontier closure proof receipt")
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
    "tesseract-macro-ontology-macro-only-target-frontier-closure-proof"
  );
  assert_eq!(
    as_str(get(run, "constitution-owner")),
    "stdlib/lib/gate/tesseract-constitution.px"
  );
  assert_eq!(
    as_str(get(run, "source-receipt")),
    "tesseract-macro-ontology-macro-only-receipt-auto-approval-proof"
  );
}

#[test]
fn constitution_gate_blocks_target_frontier_closure_collapse() {
  let run = eval_fixture();
  let gate = get(run, "constitutionGate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "macro-only-target-frontier-closure-proof"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "target-frontier-closure-equals-delete-ready",
    "target-frontier-closure-equals-implementation-command",
    "target-frontier-closure-equals-global-runtime-install",
    "target-frontier-closure-equals-runtime-api-flattening",
    "target-frontier-closure-equals-meaning-db",
    "target-frontier-closure-equals-host-code-removal",
    "target-frontier-closure-equals-p-puck-semantic-owner",
    "old-host-code-authorizes-target-frontier-closure",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
}

#[test]
fn proof_generates_five_target_frontier_closure_records() {
  let run = eval_fixture();
  let proof = get(run, "target-frontier-closure-proof");
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
}

#[test]
fn closure_records_preserve_paths_and_defer_underlying_work() {
  let run = eval_fixture();
  let records = attrs_by_id(get(run, "target-frontier-closures"));
  assert_eq!(records.len(), 5);
  let record = records[
    "target-frontier-closure.auto-approval.content-write.file-creation.disk-write.materialization.file.draft.review.skeleton.candidate.receipt.macro-only-host-removal-delete-ready-target-proof"
  ];
  assert_eq!(
    as_str(get(record, "target-receipt")),
    "fixtures/tesseract-macro-legacy-probe/macro_only_host_removal_delete_ready_target_proof_receipt.px"
  );
  assert!(as_bool(get(record, "target-frontier-closure-proof")));
  assert!(as_bool(get(record, "target-frontier-closure-proof-only")));
  assert!(as_bool(get(record, "receipt-target-frontier-closed")));
  assert!(as_bool(get(record, "target-frontier-closed")));
  assert!(!as_bool(get(record, "underlying-work-frontier-closed")));
  for key in [
    "receipt-auto-approved",
    "delete-ready",
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
fn contract_closes_target_frontier_closure_proof_only() {
  let run = eval_fixture();
  let contract = get(run, "target-frontier-closure-contract");
  assert_eq!(
    as_str(get(contract, "proof-id")),
    "proof.macro-only.target-frontier-closure.v1"
  );
  assert!(as_bool(get(
    contract,
    "closes-target-frontier-closure-proof"
  )));
  assert_eq!(as_i64(get(contract, "target-frontier-closure-count")), 5);
  for key in [
    "closes-receipt-auto-approval",
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
fn migration_delta_closes_only_target_frontier_closure_frontier() {
  let run = eval_fixture();
  let delta = get(run, "migrationDelta");
  let closes = string_set(get(delta, "closes"));
  assert!(closes.contains("target-frontier-closure-after-receipt-auto-approval-proof"));
  assert_eq!(closes.len(), 1);
  let not_closed = string_set(get(delta, "does-not-close"));
  for expected in [
    "need.host-removal.delete-ready-targets-after-fresh-delete-puck",
    "need.host-removal.actual-host-removal-implementation-command",
    "need.global-runtime-install.proof-after-semantic-owner",
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
  let trials = attrs_by_id(get(run, "target-frontier-closure-trials"));
  assert_eq!(trials.len(), 20);
  assert_eq!(
    as_str(get(
      trials["trial.A.valid-target-frontier-closure-proof"],
      "outcome"
    )),
    "target-frontier-closure-proof-present"
  );
  for (id, held) in [
    (
      "trial.C.wrong-proof-id",
      "held.macro-only-target-frontier-closure.proof-id-mismatch",
    ),
    (
      "trial.D.stale-stage",
      "held.macro-only-target-frontier-closure.stale-current-stage",
    ),
    (
      "trial.E.source-mismatch",
      "held.macro-only-target-frontier-closure.source-mismatch",
    ),
    (
      "trial.F.auto-approval-missing",
      "held.macro-only-target-frontier-closure.auto-approval-proof-missing",
    ),
    (
      "trial.G.auto-approval-count-mismatch",
      "held.macro-only-target-frontier-closure.auto-approval-count-mismatch",
    ),
    (
      "trial.H.closure-count-mismatch",
      "held.macro-only-target-frontier-closure.closure-count-mismatch",
    ),
    (
      "trial.I.source-auto-approval-overclaim",
      "held.macro-only-target-frontier-closure.source-auto-approval-overclaim",
    ),
    (
      "trial.J.closure-authority-overclaim",
      "held.macro-only-target-frontier-closure.closure-authority-overclaim",
    ),
    (
      "trial.K.closure-shape-mismatch",
      "held.macro-only-target-frontier-closure.proof-shape-mismatch",
    ),
    (
      "trial.M.content-or-approval-overclaim",
      "held.macro-only-target-frontier-closure.content-or-approval-overclaim",
    ),
    (
      "trial.N.target-frontier-missing",
      "held.macro-only-target-frontier-closure.target-frontier-not-closed",
    ),
    (
      "trial.O.underlying-frontier-overclaim",
      "held.macro-only-target-frontier-closure.underlying-frontier-overclaim",
    ),
    (
      "trial.P.delete-overclaim",
      "held.macro-only-target-frontier-closure.delete-or-command-overclaim",
    ),
    (
      "trial.Q.runtime-overclaim",
      "held.macro-only-target-frontier-closure.runtime-overclaim",
    ),
    (
      "trial.R.p-puck-semantic-owner",
      "held.macro-only-target-frontier-closure.p-puck-semantic-owner",
    ),
    (
      "trial.S.old-host-authority",
      "held.macro-only-target-frontier-closure.old-host-authority",
    ),
    (
      "trial.T.gpl-family-dependency",
      "held.macro-only-target-frontier-closure.gpl-family-dependency",
    ),
  ] {
    assert_eq!(as_str(get(trials[id], "held-id")), held, "{id}");
  }
}

#[test]
fn discoveries_record_d604_through_d610() {
  let run = eval_fixture();
  let discoveries = attrs_by_id(get(run, "discoveries"));
  assert_eq!(discoveries.len(), 7);
  for expected in [
    "D604.target-frontier-closure-is-a-separate-post-auto-approval-proof",
    "D605.target-closure-preserves-five-lane-split",
    "D606.target-frontier-closed-does-not-mean-underlying-work-closed",
    "D607.target-closure-keeps-receipt-auto-approval-false",
    "D608.target-closure-records-return-to-remaining-open-frontiers",
    "D609.target-closure-hard-stops-block-delete-command-runtime-flattening-meaning-db",
    "D610.target-closure-makes-closed-frontier-claim-verifiable",
  ] {
    assert!(discoveries.contains_key(expected), "missing {expected}");
  }
}

#[test]
fn top_level_flags_close_target_frontier_but_keep_runtime_false() {
  let run = eval_fixture();
  assert!(as_bool(get(run, "target-frontier-closure-proof-present")));
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
