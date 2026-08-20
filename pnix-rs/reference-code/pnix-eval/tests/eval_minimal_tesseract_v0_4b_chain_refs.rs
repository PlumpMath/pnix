//! v0.4b multi-step chain dependency slice — extends the v0.4a
//! single-edge instance-qualified ref proof to a strictly linear
//! chain `arm_link_3 → arm_link_2 → arm_link_1` using only the
//! same relation kind (`depends-on-frame`). Each Need still
//! carries one immediate instance-qualified ref; the runner adds
//! a runtime-computed `transitive-chain-per-object` map exposing
//! the walked chain as a list of step entries. Test 88 (chain
//! shape and content) and test 91 (every chain step resolvable
//! through the same resolver) are the load-bearing v0.4b
//! invariants.
//!
//! Truth owner:        project-wiki/maps/minimal-tesseract-v0-map.md
//!                     §"v0.4b design decision — multi-step chain dependency
//!                       (same relation kind only, no plural kind, no
//!                       promotion, no cycle)"
//! Active scope:       project-wiki/maps/active-domain-constitution.md Art. 8
//!
//! What this test asserts (14 invariants, indices continued from
//! the v0.4a test's 81):
//!  82. world-set has 3 SourceObjects; relations has 2 entries
//!      (both `depends-on-frame`); v0-4b / chain-aware /
//!      instance-qualified-refs markers true.
//!  83. v0.2 trajectory surfaces still present (turns / per-
//!      object Held / per-object Repair maps still 2 entries).
//!  84. arm_link_2's cross-object Need has upstream-held-ref =
//!      instance-qualified attrset { object-id = "arm_link_1";
//!      held-kind = "structural-binding-conflict"; }.
//!  85. arm_link_3's cross-object Need has upstream-held-ref =
//!      instance-qualified attrset { object-id = "arm_link_2";
//!      held-kind = "structural-binding-conflict"; }.
//!  86. Coherence (per edge): each Need's ref.object-id matches
//!      its declared relation's `to` field.
//!  87. Resolver resolves both direct refs (arm_link_1 well-
//!      formed and arm_link_2 well-formed-mid).
//!  88. **load-bearing — transitive chain**:
//!      transitive-chain-per-object[arm_link_3] is a list of
//!      length 2: first { object-id="arm_link_2", has-held=true,
//!      held-instance-ref={...} }; second { object-id="arm_link_1",
//!      has-held=true, held-instance-ref={...} }.
//!  89. transitive-chain-per-object[arm_link_2] has length 1
//!      (single step to arm_link_1).
//!  90. transitive-chain-per-object[arm_link_1] is empty.
//!  91. **load-bearing — chain step resolution**: each chain
//!      step's held-instance-ref shape matches the v0.4a ref
//!      shape and resolves successfully via the same resolver.
//!  92. arm_link_3 is in world-set but has no Held entry
//!      (downstream-only object — heldPerObject still has 2
//!      entries); ref-resolution-cases.terminal-no-held returns
//!      dangling-no-such-object.
//!  93. without-owner-law: chain object-id sequence preserved;
//!      every step has-held=false; held-instance-ref=null;
//!      cross-object Needs non-blocking with null upstream-
//!      held-ref.
//!  94. v0_4b.1 micro-patch: relations are strictly linear and
//!      `from` values are unique (no branching); arm_link_1 has
//!      no outgoing relation. Documents `terminal-no-held` →
//!      `dangling-no-such-object` status-naming debt as a known
//!      limitation (resolver keys by heldPerObject, so an
//!      object-in-world-set-but-no-Held case currently shares
//!      the dangling status; future slice will split into
//!      `dangling-no-such-object` vs `dangling-no-held-instance`).
//!  95. v0_owner_law.px STILL exposes exactly the same 7 Lambda
//!      rules.

use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------
// Helpers — duplicated from v0..v0.4a tests because integration
// test files compile as separate crates.
// ---------------------------------------------------------------

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../../fixtures/minimal-tesseract-v0/v0_4b_run_chain_dependency.px")
}

