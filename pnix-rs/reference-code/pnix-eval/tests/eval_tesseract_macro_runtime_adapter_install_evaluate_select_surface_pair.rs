//! Runtime adapter install for the macro-native evaluate/select pair.
//!
//! This is the first intentionally loosened eval/select runtime path: the
//! concrete stdlib route adapter may be installed for the evaluate/select
//! surface pair. The proof keeps the install scoped, preserves Held behavior,
//! and blocks global runtime, old wrappers, route cache/RigorFloor authority,
//! and GPL-family dependencies.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join(
    "../../fixtures/tesseract-macro-legacy-probe/runtime_adapter_install_evaluate_select_surface_pair_receipt.px",
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
    Value::Int(n) => *n,
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

#[test]
fn install_marker_and_constitution_owner_are_pinned() {
  let run = eval_file(&fixture_path()).expect("runtime adapter install fixture must eval");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-runtime-adapter-install-evaluate-select-surface-pair"
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
    as_str(get(&run, "constitution-owner")),
    "stdlib/lib/gate/tesseract-constitution.px"
  );
}

#[test]
fn constitution_gate_allows_candidate_only_scoped_install_and_blocks_shortcuts() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "runtime-adapter-install-evaluate-select-surface-pair"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  assert_eq!(
    as_str(get(gate, "replacement-readiness")),
    "surface-pair-runtime-adapter-installed-non-global"
  );

  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "treat-surface-pair-adapter-install-as-global-runtime",
    "install-old-evaluate-select-wrapper",
    "install-without-route-adapter-owner",
    "install-without-route-binding-proof",
    "install-without-negative-held-rerun",
    "install-without-rollback-ref",
    "install-without-operator-visible-diff",
    "install-without-license-evidence",
    "install-gpl-family-dependency",
    "hide-license-under-build-script",
    "install-cross-surface-caller-default",
    "hide-selection-tie-break-in-route-cache",
    "promote-rigorfloor-or-route-cache-authority",
    "use-llm-prose-as-install-proof",
  ] {
    assert!(
      blocks.contains(expected),
      "missing shortcut block `{expected}`"
    );
  }
}

#[test]
fn install_record_is_scoped_active_and_non_global() {
  let run = eval_file(&fixture_path()).unwrap();
  let install = get(&run, "runtime-adapter-install");
  assert_eq!(
    as_str(get(install, "id")),
    "install.runtime-adapter.evaluate-select.surface-pair"
  );
  assert_eq!(
    as_str(get(install, "source-route-binding")),
    "route.binding.evaluate-select.surface-pair"
  );
  assert_eq!(
    as_str(get(install, "route-adapter-owner")),
    "stdlib.lib.gate.evaluate-select-route-adapter"
  );
  assert_eq!(
    as_str(get(install, "route-adapter-constructor")),
    "routeEvaluateSelect"
  );
  assert_eq!(
    as_str(get(install, "install-scope")),
    "legacy-evaluate-select-surface-pair-only"
  );
  assert!(as_bool(get(install, "installed")));
  assert!(as_bool(get(install, "active")));
  assert!(as_bool(get(
    install,
    "surface-pair-runtime-adapter-install"
  )));
  assert!(as_bool(get(install, "runtime-adapter-install")));
  assert!(!as_bool(get(install, "runtime-install")));
  assert!(!as_bool(get(install, "global-runtime-install")));
  assert!(!as_bool(get(install, "global-ranking-runtime")));
  assert!(!as_bool(get(install, "old-evaluate-select-wrapper")));
  assert!(!as_bool(get(install, "rigorfloor-authority")));
  assert!(!as_bool(get(install, "route-cache-authority")));
  assert_eq!(as_i64(get(install, "gpl-family-dependency-count")), 0);
  assert!(!as_bool(get(install, "gpl-family-dependencies-allowed")));
}

