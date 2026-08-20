use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/evaluate-select-route-adapter-owner.px")
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
fn route_adapter_fixture_imports_owner_and_route_binding_receipt() {
  let run = eval_file(&fixture_path()).expect("evaluate/select route adapter fixture must eval");
  assert_eq!(
    as_str(get(&run, "proof")),
    "evaluate-select-route-adapter-owner"
  );
  assert!(as_bool(get(&run, "imported-owner")));
  assert!(as_bool(get(&run, "used-px-owner")));
  assert!(as_bool(get(&run, "route-binding-receipt-present")));
  assert!(as_bool(get(&run, "route-bound-before-adapter")));
  assert_eq!(
    as_str(get(&run, "route-binding-verdict")),
    "surface-pair-executable-route-bound-non-installed"
  );
  assert!(!as_bool(get(&run, "runtime-install")));
  assert!(!as_bool(get(&run, "runtime-adapter-install")));
}

#[test]
fn owner_meta_declares_callable_non_installed_route_adapter() {
  let run = eval_file(&fixture_path()).unwrap();
  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.lib.gate.evaluate-select-route-adapter"
  );
  assert_eq!(
    as_str(get(meta, "surface-pair")),
    "macro-native.evaluate-select.surface-pair-owner"
  );
  assert_eq!(as_str(get(meta, "constructor")), "routeEvaluateSelect");
  assert_eq!(
    as_str(get(meta, "delegated-owner")),
    "stdlib.lib.gate.evaluate-select-ranking"
  );
  assert_eq!(as_str(get(meta, "delegated-constructor")), "selectWinner");
  assert_eq!(
    as_str(get(meta, "route-id")),
    "route.binding.evaluate-select.surface-pair"
  );
  assert_eq!(
    as_str(get(meta, "effect-scope")),
    "legacy-evaluate-select-surface-pair-only"
  );
  assert!(!as_bool(get(meta, "runtime-install")));
  assert!(!as_bool(get(meta, "runtime-adapter-install")));
  assert!(!as_bool(get(meta, "global-ranking-runtime")));
}

#[test]
fn successful_route_call_delegates_to_ranking_owner_and_preserves_route_metadata() {
  let run = eval_file(&fixture_path()).unwrap();
  let selected = get(&run, "selected");
  assert_eq!(as_str(get(selected, "status")), "route-ranked");
  assert_eq!(as_str(get(selected, "route-status")), "ranked");
  assert_eq!(as_str(get(selected, "source-status")), "ranked");
  assert_eq!(
    as_str(get(selected, "winner-candidate-id")),
    "candidate.beta"
  );
  assert_eq!(
    as_str(get(selected, "route-id")),
    "route.binding.evaluate-select.surface-pair"
  );
  assert_eq!(
    as_str(get(selected, "effect-scope")),
    "legacy-evaluate-select-surface-pair-only"
  );
  assert_eq!(
    as_str(get(selected, "caller-scope")),
    "legacy-evaluate-select-surface-pair-only"
  );
  assert_eq!(
    as_str(get(selected, "audit-ref")),
    "audit.evaluate-select-route-adapter.route"
  );
  assert_eq!(
    as_str(get(selected, "route-adapter-owner")),
    "stdlib.lib.gate.evaluate-select-route-adapter"
  );
  assert_eq!(
    as_str(get(selected, "ranking-owner")),
    "stdlib.lib.gate.evaluate-select-ranking"
  );
  assert_eq!(as_list(get(selected, "ranking")).len(), 3);
}

#[test]
fn successful_route_call_is_callable_but_not_runtime_install() {
  let run = eval_file(&fixture_path()).unwrap();
  let selected = get(&run, "selected");
  assert!(as_bool(get(selected, "route-bound")));
  assert!(as_bool(get(selected, "adapter-callable")));
  assert!(!as_bool(get(selected, "runtime-install")));
  assert!(!as_bool(get(selected, "ranking-runtime-install")));
  assert!(!as_bool(get(selected, "runtime-adapter-install")));
  assert!(!as_bool(get(selected, "global-ranking-runtime")));
  assert!(!as_bool(get(selected, "rigorfloor-authority")));
  assert!(!as_bool(get(selected, "route-cache-authority")));
  assert!(!as_bool(get(selected, "old-evaluate-select-wrapper")));
  assert!(!as_bool(get(selected, "split-evaluate-select-owner")));
  assert!(!as_bool(get(selected, "nix-checks-gate-added")));
}

