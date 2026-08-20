use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/pnix-query-runtime/macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application-proof-owner.px",
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
    .name("bootstrap-shallow-summary-fast-path-application-owner-eval".to_string())
    .stack_size(32 * 1024 * 1024)
    .spawn(move || {
      let run =
        eval_file(&fixture_path()).expect("shallow summary fast-path application owner fixture");
      f(run);
    })
    .expect("spawn eval thread")
    .join()
    .expect("eval thread panicked");
}

#[test]
fn fixture_imports_application_owner_and_binding_source() {
  with_run(|run| {
    assert_eq!(
      as_str(get(&run, "proof")),
      "macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application-proof-owner"
    );
    assert!(as_bool(get(&run, "imported-owner")));
    assert!(as_bool(get(&run, "imported-binding-owner")));
    assert!(as_bool(get(&run, "imported-binding-fixture")));
    assert!(as_bool(get(&run, "used-px-owner")));
  });
}

#[test]
fn owner_meta_applies_only_selected_callsite() {
  with_run(|run| {
    let meta = get(&run, "owner-meta");
    assert_eq!(
      as_str(get(meta, "owner")),
      "stdlib.lib.gate.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application-proof"
    );
    assert_eq!(
      as_str(get(meta, "constructor")),
      "validateSelfBootstrapStatusAuditShallowSummaryFastPathApplicationProof"
    );
    assert!(as_bool(get(
      meta,
      "self-bootstrap-status-audit-shallow-summary-fast-path-application-proof"
    )));
    assert!(as_bool(get(meta, "selected-callsite-replaced")));
    assert_eq!(as_i64(get(meta, "selected-callsite-count")), 1);
    assert!(as_bool(get(meta, "scoped-optimization-applied")));
    assert!(as_bool(get(meta, "optimization-applied")));
    assert!(as_bool(get(meta, "runtime-binding-installed")));
    assert!(as_bool(get(meta, "bounded-runtime-binding")));
    assert!(as_bool(get(meta, "measurement-required")));
    for key in [
      "global-optimization-applied",
      "global-default-callsite-replaced",
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
    ] {
      assert!(!as_bool(get(meta, key)), "`{key}` must stay false");
    }
  });
}

#[test]
fn application_record_and_positive_call_return_bound_status() {
  with_run(|run| {
    let record = get(&run, "application-record");
    assert_eq!(
      as_str(get(record, "id")),
      "application.fast-path.bootstrap-status-audit.shallow-summary.selected-callsite.v1"
    );
    assert_eq!(
      as_str(get(record, "callsite-id")),
      "callsite.bootstrap-status-audit.current-status.shallow-summary.v1"
    );
    assert!(as_bool(get(record, "selected-callsite-replaced")));
    assert!(as_bool(get(record, "scoped-optimization-applied")));
    assert!(as_bool(get(record, "optimization-applied")));
    assert!(!as_bool(get(record, "global-optimization-applied")));
    assert!(!as_bool(get(record, "global-default-callsite-replaced")));

    let call = get(&run, "positive-applied-callsite-call");
    assert_eq!(
      as_str(get(call, "status")),
      "fast-path-applied-shallow-summary-read"
    );
    assert!(as_bool(get(call, "selected-callsite-replaced")));
    assert!(as_bool(get(call, "runtime-binding-installed")));
    assert_eq!(as_i64(get(call, "status-field-count")), 11);
    let status = get(call, "bound-status-record");
    assert!(as_bool(get(status, "macro-only-runtime-owner")));
    assert!(as_bool(get(status, "semantic-owner")));
    assert!(as_bool(get(status, "boot-executed")));
    assert!(as_bool(get(status, "fast-path-promoted")));
    assert!(!as_bool(get(status, "external-solver-installed")));
  });
}

