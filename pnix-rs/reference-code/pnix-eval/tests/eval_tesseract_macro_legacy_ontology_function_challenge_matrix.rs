//! Legacy ontology function challenge matrix.
//!
//! This fixture imports `stdlib/lib/ontology.px` and records how far each old
//! ontology function group has been challenged by the meta-circular tesseract
//! macro replacement line. The test guards against overstating discovery-only
//! surfaces as owner-switched or global runtime-installed.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join(
    "../../fixtures/tesseract-macro-legacy-probe/legacy_ontology_function_challenge_matrix_receipt.px",
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
fn marker_truth_owner_and_constitution_owner_are_pinned() {
  let run = eval_file(&fixture_path()).expect("legacy function matrix fixture must eval");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-legacy-function-challenge-matrix"
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
fn legacy_ontology_surface_is_imported_before_status_claims() {
  let run = eval_file(&fixture_path()).unwrap();
  let ontology = get(&run, "legacy-ontology");
  assert_eq!(as_str(get(ontology, "name")), "stdlib.lib.ontology");
  assert_eq!(
    as_i64(get_path(
      &run,
      &["challenge-summary", "legacy-extern-count"]
    )),
    12
  );

  let types = string_set(get(ontology, "types"));
  for expected in ["Any", "AttrSet", "List"] {
    assert!(
      types.contains(expected),
      "missing ontology type `{expected}`"
    );
  }

  let externs = as_list(get(ontology, "externs"));
  assert_eq!(externs.len(), 12);
}

#[test]
fn imported_extern_names_match_the_full_legacy_surface() {
  let run = eval_file(&fixture_path()).unwrap();
  let extern_names = string_set(get_path(
    &run,
    &["challenge-summary", "legacy-extern-names"],
  ));
  assert_eq!(extern_names.len(), 12);
  for expected in [
    "builtins.ontologyLift",
    "builtins.ontologyEvaluate",
    "builtins.ontologySelect",
    "builtins.ontologyPromote",
    "ontology.lift",
    "ontology.evaluate",
    "ontology.select",
    "ontology.promote",
    "builtins.ontologyQuery",
    "builtins.ontologyEmit",
    "ontology.query",
    "ontology.emit",
  ] {
    assert!(
      extern_names.contains(expected),
      "missing legacy extern `{expected}`"
    );
  }
}

#[test]
fn constitution_gate_blocks_overclaiming_and_external_first_shortcuts() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "legacy-ontology-function-challenge-matrix"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));

  let held_if = string_set(get(gate, "held-if"));
  for expected in [
    "legacy-extern-list-missing",
    "surface-claimed-beyond-receipt",
    "lift-query-emit-claimed-r4-without-r3-receipt",
    "lift-query-emit-claimed-r5-without-r4-receipt",
    "lift-query-emit-claimed-readiness-without-r5-receipt",
    "lift-query-emit-runtime-install-claimed-from-r6-owner-switch",
    "lift-query-emit-runtime-install-claimed-from-r7-compat",
    "evaluate-select-scoped-install-claimed-as-global-runtime",
    "external-solver-used-for-matrix",
    "llm-prose-used-as-challenge-status",
  ] {
    assert!(held_if.contains(expected), "missing held-if `{expected}`");
  }

  let blocked = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "summarize-without-importing-stdlib-ontology-px",
    "treat-old-builtins-tests-as-macro-replacement",
    "claim-lift-query-emit-replaced-from-r3-only",
    "claim-lift-query-emit-replaced-from-r4-only",
    "claim-lift-query-emit-replaced-from-r5-only",
    "claim-lift-query-emit-owner-switched-from-readiness",
    "claim-lift-query-emit-runtime-installed-from-r6-owner-switch",
    "claim-lift-query-emit-runtime-installed-from-r7-compat",
    "delete-lift-query-emit-without-r7-delete-proof",
    "collapse-evaluate-select-scoped-install-into-global-runtime",
    "import-external-solver-before-self-map",
  ] {
    assert!(
      blocked.contains(expected),
      "missing blocked shortcut `{expected}`"
    );
  }
}

