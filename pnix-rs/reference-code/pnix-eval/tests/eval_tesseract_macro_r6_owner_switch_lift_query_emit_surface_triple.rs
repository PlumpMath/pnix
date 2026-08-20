//! R6 owner switch for the macro-native lift/query/emit surface triple.
//!
//! The surface-triple readiness receipt opened R6 review for the dependent
//! `ontologyLift` / `ontologyQuery` / `ontologyEmit` triple. This test pins the
//! next boundary: owner switch is now true for that triple only, while query
//! runtime, fact store, audit event log, expression projection owner, global
//! runtime, old wrappers, delete/archive, and LLM-prose authority remain blocked.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join(
    "../../fixtures/tesseract-macro-legacy-probe/owner_switch_lift_query_emit_surface_triple_receipt.px",
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
fn lift_query_emit_owner_switch_marker_and_constitution_owner_are_pinned() {
  let run = eval_file(&fixture_path()).expect("lift/query/emit owner-switch fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-r6-owner-switch-lift-query-emit-surface-triple"
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
fn constitution_gate_keeps_r6_triple_owner_switch_candidate_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "r6-owner-switch-lift-query-emit-surface-triple"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  assert_eq!(
    as_str(get(gate, "replacement-readiness")),
    "owner-switched-for-lift-query-emit-surface-triple"
  );

  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "switch-owner-before-readiness",
    "split-query-or-emit-away-from-lift",
    "switch-owner-without-human-consequence-authorization",
    "install-query-runtime-from-owner-switch",
    "install-fact-store-from-owner-switch",
    "install-audit-event-log-from-owner-switch",
    "emit-projection-specimen-as-expression-projection-owner",
    "globalize-lift-query-emit-owner-switch",
    "restore-old-lift-query-emit-wrapper",
    "delete-or-archive-legacy-lift-query-emit-at-r6",
    "treat-llm-prose-as-owner-switch",
  ] {
    assert!(blocks.contains(expected), "missing gate block `{expected}`");
  }
}

#[test]
fn surface_triple_owner_switch_is_scoped_and_triple_required() {
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
    as_str(get(triple, "previous-owner")),
    "legacy-ontology.lift-query-emit.reference-specimen-triple"
  );
  assert_eq!(
    as_str(get(triple, "new-owner")),
    "macro-native.lift-query-emit.surface-triple-owner"
  );
  assert_eq!(
    as_str(get(triple, "scope")),
    "legacy-lift-query-emit-triple-only"
  );
  assert!(as_bool(get(triple, "triple-required")));
  assert!(!as_bool(get(triple, "query-only-owner-switch")));
  assert!(!as_bool(get(triple, "emit-only-owner-switch")));
  assert!(!as_bool(get(triple, "global-ontology-runtime")));
}

#[test]
fn readiness_input_imports_triple_ready_state_without_prior_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  let readiness = get(&run, "readiness-input");
  assert_eq!(
    as_str(get(readiness, "readiness")),
    "ready-for-r6-owner-switch-receipt"
  );
  assert_eq!(
    as_str(get(readiness, "surface-triple")),
    "surface-triple.legacy-ontology.lift-query-emit"
  );
  assert_eq!(
    as_str(get(readiness, "candidate")),
    "r4.macro-native-lift-query-emit.rewrite-candidate"
  );
  assert!(!as_bool(get(readiness, "owner-switch-before-r6")));
  assert!(!as_bool(get(readiness, "runtime-install-before-r6")));
  assert!(!as_bool(get(readiness, "query-runtime-install-before-r6")));
  assert!(!as_bool(get(
    readiness,
    "audit-event-log-install-before-r6"
  )));
  assert!(!as_bool(get(readiness, "fact-store-install-before-r6")));
  assert!(!as_bool(get(
    readiness,
    "expression-projection-owner-before-r6"
  )));
  assert!(!as_bool(get(
    readiness,
    "global-ontology-runtime-before-r6"
  )));
  assert!(!as_bool(get(readiness, "delete-before-r6")));
  assert!(!as_bool(get(readiness, "archive-before-r6")));
  assert_eq!(
    as_str(get(readiness, "r5-verdict")),
    "reverse-replay-verified"
  );
  assert!(as_bool(get(readiness, "triple-replay")));
  assert!(as_bool(get(readiness, "all-deltas-covered")));
  assert!(as_bool(get(readiness, "lift-provenance-covered")));
  assert!(as_bool(get(readiness, "query-intent-need-covered")));
  assert!(as_bool(get(readiness, "emit-projection-surface-covered")));
  assert!(as_bool(get(readiness, "semantic-owner-need-covered")));
  assert!(as_bool(get(readiness, "audit-receipt-need-covered")));
  assert!(!as_bool(get(readiness, "unexplained-mismatch")));
  assert!(as_bool(get(readiness, "audit-refs-preserved")));
  assert!(as_bool(get(readiness, "negative-held-proof-present")));
  assert_eq!(
    as_str(get(readiness, "query-projection-regression-corpus-bound")),
    "regression-corpus-bound-candidate"
  );
  assert_eq!(
    as_str(get(readiness, "semantic-owner-readiness")),
    "semantic-owner-ready-for-r6-review"
  );
  assert_eq!(
    as_str(get(readiness, "audit-receipt-readiness")),
    "audit-receipt-ready-for-r6-review"
  );
  assert_eq!(
    as_str(get(readiness, "runtime-route-proof")),
    "runtime-route-proof-candidate-verified"
  );
  assert!(as_bool(get(readiness, "all-criteria-satisfied")));
}

