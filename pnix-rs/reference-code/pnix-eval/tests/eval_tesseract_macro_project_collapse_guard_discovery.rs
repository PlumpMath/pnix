//! Project-collapse guard discovery.
//!
//! OWNER-LAW SUPERSEDE NOTE (2026-05-10):
//!
//! The fixture this test pins claims "LLM/tool cognition is the
//! primary semantic input organ" as the contribution-axis organ.
//! That framing is REJECTED by the owner-law constitution (CLAUDE.md
//! "OWNER-LAW CONSTITUTION"): pnix design has *no LLM seat*. The
//! contribution organ is owner-internal evidence (user utterance /
//! `.px` owner / solver output / external schema lifted into
//! ContextualFact / KnowledgeRecord). This test is preserved as a
//! historical regression check on the frozen receipt body, not as
//! current owner-law. Do not cite "LLM as primary semantic input
//! organ" as authoritative.
//!
//! (Original docstring retained below for historical context only:)
//! This test pins the stability rule that contribution/substance and
//! authority/promotion are orthogonal axes. LLM/tool cognition can be the
//! primary semantic input organ and still have zero canonical promotion
//! authority. Claude/Codex generated code is a paper-note candidate, not PNIX
//! self. PNIX owns deterministic lifecycle promotion. Humans own consequence
//! /risk budget. Collapsing those axes is a Held failure mode.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base)
    .join("../../fixtures/tesseract-macro-legacy-probe/project_collapse_guard_discovery_receipt.px")
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
fn project_collapse_guard_marker_and_owner_are_pinned() {
  let run = eval_file(&fixture_path()).expect("project collapse guard fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-project-collapse-guard"
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
fn constitution_gate_blocks_collapse_shortcuts_without_acceptance() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(as_str(get(gate, "scenario")), "project-collapse-guard");
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  assert_eq!(as_str(get(gate, "replacement-readiness")), "not-proven");

  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "turn-llm-output-into-accepted-authority",
    "strip-llm-substance-because-no-authority",
    "claim-readiness-from-green-tests-alone",
    "treat-constructor-code-as-pnix-self",
    "merge-paper-note-code-without-fold-proof",
    "add-nix-checks-gate-for-this-slice",
  ] {
    assert!(blocks.contains(expected), "missing gate block `{expected}`");
  }
}

#[test]
fn contribution_and_authority_axes_are_explicitly_orthogonal() {
  let run = eval_file(&fixture_path()).unwrap();
  let axis = get(&run, "axis-model");
  assert!(as_bool(get(axis, "axis-separation-required")));

  let forbidden = string_set(get(axis, "forbidden-axis-collapses"));
  for expected in [
    "no-authority-means-no-substance",
    "substance-means-authority",
    "lifecycle-means-master",
    "consequence-owner-means-rubber-stamp",
  ] {
    assert!(
      forbidden.contains(expected),
      "missing forbidden collapse `{expected}`"
    );
  }
}

#[test]
fn llm_pnix_and_human_are_all_load_bearing_contributors() {
  let run = eval_file(&fixture_path()).unwrap();
  let contribution = get_path(&run, &["axis-model", "contribution-axis"]);

  assert_eq!(
    as_str(get_path(contribution, &["llm", "role"])),
    "primary-semantic-input-organ"
  );
  assert!(as_bool(get_path(contribution, &["llm", "load-bearing"])));
  let llm_contribution = string_set(get_path(contribution, &["llm", "contribution"]));
  for expected in [
    "semantic-substance",
    "perception-pattern-fluency",
    "candidate-turn-seed",
    "repair-proposal",
    "counterexample-proposal",
    "sandbox-experiment-seed",
  ] {
    assert!(
      llm_contribution.contains(expected),
      "missing LLM contribution `{expected}`"
    );
  }

  assert!(as_bool(get_path(contribution, &["pnix", "load-bearing"])));
  assert!(as_bool(get_path(contribution, &["human", "load-bearing"])));
}

#[test]
fn authority_axis_keeps_llm_canonical_authority_empty() {
  let run = eval_file(&fixture_path()).unwrap();
  let authority = get_path(&run, &["axis-model", "authority-axis"]);
  assert!(as_list(get(authority, "llm")).is_empty());

  let pnix_auth = string_set(get(authority, "pnix"));
  assert!(pnix_auth.contains("canonical-promotion"));
  assert!(pnix_auth.contains("owner-law-consistency"));

  let human_auth = string_set(get(authority, "human"));
  assert!(human_auth.contains("consequence-bearing-choice"));
  assert!(human_auth.contains("risk-budget"));
}

#[test]
fn collapse_modes_cover_both_llm_neutering_and_llm_trust() {
  let run = eval_file(&fixture_path()).unwrap();
  let modes = attrs_by_id(get(&run, "project-collapse-modes"));
  assert_eq!(modes.len(), 9);
  for expected in [
    "collapse.no-authority-means-no-substance",
    "collapse.substance-means-authority",
    "collapse.pnix-master-system",
    "collapse.human-rubber-stamp",
    "collapse.wiki-as-runtime-truth",
    "collapse.green-test-theater",
    "collapse.stale-plan-autopilot",
    "collapse.possibility-minimizing-agent-prose",
    "collapse.constructor-code-smuggling",
  ] {
    let mode = modes
      .get(expected)
      .unwrap_or_else(|| panic!("missing mode `{expected}`"));
    assert!(as_bool(get(mode, "held")));
  }
}

