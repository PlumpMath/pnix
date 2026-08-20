//! v0.5 promotion / reopen slice — promotion signal propagates
//! through v0.4c-shape Need / repair-effect / chain surfaces by
//! flipping `applied`, `blocking` (+ `status`), and
//! `upstream-promoted` fields. The v0.2 Held / Repair trace
//! stays byte-for-byte unchanged — promotion is signal-only,
//! NOT a physical repair apply. Test 117 (immediate-downstream
//! unblock) and test 118 (transitive non-unblock) and test 119
//! (chain depth-2 visibility) are the load-bearing v0.5
//! invariants.
//!
//! Truth owner:        project-wiki/maps/minimal-tesseract-v0-map.md
//!                     §"v0.5 design decision — promotion / reopen path
//!                       (signal-only; no physical repair apply)"
//! Active scope:       project-wiki/maps/active-domain-constitution.md Art. 8
//!
//! What this test asserts (15 invariants, indices continued
//! from the v0.4c test's 112):
//! 113. world-set / relations identical to v0.4c (3 objects,
//!      2 relations [depends-on-frame, mounted-on] in declared
//!      order); v0-5 / promotion-aware / plural-kind-aware
//!      markers true; promoted-repairs has arm_link_1 mapped
//!      to the role-binding repair-id list.
//! 114. without-promotion: every repair-effect entry's
//!      `applied = false`; every Need with upstream-Held has
//!      `blocking = true` and `status = "blocked"`; every
//!      chain step's `upstream-promoted = false`.
//! 115. with-promotion: arm_link_1's repair-effect entry
//!      toward arm_link_2 has `applied = true`,
//!      `relation-kind = "mounted-on"`, and
//!      `applied-by-repair-ids` carries the promoted
//!      repair-id list.
//! 116. with-promotion: arm_link_2's repair-effect entry
//!      toward arm_link_3 has `applied = false`
//!      (only arm_link_1 promoted in this slice).
//! 117. **load-bearing — immediate-downstream unblock**:
//!      arm_link_2's Need toward arm_link_1 has
//!      `blocking = false` and
//!      `status = "reopened-by-upstream-promotion"`.
//! 118. **load-bearing — transitive non-unblock**:
//!      arm_link_3's Need toward arm_link_2 has
//!      `blocking = true` and `status = "blocked"`.
//!      Promotion is one-step.
//! 119. **load-bearing — chain depth-2 visibility**:
//!      arm_link_3's chain step at index 1 (object-id =
//!      arm_link_1) has `upstream-promoted = true`; chain
//!      step at index 0 (object-id = arm_link_2) has
//!      `upstream-promoted = false`.
//! 120. arm_link_2's chain step at index 0 (object-id =
//!      arm_link_1) has `upstream-promoted = true`. Chain
//!      root arm_link_1 has empty chain.
//! 121. Chain step shape is exactly { object-id;
//!      relation-kind; has-held; held-instance-ref;
//!      upstream-promoted; } — five fields, no extras.
//! 122. Need shape gains exactly `status` beyond v0.4c. No
//!      physical-repair fields are added to the Need
//!      (nothing named `applied`, `attached-fact`,
//!      `committed`, `world-set-mutation`, etc.).
//! 123. with-promotion: `computed-held-per-object` and
//!      `computed-repair-per-object` are byte-for-byte
//!      identical to without-promotion's (= v0.2's) — the
//!      v0.2 trace is NOT mutated by promotion.
//! 124. with-promotion: relation-kind sequence in the chain
//!      remains [depends-on-frame, mounted-on] —
//!      promotion does not change relation-kind plurality.
//! 125. with-promotion: resolver still resolves both
//!      well-formed refs with status "resolved" — resolver
//!      is promotion-independent.
//! 126. with-promotion vs without-promotion: chain step
//!      object-id and relation-kind sequences are identical;
//!      only `upstream-promoted` flips for the depth where
//!      the promoted upstream is visited.
//! 127. v0_owner_law.px STILL exposes exactly the same 7
//!      Lambda rules.

