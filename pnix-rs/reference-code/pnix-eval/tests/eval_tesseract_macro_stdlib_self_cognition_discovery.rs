//! Stdlib self-cognition discovery.
//!
//! The tesseract macro must be able to inspect stdlib as a knowledge substrate
//! without turning stdlib imports or legacy ontology-backed constructors into
//! current semantic authority.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base)
    .join("../../fixtures/tesseract-macro-legacy-probe/stdlib_self_cognition_discovery_receipt.px")
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
fn stdlib_self_cognition_marker_and_source_are_pinned() {
  let run = eval_file(&fixture_path()).expect("stdlib self-cognition fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-stdlib-self-cognition"
  );
  assert_eq!(
    as_str(get(&run, "source-owner")),
    "stdlib/lib/gate/recipe-match.px"
  );
}

#[test]
fn stdlib_surface_records_export_and_meta_shape() {
  let run = eval_file(&fixture_path()).unwrap();
  let surface = get(&run, "stdlib-surface");
  assert_eq!(
    as_str(get(surface, "owner_path")),
    "stdlib/lib/gate/recipe-match.px"
  );
  assert_eq!(as_str(get(surface, "owner_kind")), "gate-stdlib");
  assert_eq!(
    list_strings(get(surface, "exported_symbols")),
    vec![
      "defaultPolicy",
      "normalizeSignature",
      "mkRecipeMatcher",
      "mapRecipeMatchers",
      "selectBestMatch"
    ]
  );
  assert_eq!(
    as_str(get(surface, "declared_constructor")),
    "mkRecipeMatcher"
  );
  assert_eq!(
    as_str(get(surface, "declared_evaluation_shape")),
    "ontologyLift -> ontologyEvaluate -> ontologySelect over builtins.map recipes"
  );
}

#[test]
fn pure_owner_helper_can_normalize_one_tool_surface() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get_path(&run, &["normalized-tool-surface", "tool_name"])),
    "apply_patch"
  );
  assert_eq!(
    as_str(get_path(&run, &["normalized-tool-surface", "context"])),
    "edit-scope"
  );
  assert_eq!(
    list_strings(get_path(
      &run,
      &["normalized-tool-surface", "arg_predicates"]
    )),
    vec!["external-path", "worktree-scope"]
  );
}

#[test]
fn stdlib_self_fold_keeps_authority_outside_stdlib() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "stdlib-self-fold");
  assert_eq!(as_str(get(fold, "mode")), "stdlib-self-cognition");
  assert!(!as_bool(get(fold, "comparison-peer-required")));
  assert!(!as_bool(get(fold, "old-vs-new-comparison")));
  assert!(as_bool(get(fold, "can-self-cognize")));
  assert!(as_bool(get(fold, "stdlib-as-cognition-target")));
  assert!(!as_bool(get(fold, "stdlib-as-authority-owner")));
  assert!(!as_bool(get(fold, "stdlib-as-runtime-install-source")));
  assert!(!as_bool(get(fold, "owner-law-stdlib-lift")));
  assert!(as_bool(get(fold, "owner-law-external")));
  assert!(!as_bool(get(fold, "canonical-stdlib-owner-edited")));
}

#[test]
fn stdlib_self_fold_keeps_all_six_layers_visible() {
  let run = eval_file(&fixture_path()).unwrap();
  let layers = get_path(&run, &["stdlib-self-fold", "layers"]);
  for key in [
    "surface", "ontology", "semantic", "gate", "runtime", "audit",
  ] {
    assert!(as_bool(get(layers, key)), "layer `{key}` must stay visible");
  }
}

