//! v0.4c plural-relation-kind chain slice — extends the v0.4b
//! single-kind chain proof to two distinct relation kinds across
//! one strictly linear chain. The chain depth and the
//! instance-qualified ref shape stay v0.4b-equivalent; the
//! load-bearing addition is a `relation-kind` field on each
//! chain step (and on each cross-object-repair-effect entry).
//! Test 102 (chain step relation-kind sequence) and test 106
//! (resolver kind-independence) are the load-bearing v0.4c
//! invariants.
//!
//! Truth owner:        project-wiki/maps/minimal-tesseract-v0-map.md
//!                     §"v0.4c design decision — plural relation kinds
//!                       (two distinct kinds across one chain, no
//!                       promotion, no cycle)"
//! Active scope:       project-wiki/maps/active-domain-constitution.md Art. 8
//!
//! What this test asserts (17 invariants, indices continued from
//! the v0.4b test's 95; invariants 111..112 are v0.4c.1 micro-
//! patch — Need.id format hygiene plus repair-effect existence):
//!  96. world-set has 3 SourceObjects [arm_link_1, arm_link_2,
//!      arm_link_3]; relations has 2 entries with kinds
//!      [depends-on-frame, mounted-on] in declared order;
//!      v0-4c / chain-aware / instance-qualified-refs /
//!      plural-kind-aware markers true.
//!  97. v0.2 trajectory surfaces still present (computed-turns
//!      has 6 turns; per-object Held / Repair maps still 2
//!      entries — arm_link_3 still has no Held).
//!  98. arm_link_3's cross-object Need has kind =
//!      depends-on-frame (matches its declared relation's kind).
//!  99. arm_link_2's cross-object Need has kind = mounted-on
//!      (matches its declared relation's kind).
//! 100. Each Need's upstream-held-ref still has v0.4a
//!      instance-qualified shape { object-id; held-kind; }
//!      with held-kind = "structural-binding-conflict" —
//!      resolver stays kind-independent.
//! 101. Coherence (per edge): each Need's
//!      upstream-held-ref.object-id == relation.to AND
//!      Need.kind == relation.kind.
//! 102. **load-bearing — chain step relation-kind preserved**:
//!      transitive-chain-per-object[arm_link_3] has length 2
//!      where step 0.relation-kind = depends-on-frame and
//!      step 1.relation-kind = mounted-on (the declared
//!      relations order).
//! 103. transitive-chain-per-object[arm_link_2] has length 1
//!      with relation-kind = mounted-on.
//! 104. transitive-chain-per-object[arm_link_1] is empty
//!      (chain root).
//! 105. Chain step shape is exactly { object-id; relation-kind;
//!      has-held; held-instance-ref; } — no extra fields, no
//!      missing fields.
//! 106. **load-bearing — resolver kind-independence**: every
//!      chain step's held-instance-ref resolves with status
//!      "resolved" regardless of the step's relation-kind.
//! 107. cross-object-repair-effect entries each carry
//!      relation-kind matching the declared relation's kind;
//!      applied stays false.
//! 108. without-owner-law: chain object-id sequence preserved
//!      AND chain relation-kind sequence preserved; every
//!      step has-held = false; held-instance-ref = null;
//!      cross-object Needs non-blocking with null upstream-
//!      held-ref but Need.kind STILL preserves declared
//!      relation.kind.
//! 109. v0.4b.1 linearity carries forward: relations strictly
//!      linear; relation.from values unique; arm_link_1 has
//!      no outgoing relation.
//! 110. v0_owner_law.px STILL exposes exactly the same 7
//!      Lambda rules.
//! 111. v0.4c.1 micro-patch — Need.id format carries the
//!      relation-kind in the kind segment AND uses a generic
//!      `.to.` separator (not the v0.4b legacy hardcoded
//!      `.depends-on.` segment). arm_link_3's Need.id is
//!      exactly
//!      `need.cross-object.depends-on-frame.arm_link_3.to.arm_link_2`;
//!      arm_link_2's Need.id is exactly
//!      `need.cross-object.mounted-on.arm_link_2.to.arm_link_1`.
//!      Repair-effect downstream-need-id matches those exact
//!      ids.
//! 112. v0.4c.1 micro-patch — repair-effect existence and
//!      kind: arm_link_1's repair-effect has exactly 1 entry
//!      with relation-kind = "mounted-on" and downstream-
//!      object = "arm_link_2"; arm_link_2's repair-effect has
//!      exactly 1 entry with relation-kind = "depends-on-frame"
//!      and downstream-object = "arm_link_3"; arm_link_3's
//!      repair-effect is empty (no upstream Repair points at
//!      it). Strengthens v0.4c invariant 107 by asserting
//!      entry existence and downstream wiring directly,
//!      rather than skipping empty entry lists.

