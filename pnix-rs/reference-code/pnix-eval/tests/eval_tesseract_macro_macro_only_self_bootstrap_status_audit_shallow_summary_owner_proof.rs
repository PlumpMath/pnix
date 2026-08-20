use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/tesseract-macro-legacy-probe/macro_only_self_bootstrap_status_audit_shallow_summary_owner_proof_receipt.px",
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
    .name("bootstrap-shallow-summary-receipt-eval".to_string())
    .stack_size(32 * 1024 * 1024)
    .spawn(move || {
      let run = eval_file(&fixture_path()).expect("shallow summary receipt");
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
      "tesseract-macro-ontology-macro-only-self-bootstrap-status-audit-shallow-summary-owner-proof"
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
fn constitution_gate_blocks_summary_overclaims() {
  with_run(|run| {
    let gate = get(&run, "constitutionGate");
    assert_eq!(
      as_str(get(gate, "scenario")),
      "macro-only-self-bootstrap-status-audit-shallow-summary-owner-proof"
    );
    assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
    assert!(!as_bool(get(gate, "accepted")));
    let blocks = string_set(get(gate, "blocked-shortcuts"));
    for expected in [
      "shallow-summary-equals-replay-equivalence",
      "shallow-summary-equals-optimization-application",
      "shallow-summary-equals-fast-path-promotion",
      "shallow-summary-equals-runtime-install",
      "shallow-summary-equals-runtime-api-flattening",
      "shallow-summary-equals-meaning-db",
      "shallow-summary-equals-p-puck-semantic-owner",
      "shallow-summary-equals-full-audit-deletion",
    ] {
      assert!(blocks.contains(expected), "missing block `{expected}`");
    }
  });
}

#[test]
fn contract_records_owner_shape_without_replay_equivalence() {
  with_run(|run| {
    let contract = get(&run, "shallow-summary-contract");
    assert_eq!(
      as_str(get(contract, "id")),
      "contract.macro-only-self-bootstrap-status-audit-shallow-summary-owner-proof.v1"
    );
    assert!(as_bool(get(
      contract,
      "closes-shallow-summary-owner-frontier"
    )));
    assert!(as_bool(get(contract, "opens-replay-equivalence-proof")));
    assert_eq!(
      as_str(get(contract, "shallow-summary-owner-id")),
      "owner.summary.bootstrap-status-audit.shallow.v1"
    );
    assert_eq!(as_i64(get(contract, "summary-field-count")), 11);
    assert!(as_bool(get(contract, "full-audit-replay-preserved")));
    assert!(!as_bool(get(contract, "imports-full-audit-json")));
    assert!(!as_bool(get(contract, "replay-equivalence-proven")));
    assert!(!as_bool(get(contract, "applies-optimization")));
    assert!(!as_bool(get(contract, "promotes-fast-path")));
    assert!(!as_bool(get(contract, "closes-runtime-api-flattening")));
    assert!(!as_bool(get(contract, "closes-meaning-db")));
  });
}

#[test]
fn trials_cover_summary_owner_and_held_modes() {
  with_run(|run| {
    let trials = attrs_by_id(get(&run, "shallow-summary-trials"));
    assert_eq!(
      as_str(get(
        trials["trial.A.valid-shallow-summary-owner-proof"],
        "outcome"
      )),
      "self-bootstrap-status-audit-shallow-summary-owner-proof-present"
    );
    assert_eq!(
      as_i64(get(trials["trial.C.summary-field-count"], "field-count")),
      11
    );
    assert_eq!(
      as_str(get(trials["trial.E.next-frontier"], "outcome")),
      "need.self.bootstrap-status-audit-shallow-summary-replay-equivalence-proof"
    );
    for (id, held_id) in [
      (
        "trial.F.wrong-proof-id",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-owner.proof-id-mismatch",
      ),
      (
        "trial.G.stale-stage",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-owner.stale-current-stage",
      ),
      (
        "trial.H.source-mismatch",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-owner.source-mismatch",
      ),
      (
        "trial.I.candidate-evidence-missing",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-owner.candidate-evidence-missing",
      ),
      (
        "trial.J.summary-shape-mismatch",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-owner.summary-shape-mismatch",
      ),
      (
        "trial.K.summary-boundary-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-owner.summary-boundary-overclaim",
      ),
      (
        "trial.N.optimization-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-owner.optimization-overclaim",
      ),
      (
        "trial.O.external-or-license-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-owner.external-or-license-overclaim",
      ),
      (
        "trial.P.runtime-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-owner.runtime-overclaim",
      ),
      (
        "trial.Q.authority-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-owner.authority-overclaim",
      ),
    ] {
      assert_eq!(as_str(get(trials[id], "outcome")), "Held");
      assert_eq!(as_str(get(trials[id], "held-id")), held_id);
    }
  });
}

#[test]
fn six_layer_fold_preserves_full_audit_path_and_defers_fast_path() {
  with_run(|run| {
    let fold = get(&run, "six-layer-shallow-summary-fold");
    assert_eq!(
      as_str(get(fold, "mode")),
      "macro-only-self-bootstrap-status-audit-shallow-summary-owner-proof"
    );
    let semantic = get(fold, "semantic");
    assert!(as_bool(get(semantic, "shallow-summary-owner-ready")));
    assert!(as_bool(get(semantic, "full-audit-replay-preserved")));
    assert!(!as_bool(get(semantic, "imports-full-audit-json")));
    assert!(!as_bool(get(semantic, "replay-equivalence-proven")));
    assert!(!as_bool(get(semantic, "optimization-applied")));

    let runtime = get(fold, "runtime");
    assert!(as_bool(get(runtime, "shallow-summary-owner-ready")));
    assert!(!as_bool(get(runtime, "fast-path-promoted")));
    assert!(!as_bool(get(runtime, "runtime-api-flattening")));
    assert!(!as_bool(get(runtime, "meaning-db")));
  });
}

#[test]
fn migration_delta_closes_only_shallow_summary_owner_frontier() {
  with_run(|run| {
    let delta = get(&run, "migrationDelta");
    let closes = string_set(get(delta, "closes"));
    assert!(closes.contains("need.self.bootstrap-status-audit-shallow-summary-owner-proof"));
    let opens = string_set(get(delta, "opens"));
    assert!(
      opens.contains("need.self.bootstrap-status-audit-shallow-summary-replay-equivalence-proof")
    );
    let not_closed = string_set(get(delta, "does-not-close"));
    assert!(not_closed.contains("need.global-runtime-install.proof-after-semantic-owner"));
    assert!(not_closed.contains("need.stdlib.meaning-db"));
  });
}

#[test]
fn discoveries_record_d667_through_d674() {
  with_run(|run| {
    let discoveries = attrs_by_id(get(&run, "discoveries"));
    assert_eq!(discoveries.len(), 8);
    for expected in [
      "D667.shallow-summary-owner-separates-status-read-from-full-audit",
      "D668.shallow-summary-field-set-is-bounded",
      "D669.full-audit-replay-path-is-preserved",
      "D670.summary-owner-proof-is-not-fast-path-promotion",
      "D671.replay-equivalence-proof-is-next-frontier",
      "D672.p-puck-telemetry-remains-measurement-not-owner",
      "D673.external-solver-and-gpl-intake-remain-unjustified",
      "D674.runtime-flattening-meaning-db-and-global-runtime-remain-open",
    ] {
      let d = discoveries
        .get(expected)
        .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
      assert!(as_bool(get(d, "scenario-only")));
    }
  });
}

#[test]
fn final_receipt_flags_keep_summary_owner_proof_only() {
  with_run(|run| {
    assert!(as_bool(get(
      &run,
      "self-bootstrap-status-audit-shallow-summary-owner-proof"
    )));
    assert!(as_bool(get(&run, "shallow-summary-owner-ready")));
    assert_eq!(
      as_str(get(&run, "shallow-summary-owner-id")),
      "owner.summary.bootstrap-status-audit.shallow.v1"
    );
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
