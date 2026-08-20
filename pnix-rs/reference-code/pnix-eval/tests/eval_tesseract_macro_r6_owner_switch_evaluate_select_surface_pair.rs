//! R6 owner switch for the macro-native evaluate/select surface pair.
//!
//! The surface-pair readiness receipt opened R6 review for the dependent
//! `ontologyEvaluate` / `ontologySelect` pair. This test pins the next
//! boundary: owner switch is now true for that pair only, while ranking
//! runtime install, global ranking runtime, RigorFloor / route cache authority,
//! old wrappers, delete/archive, and LLM-prose authority remain blocked.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join(
    "../../fixtures/tesseract-macro-legacy-probe/owner_switch_evaluate_select_surface_pair_receipt.px",
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
fn evalselect_owner_switch_marker_and_constitution_owner_are_pinned() {
  let run = eval_file(&fixture_path()).expect("eval/select owner-switch fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-r6-owner-switch-evaluate-select-surface-pair"
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
fn constitution_gate_keeps_r6_pair_owner_switch_candidate_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "r6-owner-switch-evaluate-select-surface-pair"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  assert_eq!(
    as_str(get(gate, "replacement-readiness")),
    "owner-switched-for-evaluate-select-surface-pair"
  );

  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "switch-owner-before-readiness",
    "split-select-owner-away-from-evaluate",
    "switch-owner-without-human-consequence-authorization",
    "install-ranking-runtime-from-owner-switch",
    "globalize-evaluate-select-owner-switch",
    "emit-score-as-RigorFloor",
    "emit-tie-break-as-route-cache",
    "restore-old-evaluate-select-wrapper",
    "delete-or-archive-legacy-evaluate-select-at-r6",
    "treat-llm-prose-as-owner-switch",
  ] {
    assert!(blocks.contains(expected), "missing gate block `{expected}`");
  }
}

#[test]
fn surface_pair_owner_switch_is_scoped_and_pair_required() {
  let run = eval_file(&fixture_path()).unwrap();
  let pair = get(&run, "surface-pair");
  assert_eq!(
    as_str(get(pair, "id")),
    "surface-pair.legacy-ontology.evaluate-select"
  );
  assert_eq!(
    as_str(get(pair, "evaluate")),
    "stdlib/lib/ontology.px::builtins.ontologyEvaluate"
  );
  assert_eq!(
    as_str(get(pair, "select")),
    "stdlib/lib/ontology.px::builtins.ontologySelect"
  );
  assert_eq!(
    as_str(get(pair, "previous-owner")),
    "legacy-ontology.evaluate-select.reference-specimen-pair"
  );
  assert_eq!(
    as_str(get(pair, "new-owner")),
    "macro-native.evaluate-select.surface-pair-owner"
  );
  assert_eq!(
    as_str(get(pair, "scope")),
    "legacy-evaluate-select-pair-only"
  );
  assert!(as_bool(get(pair, "pair-required")));
  assert!(!as_bool(get(pair, "select-only-owner-switch")));
  assert!(!as_bool(get(pair, "global-ranking-runtime")));
}

#[test]
fn readiness_input_imports_pair_ready_state_without_prior_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  let readiness = get(&run, "readiness-input");
  assert_eq!(
    as_str(get(readiness, "readiness")),
    "ready-for-r6-owner-switch-receipt"
  );
  assert_eq!(
    as_str(get(readiness, "surface-pair")),
    "surface-pair.legacy-ontology.evaluate-select"
  );
  assert_eq!(
    as_str(get(readiness, "candidate")),
    "r4.macro-native-evaluate-select.rewrite-candidate"
  );
  assert!(!as_bool(get(readiness, "owner-switch-before-r6")));
  assert!(!as_bool(get(readiness, "runtime-install-before-r6")));
  assert!(!as_bool(get(
    readiness,
    "ranking-runtime-install-before-r6"
  )));
  assert!(!as_bool(get(readiness, "global-ranking-runtime-before-r6")));
  assert!(!as_bool(get(readiness, "rigorfloor-authority-before-r6")));
  assert!(!as_bool(get(readiness, "route-cache-authority-before-r6")));
  assert!(!as_bool(get(readiness, "delete-before-r6")));
  assert!(!as_bool(get(readiness, "archive-before-r6")));
  assert_eq!(
    as_str(get(readiness, "r5-verdict")),
    "reverse-replay-verified"
  );
  assert!(as_bool(get(readiness, "paired-replay")));
  assert!(as_bool(get(readiness, "all-deltas-covered")));
  assert!(as_bool(get(readiness, "score-axis-covered")));
  assert!(as_bool(get(readiness, "winner-ranking-covered")));
  assert!(as_bool(get(readiness, "null-held-covered")));
  assert!(as_bool(get(readiness, "tie-break-delta-covered")));
  assert!(!as_bool(get(readiness, "unexplained-mismatch")));
  assert!(as_bool(get(readiness, "audit-refs-preserved")));
  assert!(as_bool(get(readiness, "negative-held-proof-present")));
  assert_eq!(
    as_str(get(readiness, "ranking-regression-corpus-bound")),
    "regression-corpus-bound-candidate"
  );
  assert_eq!(
    as_str(get(readiness, "runtime-route-proof")),
    "runtime-route-proof-candidate-verified"
  );
  assert!(as_bool(get(readiness, "all-criteria-satisfied")));
}