#[test]
fn challenge_summary_counts_surface_groups_and_runtime_boundaries() {
  let run = eval_file(&fixture_path()).unwrap();
  let summary = get(&run, "challenge-summary");
  assert_eq!(
    as_str(get(summary, "id")),
    "summary.legacy-ontology-function-challenge.v1"
  );
  assert_eq!(as_i64(get(summary, "legacy-extern-count")), 12);
  assert_eq!(as_i64(get(summary, "covered-legacy-extern-count")), 12);
  assert_eq!(as_i64(get(summary, "matrix-row-count")), 4);
  assert_eq!(as_i64(get(summary, "fully-challenged-surface-groups")), 3);
  assert_eq!(as_i64(get(summary, "r3-only-surface-groups")), 0);
  assert_eq!(as_i64(get(summary, "r4-candidate-surface-groups")), 0);
  assert_eq!(as_i64(get(summary, "r5-replay-surface-groups")), 0);
  assert_eq!(as_i64(get(summary, "readiness-surface-groups")), 0);
  assert_eq!(as_i64(get(summary, "r6-owner-switch-surface-groups")), 0);
  assert_eq!(as_i64(get(summary, "r7-compat-surface-groups")), 2);
  assert_eq!(as_i64(get(summary, "discovery-only-surface-groups")), 0);
  assert_eq!(as_i64(get(summary, "transitional-held-groups")), 1);
  assert_eq!(as_i64(get(summary, "owner-switched-groups")), 3);
  assert_eq!(
    as_i64(get(summary, "scoped-runtime-adapter-install-groups")),
    1
  );
  assert_eq!(as_i64(get(summary, "global-runtime-install-groups")), 0);
  assert_eq!(as_i64(get(summary, "external-solver-dependency-count")), 0);
  assert!(!as_bool(get(summary, "llm-main-system")));
  assert!(!as_bool(get(summary, "old-builtin-current-authority")));
}

#[test]
fn matrix_contains_four_status_rows() {
  let run = eval_file(&fixture_path()).unwrap();
  let matrix = attrs_by_id(get(&run, "matrix"));
  assert_eq!(matrix.len(), 4);
  for expected in [
    "legacy-function.builtins.ontologyPromote",
    "legacy-function.builtins.ontologyEvaluateSelect",
    "legacy-function.builtins.ontologyLiftQueryEmit",
    "legacy-function.transitional-plans",
  ] {
    assert!(
      matrix.contains_key(expected),
      "missing matrix row `{expected}`"
    );
  }
}

#[test]
fn promote_row_has_reached_r7_but_not_runtime_install() {
  let run = eval_file(&fixture_path()).unwrap();
  let matrix = attrs_by_id(get(&run, "matrix"));
  let promote = matrix
    .get("legacy-function.builtins.ontologyPromote")
    .unwrap();
  assert_eq!(as_str(get(promote, "group")), "promote");
  assert_eq!(as_str(get(promote, "reached-phase")), "R7");
  assert_eq!(
    as_str(get(promote, "challenge-status")),
    "r7-compat-retained-owner-switched-surface-scoped"
  );
  assert_eq!(
    as_str(get(promote, "source-receipt")),
    "tesseract-macro-ontology-r7-compat-archive-promote-surface"
  );
  assert!(as_bool(get(promote, "owner-switch")));
  assert!(!as_bool(get(promote, "old-builtin-current-authority")));
  assert!(!as_bool(get(promote, "runtime-adapter-install")));
  assert!(!as_bool(get(promote, "runtime-install")));
  assert!(!as_bool(get(promote, "global-runtime-install")));
}

