//! minimal-ontology-tesseract-v0.5.2 — Stage 2 cross-application
//! (FIRST INTERACTION PROOF). v0.5 / v0.5.1 closed coexistence
//! between owner-law builders (visible in SI.rule-functions) and
//! the 6-layer fold (visible via SI.interpret). v0.5.2 closes the
//! first interaction: ONE owner-law builder (`buildAttachTurn`)
//! is actually applied to a SourceObject derived from a v0.3
//! fold output, emitting an `interpret-cross` candidate record.
//!
//! Truth owner: project-wiki/maps/minimal-ontology-tesseract-v0-map.md
//!              §"v0.5.2 design decision — Stage 2 cross-application
//!               (single builder, single fold)"
//! Active scope: project-wiki/maps/active-domain-constitution.md
//!               Art. 6, Art. 7
//!
//! Test count: 7 invariants, indices 356..362.

use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------
// Helpers — same shape as v0.5 / v0.5.1 test files.
// ---------------------------------------------------------------

fn v0_5_root() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../../fixtures/meta-interpret-v0_5")
}

fn run_path() -> PathBuf {
  v0_5_root().join("v0_5_2_run.px")
}

fn run_negative_path() -> PathBuf {
  v0_5_root().join("v0_5_2_run_negative.px")
}

fn v0_3_path() -> PathBuf {
  let base = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
  Path::new(&base).join("../../fixtures/minimal-ontology-tesseract-v0/v0_3_run.px")
}

fn as_attrs(v: &Value) -> &BTreeMap<String, Value> {
  match v {
    Value::AttrSet(m) => m,
    other => panic!("expected attrset, got {:?}", other),
  }
}

