//! Runtime ranking install readiness for the macro-native evaluate/select pair.
//!
//! The runtime ranking owner contract is already pinned, but executable install
//! is still Held. This test pins the install-readiness bundle without installing
//! execution.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join(
    "../../fixtures/tesseract-macro-legacy-probe/runtime_ranking_install_readiness_evaluate_select_surface_pair_receipt.px",
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
fn install_readiness_marker_and_constitution_owner_are_pinned() {
  let run = eval_file(&fixture_path()).expect("runtime install readiness fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-runtime-ranking-install-readiness-evaluate-select-surface-pair"
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
fn constitution_gate_keeps_install_readiness_candidate_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "runtime-ranking-install-readiness-evaluate-select-surface-pair"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  assert_eq!(
    as_str(get(gate, "replacement-readiness")),
    "runtime-ranking-install-readiness-pinned-non-installed"
  );

  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "install-executable-route-from-readiness-bundle",
    "treat-fresh-replay-as-runtime-install",
    "drop-negative-held-corpus-for-green-install",
    "use-regression-corpus-as-current-authority",
    "hide-tie-break-in-route-cache",
    "skip-rollback-plan",
    "skip-effect-scope",
    "optimize-away-held-or-provenance-for-performance",
    "claim-global-ranking-runtime-from-surface-pair",
    "promote-rigorfloor-or-route-cache-authority",
    "use-llm-prose-as-install-proof",
    "treat-green-tests-as-executable-install",
  ] {
    assert!(
      blocks.contains(expected),
      "missing shortcut block `{expected}`"
    );
  }
}

#[test]
fn owner_input_imports_runtime_owner_contract_without_install() {
  let run = eval_file(&fixture_path()).unwrap();
  let input = get(&run, "owner-input");
  assert_eq!(
    as_str(get(input, "source")),
    "runtime_ranking_owner_evaluate_select_surface_pair_receipt.px"
  );
  assert_eq!(
    as_str(get(input, "owner-contract")),
    "owner.runtime-ranking.evaluate-select.surface-pair"
  );
  assert_eq!(
    as_str(get(input, "owner-contract-status")),
    "pinned-non-installed"
  );
  assert!(as_bool(get(input, "runtime-ranking-owner-receipt")));
  assert!(as_bool(get(input, "candidate-set-required")));
  assert!(as_bool(get(input, "axis-evidence-vector-required")));
  assert!(as_bool(get(input, "null-candidate-set-emits-held")));
  assert!(as_bool(get(input, "hidden-tie-break-emits-held")));
  assert!(!as_bool(get(input, "legacy-current-authority")));
  assert!(!as_bool(get(input, "runtime-install")));
  assert!(!as_bool(get(input, "ranking-runtime-install")));
  assert!(!as_bool(get(input, "global-ranking-runtime")));
}

#[test]
fn install_readiness_bundle_contains_all_required_proof_inputs() {
  let run = eval_file(&fixture_path()).unwrap();
  let bundle = get(&run, "install-readiness-bundle");
  assert_eq!(
    as_str(get(bundle, "id")),
    "bundle.runtime-ranking-install-readiness.evaluate-select.surface-pair"
  );
  for key in [
    "fresh-replay-corpus-present",
    "negative-held-corpus-present",
    "ranking-regression-corpus-check-present",
    "deterministic-tie-break-check-present",
    "rollback-plan-present",
    "effect-scope-present",
    "performance-budget-present",
    "performance-budget-preserves-held",
    "performance-budget-preserves-provenance",
  ] {
    assert!(as_bool(get(bundle, key)), "`{key}` must be true");
  }
  assert_eq!(
    as_str(get(bundle, "effect-scope")),
    "legacy-evaluate-select-surface-pair-only"
  );
  assert!(!as_bool(get(bundle, "consequence-bearing")));
  assert!(!as_bool(get(
    bundle,
    "human-consequence-authorization-required"
  )));
  assert!(!as_bool(get(bundle, "executable-route-present")));
  assert!(!as_bool(get(bundle, "executable-install-authorized")));
  assert!(!as_bool(get(bundle, "runtime-install")));
}

#[test]
fn readiness_verdict_opens_executable_install_without_authorizing_it() {
  let run = eval_file(&fixture_path()).unwrap();
  let verdict = get(&run, "readiness-verdict");
  assert_eq!(
    as_str(get(verdict, "id")),
    "readiness.runtime-ranking-install.evaluate-select.surface-pair"
  );
  assert_eq!(
    as_str(get(verdict, "verdict")),
    "ready-for-executable-install-receipt"
  );
  assert!(as_bool(get(verdict, "install-readiness")));
  assert_eq!(
    as_str(get(verdict, "opens-frontier")),
    "need.executable-ranking-runtime-install-receipt"
  );
  assert!(!as_bool(get(verdict, "executable-route-present")));
  assert!(!as_bool(get(verdict, "executable-install-authorized")));
  assert!(!as_bool(get(verdict, "runtime-install")));
  assert!(!as_bool(get(verdict, "ranking-runtime-install")));
  assert!(!as_bool(get(verdict, "global-ranking-runtime")));
  assert!(!as_bool(get(verdict, "rigorfloor-authority")));
  assert!(!as_bool(get(verdict, "route-cache-authority")));
  assert!(!as_bool(get(verdict, "implementation-command")));
}

