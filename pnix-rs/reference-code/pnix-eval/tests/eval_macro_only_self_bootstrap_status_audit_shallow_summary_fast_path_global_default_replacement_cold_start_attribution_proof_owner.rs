use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/pnix-query-runtime/macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution-proof-owner.px",
  )
}

fn eval_fixture() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = fixture_path();
    let json = std::thread::Builder::new()
      .name(
        "bootstrap-shallow-summary-fast-path-global-default-cold-start-attribution-owner-eval"
          .to_string(),
      )
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("global default replacement cold-start attribution owner fixture")
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

#[test]
fn fixture_imports_attribution_owner_boundary_and_wrapper_evidence() {
  let run = eval_fixture();
  assert_eq!(
    as_str(get(run, "proof")),
    "macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution-proof-owner"
  );
  assert!(as_bool(get(run, "imported-owner")));
  assert!(as_bool(get(run, "imported-boundary-owner")));
  assert!(as_bool(get(run, "imported-boundary-fixture")));
  assert!(as_bool(get(run, "imported-wrapper-repeat-owner")));
  assert!(as_bool(get(run, "used-px-owner")));
  assert_eq!(
    as_str(get(run, "expected-source-proof")),
    "proof.macro-only.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-boundary.v1"
  );
}

#[test]
fn owner_meta_proves_wrapper_attribution_keeps_others_candidate_only() {
  let run = eval_fixture();
  let meta = get(run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.lib.gate.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution-proof"
  );
  assert!(as_bool(get(
    meta,
    "global-default-replacement-cold-start-attribution-proof"
  )));
  assert!(as_bool(get(meta, "wrapper-attribution-proven")));
  assert!(!as_bool(get(meta, "core-eval-attribution-proven")));
  assert!(!as_bool(get(meta, "unknown-attribution-proven")));
  assert!(as_bool(get(meta, "attribution-policy-frontier-required")));
  assert_eq!(as_i64(get(meta, "wrapper-attributable-min-ms")), 11059);
  assert_eq!(as_i64(get(meta, "wrapper-attributable-max-ms")), 11170);
  assert_eq!(as_i64(get(meta, "cold-warm-gap-min-ms")), 9285);
  assert_eq!(as_i64(get(meta, "cold-warm-gap-max-ms")), 10278);
  assert_eq!(
    as_str(get(meta, "attribution-verdict")),
    "wrapper-attribution-proven-core-eval-and-unknown-attribution-candidates-only"
  );
  assert!(!as_bool(get(meta, "cold-start-solved")));
  assert!(!as_bool(get(meta, "cold-start-eliminated")));
  assert!(!as_bool(get(
    meta,
    "cold-start-attributed-to-undocumented-cause"
  )));
  assert!(!as_bool(get(meta, "wrapper-bypass-applied")));
}

#[test]
fn valid_proof_closes_cold_start_attribution_and_opens_attribution_policy() {
  let run = eval_fixture();
  let valid = get(run, "valid-proof");
  assert_eq!(
    as_str(get(valid, "status")),
    "self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution-proof-present"
  );
  assert!(get(valid, "held-id").is_null());
  assert!(as_bool(get(valid, "wrapper-attribution-proven")));
  assert!(as_bool(get(valid, "core-eval-attribution-candidate-only")));
  assert!(as_bool(get(valid, "unknown-attribution-candidate-only")));
  assert!(as_bool(get(valid, "attribution-policy-frontier-required")));

  let closed = string_set(get(valid, "closes"));
  assert!(closed.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution-proof"
  ));
  let open = string_set(get(valid, "next-open-frontiers"));
  assert!(open.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution-policy-proof"
  ));
}

#[test]
fn attribution_summary_records_three_candidates_and_wrapper_share() {
  let run = eval_fixture();
  let summary = get(run, "attribution-summary");
  assert_eq!(
    as_str(get(summary, "id")),
    "attribution.fast-path.bootstrap-status-audit.shallow-summary.global-default-replacement.cold-start.v1"
  );
  assert_eq!(
    as_str(get(summary, "attribution-scope")),
    "bootstrap-status-audit-shallow-summary-status-query-family"
  );
  assert_eq!(as_i64(get(summary, "wrapper-attributable-min-ms")), 11059);
  assert_eq!(as_i64(get(summary, "wrapper-attributable-max-ms")), 11170);
  assert_eq!(as_i64(get(summary, "attribution-record-count")), 3);
  assert!(as_bool(get(summary, "wrapper-attribution-proven")));
  assert!(!as_bool(get(summary, "core-eval-attribution-proven")));
  assert!(!as_bool(get(summary, "unknown-attribution-proven")));
  assert!(as_bool(get(
    summary,
    "attribution-policy-frontier-required"
  )));
  assert_eq!(
    as_str(get(summary, "attribution-verdict")),
    "wrapper-attribution-proven-core-eval-and-unknown-attribution-candidates-only"
  );
  assert!(!as_bool(get(summary, "cold-start-solved")));
  assert!(!as_bool(get(summary, "cold-start-eliminated")));
  assert!(!as_bool(get(
    summary,
    "cold-start-attributed-to-undocumented-cause"
  )));
  assert!(!as_bool(get(summary, "wrapper-bypass-applied")));
}

#[test]
fn held_failures_cover_inputs_records_summary_frontier_and_overclaims() {
  let run = eval_fixture();
  for (field, held_id) in [
    (
      "wrong-proof",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution.proof-id-mismatch",
    ),
    (
      "stale-stage",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution.stale-current-stage",
    ),
    (
      "source-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution.source-mismatch",
    ),
    (
      "boundary-source-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution.boundary-source-missing",
    ),
    (
      "wrapper-evidence-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution.wrapper-evidence-missing",
    ),
    (
      "wrapper-share-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution.wrapper-share-mismatch",
    ),
    (
      "attribution-record-shape-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution.attribution-record-shape-mismatch",
    ),
    (
      "attribution-record-invalid",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution.attribution-record-invalid",
    ),
    (
      "attribution-status-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution.attribution-status-mismatch",
    ),
    (
      "scope-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution.scope-mismatch",
    ),
    (
      "summary-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution.summary-mismatch",
    ),
    (
      "policy-frontier-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution.policy-frontier-missing",
    ),
    (
      "audit-fallback-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution.audit-fallback-missing",
    ),
    (
      "missing-evidence",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution.missing-required-evidence",
    ),
    (
      "frontier-shape-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution.frontier-shape-mismatch",
    ),
    (
      "attribution-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution.attribution-overclaim",
    ),
    (
      "speedup-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution.speedup-overclaim",
    ),
    (
      "runtime-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution.runtime-overclaim",
    ),
    (
      "external-or-license-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution.external-or-license-overclaim",
    ),
    (
      "authority-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution.authority-overclaim",
    ),
  ] {
    let value = get(run, field);
    assert_eq!(as_str(get(value, "status")), "Held", "{field}");
    assert_eq!(as_str(get(value, "held-id")), held_id, "{field}");
  }
}

#[test]
fn hard_stops_remain_false_after_cold_start_attribution_proof() {
  let run = eval_fixture();
  for key in [
    "cold-start-solved",
    "cold-start-eliminated",
    "cold-start-attributed-to-undocumented-cause",
    "wrapper-bypass-applied",
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
