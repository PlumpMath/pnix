//! Macro-only self p-puck wrapper cold-start repeat proof.
//!
//! This receipt consumes bottleneck attribution and records two repeated p-puck
//! status queries over the current owner. Both repeat runs are within threshold,
//! so the wrapper slow-path candidate is not persistent on this cut. The proof
//! closes only the wrapper repeat frontier and leaves bootstrap audit profiling
//! plus optimization selection open.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../..")
}

fn fixture_path() -> PathBuf {
  repo_root().join(
    "fixtures/tesseract-macro-legacy-probe/macro_only_self_p_puck_wrapper_cold_start_repeat_proof_receipt.px",
  )
}

fn with_receipt(f: impl FnOnce(Value) + Send + 'static) {
  let path = fixture_path();
  std::thread::Builder::new()
    .name("eval-self-p-puck-wrapper-repeat-receipt".to_string())
    .stack_size(32 * 1024 * 1024)
    .spawn(move || {
      let run = eval_file(&path).expect("self p-puck wrapper repeat receipt");
      f(run);
    })
    .expect("spawn evaluator thread")
    .join()
    .expect("evaluator thread");
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

fn get_path<'a>(root: &'a Value, path: &[&str]) -> &'a Value {
  let mut cur = root;
  for key in path {
    cur = get(cur, key);
  }
  cur
}

fn string_set(v: &Value) -> BTreeSet<&str> {
  as_list(v).iter().map(as_str).collect()
}

fn attrs_by_id<'a>(items: &'a Value) -> BTreeMap<&'a str, &'a Value> {
  as_list(items)
    .iter()
    .map(|item| (as_str(get(item, "id")), item))
    .collect()
}

#[test]
fn marker_and_owner_surfaces_are_pinned() {
  with_receipt(|run| {
    assert_eq!(
      as_str(get(&run, "probe-marker")),
      "tesseract-macro-ontology-macro-only-self-p-puck-wrapper-cold-start-repeat-proof"
    );
    assert_eq!(
      as_str(get(&run, "truth-owner")),
      "project-wiki/maps/tesseract-macro-ontology-discovery-ledger.md"
    );
    assert_eq!(
      as_str(get(&run, "constitution-owner")),
      "stdlib/lib/gate/tesseract-constitution.px"
    );
    for path in [
      "stdlib/lib/gate/macro-only-self-p-puck-wrapper-cold-start-repeat-proof.px",
      "fixtures/pnix-query-runtime/macro-only-self-p-puck-wrapper-cold-start-repeat-proof-owner.px",
      "fixtures/tesseract-macro-legacy-probe/macro_only_self_p_puck_wrapper_cold_start_repeat_proof_receipt.px",
    ] {
      assert!(repo_root().join(path).is_file(), "missing `{path}`");
    }
  });
}

#[test]
fn constitution_gate_blocks_wrapper_repeat_overclaims() {
  with_receipt(|run| {
    let gate = get(&run, "constitutionGate");
    assert_eq!(
      as_str(get(gate, "scenario")),
      "macro-only-self-p-puck-wrapper-cold-start-repeat-proof"
    );
    assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
    assert!(!as_bool(get(gate, "accepted")));
    let blocks = string_set(get(gate, "blocked-shortcuts"));
    for expected in [
      "within-threshold-repeat-equals-fast-path-promotion",
      "within-threshold-repeat-erases-bootstrap-audit-bottleneck",
      "wrapper-repeat-equals-global-runtime",
      "wrapper-repeat-equals-runtime-api-flattening",
      "wrapper-repeat-equals-meaning-db",
      "p-puck-repeat-equals-semantic-owner",
      "wrapper-repeat-equals-external-solver-intake",
      "wrapper-repeat-equals-self-modification",
    ] {
      assert!(blocks.contains(expected), "missing block `{expected}`");
    }
  });
}

