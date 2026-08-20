//! R3 role-emission verdict for legacy ontologyLift / ontologyQuery / ontologyEmit.
//!
//! D8-D10 turned lift/query/emit into specimen evidence. This test pins the
//! next boundary: the macro fold emits source/query/projection roles and Held
//! needs, but does not create a fact store, query answer engine, event log, or
//! runtime authority.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join(
    "../../fixtures/tesseract-macro-legacy-probe/role_emission_lift_query_emit_verdict_receipt.px",
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
fn lift_query_emit_r3_marker_and_constitution_owner_are_pinned() {
  let run = eval_file(&fixture_path()).expect("lift/query/emit R3 fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-r3-lift-query-emit-role-emission-verdict"
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
fn constitution_gate_keeps_lift_query_emit_r3_candidate_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "r3-role-emission-lift-query-emit-verdict"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "lifted-candidate-as-Accepted-fact",
    "query-envelope-as-answer",
    "emit-projection-as-audit-proof",
    "projection-record-as-semantic-owner",
    "ContextualFactStore-from-lift",
    "NeedCursor-from-query-intent",
    "AuditEventLog-from-emit",
    "install-query-runtime-from-r3",
    "treat-llm-prose-as-query-answer",
  ] {
    assert!(blocks.contains(expected), "missing gate block `{expected}`");
  }
}

#[test]
fn source_surfaces_remain_lift_query_emit_specimens() {
  let run = eval_file(&fixture_path()).unwrap();
  let surfaces = as_list(get(&run, "source-surfaces"));
  assert_eq!(surfaces.len(), 3);
  let symbols: BTreeSet<&str> = surfaces
    .iter()
    .map(|surface| as_str(get(surface, "source-symbol")))
    .collect();
  assert_eq!(
    symbols,
    [
      "builtins.ontologyLift",
      "builtins.ontologyQuery",
      "builtins.ontologyEmit",
    ]
    .into_iter()
    .collect()
  );
  for surface in surfaces {
    assert_eq!(as_str(get(surface, "specimen-role")), "reference-specimen");
  }
}

#[test]
fn surface_triple_is_scoped_and_non_global() {
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
  assert_eq!(
    as_str(get(triple, "scope")),
    "legacy-lift-query-emit-triple-only"
  );
  assert!(as_bool(get(triple, "triple-required-for-r4")));
  assert!(!as_bool(get(triple, "global-ontology-runtime")));
}

#[test]
fn specimen_evidence_imports_d8_through_d10_behavior() {
  let run = eval_file(&fixture_path()).unwrap();
  let evidence = get(&run, "specimen-evidence");
  assert_eq!(
    as_str(get(evidence, "lift-output-shape")),
    "input-attrs-plus-ontology-context-and-Candidate-status"
  );
  assert_eq!(
    as_str(get(evidence, "query-output-shape")),
    "query-kind-envelope"
  );
  assert_eq!(
    as_str(get(evidence, "emit-output-shape")),
    "expression-projection-with-four-surface-forms"
  );
  assert!(!as_bool(get(evidence, "query-store-lookup")));
  assert!(!as_bool(get(evidence, "emit-event-log-write")));
  assert!(!as_bool(get(evidence, "lift-current-authority")));
  assert!(!as_bool(get(evidence, "query-current-authority")));
  assert!(!as_bool(get(evidence, "emit-current-authority")));
  assert_eq!(as_str(get(evidence, "discovery-readiness")), "not-proven");
  assert!(!as_bool(get(evidence, "discovery-owner-switch")));
}

#[test]
fn emitted_roles_split_lift_query_emit_into_candidate_intent_projection_and_needs() {
  let run = eval_file(&fixture_path()).unwrap();
  let roles = attrs_by_id(get(&run, "role-emission-verdicts"));
  assert_eq!(roles.len(), 11);
  for expected in [
    "role.source-object-candidate",
    "role.provenance-pressure",
    "role.query-intent",
    "role.query-need",
    "role.projection-event-specimen",
    "role.projection-surface-set",
    "role.semantic-owner-need",
    "role.audit-receipt-need",
    "role.held-result-boundary",
    "role.reverse-replay-requirement",
    "role.lift-query-emit-rewrite-need",
  ] {
    let role = roles
      .get(expected)
      .unwrap_or_else(|| panic!("missing role `{expected}`"));
    assert!(as_bool(get(role, "emitted")));
    assert!(!as_bool(get(role, "accepted")));
    assert!(!as_bool(get(role, "implementation-target")));
    assert!(!as_bool(get(role, "owner-switch")));
  }
  assert_eq!(
    as_str(get(roles.get("role.query-intent").unwrap(), "verdict")),
    "keep-as-intent-not-answer"
  );
  assert_eq!(
    as_str(get(
      roles.get("role.projection-event-specimen").unwrap(),
      "verdict"
    )),
    "keep-as-projection-specimen"
  );
  assert_eq!(
    as_str(get(
      roles.get("role.held-result-boundary").unwrap(),
      "verdict"
    )),
    "keep-as-held-boundary"
  );
}