use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------
// Helpers — duplicated from prior v0..v0.4c tests because
// integration test files compile as separate crates.
// ---------------------------------------------------------------

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../../fixtures/minimal-tesseract-v0/v0_5_run_promotion_reopen.px")
}

fn fixture_without_promotion_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../../fixtures/minimal-tesseract-v0/v0_5_run_without_promotion.px")
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

// ---------------------------------------------------------------
// Tests.
// ---------------------------------------------------------------

#[test]
fn v0_5_world_set_relations_promoted_repairs_marker() {
  // Invariant 113.
  let value = eval_file(&fixture_path()).expect("v0.5 with-promotion harness must evaluate");

  let world_set = as_list(get(&value, "world-set"));
  assert_eq!(world_set.len(), 3);
  let ids: Vec<&str> = world_set.iter().map(|o| as_str(get(o, "id"))).collect();
  assert_eq!(ids, vec!["arm_link_1", "arm_link_2", "arm_link_3"]);

  let relations = as_list(get(&value, "relations"));
  assert_eq!(relations.len(), 2);
  let kinds: Vec<&str> = relations.iter().map(|r| as_str(get(r, "kind"))).collect();
  assert_eq!(kinds, vec!["depends-on-frame", "mounted-on"]);

  assert!(as_bool(get(&value, "v0-5")));
  assert!(as_bool(get(&value, "promotion-aware")));
  assert!(as_bool(get(&value, "chain-aware")));
  assert!(as_bool(get(&value, "plural-kind-aware")));

  let promoted = as_attrs(get(&value, "promoted-repairs"));
  assert_eq!(
    promoted.len(),
    1,
    "v0.5 promoted-repairs has exactly one promoted upstream"
  );
  let arm_link_1_promoted = as_list(
    promoted
      .get("arm_link_1")
      .expect("arm_link_1 must be promoted"),
  );
  assert_eq!(arm_link_1_promoted.len(), 1);
  assert_eq!(
    as_str(&arm_link_1_promoted[0]),
    "structural-binding-conflict.role-binding"
  );
}

#[test]
fn v0_5_without_promotion_applied_false_blocking_true_no_upstream_promoted() {
  // Invariant 114.
  let value = eval_file(&fixture_without_promotion_path()).unwrap();

  // promoted-repairs is empty.
  assert!(as_attrs(get(&value, "promoted-repairs")).is_empty());

  // Every repair-effect entry has applied=false.
  let repair_effect_map = as_attrs(get(&value, "computed-cross-object-repair-effect"));
  for (_upstream, entries_value) in repair_effect_map {
    for entry in as_list(entries_value) {
      assert!(
        !as_bool(get(entry, "applied")),
        "without-promotion: every repair-effect entry must have applied=false"
      );
      let applied_by = as_list(get(entry, "applied-by-repair-ids"));
      assert!(
        applied_by.is_empty(),
        "without-promotion: applied-by-repair-ids must be empty list"
      );
    }
  }

  // arm_link_2's Need toward arm_link_1: blocking=true, status=blocked.
  let need_2 = &as_list(get_path(
    &value,
    &["computed-cross-object-needs-per-object", "arm_link_2"],
  ))[0];
  assert!(as_bool(get(need_2, "blocking")));
  assert_eq!(as_str(get(need_2, "status")), "blocked");

  // arm_link_3's Need toward arm_link_2: blocking=true, status=blocked.
  let need_3 = &as_list(get_path(
    &value,
    &["computed-cross-object-needs-per-object", "arm_link_3"],
  ))[0];
  assert!(as_bool(get(need_3, "blocking")));
  assert_eq!(as_str(get(need_3, "status")), "blocked");

  // Every chain step has upstream-promoted=false.
  let chain_map = as_attrs(get(&value, "transitive-chain-per-object"));
  for (_id, chain_value) in chain_map {
    for step in as_list(chain_value) {
      assert!(
        !as_bool(get(step, "upstream-promoted")),
        "without-promotion: every chain step must have upstream-promoted=false"
      );
    }
  }
}

