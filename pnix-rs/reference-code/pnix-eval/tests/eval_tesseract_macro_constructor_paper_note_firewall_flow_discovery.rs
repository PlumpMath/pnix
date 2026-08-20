//! Constructor paper-note firewall flow discovery.
//!
//! D64 named the firewall: Claude/Codex generated code is a paper-note
//! candidate, not PNIX self. This test pins the firewall enforcement flow:
//! a constructor patch walks an ordered five-gate sequence (intent-receipt,
//! macro-fold, axis-separation, regression-proof, owner-law). The first
//! failing gate emits Held and stops enforcement, but does not stop diagnostic
//! cognition. Passing all five gates produces a candidate-cleared paper-note
//! entering a PNIX promotion queue, not brain mutation.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join(
    "../../fixtures/tesseract-macro-legacy-probe/constructor_paper_note_firewall_flow_discovery_receipt.px",
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
fn firewall_flow_marker_and_constitution_owner_are_pinned() {
  let run = eval_file(&fixture_path()).expect("firewall flow fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-constructor-paper-note-firewall-flow"
  );
  assert_eq!(
    as_str(get(&run, "truth-owner")),
    "project-wiki/maps/tesseract-macro-ontology-discovery-ledger.md"
  );
  assert_eq!(
    as_str(get(&run, "constitution-owner")),
    "stdlib/lib/gate/tesseract-constitution.px"
  );
}

#[test]
fn constitution_gate_keeps_firewall_flow_candidate_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "constructor-paper-note-firewall-flow"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  assert_eq!(as_str(get(gate, "replacement-readiness")), "not-proven");

  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "merge-constructor-code-without-intent-receipt",
    "merge-constructor-code-on-fold-mismatch",
    "merge-constructor-code-with-axis-collapse",
    "merge-constructor-code-with-regression",
    "merge-constructor-code-without-owner-law-gate",
    "promote-constructor-self-report-as-authority",
    "treat-green-tests-as-owner-law-substitute",
    "skip-firewall-gates-due-to-fluency",
    "discard-held-constructor-substance",
    "treat-first-fail-as-end-of-cognition",
    "treat-human-authorization-as-global-brake",
  ] {
    assert!(blocks.contains(expected), "missing gate block `{expected}`");
  }
}

#[test]
fn five_gates_appear_in_strict_order() {
  let run = eval_file(&fixture_path()).unwrap();
  let gates = as_list(get(&run, "firewall-gates"));
  assert_eq!(gates.len(), 5);

  let expected_order = [
    "gate.1.intent-receipt",
    "gate.2.macro-fold",
    "gate.3.axis-separation",
    "gate.4.regression-proof",
    "gate.5.owner-law",
  ];
  for (idx, gate) in gates.iter().enumerate() {
    assert_eq!(as_str(get(gate, "id")), expected_order[idx]);
    assert_eq!(as_number(get(gate, "order")), (idx + 1) as f64);
  }
}

#[test]
fn each_gate_blocks_specific_bypass_attempts() {
  let run = eval_file(&fixture_path()).unwrap();
  let gates = attrs_by_id(get(&run, "firewall-gates"));

  let intent = gates.get("gate.1.intent-receipt").unwrap();
  let intent_blocks = string_set(get(intent, "bypass-attempts-blocked"));
  assert!(intent_blocks.contains("constructor-self-report-as-intent"));
  assert!(intent_blocks.contains("green-tests-as-intent-substitute"));

  let axis = gates.get("gate.3.axis-separation").unwrap();
  let axis_blocks = string_set(get(axis, "bypass-attempts-blocked"));
  assert!(axis_blocks.contains("rename-llm-output-as-final-word-without-fold-proof"));
  assert!(axis_blocks.contains("downgrade-human-to-approval-button"));

  let owner_law = gates.get("gate.5.owner-law").unwrap();
  let owner_blocks = string_set(get(owner_law, "bypass-attempts-blocked"));
  assert!(owner_blocks.contains("set-replacement-readiness-true-without-r6-receipt"));
  assert!(owner_blocks.contains("set-owner-switch-true-on-fixture-success"));
  assert!(owner_blocks.contains("promote-candidate-to-accepted-without-replay"));
}

