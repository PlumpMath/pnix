//! Read-only graph-mode `.px` corpus inventory.
//!
//! OWNER-LAW (2026-05-13): the algorithm corpus
//! (`examples/pnix_algo/**/*.px`) holds 1414+ files. Before any
//! evolution-lane / candidate-row-proposal logic can absorb them
//! as input, we need a typed *inventory* — what's there, in what
//! shape, with what entry counts. This module is that surface.
//!
//! Scope: **read-only**. Callers stage `(path, content)` pairs;
//! the inventory walker runs the existing `parse_pnix_graph_mode`
//! + `parse_list_entries` helpers and aggregates results. No disk
//! I/O, no edits, no dispatcher entry — inventory is a downstream
//! input for the evolution lane (other-session territory), not a
//! per-transform action.
//!
//! Output: a `PnixGraphInventory` aggregating per-file
//! `PnixGraphFileSummary` rows plus totals. Each summary is one
//! `coding.pnix-graph-inventory` artifact when serialized
//! (or many summaries roll up into a single inventory artifact —
//! caller's choice).

use super::pnix_graph::{parse_list_entries, parse_pnix_graph_mode};
use serde::{Deserialize, Serialize};
use pnix_hash::{Digest, Sha256};
use std::collections::BTreeMap;

/// Coarse classification of a `.px` file's syntactic shape. The
/// algorithm corpus is overwhelmingly `GraphMode`; the other modes
/// matter for evolution-lane routing (different cues / transforms
/// apply).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PnixGraphFileMode {
  /// Outer attrset with at least one of `externs` / `nodes` /
  /// `edges`. The canonical algorithm-corpus shape.
  GraphMode,
  /// Outer attrset present but no graph-defining sections —
  /// authoring-state or pure metadata file.
  TopAttrset,
  /// `let X = ...; in ...` form (stdlib owners, gate registries).
  ExpressionMode,
  /// Neither — comment-only files, malformed shells, etc.
  Unrecognized,
}

impl PnixGraphFileMode {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::GraphMode => "graph-mode",
      Self::TopAttrset => "top-attrset",
      Self::ExpressionMode => "expression-mode",
      Self::Unrecognized => "unrecognized",
    }
  }
}

/// Per-file inventory summary. Section counts cover the lists
/// where `parse_list_entries` can extract `{...}` entry boundaries
/// — `externs` / `nodes` / `edges`. Other sections (`types`,
/// `inputs`, `name`) get a `presence` bool flag instead.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PnixGraphFileSummary {
  pub path: String,
  pub mode: PnixGraphFileMode,
  /// Counts of `{ ... }` entries per list-shape section. Only
  /// populated for `GraphMode` files. Keys are section names from
  /// `pnix_graph::GRAPH_SECTIONS` that have `ValueShape::List`.
  pub section_entry_counts: BTreeMap<String, usize>,
  /// Presence-only flags for sections whose value shape is not a
  /// list of `{...}` (e.g. `name` ScalarString, `inputs` Attrset,
  /// `types` list of strings). `true` iff the section appears in
  /// the file.
  pub section_presence: BTreeMap<String, bool>,
  /// File byte length (no I/O — just the staged content's len).
  pub byte_len: usize,
}

/// Aggregate inventory over a set of `.px` files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PnixGraphInventory {
  pub files: Vec<PnixGraphFileSummary>,
  pub graph_mode_count: usize,
  pub top_attrset_count: usize,
  pub expression_mode_count: usize,
  pub unrecognized_count: usize,
  /// Sum of `section_entry_counts` across every graph-mode file.
  /// Lets the evolution lane ask "how many extern entries does the
  /// whole corpus have?" in one number.
  pub totals: BTreeMap<String, usize>,
}

