//! R5 reverse replay for the macro-native promote candidate.
//!
//! R4 wrote a candidate for `builtins.ontologyPromote`. R5 replays that
//! candidate against the legacy promote specimen and checks every reference
//! delta. This is replay evidence only; it cannot claim replacement readiness,
//! runtime install, or owner switch.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base)
    .join("../../fixtures/tesseract-macro-legacy-probe/reverse_replay_promote_candidate_receipt.px")
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
  as_list(v).iter().map(|item| as_str(item)).collect()
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
fn r5_marker_and_constitution_owner_are_pinned() {
  let run = eval_file(&fixture_path()).expect("R5 reverse replay fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-r5-reverse-replay-promote-candidate"
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
fn constitution_gate_keeps_r5_candidate_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "r5-reverse-replay-promote-candidate"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  assert_eq!(as_str(get(gate, "replacement-readiness")), "not-proven");

  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "treat-r4-green-candidate-as-replay",
    "treat-reverse-turn-as-reverse-replay",
    "drop-reference-delta-check",
    "ignore-unexplained-mismatch",
    "drop-audit-ref",
    "emit-legacy-Accepted-as-current-proof",
    "install-runtime-route-from-r5",
    "treat-r5-replay-as-owner-switch",
    "claim-replacement-readiness-from-r5-alone",
    "treat-llm-prose-as-replay-result",
  ] {
    assert!(blocks.contains(expected), "missing gate block `{expected}`");
  }
}

#[test]
fn replay_inputs_link_r4_candidate_to_legacy_specimen() {
  let run = eval_file(&fixture_path()).unwrap();
  let legacy = get(&run, "legacy-replay-specimen");
  assert_eq!(
    as_str(get(legacy, "source-symbol")),
    "builtins.ontologyPromote"
  );
  assert_eq!(
    as_str(get_path(legacy, &["expected-output", "status"])),
    "Accepted"
  );
  assert!(!as_bool(get_path(
    legacy,
    &["expected-output", "current-authority"]
  )));

  let r4 = get(&run, "r4-replay-target");
  assert_eq!(
    as_str(get(r4, "id")),
    "r4.macro-native-promote.rewrite-candidate"
  );
  assert_eq!(
    as_str(get(r4, "output-status")),
    "ready-for-r5-reverse-replay"
  );
  assert!(!as_bool(get(r4, "calls-legacy-ontologyPromote")));
  assert!(!as_bool(get(r4, "old-accepted-output")));
}

#[test]
fn replay_steps_execute_compare_check_deltas_and_emit_r6_need() {
  let run = eval_file(&fixture_path()).unwrap();
  let steps = attrs_by_id(get(&run, "replay-steps"));
  assert_eq!(steps.len(), 6);
  for expected in [
    "step.1.load-r4-candidate",
    "step.2.load-legacy-specimen",
    "step.3.execute-replay-comparison",
    "step.4.check-reference-deltas",
    "step.5.record-negative-held-proof",
    "step.6.emit-r6-need",
  ] {
    let step = steps
      .get(expected)
      .unwrap_or_else(|| panic!("missing `{expected}`"));
    assert!(!as_bool(get(step, "held")));
  }
  assert_eq!(
    as_str(get(steps.get("step.6.emit-r6-need").unwrap(), "outcome")),
    "need.r6.owner-switch-receipt"
  );
}

#[test]
fn all_r4_reference_deltas_are_observed_and_covered() {
  let run = eval_file(&fixture_path()).unwrap();
  let deltas = attrs_by_id(get(&run, "delta-verdicts"));
  assert_eq!(deltas.len(), 5);
  for expected in [
    "delta.authority",
    "delta.output-status",
    "delta.runtime",
    "delta.proof",
    "delta.source-provenance",
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
fn replay_comparisons_turn_differences_into_covered_deltas() {
  let run = eval_file(&fixture_path()).unwrap();
  let comparisons = attrs_by_id(get(&run, "replay-comparisons"));
  assert_eq!(comparisons.len(), 4);
  for expected in [
    "compare.legacy-status",
    "compare.authority",
    "compare.proof",
    "compare.source-provenance",
  ] {
    let cmp = comparisons
      .get(expected)
      .unwrap_or_else(|| panic!("missing comparison `{expected}`"));
    assert_eq!(as_str(get(cmp, "verdict")), "covered-delta");
    assert!(!as_bool(get(cmp, "held")));
  }
}

#[test]
fn replay_trials_hold_missing_inputs_mismatch_audit_loss_and_reverse_turn_confusion() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "replay-trials"));
  assert_eq!(trials.len(), 7);
  for expected in [
    "trial.A.r4-candidate-missing",
    "trial.B.legacy-specimen-missing",
    "trial.C.delta-set-missing",
    "trial.D.unexplained-mismatch",
    "trial.E.audit-ref-lost",
    "trial.F.reverse-turn-confused",
  ] {
    let trial = trials
      .get(expected)
      .unwrap_or_else(|| panic!("missing trial `{expected}`"));
    assert_eq!(as_str(get(trial, "outcome")), "Held");
    assert!(!as_bool(get(trial, "replay-verified")));
    assert_ne!(as_str(get(trial, "rewrite-debt")), "none");
  }
  let complete = trials.get("trial.G.complete-replay").unwrap();
  assert_eq!(as_str(get(complete, "outcome")), "reverse-replay-verified");
  assert!(as_bool(get(complete, "replay-verified")));
}

