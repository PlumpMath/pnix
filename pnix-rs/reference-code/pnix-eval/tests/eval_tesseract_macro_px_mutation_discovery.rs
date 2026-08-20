//! Repeated .px mutation discovery over fixture-local variants.
//!
//! The canonical old .px owner stays untouched; copied variants are evaled to
//! learn which macro roles or Held states appear.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base)
    .join("../../fixtures/tesseract-macro-legacy-probe/px_mutation_discovery_receipt.px")
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
  as_list(v).iter().map(|item| as_str(item)).collect()
}

fn string_set(v: &Value) -> BTreeSet<&str> {
  list_strings(v).into_iter().collect()
}

fn trial_by_id<'a>(run: &'a Value, id: &str) -> &'a Value {
  as_list(get(run, "trials"))
    .iter()
    .find(|trial| as_str(get(trial, "id")) == id)
    .unwrap_or_else(|| panic!("missing trial `{id}`"))
}

#[test]
fn mutation_receipt_marker_and_policy_are_pinned() {
  let run = eval_file(&fixture_path()).expect("px mutation discovery fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-px-mutation-discovery"
  );
  assert_eq!(
    as_str(get(&run, "source-owner")),
    "stdlib/lib/gate/recipe-match.px"
  );
  assert!(!as_bool(get_path(
    &run,
    &["mutation-policy", "canonical-old-owner-edited"]
  )));
  assert!(as_bool(get_path(
    &run,
    &["mutation-policy", "fixture-local-variants"]
  )));
  assert_eq!(
    as_str(get_path(&run, &["mutation-policy", "old-owner-role"])),
    "reference-specimen"
  );

  assert_eq!(
    as_str(get_path(&run, &["function-expansion", "old-px-operation"])),
    "recipe-match missing predicate evaluation"
  );
  assert_eq!(
    as_i64(get_path(
      &run,
      &["function-expansion", "old-operation-count"]
    )),
    1
  );
  assert_eq!(
    as_i64(get_path(
      &run,
      &["function-expansion", "macro-function-count"]
    )),
    3
  );
  let functions = string_set(get_path(&run, &["function-expansion", "macro-functions"]));
  for expected in [
    "forward-middle-clue-emission",
    "reverse-turn-middle-clue-emission",
    "held-ambiguity-and-drift-classification",
  ] {
    assert!(
      functions.contains(expected),
      "missing expanded macro function `{expected}`"
    );
  }
  assert!(!as_bool(get_path(
    &run,
    &["function-expansion", "implementation-command"]
  )));
}

#[test]
fn runtime_addition_observation_stays_candidate_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let runtime = get(&run, "runtime-addition-observation");
  assert_eq!(
    as_str(get(runtime, "observation-model")),
    "tesseract-world-observes-foreign-runtime-world"
  );
  assert_eq!(
    as_str(get(runtime, "observation-metaphor")),
    "meta-circular-mirror"
  );
  assert_eq!(
    as_str(get(runtime, "observer-world")),
    "meta-circular-tesseract-macro"
  );
  assert_eq!(
    as_str(get(runtime, "observed-world")),
    "fixture-local-old-px-variant-world"
  );
  assert!(as_bool(get(runtime, "can-appear-at-runtime")));
  assert!(!as_bool(get(runtime, "canonical-runtime-installed")));
  assert!(!as_bool(get(runtime, "executable-now")));
  assert!(as_bool(get(runtime, "candidate-only")));
  assert!(!as_bool(get(runtime, "owner-switch")));

  let required = string_set(get(runtime, "required-before-install"));
  for expected in [
    "role-emission-verdict",
    "runtime-route-proof",
    "reverse-replay",
    "negative-held-proof",
    "R6-owner-switch-receipt",
  ] {
    assert!(
      required.contains(expected),
      "missing runtime install gate `{expected}`"
    );
  }

  let candidates = as_list(get(runtime, "runtime-added-candidates"));
  assert_eq!(candidates.len(), 3);
  let ids: BTreeSet<&str> = candidates
    .iter()
    .map(|candidate| as_str(get(candidate, "id")))
    .collect();
  assert_eq!(
    ids,
    [
      "runtime.forward-middle-clue-emission",
      "runtime.reverse-turn-middle-clue-emission",
      "runtime.held-ambiguity-and-drift-classification"
    ]
    .into_iter()
    .collect()
  );
  for candidate in candidates {
    assert_eq!(as_str(get(candidate, "status")), "candidate");
    assert!(!as_bool(get(candidate, "installed")));
  }
}

#[test]
fn base_signature_starts_with_missing_px_target() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    list_strings(get_path(&run, &["base-tool", "arg_predicates"])),
    vec!["external-path", "worktree-scope"]
  );
  assert_eq!(
    list_strings(get_path(&run, &["base-recipe", "arg_predicates"])),
    vec!["external-path", "worktree-scope", "px-target"]
  );
}

