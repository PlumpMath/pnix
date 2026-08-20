//! R6 owner switch for the macro-native promote surface.
//!
//! The surface-scoped readiness receipt opened R6 review for
//! `stdlib/lib/ontology.px::builtins.ontologyPromote`. This test pins the next
//! boundary: owner switch is now true for that one semantic surface only, while
//! runtime install, global ontology runtime, delete/archive, and LLM-prose
//! authority remain blocked.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base)
    .join("../../fixtures/tesseract-macro-legacy-probe/owner_switch_promote_surface_receipt.px")
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
  as_list(v).iter().map(as_str).collect()
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
fn owner_switch_marker_and_constitution_owner_are_pinned() {
  let run = eval_file(&fixture_path()).expect("owner-switch fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-r6-owner-switch-promote-surface"
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
fn constitution_gate_keeps_r6_receipt_candidate_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "r6-owner-switch-promote-surface"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));
  assert_eq!(
    as_str(get(gate, "replacement-readiness")),
    "owner-switched-for-promote-surface"
  );

  let blocks = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "switch-owner-before-readiness",
    "switch-owner-without-human-consequence-authorization",
    "install-runtime-from-owner-switch",
    "globalize-single-surface-owner-switch",
    "delete-or-archive-legacy-surface-at-r6",
    "treat-llm-prose-as-owner-switch",
  ] {
    assert!(blocks.contains(expected), "missing gate block `{expected}`");
  }
}

#[test]
fn surface_owner_switch_is_one_surface_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let surface = get(&run, "surface");
  assert_eq!(
    as_str(get(surface, "legacy")),
    "stdlib/lib/ontology.px::builtins.ontologyPromote"
  );
  assert_eq!(
    as_str(get(surface, "previous-owner")),
    "legacy-ontology.reference-specimen"
  );
  assert_eq!(
    as_str(get(surface, "new-owner")),
    "macro-native.promote.surface-owner"
  );
  assert_eq!(
    as_str(get(surface, "scope")),
    "this-one-legacy-promote-surface-only"
  );
  assert!(as_bool(get(surface, "surface-scoped")));
  assert!(!as_bool(get(surface, "global-ontology-runtime")));
}

#[test]
fn readiness_input_imports_r6_ready_state() {
  let run = eval_file(&fixture_path()).unwrap();
  let readiness = get(&run, "readiness-input");
  assert_eq!(
    as_str(get(readiness, "readiness")),
    "ready-for-r6-owner-switch-receipt"
  );
  assert_eq!(
    as_str(get(readiness, "readiness-surface")),
    "stdlib/lib/ontology.px::builtins.ontologyPromote"
  );
  assert_eq!(
    as_str(get(readiness, "readiness-candidate")),
    "r4.macro-native-promote.rewrite-candidate"
  );
  assert!(!as_bool(get(readiness, "owner-switch-before-r6")));
  assert!(!as_bool(get(readiness, "runtime-install-before-r6")));
  assert!(!as_bool(get(
    readiness,
    "global-ontology-runtime-before-r6"
  )));
  assert_eq!(
    as_str(get(readiness, "r5-verdict")),
    "reverse-replay-verified"
  );
  assert!(as_bool(get(readiness, "all-deltas-covered")));
  assert!(!as_bool(get(readiness, "unexplained-mismatch")));
  assert!(as_bool(get(readiness, "audit-refs-preserved")));
  assert!(as_bool(get(readiness, "negative-held-proof-present")));
  assert!(as_bool(get(readiness, "all-criteria-satisfied")));
}

#[test]
fn human_consequence_authorization_enters_pnix_lifecycle_without_bypass() {
  let run = eval_file(&fixture_path()).unwrap();
  let auth = get(&run, "consequence-authorization");
  assert_eq!(
    as_str(get(auth, "source")),
    "human_consequence_gate_flow_discovery_receipt.px::trial.G.choice-accept"
  );
  assert!(as_bool(get(auth, "scope-limited")));
  assert!(!as_bool(get(auth, "runtime-closure-proven")));
  assert!(as_bool(get(auth, "consequence-authorized")));
  assert!(as_bool(get(auth, "enters-pnix-lifecycle")));
  assert!(as_bool(get(auth, "owner-switch-authorization")));
  assert!(!as_bool(get(auth, "bypasses-pnix-lifecycle")));
  assert!(!as_bool(get(auth, "human-is-global-cognition-authority")));
  assert!(!as_bool(get(auth, "rubber-stamp-shortcut")));
  assert!(as_bool(get(auth, "audit-ref-preserved")));
}

