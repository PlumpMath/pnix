//! R4 macro-native lift/query/emit rewrite candidate discovery.
//!
//! R3 pinned role emission for the dependent `ontologyLift` / `ontologyQuery`
//! / `ontologyEmit` triple. This test pins the next narrow boundary: R4 writes
//! a triple-scoped macro-native candidate only. It must not call old
//! lift/query/emit, install query runtime, create a fact store/event log,
//! treat a projection as owner, or switch owners.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join(
    "../../fixtures/tesseract-macro-legacy-probe/macro_native_lift_query_emit_rewrite_candidate_receipt.px",
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
fn lift_query_emit_r4_marker_and_constitution_owner_are_pinned() {
  let run = eval_file(&fixture_path()).expect("lift/query/emit R4 fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-r4-macro-native-lift-query-emit-rewrite-candidate"
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
fn constitution_gate_keeps_lift_query_emit_r4_candidate_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "r4-macro-native-lift-query-emit-rewrite-candidate"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  assert_eq!(as_str(get(gate, "replacement-readiness")), "not-proven");

  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "call-old-builtins.ontologyLift",
    "call-old-builtins.ontologyQuery",
    "call-old-builtins.ontologyEmit",
    "split-query-or-emit-rewrite-from-lift",
    "emit-source-object-as-ContextualFactStore",
    "emit-query-intent-as-answer",
    "emit-projection-specimen-as-semantic-owner",
    "install-query-runtime-from-r4",
    "install-audit-event-log-from-r4",
    "claim-replacement-readiness-at-r4",
    "treat-llm-prose-as-query-answer",
  ] {
    assert!(blocks.contains(expected), "missing gate block `{expected}`");
  }
}

#[test]
fn surface_triple_and_r3_input_are_imported() {
  let run = eval_file(&fixture_path()).unwrap();
  let triple = get(&run, "surface-triple");
  assert_eq!(
    as_str(get(triple, "lift")),
    "stdlib/lib/ontology.px::builtins.ontologyLift"
  );
  assert_eq!(
    as_str(get(triple, "query")),
    "stdlib/lib/ontology.px::builtins.ontologyQuery"
  );
  assert_eq!(
    as_str(get(triple, "emit")),
    "stdlib/lib/ontology.px::builtins.ontologyEmit"
  );
  assert!(as_bool(get(triple, "triple-required-for-r4")));
  assert_eq!(
    as_str(get(triple, "scope")),
    "legacy-lift-query-emit-triple-only"
  );

  let r3 = get(&run, "r3-input");
  assert_eq!(
    as_str(get(r3, "r3-verdict-ref")),
    "tesseract-macro-ontology-r3-lift-query-emit-role-emission-verdict"
  );
  assert!(as_bool(get(
    r3,
    "r3-verdict-closed-for-this-surface-triple"
  )));
  assert!(as_bool(get(
    r3,
    "r4-macro-native-rewrite-candidate-may-start"
  )));
  assert_eq!(as_list(get(r3, "emitted-roles")).len(), 11);
  assert_eq!(as_list(get(r3, "non-emitted-legacy-plan-roles")).len(), 7);
  assert_eq!(as_str(get(r3, "replacement-readiness")), "not-proven");
  assert!(!as_bool(get(r3, "owner-switch")));
  assert!(!as_bool(get(r3, "runtime-install")));
}

#[test]
fn rewrite_candidate_is_triple_macro_native_and_non_executable() {
  let run = eval_file(&fixture_path()).unwrap();
  let candidate = get(&run, "rewrite-candidate");
  assert_eq!(
    as_str(get(candidate, "id")),
    "r4.macro-native-lift-query-emit.rewrite-candidate"
  );
  assert_eq!(as_str(get(candidate, "phase")), "R4");
  assert_eq!(
    as_str(get(candidate, "candidate-kind")),
    "macro-native-triple-rewrite-candidate"
  );
  assert_eq!(
    as_str(get(candidate, "scope")),
    "legacy-lift-query-emit-triple-only"
  );
  assert!(as_bool(get(candidate, "triple-required")));
  assert_eq!(as_list(get(candidate, "surfaces")).len(), 3);
  assert_eq!(as_list(get(candidate, "uses-emitted-r3-roles")).len(), 11);
  assert!(!as_bool(get(candidate, "calls-legacy-ontologyLift")));
  assert!(!as_bool(get(candidate, "calls-legacy-ontologyQuery")));
  assert!(!as_bool(get(candidate, "calls-legacy-ontologyEmit")));
  assert!(as_bool(get(candidate, "candidate-only")));
  assert!(!as_bool(get(candidate, "executable-now")));
  assert!(!as_bool(get(candidate, "installed")));
  assert_eq!(
    as_str(get(candidate, "output-status")),
    "ready-for-r5-reverse-replay"
  );
}

