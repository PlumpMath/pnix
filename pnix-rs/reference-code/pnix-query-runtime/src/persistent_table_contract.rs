//! Generic persistent-table contract reader.
//!
//! OWNER-LAW (2026-05-13): substrate-execution side of the `.px`-
//! owned persistence contract. The `.px` owner declares each redb
//! table's key field order, separator, value family, merge policy,
//! required value fields, capability, and restore order via the
//! `persistentTables` list. This module evaluates the `.px` once and
//! exposes the contract as typed Rust data; downstream Rust adapters
//! (doghouse-core / freecat / etc.) consume the typed
//! `PersistentTableContract` value — they do NOT re-evaluate `.px`,
//! and they do NOT reimplement fingerprint / parse / validate
//! algorithms.
//!
//! Owners that currently declare this contract:
//!
//!   stdlib/lib/gate/algorithm-synthesis/discourse-state.px
//!     ├── discourse-turn-records   (append-only)
//!     └── discourse-thread-index   (latest-only-pointer)
//!
//!   stdlib/lib/gate/algorithm-synthesis/ankh-retrieval-cache.px
//!     └── ankh-retrieval-entries   (latest-only-pointer)
//!
//! # Why this lives in pnix-query-runtime, not doghouse-core
//!
//! Per CLAUDE.md / OWNER-LAW CONSTITUTION: doghouse is a headless
//! storage + transport surface. It does NOT execute the pnix
//! language — `pnix_eval::eval_pnix_file` belongs in the substrate-
//! execution layer (here in `pnix-query-runtime`), not inside the
//! storage adapter. doghouse consumes the typed
//! `PersistentTableContract` value produced here and uses it for
//! K/V byte transport. This crate already owns `pnix_eval` access
//! (via `px_eval_json` and
//! `px::parse_px_file_with_pnix_eval_fallback`).
//!
//! 2026-05-17 C6 convergence note: this is an allowed read-model /
//! config-contract loader, not canonical mirror primitive proof. If a
//! table contract starts making user-visible substrate judgements, that
//! judgement belongs behind a registered mirror lens driven by pnixc-meta.
//!
//! # What this module replaces
//!
//! Previous `ankh_persistent_store::key_fingerprint` hardcoded
//! `format!("{}|{}|{}", ...)` over three field names already
//! declared in the `.px` `ankhKeyFields` list. That redeclaration
//! was the Rust-debt the 2026-05-13 slice repays — there is now a
//! single substrate-side reader, and the transport adapter takes a
//! `&PersistentTableContract` value.

use anyhow::{anyhow, Context, Result};
use std::collections::BTreeMap;
use std::path::Path;

use crate::px::PxValue;

/// One redb table contract as declared by a `.px` owner. Reading
/// fields from a `PersistentTableContract` is the only sanctioned
/// way for a Rust adapter to know a table's shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistentTableContract {
  /// Stable identifier — same string the `.px` owner uses as
  /// `table_id`. The Rust adapter uses this both to pick which
  /// contract from a list and to name the redb table in
  /// `doghouse-core::store`.
  pub table_id: String,
  /// Free-text role from the contract. Carrier-side only — not
  /// load-bearing for behavior.
  pub role: String,
  /// Key field names in declared order. The fingerprint is built by
  /// joining these fields' values (looked up from a record) with
  /// `key_separator`.
  pub key_fields: Vec<String>,
  /// Separator string. v0 owners use `"|"`. The adapter never
  /// hardcodes this value.
  pub key_separator: String,
  /// Value serialization family — `json-v1` is currently the only
  /// supported family. Future owners may add binary families
  /// without changing this module.
  pub value_family: String,
  /// Symbolic name of the value root type (`AnkhEntry`,
  /// `DiscourseTurnRecord`, etc.). Documentation-only on the
  /// adapter side; the actual Rust struct shape is the carrier's
  /// concern.
  pub value_root: String,
  /// Required value field names. `validate_value_record` returns
  /// the missing-field list (empty = ready to persist).
  pub required_value_fields: Vec<String>,
  /// Merge policy — one of `append-only` / `latest-only-pointer`.
  /// The adapter uses this to decide whether a write is allowed to
  /// replace an existing row (latest-only) or whether the write
  /// must reject if the row already exists (append-only — though
  /// append-only at the redb layer is effectively `put` since each
  /// row has a unique key already).
  pub merge_policy: String,
  /// Capability symbolic name the adapter requires in the calling
  /// context before write. Adapter-side enforcement is separate;
  /// this contract is the declarative source.
  pub capability_required: String,
  /// Restore order — `by-stored-at-ms` / `by-key`. Used by adapters
  /// that need to scan + replay the table for cache rehydration.
  pub restore_order: String,
}

