//! tesseract-macro legacy ontology surface probe.
//!
//! This is a replacement-readiness probe, not a legacy bridge. It
//! verifies that an old ontology surface can be folded as a
//! reference/specimen by the new tesseract macro framing without
//! importing the old stdlib ontology file, calling old ontology
//! builtins, or treating legacy Accepted output as current proof.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base)
    .join("../../fixtures/tesseract-macro-legacy-probe/legacy_ontology_surface_probe.px")
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

#[test]
fn legacy_probe_marker_and_policy_pin_reference_mode() {
  let run = eval_file(&fixture_path()).expect("legacy probe fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-legacy-ontology-surface-probe"
  );
  assert_eq!(
    as_str(get(&run, "truth-owner")),
    "project-wiki/maps/post-v0_5_2-freeze-supersede-map.md"
  );

  let policy = get(&run, "source-policy");
  assert!(!as_bool(get(policy, "imports-legacy-surface")));
  assert!(!as_bool(get(policy, "calls-legacy-ontology-builtins")));
  assert!(!as_bool(get(policy, "treats-legacy-accepted-as-proof")));
}

#[test]
fn legacy_probe_all_six_layers_are_active_for_the_probe() {
  let run = eval_file(&fixture_path()).unwrap();
  let layers = get(&run, "layers");
  for key in [
    "surface-active",
    "ontology-active",
    "semantic-active",
    "gate-active",
    "runtime-active",
    "audit-active",
  ] {
    assert!(
      as_bool(get(layers, key)),
      "probe must keep `{}` active to exercise the whole fold",
      key
    );
  }
}

#[test]
fn legacy_probe_surface_names_old_ontology_surface_as_specimen() {
  let run = eval_file(&fixture_path()).unwrap();
  let surface = get_path(&run, &["fold", "surface"]);
  assert_eq!(
    as_str(get(surface, "source-file")),
    "stdlib/lib/ontology.px"
  );
  assert_eq!(
    as_str(get(surface, "source-symbol")),
    "builtins.ontologyPromote"
  );
  assert_eq!(
    list_strings(get(surface, "path")),
    vec!["legacyOntology", "promotionDecision", "accepted"]
  );
  assert!(!as_bool(get_path(surface, &["value", "current-authority"])));
}

#[test]
fn legacy_probe_ontology_classifies_old_engine_as_reference_roles() {
  let run = eval_file(&fixture_path()).unwrap();
  let ontology = get_path(&run, &["fold", "ontology"]);
  assert_eq!(
    as_str(get(ontology, "legacyOntology")),
    "LegacyEngineSurface"
  );
  assert_eq!(
    as_str(get(ontology, "promotionDecision")),
    "PromotionCandidate"
  );
  assert_eq!(as_str(get(ontology, "accepted")), "LegacyStatus");
}

#[test]
fn legacy_probe_semantic_demotes_accepted_to_reference_specimen() {
  let run = eval_file(&fixture_path()).unwrap();
  let semantic = get_path(&run, &["fold", "semantic"]);
  assert_eq!(as_str(get(semantic, "frame")), "object");
  assert_eq!(as_str(get(semantic, "authority")), "reference-only");
  assert_eq!(
    list_strings(get_path(semantic, &["normalized", "path"])),
    vec!["object", "legacyOntology", "promotionDecision", "accepted"]
  );
  assert!(!as_bool(get_path(
    semantic,
    &["normalized", "value", "current-authority"]
  )));
  assert_eq!(
    as_str(get_path(
      semantic,
      &["normalized", "value", "tesseract-role"]
    )),
    "reference-specimen"
  );
}

#[test]
fn legacy_probe_gates_are_candidate_only_and_block_direct_acceptance() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = as_list(get_path(&run, &["fold", "gate"]));
  let ids: BTreeSet<&str> = gate.iter().map(|g| as_str(get(g, "id"))).collect();
  let expected: BTreeSet<&str> = [
    "gate.legacyOntology.reference-only",
    "gate.promotionDecision.no-direct-accepted",
    "gate.accepted.demote-to-candidate",
  ]
  .into_iter()
  .collect();
  assert_eq!(ids, expected);
  for g in gate {
    assert_eq!(
      as_str(get(g, "status")),
      "candidate",
      "legacy probe gates must never auto-satisfy"
    );
  }
}

#[test]
fn legacy_probe_runtime_route_is_non_executable_migration_probe() {
  let run = eval_file(&fixture_path()).unwrap();
  let runtime = as_list(get_path(&run, &["fold", "runtime"]));
  assert_eq!(runtime.len(), 1);
  let route = &runtime[0];
  assert_eq!(
    as_str(get(route, "id")),
    "route.legacyOntology.rewrite-probe"
  );
  assert_eq!(as_str(get(route, "route-kind")), "migration-probe");
  assert_eq!(as_str(get(route, "status")), "candidate");
  assert!(!as_bool(get(route, "executable")));
}

#[test]
fn legacy_probe_audit_and_replacement_readiness_are_explicitly_not_proven() {
  let run = eval_file(&fixture_path()).unwrap();
  let audit = get_path(&run, &["fold", "audit"]);
  assert_eq!(
    as_str(get(audit, "fold-reason")),
    "LegacyEngineSurface|PromotionCandidate|LegacyStatus -> object"
  );
  assert_eq!(as_str(get(audit, "replacement-readiness")), "not-proven");
  assert_eq!(
    as_str(get(audit, "replay-ref")),
    "audit.legacy-probe.legacy-ontology.promote.accepted"
  );

  let replacement = get_path(&run, &["fold", "replacement"]);
  assert_eq!(as_str(get(replacement, "readiness")), "not-proven");
  assert!(!as_bool(get(replacement, "direct-stage2-primitive")));
  assert!(!as_bool(get(replacement, "current-semantic-owner")));
  assert_eq!(
    as_str(get(replacement, "next-action")),
    "macro-fold-rewrite-required"
  );
}