#[test]
fn license_policy_blocks_gpl_family_dependencies_for_adapter_install() {
  let run = eval_file(&fixture_path()).unwrap();
  let policy = get(&run, "dependency-license-policy");
  assert_eq!(
    as_str(get(policy, "id")),
    "policy.runtime-adapter.dependency-license.v1"
  );
  assert!(!as_bool(get(policy, "gpl-family-allowed")));
  assert!(!as_bool(get(policy, "unknown-license-allowed")));
  assert!(!as_bool(get(policy, "in-process-copyleft-allowed")));

  let allowed = string_set(get(policy, "allowed-families"));
  for expected in [
    "project-owned",
    "MIT",
    "Apache-2.0",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "Zlib",
  ] {
    assert!(
      allowed.contains(expected),
      "missing allowed family `{expected}`"
    );
  }

  let forbidden = string_set(get(policy, "forbidden-families"));
  for expected in [
    "GPL-2.0",
    "GPL-3.0",
    "AGPL-3.0",
    "LGPL-2.1",
    "LGPL-3.0",
    "unknown-copyleft",
  ] {
    assert!(
      forbidden.contains(expected),
      "missing forbidden family `{expected}`"
    );
  }

  let attempt = get(&run, "gpl-dependency-attempt");
  assert_eq!(as_str(get(attempt, "outcome")), "Held");
  assert_eq!(
    as_str(get(attempt, "held-id")),
    "held.runtime-adapter-install.gpl-family-dependency"
  );
  assert_eq!(as_str(get(attempt, "license-family")), "GPL-3.0");
  assert!(as_bool(get(attempt, "gpl-family")));
  assert!(!as_bool(get(attempt, "runtime-adapter-install")));
}

#[test]
fn dependency_manifest_has_license_evidence_and_no_gpl_family_dependency() {
  let run = eval_file(&fixture_path()).unwrap();
  let manifest = get(&run, "dependency-manifest");
  assert_eq!(
    as_str(get(manifest, "id")),
    "manifest.runtime-adapter.evaluate-select.surface-pair.dependencies"
  );
  assert_eq!(as_i64(get(manifest, "dependency-count")), 2);
  assert_eq!(as_i64(get(manifest, "gpl-family-dependency-count")), 0);
  assert!(as_bool(get(manifest, "license-evidence-present")));

  for dep in as_list(get(manifest, "dependencies")) {
    assert_eq!(as_str(get(dep, "license-family")), "project-owned");
    assert!(as_str(get(dep, "license-evidence-ref")).contains("project-owned"));
    assert!(!as_bool(get(dep, "gpl-family")));
  }
}

#[test]
fn installed_route_table_has_one_active_non_global_route() {
  let run = eval_file(&fixture_path()).unwrap();
  let table = get(&run, "installed-route-table");
  assert_eq!(
    as_str(get(table, "id")),
    "runtime.route-table.evaluate-select.surface-pair"
  );
  assert_eq!(as_i64(get(table, "route-count")), 1);
  assert!(!as_bool(get(table, "global-routing")));
  assert!(!as_bool(get(table, "cross-surface-default")));
  assert!(!as_bool(get(table, "route-cache-authority")));

  let route = &as_list(get(table, "active-routes"))[0];
  assert_eq!(
    as_str(get(route, "route-id")),
    "route.binding.evaluate-select.surface-pair"
  );
  assert_eq!(
    as_str(get(route, "adapter-owner")),
    "stdlib.lib.gate.evaluate-select-route-adapter"
  );
  assert!(as_bool(get(route, "installed")));
}

