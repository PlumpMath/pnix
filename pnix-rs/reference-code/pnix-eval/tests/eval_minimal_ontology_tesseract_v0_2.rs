//! minimal-ontology-tesseract-v0.2 — layer 5 runtime route
//! candidate emission. Pure additive layer on top of v0.1's
//! closed gate fold. Routes are emitted as candidate-only
//! records derived from the semantic frame of each input,
//! annotated with the gate.id list as required-gates. Whether
//! a route would actually execute (gate satisfaction, runtime
//! invocation, native API call) is OUT of v0.2 scope.
//!
//! Load-bearing claim:
//!   semantic frame determines the runtime route family;
//!   the fold's gate.id list becomes the route's
//!   required-gates annotation; v0.2 is a pure additive
//!   layer (only `runtime` field changes vs v0.1); null
//!   semantic blocks runtime route emission (gate-only
//!   emission is forbidden).
//!
//! Truth owner:        project-wiki/maps/minimal-ontology-tesseract-v0-map.md
//!                     §"v0.2 design decision — layer 5 runtime route
//!                       candidate emission"
//! Active scope:       project-wiki/maps/active-domain-constitution.md
//!                     Art. 6, Art. 7
//!
//! Test count: 8 invariants, indices 325..332, expected as 8
//! distinct `#[test]` functions (per map's pinned plan).

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------
// Helpers — same shape as v0 / v0.1 test files.
// ---------------------------------------------------------------

fn fixture_root() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../../fixtures/minimal-ontology-tesseract-v0")
}

fn route_law_path() -> PathBuf {
  fixture_root().join("v0_2_route_law.px")
}

fn v0_1_run_path() -> PathBuf {
  fixture_root().join("v0_1_run.px")
}

fn v0_1_run_negative_path() -> PathBuf {
  fixture_root().join("v0_1_run_negative.px")
}

fn v0_2_run_path() -> PathBuf {
  fixture_root().join("v0_2_run.px")
}

fn v0_2_run_negative_path() -> PathBuf {
  fixture_root().join("v0_2_run_negative.px")
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

fn list_strings(v: &Value) -> Vec<&str> {
  as_list(v).iter().map(|item| as_str(item)).collect()
}

/// Convert a single route emission to a deterministic 8-tuple
/// for exact equality assertions:
///   (id, target, route-kind, api-shape, safety-level,
///    source-frame, required-gates joined by ",", status)
fn route_tuple<'a>(
  r: &'a Value,
) -> (
  &'a str,
  &'a str,
  &'a str,
  &'a str,
  &'a str,
  &'a str,
  String,
  &'a str,
) {
  let req_gates: Vec<&str> = list_strings(get(r, "required-gates"));
  (
    as_str(get(r, "id")),
    as_str(get(r, "target")),
    as_str(get(r, "route-kind")),
    as_str(get(r, "api-shape")),
    as_str(get(r, "safety-level")),
    as_str(get(r, "source-frame")),
    req_gates.join(","),
    as_str(get(r, "status")),
  )
}

// ---------------------------------------------------------------
// Tests.
// ---------------------------------------------------------------

#[test]
fn v0_2_route_kind_enum_and_status_literal() {
  // Invariant 325.
  let law = eval_file(&route_law_path()).expect("v0_2_route_law.px must evaluate");
  let kind_enum: Vec<&str> = list_strings(get(&law, "routeKindEnum"));
  let expected = vec!["evaluator", "emitter", "simulator", "controller"];
  assert_eq!(
    kind_enum, expected,
    "routeKindEnum must be exactly the 4 canonical kinds in declared order"
  );
  assert_eq!(
    as_str(get(&law, "routeStatusLiteral")),
    "candidate",
    "routeStatusLiteral must equal \"candidate\""
  );
}

