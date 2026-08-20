use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn receipt_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/tesseract-macro-legacy-probe/macro_only_self_bootstrap_status_audit_shallow_summary_fast_path_global_default_replacement_cold_start_boundary_proof_receipt.px",
  )
}

fn eval_receipt() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = receipt_path();
    let json = std::thread::Builder::new()
      .name("tesseract-global-default-cold-start-boundary-receipt-eval".to_string())
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("global default replacement cold-start boundary proof receipt")
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
    "tesseract-macro-ontology-macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-boundary-proof"
  );
  assert!(as_bool(get(
    receipt,
    "global-default-replacement-cold-start-boundary-proof"
  )));
  assert!(as_bool(get(receipt, "cold-warm-envelopes-separated")));
  assert!(as_bool(get(receipt, "cold-warm-gap-positive")));
  assert!(as_bool(get(
    receipt,
    "cold-start-attribution-frontier-required"
  )));
  assert!(!as_bool(get(receipt, "cold-start-solved")));
  assert!(!as_bool(get(receipt, "cold-start-eliminated")));
  assert!(!as_bool(get(receipt, "cold-start-attributed")));
}

#[test]
fn contract_records_cold_warm_separation_not_cold_start_solution() {
  let receipt = eval_receipt();
  let contract = get(receipt, "boundary-contract");
  assert_eq!(
    as_str(get(contract, "current-status")),
    "self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-boundary-proof-present"
  );
  assert_eq!(
    as_str(get(contract, "boundary-verdict")),
    "cold-warm-separated-cold-start-attribution-required"
  );
  assert_eq!(as_i64(get(contract, "warm-envelope-min-duration-ms")), 266);
  assert_eq!(as_i64(get(contract, "warm-envelope-max-duration-ms")), 312);
  assert_eq!(as_i64(get(contract, "cold-envelope-min-duration-ms")), 9597);
  assert_eq!(
    as_i64(get(contract, "cold-envelope-max-duration-ms")),
    10544
  );
  assert_eq!(as_i64(get(contract, "cold-warm-gap-min-ms")), 9285);
  assert_eq!(as_i64(get(contract, "cold-warm-gap-max-ms")), 10278);
  assert_eq!(as_i64(get(contract, "slow-threshold-ms")), 5000);
  assert!(as_bool(get(contract, "cold-warm-envelopes-separated")));
  assert!(as_bool(get(contract, "cold-warm-gap-positive")));
  assert!(as_bool(get(
    contract,
    "cold-start-attribution-frontier-required"
  )));
  assert!(as_bool(get(
    contract,
    "closes-cold-start-boundary-frontier"
  )));
  assert!(as_bool(get(
    contract,
    "opens-cold-start-attribution-frontier"
  )));
  assert!(!as_bool(get(contract, "cold-start-solved")));
  assert!(!as_bool(get(contract, "cold-start-eliminated")));
  assert!(!as_bool(get(contract, "cold-start-attributed")));
  assert!(!as_bool(get(contract, "global-speedup-claimed")));
  assert!(!as_bool(get(contract, "whole-system-speedup-claimed")));
  assert!(!as_bool(get(contract, "runtime-install")));
  assert!(!as_bool(get(contract, "self-modification")));
}

#[test]
fn trials_cover_valid_negative_overclaim_and_p_puck_cold_sample() {
  let receipt = eval_receipt();
  let trials = as_list(get(receipt, "boundary-trials"));
  assert_eq!(trials.len(), 25);
  let ids = trials
    .iter()
    .map(|trial| as_str(get(trial, "id")))
    .collect::<BTreeSet<_>>();
  for id in [
    "trial.A.valid-cold-start-boundary-proof",
    "trial.B.cold-warm-envelope",
    "trial.C.cold-warm-separation",
    "trial.D.next-frontier",
    "trial.H.benchmark-source-missing",
    "trial.J.cold-envelope-mismatch",
    "trial.M.separation-mismatch",
    "trial.P.attribution-frontier-missing",
    "trial.T.cold-start-overclaim",
    "trial.U.speedup-overclaim",
    "trial.V.runtime-overclaim",
    "trial.W.external-or-license-overclaim",
    "trial.X.authority-overclaim",
    "trial.Y.p-puck-cold-sample",
  ] {
    assert!(ids.contains(id), "missing {id}");
  }
}

