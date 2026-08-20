//! v0.6 cycle handling — livelocked Helds. The chain walker
//! gains visited-set termination so cycles surface as bounded-
//! length chains with `cycle-detected = true` and
//! `cycle-loop-target = <upstream>` on the closure step. A
//! new top-level surface `computed-cycle-helds-per-object`
//! emits a fixture-local Held entry per cycle-participating
//! object with `held-kind = "dependency-cycle"` and
//! `promoted = false`. Without owner-law the cycle structure
//! is still visible from the relations alone, but the cycle-
//! Held overlay is empty (gated on owner-law presence).
//! Tests 140 (walker termination), 141 (cycle-detected
//! surfaced), 144 (cycle-Held overlay), and 147 (without-
//! owner-law cycle visibility) are the load-bearing v0.6
//! invariants.
//!
//! Truth owner:        project-wiki/maps/minimal-tesseract-v0-map.md
//!                     §"v0.6 design decision — cycle handling
//!                       (livelocked Helds; chain walker terminates;
//!                       promotion does not auto-resolve)"
//! Active scope:       project-wiki/maps/active-domain-constitution.md Art. 8
//!
//! What this test asserts (13 invariants, indices continued
//! from the v0.5.1 test's 138):
//! 139. world-set has 2 SourceObjects [arm_link_1,
//!      arm_link_2]; relations form a 2-edge cycle with
//!      kinds [depends-on-frame, mounted-on] in declared
//!      order; v0-6 / chain-aware / cycle-aware markers
//!      true.
//! 140. **load-bearing — walker terminates on cycle**:
//!      `transitive-chain-per-object[arm_link_1]` has
//!      bounded length (depth <= 2). The walker does NOT
//!      infinite-recurse.
//! 141. **load-bearing — exact arm_link_1 chain sequence**
//!      (v0.6.1 micro-patch strengthening): chain has
//!      length 2; step 0 is { object-id="arm_link_2",
//!      relation-kind="depends-on-frame",
//!      cycle-detected=false, cycle-loop-target=null };
//!      step 1 is { object-id="arm_link_1",
//!      relation-kind="mounted-on", cycle-detected=true,
//!      cycle-loop-target="arm_link_1" }.
//! 142. **exact arm_link_2 chain sequence** (v0.6.1
//!      micro-patch strengthening): chain has length 2;
//!      step 0 is { object-id="arm_link_1",
//!      relation-kind="mounted-on", cycle-detected=false,
//!      cycle-loop-target=null }; step 1 is
//!      { object-id="arm_link_2",
//!      relation-kind="depends-on-frame",
//!      cycle-detected=true,
//!      cycle-loop-target="arm_link_2" }.
//! 143. Chain step shape is exactly { object-id;
//!      relation-kind; has-held; held-instance-ref;
//!      cycle-detected; cycle-loop-target; } — six
//!      fields, no extras.
//! 144. **load-bearing — cycle-Held overlay present**:
//!      computed-cycle-helds-per-object has entries for
//!      arm_link_1 and arm_link_2; each carries
//!      `held-kind = "dependency-cycle"`,
//!      `promoted = false`, and a non-null
//!      `cycle-loop-target`.
//! 145. **exact cycle-path** (v0.6.1 micro-patch
//!      strengthening): arm_link_1's cycle-path is
//!      exactly [ "arm_link_1", "arm_link_2",
//!      "arm_link_1" ]; arm_link_2's cycle-path is
//!      exactly [ "arm_link_2", "arm_link_1",
//!      "arm_link_2" ].
//! 146. v0.2 trajectory surfaces still present
//!      (computed-turns has 6 turns; per-object Held /
//!      Repair maps still 2 entries — arm_link_1 and
//!      arm_link_2 retain their structural-binding-
//!      conflict Helds). The v0.6 cycle-Held overlay is
//!      ADDITIVE; it does not replace or mutate the
//!      v0.2 Held map.
//! 147. **load-bearing — without-owner-law cycle
//!      visibility (both objects, v0.6.1 micro-patch
//!      symmetry)**: the chain walker still produces
//!      `cycle-detected = true` and a non-null
//!      `cycle-loop-target` at the closure point for
//!      BOTH arm_link_1 and arm_link_2 — cycle structure
//!      is visible from relations alone. Every step on
//!      both chains has `has-held = false` and
//!      `held-instance-ref = null`.
//! 148. without-owner-law: computed-cycle-helds-per-
//!      object is empty — the overlay is gated on
//!      owner-law presence.
//! 149. relation-kind plurality preserved: the chain
//!      step's `relation-kind` field carries the
//!      producing edge's kind (depends-on-frame or
//!      mounted-on).
//! 150. promotion machinery is NOT present in v0.6:
//!      Need does NOT carry `status`, repair-effect
//!      does NOT carry `applied` /
//!      `applied-by-repair-ids`, chain step does NOT
//!      carry `upstream-promoted`.
//! 151. v0_owner_law.px STILL exposes exactly the same 7
//!      Lambda rules.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------
// Helpers — duplicated from prior tests (integration test
// crates compile separately).
// ---------------------------------------------------------------

