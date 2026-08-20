use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/macro-only-self-receipt-frontier-emission-owner.px")
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

fn as_i64(v: &Value) -> i64 {
  match v {
    Value::Int(i) => *i,
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

fn string_set(v: &Value) -> BTreeSet<&str> {
  as_list(v).iter().map(as_str).collect()
}

fn attrs_by_id<'a>(items: &'a Value) -> BTreeMap<&'a str, &'a Value> {
  as_list(items)
    .iter()
    .map(|item| (as_str(get(item, "id")), item))
    .collect()
}

#[test]
fn fixture_imports_owner_and_frontier_source() {
  let run = eval_file(&fixture_path()).expect("self receipt frontier emission owner fixture");
  assert_eq!(
    as_str(get(&run, "proof")),
    "macro-only-self-receipt-frontier-emission-owner"
  );
  assert!(as_bool(get(&run, "imported-owner")));
  assert!(as_bool(get(&run, "used-px-owner")));
  assert_eq!(
    as_str(get(&run, "imported-frontier-source")),
    "tesseract-macro-ontology-macro-only-host-removal-fresh-delete-p-puck-current-cut"
  );
  assert_eq!(
    as_str(get(&run, "frontier-source-status")),
    "host-removal-fresh-delete-puck-current-cut-present-not-delete-ready"
  );
}

#[test]
fn owner_meta_declares_detector_without_writer_or_runtime() {
  let run = eval_file(&fixture_path()).unwrap();
  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.lib.gate.macro-only-self-receipt-frontier-emission"
  );
  assert_eq!(
    as_str(get(meta, "constructor")),
    "validateSelfReceiptFrontierEmission"
  );
  assert!(as_bool(get(meta, "self-receipt-frontier-emission")));
  assert!(as_bool(get(meta, "receipt-needed-detector")));
  assert_eq!(as_i64(get(meta, "emitted-candidate-count")), 5);
  for key in [
    "receipt-auto-written",
    "receipt-auto-approved",
    "receipt-file-created",
    "delete-ready",
    "remove-now",
    "host-code-removal-started",
    "implementation-command",
    "runtime-install",
    "global-ontology-runtime",
    "runtime-api-flattening",
    "meaning-db",
    "new-engine-from-zero",
    "p-puck-is-semantic-owner",
    "old-host-authority",
    "gpl-family-dependencies",
  ] {
    assert!(!as_bool(get(meta, key)), "`{key}` must stay false");
  }
}

#[test]
fn required_frontiers_and_candidate_names_are_pinned() {
  let run = eval_file(&fixture_path()).unwrap();
  let frontiers = string_set(get(&run, "required-open-frontiers"));
  for expected in [
    "delete-ready-targets-after-fresh-delete-puck",
    "actual-host-removal-implementation-command",
    "global-runtime-install-proof-after-semantic-owner",
    "domain-runtime-api-flattening-after-semantic-owner",
    "lift-query-emit-runtime-owner-or-host-removal-proof",
  ] {
    assert!(
      frontiers.contains(expected),
      "missing frontier `{expected}`"
    );
  }
  assert_eq!(frontiers.len(), 5);

  let candidates = attrs_by_id(get(&run, "receipt-candidates"));
  assert_eq!(candidates.len(), 5);
  for expected in [
    "candidate.receipt.macro-only-host-removal-delete-ready-target-proof",
    "candidate.receipt.macro-only-host-removal-implementation-command-proof",
    "candidate.receipt.global-runtime-install-proof-after-semantic-owner",
    "candidate.receipt.domain-runtime-api-flattening-map",
    "candidate.receipt.lift-query-emit-runtime-owner-or-host-removal-proof",
  ] {
    assert!(
      candidates.contains_key(expected),
      "missing candidate `{expected}`"
    );
    assert_eq!(
      as_str(get(candidates[expected], "authority")),
      "candidate-name-only"
    );
  }
}

#[test]
fn valid_proof_emits_candidates_only() {
  let run = eval_file(&fixture_path()).unwrap();
  let valid = get(&run, "valid-proof");
  assert_eq!(
    as_str(get(valid, "status")),
    "self-receipt-frontier-emission-present"
  );
  assert!(matches!(get(valid, "held-id"), Value::Null));
  assert!(as_bool(get(valid, "self-receipt-frontier-emission")));
  assert!(as_bool(get(valid, "receipt-needed-detector")));
  assert_eq!(as_i64(get(valid, "emitted-candidate-count")), 5);
  assert_eq!(as_i64(get(valid, "covered-frontier-count")), 5);
  assert_eq!(as_list(get(valid, "emitted-receipt-candidates")).len(), 5);
  assert!(string_set(get(valid, "closes")).contains("need.self.receipt-needed-detector"));
  assert!(string_set(get(valid, "next-open-frontiers"))
    .contains("receipt-skeleton-generator-after-frontier-emission"));
  for key in [
    "receipt-auto-written",
    "receipt-auto-approved",
    "receipt-file-created",
    "delete-ready",
    "host-code-removal-started",
    "implementation-command",
    "runtime-install",
    "global-ontology-runtime",
    "runtime-api-flattening",
    "meaning-db",
  ] {
    assert!(!as_bool(get(valid, key)), "`{key}` must stay false");
  }
}

