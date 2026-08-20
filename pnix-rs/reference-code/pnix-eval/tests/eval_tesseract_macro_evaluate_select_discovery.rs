//! Discovery receipt for legacy ontologyEvaluate / ontologySelect surfaces.
//!
//! This keeps six-axis evaluation and deterministic selection as specimen data
//! until the tesseract macro emits stable roles and replay/audit proof.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base)
    .join("../../fixtures/tesseract-macro-legacy-probe/evaluate_select_discovery_receipt.px")
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
  as_list(v).iter().map(|item| as_str(item)).collect()
}

fn string_set(v: &Value) -> BTreeSet<&str> {
  list_strings(v).into_iter().collect()
}

#[test]
fn evaluate_select_discovery_marker_and_owners_are_pinned() {
  let run = eval_file(&fixture_path()).expect("evaluate/select discovery fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-evaluate-select-discovery"
  );
  assert_eq!(
    as_str(get(&run, "truth-owner")),
    "project-wiki/maps/tesseract-macro-ontology-discovery-ledger.md"
  );
  assert_eq!(
    as_str(get(&run, "replacement-map")),
    "project-wiki/maps/tesseract-macro-ontology-replacement-map.md"
  );
}

#[test]
fn evaluate_select_discovery_names_both_legacy_surfaces_as_specimens() {
  let run = eval_file(&fixture_path()).unwrap();
  let surfaces = as_list(get(&run, "source-surfaces"));
  assert_eq!(surfaces.len(), 2);
  let symbols: BTreeSet<&str> = surfaces
    .iter()
    .map(|s| as_str(get(s, "source-symbol")))
    .collect();
  assert_eq!(
    symbols,
    ["builtins.ontologyEvaluate", "builtins.ontologySelect"]
      .into_iter()
      .collect()
  );
  for surface in surfaces {
    assert_eq!(
      as_str(get(surface, "source-file")),
      "stdlib/lib/ontology.px"
    );
    assert_eq!(as_str(get(surface, "specimen-role")), "reference-specimen");
  }
}

#[test]
fn old_behavior_records_axes_and_tie_breaks_without_current_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  let axes = string_set(get_path(&run, &["old-behavior", "evaluate", "axes"]));
  assert_eq!(
    axes,
    [
      "coherence",
      "coverage",
      "loss",
      "cost",
      "replayability",
      "safety",
      "score"
    ]
    .into_iter()
    .collect()
  );
  assert_eq!(
    as_str(get_path(
      &run,
      &["old-behavior", "evaluate", "authority-style"]
    )),
    "deterministic-score"
  );

  let tie_breaks = list_strings(get_path(
    &run,
    &["old-behavior", "select", "tie-break-order"],
  ));
  assert_eq!(
    tie_breaks,
    vec![
      "score",
      "safety",
      "replayability",
      "lower-loss",
      "lower-cost",
      "lexical-interpretation-id"
    ]
  );
  assert_eq!(
    as_str(get_path(
      &run,
      &["old-behavior", "select", "authority-style"]
    )),
    "deterministic-winner"
  );
}

#[test]
fn macro_candidate_keeps_layers_visible_and_runtime_non_executable() {
  let run = eval_file(&fixture_path()).unwrap();
  assert!(!as_bool(get_path(
    &run,
    &["macro-candidate", "direct-authority"]
  )));
  assert!(as_bool(get_path(
    &run,
    &["macro-candidate", "candidate-only"]
  )));
  assert!(!as_bool(get_path(
    &run,
    &["macro-candidate", "owner-switch"]
  )));
  assert!(!as_bool(get_path(
    &run,
    &["macro-candidate", "runtime-executable"]
  )));
  assert_eq!(
    as_str(get_path(
      &run,
      &["macro-candidate", "replacement-readiness"]
    )),
    "not-proven"
  );

  let layers = get_path(&run, &["macro-candidate", "layers"]);
  for key in [
    "surface", "ontology", "semantic", "gate", "runtime", "audit",
  ] {
    assert!(as_bool(get(layers, key)), "layer `{key}` must stay visible");
  }
}