#[test]
fn install_gate_after_readiness_still_holds_executable_install() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "install-gate-after-readiness");
  assert_eq!(as_str(get(gate, "previous-verdict")), "install-held");
  assert!(as_bool(get(gate, "owner-contract-present")));
  assert!(as_bool(get(gate, "install-readiness-bundle-present")));
  assert!(!as_bool(get(gate, "install-receipt-present")));
  assert!(!as_bool(get(gate, "executable-route-present")));
  assert_eq!(as_str(get(gate, "verdict")), "executable-install-held");

  let required = string_set(get(gate, "required-before-executable-install"));
  for expected in [
    "executable-install-receipt",
    "route-owner-binding",
    "runtime-effect-audit",
    "fresh-replay-rerun-at-install-time",
    "negative-held-rerun-at-install-time",
    "rollback-dry-run",
    "operator-visible-install-diff",
    "human-consequence-authorization-if-consequence-bearing",
  ] {
    assert!(
      required.contains(expected),
      "missing executable proof `{expected}`"
    );
  }
}

#[test]
fn readiness_trials_hold_missing_inputs_and_accept_complete_readiness() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "readiness-trials"));
  assert_eq!(trials.len(), 11);
  for expected in [
    "trial.A.owner-contract-missing",
    "trial.B.fresh-replay-missing",
    "trial.C.negative-held-missing",
    "trial.D.regression-corpus-check-missing",
    "trial.E.tie-break-nondeterministic",
    "trial.F.rollback-missing",
    "trial.G.effect-scope-missing",
    "trial.H.performance-removes-held",
    "trial.I.readiness-installs-runtime",
    "trial.J.llm-prose-install-proof",
  ] {
    let trial = trials
      .get(expected)
      .unwrap_or_else(|| panic!("missing trial `{expected}`"));
    assert_eq!(as_str(get(trial, "outcome")), "Held");
    assert!(!as_bool(get(trial, "install-readiness")));
    assert!(!as_bool(get(trial, "runtime-install")));
  }

  let complete = trials.get("trial.K.complete-install-readiness").unwrap();
  assert_eq!(
    as_str(get(complete, "outcome")),
    "ready-for-executable-install-receipt"
  );
  assert_eq!(as_str(get(complete, "held-id")), "none");
  assert!(as_bool(get(complete, "install-readiness")));
  assert!(!as_bool(get(complete, "runtime-install")));
}

#[test]
fn six_layer_readiness_fold_preserves_execution_boundary() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-readiness-fold");
  assert_eq!(
    as_str(get(fold, "mode")),
    "runtime-ranking-install-readiness-evaluate-select-surface-pair"
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
    as_str(get_path(fold, &["surface", "effect-scope"])),
    "legacy-evaluate-select-surface-pair-only"
  );
  assert!(!as_bool(get_path(
    fold,
    &["surface", "split-proof-present"]
  )));
  assert_eq!(
    as_str(get_path(fold, &["ontology", "readiness-verdict"])),
    "ready-for-executable-install-receipt"
  );
  assert!(!as_bool(get_path(
    fold,
    &["ontology", "global-ranking-runtime"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "fresh-replay-corpus-present"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "negative-held-corpus-present"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "performance-preserves-held"]
  )));
  assert_eq!(
    as_str(get_path(fold, &["gate", "install-gate-verdict"])),
    "executable-install-held"
  );
  assert!(!as_bool(get_path(
    fold,
    &["gate", "executable-install-authorized"]
  )));
  assert!(as_bool(get_path(fold, &["runtime", "install-readiness"])));
  assert!(!as_bool(get_path(fold, &["runtime", "executable-now"])));
  assert!(!as_bool(get_path(
    fold,
    &["runtime", "ranking-runtime-installed"]
  )));
}

#[test]
fn runtime_observation_exposes_readiness_but_no_installed_runtime() {
  let run = eval_file(&fixture_path()).unwrap();
  let runtime = get(&run, "runtime-observation");
  assert_eq!(
    as_str(get(runtime, "observation-model")),
    "runtime-ranking-install-readiness-evaluate-select-non-installed"
  );
  assert!(as_bool(get(runtime, "can-appear-at-runtime")));
  assert!(as_bool(get(runtime, "install-readiness")));
  assert!(!as_bool(get(runtime, "canonical-runtime-installed")));
  assert!(!as_bool(get(runtime, "executable-now")));
  assert!(!as_bool(get(runtime, "ranking-runtime-installed")));
  assert!(!as_bool(get(runtime, "global-ranking-runtime")));
  assert_eq!(as_list(get(runtime, "runtime-added-candidates")).len(), 3);
  let candidates = attrs_by_id(get(runtime, "runtime-added-candidates"));
  assert_eq!(
    as_str(get(
      candidates
        .get("runtime.ranking-install-readiness.install-gate")
        .unwrap(),
      "status"
    )),
    "Held"
  );
}

