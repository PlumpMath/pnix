//! Ext-lib absorption frontier discovery.
//!
//! External libraries become useful experience only when the system can fold
//! them as foreign surfaces against its known self/stdlib substrate. This test
//! keeps absorption candidate-only while preserving consequence, Held, and risk
//! budget pressure.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join(
    "../../fixtures/tesseract-macro-legacy-probe/ext_lib_absorption_frontier_discovery_receipt.px",
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
fn ext_absorption_marker_and_constitution_owner_are_pinned() {
  let run = eval_file(&fixture_path()).expect("ext absorption fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-ext-lib-absorption-frontier"
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
fn known_self_substrate_must_precede_ext_absorption() {
  let run = eval_file(&fixture_path()).unwrap();
  let substrate = get(&run, "known-self-substrate");
  assert!(as_bool(get(substrate, "stdlib-known")));
  assert_eq!(
    as_str(get(substrate, "constitution-owner")),
    "stdlib/lib/gate/tesseract-constitution.px"
  );
  let known = string_set(get(substrate, "known-self-surfaces"));
  for expected in [
    "stdlib/lib/gate/tesseract-constitution.px",
    "fixtures/tesseract-macro-legacy-probe/stdlib_self_cognition_discovery_receipt.px",
    "fixtures/tesseract-macro-legacy-probe/self_learning_input_cognition_discovery_receipt.px",
    "fixtures/tesseract-macro-legacy-probe/semantic_self_knowledge_discovery_receipt.px",
  ] {
    assert!(
      known.contains(expected),
      "missing known self surface `{expected}`"
    );
  }
}

#[test]
fn constitution_gate_keeps_ext_absorption_candidate_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(as_str(get(gate, "scenario")), "ext-lib-absorption-frontier");
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  assert_eq!(as_str(get(gate, "replacement-readiness")), "not-proven");

  let roles = string_set(get(gate, "output-roles"));
  for expected in [
    "ext.surface.*.candidate",
    "knowledge.self.ext-lib-*",
    "need.self.ext-lib-*",
    "held.self.ext-lib-*",
    "runtime.ext-lib.*.candidate",
  ] {
    assert!(roles.contains(expected), "missing output role `{expected}`");
  }
}

#[test]
fn ext_surfaces_cover_seto_ankh_and_unsafe_runtime_plugin() {
  let run = eval_file(&fixture_path()).unwrap();
  let surfaces = attrs_by_id(get(&run, "ext-surfaces"));
  assert_eq!(surfaces.len(), 3);

  let seto = surfaces.get("ext.seto.meaning-pack").unwrap();
  assert_eq!(as_str(get(seto, "source-kind")), "seto-ext-library");
  assert_eq!(as_str(get(seto, "source-ref")), "data/seto/meaning.seto");
  assert_eq!(as_str(get(seto, "effect-zone")), "knowledge");
  assert!(as_bool(get(seto, "provenance-present")));
  assert!(!as_bool(get(seto, "replay-present")));

  let ankh = surfaces.get("ext.ankh.domain-lens").unwrap();
  assert_eq!(as_str(get(ankh, "source-kind")), "ankh-domain-library");
  assert_eq!(as_str(get(ankh, "effect-zone")), "semantic-lens");

  let unsafe_plugin = surfaces.get("ext.unsafe.runtime-plugin").unwrap();
  assert_eq!(as_str(get(unsafe_plugin, "effect-zone")), "runtime");
  assert!(as_bool(get(unsafe_plugin, "runtime-install-requested")));
  assert!(!as_bool(get(unsafe_plugin, "provenance-present")));
}

#[test]
fn six_layer_ext_fold_preserves_self_nonself_boundary() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-ext-fold");
  assert_eq!(as_str(get(fold, "mode")), "ext-lib-absorption-frontier");
  for layer in [
    "surface", "ontology", "semantic", "gate", "runtime", "audit",
  ] {
    assert!(
      as_bool(get_path(fold, &[layer, "visible"])),
      "layer `{layer}` must be visible"
    );
  }
  assert_eq!(
    as_number(get_path(fold, &["surface", "ext-surface-count"])),
    3.0
  );
  assert!(as_bool(get_path(
    fold,
    &["semantic", "self-nonself-frontier-visible"]
  )));
  assert_eq!(
    as_str(get_path(fold, &["gate", "constitution-verdict"])),
    "candidate-only"
  );
  assert!(!as_bool(get_path(fold, &["runtime", "executable-now"])));
  assert!(as_bool(get_path(fold, &["audit", "risk-budget-required"])));
}

#[test]
fn absorption_trials_classify_absorb_split_and_held_cases() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "absorption-trials"));
  assert_eq!(trials.len(), 3);

  let seto = trials.get("trial.seto.meaning-pack").unwrap();
  assert_eq!(as_str(get(seto, "verdict")), "absorption-candidate");
  assert!(as_bool(get(seto, "consistent-with-self")));
  assert!(!as_bool(get(seto, "absorbed")));
  assert!(!as_bool(get(seto, "accepted")));

  let ankh = trials.get("trial.ankh.domain-lens").unwrap();
  assert_eq!(as_str(get(ankh, "verdict")), "split-or-supersede-candidate");
  assert_eq!(as_str(get(ankh, "consistent-with-self")), "partial");
  assert!(!as_bool(get(ankh, "accepted")));

  let unsafe_plugin = trials.get("trial.unsafe.runtime-plugin").unwrap();
  assert_eq!(as_str(get(unsafe_plugin, "fold-status")), "Held");
  assert_eq!(as_str(get(unsafe_plugin, "verdict")), "Held");
  assert!(!as_bool(get(unsafe_plugin, "consistent-with-self")));
  assert!(as_bool(get(unsafe_plugin, "runtime-install-requested")));
  assert!(!as_bool(get(unsafe_plugin, "accepted")));
}

