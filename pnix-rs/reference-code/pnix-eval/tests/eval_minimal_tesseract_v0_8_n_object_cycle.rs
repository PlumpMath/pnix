//! v0.8 N-object cycle (3-object cycle proof). Scales the
//! visited-set chain walker from N=2 (v0.6) to N=3 to prove
//! generality. Same walker code as v0.6 — only the inputs
//! differ (3 objects, 3 cycle edges with kinds [depends-on-
//! frame, mounted-on, depends-on-frame]). v0.8 derives from
//! v0.6 (cycle-aware), NOT v0.7 (promotion+cycle); promotion
//! machinery is intentionally NOT present.
//!
//! Tests 169 (walker termination at N=3), 170/171/172 (exact
//! 3-step chain per object), 174 (3-entry cycle-Held overlay),
//! and 175 (exact cycle-path triple+closure) are load-
//! bearing.
//!
//! Truth owner:        project-wiki/maps/minimal-tesseract-v0-map.md
//!                     §"v0.8 design decision — N-object cycle
//!                       (3-object cycle proof; visited-set walker
//!                       generality; no promotion)"
//! Active scope:       project-wiki/maps/active-domain-constitution.md Art. 8
//!
//! What this test asserts (16 invariants, indices continued
//! from the v0.7 test's 167; invariants 182..183 are v0.8.1
//! micro-patch — proof hygiene: real byte-equality v0.2
//! surface check + explicit owner-law presence gating):
//! 168. world-set has 3 SourceObjects [arm_link_1,
//!      arm_link_2, arm_link_3]; relations form a 3-edge
//!      directed cycle with kinds [depends-on-frame,
//!      mounted-on, depends-on-frame] (declared order);
//!      v0-8 / cycle-aware / chain-aware markers true.
//! 169. **load-bearing — walker terminates on N=3 cycle**:
//!      `transitive-chain-per-object[obj]` has length
//!      EXACTLY 3 for every obj in world-set.
//! 170. **load-bearing — arm_link_1 chain exact 3-step
//!      sequence**: step 0 = { arm_link_2,
//!      depends-on-frame, cd=false, has-held=true };
//!      step 1 = { arm_link_3, mounted-on, cd=false,
//!      has-held=false }; step 2 = { arm_link_1,
//!      depends-on-frame, cd=true, clt=arm_link_1,
//!      has-held=true }.
//! 171. **load-bearing — arm_link_2 chain exact 3-step
//!      sequence**: step 0 = { arm_link_3, mounted-on,
//!      cd=false, has-held=false }; step 1 = { arm_link_1,
//!      depends-on-frame, cd=false, has-held=true };
//!      step 2 = { arm_link_2, depends-on-frame, cd=true,
//!      clt=arm_link_2, has-held=true }.
//! 172. **load-bearing — arm_link_3 chain exact 3-step
//!      sequence**: step 0 = { arm_link_1,
//!      depends-on-frame, cd=false, has-held=true };
//!      step 1 = { arm_link_2, depends-on-frame, cd=false,
//!      has-held=true }; step 2 = { arm_link_3,
//!      mounted-on, cd=true, clt=arm_link_3,
//!      has-held=false }.
//! 173. Chain step shape is exactly { object-id;
//!      relation-kind; has-held; held-instance-ref;
//!      cycle-detected; cycle-loop-target; } — six fields
//!      (v0.6 shape; no upstream-promoted because v0.8 is
//!      post-v0.6 not post-v0.7).
//! 174. **load-bearing — cycle-Held overlay has 3
//!      entries**: arm_link_1, arm_link_2, AND arm_link_3
//!      each carry held-kind = "dependency-cycle",
//!      promoted = false.
//! 175. **load-bearing — exact cycle-path triple+closure**:
//!      arm_link_1 = ["arm_link_1", "arm_link_2",
//!      "arm_link_3", "arm_link_1"]; arm_link_2 =
//!      ["arm_link_2", "arm_link_3", "arm_link_1",
//!      "arm_link_2"]; arm_link_3 = ["arm_link_3",
//!      "arm_link_1", "arm_link_2", "arm_link_3"].
//! 176. v0.2 trace surfaces still present (computed-turns
//!      6 turns; per-object Held / Repair maps still 2
//!      entries; arm_link_3 still has no Held in v0.2
//!      trace).
//! 177. without-owner-law: chain still produces
//!      cycle-detected = true at every chain's closure;
//!      every step has-held=false and held-instance-ref=null.
//! 178. without-owner-law: computed-cycle-helds-per-object
//!      empty.
//! 179. relation-kind plurality preserved at N=3:
//!      mounted-on appears at chain steps where the
//!      producing edge was arm_link_2 → arm_link_3
//!      (mounted-on); depends-on-frame appears at the
//!      other two edges and surfaces on multiple chain
//!      steps without collapsing to a per-cycle constant.
//! 180. promotion machinery is NOT present in v0.8: Need
//!      has no `status`, repair-effect has no `applied-by-
//!      repair-ids`, chain step has no `upstream-promoted`.
//! 181. v0_owner_law.px STILL exposes exactly the same 7
//!      Lambda rules.
//! 182. **v0.8.1 micro-patch — real byte-equality v0.2
//!      surfaces**: evaluate v0_2_run_set_aware.px directly
//!      and confirm
//!      `eval(v0_8_run_cycle_aware).computed-turns ==
//!      eval(v0_2_run_set_aware).computed-turns`,
//!      `eval(...).computed-held-per-object ==
//!      eval(v0_2_run_set_aware).computed-held-per-object`,
//!      and `eval(...).computed-repair-per-object ==
//!      eval(v0_2_run_set_aware).computed-repair-per-object`
//!      via `to_json()`. Strengthens invariant 176 from
//!      shape-only ("still 6 turns, still 2 entries") to
//!      byte-for-byte identity with the unmodified v0.2
//!      trace. Closes the v0.8 documentation claim that
//!      "v0.2 trace stays byte-for-byte unchanged".
//! 183. **v0.8.1 micro-patch — owner-law presence gates
//!      cycle-Held overlay**: positive runner has
//!      `owner-law-loaded = true` AND
//!      `computed-cycle-helds-per-object` has 3 entries;
//!      without-owner-law runner has
//!      `owner-law-loaded = false` AND
//!      `computed-cycle-helds-per-object` is empty.
//!      Encodes the v0.6/v0.8 design invariant "cycle
//!      structure is purely relational; cycle-Held
//!      materialization is owner-law-gated" directly as a
//!      test rather than implicitly via fixture choice.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------
// Helpers — duplicated from prior tests.
// ---------------------------------------------------------------

