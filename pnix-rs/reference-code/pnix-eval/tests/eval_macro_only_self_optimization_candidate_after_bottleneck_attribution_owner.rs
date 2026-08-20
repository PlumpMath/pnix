use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/pnix-query-runtime/macro-only-self-optimization-candidate-after-bottleneck-attribution-owner.px",
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

fn string_set(v: &Value) -> BTreeSet<&str> {
  as_list(v).iter().map(as_str).collect()
}

fn with_run(f: impl FnOnce(Value) + Send + 'static) {
  std::thread::Builder::new()
    .name("optimization-candidate-owner-eval".to_string())
    .stack_size(32 * 1024 * 1024)
    .spawn(move || {
      let run = eval_file(&fixture_path()).expect("optimization candidate owner fixture");
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
      "macro-only-self-optimization-candidate-after-bottleneck-attribution-owner"
    );
    assert!(as_bool(get(&run, "imported-owner")));
    assert!(as_bool(get(&run, "used-px-owner")));
    assert_eq!(
      as_str(get(&run, "optimization-candidate-source")),
      "bootstrap status audit profile split proof over actual p-puck/cargo timing evidence"
    );
  });
}

#[test]
fn owner_meta_selects_candidate_without_applying_optimization() {
  with_run(|run| {
    let meta = get(&run, "owner-meta");
    assert_eq!(
      as_str(get(meta, "owner")),
      "stdlib.lib.gate.macro-only-self-optimization-candidate-after-bottleneck-attribution"
    );
    assert_eq!(
      as_str(get(meta, "constructor")),
      "validateSelfOptimizationCandidateAfterBottleneckAttribution"
    );
    assert!(as_bool(get(
      meta,
      "self-optimization-candidate-after-bottleneck-attribution-proof"
    )));
    assert!(as_bool(get(meta, "optimization-candidate-selected")));
    assert!(as_bool(get(meta, "optimization-candidate-only")));
    assert_eq!(
      as_str(get(meta, "selected-candidate-target")),
      "full-bootstrap-status-audit-json-test-path"
    );
    assert_eq!(
      as_str(get(meta, "selected-candidate-kind")),
      "shallow-bootstrap-status-summary-owner"
    );
    for key in [
      "optimization-applied",
      "optimization-selected",
      "optimization-implementation-selected",
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
fn expected_candidate_shape_is_pinned() {
  with_run(|run| {
    let candidate = get(&run, "expected-candidate");
    assert_eq!(
      as_str(get(candidate, "id")),
      "candidate.optimization.bootstrap-status-audit.shallow-summary-owner.v1"
    );
    assert_eq!(
      as_str(get(candidate, "target")),
      "full-bootstrap-status-audit-json-test-path"
    );
    assert_eq!(
      as_str(get(candidate, "kind")),
      "shallow-bootstrap-status-summary-owner"
    );
    assert_eq!(
      as_str(get(candidate, "source-profile-record")),
      "profile.bootstrap-status-audit.full-json-test-path.long-running"
    );
    assert!(as_bool(get(candidate, "candidate-only")));
    assert!(!as_bool(get(candidate, "implementation-command")));
    assert!(!as_bool(get(candidate, "optimization-applied")));
    assert!(!as_bool(get(candidate, "runtime-api-flattening")));
    assert!(!as_bool(get(candidate, "gpl-family-dependencies")));
  });
}

#[test]
fn valid_proof_closes_attribution_frontier_and_opens_shallow_summary_frontier() {
  with_run(|run| {
    let valid = get(&run, "valid-proof");
    assert_eq!(
      as_str(get(valid, "status")),
      "self-optimization-candidate-after-bottleneck-attribution-proof-present"
    );
    assert!(matches!(get(valid, "held-id"), Value::Null));
    assert!(as_bool(get(
      valid,
      "self-optimization-candidate-after-bottleneck-attribution-proof"
    )));
    assert!(as_bool(get(valid, "optimization-candidate-selected")));
    assert!(as_bool(get(valid, "optimization-candidate-only")));
    assert_eq!(
      as_str(get(valid, "selected-candidate-id")),
      "candidate.optimization.bootstrap-status-audit.shallow-summary-owner.v1"
    );
    assert!(!as_bool(get(valid, "optimization-applied")));
    assert!(!as_bool(get(valid, "optimization-selected")));

    let closed = string_set(get(valid, "closes"));
    assert!(closed.contains("need.self.optimization-candidate-after-bottleneck-attribution"));
    let open = string_set(get(valid, "next-open-frontiers"));
    assert!(!open.contains("need.self.optimization-candidate-after-bottleneck-attribution"));
    assert!(open.contains("need.self.bootstrap-status-audit-shallow-summary-owner-proof"));
  });
}

#[test]
fn required_evidence_keeps_runtime_and_authority_boundaries() {
  with_run(|run| {
    let evidence = string_set(get(&run, "required-evidence"));
    for expected in [
      "self-bootstrap-status-audit-profile-split-proof-present",
      "full-bootstrap-status-audit-json-test-path-bottleneck",
      "selected-candidate-targets-full-json-test-path",
      "selected-candidate-preserves-full-audit-replay",
      "optimization-application-deferred",
      "no-runtime-api-flattening",
      "no-meaning-db",
      "no-p-puck-semantic-owner",
      "no-gpl-family-dependencies",
    ] {
      assert!(evidence.contains(expected), "missing evidence `{expected}`");
    }
  });
}

#[test]
fn held_failures_cover_source_candidate_frontier_and_overclaims() {
  with_run(|run| {
    for (field, held_id) in [
      (
        "wrong-proof",
        "held.macro-only-self-optimization-candidate-after-bottleneck-attribution.proof-id-mismatch",
      ),
      (
        "stale-stage",
        "held.macro-only-self-optimization-candidate-after-bottleneck-attribution.stale-current-stage",
      ),
      (
        "source-mismatch",
        "held.macro-only-self-optimization-candidate-after-bottleneck-attribution.source-mismatch",
      ),
      (
        "profile-evidence-missing",
        "held.macro-only-self-optimization-candidate-after-bottleneck-attribution.profile-evidence-missing",
      ),
      (
        "candidate-shape-mismatch",
        "held.macro-only-self-optimization-candidate-after-bottleneck-attribution.candidate-shape-mismatch",
      ),
      (
        "candidate-boundary-overclaim",
        "held.macro-only-self-optimization-candidate-after-bottleneck-attribution.candidate-boundary-overclaim",
      ),
      (
        "missing-evidence",
        "held.macro-only-self-optimization-candidate-after-bottleneck-attribution.missing-required-evidence",
      ),
      (
        "frontier-shape-mismatch",
        "held.macro-only-self-optimization-candidate-after-bottleneck-attribution.frontier-shape-mismatch",
      ),
      (
        "optimization-overclaim",
        "held.macro-only-self-optimization-candidate-after-bottleneck-attribution.optimization-overclaim",
      ),
      (
        "external-or-license-overclaim",
        "held.macro-only-self-optimization-candidate-after-bottleneck-attribution.external-or-license-overclaim",
      ),
      (
        "runtime-overclaim",
        "held.macro-only-self-optimization-candidate-after-bottleneck-attribution.runtime-overclaim",
      ),
      (
        "authority-overclaim",
        "held.macro-only-self-optimization-candidate-after-bottleneck-attribution.authority-overclaim",
      ),
    ] {
      let result = get(&run, field);
      assert_eq!(as_str(get(result, "status")), "Held");
      assert_eq!(as_str(get(result, "held-id")), held_id);
      assert!(!as_bool(get(result, "optimization-candidate-selected")));
      assert!(!as_bool(get(result, "optimization-applied")));
    }
  });
}

#[test]
fn final_fixture_flags_are_candidate_only() {
  with_run(|run| {
    assert!(as_bool(get(
      &run,
      "self-optimization-candidate-after-bottleneck-attribution-proof"
    )));
    assert!(as_bool(get(&run, "optimization-candidate-selected")));
    assert!(as_bool(get(&run, "optimization-candidate-only")));
    for key in [
      "optimization-applied",
      "optimization-selected",
      "optimization-implementation-selected",
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
      "implementation-command",
    ] {
      assert!(!as_bool(get(&run, key)), "`{key}` must stay false");
    }
  });
}
