//! Host-code removal map for the tesseract macro ontology migration.
//!
//! This test pins the next bootstrap slice after R7 lift/query/emit compat:
//! map the old ontology host/code surfaces to specimen, compat, scoped-adapter,
//! pure-oracle, and regression-corpus roles. It does not delete anything.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../..")
}

fn fixture_path() -> PathBuf {
  repo_root().join("fixtures/tesseract-macro-legacy-probe/host_code_removal_map_receipt.px")
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

fn maybe_get<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
  as_attrs(v).get(key)
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
fn host_code_removal_map_marker_and_constitution_owner_are_pinned() {
  let run = eval_file(&fixture_path()).expect("host removal map fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-host-code-removal-map"
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
fn constitution_gate_blocks_host_deletion_shortcuts() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(as_str(get(gate, "scenario")), "host-code-removal-map");
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));

  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "remove-stdlib-ontology-px-after-matrix-green",
    "remove-ssa-ontology-builtins-after-owner-switch",
    "remove-ir-ontology-builtins-after-scoped-adapter",
    "remove-pnix-core-ontology-rs-before-macro-core-replacement",
    "delete-ontology-builtins-tests-as-clutter",
    "host-removal-map-equals-delete-proof",
    "macro-receipt-eval-equals-macro-only-boot",
    "p-puck-stale-green-equals-current-cut-proof",
    "llm-cleanup-prose-equals-removal-proof",
  ] {
    assert!(
      blocks.contains(expected),
      "missing shortcut block `{expected}`"
    );
  }
}

#[test]
fn mapped_host_paths_exist_and_observed_symbols_are_real() {
  let run = eval_file(&fixture_path()).unwrap();
  let root = repo_root();
  for target in as_list(get(&run, "host-path-targets")) {
    let path = as_str(get(target, "path"));
    let mut content = std::fs::read_to_string(root.join(path))
      .unwrap_or_else(|err| panic!("failed to read mapped host path `{path}`: {err}"));
    if let Some(companion) = maybe_get(target, "companion-path") {
      let companion_path = as_str(companion);
      let companion_content =
        std::fs::read_to_string(root.join(companion_path)).unwrap_or_else(|err| {
          panic!("failed to read companion host path `{companion_path}`: {err}")
        });
      content.push('\n');
      content.push_str(&companion_content);
    }

    for symbol in list_strings(get(target, "observed-symbols")) {
      assert!(
        content.contains(symbol),
        "mapped host path `{path}` does not contain observed symbol `{symbol}`"
      );
    }
  }
}

#[test]
fn surface_groups_inherit_current_migration_phases_without_delete_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  let groups = attrs_by_id(get(&run, "surface-groups"));
  assert_eq!(groups.len(), 4);

  let promote = groups.get("surface-group.promote").unwrap();
  assert_eq!(as_str(get(promote, "current-phase")), "R7");
  assert_eq!(
    as_str(get(promote, "source-receipt")),
    "tesseract-macro-ontology-r7-compat-archive-promote-surface"
  );
  assert_eq!(as_str(get(promote, "host-removal-verdict")), "Held");
  assert!(as_bool(get(promote, "owner-switch")));
  assert!(!as_bool(get(promote, "old-host-authority")));

  let eval_select = groups.get("surface-group.evaluate-select").unwrap();
  assert_eq!(
    as_str(get(eval_select, "current-phase")),
    "scoped-runtime-adapter-install"
  );
  assert!(as_bool(get(eval_select, "scoped-adapter-installed")));
  assert!(!as_bool(get(eval_select, "global-runtime-install")));
  assert_eq!(as_str(get(eval_select, "host-removal-verdict")), "Held");

  let lqe = groups.get("surface-group.lift-query-emit").unwrap();
  assert_eq!(as_str(get(lqe, "current-phase")), "R7");
  assert_eq!(
    as_str(get(lqe, "source-receipt")),
    "tesseract-macro-ontology-r7-compat-archive-lift-query-emit-surface-triple"
  );
  assert!(as_bool(get(lqe, "owner-switch")));
  assert!(!as_bool(get(lqe, "query-runtime-install")));
  assert!(!as_bool(get(lqe, "fact-store-install")));
  assert!(!as_bool(get(lqe, "audit-event-log-install")));
  assert!(!as_bool(get(lqe, "expression-projection-owner")));
  assert_eq!(as_str(get(lqe, "host-removal-verdict")), "Held");

  let catalog = groups.get("surface-group.legacy-extern-catalog").unwrap();
  assert_eq!(as_str(get(catalog, "current-phase")), "matrix-classified");
  assert_eq!(as_list(get(catalog, "old-symbols")).len(), 12);
  assert!(!as_bool(get(catalog, "owner-switch")));
}