#[test]
fn v0_2_per_frame_route_shape_and_kind_membership() {
  // Invariant 326.
  let law = eval_file(&route_law_path()).unwrap();
  let per_frame = as_attrs(get(&law, "perFrameRoute"));

  // Exactly 2 keys.
  let keys: BTreeSet<&str> = per_frame.keys().map(|s| s.as_str()).collect();
  let expected: BTreeSet<&str> = ["invariant", "property"].into_iter().collect();
  assert_eq!(
    keys, expected,
    "perFrameRoute must have exactly 2 keys: invariant / property"
  );

  // Every template has the 5 required fields; route-kind must be
  // in routeKindEnum.
  let kind_enum: BTreeSet<&str> = list_strings(get(&law, "routeKindEnum"))
    .into_iter()
    .collect();
  for (frame, templates) in per_frame {
    let tlist = as_list(templates);
    assert!(
      !tlist.is_empty(),
      "perFrameRoute.`{}` must be non-empty",
      frame
    );
    for t in tlist {
      let _ = as_str(get(t, "id-template"));
      let _ = as_str(get(t, "target"));
      let kind = as_str(get(t, "route-kind"));
      let _ = as_str(get(t, "api-shape"));
      let _ = as_str(get(t, "safety-level"));
      assert!(
        kind_enum.contains(kind),
        "perFrameRoute.`{}` template route-kind {:?} not in routeKindEnum",
        frame,
        kind
      );
    }
  }
}

#[test]
fn v0_2_runtime_input_a_load_bearing() {
  // Invariant 327.
  let run = eval_file(&v0_2_run_path()).unwrap();
  let routes = as_list(get_path(&run, &["fold-A", "runtime"]));
  let actual: Vec<_> = routes.iter().map(route_tuple).collect();
  let expected_required = "gate.triangle.valid,gate.area.computable,gate.min.satisfiable";
  let expected = vec![
    (
      "route.invariant.geometry-evaluator",
      "geometry-evaluator",
      "evaluator",
      "evaluate-invariant",
      "local-deterministic",
      "invariant",
      expected_required.to_string(),
      "candidate",
    ),
    (
      "route.invariant.x3d-emitter",
      "x3d-emitter",
      "emitter",
      "emit-constraint-overlay",
      "render-only",
      "invariant",
      expected_required.to_string(),
      "candidate",
    ),
  ];
  assert_eq!(
    actual, expected,
    "fold-A.runtime must equal the 2-element list in declared order with exact field values"
  );
}

#[test]
fn v0_2_runtime_input_b_load_bearing() {
  // Invariant 328.
  let run = eval_file(&v0_2_run_path()).unwrap();
  let routes = as_list(get_path(&run, &["fold-B", "runtime"]));
  let actual: Vec<_> = routes.iter().map(route_tuple).collect();
  let expected_required = "gate.triangle.valid,gate.area.computable";
  let expected = vec![(
    "route.property.geometry-evaluator",
    "geometry-evaluator",
    "evaluator",
    "evaluate-property",
    "local-deterministic",
    "property",
    expected_required.to_string(),
    "candidate",
  )];
  assert_eq!(
    actual, expected,
    "fold-B.runtime must equal the 1-element list (NO x3d-emitter — perFrameRoute.property has no emitter template)"
  );
  // Explicitly assert x3d-emitter is NOT present on fold-B.
  for r in routes {
    assert_ne!(
      as_str(get(r, "id")),
      "route.property.x3d-emitter",
      "fold-B has frame=property which has no emitter template"
    );
  }
}

#[test]
fn v0_2_required_gates_match_fold_gate_ids_load_bearing() {
  // Invariant 329.
  // Every emitted route's `required-gates` must equal the
  // fold's gate.id list verbatim — proves the gate-annotation
  // contract that ties layer 5 to layer 4.
  let run = eval_file(&v0_2_run_path()).unwrap();
  for fold_name in &["fold-A", "fold-B"] {
    let fold = get(&run, fold_name);
    let gate_ids: Vec<&str> = as_list(get(fold, "gate"))
      .iter()
      .map(|g| as_str(get(g, "id")))
      .collect();
    let routes = as_list(get(fold, "runtime"));
    for r in routes {
      let req: Vec<&str> = list_strings(get(r, "required-gates"));
      assert_eq!(
        req,
        gate_ids,
        "[{}] route `{}`'s required-gates must equal the fold's gate.id list verbatim",
        fold_name,
        as_str(get(r, "id"))
      );
    }
  }
}

