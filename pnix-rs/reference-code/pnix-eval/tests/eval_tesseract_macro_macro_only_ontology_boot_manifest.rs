//! Macro-only ontology boot manifest for the tesseract macro migration.
//!
//! This pins the next slice after the host-code removal map: the boot
//! composition manifest can be written, but boot execution, new-engine-from-zero,
//! and old host deletion remain unproven.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../..")
}

fn fixture_path() -> PathBuf {
  repo_root()
    .join("fixtures/tesseract-macro-legacy-probe/macro_only_ontology_boot_manifest_receipt.px")
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

fn list_strings(v: &Value) -> Vec<&str> {
  as_list(v).iter().map(as_str).collect()
}

fn string_set(v: &Value) -> BTreeSet<&str> {
  list_strings(v).into_iter().collect()
}

fn attrs_by_id<'a>(items: &'a Value) -> BTreeMap<&'a str, &'a Value> {
  as_list(items)
    .iter()
    .map(|item| (as_str(get(item, "id")), item))
    .collect()
}

fn attrs_by_path<'a>(items: &'a Value) -> BTreeMap<&'a str, &'a Value> {
  as_list(items)
    .iter()
    .map(|item| (as_str(get(item, "path")), item))
    .collect()
}

#[test]
fn macro_only_boot_manifest_marker_and_constitution_owner_are_pinned() {
  let run = eval_file(&fixture_path()).expect("macro-only boot manifest must evaluate");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-macro-only-boot-manifest"
  );
  assert_eq!(
    as_str(get(&run, "truth-owner")),
    "project-wiki/maps/tesseract-macro-ontology-discovery-ledger.md"
  );
  assert_eq!(
    as_str(get(&run, "replacement-map")),
    "project-wiki/maps/tesseract-macro-ontology-replacement-map.md"
  );
  assert_eq!(
    as_str(get(&run, "migration-map")),
    "project-wiki/maps/tesseract-macro-ontology-migration-algorithm-map.md"
  );
  assert_eq!(
    as_str(get(&run, "constitution-owner")),
    "stdlib/lib/gate/tesseract-constitution.px"
  );
}

#[test]
fn constitution_gate_blocks_boot_manifest_overclaims() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "macro-only-ontology-boot-manifest"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));

  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "manifest-written-equals-macro-only-runtime-booted",
    "old-stdlib-ontology-px-is-boot-authority",
    "old-ssa-builtins-are-boot-authority",
    "old-ir-dispatch-is-boot-authority",
    "pnix-core-ontology-rs-is-current-ontology-authority",
    "host-removal-map-equals-host-delete-proof",
    "evaluate-select-scoped-adapter-equals-global-runtime",
    "lift-query-emit-r7-compat-equals-query-runtime",
    "stale-p-puck-audit-equals-current-boot-proof",
    "llm-prose-equals-boot-execution",
  ] {
    assert!(blocks.contains(expected), "missing gate block `{expected}`");
  }
}

#[test]
fn manifest_paths_exist_and_are_not_old_host_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  let root = repo_root();
  for row in as_list(get(&run, "boot-surface-manifest")) {
    let path = as_str(get(row, "path"));
    assert!(
      root.join(path).is_file(),
      "manifest path `{path}` must exist"
    );
    assert!(as_bool(get(row, "required-for-manifest")));
    assert!(!as_bool(get(row, "old-host-authority")));
  }
  for row in as_list(get(&run, "excluded-host-authority")) {
    let path = as_str(get(row, "path"));
    assert!(
      root.join(path).is_file(),
      "excluded host path `{path}` must exist"
    );
    assert!(!as_bool(get(row, "current-semantic-authority")));
    assert!(as_bool(get(row, "may-feed-regression")));
    assert!(!as_bool(get(row, "delete-ready")));
  }
}

