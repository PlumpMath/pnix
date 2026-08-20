//! Peer-treaty owner boundary discovery.
//!
//! OWNER-LAW SUPERSEDE NOTE (2026-05-10):
//!
//! The fixture this test pins encodes a "PNIX/LLM/human peer-treaty
//! (3-peer)" framing where LLM/tool cognition is an active-candidate
//! peer. That framing is REJECTED by the owner-law constitution
//! (CLAUDE.md "OWNER-LAW CONSTITUTION"): pnix design has *no LLM
//! seat*. This test is preserved as a historical regression check on
//! the frozen receipt body, not as current owner-law. Do not cite
//! "PNIX/LLM/human peer treaty" as authoritative.
//!
//! (Original docstring retained below for historical context only:)
//! PNIX independence is not LLM demotion, LLM trust authority, passive-clue
//! neutering, or human replacement. PNIX, LLM/tool cognition, and human
//! consequence ownership form peer domains with a symmetric candidate boundary:
//! LLM output can be active candidate evidence without becoming authority.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join(
    "../../fixtures/tesseract-macro-legacy-probe/peer_treaty_owner_boundary_discovery_receipt.px",
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
fn peer_treaty_marker_and_owner_are_pinned() {
  let run = eval_file(&fixture_path()).expect("peer treaty fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-peer-treaty-owner-boundary"
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
fn constitution_gate_keeps_peer_treaty_candidate_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(as_str(get(gate, "scenario")), "peer-treaty-owner-boundary");
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  assert_eq!(as_str(get(gate, "replacement-readiness")), "not-proven");

  let roles = string_set(get(gate, "output-roles"));
  for expected in [
    "peer.owner.*",
    "treaty.boundary.*",
    "need.peer.*",
    "held.peer.*",
    "runtime.peer-treaty.*.candidate",
  ] {
    assert!(roles.contains(expected), "missing output role `{expected}`");
  }
}

#[test]
fn three_peer_owners_have_equal_status_and_distinct_domains() {
  let run = eval_file(&fixture_path()).unwrap();
  let peers = get(&run, "peer-owners");

  assert_eq!(
    as_str(get_path(peers, &["pnix", "owner-domain"])),
    "judgement-lifecycle"
  );
  assert_eq!(
    as_str(get_path(peers, &["llm", "owner-domain"])),
    "perception-pattern-fluency-active-candidate"
  );
  assert_eq!(
    as_str(get_path(peers, &["human", "owner-domain"])),
    "consequence-and-risk-budget"
  );

  for peer in ["pnix", "llm", "human"] {
    assert_eq!(as_str(get_path(peers, &[peer, "peer-status"])), "equal");
    assert!(as_bool(get_path(peers, &[peer, "independence"])));
    assert!(!as_bool(get_path(peers, &[peer, "supremacy"])));
  }
  assert!(as_bool(get_path(
    peers,
    &["llm", "can-emit-active-candidates"]
  )));
  assert!(!as_bool(get_path(peers, &["llm", "passive-clue-only"])));
  assert!(!as_bool(get_path(peers, &["llm", "trust-authority"])));
}

#[test]
fn pnix_llm_and_human_cannot_own_each_other_domains() {
  let run = eval_file(&fixture_path()).unwrap();
  let peers = get(&run, "peer-owners");

  let pnix_cannot = string_set(get_path(peers, &["pnix", "cannot-own"]));
  assert!(pnix_cannot.contains("natural-language-fluency-final-word"));
  assert!(pnix_cannot.contains("human-consequence-budget"));

  let llm_cannot = string_set(get_path(peers, &["llm", "cannot-own"]));
  assert!(llm_cannot.contains("canonical-accepted-status"));
  assert!(llm_cannot.contains("canonical-promotion"));
  assert!(llm_cannot.contains("owner-law-consistency"));
  assert!(llm_cannot.contains("project-calibration-authority"));
  assert!(llm_cannot.contains("self-report-authority"));
  assert!(as_list(get_path(peers, &["llm", "canonical-final-word-over"])).is_empty());
  let llm_surfaces = string_set(get_path(peers, &["llm", "active-surface-over"]));
  assert!(llm_surfaces.contains("natural-language-naturalness-clue"));
  assert!(llm_surfaces.contains("pattern-proposal"));
  let llm_participation = string_set(get_path(peers, &["llm", "lifecycle-participation-over"]));
  assert!(llm_participation.contains("candidate-turn-seed"));
  assert!(llm_participation.contains("repair-proposal"));
  assert!(llm_participation.contains("counterexample-proposal"));
  assert!(llm_participation.contains("sandbox-experiment-seed"));

  let human_cannot = string_set(get_path(peers, &["human", "cannot-own"]));
  assert!(human_cannot.contains("all-candidate-manual-review-at-scale"));
  assert!(human_cannot.contains("byte-equal-replay-by-attention"));
}

#[test]
fn treaty_rules_encode_no_master_servant_and_symmetric_candidates() {
  let run = eval_file(&fixture_path()).unwrap();
  let rules = attrs_by_id(get(&run, "treaty-rules"));
  assert_eq!(rules.len(), 6);
  for expected in [
    "treaty.symmetric-candidate-boundary",
    "treaty.domain-final-word",
    "treaty.llm-output-active-candidate-not-authority",
    "treaty.no-passive-clue-neutering",
    "treaty.no-master-servant-framing",
    "treaty.human-risk-budget-not-machine-owned",
  ] {
    let rule = rules
      .get(expected)
      .unwrap_or_else(|| panic!("missing rule `{expected}`"));
    assert_eq!(as_str(get(rule, "status")), "candidate");
    assert!(!as_bool(get(rule, "accepted")));
  }
}

#[test]
fn cross_domain_examples_are_candidate_only_with_domain_final_owner() {
  let run = eval_file(&fixture_path()).unwrap();
  let examples = attrs_by_id(get(&run, "cross-domain-examples"));
  assert_eq!(examples.len(), 3);

  let llm_to_pnix = examples.get("cross.llm-to-pnix").unwrap();
  assert_eq!(as_str(get(llm_to_pnix, "required-status")), "candidate");
  assert!(as_bool(get(llm_to_pnix, "lifecycle-participation")));
  assert_eq!(
    as_str(get(llm_to_pnix, "output-effect")),
    "candidate-or-held-pressure"
  );
  assert_eq!(as_str(get(llm_to_pnix, "final-owner")), "pnix");
  assert!(!as_bool(get(llm_to_pnix, "accepted")));

  let pnix_to_llm = examples.get("cross.pnix-to-llm").unwrap();
  assert_eq!(as_str(get(pnix_to_llm, "required-status")), "candidate");
  assert_eq!(
    as_str(get(pnix_to_llm, "final-owner")),
    "llm-active-candidate-responder"
  );
  assert_eq!(
    as_str(get(pnix_to_llm, "output-authority")),
    "active-candidate-evidence"
  );
  assert!(!as_bool(get(pnix_to_llm, "passive-clue-only")));
  assert!(!as_bool(get(pnix_to_llm, "trust-authority")));

  let machine_to_human = examples.get("cross.machine-to-human").unwrap();
  assert_eq!(
    as_str(get(machine_to_human, "required-status")),
    "candidate"
  );
  assert_eq!(as_str(get(machine_to_human, "final-owner")), "human");
}

#[test]
fn independence_invariant_blocks_hierarchy_and_machine_consequence_ownership() {
  let run = eval_file(&fixture_path()).unwrap();
  let inv = get(&run, "independence-invariant");
  assert!(as_bool(get(inv, "pnix-independent-without-llm")));
  assert!(as_bool(get(inv, "llm-not-demoted")));
  assert!(as_bool(get(inv, "llm-active-candidate-power")));
  assert!(!as_bool(get(inv, "llm-passive-clue-only")));
  assert!(!as_bool(get(inv, "llm-canonical-promotion-authority")));
  assert!(as_bool(get(inv, "human-not-rubber-stamp")));
  assert!(as_bool(get(inv, "peer-treaty-required")));
  assert!(as_bool(get(inv, "symmetric-candidate-boundary")));
  assert!(!as_bool(get(inv, "llm-output-trust-authority")));
  assert!(!as_bool(get(inv, "llm-self-report-authority")));
  assert!(!as_bool(get(inv, "llm-project-calibration-authority")));
  assert!(!as_bool(get(inv, "machine-consequence-owner")));
  assert!(!as_bool(get(inv, "pnix-master-over-llm")));
  assert!(!as_bool(get(inv, "llm-master-over-pnix")));
}

#[test]
fn six_layer_treaty_fold_preserves_owner_boundaries() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-treaty-fold");
  assert_eq!(as_str(get(fold, "mode")), "peer-treaty-owner-boundary");
  for layer in [
    "surface", "ontology", "semantic", "gate", "runtime", "audit",
  ] {
    assert!(
      as_bool(get_path(fold, &[layer, "visible"])),
      "layer `{layer}` must be visible"
    );
  }
  assert_eq!(
    as_number(get_path(fold, &["semantic", "peer-owner-count"])),
    3.0
  );
  assert_eq!(
    as_str(get_path(fold, &["semantic", "derived-meaning"])),
    "independence means peer-domain autonomy where LLM output has active candidate power, not passive clue status or trust authority"
  );
  assert!(!as_bool(get_path(fold, &["runtime", "executable-now"])));
  assert!(as_bool(get_path(
    fold,
    &["audit", "treaty-drift-check-required"]
  )));
}

