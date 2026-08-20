use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/pnix-query-runtime/macro-only-self-bootstrap-status-audit-shallow-summary-owner-proof-owner.px",
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

fn with_run(f: impl FnOnce(Value) + Send + 'static) {
  std::thread::Builder::new()
    .name("bootstrap-shallow-summary-owner-eval".to_string())
    .stack_size(32 * 1024 * 1024)
    .spawn(move || {
      let run = eval_file(&fixture_path()).expect("shallow summary owner fixture");
      f(run);
    })
    .expect("spawn eval thread")
    .join()
    .expect("eval thread panicked");
}

#[test]
fn fixture_imports_owner_and_records_candidate_source() {
  with_run(|run| {
    assert_eq!(
      as_str(get(&run, "proof")),
      "macro-only-self-bootstrap-status-audit-shallow-summary-owner-proof-owner"
    );
    assert!(as_bool(get(&run, "imported-owner")));
    assert!(as_bool(get(&run, "used-px-owner")));
    assert_eq!(
      as_str(get(&run, "summary-owner-source")),
      "optimization candidate after bottleneck attribution proof"
    );
  });
}

#[test]
fn owner_meta_proves_shallow_summary_without_installing_fast_path() {
  with_run(|run| {
    let meta = get(&run, "owner-meta");
    assert_eq!(
      as_str(get(meta, "owner")),
      "stdlib.lib.gate.macro-only-self-bootstrap-status-audit-shallow-summary-owner-proof"
    );
    assert_eq!(
      as_str(get(meta, "constructor")),
      "validateSelfBootstrapStatusAuditShallowSummaryOwnerProof"
    );
    assert!(as_bool(get(
      meta,
      "self-bootstrap-status-audit-shallow-summary-owner-proof"
    )));
    assert!(as_bool(get(meta, "shallow-summary-owner-ready")));
    assert_eq!(
      as_str(get(meta, "shallow-summary-owner-id")),
      "owner.summary.bootstrap-status-audit.shallow.v1"
    );
    assert!(as_bool(get(meta, "full-audit-replay-preserved")));
    assert!(!as_bool(get(meta, "imports-full-audit-json")));
    for key in [
      "replay-equivalence-proven",
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
fn expected_summary_owner_shape_is_bounded_and_replay_preserving() {
  with_run(|run| {
    let summary = get(&run, "expected-summary-owner");
    assert_eq!(
      as_str(get(summary, "id")),
      "owner.summary.bootstrap-status-audit.shallow.v1"
    );
    assert_eq!(
      as_str(get(summary, "source-candidate")),
      "candidate.optimization.bootstrap-status-audit.shallow-summary-owner.v1"
    );
    assert_eq!(
      as_str(get(summary, "full-audit-replay-path")),
      "fixtures/tesseract-macro-legacy-probe/macro_ontology_bootstrap_status_audit_receipt.px"
    );
    assert_eq!(as_i64(get(summary, "field-count")), 11);
    let fields = string_set(get(summary, "summary-fields"));
    for expected in [
      "new-engine-from-zero",
      "macro-only-runtime-owner",
      "semantic-owner",
      "boot-executed",
      "host-code-removal-started",
      "runtime-api-flattening",
      "meaning-db",
      "optimization-applied",
    ] {
      assert!(fields.contains(expected), "missing field `{expected}`");
    }
    assert!(as_bool(get(summary, "full-audit-replay-preserved")));
    assert!(!as_bool(get(summary, "imports-full-audit-json")));
    assert!(!as_bool(get(summary, "replay-equivalence-proven")));
  });
}

#[test]
fn valid_proof_closes_owner_frontier_and_opens_replay_equivalence() {
  with_run(|run| {
    let valid = get(&run, "valid-proof");
    assert_eq!(
      as_str(get(valid, "status")),
      "self-bootstrap-status-audit-shallow-summary-owner-proof-present"
    );
    assert!(matches!(get(valid, "held-id"), Value::Null));
    assert!(as_bool(get(
      valid,
      "self-bootstrap-status-audit-shallow-summary-owner-proof"
    )));
    assert!(as_bool(get(valid, "shallow-summary-owner-ready")));
    assert!(as_bool(get(valid, "full-audit-replay-preserved")));
    assert!(!as_bool(get(valid, "imports-full-audit-json")));
    assert!(!as_bool(get(valid, "replay-equivalence-proven")));

    let closed = string_set(get(valid, "closes"));
    assert!(closed.contains("need.self.bootstrap-status-audit-shallow-summary-owner-proof"));
    let open = string_set(get(valid, "next-open-frontiers"));
    assert!(!open.contains("need.self.bootstrap-status-audit-shallow-summary-owner-proof"));
    assert!(
      open.contains("need.self.bootstrap-status-audit-shallow-summary-replay-equivalence-proof")
    );
  });
}

#[test]
fn required_evidence_preserves_full_audit_and_runtime_boundaries() {
  with_run(|run| {
    let evidence = string_set(get(&run, "required-evidence"));
    for expected in [
      "self-optimization-candidate-after-bottleneck-attribution-proof-present",
      "summary-owner-shape-present",
      "summary-fields-bounded",
      "full-audit-replay-path-preserved",
      "summary-does-not-import-full-audit-json",
      "replay-equivalence-deferred",
      "no-fast-path-promotion",
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
fn held_failures_cover_source_summary_frontier_and_overclaims() {
  with_run(|run| {
    for (field, held_id) in [
      (
        "wrong-proof",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-owner.proof-id-mismatch",
      ),
      (
        "stale-stage",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-owner.stale-current-stage",
      ),
      (
        "source-mismatch",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-owner.source-mismatch",
      ),
      (
        "candidate-evidence-missing",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-owner.candidate-evidence-missing",
      ),
      (
        "summary-shape-mismatch",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-owner.summary-shape-mismatch",
      ),
      (
        "summary-boundary-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-owner.summary-boundary-overclaim",
      ),
      (
        "missing-evidence",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-owner.missing-required-evidence",
      ),
      (
        "frontier-shape-mismatch",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-owner.frontier-shape-mismatch",
      ),
      (
        "optimization-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-owner.optimization-overclaim",
      ),
      (
        "external-or-license-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-owner.external-or-license-overclaim",
      ),
      (
        "runtime-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-owner.runtime-overclaim",
      ),
      (
        "authority-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-owner.authority-overclaim",
      ),
    ] {
      let result = get(&run, field);
      assert_eq!(as_str(get(result, "status")), "Held");
      assert_eq!(as_str(get(result, "held-id")), held_id);
      assert!(!as_bool(get(
        result,
        "self-bootstrap-status-audit-shallow-summary-owner-proof"
      )));
      assert!(!as_bool(get(result, "optimization-applied")));
    }
  });
}

#[test]
fn final_fixture_flags_keep_summary_proof_only() {
  with_run(|run| {
    assert!(as_bool(get(
      &run,
      "self-bootstrap-status-audit-shallow-summary-owner-proof"
    )));
    assert!(as_bool(get(&run, "shallow-summary-owner-ready")));
    assert!(as_bool(get(&run, "full-audit-replay-preserved")));
    assert!(!as_bool(get(&run, "imports-full-audit-json")));
    for key in [
      "replay-equivalence-proven",
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
