//! v0.3 cross-object Held/Need dependency slice — first causal
//! coupling proof. Layers fixture-local cross-object Need /
//! Repair-effect surfaces on top of the v0.2 evaluated trajectory
//! (imported by `v0_3_run_cross_object_dependency.px` from
//! `v0_2_run_set_aware.px`). Asymmetry of the resulting graph is
//! the load-bearing v0.3 invariant.
//!
//! Truth owner:        project-wiki/maps/minimal-tesseract-v0-map.md
//!                     §"v0.3 design decision — cross-object Held/Need
//!                       dependency (fixture-local, post-trajectory derivation)"
//! Active scope:       project-wiki/maps/active-domain-constitution.md Art. 8
//!
//! What this test asserts (13 invariants, indices continued from
//! the v0.2 test's 55):
//!  56. world-set has 2 SourceObjects [arm_link_1, arm_link_2].
//!  57. relations field has exactly 1 entry: arm_link_2
//!      depends-on-frame arm_link_1.
//!  58. v0.2 trajectory surfaces still present and forwarded
//!      verbatim (computed-turns has 6 turns; per-object Held
//!      / Repair maps have 2 entries each).
//!  59. computed-cross-object-needs-per-object[arm_link_2] has
//!      exactly 1 entry: from=arm_link_2, to=arm_link_1,
//!      kind=depends-on-frame, blocking=true, upstream-held-ref
//!      matches actual id of computed-held-per-object[arm_link_1].
//!  60. computed-cross-object-needs-per-object[arm_link_1] is
//!      empty (no upstream).
//!  61. computed-cross-object-repair-effect[arm_link_1] has
//!      exactly 1 entry pointing at the cross-object Need on
//!      arm_link_2.
//!  62. computed-cross-object-repair-effect[arm_link_2] is empty
//!      (no downstream depends on arm_link_2).
//!  63. ASYMMETRY (load-bearing): need-emptiness and effect-
//!      emptiness flip between the two objects.
//!  64. without-owner-law: cross-object Needs are non-blocking
//!      (upstream-held-ref == null, blocking == false);
//!      cross-object Repair-effect maps are empty.
//!  65. v0_owner_law.px STILL exposes exactly the same 7 Lambda
//!      rules.
//!  66. promotion stays `pending` for both per-object Repairs;
//!      cross-object Repair-effect entries carry `applied=false`.
//!  67. coherent dependency graph: every cross-object Need's
//!      upstream-held-ref (when non-null) is a real Held id in
//!      computed-held-per-object — no dangling references.
//!  68. **REFINED contamination v0.3**: per-object Held / Repair
//!      bodies must NOT mention the other id (v0.2 rule);
//!      cross-object Need / Repair-effect surfaces MAY mention
//!      both ids (that is precisely their purpose).

use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------
// Helpers — duplicated from v0 / v0.1 / v0.2 tests because
// integration test files compile as separate crates. Duplication
// is small and keeps the v0.3 test independently auditable.
// ---------------------------------------------------------------

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../../fixtures/minimal-tesseract-v0/v0_3_run_cross_object_dependency.px")
}

fn fixture_without_owner_law_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../../fixtures/minimal-tesseract-v0/v0_3_run_without_owner_law.px")
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

// ---------------------------------------------------------------
// Tests.
// ---------------------------------------------------------------