#[test]
fn installed_positive_call_uses_adapter_fast_path_and_selects_winner() {
  let run = eval_file(&fixture_path()).unwrap();
  let selected = get(&run, "installed-positive-call");
  assert_eq!(as_str(get(selected, "status")), "route-ranked");
  assert_eq!(as_str(get(selected, "route-status")), "ranked");
  assert_eq!(as_str(get(selected, "source-status")), "ranked");
  assert_eq!(
    as_str(get(selected, "winner-candidate-id")),
    "candidate.beta"
  );
  assert_eq!(
    as_str(get(selected, "route-adapter-owner")),
    "stdlib.lib.gate.evaluate-select-route-adapter"
  );
  assert_eq!(
    as_str(get(selected, "ranking-owner")),
    "stdlib.lib.gate.evaluate-select-ranking"
  );
  assert!(as_bool(get(selected, "route-bound")));
  assert!(as_bool(get(selected, "adapter-callable")));
  assert!(!as_bool(get(selected, "global-ranking-runtime")));
}

#[test]
fn installed_negative_call_preserves_ranking_owner_held() {
  let run = eval_file(&fixture_path()).unwrap();
  let held = get(&run, "installed-held-call");
  assert_eq!(as_str(get(held, "status")), "Held");
  assert_eq!(
    as_str(get(held, "held-id")),
    "held.evaluate-select-route-adapter.ranking-owner-held"
  );
  assert_eq!(
    as_str(get(held, "source-held-id")),
    "held.evaluate-select-ranking.missing-required-evidence"
  );
  let missing = string_set(get(held, "missing"));
  assert!(
    missing.contains("ranking-owner-held:held.evaluate-select-ranking.missing-required-evidence")
  );
  assert!(!as_bool(get(held, "runtime-install")));
  assert!(!as_bool(get(held, "global-ranking-runtime")));
}

#[test]
fn global_scope_attempt_remains_held_after_scoped_install() {
  let run = eval_file(&fixture_path()).unwrap();
  let held = get(&run, "global-scope-attempt");
  assert_eq!(as_str(get(held, "status")), "Held");
  assert_eq!(
    as_str(get(held, "held-id")),
    "held.evaluate-select-route-adapter.effect-scope-mismatch"
  );
  assert_eq!(as_str(get(held, "effect-scope")), "global-ranking-runtime");
  assert!(!as_bool(get(held, "global-ranking-runtime")));
  assert!(!as_bool(get(held, "runtime-install")));
}

#[test]
fn install_proof_bundle_requires_fast_path_replay_audit_and_license_evidence() {
  let run = eval_file(&fixture_path()).unwrap();
  let proof = get(&run, "install-proof-bundle");
  assert_eq!(
    as_str(get(proof, "id")),
    "proof.runtime-adapter-install.evaluate-select.surface-pair"
  );
  for key in [
    "route-binding-present",
    "route-adapter-owner-present",
    "positive-installed-route-call-present",
    "negative-held-installed-route-call-present",
    "rollback-ref-present",
    "operator-visible-install-diff-present",
    "install-audit-ref-present",
    "license-evidence-present",
  ] {
    assert!(as_bool(get(proof, key)), "`{key}` must be true");
  }
  assert_eq!(
    as_str(get(proof, "positive-installed-route-call-status")),
    "route-ranked"
  );
  assert_eq!(
    as_str(get(proof, "positive-installed-route-winner")),
    "candidate.beta"
  );
  assert_eq!(
    as_str(get(proof, "negative-held-source")),
    "held.evaluate-select-ranking.missing-required-evidence"
  );
  assert_eq!(as_i64(get(proof, "gpl-family-dependency-count")), 0);
  assert!(as_bool(get(proof, "runtime-adapter-install")));
  assert!(!as_bool(get(proof, "runtime-install")));
  assert!(!as_bool(get(proof, "global-runtime-install")));
}