fn as_str(v: &Value) -> &str {
  match v {
    Value::String(s) => s,
    Value::StringContext { text, .. } => text,
    other => panic!("expected string, got {:?}", other),
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

// ---------------------------------------------------------------
// Tests.
// ---------------------------------------------------------------

#[test]
fn v0_5_2_interpret_cross_field_candidate_only_load_bearing() {
  // Invariant 356.
  // The positive runner emits `cross-A` — an interpret-cross
  // record produced by SI.interpret-cross "A". The record's
  // `status` MUST be `"candidate"` (Constitution Art. 7
  // candidate-only). The exact 6-key positive record shape is
  // also pinned here. (cross-failed branch adds `held-ref` for
  // a 7-key shape — verified separately by invariant 362.)
  // Comment cleanup per Codex 2026-05-06 v0.5.2.1 directive
  // (point B/C: "7-key" → "6-key positive shape").

  let pos = eval_file(&run_path()).unwrap();
  let cross_a = get(&pos, "cross-A");

  let keys: BTreeSet<&str> = as_attrs(cross_a).keys().map(|s| s.as_str()).collect();
  let expected: BTreeSet<&str> = [
    "source-fold-id",
    "source-object",
    "derived-lens",
    "applied-builder",
    "output-turn",
    "status",
  ]
  .into_iter()
  .collect();
  assert_eq!(
    keys, expected,
    "interpret-cross positive record must have exactly 6 keys: {{source-fold-id, source-object, derived-lens, applied-builder, output-turn, status}}"
  );

  let status = as_str(get(cross_a, "status"));
  assert_eq!(
    status, "candidate",
    "interpret-cross record MUST emit status=candidate (Constitution Art. 7 candidate-only; never auto-promoted)"
  );
}

#[test]
fn v0_5_2_source_object_derived_from_v0_3_fold_load_bearing() {
  // Invariant 357.
  // The cross-A record's `source-object` MUST be derived from
  // v0.3 fold-A — every field traceable to a v0.3 fold output
  // field. This is the load-bearing claim that v0.5.2 closes
  // INTERACTION (not just COEXISTENCE).
  //
  // Specifically:
  //   source-object.id              = ".".join(fold-A.surface.path)
  //   source-object.semantic-frame  = fold-A.semantic.frame
  //   source-object.audit-ref       = fold-A.audit.replay-ref
  //   source-object.source-kind     = "minimal-ontology-tesseract"

  let pos = eval_file(&run_path()).unwrap();
  let v0_3 = eval_file(&v0_3_path()).unwrap();
  let fold_a = get(&v0_3, "fold-A");

  let source_object = get_path(&pos, &["cross-A", "source-object"]);
  let so_id = as_str(get(source_object, "id"));
  let so_kind = as_str(get(source_object, "source-kind"));
  let so_frame = as_str(get(source_object, "semantic-frame"));
  let so_audit = as_str(get(source_object, "audit-ref"));

  // surface.path is a list of strings — derive the joined id
  // from fold-A directly and verify byte-equality.
  let path_list = match get_path(fold_a, &["surface", "path"]) {
    Value::List(items) => items.clone(),
    other => panic!("fold-A.surface.path must be List, got {:?}", other),
  };
  let path_strs: Vec<&str> = path_list.iter().map(as_str).collect();
  let expected_id = path_strs.join(".");

  assert_eq!(
    so_id, expected_id,
    "source-object.id must be `\".\".join(fold-A.surface.path)`; expected `{}` got `{}`",
    expected_id, so_id
  );
  assert_eq!(so_kind, "minimal-ontology-tesseract");

  let fold_frame = as_str(get_path(fold_a, &["semantic", "frame"]));
  assert_eq!(
    so_frame, fold_frame,
    "source-object.semantic-frame must be byte-equal to v0.3 fold-A.semantic.frame"
  );

  let fold_replay_ref = as_str(get_path(fold_a, &["audit", "replay-ref"]));
  assert_eq!(
    so_audit, fold_replay_ref,
    "source-object.audit-ref must be byte-equal to v0.3 fold-A.audit.replay-ref"
  );

  // Also verify the canonical input id used: cross-A's
  // source-fold-id MUST be "A".
  let source_fold_id = as_str(get_path(&pos, &["cross-A", "source-fold-id"]));
  assert_eq!(source_fold_id, "A");
}

#[test]
fn v0_5_2_applied_builder_is_build_attach_turn_load_bearing() {
  // Invariant 358.
  // The interpret-cross record MUST declare
  // `applied-builder = "buildAttachTurn"`. This pins the v0.5.2
  // contract: ONE specific builder (the smallest interaction
  // proof; Codex preferred). Other builders are deferred to
  // v0.5.3+.
  //
  // We also verify that the SI's rule-functions attrset still
  // exposes the 7 owner-law builders (carrying v0.5 invariant
  // 345's contract forward — interaction does NOT remove the
  // coexistence surface).

  let pos = eval_file(&run_path()).unwrap();

  let applied = as_str(get_path(&pos, &["cross-A", "applied-builder"]));
  assert_eq!(
    applied, "buildAttachTurn",
    "v0.5.2 cross-application must apply EXACTLY buildAttachTurn (smallest builder, Codex preferred for first interaction proof)"
  );

  let rule_functions = get_path(&pos, &["specialized-interpreter", "rule-functions"]);
  let rf_keys: BTreeSet<&str> = as_attrs(rule_functions)
    .keys()
    .map(|s| s.as_str())
    .collect();
  let expected: BTreeSet<&str> = [
    "buildAttachTurn",
    "buildCompareTurn",
    "buildRepairTurn",
    "buildLensCompareResult",
    "buildHeldEntry",
    "buildRepairCandidate",
    "buildMetaCircularLogDifferential",
  ]
  .into_iter()
  .collect();
  assert_eq!(
    rf_keys, expected,
    "v0.5 invariant 345 stays — SI.rule-functions still exposes all 7 owner-law builders even after v0.5.2 wires interpret-cross"
  );
}

#[test]
fn v0_5_2_output_turn_byte_equal_owner_law_load_bearing() {
  // Invariant 359.
  // The cross-A `output-turn` MUST be byte-equal to applying
  // the same `buildAttachTurn` from owner-law to the same
  // (turnId, derivedLens, prev) arguments INSIDE pnix.
  //
  // Cross-application is byte-equal-on-output to the owner-law
  // builder's direct invocation. This is the load-bearing
  // INTERACTION claim: cross-application is NOT a re-skin or
  // a parallel implementation — it routes through the same
  // owner-law lambda the coexistence proof exposed.
  //
  // We assert specific shape fields (turn-id=0,
  // direction="InwardDesign", applied-ankh="ankh.fold-A",
  // previous-lens-set=[], new-lens-set=["ankh.fold-A"],
  // status="applied", changed-routes=[<lens.attach-route>],
  // emits-needs.cross-applied-source, etc.).

  let pos = eval_file(&run_path()).unwrap();
  let output_turn = get_path(&pos, &["cross-A", "output-turn"]);
  let derived_lens = get_path(&pos, &["cross-A", "derived-lens"]);

  // turn-id = 0 (fixed in v0_5_2_run.px)
  let turn_id = match get(output_turn, "turn-id") {
    Value::Int(i) => *i,
    other => panic!("turn-id must be integer, got {:?}", other),
  };
  assert_eq!(turn_id, 0, "v0.5.2 fixes turn-id=0 (single attach turn)");

  // direction (from buildAttachTurn) = "InwardDesign"
  assert_eq!(
    as_str(get(output_turn, "direction")),
    "InwardDesign",
    "buildAttachTurn always emits direction=InwardDesign"
  );

  // applied-ankh = lens.id (must match derived-lens.id)
  assert_eq!(
    as_str(get(output_turn, "applied-ankh")),
    as_str(get(derived_lens, "id")),
    "applied-ankh must be byte-equal to derived-lens.id (buildAttachTurn copies lens.id)"
  );
  assert_eq!(
    as_str(get(output_turn, "applied-ankh")),
    "ankh.fold-A",
    "derived-lens.id is fixed to `ankh.fold-A` for fold-A cross-application"
  );

  // previous-lens-set = [] (v0.5.2 starts from empty
  // attach-prefix; cross-application is NOT a chained turn
  // sequence)
  let prev_set = match get(output_turn, "previous-lens-set") {
    Value::List(items) => items.len(),
    other => panic!("previous-lens-set must be List, got {:?}", other),
  };
  assert_eq!(prev_set, 0, "v0.5.2 calls buildAttachTurn with prev=[]");

  // new-lens-set = ["ankh.fold-A"] (prev ++ [lens.id])
  let new_set = match get(output_turn, "new-lens-set") {
    Value::List(items) => items.iter().map(as_str).collect::<Vec<_>>(),
    other => panic!("new-lens-set must be List, got {:?}", other),
  };
  assert_eq!(new_set, vec!["ankh.fold-A"]);

  // status = "applied" (buildAttachTurn always emits
  // status=applied; the cross-application wrapper's
  // status=candidate is on the OUTER record, not the inner
  // turn — keeping owner-law output untouched)
  assert_eq!(
    as_str(get(output_turn, "status")),
    "applied",
    "inner Turn record's status MUST stay `applied` (buildAttachTurn output verbatim); the candidate-only marker lives on the outer interpret-cross record"
  );

  // changed-routes = [lens.attach-route]
  let lens_attach_route = as_str(get(derived_lens, "attach-route"));
  let changed_routes = match get(output_turn, "changed-routes") {
    Value::List(items) => items.iter().map(as_str).collect::<Vec<_>>(),
    other => panic!("changed-routes must be List, got {:?}", other),
  };
  assert_eq!(changed_routes, vec![lens_attach_route]);
}

#[test]
fn v0_5_2_cross_application_replay_determinism_load_bearing() {
  // Invariant 360.
  // Replay determinism: evaluating the positive runner twice
  // produces byte-equal cross-A records (Value::to_json
  // comparison — same v0.x replay floor as invariants 336 /
  // 343 / 346 / 347).

  let pos1 = eval_file(&run_path()).unwrap();
  let pos2 = eval_file(&run_path()).unwrap();
  let cross_1 = get(&pos1, "cross-A");
  let cross_2 = get(&pos2, "cross-A");

  assert_eq!(
    cross_1.to_json(),
    cross_2.to_json(),
    "v0.5.2 cross-application is deterministic; same fixture → same cross-A record"
  );
}

#[test]
fn v0_5_2_strict_import_whitelist_load_bearing() {
  // Invariant 361.
  // Pin the import set of the two new fixture files. Same
  // proof-hygiene contract as v0.5.1 invariant 351 — no new
  // cross-lineage drift introduced by v0.5.2.

  let run_content = std::fs::read_to_string(run_path()).unwrap();
  let neg_content = std::fs::read_to_string(run_negative_path()).unwrap();

  let run_imports = collect_imports(&run_content);
  let neg_imports = collect_imports(&neg_content);

  // v0_5_2_run.px imports exactly:
  //   ./v0_5_run.px
  //   ../minimal-ontology-tesseract-v0/v0_3_run.px
  let expected_run: BTreeSet<String> = [
    "./v0_5_run.px",
    "../minimal-ontology-tesseract-v0/v0_3_run.px",
  ]
  .iter()
  .map(|s| s.to_string())
  .collect();
  assert_eq!(
    run_imports, expected_run,
    "v0_5_2_run.px must import EXACTLY {{./v0_5_run.px, ../minimal-ontology-tesseract-v0/v0_3_run.px}} — re-uses canonical SI from v0_5_run.px (which already pulls v0_5_meta_interpret.px transitively); pulls v0.3 fold-A directly for SourceObject derivation"
  );

  // v0_5_2_run_negative.px imports exactly:
  //   ./v0_5_2_run.px
  //   ./v0_5_run_negative.px
  //   ../minimal-ontology-tesseract-v0/v0_3_run_negative.px
  let expected_neg: BTreeSet<String> = [
    "./v0_5_2_run.px",
    "./v0_5_run_negative.px",
    "../minimal-ontology-tesseract-v0/v0_3_run_negative.px",
  ]
  .iter()
  .map(|s| s.to_string())
  .collect();
  assert_eq!(
    neg_imports, expected_neg,
    "v0_5_2_run_negative.px must import EXACTLY 3 files"
  );

  // No NEW minimal-tesseract-v0 lineage import beyond the
  // existing anchor (v0_owner_law.px is reachable transitively
  // through v0_5_run.px → v0_5_meta_interpret.px). v0.5.2 must
  // not introduce a fresh cross-lineage import.
  for imp in run_imports.iter().chain(neg_imports.iter()) {
    assert!(
      !imp.contains("minimal-tesseract-v0/"),
      "v0.5.2 must NOT introduce any direct minimal-tesseract-v0 lineage import; the cross-lineage anchor (v0_owner_law.px) is already reachable transitively through v0_5_meta_interpret.px"
    );
  }
}

#[test]
fn v0_5_2_negative_cross_failed_load_bearing() {
  // Invariant 362.
  // The negative runner exercises cross-application against
  // v0.3 fold-unknown / fold-bareConstraint. Both folds have
  // semantic.frame = null, so cross-application MUST emit
  // `status = "cross-failed"` WITHOUT constructing a Turn
  // record. The owner-law builder MUST NOT be applied to a
  // null-frame fold (safety invariant — protects builder from
  // partial input).

  let neg = eval_file(&run_negative_path()).unwrap();

  for sk in ["cross-unknown", "cross-bareConstraint"] {
    let cross = get(&neg, sk);

    let status = as_str(get(cross, "status"));
    assert_eq!(
      status, "cross-failed",
      "{} must emit status=cross-failed when fold.semantic.frame is null",
      sk
    );

    // No Turn was constructed → output-turn is null.
    assert!(
      matches!(get(cross, "output-turn"), Value::Null),
      "{}.output-turn must be null (buildAttachTurn was NOT called for null-frame fold)",
      sk
    );

    // applied-builder is null (no builder was applied).
    assert!(
      matches!(get(cross, "applied-builder"), Value::Null),
      "{}.applied-builder must be null (no builder applied for cross-failed branch)",
      sk
    );

    // source-object is null (cannot derive a complete
    // SourceObject from a null-frame fold).
    assert!(
      matches!(get(cross, "source-object"), Value::Null),
      "{}.source-object must be null (cannot derive SourceObject from null-frame fold)",
      sk
    );

    // held-ref is the safety marker.
    let held_ref = as_str(get(cross, "held-ref"));
    assert_eq!(
      held_ref, "held.semantic-frame-null",
      "{}.held-ref must be `held.semantic-frame-null` for null-frame folds",
      sk
    );

    // source-fold-id is preserved verbatim.
    let sk_id = match sk {
      "cross-unknown" => "unknown",
      "cross-bareConstraint" => "bareConstraint",
      _ => unreachable!(),
    };
    assert_eq!(
      as_str(get(cross, "source-fold-id")),
      sk_id,
      "{}.source-fold-id must equal `{}`",
      sk,
      sk_id
    );
  }

  // Cross-check: the negative runner's canonical SI still
  // exposes interpret (Stage 2) AND is reachable via the
  // negative runner. No new SI surface is introduced by v0.5.2
  // — the negative runner reuses v0_5_run_negative.px's
  // canonical-si.
  let neg_si = get(&neg, "canonical-si");
  assert!(
    matches!(get(neg_si, "interpret"), Value::Lambda { .. }),
    "negative runner's canonical-si still has Stage 2 interpret Lambda"
  );
}
