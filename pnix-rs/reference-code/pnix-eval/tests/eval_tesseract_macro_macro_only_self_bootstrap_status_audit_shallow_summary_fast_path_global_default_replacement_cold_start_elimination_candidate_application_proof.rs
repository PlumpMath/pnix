use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn combined_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../stdlib/lib/gate/macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application-proof.px",
  )
}

fn eval_combined() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = combined_path();
    let json = std::thread::Builder::new()
      .name("cold-start-elimination-candidate-application-receipt-eval".to_string())
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("cold-start elimination candidate application receipt eval")
      })
      .expect("spawn eval thread")
      .join()
      .expect("eval thread panicked");
    serde_json::from_str(&json).expect("combined JSON")
  })
}

fn as_attrs(v: &Value) -> &Map<String, Value> {
  v.as_object()
    .unwrap_or_else(|| panic!("expected object, got {v:?}"))
}

fn as_list(v: &Value) -> &Vec<Value> {
  v.as_array()
    .unwrap_or_else(|| panic!("expected array, got {v:?}"))
}

fn as_str(v: &Value) -> &str {
  v.as_str()
    .unwrap_or_else(|| panic!("expected string, got {v:?}"))
}

fn as_bool(v: &Value) -> bool {
  v.as_bool()
    .unwrap_or_else(|| panic!("expected bool, got {v:?}"))
}

fn as_i64(v: &Value) -> i64 {
  v.as_i64()
    .unwrap_or_else(|| panic!("expected integer, got {v:?}"))
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

fn receipt() -> &'static Value {
  static R: OnceLock<&'static Value> = OnceLock::new();
  R.get_or_init(|| get(eval_combined(), "receipt"))
}

fn string_set(v: &Value) -> BTreeSet<&str> {
  as_list(v).iter().map(as_str).collect()
}

#[test]
fn receipt_exposes_probe_marker_and_helpers_reference() {
  let r = receipt();
  assert_eq!(
    as_str(get(r, "probe-marker")),
    "tesseract-macro-ontology-macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application-proof"
  );
  assert_eq!(
    as_str(get(r, "proof-helpers-owner")),
    "stdlib/lib/gate/macro-only-proof-helpers.px"
  );
  assert!(as_bool(get(
    r,
    "global-default-replacement-cold-start-elimination-candidate-application-proof"
  )));
  assert!(as_bool(get(r, "wrapper-bypass-applied")));
  assert!(as_bool(get(r, "bounded-application")));
  assert!(as_bool(get(r, "application-measurement-required")));
  assert!(!as_bool(get(r, "cold-start-solved")));
  assert!(!as_bool(get(r, "cold-start-eliminated")));
  assert!(!as_bool(get(r, "cold-start-globally-bypassed")));
  assert!(!as_bool(get(r, "elimination-applied-globally")));
}

#[test]
fn contract_records_bounded_application_not_global_elimination() {
  let r = receipt();
  let contract = get(r, "application-contract");
  assert_eq!(
    as_str(get(contract, "current-status")),
    "self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application-proof-present"
  );
  assert_eq!(
    as_str(get(contract, "application-bounded-scope")),
    "bootstrap-status-audit-shallow-summary-status-query-family"
  );
  assert_eq!(as_i64(get(contract, "application-record-count")), 3);
  assert_eq!(as_i64(get(contract, "applied-record-count")), 1);
  assert_eq!(
    as_str(get(contract, "applied-implementation")),
    "direct-evaluator-call-without-p-puck-cargo-spawn"
  );
  assert!(as_bool(get(contract, "wrapper-bypass-applied")));
  assert!(as_bool(get(contract, "bounded-application")));
  assert!(as_bool(get(contract, "application-measurement-required")));
  assert!(as_bool(get(
    contract,
    "closes-cold-start-elimination-candidate-application-frontier"
  )));
  assert!(as_bool(get(
    contract,
    "opens-cold-start-elimination-candidate-application-measurement-frontier"
  )));
  assert!(!as_bool(get(contract, "cold-start-solved")));
  assert!(!as_bool(get(contract, "cold-start-eliminated")));
  assert!(!as_bool(get(contract, "cold-start-globally-bypassed")));
  assert!(!as_bool(get(contract, "elimination-applied-globally")));
  assert!(!as_bool(get(contract, "runtime-install")));
  assert!(!as_bool(get(contract, "self-modification")));
}