fn fixture_root() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../../fixtures/minimal-tesseract-v0")
}

fn fixture_path() -> PathBuf {
  fixture_root().join("v0_6_run_cycle_aware.px")
}

fn fixture_without_owner_law_path() -> PathBuf {
  fixture_root().join("v0_6_run_without_owner_law.px")
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

// ---------------------------------------------------------------
// Tests.
// ---------------------------------------------------------------

#[test]
fn v0_6_world_set_two_objects_cycle_relations_with_markers() {
  // Invariant 139.
  let value = eval_file(&fixture_path()).expect("v0.6 cycle-aware harness must evaluate");

  let world_set = as_list(get(&value, "world-set"));
  assert_eq!(world_set.len(), 2);
  let ids: Vec<&str> = world_set.iter().map(|o| as_str(get(o, "id"))).collect();
  assert_eq!(ids, vec!["arm_link_1", "arm_link_2"]);

  let relations = as_list(get(&value, "relations"));
  assert_eq!(relations.len(), 2);
  let kinds: Vec<&str> = relations.iter().map(|r| as_str(get(r, "kind"))).collect();
  assert_eq!(kinds, vec!["depends-on-frame", "mounted-on"]);
  // Confirm cycle structure from relations alone.
  assert_eq!(as_str(get(&relations[0], "from")), "arm_link_1");
  assert_eq!(as_str(get(&relations[0], "to")), "arm_link_2");
  assert_eq!(as_str(get(&relations[1], "from")), "arm_link_2");
  assert_eq!(as_str(get(&relations[1], "to")), "arm_link_1");

  assert!(as_bool(get(&value, "v0-6")));
  assert!(as_bool(get(&value, "cycle-aware")));
  assert!(as_bool(get(&value, "chain-aware")));
}

#[test]
fn v0_6_walker_terminates_on_cycle_bounded_chain_length() {
  // Invariant 140 — load-bearing.
  let value = eval_file(&fixture_path()).unwrap();
  let chain_map = as_attrs(get(&value, "transitive-chain-per-object"));
  for (object_id, chain_value) in chain_map {
    let chain = as_list(chain_value);
    assert!(
      chain.len() <= 2,
      "v0.6 walker must terminate at depth <= 2 for object {}; got chain length {}",
      object_id,
      chain.len()
    );
  }
}

#[test]
fn v0_6_arm_link_1_chain_exact_two_step_sequence() {
  // Invariant 141 — load-bearing (v0.6.1 micro-patch
  // strengthening). Pin the EXACT 2-step sequence so a
  // future regression that swaps step order, drops the
  // cycle-detected flag, or mis-attributes relation-kind
  // surfaces immediately.
  let value = eval_file(&fixture_path()).unwrap();
  let chain1 = as_list(get_path(
    &value,
    &["transitive-chain-per-object", "arm_link_1"],
  ));
  assert_eq!(
    chain1.len(),
    2,
    "arm_link_1's chain must have exactly 2 steps; got {}",
    chain1.len()
  );

  // Step 0 — depends-on-frame edge to arm_link_2 (no cycle yet).
  let step0 = &chain1[0];
  assert_eq!(as_str(get(step0, "object-id")), "arm_link_2");
  assert_eq!(as_str(get(step0, "relation-kind")), "depends-on-frame");
  assert!(!as_bool(get(step0, "cycle-detected")));
  assert!(is_null(get(step0, "cycle-loop-target")));

  // Step 1 — mounted-on edge back to arm_link_1 (cycle closure).
  let step1 = &chain1[1];
  assert_eq!(as_str(get(step1, "object-id")), "arm_link_1");
  assert_eq!(as_str(get(step1, "relation-kind")), "mounted-on");
  assert!(as_bool(get(step1, "cycle-detected")));
  assert_eq!(as_str(get(step1, "cycle-loop-target")), "arm_link_1");
}

#[test]
fn v0_6_arm_link_2_chain_exact_two_step_sequence() {
  // Invariant 142 (v0.6.1 micro-patch strengthening).
  let value = eval_file(&fixture_path()).unwrap();
  let chain2 = as_list(get_path(
    &value,
    &["transitive-chain-per-object", "arm_link_2"],
  ));
  assert_eq!(
    chain2.len(),
    2,
    "arm_link_2's chain must have exactly 2 steps; got {}",
    chain2.len()
  );

  // Step 0 — mounted-on edge to arm_link_1 (no cycle yet).
  let step0 = &chain2[0];
  assert_eq!(as_str(get(step0, "object-id")), "arm_link_1");
  assert_eq!(as_str(get(step0, "relation-kind")), "mounted-on");
  assert!(!as_bool(get(step0, "cycle-detected")));
  assert!(is_null(get(step0, "cycle-loop-target")));

  // Step 1 — depends-on-frame edge back to arm_link_2 (cycle closure).
  let step1 = &chain2[1];
  assert_eq!(as_str(get(step1, "object-id")), "arm_link_2");
  assert_eq!(as_str(get(step1, "relation-kind")), "depends-on-frame");
  assert!(as_bool(get(step1, "cycle-detected")));
  assert_eq!(as_str(get(step1, "cycle-loop-target")), "arm_link_2");
}

#[test]
fn v0_6_chain_step_shape_is_exactly_six_fields() {
  // Invariant 143.
  let value = eval_file(&fixture_path()).unwrap();
  let chain_map = as_attrs(get(&value, "transitive-chain-per-object"));
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
fn v0_6_cycle_helds_overlay_present_with_dependency_cycle_kind() {
  // Invariant 144 — load-bearing.
  let value = eval_file(&fixture_path()).unwrap();
  let cycle_helds = as_attrs(get(&value, "computed-cycle-helds-per-object"));
  assert_eq!(cycle_helds.len(), 2);
  for object_id in ["arm_link_1", "arm_link_2"] {
    let entry = cycle_helds.get(object_id).unwrap_or_else(|| {
      panic!(
        "computed-cycle-helds-per-object must contain `{}`; got keys {:?}",
        object_id,
        cycle_helds.keys().collect::<Vec<_>>()
      )
    });
    assert_eq!(as_str(get(entry, "held-kind")), "dependency-cycle");
    assert!(!as_bool(get(entry, "promoted")));
    assert!(
      !is_null(get(entry, "cycle-loop-target")),
      "cycle-Held entry for {} must have non-null cycle-loop-target",
      object_id
    );
    assert_eq!(as_str(get(entry, "applies-at")), object_id);
  }
}

#[test]
fn v0_6_cycle_held_exact_cycle_path() {
  // Invariant 145 (v0.6.1 micro-patch strengthening). Pin
  // the exact cycle-path for each cycle-Held entry so
  // a future regression that reverses the path direction
  // or drops the cycle-closure step surfaces immediately.
  let value = eval_file(&fixture_path()).unwrap();

  let entry_1 = get_path(&value, &["computed-cycle-helds-per-object", "arm_link_1"]);
  let path_1: Vec<&str> = as_list(get(entry_1, "cycle-path"))
    .iter()
    .map(|p| as_str(p))
    .collect();
  assert_eq!(
    path_1,
    vec!["arm_link_1", "arm_link_2", "arm_link_1"],
    "arm_link_1's cycle-path must be [start, upstream, cycle-closure]"
  );

  let entry_2 = get_path(&value, &["computed-cycle-helds-per-object", "arm_link_2"]);
  let path_2: Vec<&str> = as_list(get(entry_2, "cycle-path"))
    .iter()
    .map(|p| as_str(p))
    .collect();
  assert_eq!(
    path_2,
    vec!["arm_link_2", "arm_link_1", "arm_link_2"],
    "arm_link_2's cycle-path must mirror arm_link_1's (symmetric 2-cycle)"
  );
}

#[test]
fn v0_6_v0_2_trace_unchanged_under_cycle_overlay() {
  // Invariant 146.
  let value = eval_file(&fixture_path()).unwrap();

  let turns = as_list(get(&value, "computed-turns"));
  assert_eq!(turns.len(), 6);
  let turn_ids: Vec<i64> = turns.iter().map(|t| as_int(get(t, "turn-id"))).collect();
  assert_eq!(turn_ids, vec![0, 1, 2, 3, 4, 5]);

  let held_map = as_attrs(get(&value, "computed-held-per-object"));
  assert_eq!(held_map.len(), 2);
  for object_id in ["arm_link_1", "arm_link_2"] {
    let held = held_map
      .get(object_id)
      .unwrap_or_else(|| panic!("v0.2 held map must contain `{}`", object_id));
    assert_eq!(
      as_str(get(held, "held-kind")),
      "structural-binding-conflict",
      "v0.6 must NOT mutate v0.2's per-object Held entries"
    );
  }

  let repair_map = as_attrs(get(&value, "computed-repair-per-object"));
  assert_eq!(repair_map.len(), 2);
}

#[test]
fn v0_6_without_owner_law_cycle_structure_visible_held_collapsed() {
  // Invariant 147 — load-bearing (v0.6.1 micro-patch
  // symmetry). Walk BOTH arm_link_1 and arm_link_2 chains;
  // confirm cycle structure is visible from relations
  // alone for both, while every step's has-held=false
  // and held-instance-ref=null.
  let value = eval_file(&fixture_without_owner_law_path()).unwrap();
  assert!(!as_bool(get(&value, "owner-law-loaded")));

  for (start_id, expected_loop_target) in
    [("arm_link_1", "arm_link_1"), ("arm_link_2", "arm_link_2")]
  {
    let chain = as_list(get_path(&value, &["transitive-chain-per-object", start_id]));
    assert!(
      !chain.is_empty(),
      "without owner-law: {}'s chain must be non-empty",
      start_id
    );
    let cycle_step = chain
      .iter()
      .find(|s| as_bool(get(s, "cycle-detected")))
      .unwrap_or_else(|| {
        panic!(
          "without owner-law: {}'s chain must STILL surface cycle-detected step",
          start_id
        )
      });
    assert_eq!(
      as_str(get(cycle_step, "cycle-loop-target")),
      expected_loop_target,
      "without owner-law: {}'s cycle closure must point back at {}",
      start_id,
      expected_loop_target
    );

    for step in chain {
      assert!(
        !as_bool(get(step, "has-held")),
        "without owner-law: {} chain step has-held must be false",
        start_id
      );
      assert!(
        is_null(get(step, "held-instance-ref")),
        "without owner-law: {} chain step held-instance-ref must be null",
        start_id
      );
    }
  }
}

#[test]
fn v0_6_without_owner_law_cycle_helds_overlay_empty() {
  // Invariant 148.
  let value = eval_file(&fixture_without_owner_law_path()).unwrap();
  let cycle_helds = as_attrs(get(&value, "computed-cycle-helds-per-object"));
  assert!(
    cycle_helds.is_empty(),
    "without owner-law: cycle-Held overlay must be empty (gated on owner-law presence); got keys {:?}",
    cycle_helds.keys().collect::<Vec<_>>()
  );
}

#[test]
fn v0_6_relation_kind_plurality_preserved_in_chain() {
  // Invariant 149.
  let value = eval_file(&fixture_path()).unwrap();
  // arm_link_1's chain step 0 (upstream = arm_link_2) was
  // produced by the depends-on-frame edge.
  let chain1 = as_list(get_path(
    &value,
    &["transitive-chain-per-object", "arm_link_1"],
  ));
  let step0 = &chain1[0];
  assert_eq!(as_str(get(step0, "object-id")), "arm_link_2");
  assert_eq!(as_str(get(step0, "relation-kind")), "depends-on-frame");

  // arm_link_2's chain step 0 (upstream = arm_link_1) was
  // produced by the mounted-on edge.
  let chain2 = as_list(get_path(
    &value,
    &["transitive-chain-per-object", "arm_link_2"],
  ));
  let step0_2 = &chain2[0];
  assert_eq!(as_str(get(step0_2, "object-id")), "arm_link_1");
  assert_eq!(as_str(get(step0_2, "relation-kind")), "mounted-on");
}

#[test]
fn v0_6_no_promotion_machinery_in_need_repair_or_chain() {
  // Invariant 150.
  let value = eval_file(&fixture_path()).unwrap();

  // Need does NOT carry `status`.
  let needs_map = as_attrs(get(&value, "computed-cross-object-needs-per-object"));
  for (_obj, needs_value) in needs_map {
    for need in as_list(needs_value) {
      let attrs = as_attrs(need);
      assert!(
        !attrs.contains_key("status"),
        "v0.6 Need must NOT carry `status` (that is v0.5+ promotion-aware territory)"
      );
    }
  }

  // Repair-effect does NOT carry `applied-by-repair-ids`. (It
  // may carry `applied = false` from v0.4c shape, but not the
  // promotion-list.)
  let repair_effect_map = as_attrs(get(&value, "computed-cross-object-repair-effect"));
  for (_upstream, entries_value) in repair_effect_map {
    for entry in as_list(entries_value) {
      let attrs = as_attrs(entry);
      assert!(
        !attrs.contains_key("applied-by-repair-ids"),
        "v0.6 repair-effect entry must NOT carry `applied-by-repair-ids` (v0.5+ territory)"
      );
      assert!(
        !as_bool(get(entry, "applied")),
        "v0.6 repair-effect entry must keep applied=false (no promotion)"
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
        "v0.6 chain step must NOT carry `upstream-promoted` (v0.5+ territory)"
      );
    }
  }
}

#[test]
fn v0_6_owner_law_file_unchanged_seven_rule_surface() {
  // Invariant 151.
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
        "v0_owner_law.px must still expose `{}` after v0.6; available: {:?}",
        rule,
        attrs.keys().collect::<Vec<_>>()
      )
    });
    assert!(
      matches!(entry, Value::Lambda { .. }),
      "v0_owner_law.px:`{}` must still be a Lambda after v0.6, got {:?}",
      rule,
      entry
    );
  }

  let allowed: BTreeSet<&str> = required_rules.iter().copied().collect();
  for key in attrs.keys() {
    assert!(
      allowed.contains(key.as_str()),
      "v0.6 must NOT add new owner-law rules; v0_owner_law.px carries unexpected key `{}`",
      key
    );
  }
}