#[test]
fn rewrite_candidate_preserves_store_query_projection_and_runtime_boundaries() {
  let run = eval_file(&fixture_path()).unwrap();
  let candidate = get(&run, "rewrite-candidate");
  assert!(as_bool(get(candidate, "uses-source-object-candidate")));
  assert!(as_bool(get(candidate, "uses-query-intent")));
  assert!(as_bool(get(candidate, "uses-query-need")));
  assert!(as_bool(get(candidate, "uses-projection-surface-set")));
  assert!(as_bool(get(candidate, "uses-held-result-boundary")));
  assert!(as_bool(get(candidate, "uses-reverse-replay-requirement")));
  assert!(!as_bool(get(candidate, "emits-contextualfact-store")));
  assert!(!as_bool(get(candidate, "emits-query-answer-engine")));
  assert!(!as_bool(get(candidate, "emits-needcursor-store")));
  assert!(!as_bool(get(
    candidate,
    "emits-expression-projection-owner"
  )));
  assert!(!as_bool(get(candidate, "emits-audit-event-log")));
  assert!(!as_bool(get(candidate, "emits-doghouse-store")));
  assert!(!as_bool(get(candidate, "emits-global-ontology-runtime")));
  assert_eq!(
    as_str(get(candidate, "replacement-readiness")),
    "not-proven"
  );
  assert!(!as_bool(get(candidate, "query-runtime-install")));
  assert!(!as_bool(get(candidate, "audit-event-log-install")));
}

#[test]
fn rewrite_steps_use_r3_roles_and_end_at_r5_need() {
  let run = eval_file(&fixture_path()).unwrap();
  let steps = attrs_by_id(get(&run, "rewrite-steps"));
  assert_eq!(steps.len(), 8);
  for (id, role) in [
    ("step.1.load-triple-surfaces", "LiftQueryEmitRewriteNeed"),
    (
      "step.2.lower-lift-to-source-object",
      "SourceObjectCandidate",
    ),
    ("step.3.attach-provenance-pressure", "ProvenancePressure"),
    ("step.4.lower-query-to-intent-and-need", "QueryIntent"),
    (
      "step.5.lower-emit-to-projection-specimen",
      "ProjectionEventSpecimen",
    ),
    ("step.6.attach-owner-and-audit-needs", "SemanticOwnerNeed"),
    (
      "step.7.attach-held-boundaries-and-reference-deltas",
      "HeldResultBoundary",
    ),
    ("step.8.emit-r5-replay-need", "ReverseReplayRequirement"),
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
      steps.get("step.8.emit-r5-replay-need").unwrap(),
      "emits"
    )),
    "need.r5.liftqueryemit-reverse-replay"
  );
}

#[test]
fn reference_deltas_are_explicit_before_r5() {
  let run = eval_file(&fixture_path()).unwrap();
  let deltas = attrs_by_id(get(&run, "reference-deltas"));
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
    assert!(as_bool(get(delta, "allowed")));
  }
  assert_eq!(
    as_str(get(deltas.get("delta.query-answer").unwrap(), "macro")),
    "QueryIntent-plus-QueryNeed"
  );
  assert_eq!(
    as_str(get(deltas.get("delta.emit-projection").unwrap(), "macro")),
    "ProjectionEventSpecimen-plus-ProjectionSurfaceSet"
  );
}

