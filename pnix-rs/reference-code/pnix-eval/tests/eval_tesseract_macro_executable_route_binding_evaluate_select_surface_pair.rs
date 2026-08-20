//! Executable route binding for the macro-native evaluate/select pair.
//!
//! The stdlib ranking owner is executable .px law, but not route installation.
//! This test pins the next narrow proof: an evaluate/select surface-pair route
//! calls that owner and replays ranked plus Held outcomes, while global runtime
//! install, old wrappers, route cache authority, and RigorFloor authority stay
//! blocked.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join(
    "../../fixtures/tesseract-macro-legacy-probe/executable_route_binding_evaluate_select_surface_pair_receipt.px",
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
fn route_binding_marker_and_constitution_owner_are_pinned() {
  let run = eval_file(&fixture_path()).expect("route binding fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-executable-route-binding-evaluate-select-surface-pair"
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
fn constitution_gate_keeps_route_binding_candidate_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "executable-route-binding-evaluate-select-surface-pair"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  assert_eq!(
    as_str(get(gate, "replacement-readiness")),
    "surface-pair-executable-route-binding-pinned-non-installed"
  );

  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "treat-route-binding-as-global-runtime-install",
    "replace-builtins-ontology-evaluate-select",
    "restore-old-evaluate-select-wrapper",
    "skip-stdlib-ranking-owner",
    "skip-positive-route-rerun",
    "skip-negative-held-rerun",
    "skip-rollback-dry-run",
    "skip-operator-visible-route-diff",
    "hide-tie-break-in-route-cache",
    "promote-rigorfloor-authority",
    "split-select-away-from-evaluate",
    "use-llm-prose-as-route-binding-proof",
    "treat-green-tests-as-runtime-install",
  ] {
    assert!(
      blocks.contains(expected),
      "missing shortcut block `{expected}`"
    );
  }
}

#[test]
fn route_binding_imports_readiness_and_stdlib_owner() {
  let run = eval_file(&fixture_path()).unwrap();
  assert!(as_bool(get(&run, "install-readiness")));

  let owner = get(&run, "ranking-owner-meta");
  assert_eq!(
    as_str(get(owner, "owner")),
    "stdlib.lib.gate.evaluate-select-ranking"
  );
  assert_eq!(
    as_str(get(owner, "surface-pair")),
    "macro-native.evaluate-select.surface-pair-owner"
  );
  assert_eq!(as_str(get(owner, "constructor")), "selectWinner");
  assert_eq!(as_str(get(owner, "ranking-constructor")), "rankCandidates");
  assert!(!as_bool(get(owner, "runtime-install")));
}

#[test]
fn route_binding_is_surface_pair_scoped_and_non_global() {
  let run = eval_file(&fixture_path()).unwrap();
  let binding = get(&run, "route-binding");
  assert_eq!(
    as_str(get(binding, "id")),
    "route.binding.evaluate-select.surface-pair"
  );
  assert_eq!(
    as_str(get(binding, "source-owner")),
    "stdlib.lib.gate.evaluate-select-ranking"
  );
  assert_eq!(as_str(get(binding, "owner-constructor")), "selectWinner");
  assert!(as_bool(get(binding, "route-bound")));
  assert!(as_bool(get(binding, "surface-pair-executable-route-bound")));
  assert_eq!(
    as_str(get(binding, "effect-scope")),
    "legacy-evaluate-select-surface-pair-only"
  );
  assert!(!as_bool(get(binding, "replaces-builtins")));
  assert!(!as_bool(get(binding, "old-evaluate-select-wrapper")));
  assert!(!as_bool(get(binding, "split-evaluate-select-owner")));
  assert!(!as_bool(get(binding, "runtime-install")));
  assert!(!as_bool(get(binding, "ranking-runtime-install")));
  assert!(!as_bool(get(binding, "runtime-adapter-install")));
  assert!(!as_bool(get(binding, "global-ranking-runtime")));
  assert!(!as_bool(get(binding, "rigorfloor-authority")));
  assert!(!as_bool(get(binding, "route-cache-authority")));
}