#[test]
fn evaluate_select_row_is_scoped_adapter_install_not_global_runtime() {
  let run = eval_file(&fixture_path()).unwrap();
  let matrix = attrs_by_id(get(&run, "matrix"));
  let eval_select = matrix
    .get("legacy-function.builtins.ontologyEvaluateSelect")
    .unwrap();
  assert_eq!(as_str(get(eval_select, "group")), "evaluate-select");
  assert_eq!(
    as_str(get(eval_select, "reached-phase")),
    "scoped-runtime-adapter-install"
  );
  assert_eq!(
    as_str(get(eval_select, "source-receipt")),
    "tesseract-macro-ontology-runtime-adapter-install-evaluate-select-surface-pair"
  );
  assert!(as_bool(get(eval_select, "owner-switch")));
  assert!(as_bool(get(eval_select, "runtime-adapter-install")));
  assert_eq!(
    as_str(get(eval_select, "installed-scope")),
    "legacy-evaluate-select-surface-pair-only"
  );
  assert!(!as_bool(get(eval_select, "runtime-install")));
  assert!(!as_bool(get(eval_select, "global-runtime-install")));
  assert!(!as_bool(get(eval_select, "external-solver-required")));
}

#[test]
fn lift_query_emit_row_is_r7_compat_retained_without_runtime_install() {
  let run = eval_file(&fixture_path()).unwrap();
  let matrix = attrs_by_id(get(&run, "matrix"));
  let lqe = matrix
    .get("legacy-function.builtins.ontologyLiftQueryEmit")
    .unwrap();
  assert_eq!(as_str(get(lqe, "group")), "lift-query-emit");
  assert_eq!(as_str(get(lqe, "reached-phase")), "R7");
  assert_eq!(
    as_str(get(lqe, "challenge-status")),
    "r7-compat-retained-owner-switched-no-runtime-install"
  );
  assert_eq!(
    as_str(get(lqe, "source-receipt")),
    "tesseract-macro-ontology-r7-compat-archive-lift-query-emit-surface-triple"
  );
  assert_eq!(
    as_str(get(lqe, "r6-receipt")),
    "tesseract-macro-ontology-r6-owner-switch-lift-query-emit-surface-triple"
  );
  assert_eq!(
    as_str(get(lqe, "r5-receipt")),
    "tesseract-macro-ontology-r5-reverse-replay-lift-query-emit-candidate"
  );
  assert_eq!(
    as_str(get(lqe, "r4-receipt")),
    "tesseract-macro-ontology-r4-macro-native-lift-query-emit-rewrite-candidate"
  );
  assert_eq!(
    as_str(get(lqe, "r3-receipt")),
    "tesseract-macro-ontology-r3-lift-query-emit-role-emission-verdict"
  );
  assert_eq!(
    as_str(get(lqe, "readiness-receipt")),
    "tesseract-macro-ontology-surface-triple-replacement-readiness-lift-query-emit-candidate"
  );
  assert_eq!(
    as_str(get(lqe, "discovery-receipt")),
    "tesseract-macro-ontology-lift-query-emit-discovery"
  );
  assert_eq!(
    as_str(get(lqe, "macro-owner")),
    "macro-native.lift-query-emit.surface-triple-owner"
  );
  assert_eq!(
    as_str(get(lqe, "old-surface-current-role")),
    "compat-reference-regression-corpus-triple"
  );
  assert!(as_bool(get(lqe, "owner-switch")));
  assert!(!as_bool(get(lqe, "runtime-adapter-install")));
  assert!(!as_bool(get(lqe, "runtime-install")));
  assert!(!as_bool(get(lqe, "query-runtime-install")));
  assert!(!as_bool(get(lqe, "fact-store-install")));
  assert!(!as_bool(get(lqe, "audit-event-log-install")));
  assert!(!as_bool(get(lqe, "archive-legacy-surfaces")));
  assert!(!as_bool(get(lqe, "delete-legacy-surfaces")));
  assert_eq!(
    as_str(get(lqe, "replacement-readiness")),
    "owner-switched-for-lift-query-emit-surface-triple"
  );
  assert_eq!(
    as_str(get(lqe, "compat-status")),
    "compat-retained-for-lift-query-emit-surface-triple"
  );
  assert_eq!(
    as_str(get(lqe, "next-frontier")),
    "runtime-owner-or-host-removal-map-if-needed"
  );
}

