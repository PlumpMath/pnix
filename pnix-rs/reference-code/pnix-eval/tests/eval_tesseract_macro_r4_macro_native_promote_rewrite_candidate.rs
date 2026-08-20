//! R4 macro-native promote rewrite candidate discovery.
//!
//! R3 pinned role emission for `builtins.ontologyPromote`. This test pins the
//! next narrow boundary: R4 writes a macro-native rewrite candidate for that
//! one surface only. It must not call the old ontologyPromote surface, emit
//! legacy Accepted, install runtime behavior, or switch owners.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join(
    "../../fixtures/tesseract-macro-legacy-probe/macro_native_promote_rewrite_candidate_receipt.px",
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
fn r4_marker_and_constitution_owner_are_pinned() {
  let run = eval_file(&fixture_path()).expect("R4 rewrite candidate fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-r4-macro-native-promote-rewrite-candidate"
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
fn constitution_gate_keeps_r4_candidate_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "r4-macro-native-promote-rewrite-candidate"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  assert_eq!(as_str(get(gate, "replacement-readiness")), "not-proven");

  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "thin-wrapper-around-legacy-ontologyPromote",
    "emit-legacy-Accepted-as-r4-output",
    "drop-source-provenance",
    "skip-held-reopen-path",
    "skip-negative-held-proof",
    "skip-r5-reverse-replay",
    "install-runtime-route-from-r4",
    "treat-r4-candidate-as-owner-switch",
    "prebuild-NeedGraph-store-from-r4",
    "treat-llm-constructor-prose-as-rewrite",
  ] {
    assert!(blocks.contains(expected), "missing gate block `{expected}`");
  }
}

#[test]
fn r4_is_scoped_to_builtins_ontology_promote_after_r3() {
  let run = eval_file(&fixture_path()).unwrap();
  let surface = get(&run, "legacy-surface");
  assert_eq!(
    as_str(get(surface, "source-symbol")),
    "builtins.ontologyPromote"
  );
  assert_eq!(as_str(get(surface, "specimen-role")), "reference-specimen");
  assert!(!as_bool(get_path(
    surface,
    &["old-output", "current-authority"]
  )));

  let r3 = get(&run, "r3-boundary");
  assert_eq!(
    as_str(get(r3, "r3-verdict-ref")),
    "tesseract-macro-ontology-r3-role-emission-verdict"
  );
  assert!(as_bool(get(r3, "r3-verdict-closed-for-this-surface")));
  assert_eq!(
    as_str(get(r3, "r4-scope")),
    "this-one-legacy-promote-surface-only"
  );
}

#[test]
fn rewrite_candidate_is_macro_native_not_wrapper_or_accepted_output() {
  let run = eval_file(&fixture_path()).unwrap();
  let candidate = get(&run, "rewrite-candidate");
  assert_eq!(
    as_str(get(candidate, "id")),
    "r4.macro-native-promote.rewrite-candidate"
  );
  assert_eq!(as_str(get(candidate, "phase")), "R4");
  assert_eq!(
    as_str(get(candidate, "candidate-kind")),
    "macro-native-rewrite-candidate"
  );
  assert!(as_bool(get(candidate, "uses-emitted-r3-roles")));
  assert!(!as_bool(get(candidate, "calls-legacy-ontologyPromote")));
  assert!(!as_bool(get(candidate, "thin-wrapper")));
  assert!(!as_bool(get(candidate, "direct-accepted")));
  assert!(!as_bool(get(candidate, "old-accepted-output")));
  assert!(as_bool(get(candidate, "candidate-only")));
  assert!(!as_bool(get(candidate, "executable-now")));
  assert!(!as_bool(get(candidate, "installed")));
  assert!(!as_bool(get(candidate, "owner-switch")));
  assert_eq!(
    as_str(get(candidate, "output-status")),
    "ready-for-r5-reverse-replay"
  );
}

