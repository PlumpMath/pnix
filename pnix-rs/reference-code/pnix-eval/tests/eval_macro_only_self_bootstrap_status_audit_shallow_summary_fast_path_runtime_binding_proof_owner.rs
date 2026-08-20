use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/pnix-query-runtime/macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-runtime-binding-proof-owner.px",
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
    .name("bootstrap-shallow-summary-fast-path-runtime-binding-owner-eval".to_string())
    .stack_size(32 * 1024 * 1024)
    .spawn(move || {
      let run = eval_file(&fixture_path())
        .expect("shallow summary fast-path runtime binding owner fixture");
      f(run);
    })
    .expect("spawn eval thread")
    .join()
    .expect("eval thread panicked");
}

#[test]
fn fixture_imports_runtime_binding_owner_and_promotion_source() {
  with_run(|run| {
    assert_eq!(
      as_str(get(&run, "proof")),
      "macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-runtime-binding-proof-owner"
    );
    assert!(as_bool(get(&run, "imported-owner")));
    assert!(as_bool(get(&run, "imported-promotion-owner")));
    assert!(as_bool(get(&run, "imported-promotion-fixture")));
    assert!(as_bool(get(&run, "used-px-owner")));
  });
}

#[test]
fn owner_meta_installs_bounded_binding_without_applying_optimization() {
  with_run(|run| {
    let meta = get(&run, "owner-meta");
    assert_eq!(
      as_str(get(meta, "owner")),
      "stdlib.lib.gate.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-runtime-binding-proof"
    );
    assert_eq!(
      as_str(get(meta, "constructor")),
      "validateSelfBootstrapStatusAuditShallowSummaryFastPathRuntimeBindingProof"
    );
    assert!(as_bool(get(
      meta,
      "self-bootstrap-status-audit-shallow-summary-fast-path-runtime-binding-proof"
    )));
    assert!(as_bool(get(meta, "runtime-binding-installed")));
    assert!(as_bool(get(meta, "bounded-runtime-binding")));
    assert_eq!(as_i64(get(meta, "status-field-count")), 11);
    assert!(as_bool(get(meta, "fast-path-promoted")));
    assert!(as_bool(get(meta, "optimization-selected")));
    assert!(as_bool(get(meta, "optimization-implementation-selected")));
    for key in [
      "optimization-applied",
      "default-callsite-replaced",
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
fn binding_record_and_positive_call_return_promotion_aware_status() {
  with_run(|run| {
    let binding = get(&run, "runtime-binding-record");
    assert_eq!(
      as_str(get(binding, "id")),
      "binding.runtime.bootstrap-status-audit.shallow-summary.fast-path.v1"
    );
    assert_eq!(
      as_str(get(binding, "route-table-id")),
      "runtime.route-table.bootstrap-status-audit.shallow-summary.fast-path.v1"
    );
    assert_eq!(
      as_str(get(binding, "binding-scope")),
      "bootstrap-status-audit-shallow-summary-only"
    );
    assert!(as_bool(get(binding, "runtime-binding-installed")));
    assert!(!as_bool(get(binding, "default-callsite-replaced")));

    let call = get(&run, "positive-bound-status-call");
    assert_eq!(
      as_str(get(call, "status")),
      "runtime-bound-shallow-summary-read"
    );
    assert!(as_bool(get(call, "runtime-binding-installed")));
    assert_eq!(as_i64(get(call, "status-field-count")), 11);
    let status = get(call, "bound-status-record");
    assert!(as_bool(get(status, "macro-only-runtime-owner")));
    assert!(as_bool(get(status, "semantic-owner")));
    assert!(as_bool(get(status, "boot-executed")));
    assert!(as_bool(get(status, "fast-path-promoted")));
    assert!(!as_bool(get(status, "optimization-applied")));
    assert!(!as_bool(get(status, "external-solver-installed")));
  });
}

#[test]
fn route_constructor_holds_wrong_route_scope_fields_and_binding_record() {
  with_run(|run| {
    for (field, held_id) in [
      (
        "negative-held-route-call",
        "held.bootstrap-status-shallow-summary-runtime-binding.route-id-mismatch",
      ),
      (
        "scope-held-call",
        "held.bootstrap-status-shallow-summary-runtime-binding.effect-scope-mismatch",
      ),
      (
        "field-held-call",
        "held.bootstrap-status-shallow-summary-runtime-binding.field-shape-mismatch",
      ),
      (
        "binding-record-held-call",
        "held.bootstrap-status-shallow-summary-runtime-binding.binding-record-mismatch",
      ),
    ] {
      let result = get(&run, field);
      assert_eq!(as_str(get(result, "status")), "Held");
      assert_eq!(as_str(get(result, "held-id")), held_id);
      assert!(!as_bool(get(result, "runtime-binding-installed")));
    }
  });
}

#[test]
fn valid_proof_closes_runtime_binding_and_opens_application_proof() {
  with_run(|run| {
    let valid = get(&run, "valid-proof");
    assert_eq!(
      as_str(get(valid, "status")),
      "self-bootstrap-status-audit-shallow-summary-fast-path-runtime-binding-proof-present"
    );
    assert!(matches!(get(valid, "held-id"), Value::Null));
    assert!(as_bool(get(valid, "runtime-binding-installed")));
    assert!(as_bool(get(valid, "positive-bound-status-call-present")));
    assert!(as_bool(get(valid, "negative-held-route-call-present")));
    assert!(!as_bool(get(valid, "optimization-applied")));
    assert!(!as_bool(get(valid, "default-callsite-replaced")));

    let closed = string_set(get(valid, "closes"));
    assert!(closed.contains(
      "need.self.bootstrap-status-audit-shallow-summary-fast-path-runtime-binding-proof"
    ));
    let open = string_set(get(valid, "next-open-frontiers"));
    assert!(
      open.contains("need.self.bootstrap-status-audit-shallow-summary-fast-path-application-proof")
    );
  });
}

#[test]
fn held_failures_cover_binding_application_runtime_external_and_authority_overclaims() {
  with_run(|run| {
    for (field, held_id) in [
      (
        "wrong-proof",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-runtime-binding.proof-id-mismatch",
      ),
      (
        "stale-stage",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-runtime-binding.stale-current-stage",
      ),
      (
        "source-mismatch",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-runtime-binding.source-mismatch",
      ),
      (
        "promotion-evidence-missing",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-runtime-binding.promotion-evidence-missing",
      ),
      (
        "binding-record-mismatch",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-runtime-binding.binding-record-mismatch",
      ),
      (
        "positive-call-missing",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-runtime-binding.positive-call-missing",
      ),
      (
        "negative-held-missing",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-runtime-binding.negative-held-missing",
      ),
      (
        "audit-or-rollback-missing",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-runtime-binding.audit-or-rollback-missing",
      ),
      (
        "missing-evidence",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-runtime-binding.missing-required-evidence",
      ),
      (
        "frontier-shape-mismatch",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-runtime-binding.frontier-shape-mismatch",
      ),
      (
        "application-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-runtime-binding.application-overclaim",
      ),
      (
        "runtime-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-runtime-binding.runtime-overclaim",
      ),
      (
        "external-or-license-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-runtime-binding.external-or-license-overclaim",
      ),
      (
        "authority-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-runtime-binding.authority-overclaim",
      ),
    ] {
      let result = get(&run, field);
      assert_eq!(as_str(get(result, "status")), "Held");
      assert_eq!(as_str(get(result, "held-id")), held_id);
      assert!(!as_bool(get(
        result,
        "self-bootstrap-status-audit-shallow-summary-fast-path-runtime-binding-proof"
      )));
    }
  });
}

#[test]
fn final_fixture_flags_install_only_bounded_runtime_binding() {
  with_run(|run| {
    assert!(as_bool(get(
      &run,
      "self-bootstrap-status-audit-shallow-summary-fast-path-runtime-binding-proof"
    )));
    assert!(as_bool(get(&run, "runtime-binding-installed")));
    assert!(as_bool(get(&run, "bounded-runtime-binding")));
    assert!(as_bool(get(&run, "positive-bound-status-call-present")));
    assert!(as_bool(get(&run, "negative-held-route-call-present")));
    assert!(as_bool(get(&run, "fast-path-promoted")));
    for key in [
      "optimization-applied",
      "default-callsite-replaced",
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
