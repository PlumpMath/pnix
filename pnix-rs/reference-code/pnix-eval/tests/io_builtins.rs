use pnix_eval::{eval_expr, eval_file, Value};
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(1);

fn env_lock() -> &'static Mutex<()> {
  static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
  LOCK.get_or_init(|| Mutex::new(()))
}

struct EnvVarGuard {
  key: &'static str,
  previous: Option<OsString>,
  _lock: std::sync::MutexGuard<'static, ()>,
}

impl EnvVarGuard {
  fn set(key: &'static str, value: String) -> Self {
    let _lock = env_lock().lock().unwrap_or_else(|err| err.into_inner());
    let previous = env::var_os(key);
    env::set_var(key, value);
    Self {
      key,
      previous,
      _lock,
    }
  }
}

impl Drop for EnvVarGuard {
  fn drop(&mut self) {
    if let Some(value) = self.previous.take() {
      env::set_var(self.key, value);
    } else {
      env::remove_var(self.key);
    }
  }
}

fn temp_dir(name: &str) -> PathBuf {
  let id = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
  let dir = std::env::temp_dir().join(format!("pnix-eval-{}-{}-{}", name, std::process::id(), id));
  fs::create_dir_all(&dir).expect("create temp dir");
  dir
}

fn workspace_root() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .expect("crate dir has workspace parent")
    .parent()
    .expect("workspace root")
    .to_path_buf()
}

