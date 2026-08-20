//! minimal-ontology-tesseract-v0.3 — layer 6 audit fold log.
//! Pure additive layer on top of v0.2's closed runtime route
//! fold. Audit captures the deterministic trace of how each
//! fold result was produced (ontology classification, semantic
//! frame selection, gate emission, runtime emission, Held
//! information when present). v0.3 closure means the first
//! canonical 6-layer tesseract fold instance proof is
//! complete: surface → ontology → semantic → gate → runtime
//! → audit.
//!
//! Load-bearing claim:
//!   audit captures every layer's deterministic decision,
//!   including unknown path segments and full Held content
//!   (kind / subject / candidate-frames / reason); audit is
//!   replay-deterministic via Value::to_json normalized
//!   structural equality (the v0.x floor unchanged); v0.3 is
//!   a pure additive layer (only `audit` field changes vs
//!   v0.2).
//!
//! Truth owner:        project-wiki/maps/minimal-ontology-tesseract-v0-map.md
//!                     §"v0.3 design decision — layer 6 audit fold log"
//! Active scope:       project-wiki/maps/active-domain-constitution.md
//!                     Art. 6, Art. 7
//!
//! Test count: 8 invariants, indices 333..340.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------
// Helpers — same shape as v0 / v0.1 / v0.2 test files.
// ---------------------------------------------------------------

fn fixture_root() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../../fixtures/minimal-ontology-tesseract-v0")
}

fn v0_2_run_path() -> PathBuf {
  fixture_root().join("v0_2_run.px")
}

fn v0_2_run_negative_path() -> PathBuf {
  fixture_root().join("v0_2_run_negative.px")
}

fn v0_3_run_path() -> PathBuf {
  fixture_root().join("v0_3_run.px")
}

