//! v0.9.1 wrong-repair-id at N=3. Near-clone of the v0.9
//! validated runner with `promoted-repairs` overridden to
//! declare a wrong repair-id for arm_link_1. The v0.5.1 id-
//! match predicate (carried verbatim through v0.7 / v0.9)
//! rejects it; arm_link_1 stays unvalidated; the downstream
//! Need at arm_link_3 → arm_link_1 carries
//! `status = "promotion-rejected-invalid-repair-id"`; every
//! chain step visiting arm_link_1 has
//! `upstream-promoted = false`; the cycle-Held overlay is
//! byte-identical to BOTH the v0.9 validated and v0.9
//! without-promotion runners.
//!
//! Load-bearing claims:
//!   - id validation (v0.5.1) carries forward unchanged
//!     through v0.7's promotion-aware shape and v0.8's N=3
//!     walker into v0.9's synthesis runner — wrong id at
//!     N=3 still rejects.
//!   - `promotion-rejected-invalid-repair-id` is reachable
//!     on a 3-edge directed cycle, not just on the v0.5.1
//!     2-object scaffold.
//!   - cycle-Held overlay survives wrong-id promotion at
//!     N=3 byte-for-byte identical to without-promotion AND
//!     to v0.9 validated promotion (cycle structure is
//!     independent of promotion outcome at depth-N).
//!
//! Truth owner:        project-wiki/maps/minimal-tesseract-v0-map.md
//!                     §"v0.9.1 design decision — wrong-repair-id at
//!                       N=3"
//! Active scope:       project-wiki/maps/active-domain-constitution.md Art. 8
//!
//! What this test asserts (12 invariants, indices continued
//! from the v0.9 test's 198):
//! 199. world-set / relations identical to v0.8/v0.9 (3
//!      objects, 3-edge cycle); v0-9 / v0-9-1 / promotion-
//!      aware / id-validation-aware / cycle-aware markers
//!      true; promoted-repairs = { arm_link_1 = [
//!      "wrong.repair.id.does.not.match" ]; }.
//! 200. **load-bearing — walker terminates at N=3 under
//!      wrong-id promotion**: every chain length EXACTLY 3.
//! 201. **load-bearing — wrong id rejected at N=3**:
//!      arm_link_3's Need toward arm_link_1 has
//!      `blocking = true` and `status =
//!      "promotion-rejected-invalid-repair-id"` (NOT
//!      `reopened-by-upstream-promotion`).
//! 202. arm_link_1's Need toward arm_link_2 stays blocked
//!      and arm_link_2's Need toward arm_link_3 stays
//!      `non-blocking-no-held` (unchanged from v0.9).
//! 203. arm_link_1's repair-effect entry: applied = false,
//!      applied-by-repair-ids = [ ] (carries the wrong-id
//!      rejection into the downstream-effect surface).
//! 204. **load-bearing — every chain step visiting
//!      arm_link_1 has `upstream-promoted = false`** (id
//!      validation rejects across all chain start points
//!      and depths).
//! 205. **load-bearing — cycle-Held overlay byte-for-byte
//!      identical with wrong-id vs v0.9 validated** at N=3
//!      (cycle structure is independent of promotion
//!      outcome).
//! 206. **load-bearing — cycle-Held overlay byte-for-byte
//!      identical with wrong-id vs v0.9 without-promotion**
//!      at N=3 (transitivity through validated /
//!      without-promotion).
//! 207. v0.2 trace byte-for-byte identical across v0.9.1 vs
//!      v0.9 validated.
//! 208. allowed-delta-only diff (wrong-id vs v0.9
//!      validated): only { Need.blocking, Need.status,
//!      repair-effect.applied,
//!      repair-effect.applied-by-repair-ids,
//!      chain-step.upstream-promoted } may differ.
//!      cycle-detected and cycle-loop-target NOT in the
//!      allowed delta set.
//! 209. v0_owner_law.px STILL exposes exactly the same 7
//!      Lambda rules.
//! 210. **load-bearing — strict diff between wrong-id
//!      and without-promotion**: at N=3 the only
//!      difference between rejected promotion and absent
//!      promotion is `Need.status` (rejected says
//!      `promotion-rejected-invalid-repair-id`, absent
//!      says `blocked`). repair-effect entries are
//!      byte-equal; transitive-chain entries are byte-
//!      equal (including upstream-promoted, which is
//!      false in BOTH); cycle-detected and
//!      cycle-loop-target are non-delta. This pins
//!      "rejected and absent agree on every surface
//!      except the rejection reason itself."

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

fn wrong_id_path() -> PathBuf {
  fixture_root().join("v0_9_1_run_wrong_repair_id_with_3_cycle.px")
}

fn v0_9_validated_path() -> PathBuf {
  fixture_root().join("v0_9_run_promotion_with_3_cycle.px")
}