impl PersistentTableContract {
  /// Build a fingerprint string from a record. Looks up each
  /// `key_field` in `fields`; empty / missing fields contribute an
  /// empty string (same as the `.px` `buildFingerprint` lambda).
  /// The exact bytes match the `.px` lambda's output for the same
  /// record — verified by the test below and by the existing
  /// `ankh_persistent_store` round-trip tests.
  pub fn fingerprint(&self, fields: &BTreeMap<String, String>) -> String {
    let parts: Vec<&str> = self
      .key_fields
      .iter()
      .map(|f| fields.get(f).map(|s| s.as_str()).unwrap_or(""))
      .collect();
    parts.join(&self.key_separator)
  }

  /// Parse a fingerprint back into a `{field -> value}` map. Returns
  /// `None` when the part count doesn't match `key_fields.len()` —
  /// same null semantics as `.px` `parseFingerprint`.
  pub fn parse_fingerprint(&self, fp: &str) -> Option<BTreeMap<String, String>> {
    let expected = self.key_fields.len();
    let parts: Vec<&str> = fp.splitn(expected, self.key_separator.as_str()).collect();
    if parts.len() != expected {
      return None;
    }
    Some(
      self
        .key_fields
        .iter()
        .cloned()
        .zip(parts.into_iter().map(|s| s.to_string()))
        .collect(),
    )
  }

  /// Return the names of required value fields that are missing or
  /// empty in `record`. Empty list = record is ready to persist.
  /// Same semantics as `.px` `validatePersistentValue`.
  pub fn missing_value_fields(&self, record: &BTreeMap<String, String>) -> Vec<String> {
    self
      .required_value_fields
      .iter()
      .filter(|f| record.get(f.as_str()).map(|v| v.is_empty()).unwrap_or(true))
      .cloned()
      .collect()
  }
}

/// Load a single named table contract from a `.px` owner file.
///
/// The owner is expected to export `persistentTables` as a list of
/// attrsets; this function picks the one whose `table_id` matches
/// `wanted_table_id`. Missing key fields or wrong shape produce an
/// error rather than a silent default — the contract is required.
pub fn load_contract(px_path: &Path, wanted_table_id: &str) -> Result<PersistentTableContract> {
  // `.px` owners use `let ... in { ... }` expressions — must go
  // through the pnix evaluator (not the literal `parse_px`, which
  // only handles bare attrset/list literals). Same path the
  // `pnix-query-px-eval` binary takes.
  //
  // The strict `pnix_eval_value_to_px_value` rejects lambda/thunk
  // variants — discourse-state.px and ankh-retrieval-cache.px both
  // export lambdas (buildFingerprint, classifyTurn, etc.) alongside
  // pure data, so we use a lenient lowering that *skips* those keys
  // and keeps only the contract data this module reads.
  let evaluated = pnix_eval::eval_pnix_file(px_path)
    .map_err(|e| anyhow!("pnix-eval {}: {e}", px_path.display()))?;
  let value = lower_lenient(&evaluated).ok_or_else(|| {
    anyhow!(
      "pnix-eval result of {} is not lowerable to PxValue (top must be attrset/list/string)",
      px_path.display()
    )
  })?;
  load_contract_from_px_value(&value, wanted_table_id).with_context(|| {
    format!(
      "extract table_id={wanted_table_id} from {}",
      px_path.display()
    )
  })
}

