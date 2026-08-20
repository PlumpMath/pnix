use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/pnix-query-runtime/macro-only-self-p-puck-wrapper-cold-start-repeat-proof-owner.px",
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

fn as_i64(v: &Value) -> i64 {
  match v {
    Value::Int(i) => *i,
    other => panic!("expected int, got {:?}", other),
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

fn string_set(v: &Value) -> BTreeSet<&str> {
  as_list(v).iter().map(as_str).collect()
}

#[test]
fn fixture_imports_owner_and_uses_actual_repeat_queries() {
  let run = eval_file(&fixture_path()).expect("self p-puck wrapper repeat owner fixture");
  assert_eq!(
    as_str(get(&run, "proof")),
    "macro-only-self-p-puck-wrapper-cold-start-repeat-proof-owner"
  );
  assert!(as_bool(get(&run, "imported-owner")));
  assert!(as_bool(get(&run, "used-px-owner")));
  assert_eq!(
    as_str(get(&run, "p-puck-proof-source")),
    "actual repeated p-puck pnix preset status queries over bottleneck attribution proof owner"
  );
}

#[test]
fn owner_meta_closes_wrapper_repeat_without_optimization() {
  let run = eval_file(&fixture_path()).unwrap();
  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.lib.gate.macro-only-self-p-puck-wrapper-cold-start-repeat-proof"
  );
  assert_eq!(
    as_str(get(meta, "constructor")),
    "validateSelfPPuckWrapperColdStartRepeatProof"
  );
  assert!(as_bool(get(meta, "p-puck-wrapper-cold-start-repeat-proof")));
  assert!(as_bool(get(meta, "p-puck-wrapper-repeat-within-threshold")));
  assert!(as_bool(get(meta, "wrapper-repeat-frontier-closed")));
  assert!(!as_bool(get(meta, "persistent-p-puck-wrapper-slow-path")));
  assert!(!as_bool(get(meta, "profile-required-from-wrapper-repeat")));
  assert!(as_bool(get(
    meta,
    "bootstrap-status-audit-bottleneck-candidate"
  )));
  for key in [
    "optimization-selected",
    "fast-path-promoted",
    "external-solver-installed",
    "runtime-install",
    "global-ontology-runtime",
    "runtime-api-flattening",
    "meaning-db",
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
fn telemetry_constants_are_pinned() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(
    as_i64(get(&run, "prior-execution-wrapper-duration-ms")),
    9420
  );
  assert_eq!(
    as_i64(get(&run, "prior-attribution-wrapper-duration-ms")),
    11416
  );
  assert_eq!(as_i64(get(&run, "repeat-one-duration-ms")), 357);
  assert_eq!(as_i64(get(&run, "repeat-two-duration-ms")), 246);
  assert_eq!(as_i64(get(&run, "repeat-max-duration-ms")), 357);
  assert_eq!(as_i64(get(&run, "repeat-min-duration-ms")), 246);
  assert_eq!(as_i64(get(&run, "repeat-delta-from-prior-ms")), -11059);
  assert_eq!(as_i64(get(&run, "slow-threshold-ms")), 5000);
  assert_eq!(
    as_str(get(&run, "expected-prior-slow-path-status")),
    "slow-path-candidate"
  );
  assert_eq!(
    as_str(get(&run, "expected-repeat-slow-path-status")),
    "within-threshold"
  );
}

#[test]
fn valid_proof_closes_wrapper_repeat_frontier_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let valid = get(&run, "valid-proof");
  assert_eq!(
    as_str(get(valid, "status")),
    "self-p-puck-wrapper-cold-start-repeat-proof-present"
  );
  assert!(matches!(get(valid, "held-id"), Value::Null));
  assert!(as_bool(get(
    valid,
    "p-puck-wrapper-cold-start-repeat-proof"
  )));
  assert!(as_bool(get(
    valid,
    "p-puck-wrapper-repeat-within-threshold"
  )));
  assert!(as_bool(get(valid, "wrapper-repeat-frontier-closed")));
  assert!(!as_bool(get(valid, "persistent-p-puck-wrapper-slow-path")));
  assert!(!as_bool(get(valid, "profile-required-from-wrapper-repeat")));
  assert!(as_bool(get(
    valid,
    "bootstrap-status-audit-bottleneck-candidate"
  )));
  assert_eq!(as_i64(get(valid, "repeat-record-count")), 2);
  assert_eq!(as_i64(get(valid, "repeat-max-duration-ms")), 357);
  assert_eq!(as_i64(get(valid, "repeat-min-duration-ms")), 246);
  for key in [
    "optimization-selected",
    "fast-path-promoted",
    "external-solver-installed",
    "runtime-install",
    "global-ontology-runtime",
    "runtime-api-flattening",
    "meaning-db",
    "self-modification",
    "llm-authority",
    "p-puck-is-semantic-owner",
    "old-host-authority",
    "gpl-family-dependencies",
  ] {
    assert!(!as_bool(get(valid, key)), "`{key}` must stay false");
  }
}

#[test]
fn required_evidence_and_remaining_frontiers_are_explicit() {
  let run = eval_file(&fixture_path()).unwrap();
  let evidence = string_set(get(&run, "required-evidence"));
  for expected in [
    "self-bottleneck-attribution-proof-present",
    "p-puck-wrapper-bottleneck-candidate-present",
    "repeat-run-1-exit-zero",
    "repeat-run-2-exit-zero",
    "repeat-durations-within-threshold",
    "persistent-wrapper-slow-path-false-recorded",
    "bootstrap-audit-frontier-retained",
    "optimization-deferred-after-repeat",
  ] {
    assert!(evidence.contains(expected), "missing evidence `{expected}`");
  }

  let frontiers = string_set(get(&run, "remaining-open-frontiers"));
  assert!(!frontiers.contains("need.self.p-puck-wrapper-cold-start-repeat-proof"));
  assert!(frontiers.contains("need.self.bootstrap-status-audit-profile-split-proof"));
  assert!(frontiers.contains("need.self.optimization-candidate-after-bottleneck-attribution"));
}

#[test]
fn stale_wrong_source_repeat_and_frontier_cases_are_held() {
  let run = eval_file(&fixture_path()).unwrap();
  for (key, held_id) in [
    (
      "wrong-proof",
      "held.macro-only-self-p-puck-wrapper-cold-start-repeat.proof-id-mismatch",
    ),
    (
      "stale-stage",
      "held.macro-only-self-p-puck-wrapper-cold-start-repeat.stale-current-stage",
    ),
    (
      "source-mismatch",
      "held.macro-only-self-p-puck-wrapper-cold-start-repeat.source-mismatch",
    ),
    (
      "attribution-input-missing",
      "held.macro-only-self-p-puck-wrapper-cold-start-repeat.attribution-input-missing",
    ),
    (
      "prior-slow-path-drift",
      "held.macro-only-self-p-puck-wrapper-cold-start-repeat.prior-slow-path-drift",
    ),
    (
      "repeat-record-shape-mismatch",
      "held.macro-only-self-p-puck-wrapper-cold-start-repeat.repeat-record-shape-mismatch",
    ),
    (
      "repeat-record-invalid",
      "held.macro-only-self-p-puck-wrapper-cold-start-repeat.repeat-record-invalid",
    ),
    (
      "repeat-summary-drift",
      "held.macro-only-self-p-puck-wrapper-cold-start-repeat.repeat-summary-drift",
    ),
    (
      "missing-evidence",
      "held.macro-only-self-p-puck-wrapper-cold-start-repeat.missing-required-evidence",
    ),
    (
      "frontier-shape-mismatch",
      "held.macro-only-self-p-puck-wrapper-cold-start-repeat.frontier-shape-mismatch",
    ),
  ] {
    let output = get(&run, key);
    assert_eq!(as_str(get(output, "status")), "Held", "{key}");
    assert_eq!(as_str(get(output, "held-id")), held_id, "{key}");
    assert!(!as_bool(get(
      output,
      "p-puck-wrapper-cold-start-repeat-proof"
    )));
  }
}

#[test]
fn persistent_optimization_runtime_authority_and_gpl_overclaims_are_held() {
  let run = eval_file(&fixture_path()).unwrap();
  for (key, held_id) in [
    (
      "persistent-wrapper-overclaim",
      "held.macro-only-self-p-puck-wrapper-cold-start-repeat.persistent-wrapper-overclaim",
    ),
    (
      "optimization-overclaim",
      "held.macro-only-self-p-puck-wrapper-cold-start-repeat.optimization-overclaim",
    ),
    (
      "runtime-overclaim",
      "held.macro-only-self-p-puck-wrapper-cold-start-repeat.runtime-overclaim",
    ),
    (
      "authority-overclaim",
      "held.macro-only-self-p-puck-wrapper-cold-start-repeat.authority-overclaim",
    ),
    (
      "gpl-claim",
      "held.macro-only-self-p-puck-wrapper-cold-start-repeat.gpl-family-dependency",
    ),
  ] {
    let output = get(&run, key);
    assert_eq!(as_str(get(output, "status")), "Held", "{key}");
    assert_eq!(as_str(get(output, "held-id")), held_id, "{key}");
    assert!(!as_bool(get(
      output,
      "p-puck-wrapper-cold-start-repeat-proof"
    )));
    assert!(!as_bool(get(output, "optimization-selected")));
    assert!(!as_bool(get(output, "global-ontology-runtime")));
  }
}