#[test]
fn held_rewrite_trials_cover_r3_triple_legacy_store_query_projection_and_runtime_failures() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "held-rewrite-trials"));
  assert_eq!(trials.len(), 9);

  for expected in [
    "trial.A.r3-verdict-missing",
    "trial.B.triple-split",
    "trial.C.legacy-call-requested",
    "trial.D.fact-store-requested",
    "trial.E.query-answer-engine-requested",
    "trial.F.projection-owner-requested",
    "trial.G.audit-log-install-requested",
    "trial.H.runtime-install-requested",
  ] {
    let trial = trials
      .get(expected)
      .unwrap_or_else(|| panic!("missing trial `{expected}`"));
    assert_eq!(as_str(get(trial, "outcome")), "Held");
    assert!(!as_bool(get(trial, "accepted")));
    assert_ne!(as_str(get(trial, "reopen-path")), "not-needed");
  }

  let complete = trials.get("trial.I.complete-triple-candidate").unwrap();
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
    "r4-macro-native-lift-query-emit-rewrite-candidate"
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
    &["semantic", "lift-demoted-to-sourceobject-candidate"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "query-demoted-to-intent-and-need"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "emit-demoted-to-projection-specimen"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "semantic-owner-remains-need"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["runtime", "query-runtime-installed"]
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
    "replay-r4-lift-query-emit-candidate-against-legacy-triple-specimens"
  );
  assert_eq!(
    as_str(get(r5, "replacement-readiness-after-r4")),
    "not-proven"
  );
  assert!(!as_bool(get(r5, "owner-switch-open")));
  assert!(!as_bool(get(r5, "runtime-install-open")));
  assert!(!as_bool(get(r5, "query-runtime-install-open")));
  assert!(!as_bool(get(r5, "event-log-install-open")));

  let required = string_set(get(r5, "required-next"));
  for expected in [
    "replay-lift-sourceobject-and-provenance-against-lift-specimen",
    "replay-query-intent-and-need-against-query-specimen",
    "replay-emit-projection-surface-against-emit-specimen",
    "replay-semantic-owner-and-audit-needs",
    "preserve-negative-held-evidence",
    "emit-held-if-unexplained-answer-or-projection-mismatch",
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
    "r4-macro-native-lift-query-emit-runtime-candidates"
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
fn discoveries_record_d291_through_d299() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 9);
  for expected in [
    "D291.lift-query-emit-r4-rewrite-is-triple-scoped",
    "D292.lift-r4-lowers-to-sourceobject-not-contextualfact-store",
    "D293.query-r4-lowers-to-intent-and-need-not-answer-engine",
    "D294.emit-r4-lowers-to-projection-specimen-not-owner-or-audit-log",
    "D295.semantic-owner-and-audit-remain-r5-needs",
    "D296.no-store-query-engine-event-log-doghouse-or-global-runtime-at-r4",
    "D297.lift-query-emit-r4-opens-r5-only",
    "D298.reference-deltas-cover-lift-query-emit-authority-changes",
    "D299.lift-query-emit-runtime-candidates-remain-non-executable",
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
    "held-lift-as-sourceobject-not-store"
  );
  assert_eq!(
    as_str(get_path(affected, &["retrievalTimeInference", "pressure"])),
    "held-query-as-intent-not-answer"
  );
  assert_eq!(
    as_str(get_path(affected, &["AuditEventLog", "pressure"])),
    "held-emit-before-audit-receipt-owner"
  );
  assert_eq!(
    as_str(get_path(affected, &["liftQueryEmitRewrite", "pressure"])),
    "ready-for-r5-reverse-replay"
  );
  assert_eq!(
    as_str(get_path(affected, &["ownerSwitch", "pressure"])),
    "forbidden-at-r4"
  );
  for key in [
    "ContextualFactStore",
    "retrievalTimeInference",
    "NeedCursor",
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
fn negative_held_evidence_blocks_old_wrappers_store_query_runtime_and_prose() {
  let run = eval_file(&fixture_path()).unwrap();
  let negative = get(&run, "negative-held-evidence");
  assert_eq!(as_str(get(negative, "status")), "present");
  let rejects = string_set(get(negative, "rejects"));
  for expected in [
    "r3-verdict-missing",
    "triple-split-without-proof",
    "legacy-lift-query-emit-call",
    "lifted-candidate-as-contextualfact-store",
    "query-intent-as-answer-engine",
    "query-need-as-needcursor-store",
    "projection-specimen-as-semantic-owner",
    "emit-output-as-audit-log-proof",
    "query-runtime-install-at-r4",
    "event-log-install-at-r4",
    "doghouse-or-global-runtime-install-at-r4",
    "llm-prose-query-answer",
  ] {
    assert!(rejects.contains(expected), "missing rejection `{expected}`");
  }
}

#[test]
fn blocked_shortcuts_cover_all_r4_lift_query_emit_collapses() {
  let run = eval_file(&fixture_path()).unwrap();
  let blocks = string_set(get(&run, "blocked-shortcuts"));
  for expected in [
    "call-old-builtins.ontologyLift",
    "call-old-builtins.ontologyQuery",
    "call-old-builtins.ontologyEmit",
    "split-query-or-emit-rewrite-from-lift",
    "emit-source-object-as-ContextualFactStore",
    "emit-query-intent-as-answer",
    "emit-query-need-as-NeedCursorStore",
    "emit-projection-specimen-as-semantic-owner",
    "emit-projection-as-audit-proof",
    "install-query-runtime-from-r4",
    "install-audit-event-log-from-r4",
    "claim-replacement-readiness-at-r4",
    "treat-llm-prose-as-query-answer",
  ] {
    assert!(
      blocks.contains(expected),
      "missing shortcut block `{expected}`"
    );
  }
}

#[test]
fn top_level_state_keeps_lift_query_emit_unproven_without_runtime_or_owner_switch() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "reverse-replay-status")),
    "required-not-run"
  );
  assert_eq!(as_str(get(&run, "replacement-readiness")), "not-proven");
  assert!(!as_bool(get(&run, "owner-switch")));
  assert!(!as_bool(get(&run, "runtime-install")));
  assert!(!as_bool(get(&run, "query-runtime-install")));
  assert!(!as_bool(get(&run, "audit-event-log-install")));
  assert!(!as_bool(get(&run, "global-ontology-runtime")));
  assert!(!as_bool(get(&run, "implementation-command")));
}