#[test]
fn audit_trace_preserves_legacy_r4_and_r5_refs() {
  let run = eval_file(&fixture_path()).unwrap();
  let audit = get(&run, "audit-trace");
  assert_eq!(
    as_str(get(audit, "legacy-ref")),
    "audit.r5.legacy-specimen.legacy-ontology.promote.accepted"
  );
  assert_eq!(
    as_str(get(audit, "r4-ref")),
    "audit.r4.macro-native-promote.rewrite-candidate"
  );
  assert_eq!(
    as_str(get(audit, "r5-ref")),
    "audit.r5.reverse-replay.promote-candidate"
  );
  assert!(as_bool(get(audit, "refs-preserved")));
  assert_eq!(as_number(get(audit, "delta-verdict-count")), 5.0);
  assert!(as_bool(get(audit, "negative-held-present")));
}

#[test]
fn reverse_replay_verdict_opens_readiness_work_not_readiness_or_owner_switch() {
  let run = eval_file(&fixture_path()).unwrap();
  let verdict = get(&run, "reverse-replay-verdict");
  assert_eq!(as_str(get(verdict, "replay-kind")), "reverse-replay");
  assert!(!as_bool(get(verdict, "reverse-turn-instance")));
  assert!(as_bool(get(verdict, "all-deltas-covered")));
  assert!(!as_bool(get(verdict, "unexplained-mismatch")));
  assert!(as_bool(get(verdict, "audit-refs-preserved")));
  assert_eq!(as_str(get(verdict, "verdict")), "reverse-replay-verified");
  assert_eq!(
    as_str(get(verdict, "replacement-readiness-after-r5")),
    "not-proven"
  );
  assert!(!as_bool(get(verdict, "owner-switch-open")));
  assert!(!as_bool(get(verdict, "runtime-install-open")));

  let required = string_set(get(verdict, "next-required"));
  for expected in [
    "owner-law-gate",
    "runtime-route-proof",
    "replacement-readiness-receipt",
    "r6-owner-switch-receipt",
  ] {
    assert!(
      required.contains(expected),
      "missing next requirement `{expected}`"
    );
  }
}

#[test]
fn six_layer_replay_fold_preserves_replay_boundaries() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-replay-fold");
  assert_eq!(
    as_str(get(fold, "mode")),
    "r5-reverse-replay-promote-candidate"
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
    as_str(get_path(fold, &["ontology", "replay-kind"])),
    "reverse-replay"
  );
  assert!(!as_bool(get_path(
    fold,
    &["ontology", "reverse-turn-instance"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "legacy-accepted-is-specimen"]
  )));
  assert_eq!(
    as_number(get_path(fold, &["semantic", "covered-delta-count"])),
    5.0
  );
  assert!(!as_bool(get_path(fold, &["runtime", "executable-now"])));
  assert!(!as_bool(get_path(
    fold,
    &["runtime", "runtime-route-installed"]
  )));
  assert!(!as_bool(get_path(fold, &["runtime", "owner-switch"])));
  assert!(as_bool(get_path(fold, &["audit", "refs-preserved"])));
}

#[test]
fn runtime_observation_is_candidate_only_and_not_installed() {
  let run = eval_file(&fixture_path()).unwrap();
  let runtime = get(&run, "runtime-observation");
  assert_eq!(
    as_str(get(runtime, "observation-model")),
    "r5-reverse-replay-runtime-candidates"
  );
  assert!(as_bool(get(runtime, "can-appear-at-runtime")));
  assert!(as_bool(get(runtime, "candidate-only")));
  assert!(!as_bool(get(runtime, "canonical-runtime-installed")));
  assert!(!as_bool(get(runtime, "executable-now")));
  assert!(!as_bool(get(runtime, "owner-switch")));
  assert_eq!(as_list(get(runtime, "runtime-added-candidates")).len(), 3);
}

#[test]
fn discoveries_record_d99_through_d107() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 9);
  for expected in [
    "D99.r5-reverse-replay-is-distinct-from-forward-fold-and-reverse-turn",
    "D100.r5-covered-deltas-are-checked-not-assumed",
    "D101.unexplained-replay-mismatch-emits-held-and-rewrite-debt",
    "D102.audit-refs-are-preserved-through-r4-to-r5",
    "D103.legacy-accepted-remains-specimen-during-replay",
    "D104.r5-verifies-replay-evidence-not-replacement-readiness",
    "D105.r5-runtime-route-proof-remains-candidate",
    "D106.r5-opens-r6-need-without-owner-switch",
    "D107.r5-preserves-pnix-independence-from-llm-replay-claims",
  ] {
    let d = discoveries
      .get(expected)
      .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
    assert!(as_bool(get(d, "scenario-only")));
  }
}

#[test]
fn affected_plans_do_not_become_implementation_targets() {
  let run = eval_file(&fixture_path()).unwrap();
  let affected = get(&run, "affected-plans");
  assert_eq!(
    as_str(get_path(affected, &["legacyPromote", "pressure"])),
    "r5-replay-verified"
  );
  assert_eq!(
    as_str(get_path(affected, &["ownerSwitch", "pressure"])),
    "need-emitted-but-forbidden-at-r5"
  );
  for key in [
    "legacyPromote",
    "r4Candidate",
    "runtimeRoute",
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
    "reverse-turn-as-reverse-replay",
    "uncovered-reference-delta",
    "unexplained-mismatch",
    "audit-ref-loss",
    "legacy-Accepted-as-current-proof",
    "runtime-install-at-r5",
    "owner-switch-at-r5",
    "replacement-readiness-from-r5-alone",
    "llm-prose-as-replay-result",
  ] {
    assert!(rejects.contains(expected), "missing rejection `{expected}`");
  }
}

#[test]
fn r5_receipt_keeps_replacement_not_ready() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(as_str(get(&run, "reverse-replay-status")), "verified");
  assert_eq!(as_str(get(&run, "replacement-readiness")), "not-proven");
  assert!(!as_bool(get(&run, "owner-switch")));
  assert!(!as_bool(get(&run, "implementation-command")));
}
