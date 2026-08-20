//! v0.9 promotion + 3-cycle synthesis. Combines v0.7's
//! validated promotion + cycle-Held non-resolution claim
//! with v0.8's N=3 cycle generality. Same 3-edge directed
//! cycle as v0.8 (`arm_link_1 → arm_link_2 → arm_link_3 →
//! arm_link_1` with kinds [depends-on-frame, mounted-on,
//! depends-on-frame]) but with v0.5.1-style validated
//! promoted-repairs declaring arm_link_1's actual
//! RepairCandidate id.
//!
//! Load-bearing claims (extending v0.7's negative
//! invariant to N=3):
//!   - Walker terminates at N=3 under promotion (chain
//!     length stays 3).
//!   - Promotion does NOT transitively unblock — only the
//!     immediate downstream Need (arm_link_3 → arm_link_1)
//!     reopens; arm_link_1's Need toward arm_link_2 stays
//!     blocked.
//!   - cycle-Held overlay survives validated promotion at
//!     N=3 (3 entries, all `promoted = false`).
//!   - cycle-detected and upstream-promoted are
//!     INDEPENDENT per step at depth-N — arm_link_2's
//!     chain has the promoted upstream at depth 1 and the
//!     cycle closure at depth 2 (different positions).
//!   - cycle-Held overlay byte-for-byte identical between
//!     with-promotion and without-promotion runners at
//!     N=3.
//!
//! Truth owner:        project-wiki/maps/minimal-tesseract-v0-map.md
//!                     §"v0.9 design decision — promotion + 3-cycle
//!                       synthesis"
//! Active scope:       project-wiki/maps/active-domain-constitution.md Art. 8
//!
//! What this test asserts (15 invariants, indices continued
//! from the v0.8 test's 183):
//! 184. world-set / relations identical to v0.8 (3 objects,
//!      3-edge cycle); v0-9 / promotion-aware /
//!      id-validation-aware / cycle-aware markers true;
//!      promoted-repairs = { arm_link_1 = [
//!      "repair.role-binding" ]; }.
//! 185. **load-bearing — walker terminates at N=3 under
//!      promotion**: every chain length EXACTLY 3.
//! 186. arm_link_3's Need toward arm_link_1 has
//!      `blocking = false` and
//!      `status = "reopened-by-upstream-promotion"`.
//! 187. arm_link_1's Need toward arm_link_2 has
//!      `blocking = true` and `status = "blocked"`.
//! 188. arm_link_2's Need toward arm_link_3 has
//!      `blocking = false` and
//!      `status = "non-blocking-no-held"` (arm_link_3 has
//!      no v0.2 Held — new status state at N=3).
//! 189. **load-bearing — promotion does NOT transitively
//!      unblock**: arm_link_1's Need stays blocked even
//!      though arm_link_3's Need was reopened.
//! 190. arm_link_1's repair-effect entry: downstream =
//!      arm_link_3, applied = true, applied-by-repair-ids
//!      = [ "repair.role-binding" ], relation-kind =
//!      depends-on-frame.
//! 191. arm_link_2's repair-effect entry: downstream =
//!      arm_link_1, applied = false,
//!      applied-by-repair-ids = [ ].
//! 192. arm_link_3's repair-effect entry list is empty
//!      (no v0.2 Repair for arm_link_3).
//! 193. **load-bearing — cycle-Held overlay survives
//!      validated promotion at N=3**: 3 entries,
//!      held-kind = "dependency-cycle", promoted = false,
//!      non-null cycle-loop-target.
//! 194. **load-bearing — cycle-detected and upstream-
//!      promoted independent at depth-N**: arm_link_2
//!      chain has step 1 with upstream-promoted=true,
//!      cycle-detected=false (visiting promoted
//!      arm_link_1 mid-chain); step 2 with
//!      cycle-detected=true, upstream-promoted=false
//!      (closure at arm_link_2). The two facts occupy
//!      different depths.
//! 195. **load-bearing — cycle-Held overlay byte-for-byte
//!      identical with vs without promotion** at N=3
//!      (scaling v0.7 invariant 161 to N=3).
//! 196. v0.2 trace byte-for-byte identical across both
//!      runners.
//! 197. allowed-delta-only diff (validated vs without-
//!      promotion): only { Need.blocking, Need.status,
//!      repair-effect.applied,
//!      repair-effect.applied-by-repair-ids,
//!      chain-step.upstream-promoted } may differ.
//!      cycle-detected and cycle-loop-target NOT in the
//!      allowed delta set.
//! 198. v0_owner_law.px STILL exposes exactly the same 7
//!      Lambda rules.

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
  fixture_root().join("v0_9_run_promotion_with_3_cycle.px")
}