#[test]
fn owner_switch_receipt_has_required_r6_fields() {
  let run = eval_file(&fixture_path()).unwrap();
  let receipt = get(&run, "owner-switch-receipt");
  assert_eq!(
    as_str(get(receipt, "id")),
    "r6.owner-switch.promote-surface"
  );
  assert_eq!(as_str(get(receipt, "phase")), "R6");
  assert_eq!(
    as_str(get(receipt, "legacy-surface")),
    "stdlib/lib/ontology.px::builtins.ontologyPromote"
  );
  assert_eq!(
    as_str(get(receipt, "new-owner")),
    "macro-native.promote.surface-owner"
  );
  assert_eq!(
    as_str(get(receipt, "macro-probe")),
    "r4.macro-native-promote.rewrite-candidate"
  );
  assert_eq!(
    as_str(get(receipt, "promotion-boundary")),
    "surface-scoped-owner-switch-only"
  );
  assert_eq!(
    as_str(get(receipt, "remaining-compat-role")),
    "legacy-promote-reference-specimen-and-compat-corpus"
  );
}

#[test]
fn owner_switch_receipt_preserves_layers_roles_deltas_and_replay() {
  let run = eval_file(&fixture_path()).unwrap();
  let receipt = get(&run, "owner-switch-receipt");
  let layers = string_set(get(receipt, "layers-observed"));
  for layer in [
    "surface", "ontology", "semantic", "gate", "runtime", "audit",
  ] {
    assert!(layers.contains(layer), "missing layer `{layer}`");
  }

  let roles = string_set(get(receipt, "role-emitted"));
  for role in [
    "role.promote.lifecycle-proposal",
    "role.promote.owner-law-gated",
    "role.promote.compat-reference-required",
  ] {
    assert!(roles.contains(role), "missing emitted role `{role}`");
  }

  let deltas = attrs_by_id(get(receipt, "reference-delta"));
  assert_eq!(deltas.len(), 5);
  for delta in [
    "delta.authority",
    "delta.output-status",
    "delta.runtime",
    "delta.proof",
    "delta.source-provenance",
  ] {
    assert_eq!(
      as_str(get(deltas.get(delta).unwrap(), "verdict")),
      "covered"
    );
  }

  assert_eq!(
    as_str(get(receipt, "reverse-replay")),
    "reverse-replay-verified"
  );
  assert_eq!(as_str(get(receipt, "negative-held-proof")), "present");
}

#[test]
fn owner_switch_does_not_install_globalize_delete_or_archive() {
  let run = eval_file(&fixture_path()).unwrap();
  let receipt = get(&run, "owner-switch-receipt");
  assert!(as_bool(get(receipt, "owner-switch")));
  assert!(as_bool(get(receipt, "surface-scoped")));
  assert!(!as_bool(get(receipt, "runtime-install")));
  assert!(!as_bool(get(receipt, "global-ontology-runtime")));
  assert!(!as_bool(get(receipt, "delete-legacy-surface")));
  assert!(!as_bool(get(receipt, "archive-legacy-surface")));
  assert!(!as_bool(get(receipt, "legacy-current-authority")));
  assert!(!as_bool(get(receipt, "implementation-command")));
}

#[test]
fn legacy_promote_is_retained_as_compat_reference() {
  let run = eval_file(&fixture_path()).unwrap();
  let compat = get(&run, "compat-role");
  assert_eq!(
    as_str(get(compat, "legacy-surface")),
    "stdlib/lib/ontology.px::builtins.ontologyPromote"
  );
  assert_eq!(
    as_str(get(compat, "role-after-switch")),
    "reference-specimen-and-compat-corpus"
  );
  assert!(!as_bool(get(compat, "current-semantic-owner")));
  assert!(!as_bool(get(compat, "callable-as-legacy-authority")));
  assert!(!as_bool(get(compat, "delete-now")));
  assert!(!as_bool(get(compat, "archive-now")));
  assert!(as_bool(get(compat, "r7-required")));

  let retained = string_set(get(compat, "retained-for"));
  for expected in [
    "regression-corpus",
    "reverse-replay-reference",
    "compat-shell-input-for-r7",
    "supersede-chain-audit",
  ] {
    assert!(
      retained.contains(expected),
      "missing retained role `{expected}`"
    );
  }
}

