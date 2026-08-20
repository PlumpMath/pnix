use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/tesseract-macro-legacy-probe/macro_only_self_bootstrap_status_audit_shallow_summary_fast_path_promotion_proof_receipt.px",
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
    .name("bootstrap-shallow-summary-fast-path-promotion-receipt-eval".to_string())
    .stack_size(32 * 1024 * 1024)
    .spawn(move || {
      let run = eval_file(&fixture_path()).expect("shallow summary fast-path promotion receipt");
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
      "tesseract-macro-ontology-macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-promotion-proof"
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
fn constitution_gate_blocks_promotion_overclaims() {
  with_run(|run| {
    let gate = get(&run, "constitutionGate");
    assert_eq!(
      as_str(get(gate, "scenario")),
      "macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-promotion-proof"
    );
    assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
    assert!(!as_bool(get(gate, "accepted")));
    let blocks = string_set(get(gate, "blocked-shortcuts"));
    for expected in [
      "fast-path-promotion-equals-runtime-binding",
      "fast-path-promotion-equals-optimization-application",
      "fast-path-promotion-equals-runtime-install",
      "fast-path-promotion-equals-global-runtime",
      "fast-path-promotion-equals-runtime-api-flattening",
      "fast-path-promotion-equals-meaning-db",
      "fast-path-promotion-equals-whole-json-equivalence",
      "fast-path-promotion-equals-p-puck-semantic-owner",
      "fast-path-promotion-equals-external-solver-intake",
    ] {
      assert!(blocks.contains(expected), "missing block `{expected}`");
    }
  });
}

#[test]
fn contract_promotes_route_candidate_without_binding_runtime() {
  with_run(|run| {
    let contract = get(&run, "promotion-contract");
    assert_eq!(
      as_str(get(contract, "id")),
      "contract.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-promotion-proof.v1"
    );
    assert!(as_bool(get(
      contract,
      "closes-fast-path-promotion-frontier"
    )));
    assert!(as_bool(get(contract, "opens-runtime-binding-proof")));
    assert_eq!(
      as_str(get(contract, "promotion-id")),
      "promotion.fast-path.bootstrap-status-audit.shallow-summary.v1"
    );
    assert_eq!(
      as_str(get(contract, "route-id")),
      "route.fast-path.bootstrap-status-audit.shallow-summary.v1"
    );
    assert_eq!(
      as_str(get(contract, "route-owner")),
      "owner.summary.bootstrap-status-audit.shallow.v1"
    );
    assert_eq!(as_i64(get(contract, "projection-field-count")), 11);
    assert!(as_bool(get(contract, "bounded-field-replay-equivalence")));
    assert!(as_bool(get(contract, "full-audit-fallback-preserved")));
    assert!(!as_bool(get(contract, "whole-json-equivalence-proven")));
    assert!(as_bool(get(contract, "fast-path-promoted")));
    assert!(as_bool(get(contract, "optimization-selected")));
    assert!(as_bool(get(
      contract,
      "optimization-implementation-selected"
    )));
    assert!(!as_bool(get(contract, "applies-optimization")));
    assert!(!as_bool(get(contract, "runtime-binding-installed")));
  });
}

#[test]
fn trials_cover_valid_path_source_and_held_modes() {
  with_run(|run| {
    let trials = attrs_by_id(get(&run, "promotion-trials"));
    assert_eq!(
      as_str(get(
        trials["trial.A.valid-fast-path-promotion-proof"],
        "outcome"
      )),
      "self-bootstrap-status-audit-shallow-summary-fast-path-promotion-proof-present"
    );
    assert_eq!(
      as_str(get(trials["trial.C.promotion-record"], "outcome")),
      "promotion.fast-path.bootstrap-status-audit.shallow-summary.v1"
    );
    assert_eq!(
      as_str(get(trials["trial.D.runtime-binding-deferred"], "outcome")),
      "runtime-binding-installed=false"
    );
    assert_eq!(
      as_str(get(trials["trial.E.next-frontier"], "outcome")),
      "need.self.bootstrap-status-audit-shallow-summary-fast-path-runtime-binding-proof"
    );
    for (id, held_id) in [
      (
        "trial.F.wrong-proof-id",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-promotion.proof-id-mismatch",
      ),
      (
        "trial.G.stale-stage",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-promotion.stale-current-stage",
      ),
      (
        "trial.H.source-mismatch",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-promotion.source-mismatch",
      ),
      (
        "trial.I.replay-evidence-missing",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-promotion.replay-evidence-missing",
      ),
      (
        "trial.J.promotion-record-mismatch",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-promotion.promotion-record-mismatch",
      ),
      (
        "trial.K.missing-evidence",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-promotion.missing-required-evidence",
      ),
      (
        "trial.L.frontier-shape-mismatch",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-promotion.frontier-shape-mismatch",
      ),
      (
        "trial.M.whole-json-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-promotion.whole-json-overclaim",
      ),
      (
        "trial.N.runtime-binding-or-application-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-promotion.runtime-binding-or-application-overclaim",
      ),
      (
        "trial.O.external-or-license-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-promotion.external-or-license-overclaim",
      ),
      (
        "trial.P.runtime-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-promotion.runtime-overclaim",
      ),
      (
        "trial.Q.authority-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-promotion.authority-overclaim",
      ),
    ] {
      assert_eq!(as_str(get(trials[id], "outcome")), "Held");
      assert_eq!(as_str(get(trials[id], "held-id")), held_id);
    }
  });
}

#[test]
fn six_layer_fold_keeps_runtime_application_deferred() {
  with_run(|run| {
    let fold = get(&run, "six-layer-promotion-fold");
    assert_eq!(
      as_str(get(fold, "mode")),
      "macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-promotion-proof"
    );
    let semantic = get(fold, "semantic");
    assert!(as_bool(get(semantic, "bounded-field-replay-equivalence")));
    assert!(as_bool(get(semantic, "fast-path-promoted")));
    assert!(as_bool(get(semantic, "optimization-selected")));
    assert!(as_bool(get(
      semantic,
      "optimization-implementation-selected"
    )));
    assert!(!as_bool(get(semantic, "whole-json-equivalence-proven")));
    assert!(as_bool(get(semantic, "full-audit-fallback-preserved")));

    let runtime = get(fold, "runtime");
    assert!(as_bool(get(runtime, "fast-path-promoted")));
    assert!(as_bool(get(runtime, "optimization-selected")));
    assert!(as_bool(get(
      runtime,
      "optimization-implementation-selected"
    )));
    assert!(!as_bool(get(runtime, "optimization-applied")));
    assert!(!as_bool(get(runtime, "runtime-binding-installed")));
    assert!(!as_bool(get(runtime, "runtime-api-flattening")));
    assert!(!as_bool(get(runtime, "meaning-db")));
  });
}

#[test]
fn migration_delta_closes_only_fast_path_promotion_frontier() {
  with_run(|run| {
    let delta = get(&run, "migrationDelta");
    let closes = string_set(get(delta, "closes"));
    assert!(
      closes.contains("need.self.bootstrap-status-audit-shallow-summary-fast-path-promotion-proof")
    );
    let opens = string_set(get(delta, "opens"));
    assert!(opens.contains(
      "need.self.bootstrap-status-audit-shallow-summary-fast-path-runtime-binding-proof"
    ));
    let not_closed = string_set(get(delta, "does-not-close"));
    assert!(not_closed.contains("need.global-runtime-install.proof-after-semantic-owner"));
    assert!(not_closed.contains("need.stdlib.meaning-db"));
  });
}

#[test]
fn discoveries_record_d683_through_d690() {
  with_run(|run| {
    let discoveries = attrs_by_id(get(&run, "discoveries"));
    assert_eq!(discoveries.len(), 8);
    for expected in [
      "D683.fast-path-promotion-is-route-candidate-not-runtime-install",
      "D684.promotion-consumes-bounded-replay-equivalence",
      "D685.full-audit-fallback-survives-fast-path-promotion",
      "D686.optimization-selected-is-not-optimization-applied",
      "D687.whole-json-equivalence-remains-false-after-promotion",
      "D688.external-solver-and-gpl-boundaries-remain-closed-after-promotion",
      "D689.p-puck-and-llm-remain-measurement-and-input-not-authority",
      "D690.next-frontier-is-runtime-binding-proof-not-global-runtime",
    ] {
      let d = discoveries
        .get(expected)
        .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
      assert!(as_bool(get(d, "scenario-only")));
    }
  });
}

#[test]
fn final_receipt_flags_promote_fast_path_but_do_not_install_runtime() {
  with_run(|run| {
    assert!(as_bool(get(
      &run,
      "self-bootstrap-status-audit-shallow-summary-fast-path-promotion-proof"
    )));
    assert!(as_bool(get(&run, "bounded-field-replay-equivalence")));
    assert!(as_bool(get(&run, "replay-equivalence-proven")));
    assert!(!as_bool(get(&run, "whole-json-equivalence-proven")));
    assert!(as_bool(get(&run, "full-audit-fallback-preserved")));
    assert_eq!(as_i64(get(&run, "projection-field-count")), 11);
    assert_eq!(as_i64(get(&run, "direct-full-audit-field-count")), 8);
    assert_eq!(as_i64(get(&run, "derived-boundary-field-count")), 3);
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
