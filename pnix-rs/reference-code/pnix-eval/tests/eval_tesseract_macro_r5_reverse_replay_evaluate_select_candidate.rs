//! R5 reverse replay for the macro-native evaluate/select candidate.
//!
//! R4 wrote a paired candidate for `ontologyEvaluate` / `ontologySelect`.
//! R5 replays that candidate against the legacy pair specimens and checks each
//! reference delta. Replay success is evidence for a future readiness receipt;
//! it is not readiness, owner switch, or ranking runtime install.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join(
    "../../fixtures/tesseract-macro-legacy-probe/reverse_replay_evaluate_select_candidate_receipt.px",
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

fn as_number(v: &Value) -> f64 {
  match v {
    Value::Int(n) => *n as f64,
    Value::Float(n) => *n,
    other => panic!("expected number, got {:?}", other),
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
fn evalselect_r5_marker_and_constitution_owner_are_pinned() {
  let run = eval_file(&fixture_path()).expect("eval/select R5 fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-r5-reverse-replay-evaluate-select-candidate"
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
fn constitution_gate_keeps_evalselect_r5_candidate_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "r5-reverse-replay-evaluate-select-candidate"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  assert_eq!(as_str(get(gate, "replacement-readiness")), "not-proven");

  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "treat-r4-green-candidate-as-replay",
    "replay-select-without-evaluate",
    "drop-score-axis-replay",
    "drop-winner-ranking-replay",
    "treat-null-select-as-success",
    "compile-tie-break-order-as-route-cache",
    "drop-reference-delta-check",
    "ignore-unexplained-ranking-mismatch",
    "install-ranking-runtime-from-r5",
    "claim-replacement-readiness-from-r5-alone",
    "treat-llm-prose-as-replay-result",
  ] {
    assert!(blocks.contains(expected), "missing gate block `{expected}`");
  }
}

#[test]
fn replay_inputs_link_r4_candidate_to_legacy_pair_specimens() {
  let run = eval_file(&fixture_path()).unwrap();
  let legacy = get(&run, "legacy-replay-specimens");
  let evaluate = get(legacy, "evaluate");
  let select = get(legacy, "select");
  assert_eq!(
    as_str(get(evaluate, "source-symbol")),
    "builtins.ontologyEvaluate"
  );
  assert_eq!(
    as_str(get(evaluate, "expected-output-shape")),
    "interpretation-plus-six-axis-fields"
  );
  assert_eq!(as_list(get(evaluate, "expected-axes")).len(), 7);
  assert!(!as_bool(get(evaluate, "current-authority")));
  assert_eq!(
    as_str(get(select, "source-symbol")),
    "builtins.ontologySelect"
  );
  assert_eq!(as_str(get(select, "empty-list-result")), "null");
  assert_eq!(as_list(get(select, "expected-tie-break-order")).len(), 6);
  assert!(!as_bool(get(select, "current-authority")));

  let r4 = get(&run, "r4-replay-target");
  assert_eq!(
    as_str(get(r4, "id")),
    "r4.macro-native-evaluate-select.rewrite-candidate"
  );
  assert!(as_bool(get(r4, "pair-required")));
  assert!(!as_bool(get(r4, "calls-legacy-ontologyEvaluate")));
  assert!(!as_bool(get(r4, "calls-legacy-ontologySelect")));
  assert!(!as_bool(get(r4, "emits-rigorfloor")));
  assert!(!as_bool(get(r4, "emits-route-cache")));
  assert!(!as_bool(get(r4, "ranking-runtime-install")));
}

#[test]
fn replay_steps_cover_pair_score_winner_null_and_readiness_need() {
  let run = eval_file(&fixture_path()).unwrap();
  let steps = attrs_by_id(get(&run, "replay-steps"));
  assert_eq!(steps.len(), 7);
  for expected in [
    "step.1.load-r4-paired-candidate",
    "step.2.load-legacy-evaluate-specimen",
    "step.3.load-legacy-select-specimen",
    "step.4.replay-score-axis-evidence",
    "step.5.replay-winner-and-tie-break",
    "step.6.replay-empty-selection-held",
    "step.7.emit-readiness-need",
  ] {
    let step = steps
      .get(expected)
      .unwrap_or_else(|| panic!("missing `{expected}`"));
    assert!(!as_bool(get(step, "held")));
  }
  assert_eq!(
    as_str(get(
      steps.get("step.7.emit-readiness-need").unwrap(),
      "outcome"
    )),
    "need.evalselect.replacement-readiness-receipt"
  );
}

#[test]
fn all_r4_reference_deltas_are_observed_and_covered() {
  let run = eval_file(&fixture_path()).unwrap();
  let deltas = attrs_by_id(get(&run, "delta-verdicts"));
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
    assert!(as_bool(get(delta, "allowed-by-r4")));
    assert!(as_bool(get(delta, "replay-observed")));
    assert_eq!(as_str(get(delta, "verdict")), "covered");
  }
}

#[test]
fn replay_comparisons_cover_score_axes_winner_null_tie_break_and_runtime() {
  let run = eval_file(&fixture_path()).unwrap();
  let comparisons = attrs_by_id(get(&run, "replay-comparisons"));
  assert_eq!(comparisons.len(), 6);
  for expected in [
    "compare.evaluate-output-shape",
    "compare.evaluate-axes",
    "compare.select-output-shape",
    "compare.null-select",
    "compare.tie-break-order",
    "compare.runtime-proof",
  ] {
    let cmp = comparisons
      .get(expected)
      .unwrap_or_else(|| panic!("missing comparison `{expected}`"));
    assert_eq!(as_str(get(cmp, "verdict")), "covered-delta");
    assert!(!as_bool(get(cmp, "held")));
  }
  assert_eq!(
    as_str(get(
      comparisons.get("compare.null-select").unwrap(),
      "macro-value"
    )),
    "EmptySelectionHeld"
  );
}

#[test]
fn replay_trials_hold_missing_inputs_mismatches_audit_loss_and_runtime_install() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "replay-trials"));
  assert_eq!(trials.len(), 10);
  for expected in [
    "trial.A.r4-candidate-missing",
    "trial.B.legacy-evaluate-specimen-missing",
    "trial.C.legacy-select-specimen-missing",
    "trial.D.delta-set-missing",
    "trial.E.score-axis-mismatch",
    "trial.F.winner-ranking-mismatch",
    "trial.G.null-held-missing",
    "trial.H.audit-ref-lost",
    "trial.I.runtime-install-requested",
  ] {
    let trial = trials
      .get(expected)
      .unwrap_or_else(|| panic!("missing trial `{expected}`"));
    assert_eq!(as_str(get(trial, "outcome")), "Held");
    assert!(!as_bool(get(trial, "replay-verified")));
    assert_ne!(as_str(get(trial, "rewrite-debt")), "none");
  }
  let complete = trials.get("trial.J.complete-replay").unwrap();
  assert_eq!(as_str(get(complete, "outcome")), "reverse-replay-verified");
  assert!(as_bool(get(complete, "replay-verified")));
}