#[test]
fn peer_needs_and_held_entries_capture_failure_modes() {
  let run = eval_file(&fixture_path()).unwrap();
  let entries = as_list(get(&run, "peer-needs-and-held"));
  assert_eq!(entries.len(), 9);
  let ids: BTreeSet<&str> = entries
    .iter()
    .map(|entry| as_str(get(entry, "id")))
    .collect();
  for expected in [
    "need.peer.domain-boundary-contract",
    "need.peer.symmetric-candidate-protocol",
    "need.peer.llm-active-candidate-provenance",
    "need.peer.human-attention-budget",
    "held.peer.llm-demotion-framing",
    "held.peer.llm-passive-clue-neutering",
    "held.peer.llm-trust-authority-framing",
    "held.peer.pnix-master-framing",
    "held.peer.human-rubber-stamp",
  ] {
    assert!(ids.contains(expected), "missing entry `{expected}`");
  }
  for entry in entries {
    assert!(!as_bool(get(entry, "accepted")));
  }
}

#[test]
fn runtime_observation_stays_candidate_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let runtime = get(&run, "runtime-observation");
  assert_eq!(
    as_str(get(runtime, "observation-model")),
    "peer-treaty-boundary-runtime-candidates"
  );
  assert!(as_bool(get(runtime, "can-appear-at-runtime")));
  assert!(!as_bool(get(runtime, "canonical-runtime-installed")));
  assert!(!as_bool(get(runtime, "executable-now")));
  assert!(as_bool(get(runtime, "candidate-only")));
  assert!(!as_bool(get(runtime, "owner-switch")));
  assert_eq!(as_list(get(runtime, "runtime-added-candidates")).len(), 6);
}