use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------
// Helpers — duplicated from prior v0..v0.4b tests because
// integration test files compile as separate crates.
// ---------------------------------------------------------------

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../../fixtures/minimal-tesseract-v0/v0_4c_run_plural_kind_chain.px")
}

fn fixture_without_owner_law_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../../fixtures/minimal-tesseract-v0/v0_4c_run_without_owner_law.px")
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

fn is_attrset(v: &Value) -> bool {
  matches!(v, Value::AttrSet(_))
}

// ---------------------------------------------------------------
// Tests.
// ---------------------------------------------------------------

#[test]
fn v0_4c_world_set_three_objects_two_relations_with_plural_kinds() {
  // Invariant 96.
  let value = eval_file(&fixture_path()).expect("v0.4c harness must evaluate");

  let world_set = as_list(get(&value, "world-set"));
  assert_eq!(world_set.len(), 3, "v0.4c world-set must have 3 objects");
  let ids: Vec<&str> = world_set.iter().map(|o| as_str(get(o, "id"))).collect();
  assert_eq!(ids, vec!["arm_link_1", "arm_link_2", "arm_link_3"]);

  let relations = as_list(get(&value, "relations"));
  assert_eq!(relations.len(), 2, "v0.4c must declare exactly 2 relations");
  let kinds: Vec<&str> = relations.iter().map(|r| as_str(get(r, "kind"))).collect();
  assert_eq!(
    kinds,
    vec!["depends-on-frame", "mounted-on"],
    "v0.4c relations carry two distinct kinds in declared order"
  );

  assert!(as_bool(get(&value, "v0-4c")), "v0-4c marker must be true");
  assert!(
    as_bool(get(&value, "chain-aware")),
    "chain-aware marker must be true"
  );
  assert!(
    as_bool(get(&value, "instance-qualified-refs")),
    "instance-qualified-refs marker must be true"
  );
  assert!(
    as_bool(get(&value, "plural-kind-aware")),
    "plural-kind-aware marker must be true"
  );
}

#[test]
fn v0_4c_v0_2_trajectory_forwarded_held_repair_still_two_entries() {
  // Invariant 97.
  let value = eval_file(&fixture_path()).unwrap();

  let turns = as_list(get(&value, "computed-turns"));
  assert_eq!(turns.len(), 6, "v0.4c must forward v0.2's 6 turns");
  let turn_ids: Vec<i64> = turns.iter().map(|t| as_int(get(t, "turn-id"))).collect();
  assert_eq!(turn_ids, vec![0, 1, 2, 3, 4, 5]);

  let held_map = as_attrs(get(&value, "computed-held-per-object"));
  assert_eq!(
    held_map.len(),
    2,
    "heldPerObject must still have 2 entries (arm_link_3 has no own Held)"
  );
  assert!(held_map.contains_key("arm_link_1"));
  assert!(held_map.contains_key("arm_link_2"));
  assert!(!held_map.contains_key("arm_link_3"));

  let repair_map = as_attrs(get(&value, "computed-repair-per-object"));
  assert_eq!(repair_map.len(), 2);
  assert!(!repair_map.contains_key("arm_link_3"));
}

#[test]
fn v0_4c_arm_link_3_need_kind_is_depends_on_frame() {
  // Invariant 98.
  let value = eval_file(&fixture_path()).unwrap();
  let needs3 = as_list(get_path(
    &value,
    &["computed-cross-object-needs-per-object", "arm_link_3"],
  ));
  assert_eq!(needs3.len(), 1);
  assert_eq!(
    as_str(get(&needs3[0], "kind")),
    "depends-on-frame",
    "arm_link_3's Need.kind must mirror its declared relation's kind"
  );
}

#[test]
fn v0_4c_arm_link_2_need_kind_is_mounted_on() {
  // Invariant 99.
  let value = eval_file(&fixture_path()).unwrap();
  let needs2 = as_list(get_path(
    &value,
    &["computed-cross-object-needs-per-object", "arm_link_2"],
  ));
  assert_eq!(needs2.len(), 1);
  assert_eq!(
    as_str(get(&needs2[0], "kind")),
    "mounted-on",
    "arm_link_2's Need.kind must mirror its declared relation's kind"
  );
}

