//! R7 compat/archive decision for the macro-native lift/query/emit triple.
//!
//! R6 switched semantic ownership for the dependent `ontologyLift` /
//! `ontologyQuery` / `ontologyEmit` triple. R7 decides what happens to the old
//! triple after that switch: retain it as compat/reference and query/projection
//! regression material, not current authority and not deleted/archived.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join(
    "../../fixtures/tesseract-macro-legacy-probe/compat_archive_lift_query_emit_surface_triple_receipt.px",
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
fn lift_query_emit_r7_marker_and_constitution_owner_are_pinned() {
  let run = eval_file(&fixture_path()).expect("lift/query/emit R7 compat fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-r7-compat-archive-lift-query-emit-surface-triple"
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
fn constitution_gate_keeps_lift_query_emit_r7_candidate_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "r7-compat-archive-lift-query-emit-surface-triple"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  assert_eq!(
    as_str(get(gate, "replacement-readiness")),
    "compat-retained-for-lift-query-emit-surface-triple"
  );

  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "delete-legacy-lift-query-emit-because-owner-switched",
    "archive-legacy-lift-query-emit-without-usage-scan",
    "drop-query-projection-regression-corpus-after-green-owner-switch",
    "drop-lift-provenance-regression-after-r6",
    "drop-query-intent-regression-after-r6",
    "drop-emit-projection-regression-after-r6",
    "treat-compat-shell-as-current-authority",
    "install-query-runtime-route-at-r7",
    "install-fact-store-at-r7",
    "install-audit-event-log-at-r7",
    "emit-projection-specimen-as-expression-projection-owner-at-r7",
    "globalize-lift-query-emit-compat-decision",
    "split-lift-query-emit-compat-without-proof",
    "restore-old-lift-query-emit-wrapper",
    "treat-llm-cleanup-prose-as-delete-proof",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
}

#[test]
fn surface_triple_is_lift_query_emit_only_and_uses_macro_owner() {
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
    as_str(get(triple, "semantic-owner")),
    "macro-native.lift-query-emit.surface-triple-owner"
  );
  assert_eq!(
    as_str(get(triple, "old-owner-role")),
    "reference-specimen-triple-and-query-projection-regression-corpus"
  );
  assert_eq!(
    as_str(get(triple, "scope")),
    "legacy-lift-query-emit-triple-only"
  );
  assert!(as_bool(get(triple, "triple-required")));
  assert!(!as_bool(get(triple, "query-only-compat")));
  assert!(!as_bool(get(triple, "emit-only-compat")));
  assert!(!as_bool(get(triple, "split-proof-present")));
  assert!(!as_bool(get(triple, "other-surfaces-included")));
}

#[test]
fn r6_input_imports_triple_owner_switch_state() {
  let run = eval_file(&fixture_path()).unwrap();
  let input = get(&run, "r6-input");
  assert_eq!(
    as_str(get(input, "owner-switch-receipt")),
    "r6.owner-switch.lift-query-emit-surface-triple"
  );
  assert_eq!(
    as_str(get(input, "replacement-readiness")),
    "owner-switched-for-lift-query-emit-surface-triple"
  );
  assert!(as_bool(get(input, "owner-switch")));
  assert_eq!(
    as_str(get(input, "semantic-owner")),
    "macro-native.lift-query-emit.surface-triple-owner"
  );
  assert!(!as_bool(get(input, "legacy-current-authority")));
  assert!(!as_bool(get(input, "runtime-install")));
  assert!(!as_bool(get(input, "query-runtime-install")));
  assert!(!as_bool(get(input, "fact-store-install")));
  assert!(!as_bool(get(input, "audit-event-log-install")));
  assert!(!as_bool(get(input, "expression-projection-owner")));
  assert!(!as_bool(get(input, "global-ontology-runtime")));
  assert!(as_bool(get(input, "r7-required")));
  assert!(as_bool(get(input, "negative-held-survives")));
  assert!(as_bool(get(input, "audit-refs-preserved")));
  assert_eq!(
    as_str(get(input, "query-projection-regression-corpus")),
    "query-projection-regression-corpus.lift-query-emit"
  );
}

