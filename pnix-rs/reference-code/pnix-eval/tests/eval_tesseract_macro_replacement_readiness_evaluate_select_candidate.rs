//! Surface-pair replacement readiness for the macro-native evaluate/select candidate.
//!
//! R5 verified paired reverse replay for the R4 candidate. This test pins the
//! next boundary: readiness may aggregate D4-D6 and R3/R4/R5 evidence and open
//! R6 owner-switch review, but it still cannot switch owners, install ranking
//! runtime, globalize readiness, or turn score/winner into authority.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join(
    "../../fixtures/tesseract-macro-legacy-probe/replacement_readiness_evaluate_select_candidate_receipt.px",
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
fn evalselect_readiness_marker_and_constitution_owner_are_pinned() {
  let run = eval_file(&fixture_path()).expect("eval/select readiness fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-surface-pair-replacement-readiness-evaluate-select-candidate"
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
fn constitution_gate_allows_readiness_without_acceptance_or_runtime() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "surface-pair-replacement-readiness-evaluate-select-candidate"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  assert_eq!(
    as_str(get(gate, "replacement-readiness")),
    "ready-for-r6-owner-switch-receipt"
  );

  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "treat-readiness-as-owner-switch",
    "treat-readiness-as-ranking-runtime-install",
    "claim-global-ranking-readiness",
    "skip-r5-reverse-replay",
    "replay-select-without-evaluate",
    "ignore-uncovered-delta",
    "emit-score-as-RigorFloor",
    "emit-tie-break-as-route-cache",
    "emit-winner-as-current-proof",
    "treat-null-select-as-success",
    "treat-llm-prose-as-readiness",
  ] {
    assert!(blocks.contains(expected), "missing gate block `{expected}`");
  }
}

#[test]
fn readiness_is_surface_pair_scoped_to_evaluate_select() {
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
    as_str(get(pair, "scope")),
    "legacy-evaluate-select-pair-only"
  );
  assert!(as_bool(get(pair, "pair-required")));
  assert!(!as_bool(get(pair, "global-ranking-runtime")));
}

#[test]
fn evidence_bundle_imports_r5_pair_replay_state() {
  let run = eval_file(&fixture_path()).unwrap();
  let evidence = get(&run, "evidence-bundle");
  assert_eq!(
    as_str(get(evidence, "evaluate-specimen")),
    "legacy-replay-specimen.evaluate.six-axis"
  );
  assert_eq!(
    as_str(get(evidence, "select-specimen")),
    "legacy-replay-specimen.select.tie-break"
  );
  assert_eq!(
    as_str(get(evidence, "r4-candidate")),
    "r4.macro-native-evaluate-select.rewrite-candidate"
  );
  assert_eq!(
    as_str(get(evidence, "r5-verdict")),
    "reverse-replay-verified"
  );
  assert!(as_bool(get(evidence, "paired-replay")));
  assert!(as_bool(get(evidence, "all-deltas-covered")));
  assert!(as_bool(get(evidence, "score-axis-covered")));
  assert!(as_bool(get(evidence, "winner-ranking-covered")));
  assert!(as_bool(get(evidence, "empty-selection-held-covered")));
  assert!(as_bool(get(evidence, "tie-break-delta-covered")));
  assert!(!as_bool(get(evidence, "unexplained-mismatch")));
  assert!(as_bool(get(evidence, "audit-refs-preserved")));
  assert!(as_bool(get(evidence, "negative-held-proof-present")));
  assert!(!as_bool(get(evidence, "owner-switch-before-readiness")));
}

