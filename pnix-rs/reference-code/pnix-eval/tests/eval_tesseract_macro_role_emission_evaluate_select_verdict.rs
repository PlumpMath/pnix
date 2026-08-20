//! R3 role-emission verdict for legacy ontologyEvaluate / ontologySelect.
//!
//! D4-D6 turned six-axis evaluation and deterministic selection into specimen
//! evidence. This test pins the next boundary: the macro fold may emit roles
//! for the evaluate/select pair, but score, winner, null select, route cache,
//! and RigorFloor do not become authority.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join(
    "../../fixtures/tesseract-macro-legacy-probe/role_emission_evaluate_select_verdict_receipt.px",
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
fn evalselect_r3_marker_and_constitution_owner_are_pinned() {
  let run = eval_file(&fixture_path()).expect("eval/select R3 fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-r3-evaluate-select-role-emission-verdict"
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
fn constitution_gate_keeps_evalselect_r3_candidate_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "r3-role-emission-evaluate-select-verdict"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "evaluation-score-as-current-proof",
    "select-winner-as-owner-switch",
    "null-select-as-success",
    "route-cache-as-semantic-owner",
    "RigorFloor-from-score-only",
    "install-ranking-runtime-from-r3",
    "treat-llm-prose-as-selection-verdict",
  ] {
    assert!(blocks.contains(expected), "missing gate block `{expected}`");
  }
}

#[test]
fn source_surfaces_remain_evaluate_select_specimens() {
  let run = eval_file(&fixture_path()).unwrap();
  let surfaces = as_list(get(&run, "source-surfaces"));
  assert_eq!(surfaces.len(), 2);
  let symbols: BTreeSet<&str> = surfaces
    .iter()
    .map(|surface| as_str(get(surface, "source-symbol")))
    .collect();
  assert_eq!(
    symbols,
    ["builtins.ontologyEvaluate", "builtins.ontologySelect"]
      .into_iter()
      .collect()
  );
  for surface in surfaces {
    assert_eq!(as_str(get(surface, "specimen-role")), "reference-specimen");
  }
}

#[test]
fn surface_pair_is_scoped_and_dependency_preserved() {
  let run = eval_file(&fixture_path()).unwrap();
  let pair = get(&run, "surface-pair");
  assert_eq!(
    as_str(get(pair, "evaluate")),
    "stdlib/lib/ontology.px::builtins.ontologyEvaluate"
  );
  assert_eq!(
    as_str(get(pair, "select")),
    "stdlib/lib/ontology.px::builtins.ontologySelect"
  );
  assert_eq!(
    as_str(get(pair, "scope")),
    "legacy-evaluate-select-pair-only"
  );
  assert!(as_bool(get(pair, "pair-required")));
  assert!(!as_bool(get(pair, "global-ranking-runtime")));
}

#[test]
fn specimen_evidence_imports_d4_through_d6_behavior() {
  let run = eval_file(&fixture_path()).unwrap();
  let evidence = get(&run, "specimen-evidence");
  assert_eq!(
    as_str(get(evidence, "evaluate-output-shape")),
    "interpretation-plus-six-axis-fields"
  );
  let axes = string_set(get(evidence, "evaluate-axes"));
  for axis in [
    "coherence",
    "coverage",
    "loss",
    "cost",
    "replayability",
    "safety",
    "score",
  ] {
    assert!(axes.contains(axis), "missing axis `{axis}`");
  }
  assert_eq!(
    as_str(get(evidence, "select-output-shape")),
    "best-interpretation-or-null"
  );
  assert_eq!(as_str(get(evidence, "select-empty-list-result")), "null");
  assert!(!as_bool(get(evidence, "evaluate-current-authority")));
  assert!(!as_bool(get(evidence, "select-current-authority")));
  assert_eq!(as_str(get(evidence, "discovery-readiness")), "not-proven");
  assert!(!as_bool(get(evidence, "discovery-owner-switch")));
}

