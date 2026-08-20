use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn receipt_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/tesseract-macro-legacy-probe/macro_only_self_bootstrap_status_audit_shallow_summary_fast_path_global_default_replacement_comparable_benchmark_proof_receipt.px",
  )
}

fn eval_receipt() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = receipt_path();
    let json = std::thread::Builder::new()
      .name("tesseract-global-default-comparable-benchmark-receipt-eval".to_string())
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("global default replacement comparable benchmark proof receipt")
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
fn receipt_exposes_probe_marker_and_positive_benchmark_contract() {
  let receipt = eval_receipt();
  assert_eq!(
    as_str(get(receipt, "probe-marker")),
    "tesseract-macro-ontology-macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-comparable-benchmark-proof"
  );
  assert!(as_bool(get(
    receipt,
    "global-default-replacement-comparable-benchmark-proof"
  )));
  assert!(as_bool(get(receipt, "comparable-benchmark-present")));
  assert!(as_bool(get(receipt, "bounded-status-query-speedup-proven")));
  assert!(!as_bool(get(receipt, "global-speedup-claimed")));
}

#[test]
fn contract_records_bounded_status_query_speedup_not_global_speedup() {
  let receipt = eval_receipt();
  let contract = get(receipt, "benchmark-contract");
  assert_eq!(
    as_str(get(contract, "current-status")),
    "self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-comparable-benchmark-proof-present"
  );
  assert_eq!(
    as_str(get(contract, "benchmark-verdict")),
    "bounded-status-query-speedup-proven-whole-system-speedup-held"
  );
  assert_eq!(as_i64(get(contract, "baseline-warm-max-duration-ms")), 827);
  assert_eq!(as_i64(get(contract, "candidate-warm-max-duration-ms")), 312);
  assert_eq!(as_i64(get(contract, "warm-max-improvement-ms")), 515);
  assert_eq!(as_str(get(contract, "warm-max-speedup-ratio")), "2.65x");
  assert!(as_bool(get(
    contract,
    "apples-to-apples-status-query-comparison"
  )));
  assert!(!as_bool(get(
    contract,
    "apples-to-apples-global-speedup-comparison"
  )));
  assert!(as_bool(get(
    contract,
    "bounded-status-query-speedup-proven"
  )));
  assert!(!as_bool(get(contract, "global-speedup-claimed")));
  assert!(!as_bool(get(contract, "whole-system-speedup-claimed")));
  assert!(!as_bool(get(contract, "cold-start-solved")));
  assert!(as_bool(get(contract, "cold-start-boundary-proof-required")));
}

#[test]
fn trials_cover_valid_negative_and_overclaim_paths() {
  let receipt = eval_receipt();
  let trials = as_list(get(receipt, "benchmark-trials"));
  assert_eq!(trials.len(), 19);
  let ids = trials
    .iter()
    .map(|trial| as_str(get(trial, "id")))
    .collect::<BTreeSet<_>>();
  for id in [
    "trial.A.valid-comparable-benchmark-proof",
    "trial.H.speedup-boundary-missing",
    "trial.I.baseline-missing",
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
fn six_layer_fold_keeps_benchmark_speedup_separate_from_runtime_install() {
  let receipt = eval_receipt();
  let fold = get(receipt, "six-layer-benchmark-fold");
  assert_eq!(
    as_str(get(fold, "mode")),
    "macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-comparable-benchmark-proof"
  );
  let semantic = get(fold, "semantic");
  assert!(as_bool(get(
    semantic,
    "bounded-status-query-speedup-proven"
  )));
  assert!(as_bool(get(
    semantic,
    "apples-to-apples-status-query-comparison"
  )));
  assert!(!as_bool(get(
    semantic,
    "apples-to-apples-global-speedup-comparison"
  )));
  assert!(!as_bool(get(semantic, "global-speedup-claimed")));
  assert!(!as_bool(get(semantic, "whole-system-speedup-claimed")));
  assert!(!as_bool(get(semantic, "cold-start-solved")));

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
fn migration_delta_closes_comparable_benchmark_and_opens_cold_start_boundary_only() {
  let receipt = eval_receipt();
  let delta = get(receipt, "migrationDelta");
  let closes = string_set(get(delta, "closes"));
  assert!(closes.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-comparable-benchmark-proof"
  ));
  let opens = string_set(get(delta, "opens"));
  assert!(opens.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-boundary-proof"
  ));
  let does_not_close = string_set(get(delta, "does-not-close"));
  assert!(does_not_close.contains("need.global-speedup-proof"));
  assert!(does_not_close.contains("need.self.bootstrap-status-audit-cold-start-elimination-proof"));
  assert!(does_not_close.contains("need.stdlib.meaning-db"));
}

#[test]
fn discoveries_d771_to_d778_are_recorded_and_hard_stops_remain_false() {
  let receipt = eval_receipt();
  let discoveries = as_list(get(receipt, "discoveries"));
  assert_eq!(discoveries.len(), 8);
  let ids = discoveries
    .iter()
    .map(|entry| as_str(get(entry, "id")))
    .collect::<BTreeSet<_>>();
  for id in [
    "D771.comparable-benchmark-consumes-speedup-boundary",
    "D772.comparison-scope-is-status-query-family-not-whole-system",
    "D773.pre-global-default-warm-envelope-is-baseline",
    "D774.post-global-default-warm-envelope-is-candidate",
    "D775.bounded-status-query-speedup-is-proven",
    "D776.whole-system-global-speedup-remains-held",
    "D777.cold-start-remains-separate-frontier",
    "D778.next-frontier-is-cold-start-boundary-proof",
  ] {
    assert!(ids.contains(id), "missing {id}");
  }

  for key in [
    "global-speedup-claimed",
    "whole-system-speedup-claimed",
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