#[test]
fn v0_4c_each_need_held_ref_is_v0_4a_shape_kind_independent() {
  // Invariant 100. Both Needs (depends-on-frame and mounted-on)
  // resolve their upstream-held-ref into the same v0.4a-shape
  // attrset. The Held kind is `structural-binding-conflict` for
  // both, regardless of relation-kind.
  let value = eval_file(&fixture_path()).unwrap();
  for (from_obj, expected_to) in [("arm_link_3", "arm_link_2"), ("arm_link_2", "arm_link_1")] {
    let needs = as_list(get_path(
      &value,
      &["computed-cross-object-needs-per-object", from_obj],
    ));
    assert_eq!(
      needs.len(),
      1,
      "{} must have exactly 1 cross-object Need",
      from_obj
    );
    let ref_attrs = get(&needs[0], "upstream-held-ref");
    assert!(
      is_attrset(ref_attrs),
      "{}'s upstream-held-ref must be an attrset (v0.4a shape preserved)",
      from_obj
    );
    assert_eq!(as_str(get(ref_attrs, "object-id")), expected_to);
    assert_eq!(
      as_str(get(ref_attrs, "held-kind")),
      "structural-binding-conflict"
    );
  }
}

#[test]
fn v0_4c_each_need_object_id_and_kind_match_relation() {
  // Invariant 101 — coherence per edge.
  let value = eval_file(&fixture_path()).unwrap();
  let relations = as_list(get(&value, "relations"));

  for relation in relations {
    let from = as_str(get(relation, "from"));
    let to = as_str(get(relation, "to"));
    let kind = as_str(get(relation, "kind"));
    let needs = as_list(get_path(
      &value,
      &["computed-cross-object-needs-per-object", from],
    ));
    let need = needs
      .iter()
      .find(|n| as_str(get(n, "to")) == to)
      .unwrap_or_else(|| panic!("no Need entry from {} to {}", from, to));
    assert_eq!(
      as_str(get_path(need, &["upstream-held-ref", "object-id"])),
      to,
      "Need.upstream-held-ref.object-id must match relation.to"
    );
    assert_eq!(
      as_str(get(need, "kind")),
      kind,
      "Need.kind must match relation.kind"
    );
  }
}

#[test]
fn v0_4c_chain_for_arm_link_3_has_two_kinds_in_declared_order() {
  // Invariant 102 — load-bearing.
  let value = eval_file(&fixture_path()).unwrap();
  let chain3 = as_list(get_path(
    &value,
    &["transitive-chain-per-object", "arm_link_3"],
  ));
  assert_eq!(chain3.len(), 2, "arm_link_3 chain must have length 2");

  let step0 = &chain3[0];
  assert_eq!(as_str(get(step0, "object-id")), "arm_link_2");
  assert_eq!(
    as_str(get(step0, "relation-kind")),
    "depends-on-frame",
    "chain step 0 must carry the kind of the producing edge (arm_link_3 → arm_link_2)"
  );
  assert!(as_bool(get(step0, "has-held")));

  let step1 = &chain3[1];
  assert_eq!(as_str(get(step1, "object-id")), "arm_link_1");
  assert_eq!(
    as_str(get(step1, "relation-kind")),
    "mounted-on",
    "chain step 1 must carry the kind of the producing edge (arm_link_2 → arm_link_1)"
  );
  assert!(as_bool(get(step1, "has-held")));
}

#[test]
fn v0_4c_chain_for_arm_link_2_has_mounted_on() {
  // Invariant 103.
  let value = eval_file(&fixture_path()).unwrap();
  let chain2 = as_list(get_path(
    &value,
    &["transitive-chain-per-object", "arm_link_2"],
  ));
  assert_eq!(chain2.len(), 1, "arm_link_2 chain must have length 1");
  let step = &chain2[0];
  assert_eq!(as_str(get(step, "object-id")), "arm_link_1");
  assert_eq!(as_str(get(step, "relation-kind")), "mounted-on");
}

#[test]
fn v0_4c_chain_for_arm_link_1_is_empty() {
  // Invariant 104.
  let value = eval_file(&fixture_path()).unwrap();
  let chain1 = as_list(get_path(
    &value,
    &["transitive-chain-per-object", "arm_link_1"],
  ));
  assert!(
    chain1.is_empty(),
    "chain root arm_link_1 must have empty chain"
  );
}

