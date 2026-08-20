//! R7 compat/archive decision for the macro-native evaluate/select pair.
//!
//! R6 switched semantic ownership for the dependent `ontologyEvaluate` /
//! `ontologySelect` pair. R7 decides what happens to the old pair after that
//! switch: retain it as compat/reference and ranking regression material, not
//! current authority and not deleted/archived.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join(
    "../../fixtures/tesseract-macro-legacy-probe/compat_archive_evaluate_select_surface_pair_receipt.px",
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
fn evalselect_r7_marker_and_constitution_owner_are_pinned() {
  let run = eval_file(&fixture_path()).expect("eval/select R7 compat fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-r7-compat-archive-evaluate-select-surface-pair"
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
fn constitution_gate_keeps_evalselect_r7_candidate_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "r7-compat-archive-evaluate-select-surface-pair"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  assert_eq!(
    as_str(get(gate, "replacement-readiness")),
    "compat-retained-for-evaluate-select-surface-pair"
  );

  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "delete-legacy-evaluate-select-because-owner-switched",
    "archive-legacy-evaluate-select-without-usage-scan",
    "drop-ranking-regression-corpus-after-green-owner-switch",
    "drop-null-held-regression-case-after-r6",
    "drop-tie-break-reference-delta-after-r6",
    "treat-compat-shell-as-current-authority",
    "install-ranking-runtime-route-at-r7",
    "globalize-evaluate-select-compat-decision",
    "split-evaluate-select-compat-without-proof",
    "restore-old-evaluate-select-wrapper",
    "treat-llm-cleanup-prose-as-delete-proof",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
}

#[test]
fn surface_pair_is_eval_select_only_and_uses_macro_owner() {
  let run = eval_file(&fixture_path()).unwrap();
  let pair = get(&run, "surface-pair");
  assert_eq!(
    as_str(get(pair, "id")),
    "surface-pair.legacy-ontology.evaluate-select"
  );
  assert_eq!(
    as_str(get(pair, "evaluate")),
    "stdlib/lib/ontology.px::builtins.ontologyEvaluate"
  );
  assert_eq!(
    as_str(get(pair, "select")),
    "stdlib/lib/ontology.px::builtins.ontologySelect"
  );
  assert_eq!(
    as_str(get(pair, "semantic-owner")),
    "macro-native.evaluate-select.surface-pair-owner"
  );
  assert_eq!(
    as_str(get(pair, "old-owner-role")),
    "reference-specimen-pair-and-ranking-regression-corpus"
  );
  assert_eq!(
    as_str(get(pair, "scope")),
    "legacy-evaluate-select-pair-only"
  );
  assert!(as_bool(get(pair, "pair-required")));
  assert!(!as_bool(get(pair, "select-only-compat")));
  assert!(!as_bool(get(pair, "split-proof-present")));
  assert!(!as_bool(get(pair, "other-surfaces-included")));
}

#[test]
fn r6_input_imports_pair_owner_switch_state() {
  let run = eval_file(&fixture_path()).unwrap();
  let input = get(&run, "r6-input");
  assert_eq!(
    as_str(get(input, "owner-switch-receipt")),
    "r6.owner-switch.evaluate-select-surface-pair"
  );
  assert_eq!(
    as_str(get(input, "replacement-readiness")),
    "owner-switched-for-evaluate-select-surface-pair"
  );
  assert!(as_bool(get(input, "owner-switch")));
  assert_eq!(
    as_str(get(input, "semantic-owner")),
    "macro-native.evaluate-select.surface-pair-owner"
  );
  assert!(!as_bool(get(input, "legacy-current-authority")));
  assert!(!as_bool(get(input, "runtime-install")));
  assert!(!as_bool(get(input, "ranking-runtime-install")));
  assert!(!as_bool(get(input, "global-ranking-runtime")));
  assert!(!as_bool(get(input, "rigorfloor-authority")));
  assert!(!as_bool(get(input, "route-cache-authority")));
  assert!(as_bool(get(input, "r7-required")));
  assert!(as_bool(get(input, "negative-held-survives")));
  assert!(as_bool(get(input, "audit-refs-preserved")));
  assert_eq!(
    as_str(get(input, "ranking-regression-corpus")),
    "ranking-regression-corpus.evaluate-select"
  );
}

#[test]
fn compat_decision_retains_legacy_pair_without_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  let decision = get(&run, "compat-decision");
  assert_eq!(
    as_str(get(decision, "id")),
    "r7.compat-retain.evaluate-select-surface-pair"
  );
  assert_eq!(as_str(get(decision, "phase")), "R7");
  assert_eq!(
    as_str(get(decision, "decision")),
    "retain-compat-reference-pair"
  );
  assert_eq!(
    as_str(get(decision, "compat-status")),
    "compat-retained-for-evaluate-select-surface-pair"
  );
  assert!(!as_bool(get(decision, "current-semantic-authority")));
  assert!(!as_bool(get(decision, "legacy-score-is-current-proof")));
  assert!(!as_bool(get(decision, "legacy-winner-is-current-proof")));
  assert!(!as_bool(get(decision, "null-held-is-success")));
  assert!(!as_bool(get(decision, "tie-break-is-route-cache")));
  assert!(!as_bool(get(decision, "old-evaluate-select-wrapper")));
  assert_eq!(as_str(get(decision, "compat-shell")), "candidate-only");
  assert_eq!(
    as_str(get(decision, "docs-role")),
    "historical-reference-pair"
  );
}

