//! v0.1 multi-object slice — same 7-rule owner-law applied per
//! SourceObject (Option B′ from
//! `project-wiki/maps/minimal-tesseract-v0-map.md`
//! §"v0.1 design decision").
//!
//! Truth owner:        project-wiki/maps/minimal-tesseract-v0-map.md
//! Active scope:       project-wiki/maps/active-domain-constitution.md Art. 8
//!
//! What this test asserts (8 invariants, indices continued from
//! the v0 test's 35):
//!  36. `source-objects` length == 2 with ids
//!      [arm_link_1, arm_link_2].
//!  37. each `computed-traces.{arm_link_1,arm_link_2}` has 6 turns
//!      with ids 0..5.
//!  38. each trace's `computed-held.held-kind` ==
//!      "structural-binding-conflict".
//!  39. each trace's `computed-repair.id` == "repair.role-binding"
//!      with `promotion` == "pending".
//!  40. each trace's `computed-meta-circular-log-differential.
//!      after-turn-5.open-held` contains
//!      "held.structural-binding-conflict".
//!  41. without-owner-law: each trace has empty `computed-turns`,
//!      null `computed-{lens-compare,held,repair}`, and a
//!      no-change `computed-meta-circular-log-differential`
//!      (`after-turn-5 == initial-state`).
//!  42. **No cross-object contamination**: the JSON of
//!      `computed-traces.arm_link_1` contains no "arm_link_2"
//!      string; the JSON of `computed-traces.arm_link_2` contains
//!      no "arm_link_1" string. This is the *load-bearing*
//!      invariant of v0.1; the others are shape-sanity that flows
//!      from v0 invariants applied per object.
//!  43. `v0_owner_law.px` still exposes exactly the same 7
//!      `Value::Lambda` rules — v0.1 must NOT modify the
//!      owner-law surface.
//!
//! What this test deliberately does NOT do (per v0.1 design
//! decision in the map):
//!
//!   - It does NOT apply owner-law in Rust. Owner-law application
//!     happens inside `v0_1_run_multi_object.px`.
//!   - It does NOT introduce a new ontology record kind.
//!   - It does NOT add a CapabilityCard / NeedGraph / HeldGraph /
//!     BenchmarkGraph / RigorFloor registry.
//!   - It does NOT touch the v0 owner-law file, the v0 inputs
//!     file, the v0 runners, or the v0 Rust test. v0.1 is
//!     additions-only.
//!   - It does NOT exercise cross-object lens overlay semantics
//!     (Option A) or a plural HeldGraph carrier (Option C). Both
//!     are deferred to v0.2 / v0.3.

use pnix_eval::{eval_file, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------
// Helpers — duplicated from `eval_minimal_tesseract_v0.rs` because
// integration test files are compiled as separate crates and
// cannot share private helpers. The duplication is small and
// keeps the v0 and v0.1 tests cleanly independent.
// ---------------------------------------------------------------

fn fixture_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../../fixtures/minimal-tesseract-v0/v0_1_run_multi_object.px")
}

fn fixture_without_owner_law_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../../fixtures/minimal-tesseract-v0/v0_1_run_without_owner_law.px")
}