#[test]
fn human_consequence_authorization_enters_triple_lifecycle_without_bypass() {
  let run = eval_file(&fixture_path()).unwrap();
  let auth = get(&run, "consequence-authorization");
  assert_eq!(
    as_str(get(auth, "source")),
    "human_consequence_gate_flow_discovery_receipt.px::trial.G.choice-accept"
  );
  assert!(as_bool(get(auth, "scope-limited")));
  assert!(!as_bool(get(auth, "runtime-closure-proven")));
  assert!(as_bool(get(auth, "consequence-authorized")));
  assert!(as_bool(get(auth, "enters-pnix-lifecycle")));
  assert!(as_bool(get(auth, "owner-switch-authorization")));
  assert!(!as_bool(get(auth, "bypasses-pnix-lifecycle")));
  assert!(!as_bool(get(auth, "human-is-global-cognition-authority")));
  assert!(!as_bool(get(auth, "rubber-stamp-shortcut")));
  assert!(as_bool(get(auth, "audit-ref-preserved")));
}

#[test]
fn owner_switch_receipt_records_triple_roles_deltas_and_boundaries() {
  let run = eval_file(&fixture_path()).unwrap();
  let receipt = get(&run, "owner-switch-receipt");
  assert_eq!(
    as_str(get(receipt, "id")),
    "r6.owner-switch.lift-query-emit-surface-triple"
  );
  assert_eq!(as_str(get(receipt, "phase")), "R6");
  assert_eq!(
    as_str(get(receipt, "surface-triple")),
    "surface-triple.legacy-ontology.lift-query-emit"
  );
  assert_eq!(
    as_str(get(receipt, "new-owner")),
    "macro-native.lift-query-emit.surface-triple-owner"
  );
  assert_eq!(
    as_str(get(receipt, "macro-probe")),
    "r4.macro-native-lift-query-emit.rewrite-candidate"
  );
  assert!(as_bool(get(receipt, "triple-required")));
  assert!(!as_bool(get(receipt, "query-only-owner-switch")));
  assert!(!as_bool(get(receipt, "emit-only-owner-switch")));
  assert_eq!(
    as_str(get(receipt, "promotion-boundary")),
    "surface-triple-semantic-owner-switch-only"
  );
  assert_eq!(
    as_str(get(receipt, "remaining-compat-role")),
    "legacy-lift-query-emit-reference-specimen-triple-and-query-projection-regression-corpus"
  );

  let roles = string_set(get(receipt, "role-emitted"));
  for role in [
    "role.lift.provenance-semantic-owner-law-gated",
    "role.query.intent-need-owner-law-gated",
    "role.emit.projection-surface-owner-law-gated",
    "role.liftqueryemit.semantic-owner-need-owner-law-gated",
    "role.liftqueryemit.audit-receipt-need-owner-law-gated",
    "role.liftqueryemit.compat-reference-triple-required",
  ] {
    assert!(roles.contains(role), "missing role `{role}`");
  }

  let deltas = attrs_by_id(get(receipt, "reference-delta"));
  assert_eq!(deltas.len(), 7);
  for delta in [
    "delta.lift-provenance",
    "delta.query-intent-need",
    "delta.emit-projection-surface",
    "delta.semantic-owner-need",
    "delta.audit-receipt-need",
    "delta.runtime-and-store",
    "delta.proof",
  ] {
    assert_eq!(
      as_str(get(deltas.get(delta).unwrap(), "verdict")),
      "covered"
    );
  }
}

