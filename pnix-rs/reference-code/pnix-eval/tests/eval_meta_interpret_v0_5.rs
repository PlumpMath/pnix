//! minimal-ontology-tesseract-v0.5 — metaInterpret +
//! SpecializedInterpreter formalization. Two-lineage
//! convergence: minimal-tesseract-v0 (owner-law / trajectory)
//! + minimal-ontology-tesseract-v0 (6-layer fold) → same
//! SpecializedInterpreter instance.
//!
//! Load-bearing claim:
//!   metaInterpret(active context) → SpecializedInterpreter is
//!   deterministic; different context → different SI id;
//!   SI.rule-functions exposes v0_owner_law.px's 7 builders
//!   (FIRST cross-lineage import in this lineage);
//!   SI.interpret(input) is byte-equal to v0.3's 6-layer fold
//!   for every input (Stage 2 cut); both surfaces coexist in
//!   one interpreter instance.
//!
//! Truth owner:        project-wiki/maps/minimal-ontology-tesseract-v0-map.md
//!                     §"v0.5 design decision — metaInterpret +
//!                       SpecializedInterpreter formalization"
//! Active scope:       project-wiki/maps/active-domain-constitution.md
//!                     Art. 6, Art. 7
//!
//! Test count: 10 invariants, indices 341..350.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------
// Helpers — same shape as v0..v0.3 test files.
// ---------------------------------------------------------------

fn v0_5_root() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../../fixtures/meta-interpret-v0_5")
}

fn v0_3_root() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../../fixtures/minimal-ontology-tesseract-v0")
}

fn context_path() -> PathBuf {
  v0_5_root().join("v0_5_context.px")
}

fn meta_interpret_path() -> PathBuf {
  v0_5_root().join("v0_5_meta_interpret.px")
}

fn run_path() -> PathBuf {
  v0_5_root().join("v0_5_run.px")
}

fn run_negative_path() -> PathBuf {
  v0_5_root().join("v0_5_run_negative.px")
}

fn v0_3_run_path() -> PathBuf {
  v0_3_root().join("v0_3_run.px")
}