fn fixture_without_owner_law_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../../fixtures/minimal-tesseract-v0/v0_4b_run_without_owner_law.px")
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
fn v0_4b_world_set_three_objects_two_relations_with_v0_4b_markers() {
  // Invariant 82.
  let value = eval_file(&fixture_path()).expect("v0.4b harness must evaluate");

  let world_set = as_list(get(&value, "world-set"));
  assert_eq!(world_set.len(), 3, "v0.4b world-set must have 3 objects");
  let ids: Vec<&str> = world_set.iter().map(|o| as_str(get(o, "id"))).collect();
  assert_eq!(ids, vec!["arm_link_1", "arm_link_2", "arm_link_3"]);

  let relations = as_list(get(&value, "relations"));
  assert_eq!(
    relations.len(),
    2,
    "v0.4b must declare exactly 2 cross-object relations; got {}",
    relations.len()
  );
  for r in relations {
    assert_eq!(
      as_str(get(r, "kind")),
      "depends-on-frame",
      "v0.4b uses only depends-on-frame relation kind; got {}",
      as_str(get(r, "kind"))
    );
  }

  assert!(as_bool(get(&value, "v0-4b")), "v0-4b marker must be true");
  assert!(
    as_bool(get(&value, "chain-aware")),
    "chain-aware marker must be true"
  );
  assert!(
    as_bool(get(&value, "instance-qualified-refs")),
    "instance-qualified-refs marker must be true"
  );
}

#[test]
fn v0_4b_v0_2_trajectory_forwarded_held_repair_still_two_entries() {
  // Invariant 83.
  let value = eval_file(&fixture_path()).unwrap();

  let turns = as_list(get(&value, "computed-turns"));
  assert_eq!(turns.len(), 6, "v0.4b must forward v0.2's 6 turns");
  let turn_ids: Vec<i64> = turns.iter().map(|t| as_int(get(t, "turn-id"))).collect();
  assert_eq!(turn_ids, vec![0, 1, 2, 3, 4, 5]);

  // Per-object Held / Repair still have 2 entries — arm_link_3
  // is in world-set but does NOT get a Held (downstream-only
  // object).
  let held_map = as_attrs(get(&value, "computed-held-per-object"));
  assert_eq!(
    held_map.len(),
    2,
    "heldPerObject must still have 2 entries (arm_link_3 has no own Held)"
  );
  assert!(held_map.contains_key("arm_link_1"));
  assert!(held_map.contains_key("arm_link_2"));
  assert!(
    !held_map.contains_key("arm_link_3"),
    "arm_link_3 must NOT have its own Held (downstream-only)"
  );

  let repair_map = as_attrs(get(&value, "computed-repair-per-object"));
  assert_eq!(repair_map.len(), 2);
  assert!(!repair_map.contains_key("arm_link_3"));
}

#[test]
fn v0_4b_arm_link_2_need_points_at_arm_link_1_instance_ref() {
  // Invariant 84.
  let value = eval_file(&fixture_path()).unwrap();
  let needs2 = as_list(get_path(
    &value,
    &["computed-cross-object-needs-per-object", "arm_link_2"],
  ));
  assert_eq!(
    needs2.len(),
    1,
    "arm_link_2 must have exactly 1 cross-object Need"
  );

  let ref_attrs = get(&needs2[0], "upstream-held-ref");
  assert!(
    is_attrset(ref_attrs),
    "upstream-held-ref must be a Value::AttrSet; got {:?}",
    ref_attrs
  );
  assert_eq!(as_str(get(ref_attrs, "object-id")), "arm_link_1");
  assert_eq!(
    as_str(get(ref_attrs, "held-kind")),
    "structural-binding-conflict"
  );
}

#[test]
fn v0_4b_arm_link_3_need_points_at_arm_link_2_instance_ref() {
  // Invariant 85.
  let value = eval_file(&fixture_path()).unwrap();
  let needs3 = as_list(get_path(
    &value,
    &["computed-cross-object-needs-per-object", "arm_link_3"],
  ));
  assert_eq!(
    needs3.len(),
    1,
    "arm_link_3 must have exactly 1 cross-object Need"
  );

  let ref_attrs = get(&needs3[0], "upstream-held-ref");
  assert!(
    is_attrset(ref_attrs),
    "upstream-held-ref must be a Value::AttrSet; got {:?}",
    ref_attrs
  );
  assert_eq!(as_str(get(ref_attrs, "object-id")), "arm_link_2");
  assert_eq!(
    as_str(get(ref_attrs, "held-kind")),
    "structural-binding-conflict"
  );
}

