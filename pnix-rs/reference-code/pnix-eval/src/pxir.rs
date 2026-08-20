//! Perf P4: canonical PxIR artifact record (host transport metadata).
//!
//! PxIR does not replace interpretation; it captures deterministic
//! fingerprints and export tables for pxmeta manifest stale detection.

use serde::{Deserialize, Serialize};

/// Canonical PxIR row for one `.px` owner surface.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PxirArtifactRecord {
  pub rel_path: String,
  pub source_hash: String,
  pub dependency_hash: String,
  pub symbol_table: Vec<String>,
  pub owner_table: Vec<String>,
  pub gate_table: Vec<String>,
  pub dispatch_table: Vec<String>,
  pub receipt_schema_table: Vec<String>,
  pub effect_lock_table: Vec<String>,
  pub call_graph: Vec<String>,
  pub compiler_version: String,
  pub evaluator_version: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ClassifiedSymbolTables {
  pub owner_table: Vec<String>,
  pub gate_table: Vec<String>,
  pub dispatch_table: Vec<String>,
  pub receipt_schema_table: Vec<String>,
  pub effect_lock_table: Vec<String>,
}

fn push_sorted_unique(target: &mut Vec<String>, name: String) {
  if !target.iter().any(|existing| existing == &name) {
    target.push(name);
  }
}

fn sort_tables(tables: &mut ClassifiedSymbolTables) {
  tables.owner_table.sort();
  tables.gate_table.sort();
  tables.dispatch_table.sort();
  tables.receipt_schema_table.sort();
  tables.effect_lock_table.sort();
}

/// Classify export names into closed PxIR table buckets (deterministic).
pub fn classify_symbol_tables(symbol_table: &[String]) -> ClassifiedSymbolTables {
  let mut tables = ClassifiedSymbolTables::default();
  for name in symbol_table {
    let lower = name.to_ascii_lowercase();
    if matches!(
      name.as_str(),
      "supportedOps"
        | "mirrorPlateLensRegistry"
        | "selectWinner"
        | "rankCandidates"
        | "routeEvaluateSelect"
        | "dispatchRows"
        | "dispatchTable"
        | "dispatchUtteranceWithContext"
    ) {
      push_sorted_unique(&mut tables.dispatch_table, name.clone());
      continue;
    }
    if matches!(
      name.as_str(),
      "reasoningOwnerPathManifest" | "knownTask6wSourceFamilies"
    ) {
      push_sorted_unique(&mut tables.owner_table, name.clone());
      continue;
    }
    if lower.contains("effect") && lower.contains("lock") {
      push_sorted_unique(&mut tables.effect_lock_table, name.clone());
      continue;
    }
    if lower.contains("receipt")
      && (lower.contains("schema") || lower.contains("outcome") || lower.contains("verdict"))
    {
      push_sorted_unique(&mut tables.receipt_schema_table, name.clone());
      continue;
    }
    if lower.contains("dispatch") {
      push_sorted_unique(&mut tables.dispatch_table, name.clone());
      continue;
    }
    if lower.contains("owner") || lower.ends_with("pathmanifest") {
      push_sorted_unique(&mut tables.owner_table, name.clone());
      continue;
    }
    if lower.contains("gate") {
      push_sorted_unique(&mut tables.gate_table, name.clone());
    }
  }
  sort_tables(&mut tables);
  tables
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn classify_symbol_tables_pins_known_registry_exports() {
    let symbols = vec![
      "selectWinner".to_string(),
      "rankCandidates".to_string(),
      "requiredAxes".to_string(),
      "supportedOps".to_string(),
      "reasoningOwnerPathManifest".to_string(),
    ];
    let tables = classify_symbol_tables(&symbols);
    assert_eq!(
      tables.dispatch_table,
      vec!["rankCandidates", "selectWinner", "supportedOps"]
    );
    assert_eq!(tables.owner_table, vec!["reasoningOwnerPathManifest"]);
    assert!(tables.gate_table.is_empty());
  }
}
