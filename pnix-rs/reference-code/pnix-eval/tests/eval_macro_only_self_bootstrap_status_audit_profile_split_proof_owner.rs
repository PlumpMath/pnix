use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/pnix-query-runtime/macro-only-self-bootstrap-status-audit-profile-split-proof-owner.px",
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

fn attrs_by_id(v: &Value) -> BTreeMap<&str, &Value> {
  as_list(v)
    .iter()
    .map(|item| (as_str(get(item, "id")), item))
    .collect()
}

fn with_run(f: impl FnOnce(Value) + Send + 'static) {
  std::thread::Builder::new()
    .name("bootstrap-profile-split-owner-eval".to_string())
    .stack_size(32 * 1024 * 1024)
    .spawn(move || {
      let run =
        eval_file(&fixture_path()).expect("bootstrap status audit profile split owner fixture");
      f(run);
    })
    .expect("spawn eval thread")
    .join()
    .expect("eval thread panicked");
}

#[test]
fn fixture_imports_owner_and_records_profile_source() {
  with_run(|run| {
    assert_eq!(
      as_str(get(&run, "proof")),
      "macro-only-self-bootstrap-status-audit-profile-split-proof-owner"
    );
    assert!(as_bool(get(&run, "imported-owner")));
    assert!(as_bool(get(&run, "used-px-owner")));
    assert_eq!(
      as_str(get(&run, "profile-proof-source")),
      "actual p-puck probe-marker repeat plus terminated cargo test lower-bound over bootstrap status audit"
    );
  });
}