#[test]
fn emitted_roles_demote_score_winner_and_null_select() {
  let run = eval_file(&fixture_path()).unwrap();
  let roles = attrs_by_id(get(&run, "role-emission-verdicts"));
  assert_eq!(roles.len(), 9);
  for expected in [
    "role.evaluation-vector-specimen",
    "role.axis-evidence",
    "role.selection-outcome-specimen",
    "role.candidate-ranking",
    "role.tie-break-evidence",
    "role.empty-selection-held",
    "role.ranking-delta-receipt",
    "role.reverse-replay-requirement",
    "role.evalselect-owner-switch-need",
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
    as_str(get(roles.get("role.axis-evidence").unwrap(), "verdict")),
    "demote-score-to-evidence"
  );
  assert_eq!(
    as_str(get(roles.get("role.candidate-ranking").unwrap(), "verdict")),
    "keep-as-ranking-observation"
  );
  assert_eq!(
    as_str(get(
      roles.get("role.empty-selection-held").unwrap(),
      "verdict"
    )),
    "split-null-into-held"
  );
}

#[test]
fn legacy_plan_roles_are_not_emitted() {
  let run = eval_file(&fixture_path()).unwrap();
  let roles = attrs_by_id(get(&run, "non-emitted-legacy-plan-roles"));
  assert_eq!(roles.len(), 5);
  for expected in [
    "role.rigorfloor-schema",
    "role.benchmarkgraph-store",
    "role.route-cache-authority",
    "role.needcursor-store",
    "role.repaircandidate-runtime",
  ] {
    let role = roles
      .get(expected)
      .unwrap_or_else(|| panic!("missing non-emitted role `{expected}`"));
    assert!(!as_bool(get(role, "emitted")));
  }
  assert_eq!(
    as_str(get(
      roles.get("role.route-cache-authority").unwrap(),
      "verdict"
    )),
    "not-emitted-held"
  );
}

#[test]
fn paired_dependency_blocks_select_without_evaluate() {
  let run = eval_file(&fixture_path()).unwrap();
  let dependency = get(&run, "paired-dependency");
  assert!(as_bool(get(dependency, "evaluate-feeds-select")));
  assert_eq!(as_str(get(dependency, "select-without-evaluate")), "Held");
  assert!(!as_bool(get(dependency, "evaluate-score-current-proof")));
  assert!(!as_bool(get(dependency, "select-winner-current-proof")));
  assert_eq!(
    as_str(get(dependency, "pair-scope")),
    "legacy-evaluate-select-pair-only"
  );
  assert!(!as_bool(get(dependency, "split-rewrite-allowed")));
}

#[test]
fn six_layer_role_fold_preserves_score_winner_and_runtime_boundaries() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-role-fold");
  assert_eq!(
    as_str(get(fold, "mode")),
    "r3-role-emission-evaluate-select-verdict"
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
  assert!(!as_bool(get_path(
    fold,
    &["ontology", "global-ranking-runtime"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "score-demoted-to-evidence"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "winner-demoted-to-observation"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "null-select-demoted-to-held"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "route-cache-authority-blocked"]
  )));
  assert!(!as_bool(get_path(fold, &["gate", "owner-switch"])));
  assert!(!as_bool(get_path(fold, &["runtime", "executable-now"])));
  assert!(!as_bool(get_path(
    fold,
    &["runtime", "ranking-runtime-installed"]
  )));
  assert_eq!(
    as_str(get_path(fold, &["audit", "reverse-replay-status"])),
    "required-not-run"
  );
}

#[test]
fn r4_entry_opens_only_for_paired_rewrite_candidate() {
  let run = eval_file(&fixture_path()).unwrap();
  let boundary = get(&run, "r4-entry-boundary");
  assert!(as_bool(get(
    boundary,
    "r3-verdict-closed-for-this-surface-pair"
  )));
  assert!(as_bool(get(
    boundary,
    "r4-macro-native-rewrite-candidate-may-start"
  )));
  assert_eq!(
    as_str(get(boundary, "r4-scope")),
    "legacy-evaluate-select-pair-only"
  );
  assert!(!as_bool(get(boundary, "broad-ranking-runtime-open")));
  assert!(!as_bool(get(boundary, "owner-switch-open")));
  assert!(!as_bool(get(boundary, "runtime-install-open")));
  assert!(!as_bool(get(boundary, "rigorfloor-schema-open")));
  assert!(!as_bool(get(boundary, "route-cache-open")));

  let required = string_set(get(boundary, "required-next"));
  for expected in [
    "macro-native-evaluate-select-rewrite-candidate",
    "ranking-reference-delta",
    "reverse-replay",
    "empty-selection-held-proof",
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
    "r3-evaluate-select-role-emission-runtime-candidates"
  );
  assert!(as_bool(get(runtime, "can-appear-at-runtime")));
  assert!(as_bool(get(runtime, "candidate-only")));
  assert!(!as_bool(get(runtime, "canonical-runtime-installed")));
  assert!(!as_bool(get(runtime, "executable-now")));
  assert!(!as_bool(get(runtime, "owner-switch")));
  assert_eq!(as_list(get(runtime, "runtime-added-candidates")).len(), 4);
}