#[test]
fn contract_closes_wrapper_repeat_only() {
  with_receipt(|run| {
    let contract = get(&run, "wrapper-repeat-contract");
    assert_eq!(
      as_str(get(contract, "id")),
      "contract.macro-only-self-p-puck-wrapper-cold-start-repeat-proof.v1"
    );
    assert_eq!(
      as_str(get(contract, "current-status")),
      "self-p-puck-wrapper-cold-start-repeat-proof-present"
    );
    assert_eq!(
      as_i64(get(contract, "prior-execution-wrapper-duration-ms")),
      9420
    );
    assert_eq!(
      as_i64(get(contract, "prior-attribution-wrapper-duration-ms")),
      11416
    );
    assert_eq!(as_i64(get(contract, "repeat-one-duration-ms")), 357);
    assert_eq!(as_i64(get(contract, "repeat-two-duration-ms")), 246);
    assert!(as_bool(get(contract, "closes-wrapper-repeat-frontier")));
    assert!(!as_bool(get(
      contract,
      "persistent-p-puck-wrapper-slow-path"
    )));
    assert!(!as_bool(get(
      contract,
      "profile-required-from-wrapper-repeat"
    )));
    assert!(as_bool(get(
      contract,
      "bootstrap-status-audit-bottleneck-candidate"
    )));
    for key in [
      "selects-optimization",
      "promotes-fast-path",
      "installs-external-solver",
      "closes-global-runtime",
      "closes-runtime-api-flattening",
      "closes-meaning-db",
      "grants-llm-authority",
      "self-modification",
    ] {
      assert!(!as_bool(get(contract, key)), "`{key}` must stay false");
    }
  });
}

#[test]
fn wrapper_repeat_proof_records_within_threshold_runs() {
  with_receipt(|run| {
    let proof = get(&run, "wrapper-repeat-proof");
    assert_eq!(
      as_str(get(proof, "status")),
      "self-p-puck-wrapper-cold-start-repeat-proof-present"
    );
    assert!(as_bool(get(
      proof,
      "p-puck-wrapper-cold-start-repeat-proof"
    )));
    assert!(as_bool(get(
      proof,
      "p-puck-wrapper-repeat-within-threshold"
    )));
    assert_eq!(as_i64(get(proof, "repeat-record-count")), 2);
    assert_eq!(as_i64(get(proof, "repeat-max-duration-ms")), 357);
    assert_eq!(as_i64(get(proof, "repeat-min-duration-ms")), 246);
    assert_eq!(as_i64(get(proof, "repeat-delta-from-prior-ms")), -11059);
    assert!(!as_bool(get(proof, "persistent-p-puck-wrapper-slow-path")));
    assert!(!as_bool(get(proof, "profile-required-from-wrapper-repeat")));
    assert!(as_bool(get(
      proof,
      "bootstrap-status-audit-bottleneck-candidate"
    )));
  });
}

#[test]
fn trials_cover_valid_repeat_and_held_overclaims() {
  with_receipt(|run| {
    let trials = attrs_by_id(get(&run, "wrapper-repeat-trials"));
    assert_eq!(trials.len(), 19);
    assert_eq!(
      as_str(get(trials["trial.A.valid-wrapper-repeat-proof"], "outcome")),
      "self-p-puck-wrapper-cold-start-repeat-proof-present"
    );
    assert_eq!(
      as_str(get(
        trials["trial.C.prior-wrapper-slow-path-records"],
        "outcome"
      )),
      "slow-path-candidate"
    );
    assert_eq!(
      as_str(get(trials["trial.D.repeat-run-1"], "outcome")),
      "within-threshold"
    );
    assert_eq!(
      as_i64(get(trials["trial.D.repeat-run-1"], "duration-ms")),
      357
    );
    assert_eq!(
      as_i64(get(trials["trial.E.repeat-run-2"], "duration-ms")),
      246
    );
    for (id, held) in [
      (
        "trial.F.wrong-proof-id",
        "held.macro-only-self-p-puck-wrapper-cold-start-repeat.proof-id-mismatch",
      ),
      (
        "trial.G.stale-stage",
        "held.macro-only-self-p-puck-wrapper-cold-start-repeat.stale-current-stage",
      ),
      (
        "trial.H.source-mismatch",
        "held.macro-only-self-p-puck-wrapper-cold-start-repeat.source-mismatch",
      ),
      (
        "trial.I.attribution-input-missing",
        "held.macro-only-self-p-puck-wrapper-cold-start-repeat.attribution-input-missing",
      ),
      (
        "trial.J.prior-slow-path-drift",
        "held.macro-only-self-p-puck-wrapper-cold-start-repeat.prior-slow-path-drift",
      ),
      (
        "trial.K.repeat-record-shape-mismatch",
        "held.macro-only-self-p-puck-wrapper-cold-start-repeat.repeat-record-shape-mismatch",
      ),
      (
        "trial.L.repeat-record-invalid",
        "held.macro-only-self-p-puck-wrapper-cold-start-repeat.repeat-record-invalid",
      ),
      (
        "trial.M.repeat-summary-drift",
        "held.macro-only-self-p-puck-wrapper-cold-start-repeat.repeat-summary-drift",
      ),
      (
        "trial.N.frontier-shape-mismatch",
        "held.macro-only-self-p-puck-wrapper-cold-start-repeat.frontier-shape-mismatch",
      ),
      (
        "trial.O.persistent-wrapper-overclaim",
        "held.macro-only-self-p-puck-wrapper-cold-start-repeat.persistent-wrapper-overclaim",
      ),
      (
        "trial.P.optimization-overclaim",
        "held.macro-only-self-p-puck-wrapper-cold-start-repeat.optimization-overclaim",
      ),
      (
        "trial.Q.runtime-overclaim",
        "held.macro-only-self-p-puck-wrapper-cold-start-repeat.runtime-overclaim",
      ),
      (
        "trial.R.authority-overclaim",
        "held.macro-only-self-p-puck-wrapper-cold-start-repeat.authority-overclaim",
      ),
      (
        "trial.S.gpl-family-dependency",
        "held.macro-only-self-p-puck-wrapper-cold-start-repeat.gpl-family-dependency",
      ),
    ] {
      assert_eq!(as_str(get(trials[id], "outcome")), "Held", "{id}");
      assert_eq!(as_str(get(trials[id], "held-id")), held, "{id}");
    }
  });
}