#[test]
fn human_consequence_authorization_enters_pair_lifecycle_without_bypass() {
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
fn owner_switch_receipt_records_pair_roles_deltas_and_boundaries() {
  let run = eval_file(&fixture_path()).unwrap();
  let receipt = get(&run, "owner-switch-receipt");
  assert_eq!(
    as_str(get(receipt, "id")),
    "r6.owner-switch.evaluate-select-surface-pair"
  );
  assert_eq!(as_str(get(receipt, "phase")), "R6");
  assert_eq!(
    as_str(get(receipt, "surface-pair")),
    "surface-pair.legacy-ontology.evaluate-select"
  );
  assert_eq!(
    as_str(get(receipt, "new-owner")),
    "macro-native.evaluate-select.surface-pair-owner"
  );
  assert_eq!(
    as_str(get(receipt, "macro-probe")),
    "r4.macro-native-evaluate-select.rewrite-candidate"
  );
  assert!(as_bool(get(receipt, "pair-required")));
  assert!(!as_bool(get(receipt, "select-only-owner-switch")));
  assert_eq!(
    as_str(get(receipt, "promotion-boundary")),
    "surface-pair-owner-switch-only"
  );
  assert_eq!(
    as_str(get(receipt, "remaining-compat-role")),
    "legacy-evaluate-select-reference-specimen-pair-and-ranking-regression-corpus"
  );

  let roles = string_set(get(receipt, "role-emitted"));
  for role in [
    "role.evaluate.axis-evidence-owner-law-gated",
    "role.select.candidate-ranking-owner-law-gated",
    "role.evalselect.empty-selection-held-owner-law-gated",
    "role.evalselect.tie-break-delta-owner-law-gated",
    "role.evalselect.compat-reference-pair-required",
  ] {
    assert!(roles.contains(role), "missing role `{role}`");
  }

  let deltas = attrs_by_id(get(receipt, "reference-delta"));
  assert_eq!(deltas.len(), 6);
  for delta in [
    "delta.score-authority",
    "delta.winner-authority",
    "delta.null-behavior",
    "delta.tie-break-order",
    "delta.runtime",
    "delta.proof",
  ] {
    assert_eq!(
      as_str(get(deltas.get(delta).unwrap(), "verdict")),
      "covered"
    );
  }
}

#[test]
fn owner_switch_does_not_install_globalize_or_create_ranking_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  let receipt = get(&run, "owner-switch-receipt");
  assert!(as_bool(get(receipt, "owner-switch")));
  assert!(as_bool(get(receipt, "surface-pair-scoped")));
  assert!(!as_bool(get(receipt, "runtime-install")));
  assert!(!as_bool(get(receipt, "ranking-runtime-install")));
  assert!(!as_bool(get(receipt, "global-ranking-runtime")));
  assert!(!as_bool(get(receipt, "rigorfloor-authority")));
  assert!(!as_bool(get(receipt, "route-cache-authority")));
  assert!(!as_bool(get(receipt, "old-evaluate-select-wrapper")));
  assert!(!as_bool(get(receipt, "delete-legacy-surfaces")));
  assert!(!as_bool(get(receipt, "archive-legacy-surfaces")));
  assert!(!as_bool(get(receipt, "legacy-current-authority")));
  assert!(!as_bool(get(receipt, "implementation-command")));
}

