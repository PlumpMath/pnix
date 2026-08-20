//! Post-bounded-replay p-puck current-cut receipt.
//!
//! This pins the actual p-puck pnixc report over the bounded replay receipt as
//! current-cut freshness telemetry. It does not close full receipt audit, boot
//! execution, runtime ownership, semantic ownership, host removal, or deletion.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../..")
}

fn fixture_path() -> PathBuf {
  repo_root().join(
    "fixtures/tesseract-macro-legacy-probe/macro_only_post_bounded_replay_p_puck_current_cut_receipt.px",
  )
}

fn as_attrs(v: &Value) -> &BTreeMap<String, Value> {
  match v {
    Value::AttrSet(m) => m,
    other => panic!("expected attrset, got {:?}", other),
  }
}

fn as_list(v: &Value) -> &Vec<Value> {
  match v {
    Value::List(items) => items,
    other => panic!("expected list, got {:?}", other),
  }
}

fn as_str(v: &Value) -> &str {
  match v {
    Value::String(s) => s,
    Value::StringContext { text, .. } => text,
    other => panic!("expected string, got {:?}", other),
  }
}

fn as_bool(v: &Value) -> bool {
  match v {
    Value::Bool(b) => *b,
    other => panic!("expected bool, got {:?}", other),
  }
}

fn as_i64(v: &Value) -> i64 {
  match v {
    Value::Int(i) => *i,
    other => panic!("expected int, got {:?}", other),
  }
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
  let run = eval_file(&fixture_path()).expect("post replay p-puck receipt");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-macro-only-post-bounded-replay-p-puck-current-cut"
  );
  assert_eq!(
    as_str(get(&run, "truth-owner")),
    "project-wiki/maps/tesseract-macro-ontology-discovery-ledger.md"
  );
  assert_eq!(
    as_str(get(&run, "constitution-owner")),
    "stdlib/lib/gate/tesseract-constitution.px"
  );
  for path in [
    "stdlib/lib/gate/macro-only-boot-post-bounded-replay-p-puck-current-cut.px",
    "fixtures/pnix-query-runtime/macro-only-boot-post-bounded-replay-p-puck-current-cut-owner.px",
    "fixtures/tesseract-macro-legacy-probe/macro_only_post_bounded_replay_p_puck_current_cut_receipt.px",
  ] {
    assert!(repo_root().join(path).is_file(), "missing `{path}`");
  }
}

#[test]
fn constitution_gate_blocks_p_puck_current_cut_overclaims() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "macro-only-post-bounded-replay-p-puck-current-cut"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));

  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "p-puck-current-cut-equals-full-current-receipt-audit",
    "p-puck-current-cut-equals-boot-success",
    "p-puck-current-cut-equals-runtime-owner",
    "p-puck-current-cut-equals-semantic-owner",
    "p-puck-current-cut-equals-host-removal",
    "bounded-replay-plus-p-puck-equals-new-engine-from-zero",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
}

#[test]
fn contract_closes_post_replay_puck_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let contract = get(&run, "post-replay-p-puck-contract");
  assert_eq!(
    as_str(get(contract, "id")),
    "contract.macro-only-post-bounded-replay-p-puck-current-cut.v1"
  );
  assert_eq!(
    as_str(get(contract, "owner")),
    "stdlib.lib.gate.macro-only-boot-post-bounded-replay-p-puck-current-cut"
  );
  assert_eq!(
    as_str(get(contract, "current-status")),
    "post-bounded-replay-p-puck-current-cut-present"
  );
  assert!(as_bool(get(
    contract,
    "closes-post-bounded-replay-p-puck-current-cut"
  )));
  for key in [
    "closes-full-current-receipt-audit",
    "closes-boot-execution-proof",
    "closes-macro-only-runtime-owner",
    "closes-new-engine-from-zero",
    "closes-host-removal",
    "closes-delete-ready-targets",
    "closes-semantic-owner-proof",
  ] {
    assert!(!as_bool(get(contract, key)), "`{key}` must stay false");
  }
}

#[test]
fn proof_records_actual_p_puck_telemetry_after_bounded_replay() {
  let run = eval_file(&fixture_path()).unwrap();
  let proof = get(&run, "post-replay-p-puck-proof");
  assert_eq!(
    as_str(get(proof, "status")),
    "post-bounded-replay-p-puck-current-cut-present"
  );
  assert!(as_bool(get(
    proof,
    "post-bounded-replay-p-puck-current-cut"
  )));
  assert!(as_bool(get(proof, "bounded-replay-executed-input")));
  assert!(as_bool(get(proof, "p-puck-wrapper-proof")));
  assert_eq!(
    as_str(get(proof, "report-name")),
    "macro-only-current-cut-bounded-replay"
  );
  assert_eq!(
    as_str(get(proof, "audited-receipt")),
    "fixtures/tesseract-macro-legacy-probe/macro_only_bounded_replay_execution_receipt.px"
  );
  assert_eq!(as_i64(get(proof, "duration-ms")), 4934);
  assert_eq!(as_i64(get(proof, "slow-threshold-ms")), 5000);
  assert_eq!(as_str(get(proof, "slow-path-status")), "within-threshold");
  assert!(!as_bool(get(proof, "full-current-receipt-audit")));
  assert!(!as_bool(get(proof, "boot-executed")));
  assert!(!as_bool(get(proof, "macro-only-runtime-owner-booted")));
  assert!(!as_bool(get(proof, "host-code-removal-started")));
  assert!(!as_bool(get(proof, "semantic-owner")));
}