#[test]
fn six_layer_fold_separates_wrapper_repeat_from_bootstrap_and_optimization() {
  with_receipt(|run| {
    assert!(as_bool(get_path(
      &run,
      &[
        "six-layer-wrapper-repeat-fold",
        "semantic",
        "p-puck-wrapper-repeat-within-threshold",
      ],
    )));
    assert!(!as_bool(get_path(
      &run,
      &[
        "six-layer-wrapper-repeat-fold",
        "semantic",
        "persistent-p-puck-wrapper-slow-path",
      ],
    )));
    assert!(as_bool(get_path(
      &run,
      &[
        "six-layer-wrapper-repeat-fold",
        "semantic",
        "bootstrap-status-audit-bottleneck-candidate",
      ],
    )));
    for path in [
      &[
        "six-layer-wrapper-repeat-fold",
        "runtime",
        "optimization-selected",
      ][..],
      &[
        "six-layer-wrapper-repeat-fold",
        "runtime",
        "fast-path-promoted",
      ][..],
      &[
        "six-layer-wrapper-repeat-fold",
        "runtime",
        "runtime-api-flattening",
      ][..],
      &["six-layer-wrapper-repeat-fold", "runtime", "meaning-db"][..],
    ] {
      assert!(!as_bool(get_path(&run, path)), "{path:?} must stay false");
    }
  });
}

#[test]
fn discoveries_record_d643_through_d650() {
  with_receipt(|run| {
    let discoveries = attrs_by_id(get(&run, "discoveries"));
    assert_eq!(discoveries.len(), 8);
    for expected in [
      "D643.p-puck-wrapper-slow-path-is-not-persistent-on-repeat",
      "D644.wrapper-repeat-separates-cold-start-spike-from-semantic-fold",
      "D645.wrapper-repeat-closes-only-wrapper-repeat-frontier",
      "D646.bootstrap-status-audit-remains-bottleneck-candidate",
      "D647.optimization-candidate-stays-held-until-bootstrap-profile-split",
      "D648.p-puck-repeat-is-measurement-not-semantic-owner",
      "D649.external-solver-intake-not-justified-by-wrapper-repeat",
      "D650.runtime-flattening-and-meaning-db-remain-open-after-wrapper-repeat",
    ] {
      let d = discoveries
        .get(expected)
        .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
      assert!(as_bool(get(d, "scenario-only")));
      assert_eq!(as_str(get(d, "decision-pressure")), "keep");
    }
  });
}

#[test]
fn migration_delta_closes_wrapper_repeat_and_leaves_followups_open() {
  with_receipt(|run| {
    let delta = get(&run, "migrationDelta");
    let closes = string_set(get(delta, "closes"));
    assert!(closes.contains("need.self.p-puck-wrapper-cold-start-repeat-proof"));
    let does_not_close = string_set(get(delta, "does-not-close"));
    assert!(does_not_close.contains("need.self.bootstrap-status-audit-profile-split-proof"));
    assert!(
      does_not_close.contains("need.self.optimization-candidate-after-bottleneck-attribution")
    );
    for key in [
      "optimization-selected",
      "fast-path-promoted",
      "external-solver-installed",
      "global-ontology-runtime",
      "runtime-api-flattening",
      "meaning-db",
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
