//! Surface-scoped replacement readiness for the macro-native promote candidate.
//!
//! R5 verified reverse replay for the R4 candidate. This test pins the next
//! boundary: a readiness receipt may aggregate R2-R5 evidence and open R6
//! owner-switch review, but it still cannot switch owners, install runtime
//! behavior, globalize readiness, or delete/archive legacy code.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join(
    "../../fixtures/tesseract-macro-legacy-probe/replacement_readiness_promote_candidate_receipt.px",
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

fn attrs_by_id<'a>(items: &'a Value) -> BTreeMap<&'a str, &'a Value> {
  as_list(items)
    .iter()
    .map(|item| (as_str(get(item, "id")), item))
    .collect()
}

#[test]
fn readiness_marker_and_constitution_owner_are_pinned() {
  let run = eval_file(&fixture_path()).expect("readiness fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-surface-replacement-readiness-promote-candidate"
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
fn constitution_gate_allows_readiness_without_acceptance() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "surface-scoped-replacement-readiness-promote-candidate"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  assert_eq!(
    as_str(get(gate, "replacement-readiness")),
    "ready-for-r6-owner-switch-receipt"
  );

  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "treat-readiness-as-owner-switch",
    "treat-readiness-as-runtime-install",
    "claim-global-ontology-readiness",
    "skip-r5-reverse-replay",
    "ignore-uncovered-delta",
    "drop-negative-held-proof",
    "drop-audit-ref",
    "emit-legacy-Accepted-as-current-proof",
    "delete-or-archive-legacy-surface-from-readiness",
    "treat-llm-prose-as-readiness",
  ] {
    assert!(blocks.contains(expected), "missing gate block `{expected}`");
  }
}

#[test]
fn readiness_is_surface_scoped_to_legacy_promote() {
  let run = eval_file(&fixture_path()).unwrap();
  let surface = get(&run, "surface");
  assert_eq!(
    as_str(get(surface, "legacy")),
    "stdlib/lib/ontology.px::builtins.ontologyPromote"
  );
  assert_eq!(
    as_str(get(surface, "candidate")),
    "r4.macro-native-promote.rewrite-candidate"
  );
  assert_eq!(
    as_str(get(surface, "scope")),
    "this-one-legacy-promote-surface-only"
  );
  assert!(!as_bool(get(surface, "global-ontology-runtime")));
}

#[test]
fn evidence_bundle_imports_r5_receipt_state() {
  let run = eval_file(&fixture_path()).unwrap();
  let evidence = get(&run, "evidence-bundle");
  assert_eq!(
    as_str(get(evidence, "legacy-specimen")),
    "legacy-replay-specimen.promote.accepted"
  );
  assert_eq!(
    as_str(get(evidence, "r4-candidate")),
    "r4.macro-native-promote.rewrite-candidate"
  );
  assert_eq!(
    as_str(get(evidence, "r5-verdict")),
    "reverse-replay-verified"
  );
  assert!(as_bool(get(evidence, "all-deltas-covered")));
  assert!(!as_bool(get(evidence, "unexplained-mismatch")));
  assert!(as_bool(get(evidence, "audit-refs-preserved")));
  assert!(as_bool(get(evidence, "negative-held-proof-present")));
  assert!(!as_bool(get(evidence, "reverse-turn-instance")));
  assert_eq!(as_str(get(evidence, "replay-kind")), "reverse-replay");
}

#[test]
fn all_readiness_criteria_are_satisfied_without_owner_switch() {
  let run = eval_file(&fixture_path()).unwrap();
  let criteria = attrs_by_id(get(&run, "readiness-criteria"));
  assert_eq!(criteria.len(), 9);
  for expected in [
    "criteria.six-layers-visible",
    "criteria.legacy-authority-blocked",
    "criteria.candidate-boundary-preserved",
    "criteria.role-emitted-by-fold",
    "criteria.negative-path-present",
    "criteria.reference-delta-covered",
    "criteria.reverse-replay-present",
    "criteria.runtime-route-proof-non-executable",
    "criteria.docs-and-discovery-recorded",
  ] {
    let item = criteria
      .get(expected)
      .unwrap_or_else(|| panic!("missing criteria `{expected}`"));
    assert!(as_bool(get(item, "satisfied")));
    assert_eq!(as_str(get(item, "verdict")), "satisfied");
  }
}

