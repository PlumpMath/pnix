use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/tesseract-macro-legacy-probe/macro_only_self_bootstrap_status_audit_shallow_summary_fast_path_runtime_binding_proof_receipt.px",
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
    .name("bootstrap-shallow-summary-fast-path-runtime-binding-receipt-eval".to_string())
    .stack_size(32 * 1024 * 1024)
    .spawn(move || {
      let run =
        eval_file(&fixture_path()).expect("shallow summary fast-path runtime binding receipt");
      f(run);
    })
    .expect("spawn eval thread")
    .join()
    .expect("eval thread panicked");
}

#[test]
fn marker_and_owner_surfaces_are_pinned() {
  with_run(|run| {
    assert_eq!(
      as_str(get(&run, "probe-marker")),
      "tesseract-macro-ontology-macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-runtime-binding-proof"
    );
    assert_eq!(
      as_str(get(&run, "constitution-owner")),
      "stdlib/lib/gate/tesseract-constitution.px"
    );
    assert_eq!(
      as_str(get(&run, "truth-owner")),
      "project-wiki/maps/tesseract-macro-ontology-discovery-ledger.md"
    );
  });
}

#[test]
fn constitution_gate_blocks_runtime_binding_overclaims() {
  with_run(|run| {
    let gate = get(&run, "constitutionGate");
    assert_eq!(
      as_str(get(gate, "scenario")),
      "macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-runtime-binding-proof"
    );
    assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
    assert!(!as_bool(get(gate, "accepted")));
    let blocks = string_set(get(gate, "blocked-shortcuts"));
    for expected in [
      "runtime-binding-equals-default-callsite-replacement",
      "runtime-binding-equals-optimization-application",
      "runtime-binding-equals-global-runtime",
      "runtime-binding-equals-runtime-api-flattening",
      "runtime-binding-equals-meaning-db",
      "runtime-binding-equals-external-solver-intake",
      "runtime-binding-equals-p-puck-semantic-owner",
      "runtime-binding-without-negative-held-rerun",
      "runtime-binding-without-full-audit-fallback",
    ] {
      assert!(blocks.contains(expected), "missing block `{expected}`");
    }
  });
}

#[test]
fn contract_binds_route_without_replacing_default_callsite() {
  with_run(|run| {
    let contract = get(&run, "binding-contract");
    assert_eq!(
      as_str(get(contract, "id")),
      "contract.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-runtime-binding-proof.v1"
    );
    assert!(as_bool(get(contract, "closes-runtime-binding-frontier")));
    assert!(as_bool(get(contract, "opens-application-proof")));
    assert_eq!(
      as_str(get(contract, "binding-id")),
      "binding.runtime.bootstrap-status-audit.shallow-summary.fast-path.v1"
    );
    assert_eq!(
      as_str(get(contract, "binding-scope")),
      "bootstrap-status-audit-shallow-summary-only"
    );
    assert!(as_bool(get(contract, "runtime-binding-installed")));
    assert!(as_bool(get(contract, "bounded-runtime-binding")));
    assert!(as_bool(get(contract, "positive-bound-status-call-present")));
    assert!(as_bool(get(contract, "negative-held-route-call-present")));
    assert!(!as_bool(get(contract, "applies-optimization")));
    assert!(!as_bool(get(contract, "default-callsite-replaced")));
  });
}

#[test]
fn trials_cover_valid_route_calls_and_held_modes() {
  with_run(|run| {
    let trials = attrs_by_id(get(&run, "binding-trials"));
    assert_eq!(
      as_str(get(
        trials["trial.A.valid-runtime-binding-proof"],
        "outcome"
      )),
      "self-bootstrap-status-audit-shallow-summary-fast-path-runtime-binding-proof-present"
    );
    assert_eq!(
      as_str(get(trials["trial.B.positive-bound-status-call"], "outcome")),
      "runtime-bound-shallow-summary-read"
    );
    assert_eq!(
      as_str(get(trials["trial.G.next-frontier"], "outcome")),
      "need.self.bootstrap-status-audit-shallow-summary-fast-path-application-proof"
    );
    for (id, held_id) in [
      (
        "trial.C.negative-held-route-call",
        "held.bootstrap-status-shallow-summary-runtime-binding.route-id-mismatch",
      ),
      (
        "trial.D.scope-held-call",
        "held.bootstrap-status-shallow-summary-runtime-binding.effect-scope-mismatch",
      ),
      (
        "trial.E.field-held-call",
        "held.bootstrap-status-shallow-summary-runtime-binding.field-shape-mismatch",
      ),
      (
        "trial.F.binding-record-held-call",
        "held.bootstrap-status-shallow-summary-runtime-binding.binding-record-mismatch",
      ),
      (
        "trial.K.promotion-evidence-missing",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-runtime-binding.promotion-evidence-missing",
      ),
      (
        "trial.L.binding-record-mismatch",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-runtime-binding.binding-record-mismatch",
      ),
      (
        "trial.M.positive-call-missing",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-runtime-binding.positive-call-missing",
      ),
      (
        "trial.N.negative-held-missing",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-runtime-binding.negative-held-missing",
      ),
      (
        "trial.O.audit-or-rollback-missing",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-runtime-binding.audit-or-rollback-missing",
      ),
      (
        "trial.P.application-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-runtime-binding.application-overclaim",
      ),
      (
        "trial.Q.runtime-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-runtime-binding.runtime-overclaim",
      ),
      (
        "trial.R.external-or-license-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-runtime-binding.external-or-license-overclaim",
      ),
      (
        "trial.S.authority-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-runtime-binding.authority-overclaim",
      ),
    ] {
      assert_eq!(as_str(get(trials[id], "outcome")), "Held");
      assert_eq!(as_str(get(trials[id], "held-id")), held_id);
    }
  });
}

