//! R3 role-emission verdict discovery.
//!
//! R2 compared the old `builtins.ontologyPromote` surface with the macro
//! candidate shape. This test pins the next boundary: R3 records which roles
//! the tesseract macro fold emits for that one surface. It does not implement a
//! replacement, install runtime behavior, or switch owners.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base)
    .join("../../fixtures/tesseract-macro-legacy-probe/role_emission_verdict_discovery_receipt.px")
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

fn as_number(v: &Value) -> f64 {
  match v {
    Value::Int(n) => *n as f64,
    Value::Float(n) => *n,
    other => panic!("expected number, got {:?}", other),
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

fn attrs_by_id<'a>(items: &'a Value) -> BTreeMap<&'a str, &'a Value> {
  as_list(items)
    .iter()
    .map(|item| (as_str(get(item, "id")), item))
    .collect()
}

#[test]
fn r3_marker_and_constitution_owner_are_pinned() {
  let run = eval_file(&fixture_path()).expect("R3 role emission fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-r3-role-emission-verdict"
  );
  assert_eq!(
    as_str(get(&run, "truth-owner")),
    "project-wiki/maps/tesseract-macro-ontology-discovery-ledger.md"
  );
  assert_eq!(
    as_str(get(&run, "replacement-map")),
    "project-wiki/maps/tesseract-macro-ontology-replacement-map.md"
  );
  assert_eq!(
    as_str(get(&run, "constitution-owner")),
    "stdlib/lib/gate/tesseract-constitution.px"
  );
}

#[test]
fn constitution_gate_keeps_r3_candidate_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(as_str(get(gate, "scenario")), "r3-role-emission-verdict");
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  assert_eq!(as_str(get(gate, "replacement-readiness")), "not-proven");

  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "use-legacy-accepted-as-current-role-proof",
    "prebuild-NeedGraph-store-from-role-name",
    "prebuild-CapabilityCard-registry-from-role-name",
    "treat-role-verdict-as-owner-switch",
    "skip-r4-macro-native-rewrite-candidate",
    "skip-r5-reverse-replay",
    "install-runtime-route-from-role-verdict",
    "treat-llm-constructor-prose-as-role-emission",
  ] {
    assert!(blocks.contains(expected), "missing gate block `{expected}`");
  }
}

#[test]
fn legacy_surface_is_builtins_ontology_promote_reference_specimen() {
  let run = eval_file(&fixture_path()).unwrap();
  let surface = get(&run, "legacy-surface");
  assert_eq!(
    as_str(get(surface, "id")),
    "legacy-ontology.promote.accepted"
  );
  assert_eq!(
    as_str(get(surface, "source-file")),
    "stdlib/lib/ontology.px"
  );
  assert_eq!(
    as_str(get(surface, "source-symbol")),
    "builtins.ontologyPromote"
  );
  assert_eq!(as_str(get(surface, "specimen-role")), "reference-specimen");
  assert!(!as_bool(get_path(
    surface,
    &["old-output", "current-authority"]
  )));
}

#[test]
fn emitted_roles_are_candidates_not_implementation_targets() {
  let run = eval_file(&fixture_path()).unwrap();
  let roles = attrs_by_id(get(&run, "role-emission-verdicts"));
  assert_eq!(roles.len(), 8);

  for expected in [
    "role.source-object",
    "role.legacy-promotion-specimen",
    "role.candidate-gate",
    "role.promotion-readiness",
    "role.audit-receipt",
    "role.reverse-replay-requirement",
    "role.accepted-status-evidence",
    "role.owner-switch-receipt-need",
  ] {
    let role = roles
      .get(expected)
      .unwrap_or_else(|| panic!("missing `{expected}`"));
    assert!(as_bool(get(role, "emitted")));
    assert!(!as_bool(get(role, "accepted")));
    assert!(!as_bool(get(role, "implementation-target")));
    assert!(!as_bool(get(role, "owner-switch")));
  }
}