#[test]
fn compat_decision_retains_legacy_triple_without_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  let decision = get(&run, "compat-decision");
  assert_eq!(
    as_str(get(decision, "id")),
    "r7.compat-retain.lift-query-emit-surface-triple"
  );
  assert_eq!(as_str(get(decision, "phase")), "R7");
  assert_eq!(
    as_str(get(decision, "decision")),
    "retain-compat-reference-triple"
  );
  assert_eq!(
    as_str(get(decision, "compat-status")),
    "compat-retained-for-lift-query-emit-surface-triple"
  );
  assert!(!as_bool(get(decision, "current-semantic-authority")));
  assert!(!as_bool(get(decision, "legacy-lift-is-current-fact-store")));
  assert!(!as_bool(get(decision, "legacy-query-is-answer-engine")));
  assert!(!as_bool(get(decision, "legacy-emit-is-expression-owner")));
  assert!(!as_bool(get(decision, "legacy-audit-is-event-log")));
  assert!(!as_bool(get(decision, "old-lift-query-emit-wrapper")));
  assert_eq!(as_str(get(decision, "compat-shell")), "candidate-only");
  assert_eq!(
    as_str(get(decision, "docs-role")),
    "historical-reference-triple"
  );
}

#[test]
fn compat_decision_blocks_runtime_global_split_delete_and_archive() {
  let run = eval_file(&fixture_path()).unwrap();
  let decision = get(&run, "compat-decision");
  assert!(!as_bool(get(decision, "compat-route-installed")));
  assert!(as_bool(get(
    decision,
    "query-projection-regression-corpus-retained"
  )));
  assert!(as_bool(get(decision, "reverse-replay-reference-retained")));
  assert!(as_bool(get(
    decision,
    "lift-provenance-regression-retained"
  )));
  assert!(as_bool(get(decision, "query-intent-regression-retained")));
  assert!(as_bool(get(
    decision,
    "emit-projection-regression-retained"
  )));
  assert!(as_bool(get(
    decision,
    "semantic-owner-need-regression-retained"
  )));
  assert!(as_bool(get(
    decision,
    "audit-receipt-need-regression-retained"
  )));
  assert!(as_bool(get(decision, "supersede-chain-retained")));
  assert!(as_bool(get(decision, "rollback-evidence-retained")));
  assert!(!as_bool(get(decision, "delete-legacy-surfaces")));
  assert!(!as_bool(get(decision, "archive-legacy-surfaces")));
  assert!(!as_bool(get(decision, "runtime-install")));
  assert!(!as_bool(get(decision, "query-runtime-install")));
  assert!(!as_bool(get(decision, "fact-store-install")));
  assert!(!as_bool(get(decision, "audit-event-log-install")));
  assert!(!as_bool(get(decision, "expression-projection-owner")));
  assert!(!as_bool(get(decision, "global-ontology-runtime")));
  assert!(!as_bool(get(decision, "split-lift-query-emit-owner")));
  assert!(!as_bool(get(decision, "other-surfaces-included")));
  assert!(!as_bool(get(decision, "implementation-command")));
}

#[test]
fn retention_policy_preserves_lift_query_emit_regression_evidence() {
  let run = eval_file(&fixture_path()).unwrap();
  let policy = get(&run, "retention-policy");
  assert_eq!(
    as_str(get(policy, "id")),
    "retention.r7.lift-query-emit-surface-triple"
  );
  let retained = string_set(get(policy, "retained-for"));
  for expected in [
    "query-projection-regression-corpus",
    "reverse-replay-reference",
    "compat-shell-candidate-input",
    "supersede-chain-audit",
    "rollback-evidence",
    "lift-provenance-regression-case",
    "query-intent-regression-case",
    "emit-projection-regression-case",
    "semantic-owner-need-regression-case",
    "audit-receipt-need-regression-case",
    "runtime-store-held-regression-case",
    "historical-doc-reference",
  ] {
    assert!(
      retained.contains(expected),
      "missing retained role `{expected}`"
    );
  }
  assert!(!as_bool(get(policy, "can-be-called-as-current-authority")));
  assert!(as_bool(get(policy, "can-be-used-as-test-oracle")));
  assert!(as_bool(get(policy, "can-be-used-for-reverse-replay")));
  assert!(as_bool(get(policy, "can-be-used-for-rollback-analysis")));
  assert!(as_bool(get(
    policy,
    "can-be-used-for-lift-provenance-regression"
  )));
  assert!(as_bool(get(
    policy,
    "can-be-used-for-query-intent-regression"
  )));
  assert!(as_bool(get(
    policy,
    "can-be-used-for-emit-projection-regression"
  )));
  assert!(as_bool(get(
    policy,
    "can-be-used-for-semantic-owner-regression"
  )));
  assert!(as_bool(get(policy, "can-be-used-for-audit-regression")));
  assert!(!as_bool(get(policy, "delete-now")));
  assert!(!as_bool(get(policy, "archive-now")));
}

