use pnix_query_runtime::px_eval_json;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(1);

fn workspace_root() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .and_then(|path| path.parent())
    .expect("CARGO_MANIFEST_DIR must resolve to workspace root via two parents")
    .to_path_buf()
}

fn write_temp_px_file(source: &str) -> PathBuf {
  let id = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
  let path = std::env::temp_dir().join(format!(
    "pnix-query-runtime-px-eval-{}-{}.px",
    std::process::id(),
    id
  ));
  fs::write(&path, source).expect("write temp px file");
  path
}

#[test]
fn px_eval_json_library_evaluates_lookup_owner_surface() {
  let root = workspace_root();
  let source = format!(
    r#"
      let
        root = "{}";
        rules = import (root + "/stdlib/lib/gate/lookup-rules.px");
      in rules.mkLookupRule {{
        context = "Physics.Classical";
      }}
    "#,
    root.display()
  );

  let json = px_eval_json::eval_px_source_to_json(&source).expect("pnix-eval source evaluation");
  assert!(
    json.contains(r#""tool-name":"ontology_lookup_related""#),
    "lookup-rules owner must be reachable through pnix-eval helper: {json}"
  );
  assert!(
    json.contains(r#""packet-shape":"facts+provenance_refs-only""#),
    "lookup-rules owner must preserve packet shape: {json}"
  );
}

#[test]
fn px_eval_json_binary_matches_library_on_file() {
  let path = write_temp_px_file(
    r#"
      let
        answer = 41 + 1;
        label = "direct";
      in {
        answer = answer;
        label = label;
      }
    "#,
  );

  let expected = px_eval_json::eval_px_file_to_json(&path).expect("library file evaluation");
  let output = Command::new(env!("CARGO_BIN_EXE_pnix-query-px-eval"))
    .arg("--file")
    .arg(&path)
    .output()
    .expect("spawn pnix-query-px-eval");
  assert!(
    output.status.success(),
    "pnix-query-px-eval should exit successfully: {:?}",
    output
  );
  let stdout = String::from_utf8(output.stdout).expect("helper stdout should be utf-8");
  assert_eq!(
    stdout.trim_end(),
    expected.trim_end(),
    "binary helper must match direct library evaluation"
  );

  let _ = fs::remove_file(path);
}

#[test]
fn px_eval_json_escapes_nested_string_quotes() {
  let json = px_eval_json::eval_px_source_to_json(
    r#"{ content_preview = "{ subject = \"alias-write\"; }"; }"#,
  )
  .expect("pnix-eval source evaluation");

  assert!(
    json.contains(r#""content_preview":"{ subject = \"alias-write\"; }""#),
    "embedded quotes must stay escaped JSON string content: {json}"
  );
}
