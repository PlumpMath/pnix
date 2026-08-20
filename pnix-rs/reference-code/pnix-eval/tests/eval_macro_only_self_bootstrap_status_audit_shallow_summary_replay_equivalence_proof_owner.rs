use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/pnix-query-runtime/macro-only-self-bootstrap-status-audit-shallow-summary-replay-equivalence-proof-owner.px",
  )
}

fn full_audit_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/tesseract-macro-legacy-probe/macro_ontology_bootstrap_status_audit_receipt.px",
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

fn records_by_field(v: &Value) -> BTreeMap<&str, &Value> {
  as_list(v)
    .iter()
    .map(|item| (as_str(get(item, "field")), item))
    .collect()
}

fn with_run(f: impl FnOnce(Value) + Send + 'static) {
  std::thread::Builder::new()
    .name("bootstrap-shallow-summary-replay-equivalence-owner-eval".to_string())
    .stack_size(32 * 1024 * 1024)
    .spawn(move || {
      let run =
        eval_file(&fixture_path()).expect("shallow summary replay equivalence owner fixture");
      f(run);
    })
    .expect("spawn eval thread")
    .join()
    .expect("eval thread panicked");
}

#[test]
fn fixture_uses_owner_and_bounded_full_audit_projection_source() {
  with_run(|run| {
    assert_eq!(
      as_str(get(&run, "proof")),
      "macro-only-self-bootstrap-status-audit-shallow-summary-replay-equivalence-proof-owner"
    );
    assert!(as_bool(get(&run, "imported-owner")));
    assert!(!as_bool(get(
      &run,
      "imported-full-audit-for-bounded-projection"
    )));
    assert!(as_bool(get(
      &run,
      "used-bounded-full-audit-projection-source"
    )));
    assert!(as_bool(get(&run, "used-px-owner")));
  });
}

#[test]
fn bounded_projection_source_matches_current_full_audit_top_level_status_lines() {
  let source = fs::read_to_string(full_audit_path()).expect("full audit receipt source");
  for expected in [
    "boot-executed = true;",
    "macro-only-runtime-owner-booted = true;",
    "semantic-owner = true;",
    "new-engine-from-zero = false;",
    "host-code-removal-started = false;",
    "global-ontology-runtime = false;",
    "runtime-api-flattening = false;",
    "meaning-db = false;",
    "implementation-command = false;",
  ] {
    assert!(
      source.contains(expected),
      "full audit source missing `{expected}`"
    );
  }
}