/// Lower a `pnix_eval::Value` into `PxValue`, *skipping* keys whose
/// values can't be represented (lambdas, thunks, paths-as-paths, etc.).
/// Returns `None` if the top-level value itself is unlowerable.
fn lower_lenient(value: &pnix_eval::Value) -> Option<PxValue> {
  use pnix_eval::Value;
  match value {
    Value::String(s) => Some(PxValue::String(s.clone())),
    Value::StringContext { text, .. } => Some(PxValue::String(text.clone())),
    Value::Int(i) => Some(PxValue::String(i.to_string())),
    Value::Float(f) => Some(PxValue::String(f.to_string())),
    Value::Bool(b) => Some(PxValue::String(b.to_string())),
    Value::Null => Some(PxValue::String(String::new())),
    Value::Path(p) => Some(PxValue::String(p.to_string_lossy().into_owned())),
    Value::List(items) => Some(PxValue::List(
      items.iter().filter_map(lower_lenient).collect(),
    )),
    Value::AttrSet(map) => {
      let mut out = std::collections::BTreeMap::new();
      for (k, v) in map.iter() {
        if let Some(lowered) = lower_lenient(v) {
          out.insert(k.clone(), lowered);
        }
      }
      Some(PxValue::AttrSet(out))
    }
    _ => None,
  }
}

/// Same as `load_contract` but takes a pre-parsed `PxValue`.
pub fn load_contract_from_px_value(
  value: &PxValue,
  wanted_table_id: &str,
) -> Result<PersistentTableContract> {
  let root = value
    .as_attrset()
    .ok_or_else(|| anyhow!("owner root is not an attrset"))?;
  let tables_value = root
    .get("persistentTables")
    .ok_or_else(|| anyhow!("owner does not export `persistentTables`"))?;
  let tables = match tables_value {
    PxValue::List(items) => items,
    _ => return Err(anyhow!("`persistentTables` is not a list")),
  };

  for item in tables {
    let attrs = match item.as_attrset() {
      Some(a) => a,
      None => continue,
    };
    let id = attrs
      .get("table_id")
      .and_then(PxValue::as_str)
      .unwrap_or("");
    if id != wanted_table_id {
      continue;
    }
    return contract_from_attrset(item);
  }

  Err(anyhow!(
    "no persistentTables row with table_id=`{wanted_table_id}` in owner"
  ))
}

