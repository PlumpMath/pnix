//! Scenario discovery for old A->B->C versus macro A->C path compression.
//!
//! This is an experiment contract: middle B may emerge as a candidate clue, but
//! it is not accepted proof or an implementation command.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base)
    .join("../../fixtures/tesseract-macro-legacy-probe/path_compression_discovery_receipt.px")
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
fn path_compression_marker_and_truth_owner_are_pinned() {
  let run = eval_file(&fixture_path()).expect("path compression discovery fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-path-compression-discovery"
  );
  assert_eq!(
    as_str(get(&run, "truth-owner")),
    "project-wiki/maps/tesseract-macro-ontology-discovery-ledger.md"
  );
}

#[test]
fn old_trajectory_requires_explicit_a_to_b_to_c_chain() {
  let run = eval_file(&fixture_path()).unwrap();
  let old = get(&run, "old-trajectory");
  assert_eq!(as_str(get(old, "style")), "legacy-explicit-chain");
  assert!(as_bool(get(old, "requires-explicit-middle")));
  let path = as_list(get(old, "path"));
  assert_eq!(path.len(), 2);
  assert_eq!(as_str(get(&path[0], "from")), "A");
  assert_eq!(as_str(get(&path[0], "to")), "B");
  assert_eq!(as_str(get(&path[1], "from")), "B");
  assert_eq!(as_str(get(&path[1], "to")), "C");
  assert_eq!(as_str(get_path(old, &["old-missing-case", "missing"])), "B");
}

#[test]
fn macro_experiment_starts_from_a_to_c_and_emits_candidate_b_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let macro_exp = get(&run, "macro-experiment");
  assert_eq!(
    as_str(get(macro_exp, "turn-instance")),
    "turn.forward.A-to-C"
  );
  assert_eq!(as_str(get(macro_exp, "starts-from")), "A");
  assert_eq!(as_str(get(macro_exp, "target")), "C");
  assert_eq!(as_str(get(macro_exp, "direction")), "forward");
  assert_eq!(as_str(get_path(macro_exp, &["input-edge", "from"])), "A");
  assert_eq!(as_str(get_path(macro_exp, &["input-edge", "to"])), "C");
  assert!(!as_bool(get(macro_exp, "direct-proof")));
  assert!(as_bool(get(macro_exp, "candidate-only")));
  assert!(!as_bool(get(macro_exp, "auto-apply-middle")));
  assert_eq!(as_str(get_path(macro_exp, &["inferred-middle", "id"])), "B");
  assert_eq!(
    as_str(get_path(macro_exp, &["inferred-middle", "status"])),
    "candidate"
  );
  assert!(!as_bool(get_path(
    macro_exp,
    &["inferred-middle", "accepted"]
  )));
}

#[test]
fn reverse_turn_starts_from_c_and_is_a_distinct_tesseract_instance() {
  let run = eval_file(&fixture_path()).unwrap();
  let reverse = get(&run, "reverse-turn-experiment");
  assert_eq!(as_str(get(reverse, "turn-instance")), "turn.reverse.C-to-A");
  assert_eq!(as_str(get(reverse, "distinct-from")), "turn.forward.A-to-C");
  assert_eq!(as_str(get(reverse, "starts-from")), "C");
  assert_eq!(as_str(get(reverse, "target")), "A");
  assert_eq!(as_str(get(reverse, "direction")), "reverse");
  assert!(as_bool(get(reverse, "creates-separate-instance")));
  assert!(!as_bool(get(reverse, "direct-proof")));
  assert!(as_bool(get(reverse, "candidate-only")));
  assert!(!as_bool(get(reverse, "auto-apply-middle")));
}

#[test]
fn reverse_turn_emits_candidate_b_without_accepting_it() {
  let run = eval_file(&fixture_path()).unwrap();
  let reverse = get(&run, "reverse-turn-experiment");
  assert_eq!(as_str(get_path(reverse, &["input-edge", "from"])), "C");
  assert_eq!(as_str(get_path(reverse, &["input-edge", "to"])), "A");
  assert_eq!(as_str(get_path(reverse, &["inferred-middle", "id"])), "B");
  assert_eq!(
    as_str(get_path(reverse, &["inferred-middle", "role"])),
    "ReverseMiddleClue"
  );
  assert!(!as_bool(get_path(
    reverse,
    &["inferred-middle", "accepted"]
  )));

  let emitted_by = string_set(get_path(reverse, &["inferred-middle", "emitted-by"]));
  for expected in [
    "reverse-role-pressure",
    "reverse-candidate-ranking",
    "reverse-repair-candidate",
    "reverse-audit-delta",
  ] {
    assert!(
      emitted_by.contains(expected),
      "missing reverse source `{expected}`"
    );
  }
}