#[test]
fn discoveries_record_d134_through_d142() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 9);
  for expected in [
    "D134.evalselect-r3-is-surface-pair-scoped",
    "D135.score-demotes-to-axis-evidence-not-rigorfloor",
    "D136.select-winner-demotes-to-ranking-observation",
    "D137.null-select-emits-held-not-success",
    "D138.tie-break-order-is-delta-evidence-not-route-cache",
    "D139.evaluate-select-replay-must-stay-paired",
    "D140.r4-entry-opens-for-evaluate-select-pair-only",
    "D141.evalselect-runtime-candidates-are-non-executable",
    "D142.evalselect-preserves-pnix-independence-from-llm-selection",
  ] {
    let discovery = discoveries
      .get(expected)
      .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
    assert!(as_bool(get(discovery, "scenario-only")));
  }
}

#[test]
fn affected_plans_remain_observation_handles() {
  let run = eval_file(&fixture_path()).unwrap();
  let affected = get(&run, "affected-plans");
  assert_eq!(
    as_str(get_path(affected, &["RigorFloor", "pressure"])),
    "demote-score-to-axis-evidence"
  );
  assert_eq!(
    as_str(get_path(affected, &["routeCache", "pressure"])),
    "hold-route-cache-authority"
  );
  assert_eq!(
    as_str(get_path(affected, &["evaluateSelectRewrite", "pressure"])),
    "may-start-paired-r4-rewrite"
  );
  for key in [
    "RigorFloor",
    "routeCache",
    "NeedCursor",
    "RepairCandidate",
    "evaluateSelectRewrite",
  ] {
    assert!(!as_bool(get_path(
      affected,
      &[key, "implementation-target"]
    )));
  }
}

#[test]
fn negative_held_evidence_blocks_score_winner_null_runtime_and_llm_shortcuts() {
  let run = eval_file(&fixture_path()).unwrap();
  let negative = get(&run, "negative-held-evidence");
  assert_eq!(as_str(get(negative, "status")), "present");
  let rejects = string_set(get(negative, "rejects"));
  for expected in [
    "score-as-current-proof",
    "winner-as-owner-switch",
    "null-select-as-success",
    "tie-break-as-route-cache",
    "rigorfloor-from-score-only",
    "evaluate-select-split-without-dependency-proof",
    "runtime-ranking-install-from-r3",
    "llm-prose-selection-verdict",
  ] {
    assert!(rejects.contains(expected), "missing rejection `{expected}`");
  }
}

#[test]
fn blocked_shortcuts_cover_rigorfloor_route_cache_and_selection_claims() {
  let run = eval_file(&fixture_path()).unwrap();
  let blocks = string_set(get(&run, "blocked-shortcuts"));
  for expected in [
    "evaluation-score-as-current-proof",
    "select-winner-as-owner-switch",
    "null-select-as-success",
    "route-cache-as-semantic-owner",
    "RigorFloor-from-score-only",
    "BenchmarkGraph-from-evaluate-select-pair",
    "NeedCursor-from-winner",
    "RepairCandidate-from-null-select-without-held",
    "install-ranking-runtime-from-r3",
    "treat-llm-prose-as-selection-verdict",
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
  assert!(!as_bool(get(&run, "global-ranking-runtime")));
  assert!(!as_bool(get(&run, "implementation-command")));
}
