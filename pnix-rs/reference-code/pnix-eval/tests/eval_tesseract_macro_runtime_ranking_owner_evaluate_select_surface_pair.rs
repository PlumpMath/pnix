//! Runtime ranking owner contract for the macro-native evaluate/select pair.
//!
//! R7 retained the old `ontologyEvaluate` / `ontologySelect` pair as compat
//! reference and ranking regression corpus. This test pins the next step as a
//! non-installed runtime ranking owner contract: explicit candidate set plus
//! axis evidence in, replayable ranking / winner / Held out. It does not
//! install an executable runtime.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join(
    "../../fixtures/tesseract-macro-legacy-probe/runtime_ranking_owner_evaluate_select_surface_pair_receipt.px",
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
fn runtime_owner_marker_and_constitution_owner_are_pinned() {
  let run = eval_file(&fixture_path()).expect("runtime ranking owner fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-runtime-ranking-owner-evaluate-select-surface-pair"
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
fn constitution_gate_keeps_runtime_owner_contract_candidate_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "runtime-ranking-owner-evaluate-select-surface-pair"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  assert_eq!(
    as_str(get(gate, "replacement-readiness")),
    "runtime-ranking-owner-contract-pinned-non-installed"
  );

  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "install-ranking-runtime-from-owner-contract",
    "treat-runtime-owner-contract-as-executable",
    "use-legacy-select-winner-as-current-proof",
    "use-llm-prose-as-ranking-judgement",
    "default-select-first-candidate-on-null-evidence",
    "hide-tie-break-inside-route-cache",
    "promote-score-axis-to-rigorfloor-authority",
    "globalize-evaluate-select-ranking-owner",
    "split-select-away-from-evaluate-without-proof",
    "restore-old-evaluate-select-wrapper-as-runtime-owner",
    "drop-r7-ranking-regression-corpus",
    "treat-green-tests-as-install-receipt",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
}

#[test]
fn surface_pair_scope_survives_runtime_owner_contract() {
  let run = eval_file(&fixture_path()).unwrap();
  let pair = get(&run, "surface-pair");
  assert_eq!(
    as_str(get(pair, "id")),
    "surface-pair.legacy-ontology.evaluate-select"
  );
  assert_eq!(
    as_str(get(pair, "semantic-owner")),
    "macro-native.evaluate-select.surface-pair-owner"
  );
  assert_eq!(
    as_str(get(pair, "scope")),
    "legacy-evaluate-select-pair-only"
  );
  assert!(as_bool(get(pair, "pair-required")));
  assert!(!as_bool(get(pair, "split-proof-present")));
  assert!(!as_bool(get(pair, "other-surfaces-included")));
}

#[test]
fn r7_input_keeps_compat_corpus_without_runtime_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  let input = get(&run, "r7-input");
  assert_eq!(
    as_str(get(input, "source")),
    "compat_archive_evaluate_select_surface_pair_receipt.px"
  );
  assert_eq!(
    as_str(get(input, "compat-status")),
    "compat-retained-for-evaluate-select-surface-pair"
  );
  assert!(as_bool(get(input, "owner-switch")));
  assert!(as_bool(get(input, "compat-retained")));
  assert!(!as_bool(get(input, "legacy-current-authority")));
  assert!(as_bool(get(input, "ranking-regression-corpus-retained")));
  assert!(as_bool(get(input, "null-held-regression-retained")));
  assert!(as_bool(get(input, "tie-break-reference-retained")));
  assert!(!as_bool(get(input, "runtime-install")));
  assert!(!as_bool(get(input, "ranking-runtime-install")));
  assert!(!as_bool(get(input, "global-ranking-runtime")));
  assert!(!as_bool(get(input, "rigorfloor-authority")));
  assert!(!as_bool(get(input, "route-cache-authority")));
}