#[test]
fn ranking_owner_held_cases_are_preserved_by_route_adapter() {
  let run = eval_file(&fixture_path()).unwrap();
  for (key, source_id) in [
    (
      "empty-selected",
      "held.evaluate-select-ranking.empty-candidate-set",
    ),
    (
      "missing-axis-selected",
      "held.evaluate-select-ranking.missing-required-evidence",
    ),
    (
      "no-tie-break-selected",
      "held.evaluate-select-ranking.tie-break-ref-missing",
    ),
    (
      "no-provenance-selected",
      "held.evaluate-select-ranking.missing-required-evidence",
    ),
  ] {
    let held = get(&run, key);
    assert_eq!(as_str(get(held, "status")), "Held");
    assert_eq!(as_str(get(held, "route-status")), "Held");
    assert_eq!(
      as_str(get(held, "held-id")),
      "held.evaluate-select-route-adapter.ranking-owner-held"
    );
    assert_eq!(as_str(get(held, "source-held-id")), source_id);
    assert!(
      string_set(get(held, "missing")).contains(format!("ranking-owner-held:{source_id}").as_str())
    );
    assert!(!as_bool(get(held, "runtime-install")));
    assert!(!as_bool(get(held, "runtime-adapter-install")));
  }
}

#[test]
fn wrong_route_id_is_held_before_ranking_authority_expands() {
  let run = eval_file(&fixture_path()).unwrap();
  let held = get(&run, "wrong-route-selected");
  assert_eq!(as_str(get(held, "status")), "Held");
  assert_eq!(
    as_str(get(held, "held-id")),
    "held.evaluate-select-route-adapter.route-id-mismatch"
  );
  assert_eq!(
    as_str(get(held, "route-id")),
    "route.binding.global-ranking"
  );
  let missing = string_set(get(held, "missing"));
  assert!(missing.contains("expected-route-id:route.binding.evaluate-select.surface-pair"));
  assert!(!as_bool(get(held, "route-bound")));
}

#[test]
fn wrong_effect_scope_is_held_before_global_runtime_claim() {
  let run = eval_file(&fixture_path()).unwrap();
  let held = get(&run, "wrong-effect-scope-selected");
  assert_eq!(as_str(get(held, "status")), "Held");
  assert_eq!(
    as_str(get(held, "held-id")),
    "held.evaluate-select-route-adapter.effect-scope-mismatch"
  );
  assert_eq!(as_str(get(held, "effect-scope")), "global-ranking-runtime");
  let missing = string_set(get(held, "missing"));
  assert!(missing.contains("expected-effect-scope:legacy-evaluate-select-surface-pair-only"));
  assert!(!as_bool(get(held, "global-ranking-runtime")));
}

#[test]
fn wrong_caller_scope_is_held_before_cross_surface_reuse() {
  let run = eval_file(&fixture_path()).unwrap();
  let held = get(&run, "wrong-caller-scope-selected");
  assert_eq!(as_str(get(held, "status")), "Held");
  assert_eq!(
    as_str(get(held, "held-id")),
    "held.evaluate-select-route-adapter.caller-scope-mismatch"
  );
  assert_eq!(as_str(get(held, "caller-scope")), "legacy-ontology.promote");
  let missing = string_set(get(held, "missing"));
  assert!(missing.contains("expected-caller-scope:legacy-evaluate-select-surface-pair-only"));
  assert!(!as_bool(get(held, "runtime-install")));
}

#[test]
fn held_outputs_carry_no_global_runtime_or_store_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  for key in [
    "empty-selected",
    "missing-axis-selected",
    "no-tie-break-selected",
    "no-provenance-selected",
    "wrong-route-selected",
    "wrong-effect-scope-selected",
    "wrong-caller-scope-selected",
  ] {
    let held = get(&run, key);
    assert_eq!(as_str(get(held, "status")), "Held");
    assert!(
      !as_bool(get(held, "runtime-install")),
      "`{key}` installed runtime"
    );
    assert!(
      !as_bool(get(held, "runtime-adapter-install")),
      "`{key}` installed adapter"
    );
    assert!(
      !as_bool(get(held, "global-ranking-runtime")),
      "`{key}` claimed global runtime"
    );
    assert!(
      !as_bool(get(held, "rigorfloor-authority")),
      "`{key}` claimed RigorFloor"
    );
    assert!(
      !as_bool(get(held, "route-cache-authority")),
      "`{key}` claimed route cache"
    );
  }
}

#[test]
fn top_level_state_records_adapter_owner_without_install() {
  let run = eval_file(&fixture_path()).unwrap();
  assert!(as_bool(get(&run, "adapter-callable")));
  assert!(!as_bool(get(&run, "runtime-install")));
  assert!(!as_bool(get(&run, "ranking-runtime-install")));
  assert!(!as_bool(get(&run, "runtime-adapter-install")));
  assert!(!as_bool(get(&run, "global-ranking-runtime")));
  assert!(!as_bool(get(&run, "implementation-command")));
}