#[test]
fn six_layer_fold_keeps_application_and_global_runtime_deferred() {
  with_run(|run| {
    let fold = get(&run, "six-layer-binding-fold");
    assert_eq!(
      as_str(get(fold, "mode")),
      "macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-runtime-binding-proof"
    );
    let runtime = get(fold, "runtime");
    assert!(as_bool(get(runtime, "runtime-binding-installed")));
    assert!(as_bool(get(runtime, "bounded-runtime-binding")));
    let status = get(runtime, "bound-status-record");
    assert!(as_bool(get(status, "fast-path-promoted")));
    assert!(!as_bool(get(runtime, "optimization-applied")));
    assert!(!as_bool(get(runtime, "default-callsite-replaced")));
    assert!(!as_bool(get(runtime, "runtime-api-flattening")));
    assert!(!as_bool(get(runtime, "meaning-db")));
  });
}

#[test]
fn migration_delta_closes_only_runtime_binding_frontier() {
  with_run(|run| {
    let delta = get(&run, "migrationDelta");
    let closes = string_set(get(delta, "closes"));
    assert!(closes.contains(
      "need.self.bootstrap-status-audit-shallow-summary-fast-path-runtime-binding-proof"
    ));
    let opens = string_set(get(delta, "opens"));
    assert!(opens
      .contains("need.self.bootstrap-status-audit-shallow-summary-fast-path-application-proof"));
    let not_closed = string_set(get(delta, "does-not-close"));
    assert!(not_closed.contains("need.global-runtime-install.proof-after-semantic-owner"));
    assert!(not_closed.contains("need.stdlib.meaning-db"));
  });
}

#[test]
fn discoveries_record_d691_through_d698() {
  with_run(|run| {
    let discoveries = attrs_by_id(get(&run, "discoveries"));
    assert_eq!(discoveries.len(), 8);
    for expected in [
      "D691.runtime-binding-is-bounded-route-table-not-global-runtime",
      "D692.binding-consumes-fast-path-promotion-record",
      "D693.bound-status-read-is-promotion-aware-eleven-field-summary",
      "D694.negative-held-route-rerun-survives-runtime-binding",
      "D695.runtime-binding-does-not-apply-optimization",
      "D696.full-audit-fallback-remains-after-runtime-binding",
      "D697.external-authority-and-license-boundaries-remain-closed",
      "D698.next-frontier-is-fast-path-application-proof",
    ] {
      let d = discoveries
        .get(expected)
        .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
      assert!(as_bool(get(d, "scenario-only")));
    }
  });
}

#[test]
fn final_receipt_flags_install_only_runtime_binding() {
  with_run(|run| {
    assert!(as_bool(get(
      &run,
      "self-bootstrap-status-audit-shallow-summary-fast-path-runtime-binding-proof"
    )));
    assert!(as_bool(get(&run, "runtime-binding-installed")));
    assert!(as_bool(get(&run, "bounded-runtime-binding")));
    assert!(as_bool(get(&run, "positive-bound-status-call-present")));
    assert!(as_bool(get(&run, "negative-held-route-call-present")));
    assert_eq!(as_i64(get(&run, "status-field-count")), 11);
    assert!(as_bool(get(&run, "fast-path-promoted")));
    assert!(as_bool(get(&run, "optimization-selected")));
    assert!(as_bool(get(&run, "optimization-implementation-selected")));
    for key in [
      "optimization-applied",
      "default-callsite-replaced",
      "external-solver-installed",
      "runtime-install",
      "global-ontology-runtime",
      "runtime-api-flattening",
      "meaning-db",
      "host-code-removal-started",
      "implementation-command",
      "llm-authority",
      "self-modification",
      "p-puck-is-semantic-owner",
      "old-host-authority",
      "gpl-family-dependencies",
    ] {
      assert!(!as_bool(get(&run, key)), "`{key}` must stay false");
    }
  });
}