#[test]
fn v0_4c_chain_step_shape_is_exactly_four_fields() {
  // Invariant 105.
  let value = eval_file(&fixture_path()).unwrap();
  let chain_map = as_attrs(get(&value, "transitive-chain-per-object"));
  let expected: std::collections::BTreeSet<&str> = [
    "object-id",
    "relation-kind",
    "has-held",
    "held-instance-ref",
  ]
  .iter()
  .copied()
  .collect();
  for (object_id, chain_value) in chain_map {
    for step in as_list(chain_value) {
      let actual: std::collections::BTreeSet<&str> =
        as_attrs(step).keys().map(|k| k.as_str()).collect();
      assert_eq!(
        actual, expected,
        "chain step shape mismatch for object {}: got {:?}",
        object_id, actual
      );
    }
  }
}

#[test]
fn v0_4c_resolver_kind_independent_resolves_every_chain_step() {
  // Invariant 106 — load-bearing.
  let value = eval_file(&fixture_path()).unwrap();
  let chain3 = as_list(get_path(
    &value,
    &["transitive-chain-per-object", "arm_link_3"],
  ));
  let held_map = as_attrs(get(&value, "computed-held-per-object"));

  for step in chain3 {
    let object_id = as_str(get(step, "object-id"));
    let ref_attrs = get(step, "held-instance-ref");
    assert!(
      is_attrset(ref_attrs),
      "chain step {} must carry an attrset held-instance-ref",
      object_id
    );
    assert_eq!(as_str(get(ref_attrs, "object-id")), object_id);
    let actual_held = held_map
      .get(object_id)
      .unwrap_or_else(|| panic!("held map must contain {}", object_id));
    assert_eq!(
      as_str(get(actual_held, "held-kind")),
      as_str(get(ref_attrs, "held-kind")),
      "chain step {} held-kind must match heldPerObject's actual held-kind (resolver stays kind-independent)",
      object_id
    );
  }

  // Spot-check the resolver result for the chain root via the
  // pre-computed cases. Resolver returns "resolved" regardless of
  // which relation-kind the step carries.
  let well_root = get_path(&value, &["ref-resolution-cases", "well-formed"]);
  assert_eq!(as_str(get(well_root, "status")), "resolved");
  let well_mid = get_path(&value, &["ref-resolution-cases", "well-formed-mid"]);
  assert_eq!(as_str(get(well_mid, "status")), "resolved");
}

#[test]
fn v0_4c_repair_effect_carries_relation_kind_and_applied_false() {
  // Invariant 107 + invariant 112 (v0.4c.1 micro-patch
  // strengthening). Per upstream object, repair-effect entries
  // carry relation-kind matching the declared edge's kind;
  // applied stays false (kind-dependent policy is v0.5+). The
  // v0.4c.1 strengthening asserts entry existence and
  // downstream wiring per upstream directly, rather than
  // skipping empty entry lists.
  let value = eval_file(&fixture_path()).unwrap();
  let repair_effect_map = as_attrs(get(&value, "computed-cross-object-repair-effect"));

  // arm_link_1's Repair has 1 downstream effect: arm_link_2 via
  // mounted-on (the v0.4c edge arm_link_2 → arm_link_1).
  let arm_link_1_entries = as_list(
    repair_effect_map
      .get("arm_link_1")
      .expect("repair-effect map must include arm_link_1"),
  );
  assert_eq!(
    arm_link_1_entries.len(),
    1,
    "arm_link_1's repair-effect must have exactly 1 entry (downstream arm_link_2 via mounted-on)"
  );
  let entry_1 = &arm_link_1_entries[0];
  assert_eq!(as_str(get(entry_1, "relation-kind")), "mounted-on");
  assert_eq!(as_str(get(entry_1, "downstream-object")), "arm_link_2");
  assert!(!as_bool(get(entry_1, "applied")));

  // arm_link_2's Repair has 1 downstream effect: arm_link_3 via
  // depends-on-frame (the v0.4c edge arm_link_3 → arm_link_2).
  let arm_link_2_entries = as_list(
    repair_effect_map
      .get("arm_link_2")
      .expect("repair-effect map must include arm_link_2"),
  );
  assert_eq!(
    arm_link_2_entries.len(),
    1,
    "arm_link_2's repair-effect must have exactly 1 entry (downstream arm_link_3 via depends-on-frame)"
  );
  let entry_2 = &arm_link_2_entries[0];
  assert_eq!(as_str(get(entry_2, "relation-kind")), "depends-on-frame");
  assert_eq!(as_str(get(entry_2, "downstream-object")), "arm_link_3");
  assert!(!as_bool(get(entry_2, "applied")));

  // arm_link_3 is downstream-only — no relation declares it as
  // an upstream `to`, so its repair-effect entry list is empty.
  let arm_link_3_entries = as_list(
    repair_effect_map
      .get("arm_link_3")
      .expect("repair-effect map must include arm_link_3 (even when empty)"),
  );
  assert!(
    arm_link_3_entries.is_empty(),
    "arm_link_3's repair-effect must be empty (downstream-only object); got {} entries",
    arm_link_3_entries.len()
  );
}