fn v0_9_without_promotion_path() -> PathBuf {
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
fn v0_9_1_world_set_relations_promoted_repairs_marker() {
  // Invariant 199.
  let value = eval_file(&wrong_id_path()).expect("v0.9.1 wrong-id harness must evaluate");

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
  assert!(as_bool(get(&value, "v0-9-1")));
  assert!(as_bool(get(&value, "promotion-aware")));
  assert!(as_bool(get(&value, "id-validation-aware")));
  assert!(as_bool(get(&value, "cycle-aware")));

  let promoted = as_attrs(get(&value, "promoted-repairs"));
  assert_eq!(promoted.len(), 1);
  let arm_link_1_promoted = as_list(promoted.get("arm_link_1").unwrap());
  assert_eq!(arm_link_1_promoted.len(), 1);
  assert_eq!(
    as_str(&arm_link_1_promoted[0]),
    "wrong.repair.id.does.not.match"
  );
}

#[test]
fn v0_9_1_walker_terminates_at_n_3_under_wrong_id() {
  // Invariant 200 — load-bearing.
  let value = eval_file(&wrong_id_path()).unwrap();
  let chain_map = as_attrs(get(&value, "transitive-chain-per-object"));
  assert_eq!(chain_map.len(), 3);
  for (object_id, chain_value) in chain_map {
    let chain = as_list(chain_value);
    assert_eq!(
      chain.len(),
      3,
      "v0.9.1 walker must terminate at length EXACTLY 3 for {} under wrong-id promotion",
      object_id
    );
  }
}

#[test]
fn v0_9_1_arm_link_3_need_promotion_rejected() {
  // Invariant 201 — load-bearing.
  // Wrong id rejected: arm_link_3 → arm_link_1 stays
  // blocking=true with status=promotion-rejected-invalid-
  // repair-id (NOT reopened-by-upstream-promotion as v0.9
  // validated).
  let value = eval_file(&wrong_id_path()).unwrap();
  let needs = as_list(get_path(
    &value,
    &["computed-cross-object-needs-per-object", "arm_link_3"],
  ));
  assert_eq!(needs.len(), 1);
  let need = &needs[0];
  assert_eq!(as_str(get(need, "to")), "arm_link_1");
  assert!(
    as_bool(get(need, "blocking")),
    "arm_link_3 Need MUST stay blocking under wrong id (no validated promotion)"
  );
  assert_eq!(
    as_str(get(need, "status")),
    "promotion-rejected-invalid-repair-id",
    "arm_link_3 Need surfaces promotion-rejected-invalid-repair-id (the v0.5.1 rejection arm) at N=3"
  );
}

#[test]
fn v0_9_1_other_needs_carry_forward_unchanged_from_v0_9() {
  // Invariant 202.
  // arm_link_1 Need toward arm_link_2: stays blocked
  // (arm_link_2 was never input as promoted, so wrong-id
  // does not affect it — same as v0.9 validated).
  // arm_link_2 Need toward arm_link_3: stays
  // non-blocking-no-held (arm_link_3 has no v0.2 Held —
  // entirely independent of input promotion id).
  let value = eval_file(&wrong_id_path()).unwrap();

  let need_1 = &as_list(get_path(
    &value,
    &["computed-cross-object-needs-per-object", "arm_link_1"],
  ))[0];
  assert_eq!(as_str(get(need_1, "to")), "arm_link_2");
  assert!(as_bool(get(need_1, "blocking")));
  assert_eq!(as_str(get(need_1, "status")), "blocked");

  let need_2 = &as_list(get_path(
    &value,
    &["computed-cross-object-needs-per-object", "arm_link_2"],
  ))[0];
  assert_eq!(as_str(get(need_2, "to")), "arm_link_3");
  assert!(!as_bool(get(need_2, "blocking")));
  assert_eq!(as_str(get(need_2, "status")), "non-blocking-no-held");
}

#[test]
fn v0_9_1_arm_link_1_repair_effect_not_applied() {
  // Invariant 203.
  // arm_link_1's repair-effect entry exists (because
  // arm_link_1 has a v0.2 Repair) but applied=false and
  // applied-by-repair-ids=[] under wrong id. This is
  // distinct from v0.9 validated where applied=true and
  // applied-by-repair-ids=["repair.role-binding"].
  let value = eval_file(&wrong_id_path()).unwrap();
  let entries = as_list(get_path(
    &value,
    &["computed-cross-object-repair-effect", "arm_link_1"],
  ));
  assert_eq!(entries.len(), 1);
  let entry = &entries[0];
  assert_eq!(as_str(get(entry, "downstream-object")), "arm_link_3");
  assert_eq!(as_str(get(entry, "relation-kind")), "depends-on-frame");
  assert!(
    !as_bool(get(entry, "applied")),
    "arm_link_1's repair-effect.applied MUST be false under wrong id"
  );
  assert!(
    as_list(get(entry, "applied-by-repair-ids")).is_empty(),
    "arm_link_1's applied-by-repair-ids MUST be empty under wrong id"
  );
}

#[test]
fn v0_9_1_every_chain_step_visiting_arm_link_1_has_upstream_promoted_false() {
  // Invariant 204 — load-bearing.
  // Across all 3 chains, every step that visits arm_link_1
  // must have upstream-promoted=false. This is the
  // wrong-id rejection signature at depth-N.
  let value = eval_file(&wrong_id_path()).unwrap();
  let chain_map = as_attrs(get(&value, "transitive-chain-per-object"));

  let mut visits = 0usize;
  for (object_id, chain_value) in chain_map {
    let chain = as_list(chain_value);
    for (i, step) in chain.iter().enumerate() {
      if as_str(get(step, "object-id")) == "arm_link_1" {
        visits += 1;
        assert!(
          !as_bool(get(step, "upstream-promoted")),
          "chain[{}].step[{}] visits arm_link_1 — upstream-promoted MUST be false under wrong id",
          object_id,
          i
        );
      }
    }
  }
  // N=3 directed cycle: each of the 3 chains visits
  // every object exactly once → arm_link_1 must be
  // visited EXACTLY 3 times across the chain map.
  assert_eq!(
    visits, 3,
    "expected exactly 3 chain steps visiting arm_link_1 across the 3 chains (N=3 directed cycle: each object visited once per chain), got {}",
    visits
  );
}

#[test]
fn v0_9_1_cycle_helds_byte_identical_with_v0_9_validated() {
  // Invariant 205 — load-bearing.
  // cycle-Held overlay survives wrong-id promotion at N=3
  // byte-for-byte identical to v0.9 validated promotion.
  let wrong_id = eval_file(&wrong_id_path()).unwrap();
  let validated = eval_file(&v0_9_validated_path()).unwrap();

  assert_eq!(
    get(&wrong_id, "computed-cycle-helds-per-object").to_json(),
    get(&validated, "computed-cycle-helds-per-object").to_json(),
    "computed-cycle-helds-per-object must be byte-for-byte identical between v0.9.1 wrong-id and v0.9 validated at N=3 (cycle structure is independent of promotion outcome)"
  );
}

#[test]
fn v0_9_1_cycle_helds_byte_identical_with_v0_9_without_promotion() {
  // Invariant 206 — load-bearing.
  // Transitivity: cycle-Held overlay byte-for-byte
  // identical between v0.9.1 wrong-id and v0.9 without-
  // promotion at N=3.
  let wrong_id = eval_file(&wrong_id_path()).unwrap();
  let without_p = eval_file(&v0_9_without_promotion_path()).unwrap();

  assert_eq!(
    get(&wrong_id, "computed-cycle-helds-per-object").to_json(),
    get(&without_p, "computed-cycle-helds-per-object").to_json(),
    "computed-cycle-helds-per-object must be byte-for-byte identical between v0.9.1 wrong-id and v0.9 without-promotion at N=3"
  );

  // Spot-check the overlay shape: 3 entries, all
  // promoted=false, held-kind=dependency-cycle.
  let cycle_helds = as_attrs(get(&wrong_id, "computed-cycle-helds-per-object"));
  assert_eq!(cycle_helds.len(), 3);
  for object_id in ["arm_link_1", "arm_link_2", "arm_link_3"] {
    let entry = cycle_helds
      .get(object_id)
      .unwrap_or_else(|| panic!("missing cycle-Held for {}", object_id));
    assert_eq!(as_str(get(entry, "held-kind")), "dependency-cycle");
    assert!(!as_bool(get(entry, "promoted")));
  }
}

#[test]
fn v0_9_1_v0_2_trace_unchanged_vs_v0_9_validated() {
  // Invariant 207.
  let wrong_id = eval_file(&wrong_id_path()).unwrap();
  let validated = eval_file(&v0_9_validated_path()).unwrap();
  for surface in [
    "computed-held-per-object",
    "computed-repair-per-object",
    "computed-turns",
  ] {
    assert_eq!(
      get(&wrong_id, surface).to_json(),
      get(&validated, surface).to_json(),
      "v0.2 trace surface `{}` must be byte-for-byte identical between v0.9.1 wrong-id and v0.9 validated",
      surface
    );
  }
}

#[test]
fn v0_9_1_allowed_delta_only_diff_wrong_id_vs_validated() {
  // Invariant 208.
  let wrong_id = eval_file(&wrong_id_path()).unwrap();
  let validated = eval_file(&v0_9_validated_path()).unwrap();

  assert_per_object_entries_match_except(
    get(&wrong_id, "computed-cross-object-needs-per-object"),
    get(&validated, "computed-cross-object-needs-per-object"),
    &["blocking", "status"],
    "computed-cross-object-needs-per-object",
  );
  assert_per_object_entries_match_except(
    get(&wrong_id, "computed-cross-object-repair-effect"),
    get(&validated, "computed-cross-object-repair-effect"),
    &["applied", "applied-by-repair-ids"],
    "computed-cross-object-repair-effect",
  );
  // Chain steps: only upstream-promoted may differ.
  // cycle-detected and cycle-loop-target are NOT in the
  // allowed delta set — cycle structure must agree byte-
  // for-byte across runners at N=3 even when promotion
  // outcome flips.
  assert_per_object_entries_match_except(
    get(&wrong_id, "transitive-chain-per-object"),
    get(&validated, "transitive-chain-per-object"),
    &["upstream-promoted"],
    "transitive-chain-per-object",
  );
}

#[test]
fn v0_9_1_owner_law_file_unchanged_seven_rule_surface() {
  // Invariant 209.
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
        "v0_owner_law.px must still expose `{}` after v0.9.1; available: {:?}",
        rule,
        attrs.keys().collect::<Vec<_>>()
      )
    });
    assert!(
      matches!(entry, Value::Lambda { .. }),
      "v0_owner_law.px:`{}` must still be a Lambda after v0.9.1, got {:?}",
      rule,
      entry
    );
  }

  let allowed: BTreeSet<&str> = required_rules.iter().copied().collect();
  let actual: BTreeSet<&str> = attrs.keys().map(|s| s.as_str()).collect();
  assert_eq!(
    actual, allowed,
    "v0_owner_law.px must still expose exactly the 7 Lambda rules after v0.9.1; extra/missing keys would indicate a fork"
  );
}

