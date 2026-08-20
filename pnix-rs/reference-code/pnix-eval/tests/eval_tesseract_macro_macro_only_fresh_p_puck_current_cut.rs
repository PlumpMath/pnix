//! Macro-only fresh p-puck current-cut receipt.
//!
//! This pins the first fresh p-puck proof after target-specific delete proof.
//! The proof is deliberately narrow: p-puck evaluates the latest current-cut
//! receipt via the pnixc preset and writes a report. That removes the runner's
//! final Held item and opens bounded replay readiness, but it does not execute
//! replay, boot the macro-only runtime, delete host code, or claim semantic
//! ownership.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../..")
}

fn fixture_path() -> PathBuf {
  repo_root()
    .join("fixtures/tesseract-macro-legacy-probe/macro_only_fresh_p_puck_current_cut_receipt.px")
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
fn fresh_puck_marker_and_owner_surfaces_are_pinned() {
  let run = eval_file(&fixture_path()).expect("fresh p-puck current-cut receipt");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-macro-only-fresh-p-puck-current-cut"
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
    "stdlib/lib/gate/macro-only-boot-fresh-p-puck-current-cut.px",
    "fixtures/pnix-query-runtime/macro-only-boot-fresh-p-puck-current-cut-owner.px",
    "fixtures/tesseract-macro-legacy-probe/macro_only_fresh_p_puck_current_cut_receipt.px",
  ] {
    assert!(repo_root().join(path).is_file(), "missing `{path}`");
  }
}

#[test]
fn p_puck_report_exists_outside_repo_as_runtime_sidecar() {
  let report = Path::new(
    "/home/gp/kimchi/dev/pnix/p-puck/runtime/pnix-reports/macro-only-current-cut-target-delete-proof.json",
  );
  assert!(report.is_file(), "missing p-puck report sidecar");
}

#[test]
fn constitution_gate_blocks_fresh_puck_overclaims() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "macro-only-fresh-p-puck-current-cut"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "px-owner-audit-only-equals-current-cut-proof",
    "fresh-puck-equals-full-current-receipt-audit",
    "fresh-puck-equals-replay-executed",
    "fresh-puck-equals-boot-executed",
    "fresh-puck-equals-runtime-owner",
    "fresh-puck-equals-semantic-owner",
    "fresh-puck-equals-host-removal",
    "fresh-puck-equals-delete-ready-targets",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
}

#[test]
fn fresh_puck_contract_closes_freshness_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let contract = get(&run, "fresh-p-puck-contract");
  assert_eq!(
    as_str(get(contract, "id")),
    "contract.macro-only-fresh-p-puck-current-cut.v1"
  );
  assert_eq!(
    as_str(get(contract, "owner")),
    "stdlib.lib.gate.macro-only-boot-fresh-p-puck-current-cut"
  );
  assert_eq!(as_str(get(contract, "preset")), "pnixc");
  assert_eq!(as_str(get(contract, "runner")), "cargo-bin");
  assert_eq!(as_str(get(contract, "telemetry-source")), "p-puck");
  assert_eq!(
    as_str(get(contract, "audited-receipt")),
    "fixtures/tesseract-macro-legacy-probe/macro_only_target_delete_proof_receipt.px"
  );
  assert_eq!(as_i64(get(contract, "previous-receipt-audit-count")), 38);
  assert_eq!(as_i64(get(contract, "current-tesseract-receipt-count")), 55);
  assert!(as_bool(get(contract, "closes-fresh-puck-proof")));
  for key in [
    "closes-full-current-receipt-audit",
    "closes-bounded-replay-execution-proof",
    "closes-boot-execution-proof",
    "closes-host-removal",
    "closes-delete-ready-targets",
    "closes-semantic-owner-proof",
    "owns-p-puck",
    "owns-runtime",
    "owns-semantic-authority",
    "runtime-install",
    "global-ontology-runtime",
  ] {
    assert!(!as_bool(get(contract, key)), "`{key}` must stay false");
  }
}

#[test]
fn fresh_puck_proof_records_current_cut_report_and_telemetry() {
  let run = eval_file(&fixture_path()).unwrap();
  let proof = get(&run, "fresh-p-puck-proof");
  assert_eq!(
    as_str(get(proof, "status")),
    "fresh-p-puck-current-cut-present"
  );
  assert!(as_bool(get(proof, "fresh-p-puck-after-current-cut")));
  assert!(as_bool(get(proof, "p-puck-wrapper-proof")));
  assert!(!as_bool(get(proof, "p-puck-is-semantic-owner")));
  assert!(!as_bool(get(proof, "full-current-receipt-audit")));
  assert_eq!(as_str(get(proof, "report-kind")), "pnix-preset");
  assert_eq!(
    as_str(get(proof, "report-name")),
    "macro-only-current-cut-target-delete-proof"
  );
  assert_eq!(as_str(get(proof, "preset")), "pnixc");
  assert_eq!(as_str(get(proof, "telemetry-source")), "p-puck");
  assert_eq!(as_i64(get(proof, "duration-ms")), 701);
  assert_eq!(as_str(get(proof, "slow-path-status")), "within-threshold");
}