#[test]
fn v0_4c_need_id_format_uses_generic_to_separator() {
  // Invariant 111 (v0.4c.1 micro-patch). Need.id format
  // carries the relation-kind in the kind segment AND uses a
  // generic `.to.` separator. The pre-v0.4c.1 format had a
  // hardcoded `.depends-on.` segment that was wrong for any
  // relation kind other than depends-on-frame; v0.4c.1
  // replaces it with `.to.` so the id is consistent across
  // kinds. Also asserts the repair-effect downstream-need-id
  // matches the same exact format.
  let value = eval_file(&fixture_path()).unwrap();

  let needs3 = as_list(get_path(
    &value,
    &["computed-cross-object-needs-per-object", "arm_link_3"],
  ));
  assert_eq!(needs3.len(), 1);
  let need_3 = &needs3[0];
  let need_3_id = as_str(get(need_3, "id"));
  assert_eq!(
    need_3_id, "need.cross-object.depends-on-frame.arm_link_3.to.arm_link_2",
    "arm_link_3's Need.id must use the v0.4c.1 generic `.to.` separator"
  );
  assert!(
    !need_3_id.contains(".depends-on."),
    "Need.id must NOT contain the legacy `.depends-on.` segment; got {}",
    need_3_id
  );

  let needs2 = as_list(get_path(
    &value,
    &["computed-cross-object-needs-per-object", "arm_link_2"],
  ));
  assert_eq!(needs2.len(), 1);
  let need_2 = &needs2[0];
  let need_2_id = as_str(get(need_2, "id"));
  assert_eq!(
    need_2_id, "need.cross-object.mounted-on.arm_link_2.to.arm_link_1",
    "arm_link_2's Need.id must use the v0.4c.1 generic `.to.` separator"
  );
  assert!(
    !need_2_id.contains(".depends-on."),
    "Need.id must NOT contain the legacy `.depends-on.` segment; got {}",
    need_2_id
  );

  // Repair-effect downstream-need-id must match the new format
  // exactly, so a future change cannot drift the two formats
  // out of sync.
  let repair_effect_map = as_attrs(get(&value, "computed-cross-object-repair-effect"));
  let arm_link_1_entries = as_list(repair_effect_map.get("arm_link_1").unwrap());
  let downstream_need_id_for_arm_link_1 = as_str(get(&arm_link_1_entries[0], "downstream-need-id"));
  assert_eq!(
    downstream_need_id_for_arm_link_1, "need.cross-object.mounted-on.arm_link_2.to.arm_link_1",
    "repair-effect downstream-need-id for arm_link_1 must match arm_link_2's Need.id exactly"
  );
  let arm_link_2_entries = as_list(repair_effect_map.get("arm_link_2").unwrap());
  let downstream_need_id_for_arm_link_2 = as_str(get(&arm_link_2_entries[0], "downstream-need-id"));
  assert_eq!(
    downstream_need_id_for_arm_link_2,
    "need.cross-object.depends-on-frame.arm_link_3.to.arm_link_2",
    "repair-effect downstream-need-id for arm_link_2 must match arm_link_3's Need.id exactly"
  );
}

