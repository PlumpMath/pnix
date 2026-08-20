//! v0.4a Held/Need instance-qualified reference slice — closes the
//! Held instance identity gap v0.3 left dangling. The cross-object
//! Need's `upstream-held-ref` is now an instance-qualified attrset
//! `{ object-id; held-kind; }`, and a fixture-local resolver
//! exposes four pre-computed resolution cases (well-formed, kind-
//! only, wrong-object, wrong-kind). Test 71 (ref shape upgrade is
//! a real attrset) and tests 75..78 (four resolution statuses)
//! are the load-bearing v0.4a invariants.
//!
//! Truth owner:        project-wiki/maps/minimal-tesseract-v0-map.md
//!                     §"v0.4a design decision — Held/Need instance-qualified
//!                       reference (kind+object-id, no chain, no plural kind,
//!                       no promotion)"
//! Active scope:       project-wiki/maps/active-domain-constitution.md Art. 8
//!
//! What this test asserts (13 invariants, indices continued from
//! the v0.3 test's 68):
//!  69. world-set has 2 SourceObjects; relations has 1 entry;
//!      v0-4 / instance-qualified-refs markers are true.
//!  70. v0.2 trajectory surfaces still present (turns / per-object
//!      Held / per-object Repair / meta-log).
//!  71. arm_link_2's cross-object Need has upstream-held-ref as
//!      a `Value::AttrSet` (NOT a `Value::String`).
//!  72. upstream-held-ref.object-id == "arm_link_1".
//!  73. upstream-held-ref.held-kind == "structural-binding-conflict".
//!  74. **load-bearing**: ref.object-id matches the relation's
//!      `to` field — coherence between declared relation and
//!      produced ref.
//!  75. ref-resolution-cases.well-formed.status == "resolved" AND
//!      .resolved is the actual computed-held-per-object[arm_link_1]
//!      attrset.
//!  76. ref-resolution-cases.kind-only.status == "ambiguous-kind-only"
//!      AND .resolved == null.
//!  77. ref-resolution-cases.wrong-object.status == "dangling-no-such-object"
//!      AND .resolved == null.
//!  78. ref-resolution-cases.wrong-kind.status == "kind-mismatch"
//!      AND .resolved == null.
//!  79. cross-object Repair-effect carries `resolves-held-instance-ref`
//!      as instance-qualified attrset; arm_link_2 effect map empty
//!      (asymmetry preserved from v0.3).
//!  80. without-owner-law: cross-object Need's upstream-held-ref
//!      collapses to null; resolver cases return appropriate
//!      non-resolved statuses.
//!  81. v0_owner_law.px STILL exposes exactly the same 7 Lambda
//!      rules.

use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------
// Helpers — duplicated from v0 / v0.1 / v0.2 / v0.3 tests because
// integration test files compile as separate crates.
// ---------------------------------------------------------------

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base)
    .join("../../fixtures/minimal-tesseract-v0/v0_4_run_instance_qualified_dependency.px")
}

fn fixture_without_owner_law_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../../fixtures/minimal-tesseract-v0/v0_4_run_without_owner_law.px")
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

fn is_null(v: &Value) -> bool {
  matches!(v, Value::Null)
}

fn is_attrset(v: &Value) -> bool {
  matches!(v, Value::AttrSet(_))
}

fn is_string(v: &Value) -> bool {
  matches!(v, Value::String(_) | Value::StringContext { .. })
}

// ---------------------------------------------------------------
// Tests.
// ---------------------------------------------------------------

