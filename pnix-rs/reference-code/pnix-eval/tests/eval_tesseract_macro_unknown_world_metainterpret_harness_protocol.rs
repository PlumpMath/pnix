//! Unknown-world metaInterpret harness protocol.
//!
//! This test pins the development method itself: PNIX tesseract macro ontology
//! work must be evaluator-first, discovery-ledger-backed, and constitution
//! gated. The receipt registers discovered harness techniques without claiming
//! runtime install, macro-only boot, or semantic owner switch.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join(
    "../../fixtures/tesseract-macro-legacy-probe/unknown_world_metainterpret_harness_protocol_receipt.px",
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

#[test]
fn marker_and_truth_surfaces_are_pinned() {
  let run = eval_file(&fixture_path()).expect("unknown-world harness receipt must evaluate");
  assert_eq!(
    as_str(get(&run, "probe-marker")),
    "unknown-world-metainterpret-harness-protocol"
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
}

#[test]
fn constitution_gate_blocks_fake_discovery_development_shortcuts() {
  let run = eval_file(&fixture_path()).unwrap();
  let gate = get(&run, "constitution-gate");
  assert_eq!(
    as_str(get(gate, "scenario")),
    "unknown-world-metainterpret-harness-protocol"
  );
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));

  let held_if = string_set(get(gate, "held-if"));
  for expected in [
    "claims-discovery-without-px-evaluation",
    "claims-migration-without-old-vs-new-compare",
    "implements-old-name-before-role-emission",
    "claims-green-test-as-semantic-discovery",
    "claims-fast-path-without-replay-and-delta-proof",
    "claims-intuition-as-proof",
    "claims-scientific-metaphor-as-implementation",
    "claims-performance-improvement-without-before-after-measurement",
    "claims-self-optimization-without-bottleneck-evidence",
    "claims-macro-authoring-self-extension-without-authoring-receipt",
    "claims-runtime-self-modification-from-macro-authoring-candidate",
    "claims-api-playback-as-semantic-understanding",
    "claims-forward-execution-without-reverse-meaning-abstraction",
    "claims-discovered-function-as-installed-tool-without-owner-proof",
    "claims-short-path-selection-without-held-loss-replay",
    "claims-p-puck-wrapper-as-semantic-owner",
    "adds-host-rust-before-px-owner-or-harness-gap",
    "uses-external-solver-before-internal-map-and-benchmark",
    "claims-macro-only-boot-before-bootstrap-manifest",
    "claims-macro-only-boot-manifest-as-runtime-boot",
    "claims-macro-only-boot-attempt-as-runtime-boot",
    "claims-macro-only-boot-runner-owner-as-runtime-boot",
    "claims-bounded-replay-strategy-as-runtime-boot",
    "claims-regression-corpus-retention-as-runtime-boot",
    "claims-regression-corpus-retention-as-fresh-puck-or-compare",
    "claims-bootstrap-audit-update-as-runtime-boot",
    "claims-bootstrap-audit-update-as-fresh-puck-or-compare",
    "claims-compare-after-boot-as-runtime-boot",
    "claims-compare-after-boot-as-fresh-puck-or-semantic-owner",
    "claims-target-delete-preflight-as-delete-proof",
    "claims-target-delete-preflight-as-host-removal",
    "claims-target-specific-delete-proof-as-host-removal",
    "claims-target-specific-delete-proof-as-fresh-puck",
    "claims-fresh-p-puck-as-full-receipt-audit",
    "claims-fresh-p-puck-as-replay-execution",
    "claims-fresh-p-puck-as-runtime-boot",
    "claims-fresh-p-puck-as-host-removal",
    "claims-bounded-replay-execution-as-runtime-boot",
    "claims-bounded-replay-execution-as-host-removal",
    "claims-bounded-replay-execution-as-semantic-owner",
    "claims-boot-execution-proof-as-runtime-owner",
    "claims-boot-execution-proof-as-new-engine-from-zero",
    "claims-boot-execution-proof-as-host-removal",
    "claims-boot-execution-proof-as-semantic-owner",
    "claims-runtime-owner-proof-as-new-engine-from-zero",
    "claims-runtime-owner-proof-as-global-runtime-install",
    "claims-runtime-owner-proof-as-host-removal",
    "claims-runtime-owner-proof-as-semantic-owner",
    "claims-semantic-owner-proof-as-new-engine-from-zero",
    "claims-semantic-owner-proof-as-global-runtime-install",
    "claims-semantic-owner-proof-as-host-removal",
    "claims-semantic-owner-proof-as-delete-ready",
  ] {
    assert!(held_if.contains(expected), "missing held-if `{expected}`");
  }

  let blocks = string_set(get(gate, "blocked-shortcuts"));
  assert!(blocks.contains("prose-only-discovery"));
  assert!(blocks.contains("book-backed-assumption-for-metainterpret"));
  assert!(blocks.contains("shortcut-sense-equals-accepted-fast-path"));
  assert!(blocks.contains("mathematical-intuition-equals-proof"));
  assert!(blocks.contains("scientific-metaphor-equals-code"));
  assert!(blocks.contains("unmeasured-fast-path-claim"));
  assert!(blocks.contains("single-benchmark-number-equals-proof"));
  assert!(blocks.contains("slow-path-telemetry-equals-policy-mutation"));
  assert!(blocks.contains("runtime-self-modification-disguised-as-macro-authoring"));
  assert!(blocks.contains("canonical-macro-owner-edit-without-replay"));
  assert!(blocks.contains("macro-code-candidate-installed-without-owner-gate"));
  assert!(blocks.contains("fixed-app-tape-equals-intelligence"));
  assert!(blocks.contains("command-splitting-without-meaning-representation"));
  assert!(blocks.contains("candidate-toolbox-equals-installed-runtime"));
  assert!(blocks.contains("short-path-choice-without-measurement"));
  assert!(blocks.contains("compare-after-boot-equals-boot-success"));
  assert!(blocks.contains("compare-after-boot-equals-fresh-puck-or-semantic-owner"));
  assert!(blocks.contains("target-delete-preflight-equals-delete-proof"));
  assert!(blocks.contains("target-delete-preflight-equals-host-removal"));
  assert!(blocks.contains("fresh-p-puck-equals-full-current-receipt-audit"));
  assert!(blocks.contains("fresh-p-puck-equals-replay-executed"));
  assert!(blocks.contains("fresh-p-puck-equals-boot-success"));
  assert!(blocks.contains("fresh-p-puck-equals-host-removal"));
  assert!(blocks.contains("bounded-replay-execution-equals-boot-success"));
  assert!(blocks.contains("bounded-replay-execution-equals-host-removal"));
  assert!(blocks.contains("bounded-replay-execution-equals-semantic-owner"));
  assert!(blocks.contains("boot-execution-proof-equals-runtime-owner"));
  assert!(blocks.contains("boot-execution-proof-equals-new-engine-from-zero"));
  assert!(blocks.contains("boot-execution-proof-equals-host-removal"));
  assert!(blocks.contains("boot-execution-proof-equals-semantic-owner"));
  assert!(blocks.contains("runtime-owner-proof-equals-new-engine-from-zero"));
  assert!(blocks.contains("runtime-owner-proof-equals-global-runtime-install"));
  assert!(blocks.contains("runtime-owner-proof-equals-host-removal"));
  assert!(blocks.contains("runtime-owner-proof-equals-semantic-owner"));
  assert!(blocks.contains("semantic-owner-proof-equals-new-engine-from-zero"));
  assert!(blocks.contains("semantic-owner-proof-equals-global-runtime-install"));
  assert!(blocks.contains("semantic-owner-proof-equals-host-removal"));
  assert!(blocks.contains("semantic-owner-proof-equals-delete-ready"));
  assert!(blocks.contains("host-code-growth-as-default"));
}

