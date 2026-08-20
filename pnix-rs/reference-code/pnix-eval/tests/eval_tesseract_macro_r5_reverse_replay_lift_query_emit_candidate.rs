//! R5 reverse replay for the macro-native lift/query/emit candidate.
//!
//! R4 wrote a triple-scoped candidate for `ontologyLift` / `ontologyQuery` /
//! `ontologyEmit`. R5 replays that candidate against the legacy triple
//! specimens and checks each reference delta. Replay success is evidence for a
//! future readiness receipt; it is not readiness, owner switch, query runtime,
//! audit event log, or global ontology runtime install.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join(
    "../../fixtures/tesseract-macro-legacy-probe/reverse_replay_lift_query_emit_candidate_receipt.px",
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
fn lift_query_emit_r5_marker_and_constitution_owner_are_pinned() {
  let run = eval_file(&fixture_path()).expect("lift/query/emit R5 fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-r5-reverse-replay-lift-query-emit-candidate"
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
fn constitution_gate_keeps_lift_query_emit_r5_candidate_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "r5-reverse-replay-lift-query-emit-candidate"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  assert_eq!(as_str(get(gate, "replacement-readiness")), "not-proven");

  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "treat-r4-green-candidate-as-replay",
    "replay-query-or-emit-without-lift",
    "drop-lift-provenance-replay",
    "drop-query-intent-need-replay",
    "drop-projection-surface-replay",
    "treat-query-intent-as-answer",
    "treat-projection-specimen-as-semantic-owner",
    "treat-emit-output-as-audit-proof",
    "drop-reference-delta-check",
    "ignore-unexplained-answer-or-projection-mismatch",
    "install-query-runtime-from-r5",
    "install-audit-event-log-from-r5",
    "treat-r5-replay-as-owner-switch",
    "claim-replacement-readiness-from-r5-alone",
    "treat-llm-prose-as-replay-result",
  ] {
    assert!(blocks.contains(expected), "missing gate block `{expected}`");
  }
}

#[test]
fn replay_inputs_link_r4_candidate_to_legacy_triple_specimens() {
  let run = eval_file(&fixture_path()).unwrap();
  let legacy = get(&run, "legacy-replay-specimens");
  let lift = get(legacy, "lift");
  let query = get(legacy, "query");
  let emit = get(legacy, "emit");

  assert_eq!(as_str(get(lift, "source-symbol")), "builtins.ontologyLift");
  assert_eq!(
    as_str(get(lift, "expected-output-shape")),
    "input-attrs-plus-ontology-context-and-Candidate-status"
  );
  assert_eq!(
    as_str(get(lift, "expected-provenance-refs")),
    "only-when-string-context-is-visible"
  );
  assert!(!as_bool(get(lift, "current-authority")));

  assert_eq!(
    as_str(get(query, "source-symbol")),
    "builtins.ontologyQuery"
  );
  assert_eq!(
    as_str(get(query, "expected-output-shape")),
    "query-kind-envelope"
  );
  assert!(!as_bool(get(query, "store-lookup")));
  assert!(as_bool(get(query, "preserves-custom-query-kind")));
  assert!(!as_bool(get(query, "current-authority")));

  assert_eq!(as_str(get(emit, "source-symbol")), "builtins.ontologyEmit");
  assert_eq!(
    as_str(get(emit, "expected-output-shape")),
    "expression-projection-with-four-surface-forms"
  );
  assert!(!as_bool(get(emit, "event-log-write")));
  assert_eq!(as_str(get(emit, "default-projection-family")), "expmath");
  assert!(!as_bool(get(emit, "current-authority")));
}

#[test]
fn r4_replay_target_preserves_triple_candidate_boundaries() {
  let run = eval_file(&fixture_path()).unwrap();
  let r4 = get(&run, "r4-replay-target");
  assert_eq!(
    as_str(get(r4, "id")),
    "r4.macro-native-lift-query-emit.rewrite-candidate"
  );
  assert_eq!(
    as_str(get(r4, "scope")),
    "legacy-lift-query-emit-triple-only"
  );
  assert!(as_bool(get(r4, "triple-required")));
  assert!(as_bool(get(r4, "uses-source-object-candidate")));
  assert!(as_bool(get(r4, "uses-query-intent")));
  assert!(as_bool(get(r4, "uses-query-need")));
  assert!(as_bool(get(r4, "uses-projection-surface-set")));
  assert!(as_bool(get(r4, "uses-held-result-boundary")));
  assert!(!as_bool(get(r4, "calls-legacy-ontologyLift")));
  assert!(!as_bool(get(r4, "calls-legacy-ontologyQuery")));
  assert!(!as_bool(get(r4, "calls-legacy-ontologyEmit")));
  assert!(!as_bool(get(r4, "emits-contextualfact-store")));
  assert!(!as_bool(get(r4, "emits-query-answer-engine")));
  assert!(!as_bool(get(r4, "emits-audit-event-log")));
  assert!(!as_bool(get(r4, "emits-global-ontology-runtime")));
  assert!(!as_bool(get(r4, "query-runtime-install")));
  assert!(!as_bool(get(r4, "audit-event-log-install")));
  assert_eq!(as_list(get(r4, "reference-deltas")).len(), 7);
}