fn owner_law_file_path() -> PathBuf {
  // v0.1 reuses the v0 owner-law file by reference. Test 43
  // asserts it carries exactly the same 7-rule surface v0 closed
  // with — any v0.1-side change to that file would be a
  // protocol violation.
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

fn is_null(v: &Value) -> bool {
  matches!(v, Value::Null)
}

// ---------------------------------------------------------------
// Tests.
// ---------------------------------------------------------------

#[test]
fn v0_1_two_source_objects_present() {
  // Invariant 36: 2 SourceObjects with the canonical v0.1 ids.
  let value = eval_file(&fixture_path()).expect("v0.1 multi-object harness must evaluate");
  let objects = as_list(get(&value, "source-objects"));
  assert_eq!(
    objects.len(),
    2,
    "v0.1 must declare exactly 2 SourceObjects; got {}",
    objects.len()
  );
  let ids: Vec<&str> = objects.iter().map(|o| as_str(get(o, "id"))).collect();
  assert_eq!(
    ids,
    vec!["arm_link_1", "arm_link_2"],
    "v0.1 source-object ids must be [arm_link_1, arm_link_2]"
  );

  // The owner-law marker stays true and points at the v0 file
  // (v0.1 reuses it — no v0.1-only owner-law file exists).
  match get(&value, "owner-law-loaded") {
    Value::Bool(b) => assert!(
      *b,
      "v0.1 multi-object harness must report owner-law-loaded=true"
    ),
    other => panic!("owner-law-loaded must be a Bool, got {:?}", other),
  }
  assert_eq!(
    as_str(get(&value, "owner-law-source")),
    "fixtures/minimal-tesseract-v0/v0_owner_law.px",
    "v0.1 must reuse the v0 owner-law file by reference"
  );
}

#[test]
fn v0_1_each_object_produces_six_turns() {
  // Invariant 37: each per-object trace has 6 turns (ids 0..5).
  // The same v0 invariant, applied per object via Option B′.
  let value = eval_file(&fixture_path()).unwrap();
  let traces = get(&value, "computed-traces");

  for object_id in ["arm_link_1", "arm_link_2"] {
    let trace = get(traces, object_id);
    let turns = as_list(get(trace, "computed-turns"));
    assert_eq!(
      turns.len(),
      6,
      "computed-traces.{} must have 6 turns; got {}",
      object_id,
      turns.len()
    );
    let turn_ids: Vec<i64> = turns.iter().map(|t| as_int(get(t, "turn-id"))).collect();
    assert_eq!(
      turn_ids,
      vec![0, 1, 2, 3, 4, 5],
      "computed-traces.{} turn-id sequence must be [0,1,2,3,4,5]",
      object_id
    );
  }
}

#[test]
fn v0_1_each_object_produces_structural_binding_conflict_held() {
  // Invariant 38: each trace's Held kind is structural-binding-
  // conflict — same v0 conflict, surfaced per object.
  let value = eval_file(&fixture_path()).unwrap();
  let traces = get(&value, "computed-traces");

  for object_id in ["arm_link_1", "arm_link_2"] {
    let trace = get(traces, object_id);
    let held_kind = as_str(get_path(trace, &["computed-held", "held-kind"]));
    assert_eq!(
      held_kind, "structural-binding-conflict",
      "computed-traces.{}.computed-held.held-kind must be structural-binding-conflict",
      object_id
    );
    // Each Held instance points at its own SourceObject.
    let blocked_node = as_str(get_path(trace, &["computed-held", "blocked-node"]));
    assert_eq!(
      blocked_node, object_id,
      "computed-traces.{}.computed-held.blocked-node must point at the same object id",
      object_id
    );
  }
}

#[test]
fn v0_1_each_object_produces_repair_role_binding_pending() {
  // Invariant 39: each trace's RepairCandidate is repair.role-
  // binding with promotion=pending. v0 forbids auto-apply; v0.1
  // inherits that invariant per object.
  let value = eval_file(&fixture_path()).unwrap();
  let traces = get(&value, "computed-traces");

  for object_id in ["arm_link_1", "arm_link_2"] {
    let trace = get(traces, object_id);
    assert_eq!(
      as_str(get_path(trace, &["computed-repair", "id"])),
      "repair.role-binding",
      "computed-traces.{}.computed-repair.id must be repair.role-binding",
      object_id
    );
    assert_eq!(
      as_str(get_path(trace, &["computed-repair", "promotion"])),
      "pending",
      "computed-traces.{}.computed-repair.promotion must remain pending",
      object_id
    );
    // applies-at points at this trace's SourceObject — the
    // owner-law parameterisation is correct.
    assert_eq!(
      as_str(get_path(trace, &["computed-repair", "applies-at"])),
      object_id,
      "computed-traces.{}.computed-repair.applies-at must equal the same object id",
      object_id
    );
  }
}

#[test]
fn v0_1_each_object_produces_meta_circular_log_differential() {
  // Invariant 40: each trace's log differential has
  // after-turn-5.open-held containing structural-binding-conflict
  // and after-turn-5.active-lenses with all four lens ids.
  let value = eval_file(&fixture_path()).unwrap();
  let traces = get(&value, "computed-traces");

  for object_id in ["arm_link_1", "arm_link_2"] {
    let trace = get(traces, object_id);

    let open_held = list_of_strings(get_path(
      trace,
      &[
        "computed-meta-circular-log-differential",
        "after-turn-5",
        "open-held",
      ],
    ));
    assert!(
      open_held.contains(&"held.structural-binding-conflict"),
      "computed-traces.{}.after-turn-5.open-held must carry structural-binding-conflict; got {:?}",
      object_id,
      open_held
    );

    let active_lenses = list_of_strings(get_path(
      trace,
      &[
        "computed-meta-circular-log-differential",
        "after-turn-5",
        "active-lenses",
      ],
    ));
    assert_eq!(
      active_lenses,
      vec![
        "lens.base",
        "lens.mechanism",
        "lens.animation",
        "lens.projection"
      ],
      "computed-traces.{}.after-turn-5.active-lenses must list all four v0 lenses",
      object_id
    );

    // Initial state is the same shared empty state for both
    // objects (passed in from inputs.initialState).
    let initial_active = list_of_strings(get_path(
      trace,
      &[
        "computed-meta-circular-log-differential",
        "initial-state",
        "active-lenses",
      ],
    ));
    assert_eq!(
      initial_active,
      Vec::<&str>::new(),
      "computed-traces.{}.initial-state.active-lenses must be empty",
      object_id
    );
  }
}

#[test]
fn v0_1_without_owner_law_each_object_empty_trajectory() {
  // Invariant 41: without-owner-law, each trace is empty/null
  // and its log differential is the no-change record.
  let value = eval_file(&fixture_without_owner_law_path()).unwrap();

  match get(&value, "owner-law-loaded") {
    Value::Bool(b) => assert!(
      !*b,
      "without-owner-law harness must report owner-law-loaded=false"
    ),
    other => panic!("owner-law-loaded must be a Bool, got {:?}", other),
  }

  let traces = get(&value, "computed-traces");

  for object_id in ["arm_link_1", "arm_link_2"] {
    let trace = get(traces, object_id);

    let turns = as_list(get(trace, "computed-turns"));
    assert!(
      turns.is_empty(),
      "without owner-law: computed-traces.{}.computed-turns must be empty; got {} turns",
      object_id,
      turns.len()
    );

    assert!(
      is_null(get(trace, "computed-lens-compare")),
      "without owner-law: computed-traces.{}.computed-lens-compare must be null",
      object_id
    );
    assert!(
      is_null(get(trace, "computed-held")),
      "without owner-law: computed-traces.{}.computed-held must be null",
      object_id
    );
    assert!(
      is_null(get(trace, "computed-repair")),
      "without owner-law: computed-traces.{}.computed-repair must be null",
      object_id
    );

    // No-change log differential: after-turn-5 == initial-state.
    let initial = get_path(
      trace,
      &["computed-meta-circular-log-differential", "initial-state"],
    )
    .to_json();
    let after = get_path(
      trace,
      &["computed-meta-circular-log-differential", "after-turn-5"],
    )
    .to_json();
    assert_eq!(
      initial, after,
      "without owner-law: computed-traces.{} after-turn-5 must equal initial-state (no turns occurred);\n\
       initial = {}\n\
       after   = {}",
      object_id, initial, after
    );
  }
}

#[test]
fn v0_1_no_cross_object_contamination_in_trace() {
  // Invariant 42 — THE load-bearing v0.1 invariant. Each
  // per-object trace's JSON serialisation must NOT mention the
  // OTHER object's id anywhere. If lens metadata interpolation
  // leaked the wrong object id, a shared-state cache reused a
  // string across objects, or a runner accidentally pulled
  // sourceObject from the wrong slot, this test catches it.
  //
  // The contamination test runs on BOTH the with-owner-law and
  // the without-owner-law trajectories — owner-law absence is
  // not an excuse for cross-object leak.
  for (label, path) in [
    ("with-owner-law", fixture_path()),
    ("without-owner-law", fixture_without_owner_law_path()),
  ] {
    let value = eval_file(&path).expect("harness must evaluate");
    let traces = get(&value, "computed-traces");

    let trace1 = get(traces, "arm_link_1").to_json();
    assert!(
      !trace1.contains("arm_link_2"),
      "[{}] arm_link_1 trace must not mention arm_link_2 anywhere; full trace JSON:\n{}",
      label,
      trace1
    );

    let trace2 = get(traces, "arm_link_2").to_json();
    assert!(
      !trace2.contains("arm_link_1"),
      "[{}] arm_link_2 trace must not mention arm_link_1 anywhere; full trace JSON:\n{}",
      label,
      trace2
    );

    // Sanity: each trace DOES mention its own id (otherwise the
    // contamination assertion above would be trivially true).
    assert!(
      trace1.contains("arm_link_1"),
      "[{}] arm_link_1 trace must contain its own id; got:\n{}",
      label,
      trace1
    );
    assert!(
      trace2.contains("arm_link_2"),
      "[{}] arm_link_2 trace must contain its own id; got:\n{}",
      label,
      trace2
    );
  }
}

#[test]
fn v0_1_owner_law_file_unchanged_seven_rule_surface() {
  // Invariant 43: v0.1 reuses v0_owner_law.px by reference. The
  // file must still expose exactly the same 7 Lambda rules and
  // carry no oracle / source-object / lens leak. If a v0.1-side
  // edit slipped into the owner-law file, this test catches it.
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
        "v0_owner_law.px must still expose `{}` after v0.1; available: {:?}",
        rule,
        attrs.keys().collect::<Vec<_>>()
      )
    });
    assert!(
      matches!(entry, Value::Lambda { .. }),
      "v0_owner_law.px:`{}` must still be a Lambda after v0.1, got {:?}",
      rule,
      entry
    );
  }

  let allowed: std::collections::BTreeSet<&str> = required_rules.iter().copied().collect();
  for key in attrs.keys() {
    assert!(
      allowed.contains(key.as_str()),
      "v0.1 must NOT add new owner-law rules; v0_owner_law.px carries unexpected key `{}`",
      key
    );
  }
}