#[test]
fn owner_switch_does_not_install_globalize_or_create_runtime_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  let receipt = get(&run, "owner-switch-receipt");
  assert!(as_bool(get(receipt, "owner-switch")));
  assert!(as_bool(get(receipt, "surface-triple-scoped")));
  assert!(!as_bool(get(receipt, "runtime-install")));
  assert!(!as_bool(get(receipt, "query-runtime-install")));
  assert!(!as_bool(get(receipt, "fact-store-install")));
  assert!(!as_bool(get(receipt, "audit-event-log-install")));
  assert!(!as_bool(get(receipt, "expression-projection-owner")));
  assert!(!as_bool(get(receipt, "global-ontology-runtime")));
  assert!(!as_bool(get(receipt, "old-lift-query-emit-wrapper")));
  assert!(!as_bool(get(receipt, "delete-legacy-surfaces")));
  assert!(!as_bool(get(receipt, "archive-legacy-surfaces")));
  assert!(!as_bool(get(receipt, "legacy-current-authority")));
  assert!(!as_bool(get(receipt, "implementation-command")));
}

#[test]
fn compat_role_retains_legacy_triple_as_reference_and_regression_corpus() {
  let run = eval_file(&fixture_path()).unwrap();
  let compat = get(&run, "compat-role");
  let surfaces = string_set(get(compat, "legacy-surfaces"));
  for surface in [
    "stdlib/lib/ontology.px::builtins.ontologyLift",
    "stdlib/lib/ontology.px::builtins.ontologyQuery",
    "stdlib/lib/ontology.px::builtins.ontologyEmit",
  ] {
    assert!(
      surfaces.contains(surface),
      "missing compat surface `{surface}`"
    );
  }
  assert_eq!(
    as_str(get(compat, "role-after-switch")),
    "reference-specimen-triple-and-query-projection-regression-corpus"
  );
  assert!(!as_bool(get(compat, "current-semantic-owner")));
  assert!(!as_bool(get(compat, "callable-as-legacy-authority")));
  assert!(!as_bool(get(compat, "wrapper-restored")));
  assert!(!as_bool(get(compat, "delete-now")));
  assert!(!as_bool(get(compat, "archive-now")));
  assert!(as_bool(get(compat, "r7-required")));

  let retained = string_set(get(compat, "retained-for"));
  for expected in [
    "query-projection-regression-corpus",
    "reverse-replay-reference",
    "compat-shell-input-for-r7",
    "supersede-chain-audit",
    "lift-provenance-regression-case",
    "query-intent-regression-case",
    "emit-projection-regression-case",
    "semantic-owner-need-regression-case",
    "audit-receipt-need-regression-case",
  ] {
    assert!(
      retained.contains(expected),
      "missing retained role `{expected}`"
    );
  }
}

#[test]
fn post_switch_state_routes_next_work_without_runtime_install() {
  let run = eval_file(&fixture_path()).unwrap();
  let state = get(&run, "post-switch-state");
  assert_eq!(
    as_str(get(state, "replacement-readiness")),
    "owner-switched-for-lift-query-emit-surface-triple"
  );
  assert!(as_bool(get(state, "owner-switch")));
  assert_eq!(
    as_str(get(state, "semantic-owner")),
    "macro-native.lift-query-emit.surface-triple-owner"
  );
  assert_eq!(
    as_str(get(state, "previous-owner-role")),
    "reference-specimen-triple-and-query-projection-regression-corpus"
  );
  assert!(!as_bool(get(state, "old-authority-active")));
  assert!(as_bool(get(state, "new-authority-surface-triple-scoped")));
  assert!(as_bool(get(state, "triple-required")));
  assert!(!as_bool(get(state, "runtime-install")));
  assert!(!as_bool(get(state, "runtime-executable-now")));
  assert!(!as_bool(get(state, "query-runtime-install")));
  assert!(!as_bool(get(state, "fact-store-install")));
  assert!(!as_bool(get(state, "audit-event-log-install")));
  assert!(!as_bool(get(state, "expression-projection-owner")));
  assert!(!as_bool(get(state, "global-ontology-runtime")));
  assert!(!as_bool(get(state, "old-lift-query-emit-wrapper")));
  assert!(!as_bool(get(state, "delete-legacy-surfaces")));
  assert!(!as_bool(get(state, "archive-legacy-surfaces")));

  let next = string_set(get(state, "next-required"));
  for expected in [
    "r7-lift-query-emit-compat-or-archive-receipt",
    "query-runtime-owner-receipt-before-install",
    "fact-store-owner-receipt-before-install",
    "audit-event-log-owner-receipt-before-install",
    "expression-projection-owner-receipt-before-install",
    "macro-only-boot-host-removal-map",
    "split-proof-before-any-lift-query-emit-owner-split",
  ] {
    assert!(
      next.contains(expected),
      "missing next requirement `{expected}`"
    );
  }
}