#[test]
fn boot_surface_manifest_classifies_macro_stage_surfaces_without_global_runtime() {
  let run = eval_file(&fixture_path()).unwrap();
  let rows = attrs_by_id(get(&run, "boot-surface-manifest"));
  assert_eq!(rows.len(), 8);

  let constitution = rows.get("boot.surface.constitution-gate-owner").unwrap();
  assert_eq!(
    as_str(get(constitution, "owner")),
    "stdlib/lib/gate/tesseract-constitution.px"
  );
  assert!(!as_bool(get(constitution, "runtime-install")));

  let promote = rows.get("boot.surface.promote-r7-owner").unwrap();
  assert_eq!(as_str(get(promote, "phase")), "R7");
  assert_eq!(
    as_str(get(promote, "source-receipt")),
    "tesseract-macro-ontology-r7-compat-archive-promote-surface"
  );
  assert!(as_bool(get(promote, "owner-switch")));
  assert!(!as_bool(get(promote, "global-runtime-install")));

  let ranking = rows
    .get("boot.surface.evaluate-select-ranking-owner")
    .expect("ranking owner row");
  assert_eq!(
    as_str(get(ranking, "owner")),
    "stdlib.lib.gate.evaluate-select-ranking"
  );
  assert_eq!(as_str(get(ranking, "constructor")), "selectWinner");
  assert!(as_bool(get(ranking, "owner-surface-present")));

  let adapter_owner = rows
    .get("boot.surface.evaluate-select-route-adapter-owner")
    .expect("route adapter owner row");
  assert_eq!(
    as_str(get(adapter_owner, "owner")),
    "stdlib.lib.gate.evaluate-select-route-adapter"
  );
  assert_eq!(
    as_str(get(adapter_owner, "effect-scope")),
    "legacy-evaluate-select-surface-pair-only"
  );

  let adapter_install = rows
    .get("boot.surface.evaluate-select-scoped-adapter-install")
    .expect("adapter install row");
  assert!(as_bool(get(
    adapter_install,
    "surface-pair-runtime-adapter-install"
  )));
  assert!(as_bool(get(
    adapter_install,
    "runtime-adapter-install-enabled"
  )));
  assert!(!as_bool(get(adapter_install, "runtime-install")));
  assert!(!as_bool(get(adapter_install, "global-runtime-install")));

  let lqe = rows.get("boot.surface.lift-query-emit-r7-compat").unwrap();
  assert_eq!(as_str(get(lqe, "phase")), "R7");
  assert!(as_bool(get(lqe, "owner-switch")));
  assert!(!as_bool(get(lqe, "query-runtime-install")));
  assert!(!as_bool(get(lqe, "fact-store-install")));
  assert!(!as_bool(get(lqe, "audit-event-log-install")));
  assert!(!as_bool(get(lqe, "expression-projection-owner")));

  let host_map = rows.get("boot.surface.host-code-removal-map").unwrap();
  assert!(as_bool(get(host_map, "host-removal-map-written")));
  assert!(!as_bool(get(host_map, "host-code-removal-started")));
  assert_eq!(as_i64(get(host_map, "delete-ready-target-count")), 0);
}

#[test]
fn excluded_old_host_surfaces_remain_regression_inputs_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let excluded = attrs_by_path(get(&run, "excluded-host-authority"));
  assert_eq!(excluded.len(), 5);
  for path in [
    "stdlib/lib/ontology.px",
    "crates/pnix-runtime-legacy/src/ssa_eval/builtins/mod.rs",
    "crates/pnix-runtime-legacy/src/ir/eval.rs",
    "crates/pnix-core/src/ontology.rs",
    "crates/pnix-eval/tests/ontology_builtins.rs",
  ] {
    let row = excluded
      .get(path)
      .unwrap_or_else(|| panic!("missing `{path}`"));
    assert!(!as_bool(get(row, "current-semantic-authority")));
    assert!(as_bool(get(row, "may-feed-regression")));
    assert!(!as_bool(get(row, "delete-ready")));
    assert!(!as_str(get(row, "retained-role")).is_empty());
  }
}