#[test]
fn discoveries_record_peer_treaty_correction() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = as_list(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 7);
  let ids: BTreeSet<&str> = discoveries
    .iter()
    .map(|discovery| as_str(get(discovery, "id")))
    .collect();
  for expected in [
    "D49.pnix-independence-is-peer-autonomy-not-hierarchy",
    "D50.llm-domain-authority-must-not-be-demoted",
    "D51.human-consequence-owner-closes-risk-budget-frontier",
    "D52.cross-domain-interface-is-symmetric-candidate-only",
    "D53.peer-framing-drift-is-held",
    "D54.llm-output-is-active-candidate-not-authority",
    "D55.passive-clue-framing-neuters-the-loop",
  ] {
    assert!(ids.contains(expected), "missing discovery `{expected}`");
  }
}

#[test]
fn affected_plans_and_blocks_prevent_peer_demotion_or_supremacy() {
  let run = eval_file(&fixture_path()).unwrap();
  let affected = get(&run, "affected-plans");
  for key in ["pnix", "llm", "human", "integration"] {
    assert!(!as_bool(get_path(
      affected,
      &[key, "implementation-target"]
    )));
  }
  assert_eq!(
    as_str(get_path(affected, &["llm", "pressure"])),
    "keep-active-candidate-peer-not-authority"
  );
  assert_eq!(
    as_str(get_path(affected, &["integration", "pressure"])),
    "redesign"
  );

  let blocks = string_set(get(&run, "blocked-shortcuts"));
  for expected in [
    "treat-pnix-as-master-over-llm",
    "treat-llm-as-servant-peripheral",
    "reduce-llm-output-to-passive-clue",
    "trust-llm-self-report-as-authority",
    "trust-llm-project-calibration-as-authority",
    "treat-human-as-rubber-stamp",
    "let-llm-own-canonical-promotion",
    "let-pnix-own-llm-perception-domain",
    "let-machine-own-consequence-budget",
  ] {
    assert!(blocks.contains(expected), "missing block `{expected}`");
  }
}
