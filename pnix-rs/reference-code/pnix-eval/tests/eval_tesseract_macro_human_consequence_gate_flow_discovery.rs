//! Human consequence gate flow discovery.
//!
//! D48 named the risk-budget owner as an open frontier. D51 said the human
//! consequence owner closes that frontier for consequence-bearing action. The
//! constructor paper-note firewall flow ends in candidate-cleared, then PNIX
//! scope routing decides whether the candidate is consequence-bearing or
//! internal cognition. This test pins the operational closure: only
//! consequence-bearing candidates enter the staged human gate; internal
//! self-knowledge / repair / analysis stays in PNIX lifecycle. Human Held
//! pauses the consequence lane, not PNIX cognition.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join(
    "../../fixtures/tesseract-macro-legacy-probe/human_consequence_gate_flow_discovery_receipt.px",
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
fn consequence_gate_marker_and_constitution_owner_are_pinned() {
  let run = eval_file(&fixture_path()).expect("consequence gate fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-human-consequence-gate-flow"
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
fn constitution_gate_keeps_consequence_flow_candidate_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(as_str(get(gate, "scenario")), "human-consequence-gate-flow");
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  assert_eq!(as_str(get(gate, "replacement-readiness")), "not-proven");

  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "treat-firewall-cleared-as-consequence-authorized",
    "route-internal-cognition-through-human-consequence-gate",
    "skip-human-presentation-stage",
    "auto-approve-on-attention-overload",
    "infer-understanding-from-silence",
    "infer-risk-budget-from-default-value",
    "treat-consent-clickthrough-as-explicit-choice",
    "drop-audit-ref-on-rejection",
    "treat-accept-as-final-promotion",
    "treat-human-held-as-pnix-cognition-freeze",
    "treat-human-authorization-as-global-cognition-brake",
    "treat-d48-operational-closure-as-runtime-installed",
  ] {
    assert!(blocks.contains(expected), "missing gate block `{expected}`");
  }
}

#[test]
fn scope_router_limits_human_gate_to_consequence_bearing_candidates() {
  let run = eval_file(&fixture_path()).unwrap();
  let router = get(&run, "consequence-scope-routes");
  assert_eq!(as_str(get(router, "owner")), "pnix-lifecycle");
  assert_eq!(
    as_str(get(router, "position")),
    "after-firewall-candidate-cleared-before-human-consequence-gate"
  );

  let routes = attrs_by_id(get(router, "routes"));
  assert_eq!(routes.len(), 3);

  for route_id in [
    "route.internal-self-knowledge",
    "route.repair-only-paper-note",
  ] {
    let route = routes.get(route_id).unwrap();
    assert!(!as_bool(get(route, "consequence-bearing")));
    assert!(!as_bool(get(route, "enters-human-consequence-gate")));
    assert!(as_bool(get(route, "enters-pnix-lifecycle")));
    assert!(!as_bool(get(route, "requires-human-risk-budget")));
  }

  let consequence = routes.get("route.consequence-bearing-action").unwrap();
  assert!(as_bool(get(consequence, "consequence-bearing")));
  assert!(as_bool(get(consequence, "enters-human-consequence-gate")));
  assert!(as_bool(get(consequence, "enters-pnix-lifecycle")));
  assert!(as_bool(get(consequence, "requires-human-risk-budget")));

  let inv = get(router, "invariants");
  for key in [
    "consequence-gate-is-scope-limited",
    "internal-cognition-does-not-require-human-risk-budget",
    "repair-only-candidates-stay-in-pnix-lifecycle",
    "scope-routing-is-pnix-lifecycle-decision",
    "human-gate-is-not-global-promotion-owner",
  ] {
    assert!(
      as_bool(get(inv, key)),
      "scope invariant `{key}` must be true"
    );
  }
}

#[test]
fn five_stages_appear_in_strict_order() {
  let run = eval_file(&fixture_path()).unwrap();
  let stages = as_list(get(&run, "consequence-stages"));
  assert_eq!(stages.len(), 5);

  let expected_order = [
    "stage.1.presentation",
    "stage.2.attention-budget",
    "stage.3.understanding",
    "stage.4.risk-budget-allocation",
    "stage.5.choice",
  ];
  for (idx, stage) in stages.iter().enumerate() {
    assert_eq!(as_str(get(stage, "id")), expected_order[idx]);
    assert_eq!(as_number(get(stage, "order")), (idx + 1) as f64);
  }
}