#[test]
fn unknown_world_doctrine_requires_live_evaluation_and_conditional_compare_policy() {
  let run = eval_file(&fixture_path()).unwrap();
  let doctrine = get(&run, "unknown-world-doctrine");
  assert_eq!(
    as_str(get(doctrine, "id")),
    "doctrine.unknown-world-metainterpret"
  );
  assert!(!as_bool(get(doctrine, "book-backed-domain")));
  assert!(as_bool(get(doctrine, "requires-live-evaluation")));
  assert!(!as_bool(get(doctrine, "requires-old-vs-new-compare")));
  assert!(as_str(get(doctrine, "old-vs-new-compare-scope")).contains("conditional"));
  assert!(
    as_str(get(doctrine, "old-vs-new-compare-scope")).contains("direct runtime API absorption")
  );
  assert!(as_bool(get(
    doctrine,
    "direct-runtime-api-absorption-target"
  )));
  assert!(as_bool(get(doctrine, "requires-discovery-ledger")));
  assert!(as_bool(get(doctrine, "requires-candidate-registry")));
  assert!(as_bool(get(doctrine, "searches-fast-paths")));
  assert!(as_bool(get(doctrine, "one-step-route-hypothesis")));
  assert!(as_bool(get(
    doctrine,
    "mathematical-intuition-as-candidate"
  )));
  assert!(as_bool(get(doctrine, "requires-performance-measurement")));
  assert!(as_bool(get(doctrine, "requires-before-after-benchmark")));
  assert!(as_bool(get(
    doctrine,
    "macro-authoring-self-extension-target"
  )));
  assert!(
    as_str(get(doctrine, "self-extension-definition")).contains("macro-code candidate generation")
  );
  assert!(as_str(get(doctrine, "runtime-api-coding-definition"))
    .contains("abstracting the execution trace back into meaning"));
  assert!(as_str(get(doctrine, "ai-definition"))
    .contains("decomposes a goal sentence into real-time executable commands"));
  assert!(as_str(get(doctrine, "ai-definition"))
    .contains("abstract / metaphorical / intuitive representation"));
  assert!(as_str(get(doctrine, "discovered-function-use-definition"))
    .contains("candidate tools for splitting meanings"));
  assert!(as_str(get(doctrine, "hypothesis-definition"))
    .contains("carrying proof obligations rather than proof authority"));
  assert!(
    as_str(get(doctrine, "self-observation-driver")).contains("meta-circular self-observation")
  );
  assert!(as_str(get(doctrine, "slow-path-self-optimization"))
    .contains("never automatic policy mutation"));
  assert!(as_bool(get(doctrine, "requires-bootstrap-honesty")));
}

#[test]
fn absorption_mode_policy_keeps_compare_conditional_for_future_ext_runtime_flattening() {
  let run = eval_file(&fixture_path()).unwrap();
  let policy = get(&run, "absorption-mode-policy");
  assert_eq!(
    as_str(get(policy, "id")),
    "policy.domain-runtime-absorption-modes.v1"
  );
  assert_eq!(
    as_str(get(policy, "default-target")),
    "direct-semantic-absorption"
  );
  assert!(!as_bool(get(policy, "old-vs-new-compare-default")));

  let modes = attrs_by_key(get(policy, "modes"), "id");
  assert_eq!(modes.len(), 5);

  for id in [
    "mode.legacy-replacement-specimen",
    "mode.unowned-or-unknown-api-probe",
    "mode.self-made-substitute",
  ] {
    assert!(as_bool(get(modes.get(id).unwrap(), "compare-required")));
  }

  for id in [
    "mode.direct-prepared-runtime-api",
    "mode.ext-library-flattening",
  ] {
    assert!(!as_bool(get(modes.get(id).unwrap(), "compare-required")));
  }

  assert!(as_str(get(
    modes.get("mode.ext-library-flattening").unwrap(),
    "use-when"
  ))
  .contains("X3D/CSS/SVG"));

  let boundaries = string_set(get(policy, "hard-boundaries"));
  assert!(boundaries.contains("direct-absorption-still-needs-owner-route-audit"));
  assert!(boundaries.contains("no-raw-api-call-as-semantic-call"));
  assert!(boundaries.contains("turn-compare-on-when-meaning-or-safety-is-unknown"));
}

#[test]
fn scientific_concept_coding_turns_models_into_self_insight_candidates() {
  let run = eval_file(&fixture_path()).unwrap();
  let doctrine = get(&run, "scientific-concept-coding");
  assert_eq!(
    as_str(get(doctrine, "id")),
    "doctrine.scientific-concept-coding"
  );
  assert!(as_str(get(doctrine, "mode")).contains("scientific concept models"));
  assert!(as_str(get(doctrine, "internal-self-insight")).contains("self-insight candidates"));

  let domains = string_set(get(doctrine, "active-domain-families"));
  assert!(domains.contains("mathematical-structure"));
  assert!(domains.contains("physical-system"));
  assert!(domains.contains("biological-system"));
  assert!(domains.contains("semantic-api-system"));

  let thought_basis = string_set(get(doctrine, "scientific-thought-basis"));
  for expected in [
    "deductive",
    "inductive",
    "abductive",
    "analogical",
    "counterexample-driven",
    "invariant-based",
    "causal",
    "dynamical-system",
    "evolutionary",
    "experimental",
    "statistical",
    "dimensional-analysis",
    "systems-theoretic",
  ] {
    assert!(
      thought_basis.contains(expected),
      "missing thought basis `{expected}`"
    );
  }

  let coding_loop = string_set(get(doctrine, "coding-loop"));
  assert!(coding_loop.contains("conceptual-model"));
  assert!(coding_loop.contains("px-receipt"));
  assert!(coding_loop.contains("proof-obligation"));

  let boundaries = string_set(get(doctrine, "strict-boundaries"));
  assert!(boundaries.contains("scientific-concept-is-model-not-proof"));
  assert!(boundaries.contains("metaphor-is-not-implementation"));
  assert!(boundaries.contains("self-insight-is-candidate-not-authority"));
}

