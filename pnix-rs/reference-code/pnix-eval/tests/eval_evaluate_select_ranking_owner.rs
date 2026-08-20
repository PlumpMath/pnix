use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/evaluate-select-ranking-owner.px")
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

fn as_i64(v: &Value) -> i64 {
  match v {
    Value::Int(n) => *n,
    other => panic!("expected int, got {:?}", other),
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
fn ranking_owner_fixture_imports_px_owner() {
  let run = eval_file(&fixture_path()).expect("evaluate/select ranking owner fixture must eval");
  assert_eq!(as_str(get(&run, "proof")), "evaluate-select-ranking-owner");
  assert!(as_bool(get(&run, "imported-owner")));
  assert!(as_bool(get(&run, "used-px-owner")));
  assert_eq!(as_i64(get(&run, "candidate-count")), 3);
  assert!(!as_bool(get(&run, "runtime-install")));
  assert!(!as_bool(get(&run, "ranking-runtime-install")));
  assert!(!as_bool(get(&run, "implementation-command")));
}

#[test]
fn owner_meta_declares_non_installed_eval_select_ranking_law() {
  let run = eval_file(&fixture_path()).unwrap();
  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.lib.gate.evaluate-select-ranking"
  );
  assert_eq!(
    as_str(get(meta, "surface-pair")),
    "macro-native.evaluate-select.surface-pair-owner"
  );
  assert_eq!(as_str(get(meta, "constructor")), "selectWinner");
  assert_eq!(as_str(get(meta, "ranking-constructor")), "rankCandidates");
  assert_eq!(
    as_str(get(meta, "selection-order")),
    "score-safety-replayability-evidence-loss-cost-candidate-id"
  );
  assert_eq!(as_str(get(meta, "empty-candidate-set")), "Held");
  assert_eq!(as_str(get(meta, "missing-axis")), "Held");
  assert_eq!(as_str(get(meta, "hidden-tie-break")), "Held");
  assert!(!as_bool(get(meta, "runtime-install")));
}

#[test]
fn required_axes_are_explicit_and_stable() {
  let run = eval_file(&fixture_path()).unwrap();
  let axes = string_set(get(&run, "required-axes"));
  for expected in [
    "score",
    "safety",
    "replayability",
    "evidence",
    "loss",
    "cost",
  ] {
    assert!(axes.contains(expected), "missing axis `{expected}`");
  }
  assert_eq!(axes.len(), 6);
}

#[test]
fn ranking_uses_score_then_safety_then_remaining_axes_then_id() {
  let run = eval_file(&fixture_path()).unwrap();
  let ranked = as_list(get(&run, "ranked"));
  assert_eq!(ranked.len(), 3);
  assert_eq!(as_str(get(&ranked[0], "candidate-id")), "candidate.beta");
  assert_eq!(as_str(get(&ranked[1], "candidate-id")), "candidate.alpha");
  assert_eq!(as_str(get(&ranked[2], "candidate-id")), "candidate.gamma");
}

#[test]
fn select_winner_emits_ranked_status_without_runtime_install() {
  let run = eval_file(&fixture_path()).unwrap();
  let selected = get(&run, "selected");
  assert_eq!(as_str(get(selected, "status")), "ranked");
  assert_eq!(
    as_str(get(selected, "winner-candidate-id")),
    "candidate.beta"
  );
  assert_eq!(
    as_str(get(selected, "winner-reason")),
    "score>safety>replayability>evidence>loss>cost>candidate-id"
  );
  assert_eq!(
    as_str(get(selected, "tie-break-ref")),
    "tie-break.lexical-candidate-id.v1"
  );
  assert!(!as_bool(get(selected, "runtime-install")));
  assert!(!as_bool(get(selected, "ranking-runtime-install")));
  assert!(!as_bool(get(selected, "global-ranking-runtime")));
  assert!(!as_bool(get(selected, "rigorfloor-authority")));
  assert!(!as_bool(get(selected, "route-cache-authority")));
}

#[test]
fn deterministic_tie_break_uses_candidate_id_when_axes_equal() {
  let run = eval_file(&fixture_path()).unwrap();
  let selected = get(&run, "tie-selected");
  assert_eq!(as_str(get(selected, "status")), "ranked");
  assert_eq!(as_str(get(selected, "winner-candidate-id")), "candidate.a");
  let ranking = as_list(get(selected, "ranking"));
  assert_eq!(as_str(get(&ranking[0], "candidate-id")), "candidate.a");
  assert_eq!(as_str(get(&ranking[1], "candidate-id")), "candidate.b");
}

#[test]
fn empty_candidate_set_is_held_not_default_winner() {
  let run = eval_file(&fixture_path()).unwrap();
  let held = get(&run, "empty-selected");
  assert_eq!(as_str(get(held, "status")), "Held");
  assert_eq!(
    as_str(get(held, "held-id")),
    "held.evaluate-select-ranking.empty-candidate-set"
  );
  assert_eq!(as_list(get(held, "ranking")).len(), 0);
  assert!(matches!(get(held, "winner-candidate-id"), Value::Null));
  assert!(!as_bool(get(held, "runtime-install")));
}

#[test]
fn missing_axis_is_held_with_diagnostic_reason() {
  let run = eval_file(&fixture_path()).unwrap();
  let held = get(&run, "missing-axis-selected");
  assert_eq!(as_str(get(held, "status")), "Held");
  assert_eq!(
    as_str(get(held, "held-id")),
    "held.evaluate-select-ranking.missing-required-evidence"
  );
  let missing = string_set(get(held, "missing"));
  assert!(missing.contains("axis-evidence-missing:candidate.missing"));
  assert!(!as_bool(get(held, "runtime-install")));
}

#[test]
fn missing_tie_break_ref_is_held_not_hidden_route_cache() {
  let run = eval_file(&fixture_path()).unwrap();
  let held = get(&run, "no-tie-break-selected");
  assert_eq!(as_str(get(held, "status")), "Held");
  assert_eq!(
    as_str(get(held, "held-id")),
    "held.evaluate-select-ranking.tie-break-ref-missing"
  );
  let missing = string_set(get(held, "missing"));
  assert!(missing.contains("tie-break-ref"));
  assert!(!as_bool(get(held, "route-cache-authority")));
}

#[test]
fn missing_provenance_is_held_before_ranking() {
  let run = eval_file(&fixture_path()).unwrap();
  let held = get(&run, "no-provenance-selected");
  assert_eq!(as_str(get(held, "status")), "Held");
  assert_eq!(
    as_str(get(held, "held-id")),
    "held.evaluate-select-ranking.missing-required-evidence"
  );
  let missing = string_set(get(held, "missing"));
  assert!(missing.contains("candidate-provenance-ref-missing:candidate.no-provenance"));
}

#[test]
fn held_outputs_preserve_no_runtime_authority_flags() {
  let run = eval_file(&fixture_path()).unwrap();
  for key in [
    "empty-selected",
    "missing-axis-selected",
    "no-tie-break-selected",
    "no-provenance-selected",
  ] {
    let held = get(&run, key);
    assert_eq!(as_str(get(held, "status")), "Held");
    assert!(
      !as_bool(get(held, "runtime-install")),
      "`{key}` installed runtime"
    );
    assert!(
      !as_bool(get(held, "ranking-runtime-install")),
      "`{key}` installed ranking runtime"
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
