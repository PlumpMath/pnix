//! v0.2 set-aware lens overlay slice — Option A-lite from
//! `project-wiki/maps/minimal-tesseract-v0-map.md`
//! §"v0.2 design decision".
//!
//! Lens attaches once to the world-set; per-attach affected-slice
//! contains every object id; LensCompare emits per-object
//! conflict rows; Held / Repair stay per-object as fixture-local
//! `*-per-object` maps (not a sanctioned plural HeldGraph /
//! NeedGraph carrier — that decision is deferred to v0.3).
//!
//! Truth owner:        project-wiki/maps/minimal-tesseract-v0-map.md
//! Active scope:       project-wiki/maps/active-domain-constitution.md Art. 8
//!
//! What this test asserts (12 invariants, indices continued from
//! the v0.1 test's 43):
//!  44. world-set has 2 SourceObjects with ids
//!      [arm_link_1, arm_link_2].
//!  45. owner-law-loaded == true; owner-law-source points at
//!      v0_owner_law.px; set-aware == true.
//!  46. computed-turns has 6 turns with ids 0..5.
//!  47. each attach turn (Turn 0..3) `affected-slice` contains
//!      BOTH arm_link_1 AND arm_link_2 — load-bearing v0.2
//!      invariant.
//!  48. each attach turn's `attach-route`/`changed-routes`
//!      mentions the world-set (string contains both object ids).
//!  49. computed-lens-compare.source-ids == [arm_link_1,
//!      arm_link_2] AND computed-lens-compare.conflict has a row
//!      per object with blocked-node matching that object's id.
//!  50. computed-held-per-object has exactly 2 keys; each value
//!      has held-kind structural-binding-conflict and blocked-node
//!      matching the key.
//!  51. computed-repair-per-object has exactly 2 keys; each value
//!      is a RepairCandidate with id repair.role-binding,
//!      promotion pending, and applies-at matching the key.
//!  52. computed-meta-circular-log-differential.after-turn-5
//!      .active-lenses contains all 4 lens ids; open-needs has 4
//!      entries; open-held has 1 entry (kind id deduplicated by
//!      the set-aware meta-log helper).
//!  53. without-owner-law: computed-turns == [],
//!      computed-held-per-object == {}, computed-repair-per-object
//!      == {}, computed-meta-circular-log-differential is the
//!      no-change record.
//!  54. v0_owner_law.px STILL exposes exactly the same 7 Lambda
//!      rules (v0.2 must not modify the owner-law surface).
//!  55. **REFINED contamination**: shared turn fields (affected-slice,
//!      attach-route, conflict[]) MAY mention both object ids
//!      (set-aware semantics); each per-object Held body must
//!      contain only its own object id; each per-object Repair
//!      body must contain only its own object id. v0.1's
//!      pure-contamination form ("each trace's JSON contains no
//!      other id") is split into "shared fields can carry both
//!      ids; disjoint per-object bodies must not".
//!
//! What this test deliberately does NOT do:
//!
//!   - It does NOT apply owner-law in Rust. Owner-law application
//!     happens inside `v0_2_run_set_aware.px`.
//!   - It does NOT introduce a new ontology record kind.
//!   - It does NOT add a CapabilityCard / NeedGraph / HeldGraph /
//!     BenchmarkGraph / RigorFloor registry.
//!   - It does NOT touch v0 / v0.1 files; v0.2 is additions-only.
//!   - It does NOT exercise cross-object Held / Need dependencies
//!     (v0.3 territory).

use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------
// Helpers — duplicated from the v0 / v0.1 tests because
// integration test files are compiled as separate crates and
// cannot share private helpers across files. Duplication is
// small and keeps the v0.2 test independently auditable.
// ---------------------------------------------------------------

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../../fixtures/minimal-tesseract-v0/v0_2_run_set_aware.px")
}

fn fixture_without_owner_law_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../../fixtures/minimal-tesseract-v0/v0_2_run_without_owner_law.px")
}

