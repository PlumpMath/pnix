//! minimal-ontology-tesseract-v0.1 — layer 4 gate generation.
//! Pure additive layer on top of v0's closed
//! surface → ontology → semantic fold. Gates are emitted as
//! candidate-only requirements derived from the ontology kinds
//! present along each input's path; runtime route selection
//! (whether a gate is satisfied) is v0.2's question.
//!
//! Load-bearing claim:
//!   ontology kind sequence determines the gate requirement set
//!   the same way it determines the semantic frame; v0.1 is a
//!   pure additive layer (only `gate` field changes vs v0).
//!
//! Truth owner:        project-wiki/maps/minimal-ontology-tesseract-v0-map.md
//!                     §"v0.1 design decision — layer 4 gate generation"
//! Active scope:       project-wiki/maps/active-domain-constitution.md
//!                     Art. 6, Art. 7
//!
//! Test count: 8 invariants, indices 317..324, expected as 8
//! distinct `#[test]` functions (per map's pinned plan).
//!
//! 317. `v0_1_gate_law.px`'s `perKindGate` declares exactly 3
//!      keys (`GeometryObject` / `Measure` / `Constraint`); each
//!      maps to a non-empty list of gate-requirement templates;
//!      every template's `kind` value is a member of
//!      `gateKindEnum`.
//! 318. `gateKindEnum` is exactly the 5 canonical kinds in
//!      declared order:
//!      `["type-check","permission","proof","safety","computability"]`.
//! 319. **load-bearing — gate emission per input A**: fold-A.gate
//!      equals exactly the 3-element list
//!      [ gate.triangle.valid (proof, GeometryObject),
//!        gate.area.computable (computability, Measure),
//!        gate.min.satisfiable (proof, Constraint) ]
//!      in path-segment order; every gate carries
//!      `subject = <its segment>` and `status = "candidate"`.
//! 320. **load-bearing — gate emission per input B**: fold-B.gate
//!      equals exactly the 2-element list
//!      [ gate.triangle.valid, gate.area.computable ] in
//!      path-segment order; `gate.min.satisfiable` is NOT
//!      present (the central v0.1 claim that gate set
//!      discriminates by ontology kind sequence).
//! 321. **load-bearing — candidate-only**: every gate emitted
//!      across folds A / B / unknown / bareConstraint has
//!      `status = "candidate"`; no other status value appears.
//! 322. **load-bearing — gate emission on null-semantic inputs
//!      EXACT list, not just non-empty**: fold-unknown.gate
//!      equals exactly [ gate.triangle.valid, gate.min.satisfiable ]
//!      (the unknown segment "perimeter" contributes NO gate);
//!      fold-bareConstraint.gate equals exactly
//!      [ gate.min.satisfiable ]; both folds keep
//!      semantic.frame = null (Held emission preserved verbatim).
//! 323. **load-bearing — allowed-delta-only diff vs v0,
//!      compared pairwise per runner**:
//!      (a) positive: every input in {A, B} has byte-equal
//!          output via `Value::to_json()` between v0_run.px and
//!          v0_1_run.px on every field EXCEPT `gate`;
//!      (b) negative: every input in {unknown, bareConstraint}
//!          has byte-equal output between v0_run_negative.px and
//!          v0_1_run_negative.px on every field EXCEPT `gate`.
//!      Any drift outside `gate` fails the test, proving v0.1
//!      is a pure additive layer.
//! 324. layer marker: v0_1_run.px emits `layers.gate-active =
//!      true`; runtime-active and audit-active remain false;
//!      surface/ontology/semantic-active stay true.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------
// Helpers — same shape as v0 test file.
// ---------------------------------------------------------------

fn fixture_root() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../../fixtures/minimal-ontology-tesseract-v0")
}

fn gate_law_path() -> PathBuf {
  fixture_root().join("v0_1_gate_law.px")
}

fn v0_run_path() -> PathBuf {
  fixture_root().join("v0_run.px")
}

fn v0_run_negative_path() -> PathBuf {
  fixture_root().join("v0_run_negative.px")
}

fn v0_1_run_path() -> PathBuf {
  fixture_root().join("v0_1_run.px")
}