fn fixture_root() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../../fixtures/minimal-tesseract-v0")
}

fn fixture_path() -> PathBuf {
  fixture_root().join("v0_8_run_cycle_aware.px")
}

fn fixture_without_owner_law_path() -> PathBuf {
  fixture_root().join("v0_8_run_without_owner_law.px")
}

fn v0_2_set_aware_path() -> PathBuf {
  fixture_root().join("v0_2_run_set_aware.px")
}

fn owner_law_file_path() -> PathBuf {
  fixture_root().join("v0_owner_law.px")
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

fn get_path<'a>(root: &'a Value, path: &[&str]) -> &'a Value {
  let mut cur = root;
  for key in path {
    cur = get(cur, key);
  }
  cur
}

fn is_null(v: &Value) -> bool {
  matches!(v, Value::Null)
}

/// Helper: assert a chain step matches an expected tuple
/// (object-id, relation-kind, cycle-detected, cycle-loop-target,
/// has-held).
fn assert_step(
  step: &Value,
  expected_oid: &str,
  expected_kind: &str,
  expected_cycle: bool,
  expected_loop_target: Option<&str>,
  expected_has_held: bool,
  context: &str,
) {
  assert_eq!(
    as_str(get(step, "object-id")),
    expected_oid,
    "[{}] object-id mismatch",
    context
  );
  assert_eq!(
    as_str(get(step, "relation-kind")),
    expected_kind,
    "[{}] relation-kind mismatch",
    context
  );
  assert_eq!(
    as_bool(get(step, "cycle-detected")),
    expected_cycle,
    "[{}] cycle-detected mismatch",
    context
  );
  match expected_loop_target {
    Some(target) => assert_eq!(
      as_str(get(step, "cycle-loop-target")),
      target,
      "[{}] cycle-loop-target mismatch",
      context
    ),
    None => assert!(
      is_null(get(step, "cycle-loop-target")),
      "[{}] cycle-loop-target must be null",
      context
    ),
  }
  assert_eq!(
    as_bool(get(step, "has-held")),
    expected_has_held,
    "[{}] has-held mismatch",
    context
  );
}

// ---------------------------------------------------------------
// Tests.
// ---------------------------------------------------------------