#[test]
fn all_readiness_criteria_are_satisfied_without_splitting_pair() {
  let run = eval_file(&fixture_path()).unwrap();
  let criteria = attrs_by_id(get(&run, "readiness-criteria"));
  assert_eq!(criteria.len(), 13);
  for expected in [
    "criteria.six-layers-visible",
    "criteria.paired-replay-present",
    "criteria.score-axis-covered",
    "criteria.winner-ranking-covered",
    "criteria.null-held-covered",
    "criteria.tie-break-delta-covered",
    "criteria.no-unexplained-mismatch",
    "criteria.legacy-authority-blocked",
    "criteria.negative-path-present",
    "criteria.audit-refs-preserved",
    "criteria.ranking-regression-corpus-bound",
    "criteria.runtime-route-proof-non-executable",
    "criteria.docs-and-discovery-recorded",
  ] {
    let item = criteria
      .get(expected)
      .unwrap_or_else(|| panic!("missing criteria `{expected}`"));
    assert!(as_bool(get(item, "satisfied")));
    assert_eq!(as_str(get(item, "verdict")), "satisfied");
  }
}

#[test]
fn ranking_regression_corpus_is_bound_but_not_installed() {
  let run = eval_file(&fixture_path()).unwrap();
  let corpus = get(&run, "ranking-regression-corpus");
  assert_eq!(
    as_str(get(corpus, "id")),
    "ranking-regression-corpus.evaluate-select"
  );
  assert_eq!(
    as_str(get(corpus, "corpus-kind")),
    "surface-pair-replay-regression-corpus"
  );
  assert_eq!(as_list(get(corpus, "covered-deltas")).len(), 6);
  let held = string_set(get(corpus, "held-regression-cases"));
  for expected in [
    "score-axis-mismatch",
    "winner-ranking-mismatch",
    "null-held-missing",
    "tie-break-as-route-cache",
    "select-only-replay",
  ] {
    assert!(
      held.contains(expected),
      "missing held corpus case `{expected}`"
    );
  }
  assert!(!as_bool(get(corpus, "installed")));
  assert!(!as_bool(get(corpus, "executable-now")));
}

#[test]
fn runtime_route_proof_is_non_executable_and_not_ranking_runtime() {
  let run = eval_file(&fixture_path()).unwrap();
  let proof = get(&run, "runtime-route-proof");
  assert_eq!(
    as_str(get(proof, "proof-kind")),
    "non-executable-ranking-route-proof"
  );
  assert_eq!(
    as_str(get(proof, "verdict")),
    "runtime-route-proof-candidate-verified"
  );
  assert!(!as_bool(get(proof, "installed")));
  assert!(!as_bool(get(proof, "executable-now")));
  assert!(!as_bool(get(proof, "ranking-runtime-installed")));
  assert!(!as_bool(get(proof, "owner-switch")));
}

#[test]
fn owner_law_readiness_opens_r6_review_not_owner_switch() {
  let run = eval_file(&fixture_path()).unwrap();
  let owner = get(&run, "owner-law-readiness");
  assert!(as_bool(get(owner, "all-criteria-satisfied")));
  assert!(as_bool(get(
    owner,
    "consequence-gate-required-if-consequence-bearing"
  )));
  assert_eq!(as_str(get(owner, "owner-law-gate")), "ready-for-r6-review");
  assert_eq!(
    as_str(get(owner, "verdict")),
    "owner-law-ready-for-r6-owner-switch-receipt"
  );
  assert!(!as_bool(get(owner, "accepted")));
  assert!(!as_bool(get(owner, "owner-switch")));
}

