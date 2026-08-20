use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/mirror-learning-loop-receipt.px")
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
  let run = eval_file(&fixture_path()).expect("mirror learning loop fixture must evaluate");
  assert_eq!(as_str(get(&run, "proof")), "mirror-learning-loop");

  assert_eq!(
    as_str(get(get(&run, "learning-owner-meta"), "owner")),
    "stdlib.agent.mirror-learning-observation"
  );
  assert_eq!(
    as_str(get(get(&run, "search-owner-meta"), "owner")),
    "stdlib.agent.search-need-plan"
  );
  assert_eq!(
    as_str(get(get(&run, "route-policy-owner-meta"), "owner")),
    "stdlib.agent.route-policy-update"
  );
}

#[test]
fn success_and_missing_knowledge_become_learning_observations() {
  let run = eval_file(&fixture_path()).unwrap();

  let success = get(&run, "success-learning");
  assert_eq!(
    as_str(get(success, "schema")),
    "puncheetah.mirror.learning-observation.v0"
  );
  assert_eq!(
    as_str(get(success, "outcome")),
    "mirror-learning-observation-built"
  );
  assert!(as_bool(get(success, "verified")));
  assert_eq!(as_str(get(success, "learning_status")), "success");
  assert_eq!(as_str(get(success, "failure_kind")), "none");
  assert!(!as_bool(get(success, "search_needed")));
  assert_eq!(
    as_str(get(success, "next_policy_hint")),
    "prefer-route-in-similar-state"
  );
  assert!(!as_bool(get(success, "host_apply_allowed")));
  assert!(!as_bool(get(success, "file_write_allowed")));
  assert!(!as_bool(get(success, "search_execution_allowed")));
  assert!(!as_bool(get(success, "policy_persistence_allowed")));

  let accepted = as_list(get(success, "accepted_learning"));
  assert_eq!(accepted.len(), 1);
  assert_eq!(
    as_str(get(&accepted[0], "route_id")),
    "generic-host-plan-route"
  );

  let unknown = get(&run, "unknown-learning");
  assert_eq!(as_str(get(unknown, "learning_status")), "failure");
  assert_eq!(as_str(get(unknown, "failure_kind")), "missing-knowledge");
  assert!(as_bool(get(unknown, "search_needed")));
  assert_eq!(as_str(get(unknown, "search_query_kind")), "docs");
  assert_eq!(
    as_str(get(unknown, "next_policy_hint")),
    "build-search-need-plan-before-retry"
  );
  let rejected = as_list(get(unknown, "rejected_learning"));
  assert_eq!(rejected.len(), 1);
  assert_eq!(as_str(get(&rejected[0], "route_id")), "generic-api-route");

  let dispatched = get(&run, "dispatched-learning");
  assert_eq!(as_str(get(dispatched, "op")), "mirror-learn-observe");
  assert_eq!(
    as_str(get(get(dispatched, "result"), "outcome")),
    "mirror-learning-observation-built"
  );
}

#[test]
fn search_need_plan_is_candidate_only_and_never_executes_search() {
  let run = eval_file(&fixture_path()).unwrap();

  let success_plan = get(&run, "success-search-plan");
  assert_eq!(
    as_str(get(success_plan, "outcome")),
    "search-need-plan-not-required"
  );
  assert!(!as_bool(get(success_plan, "search_needed")));
  assert!(!as_bool(get(success_plan, "search_execution_allowed")));

  let unknown_plan = get(&run, "unknown-search-plan");
  assert_eq!(
    as_str(get(unknown_plan, "outcome")),
    "search-need-plan-built"
  );
  assert!(as_bool(get(unknown_plan, "search_plan_built")));
  assert!(as_bool(get(unknown_plan, "search_needed")));
  assert_eq!(
    as_str(get(unknown_plan, "evidence_lane")),
    "external-candidate"
  );
  assert!(as_bool(get(unknown_plan, "verification_required")));
  assert!(!as_bool(get(unknown_plan, "search_execution_allowed")));
  assert!(!as_bool(get(unknown_plan, "memory_write_allowed")));
  assert!(as_str(get(unknown_plan, "query_text")).contains("failure_kind=missing-knowledge"));

  let dispatched = get(&run, "dispatched-search");
  assert_eq!(as_str(get(dispatched, "op")), "plan-search-need");
  assert_eq!(
    as_str(get(get(dispatched, "result"), "outcome")),
    "search-need-plan-built"
  );
}

#[test]
fn route_policy_update_returns_data_delta_without_persistence() {
  let run = eval_file(&fixture_path()).unwrap();

  let success = get(&run, "success-policy");
  assert_eq!(as_str(get(success, "outcome")), "route-policy-update-built");
  assert!(as_bool(get(success, "route_policy_delta_built")));
  assert!(as_bool(get(success, "route_policy_updated")));
  assert_eq!(as_list(get(success, "promote_routes")).len(), 1);
  assert_eq!(as_list(get(success, "demote_routes")).len(), 0);
  assert!(!as_bool(get(success, "policy_persistence_allowed")));
  let success_routes = as_list(get(success, "updated_routes"));
  let host_route = find_route(success_routes, "generic-host-plan-route");
  assert_eq!(as_i64(get(host_route, "attempts")), 3);
  assert_eq!(as_i64(get(host_route, "successes")), 2);

  let failure = get(&run, "failure-policy");
  assert_eq!(as_str(get(failure, "outcome")), "route-policy-update-built");
  assert_eq!(as_list(get(failure, "promote_routes")).len(), 0);
  assert_eq!(as_list(get(failure, "demote_routes")).len(), 1);
  assert_eq!(
    as_str(&as_list(get(failure, "avoid_patterns"))[0]),
    "missing-knowledge"
  );
  let failure_routes = as_list(get(failure, "updated_routes"));
  let api_route = find_route(failure_routes, "generic-api-route");
  assert_eq!(as_i64(get(api_route, "attempts")), 2);
  assert_eq!(as_i64(get(api_route, "successes")), 1);
  assert!(!as_bool(get(failure, "memory_write_allowed")));
  assert!(!as_bool(get(failure, "policy_persistence_allowed")));

  let dispatched = get(&run, "dispatched-policy");
  assert_eq!(as_str(get(dispatched, "op")), "update-route-policy");
  assert_eq!(
    as_str(get(get(dispatched, "result"), "outcome")),
    "route-policy-update-built"
  );
}

#[test]
fn direct_effect_requests_are_held() {
  let run = eval_file(&fixture_path()).unwrap();
  let held = get(&run, "effect-held");
  assert!(as_bool(get(held, "is_held")));
  assert_eq!(
    as_str(get(held, "outcome")),
    "held-mirror-learning-observation-effect-blocked"
  );
  assert!(!as_bool(get(held, "search_execution_allowed")));
  assert!(!as_bool(get(held, "memory_write_allowed")));
  assert!(!as_bool(get(held, "policy_persistence_allowed")));
}
