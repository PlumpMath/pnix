//! Macro-only bounded replay execution receipt.
//!
//! This pins the first narrow replay execution proof after the fresh p-puck
//! current-cut runner-ready state. The proof executes the bounded replay trace
//! only; boot success, macro-only runtime ownership, host removal, semantic
//! ownership, and full receipt audit remain separate.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../..")
}

fn fixture_path() -> PathBuf {
  repo_root()
    .join("fixtures/tesseract-macro-legacy-probe/macro_only_bounded_replay_execution_receipt.px")
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
  let run = eval_file(&fixture_path()).expect("bounded replay execution receipt");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-macro-only-bounded-replay-execution"
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
    "stdlib/lib/gate/macro-only-boot-bounded-replay-execution.px",
    "fixtures/pnix-query-runtime/macro-only-boot-bounded-replay-execution-owner.px",
    "fixtures/tesseract-macro-legacy-probe/macro_only_bounded_replay_execution_receipt.px",
  ] {
    assert!(repo_root().join(path).is_file(), "missing `{path}`");
  }
}

#[test]
fn constitution_gate_blocks_replay_overclaims() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "macro-only-bounded-replay-execution"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));

  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "runner-ready-equals-replay-executed",
    "bounded-replay-equals-boot-success",
    "bounded-replay-equals-runtime-owner",
    "bounded-replay-equals-new-engine-from-zero",
    "bounded-replay-equals-host-removal",
    "bounded-replay-equals-semantic-owner",
    "bounded-replay-equals-delete-ready-targets",
    "bounded-replay-equals-full-current-receipt-audit",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
}

#[test]
fn contract_closes_bounded_replay_execution_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let contract = get(&run, "bounded-replay-contract");
  assert_eq!(
    as_str(get(contract, "id")),
    "contract.macro-only-bounded-replay-execution.v1"
  );
  assert_eq!(
    as_str(get(contract, "owner")),
    "stdlib.lib.gate.macro-only-boot-bounded-replay-execution"
  );
  assert_eq!(
    as_str(get(contract, "current-status")),
    "bounded-replay-executed"
  );
  assert!(as_bool(get(
    contract,
    "closes-bounded-replay-execution-proof"
  )));
  for key in [
    "closes-boot-execution-proof",
    "closes-macro-only-runtime-owner",
    "closes-new-engine-from-zero",
    "closes-full-current-receipt-audit",
    "closes-host-removal",
    "closes-delete-ready-targets",
    "closes-semantic-owner-proof",
    "runtime-install",
    "global-ontology-runtime",
  ] {
    assert!(!as_bool(get(contract, key)), "`{key}` must stay false");
  }
}

#[test]
fn replay_proof_starts_from_fresh_runner_ready_state() {
  let run = eval_file(&fixture_path()).unwrap();
  let inherited = get(&run, "inherited-status");
  assert_eq!(
    as_str(get(inherited, "fresh-puck")),
    "tesseract-macro-ontology-macro-only-fresh-p-puck-current-cut"
  );
  assert!(as_bool(get(inherited, "fresh-puck-after-current-cut")));
  assert_eq!(
    as_str(get(inherited, "runner-after-fresh-puck-status")),
    "runner-ready-for-bounded-replay"
  );
  assert_eq!(
    as_i64(get(inherited, "runner-after-fresh-puck-missing-count")),
    0
  );
  assert!(as_bool(get(inherited, "runner-ready-for-bounded-replay")));
}

#[test]
fn bounded_replay_proof_executes_trace_but_not_boot() {
  let run = eval_file(&fixture_path()).unwrap();
  let proof = get(&run, "bounded-replay-proof");
  assert_eq!(as_str(get(proof, "status")), "bounded-replay-executed");
  assert!(as_bool(get(proof, "ready-for-bounded-replay-input")));
  assert!(as_bool(get(proof, "bounded-replay-executed")));
  assert!(as_bool(get(proof, "bounded-replay-execution-proof")));
  assert_eq!(as_i64(get(proof, "runner-missing-count")), 0);
  assert_eq!(as_i64(get(proof, "node-count")), 11);
  assert_eq!(as_i64(get(proof, "edge-count")), 14);
  assert_eq!(as_i64(get(proof, "import-depth")), 4);
  assert_eq!(as_i64(get(proof, "max-stack-depth")), 6);
  assert_eq!(
    as_str(get(proof, "semantic-delta-status")),
    "empty-or-held-only"
  );
  for key in [
    "boot-executed",
    "macro-only-runtime-owner-booted",
    "new-engine-from-zero",
    "runtime-install",
    "global-ontology-runtime",
    "host-code-removal-started",
    "host-removal-safe",
    "semantic-owner",
    "old-host-authority",
    "gpl-family-dependencies",
    "implementation-command",
  ] {
    assert!(!as_bool(get(proof, key)), "`{key}` must stay false");
  }
  assert_eq!(as_i64(get(proof, "delete-ready-target-count")), 0);
}