#[test]
fn each_stage_blocks_specific_bypass_attempts() {
  let run = eval_file(&fixture_path()).unwrap();
  let stages = attrs_by_id(get(&run, "consequence-stages"));

  let attention = stages.get("stage.2.attention-budget").unwrap();
  let attention_blocks = string_set(get(attention, "bypass-attempts-blocked"));
  assert!(attention_blocks.contains("auto-approve-during-fatigue-window"));
  assert!(attention_blocks.contains("interpret-quick-yes-as-considered-judgement"));

  let understanding = stages.get("stage.3.understanding").unwrap();
  let understanding_blocks = string_set(get(understanding, "bypass-attempts-blocked"));
  assert!(understanding_blocks.contains("infer-understanding-from-silence"));

  let risk_budget = stages.get("stage.4.risk-budget-allocation").unwrap();
  let risk_blocks = string_set(get(risk_budget, "bypass-attempts-blocked"));
  assert!(risk_blocks.contains("use-default-risk-budget-without-explicit-confirmation"));
  assert!(risk_blocks.contains("treat-zero-budget-as-implicit-no-action"));

  let choice = stages.get("stage.5.choice").unwrap();
  let choice_blocks = string_set(get(choice, "bypass-attempts-blocked"));
  assert!(choice_blocks.contains("treat-clickthrough-as-explicit-choice"));
  assert!(choice_blocks.contains("convert-hold-into-implicit-accept-after-timeout"));
  assert!(choice_blocks.contains("treat-binary-yes-no-as-the-only-outcomes"));
}

#[test]
fn seven_trials_cover_each_stage_failure_plus_three_outcomes() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = as_list(get(&run, "consequence-trials"));
  assert_eq!(trials.len(), 7);

  let by_id = attrs_by_id(get(&run, "consequence-trials"));
  for expected in [
    "trial.A.presentation-incomplete",
    "trial.B.attention-overload",
    "trial.C.understanding-missing",
    "trial.D.risk-budget-unallocated",
    "trial.E.choice-hold",
    "trial.F.choice-reject",
    "trial.G.choice-accept",
  ] {
    assert!(by_id.contains_key(expected), "missing trial `{expected}`");
  }
}

#[test]
fn first_failing_stage_emits_held_and_blocks_consequence_authorization() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "consequence-trials"));

  let cases = [
    ("trial.A.presentation-incomplete", "stage.1.presentation", 0),
    ("trial.B.attention-overload", "stage.2.attention-budget", 1),
    ("trial.C.understanding-missing", "stage.3.understanding", 2),
    (
      "trial.D.risk-budget-unallocated",
      "stage.4.risk-budget-allocation",
      3,
    ),
    ("trial.E.choice-hold", "stage.5.choice", 4),
  ];

  for (trial_id, expected_failing_stage, expected_passed_count) in cases {
    let trial = trials.get(trial_id).unwrap();
    assert_eq!(
      as_str(get(trial, "first-failing-stage")),
      expected_failing_stage,
      "{trial_id} must fail at {expected_failing_stage}"
    );
    assert_eq!(as_str(get(trial, "verdict")), "Held");
    assert!(!as_bool(get(trial, "brain-mutation-applied")));
    assert!(!as_bool(get(trial, "consequence-authorized")));
    assert!(as_bool(get(trial, "consequence-lane-paused")));
    assert!(as_bool(get(trial, "pnix-cognition-continues")));
    assert!(as_bool(get(trial, "repair-candidate-emission-allowed")));
    assert!(as_bool(get(trial, "audit-ref-preserved")));
    assert_eq!(
      as_list(get(trial, "passed-stages")).len(),
      expected_passed_count
    );
  }
}

#[test]
fn reject_outcome_preserves_audit_without_consequence_authorization() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "consequence-trials"));
  let f = trials.get("trial.F.choice-reject").unwrap();

  assert_eq!(as_str(get(f, "verdict")), "rejected");
  assert!(!as_bool(get(f, "brain-mutation-applied")));
  assert!(!as_bool(get(f, "consequence-authorized")));
  assert!(as_bool(get(f, "audit-ref-preserved")));
  assert_eq!(as_list(get(f, "passed-stages")).len(), 5);

  let reason = as_str(get(f, "reason"));
  assert!(
    reason.contains("supersede chain"),
    "trial.F reason must reference supersede chain preservation"
  );
}

#[test]
fn accept_outcome_authorizes_pnix_lifecycle_without_bypassing_it() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "consequence-trials"));
  let g = trials.get("trial.G.choice-accept").unwrap();

  assert_eq!(as_str(get(g, "verdict")), "consequence-authorized");
  assert!(as_bool(get(g, "consequence-authorized")));
  // Brain mutation has not been applied even on accept; PNIX lifecycle
  // still needs to fold/replay/supersede before any Accepted state.
  assert!(!as_bool(get(g, "brain-mutation-applied")));
  assert!(as_bool(get(g, "audit-ref-preserved")));
  assert!(as_bool(get(g, "enters-pnix-lifecycle")));

  let reason = as_str(get(g, "reason"));
  assert!(
    reason.contains("does NOT bypass PNIX"),
    "trial.G reason must explicitly state accept does not bypass PNIX lifecycle"
  );
}