/// Classify a single `.px` file's content. Pure function — no disk
/// I/O, no parser side effects. Returns a fully populated
/// `PnixGraphFileSummary`.
pub fn analyze_pnix_source(path: &str, source: &str) -> PnixGraphFileSummary {
  // Expression-mode probe runs FIRST. `let X = ...; in { ... }` —
  // the `in { ... }` attrset would otherwise be picked up as the
  // outer attrset of the graph-mode parser, mis-classifying the
  // file as `top-attrset` / `graph-mode`. The probe is narrow: a
  // `let` token at the start of the first non-comment non-whitespace
  // position commits the file to `ExpressionMode` before the
  // graph-mode parser runs.
  let first_meaningful = source
    .lines()
    .map(|l| l.trim_start())
    .find(|l| !l.is_empty() && !l.starts_with('#'))
    .unwrap_or("");
  let looks_like_let = first_meaningful.starts_with("let ")
    || first_meaningful.starts_with("let\t")
    || first_meaningful == "let";

  let parse = if looks_like_let {
    None
  } else {
    parse_pnix_graph_mode(source)
  };
  let mode = match &parse {
    Some(g) if g.looks_like_graph_mode() => PnixGraphFileMode::GraphMode,
    Some(_) => PnixGraphFileMode::TopAttrset,
    None => {
      if looks_like_let {
        PnixGraphFileMode::ExpressionMode
      } else {
        PnixGraphFileMode::Unrecognized
      }
    }
  };

  let mut section_entry_counts: BTreeMap<String, usize> = BTreeMap::new();
  let mut section_presence: BTreeMap<String, bool> = BTreeMap::new();

  if let Some(g) = &parse {
    // List-shape sections with `{...}` entries — count via
    // parse_list_entries.
    for section_name in ["externs", "nodes", "edges"] {
      match g.section(section_name) {
        Some(section) => {
          let entries = parse_list_entries(source, section).unwrap_or_default();
          section_entry_counts.insert(section_name.to_string(), entries.len());
        }
        None => {
          // Missing section is fine; just don't emit a count row.
        }
      }
    }
    // Presence-only sections.
    for section_name in ["name", "types", "inputs"] {
      section_presence.insert(section_name.to_string(), g.section(section_name).is_some());
    }
  }

  PnixGraphFileSummary {
    path: path.to_string(),
    mode,
    section_entry_counts,
    section_presence,
    byte_len: source.len(),
  }
}

/// Aggregate inventory over a slice of `(path, source)` pairs.
/// Caller is responsible for staging the file list (e.g. `glob`-
/// driven walk of `examples/pnix_algo/**/*.px`). The aggregator is
/// pure.
pub fn analyze_pnix_corpus(files: &[(&str, &str)]) -> PnixGraphInventory {
  let mut summaries: Vec<PnixGraphFileSummary> = Vec::with_capacity(files.len());
  let mut graph_mode_count = 0usize;
  let mut top_attrset_count = 0usize;
  let mut expression_mode_count = 0usize;
  let mut unrecognized_count = 0usize;
  let mut totals: BTreeMap<String, usize> = BTreeMap::new();

  for (path, content) in files {
    let summary = analyze_pnix_source(path, content);
    match summary.mode {
      PnixGraphFileMode::GraphMode => {
        graph_mode_count += 1;
        for (k, v) in &summary.section_entry_counts {
          *totals.entry(k.clone()).or_insert(0) += *v;
        }
      }
      PnixGraphFileMode::TopAttrset => top_attrset_count += 1,
      PnixGraphFileMode::ExpressionMode => expression_mode_count += 1,
      PnixGraphFileMode::Unrecognized => unrecognized_count += 1,
    }
    summaries.push(summary);
  }

  PnixGraphInventory {
    files: summaries,
    graph_mode_count,
    top_attrset_count,
    expression_mode_count,
    unrecognized_count,
    totals,
  }
}