/// Build a `PersistentTableContract` from a single attrset value.
/// Internal helper — `load_contract` selects the row first.
fn contract_from_attrset(value: &PxValue) -> Result<PersistentTableContract> {
  let attrs = value
    .as_attrset()
    .ok_or_else(|| anyhow!("persistentTables row is not an attrset"))?;

  let req_str = |k: &str| -> Result<String> {
    attrs
      .get(k)
      .and_then(PxValue::as_str)
      .map(|s| s.to_string())
      .ok_or_else(|| anyhow!("persistentTables row missing required field `{k}`"))
  };

  let key_fields = value.get_str_list("key_fields");
  if key_fields.is_empty() {
    return Err(anyhow!("persistentTables row has empty `key_fields`"));
  }
  let required_value_fields = value.get_str_list("required_value_fields");
  if required_value_fields.is_empty() {
    return Err(anyhow!(
      "persistentTables row has empty `required_value_fields`"
    ));
  }

  Ok(PersistentTableContract {
    table_id: req_str("table_id")?,
    role: req_str("role").unwrap_or_default(),
    key_fields,
    key_separator: req_str("key_separator")?,
    value_family: req_str("value_family")?,
    value_root: req_str("value_root").unwrap_or_default(),
    required_value_fields,
    merge_policy: req_str("merge_policy")?,
    capability_required: req_str("capability_required")?,
    restore_order: req_str("restore_order")?,
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::env;

  fn repo_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
      .join("..")
      .join("..")
      .canonicalize()
      .expect("canonicalize repo root")
  }

  #[test]
  fn loads_ankh_retrieval_entries_contract() {
    let path = repo_root().join("stdlib/lib/gate/algorithm-synthesis/ankh-retrieval-cache.px");
    let c =
      load_contract(&path, "ankh-retrieval-entries").expect("load ankh-retrieval-entries contract");
    assert_eq!(c.table_id, "ankh-retrieval-entries");
    assert_eq!(c.key_fields, vec!["query_kind", "target_path", "language"]);
    assert_eq!(c.key_separator, "|");
    assert_eq!(c.value_family, "json-v1");
    assert_eq!(c.merge_policy, "latest-only-pointer");
    assert!(c
      .required_value_fields
      .contains(&"provenance_source".to_string()));
  }

  #[test]
  fn loads_discourse_turn_records_contract() {
    let path = repo_root().join("stdlib/lib/gate/algorithm-synthesis/discourse-state.px");
    let c = load_contract(&path, "discourse-turn-records").expect("load discourse-turn-records");
    assert_eq!(c.table_id, "discourse-turn-records");
    assert_eq!(
      c.key_fields,
      vec!["tenant_id", "actor_id", "thread_id", "turn_id"]
    );
    assert_eq!(c.merge_policy, "append-only");
    assert_eq!(c.capability_required, "AppendDiscourseTurn");
  }

  #[test]
  fn loads_discourse_thread_index_contract() {
    let path = repo_root().join("stdlib/lib/gate/algorithm-synthesis/discourse-state.px");
    let c = load_contract(&path, "discourse-thread-index").expect("load discourse-thread-index");
    assert_eq!(c.key_fields, vec!["tenant_id", "actor_id", "thread_id"]);
    assert_eq!(c.merge_policy, "latest-only-pointer");
  }

  #[test]
  fn fingerprint_matches_legacy_ankh_format() {
    // Was hardcoded as `format!("{}|{}|{}", query_kind, target_path,
    // language)` in `ankh_persistent_store::key_fingerprint`. The
    // generic contract-driven version must produce the byte-
    // identical output for the same inputs.
    let path = repo_root().join("stdlib/lib/gate/algorithm-synthesis/ankh-retrieval-cache.px");
    let c = load_contract(&path, "ankh-retrieval-entries").unwrap();
    let mut fields = BTreeMap::new();
    fields.insert(
      "query_kind".to_string(),
      "lookup-module-providing-symbol".to_string(),
    );
    fields.insert("target_path".to_string(), "src/foo.py".to_string());
    fields.insert("language".to_string(), "python".to_string());
    let fp = c.fingerprint(&fields);
    assert_eq!(fp, "lookup-module-providing-symbol|src/foo.py|python");
  }

  #[test]
  fn parse_fingerprint_round_trips() {
    let path = repo_root().join("stdlib/lib/gate/algorithm-synthesis/ankh-retrieval-cache.px");
    let c = load_contract(&path, "ankh-retrieval-entries").unwrap();
    let fp = "lookup-module-providing-symbol|src/foo.py|python";
    let parsed = c.parse_fingerprint(fp).expect("parse fingerprint");
    assert_eq!(
      parsed.get("query_kind").unwrap(),
      "lookup-module-providing-symbol"
    );
    assert_eq!(parsed.get("target_path").unwrap(), "src/foo.py");
    assert_eq!(parsed.get("language").unwrap(), "python");
    // Round-trip
    assert_eq!(c.fingerprint(&parsed), fp);
  }

  #[test]
  fn parse_fingerprint_rejects_wrong_part_count() {
    let path = repo_root().join("stdlib/lib/gate/algorithm-synthesis/ankh-retrieval-cache.px");
    let c = load_contract(&path, "ankh-retrieval-entries").unwrap();
    assert!(c.parse_fingerprint("only|two").is_none());
    assert!(c.parse_fingerprint("").is_none());
  }

  #[test]
  fn missing_value_fields_reports_gaps() {
    let path = repo_root().join("stdlib/lib/gate/algorithm-synthesis/ankh-retrieval-cache.px");
    let c = load_contract(&path, "ankh-retrieval-entries").unwrap();
    let mut record = BTreeMap::new();
    record.insert("query_kind".to_string(), "x".to_string());
    record.insert("stored_at_ms".to_string(), "1".to_string());
    let missing = c.missing_value_fields(&record);
    assert!(missing.contains(&"provenance_source".to_string()));
    assert!(missing.contains(&"contributing_actor_id".to_string()));
  }

  #[test]
  fn missing_table_returns_error() {
    let path = repo_root().join("stdlib/lib/gate/algorithm-synthesis/ankh-retrieval-cache.px");
    let err = load_contract(&path, "ghost-table").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("ghost-table") || msg.contains("table_id"));
  }
}
