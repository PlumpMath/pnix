use pnix_eval::{eval_file, Value};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

fn workspace_root() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("../..")
    .canonicalize()
    .unwrap()
}

fn env_lock() -> &'static Mutex<()> {
  static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
  LOCK.get_or_init(|| Mutex::new(()))
}

#[test]
fn live_lift_rules_fixture_evaluates_directly() {
  let _guard = env_lock().lock().unwrap();
  let fixture = workspace_root().join("fixtures/pnix-query-runtime/lift-rules-contextual-facts.px");
  std::env::set_var("PNIX_WORKSPACE_ROOT", workspace_root());
  let value = eval_file(&fixture).unwrap();
  std::env::remove_var("PNIX_WORKSPACE_ROOT");

  match value {
    Value::AttrSet(map) => {
      assert!(matches!(
        map.get("proof"),
        Some(Value::String(s)) if s == "pnix-gate-lift-rules-contextual-facts"
      ));
      assert!(matches!(map.get("used-px-owner"), Some(Value::Bool(true))));
      assert!(matches!(map.get("imported-owner"), Some(Value::Bool(true))));
      assert!(matches!(map.get("rule-count"), Some(Value::Int(7))));
      assert!(matches!(map.get("fact-count"), Some(Value::Int(7))));
      assert!(matches!(map.get("all-candidate"), Some(Value::Bool(true))));
      assert!(matches!(
        map.get("all-contextual-fact"),
        Some(Value::Bool(true))
      ));
    }
    other => panic!("expected fixture attrset, got {:?}", other),
  }
}
