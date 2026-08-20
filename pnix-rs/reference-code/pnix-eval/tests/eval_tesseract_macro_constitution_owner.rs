//! Tesseract constitution owner proof.
//!
//! The constitution gate must be owned by a .px surface. Project-wiki can route
//! sessions to it, but the gate shape itself has to be emitted by the owner.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base)
    .join("../../fixtures/tesseract-macro-legacy-probe/tesseract_constitution_owner_receipt.px")
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
fn constitution_owner_marker_and_import_are_pinned() {
  let run = eval_file(&fixture_path()).expect("constitution owner fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-constitution-owner"
  );
  assert!(as_bool(get(&run, "used-px-owner")));
  assert_eq!(
    as_str(get(&run, "truth-owner")),
    "stdlib/lib/gate/tesseract-constitution.px"
  );
  assert_eq!(
    as_str(get(&run, "imported-owner")),
    "stdlib/lib/gate/tesseract-constitution.px"
  );
}

#[test]
fn owner_meta_declares_constructor_and_non_runtime_role() {
  let run = eval_file(&fixture_path()).unwrap();
  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib/lib/gate/tesseract-constitution.px"
  );
  assert_eq!(as_str(get(meta, "constructor")), "constitutionGate");
  assert_eq!(
    as_str(get(meta, "base")),
    "meta-circular-tesseract-macro-ontology"
  );
  assert!(
    as_str(get(meta, "rev_note")).contains("not an ontology runtime"),
    "meta rev_note must reject runtime ownership"
  );
}

#[test]
fn safe_gate_keeps_constitution_shape_candidate_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "safe-gate");
  assert_eq!(
    as_str(get(gate, "artifact_family")),
    "tesseract.macro.constitution-gate"
  );
  assert_eq!(
    as_str(get(gate, "base")),
    "meta-circular-tesseract-macro-ontology"
  );
  assert_eq!(as_str(get(gate, "role")), "constitution-gate");
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  assert_eq!(as_str(get(gate, "replacement-readiness")), "not-proven");
}

#[test]
fn safe_gate_preserves_all_six_layers_and_required_boundaries() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "safe-gate");
  let layers = get(gate, "layers");
  for key in [
    "surface", "ontology", "semantic", "gate", "runtime", "audit",
  ] {
    assert!(as_bool(get(layers, key)), "layer `{key}` must stay visible");
  }
  assert!(as_bool(get(gate, "all-layers-visible")));

  let boundaries = string_set(get(gate, "required-boundaries"));
  for expected in [
    "candidate-only",
    "owner-law-external",
    "no-policy-mutation",
    "no-runtime-install",
    "replay-required-before-accept",
    "negative-held-proof-required",
  ] {
    assert!(
      boundaries.contains(expected),
      "missing boundary `{expected}`"
    );
  }
}

#[test]
fn safe_gate_includes_default_and_payload_substrate_owners() {
  let run = eval_file(&fixture_path()).unwrap();
  let owners = string_set(get_path(&run, &["safe-gate", "substrate-owner-surfaces"]));
  for expected in [
    "macro.md",
    "project-wiki/maps/minimal-ontology-tesseract-v0-map.md",
    "stdlib/lib/gate/tesseract-constitution.px",
    "stdlib/lib/gate/self-optimization.px",
    "stdlib/lib/gate/learning-progress.px",
  ] {
    assert!(
      owners.contains(expected),
      "missing substrate owner `{expected}`"
    );
  }
}

#[test]
fn blocked_gate_turns_shortcuts_into_held_without_acceptance() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "blocked-gate");
  assert_eq!(as_str(get(gate, "verdict")), "Held");
  assert!(!as_bool(get(gate, "accepted")));
  assert!(as_bool(get(gate, "standalone-learning-engine")));
  assert!(as_bool(get(gate, "old-ontology-authority")));
  assert!(!as_bool(get(gate, "candidate-only")));
  assert!(!as_bool(get(gate, "owner-law-external")));
  assert!(as_bool(get(gate, "policy-mutation-applied")));
  assert!(as_bool(get(gate, "runtime-installed")));
  assert!(as_bool(get(gate, "owner-switch")));
  assert!(as_bool(get(gate, "implementation-command")));
  assert!(!as_bool(get(gate, "all-layers-visible")));
}

#[test]
fn blocked_shortcuts_include_default_and_payload_blocks() {
  let run = eval_file(&fixture_path()).unwrap();
  let blocks = string_set(get_path(&run, &["blocked-gate", "blocked-shortcuts"]));
  for expected in [
    "skip-tesseract-constitution-gate",
    "treat-project-wiki-as-runtime-truth-store",
    "treat-existing-px-owner-as-standalone-engine",
    "mutate-policy-without-owner-proof",
    "install-runtime-candidate-without-replay",
    "promote-held-or-need-without-negative-proof",
    "test-shortcut-owner-switch",
    "test-shortcut-runtime-install",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
}

#[test]
fn held_conditions_include_default_and_payload_conditions() {
  let run = eval_file(&fixture_path()).unwrap();
  let held_if = string_set(get_path(&run, &["blocked-gate", "held-if"]));
  for expected in [
    "owner-proof-missing",
    "before-after-proof-missing",
    "negative-held-proof-missing",
    "runtime-route-proof-missing",
    "owner-law-gate-not-closed",
    "test-held-shortcut-requested",
  ] {
    assert!(
      held_if.contains(expected),
      "missing Held condition `{expected}`"
    );
  }
}

#[test]
fn project_wiki_remains_development_map_not_runtime_truth() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "project-wiki-role")),
    "development-reference-map"
  );
  assert!(!as_bool(get(&run, "runtime-truth-store")));
  assert_eq!(as_str(get(&run, "replacement-readiness")), "not-proven");
  assert!(!as_bool(get(&run, "owner-switch")));
  assert!(!as_bool(get(&run, "implementation-command")));

  let discoveries = as_list(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 3);
  let ids: BTreeSet<&str> = discoveries
    .iter()
    .map(|discovery| as_str(get(discovery, "id")))
    .collect();
  for expected in [
    "D35.tesseract-constitution-owner-surface-landed",
    "D36.constitution-owner-blocks-authority-and-runtime-shortcuts",
    "D37.project-wiki-remains-development-map-not-runtime-authority",
  ] {
    assert!(ids.contains(expected), "missing discovery `{expected}`");
  }
}