#[test]
fn v0_8_world_set_three_object_cycle_with_two_kinds() {
  // Invariant 168.
  let value = eval_file(&fixture_path()).expect("v0.8 cycle-aware harness must evaluate");

  let world_set = as_list(get(&value, "world-set"));
  assert_eq!(world_set.len(), 3);
  let ids: Vec<&str> = world_set.iter().map(|o| as_str(get(o, "id"))).collect();
  assert_eq!(ids, vec!["arm_link_1", "arm_link_2", "arm_link_3"]);

  let relations = as_list(get(&value, "relations"));
  assert_eq!(relations.len(), 3);
  let kinds: Vec<&str> = relations.iter().map(|r| as_str(get(r, "kind"))).collect();
  assert_eq!(
    kinds,
    vec!["depends-on-frame", "mounted-on", "depends-on-frame"]
  );

  // Cycle structure: 1→2, 2→3, 3→1.
  let froms: Vec<&str> = relations.iter().map(|r| as_str(get(r, "from"))).collect();
  let tos: Vec<&str> = relations.iter().map(|r| as_str(get(r, "to"))).collect();
  assert_eq!(froms, vec!["arm_link_1", "arm_link_2", "arm_link_3"]);
  assert_eq!(tos, vec!["arm_link_2", "arm_link_3", "arm_link_1"]);

  assert!(as_bool(get(&value, "v0-8")));
  assert!(as_bool(get(&value, "cycle-aware")));
  assert!(as_bool(get(&value, "chain-aware")));
}

#[test]
fn v0_8_walker_terminates_on_3_cycle_every_chain_length_three() {
  // Invariant 169 — load-bearing.
  let value = eval_file(&fixture_path()).unwrap();
  let chain_map = as_attrs(get(&value, "transitive-chain-per-object"));
  assert_eq!(chain_map.len(), 3);
  for (object_id, chain_value) in chain_map {
    let chain = as_list(chain_value);
    assert_eq!(
      chain.len(),
      3,
      "v0.8 walker must terminate at length EXACTLY 3 for {}; got {}",
      object_id,
      chain.len()
    );
  }
}

#[test]
fn v0_8_arm_link_1_chain_exact_three_step_sequence() {
  // Invariant 170 — load-bearing.
  let value = eval_file(&fixture_path()).unwrap();
  let chain = as_list(get_path(
    &value,
    &["transitive-chain-per-object", "arm_link_1"],
  ));
  assert_eq!(chain.len(), 3);
  assert_step(
    &chain[0],
    "arm_link_2",
    "depends-on-frame",
    false,
    None,
    true,
    "arm_link_1.step0",
  );
  assert_step(
    &chain[1],
    "arm_link_3",
    "mounted-on",
    false,
    None,
    false,
    "arm_link_1.step1",
  );
  assert_step(
    &chain[2],
    "arm_link_1",
    "depends-on-frame",
    true,
    Some("arm_link_1"),
    true,
    "arm_link_1.step2",
  );
}

#[test]
fn v0_8_arm_link_2_chain_exact_three_step_sequence() {
  // Invariant 171 — load-bearing.
  let value = eval_file(&fixture_path()).unwrap();
  let chain = as_list(get_path(
    &value,
    &["transitive-chain-per-object", "arm_link_2"],
  ));
  assert_eq!(chain.len(), 3);
  assert_step(
    &chain[0],
    "arm_link_3",
    "mounted-on",
    false,
    None,
    false,
    "arm_link_2.step0",
  );
  assert_step(
    &chain[1],
    "arm_link_1",
    "depends-on-frame",
    false,
    None,
    true,
    "arm_link_2.step1",
  );
  assert_step(
    &chain[2],
    "arm_link_2",
    "depends-on-frame",
    true,
    Some("arm_link_2"),
    true,
    "arm_link_2.step2",
  );
}

#[test]
fn v0_8_arm_link_3_chain_exact_three_step_sequence() {
  // Invariant 172 — load-bearing.
  let value = eval_file(&fixture_path()).unwrap();
  let chain = as_list(get_path(
    &value,
    &["transitive-chain-per-object", "arm_link_3"],
  ));
  assert_eq!(chain.len(), 3);
  assert_step(
    &chain[0],
    "arm_link_1",
    "depends-on-frame",
    false,
    None,
    true,
    "arm_link_3.step0",
  );
  assert_step(
    &chain[1],
    "arm_link_2",
    "depends-on-frame",
    false,
    None,
    true,
    "arm_link_3.step1",
  );
  assert_step(
    &chain[2],
    "arm_link_3",
    "mounted-on",
    true,
    Some("arm_link_3"),
    false,
    "arm_link_3.step2",
  );
}