fn v0_3_run_negative_path() -> PathBuf {
  fixture_root().join("v0_3_run_negative.px")
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

// ---------------------------------------------------------------
// Tests.
// ---------------------------------------------------------------

#[test]
fn v0_3_audit_field_seven_key_shape() {
  // Invariant 333.
  let run = eval_file(&v0_3_run_path()).unwrap();
  let neg = eval_file(&v0_3_run_negative_path()).unwrap();
  let folds: Vec<(&str, &Value)> = vec![
    ("fold-A", get(&run, "fold-A")),
    ("fold-B", get(&run, "fold-B")),
    ("fold-unknown", get(&neg, "fold-unknown")),
    ("fold-bareConstraint", get(&neg, "fold-bareConstraint")),
  ];
  let expected_keys: BTreeSet<&str> = [
    "fold-reason",
    "ontology-trace",
    "semantic-trace",
    "gate-trace",
    "runtime-trace",
    "failure-reason",
    "replay-ref",
  ]
  .into_iter()
  .collect();
  for (label, fold) in folds {
    let audit = get(fold, "audit");
    let keys: BTreeSet<&str> = as_attrs(audit).keys().map(|s| s.as_str()).collect();
    assert_eq!(
      keys, expected_keys,
      "[{}] audit must have exactly the 7 required keys",
      label
    );
  }
}

#[test]
fn v0_3_fold_a_audit_exact_content_load_bearing() {
  // Invariant 334.
  let run = eval_file(&v0_3_run_path()).unwrap();
  let audit = get_path(&run, &["fold-A", "audit"]);

  assert_eq!(
    as_str(get(audit, "fold-reason")),
    "GeometryObject|Measure|Constraint -> invariant"
  );

  // ontology-trace
  let ot = as_list(get(audit, "ontology-trace"));
  assert_eq!(ot.len(), 3);
  let ot_tuples: Vec<(&str, Option<&str>, &str)> = ot
    .iter()
    .map(|e| {
      let kind_v = get(e, "kind");
      let kind = if is_null(kind_v) {
        None
      } else {
        Some(as_str(kind_v))
      };
      (
        as_str(get(e, "segment")),
        kind,
        as_str(get(e, "source-rule")),
      )
    })
    .collect();
  assert_eq!(
    ot_tuples,
    vec![
      ("triangle", Some("GeometryObject"), "known"),
      ("area", Some("Measure"), "known"),
      ("min", Some("Constraint"), "known"),
    ]
  );

  // semantic-trace
  let st = get(audit, "semantic-trace");
  assert_eq!(as_str(get(st, "frame")), "invariant");
  assert_eq!(as_str(get(st, "rule")), "GeometryObject|Measure|Constraint");
  assert!(is_null(get(st, "cause")));

  // gate-trace size
  assert_eq!(as_list(get(audit, "gate-trace")).len(), 3);
  // runtime-trace size
  assert_eq!(as_list(get(audit, "runtime-trace")).len(), 2);
  // failure-reason
  assert!(is_null(get(audit, "failure-reason")));
  // replay-ref
  assert_eq!(as_str(get(audit, "replay-ref")), "audit.fold.A");
}

#[test]
fn v0_3_fold_b_audit_exact_content_load_bearing() {
  // Invariant 335.
  let run = eval_file(&v0_3_run_path()).unwrap();
  let audit = get_path(&run, &["fold-B", "audit"]);

  assert_eq!(
    as_str(get(audit, "fold-reason")),
    "GeometryObject|Measure -> property"
  );

  let ot = as_list(get(audit, "ontology-trace"));
  assert_eq!(ot.len(), 2);
  let ot_segs: Vec<&str> = ot.iter().map(|e| as_str(get(e, "segment"))).collect();
  assert_eq!(ot_segs, vec!["triangle", "area"]);
  for entry in ot {
    assert_eq!(as_str(get(entry, "source-rule")), "known");
  }

  let st = get(audit, "semantic-trace");
  assert_eq!(as_str(get(st, "frame")), "property");
  assert_eq!(as_str(get(st, "rule")), "GeometryObject|Measure");
  assert!(is_null(get(st, "cause")));

  assert_eq!(as_list(get(audit, "gate-trace")).len(), 2);
  assert_eq!(as_list(get(audit, "runtime-trace")).len(), 1);
  assert!(is_null(get(audit, "failure-reason")));
  assert_eq!(as_str(get(audit, "replay-ref")), "audit.fold.B");
}

#[test]
fn v0_3_audit_replay_determinism_load_bearing() {
  // Invariant 336.
  // Two evaluations of the same fixture must produce
  // identical audit fields per fold via Value::to_json()
  // (the v0.x normalized structural equality floor).
  let run_a = eval_file(&v0_3_run_path()).unwrap();
  let run_b = eval_file(&v0_3_run_path()).unwrap();
  for fold_name in &["fold-A", "fold-B"] {
    let lhs = get_path(&run_a, &[fold_name, "audit"]);
    let rhs = get_path(&run_b, &[fold_name, "audit"]);
    assert_eq!(
      lhs.to_json(),
      rhs.to_json(),
      "[{}] audit field must be replay-deterministic via Value::to_json()",
      fold_name
    );
  }
  let neg_a = eval_file(&v0_3_run_negative_path()).unwrap();
  let neg_b = eval_file(&v0_3_run_negative_path()).unwrap();
  for fold_name in &["fold-unknown", "fold-bareConstraint"] {
    let lhs = get_path(&neg_a, &[fold_name, "audit"]);
    let rhs = get_path(&neg_b, &[fold_name, "audit"]);
    assert_eq!(lhs.to_json(), rhs.to_json());
  }
}

#[test]
fn v0_3_failure_case_audit_exact_load_bearing() {
  // Invariant 337.
  let neg = eval_file(&v0_3_run_negative_path()).unwrap();

  // fold-unknown
  {
    let audit = get_path(&neg, &["fold-unknown", "audit"]);
    assert_eq!(
      as_str(get(audit, "fold-reason")),
      "ontology-unknown -> held"
    );

    // ontology-trace MUST include perimeter with kind=null
    // and source-rule="ontology-unknown" — proves audit
    // captures unknown segments, not just known ones.
    let ot = as_list(get(audit, "ontology-trace"));
    assert_eq!(ot.len(), 3);
    let ot_tuples: Vec<(&str, bool, &str)> = ot
      .iter()
      .map(|e| {
        (
          as_str(get(e, "segment")),
          is_null(get(e, "kind")),
          as_str(get(e, "source-rule")),
        )
      })
      .collect();
    assert_eq!(
      ot_tuples,
      vec![
        ("triangle", false, "known"),
        ("perimeter", true, "ontology-unknown"),
        ("min", false, "known"),
      ],
      "fold-unknown.audit.ontology-trace MUST include perimeter as kind=null with source-rule=\"ontology-unknown\""
    );

    // semantic-trace null frame
    let st = get(audit, "semantic-trace");
    assert!(is_null(get(st, "frame")));
    assert!(is_null(get(st, "rule")));
    assert_eq!(as_str(get(st, "cause")), "ontology-unknown");

    // failure-reason exact
    let fr = get(audit, "failure-reason");
    assert!(!is_null(fr));
    assert_eq!(as_str(get(fr, "kind")), "ontology-unknown");
    assert_eq!(as_str(get(fr, "subject")), "perimeter");
    let cf: Vec<&str> = list_strings(get(fr, "candidate-frames"));
    assert!(cf.is_empty());
    assert_eq!(
      as_str(get(fr, "reason")),
      "ontology has no entry for this path segment"
    );

    assert_eq!(as_str(get(audit, "replay-ref")), "audit.fold.unknown");
    // runtime-trace must be empty (v0.2 enforced runtime=[]
    // for null semantic).
    assert!(as_list(get(audit, "runtime-trace")).is_empty());
  }

  // fold-bareConstraint
  {
    let audit = get_path(&neg, &["fold-bareConstraint", "audit"]);
    assert_eq!(
      as_str(get(audit, "fold-reason")),
      "semantic-frame-ambiguous -> held"
    );

    let st = get(audit, "semantic-trace");
    assert!(is_null(get(st, "frame")));
    assert!(is_null(get(st, "rule")));
    assert_eq!(as_str(get(st, "cause")), "semantic-frame-ambiguous");

    let fr = get(audit, "failure-reason");
    assert!(!is_null(fr));
    assert_eq!(as_str(get(fr, "kind")), "semantic-frame-ambiguous");
    assert_eq!(as_str(get(fr, "subject")), "min");
    // candidate-frames MUST equal exactly ["invariant", "goal"]
    // — Codex 2026-05-06 directive D requires Held to carry
    // candidate-frames into audit, not just kind/subject/reason.
    let cf: Vec<&str> = list_strings(get(fr, "candidate-frames"));
    assert_eq!(cf, vec!["invariant", "goal"]);
    assert_eq!(
      as_str(get(fr, "reason")),
      "Constraint requires a Measure target"
    );

    assert_eq!(
      as_str(get(audit, "replay-ref")),
      "audit.fold.bareConstraint"
    );
    assert!(as_list(get(audit, "runtime-trace")).is_empty());
  }
}

#[test]
fn v0_3_allowed_delta_only_diff_vs_v0_2_load_bearing() {
  // Invariant 338.
  let v0_2_run = eval_file(&v0_2_run_path()).unwrap();
  let v0_2_neg = eval_file(&v0_2_run_negative_path()).unwrap();
  let v0_3_run = eval_file(&v0_3_run_path()).unwrap();
  let v0_3_neg = eval_file(&v0_3_run_negative_path()).unwrap();

  fn assert_only_audit_differs(lhs: &Value, rhs: &Value, label: &str) {
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
      if *key == "audit" {
        continue;
      }
      let lv = l.get(*key).unwrap();
      let rv = r.get(*key).unwrap();
      assert_eq!(
        lv.to_json(),
        rv.to_json(),
        "[{}] field `{}` must be byte-equal between v0.2 and v0.3 (only `audit` may differ)",
        label,
        key
      );
    }
  }

  // (a) positive folds.
  for fold_name in &["fold-A", "fold-B"] {
    assert_only_audit_differs(
      get(&v0_2_run, fold_name),
      get(&v0_3_run, fold_name),
      &format!("positive {}", fold_name),
    );
    assert!(is_null(get_path(&v0_2_run, &[fold_name, "audit"])));
    assert!(!is_null(get_path(&v0_3_run, &[fold_name, "audit"])));
  }

  // (b) negative folds.
  for fold_name in &["fold-unknown", "fold-bareConstraint"] {
    assert_only_audit_differs(
      get(&v0_2_neg, fold_name),
      get(&v0_3_neg, fold_name),
      &format!("negative {}", fold_name),
    );
    assert!(is_null(get_path(&v0_2_neg, &[fold_name, "audit"])));
    assert!(!is_null(get_path(&v0_3_neg, &[fold_name, "audit"])));
  }
}