#[test]
fn trials_cover_first_time_wrapper_bypass_applied_and_negatives() {
  let r = receipt();
  let trials = as_list(get(r, "application-trials"));
  assert_eq!(trials.len(), 25);
  let ids = trials
    .iter()
    .map(|trial| as_str(get(trial, "id")))
    .collect::<BTreeSet<_>>();
  for id in [
    "trial.A.valid-cold-start-elimination-candidate-application-proof",
    "trial.B.application-record-set",
    "trial.C.next-frontier",
    "trial.D.first-time-wrapper-bypass-applied-true",
    "trial.H.candidate-source-missing",
    "trial.L.application-mismatch",
    "trial.M.application-status-mismatch",
    "trial.O.registry-mismatch",
    "trial.P.measurement-frontier-missing",
    "trial.T.application-overclaim",
    "trial.U.speedup-overclaim",
    "trial.V.runtime-overclaim",
    "trial.W.external-or-license-overclaim",
    "trial.X.authority-overclaim",
    "trial.Y.combined-file-emits-three-named-fields",
  ] {
    assert!(ids.contains(id), "missing {id}");
  }
}

#[test]
fn six_layer_fold_keeps_application_separate_from_global_elimination() {
  let r = receipt();
  let fold = get(
    r,
    "six-layer-cold-start-elimination-candidate-application-fold",
  );
  assert_eq!(
    as_str(get(fold, "mode")),
    "macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application-proof"
  );
  let surface = get(fold, "surface");
  assert_eq!(
    as_str(get(surface, "single-file-emits")),
    "owner+ownerFixture+receipt"
  );
  assert_eq!(
    as_str(get(surface, "shared-helpers")),
    "stdlib/lib/gate/macro-only-proof-helpers.px"
  );

  let semantic = get(fold, "semantic");
  assert!(as_bool(get(semantic, "wrapper-bypass-applied")));
  assert!(as_bool(get(semantic, "bounded-application")));
  assert!(as_bool(get(semantic, "application-measurement-required")));
  assert!(!as_bool(get(semantic, "cold-start-solved")));
  assert!(!as_bool(get(semantic, "cold-start-eliminated")));
  assert!(!as_bool(get(semantic, "cold-start-globally-bypassed")));
  assert!(!as_bool(get(semantic, "elimination-applied-globally")));

  let runtime = get(fold, "runtime");
  for key in [
    "runtime-install",
    "global-ontology-runtime",
    "runtime-api-flattening",
    "meaning-db",
    "external-solver-installed",
    "self-modification",
  ] {
    assert!(!as_bool(get(runtime, key)), "`{key}` must stay false");
  }

  let audit = get(fold, "audit");
  let p_puck = get(audit, "p-puck-cold-sample-shape");
  assert_eq!(as_i64(get(p_puck, "duration-ms")), 12457);
  assert_eq!(as_str(get(p_puck, "status")), "slow-path-candidate");
  assert!(!as_bool(get(p_puck, "semantic-owner")));
  assert!(!as_bool(get(p_puck, "cold-start-attributable")));
  assert!(!as_bool(get(p_puck, "cold-start-solved")));
}

#[test]
fn migration_delta_closes_application_and_opens_measurement_only() {
  let r = receipt();
  let delta = get(r, "migrationDelta");
  let closes = string_set(get(delta, "closes"));
  assert!(closes.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application-proof"
  ));
  let opens = string_set(get(delta, "opens"));
  assert!(opens.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application-measurement-proof"
  ));
  let does_not_close = string_set(get(delta, "does-not-close"));
  assert!(does_not_close.contains("need.self.bootstrap-status-audit-cold-start-elimination-proof"));
  assert!(
    does_not_close.contains("need.self.bootstrap-status-audit-cold-start-globally-bypassed-proof")
  );
  assert!(
    does_not_close.contains("need.self.bootstrap-status-audit-elimination-applied-globally-proof")
  );
  assert!(does_not_close.contains("need.global-speedup-proof"));
}

#[test]
fn discoveries_d811_to_d818_recorded_first_wrapper_bypass_applied() {
  let r = receipt();
  let discoveries = as_list(get(r, "discoveries"));
  assert_eq!(discoveries.len(), 8);
  let ids = discoveries
    .iter()
    .map(|entry| as_str(get(entry, "id")))
    .collect::<BTreeSet<_>>();
  for id in [
    "D811.cold-start-elimination-candidate-application-consumes-candidate-proof",
    "D812.only-selected-wrapper-bypass-candidate-is-applied",
    "D813.first-slice-to-set-wrapper-bypass-applied-true",
    "D814.applied-scope-is-bounded-bootstrap-status-audit-shallow-summary-status-query",
    "D815.application-is-not-cold-start-solution-or-global-elimination",
    "D816.post-application-measurement-required",
    "D817.next-frontier-is-application-measurement-proof",
    "D818.continues-constitutional-combined-file-pattern",
  ] {
    assert!(ids.contains(id), "missing {id}");
  }

  for key in [
    "cold-start-solved",
    "cold-start-eliminated",
    "cold-start-globally-bypassed",
    "elimination-applied-globally",
    "global-speedup-claimed",
    "whole-system-speedup-claimed",
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
    "implementation-command",
  ] {
    assert!(!as_bool(get(r, key)), "`{key}` must stay false");
  }
  assert!(as_bool(get(r, "wrapper-bypass-applied")));
  assert!(as_bool(get(r, "bounded-application")));
}