#[test]
fn flow_invariants_pin_load_bearing_constraints() {
  let run = eval_file(&fixture_path()).unwrap();
  let inv = get(&run, "flow-invariants");

  for key in [
    "stage-order-is-fixed",
    "consequence-gate-is-scope-limited",
    "internal-cognition-does-not-enter-human-consequence-gate",
    "internal-cognition-does-not-require-human-risk-budget",
    "first-fail-emits-held",
    "held-skips-remaining-consequence-stages-only",
    "held-pauses-consequence-lane-not-pnix-cognition",
    "held-allows-repair-candidate-emission",
    "audit-ref-preserved-through-all-outcomes",
    "rubber-stamp-pressure-is-held-not-implicit-accept",
    "silence-is-not-understanding",
    "default-value-is-not-explicit-budget",
    "accept-does-not-bypass-pnix-lifecycle",
    "human-authorization-is-not-global-cognition-brake",
    "reject-preserves-supersede-chain",
    "hold-preserves-resumption-context",
    "outcomes-are-non-binary",
    "human-cannot-be-replaced-by-machine-default",
    "d48-closure-is-operational-shape-not-runtime-install",
  ] {
    assert!(
      as_bool(get(inv, key)),
      "flow invariant `{key}` must be true"
    );
  }
}

#[test]
fn six_layer_fold_records_non_binary_outcome_and_pnix_routing() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-consequence-fold");
  assert_eq!(as_str(get(fold, "mode")), "human-consequence-gate-flow");
  for layer in [
    "surface", "ontology", "semantic", "gate", "runtime", "audit",
  ] {
    assert!(
      as_bool(get_path(fold, &[layer, "visible"])),
      "layer `{layer}` must be visible"
    );
  }
  assert_eq!(as_number(get_path(fold, &["surface", "stage-count"])), 5.0);
  assert_eq!(as_number(get_path(fold, &["surface", "trial-count"])), 7.0);
  assert!(as_bool(get_path(
    fold,
    &["semantic", "stage-order-visible"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "scope-limited-visible"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "non-binary-outcome-visible"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "rubber-stamp-discrimination-visible"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["runtime", "accept-routes-into-pnix-lifecycle"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["runtime", "internal-cognition-routes-around-human-gate"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["runtime", "held-continuation-candidate-only"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["audit", "audit-ref-preserved-on-accept"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["audit", "audit-ref-preserved-on-reject"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["audit", "audit-ref-preserved-on-hold"]
  )));
  assert!(as_bool(get_path(fold, &["audit", "scope-route-required"])));
  assert!(as_bool(get_path(
    fold,
    &["audit", "choice-reason-required"]
  )));
}

#[test]
fn held_continuation_keeps_pnix_cognition_alive_without_applying_consequence() {
  let run = eval_file(&fixture_path()).unwrap();
  let continuation = get(&run, "consequence-held-continuation");
  assert_eq!(as_str(get(continuation, "status")), "candidate-only");
  assert!(as_bool(get(continuation, "runs-after-held")));
  assert!(!as_bool(get(continuation, "applies-consequence")));
  assert!(as_bool(get(continuation, "can-refine-presentation")));
  assert!(as_bool(get(continuation, "can-split-candidate")));
  assert!(as_bool(get(continuation, "can-simulate-blast-radius")));
  assert!(as_bool(get(continuation, "can-request-more-evidence")));
  assert!(as_bool(get(continuation, "can-emit-repair-candidates")));
  assert!(as_bool(get(
    continuation,
    "cannot-mark-human-understanding"
  )));
  assert!(as_bool(get(
    continuation,
    "cannot-allocate-human-risk-budget"
  )));
  assert!(as_bool(get(continuation, "preserves-pnix-cognition")));
}

#[test]
fn self_knowledge_candidates_record_open_frontiers_in_consequence_flow() {
  let run = eval_file(&fixture_path()).unwrap();
  let candidates = as_list(get(&run, "self-knowledge-candidates"));
  assert_eq!(candidates.len(), 8);
  let ids: BTreeSet<&str> = candidates
    .iter()
    .map(|candidate| as_str(get(candidate, "id")))
    .collect();
  for expected in [
    "knowledge.consequence.scope-limited-not-global-brake",
    "knowledge.consequence.held-does-not-freeze-pnix-cognition",
    "knowledge.consequence.d48-risk-budget-owner-is-human",
    "need.consequence.attention-budget-signal-source",
    "need.consequence.risk-budget-vocabulary",
    "held.consequence.split-outcome-deferred",
    "held.consequence.d48-runtime-closure-not-proven",
    "held.consequence.no-runtime-installer-yet",
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
    "human-consequence-gate-flow-runtime-candidates"
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
fn discoveries_record_d74_through_d83() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 10);
  for expected in [
    "D74.human-consequence-gate-is-staged-not-binary",
    "D75.rubber-stamp-pressure-is-itself-a-held-mode",
    "D76.consequence-outcomes-are-non-binary",
    "D77.audit-ref-preserved-across-all-outcomes",
    "D78.accept-authorizes-pnix-lifecycle-not-bypasses-it",
    "D79.human-consequence-gate-operationally-closes-d48-risk-budget-frontier",
    "D80.consequence-gate-is-scope-limited-not-global-cognition-brake",
    "D81.human-held-pauses-consequence-lane-not-pnix-cognition",
    "D82.d48-closure-is-receipt-shape-not-runtime-install",
    "D83.human-authorization-is-not-global-cognition-authority",
  ] {
    let d = discoveries
      .get(expected)
      .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
    assert!(as_bool(get(d, "scenario-only")));
    assert_eq!(as_str(get(d, "decision-pressure")), "keep");
  }
}

#[test]
fn d48_frontier_status_records_operational_closure() {
  let run = eval_file(&fixture_path()).unwrap();
  let status = get(&run, "d48-frontier-status");
  assert_eq!(as_str(get(status, "previous-status")), "open-frontier");
  assert!(as_str(get(status, "closure-source")).contains("D51"));
  let operational = as_str(get(status, "operational-closure"));
  assert!(operational.contains("D74"));
  assert!(operational.contains("D83"));
  assert!(as_bool(get(status, "scope-limited")));
  assert!(!as_bool(get(status, "runtime-closure-proven")));
  assert!(!as_bool(get(status, "canonical-promotion")));
}

#[test]
fn affected_plans_and_blocks_keep_consequence_flow_scoped() {
  let run = eval_file(&fixture_path()).unwrap();
  let affected = get(&run, "affected-plans");
  assert_eq!(
    as_str(get_path(affected, &["human", "pressure"])),
    "operationalize-as-staged-gate-with-held-discrimination"
  );
  assert_eq!(
    as_str(get_path(affected, &["pnix", "pressure"])),
    "scope-router-keeps-internal-cognition-inside-pnix-lifecycle"
  );
  assert_eq!(
    as_str(get_path(affected, &["constructor", "pressure"])),
    "candidate-cleared-feeds-this-gate-not-bypasses-it"
  );
  for key in ["human", "pnix", "constructor", "project"] {
    assert!(!as_bool(get_path(
      affected,
      &[key, "implementation-target"]
    )));
  }

  let blocks = string_set(get(&run, "blocked-shortcuts"));
  for expected in [
    "treat-firewall-cleared-as-consequence-authorized",
    "route-internal-cognition-through-human-consequence-gate",
    "treat-accept-as-final-promotion",
    "collapse-non-binary-outcomes-into-yes-no",
    "treat-human-held-as-pnix-cognition-freeze",
    "treat-human-authorization-as-global-cognition-brake",
    "treat-d48-operational-closure-as-runtime-installed",
    "install-consequence-gate-as-runtime-without-replay",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }

  assert_eq!(as_str(get(&run, "replacement-readiness")), "not-proven");
  assert!(!as_bool(get(&run, "owner-switch")));
  assert!(!as_bool(get(&run, "implementation-command")));
}

#[test]
fn negative_held_evidence_lists_each_held_trial() {
  let run = eval_file(&fixture_path()).unwrap();
  let negative = get(&run, "negative-held-evidence");
  assert_eq!(as_str(get(negative, "status")), "present");
  assert!(as_bool(get(negative, "held-at-each-stage")));
  assert!(as_bool(get(negative, "consequence-lane-paused-only")));
  assert!(as_bool(get(negative, "pnix-cognition-continues")));
  let held_trials = string_set(get(negative, "held-trials"));
  for expected in [
    "trial.A.presentation-incomplete",
    "trial.B.attention-overload",
    "trial.C.understanding-missing",
    "trial.D.risk-budget-unallocated",
    "trial.E.choice-hold",
  ] {
    assert!(
      held_trials.contains(expected),
      "missing held trial `{expected}`"
    );
  }
}