#[test]
fn compat_decision_blocks_runtime_global_split_delete_and_archive() {
  let run = eval_file(&fixture_path()).unwrap();
  let decision = get(&run, "compat-decision");
  assert!(!as_bool(get(decision, "compat-route-installed")));
  assert!(as_bool(get(decision, "ranking-regression-corpus-retained")));
  assert!(as_bool(get(decision, "reverse-replay-reference-retained")));
  assert!(as_bool(get(decision, "null-held-regression-retained")));
  assert!(as_bool(get(decision, "tie-break-reference-retained")));
  assert!(as_bool(get(decision, "supersede-chain-retained")));
  assert!(as_bool(get(decision, "rollback-evidence-retained")));
  assert!(!as_bool(get(decision, "delete-legacy-surfaces")));
  assert!(!as_bool(get(decision, "archive-legacy-surfaces")));
  assert!(!as_bool(get(decision, "runtime-install")));
  assert!(!as_bool(get(decision, "ranking-runtime-install")));
  assert!(!as_bool(get(decision, "global-ranking-runtime")));
  assert!(!as_bool(get(decision, "rigorfloor-authority")));
  assert!(!as_bool(get(decision, "route-cache-authority")));
  assert!(!as_bool(get(decision, "split-evaluate-select-owner")));
  assert!(!as_bool(get(decision, "other-surfaces-included")));
  assert!(!as_bool(get(decision, "implementation-command")));
}

#[test]
fn retention_policy_preserves_ranking_replay_null_and_tie_break_evidence() {
  let run = eval_file(&fixture_path()).unwrap();
  let policy = get(&run, "retention-policy");
  assert_eq!(
    as_str(get(policy, "id")),
    "retention.r7.evaluate-select-surface-pair"
  );
  let retained = string_set(get(policy, "retained-for"));
  for expected in [
    "ranking-regression-corpus",
    "reverse-replay-reference",
    "compat-shell-candidate-input",
    "supersede-chain-audit",
    "rollback-evidence",
    "null-held-regression-case",
    "tie-break-reference-delta",
    "score-axis-baseline",
    "winner-ranking-baseline",
    "historical-doc-reference",
  ] {
    assert!(
      retained.contains(expected),
      "missing retained role `{expected}`"
    );
  }
  assert!(!as_bool(get(policy, "can-be-called-as-current-authority")));
  assert!(as_bool(get(policy, "can-be-used-as-test-oracle")));
  assert!(as_bool(get(policy, "can-be-used-for-reverse-replay")));
  assert!(as_bool(get(policy, "can-be-used-for-rollback-analysis")));
  assert!(as_bool(get(policy, "can-be-used-for-null-held-regression")));
  assert!(as_bool(get(policy, "can-be-used-for-tie-break-regression")));
  assert!(!as_bool(get(policy, "delete-now")));
  assert!(!as_bool(get(policy, "archive-now")));
}

