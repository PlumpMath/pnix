use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/macro-only-boot-regression-corpus-owner.px")
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

fn string_set(v: &Value) -> BTreeSet<&str> {
  as_list(v).iter().map(as_str).collect()
}

#[test]
fn corpus_fixture_imports_owner_and_replay_strategy() {
  let run =
    eval_file(&fixture_path()).expect("macro-only boot regression corpus fixture must eval");
  assert_eq!(
    as_str(get(&run, "proof")),
    "macro-only-boot-regression-corpus-owner"
  );
  assert!(as_bool(get(&run, "imported-owner")));
  assert!(as_bool(get(&run, "used-px-owner")));
  assert_eq!(
    as_str(get(&run, "imported-replay-strategy")),
    "tesseract-macro-ontology-macro-only-bounded-replay-strategy"
  );
  assert_eq!(
    as_str(get(&run, "expected-corpus-id")),
    "corpus.macro-only-boot.regression-retention.v1"
  );
}

#[test]
fn owner_meta_declares_corpus_without_runtime_or_audit_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.lib.gate.macro-only-boot-regression-corpus"
  );
  assert_eq!(as_str(get(meta, "constructor")), "validateRegressionCorpus");
  assert_eq!(
    as_str(get(meta, "output-shape")),
    "regression-corpus-transfer-present or Held"
  );
  for key in [
    "compare-after-boot",
    "fresh-p-puck-after-current-cut",
    "replay-executed",
    "boot-executed",
    "macro-only-runtime-owner-booted",
    "new-engine-from-zero",
    "runtime-install",
    "global-ontology-runtime",
    "host-code-removal-started",
    "p-puck-owned-by-corpus",
    "compare-owned-by-corpus",
  ] {
    assert!(!as_bool(get(meta, key)), "`{key}` must stay false");
  }
}

#[test]
fn required_corpus_ids_cover_legacy_specimens_negative_held_and_rollback() {
  let run = eval_file(&fixture_path()).unwrap();
  let corpus = string_set(get(&run, "required-corpus-ids"));
  for expected in [
    "corpus.promote-r7-compat-regression",
    "corpus.evaluate-select-ranking-regression",
    "corpus.evaluate-select-negative-held",
    "corpus.lift-query-emit-query-projection-regression",
    "corpus.lift-query-emit-negative-held",
    "corpus.host-stdlib-ontology-specimen",
    "corpus.host-ssa-builtins-shim",
    "corpus.host-ir-dispatch",
    "corpus.pnix-core-ontology-oracle",
    "corpus.ontology-builtin-tests",
    "corpus.rollback-and-supersede-audit",
  ] {
    assert!(corpus.contains(expected), "missing corpus `{expected}`");
  }
  assert_eq!(corpus.len(), 11);
}

#[test]
fn valid_corpus_is_present_but_does_not_run_compare_puck_replay_or_boot() {
  let run = eval_file(&fixture_path()).unwrap();
  let valid = get(&run, "valid-corpus");
  assert_eq!(
    as_str(get(valid, "status")),
    "regression-corpus-transfer-present"
  );
  assert_eq!(as_str(get(valid, "corpus-status")), "present");
  assert!(matches!(get(valid, "held-id"), Value::Null));
  assert!(as_bool(get(valid, "regression-corpus-transfer-present")));
  assert_eq!(as_list(get(valid, "missing")).len(), 0);
  for key in [
    "compare-after-boot",
    "fresh-p-puck-after-current-cut",
    "replay-executed",
    "boot-executed",
    "macro-only-runtime-owner-booted",
    "new-engine-from-zero",
    "runtime-install",
    "global-ontology-runtime",
    "host-code-removal-started",
    "implementation-command",
  ] {
    assert!(!as_bool(get(valid, key)), "`{key}` must stay false");
  }
}

#[test]
fn missing_corpus_and_negative_held_are_held_before_presence() {
  let run = eval_file(&fixture_path()).unwrap();
  let missing_corpus = get(&run, "missing-corpus");
  assert_eq!(as_str(get(missing_corpus, "status")), "Held");
  assert_eq!(
    as_str(get(missing_corpus, "held-id")),
    "held.macro-only-boot-regression-corpus.missing-required-corpus"
  );
  let missing = string_set(get(missing_corpus, "missing"));
  assert!(missing.contains("corpus.evaluate-select-negative-held"));
  assert!(missing.contains("corpus.rollback-and-supersede-audit"));

  let missing_held = get(&run, "missing-negative-held");
  assert_eq!(as_str(get(missing_held, "status")), "Held");
  assert!(string_set(get(missing_held, "missing")).contains("negative-held-retained"));
}

