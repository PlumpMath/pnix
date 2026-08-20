//! Single-surface self-cognition discovery.
//!
//! The tesseract macro must be able to observe one surface by itself. Pairwise
//! comparison is a useful discovery method, not a semantic requirement.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join(
    "../../fixtures/tesseract-macro-legacy-probe/single_surface_self_cognition_discovery_receipt.px",
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
  as_list(v).iter().map(|item| as_str(item)).collect()
}

fn string_set(v: &Value) -> BTreeSet<&str> {
  list_strings(v).into_iter().collect()
}

#[test]
fn self_cognition_marker_and_source_owner_are_pinned() {
  let run = eval_file(&fixture_path()).expect("single-surface fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-single-surface-self-cognition"
  );
  assert_eq!(
    as_str(get(&run, "source-owner")),
    "stdlib/lib/gate/recipe-match.px"
  );
}

#[test]
fn normalized_single_surface_uses_pure_owner_helper() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get_path(&run, &["normalized-surface", "tool_name"])),
    "apply_patch"
  );
  assert_eq!(
    as_str(get_path(&run, &["normalized-surface", "context"])),
    "edit-scope"
  );
  assert_eq!(
    list_strings(get_path(&run, &["normalized-surface", "arg_predicates"])),
    vec!["external-path", "worktree-scope"]
  );
  assert_eq!(
    as_str(get_path(&run, &["self-fold", "pure-owner-helper-used"])),
    "normalizeSignature"
  );
}

#[test]
fn self_fold_does_not_require_pairwise_comparison() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "self-fold");
  assert_eq!(as_str(get(fold, "mode")), "single-surface-self-cognition");
  assert!(!as_bool(get(fold, "comparison-peer-required")));
  assert!(!as_bool(get(fold, "old-vs-new-comparison")));
  assert!(as_bool(get(fold, "can-self-cognize")));
  assert!(!as_bool(get(fold, "canonical-old-owner-edited")));
}

#[test]
fn self_fold_keeps_all_six_layers_visible() {
  let run = eval_file(&fixture_path()).unwrap();
  let layers = get_path(&run, &["self-fold", "layers"]);
  for key in [
    "surface", "ontology", "semantic", "gate", "runtime", "audit",
  ] {
    assert!(as_bool(get(layers, key)), "layer `{key}` must stay visible");
  }
}

#[test]
fn one_surface_emits_self_needs_and_self_held_candidate() {
  let run = eval_file(&fixture_path()).unwrap();
  let candidates = as_list(get(&run, "self-observed-candidates"));
  assert_eq!(candidates.len(), 3);
  let ids: BTreeSet<&str> = candidates
    .iter()
    .map(|candidate| as_str(get(candidate, "id")))
    .collect();
  assert_eq!(
    ids,
    [
      "held.self.incomplete-edit-surface",
      "need.self.edit-target",
      "need.self.provenance"
    ]
    .into_iter()
    .collect()
  );
  for candidate in candidates {
    assert!(!as_bool(get(candidate, "accepted")));
  }
}

#[test]
fn self_runtime_observation_is_candidate_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let runtime = get(&run, "self-runtime-observation");
  assert_eq!(
    as_str(get(runtime, "observation-model")),
    "single-surface-meta-circular-mirror"
  );
  assert!(as_bool(get(runtime, "can-appear-at-runtime")));
  assert!(!as_bool(get(runtime, "canonical-runtime-installed")));
  assert!(!as_bool(get(runtime, "executable-now")));
  assert!(as_bool(get(runtime, "candidate-only")));
  assert!(!as_bool(get(runtime, "owner-switch")));

  let candidates = as_list(get(runtime, "runtime-added-candidates"));
  assert_eq!(candidates.len(), 3);
  for candidate in candidates {
    assert_eq!(as_str(get(candidate, "status")), "candidate");
    assert!(!as_bool(get(candidate, "installed")));
  }
}

#[test]
fn discoveries_record_comparison_as_method_not_requirement() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = as_list(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 3);
  let ids: BTreeSet<&str> = discoveries
    .iter()
    .map(|discovery| as_str(get(discovery, "id")))
    .collect();
  assert_eq!(
    ids,
    [
      "D23.single-surface-can-self-cognize",
      "D24.single-surface-can-emit-self-needs-and-held",
      "D25.comparison-is-method-not-semantic-requirement"
    ]
    .into_iter()
    .collect()
  );
  for discovery in discoveries {
    assert!(as_bool(get(discovery, "scenario-only")));
  }
}

#[test]
fn blocked_shortcuts_prevent_single_surface_from_becoming_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  let blocks = string_set(get(&run, "blocked-shortcuts"));
  for expected in [
    "require-two-surfaces-for-cognition",
    "accept-self-need-without-replay",
    "install-self-runtime-candidate",
    "drop-held-from-single-surface",
    "edit-canonical-old-px-as-experiment",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }

  let held_if = string_set(get_path(&run, &["negative-held-evidence", "held-if"]));
  for expected in [
    "provenance-missing",
    "edit-target-missing",
    "self-runtime-route-unproven",
    "single-surface-replay-missing",
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