#[test]
fn compat_role_retains_legacy_pair_as_reference_and_regression_corpus() {
  let run = eval_file(&fixture_path()).unwrap();
  let compat = get(&run, "compat-role");
  let surfaces = string_set(get(compat, "legacy-surfaces"));
  for surface in [
    "stdlib/lib/ontology.px::builtins.ontologyEvaluate",
    "stdlib/lib/ontology.px::builtins.ontologySelect",
  ] {
    assert!(
      surfaces.contains(surface),
      "missing compat surface `{surface}`"
    );
  }
  assert_eq!(
    as_str(get(compat, "role-after-switch")),
    "reference-specimen-pair-and-ranking-regression-corpus"
  );
  assert!(!as_bool(get(compat, "current-semantic-owner")));
  assert!(!as_bool(get(compat, "callable-as-legacy-authority")));
  assert!(!as_bool(get(compat, "wrapper-restored")));
  assert!(!as_bool(get(compat, "delete-now")));
  assert!(!as_bool(get(compat, "archive-now")));
  assert!(as_bool(get(compat, "r7-required")));

  let retained = string_set(get(compat, "retained-for"));
  for expected in [
    "ranking-regression-corpus",
    "reverse-replay-reference",
    "compat-shell-input-for-r7",
    "supersede-chain-audit",
    "null-held-regression-case",
    "tie-break-reference-delta",
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
    "owner-switched-for-evaluate-select-surface-pair"
  );
  assert!(as_bool(get(state, "owner-switch")));
  assert_eq!(
    as_str(get(state, "semantic-owner")),
    "macro-native.evaluate-select.surface-pair-owner"
  );
  assert_eq!(
    as_str(get(state, "previous-owner-role")),
    "reference-specimen-pair-and-ranking-regression-corpus"
  );
  assert!(!as_bool(get(state, "old-authority-active")));
  assert!(as_bool(get(state, "new-authority-surface-pair-scoped")));
  assert!(as_bool(get(state, "pair-required")));
  assert!(!as_bool(get(state, "runtime-install")));
  assert!(!as_bool(get(state, "runtime-executable-now")));
  assert!(!as_bool(get(state, "ranking-runtime-install")));
  assert!(!as_bool(get(state, "global-ranking-runtime")));
  assert!(!as_bool(get(state, "rigorfloor-authority")));
  assert!(!as_bool(get(state, "route-cache-authority")));
  assert!(!as_bool(get(state, "old-evaluate-select-wrapper")));
  assert!(!as_bool(get(state, "delete-legacy-surfaces")));
  assert!(!as_bool(get(state, "archive-legacy-surfaces")));

  let next = string_set(get(state, "next-required"));
  for expected in [
    "r7-evalselect-compat-or-archive-receipt",
    "runtime-ranking-owner-receipt-before-install",
    "separate-surface-receipts-for-lift-query-emit",
    "split-proof-before-any-evaluate-select-owner-split",
  ] {
    assert!(
      next.contains(expected),
      "missing next requirement `{expected}`"
    );
  }
}

#[test]
fn held_trials_block_pair_owner_switch_shortcuts() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "held-owner-switch-trials"));
  assert_eq!(trials.len(), 12);
  for expected in [
    "trial.A.readiness-missing",
    "trial.B.surface-pair-split",
    "trial.C.human-consequence-authorization-missing",
    "trial.D.uncovered-delta",
    "trial.E.regression-corpus-missing",
    "trial.F.runtime-install-requested",
    "trial.G.rigorfloor-or-route-cache-authority",
    "trial.H.global-owner-switch-requested",
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
    "owner-switched-for-evaluate-select-surface-pair"
  );
  assert!(as_bool(get(complete, "owner-switch")));
}

