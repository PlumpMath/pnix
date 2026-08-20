use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/pnix-query-runtime/macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-promotion-proof-owner.px",
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
    .name("bootstrap-shallow-summary-fast-path-promotion-owner-eval".to_string())
    .stack_size(32 * 1024 * 1024)
    .spawn(move || {
      let run =
        eval_file(&fixture_path()).expect("shallow summary fast-path promotion owner fixture");
      f(run);
    })
    .expect("spawn eval thread")
    .join()
    .expect("eval thread panicked");
}

#[test]
fn fixture_imports_promotion_owner_and_replay_source() {
  with_run(|run| {
    assert_eq!(
      as_str(get(&run, "proof")),
      "macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-promotion-proof-owner"
    );
    assert!(as_bool(get(&run, "imported-owner")));
    assert!(as_bool(get(&run, "imported-replay-owner")));
    assert!(as_bool(get(&run, "imported-replay-fixture")));
    assert!(as_bool(get(&run, "used-px-owner")));
  });
}

#[test]
fn owner_meta_promotes_fast_path_without_runtime_binding() {
  with_run(|run| {
    let meta = get(&run, "owner-meta");
    assert_eq!(
      as_str(get(meta, "owner")),
      "stdlib.lib.gate.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-promotion-proof"
    );
    assert_eq!(
      as_str(get(meta, "constructor")),
      "validateSelfBootstrapStatusAuditShallowSummaryFastPathPromotionProof"
    );
    assert!(as_bool(get(
      meta,
      "self-bootstrap-status-audit-shallow-summary-fast-path-promotion-proof"
    )));
    assert!(as_bool(get(meta, "bounded-field-replay-equivalence")));
    assert!(as_bool(get(meta, "replay-equivalence-proven")));
    assert!(!as_bool(get(meta, "whole-json-equivalence-proven")));
    assert_eq!(as_i64(get(meta, "projection-field-count")), 11);
    assert_eq!(as_i64(get(meta, "direct-full-audit-field-count")), 8);
    assert_eq!(as_i64(get(meta, "derived-boundary-field-count")), 3);
    assert!(as_bool(get(meta, "fast-path-promotion-eligible")));
    assert!(as_bool(get(meta, "fast-path-promoted")));
    assert!(as_bool(get(meta, "optimization-selected")));
    assert!(as_bool(get(meta, "optimization-implementation-selected")));
    assert_eq!(
      as_str(get(meta, "optimization-implementation-kind")),
      "bounded-macro-route-candidate-not-runtime-installer"
    );
    for key in [
      "optimization-applied",
      "runtime-binding-installed",
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
fn promotion_record_preserves_source_replay_and_route_shape() {
  with_run(|run| {
    let record = get(&run, "promotion-record");
    assert_eq!(
      as_str(get(record, "id")),
      "promotion.fast-path.bootstrap-status-audit.shallow-summary.v1"
    );
    assert_eq!(
      as_str(get(record, "route-id")),
      "route.fast-path.bootstrap-status-audit.shallow-summary.v1"
    );
    assert_eq!(
      as_str(get(record, "route-owner")),
      "owner.summary.bootstrap-status-audit.shallow.v1"
    );
    assert_eq!(
      as_str(get(record, "source-replay-equivalence-proof")),
      "proof.macro-only.self.bootstrap-status-audit-shallow-summary-replay-equivalence.v1"
    );
    assert!(as_bool(get(record, "bounded-field-replay-equivalence")));
    assert!(as_bool(get(record, "full-audit-fallback-preserved")));
    assert!(as_bool(get(record, "fast-path-promoted")));
    assert!(as_bool(get(record, "optimization-selected")));
    assert!(as_bool(get(record, "optimization-implementation-selected")));
    assert!(!as_bool(get(record, "optimization-applied")));
    assert!(!as_bool(get(record, "runtime-binding-installed")));
  });
}

#[test]
fn valid_proof_closes_promotion_and_opens_runtime_binding_proof() {
  with_run(|run| {
    let valid = get(&run, "valid-proof");
    assert_eq!(
      as_str(get(valid, "status")),
      "self-bootstrap-status-audit-shallow-summary-fast-path-promotion-proof-present"
    );
    assert!(matches!(get(valid, "held-id"), Value::Null));
    assert!(as_bool(get(
      valid,
      "self-bootstrap-status-audit-shallow-summary-fast-path-promotion-proof"
    )));
    assert!(as_bool(get(valid, "fast-path-promoted")));
    assert!(as_bool(get(valid, "optimization-selected")));
    assert!(as_bool(get(valid, "optimization-implementation-selected")));
    assert!(!as_bool(get(valid, "optimization-applied")));
    assert!(!as_bool(get(valid, "runtime-binding-installed")));

    let closed = string_set(get(valid, "closes"));
    assert!(
      closed.contains("need.self.bootstrap-status-audit-shallow-summary-fast-path-promotion-proof")
    );
    let open = string_set(get(valid, "next-open-frontiers"));
    assert!(
      !open.contains("need.self.bootstrap-status-audit-shallow-summary-fast-path-promotion-proof")
    );
    assert!(open.contains(
      "need.self.bootstrap-status-audit-shallow-summary-fast-path-runtime-binding-proof"
    ));
  });
}

#[test]
fn required_evidence_keeps_promotion_narrow() {
  with_run(|run| {
    let evidence = string_set(get(&run, "required-evidence"));
    for expected in [
      "self-bootstrap-status-audit-shallow-summary-replay-equivalence-proof-present",
      "bounded-field-replay-equivalence-proven",
      "whole-json-equivalence-not-claimed",
      "full-audit-fallback-preserved",
      "promotion-record-present",
      "promotion-route-owner-is-shallow-summary-owner",
      "optimization-selected",
      "optimization-implementation-selected",
      "optimization-application-deferred",
      "runtime-binding-deferred",
      "no-runtime-install",
      "no-runtime-api-flattening",
      "no-meaning-db",
      "no-external-solver-intake",
      "no-p-puck-semantic-owner",
      "no-gpl-family-dependencies",
    ] {
      assert!(evidence.contains(expected), "missing evidence `{expected}`");
    }
  });
}

#[test]
fn held_failures_cover_replay_promotion_runtime_and_authority_overclaims() {
  with_run(|run| {
    for (field, held_id) in [
      (
        "wrong-proof",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-promotion.proof-id-mismatch",
      ),
      (
        "stale-stage",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-promotion.stale-current-stage",
      ),
      (
        "source-mismatch",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-promotion.source-mismatch",
      ),
      (
        "replay-evidence-missing",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-promotion.replay-evidence-missing",
      ),
      (
        "promotion-record-mismatch",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-promotion.promotion-record-mismatch",
      ),
      (
        "missing-evidence",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-promotion.missing-required-evidence",
      ),
      (
        "frontier-shape-mismatch",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-promotion.frontier-shape-mismatch",
      ),
      (
        "whole-json-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-promotion.whole-json-overclaim",
      ),
      (
        "runtime-binding-or-application-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-promotion.runtime-binding-or-application-overclaim",
      ),
      (
        "external-or-license-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-promotion.external-or-license-overclaim",
      ),
      (
        "runtime-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-promotion.runtime-overclaim",
      ),
      (
        "authority-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-promotion.authority-overclaim",
      ),
    ] {
      let result = get(&run, field);
      assert_eq!(as_str(get(result, "status")), "Held");
      assert_eq!(as_str(get(result, "held-id")), held_id);
      assert!(!as_bool(get(
        result,
        "self-bootstrap-status-audit-shallow-summary-fast-path-promotion-proof"
      )));
      assert!(!as_bool(get(result, "fast-path-promoted")));
    }
  });
}

#[test]
fn final_fixture_flags_promote_only_the_route_candidate() {
  with_run(|run| {
    assert!(as_bool(get(
      &run,
      "self-bootstrap-status-audit-shallow-summary-fast-path-promotion-proof"
    )));
    assert!(as_bool(get(&run, "bounded-field-replay-equivalence")));
    assert!(as_bool(get(&run, "replay-equivalence-proven")));
    assert!(!as_bool(get(&run, "whole-json-equivalence-proven")));
    assert!(as_bool(get(&run, "full-audit-fallback-preserved")));
    assert!(as_bool(get(&run, "fast-path-promotion-eligible")));
    assert!(as_bool(get(&run, "fast-path-promoted")));
    assert!(as_bool(get(&run, "optimization-selected")));
    assert!(as_bool(get(&run, "optimization-implementation-selected")));
    for key in [
      "optimization-applied",
      "runtime-binding-installed",
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