#[test]
fn readiness_verdict_does_not_install_switch_globalize_or_create_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  let verdict = get(&run, "readiness-verdict");
  assert_eq!(
    as_str(get(verdict, "readiness")),
    "ready-for-r6-owner-switch-receipt"
  );
  assert_eq!(
    as_str(get(verdict, "scope")),
    "legacy-evaluate-select-pair-only"
  );
  assert!(!as_bool(get(verdict, "owner-switch")));
  assert!(!as_bool(get(verdict, "runtime-install")));
  assert!(!as_bool(get(verdict, "ranking-runtime-install")));
  assert!(!as_bool(get(verdict, "global-ranking-runtime")));
  assert!(!as_bool(get(verdict, "rigorfloor-authority")));
  assert!(!as_bool(get(verdict, "route-cache-authority")));
  assert!(!as_bool(get(verdict, "delete-legacy-surfaces")));
  assert!(!as_bool(get(verdict, "archive-legacy-surfaces")));

  let required = string_set(get(verdict, "next-required"));
  for expected in [
    "r6-owner-switch-receipt",
    "human-consequence-authorization-if-consequence-bearing",
    "runtime-ranking-owner-receipt-after-owner-switch",
    "compat-or-archive-decision-after-owner-switch",
  ] {
    assert!(
      required.contains(expected),
      "missing next requirement `{expected}`"
    );
  }
}

#[test]
fn held_trials_block_readiness_shortcuts() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "held-readiness-trials"));
  assert_eq!(trials.len(), 10);
  for expected in [
    "trial.A.reverse-replay-not-verified",
    "trial.B.select-only-readiness",
    "trial.C.uncovered-delta",
    "trial.D.audit-ref-missing",
    "trial.E.negative-held-missing",
    "trial.F.runtime-route-proof-missing",
    "trial.G.ranking-regression-corpus-missing",
    "trial.H.owner-switch-requested",
    "trial.I.global-runtime-requested",
  ] {
    let trial = trials
      .get(expected)
      .unwrap_or_else(|| panic!("missing trial `{expected}`"));
    assert_eq!(as_str(get(trial, "outcome")), "Held");
    assert!(!as_bool(get(trial, "owner-switch")));
  }
  let complete = trials.get("trial.J.complete-readiness").unwrap();
  assert_eq!(
    as_str(get(complete, "outcome")),
    "ready-for-r6-owner-switch-receipt"
  );
  assert!(!as_bool(get(complete, "owner-switch")));
}

