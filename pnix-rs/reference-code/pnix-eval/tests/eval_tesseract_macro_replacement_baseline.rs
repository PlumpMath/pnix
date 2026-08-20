//! Initial old-vs-new ontology comparison baseline.
//!
//! This does not implement the replacement ontology. It proves that the current
//! replacement lane can compare old ontology style against a tesseract-macro
//! candidate shape while keeping broad ontology operation design frozen.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base)
    .join("../../fixtures/tesseract-macro-legacy-probe/legacy_vs_macro_initial_comparison.px")
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
fn baseline_marker_and_scope_keep_operation_design_frozen() {
  let run = eval_file(&fixture_path()).expect("baseline fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-initial-comparison-baseline"
  );
  assert_eq!(
    as_str(get(&run, "truth-owner")),
    "project-wiki/maps/tesseract-macro-ontology-replacement-map.md"
  );

  let scope = get(&run, "implementation-scope");
  assert_eq!(as_str(get(scope, "kind")), "initial-comparison-baseline");
  assert_eq!(as_str(get(scope, "ontology-operation-design")), "frozen");
  assert!(!as_bool(get(scope, "opens-new-ontology-runtime")));
  assert!(!as_bool(get(scope, "adds-rust-ontology-behavior")));
  assert!(!as_bool(get(scope, "adds-px-ontology-owner")));
}

#[test]
fn baseline_compares_same_legacy_surface_in_old_and_macro_styles() {
  let run = eval_file(&fixture_path()).unwrap();
  let source = get(&run, "source");
  assert_eq!(
    as_str(get(source, "id")),
    "legacy-ontology.promote.accepted"
  );
  assert_eq!(as_str(get(source, "source-file")), "stdlib/lib/ontology.px");
  assert_eq!(
    as_str(get(source, "source-symbol")),
    "builtins.ontologyPromote"
  );

  assert_eq!(
    as_str(get_path(&run, &["old-style", "input-id"])),
    as_str(get_path(&run, &["macro-style", "input-id"]))
  );
  assert!(as_bool(get_path(&run, &["comparison", "same-input-id"])));
}

#[test]
fn baseline_blocks_legacy_direct_accepted_as_current_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  assert!(as_bool(get_path(&run, &["old-style", "direct-accepted"])));
  assert!(!as_bool(get_path(
    &run,
    &["macro-style", "direct-accepted"]
  )));
  assert!(as_bool(get_path(&run, &["macro-style", "candidate-only"])));
  assert_eq!(
    as_str(get_path(&run, &["comparison", "replacement-readiness"])),
    "not-proven"
  );
  assert!(!as_bool(get_path(&run, &["comparison", "owner-switch"])));
}

#[test]
fn baseline_keeps_all_six_macro_layers_visible() {
  let run = eval_file(&fixture_path()).unwrap();
  let layers = get_path(&run, &["macro-style", "layers"]);
  for key in [
    "surface", "ontology", "semantic", "gate", "runtime", "audit",
  ] {
    assert!(
      as_bool(get(layers, key)),
      "layer `{}` must stay visible",
      key
    );
  }
  assert!(as_bool(get_path(&run, &["macro-style", "six-layer-fold"])));
  assert!(as_bool(get_path(
    &run,
    &["macro-style", "reverse-replay-required"]
  )));
}

#[test]
fn baseline_names_macro_role_emission_without_prebuilding_stores() {
  let run = eval_file(&fixture_path()).unwrap();
  let roles = get_path(&run, &["macro-style", "role-emission"]);
  assert_eq!(as_str(get(roles, "SourceObject")), "SourceObject");
  assert_eq!(
    as_str(get(roles, "LegacyPromotionSpecimen")),
    "LegacyPromotionSpecimen"
  );
  assert_eq!(as_str(get(roles, "CandidateGate")), "CandidateGate");
  assert_eq!(as_str(get(roles, "AuditReceipt")), "AuditReceipt");
}

#[test]
fn baseline_gate_blocks_old_shortcuts_and_runtime_is_non_executable() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get_path(&run, &["macro-style", "gate", "status"])),
    "candidate"
  );
  let blocks: BTreeSet<&str> = list_strings(get_path(&run, &["macro-style", "gate", "blocks"]))
    .into_iter()
    .collect();
  for expected in [
    "legacy-direct-accepted",
    "stage2-primitive-import",
    "owner-switch-without-replay",
  ] {
    assert!(
      blocks.contains(expected),
      "missing gate block `{}`",
      expected
    );
  }

  assert_eq!(
    as_str(get_path(&run, &["macro-style", "runtime", "route-kind"])),
    "comparison-baseline"
  );
  assert!(!as_bool(get_path(
    &run,
    &["macro-style", "runtime", "executable"]
  )));
}

#[test]
fn baseline_declares_only_allowed_reference_deltas() {
  let run = eval_file(&fixture_path()).unwrap();
  let deltas = as_list(get_path(&run, &["comparison", "allowed-deltas"]));
  assert_eq!(deltas.len(), 3);
  let fields: BTreeSet<&str> = deltas.iter().map(|d| as_str(get(d, "field"))).collect();
  assert_eq!(
    fields,
    ["authority", "proof", "runtime"].into_iter().collect()
  );
  for delta in deltas {
    assert!(
      as_bool(get(delta, "allowed")),
      "all initial comparison deltas must be explicit and allowed"
    );
  }
}

#[test]
fn baseline_keeps_old_plans_frozen_or_superseded() {
  let run = eval_file(&fixture_path()).unwrap();
  let plans = get_path(&run, &["comparison", "frozen-plan-updates"]);
  assert_eq!(as_str(get_path(plans, &["NeedGraph", "status"])), "freeze");
  assert_eq!(
    as_str(get_path(plans, &["NeedCursor", "status"])),
    "role-only"
  );
  assert_eq!(
    as_str(get_path(plans, &["CapabilityCard", "status"])),
    "freeze"
  );
  assert_eq!(
    as_str(get_path(plans, &["AssemblyTree", "status"])),
    "freeze"
  );
  assert_eq!(as_str(get_path(plans, &["RigorFloor", "status"])), "freeze");
  assert_eq!(
    as_str(get_path(plans, &["legacySeedRegistry", "status"])),
    "supersede"
  );

  assert_eq!(
    list_strings(get_path(&run, &["comparison", "next-safe-development"])),
    vec![
      "expand-comparison-baseline-one-surface-at-a-time",
      "update-frozen-plans-from-observed-role-emission",
      "do-not-open-runtime-operation-design"
    ]
  );
}