#[test]
fn rewrite_steps_use_emitted_roles_and_remain_candidate_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let steps = attrs_by_id(get(&run, "rewrite-steps"));
  assert_eq!(steps.len(), 5);
  for (id, role) in [
    ("step.1.source-object", "SourceObject"),
    ("step.2.legacy-specimen", "LegacyPromotionSpecimen"),
    ("step.3.candidate-gate", "CandidateGate"),
    ("step.4.promotion-readiness", "PromotionReadiness"),
    ("step.5.audit-receipt", "AuditReceipt"),
  ] {
    let step = steps.get(id).unwrap_or_else(|| panic!("missing `{id}`"));
    assert_eq!(as_str(get(step, "role")), role);
    assert!(as_bool(get(step, "candidate-only")));
    assert!(!as_bool(get(step, "accepted")));
  }
}

#[test]
fn reference_deltas_are_explicit_before_r5() {
  let run = eval_file(&fixture_path()).unwrap();
  let deltas = attrs_by_id(get(&run, "reference-deltas"));
  assert_eq!(deltas.len(), 5);
  for expected in [
    "authority",
    "output-status",
    "runtime",
    "proof",
    "source-provenance",
  ] {
    let delta = deltas
      .get(expected)
      .unwrap_or_else(|| panic!("missing delta `{expected}`"));
    assert!(as_bool(get(delta, "allowed")));
  }
  assert_eq!(
    as_str(get(deltas.get("output-status").unwrap(), "legacy")),
    "Accepted"
  );
  assert_eq!(
    as_str(get(deltas.get("output-status").unwrap(), "macro")),
    "ready-for-r5-reverse-replay"
  );
}

#[test]
fn held_reopen_trials_cover_wrapper_accepted_delta_negative_and_r5_failures() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "held-reopen-trials"));
  assert_eq!(trials.len(), 7);

  for expected in [
    "trial.A.source-provenance-missing",
    "trial.B.legacy-wrapper-requested",
    "trial.C.accepted-output-requested",
    "trial.D.reference-delta-missing",
    "trial.E.negative-held-missing",
    "trial.F.r5-target-missing",
  ] {
    let trial = trials
      .get(expected)
      .unwrap_or_else(|| panic!("missing trial `{expected}`"));
    assert_eq!(as_str(get(trial, "outcome")), "Held");
    assert!(!as_bool(get(trial, "accepted")));
    assert_ne!(as_str(get(trial, "reopen-path")), "not-needed");
  }

  let complete = trials.get("trial.G.complete-candidate").unwrap();
  assert_eq!(
    as_str(get(complete, "outcome")),
    "ready-for-r5-reverse-replay"
  );
  assert!(!as_bool(get(complete, "accepted")));
}

#[test]
fn six_layer_rewrite_fold_preserves_runtime_and_audit_boundaries() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-rewrite-fold");
  assert_eq!(
    as_str(get(fold, "mode")),
    "r4-macro-native-promote-rewrite-candidate"
  );
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
  assert!(!as_bool(get_path(
    fold,
    &["ontology", "old-store-roles-emitted"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "old-accepted-demoted"]
  )));
  assert!(!as_bool(get_path(fold, &["runtime", "executable-now"])));
  assert!(!as_bool(get_path(fold, &["runtime", "installed"])));
  assert_eq!(
    as_number(get_path(fold, &["audit", "reference-delta-count"])),
    5.0
  );
  assert_eq!(
    as_str(get_path(fold, &["audit", "reverse-replay-status"])),
    "required-not-run"
  );
}

#[test]
fn r5_boundary_opens_replay_not_replacement_readiness() {
  let run = eval_file(&fixture_path()).unwrap();
  let r5 = get(&run, "r5-boundary");
  assert!(as_bool(get(r5, "r4-candidate-written")));
  assert!(as_bool(get(r5, "r5-reverse-replay-may-start")));
  assert_eq!(
    as_str(get(r5, "r5-scope")),
    "replay-r4-candidate-against-legacy-promote-specimen"
  );
  assert_eq!(
    as_str(get(r5, "replacement-readiness-after-r4")),
    "not-proven"
  );
  assert!(!as_bool(get(r5, "owner-switch-open")));
  assert!(!as_bool(get(r5, "runtime-install-open")));

  let required = string_set(get(r5, "required-next"));
  for expected in [
    "execute-or-replay-candidate-through-current-runtime",
    "compare-against-legacy-promotion-specimen",
    "check-reference-deltas",
    "preserve-audit-refs",
    "emit-held-if-unexplained-mismatch",
  ] {
    assert!(
      required.contains(expected),
      "missing R5 requirement `{expected}`"
    );
  }
}