#[test]
fn discoveries_record_d197_through_d205() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 9);
  for expected in [
    "D197.install-readiness-is-not-runtime-install",
    "D198.fresh-replay-corpus-gates-ranking-install",
    "D199.negative-held-corpus-is-load-bearing-for-install",
    "D200.deterministic-tie-break-check-precedes-install",
    "D201.rollback-and-effect-scope-precede-executable-route",
    "D202.performance-budget-cannot-remove-held-or-provenance",
    "D203.regression-corpus-check-remains-oracle-not-authority",
    "D204.install-readiness-keeps-consequence-authorization-conditional",
    "D205.executable-ranking-install-remains-future-frontier",
  ] {
    let discovery = discoveries
      .get(expected)
      .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
    assert!(as_bool(get(discovery, "scenario-only")));
  }
}

#[test]
fn affected_plans_keep_executable_install_and_global_authority_unimplemented() {
  let run = eval_file(&fixture_path()).unwrap();
  let affected = get(&run, "affected-plans");
  assert_eq!(
    as_str(get_path(
      affected,
      &["runtimeRankingInstallReadiness", "pressure"]
    )),
    "ready-for-future-executable-install-receipt"
  );
  assert_eq!(
    as_str(get_path(
      affected,
      &["executableRankingRuntimeInstall", "pressure"]
    )),
    "still-held-after-readiness"
  );
  for key in [
    "runtimeRankingInstallReadiness",
    "executableRankingRuntimeInstall",
    "runtimeRankingOwner",
    "legacyEvaluateSelect",
    "RigorFloor",
    "routeCache",
    "globalRankingRuntime",
    "otherOntologySurfaces",
  ] {
    assert!(!as_bool(get_path(
      affected,
      &[key, "implementation-target"]
    )));
  }
}

#[test]
fn negative_held_evidence_survives_install_readiness() {
  let run = eval_file(&fixture_path()).unwrap();
  let negative = get(&run, "negative-held-evidence");
  assert_eq!(as_str(get(negative, "status")), "present");
  assert!(as_bool(get(negative, "survives-install-readiness")));
  let rejects = string_set(get(negative, "rejects"));
  for expected in [
    "install-readiness-before-owner-contract",
    "install-readiness-without-fresh-replay",
    "install-readiness-without-negative-held",
    "install-readiness-without-regression-corpus-check",
    "install-readiness-with-nondeterministic-tie-break",
    "install-readiness-without-rollback",
    "install-readiness-without-effect-scope",
    "performance-removes-held-or-provenance",
    "readiness-installs-executable-route",
    "llm-prose-as-install-proof",
    "global-ranking-runtime-from-readiness",
    "rigorfloor-routecache-authority-from-readiness",
  ] {
    assert!(rejects.contains(expected), "missing rejection `{expected}`");
  }
}

#[test]
fn blocked_shortcuts_cover_readiness_as_install_collapses() {
  let run = eval_file(&fixture_path()).unwrap();
  let blocks = string_set(get(&run, "blocked-shortcuts"));
  for expected in [
    "install-executable-route-from-readiness-bundle",
    "treat-fresh-replay-as-runtime-install",
    "drop-negative-held-corpus-for-green-install",
    "use-regression-corpus-as-current-authority",
    "hide-tie-break-in-route-cache",
    "skip-rollback-plan",
    "skip-effect-scope",
    "optimize-away-held-or-provenance-for-performance",
    "claim-global-ranking-runtime-from-surface-pair",
    "promote-rigorfloor-or-route-cache-authority",
    "use-llm-prose-as-install-proof",
    "treat-green-tests-as-executable-install",
  ] {
    assert!(
      blocks.contains(expected),
      "missing shortcut block `{expected}`"
    );
  }
}

#[test]
fn top_level_state_records_install_readiness_without_execution() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "replacement-readiness")),
    "runtime-ranking-install-readiness-pinned-non-installed"
  );
  assert!(as_bool(get(&run, "owner-switch")));
  assert!(as_bool(get(&run, "runtime-ranking-owner-receipt")));
  assert!(as_bool(get(&run, "runtime-ranking-install-readiness")));
  assert!(as_bool(get(&run, "install-readiness")));
  assert!(!as_bool(get(&run, "executable-install-receipt")));
  assert!(!as_bool(get(&run, "runtime-install")));
  assert!(!as_bool(get(&run, "ranking-runtime-install")));
  assert!(!as_bool(get(&run, "global-ranking-runtime")));
  assert!(!as_bool(get(&run, "rigorfloor-authority")));
  assert!(!as_bool(get(&run, "route-cache-authority")));
  assert!(!as_bool(get(&run, "implementation-command")));
}