#[test]
fn measurement_algorithms_register_speed_quality_and_bottleneck_metrics() {
  let run = eval_file(&fixture_path()).unwrap();
  let registry = get(&run, "measurement-algorithm-registry");
  assert_eq!(
    as_str(get(registry, "id")),
    "registry.performance-measurement-algorithms.v1"
  );
  assert_eq!(
    as_str(get(registry, "status")),
    "candidate-registered-not-runtime"
  );

  let shape = string_set(get(registry, "required-comparison-shape"));
  for expected in [
    "baseline-run",
    "candidate-run",
    "repeated-run-distribution",
    "semantic-equivalence-or-held-delta",
    "before-after-replay",
    "bottleneck-attribution",
  ] {
    assert!(
      shape.contains(expected),
      "missing comparison shape `{expected}`"
    );
  }

  let algorithms = attrs_by_key(get(registry, "algorithms"), "id");
  assert_eq!(algorithms.len(), 10);
  for id in [
    "measure.wall-clock-distribution",
    "measure.step-count-and-fold-depth",
    "measure.operation-count",
    "measure.memory-and-allocation-pressure",
    "measure.cache-and-reuse-rate",
    "measure.replay-determinism-hash",
    "measure.held-and-loss-delta",
    "measure.regression-corpus-throughput",
    "measure.bottleneck-attribution",
    "measure.asymptotic-shape-hypothesis",
  ] {
    assert!(algorithms.contains_key(id), "missing measurement `{id}`");
  }

  let wall = algorithms.get("measure.wall-clock-distribution").unwrap();
  let wall_measures = string_set(get(wall, "measures"));
  assert!(wall_measures.contains("p50"));
  assert!(wall_measures.contains("p95"));

  let semantic = algorithms.get("measure.held-and-loss-delta").unwrap();
  let semantic_measures = string_set(get(semantic, "measures"));
  assert!(semantic_measures.contains("held-count-delta"));
  assert!(semantic_measures.contains("negative-evidence-retained"));

  let not_proof = string_set(get(registry, "not-proof-by-itself"));
  assert!(not_proof.contains("single-run-duration"));
  assert!(not_proof.contains("faster-but-held-evidence-erased"));
}

#[test]
fn slow_path_telemetry_becomes_self_optimization_candidate_not_self_mutation() {
  let run = eval_file(&fixture_path()).unwrap();
  let loop_shape = get(&run, "self-optimization-feedback-loop");
  assert_eq!(
    as_str(get(loop_shape, "id")),
    "loop.slow-path-self-optimization-candidate.v1"
  );
  assert_eq!(
    as_str(get(loop_shape, "status")),
    "candidate-registered-not-runtime"
  );

  let sources = string_set(get(loop_shape, "source-signals"));
  assert!(sources.contains("p-puck slow-path telemetry"));
  assert!(sources.contains("receipt replay bottleneck ranking"));
  assert!(sources.contains("Held/loss delta after candidate rewrite"));

  let stages = string_set(get(loop_shape, "stages"));
  assert!(stages.contains("attribute-bottleneck"));
  assert!(stages.contains("run-baseline-and-candidate"));
  assert!(stages.contains("owner-gated-absorption-or-held"));

  let boundaries = string_set(get(loop_shape, "strict-boundaries"));
  assert!(boundaries.contains("no-automatic-self-modification"));
  assert!(boundaries.contains("no-policy-mutation-from-telemetry-alone"));
  assert!(boundaries.contains("no-speed-win-if-held-or-loss-regresses"));

  let outputs = string_set(get(loop_shape, "output-candidates"));
  assert!(outputs.contains("candidate.route-shortening"));
  assert!(outputs.contains("candidate.proof-reuse"));
}

#[test]
fn macro_authoring_self_extension_is_candidate_layer_not_runtime_self_modification() {
  let run = eval_file(&fixture_path()).unwrap();
  let policy = get(&run, "macro-authoring-self-extension-policy");
  assert_eq!(
    as_str(get(policy, "id")),
    "policy.macro-authoring-self-extension.v1"
  );
  assert_eq!(
    as_str(get(policy, "status")),
    "candidate-registered-not-runtime"
  );
  assert_eq!(
    as_str(get(policy, "allowed-self-modify-scope")),
    "macro-authoring-candidate-layer"
  );
  assert!(as_str(get(policy, "self-modify-interpretation")).contains("no direct runtime"));

  let forbidden_scope = string_set(get(policy, "forbidden-self-modify-scope"));
  assert!(forbidden_scope.contains("runtime-policy-mutation"));
  assert!(forbidden_scope.contains("canonical-owner-edit-without-replay"));
  assert!(forbidden_scope.contains("host-code-rewrite-as-first-step"));
  assert!(forbidden_scope.contains("p-puck-wrapper-as-author"));

  let stages = string_set(get(policy, "stages"));
  assert!(stages.contains("observe-current-macro-authoring-pattern"));
  assert!(stages.contains("extract-authoring-invariants"));
  assert!(stages.contains("emit-macro-code-candidate"));
  assert!(stages.contains("run-fixture-local-evaluation"));
  assert!(stages.contains("compare-held-loss-and-measurement-delta"));
  assert!(stages.contains("owner-gated-absorption-or-held"));

  let boundaries = string_set(get(policy, "strict-boundaries"));
  assert!(boundaries.contains("macro-authoring-candidate-is-not-runtime-self-modify"));
  assert!(boundaries.contains("candidate-only-before-owner-gate"));
  assert!(boundaries.contains("candidate-must-have-authoring-receipt"));
  assert!(boundaries.contains("candidate-must-run-through-evaluator"));
  assert!(boundaries.contains("candidate-must-pass-owner-gate-before-absorption"));

  let outputs = string_set(get(policy, "output-candidates"));
  assert!(outputs.contains("candidate.macro-template"));
  assert!(outputs.contains("candidate.macro-builder"));
  assert!(outputs.contains("candidate.macro-code-rewrite"));
}