fn owner_law_file_path() -> PathBuf {
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
// Tests.
// ---------------------------------------------------------------

#[test]
fn v0_2_world_set_has_two_source_objects() {
  // Invariant 44 + 45.
  let value = eval_file(&fixture_path()).expect("v0.2 set-aware harness must evaluate");

  let world_set = as_list(get(&value, "world-set"));
  assert_eq!(
    world_set.len(),
    2,
    "v0.2 world-set must have exactly 2 SourceObjects; got {}",
    world_set.len()
  );
  let ids: Vec<&str> = world_set.iter().map(|o| as_str(get(o, "id"))).collect();
  assert_eq!(
    ids,
    vec!["arm_link_1", "arm_link_2"],
    "v0.2 world-set ids must be [arm_link_1, arm_link_2]"
  );

  // Markers.
  match get(&value, "owner-law-loaded") {
    Value::Bool(b) => assert!(
      *b,
      "v0.2 set-aware harness must report owner-law-loaded=true"
    ),
    other => panic!("owner-law-loaded must be a Bool, got {:?}", other),
  }
  assert_eq!(
    as_str(get(&value, "owner-law-source")),
    "fixtures/minimal-tesseract-v0/v0_owner_law.px",
    "v0.2 must reuse the v0 owner-law file by reference"
  );
  match get(&value, "set-aware") {
    Value::Bool(b) => assert!(*b, "v0.2 harness must report set-aware=true"),
    other => panic!("set-aware must be a Bool, got {:?}", other),
  }
}

#[test]
fn v0_2_six_turn_trajectory() {
  // Invariant 46.
  let value = eval_file(&fixture_path()).unwrap();
  let turns = as_list(get(&value, "computed-turns"));
  assert_eq!(turns.len(), 6, "v0.2 must produce 6 turns");
  let turn_ids: Vec<i64> = turns.iter().map(|t| as_int(get(t, "turn-id"))).collect();
  assert_eq!(
    turn_ids,
    vec![0, 1, 2, 3, 4, 5],
    "v0.2 turn-id sequence must be [0,1,2,3,4,5]"
  );
}

#[test]
fn v0_2_each_attach_turn_affects_both_objects() {
  // Invariant 47 — load-bearing v0.2 invariant. Each of the 4
  // attach turns (Turn 0..3) must reference BOTH arm_link_1 and
  // arm_link_2 in its affected-slice. The compare/repair turns
  // (Turn 4 / Turn 5) must too — set-aware semantics propagate
  // through every turn that touches the world-set.
  let value = eval_file(&fixture_path()).unwrap();
  let turns = as_list(get(&value, "computed-turns"));

  for (idx, turn) in turns.iter().enumerate() {
    let slice = list_of_strings(get(turn, "affected-slice"));

    // Turn 0 (lens.base) is the only attach turn whose
    // affected-slice uses the per-object-identity shape
    // (`<id>.identity` rather than bare `<id>`). Both objects'
    // identity entries must appear.
    let (needle_a, needle_b) = if idx == 0 {
      ("arm_link_1.identity", "arm_link_2.identity")
    } else {
      ("arm_link_1", "arm_link_2")
    };

    assert!(
      slice.iter().any(|s| *s == needle_a),
      "Turn {}.affected-slice must contain `{}` (set-aware: lens attaches to world-set); got {:?}",
      idx,
      needle_a,
      slice
    );
    assert!(
      slice.iter().any(|s| *s == needle_b),
      "Turn {}.affected-slice must contain `{}` (set-aware: lens attaches to world-set); got {:?}",
      idx,
      needle_b,
      slice
    );
  }
}

#[test]
fn v0_2_each_attach_route_mentions_world_set() {
  // Invariant 48. attach-route (carried in changed-routes for
  // attach turns; in changed-routes for compare/repair turns) is
  // a string that must mention the world-set when the turn is
  // set-aware. We assert each turn's first changed-route string
  // contains both object ids — that is the literal world-set
  // reference.
  let value = eval_file(&fixture_path()).unwrap();
  let turns = as_list(get(&value, "computed-turns"));

  for (idx, turn) in turns.iter().enumerate() {
    let routes = list_of_strings(get(turn, "changed-routes"));
    if routes.is_empty() {
      continue;
    }
    // Skip turn 5 (repair-candidate emission) — its
    // changed-routes is the generic "repair candidate emitted
    // (NOT auto-applied)" string with no world-set reference;
    // its set-awareness shows up in affected-slice (already
    // covered by test 47).
    if idx == 5 {
      continue;
    }
    let route0 = routes[0];
    assert!(
      route0.contains("arm_link_1") && route0.contains("arm_link_2"),
      "Turn {}.changed-routes[0] must mention both world-set object ids; got `{}`",
      idx,
      route0
    );
  }
}

#[test]
fn v0_2_lens_compare_per_object_conflict_rows() {
  // Invariant 49. computed-lens-compare.source-ids is plural;
  // conflict[] has one row per object with blocked-node matching
  // that object's id.
  let value = eval_file(&fixture_path()).unwrap();
  let lc = get(&value, "computed-lens-compare");

  let source_ids = list_of_strings(get(lc, "source-ids"));
  assert_eq!(
    source_ids,
    vec!["arm_link_1", "arm_link_2"],
    "computed-lens-compare.source-ids must == [arm_link_1, arm_link_2]"
  );

  let conflicts = as_list(get(lc, "conflict"));
  assert_eq!(
    conflicts.len(),
    2,
    "computed-lens-compare.conflict must have 2 rows (one per world-set object); got {}",
    conflicts.len()
  );

  let blocked: Vec<&str> = conflicts
    .iter()
    .map(|c| as_str(get(c, "blocked-node")))
    .collect();
  assert!(blocked.contains(&"arm_link_1"));
  assert!(blocked.contains(&"arm_link_2"));

  for c in conflicts {
    assert_eq!(
      as_str(get(c, "kind")),
      "structural-binding-conflict",
      "every conflict row must carry kind=structural-binding-conflict"
    );
    assert_eq!(
      as_str(get(c, "held-ref")),
      "held.structural-binding-conflict",
      "every conflict row must reference the structural-binding-conflict Held kind"
    );
  }
}

#[test]
fn v0_2_held_per_object_two_entries_with_matching_blocked_node() {
  // Invariant 50.
  let value = eval_file(&fixture_path()).unwrap();
  let held_map = as_attrs(get(&value, "computed-held-per-object"));

  let keys: Vec<&str> = held_map.keys().map(|s| s.as_str()).collect();
  assert_eq!(
    keys,
    vec!["arm_link_1", "arm_link_2"],
    "computed-held-per-object must have exactly the two world-set object ids as keys; got {:?}",
    keys
  );

  for object_id in ["arm_link_1", "arm_link_2"] {
    let held = held_map.get(object_id).unwrap();
    assert_eq!(
      as_str(get(held, "held-kind")),
      "structural-binding-conflict",
      "held-per-object[{}].held-kind must be structural-binding-conflict",
      object_id
    );
    assert_eq!(
      as_str(get(held, "blocked-node")),
      object_id,
      "held-per-object[{}].blocked-node must equal the same object id (parametric reuse)",
      object_id
    );
  }
}

#[test]
fn v0_2_repair_per_object_two_entries_pending_with_matching_applies_at() {
  // Invariant 51.
  let value = eval_file(&fixture_path()).unwrap();
  let repair_map = as_attrs(get(&value, "computed-repair-per-object"));

  let keys: Vec<&str> = repair_map.keys().map(|s| s.as_str()).collect();
  assert_eq!(
    keys,
    vec!["arm_link_1", "arm_link_2"],
    "computed-repair-per-object must have exactly the two world-set object ids as keys; got {:?}",
    keys
  );

  for object_id in ["arm_link_1", "arm_link_2"] {
    let repair = repair_map.get(object_id).unwrap();
    assert_eq!(
      as_str(get(repair, "id")),
      "repair.role-binding",
      "repair-per-object[{}].id must be repair.role-binding",
      object_id
    );
    assert_eq!(
      as_str(get(repair, "promotion")),
      "pending",
      "repair-per-object[{}].promotion must remain pending — v0 forbids auto-apply",
      object_id
    );
    assert_eq!(
      as_str(get(repair, "applies-at")),
      object_id,
      "repair-per-object[{}].applies-at must equal the same object id (parametric reuse)",
      object_id
    );
  }
}

#[test]
fn v0_2_meta_log_aggregates_set_aware_state() {
  // Invariant 52.
  let value = eval_file(&fixture_path()).unwrap();
  let after = get_path(
    &value,
    &["computed-meta-circular-log-differential", "after-turn-5"],
  );

  let active_lenses = list_of_strings(get(after, "active-lenses"));
  assert_eq!(
    active_lenses,
    vec![
      "lens.base",
      "lens.mechanism",
      "lens.animation",
      "lens.projection"
    ],
    "after-turn-5.active-lenses must list all 4 v0 lens ids"
  );

  let open_needs = list_of_strings(get(after, "open-needs"));
  assert_eq!(
    open_needs.len(),
    4,
    "after-turn-5.open-needs must have 4 entries (one per Need-emitting turn); got {:?}",
    open_needs
  );
  assert!(open_needs.contains(&"need.joint-role-binding"));
  assert!(open_needs.contains(&"need.drive-to-joint-binding"));
  assert!(open_needs.contains(&"need.observer-to-frame-binding"));
  assert!(open_needs.contains(&"need.repair-promotion-decision"));

  let open_held = list_of_strings(get(after, "open-held"));
  // v0.2 has two Held instances of the same kind id; the
  // set-aware helper deduplicates by kind, so open-held is the
  // single-element kind list.
  assert_eq!(
    open_held,
    vec!["held.structural-binding-conflict"],
    "after-turn-5.open-held must be the deduplicated kind list (v0.2 has one kind across both objects)"
  );
}

#[test]
fn v0_2_without_owner_law_empty_set_trajectory() {
  // Invariant 53.
  let value = eval_file(&fixture_without_owner_law_path()).unwrap();

  match get(&value, "owner-law-loaded") {
    Value::Bool(b) => assert!(
      !*b,
      "without-owner-law harness must report owner-law-loaded=false"
    ),
    other => panic!("owner-law-loaded must be a Bool, got {:?}", other),
  }

  let turns = as_list(get(&value, "computed-turns"));
  assert!(
    turns.is_empty(),
    "without owner-law: computed-turns must be empty; got {} turns",
    turns.len()
  );

  let held_map = as_attrs(get(&value, "computed-held-per-object"));
  assert!(
    held_map.is_empty(),
    "without owner-law: computed-held-per-object must be empty; got keys {:?}",
    held_map.keys().collect::<Vec<_>>()
  );

  let repair_map = as_attrs(get(&value, "computed-repair-per-object"));
  assert!(
    repair_map.is_empty(),
    "without owner-law: computed-repair-per-object must be empty; got keys {:?}",
    repair_map.keys().collect::<Vec<_>>()
  );

  // No-change meta log.
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
    "without owner-law: after-turn-5 must equal initial-state (no turns occurred);\n\
     initial = {}\n\
     after   = {}",
    initial, after
  );
}

