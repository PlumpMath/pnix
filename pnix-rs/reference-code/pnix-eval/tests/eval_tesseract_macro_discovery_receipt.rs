//! Discovery receipt for tesseract-macro ontology replacement probes.
//!
//! This keeps macro-vs-legacy discoveries out of broad ontology operation design:
//! discoveries are decision evidence, not implementation commands.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../../fixtures/tesseract-macro-legacy-probe/ontology_discovery_receipt.px")
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
fn discovery_receipt_marker_and_truth_owner_are_pinned() {
  let run = eval_file(&fixture_path()).expect("discovery receipt fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-discovery-receipt"
  );
  assert_eq!(
    as_str(get_path(&run, &["receipt", "truth-owner"])),
    "project-wiki/maps/tesseract-macro-ontology-discovery-ledger.md"
  );
  assert_eq!(
    as_str(get_path(&run, &["receipt", "replacement-map"])),
    "project-wiki/maps/tesseract-macro-ontology-replacement-map.md"
  );
}

#[test]
fn discovery_receipt_records_required_macro_capability_axes() {
  let run = eval_file(&fixture_path()).unwrap();
  let axes = string_set(get_path(&run, &["receipt", "macro-capability-applied"]));
  for expected in [
    "six-layer-fold",
    "symbol-preserving-normalization",
    "owner-law-gate",
    "metaInterpret-cut",
    "SourceObject-derivation",
    "interpret-cross-builder-interaction",
    "Held-Reopen-RepairCandidate",
    "route-candidate-ranking",
    "reverse-replay-required",
    "allowed-delta-audit",
    "negative-corruption-proof",
    "performance-route-cache-pressure",
  ] {
    assert!(
      axes.contains(expected),
      "missing macro capability axis `{expected}`"
    );
  }
}

#[test]
fn discovery_compares_old_behavior_to_macro_candidate_without_owner_switch() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get_path(&run, &["receipt", "legacy-surface"])),
    "stdlib/lib/ontology.px::builtins.ontologyPromote"
  );
  assert!(as_bool(get_path(
    &run,
    &["receipt", "old-behavior", "direct-accepted"]
  )));
  assert!(!as_bool(get_path(
    &run,
    &["receipt", "macro-candidate-behavior", "direct-accepted"]
  )));
  assert!(as_bool(get_path(
    &run,
    &["receipt", "macro-candidate-behavior", "candidate-only"]
  )));
  assert_eq!(
    as_str(get_path(&run, &["receipt", "replacement-readiness"])),
    "not-proven"
  );
  assert!(!as_bool(get_path(&run, &["receipt", "owner-switch"])));
  assert!(!as_bool(get_path(
    &run,
    &["receipt", "implementation-command"]
  )));
}

#[test]
fn discovery_records_extra_and_unexpected_effects() {
  let run = eval_file(&fixture_path()).unwrap();
  let effects = string_set(get_path(&run, &["receipt", "new-capability-effect"]));
  for expected in [
    "authority-split",
    "layer-visible-audit",
    "candidate-only-gate",
    "reverse-replay-requirement",
    "transition-name-pressure",
  ] {
    assert!(
      effects.contains(expected),
      "missing new capability effect `{expected}`"
    );
  }

  let unexpected = list_strings(get_path(&run, &["receipt", "unexpected-effect"]));
  assert!(
    unexpected
      .iter()
      .any(|item| item.contains("comparison evidence instead of proof")),
    "legacy Accepted demotion must be recorded as an unexpected/design discovery"
  );
  assert!(
    unexpected
      .iter()
      .any(|item| item.contains("emitted roles instead of standalone stores")),
    "standalone store collapse must remain discovery data"
  );
}

#[test]
fn discovery_blocks_old_shortcuts_and_names_allowed_deltas() {
  let run = eval_file(&fixture_path()).unwrap();
  let blocks = string_set(get_path(&run, &["receipt", "blocked-old-shortcut"]));
  for expected in [
    "legacy-direct-accepted",
    "stage2-primitive-import",
    "owner-switch-without-replay",
    "seed-registry-precommit",
  ] {
    assert!(
      blocks.contains(expected),
      "missing blocked shortcut `{expected}`"
    );
  }

  let deltas = as_list(get_path(&run, &["receipt", "reference-delta"]));
  let fields: BTreeSet<&str> = deltas.iter().map(|d| as_str(get(d, "field"))).collect();
  assert_eq!(
    fields,
    ["authority", "proof", "runtime"].into_iter().collect()
  );
  for delta in deltas {
    assert!(as_bool(get(delta, "allowed")));
  }
}

#[test]
fn transitional_names_are_observation_handles_not_implementation_targets() {
  let run = eval_file(&fixture_path()).unwrap();
  let names = get_path(&run, &["receipt", "transitional-names-affected"]);
  for expected in [
    "NeedGraph",
    "NeedCursor",
    "CapabilityCard",
    "AssemblyTree",
    "RigorFloor",
    "legacySeedRegistry",
    "routeCache",
    "repairPromote",
  ] {
    let entry = get(names, expected);
    assert_eq!(as_str(get(entry, "role")), "observation-handle");
    assert!(
      !as_bool(get(entry, "implementation-target")),
      "`{expected}` must not become implementation target from discovery alone"
    );
  }
}

#[test]
fn decision_pressure_covers_keep_redesign_split_demote_supersede_and_hold() {
  let run = eval_file(&fixture_path()).unwrap();
  let pressure = get_path(&run, &["receipt", "decision-pressure"]);
  assert!(as_list(get(pressure, "keep")).is_empty());
  assert_eq!(
    string_set(get(pressure, "redesign")),
    ["NeedCursor", "repairPromote"].into_iter().collect()
  );
  assert_eq!(
    string_set(get(pressure, "split")),
    ["AssemblyTree"].into_iter().collect()
  );
  assert_eq!(
    string_set(get(pressure, "demote")),
    ["CapabilityCard"].into_iter().collect()
  );
  assert_eq!(
    string_set(get(pressure, "supersede")),
    ["legacySeedRegistry"].into_iter().collect()
  );
  assert_eq!(
    string_set(get(pressure, "hold")),
    ["NeedGraph", "RigorFloor", "routeCache"]
      .into_iter()
      .collect()
  );
}

#[test]
fn negative_held_and_reverse_replay_stay_required_before_readiness() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get_path(
      &run,
      &["receipt", "negative-held-evidence", "status"]
    )),
    "present"
  );
  let held_if = string_set(get_path(
    &run,
    &["receipt", "negative-held-evidence", "held-if"],
  ));
  for expected in [
    "macro-role-not-emitted",
    "reverse-replay-missing",
    "reference-delta-unnamed",
  ] {
    assert!(
      held_if.contains(expected),
      "missing Held condition `{expected}`"
    );
  }
  assert_eq!(
    as_str(get_path(&run, &["receipt", "reverse-replay-status"])),
    "required-not-run"
  );
}
