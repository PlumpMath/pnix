use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/tesseract-macro-legacy-probe/macro_only_self_bootstrap_status_audit_shallow_summary_replay_equivalence_proof_receipt.px",
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

fn records_by_field(v: &Value) -> BTreeMap<&str, &Value> {
  as_list(v)
    .iter()
    .map(|item| (as_str(get(item, "field")), item))
    .collect()
}

fn with_run(f: impl FnOnce(Value) + Send + 'static) {
  std::thread::Builder::new()
    .name("bootstrap-shallow-summary-replay-equivalence-receipt-eval".to_string())
    .stack_size(32 * 1024 * 1024)
    .spawn(move || {
      let run = eval_file(&fixture_path()).expect("shallow summary replay equivalence receipt");
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
      "tesseract-macro-ontology-macro-only-self-bootstrap-status-audit-shallow-summary-replay-equivalence-proof"
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
fn constitution_gate_blocks_replay_equivalence_overclaims() {
  with_run(|run| {
    let gate = get(&run, "constitutionGate");
    assert_eq!(
      as_str(get(gate, "scenario")),
      "macro-only-self-bootstrap-status-audit-shallow-summary-replay-equivalence-proof"
    );
    assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
    assert!(!as_bool(get(gate, "accepted")));
    let blocks = string_set(get(gate, "blocked-shortcuts"));
    for expected in [
      "bounded-field-equivalence-equals-whole-json-equivalence",
      "bounded-field-equivalence-imports-full-audit-json-into-summary",
      "bounded-field-equivalence-equals-optimization-application",
      "bounded-field-equivalence-equals-fast-path-promotion",
      "bounded-field-equivalence-equals-runtime-api-flattening",
      "bounded-field-equivalence-equals-meaning-db",
      "bounded-field-equivalence-equals-p-puck-semantic-owner",
      "bounded-field-equivalence-equals-external-solver-intake",
    ] {
      assert!(blocks.contains(expected), "missing block `{expected}`");
    }
  });
}

#[test]
fn contract_records_bounded_replay_equivalence_not_whole_json_equivalence() {
  with_run(|run| {
    let contract = get(&run, "replay-equivalence-contract");
    assert_eq!(
      as_str(get(contract, "id")),
      "contract.macro-only-self-bootstrap-status-audit-shallow-summary-replay-equivalence-proof.v1"
    );
    assert!(as_bool(get(contract, "closes-replay-equivalence-frontier")));
    assert!(as_bool(get(contract, "opens-fast-path-promotion-proof")));
    assert_eq!(as_i64(get(contract, "projection-field-count")), 11);
    assert_eq!(as_i64(get(contract, "direct-full-audit-field-count")), 8);
    assert_eq!(as_i64(get(contract, "derived-boundary-field-count")), 3);
    assert!(as_bool(get(contract, "bounded-field-replay-equivalence")));
    assert!(as_bool(get(contract, "replay-equivalence-proven")));
    assert!(!as_bool(get(contract, "whole-json-equivalence-proven")));
    assert!(!as_bool(get(
      contract,
      "full-audit-json-imported-by-shallow-summary"
    )));
    assert!(as_bool(get(contract, "fast-path-promotion-eligible")));
    assert!(!as_bool(get(contract, "applies-optimization")));
    assert!(!as_bool(get(contract, "promotes-fast-path")));
    assert!(!as_bool(get(contract, "closes-runtime-api-flattening")));
    assert!(!as_bool(get(contract, "closes-meaning-db")));
  });
}

#[test]
fn projection_records_preserve_field_values_and_source_classes() {
  with_run(|run| {
    let records = records_by_field(get(&run, "projection-records"));
    assert_eq!(records.len(), 11);
    for field in [
      "macro-only-runtime-owner",
      "semantic-owner",
      "boot-executed",
    ] {
      let record = records[field];
      assert!(as_bool(get(record, "shallow-value")));
      assert!(as_bool(get(record, "full-audit-value")));
      assert_eq!(
        as_str(get(record, "full-audit-source")),
        "direct-full-audit-field"
      );
      assert!(as_bool(get(record, "equivalent")));
    }
    for field in [
      "new-engine-from-zero",
      "host-code-removal-started",
      "global-ontology-runtime",
      "runtime-api-flattening",
      "meaning-db",
    ] {
      let record = records[field];
      assert!(!as_bool(get(record, "shallow-value")));
      assert!(!as_bool(get(record, "full-audit-value")));
      assert_eq!(
        as_str(get(record, "full-audit-source")),
        "direct-full-audit-field"
      );
      assert!(as_bool(get(record, "equivalent")));
    }
    for field in [
      "optimization-applied",
      "fast-path-promoted",
      "external-solver-installed",
    ] {
      let record = records[field];
      assert!(!as_bool(get(record, "shallow-value")));
      assert!(!as_bool(get(record, "full-audit-value")));
      assert_eq!(as_str(get(record, "full-audit-source")), "derived-boundary");
      assert_ne!(as_str(get(record, "boundary-derivation")), "");
      assert!(as_bool(get(record, "equivalent")));
    }
  });
}

#[test]
fn trials_cover_valid_path_source_check_and_held_modes() {
  with_run(|run| {
    let trials = attrs_by_id(get(&run, "replay-equivalence-trials"));
    assert_eq!(
      as_str(get(
        trials["trial.A.valid-bounded-replay-equivalence-proof"],
        "outcome"
      )),
      "self-bootstrap-status-audit-shallow-summary-replay-equivalence-proof-present"
    );
    assert_eq!(
      as_i64(get(
        trials["trial.A.valid-bounded-replay-equivalence-proof"],
        "projection-field-count"
      )),
      11
    );
    assert_eq!(
      as_str(get(
        trials["trial.B.full-audit-source-text-check"],
        "outcome"
      )),
      "bounded-top-level-status-field-projection"
    );
    assert!(as_bool(get(
      trials["trial.B.full-audit-source-text-check"],
      "source-text-checked-by-rust"
    )));
    assert_eq!(
      as_str(get(trials["trial.E.next-frontier"], "outcome")),
      "need.self.bootstrap-status-audit-shallow-summary-fast-path-promotion-proof"
    );
    for (id, held_id) in [
      (
        "trial.F.wrong-proof-id",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-replay-equivalence.proof-id-mismatch",
      ),
      (
        "trial.G.stale-stage",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-replay-equivalence.stale-current-stage",
      ),
      (
        "trial.H.source-mismatch",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-replay-equivalence.source-mismatch",
      ),
      (
        "trial.I.summary-owner-missing",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-replay-equivalence.summary-owner-missing",
      ),
      (
        "trial.J.projection-source-missing",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-replay-equivalence.projection-source-missing",
      ),
      (
        "trial.K.projection-shape-mismatch",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-replay-equivalence.projection-shape-mismatch",
      ),
      (
        "trial.L.projection-value-mismatch",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-replay-equivalence.projection-value-mismatch",
      ),
      (
        "trial.O.whole-json-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-replay-equivalence.whole-json-overclaim",
      ),
      (
        "trial.P.optimization-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-replay-equivalence.optimization-overclaim",
      ),
      (
        "trial.Q.external-or-license-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-replay-equivalence.external-or-license-overclaim",
      ),
      (
        "trial.R.runtime-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-replay-equivalence.runtime-overclaim",
      ),
      (
        "trial.S.authority-overclaim",
        "held.macro-only-self-bootstrap-status-audit-shallow-summary-replay-equivalence.authority-overclaim",
      ),
    ] {
      assert_eq!(as_str(get(trials[id], "outcome")), "Held");
      assert_eq!(as_str(get(trials[id], "held-id")), held_id);
    }
  });
}

#[test]
fn six_layer_fold_keeps_fast_path_promotion_deferred() {
  with_run(|run| {
    let fold = get(&run, "six-layer-replay-equivalence-fold");
    assert_eq!(
      as_str(get(fold, "mode")),
      "macro-only-self-bootstrap-status-audit-shallow-summary-replay-equivalence-proof"
    );
    let semantic = get(fold, "semantic");
    assert!(as_bool(get(semantic, "bounded-field-replay-equivalence")));
    assert!(as_bool(get(semantic, "replay-equivalence-proven")));
    assert!(!as_bool(get(semantic, "whole-json-equivalence-proven")));
    assert!(!as_bool(get(
      semantic,
      "full-audit-json-imported-by-shallow-summary"
    )));
    assert!(as_bool(get(semantic, "fast-path-promotion-eligible")));
    assert!(!as_bool(get(semantic, "optimization-applied")));
    assert!(!as_bool(get(semantic, "fast-path-promoted")));

    let runtime = get(fold, "runtime");
    assert!(as_bool(get(runtime, "bounded-field-replay-equivalence")));
    assert!(as_bool(get(runtime, "fast-path-promotion-eligible")));
    assert!(!as_bool(get(runtime, "optimization-selected")));
    assert!(!as_bool(get(runtime, "runtime-api-flattening")));
    assert!(!as_bool(get(runtime, "meaning-db")));
  });
}

#[test]
fn migration_delta_closes_only_replay_equivalence_frontier() {
  with_run(|run| {
    let delta = get(&run, "migrationDelta");
    let closes = string_set(get(delta, "closes"));
    assert!(
      closes.contains("need.self.bootstrap-status-audit-shallow-summary-replay-equivalence-proof")
    );
    let opens = string_set(get(delta, "opens"));
    assert!(
      opens.contains("need.self.bootstrap-status-audit-shallow-summary-fast-path-promotion-proof")
    );
    let not_closed = string_set(get(delta, "does-not-close"));
    assert!(not_closed.contains("need.global-runtime-install.proof-after-semantic-owner"));
    assert!(not_closed.contains("need.stdlib.meaning-db"));
  });
}

#[test]
fn discoveries_record_d675_through_d682() {
  with_run(|run| {
    let discoveries = attrs_by_id(get(&run, "discoveries"));
    assert_eq!(discoveries.len(), 8);
    for expected in [
      "D675.bounded-field-replay-equivalence-not-whole-json-equivalence",
      "D676.direct-and-derived-boundary-status-sources-are-classified",
      "D677.shallow-summary-still-does-not-import-full-audit-json",
      "D678.full-audit-source-text-check-keeps-projection-honest",
      "D679.replay-equivalence-opens-fast-path-promotion-proof-only",
      "D680.replay-equivalence-does-not-apply-optimization-or-install-runtime",
      "D681.p-puck-telemetry-remains-measurement-not-authority",
      "D682.external-solver-and-gpl-boundaries-remain-closed",
    ] {
      let d = discoveries
        .get(expected)
        .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
      assert!(as_bool(get(d, "scenario-only")));
    }
  });
}

#[test]
fn final_receipt_flags_keep_promotion_and_runtime_open() {
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
    assert_eq!(as_i64(get(&run, "projection-field-count")), 11);
    assert_eq!(as_i64(get(&run, "direct-full-audit-field-count")), 8);
    assert_eq!(as_i64(get(&run, "derived-boundary-field-count")), 3);
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