#[test]
fn baseline_missing_trial_emits_reverse_middle_candidate() {
  let run = eval_file(&fixture_path()).unwrap();
  let trial = trial_by_id(&run, "trial.baseline-missing-px-target");
  assert_eq!(as_str(get(trial, "status")), "candidate-middle-found");
  assert!(!as_bool(get(trial, "held")));
  assert_eq!(
    list_strings(get(trial, "missing-predicates")),
    vec!["px-target"]
  );
  assert_eq!(as_i64(get(trial, "predicate-miss-total")), 1);
  assert_eq!(
    as_str(get_path(trial, &["reverse-turn", "inferred-middle", "id"])),
    "px-target"
  );
  assert_eq!(
    as_str(get_path(
      trial,
      &["reverse-turn", "inferred-middle", "role"]
    )),
    "ReverseMissingPredicateClue"
  );
}

#[test]
fn adding_px_target_resolves_middle_pressure_without_owner_switch() {
  let run = eval_file(&fixture_path()).unwrap();
  let trial = trial_by_id(&run, "trial.add-px-target-to-tool");
  assert_eq!(as_str(get(trial, "status")), "candidate-resolved");
  assert!(as_list(get(trial, "missing-predicates")).is_empty());
  assert_eq!(as_i64(get(trial, "predicate-miss-total")), 0);
  assert!(matches!(
    get_path(trial, &["reverse-turn", "inferred-middle"]),
    Value::Null
  ));
  assert!(as_bool(get(trial, "candidate-only")));
  assert!(!as_bool(get(trial, "owner-switch")));
}

#[test]
fn multiple_missing_predicates_become_held_ambiguity() {
  let run = eval_file(&fixture_path()).unwrap();
  let trial = trial_by_id(&run, "trial.recipe-adds-two-missing-predicates");
  assert_eq!(as_str(get(trial, "status")), "Held");
  assert!(as_bool(get(trial, "held")));
  assert_eq!(
    as_str(get(trial, "held-kind")),
    "ambiguous-missing-predicate"
  );
  assert_eq!(
    list_strings(get(trial, "missing-predicates")),
    vec!["px-target", "path-target"]
  );
  assert!(matches!(
    get_path(trial, &["reverse-turn", "inferred-middle"]),
    Value::Null
  ));
}

#[test]
fn context_drift_becomes_held_before_ranking() {
  let run = eval_file(&fixture_path()).unwrap();
  let trial = trial_by_id(&run, "trial.recipe-context-drift");
  assert_eq!(as_str(get(trial, "status")), "Held");
  assert!(as_bool(get(trial, "held")));
  assert_eq!(as_str(get(trial, "held-kind")), "context-mismatch");
  assert_eq!(
    as_str(get_path(trial, &["recipe", "context"])),
    "runtime-scope"
  );
  assert_eq!(
    list_strings(get(trial, "missing-predicates")),
    vec!["px-target"]
  );
}

#[test]
fn discoveries_record_mutation_loop_effects() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = as_list(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 4);
  let ids: BTreeSet<&str> = discoveries.iter().map(|d| as_str(get(d, "id"))).collect();
  assert_eq!(
    ids,
    [
      "D19.fixture-local-px-mutation-loop-is-safe-discovery-method",
      "D20.resolved-middle-removes-reverse-clue-pressure",
      "D21.multiple-missing-predicates-become-held-ambiguity",
      "D22.context-drift-becomes-held-before-repair-ranking"
    ]
    .into_iter()
    .collect()
  );
  for discovery in discoveries {
    assert!(as_bool(get(discovery, "scenario-only")));
  }
}

#[test]
fn blocked_shortcuts_keep_mutation_loop_non_authoritative() {
  let run = eval_file(&fixture_path()).unwrap();
  let blocks = string_set(get(&run, "blocked-shortcuts"));
  for expected in [
    "edit-canonical-old-px-as-experiment",
    "accept-green-variant-as-owner-switch",
    "hide-ambiguous-missing-predicate",
    "rank-context-drift-before-held",
    "drop-reference-baseline",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }

  let held_if = string_set(get_path(&run, &["negative-held-evidence", "held-if"]));
  for expected in [
    "multiple-missing-predicates",
    "context-mismatch",
    "tool-name-mismatch",
    "canonical-old-owner-edited",
    "reference-baseline-missing",
  ] {
    assert!(
      held_if.contains(expected),
      "missing Held condition `{expected}`"
    );
  }
  assert_eq!(as_str(get(&run, "replacement-readiness")), "not-proven");
  assert!(!as_bool(get(&run, "owner-switch")));
  assert!(!as_bool(get(&run, "implementation-command")));
}