#[test]
fn runner_after_fresh_puck_is_ready_for_bounded_replay_not_boot() {
  let run = eval_file(&fixture_path()).unwrap();
  let runner = get(&run, "runner-after-fresh-puck-current-cut");
  assert_eq!(
    as_str(get(runner, "status")),
    "runner-ready-for-bounded-replay"
  );
  assert_eq!(
    as_str(get(runner, "runner-status")),
    "ready-for-bounded-replay"
  );
  assert!(as_bool(get(runner, "ready-for-bounded-replay")));
  assert_eq!(as_list(get(runner, "missing")).len(), 0);
  assert!(matches!(get(runner, "held-id"), Value::Null));
  for key in [
    "boot-executed",
    "macro-only-runtime-owner-booted",
    "new-engine-from-zero",
    "runtime-install",
    "global-ontology-runtime",
    "host-code-removal-started",
    "old-host-authority",
    "implementation-command",
  ] {
    assert!(!as_bool(get(runner, key)), "`{key}` must stay false");
  }
  assert_eq!(as_i64(get(runner, "delete-ready-target-count")), 0);
}

#[test]
fn fresh_puck_trials_cover_valid_runner_and_blocked_overclaims() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "fresh-p-puck-trials"));
  assert_eq!(trials.len(), 14);
  assert_eq!(
    as_str(get(
      trials["trial.A.valid-fresh-p-puck-current-cut"],
      "outcome"
    )),
    "fresh-p-puck-current-cut-present"
  );
  assert_eq!(
    as_str(get(
      trials["trial.B.runner-after-fresh-puck-current-cut"],
      "outcome"
    )),
    "runner-ready-for-bounded-replay"
  );
  for (id, held) in [
    (
      "trial.C.missing-p-puck-evidence",
      "held.macro-only-fresh-p-puck.missing-required-evidence",
    ),
    (
      "trial.D.stale-stage",
      "held.macro-only-fresh-p-puck.stale-current-stage",
    ),
    (
      "trial.E.report-mismatch",
      "held.macro-only-fresh-p-puck.report-mismatch",
    ),
    (
      "trial.F.preset-mismatch",
      "held.macro-only-fresh-p-puck.preset-or-runner-mismatch",
    ),
    (
      "trial.G.telemetry-mismatch",
      "held.macro-only-fresh-p-puck.telemetry-mismatch",
    ),
    (
      "trial.H.receipt-mismatch",
      "held.macro-only-fresh-p-puck.current-cut-receipt-mismatch",
    ),
    (
      "trial.I.telemetry-missing",
      "held.macro-only-fresh-p-puck.telemetry-missing",
    ),
    (
      "trial.J.full-audit-overclaim",
      "held.macro-only-fresh-p-puck.full-audit-overclaim",
    ),
    (
      "trial.K.boot-claim",
      "held.macro-only-fresh-p-puck.boot-claim",
    ),
    (
      "trial.L.host-removal-claim",
      "held.macro-only-fresh-p-puck.host-removal-claim",
    ),
    (
      "trial.M.semantic-owner-claim",
      "held.macro-only-fresh-p-puck.semantic-owner-claim",
    ),
    (
      "trial.N.gpl-family-dependency",
      "held.macro-only-fresh-p-puck.gpl-family-dependency",
    ),
  ] {
    assert_eq!(as_str(get(trials[id], "outcome")), "Held");
    assert_eq!(as_str(get(trials[id], "held-id")), held);
  }
}

#[test]
fn six_layer_fold_records_wrapper_proof_and_replay_readiness() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-fresh-p-puck-fold");
  for layer in [
    "surface", "ontology", "semantic", "gate", "runtime", "audit",
  ] {
    assert!(
      as_bool(get(get(fold, layer), "visible")),
      "layer `{layer}` invisible"
    );
  }
  assert_eq!(
    as_str(get_path(fold, &["surface", "audited-receipt"])),
    "fixtures/tesseract-macro-legacy-probe/macro_only_target_delete_proof_receipt.px"
  );
  assert!(as_bool(get_path(
    fold,
    &["ontology", "fresh-p-puck-after-current-cut"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["ontology", "full-current-receipt-audit"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "p-puck-is-wrapper-proof-not-semantic-owner"]
  )));
  assert_eq!(
    as_i64(get_path(
      fold,
      &["gate", "runner-missing-after-fresh-puck-count"]
    )),
    0
  );
  assert!(as_bool(get_path(
    fold,
    &["runtime", "ready-for-bounded-replay"]
  )));
  assert!(!as_bool(get_path(fold, &["runtime", "boot-executed"])));
}

