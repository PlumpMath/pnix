use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn receipt_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/tesseract-macro-legacy-probe/macro_only_self_bootstrap_status_audit_shallow_summary_fast_path_global_default_replacement_measurement_proof_receipt.px",
  )
}

fn eval_receipt() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = receipt_path();
    let json = std::thread::Builder::new()
      .name("tesseract-global-default-measurement-receipt-eval".to_string())
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("global default replacement measurement proof receipt")
      })
      .expect("spawn eval thread")
      .join()
      .expect("eval thread panicked");
    serde_json::from_str(&json).expect("receipt JSON")
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

fn string_set(v: &Value) -> BTreeSet<&str> {
  as_list(v).iter().map(as_str).collect()
}

#[test]
fn receipt_exposes_probe_marker_and_positive_measurement_contract() {
  let receipt = eval_receipt();
  assert_eq!(
    as_str(get(receipt, "probe-marker")),
    "tesseract-macro-ontology-macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-measurement-proof"
  );
  assert!(as_bool(get(
    receipt,
    "global-default-replacement-measurement-proof"
  )));
  assert!(as_bool(get(receipt, "post-application-measured")));
  assert!(as_bool(get(
    receipt,
    "post-application-warm-repeats-within-threshold"
  )));
  assert!(as_bool(get(
    receipt,
    "post-application-cold-start-slow-path-candidate"
  )));
  assert!(as_bool(get(
    receipt,
    "global-speedup-boundary-proof-required"
  )));
}

#[test]
fn contract_records_actual_post_application_samples_without_speedup_or_cold_start_claim() {
  let receipt = eval_receipt();
  let contract = get(receipt, "measurement-contract");
  assert_eq!(
    as_str(get(contract, "current-status")),
    "self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-measurement-proof-present"
  );
  assert_eq!(
    as_i64(get(contract, "post-application-cold-start-duration-ms")),
    10544
  );
  assert_eq!(
    as_i64(get(contract, "post-application-warm-min-duration-ms")),
    266
  );
  assert_eq!(
    as_i64(get(contract, "post-application-warm-max-duration-ms")),
    312
  );
  assert_eq!(
    as_i64(get(contract, "pre-application-measurement-record-count")),
    8
  );
  assert_eq!(
    as_i64(get(contract, "post-application-measurement-record-count")),
    3
  );
  assert_eq!(
    as_i64(get(contract, "combined-measurement-record-count")),
    11
  );
  assert!(as_bool(get(
    contract,
    "post-application-warm-repeats-within-threshold"
  )));
  assert!(!as_bool(get(contract, "global-speedup-claimed")));
  assert!(!as_bool(get(contract, "cold-start-solved")));
}

#[test]
fn trials_cover_valid_negative_and_overclaim_paths() {
  let receipt = eval_receipt();
  let trials = as_list(get(receipt, "measurement-trials"));
  assert_eq!(trials.len(), 21);
  let ids = trials
    .iter()
    .map(|trial| as_str(get(trial, "id")))
    .collect::<BTreeSet<_>>();
  for id in [
    "trial.A.valid-measurement-proof",
    "trial.H.application-evidence-missing",
    "trial.K.record-shape-invalid",
    "trial.L.sample-values-mismatch",
    "trial.R.measurement-overclaim",
    "trial.S.runtime-overclaim",
    "trial.T.external-or-license-overclaim",
    "trial.U.authority-overclaim",
  ] {
    assert!(ids.contains(id), "missing {id}");
  }
}

#[test]
fn six_layer_fold_keeps_measurement_scope_separate_from_runtime_install() {
  let receipt = eval_receipt();
  let fold = get(receipt, "six-layer-measurement-fold");
  assert_eq!(
    as_str(get(fold, "mode")),
    "macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-measurement-proof"
  );
  let semantic = get(fold, "semantic");
  assert_eq!(
    as_str(get(semantic, "performance-envelope")),
    "global-default-replacement-post-application-cold-start-slow-warm-repeats-within-threshold"
  );
  assert!(as_bool(get(
    semantic,
    "post-application-warm-repeats-within-threshold"
  )));
  assert!(!as_bool(get(semantic, "global-speedup-claimed")));
  assert!(!as_bool(get(semantic, "cold-start-solved")));

  let runtime = get(fold, "runtime");
  assert!(as_bool(get(runtime, "global-default-replacement-applied")));
  assert!(as_bool(get(runtime, "global-default-callsite-replaced")));
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
fn migration_delta_closes_measurement_and_opens_speedup_boundary_only() {
  let receipt = eval_receipt();
  let delta = get(receipt, "migrationDelta");
  let closes = string_set(get(delta, "closes"));
  assert!(closes.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-measurement-proof"
  ));
  let opens = string_set(get(delta, "opens"));
  assert!(opens.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-speedup-boundary-proof"
  ));
  let does_not_close = string_set(get(delta, "does-not-close"));
  assert!(does_not_close.contains("need.global-speedup-proof"));
  assert!(does_not_close.contains("need.global-runtime-install.proof-after-semantic-owner"));
  assert!(does_not_close.contains("need.stdlib.meaning-db"));
}

#[test]
fn discoveries_d755_to_d762_are_recorded() {
  let receipt = eval_receipt();
  let discoveries = as_list(get(receipt, "discoveries"));
  assert_eq!(discoveries.len(), 8);
  let ids = discoveries
    .iter()
    .map(|entry| as_str(get(entry, "id")))
    .collect::<BTreeSet<_>>();
  for id in [
    "D755.measurement-consumes-application-not-readiness",
    "D756.post-application-p-puck-samples-recorded",
    "D757.pre-application-envelope-is-inherited",
    "D758.warm-repeat-evidence-is-not-cold-start-solution",
    "D759.status-query-measurement-is-not-whole-system-speedup",
    "D760.bounded-default-replacement-remains-three-callsite-scope",
    "D761.measurement-preserves-runtime-external-license-and-authority-boundaries",
    "D762.next-frontier-is-global-default-replacement-speedup-boundary-proof",
  ] {
    assert!(ids.contains(id), "missing {id}");
  }
}

#[test]
fn receipt_preserves_all_hard_stops() {
  let receipt = eval_receipt();
  for key in [
    "global-speedup-claimed",
    "cold-start-solved",
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
    assert!(!as_bool(get(receipt, key)), "`{key}` must stay false");
  }
}