#[test]
fn wrong_corpus_and_old_host_authority_are_held() {
  let run = eval_file(&fixture_path()).unwrap();
  let wrong = get(&run, "wrong-corpus");
  assert_eq!(as_str(get(wrong, "status")), "Held");
  assert_eq!(
    as_str(get(wrong, "held-id")),
    "held.macro-only-boot-regression-corpus.corpus-id-mismatch"
  );
  assert!(string_set(get(wrong, "missing"))
    .contains("expected-corpus-id:corpus.macro-only-boot.regression-retention.v1"));

  let old_host = get(&run, "old-host-authority");
  assert_eq!(as_str(get(old_host, "status")), "Held");
  assert_eq!(
    as_str(get(old_host, "held-id")),
    "held.macro-only-boot-regression-corpus.old-host-authority"
  );
  assert!(!as_bool(get(old_host, "old-host-authority")));
}

#[test]
fn corpus_cannot_claim_delete_external_audit_boot_or_gpl_dependency() {
  let run = eval_file(&fixture_path()).unwrap();
  let delete = get(&run, "delete-claim");
  assert_eq!(as_str(get(delete, "status")), "Held");
  assert_eq!(
    as_str(get(delete, "held-id")),
    "held.macro-only-boot-regression-corpus.delete-before-boot"
  );

  let audit = get(&run, "external-audit-claim");
  assert_eq!(as_str(get(audit, "status")), "Held");
  assert_eq!(
    as_str(get(audit, "held-id")),
    "held.macro-only-boot-regression-corpus.external-audit-claim"
  );

  let boot = get(&run, "boot-claim");
  assert_eq!(as_str(get(boot, "status")), "Held");
  assert_eq!(
    as_str(get(boot, "held-id")),
    "held.macro-only-boot-regression-corpus.corpus-is-not-boot"
  );

  let gpl = get(&run, "gpl-claim");
  assert_eq!(as_str(get(gpl, "status")), "Held");
  assert_eq!(
    as_str(get(gpl, "held-id")),
    "held.macro-only-boot-regression-corpus.gpl-family-dependency"
  );
}

#[test]
fn all_outputs_preserve_no_runtime_or_host_delete_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  for key in [
    "valid-corpus",
    "missing-corpus",
    "missing-negative-held",
    "wrong-corpus",
    "old-host-authority",
    "delete-claim",
    "external-audit-claim",
    "boot-claim",
    "gpl-claim",
  ] {
    let value = get(&run, key);
    assert!(!as_bool(get(value, "boot-executed")), "`{key}` booted");
    assert!(
      !as_bool(get(value, "new-engine-from-zero")),
      "`{key}` claimed zero boot"
    );
    assert!(
      !as_bool(get(value, "runtime-install")),
      "`{key}` installed runtime"
    );
    assert!(
      !as_bool(get(value, "global-ontology-runtime")),
      "`{key}` claimed global runtime"
    );
    assert!(
      !as_bool(get(value, "host-code-removal-started")),
      "`{key}` removed host code"
    );
    assert!(
      !as_bool(get(value, "compare-after-boot")),
      "`{key}` claimed compare"
    );
    assert!(
      !as_bool(get(value, "fresh-p-puck-after-current-cut")),
      "`{key}` claimed p-puck"
    );
  }
}

#[test]
fn top_level_state_records_corpus_owner_without_runtime_install() {
  let run = eval_file(&fixture_path()).unwrap();
  assert!(as_bool(get(&run, "regression-corpus-transfer-present")));
  for key in [
    "compare-after-boot",
    "fresh-p-puck-after-current-cut",
    "replay-executed",
    "boot-executed",
    "macro-only-runtime-owner-booted",
    "new-engine-from-zero",
    "runtime-install",
    "global-ontology-runtime",
    "host-code-removal-started",
    "implementation-command",
  ] {
    assert!(!as_bool(get(&run, key)), "`{key}` must stay false");
  }
}