#[test]
fn replay_steps_cover_triple_specimens_deltas_and_readiness_need() {
  let run = eval_file(&fixture_path()).unwrap();
  let steps = attrs_by_id(get(&run, "replay-steps"));
  assert_eq!(steps.len(), 8);
  for expected in [
    "step.1.load-r4-triple-candidate",
    "step.2.load-legacy-lift-specimen",
    "step.3.load-legacy-query-specimen",
    "step.4.load-legacy-emit-specimen",
    "step.5.replay-lift-sourceobject-provenance",
    "step.6.replay-query-intent-and-need",
    "step.7.replay-emit-projection-and-audit-needs",
    "step.8.emit-readiness-need",
  ] {
    let step = steps
      .get(expected)
      .unwrap_or_else(|| panic!("missing `{expected}`"));
    assert!(!as_bool(get(step, "held")));
  }
  assert_eq!(
    as_str(get(
      steps.get("step.8.emit-readiness-need").unwrap(),
      "outcome"
    )),
    "need.liftqueryemit.replacement-readiness-receipt"
  );
}

#[test]
fn all_r4_reference_deltas_are_observed_and_covered() {
  let run = eval_file(&fixture_path()).unwrap();
  let deltas = attrs_by_id(get(&run, "delta-verdicts"));
  assert_eq!(deltas.len(), 7);
  for expected in [
    "delta.lift-authority",
    "delta.query-answer",
    "delta.emit-projection",
    "delta.semantic-owner",
    "delta.audit-proof",
    "delta.runtime-and-store",
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
fn replay_comparisons_cover_lift_query_emit_owner_audit_and_runtime() {
  let run = eval_file(&fixture_path()).unwrap();
  let comparisons = attrs_by_id(get(&run, "replay-comparisons"));
  assert_eq!(comparisons.len(), 7);
  for expected in [
    "compare.lift-output-shape",
    "compare.query-output-shape",
    "compare.query-store-lookup",
    "compare.emit-output-shape",
    "compare.semantic-owner",
    "compare.audit-proof",
    "compare.runtime-store",
  ] {
    let cmp = comparisons
      .get(expected)
      .unwrap_or_else(|| panic!("missing comparison `{expected}`"));
    assert_eq!(as_str(get(cmp, "verdict")), "covered-delta");
    assert!(!as_bool(get(cmp, "held")));
  }
  assert_eq!(
    as_str(get(
      comparisons.get("compare.semantic-owner").unwrap(),
      "macro-value"
    )),
    "SemanticOwnerNeed"
  );
}

#[test]
fn replay_trials_hold_missing_inputs_mismatches_runtime_and_readiness_claims() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "replay-trials"));
  assert_eq!(trials.len(), 12);
  for expected in [
    "trial.A.r4-candidate-missing",
    "trial.B.legacy-lift-specimen-missing",
    "trial.C.legacy-query-specimen-missing",
    "trial.D.legacy-emit-specimen-missing",
    "trial.E.delta-set-missing",
    "trial.F.lift-provenance-mismatch",
    "trial.G.query-answer-claim",
    "trial.H.projection-owner-claim",
    "trial.I.audit-ref-lost",
    "trial.J.runtime-install-requested",
    "trial.K.replacement-readiness-requested",
  ] {
    let trial = trials
      .get(expected)
      .unwrap_or_else(|| panic!("missing trial `{expected}`"));
    assert_eq!(as_str(get(trial, "outcome")), "Held");
    assert!(!as_bool(get(trial, "replay-verified")));
    assert_ne!(as_str(get(trial, "rewrite-debt")), "none");
  }

  let complete = trials.get("trial.L.complete-replay").unwrap();
  assert_eq!(as_str(get(complete, "outcome")), "reverse-replay-verified");
  assert!(as_bool(get(complete, "replay-verified")));
}

