use serde_json::Value;
use std::collections::BTreeMap;

/// Canonicalize JSON value for deterministic hashing/comparison.
///
/// - Object keys are sorted
/// - Arrays preserve order while recursively canonicalizing items
pub fn canonicalize_value(value: &Value) -> Value {
  match value {
    Value::Object(map) => {
      let mut sorted: BTreeMap<String, Value> = BTreeMap::new();
      for (key, inner) in map {
        sorted.insert(key.clone(), canonicalize_value(inner));
      }
      let mut out = serde_json::Map::new();
      for (key, inner) in sorted {
        out.insert(key, inner);
      }
      Value::Object(out)
    }
    Value::Array(items) => Value::Array(items.iter().map(canonicalize_value).collect()),
    _ => value.clone(),
  }
}