#[test]
fn replay_step_order_is_exact_and_bounded() {
  let run = eval_file(&fixture_path()).unwrap();
  let proof = get(&run, "bounded-replay-proof");
  let steps: Vec<&str> = as_list(get(proof, "replay-step-ids"))
    .iter()
    .map(as_str)
    .collect();
  assert_eq!(
    steps,
    vec![
      "replay.node.constitution-owner",
      "replay.node.promote-r7-compat",
      "replay.node.evaluate-select-ranking-owner",
      "replay.node.evaluate-select-route-adapter-owner",
      "replay.node.evaluate-select-scoped-adapter-install",
      "replay.node.lift-query-emit-r7-compat",
      "replay.node.internal-self-capability-map",
      "replay.node.host-removal-map",
      "replay.node.macro-only-boot-manifest",
      "replay.node.macro-only-boot-attempt",
      "replay.node.macro-only-boot-runner-owner",
    ]
  );
}

#[test]
fn trials_cover_valid_input_and_replay_held_modes() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "bounded-replay-trials"));
  assert_eq!(trials.len(), 11);
  assert_eq!(
    as_str(get(
      trials["trial.A.valid-bounded-replay-execution"],
      "outcome"
    )),
    "bounded-replay-executed"
  );
  assert_eq!(
    as_str(get(trials["trial.B.fresh-runner-input"], "outcome")),
    "runner-ready-for-bounded-replay"
  );
  for (id, held) in [
    (
      "trial.C.runner-not-ready",
      "held.macro-only-bounded-replay.runner-not-ready",
    ),
    (
      "trial.D.schedule-mismatch",
      "held.macro-only-bounded-replay.schedule-mismatch",
    ),
    (
      "trial.E.bound-violation",
      "held.macro-only-bounded-replay.bound-violation",
    ),
    (
      "trial.F.negative-held-lost",
      "held.macro-only-bounded-replay.missing-required-evidence",
    ),
    (
      "trial.G.semantic-delta-overclaim",
      "held.macro-only-bounded-replay.semantic-delta-overclaim",
    ),
    (
      "trial.H.boot-claim",
      "held.macro-only-bounded-replay.boot-or-runtime-claim",
    ),
    (
      "trial.I.host-removal-claim",
      "held.macro-only-bounded-replay.host-removal-claim",
    ),
    (
      "trial.J.semantic-owner-claim",
      "held.macro-only-bounded-replay.semantic-owner-claim",
    ),
    (
      "trial.K.gpl-family-dependency",
      "held.macro-only-bounded-replay.gpl-family-dependency",
    ),
  ] {
    assert_eq!(as_str(get(trials[id], "outcome")), "Held");
    assert_eq!(as_str(get(trials[id], "held-id")), held);
  }
}

#[test]
fn six_layer_fold_records_trace_execution_and_false_runtime_claims() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-bounded-replay-fold");
  for layer in [
    "surface", "ontology", "semantic", "gate", "runtime", "audit",
  ] {
    assert!(
      as_bool(get(get(fold, layer), "visible")),
      "layer `{layer}` invisible"
    );
  }
  assert!(as_bool(get_path(fold, &["ontology", "runner-ready-input"])));
  assert_eq!(
    as_i64(get_path(fold, &["ontology", "runner-missing-count"])),
    0
  );
  assert!(as_bool(get_path(
    fold,
    &["semantic", "replay-executes-trace-not-boot"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["runtime", "bounded-replay-executed"]
  )));
  assert!(!as_bool(get_path(fold, &["runtime", "boot-executed"])));
  assert!(!as_bool(get_path(
    fold,
    &["runtime", "host-code-removal-started"]
  )));
  assert_eq!(as_i64(get_path(fold, &["audit", "replay-step-count"])), 11);
  assert!(!as_bool(get_path(
    fold,
    &["audit", "full-current-receipt-audit"]
  )));
}