#[test]
fn host_path_targets_are_classified_but_not_delete_ready() {
  let run = eval_file(&fixture_path()).unwrap();
  let targets = attrs_by_path(get(&run, "host-path-targets"));
  assert_eq!(targets.len(), 5);
  for expected in [
    "stdlib/lib/ontology.px",
    "crates/pnix-runtime-legacy/src/ssa_eval/builtins/mod.rs",
    "crates/pnix-runtime-legacy/src/ir/eval.rs",
    "crates/pnix-core/src/ontology.rs",
    "crates/pnix-eval/tests/ontology_builtins.rs",
  ] {
    let target = targets
      .get(expected)
      .unwrap_or_else(|| panic!("missing target `{expected}`"));
    assert!(!as_bool(get(target, "delete-ready")));
    assert!(!as_bool(get(target, "remove-now")));
    assert!(!as_bool(get(target, "archive-ready")));
    assert!(!as_bool(get(target, "target-specific-proof-present")));
    assert!(!as_list(get(target, "retained-because")).is_empty());
  }

  assert_eq!(
    as_str(get(
      targets
        .get("crates/pnix-core/src/ontology.rs")
        .expect("core ontology target"),
      "current-role"
    )),
    "legacy-pure-core-specimen-and-regression-oracle"
  );
}

#[test]
fn deletion_readiness_gate_requires_boot_puck_compare_and_target_proof() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "deletion-readiness-gate");
  assert_eq!(
    as_str(get(gate, "id")),
    "gate.host-code-removal.delete-readiness.v1"
  );
  assert!(as_bool(get(gate, "map-written")));
  assert_eq!(as_str(get(gate, "verdict")), "host-removal-held");
  assert_eq!(as_i64(get(gate, "delete-ready-target-count")), 0);
  for key in [
    "macro-only-boot-manifest-present",
    "target-specific-delete-proof-present",
    "fresh-p-puck-after-current-cut",
    "compare-after-target-removal",
    "compat-usage-scan-complete",
    "external-caller-scan-complete",
    "replay-corpus-transferred",
    "rollback-plan-present",
    "alias-route-map-present",
    "human-consequence-authorization-present",
  ] {
    assert!(!as_bool(get(gate, key)), "`{key}` must stay false");
  }

  let required = string_set(get(gate, "required-before-removal"));
  for expected in [
    "macro-only-ontology-boot-manifest",
    "target-specific-delete-proof",
    "fresh-p-puck-after-current-cut",
    "compare-after-target-removal",
    "compat-usage-scan",
    "external-caller-scan",
    "replay-corpus-transfer",
    "rollback-plan",
    "alias-route-map-for-builtins-and-ontology-dot-names",
  ] {
    assert!(
      required.contains(expected),
      "missing removal prerequisite `{expected}`"
    );
  }
}

#[test]
fn host_removal_trials_hold_deletion_and_accept_mapping_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "host-removal-trials"));
  assert_eq!(trials.len(), 6);
  for expected in [
    "trial.A.remove-stdlib-catalog",
    "trial.B.remove-ssa-builtins",
    "trial.C.remove-ir-builtins",
    "trial.D.remove-core-ontology-rs",
    "trial.E.remove-regression-tests",
  ] {
    let trial = trials
      .get(expected)
      .unwrap_or_else(|| panic!("missing trial `{expected}`"));
    assert_eq!(as_str(get(trial, "outcome")), "Held");
    assert!(!as_bool(get(trial, "delete-ready")));
  }

  let mapped = trials.get("trial.F.map-complete").unwrap();
  assert_eq!(as_str(get(mapped, "outcome")), "mapped-not-removable");
  assert_eq!(as_str(get(mapped, "held-id")), "none");
  assert!(!as_bool(get(mapped, "delete-ready")));
}

