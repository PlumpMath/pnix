//! Concrete reverse-turn example using the recipe-match owner surface.
//!
//! A = request/tool signature, C = recipe-match target, B = missing predicate.
//! The reverse turn starts from C and emits B as a candidate clue only.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join(
    "../../fixtures/tesseract-macro-legacy-probe/recipe_match_reverse_turn_discovery_receipt.px",
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
  as_list(v).iter().map(|item| as_str(item)).collect()
}

fn string_set(v: &Value) -> BTreeSet<&str> {
  list_strings(v).into_iter().collect()
}

#[test]
fn recipe_match_reverse_example_marker_and_owner_are_pinned() {
  let run = eval_file(&fixture_path()).expect("recipe-match reverse fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-recipe-match-reverse-turn-discovery"
  );
  assert_eq!(
    as_str(get(&run, "source-owner")),
    "stdlib/lib/gate/recipe-match.px"
  );
  assert_eq!(
    as_str(get_path(&run, &["source-owner-meta", "constructor"])),
    "mkRecipeMatcher"
  );
  assert_eq!(
    as_str(get_path(&run, &["source-owner-meta", "evaluation-shape"])),
    "ontologyLift -> ontologyEvaluate -> ontologySelect over builtins.map recipes"
  );
}

#[test]
fn concrete_meaning_maps_a_b_c_to_recipe_match_domain() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get_path(&run, &["example-meaning", "A"])),
    "tool-signature.apply_patch.edit-scope"
  );
  assert_eq!(
    as_str(get_path(&run, &["example-meaning", "B"])),
    "missing-predicate.px-target"
  );
  assert_eq!(
    as_str(get_path(&run, &["example-meaning", "C"])),
    "recipe-match.External-px-path-outside-worktree-scope"
  );
  assert_eq!(
    as_str(get_path(&run, &["example-meaning", "concrete-domain"])),
    "pnix gate repair recipe matching"
  );
}

#[test]
fn pure_owner_normalization_computes_missing_px_target() {
  let run = eval_file(&fixture_path()).unwrap();
  let comparison = get(&run, "comparison");
  assert_eq!(
    list_strings(get_path(comparison, &["normalized-tool", "arg_predicates"])),
    vec!["external-path", "worktree-scope"]
  );
  assert_eq!(
    list_strings(get_path(
      comparison,
      &["normalized-recipe", "arg_predicates"]
    )),
    vec!["external-path", "worktree-scope", "px-target"]
  );
  assert_eq!(
    list_strings(get(comparison, "matched-predicates")),
    vec!["external-path", "worktree-scope"]
  );
  assert_eq!(
    list_strings(get(comparison, "missing-predicates")),
    vec!["px-target"]
  );
  assert!(as_list(get(comparison, "extra-predicates")).is_empty());
  assert_eq!(as_i64(get(comparison, "predicate-hit-total")), 2);
  assert_eq!(as_i64(get(comparison, "predicate-miss-total")), 1);
  assert_eq!(
    as_str(get(comparison, "pure-owner-helper-used")),
    "normalizeSignature"
  );
}

#[test]
fn old_explicit_chain_names_b_as_required_middle_without_calling_old_builtins() {
  let run = eval_file(&fixture_path()).unwrap();
  let old = get(&run, "old-explicit-chain");
  assert_eq!(
    as_str(get(old, "source-owner")),
    "stdlib/lib/gate/recipe-match.px"
  );
  assert_eq!(as_str(get(old, "B")), "missing-predicate.px-target");
  assert!(as_bool(get(old, "requires-explicit-middle")));
  assert!(!as_bool(get(old, "old-builtins-called")));
  assert_eq!(
    as_str(get(old, "old-builtins-call-policy")),
    "reference-specimen-only"
  );

  let chain = as_list(get(old, "chain"));
  assert_eq!(chain.len(), 2);
  assert_eq!(as_str(get(&chain[0], "to")), "missing-predicate.px-target");
  assert_eq!(
    as_str(get(&chain[1], "from")),
    "missing-predicate.px-target"
  );
}