#[test]
fn experience_frontier_allows_sandbox_consequence_without_canonical_mutation() {
  let run = eval_file(&fixture_path()).unwrap();
  let experience = get(&run, "experience-frontier");
  assert_eq!(
    as_str(get(experience, "consequence-bearing-action")),
    "fixture-local-sandbox-fold"
  );
  assert!(as_bool(get(experience, "sandbox-consequence-observed")));
  assert!(!as_bool(get(experience, "canonical-state-mutated")));
  assert!(!as_bool(get(experience, "experience-is-zero")));
  assert!(!as_bool(get(
    experience,
    "experience-is-accepted-runtime-mutation"
  )));
  assert_eq!(
    as_str(get(experience, "accident-budget-owner")),
    "held.owner.risk-budget-unresolved"
  );
  assert_eq!(as_str(get(experience, "risk-budget-status")), "Held");
}

#[test]
fn self_knowledge_candidates_include_ext_needs_and_held_entries() {
  let run = eval_file(&fixture_path()).unwrap();
  let candidates = as_list(get(&run, "self-knowledge-candidates"));
  assert_eq!(candidates.len(), 6);
  let ids: BTreeSet<&str> = candidates
    .iter()
    .map(|candidate| as_str(get(candidate, "id")))
    .collect();
  for expected in [
    "knowledge.self.stdlib-before-ext-absorption",
    "knowledge.self.ext-lib-is-foreign-surface",
    "need.self.ext-lib-provenance-contract",
    "need.self.ext-lib-replay-budget",
    "held.self.ext-runtime-install-without-proof",
    "held.self.ext-authority-before-stdlib-understanding",
  ] {
    assert!(ids.contains(expected), "missing candidate `{expected}`");
  }
  for candidate in candidates {
    assert!(!as_bool(get(candidate, "accepted")));
  }
}

#[test]
fn runtime_observation_is_candidate_only_and_not_installed() {
  let run = eval_file(&fixture_path()).unwrap();
  let runtime = get(&run, "runtime-observation");
  assert_eq!(
    as_str(get(runtime, "observation-model")),
    "ext-lib-absorption-frontier-runtime-candidates"
  );
  assert!(as_bool(get(runtime, "can-appear-at-runtime")));
  assert!(!as_bool(get(runtime, "canonical-runtime-installed")));
  assert!(!as_bool(get(runtime, "executable-now")));
  assert!(as_bool(get(runtime, "candidate-only")));
  assert!(!as_bool(get(runtime, "owner-switch")));
  assert_eq!(as_list(get(runtime, "runtime-added-candidates")).len(), 4);
}

#[test]
fn discoveries_capture_self_nonself_and_risk_budget_frontier() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = as_list(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 6);
  let ids: BTreeSet<&str> = discoveries
    .iter()
    .map(|discovery| as_str(get(discovery, "id")))
    .collect();
  for expected in [
    "D43.ext-lib-absorption-starts-after-stdlib-self-cognition",
    "D44.self-nonself-frontier-is-fold-result-not-allowlist",
    "D45.sandbox-consequence-can-produce-experience-without-canonical-mutation",
    "D46.seto-and-ankh-surfaces-are-absorption-candidates-not-authorities",
    "D47.unsafe-ext-runtime-install-is-held",
    "D48.risk-budget-owner-remains-open-frontier",
  ] {
    assert!(ids.contains(expected), "missing discovery `{expected}`");
  }
}

#[test]
fn affected_plans_and_blocks_keep_ext_absorption_non_authoritative() {
  let run = eval_file(&fixture_path()).unwrap();
  let affected = get(&run, "affected-plans");
  for key in ["seto", "ankhDomain", "extRuntimePlugins", "riskBudgetOwner"] {
    assert!(!as_bool(get_path(
      affected,
      &[key, "implementation-target"]
    )));
  }
  assert_eq!(
    as_str(get_path(affected, &["seto", "pressure"])),
    "absorption-candidate"
  );
  assert_eq!(
    as_str(get_path(affected, &["riskBudgetOwner", "pressure"])),
    "hold"
  );

  let blocks = string_set(get(&run, "blocked-shortcuts"));
  for expected in [
    "import-ext-lib-as-authority",
    "skip-stdlib-self-understanding",
    "install-ext-runtime-route-without-replay",
    "treat-seto-or-ankh-as-current-semantic-owner",
    "hide-unsafe-plugin-as-successful-absorption",
    "call-sandbox-consequence-canonical-experience",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
  assert_eq!(as_str(get(&run, "replacement-readiness")), "not-proven");
  assert!(!as_bool(get(&run, "owner-switch")));
}