#[test]
fn transitional_plan_names_remain_observation_handles_not_implementation_targets() {
  let run = eval_file(&fixture_path()).unwrap();
  let matrix = attrs_by_id(get(&run, "matrix"));
  let row = matrix.get("legacy-function.transitional-plans").unwrap();
  assert_eq!(as_str(get(row, "group")), "transitional-plan-names");
  assert_eq!(
    as_str(get(row, "challenge-status")),
    "observation-handles-frozen-or-held"
  );
  assert_eq!(
    as_str(get(row, "reached-phase")),
    "not-implementation-target"
  );
  assert_eq!(as_str(get(row, "macro-owner")), "none");
  assert!(!as_bool(get(row, "owner-switch")));
  assert!(!as_bool(get(row, "runtime-install")));

  let symbols = string_set(get(row, "old-symbols"));
  for expected in [
    "NeedGraph",
    "NeedCursor",
    "CapabilityCard",
    "AssemblyTree",
    "RigorFloor",
    "seed-registry",
    "route-cache",
    "repair-runtime",
  ] {
    assert!(
      symbols.contains(expected),
      "missing transitional name `{expected}`"
    );
  }
}

#[test]
fn every_legacy_extern_is_covered_by_the_matrix_classification() {
  let run = eval_file(&fixture_path()).unwrap();
  let imported = string_set(get_path(
    &run,
    &["challenge-summary", "legacy-extern-names"],
  ));
  let covered = string_set(get_path(
    &run,
    &["challenge-summary", "covered-legacy-externs"],
  ));
  assert_eq!(imported, covered);
}

#[test]
fn no_matrix_row_requires_external_solver_or_global_runtime() {
  let run = eval_file(&fixture_path()).unwrap();
  for row in as_list(get(&run, "matrix")) {
    assert!(as_bool(get(row, "challenged")));
    assert!(!as_bool(get(row, "old-builtin-current-authority")));
    assert!(!as_bool(get(row, "external-solver-required")));
    assert!(!as_bool(get(row, "global-runtime-install")));
  }
  assert_eq!(as_i64(get(&run, "external-solver-dependency-count")), 0);
  assert!(!as_bool(get(&run, "llm-main-system")));
}

#[test]
fn next_frontiers_keep_remaining_work_internal_and_explicit() {
  let run = eval_file(&fixture_path()).unwrap();
  let frontiers = attrs_by_id(get(&run, "next-frontiers"));
  assert_eq!(frontiers.len(), 10);
  for expected in [
    "need.legacy-function.lift-query-emit-r3",
    "need.legacy-function.lift-query-emit-r4",
    "need.legacy-function.lift-query-emit-r5",
    "need.legacy-function.lift-query-emit-readiness",
    "need.legacy-function.lift-query-emit-r6",
    "need.legacy-function.lift-query-emit-r7",
    "need.legacy-function.lift-query-emit-runtime-owner-or-host-removal",
    "need.legacy-function.operation-catalog",
    "held.legacy-function.global-runtime",
    "held.legacy-function.old-builtin-authority",
  ] {
    assert!(
      frontiers.contains_key(expected),
      "missing frontier `{expected}`"
    );
  }
  assert_eq!(
    as_str(get(
      frontiers
        .get("need.legacy-function.lift-query-emit-r3")
        .unwrap(),
      "status"
    )),
    "ClosedByD263D271"
  );
  assert_eq!(
    as_str(get(
      frontiers
        .get("need.legacy-function.lift-query-emit-r4")
        .unwrap(),
      "status"
    )),
    "ClosedByD291D299"
  );
  assert_eq!(
    as_str(get(
      frontiers
        .get("need.legacy-function.lift-query-emit-r5")
        .unwrap(),
      "status"
    )),
    "ClosedByD300D308"
  );
  assert_eq!(
    as_str(get(
      frontiers
        .get("need.legacy-function.lift-query-emit-readiness")
        .unwrap(),
      "status"
    )),
    "ClosedByD309D317"
  );
  assert_eq!(
    as_str(get(
      frontiers
        .get("need.legacy-function.lift-query-emit-r6")
        .unwrap(),
      "status"
    )),
    "ClosedByD318D326"
  );
  assert_eq!(
    as_str(get(
      frontiers
        .get("need.legacy-function.lift-query-emit-r7")
        .unwrap(),
      "status"
    )),
    "ClosedByD327D335"
  );
  assert_eq!(
    as_str(get(
      frontiers
        .get("need.legacy-function.lift-query-emit-runtime-owner-or-host-removal")
        .unwrap(),
      "status"
    )),
    "Need"
  );
  assert_eq!(
    as_str(get(
      frontiers
        .get("held.legacy-function.global-runtime")
        .unwrap(),
      "status"
    )),
    "Held"
  );
  for frontier in frontiers.values() {
    assert!(!as_bool(get(frontier, "external-solver-required")));
  }
}

