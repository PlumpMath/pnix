use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/macro-only-boot-runner-owner.px")
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
fn boot_runner_fixture_imports_owner_and_boot_attempt() {
  let run = eval_file(&fixture_path()).expect("macro-only boot runner owner fixture must eval");
  assert_eq!(as_str(get(&run, "proof")), "macro-only-boot-runner-owner");
  assert!(as_bool(get(&run, "imported-owner")));
  assert!(as_bool(get(&run, "used-px-owner")));
  assert_eq!(
    as_str(get(&run, "imported-boot-attempt")),
    "tesseract-macro-ontology-macro-only-boot-execution-attempt"
  );
  assert_eq!(
    as_str(get(&run, "expected-runner-id")),
    "runner.macro-only-boot.v1"
  );
  assert!(as_bool(get(&run, "boot-runner-owner-present")));
}

#[test]
fn owner_meta_declares_runner_without_runtime_or_puck_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.lib.gate.macro-only-boot-runner"
  );
  assert_eq!(as_str(get(meta, "constructor")), "runBootAttempt");
  assert_eq!(as_str(get(meta, "runner-id")), "runner.macro-only-boot.v1");
  assert_eq!(
    as_str(get(meta, "output-shape")),
    "runner-ready-for-bounded-replay or Held"
  );
  for key in [
    "boot-executed",
    "macro-only-runtime-owner-booted",
    "new-engine-from-zero",
    "runtime-install",
    "global-ontology-runtime",
    "host-code-removal-started",
    "p-puck-owned-by-runner",
    "compare-owned-by-runner",
  ] {
    assert!(!as_bool(get(meta, key)), "`{key}` must stay false");
  }
}

#[test]
fn required_evidence_names_all_boot_success_prerequisites() {
  let run = eval_file(&fixture_path()).unwrap();
  let required = string_set(get(&run, "required-evidence"));
  for expected in [
    "manifest-loaded",
    "manifest-complete",
    "bounded-replay-strategy-present",
    "fresh-p-puck-after-current-cut",
    "compare-after-boot",
    "regression-corpus-transfer-present",
    "bootstrap-status-audit-update-plan-present",
    "target-specific-delete-proof-present",
  ] {
    assert!(required.contains(expected), "missing evidence `{expected}`");
  }
  assert_eq!(required.len(), 8);
}

#[test]
fn current_attempt_is_held_because_replay_puck_compare_and_delete_evidence_are_missing() {
  let run = eval_file(&fixture_path()).unwrap();
  let attempt = get(&run, "current-attempt");
  assert_eq!(as_str(get(attempt, "status")), "Held");
  assert_eq!(
    as_str(get(attempt, "held-id")),
    "held.macro-only-boot-runner.missing-required-evidence"
  );
  assert!(as_bool(get(attempt, "boot-runner-owner-present")));
  assert!(!as_bool(get(attempt, "ready-for-bounded-replay")));
  let missing = string_set(get(attempt, "missing"));
  for expected in [
    "bounded-replay-strategy-present",
    "fresh-p-puck-after-current-cut",
    "compare-after-boot",
    "regression-corpus-transfer-present",
    "bootstrap-status-audit-update-plan-present",
    "target-specific-delete-proof-present",
  ] {
    assert!(
      missing.contains(expected),
      "missing Held evidence `{expected}`"
    );
  }
}

#[test]
fn ready_candidate_is_replay_ready_but_still_not_boot_executed() {
  let run = eval_file(&fixture_path()).unwrap();
  let ready = get(&run, "ready-candidate");
  assert_eq!(
    as_str(get(ready, "status")),
    "runner-ready-for-bounded-replay"
  );
  assert_eq!(
    as_str(get(ready, "runner-status")),
    "ready-for-bounded-replay"
  );
  assert!(matches!(get(ready, "held-id"), Value::Null));
  assert!(as_bool(get(ready, "ready-for-bounded-replay")));
  assert_eq!(as_list(get(ready, "missing")).len(), 0);
  for key in [
    "boot-executed",
    "macro-only-runtime-owner-booted",
    "new-engine-from-zero",
    "runtime-install",
    "global-ontology-runtime",
    "host-code-removal-started",
    "implementation-command",
  ] {
    assert!(!as_bool(get(ready, key)), "`{key}` must stay false");
  }
}

#[test]
fn no_manifest_is_held_before_any_runner_readiness() {
  let run = eval_file(&fixture_path()).unwrap();
  let held = get(&run, "no-manifest");
  assert_eq!(as_str(get(held, "status")), "Held");
  assert_eq!(
    as_str(get(held, "held-id")),
    "held.macro-only-boot-runner.manifest-marker-missing"
  );
  assert!(!as_bool(get(held, "ready-for-bounded-replay")));
}

#[test]
fn wrong_runner_id_is_held_before_cross_runner_claim() {
  let run = eval_file(&fixture_path()).unwrap();
  let held = get(&run, "wrong-runner");
  assert_eq!(as_str(get(held, "status")), "Held");
  assert_eq!(
    as_str(get(held, "held-id")),
    "held.macro-only-boot-runner.runner-id-mismatch"
  );
  let missing = string_set(get(held, "missing"));
  assert!(missing.contains("expected-runner-id:runner.macro-only-boot.v1"));
}

#[test]
fn old_host_authority_is_held_even_with_other_evidence_present() {
  let run = eval_file(&fixture_path()).unwrap();
  let held = get(&run, "old-host-authority");
  assert_eq!(as_str(get(held, "status")), "Held");
  assert_eq!(
    as_str(get(held, "held-id")),
    "held.macro-only-boot-runner.old-host-authority"
  );
  assert!(!as_bool(get(held, "old-host-authority")));
  assert!(!as_bool(get(held, "ready-for-bounded-replay")));
}

#[test]
fn all_outputs_preserve_no_runtime_or_host_delete_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  for key in [
    "current-attempt",
    "ready-candidate",
    "no-manifest",
    "wrong-runner",
    "old-host-authority",
  ] {
    let value = get(&run, key);
    assert!(!as_bool(get(value, "boot-executed")), "`{key}` booted");
    assert!(
      !as_bool(get(value, "macro-only-runtime-owner-booted")),
      "`{key}` claimed runtime owner"
    );
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
      !as_bool(get(value, "p-puck-owned-by-runner")),
      "`{key}` owned p-puck"
    );
    assert!(
      !as_bool(get(value, "compare-owned-by-runner")),
      "`{key}` owned compare"
    );
  }
}

#[test]
fn top_level_state_records_owner_without_runtime_install() {
  let run = eval_file(&fixture_path()).unwrap();
  assert!(as_bool(get(&run, "boot-runner-owner-present")));
  for key in [
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