#[test]
fn v0_8_chain_step_shape_is_exactly_six_fields() {
  // Invariant 173.
  let value = eval_file(&fixture_path()).unwrap();
  let expected: BTreeSet<&str> = [
    "object-id",
    "relation-kind",
    "has-held",
    "held-instance-ref",
    "cycle-detected",
    "cycle-loop-target",
  ]
  .iter()
  .copied()
  .collect();
  let chain_map = as_attrs(get(&value, "transitive-chain-per-object"));
  for (object_id, chain_value) in chain_map {
    for step in as_list(chain_value) {
      let actual: BTreeSet<&str> = as_attrs(step).keys().map(|k| k.as_str()).collect();
      assert_eq!(
        actual, expected,
        "chain step shape mismatch for {}: got {:?}",
        object_id, actual
      );
    }
  }
}

#[test]
fn v0_8_cycle_helds_overlay_has_three_entries() {
  // Invariant 174 — load-bearing.
  let value = eval_file(&fixture_path()).unwrap();
  let cycle_helds = as_attrs(get(&value, "computed-cycle-helds-per-object"));
  assert_eq!(
    cycle_helds.len(),
    3,
    "3-cycle yields 3 cycle-Held entries (one per cycle participant); got {}",
    cycle_helds.len()
  );
  for object_id in ["arm_link_1", "arm_link_2", "arm_link_3"] {
    let entry = cycle_helds.get(object_id).unwrap_or_else(|| {
      panic!(
        "computed-cycle-helds-per-object must contain `{}`",
        object_id
      )
    });
    assert_eq!(as_str(get(entry, "held-kind")), "dependency-cycle");
    assert!(!as_bool(get(entry, "promoted")));
    assert_eq!(as_str(get(entry, "applies-at")), object_id);
    assert_eq!(as_str(get(entry, "cycle-loop-target")), object_id);
  }
}

#[test]
fn v0_8_cycle_helds_exact_path_triple_plus_closure() {
  // Invariant 175 — load-bearing.
  let value = eval_file(&fixture_path()).unwrap();

  let entry_1 = get_path(&value, &["computed-cycle-helds-per-object", "arm_link_1"]);
  let path_1: Vec<&str> = as_list(get(entry_1, "cycle-path"))
    .iter()
    .map(|p| as_str(p))
    .collect();
  assert_eq!(
    path_1,
    vec!["arm_link_1", "arm_link_2", "arm_link_3", "arm_link_1"],
    "arm_link_1 cycle-path must be the 3-cycle starting and closing at arm_link_1"
  );

  let entry_2 = get_path(&value, &["computed-cycle-helds-per-object", "arm_link_2"]);
  let path_2: Vec<&str> = as_list(get(entry_2, "cycle-path"))
    .iter()
    .map(|p| as_str(p))
    .collect();
  assert_eq!(
    path_2,
    vec!["arm_link_2", "arm_link_3", "arm_link_1", "arm_link_2"]
  );

  let entry_3 = get_path(&value, &["computed-cycle-helds-per-object", "arm_link_3"]);
  let path_3: Vec<&str> = as_list(get(entry_3, "cycle-path"))
    .iter()
    .map(|p| as_str(p))
    .collect();
  assert_eq!(
    path_3,
    vec!["arm_link_3", "arm_link_1", "arm_link_2", "arm_link_3"]
  );
}

#[test]
fn v0_8_v0_2_trace_unchanged_under_cycle_overlay() {
  // Invariant 176.
  let value = eval_file(&fixture_path()).unwrap();

  let turns = as_list(get(&value, "computed-turns"));
  assert_eq!(turns.len(), 6);
  let turn_ids: Vec<i64> = turns.iter().map(|t| as_int(get(t, "turn-id"))).collect();
  assert_eq!(turn_ids, vec![0, 1, 2, 3, 4, 5]);

  let held_map = as_attrs(get(&value, "computed-held-per-object"));
  assert_eq!(
    held_map.len(),
    2,
    "v0.2 Held map must STILL have exactly 2 entries; arm_link_3 has no Held in v0.2 trace even though it now participates in v0.8's cycle"
  );
  assert!(held_map.contains_key("arm_link_1"));
  assert!(held_map.contains_key("arm_link_2"));
  assert!(!held_map.contains_key("arm_link_3"));

  let repair_map = as_attrs(get(&value, "computed-repair-per-object"));
  assert_eq!(repair_map.len(), 2);
}