#[test]
fn application_constructor_holds_unselected_and_broken_calls() {
  with_run(|run| {
    for (field, held_id) in [
      (
        "negative-held-unselected-callsite",
        "held.bootstrap-status-shallow-summary-fast-path-application.callsite-mismatch",
      ),
      (
        "negative-held-broken-binding-callsite",
        "held.bootstrap-status-shallow-summary-fast-path-application.route-result-held",
      ),
      (
        "negative-held-field-shape-callsite",
        "held.bootstrap-status-shallow-summary-fast-path-application.route-result-held",
      ),
    ] {
      let result = get(&run, field);
      assert_eq!(as_str(get(result, "status")), "Held");
      assert_eq!(as_str(get(result, "held-id")), held_id);
      assert!(!as_bool(get(result, "selected-callsite-replaced")));
    }
  });
}

#[test]
fn valid_proof_closes_application_and_opens_measurement() {
  with_run(|run| {
    let valid = get(&run, "valid-proof");
    assert_eq!(
      as_str(get(valid, "status")),
      "self-bootstrap-status-audit-shallow-summary-fast-path-application-proof-present"
    );
    assert!(matches!(get(valid, "held-id"), Value::Null));
    assert!(as_bool(get(valid, "selected-callsite-replaced")));
    assert!(as_bool(get(valid, "scoped-optimization-applied")));
    assert!(as_bool(get(valid, "optimization-applied")));
    assert!(!as_bool(get(valid, "global-optimization-applied")));
    assert!(!as_bool(get(valid, "global-default-callsite-replaced")));
    assert!(as_bool(get(valid, "measurement-required")));

    let closed = string_set(get(valid, "closes"));
    assert!(closed
      .contains("need.self.bootstrap-status-audit-shallow-summary-fast-path-application-proof"));
    let open = string_set(get(valid, "next-open-frontiers"));
    assert!(open.contains(
      "need.self.bootstrap-status-audit-shallow-summary-fast-path-application-measurement-proof"
    ));
  });
}

#[test]
fn held_failures_cover_application_runtime_external_and_authority_overclaims() {
  with_run(|run| {
    for (field, held_id) in [
      (
        "wrong-proof",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application.proof-id-mismatch",
      ),
      (
        "stale-stage",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application.stale-current-stage",
      ),
      (
        "source-mismatch",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application.source-mismatch",
      ),
      (
        "binding-evidence-missing",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application.binding-evidence-missing",
      ),
      (
        "application-record-mismatch",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application.application-record-mismatch",
      ),
      (
        "positive-call-missing",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application.positive-callsite-missing",
      ),
      (
        "negative-unselected-missing",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application.negative-unselected-missing",
      ),
      (
        "negative-broken-binding-missing",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application.negative-broken-binding-missing",
      ),
      (
        "scoped-application-missing",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application.scoped-application-missing",
      ),
      (
        "audit-or-measurement-missing",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application.audit-or-measurement-missing",
      ),
      (
        "missing-evidence",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application.missing-required-evidence",
      ),
      (
        "frontier-shape-mismatch",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application.frontier-shape-mismatch",
      ),
      (
        "global-application-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application.global-application-overclaim",
      ),
      (
        "runtime-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application.runtime-overclaim",
      ),
      (
        "external-or-license-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application.external-or-license-overclaim",
      ),
      (
        "authority-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application.authority-overclaim",
      ),
    ] {
      let result = get(&run, field);
      assert_eq!(as_str(get(result, "status")), "Held");
      assert_eq!(as_str(get(result, "held-id")), held_id);
      assert!(!as_bool(get(
        result,
        "self-bootstrap-status-audit-shallow-summary-fast-path-application-proof"
      )));
    }
  });
}

#[test]
fn final_fixture_flags_apply_only_scoped_callsite() {
  with_run(|run| {
    assert!(as_bool(get(
      &run,
      "self-bootstrap-status-audit-shallow-summary-fast-path-application-proof"
    )));
    assert!(as_bool(get(&run, "selected-callsite-replaced")));
    assert!(as_bool(get(&run, "scoped-optimization-applied")));
    assert!(as_bool(get(&run, "optimization-applied")));
    assert!(as_bool(get(&run, "runtime-binding-installed")));
    assert!(as_bool(get(&run, "bounded-runtime-binding")));
    assert!(as_bool(get(&run, "measurement-required")));
    for key in [
      "global-optimization-applied",
      "global-default-callsite-replaced",
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
