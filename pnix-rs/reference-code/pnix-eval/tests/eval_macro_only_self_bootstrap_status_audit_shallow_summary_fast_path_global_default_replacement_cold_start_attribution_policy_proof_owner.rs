use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

// Combined-file pattern: the slice .px emits an attrset with three named
// children (owner-shape, owner-fixture, receipt).  This owner test loads the
// combined eval result and navigates to `.owner-fixture`.

fn combined_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../stdlib/lib/gate/macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution-policy-proof.px",
  )
}

fn eval_combined() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = combined_path();
    let json = std::thread::Builder::new()
      .name("cold-start-attribution-policy-combined-eval".to_string())
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("cold-start attribution policy combined eval")
      })
      .expect("spawn eval thread")
      .join()
      .expect("eval thread panicked");
    serde_json::from_str(&json).expect("combined JSON")
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

fn fixture() -> &'static Value {
  // navigate combined → owner-fixture
  static F: OnceLock<&'static Value> = OnceLock::new();
  F.get_or_init(|| get(eval_combined(), "owner-fixture"))
}

fn string_set(v: &Value) -> BTreeSet<&str> {
  as_list(v).iter().map(as_str).collect()
}

#[test]
fn fixture_imports_attribution_policy_owner_and_attribution_source() {
  let run = fixture();
  assert_eq!(
    as_str(get(run, "proof")),
    "macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution-policy-proof-owner"
  );
  assert!(as_bool(get(run, "imported-owner")));
  assert!(as_bool(get(run, "imported-attribution-owner")));
  assert!(as_bool(get(run, "used-px-owner")));
  assert_eq!(
    as_str(get(run, "expected-source-proof")),
    "proof.macro-only.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution.v1"
  );
}

#[test]
fn owner_meta_marks_three_eligibility_categories() {
  let run = fixture();
  let meta = get(run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.lib.gate.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution-policy-proof"
  );
  assert!(as_bool(get(
    meta,
    "global-default-replacement-cold-start-attribution-policy-proof"
  )));
  assert!(as_bool(get(meta, "wrapper-elimination-candidate-eligible")));
  assert!(as_bool(get(meta, "core-eval-measurement-required")));
  assert!(as_bool(get(meta, "unknown-deferred-until-residual")));
  assert!(as_bool(get(
    meta,
    "elimination-candidate-frontier-required"
  )));
  assert_eq!(
    as_str(get(meta, "policy-verdict")),
    "wrapper-elimination-candidate-eligible-others-deferred-or-measurement-required"
  );
  assert!(!as_bool(get(meta, "cold-start-solved")));
  assert!(!as_bool(get(meta, "cold-start-eliminated")));
  assert!(!as_bool(get(meta, "wrapper-bypass-applied")));
  assert!(!as_bool(get(meta, "elimination-applied")));
}

#[test]
fn valid_proof_closes_attribution_policy_and_opens_elimination_candidate() {
  let run = fixture();
  let valid = get(run, "valid-proof");
  assert_eq!(
    as_str(get(valid, "status")),
    "self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution-policy-proof-present"
  );
  assert!(get(valid, "held-id").is_null());
  assert!(as_bool(get(
    valid,
    "wrapper-elimination-candidate-eligible"
  )));
  assert!(as_bool(get(valid, "core-eval-measurement-required")));
  assert!(as_bool(get(valid, "unknown-deferred-until-residual")));

  let closed = string_set(get(valid, "closes"));
  assert!(closed.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution-policy-proof"
  ));
  let open = string_set(get(valid, "next-open-frontiers"));
  assert!(open.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-proof"
  ));
}

#[test]
fn policy_summary_records_three_candidates() {
  let run = fixture();
  let summary = get(run, "policy-summary");
  assert_eq!(
    as_str(get(summary, "id")),
    "policy.fast-path.bootstrap-status-audit.shallow-summary.global-default-replacement.cold-start-attribution.v1"
  );
  assert_eq!(as_i64(get(summary, "candidate-count")), 3);
  assert!(as_bool(get(
    summary,
    "wrapper-elimination-candidate-eligible"
  )));
  assert!(as_bool(get(summary, "core-eval-measurement-required")));
  assert!(as_bool(get(summary, "unknown-deferred-until-residual")));
  assert!(as_bool(get(
    summary,
    "elimination-candidate-frontier-required"
  )));
  assert!(!as_bool(get(summary, "cold-start-solved")));
  assert!(!as_bool(get(summary, "cold-start-eliminated")));
  assert!(!as_bool(get(summary, "wrapper-bypass-applied")));
  assert!(!as_bool(get(summary, "elimination-applied")));
}

#[test]
fn held_failures_cover_inputs_eligibility_summary_frontier_and_overclaims() {
  let run = fixture();
  for (field, held_id) in [
    (
      "wrong-proof",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution-policy.proof-id-mismatch",
    ),
    (
      "stale-stage",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution-policy.stale-current-stage",
    ),
    (
      "source-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution-policy.source-mismatch",
    ),
    (
      "attribution-source-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution-policy.attribution-source-missing",
    ),
    (
      "attribution-input-shape-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution-policy.attribution-input-shape-mismatch",
    ),
    (
      "policy-candidate-shape-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution-policy.policy-candidate-shape-mismatch",
    ),
    (
      "policy-candidate-invalid",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution-policy.policy-candidate-invalid",
    ),
    (
      "eligibility-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution-policy.eligibility-mismatch",
    ),
    (
      "scope-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution-policy.scope-mismatch",
    ),
    (
      "summary-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution-policy.summary-mismatch",
    ),
    (
      "elimination-frontier-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution-policy.elimination-frontier-missing",
    ),
    (
      "audit-fallback-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution-policy.audit-fallback-missing",
    ),
    (
      "missing-evidence",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution-policy.missing-required-evidence",
    ),
    (
      "frontier-shape-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution-policy.frontier-shape-mismatch",
    ),
    (
      "policy-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution-policy.policy-overclaim",
    ),
    (
      "speedup-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution-policy.speedup-overclaim",
    ),
    (
      "runtime-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution-policy.runtime-overclaim",
    ),
    (
      "external-or-license-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution-policy.external-or-license-overclaim",
    ),
    (
      "authority-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution-policy.authority-overclaim",
    ),
  ] {
    let value = get(run, field);
    assert_eq!(as_str(get(value, "status")), "Held", "{field}");
    assert_eq!(as_str(get(value, "held-id")), held_id, "{field}");
  }
}

#[test]
fn hard_stops_remain_false_after_cold_start_attribution_policy_proof() {
  let run = fixture();
  for key in [
    "cold-start-solved",
    "cold-start-eliminated",
    "wrapper-bypass-applied",
    "elimination-applied",
    "global-speedup-claimed",
    "whole-system-speedup-claimed",
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
    "implementation-command",
  ] {
    assert!(!as_bool(get(run, key)), "`{key}` must stay false");
  }
}
