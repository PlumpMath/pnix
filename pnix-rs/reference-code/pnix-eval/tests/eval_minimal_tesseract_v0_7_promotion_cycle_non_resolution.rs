//! v0.7 promotion-cycle non-resolution invariant. Synthesis
//! of v0.5.1 (validated promotion via id-match predicate) and
//! v0.6 (visited-set walker + fixture-local cycle-Held
//! overlay) under one runner. Same 2-object cycle from v0.6
//! (`arm_link_1 ↔ arm_link_2`) with two distinct relation
//! kinds, plus v0.5.1's `promoted-repairs` declaring
//! arm_link_1's actual RepairCandidate id
//! (`repair.role-binding`).
//!
//! Load-bearing claim: cycle-Held entries do NOT auto-resolve
//! under valid promotion. Even when the upstream's
//! RepairCandidate id matches and the per-edge Need reopens,
//! the per-object cycle-Held overlay stays at `promoted =
//! false` and stays present. Cycle detection (a structural
//! property derived from the relations graph) and promotion
//! (an input-driven signal that flips per-edge status) are
//! ORTHOGONAL surfaces. Tests 156, 157, 159, 161 are load-
//! bearing.
//!
//! Truth owner:        project-wiki/maps/minimal-tesseract-v0-map.md
//!                     §"v0.7 design decision — promotion-cycle non-
//!                       resolution invariant (cycle-Held survives
//!                       validated promotion)"
//! Active scope:       project-wiki/maps/active-domain-constitution.md Art. 8
//!
//! What this test asserts (16 invariants, indices continued
//! from the v0.6 test's 151; invariants 165..167 are v0.7.1
//! micro-patch — exact 2-step pattern for both chains plus
//! exact cycle-path):
//! 152. world-set / relations identical to v0.6 (2 objects,
//!      2 cycle relations, kinds [depends-on-frame,
//!      mounted-on]); v0-7 / promotion-aware /
//!      id-validation-aware / cycle-aware markers true;
//!      promoted-repairs has arm_link_1 mapped to
//!      [ "repair.role-binding" ].
//! 153. validated runner: arm_link_1's repair-effect entry
//!      toward arm_link_2 has `applied = true` and
//!      `applied-by-repair-ids = [ "repair.role-binding" ]`.
//! 154. validated runner: arm_link_2's Need toward
//!      arm_link_1 has `blocking = false` and
//!      `status = "reopened-by-upstream-promotion"`.
//! 155. validated runner: arm_link_1's Need toward
//!      arm_link_2 still has `blocking = true` and
//!      `status = "blocked"` (transitive non-unblock from
//!      v0.5).
//! 156. **load-bearing — cycle-Held overlay survives
//!      validated promotion**: cycle-Held entries for
//!      arm_link_1 AND arm_link_2 are STILL present; each
//!      carries `held-kind = "dependency-cycle"`,
//!      `promoted = false`, and v0.6-equivalent
//!      `cycle-path`.
//! 157. **load-bearing — cycle still detected on chain step
//!      under validated promotion**: arm_link_1's chain
//!      step at index 1 has `cycle-detected = true` and
//!      `cycle-loop-target = "arm_link_1"`; arm_link_2
//!      symmetrically.
//! 158. Chain step shape is exactly { object-id;
//!      relation-kind; has-held; held-instance-ref;
//!      cycle-detected; cycle-loop-target;
//!      upstream-promoted; } — seven fields.
//! 159. **load-bearing — chain step's upstream-promoted and
//!      cycle-detected are independent**: arm_link_1's
//!      chain closure step (index 1) has BOTH
//!      `cycle-detected = true` AND `upstream-promoted =
//!      true`.
//! 160. without-promotion runner: every repair-effect
//!      entry's `applied = false`; every Need with
//!      upstream-Held has `blocking = true` and
//!      `status = "blocked"`; every chain step's
//!      `upstream-promoted = false`. Cycle-Held overlay
//!      STILL present.
//! 161. **load-bearing — cycle-Held overlay byte-for-byte
//!      identical with vs without promotion**: cycle-Held
//!      is promotion-independent at the byte level.
//! 162. v0.2 trace byte-for-byte identical across both
//!      runners.
//! 163. allowed-delta-only diff: validated vs without-
//!      promotion differ ONLY on { Need.blocking,
//!      Need.status, repair-effect.applied,
//!      repair-effect.applied-by-repair-ids,
//!      chain-step.upstream-promoted }. cycle-detected
//!      and cycle-loop-target are NOT in the allowed
//!      delta set.
//! 164. v0_owner_law.px STILL exposes exactly the same 7
//!      Lambda rules.
//! 165. **v0.7.1 micro-patch — exact arm_link_1 chain
//!      pattern**: chain has length 2; step 0 is exactly
//!      { object-id="arm_link_2",
//!      relation-kind="depends-on-frame",
//!      cycle-detected=false, cycle-loop-target=null,
//!      upstream-promoted=false }; step 1 is exactly
//!      { object-id="arm_link_1",
//!      relation-kind="mounted-on", cycle-detected=true,
//!      cycle-loop-target="arm_link_1",
//!      upstream-promoted=true }.
//! 166. **v0.7.1 micro-patch — exact arm_link_2 chain
//!      pattern (asymmetry)**: chain has length 2;
//!      step 0 is exactly { object-id="arm_link_1",
//!      relation-kind="mounted-on", cycle-detected=false,
//!      cycle-loop-target=null, upstream-promoted=true };
//!      step 1 is exactly { object-id="arm_link_2",
//!      relation-kind="depends-on-frame",
//!      cycle-detected=true,
//!      cycle-loop-target="arm_link_2",
//!      upstream-promoted=false }. Note the asymmetry: in
//!      arm_link_2's chain, the PROMOTED step is at index 0
//!      (visiting arm_link_1) and the CLOSURE step is at
//!      index 1 (visiting back at arm_link_2 which was not
//!      promoted) — opposite arrangement to arm_link_1's
//!      chain.
//! 167. **v0.7.1 micro-patch — exact cycle-path** for both
//!      cycle-Held entries: arm_link_1's cycle-path is
//!      exactly [ "arm_link_1", "arm_link_2",
//!      "arm_link_1" ]; arm_link_2's cycle-path is exactly
//!      [ "arm_link_2", "arm_link_1", "arm_link_2" ].
//!      cycle-Held.promoted reasserted as false.

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