#[test]
fn accepted_status_is_demoted_and_promotion_readiness_is_split() {
  let run = eval_file(&fixture_path()).unwrap();
  let roles = attrs_by_id(get(&run, "role-emission-verdicts"));

  let accepted = roles.get("role.accepted-status-evidence").unwrap();
  assert_eq!(
    as_str(get(accepted, "verdict")),
    "demote-from-proof-to-evidence"
  );
  assert!(
    as_str(get(accepted, "replacement-use")).contains("old behavior only"),
    "Accepted status must remain old-behavior evidence"
  );

  let readiness = roles.get("role.promotion-readiness").unwrap();
  assert_eq!(
    as_str(get(readiness, "verdict")),
    "split-into-proof-obligations"
  );
  let use_text = as_str(get(readiness, "replacement-use"));
  for expected in ["replay", "negative Held", "reference delta", "owner-law"] {
    assert!(
      use_text.contains(expected),
      "readiness split must mention `{expected}`"
    );
  }
}

#[test]
fn legacy_plan_store_roles_are_not_emitted() {
  let run = eval_file(&fixture_path()).unwrap();
  let plans = attrs_by_id(get(&run, "non-emitted-legacy-plan-roles"));
  assert_eq!(plans.len(), 4);
  for expected in [
    "role.needgraph-store",
    "role.capability-card-registry",
    "role.assembly-tree-runtime",
    "role.rigorfloor-schema",
  ] {
    let plan = plans
      .get(expected)
      .unwrap_or_else(|| panic!("missing `{expected}`"));
    assert!(!as_bool(get(plan, "emitted")));
    assert!(
      as_str(get(plan, "verdict")).starts_with("not-emitted"),
      "`{expected}` must stay not emitted"
    );
  }
}

#[test]
fn six_layer_role_fold_preserves_candidate_boundary() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-role-fold");
  assert_eq!(as_str(get(fold, "mode")), "r3-role-emission-verdict");
  for layer in [
    "surface", "ontology", "semantic", "gate", "runtime", "audit",
  ] {
    assert!(
      as_bool(get_path(fold, &[layer, "visible"])),
      "layer `{layer}` must stay visible"
    );
  }
  assert_eq!(
    as_str(get_path(fold, &["surface", "source-symbol"])),
    "builtins.ontologyPromote"
  );
  assert_eq!(
    as_number(get_path(fold, &["ontology", "emitted-role-count"])),
    8.0
  );
  assert!(as_bool(get_path(
    fold,
    &["semantic", "role-name-precommit-blocked"]
  )));
  assert!(!as_bool(get_path(fold, &["runtime", "executable-now"])));
  assert_eq!(
    as_str(get_path(fold, &["audit", "reverse-replay-status"])),
    "required-not-run"
  );
}

#[test]
fn r4_entry_opens_only_for_this_surface_without_owner_switch() {
  let run = eval_file(&fixture_path()).unwrap();
  let r4 = get(&run, "r4-entry-boundary");
  assert_eq!(
    as_str(get(r4, "surface")),
    "stdlib/lib/ontology.px::builtins.ontologyPromote"
  );
  assert!(as_bool(get(r4, "r3-verdict-closed-for-this-surface")));
  assert!(as_bool(get(
    r4,
    "r4-macro-native-rewrite-candidate-may-start"
  )));
  assert_eq!(
    as_str(get(r4, "r4-scope")),
    "this-one-legacy-promote-surface-only"
  );
  assert!(!as_bool(get(r4, "broad-ontology-runtime-open")));
  assert!(!as_bool(get(r4, "owner-switch-open")));
  assert!(!as_bool(get(r4, "runtime-install-open")));
  assert!(!as_bool(get(r4, "role-store-schema-open")));

  let required = string_set(get(r4, "required-next"));
  for expected in [
    "macro-native-rewrite-candidate",
    "reference-delta",
    "reverse-replay",
    "negative-held-proof",
  ] {
    assert!(
      required.contains(expected),
      "missing R4/R5 requirement `{expected}`"
    );
  }
}

