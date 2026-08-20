//! Discovery receipt for emergent A->B clue behavior.
//!
//! This proves why old planned ontology features may remain non-targets:
//! the tesseract macro can emit clue material that the old ontology did not.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base)
    .join("../../fixtures/tesseract-macro-legacy-probe/emergent_clue_discovery_receipt.px")
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
fn emergent_clue_marker_and_truth_owner_are_pinned() {
  let run = eval_file(&fixture_path()).expect("emergent clue fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-emergent-clue-discovery"
  );
  assert_eq!(
    as_str(get(&run, "truth-owner")),
    "project-wiki/maps/tesseract-macro-ontology-discovery-ledger.md"
  );
}

#[test]
fn old_gap_records_missing_a_to_b_information() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(as_str(get_path(&run, &["old-gap", "missing-edge"])), "A->B");
  assert_eq!(
    as_str(get_path(&run, &["old-gap", "old-ontology-output"])),
    "no-link-or-no-clue"
  );
  let old_pressure = string_set(get_path(&run, &["old-gap", "old-design-pressure"]));
  for expected in [
    "NeedGraph",
    "NeedCursor",
    "insertable-inference",
    "route-cache",
    "retrieval-time-inference",
  ] {
    assert!(
      old_pressure.contains(expected),
      "missing old pressure `{expected}`"
    );
  }
}

#[test]
fn macro_observation_emits_clues_without_claiming_direct_proof() {
  let run = eval_file(&fixture_path()).unwrap();
  assert!(!as_bool(get_path(
    &run,
    &["macro-observation", "direct-edge-proof"]
  )));
  assert!(as_bool(get_path(
    &run,
    &["macro-observation", "clue-emitted"]
  )));
  assert!(as_bool(get_path(
    &run,
    &["macro-observation", "candidate-only"]
  )));
  assert!(as_bool(get_path(
    &run,
    &["macro-observation", "built-in-capability-candidate"]
  )));
  assert_eq!(
    as_str(get_path(
      &run,
      &["macro-observation", "replacement-readiness"]
    )),
    "not-proven"
  );
  assert!(!as_bool(get_path(
    &run,
    &["macro-observation", "owner-switch"]
  )));
}

#[test]
fn emitted_clue_chain_preserves_layers_and_candidate_confidence() {
  let run = eval_file(&fixture_path()).unwrap();
  let clues = as_list(get_path(&run, &["macro-observation", "emitted-clues"]));
  assert_eq!(clues.len(), 3);
  let layers: BTreeSet<&str> = clues.iter().map(|c| as_str(get(c, "layer"))).collect();
  assert_eq!(
    layers,
    ["audit", "ontology", "runtime"].into_iter().collect()
  );
  for clue in clues {
    assert_eq!(as_str(get(clue, "confidence")), "candidate");
  }
}

#[test]
fn macro_default_functions_explain_why_old_design_may_be_redundant() {
  let run = eval_file(&fixture_path()).unwrap();
  let defaults = string_set(get_path(
    &run,
    &["macro-observation", "macro-default-functions"],
  ));
  for expected in [
    "clue-emission",
    "role-pressure",
    "candidate-ranking",
    "Held-if-edge-unproven",
    "audit-delta",
  ] {
    assert!(
      defaults.contains(expected),
      "missing macro default `{expected}`"
    );
  }
}

#[test]
fn design_non_target_reason_blocks_named_plan_implementation() {
  let run = eval_file(&fixture_path()).unwrap();
  let reason = get(&run, "design-non-target-reason");
  assert_eq!(
    as_str(get(reason, "rule")),
    "observe-emitted-clue-before-implementing-old-design"
  );
  assert!(as_bool(get(reason, "old-plan-is-bait")));
  assert!(!as_bool(get(reason, "implementation-command")));
}

#[test]
fn affected_plans_remain_observation_handles() {
  let run = eval_file(&fixture_path()).unwrap();
  let plans = get(&run, "affected-plans");
  for expected in [
    "NeedGraph",
    "NeedCursor",
    "insertableInference",
    "routeCache",
    "retrievalTimeInference",
  ] {
    let entry = get(plans, expected);
    assert_eq!(as_str(get(entry, "role")), "observation-handle");
    assert!(
      !as_bool(get(entry, "implementation-target")),
      "`{expected}` must not become an implementation target from clue discovery"
    );
  }
  assert_eq!(
    as_str(get_path(plans, &["insertableInference", "pressure"])),
    "supersede"
  );
}

#[test]
fn blocked_shortcuts_and_held_conditions_preserve_candidate_boundary() {
  let run = eval_file(&fixture_path()).unwrap();
  let blocks = string_set(get(&run, "blocked-shortcuts"));
  for expected in [
    "implement-NeedGraph-before-clue-observation",
    "implement-insertable-inference-before-fold",
    "treat-clue-as-direct-proof",
    "promote-route-hint-to-owner",
    "drop-Held-when-edge-unproven",
  ] {
    assert!(
      blocks.contains(expected),
      "missing blocked shortcut `{expected}`"
    );
  }

  let held_if = string_set(get_path(&run, &["negative-held-evidence", "held-if"]));
  for expected in [
    "clue-chain-incomplete",
    "A-to-B-edge-unproven",
    "reverse-replay-missing",
    "audit-delta-unnamed",
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
