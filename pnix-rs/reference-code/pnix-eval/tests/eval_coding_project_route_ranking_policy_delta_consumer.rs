use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join(
    "../../fixtures/pnix-query-runtime/coding-project-route-ranking-policy-delta-consumer-receipt.px",
  )
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
    Value::Int(i) => *i,
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

fn find_route<'a>(routes: &'a [Value], route_id: &str) -> &'a Value {
  routes
    .iter()
    .find(|route| as_str(get(route, "route_id")) == route_id)
    .unwrap_or_else(|| panic!("missing route `{}`", route_id))
}

#[test]
fn fixture_evaluates_with_pnix_eval_not_nix() {
  let run = eval_file(&fixture_path())
    .expect("coding project route ranking policy delta consumer fixture must evaluate");
  assert_eq!(
    as_str(get(&run, "proof")),
    "coding-project-route-ranking-policy-delta-consumer"
  );

  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "puncheetah.contract.coding-project-route-ranking-policy-delta-consumer.v0"
  );
  assert_eq!(
    as_str(get(meta, "ranking_backend")),
    "stdlib.agent.route-ranking-mirror"
  );
}

#[test]
fn policy_delta_changes_ranked_route_without_persistence_or_execution() {
  let run = eval_file(&fixture_path()).unwrap();
  let passed = get(&run, "passed");

  assert_eq!(
    as_str(get(passed, "schema")),
    "puncheetah.code.route-ranking-policy-delta-consumer.v0"
  );
  assert_eq!(
    as_str(get(passed, "outcome")),
    "coding-project-route-ranking-policy-delta-consumed"
  );
  assert!(as_bool(get(passed, "verified")));
  assert!(as_bool(get(passed, "route_policy_delta_consumed")));
  assert!(as_bool(get(passed, "route_stats_loaded")));
  assert!(as_bool(get(passed, "route_candidates_ranked")));
  assert!(as_bool(get(passed, "ranking_changed")));
  assert_eq!(
    as_str(get(passed, "baseline_selected_route")),
    "generic-api-route"
  );
  assert_eq!(
    as_str(get(passed, "selected_route")),
    "generic-search-before-retry-route"
  );
  assert_eq!(
    as_str(get(passed, "next_gate")),
    "coding-project-route-selection-plan"
  );

  let ranked = as_list(get(passed, "ranked_routes"));
  assert_eq!(
    as_str(get(&ranked[0], "route_id")),
    "generic-search-before-retry-route"
  );
  assert!(as_bool(get(&ranked[0], "policy_hint_matched")));

  let baseline = as_list(get(passed, "baseline_ranked_routes"));
  assert_eq!(as_str(get(&baseline[0], "route_id")), "generic-api-route");

  let ranking_input = as_list(get(passed, "ranking_input_routes"));
  let api = find_route(ranking_input, "generic-api-route");
  assert_eq!(as_i64(get(api, "attempts")), 2);
  assert_eq!(as_i64(get(api, "successes")), 1);
  assert!(as_bool(get(api, "policy_delta_applied")));

  let search = find_route(ranking_input, "generic-search-before-retry-route");
  assert_eq!(as_i64(get(search, "prior_weight")), 5);
  assert!(as_bool(get(search, "policy_hint_matched")));

  assert!(!as_bool(get(passed, "route_execution_allowed")));
  assert!(!as_bool(get(passed, "memory_write_allowed")));
  assert!(!as_bool(get(passed, "policy_persistence_allowed")));
  assert!(!as_bool(get(passed, "search_execution_allowed")));
  assert!(!as_bool(get(passed, "file_write_allowed")));
  assert!(!as_bool(get(passed, "host_execution_allowed")));

  let receipt = get(passed, "receipt");
  assert_eq!(
    as_str(get(receipt, "invariant")),
    "route_policy_delta is consumed as ranking input only; persistence and execution require later gates"
  );
}

#[test]
fn reasoning_dispatch_can_consume_route_policy_delta() {
  let run = eval_file(&fixture_path()).unwrap();
  let dispatched = get(&run, "dispatched");
  assert_eq!(as_str(get(dispatched, "op")), "consume-route-policy-delta");
  let result = get(dispatched, "result");
  assert_eq!(
    as_str(get(result, "outcome")),
    "coding-project-route-ranking-policy-delta-consumed"
  );
  assert_eq!(
    as_str(get(result, "selected_route")),
    "generic-search-before-retry-route"
  );
  assert!(!as_bool(get(result, "memory_write_allowed")));
  assert!(!as_bool(get(result, "policy_persistence_allowed")));
}

#[test]
fn invalid_delta_stats_and_effect_requests_are_held() {
  let run = eval_file(&fixture_path()).unwrap();

  let missing_delta = get(&run, "missing-delta");
  assert!(as_bool(get(missing_delta, "is_held")));
  assert_eq!(
    as_str(get(missing_delta, "outcome")),
    "held-route-ranking-policy-delta-required"
  );

  let missing_stats = get(&run, "missing-stats");
  assert!(as_bool(get(missing_stats, "is_held")));
  assert_eq!(
    as_str(get(missing_stats, "outcome")),
    "held-route-ranking-route-stats-required"
  );

  let selected_missing = get(&run, "selected-missing");
  assert!(as_bool(get(selected_missing, "is_held")));
  assert_eq!(
    as_str(get(selected_missing, "outcome")),
    "held-route-ranking-selected-route-missing"
  );

  let invalid_stats = get(&run, "invalid-stats");
  assert!(as_bool(get(invalid_stats, "is_held")));
  assert_eq!(
    as_str(get(invalid_stats, "outcome")),
    "held-route-ranking-invalid-route-stats"
  );

  let policy = get(&run, "policy-persistence-held");
  assert!(as_bool(get(policy, "is_held")));
  assert_eq!(
    as_str(get(policy, "outcome")),
    "held-route-ranking-policy-delta-required"
  );
  assert!(!as_bool(get(policy, "policy_persistence_allowed")));

  let memory = get(&run, "memory-write-held");
  assert!(as_bool(get(memory, "is_held")));
  assert_eq!(
    as_str(get(memory, "outcome")),
    "held-route-ranking-memory-write-blocked"
  );
  assert!(!as_bool(get(memory, "memory_write_allowed")));

  let search = get(&run, "search-execution-held");
  assert!(as_bool(get(search, "is_held")));
  assert_eq!(
    as_str(get(search, "outcome")),
    "held-route-ranking-search-execution-blocked"
  );
  assert!(!as_bool(get(search, "search_execution_allowed")));
}
