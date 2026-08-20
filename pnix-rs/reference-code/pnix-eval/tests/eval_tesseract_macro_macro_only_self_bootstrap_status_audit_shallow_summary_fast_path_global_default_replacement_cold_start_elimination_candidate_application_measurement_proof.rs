use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn combined_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../stdlib/lib/gate/macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application-measurement-proof.px",
  )
}

fn eval_combined() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = combined_path();
    let json = std::thread::Builder::new()
      .name("cold-start-application-measurement-receipt-eval".to_string())
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("cold-start application measurement receipt eval")
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
    "tesseract-macro-ontology-macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application-measurement-proof"
  );
  assert_eq!(
    as_str(get(r, "proof-helpers-owner")),
    "stdlib/lib/gate/macro-only-proof-helpers.px"
  );
  assert!(as_bool(get(
    r,
    "global-default-replacement-cold-start-elimination-candidate-application-measurement-proof"
  )));
  assert_eq!(as_i64(get(r, "cold-delta-ms")), 9474);
  assert!(as_bool(get(
    r,
    "bounded-status-query-cold-improvement-recorded"
  )));
  assert!(as_bool(get(r, "runtime-wiring-required")));
  assert!(!as_bool(get(r, "runtime-wired")));
  assert!(!as_bool(get(r, "cold-start-solved")));
  assert!(!as_bool(get(r, "cold-start-eliminated")));
  assert!(!as_bool(get(r, "cold-start-globally-bypassed")));
  assert!(!as_bool(get(r, "elimination-applied-globally")));
}

#[test]
fn contract_records_cold_delta_keeps_global_claims_held() {
  let r = receipt();
  let contract = get(r, "measurement-contract");
  assert_eq!(
    as_str(get(contract, "current-status")),
    "self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application-measurement-proof-present"
  );
  assert_eq!(as_i64(get(contract, "pre-wrapper-cold-duration-ms")), 12457);
  assert_eq!(as_i64(get(contract, "post-direct-cold-duration-ms")), 2983);
  assert_eq!(
    as_i64(get(contract, "post-direct-warm-min-duration-ms")),
    3024
  );
  assert_eq!(
    as_i64(get(contract, "post-direct-warm-max-duration-ms")),
    3025
  );
  assert_eq!(as_i64(get(contract, "cold-delta-ms")), 9474);
  assert_eq!(as_i64(get(contract, "measurement-record-count")), 4);
  assert!(as_bool(get(
    contract,
    "bounded-status-query-cold-improvement-recorded"
  )));
  assert!(as_bool(get(
    contract,
    "application-speedup-boundary-frontier-required"
  )));
  assert!(as_bool(get(contract, "runtime-wiring-required")));
  assert!(as_bool(get(
    contract,
    "closes-cold-start-application-measurement-frontier"
  )));
  assert!(as_bool(get(
    contract,
    "opens-cold-start-application-speedup-boundary-frontier"
  )));
  assert!(!as_bool(get(contract, "runtime-wired")));
  assert!(!as_bool(get(contract, "cold-start-solved")));
  assert!(!as_bool(get(contract, "cold-start-eliminated")));
  assert!(!as_bool(get(contract, "cold-start-globally-bypassed")));
  assert!(!as_bool(get(contract, "elimination-applied-globally")));
  assert!(!as_bool(get(contract, "global-speedup-claimed")));
  assert!(!as_bool(get(contract, "runtime-install")));
  assert!(!as_bool(get(contract, "self-modification")));
}

#[test]
fn trials_cover_cold_delta_warm_envelope_and_negatives() {
  let r = receipt();
  let trials = as_list(get(r, "measurement-trials"));
  assert_eq!(trials.len(), 27);
  let ids = trials
    .iter()
    .map(|trial| as_str(get(trial, "id")))
    .collect::<BTreeSet<_>>();
  for id in [
    "trial.A.valid-cold-start-application-measurement-proof",
    "trial.B.measurement-record-set",
    "trial.C.cold-delta-recorded",
    "trial.D.warm-envelope-recorded",
    "trial.E.next-frontier",
    "trial.I.application-source-missing",
    "trial.O.delta-mismatch",
    "trial.R.boundary-frontier-missing",
    "trial.V.measurement-overclaim",
    "trial.W.speedup-overclaim",
    "trial.X.runtime-overclaim",
    "trial.Y.external-or-license-overclaim",
    "trial.Z.authority-overclaim",
    "trial.AA.combined-file-emits-three-named-fields",
  ] {
    assert!(ids.contains(id), "missing {id}");
  }
}

#[test]
fn six_layer_fold_keeps_measurement_separate_from_runtime_wired() {
  let r = receipt();
  let fold = get(r, "six-layer-cold-start-application-measurement-fold");
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
  assert_eq!(as_i64(get(semantic, "pre-wrapper-cold-ms")), 12457);
  assert_eq!(as_i64(get(semantic, "post-direct-cold-ms")), 2983);
  assert_eq!(as_i64(get(semantic, "cold-delta-ms")), 9474);
  assert!(as_bool(get(
    semantic,
    "bounded-status-query-cold-improvement-recorded"
  )));
  assert!(as_bool(get(
    semantic,
    "application-speedup-boundary-frontier-required"
  )));
  assert!(as_bool(get(semantic, "runtime-wiring-required")));
  assert!(!as_bool(get(semantic, "runtime-wired")));
  assert!(!as_bool(get(semantic, "cold-start-solved")));
  assert!(!as_bool(get(semantic, "cold-start-eliminated")));
  assert!(!as_bool(get(semantic, "cold-start-globally-bypassed")));
  assert!(!as_bool(get(semantic, "elimination-applied-globally")));
  assert!(!as_bool(get(semantic, "global-speedup-claimed")));

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
}

#[test]
fn migration_delta_closes_application_measurement_and_opens_speedup_boundary_only() {
  let r = receipt();
  let delta = get(r, "migrationDelta");
  let closes = string_set(get(delta, "closes"));
  assert!(closes.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application-measurement-proof"
  ));
  let opens = string_set(get(delta, "opens"));
  assert!(opens.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application-speedup-boundary-proof"
  ));
  let does_not_close = string_set(get(delta, "does-not-close"));
  assert!(does_not_close.contains("need.self.bootstrap-status-audit-runtime-wired-proof"));
  assert!(does_not_close.contains("need.self.bootstrap-status-audit-cold-start-elimination-proof"));
  assert!(does_not_close.contains("need.global-speedup-proof"));
}

#[test]
fn discoveries_d819_to_d826_recorded_with_runtime_wired_held() {
  let r = receipt();
  let discoveries = as_list(get(r, "discoveries"));
  assert_eq!(discoveries.len(), 8);
  let ids = discoveries
    .iter()
    .map(|entry| as_str(get(entry, "id")))
    .collect::<BTreeSet<_>>();
  for id in [
    "D819.application-measurement-consumes-application-proof",
    "D820.measurement-records-pre-wrapper-cold-and-post-direct-cold-and-warm",
    "D821.cold-delta-9474ms-bounded-status-query-cold-improvement",
    "D822.warm-direct-stays-within-threshold",
    "D823.measurement-evidence-is-not-cold-start-solved-or-runtime-wired",
    "D824.runtime-wiring-required-as-separate-frontier",
    "D825.next-frontier-is-application-speedup-boundary-proof",
    "D826.continues-constitutional-combined-file-pattern",
  ] {
    assert!(ids.contains(id), "missing {id}");
  }

  for key in [
    "runtime-wired",
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
}