#[test]
fn self_knowledge_candidates_keep_r3_from_becoming_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  let candidates = attrs_by_id(get(&run, "self-knowledge-candidates"));
  assert_eq!(candidates.len(), 5);
  for expected in [
    "knowledge.r3.role-emission-verdict-pinned",
    "held.r3.role-store-schema-not-emitted",
    "held.r3.reverse-replay-not-run",
    "need.r3.macro-native-rewrite-candidate",
    "need.r3.owner-switch-receipt-later",
  ] {
    let candidate = candidates
      .get(expected)
      .unwrap_or_else(|| panic!("missing `{expected}`"));
    assert!(!as_bool(get(candidate, "accepted")));
  }
}

#[test]
fn runtime_observation_is_candidate_only_and_not_installed() {
  let run = eval_file(&fixture_path()).unwrap();
  let runtime = get(&run, "runtime-observation");
  assert_eq!(
    as_str(get(runtime, "observation-model")),
    "r3-role-emission-runtime-candidates"
  );
  assert!(as_bool(get(runtime, "can-appear-at-runtime")));
  assert!(as_bool(get(runtime, "candidate-only")));
  assert!(!as_bool(get(runtime, "canonical-runtime-installed")));
  assert!(!as_bool(get(runtime, "executable-now")));
  assert!(!as_bool(get(runtime, "owner-switch")));
  assert_eq!(as_list(get(runtime, "runtime-added-candidates")).len(), 3);
}

#[test]
fn discoveries_record_d84_through_d90() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 7);
  for expected in [
    "D84.r3-role-emission-verdict-is-surface-scoped",
    "D85.legacy-accepted-demotes-to-evidence-role",
    "D86.promotion-readiness-splits-into-proof-obligations",
    "D87.role-name-precommit-is-blocked",
    "D88.r4-entry-opens-only-after-r3-and-only-for-one-surface",
    "D89.r3-runtime-candidates-are-non-executable",
    "D90.r3-preserves-pnix-independence-from-llm-prose",
  ] {
    let d = discoveries
      .get(expected)
      .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
    assert!(as_bool(get(d, "scenario-only")));
  }
}

#[test]
fn affected_plans_do_not_become_implementation_targets() {
  let run = eval_file(&fixture_path()).unwrap();
  let affected = get(&run, "affected-plans");
  assert_eq!(
    as_str(get_path(affected, &["legacyPromote", "pressure"])),
    "allow-r4-macro-native-rewrite-candidate"
  );
  assert_eq!(
    as_str(get_path(affected, &["NeedGraph", "pressure"])),
    "hold-store-schema-not-emitted"
  );
  assert_eq!(
    as_str(get_path(affected, &["ownerSwitch", "pressure"])),
    "forbidden-at-r3"
  );
  for key in [
    "legacyPromote",
    "NeedGraph",
    "CapabilityCard",
    "ownerSwitch",
  ] {
    assert!(!as_bool(get_path(
      affected,
      &[key, "implementation-target"]
    )));
  }
}

#[test]
fn negative_held_evidence_blocks_old_and_constructor_shortcuts() {
  let run = eval_file(&fixture_path()).unwrap();
  let negative = get(&run, "negative-held-evidence");
  assert_eq!(as_str(get(negative, "status")), "present");
  let rejects = string_set(get(negative, "rejects"));
  for expected in [
    "legacy-direct-accepted-as-role-proof",
    "role-name-precommit",
    "owner-switch-at-r3",
    "runtime-install-at-r3",
    "llm-prose-as-role-emission",
  ] {
    assert!(rejects.contains(expected), "missing rejection `{expected}`");
  }

  let blocks = string_set(get(&run, "blocked-shortcuts"));
  for expected in [
    "use-legacy-accepted-as-current-role-proof",
    "prebuild-NeedGraph-store-from-role-name",
    "treat-role-verdict-as-owner-switch",
    "install-runtime-route-from-role-verdict",
    "treat-llm-constructor-prose-as-role-emission",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
}

#[test]
fn r3_receipt_keeps_replacement_not_ready() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "reverse-replay-status")),
    "required-not-run"
  );
  assert_eq!(as_str(get(&run, "replacement-readiness")), "not-proven");
  assert!(!as_bool(get(&run, "owner-switch")));
  assert!(!as_bool(get(&run, "implementation-command")));
}