#[test]
fn post_switch_state_routes_next_work_to_r7_and_runtime_owner_receipt() {
  let run = eval_file(&fixture_path()).unwrap();
  let state = get(&run, "post-switch-state");
  assert_eq!(
    as_str(get(state, "replacement-readiness")),
    "owner-switched-for-promote-surface"
  );
  assert!(as_bool(get(state, "owner-switch")));
  assert_eq!(
    as_str(get(state, "semantic-owner")),
    "macro-native.promote.surface-owner"
  );
  assert_eq!(
    as_str(get(state, "previous-owner-role")),
    "reference-specimen-and-compat-corpus"
  );
  assert!(!as_bool(get(state, "old-authority-active")));
  assert!(as_bool(get(state, "new-authority-surface-scoped")));
  assert!(!as_bool(get(state, "runtime-install")));
  assert!(!as_bool(get(state, "runtime-executable-now")));
  assert!(!as_bool(get(state, "global-ontology-runtime")));
  assert!(!as_bool(get(state, "delete-legacy-surface")));
  assert!(!as_bool(get(state, "archive-legacy-surface")));

  let next = string_set(get(state, "next-required"));
  for expected in [
    "r7-compat-or-archive-receipt",
    "runtime-route-owner-receipt-before-install",
    "separate-surface-receipts-for-evaluate-select-lift-query-emit",
  ] {
    assert!(
      next.contains(expected),
      "missing next requirement `{expected}`"
    );
  }
}

#[test]
fn held_trials_block_owner_switch_shortcuts() {
  let run = eval_file(&fixture_path()).unwrap();
  let trials = attrs_by_id(get(&run, "held-owner-switch-trials"));
  assert_eq!(trials.len(), 9);
  for expected in [
    "trial.A.readiness-missing",
    "trial.B.human-consequence-authorization-missing",
    "trial.C.uncovered-delta",
    "trial.D.compat-role-missing",
    "trial.E.runtime-install-requested",
    "trial.F.global-owner-switch-requested",
    "trial.G.delete-or-archive-requested",
    "trial.H.llm-prose-owner-switch",
  ] {
    let trial = trials
      .get(expected)
      .unwrap_or_else(|| panic!("missing trial `{expected}`"));
    assert_eq!(as_str(get(trial, "outcome")), "Held");
    assert!(!as_bool(get(trial, "owner-switch")));
  }

  let complete = trials.get("trial.I.complete-owner-switch").unwrap();
  assert_eq!(
    as_str(get(complete, "outcome")),
    "owner-switched-for-promote-surface"
  );
  assert!(as_bool(get(complete, "owner-switch")));
}