#[test]
fn v0_3_world_set_two_objects_one_relation() {
  // Invariants 56 + 57.
  let value = eval_file(&fixture_path()).expect("v0.3 cross-object harness must evaluate");

  let world_set = as_list(get(&value, "world-set"));
  assert_eq!(world_set.len(), 2, "v0.3 world-set must have 2 objects");
  let ids: Vec<&str> = world_set.iter().map(|o| as_str(get(o, "id"))).collect();
  assert_eq!(ids, vec!["arm_link_1", "arm_link_2"]);

  let relations = as_list(get(&value, "relations"));
  assert_eq!(
    relations.len(),
    1,
    "v0.3 must declare exactly 1 cross-object relation; got {}",
    relations.len()
  );
  let r = &relations[0];
  assert_eq!(as_str(get(r, "from")), "arm_link_2");
  assert_eq!(as_str(get(r, "to")), "arm_link_1");
  assert_eq!(as_str(get(r, "kind")), "depends-on-frame");

  // Markers.
  match get(&value, "v0-3") {
    Value::Bool(b) => assert!(*b, "v0.3 marker must be true"),
    other => panic!("v0-3 must be a Bool, got {:?}", other),
  }
  match get(&value, "cross-object-aware") {
    Value::Bool(b) => assert!(*b, "cross-object-aware marker must be true"),
    other => panic!("cross-object-aware must be a Bool, got {:?}", other),
  }
}

#[test]
fn v0_3_v0_2_trajectory_forwarded_unchanged() {
  // Invariant 58 — the v0.2 surfaces flow through v0.3 verbatim.
  let value = eval_file(&fixture_path()).unwrap();

  // 6 turns 0..5 — same as v0.2.
  let turns = as_list(get(&value, "computed-turns"));
  assert_eq!(turns.len(), 6, "v0.3 must forward v0.2's 6 turns");
  let turn_ids: Vec<i64> = turns.iter().map(|t| as_int(get(t, "turn-id"))).collect();
  assert_eq!(turn_ids, vec![0, 1, 2, 3, 4, 5]);

  // Per-object Held / Repair maps each have 2 entries (v0.2 shape).
  let held_map = as_attrs(get(&value, "computed-held-per-object"));
  assert_eq!(held_map.len(), 2);
  assert!(held_map.contains_key("arm_link_1"));
  assert!(held_map.contains_key("arm_link_2"));

  let repair_map = as_attrs(get(&value, "computed-repair-per-object"));
  assert_eq!(repair_map.len(), 2);
  assert!(repair_map.contains_key("arm_link_1"));
  assert!(repair_map.contains_key("arm_link_2"));
}

#[test]
fn v0_3_arm_link_2_has_blocking_cross_object_need_to_arm_link_1() {
  // Invariant 59.
  let value = eval_file(&fixture_path()).unwrap();
  let needs2 = as_list(get_path(
    &value,
    &["computed-cross-object-needs-per-object", "arm_link_2"],
  ));
  assert_eq!(
    needs2.len(),
    1,
    "arm_link_2 must have exactly 1 cross-object Need (its dependency on arm_link_1)"
  );

  let need = &needs2[0];
  assert_eq!(as_str(get(need, "from")), "arm_link_2");
  assert_eq!(as_str(get(need, "to")), "arm_link_1");
  assert_eq!(as_str(get(need, "kind")), "depends-on-frame");
  match get(need, "blocking") {
    Value::Bool(b) => assert!(
      *b,
      "arm_link_2's cross-object Need must be blocking (upstream arm_link_1 has Held)"
    ),
    other => panic!("blocking must be a Bool, got {:?}", other),
  }
  // Coherence: upstream-held-ref must equal arm_link_1's actual
  // Held id (test 67's per-need form).
  let upstream_held_ref = as_str(get(need, "upstream-held-ref"));
  let actual_held_id = as_str(get_path(
    &value,
    &["computed-held-per-object", "arm_link_1", "id"],
  ));
  assert_eq!(
    upstream_held_ref, actual_held_id,
    "cross-object Need's upstream-held-ref must match the actual Held id of arm_link_1; \
     ref = `{}`, actual = `{}`",
    upstream_held_ref, actual_held_id
  );
}

#[test]
fn v0_3_arm_link_1_has_no_cross_object_needs() {
  // Invariant 60.
  let value = eval_file(&fixture_path()).unwrap();
  let needs1 = as_list(get_path(
    &value,
    &["computed-cross-object-needs-per-object", "arm_link_1"],
  ));
  assert!(
    needs1.is_empty(),
    "arm_link_1 must have no cross-object Needs (no upstream); got {:?}",
    needs1
  );
}