fn v0_3_run_negative_path() -> PathBuf {
  v0_3_root().join("v0_3_run_negative.px")
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

// ---------------------------------------------------------------
// Tests.
// ---------------------------------------------------------------

#[test]
fn v0_5_canonical_context_full_seven_key_shape() {
  // Invariant 341.
  let ctx_file = eval_file(&context_path()).unwrap();
  let canonical = get(&ctx_file, "canonical");
  let keys: BTreeSet<&str> = as_attrs(canonical).keys().map(|s| s.as_str()).collect();
  let expected: BTreeSet<&str> = [
    "active-lens-set",
    "capability-registry",
    "need-cursor",
    "role-tags",
    "rigor-floor",
    "source-kind",
    "owner-context",
  ]
  .into_iter()
  .collect();
  assert_eq!(
    keys, expected,
    "canonical context must have exactly the 7 required keys"
  );
  assert_eq!(
    as_str(get(canonical, "source-kind")),
    "minimal-ontology-tesseract"
  );
  let owner_ctx = get(canonical, "owner-context");
  let _ = as_str(get(owner_ctx, "owner"));
  let _ = as_str(get(owner_ctx, "version"));
  let _ = as_str(get(owner_ctx, "digest"));
}

#[test]
fn v0_5_specialized_interpreter_nine_key_shape() {
  // Invariant 342.
  let run = eval_file(&run_path()).unwrap();
  let si = get(&run, "specialized-interpreter");
  let keys: BTreeSet<&str> = as_attrs(si).keys().map(|s| s.as_str()).collect();
  // SI shape per meta-interpret-instance-map.md "Minimum Shape":
  // 9 keys for the metadata frame; v0_5_run.px also attaches
  // `interpret` lambda → so SI exposed under
  // `specialized-interpreter` has 10 keys (9 metadata + interpret).
  let expected: BTreeSet<&str> = [
    "id",
    "source-kind",
    "active-lens-set",
    "visible-capabilities",
    "active-need",
    "rule-functions",
    "expected-receipt-shape",
    "forbidden-actions",
    "owner-refs",
    "interpret",
  ]
  .into_iter()
  .collect();
  assert_eq!(
    keys, expected,
    "specialized-interpreter must have the 9 metadata keys plus the `interpret` lambda attached by Stage 2"
  );
  let _ = as_str(get(si, "id"));
  let _ = as_str(get(si, "source-kind"));
  let _ = as_list(get(si, "active-lens-set"));
  let _ = as_list(get(si, "visible-capabilities"));
  let _ = as_str(get(si, "active-need"));
  let _ = as_attrs(get(si, "rule-functions"));
  let _ = as_attrs(get(si, "expected-receipt-shape"));
  let _ = as_list(get(si, "forbidden-actions"));
  let _ = as_list(get(si, "owner-refs"));
  assert!(
    matches!(get(si, "interpret"), Value::Lambda { .. }),
    "interpret must be a Lambda (Stage 2 wiring)"
  );
}

#[test]
fn v0_5_same_context_same_si_id_load_bearing() {
  // Invariant 343.
  let run_a = eval_file(&run_path()).unwrap();
  let run_b = eval_file(&run_path()).unwrap();
  let id_a = as_str(get_path(&run_a, &["specialized-interpreter", "id"]));
  let id_b = as_str(get_path(&run_b, &["specialized-interpreter", "id"]));
  assert_eq!(
    id_a, id_b,
    "metaInterpret on the same context must produce the same SpecializedInterpreter id (replay determinism)"
  );
  // Spot-check the deterministic format.
  assert!(id_a.starts_with("si|minimal-ontology-tesseract|"));
  assert!(id_a.contains("owner:v0.3-pipeline-closure"));
  assert!(id_a.contains("need:need.fold-input"));
  assert!(id_a.contains("lens:0"));
  assert!(id_a.contains("roles:0"));
}

#[test]
fn v0_5_different_context_different_si_id_load_bearing() {
  // Invariant 344.
  let neg = eval_file(&run_negative_path()).unwrap();
  let canonical_id = as_str(get(&neg, "canonical-si-id"));
  let delta_id = as_str(get(&neg, "delta-si-id"));
  assert_ne!(
    canonical_id, delta_id,
    "different context must produce different SpecializedInterpreter id"
  );
  // The delta differs only on owner-context.version, so the id
  // delta should be localized to the owner: segment.
  assert!(canonical_id.contains("owner:v0.3-pipeline-closure"));
  assert!(delta_id.contains("owner:v0.3-pipeline-closure-delta"));
  // All other segments should match (same source-kind / need /
  // lens / roles).
  for segment in [
    "si",
    "minimal-ontology-tesseract",
    "need:need.fold-input",
    "lens:0",
    "roles:0",
  ] {
    assert!(canonical_id.contains(segment));
    assert!(delta_id.contains(segment));
  }
}

#[test]
fn v0_5_rule_functions_owner_law_seven_builders_load_bearing() {
  // Invariant 345 — the load-bearing two-lineage convergence claim.
  let run = eval_file(&run_path()).unwrap();
  let rule_fns = get_path(&run, &["specialized-interpreter", "rule-functions"]);
  let attrs = as_attrs(rule_fns);
  let expected_builders = [
    "buildAttachTurn",
    "buildCompareTurn",
    "buildRepairTurn",
    "buildLensCompareResult",
    "buildHeldEntry",
    "buildRepairCandidate",
    "buildMetaCircularLogDifferential",
  ];
  let actual: BTreeSet<&str> = attrs.keys().map(|s| s.as_str()).collect();
  let expected: BTreeSet<&str> = expected_builders.iter().copied().collect();
  assert_eq!(
    actual, expected,
    "rule-functions must contain exactly the 7 owner-law builders (cross-lineage convergence)"
  );
  for builder_name in expected_builders {
    let entry = attrs.get(builder_name).unwrap();
    assert!(
      matches!(entry, Value::Lambda { .. }),
      "rule-functions.{} must be a Lambda (imported from v0_owner_law.px)",
      builder_name
    );
  }
  // SI.owner-refs must reference the owner-law file path
  // (the convergence anchor).
  let owner_refs: Vec<&str> =
    list_strings(get_path(&run, &["specialized-interpreter", "owner-refs"]));
  assert_eq!(owner_refs.len(), 1);
  assert_eq!(
    owner_refs[0], "fixtures/minimal-tesseract-v0/v0_owner_law.px",
    "owner-refs must point to the owner-law file (cross-lineage anchor)"
  );
}

#[test]
fn v0_5_interpret_input_a_byte_equal_v0_3_load_bearing() {
  // Invariant 346.
  let run = eval_file(&run_path()).unwrap();
  let v0_3 = eval_file(&v0_3_run_path()).unwrap();
  let v0_5_fold_a = get(&run, "fold-A");
  let v0_3_fold_a = get(&v0_3, "fold-A");
  assert_eq!(
    v0_5_fold_a.to_json(),
    v0_3_fold_a.to_json(),
    "SpecializedInterpreter.interpret(A) must be byte-equal to v0.3's fold-A via Value::to_json (Stage 2 cut)"
  );
}

#[test]
fn v0_5_interpret_input_b_byte_equal_v0_3_load_bearing() {
  // Invariant 347.
  let run = eval_file(&run_path()).unwrap();
  let v0_3 = eval_file(&v0_3_run_path()).unwrap();
  let v0_5_fold_b = get(&run, "fold-B");
  let v0_3_fold_b = get(&v0_3, "fold-B");
  assert_eq!(
    v0_5_fold_b.to_json(),
    v0_3_fold_b.to_json(),
    "SpecializedInterpreter.interpret(B) must be byte-equal to v0.3's fold-B"
  );
}

#[test]
fn v0_5_interpret_negative_byte_equal_v0_3_load_bearing() {
  // Invariant 348.
  let neg = eval_file(&run_negative_path()).unwrap();
  let v0_3_neg = eval_file(&v0_3_run_negative_path()).unwrap();

  let v0_5_unknown = get(&neg, "fold-unknown");
  let v0_3_unknown = get(&v0_3_neg, "fold-unknown");
  assert_eq!(
    v0_5_unknown.to_json(),
    v0_3_unknown.to_json(),
    "SI.interpret(unknown) must be byte-equal to v0.3's fold-unknown (Held + audit preserved verbatim)"
  );

  let v0_5_bare = get(&neg, "fold-bareConstraint");
  let v0_3_bare = get(&v0_3_neg, "fold-bareConstraint");
  assert_eq!(
    v0_5_bare.to_json(),
    v0_3_bare.to_json(),
    "SI.interpret(bareConstraint) must be byte-equal to v0.3's fold-bareConstraint"
  );

  // candidate-frames preservation spot-check (the load-bearing
  // detail from v0.3 / Codex 2026-05-06 directive D).
  let cf: Vec<&str> = list_strings(get_path(
    &neg,
    &[
      "fold-bareConstraint",
      "audit",
      "failure-reason",
      "candidate-frames",
    ],
  ));
  assert_eq!(cf, vec!["invariant", "goal"]);
}

#[test]
fn v0_5_forbidden_actions_exact() {
  // Invariant 349.
  let run = eval_file(&run_path()).unwrap();
  let forbidden: Vec<&str> = list_strings(get_path(
    &run,
    &["specialized-interpreter", "forbidden-actions"],
  ));
  let expected = vec!["oracle", "llm", "network", "p-puck", "provider"];
  assert_eq!(
    forbidden, expected,
    "forbidden-actions MUST be exactly [oracle, llm, network, p-puck, provider] in declared order"
  );
}

#[test]
fn v0_5_two_lineage_convergence_marker_load_bearing() {
  // Invariant 350.
  let run = eval_file(&run_path()).unwrap();
  let layers = get(&run, "layers");
  // ALL six v0.3 fold layer flags must still be true.
  for layer in &[
    "surface-active",
    "ontology-active",
    "semantic-active",
    "gate-active",
    "runtime-active",
    "audit-active",
  ] {
    assert!(
      as_bool(get(layers, layer)),
      "layers.{} must remain true in v0.5 (6-layer pipeline closure preserved)",
      layer
    );
  }
  // v0.5-specific layer markers.
  assert!(
    as_bool(get(layers, "meta-interpret-active")),
    "v0.5 MUST flip layers.meta-interpret-active = true"
  );
  assert!(
    as_bool(get(layers, "specialized-interpreter-built")),
    "v0.5 MUST flip layers.specialized-interpreter-built = true"
  );
  // Cross-lineage anchor — SI.owner-refs references owner-law
  // (already asserted in 345; here it's asserted as the
  // convergence-marker companion check).
  let owner_refs: Vec<&str> =
    list_strings(get_path(&run, &["specialized-interpreter", "owner-refs"]));
  assert!(owner_refs.iter().any(|r| r.contains("v0_owner_law.px")));
  // Source-kind anchors the minimal-ontology-tesseract-v0 lineage.
  assert_eq!(
    as_str(get_path(&run, &["specialized-interpreter", "source-kind"])),
    "minimal-ontology-tesseract"
  );
  // meta-interpret-version marker.
  assert_eq!(as_str(get(&run, "meta-interpret-version")), "0.5");
}
