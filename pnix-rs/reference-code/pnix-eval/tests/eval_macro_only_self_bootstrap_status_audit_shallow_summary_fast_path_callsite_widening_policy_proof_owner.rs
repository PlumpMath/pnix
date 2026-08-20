use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/pnix-query-runtime/macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-policy-proof-owner.px",
  )
}

fn eval_fixture() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = fixture_path();
    let json = std::thread::Builder::new()
      .name("bootstrap-shallow-summary-fast-path-callsite-widening-policy-owner-eval".to_string())
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("shallow summary fast-path callsite widening policy owner fixture")
      })
      .expect("spawn eval thread")
      .join()
      .expect("eval thread panicked");
    serde_json::from_str(&json).expect("fixture JSON")
  })
}

fn as_attrs(v: &Value) -> &Map<String, Value> {
  v.as_object()
    .unwrap_or_else(|| panic!("expected object, got {v:?}"))
}

fn as_list(v: &Value) -> &Vec<Value> {
  v.as_array()
    .unwrap_or_else(|| panic!("expected array, got {v:?}"))
}

fn as_str(v: &Value) -> &str {
  v.as_str()
    .unwrap_or_else(|| panic!("expected string, got {v:?}"))
}

fn as_bool(v: &Value) -> bool {
  v.as_bool()
    .unwrap_or_else(|| panic!("expected bool, got {v:?}"))
}

fn as_i64(v: &Value) -> i64 {
  v.as_i64()
    .unwrap_or_else(|| panic!("expected integer, got {v:?}"))
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

fn string_set(v: &Value) -> BTreeSet<&str> {
  as_list(v).iter().map(as_str).collect()
}

fn attrs_by_callsite<'a>(items: &'a Value) -> BTreeMap<&'a str, &'a Value> {
  as_list(items)
    .iter()
    .map(|item| (as_str(get(item, "callsite-id")), item))
    .collect()
}

#[test]
fn fixture_imports_policy_owner_and_measurement_source() {
  let run = eval_fixture();
  assert_eq!(
    as_str(get(run, "proof")),
    "macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-policy-proof-owner"
  );
  assert!(as_bool(get(run, "imported-owner")));
  assert!(as_bool(get(run, "imported-measurement-owner")));
  assert!(as_bool(get(run, "imported-measurement-fixture")));
  assert!(as_bool(get(run, "used-px-owner")));
  assert_eq!(
    as_str(get(run, "expected-source-proof")),
    "proof.macro-only.self.bootstrap-status-audit-shallow-summary-fast-path-application-measurement.v1"
  );
}

#[test]
fn owner_meta_approves_policy_only_not_widening_application() {
  let run = eval_fixture();
  let meta = get(run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.lib.gate.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-policy-proof"
  );
  assert_eq!(
    as_str(get(meta, "constructor")),
    "validateSelfBootstrapStatusAuditShallowSummaryFastPathCallsiteWideningPolicyProof"
  );
  assert!(as_bool(get(
    meta,
    "self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-policy-proof"
  )));
  assert!(as_bool(get(meta, "callsite-widening-policy-approved")));
  assert_eq!(as_i64(get(meta, "allowed-callsite-count")), 3);
  assert_eq!(as_i64(get(meta, "eligible-new-callsite-count")), 2);
  assert!(as_bool(get(meta, "selected-callsite-remains-applied")));
  assert!(as_bool(get(meta, "application-proof-required")));
  for key in [
    "callsite-widening-applied",
    "additional-callsites-applied",
    "global-default-callsite-replaced",
    "global-speedup-claimed",
    "runtime-install",
    "global-ontology-runtime",
    "runtime-api-flattening",
    "meaning-db",
    "external-solver-installed",
    "self-modification",
    "llm-authority",
    "p-puck-is-semantic-owner",
    "old-host-authority",
    "gpl-family-dependencies",
  ] {
    assert!(!as_bool(get(meta, key)), "`{key}` must stay false");
  }
}

#[test]
fn allowlist_contains_selected_callsite_and_two_new_candidates() {
  let run = eval_fixture();
  let ids = string_set(get(run, "allowed-callsite-ids"));
  assert_eq!(ids.len(), 3);
  assert!(ids.contains("callsite.bootstrap-status-audit.current-status.shallow-summary.v1"));
  assert!(ids.contains("callsite.bootstrap-status-audit.operator-panel.shallow-summary.v1"));
  assert!(ids.contains("callsite.bootstrap-status-audit.index-status.shallow-summary.v1"));

  let new_ids = string_set(get(run, "eligible-new-callsite-ids"));
  assert_eq!(new_ids.len(), 2);
  assert!(new_ids.contains("callsite.bootstrap-status-audit.operator-panel.shallow-summary.v1"));
  assert!(new_ids.contains("callsite.bootstrap-status-audit.index-status.shallow-summary.v1"));

  let fields = string_set(get(run, "expected-summary-fields"));
  for expected in [
    "new-engine-from-zero",
    "macro-only-runtime-owner",
    "semantic-owner",
    "boot-executed",
    "host-code-removal-started",
    "global-ontology-runtime",
    "runtime-api-flattening",
    "meaning-db",
    "optimization-applied",
    "fast-path-promoted",
    "external-solver-installed",
  ] {
    assert!(
      fields.contains(expected),
      "missing summary field `{expected}`"
    );
  }
}