#[test]
fn v0_2_null_semantic_runtime_empty_load_bearing() {
  // Invariant 330.
  let neg = eval_file(&v0_2_run_negative_path()).unwrap();

  for fold_name in &["fold-unknown", "fold-bareConstraint"] {
    let fold = get(&neg, fold_name);
    // semantic.frame must remain null (Held preserved verbatim).
    assert!(
      matches!(get_path(fold, &["semantic", "frame"]), Value::Null),
      "[{}] semantic.frame must remain null (Held preserved from v0.1/v0)",
      fold_name
    );
    // gate is non-empty (v0.1 invariant 322).
    let gates = as_list(get(fold, "gate"));
    assert!(
      !gates.is_empty(),
      "[{}] fold.gate must be non-empty (v0.1 emits gate on known ontology kinds even with null semantic)",
      fold_name
    );
    // runtime MUST be exactly [].
    let routes = as_list(get(fold, "runtime"));
    assert!(
      routes.is_empty(),
      "[{}] fold.runtime MUST be [] when semantic.frame is null — gate-only route emission is forbidden",
      fold_name
    );
    // Held entry preserved.
    assert!(
      !matches!(get(fold, "held"), Value::Null),
      "[{}] fold.held MUST be preserved (non-null) from v0/v0.1",
      fold_name
    );
  }
}

#[test]
fn v0_2_allowed_delta_only_diff_vs_v0_1_load_bearing() {
  // Invariant 331.
  // Pairwise comparison: every non-`runtime` field must be
  // byte-equal between v0.1 fold output and v0.2 fold output.
  let v0_1_run = eval_file(&v0_1_run_path()).unwrap();
  let v0_1_neg = eval_file(&v0_1_run_negative_path()).unwrap();
  let v0_2_run = eval_file(&v0_2_run_path()).unwrap();
  let v0_2_neg = eval_file(&v0_2_run_negative_path()).unwrap();

  fn assert_only_runtime_differs(lhs: &Value, rhs: &Value, label: &str) {
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
      if *key == "runtime" {
        continue;
      }
      let lv = l.get(*key).unwrap();
      let rv = r.get(*key).unwrap();
      assert_eq!(
        lv.to_json(),
        rv.to_json(),
        "[{}] field `{}` must be byte-for-byte identical between v0.1 and v0.2 (only `runtime` may differ)",
        label, key
      );
    }
  }

  // (a) positive folds.
  for fold_name in &["fold-A", "fold-B"] {
    assert_only_runtime_differs(
      get(&v0_1_run, fold_name),
      get(&v0_2_run, fold_name),
      &format!("positive {}", fold_name),
    );
    // confirm runtime is the actual delta: v0.1=null, v0.2=non-null.
    assert!(matches!(
      get_path(&v0_1_run, &[fold_name, "runtime"]),
      Value::Null
    ));
    assert!(!matches!(
      get_path(&v0_2_run, &[fold_name, "runtime"]),
      Value::Null
    ));
  }

  // (b) negative folds.
  for fold_name in &["fold-unknown", "fold-bareConstraint"] {
    assert_only_runtime_differs(
      get(&v0_1_neg, fold_name),
      get(&v0_2_neg, fold_name),
      &format!("negative {}", fold_name),
    );
    assert!(matches!(
      get_path(&v0_1_neg, &[fold_name, "runtime"]),
      Value::Null
    ));
    // v0.2 negative runtime is [] (List), NOT null.
    assert!(matches!(
      get_path(&v0_2_neg, &[fold_name, "runtime"]),
      Value::List(_)
    ));
    assert!(as_list(get_path(&v0_2_neg, &[fold_name, "runtime"])).is_empty());
  }
}

#[test]
fn v0_2_layer_marker_runtime_active_true() {
  // Invariant 332.
  let run = eval_file(&v0_2_run_path()).unwrap();
  let layers = get(&run, "layers");
  assert!(as_bool(get(layers, "surface-active")));
  assert!(as_bool(get(layers, "ontology-active")));
  assert!(as_bool(get(layers, "semantic-active")));
  assert!(as_bool(get(layers, "gate-active")));
  assert!(
    as_bool(get(layers, "runtime-active")),
    "v0.2 MUST flip layers.runtime-active = true"
  );
  assert!(
    !as_bool(get(layers, "audit-active")),
    "layers.audit-active stays false in v0.2 (deferred to v0.3)"
  );
  assert_eq!(
    as_str(get(&run, "v0-marker")),
    "minimal-ontology-tesseract-v0.2"
  );
}
