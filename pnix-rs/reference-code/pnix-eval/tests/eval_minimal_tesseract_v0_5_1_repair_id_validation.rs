//! v0.5.1 RepairCandidate id verification + allowed-delta-only
//! diff. v0.5 only consulted the **keys** of `promoted-repairs`;
//! the `repair-id` list values were carried forward without
//! validation. v0.5.1 gates promotion on id match: a
//! promoted-repairs input whose value list does not intersect
//! with the upstream's actual RepairCandidate id is rejected.
//! The Need status enum gains `"promotion-rejected-invalid-
//! repair-id"` to distinguish "promotion attempted but
//! rejected" from "no promotion attempted". v0.5.1 also adds
//! a structural-diff guard: only five sanctioned fields may
//! differ between the validated runner and the without-
//! promotion / wrong-id runners. Tests 135 (validated vs
//! without-promotion allowed-delta-only) and 136 (validated
//! vs wrong-id allowed-delta-only) are the load-bearing
//! v0.5.1 invariants.
//!
//! Truth owner:        project-wiki/maps/minimal-tesseract-v0-map.md
//!                     §"v0.5.1 design decision — RepairCandidate id
//!                       verification + allowed-delta-only diff"
//! Active scope:       project-wiki/maps/active-domain-constitution.md Art. 8
//!
//! What this test asserts (11 invariants, indices continued
//! from the v0.5 test's 127):
//! 128. world-set / relations identical to v0.4c / v0.5;
//!      v0-5-1 / promotion-aware / id-validation-aware
//!      markers true; the validated runner's
//!      promoted-repairs has arm_link_1 mapped to the
//!      actual RepairCandidate id `repair.role-binding`
//!      (matches `v0_owner_law.buildRepairCandidate`).
//! 129. validated runner: arm_link_1's repair-effect entry
//!      toward arm_link_2 has `applied = true`,
//!      `applied-by-repair-ids = [ "repair.role-binding" ]`,
//!      `relation-kind = "mounted-on"`.
//! 130. validated runner: arm_link_2's Need toward
//!      arm_link_1 has `blocking = false` and
//!      `status = "reopened-by-upstream-promotion"`.
//! 131. validated runner: arm_link_3's Need toward
//!      arm_link_2 still has `blocking = true` and
//!      `status = "blocked"`.
//! 132. wrong-id runner: arm_link_1's repair-effect entry
//!      has `applied = false` and
//!      `applied-by-repair-ids = [ ]`; the proposed wrong
//!      id contributes nothing.
//! 133. wrong-id runner: arm_link_2's Need has
//!      `blocking = true` and
//!      `status = "promotion-rejected-invalid-repair-id"`.
//! 134. wrong-id runner: arm_link_3's chain step at index
//!      1 (object-id = arm_link_1) has
//!      `upstream-promoted = false` — rejected promotion
//!      does NOT mark the chain.
//! 135. **load-bearing — allowed-delta-only diff (validated
//!      vs without-promotion)**: every Need / repair-effect
//!      entry / chain step in the validated runner agrees
//!      byte-for-byte with the v0.5 without-promotion
//!      runner EXCEPT on { Need.blocking, Need.status,
//!      repair-effect.applied,
//!      repair-effect.applied-by-repair-ids,
//!      chain-step.upstream-promoted }.
//! 136. **load-bearing — allowed-delta-only diff (validated
//!      vs wrong-id)**: same allowed-delta-only constraint
//!      applies between the validated and wrong-id runners.
//!      v0.2 trace (computed-held-per-object / computed-
//!      repair-per-object / computed-turns) is byte-for-
//!      byte identical across all three runners.
//! 137. v0.5.1 status enum across validated / wrong-id /
//!      without-promotion runners is a subset of
//!      { "blocked", "reopened-by-upstream-promotion",
//!      "promotion-rejected-invalid-repair-id",
//!      "non-blocking-no-held" }.
//! 138. v0_owner_law.px STILL exposes exactly the same 7
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

