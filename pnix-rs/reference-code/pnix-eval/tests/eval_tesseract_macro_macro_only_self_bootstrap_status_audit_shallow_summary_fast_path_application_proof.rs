use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/tesseract-macro-legacy-probe/macro_only_self_bootstrap_status_audit_shallow_summary_fast_path_application_proof_receipt.px",
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
    .name("bootstrap-shallow-summary-fast-path-application-receipt-eval".to_string())
    .stack_size(32 * 1024 * 1024)
    .spawn(move || {
      let run = eval_file(&fixture_path()).expect("shallow summary fast-path application receipt");
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
      "tesseract-macro-ontology-macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application-proof"
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
fn constitution_gate_blocks_application_overclaims() {
  with_run(|run| {
    let gate = get(&run, "constitutionGate");
    assert_eq!(
      as_str(get(gate, "scenario")),
      "macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application-proof"
    );
    assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
    assert!(!as_bool(get(gate, "accepted")));
    let blocks = string_set(get(gate, "blocked-shortcuts"));
    for expected in [
      "selected-callsite-application-equals-global-runtime",
      "selected-callsite-application-equals-runtime-api-flattening",
      "selected-callsite-application-equals-meaning-db",
      "selected-callsite-application-equals-external-solver-intake",
      "selected-callsite-application-equals-p-puck-semantic-owner",
      "selected-callsite-application-without-negative-held-unselected-callsite",
      "selected-callsite-application-without-broken-binding-held-rerun",
      "selected-callsite-application-without-measurement-frontier",
    ] {
      assert!(blocks.contains(expected), "missing block `{expected}`");
    }
  });
}

#[test]
fn contract_applies_selected_callsite_without_global_runtime() {
  with_run(|run| {
    let contract = get(&run, "application-contract");
    assert_eq!(
      as_str(get(contract, "id")),
      "contract.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application-proof.v1"
    );
    assert!(as_bool(get(contract, "closes-application-frontier")));
    assert!(as_bool(get(contract, "opens-measurement-proof")));
    assert_eq!(
      as_str(get(contract, "selected-callsite-id")),
      "callsite.bootstrap-status-audit.current-status.shallow-summary.v1"
    );
    assert!(as_bool(get(contract, "selected-callsite-replaced")));
    assert_eq!(as_i64(get(contract, "selected-callsite-count")), 1);
    assert!(as_bool(get(contract, "scoped-optimization-applied")));
    assert!(as_bool(get(contract, "optimization-applied")));
    assert!(!as_bool(get(contract, "global-optimization-applied")));
    assert!(!as_bool(get(contract, "global-default-callsite-replaced")));
    assert!(!as_bool(get(contract, "closes-global-runtime")));
    assert!(!as_bool(get(contract, "closes-meaning-db")));
  });
}

#[test]
fn trials_cover_valid_applied_call_and_held_modes() {
  with_run(|run| {
    let trials = attrs_by_id(get(&run, "application-trials"));
    assert_eq!(
      as_str(get(trials["trial.A.valid-application-proof"], "outcome")),
      "self-bootstrap-status-audit-shallow-summary-fast-path-application-proof-present"
    );
    assert_eq!(
      as_str(get(trials["trial.B.positive-applied-callsite"], "outcome")),
      "fast-path-applied-shallow-summary-read"
    );
    assert_eq!(
      as_str(get(trials["trial.F.next-frontier"], "outcome")),
      "need.self.bootstrap-status-audit-shallow-summary-fast-path-application-measurement-proof"
    );
    for (id, held_id) in [
      (
        "trial.C.negative-unselected-callsite",
        "held.bootstrap-status-shallow-summary-fast-path-application.callsite-mismatch",
      ),
      (
        "trial.D.negative-broken-binding-callsite",
        "held.bootstrap-status-shallow-summary-fast-path-application.route-result-held",
      ),
      (
        "trial.E.negative-field-shape-callsite",
        "held.bootstrap-status-shallow-summary-fast-path-application.route-result-held",
      ),
      (
        "trial.J.binding-evidence-missing",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application.binding-evidence-missing",
      ),
      (
        "trial.K.application-record-mismatch",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application.application-record-mismatch",
      ),
      (
        "trial.L.positive-call-missing",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application.positive-callsite-missing",
      ),
      (
        "trial.M.negative-unselected-missing",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application.negative-unselected-missing",
      ),
      (
        "trial.N.negative-broken-binding-missing",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application.negative-broken-binding-missing",
      ),
      (
        "trial.O.scoped-application-missing",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application.scoped-application-missing",
      ),
      (
        "trial.P.audit-or-measurement-missing",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application.audit-or-measurement-missing",
      ),
      (
        "trial.Q.global-application-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application.global-application-overclaim",
      ),
      (
        "trial.R.runtime-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application.runtime-overclaim",
      ),
      (
        "trial.S.external-or-license-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application.external-or-license-overclaim",
      ),
      (
        "trial.T.authority-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application.authority-overclaim",
      ),
    ] {
      assert_eq!(as_str(get(trials[id], "outcome")), "Held");
      assert_eq!(as_str(get(trials[id], "held-id")), held_id);
    }
  });
}

#[test]
fn six_layer_fold_keeps_global_runtime_and_meaning_db_deferred() {
  with_run(|run| {
    let fold = get(&run, "six-layer-application-fold");
    assert_eq!(
      as_str(get(fold, "mode")),
      "macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-application-proof"
    );
    let runtime = get(fold, "runtime");
    assert!(as_bool(get(runtime, "selected-callsite-replaced")));
    assert!(as_bool(get(runtime, "scoped-optimization-applied")));
    assert!(as_bool(get(runtime, "optimization-applied")));
    assert!(as_bool(get(runtime, "runtime-binding-installed")));
    assert!(!as_bool(get(runtime, "global-optimization-applied")));
    assert!(!as_bool(get(runtime, "global-default-callsite-replaced")));
    assert!(!as_bool(get(runtime, "runtime-api-flattening")));
    assert!(!as_bool(get(runtime, "meaning-db")));
  });
}

#[test]
fn migration_delta_closes_only_application_frontier() {
  with_run(|run| {
    let delta = get(&run, "migrationDelta");
    let closes = string_set(get(delta, "closes"));
    assert!(closes
      .contains("need.self.bootstrap-status-audit-shallow-summary-fast-path-application-proof"));
    let opens = string_set(get(delta, "opens"));
    assert!(opens.contains(
      "need.self.bootstrap-status-audit-shallow-summary-fast-path-application-measurement-proof"
    ));
    let not_closed = string_set(get(delta, "does-not-close"));
    assert!(not_closed.contains("need.global-runtime-install.proof-after-semantic-owner"));
    assert!(not_closed.contains("need.stdlib.meaning-db"));
    assert!(not_closed.contains("need.global-default-callsite-replacement"));
  });
}

#[test]
fn discoveries_record_d699_through_d706() {
  with_run(|run| {
    let discoveries = attrs_by_id(get(&run, "discoveries"));
    assert_eq!(discoveries.len(), 8);
    for expected in [
      "D699.application-is-selected-callsite-only-not-global-runtime",
      "D700.application-consumes-bounded-runtime-binding-record",
      "D701.applied-callsite-returns-promotion-aware-eleven-field-summary",
      "D702.unselected-and-broken-callsite-reruns-stay-held",
      "D703.full-audit-fallback-and-rollback-remain-after-application",
      "D704.optimization-application-is-scoped-not-global",
      "D705.external-authority-and-license-boundaries-remain-closed",
      "D706.next-frontier-is-application-measurement-proof",
    ] {
      let d = discoveries
        .get(expected)
        .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
      assert!(as_bool(get(d, "scenario-only")));
    }
  });
}

#[test]
fn final_receipt_flags_apply_only_selected_callsite() {
  with_run(|run| {
    assert!(as_bool(get(
      &run,
      "self-bootstrap-status-audit-shallow-summary-fast-path-application-proof"
    )));
    assert!(as_bool(get(&run, "selected-callsite-replaced")));
    assert_eq!(as_i64(get(&run, "selected-callsite-count")), 1);
    assert!(as_bool(get(&run, "scoped-optimization-applied")));
    assert!(as_bool(get(&run, "optimization-applied")));
    assert!(as_bool(get(&run, "runtime-binding-installed")));
    assert!(as_bool(get(&run, "bounded-runtime-binding")));
    assert!(as_bool(get(&run, "measurement-required")));
    assert_eq!(as_i64(get(&run, "status-field-count")), 11);
    for key in [
      "global-optimization-applied",
      "global-default-callsite-replaced",
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