#[test]
fn policy_approvals_preserve_shape_and_refuse_application_claims() {
  let run = eval_fixture();
  let approvals = attrs_by_callsite(get(run, "expected-policy-approvals"));
  assert_eq!(approvals.len(), 3);
  for id in [
    "callsite.bootstrap-status-audit.current-status.shallow-summary.v1",
    "callsite.bootstrap-status-audit.operator-panel.shallow-summary.v1",
    "callsite.bootstrap-status-audit.index-status.shallow-summary.v1",
  ] {
    let approval = approvals[id];
    assert_eq!(as_str(get(approval, "status")), "policy-eligible");
    assert!(as_bool(get(approval, "callsite-widening-policy-approved")));
    assert!(as_bool(get(approval, "application-proof-required")));
    assert!(as_bool(get(approval, "full-audit-fallback-preserved")));
    assert!(!as_bool(get(approval, "callsite-widening-applied")));
    assert!(!as_bool(get(approval, "additional-callsites-applied")));
    assert!(!as_bool(get(approval, "global-default-callsite-replaced")));
    assert!(!as_bool(get(approval, "global-speedup-claimed")));
  }
}

#[test]
fn valid_proof_closes_policy_and_opens_application_frontier() {
  let run = eval_fixture();
  let valid = get(run, "valid-proof");
  assert_eq!(
    as_str(get(valid, "status")),
    "self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-policy-proof-present"
  );
  assert!(get(valid, "held-id").is_null());
  assert!(as_bool(get(valid, "callsite-widening-policy-approved")));
  assert!(!as_bool(get(valid, "callsite-widening-applied")));
  assert!(!as_bool(get(valid, "additional-callsites-applied")));
  assert_eq!(as_i64(get(valid, "allowed-callsite-count")), 3);
  assert_eq!(as_i64(get(valid, "eligible-new-callsite-count")), 2);

  let closed = string_set(get(valid, "closes"));
  assert!(closed.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-policy-proof"
  ));
  let open = string_set(get(valid, "next-open-frontiers"));
  assert!(open.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-application-proof"
  ));
}

#[test]
fn held_failures_cover_negative_classification_and_overclaims() {
  let run = eval_fixture();
  for (field, held_id) in [
    (
      "negative-held-full-json-shape",
      "held.bootstrap-status-shallow-summary-callsite-widening-policy.full-json-shape",
    ),
    (
      "negative-held-not-allowlisted",
      "held.bootstrap-status-shallow-summary-callsite-widening-policy.not-allowlisted",
    ),
    (
      "negative-held-fallback-missing",
      "held.bootstrap-status-shallow-summary-callsite-widening-policy.fallback-missing",
    ),
    (
      "negative-held-global-overclaim",
      "held.bootstrap-status-shallow-summary-callsite-widening-policy.global-overclaim",
    ),
    (
      "negative-held-field-shape",
      "held.bootstrap-status-shallow-summary-callsite-widening-policy.field-shape",
    ),
    (
      "negative-held-domain-mismatch",
      "held.bootstrap-status-shallow-summary-callsite-widening-policy.domain-mismatch",
    ),
    (
      "wrong-proof",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-policy.proof-id-mismatch",
    ),
    (
      "stale-stage",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-policy.stale-current-stage",
    ),
    (
      "source-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-policy.source-mismatch",
    ),
    (
      "measurement-evidence-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-policy.measurement-evidence-missing",
    ),
    (
      "policy-record-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-policy.policy-record-mismatch",
    ),
    (
      "approval-shape-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-policy.approval-shape-mismatch",
    ),
    (
      "selected-lost",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-policy.selected-callsite-lost",
    ),
    (
      "negative-held-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-policy.negative-held-missing",
    ),
    (
      "missing-evidence",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-policy.missing-required-evidence",
    ),
    (
      "frontier-shape-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-policy.frontier-shape-mismatch",
    ),
    (
      "application-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-policy.application-overclaim",
    ),
    (
      "runtime-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-policy.runtime-overclaim",
    ),
    (
      "external-or-license-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-policy.external-or-license-overclaim",
    ),
    (
      "authority-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-callsite-widening-policy.authority-overclaim",
    ),
  ] {
    let held = get(run, field);
    assert_eq!(as_str(get(held, "status")), "Held", "`{field}` status");
    assert_eq!(as_str(get(held, "held-id")), held_id, "`{field}` held id");
  }
}