#[test]
fn macro_middle_clue_names_emission_sources_and_layers() {
  let run = eval_file(&fixture_path()).unwrap();
  let emitted_by = string_set(get_path(
    &run,
    &["macro-experiment", "inferred-middle", "emitted-by"],
  ));
  for expected in [
    "role-pressure",
    "candidate-ranking",
    "repair-candidate",
    "audit-delta",
  ] {
    assert!(
      emitted_by.contains(expected),
      "missing emission source `{expected}`"
    );
  }

  let layers = get_path(&run, &["macro-experiment", "layers"]);
  for key in [
    "surface", "ontology", "semantic", "gate", "runtime", "audit",
  ] {
    assert!(as_bool(get(layers, key)), "layer `{key}` must stay visible");
  }
}

#[test]
fn trials_cover_candidate_ambiguous_and_missing_middle_cases() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = as_list(get(&run, "trials"));
  assert_eq!(trials.len(), 4);
  let statuses: BTreeSet<&str> = trials
    .iter()
    .map(|trial| as_str(get(trial, "status")))
    .collect();
  assert!(statuses.contains("candidate-middle-found"));
  assert!(statuses.contains("Held"));

  let held_kinds: BTreeSet<&str> = trials
    .iter()
    .filter_map(|trial| as_attrs(trial).get("held-kind").map(as_str))
    .collect();
  assert_eq!(
    held_kinds,
    ["ambiguous-middle", "missing-middle"].into_iter().collect()
  );
  let trial_ids: BTreeSet<&str> = trials
    .iter()
    .map(|trial| as_str(get(trial, "id")))
    .collect();
  assert!(trial_ids.contains("trial.reverse-C-to-A"));
  for trial in trials {
    assert!(!as_bool(get(trial, "accepted")));
  }
}

#[test]
fn discoveries_mark_path_capability_as_scenario_experiment() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = as_list(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 5);
  let ids: BTreeSet<&str> = discoveries.iter().map(|d| as_str(get(d, "id"))).collect();
  assert_eq!(
    ids,
    [
      "D11.compressed-path-can-emit-middle-clue",
      "D12.middle-clue-is-repair-candidate-not-accepted-proof",
      "D13.ambiguity-becomes-held-not-silent-choice",
      "D14.path-capability-must-be-found-by-experiment",
      "D15.reverse-turn-can-create-separate-instance"
    ]
    .into_iter()
    .collect()
  );
  for discovery in discoveries {
    assert!(as_bool(get(discovery, "scenario-only")));
  }
}

#[test]
fn affected_plans_stay_observation_handles() {
  let run = eval_file(&fixture_path()).unwrap();
  let plans = get(&run, "affected-plans");
  for expected in [
    "insertableInference",
    "NeedGraph",
    "NeedCursor",
    "RepairCandidate",
    "routeRanking",
    "reverseTurnInstance",
  ] {
    let entry = get(plans, expected);
    assert_eq!(as_str(get(entry, "role")), "observation-handle");
    assert!(
      !as_bool(get(entry, "implementation-target")),
      "`{expected}` must remain observation-only"
    );
  }
  assert_eq!(
    as_str(get_path(plans, &["insertableInference", "pressure"])),
    "demote-old-plan"
  );
}

#[test]
fn blocked_shortcuts_and_held_conditions_prevent_auto_inference() {
  let run = eval_file(&fixture_path()).unwrap();
  let blocks = string_set(get(&run, "blocked-shortcuts"));
  for expected in [
    "accept-B-without-replay",
    "collapse-ambiguous-middle",
    "treat-A-to-C-as-complete-proof",
    "implement-insertable-inference-before-scenario",
    "drop-old-chain-comparison",
    "reuse-forward-turn-as-reverse-proof",
    "accept-reverse-B-without-replay",
  ] {
    assert!(
      blocks.contains(expected),
      "missing blocked shortcut `{expected}`"
    );
  }

  let held_if = string_set(get_path(&run, &["negative-held-evidence", "held-if"]));
  for expected in [
    "middle-ambiguous",
    "middle-missing",
    "B-replay-missing",
    "old-chain-delta-unnamed",
    "reverse-turn-instance-missing",
    "reverse-middle-ambiguous",
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
