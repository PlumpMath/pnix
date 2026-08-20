//! Internal self-capability map discovery.
//!
//! This pins the corrected priority: PNIX maps its own deterministic symbolic /
//! meta-circular / tesseract capabilities before depending on external solvers.
//! External solvers are later acceleration adapters only after internal
//! benchmark/demo bottlenecks prove need.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join(
    "../../fixtures/tesseract-macro-legacy-probe/internal_self_capability_map_discovery_receipt.px",
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

fn get_path<'a>(root: &'a Value, path: &[&str]) -> &'a Value {
  let mut cur = root;
  for key in path {
    cur = get(cur, key);
  }
  cur
}

fn list_strings(v: &Value) -> Vec<&str> {
  as_list(v).iter().map(as_str).collect()
}

fn string_set(v: &Value) -> BTreeSet<&str> {
  list_strings(v).into_iter().collect()
}

fn attrs_by_id<'a>(items: &'a Value) -> BTreeMap<&'a str, &'a Value> {
  as_list(items)
    .iter()
    .map(|item| (as_str(get(item, "id")), item))
    .collect()
}

#[test]
fn marker_truth_owner_and_constitution_owner_are_pinned() {
  let run = eval_file(&fixture_path()).expect("internal self-capability map fixture must eval");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "tesseract-macro-ontology-internal-self-capability-map"
  );
  assert_eq!(
    as_str(get(&run, "truth-owner")),
    "project-wiki/maps/tesseract-macro-ontology-discovery-ledger.md"
  );
  assert_eq!(
    as_str(get(&run, "replacement-map")),
    "project-wiki/maps/tesseract-macro-ontology-replacement-map.md"
  );
  assert_eq!(
    as_str(get(&run, "constitution-owner")),
    "stdlib/lib/gate/tesseract-constitution.px"
  );
}

#[test]
fn directive_surface_prioritizes_internal_map_before_external_solver() {
  let run = eval_file(&fixture_path()).unwrap();
  let directive = get(&run, "directive-surface");
  assert_eq!(
    as_str(get(directive, "id")),
    "surface.user-directive.internal-self-capability-first"
  );
  assert_eq!(
    as_str(get(directive, "author-intent")),
    "internal-self-capability-map-before-external-solver"
  );
  assert_eq!(
    as_str(get(directive, "ai-role")),
    "human-tool-not-main-authority"
  );
  assert!(!as_bool(get(directive, "llm-main-system")));
  assert!(!as_bool(get(directive, "external-solver-first")));
  assert!(!as_bool(get(directive, "command-is-implementation")));
  assert!(as_bool(get(directive, "requires-constitution-gate")));
}

#[test]
fn constitution_gate_blocks_external_first_and_llm_main_shortcuts() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "internal-self-capability-map"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));

  let held_if = string_set(get(gate, "held-if"));
  for expected in [
    "external-solver-dependency-added-before-self-map",
    "llm-output-treated-as-main-intelligence",
    "capability-claimed-without-receipt",
    "self-map-without-benchmark-frontier",
    "demo-bottleneck-missing",
    "gpl-family-dependency-added",
    "nondeterministic-output-treated-as-canonical-proof",
  ] {
    assert!(held_if.contains(expected), "missing held-if `{expected}`");
  }

  let blocked = string_set(get(gate, "blocked-shortcuts"));
  for expected in [
    "import-external-solver-before-internal-map",
    "treat-llm-prose-as-deterministic-core",
    "claim-math-kernel-without-internal-proof-carrier",
    "hide-self-capability-gaps",
    "convert-held-frontier-to-false-success",
    "use-gpl-family-solver-as-runtime-dependency",
    "treat-demo-speed-pressure-as-owner-proof",
  ] {
    assert!(blocked.contains(expected), "missing block `{expected}`");
  }
}

