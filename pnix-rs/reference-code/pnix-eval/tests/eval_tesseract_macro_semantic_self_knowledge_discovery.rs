//! Semantic self-knowledge discovery.
//!
//! Meaning interpretation and self-knowledgeization are first-class cognition
//! surfaces, but they remain candidate/Held data under the tesseract
//! constitution gate until replay, provenance, and owner proof close.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join(
    "../../fixtures/tesseract-macro-legacy-probe/semantic_self_knowledge_discovery_receipt.px",
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
fn semantic_self_knowledge_marker_and_owner_are_pinned() {
  let run = eval_file(&fixture_path()).expect("semantic self-knowledge fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-semantic-self-knowledge"
  );
  assert_eq!(
    as_str(get(&run, "truth-owner")),
    "project-wiki/maps/tesseract-macro-ontology-discovery-ledger.md"
  );
  assert_eq!(
    as_str(get(&run, "constitution-owner")),
    "stdlib/lib/gate/tesseract-constitution.px"
  );
}

#[test]
fn meaning_surface_is_directive_data_not_implementation_command() {
  let run = eval_file(&fixture_path()).unwrap();
  let surface = get(&run, "meaning-surface");
  assert_eq!(
    as_str(get(surface, "id")),
    "surface.user-directive.semantic-self-knowledge"
  );
  assert_eq!(as_str(get(surface, "source-kind")), "user-directive");
  assert_eq!(
    as_str(get(surface, "author-intent")),
    "push-meaning-interpretation-and-self-knowledgeization"
  );
  assert!(!as_bool(get(surface, "command-is-implementation")));
  assert!(!as_bool(get(surface, "old-plan-target")));
  assert!(as_bool(get(surface, "requires-constitution-gate")));
}

#[test]
fn constitution_gate_keeps_meaning_candidate_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(
    as_str(get(gate, "artifact_family")),
    "tesseract.macro.constitution-gate"
  );
  assert_eq!(
    as_str(get(gate, "scenario")),
    "semantic-self-knowledge-cognition"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  assert_eq!(as_str(get(gate, "replacement-readiness")), "not-proven");

  let output_roles = string_set(get(gate, "output-roles"));
  for expected in [
    "meaning.intent.*",
    "knowledge.self.*.candidate",
    "need.self.semantic-*",
    "held.self.semantic-*",
    "runtime.self-knowledge.*.candidate",
  ] {
    assert!(
      output_roles.contains(expected),
      "missing output role `{expected}`"
    );
  }
}

#[test]
fn six_layer_meaning_fold_keeps_surface_through_audit_visible() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-meaning-fold");
  assert_eq!(
    as_str(get(fold, "mode")),
    "semantic-self-knowledge-cognition"
  );
  assert!(!as_bool(get(fold, "comparison-peer-required")));
  for layer in [
    "surface", "ontology", "semantic", "gate", "runtime", "audit",
  ] {
    assert!(
      as_bool(get_path(fold, &[layer, "visible"])),
      "layer `{layer}` must be visible"
    );
  }
  assert_eq!(
    as_str(get_path(fold, &["gate", "constitution-verdict"])),
    "candidate-only"
  );
  assert!(!as_bool(get_path(fold, &["runtime", "executable-now"])));
  assert!(as_bool(get_path(fold, &["audit", "replay-required"])));
  assert!(as_bool(get_path(
    fold,
    &["audit", "negative-held-proof-required"]
  )));
}

#[test]
fn semantic_interpretation_extracts_roles_without_autopromotion() {
  let run = eval_file(&fixture_path()).unwrap();
  let sem = get(&run, "semantic-interpretation");
  assert_eq!(
    as_str(get(sem, "strong-push-means")),
    "more probes, more meaning roles, more self-knowledge candidates, more Held evidence"
  );
  let not_mean = string_set(get(sem, "strong-push-does-not-mean"));
  for expected in [
    "auto-accept-policy",
    "runtime-install",
    "owner-switch",
    "drop-held-or-replay",
  ] {
    assert!(
      not_mean.contains(expected),
      "missing no-meaning `{expected}`"
    );
  }
  assert_eq!(
    as_str(get(sem, "llm-slot")),
    "proposal-verbalization-adapter"
  );
  assert_eq!(
    as_str(get(sem, "inner-lifecycle-owner")),
    "meta-circular-tesseract-macro-ontology"
  );

  let roles = as_list(get(sem, "semantic-roles"));
  assert_eq!(roles.len(), 4);
  let ids: BTreeSet<&str> = roles.iter().map(|role| as_str(get(role, "id"))).collect();
  for expected in [
    "meaning.intent.push-strongly",
    "meaning.boundary.not-implementation-command",
    "meaning.boundary.llm-not-lifecycle-owner",
    "meaning.self.knowledgeization-pressure",
  ] {
    assert!(ids.contains(expected), "missing semantic role `{expected}`");
  }
  for role in roles {
    assert!(!as_bool(get(role, "accepted")));
  }
}