#[test]
fn v0_4b_each_need_ref_object_id_matches_its_relation_to_field() {
  // Invariant 86 — coherence per edge.
  let value = eval_file(&fixture_path()).unwrap();
  let relations = as_list(get(&value, "relations"));

  for relation in relations {
    let from = as_str(get(relation, "from"));
    let to = as_str(get(relation, "to"));
    let needs = as_list(get_path(
      &value,
      &["computed-cross-object-needs-per-object", from],
    ));
    let need = needs
      .iter()
      .find(|n| as_str(get(n, "to")) == to)
      .unwrap_or_else(|| panic!("no Need entry from {} to {}", from, to));
    let ref_object_id = as_str(get_path(need, &["upstream-held-ref", "object-id"]));
    assert_eq!(
      ref_object_id, to,
      "Need[{}→{}].upstream-held-ref.object-id must coincide with relation.to",
      from, to
    );
  }
}

#[test]
fn v0_4b_resolver_resolves_both_direct_refs() {
  // Invariant 87 — resolver works on chain root and chain interior.
  let value = eval_file(&fixture_path()).unwrap();

  let well_root = get_path(&value, &["ref-resolution-cases", "well-formed"]);
  assert_eq!(as_str(get(well_root, "status")), "resolved");
  let resolved_root = get(well_root, "resolved");
  let actual_root = get_path(&value, &["computed-held-per-object", "arm_link_1"]);
  assert_eq!(
    resolved_root.to_json(),
    actual_root.to_json(),
    "well-formed (arm_link_1) resolver result must byte-equal the actual Held"
  );

  let well_mid = get_path(&value, &["ref-resolution-cases", "well-formed-mid"]);
  assert_eq!(
    as_str(get(well_mid, "status")),
    "resolved",
    "resolver must work on chain interior (arm_link_2), not just chain root"
  );
  let resolved_mid = get(well_mid, "resolved");
  let actual_mid = get_path(&value, &["computed-held-per-object", "arm_link_2"]);
  assert_eq!(
    resolved_mid.to_json(),
    actual_mid.to_json(),
    "well-formed-mid (arm_link_2) resolver result must byte-equal the actual Held"
  );
}

#[test]
fn v0_4b_transitive_chain_for_arm_link_3_is_full_chain() {
  // Invariant 88 — load-bearing transitive chain proof.
  let value = eval_file(&fixture_path()).unwrap();
  let chain3 = as_list(get_path(
    &value,
    &["transitive-chain-per-object", "arm_link_3"],
  ));
  assert_eq!(
    chain3.len(),
    2,
    "arm_link_3's transitive chain must have length 2 (arm_link_2 then arm_link_1); got {}",
    chain3.len()
  );

  // First step → arm_link_2 (immediate upstream).
  let step1 = &chain3[0];
  assert_eq!(as_str(get(step1, "object-id")), "arm_link_2");
  assert!(
    as_bool(get(step1, "has-held")),
    "step 1 must have has-held=true (arm_link_2 has Held in v0.2 trace)"
  );
  let ref1 = get(step1, "held-instance-ref");
  assert!(
    is_attrset(ref1),
    "step 1 held-instance-ref must be an attrset"
  );
  assert_eq!(as_str(get(ref1, "object-id")), "arm_link_2");
  assert_eq!(
    as_str(get(ref1, "held-kind")),
    "structural-binding-conflict"
  );

  // Second step → arm_link_1 (transitive upstream via arm_link_2).
  let step2 = &chain3[1];
  assert_eq!(as_str(get(step2, "object-id")), "arm_link_1");
  assert!(as_bool(get(step2, "has-held")));
  let ref2 = get(step2, "held-instance-ref");
  assert!(is_attrset(ref2));
  assert_eq!(as_str(get(ref2, "object-id")), "arm_link_1");
  assert_eq!(
    as_str(get(ref2, "held-kind")),
    "structural-binding-conflict"
  );
}

#[test]
fn v0_4b_transitive_chain_for_arm_link_2_has_one_step() {
  // Invariant 89.
  let value = eval_file(&fixture_path()).unwrap();
  let chain2 = as_list(get_path(
    &value,
    &["transitive-chain-per-object", "arm_link_2"],
  ));
  assert_eq!(
    chain2.len(),
    1,
    "arm_link_2's transitive chain must have length 1"
  );
  let step = &chain2[0];
  assert_eq!(as_str(get(step, "object-id")), "arm_link_1");
  assert!(as_bool(get(step, "has-held")));
}