#[test]
fn owner_meta_splits_bootstrap_profile_without_selecting_optimization() {
  with_run(|run| {
    let meta = get(&run, "owner-meta");
    assert_eq!(
      as_str(get(meta, "owner")),
      "stdlib.lib.gate.macro-only-self-bootstrap-status-audit-profile-split-proof"
    );
    assert_eq!(
      as_str(get(meta, "constructor")),
      "validateSelfBootstrapStatusAuditProfileSplitProof"
    );
    assert!(as_bool(get(
      meta,
      "bootstrap-status-audit-profile-split-proof"
    )));
    assert!(as_bool(get(
      meta,
      "bootstrap-probe-marker-repeat-within-threshold"
    )));
    assert!(!as_bool(get(meta, "marker-import-persistent-bottleneck")));
    assert!(as_bool(get(
      meta,
      "full-bootstrap-status-audit-json-test-path-bottleneck"
    )));
    assert!(as_bool(get(
      meta,
      "optimization-candidate-ready-after-profile-split"
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
  });
}

#[test]
fn telemetry_and_profile_records_are_pinned() {
  with_run(|run| {
    assert_eq!(as_i64(get(&run, "initial-probe-duration-ms")), 11167);
    assert_eq!(as_i64(get(&run, "repeat-probe-duration-ms")), 1541);
    assert_eq!(as_i64(get(&run, "full-test-lower-bound-ms")), 60000);
    assert_eq!(as_i64(get(&run, "slow-threshold-ms")), 5000);
    assert_eq!(
      as_str(get(&run, "expected-initial-probe-status")),
      "slow-path-candidate"
    );
    assert_eq!(
      as_str(get(&run, "expected-repeat-probe-status")),
      "within-threshold"
    );
    assert_eq!(
      as_str(get(&run, "expected-full-test-status")),
      "long-running-terminated-after-60s"
    );

    let records = attrs_by_id(get(&run, "expected-profile-records"));
    assert_eq!(records.len(), 3);
    assert_eq!(
      as_i64(get(
        records["profile.bootstrap-status-audit.probe-marker.repeat-within-threshold"],
        "duration-ms"
      )),
      1541
    );
    assert!(as_bool(get(
      records["profile.bootstrap-status-audit.full-json-test-path.long-running"],
      "is-bottleneck"
    )));
  });
}

#[test]
fn valid_proof_closes_profile_split_frontier_only() {
  with_run(|run| {
    let valid = get(&run, "valid-proof");
    assert_eq!(
      as_str(get(valid, "status")),
      "self-bootstrap-status-audit-profile-split-proof-present"
    );
    assert!(matches!(get(valid, "held-id"), Value::Null));
    assert!(as_bool(get(
      valid,
      "bootstrap-status-audit-profile-split-proof"
    )));
    assert!(as_bool(get(
      valid,
      "bootstrap-probe-marker-repeat-within-threshold"
    )));
    assert!(!as_bool(get(valid, "marker-import-persistent-bottleneck")));
    assert!(as_bool(get(
      valid,
      "full-bootstrap-status-audit-json-test-path-bottleneck"
    )));
    assert!(as_bool(get(
      valid,
      "optimization-candidate-ready-after-profile-split"
    )));
    assert!(!as_bool(get(valid, "optimization-selected")));
    assert!(!as_bool(get(valid, "fast-path-promoted")));
    assert!(!as_bool(get(valid, "runtime-api-flattening")));
    assert!(!as_bool(get(valid, "meaning-db")));
  });
}

#[test]
fn evidence_and_remaining_frontiers_are_explicit() {
  with_run(|run| {
    let evidence = string_set(get(&run, "required-evidence"));
    for expected in [
      "self-p-puck-wrapper-cold-start-repeat-proof-present",
      "bootstrap-probe-marker-repeat-within-threshold-recorded",
      "full-bootstrap-status-audit-test-lower-bound-recorded",
      "profile-split-separates-marker-import-from-full-json-test-path",
      "optimization-deferred-after-profile-split",
      "no-p-puck-semantic-owner",
    ] {
      assert!(evidence.contains(expected), "missing evidence `{expected}`");
    }

    let frontiers = string_set(get(&run, "remaining-open-frontiers"));
    assert!(!frontiers.contains("need.self.bootstrap-status-audit-profile-split-proof"));
    assert!(frontiers.contains("need.self.optimization-candidate-after-bottleneck-attribution"));
  });
}

#[test]
fn stale_wrong_source_profile_and_frontier_cases_are_held() {
  with_run(|run| {
    for (key, held_id) in [
      (
        "wrong-proof",
        "held.macro-only-self-bootstrap-status-audit-profile-split.proof-id-mismatch",
      ),
      (
        "stale-stage",
        "held.macro-only-self-bootstrap-status-audit-profile-split.stale-current-stage",
      ),
      (
        "source-mismatch",
        "held.macro-only-self-bootstrap-status-audit-profile-split.source-mismatch",
      ),
      (
        "source-evidence-missing",
        "held.macro-only-self-bootstrap-status-audit-profile-split.source-evidence-missing",
      ),
      (
        "profile-record-shape-mismatch",
        "held.macro-only-self-bootstrap-status-audit-profile-split.profile-record-shape-mismatch",
      ),
      (
        "profile-record-invalid",
        "held.macro-only-self-bootstrap-status-audit-profile-split.profile-record-invalid",
      ),
      (
        "timing-summary-drift",
        "held.macro-only-self-bootstrap-status-audit-profile-split.timing-summary-drift",
      ),
      (
        "missing-evidence",
        "held.macro-only-self-bootstrap-status-audit-profile-split.missing-required-evidence",
      ),
      (
        "frontier-shape-mismatch",
        "held.macro-only-self-bootstrap-status-audit-profile-split.frontier-shape-mismatch",
      ),
    ] {
      let output = get(&run, key);
      assert_eq!(as_str(get(output, "status")), "Held", "{key}");
      assert_eq!(as_str(get(output, "held-id")), held_id, "{key}");
      assert!(!as_bool(get(
        output,
        "bootstrap-status-audit-profile-split-proof"
      )));
    }
  });
}

#[test]
fn split_optimization_runtime_authority_and_gpl_overclaims_are_held() {
  with_run(|run| {
    for (key, held_id) in [
      (
        "split-overclaim",
        "held.macro-only-self-bootstrap-status-audit-profile-split.split-overclaim-or-underclaim",
      ),
      (
        "optimization-overclaim",
        "held.macro-only-self-bootstrap-status-audit-profile-split.optimization-overclaim",
      ),
      (
        "runtime-overclaim",
        "held.macro-only-self-bootstrap-status-audit-profile-split.runtime-overclaim",
      ),
      (
        "authority-overclaim",
        "held.macro-only-self-bootstrap-status-audit-profile-split.authority-overclaim",
      ),
      (
        "gpl-claim",
        "held.macro-only-self-bootstrap-status-audit-profile-split.gpl-family-dependency",
      ),
    ] {
      let output = get(&run, key);
      assert_eq!(as_str(get(output, "status")), "Held", "{key}");
      assert_eq!(as_str(get(output, "held-id")), held_id, "{key}");
      assert!(!as_bool(get(
        output,
        "bootstrap-status-audit-profile-split-proof"
      )));
      assert!(!as_bool(get(output, "optimization-selected")));
      assert!(!as_bool(get(output, "global-ontology-runtime")));
    }
  });
}