#[test]
fn six_layer_owner_switch_fold_preserves_pair_and_runtime_boundaries() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-owner-switch-fold");
  assert_eq!(
    as_str(get(fold, "mode")),
    "r6-owner-switch-evaluate-select-surface-pair"
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
  assert!(as_bool(get_path(fold, &["surface", "owner-switch"])));
  assert_eq!(
    as_str(get_path(fold, &["ontology", "switch-scope"])),
    "surface-pair-scoped"
  );
  assert!(!as_bool(get_path(
    fold,
    &["ontology", "global-ranking-runtime"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["ontology", "other-legacy-surfaces-switched"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["ontology", "rigorfloor-authority"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["ontology", "route-cache-authority"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "previous-owner-demoted-to-compat"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["semantic", "score-is-current-proof"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["semantic", "winner-is-current-proof"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["semantic", "null-select-is-success"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["gate", "owner-switch-receipt-complete"]
  )));
  assert!(!as_bool(get_path(fold, &["runtime", "executable-now"])));
  assert!(!as_bool(get_path(fold, &["runtime", "installed"])));
  assert!(!as_bool(get_path(
    fold,
    &["runtime", "ranking-runtime-installed"]
  )));
  assert_eq!(
    as_str(get_path(fold, &["runtime", "runtime-ranking-owner"])),
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
    "owner-switched-evaluate-select-surface-pair-non-installed-runtime"
  );
  assert!(as_bool(get(runtime, "can-appear-at-runtime")));
  assert!(as_bool(get(runtime, "owner-switch")));
  assert!(as_bool(get(runtime, "surface-pair-scoped")));
  assert!(!as_bool(get(runtime, "canonical-runtime-installed")));
  assert!(!as_bool(get(runtime, "executable-now")));
  assert!(!as_bool(get(runtime, "ranking-runtime-installed")));
  assert!(!as_bool(get(runtime, "global-ranking-runtime")));
  assert_eq!(as_list(get(runtime, "runtime-added-candidates")).len(), 3);
}

#[test]
fn discoveries_record_d170_through_d178() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 9);
  for expected in [
    "D170.evalselect-owner-switch-is-surface-pair-scoped",
    "D171.pair-dependency-survives-owner-switch",
    "D172.evalselect-owner-switch-requires-readiness-and-human-consequence",
    "D173.ranking-regression-corpus-survives-owner-switch",
    "D174.evalselect-owner-switch-is-not-ranking-runtime-install",
    "D175.score-and-tie-break-do-not-become-rigorfloor-or-route-cache",
    "D176.legacy-evaluate-select-retained-as-compat-reference-pair",
    "D177.evalselect-owner-switch-blocks-old-wrapper-and-prose-authority",
    "D178.evalselect-r6-opens-r7-and-runtime-owner-needs",
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
    as_str(get_path(affected, &["macroEvaluateSelect", "pressure"])),
    "owner-switched-for-evaluate-select-surface-pair"
  );
  assert_eq!(
    as_str(get_path(affected, &["rankingRuntime", "pressure"])),
    "needs-runtime-ranking-owner-receipt-before-install"
  );
  assert_eq!(
    as_str(get_path(affected, &["otherOntologySurfaces", "pressure"])),
    "separate-receipts-required"
  );
  for key in [
    "legacyEvaluateSelect",
    "macroEvaluateSelect",
    "rankingRuntime",
    "RigorFloor",
    "routeCache",
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
fn negative_held_evidence_survives_pair_owner_switch() {
  let run = eval_file(&fixture_path()).unwrap();
  let negative = get(&run, "negative-held-evidence");
  assert_eq!(as_str(get(negative, "status")), "present");
  assert!(as_bool(get(negative, "survives-owner-switch")));
  let rejects = string_set(get(negative, "rejects"));
  for expected in [
    "owner-switch-before-readiness",
    "surface-pair-split-owner-switch",
    "owner-switch-without-human-consequence-authorization",
    "owner-switch-with-uncovered-score-axis-delta",
    "owner-switch-with-uncovered-winner-ranking-delta",
    "owner-switch-with-null-success-collapse",
    "owner-switch-with-tie-break-route-cache-collapse",
    "owner-switch-without-audit-ref",
    "owner-switch-without-negative-held-proof",
    "owner-switch-without-ranking-regression-corpus",
    "ranking-runtime-install-from-owner-switch",
    "rigorfloor-or-route-cache-authority-from-owner-switch",
    "global-ranking-owner-switch-from-surface-pair",
    "old-evaluate-select-wrapper-from-owner-switch",
    "delete-or-archive-from-r6",
    "llm-prose-as-owner-switch",
  ] {
    assert!(rejects.contains(expected), "missing rejection `{expected}`");
  }
}

#[test]
fn blocked_shortcuts_cover_pair_owner_switch_collapse_modes() {
  let run = eval_file(&fixture_path()).unwrap();
  let blocks = string_set(get(&run, "blocked-shortcuts"));
  for expected in [
    "switch-owner-before-readiness",
    "split-select-owner-away-from-evaluate",
    "switch-owner-without-human-consequence-authorization",
    "switch-owner-with-uncovered-score-axis-delta",
    "switch-owner-with-uncovered-winner-ranking-delta",
    "switch-owner-with-null-success-collapse",
    "switch-owner-with-tie-break-route-cache-collapse",
    "install-ranking-runtime-from-owner-switch",
    "globalize-evaluate-select-owner-switch",
    "emit-score-as-RigorFloor",
    "emit-tie-break-as-route-cache",
    "restore-old-evaluate-select-wrapper",
    "delete-or-archive-legacy-evaluate-select-at-r6",
    "treat-llm-prose-as-owner-switch",
  ] {
    assert!(
      blocks.contains(expected),
      "missing shortcut block `{expected}`"
    );
  }
}

#[test]
fn top_level_state_records_pair_owner_switch_without_runtime_command() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "replacement-readiness")),
    "owner-switched-for-evaluate-select-surface-pair"
  );
  assert!(as_bool(get(&run, "owner-switch")));
  assert!(!as_bool(get(&run, "runtime-install")));
  assert!(!as_bool(get(&run, "ranking-runtime-install")));
  assert!(!as_bool(get(&run, "global-ranking-runtime")));
  assert!(!as_bool(get(&run, "rigorfloor-authority")));
  assert!(!as_bool(get(&run, "route-cache-authority")));
  assert!(!as_bool(get(&run, "implementation-command")));
}
