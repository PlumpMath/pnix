use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn receipt_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/tesseract-macro-legacy-probe/macro_only_self_bootstrap_status_audit_shallow_summary_fast_path_global_default_replacement_cold_start_attribution_proof_receipt.px",
  )
}

fn eval_receipt() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = receipt_path();
    let json = std::thread::Builder::new()
      .name("tesseract-global-default-cold-start-attribution-receipt-eval".to_string())
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("global default replacement cold-start attribution proof receipt")
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
fn receipt_exposes_probe_marker_and_positive_attribution_contract() {
  let receipt = eval_receipt();
  assert_eq!(
    as_str(get(receipt, "probe-marker")),
    "tesseract-macro-ontology-macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution-proof"
  );
  assert!(as_bool(get(
    receipt,
    "global-default-replacement-cold-start-attribution-proof"
  )));
  assert!(as_bool(get(receipt, "wrapper-attribution-proven")));
  assert!(as_bool(get(
    receipt,
    "core-eval-attribution-candidate-only"
  )));
  assert!(as_bool(get(receipt, "unknown-attribution-candidate-only")));
  assert!(as_bool(get(
    receipt,
    "attribution-policy-frontier-required"
  )));
  assert!(!as_bool(get(receipt, "cold-start-solved")));
  assert!(!as_bool(get(receipt, "cold-start-eliminated")));
  assert!(!as_bool(get(receipt, "wrapper-bypass-applied")));
}

#[test]
fn contract_records_wrapper_proven_others_candidate_only() {
  let receipt = eval_receipt();
  let contract = get(receipt, "attribution-contract");
  assert_eq!(
    as_str(get(contract, "current-status")),
    "self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution-proof-present"
  );
  assert_eq!(
    as_str(get(contract, "attribution-verdict")),
    "wrapper-attribution-proven-core-eval-and-unknown-attribution-candidates-only"
  );
  assert_eq!(as_i64(get(contract, "wrapper-attributable-min-ms")), 11059);
  assert_eq!(as_i64(get(contract, "wrapper-attributable-max-ms")), 11170);
  assert_eq!(as_i64(get(contract, "cold-warm-gap-min-ms")), 9285);
  assert_eq!(as_i64(get(contract, "cold-warm-gap-max-ms")), 10278);
  assert_eq!(as_i64(get(contract, "attribution-record-count")), 3);
  assert!(as_bool(get(contract, "wrapper-attribution-proven")));
  assert!(as_bool(get(
    contract,
    "core-eval-attribution-candidate-only"
  )));
  assert!(as_bool(get(contract, "unknown-attribution-candidate-only")));
  assert!(as_bool(get(
    contract,
    "attribution-policy-frontier-required"
  )));
  assert!(as_bool(get(
    contract,
    "closes-cold-start-attribution-frontier"
  )));
  assert!(as_bool(get(
    contract,
    "opens-cold-start-attribution-policy-frontier"
  )));
  assert!(!as_bool(get(contract, "cold-start-solved")));
  assert!(!as_bool(get(contract, "cold-start-eliminated")));
  assert!(!as_bool(get(
    contract,
    "cold-start-attributed-to-undocumented-cause"
  )));
  assert!(!as_bool(get(contract, "wrapper-bypass-applied")));
  assert!(!as_bool(get(contract, "global-speedup-claimed")));
  assert!(!as_bool(get(contract, "runtime-install")));
  assert!(!as_bool(get(contract, "self-modification")));
}