#[test]
fn migration_delta_closes_fresh_puck_and_opens_bounded_replay_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let delta = get(&run, "migration-delta");
  let closes = string_set(get(delta, "closes"));
  assert!(closes.contains("need.bootstrap.fresh-p-puck-after-current-cut"));
  assert!(closes.contains("need.bootstrap.runner-ready-for-bounded-replay"));
  let not = string_set(get(delta, "does-not-close"));
  for expected in [
    "need.bootstrap.full-current-receipt-audit",
    "need.bootstrap.bounded-replay-execution-proof",
    "need.bootstrap.macro-only-boot-execution-proof",
    "need.bootstrap.macro-only-runtime-owner-boot",
    "need.bootstrap.new-engine-from-zero-proof",
    "need.host-removal.host-code-removal-execution",
  ] {
    assert!(not.contains(expected), "missing non-close `{expected}`");
  }
  let next = string_set(get(delta, "next-required"));
  assert!(next.contains("bounded-replay-execution-proof-after-runner-ready"));
}

#[test]
fn discoveries_record_d426_through_d434() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 9);
  for expected in [
    "D426.fresh-p-puck-current-cut-is-bounded-wrapper-proof",
    "D427.p-puck-wrapper-proof-is-not-semantic-owner",
    "D428.current-cut-puck-proof-is-not-full-receipt-audit",
    "D429.p-puck-telemetry-becomes-speed-baseline",
    "D430.fresh-puck-removes-final-runner-held-item",
    "D431.runner-ready-for-bounded-replay-is-not-boot",
    "D432.fresh-puck-cannot-manufacture-host-removal",
    "D433.fresh-puck-cannot-manufacture-replay-boot-runtime-or-new-engine",
    "D434.next-frontier-is-bounded-replay-execution",
  ] {
    let d = discoveries
      .get(expected)
      .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
    assert_eq!(as_str(get(d, "decision-pressure")), "keep");
    assert!(as_bool(get(d, "scenario-only")));
  }
}

#[test]
fn inherited_status_records_runner_transition_from_held_to_ready() {
  let run = eval_file(&fixture_path()).unwrap();
  let inherited = get(&run, "inherited-status");
  assert_eq!(
    as_str(get(inherited, "runner-after-target-proof-status")),
    "Held"
  );
  assert_eq!(
    as_i64(get(inherited, "runner-after-target-proof-missing-count")),
    1
  );
  assert_eq!(
    as_str(get(inherited, "runner-after-fresh-puck-status")),
    "runner-ready-for-bounded-replay"
  );
  assert_eq!(
    as_i64(get(inherited, "runner-after-fresh-puck-missing-count")),
    0
  );
}

#[test]
fn negative_held_evidence_blocks_fresh_puck_shortcuts() {
  let run = eval_file(&fixture_path()).unwrap();
  let evidence = get(&run, "negative-held-evidence");
  assert_eq!(as_str(get(evidence, "status")), "present");
  let rejects = string_set(get(evidence, "rejects"));
  for expected in [
    "fresh-puck-as-full-current-receipt-audit",
    "fresh-puck-as-replay-executed",
    "fresh-puck-as-boot-executed",
    "fresh-puck-as-runtime-owner",
    "fresh-puck-as-new-engine-from-zero",
    "fresh-puck-as-host-removal-started",
    "fresh-puck-as-delete-ready-targets",
    "fresh-puck-as-semantic-owner",
    "fresh-puck-with-gpl-family-dependency",
  ] {
    assert!(rejects.contains(expected), "missing reject `{expected}`");
  }
}

#[test]
fn top_level_state_records_replay_ready_without_boot_or_delete() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "replacement-readiness")),
    "macro-only-runner-ready-for-bounded-replay"
  );
  assert!(as_bool(get(&run, "fresh-p-puck-after-current-cut")));
  assert!(!as_bool(get(&run, "full-current-receipt-audit")));
  assert!(as_bool(get(&run, "ready-for-bounded-replay")));
  assert!(!as_bool(get(&run, "replay-executed")));
  assert!(!as_bool(get(&run, "boot-executed")));
  assert!(!as_bool(get(&run, "macro-only-runtime-owner-booted")));
  assert!(!as_bool(get(&run, "new-engine-from-zero")));
  assert!(!as_bool(get(&run, "runtime-install")));
  assert!(!as_bool(get(&run, "global-ontology-runtime")));
  assert!(!as_bool(get(&run, "host-code-removal-started")));
  assert!(!as_bool(get(&run, "host-removal-safe")));
  assert_eq!(as_i64(get(&run, "delete-ready-target-count")), 0);
  assert!(!as_bool(get(&run, "gpl-family-dependencies")));
  assert_eq!(as_i64(get(&run, "external-solver-dependency-count")), 0);
}
