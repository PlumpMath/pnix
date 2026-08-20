use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/tesseract-macro-legacy-probe/macro_only_self_bootstrap_status_audit_profile_split_proof_receipt.px",
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
    .name("bootstrap-profile-split-receipt-eval".to_string())
    .stack_size(32 * 1024 * 1024)
    .spawn(move || {
      let run = eval_file(&fixture_path()).expect("bootstrap profile split receipt");
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
      "tesseract-macro-ontology-macro-only-self-bootstrap-status-audit-profile-split-proof"
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
fn constitution_gate_blocks_profile_split_overclaims() {
  with_run(|run| {
    let gate = get(&run, "constitutionGate");
    assert_eq!(
      as_str(get(gate, "scenario")),
      "macro-only-self-bootstrap-status-audit-profile-split-proof"
    );
    assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
    assert!(!as_bool(get(gate, "accepted")));
    let blocks = string_set(get(gate, "blocked-shortcuts"));
    for expected in [
      "profile-split-equals-optimization-selection",
      "profile-split-equals-fast-path-promotion",
      "full-test-lower-bound-equals-runtime-install",
      "marker-repeat-equals-bootstrap-audit-clean",
      "profile-split-equals-runtime-api-flattening",
      "profile-split-equals-meaning-db",
      "profile-split-equals-p-puck-semantic-owner",
      "profile-split-equals-external-solver-intake",
    ] {
      assert!(blocks.contains(expected), "missing block `{expected}`");
    }
  });
}

#[test]
fn contract_records_profile_split_without_optimization_selection() {
  with_run(|run| {
    let contract = get(&run, "profile-split-contract");
    assert_eq!(
      as_str(get(contract, "id")),
      "contract.macro-only-self-bootstrap-status-audit-profile-split-proof.v1"
    );
    assert!(as_bool(get(
      contract,
      "closes-bootstrap-profile-split-frontier"
    )));
    assert_eq!(as_i64(get(contract, "initial-probe-duration-ms")), 11167);
    assert_eq!(as_i64(get(contract, "repeat-probe-duration-ms")), 1541);
    assert_eq!(as_i64(get(contract, "full-test-lower-bound-ms")), 60000);
    assert!(!as_bool(get(
      contract,
      "marker-import-persistent-bottleneck"
    )));
    assert!(as_bool(get(
      contract,
      "full-bootstrap-status-audit-json-test-path-bottleneck"
    )));
    assert!(as_bool(get(
      contract,
      "optimization-candidate-ready-after-profile-split"
    )));
    assert!(!as_bool(get(contract, "selects-optimization")));
    assert!(!as_bool(get(contract, "promotes-fast-path")));
    assert!(!as_bool(get(contract, "closes-runtime-api-flattening")));
  });
}

#[test]
fn profile_records_pin_marker_repeat_and_full_test_lower_bound() {
  with_run(|run| {
    let records = attrs_by_id(get(&run, "profile-records"));
    assert_eq!(records.len(), 3);
    assert_eq!(
      as_i64(get(
        records["profile.bootstrap-status-audit.probe-marker.initial-slow-path"],
        "duration-ms"
      )),
      11167
    );
    assert_eq!(
      as_i64(get(
        records["profile.bootstrap-status-audit.probe-marker.repeat-within-threshold"],
        "duration-ms"
      )),
      1541
    );
    assert_eq!(
      as_i64(get(
        records["profile.bootstrap-status-audit.full-json-test-path.long-running"],
        "duration-lower-bound-ms"
      )),
      60000
    );
    assert!(as_bool(get(
      records["profile.bootstrap-status-audit.full-json-test-path.long-running"],
      "is-bottleneck"
    )));
  });
}

#[test]
fn six_layer_fold_separates_marker_import_from_full_json_path() {
  with_run(|run| {
    let fold = get(&run, "six-layer-profile-split-fold");
    assert_eq!(
      as_str(get(fold, "mode")),
      "macro-only-self-bootstrap-status-audit-profile-split-proof"
    );
    let semantic = get(fold, "semantic");
    assert!(!as_bool(get(
      semantic,
      "marker-import-persistent-bottleneck"
    )));
    assert!(as_bool(get(
      semantic,
      "full-bootstrap-status-audit-json-test-path-bottleneck"
    )));
    assert!(as_bool(get(
      semantic,
      "optimization-candidate-ready-after-profile-split"
    )));
    assert!(!as_bool(get(semantic, "optimization-selected")));

    let runtime = get(fold, "runtime");
    assert!(as_bool(get(
      runtime,
      "bootstrap-status-audit-profile-split-proof"
    )));
    assert!(!as_bool(get(runtime, "fast-path-promoted")));
    assert!(!as_bool(get(runtime, "runtime-api-flattening")));
    assert!(!as_bool(get(runtime, "meaning-db")));
  });
}

#[test]
fn trials_cover_valid_profile_and_held_overclaims() {
  with_run(|run| {
    let trials = attrs_by_id(get(&run, "profile-split-trials"));
    assert_eq!(
      as_str(get(trials["trial.A.valid-profile-split-proof"], "outcome")),
      "self-bootstrap-status-audit-profile-split-proof-present"
    );
    assert_eq!(
      as_i64(get(
        trials["trial.E.full-bootstrap-test-lower-bound"],
        "lower-bound-ms"
      )),
      60000
    );
    for (id, held_id) in [
      (
        "trial.F.wrong-proof-id",
        "held.macro-only-self-bootstrap-status-audit-profile-split.proof-id-mismatch",
      ),
      (
        "trial.G.stale-stage",
        "held.macro-only-self-bootstrap-status-audit-profile-split.stale-current-stage",
      ),
      (
        "trial.H.source-mismatch",
        "held.macro-only-self-bootstrap-status-audit-profile-split.source-mismatch",
      ),
      (
        "trial.K.profile-record-invalid",
        "held.macro-only-self-bootstrap-status-audit-profile-split.profile-record-invalid",
      ),
      (
        "trial.N.split-overclaim",
        "held.macro-only-self-bootstrap-status-audit-profile-split.split-overclaim-or-underclaim",
      ),
      (
        "trial.O.optimization-overclaim",
        "held.macro-only-self-bootstrap-status-audit-profile-split.optimization-overclaim",
      ),
      (
        "trial.P.runtime-overclaim",
        "held.macro-only-self-bootstrap-status-audit-profile-split.runtime-overclaim",
      ),
      (
        "trial.Q.authority-overclaim",
        "held.macro-only-self-bootstrap-status-audit-profile-split.authority-overclaim",
      ),
      (
        "trial.R.gpl-family-dependency",
        "held.macro-only-self-bootstrap-status-audit-profile-split.gpl-family-dependency",
      ),
    ] {
      assert_eq!(as_str(get(trials[id], "outcome")), "Held", "{id}");
      assert_eq!(as_str(get(trials[id], "held-id")), held_id, "{id}");
    }
  });
}

#[test]
fn migration_delta_closes_profile_split_and_leaves_optimization_open() {
  with_run(|run| {
    let delta = get(&run, "migrationDelta");
    let closes = string_set(get(delta, "closes"));
    assert!(closes.contains("need.self.bootstrap-status-audit-profile-split-proof"));

    let not_closed = string_set(get(delta, "does-not-close"));
    assert!(not_closed.contains("need.self.optimization-candidate-after-bottleneck-attribution"));
    assert!(not_closed.contains("need.domain-runtime-api-flattening-after-semantic-owner"));

    assert!(as_bool(get(
      &run,
      "optimization-candidate-ready-after-profile-split"
    )));
    assert!(!as_bool(get(&run, "optimization-selected")));
    assert!(!as_bool(get(&run, "fast-path-promoted")));
  });
}

#[test]
fn discoveries_record_d651_through_d658() {
  with_run(|run| {
    let discoveries = attrs_by_id(get(&run, "discoveries"));
    assert_eq!(discoveries.len(), 8);
    for expected in [
      "D651.bootstrap-status-audit-profile-split-separates-marker-from-full-json-path",
      "D652.bootstrap-probe-marker-repeat-is-within-threshold",
      "D653.full-bootstrap-status-audit-json-test-path-remains-long-running",
      "D654.profile-split-closes-only-bootstrap-profile-frontier",
      "D655.optimization-candidate-is-ready-but-not-selected",
      "D656.full-json-bottleneck-is-not-external-solver-bottleneck",
      "D657.profile-telemetry-is-not-semantic-owner",
      "D658.runtime-flattening-and-meaning-db-remain-open-after-profile-split",
    ] {
      assert!(discoveries.contains_key(expected), "missing {expected}");
      assert!(as_bool(get(discoveries[expected], "scenario-only")));
    }
  });
}
