//! R4 macro-native evaluate/select rewrite candidate discovery.
//!
//! R3 pinned role emission for the dependent `ontologyEvaluate` /
//! `ontologySelect` pair. This test pins the next narrow boundary: R4 writes
//! a paired macro-native candidate only. It must not call old evaluate/select,
//! install ranking runtime, turn score/winner into authority, treat null as
//! success, or switch owners.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join(
    "../../fixtures/tesseract-macro-legacy-probe/macro_native_evaluate_select_rewrite_candidate_receipt.px",
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
fn evalselect_r4_marker_and_constitution_owner_are_pinned() {
  let run = eval_file(&fixture_path()).expect("eval/select R4 fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-r4-macro-native-evaluate-select-rewrite-candidate"
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
fn constitution_gate_keeps_evalselect_r4_candidate_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "r4-macro-native-evaluate-select-rewrite-candidate"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  assert_eq!(as_str(get(gate, "replacement-readiness")), "not-proven");

  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "call-old-builtins.ontologyEvaluate",
    "call-old-builtins.ontologySelect",
    "split-select-rewrite-from-evaluate",
    "emit-score-as-RigorFloor",
    "emit-winner-as-owner-switch",
    "treat-null-select-as-success",
    "compile-tie-break-order-as-route-cache",
    "install-ranking-runtime-from-r4",
    "claim-replacement-readiness-at-r4",
    "treat-llm-prose-as-selection-verdict",
  ] {
    assert!(blocks.contains(expected), "missing gate block `{expected}`");
  }
}

#[test]
fn surface_pair_and_r3_input_are_imported() {
  let run = eval_file(&fixture_path()).unwrap();
  let pair = get(&run, "surface-pair");
  assert_eq!(
    as_str(get(pair, "evaluate")),
    "stdlib/lib/ontology.px::builtins.ontologyEvaluate"
  );
  assert_eq!(
    as_str(get(pair, "select")),
    "stdlib/lib/ontology.px::builtins.ontologySelect"
  );
  assert!(as_bool(get(pair, "pair-required")));
  assert_eq!(
    as_str(get(pair, "scope")),
    "legacy-evaluate-select-pair-only"
  );

  let r3 = get(&run, "r3-input");
  assert_eq!(
    as_str(get(r3, "r3-verdict-ref")),
    "tesseract-macro-ontology-r3-evaluate-select-role-emission-verdict"
  );
  assert!(as_bool(get(r3, "r3-verdict-closed-for-this-surface-pair")));
  assert!(as_bool(get(
    r3,
    "r4-macro-native-rewrite-candidate-may-start"
  )));
  assert_eq!(as_list(get(r3, "emitted-roles")).len(), 9);
  assert_eq!(as_str(get(r3, "replacement-readiness")), "not-proven");
  assert!(!as_bool(get(r3, "owner-switch")));
  assert!(!as_bool(get(r3, "runtime-install")));
}

#[test]
fn rewrite_candidate_is_paired_macro_native_and_non_executable() {
  let run = eval_file(&fixture_path()).unwrap();
  let candidate = get(&run, "rewrite-candidate");
  assert_eq!(
    as_str(get(candidate, "id")),
    "r4.macro-native-evaluate-select.rewrite-candidate"
  );
  assert_eq!(as_str(get(candidate, "phase")), "R4");
  assert_eq!(
    as_str(get(candidate, "candidate-kind")),
    "macro-native-paired-rewrite-candidate"
  );
  assert_eq!(
    as_str(get(candidate, "scope")),
    "legacy-evaluate-select-pair-only"
  );
  assert!(as_bool(get(candidate, "pair-required")));
  assert_eq!(as_list(get(candidate, "surfaces")).len(), 2);
  assert_eq!(as_list(get(candidate, "uses-emitted-r3-roles")).len(), 9);
  assert!(!as_bool(get(candidate, "calls-legacy-ontologyEvaluate")));
  assert!(!as_bool(get(candidate, "calls-legacy-ontologySelect")));
  assert!(!as_bool(get(candidate, "emits-rigorfloor")));
  assert!(!as_bool(get(candidate, "emits-route-cache")));
  assert!(as_bool(get(candidate, "candidate-only")));
  assert!(!as_bool(get(candidate, "executable-now")));
  assert!(!as_bool(get(candidate, "installed")));
  assert_eq!(
    as_str(get(candidate, "output-status")),
    "ready-for-r5-reverse-replay"
  );
}

#[test]
fn rewrite_candidate_preserves_score_winner_null_and_runtime_boundaries() {
  let run = eval_file(&fixture_path()).unwrap();
  let candidate = get(&run, "rewrite-candidate");
  assert!(as_bool(get(candidate, "uses-axis-evidence")));
  assert!(as_bool(get(candidate, "uses-candidate-ranking")));
  assert!(as_bool(get(candidate, "uses-empty-selection-held")));
  assert!(as_bool(get(candidate, "uses-ranking-delta-receipt")));
  assert!(!as_bool(get(
    candidate,
    "emits-legacy-winner-as-current-proof"
  )));
  assert!(!as_bool(get(candidate, "null-select-success")));
  assert!(!as_bool(get(candidate, "owner-switch")));
  assert_eq!(
    as_str(get(candidate, "replacement-readiness")),
    "not-proven"
  );
  assert!(!as_bool(get(candidate, "ranking-runtime-install")));
}