#[test]
fn v0_3_six_layer_closure_marker_load_bearing() {
  // Invariant 339.
  let run = eval_file(&v0_3_run_path()).unwrap();
  let layers = get(&run, "layers");
  // ALL 6 layer flags must be true now.
  assert!(as_bool(get(layers, "surface-active")));
  assert!(as_bool(get(layers, "ontology-active")));
  assert!(as_bool(get(layers, "semantic-active")));
  assert!(as_bool(get(layers, "gate-active")));
  assert!(as_bool(get(layers, "runtime-active")));
  assert!(
    as_bool(get(layers, "audit-active")),
    "v0.3 MUST flip layers.audit-active = true (6-layer closure)"
  );
  assert_eq!(
    as_str(get(&run, "v0-marker")),
    "minimal-ontology-tesseract-v0.3"
  );
}

#[test]
fn v0_3_strict_import_whitelist() {
  // Invariant 340.
  fn collect_imports(content: &str) -> BTreeSet<String> {
    let mut imports = BTreeSet::new();
    for line in content.lines() {
      let trimmed = line.trim_start();
      if trimmed.starts_with('#') {
        continue;
      }
      if let Some(idx) = trimmed.find("import ") {
        let rest = &trimmed[idx + "import ".len()..];
        let tok = rest
          .split_whitespace()
          .next()
          .unwrap_or("")
          .trim_end_matches(';')
          .trim();
        if !tok.is_empty() {
          imports.insert(tok.to_string());
        }
      }
    }
    imports
  }

  let run_path = fixture_root().join("v0_3_run.px");
  let neg_path = fixture_root().join("v0_3_run_negative.px");
  let run_content = std::fs::read_to_string(&run_path).unwrap();
  let neg_content = std::fs::read_to_string(&neg_path).unwrap();

  let run_imports = collect_imports(&run_content);
  let neg_imports = collect_imports(&neg_content);

  let expected_run: BTreeSet<String> = ["./v0_2_run.px"].iter().map(|s| s.to_string()).collect();
  let expected_neg: BTreeSet<String> = ["./v0_2_run_negative.px", "./v0_3_run.px"]
    .iter()
    .map(|s| s.to_string())
    .collect();

  assert_eq!(
    run_imports, expected_run,
    "v0_3_run.px must import exactly {{./v0_2_run.px}}"
  );
  assert_eq!(
    neg_imports, expected_neg,
    "v0_3_run_negative.px must import exactly {{./v0_2_run_negative.px, ./v0_3_run.px}}"
  );
}