#[test]
fn archive_delete_gate_holds_without_triple_specific_proof() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "archive-delete-gate");
  assert_eq!(
    as_str(get(gate, "id")),
    "gate.r7.archive-delete.lift-query-emit-surface-triple"
  );
  assert_eq!(as_str(get(gate, "verdict")), "delete-and-archive-held");
  for key in [
    "delete-proof-present",
    "archive-proof-present",
    "usage-scan-complete",
    "external-caller-scan-complete",
    "replay-corpus-replacement-present",
    "query-projection-corpus-replacement-present",
    "query-runtime-replacement-present",
    "fact-store-replacement-present",
    "audit-event-log-replacement-present",
    "expression-projection-owner-replacement-present",
    "split-proof-present",
    "rollback-plan-present",
    "human-consequence-authorization-present",
  ] {
    assert!(!as_bool(get(gate, key)), "`{key}` must stay false");
  }

  let required = string_set(get(gate, "required-before-delete-or-archive"));
  for expected in [
    "compat-usage-scan",
    "external-caller-scan",
    "replacement-replay-corpus",
    "replacement-query-projection-regression-corpus",
    "query-runtime-owner-receipt-if-install-needed",
    "fact-store-owner-receipt-if-install-needed",
    "audit-event-log-owner-receipt-if-install-needed",
    "expression-projection-owner-receipt-if-install-needed",
    "split-proof-if-triple-is-separated",
    "rollback-plan",
    "supersede-chain-audit",
    "human-consequence-authorization-if-consequence-bearing",
  ] {
    assert!(
      required.contains(expected),
      "missing archive/delete proof `{expected}`"
    );
  }
}

#[test]
fn compat_routes_are_candidates_not_current_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  let routes = attrs_by_id(get(&run, "compat-route-candidates"));
  assert_eq!(routes.len(), 5);
  for expected in [
    "compat.route.legacy-lift-to-macro-owner",
    "compat.route.legacy-query-to-macro-owner",
    "compat.route.legacy-emit-to-macro-owner",
    "compat.route.query-projection-regression-oracle",
    "compat.route.rollback-analysis",
  ] {
    let route = routes
      .get(expected)
      .unwrap_or_else(|| panic!("missing route `{expected}`"));
    assert!(!as_bool(get(route, "installed")));
    assert!(!as_bool(get(route, "current-authority")));
  }
  assert_eq!(
    as_str(get(
      routes
        .get("compat.route.legacy-query-to-macro-owner")
        .unwrap(),
      "held-if"
    )),
    "query-runtime-owner-receipt-missing"
  );
  assert_eq!(
    as_str(get(
      routes
        .get("compat.route.query-projection-regression-oracle")
        .unwrap(),
      "held-if"
    )),
    "used-as-current-proof"
  );
}

#[test]
fn r7_trials_hold_shortcuts_and_accept_triple_compat_retention() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "r7-trials"));
  assert_eq!(trials.len(), 13);
  for expected in [
    "trial.A.r6-owner-switch-missing",
    "trial.B.compat-policy-missing",
    "trial.C.legacy-lift-current-store",
    "trial.D.legacy-query-answer-engine",
    "trial.E.legacy-emit-expression-owner",
    "trial.F.delete-without-proof",
    "trial.G.archive-without-usage-scan",
    "trial.H.runtime-install-requested",
    "trial.I.global-ontology-compat",
    "trial.J.split-compat-without-proof",
    "trial.K.old-wrapper-restored",
    "trial.L.llm-cleanup-delete",
  ] {
    let trial = trials
      .get(expected)
      .unwrap_or_else(|| panic!("missing trial `{expected}`"));
    assert_eq!(as_str(get(trial, "outcome")), "Held");
    assert!(!as_bool(get(trial, "delete-legacy-surfaces")));
    assert!(!as_bool(get(trial, "archive-legacy-surfaces")));
  }

  let complete = trials.get("trial.M.complete-compat-retain").unwrap();
  assert_eq!(
    as_str(get(complete, "outcome")),
    "compat-retained-for-lift-query-emit-surface-triple"
  );
  assert_eq!(
    as_str(get(complete, "compat-status")),
    "compat-retained-for-lift-query-emit-surface-triple"
  );
  assert!(!as_bool(get(complete, "delete-legacy-surfaces")));
  assert!(!as_bool(get(complete, "archive-legacy-surfaces")));
}