fn validated_path() -> PathBuf {
  fixture_root().join("v0_7_run_promotion_with_cycle.px")
}

fn without_promotion_path() -> PathBuf {
  fixture_root().join("v0_7_run_without_promotion_with_cycle.px")
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

/// For two attrset values, assert that every key NOT in
/// `delta_keys` has byte-for-byte equal value (via to_json).
fn assert_attrs_equal_except(lhs: &Value, rhs: &Value, delta_keys: &[&str], context: &str) {
  let l = as_attrs(lhs);
  let r = as_attrs(rhs);
  let l_keys: BTreeSet<&str> = l.keys().map(|s| s.as_str()).collect();
  let r_keys: BTreeSet<&str> = r.keys().map(|s| s.as_str()).collect();
  assert_eq!(
    l_keys, r_keys,
    "[{}] attrset key sets differ; lhs={:?} rhs={:?}",
    context, l_keys, r_keys
  );
  let delta: BTreeSet<&str> = delta_keys.iter().copied().collect();
  for key in &l_keys {
    if delta.contains(key) {
      continue;
    }
    let lv = l.get(*key).unwrap();
    let rv = r.get(*key).unwrap();
    assert_eq!(
      lv.to_json(),
      rv.to_json(),
      "[{}] key `{}` must be byte-for-byte identical (not in allowed-delta set)",
      context,
      key
    );
  }
}

/// For two attr-of-list-of-attrs surfaces, walk per-object
/// entries and apply `assert_attrs_equal_except` to each
/// matching pair.
fn assert_per_object_entries_match_except(
  lhs_root: &Value,
  rhs_root: &Value,
  delta_keys: &[&str],
  surface_name: &str,
) {
  let l = as_attrs(lhs_root);
  let r = as_attrs(rhs_root);
  let l_keys: BTreeSet<&str> = l.keys().map(|s| s.as_str()).collect();
  let r_keys: BTreeSet<&str> = r.keys().map(|s| s.as_str()).collect();
  assert_eq!(
    l_keys, r_keys,
    "[{}] per-object key sets differ",
    surface_name
  );
  for object_id in l_keys {
    let l_list = as_list(l.get(object_id).unwrap());
    let r_list = as_list(r.get(object_id).unwrap());
    assert_eq!(
      l_list.len(),
      r_list.len(),
      "[{}] entry list length for `{}` differs",
      surface_name,
      object_id
    );
    for (i, (le, re)) in l_list.iter().zip(r_list.iter()).enumerate() {
      assert_attrs_equal_except(
        le,
        re,
        delta_keys,
        &format!("{}[{}][{}]", surface_name, object_id, i),
      );
    }
  }
}

// ---------------------------------------------------------------
// Tests.
// ---------------------------------------------------------------

#[test]
fn v0_7_world_set_relations_promoted_repairs_marker() {
  // Invariant 152.
  let value = eval_file(&validated_path()).expect("v0.7 validated harness must evaluate");

  let world_set = as_list(get(&value, "world-set"));
  assert_eq!(world_set.len(), 2);
  let ids: Vec<&str> = world_set.iter().map(|o| as_str(get(o, "id"))).collect();
  assert_eq!(ids, vec!["arm_link_1", "arm_link_2"]);

  let relations = as_list(get(&value, "relations"));
  assert_eq!(relations.len(), 2);
  let kinds: Vec<&str> = relations.iter().map(|r| as_str(get(r, "kind"))).collect();
  assert_eq!(kinds, vec!["depends-on-frame", "mounted-on"]);
  // Cycle structure assertion.
  assert_eq!(as_str(get(&relations[0], "from")), "arm_link_1");
  assert_eq!(as_str(get(&relations[0], "to")), "arm_link_2");
  assert_eq!(as_str(get(&relations[1], "from")), "arm_link_2");
  assert_eq!(as_str(get(&relations[1], "to")), "arm_link_1");

  assert!(as_bool(get(&value, "v0-7")));
  assert!(as_bool(get(&value, "promotion-aware")));
  assert!(as_bool(get(&value, "id-validation-aware")));
  assert!(as_bool(get(&value, "cycle-aware")));

  let promoted = as_attrs(get(&value, "promoted-repairs"));
  assert_eq!(promoted.len(), 1);
  let arm_link_1_promoted = as_list(promoted.get("arm_link_1").unwrap());
  assert_eq!(arm_link_1_promoted.len(), 1);
  assert_eq!(as_str(&arm_link_1_promoted[0]), "repair.role-binding");
}

#[test]
fn v0_7_validated_arm_link_1_repair_effect_applied_with_real_id() {
  // Invariant 153.
  let value = eval_file(&validated_path()).unwrap();
  let entries = as_list(get_path(
    &value,
    &["computed-cross-object-repair-effect", "arm_link_1"],
  ));
  assert_eq!(entries.len(), 1);
  let entry = &entries[0];
  assert!(as_bool(get(entry, "applied")));
  let applied_by = as_list(get(entry, "applied-by-repair-ids"));
  assert_eq!(applied_by.len(), 1);
  assert_eq!(as_str(&applied_by[0]), "repair.role-binding");
}

#[test]
fn v0_7_validated_arm_link_2_need_unblocked_reopened() {
  // Invariant 154.
  let value = eval_file(&validated_path()).unwrap();
  let needs = as_list(get_path(
    &value,
    &["computed-cross-object-needs-per-object", "arm_link_2"],
  ));
  assert_eq!(needs.len(), 1);
  let need = &needs[0];
  assert_eq!(as_str(get(need, "to")), "arm_link_1");
  assert!(!as_bool(get(need, "blocking")));
  assert_eq!(
    as_str(get(need, "status")),
    "reopened-by-upstream-promotion"
  );
}

#[test]
fn v0_7_validated_arm_link_1_need_still_blocked() {
  // Invariant 155.
  let value = eval_file(&validated_path()).unwrap();
  let needs = as_list(get_path(
    &value,
    &["computed-cross-object-needs-per-object", "arm_link_1"],
  ));
  assert_eq!(needs.len(), 1);
  let need = &needs[0];
  assert_eq!(as_str(get(need, "to")), "arm_link_2");
  assert!(
    as_bool(get(need, "blocking")),
    "arm_link_1 Need stays blocked: arm_link_2 was not promoted"
  );
  assert_eq!(as_str(get(need, "status")), "blocked");
}

#[test]
fn v0_7_cycle_helds_overlay_survives_validated_promotion() {
  // Invariant 156 — load-bearing.
  let value = eval_file(&validated_path()).unwrap();
  let cycle_helds = as_attrs(get(&value, "computed-cycle-helds-per-object"));
  assert_eq!(
    cycle_helds.len(),
    2,
    "validated runner: cycle-Held overlay must STILL have 2 entries; got {}",
    cycle_helds.len()
  );
  for object_id in ["arm_link_1", "arm_link_2"] {
    let entry = cycle_helds.get(object_id).unwrap_or_else(|| {
      panic!(
        "validated runner: cycle-Held overlay must STILL contain `{}`",
        object_id
      )
    });
    assert_eq!(as_str(get(entry, "held-kind")), "dependency-cycle");
    assert!(
      !as_bool(get(entry, "promoted")),
      "validated runner: cycle-Held.promoted MUST stay false even when upstream RepairCandidate id validates"
    );
    assert_eq!(as_str(get(entry, "applies-at")), object_id);
  }
}

#[test]
fn v0_7_cycle_still_detected_under_validated_promotion() {
  // Invariant 157 — load-bearing.
  let value = eval_file(&validated_path()).unwrap();

  let chain1 = as_list(get_path(
    &value,
    &["transitive-chain-per-object", "arm_link_1"],
  ));
  assert_eq!(chain1.len(), 2);
  let step1 = &chain1[1];
  assert!(
    as_bool(get(step1, "cycle-detected")),
    "validated runner: arm_link_1's chain closure step must STILL have cycle-detected=true"
  );
  assert_eq!(as_str(get(step1, "cycle-loop-target")), "arm_link_1");

  let chain2 = as_list(get_path(
    &value,
    &["transitive-chain-per-object", "arm_link_2"],
  ));
  assert_eq!(chain2.len(), 2);
  let step1_2 = &chain2[1];
  assert!(as_bool(get(step1_2, "cycle-detected")));
  assert_eq!(as_str(get(step1_2, "cycle-loop-target")), "arm_link_2");
}

#[test]
fn v0_7_chain_step_shape_is_exactly_seven_fields() {
  // Invariant 158.
  let value = eval_file(&validated_path()).unwrap();
  let expected: BTreeSet<&str> = [
    "object-id",
    "relation-kind",
    "has-held",
    "held-instance-ref",
    "cycle-detected",
    "cycle-loop-target",
    "upstream-promoted",
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
fn v0_7_chain_closure_carries_both_cycle_detected_and_upstream_promoted() {
  // Invariant 159 — load-bearing.
  // arm_link_1's chain closes at arm_link_1 (cycle-detected=true,
  // cycle-loop-target=arm_link_1) AND arm_link_1 is the
  // promoted upstream (upstream-promoted=true). Both facts
  // must be visible on the SAME chain step.
  let value = eval_file(&validated_path()).unwrap();
  let chain1 = as_list(get_path(
    &value,
    &["transitive-chain-per-object", "arm_link_1"],
  ));
  let step1 = &chain1[1];
  assert_eq!(as_str(get(step1, "object-id")), "arm_link_1");
  assert!(
    as_bool(get(step1, "cycle-detected")),
    "step must carry cycle-detected=true"
  );
  assert!(
    as_bool(get(step1, "upstream-promoted")),
    "step must ALSO carry upstream-promoted=true (promoted upstream + cycle closure both visible)"
  );
}

#[test]
fn v0_7_without_promotion_baseline() {
  // Invariant 160.
  let value = eval_file(&without_promotion_path()).unwrap();
  assert!(as_attrs(get(&value, "promoted-repairs")).is_empty());

  // applied=false everywhere.
  let repair_effect_map = as_attrs(get(&value, "computed-cross-object-repair-effect"));
  for (_upstream, entries_value) in repair_effect_map {
    for entry in as_list(entries_value) {
      assert!(!as_bool(get(entry, "applied")));
      assert!(as_list(get(entry, "applied-by-repair-ids")).is_empty());
    }
  }

  // Need: blocking=true, status="blocked" everywhere (both
  // upstreams have Held).
  let needs_map = as_attrs(get(&value, "computed-cross-object-needs-per-object"));
  for (_obj, needs_value) in needs_map {
    for need in as_list(needs_value) {
      assert!(as_bool(get(need, "blocking")));
      assert_eq!(as_str(get(need, "status")), "blocked");
    }
  }

  // Chain step upstream-promoted=false everywhere.
  let chain_map = as_attrs(get(&value, "transitive-chain-per-object"));
  for (_obj, chain_value) in chain_map {
    for step in as_list(chain_value) {
      assert!(!as_bool(get(step, "upstream-promoted")));
    }
  }

  // Cycle-Held overlay STILL present (same 2 entries).
  let cycle_helds = as_attrs(get(&value, "computed-cycle-helds-per-object"));
  assert_eq!(cycle_helds.len(), 2);
}

#[test]
fn v0_7_cycle_helds_byte_identical_with_vs_without_promotion() {
  // Invariant 161 — load-bearing.
  let validated = eval_file(&validated_path()).unwrap();
  let without_p = eval_file(&without_promotion_path()).unwrap();

  let v_cycle = get(&validated, "computed-cycle-helds-per-object");
  let w_cycle = get(&without_p, "computed-cycle-helds-per-object");
  assert_eq!(
    v_cycle.to_json(),
    w_cycle.to_json(),
    "computed-cycle-helds-per-object must be byte-for-byte identical between with-promotion and without-promotion runners (cycle-Held is promotion-independent)"
  );
}

#[test]
fn v0_7_v0_2_trace_unchanged_across_runners() {
  // Invariant 162.
  let validated = eval_file(&validated_path()).unwrap();
  let without_p = eval_file(&without_promotion_path()).unwrap();

  for surface in [
    "computed-held-per-object",
    "computed-repair-per-object",
    "computed-turns",
  ] {
    assert_eq!(
      get(&validated, surface).to_json(),
      get(&without_p, surface).to_json(),
      "v0.2 trace surface `{}` must be byte-for-byte identical across v0.7 runners",
      surface
    );
  }
}

#[test]
fn v0_7_allowed_delta_only_diff_validated_vs_without() {
  // Invariant 163.
  let validated = eval_file(&validated_path()).unwrap();
  let without_p = eval_file(&without_promotion_path()).unwrap();

  // Need: only blocking + status may differ.
  assert_per_object_entries_match_except(
    get(&validated, "computed-cross-object-needs-per-object"),
    get(&without_p, "computed-cross-object-needs-per-object"),
    &["blocking", "status"],
    "computed-cross-object-needs-per-object",
  );

  // Repair-effect: only applied + applied-by-repair-ids may differ.
  assert_per_object_entries_match_except(
    get(&validated, "computed-cross-object-repair-effect"),
    get(&without_p, "computed-cross-object-repair-effect"),
    &["applied", "applied-by-repair-ids"],
    "computed-cross-object-repair-effect",
  );

  // Chain steps: only upstream-promoted may differ.
  // CRUCIAL: cycle-detected and cycle-loop-target are NOT in
  // the allowed delta set — cycle structure must be byte-for-
  // byte identical between with and without promotion.
  assert_per_object_entries_match_except(
    get(&validated, "transitive-chain-per-object"),
    get(&without_p, "transitive-chain-per-object"),
    &["upstream-promoted"],
    "transitive-chain-per-object",
  );
}

#[test]
fn v0_7_owner_law_file_unchanged_seven_rule_surface() {
  // Invariant 164.
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
        "v0_owner_law.px must still expose `{}` after v0.7; available: {:?}",
        rule,
        attrs.keys().collect::<Vec<_>>()
      )
    });
    assert!(
      matches!(entry, Value::Lambda { .. }),
      "v0_owner_law.px:`{}` must still be a Lambda after v0.7, got {:?}",
      rule,
      entry
    );
  }

  let allowed: BTreeSet<&str> = required_rules.iter().copied().collect();
  for key in attrs.keys() {
    assert!(
      allowed.contains(key.as_str()),
      "v0.7 must NOT add new owner-law rules; v0_owner_law.px carries unexpected key `{}`",
      key
    );
  }
}

