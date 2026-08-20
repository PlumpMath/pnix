use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn receipt_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/tesseract-macro-legacy-probe/macro_only_self_bootstrap_status_audit_shallow_summary_fast_path_global_default_replacement_speedup_boundary_proof_receipt.px",
  )
}

fn eval_receipt() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = receipt_path();
    let json = std::thread::Builder::new()
      .name("tesseract-global-default-speedup-boundary-receipt-eval".to_string())
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("global default replacement speedup boundary proof receipt")
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
fn receipt_exposes_probe_marker_and_positive_boundary_contract() {
  let receipt = eval_receipt();
  assert_eq!(
    as_str(get(receipt, "probe-marker")),
    "tesseract-macro-ontology-macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-speedup-boundary-proof"
  );
  assert!(as_bool(get(
    receipt,
    "global-default-replacement-speedup-boundary-proof"
  )));
  assert!(as_bool(get(receipt, "bounded-warm-envelope-accepted")));
  assert!(as_bool(get(
    receipt,
    "bounded-status-query-fast-path-signal"
  )));
  assert!(as_bool(get(receipt, "comparable-benchmark-required")));
  assert!(!as_bool(get(receipt, "global-speedup-claimed")));
}

#[test]
fn contract_accepts_bounded_warm_signal_and_keeps_global_speedup_held() {
  let receipt = eval_receipt();
  let contract = get(receipt, "boundary-contract");
  assert_eq!(
    as_str(get(contract, "current-status")),
    "self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-speedup-boundary-proof-present"
  );
  assert_eq!(
    as_str(get(contract, "speedup-boundary-verdict")),
    "bounded-warm-envelope-accepted-global-speedup-held-comparable-benchmark-required"
  );
  assert_eq!(
    as_str(get(contract, "accepted-signal")),
    "bounded-status-query-warm-repeat-envelope"
  );
  assert_eq!(
    as_i64(get(contract, "post-application-warm-min-duration-ms")),
    266
  );
  assert_eq!(
    as_i64(get(contract, "post-application-warm-max-duration-ms")),
    312
  );
  assert!(as_bool(get(contract, "bounded-warm-envelope-accepted")));
  assert!(as_bool(get(contract, "comparable-benchmark-required")));
  assert!(!as_bool(get(contract, "comparable-benchmark-present")));
  assert!(!as_bool(get(
    contract,
    "apples-to-apples-global-speedup-comparison"
  )));
  assert_eq!(
    as_str(get(contract, "global-speedup-comparison-status")),
    "Held"
  );
  assert!(!as_bool(get(contract, "global-speedup-claimed")));
  assert!(!as_bool(get(contract, "cold-start-solved")));
}

#[test]
fn trials_cover_valid_negative_and_overclaim_paths() {
  let receipt = eval_receipt();
  let trials = as_list(get(receipt, "boundary-trials"));
  assert_eq!(trials.len(), 19);
  let ids = trials
    .iter()
    .map(|trial| as_str(get(trial, "id")))
    .collect::<BTreeSet<_>>();
  for id in [
    "trial.A.valid-speedup-boundary-proof",
    "trial.H.measurement-evidence-missing",
    "trial.L.comparability-mismatch",
    "trial.P.speedup-overclaim",
    "trial.Q.runtime-overclaim",
    "trial.R.external-or-license-overclaim",
    "trial.S.authority-overclaim",
  ] {
    assert!(ids.contains(id), "missing {id}");
  }
}

#[test]
fn six_layer_fold_keeps_boundary_signal_separate_from_runtime_install() {
  let receipt = eval_receipt();
  let fold = get(receipt, "six-layer-boundary-fold");
  assert_eq!(
    as_str(get(fold, "mode")),
    "macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-speedup-boundary-proof"
  );
  let semantic = get(fold, "semantic");
  assert!(as_bool(get(semantic, "bounded-warm-envelope-accepted")));
  assert!(as_bool(get(
    semantic,
    "bounded-status-query-fast-path-signal"
  )));
  assert_eq!(
    as_str(get(semantic, "global-speedup-comparison-status")),
    "Held"
  );
  assert!(!as_bool(get(semantic, "global-speedup-claimed")));
  assert!(!as_bool(get(semantic, "cold-start-solved")));

  let runtime = get(fold, "runtime");
  assert!(as_bool(get(runtime, "local-fast-path-signal")));
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
fn migration_delta_closes_boundary_and_opens_comparable_benchmark_only() {
  let receipt = eval_receipt();
  let delta = get(receipt, "migrationDelta");
  let closes = string_set(get(delta, "closes"));
  assert!(closes.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-speedup-boundary-proof"
  ));
  let opens = string_set(get(delta, "opens"));
  assert!(opens.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-comparable-benchmark-proof"
  ));
  let does_not_close = string_set(get(delta, "does-not-close"));
  assert!(does_not_close.contains("need.global-speedup-proof"));
  assert!(does_not_close.contains("need.global-runtime-install.proof-after-semantic-owner"));
  assert!(does_not_close.contains("need.stdlib.meaning-db"));
}

#[test]
fn discoveries_d763_to_d770_are_recorded_and_hard_stops_remain_false() {
  let receipt = eval_receipt();
  let discoveries = as_list(get(receipt, "discoveries"));
  assert_eq!(discoveries.len(), 8);
  let ids = discoveries
    .iter()
    .map(|entry| as_str(get(entry, "id")))
    .collect::<BTreeSet<_>>();
  for id in [
    "D763.speedup-boundary-consumes-measurement-proof",
    "D764.bounded-warm-envelope-becomes-usable-fast-path-signal",
    "D765.global-speedup-remains-held-without-comparable-benchmark",
    "D766.cold-start-remains-held-after-warm-signal",
    "D767.comparable-benchmark-proof-is-required",
    "D768.scope-comparability-prevents-measurement-overclaim",
    "D769.boundary-preserves-runtime-external-license-and-authority-stops",
    "D770.next-frontier-is-comparable-benchmark-proof",
  ] {
    assert!(ids.contains(id), "missing {id}");
  }

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
