//! Mechanical metaInterpret self-extension pipeline.
//!
//! This pins the currently practical self-extension path: generated
//! metaInterpret candidates must run through evaluators, compare/replay when a
//! reference exists, register useful repeated routes as ankh fast-path
//! candidates, and keep runtime expansion behind owner gates.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

static EVAL_LOCK: Mutex<()> = Mutex::new(());

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join(
    "../../fixtures/tesseract-macro-legacy-probe/mechanical_metainterpret_self_extension_pipeline_receipt.px",
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

fn list_strings(v: &Value) -> Vec<&str> {
  as_list(v).iter().map(as_str).collect()
}

fn string_set(v: &Value) -> BTreeSet<&str> {
  list_strings(v).into_iter().collect()
}

fn attrs_by_key<'a>(items: &'a Value, key: &str) -> BTreeMap<&'a str, &'a Value> {
  as_list(items)
    .iter()
    .map(|item| (as_str(get(item, key)), item))
    .collect()
}

fn run() -> Value {
  let _guard = EVAL_LOCK.lock().expect("eval lock poisoned");
  eval_file(&fixture_path()).expect("mechanical self-extension receipt must evaluate")
}

#[test]
fn marker_and_truth_surfaces_are_pinned() {
  let run = run();
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "mechanical-metainterpret-self-extension-pipeline"
  );
  assert_eq!(
    as_str(get(&run, "truth-owner")),
    "project-wiki/maps/tesseract-macro-ontology-migration-algorithm-map.md"
  );
  assert_eq!(
    as_str(get(&run, "discovery-ledger")),
    "project-wiki/maps/tesseract-macro-ontology-discovery-ledger.md"
  );
  assert_eq!(
    as_str(get(&run, "constitution-owner")),
    "stdlib/lib/gate/tesseract-constitution.px"
  );
  assert_eq!(
    as_str(get(&run, "unknown-world-basis")),
    "unknown-world-metainterpret-harness-protocol"
  );
}