#[test]
fn v0_7_1_arm_link_1_chain_exact_promotion_cycle_pattern() {
  // Invariant 165 (v0.7.1 micro-patch). Pin the EXACT 2-step
  // pattern for arm_link_1's chain under validated promotion:
  // step 0 is the depends-on-frame edge to arm_link_2 (no
  // cycle, no promotion); step 1 is the mounted-on closure
  // back at arm_link_1 (cycle AND promotion both true on the
  // SAME step). A future regression that swaps step order,
  // mis-attributes relation-kind, or decouples the cycle/
  // promotion fields surfaces immediately.
  let value = eval_file(&validated_path()).unwrap();
  let chain = as_list(get_path(
    &value,
    &["transitive-chain-per-object", "arm_link_1"],
  ));
  assert_eq!(chain.len(), 2, "arm_link_1 chain must have exactly 2 steps");

  let step0 = &chain[0];
  assert_eq!(as_str(get(step0, "object-id")), "arm_link_2");
  assert_eq!(as_str(get(step0, "relation-kind")), "depends-on-frame");
  assert!(!as_bool(get(step0, "cycle-detected")));
  assert!(is_null(get(step0, "cycle-loop-target")));
  assert!(
    !as_bool(get(step0, "upstream-promoted")),
    "step 0 visits arm_link_2 which was not promoted"
  );

  let step1 = &chain[1];
  assert_eq!(as_str(get(step1, "object-id")), "arm_link_1");
  assert_eq!(as_str(get(step1, "relation-kind")), "mounted-on");
  assert!(as_bool(get(step1, "cycle-detected")));
  assert_eq!(as_str(get(step1, "cycle-loop-target")), "arm_link_1");
  assert!(
    as_bool(get(step1, "upstream-promoted")),
    "step 1 closure target arm_link_1 IS promoted; same step carries cycle + promoted simultaneously"
  );
}