#[test]
fn runtime_observation_is_candidate_only_and_not_installed() {
  let run = eval_file(&fixture_path()).unwrap();
  let runtime = get(&run, "runtime-observation");
  assert_eq!(
    as_str(get(runtime, "observation-model")),
    "r4-macro-native-promote-runtime-candidates"
  );
  assert!(as_bool(get(runtime, "can-appear-at-runtime")));
  assert!(as_bool(get(runtime, "candidate-only")));
  assert!(!as_bool(get(runtime, "canonical-runtime-installed")));
  assert!(!as_bool(get(runtime, "executable-now")));
  assert!(!as_bool(get(runtime, "owner-switch")));
  assert_eq!(as_list(get(runtime, "runtime-added-candidates")).len(), 3);
}

#[test]
fn discoveries_record_d91_through_d98() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 8);
  for expected in [
    "D91.r4-rewrite-is-macro-native-not-legacy-wrapper",
    "D92.r4-preserves-source-provenance",
    "D93.r4-demotes-accepted-output",
    "D94.r4-held-reopen-paths-are-load-bearing",
    "D95.r4-reference-deltas-are-explicit-before-replay",
    "D96.r4-produces-r5-target-not-replacement-readiness",
    "D97.r4-blocks-store-schema-and-runtime-install",
    "D98.r4-preserves-pnix-independence-from-constructor-prose",
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
    "ready-for-r5-reverse-replay"
  );
  assert_eq!(
    as_str(get_path(affected, &["runtimeInstall", "role"])),
    "forbidden-at-r4"
  );
  assert_eq!(
    as_str(get_path(affected, &["ownerSwitch", "pressure"])),
    "forbidden-at-r4"
  );
  for key in [
    "legacyPromote",
    "NeedGraph",
    "CapabilityCard",
    "runtimeInstall",
    "ownerSwitch",
  ] {
    assert!(!as_bool(get_path(
      affected,
      &[key, "implementation-target"]
    )));
  }
}

#[test]
fn negative_held_evidence_blocks_old_constructor_and_runtime_shortcuts() {
  let run = eval_file(&fixture_path()).unwrap();
  let negative = get(&run, "negative-held-evidence");
  assert_eq!(as_str(get(negative, "status")), "present");
  let rejects = string_set(get(negative, "rejects"));
  for expected in [
    "legacy-ontologyPromote-wrapper",
    "legacy-Accepted-output",
    "source-provenance-drop",
    "reference-delta-omission",
    "negative-path-omission",
    "r5-reverse-replay-skip",
    "runtime-install-at-r4",
    "owner-switch-at-r4",
    "llm-prose-as-rewrite",
  ] {
    assert!(rejects.contains(expected), "missing rejection `{expected}`");
  }

  let blocks = string_set(get(&run, "blocked-shortcuts"));
  for expected in [
    "thin-wrapper-around-legacy-ontologyPromote",
    "emit-legacy-Accepted-as-r4-output",
    "skip-held-reopen-path",
    "skip-r5-reverse-replay",
    "install-runtime-route-from-r4",
    "treat-r4-candidate-as-owner-switch",
    "treat-llm-constructor-prose-as-rewrite",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
}

#[test]
fn r4_receipt_keeps_replacement_not_ready() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "reverse-replay-status")),
    "required-not-run"
  );
  assert_eq!(as_str(get(&run, "replacement-readiness")), "not-proven");
  assert!(!as_bool(get(&run, "owner-switch")));
  assert!(!as_bool(get(&run, "implementation-command")));
}