#[test]
fn audit_trace_preserves_legacy_r3_r4_and_r5_refs() {
  let run = eval_file(&fixture_path()).unwrap();
  let audit = get(&run, "audit-trace");
  assert_eq!(
    as_str(get(audit, "lift-ref")),
    "audit.r5.legacy-specimen.ontologyLift.candidate-fact"
  );
  assert_eq!(
    as_str(get(audit, "query-ref")),
    "audit.r5.legacy-specimen.ontologyQuery.envelope"
  );
  assert_eq!(
    as_str(get(audit, "emit-ref")),
    "audit.r5.legacy-specimen.ontologyEmit.projection"
  );
  assert_eq!(
    as_str(get(audit, "r3-ref")),
    "tesseract-macro-ontology-r3-lift-query-emit-role-emission-verdict"
  );
  assert_eq!(
    as_str(get(audit, "r4-ref")),
    "audit.r4.macro-native-lift-query-emit.rewrite-candidate"
  );
  assert_eq!(
    as_str(get(audit, "r5-ref")),
    "audit.r5.reverse-replay.lift-query-emit-candidate"
  );
  assert!(as_bool(get(audit, "refs-preserved")));
  assert_eq!(as_i64(get(audit, "replay-step-count")), 8);
  assert_eq!(as_i64(get(audit, "delta-verdict-count")), 7);
  assert_eq!(as_i64(get(audit, "comparison-count")), 7);
  assert!(as_bool(get(audit, "negative-held-present")));
}

#[test]
fn reverse_replay_verdict_opens_readiness_work_not_readiness_owner_or_runtime() {
  let run = eval_file(&fixture_path()).unwrap();
  let verdict = get(&run, "reverse-replay-verdict");
  assert_eq!(
    as_str(get(verdict, "surface-triple")),
    "surface-triple.legacy-ontology.lift-query-emit"
  );
  assert_eq!(as_str(get(verdict, "replay-kind")), "reverse-replay");
  assert!(!as_bool(get(verdict, "reverse-turn-instance")));
  assert!(as_bool(get(verdict, "triple-replay")));
  assert!(as_bool(get(verdict, "all-deltas-covered")));
  assert!(as_bool(get(verdict, "lift-provenance-covered")));
  assert!(as_bool(get(verdict, "query-intent-need-covered")));
  assert!(as_bool(get(verdict, "emit-projection-surface-covered")));
  assert!(as_bool(get(verdict, "semantic-owner-need-covered")));
  assert!(as_bool(get(verdict, "audit-receipt-need-covered")));
  assert!(!as_bool(get(verdict, "unexplained-mismatch")));
  assert_eq!(as_str(get(verdict, "verdict")), "reverse-replay-verified");
  assert_eq!(
    as_str(get(verdict, "replacement-readiness-after-r5")),
    "not-proven"
  );
  assert!(!as_bool(get(verdict, "owner-switch-open")));
  assert!(!as_bool(get(verdict, "runtime-install-open")));
  assert!(!as_bool(get(verdict, "query-runtime-install-open")));
  assert!(!as_bool(get(verdict, "event-log-install-open")));

  let required = string_set(get(verdict, "next-required"));
  for expected in [
    "lift-query-emit-replacement-readiness-receipt",
    "semantic-owner-proof",
    "audit-receipt-owner",
    "negative-held-retention",
    "query-projection-regression-corpus-binding",
  ] {
    assert!(
      required.contains(expected),
      "missing next requirement `{expected}`"
    );
  }
}