#[test]
fn v0_7_1_arm_link_2_chain_exact_promotion_cycle_pattern_asymmetric() {
  // Invariant 166 (v0.7.1 micro-patch). Mirror of arm_link_1
  // but with the promoted/cycle pattern intentionally
  // ASYMMETRIC: arm_link_2's chain visits arm_link_1 first
  // (which WAS promoted), then closes at arm_link_2 (which
  // was NOT promoted). So the promoted-true step is at
  // index 0 and the cycle-detected-true step is at index 1
  // — they do NOT coincide on this chain. Together with
  // invariant 165 this proves the chain step's
  // cycle-detected and upstream-promoted are populated
  // independently per step, not from a single global
  // promotion flag.
  let value = eval_file(&validated_path()).unwrap();
  let chain = as_list(get_path(
    &value,
    &["transitive-chain-per-object", "arm_link_2"],
  ));
  assert_eq!(chain.len(), 2, "arm_link_2 chain must have exactly 2 steps");

  let step0 = &chain[0];
  assert_eq!(as_str(get(step0, "object-id")), "arm_link_1");
  assert_eq!(as_str(get(step0, "relation-kind")), "mounted-on");
  assert!(!as_bool(get(step0, "cycle-detected")));
  assert!(is_null(get(step0, "cycle-loop-target")));
  assert!(
    as_bool(get(step0, "upstream-promoted")),
    "step 0 visits arm_link_1 which IS promoted"
  );

  let step1 = &chain[1];
  assert_eq!(as_str(get(step1, "object-id")), "arm_link_2");
  assert_eq!(as_str(get(step1, "relation-kind")), "depends-on-frame");
  assert!(as_bool(get(step1, "cycle-detected")));
  assert_eq!(as_str(get(step1, "cycle-loop-target")), "arm_link_2");
  assert!(
    !as_bool(get(step1, "upstream-promoted")),
    "step 1 closure target arm_link_2 was NOT promoted; cycle and promoted are NOT both true on this step"
  );
}

