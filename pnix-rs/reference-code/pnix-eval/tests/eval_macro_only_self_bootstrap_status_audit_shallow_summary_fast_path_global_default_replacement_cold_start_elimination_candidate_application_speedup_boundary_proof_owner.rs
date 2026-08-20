use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn combined_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../stdlib/lib/gate/macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application-speedup-boundary-proof.px",
  )
}

fn eval_combined() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = combined_path();
    let json = std::thread::Builder::new()
      .name("cold-start-app-speedup-boundary-combined-eval".to_string())
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("cold-start application speedup boundary combined eval")
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
fn fixture_imports_speedup_boundary_owner_and_measurement_source() {
  let run = fixture();
  assert!(as_bool(get(run, "imported-owner")));
  assert!(as_bool(get(run, "imported-measurement-owner")));
  assert!(as_bool(get(run, "used-px-owner")));
  assert_eq!(
    as_str(get(run, "expected-source-proof")),
    "proof.macro-only.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application-measurement.v1"
  );
}

#[test]
fn owner_meta_accepts_bounded_signals_keeps_global_held() {
  let run = fixture();
  let meta = get(run, "owner-meta");
  assert!(as_bool(get(meta, "bounded-warm-envelope-accepted")));
  assert!(as_bool(get(meta, "bounded-cold-delta-signal-accepted")));
  assert!(as_bool(get(meta, "bounded-status-query-fast-path-signal")));
  assert!(as_bool(get(meta, "comparable-benchmark-required")));
  assert!(as_bool(get(
    meta,
    "application-comparable-benchmark-frontier-required"
  )));
  assert!(as_bool(get(meta, "runtime-wiring-required")));
  assert!(!as_bool(get(meta, "runtime-wired")));
  assert!(!as_bool(get(meta, "cold-start-solved")));
  assert!(!as_bool(get(meta, "cold-start-eliminated")));
  assert!(!as_bool(get(meta, "cold-start-globally-bypassed")));
  assert!(!as_bool(get(meta, "elimination-applied-globally")));
  assert!(!as_bool(get(meta, "global-speedup-claimed")));
  assert!(!as_bool(get(meta, "whole-system-speedup-claimed")));
  assert_eq!(as_i64(get(meta, "accepted-warm-min-duration-ms")), 3024);
  assert_eq!(as_i64(get(meta, "accepted-warm-max-duration-ms")), 3025);
  assert_eq!(as_i64(get(meta, "accepted-cold-delta-ms")), 9474);
  assert_eq!(
    as_str(get(meta, "boundary-verdict")),
    "bounded-warm-and-cold-delta-signals-accepted-runtime-wired-and-global-speedup-held"
  );
}

#[test]
fn valid_proof_closes_speedup_boundary_and_opens_comparable_benchmark() {
  let run = fixture();
  let valid = get(run, "valid-proof");
  assert_eq!(
    as_str(get(valid, "status")),
    "self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application-speedup-boundary-proof-present"
  );
  assert!(get(valid, "held-id").is_null());
  assert!(as_bool(get(valid, "bounded-warm-envelope-accepted")));
  assert!(as_bool(get(valid, "bounded-cold-delta-signal-accepted")));
  assert!(!as_bool(get(valid, "runtime-wired")));

  let closed = string_set(get(valid, "closes"));
  assert!(closed.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application-speedup-boundary-proof"
  ));
  let open = string_set(get(valid, "next-open-frontiers"));
  assert!(open.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application-comparable-benchmark-proof"
  ));
}

#[test]
fn boundary_summary_records_acceptance_with_held_global_claims() {
  let run = fixture();
  let summary = get(run, "boundary-summary");
  assert_eq!(
    as_str(get(summary, "id")),
    "elimination-application-speedup-boundary.fast-path.bootstrap-status-audit.shallow-summary.global-default-replacement.cold-start.v1"
  );
  assert_eq!(as_i64(get(summary, "accepted-warm-min-duration-ms")), 3024);
  assert_eq!(as_i64(get(summary, "accepted-warm-max-duration-ms")), 3025);
  assert_eq!(as_i64(get(summary, "accepted-cold-delta-ms")), 9474);
  assert!(as_bool(get(summary, "bounded-warm-envelope-accepted")));
  assert!(as_bool(get(summary, "bounded-cold-delta-signal-accepted")));
  assert!(as_bool(get(
    summary,
    "bounded-status-query-fast-path-signal"
  )));
  assert!(as_bool(get(summary, "comparable-benchmark-required")));
  assert!(as_bool(get(summary, "runtime-wiring-required")));
  assert!(!as_bool(get(summary, "runtime-wired")));
  assert!(!as_bool(get(summary, "cold-start-solved")));
  assert!(!as_bool(get(summary, "global-speedup-claimed")));
}

#[test]
fn held_failures_cover_all_branches() {
  let run = fixture();
  let prefix =
    "held.macro-only-self...cold-start-elimination-candidate-application-speedup-boundary";
  for (field, suffix) in [
    ("wrong-proof", "proof-id-mismatch"),
    ("stale-stage", "stale-current-stage"),
    ("source-mismatch", "source-mismatch"),
    ("measurement-source-missing", "measurement-source-missing"),
    (
      "measurement-input-shape-mismatch",
      "measurement-input-shape-mismatch",
    ),
    ("scope-mismatch", "scope-mismatch"),
    ("summary-mismatch", "summary-mismatch"),
    ("acceptance-mismatch", "acceptance-mismatch"),
    ("held-flags-missing", "held-flags-missing"),
    ("audit-fallback-missing", "audit-fallback-missing"),
    ("missing-evidence", "missing-required-evidence"),
    ("frontier-shape-mismatch", "frontier-shape-mismatch"),
    ("boundary-overclaim", "boundary-overclaim"),
    ("speedup-overclaim", "speedup-overclaim"),
    ("runtime-overclaim", "runtime-overclaim"),
    (
      "external-or-license-overclaim",
      "external-or-license-overclaim",
    ),
    ("authority-overclaim", "authority-overclaim"),
  ] {
    let value = get(run, field);
    assert_eq!(as_str(get(value, "status")), "Held", "{field}");
    assert_eq!(
      as_str(get(value, "held-id")),
      &format!("{}.{}", prefix, suffix),
      "{field}"
    );
  }
}

#[test]
fn hard_stops_remain_false_after_speedup_boundary_proof() {
  let run = fixture();
  for key in [
    "runtime-wired",
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
  assert!(as_bool(get(run, "bounded-warm-envelope-accepted")));
  assert!(as_bool(get(run, "bounded-cold-delta-signal-accepted")));
  assert!(as_bool(get(run, "bounded-status-query-fast-path-signal")));
  assert!(as_bool(get(run, "comparable-benchmark-required")));
  assert!(as_bool(get(run, "runtime-wiring-required")));
}