#[test]
fn eval_file_resolves_relative_import_from_source_dir() {
  let dir = temp_dir("relative-import");
  let child = dir.join("child.nix");
  let main = dir.join("main.nix");
  fs::write(&child, r#"{ answer = 42; }"#).expect("write child");
  fs::write(&main, r#"let child = import ./child.nix; in child.answer"#).expect("write main");

  let value = eval_file(&main).expect("eval main with relative import");
  assert!(matches!(value, Value::Int(42)));

  let _ = fs::remove_dir_all(dir);
}

#[test]
fn builtins_read_file_uses_relative_path_from_eval_file_context() {
  let dir = temp_dir("relative-read-file");
  let data = dir.join("payload.txt");
  let main = dir.join("main.nix");
  fs::write(&data, "hello from pnix-eval").expect("write payload");
  fs::write(&main, r#"builtins.readFile ./payload.txt"#).expect("write main");

  let value = eval_file(&main).expect("eval main with relative readFile");
  assert!(matches!(value, Value::String(ref s) if s == "hello from pnix-eval"));

  let _ = fs::remove_dir_all(dir);
}

#[test]
fn builtins_to_file_writes_and_returns_path() {
  let value = eval_expr(r#"builtins.toFile "note.txt" "hello world""#).expect("eval toFile");
  let path = match value {
    Value::Path(path) => path,
    other => panic!("expected path, got {:?}", other),
  };
  let written = fs::read_to_string(&path).expect("read written file");
  assert_eq!(written, "hello world");

  let _ = fs::remove_file(path);
}

#[test]
fn builtins_read_file_type_reports_regular_file() {
  let dir = temp_dir("read-file-type");
  let data = dir.join("kind.txt");
  fs::write(&data, "kind").expect("write data");

  let value = eval_expr(&format!(r#"builtins.readFileType "{}""#, data.display()))
    .expect("eval readFileType");
  assert!(matches!(value, Value::String(ref s) if s == "regular"));

  let _ = fs::remove_dir_all(dir);
}

#[test]
fn builtins_path_exists_and_read_dir_cover_basic_fs_queries() {
  let dir = temp_dir("read-dir");
  let file = dir.join("note.txt");
  let subdir = dir.join("nested");
  fs::write(&file, "hello").expect("write file");
  fs::create_dir_all(&subdir).expect("create nested dir");

  let exists =
    eval_expr(&format!(r#"builtins.pathExists "{}""#, file.display())).expect("eval pathExists");
  assert!(matches!(exists, Value::Bool(true)));

  let missing = eval_expr(&format!(
    r#"builtins.pathExists "{}""#,
    dir.join("missing.txt").display()
  ))
  .expect("eval missing pathExists");
  assert!(matches!(missing, Value::Bool(false)));

  let listing =
    eval_expr(&format!(r#"builtins.readDir "{}""#, dir.display())).expect("eval readDir");
  let attrs = match listing {
    Value::AttrSet(attrs) => attrs,
    other => panic!("expected attrset, got {:?}", other),
  };
  assert!(matches!(attrs.get("note.txt"), Some(Value::String(s)) if s == "regular"));
  assert!(matches!(attrs.get("nested"), Some(Value::String(s)) if s == "directory"));

  let _ = fs::remove_dir_all(dir);
}

#[test]
fn builtins_base_name_of_and_dir_of_follow_paths() {
  let dir = temp_dir("path-parts");
  let nested = dir.join("alpha").join("beta.txt");
  fs::create_dir_all(nested.parent().expect("nested parent")).expect("create nested path");
  fs::write(&nested, "beta").expect("write nested file");

  let base =
    eval_expr(&format!(r#"builtins.baseNameOf "{}""#, nested.display())).expect("eval baseNameOf");
  assert!(matches!(base, Value::String(ref s) if s == "beta.txt"));

  let parent = eval_expr(&format!(r#"builtins.dirOf "{}""#, nested.display())).expect("eval dirOf");
  let expected_parent = nested
    .parent()
    .expect("nested parent")
    .to_string_lossy()
    .to_string();
  assert!(matches!(parent, Value::String(ref s) if s == &expected_parent));

  let _ = fs::remove_dir_all(dir);
}

#[test]
fn builtins_path_conversion_round_trips_strings() {
  let dir = temp_dir("path-convert");
  let file = dir.join("payload.txt");
  fs::write(&file, "payload").expect("write file");

  let to_path =
    eval_expr(&format!(r#"builtins.toPath "{}""#, file.display())).expect("eval toPath");
  assert!(matches!(to_path, Value::Path(ref path) if path == &file));

  let store_path =
    eval_expr(&format!(r#"builtins.storePath "{}""#, file.display())).expect("eval storePath");
  assert!(matches!(store_path, Value::Path(ref path) if path == &file));

  let is_path = eval_expr(&format!(
    r#"builtins.isPath (builtins.toPath "{}")"#,
    file.display()
  ))
  .expect("eval isPath");
  assert!(matches!(is_path, Value::Bool(true)));

  let _ = fs::remove_dir_all(dir);
}

#[test]
fn builtins_is_path_only_forces_outer_shape() {
  let attrs =
    eval_expr(r#"builtins.isPath { a = throw "isPath attr payload"; }"#).expect("eval attr shape");
  assert!(matches!(attrs, Value::Bool(false)));

  let list =
    eval_expr(r#"builtins.isPath [ (throw "isPath list payload") ]"#).expect("eval list shape");
  assert!(matches!(list, Value::Bool(false)));

  let err = eval_expr(r#"builtins.isPath (throw "isPath top")"#).expect_err("top thunk is forced");
  assert!(
    err.to_string().contains("isPath top"),
    "unexpected error: {err}"
  );
}

#[test]
fn builtins_get_env_reads_allowed_pnix_prefix() {
  let _guard = EnvVarGuard::set("PNIX_TEST_ALLOWED", "ok".to_string());
  let value = eval_expr(r#"builtins.getEnv "PNIX_TEST_ALLOWED""#).expect("eval getEnv");
  assert!(matches!(value, Value::String(ref s) if s == "ok"));
}

#[test]
fn builtins_get_env_blocks_unlisted_secret() {
  let _guard = EnvVarGuard::set("SECRET_TOKEN", "nope".to_string());
  let value = eval_expr(r#"builtins.getEnv "SECRET_TOKEN""#).expect("eval getEnv");
  assert!(matches!(value, Value::String(ref s) if s.is_empty()));
}

#[test]
fn proof_fixture_docset_bridge_evaluates_via_pnix_eval() {
  let root = workspace_root();
  let _guard = EnvVarGuard::set("PNIX_WORKSPACE_ROOT", root.display().to_string());
  let fixture = root.join("fixtures/pnix-query-runtime/docset-web-bridge-concept-to-fact.px");

  let value = eval_file(&fixture).expect("eval docset proof fixture");
  let map = match value {
    Value::AttrSet(map) => map,
    other => panic!("expected attrset, got {:?}", other),
  };
  assert_eq!(
    map.get("proof").and_then(|v| v.as_str()),
    Some("docset-web-bridge-conceptToFact")
  );
  assert!(matches!(map.get("used-px-owner"), Some(Value::Bool(true))));
}

#[test]
fn proof_fixture_absorb_output_fragment_evaluates_via_pnix_eval() {
  let root = workspace_root();
  let _guard = EnvVarGuard::set("PNIX_WORKSPACE_ROOT", root.display().to_string());
  let fixture = root.join("fixtures/pnix-query-runtime/absorb-policy-output-fragment.px");

  let value = eval_file(&fixture).expect("eval absorb output fragment fixture");
  let map = match value {
    Value::AttrSet(map) => map,
    other => panic!("expected attrset, got {:?}", other),
  };
  assert_eq!(
    map.get("proof").and_then(|v| v.as_str()),
    Some("absorb-policy-output-fragment")
  );
  assert_eq!(
    map.get("kind").and_then(|v| v.as_str()),
    Some("absorb-outcome")
  );
}

#[test]
fn proof_fixture_absorb_eval_owner_evaluates_via_pnix_eval() {
  let root = workspace_root();
  let _guard = EnvVarGuard::set("PNIX_WORKSPACE_ROOT", root.display().to_string());
  let fixture = root.join("fixtures/pnix-query-runtime/absorb-policy-evaluate-absorption.px");

  let value = eval_file(&fixture).expect("eval absorb eval-owner fixture");
  let map = match value {
    Value::AttrSet(map) => map,
    other => panic!("expected attrset, got {:?}", other),
  };
  assert_eq!(
    map.get("proof").and_then(|v| v.as_str()),
    Some("absorb-policy-evaluateAbsorption")
  );
  assert!(matches!(map.get("allow"), Some(Value::Bool(true))));
}

#[test]
fn proof_fixture_ontology_lift_pipeline_evaluates_via_pnix_eval() {
  let root = workspace_root();
  let _guard = EnvVarGuard::set("PNIX_WORKSPACE_ROOT", root.display().to_string());
  let fixture = root.join("fixtures/pnix-query-runtime/ontology-lift-pipeline.px");

  let value = eval_file(&fixture).expect("eval ontology lift pipeline fixture");
  let map = match value {
    Value::AttrSet(map) => map,
    other => panic!("expected attrset, got {:?}", other),
  };
  assert_eq!(
    map.get("proof").and_then(|v| v.as_str()),
    Some("ontology-lift-pipeline")
  );
  let evaluation = match map.get("evaluation") {
    Some(Value::AttrSet(evaluation)) => evaluation,
    other => panic!("expected evaluation attrset, got {:?}", other),
  };
  assert!(matches!(evaluation.get("score"), Some(Value::Float(_))));
}