#[test]
fn runtime_route_proof_is_non_executable() {
  let run = eval_file(&fixture_path()).unwrap();
  let proof = get(&run, "runtime-route-proof");
  assert_eq!(
    as_str(get(proof, "proof-kind")),
    "non-executable-route-proof"
  );
  assert_eq!(
    as_str(get(proof, "verdict")),
    "runtime-route-proof-candidate-verified"
  );
  assert!(!as_bool(get(proof, "installed")));
  assert!(!as_bool(get(proof, "executable-now")));
  assert!(!as_bool(get(proof, "owner-switch")));
}

#[test]
fn owner_law_readiness_opens_r6_review_not_owner_switch() {
  let run = eval_file(&fixture_path()).unwrap();
  let owner = get(&run, "owner-law-readiness");
  assert!(as_bool(get(owner, "all-criteria-satisfied")));
  assert_eq!(as_str(get(owner, "owner-law-gate")), "ready-for-r6-review");
  assert_eq!(
    as_str(get(owner, "verdict")),
    "owner-law-ready-for-r6-owner-switch-receipt"
  );
  assert!(!as_bool(get(owner, "accepted")));
  assert!(!as_bool(get(owner, "owner-switch")));
}

#[test]
fn readiness_verdict_does_not_install_switch_delete_or_archive() {
  let run = eval_file(&fixture_path()).unwrap();
  let verdict = get(&run, "readiness-verdict");
  assert_eq!(
    as_str(get(verdict, "readiness")),
    "ready-for-r6-owner-switch-receipt"
  );
  assert_eq!(
    as_str(get(verdict, "scope")),
    "this-one-legacy-promote-surface-only"
  );
  assert!(!as_bool(get(verdict, "owner-switch")));
  assert!(!as_bool(get(verdict, "runtime-install")));
  assert!(!as_bool(get(verdict, "global-ontology-runtime")));
  assert!(!as_bool(get(verdict, "delete-legacy-surface")));
  assert!(!as_bool(get(verdict, "archive-legacy-surface")));

  let required = string_set(get(verdict, "next-required"));
  for expected in [
    "r6-owner-switch-receipt",
    "human-consequence-authorization-if-consequence-bearing",
    "compat-or-archive-decision-after-owner-switch",
  ] {
    assert!(
      required.contains(expected),
      "missing next requirement `{expected}`"
    );
  }
}

#[test]
fn held_trials_block_readiness_shortcuts() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "held-readiness-trials"));
  assert_eq!(trials.len(), 7);
  for expected in [
    "trial.A.reverse-replay-not-verified",
    "trial.B.uncovered-delta",
    "trial.C.audit-ref-missing",
    "trial.D.runtime-route-proof-missing",
    "trial.E.owner-switch-requested",
    "trial.F.global-readiness-requested",
  ] {
    let trial = trials
      .get(expected)
      .unwrap_or_else(|| panic!("missing trial `{expected}`"));
    assert_eq!(as_str(get(trial, "outcome")), "Held");
    assert!(!as_bool(get(trial, "owner-switch")));
  }
  let complete = trials.get("trial.G.complete-readiness").unwrap();
  assert_eq!(
    as_str(get(complete, "outcome")),
    "ready-for-r6-owner-switch-receipt"
  );
  assert!(!as_bool(get(complete, "owner-switch")));
}