#[test]
fn owner_meta_proves_bounded_replay_equivalence_without_promoting_fast_path() {
  with_run(|run| {
    let meta = get(&run, "owner-meta");
    assert_eq!(
      as_str(get(meta, "owner")),
      "stdlib.lib.gate.macro-only-self-bootstrap-status-audit-shallow-summary-replay-equivalence-proof"
    );
    assert_eq!(
      as_str(get(meta, "constructor")),
      "validateSelfBootstrapStatusAuditShallowSummaryReplayEquivalenceProof"
    );
    assert!(as_bool(get(
      meta,
      "self-bootstrap-status-audit-shallow-summary-replay-equivalence-proof"
    )));
    assert!(as_bool(get(meta, "bounded-field-replay-equivalence")));
    assert!(as_bool(get(meta, "replay-equivalence-proven")));
    assert!(!as_bool(get(meta, "whole-json-equivalence-proven")));
    assert_eq!(as_i64(get(meta, "projection-field-count")), 11);
    assert_eq!(as_i64(get(meta, "direct-full-audit-field-count")), 8);
    assert_eq!(as_i64(get(meta, "derived-boundary-field-count")), 3);
    assert!(as_bool(get(meta, "fast-path-promotion-eligible")));
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
fn projection_records_cover_direct_and_derived_boundary_sources() {
  with_run(|run| {
    let direct = string_set(get(&run, "direct-full-audit-fields"));
    let derived = string_set(get(&run, "derived-boundary-fields"));
    assert_eq!(direct.len(), 8);
    assert_eq!(derived.len(), 3);
    for expected in [
      "new-engine-from-zero",
      "macro-only-runtime-owner",
      "semantic-owner",
      "boot-executed",
      "host-code-removal-started",
      "global-ontology-runtime",
      "runtime-api-flattening",
      "meaning-db",
    ] {
      assert!(
        direct.contains(expected),
        "missing direct field `{expected}`"
      );
    }
    for expected in [
      "optimization-applied",
      "fast-path-promoted",
      "external-solver-installed",
    ] {
      assert!(
        derived.contains(expected),
        "missing derived field `{expected}`"
      );
    }

    let records = records_by_field(get(&run, "actual-projection-records"));
    assert_eq!(records.len(), 11);
    for field in direct {
      let record = records[field];
      assert_eq!(
        as_str(get(record, "full-audit-source")),
        "direct-full-audit-field"
      );
      assert_eq!(as_str(get(record, "boundary-derivation")), "");
      assert!(as_bool(get(record, "equivalent")));
    }
    for field in derived {
      let record = records[field];
      assert_eq!(as_str(get(record, "full-audit-source")), "derived-boundary");
      assert_ne!(as_str(get(record, "boundary-derivation")), "");
      assert!(!as_bool(get(record, "shallow-value")));
      assert!(!as_bool(get(record, "full-audit-value")));
      assert!(as_bool(get(record, "equivalent")));
    }
  });
}

#[test]
fn full_audit_projection_matches_expected_boot_status_values() {
  with_run(|run| {
    let projection = get(&run, "full-audit-direct-status-projection");
    assert!(!as_bool(get(projection, "new-engine-from-zero")));
    assert!(as_bool(get(projection, "macro-only-runtime-owner")));
    assert!(as_bool(get(projection, "semantic-owner")));
    assert!(as_bool(get(projection, "boot-executed")));
    assert!(!as_bool(get(projection, "host-code-removal-started")));
    assert!(!as_bool(get(projection, "global-ontology-runtime")));
    assert!(!as_bool(get(projection, "runtime-api-flattening")));
    assert!(!as_bool(get(projection, "meaning-db")));
  });
}

#[test]
fn valid_proof_closes_replay_equivalence_and_opens_fast_path_promotion_proof() {
  with_run(|run| {
    let valid = get(&run, "valid-proof");
    assert_eq!(
      as_str(get(valid, "status")),
      "self-bootstrap-status-audit-shallow-summary-replay-equivalence-proof-present"
    );
    assert!(matches!(get(valid, "held-id"), Value::Null));
    assert!(as_bool(get(
      valid,
      "self-bootstrap-status-audit-shallow-summary-replay-equivalence-proof"
    )));
    assert!(as_bool(get(valid, "bounded-field-replay-equivalence")));
    assert!(as_bool(get(valid, "replay-equivalence-proven")));
    assert!(!as_bool(get(valid, "whole-json-equivalence-proven")));
    assert!(as_bool(get(valid, "fast-path-promotion-eligible")));
    assert!(!as_bool(get(valid, "fast-path-promoted")));

    let closed = string_set(get(valid, "closes"));
    assert!(
      closed.contains("need.self.bootstrap-status-audit-shallow-summary-replay-equivalence-proof")
    );
    let open = string_set(get(valid, "next-open-frontiers"));
    assert!(
      !open.contains("need.self.bootstrap-status-audit-shallow-summary-replay-equivalence-proof")
    );
    assert!(
      open.contains("need.self.bootstrap-status-audit-shallow-summary-fast-path-promotion-proof")
    );
  });
}

#[test]
fn required_evidence_preserves_boundaries() {
  with_run(|run| {
    let evidence = string_set(get(&run, "required-evidence"));
    for expected in [
      "self-bootstrap-status-audit-shallow-summary-owner-proof-present",
      "bounded-full-audit-status-projection-present",
      "shallow-summary-status-projection-present",
      "projection-field-set-matches",
      "all-projection-fields-equivalent",
      "direct-and-derived-boundary-sources-classified",
      "whole-json-equivalence-not-claimed",
      "full-audit-json-not-imported-by-shallow-summary",
      "fast-path-promotion-deferred",
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
fn held_failures_cover_projection_frontier_and_overclaims() {
  with_run(|run| {
    for (field, held_id) in [
      (
        "wrong-proof",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-replay-equivalence.proof-id-mismatch",
      ),
      (
        "stale-stage",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-replay-equivalence.stale-current-stage",
      ),
      (
        "source-mismatch",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-replay-equivalence.source-mismatch",
      ),
      (
        "summary-owner-missing",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-replay-equivalence.summary-owner-missing",
      ),
      (
        "projection-source-missing",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-replay-equivalence.projection-source-missing",
      ),
      (
        "projection-shape-mismatch",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-replay-equivalence.projection-shape-mismatch",
      ),
      (
        "projection-value-mismatch",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-replay-equivalence.projection-value-mismatch",
      ),
      (
        "missing-evidence",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-replay-equivalence.missing-required-evidence",
      ),
      (
        "frontier-shape-mismatch",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-replay-equivalence.frontier-shape-mismatch",
      ),
      (
        "whole-json-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-replay-equivalence.whole-json-overclaim",
      ),
      (
        "optimization-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-replay-equivalence.optimization-overclaim",
      ),
      (
        "external-or-license-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-replay-equivalence.external-or-license-overclaim",
      ),
      (
        "runtime-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-replay-equivalence.runtime-overclaim",
      ),
      (
        "authority-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-replay-equivalence.authority-overclaim",
      ),
    ] {
      let result = get(&run, field);
      assert_eq!(as_str(get(result, "status")), "Held");
      assert_eq!(as_str(get(result, "held-id")), held_id);
      assert!(!as_bool(get(
        result,
        "self-bootstrap-status-audit-shallow-summary-replay-equivalence-proof"
      )));
      assert!(!as_bool(get(result, "fast-path-promoted")));
    }
  });
}

#[test]
fn final_fixture_flags_keep_fast_path_deferred() {
  with_run(|run| {
    assert!(as_bool(get(
      &run,
      "self-bootstrap-status-audit-shallow-summary-replay-equivalence-proof"
    )));
    assert!(as_bool(get(&run, "bounded-field-replay-equivalence")));
    assert!(as_bool(get(&run, "replay-equivalence-proven")));
    assert!(!as_bool(get(&run, "whole-json-equivalence-proven")));
    assert!(!as_bool(get(
      &run,
      "full-audit-json-imported-by-shallow-summary"
    )));
    assert!(as_bool(get(&run, "fast-path-promotion-eligible")));
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