#[test]
fn stale_source_frontier_and_candidate_mismatches_are_held() {
  let run = eval_file(&fixture_path()).unwrap();
  for (key, held_id) in [
    (
      "wrong-proof",
      "held.macro-only-self-receipt-frontier-emission.proof-id-mismatch",
    ),
    (
      "stale-stage",
      "held.macro-only-self-receipt-frontier-emission.stale-current-stage",
    ),
    (
      "missing-frontier-source",
      "held.macro-only-self-receipt-frontier-emission.missing-frontier-source",
    ),
    (
      "missing-open-frontier",
      "held.macro-only-self-receipt-frontier-emission.frontier-or-candidate-mismatch",
    ),
    (
      "candidate-count-mismatch",
      "held.macro-only-self-receipt-frontier-emission.candidate-count-mismatch",
    ),
    (
      "unknown-candidate-frontier",
      "held.macro-only-self-receipt-frontier-emission.frontier-or-candidate-mismatch",
    ),
    (
      "authority-overclaim",
      "held.macro-only-self-receipt-frontier-emission.authority-overclaim",
    ),
  ] {
    let case = get(&run, key);
    assert_eq!(as_str(get(case, "status")), "Held", "{key}");
    assert_eq!(as_str(get(case, "held-id")), held_id, "{key}");
  }
}

#[test]
fn writer_approval_delete_runtime_owner_and_license_claims_are_held() {
  let run = eval_file(&fixture_path()).unwrap();
  for (key, held_id) in [
    (
      "auto-writer-claim",
      "held.macro-only-self-receipt-frontier-emission.auto-writer-overclaim",
    ),
    (
      "auto-approval-claim",
      "held.macro-only-self-receipt-frontier-emission.auto-approval-overclaim",
    ),
    (
      "delete-claim",
      "held.macro-only-self-receipt-frontier-emission.delete-or-command-overclaim",
    ),
    (
      "runtime-claim",
      "held.macro-only-self-receipt-frontier-emission.runtime-overclaim",
    ),
    (
      "semantic-owner-claim",
      "held.macro-only-self-receipt-frontier-emission.p-puck-semantic-owner",
    ),
    (
      "old-host-authority",
      "held.macro-only-self-receipt-frontier-emission.old-host-authority",
    ),
    (
      "gpl-claim",
      "held.macro-only-self-receipt-frontier-emission.gpl-family-dependency",
    ),
  ] {
    let case = get(&run, key);
    assert_eq!(as_str(get(case, "status")), "Held", "{key}");
    assert_eq!(as_str(get(case, "held-id")), held_id, "{key}");
  }
}

#[test]
fn required_evidence_records_no_auto_shortcuts() {
  let run = eval_file(&fixture_path()).unwrap();
  let evidence = string_set(get(&run, "required-evidence"));
  for expected in [
    "current-stage-after-fresh-delete-puck",
    "fresh-puck-before-delete-true",
    "frontier-source-present",
    "all-open-frontiers-covered",
    "candidate-count-matches-frontiers",
    "no-auto-writer",
    "no-auto-approval",
    "no-host-removal",
    "no-runtime-install",
    "no-gpl-family-dependencies",
  ] {
    assert!(evidence.contains(expected), "missing evidence `{expected}`");
  }
}

#[test]
fn top_level_state_records_detector_only() {
  let run = eval_file(&fixture_path()).unwrap();
  assert!(as_bool(get(&run, "self-receipt-frontier-emission")));
  assert!(as_bool(get(&run, "receipt-needed-detector")));
  assert_eq!(as_i64(get(&run, "emitted-candidate-count")), 5);
  assert_eq!(as_list(get(&run, "emitted-receipt-candidates")).len(), 5);
  for key in [
    "receipt-auto-written",
    "receipt-auto-approved",
    "receipt-file-created",
    "delete-ready",
    "remove-now",
    "host-code-removal-started",
    "implementation-command",
    "runtime-install",
    "global-ontology-runtime",
    "runtime-api-flattening",
    "meaning-db",
    "p-puck-is-semantic-owner",
    "gpl-family-dependencies",
  ] {
    assert!(!as_bool(get(&run, key)), "`{key}` must stay false");
  }
}