#[test]
fn v0_3_arm_link_1_has_one_downstream_repair_effect() {
  // Invariant 61.
  let value = eval_file(&fixture_path()).unwrap();
  let effects1 = as_list(get_path(
    &value,
    &["computed-cross-object-repair-effect", "arm_link_1"],
  ));
  assert_eq!(
    effects1.len(),
    1,
    "arm_link_1 must have exactly 1 cross-object Repair-effect (its repair would unblock arm_link_2)"
  );

  let effect = &effects1[0];
  assert_eq!(as_str(get(effect, "upstream-object")), "arm_link_1");
  assert_eq!(as_str(get(effect, "downstream-object")), "arm_link_2");
  assert_eq!(
    as_str(get(effect, "upstream-repair-id")),
    "repair.role-binding"
  );
  match get(effect, "applied") {
    Value::Bool(b) => assert!(
      !*b,
      "v0.3 cross-object Repair-effect must NOT be applied (auto-apply forbidden)"
    ),
    other => panic!("applied must be a Bool, got {:?}", other),
  }
  match get(effect, "would-unblock-if-promoted") {
    Value::Bool(b) => assert!(
      *b,
      "cross-object Repair-effect must record that promotion would unblock the downstream Need"
    ),
    other => panic!("would-unblock-if-promoted must be a Bool, got {:?}", other),
  }
}

#[test]
fn v0_3_arm_link_2_has_no_downstream_repair_effects() {
  // Invariant 62.
  let value = eval_file(&fixture_path()).unwrap();
  let effects2 = as_list(get_path(
    &value,
    &["computed-cross-object-repair-effect", "arm_link_2"],
  ));
  assert!(
    effects2.is_empty(),
    "arm_link_2 must have no cross-object Repair-effects (no downstream depends on it); got {:?}",
    effects2
  );
}

#[test]
fn v0_3_dependency_graph_is_asymmetric() {
  // Invariant 63 — load-bearing v0.3 invariant. The combined
  // assertion that needs ⊕ effects flip between the two objects.
  // If a bug walked relations in both directions, both sides
  // would be non-empty and this test would catch it.
  let value = eval_file(&fixture_path()).unwrap();
  let needs1 = as_list(get_path(
    &value,
    &["computed-cross-object-needs-per-object", "arm_link_1"],
  ));
  let needs2 = as_list(get_path(
    &value,
    &["computed-cross-object-needs-per-object", "arm_link_2"],
  ));
  let effects1 = as_list(get_path(
    &value,
    &["computed-cross-object-repair-effect", "arm_link_1"],
  ));
  let effects2 = as_list(get_path(
    &value,
    &["computed-cross-object-repair-effect", "arm_link_2"],
  ));

  assert!(needs1.is_empty(), "arm_link_1 needs must be empty");
  assert!(!needs2.is_empty(), "arm_link_2 needs must be non-empty");
  assert!(!effects1.is_empty(), "arm_link_1 effects must be non-empty");
  assert!(effects2.is_empty(), "arm_link_2 effects must be empty");

  // Concretely the asymmetry pattern: "upstream has effects but
  // no needs; downstream has needs but no effects". Verify by
  // structural shape.
  assert_eq!(
    (needs1.len(), effects1.len()),
    (0, 1),
    "arm_link_1 (upstream) must have (needs=0, effects=1)"
  );
  assert_eq!(
    (needs2.len(), effects2.len()),
    (1, 0),
    "arm_link_2 (downstream) must have (needs=1, effects=0)"
  );
}