#[test]
fn boot_readiness_gate_closes_manifest_but_keeps_execution_held() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "boot-readiness-gate");
  assert_eq!(
    as_str(get(gate, "id")),
    "gate.macro-only-ontology-boot.readiness.v1"
  );
  assert!(as_bool(get(gate, "manifest-written")));
  assert!(as_bool(get(gate, "manifest-complete")));
  assert_eq!(as_str(get(gate, "verdict")), "manifest-written-boot-held");
  for key in [
    "boot-executed",
    "macro-only-runtime-owner-booted",
    "new-engine-from-zero",
    "old-host-authority",
    "host-code-removal-started",
    "target-specific-delete-proof-present",
    "fresh-p-puck-after-current-cut",
    "compare-after-boot",
    "regression-corpus-transfer-present",
    "lift-query-emit-runtime-owner-present",
  ] {
    assert!(!as_bool(get(gate, key)), "`{key}` must stay false");
  }
  assert!(as_bool(get(gate, "old-host-code-still-present")));
  assert_eq!(as_i64(get(gate, "delete-ready-target-count")), 0);

  let before_boot = string_set(get(gate, "required-before-boot-claim"));
  for expected in [
    "macro-only-boot-execution-proof",
    "fresh-p-puck-after-current-cut",
    "compare-after-boot",
    "regression-corpus-transfer-or-retention-proof",
    "lift-query-emit-runtime-owner-or-explicit-compat-bound",
    "host-target-authority-scan",
    "bootstrap-status-audit-update-after-boot",
  ] {
    assert!(
      before_boot.contains(expected),
      "missing boot prerequisite `{expected}`"
    );
  }
}

#[test]
fn boot_trials_hold_bad_boot_claims_and_accept_manifest_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "boot-manifest-trials"));
  assert_eq!(trials.len(), 6);
  for expected in [
    "trial.A.constitution-owner-missing",
    "trial.B.old-host-authority-included",
    "trial.C.host-removal-map-missing",
    "trial.D.stale-p-puck-boot-claim",
    "trial.E.manifest-as-runtime-boot",
  ] {
    let trial = trials
      .get(expected)
      .unwrap_or_else(|| panic!("missing trial `{expected}`"));
    assert_eq!(as_str(get(trial, "outcome")), "Held");
    assert!(!as_bool(get(trial, "boot-executed")));
  }

  let complete = trials.get("trial.F.complete-manifest").unwrap();
  assert_eq!(
    as_str(get(complete, "outcome")),
    "manifest-written-not-booted"
  );
  assert_eq!(as_str(get(complete, "held-id")), "none");
  assert!(as_bool(get(complete, "manifest-written")));
  assert!(!as_bool(get(complete, "boot-executed")));
}

#[test]
fn six_layer_fold_keeps_manifest_execution_and_host_deletion_separate() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-macro-only-boot-fold");
  assert_eq!(
    as_str(get(fold, "mode")),
    "macro-only-ontology-boot-manifest"
  );
  for layer in [
    "surface", "ontology", "semantic", "gate", "runtime", "audit",
  ] {
    assert!(
      as_bool(get_path(fold, &[layer, "visible"])),
      "layer `{layer}` must stay visible"
    );
  }
  assert_eq!(
    as_i64(get_path(fold, &["surface", "boot-surface-count"])),
    8
  );
  assert_eq!(
    as_i64(get_path(
      fold,
      &["surface", "excluded-host-authority-count"]
    )),
    5
  );
  assert!(as_bool(get_path(fold, &["surface", "manifest-written"])));
  assert!(as_bool(get_path(
    fold,
    &["ontology", "macro-stage-surfaces-present"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["ontology", "old-host-authority"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["ontology", "old-host-code-still-present"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["ontology", "new-engine-from-zero"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "boot-composition-known"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["semantic", "boot-execution-proven"]
  )));
  assert_eq!(
    as_str(get_path(fold, &["gate", "boot-readiness-verdict"])),
    "manifest-written-boot-held"
  );
  assert!(!as_bool(get_path(fold, &["runtime", "boot-executed"])));
  assert!(!as_bool(get_path(
    fold,
    &["runtime", "macro-only-runtime-owner-booted"]
  )));
  assert!(!as_bool(get_path(fold, &["runtime", "runtime-install"])));
  assert!(as_bool(get_path(fold, &["audit", "p-puck-fresh-required"])));
}

#[test]
fn migration_delta_closes_manifest_need_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let delta = get(&run, "migration-delta");
  assert_eq!(
    as_str(get(delta, "id")),
    "migration-delta.macro-only-ontology-boot-manifest"
  );
  let closes = string_set(get(delta, "closes"));
  assert!(closes.contains("need.bootstrap.macro-only-ontology-boot-manifest"));

  let not = string_set(get(delta, "does-not-close"));
  for expected in [
    "need.bootstrap.macro-only-runtime-owner-boot",
    "need.bootstrap.new-engine-from-zero-proof",
    "need.host-removal.target-specific-delete-proof",
    "need.host-removal.fresh-p-puck-after-current-cut",
    "need.host-removal.regression-corpus-transfer",
    "need.lift-query-emit.runtime-owner-or-host-removal",
  ] {
    assert!(not.contains(expected), "missing open frontier `{expected}`");
  }

  let next = string_set(get(delta, "next-required"));
  assert!(next.contains("macro-only-boot-execution-proof"));
  assert!(next.contains("fresh-p-puck-and-compare-after-current-cut"));
  assert!(next.contains("target-specific-host-delete-proof"));
  assert!(next.contains("bootstrap-status-audit-after-boot-attempt"));
}

#[test]
fn discoveries_record_d345_through_d353() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 9);
  for expected in [
    "D345.macro-only-boot-manifest-separates-composition-from-execution",
    "D346.old-host-surfaces-excluded-as-authority-but-retained-as-regression",
    "D347.evaluate-select-scoped-adapter-is-boot-surface-not-global-runtime",
    "D348.lift-query-emit-remains-compat-reference-not-query-runtime",
    "D349.fresh-p-puck-is-required-before-actual-macro-only-boot-claim",
    "D350.macro-only-boot-manifest-does-not-delete-host-code",
    "D351.manifest-opens-target-specific-delete-proofs-without-satisfying-them",
    "D352.boot-surfaces-are-px-owner-candidate-surfaces-not-old-builtin-authority",
    "D353.new-engine-from-zero-remains-false-until-boot-execution-replay-proves-it",
  ] {
    let discovery = discoveries
      .get(expected)
      .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
    assert!(as_bool(get(discovery, "scenario-only")));
  }
}