#[test]
fn guard_devices_are_candidate_receipts_not_nix_checks() {
  let run = eval_file(&fixture_path()).unwrap();
  let guards = attrs_by_id(get(&run, "guard-devices"));
  assert_eq!(guards.len(), 7);
  for expected in [
    "guard.axis-separation-lint",
    "guard.load-bearing-triad",
    "guard.receipt-over-agent-prose",
    "guard.discovery-before-design",
    "guard.intent-receipt-before-green",
    "guard.no-nix-checks-gate-for-this-slice",
    "guard.constructor-paper-note-firewall",
  ] {
    let guard = guards
      .get(expected)
      .unwrap_or_else(|| panic!("missing guard `{expected}`"));
    assert_eq!(as_str(get(guard, "status")), "candidate");
    assert!(!as_bool(get(guard, "implementation-target")));
  }
}

#[test]
fn stability_held_records_project_collapse_failures() {
  let run = eval_file(&fixture_path()).unwrap();
  let held = attrs_by_id(get(&run, "stability-held"));
  assert_eq!(held.len(), 6);
  for expected in [
    "held.collapse.axis-collapsed",
    "held.collapse.llm-neutered",
    "held.collapse.llm-trusted",
    "held.collapse.green-with-wrong-intent",
    "held.collapse.wiki-runtime-truth",
    "held.collapse.constructor-code-smuggling",
  ] {
    let entry = held
      .get(expected)
      .unwrap_or_else(|| panic!("missing Held `{expected}`"));
    assert_eq!(as_str(get(entry, "status")), "Held");
    assert!(!as_bool(get(entry, "accepted")));
  }
}

#[test]
fn six_layer_guard_fold_preserves_intent_and_anti_collapse_eval() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-guard-fold");
  assert_eq!(as_str(get(fold, "mode")), "project-collapse-guard");
  for layer in [
    "surface", "ontology", "semantic", "gate", "runtime", "audit",
  ] {
    assert!(
      as_bool(get_path(fold, &[layer, "visible"])),
      "layer `{layer}` must be visible"
    );
  }
  assert_eq!(
    as_str(get_path(fold, &["semantic", "derived-meaning"])),
    "LLM contribution can be 100 percent load-bearing while LLM canonical authority remains zero"
  );
  assert_eq!(
    as_number(get_path(fold, &["semantic", "collapse-mode-count"])),
    9.0
  );
  assert_eq!(
    as_number(get_path(fold, &["semantic", "guard-device-count"])),
    7.0
  );
  assert!(as_bool(get_path(
    fold,
    &["audit", "intent-receipt-required"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["audit", "anti-collapse-eval-required"]
  )));
}

#[test]
fn runtime_observation_is_candidate_only_and_does_not_add_nix_gate() {
  let run = eval_file(&fixture_path()).unwrap();
  let runtime = get(&run, "runtime-observation");
  assert_eq!(
    as_str(get(runtime, "observation-model")),
    "project-collapse-guard-runtime-candidates"
  );
  assert!(!as_bool(get(runtime, "canonical-runtime-installed")));
  assert!(!as_bool(get(runtime, "executable-now")));
  assert!(as_bool(get(runtime, "candidate-only")));
  assert!(!as_bool(get(runtime, "owner-switch")));
  assert!(!as_bool(get(runtime, "nix-checks-gate-added")));
  assert_eq!(as_list(get(runtime, "runtime-added-candidates")).len(), 5);
}

#[test]
fn discoveries_record_stability_plan_without_lowering_possibility() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 9);
  for expected in [
    "D56.authority-contribution-axes-are-orthogonal",
    "D57.llm-is-primary-semantic-input-organ",
    "D58.no-authority-no-substance-is-collapse",
    "D59.substance-as-authority-is-collapse",
    "D60.receipt-over-agent-prose",
    "D61.intent-receipt-before-green-tests",
    "D62.discovery-before-design-prevents-old-plan-autopilot",
    "D63.no-nix-checks-gate-for-collapse-guard-slice",
    "D64.constructor-code-is-paper-note-candidate",
  ] {
    let discovery = discoveries
      .get(expected)
      .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
    assert!(as_bool(get(discovery, "scenario-only")));
  }
}

#[test]
fn affected_plans_and_blocks_keep_stability_guard_scoped() {
  let run = eval_file(&fixture_path()).unwrap();
  let affected = get(&run, "affected-plans");
  assert_eq!(
    as_str(get_path(affected, &["llm", "pressure"])),
    "keep-load-bearing-contribution-without-authority"
  );
  assert_eq!(
    as_str(get_path(affected, &["project", "pressure"])),
    "add-guard-receipts-not-nix-checks"
  );
  assert_eq!(
    as_str(get_path(affected, &["constructor", "pressure"])),
    "quarantine-code-until-fold-proof"
  );
  for key in ["llm", "pnix", "human", "project", "constructor"] {
    assert!(!as_bool(get_path(
      affected,
      &[key, "implementation-target"]
    )));
  }

  let blocks = string_set(get(&run, "blocked-shortcuts"));
  for expected in [
    "turn-llm-output-into-accepted-authority",
    "strip-llm-substance-because-no-authority",
    "treat-pnix-as-master-system",
    "treat-human-as-approval-button",
    "claim-readiness-from-green-tests-alone",
    "treat-constructor-code-as-pnix-self",
    "merge-paper-note-code-without-fold-proof",
    "add-nix-checks-gate-for-this-slice",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }

  assert_eq!(as_str(get(&run, "replacement-readiness")), "not-proven");
  assert!(!as_bool(get(&run, "owner-switch")));
  assert!(!as_bool(get(&run, "implementation-command")));
}