/// Build the canonical JSON payload of a
/// `coding.pnix-graph-inventory` artifact.
pub fn build_pnix_graph_inventory_payload(inv: &PnixGraphInventory) -> serde_json::Value {
  let files_arr: Vec<serde_json::Value> = inv
    .files
    .iter()
    .map(|s| {
      let counts: serde_json::Map<String, serde_json::Value> = s
        .section_entry_counts
        .iter()
        .map(|(k, v)| (k.clone(), serde_json::Value::Number((*v as u64).into())))
        .collect();
      let presence: serde_json::Map<String, serde_json::Value> = s
        .section_presence
        .iter()
        .map(|(k, v)| (k.clone(), serde_json::Value::Bool(*v)))
        .collect();
      serde_json::json!({
        "path": s.path,
        "mode": s.mode.as_str(),
        "section_entry_counts": counts,
        "section_presence": presence,
        "byte_len": s.byte_len,
      })
    })
    .collect();
  let totals_obj: serde_json::Map<String, serde_json::Value> = inv
    .totals
    .iter()
    .map(|(k, v)| (k.clone(), serde_json::Value::Number((*v as u64).into())))
    .collect();
  serde_json::json!({
    "artifact": "pnix-graph-inventory",
    "owner_law": "crates/pnix-core::code_transform::pnix_graph_inventory",
    "file_count": inv.files.len(),
    "graph_mode_count": inv.graph_mode_count,
    "top_attrset_count": inv.top_attrset_count,
    "expression_mode_count": inv.expression_mode_count,
    "unrecognized_count": inv.unrecognized_count,
    "totals": totals_obj,
    "files": files_arr,
    "candidate_only": true,
    "next_step": "evolution-lane-candidate-row-proposal-input",
  })
}

/// Wrap a `PnixGraphInventory` as a
/// `coding.pnix-graph-inventory` artifact. Replay-stable id
/// over (file paths + per-file mode + per-file counts). Same
/// corpus → same id at any wall-clock time.
pub fn build_pnix_graph_inventory_artifact(
  inv: &PnixGraphInventory,
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
) -> serde_json::Value {
  let payload = build_pnix_graph_inventory_payload(inv);

  let mut hasher = Sha256::new();
  hasher.update(b"pnix-graph-inventory\x1f");
  for s in &inv.files {
    hasher.update(s.path.as_bytes());
    hasher.update(b"\x1e");
    hasher.update(s.mode.as_str().as_bytes());
    hasher.update(b"\x1e");
    for (k, v) in &s.section_entry_counts {
      hasher.update(k.as_bytes());
      hasher.update(b":");
      hasher.update(v.to_le_bytes());
      hasher.update(b"\x1c");
    }
    hasher.update(b"\x1d");
  }
  let digest = hasher.finalize();
  let prefix = digest
    .iter()
    .take(8)
    .map(|b| format!("{b:02x}"))
    .collect::<String>();
  let id = format!("pnix-graph-inventory.{prefix}");

  let target_paths: Vec<serde_json::Value> = inv
    .files
    .iter()
    .map(|s| serde_json::Value::String(s.path.clone()))
    .collect();

  let mut artifact = serde_json::json!({
    "id": id,
    "artifact_family": "coding.pnix-graph-inventory",
    "source_surface": "code-transform.pnix-graph-inventory",
    "stored_at_ms": stored_at_ms,
    "target_paths": target_paths,
    "command_refs": serde_json::Value::Array(Vec::new()),
    "related_refs": serde_json::json!([
      "owner-law:crates/pnix-core::code_transform::pnix_graph_inventory"
    ]),
    "payload": payload,
  });
  if let Some(snap) = repo_snapshot_ref {
    artifact["repo_snapshot_ref"] = serde_json::Value::String(snap.to_string());
  }
  artifact
}

#[cfg(test)]
mod tests {
  use super::*;

  const GRAPH_SRC: &str = r#"{
  name = "g";
  types = [ "Num" "Bool" ];
  externs = [
    { name = "builtins.add"; }
    { name = "builtins.sub"; }
  ];
  inputs = {
    a = "Num";
    b = "Num";
  };
  nodes = [
    { name = "first"; uses = "builtins.add"; }
    { name = "second"; uses = "builtins.sub"; }
    { name = "third"; uses = "builtins.add"; }
  ];
  edges = [
    { from = { input = "a"; }; to = { node = "first"; port = "lhs"; }; }
  ];
}
"#;

  const EXPRESSION_SRC: &str = r#"# stdlib owner
let
  ownerName = "stdlib/lib/example.px";
  thing = x: x + 1;