#[test]
fn v0_5_with_promotion_arm_link_1_repair_effect_applied_true() {
  // Invariant 115.
  let value = eval_file(&fixture_path()).unwrap();
  let entries = as_list(get_path(
    &value,
    &["computed-cross-object-repair-effect", "arm_link_1"],
  ));
  assert_eq!(entries.len(), 1);
  let entry = &entries[0];
  assert!(
    as_bool(get(entry, "applied")),
    "arm_link_1's repair-effect entry must have applied=true after promotion"
  );
  assert_eq!(as_str(get(entry, "relation-kind")), "mounted-on");
  assert_eq!(as_str(get(entry, "downstream-object")), "arm_link_2");

  let applied_by = as_list(get(entry, "applied-by-repair-ids"));
  assert_eq!(applied_by.len(), 1);
  assert_eq!(
    as_str(&applied_by[0]),
    "structural-binding-conflict.role-binding"
  );
}

#[test]
fn v0_5_with_promotion_arm_link_2_repair_effect_still_applied_false() {
  // Invariant 116.
  let value = eval_file(&fixture_path()).unwrap();
  let entries = as_list(get_path(
    &value,
    &["computed-cross-object-repair-effect", "arm_link_2"],
  ));
  assert_eq!(entries.len(), 1);
  let entry = &entries[0];
  assert!(
    !as_bool(get(entry, "applied")),
    "arm_link_2's repair-effect must stay applied=false (only arm_link_1 promoted)"
  );
  assert_eq!(as_str(get(entry, "relation-kind")), "depends-on-frame");
  assert_eq!(as_str(get(entry, "downstream-object")), "arm_link_3");

  let applied_by = as_list(get(entry, "applied-by-repair-ids"));
  assert!(
    applied_by.is_empty(),
    "non-promoted upstream must carry empty applied-by-repair-ids"
  );
}

#[test]
fn v0_5_with_promotion_arm_link_2_need_unblocked_reopened() {
  // Invariant 117 — load-bearing immediate-downstream unblock.
  let value = eval_file(&fixture_path()).unwrap();
  let needs = as_list(get_path(
    &value,
    &["computed-cross-object-needs-per-object", "arm_link_2"],
  ));
  assert_eq!(needs.len(), 1);
  let need = &needs[0];
  assert_eq!(as_str(get(need, "to")), "arm_link_1");
  assert!(
    !as_bool(get(need, "blocking")),
    "arm_link_2's Need toward promoted arm_link_1 must unblock"
  );
  assert_eq!(
    as_str(get(need, "status")),
    "reopened-by-upstream-promotion",
    "arm_link_2's Need status must be reopened-by-upstream-promotion"
  );
}

#[test]
fn v0_5_with_promotion_arm_link_3_need_still_blocked() {
  // Invariant 118 — load-bearing transitive non-unblock.
  let value = eval_file(&fixture_path()).unwrap();
  let needs = as_list(get_path(
    &value,
    &["computed-cross-object-needs-per-object", "arm_link_3"],
  ));
  assert_eq!(needs.len(), 1);
  let need = &needs[0];
  assert_eq!(as_str(get(need, "to")), "arm_link_2");
  assert!(
    as_bool(get(need, "blocking")),
    "arm_link_3's Need stays blocked: arm_link_2 has not promoted (transitive unblock not free)"
  );
  assert_eq!(as_str(get(need, "status")), "blocked");
}

#[test]
fn v0_5_with_promotion_chain_depth_2_visibility() {
  // Invariant 119 — load-bearing chain depth-2 visibility.
  let value = eval_file(&fixture_path()).unwrap();
  let chain3 = as_list(get_path(
    &value,
    &["transitive-chain-per-object", "arm_link_3"],
  ));
  assert_eq!(chain3.len(), 2);

  let step0 = &chain3[0];
  assert_eq!(as_str(get(step0, "object-id")), "arm_link_2");
  assert!(
    !as_bool(get(step0, "upstream-promoted")),
    "chain step at depth 1 (arm_link_2) must NOT be marked promoted"
  );

  let step1 = &chain3[1];
  assert_eq!(as_str(get(step1, "object-id")), "arm_link_1");
  assert!(
    as_bool(get(step1, "upstream-promoted")),
    "chain step at depth 2 (arm_link_1) must be marked promoted (chain sees promotion at depth 2)"
  );
}