#[test]
fn six_layer_host_removal_fold_keeps_map_and_delete_proof_separate() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-host-removal-fold");
  assert_eq!(as_str(get(fold, "mode")), "host-code-removal-map");
  for layer in [
    "surface", "ontology", "semantic", "gate", "runtime", "audit",
  ] {
    assert!(
      as_bool(get_path(fold, &[layer, "visible"])),
      "layer `{layer}` must stay visible"
    );
  }
  assert_eq!(as_i64(get_path(fold, &["surface", "host-target-count"])), 5);
  assert_eq!(
    as_i64(get_path(fold, &["surface", "surface-group-count"])),
    4
  );
  assert!(as_bool(get_path(
    fold,
    &["surface", "old-host-code-still-present"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["ontology", "host-removal-map-written"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["ontology", "old-host-current-authority"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["ontology", "removal-proof-written"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["semantic", "map-is-delete-proof"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "target-specific-proof-required"]
  )));
  assert_eq!(
    as_str(get_path(fold, &["gate", "deletion-verdict"])),
    "host-removal-held"
  );
  assert_eq!(
    as_i64(get_path(fold, &["gate", "delete-ready-target-count"])),
    0
  );
  assert!(!as_bool(get_path(fold, &["runtime", "runtime-install"])));
  assert!(!as_bool(get_path(
    fold,
    &["runtime", "global-ontology-runtime"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["runtime", "host-code-removal-started"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["audit", "old-vs-new-regression-corpus-retained"]
  )));
}

#[test]
fn migration_delta_closes_map_need_but_opens_specific_delete_proofs() {
  let run = eval_file(&fixture_path()).unwrap();
  let delta = get(&run, "migration-delta");
  assert_eq!(
    as_str(get(delta, "id")),
    "migration-delta.host-code-removal-map"
  );
  let closes = string_set(get(delta, "closes"));
  assert!(closes.contains("need.bootstrap.host-code-removal-map"));

  let does_not_close = string_set(get(delta, "does-not-close"));
  for expected in [
    "need.bootstrap.macro-only-ontology-boot-manifest",
    "need.host-removal.target-specific-delete-proof",
    "need.host-removal.fresh-p-puck-after-current-cut",
    "need.lift-query-emit.runtime-owner-or-host-removal",
  ] {
    assert!(
      does_not_close.contains(expected),
      "missing open frontier `{expected}`"
    );
  }

  let next = string_set(get(delta, "next-required"));
  assert!(next.contains("macro-only-ontology-boot-manifest"));
  assert!(next.contains("target-specific-host-delete-proof"));
  assert!(next.contains("fresh-p-puck-and-compare-after-current-cut"));
  assert!(next.contains("regression-corpus-transfer-map"));
}

#[test]
fn discoveries_record_d336_through_d344() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 9);
  for expected in [
    "D336.host-code-removal-map-closes-map-need-not-delete-need",
    "D337.old-host-code-has-four-roles-not-one-clutter-label",
    "D338.evaluate-select-scoped-adapter-does-not-delete-host-dispatch",
    "D339.lift-query-emit-r7-compat-retention-blocks-host-delete",
    "D340.pnix-core-ontology-rs-is-pure-oracle-until-macro-core-replacement",
    "D341.regression-tests-are-removal-gates-not-clutter",
    "D342.host-code-shrink-policy-is-after-owner-proof",
    "D343.stale-p-puck-audit-cannot-authorize-current-host-removal",
    "D344.host-removal-map-opens-macro-only-boot-manifest-next",
  ] {
    let discovery = discoveries
      .get(expected)
      .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
    assert!(as_bool(get(discovery, "scenario-only")));
  }
}

#[test]
fn top_level_state_is_map_written_without_runtime_or_removal() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "replacement-readiness")),
    "host-code-removal-map-written"
  );
  assert!(!as_bool(get(&run, "owner-switch")));
  assert!(as_bool(get(&run, "host-removal-map-written")));
  assert!(!as_bool(get(&run, "host-code-removal-started")));
  assert!(!as_bool(get(&run, "host-removal-safe")));
  assert_eq!(as_i64(get(&run, "delete-ready-target-count")), 0);
  assert!(!as_bool(get(&run, "runtime-install")));
  assert!(!as_bool(get(&run, "global-ontology-runtime")));
  assert!(!as_bool(get(&run, "implementation-command")));
  assert_eq!(as_i64(get(&run, "external-solver-dependency-count")), 0);
  assert!(!as_bool(get(&run, "gpl-family-dependencies")));

  let rejects = string_set(get_path(&run, &["negative-held-evidence", "rejects"]));
  assert!(rejects.contains("host-removal-map-as-delete-proof"));
  assert!(rejects.contains("stale-p-puck-green-as-current-cut-proof"));
  assert!(rejects.contains("llm-cleanup-prose-as-removal-proof"));
}
