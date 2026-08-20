//! Self-learning-input cognition discovery.
//!
//! Existing learning and self-optimization .px owners are reusable substrate
//! pieces, but only under the meta-circular tesseract macro ontology
//! constitution gate. They do not become a standalone learning engine.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join(
    "../../fixtures/tesseract-macro-legacy-probe/self_learning_input_cognition_discovery_receipt.px",
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

#[test]
fn self_learning_marker_and_truth_owner_are_pinned() {
  let run = eval_file(&fixture_path()).expect("self-learning fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-self-learning-input-cognition"
  );
  assert_eq!(
    as_str(get(&run, "truth-owner")),
    "project-wiki/maps/tesseract-macro-ontology-discovery-ledger.md"
  );
}

#[test]
fn constitution_gate_is_the_base_not_a_standalone_learning_engine() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(
    as_str(get(gate, "base")),
    "meta-circular-tesseract-macro-ontology"
  );
  assert_eq!(as_str(get(gate, "role")), "constitution-gate");
  assert!(!as_bool(get(gate, "standalone-learning-engine")));
  assert!(!as_bool(get(gate, "old-ontology-authority")));

  let owners = string_set(get(gate, "substrate-owner-surfaces"));
  for expected in [
    "stdlib/lib/gate/self-optimization.px",
    "stdlib/lib/gate/learning-progress.px",
    "stdlib/lib/gate/storage-telemetry.px",
    "stdlib/lib/gate/read-metrics.px",
    "pnix-gate/px/live/self-capabilities.px",
  ] {
    assert!(owners.contains(expected), "missing owner `{expected}`");
  }
}

#[test]
fn constitution_gate_keeps_all_six_layers_and_required_boundaries() {
  let run = eval_file(&fixture_path()).unwrap();
  let layers = get_path(&run, &["constitution-gate", "layers"]);
  for key in [
    "surface", "ontology", "semantic", "gate", "runtime", "audit",
  ] {
    assert!(as_bool(get(layers, key)), "layer `{key}` must stay visible");
  }

  let boundaries = string_set(get_path(
    &run,
    &["constitution-gate", "required-boundaries"],
  ));
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
fn learning_input_fold_can_emit_candidates_but_cannot_apply_improvement() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "learning-input-fold");
  assert_eq!(as_str(get(fold, "mode")), "self-learning-input-cognition");
  assert_eq!(
    as_str(get(fold, "constitution")),
    "meta-circular-tesseract-macro-ontology"
  );
  assert!(!as_bool(get(fold, "comparison-peer-required")));
  assert!(as_bool(get(fold, "can-self-cognize")));
  assert!(as_bool(get(fold, "can-read-existing-px-owners")));
  assert!(as_bool(get(fold, "can-emit-self-improvement-candidates")));
  assert!(!as_bool(get(fold, "can-apply-self-improvement")));
  assert!(as_bool(get(fold, "owner-law-external")));
  assert!(!as_bool(get(fold, "policy-mutation-applied")));
  assert!(!as_bool(get(fold, "runtime-installed")));
  assert_eq!(as_str(get(fold, "replacement-readiness")), "not-proven");
}

#[test]
fn self_optimization_owner_projects_slow_input_to_candidate_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let candidate = get(&run, "optimization-candidate");
  assert_eq!(
    as_str(get(candidate, "artifact_family")),
    "ankh.self-optimization-candidate"
  );
  assert_eq!(as_str(get(candidate, "target")), "profile:judge-core");
  assert_eq!(
    as_str(get(candidate, "duration_delta_status")),
    "slower-than-previous"
  );
  assert_eq!(
    as_str(get(candidate, "solved_status")),
    "regressed-or-noisy-needs-review"
  );
  assert_eq!(as_str(get(candidate, "priority")), "high");
  assert_eq!(
    as_str(get(candidate, "self_modify_status")),
    "candidate-only-no-self-modification"
  );
  assert!(!as_bool(get(candidate, "policy_mutation_applied")));
  assert_eq!(as_list(get(candidate, "slowest_steps")).len(), 2);
}

#[test]
fn storage_telemetry_pressure_guards_learning_input_retention() {
  let run = eval_file(&fixture_path()).unwrap();
  let storage = get(&run, "storage-pressure");
  assert_eq!(as_str(get(storage, "hot_store_budget_status")), "pressure");
  assert!(as_bool(get(storage, "hot_store_budget_exceeded")));
  assert_eq!(
    as_number(get(storage, "hot_store_budget_remaining_bytes")),
    0.0
  );
  assert!(
    (as_number(get(storage, "hot_store_pressure_ratio")) - 1.45).abs() < 0.0001,
    "pressure ratio should be 1.45"
  );
  assert_eq!(
    as_str(get(storage, "inline_blob_mode")),
    "artifact-ref-only"
  );
  assert!(as_bool(get(storage, "state_sink_ready")));
}