#[test]
fn v0_7_1_cycle_helds_exact_path_and_promoted_false_pinned() {
  // Invariant 167 (v0.7.1 micro-patch). Pin the exact
  // cycle-path list for both cycle-Held entries (carrying
  // forward v0.6.1's exact-path discipline into v0.7) and
  // re-assert promoted=false even under validated
  // promotion. This is the cycle-Held-level analogue of
  // invariants 165 / 166's chain-level pinning.
  let value = eval_file(&validated_path()).unwrap();

  let entry_1 = get_path(&value, &["computed-cycle-helds-per-object", "arm_link_1"]);
  let path_1: Vec<&str> = as_list(get(entry_1, "cycle-path"))
    .iter()
    .map(|p| as_str(p))
    .collect();
  assert_eq!(
    path_1,
    vec!["arm_link_1", "arm_link_2", "arm_link_1"],
    "arm_link_1 cycle-path must carry forward v0.6.1's exact triple under validated promotion"
  );
  assert_eq!(as_str(get(entry_1, "held-kind")), "dependency-cycle");
  assert!(
    !as_bool(get(entry_1, "promoted")),
    "arm_link_1 cycle-Held.promoted MUST stay false even though arm_link_1's RepairCandidate id validated"
  );

  let entry_2 = get_path(&value, &["computed-cycle-helds-per-object", "arm_link_2"]);
  let path_2: Vec<&str> = as_list(get(entry_2, "cycle-path"))
    .iter()
    .map(|p| as_str(p))
    .collect();
  assert_eq!(
    path_2,
    vec!["arm_link_2", "arm_link_1", "arm_link_2"],
    "arm_link_2 cycle-path must carry forward v0.6.1's exact triple under validated promotion"
  );
  assert_eq!(as_str(get(entry_2, "held-kind")), "dependency-cycle");
  assert!(!as_bool(get(entry_2, "promoted")));
}