#[test]
fn audit_trace_preserves_legacy_r3_r4_and_r5_refs() {
  let run = eval_file(&fixture_path()).unwrap();
  let audit = get(&run, "audit-trace");
  assert_eq!(
    as_str(get(audit, "evaluate-ref")),
    "audit.r5.legacy-specimen.ontologyEvaluate.six-axis"
  );
  assert_eq!(
    as_str(get(audit, "select-ref")),
    "audit.r5.legacy-specimen.ontologySelect.tie-break"
  );
  assert_eq!(
    as_str(get(audit, "r3-ref")),
    "tesseract-macro-ontology-r3-evaluate-select-role-emission-verdict"
  );
  assert_eq!(
    as_str(get(audit, "r4-ref")),
    "audit.r4.macro-native-evaluate-select.rewrite-candidate"
  );
  assert_eq!(
    as_str(get(audit, "r5-ref")),
    "audit.r5.reverse-replay.evaluate-select-candidate"
  );
  assert!(as_bool(get(audit, "refs-preserved")));
  assert_eq!(as_number(get(audit, "replay-step-count")), 7.0);
  assert_eq!(as_number(get(audit, "delta-verdict-count")), 6.0);
  assert_eq!(as_number(get(audit, "comparison-count")), 6.0);
}

#[test]
fn reverse_replay_verdict_opens_readiness_work_not_readiness_owner_or_runtime() {
  let run = eval_file(&fixture_path()).unwrap();
  let verdict = get(&run, "reverse-replay-verdict");
  assert_eq!(
    as_str(get(verdict, "surface-pair")),
    "surface-pair.legacy-ontology.evaluate-select"
  );
  assert_eq!(as_str(get(verdict, "replay-kind")), "reverse-replay");
  assert!(!as_bool(get(verdict, "reverse-turn-instance")));
  assert!(as_bool(get(verdict, "paired-replay")));
  assert!(as_bool(get(verdict, "all-deltas-covered")));
  assert!(as_bool(get(verdict, "score-axis-covered")));
  assert!(as_bool(get(verdict, "winner-ranking-covered")));
  assert!(as_bool(get(verdict, "empty-selection-held-covered")));
  assert!(as_bool(get(verdict, "tie-break-delta-covered")));
  assert!(!as_bool(get(verdict, "unexplained-mismatch")));
  assert_eq!(as_str(get(verdict, "verdict")), "reverse-replay-verified");
  assert_eq!(
    as_str(get(verdict, "replacement-readiness-after-r5")),
    "not-proven"
  );
  assert!(!as_bool(get(verdict, "owner-switch-open")));
  assert!(!as_bool(get(verdict, "runtime-install-open")));
  assert!(!as_bool(get(verdict, "ranking-runtime-install-open")));

  let required = string_set(get(verdict, "next-required"));
  for expected in [
    "surface-pair-replacement-readiness-receipt",
    "owner-law-gate",
    "runtime-route-proof",
    "negative-held-retention",
    "ranking-regression-corpus-binding",
  ] {
    assert!(
      required.contains(expected),
      "missing next requirement `{expected}`"
    );
  }
}

