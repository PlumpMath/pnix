//! Host-removal fresh delete p-puck current-cut.
//!
//! This pins the p-puck current-cut proof over the delete patch candidate:
//! freshness can close, but delete-ready and implementation command stay false.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../..")
}

fn fixture_path() -> PathBuf {
  repo_root().join(
    "fixtures/tesseract-macro-legacy-probe/macro_only_host_removal_fresh_delete_p_puck_current_cut_receipt.px",
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
  let run = eval_file(&fixture_path()).expect("fresh delete p-puck receipt");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-macro-only-host-removal-fresh-delete-p-puck-current-cut"
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
    "stdlib/lib/gate/macro-only-host-removal-fresh-delete-p-puck-current-cut.px",
    "fixtures/pnix-query-runtime/macro-only-host-removal-fresh-delete-p-puck-current-cut-owner.px",
    "fixtures/tesseract-macro-legacy-probe/macro_only_host_removal_fresh_delete_p_puck_current_cut_receipt.px",
  ] {
    assert!(repo_root().join(path).is_file(), "missing `{path}`");
  }
}

#[test]
fn constitution_gate_blocks_fresh_delete_collapse() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitutionGate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "macro-only-host-removal-fresh-delete-p-puck-current-cut"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "fresh-delete-puck-equals-delete-ready",
    "fresh-delete-puck-equals-remove-now",
    "fresh-delete-puck-equals-host-code-removal",
    "fresh-delete-puck-equals-implementation-command",
    "fresh-delete-puck-equals-global-runtime-install",
    "fresh-delete-puck-equals-runtime-api-flattening",
    "fresh-delete-puck-equals-meaning-db",
    "within-threshold-equals-delete-ready",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
}

#[test]
fn contract_closes_freshness_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let contract = get(&run, "host-removal-fresh-delete-puck-contract");
  assert_eq!(
    as_str(get(contract, "id")),
    "contract.macro-only-host-removal-fresh-delete-p-puck-current-cut.v1"
  );
  assert_eq!(
    as_str(get(contract, "current-status")),
    "host-removal-fresh-delete-p-puck-current-cut-present"
  );
  assert_eq!(as_i64(get(contract, "duration-ms")), 1318);
  assert_eq!(as_i64(get(contract, "total-tests")), 1053);
  assert_eq!(as_i64(get(contract, "source-tracked")), 18207);
  assert_eq!(as_i64(get(contract, "source-indexed")), 18207);
  assert!(as_bool(get(contract, "closes-fresh-puck-before-delete")));
  for key in [
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
fn proof_records_fresh_delete_puck_without_delete_ready() {
  let run = eval_file(&fixture_path()).unwrap();
  let proof = get(&run, "host-removal-fresh-delete-puck-proof");
  assert_eq!(
    as_str(get(proof, "status")),
    "host-removal-fresh-delete-p-puck-current-cut-present"
  );
  assert!(as_bool(get(proof, "fresh-puck-before-delete")));
  assert!(as_bool(get(
    proof,
    "fresh-puck-before-delete-as-delete-ready-frontier-closed"
  )));
  assert_eq!(as_i64(get(proof, "duration-ms")), 1318);
  assert_eq!(as_str(get(proof, "slow-path-status")), "within-threshold");
  assert!(!as_bool(get(proof, "slow-path-candidate")));
  assert!(!as_bool(get(proof, "self-optimization-candidate")));
  assert_eq!(as_i64(get(proof, "delete-ready-target-count")), 0);
  for key in [
    "actual-host-removal-patch-authorized",
    "delete-ready",
    "remove-now",
    "host-code-removal-started",
    "implementation-command",
    "runtime-api-flattening",
    "meaning-db",
    "global-ontology-runtime",
    "p-puck-is-semantic-owner",
  ] {
    assert!(!as_bool(get(proof, key)), "`{key}` must stay false");
  }
}

#[test]
fn trials_cover_freshness_and_all_held_boundaries() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "host-removal-fresh-delete-puck-trials"));
  assert_eq!(trials.len(), 15);
  assert_eq!(
    as_str(get(
      trials["trial.A.valid-fresh-delete-puck-current-cut"],
      "outcome"
    )),
    "host-removal-fresh-delete-p-puck-current-cut-present"
  );
  assert_eq!(
    as_str(get(trials["trial.B.delete-candidate-input"], "outcome")),
    "tesseract-macro-ontology-macro-only-host-removal-delete-patch-candidate"
  );
  for (id, held) in [
    (
      "trial.C.wrong-proof-id",
      "held.macro-only-host-removal-fresh-delete-puck.proof-id-mismatch",
    ),
    (
      "trial.D.stale-stage",
      "held.macro-only-host-removal-fresh-delete-puck.stale-current-stage",
    ),
    (
      "trial.E.delete-candidate-missing",
      "held.macro-only-host-removal-fresh-delete-puck.delete-candidate-missing",
    ),
    (
      "trial.F.report-mismatch",
      "held.macro-only-host-removal-fresh-delete-puck.report-mismatch",
    ),
    (
      "trial.G.receipt-mismatch",
      "held.macro-only-host-removal-fresh-delete-puck.current-cut-receipt-mismatch",
    ),
    (
      "trial.H.telemetry-drift",
      "held.macro-only-host-removal-fresh-delete-puck.telemetry-missing-or-drifted",
    ),
    (
      "trial.I.compare-mismatch",
      "held.macro-only-host-removal-fresh-delete-puck.compare-all-mismatch",
    ),
    (
      "trial.J.source-mismatch",
      "held.macro-only-host-removal-fresh-delete-puck.source-parity-mismatch",
    ),
    (
      "trial.K.delete-overclaim",
      "held.macro-only-host-removal-fresh-delete-puck.delete-overclaim",
    ),
    (
      "trial.L.runtime-overclaim",
      "held.macro-only-host-removal-fresh-delete-puck.runtime-overclaim",
    ),
    (
      "trial.M.p-puck-semantic-owner",
      "held.macro-only-host-removal-fresh-delete-puck.p-puck-semantic-owner",
    ),
    (
      "trial.N.old-host-authority",
      "held.macro-only-host-removal-fresh-delete-puck.old-host-authority",
    ),
    (
      "trial.O.gpl-family-dependency",
      "held.macro-only-host-removal-fresh-delete-puck.gpl-family-dependency",
    ),
  ] {
    assert_eq!(as_str(get(trials[id], "outcome")), "Held");
    assert_eq!(as_str(get(trials[id], "held-id")), held);
  }
}