#[test]
fn rewrite_steps_use_r3_roles_and_end_at_r5_need() {
  let run = eval_file(&fixture_path()).unwrap();
  let steps = attrs_by_id(get(&run, "rewrite-steps"));
  assert_eq!(steps.len(), 7);
  for (id, role) in [
    ("step.1.load-paired-surfaces", "EvaluationVectorSpecimen"),
    ("step.2.lower-score-to-axis-evidence", "AxisEvidence"),
    ("step.3.lower-selection-to-ranking", "CandidateRanking"),
    ("step.4.lower-null-to-held", "EmptySelectionHeld"),
    ("step.5.attach-tie-break-delta", "TieBreakEvidence"),
    ("step.6.attach-ranking-delta-receipt", "RankingDeltaReceipt"),
    ("step.7.emit-r5-replay-need", "ReverseReplayRequirement"),
  ] {
    let step = steps
      .get(id)
      .unwrap_or_else(|| panic!("missing step `{id}`"));
    assert_eq!(as_str(get(step, "role")), role);
    assert!(as_bool(get(step, "candidate-only")));
    assert!(!as_bool(get(step, "accepted")));
  }
  assert_eq!(
    as_str(get(
      steps.get("step.7.emit-r5-replay-need").unwrap(),
      "emits"
    )),
    "need.r5.evalselect-reverse-replay"
  );
}

#[test]
fn reference_deltas_are_explicit_before_r5() {
  let run = eval_file(&fixture_path()).unwrap();
  let deltas = attrs_by_id(get(&run, "reference-deltas"));
  assert_eq!(deltas.len(), 6);
  for expected in [
    "delta.score-authority",
    "delta.winner-authority",
    "delta.null-behavior",
    "delta.tie-break-order",
    "delta.runtime",
    "delta.proof",
  ] {
    let delta = deltas
      .get(expected)
      .unwrap_or_else(|| panic!("missing delta `{expected}`"));
    assert!(as_bool(get(delta, "allowed")));
  }
  assert_eq!(
    as_str(get(deltas.get("delta.score-authority").unwrap(), "macro")),
    "AxisEvidence-and-EvaluationVectorSpecimen"
  );
  assert_eq!(
    as_str(get(deltas.get("delta.null-behavior").unwrap(), "macro")),
    "EmptySelectionHeld"
  );
}

#[test]
fn held_rewrite_trials_cover_r3_pair_legacy_authority_null_and_runtime_failures() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "held-rewrite-trials"));
  assert_eq!(trials.len(), 8);

  for expected in [
    "trial.A.r3-verdict-missing",
    "trial.B.unpaired-select-rewrite",
    "trial.C.legacy-call-requested",
    "trial.D.score-authority-requested",
    "trial.E.winner-authority-requested",
    "trial.F.null-success-requested",
    "trial.G.runtime-install-requested",
  ] {
    let trial = trials
      .get(expected)
      .unwrap_or_else(|| panic!("missing trial `{expected}`"));
    assert_eq!(as_str(get(trial, "outcome")), "Held");
    assert!(!as_bool(get(trial, "accepted")));
    assert_ne!(as_str(get(trial, "reopen-path")), "not-needed");
  }

  let complete = trials.get("trial.H.complete-paired-candidate").unwrap();
  assert_eq!(
    as_str(get(complete, "outcome")),
    "ready-for-r5-reverse-replay"
  );
  assert!(!as_bool(get(complete, "accepted")));
}

#[test]
fn six_layer_rewrite_fold_blocks_authority_and_runtime_collapse() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-rewrite-fold");
  assert_eq!(
    as_str(get(fold, "mode")),
    "r4-macro-native-evaluate-select-rewrite-candidate"
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
    &["ontology", "rigorfloor-emitted"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["ontology", "route-cache-emitted"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["ontology", "owner-switch-emitted"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "score-demoted-to-axis-evidence"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "winner-demoted-to-candidate-ranking"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "null-select-demoted-to-held"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["runtime", "ranking-runtime-installed"]
  )));
  assert_eq!(
    as_str(get_path(fold, &["audit", "reverse-replay-status"])),
    "required-not-run"
  );
}