#[test]
fn six_layer_compat_fold_preserves_triple_and_runtime_boundaries() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-compat-fold");
  assert_eq!(
    as_str(get(fold, "mode")),
    "r7-compat-archive-lift-query-emit-surface-triple"
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
    as_str(get_path(fold, &["surface", "compat-status"])),
    "compat-retained-for-lift-query-emit-surface-triple"
  );
  assert!(as_bool(get_path(fold, &["surface", "triple-required"])));
  assert!(!as_bool(get_path(fold, &["surface", "query-only-compat"])));
  assert!(!as_bool(get_path(fold, &["surface", "emit-only-compat"])));
  assert_eq!(
    as_str(get_path(fold, &["ontology", "old-triple-role"])),
    "compat-reference-triple"
  );
  assert!(!as_bool(get_path(
    fold,
    &["ontology", "query-answer-engine-authority"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["ontology", "fact-store-authority"]
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
    &["ontology", "split-proof-present"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["semantic", "current-semantic-authority"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["semantic", "legacy-lift-is-current-fact-store"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["semantic", "legacy-query-is-answer-engine"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["semantic", "legacy-emit-is-expression-owner"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["semantic", "legacy-audit-is-event-log"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "compat-route-is-candidate"]
  )));
  assert_eq!(
    as_str(get_path(fold, &["gate", "archive-delete-verdict"])),
    "delete-and-archive-held"
  );
  assert!(!as_bool(get_path(fold, &["runtime", "executable-now"])));
  assert!(!as_bool(get_path(fold, &["runtime", "installed"])));
  assert!(!as_bool(get_path(
    fold,
    &["runtime", "query-runtime-installed"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["runtime", "fact-store-installed"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["runtime", "audit-event-log-installed"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["runtime", "expression-projection-owner-installed"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["runtime", "compat-route-installed"]
  )));
  assert!(as_bool(get_path(fold, &["audit", "audit-refs-preserved"])));
  assert!(as_bool(get_path(
    fold,
    &["audit", "negative-held-survives"]
  )));
  assert!(as_bool(get_path(fold, &["audit", "retained-for-replay"])));
}

#[test]
fn runtime_observation_is_compat_retained_but_not_installed() {
  let run = eval_file(&fixture_path()).unwrap();
  let runtime = get(&run, "runtime-observation");
  assert_eq!(
    as_str(get(runtime, "observation-model")),
    "r7-compat-retained-lift-query-emit-surface-triple-non-installed-runtime"
  );
  assert!(as_bool(get(runtime, "can-appear-at-runtime")));
  assert!(as_bool(get(runtime, "owner-switch")));
  assert!(as_bool(get(runtime, "compat-retained")));
  assert!(!as_bool(get(runtime, "archive-legacy-surfaces")));
  assert!(!as_bool(get(runtime, "delete-legacy-surfaces")));
  assert!(!as_bool(get(runtime, "canonical-runtime-installed")));
  assert!(!as_bool(get(runtime, "executable-now")));
  assert!(!as_bool(get(runtime, "query-runtime-installed")));
  assert!(!as_bool(get(runtime, "fact-store-installed")));
  assert!(!as_bool(get(runtime, "audit-event-log-installed")));
  assert!(!as_bool(get(
    runtime,
    "expression-projection-owner-installed"
  )));
  assert!(!as_bool(get(runtime, "global-ontology-runtime")));
  assert_eq!(as_list(get(runtime, "runtime-added-candidates")).len(), 4);
}

#[test]
fn discoveries_record_d327_through_d335() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 9);
  for expected in [
    "D327.lift-query-emit-r7-retains-compat-reference-triple",
    "D328.legacy-lift-query-emit-is-regression-corpus-not-current-authority",
    "D329.lift-query-emit-compat-route-is-candidate-not-runtime-install",
    "D330.archive-delete-requires-triple-specific-proof",
    "D331.lift-query-emit-regression-evidence-survives-r7",
    "D332.r7-preserves-triple-dependency-until-split-proof",
    "D333.r7-lift-query-emit-does-not-create-runtime-owners",
    "D334.cleanup-prose-cannot-delete-lift-query-emit-triple",
    "D335.docs-can-be-historical-while-lift-query-emit-code-remains-retained",
  ] {
    let discovery = discoveries
      .get(expected)
      .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
    assert!(as_bool(get(discovery, "scenario-only")));
  }
}

#[test]
fn affected_plans_keep_runtime_archive_split_and_other_surfaces_unimplemented() {
  let run = eval_file(&fixture_path()).unwrap();
  let affected = get(&run, "affected-plans");
  assert_eq!(
    as_str(get_path(affected, &["legacyLiftQueryEmit", "pressure"])),
    "retain-after-owner-switch"
  );
  assert_eq!(
    as_str(get_path(affected, &["queryRuntime", "pressure"])),
    "needs-query-runtime-owner-receipt-before-install"
  );
  assert_eq!(
    as_str(get_path(affected, &["splitLiftQueryEmit", "pressure"])),
    "held-until-split-proof"
  );
  for key in [
    "legacyLiftQueryEmit",
    "macroLiftQueryEmit",
    "queryRuntime",
    "ContextualFactStore",
    "ExpressionProjectionOwner",
    "AuditEventLog",
    "archiveDelete",
    "splitLiftQueryEmit",
    "otherOntologySurfaces",
  ] {
    assert!(!as_bool(get_path(
      affected,
      &[key, "implementation-target"]
    )));
  }
}

#[test]
fn negative_held_evidence_survives_r7_triple_compat() {
  let run = eval_file(&fixture_path()).unwrap();
  let negative = get(&run, "negative-held-evidence");
  assert_eq!(as_str(get(negative, "status")), "present");
  assert!(as_bool(get(negative, "survives-r7")));
  let rejects = string_set(get(negative, "rejects"));
  for expected in [
    "r7-before-r6-owner-switch",
    "r7-without-compat-policy",
    "legacy-lift-as-current-fact-store-after-r7",
    "legacy-query-as-answer-engine-after-r7",
    "legacy-emit-as-expression-owner-after-r7",
    "legacy-audit-as-event-log-after-r7",
    "delete-without-archive-delete-proof",
    "archive-without-usage-scan",
    "query-runtime-install-from-r7-compat",
    "fact-store-install-from-r7-compat",
    "audit-event-log-install-from-r7-compat",
    "expression-projection-owner-from-r7-compat",
    "global-ontology-runtime-from-surface-triple",
    "split-compat-without-proof",
    "old-wrapper-restored-from-r7",
    "llm-cleanup-prose-as-delete-proof",
  ] {
    assert!(rejects.contains(expected), "missing rejection `{expected}`");
  }
}

#[test]
fn blocked_shortcuts_cover_delete_archive_runtime_split_and_prose() {
  let run = eval_file(&fixture_path()).unwrap();
  let blocks = string_set(get(&run, "blocked-shortcuts"));
  for expected in [
    "delete-legacy-lift-query-emit-because-owner-switched",
    "archive-legacy-lift-query-emit-without-usage-scan",
    "drop-query-projection-regression-corpus-after-green-owner-switch",
    "drop-reverse-replay-reference-after-r6",
    "drop-lift-provenance-regression-after-r6",
    "drop-query-intent-regression-after-r6",
    "drop-emit-projection-regression-after-r6",
    "drop-semantic-owner-need-regression-after-r6",
    "drop-audit-receipt-need-regression-after-r6",
    "treat-compat-shell-as-current-authority",
    "install-query-runtime-route-at-r7",
    "install-fact-store-at-r7",
    "install-audit-event-log-at-r7",
    "emit-projection-specimen-as-expression-projection-owner-at-r7",
    "globalize-lift-query-emit-compat-decision",
    "split-lift-query-emit-compat-without-proof",
    "restore-old-lift-query-emit-wrapper",
    "treat-llm-cleanup-prose-as-delete-proof",
  ] {
    assert!(
      blocks.contains(expected),
      "missing shortcut block `{expected}`"
    );
  }
}

#[test]
fn top_level_state_records_compat_retention_without_install_or_deletion() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "replacement-readiness")),
    "owner-switched-for-lift-query-emit-surface-triple"
  );
  assert!(as_bool(get(&run, "owner-switch")));
  assert_eq!(
    as_str(get(&run, "compat-status")),
    "compat-retained-for-lift-query-emit-surface-triple"
  );
  assert!(!as_bool(get(&run, "archive-legacy-surfaces")));
  assert!(!as_bool(get(&run, "delete-legacy-surfaces")));
  assert!(!as_bool(get(&run, "runtime-install")));
  assert!(!as_bool(get(&run, "query-runtime-install")));
  assert!(!as_bool(get(&run, "fact-store-install")));
  assert!(!as_bool(get(&run, "audit-event-log-install")));
  assert!(!as_bool(get(&run, "expression-projection-owner")));
  assert!(!as_bool(get(&run, "global-ontology-runtime")));
  assert!(!as_bool(get(&run, "implementation-command")));
}