in {
  inherit ownerName thing;
}
"#;

  const TOP_ATTRSET_NO_GRAPH_SECTIONS: &str = r#"{
  name = "metadata-only";
  description = "no graph-defining sections here";
}
"#;

  const UNRECOGNIZED_SRC: &str = "# pure comment\n# nothing else\n";

  // ─── analyze_pnix_source ─────────────────────────────────────

  #[test]
  fn analyze_classifies_graph_mode_file() {
    let s = analyze_pnix_source("g.px", GRAPH_SRC);
    assert_eq!(s.mode, PnixGraphFileMode::GraphMode);
    assert_eq!(s.section_entry_counts.get("externs"), Some(&2));
    assert_eq!(s.section_entry_counts.get("nodes"), Some(&3));
    assert_eq!(s.section_entry_counts.get("edges"), Some(&1));
    // Presence flags for non-list sections.
    assert_eq!(s.section_presence.get("name"), Some(&true));
    assert_eq!(s.section_presence.get("inputs"), Some(&true));
    assert_eq!(s.section_presence.get("types"), Some(&true));
    assert_eq!(s.byte_len, GRAPH_SRC.len());
  }

  #[test]
  fn analyze_classifies_expression_mode_file() {
    let s = analyze_pnix_source("stdlib/lib/example.px", EXPRESSION_SRC);
    assert_eq!(s.mode, PnixGraphFileMode::ExpressionMode);
    assert!(s.section_entry_counts.is_empty());
    assert!(s.section_presence.is_empty());
  }

  #[test]
  fn analyze_classifies_top_attrset_without_graph_sections() {
    let s = analyze_pnix_source("meta.px", TOP_ATTRSET_NO_GRAPH_SECTIONS);
    assert_eq!(s.mode, PnixGraphFileMode::TopAttrset);
    // No list-shape sections → no entry counts.
    assert!(s.section_entry_counts.is_empty());
    // Presence flags still populated — `name` is a recognized
    // section, and even non-graph-mode top-level attrsets can carry
    // one. The flag's value reflects what's actually there.
    assert_eq!(s.section_presence.get("name"), Some(&true));
    assert_eq!(s.section_presence.get("inputs"), Some(&false));
    assert_eq!(s.section_presence.get("types"), Some(&false));
  }

  #[test]
  fn analyze_classifies_unrecognized_comment_only_file() {
    let s = analyze_pnix_source("notes.px", UNRECOGNIZED_SRC);
    assert_eq!(s.mode, PnixGraphFileMode::Unrecognized);
  }

  // ─── analyze_pnix_corpus ─────────────────────────────────────

  #[test]
  fn aggregate_inventory_counts_per_mode() {
    let files = [
      ("a.px", GRAPH_SRC),
      ("b.px", GRAPH_SRC),
      ("c.px", EXPRESSION_SRC),
      ("d.px", TOP_ATTRSET_NO_GRAPH_SECTIONS),
      ("e.px", UNRECOGNIZED_SRC),
    ];
    let inv = analyze_pnix_corpus(&files);
    assert_eq!(inv.files.len(), 5);
    assert_eq!(inv.graph_mode_count, 2);
    assert_eq!(inv.expression_mode_count, 1);
    assert_eq!(inv.top_attrset_count, 1);
    assert_eq!(inv.unrecognized_count, 1);
  }

  #[test]
  fn aggregate_totals_sum_section_entry_counts_across_graph_files() {
    let files = [("a.px", GRAPH_SRC), ("b.px", GRAPH_SRC)];
    let inv = analyze_pnix_corpus(&files);
    // 2 graph files × (2 externs + 3 nodes + 1 edge) = 4 + 6 + 2.
    assert_eq!(inv.totals.get("externs"), Some(&4));
    assert_eq!(inv.totals.get("nodes"), Some(&6));
    assert_eq!(inv.totals.get("edges"), Some(&2));
  }

  #[test]
  fn aggregate_empty_corpus_yields_empty_inventory() {
    let inv = analyze_pnix_corpus(&[]);
    assert_eq!(inv.files.len(), 0);
    assert_eq!(inv.graph_mode_count, 0);
    assert!(inv.totals.is_empty());
  }

  #[test]
  fn aggregate_only_expression_files_yields_zero_totals() {
    let files = [("a.px", EXPRESSION_SRC), ("b.px", EXPRESSION_SRC)];
    let inv = analyze_pnix_corpus(&files);
    assert_eq!(inv.expression_mode_count, 2);
    assert_eq!(inv.graph_mode_count, 0);
    assert!(inv.totals.is_empty());
  }

  // ─── artifact builder ────────────────────────────────────────

  #[test]
  fn artifact_canonical_envelope_fields() {
    let inv = analyze_pnix_corpus(&[("g.px", GRAPH_SRC)]);
    let art = build_pnix_graph_inventory_artifact(&inv, 1_700_000_000_000, None);
    assert_eq!(
      art["artifact_family"].as_str(),
      Some("coding.pnix-graph-inventory")
    );
    assert_eq!(
      art["source_surface"].as_str(),
      Some("code-transform.pnix-graph-inventory")
    );
    assert!(art["id"]
      .as_str()
      .unwrap()
      .starts_with("pnix-graph-inventory."));
    assert_eq!(art["target_paths"][0].as_str(), Some("g.px"));
    let payload = &art["payload"];
    assert_eq!(payload["file_count"].as_u64(), Some(1));
    assert_eq!(payload["graph_mode_count"].as_u64(), Some(1));
    assert_eq!(payload["candidate_only"].as_bool(), Some(true));
    assert_eq!(
      payload["next_step"].as_str(),
      Some("evolution-lane-candidate-row-proposal-input")
    );
  }

  #[test]
  fn artifact_id_replay_stable_across_stored_at_ms() {
    let inv = analyze_pnix_corpus(&[("g.px", GRAPH_SRC)]);
    let a = build_pnix_graph_inventory_artifact(&inv, 0, None);
    let b = build_pnix_graph_inventory_artifact(&inv, 9_999_999, None);
    assert_eq!(a["id"], b["id"]);
    assert_ne!(a["stored_at_ms"], b["stored_at_ms"]);
  }

  #[test]
  fn artifact_id_differs_when_corpus_membership_changes() {
    let a =
      build_pnix_graph_inventory_artifact(&analyze_pnix_corpus(&[("g.px", GRAPH_SRC)]), 0, None);
    let b = build_pnix_graph_inventory_artifact(
      &analyze_pnix_corpus(&[("g.px", GRAPH_SRC), ("h.px", GRAPH_SRC)]),
      0,
      None,
    );
    assert_ne!(a["id"], b["id"]);
  }

  #[test]
  fn artifact_payload_files_carries_per_file_summary_shape() {
    let inv = analyze_pnix_corpus(&[("graph.px", GRAPH_SRC), ("expr.px", EXPRESSION_SRC)]);
    let art = build_pnix_graph_inventory_artifact(&inv, 0, None);
    let files = art["payload"]["files"].as_array().expect("files array");
    assert_eq!(files.len(), 2);
    let graph_entry = files
      .iter()
      .find(|f| f["path"].as_str() == Some("graph.px"))
      .expect("graph.px entry");
    assert_eq!(graph_entry["mode"].as_str(), Some("graph-mode"));
    assert_eq!(
      graph_entry["section_entry_counts"]["nodes"].as_u64(),
      Some(3)
    );
    assert_eq!(
      graph_entry["section_presence"]["inputs"].as_bool(),
      Some(true)
    );
    let expr_entry = files
      .iter()
      .find(|f| f["path"].as_str() == Some("expr.px"))
      .expect("expr.px entry");
    assert_eq!(expr_entry["mode"].as_str(), Some("expression-mode"));
  }

  #[test]
  fn artifact_totals_aggregate_across_all_graph_mode_files() {
    let inv = analyze_pnix_corpus(&[
      ("a.px", GRAPH_SRC),
      ("b.px", GRAPH_SRC),
      ("c.px", EXPRESSION_SRC),
    ]);
    let art = build_pnix_graph_inventory_artifact(&inv, 0, None);
    let totals = &art["payload"]["totals"];
    assert_eq!(totals["externs"].as_u64(), Some(4));
    assert_eq!(totals["nodes"].as_u64(), Some(6));
    assert_eq!(totals["edges"].as_u64(), Some(2));
  }
}