#[test]
fn archive_delete_gate_holds_without_ranking_specific_proof() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "archive-delete-gate");
  assert_eq!(
    as_str(get(gate, "id")),
    "gate.r7.archive-delete.evaluate-select-surface-pair"
  );
  assert_eq!(as_str(get(gate, "verdict")), "delete-and-archive-held");
  for key in [
    "delete-proof-present",
    "archive-proof-present",
    "usage-scan-complete",
    "external-caller-scan-complete",
    "replay-corpus-replacement-present",
    "ranking-corpus-replacement-present",
    "runtime-ranking-replacement-present",
    "split-proof-present",
    "rollback-plan-present",
    "human-consequence-authorization-present",
  ] {
    assert!(!as_bool(get(gate, key)), "`{key}` must stay false");
  }

  let required = string_set(get(gate, "required-before-delete-or-archive"));
  for expected in [
    "compat-usage-scan",
    "external-caller-scan",
    "replacement-replay-corpus",
    "replacement-ranking-regression-corpus",
    "runtime-ranking-owner-receipt-if-install-needed",
    "split-proof-if-pair-is-separated",
    "rollback-plan",
    "supersede-chain-audit",
    "human-consequence-authorization-if-consequence-bearing",
  ] {
    assert!(
      required.contains(expected),
      "missing archive/delete proof `{expected}`"
    );
  }
}

#[test]
fn compat_routes_are_candidates_not_current_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  let routes = attrs_by_id(get(&run, "compat-route-candidates"));
  assert_eq!(routes.len(), 4);
  for expected in [
    "compat.route.legacy-evaluate-to-macro-owner",
    "compat.route.legacy-select-to-macro-owner",
    "compat.route.ranking-regression-oracle",
    "compat.route.rollback-analysis",
  ] {
    let route = routes
      .get(expected)
      .unwrap_or_else(|| panic!("missing route `{expected}`"));
    assert!(!as_bool(get(route, "installed")));
    assert!(!as_bool(get(route, "current-authority")));
  }
  assert_eq!(
    as_str(get(
      routes
        .get("compat.route.legacy-evaluate-to-macro-owner")
        .unwrap(),
      "held-if"
    )),
    "runtime-ranking-owner-receipt-missing"
  );
  assert_eq!(
    as_str(get(
      routes
        .get("compat.route.ranking-regression-oracle")
        .unwrap(),
      "held-if"
    )),
    "used-as-current-proof"
  );
}

#[test]
fn r7_trials_hold_shortcuts_and_accept_pair_compat_retention() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "r7-trials"));
  assert_eq!(trials.len(), 12);
  for expected in [
    "trial.A.r6-owner-switch-missing",
    "trial.B.compat-policy-missing",
    "trial.C.legacy-score-current-proof",
    "trial.D.legacy-winner-current-proof",
    "trial.E.delete-without-proof",
    "trial.F.archive-without-usage-scan",
    "trial.G.runtime-install-requested",
    "trial.H.global-ranking-compat",
    "trial.I.split-compat-without-proof",
    "trial.J.old-wrapper-restored",
    "trial.K.llm-cleanup-delete",
  ] {
    let trial = trials
      .get(expected)
      .unwrap_or_else(|| panic!("missing trial `{expected}`"));
    assert_eq!(as_str(get(trial, "outcome")), "Held");
    assert!(!as_bool(get(trial, "delete-legacy-surfaces")));
    assert!(!as_bool(get(trial, "archive-legacy-surfaces")));
  }

  let complete = trials.get("trial.L.complete-compat-retain").unwrap();
  assert_eq!(
    as_str(get(complete, "outcome")),
    "compat-retained-for-evaluate-select-surface-pair"
  );
  assert_eq!(
    as_str(get(complete, "compat-status")),
    "compat-retained-for-evaluate-select-surface-pair"
  );
  assert!(!as_bool(get(complete, "delete-legacy-surfaces")));
  assert!(!as_bool(get(complete, "archive-legacy-surfaces")));
}

