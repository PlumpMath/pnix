//! minimal-ontology-tesseract-v0 — first slice of the parallel
//! ontology-fold lineage. Closes layers 1-3 of the canonical
//! 6-layer fold pipeline (surface → ontology → semantic).
//! gate / runtime / audit are explicitly null in v0 (deferred to
//! v0.1 / v0.2 / v0.3).
//!
//! Load-bearing claim:
//!   ontology classification of a surface form's path segments
//!   determines the semantic frame the form is folded into; the
//!   same surface form classified differently produces a different
//!   semantic normalization.
//!
//! Truth owner:        project-wiki/maps/minimal-ontology-tesseract-v0-map.md
//! Active scope:       project-wiki/maps/active-domain-constitution.md
//!                     Art. 1, Art. 6, Art. 7
//!
//! Test count: 17 invariants, indices 300..316, expected as 17
//! distinct `#[test]` functions (per map's pinned plan; 316
//! added by Codex proof-hygiene round to pin frame universe
//! exact 8 names).
//!
//! 300. fixture-local ontology has exactly 3 entries:
//!      triangle / area / min
//! 301. each ontology entry has the 5 required fields:
//!      kind / role / domain / capabilities / relations
//! 302. surface carrier preserves path-list and value verbatim
//!      for input A (`["triangle", "area", "min"]`, 0.001)
//! 303. surface carrier preserves path-list and value verbatim
//!      for input B (`["triangle", "area"]`, 0.5)
//! 304. ontology classification of input A emits exactly 3 kind
//!      bindings (triangle=GeometryObject, area=Measure,
//!      min=Constraint)
//! 305. ontology classification of input B emits exactly 2 kind
//!      bindings (triangle=GeometryObject, area=Measure)
//! 306. **load-bearing — frame discrimination**: input A's frame
//!      is "invariant"; input B's is "property"
//! 307. semantic normalizer prepends the frame segment to input
//!      A: `["invariant", "triangle", "area", "min"]`
//! 308. semantic normalizer prepends the frame segment to input
//!      B: `["property", "triangle", "area"]`
//! 309. semantic normalizer carries the value verbatim
//!      (no value coercion in v0)
//! 310. **load-bearing — gate / runtime / audit are all null**
//!      in both fold-A and fold-B
//! 311. **load-bearing — fold determinism (replay)**: re-running
//!      the fold on the same input produces normalized
//!      structural equality via `Value::to_json()`
//! 312. **load-bearing — Held on missing ontology**: input
//!      `unknown` (`["triangle", "perimeter", "min"]`) emits
//!      kind="ontology-unknown" with subject="perimeter" (first
//!      unknown segment in path order); semantic frame is null
//! 313. **load-bearing — Held on bare Constraint**: input
//!      `bareConstraint` (`["min"]`) emits exactly:
//!      `kind="semantic-frame-ambiguous"`, `subject="min"`,
//!      `candidate-frames=["invariant","goal"]`,
//!      `reason="Constraint requires a Measure target"`
//! 314. v0 fixture files do NOT import `v0_owner_law.px`
//!      (cross-lineage import deferred to v0.5)
//! 315. external-surfaces marker is all false (no LLM / VLM /
//!      network / p-puck / provider) AND import whitelist is
//!      strict: v0_run.px imports only ./v0_inputs.px and
//!      ./v0_ontology.px; v0_run_negative.px imports only
//!      ./v0_inputs.px and ./v0_run.px
//! 316. frame universe declared in v0_ontology.px has exactly
//!      8 names: object / relation / invariant / event /
//!      generator / property / measured-value / goal — and
//!      v0_run.px's `precedence-rule` output equals
//!      `"unknown-before-ambiguity"` (proves the runner reads
//!      the rule from data, not from a Rust-side constant)

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------
// Helpers.
// ---------------------------------------------------------------

fn fixture_root() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../../fixtures/minimal-ontology-tesseract-v0")
}

fn ontology_path() -> PathBuf {
  fixture_root().join("v0_ontology.px")
}

fn inputs_path() -> PathBuf {
  fixture_root().join("v0_inputs.px")
}

fn run_path() -> PathBuf {
  fixture_root().join("v0_run.px")
}