#[test]
fn six_trials_cover_one_per_gate_failure_plus_all_pass() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = as_list(get(&run, "firewall-trials"));
  assert_eq!(trials.len(), 6);

  let by_id = attrs_by_id(get(&run, "firewall-trials"));
  for expected in [
    "trial.A.no-intent-receipt",
    "trial.B.fold-inconsistent",
    "trial.C.axis-collapse",
    "trial.D.regression",
    "trial.E.owner-law-violation",
    "trial.F.all-gates-pass",
  ] {
    assert!(by_id.contains_key(expected), "missing trial `{expected}`");
  }
}

#[test]
fn first_failing_gate_emits_held_and_blocks_brain_mutation() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "firewall-trials"));

  let cases = [
    ("trial.A.no-intent-receipt", "gate.1.intent-receipt", 0),
    ("trial.B.fold-inconsistent", "gate.2.macro-fold", 1),
    ("trial.C.axis-collapse", "gate.3.axis-separation", 2),
    ("trial.D.regression", "gate.4.regression-proof", 3),
    ("trial.E.owner-law-violation", "gate.5.owner-law", 4),
  ];

  for (trial_id, expected_failing_gate, expected_passed_count) in cases {
    let trial = trials.get(trial_id).unwrap();
    assert_eq!(
      as_str(get(trial, "first-failing-gate")),
      expected_failing_gate,
      "{trial_id} must fail at {expected_failing_gate}"
    );
    assert_eq!(as_str(get(trial, "verdict")), "Held");
    assert!(!as_bool(get(trial, "brain-mutation-applied")));
    assert!(!as_bool(get(trial, "candidate-cleared")));
    assert!(as_bool(get(trial, "retained-substance")));
    assert!(as_bool(get(trial, "enforcement-stops-at-first-fail")));
    assert!(as_bool(get(trial, "diagnostic-shadow-scan-allowed")));
    assert!(
      as_str(get(trial, "held-substance-id")).starts_with("paper-note-substance."),
      "{trial_id} must retain paper-note substance"
    );
    assert_eq!(
      as_list(get(trial, "passed-gates")).len(),
      expected_passed_count
    );
  }
}

#[test]
fn all_pass_trial_is_candidate_cleared_not_brain_mutation() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "firewall-trials"));
  let f = trials.get("trial.F.all-gates-pass").unwrap();

  assert_eq!(as_str(get(f, "first-failing-gate")), "none");
  assert_eq!(as_str(get(f, "failed-gate")), "none");
  assert_eq!(as_str(get(f, "verdict")), "candidate-cleared");
  assert!(as_bool(get(f, "candidate-cleared")));
  assert!(!as_bool(get(f, "brain-mutation-applied")));
  assert!(as_bool(get(f, "retained-substance")));
  assert!(as_bool(get(f, "enters-promotion-queue")));
  assert_eq!(as_str(get(f, "promotion-queue-status")), "candidate");
  assert_eq!(
    as_str(get(f, "non-consequence-internal-promotion-owner")),
    "pnix-owner-law"
  );
  assert!(as_bool(get(
    f,
    "consequence-bearing-promotion-requires-human"
  )));
  assert_eq!(as_list(get(f, "passed-gates")).len(), 5);

  // The pass result must explicitly say candidate-cleared enters PNIX's
  // promotion queue and only consequence-bearing promotion needs the human
  // risk-budget owner. It must not be a dead end.
  let reason = as_str(get(f, "reason"));
  assert!(
    reason.contains("promotion queue"),
    "trial.F reason must reference the promotion queue"
  );
  assert!(
    reason.contains("human risk-budget"),
    "trial.F reason must reference human risk-budget authorization"
  );
}

#[test]
fn flow_invariants_pin_load_bearing_constraints() {
  let run = eval_file(&fixture_path()).unwrap();
  let inv = get(&run, "flow-invariants");

  for key in [
    "gate-order-is-fixed",
    "first-fail-emits-held",
    "first-fail-stops-enforcement-not-cognition",
    "held-skips-remaining-enforcement-gates",
    "held-preserves-constructor-substance",
    "diagnostic-shadow-scan-is-candidate-only",
    "audit-ref-preserved-through-all-gates",
    "constructor-self-report-cannot-bypass-any-gate",
    "green-tests-cannot-substitute-for-owner-law-gate",
    "all-pass-produces-candidate-cleared-not-brain-accepted",
    "candidate-cleared-enters-promotion-queue",
    "consequence-bearing-promotion-requires-human-risk-budget",
    "human-authorization-is-not-global-cognition-brake",
    "firewall-is-receipt-shape-not-rust-runtime",
    "firewall-applies-to-all-constructor-output",
  ] {
    assert!(
      as_bool(get(inv, key)),
      "flow invariant `{key}` must be true"
    );
  }
}

