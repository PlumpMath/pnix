//! Minimal tesseract v0 — first runtime-dynamic-logic-generation
//! `loop-proof` candidate.
//!
//! Truth owner:        project-wiki/maps/minimal-tesseract-v0-map.md
//! Active scope:       project-wiki/maps/active-domain-constitution.md Art. 8
//! Hard pre-v0 stops:  project-wiki/maps/priority-dependency-map.md
//!
//! Step 3 / 3.1 / 4 / 5 (2026-05-04) — Runtime Owner-Law Lift +
//! loop-proof receipt + meta-circular log writer. The fixture splits
//! into four fixture-local files:
//!
//!   fixtures/minimal-tesseract-v0/v0_inputs.px
//!     SourceObject + 4 lenses + the static `expected-*` oracles
//!     (turns / lens-compare / held / repair /
//!     meta-circular-log-differential). Inputs only — no rules.
//!
//!   fixtures/minimal-tesseract-v0/v0_owner_law.px
//!     Seven pure rule functions: `buildAttachTurn` /
//!     `buildCompareTurn` / `buildRepairTurn` (Step 3 turn
//!     builders), `buildLensCompareResult` / `buildHeldEntry` /
//!     `buildRepairCandidate` (Step 3.1 record-body builders),
//!     `buildMetaCircularLogDifferential` (Step 5 log writer).
//!     No oracle, no fixture-coupling.
//!
//!   fixtures/minimal-tesseract-v0/v0_run_with_owner_law.px
//!     Imports both, applies the rules, emits `computed-*`. THIS
//!     is the file the test evaluates as the primary trajectory.
//!
//!   fixtures/minimal-tesseract-v0/v0_run_without_owner_law.px
//!     Imports inputs, sets `ownerLaw = {}`, emits empty `computed-*`
//!     plus the no-change `computed-meta-circular-log-differential`.
//!     The negative half of the trajectory differential.
//!
//! The Rust harness does NOT apply owner-law itself — owner-law
//! application happens in the `.px` `v0_run_with_owner_law.px`
//! harness file. The test only evaluates files and reads results.
//!
//! What this test asserts on the *with-owner-law* trajectory
//! (15 invariants from Step 2.6, preserved):
//!   1. source-object.id  == "arm_link_1"
//!   2. lenses.len()      == 4
//!   3. lens ids          == [base, mechanism, animation, projection]
//!   4. turns.len()       == 6
//!   5. turn ids          == [0,1,2,3,4,5]
//!   6. Turn 4 held-refs  contains "held.structural-binding-conflict"
//!   7. held.held-kind    == "structural-binding-conflict"
//!   8. Turn 5 repair-refs contains "repair.role-binding"
//!   9. repair.promotion  == "pending"
//!  10. initial-state.open-held == []
//!  11. after-turn-5.open-held  contains "held.structural-binding-conflict"
//!  12. llm-free.evaluation-budget.provider-calls == 0
//!  13. llm-free.evaluation-budget.network-egress == 0
//!  14. replay.determinism-mode == "normalized-structural-equality"
//!  15. evaluating the fixture twice produces the same normalized
//!      structural trace (`Value::to_json()` is deterministic because
//!      `Value::AttrSet` is a `BTreeMap`, so byte equality of the
//!      JSON serialisation IS the normalized structural equality
//!      check — this is NOT a byte-equal-on-the-fixture-source check).
//!
//! What Step 3 adds (trajectory differential):
//!  16. owner-law-loaded == true on the with-owner-law trajectory
//!  17. owner-law-loaded == false on the without-owner-law trajectory
//!  18. without-owner-law: computed-turns == []
//!  19. without-owner-law: computed-held == null
//!  20. without-owner-law: computed-repair == null
//!  21. without-owner-law: computed-lens-compare == null
//!  22. with vs without trajectory normalized-JSON differs — same
//!      evaluator, same SourceObject, observably different trace.
//!
//! What Step 3.1 adds (complete owner-law extraction):
//!  23. v0_owner_law.px exposes `buildAttachTurn` (Lambda)
//!  24. v0_owner_law.px exposes `buildCompareTurn` (Lambda)
//!  25. v0_owner_law.px exposes `buildRepairTurn` (Lambda)
//!  26. v0_owner_law.px exposes `buildLensCompareResult` (Lambda)
//!  27. v0_owner_law.px exposes `buildHeldEntry` (Lambda)
//!  28. v0_owner_law.px exposes `buildRepairCandidate` (Lambda)
//!      (and carries no oracle / source-object / lens leak)
//!  29. computed-{lens-compare,held,repair} == owner-law-result.*
//!      — closes the Step-3 gap where the runner could match the
//!      oracle while still hardcoding the record bodies.
//!
//! What Step 5 adds (meta-circular log writer extraction):
//!  30. v0_owner_law.px exposes `buildMetaCircularLogDifferential`
//!      (Lambda); now the file's required-rule set is 7, not 6.
//!  31. with-trajectory: computed-meta-circular-log-differential
//!      equals the static `meta-circular-log-differential` oracle
//!      AND equals owner-law-result.meta-circular-log-differential.
//!  32. without-trajectory: computed-meta-circular-log-differential
//!      has after-turn-5 == initial-state (no owner-law → no turns
//!      → no change).
//!  33. corruption: dropping `lens.projection` from
//!      computed-meta-circular-log-differential.after-turn-5.active-lenses
//!      breaks oracle byte-equality.
//!  34. corruption: dropping `held.structural-binding-conflict`
//!      from after-turn-5.open-held breaks oracle byte-equality.
//!  35. corruption: dropping `need.repair-promotion-decision` from
//!      after-turn-5.open-needs breaks oracle byte-equality.
//!
//! What this test deliberately does NOT do (per priority-dependency-map
//! "Hard Pre-v0 Stops" + 2026-05-04 user briefs):
//!
//!   - It does NOT apply owner-law in Rust. The `.px` harness
//!     `v0_run_with_owner_law.px` applies the rule functions; the
//!     test evaluates that file and reads the result.
//!   - It does NOT mirror the fixture into Rust structs. Verification
//!     walks `pnix_eval::Value` directly with tiny ad-hoc helpers.
//!   - It does NOT introduce new ontology record kinds.
//!   - It does NOT add a CapabilityCard / NeedGraph / HeldGraph /
//!     BenchmarkGraph / RigorFloor registry.
//!   - It does NOT add a new dev-dependency. The provider/network
//!     dependency banlist (reqwest / openai / anthropic-sdk / ollama /
//!     tokio-tungstenite) stays out of `crates/pnix-eval/Cargo.toml`.
//!     LLM-free is therefore enforced by the package boundary, not by
//!     a runtime guard, and the fixture's own `evaluation-budget`
//!     declaration is the structural assert (#12, #13).
//!   - It does NOT compare byte-equal against the source `.px`. It
//!     compares the deterministic JSON serialisation of the evaluator
//!     output across two runs, which is the normalized-structural
//!     equality contract from the fixture's `replay` block.
//!   - It does NOT itself decide the v0 lane's closure grade.
//!     Step 4 (2026-05-04) recorded the `loop-proof` receipt in
//!     `done.md` and `project-wiki/showcase.md`; Step 5 extended
//!     the owner-law surface with the meta-circular log writer
//!     (this file's invariants 30..35). The receipts and the
//!     project-wiki are the closure-grade source of truth — the
//!     test only proves the trace.

