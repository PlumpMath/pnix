//! Surface-triple replacement readiness for lift/query/emit.
//!
//! R5 verified triple reverse replay for the R4 lift/query/emit candidate. This
//! test pins the next boundary: readiness may aggregate D8-D10 and R3/R4/R5
//! evidence and open R6 owner-switch review, but it still cannot switch owners,
//! install query runtime, create a fact store/event log, globalize ontology
//! runtime, or turn query/projection evidence into authority.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join(
    "../../fixtures/tesseract-macro-legacy-probe/replacement_readiness_lift_query_emit_candidate_receipt.px",
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
fn lift_query_emit_readiness_marker_and_constitution_owner_are_pinned() {
  let run = eval_file(&fixture_path()).expect("lift/query/emit readiness fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-surface-triple-replacement-readiness-lift-query-emit-candidate"
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
    "surface-triple-replacement-readiness-lift-query-emit-candidate"
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
    "treat-readiness-as-query-runtime-install",
    "treat-readiness-as-audit-event-log-install",
    "claim-global-ontology-readiness",
    "skip-r5-reverse-replay",
    "replay-query-or-emit-without-lift",
    "ignore-uncovered-delta",
    "emit-lift-as-ContextualFactStore",
    "emit-query-intent-as-answer-engine",
    "emit-projection-specimen-as-semantic-owner",
    "emit-audit-need-as-event-log",
    "treat-llm-prose-as-readiness",
  ] {
    assert!(blocks.contains(expected), "missing gate block `{expected}`");
  }
}

#[test]
fn readiness_is_surface_triple_scoped_to_lift_query_emit() {
  let run = eval_file(&fixture_path()).unwrap();
  let triple = get(&run, "surface-triple");
  assert_eq!(
    as_str(get(triple, "id")),
    "surface-triple.legacy-ontology.lift-query-emit"
  );
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
  assert!(as_bool(get(triple, "triple-required")));
  assert!(!as_bool(get(triple, "global-ontology-runtime")));
}

#[test]
fn evidence_bundle_imports_r5_triple_replay_state() {
  let run = eval_file(&fixture_path()).unwrap();
  let evidence = get(&run, "evidence-bundle");
  assert_eq!(
    as_str(get(evidence, "lift-specimen")),
    "legacy-replay-specimen.lift.candidate-fact"
  );
  assert_eq!(
    as_str(get(evidence, "query-specimen")),
    "legacy-replay-specimen.query.envelope"
  );
  assert_eq!(
    as_str(get(evidence, "emit-specimen")),
    "legacy-replay-specimen.emit.projection"
  );
  assert_eq!(
    as_str(get(evidence, "r4-candidate")),
    "r4.macro-native-lift-query-emit.rewrite-candidate"
  );
  assert_eq!(
    as_str(get(evidence, "r5-verdict")),
    "reverse-replay-verified"
  );
  assert!(as_bool(get(evidence, "triple-replay")));
  assert!(as_bool(get(evidence, "all-deltas-covered")));
  assert!(as_bool(get(evidence, "lift-provenance-covered")));
  assert!(as_bool(get(evidence, "query-intent-need-covered")));
  assert!(as_bool(get(evidence, "emit-projection-surface-covered")));
  assert!(as_bool(get(evidence, "semantic-owner-need-covered")));
  assert!(as_bool(get(evidence, "audit-receipt-need-covered")));
  assert!(!as_bool(get(evidence, "unexplained-mismatch")));
  assert!(as_bool(get(evidence, "audit-refs-preserved")));
  assert!(as_bool(get(evidence, "negative-held-proof-present")));
  assert!(!as_bool(get(evidence, "owner-switch-before-readiness")));
}