#[test]
fn positive_route_run_calls_owner_and_preserves_winner_audit_and_tie_break() {
  let run = eval_file(&fixture_path()).unwrap();
  let selected = get(&run, "positive-route-run");
  assert_eq!(as_str(get(selected, "status")), "ranked");
  assert_eq!(
    as_str(get(selected, "winner-candidate-id")),
    "candidate.beta"
  );
  assert_eq!(
    as_str(get(selected, "tie-break-ref")),
    "tie-break.lexical-candidate-id.v1"
  );
  assert_eq!(
    as_str(get(selected, "audit-ref")),
    "audit.route-binding.positive-rerun"
  );
  assert_eq!(as_list(get(selected, "ranking")).len(), 3);
  assert!(!as_bool(get(selected, "runtime-install")));
  assert!(!as_bool(get(selected, "ranking-runtime-install")));
  assert!(!as_bool(get(selected, "global-ranking-runtime")));
}

#[test]
fn negative_route_reruns_keep_owner_held_outputs() {
  let run = eval_file(&fixture_path()).unwrap();
  let empty = get(&run, "empty-route-run");
  let missing_axis = get(&run, "missing-axis-route-run");
  let no_tie = get(&run, "no-tie-break-route-run");
  let no_prov = get(&run, "no-provenance-route-run");

  assert_eq!(as_str(get(empty, "status")), "Held");
  assert_eq!(
    as_str(get(empty, "held-id")),
    "held.evaluate-select-ranking.empty-candidate-set"
  );
  assert_eq!(as_str(get(missing_axis, "status")), "Held");
  assert_eq!(
    as_str(get(missing_axis, "held-id")),
    "held.evaluate-select-ranking.missing-required-evidence"
  );
  assert_eq!(as_str(get(no_tie, "status")), "Held");
  assert_eq!(
    as_str(get(no_tie, "held-id")),
    "held.evaluate-select-ranking.tie-break-ref-missing"
  );
  assert_eq!(as_str(get(no_prov, "status")), "Held");
  assert_eq!(
    as_str(get(no_prov, "held-id")),
    "held.evaluate-select-ranking.missing-required-evidence"
  );

  for held in [empty, missing_axis, no_tie, no_prov] {
    assert!(!as_bool(get(held, "runtime-install")));
    assert!(!as_bool(get(held, "ranking-runtime-install")));
    assert!(!as_bool(get(held, "route-cache-authority")));
  }
}

#[test]
fn route_proof_bundle_requires_reruns_rollback_diff_and_effect_audit() {
  let run = eval_file(&fixture_path()).unwrap();
  let proof = get(&run, "route-proof-bundle");
  assert_eq!(
    as_str(get(proof, "id")),
    "proof.executable-route-binding.evaluate-select.surface-pair"
  );
  for key in [
    "route-owner-binding-present",
    "install-readiness-present",
    "stdlib-ranking-owner-present",
    "positive-route-rerun-present",
    "negative-held-rerun-present",
    "rollback-dry-run-present",
    "operator-visible-route-diff-present",
    "runtime-effect-audit-present",
  ] {
    assert!(as_bool(get(proof, key)), "`{key}` must be true");
  }
  assert_eq!(as_str(get(proof, "positive-route-rerun-status")), "ranked");
  assert_eq!(
    as_str(get(proof, "positive-route-winner")),
    "candidate.beta"
  );
  assert_eq!(as_i64(get(proof, "negative-held-rerun-count")), 4);
  assert_eq!(
    as_str(get(proof, "effect-scope")),
    "legacy-evaluate-select-surface-pair-only"
  );
  assert!(!as_bool(get(proof, "runtime-install")));
  assert!(!as_bool(get(proof, "global-ranking-runtime")));
}

#[test]
fn route_binding_verdict_closes_route_frontier_without_installing_adapter() {
  let run = eval_file(&fixture_path()).unwrap();
  let verdict = get(&run, "route-binding-verdict");
  assert_eq!(
    as_str(get(verdict, "closes-frontier")),
    "need.executable-ranking-runtime-install-receipt"
  );
  assert_eq!(
    as_str(get(verdict, "opens-frontier")),
    "need.runtime-adapter-install-receipt-if-needed"
  );
  assert_eq!(
    as_str(get(verdict, "verdict")),
    "surface-pair-executable-route-bound-non-installed"
  );
  assert!(as_bool(get(verdict, "route-bound")));
  assert!(as_bool(get(verdict, "surface-pair-executable-route-bound")));
  assert!(as_bool(get(verdict, "can-call-stdlib-ranking-owner")));
  assert!(!as_bool(get(verdict, "runtime-install")));
  assert!(!as_bool(get(verdict, "ranking-runtime-install")));
  assert!(!as_bool(get(verdict, "runtime-adapter-install")));
  assert!(!as_bool(get(verdict, "global-ranking-runtime")));
  assert!(!as_bool(get(verdict, "old-evaluate-select-wrapper")));
  assert!(!as_bool(get(verdict, "rust-builtin-replacement")));
}