#[test]
fn six_layer_readiness_fold_preserves_pair_authority_and_runtime_boundaries() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-readiness-fold");
  assert_eq!(
    as_str(get(fold, "mode")),
    "surface-pair-replacement-readiness-evaluate-select-candidate"
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
  assert_eq!(
    as_str(get_path(fold, &["ontology", "readiness-scope"])),
    "surface-pair-scoped"
  );
  assert!(!as_bool(get_path(
    fold,
    &["ontology", "global-ranking-runtime"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["ontology", "rigorfloor-authority"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["ontology", "route-cache-authority"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["semantic", "readiness-is-owner-switch"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["semantic", "score-is-current-proof"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["semantic", "winner-is-current-proof"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["semantic", "null-select-is-success"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["runtime", "ranking-runtime-installed"]
  )));
  assert!(as_bool(get_path(fold, &["audit", "audit-refs-preserved"])));
}

#[test]
fn runtime_observation_is_candidate_only_and_not_installed() {
  let run = eval_file(&fixture_path()).unwrap();
  let runtime = get(&run, "runtime-observation");
  assert_eq!(
    as_str(get(runtime, "observation-model")),
    "evalselect-readiness-runtime-candidates"
  );
  assert!(as_bool(get(runtime, "can-appear-at-runtime")));
  assert!(as_bool(get(runtime, "candidate-only")));
  assert!(!as_bool(get(runtime, "canonical-runtime-installed")));
  assert!(!as_bool(get(runtime, "executable-now")));
  assert!(!as_bool(get(runtime, "owner-switch")));
  assert!(!as_bool(get(runtime, "ranking-runtime-installed")));
  assert_eq!(as_list(get(runtime, "runtime-added-candidates")).len(), 3);
}

#[test]
fn discoveries_record_d161_through_d169() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 9);
  for expected in [
    "D161.evalselect-readiness-is-surface-pair-scoped",
    "D162.evalselect-readiness-aggregates-d4-through-r5",
    "D163.evalselect-readiness-keeps-pair-dependency-load-bearing",
    "D164.score-winner-null-tie-break-criteria-are-load-bearing",
    "D165.ranking-regression-corpus-binding-precedes-owner-switch",
    "D166.evalselect-runtime-route-proof-is-non-executable",
    "D167.evalselect-readiness-opens-r6-without-switching",
    "D168.evalselect-readiness-preserves-held-and-rewrite-debt",
    "D169.evalselect-readiness-is-receipt-driven-not-prose",
  ] {
    let discovery = discoveries
      .get(expected)
      .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
    assert!(as_bool(get(discovery, "scenario-only")));
  }
}

#[test]
fn affected_plans_remain_non_implementation_targets() {
  let run = eval_file(&fixture_path()).unwrap();
  let affected = get(&run, "affected-plans");
  assert_eq!(
    as_str(get_path(affected, &["RigorFloor", "pressure"])),
    "held-score-readiness-is-evidence-not-floor"
  );
  assert_eq!(
    as_str(get_path(affected, &["routeCache", "pressure"])),
    "held-tie-break-readiness-is-delta-not-cache"
  );
  assert_eq!(
    as_str(get_path(affected, &["evaluateSelectRewrite", "pressure"])),
    "ready-for-r6-owner-switch-receipt"
  );
  assert_eq!(
    as_str(get_path(affected, &["ownerSwitch", "pressure"])),
    "may-start-r6-but-not-claimed-here"
  );
  for key in [
    "RigorFloor",
    "routeCache",
    "rankingRuntime",
    "evaluateSelectRewrite",
    "ownerSwitch",
  ] {
    assert!(!as_bool(get_path(
      affected,
      &[key, "implementation-target"]
    )));
  }
}

#[test]
fn negative_held_evidence_blocks_readiness_shortcuts() {
  let run = eval_file(&fixture_path()).unwrap();
  let negative = get(&run, "negative-held-evidence");
  assert_eq!(as_str(get(negative, "status")), "present");
  let rejects = string_set(get(negative, "rejects"));
  for expected in [
    "readiness-without-r5-replay",
    "select-only-readiness",
    "readiness-with-uncovered-delta",
    "readiness-without-audit-ref",
    "readiness-without-negative-held-proof",
    "readiness-without-regression-corpus",
    "owner-switch-inside-readiness",
    "ranking-runtime-install-inside-readiness",
    "global-ranking-readiness-from-surface-pair",
    "score-as-rigorfloor-from-readiness",
    "tie-break-as-route-cache-from-readiness",
    "llm-prose-as-readiness",
  ] {
    assert!(rejects.contains(expected), "missing rejection `{expected}`");
  }
}

#[test]
fn blocked_shortcuts_cover_readiness_collapse_modes() {
  let run = eval_file(&fixture_path()).unwrap();
  let blocks = string_set(get(&run, "blocked-shortcuts"));
  for expected in [
    "treat-readiness-as-owner-switch",
    "treat-readiness-as-ranking-runtime-install",
    "claim-global-ranking-readiness",
    "skip-r5-reverse-replay",
    "replay-select-without-evaluate",
    "emit-score-as-RigorFloor",
    "emit-tie-break-as-route-cache",
    "emit-winner-as-current-proof",
    "treat-null-select-as-success",
    "treat-llm-prose-as-readiness",
  ] {
    assert!(
      blocks.contains(expected),
      "missing shortcut block `{expected}`"
    );
  }
}

#[test]
fn readiness_receipt_sets_readiness_without_owner_switch_runtime_or_command() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "replacement-readiness")),
    "ready-for-r6-owner-switch-receipt"
  );
  assert!(!as_bool(get(&run, "owner-switch")));
  assert!(!as_bool(get(&run, "runtime-install")));
  assert!(!as_bool(get(&run, "global-ranking-runtime")));
  assert!(!as_bool(get(&run, "implementation-command")));
}