#[test]
fn v0_4b_transitive_chain_for_arm_link_1_is_empty() {
  // Invariant 90.
  let value = eval_file(&fixture_path()).unwrap();
  let chain1 = as_list(get_path(
    &value,
    &["transitive-chain-per-object", "arm_link_1"],
  ));
  assert!(
    chain1.is_empty(),
    "arm_link_1 (chain root) must have empty transitive chain; got {:?}",
    chain1
  );
}

#[test]
fn v0_4b_every_chain_step_held_instance_ref_resolves_through_same_resolver() {
  // Invariant 91 — load-bearing. Walk every chain step and
  // verify its held-instance-ref shape matches the v0.4a ref
  // shape (object-id + held-kind attrset). Then check that
  // the resolver-pre-computed cases for arm_link_1 and
  // arm_link_2 (which are exactly the two chain step targets)
  // both resolve.
  let value = eval_file(&fixture_path()).unwrap();
  let chain_map = as_attrs(get(&value, "transitive-chain-per-object"));

  for (origin, chain_value) in chain_map {
    for (i, step) in as_list(chain_value).iter().enumerate() {
      let ref_value = get(step, "held-instance-ref");
      assert!(
        is_attrset(ref_value),
        "chain[{}][{}].held-instance-ref must be an attrset (matches v0.4a shape); got {:?}",
        origin,
        i,
        ref_value
      );
      // Every step in the with-owner-law trajectory has
      // has-held=true (chain steps are arm_link_1 / arm_link_2,
      // both of which have Helds).
      assert!(
        as_bool(get(step, "has-held")),
        "chain[{}][{}].has-held must be true in with-owner-law trajectory",
        origin,
        i
      );
      // Shape coherence: ref.object-id matches step.object-id.
      assert_eq!(
        as_str(get(ref_value, "object-id")),
        as_str(get(step, "object-id"))
      );
    }
  }

  // Resolver-side proof: pre-computed cases for the two chain
  // step targets resolve.
  let well_root = get_path(&value, &["ref-resolution-cases", "well-formed"]);
  assert_eq!(as_str(get(well_root, "status")), "resolved");
  let well_mid = get_path(&value, &["ref-resolution-cases", "well-formed-mid"]);
  assert_eq!(as_str(get(well_mid, "status")), "resolved");
}

#[test]
fn v0_4b_arm_link_3_has_no_held_and_terminal_ref_dangles() {
  // Invariant 92 — downstream-only object: in world-set but
  // absent from heldPerObject; a ref to arm_link_3 dangles.
  let value = eval_file(&fixture_path()).unwrap();

  let held_map = as_attrs(get(&value, "computed-held-per-object"));
  assert!(
    !held_map.contains_key("arm_link_3"),
    "arm_link_3 must be absent from heldPerObject (downstream-only)"
  );

  let terminal = get_path(&value, &["ref-resolution-cases", "terminal-no-held"]);
  assert_eq!(
    as_str(get(terminal, "status")),
    "dangling-no-such-object",
    "ref to arm_link_3 (in world-set but no Held) must dangle"
  );
  assert!(is_null(get(terminal, "resolved")));
}

#[test]
fn v0_4b_without_owner_law_chain_structurally_observable_but_held_collapsed() {
  // Invariant 93.
  let value = eval_file(&fixture_without_owner_law_path()).unwrap();

  assert!(!as_bool(get(&value, "owner-law-loaded")));

  // Cross-object Needs still exist (relation declarations
  // produce Need entries regardless of owner-law) but are
  // non-blocking with null upstream-held-ref.
  for object_id in ["arm_link_2", "arm_link_3"] {
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
  }

  // Chain object-id sequence preserved structurally; every
  // step has-held=false; held-instance-ref=null.
  let chain3 = as_list(get_path(
    &value,
    &["transitive-chain-per-object", "arm_link_3"],
  ));
  assert_eq!(
    chain3.len(),
    2,
    "without owner-law: chain length still 2 (sequence preserved)"
  );
  let chain3_ids: Vec<&str> = chain3.iter().map(|s| as_str(get(s, "object-id"))).collect();
  assert_eq!(chain3_ids, vec!["arm_link_2", "arm_link_1"]);
  for step in chain3 {
    assert!(
      !as_bool(get(step, "has-held")),
      "without owner-law: every chain step must have has-held=false"
    );
    assert!(
      is_null(get(step, "held-instance-ref")),
      "without owner-law: every chain step held-instance-ref must be null"
    );
  }

  // Resolver behaviour with empty held map.
  let well_root = get_path(&value, &["ref-resolution-cases", "well-formed"]);
  assert_eq!(as_str(get(well_root, "status")), "dangling-no-such-object");
  let well_mid = get_path(&value, &["ref-resolution-cases", "well-formed-mid"]);
  assert_eq!(as_str(get(well_mid, "status")), "dangling-no-such-object");
  let kind_only = get_path(&value, &["ref-resolution-cases", "kind-only"]);
  assert_eq!(as_str(get(kind_only, "status")), "ambiguous-kind-only");
}

