use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/tesseract-macro-legacy-probe/macro_only_self_optimization_candidate_after_bottleneck_attribution_receipt.px",
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

fn attrs_by_id(v: &Value) -> BTreeMap<&str, &Value> {
  as_list(v)
    .iter()
    .map(|item| (as_str(get(item, "id")), item))
    .collect()
}

fn with_run(f: impl FnOnce(Value) + Send + 'static) {
  std::thread::Builder::new()
    .name("optimization-candidate-receipt-eval".to_string())
    .stack_size(32 * 1024 * 1024)
    .spawn(move || {
      let run = eval_file(&fixture_path()).expect("optimization candidate receipt");
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
      "tesseract-macro-ontology-macro-only-self-optimization-candidate-after-bottleneck-attribution"
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
fn constitution_gate_blocks_candidate_selection_overclaims() {
  with_run(|run| {
    let gate = get(&run, "constitutionGate");
    assert_eq!(
      as_str(get(gate, "scenario")),
      "macro-only-self-optimization-candidate-after-bottleneck-attribution"
    );
    assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
    assert!(!as_bool(get(gate, "accepted")));
    let blocks = string_set(get(gate, "blocked-shortcuts"));
    for expected in [
      "candidate-selection-equals-optimization-application",
      "candidate-selection-equals-fast-path-promotion",
      "candidate-selection-equals-runtime-install",
      "candidate-selection-equals-runtime-api-flattening",
      "candidate-selection-equals-meaning-db",
      "candidate-selection-equals-external-solver-intake",
      "candidate-selection-equals-llm-authority",
      "candidate-selection-equals-self-modification",
    ] {
      assert!(blocks.contains(expected), "missing block `{expected}`");
    }
  });
}

#[test]
fn contract_records_selected_candidate_without_application() {
  with_run(|run| {
    let contract = get(&run, "candidate-contract");
    assert_eq!(
      as_str(get(contract, "id")),
      "contract.macro-only-self-optimization-candidate-after-bottleneck-attribution.v1"
    );
    assert!(as_bool(get(
      contract,
      "closes-optimization-candidate-frontier"
    )));
    assert_eq!(
      as_str(get(contract, "selected-candidate-id")),
      "candidate.optimization.bootstrap-status-audit.shallow-summary-owner.v1"
    );
    assert_eq!(
      as_str(get(contract, "selected-candidate-kind")),
      "shallow-bootstrap-status-summary-owner"
    );
    assert!(as_bool(get(contract, "candidate-only")));
    assert!(as_bool(get(contract, "opens-shallow-summary-owner-proof")));
    assert!(!as_bool(get(contract, "applies-optimization")));
    assert!(!as_bool(get(contract, "selects-implementation")));
    assert!(!as_bool(get(contract, "promotes-fast-path")));
    assert!(!as_bool(get(contract, "closes-runtime-api-flattening")));
    assert!(!as_bool(get(contract, "closes-meaning-db")));
  });
}

#[test]
fn trials_cover_candidate_selection_and_held_modes() {
  with_run(|run| {
    let trials = attrs_by_id(get(&run, "candidate-trials"));
    assert_eq!(
      as_str(get(trials["trial.A.valid-candidate-selection"], "outcome")),
      "self-optimization-candidate-after-bottleneck-attribution-proof-present"
    );
    assert_eq!(
      as_str(get(trials["trial.C.full-json-target"], "outcome")),
      "full-bootstrap-status-audit-json-test-path"
    );
    assert_eq!(
      as_str(get(trials["trial.E.next-frontier"], "outcome")),
      "need.self.bootstrap-status-audit-shallow-summary-owner-proof"
    );
    for (id, held_id) in [
      (
        "trial.F.wrong-proof-id",
        "held.macro-only-self-optimization-candidate-after-bottleneck-attribution.proof-id-mismatch",
      ),
      (
        "trial.G.stale-stage",
        "held.macro-only-self-optimization-candidate-after-bottleneck-attribution.stale-current-stage",
      ),
      (
        "trial.H.source-mismatch",
        "held.macro-only-self-optimization-candidate-after-bottleneck-attribution.source-mismatch",
      ),
      (
        "trial.I.profile-evidence-missing",
        "held.macro-only-self-optimization-candidate-after-bottleneck-attribution.profile-evidence-missing",
      ),
      (
        "trial.J.candidate-shape-mismatch",
        "held.macro-only-self-optimization-candidate-after-bottleneck-attribution.candidate-shape-mismatch",
      ),
      (
        "trial.K.candidate-boundary-overclaim",
        "held.macro-only-self-optimization-candidate-after-bottleneck-attribution.candidate-boundary-overclaim",
      ),
      (
        "trial.N.optimization-overclaim",
        "held.macro-only-self-optimization-candidate-after-bottleneck-attribution.optimization-overclaim",
      ),
      (
        "trial.O.external-or-license-overclaim",
        "held.macro-only-self-optimization-candidate-after-bottleneck-attribution.external-or-license-overclaim",
      ),
      (
        "trial.P.runtime-overclaim",
        "held.macro-only-self-optimization-candidate-after-bottleneck-attribution.runtime-overclaim",
      ),
      (
        "trial.Q.authority-overclaim",
        "held.macro-only-self-optimization-candidate-after-bottleneck-attribution.authority-overclaim",
      ),
    ] {
      assert_eq!(as_str(get(trials[id], "outcome")), "Held");
      assert_eq!(as_str(get(trials[id], "held-id")), held_id);
    }
  });
}

#[test]
fn six_layer_fold_keeps_candidate_only_runtime_boundary() {
  with_run(|run| {
    let fold = get(&run, "six-layer-candidate-fold");
    assert_eq!(
      as_str(get(fold, "mode")),
      "macro-only-self-optimization-candidate-after-bottleneck-attribution"
    );
    let semantic = get(fold, "semantic");
    assert_eq!(
      as_str(get(semantic, "target")),
      "full-bootstrap-status-audit-json-test-path"
    );
    assert_eq!(
      as_str(get(semantic, "kind")),
      "shallow-bootstrap-status-summary-owner"
    );
    assert!(as_bool(get(semantic, "candidate-only")));
    assert!(as_bool(get(semantic, "preserves-full-audit-replay")));
    assert!(!as_bool(get(semantic, "optimization-applied")));

    let runtime = get(fold, "runtime");
    assert!(as_bool(get(runtime, "optimization-candidate-selected")));
    assert!(as_bool(get(runtime, "optimization-candidate-only")));
    assert!(!as_bool(get(runtime, "optimization-applied")));
    assert!(!as_bool(get(runtime, "runtime-api-flattening")));
    assert!(!as_bool(get(runtime, "meaning-db")));
  });
}

#[test]
fn migration_delta_closes_only_candidate_frontier() {
  with_run(|run| {
    let delta = get(&run, "migrationDelta");
    let closes = string_set(get(delta, "closes"));
    assert!(closes.contains("need.self.optimization-candidate-after-bottleneck-attribution"));
    let opens = string_set(get(delta, "opens"));
    assert!(opens.contains("need.self.bootstrap-status-audit-shallow-summary-owner-proof"));
    let not_closed = string_set(get(delta, "does-not-close"));
    assert!(not_closed.contains("need.global-runtime-install.proof-after-semantic-owner"));
    assert!(not_closed.contains("need.stdlib.meaning-db"));
  });
}

#[test]
fn discoveries_record_d659_through_d666() {
  with_run(|run| {
    let discoveries = attrs_by_id(get(&run, "discoveries"));
    assert_eq!(discoveries.len(), 8);
    for expected in [
      "D659.optimization-candidate-target-is-full-bootstrap-json-test-path",
      "D660.shallow-bootstrap-status-summary-owner-is-selected-candidate",
      "D661.candidate-selection-is-not-optimization-application",
      "D662.marker-import-remains-non-target-after-repeat-proof",
      "D663.external-solver-intake-is-not-justified-by-internal-json-path",
      "D664.profile-telemetry-remains-measurement-not-owner",
      "D665.next-proof-must-validate-shallow-summary-before-fast-path",
      "D666.runtime-flattening-meaning-db-and-global-runtime-remain-open",
    ] {
      let d = discoveries
        .get(expected)
        .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
      assert!(as_bool(get(d, "scenario-only")));
    }
  });
}

#[test]
fn final_receipt_flags_keep_global_surfaces_open() {
  with_run(|run| {
    assert!(as_bool(get(
      &run,
      "self-optimization-candidate-after-bottleneck-attribution-proof"
    )));
    assert!(as_bool(get(&run, "optimization-candidate-selected")));
    assert!(as_bool(get(&run, "optimization-candidate-only")));
    assert_eq!(
      as_str(get(&run, "selected-candidate-target")),
      "full-bootstrap-status-audit-json-test-path"
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