fn v0_1_run_negative_path() -> PathBuf {
  fixture_root().join("v0_1_run_negative.px")
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

fn is_null(v: &Value) -> bool {
  matches!(v, Value::Null)
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

fn list_strings(v: &Value) -> Vec<&str> {
  as_list(v).iter().map(|item| as_str(item)).collect()
}

/// Convert a single gate emission to a deterministic 6-tuple
/// (id, kind, subject, predicate, source-rule, status) for
/// exact-equality assertions.
fn gate_tuple<'a>(g: &'a Value) -> (&'a str, &'a str, &'a str, &'a str, &'a str, &'a str) {
  (
    as_str(get(g, "id")),
    as_str(get(g, "kind")),
    as_str(get(g, "subject")),
    as_str(get(g, "predicate")),
    as_str(get(g, "source-rule")),
    as_str(get(g, "status")),
  )
}

// ---------------------------------------------------------------
// Tests.
// ---------------------------------------------------------------

#[test]
fn v0_1_gate_law_per_kind_gate_shape_and_kind_enum_membership() {
  // Invariant 317.
  let law = eval_file(&gate_law_path()).expect("v0_1_gate_law.px must evaluate");
  let per_kind = as_attrs(get(&law, "perKindGate"));

  // Exactly 3 keys.
  let keys: BTreeSet<&str> = per_kind.keys().map(|s| s.as_str()).collect();
  let expected: BTreeSet<&str> = ["GeometryObject", "Measure", "Constraint"]
    .into_iter()
    .collect();
  assert_eq!(
    keys, expected,
    "perKindGate must have exactly 3 keys: GeometryObject / Measure / Constraint"
  );

  // Each key maps to a non-empty list; every template has
  // `id-template`, `kind`, `predicate`, `source-rule`; every
  // `kind` value is in gateKindEnum.
  let kind_enum: BTreeSet<&str> = list_strings(get(&law, "gateKindEnum"))
    .into_iter()
    .collect();
  for (kind_key, templates) in per_kind {
    let tlist = as_list(templates);
    assert!(
      !tlist.is_empty(),
      "perKindGate.`{}` must be non-empty",
      kind_key
    );
    for t in tlist {
      let _ = as_str(get(t, "id-template"));
      let k = as_str(get(t, "kind"));
      let _ = as_str(get(t, "predicate"));
      let _ = as_str(get(t, "source-rule"));
      assert!(
        kind_enum.contains(k),
        "perKindGate.`{}` template `kind = {:?}` is NOT a member of gateKindEnum {:?}",
        kind_key,
        k,
        kind_enum
      );
    }
  }
}

#[test]
fn v0_1_gate_kind_enum_exact_five() {
  // Invariant 318.
  let law = eval_file(&gate_law_path()).unwrap();
  let kind_enum: Vec<&str> = list_strings(get(&law, "gateKindEnum"));
  let expected = vec![
    "type-check",
    "permission",
    "proof",
    "safety",
    "computability",
  ];
  assert_eq!(
    kind_enum, expected,
    "gateKindEnum must be exactly the 5 canonical kinds in declared order"
  );
}

#[test]
fn v0_1_gate_emission_input_a_load_bearing() {
  // Invariant 319.
  let run = eval_file(&v0_1_run_path()).unwrap();
  let gates = as_list(get_path(&run, &["fold-A", "gate"]));
  let actual: Vec<_> = gates.iter().map(gate_tuple).collect();
  let expected = vec![
    (
      "gate.triangle.valid",
      "proof",
      "triangle",
      "valid",
      "GeometryObject",
      "candidate",
    ),
    (
      "gate.area.computable",
      "computability",
      "area",
      "computable",
      "Measure",
      "candidate",
    ),
    (
      "gate.min.satisfiable",
      "proof",
      "min",
      "satisfiable",
      "Constraint",
      "candidate",
    ),
  ];
  assert_eq!(
    actual, expected,
    "fold-A.gate must equal the 3-element list in path-segment order with exact field values"
  );
}

#[test]
fn v0_1_gate_emission_input_b_load_bearing() {
  // Invariant 320.
  let run = eval_file(&v0_1_run_path()).unwrap();
  let gates = as_list(get_path(&run, &["fold-B", "gate"]));
  let actual: Vec<_> = gates.iter().map(gate_tuple).collect();
  let expected = vec![
    (
      "gate.triangle.valid",
      "proof",
      "triangle",
      "valid",
      "GeometryObject",
      "candidate",
    ),
    (
      "gate.area.computable",
      "computability",
      "area",
      "computable",
      "Measure",
      "candidate",
    ),
  ];
  assert_eq!(
    actual, expected,
    "fold-B.gate must equal the 2-element list in path-segment order"
  );
  // gate.min.satisfiable explicitly NOT present.
  for g in gates {
    assert_ne!(
      as_str(get(g, "id")),
      "gate.min.satisfiable",
      "fold-B has no Constraint segment, so gate.min.satisfiable MUST NOT appear"
    );
  }
}

#[test]
fn v0_1_candidate_only_load_bearing() {
  // Invariant 321.
  let run = eval_file(&v0_1_run_path()).unwrap();
  let neg = eval_file(&v0_1_run_negative_path()).unwrap();
  let folds = [
    get(&run, "fold-A"),
    get(&run, "fold-B"),
    get(&neg, "fold-unknown"),
    get(&neg, "fold-bareConstraint"),
  ];
  for fold in folds {
    let gates = as_list(get(fold, "gate"));
    for g in gates {
      let status = as_str(get(g, "status"));
      assert_eq!(
        status,
        "candidate",
        "every emitted gate MUST have status = \"candidate\"; got status = {:?} on gate id {:?}",
        status,
        as_str(get(g, "id"))
      );
    }
  }
}

#[test]
fn v0_1_gate_emission_on_null_semantic_load_bearing() {
  // Invariant 322.
  let neg = eval_file(&v0_1_run_negative_path()).unwrap();

  // unknown
  let unknown = get(&neg, "fold-unknown");
  assert!(
    is_null(get_path(unknown, &["semantic", "frame"])),
    "fold-unknown.semantic.frame must remain null (Held: ontology-unknown preserved from v0)"
  );
  let unknown_gates = as_list(get(unknown, "gate"));
  let actual_unknown: Vec<_> = unknown_gates.iter().map(gate_tuple).collect();
  let expected_unknown = vec![
    (
      "gate.triangle.valid",
      "proof",
      "triangle",
      "valid",
      "GeometryObject",
      "candidate",
    ),
    (
      "gate.min.satisfiable",
      "proof",
      "min",
      "satisfiable",
      "Constraint",
      "candidate",
    ),
  ];
  assert_eq!(
    actual_unknown, expected_unknown,
    "fold-unknown.gate must equal the 2-element list (perimeter contributes NO gate)"
  );

  // bareConstraint
  let bare = get(&neg, "fold-bareConstraint");
  assert!(
    is_null(get_path(bare, &["semantic", "frame"])),
    "fold-bareConstraint.semantic.frame must remain null"
  );
  let bare_gates = as_list(get(bare, "gate"));
  let actual_bare: Vec<_> = bare_gates.iter().map(gate_tuple).collect();
  let expected_bare = vec![(
    "gate.min.satisfiable",
    "proof",
    "min",
    "satisfiable",
    "Constraint",
    "candidate",
  )];
  assert_eq!(
    actual_bare, expected_bare,
    "fold-bareConstraint.gate must equal the 1-element list"
  );
}

#[test]
fn v0_1_allowed_delta_only_diff_vs_v0_load_bearing() {
  // Invariant 323.
  let v0_run = eval_file(&v0_run_path()).unwrap();
  let v0_neg = eval_file(&v0_run_negative_path()).unwrap();
  let v0_1_run = eval_file(&v0_1_run_path()).unwrap();
  let v0_1_neg = eval_file(&v0_1_run_negative_path()).unwrap();

  // Helper: compare two folds, every field except `gate` must
  // be byte-equal via Value::to_json().
  fn assert_only_gate_differs(lhs: &Value, rhs: &Value, label: &str) {
    let l = as_attrs(lhs);
    let r = as_attrs(rhs);
    let l_keys: BTreeSet<&str> = l.keys().map(|s| s.as_str()).collect();
    let r_keys: BTreeSet<&str> = r.keys().map(|s| s.as_str()).collect();
    assert_eq!(
      l_keys, r_keys,
      "[{}] fold attrset key sets must match",
      label
    );
    for key in &l_keys {
      if *key == "gate" {
        continue;
      }
      let lv = l.get(*key).unwrap();
      let rv = r.get(*key).unwrap();
      assert_eq!(
        lv.to_json(),
        rv.to_json(),
        "[{}] field `{}` must be byte-for-byte identical between v0 and v0.1 (only `gate` may differ)",
        label, key
      );
    }
  }

  // (a) positive
  for fold_name in &["fold-A", "fold-B"] {
    assert_only_gate_differs(
      get(&v0_run, fold_name),
      get(&v0_1_run, fold_name),
      &format!("positive {}", fold_name),
    );
    // confirm gate is the actual delta: v0=null, v0.1=non-null
    assert!(is_null(get_path(&v0_run, &[fold_name, "gate"])));
    assert!(!is_null(get_path(&v0_1_run, &[fold_name, "gate"])));
  }

  // (b) negative
  for fold_name in &["fold-unknown", "fold-bareConstraint"] {
    assert_only_gate_differs(
      get(&v0_neg, fold_name),
      get(&v0_1_neg, fold_name),
      &format!("negative {}", fold_name),
    );
    assert!(is_null(get_path(&v0_neg, &[fold_name, "gate"])));
    assert!(!is_null(get_path(&v0_1_neg, &[fold_name, "gate"])));
  }
}

#[test]
fn v0_1_layer_marker_gate_active_true() {
  // Invariant 324.
  let run = eval_file(&v0_1_run_path()).unwrap();
  let layers = get(&run, "layers");
  assert!(as_bool(get(layers, "surface-active")));
  assert!(as_bool(get(layers, "ontology-active")));
  assert!(as_bool(get(layers, "semantic-active")));
  assert!(
    as_bool(get(layers, "gate-active")),
    "v0.1 MUST flip layers.gate-active = true"
  );
  assert!(
    !as_bool(get(layers, "runtime-active")),
    "layers.runtime-active stays false in v0.1 (deferred to v0.2)"
  );
  assert!(
    !as_bool(get(layers, "audit-active")),
    "layers.audit-active stays false in v0.1 (deferred to v0.3)"
  );
  assert_eq!(
    as_str(get(&run, "v0-marker")),
    "minimal-ontology-tesseract-v0.1"
  );
}