#[test]
fn v0_9_1_strict_diff_wrong_id_vs_without_promotion() {
  // Invariant 210 — load-bearing.
  // At N=3 the only difference between rejected
  // promotion (v0.9.1) and absent promotion (v0.9
  // without-promotion) is `Need.status` (rejected
  // says `promotion-rejected-invalid-repair-id`,
  // absent says `blocked`). Every other surface
  // must be byte-equal:
  //   - Need.blocking is true on both (the rejection
  //     does NOT unblock; the absence already kept it
  //     blocked).
  //   - repair-effect.applied / applied-by-repair-ids
  //     are both false / [] on both (no validated
  //     promotion in either case).
  //   - chain-step.upstream-promoted is false on every
  //     step in both runners.
  //   - cycle-detected / cycle-loop-target agree
  //     byte-for-byte (cycle structure is independent
  //     of promotion outcome at depth-N).
  //
  // This pins "rejected and absent agree on every
  // surface except the rejection reason itself" —
  // the strongest form of the equivalence between
  // the two no-validated-promotion paths.
  let wrong_id = eval_file(&wrong_id_path()).unwrap();
  let without_p = eval_file(&v0_9_without_promotion_path()).unwrap();

  // Need: only `status` may differ. `blocking` must
  // agree (both true on the rejected/absent edge).
  assert_per_object_entries_match_except(
    get(&wrong_id, "computed-cross-object-needs-per-object"),
    get(&without_p, "computed-cross-object-needs-per-object"),
    &["status"],
    "computed-cross-object-needs-per-object (wrong-id vs without-promotion)",
  );

  // repair-effect: byte-equal (no field may differ).
  assert_per_object_entries_match_except(
    get(&wrong_id, "computed-cross-object-repair-effect"),
    get(&without_p, "computed-cross-object-repair-effect"),
    &[],
    "computed-cross-object-repair-effect (wrong-id vs without-promotion)",
  );

  // transitive-chain: byte-equal (no field may
  // differ — including upstream-promoted, which is
  // false in both runners).
  assert_per_object_entries_match_except(
    get(&wrong_id, "transitive-chain-per-object"),
    get(&without_p, "transitive-chain-per-object"),
    &[],
    "transitive-chain-per-object (wrong-id vs without-promotion)",
  );

  // Spot-check the actual status values on the
  // arm_link_3 → arm_link_1 edge to make the
  // rejection-reason delta explicit in the test.
  let rejected_status = as_str(get(
    &as_list(get_path(
      &wrong_id,
      &["computed-cross-object-needs-per-object", "arm_link_3"],
    ))[0],
    "status",
  ));
  let absent_status = as_str(get(
    &as_list(get_path(
      &without_p,
      &["computed-cross-object-needs-per-object", "arm_link_3"],
    ))[0],
    "status",
  ));
  assert_eq!(rejected_status, "promotion-rejected-invalid-repair-id");
  assert_eq!(absent_status, "blocked");
}