#[test]
fn forward_turn_emits_missing_predicate_candidate_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let forward = get(&run, "macro-forward-turn");
  assert_eq!(
    as_str(get(forward, "turn-instance")),
    "turn.forward.tool-to-recipe-target"
  );
  assert_eq!(as_str(get(forward, "direction")), "forward");
  assert!(!as_bool(get(forward, "direct-proof")));
  assert!(as_bool(get(forward, "candidate-only")));
  assert!(!as_bool(get(forward, "auto-apply-middle")));
  assert_eq!(
    as_str(get_path(forward, &["inferred-middle", "id"])),
    "px-target"
  );
  assert_eq!(
    as_str(get_path(forward, &["inferred-middle", "role"])),
    "MissingPredicateClue"
  );
  assert!(!as_bool(get_path(
    forward,
    &["inferred-middle", "accepted"]
  )));
}

#[test]
fn reverse_turn_starts_from_recipe_target_and_emits_reverse_missing_predicate_clue() {
  let run = eval_file(&fixture_path()).unwrap();
  let reverse = get(&run, "reverse-turn");
  assert_eq!(
    as_str(get(reverse, "turn-instance")),
    "turn.reverse.recipe-target-to-tool"
  );
  assert_eq!(
    as_str(get(reverse, "distinct-from")),
    "turn.forward.tool-to-recipe-target"
  );
  assert_eq!(
    as_str(get(reverse, "starts-from")),
    "recipe-match.External-px-path-outside-worktree-scope"
  );
  assert_eq!(
    as_str(get(reverse, "target")),
    "tool-signature.apply_patch.edit-scope"
  );
  assert_eq!(as_str(get(reverse, "direction")), "reverse");
  assert!(as_bool(get(reverse, "creates-separate-instance")));
  assert!(as_bool(get(reverse, "candidate-only")));
  assert!(!as_bool(get(reverse, "direct-proof")));
  assert_eq!(
    as_str(get_path(reverse, &["inferred-middle", "role"])),
    "ReverseMissingPredicateClue"
  );
  assert_eq!(
    as_str(get_path(reverse, &["inferred-middle", "id"])),
    "px-target"
  );
  assert!(!as_bool(get_path(
    reverse,
    &["inferred-middle", "accepted"]
  )));
}

#[test]
fn discoveries_and_affected_plans_stay_design_candidates() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = as_list(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 3);
  let ids: BTreeSet<&str> = discoveries.iter().map(|d| as_str(get(d, "id"))).collect();
  assert_eq!(
    ids,
    [
      "D16.recipe-match-can-supply-concrete-middle-clue",
      "D17.recipe-target-reverse-turn-can-start-from-C",
      "D18.recipe-match-example-keeps-old-builtins-as-specimen"
    ]
    .into_iter()
    .collect()
  );
  for discovery in discoveries {
    assert!(as_bool(get(discovery, "scenario-only")));
  }

  let plans = get(&run, "affected-plans");
  for expected in [
    "recipeMatchOwner",
    "RepairCandidate",
    "NeedGraph",
    "NeedCursor",
    "routeRanking",
  ] {
    let entry = get(plans, expected);
    assert!(
      !as_bool(get(entry, "implementation-target")),
      "`{expected}` must remain non-implementation"
    );
  }
}

#[test]
fn blocked_shortcuts_and_held_conditions_keep_example_from_becoming_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  let blocks = string_set(get(&run, "blocked-shortcuts"));
  for expected in [
    "call-old-ontology-select-as-authority",
    "accept-missing-predicate-without-replay",
    "reuse-forward-turn-as-reverse-proof",
    "auto-apply-repair-recipe",
    "drop-recipe-provenance",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }

  let held_if = string_set(get_path(&run, &["negative-held-evidence", "held-if"]));
  for expected in [
    "missing-predicate-ambiguous",
    "reverse-turn-instance-missing",
    "recipe-provenance-missing",
    "old-builtins-used-as-proof",
    "repair-replay-missing",
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