#[test]
fn v0_8_without_owner_law_cycle_visible_for_all_three_objects() {
  // Invariant 177.
  let value = eval_file(&fixture_without_owner_law_path()).unwrap();
  assert!(!as_bool(get(&value, "owner-law-loaded")));

  for start_id in ["arm_link_1", "arm_link_2", "arm_link_3"] {
    let chain = as_list(get_path(&value, &["transitive-chain-per-object", start_id]));
    assert_eq!(
      chain.len(),
      3,
      "without owner-law: {}'s chain must have length 3 (cycle structure preserved)",
      start_id
    );
    let closure = &chain[2];
    assert!(
      as_bool(get(closure, "cycle-detected")),
      "without owner-law: {}'s closure step must be cycle-detected",
      start_id
    );
    assert_eq!(
      as_str(get(closure, "cycle-loop-target")),
      start_id,
      "without owner-law: {}'s cycle closure must point back at {}",
      start_id,
      start_id
    );
    for step in chain {
      assert!(!as_bool(get(step, "has-held")));
      assert!(is_null(get(step, "held-instance-ref")));
    }
  }
}

#[test]
fn v0_8_without_owner_law_cycle_helds_overlay_empty() {
  // Invariant 178.
  let value = eval_file(&fixture_without_owner_law_path()).unwrap();
  let cycle_helds = as_attrs(get(&value, "computed-cycle-helds-per-object"));
  assert!(
    cycle_helds.is_empty(),
    "without owner-law: cycle-Held overlay empty; got keys {:?}",
    cycle_helds.keys().collect::<Vec<_>>()
  );
}

#[test]
fn v0_8_relation_kind_plurality_preserved_across_n_3() {
  // Invariant 179.
  let value = eval_file(&fixture_path()).unwrap();

  // Collect all chain step relation-kinds across all chains.
  let chain_map = as_attrs(get(&value, "transitive-chain-per-object"));
  let mut all_kinds: Vec<&str> = Vec::new();
  for (_obj, chain_value) in chain_map {
    for step in as_list(chain_value) {
      all_kinds.push(as_str(get(step, "relation-kind")));
    }
  }
  let kind_set: BTreeSet<&str> = all_kinds.iter().copied().collect();
  // Both kinds must appear somewhere in the chain steps.
  assert!(
    kind_set.contains("depends-on-frame"),
    "depends-on-frame must appear on at least one chain step"
  );
  assert!(
    kind_set.contains("mounted-on"),
    "mounted-on must appear on at least one chain step"
  );
  // No third kind drift.
  assert_eq!(
    kind_set,
    ["depends-on-frame", "mounted-on"]
      .iter()
      .copied()
      .collect::<BTreeSet<&str>>(),
    "no third relation kind allowed in v0.8"
  );

  // Per-edge-occurrence count: depends-on-frame appears on 2
  // edges (1→2 and 3→1) and mounted-on on 1 edge (2→3).
  // Across 3 chains × 3 steps = 9 step instances total. Each
  // edge is traversed N=3 times across all chains (each chain
  // visits all 3 edges once). So depends-on-frame should
  // appear 2*3 = 6 times and mounted-on 1*3 = 3 times.
  let dof_count = all_kinds
    .iter()
    .filter(|k| **k == "depends-on-frame")
    .count();
  let mo_count = all_kinds.iter().filter(|k| **k == "mounted-on").count();
  assert_eq!(
    dof_count, 6,
    "depends-on-frame edges traversed 6 times across all chains"
  );
  assert_eq!(
    mo_count, 3,
    "mounted-on edges traversed 3 times across all chains"
  );
}

