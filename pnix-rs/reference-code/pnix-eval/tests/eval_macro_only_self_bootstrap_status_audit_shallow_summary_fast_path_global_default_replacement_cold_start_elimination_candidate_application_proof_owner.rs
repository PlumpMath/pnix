use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn combined_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../stdlib/lib/gate/macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application-proof.px",
  )
}

fn eval_combined() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = combined_path();
    let json = std::thread::Builder::new()
      .name("cold-start-elimination-candidate-application-combined-eval".to_string())
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("cold-start elimination candidate application combined eval")
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
  static F: OnceLock<&'static Value> = OnceLock::new();
  F.get_or_init(|| get(eval_combined(), "owner-fixture"))
}

fn string_set(v: &Value) -> BTreeSet<&str> {
  as_list(v).iter().map(as_str).collect()
}

#[test]
fn fixture_imports_application_owner_and_candidate_source() {
  let run = fixture();
  assert_eq!(
    as_str(get(run, "proof")),
    "macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application-proof-owner"
  );
  assert!(as_bool(get(run, "imported-owner")));
  assert!(as_bool(get(run, "imported-candidate-owner")));
  assert!(as_bool(get(run, "used-px-owner")));
  assert_eq!(
    as_str(get(run, "expected-source-proof")),
    "proof.macro-only.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate.v1"
  );
}

#[test]
fn owner_meta_marks_wrapper_bypass_applied_with_bounded_scope() {
  let run = fixture();
  let meta = get(run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.lib.gate.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application-proof"
  );
  assert!(as_bool(get(
    meta,
    "global-default-replacement-cold-start-elimination-candidate-application-proof"
  )));
  assert!(as_bool(get(meta, "wrapper-bypass-applied")));
  assert!(as_bool(get(meta, "bounded-application")));
  assert!(as_bool(get(meta, "application-measurement-required")));
  assert!(as_bool(get(
    meta,
    "application-measurement-frontier-required"
  )));
  assert_eq!(
    as_str(get(meta, "application-bounded-scope")),
    "bootstrap-status-audit-shallow-summary-status-query-family"
  );
  assert_eq!(
    as_str(get(meta, "application-verdict")),
    "wrapper-bypass-applied-bounded-scope-application-measurement-required"
  );
  assert!(!as_bool(get(meta, "cold-start-solved")));
  assert!(!as_bool(get(meta, "cold-start-eliminated")));
  assert!(!as_bool(get(meta, "cold-start-globally-bypassed")));
  assert!(!as_bool(get(meta, "elimination-applied-globally")));
  assert!(!as_bool(get(meta, "global-speedup-claimed")));
}

#[test]
fn valid_proof_closes_application_and_opens_application_measurement() {
  let run = fixture();
  let valid = get(run, "valid-proof");
  assert_eq!(
    as_str(get(valid, "status")),
    "self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application-proof-present"
  );
  assert!(get(valid, "held-id").is_null());
  assert!(as_bool(get(valid, "wrapper-bypass-applied")));
  assert!(as_bool(get(valid, "bounded-application")));
  assert_eq!(as_i64(get(valid, "application-record-count")), 3);
  assert_eq!(as_i64(get(valid, "applied-record-count")), 1);
  assert_eq!(
    as_str(get(valid, "applied-implementation")),
    "direct-evaluator-call-without-p-puck-cargo-spawn"
  );

  let closed = string_set(get(valid, "closes"));
  assert!(closed.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application-proof"
  ));
  let open = string_set(get(valid, "next-open-frontiers"));
  assert!(open.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application-measurement-proof"
  ));
}

#[test]
fn application_registry_records_one_applied_two_deferred() {
  let run = fixture();
  let reg = get(run, "application-registry");
  assert_eq!(
    as_str(get(reg, "id")),
    "elimination-application-registry.fast-path.bootstrap-status-audit.shallow-summary.global-default-replacement.cold-start.v1"
  );
  assert_eq!(as_i64(get(reg, "application-record-count")), 3);
  assert_eq!(as_i64(get(reg, "applied-record-count")), 1);
  assert_eq!(as_i64(get(reg, "not-applied-record-count")), 2);
  assert_eq!(
    as_str(get(reg, "applied-candidate-source")),
    "elimination.candidate.wrapper-bypass.bootstrap-status-audit-shallow-summary-status-query"
  );
  assert_eq!(
    as_str(get(reg, "applied-implementation")),
    "direct-evaluator-call-without-p-puck-cargo-spawn"
  );
  assert!(as_bool(get(reg, "wrapper-bypass-applied")));
  assert!(as_bool(get(reg, "bounded-application")));
  assert!(as_bool(get(reg, "application-measurement-required")));
  assert!(!as_bool(get(reg, "cold-start-solved")));
  assert!(!as_bool(get(reg, "cold-start-eliminated")));
  assert!(!as_bool(get(reg, "cold-start-globally-bypassed")));
  assert!(!as_bool(get(reg, "elimination-applied-globally")));
}

#[test]
fn held_failures_cover_all_branches() {
  let run = fixture();
  for (field, held_id) in [
    (
      "wrong-proof",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application.proof-id-mismatch",
    ),
    (
      "stale-stage",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application.stale-current-stage",
    ),
    (
      "source-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application.source-mismatch",
    ),
    (
      "candidate-source-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application.candidate-source-missing",
    ),
    (
      "candidate-input-shape-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application.candidate-input-shape-mismatch",
    ),
    (
      "application-record-shape-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application.application-record-shape-mismatch",
    ),
    (
      "application-record-invalid",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application.application-record-invalid",
    ),
    (
      "application-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application.application-mismatch",
    ),
    (
      "application-status-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application.application-status-mismatch",
    ),
    (
      "scope-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application.scope-mismatch",
    ),
    (
      "registry-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application.registry-mismatch",
    ),
    (
      "measurement-frontier-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application.measurement-frontier-missing",
    ),
    (
      "audit-fallback-missing",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application.audit-fallback-missing",
    ),
    (
      "missing-evidence",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application.missing-required-evidence",
    ),
    (
      "frontier-shape-mismatch",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application.frontier-shape-mismatch",
    ),
    (
      "application-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application.application-overclaim",
    ),
    (
      "speedup-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application.speedup-overclaim",
    ),
    (
      "runtime-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application.runtime-overclaim",
    ),
    (
      "external-or-license-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application.external-or-license-overclaim",
    ),
    (
      "authority-overclaim",
      "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application.authority-overclaim",
    ),
  ] {
    let value = get(run, field);
    assert_eq!(as_str(get(value, "status")), "Held", "{field}");
    assert_eq!(as_str(get(value, "held-id")), held_id, "{field}");
  }
}

#[test]
fn hard_stops_remain_false_after_application_proof() {
  let run = fixture();
  for key in [
    "cold-start-solved",
    "cold-start-eliminated",
    "cold-start-globally-bypassed",
    "elimination-applied-globally",
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
  assert!(as_bool(get(run, "wrapper-bypass-applied")));
  assert!(as_bool(get(run, "bounded-application")));
}