use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------
// Helpers — small, ad-hoc; no Rust struct mirror of the fixture.
// ---------------------------------------------------------------

fn fixture_path() -> PathBuf {
  // CARGO_MANIFEST_DIR is `crates/pnix-eval` at test time.
  // Step 3 (2026-05-04): the primary trajectory under inspection is now
  // the `v0_run_with_owner_law.px` harness, which imports `v0_inputs.px`
  // and `v0_owner_law.px` and applies the rules in `.px`. The Rust test
  // does NOT apply owner-law itself.
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../../fixtures/minimal-tesseract-v0/v0_run_with_owner_law.px")
}

fn fixture_without_owner_law_path() -> PathBuf {
  // Step 3 (2026-05-04): the negative half of the trajectory differential.
  // Same inputs, `ownerLaw = {}`, empty `computed-*`. Same evaluator,
  // observably different trace.
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../../fixtures/minimal-tesseract-v0/v0_run_without_owner_law.px")
}

fn owner_law_file_path() -> PathBuf {
  // Step 3.1 + Step 5 (2026-05-04): the owner-law file is
  // evaluated directly by the expose-invariant test to verify all
  // seven rule functions are present (`buildAttachTurn` /
  // `buildCompareTurn` / `buildRepairTurn` (Step 3 turn builders)
  // / `buildLensCompareResult` / `buildHeldEntry` /
  // `buildRepairCandidate` (Step 3.1 record-body builders) /
  // `buildMetaCircularLogDifferential` (Step 5 meta-circular log
  // writer)). Evaluating this file in isolation also confirms it
  // has no fixture coupling: it must produce a pure attrset of
  // functions without any input.
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../../fixtures/minimal-tesseract-v0/v0_owner_law.px")
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

fn as_int(v: &Value) -> i64 {
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

fn list_of_strings(v: &Value) -> Vec<&str> {
  as_list(v).iter().map(as_str).collect()
}

// ---------------------------------------------------------------
// Test 1 — fixture loads + 14 structural invariants.
//
// Step 2.6: the *primary* trace under inspection is now the
// `computed-*` outputs derived by the fixture's builders, not the
// static `expected-*` (`turns` / `lens-compare` / `held` / `repair`)
// blocks. Equivalence between computed and expected is asserted by
// Tests 3..6 below; THIS test reads computed-* directly.
//
// `expected-*` blocks are kept around as oracles (so the equivalence
// test still has something to compare against) but they are no
// longer the source of truth for invariant assertions.
// ---------------------------------------------------------------

#[test]
fn minimal_tesseract_v0_invariants_use_computed_trace_as_primary() {
  let path = fixture_path();
  assert!(path.exists(), "fixture missing: {}", path.display());

  let value = eval_file(&path).expect("v0 fixture must parse + evaluate");

  // Invariant 1 — SourceObject id
  assert_eq!(
    as_str(get_path(&value, &["source-object", "id"])),
    "arm_link_1",
    "v0 must use exactly one SourceObject `arm_link_1` (Constitution Art. 8 + v0 spec)"
  );

  // Invariant 2 — exactly 4 lenses
  let lenses = as_list(get(&value, "lenses"));
  assert_eq!(
    lenses.len(),
    4,
    "v0 uses 4 ankh overlays (base / mechanism / animation / projection)"
  );

  // Invariant 3 — lens ids in canonical order
  let lens_ids: Vec<&str> = lenses.iter().map(|l| as_str(get(l, "id"))).collect();
  assert_eq!(
    lens_ids,
    vec![
      "lens.base",
      "lens.mechanism",
      "lens.animation",
      "lens.projection"
    ],
    "lens ids must match the v0 canonical ordering"
  );

  // Invariant 4 — exactly 6 *computed* turns
  let turns = as_list(get(&value, "computed-turns"));
  assert_eq!(turns.len(), 6, "v0 spec has 6 tesseract turns (0..5)");

  // Invariant 5 — computed turn ids 0..5 in order
  let turn_ids: Vec<i64> = turns.iter().map(|t| as_int(get(t, "turn-id"))).collect();
  assert_eq!(
    turn_ids,
    vec![0, 1, 2, 3, 4, 5],
    "turn-id sequence must be [0,1,2,3,4,5]"
  );

  // Invariant 6 — computed Turn 4 carries structural-binding-conflict Held ref
  let turn4_held = list_of_strings(get(&turns[4], "held-refs"));
  assert!(
    turn4_held.contains(&"held.structural-binding-conflict"),
    "computed Turn 4 (LensCompare) must hold structural-binding-conflict; got {:?}",
    turn4_held
  );

  // Invariant 7 — computed-held.held-kind
  assert_eq!(
    as_str(get_path(&value, &["computed-held", "held-kind"])),
    "structural-binding-conflict",
    "v0 exercises the structural-binding-conflict Held kind only"
  );

  // Invariant 8 — computed Turn 5 repair-refs
  let turn5_repair = list_of_strings(get(&turns[5], "repair-refs"));
  assert!(
    turn5_repair.contains(&"repair.role-binding"),
    "computed Turn 5 must emit repair.role-binding RepairCandidate; got {:?}",
    turn5_repair
  );

  // Invariant 9 — computed-repair.promotion stays `pending`
  assert_eq!(
    as_str(get_path(&value, &["computed-repair", "promotion"])),
    "pending",
    "RepairCandidate must remain `pending` — auto-apply forbidden in v0"
  );

  // Invariant 10 — initial state (declared, not computed; the
  // builders do not yet produce the meta-circular log differential
  // record, so the fixture's static block is the primary source).
  let initial_held = list_of_strings(get_path(
    &value,
    &[
      "meta-circular-log-differential",
      "initial-state",
      "open-held",
    ],
  ));
  assert_eq!(
    initial_held,
    Vec::<&str>::new(),
    "initial state (pre-Turn-0) must have no Held — the differential is what v0 proves"
  );

  // Invariant 11 — after-turn-5 held contains structural-binding-conflict
  let final_held = list_of_strings(get_path(
    &value,
    &[
      "meta-circular-log-differential",
      "after-turn-5",
      "open-held",
    ],
  ));
  assert!(
    final_held.contains(&"held.structural-binding-conflict"),
    "after-turn-5 must carry structural-binding-conflict; got {:?}",
    final_held
  );

  // Invariant 12 — provider calls budget = 0
  assert_eq!(
    as_int(get_path(
      &value,
      &["llm-free", "evaluation-budget", "provider-calls"]
    )),
    0,
    "v0 must declare a zero provider-call budget (LLM-free)"
  );

  // Invariant 13 — network egress budget = 0
  assert_eq!(
    as_int(get_path(
      &value,
      &["llm-free", "evaluation-budget", "network-egress"]
    )),
    0,
    "v0 must declare a zero network-egress budget"
  );

  // Invariant 14 — replay determinism mode (NOT byte-equal)
  assert_eq!(
    as_str(get_path(&value, &["replay", "determinism-mode"])),
    "normalized-structural-equality",
    "v0 replay contract must be normalized-structural-equality, not byte-equal"
  );
}

// ---------------------------------------------------------------
// Test 2 — Invariant 15: replay determinism
//
// Two evaluations of the same fixture with the same evaluator must
// yield identical normalized-structural traces. `Value::to_json()`
// is deterministic because `Value::AttrSet` is backed by a
// `BTreeMap` (lexically sorted keys); the JSON byte-equality across
// two runs IS the normalized structural equality the fixture's
// `replay` block declares.
//
// This is intentionally NOT a byte-equal-on-the-source check (the
// source is a single file, byte-equal trivially); it is a check
// that the *evaluator output* — which is what runtime dynamic logic
// generation cares about — is reproduced.
// ---------------------------------------------------------------

#[test]
fn minimal_tesseract_v0_replay_normalized_structural_equality() {
  let path = fixture_path();
  let v1 = eval_file(&path).expect("first evaluation must succeed");
  let v2 = eval_file(&path).expect("second evaluation must succeed");

  let j1 = v1.to_json();
  let j2 = v2.to_json();

  assert_eq!(
    j1, j2,
    "v0 replay determinism (normalized structural equality) violated;\n\
     first  trace = {}\n\
     second trace = {}",
    j1, j2
  );
}

// ---------------------------------------------------------------
// Test 3 — Step 2.5 invariants 16..19: the fixture's `.px` builders
// derive `computed-turns` / `computed-lens-compare` / `computed-held`
// / `computed-repair` from the lens metadata + builder rules, and
// each must match the corresponding static `expected-*` block
// (`turns` / `lens-compare` / `held` / `repair`) under normalized
// structural equality.
//
// This is what lifts the fixture from "fixture *contains* expected
// log" to "fixture *computes* expected log". It is still NOT a full
// loop-proof — the fixture is a closed `.px` rule set, not a runtime
// rule loaded from owner law — but it is the next grade above
// spec-as-fixture.
//
// `Value::AttrSet` is a `BTreeMap` with sorted keys, so byte equality
// of `to_json()` IS the normalized structural equality contract.
// ---------------------------------------------------------------

#[test]
fn minimal_tesseract_v0_computed_matches_expected_turns() {
  let value = eval_file(&fixture_path()).expect("fixture must evaluate");
  let computed = get(&value, "computed-turns").to_json();
  let expected = get(&value, "turns").to_json();
  assert_eq!(
    computed, expected,
    "computed-turns must match expected (`turns`) under normalized structural equality"
  );
}

#[test]
fn minimal_tesseract_v0_computed_matches_expected_lens_compare() {
  let value = eval_file(&fixture_path()).expect("fixture must evaluate");
  let computed = get(&value, "computed-lens-compare").to_json();
  let expected = get(&value, "lens-compare").to_json();
  assert_eq!(
    computed, expected,
    "computed-lens-compare must match expected (`lens-compare`) under normalized structural equality"
  );
}

#[test]
fn minimal_tesseract_v0_computed_matches_expected_held() {
  let value = eval_file(&fixture_path()).expect("fixture must evaluate");
  let computed = get(&value, "computed-held").to_json();
  let expected = get(&value, "held").to_json();
  assert_eq!(
    computed, expected,
    "computed-held must match expected (`held`) under normalized structural equality"
  );
}

#[test]
fn minimal_tesseract_v0_computed_matches_expected_repair() {
  let value = eval_file(&fixture_path()).expect("fixture must evaluate");
  let computed = get(&value, "computed-repair").to_json();
  let expected = get(&value, "repair").to_json();
  assert_eq!(
    computed, expected,
    "computed-repair must match expected (`repair`) under normalized structural equality"
  );
}

// ---------------------------------------------------------------
// Step 2.6 — Negative / corruption tests.
//
// Each test takes the fixture's computed (or expected) output,
// constructs a corrupted variant in-test, and asserts that the v0
// invariant the harness enforces would *reject* that variant.  No
// new fixture file, no fixture mutation on disk, no panic-catching:
// the corruption is built as a fresh `Value` and the same predicate
// the harness uses against the real trace is asked to return false.
//
// These tests show that the v0 harness is not just describing what
// passes; it is positively rejecting wrong shapes.
// ---------------------------------------------------------------

/// Replace a single key inside a `Value::AttrSet`.  Used to build
/// corrupted variants without mutating the original.
fn attrset_with(v: &Value, key: &str, replacement: Value) -> Value {
  match v {
    Value::AttrSet(m) => {
      let mut next = m.clone();
      std::sync::Arc::make_mut(&mut next).insert(key.to_string(), replacement);
      Value::AttrSet(next)
    }
    other => panic!("attrset_with on non-attrset: {:?}", other),
  }
}

/// Replace one element inside a `Value::List`.
fn list_with_replaced(v: &Value, idx: usize, replacement: Value) -> Value {
  match v {
    Value::List(items) => {
      let mut next = items.clone();
      std::sync::Arc::make_mut(&mut next)[idx] = replacement;
      Value::List(next)
    }
    other => panic!("list_with_replaced on non-list: {:?}", other),
  }
}

/// Drop the first element matching `needle` from a `Value::List` of
/// `Value::String`.
fn list_without_string(v: &Value, needle: &str) -> Value {
  match v {
    Value::List(items) => {
      let next: Vec<Value> = items
        .iter()
        .filter(|item| !matches!(item, Value::String(s) if s == needle))
        .cloned()
        .collect();
      Value::List(std::sync::Arc::new(next))
    }
    other => panic!("list_without_string on non-list: {:?}", other),
  }
}

/// Truncate a `Value::List` to the first `keep` elements.
fn list_take(v: &Value, keep: usize) -> Value {
  match v {
    Value::List(items) => Value::List(std::sync::Arc::new(
      items.iter().take(keep).cloned().collect(),
    )),
    other => panic!("list_take on non-list: {:?}", other),
  }
}

/// Predicate for invariant 6: Turn 4 of the trace carries the
/// `held.structural-binding-conflict` ref.
fn turn4_holds_structural_binding_conflict(turns: &Value) -> bool {
  let list = match turns {
    Value::List(l) => l,
    _ => return false,
  };
  if list.len() < 5 {
    return false;
  }
  list_of_strings(get(&list[4], "held-refs"))
    .iter()
    .any(|s| *s == "held.structural-binding-conflict")
}

/// Predicate for invariant 7: held-kind is `structural-binding-conflict`.
fn held_kind_is_structural_binding_conflict(held: &Value) -> bool {
  match held {
    Value::AttrSet(m) => match m.get("held-kind") {
      Some(Value::String(s)) => s == "structural-binding-conflict",
      _ => false,
    },
    _ => false,
  }
}

/// Predicate for invariant 9: `repair.promotion == "pending"`.
fn repair_promotion_pending(repair: &Value) -> bool {
  match repair {
    Value::AttrSet(m) => match m.get("promotion") {
      Some(Value::String(s)) => s == "pending",
      _ => false,
    },
    _ => false,
  }
}

/// Predicate for invariants 4 + 5: trace has 6 turns with ids 0..5.
fn turns_are_zero_through_five(turns: &Value) -> bool {
  let list = match turns {
    Value::List(l) => l,
    _ => return false,
  };
  if list.len() != 6 {
    return false;
  }
  list
    .iter()
    .enumerate()
    .all(|(i, t)| match get(t, "turn-id") {
      Value::Int(n) => *n == i as i64,
      _ => false,
    })
}

#[test]
fn minimal_tesseract_v0_rejects_corrupted_missing_held_ref() {
  let value = eval_file(&fixture_path()).expect("fixture must evaluate");
  let computed_turns = get(&value, "computed-turns");

  // Sanity: the real computed trace passes the predicate.
  assert!(
    turn4_holds_structural_binding_conflict(computed_turns),
    "real v0 trace should hold structural-binding-conflict on Turn 4"
  );

  // Corruption: drop the held-ref from Turn 4.
  let turn4 = match computed_turns {
    Value::List(items) => &items[4],
    _ => panic!("computed-turns must be a list"),
  };
  let turn4_held_clean = get(turn4, "held-refs");
  let turn4_held_corrupted =
    list_without_string(turn4_held_clean, "held.structural-binding-conflict");
  let turn4_corrupted = attrset_with(turn4, "held-refs", turn4_held_corrupted);
  let turns_corrupted = list_with_replaced(computed_turns, 4, turn4_corrupted);

  // Negative assertion: corrupted trace must NOT pass the invariant.
  assert!(
    !turn4_holds_structural_binding_conflict(&turns_corrupted),
    "v0 harness must reject a Turn-4 trace missing the structural-binding-conflict held ref"
  );

  // It must also stop matching the static `turns` oracle.
  let oracle = get(&value, "turns").to_json();
  assert_ne!(
    turns_corrupted.to_json(),
    oracle,
    "corrupted computed-turns must not equal the expected `turns` oracle"
  );
}

#[test]
fn minimal_tesseract_v0_rejects_wrong_held_kind() {
  let value = eval_file(&fixture_path()).expect("fixture must evaluate");
  let computed_held = get(&value, "computed-held");

  // Sanity: real held passes the predicate.
  assert!(
    held_kind_is_structural_binding_conflict(computed_held),
    "real v0 held must be structural-binding-conflict"
  );

  // Corruption: replace held-kind with a bogus value.
  let corrupted = attrset_with(
    computed_held,
    "held-kind",
    Value::String("constraint-conflict".to_string()),
  );

  // Negative assertion: corrupted held must NOT pass the predicate.
  assert!(
    !held_kind_is_structural_binding_conflict(&corrupted),
    "v0 harness must reject any Held kind other than structural-binding-conflict"
  );

  // It must also stop matching the static `held` oracle.
  let oracle = get(&value, "held").to_json();
  assert_ne!(
    corrupted.to_json(),
    oracle,
    "corrupted computed-held must not equal the expected `held` oracle"
  );
}

#[test]
fn minimal_tesseract_v0_rejects_auto_applied_repair() {
  let value = eval_file(&fixture_path()).expect("fixture must evaluate");
  let computed_repair = get(&value, "computed-repair");

  // Sanity: real repair is `pending` (candidate-only).
  assert!(
    repair_promotion_pending(computed_repair),
    "real v0 repair must be `pending`"
  );

  // Corruption: promote the repair to `accepted` (i.e. auto-apply).
  let corrupted = attrset_with(
    computed_repair,
    "promotion",
    Value::String("accepted".to_string()),
  );

  // Negative assertion: corrupted repair must NOT pass the
  // candidate-only invariant.  v0 forbids auto-apply.
  assert!(
    !repair_promotion_pending(&corrupted),
    "v0 harness must reject an auto-applied repair (promotion=accepted)"
  );

  // It must also stop matching the static `repair` oracle.
  let oracle = get(&value, "repair").to_json();
  assert_ne!(
    corrupted.to_json(),
    oracle,
    "corrupted computed-repair must not equal the expected `repair` oracle"
  );
}

#[test]
fn minimal_tesseract_v0_rejects_three_lens_trace() {
  let value = eval_file(&fixture_path()).expect("fixture must evaluate");
  let computed_turns = get(&value, "computed-turns");

  // Sanity: real trace has the full 6 turns with ids 0..5.
  assert!(
    turns_are_zero_through_five(computed_turns),
    "real v0 trace must have 6 turns 0..5"
  );

  // Corruption A: drop a lens, leaving only 5 turns. v0 demands 6.
  let truncated = list_take(computed_turns, 5);
  assert!(
    !turns_are_zero_through_five(&truncated),
    "v0 harness must reject a trace with fewer than 6 turns"
  );

  // Corruption B: build a 4-turn (3 lens) attach-only trace, no
  // compare/repair.  Still wrong shape; v0 demands the full 6.
  let three_lens_attach = list_take(computed_turns, 3);
  assert!(
    !turns_are_zero_through_five(&three_lens_attach),
    "v0 harness must reject a trace built from only 3 lens attach turns"
  );

  // The lens count itself stays at 4 in the fixture, but the trace
  // length is the relevant invariant: even if someone hand-wrote a
  // shorter trace pretending only 3 lenses were applied, the
  // harness rejects it.
}

// ---------------------------------------------------------------
// Step 3 — Runtime Owner-Law Lift: trajectory differential.
//
// The plan (project-wiki/maps/minimal-tesseract-v0-map.md §"Step 3
// Plan") requires that:
//   - With `v0_owner_law.px` loaded by the `.px` harness, the
//     evaluator produces 6 turns + Held + RepairCandidate.
//   - With `ownerLaw = {}` (owner-law absent), the evaluator
//     produces 0 turns, no Held, no RepairCandidate.
//   - Both runs use the SAME evaluator on the SAME SourceObject;
//     the observable difference is the loop-proof signal Step 3
//     emitted; Step 4 (2026-05-04) wrote the receipt in `done.md`
//     and `project-wiki/showcase.md`.
//
// The Rust harness does NOT apply owner-law itself. Owner-law
// application happens inside `v0_run_with_owner_law.px`. This block
// only evaluates files and reads the resulting `Value`.
// ---------------------------------------------------------------

/// Helper: a `Value::Null` predicate, used for the empty-trajectory
/// surfaces (`computed-held`, `computed-repair`, `computed-lens-compare`).
fn is_null(v: &Value) -> bool {
  matches!(v, Value::Null)
}

#[test]
fn minimal_tesseract_v0_owner_law_loaded_marker_present() {
  let value = eval_file(&fixture_path()).expect("with-owner-law harness must evaluate");
  // Invariant 16: positive marker.
  match get(&value, "owner-law-loaded") {
    Value::Bool(b) => assert!(
      *b,
      "v0_run_with_owner_law.px must report owner-law-loaded=true"
    ),
    other => panic!("owner-law-loaded must be a Bool, got {:?}", other),
  }
  // The source pointer is informational but useful for traceability.
  assert_eq!(
    as_str(get(&value, "owner-law-source")),
    "fixtures/minimal-tesseract-v0/v0_owner_law.px",
    "owner-law-source must point at the fixture-local rules file"
  );
}

#[test]
fn minimal_tesseract_v0_owner_law_absent_produces_empty_trajectory() {
  let value =
    eval_file(&fixture_without_owner_law_path()).expect("without-owner-law harness must evaluate");

  // Invariant 17: negative marker.
  match get(&value, "owner-law-loaded") {
    Value::Bool(b) => assert!(
      !*b,
      "v0_run_without_owner_law.px must report owner-law-loaded=false"
    ),
    other => panic!("owner-law-loaded must be a Bool, got {:?}", other),
  }

  // Invariant 18: no turns.
  let turns = as_list(get(&value, "computed-turns"));
  assert!(
    turns.is_empty(),
    "owner-law absent must yield zero computed-turns; got {} turns",
    turns.len()
  );

  // Invariant 19: no Held.
  assert!(
    is_null(get(&value, "computed-held")),
    "owner-law absent must yield computed-held=null; got {:?}",
    get(&value, "computed-held")
  );

  // Invariant 20: no RepairCandidate.
  assert!(
    is_null(get(&value, "computed-repair")),
    "owner-law absent must yield computed-repair=null; got {:?}",
    get(&value, "computed-repair")
  );

  // Invariant 21: no LensCompare result.
  assert!(
    is_null(get(&value, "computed-lens-compare")),
    "owner-law absent must yield computed-lens-compare=null; got {:?}",
    get(&value, "computed-lens-compare")
  );

  // The oracles in `v0_inputs.px` (turns / lens-compare / held /
  // repair) must STILL be present — they are the answer the
  // owner-law would produce *if* loaded. Their presence is what
  // keeps Step 3 from collapsing into a tautology.
  assert_eq!(
    as_list(get(&value, "turns")).len(),
    6,
    "expected `turns` oracle must remain in v0_inputs.px even when owner-law is absent"
  );
  assert_eq!(
    as_str(get_path(&value, &["held", "held-kind"])),
    "structural-binding-conflict",
    "expected `held` oracle must remain present"
  );
}

#[test]
fn minimal_tesseract_v0_owner_law_file_exposes_full_rule_set() {
  // Step 3.1 + Step 5 — invariants 23..28 + 30: the owner-law
  // file evaluates to an attrset that exposes ALL seven rule
  // functions. Step 3 (turn builders only) closed three;
  // Step 3.1 (record-body builders for LensCompare / Held /
  // Repair) closed three more; Step 5 (meta-circular log writer)
  // added the seventh. Without any of them, the corresponding
  // record body would still be inline in the runner and the
  // lift would be partial.
  let value = eval_file(&owner_law_file_path()).expect("owner-law file must evaluate");
  let attrs = as_attrs(&value);

  // The seven rule names the v0 owner-law surface contracts to
  // expose. The first six landed in Step 3 / Step 3.1 (turn
  // builders + record-body builders). The seventh
  // (`buildMetaCircularLogDifferential`) lifted in Step 5 — so
  // the meta-circular log differential is also derived from
  // owner-law, not inline in the runner. If any rule is missing,
  // the runner would fall back to hand-written attrsets, which
  // is exactly the gap each Step closes.
  let required_rules = [
    "buildAttachTurn",
    "buildCompareTurn",
    "buildRepairTurn",
    "buildLensCompareResult",
    "buildHeldEntry",
    "buildRepairCandidate",
    "buildMetaCircularLogDifferential",
  ];

  for rule in required_rules {
    let entry = attrs.get(rule).unwrap_or_else(|| {
      panic!(
        "v0_owner_law.px must expose `{}`; available keys: {:?}",
        rule,
        attrs.keys().collect::<Vec<_>>()
      )
    });
    // Each rule must be callable — `Value::Lambda` from a `.px`
    // function definition. (A `BuiltinPartial` would be a bug
    // because owner-law rules are user-defined `.px` lambdas, not
    // partially-applied builtins.)
    assert!(
      matches!(entry, Value::Lambda { .. }),
      "v0_owner_law.px:`{}` must be a Lambda, got {:?}",
      rule,
      entry
    );
  }

  // The file must contain ONLY rule functions — no oracle, no
  // SourceObject, no expected-* leak. If the owner-law file
  // grew an oracle, the Step-3 plan's "preserve the oracle in
  // v0_inputs.px" guard would be violated.
  let allowed: std::collections::BTreeSet<&str> = required_rules.iter().copied().collect();
  for key in attrs.keys() {
    assert!(
      allowed.contains(key.as_str()),
      "v0_owner_law.px must NOT carry non-rule keys (oracle / source-object / lenses / etc.); found unexpected key `{}`",
      key
    );
  }
}

#[test]
fn minimal_tesseract_v0_record_bodies_come_from_owner_law() {
  // Step 3.1 — invariant 29: `computed-lens-compare`,
  // `computed-held`, and `computed-repair` MUST equal the
  // owner-law function results (`owner-law-result.*`). This is
  // the test that closes the original Step-3 gap: if the runner
  // hardcoded the bodies (the Step-3 state before this 3.1 fix),
  // it could produce values matching the oracle while NOT
  // actually calling the owner-law rules. After 3.1, the runner
  // exposes both surfaces and they must be byte-equal under
  // normalized structural equality.
  let value = eval_file(&fixture_path()).expect("with-owner-law harness must evaluate");

  // Marker: the runner reports record bodies are extracted.
  match get(&value, "owner-law-record-bodies-extracted") {
    Value::Bool(b) => assert!(
      *b,
      "v0_run_with_owner_law.px must report owner-law-record-bodies-extracted=true after Step 3.1"
    ),
    other => panic!(
      "owner-law-record-bodies-extracted must be a Bool, got {:?}",
      other
    ),
  }

  let owner_law_result = get(&value, "owner-law-result");

  // computed-lens-compare equals owner-law-result.lens-compare.
  let computed_lc = get(&value, "computed-lens-compare").to_json();
  let direct_lc = get(owner_law_result, "lens-compare").to_json();
  assert_eq!(
    computed_lc, direct_lc,
    "computed-lens-compare must equal owner-law-result.lens-compare — \
     i.e. the runner must NOT hardcode the LensCompare body"
  );

  // computed-held equals owner-law-result.held.
  let computed_held = get(&value, "computed-held").to_json();
  let direct_held = get(owner_law_result, "held").to_json();
  assert_eq!(
    computed_held, direct_held,
    "computed-held must equal owner-law-result.held — \
     i.e. the runner must NOT hardcode the Held body"
  );

  // computed-repair equals owner-law-result.repair.
  let computed_repair = get(&value, "computed-repair").to_json();
  let direct_repair = get(owner_law_result, "repair").to_json();
  assert_eq!(
    computed_repair, direct_repair,
    "computed-repair must equal owner-law-result.repair — \
     i.e. the runner must NOT hardcode the Repair body"
  );
}

#[test]
fn minimal_tesseract_v0_trajectory_differential_with_vs_without_owner_law() {
  // Invariant 22: same evaluator, same SourceObject, observably
  // different trace. The differential is asserted on the
  // `computed-*` surfaces (which is where owner-law application
  // shows up); we DON'T compare the whole envelope, because the
  // shared inputs would dominate the diff and obscure the signal.

  let with_value = eval_file(&fixture_path()).expect("with-owner-law harness must evaluate");
  let without_value =
    eval_file(&fixture_without_owner_law_path()).expect("without-owner-law harness must evaluate");

  // Sanity: both harnesses share the same SourceObject.
  assert_eq!(
    as_str(get_path(&with_value, &["source-object", "id"])),
    as_str(get_path(&without_value, &["source-object", "id"])),
    "trajectory differential must hold the SourceObject constant"
  );

  // computed-turns differs: 6 vs 0.
  let with_turns = get(&with_value, "computed-turns").to_json();
  let without_turns = get(&without_value, "computed-turns").to_json();
  assert_ne!(
    with_turns, without_turns,
    "computed-turns must differ between owner-law loaded and owner-law absent"
  );

  // computed-held differs: AttrSet vs Null.
  let with_held = get(&with_value, "computed-held").to_json();
  let without_held = get(&without_value, "computed-held").to_json();
  assert_ne!(
    with_held, without_held,
    "computed-held must differ between owner-law loaded and owner-law absent"
  );

  // computed-repair differs: AttrSet vs Null.
  let with_repair = get(&with_value, "computed-repair").to_json();
  let without_repair = get(&without_value, "computed-repair").to_json();
  assert_ne!(
    with_repair, without_repair,
    "computed-repair must differ between owner-law loaded and owner-law absent"
  );

  // computed-lens-compare differs: AttrSet vs Null.
  let with_lc = get(&with_value, "computed-lens-compare").to_json();
  let without_lc = get(&without_value, "computed-lens-compare").to_json();
  assert_ne!(
    with_lc, without_lc,
    "computed-lens-compare must differ between owner-law loaded and owner-law absent"
  );
}

// ---------------------------------------------------------------
// Step 5 — meta-circular log writer extracted into owner-law.
//
// Step 3 / 3.1 lifted turn / LensCompare / Held / Repair record
// bodies into owner-law. The `meta-circular-log-differential`
// block, however, was still a static oracle in `v0_inputs.px` —
// the runtime had nothing computing it. Step 5 adds a 7th rule
// (`buildMetaCircularLogDifferential`) and wires
// `computed-meta-circular-log-differential` through it; the
// without-trajectory exposes a no-change record (after-turn-5 ==
// initial-state). The tests below close the same gap pattern
// the previous Steps used: file-exposes test (already updated to
// require the 7th rule), oracle equivalence, byte-equality with
// the owner-law-result.* surface, no-change negative half, plus
// three corruption rejections.
// ---------------------------------------------------------------

#[test]
fn minimal_tesseract_v0_meta_circular_log_from_owner_law() {
  // Invariant 31: with-trajectory `computed-meta-circular-log-
  // differential` equals the static oracle in `v0_inputs.px`
  // AND equals `owner-law-result.meta-circular-log-differential`.
  // The double check stops a future runner from quietly inlining
  // the log differential while still happening to match the
  // oracle.
  let value = eval_file(&fixture_path()).expect("with-owner-law harness must evaluate");

  let computed = get(&value, "computed-meta-circular-log-differential").to_json();
  let oracle = get(&value, "meta-circular-log-differential").to_json();
  assert_eq!(
    computed, oracle,
    "computed-meta-circular-log-differential must equal the expected oracle \
     (`meta-circular-log-differential`) under normalized structural equality"
  );

  let owner_law_result = get(&value, "owner-law-result");
  let direct = get(owner_law_result, "meta-circular-log-differential").to_json();
  assert_eq!(
    computed, direct,
    "computed-meta-circular-log-differential must equal owner-law-result.\
     meta-circular-log-differential — i.e. the runner must NOT hardcode \
     the log differential body"
  );
}

#[test]
fn minimal_tesseract_v0_owner_law_absent_log_differential_is_no_change() {
  // Invariant 32: without-trajectory the log differential records
  // a no-change state — `after-turn-5` must equal `initial-state`
  // because no turns occurred. This is what makes the
  // owner-law-absent trace observably *describable* by the
  // runtime (not just empty); the runtime correctly says
  // "nothing happened".
  let value =
    eval_file(&fixture_without_owner_law_path()).expect("without-owner-law harness must evaluate");

  let initial = get_path(
    &value,
    &["computed-meta-circular-log-differential", "initial-state"],
  )
  .to_json();
  let after = get_path(
    &value,
    &["computed-meta-circular-log-differential", "after-turn-5"],
  )
  .to_json();
  assert_eq!(
    initial, after,
    "without-owner-law: after-turn-5 must equal initial-state (no turns occurred);\n\
     initial = {}\n\
     after   = {}",
    initial, after
  );

  // And the no-owner-law side's after-turn-5 must NOT match the
  // oracle's after-turn-5 (which is the with-owner-law result).
  let oracle_after =
    get_path(&value, &["meta-circular-log-differential", "after-turn-5"]).to_json();
  assert_ne!(
    after, oracle_after,
    "without-owner-law: after-turn-5 must differ from the oracle's after-turn-5"
  );
}

#[test]
fn minimal_tesseract_v0_log_differential_rejects_missing_active_lens() {
  // Invariant 33: dropping `lens.projection` from the
  // computed-meta-circular-log-differential.after-turn-5.active-
  // lenses list breaks the oracle byte-equality. This catches a
  // future regression where the trajectory is silently truncated
  // (e.g. a buildAttachTurn rule that stops emitting the 4th
  // lens) but the writer still claims a clean differential.
  let value = eval_file(&fixture_path()).expect("fixture must evaluate");
  let log = get(&value, "computed-meta-circular-log-differential");
  let after = get(log, "after-turn-5");

  // Sanity: the real after has lens.projection.
  let real_active = list_of_strings(get(after, "active-lenses"));
  assert!(
    real_active.contains(&"lens.projection"),
    "real after-turn-5.active-lenses must contain lens.projection; got {:?}",
    real_active
  );

  // Corruption: drop lens.projection.
  let active_corrupted = list_without_string(get(after, "active-lenses"), "lens.projection");
  let after_corrupted = attrset_with(after, "active-lenses", active_corrupted);
  let log_corrupted = attrset_with(log, "after-turn-5", after_corrupted);

  // Oracle byte-equality must fail.
  let oracle = get(&value, "meta-circular-log-differential").to_json();
  assert_ne!(
    log_corrupted.to_json(),
    oracle,
    "corrupted after-turn-5 (missing lens.projection) must not equal the oracle"
  );
}

#[test]
fn minimal_tesseract_v0_log_differential_rejects_missing_held() {
  // Invariant 34: dropping `held.structural-binding-conflict`
  // from after-turn-5.open-held breaks oracle byte-equality.
  // Catches a regression where the writer claims a clean
  // trajectory while the Held was actually still open.
  let value = eval_file(&fixture_path()).expect("fixture must evaluate");
  let log = get(&value, "computed-meta-circular-log-differential");
  let after = get(log, "after-turn-5");

  let real_held = list_of_strings(get(after, "open-held"));
  assert!(
    real_held.contains(&"held.structural-binding-conflict"),
    "real after-turn-5.open-held must contain held.structural-binding-conflict; got {:?}",
    real_held
  );

  let held_corrupted =
    list_without_string(get(after, "open-held"), "held.structural-binding-conflict");
  let after_corrupted = attrset_with(after, "open-held", held_corrupted);
  let log_corrupted = attrset_with(log, "after-turn-5", after_corrupted);

  let oracle = get(&value, "meta-circular-log-differential").to_json();
  assert_ne!(
    log_corrupted.to_json(),
    oracle,
    "corrupted after-turn-5 (missing structural-binding-conflict) must not equal the oracle"
  );
}

#[test]
fn minimal_tesseract_v0_log_differential_rejects_missing_repair_need() {
  // Invariant 35: dropping `need.repair-promotion-decision` from
  // after-turn-5.open-needs breaks oracle byte-equality. Catches
  // a regression where Turn 5 (repair) silently stopped emitting
  // its Need.
  let value = eval_file(&fixture_path()).expect("fixture must evaluate");
  let log = get(&value, "computed-meta-circular-log-differential");
  let after = get(log, "after-turn-5");

  let real_needs = list_of_strings(get(after, "open-needs"));
  assert!(
    real_needs.contains(&"need.repair-promotion-decision"),
    "real after-turn-5.open-needs must contain need.repair-promotion-decision; got {:?}",
    real_needs
  );

  let needs_corrupted =
    list_without_string(get(after, "open-needs"), "need.repair-promotion-decision");
  let after_corrupted = attrset_with(after, "open-needs", needs_corrupted);
  let log_corrupted = attrset_with(log, "after-turn-5", after_corrupted);

  let oracle = get(&value, "meta-circular-log-differential").to_json();
  assert_ne!(
    log_corrupted.to_json(),
    oracle,
    "corrupted after-turn-5 (missing repair-promotion-decision) must not equal the oracle"
  );
}