#[test]
fn self_capability_map_counts_internal_families_without_external_dependencies() {
  let run = eval_file(&fixture_path()).unwrap();
  let map = get(&run, "self-capability-map");
  assert_eq!(as_str(get(map, "id")), "map.internal-self-capability.v1");
  assert_eq!(
    as_str(get(map, "priority")),
    "internal-symbolic-core-before-external-solver"
  );
  assert_eq!(as_i64(get(map, "capability-count")), 8);
  assert_eq!(as_i64(get(map, "deterministic-core-count")), 8);
  assert_eq!(as_i64(get(map, "scoped-installed-count")), 1);
  assert_eq!(as_i64(get(map, "candidate-tool-count")), 6);
  assert_eq!(as_i64(get(map, "frontier-need-count")), 1);
  assert_eq!(as_i64(get(map, "external-solver-required-count")), 0);
  assert_eq!(as_i64(get(map, "llm-required-count")), 0);
  assert_eq!(as_i64(get(map, "gpl-family-dependency-count")), 0);
}

#[test]
fn inventory_contains_expected_internal_capability_families() {
  let run = eval_file(&fixture_path()).unwrap();
  let inventory = attrs_by_id(get_path(&run, &["self-capability-map", "inventory"]));
  assert_eq!(inventory.len(), 8);
  for expected in [
    "capability.self.semantic-meaning-fold",
    "capability.self.self-learning-input-fold",
    "capability.self.fixture-local-mutation-loop",
    "capability.self.reverse-turn-path-compression",
    "capability.self.eval-select-scoped-fast-path",
    "capability.self.constitution-gated-lifecycle",
    "capability.self.six-layer-tesseract-fold",
    "capability.self.math-kernel-held-carrier",
  ] {
    assert!(
      inventory.contains_key(expected),
      "missing capability `{expected}`"
    );
  }
}

#[test]
fn every_mapped_capability_is_internal_deterministic_not_llm_or_external_solver() {
  let run = eval_file(&fixture_path()).unwrap();
  let inventory = as_list(get_path(&run, &["self-capability-map", "inventory"]));
  for capability in inventory {
    assert!(as_bool(get(capability, "deterministic-core")));
    assert!(!as_bool(get(capability, "llm-required")));
    assert!(!as_bool(get(capability, "external-solver-required")));
    assert!(
      !as_str(get(capability, "held-frontier")).is_empty(),
      "held frontier must be explicit"
    );
  }
}

#[test]
fn eval_select_fast_path_is_the_only_scoped_installed_capability() {
  let run = eval_file(&fixture_path()).unwrap();
  let inventory = attrs_by_id(get_path(&run, &["self-capability-map", "inventory"]));
  let fast = inventory
    .get("capability.self.eval-select-scoped-fast-path")
    .unwrap();
  assert_eq!(as_str(get(fast, "status")), "scoped-installed");
  assert_eq!(
    as_str(get(fast, "toolization-state")),
    "installed-surface-pair-tool"
  );
  assert!(as_bool(get(fast, "installed")));
  assert_eq!(
    as_str(get(fast, "installed-scope")),
    "legacy-evaluate-select-surface-pair-only"
  );

  for (id, capability) in inventory {
    if id != "capability.self.eval-select-scoped-fast-path" {
      assert!(
        !as_bool(get(capability, "installed")),
        "`{id}` should not be installed"
      );
    }
  }
}

#[test]
fn math_kernel_is_mapped_as_internal_frontier_need_not_external_cas_dependency() {
  let run = eval_file(&fixture_path()).unwrap();
  let inventory = attrs_by_id(get_path(&run, &["self-capability-map", "inventory"]));
  let math = inventory
    .get("capability.self.math-kernel-held-carrier")
    .unwrap();
  assert_eq!(as_str(get(math, "status")), "frontier-need");
  assert_eq!(as_str(get(math, "toolization-state")), "not-yet-built");
  assert!(as_str(get(math, "role")).contains("definitions"));
  assert!(as_str(get(math, "role")).contains("counterexamples"));
  assert!(!as_bool(get(math, "external-solver-required")));
}