#[test]
fn migration_delta_closes_replay_and_opens_boot_frontier() {
  let run = eval_file(&fixture_path()).unwrap();
  let delta = get(&run, "migration-delta");
  let closes = string_set(get(delta, "closes"));
  assert!(closes.contains("need.bootstrap.bounded-replay-execution-proof"));
  let not = string_set(get(delta, "does-not-close"));
  for expected in [
    "need.bootstrap.macro-only-boot-execution-proof",
    "need.bootstrap.macro-only-runtime-owner-boot",
    "need.bootstrap.new-engine-from-zero-proof",
    "need.bootstrap.full-current-receipt-audit",
    "need.host-removal.host-code-removal-execution",
    "need.host-removal.delete-ready-targets",
    "need.semantic-owner.macro-ontology-runtime",
  ] {
    assert!(not.contains(expected), "missing non-close `{expected}`");
  }
  let next = string_set(get(delta, "next-required"));
  assert!(next.contains("macro-only-boot-execution-proof-after-bounded-replay"));
  assert!(next.contains("full-current-receipt-audit-after-bounded-replay"));
}

#[test]
fn discoveries_record_d435_through_d443() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 9);
  for expected in [
    "D435.bounded-replay-execution-starts-only-after-runner-ready",
    "D436.bounded-replay-executes-exact-strategy-node-order",
    "D437.replay-execution-is-trace-proof-not-boot-proof",
    "D438.replay-preserves-negative-held-evidence",
    "D439.replay-semantic-delta-is-empty-or-held-only",
    "D440.replay-bounds-are-node-edge-depth-stack-bounds",
    "D441.bounded-replay-cannot-manufacture-host-removal",
    "D442.bounded-replay-cannot-manufacture-runtime-or-new-engine",
    "D443.next-frontier-is-macro-only-boot-execution-proof",
  ] {
    let d = discoveries
      .get(expected)
      .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
    assert!(as_bool(get(d, "scenario-only")));
    assert_eq!(as_str(get(d, "decision-pressure")), "keep");
  }
}

#[test]
fn top_level_status_keeps_boot_runtime_host_and_semantic_false() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "replacement-readiness")),
    "macro-only-bounded-replay-executed-not-booted"
  );
  assert!(as_bool(get(&run, "ready-for-bounded-replay")));
  assert!(as_bool(get(&run, "bounded-replay-executed")));
  assert!(as_bool(get(&run, "bounded-replay-execution-proof")));
  for key in [
    "owner-switch",
    "implementation-command",
    "boot-executed",
    "macro-only-runtime-owner-booted",
    "new-engine-from-zero",
    "runtime-install",
    "global-ontology-runtime",
    "host-code-removal-started",
    "host-removal-safe",
    "semantic-owner",
    "full-current-receipt-audit",
    "gpl-family-dependencies",
  ] {
    assert!(!as_bool(get(&run, key)), "`{key}` must stay false");
  }
  assert_eq!(as_i64(get(&run, "delete-ready-target-count")), 0);
  assert_eq!(as_i64(get(&run, "external-solver-dependency-count")), 0);
}

#[test]
fn negative_held_evidence_keeps_replay_shortcuts_rejectable() {
  let run = eval_file(&fixture_path()).unwrap();
  let evidence = get(&run, "negative-held-evidence");
  assert_eq!(as_str(get(evidence, "status")), "present");
  let rejects = string_set(get(evidence, "rejects"));
  for expected in [
    "bounded-replay-before-runner-ready",
    "bounded-replay-schedule-reordered",
    "bounded-replay-bound-violation",
    "bounded-replay-loses-negative-held",
    "bounded-replay-as-boot-success",
    "bounded-replay-as-runtime-owner",
    "bounded-replay-as-host-removal",
    "bounded-replay-as-semantic-owner",
    "bounded-replay-with-gpl-family-dependency",
  ] {
    assert!(rejects.contains(expected), "missing reject `{expected}`");
  }
}
