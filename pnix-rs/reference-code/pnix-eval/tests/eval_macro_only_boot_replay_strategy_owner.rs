use pnix_eval::{eval_file, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixture_path() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../../fixtures/pnix-query-runtime/macro-only-boot-replay-strategy-owner.px")
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
fn replay_strategy_fixture_imports_owner_and_runner_receipt() {
  let run =
    eval_file(&fixture_path()).expect("macro-only boot replay strategy owner fixture must eval");
  assert_eq!(
    as_str(get(&run, "proof")),
    "macro-only-boot-bounded-replay-strategy-owner"
  );
  assert!(as_bool(get(&run, "imported-owner")));
  assert!(as_bool(get(&run, "used-px-owner")));
  assert_eq!(
    as_str(get(&run, "imported-boot-runner")),
    "tesseract-macro-ontology-macro-only-boot-runner-owner"
  );
  assert_eq!(
    as_str(get(&run, "expected-strategy-id")),
    "strategy.macro-only-boot.bounded-full-graph-replay.v1"
  );
}

#[test]
fn owner_meta_declares_strategy_without_execution_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  let meta = get(&run, "owner-meta");
  assert_eq!(
    as_str(get(meta, "owner")),
    "stdlib.lib.gate.macro-only-boot-replay-strategy"
  );
  assert_eq!(as_str(get(meta, "constructor")), "validateReplayStrategy");
  assert_eq!(
    as_str(get(meta, "output-shape")),
    "bounded-replay-strategy-present or Held"
  );
  for key in [
    "replay-executed",
    "boot-executed",
    "macro-only-runtime-owner-booted",
    "new-engine-from-zero",
    "runtime-install",
    "global-ontology-runtime",
    "host-code-removal-started",
    "p-puck-owned-by-strategy",
    "compare-owned-by-strategy",
  ] {
    assert!(!as_bool(get(meta, key)), "`{key}` must stay false");
  }
}

#[test]
fn required_graph_nodes_and_bounds_are_explicit() {
  let run = eval_file(&fixture_path()).unwrap();
  let nodes = string_set(get(&run, "required-graph-nodes"));
  for expected in [
    "node.constitution-owner",
    "node.promote-r7-compat",
    "node.evaluate-select-ranking-owner",
    "node.evaluate-select-route-adapter-owner",
    "node.evaluate-select-scoped-adapter-install",
    "node.lift-query-emit-r7-compat",
    "node.internal-self-capability-map",
    "node.host-removal-map",
    "node.macro-only-boot-manifest",
    "node.macro-only-boot-attempt",
    "node.macro-only-boot-runner-owner",
  ] {
    assert!(nodes.contains(expected), "missing graph node `{expected}`");
  }
  assert_eq!(nodes.len(), 11);

  let bounds = string_set(get(&run, "required-bounds"));
  for expected in [
    "bound.max-node-count",
    "bound.max-edge-count",
    "bound.max-import-depth",
    "bound.cycle-policy-hold",
    "bound.stack-budget-shallow",
    "bound.no-runtime-execution",
  ] {
    assert!(bounds.contains(expected), "missing bound `{expected}`");
  }
  assert_eq!(bounds.len(), 6);
}

#[test]
fn valid_strategy_is_present_but_does_not_execute_replay_or_boot() {
  let run = eval_file(&fixture_path()).unwrap();
  let valid = get(&run, "valid-strategy");
  assert_eq!(
    as_str(get(valid, "status")),
    "bounded-replay-strategy-present"
  );
  assert_eq!(as_str(get(valid, "strategy-status")), "present");
  assert!(matches!(get(valid, "held-id"), Value::Null));
  assert!(as_bool(get(valid, "bounded-replay-strategy-present")));
  assert_eq!(as_list(get(valid, "missing")).len(), 0);
  for key in [
    "replay-executed",
    "boot-executed",
    "macro-only-runtime-owner-booted",
    "new-engine-from-zero",
    "fresh-p-puck-after-current-cut",
    "compare-after-boot",
    "runtime-install",
    "global-ontology-runtime",
    "host-code-removal-started",
    "implementation-command",
  ] {
    assert!(!as_bool(get(valid, key)), "`{key}` must stay false");
  }
}