#[test]
fn v0_4b_relations_are_linear_unique_from_edges() {
  // Invariant 94 — v0.4b.1 micro-patch. Relations form a
  // strictly linear chain with unique `from` values; the chain
  // root (arm_link_1) has no outgoing relation. This guards
  // future regressions against silent branching (where one
  // object would carry multiple outgoing relations and the
  // walker's `builtins.head` would silently pick one).
  //
  // Also documents (in the test docstring at the top of the
  // file) the `terminal-no-held` → `dangling-no-such-object`
  // status-naming debt: arm_link_3 is in world-set but has no
  // Held entry, and the resolver currently treats this as
  // "no such object" because it keys by heldPerObject. A
  // future slice should split the status.
  let value = eval_file(&fixture_path()).unwrap();
  let relations = as_list(get(&value, "relations"));

  assert_eq!(relations.len(), 2, "v0.4b chain has exactly 2 edges");

  let edge_3_to_2 = &relations[0];
  assert_eq!(as_str(get(edge_3_to_2, "from")), "arm_link_3");
  assert_eq!(as_str(get(edge_3_to_2, "to")), "arm_link_2");

  let edge_2_to_1 = &relations[1];
  assert_eq!(as_str(get(edge_2_to_1, "from")), "arm_link_2");
  assert_eq!(as_str(get(edge_2_to_1, "to")), "arm_link_1");

  let from_values: Vec<&str> = relations.iter().map(|r| as_str(get(r, "from"))).collect();
  let unique_from: std::collections::BTreeSet<&str> = from_values.iter().copied().collect();
  assert_eq!(
    from_values.len(),
    unique_from.len(),
    "relation.from values must be unique (no branching); got {:?}",
    from_values
  );

  let arm_link_1_outgoing: Vec<&Value> = relations
    .iter()
    .filter(|r| as_str(get(r, "from")) == "arm_link_1")
    .collect();
  assert!(
    arm_link_1_outgoing.is_empty(),
    "arm_link_1 (chain root) must have no outgoing relation; got {:?}",
    arm_link_1_outgoing
  );

  // Cross-check the chain map agrees: chain root has empty
  // chain, no other object has length > 2.
  let chain1 = as_list(get_path(
    &value,
    &["transitive-chain-per-object", "arm_link_1"],
  ));
  assert!(
    chain1.is_empty(),
    "chain root must have empty transitive chain"
  );
  let chain_map = as_attrs(get(&value, "transitive-chain-per-object"));
  for (id, chain_value) in chain_map {
    let len = as_list(chain_value).len();
    assert!(
      len <= 2,
      "v0.4b chain must have depth <= 2; {} has depth {}",
      id,
      len
    );
  }
}

#[test]
fn v0_4b_owner_law_file_unchanged_seven_rule_surface() {
  // Invariant 95. v0.4b must NOT have edited v0_owner_law.px.
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
        "v0_owner_law.px must still expose `{}` after v0.4b; available: {:?}",
        rule,
        attrs.keys().collect::<Vec<_>>()
      )
    });
    assert!(
      matches!(entry, Value::Lambda { .. }),
      "v0_owner_law.px:`{}` must still be a Lambda after v0.4b, got {:?}",
      rule,
      entry
    );
  }

  let allowed: std::collections::BTreeSet<&str> = required_rules.iter().copied().collect();
  for key in attrs.keys() {
    assert!(
      allowed.contains(key.as_str()),
      "v0.4b must NOT add new owner-law rules; v0_owner_law.px carries unexpected key `{}`",
      key
    );
  }
}