#[test]
fn v0_4_world_set_two_objects_one_relation_with_v0_4_markers() {
  // Invariant 69.
  let value = eval_file(&fixture_path()).expect("v0.4a harness must evaluate");

  let world_set = as_list(get(&value, "world-set"));
  assert_eq!(world_set.len(), 2, "v0.4a world-set must have 2 objects");
  let ids: Vec<&str> = world_set.iter().map(|o| as_str(get(o, "id"))).collect();
  assert_eq!(ids, vec!["arm_link_1", "arm_link_2"]);

  let relations = as_list(get(&value, "relations"));
  assert_eq!(
    relations.len(),
    1,
    "v0.4a must declare exactly 1 cross-object relation; got {}",
    relations.len()
  );

  match get(&value, "v0-4") {
    Value::Bool(b) => assert!(*b, "v0-4 marker must be true"),
    other => panic!("v0-4 must be a Bool, got {:?}", other),
  }
  match get(&value, "instance-qualified-refs") {
    Value::Bool(b) => assert!(*b, "instance-qualified-refs marker must be true"),
    other => panic!("instance-qualified-refs must be a Bool, got {:?}", other),
  }
  match get(&value, "cross-object-aware") {
    Value::Bool(b) => assert!(*b, "cross-object-aware marker must be true"),
    other => panic!("cross-object-aware must be a Bool, got {:?}", other),
  }
}

#[test]
fn v0_4_v0_2_trajectory_forwarded_unchanged() {
  // Invariant 70 — v0.2 surfaces flow through v0.4a verbatim
  // (v0.4a imports v0_2_run_set_aware.px directly).
  let value = eval_file(&fixture_path()).unwrap();

  let turns = as_list(get(&value, "computed-turns"));
  assert_eq!(turns.len(), 6, "v0.4a must forward v0.2's 6 turns");
  let turn_ids: Vec<i64> = turns.iter().map(|t| as_int(get(t, "turn-id"))).collect();
  assert_eq!(turn_ids, vec![0, 1, 2, 3, 4, 5]);

  let held_map = as_attrs(get(&value, "computed-held-per-object"));
  assert_eq!(held_map.len(), 2);
  assert!(held_map.contains_key("arm_link_1"));
  assert!(held_map.contains_key("arm_link_2"));

  let repair_map = as_attrs(get(&value, "computed-repair-per-object"));
  assert_eq!(repair_map.len(), 2);
  assert!(repair_map.contains_key("arm_link_1"));
  assert!(repair_map.contains_key("arm_link_2"));

  // Meta-log forwarded.
  let meta = get(&value, "computed-meta-circular-log-differential");
  assert!(is_attrset(meta), "meta-log must be a forwarded attrset");
}

#[test]
fn v0_4_upstream_held_ref_is_attrset_not_string() {
  // Invariant 71 — load-bearing shape proof. The whole point of
  // v0.4a is that this field is no longer a string.
  let value = eval_file(&fixture_path()).unwrap();
  let needs2 = as_list(get_path(
    &value,
    &["computed-cross-object-needs-per-object", "arm_link_2"],
  ));
  assert_eq!(needs2.len(), 1);
  let need = &needs2[0];
  let upstream_held_ref = get(need, "upstream-held-ref");

  assert!(
    is_attrset(upstream_held_ref),
    "v0.4a upstream-held-ref MUST be a Value::AttrSet (not a string); got {:?}",
    upstream_held_ref
  );
  assert!(
    !is_string(upstream_held_ref),
    "v0.4a upstream-held-ref MUST NOT be a string (regression to v0.3 shape); got {:?}",
    upstream_held_ref
  );
}

#[test]
fn v0_4_upstream_held_ref_object_id_and_held_kind() {
  // Invariants 72 + 73.
  let value = eval_file(&fixture_path()).unwrap();
  let need = &as_list(get_path(
    &value,
    &["computed-cross-object-needs-per-object", "arm_link_2"],
  ))[0];
  let ref_attrs = get(need, "upstream-held-ref");

  let object_id = as_str(get(ref_attrs, "object-id"));
  assert_eq!(
    object_id, "arm_link_1",
    "ref.object-id must be `arm_link_1` (the relation's `to` field)"
  );

  let held_kind = as_str(get(ref_attrs, "held-kind"));
  assert_eq!(
    held_kind, "structural-binding-conflict",
    "ref.held-kind must match the upstream Held's actual kind"
  );
}