#[test]
fn learning_progress_blocks_fake_learning_without_gate_or_owner_proof() {
  let run = eval_file(&fixture_path()).unwrap();
  let blocked = get(&run, "blocked-learning");
  assert!(matches!(get(blocked, "score"), Value::Null));
  assert_eq!(as_str(get(blocked, "trend")), "blocked");
  let reasons = string_set(get(blocked, "blocked_reasons"));
  for expected in [
    "learn_mode_disabled",
    "self-learning-input-not-gated",
    "owner-proof-missing",
  ] {
    assert!(reasons.contains(expected), "missing block `{expected}`");
  }

  let active = get(&run, "active-learning-preview");
  assert!(as_number(get(active, "score")) > 0.72);
  assert_eq!(as_str(get(active, "trend")), "improving");
}

#[test]
fn self_capability_surface_is_reused_as_cognition_substrate() {
  let run = eval_file(&fixture_path()).unwrap();
  let report = get(&run, "self-capability-report");
  assert_eq!(as_number(get(report, "capability_total")), 7.0);
  assert_eq!(as_number(get(report, "meta_capability_count")), 7.0);
  assert_eq!(as_str(get(report, "meta_priority")), "self-analysis-first");
  assert!(as_number(get(report, "self_referential_total")) > 0.0);
  assert!(as_number(get(report, "score")) > 0.8);
}

#[test]
fn self_learning_emits_needs_held_and_candidate_only_runtime_observations() {
  let run = eval_file(&fixture_path()).unwrap();
  let candidates = as_list(get(&run, "self-observed-candidates"));
  assert_eq!(candidates.len(), 6);
  let ids: BTreeSet<&str> = candidates
    .iter()
    .map(|candidate| as_str(get(candidate, "id")))
    .collect();
  for expected in [
    "need.self.learning-input-contract",
    "need.self.performance-bottleneck-owner-proof",
    "need.self.storage-pressure-budget",
    "need.self.capability-coverage-replay",
    "held.self.learning-mode-blocked",
    "held.self.self-modification-without-owner-proof",
  ] {
    assert!(ids.contains(expected), "missing candidate `{expected}`");
  }
  for candidate in candidates {
    assert!(!as_bool(get(candidate, "accepted")));
  }

  let runtime = get(&run, "self-runtime-observation");
  assert_eq!(
    as_str(get(runtime, "observation-model")),
    "tesseract-constitution-gated-self-learning"
  );
  assert!(as_bool(get(runtime, "can-appear-at-runtime")));
  assert!(!as_bool(get(runtime, "canonical-runtime-installed")));
  assert!(!as_bool(get(runtime, "executable-now")));
  assert!(as_bool(get(runtime, "candidate-only")));
  assert!(!as_bool(get(runtime, "owner-switch")));
  assert_eq!(as_list(get(runtime, "runtime-added-candidates")).len(), 5);
}

#[test]
fn discoveries_and_blocks_keep_self_learning_inside_tesseract_constitution() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = as_list(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 5);
  let ids: BTreeSet<&str> = discoveries
    .iter()
    .map(|discovery| as_str(get(discovery, "id")))
    .collect();
  for expected in [
    "D30.self-learning-input-is-tesseract-constitution-gated",
    "D31.existing-px-owners-already-cover-learning-substrate-pieces",
    "D32.slow-missing-repeated-failure-becomes-candidate-not-mutation",
    "D33.recording-only-block-prevents-fake-learning",
    "D34.self-improvement-needs-owner-proof-replay-and-storage-guard",
  ] {
    assert!(ids.contains(expected), "missing discovery `{expected}`");
  }
  for discovery in discoveries {
    assert!(as_bool(get(discovery, "scenario-only")));
  }

  let blocks = string_set(get(&run, "blocked-shortcuts"));
  for expected in [
    "treat-self-optimization-as-runtime-install",
    "treat-learning-progress-score-as-learning-without-gate",
    "skip-tesseract-constitution-gate",
    "mutate-policy-without-owner-proof",
    "ignore-storage-pressure-during-learning-input",
    "promote-runtime-candidate-without-replay",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
  assert_eq!(as_str(get(&run, "replacement-readiness")), "not-proven");
  assert!(!as_bool(get(&run, "owner-switch")));
  assert!(!as_bool(get(&run, "implementation-command")));
}