#[test]
fn six_layer_fold_preserves_gate_order_and_audit_trace() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-firewall-fold");
  assert_eq!(
    as_str(get(fold, "mode")),
    "constructor-paper-note-firewall-flow"
  );
  for layer in [
    "surface", "ontology", "semantic", "gate", "runtime", "audit",
  ] {
    assert!(
      as_bool(get_path(fold, &[layer, "visible"])),
      "layer `{layer}` must be visible"
    );
  }
  assert_eq!(as_number(get_path(fold, &["surface", "trial-count"])), 6.0);
  assert_eq!(as_number(get_path(fold, &["surface", "gate-count"])), 5.0);
  assert!(as_bool(get_path(fold, &["semantic", "gate-order-visible"])));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "first-fail-discrimination-visible"]
  )));
  assert!(as_bool(get_path(fold, &["audit", "audit-ref-preserved"])));
  assert!(as_bool(get_path(
    fold,
    &["audit", "first-fail-trace-required"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["audit", "diagnostic-shadow-trace-allowed"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["audit", "all-gate-trace-required"]
  )));
}

#[test]
fn diagnostic_shadow_scan_keeps_learning_alive_after_first_fail() {
  let run = eval_file(&fixture_path()).unwrap();
  let scan = get(&run, "diagnostic-shadow-scan");
  assert_eq!(as_str(get(scan, "status")), "candidate-only");
  assert!(as_bool(get(scan, "runs-after-first-fail")));
  assert!(as_bool(get(scan, "can-discover-additional-held")));
  assert!(as_bool(get(scan, "can-emit-repair-candidates")));
  assert!(as_bool(get(scan, "cannot-clear-patch")));
  assert!(as_bool(get(scan, "cannot-bypass-first-fail")));
  assert!(as_bool(get(scan, "preserves-learning-substance")));

  let scan_targets = string_set(get(scan, "scan-targets"));
  for expected in [
    "gate.1.intent-receipt",
    "gate.2.macro-fold",
    "gate.3.axis-separation",
    "gate.4.regression-proof",
    "gate.5.owner-law",
  ] {
    assert!(
      scan_targets.contains(expected),
      "missing scan target `{expected}`"
    );
  }
}

#[test]
fn promotion_queue_is_not_dead_end_or_immediate_brain_mutation() {
  let run = eval_file(&fixture_path()).unwrap();
  let queue = get(&run, "promotion-queue");
  assert_eq!(
    as_str(get(queue, "id")),
    "promotion.queue.constructor-paper-note"
  );
  assert_eq!(as_str(get(queue, "receives")), "trial.F.all-gates-pass");
  assert_eq!(as_str(get(queue, "status")), "candidate");
  assert!(!as_bool(get(queue, "brain-mutation-applied")));
  assert!(!as_bool(get(queue, "dead-end")));
  assert_eq!(
    as_str(get(queue, "non-consequence-internal-promotion-owner")),
    "pnix-owner-law"
  );
  assert!(as_bool(get(
    queue,
    "consequence-bearing-promotion-requires-human"
  )));

  let can_drive = string_set(get(queue, "can-drive"));
  for expected in [
    "repair-plan",
    "self-knowledge-candidate",
    "future-proof-work",
    "pnix-owner-law-promotion-check",
  ] {
    assert!(
      can_drive.contains(expected),
      "missing promotion driver `{expected}`"
    );
  }
}

#[test]
fn self_knowledge_candidates_record_open_firewall_frontiers() {
  let run = eval_file(&fixture_path()).unwrap();
  let candidates = as_list(get(&run, "self-knowledge-candidates"));
  assert_eq!(candidates.len(), 7);
  let ids: BTreeSet<&str> = candidates
    .iter()
    .map(|candidate| as_str(get(candidate, "id")))
    .collect();
  for expected in [
    "knowledge.firewall.gate-order-is-load-bearing",
    "need.firewall.intent-receipt-source",
    "need.firewall.regression-corpus-binding",
    "held.firewall.candidate-cleared-is-not-promotion",
    "held.firewall.no-runtime-installer-yet",
    "knowledge.firewall.held-substance-is-preserved",
    "knowledge.firewall.diagnostic-shadow-scan",
  ] {
    assert!(ids.contains(expected), "missing candidate `{expected}`");
  }
  for candidate in candidates {
    assert!(!as_bool(get(candidate, "accepted")));
  }
}

#[test]
fn runtime_observation_is_candidate_only_no_nix_gate() {
  let run = eval_file(&fixture_path()).unwrap();
  let runtime = get(&run, "runtime-observation");
  assert_eq!(
    as_str(get(runtime, "observation-model")),
    "constructor-paper-note-firewall-flow-runtime-candidates"
  );
  assert!(as_bool(get(runtime, "can-appear-at-runtime")));
  assert!(!as_bool(get(runtime, "canonical-runtime-installed")));
  assert!(!as_bool(get(runtime, "executable-now")));
  assert!(as_bool(get(runtime, "candidate-only")));
  assert!(!as_bool(get(runtime, "owner-switch")));
  assert!(!as_bool(get(runtime, "nix-checks-gate-added")));
  assert_eq!(as_list(get(runtime, "runtime-added-candidates")).len(), 6);
}

#[test]
fn discoveries_record_d65_through_d73() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 9);
  for expected in [
    "D65.firewall-gate-sequence-is-ordered-and-held-on-first-fail",
    "D66.each-firewall-gate-emits-independent-held",
    "D67.firewall-pass-is-candidate-cleared-not-brain-accepted",
    "D68.audit-ref-preserved-through-firewall-flow",
    "D69.constructor-self-report-cannot-bypass-any-gate",
    "D70.green-tests-alone-cannot-substitute-for-owner-law-gate",
    "D71.held-firewall-failures-preserve-constructor-substance",
    "D72.first-fail-stops-enforcement-not-diagnostic-cognition",
    "D73.candidate-cleared-enters-promotion-queue-not-dead-end",
  ] {
    let d = discoveries
      .get(expected)
      .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
    assert!(as_bool(get(d, "scenario-only")));
    assert_eq!(as_str(get(d, "decision-pressure")), "keep");
  }
}