#[test]
fn v0_4_ref_object_id_matches_relation_to() {
  // Invariant 74 — coherence between declared relation and
  // produced ref. This is the load-bearing structural invariant
  // for the ref carrier shape.
  let value = eval_file(&fixture_path()).unwrap();
  let relations = as_list(get(&value, "relations"));
  assert_eq!(relations.len(), 1);
  let relation = &relations[0];
  let relation_to = as_str(get(relation, "to"));

  let need = &as_list(get_path(
    &value,
    &["computed-cross-object-needs-per-object", "arm_link_2"],
  ))[0];
  let ref_object_id = as_str(get_path(need, &["upstream-held-ref", "object-id"]));

  assert_eq!(
    ref_object_id, relation_to,
    "ref.object-id `{}` must coincide with the declared relation's `to` field `{}`",
    ref_object_id, relation_to
  );
}

#[test]
fn v0_4_resolver_well_formed_returns_resolved_with_real_held() {
  // Invariant 75.
  let value = eval_file(&fixture_path()).unwrap();
  let well = get_path(&value, &["ref-resolution-cases", "well-formed"]);

  let status = as_str(get(well, "status"));
  assert_eq!(status, "resolved", "well-formed ref must resolve");

  let resolved = get(well, "resolved");
  assert!(
    is_attrset(resolved),
    "resolved value must be the actual Held attrset; got {:?}",
    resolved
  );

  // The resolved value must byte-equal the actual Held entry.
  // (Compare via deterministic JSON projection because Value
  // does not implement PartialEq.)
  let actual_held = get_path(&value, &["computed-held-per-object", "arm_link_1"]);
  assert_eq!(
    resolved.to_json(),
    actual_held.to_json(),
    "resolver's `.resolved` must byte-equal computed-held-per-object[arm_link_1]"
  );
}

#[test]
fn v0_4_resolver_kind_only_is_ambiguous() {
  // Invariant 76 — kind-only refs must NOT silently resolve.
  let value = eval_file(&fixture_path()).unwrap();
  let kind_only = get_path(&value, &["ref-resolution-cases", "kind-only"]);

  let status = as_str(get(kind_only, "status"));
  assert_eq!(
    status, "ambiguous-kind-only",
    "kind-only ref (no object-id) must be flagged as ambiguous"
  );

  assert!(
    is_null(get(kind_only, "resolved")),
    "kind-only ref must have resolved == null"
  );
}

#[test]
fn v0_4_resolver_wrong_object_is_dangling() {
  // Invariant 77.
  let value = eval_file(&fixture_path()).unwrap();
  let wrong_obj = get_path(&value, &["ref-resolution-cases", "wrong-object"]);

  let status = as_str(get(wrong_obj, "status"));
  assert_eq!(
    status, "dangling-no-such-object",
    "ref to non-existent object must be flagged as dangling"
  );

  assert!(
    is_null(get(wrong_obj, "resolved")),
    "dangling ref must have resolved == null"
  );
}

#[test]
fn v0_4_resolver_wrong_kind_is_kind_mismatch() {
  // Invariant 78.
  let value = eval_file(&fixture_path()).unwrap();
  let wrong_kind = get_path(&value, &["ref-resolution-cases", "wrong-kind"]);

  let status = as_str(get(wrong_kind, "status"));
  assert_eq!(
    status, "kind-mismatch",
    "ref with wrong kind on a real object must be flagged as kind-mismatch"
  );

  assert!(
    is_null(get(wrong_kind, "resolved")),
    "kind-mismatch ref must have resolved == null"
  );
}