#[test]
fn macro_candidate_roles_do_not_prebuild_rigor_or_route_cache() {
  let run = eval_file(&fixture_path()).unwrap();
  let roles = get_path(&run, &["macro-candidate", "roles"]);
  assert_eq!(
    as_str(get(roles, "EvaluationVectorSpecimen")),
    "comparison/audit-baseline"
  );
  assert_eq!(
    as_str(get(roles, "CandidateRanking")),
    "route-candidate-observation"
  );
  assert_eq!(as_str(get(roles, "CandidateGate")), "blocks-direct-winner");
  assert_eq!(
    as_str(get(roles, "HeldEntry")),
    "missing-or-contradicted-candidate"
  );
  assert_eq!(
    as_str(get(roles, "AuditReceipt")),
    "delta-and-replay-required"
  );
}

#[test]
fn discoveries_capture_axis_ranking_and_held_pressure() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = as_list(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 3);
  let ids: BTreeSet<&str> = discoveries.iter().map(|d| as_str(get(d, "id"))).collect();
  assert_eq!(
    ids,
    [
      "D4.evaluate-axis-is-audit-baseline",
      "D5.select-winner-is-ranking-observation",
      "D6.empty-select-becomes-held-pressure"
    ]
    .into_iter()
    .collect()
  );
  let effects: Vec<&str> = discoveries
    .iter()
    .map(|d| as_str(get(d, "macro-effect")))
    .collect();
  assert!(
    effects
      .iter()
      .any(|effect| effect.contains("not RigorFloor authority")),
    "evaluate axes must not become RigorFloor authority by shortcut"
  );
  assert!(
    effects
      .iter()
      .any(|effect| effect.contains("CandidateRanking observation")),
    "select winner must remain ranking observation"
  );
  assert!(
    effects
      .iter()
      .any(|effect| effect.contains("Held/missing-input")),
    "empty select must become Held pressure"
  );
}

#[test]
fn transitional_names_remain_observation_handles() {
  let run = eval_file(&fixture_path()).unwrap();
  let names = get(&run, "transitional-names-affected");
  for expected in [
    "RigorFloor",
    "BenchmarkGraph",
    "routeCache",
    "NeedCursor",
    "RepairCandidate",
  ] {
    let entry = get(names, expected);
    assert_eq!(as_str(get(entry, "role")), "observation-handle");
    assert!(
      !as_bool(get(entry, "implementation-target")),
      "`{expected}` must remain observation-only"
    );
  }
  assert_eq!(
    as_str(get_path(names, &["RigorFloor", "pressure"])),
    "demote"
  );
  assert_eq!(
    as_str(get_path(names, &["NeedCursor", "pressure"])),
    "redesign"
  );
  assert_eq!(
    as_str(get_path(names, &["RepairCandidate", "pressure"])),
    "split"
  );
}

#[test]
fn blocked_shortcuts_and_held_conditions_protect_readiness_boundary() {
  let run = eval_file(&fixture_path()).unwrap();
  let blocks = string_set(get(&run, "blocked-shortcuts"));
  for expected in [
    "evaluation-score-as-current-proof",
    "select-winner-as-owner-switch",
    "null-select-as-success",
    "route-cache-as-semantic-owner",
    "RigorFloor-from-score-only",
  ] {
    assert!(
      blocks.contains(expected),
      "missing blocked shortcut `{expected}`"
    );
  }

  let held_if = string_set(get_path(&run, &["negative-held-evidence", "held-if"]));
  for expected in [
    "empty-candidate-list",
    "contradicted-status",
    "unnamed-ranking-delta",
    "reverse-replay-missing",
  ] {
    assert!(
      held_if.contains(expected),
      "missing Held condition `{expected}`"
    );
  }
  assert_eq!(
    as_str(get(&run, "reverse-replay-status")),
    "required-not-run"
  );
  assert_eq!(as_str(get(&run, "replacement-readiness")), "not-proven");
  assert!(!as_bool(get(&run, "owner-switch")));
  assert!(!as_bool(get(&run, "implementation-command")));
}