#[test]
fn trials_cover_valid_negative_overclaim_and_p_puck_cold_sample() {
  let receipt = eval_receipt();
  let trials = as_list(get(receipt, "attribution-trials"));
  assert_eq!(trials.len(), 25);
  let ids = trials
    .iter()
    .map(|trial| as_str(get(trial, "id")))
    .collect::<BTreeSet<_>>();
  for id in [
    "trial.A.valid-cold-start-attribution-proof",
    "trial.B.attribution-record-set",
    "trial.C.wrapper-attributable-share",
    "trial.D.next-frontier",
    "trial.H.boundary-source-missing",
    "trial.I.wrapper-evidence-missing",
    "trial.J.wrapper-share-mismatch",
    "trial.K.attribution-record-shape-mismatch",
    "trial.M.attribution-status-mismatch",
    "trial.P.policy-frontier-missing",
    "trial.T.attribution-overclaim",
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
fn six_layer_fold_keeps_attribution_separate_from_runtime_install() {
  let receipt = eval_receipt();
  let fold = get(receipt, "six-layer-cold-start-attribution-fold");
  assert_eq!(
    as_str(get(fold, "mode")),
    "macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution-proof"
  );
  let semantic = get(fold, "semantic");
  assert!(as_bool(get(semantic, "wrapper-attribution-proven")));
  assert!(as_bool(get(
    semantic,
    "core-eval-attribution-candidate-only"
  )));
  assert!(as_bool(get(semantic, "unknown-attribution-candidate-only")));
  assert!(as_bool(get(
    semantic,
    "attribution-policy-frontier-required"
  )));
  assert!(!as_bool(get(semantic, "cold-start-solved")));
  assert!(!as_bool(get(semantic, "cold-start-eliminated")));
  assert!(!as_bool(get(semantic, "wrapper-bypass-applied")));
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

  let audit = get(fold, "audit");
  let p_puck = get(audit, "p-puck-cold-sample");
  assert_eq!(as_i64(get(p_puck, "duration-ms")), 7267);
  assert_eq!(as_str(get(p_puck, "status")), "slow-path-candidate");
  assert!(!as_bool(get(p_puck, "semantic-owner")));
  assert!(!as_bool(get(p_puck, "cold-start-attributable")));
  assert!(!as_bool(get(p_puck, "cold-start-solved")));
}

#[test]
fn migration_delta_closes_cold_start_attribution_and_opens_attribution_policy_only() {
  let receipt = eval_receipt();
  let delta = get(receipt, "migrationDelta");
  let closes = string_set(get(delta, "closes"));
  assert!(closes.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution-proof"
  ));
  let opens = string_set(get(delta, "opens"));
  assert!(opens.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution-policy-proof"
  ));
  let does_not_close = string_set(get(delta, "does-not-close"));
  assert!(does_not_close.contains("need.self.bootstrap-status-audit-cold-start-elimination-proof"));
  assert!(does_not_close.contains("need.self.bootstrap-status-audit-wrapper-bypass-proof"));
  assert!(does_not_close.contains("need.global-speedup-proof"));
  assert!(does_not_close.contains("need.stdlib.meaning-db"));
}

#[test]
fn discoveries_d787_to_d794_are_recorded_and_hard_stops_remain_false() {
  let receipt = eval_receipt();
  let discoveries = as_list(get(receipt, "discoveries"));
  assert_eq!(discoveries.len(), 8);
  let ids = discoveries
    .iter()
    .map(|entry| as_str(get(entry, "id")))
    .collect::<BTreeSet<_>>();
  for id in [
    "D787.cold-start-attribution-consumes-cold-start-boundary",
    "D788.attribution-splits-cold-cause-into-three-candidates",
    "D789.wrapper-attribution-proven-via-prior-wrapper-repeat-proof",
    "D790.core-eval-attribution-stays-candidate-only",
    "D791.unknown-attribution-stays-catch-all-candidate",
    "D792.attribution-does-not-equal-cold-start-solution-or-elimination",
    "D793.next-frontier-is-cold-start-attribution-policy-proof",
    "D794.attribution-proof-does-not-grant-runtime-install-or-self-modification",
  ] {
    assert!(ids.contains(id), "missing {id}");
  }

  for key in [
    "cold-start-solved",
    "cold-start-eliminated",
    "cold-start-attributed-to-undocumented-cause",
    "wrapper-bypass-applied",
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