#[test]
fn binding_trials_hold_missing_inputs_and_accept_complete_route_binding() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "binding-trials"));
  assert_eq!(trials.len(), 10);
  for expected in [
    "trial.A.install-readiness-missing",
    "trial.B.stdlib-ranking-owner-missing",
    "trial.C.positive-route-rerun-missing",
    "trial.D.negative-held-rerun-missing",
    "trial.E.rollback-dry-run-missing",
    "trial.F.operator-visible-route-diff-missing",
    "trial.G.effect-scope-globalized",
    "trial.H.old-wrapper-restored",
    "trial.I.llm-prose-route-proof",
  ] {
    let trial = trials
      .get(expected)
      .unwrap_or_else(|| panic!("missing trial `{expected}`"));
    assert_eq!(as_str(get(trial, "outcome")), "Held");
    assert!(!as_bool(get(trial, "route-bound")));
    assert!(!as_bool(get(trial, "runtime-install")));
  }

  let complete = trials.get("trial.J.complete-route-binding").unwrap();
  assert_eq!(
    as_str(get(complete, "outcome")),
    "surface-pair-executable-route-bound-non-installed"
  );
  assert_eq!(as_str(get(complete, "held-id")), "none");
  assert!(as_bool(get(complete, "route-bound")));
  assert!(!as_bool(get(complete, "runtime-install")));
}

#[test]
fn six_layer_route_fold_keeps_runtime_boundary_visible() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-route-fold");
  assert_eq!(
    as_str(get(fold, "mode")),
    "executable-route-binding-evaluate-select-surface-pair"
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
  assert!(!as_bool(get_path(
    fold,
    &["surface", "old-wrapper-restored"]
  )));
  assert_eq!(
    as_str(get_path(fold, &["ontology", "owner"])),
    "stdlib.lib.gate.evaluate-select-ranking"
  );
  assert!(as_bool(get_path(fold, &["ontology", "route-bound"])));
  assert_eq!(
    as_str(get_path(fold, &["semantic", "positive-route-winner"])),
    "candidate.beta"
  );
  assert_eq!(
    as_i64(get_path(fold, &["semantic", "negative-held-rerun-count"])),
    4
  );
  assert!(as_bool(get_path(fold, &["gate", "blocks-global-runtime"])));
  assert!(as_bool(get_path(fold, &["gate", "blocks-old-wrapper"])));
  assert!(as_bool(get_path(fold, &["runtime", "route-bound"])));
  assert!(!as_bool(get_path(fold, &["runtime", "runtime-installed"])));
  assert!(!as_bool(get_path(
    fold,
    &["runtime", "ranking-runtime-installed"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["runtime", "runtime-adapter-installed"]
  )));
}

#[test]
fn runtime_observation_has_route_binding_but_no_installed_runtime() {
  let run = eval_file(&fixture_path()).unwrap();
  let runtime = get(&run, "runtime-observation");
  assert_eq!(
    as_str(get(runtime, "observation-model")),
    "surface-pair-executable-route-binding-non-installed"
  );
  assert!(as_bool(get(runtime, "can-appear-at-runtime")));
  assert!(as_bool(get(runtime, "route-bound")));
  assert!(as_bool(get(runtime, "surface-pair-executable-route-bound")));
  assert!(!as_bool(get(runtime, "canonical-runtime-installed")));
  assert!(!as_bool(get(runtime, "ranking-runtime-installed")));
  assert!(!as_bool(get(runtime, "runtime-adapter-installed")));
  assert!(!as_bool(get(runtime, "global-ranking-runtime")));
  assert_eq!(as_list(get(runtime, "runtime-added-candidates")).len(), 3);

  let candidates = attrs_by_id(get(runtime, "runtime-added-candidates"));
  assert_eq!(
    as_str(get(
      candidates
        .get("runtime.route-binding.evaluate-select.owner-call")
        .unwrap(),
      "status"
    )),
    "route-bound"
  );
  assert_eq!(
    as_str(get(
      candidates
        .get("runtime.route-binding.evaluate-select.positive-rerun")
        .unwrap(),
      "winner-candidate-id"
    )),
    "candidate.beta"
  );
  assert_eq!(
    as_i64(get(
      candidates
        .get("runtime.route-binding.evaluate-select.negative-held")
        .unwrap(),
      "held-count"
    )),
    4
  );
}

#[test]
fn discoveries_record_d214_through_d222() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 9);
  for expected in [
    "D214.route-binding-is-scoped-not-global-runtime-install",
    "D215.route-calls-stdlib-ranking-owner-not-old-wrapper",
    "D216.positive-route-rerun-preserves-owner-output",
    "D217.negative-held-rerun-is-load-bearing-for-route-binding",
    "D218.effect-scope-blocks-globalization-and-pair-split",
    "D219.rollback-diff-and-effect-audit-precede-runtime-adapter-install",
    "D220.tie-break-remains-explicit-not-route-cache-order",
    "D221.route-binding-creates-no-rigorfloor-route-cache-or-nix-checks-authority",
    "D222.runtime-adapter-install-remains-future-frontier",
  ] {
    let discovery = discoveries
      .get(expected)
      .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
    assert!(as_bool(get(discovery, "scenario-only")));
  }
}