#[test]
fn ranking_owner_contract_closes_frontier_without_installing_runtime() {
  let run = eval_file(&fixture_path()).unwrap();
  let contract = get(&run, "ranking-owner-contract");
  assert_eq!(
    as_str(get(contract, "id")),
    "owner.runtime-ranking.evaluate-select.surface-pair"
  );
  assert_eq!(
    as_str(get(contract, "contract-status")),
    "pinned-non-installed"
  );
  assert_eq!(
    as_str(get(contract, "closes-frontier")),
    "need.runtime-ranking-owner-receipt-before-install"
  );
  assert_eq!(
    as_str(get(contract, "opens-frontier")),
    "need.runtime-ranking-install-receipt-if-execution-needed"
  );
  assert_eq!(
    as_str(get(contract, "input-kind")),
    "candidate-set-plus-axis-evidence"
  );
  assert_eq!(
    as_str(get(contract, "output-kind")),
    "candidate-ranking-plus-winner-or-held"
  );
  assert!(as_bool(get(contract, "candidate-set-required")));
  assert!(as_bool(get(contract, "axis-evidence-vector-required")));
  assert!(as_bool(get(contract, "axis-order-explicit")));
  assert!(as_bool(get(contract, "score-axis-is-evidence")));
  assert!(as_bool(get(contract, "winner-is-ranking-output")));
  assert!(!as_bool(get(contract, "runtime-install")));
  assert!(!as_bool(get(contract, "executable-now")));
  assert!(!as_bool(get(contract, "ranking-runtime-install")));
  assert!(!as_bool(get(contract, "global-ranking-runtime")));
  assert!(!as_bool(get(contract, "implementation-command")));
}

#[test]
fn ranking_owner_blocks_legacy_llm_rigorfloor_routecache_and_split_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  let contract = get(&run, "ranking-owner-contract");
  assert!(as_bool(get(contract, "null-candidate-set-emits-held")));
  assert!(as_bool(get(contract, "missing-axis-emits-held")));
  assert!(as_bool(get(contract, "hidden-tie-break-emits-held")));
  assert!(!as_bool(get(contract, "old-winner-is-current-proof")));
  assert!(!as_bool(get(contract, "llm-prose-can-choose-winner")));
  assert!(!as_bool(get(contract, "legacy-current-authority")));
  assert!(!as_bool(get(contract, "rigorfloor-authority")));
  assert!(!as_bool(get(contract, "route-cache-authority")));
  assert!(!as_bool(get(contract, "old-evaluate-select-wrapper")));
  assert!(!as_bool(get(contract, "split-evaluate-select-owner")));
}

#[test]
fn ranking_law_requires_explicit_replayable_inputs_and_outputs() {
  let run = eval_file(&fixture_path()).unwrap();
  let law = get(&run, "ranking-law");
  assert_eq!(
    as_str(get(law, "id")),
    "law.runtime-ranking.evaluate-select.surface-pair"
  );
  let input_fields = string_set(get(law, "input-fields"));
  for expected in [
    "candidate-id",
    "candidate-provenance-ref",
    "axis-evidence-vector",
    "axis-order",
    "comparison-baseline-ref",
    "tie-break-rule-ref",
    "audit-ref",
  ] {
    assert!(
      input_fields.contains(expected),
      "missing input field `{expected}`"
    );
  }
  let output_fields = string_set(get(law, "output-fields"));
  for expected in [
    "ranking",
    "winner-candidate-id",
    "winner-reason",
    "held-id",
    "tie-break-ref",
    "regression-corpus-ref",
    "audit-ref",
  ] {
    assert!(
      output_fields.contains(expected),
      "missing output field `{expected}`"
    );
  }
}