#[test]
fn six_layer_replay_fold_preserves_triple_and_runtime_boundaries() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-replay-fold");
  assert_eq!(
    as_str(get(fold, "mode")),
    "r5-reverse-replay-lift-query-emit-candidate"
  );
  for layer in [
    "surface", "ontology", "semantic", "gate", "runtime", "audit",
  ] {
    assert!(
      as_bool(get_path(fold, &[layer, "visible"])),
      "layer `{layer}` must stay visible"
    );
  }
  assert!(as_bool(get_path(fold, &["surface", "triple-required"])));
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
    &["ontology", "contextualfact-store-emitted"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["ontology", "query-answer-engine-emitted"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["ontology", "audit-event-log-emitted"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["ontology", "global-ontology-runtime-emitted"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "lift-provenance-covered"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "query-intent-need-covered"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "emit-projection-surface-covered"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "semantic-owner-need-covered"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "audit-receipt-need-covered"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["runtime", "query-runtime-installed"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["runtime", "audit-event-log-installed"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["runtime", "global-ontology-runtime"]
  )));
  assert_eq!(
    as_str(get_path(fold, &["audit", "r5-verdict"])),
    "reverse-replay-verified"
  );
}

#[test]
fn runtime_observation_is_candidate_only_and_not_installed() {
  let run = eval_file(&fixture_path()).unwrap();
  let runtime = get(&run, "runtime-observation");
  assert_eq!(
    as_str(get(runtime, "observation-model")),
    "r5-reverse-replay-lift-query-emit-runtime-candidates"
  );
  assert!(as_bool(get(runtime, "can-appear-at-runtime")));
  assert!(as_bool(get(runtime, "candidate-only")));
  assert!(!as_bool(get(runtime, "canonical-runtime-installed")));
  assert!(!as_bool(get(runtime, "executable-now")));
  assert!(!as_bool(get(runtime, "owner-switch")));
  assert!(!as_bool(get(runtime, "query-runtime-installed")));
  assert!(!as_bool(get(runtime, "audit-event-log-installed")));
  assert_eq!(as_list(get(runtime, "runtime-added-candidates")).len(), 4);
}

#[test]
fn discoveries_record_d300_through_d308() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 9);
  for expected in [
    "D300.lift-query-emit-r5-replay-is-triple-scoped",
    "D301.lift-provenance-delta-is-replayed-against-lift-specimen",
    "D302.query-intent-need-delta-is-replayed-against-query-specimen",
    "D303.emit-projection-surface-delta-is-replayed-against-emit-specimen",
    "D304.semantic-owner-and-audit-needs-survive-replay",
    "D305.unexplained-answer-or-projection-mismatch-emits-held-and-rewrite-debt",
    "D306.audit-refs-preserve-r3-r4-r5-and-triple-specimen-lineage",
    "D307.lift-query-emit-r5-verifies-replay-not-readiness",
    "D308.lift-query-emit-replay-success-is-receipt-driven-not-prose",
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
    as_str(get_path(affected, &["ContextualFactStore", "pressure"])),
    "held-lift-replay-is-evidence-not-store"
  );
  assert_eq!(
    as_str(get_path(affected, &["retrievalTimeInference", "pressure"])),
    "held-query-replay-is-intent-not-answer"
  );
  assert_eq!(
    as_str(get_path(
      affected,
      &["ExpressionProjectionOwner", "pressure"]
    )),
    "held-emit-replay-is-projection-surface-not-owner"
  );
  assert_eq!(
    as_str(get_path(affected, &["AuditEventLog", "pressure"])),
    "held-audit-replay-is-receipt-need-not-event-log"
  );
  assert_eq!(
    as_str(get_path(affected, &["liftQueryEmitRewrite", "pressure"])),
    "advance-to-replacement-readiness-receipt"
  );
  for key in [
    "ContextualFactStore",
    "retrievalTimeInference",
    "ExpressionProjectionOwner",
    "AuditEventLog",
    "liftQueryEmitRewrite",
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
    "query-or-emit-only-replay",
    "lift-provenance-replay-drop",
    "query-intent-need-replay-drop",
    "projection-surface-replay-drop",
    "query-intent-as-answer",
    "projection-specimen-as-semantic-owner",
    "emit-output-as-audit-proof",
    "uncovered-reference-delta",
    "unexplained-answer-or-projection-mismatch",
    "audit-ref-loss",
    "query-runtime-install-at-r5",
    "event-log-install-at-r5",
    "owner-switch-at-r5",
    "replacement-readiness-from-r5-alone",
    "llm-prose-as-replay-result",
  ] {
    assert!(rejects.contains(expected), "missing rejection `{expected}`");
  }
}

#[test]
fn blocked_shortcuts_cover_r5_lift_query_emit_collapse_modes() {
  let run = eval_file(&fixture_path()).unwrap();
  let blocks = string_set(get(&run, "blocked-shortcuts"));
  for expected in [
    "treat-r4-green-candidate-as-replay",
    "replay-query-or-emit-without-lift",
    "drop-lift-provenance-replay",
    "drop-query-intent-need-replay",
    "drop-projection-surface-replay",
    "treat-query-intent-as-answer",
    "treat-projection-specimen-as-semantic-owner",
    "treat-emit-output-as-audit-proof",
    "drop-reference-delta-check",
    "ignore-unexplained-answer-or-projection-mismatch",
    "drop-audit-ref",
    "install-query-runtime-from-r5",
    "install-audit-event-log-from-r5",
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
fn top_level_state_keeps_lift_query_emit_not_ready_after_replay() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(as_str(get(&run, "reverse-replay-status")), "verified");
  assert_eq!(as_str(get(&run, "replacement-readiness")), "not-proven");
  assert!(!as_bool(get(&run, "owner-switch")));
  assert!(!as_bool(get(&run, "runtime-install")));
  assert!(!as_bool(get(&run, "query-runtime-install")));
  assert!(!as_bool(get(&run, "audit-event-log-install")));
  assert!(!as_bool(get(&run, "global-ontology-runtime")));
  assert!(!as_bool(get(&run, "implementation-command")));
}