#[test]
fn legacy_store_query_engine_and_event_log_roles_are_not_emitted() {
  let run = eval_file(&fixture_path()).unwrap();
  let roles = attrs_by_id(get(&run, "non-emitted-legacy-plan-roles"));
  assert_eq!(roles.len(), 7);
  for expected in [
    "role.contextualfact-store",
    "role.retrieval-time-inference",
    "role.needcursor-store",
    "role.expression-projection-owner",
    "role.audit-event-log",
    "role.doghouse-store",
    "role.global-ontology-runtime",
  ] {
    let role = roles
      .get(expected)
      .unwrap_or_else(|| panic!("missing non-emitted role `{expected}`"));
    assert!(!as_bool(get(role, "emitted")));
  }
  assert_eq!(
    as_str(get(
      roles.get("role.global-ontology-runtime").unwrap(),
      "verdict"
    )),
    "not-emitted-held"
  );
}

#[test]
fn triple_dependency_blocks_answer_and_projection_authority_without_owner_proofs() {
  let run = eval_file(&fixture_path()).unwrap();
  let dependency = get(&run, "triple-dependency");
  assert!(as_bool(get(dependency, "lift-feeds-query-and-emit")));
  assert_eq!(
    as_str(get(dependency, "query-without-lifted-substrate")),
    "IntentOnly"
  );
  assert_eq!(
    as_str(get(dependency, "emit-without-semantic-owner")),
    "Held"
  );
  assert!(!as_bool(get(dependency, "lifted-candidate-current-fact")));
  assert!(!as_bool(get(dependency, "query-result-current-answer")));
  assert!(!as_bool(get(dependency, "emit-current-proof")));
  assert_eq!(
    as_str(get(dependency, "triple-scope")),
    "legacy-lift-query-emit-triple-only"
  );
  assert!(!as_bool(get(dependency, "split-rewrite-allowed")));
}

#[test]
fn six_layer_role_fold_preserves_store_query_event_and_runtime_boundaries() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-role-fold");
  assert_eq!(
    as_str(get(fold, "mode")),
    "r3-role-emission-lift-query-emit-verdict"
  );
  for layer in [
    "surface", "ontology", "semantic", "gate", "runtime", "audit",
  ] {
    assert!(
      as_bool(get_path(fold, &[layer, "visible"])),
      "layer `{layer}` must stay visible"
    );
  }
  assert!(as_bool(get_path(
    fold,
    &["surface", "triple-required-for-r4"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["ontology", "global-ontology-runtime"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "lifted-candidate-demoted-to-sourceobject"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "query-envelope-demoted-to-intent"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "emit-projection-demoted-to-specimen"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "semantic-owner-need-emitted"]
  )));
  assert!(!as_bool(get_path(fold, &["gate", "owner-switch"])));
  assert!(!as_bool(get_path(fold, &["runtime", "executable-now"])));
  assert!(!as_bool(get_path(
    fold,
    &["runtime", "query-runtime-installed"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["runtime", "event-log-installed"]
  )));
  assert_eq!(
    as_str(get_path(fold, &["audit", "reverse-replay-status"])),
    "required-not-run"
  );
}

#[test]
fn r4_entry_opens_only_for_triple_rewrite_candidate() {
  let run = eval_file(&fixture_path()).unwrap();
  let boundary = get(&run, "r4-entry-boundary");
  assert!(as_bool(get(
    boundary,
    "r3-verdict-closed-for-this-surface-triple"
  )));
  assert!(as_bool(get(
    boundary,
    "r4-macro-native-rewrite-candidate-may-start"
  )));
  assert_eq!(
    as_str(get(boundary, "r4-scope")),
    "legacy-lift-query-emit-triple-only"
  );
  assert!(!as_bool(get(boundary, "broad-ontology-runtime-open")));
  assert!(!as_bool(get(boundary, "owner-switch-open")));
  assert!(!as_bool(get(boundary, "runtime-install-open")));
  assert!(!as_bool(get(boundary, "fact-store-open")));
  assert!(!as_bool(get(boundary, "query-engine-open")));
  assert!(!as_bool(get(boundary, "event-log-open")));

  let required = string_set(get(boundary, "required-next"));
  for expected in [
    "macro-native-lift-query-emit-rewrite-candidate",
    "provenance-reference-delta",
    "query-intent-held-proof",
    "projection-surface-delta",
    "semantic-owner-proof",
    "reverse-replay",
    "negative-held-proof",
  ] {
    assert!(
      required.contains(expected),
      "missing next requirement `{expected}`"
    );
  }
}

#[test]
fn runtime_observation_is_candidate_only_and_non_executable() {
  let run = eval_file(&fixture_path()).unwrap();
  let runtime = get(&run, "runtime-observation");
  assert_eq!(
    as_str(get(runtime, "observation-model")),
    "r3-lift-query-emit-role-emission-runtime-candidates"
  );
  assert!(as_bool(get(runtime, "can-appear-at-runtime")));
  assert!(as_bool(get(runtime, "candidate-only")));
  assert!(!as_bool(get(runtime, "canonical-runtime-installed")));
  assert!(!as_bool(get(runtime, "executable-now")));
  assert!(!as_bool(get(runtime, "owner-switch")));
  assert_eq!(as_list(get(runtime, "runtime-added-candidates")).len(), 4);
}

#[test]
fn discoveries_record_d263_through_d271() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 9);
  for expected in [
    "D263.lift-query-emit-r3-is-surface-triple-scoped",
    "D264.lifted-candidate-demotes-to-sourceobject-candidate",
    "D265.query-envelope-demotes-to-query-intent",
    "D266.emit-projection-demotes-to-projection-specimen",
    "D267.semantic-owner-and-audit-receipt-are-explicit-needs",
    "D268.contextualfact-store-query-engine-and-event-log-are-not-emitted",
    "D269.r4-entry-opens-for-lift-query-emit-triple-only",
    "D270.lift-query-emit-runtime-candidates-are-non-executable",
    "D271.llm-prose-cannot-answer-query-or-authorize-projection",
  ] {
    let discovery = discoveries
      .get(expected)
      .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
    assert!(as_bool(get(discovery, "scenario-only")));
  }
}