#[test]
fn restriction_absorption_turns_limits_into_gates_and_frontiers() {
  let run = eval_file(&fixture_path()).unwrap();
  let items: BTreeMap<&str, &Value> = as_list(get(&run, "restriction-absorption"))
    .iter()
    .map(|item| (as_str(get(item, "restriction")), item))
    .collect();
  assert_eq!(items.len(), 4);
  assert_eq!(
    as_str(get(items.get("no-global-runtime").unwrap(), "status")),
    "partially-opened"
  );
  assert_eq!(
    as_str(get(
      items.get("no-external-solver-first").unwrap(),
      "status"
    )),
    "kept-as-priority-gate"
  );
  assert_eq!(
    as_str(get(
      items.get("no-llm-main-authority").unwrap(),
      "absorbed-as"
    )),
    "llm-as-proposal-surface-only"
  );
  assert_eq!(
    as_str(get(items.get("no-fake-success").unwrap(), "absorbed-as")),
    "Held/Need gaps stay first-class map entries"
  );
}

#[test]
fn external_solver_policy_is_deferred_until_bottleneck_and_license_evidence() {
  let run = eval_file(&fixture_path()).unwrap();
  let policy = get(&run, "external-solver-policy");
  assert_eq!(
    as_str(get(policy, "id")),
    "policy.external-solver.deferred-until-bottleneck"
  );
  assert_eq!(as_str(get(policy, "status")), "deferred");
  assert_eq!(as_i64(get(policy, "dependency-count")), 0);
  assert!(!as_bool(get(policy, "use-before-self-map")));
  assert!(!as_bool(get(policy, "use-before-demo-bottleneck")));
  assert!(as_str(get(policy, "allowed-trigger")).contains("too slow"));
  assert!(!as_bool(get(policy, "gpl-family-dependencies-allowed")));
  assert!(as_bool(get(policy, "dependency-license-evidence-required")));
  assert_eq!(
    as_str(get(policy, "external-solver-role")),
    "optional acceleration adapter, not intelligence owner"
  );
}

#[test]
fn human_tool_doctrine_keeps_pnix_deterministic_and_llm_subordinate() {
  let run = eval_file(&fixture_path()).unwrap();
  let doctrine = get(&run, "human-tool-doctrine");
  assert_eq!(as_str(get(doctrine, "id")), "doctrine.ai-is-human-tool");
  assert!(as_str(get(doctrine, "human-role")).contains("goal"));
  assert!(as_str(get(doctrine, "pnix-role")).contains("deterministic"));
  assert!(as_str(get(doctrine, "llm-role")).contains("nondeterministic"));
  assert!(as_str(get(doctrine, "external-solver-role")).contains("accelerator"));
  assert!(!as_bool(get(doctrine, "llm-as-main-system")));
  assert!(!as_bool(get(doctrine, "external-solver-as-main-system")));
  assert!(as_bool(get(doctrine, "pnix-self-capability-map-first")));
}

#[test]
fn toolization_frontiers_record_needs_and_held_without_hiding_gaps() {
  let run = eval_file(&fixture_path()).unwrap();
  let frontiers = attrs_by_id(get(&run, "toolization-frontiers"));
  assert_eq!(frontiers.len(), 5);
  for expected in [
    "need.self-capability.operation-catalog",
    "need.self-capability.benchmark-map",
    "need.math-kernel.minimal-internal-carrier",
    "held.external-solver-before-self-map",
    "held.llm-as-main-ai-core",
  ] {
    assert!(
      frontiers.contains_key(expected),
      "missing frontier `{expected}`"
    );
  }
  assert_eq!(
    as_str(get(
      frontiers
        .get("need.math-kernel.minimal-internal-carrier")
        .unwrap(),
      "status"
    )),
    "Need"
  );
  assert_eq!(
    as_str(get(
      frontiers
        .get("held.external-solver-before-self-map")
        .unwrap(),
      "status"
    )),
    "Held"
  );
  for frontier in frontiers.values() {
    assert!(!as_bool(get(frontier, "external-solver-required")));
  }
}

#[test]
fn runtime_observation_records_candidate_map_without_external_install() {
  let run = eval_file(&fixture_path()).unwrap();
  let runtime = get(&run, "runtime-observation");
  assert_eq!(
    as_str(get(runtime, "observation-model")),
    "internal-self-capability-map-before-external-solver"
  );
  assert!(as_bool(get(runtime, "can-appear-at-runtime")));
  assert!(as_bool(get(runtime, "candidate-only")));
  assert!(!as_bool(get(runtime, "runtime-install")));
  assert!(!as_bool(get(runtime, "external-solver-installed")));
  assert_eq!(as_i64(get(runtime, "external-solver-dependency-count")), 0);
  assert_eq!(as_i64(get(runtime, "scoped-fast-path-count")), 1);
  assert_eq!(as_i64(get(runtime, "internal-capability-count")), 8);
  assert_eq!(as_list(get(runtime, "runtime-added-candidates")).len(), 3);
}