#[test]
fn v0_8_no_promotion_machinery_in_need_repair_or_chain() {
  // Invariant 180.
  let value = eval_file(&fixture_path()).unwrap();

  // Need does NOT carry `status`.
  let needs_map = as_attrs(get(&value, "computed-cross-object-needs-per-object"));
  for (_obj, needs_value) in needs_map {
    for need in as_list(needs_value) {
      let attrs = as_attrs(need);
      assert!(
        !attrs.contains_key("status"),
        "v0.8 Need must NOT carry `status` (v0.5+ territory)"
      );
    }
  }

  // Repair-effect does NOT carry `applied-by-repair-ids`.
  let repair_effect_map = as_attrs(get(&value, "computed-cross-object-repair-effect"));
  for (_upstream, entries_value) in repair_effect_map {
    for entry in as_list(entries_value) {
      let attrs = as_attrs(entry);
      assert!(
        !attrs.contains_key("applied-by-repair-ids"),
        "v0.8 repair-effect entry must NOT carry `applied-by-repair-ids`"
      );
      assert!(
        !as_bool(get(entry, "applied")),
        "v0.8 repair-effect entry must keep applied=false (no promotion)"
      );
    }
  }

  // Chain step does NOT carry `upstream-promoted`.
  let chain_map = as_attrs(get(&value, "transitive-chain-per-object"));
  for (_obj, chain_value) in chain_map {
    for step in as_list(chain_value) {
      let attrs = as_attrs(step);
      assert!(
        !attrs.contains_key("upstream-promoted"),
        "v0.8 chain step must NOT carry `upstream-promoted` (v0.5+/v0.7 territory)"
      );
    }
  }
}

#[test]
fn v0_8_owner_law_file_unchanged_seven_rule_surface() {
  // Invariant 181.
  let value = eval_file(&owner_law_file_path()).expect("owner-law file must evaluate");
  let attrs = as_attrs(&value);

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
        "v0_owner_law.px must still expose `{}` after v0.8; available: {:?}",
        rule,
        attrs.keys().collect::<Vec<_>>()
      )
    });
    assert!(
      matches!(entry, Value::Lambda { .. }),
      "v0_owner_law.px:`{}` must still be a Lambda after v0.8, got {:?}",
      rule,
      entry
    );
  }

  let allowed: BTreeSet<&str> = required_rules.iter().copied().collect();
  for key in attrs.keys() {
    assert!(
      allowed.contains(key.as_str()),
      "v0.8 must NOT add new owner-law rules; v0_owner_law.px carries unexpected key `{}`",
      key
    );
  }
}

#[test]
fn v0_8_1_v0_2_surfaces_byte_equal_to_direct_v0_2() {
  // Invariant 182 (v0.8.1 micro-patch). Strengthens 176
  // from shape-only checks (turn count, Held map size) to
  // byte-for-byte identity with a direct v0.2 evaluation.
  // Closes the v0.8 design-decision claim that "v0.2 trace
  // stays byte-for-byte unchanged" — previously asserted
  // only via shape proxies, now via to_json() equality.
  let v0_8_value = eval_file(&fixture_path()).unwrap();
  let v0_2_value = eval_file(&v0_2_set_aware_path()).expect("v0_2_run_set_aware.px must evaluate");

  for surface in [
    "computed-turns",
    "computed-held-per-object",
    "computed-repair-per-object",
  ] {
    assert_eq!(
      get(&v0_8_value, surface).to_json(),
      get(&v0_2_value, surface).to_json(),
      "v0.8 surface `{}` must be byte-for-byte identical to direct v0_2_run_set_aware.px output",
      surface
    );
  }
}

#[test]
fn v0_8_1_owner_law_loaded_marker_gates_cycle_helds_overlay() {
  // Invariant 183 (v0.8.1 micro-patch). Encodes the v0.6 /
  // v0.8 design invariant "cycle structure is purely
  // relational; cycle-Held materialization is owner-law-
  // gated" directly: positive runner has
  // owner-law-loaded=true AND cycle-Helds 3 entries;
  // negative runner has owner-law-loaded=false AND cycle-
  // Helds empty. Previously this gating was implicit via
  // fixture choice (without_owner_law runner hardcoded
  // `cycleHeldsPerObject = { }`); v0.8.1 makes the gate
  // explicit in the positive runner via `if !ownerLawLoaded
  // then { } else ...` and asserts the conditional via
  // this test.
  let positive = eval_file(&fixture_path()).unwrap();
  assert!(
    as_bool(get(&positive, "owner-law-loaded")),
    "positive runner must inherit owner-law-loaded=true from v0_2_run_set_aware.px"
  );
  assert_eq!(
    as_attrs(get(&positive, "computed-cycle-helds-per-object")).len(),
    3,
    "owner-law-loaded=true → cycle-Held overlay materialized with 3 entries"
  );

  let negative = eval_file(&fixture_without_owner_law_path()).unwrap();
  assert!(
    !as_bool(get(&negative, "owner-law-loaded")),
    "without_owner_law runner must have owner-law-loaded=false from v0_2_run_without_owner_law.px"
  );
  assert!(
    as_attrs(get(&negative, "computed-cycle-helds-per-object")).is_empty(),
    "owner-law-loaded=false → cycle-Held overlay empty (gated)"
  );
}