#[test]
fn six_layer_fold_records_fresh_cut_without_runtime_or_db() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-host-removal-fresh-delete-puck-fold");
  for layer in [
    "surface", "ontology", "semantic", "gate", "runtime", "audit",
  ] {
    assert!(
      as_bool(get(get(fold, layer), "visible")),
      "layer `{layer}` hidden"
    );
  }
  assert!(as_bool(get_path(
    fold,
    &["ontology", "delete-patch-candidate-input"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "within-threshold-is-not-delete-ready"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "runtime-api-flattening-deferred"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "meaning-db-deferred"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["runtime", "fresh-puck-before-delete"]
  )));
  assert!(!as_bool(get_path(fold, &["runtime", "delete-ready"])));
  assert_eq!(
    as_i64(get_path(fold, &["runtime", "delete-ready-target-count"])),
    0
  );
  assert!(!as_bool(get_path(
    fold,
    &["runtime", "implementation-command"]
  )));
  assert_eq!(as_i64(get_path(fold, &["audit", "duration-ms"])), 1318);
}

#[test]
fn migration_delta_closes_fresh_puck_not_delete_ready() {
  let run = eval_file(&fixture_path()).unwrap();
  let delta = get(&run, "migrationDelta");
  let closes = string_set(get(delta, "closes"));
  assert!(closes.contains("need.host-removal.fresh-puck-before-delete-as-delete-ready"));
  let not = string_set(get(delta, "does-not-close"));
  assert!(not.contains("need.host-removal.delete-ready-targets"));
  assert!(not.contains("need.host-removal.actual-host-removal-implementation-command"));
  assert!(not.contains("need.domain-runtime-api-flattening-after-semantic-owner"));
  assert!(not.contains("need.stdlib.meaning-db"));
  let next = string_set(get(delta, "next-required"));
  assert!(next.contains("delete-ready-targets-after-fresh-delete-puck"));
  assert!(next.contains("actual-host-removal-implementation-command"));
}

#[test]
fn discoveries_record_d516_through_d523() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 8);
  for expected in [
    "D516.fresh-delete-puck-must-follow-delete-patch-candidate",
    "D517.delete-candidate-puck-current-cut-is-per-receipt",
    "D518.within-threshold-delete-candidate-puck-closes-freshness-only",
    "D519.fresh-delete-puck-and-delete-ready-are-separate-states",
    "D520.fresh-delete-puck-does-not-emit-implementation-command",
    "D521.fresh-delete-puck-keeps-runtime-flattening-and-meaning-db-deferred",
    "D522.p-puck-wrapper-remains-non-semantic-owner-on-delete-cut",
    "D523.host-removal-frontier-shifts-to-delete-ready-target-proof",
  ] {
    assert!(discoveries.contains_key(expected), "missing `{expected}`");
  }
}

#[test]
fn top_level_state_is_fresh_but_still_non_delete() {
  let run = eval_file(&fixture_path()).unwrap();
  assert!(as_bool(get(
    &run,
    "host-removal-fresh-delete-p-puck-current-cut"
  )));
  assert!(as_bool(get(&run, "fresh-puck-before-delete")));
  assert!(as_bool(get(
    &run,
    "fresh-puck-before-delete-as-delete-ready-frontier-closed"
  )));
  assert!(as_bool(get(&run, "actual-host-removal-patch-candidate")));
  assert!(!as_bool(get(&run, "actual-host-removal-patch-authorized")));
  assert!(!as_bool(get(&run, "delete-ready")));
  assert_eq!(as_i64(get(&run, "delete-ready-target-count")), 0);
  assert!(!as_bool(get(&run, "remove-now")));
  assert!(!as_bool(get(&run, "host-code-removal-started")));
  assert!(!as_bool(get(&run, "runtime-api-flattening")));
  assert!(!as_bool(get(&run, "meaning-db")));
  assert!(!as_bool(get(&run, "implementation-command")));
  assert_eq!(
    as_str(get(&run, "replacement-readiness")),
    "host-removal-fresh-delete-puck-current-cut-present-not-delete-ready"
  );
}