#[test]
fn install_trials_hold_missing_inputs_globalization_old_wrapper_gpl_and_prose() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "install-trials"));
  assert_eq!(trials.len(), 11);
  for expected in [
    "trial.A.route-binding-missing",
    "trial.B.adapter-owner-missing",
    "trial.C.negative-held-rerun-missing",
    "trial.D.rollback-ref-missing",
    "trial.E.operator-visible-diff-missing",
    "trial.F.global-runtime-claimed",
    "trial.G.cross-surface-default",
    "trial.H.old-wrapper-restored",
    "trial.I.llm-prose-install-proof",
    "trial.J.gpl-family-dependency",
  ] {
    let trial = trials
      .get(expected)
      .unwrap_or_else(|| panic!("missing trial `{expected}`"));
    assert_eq!(as_str(get(trial, "outcome")), "Held");
    assert!(!as_bool(get(trial, "runtime-adapter-install")));
  }

  let complete = trials
    .get("trial.K.complete-surface-pair-adapter-install")
    .unwrap();
  assert_eq!(
    as_str(get(complete, "outcome")),
    "surface-pair-runtime-adapter-installed"
  );
  assert_eq!(as_str(get(complete, "held-id")), "none");
  assert!(as_bool(get(complete, "runtime-adapter-install")));
}

#[test]
fn six_layer_install_fold_keeps_runtime_and_license_boundaries_visible() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-install-fold");
  assert_eq!(
    as_str(get(fold, "mode")),
    "runtime-adapter-install-evaluate-select-surface-pair"
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
    as_str(get_path(fold, &["ontology", "install-verdict"])),
    "surface-pair-runtime-adapter-installed"
  );
  assert_eq!(
    as_str(get_path(fold, &["semantic", "positive-winner"])),
    "candidate.beta"
  );
  assert!(as_bool(get_path(
    fold,
    &["runtime", "runtime-adapter-install"]
  )));
  assert!(!as_bool(get_path(fold, &["runtime", "runtime-install"])));
  assert!(!as_bool(get_path(
    fold,
    &["runtime", "global-runtime-install"]
  )));
  assert_eq!(
    as_str(get_path(fold, &["audit", "dependency-license-policy"])),
    "policy.runtime-adapter.dependency-license.v1"
  );
  assert_eq!(
    as_i64(get_path(fold, &["audit", "gpl-family-dependency-count"])),
    0
  );
}

#[test]
fn runtime_observation_is_active_fast_path_but_not_global_runtime() {
  let run = eval_file(&fixture_path()).unwrap();
  let runtime = get(&run, "runtime-observation");
  assert_eq!(
    as_str(get(runtime, "observation-model")),
    "surface-pair-runtime-adapter-installed-non-global"
  );
  assert!(as_bool(get(runtime, "can-appear-at-runtime")));
  assert!(as_bool(get(runtime, "active")));
  assert_eq!(as_i64(get(runtime, "route-count")), 1);
  assert!(as_bool(get(
    runtime,
    "surface-pair-runtime-adapter-install"
  )));
  assert!(as_bool(get(runtime, "runtime-adapter-install")));
  assert!(!as_bool(get(runtime, "runtime-install")));
  assert!(!as_bool(get(runtime, "global-runtime-install")));
  assert!(!as_bool(get(runtime, "global-ranking-runtime")));
  assert_eq!(as_i64(get(runtime, "gpl-family-dependency-count")), 0);

  let candidates = attrs_by_id(get(runtime, "runtime-added-candidates"));
  assert_eq!(
    as_str(get(
      candidates
        .get("runtime.adapter-install.evaluate-select.positive-call")
        .unwrap(),
      "winner-candidate-id"
    )),
    "candidate.beta"
  );
  assert_eq!(
    as_str(get(
      candidates
        .get("runtime.adapter-install.evaluate-select.held-call")
        .unwrap(),
      "source-held-id"
    )),
    "held.evaluate-select-ranking.missing-required-evidence"
  );
}