#[test]
fn ranking_law_fail_closed_cases_cover_subtle_neutering_points() {
  let run = eval_file(&fixture_path()).unwrap();
  let law = get(&run, "ranking-law");
  let requirements = string_set(get(law, "deterministic-requirements"));
  for expected in [
    "stable-candidate-identity",
    "explicit-axis-order",
    "no-hidden-default-winner",
    "replayable-tie-break",
    "old-corpus-compared-as-regression-not-authority",
    "negative-held-preserved",
  ] {
    assert!(
      requirements.contains(expected),
      "missing deterministic requirement `{expected}`"
    );
  }
  let held = string_set(get(law, "fail-closed-cases"));
  for expected in [
    "candidate-set-empty",
    "candidate-id-unstable",
    "axis-evidence-missing",
    "axis-order-missing",
    "tie-break-hidden",
    "legacy-winner-used-as-proof",
    "llm-prose-used-as-winner",
    "route-cache-claims-authority",
  ] {
    assert!(
      held.contains(expected),
      "missing fail-closed case `{expected}`"
    );
  }
}

#[test]
fn regression_binding_uses_old_pair_as_oracle_not_current_proof() {
  let run = eval_file(&fixture_path()).unwrap();
  let binding = get(&run, "regression-binding");
  assert_eq!(
    as_str(get(binding, "id")),
    "binding.runtime-ranking.regression-corpus.evaluate-select"
  );
  assert_eq!(
    as_str(get(binding, "corpus-role")),
    "regression-oracle-not-current-authority"
  );
  assert!(as_bool(get(binding, "ranking-regression-corpus-retained")));
  assert!(as_bool(get(binding, "reverse-replay-reference-retained")));
  assert!(as_bool(get(binding, "null-held-regression-retained")));
  assert!(as_bool(get(binding, "tie-break-reference-retained")));
  assert!(as_bool(get(binding, "can-fail-new-owner")));
  assert!(!as_bool(get(binding, "can-prove-current-winner-alone")));
  assert!(!as_bool(get(binding, "can-delete-old-pair")));
  assert!(!as_bool(get(binding, "can-install-runtime")));
}

#[test]
fn install_gate_keeps_execution_held_after_owner_contract() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "install-gate");
  assert_eq!(
    as_str(get(gate, "id")),
    "gate.runtime-ranking.install.evaluate-select.surface-pair"
  );
  assert!(as_bool(get(gate, "owner-contract-present")));
  assert_eq!(as_str(get(gate, "verdict")), "install-held");
  for key in [
    "install-receipt-present",
    "executable-route-present",
    "fresh-replay-corpus-present",
    "negative-held-corpus-present",
    "rollback-plan-present",
    "performance-budget-present",
    "human-consequence-authorization-present",
  ] {
    assert!(!as_bool(get(gate, key)), "`{key}` must stay false");
  }
  let required = string_set(get(gate, "required-before-install"));
  for expected in [
    "runtime-install-receipt",
    "fresh-replay-corpus",
    "negative-held-corpus",
    "ranking-regression-corpus-check",
    "deterministic-tie-break-check",
    "rollback-plan",
    "effect-scope",
    "human-consequence-authorization-if-consequence-bearing",
  ] {
    assert!(
      required.contains(expected),
      "missing install proof `{expected}`"
    );
  }
}

#[test]
fn owner_trials_hold_shortcuts_and_pin_complete_contract() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "owner-trials"));
  assert_eq!(trials.len(), 11);
  for expected in [
    "trial.A.r7-compat-missing",
    "trial.B.candidate-set-missing",
    "trial.C.axis-vector-missing",
    "trial.D.null-candidate-success",
    "trial.E.hidden-tie-break",
    "trial.F.legacy-winner-current-proof",
    "trial.G.llm-prose-winner",
    "trial.H.global-ranking-runtime",
    "trial.I.rigorfloor-routecache-authority",
    "trial.J.install-requested",
  ] {
    let trial = trials
      .get(expected)
      .unwrap_or_else(|| panic!("missing trial `{expected}`"));
    assert_eq!(as_str(get(trial, "outcome")), "Held");
    assert!(!as_bool(get(trial, "runtime-install")));
  }

  let complete = trials.get("trial.K.contract-complete").unwrap();
  assert_eq!(
    as_str(get(complete, "outcome")),
    "runtime-ranking-owner-contract-pinned"
  );
  assert_eq!(as_str(get(complete, "held-id")), "none");
  assert!(!as_bool(get(complete, "runtime-install")));
}