#[test]
fn constitution_gate_blocks_mechanical_overclaims() {
  let run = run();
  let gate = get(&run, "constitution-gate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "mechanical-metainterpret-self-extension-pipeline"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));

  let held_if = string_set(get(gate, "held-if"));
  for expected in [
    "claims-insight-without-generated-metainterpret-evaluation",
    "claims-fast-path-without-replay-determinism",
    "claims-ankh-as-second-brain-or-owner-override",
    "claims-runtime-extension-without-owner-proof",
    "claims-reverse-transform-as-forward-proof",
    "claims-external-solver-before-internal-capability-map-and-benchmark",
    "claims-scientific-intuition-as-current-mechanical-implementation",
    "claims-candidate-toolbox-as-installed-runtime",
  ] {
    assert!(held_if.contains(expected), "missing held-if `{expected}`");
  }

  let blocks = string_set(get(gate, "blocked-shortcuts"));
  assert!(blocks.contains("prose-insight-equals-mechanical-discovery"));
  assert!(blocks.contains("generated-metainterpret-not-run"));
  assert!(blocks.contains("compare-skipped-for-legacy-replacement"));
  assert!(blocks.contains("ankh-fast-path-without-same-judge-lifecycle"));
  assert!(blocks.contains("short-path-selected-with-held-loss-regression"));
  assert!(blocks.contains("runtime-extension-installed-from-candidate"));
  assert!(blocks.contains("reverse-fold-reuses-forward-authority"));
  assert!(blocks.contains("external-solver-first"));
}

#[test]
fn doctrine_separates_current_mechanical_path_from_deferred_scientific_insight() {
  let run = run();
  let doctrine = get(&run, "mechanical-doctrine");
  assert_eq!(
    as_str(get(doctrine, "id")),
    "doctrine.mechanical-metainterpret-self-extension.v1"
  );
  assert!(as_str(get(doctrine, "current-practical-insight"))
    .contains("mechanical metaInterpret generation"));
  assert_eq!(
    as_str(get(doctrine, "scientific-insight-status")),
    "deferred-domain-absorption-candidate"
  );
  assert!(as_str(get(doctrine, "scientific-insight-use"))
    .contains("later math/physics/biology/domain-library absorption"));

  let does_not_rely = string_set(get(doctrine, "current-system-does-not-rely-on"));
  assert!(does_not_rely.contains("prose-only-intuition"));
  assert!(does_not_rely.contains("scientific-metaphor-as-implementation"));
  assert!(does_not_rely.contains("LLM-as-main-system"));
  assert!(does_not_rely.contains("external-solver-first"));

  let relies = string_set(get(doctrine, "current-system-relies-on"));
  assert!(relies.contains("generated-metainterpret-receipt"));
  assert!(relies.contains("actual-px-evaluation"));
  assert!(relies.contains("comparison-or-replay-when-reference-exists"));
  assert!(relies.contains("ankh same-judge fast-path candidate"));
  assert!(relies.contains("owner-gated runtime extension candidate"));
}

#[test]
fn lifecycle_orders_generate_run_compare_ankh_fast_path_runtime_and_reverse() {
  let run = run();
  let lifecycle = get(&run, "metainterpret-lifecycle");
  assert_eq!(
    as_str(get(lifecycle, "id")),
    "pipeline.metainterpret-generate-run-evaluate-compare"
  );
  assert!(as_bool(get(lifecycle, "not-installed-runtime")));

  let stages = attrs_by_key(get(lifecycle, "stages"), "id");
  assert_eq!(stages.len(), 10);
  for id in [
    "stage.1.pressure",
    "stage.2.generate-metainterpret",
    "stage.3.execute-evaluator",
    "stage.4.compare-or-replay",
    "stage.5.fold-output",
    "stage.6.register-ankh-candidate",
    "stage.7.benchmark-short-path",
    "stage.8.runtime-extension-candidate",
    "stage.9.reverse-transform",
    "stage.10.ext-or-solver-intake",
  ] {
    assert!(stages.contains_key(id), "missing lifecycle stage `{id}`");
  }
  assert!(as_str(get(
    stages.get("stage.3.execute-evaluator").unwrap(),
    "action"
  ))
  .contains("p-puck"));
  assert!(as_str(get(
    stages.get("stage.6.register-ankh-candidate").unwrap(),
    "action"
  ))
  .contains("same judge/lifecycle"));
  assert!(as_str(get(
    stages.get("stage.7.benchmark-short-path").unwrap(),
    "action"
  ))
  .contains("Held/loss"));
}

#[test]
fn ankh_fast_path_policy_blocks_second_brain_and_owner_override() {
  let run = run();
  let policy = get(&run, "ankh-fast-path-policy");
  assert_eq!(
    as_str(get(policy, "id")),
    "policy.ankh-fast-path-registration.v1"
  );
  assert_eq!(
    as_str(get(policy, "status")),
    "candidate-registered-not-runtime"
  );
  assert!(
    as_str(get(policy, "ankh-definition")).contains("PNIX-owned macro code / route structure")
  );
  assert!(
    as_str(get(policy, "intelligence-core-claim")).contains("ankh as self macro-code structure")
  );

  let evidence = string_set(get(policy, "required-evidence"));
  assert!(evidence.contains("same-judge-same-lifecycle"));
  assert!(evidence.contains("repeated-use-or-route-pressure"));
  assert!(evidence.contains("replay-determinism"));
  assert!(evidence.contains("Held/loss-preservation"));
  assert!(evidence.contains("audit-ref-preservation"));
  assert!(evidence.contains("owner-gate-before-runtime-route"));

  let blocked = string_set(get(policy, "blocked-effects"));
  assert!(blocked.contains("second-brain"));
  assert!(blocked.contains("owner-law-override"));
  assert!(blocked.contains("accepted-runtime-install"));
  assert!(blocked.contains("negative-evidence-erasure"));
  assert!(blocked.contains("stdlib-meaning-db-as-core-intelligence"));

  let candidates = string_set(get(policy, "ankh-candidates"));
  assert!(candidates.contains("candidate.ankh.frequent-route"));
  assert!(candidates.contains("candidate.ankh.semantic-split-merge-tool"));
  assert!(candidates.contains("candidate.ankh.reverse-transform-pattern"));
}

#[test]
fn stdlib_meaning_db_is_future_lookup_substrate_not_ankh_core() {
  let run = run();
  let policy = get(&run, "stdlib-meaning-db-policy");
  assert_eq!(
    as_str(get(policy, "id")),
    "policy.stdlib-meaning-db-after-ontology-completion.v1"
  );
  assert_eq!(
    as_str(get(policy, "status")),
    "future-candidate-not-runtime"
  );
  assert!(as_str(get(policy, "purpose")).contains("PNIX can interpret its own language"));
  assert!(as_str(get(policy, "relation-to-ankh"))
    .contains("ankh is self-owned macro-code intelligence structure"));

  let surfaces = string_set(get(policy, "allowed-surfaces"));
  assert!(surfaces.contains("stdlib symbol meaning index"));
  assert!(surfaces.contains("owner route lookup"));
  assert!(surfaces.contains("semantic import map"));
  assert!(surfaces.contains("projection cache with replay hash"));

  let boundaries = string_set(get(policy, "hard-boundaries"));
  assert!(boundaries.contains("meaning-db-is-not-owner-law"));
  assert!(boundaries.contains("meaning-db-is-not-ankh"));
  assert!(boundaries.contains("lookup-cache-is-not-intelligence-core"));
  assert!(boundaries.contains("stdlib-interpretation-requires-ontology-owner-proof"));
  assert!(boundaries.contains("db-entry-cannot-install-runtime-route"));
}

#[test]
fn runtime_extension_is_candidate_route_after_proof_not_host_growth() {
  let run = run();
  let policy = get(&run, "runtime-extension-policy");
  assert_eq!(
    as_str(get(policy, "id")),
    "policy.runtime-extension-candidate-from-mechanical-proof.v1"
  );
  assert_eq!(
    as_str(get(policy, "status")),
    "candidate-registered-not-runtime"
  );
  assert!(as_str(get(policy, "route")).contains("owner-gated route/adapter candidate"));

  let surfaces = string_set(get(policy, "allowed-runtime-surfaces"));
  assert!(surfaces.contains("scoped route adapter candidate"));
  assert!(surfaces.contains("semantic API call candidate"));
  assert!(surfaces.contains("domain library absorption candidate"));
  assert!(surfaces.contains("non-GPL external accelerator adapter candidate"));

  let stops = string_set(get(policy, "hard-stops"));
  assert!(stops.contains("no-global-runtime-install"));
  assert!(stops.contains("no-host-growth-before-px-owner-or-harness-gap"));
  assert!(stops.contains("no-runtime-extension-from-prose"));
  assert!(stops.contains("no-short-path-if-Held-loss-regresses"));
}

#[test]
fn reverse_transform_is_separate_candidate_turn_not_forward_authority() {
  let run = run();
  let policy = get(&run, "reverse-transform-policy");
  assert_eq!(
    as_str(get(policy, "id")),
    "policy.reverse-fold-transform.v1"
  );
  assert_eq!(
    as_str(get(policy, "status")),
    "candidate-registered-not-runtime"
  );
  assert_eq!(
    as_str(get(policy, "forward-example")),
    "A -> C path compression candidate"
  );
  assert_eq!(
    as_str(get(policy, "reverse-example")),
    "C-start / A<-C reverse turn candidate"
  );
  assert!(as_str(get(policy, "rule")).contains("separate tesseract turn instance"));

  let imports = string_set(get(policy, "imports"));
  assert!(imports.contains("tesseract-macro-ontology-path-compression-discovery"));
  assert!(imports.contains("tesseract-macro-ontology-recipe-match-reverse-turn-discovery"));

  let uses = string_set(get(policy, "uses"));
  assert!(uses.contains("repair search"));
  assert!(uses.contains("missing middle clue"));
  assert!(uses.contains("domain API route inversion"));
  assert!(uses.contains("math contrapositive / contradiction candidate"));

  let blocked = string_set(get(policy, "blocked-shortcuts"));
  assert!(blocked.contains("reuse-forward-turn-as-reverse-proof"));
  assert!(blocked.contains("accept-middle-clue-without-replay"));
}

#[test]
fn external_intake_stays_after_internal_map_benchmark_license_and_bottleneck() {
  let run = run();
  let policy = get(&run, "external-intake-policy");
  assert_eq!(
    as_str(get(policy, "id")),
    "policy.ext-library-and-solver-intake-after-internal-map.v1"
  );
  assert_eq!(
    as_str(get(policy, "status")),
    "candidate-registered-not-runtime"
  );
  assert_eq!(
    as_i64(get(policy, "external-solver-dependency-count-now")),
    0
  );
  assert_eq!(
    as_str(get(policy, "ext-absorption-source")),
    "tesseract-macro-ontology-ext-lib-absorption-frontier-discovery"
  );
  assert!(as_str(get(policy, "demo-trigger")).contains("demo-scale bottleneck"));

  let order = string_set(get(policy, "order"));
  assert!(order.contains("map PNIX internal capability first"));
  assert!(order.contains("benchmark PNIX-owned behavior"));
  assert!(order.contains("require non-GPL/license evidence"));
  assert!(order.contains("treat external solver as subordinate accelerator candidate"));

  let stops = string_set(get(policy, "hard-stops"));
  assert!(stops.contains("GPL-family-dependency-held"));
  assert!(stops.contains("solver-is-not-intelligence-owner"));
  assert!(stops.contains("no-solver-before-internal-map"));
  assert!(stops.contains("no-raw-library-call-as-semantic-call"));
}

#[test]
fn capability_toolbox_uses_discovered_capabilities_without_installing_them() {
  let run = run();
  let toolbox = get(&run, "capability-toolbox");
  assert_eq!(
    as_str(get(toolbox, "id")),
    "registry.mechanical-capability-toolbox.v1"
  );
  assert_eq!(
    as_str(get(toolbox, "status")),
    "candidate-registered-not-runtime"
  );
  assert_eq!(
    as_str(get(toolbox, "selected-algorithm-status")),
    "candidate-selected-after-replay-not-installed-runtime"
  );
  assert!(
    as_str(get(toolbox, "rule")).contains("tool selection is not runtime install or owner switch")
  );

  let families = string_set(get(toolbox, "source-families"));
  assert!(families.contains("generated-metainterpret"));
  assert!(families.contains("ankh-fast-path"));
  assert!(families.contains("reverse-fold-transform"));
  assert!(families.contains("runtime-extension-route"));
  assert!(families.contains("ext-library-or-solver-intake"));
  assert!(families.contains("macro-authoring-self-extension"));

  let uses = string_set(get(toolbox, "split-merge-usage"));
  assert!(uses.contains("split-goal-meaning-into-tool-roles"));
  assert!(uses.contains("select-candidate-tool-for-each-role"));
  assert!(uses.contains("merge-command-trace-into-abstract-meaning"));
}

#[test]
fn selection_trials_cover_fake_insight_compare_gap_ankh_abuse_loss_and_complete_candidate() {
  let run = run();
  let trials = attrs_by_key(get(&run, "selection-trials"), "id");
  assert_eq!(trials.len(), 5);
  for (id, held) in [
    ("trial.A.prose-insight", "held.mechanical.no-evaluator"),
    (
      "trial.B.generated-not-compared",
      "held.mechanical.compare-or-replay-missing",
    ),
    (
      "trial.C.ankh-second-brain",
      "held.mechanical.ankh-owner-override",
    ),
    (
      "trial.D.short-path-regresses-held",
      "held.mechanical.short-path-loss-regression",
    ),
  ] {
    let trial = trials.get(id).expect("trial");
    assert_eq!(as_str(get(trial, "verdict")), "Held");
    assert_eq!(as_str(get(trial, "held-id")), held);
  }
  let complete = trials
    .get("trial.E.complete-mechanical-candidate")
    .expect("complete candidate trial");
  assert_eq!(
    as_str(get(complete, "verdict")),
    "candidate-selected-after-replay"
  );
}

#[test]
fn discoveries_record_d272_through_d281() {
  let run = run();
  let discoveries = attrs_by_key(get(&run, "discoveries"), "id");
  assert_eq!(discoveries.len(), 10);
  for id in [
    "D272.current-self-extension-is-mechanical-metainterpret-loop",
    "D273.scientific-intuition-is-deferred-domain-absorption-tool",
    "D274.ankh-fast-path-registration-requires-same-judge-lifecycle",
    "D275.short-path-selection-is-algorithm-candidate-not-runtime-install",
    "D276.runtime-extension-follows-owner-gated-route-candidate",
    "D277.reverse-fold-transform-is-separate-candidate-turn",
    "D278.external-solver-intake-is-after-internal-map-and-benchmark",
    "D279.discovered-capabilities-enter-candidate-toolbox",
    "D280.complete-mechanical-candidate-opens-owner-gated-route-only",
    "D281.stdlib-meaning-db-is-lookup-substrate-not-ankh-core",
  ] {
    let d = discoveries.get(id).expect("discovery id");
    assert_eq!(as_str(get(d, "decision-pressure")), "keep");
    assert!(as_bool(get(d, "scenario-only")));
  }
}

#[test]
fn top_level_protocol_remains_candidate_only() {
  let run = run();
  assert_eq!(
    as_str(get(&run, "replacement-readiness")),
    "protocol-registered-not-runtime"
  );
  assert!(!as_bool(get(&run, "owner-switch")));
  assert!(!as_bool(get(&run, "runtime-install")));
  assert!(!as_bool(get(&run, "global-ontology-runtime")));
  assert!(!as_bool(get(&run, "macro-only-runtime-owner")));
  assert!(!as_bool(get(&run, "implementation-command")));
}