fn without_promotion_path() -> PathBuf {
  fixture_root().join("v0_9_run_without_promotion_with_3_cycle.px")
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

/// Allowed-delta-only diff helper: every key NOT in
/// `delta_keys` must be byte-for-byte identical via to_json.
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
fn v0_9_world_set_relations_promoted_repairs_marker() {
  // Invariant 184.
  let value = eval_file(&validated_path()).expect("v0.9 validated harness must evaluate");

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

  assert!(as_bool(get(&value, "v0-9")));
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
fn v0_9_walker_terminates_at_n_3_under_promotion() {
  // Invariant 185 — load-bearing.
  let value = eval_file(&validated_path()).unwrap();
  let chain_map = as_attrs(get(&value, "transitive-chain-per-object"));
  assert_eq!(chain_map.len(), 3);
  for (object_id, chain_value) in chain_map {
    let chain = as_list(chain_value);
    assert_eq!(
      chain.len(),
      3,
      "v0.9 walker must terminate at length EXACTLY 3 for {} under promotion",
      object_id
    );
  }
}

#[test]
fn v0_9_arm_link_3_need_reopens_to_arm_link_1() {
  // Invariant 186.
  let value = eval_file(&validated_path()).unwrap();
  let needs = as_list(get_path(
    &value,
    &["computed-cross-object-needs-per-object", "arm_link_3"],
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
fn v0_9_arm_link_1_need_stays_blocked_to_arm_link_2() {
  // Invariant 187.
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
    "arm_link_1 Need stays blocked: arm_link_2 not promoted"
  );
  assert_eq!(as_str(get(need, "status")), "blocked");
}

#[test]
fn v0_9_arm_link_2_need_non_blocking_no_held() {
  // Invariant 188 — new status state surfaced at N=3.
  let value = eval_file(&validated_path()).unwrap();
  let needs = as_list(get_path(
    &value,
    &["computed-cross-object-needs-per-object", "arm_link_2"],
  ));
  assert_eq!(needs.len(), 1);
  let need = &needs[0];
  assert_eq!(as_str(get(need, "to")), "arm_link_3");
  assert!(
    !as_bool(get(need, "blocking")),
    "arm_link_2 Need is non-blocking: arm_link_3 has no v0.2 Held"
  );
  assert_eq!(
    as_str(get(need, "status")),
    "non-blocking-no-held",
    "arm_link_2's status surfaces non-blocking-no-held at N=3 (new state vs v0.7's 2-cycle)"
  );
}

#[test]
fn v0_9_promotion_does_not_transitively_unblock_at_n_3() {
  // Invariant 189 — load-bearing.
  // arm_link_3 Need reopens (immediate downstream of
  // promoted arm_link_1) AND arm_link_1 Need stays
  // blocked (its upstream arm_link_2 was not promoted).
  // Both invariants checked together to encode the
  // "promotion is per-edge, not per-cycle" claim.
  let value = eval_file(&validated_path()).unwrap();

  let need_3 = &as_list(get_path(
    &value,
    &["computed-cross-object-needs-per-object", "arm_link_3"],
  ))[0];
  assert!(!as_bool(get(need_3, "blocking")));
  assert_eq!(
    as_str(get(need_3, "status")),
    "reopened-by-upstream-promotion",
    "arm_link_3 Need DOES reopen (immediate downstream of promoted arm_link_1)"
  );

  let need_1 = &as_list(get_path(
    &value,
    &["computed-cross-object-needs-per-object", "arm_link_1"],
  ))[0];
  assert!(
    as_bool(get(need_1, "blocking")),
    "arm_link_1 Need does NOT auto-unblock — promotion is per-edge, not transitive across the cycle"
  );
  assert_eq!(as_str(get(need_1, "status")), "blocked");
}

#[test]
fn v0_9_arm_link_1_repair_effect_applied_with_real_id() {
  // Invariant 190.
  let value = eval_file(&validated_path()).unwrap();
  let entries = as_list(get_path(
    &value,
    &["computed-cross-object-repair-effect", "arm_link_1"],
  ));
  assert_eq!(entries.len(), 1);
  let entry = &entries[0];
  assert_eq!(as_str(get(entry, "downstream-object")), "arm_link_3");
  assert_eq!(as_str(get(entry, "relation-kind")), "depends-on-frame");
  assert!(as_bool(get(entry, "applied")));
  let applied_by = as_list(get(entry, "applied-by-repair-ids"));
  assert_eq!(applied_by.len(), 1);
  assert_eq!(as_str(&applied_by[0]), "repair.role-binding");
}

#[test]
fn v0_9_arm_link_2_repair_effect_not_applied() {
  // Invariant 191.
  let value = eval_file(&validated_path()).unwrap();
  let entries = as_list(get_path(
    &value,
    &["computed-cross-object-repair-effect", "arm_link_2"],
  ));
  assert_eq!(entries.len(), 1);
  let entry = &entries[0];
  assert_eq!(as_str(get(entry, "downstream-object")), "arm_link_1");
  assert!(!as_bool(get(entry, "applied")));
  assert!(as_list(get(entry, "applied-by-repair-ids")).is_empty());
}

#[test]
fn v0_9_arm_link_3_repair_effect_empty() {
  // Invariant 192.
  let value = eval_file(&validated_path()).unwrap();
  let entries = as_list(get_path(
    &value,
    &["computed-cross-object-repair-effect", "arm_link_3"],
  ));
  assert!(
    entries.is_empty(),
    "arm_link_3 has no v0.2 Repair entry, so its repair-effect entry list is empty even though arm_link_2 → arm_link_3 exists"
  );
}

#[test]
fn v0_9_cycle_helds_overlay_survives_validated_promotion_at_n_3() {
  // Invariant 193 — load-bearing.
  let value = eval_file(&validated_path()).unwrap();
  let cycle_helds = as_attrs(get(&value, "computed-cycle-helds-per-object"));
  assert_eq!(cycle_helds.len(), 3);
  for object_id in ["arm_link_1", "arm_link_2", "arm_link_3"] {
    let entry = cycle_helds.get(object_id).unwrap_or_else(|| {
      panic!(
        "validated runner: cycle-Held overlay must STILL contain `{}` at N=3",
        object_id
      )
    });
    assert_eq!(as_str(get(entry, "held-kind")), "dependency-cycle");
    assert!(
      !as_bool(get(entry, "promoted")),
      "cycle-Held.promoted MUST stay false even when arm_link_1's RepairCandidate id validates (load-bearing N=3 carryover from v0.7)"
    );
    assert_eq!(as_str(get(entry, "applies-at")), object_id);
    assert_eq!(as_str(get(entry, "cycle-loop-target")), object_id);
  }
}

#[test]
fn v0_9_cycle_detected_and_upstream_promoted_independent_at_depth_n() {
  // Invariant 194 — load-bearing.
  // arm_link_2's chain has the promoted upstream
  // (arm_link_1) at depth 1 and the cycle closure at
  // depth 2 — different positions, proving per-step
  // independence at N=3.
  let value = eval_file(&validated_path()).unwrap();
  let chain = as_list(get_path(
    &value,
    &["transitive-chain-per-object", "arm_link_2"],
  ));
  assert_eq!(chain.len(), 3);

  // Step 0 (depth 1): arm_link_3, mounted-on (no promotion, no cycle).
  let step0 = &chain[0];
  assert_eq!(as_str(get(step0, "object-id")), "arm_link_3");
  assert!(!as_bool(get(step0, "cycle-detected")));
  assert!(
    !as_bool(get(step0, "upstream-promoted")),
    "step 0 visits arm_link_3 which was not promoted"
  );

  // Step 1 (depth 2): arm_link_1, depends-on-frame
  // — visiting promoted upstream MID-chain.
  let step1 = &chain[1];
  assert_eq!(as_str(get(step1, "object-id")), "arm_link_1");
  assert!(
    !as_bool(get(step1, "cycle-detected")),
    "step 1 is mid-chain, not the cycle closure"
  );
  assert!(
    as_bool(get(step1, "upstream-promoted")),
    "step 1 visits PROMOTED arm_link_1 — upstream-promoted=true mid-chain"
  );

  // Step 2 (depth 3): arm_link_2, depends-on-frame
  // — cycle closure at NOT-promoted target.
  let step2 = &chain[2];
  assert_eq!(as_str(get(step2, "object-id")), "arm_link_2");
  assert!(
    as_bool(get(step2, "cycle-detected")),
    "step 2 is the cycle closure"
  );
  assert_eq!(as_str(get(step2, "cycle-loop-target")), "arm_link_2");
  assert!(
    !as_bool(get(step2, "upstream-promoted")),
    "step 2 closure target arm_link_2 was NOT promoted — cycle and promoted occupy DIFFERENT depths"
  );
}

#[test]
fn v0_9_cycle_helds_byte_identical_with_vs_without_promotion_at_n_3() {
  // Invariant 195 — load-bearing.
  let validated = eval_file(&validated_path()).unwrap();
  let without_p = eval_file(&without_promotion_path()).unwrap();

  assert_eq!(
    get(&validated, "computed-cycle-helds-per-object").to_json(),
    get(&without_p, "computed-cycle-helds-per-object").to_json(),
    "computed-cycle-helds-per-object must be byte-for-byte identical with vs without promotion at N=3 (cycle-Held promotion-independent)"
  );
}

#[test]
fn v0_9_v0_2_trace_unchanged_across_runners() {
  // Invariant 196.
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
      "v0.2 trace surface `{}` must be byte-for-byte identical across v0.9 runners",
      surface
    );
  }
}

#[test]
fn v0_9_allowed_delta_only_diff_validated_vs_without() {
  // Invariant 197.
  let validated = eval_file(&validated_path()).unwrap();
  let without_p = eval_file(&without_promotion_path()).unwrap();

  assert_per_object_entries_match_except(
    get(&validated, "computed-cross-object-needs-per-object"),
    get(&without_p, "computed-cross-object-needs-per-object"),
    &["blocking", "status"],
    "computed-cross-object-needs-per-object",
  );
  assert_per_object_entries_match_except(
    get(&validated, "computed-cross-object-repair-effect"),
    get(&without_p, "computed-cross-object-repair-effect"),
    &["applied", "applied-by-repair-ids"],
    "computed-cross-object-repair-effect",
  );
  // Chain steps: only upstream-promoted may differ.
  // cycle-detected and cycle-loop-target are NOT in the
  // allowed delta set — cycle structure must agree byte-
  // for-byte across runners at N=3.
  assert_per_object_entries_match_except(
    get(&validated, "transitive-chain-per-object"),
    get(&without_p, "transitive-chain-per-object"),
    &["upstream-promoted"],
    "transitive-chain-per-object",
  );
}

#[test]
fn v0_9_owner_law_file_unchanged_seven_rule_surface() {
  // Invariant 198.
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
        "v0_owner_law.px must still expose `{}` after v0.9; available: {:?}",
        rule,
        attrs.keys().collect::<Vec<_>>()
      )
    });
    assert!(
      matches!(entry, Value::Lambda { .. }),
      "v0_owner_law.px:`{}` must still be a Lambda after v0.9, got {:?}",
      rule,
      entry
    );
  }

  let allowed: BTreeSet<&str> = required_rules.iter().copied().collect();
  for key in attrs.keys() {
    assert!(
      allowed.contains(key.as_str()),
      "v0.9 must NOT add new owner-law rules; v0_owner_law.px carries unexpected key `{}`",
      key
    );
  }
}