#[test]
fn r5_boundary_opens_replay_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let r5 = get(&run, "r5-boundary");
  assert!(as_bool(get(r5, "r4-candidate-written")));
  assert!(as_bool(get(r5, "r5-reverse-replay-may-start")));
  assert_eq!(
    as_str(get(r5, "r5-scope")),
    "replay-r4-evaluate-select-candidate-against-legacy-pair-specimens"
  );
  assert_eq!(
    as_str(get(r5, "replacement-readiness-after-r4")),
    "not-proven"
  );
  assert!(!as_bool(get(r5, "owner-switch-open")));
  assert!(!as_bool(get(r5, "runtime-install-open")));
  assert!(!as_bool(get(r5, "ranking-runtime-install-open")));

  let required = string_set(get(r5, "required-next"));
  for expected in [
    "replay-score-axis-evidence-against-evaluate-specimen",
    "replay-winner-ranking-against-select-specimen",
    "replay-empty-selection-as-held",
    "replay-tie-break-order-as-reference-delta",
    "preserve-negative-held-evidence",
    "emit-held-if-unexplained-ranking-mismatch",
  ] {
    assert!(
      required.contains(expected),
      "missing R5 requirement `{expected}`"
    );
  }
}

#[test]
fn runtime_observation_is_candidate_only_and_not_installed() {
  let run = eval_file(&fixture_path()).unwrap();
  let runtime = get(&run, "runtime-observation");
  assert_eq!(
    as_str(get(runtime, "observation-model")),
    "r4-macro-native-evaluate-select-runtime-candidates"
  );
  assert!(as_bool(get(runtime, "can-appear-at-runtime")));
  assert!(as_bool(get(runtime, "candidate-only")));
  assert!(!as_bool(get(runtime, "canonical-runtime-installed")));
  assert!(!as_bool(get(runtime, "executable-now")));
  assert!(!as_bool(get(runtime, "owner-switch")));
  assert!(!as_bool(get(runtime, "ranking-runtime-installed")));
  assert_eq!(as_list(get(runtime, "runtime-added-candidates")).len(), 4);
}

#[test]
fn discoveries_record_d143_through_d151() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 9);
  for expected in [
    "D143.evalselect-r4-rewrite-is-paired",
    "D144.evalselect-r4-is-macro-native-not-legacy-wrapper",
    "D145.score-and-axes-lower-to-evidence-not-rigorfloor",
    "D146.winner-lowers-to-ranking-observation-not-proof",
    "D147.null-select-lowers-to-held-pressure",
    "D148.tie-break-order-is-r4-reference-delta",
    "D149.evalselect-r4-opens-r5-only",
    "D150.evalselect-ranking-runtime-remains-non-executable",
    "D151.evalselect-selection-verdict-is-receipt-driven-not-prose",
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
    "held-score-as-evidence-not-floor"
  );
  assert_eq!(
    as_str(get_path(affected, &["routeCache", "pressure"])),
    "held-tie-break-as-delta-not-cache"
  );
  assert_eq!(
    as_str(get_path(affected, &["evaluateSelectRewrite", "pressure"])),
    "ready-for-r5-reverse-replay"
  );
  assert_eq!(
    as_str(get_path(affected, &["ownerSwitch", "pressure"])),
    "forbidden-at-r4"
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
fn negative_held_evidence_blocks_old_wrappers_authority_runtime_and_prose() {
  let run = eval_file(&fixture_path()).unwrap();
  let negative = get(&run, "negative-held-evidence");
  assert_eq!(as_str(get(negative, "status")), "present");
  let rejects = string_set(get(negative, "rejects"));
  for expected in [
    "r3-verdict-missing",
    "unpaired-select-rewrite",
    "legacy-evaluate-or-select-call",
    "score-as-rigorfloor-or-current-proof",
    "winner-as-owner-switch-or-current-proof",
    "null-select-as-success",
    "tie-break-as-route-cache",
    "ranking-runtime-install-at-r4",
    "llm-prose-selection-verdict",
  ] {
    assert!(rejects.contains(expected), "missing rejection `{expected}`");
  }
}

#[test]
fn blocked_shortcuts_cover_all_r4_evalselect_collapses() {
  let run = eval_file(&fixture_path()).unwrap();
  let blocks = string_set(get(&run, "blocked-shortcuts"));
  for expected in [
    "call-old-builtins.ontologyEvaluate",
    "call-old-builtins.ontologySelect",
    "split-select-rewrite-from-evaluate",
    "emit-score-as-RigorFloor",
    "emit-score-as-current-proof",
    "emit-winner-as-owner-switch",
    "emit-winner-as-current-proof",
    "treat-null-select-as-success",
    "compile-tie-break-order-as-route-cache",
    "install-ranking-runtime-from-r4",
    "claim-replacement-readiness-at-r4",
    "treat-llm-prose-as-selection-verdict",
  ] {
    assert!(
      blocks.contains(expected),
      "missing shortcut block `{expected}`"
    );
  }
}

#[test]
fn top_level_state_keeps_evalselect_unproven_without_runtime_or_owner_switch() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "reverse-replay-status")),
    "required-not-run"
  );
  assert_eq!(as_str(get(&run, "replacement-readiness")), "not-proven");
  assert!(!as_bool(get(&run, "owner-switch")));
  assert!(!as_bool(get(&run, "runtime-install")));
  assert!(!as_bool(get(&run, "global-ranking-runtime")));
  assert!(!as_bool(get(&run, "implementation-command")));
}