#[test]
fn all_readiness_criteria_are_satisfied_without_splitting_triple() {
  let run = eval_file(&fixture_path()).unwrap();
  let criteria = attrs_by_id(get(&run, "readiness-criteria"));
  assert_eq!(criteria.len(), 16);
  for expected in [
    "criteria.six-layers-visible",
    "criteria.triple-replay-present",
    "criteria.lift-provenance-covered",
    "criteria.query-intent-need-covered",
    "criteria.emit-projection-surface-covered",
    "criteria.semantic-owner-need-covered",
    "criteria.audit-receipt-need-covered",
    "criteria.no-unexplained-mismatch",
    "criteria.legacy-authority-blocked",
    "criteria.negative-path-present",
    "criteria.audit-refs-preserved",
    "criteria.query-projection-regression-corpus-bound",
    "criteria.semantic-owner-readiness-proof-present",
    "criteria.audit-receipt-readiness-proof-present",
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
fn query_projection_regression_corpus_is_bound_but_not_installed() {
  let run = eval_file(&fixture_path()).unwrap();
  let corpus = get(&run, "query-projection-regression-corpus");
  assert_eq!(
    as_str(get(corpus, "id")),
    "query-projection-regression-corpus.lift-query-emit"
  );
  assert_eq!(
    as_str(get(corpus, "corpus-kind")),
    "surface-triple-replay-regression-corpus"
  );
  assert_eq!(as_list(get(corpus, "covered-deltas")).len(), 7);
  let held = string_set(get(corpus, "held-regression-cases"));
  for expected in [
    "lift-provenance-mismatch",
    "query-answer-claim",
    "projection-owner-claim",
    "audit-ref-lost",
    "query-or-emit-only-replay",
    "runtime-install-requested",
    "replacement-readiness-requested",
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
fn semantic_owner_and_audit_readiness_are_not_install_or_owner_switch() {
  let run = eval_file(&fixture_path()).unwrap();
  let semantic = get(&run, "semantic-owner-readiness");
  assert_eq!(
    as_str(get(semantic, "verdict")),
    "semantic-owner-ready-for-r6-review"
  );
  assert!(as_bool(get(semantic, "covered-by-r5")));
  assert!(!as_bool(get(semantic, "owner-installed")));
  assert!(!as_bool(get(semantic, "semantic-owner-switch")));

  let audit = get(&run, "audit-receipt-readiness");
  assert_eq!(
    as_str(get(audit, "verdict")),
    "audit-receipt-ready-for-r6-review"
  );
  assert!(as_bool(get(audit, "covered-by-r5")));
  assert!(!as_bool(get(audit, "event-log-installed")));
  assert!(!as_bool(get(audit, "audit-owner-switch")));
}

#[test]
fn runtime_route_proof_is_non_executable_and_not_query_runtime() {
  let run = eval_file(&fixture_path()).unwrap();
  let proof = get(&run, "runtime-route-proof");
  assert_eq!(
    as_str(get(proof, "proof-kind")),
    "non-executable-lift-query-emit-route-proof"
  );
  assert_eq!(
    as_str(get(proof, "verdict")),
    "runtime-route-proof-candidate-verified"
  );
  assert!(!as_bool(get(proof, "installed")));
  assert!(!as_bool(get(proof, "executable-now")));
  assert!(!as_bool(get(proof, "query-runtime-installed")));
  assert!(!as_bool(get(proof, "audit-event-log-installed")));
  assert!(!as_bool(get(proof, "fact-store-installed")));
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
    "legacy-lift-query-emit-triple-only"
  );
  assert!(!as_bool(get(verdict, "owner-switch")));
  assert!(!as_bool(get(verdict, "runtime-install")));
  assert!(!as_bool(get(verdict, "query-runtime-install")));
  assert!(!as_bool(get(verdict, "audit-event-log-install")));
  assert!(!as_bool(get(verdict, "fact-store-install")));
  assert!(!as_bool(get(verdict, "expression-projection-owner")));
  assert!(!as_bool(get(verdict, "global-ontology-runtime")));
  assert!(!as_bool(get(verdict, "delete-legacy-surfaces")));
  assert!(!as_bool(get(verdict, "archive-legacy-surfaces")));

  let required = string_set(get(verdict, "next-required"));
  for expected in [
    "r6-lift-query-emit-owner-switch-receipt",
    "human-consequence-authorization-if-consequence-bearing",
    "query-runtime-owner-receipt-after-owner-switch",
    "audit-event-log-owner-receipt-after-owner-switch",
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
  assert_eq!(trials.len(), 11);
  for expected in [
    "trial.A.reverse-replay-not-verified",
    "trial.B.query-or-emit-only-readiness",
    "trial.C.uncovered-delta",
    "trial.D.semantic-owner-proof-missing",
    "trial.E.audit-receipt-proof-missing",
    "trial.F.negative-held-missing",
    "trial.G.runtime-route-proof-missing",
    "trial.H.regression-corpus-missing",
    "trial.I.owner-switch-requested",
    "trial.J.runtime-install-requested",
  ] {
    let trial = trials
      .get(expected)
      .unwrap_or_else(|| panic!("missing trial `{expected}`"));
    assert_eq!(as_str(get(trial, "outcome")), "Held");
    assert!(!as_bool(get(trial, "owner-switch")));
  }
  let complete = trials.get("trial.K.complete-readiness").unwrap();
  assert_eq!(
    as_str(get(complete, "outcome")),
    "ready-for-r6-owner-switch-receipt"
  );
  assert!(!as_bool(get(complete, "owner-switch")));
}

#[test]
fn six_layer_readiness_fold_preserves_triple_authority_and_runtime_boundaries() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-readiness-fold");
  assert_eq!(
    as_str(get(fold, "mode")),
    "surface-triple-replacement-readiness-lift-query-emit-candidate"
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
    as_str(get_path(fold, &["ontology", "readiness-scope"])),
    "surface-triple-scoped"
  );
  assert!(!as_bool(get_path(
    fold,
    &["ontology", "fact-store-authority"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["ontology", "query-answer-engine-authority"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["ontology", "expression-projection-owner"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["ontology", "audit-event-log-authority"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["ontology", "global-ontology-runtime"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["semantic", "readiness-is-owner-switch"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["semantic", "query-intent-is-answer"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["semantic", "projection-specimen-is-semantic-owner"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["runtime", "query-runtime-installed"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["runtime", "audit-event-log-installed"]
  )));
  assert!(as_bool(get_path(fold, &["audit", "audit-refs-preserved"])));
}

#[test]
fn runtime_observation_is_candidate_only_and_not_installed() {
  let run = eval_file(&fixture_path()).unwrap();
  let runtime = get(&run, "runtime-observation");
  assert_eq!(
    as_str(get(runtime, "observation-model")),
    "liftqueryemit-readiness-runtime-candidates"
  );
  assert!(as_bool(get(runtime, "can-appear-at-runtime")));
  assert!(as_bool(get(runtime, "candidate-only")));
  assert!(!as_bool(get(runtime, "canonical-runtime-installed")));
  assert!(!as_bool(get(runtime, "executable-now")));
  assert!(!as_bool(get(runtime, "owner-switch")));
  assert!(!as_bool(get(runtime, "query-runtime-installed")));
  assert!(!as_bool(get(runtime, "audit-event-log-installed")));
  assert!(!as_bool(get(runtime, "fact-store-installed")));
  assert_eq!(as_list(get(runtime, "runtime-added-candidates")).len(), 4);
}

#[test]
fn discoveries_record_d309_through_d317() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 9);
  for expected in [
    "D309.lift-query-emit-readiness-is-surface-triple-scoped",
    "D310.lift-query-emit-readiness-aggregates-d8-through-r5",
    "D311.lift-query-emit-readiness-keeps-triple-dependency-load-bearing",
    "D312.semantic-owner-and-audit-need-readiness-precedes-owner-switch",
    "D313.query-projection-regression-corpus-binding-precedes-owner-switch",
    "D314.lift-query-emit-runtime-route-proof-is-non-executable",
    "D315.lift-query-emit-readiness-opens-r6-without-switching",
    "D316.lift-query-emit-readiness-preserves-held-and-rewrite-debt",
    "D317.lift-query-emit-readiness-is-receipt-driven-not-prose",
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
    "held-lift-readiness-is-evidence-not-store"
  );
  assert_eq!(
    as_str(get_path(affected, &["retrievalTimeInference", "pressure"])),
    "held-query-readiness-is-intent-not-answer-engine"
  );
  assert_eq!(
    as_str(get_path(
      affected,
      &["ExpressionProjectionOwner", "pressure"]
    )),
    "held-projection-readiness-is-surface-not-owner"
  );
  assert_eq!(
    as_str(get_path(affected, &["AuditEventLog", "pressure"])),
    "held-audit-readiness-is-receipt-not-event-log"
  );
  assert_eq!(
    as_str(get_path(affected, &["liftQueryEmitRewrite", "pressure"])),
    "ready-for-r6-owner-switch-receipt"
  );
  assert_eq!(
    as_str(get_path(affected, &["ownerSwitch", "pressure"])),
    "may-start-r6-but-not-claimed-here"
  );
  for key in [
    "ContextualFactStore",
    "retrievalTimeInference",
    "ExpressionProjectionOwner",
    "AuditEventLog",
    "queryRuntime",
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
fn negative_held_evidence_blocks_readiness_shortcuts() {
  let run = eval_file(&fixture_path()).unwrap();
  let negative = get(&run, "negative-held-evidence");
  assert_eq!(as_str(get(negative, "status")), "present");
  let rejects = string_set(get(negative, "rejects"));
  for expected in [
    "readiness-without-r5-replay",
    "query-or-emit-only-readiness",
    "readiness-with-uncovered-delta",
    "readiness-without-audit-ref",
    "readiness-without-negative-held-proof",
    "readiness-without-regression-corpus",
    "readiness-without-semantic-owner-proof",
    "readiness-without-audit-receipt-proof",
    "owner-switch-inside-readiness",
    "query-runtime-install-inside-readiness",
    "audit-event-log-install-inside-readiness",
    "global-ontology-readiness-from-surface-triple",
    "lift-as-contextualfact-store-from-readiness",
    "query-intent-as-answer-from-readiness",
    "projection-specimen-as-semantic-owner-from-readiness",
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
    "treat-readiness-as-query-runtime-install",
    "treat-readiness-as-audit-event-log-install",
    "claim-global-ontology-readiness",
    "skip-r5-reverse-replay",
    "replay-query-or-emit-without-lift",
    "emit-lift-as-ContextualFactStore",
    "emit-query-intent-as-answer-engine",
    "emit-projection-specimen-as-semantic-owner",
    "emit-audit-need-as-event-log",
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
  assert!(!as_bool(get(&run, "query-runtime-install")));
  assert!(!as_bool(get(&run, "audit-event-log-install")));
  assert!(!as_bool(get(&run, "fact-store-install")));
  assert!(!as_bool(get(&run, "global-ontology-runtime")));
  assert!(!as_bool(get(&run, "implementation-command")));
}
