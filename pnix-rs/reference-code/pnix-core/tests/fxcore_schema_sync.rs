//! FxCore 스키마 동기화 테스트: FxCore 스키마 동기화 검증
//!
//! FxCore 스키마가 올바르게 동기화되는지 검증합니다.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .join("..")
    .join("..")
    .canonicalize()
    .unwrap()
}

fn read_to_string(path: &Path) -> String {
  std::fs::read_to_string(path)
    .unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e))
}

fn extract_struct_fields(src: &str, struct_name: &str) -> Vec<String> {
  let needle = format!("pub struct {} ", struct_name);
  let start = src
    .find(&needle)
    .unwrap_or_else(|| panic!("struct declaration not found: {}", struct_name));
  let brace_start = src[start..]
    .find('{')
    .map(|idx| start + idx)
    .unwrap_or_else(|| panic!("struct body not found: {}", struct_name));

  let mut depth = 0usize;
  let mut brace_end: Option<usize> = None;
  for (offset, ch) in src[brace_start..].char_indices() {
    match ch {
      '{' => depth += 1,
      '}' => {
        depth = depth.saturating_sub(1);
        if depth == 0 {
          brace_end = Some(brace_start + offset);
          break;
        }
      }
      _ => {}
    }
  }
  let brace_end = brace_end.unwrap_or_else(|| panic!("unterminated struct body: {}", struct_name));

  let body = &src[brace_start + 1..brace_end];
  body
    .lines()
    .filter_map(|line| pub_field_name(line))
    .map(str::to_string)
    .collect()
}

fn pub_field_name(line: &str) -> Option<&str> {
  let line = line.trim();
  if !line.starts_with("pub") {
    return None;
  }

  let mut rest = &line["pub".len()..];
  if rest.starts_with('(') {
    let close = rest.find(')')?;
    rest = &rest[close + 1..];
  }
  rest = rest.trim_start();

  let colon = rest.find(':')?;
  let name = rest[..colon].trim();
  if name.is_empty() {
    return None;
  }
  if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
    return None;
  }
  Some(name)
}

fn schema_property_keys(schema: &serde_json::Value, definition: Option<&str>) -> BTreeSet<String> {
  let props = match definition {
    None => schema
      .get("properties")
      .and_then(|v| v.as_object())
      .unwrap_or_else(|| panic!("schema root properties missing")),
    Some(def) => schema
      .get("definitions")
      .and_then(|v| v.get(def))
      .and_then(|v| v.get("properties"))
      .and_then(|v| v.as_object())
      .unwrap_or_else(|| panic!("schema definition properties missing: {}", def)),
  };
  props.keys().cloned().collect()
}

#[test]
fn fxcore_schema_includes_all_core_fields() {
  let root = workspace_root();
  let schema_txt = read_to_string(&root.join("schema").join("fxcore.schema.json"));
  let schema: serde_json::Value =
    serde_json::from_str(&schema_txt).expect("schema/fxcore.schema.json must be valid JSON");

  let fxcore_src = read_to_string(
    &root
      .join("crates")
      .join("pnix-fxcore-types")
      .join("src")
      .join("lib.rs"),
  );

  let checks: [(&str, Option<&str>); 10] = [
    ("FxCoreModule", None),
    ("FxCoreMeta", Some("FxCoreMeta")),
    ("FxInput", Some("FxInput")),
    ("FxPort", Some("FxPort")),
    ("FxMorphism", Some("FxMorphism")),
    ("ExecutionContract", Some("ExecutionContract")),
    ("FxNodeMeta", Some("FxNodeMeta")),
    ("FxScope", Some("FxScope")),
    ("FxNode", Some("FxNode")),
    ("FxEdge", Some("FxEdge")),
  ];

  for (struct_name, def_name) in checks {
    let fields = extract_struct_fields(&fxcore_src, struct_name);
    assert!(
      !fields.is_empty(),
      "expected to extract at least 1 field for {}",
      struct_name
    );

    let keys = schema_property_keys(&schema, def_name);
    for field in fields {
      assert!(
        keys.contains(&field),
        "schema is missing field `{}` for {} ({:?})",
        field,
        struct_name,
        def_name
      );
    }
  }
}