#[test]
fn discoveries_record_d241_through_d252() {
  let run = eval_file(&fixture_path()).unwrap();
  let discoveries = attrs_by_id(get(&run, "discoveries"));
  assert_eq!(discoveries.len(), 12);
  for expected in [
    "D241.internal-capability-map-precedes-external-solver-intake",
    "D242.restrictions-are-converted-to-toolization-gates-not-permanent-neutering",
    "D243.current-self-map-already-has-eight-internal-capability-families",
    "D244.llm-is-proposal-surface-not-main-symbolic-core",
    "D245.external-solvers-are-deferred-until-self-map-and-demo-bottleneck",
    "D246.metaInterpret-tesseract-needs-operation-catalog-before-broad-toolization",
    "D247.math-kernel-starts-with-internal-held-aware-carrier",
    "D248.scoped-fast-path-is-allowed-only-after-internal-proof",
    "D249.human-tool-doctrine-keeps-ai-subordinate-to-human-purpose",
    "D250.unknown-self-capability-must-emit-need-or-held",
    "D251.self-capability-map-becomes-benchmark-target",
    "D252.external-dependency-policy-remains-future-acceleration-boundary",
  ] {
    let discovery = discoveries
      .get(expected)
      .unwrap_or_else(|| panic!("missing discovery `{expected}`"));
    assert!(as_bool(get(discovery, "scenario-only")));
  }
}

#[test]
fn affected_plans_prioritize_self_map_and_math_kernel_over_external_solver() {
  let run = eval_file(&fixture_path()).unwrap();
  let affected = get(&run, "affected-plans");
  assert!(as_bool(get_path(
    affected,
    &["internalSelfCapabilityMap", "implementation-target"]
  )));
  assert!(as_bool(get_path(
    affected,
    &["mathKernel", "implementation-target"]
  )));
  assert!(!as_bool(get_path(
    affected,
    &["externalSolverAdapters", "implementation-target"]
  )));
  assert_eq!(
    as_str(get_path(affected, &["externalSolverAdapters", "pressure"])),
    "defer-until-self-map-benchmark-demo-bottleneck"
  );
  assert_eq!(
    as_str(get_path(affected, &["llmAdapters", "role"])),
    "proposal-surface"
  );
}

#[test]
fn negative_held_evidence_rejects_external_first_llm_main_and_fake_capability() {
  let run = eval_file(&fixture_path()).unwrap();
  let negative = get(&run, "negative-held-evidence");
  assert_eq!(as_str(get(negative, "status")), "present");
  let rejects = string_set(get(negative, "rejects"));
  for expected in [
    "external-solver-before-internal-map",
    "llm-as-main-symbolic-core",
    "capability-claimed-without-receipt",
    "math-kernel-claimed-without-held-aware-carrier",
    "self-map-without-benchmark-frontier",
    "gpl-family-dependency-added",
    "nondeterministic-output-as-canonical-proof",
  ] {
    assert!(rejects.contains(expected), "missing rejection `{expected}`");
  }
}

#[test]
fn top_level_state_records_internal_map_first_without_runtime_or_external_solver() {
  let run = eval_file(&fixture_path()).unwrap();
  assert_eq!(as_str(get(&run, "replacement-readiness")), "not-proven");
  assert!(!as_bool(get(&run, "owner-switch")));
  assert!(!as_bool(get(&run, "runtime-install")));
  assert!(!as_bool(get(&run, "external-solver-installed")));
  assert_eq!(as_i64(get(&run, "external-solver-dependency-count")), 0);
  assert!(as_bool(get(&run, "self-capability-map-first")));
  assert!(!as_bool(get(&run, "llm-main-system")));
  assert!(!as_bool(get(&run, "implementation-command")));
}