#[test]
fn runtime_api_self_representation_treats_apps_as_playback_surfaces_under_semantic_direction() {
  let run = eval_file(&fixture_path()).unwrap();
  let model = get(&run, "runtime-api-self-representation");
  assert_eq!(
    as_str(get(model, "id")),
    "self-representation.runtime-api-director-model.v1"
  );
  assert_eq!(
    as_str(get(model, "status")),
    "candidate-registered-not-runtime"
  );
  assert!(as_str(get(model, "core-claim")).contains("pre-authored callable playback surface"));
  assert!(as_str(get(model, "runtime-api-coding")).contains("abstract the trace back into meaning"));
  assert!(as_str(get(model, "ai-definition")).contains("decompose a goal sentence"));

  let lowering = get(model, "metaphor-lowering");
  assert!(
    as_str(get(lowering, "fixed-video-tape")).contains("pre-authored app/runtime behavior chunk")
  );
  assert!(as_str(get(lowering, "actor-performance")).contains("callable function or API action"));
  assert!(as_str(get(lowering, "director"))
    .contains("decomposes one intention into ordered API actor calls"));
  assert!(
    as_str(get(lowering, "freshly-shot-tape-effect")).contains("still becomes a playback surface")
  );

  let pipeline = string_set(get(model, "semantic-pipeline"));
  assert!(pipeline.contains("whole-meaning-intent"));
  assert!(pipeline.contains("split-meaning-into-action-roles"));
  assert!(pipeline.contains("select-runtime-api-actor"));
  assert!(pipeline.contains("play-call-under-owner-route"));
  assert!(pipeline.contains("abstract-execution-trace-to-meaning"));
  assert!(pipeline.contains("record-audit-and-route-candidate"));

  let forward = string_set(get(model, "forward-lane"));
  assert!(forward.contains("goal-sentence"));
  assert!(forward.contains("semantic-decomposition"));
  assert!(forward.contains("real-time-command-generation"));
  assert!(forward.contains("runtime-api-call"));

  let reverse = string_set(get(model, "reverse-lane"));
  assert!(reverse.contains("decomposed-command-trace"));
  assert!(reverse.contains("abstract-meaning"));
  assert!(reverse.contains("metaphorical-intuition"));
  assert!(reverse.contains("correct-meaning-expression"));

  let boundaries = string_set(get(model, "hard-boundaries"));
  assert!(boundaries.contains("api-call-is-played-action-not-understanding"));
  assert!(boundaries.contains("llm-director-output-is-candidate-not-owner"));
  assert!(boundaries.contains("runtime-api-coding-is-semantic-splitting-plus-owner-route"));
  assert!(boundaries.contains("forward-command-generation-needs-reverse-meaning-audit"));
  assert!(boundaries.contains("reverse-abstraction-is-representation-not-proof"));
  assert!(boundaries.contains("fixed-app-tape-is-not-pnix-cognition"));

  let candidates = string_set(get(model, "future-candidates"));
  assert!(candidates.contains("jarvis-style-app-director"));
  assert!(candidates.contains("meaning-to-api-actor-splitting"));
  assert!(candidates.contains("command-trace-to-meaning-abstraction"));
  assert!(candidates.contains("runtime-ext-library-flattening"));
  assert!(candidates.contains("domain-api-call-choreography"));
}

#[test]
fn discovered_functions_become_split_merge_tool_candidates_and_short_path_algorithm_candidates() {
  let run = eval_file(&fixture_path()).unwrap();
  let toolbox = get(&run, "discovered-function-toolbox");
  assert_eq!(
    as_str(get(toolbox, "id")),
    "toolbox.discovered-function-split-merge.v1"
  );
  assert_eq!(
    as_str(get(toolbox, "status")),
    "candidate-registered-not-runtime"
  );
  assert!(as_str(get(toolbox, "core-claim")).contains("semantic splitting, trace merging"));
  assert!(
    as_str(get(toolbox, "ai-operation-model")).contains("decompose a goal, select candidate tools")
  );
  assert_eq!(
    as_str(get(toolbox, "selected-algorithm-status")),
    "candidate-selected-after-replay-not-installed-runtime"
  );

  let sources = string_set(get(toolbox, "tool-candidate-sources"));
  assert!(sources.contains("role/need/Held emissions"));
  assert!(sources.contains("self-capability map rows"));
  assert!(sources.contains("unknown-world fast-path hypotheses"));
  assert!(sources.contains("macro-authoring self-extension candidates"));

  let uses = string_set(get(toolbox, "split-merge-uses"));
  assert!(uses.contains("split-goal-meaning-into-tool-roles"));
  assert!(uses.contains("select-candidate-tool-for-each-role"));
  assert!(uses.contains("compose-tool-route"));
  assert!(uses.contains("merge-command-trace-into-abstract-meaning"));
  assert!(uses.contains("emit-new-Held-need-or-tool-candidate"));

  let selection = string_set(get(toolbox, "short-path-selection-loop"));
  assert!(selection.contains("observe-many-step-route"));
  assert!(selection.contains("propose-short-path-candidate"));
  assert!(selection.contains("bind-required-tool-candidates"));
  assert!(selection.contains("compare-replay-determinism-held-loss-and-measurement"));
  assert!(selection.contains("select-as-algorithm-candidate-or-Held"));

  let boundaries = string_set(get(toolbox, "hard-boundaries"));
  assert!(boundaries.contains("candidate-tool-is-not-installed-api"));
  assert!(boundaries.contains("candidate-tool-selection-is-not-owner-switch"));
  assert!(boundaries.contains("short-path-selection-needs-replay-and-Held-loss-proof"));
  assert!(boundaries.contains("split-merge-toolbox-cannot-erase-negative-evidence"));
  assert!(boundaries.contains("autonomous-operation-is-gated-tool-composition-not-policy-mutation"));

  let candidates = string_set(get(toolbox, "future-candidates"));
  assert!(candidates.contains("semantic-split-merge-planner"));
  assert!(candidates.contains("candidate-tool-ranking"));
  assert!(candidates.contains("short-path-algorithm-chooser"));
  assert!(candidates.contains("self-capability-composition-map"));
}

#[test]
fn exploration_loop_is_evaluator_first_and_registers_next_frontier() {
  let run = eval_file(&fixture_path()).unwrap();
  let steps = attrs_by_key(get(&run, "exploration-loop"), "step");
  assert_eq!(steps.len(), 11);
  for id in [
    "loop.1.question",
    "loop.2.surface-inventory",
    "loop.3.evaluator-run",
    "loop.4.six-layer-fold",
    "loop.5.emit",
    "loop.6.compare",
    "loop.7.fast-path-hypothesis",
    "loop.8.measure",
    "loop.9.macro-authoring",
    "loop.10.register",
    "loop.11.gate-next",
  ] {
    assert!(steps.contains_key(id), "missing exploration step `{id}`");
  }
  assert!(
    as_str(get(steps.get("loop.3.evaluator-run").unwrap(), "action")).contains("p-puck wrapper")
  );
  assert!(as_str(get(
    steps.get("loop.7.fast-path-hypothesis").unwrap(),
    "action"
  ))
  .contains("mathematical intuition"));
  assert!(as_str(get(steps.get("loop.8.measure").unwrap(), "action"))
    .contains("baseline and candidate algorithms"));
  assert!(
    as_str(get(steps.get("loop.9.macro-authoring").unwrap(), "action"))
      .contains("macro-code candidates")
  );
  assert!(
    as_str(get(steps.get("loop.11.gate-next").unwrap(), "action")).contains("next scoped frontier")
  );
}