#[test]
fn discoveries_record_d253_through_d262() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 10);
  for expected in [
    "D253.legacy-ontology-extern-list-is-imported-before-status-claim",
    "D254.promote-is-the-deepest-legacy-challenge-line",
    "D255.evaluate-select-reached-scoped-runtime-adapter-install-not-global-runtime",
    "D256.lift-query-emit-discovery-frontier-is-superseded-by-r3",
    "D257.old-builtin-tests-do-not-equal-macro-replacement-proof",
    "D258.transitional-ontology-plan-names-remain-observation-handles",
    "D259.old-ontology-current-authority-is-false-across-the-matrix",
    "D260.challenge-matrix-is-a-px-interpreted-artifact",
    "D261.next-frontier-is-lift-query-emit-runtime-owner-or-operation-catalog-not-external-solver",
    "D262.all-legacy-externs-are-classified-with-zero-external-dependencies",
  ] {
    let discovery = discoveries
      .get(expected)
      .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
    assert!(as_bool(get(discovery, "scenario-only")));
  }
}

#[test]
fn affected_plans_show_eval_select_and_lift_query_emit_as_next_internal_work() {
  let run = eval_file(&fixture_path()).unwrap();
  let affected = get(&run, "affected-plans");
  assert!(!as_bool(get_path(
    affected,
    &["promoteSurface", "implementation-target"]
  )));
  assert!(as_bool(get_path(
    affected,
    &["evaluateSelectSurfacePair", "implementation-target"]
  )));
  assert!(!as_bool(get_path(
    affected,
    &["liftQueryEmitSurface", "implementation-target"]
  )));
  assert!(!as_bool(get_path(
    affected,
    &["transitionalOntologyPlans", "implementation-target"]
  )));
  assert!(!as_bool(get_path(
    affected,
    &["externalSolverAdapters", "implementation-target"]
  )));
  assert_eq!(
    as_i64(get_path(
      affected,
      &["externalSolverAdapters", "dependency-count"]
    )),
    0
  );
}

#[test]
fn negative_held_evidence_rejects_old_authority_global_runtime_and_prose_proof() {
  let run = eval_file(&fixture_path()).unwrap();
  let negative = get(&run, "negative-held-evidence");
  assert_eq!(as_str(get(negative, "status")), "present");
  let rejects = string_set(get(negative, "rejects"));
  for expected in [
    "old-builtin-tests-as-current-authority",
    "lift-query-emit-owner-switch-from-readiness",
    "lift-query-emit-runtime-install-without-runtime-owner",
    "lift-query-emit-delete-without-r7-delete-proof",
    "evaluate-select-global-runtime-claim",
    "transitional-plan-name-as-implementation",
    "external-solver-first",
    "llm-prose-as-matrix-proof",
  ] {
    assert!(rejects.contains(expected), "missing rejection `{expected}`");
  }
}

#[test]
fn top_level_flags_keep_matrix_candidate_only_without_implementation_command() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "replacement-readiness")),
    "mixed-surface-matrix"
  );
  assert!(!as_bool(get(&run, "owner-switch")));
  assert!(!as_bool(get(&run, "implementation-command")));
  assert_eq!(as_i64(get(&run, "external-solver-dependency-count")), 0);
  assert!(!as_bool(get(&run, "llm-main-system")));
}