#[test]
fn affected_plans_remain_observation_handles_except_r4_frontier() {
  let run = eval_file(&fixture_path()).unwrap();
  let affected = get(&run, "affected-plans");
  assert_eq!(
    as_str(get_path(affected, &["ContextualFactStore", "pressure"])),
    "redesign-to-sourceobject-candidate"
  );
  assert_eq!(
    as_str(get_path(affected, &["retrievalTimeInference", "pressure"])),
    "demote-query-envelope-to-intent"
  );
  assert_eq!(
    as_str(get_path(affected, &["AuditEventLog", "pressure"])),
    "hold-until-audit-receipt-owner"
  );
  assert_eq!(
    as_str(get_path(affected, &["liftQueryEmitRewrite", "pressure"])),
    "may-start-triple-r4-rewrite"
  );
  for key in [
    "ContextualFactStore",
    "retrievalTimeInference",
    "NeedCursor",
    "ExpressionProjectionOwner",
    "AuditEventLog",
    "liftQueryEmitRewrite",
  ] {
    assert!(!as_bool(get_path(
      affected,
      &[key, "implementation-target"]
    )));
  }
}

#[test]
fn negative_held_evidence_blocks_fact_answer_projection_runtime_and_llm_shortcuts() {
  let run = eval_file(&fixture_path()).unwrap();
  let negative = get(&run, "negative-held-evidence");
  assert_eq!(as_str(get(negative, "status")), "present");
  let rejects = string_set(get(negative, "rejects"));
  for expected in [
    "lifted-candidate-as-accepted-fact",
    "query-envelope-as-answer",
    "emit-projection-as-audit-proof",
    "projection-record-as-semantic-owner",
    "contextualfact-store-from-r3",
    "needcursor-from-query-intent",
    "audit-event-log-from-emit",
    "query-runtime-install-from-r3",
    "llm-prose-query-answer",
  ] {
    assert!(rejects.contains(expected), "missing rejection `{expected}`");
  }
}

#[test]
fn blocked_shortcuts_cover_store_answer_projection_and_runtime_claims() {
  let run = eval_file(&fixture_path()).unwrap();
  let blocks = string_set(get(&run, "blocked-shortcuts"));
  for expected in [
    "lifted-candidate-as-Accepted-fact",
    "query-envelope-as-answer",
    "emit-projection-as-audit-proof",
    "projection-record-as-semantic-owner",
    "ContextualFactStore-from-lift",
    "NeedCursor-from-query-intent",
    "AuditEventLog-from-emit",
    "doghouse-store-as-ontology-owner",
    "install-query-runtime-from-r3",
    "treat-llm-prose-as-query-answer",
  ] {
    assert!(
      blocks.contains(expected),
      "missing shortcut block `{expected}`"
    );
  }
}

#[test]
fn top_level_state_keeps_replacement_unproven_without_runtime_or_owner_switch() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(as_str(get(&run, "replacement-readiness")), "not-proven");
  assert!(!as_bool(get(&run, "owner-switch")));
  assert!(!as_bool(get(&run, "runtime-install")));
  assert!(!as_bool(get(&run, "global-ontology-runtime")));
  assert!(!as_bool(get(&run, "implementation-command")));
}