#[test]
fn harness_technique_registry_contains_load_bearing_methods() {
  let run = eval_file(&fixture_path()).unwrap();
  let techniques = attrs_by_key(get(&run, "harness-techniques"), "id");
  assert_eq!(techniques.len(), 17);
  for id in [
    "technique.evaluator-first-receipt",
    "technique.specimen-before-replacement",
    "technique.six-layer-fold-observation",
    "technique.role-need-held-emission",
    "technique.reverse-replay-delta",
    "technique.fixture-local-mutation-loop",
    "technique.negative-held-preservation",
    "technique.p-puck-wrapper-audit",
    "technique.bootstrap-overclaim-correction",
    "technique.application-candidate-registration",
    "technique.one-step-fast-path-hypothesis",
    "technique.scientific-concept-coding",
    "technique.performance-measurement-algorithm-registry",
    "technique.slow-path-self-optimization-loop",
    "technique.macro-authoring-self-extension",
    "technique.host-shrink-after-owner-proof",
    "technique.external-solver-benchmark-deferral",
  ] {
    assert!(techniques.contains_key(id), "missing technique `{id}`");
    assert!(!as_bool(get(
      techniques.get(id).unwrap(),
      "installed-runtime"
    )));
  }
}

#[test]
fn technique_effects_register_future_candidates_without_runtime_install() {
  let run = eval_file(&fixture_path()).unwrap();
  let techniques = attrs_by_key(get(&run, "harness-techniques"), "id");

  let role_emit = techniques
    .get("technique.role-need-held-emission")
    .expect("role technique");
  let candidates = string_set(get(role_emit, "future-candidates"));
  assert!(candidates.contains("lift-query-emit runtime owner or host removal proof"));
  assert!(candidates.contains("held-aware theorem store"));

  let host_shrink = techniques
    .get("technique.host-shrink-after-owner-proof")
    .expect("host shrink technique");
  assert!(as_str(get(host_shrink, "use")).contains("remove old host code after owner proof"));
  assert!(!as_bool(get(host_shrink, "installed-runtime")));

  let fast_path = techniques
    .get("technique.one-step-fast-path-hypothesis")
    .expect("fast path technique");
  assert!(as_str(get(fast_path, "capability-effect")).contains("replayable candidate evidence"));
  let fast_candidates = string_set(get(fast_path, "future-candidates"));
  assert!(fast_candidates.contains("many-step-to-one-step route fold"));
  assert!(fast_candidates.contains("domain API direct semantic route"));
  assert!(!as_bool(get(fast_path, "installed-runtime")));

  let scientific = techniques
    .get("technique.scientific-concept-coding")
    .expect("scientific concept coding technique");
  assert!(as_str(get(scientific, "capability-effect")).contains("self-insight candidates"));
  let scientific_candidates = string_set(get(scientific, "future-candidates"));
  assert!(scientific_candidates.contains("physics model fold receipts"));
  assert!(scientific_candidates.contains("biological system analogy receipts"));

  let measurement = techniques
    .get("technique.performance-measurement-algorithm-registry")
    .expect("measurement technique");
  assert!(as_str(get(measurement, "capability-effect")).contains("baseline/candidate evidence"));
  let measurement_candidates = string_set(get(measurement, "future-candidates"));
  assert!(measurement_candidates.contains("fast-path benchmark suite"));
  assert!(measurement_candidates.contains("semantic route measurement"));

  let self_opt = techniques
    .get("technique.slow-path-self-optimization-loop")
    .expect("self optimization technique");
  assert!(as_str(get(self_opt, "capability-effect")).contains("without automatic self-mutation"));
  let self_opt_candidates = string_set(get(self_opt, "future-candidates"));
  assert!(self_opt_candidates.contains("route shortening candidate"));
  assert!(self_opt_candidates.contains("proof reuse candidate"));

  let macro_authoring = techniques
    .get("technique.macro-authoring-self-extension")
    .expect("macro authoring technique");
  assert!(as_str(get(macro_authoring, "capability-effect")).contains("controlled self-extension"));
  let macro_candidates = string_set(get(macro_authoring, "future-candidates"));
  assert!(macro_candidates.contains("macro builder candidate"));
  assert!(macro_candidates.contains("macro code rewrite candidate"));
  assert!(!as_bool(get(macro_authoring, "installed-runtime")));
}

#[test]
fn claim_proof_floor_rejects_prose_green_tests_and_old_name_matches() {
  let run = eval_file(&fixture_path()).unwrap();
  let floor = get(&run, "claim-proof-floor");
  assert_eq!(as_str(get(floor, "id")), "gate.discovery-claim-proof-floor");

  let minimum = string_set(get(floor, "discovery-claim-minimum"));
  for expected in [
    "actual .px receipt evaluates",
    "constitution gate is visible",
    "negative Held / blocked shortcuts are explicit",
    "fast-path claim includes replay/delta/Held evidence",
    "intuition/hypothesis is marked as candidate, not proof",
    "scientific concept is lowered to receipt/model/proof obligation",
    "performance claim includes baseline/candidate measurement",
    "self-optimization claim includes bottleneck attribution and before-after replay",
    "macro-authoring self-extension claim includes authoring receipt and fixture-local evaluation",
    "future application candidates are registered without install claim",
    "fresh p-puck current-cut proof is wrapper evidence, not replay or boot",
    "bootstrap status remains honest",
  ] {
    assert!(minimum.contains(expected), "missing minimum `{expected}`");
  }

  let not_enough = string_set(get(floor, "not-enough"));
  assert!(not_enough.contains("agent prose"));
  assert!(not_enough.contains("green test without semantic receipt"));
  assert!(not_enough.contains("old function name match"));
  assert!(not_enough.contains("shortcut intuition without replay"));
  assert!(not_enough.contains("mathematical intuition without proof obligation"));
  assert!(not_enough.contains("scientific metaphor without executable receipt"));
  assert!(not_enough.contains("single timing number without baseline"));
  assert!(not_enough.contains("slow-path telemetry used as policy mutation"));
  assert!(not_enough.contains("macro code generated without authoring receipt"));
  assert!(not_enough.contains("generated macro code without owner-gated replay"));
  assert!(not_enough.contains("command splitting without meaning representation"));
}

#[test]
fn macro_and_generated_ontology_are_separate_core_surfaces_for_future_apis() {
  let run = eval_file(&fixture_path()).unwrap();
  let pair = get(&run, "macro-ontology-pair");
  assert_eq!(
    as_str(get(pair, "id")),
    "pair.meta-circular-tesseract-macro-and-generated-ontology"
  );
  assert!(
    as_str(get(pair, "tesseract-macro-role")).contains("generative fold / metaInterpret principle")
  );
  assert!(
    as_str(get(pair, "generated-ontology-role")).contains("API roles, needs, Held boundaries")
  );

  let reasons = string_set(get(pair, "why-both-are-core"));
  assert!(reasons.contains(
    "math/domain/API libraries later depend on these discovered roles, not on LLM prose"
  ));
  assert!(
    reasons.contains("meaning-based calls require ontology routing plus owner/gate/audit proof")
  );

  let separation = string_set(get(pair, "strict-separation"));
  assert!(separation.contains("macro-is-not-a-library-api-by-itself"));
  assert!(separation.contains("semantic-call-is-not-raw-function-call"));
  assert!(separation.contains("runtime-api-playback-is-not-cognition-by-itself"));
  assert!(separation.contains("candidate-toolbox-is-not-installed-runtime"));
}