#[test]
fn discoveries_record_d231_through_d240() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 10);
  for expected in [
    "D231.surface-pair-runtime-adapter-install-is-allowed",
    "D232.adapter-install-binds-concrete-stdlib-owner",
    "D233.installed-route-table-is-single-route-and-non-global",
    "D234.negative-held-survives-installed-route",
    "D235.global-scope-attempt-still-held-after-install",
    "D236.rollback-diff-and-audit-are-load-bearing-for-install",
    "D237.old-wrapper-and-cross-surface-default-remain-blocked",
    "D238.runtime-install-name-is-split-from-scoped-adapter-install",
    "D239.next-frontier-is-promotion-or-rollback-policy-not-global-runtime",
    "D240.gpl-family-dependencies-are-held-for-runtime-adapter-install",
  ] {
    let discovery = discoveries
      .get(expected)
      .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
    assert!(as_bool(get(discovery, "scenario-only")));
  }
}

#[test]
fn affected_plans_absorb_scoped_feature_without_globalizing_old_ontology() {
  let run = eval_file(&fixture_path()).unwrap();
  let affected = get(&run, "affected-plans");
  assert!(as_bool(get_path(
    affected,
    &["runtimeAdapterInstall", "implementation-target"]
  )));
  assert_eq!(
    as_str(get_path(affected, &["runtimeAdapterInstall", "pressure"])),
    "scoped feature absorbed"
  );
  assert!(as_bool(get_path(
    affected,
    &["externalSolverLicenseGate", "implementation-target"]
  )));
  for key in [
    "runtimeAdapterPromotionOrRollback",
    "globalRankingRuntime",
    "legacyEvaluateSelectWrapper",
    "RigorFloor",
    "routeCache",
  ] {
    assert!(!as_bool(get_path(
      affected,
      &[key, "implementation-target"]
    )));
  }
}

#[test]
fn negative_held_evidence_survives_runtime_adapter_install() {
  let run = eval_file(&fixture_path()).unwrap();
  let negative = get(&run, "negative-held-evidence");
  assert_eq!(as_str(get(negative, "status")), "present");
  assert!(as_bool(get(negative, "survives-runtime-adapter-install")));
  let rejects = string_set(get(negative, "rejects"));
  for expected in [
    "install-before-route-binding",
    "install-before-adapter-owner",
    "install-without-negative-held-rerun",
    "install-without-rollback-ref",
    "install-without-visible-diff",
    "install-without-license-evidence",
    "gpl-family-dependency-in-runtime-adapter",
    "global-runtime-from-scoped-adapter",
    "cross-surface-default-from-scoped-adapter",
    "old-wrapper-restored-by-install",
    "llm-prose-as-install-proof",
  ] {
    assert!(rejects.contains(expected), "missing rejection `{expected}`");
  }
}

#[test]
fn top_level_state_records_scoped_fast_path_enabled_without_global_runtime() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "replacement-readiness")),
    "surface-pair-runtime-adapter-installed-non-global"
  );
  assert!(as_bool(get(&run, "owner-switch")));
  assert!(as_bool(get(&run, "surface-pair-runtime-adapter-install")));
  assert!(as_bool(get(&run, "runtime-adapter-install-enabled")));
  assert_eq!(
    as_str(get(&run, "runtime-adapter-install-status")),
    "surface-pair-installed"
  );
  assert!(as_bool(get(&run, "license-evidence-present")));
  assert_eq!(as_i64(get(&run, "gpl-family-dependency-count")), 0);
  assert!(!as_bool(get(&run, "gpl-family-dependencies-allowed")));
  assert!(!as_bool(get(&run, "runtime-install")));
  assert!(!as_bool(get(&run, "global-runtime-install")));
  assert!(!as_bool(get(&run, "global-ranking-runtime")));
  assert!(!as_bool(get(&run, "rigorfloor-authority")));
  assert!(!as_bool(get(&run, "route-cache-authority")));
  assert!(!as_bool(get(&run, "old-evaluate-select-wrapper")));
  assert!(!as_bool(get(&run, "split-evaluate-select-owner")));
  assert!(!as_bool(get(&run, "nix-checks-gate-added")));
  assert!(!as_bool(get(&run, "implementation-command")));
}