#[test]
fn v0_3_without_owner_law_cross_object_needs_non_blocking() {
  // Invariant 64.
  let value = eval_file(&fixture_without_owner_law_path()).unwrap();

  match get(&value, "owner-law-loaded") {
    Value::Bool(b) => assert!(!*b),
    other => panic!("owner-law-loaded must be a Bool, got {:?}", other),
  }

  // Cross-object Need entries: arm_link_2 still has 1 entry
  // (the relation is declared in inputs regardless of owner-law),
  // but it's non-blocking with null upstream-held-ref.
  let needs2 = as_list(get_path(
    &value,
    &["computed-cross-object-needs-per-object", "arm_link_2"],
  ));
  assert_eq!(
    needs2.len(),
    1,
    "without owner-law: relation declaration still produces a Need entry, just non-blocking"
  );
  let need = &needs2[0];
  match get(need, "blocking") {
    Value::Bool(b) => assert!(
      !*b,
      "without owner-law: cross-object Need must be non-blocking (upstream has no Held)"
    ),
    other => panic!("blocking must be a Bool, got {:?}", other),
  }
  assert!(
    is_null(get(need, "upstream-held-ref")),
    "without owner-law: cross-object Need's upstream-held-ref must be null"
  );

  // Cross-object Repair-effect maps must be empty on both sides
  // (no Repair candidates exist to have an effect).
  let effects1 = as_list(get_path(
    &value,
    &["computed-cross-object-repair-effect", "arm_link_1"],
  ));
  let effects2 = as_list(get_path(
    &value,
    &["computed-cross-object-repair-effect", "arm_link_2"],
  ));
  assert!(
    effects1.is_empty(),
    "without owner-law: no cross-object effects on arm_link_1"
  );
  assert!(
    effects2.is_empty(),
    "without owner-law: no cross-object effects on arm_link_2"
  );
}

#[test]
fn v0_3_owner_law_file_unchanged_seven_rule_surface() {
  // Invariant 65. v0.3 must NOT have edited v0_owner_law.px.
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
        "v0_owner_law.px must still expose `{}` after v0.3; available: {:?}",
        rule,
        attrs.keys().collect::<Vec<_>>()
      )
    });
    assert!(
      matches!(entry, Value::Lambda { .. }),
      "v0_owner_law.px:`{}` must still be a Lambda after v0.3, got {:?}",
      rule,
      entry
    );
  }

  let allowed: std::collections::BTreeSet<&str> = required_rules.iter().copied().collect();
  for key in attrs.keys() {
    assert!(
      allowed.contains(key.as_str()),
      "v0.3 must NOT add new owner-law rules; v0_owner_law.px carries unexpected key `{}`",
      key
    );
  }
}

#[test]
fn v0_3_repair_promotion_stays_pending_and_effects_unapplied() {
  // Invariant 66 — v0's no-auto-apply invariant transfers per
  // object AND per cross-object effect.
  let value = eval_file(&fixture_path()).unwrap();

  for object_id in ["arm_link_1", "arm_link_2"] {
    let promotion = as_str(get_path(
      &value,
      &["computed-repair-per-object", object_id, "promotion"],
    ));
    assert_eq!(
      promotion, "pending",
      "repair-per-object[{}].promotion must remain pending",
      object_id
    );
  }

  // Each cross-object Repair-effect entry must carry applied=false.
  let effect_map = as_attrs(get(&value, "computed-cross-object-repair-effect"));
  for (object_id, effects_value) in effect_map {
    for effect in as_list(effects_value) {
      match get(effect, "applied") {
        Value::Bool(b) => assert!(
          !*b,
          "cross-object Repair-effect on {} must carry applied=false",
          object_id
        ),
        other => panic!("applied must be a Bool, got {:?}", other),
      }
    }
  }
}