#[test]
fn affected_plans_and_blocks_keep_firewall_scoped() {
  let run = eval_file(&fixture_path()).unwrap();
  let affected = get(&run, "affected-plans");
  assert_eq!(
    as_str(get_path(affected, &["constructor", "pressure"])),
    "must-pass-five-gate-firewall-before-candidate-cleared"
  );
  assert_eq!(
    as_str(get_path(affected, &["pnix", "pressure"])),
    "route-cleared-candidates-through-pnix-promotion-queue"
  );
  assert_eq!(
    as_str(get_path(affected, &["human", "pressure"])),
    "authorize-consequence-bearing-promotion-not-global-cognition-brake"
  );
  for key in ["constructor", "pnix", "human", "project"] {
    assert!(!as_bool(get_path(
      affected,
      &[key, "implementation-target"]
    )));
  }

  let blocks = string_set(get(&run, "blocked-shortcuts"));
  for expected in [
    "merge-constructor-code-without-intent-receipt",
    "treat-candidate-cleared-as-canonical-promotion",
    "treat-candidate-cleared-as-dead-end",
    "discard-held-constructor-substance",
    "treat-first-fail-as-end-of-cognition",
    "treat-human-authorization-as-global-brake",
    "install-firewall-as-runtime-without-replay",
    "skip-firewall-gates-due-to-fluency",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }

  assert_eq!(as_str(get(&run, "replacement-readiness")), "not-proven");
  assert!(!as_bool(get(&run, "owner-switch")));
  assert!(!as_bool(get(&run, "implementation-command")));
}

#[test]
fn negative_held_evidence_lists_each_failing_trial() {
  let run = eval_file(&fixture_path()).unwrap();
  let negative = get(&run, "negative-held-evidence");
  assert_eq!(as_str(get(negative, "status")), "present");
  assert!(as_bool(get(negative, "held-at-each-gate")));
  let held_trials = string_set(get(negative, "held-trials"));
  for expected in [
    "trial.A.no-intent-receipt",
    "trial.B.fold-inconsistent",
    "trial.C.axis-collapse",
    "trial.D.regression",
    "trial.E.owner-law-violation",
  ] {
    assert!(
      held_trials.contains(expected),
      "missing held trial `{expected}`"
    );
  }
}
