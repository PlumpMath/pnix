use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn combined_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../stdlib/lib/gate/macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application-speedup-boundary-proof.px",
  )
}

fn eval_combined() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = combined_path();
    let json = std::thread::Builder::new()
      .name("cold-start-app-speedup-boundary-receipt-eval".to_string())
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("cold-start application speedup boundary receipt eval")
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
    "tesseract-macro-ontology-macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application-speedup-boundary-proof"
  );
  assert_eq!(
    as_str(get(r, "proof-helpers-owner")),
    "stdlib/lib/gate/macro-only-proof-helpers.px"
  );
  assert!(as_bool(get(r, "bounded-warm-envelope-accepted")));
  assert!(as_bool(get(r, "bounded-cold-delta-signal-accepted")));
  assert!(as_bool(get(r, "bounded-status-query-fast-path-signal")));
  assert!(as_bool(get(r, "comparable-benchmark-required")));
  assert!(as_bool(get(r, "runtime-wiring-required")));
  assert!(!as_bool(get(r, "runtime-wired")));
  assert!(!as_bool(get(r, "cold-start-solved")));
  assert!(!as_bool(get(r, "cold-start-eliminated")));
  assert!(!as_bool(get(r, "global-speedup-claimed")));
}

#[test]
fn contract_records_acceptance_keeps_runtime_wired_and_global_speedup_held() {
  let r = receipt();
  let contract = get(r, "boundary-contract");
  assert_eq!(
    as_str(get(contract, "current-status")),
    "self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application-speedup-boundary-proof-present"
  );
  assert_eq!(
    as_str(get(contract, "boundary-verdict")),
    "bounded-warm-and-cold-delta-signals-accepted-runtime-wired-and-global-speedup-held"
  );
  assert_eq!(as_i64(get(contract, "accepted-warm-min-duration-ms")), 3024);
  assert_eq!(as_i64(get(contract, "accepted-warm-max-duration-ms")), 3025);
  assert_eq!(as_i64(get(contract, "accepted-cold-delta-ms")), 9474);
  assert!(as_bool(get(contract, "bounded-warm-envelope-accepted")));
  assert!(as_bool(get(contract, "bounded-cold-delta-signal-accepted")));
  assert!(as_bool(get(contract, "comparable-benchmark-required")));
  assert!(as_bool(get(contract, "runtime-wiring-required")));
  assert!(as_bool(get(
    contract,
    "closes-cold-start-application-speedup-boundary-frontier"
  )));
  assert!(as_bool(get(
    contract,
    "opens-cold-start-application-comparable-benchmark-frontier"
  )));
  assert!(!as_bool(get(contract, "runtime-wired")));
  assert!(!as_bool(get(contract, "cold-start-solved")));
  assert!(!as_bool(get(contract, "global-speedup-claimed")));
  assert!(!as_bool(get(contract, "runtime-install")));
}

#[test]
fn trials_cover_acceptance_and_negatives() {
  let r = receipt();
  let trials = as_list(get(r, "boundary-trials"));
  assert_eq!(trials.len(), 22);
  let ids = trials
    .iter()
    .map(|trial| as_str(get(trial, "id")))
    .collect::<BTreeSet<_>>();
  for id in [
    "trial.A.valid-cold-start-application-speedup-boundary-proof",
    "trial.B.bounded-warm-envelope",
    "trial.C.bounded-cold-delta",
    "trial.D.next-frontier",
    "trial.H.measurement-source-missing",
    "trial.L.acceptance-mismatch",
    "trial.M.held-flags-missing",
    "trial.Q.boundary-overclaim",
    "trial.R.speedup-overclaim",
    "trial.S.runtime-overclaim",
    "trial.T.external-or-license-overclaim",
    "trial.U.authority-overclaim",
    "trial.V.combined-file-emits-three-named-fields",
  ] {
    assert!(ids.contains(id), "missing {id}");
  }
}

#[test]
fn six_layer_fold_keeps_acceptance_separate_from_runtime_wired() {
  let r = receipt();
  let fold = get(r, "six-layer-cold-start-application-speedup-boundary-fold");
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
  assert!(as_bool(get(semantic, "bounded-warm-envelope-accepted")));
  assert!(as_bool(get(semantic, "bounded-cold-delta-signal-accepted")));
  assert!(as_bool(get(
    semantic,
    "bounded-status-query-fast-path-signal"
  )));
  assert!(as_bool(get(semantic, "comparable-benchmark-required")));
  assert!(as_bool(get(semantic, "runtime-wiring-required")));
  assert!(!as_bool(get(semantic, "runtime-wired")));
  assert!(!as_bool(get(semantic, "cold-start-solved")));
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
fn migration_delta_closes_speedup_boundary_and_opens_comparable_benchmark_only() {
  let r = receipt();
  let delta = get(r, "migrationDelta");
  let closes = string_set(get(delta, "closes"));
  assert!(closes.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application-speedup-boundary-proof"
  ));
  let opens = string_set(get(delta, "opens"));
  assert!(opens.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-application-comparable-benchmark-proof"
  ));
  let does_not_close = string_set(get(delta, "does-not-close"));
  assert!(does_not_close.contains("need.self.bootstrap-status-audit-runtime-wired-proof"));
  assert!(does_not_close.contains("need.self.bootstrap-status-audit-cold-start-elimination-proof"));
  assert!(does_not_close.contains("need.global-speedup-proof"));
}

#[test]
fn discoveries_d827_to_d834_recorded_with_all_global_held() {
  let r = receipt();
  let discoveries = as_list(get(r, "discoveries"));
  assert_eq!(discoveries.len(), 8);
  let ids = discoveries
    .iter()
    .map(|entry| as_str(get(entry, "id")))
    .collect::<BTreeSet<_>>();
  for id in [
    "D827.application-speedup-boundary-consumes-application-measurement",
    "D828.bounded-warm-envelope-accepted-as-usable-signal",
    "D829.bounded-cold-delta-signal-accepted-as-usable-signal",
    "D830.global-speedup-and-runtime-wired-remain-held",
    "D831.comparable-benchmark-required-as-next-frontier",
    "D832.runtime-wiring-required-stays-set",
    "D833.next-frontier-is-application-comparable-benchmark-proof",
    "D834.continues-constitutional-combined-file-pattern",
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