#[test]
fn held_trials_block_triple_owner_switch_shortcuts() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "held-owner-switch-trials"));
  assert_eq!(trials.len(), 12);
  for expected in [
    "trial.A.readiness-missing",
    "trial.B.surface-triple-split",
    "trial.C.human-consequence-authorization-missing",
    "trial.D.uncovered-delta",
    "trial.E.semantic-owner-proof-missing",
    "trial.F.audit-receipt-proof-missing",
    "trial.G.regression-corpus-missing",
    "trial.H.runtime-install-requested",
    "trial.I.old-wrapper-restored",
    "trial.J.delete-or-archive-requested",
    "trial.K.llm-prose-owner-switch",
  ] {
    let trial = trials
      .get(expected)
      .unwrap_or_else(|| panic!("missing trial `{expected}`"));
    assert_eq!(as_str(get(trial, "outcome")), "Held");
    assert!(!as_bool(get(trial, "owner-switch")));
  }

  let complete = trials.get("trial.L.complete-owner-switch").unwrap();
  assert_eq!(
    as_str(get(complete, "outcome")),
    "owner-switched-for-lift-query-emit-surface-triple"
  );
  assert!(as_bool(get(complete, "owner-switch")));
}

#[test]
fn six_layer_owner_switch_fold_preserves_triple_and_runtime_boundaries() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-owner-switch-fold");
  assert_eq!(
    as_str(get(fold, "mode")),
    "r6-owner-switch-lift-query-emit-surface-triple"
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
  assert!(as_bool(get_path(fold, &["surface", "owner-switch"])));
  assert_eq!(
    as_str(get_path(fold, &["ontology", "switch-scope"])),
    "surface-triple-scoped"
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
    &["ontology", "other-legacy-surfaces-switched"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "previous-owner-demoted-to-compat"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["semantic", "lift-is-current-fact-store"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["semantic", "query-intent-is-answer-engine"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["semantic", "emit-projection-is-expression-owner"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["semantic", "audit-need-is-event-log"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["gate", "owner-switch-receipt-complete"]
  )));
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
  assert_eq!(
    as_str(get_path(
      fold,
      &["runtime", "runtime-lift-query-emit-owner"]
    )),
    "not-yet-proven"
  );
  assert!(as_bool(get_path(fold, &["audit", "audit-refs-preserved"])));
  assert!(as_bool(get_path(
    fold,
    &["audit", "negative-held-proof-present"]
  )));
}

#[test]
fn runtime_observation_is_owner_switched_but_not_installed() {
  let run = eval_file(&fixture_path()).unwrap();
  let runtime = get(&run, "runtime-observation");
  assert_eq!(
    as_str(get(runtime, "observation-model")),
    "owner-switched-lift-query-emit-surface-triple-non-installed-runtime"
  );
  assert!(as_bool(get(runtime, "can-appear-at-runtime")));
  assert!(as_bool(get(runtime, "owner-switch")));
  assert!(as_bool(get(runtime, "surface-triple-scoped")));
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
fn discoveries_record_d318_through_d326() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 9);
  for expected in [
    "D318.lift-query-emit-owner-switch-is-surface-triple-scoped",
    "D319.readiness-proof-becomes-owner-switch-input-not-runtime-input",
    "D320.semantic-owner-need-closes-as-macro-native-triple-owner",
    "D321.audit-receipt-need-becomes-lineage-obligation-not-event-log-install",
    "D322.triple-dependency-survives-lift-query-emit-owner-switch",
    "D323.legacy-lift-query-emit-retained-as-compat-reference-triple",
    "D324.query-runtime-fact-store-event-log-remain-future-owner-surfaces",
    "D325.human-consequence-authorizes-pnix-lifecycle-without-bypass",
    "D326.lift-query-emit-r6-opens-r7-compat-and-runtime-owner-needs",
  ] {
    let discovery = discoveries
      .get(expected)
      .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
    assert!(as_bool(get(discovery, "scenario-only")));
  }
}

#[test]
fn affected_plans_keep_runtime_and_other_surfaces_unimplemented() {
  let run = eval_file(&fixture_path()).unwrap();
  let affected = get(&run, "affected-plans");
  assert_eq!(
    as_str(get_path(affected, &["macroLiftQueryEmit", "pressure"])),
    "owner-switched-for-lift-query-emit-surface-triple"
  );
  assert_eq!(
    as_str(get_path(affected, &["queryRuntime", "pressure"])),
    "needs-query-runtime-owner-receipt-before-install"
  );
  for key in [
    "legacyLiftQueryEmit",
    "macroLiftQueryEmit",
    "queryRuntime",
    "ContextualFactStore",
    "ExpressionProjectionOwner",
    "AuditEventLog",
    "otherOntologySurfaces",
    "legacyArchive",
  ] {
    assert!(!as_bool(get_path(
      affected,
      &[key, "implementation-target"]
    )));
  }
}

#[test]
fn negative_held_evidence_survives_triple_owner_switch() {
  let run = eval_file(&fixture_path()).unwrap();
  let negative = get(&run, "negative-held-evidence");
  assert_eq!(as_str(get(negative, "status")), "present");
  assert!(as_bool(get(negative, "survives-owner-switch")));
  let rejects = string_set(get(negative, "rejects"));
  for expected in [
    "owner-switch-before-readiness",
    "surface-triple-split-owner-switch",
    "owner-switch-without-human-consequence-authorization",
    "owner-switch-with-uncovered-lift-provenance-delta",
    "owner-switch-with-uncovered-query-intent-delta",
    "owner-switch-with-uncovered-emit-projection-delta",
    "owner-switch-without-semantic-owner-proof",
    "owner-switch-without-audit-receipt-proof",
    "owner-switch-without-audit-ref",
    "owner-switch-without-negative-held-proof",
    "owner-switch-without-query-projection-regression-corpus",
    "query-runtime-install-from-owner-switch",
    "fact-store-install-from-owner-switch",
    "audit-event-log-install-from-owner-switch",
    "expression-projection-owner-from-owner-switch",
    "global-ontology-runtime-from-surface-triple",
    "old-lift-query-emit-wrapper-from-owner-switch",
    "delete-or-archive-from-r6",
    "llm-prose-as-owner-switch",
  ] {
    assert!(rejects.contains(expected), "missing rejection `{expected}`");
  }
}

#[test]
fn blocked_shortcuts_cover_triple_owner_switch_collapse_modes() {
  let run = eval_file(&fixture_path()).unwrap();
  let blocks = string_set(get(&run, "blocked-shortcuts"));
  for expected in [
    "switch-owner-before-readiness",
    "split-query-or-emit-away-from-lift",
    "switch-owner-without-human-consequence-authorization",
    "switch-owner-with-uncovered-lift-provenance-delta",
    "switch-owner-with-uncovered-query-intent-delta",
    "switch-owner-with-uncovered-emit-projection-delta",
    "switch-owner-without-semantic-owner-proof",
    "switch-owner-without-audit-receipt-proof",
    "install-query-runtime-from-owner-switch",
    "install-fact-store-from-owner-switch",
    "install-audit-event-log-from-owner-switch",
    "emit-projection-specimen-as-expression-projection-owner",
    "globalize-lift-query-emit-owner-switch",
    "restore-old-lift-query-emit-wrapper",
    "delete-or-archive-legacy-lift-query-emit-at-r6",
    "treat-llm-prose-as-owner-switch",
  ] {
    assert!(
      blocks.contains(expected),
      "missing shortcut block `{expected}`"
    );
  }
}

#[test]
fn top_level_state_records_triple_owner_switch_without_runtime_command() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "replacement-readiness")),
    "owner-switched-for-lift-query-emit-surface-triple"
  );
  assert!(as_bool(get(&run, "owner-switch")));
  assert!(!as_bool(get(&run, "runtime-install")));
  assert!(!as_bool(get(&run, "query-runtime-install")));
  assert!(!as_bool(get(&run, "fact-store-install")));
  assert!(!as_bool(get(&run, "audit-event-log-install")));
  assert!(!as_bool(get(&run, "expression-projection-owner")));
  assert!(!as_bool(get(&run, "global-ontology-runtime")));
  assert!(!as_bool(get(&run, "implementation-command")));
}