#[test]
fn v0_4c_without_owner_law_kind_sequence_preserved_held_collapsed() {
  // Invariant 108.
  let value = eval_file(&fixture_without_owner_law_path()).unwrap();

  assert!(!as_bool(get(&value, "owner-law-loaded")));

  // Cross-object Needs still exist; non-blocking; ref null;
  // Need.kind STILL preserves declared relation.kind.
  for (object_id, expected_kind) in [
    ("arm_link_3", "depends-on-frame"),
    ("arm_link_2", "mounted-on"),
  ] {
    let needs = as_list(get_path(
      &value,
      &["computed-cross-object-needs-per-object", object_id],
    ));
    assert_eq!(needs.len(), 1, "{} must still have 1 Need entry", object_id);
    let need = &needs[0];
    assert!(
      is_null(get(need, "upstream-held-ref")),
      "without owner-law: upstream-held-ref must be null"
    );
    assert!(
      !as_bool(get(need, "blocking")),
      "without owner-law: cross-object Need must be non-blocking"
    );
    assert_eq!(
      as_str(get(need, "kind")),
      expected_kind,
      "without owner-law: Need.kind STILL preserves declared relation.kind"
    );
  }

  // Chain object-id sequence AND relation-kind sequence
  // preserved structurally; every step has-held=false;
  // held-instance-ref=null.
  let chain3 = as_list(get_path(
    &value,
    &["transitive-chain-per-object", "arm_link_3"],
  ));
  assert_eq!(chain3.len(), 2);
  let kinds: Vec<&str> = chain3
    .iter()
    .map(|s| as_str(get(s, "relation-kind")))
    .collect();
  assert_eq!(
    kinds,
    vec!["depends-on-frame", "mounted-on"],
    "without owner-law: chain relation-kind sequence preserved"
  );
  let ids: Vec<&str> = chain3.iter().map(|s| as_str(get(s, "object-id"))).collect();
  assert_eq!(ids, vec!["arm_link_2", "arm_link_1"]);
  for step in chain3 {
    assert!(!as_bool(get(step, "has-held")));
    assert!(is_null(get(step, "held-instance-ref")));
  }

  // Resolver behaviour with empty held map.
  let well_root = get_path(&value, &["ref-resolution-cases", "well-formed"]);
  assert_eq!(as_str(get(well_root, "status")), "dangling-no-such-object");
  let kind_only = get_path(&value, &["ref-resolution-cases", "kind-only"]);
  assert_eq!(as_str(get(kind_only, "status")), "ambiguous-kind-only");
}

#[test]
fn v0_4c_relations_are_linear_unique_from_edges() {
  // Invariant 109 — v0.4b.1 linearity carries forward.
  let value = eval_file(&fixture_path()).unwrap();
  let relations = as_list(get(&value, "relations"));

  assert_eq!(relations.len(), 2);

  let edge_3_to_2 = &relations[0];
  assert_eq!(as_str(get(edge_3_to_2, "from")), "arm_link_3");
  assert_eq!(as_str(get(edge_3_to_2, "to")), "arm_link_2");

  let edge_2_to_1 = &relations[1];
  assert_eq!(as_str(get(edge_2_to_1, "from")), "arm_link_2");
  assert_eq!(as_str(get(edge_2_to_1, "to")), "arm_link_1");

  let from_values: Vec<&str> = relations.iter().map(|r| as_str(get(r, "from"))).collect();
  let unique_from: std::collections::BTreeSet<&str> = from_values.iter().copied().collect();
  assert_eq!(from_values.len(), unique_from.len());

  let arm_link_1_outgoing: Vec<&Value> = relations
    .iter()
    .filter(|r| as_str(get(r, "from")) == "arm_link_1")
    .collect();
  assert!(
    arm_link_1_outgoing.is_empty(),
    "arm_link_1 root must have no outgoing relation"
  );
}

#[test]
fn v0_4c_owner_law_file_unchanged_seven_rule_surface() {
  // Invariant 110. v0.4c must NOT have edited v0_owner_law.px.
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
        "v0_owner_law.px must still expose `{}` after v0.4c; available: {:?}",
        rule,
        attrs.keys().collect::<Vec<_>>()
      )
    });
    assert!(
      matches!(entry, Value::Lambda { .. }),
      "v0_owner_law.px:`{}` must still be a Lambda after v0.4c, got {:?}",
      rule,
      entry
    );
  }

  let allowed: std::collections::BTreeSet<&str> = required_rules.iter().copied().collect();
  for key in attrs.keys() {
    assert!(
      allowed.contains(key.as_str()),
      "v0.4c must NOT add new owner-law rules; v0_owner_law.px carries unexpected key `{}`",
      key
    );
  }
}