#[test]
fn v0_2_owner_law_file_unchanged_seven_rule_surface() {
  // Invariant 54. v0.2 must NOT have edited v0_owner_law.px.
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
        "v0_owner_law.px must still expose `{}` after v0.2; available: {:?}",
        rule,
        attrs.keys().collect::<Vec<_>>()
      )
    });
    assert!(
      matches!(entry, Value::Lambda { .. }),
      "v0_owner_law.px:`{}` must still be a Lambda after v0.2, got {:?}",
      rule,
      entry
    );
  }

  let allowed: std::collections::BTreeSet<&str> = required_rules.iter().copied().collect();
  for key in attrs.keys() {
    assert!(
      allowed.contains(key.as_str()),
      "v0.2 must NOT add new owner-law rules; v0_owner_law.px carries unexpected key `{}`",
      key
    );
  }
}

#[test]
fn v0_2_no_contamination_in_object_specific_held_and_repair_bodies() {
  // Invariant 55 — REFINED contamination. Shared turn fields
  // (affected-slice, attach-route, conflict[]) MAY mention both
  // ids — that is the set-aware behaviour. But each per-object
  // Held / Repair body must carry only its own id. v0.1's
  // pure-contamination form ("each trace's JSON contains no
  // other id") split into two halves: shared fields can carry
  // both; disjoint per-object bodies must not.
  for (label, path) in [
    ("with-owner-law", fixture_path()),
    ("without-owner-law", fixture_without_owner_law_path()),
  ] {
    let value = eval_file(&path).expect("harness must evaluate");

    // Per-object Held bodies — each must contain only its own id.
    let held_map = as_attrs(get(&value, "computed-held-per-object"));
    for (object_id, held_value) in held_map {
      let other_id = if object_id == "arm_link_1" {
        "arm_link_2"
      } else {
        "arm_link_1"
      };
      let json = held_value.to_json();
      assert!(
        !json.contains(other_id),
        "[{}] held-per-object[{}] body must NOT mention {}; got JSON:\n{}",
        label,
        object_id,
        other_id,
        json
      );
      // Sanity: the held body for arm_link_X must mention its
      // own id (otherwise the contamination assertion above is
      // trivially true). Skip when the map is empty.
      if !json.is_empty() && json != "null" {
        assert!(
          json.contains(object_id),
          "[{}] held-per-object[{}] body must mention its own id; got JSON:\n{}",
          label,
          object_id,
          json
        );
      }
    }

    // Per-object Repair bodies — same rule.
    let repair_map = as_attrs(get(&value, "computed-repair-per-object"));
    for (object_id, repair_value) in repair_map {
      let other_id = if object_id == "arm_link_1" {
        "arm_link_2"
      } else {
        "arm_link_1"
      };
      let json = repair_value.to_json();
      assert!(
        !json.contains(other_id),
        "[{}] repair-per-object[{}] body must NOT mention {}; got JSON:\n{}",
        label,
        object_id,
        other_id,
        json
      );
      if !json.is_empty() && json != "null" {
        assert!(
          json.contains(object_id),
          "[{}] repair-per-object[{}] body must mention its own id; got JSON:\n{}",
          label,
          object_id,
          json
        );
      }
    }
  }
}
