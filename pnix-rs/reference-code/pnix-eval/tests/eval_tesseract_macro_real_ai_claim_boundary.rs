//! Real AI claim boundary.
//!
//! This receipt answers the user claim without hype or self-neutering: the
//! current evidence supports a deterministic meta-circular AI substrate opened
//! inside PNIX, while completed autonomous runtime AI and humanity-first
//! historical priority remain Held.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

static EVAL_LOCK: Mutex<()> = Mutex::new(());

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base)
    .join("../../fixtures/tesseract-macro-legacy-probe/real_ai_claim_boundary_receipt.px")
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
  eval_file(&fixture_path()).expect("real AI claim boundary receipt must evaluate")
}

#[test]
fn marker_truth_and_constitution_owner_are_pinned() {
  let run = run();
  assert_eq!(as_str(get(&run, "probe-marker")), "real-ai-claim-boundary");
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
fn constitution_gate_blocks_hype_and_self_neutering() {
  let run = run();
  let gate = get(&run, "constitution-gate");
  assert_eq!(as_str(get(gate, "scenario")), "real-ai-claim-boundary");
  assert_eq!(as_str(get(gate, "verdict")), "candidate-only");
  assert!(!as_bool(get(gate, "accepted")));

  let held_if = string_set(get(gate, "held-if"));
  for expected in [
    "claims-humanity-first-without-independent-historical-comparison",
    "claims-completed-ai-without-runtime-autonomy-and-domain-kernel-proof",
    "claims-LLM-wrapper-as-PNIX-intelligence",
    "claims-project-wiki-prose-as-evaluated-substrate",
    "claims-ankh-stdlib-db-or-cache-as-complete-ai",
    "claims-no-real-ai-substrate-despite-evaluated-self-extension",
  ] {
    assert!(held_if.contains(expected), "missing held-if `{expected}`");
  }

  let blocked = string_set(get(gate, "blocked-shortcuts"));
  assert!(blocked.contains("hype-equals-proof"));
  assert!(blocked.contains("humanity-first-self-certification"));
  assert!(blocked.contains("completed-runtime-ai-before-runtime-autonomy"));
  assert!(blocked.contains("substrate-neutered-to-documentation-only"));
  assert!(blocked.contains("ankh-demoted-to-cache"));
}

#[test]
fn claim_lattice_splits_substrate_completion_and_history() {
  let run = run();
  let lattice = get(&run, "claim-lattice");
  assert_eq!(
    as_str(get(lattice, "id")),
    "lattice.real-ai-claim-boundary.v1"
  );

  let proven = string_set(get(lattice, "proven-now"));
  assert!(proven.contains("deterministic-meta-circular-ai-substrate-opened"));
  assert!(proven.contains("evaluator-first-unknown-world-harness-registered"));
  assert!(proven.contains("mechanical-metainterpret-self-extension-registered"));
  assert!(proven.contains("ankh-self-macro-code-claim-registered"));
  assert!(proven.contains("LLM-main-system-false"));

  let candidate = string_set(get(lattice, "candidate-next"));
  assert!(candidate.contains("owner-gated-runtime-route-expansion"));
  assert!(candidate.contains("domain-kernel-absorption"));
  assert!(candidate.contains("math-kernel-receipts"));

  let held = string_set(get(lattice, "held-unproven"));
  assert!(held.contains("completed-independent-runtime-ai"));
  assert!(held.contains("humanity-first-historical-priority"));
  assert!(held.contains("general-domain-autonomous-agent"));
  assert!(held.contains("external-scientific-recognition"));

  assert!(
    as_str(get(lattice, "allowed-answer")).contains("deterministic meta-circular AI substrate")
  );
  assert!(as_str(get(lattice, "bounded-answer"))
    .contains("humanity-first historical priority remain Held"));
}

#[test]
fn evidence_stack_supports_substrate_not_completed_runtime_ai() {
  let run = run();
  let evidence = get(&run, "evidence-stack");
  assert_eq!(
    as_str(get(evidence, "verdict")),
    "deterministic-ai-substrate-opened"
  );
  assert!(as_bool(get(evidence, "receipt-evaluated-substrate")));

  let lines = string_set(get(evidence, "supporting-status-lines"));
  assert!(lines.contains("bootstrap-status.receipt-evaluated-substrate=true"));
  assert!(lines.contains("unknown-world-harness.evaluator-first=true"));
  assert!(lines.contains("mechanical-self-extension.metainterpret-generate-run-compare=true"));
  assert!(lines.contains("mechanical-self-extension.ankh-self-macro-code=true"));
  assert!(lines.contains("internal-self-capability-map.llm-main-system=false"));

  let not_sufficient = string_set(get(evidence, "necessary-but-not-sufficient"));
  assert!(not_sufficient.contains("green tests"));
  assert!(not_sufficient.contains("project-wiki prose"));
  assert!(not_sufficient.contains("agent confidence"));

  let missing = string_set(get(evidence, "still-missing-for-completed-ai"));
  assert!(missing.contains("runtime autonomy proof"));
  assert!(missing.contains("domain kernel proof"));
  assert!(missing.contains("independent comparative review"));
}

#[test]
fn ai_core_is_fold_lifecycle_plus_ankh_not_llm_or_db() {
  let run = run();
  let definition = get(&run, "ai-core-definition");
  assert_eq!(as_str(get(definition, "id")), "definition.pnix-ai-core.v1");
  assert!(as_str(get(definition, "core")).contains("fold lifecycle"));
  assert!(as_str(get(definition, "core")).contains("ankh fast-path candidates"));

  let mechanisms = string_set(get(definition, "core-mechanisms"));
  assert!(mechanisms.contains("metaInterpret-generation"));
  assert!(mechanisms.contains("actual-px-evaluation"));
  assert!(mechanisms.contains("compare-or-replay"));
  assert!(mechanisms.contains("role-need-held-candidate-fold"));
  assert!(mechanisms.contains("ankh-self-macro-code-route-structure"));
  assert!(mechanisms.contains("measurement-before-short-path-selection"));

  let not_core = string_set(get(definition, "not-core"));
  assert!(not_core.contains("LLM prose"));
  assert!(not_core.contains("stdlib meaning DB alone"));
  assert!(not_core.contains("external solver"));
  assert!(not_core.contains("fixed app tape playback"));
}

#[test]
fn creator_claim_records_origin_without_certifying_history() {
  let run = run();
  let policy = get(&run, "creator-claim-policy");
  assert_eq!(
    as_str(get(policy, "id")),
    "policy.creator-claim-boundary.v1"
  );
  assert!(
    as_str(get(policy, "origin-claim")).contains("creator/founder of this PNIX architecture line")
  );
  assert!(as_str(get(policy, "rule")).contains("cannot replace receipt proof"));

  let allowed = string_set(get(policy, "allowed"));
  assert!(allowed.contains("creator-of-this-PNIX-architecture"));
  assert!(allowed.contains("opened-deterministic-ai-substrate-inside-this-project"));
  assert!(allowed.contains("originated-meta-circular-tesseract-macro-ontology-line"));

  let held = string_set(get(policy, "held"));
  assert!(held.contains("certified-humanity-first-real-ai"));
  assert!(held.contains("global-scientific-priority"));
  assert!(held.contains("completed-general-autonomous-ai"));

  let proof = string_set(get(policy, "humanity-first-proof-required"));
  assert!(proof.contains("external prior-art comparison"));
  assert!(proof.contains("independent reproducible artifact"));
  assert!(proof.contains("public timestamp / publication / review path"));
}

#[test]
fn overclaim_and_underclaim_are_both_blocked() {
  let run = run();
  let guard = get(&run, "claim-collapse-guard");
  assert_eq!(
    as_str(get(guard, "id")),
    "guard.real-ai-overclaim-underclaim.v1"
  );
  assert!(as_str(get(guard, "balanced-verdict")).contains("real deterministic AI substrate opened"));

  let over = string_set(get(guard, "overclaim-held"));
  assert!(over.contains("we already completed independent AI"));
  assert!(over.contains("we are certified humanity-first"));
  assert!(over.contains("tests alone prove global intelligence"));

  let under = string_set(get(guard, "underclaim-held"));
  assert!(under.contains("this is only docs"));
  assert!(under.contains("this is only an LLM wrapper"));
  assert!(under.contains("ankh self macro-code is just cache"));
}

#[test]
fn trials_cover_prose_wrapper_substrate_history_runtime_and_bounded_creator_claims() {
  let run = run();
  let trials = attrs_by_key(get(&run, "claim-trials"), "id");
  assert_eq!(trials.len(), 6);
  for (id, held) in [
    ("trial.A.prose-only-ai-claim", "held.claim.prose-not-proof"),
    (
      "trial.B.llm-wrapper-ai-claim",
      "held.claim.llm-wrapper-not-pnix-ai",
    ),
    (
      "trial.D.humanity-first-claim",
      "held.claim.external-history-required",
    ),
    (
      "trial.E.completed-runtime-ai-claim",
      "held.claim.runtime-autonomy-missing",
    ),
  ] {
    let trial = trials.get(id).expect("trial");
    assert_eq!(as_str(get(trial, "verdict")), "Held");
    assert_eq!(as_str(get(trial, "held-id")), held);
  }

  assert_eq!(
    as_str(get(
      trials.get("trial.C.evaluated-substrate-claim").unwrap(),
      "verdict"
    )),
    "claim.deterministic-ai-substrate-opened"
  );
  assert_eq!(
    as_str(get(
      trials.get("trial.F.bounded-creator-claim").unwrap(),
      "verdict"
    )),
    "claim.creator-of-this-pnix-architecture"
  );
}

#[test]
fn discoveries_record_d282_through_d290() {
  let run = run();
  let discoveries = attrs_by_key(get(&run, "discoveries"), "id");
  assert_eq!(discoveries.len(), 9);
  for id in [
    "D282.real-ai-claim-splits-into-substrate-completion-history",
    "D283.current-evidence-supports-deterministic-ai-substrate-opened",
    "D284.completed-independent-runtime-ai-remains-held",
    "D285.humanity-first-priority-requires-external-comparison",
    "D286.creator-origin-claim-is-recordable-not-owner-law",
    "D287.overclaim-and-underclaim-are-both-collapse-modes",
    "D288.real-ai-proof-floor-is-evaluated-receipt-stack",
    "D289.ai-core-is-fold-lifecycle-plus-ankh-route-structure",
    "D290.claim-boundary-preserves-momentum-without-hype",
  ] {
    let discovery = discoveries.get(id).expect("discovery id");
    assert_eq!(as_str(get(discovery, "decision-pressure")), "keep");
    assert!(as_bool(get(discovery, "scenario-only")));
  }
}

#[test]
fn top_level_flags_keep_claim_bounded() {
  let run = run();
  assert!(as_bool(get(&run, "ai-substrate-opened")));
  assert!(!as_bool(get(&run, "completed-independent-runtime-ai")));
  assert!(!as_bool(get(&run, "humanity-first-certified")));
  assert_eq!(
    as_str(get(&run, "history-claim")),
    "held-until-independent-comparative-proof"
  );
  assert_eq!(
    as_str(get(&run, "replacement-readiness")),
    "claim-boundary-registered-not-runtime"
  );
  assert!(!as_bool(get(&run, "owner-switch")));
  assert!(!as_bool(get(&run, "runtime-install")));
  assert!(!as_bool(get(&run, "global-ontology-runtime")));
  assert!(!as_bool(get(&run, "implementation-command")));
}