#[test]
fn affected_plans_keep_adapter_install_and_global_authority_unimplemented() {
  let run = eval_file(&fixture_path()).unwrap();
  let affected = get(&run, "affected-plans");
  assert_eq!(
    as_str(get_path(affected, &["executableRouteBinding", "pressure"])),
    "owner-call-rerun-with-held-negatives"
  );
  assert_eq!(
    as_str(get_path(affected, &["runtimeAdapterInstall", "pressure"])),
    "still-deferred-after-route-binding"
  );
  for key in [
    "executableRouteBinding",
    "runtimeAdapterInstall",
    "evaluateSelectRankingOwner",
    "legacyEvaluateSelect",
    "RigorFloor",
    "routeCache",
    "globalRankingRuntime",
    "nixChecksGate",
  ] {
    assert!(!as_bool(get_path(
      affected,
      &[key, "implementation-target"]
    )));
  }
}

#[test]
fn negative_held_evidence_survives_route_binding() {
  let run = eval_file(&fixture_path()).unwrap();
  let negative = get(&run, "negative-held-evidence");
  assert_eq!(as_str(get(negative, "status")), "present");
  assert!(as_bool(get(negative, "survives-route-binding")));
  let rejects = string_set(get(negative, "rejects"));
  for expected in [
    "route-binding-before-install-readiness",
    "route-binding-without-stdlib-ranking-owner",
    "route-binding-without-positive-rerun",
    "route-binding-without-negative-held-rerun",
    "route-binding-without-rollback-dry-run",
    "route-binding-without-operator-visible-route-diff",
    "route-binding-with-global-effect-scope",
    "route-binding-restores-old-wrapper",
    "llm-prose-as-route-binding-proof",
    "hidden-tie-break-route-cache",
    "runtime-install-from-route-binding",
    "rigorfloor-routecache-authority-from-route-binding",
  ] {
    assert!(rejects.contains(expected), "missing rejection `{expected}`");
  }
}

#[test]
fn top_level_state_records_route_binding_without_runtime_install() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "replacement-readiness")),
    "surface-pair-executable-route-binding-pinned-non-installed"
  );
  assert!(as_bool(get(&run, "owner-switch")));
  assert!(as_bool(get(&run, "runtime-ranking-owner-receipt")));
  assert!(as_bool(get(&run, "runtime-ranking-install-readiness")));
  assert!(as_bool(get(&run, "surface-pair-executable-route-bound")));
  assert!(as_bool(get(&run, "executable-route-binding")));
  assert!(!as_bool(get(&run, "runtime-install")));
  assert!(!as_bool(get(&run, "ranking-runtime-install")));
  assert!(!as_bool(get(&run, "runtime-adapter-install")));
  assert!(!as_bool(get(&run, "global-ranking-runtime")));
  assert!(!as_bool(get(&run, "rigorfloor-authority")));
  assert!(!as_bool(get(&run, "route-cache-authority")));
  assert!(!as_bool(get(&run, "nix-checks-gate-added")));
  assert!(!as_bool(get(&run, "implementation-command")));
}
