//! Discovery receipt for legacy ontologyLift / ontologyQuery / ontologyEmit.
//!
//! These old surfaces are specimen data. The macro candidate may emit richer
//! source, query-intent, projection, provenance, Held, and audit roles.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base)
    .join("../../fixtures/tesseract-macro-legacy-probe/lift_query_emit_discovery_receipt.px")
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
fn lift_query_emit_marker_and_truth_owner_are_pinned() {
  let run = eval_file(&fixture_path()).expect("lift/query/emit discovery fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-lift-query-emit-discovery"
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
fn lift_query_emit_names_three_legacy_surfaces_as_specimens() {
  let run = eval_file(&fixture_path()).unwrap();
  let surfaces = as_list(get(&run, "source-surfaces"));
  assert_eq!(surfaces.len(), 3);
  let symbols: BTreeSet<&str> = surfaces
    .iter()
    .map(|surface| as_str(get(surface, "source-symbol")))
    .collect();
  assert_eq!(
    symbols,
    [
      "builtins.ontologyLift",
      "builtins.ontologyQuery",
      "builtins.ontologyEmit"
    ]
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
fn old_behavior_records_lift_query_emit_shapes_without_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get_path(&run, &["old-behavior", "lift", "authority-style"])),
    "candidate-fact"
  );
  assert_eq!(
    as_str(get_path(
      &run,
      &["old-behavior", "query", "authority-style"]
    )),
    "request-descriptor"
  );
  assert!(!as_bool(get_path(
    &run,
    &["old-behavior", "query", "store-lookup"]
  )));
  assert_eq!(
    as_str(get_path(&run, &["old-behavior", "emit", "authority-style"])),
    "projection-record"
  );
  assert!(!as_bool(get_path(
    &run,
    &["old-behavior", "emit", "event-log-write"]
  )));
}

#[test]
fn macro_candidate_keeps_roles_candidate_only_and_layers_visible() {
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
    &["macro-candidate", "runtime-executable"]
  )));
  assert!(!as_bool(get_path(
    &run,
    &["macro-candidate", "owner-switch"]
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
fn macro_candidate_names_richer_roles_without_store_or_event_ownership() {
  let run = eval_file(&fixture_path()).unwrap();
  let roles = get_path(&run, &["macro-candidate", "roles"]);
  assert_eq!(
    as_str(get(roles, "SourceObjectCandidate")),
    "lifted surface with provenance pressure"
  );
  assert_eq!(
    as_str(get(roles, "QueryIntent")),
    "query request as candidate need/lookup pressure"
  );
  assert_eq!(
    as_str(get(roles, "ProjectionEventSpecimen")),
    "emit output as projection specimen"
  );
  assert_eq!(
    as_str(get(roles, "ProvenanceClue")),
    "path/string-context/audit clue when present"
  );
  assert_eq!(
    as_str(get(roles, "AuditReceipt")),
    "required before query/emit becomes proof"
  );
}

#[test]
fn discoveries_capture_lift_query_emit_macro_differences() {
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
      "D8.lift-candidate-fact-becomes-sourceobject-pressure",
      "D9.query-envelope-becomes-query-intent",
      "D10.emit-projection-becomes-audit-clue"
    ]
    .into_iter()
    .collect()
  );
  let effects: Vec<&str> = discoveries
    .iter()
    .map(|discovery| as_str(get(discovery, "macro-effect")))
    .collect();
  assert!(
    effects
      .iter()
      .any(|effect| effect.contains("SourceObjectCandidate")),
    "lift must become SourceObject/provenance pressure"
  );
  assert!(
    effects.iter().any(|effect| effect.contains("QueryIntent")),
    "query must become intent/Need pressure"
  );
  assert!(
    effects.iter().any(|effect| effect.contains("audit clue")),
    "emit must become projection/audit clue"
  );
}

#[test]
fn affected_plans_remain_observation_handles() {
  let run = eval_file(&fixture_path()).unwrap();
  let plans = get(&run, "affected-plans");
  for expected in [
    "ContextualFactStore",
    "retrievalTimeInference",
    "NeedCursor",
    "ExpressionProjectionOwner",
    "AuditEventLog",
    "doghouseStore",
  ] {
    let entry = get(plans, expected);
    assert_eq!(as_str(get(entry, "role")), "observation-handle");
    assert!(
      !as_bool(get(entry, "implementation-target")),
      "`{expected}` must remain observation-only"
    );
  }
  assert_eq!(
    as_str(get_path(plans, &["ExpressionProjectionOwner", "pressure"])),
    "split"
  );
  assert_eq!(
    as_str(get_path(plans, &["doghouseStore", "pressure"])),
    "demote"
  );
}

#[test]
fn blocked_shortcuts_and_held_conditions_keep_boundary_closed() {
  let run = eval_file(&fixture_path()).unwrap();
  let blocks = string_set(get(&run, "blocked-shortcuts"));
  for expected in [
    "lifted-candidate-as-Accepted-fact",
    "query-envelope-as-answer",
    "emit-projection-as-audit-proof",
    "doghouse-store-as-ontology-owner",
    "projection-record-as-semantic-owner",
  ] {
    assert!(
      blocks.contains(expected),
      "missing blocked shortcut `{expected}`"
    );
  }

  let held_if = string_set(get_path(&run, &["negative-held-evidence", "held-if"]));
  for expected in [
    "provenance-missing",
    "query-result-missing",
    "emit-replay-missing",
    "semantic-owner-unnamed",
    "audit-receipt-missing",
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