#[test]
fn v0_4_repair_effect_carries_instance_qualified_resolves_ref_and_preserves_asymmetry() {
  // Invariant 79.
  let value = eval_file(&fixture_path()).unwrap();

  let effects1 = as_list(get_path(
    &value,
    &["computed-cross-object-repair-effect", "arm_link_1"],
  ));
  assert_eq!(
    effects1.len(),
    1,
    "arm_link_1 must have exactly 1 cross-object Repair-effect"
  );

  let effect = &effects1[0];
  let resolves_ref = get(effect, "resolves-held-instance-ref");
  assert!(
    is_attrset(resolves_ref),
    "Repair-effect's resolves-held-instance-ref must be a Value::AttrSet; got {:?}",
    resolves_ref
  );

  assert_eq!(
    as_str(get(resolves_ref, "object-id")),
    "arm_link_1",
    "resolves-held-instance-ref.object-id must point at arm_link_1's Held"
  );
  assert_eq!(
    as_str(get(resolves_ref, "held-kind")),
    "structural-binding-conflict",
    "resolves-held-instance-ref.held-kind must match arm_link_1's Held kind"
  );

  // arm_link_2 still has no downstream Repair-effect (asymmetry
  // preserved from v0.3).
  let effects2 = as_list(get_path(
    &value,
    &["computed-cross-object-repair-effect", "arm_link_2"],
  ));
  assert!(
    effects2.is_empty(),
    "arm_link_2 must have no cross-object Repair-effects (asymmetry preserved); got {:?}",
    effects2
  );
}

#[test]
fn v0_4_without_owner_law_ref_collapses_to_null_and_resolver_returns_non_resolved() {
  // Invariant 80.
  let value = eval_file(&fixture_without_owner_law_path()).unwrap();

  match get(&value, "owner-law-loaded") {
    Value::Bool(b) => assert!(!*b),
    other => panic!("owner-law-loaded must be a Bool, got {:?}", other),
  }

  // upstream-held-ref must collapse to null on every cross-
  // object Need entry.
  let needs2 = as_list(get_path(
    &value,
    &["computed-cross-object-needs-per-object", "arm_link_2"],
  ));
  assert_eq!(needs2.len(), 1);
  let need = &needs2[0];
  assert!(
    is_null(get(need, "upstream-held-ref")),
    "without owner-law: upstream-held-ref must be null; got {:?}",
    get(need, "upstream-held-ref")
  );
  match get(need, "blocking") {
    Value::Bool(b) => assert!(
      !*b,
      "without owner-law: cross-object Need must be non-blocking"
    ),
    other => panic!("blocking must be a Bool, got {:?}", other),
  }

  // Resolver behaviour with empty held map.
  let well = get_path(&value, &["ref-resolution-cases", "well-formed"]);
  assert_eq!(
    as_str(get(well, "status")),
    "dangling-no-such-object",
    "without owner-law: even a well-formed ref dangles because held map is empty"
  );
  assert!(is_null(get(well, "resolved")));

  let kind_only = get_path(&value, &["ref-resolution-cases", "kind-only"]);
  assert_eq!(
    as_str(get(kind_only, "status")),
    "ambiguous-kind-only",
    "kind-only ref ambiguity is shape-only; not affected by absent owner-law"
  );
  assert!(is_null(get(kind_only, "resolved")));

  let wrong_obj = get_path(&value, &["ref-resolution-cases", "wrong-object"]);
  assert_eq!(as_str(get(wrong_obj, "status")), "dangling-no-such-object");
  assert!(is_null(get(wrong_obj, "resolved")));

  let wrong_kind = get_path(&value, &["ref-resolution-cases", "wrong-kind"]);
  assert_eq!(
    as_str(get(wrong_kind, "status")),
    "dangling-no-such-object",
    "without owner-law: wrong-kind degrades to dangling because object-id is not in empty held map"
  );
  assert!(is_null(get(wrong_kind, "resolved")));
}

#[test]
fn v0_4_owner_law_file_unchanged_seven_rule_surface() {
  // Invariant 81. v0.4a must NOT have edited v0_owner_law.px.
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
        "v0_owner_law.px must still expose `{}` after v0.4a; available: {:?}",
        rule,
        attrs.keys().collect::<Vec<_>>()
      )
    });
    assert!(
      matches!(entry, Value::Lambda { .. }),
      "v0_owner_law.px:`{}` must still be a Lambda after v0.4a, got {:?}",
      rule,
      entry
    );
  }

  let allowed: std::collections::BTreeSet<&str> = required_rules.iter().copied().collect();
  for key in attrs.keys() {
    assert!(
      allowed.contains(key.as_str()),
      "v0.4a must NOT add new owner-law rules; v0_owner_law.px carries unexpected key `{}`",
      key
    );
  }
}