#[test]
fn missing_node_and_cycle_guard_are_held_before_strategy_presence() {
  let run = eval_file(&fixture_path()).unwrap();
  let missing_node = get(&run, "missing-node");
  assert_eq!(as_str(get(missing_node, "status")), "Held");
  assert_eq!(
    as_str(get(missing_node, "held-id")),
    "held.macro-only-boot-replay-strategy.missing-required-shape"
  );
  let missing = string_set(get(missing_node, "missing"));
  assert!(missing.contains("node.evaluate-select-route-adapter-owner"));
  assert!(missing.contains("node.macro-only-boot-runner-owner"));

  let missing_cycle = get(&run, "missing-cycle-guard");
  assert_eq!(as_str(get(missing_cycle, "status")), "Held");
  assert!(string_set(get(missing_cycle, "missing")).contains("cycle-guard-present"));
}

#[test]
fn wrong_strategy_id_is_held_before_cross_strategy_claim() {
  let run = eval_file(&fixture_path()).unwrap();
  let held = get(&run, "wrong-strategy");
  assert_eq!(as_str(get(held, "status")), "Held");
  assert_eq!(
    as_str(get(held, "held-id")),
    "held.macro-only-boot-replay-strategy.strategy-id-mismatch"
  );
  assert!(string_set(get(held, "missing"))
    .contains("expected-strategy-id:strategy.macro-only-boot.bounded-full-graph-replay.v1"));
}

#[test]
fn old_host_authority_is_held_even_with_complete_shape() {
  let run = eval_file(&fixture_path()).unwrap();
  let held = get(&run, "old-host-authority");
  assert_eq!(as_str(get(held, "status")), "Held");
  assert_eq!(
    as_str(get(held, "held-id")),
    "held.macro-only-boot-replay-strategy.old-host-authority"
  );
  assert!(!as_bool(get(held, "old-host-authority")));
  assert!(!as_bool(get(held, "bounded-replay-strategy-present")));
}

#[test]
fn strategy_cannot_claim_boot_execution_or_external_audits() {
  let run = eval_file(&fixture_path()).unwrap();
  let boot_claim = get(&run, "boot-claim");
  assert_eq!(as_str(get(boot_claim, "status")), "Held");
  assert_eq!(
    as_str(get(boot_claim, "held-id")),
    "held.macro-only-boot-replay-strategy.strategy-is-not-execution"
  );
  assert!(!as_bool(get(boot_claim, "boot-executed")));

  let audit_claim = get(&run, "external-audit-claim");
  assert_eq!(as_str(get(audit_claim, "status")), "Held");
  assert_eq!(
    as_str(get(audit_claim, "held-id")),
    "held.macro-only-boot-replay-strategy.external-audit-claim"
  );
  assert!(!as_bool(get(audit_claim, "fresh-p-puck-after-current-cut")));
  assert!(!as_bool(get(audit_claim, "compare-after-boot")));
}

#[test]
fn all_outputs_preserve_no_runtime_or_host_delete_authority() {
  let run = eval_file(&fixture_path()).unwrap();
  for key in [
    "valid-strategy",
    "missing-node",
    "missing-cycle-guard",
    "wrong-strategy",
    "old-host-authority",
    "boot-claim",
    "external-audit-claim",
  ] {
    let value = get(&run, key);
    assert!(!as_bool(get(value, "replay-executed")), "`{key}` replayed");
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
  }
}

#[test]
fn top_level_state_records_strategy_owner_without_runtime_install() {
  let run = eval_file(&fixture_path()).unwrap();
  assert!(as_bool(get(&run, "bounded-replay-strategy-present")));
  for key in [
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