#[test]
fn stdlib_capability_split_separates_pure_helper_from_legacy_constructors() {
  let run = eval_file(&fixture_path()).unwrap();
  let split = get(&run, "stdlib-capability-split");
  let pure = as_list(get(split, "pure_helpers"));
  assert_eq!(pure.len(), 1);
  assert_eq!(
    as_str(get(&pure[0], "id")),
    "stdlib.recipe-match.normalizeSignature"
  );
  assert_eq!(as_str(get(&pure[0], "status")), "usable-as-pure-helper");
  assert!(!as_bool(get(&pure[0], "authority")));

  let legacy = as_list(get(split, "legacy_ontology_backed_constructors"));
  assert_eq!(legacy.len(), 3);
  let symbols: BTreeSet<&str> = legacy
    .iter()
    .map(|constructor| as_str(get(constructor, "symbol")))
    .collect();
  assert_eq!(
    symbols,
    ["mkRecipeMatcher", "mapRecipeMatchers", "selectBestMatch"]
      .into_iter()
      .collect()
  );
  for constructor in legacy {
    assert_eq!(as_str(get(constructor, "status")), "reference-specimen");
    assert!(!as_bool(get(constructor, "authority")));
  }
}

#[test]
fn stdlib_self_cognition_emits_need_candidates_and_authority_held() {
  let run = eval_file(&fixture_path()).unwrap();
  let candidates = as_list(get(&run, "self-observed-candidates"));
  assert_eq!(candidates.len(), 4);
  let ids: BTreeSet<&str> = candidates
    .iter()
    .map(|candidate| as_str(get(candidate, "id")))
    .collect();
  assert_eq!(
    ids,
    [
      "held.self.stdlib-authority-lift",
      "need.self.stdlib-export-contract",
      "need.self.stdlib-proof-surface",
      "need.self.stdlib-purity-boundary"
    ]
    .into_iter()
    .collect()
  );
  for candidate in candidates {
    assert!(!as_bool(get(candidate, "accepted")));
  }
}

#[test]
fn stdlib_runtime_observation_is_candidate_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let runtime = get(&run, "self-runtime-observation");
  assert_eq!(
    as_str(get(runtime, "observation-model")),
    "stdlib-self-meta-circular-mirror"
  );
  assert!(as_bool(get(runtime, "can-appear-at-runtime")));
  assert!(!as_bool(get(runtime, "canonical-runtime-installed")));
  assert!(!as_bool(get(runtime, "executable-now")));
  assert!(as_bool(get(runtime, "candidate-only")));
  assert!(!as_bool(get(runtime, "owner-switch")));

  let runtime_candidates = as_list(get(runtime, "runtime-added-candidates"));
  assert_eq!(runtime_candidates.len(), 4);
  for candidate in runtime_candidates {
    assert_eq!(as_str(get(candidate, "status")), "candidate");
    assert!(!as_bool(get(candidate, "installed")));
  }
}

#[test]
fn stdlib_discoveries_and_blocks_prevent_authority_shortcut() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = as_list(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 4);
  let discovery_ids: BTreeSet<&str> = discoveries
    .iter()
    .map(|discovery| as_str(get(discovery, "id")))
    .collect();
  assert_eq!(
    discovery_ids,
    [
      "D26.stdlib-can-be-self-cognized",
      "D27.stdlib-symbols-split-by-purity-and-legacy-shape",
      "D28.owner-law-stdlib-lift-stays-blocked",
      "D29.self-cognition-extends-to-language-and-stdlib"
    ]
    .into_iter()
    .collect()
  );
  for discovery in discoveries {
    assert!(as_bool(get(discovery, "scenario-only")));
  }

  let blocks = string_set(get(&run, "blocked-shortcuts"));
  for expected in [
    "treat-stdlib-import-as-owner-switch",
    "promote-old-ontology-backed-stdlib-constructor",
    "install-stdlib-runtime-candidate",
    "erase-purity-boundary",
    "edit-canonical-stdlib-owner-as-experiment",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }

  let held_if = string_set(get_path(&run, &["negative-held-evidence", "held-if"]));
  for expected in [
    "stdlib-export-contract-missing",
    "stdlib-purity-boundary-unproven",
    "consumer-proof-not-replayed",
    "owner-law-lift-requested",
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