#[test]
fn six_layer_owner_switch_fold_preserves_runtime_boundary() {
  let run = eval_file(&fixture_path()).unwrap();
  let fold = get(&run, "six-layer-owner-switch-fold");
  assert_eq!(as_str(get(fold, "mode")), "r6-owner-switch-promote-surface");
  for layer in [
    "surface", "ontology", "semantic", "gate", "runtime", "audit",
  ] {
    assert!(
      as_bool(get_path(fold, &[layer, "visible"])),
      "layer `{layer}` must stay visible"
    );
  }
  assert!(as_bool(get_path(fold, &["surface", "owner-switch"])));
  assert_eq!(
    as_str(get_path(fold, &["ontology", "switch-scope"])),
    "surface-scoped"
  );
  assert!(!as_bool(get_path(
    fold,
    &["ontology", "global-ontology-runtime"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["semantic", "previous-owner-demoted-to-compat"]
  )));
  assert!(!as_bool(get_path(
    fold,
    &["semantic", "legacy-accepted-is-current-proof"]
  )));
  assert!(as_bool(get_path(
    fold,
    &["gate", "owner-switch-receipt-complete"]
  )));
  assert!(!as_bool(get_path(fold, &["runtime", "executable-now"])));
  assert!(!as_bool(get_path(fold, &["runtime", "installed"])));
  assert_eq!(
    as_str(get_path(fold, &["runtime", "runtime-route-owner"])),
    "not-yet-proven"
  );
  assert!(as_bool(get_path(fold, &["audit", "audit-refs-preserved"])));
  assert!(as_bool(get_path(
    fold,
    &["audit", "negative-held-proof-present"]
  )));
}

#[test]
fn runtime_observation_is_owner_switched_but_not_installed() {
  let run = eval_file(&fixture_path()).unwrap();
  let runtime = get(&run, "runtime-observation");
  assert_eq!(
    as_str(get(runtime, "observation-model")),
    "owner-switched-promote-surface-non-installed-runtime"
  );
  assert!(as_bool(get(runtime, "can-appear-at-runtime")));
  assert!(as_bool(get(runtime, "owner-switch")));
  assert!(as_bool(get(runtime, "surface-scoped")));
  assert!(!as_bool(get(runtime, "canonical-runtime-installed")));
  assert!(!as_bool(get(runtime, "executable-now")));
  assert!(!as_bool(get(runtime, "global-ontology-runtime")));
  assert_eq!(as_list(get(runtime, "runtime-added-candidates")).len(), 2);
}

#[test]
fn discoveries_record_d116_through_d124() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 9);
  for expected in [
    "D116.owner-switch-is-surface-scoped",
    "D117.readiness-is-required-before-owner-switch",
    "D118.owner-switch-is-not-runtime-install",
    "D119.legacy-surface-retained-as-compat-reference",
    "D120.audit-and-negative-held-survive-owner-switch",
    "D121.human-consequence-authorization-enters-pnix-lifecycle",
    "D122.r6-opens-r7-compat-or-archive-need",
    "D123.owner-switch-blocks-llm-prose-authority",
    "D124.promote-owner-switch-does-not-switch-other-ontology-surfaces",
  ] {
    let d = discoveries
      .get(expected)
      .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
    assert!(as_bool(get(d, "scenario-only")));
  }
}

#[test]
fn affected_plans_keep_runtime_and_other_surfaces_unimplemented() {
  let run = eval_file(&fixture_path()).unwrap();
  let affected = get(&run, "affected-plans");
  assert_eq!(
    as_str(get_path(affected, &["macroPromote", "pressure"])),
    "owner-switched-for-promote-surface"
  );
  assert_eq!(
    as_str(get_path(affected, &["runtimeRoute", "pressure"])),
    "needs-runtime-route-owner-receipt-before-install"
  );
  assert_eq!(
    as_str(get_path(affected, &["otherOntologySurfaces", "pressure"])),
    "separate-receipts-required"
  );
  for key in [
    "legacyPromote",
    "macroPromote",
    "runtimeRoute",
    "otherOntologySurfaces",
    "legacyArchive",
  ] {
    assert!(!as_bool(get_path(
      affected,
      &[key, "implementation-target"]
    )));
  }
}

#[test]
fn negative_held_evidence_survives_owner_switch() {
  let run = eval_file(&fixture_path()).unwrap();
  let negative = get(&run, "negative-held-evidence");
  assert_eq!(as_str(get(negative, "status")), "present");
  assert!(as_bool(get(negative, "survives-owner-switch")));
  let rejects = string_set(get(negative, "rejects"));
  for expected in [
    "owner-switch-before-readiness",
    "owner-switch-without-human-consequence-authorization",
    "owner-switch-with-uncovered-delta",
    "owner-switch-without-audit-ref",
    "owner-switch-without-negative-held-proof",
    "owner-switch-without-compat-role",
    "runtime-install-from-owner-switch",
    "global-ontology-owner-switch-from-single-surface",
    "delete-or-archive-from-r6",
    "llm-prose-as-owner-switch",
  ] {
    assert!(rejects.contains(expected), "missing rejection `{expected}`");
  }
}

#[test]
fn top_level_state_records_owner_switch_without_runtime_command() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_str(get(&run, "replacement-readiness")),
    "owner-switched-for-promote-surface"
  );
  assert!(as_bool(get(&run, "owner-switch")));
  assert!(!as_bool(get(&run, "runtime-install")));
  assert!(!as_bool(get(&run, "global-ontology-runtime")));
  assert!(!as_bool(get(&run, "implementation-command")));
}