#[test]
fn inherited_status_keeps_host_map_and_internal_capability_boundaries() {
  let run = eval_file(&fixture_path()).unwrap();
  let status = get(&run, "inherited-status");
  assert_eq!(
    as_str(get(status, "host-removal-map")),
    "tesseract-macro-ontology-host-code-removal-map"
  );
  assert!(as_bool(get(status, "host-removal-map-written")));
  assert!(!as_bool(get(status, "host-code-removal-started")));
  assert_eq!(as_i64(get(status, "host-delete-ready-target-count")), 0);
  assert_eq!(as_str(get(status, "promote-phase")), "R7");
  assert!(as_bool(get(
    status,
    "evaluate-select-scoped-adapter-install"
  )));
  assert_eq!(
    as_str(get(status, "evaluate-select-install-scope")),
    "legacy-evaluate-select-surface-pair-only"
  );
  assert_eq!(as_str(get(status, "lift-query-emit-phase")), "R7");
  assert!(!as_bool(get(
    status,
    "lift-query-emit-runtime-owner-present"
  )));
  assert_eq!(as_i64(get(status, "external-solver-dependency-count")), 0);
}

#[test]
fn top_level_state_is_manifest_written_not_booted() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "replacement-readiness")),
    "macro-only-boot-manifest-written"
  );
  assert!(!as_bool(get(&run, "owner-switch")));
  assert!(as_bool(get(&run, "macro-only-boot-manifest-written")));
  assert!(!as_bool(get(&run, "macro-only-runtime-owner-booted")));
  assert!(!as_bool(get(&run, "new-engine-from-zero")));
  assert!(!as_bool(get(&run, "old-host-authority")));
  assert!(as_bool(get(&run, "old-host-code-still-present")));
  assert!(!as_bool(get(&run, "host-code-removal-started")));
  assert!(!as_bool(get(&run, "host-removal-safe")));
  assert_eq!(as_i64(get(&run, "delete-ready-target-count")), 0);
  assert!(!as_bool(get(&run, "runtime-install")));
  assert!(!as_bool(get(&run, "global-ontology-runtime")));
  assert!(!as_bool(get(&run, "implementation-command")));
  assert_eq!(as_i64(get(&run, "external-solver-dependency-count")), 0);
  assert!(!as_bool(get(&run, "gpl-family-dependencies")));

  let rejects = string_set(get_path(&run, &["negative-held-evidence", "rejects"]));
  assert!(rejects.contains("manifest-written-as-runtime-boot"));
  assert!(rejects.contains("old-stdlib-ontology-as-boot-authority"));
  assert!(rejects.contains("host-removal-map-as-delete-proof"));
  assert!(rejects.contains("stale-p-puck-as-current-boot-proof"));
  assert!(rejects.contains("llm-prose-as-boot-execution"));
}