#[test]
fn six_layer_replay_fold_preserves_pair_and_runtime_boundaries() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-replay-fold");
  assert_eq!(
    as_str(get(fold, "mode")),
    "r5-reverse-replay-evaluate-select-candidate"
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
    as_str(get_path(fold, &["ontology", "replay-kind"])),
    "reverse-replay"
  );
  assert!(!as_bool(get_path(
    fold,
    &["ontology", "reverse-turn-instance"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["ontology", "rigorfloor-emitted"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["ontology", "route-cache-emitted"]
  )));
  assert!(as_bool(get_path(fold, &["semantic", "score-axis-covered"])));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "winner-ranking-covered"]
  )));
  assert!(as_bool(get_path(fold, &["semantic", "null-held-covered"])));
  assert!(!as_bool(get_path(
    fold,
    &["runtime", "ranking-runtime-installed"]
  )));
  assert!(as_bool(get_path(fold, &["audit", "refs-preserved"])));
}

#[test]
fn runtime_observation_is_candidate_only_and_not_installed() {
  let run = eval_file(&fixture_path()).unwrap();
  let runtime = get(&run, "runtime-observation");
  assert_eq!(
    as_str(get(runtime, "observation-model")),
    "r5-reverse-replay-evaluate-select-runtime-candidates"
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
fn discoveries_record_d152_through_d160() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 9);
  for expected in [
    "D152.evalselect-r5-replay-is-paired",
    "D153.score-axis-deltas-are-replayed-against-evaluate-specimen",
    "D154.winner-ranking-deltas-are-replayed-against-select-specimen",
    "D155.null-select-replay-stays-held-not-success",
    "D156.tie-break-delta-is-checked-not-compiled",
    "D157.unexplained-ranking-mismatch-emits-held-and-rewrite-debt",
    "D158.audit-refs-preserve-r3-r4-r5-lineage",
    "D159.evalselect-r5-verifies-replay-not-readiness",
    "D160.evalselect-replay-success-is-receipt-driven-not-prose",
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
    "held-score-replay-is-evidence-not-floor"
  );
  assert_eq!(
    as_str(get_path(affected, &["routeCache", "pressure"])),
    "held-tie-break-replay-is-delta-not-cache"
  );
  assert_eq!(
    as_str(get_path(affected, &["evaluateSelectRewrite", "pressure"])),
    "advance-to-surface-pair-readiness-receipt"
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
fn negative_held_evidence_blocks_replay_shortcuts() {
  let run = eval_file(&fixture_path()).unwrap();
  let negative = get(&run, "negative-held-evidence");
  assert_eq!(as_str(get(negative, "status")), "present");
  let rejects = string_set(get(negative, "rejects"));
  for expected in [
    "r4-green-candidate-as-replay",
    "select-only-replay",
    "score-axis-replay-drop",
    "winner-ranking-replay-drop",
    "null-select-as-success",
    "tie-break-as-route-cache",
    "uncovered-reference-delta",
    "unexplained-ranking-mismatch",
    "audit-ref-loss",
    "ranking-runtime-install-at-r5",
    "owner-switch-at-r5",
    "replacement-readiness-from-r5-alone",
    "llm-prose-as-replay-result",
  ] {
    assert!(rejects.contains(expected), "missing rejection `{expected}`");
  }
}

#[test]
fn blocked_shortcuts_cover_r5_evalselect_collapse_modes() {
  let run = eval_file(&fixture_path()).unwrap();
  let blocks = string_set(get(&run, "blocked-shortcuts"));
  for expected in [
    "treat-r4-green-candidate-as-replay",
    "replay-select-without-evaluate",
    "drop-score-axis-replay",
    "drop-winner-ranking-replay",
    "treat-null-select-as-success",
    "compile-tie-break-order-as-route-cache",
    "drop-reference-delta-check",
    "ignore-unexplained-ranking-mismatch",
    "drop-audit-ref",
    "install-ranking-runtime-from-r5",
    "treat-r5-replay-as-owner-switch",
    "claim-replacement-readiness-from-r5-alone",
    "treat-llm-prose-as-replay-result",
  ] {
    assert!(
      blocks.contains(expected),
      "missing shortcut block `{expected}`"
    );
  }
}

#[test]
fn top_level_state_keeps_evalselect_not_ready_after_replay() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(as_str(get(&run, "reverse-replay-status")), "verified");
  assert_eq!(as_str(get(&run, "replacement-readiness")), "not-proven");
  assert!(!as_bool(get(&run, "owner-switch")));
  assert!(!as_bool(get(&run, "runtime-install")));
  assert!(!as_bool(get(&run, "global-ranking-runtime")));
  assert!(!as_bool(get(&run, "implementation-command")));
}
