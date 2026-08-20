use pnix_eval::eval_to_json;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

// Combined-file pattern: this receipt test loads the same combined .px and
// navigates to `.receipt`.

fn combined_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../stdlib/lib/gate/macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution-policy-proof.px",
  )
}

fn eval_combined() -> &'static Value {
  static VALUE: OnceLock<Value> = OnceLock::new();
  VALUE.get_or_init(|| {
    let path = combined_path();
    let json = std::thread::Builder::new()
      .name("cold-start-attribution-policy-receipt-eval".to_string())
      .stack_size(32 * 1024 * 1024)
      .spawn(move || {
        eval_to_json(path.to_str().expect("utf-8 path"), true)
          .expect("cold-start attribution policy receipt eval")
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
fn receipt_exposes_probe_marker_and_constitutional_helpers_reference() {
  let r = receipt();
  assert_eq!(
    as_str(get(r, "probe-marker")),
    "tesseract-macro-ontology-macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution-policy-proof"
  );
  assert_eq!(
    as_str(get(r, "proof-helpers-owner")),
    "stdlib/lib/gate/macro-only-proof-helpers.px"
  );
  assert!(as_bool(get(
    r,
    "global-default-replacement-cold-start-attribution-policy-proof"
  )));
  assert!(as_bool(get(r, "wrapper-elimination-candidate-eligible")));
  assert!(as_bool(get(r, "core-eval-measurement-required")));
  assert!(as_bool(get(r, "unknown-deferred-until-residual")));
  assert!(!as_bool(get(r, "cold-start-solved")));
  assert!(!as_bool(get(r, "cold-start-eliminated")));
  assert!(!as_bool(get(r, "wrapper-bypass-applied")));
  assert!(!as_bool(get(r, "elimination-applied")));
}

#[test]
fn contract_records_eligibility_split_not_remediation() {
  let r = receipt();
  let contract = get(r, "policy-contract");
  assert_eq!(
    as_str(get(contract, "current-status")),
    "self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution-policy-proof-present"
  );
  assert_eq!(
    as_str(get(contract, "policy-verdict")),
    "wrapper-elimination-candidate-eligible-others-deferred-or-measurement-required"
  );
  assert_eq!(as_i64(get(contract, "policy-candidate-count")), 3);
  assert!(as_bool(get(
    contract,
    "wrapper-elimination-candidate-eligible"
  )));
  assert!(as_bool(get(contract, "core-eval-measurement-required")));
  assert!(as_bool(get(contract, "unknown-deferred-until-residual")));
  assert!(as_bool(get(
    contract,
    "elimination-candidate-frontier-required"
  )));
  assert!(as_bool(get(
    contract,
    "closes-cold-start-attribution-policy-frontier"
  )));
  assert!(as_bool(get(
    contract,
    "opens-cold-start-elimination-candidate-frontier"
  )));
  assert!(!as_bool(get(contract, "cold-start-solved")));
  assert!(!as_bool(get(contract, "cold-start-eliminated")));
  assert!(!as_bool(get(contract, "wrapper-bypass-applied")));
  assert!(!as_bool(get(contract, "elimination-applied")));
  assert!(!as_bool(get(contract, "global-speedup-claimed")));
  assert!(!as_bool(get(contract, "runtime-install")));
  assert!(!as_bool(get(contract, "self-modification")));
}

#[test]
fn trials_cover_valid_negative_overclaim_and_combined_file_marker() {
  let r = receipt();
  let trials = as_list(get(r, "policy-trials"));
  assert_eq!(trials.len(), 23);
  let ids = trials
    .iter()
    .map(|trial| as_str(get(trial, "id")))
    .collect::<BTreeSet<_>>();
  for id in [
    "trial.A.valid-cold-start-attribution-policy-proof",
    "trial.B.policy-candidate-set",
    "trial.C.next-frontier",
    "trial.G.attribution-source-missing",
    "trial.H.attribution-input-shape-mismatch",
    "trial.K.eligibility-mismatch",
    "trial.N.elimination-frontier-missing",
    "trial.R.policy-overclaim",
    "trial.S.speedup-overclaim",
    "trial.T.runtime-overclaim",
    "trial.U.external-or-license-overclaim",
    "trial.V.authority-overclaim",
    "trial.W.combined-file-emits-three-named-fields",
  ] {
    assert!(ids.contains(id), "missing {id}");
  }
}

#[test]
fn six_layer_fold_marks_constitutional_combined_file_pattern() {
  let r = receipt();
  let fold = get(r, "six-layer-cold-start-attribution-policy-fold");
  assert_eq!(
    as_str(get(fold, "mode")),
    "macro-only-self-bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution-policy-proof"
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
  assert!(as_bool(get(
    semantic,
    "wrapper-elimination-candidate-eligible"
  )));
  assert!(as_bool(get(semantic, "core-eval-measurement-required")));
  assert!(as_bool(get(semantic, "unknown-deferred-until-residual")));
  assert!(!as_bool(get(semantic, "cold-start-solved")));
  assert!(!as_bool(get(semantic, "cold-start-eliminated")));
  assert!(!as_bool(get(semantic, "wrapper-bypass-applied")));
  assert!(!as_bool(get(semantic, "elimination-applied")));

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
fn migration_delta_closes_attribution_policy_and_opens_elimination_candidate_only() {
  let r = receipt();
  let delta = get(r, "migrationDelta");
  let closes = string_set(get(delta, "closes"));
  assert!(closes.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-attribution-policy-proof"
  ));
  let opens = string_set(get(delta, "opens"));
  assert!(opens.contains(
    "need.self.bootstrap-status-audit-shallow-summary-fast-path-global-default-replacement-cold-start-elimination-candidate-proof"
  ));
  let does_not_close = string_set(get(delta, "does-not-close"));
  assert!(does_not_close.contains("need.self.bootstrap-status-audit-cold-start-elimination-proof"));
  assert!(does_not_close.contains("need.self.bootstrap-status-audit-wrapper-bypass-proof"));
  assert!(does_not_close.contains("need.global-speedup-proof"));
}

#[test]
fn discoveries_d795_to_d802_recorded_with_constitutional_baseline() {
  let r = receipt();
  let discoveries = as_list(get(r, "discoveries"));
  assert_eq!(discoveries.len(), 8);
  let ids = discoveries
    .iter()
    .map(|entry| as_str(get(entry, "id")))
    .collect::<BTreeSet<_>>();
  for id in [
    "D795.cold-start-attribution-policy-consumes-cold-start-attribution",
    "D796.policy-emits-one-eligibility-per-attribution-candidate",
    "D797.wrapper-policy-status-is-elimination-candidate-eligible",
    "D798.core-eval-policy-status-is-measurement-required",
    "D799.unknown-policy-status-is-deferred-until-residual-recorded",
    "D800.policy-does-not-equal-elimination-or-wrapper-bypass",
    "D801.next-frontier-is-cold-start-elimination-candidate-proof",
    "D802.constitutional-referential-transparency-baseline",
  ] {
    assert!(ids.contains(id), "missing {id}");
  }

  for key in [
    "cold-start-solved",
    "cold-start-eliminated",
    "wrapper-bypass-applied",
    "elimination-applied",
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