fn validated_path() -> PathBuf {
  fixture_root().join("v0_5_1_run_validated_promotion.px")
}

fn wrong_id_path() -> PathBuf {
  fixture_root().join("v0_5_1_run_wrong_repair_id.px")
}

fn without_promotion_path() -> PathBuf {
  fixture_root().join("v0_5_run_without_promotion.px")
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

/// For two attrset values, assert that every key NOT in
/// `delta_keys` has byte-for-byte equal value (via to_json).
/// Keys in `delta_keys` are skipped (they may differ).
/// Both sides must have exactly the same key set.
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

/// For two attr-of-list-of-attrs surfaces, walk the per-object
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
    "[{}] per-object key sets differ; lhs={:?} rhs={:?}",
    surface_name, l_keys, r_keys
  );
  for object_id in l_keys {
    let l_list = as_list(l.get(object_id).unwrap());
    let r_list = as_list(r.get(object_id).unwrap());
    assert_eq!(
      l_list.len(),
      r_list.len(),
      "[{}] entry list length for `{}` differs ({} vs {})",
      surface_name,
      object_id,
      l_list.len(),
      r_list.len()
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
fn v0_5_1_world_set_relations_validated_promoted_repairs() {
  // Invariant 128.
  let value = eval_file(&validated_path()).expect("v0.5.1 validated harness must evaluate");

  let world_set = as_list(get(&value, "world-set"));
  assert_eq!(world_set.len(), 3);
  let ids: Vec<&str> = world_set.iter().map(|o| as_str(get(o, "id"))).collect();
  assert_eq!(ids, vec!["arm_link_1", "arm_link_2", "arm_link_3"]);

  let relations = as_list(get(&value, "relations"));
  assert_eq!(relations.len(), 2);
  let kinds: Vec<&str> = relations.iter().map(|r| as_str(get(r, "kind"))).collect();
  assert_eq!(kinds, vec!["depends-on-frame", "mounted-on"]);

  assert!(as_bool(get(&value, "v0-5-1")));
  assert!(as_bool(get(&value, "promotion-aware")));
  assert!(as_bool(get(&value, "id-validation-aware")));

  let promoted = as_attrs(get(&value, "promoted-repairs"));
  assert_eq!(promoted.len(), 1);
  let arm_link_1_promoted = as_list(promoted.get("arm_link_1").unwrap());
  assert_eq!(arm_link_1_promoted.len(), 1);
  assert_eq!(
    as_str(&arm_link_1_promoted[0]),
    "repair.role-binding",
    "validated runner's promoted id must match v0_owner_law.buildRepairCandidate.id verbatim"
  );
}

#[test]
fn v0_5_1_validated_arm_link_1_repair_effect_applied_with_real_id() {
  // Invariant 129.
  let value = eval_file(&validated_path()).unwrap();
  let entries = as_list(get_path(
    &value,
    &["computed-cross-object-repair-effect", "arm_link_1"],
  ));
  assert_eq!(entries.len(), 1);
  let entry = &entries[0];
  assert!(
    as_bool(get(entry, "applied")),
    "validated runner: applied must be true"
  );
  let applied_by = as_list(get(entry, "applied-by-repair-ids"));
  assert_eq!(applied_by.len(), 1);
  assert_eq!(as_str(&applied_by[0]), "repair.role-binding");
  assert_eq!(as_str(get(entry, "relation-kind")), "mounted-on");
  assert_eq!(as_str(get(entry, "downstream-object")), "arm_link_2");
}

#[test]
fn v0_5_1_validated_arm_link_2_need_unblocked_reopened() {
  // Invariant 130.
  let value = eval_file(&validated_path()).unwrap();
  let needs = as_list(get_path(
    &value,
    &["computed-cross-object-needs-per-object", "arm_link_2"],
  ));
  assert_eq!(needs.len(), 1);
  let need = &needs[0];
  assert!(!as_bool(get(need, "blocking")));
  assert_eq!(
    as_str(get(need, "status")),
    "reopened-by-upstream-promotion"
  );
}

#[test]
fn v0_5_1_validated_arm_link_3_need_still_blocked() {
  // Invariant 131.
  let value = eval_file(&validated_path()).unwrap();
  let needs = as_list(get_path(
    &value,
    &["computed-cross-object-needs-per-object", "arm_link_3"],
  ));
  assert_eq!(needs.len(), 1);
  let need = &needs[0];
  assert!(as_bool(get(need, "blocking")));
  assert_eq!(as_str(get(need, "status")), "blocked");
}

#[test]
fn v0_5_1_wrong_id_arm_link_1_repair_effect_rejected() {
  // Invariant 132.
  let value = eval_file(&wrong_id_path()).unwrap();
  let entries = as_list(get_path(
    &value,
    &["computed-cross-object-repair-effect", "arm_link_1"],
  ));
  assert_eq!(entries.len(), 1);
  let entry = &entries[0];
  assert!(
    !as_bool(get(entry, "applied")),
    "wrong-id runner: applied must be false"
  );
  let applied_by = as_list(get(entry, "applied-by-repair-ids"));
  assert!(
    applied_by.is_empty(),
    "wrong-id runner: applied-by-repair-ids must be empty (the proposed wrong id contributes nothing)"
  );
}

#[test]
fn v0_5_1_wrong_id_arm_link_2_need_status_rejected() {
  // Invariant 133.
  let value = eval_file(&wrong_id_path()).unwrap();
  let needs = as_list(get_path(
    &value,
    &["computed-cross-object-needs-per-object", "arm_link_2"],
  ));
  assert_eq!(needs.len(), 1);
  let need = &needs[0];
  assert!(as_bool(get(need, "blocking")));
  assert_eq!(
    as_str(get(need, "status")),
    "promotion-rejected-invalid-repair-id",
    "wrong-id runner: arm_link_2's Need status must distinguish rejected promotion from no promotion attempted"
  );
}

#[test]
fn v0_5_1_wrong_id_chain_step_not_marked_promoted() {
  // Invariant 134.
  let value = eval_file(&wrong_id_path()).unwrap();
  let chain3 = as_list(get_path(
    &value,
    &["transitive-chain-per-object", "arm_link_3"],
  ));
  assert_eq!(chain3.len(), 2);
  for (i, step) in chain3.iter().enumerate() {
    assert!(
      !as_bool(get(step, "upstream-promoted")),
      "wrong-id runner: chain step at index {} must NOT be marked promoted (rejected promotion does not mark chain)",
      i
    );
  }
}

#[test]
fn v0_5_1_allowed_delta_only_validated_vs_without_promotion() {
  // Invariant 135 — load-bearing.
  let validated = eval_file(&validated_path()).unwrap();
  let without_p = eval_file(&without_promotion_path()).unwrap();

  // v0.2 trace: full byte-for-byte equality.
  assert_eq!(
    get(&validated, "computed-held-per-object").to_json(),
    get(&without_p, "computed-held-per-object").to_json(),
  );
  assert_eq!(
    get(&validated, "computed-repair-per-object").to_json(),
    get(&without_p, "computed-repair-per-object").to_json(),
  );
  assert_eq!(
    get(&validated, "computed-turns").to_json(),
    get(&without_p, "computed-turns").to_json(),
  );

  // Need entries: only blocking + status may differ.
  assert_per_object_entries_match_except(
    get(&validated, "computed-cross-object-needs-per-object"),
    get(&without_p, "computed-cross-object-needs-per-object"),
    &["blocking", "status"],
    "computed-cross-object-needs-per-object",
  );

  // Repair-effect entries: only applied + applied-by-repair-ids may differ.
  assert_per_object_entries_match_except(
    get(&validated, "computed-cross-object-repair-effect"),
    get(&without_p, "computed-cross-object-repair-effect"),
    &["applied", "applied-by-repair-ids"],
    "computed-cross-object-repair-effect",
  );

  // Chain steps: only upstream-promoted may differ.
  assert_per_object_entries_match_except(
    get(&validated, "transitive-chain-per-object"),
    get(&without_p, "transitive-chain-per-object"),
    &["upstream-promoted"],
    "transitive-chain-per-object",
  );
}

#[test]
fn v0_5_1_allowed_delta_only_validated_vs_wrong_id() {
  // Invariant 136 — load-bearing.
  let validated = eval_file(&validated_path()).unwrap();
  let wrong_id = eval_file(&wrong_id_path()).unwrap();

  assert_eq!(
    get(&validated, "computed-held-per-object").to_json(),
    get(&wrong_id, "computed-held-per-object").to_json(),
  );
  assert_eq!(
    get(&validated, "computed-repair-per-object").to_json(),
    get(&wrong_id, "computed-repair-per-object").to_json(),
  );
  assert_eq!(
    get(&validated, "computed-turns").to_json(),
    get(&wrong_id, "computed-turns").to_json(),
  );

  assert_per_object_entries_match_except(
    get(&validated, "computed-cross-object-needs-per-object"),
    get(&wrong_id, "computed-cross-object-needs-per-object"),
    &["blocking", "status"],
    "computed-cross-object-needs-per-object",
  );
  assert_per_object_entries_match_except(
    get(&validated, "computed-cross-object-repair-effect"),
    get(&wrong_id, "computed-cross-object-repair-effect"),
    &["applied", "applied-by-repair-ids"],
    "computed-cross-object-repair-effect",
  );
  assert_per_object_entries_match_except(
    get(&validated, "transitive-chain-per-object"),
    get(&wrong_id, "transitive-chain-per-object"),
    &["upstream-promoted"],
    "transitive-chain-per-object",
  );
}

#[test]
fn v0_5_1_status_enum_subset_across_runners() {
  // Invariant 137.
  let validated = eval_file(&validated_path()).unwrap();
  let wrong_id = eval_file(&wrong_id_path()).unwrap();
  let without_p = eval_file(&without_promotion_path()).unwrap();

  let allowed: BTreeSet<&str> = [
    "blocked",
    "reopened-by-upstream-promotion",
    "promotion-rejected-invalid-repair-id",
    "non-blocking-no-held",
  ]
  .iter()
  .copied()
  .collect();

  for (label, value) in [
    ("validated", &validated),
    ("wrong-id", &wrong_id),
    ("without-promotion", &without_p),
  ] {
    let needs_map = as_attrs(get(value, "computed-cross-object-needs-per-object"));
    for (object_id, needs_value) in needs_map {
      for need in as_list(needs_value) {
        let s = as_str(get(need, "status"));
        assert!(
          allowed.contains(s),
          "[{}] Need status `{}` for {} not in allowed v0.5.1 enum",
          label,
          s,
          object_id
        );
      }
    }
  }
}

#[test]
fn v0_5_1_owner_law_file_unchanged_seven_rule_surface() {
  // Invariant 138.
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
        "v0_owner_law.px must still expose `{}` after v0.5.1; available: {:?}",
        rule,
        attrs.keys().collect::<Vec<_>>()
      )
    });
    assert!(
      matches!(entry, Value::Lambda { .. }),
      "v0_owner_law.px:`{}` must still be a Lambda after v0.5.1, got {:?}",
      rule,
      entry
    );
  }

  let allowed: BTreeSet<&str> = required_rules.iter().copied().collect();
  for key in attrs.keys() {
    assert!(
      allowed.contains(key.as_str()),
      "v0.5.1 must NOT add new owner-law rules; v0_owner_law.px carries unexpected key `{}`",
      key
    );
  }
}