#[test]
fn v0_3_dependency_graph_is_coherent() {
  // Invariant 67 — every cross-object Need's upstream-held-ref
  // (when non-null) must be a real Held id present in
  // computed-held-per-object[upstream-object]. No dangling refs.
  let value = eval_file(&fixture_path()).unwrap();
  let needs_map = as_attrs(get(&value, "computed-cross-object-needs-per-object"));
  let held_map = as_attrs(get(&value, "computed-held-per-object"));

  for (_dependent_id, needs_value) in needs_map {
    for need in as_list(needs_value) {
      let upstream_obj = as_str(get(need, "upstream-object"));
      let upstream_held_ref = get(need, "upstream-held-ref");
      if !is_null(upstream_held_ref) {
        let ref_str = as_str(upstream_held_ref);
        let upstream_held_id = held_map
          .get(upstream_obj)
          .and_then(|h| match h {
            Value::AttrSet(m) => m.get("id").and_then(|v| match v {
              Value::String(s) => Some(s.as_str()),
              Value::StringContext { text, .. } => Some(text.as_str()),
              _ => None,
            }),
            _ => None,
          })
          .unwrap_or_else(|| {
            panic!(
              "cross-object Need points at upstream-object `{}` but no Held entry exists for that key",
              upstream_obj
            )
          });
        assert_eq!(
          ref_str, upstream_held_id,
          "cross-object Need's upstream-held-ref `{}` must match the actual Held id `{}` for object `{}`",
          ref_str, upstream_held_id, upstream_obj
        );
      }
    }
  }
}

#[test]
fn v0_3_refined_contamination_per_object_bodies_isolated_cross_object_surfaces_shared() {
  // Invariant 68 — REFINED contamination v0.3.
  //
  //   per-object Held bodies         : MUST NOT mention the other id
  //   per-object Repair bodies       : MUST NOT mention the other id
  //   cross-object Need entries      : MAY mention both ids (purpose)
  //   cross-object Repair-effect entries : MAY mention both ids (purpose)
  //
  // The split says "disjoint per-object surfaces stay isolated;
  // surfaces that explicitly couple objects can mention both
  // ids". v0.1's pure-isolation rule no longer applies because
  // cross-object surfaces *exist to couple*.
  for (label, path) in [
    ("with-owner-law", fixture_path()),
    ("without-owner-law", fixture_without_owner_law_path()),
  ] {
    let value = eval_file(&path).expect("harness must evaluate");

    // Per-object Held bodies — isolation rule (v0.2 carries over).
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
        "[{}] per-object Held body for {} must NOT mention {}; got JSON:\n{}",
        label,
        object_id,
        other_id,
        json
      );
    }

    // Per-object Repair bodies — isolation rule.
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
        "[{}] per-object Repair body for {} must NOT mention {}; got JSON:\n{}",
        label,
        object_id,
        other_id,
        json
      );
    }

    // Cross-object Need entries — coupling rule. Each Need
    // entry must mention BOTH object ids (its `from` and `to`)
    // because that is exactly what makes it a cross-object
    // surface. The "non-blocking" form (without-owner-law)
    // still carries the relation metadata, so the same rule
    // applies on both sides.
    let needs_map = as_attrs(get(&value, "computed-cross-object-needs-per-object"));
    for (_dependent_id, needs_value) in needs_map {
      for need in as_list(needs_value) {
        let json = need.to_json();
        // Each cross-object Need has a from + to pair.
        let from = as_str(get(need, "from"));
        let to = as_str(get(need, "to"));
        assert!(
          json.contains(from),
          "[{}] cross-object Need must mention its own `from` id `{}`; got JSON:\n{}",
          label,
          from,
          json
        );
        assert!(
          json.contains(to),
          "[{}] cross-object Need must mention its `to` id `{}` (that is the upstream coupling); got JSON:\n{}",
          label, to, json
        );
      }
    }

    // Cross-object Repair-effect entries — same coupling rule.
    let effect_map = as_attrs(get(&value, "computed-cross-object-repair-effect"));
    for (_upstream_id, effects_value) in effect_map {
      for effect in as_list(effects_value) {
        let json = effect.to_json();
        let upstream = as_str(get(effect, "upstream-object"));
        let downstream = as_str(get(effect, "downstream-object"));
        assert!(
          json.contains(upstream),
          "[{}] cross-object Repair-effect must mention upstream `{}`; got JSON:\n{}",
          label,
          upstream,
          json
        );
        assert!(
          json.contains(downstream),
          "[{}] cross-object Repair-effect must mention downstream `{}`; got JSON:\n{}",
          label,
          downstream,
          json
        );
      }
    }
  }
}