#[test]
fn six_layer_compat_fold_preserves_pair_and_runtime_boundaries() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-compat-fold");
  assert_eq!(
    as_str(get(fold, "mode")),
    "r7-compat-archive-evaluate-select-surface-pair"
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
    as_str(get_path(fold, &["surface", "compat-status"])),
    "compat-retained-for-evaluate-select-surface-pair"
  );
  assert!(as_bool(get_path(fold, &["surface", "pair-required"])));
  assert!(!as_bool(get_path(fold, &["surface", "select-only-compat"])));
  assert_eq!(
    as_str(get_path(fold, &["ontology", "old-pair-role"])),
    "compat-reference-pair"
  );
  assert!(!as_bool(get_path(
    fold,
    &["ontology", "global-ranking-runtime"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["ontology", "split-proof-present"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["semantic", "current-semantic-authority"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["semantic", "legacy-score-is-current-proof"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["semantic", "legacy-winner-is-current-proof"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["semantic", "null-held-is-success"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["semantic", "tie-break-is-route-cache"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "compat-route-is-candidate"]
  )));
  assert_eq!(
    as_str(get_path(fold, &["gate", "archive-delete-verdict"])),
    "delete-and-archive-held"
  );
  assert!(!as_bool(get_path(fold, &["runtime", "executable-now"])));
  assert!(!as_bool(get_path(fold, &["runtime", "installed"])));
  assert!(!as_bool(get_path(
    fold,
    &["runtime", "ranking-runtime-installed"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["runtime", "compat-route-installed"]
  )));
  assert!(as_bool(get_path(fold, &["audit", "audit-refs-preserved"])));
  assert!(as_bool(get_path(
    fold,
    &["audit", "negative-held-survives"]
  )));
  assert!(as_bool(get_path(fold, &["audit", "retained-for-replay"])));
}

#[test]
fn runtime_observation_is_compat_retained_but_not_installed() {
  let run = eval_file(&fixture_path()).unwrap();
  let runtime = get(&run, "runtime-observation");
  assert_eq!(
    as_str(get(runtime, "observation-model")),
    "r7-compat-retained-evaluate-select-surface-pair-non-installed-runtime"
  );
  assert!(as_bool(get(runtime, "can-appear-at-runtime")));
  assert!(as_bool(get(runtime, "owner-switch")));
  assert!(as_bool(get(runtime, "compat-retained")));
  assert!(!as_bool(get(runtime, "archive-legacy-surfaces")));
  assert!(!as_bool(get(runtime, "delete-legacy-surfaces")));
  assert!(!as_bool(get(runtime, "canonical-runtime-installed")));
  assert!(!as_bool(get(runtime, "executable-now")));
  assert!(!as_bool(get(runtime, "ranking-runtime-installed")));
  assert!(!as_bool(get(runtime, "global-ranking-runtime")));
  assert_eq!(as_list(get(runtime, "runtime-added-candidates")).len(), 3);
}

#[test]
fn discoveries_record_d179_through_d187() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 9);
  for expected in [
    "D179.evalselect-r7-retains-compat-reference-pair",
    "D180.legacy-evaluate-select-is-regression-corpus-not-current-authority",
    "D181.evalselect-compat-route-is-candidate-not-runtime-install",
    "D182.archive-delete-requires-ranking-specific-proof",
    "D183.null-held-and-tie-break-evidence-survive-r7",
    "D184.r7-preserves-pair-dependency-until-split-proof",
    "D185.r7-evalselect-does-not-create-rigorfloor-or-route-cache",
    "D186.cleanup-prose-cannot-delete-evaluate-select-pair",
    "D187.docs-can-be-historical-while-eval-select-code-remains-retained",
  ] {
    let discovery = discoveries
      .get(expected)
      .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
    assert!(as_bool(get(discovery, "scenario-only")));
  }
}

#[test]
fn affected_plans_keep_runtime_archive_split_and_other_surfaces_unimplemented() {
  let run = eval_file(&fixture_path()).unwrap();
  let affected = get(&run, "affected-plans");
  assert_eq!(
    as_str(get_path(affected, &["legacyEvaluateSelect", "pressure"])),
    "retain-after-owner-switch"
  );
  assert_eq!(
    as_str(get_path(affected, &["rankingRuntime", "pressure"])),
    "needs-runtime-ranking-owner-receipt-before-install"
  );
  assert_eq!(
    as_str(get_path(affected, &["splitEvaluateSelect", "pressure"])),
    "held-until-split-proof"
  );
  for key in [
    "legacyEvaluateSelect",
    "macroEvaluateSelect",
    "rankingRuntime",
    "RigorFloor",
    "routeCache",
    "archiveDelete",
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
fn negative_held_evidence_survives_r7_pair_compat() {
  let run = eval_file(&fixture_path()).unwrap();
  let negative = get(&run, "negative-held-evidence");
  assert_eq!(as_str(get(negative, "status")), "present");
  assert!(as_bool(get(negative, "survives-r7")));
  let rejects = string_set(get(negative, "rejects"));
  for expected in [
    "r7-before-r6-owner-switch",
    "r7-without-compat-policy",
    "legacy-score-as-current-proof-after-r7",
    "legacy-winner-as-current-proof-after-r7",
    "null-held-as-success-after-r7",
    "tie-break-as-route-cache-after-r7",
    "delete-without-archive-delete-proof",
    "archive-without-usage-scan",
    "runtime-install-from-r7-compat",
    "global-ranking-compat-from-surface-pair",
    "split-compat-without-proof",
    "old-wrapper-restored-from-r7",
    "llm-cleanup-prose-as-delete-proof",
  ] {
    assert!(rejects.contains(expected), "missing rejection `{expected}`");
  }
}

#[test]
fn blocked_shortcuts_cover_delete_archive_runtime_split_and_prose() {
  let run = eval_file(&fixture_path()).unwrap();
  let blocks = string_set(get(&run, "blocked-shortcuts"));
  for expected in [
    "delete-legacy-evaluate-select-because-owner-switched",
    "archive-legacy-evaluate-select-without-usage-scan",
    "drop-ranking-regression-corpus-after-green-owner-switch",
    "drop-reverse-replay-reference-after-r6",
    "drop-null-held-regression-case-after-r6",
    "drop-tie-break-reference-delta-after-r6",
    "treat-compat-shell-as-current-authority",
    "install-ranking-runtime-route-at-r7",
    "globalize-evaluate-select-compat-decision",
    "split-evaluate-select-compat-without-proof",
    "restore-old-evaluate-select-wrapper",
    "treat-llm-cleanup-prose-as-delete-proof",
  ] {
    assert!(
      blocks.contains(expected),
      "missing shortcut block `{expected}`"
    );
  }
}

#[test]
fn top_level_state_records_compat_retention_without_install_or_deletion() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "replacement-readiness")),
    "owner-switched-for-evaluate-select-surface-pair"
  );
  assert!(as_bool(get(&run, "owner-switch")));
  assert_eq!(
    as_str(get(&run, "compat-status")),
    "compat-retained-for-evaluate-select-surface-pair"
  );
  assert!(!as_bool(get(&run, "archive-legacy-surfaces")));
  assert!(!as_bool(get(&run, "delete-legacy-surfaces")));
  assert!(!as_bool(get(&run, "runtime-install")));
  assert!(!as_bool(get(&run, "ranking-runtime-install")));
  assert!(!as_bool(get(&run, "global-ranking-runtime")));
  assert!(!as_bool(get(&run, "rigorfloor-authority")));
  assert!(!as_bool(get(&run, "route-cache-authority")));
  assert!(!as_bool(get(&run, "implementation-command")));
}