fn run_negative_path() -> PathBuf {
  fixture_root().join("v0_run_negative.px")
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

fn as_float(v: &Value) -> f64 {
  match v {
    Value::Float(f) => *f,
    Value::Int(i) => *i as f64,
    other => panic!("expected number, got {:?}", other),
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
fn v0_ontology_has_three_entries() {
  // Invariant 300.
  let onto_file = eval_file(&ontology_path()).expect("v0_ontology.px must evaluate");
  let ontology = as_attrs(get(&onto_file, "ontology"));
  let keys: BTreeSet<&str> = ontology.keys().map(|s| s.as_str()).collect();
  let expected: BTreeSet<&str> = ["triangle", "area", "min"].into_iter().collect();
  assert_eq!(
    keys, expected,
    "v0 ontology must have exactly 3 entries: triangle / area / min"
  );
}

#[test]
fn v0_each_ontology_entry_has_required_fields() {
  // Invariant 301.
  let onto_file = eval_file(&ontology_path()).unwrap();
  let ontology = as_attrs(get(&onto_file, "ontology"));
  let required = ["kind", "role", "domain", "capabilities", "relations"];
  for (entry_name, entry) in ontology {
    let attrs = as_attrs(entry);
    for field in &required {
      assert!(
        attrs.contains_key(*field),
        "ontology entry `{}` missing required field `{}`; have {:?}",
        entry_name,
        field,
        attrs.keys().collect::<Vec<_>>()
      );
    }
    // Type sanity for the structural fields.
    let _ = as_str(get(entry, "kind"));
    let _ = as_str(get(entry, "role"));
    let _ = as_str(get(entry, "domain"));
    let _ = as_list(get(entry, "capabilities"));
    let _ = as_list(get(entry, "relations"));
  }
}

#[test]
fn v0_surface_carrier_preserves_input_a() {
  // Invariant 302.
  let run = eval_file(&run_path()).unwrap();
  let surface = get_path(&run, &["fold-A", "surface"]);
  let path: Vec<&str> = list_strings(get(surface, "path"));
  assert_eq!(path, vec!["triangle", "area", "min"]);
  assert!((as_float(get(surface, "value")) - 0.001).abs() < 1e-12);
}

#[test]
fn v0_surface_carrier_preserves_input_b() {
  // Invariant 303.
  let run = eval_file(&run_path()).unwrap();
  let surface = get_path(&run, &["fold-B", "surface"]);
  let path: Vec<&str> = list_strings(get(surface, "path"));
  assert_eq!(path, vec!["triangle", "area"]);
  assert!((as_float(get(surface, "value")) - 0.5).abs() < 1e-12);
}

#[test]
fn v0_ontology_classification_input_a() {
  // Invariant 304.
  let run = eval_file(&run_path()).unwrap();
  let bindings = as_attrs(get_path(&run, &["fold-A", "ontology"]));
  assert_eq!(bindings.len(), 3, "input A must produce 3 kind bindings");
  assert_eq!(as_str(bindings.get("triangle").unwrap()), "GeometryObject");
  assert_eq!(as_str(bindings.get("area").unwrap()), "Measure");
  assert_eq!(as_str(bindings.get("min").unwrap()), "Constraint");
}

#[test]
fn v0_ontology_classification_input_b() {
  // Invariant 305.
  let run = eval_file(&run_path()).unwrap();
  let bindings = as_attrs(get_path(&run, &["fold-B", "ontology"]));
  assert_eq!(bindings.len(), 2, "input B must produce 2 kind bindings");
  assert_eq!(as_str(bindings.get("triangle").unwrap()), "GeometryObject");
  assert_eq!(as_str(bindings.get("area").unwrap()), "Measure");
  assert!(
    !bindings.contains_key("min"),
    "input B's path has no `min` segment, so no `min` binding"
  );
}

#[test]
fn v0_frame_discrimination_load_bearing() {
  // Invariant 306 — load-bearing.
  let run = eval_file(&run_path()).unwrap();
  let frame_a = as_str(get_path(&run, &["fold-A", "semantic", "frame"]));
  let frame_b = as_str(get_path(&run, &["fold-B", "semantic", "frame"]));
  assert_eq!(
    frame_a, "invariant",
    "input A (GeometryObject+Measure+Constraint) must fold to `invariant`"
  );
  assert_eq!(
    frame_b, "property",
    "input B (GeometryObject+Measure) must fold to `property`"
  );
  assert_ne!(
    frame_a, frame_b,
    "the two inputs must produce DIFFERENT frames — this is the central v0 claim that ontology classification changes the semantic frame"
  );
}

#[test]
fn v0_normalized_path_input_a() {
  // Invariant 307.
  let run = eval_file(&run_path()).unwrap();
  let normalized_path: Vec<&str> = list_strings(get_path(
    &run,
    &["fold-A", "semantic", "normalized", "path"],
  ));
  assert_eq!(
    normalized_path,
    vec!["invariant", "triangle", "area", "min"],
    "input A's normalized path must prepend `invariant` while preserving the original path"
  );
}

#[test]
fn v0_normalized_path_input_b() {
  // Invariant 308.
  let run = eval_file(&run_path()).unwrap();
  let normalized_path: Vec<&str> = list_strings(get_path(
    &run,
    &["fold-B", "semantic", "normalized", "path"],
  ));
  assert_eq!(
    normalized_path,
    vec!["property", "triangle", "area"],
    "input B's normalized path must prepend `property` while preserving the original path"
  );
}

#[test]
fn v0_value_carried_verbatim() {
  // Invariant 309.
  let run = eval_file(&run_path()).unwrap();
  let surf_a = as_float(get_path(&run, &["fold-A", "surface", "value"]));
  let norm_a = as_float(get_path(
    &run,
    &["fold-A", "semantic", "normalized", "value"],
  ));
  let surf_b = as_float(get_path(&run, &["fold-B", "surface", "value"]));
  let norm_b = as_float(get_path(
    &run,
    &["fold-B", "semantic", "normalized", "value"],
  ));
  assert!((surf_a - 0.001).abs() < 1e-12);
  assert!((norm_a - 0.001).abs() < 1e-12);
  assert!((surf_b - 0.5).abs() < 1e-12);
  assert!((norm_b - 0.5).abs() < 1e-12);
  assert_eq!(
    surf_a, norm_a,
    "v0 must carry value verbatim through layer 3"
  );
  assert_eq!(surf_b, norm_b);
}

#[test]
fn v0_gate_runtime_audit_all_null_load_bearing() {
  // Invariant 310 — load-bearing.
  let run = eval_file(&run_path()).unwrap();
  for fold_name in &["fold-A", "fold-B"] {
    let fold = get(&run, fold_name);
    for layer in &["gate", "runtime", "audit"] {
      let v = get(fold, layer);
      assert!(
        is_null(v),
        "v0 must keep `{}` null in `{}`; non-null would mean v0 leaked into v0.1+",
        layer,
        fold_name
      );
    }
  }
  // The runner-level layers attrset also pins it.
  let layers = get(&run, "layers");
  assert!(as_bool(get(layers, "surface-active")));
  assert!(as_bool(get(layers, "ontology-active")));
  assert!(as_bool(get(layers, "semantic-active")));
  assert!(!as_bool(get(layers, "gate-active")));
  assert!(!as_bool(get(layers, "runtime-active")));
  assert!(!as_bool(get(layers, "audit-active")));
}

#[test]
fn v0_fold_determinism_replay_load_bearing() {
  // Invariant 311 — load-bearing.
  // Two independent evaluations of the same fixture must
  // produce normalized structural equality via Value::to_json()
  // on the DATA emission keys only. `foldInput` is a
  // fixture-local helper export (lambda) and is NOT part of v0's
  // semantic emission — comparing it would conflate proof
  // surfaces.
  //
  // Data emission keys (per map §"Expected v0 emissions"):
  //   v0-marker / layers / external-surfaces / frame-universe /
  //   precedence-rule / fold-A / fold-B
  let run_one = eval_file(&run_path()).unwrap();
  let run_two = eval_file(&run_path()).unwrap();
  let data_keys = [
    "v0-marker",
    "layers",
    "external-surfaces",
    "frame-universe",
    "precedence-rule",
    "fold-A",
    "fold-B",
  ];
  for key in &data_keys {
    let lhs = get(&run_one, key);
    let rhs = get(&run_two, key);
    assert_eq!(
      lhs.to_json(),
      rhs.to_json(),
      "fold determinism: data emission key `{}` must be byte-for-byte identical across replays (normalized structural equality via Value::to_json)",
      key
    );
  }
}

#[test]
fn v0_held_on_missing_ontology_load_bearing() {
  // Invariant 312 — load-bearing.
  let neg = eval_file(&run_negative_path()).unwrap();
  let fold = get(&neg, "fold-unknown");
  // semantic frame must be null on the unknown path
  assert!(
    is_null(get_path(fold, &["semantic", "frame"])),
    "unknown-segment input must leave semantic.frame null (negative precedence: unknown wins)"
  );
  assert!(is_null(get_path(fold, &["semantic", "normalized"])));
  // Held emission must follow the deterministic rule
  let held = get(fold, "held");
  assert!(!is_null(held), "unknown input must emit a non-null Held");
  assert_eq!(as_str(get(held, "kind")), "ontology-unknown");
  assert_eq!(
    as_str(get(held, "subject")),
    "perimeter",
    "subject MUST be the first unknown segment in path order (deterministic)"
  );
}

#[test]
fn v0_held_on_bare_constraint_load_bearing() {
  // Invariant 313 — load-bearing.
  let neg = eval_file(&run_negative_path()).unwrap();
  let fold = get(&neg, "fold-bareConstraint");
  // semantic frame must be null
  assert!(is_null(get_path(fold, &["semantic", "frame"])));
  assert!(is_null(get_path(fold, &["semantic", "normalized"])));
  // Held emission must equal the exact deterministic shape
  let held = get(fold, "held");
  assert!(!is_null(held));
  assert_eq!(as_str(get(held, "kind")), "semantic-frame-ambiguous");
  assert_eq!(as_str(get(held, "subject")), "min");
  let candidate_frames: Vec<&str> = list_strings(get(held, "candidate-frames"));
  assert_eq!(
    candidate_frames,
    vec!["invariant", "goal"],
    "candidate-frames MUST be exactly [\"invariant\", \"goal\"] — not just non-null"
  );
  assert_eq!(
    as_str(get(held, "reason")),
    "Constraint requires a Measure target"
  );
}

#[test]
fn v0_does_not_import_owner_law() {
  // Invariant 314.
  // Concrete file-text scan: none of the four v0 fixture files
  // may contain `v0_owner_law` in any import-like position.
  // Even a comment mentioning the cross-lineage decision is
  // fine, but actual `import ./v0_owner_law` etc. must NOT
  // appear. We use a strict substring check because all
  // mentions in the v0 fixture should be in commentary about
  // NOT importing it.
  for path in [
    ontology_path(),
    inputs_path(),
    run_path(),
    run_negative_path(),
  ] {
    let content =
      std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {:?}: {}", path, e));
    // We allow the substring `v0_owner_law` to appear in
    // commentary (the map's "Non-import" rationale is referenced
    // in headers), but the strong rule is: no actual `import`
    // expression uses it. Look for any occurrence of
    // `import .*v0_owner_law` to catch any code-level import
    // regardless of formatting.
    // Skip comment lines (begin with `#` after optional
    // whitespace). Commentary about NOT importing v0_owner_law
    // is allowed in headers; only actual code-level imports
    // are forbidden.
    for line in content.lines() {
      let trimmed = line.trim_start();
      if trimmed.starts_with('#') {
        continue;
      }
      if trimmed.contains("v0_owner_law") && trimmed.contains("import") {
        panic!(
          "{:?} contains a non-comment `import ... v0_owner_law` expression; v0 must not import the owner-law (cross-lineage import deferred to v0.5). Offending line: `{}`",
          path,
          trimmed
        );
      }
      // Defensive: reject the bare `./v0_owner_law` import-path
      // token if it appears outside a comment line.
      if trimmed.contains("./v0_owner_law") {
        panic!(
          "{:?} contains a non-comment reference to `./v0_owner_law`; v0 must not import the owner-law",
          path
        );
      }
    }
  }
}

#[test]
fn v0_external_surfaces_all_false() {
  // Invariant 315.
  let run = eval_file(&run_path()).unwrap();
  let marker = get(&run, "external-surfaces");
  let attrs = as_attrs(marker);
  let required = ["llm", "vlm", "network", "p-puck", "provider"];
  for key in &required {
    let v = attrs
      .get(*key)
      .unwrap_or_else(|| panic!("external-surfaces missing `{}`", key));
    assert!(
      !as_bool(v),
      "external-surfaces.`{}` MUST be false in v0; non-false would indicate the v0 emission path leaked into a forbidden surface",
      key
    );
  }
  // Negative runner must mirror the same marker.
  let neg = eval_file(&run_negative_path()).unwrap();
  let neg_marker = get(&neg, "external-surfaces");
  for key in &required {
    let v = as_attrs(neg_marker)
      .get(*key)
      .unwrap_or_else(|| panic!("negative runner external-surfaces missing `{}`", key));
    assert!(!as_bool(v));
  }

  // Strict import whitelist: collect every import target on
  // non-comment lines, then assert exact match.
  fn collect_imports(content: &str) -> BTreeSet<String> {
    let mut imports = BTreeSet::new();
    for line in content.lines() {
      let trimmed = line.trim_start();
      if trimmed.starts_with('#') {
        continue;
      }
      if let Some(idx) = trimmed.find("import ") {
        let rest = &trimmed[idx + "import ".len()..];
        // Take the next whitespace-delimited token; strip a
        // trailing `;` if present.
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

  let run_content = std::fs::read_to_string(run_path()).unwrap();
  let neg_content = std::fs::read_to_string(run_negative_path()).unwrap();

  let run_imports = collect_imports(&run_content);
  let neg_imports = collect_imports(&neg_content);

  let expected_run: BTreeSet<String> = ["./v0_inputs.px", "./v0_ontology.px"]
    .iter()
    .map(|s| s.to_string())
    .collect();
  let expected_neg: BTreeSet<String> = ["./v0_inputs.px", "./v0_run.px"]
    .iter()
    .map(|s| s.to_string())
    .collect();

  assert_eq!(
    run_imports, expected_run,
    "v0_run.px must import exactly {{./v0_inputs.px, ./v0_ontology.px}} — no external surfaces"
  );
  assert_eq!(
    neg_imports, expected_neg,
    "v0_run_negative.px must import exactly {{./v0_inputs.px, ./v0_run.px}} — no external surfaces"
  );
}

#[test]
fn v0_frame_universe_and_precedence_rule_data_read() {
  // Invariant 316.
  // Two-part check:
  //   (a) frame universe declared in v0_ontology.px has exactly
  //       the 8 canonical names. This pins the universe
  //       boundary at v0; any drift (rename / add / remove) will
  //       fail the test.
  //   (b) v0_run.px's `precedence-rule` output equals the rule
  //       string declared in v0_ontology.px. This proves the
  //       runner reads the rule from data — if the runner ever
  //       hardcodes the rule check on a different string, the
  //       output would diverge from the declared value and this
  //       test fails.

  // Part (a): frame universe.
  let onto_file = eval_file(&ontology_path()).unwrap();
  let universe: Vec<&str> = list_strings(get(&onto_file, "frameUniverse"));
  let expected = vec![
    "object",
    "relation",
    "invariant",
    "event",
    "generator",
    "property",
    "measured-value",
    "goal",
  ];
  assert_eq!(
    universe, expected,
    "frame universe in v0_ontology.px must be exactly the 8 canonical names in declared order"
  );

  // Same exact universe must surface on the runner output.
  let run = eval_file(&run_path()).unwrap();
  let run_universe: Vec<&str> = list_strings(get(&run, "frame-universe"));
  assert_eq!(run_universe, expected);

  // Part (b): precedence rule data-read.
  let onto_rule = as_str(get_path(&onto_file, &["negativePrecedence", "rule"]));
  let run_rule = as_str(get(&run, "precedence-rule"));
  assert_eq!(
    onto_rule, "unknown-before-ambiguity",
    "v0_ontology.px must declare the precedence rule literal `unknown-before-ambiguity`"
  );
  assert_eq!(
    run_rule, onto_rule,
    "v0_run.px's `precedence-rule` output must equal the rule string read from v0_ontology.px (proves data-driven rule consumption, not Rust-side constant)"
  );
}