#[test]
fn six_layer_fold_keeps_cold_warm_separation_separate_from_runtime_install() {
  let receipt = eval_receipt();
  let fold = get(receipt, "six-layer-cold-start-boundary-fold");
  assert_eq!(
    as_str(get(fold, "mode")),
    "macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-boundary-proof"
  );
  let semantic = get(fold, "semantic");
  assert!(as_bool(get(semantic, "cold-warm-gap-positive")));
  assert!(as_bool(get(
    semantic,
    "cold-start-attribution-frontier-required"
  )));
  assert!(!as_bool(get(semantic, "cold-start-solved")));
  assert!(!as_bool(get(semantic, "cold-start-eliminated")));
  assert!(!as_bool(get(semantic, "cold-start-attributed")));
  assert!(!as_bool(get(semantic, "global-speedup-claimed")));
  assert!(!as_bool(get(semantic, "whole-system-speedup-claimed")));

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
  let p_puck = get(audit, "p-puck-cold-sample");
  assert_eq!(as_i64(get(p_puck, "duration-ms")), 12411);
  assert_eq!(as_str(get(p_puck, "status")), "slow-path-candidate");
  assert!(!as_bool(get(p_puck, "semantic-owner")));
  assert!(!as_bool(get(p_puck, "cold-start-attributable")));
  assert!(!as_bool(get(p_puck, "cold-start-solved")));
}

#[test]
fn migration_delta_closes_cold_start_boundary_and_opens_cold_start_attribution_only() {
  let receipt = eval_receipt();
  let delta = get(receipt, "migrationDelta");
  let closes = string_set(get(delta, "closes"));
  assert!(closes.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-boundary-proof"
  ));
  let opens = string_set(get(delta, "opens"));
  assert!(opens.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution-proof"
  ));
  let does_not_close = string_set(get(delta, "does-not-close"));
  assert!(does_not_close.contains("need.self.bootstrap-status-audit-cold-start-elimination-proof"));
  assert!(does_not_close.contains("need.self.bootstrap-status-audit-cold-start-attribution-proof"));
  assert!(does_not_close.contains("need.global-speedup-proof"));
  assert!(does_not_close.contains("need.stdlib.meaning-db"));
}

#[test]
fn discoveries_d779_to_d786_are_recorded_and_hard_stops_remain_false() {
  let receipt = eval_receipt();
  let discoveries = as_list(get(receipt, "discoveries"));
  assert_eq!(discoveries.len(), 8);
  let ids = discoveries
    .iter()
    .map(|entry| as_str(get(entry, "id")))
    .collect::<BTreeSet<_>>();
  for id in [
    "D779.cold-start-boundary-consumes-comparable-benchmark",
    "D780.cold-envelope-pinned-from-prior-measurement-cuts",
    "D781.warm-envelope-preserved-from-comparable-benchmark",
    "D782.cold-warm-gap-positive-9285-to-10278-ms",
    "D783.cold-records-stay-above-slow-threshold",
    "D784.cold-start-solution-remains-separate-frontier",
    "D785.cold-start-attribution-is-the-next-frontier",
    "D786.boundary-proof-does-not-own-attribution-or-runtime-install",
  ] {
    assert!(ids.contains(id), "missing {id}");
  }

  for key in [
    "cold-start-solved",
    "cold-start-eliminated",
    "cold-start-attributed",
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
    assert!(!as_bool(get(receipt, key)), "`{key}` must stay false");
  }
}