#[test]
fn detailed_protocols_refine_prior_constitution_without_replacing_owner() {
  let run = eval_file(&fixture_path()).unwrap();
  let layering = get(&run, "constitutional-layering");
  assert_eq!(
    as_str(get(layering, "id")),
    "constitution.layering.detail-protocols-under-tesseract-owner"
  );
  assert_eq!(
    as_str(get(layering, "base-owner")),
    "stdlib/lib/gate/tesseract-constitution.px"
  );
  assert!(as_bool(get(
    layering,
    "replaces-vague-prior-guard-language"
  )));
  assert!(!as_bool(get(layering, "replaces-base-constitution-owner")));
  assert!(as_str(get(layering, "relation")).contains("detailed sub-constitutional protocols"));

  let protocols = string_set(get(layering, "detailed-protocols"));
  assert!(protocols.contains("unknown-world evaluator-first discovery loop"));
  assert!(protocols.contains("discovery claim proof floor"));
  assert!(protocols.contains("future semantic API call boundary"));
  assert!(protocols.contains("scientific-concept coding doctrine"));
  assert!(protocols.contains("macro-authoring self-extension candidate lane"));
  assert!(protocols.contains("runtime API director self-representation"));
  assert!(protocols.contains("forward/reverse meaning split representation"));
  assert!(protocols.contains("discovered function split/merge toolbox"));
  assert!(protocols.contains("host-shrink-after-owner-proof rule"));
  assert!(protocols.contains("macro-only boot manifest before boot execution rule"));

  let forbidden = string_set(get(layering, "forbidden-readings"));
  assert!(forbidden.contains("sub-constitution-replaces-owner-law"));
  assert!(forbidden.contains("application-candidate-equals-installed-api"));
  assert!(forbidden.contains("scientific-metaphor-equals-implementation"));
  assert!(forbidden.contains("macro-authoring-candidate-equals-runtime-self-modification"));
  assert!(forbidden.contains("api-playback-equals-cognition"));
  assert!(forbidden.contains("command-generation-without-meaning-abstraction-equals-AI"));
  assert!(forbidden.contains("candidate-tool-selection-equals-owner-switch"));
}

#[test]
fn application_candidate_registry_links_discoveries_to_future_work() {
  let run = eval_file(&fixture_path()).unwrap();
  let registry = attrs_by_key(get(&run, "application-candidate-registry"), "family");
  for family in [
    "math-kernel",
    "query-kernel",
    "domain-library-generation",
    "api-library-semantics",
    "runtime-api-director-self-representation",
    "discovered-function-split-merge-toolbox",
    "fast-path-discovery",
    "scientific-domain-kernels",
    "self-optimization",
    "macro-authoring-self-extension",
    "p-puck",
    "host-shrink",
  ] {
    let row = registry.get(family).expect("candidate family");
    if family == "host-shrink" {
      assert_eq!(
        as_str(get(row, "status")),
        "semantic-owner-proof-present-candidate-registered"
      );
      assert!(as_str(get(row, "current-proof-gap")).contains("runner owner is present"));
      assert!(as_str(get(row, "current-proof-gap")).contains("bounded replay strategy is present"));
      assert!(
        as_str(get(row, "current-proof-gap")).contains("regression corpus retention is present")
      );
      assert!(as_str(get(row, "current-proof-gap")).contains("bootstrap audit update is present"));
      assert!(as_str(get(row, "current-proof-gap")).contains("compare-after-boot proof is present"));
      assert!(as_str(get(row, "current-proof-gap")).contains("target-delete preflight is present"));
      assert!(
        as_str(get(row, "current-proof-gap")).contains("target-specific delete proof is present")
      );
      assert!(
        as_str(get(row, "current-proof-gap")).contains("fresh p-puck current-cut proof is present")
      );
      assert!(as_str(get(row, "current-proof-gap")).contains("bounded replay execution is present"));
      assert!(as_str(get(row, "current-proof-gap"))
        .contains("macro-only boot execution proof is present"));
      assert!(as_str(get(row, "current-proof-gap"))
        .contains("bounded macro-only runtime owner proof is present"));
      assert!(as_str(get(row, "current-proof-gap"))
        .contains("bounded macro-only semantic owner proof is present"));
      assert!(as_str(get(row, "current-proof-gap"))
        .contains("host-removal execution proof and global runtime install remain open"));
      assert!(string_set(get(row, "candidates")).contains("macro-only-boot-execution-attempt"));
      assert!(string_set(get(row, "candidates")).contains("macro-only-boot-runner-owner"));
      assert!(string_set(get(row, "candidates")).contains("macro-only-bounded-replay-strategy"));
      assert!(string_set(get(row, "candidates")).contains("macro-only-regression-corpus-retention"));
      assert!(string_set(get(row, "candidates")).contains("macro-only-bootstrap-audit-update"));
      assert!(string_set(get(row, "candidates")).contains("macro-only-compare-after-boot"));
      assert!(string_set(get(row, "candidates")).contains("macro-only-target-delete-preflight"));
      assert!(
        string_set(get(row, "candidates")).contains("macro-only-target-specific-delete-proof")
      );
      assert!(string_set(get(row, "candidates")).contains("macro-only-fresh-p-puck-current-cut"));
      assert!(
        string_set(get(row, "candidates")).contains("macro-only-bounded-replay-execution-proof")
      );
      assert!(string_set(get(row, "candidates")).contains("macro-only-boot-execution-proof"));
      assert!(string_set(get(row, "candidates")).contains("macro-only-runtime-owner-proof"));
      assert!(string_set(get(row, "candidates")).contains("macro-only-semantic-owner-proof"));
    } else {
      assert_eq!(as_str(get(row, "status")), "candidate-registered");
    }
    assert!(!as_list(get(row, "candidates")).is_empty());
    assert!(as_str(get(row, "current-proof-gap")).len() > 10);
  }

  let math = registry.get("math-kernel").unwrap();
  let math_candidates = string_set(get(math, "candidates"));
  assert!(math_candidates.contains("held-aware-theorem-store"));
  assert!(math_candidates.contains("reverse-turn-proof-separation"));

  let api = registry.get("api-library-semantics").unwrap();
  let api_candidates = string_set(get(api, "candidates"));
  assert!(api_candidates.contains("meaning-connected-api-call"));
  assert!(api_candidates.contains("meaning-to-api-actor-splitting"));
  assert!(api_candidates.contains("audit-replayable-api-binding"));

  let runtime_director = registry
    .get("runtime-api-director-self-representation")
    .unwrap();
  let runtime_director_candidates = string_set(get(runtime_director, "candidates"));
  assert!(runtime_director_candidates.contains("jarvis-style-app-director"));
  assert!(runtime_director_candidates.contains("forward-goal-to-command-route"));
  assert!(runtime_director_candidates.contains("reverse-command-trace-to-meaning-route"));
  assert!(runtime_director_candidates.contains("semantic-ui-action-route"));
  assert!(runtime_director_candidates.contains("domain-api-call-choreography"));

  let toolbox = registry
    .get("discovered-function-split-merge-toolbox")
    .unwrap();
  let toolbox_candidates = string_set(get(toolbox, "candidates"));
  assert!(toolbox_candidates.contains("semantic-split-merge-planner"));
  assert!(toolbox_candidates.contains("candidate-tool-ranking"));
  assert!(toolbox_candidates.contains("short-path-algorithm-chooser"));
  assert!(toolbox_candidates.contains("self-capability-composition-map"));

  let fast = registry.get("fast-path-discovery").unwrap();
  let fast_candidates = string_set(get(fast, "candidates"));
  assert!(fast_candidates.contains("many-step-to-one-step-fold"));
  assert!(fast_candidates.contains("intuition-to-proof-obligation"));
  assert!(fast_candidates.contains("Held-preserving-fast-path-rejection"));

  let science = registry.get("scientific-domain-kernels").unwrap();
  let science_candidates = string_set(get(science, "candidates"));
  assert!(science_candidates.contains("mathematical-structure-kernel"));
  assert!(science_candidates.contains("physical-system-model-kernel"));
  assert!(science_candidates.contains("biological-system-model-kernel"));

  let self_opt = registry.get("self-optimization").unwrap();
  let self_opt_candidates = string_set(get(self_opt, "candidates"));
  assert!(self_opt_candidates.contains("measurement-algorithm-registry"));
  assert!(self_opt_candidates.contains("bottleneck-attribution"));
  assert!(self_opt_candidates.contains("before-after-benchmark"));

  let macro_authoring = registry.get("macro-authoring-self-extension").unwrap();
  let macro_candidates = string_set(get(macro_authoring, "candidates"));
  assert!(macro_candidates.contains("macro-template-candidate"));
  assert!(macro_candidates.contains("macro-builder-candidate"));
  assert!(macro_candidates.contains("macro-code-rewrite-candidate"));

  let puck = registry.get("p-puck").unwrap();
  let puck_candidates = string_set(get(puck, "candidates"));
  assert!(puck_candidates.contains("fresh-current-cut-wrapper-proof"));
  assert!(
    as_str(get(puck, "current-proof-gap")).contains("not semantic owner or full receipt audit")
  );
}