#[test]
fn v0_5_with_promotion_arm_link_2_chain_depth_1_promoted() {
  // Invariant 120.
  let value = eval_file(&fixture_path()).unwrap();
  let chain2 = as_list(get_path(
    &value,
    &["transitive-chain-per-object", "arm_link_2"],
  ));
  assert_eq!(chain2.len(), 1);
  let step = &chain2[0];
  assert_eq!(as_str(get(step, "object-id")), "arm_link_1");
  assert!(as_bool(get(step, "upstream-promoted")));

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
fn v0_5_chain_step_shape_is_exactly_five_fields() {
  // Invariant 121.
  let value = eval_file(&fixture_path()).unwrap();
  let chain_map = as_attrs(get(&value, "transitive-chain-per-object"));
  let expected: std::collections::BTreeSet<&str> = [
    "object-id",
    "relation-kind",
    "has-held",
    "held-instance-ref",
    "upstream-promoted",
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
fn v0_5_need_shape_no_physical_repair_fields() {
  // Invariant 122. Need gains `status` beyond v0.4c. It must
  // NOT carry any physical-repair field.
  let value = eval_file(&fixture_path()).unwrap();
  let needs_map = as_attrs(get(&value, "computed-cross-object-needs-per-object"));

  let forbidden = [
    "applied",
    "attached-fact",
    "committed",
    "world-set-mutation",
    "physical-repair",
    "post-repair-state",
  ];

  for (_obj, needs_value) in needs_map {
    for need in as_list(needs_value) {
      let attrs = as_attrs(need);
      assert!(
        attrs.contains_key("status"),
        "v0.5 Need must carry `status` field (added beyond v0.4c)"
      );
      for f in forbidden {
        assert!(
          !attrs.contains_key(f),
          "v0.5 Need must NOT contain physical-repair field `{}` (promotion is signal-only)",
          f
        );
      }
    }
  }
}

#[test]
fn v0_5_v0_2_trace_unchanged_under_promotion() {
  // Invariant 123. Promotion is signal-only: with-promotion
  // and without-promotion must agree on computed-held-per-
  // object and computed-repair-per-object byte-for-byte.
  let with_p = eval_file(&fixture_path()).unwrap();
  let without_p = eval_file(&fixture_without_promotion_path()).unwrap();

  assert_eq!(
    get(&with_p, "computed-held-per-object").to_json(),
    get(&without_p, "computed-held-per-object").to_json(),
    "computed-held-per-object must be byte-for-byte identical with vs without promotion"
  );
  assert_eq!(
    get(&with_p, "computed-repair-per-object").to_json(),
    get(&without_p, "computed-repair-per-object").to_json(),
    "computed-repair-per-object must be byte-for-byte identical with vs without promotion"
  );
  // Also computed-turns must be identical.
  assert_eq!(
    get(&with_p, "computed-turns").to_json(),
    get(&without_p, "computed-turns").to_json(),
    "computed-turns must be identical (no promotion-driven turn injection)"
  );
}

#[test]
fn v0_5_relation_kind_sequence_unchanged_under_promotion() {
  // Invariant 124.
  let value = eval_file(&fixture_path()).unwrap();
  let chain3 = as_list(get_path(
    &value,
    &["transitive-chain-per-object", "arm_link_3"],
  ));
  let kinds: Vec<&str> = chain3
    .iter()
    .map(|s| as_str(get(s, "relation-kind")))
    .collect();
  assert_eq!(
    kinds,
    vec!["depends-on-frame", "mounted-on"],
    "relation-kind sequence must remain [depends-on-frame, mounted-on] under promotion"
  );
}

#[test]
fn v0_5_resolver_promotion_independent() {
  // Invariant 125.
  let value = eval_file(&fixture_path()).unwrap();
  let well_root = get_path(&value, &["ref-resolution-cases", "well-formed"]);
  assert_eq!(as_str(get(well_root, "status")), "resolved");
  let well_mid = get_path(&value, &["ref-resolution-cases", "well-formed-mid"]);
  assert_eq!(as_str(get(well_mid, "status")), "resolved");
  let kind_only = get_path(&value, &["ref-resolution-cases", "kind-only"]);
  assert_eq!(as_str(get(kind_only, "status")), "ambiguous-kind-only");
  let wrong_kind = get_path(&value, &["ref-resolution-cases", "wrong-kind"]);
  assert_eq!(as_str(get(wrong_kind, "status")), "kind-mismatch");
}

#[test]
fn v0_5_with_vs_without_chain_object_kind_identical_only_promoted_flips() {
  // Invariant 126. Walk arm_link_3's chain in both runners
  // and confirm object-id and relation-kind sequences are
  // identical; only `upstream-promoted` flips for the
  // depth where the promoted upstream is visited.
  let with_p = eval_file(&fixture_path()).unwrap();
  let without_p = eval_file(&fixture_without_promotion_path()).unwrap();

  let chain_with = as_list(get_path(
    &with_p,
    &["transitive-chain-per-object", "arm_link_3"],
  ));
  let chain_without = as_list(get_path(
    &without_p,
    &["transitive-chain-per-object", "arm_link_3"],
  ));
  assert_eq!(chain_with.len(), chain_without.len());

  for (i, (s_with, s_without)) in chain_with.iter().zip(chain_without.iter()).enumerate() {
    assert_eq!(
      as_str(get(s_with, "object-id")),
      as_str(get(s_without, "object-id")),
      "chain object-id at depth {} must match between with/without promotion",
      i
    );
    assert_eq!(
      as_str(get(s_with, "relation-kind")),
      as_str(get(s_without, "relation-kind")),
      "chain relation-kind at depth {} must match between with/without promotion",
      i
    );
    assert_eq!(
      as_bool(get(s_with, "has-held")),
      as_bool(get(s_without, "has-held")),
      "chain has-held at depth {} must match between with/without promotion",
      i
    );
  }

  // Step 1 (depth 2, object-id arm_link_1) flips promoted in
  // with-promotion only.
  assert!(
    !as_bool(get(&chain_without[1], "upstream-promoted")),
    "without-promotion: depth-2 step must NOT be marked promoted"
  );
  assert!(
    as_bool(get(&chain_with[1], "upstream-promoted")),
    "with-promotion: depth-2 step must be marked promoted"
  );
  // Step 0 (depth 1, object-id arm_link_2) is NOT marked
  // promoted in either runner because arm_link_2 was never
  // promoted.
  assert!(
    !as_bool(get(&chain_with[0], "upstream-promoted")),
    "with-promotion: depth-1 step (arm_link_2) must NOT be marked promoted"
  );
  assert!(
    !as_bool(get(&chain_without[0], "upstream-promoted")),
    "without-promotion: depth-1 step must NOT be marked promoted"
  );
}

#[test]
fn v0_5_owner_law_file_unchanged_seven_rule_surface() {
  // Invariant 127. v0.5 must NOT have edited v0_owner_law.px.
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
        "v0_owner_law.px must still expose `{}` after v0.5; available: {:?}",
        rule,
        attrs.keys().collect::<Vec<_>>()
      )
    });
    assert!(
      matches!(entry, Value::Lambda { .. }),
      "v0_owner_law.px:`{}` must still be a Lambda after v0.5, got {:?}",
      rule,
      entry
    );
  }

  let allowed: std::collections::BTreeSet<&str> = required_rules.iter().copied().collect();
  for key in attrs.keys() {
    assert!(
      allowed.contains(key.as_str()),
      "v0.5 must NOT add new owner-law rules; v0_owner_law.px carries unexpected key `{}`",
      key
    );
  }
}