#[test]
fn trials_cover_valid_report_and_held_overclaims() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "post-replay-p-puck-trials"));
  assert_eq!(trials.len(), 11);
  assert_eq!(
    as_str(get(
      trials["trial.A.valid-post-replay-p-puck-current-cut"],
      "outcome"
    )),
    "post-bounded-replay-p-puck-current-cut-present"
  );
  assert_eq!(
    as_str(get(trials["trial.B.bounded-replay-input"], "outcome")),
    "tesseract-macro-ontology-macro-only-bounded-replay-execution"
  );
  for (id, held) in [
    (
      "trial.C.bounded-replay-missing",
      "held.macro-only-post-replay-p-puck.bounded-replay-missing",
    ),
    (
      "trial.D.report-mismatch",
      "held.macro-only-post-replay-p-puck.report-mismatch",
    ),
    (
      "trial.E.receipt-mismatch",
      "held.macro-only-post-replay-p-puck.current-cut-receipt-mismatch",
    ),
    (
      "trial.F.telemetry-missing",
      "held.macro-only-post-replay-p-puck.telemetry-missing",
    ),
    (
      "trial.G.full-audit-overclaim",
      "held.macro-only-post-replay-p-puck.full-audit-overclaim",
    ),
    (
      "trial.H.boot-claim",
      "held.macro-only-post-replay-p-puck.boot-or-runtime-claim",
    ),
    (
      "trial.I.host-removal-claim",
      "held.macro-only-post-replay-p-puck.host-removal-claim",
    ),
    (
      "trial.J.semantic-owner-claim",
      "held.macro-only-post-replay-p-puck.semantic-owner-claim",
    ),
    (
      "trial.K.gpl-family-dependency",
      "held.macro-only-post-replay-p-puck.gpl-family-dependency",
    ),
  ] {
    assert_eq!(as_str(get(trials[id], "outcome")), "Held");
    assert_eq!(as_str(get(trials[id], "held-id")), held);
  }
}

#[test]
fn six_layer_fold_records_current_cut_without_runtime_install() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-post-replay-p-puck-fold");
  for layer in [
    "surface", "ontology", "semantic", "gate", "runtime", "audit",
  ] {
    assert!(
      as_bool(get(get(fold, layer), "visible")),
      "layer `{layer}` invisible"
    );
  }
  assert!(as_bool(get_path(
    fold,
    &["ontology", "bounded-replay-executed-input"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["ontology", "p-puck-wrapper-proof"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["semantic", "p-puck-is-semantic-owner"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["semantic", "full-current-receipt-audit"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["runtime", "post-bounded-replay-p-puck-current-cut"]
  )));
  assert!(!as_bool(get_path(fold, &["runtime", "boot-executed"])));
  assert_eq!(as_i64(get_path(fold, &["audit", "duration-ms"])), 4934);
}

#[test]
fn migration_delta_keeps_full_audit_and_boot_open() {
  let run = eval_file(&fixture_path()).unwrap();
  let delta = get(&run, "migration-delta");
  let closes = string_set(get(delta, "closes"));
  assert!(closes.contains("need.bootstrap.post-bounded-replay-p-puck-current-cut"));
  let not = string_set(get(delta, "does-not-close"));
  assert!(not.contains("need.bootstrap.full-current-receipt-audit-after-bounded-replay"));
  assert!(not.contains("need.bootstrap.macro-only-boot-execution-proof"));
  assert!(not.contains("need.host-removal.host-code-removal-execution"));
  let next = string_set(get(delta, "next-required"));
  assert!(next.contains("full-current-receipt-audit-after-bounded-replay"));
  assert!(next.contains("macro-only-boot-execution-proof-after-post-replay-p-puck"));
}

#[test]
fn discoveries_record_d444_through_d451() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 8);
  for expected in [
    "D444.post-replay-p-puck-current-cut-must-follow-bounded-replay",
    "D445.p-puck-freshness-is-per-current-cut",
    "D446.actual-p-puck-telemetry-becomes-measurement-input",
    "D447.current-cut-p-puck-is-not-full-current-receipt-audit",
    "D448.p-puck-wrapper-cannot-turn-replay-into-boot",
    "D449.post-replay-p-puck-preserves-owner-and-license-boundaries",
    "D450.next-frontier-is-full-audit-plus-boot-proof",
    "D451.report-catalog-now-has-post-replay-current-cut-baseline",
  ] {
    let d = discoveries
      .get(expected)
      .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
    assert!(as_bool(get(d, "scenario-only")));
    assert_eq!(as_str(get(d, "decision-pressure")), "keep");
  }
}

#[test]
fn top_level_state_stays_non_boot_non_delete_non_semantic() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "replacement-readiness")),
    "post-bounded-replay-p-puck-current-cut-present-not-booted"
  );
  assert!(as_bool(get(&run, "post-bounded-replay-p-puck-current-cut")));
  assert!(as_bool(get(&run, "p-puck-wrapper-proof")));
  assert!(as_bool(get(&run, "bounded-replay-executed")));
  for key in [
    "full-current-receipt-audit",
    "boot-executed",
    "macro-only-runtime-owner-booted",
    "new-engine-from-zero",
    "runtime-install",
    "global-ontology-runtime",
    "host-code-removal-started",
    "host-removal-safe",
    "semantic-owner",
    "gpl-family-dependencies",
    "implementation-command",
  ] {
    assert!(!as_bool(get(&run, key)), "`{key}` must stay false");
  }
  assert_eq!(as_i64(get(&run, "delete-ready-target-count")), 0);
}