#[test]
fn migration_context_inherits_bootstrap_and_legacy_matrix_boundaries() {
  let run = eval_file(&fixture_path()).unwrap();
  let context = get(&run, "migration-context");
  assert_eq!(as_i64(get(context, "legacy-extern-count")), 12);
  assert_eq!(as_i64(get(context, "legacy-externs-classified")), 12);
  assert_eq!(as_str(get(context, "lift-query-emit-phase")), "R7");
  assert_eq!(
    as_str(get(context, "lift-query-emit-next")),
    "runtime owner or host removal proof"
  );
  assert_eq!(
    as_str(get(context, "lift-query-emit-compat")),
    "tesseract-macro-ontology-r7-compat-archive-lift-query-emit-surface-triple"
  );
  assert_eq!(
    as_str(get(context, "lift-query-emit-compat-status")),
    "compat-retained-for-lift-query-emit-surface-triple"
  );
  assert_eq!(
    as_str(get(context, "host-removal-map")),
    "tesseract-macro-ontology-host-code-removal-map"
  );
  assert!(as_bool(get(context, "host-removal-map-written")));
  assert_eq!(
    as_i64(get(context, "host-removal-delete-ready-target-count")),
    0
  );
  assert_eq!(
    as_str(get(context, "macro-only-boot-manifest")),
    "tesseract-macro-ontology-macro-only-boot-manifest"
  );
  assert!(as_bool(get(context, "macro-only-boot-manifest-written")));
  assert_eq!(
    as_str(get(context, "macro-only-boot-execution-attempt")),
    "tesseract-macro-ontology-macro-only-boot-execution-attempt"
  );
  assert!(as_bool(get(context, "macro-only-boot-execution-attempted")));
  assert_eq!(
    as_str(get(context, "macro-only-boot-runner-owner")),
    "tesseract-macro-ontology-macro-only-boot-runner-owner"
  );
  assert!(as_bool(get(
    context,
    "macro-only-boot-runner-owner-present"
  )));
  assert_eq!(
    as_str(get(context, "bounded-replay-strategy")),
    "tesseract-macro-ontology-macro-only-bounded-replay-strategy"
  );
  assert!(as_bool(get(
    context,
    "bounded-full-graph-replay-strategy-present"
  )));
  assert_eq!(
    as_str(get(context, "regression-corpus-retention")),
    "tesseract-macro-ontology-macro-only-regression-corpus-retention"
  );
  assert!(as_bool(get(context, "regression-corpus-transfer-present")));
  assert_eq!(
    as_str(get(context, "bootstrap-audit-update")),
    "tesseract-macro-ontology-macro-only-bootstrap-audit-update"
  );
  assert!(as_bool(get(
    context,
    "bootstrap-status-audit-update-plan-present"
  )));
  assert_eq!(as_str(get(context, "runner-after-audit-status")), "Held");
  assert_eq!(as_i64(get(context, "runner-after-audit-missing-count")), 3);
  assert_eq!(
    as_str(get(context, "compare-after-boot-proof")),
    "tesseract-macro-ontology-macro-only-compare-after-boot"
  );
  assert!(as_bool(get(context, "compare-after-boot")));
  assert_eq!(as_str(get(context, "runner-after-compare-status")), "Held");
  assert_eq!(
    as_i64(get(context, "runner-after-compare-missing-count")),
    2
  );
  assert_eq!(
    as_str(get(context, "target-delete-preflight")),
    "tesseract-macro-ontology-macro-only-target-delete-preflight"
  );
  assert!(as_bool(get(context, "target-delete-preflight-present")));
  assert_eq!(
    as_str(get(context, "runner-after-preflight-status")),
    "Held"
  );
  assert_eq!(
    as_i64(get(context, "runner-after-preflight-missing-count")),
    2
  );
  assert_eq!(
    as_str(get(context, "target-specific-delete-proof")),
    "tesseract-macro-ontology-macro-only-target-specific-delete-proof"
  );
  assert!(as_bool(get(
    context,
    "target-specific-delete-proof-present"
  )));
  assert_eq!(
    as_str(get(context, "runner-after-target-proof-status")),
    "Held"
  );
  assert_eq!(
    as_i64(get(context, "runner-after-target-proof-missing-count")),
    1
  );
  assert_eq!(
    as_str(get(context, "fresh-p-puck-current-cut")),
    "tesseract-macro-ontology-macro-only-fresh-p-puck-current-cut"
  );
  assert!(as_bool(get(context, "fresh-p-puck-after-current-cut")));
  assert_eq!(
    as_str(get(context, "runner-after-fresh-puck-status")),
    "runner-ready-for-bounded-replay"
  );
  assert_eq!(
    as_i64(get(context, "runner-after-fresh-puck-missing-count")),
    0
  );
  assert!(as_bool(get(context, "ready-for-bounded-replay")));
  assert!(as_bool(get(context, "full-current-receipt-audit")));
  assert_eq!(
    as_str(get(context, "full-current-receipt-audit-receipt")),
    "tesseract-macro-ontology-macro-only-full-current-receipt-audit"
  );
  assert_eq!(
    as_i64(get(context, "full-current-receipt-audit-total-tests")),
    915
  );
  assert_eq!(
    as_i64(get(context, "full-current-receipt-audit-source-tracked")),
    18167
  );
  assert_eq!(
    as_i64(get(context, "full-current-receipt-audit-source-indexed")),
    18167
  );
  assert_eq!(
    as_str(get(context, "macro-only-boot-execution-proof")),
    "tesseract-macro-ontology-macro-only-boot-execution-proof"
  );
  assert!(as_bool(get(
    context,
    "macro-only-boot-execution-proof-present"
  )));
  assert_eq!(
    as_i64(get(context, "macro-only-boot-proof-total-tests")),
    931
  );
  assert_eq!(
    as_i64(get(context, "macro-only-boot-proof-source-tracked")),
    18172
  );
  assert_eq!(
    as_i64(get(context, "macro-only-boot-proof-source-indexed")),
    18172
  );
  assert_eq!(
    as_str(get(context, "macro-only-runtime-owner-proof")),
    "tesseract-macro-ontology-macro-only-runtime-owner-proof"
  );
  assert!(as_bool(get(
    context,
    "macro-only-runtime-owner-proof-present"
  )));
  assert_eq!(
    as_i64(get(context, "macro-only-runtime-owner-proof-total-tests")),
    947
  );
  assert_eq!(
    as_i64(get(
      context,
      "macro-only-runtime-owner-proof-source-tracked"
    )),
    18177
  );
  assert_eq!(
    as_i64(get(
      context,
      "macro-only-runtime-owner-proof-source-indexed"
    )),
    18177
  );
  assert_eq!(
    as_str(get(context, "macro-only-runtime-owner-scope")),
    "bounded-receipt-trajectory-owner"
  );
  assert_eq!(
    as_str(get(context, "macro-only-semantic-owner-proof")),
    "tesseract-macro-ontology-macro-only-semantic-owner-proof"
  );
  assert!(as_bool(get(
    context,
    "macro-only-semantic-owner-proof-present"
  )));
  assert_eq!(
    as_i64(get(context, "macro-only-semantic-owner-proof-total-tests")),
    963
  );
  assert_eq!(
    as_i64(get(
      context,
      "macro-only-semantic-owner-proof-source-tracked"
    )),
    18182
  );
  assert_eq!(
    as_i64(get(
      context,
      "macro-only-semantic-owner-proof-source-indexed"
    )),
    18182
  );
  assert_eq!(
    as_str(get(context, "macro-only-semantic-owner-scope")),
    "bounded-generated-ontology-semantic-owner"
  );
  assert!(as_bool(get(context, "semantic-owner")));
  assert_eq!(
    as_str(get(context, "bounded-replay-execution")),
    "tesseract-macro-ontology-macro-only-bounded-replay-execution"
  );
  assert!(as_bool(get(context, "bounded-replay-executed")));
  assert_eq!(as_i64(get(context, "bounded-replay-step-count")), 11);
  assert_eq!(
    as_str(get(context, "bounded-replay-semantic-delta-status")),
    "empty-or-held-only"
  );
  assert!(!as_bool(get(
    context,
    "boot-execution-attempt-boot-executed"
  )));
  assert!(as_bool(get(context, "boot-executed")));
  assert!(as_bool(get(context, "macro-only-runtime-owner-booted")));
  assert_eq!(
    as_str(get(context, "macro-only-boot-next")),
    "host removal proof or global runtime proof after bounded semantic owner"
  );
  assert_eq!(
    as_str(get(context, "host-removal-next")),
    "host removal execution proof after semantic-owner proof stays honest"
  );
  assert_eq!(
    as_i64(get(context, "p-puck-receipt-count-after-this-receipt")),
    44
  );
  assert!(as_bool(get(
    context,
    "p-puck-audit-fresh-after-current-cut"
  )));
  assert!(!as_bool(get(context, "p-puck-full-current-receipt-audit")));

  let bootstrap = get(context, "bootstrap-status");
  assert!(as_bool(get(bootstrap, "receipt-evaluated-macro-substrate")));
  assert!(as_bool(get(bootstrap, "macro-only-boot-manifest-written")));
  assert!(as_bool(get(
    bootstrap,
    "macro-only-boot-execution-attempted"
  )));
  assert!(as_bool(get(
    bootstrap,
    "macro-only-boot-runner-owner-present"
  )));
  assert!(as_bool(get(
    bootstrap,
    "bounded-full-graph-replay-strategy-present"
  )));
  assert!(as_bool(get(
    bootstrap,
    "regression-corpus-transfer-present"
  )));
  assert!(as_bool(get(
    bootstrap,
    "bootstrap-status-audit-update-plan-present"
  )));
  assert!(as_bool(get(bootstrap, "compare-after-boot")));
  assert!(as_bool(get(bootstrap, "target-delete-preflight-present")));
  assert!(as_bool(get(
    bootstrap,
    "target-specific-delete-proof-present"
  )));
  assert!(as_bool(get(bootstrap, "fresh-p-puck-after-current-cut")));
  assert!(as_bool(get(bootstrap, "ready-for-bounded-replay")));
  assert!(as_bool(get(bootstrap, "full-current-receipt-audit")));
  assert!(as_bool(get(bootstrap, "bounded-replay-executed")));
  assert!(as_bool(get(
    bootstrap,
    "macro-only-boot-execution-proof-present"
  )));
  assert!(as_bool(get(bootstrap, "boot-executed")));
  assert!(as_bool(get(
    bootstrap,
    "macro-only-runtime-owner-proof-present"
  )));
  assert!(as_bool(get(
    bootstrap,
    "macro-only-semantic-owner-proof-present"
  )));
  assert!(!as_bool(get(bootstrap, "new-engine-from-zero")));
  assert!(as_bool(get(bootstrap, "macro-only-runtime-owner-booted")));
  assert!(as_bool(get(bootstrap, "semantic-owner")));
  assert_eq!(
    as_str(get(bootstrap, "semantic-owner-scope")),
    "bounded-generated-ontology-semantic-owner"
  );
}

#[test]
fn top_level_protocol_is_not_runtime_install_or_owner_switch() {
  let run = eval_file(&fixture_path()).unwrap();
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