#[test]
fn self_knowledge_candidates_include_needs_and_held_entries() {
  let run = eval_file(&fixture_path()).unwrap();
  let candidates = as_list(get(&run, "self-knowledge-candidates"));
  assert_eq!(candidates.len(), 6);
  let ids: BTreeSet<&str> = candidates
    .iter()
    .map(|candidate| as_str(get(candidate, "id")))
    .collect();
  for expected in [
    "knowledge.self.meaning-directive",
    "knowledge.self.judgement-lifecycle-owner",
    "need.self.semantic-provenance",
    "need.self.knowledge-replay",
    "held.self.meaning-as-policy-shortcut",
    "held.self.llm-output-as-inner-owner",
  ] {
    assert!(
      ids.contains(expected),
      "missing self-knowledge candidate `{expected}`"
    );
  }
  for candidate in candidates {
    assert!(!as_bool(get(candidate, "accepted")));
  }

  let held_count = candidates
    .iter()
    .filter(|candidate| as_str(get(candidate, "status")) == "Held")
    .count();
  assert_eq!(held_count, 2);
}

#[test]
fn runtime_observation_is_candidate_only_and_not_installed() {
  let run = eval_file(&fixture_path()).unwrap();
  let runtime = get(&run, "self-knowledge-runtime-observation");
  assert_eq!(
    as_str(get(runtime, "observation-model")),
    "semantic-self-knowledge-candidate-runtime"
  );
  assert!(as_bool(get(runtime, "can-appear-at-runtime")));
  assert!(!as_bool(get(runtime, "canonical-runtime-installed")));
  assert!(!as_bool(get(runtime, "executable-now")));
  assert!(as_bool(get(runtime, "candidate-only")));
  assert!(!as_bool(get(runtime, "owner-switch")));

  let additions = as_list(get(runtime, "runtime-added-candidates"));
  assert_eq!(additions.len(), 4);
  for addition in additions {
    assert_eq!(as_str(get(addition, "status")), "candidate");
    assert!(!as_bool(get(addition, "installed")));
  }
}

#[test]
fn blocked_shortcuts_and_negative_held_evidence_are_explicit() {
  let run = eval_file(&fixture_path()).unwrap();
  let blocks = string_set(get(&run, "blocked-shortcuts"));
  for expected in [
    "treat-user-directive-as-accepted-policy",
    "treat-llm-verbalization-as-inner-lifecycle-owner",
    "install-self-knowledge-runtime-without-replay",
    "promote-meaning-candidate-without-negative-proof",
    "drop-provenance-to-make-knowledge-look-complete",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }

  let held = get(&run, "negative-held-evidence");
  assert_eq!(as_str(get(held, "status")), "present");
  let held_if = string_set(get(held, "held-if"));
  for expected in [
    "meaning-provenance-missing",
    "knowledge-replay-missing",
    "owner-proof-missing",
    "llm-output-treated-as-authority",
    "runtime-install-requested",
  ] {
    assert!(held_if.contains(expected), "missing held-if `{expected}`");
  }
}

#[test]
fn discoveries_record_meaning_and_self_knowledge_as_design_candidates() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = as_list(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 5);
  let ids: BTreeSet<&str> = discoveries
    .iter()
    .map(|discovery| as_str(get(discovery, "id")))
    .collect();
  for expected in [
    "D38.meaning-interpretation-is-first-class-tesseract-input",
    "D39.strong-push-means-more-probes-not-auto-promotion",
    "D40.self-knowledgeization-emits-candidate-knowledge-atoms",
    "D41.llm-is-proposal-surface-not-inner-lifecycle-owner",
    "D42.self-knowledge-runtime-functions-remain-candidate-only",
  ] {
    assert!(ids.contains(expected), "missing discovery `{expected}`");
  }
}

#[test]
fn affected_plans_remain_non_implementation_targets() {
  let run = eval_file(&fixture_path()).unwrap();
  let affected = get(&run, "affected-plans");
  for key in [
    "meaningInterpretation",
    "selfKnowledgeization",
    "llmAdapters",
    "runtimeAdditions",
  ] {
    let plan = get(affected, key);
    assert!(!as_bool(get(plan, "implementation-target")));
  }
  assert_eq!(
    as_str(get_path(affected, &["meaningInterpretation", "pressure"])),
    "strong-keep"
  );
  assert_eq!(
    as_str(get_path(affected, &["llmAdapters", "pressure"])),
    "demote-from-lifecycle-owner"
  );
  assert!(!as_bool(get(&run, "owner-switch")));
  assert!(!as_bool(get(&run, "implementation-command")));
}