#[test]
fn six_layer_readiness_fold_preserves_boundaries() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-readiness-fold");
  assert_eq!(
    as_str(get(fold, "mode")),
    "surface-scoped-replacement-readiness-promote-candidate"
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
    as_str(get_path(fold, &["ontology", "readiness-scope"])),
    "surface-scoped"
  );
  assert!(!as_bool(get_path(
    fold,
    &["ontology", "global-ontology-runtime"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["semantic", "readiness-is-owner-switch"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["semantic", "readiness-is-runtime-install"]
  )));
  assert!(!as_bool(get_path(fold, &["runtime", "executable-now"])));
  assert!(!as_bool(get_path(fold, &["runtime", "installed"])));
  assert!(!as_bool(get_path(fold, &["runtime", "owner-switch"])));
  assert!(as_bool(get_path(fold, &["audit", "audit-refs-preserved"])));
}

#[test]
fn runtime_observation_is_candidate_only_and_not_installed() {
  let run = eval_file(&fixture_path()).unwrap();
  let runtime = get(&run, "runtime-observation");
  assert_eq!(
    as_str(get(runtime, "observation-model")),
    "readiness-runtime-candidates"
  );
  assert!(as_bool(get(runtime, "can-appear-at-runtime")));
  assert!(as_bool(get(runtime, "candidate-only")));
  assert!(!as_bool(get(runtime, "canonical-runtime-installed")));
  assert!(!as_bool(get(runtime, "executable-now")));
  assert!(!as_bool(get(runtime, "owner-switch")));
  assert_eq!(as_list(get(runtime, "runtime-added-candidates")).len(), 2);
}

#[test]
fn discoveries_record_d108_through_d115() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 8);
  for expected in [
    "D108.readiness-is-surface-scoped-not-global",
    "D109.readiness-aggregates-r2-through-r5-receipts",
    "D110.owner-law-readiness-is-not-owner-switch",
    "D111.runtime-route-proof-is-non-executable",
    "D112.readiness-blocks-global-readiness-and-delete-archive-shortcuts",
    "D113.readiness-preserves-negative-held-as-future-regression-guard",
    "D114.readiness-can-emit-r6-need-without-switching",
    "D115.readiness-preserves-pnix-independence-from-llm-claims",
  ] {
    let d = discoveries
      .get(expected)
      .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
    assert!(as_bool(get(d, "scenario-only")));
  }
}

#[test]
fn affected_plans_remain_non_implementation_targets() {
  let run = eval_file(&fixture_path()).unwrap();
  let affected = get(&run, "affected-plans");
  assert_eq!(
    as_str(get_path(affected, &["legacyPromote", "pressure"])),
    "ready-for-r6-owner-switch-receipt"
  );
  assert_eq!(
    as_str(get_path(affected, &["ownerSwitch", "pressure"])),
    "may-start-r6-but-not-claimed-here"
  );
  for key in [
    "legacyPromote",
    "runtimeRoute",
    "ownerSwitch",
    "legacyArchive",
  ] {
    assert!(!as_bool(get_path(
      affected,
      &[key, "implementation-target"]
    )));
  }
}

#[test]
fn negative_held_evidence_blocks_readiness_shortcuts() {
  let run = eval_file(&fixture_path()).unwrap();
  let negative = get(&run, "negative-held-evidence");
  assert_eq!(as_str(get(negative, "status")), "present");
  let rejects = string_set(get(negative, "rejects"));
  for expected in [
    "readiness-without-r5-replay",
    "readiness-with-uncovered-delta",
    "readiness-without-audit-ref",
    "readiness-without-negative-held-proof",
    "owner-switch-inside-readiness",
    "runtime-install-inside-readiness",
    "global-ontology-readiness-from-single-surface",
    "delete-or-archive-from-readiness",
    "llm-prose-as-readiness",
  ] {
    assert!(rejects.contains(expected), "missing rejection `{expected}`");
  }
}

#[test]
fn readiness_receipt_sets_readiness_without_owner_switch_or_command() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "replacement-readiness")),
    "ready-for-r6-owner-switch-receipt"
  );
  assert!(!as_bool(get(&run, "owner-switch")));
  assert!(!as_bool(get(&run, "implementation-command")));
}