#[test]
fn six_layer_owner_fold_preserves_runtime_boundary() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-owner-fold");
  assert_eq!(
    as_str(get(fold, "mode")),
    "runtime-ranking-owner-evaluate-select-surface-pair"
  );
  for layer in [
    "surface", "ontology", "semantic", "gate", "runtime", "audit",
  ] {
    assert!(
      as_bool(get_path(fold, &[layer, "visible"])),
      "layer `{layer}` must stay visible"
    );
  }
  assert!(as_bool(get_path(fold, &["surface", "pair-required"])));
  assert!(!as_bool(get_path(
    fold,
    &["surface", "split-proof-present"]
  )));
  assert_eq!(
    as_str(get_path(fold, &["ontology", "owner-contract"])),
    "owner.runtime-ranking.evaluate-select.surface-pair"
  );
  assert!(!as_bool(get_path(
    fold,
    &["ontology", "global-ranking-runtime"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "score-axis-is-evidence"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "winner-is-ranking-output"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["semantic", "old-winner-is-current-proof"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["semantic", "llm-prose-can-choose-winner"]
  )));
  assert_eq!(
    as_str(get_path(fold, &["gate", "install-verdict"])),
    "install-held"
  );
  assert!(as_bool(get_path(
    fold,
    &["runtime", "owner-contract-pinned"]
  )));
  assert!(!as_bool(get_path(fold, &["runtime", "executable-now"])));
  assert!(!as_bool(get_path(
    fold,
    &["runtime", "ranking-runtime-installed"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["audit", "negative-held-survives"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["audit", "tie-break-reference-retained"]
  )));
}

#[test]
fn runtime_observation_is_owner_contract_visible_but_not_installed() {
  let run = eval_file(&fixture_path()).unwrap();
  let runtime = get(&run, "runtime-observation");
  assert_eq!(
    as_str(get(runtime, "observation-model")),
    "runtime-ranking-owner-contract-evaluate-select-non-installed"
  );
  assert!(as_bool(get(runtime, "can-appear-at-runtime")));
  assert!(as_bool(get(runtime, "owner-contract-pinned")));
  assert!(!as_bool(get(runtime, "canonical-runtime-installed")));
  assert!(!as_bool(get(runtime, "executable-now")));
  assert!(!as_bool(get(runtime, "ranking-runtime-installed")));
  assert!(!as_bool(get(runtime, "global-ranking-runtime")));
  assert_eq!(as_list(get(runtime, "runtime-added-candidates")).len(), 4);
  let candidates = attrs_by_id(get(runtime, "runtime-added-candidates"));
  assert_eq!(
    as_str(get(
      candidates
        .get("runtime.ranking-owner.install-gate")
        .unwrap(),
      "status"
    )),
    "Held"
  );
}

#[test]
fn discoveries_record_d188_through_d196() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 9);
  for expected in [
    "D188.runtime-ranking-owner-is-contract-not-install",
    "D189.ranking-owner-consumes-candidate-set-and-axis-evidence",
    "D190.null-or-missing-ranking-input-is-held",
    "D191.tie-break-must-be-explicit-and-replayable",
    "D192.retained-legacy-ranking-corpus-is-regression-not-authority",
    "D193.llm-prose-cannot-choose-ranking-winner",
    "D194.rigorfloor-routecache-and-global-ranking-remain-blocked",
    "D195.install-requires-future-runtime-install-receipt",
    "D196.evaluate-select-pair-boundary-survives-runtime-owner-contract",
  ] {
    let discovery = discoveries
      .get(expected)
      .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
    assert!(as_bool(get(discovery, "scenario-only")));
  }
}

#[test]
fn affected_plans_keep_install_global_and_store_authority_unimplemented() {
  let run = eval_file(&fixture_path()).unwrap();
  let affected = get(&run, "affected-plans");
  assert_eq!(
    as_str(get_path(affected, &["runtimeRankingOwner", "pressure"])),
    "pinned-before-runtime-install"
  );
  assert_eq!(
    as_str(get_path(affected, &["rankingRuntimeInstall", "pressure"])),
    "held-until-fresh-replay-negative-corpus-rollback-effect-scope"
  );
  for key in [
    "runtimeRankingOwner",
    "rankingRuntimeInstall",
    "legacyEvaluateSelect",
    "RigorFloor",
    "routeCache",
    "globalRankingRuntime",
    "splitEvaluateSelect",
    "otherOntologySurfaces",
  ] {
    assert!(!as_bool(get_path(
      affected,
      &[key, "implementation-target"]
    )));
  }
}

#[test]
fn negative_held_evidence_survives_runtime_owner_contract() {
  let run = eval_file(&fixture_path()).unwrap();
  let negative = get(&run, "negative-held-evidence");
  assert_eq!(as_str(get(negative, "status")), "present");
  assert!(as_bool(get(negative, "survives-runtime-owner-contract")));
  let rejects = string_set(get(negative, "rejects"));
  for expected in [
    "runtime-owner-before-r7-compat",
    "ranking-without-candidate-set",
    "ranking-without-axis-vector",
    "null-candidate-as-success",
    "hidden-tie-break-route-cache",
    "legacy-winner-as-current-proof",
    "llm-prose-as-winner",
    "global-ranking-runtime-from-pair-owner",
    "rigorfloor-authority-from-score-axis",
    "route-cache-authority-from-tie-break",
    "runtime-install-from-owner-contract",
    "select-split-without-proof",
    "green-tests-as-install-receipt",
  ] {
    assert!(rejects.contains(expected), "missing rejection `{expected}`");
  }
}

#[test]
fn blocked_shortcuts_cover_install_prose_legacy_and_store_collapses() {
  let run = eval_file(&fixture_path()).unwrap();
  let blocks = string_set(get(&run, "blocked-shortcuts"));
  for expected in [
    "install-ranking-runtime-from-owner-contract",
    "treat-runtime-owner-contract-as-executable",
    "use-legacy-select-winner-as-current-proof",
    "use-llm-prose-as-ranking-judgement",
    "default-select-first-candidate-on-null-evidence",
    "hide-tie-break-inside-route-cache",
    "promote-score-axis-to-rigorfloor-authority",
    "globalize-evaluate-select-ranking-owner",
    "split-select-away-from-evaluate-without-proof",
    "restore-old-evaluate-select-wrapper-as-runtime-owner",
    "drop-r7-ranking-regression-corpus",
    "treat-green-tests-as-install-receipt",
  ] {
    assert!(
      blocks.contains(expected),
      "missing shortcut block `{expected}`"
    );
  }
}

#[test]
fn top_level_state_records_owner_contract_without_runtime_install() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "replacement-readiness")),
    "runtime-ranking-owner-contract-pinned-non-installed"
  );
  assert!(as_bool(get(&run, "owner-switch")));
  assert_eq!(
    as_str(get(&run, "compat-status")),
    "compat-retained-for-evaluate-select-surface-pair"
  );
  assert!(as_bool(get(&run, "runtime-ranking-owner-receipt")));
  assert_eq!(
    as_str(get(&run, "runtime-ranking-owner-contract")),
    "pinned-non-installed"
  );
  assert!(!as_bool(get(&run, "runtime-install")));
  assert!(!as_bool(get(&run, "ranking-runtime-install")));
  assert!(!as_bool(get(&run, "global-ranking-runtime")));
  assert!(!as_bool(get(&run, "rigorfloor-authority")));
  assert!(!as_bool(get(&run, "route-cache-authority")));
  assert!(!as_bool(get(&run, "implementation-command")));
}
