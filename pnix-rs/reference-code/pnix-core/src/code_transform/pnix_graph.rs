//! Graph-mode `.px` parser + minimal editors.
//!
//! OWNER-LAW (2026-05-13): the algorithm corpus (`examples/pnix_algo`
//! and the 1414-file expanded set) uses a Nix-flavored graph DSL
//! distinct from the `let X = ...; in ...` expression mode used by
//! stdlib/lib owners. A typical graph-mode `.px` file is a top-level
//! attrset like:
//!
//! ```text
//! {
//!   name = "...";
//!   types = [ "Num" "Bool" ];
//!   externs = [ { name = "..."; inputs = [...]; outputs = [...]; } ... ];
//!   inputs = { a = "Num"; b = "Num"; };
//!   nodes = [ { name = "..."; uses = "..."; } ... ];
//!   edges = [ { from = ...; to = ...; } ... ];
//! }
//! ```
//!
//! This module gives the host CST emit layer a typed lift of those
//! sections — byte ranges only, no semantic evaluation. Downstream
//! transforms (D-1: `add-extern`; D-2: `add-node`; D-3: `add-edge`;
//! D-4: `rename-node-id`) build on this parser.
//!
//! Substrate-share discipline (per `freecat-cli` CLAUDE.md §1.1 /
//! `pnix` CLAUDE.md §14): this is **registry-driven** — section names
//! live in `GRAPH_SECTIONS`, the parser is one generic scan, and
//! adding a new graph-mode section = one new entry, no new branch.
//! No hardcoded if-else per section.
//!
//! No semantic interpretation here. Section bodies are returned as
//! `(start, end)` byte ranges; downstream transforms parse the
//! entries they need.
//!
//! `#` line comments and `"..."` / `''...''` strings inside the
//! source are honored via `rename_symbol::pnix_skip_zones`, so a
//! literal `externs = [...]` inside a string body or comment
//! does NOT register as a real section.

use super::rename_symbol::{pnix_skip_zones, SkipZone};
use serde::{Deserialize, Serialize};
use pnix_hash::{Digest, Sha256};

/// Canonical graph-mode section names recognized by the parser. The
/// `value_shape` tag describes the immediate syntactic shape of the
/// section body so downstream transforms know whether they're
/// dealing with a list-of-attrset (`[ ... ]`) or a single attrset
/// (`{ ... }`).
///
/// Adding a new graph-mode section name = one new entry. The
/// generic parser walks this slice and tries each in turn.
pub const GRAPH_SECTIONS: &[GraphSectionSpec] = &[
  GraphSectionSpec {
    name: "name",
    value_shape: ValueShape::ScalarString,
  },
  GraphSectionSpec {
    name: "types",
    value_shape: ValueShape::List,
  },
  GraphSectionSpec {
    name: "externs",
    value_shape: ValueShape::List,
  },
  GraphSectionSpec {
    name: "inputs",
    value_shape: ValueShape::Attrset,
  },
  GraphSectionSpec {
    name: "nodes",
    value_shape: ValueShape::List,
  },
  GraphSectionSpec {
    name: "edges",
    value_shape: ValueShape::List,
  },
];

/// One registry row for `GRAPH_SECTIONS`. Pure data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GraphSectionSpec {
  pub name: &'static str,
  pub value_shape: ValueShape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueShape {
  /// `"..."` quoted string scalar.
  ScalarString,
  /// `[ ... ]` list.
  List,
  /// `{ ... }` attrset.
  Attrset,
}

/// Located graph-mode section in a source string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PnixGraphSection {
  /// Canonical section name (e.g. `"externs"`).
  pub name: &'static str,
  /// `value_shape` from `GRAPH_SECTIONS`.
  pub value_shape: ValueShape,
  /// Byte offset of the first character of the section name in source.
  pub name_start_byte: usize,
  /// Byte offset just past the closing bracket / brace of the body.
  pub body_end_byte: usize,
  /// Byte offset just past the opening bracket / brace of the body.
  /// For `ScalarString`, this is the byte offset of the opening `"`.
  pub body_inner_start_byte: usize,
  /// Byte offset of the closing bracket / brace / `"`.
  pub body_inner_end_byte: usize,
}

/// Top-level graph-mode shape extracted from a `.px` source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PnixGraphMode {
  pub sections: Vec<PnixGraphSection>,
}

impl PnixGraphMode {
  /// Look up a section by its canonical name. Returns the first
  /// matching section (graph-mode files have at most one of each).
  pub fn section(&self, name: &str) -> Option<&PnixGraphSection> {
    self.sections.iter().find(|s| s.name == name)
  }

  /// True if this shape carries at least one graph-mode-defining
  /// section. The bare `name = "..."` row alone is NOT enough — many
  /// expression-mode files also bind `name = ...` for unrelated
  /// reasons. The graph-defining sections are
  /// `externs` / `nodes` / `edges`; presence of any one of those is
  /// the discriminator.
  pub fn looks_like_graph_mode(&self) -> bool {
    self
      .sections
      .iter()
      .any(|s| matches!(s.name, "externs" | "nodes" | "edges"))
  }
}

/// Parse a `.px` source string into a `PnixGraphMode` shape. Returns
/// `None` only on a fundamental shape failure (e.g. the source is
/// empty); otherwise returns whatever sections the parser could
/// locate. Caller uses `looks_like_graph_mode` to decide if the
/// result is actually a graph-mode file.
///
/// OWNER-LAW (2026-05-13): conservative scanner. The Nix language has
/// many ways to write `X = ...` (e.g. inside nested attrsets, inside
/// `let ... in` bindings); this parser intentionally only matches
/// the **top-level outer attrset**'s direct bindings — the
/// canonical algorithm-corpus shape. Future slices can widen this.
pub fn parse_pnix_graph_mode(source: &str) -> Option<PnixGraphMode> {
  if source.is_empty() {
    return None;
  }
  let bytes = source.as_bytes();
  let skip = pnix_skip_zones(source);

  // Top-level outer attrset opens with `{` after optional whitespace
  // / comments. Find it. (Skip zones already account for `#` and
  // strings.)
  let outer_open = find_top_level_open_brace(bytes, &skip)?;
  let outer_close = find_matching_close_brace(bytes, &skip, outer_open)?;

  // Walk the bytes from outer_open+1 to outer_close at depth-0
  // relative to the outer attrset. Direct bindings have:
  //   <ident> = <value> ;
  // where ident is one of the GRAPH_SECTIONS names.
  let mut sections: Vec<PnixGraphSection> = Vec::new();
  for spec in GRAPH_SECTIONS {
    if let Some(s) = locate_direct_binding(bytes, &skip, outer_open, outer_close, spec) {
      sections.push(s);
    }
  }

  Some(PnixGraphMode { sections })
}

/// Find the byte offset of the first `{` at top level — outside any
/// skip zone. None if no `{` exists in source.
fn find_top_level_open_brace(bytes: &[u8], skip: &[SkipZone]) -> Option<usize> {
  let mut i = 0usize;
  while i < bytes.len() {
    if in_skip_zone(skip, i) {
      i = skip_zone_end(skip, i).unwrap_or(i + 1);
      continue;
    }
    if bytes[i] == b'{' {
      return Some(i);
    }
    i += 1;
  }
  None
}

/// Brace-balanced match: find the `}` matching `open_pos` (a `{`).
/// Honors skip zones (braces inside strings/comments don't count).
fn find_matching_close_brace(bytes: &[u8], skip: &[SkipZone], open_pos: usize) -> Option<usize> {
  if open_pos >= bytes.len() || bytes[open_pos] != b'{' {
    return None;
  }
  let mut depth = 1i32;
  let mut i = open_pos + 1;
  while i < bytes.len() {
    if in_skip_zone(skip, i) {
      i = skip_zone_end(skip, i).unwrap_or(i + 1);
      continue;
    }
    match bytes[i] {
      b'{' => depth += 1,
      b'}' => {
        depth -= 1;
        if depth == 0 {
          return Some(i);
        }
      }
      _ => {}
    }
    i += 1;
  }
  None
}

/// Bracket-balanced match: find the `]` matching `open_pos` (a `[`).
fn find_matching_close_bracket(bytes: &[u8], skip: &[SkipZone], open_pos: usize) -> Option<usize> {
  if open_pos >= bytes.len() || bytes[open_pos] != b'[' {
    return None;
  }
  let mut depth = 1i32;
  let mut i = open_pos + 1;
  while i < bytes.len() {
    if in_skip_zone(skip, i) {
      i = skip_zone_end(skip, i).unwrap_or(i + 1);
      continue;
    }
    match bytes[i] {
      b'[' => depth += 1,
      b']' => {
        depth -= 1;
        if depth == 0 {
          return Some(i);
        }
      }
      _ => {}
    }
    i += 1;
  }
  None
}

/// Find a closing `"` for a string starting at `open_pos`. Honors
/// `\"` escape (one byte of escape skip).
fn find_matching_close_quote(bytes: &[u8], open_pos: usize) -> Option<usize> {
  if open_pos >= bytes.len() || bytes[open_pos] != b'"' {
    return None;
  }
  let mut i = open_pos + 1;
  while i < bytes.len() {
    if bytes[i] == b'\\' {
      i += 2;
      continue;
    }
    if bytes[i] == b'"' {
      return Some(i);
    }
    i += 1;
  }
  None
}

fn in_skip_zone(skip: &[SkipZone], byte: usize) -> bool {
  skip.iter().any(|z| byte >= z.start && byte < z.end)
}

fn skip_zone_end(skip: &[SkipZone], byte: usize) -> Option<usize> {
  skip
    .iter()
    .find(|z| byte >= z.start && byte < z.end)
    .map(|z| z.end)
}

/// Locate a direct binding `<spec.name> = <value>` inside the outer
/// attrset spanning `(outer_open, outer_close)`. Walks the bytes at
/// depth-0 relative to the outer attrset (i.e. ignores bindings
/// inside nested attrsets).
fn locate_direct_binding(
  bytes: &[u8],
  skip: &[SkipZone],
  outer_open: usize,
  outer_close: usize,
  spec: &GraphSectionSpec,
) -> Option<PnixGraphSection> {
  let needle = spec.name.as_bytes();
  let mut i = outer_open + 1;
  let mut depth_brace = 0i32;
  let mut depth_bracket = 0i32;
  while i < outer_close {
    if in_skip_zone(skip, i) {
      i = skip_zone_end(skip, i).unwrap_or(i + 1);
      continue;
    }
    match bytes[i] {
      b'{' => {
        depth_brace += 1;
        i += 1;
        continue;
      }
      b'}' => {
        if depth_brace > 0 {
          depth_brace -= 1;
        }
        i += 1;
        continue;
      }
      b'[' => {
        depth_bracket += 1;
        i += 1;
        continue;
      }
      b']' => {
        if depth_bracket > 0 {
          depth_bracket -= 1;
        }
        i += 1;
        continue;
      }
      _ => {}
    }
    // Only consider matches at depth-0 (direct binding in outer
    // attrset).
    if depth_brace != 0 || depth_bracket != 0 {
      i += 1;
      continue;
    }
    // Token boundary check: previous byte must NOT be an identifier
    // char.
    let preceded_by_ident =
      i > outer_open + 1 && (bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_');
    if preceded_by_ident {
      i += 1;
      continue;
    }
    // Does this position match the spec.name token?
    if i + needle.len() > outer_close {
      i += 1;
      continue;
    }
    if &bytes[i..i + needle.len()] != needle {
      i += 1;
      continue;
    }
    // Trailing token-boundary check.
    let after_idx = i + needle.len();
    if after_idx < outer_close
      && (bytes[after_idx].is_ascii_alphanumeric() || bytes[after_idx] == b'_')
    {
      i = after_idx;
      continue;
    }
    // Skip whitespace, then expect `=`.
    let mut j = after_idx;
    while j < outer_close && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\n') {
      j += 1;
    }
    if j >= outer_close || bytes[j] != b'=' {
      i = after_idx;
      continue;
    }
    j += 1;
    while j < outer_close && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\n') {
      j += 1;
    }
    if j >= outer_close {
      i = after_idx;
      continue;
    }
    // Match value shape.
    let value_open = j;
    let body_inner_start;
    let body_inner_end;
    let body_end;
    match (spec.value_shape, bytes[value_open]) {
      (ValueShape::List, b'[') => {
        let close = find_matching_close_bracket(bytes, skip, value_open)?;
        body_inner_start = value_open + 1;
        body_inner_end = close;
        body_end = close + 1;
      }
      (ValueShape::Attrset, b'{') => {
        let close = find_matching_close_brace(bytes, skip, value_open)?;
        body_inner_start = value_open + 1;
        body_inner_end = close;
        body_end = close + 1;
      }
      (ValueShape::ScalarString, b'"') => {
        let close = find_matching_close_quote(bytes, value_open)?;
        body_inner_start = value_open + 1;
        body_inner_end = close;
        body_end = close + 1;
      }
      _ => {
        // Shape mismatch — caller's source uses a non-canonical
        // value form for this section. Skip.
        i = after_idx;
        continue;
      }
    }
    return Some(PnixGraphSection {
      name: spec.name,
      value_shape: spec.value_shape,
      name_start_byte: i,
      body_end_byte: body_end,
      body_inner_start_byte: body_inner_start,
      body_inner_end_byte: body_inner_end,
    });
  }
  None
}

/// Generic editor: insert a new entry into a `ValueShape::List`
/// section of a graph-mode `.px` source. Preserves the closing `]`'s
/// existing indentation and gives the new entry +2 spaces of nesting
/// indent.
///
/// Returns `None` if the section is missing, isn't a list, or the
/// source is malformed. `entry_text` is wrapped in `{ ... ; }` —
/// pass only the inner attrset body, e.g. `name = "py.add"`.
///
/// OWNER-LAW (2026-05-13): single source of truth for graph-mode
/// list-section appends. `add_extern_entry_to_source`,
/// `add_node_entry_to_source`, and the future
/// `add_edge_entry_to_source` are all one-line thin wrappers over
/// this function — registry-driven per CLAUDE.md §14, no per-section
/// duplication.
pub fn add_entry_to_list_section(
  source: &str,
  section_name: &str,
  entry_text: &str,
) -> Option<String> {
  let graph = parse_pnix_graph_mode(source)?;
  let section = graph.section(section_name)?;
  if !matches!(section.value_shape, ValueShape::List) {
    return None;
  }
  let bytes = source.as_bytes();
  let close_pos = section.body_inner_end_byte;
  let line_start = bytes[..close_pos]
    .iter()
    .rposition(|&b| b == b'\n')
    .map(|n| n + 1)
    .unwrap_or(0);
  let indent_run: &[u8] = &bytes[line_start..close_pos];
  let close_indent: String = indent_run
    .iter()
    .take_while(|&&b| b == b' ' || b == b'\t')
    .map(|&b| b as char)
    .collect();
  let entry_indent = format!("{close_indent}  ");
  let new_entry = format!("{entry_indent}{{ {entry_text}; }}\n");

  let mut out = String::with_capacity(source.len() + new_entry.len());
  out.push_str(&source[..close_pos]);
  if !out.ends_with('\n') {
    out.push('\n');
  }
  out.push_str(&new_entry);
  out.push_str(&close_indent);
  out.push_str(&source[close_pos..]);
  Some(out)
}

/// Insert a new entry into the `externs` list of a graph-mode `.px`
/// source. Thin wrapper over `add_entry_to_list_section("externs",
/// ...)`. `entry_text` is wrapped in `{ ... ; }` — pass only the
/// inner attrset body, e.g. `name = "py.add"`.
///
/// Returns `None` if the source has no `externs = [ ... ]` section,
/// or if the source is malformed.
pub fn add_extern_entry_to_source(source: &str, entry_text: &str) -> Option<String> {
  add_entry_to_list_section(source, "externs", entry_text)
}

/// Insert a new entry into the `nodes` list of a graph-mode `.px`
/// source. Thin wrapper over `add_entry_to_list_section("nodes",
/// ...)`. `entry_text` is wrapped in `{ ... ; }`.
///
/// Canonical node entry shape (from the algorithm corpus):
///
/// ```text
/// { name = "is_target"; uses = "builtins.eq"; }
/// ```
///
/// Caller passes `name = "is_target"; uses = "builtins.eq"` and the
/// wrapper adds the outer braces.
pub fn add_node_entry_to_source(source: &str, entry_text: &str) -> Option<String> {
  add_entry_to_list_section(source, "nodes", entry_text)
}

/// Typed builder for an `externs` list entry. v0 carries the
/// minimal corpus shape (`{ name = "X"; }`); future slices can
/// extend with `inputs` / `outputs` nested attrset lists.
///
/// OWNER-LAW (2026-05-13): caller-side ergonomics. Plug
/// `build_extern_entry_text(&spec)` into
/// `add_extern_entry_to_source(source, &entry)` or into the
/// `add-pnix-extern` dispatcher request's `entry_text` field — same
/// raw string the caller would otherwise hand-write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternSpec<'a> {
  pub name: &'a str,
}

/// Render an `ExternSpec` as the inner body of an `externs`
/// list entry — no outer `{ ... ; }` braces (those are added by the
/// list inserter).
pub fn build_extern_entry_text(spec: &ExternSpec<'_>) -> String {
  format!(r#"name = "{name}""#, name = spec.name)
}

/// Typed builder for a `nodes` list entry. Carries the canonical
/// corpus fields: `name`, `uses`, and the optional `gate` flag (a
/// few corpus rows mark gating nodes via `gate = true`). v0 stops
/// at these three fields — future slices can extend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeSpec<'a> {
  pub name: &'a str,
  pub uses: &'a str,
  /// Optional gating flag. `None` omits the field entirely; `Some`
  /// emits `gate = true` / `gate = false`.
  pub gate: Option<bool>,
}

/// Render a `NodeSpec` as the inner body of a `nodes` list entry.
pub fn build_node_entry_text(spec: &NodeSpec<'_>) -> String {
  let mut out = format!(
    r#"name = "{name}"; uses = "{uses}""#,
    name = spec.name,
    uses = spec.uses
  );
  if let Some(g) = spec.gate {
    out.push_str(&format!("; gate = {g}"));
  }
  out
}

/// One end of a graph-mode edge. Mirrors the canonical corpus
/// shapes seen in `examples/pnix_algo/**/*.px`:
///
///   - `{ input = "current"; }`             — graph input port
///   - `{ node = "is_target"; port = "a"; }` — named port of a node
///
/// Some corpus rows omit `port` (single-output convention); both
/// `Some` and `None` are accepted by the builder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EdgeEndpoint<'a> {
  /// `{ input = "<name>"; }` — references a graph input by name.
  Input { name: &'a str },
  /// `{ node = "<name>"; port = "<port>"; }` — references a node's
  /// named port; or `{ node = "<name>"; }` when `port` is `None`.
  Node {
    name: &'a str,
    port: Option<&'a str>,
  },
}

impl<'a> EdgeEndpoint<'a> {
  /// Render the endpoint as a Nix attrset literal (no surrounding
  /// braces — caller handles the `from = { ... }` framing).
  fn render_inner(&self) -> String {
    match self {
      Self::Input { name } => format!("input = \"{name}\""),
      Self::Node { name, port: None } => format!("node = \"{name}\""),
      Self::Node {
        name,
        port: Some(p),
      } => format!("node = \"{name}\"; port = \"{p}\""),
    }
  }
}

/// Owned variant of [`EdgeEndpoint`] for the dispatcher / request
/// JSON layer (lifetimes don't cross JSON boundaries cleanly).
///
/// OWNER-LAW (2026-05-13): `EdgeEndpoint<'a>` is the borrowed
/// builder shape — caller has `&str`s already. `OwnedEdgeEndpoint`
/// is the typed serializable shape — caller builds it from a JSON
/// request or constructs from owned strings. Both render to the
/// same canonical Nix attrset body via `render_inner()`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum OwnedEdgeEndpoint {
  Input {
    name: String,
  },
  Node {
    name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    port: Option<String>,
  },
}

impl OwnedEdgeEndpoint {
  /// Render as the inner body of a `from = { ... }` /
  /// `to = { ... }` attrset (no outer braces).
  pub fn render_inner(&self) -> String {
    match self {
      Self::Input { name } => format!("input = \"{name}\""),
      Self::Node { name, port: None } => format!("node = \"{name}\""),
      Self::Node {
        name,
        port: Some(p),
      } => format!("node = \"{name}\"; port = \"{p}\""),
    }
  }
}

/// Build the inner body of an `edges = [ ... ]` entry (no outer
/// `{ ... ; }` framing — that comes from
/// `add_entry_to_list_section`). Output shape:
///
/// ```text
/// from = { input = "current"; }; to = { node = "x"; port = "a"; }
/// ```
///
/// OWNER-LAW (2026-05-13): callers that want typed edges should use
/// this builder + `add_edge_entry_to_source`. Callers with already-
/// formatted entry text can pass it straight to
/// `add_edge_entry_to_source` (registry-driven thin wrapper).
pub fn build_edge_entry_text(from: &EdgeEndpoint<'_>, to: &EdgeEndpoint<'_>) -> String {
  format!(
    "from = {{ {from_body}; }}; to = {{ {to_body}; }}",
    from_body = from.render_inner(),
    to_body = to.render_inner()
  )
}

/// Insert a new entry into the `edges` list of a graph-mode `.px`
/// source. Thin wrapper over `add_entry_to_list_section("edges",
/// ...)`. `entry_text` is wrapped in `{ ... ; }`.
///
/// Use `build_edge_entry_text(from, to)` to construct the entry from
/// typed `EdgeEndpoint` values, or pass a pre-formatted body
/// directly.
///
/// Canonical edge entry shapes from the algorithm corpus:
///
/// ```text
/// { from = { input = "current"; };          to = { node = "x"; port = "a"; }; }
/// { from = { node = "y"; port = "out"; };   to = { node = "z"; port = "b"; }; }
/// ```
pub fn add_edge_entry_to_source(source: &str, entry_text: &str) -> Option<String> {
  add_entry_to_list_section(source, "edges", entry_text)
}

/// Errors raised by [`remove_edge_entry_by_from_to_in_source`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoveEdgeError {
  /// Source has no graph-mode shape.
  NotGraphMode,
  /// No `edges = [ ... ]` section in source.
  NoEdgesSection,
  /// Edges section has unbalanced braces.
  MalformedEdgesSection,
  /// No edge entry matches the (from, to) substring pair.
  EdgeNotFound,
}

/// Remove every edge entry whose body matches both
/// `from = { <from_body>; }` AND `to = { <to_body>; }` substrings.
///
/// Match semantics: substring of the rendered Nix bodies. The
/// canonical corpus emits one specific spacing (single space around
/// `=`, trailing `;`), and so does `build_edge_entry_text` / the
/// `add-pnix-edge` carrier. Edges authored by hand with different
/// whitespace won't be matched — a future CST-aware variant could
/// refine.
///
/// Removes preceding line-indent and trailing newline of each
/// matching entry, same gobble pattern as
/// `remove_node_entry_by_name`.
pub fn remove_edge_entry_by_from_to_in_source(
  source: &str,
  from: &OwnedEdgeEndpoint,
  to: &OwnedEdgeEndpoint,
) -> Result<String, RemoveEdgeError> {
  let graph = parse_pnix_graph_mode(source).ok_or(RemoveEdgeError::NotGraphMode)?;
  if !graph.looks_like_graph_mode() {
    return Err(RemoveEdgeError::NotGraphMode);
  }
  let edges = graph
    .section("edges")
    .ok_or(RemoveEdgeError::NoEdgesSection)?;
  let entries = parse_list_entries(source, edges).ok_or(RemoveEdgeError::MalformedEdgesSection)?;

  let target_from = format!("from = {{ {}; }}", from.render_inner());
  let target_to = format!("to = {{ {}; }}", to.render_inner());

  let target_entries: Vec<(usize, usize)> = entries
    .iter()
    .copied()
    .filter(|(s, e)| {
      let body = &source[*s..*e];
      body.contains(&target_from) && body.contains(&target_to)
    })
    .collect();

  if target_entries.is_empty() {
    return Err(RemoveEdgeError::EdgeNotFound);
  }

  let bytes = source.as_bytes();
  let mut removals: Vec<(usize, usize)> = Vec::new();
  for (entry_start, entry_end) in target_entries {
    let line_start = bytes[..entry_start]
      .iter()
      .rposition(|&b| b == b'\n')
      .map(|n| n + 1)
      .unwrap_or(0);
    let preceding_ws_only = bytes[line_start..entry_start]
      .iter()
      .all(|&b| b == b' ' || b == b'\t');
    let trailing_nl_end =
      if preceding_ws_only && entry_end < bytes.len() && bytes[entry_end] == b'\n' {
        entry_end + 1
      } else {
        entry_end
      };
    let (kill_start, kill_end) = if preceding_ws_only {
      (line_start, trailing_nl_end)
    } else {
      (entry_start, entry_end)
    };
    removals.push((kill_start, kill_end));
  }
  removals.sort_by_key(|(s, _)| *s);

  let mut out = String::with_capacity(source.len());
  let mut cursor = 0usize;
  for (start, end) in removals {
    out.push_str(&source[cursor..start]);
    cursor = end;
  }
  out.push_str(&source[cursor..]);
  Ok(out)
}

/// Parse the top-level `{ ... }` entries inside a list-shape graph
/// section (`externs` / `nodes` / `edges` / `types`). Returns each
/// entry's `(start_byte, end_byte_exclusive)` pair. `start_byte` is
/// the opening `{`; `end_byte_exclusive` is one past the matching
/// `}`.
///
/// Returns `None` if the section isn't `ValueShape::List` or the
/// source is malformed (unbalanced braces inside the body).
///
/// OWNER-LAW (2026-05-13): single source of truth for list-entry
/// boundary detection — both `remove_node_entry_by_name` and future
/// `remove_extern_entry_by_name` / `remove_edge_entry_*` /
/// per-entry inspectors use this. Honors `pnix_skip_zones` so a
/// `{` inside a `#` comment or `"..."` string does NOT count as an
/// entry opener.
pub fn parse_list_entries(source: &str, section: &PnixGraphSection) -> Option<Vec<(usize, usize)>> {
  if !matches!(section.value_shape, ValueShape::List) {
    return None;
  }
  let bytes = source.as_bytes();
  let skip = pnix_skip_zones(source);
  let body_start = section.body_inner_start_byte;
  let body_end = section.body_inner_end_byte;
  let mut entries: Vec<(usize, usize)> = Vec::new();
  let mut i = body_start;
  while i < body_end {
    if in_skip_zone(&skip, i) {
      i = skip_zone_end(&skip, i).unwrap_or(i + 1);
      continue;
    }
    if bytes[i] == b'{' {
      let close = find_matching_close_brace(bytes, &skip, i)?;
      // Malformed if entry extends past the section body.
      if close >= body_end {
        return None;
      }
      entries.push((i, close + 1));
      i = close + 1;
      continue;
    }
    i += 1;
  }
  Some(entries)
}

/// Errors raised by [`remove_node_entry_by_name`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoveNodeIdError {
  /// `name` is empty.
  EmptyName,
  /// `name` is not a valid ASCII identifier.
  InvalidIdentifier,
  /// Source has no graph-mode shape.
  NotGraphMode,
  /// No `nodes = [ ... ]` section in source.
  NoNodesSection,
  /// No node entry with `name = "<name>"` declared.
  NodeNotFound,
  /// The malformed-source case `parse_list_entries` cannot parse.
  MalformedNodesSection,
  /// `edges` section still references the node `ref_count` times.
  /// Strict-mode refuses to silently break the graph; caller can
  /// (1) explicitly remove the referencing edges first, or
  /// (2) call a future cascade variant.
  StillReferencedByEdges { ref_count: usize },
}

/// Remove a node entry from the `nodes` section by name (strict —
/// refuses if any `edges` entry references the node).
///
/// Removed bytes: the matching `{ ... }` entry plus its preceding
/// line-indent and the trailing newline, so the resulting source
/// has no orphaned blank line. If the entry is on a line shared
/// with other content (rare), only the entry bytes themselves are
/// removed (conservative).
///
/// OWNER-LAW (2026-05-13): strict by design — silently breaking
/// edges is the kind of debt that compounds. Caller who wants
/// cascade semantics calls a future
/// `remove_node_entry_by_name_cascade` (D-6 slice).
pub fn remove_node_entry_by_name(source: &str, name: &str) -> Result<String, RemoveNodeIdError> {
  if name.is_empty() {
    return Err(RemoveNodeIdError::EmptyName);
  }
  if !is_valid_pnix_node_identifier(name) {
    return Err(RemoveNodeIdError::InvalidIdentifier);
  }
  let graph = parse_pnix_graph_mode(source).ok_or(RemoveNodeIdError::NotGraphMode)?;
  if !graph.looks_like_graph_mode() {
    return Err(RemoveNodeIdError::NotGraphMode);
  }
  let nodes = graph
    .section("nodes")
    .ok_or(RemoveNodeIdError::NoNodesSection)?;
  let entries =
    parse_list_entries(source, nodes).ok_or(RemoveNodeIdError::MalformedNodesSection)?;
  let target_decl = format!("name = \"{name}\"");
  let target = entries
    .iter()
    .find(|(s, e)| source[*s..*e].contains(&target_decl))
    .copied()
    .ok_or(RemoveNodeIdError::NodeNotFound)?;

  // Strict reference check: count `node = "<name>"` occurrences
  // anywhere in the edges body. Note: `input = "<name>"` is NOT a
  // reference to a node (graph-input namespace is separate).
  if let Some(edges) = graph.section("edges") {
    let edges_body = &source[edges.body_inner_start_byte..edges.body_inner_end_byte];
    let ref_pattern = format!("node = \"{name}\"");
    let ref_count = edges_body.matches(&ref_pattern).count();
    if ref_count > 0 {
      return Err(RemoveNodeIdError::StillReferencedByEdges { ref_count });
    }
  }

  let (entry_start, entry_end) = target;
  let bytes = source.as_bytes();
  // Detect "entry on its own line": preceding bytes back to the
  // most recent `\n` are all whitespace.
  let line_start = bytes[..entry_start]
    .iter()
    .rposition(|&b| b == b'\n')
    .map(|n| n + 1)
    .unwrap_or(0);
  let preceding_ws_only = bytes[line_start..entry_start]
    .iter()
    .all(|&b| b == b' ' || b == b'\t');
  // Gobble the trailing newline only if the entry was on its own
  // line.
  let trailing_nl_end = if preceding_ws_only && entry_end < bytes.len() && bytes[entry_end] == b'\n'
  {
    entry_end + 1
  } else {
    entry_end
  };
  let mut out = String::with_capacity(source.len());
  if preceding_ws_only {
    out.push_str(&source[..line_start]);
    out.push_str(&source[trailing_nl_end..]);
  } else {
    out.push_str(&source[..entry_start]);
    out.push_str(&source[entry_end..]);
  }
  Ok(out)
}

/// Result of a cascade remove — `(new_source, edges_removed_count)`.
pub struct RemoveNodeCascadeOutcome {
  pub new_source: String,
  pub edges_removed: usize,
}

/// Cascade variant of [`remove_node_entry_by_name`]. Removes the
/// node entry **and** every edge entry whose body contains
/// `node = "<name>"`. The number of edges removed is returned so
/// the caller can surface it to the operator.
///
/// OWNER-LAW (2026-05-13): cascade is **opt-in**. The strict
/// variant remains the default elsewhere — `remove-node-id` request
/// must carry `cascade: true` to invoke this. Edge removal is
/// substring-based (`node = "<name>"` inside the edge entry body)
/// — sufficient for the canonical corpus shape; future CST-aware
/// variant can refine.
///
/// Returns the same `RemoveNodeIdError` variants as the strict
/// helper **except** `StillReferencedByEdges` (cascade never raises
/// that — it just removes the referencing edges).
pub fn remove_node_entry_by_name_cascade(
  source: &str,
  name: &str,
) -> Result<RemoveNodeCascadeOutcome, RemoveNodeIdError> {
  if name.is_empty() {
    return Err(RemoveNodeIdError::EmptyName);
  }
  if !is_valid_pnix_node_identifier(name) {
    return Err(RemoveNodeIdError::InvalidIdentifier);
  }
  let graph = parse_pnix_graph_mode(source).ok_or(RemoveNodeIdError::NotGraphMode)?;
  if !graph.looks_like_graph_mode() {
    return Err(RemoveNodeIdError::NotGraphMode);
  }
  let nodes = graph
    .section("nodes")
    .ok_or(RemoveNodeIdError::NoNodesSection)?;
  let node_entries =
    parse_list_entries(source, nodes).ok_or(RemoveNodeIdError::MalformedNodesSection)?;
  let target_decl = format!("name = \"{name}\"");
  let target_node = node_entries
    .iter()
    .find(|(s, e)| source[*s..*e].contains(&target_decl))
    .copied()
    .ok_or(RemoveNodeIdError::NodeNotFound)?;

  // Collect edge entries that reference `<name>` (substring
  // `node = "<name>"`). For substrings inside list bodies — not
  // attribute names of attrsets — this naive contains check is
  // sufficient: `node = "X"` only appears as an edge endpoint.
  let ref_pattern = format!("node = \"{name}\"");
  let edge_entries_to_remove: Vec<(usize, usize)> = if let Some(edges) = graph.section("edges") {
    parse_list_entries(source, edges)
      .unwrap_or_default()
      .into_iter()
      .filter(|(s, e)| source[*s..*e].contains(&ref_pattern))
      .collect()
  } else {
    Vec::new()
  };
  let edges_removed = edge_entries_to_remove.len();

  // Gather all bytes to remove (node entry + edge entries) along
  // with their gobbled-line ranges. Apply in source order in a
  // single rebuild pass.
  let bytes = source.as_bytes();
  let mut removals: Vec<(usize, usize)> = Vec::new();
  for entry in std::iter::once(target_node).chain(edge_entries_to_remove.into_iter()) {
    let (entry_start, entry_end) = entry;
    let line_start = bytes[..entry_start]
      .iter()
      .rposition(|&b| b == b'\n')
      .map(|n| n + 1)
      .unwrap_or(0);
    let preceding_ws_only = bytes[line_start..entry_start]
      .iter()
      .all(|&b| b == b' ' || b == b'\t');
    let trailing_nl_end =
      if preceding_ws_only && entry_end < bytes.len() && bytes[entry_end] == b'\n' {
        entry_end + 1
      } else {
        entry_end
      };
    let (kill_start, kill_end) = if preceding_ws_only {
      (line_start, trailing_nl_end)
    } else {
      (entry_start, entry_end)
    };
    removals.push((kill_start, kill_end));
  }
  removals.sort_by_key(|(s, _)| *s);
  // Sanity: removals must not overlap (caller never gets a
  // single-byte entry that spans two list bodies). Defensive: drop
  // any overlap.
  let mut deduped: Vec<(usize, usize)> = Vec::new();
  for (s, e) in removals {
    if let Some(last) = deduped.last() {
      if s < last.1 {
        continue;
      }
    }
    deduped.push((s, e));
  }

  let mut out = String::with_capacity(source.len());
  let mut cursor = 0usize;
  for (s, e) in deduped {
    out.push_str(&source[cursor..s]);
    cursor = e;
  }
  out.push_str(&source[cursor..]);
  Ok(RemoveNodeCascadeOutcome {
    new_source: out,
    edges_removed,
  })
}

// ─── rename-node-id transform (D-8: dispatcher / artifact surface) ──

/// Request shape for the `rename-node-id` graph-mode transform.
/// Mirrors the shape of `rename_symbol::RenameRequest` but scoped to
/// a single graph-mode `.px` file (a graph lives in one file, not
/// across many).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RenameNodeIdRequest {
  pub target_path: String,
  pub old_name: String,
  pub new_name: String,
}

/// Held / Rejected kinds emitted by [`classify_rename_node_id`] or
/// [`compute_rename_node_id_patch_candidate`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RenameNodeIdHeldKind {
  /// `old_name` empty.
  EmptyOldName,
  /// `new_name` empty.
  EmptyNewName,
  /// `old_name == new_name` — structurally redundant.
  OldEqualsNew,
  /// Either name fails the ASCII identifier rule.
  InvalidIdentifier,
  /// `target_path` empty.
  EmptyTargetPath,
  /// `target_path` contains `..` or null bytes.
  TargetPathOutOfProject,
  /// File content is not a recognizable graph-mode `.px`.
  NotGraphMode,
  /// Graph file has no `nodes = [ ... ]` section.
  NoNodesSection,
  /// `nodes` section has no entry with `name = "<old_name>"`.
  NodeNotFound,
  /// `nodes` section already has `name = "<new_name>"` — refuse to
  /// create a duplicate.
  NewNameAlreadyDeclared,
}

impl RenameNodeIdHeldKind {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::EmptyOldName => "empty-old-name",
      Self::EmptyNewName => "empty-new-name",
      Self::OldEqualsNew => "old-equals-new",
      Self::InvalidIdentifier => "invalid-identifier",
      Self::EmptyTargetPath => "empty-target-path",
      Self::TargetPathOutOfProject => "target-path-out-of-project",
      Self::NotGraphMode => "not-graph-mode",
      Self::NoNodesSection => "no-nodes-section",
      Self::NodeNotFound => "node-not-found",
      Self::NewNameAlreadyDeclared => "new-name-already-declared",
    }
  }
}

/// Verdict emitted by [`classify_rename_node_id`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "verdict")]
pub enum RenameNodeIdVerdict {
  RenameNodeIdReady,
  RenameNodeIdHeld {
    held_kind: RenameNodeIdHeldKind,
    reason: String,
  },
  RenameNodeIdRejected {
    held_kind: RenameNodeIdHeldKind,
    reason: String,
  },
}

/// Request-shape classifier — file-content-independent. Catches the
/// problems that don't need the file to be loaded.
pub fn classify_rename_node_id(req: &RenameNodeIdRequest) -> RenameNodeIdVerdict {
  if req.old_name.is_empty() {
    return RenameNodeIdVerdict::RenameNodeIdHeld {
      held_kind: RenameNodeIdHeldKind::EmptyOldName,
      reason: "rename-node-id requires a non-empty `old_name`".to_string(),
    };
  }
  if req.new_name.is_empty() {
    return RenameNodeIdVerdict::RenameNodeIdHeld {
      held_kind: RenameNodeIdHeldKind::EmptyNewName,
      reason: "rename-node-id requires a non-empty `new_name`".to_string(),
    };
  }
  if req.old_name == req.new_name {
    return RenameNodeIdVerdict::RenameNodeIdRejected {
      held_kind: RenameNodeIdHeldKind::OldEqualsNew,
      reason: "old_name == new_name — nothing to do".to_string(),
    };
  }
  if !is_valid_pnix_node_identifier(&req.old_name) || !is_valid_pnix_node_identifier(&req.new_name)
  {
    return RenameNodeIdVerdict::RenameNodeIdHeld {
      held_kind: RenameNodeIdHeldKind::InvalidIdentifier,
      reason: "old_name or new_name is not a valid ASCII identifier".to_string(),
    };
  }
  if req.target_path.is_empty() {
    return RenameNodeIdVerdict::RenameNodeIdHeld {
      held_kind: RenameNodeIdHeldKind::EmptyTargetPath,
      reason: "rename-node-id requires a non-empty `target_path`".to_string(),
    };
  }
  if req.target_path.contains("..") || req.target_path.contains('\u{0}') {
    return RenameNodeIdVerdict::RenameNodeIdHeld {
      held_kind: RenameNodeIdHeldKind::TargetPathOutOfProject,
      reason: "target_path must be within the project root and must not contain `..` or null bytes"
        .to_string(),
    };
  }
  RenameNodeIdVerdict::RenameNodeIdReady
}

/// Per-file patch emitted by `compute_rename_node_id_patch_candidate`.
/// Same shape as `add_test_stub::AddTestStubFilePatch` so downstream
/// materialization treats every graph-mode edit uniformly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenameNodeIdFilePatch {
  pub path: String,
  pub before_content: String,
  pub after_content: String,
  pub before_sha256: String,
  pub after_sha256: String,
}

/// Per-file input — borrowed `(path, content)` pair.
#[derive(Debug, Clone, Copy)]
pub struct RenameNodeIdFileInput<'a> {
  pub path: &'a str,
  pub content: &'a str,
}

/// Sealed candidate emitted by
/// `compute_rename_node_id_patch_candidate`. Always carries the
/// request + verdict; `file_patches` and `unified_diff` are
/// populated only when the verdict is Ready.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenameNodeIdPatchCandidate {
  pub request: RenameNodeIdRequest,
  pub verdict: RenameNodeIdVerdict,
  pub file_patches: Vec<RenameNodeIdFilePatch>,
  pub unified_diff: String,
}

fn sha256_hex_bytes(bytes: &[u8]) -> String {
  let mut h = Sha256::new();
  h.update(bytes);
  format!("{:x}", h.finalize())
}

/// Map a low-level `RenameNodeIdError` (from `rename_node_id_in_source`)
/// to the file-content-tier Held kinds.
fn rename_node_id_error_to_verdict(err: RenameNodeIdError) -> RenameNodeIdVerdict {
  match err {
    RenameNodeIdError::EmptyName => RenameNodeIdVerdict::RenameNodeIdHeld {
      held_kind: RenameNodeIdHeldKind::EmptyOldName,
      reason: "old_name or new_name is empty".to_string(),
    },
    RenameNodeIdError::OldEqualsNew => RenameNodeIdVerdict::RenameNodeIdRejected {
      held_kind: RenameNodeIdHeldKind::OldEqualsNew,
      reason: "old_name == new_name".to_string(),
    },
    RenameNodeIdError::InvalidIdentifier => RenameNodeIdVerdict::RenameNodeIdHeld {
      held_kind: RenameNodeIdHeldKind::InvalidIdentifier,
      reason: "old_name or new_name is not a valid ASCII identifier".to_string(),
    },
    RenameNodeIdError::NotGraphMode => RenameNodeIdVerdict::RenameNodeIdHeld {
      held_kind: RenameNodeIdHeldKind::NotGraphMode,
      reason: "target file is not a recognizable graph-mode `.px`".to_string(),
    },
    RenameNodeIdError::NoNodesSection => RenameNodeIdVerdict::RenameNodeIdHeld {
      held_kind: RenameNodeIdHeldKind::NoNodesSection,
      reason: "graph file has no `nodes = [ ... ]` section".to_string(),
    },
    RenameNodeIdError::NodeNotFound => RenameNodeIdVerdict::RenameNodeIdHeld {
      held_kind: RenameNodeIdHeldKind::NodeNotFound,
      reason: "no node entry with `name = \"<old_name>\"` was found".to_string(),
    },
    RenameNodeIdError::NewNameAlreadyDeclared => RenameNodeIdVerdict::RenameNodeIdHeld {
      held_kind: RenameNodeIdHeldKind::NewNameAlreadyDeclared,
      reason: "`name = \"<new_name>\"` already declared in nodes section".to_string(),
    },
  }
}

/// Render a unified diff for the rename-node-id patch — same minimal
/// canonical format as `add_test_stub::render_unified_diff_*`.
fn render_unified_diff_rename_node_id(patches: &[RenameNodeIdFilePatch]) -> String {
  let mut out = String::new();
  for p in patches {
    out.push_str(&format!("--- a/{}\n", p.path));
    out.push_str(&format!("+++ b/{}\n", p.path));
    let before_lines: Vec<&str> = p.before_content.split_inclusive('\n').collect();
    let after_lines: Vec<&str> = p.after_content.split_inclusive('\n').collect();
    for line in &before_lines {
      out.push('-');
      out.push_str(line);
      if !line.ends_with('\n') {
        out.push('\n');
      }
    }
    for line in &after_lines {
      out.push('+');
      out.push_str(line);
      if !line.ends_with('\n') {
        out.push('\n');
      }
    }
  }
  out
}

/// Compute a `RenameNodeIdPatchCandidate`. Runs the classifier
/// first, then (if Ready) invokes `rename_node_id_in_source` on the
/// file content. Failures from either layer downgrade to the
/// appropriate Held verdict; `file_patches` is empty on Held.
///
/// OWNER-LAW (2026-05-13): the file-content-aware emit boundary for
/// graph-mode rename. Mirrors the shape of
/// `add_test_stub::compute_add_test_stub_patch_candidate_rust` so the
/// dispatcher can treat all transforms uniformly.
pub fn compute_rename_node_id_patch_candidate(
  request: &RenameNodeIdRequest,
  file_input: &RenameNodeIdFileInput<'_>,
) -> RenameNodeIdPatchCandidate {
  let verdict = classify_rename_node_id(request);
  if !matches!(verdict, RenameNodeIdVerdict::RenameNodeIdReady) {
    return RenameNodeIdPatchCandidate {
      request: request.clone(),
      verdict,
      file_patches: Vec::new(),
      unified_diff: String::new(),
    };
  }
  // Path mismatch — file_input doesn't cover request.target_path.
  // Treat as no-op (verdict stays Ready but no patches emitted).
  if file_input.path != request.target_path {
    return RenameNodeIdPatchCandidate {
      request: request.clone(),
      verdict,
      file_patches: Vec::new(),
      unified_diff: String::new(),
    };
  }
  let before = file_input.content;
  match rename_node_id_in_source(before, &request.old_name, &request.new_name) {
    Ok(after) => {
      if after == before {
        // Defensive: shouldn't happen since classifier-Ready means
        // the rename did real work, but keep the safety net.
        return RenameNodeIdPatchCandidate {
          request: request.clone(),
          verdict,
          file_patches: Vec::new(),
          unified_diff: String::new(),
        };
      }
      let patch = RenameNodeIdFilePatch {
        path: file_input.path.to_string(),
        before_content: before.to_string(),
        after_content: after,
        before_sha256: sha256_hex_bytes(before.as_bytes()),
        after_sha256: String::new(),
      };
      // Compute after sha now that we have the after_content.
      let mut patch = patch;
      patch.after_sha256 = sha256_hex_bytes(patch.after_content.as_bytes());
      let unified_diff = render_unified_diff_rename_node_id(std::slice::from_ref(&patch));
      RenameNodeIdPatchCandidate {
        request: request.clone(),
        verdict,
        file_patches: vec![patch],
        unified_diff,
      }
    }
    Err(err) => RenameNodeIdPatchCandidate {
      request: request.clone(),
      verdict: rename_node_id_error_to_verdict(err),
      file_patches: Vec::new(),
      unified_diff: String::new(),
    },
  }
}

/// Render an `RenameNodeIdPatchCandidate` as the canonical JSON
/// payload of a `coding.generated-patch-candidate` artifact for
/// the rename-node-id transform.
pub fn build_rename_node_id_patch_candidate_payload(
  candidate: &RenameNodeIdPatchCandidate,
) -> serde_json::Value {
  let req = &candidate.request;
  let verdict_str = match &candidate.verdict {
    RenameNodeIdVerdict::RenameNodeIdReady => "rename-node-id-ready",
    RenameNodeIdVerdict::RenameNodeIdHeld { .. } => "rename-node-id-held",
    RenameNodeIdVerdict::RenameNodeIdRejected { .. } => "rename-node-id-rejected",
  };
  let file_patches_arr: Vec<serde_json::Value> = candidate
    .file_patches
    .iter()
    .map(|fp| {
      serde_json::json!({
        "path": fp.path,
        "before_sha256": fp.before_sha256,
        "after_sha256": fp.after_sha256,
        "before_byte_len": fp.before_content.len(),
        "after_byte_len": fp.after_content.len(),
      })
    })
    .collect();
  let mut payload = serde_json::json!({
    "transform": "rename-node-id",
    "owner_law": "crates/pnix-core::code_transform::pnix_graph",
    "target_path": req.target_path,
    "old_name": req.old_name,
    "new_name": req.new_name,
    "language": "pnix",
    "scope": "single-graph-file",
    "verdict": verdict_str,
    "capability_required": "EditWithinTargetPaths",
    "file_patches": file_patches_arr,
    "unified_diff": candidate.unified_diff,
    "candidate_only": true,
    "next_step": match candidate.verdict {
      RenameNodeIdVerdict::RenameNodeIdReady => "tool-action-approval-then-materialize",
      _ => "operator-decision-or-resubmit",
    },
  });
  match &candidate.verdict {
    RenameNodeIdVerdict::RenameNodeIdReady => {}
    RenameNodeIdVerdict::RenameNodeIdHeld { held_kind, reason }
    | RenameNodeIdVerdict::RenameNodeIdRejected { held_kind, reason } => {
      payload["held_kind"] = serde_json::Value::String(held_kind.as_str().to_string());
      payload["reason"] = serde_json::Value::String(reason.clone());
    }
  }
  payload
}

/// Wrap a `RenameNodeIdPatchCandidate` as a
/// `coding.generated-patch-candidate` artifact. Replay-stable
/// SHA-256 id mixes request identity + per-file sha pair.
pub fn build_rename_node_id_patch_candidate_artifact(
  candidate: &RenameNodeIdPatchCandidate,
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
) -> serde_json::Value {
  let payload = build_rename_node_id_patch_candidate_payload(candidate);
  let req = &candidate.request;

  let mut hasher = Sha256::new();
  hasher.update(b"rename-node-id-patch\x1f");
  hasher.update(req.target_path.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(req.old_name.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(req.new_name.as_bytes());
  hasher.update(b"\x1f");
  for fp in &candidate.file_patches {
    hasher.update(fp.path.as_bytes());
    hasher.update(b"\x1e");
    hasher.update(fp.before_sha256.as_bytes());
    hasher.update(b"\x1e");
    hasher.update(fp.after_sha256.as_bytes());
    hasher.update(b"\x1d");
  }
  let digest = hasher.finalize();
  let prefix = digest
    .iter()
    .take(8)
    .map(|b| format!("{b:02x}"))
    .collect::<String>();
  let id = format!("generated-patch.rename-node-id.{prefix}");

  let mut artifact = serde_json::json!({
    "id": id,
    "artifact_family": "coding.generated-patch-candidate",
    "source_surface": "code-transform.rename-node-id",
    "stored_at_ms": stored_at_ms,
    "target_paths": [req.target_path.clone()],
    "command_refs": serde_json::Value::Array(Vec::new()),
    "related_refs": serde_json::json!([
      "owner-law:crates/pnix-core::code_transform::pnix_graph"
    ]),
    "payload": payload,
  });
  if let Some(snap) = repo_snapshot_ref {
    artifact["repo_snapshot_ref"] = serde_json::Value::String(snap.to_string());
  }
  artifact
}

// ─── add-* graph-mode transforms (D-13: dispatcher integration) ──
//
// Three transforms share the same shape — `{ target_path, entry_text }`
// request, single-file patch, list-section append. The trio
// completes the graph-mode CRUD's Create side at the dispatcher
// layer. Typed builders (`EdgeEndpoint`, future `NodeSpec`) are
// caller-side ergonomics — the dispatcher only sees raw entry_text.

/// Held / Rejected kinds shared by `add-pnix-extern` /
/// `add-pnix-node` / `add-pnix-edge`. Kebab-case for serialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AddPnixGraphHeldKind {
  EmptyEntryText,
  EmptyTargetPath,
  TargetPathOutOfProject,
  NotGraphMode,
  NoSectionInGraph,
}

impl AddPnixGraphHeldKind {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::EmptyEntryText => "empty-entry-text",
      Self::EmptyTargetPath => "empty-target-path",
      Self::TargetPathOutOfProject => "target-path-out-of-project",
      Self::NotGraphMode => "not-graph-mode",
      Self::NoSectionInGraph => "no-section-in-graph",
    }
  }
}

/// Per-file patch — shared across all three add-* transforms (the
/// shape is identical: single file, before/after sha pair).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddPnixGraphFilePatch {
  pub path: String,
  pub before_content: String,
  pub after_content: String,
  pub before_sha256: String,
  pub after_sha256: String,
}

#[derive(Debug, Clone, Copy)]
pub struct AddPnixGraphFileInput<'a> {
  pub path: &'a str,
  pub content: &'a str,
}

fn render_unified_diff_add_pnix_graph(patches: &[AddPnixGraphFilePatch]) -> String {
  let mut out = String::new();
  for p in patches {
    out.push_str(&format!("--- a/{}\n", p.path));
    out.push_str(&format!("+++ b/{}\n", p.path));
    let before_lines: Vec<&str> = p.before_content.split_inclusive('\n').collect();
    let after_lines: Vec<&str> = p.after_content.split_inclusive('\n').collect();
    for line in &before_lines {
      out.push('-');
      out.push_str(line);
      if !line.ends_with('\n') {
        out.push('\n');
      }
    }
    for line in &after_lines {
      out.push('+');
      out.push_str(line);
      if !line.ends_with('\n') {
        out.push('\n');
      }
    }
  }
  out
}

/// Shared classifier — request-shape validation common to all three
/// add-* transforms. File-content-tier checks come later in the
/// compute step.
fn classify_add_pnix_graph_request(
  target_path: &str,
  entry_text: &str,
) -> Option<(AddPnixGraphHeldKind, String)> {
  if entry_text.is_empty() {
    return Some((
      AddPnixGraphHeldKind::EmptyEntryText,
      "entry_text must be non-empty".to_string(),
    ));
  }
  if target_path.is_empty() {
    return Some((
      AddPnixGraphHeldKind::EmptyTargetPath,
      "target_path must be non-empty".to_string(),
    ));
  }
  if target_path.contains("..") || target_path.contains('\u{0}') {
    return Some((
      AddPnixGraphHeldKind::TargetPathOutOfProject,
      "target_path must be within project root and must not contain `..` or null bytes".to_string(),
    ));
  }
  None
}

/// Shared file-content check: confirm source is graph-mode and has
/// the requested section. Returns `Ok(graph)` on success.
fn validate_graph_section(
  source: &str,
  section_name: &str,
) -> Result<PnixGraphMode, (AddPnixGraphHeldKind, String)> {
  let graph = parse_pnix_graph_mode(source).ok_or((
    AddPnixGraphHeldKind::NotGraphMode,
    "target file is not a recognizable graph-mode `.px`".to_string(),
  ))?;
  if !graph.looks_like_graph_mode() {
    return Err((
      AddPnixGraphHeldKind::NotGraphMode,
      "target file is not a recognizable graph-mode `.px`".to_string(),
    ));
  }
  if graph.section(section_name).is_none() {
    return Err((
      AddPnixGraphHeldKind::NoSectionInGraph,
      format!("graph file has no `{section_name} = [ ... ]` section"),
    ));
  }
  Ok(graph)
}

// ─── add-pnix-extern ─────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AddPnixExternRequest {
  pub target_path: String,
  pub entry_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "verdict")]
pub enum AddPnixExternVerdict {
  AddPnixExternReady,
  AddPnixExternHeld {
    held_kind: AddPnixGraphHeldKind,
    reason: String,
  },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddPnixExternPatchCandidate {
  pub request: AddPnixExternRequest,
  pub verdict: AddPnixExternVerdict,
  pub file_patches: Vec<AddPnixGraphFilePatch>,
  pub unified_diff: String,
}

pub fn compute_add_pnix_extern_patch_candidate(
  request: &AddPnixExternRequest,
  file_input: &AddPnixGraphFileInput<'_>,
) -> AddPnixExternPatchCandidate {
  if let Some((kind, reason)) =
    classify_add_pnix_graph_request(&request.target_path, &request.entry_text)
  {
    return AddPnixExternPatchCandidate {
      request: request.clone(),
      verdict: AddPnixExternVerdict::AddPnixExternHeld {
        held_kind: kind,
        reason,
      },
      file_patches: Vec::new(),
      unified_diff: String::new(),
    };
  }
  if file_input.path != request.target_path {
    return AddPnixExternPatchCandidate {
      request: request.clone(),
      verdict: AddPnixExternVerdict::AddPnixExternReady,
      file_patches: Vec::new(),
      unified_diff: String::new(),
    };
  }
  if let Err((kind, reason)) = validate_graph_section(file_input.content, "externs") {
    return AddPnixExternPatchCandidate {
      request: request.clone(),
      verdict: AddPnixExternVerdict::AddPnixExternHeld {
        held_kind: kind,
        reason,
      },
      file_patches: Vec::new(),
      unified_diff: String::new(),
    };
  }
  let before = file_input.content;
  let after =
    add_extern_entry_to_source(before, &request.entry_text).expect("section validated above");
  let mut patch = AddPnixGraphFilePatch {
    path: file_input.path.to_string(),
    before_content: before.to_string(),
    after_content: after,
    before_sha256: sha256_hex_bytes(before.as_bytes()),
    after_sha256: String::new(),
  };
  patch.after_sha256 = sha256_hex_bytes(patch.after_content.as_bytes());
  let unified_diff = render_unified_diff_add_pnix_graph(std::slice::from_ref(&patch));
  AddPnixExternPatchCandidate {
    request: request.clone(),
    verdict: AddPnixExternVerdict::AddPnixExternReady,
    file_patches: vec![patch],
    unified_diff,
  }
}

pub fn build_add_pnix_extern_patch_candidate_artifact(
  candidate: &AddPnixExternPatchCandidate,
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
) -> serde_json::Value {
  build_add_pnix_graph_artifact(
    "add-pnix-extern",
    &candidate.request.target_path,
    &candidate.request.entry_text,
    match &candidate.verdict {
      AddPnixExternVerdict::AddPnixExternReady => None,
      AddPnixExternVerdict::AddPnixExternHeld { held_kind, reason } => {
        Some((*held_kind, reason.clone()))
      }
    },
    &candidate.file_patches,
    &candidate.unified_diff,
    stored_at_ms,
    repo_snapshot_ref,
  )
}

// ─── add-pnix-node ───────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AddPnixNodeRequest {
  pub target_path: String,
  pub entry_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "verdict")]
pub enum AddPnixNodeVerdict {
  AddPnixNodeReady,
  AddPnixNodeHeld {
    held_kind: AddPnixGraphHeldKind,
    reason: String,
  },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddPnixNodePatchCandidate {
  pub request: AddPnixNodeRequest,
  pub verdict: AddPnixNodeVerdict,
  pub file_patches: Vec<AddPnixGraphFilePatch>,
  pub unified_diff: String,
}

pub fn compute_add_pnix_node_patch_candidate(
  request: &AddPnixNodeRequest,
  file_input: &AddPnixGraphFileInput<'_>,
) -> AddPnixNodePatchCandidate {
  if let Some((kind, reason)) =
    classify_add_pnix_graph_request(&request.target_path, &request.entry_text)
  {
    return AddPnixNodePatchCandidate {
      request: request.clone(),
      verdict: AddPnixNodeVerdict::AddPnixNodeHeld {
        held_kind: kind,
        reason,
      },
      file_patches: Vec::new(),
      unified_diff: String::new(),
    };
  }
  if file_input.path != request.target_path {
    return AddPnixNodePatchCandidate {
      request: request.clone(),
      verdict: AddPnixNodeVerdict::AddPnixNodeReady,
      file_patches: Vec::new(),
      unified_diff: String::new(),
    };
  }
  if let Err((kind, reason)) = validate_graph_section(file_input.content, "nodes") {
    return AddPnixNodePatchCandidate {
      request: request.clone(),
      verdict: AddPnixNodeVerdict::AddPnixNodeHeld {
        held_kind: kind,
        reason,
      },
      file_patches: Vec::new(),
      unified_diff: String::new(),
    };
  }
  let before = file_input.content;
  let after =
    add_node_entry_to_source(before, &request.entry_text).expect("section validated above");
  let mut patch = AddPnixGraphFilePatch {
    path: file_input.path.to_string(),
    before_content: before.to_string(),
    after_content: after,
    before_sha256: sha256_hex_bytes(before.as_bytes()),
    after_sha256: String::new(),
  };
  patch.after_sha256 = sha256_hex_bytes(patch.after_content.as_bytes());
  let unified_diff = render_unified_diff_add_pnix_graph(std::slice::from_ref(&patch));
  AddPnixNodePatchCandidate {
    request: request.clone(),
    verdict: AddPnixNodeVerdict::AddPnixNodeReady,
    file_patches: vec![patch],
    unified_diff,
  }
}

pub fn build_add_pnix_node_patch_candidate_artifact(
  candidate: &AddPnixNodePatchCandidate,
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
) -> serde_json::Value {
  build_add_pnix_graph_artifact(
    "add-pnix-node",
    &candidate.request.target_path,
    &candidate.request.entry_text,
    match &candidate.verdict {
      AddPnixNodeVerdict::AddPnixNodeReady => None,
      AddPnixNodeVerdict::AddPnixNodeHeld { held_kind, reason } => {
        Some((*held_kind, reason.clone()))
      }
    },
    &candidate.file_patches,
    &candidate.unified_diff,
    stored_at_ms,
    repo_snapshot_ref,
  )
}

// ─── add-pnix-edge ───────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AddPnixEdgeRequest {
  pub target_path: String,
  pub entry_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "verdict")]
pub enum AddPnixEdgeVerdict {
  AddPnixEdgeReady,
  AddPnixEdgeHeld {
    held_kind: AddPnixGraphHeldKind,
    reason: String,
  },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddPnixEdgePatchCandidate {
  pub request: AddPnixEdgeRequest,
  pub verdict: AddPnixEdgeVerdict,
  pub file_patches: Vec<AddPnixGraphFilePatch>,
  pub unified_diff: String,
}

pub fn compute_add_pnix_edge_patch_candidate(
  request: &AddPnixEdgeRequest,
  file_input: &AddPnixGraphFileInput<'_>,
) -> AddPnixEdgePatchCandidate {
  if let Some((kind, reason)) =
    classify_add_pnix_graph_request(&request.target_path, &request.entry_text)
  {
    return AddPnixEdgePatchCandidate {
      request: request.clone(),
      verdict: AddPnixEdgeVerdict::AddPnixEdgeHeld {
        held_kind: kind,
        reason,
      },
      file_patches: Vec::new(),
      unified_diff: String::new(),
    };
  }
  if file_input.path != request.target_path {
    return AddPnixEdgePatchCandidate {
      request: request.clone(),
      verdict: AddPnixEdgeVerdict::AddPnixEdgeReady,
      file_patches: Vec::new(),
      unified_diff: String::new(),
    };
  }
  if let Err((kind, reason)) = validate_graph_section(file_input.content, "edges") {
    return AddPnixEdgePatchCandidate {
      request: request.clone(),
      verdict: AddPnixEdgeVerdict::AddPnixEdgeHeld {
        held_kind: kind,
        reason,
      },
      file_patches: Vec::new(),
      unified_diff: String::new(),
    };
  }
  let before = file_input.content;
  let after =
    add_edge_entry_to_source(before, &request.entry_text).expect("section validated above");
  let mut patch = AddPnixGraphFilePatch {
    path: file_input.path.to_string(),
    before_content: before.to_string(),
    after_content: after,
    before_sha256: sha256_hex_bytes(before.as_bytes()),
    after_sha256: String::new(),
  };
  patch.after_sha256 = sha256_hex_bytes(patch.after_content.as_bytes());
  let unified_diff = render_unified_diff_add_pnix_graph(std::slice::from_ref(&patch));
  AddPnixEdgePatchCandidate {
    request: request.clone(),
    verdict: AddPnixEdgeVerdict::AddPnixEdgeReady,
    file_patches: vec![patch],
    unified_diff,
  }
}

pub fn build_add_pnix_edge_patch_candidate_artifact(
  candidate: &AddPnixEdgePatchCandidate,
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
) -> serde_json::Value {
  build_add_pnix_graph_artifact(
    "add-pnix-edge",
    &candidate.request.target_path,
    &candidate.request.entry_text,
    match &candidate.verdict {
      AddPnixEdgeVerdict::AddPnixEdgeReady => None,
      AddPnixEdgeVerdict::AddPnixEdgeHeld { held_kind, reason } => {
        Some((*held_kind, reason.clone()))
      }
    },
    &candidate.file_patches,
    &candidate.unified_diff,
    stored_at_ms,
    repo_snapshot_ref,
  )
}

/// Generic artifact builder shared by add-pnix-extern / -node /
/// -edge. Single source of truth for the canonical JSON shape +
/// id hashing.
fn build_add_pnix_graph_artifact(
  transform: &str,
  target_path: &str,
  entry_text: &str,
  held: Option<(AddPnixGraphHeldKind, String)>,
  file_patches: &[AddPnixGraphFilePatch],
  unified_diff: &str,
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
) -> serde_json::Value {
  let verdict_str = if held.is_some() {
    format!("{transform}-held")
  } else {
    format!("{transform}-ready")
  };
  let file_patches_arr: Vec<serde_json::Value> = file_patches
    .iter()
    .map(|fp| {
      serde_json::json!({
        "path": fp.path,
        "before_sha256": fp.before_sha256,
        "after_sha256": fp.after_sha256,
        "before_byte_len": fp.before_content.len(),
        "after_byte_len": fp.after_content.len(),
      })
    })
    .collect();
  let mut payload = serde_json::json!({
    "transform": transform,
    "owner_law": "crates/pnix-core::code_transform::pnix_graph",
    "target_path": target_path,
    "entry_text": entry_text,
    "language": "pnix",
    "scope": "single-graph-file",
    "verdict": verdict_str,
    "capability_required": "EditWithinTargetPaths",
    "file_patches": file_patches_arr,
    "unified_diff": unified_diff,
    "candidate_only": true,
    "next_step": if held.is_some() {
      "operator-decision-or-resubmit"
    } else {
      "tool-action-approval-then-materialize"
    },
  });
  if let Some((kind, reason)) = held {
    payload["held_kind"] = serde_json::Value::String(kind.as_str().to_string());
    payload["reason"] = serde_json::Value::String(reason);
  }

  let mut hasher = Sha256::new();
  hasher.update(transform.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(target_path.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(entry_text.as_bytes());
  hasher.update(b"\x1f");
  for fp in file_patches {
    hasher.update(fp.path.as_bytes());
    hasher.update(b"\x1e");
    hasher.update(fp.before_sha256.as_bytes());
    hasher.update(b"\x1e");
    hasher.update(fp.after_sha256.as_bytes());
    hasher.update(b"\x1d");
  }
  let digest = hasher.finalize();
  let prefix = digest
    .iter()
    .take(8)
    .map(|b| format!("{b:02x}"))
    .collect::<String>();
  let id = format!("generated-patch.{transform}.{prefix}");

  let mut artifact = serde_json::json!({
    "id": id,
    "artifact_family": "coding.generated-patch-candidate",
    "source_surface": format!("code-transform.{transform}"),
    "stored_at_ms": stored_at_ms,
    "target_paths": [target_path.to_string()],
    "command_refs": serde_json::Value::Array(Vec::new()),
    "related_refs": serde_json::json!([
      "owner-law:crates/pnix-core::code_transform::pnix_graph"
    ]),
    "payload": payload,
  });
  if let Some(snap) = repo_snapshot_ref {
    artifact["repo_snapshot_ref"] = serde_json::Value::String(snap.to_string());
  }
  artifact
}

// ─── remove-pnix-edge transform (D-18: edge deletion) ───────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RemovePnixEdgeRequest {
  pub target_path: String,
  pub from: OwnedEdgeEndpoint,
  pub to: OwnedEdgeEndpoint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RemovePnixEdgeHeldKind {
  EmptyTargetPath,
  TargetPathOutOfProject,
  NotGraphMode,
  NoEdgesSection,
  MalformedEdgesSection,
  EdgeNotFound,
}

impl RemovePnixEdgeHeldKind {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::EmptyTargetPath => "empty-target-path",
      Self::TargetPathOutOfProject => "target-path-out-of-project",
      Self::NotGraphMode => "not-graph-mode",
      Self::NoEdgesSection => "no-edges-section",
      Self::MalformedEdgesSection => "malformed-edges-section",
      Self::EdgeNotFound => "edge-not-found",
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "verdict")]
pub enum RemovePnixEdgeVerdict {
  RemovePnixEdgeReady,
  RemovePnixEdgeHeld {
    held_kind: RemovePnixEdgeHeldKind,
    reason: String,
  },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemovePnixEdgePatchCandidate {
  pub request: RemovePnixEdgeRequest,
  pub verdict: RemovePnixEdgeVerdict,
  pub file_patches: Vec<AddPnixGraphFilePatch>,
  pub unified_diff: String,
  /// Count of edge entries removed. Multiple edges can match the
  /// same (from, to) pair in a corpus where graph dataflow allows
  /// redundancy; this surfaces the count.
  pub edges_removed: Option<usize>,
}

pub fn compute_remove_pnix_edge_patch_candidate(
  request: &RemovePnixEdgeRequest,
  file_input: &AddPnixGraphFileInput<'_>,
) -> RemovePnixEdgePatchCandidate {
  // Request-shape classifier (file-independent).
  if request.target_path.is_empty() {
    return RemovePnixEdgePatchCandidate {
      request: request.clone(),
      verdict: RemovePnixEdgeVerdict::RemovePnixEdgeHeld {
        held_kind: RemovePnixEdgeHeldKind::EmptyTargetPath,
        reason: "remove-pnix-edge requires a non-empty `target_path`".to_string(),
      },
      file_patches: Vec::new(),
      unified_diff: String::new(),
      edges_removed: None,
    };
  }
  if request.target_path.contains("..") || request.target_path.contains('\u{0}') {
    return RemovePnixEdgePatchCandidate {
      request: request.clone(),
      verdict: RemovePnixEdgeVerdict::RemovePnixEdgeHeld {
        held_kind: RemovePnixEdgeHeldKind::TargetPathOutOfProject,
        reason: "target_path must be within project root and must not contain `..` or null bytes"
          .to_string(),
      },
      file_patches: Vec::new(),
      unified_diff: String::new(),
      edges_removed: None,
    };
  }
  if file_input.path != request.target_path {
    return RemovePnixEdgePatchCandidate {
      request: request.clone(),
      verdict: RemovePnixEdgeVerdict::RemovePnixEdgeReady,
      file_patches: Vec::new(),
      unified_diff: String::new(),
      edges_removed: None,
    };
  }

  let before = file_input.content;
  match remove_edge_entry_by_from_to_in_source(before, &request.from, &request.to) {
    Ok(after) => {
      // Count edges removed = difference in entry count. Parse
      // both before/after edges sections.
      let edges_removed =
        count_edges_in_source(before).saturating_sub(count_edges_in_source(&after));
      let mut patch = AddPnixGraphFilePatch {
        path: file_input.path.to_string(),
        before_content: before.to_string(),
        after_content: after,
        before_sha256: sha256_hex_bytes(before.as_bytes()),
        after_sha256: String::new(),
      };
      patch.after_sha256 = sha256_hex_bytes(patch.after_content.as_bytes());
      let unified_diff = render_unified_diff_add_pnix_graph(std::slice::from_ref(&patch));
      RemovePnixEdgePatchCandidate {
        request: request.clone(),
        verdict: RemovePnixEdgeVerdict::RemovePnixEdgeReady,
        file_patches: vec![patch],
        unified_diff,
        edges_removed: Some(edges_removed),
      }
    }
    Err(err) => {
      let (held_kind, reason) = match err {
        RemoveEdgeError::NotGraphMode => (
          RemovePnixEdgeHeldKind::NotGraphMode,
          "target file is not a recognizable graph-mode `.px`".to_string(),
        ),
        RemoveEdgeError::NoEdgesSection => (
          RemovePnixEdgeHeldKind::NoEdgesSection,
          "graph file has no `edges = [ ... ]` section".to_string(),
        ),
        RemoveEdgeError::MalformedEdgesSection => (
          RemovePnixEdgeHeldKind::MalformedEdgesSection,
          "edges section has unbalanced braces".to_string(),
        ),
        RemoveEdgeError::EdgeNotFound => (
          RemovePnixEdgeHeldKind::EdgeNotFound,
          "no edge entry matches the requested (from, to) endpoints".to_string(),
        ),
      };
      RemovePnixEdgePatchCandidate {
        request: request.clone(),
        verdict: RemovePnixEdgeVerdict::RemovePnixEdgeHeld { held_kind, reason },
        file_patches: Vec::new(),
        unified_diff: String::new(),
        edges_removed: None,
      }
    }
  }
}

fn count_edges_in_source(source: &str) -> usize {
  parse_pnix_graph_mode(source)
    .and_then(|g| {
      g.section("edges")
        .and_then(|s| parse_list_entries(source, s).map(|v| v.len()))
    })
    .unwrap_or(0)
}

pub fn build_remove_pnix_edge_patch_candidate_payload(
  candidate: &RemovePnixEdgePatchCandidate,
) -> serde_json::Value {
  let req = &candidate.request;
  let verdict_str = match &candidate.verdict {
    RemovePnixEdgeVerdict::RemovePnixEdgeReady => "remove-pnix-edge-ready",
    RemovePnixEdgeVerdict::RemovePnixEdgeHeld { .. } => "remove-pnix-edge-held",
  };
  let file_patches_arr: Vec<serde_json::Value> = candidate
    .file_patches
    .iter()
    .map(|fp| {
      serde_json::json!({
        "path": fp.path,
        "before_sha256": fp.before_sha256,
        "after_sha256": fp.after_sha256,
        "before_byte_len": fp.before_content.len(),
        "after_byte_len": fp.after_content.len(),
      })
    })
    .collect();
  let mut payload = serde_json::json!({
    "transform": "remove-pnix-edge",
    "owner_law": "crates/pnix-core::code_transform::pnix_graph",
    "target_path": req.target_path,
    "from": req.from,
    "to": req.to,
    "language": "pnix",
    "scope": "single-graph-file",
    "verdict": verdict_str,
    "capability_required": "EditWithinTargetPaths",
    "file_patches": file_patches_arr,
    "unified_diff": candidate.unified_diff,
    "candidate_only": true,
    "next_step": match candidate.verdict {
      RemovePnixEdgeVerdict::RemovePnixEdgeReady => "tool-action-approval-then-materialize",
      _ => "operator-decision-or-resubmit",
    },
  });
  if let RemovePnixEdgeVerdict::RemovePnixEdgeHeld { held_kind, reason } = &candidate.verdict {
    payload["held_kind"] = serde_json::Value::String(held_kind.as_str().to_string());
    payload["reason"] = serde_json::Value::String(reason.clone());
  }
  if let Some(n) = candidate.edges_removed {
    payload["edges_removed"] = serde_json::Value::Number(n.into());
  }
  payload
}

pub fn build_remove_pnix_edge_patch_candidate_artifact(
  candidate: &RemovePnixEdgePatchCandidate,
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
) -> serde_json::Value {
  let payload = build_remove_pnix_edge_patch_candidate_payload(candidate);
  let req = &candidate.request;

  let mut hasher = Sha256::new();
  hasher.update(b"remove-pnix-edge-patch\x1f");
  hasher.update(req.target_path.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(req.from.render_inner().as_bytes());
  hasher.update(b"\x1f");
  hasher.update(req.to.render_inner().as_bytes());
  hasher.update(b"\x1f");
  for fp in &candidate.file_patches {
    hasher.update(fp.path.as_bytes());
    hasher.update(b"\x1e");
    hasher.update(fp.before_sha256.as_bytes());
    hasher.update(b"\x1e");
    hasher.update(fp.after_sha256.as_bytes());
    hasher.update(b"\x1d");
  }
  let digest = hasher.finalize();
  let prefix = digest
    .iter()
    .take(8)
    .map(|b| format!("{b:02x}"))
    .collect::<String>();
  let id = format!("generated-patch.remove-pnix-edge.{prefix}");

  let mut artifact = serde_json::json!({
    "id": id,
    "artifact_family": "coding.generated-patch-candidate",
    "source_surface": "code-transform.remove-pnix-edge",
    "stored_at_ms": stored_at_ms,
    "target_paths": [req.target_path.clone()],
    "command_refs": serde_json::Value::Array(Vec::new()),
    "related_refs": serde_json::json!([
      "owner-law:crates/pnix-core::code_transform::pnix_graph"
    ]),
    "payload": payload,
  });
  if let Some(snap) = repo_snapshot_ref {
    artifact["repo_snapshot_ref"] = serde_json::Value::String(snap.to_string());
  }
  artifact
}

// ─── remove-node-id transform (D-12: full chain layer) ──────────

/// Request shape for the `remove-node-id` graph-mode transform.
/// Single-file like `RenameNodeIdRequest`.
///
/// `cascade` (default `false`): when `true`, the carrier removes
/// the node entry AND every edge entry that references the node.
/// `cascade_edges_removed` in the resulting candidate carries the
/// count. When `false` (strict, default), the carrier refuses with
/// `still-referenced-by-edges` if any edge references the node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RemoveNodeIdRequest {
  pub target_path: String,
  pub name: String,
  #[serde(default)]
  pub cascade: bool,
}

/// Held / Rejected kinds. Mirrors the file-content-tier variants
/// from `RemoveNodeIdError` plus the request-shape-tier variants
/// (`empty-target-path` / `target-path-out-of-project`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RemoveNodeIdHeldKind {
  EmptyName,
  InvalidIdentifier,
  EmptyTargetPath,
  TargetPathOutOfProject,
  NotGraphMode,
  NoNodesSection,
  NodeNotFound,
  MalformedNodesSection,
  /// Strict-mode refusal: edges still reference the node. Caller
  /// removes the referencing edges first or invokes a future
  /// cascade variant.
  StillReferencedByEdges,
}

impl RemoveNodeIdHeldKind {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::EmptyName => "empty-name",
      Self::InvalidIdentifier => "invalid-identifier",
      Self::EmptyTargetPath => "empty-target-path",
      Self::TargetPathOutOfProject => "target-path-out-of-project",
      Self::NotGraphMode => "not-graph-mode",
      Self::NoNodesSection => "no-nodes-section",
      Self::NodeNotFound => "node-not-found",
      Self::MalformedNodesSection => "malformed-nodes-section",
      Self::StillReferencedByEdges => "still-referenced-by-edges",
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "verdict")]
pub enum RemoveNodeIdVerdict {
  RemoveNodeIdReady,
  RemoveNodeIdHeld {
    held_kind: RemoveNodeIdHeldKind,
    reason: String,
  },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoveNodeIdFilePatch {
  pub path: String,
  pub before_content: String,
  pub after_content: String,
  pub before_sha256: String,
  pub after_sha256: String,
}

#[derive(Debug, Clone, Copy)]
pub struct RemoveNodeIdFileInput<'a> {
  pub path: &'a str,
  pub content: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoveNodeIdPatchCandidate {
  pub request: RemoveNodeIdRequest,
  pub verdict: RemoveNodeIdVerdict,
  pub file_patches: Vec<RemoveNodeIdFilePatch>,
  pub unified_diff: String,
  /// Set only when the strict carrier refused with
  /// `still-referenced-by-edges`. The count of `node = "<name>"`
  /// references in the edges section. Caller can show this to the
  /// operator so they know how many edges they need to clean up
  /// first (or pass `cascade: true` to remove them automatically).
  pub edges_ref_count: Option<usize>,
  /// Set only when `request.cascade == true` AND the carrier
  /// succeeded. Count of edge entries the cascade variant
  /// auto-removed. `0` means the node had no edge references.
  pub cascade_edges_removed: Option<usize>,
}

/// Request-shape classifier.
pub fn classify_remove_node_id(req: &RemoveNodeIdRequest) -> RemoveNodeIdVerdict {
  if req.name.is_empty() {
    return RemoveNodeIdVerdict::RemoveNodeIdHeld {
      held_kind: RemoveNodeIdHeldKind::EmptyName,
      reason: "remove-node-id requires a non-empty `name`".to_string(),
    };
  }
  if !is_valid_pnix_node_identifier(&req.name) {
    return RemoveNodeIdVerdict::RemoveNodeIdHeld {
      held_kind: RemoveNodeIdHeldKind::InvalidIdentifier,
      reason: "name is not a valid ASCII identifier".to_string(),
    };
  }
  if req.target_path.is_empty() {
    return RemoveNodeIdVerdict::RemoveNodeIdHeld {
      held_kind: RemoveNodeIdHeldKind::EmptyTargetPath,
      reason: "remove-node-id requires a non-empty `target_path`".to_string(),
    };
  }
  if req.target_path.contains("..") || req.target_path.contains('\u{0}') {
    return RemoveNodeIdVerdict::RemoveNodeIdHeld {
      held_kind: RemoveNodeIdHeldKind::TargetPathOutOfProject,
      reason: "target_path must be within the project root and must not contain `..` or null bytes"
        .to_string(),
    };
  }
  RemoveNodeIdVerdict::RemoveNodeIdReady
}

fn remove_node_id_error_to_verdict(err: RemoveNodeIdError) -> (RemoveNodeIdVerdict, Option<usize>) {
  match err {
    RemoveNodeIdError::EmptyName => (
      RemoveNodeIdVerdict::RemoveNodeIdHeld {
        held_kind: RemoveNodeIdHeldKind::EmptyName,
        reason: "name is empty".to_string(),
      },
      None,
    ),
    RemoveNodeIdError::InvalidIdentifier => (
      RemoveNodeIdVerdict::RemoveNodeIdHeld {
        held_kind: RemoveNodeIdHeldKind::InvalidIdentifier,
        reason: "name is not a valid ASCII identifier".to_string(),
      },
      None,
    ),
    RemoveNodeIdError::NotGraphMode => (
      RemoveNodeIdVerdict::RemoveNodeIdHeld {
        held_kind: RemoveNodeIdHeldKind::NotGraphMode,
        reason: "target file is not a recognizable graph-mode `.px`".to_string(),
      },
      None,
    ),
    RemoveNodeIdError::NoNodesSection => (
      RemoveNodeIdVerdict::RemoveNodeIdHeld {
        held_kind: RemoveNodeIdHeldKind::NoNodesSection,
        reason: "graph file has no `nodes = [ ... ]` section".to_string(),
      },
      None,
    ),
    RemoveNodeIdError::NodeNotFound => (
      RemoveNodeIdVerdict::RemoveNodeIdHeld {
        held_kind: RemoveNodeIdHeldKind::NodeNotFound,
        reason: "no node entry with `name = \"<name>\"` was found".to_string(),
      },
      None,
    ),
    RemoveNodeIdError::MalformedNodesSection => (
      RemoveNodeIdVerdict::RemoveNodeIdHeld {
        held_kind: RemoveNodeIdHeldKind::MalformedNodesSection,
        reason: "nodes section has unbalanced braces or other malformed shape".to_string(),
      },
      None,
    ),
    RemoveNodeIdError::StillReferencedByEdges { ref_count } => (
      RemoveNodeIdVerdict::RemoveNodeIdHeld {
        held_kind: RemoveNodeIdHeldKind::StillReferencedByEdges,
        reason: format!(
          "edges section still references this node {ref_count} time(s); \
           remove the referencing edges first"
        ),
      },
      Some(ref_count),
    ),
  }
}

fn render_unified_diff_remove_node_id(patches: &[RemoveNodeIdFilePatch]) -> String {
  let mut out = String::new();
  for p in patches {
    out.push_str(&format!("--- a/{}\n", p.path));
    out.push_str(&format!("+++ b/{}\n", p.path));
    let before_lines: Vec<&str> = p.before_content.split_inclusive('\n').collect();
    let after_lines: Vec<&str> = p.after_content.split_inclusive('\n').collect();
    for line in &before_lines {
      out.push('-');
      out.push_str(line);
      if !line.ends_with('\n') {
        out.push('\n');
      }
    }
    for line in &after_lines {
      out.push('+');
      out.push_str(line);
      if !line.ends_with('\n') {
        out.push('\n');
      }
    }
  }
  out
}

/// Compute a `RemoveNodeIdPatchCandidate`. Strict by default;
/// when `request.cascade == true` the carrier also removes every
/// edge entry that references the node, surfacing the count in
/// `cascade_edges_removed`.
pub fn compute_remove_node_id_patch_candidate(
  request: &RemoveNodeIdRequest,
  file_input: &RemoveNodeIdFileInput<'_>,
) -> RemoveNodeIdPatchCandidate {
  let verdict = classify_remove_node_id(request);
  if !matches!(verdict, RemoveNodeIdVerdict::RemoveNodeIdReady) {
    return RemoveNodeIdPatchCandidate {
      request: request.clone(),
      verdict,
      file_patches: Vec::new(),
      unified_diff: String::new(),
      edges_ref_count: None,
      cascade_edges_removed: None,
    };
  }
  if file_input.path != request.target_path {
    return RemoveNodeIdPatchCandidate {
      request: request.clone(),
      verdict,
      file_patches: Vec::new(),
      unified_diff: String::new(),
      edges_ref_count: None,
      cascade_edges_removed: None,
    };
  }
  let before = file_input.content;
  if request.cascade {
    // ── Cascade lane ─────────────────────────────────────────
    match remove_node_entry_by_name_cascade(before, &request.name) {
      Ok(outcome) => {
        let mut patch = RemoveNodeIdFilePatch {
          path: file_input.path.to_string(),
          before_content: before.to_string(),
          after_content: outcome.new_source,
          before_sha256: sha256_hex_bytes(before.as_bytes()),
          after_sha256: String::new(),
        };
        patch.after_sha256 = sha256_hex_bytes(patch.after_content.as_bytes());
        let unified_diff = render_unified_diff_remove_node_id(std::slice::from_ref(&patch));
        RemoveNodeIdPatchCandidate {
          request: request.clone(),
          verdict,
          file_patches: vec![patch],
          unified_diff,
          edges_ref_count: None,
          cascade_edges_removed: Some(outcome.edges_removed),
        }
      }
      Err(err) => {
        let (held_verdict, _) = remove_node_id_error_to_verdict(err);
        RemoveNodeIdPatchCandidate {
          request: request.clone(),
          verdict: held_verdict,
          file_patches: Vec::new(),
          unified_diff: String::new(),
          edges_ref_count: None,
          cascade_edges_removed: None,
        }
      }
    }
  } else {
    // ── Strict lane (D-12 behavior) ──────────────────────────
    match remove_node_entry_by_name(before, &request.name) {
      Ok(after) => {
        let mut patch = RemoveNodeIdFilePatch {
          path: file_input.path.to_string(),
          before_content: before.to_string(),
          after_content: after,
          before_sha256: sha256_hex_bytes(before.as_bytes()),
          after_sha256: String::new(),
        };
        patch.after_sha256 = sha256_hex_bytes(patch.after_content.as_bytes());
        let unified_diff = render_unified_diff_remove_node_id(std::slice::from_ref(&patch));
        RemoveNodeIdPatchCandidate {
          request: request.clone(),
          verdict,
          file_patches: vec![patch],
          unified_diff,
          edges_ref_count: None,
          cascade_edges_removed: None,
        }
      }
      Err(err) => {
        let (held_verdict, ref_count) = remove_node_id_error_to_verdict(err);
        RemoveNodeIdPatchCandidate {
          request: request.clone(),
          verdict: held_verdict,
          file_patches: Vec::new(),
          unified_diff: String::new(),
          edges_ref_count: ref_count,
          cascade_edges_removed: None,
        }
      }
    }
  }
}

pub fn build_remove_node_id_patch_candidate_payload(
  candidate: &RemoveNodeIdPatchCandidate,
) -> serde_json::Value {
  let req = &candidate.request;
  let verdict_str = match &candidate.verdict {
    RemoveNodeIdVerdict::RemoveNodeIdReady => "remove-node-id-ready",
    RemoveNodeIdVerdict::RemoveNodeIdHeld { .. } => "remove-node-id-held",
  };
  let file_patches_arr: Vec<serde_json::Value> = candidate
    .file_patches
    .iter()
    .map(|fp| {
      serde_json::json!({
        "path": fp.path,
        "before_sha256": fp.before_sha256,
        "after_sha256": fp.after_sha256,
        "before_byte_len": fp.before_content.len(),
        "after_byte_len": fp.after_content.len(),
      })
    })
    .collect();
  let mut payload = serde_json::json!({
    "transform": "remove-node-id",
    "owner_law": "crates/pnix-core::code_transform::pnix_graph",
    "target_path": req.target_path,
    "name": req.name,
    "language": "pnix",
    "scope": "single-graph-file",
    "verdict": verdict_str,
    "capability_required": "EditWithinTargetPaths",
    "file_patches": file_patches_arr,
    "unified_diff": candidate.unified_diff,
    "candidate_only": true,
    "next_step": match candidate.verdict {
      RemoveNodeIdVerdict::RemoveNodeIdReady => "tool-action-approval-then-materialize",
      _ => "operator-decision-or-resubmit",
    },
  });
  if let RemoveNodeIdVerdict::RemoveNodeIdHeld { held_kind, reason } = &candidate.verdict {
    payload["held_kind"] = serde_json::Value::String(held_kind.as_str().to_string());
    payload["reason"] = serde_json::Value::String(reason.clone());
  }
  if let Some(rc) = candidate.edges_ref_count {
    payload["edges_ref_count"] = serde_json::Value::Number(rc.into());
  }
  // cascade signal — the request's cascade mode + how many edges
  // the cascade removed (when applicable).
  payload["cascade"] = serde_json::Value::Bool(candidate.request.cascade);
  if let Some(n) = candidate.cascade_edges_removed {
    payload["cascade_edges_removed"] = serde_json::Value::Number(n.into());
  }
  payload
}

pub fn build_remove_node_id_patch_candidate_artifact(
  candidate: &RemoveNodeIdPatchCandidate,
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
) -> serde_json::Value {
  let payload = build_remove_node_id_patch_candidate_payload(candidate);
  let req = &candidate.request;

  let mut hasher = Sha256::new();
  hasher.update(b"remove-node-id-patch\x1f");
  hasher.update(req.target_path.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(req.name.as_bytes());
  hasher.update(b"\x1f");
  // Cascade in/out distinguishes strict vs cascade dispatch on the
  // same source — same name + path + cascade=false produces a
  // different id from same name + path + cascade=true.
  hasher.update(if req.cascade { b"\x01" } else { b"\x00" });
  hasher.update(b"\x1f");
  for fp in &candidate.file_patches {
    hasher.update(fp.path.as_bytes());
    hasher.update(b"\x1e");
    hasher.update(fp.before_sha256.as_bytes());
    hasher.update(b"\x1e");
    hasher.update(fp.after_sha256.as_bytes());
    hasher.update(b"\x1d");
  }
  let digest = hasher.finalize();
  let prefix = digest
    .iter()
    .take(8)
    .map(|b| format!("{b:02x}"))
    .collect::<String>();
  let id = format!("generated-patch.remove-node-id.{prefix}");

  let mut artifact = serde_json::json!({
    "id": id,
    "artifact_family": "coding.generated-patch-candidate",
    "source_surface": "code-transform.remove-node-id",
    "stored_at_ms": stored_at_ms,
    "target_paths": [req.target_path.clone()],
    "command_refs": serde_json::Value::Array(Vec::new()),
    "related_refs": serde_json::json!([
      "owner-law:crates/pnix-core::code_transform::pnix_graph"
    ]),
    "payload": payload,
  });
  if let Some(snap) = repo_snapshot_ref {
    artifact["repo_snapshot_ref"] = serde_json::Value::String(snap.to_string());
  }
  artifact
}

/// Result of [`rename_node_id_in_source`] when input validation
/// catches a problem upstream of the actual edit. Mirrors the
/// per-transform Held / Rejected ladder shape used by
/// `rename_symbol::classify_rename`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenameNodeIdError {
  /// `old_name` or `new_name` is empty.
  EmptyName,
  /// `old_name == new_name` — nothing to do.
  OldEqualsNew,
  /// `old_name` or `new_name` is not a valid ASCII identifier.
  InvalidIdentifier,
  /// Source has no graph-mode shape (no outer attrset or no
  /// graph-defining sections).
  NotGraphMode,
  /// `nodes = [ ... ]` section missing — there's nothing to rename.
  NoNodesSection,
  /// `nodes` body has no `name = "<old_name>"` declaration.
  NodeNotFound,
  /// `nodes` body already contains `name = "<new_name>"` — rename
  /// would create a duplicate declaration.
  NewNameAlreadyDeclared,
}

fn is_valid_pnix_node_identifier(name: &str) -> bool {
  if name.is_empty() {
    return false;
  }
  let bytes = name.as_bytes();
  let first = bytes[0];
  if !(first.is_ascii_alphabetic() || first == b'_') {
    return false;
  }
  bytes[1..]
    .iter()
    .all(|&b| b.is_ascii_alphanumeric() || b == b'_')
}

/// Rename a node identity declared in the `nodes` section. Updates:
///
///   1. The node's `name = "<old>"` row inside `nodes` →
///      `name = "<new>"`.
///   2. Every `node = "<old>"` reference inside `edges` →
///      `node = "<new>"`.
///
/// Out of scope (intentionally — these are different concepts and
/// must not be silently renamed alongside the node id):
///
///   - `input = "<X>"` references in `edges` (those refer to graph
///     inputs, not nodes).
///   - `uses = "<X>"` in `nodes` (the extern the node delegates to).
///   - `externs[*].name`, top-level `name`, `types[*]`, `inputs[*]`.
///
/// OWNER-LAW (2026-05-13): the first **semantic** graph-mode
/// refactor — affects multiple call sites in a coordinated way and
/// validates name-collision before editing. Composes with
/// `add_node` / `add_edge` so a turn that adds-then-renames works
/// cleanly.
pub fn rename_node_id_in_source(
  source: &str,
  old_name: &str,
  new_name: &str,
) -> Result<String, RenameNodeIdError> {
  if old_name.is_empty() || new_name.is_empty() {
    return Err(RenameNodeIdError::EmptyName);
  }
  if old_name == new_name {
    return Err(RenameNodeIdError::OldEqualsNew);
  }
  if !is_valid_pnix_node_identifier(old_name) || !is_valid_pnix_node_identifier(new_name) {
    return Err(RenameNodeIdError::InvalidIdentifier);
  }
  let graph = parse_pnix_graph_mode(source).ok_or(RenameNodeIdError::NotGraphMode)?;
  if !graph.looks_like_graph_mode() {
    return Err(RenameNodeIdError::NotGraphMode);
  }
  let nodes = graph
    .section("nodes")
    .ok_or(RenameNodeIdError::NoNodesSection)?;
  let nodes_body = &source[nodes.body_inner_start_byte..nodes.body_inner_end_byte];

  let old_decl = format!("name = \"{old_name}\"");
  let new_decl = format!("name = \"{new_name}\"");
  if !nodes_body.contains(&old_decl) {
    return Err(RenameNodeIdError::NodeNotFound);
  }
  // Collision: new_name already declared (potentially as a different
  // existing node, OR as a literal `name = "<new>"` somewhere in the
  // nodes body). Refuse rather than create a duplicate.
  if nodes_body.contains(&new_decl) {
    return Err(RenameNodeIdError::NewNameAlreadyDeclared);
  }

  let old_ref = format!("node = \"{old_name}\"");
  let new_ref = format!("node = \"{new_name}\"");

  // Collect section-bounded edits as (start, end, replacement-text).
  // Apply in source-order. Other regions stay byte-identical.
  let mut edits: Vec<(usize, usize, String)> = Vec::new();
  edits.push((
    nodes.body_inner_start_byte,
    nodes.body_inner_end_byte,
    nodes_body.replace(&old_decl, &new_decl),
  ));
  if let Some(edges) = graph.section("edges") {
    let edges_body = &source[edges.body_inner_start_byte..edges.body_inner_end_byte];
    edits.push((
      edges.body_inner_start_byte,
      edges.body_inner_end_byte,
      edges_body.replace(&old_ref, &new_ref),
    ));
  }
  edits.sort_by_key(|(s, _, _)| *s);

  let mut out = String::with_capacity(source.len() + 16);
  let mut cursor = 0usize;
  for (start, end, new_text) in edits {
    out.push_str(&source[cursor..start]);
    out.push_str(&new_text);
    cursor = end;
  }
  out.push_str(&source[cursor..]);
  Ok(out)
}

#[cfg(test)]
mod tests {
  use super::*;

  // ─── parser ────────────────────────────────────────────────────

  const TREAP_GRAPH_MODE: &str = r#"# 트립 삭제
{
  name = "delete_treap_3";

  types = [ "Num" "Bool" ];

  externs = [
    {
      name = "builtins.eq";
      inputs = [ { name = "a"; ty = "Num"; } ];
      outputs = [ { name = "out"; ty = "Bool"; } ];
    }
  ];

  inputs = {
    current = "Num";
    delete_val = "Num";
  };

  nodes = [
    { name = "is_target"; uses = "builtins.eq"; }
  ];

  edges = [
    { from = { input = "current"; }; to = { node = "is_target"; port = "a"; }; }
  ];
}
"#;

  #[test]
  fn parses_treap_graph_mode_sample() {
    let g = parse_pnix_graph_mode(TREAP_GRAPH_MODE).expect("parse ok");
    assert!(g.looks_like_graph_mode());
    let names: Vec<&str> = g.sections.iter().map(|s| s.name).collect();
    // Every canonical section present.
    for expected in ["name", "types", "externs", "inputs", "nodes", "edges"] {
      assert!(
        names.contains(&expected),
        "missing section {expected}; got {names:?}"
      );
    }
  }

  #[test]
  fn parses_section_byte_ranges_correctly() {
    let g = parse_pnix_graph_mode(TREAP_GRAPH_MODE).expect("parse");
    let externs = g.section("externs").expect("externs");
    let body = &TREAP_GRAPH_MODE[externs.body_inner_start_byte..externs.body_inner_end_byte];
    assert!(body.contains("builtins.eq"));
    let inputs = g.section("inputs").expect("inputs");
    let inputs_body = &TREAP_GRAPH_MODE[inputs.body_inner_start_byte..inputs.body_inner_end_byte];
    assert!(inputs_body.contains("current"));
    assert!(inputs_body.contains("delete_val"));
  }

  #[test]
  fn parse_returns_none_on_empty_source() {
    assert!(parse_pnix_graph_mode("").is_none());
  }

  #[test]
  fn expression_mode_px_not_classified_as_graph_mode() {
    // Expression-mode .px (`let ... in ...`) has no externs / nodes
    // / edges, so `looks_like_graph_mode` returns false even if the
    // parser succeeds at locating top-level shape.
    let src = "let foo = 1; in foo + 2";
    let g = parse_pnix_graph_mode(src);
    // No outer `{ }` → parser returns None.
    assert!(g.is_none());
  }

  #[test]
  fn parser_skips_section_words_in_comments_and_strings() {
    // Source has `externs` inside a `#` comment and a `"..."`
    // string. The skip-zone scanner must prevent the parser from
    // matching those.
    let src = r#"{
  name = "the externs section is real, not this string";
  # The real externs = [...] is below; this comment mentions externs.
  externs = [
    { name = "real.extern"; }
  ];
}
"#;
    let g = parse_pnix_graph_mode(src).expect("parse");
    let externs = g.section("externs").expect("externs section");
    let body = &src[externs.body_inner_start_byte..externs.body_inner_end_byte];
    assert!(body.contains("real.extern"));
  }

  #[test]
  fn parser_handles_section_inside_nested_attrset_correctly() {
    // The OUTER externs is the one we want, not the inner one.
    // Nested attrsets must be at-depth>0 so they're not picked.
    let src = r#"{
  name = "outer";
  externs = [
    { name = "outer.ext"; }
  ];
  inputs = {
    # This `externs` is a key INSIDE the inputs attrset, not a
    # top-level section. Depth-aware parser must skip it.
    externs_placeholder = "nope";
  };
}
"#;
    let g = parse_pnix_graph_mode(src).expect("parse");
    let externs = g.section("externs").expect("externs");
    let body = &src[externs.body_inner_start_byte..externs.body_inner_end_byte];
    assert!(body.contains("outer.ext"));
    assert!(!body.contains("nope"));
  }

  #[test]
  fn parser_returns_partial_shape_when_some_sections_missing() {
    // A minimal externs-only file (e.g. an extern manifest).
    let src = r#"{
  externs = [
    { name = "a"; }
  ];
}
"#;
    let g = parse_pnix_graph_mode(src).expect("parse");
    assert!(g.looks_like_graph_mode(), "has externs → graph-mode");
    let names: Vec<&str> = g.sections.iter().map(|s| s.name).collect();
    assert!(names.contains(&"externs"));
    assert!(!names.contains(&"nodes")); // not present in source
    assert!(!names.contains(&"edges"));
  }

  // ─── add_extern_entry_to_source ────────────────────────────────

  #[test]
  fn add_extern_into_non_empty_list_appends_at_end() {
    let src = r#"{
  name = "g";
  externs = [
    { name = "a.b"; }
  ];
}
"#;
    let edited = add_extern_entry_to_source(src, r#"name = "c.d""#).expect("edit ok");
    // Both entries present in order.
    let pos_a = edited.find("a.b").expect("a.b in edited");
    let pos_c = edited.find("c.d").expect("c.d in edited");
    assert!(pos_a < pos_c, "new entry must be appended");
    // Closing `]` still present.
    assert!(edited.contains("];"));
    // Parser still works post-edit.
    let g = parse_pnix_graph_mode(&edited).expect("reparse");
    let body = &edited[g.section("externs").unwrap().body_inner_start_byte
      ..g.section("externs").unwrap().body_inner_end_byte];
    assert!(body.contains("a.b"));
    assert!(body.contains("c.d"));
  }

  #[test]
  fn add_extern_indents_to_match_existing_close_bracket() {
    let src = "{\n  externs = [\n    { name = \"a\"; }\n  ];\n}\n";
    let edited = add_extern_entry_to_source(src, r#"name = "b""#).expect("edit");
    // The new entry should be at the same 4-space indent as the
    // existing entry. We don't check exact column, but the new
    // entry's line should start with at least 4 spaces.
    let new_line = edited
      .lines()
      .find(|l| l.contains("\"b\""))
      .expect("new entry line");
    assert!(
      new_line.starts_with("    "),
      "expected 4-space indent on new entry; got: {new_line:?}"
    );
  }

  #[test]
  fn add_extern_returns_none_when_no_externs_section() {
    let src = "{\n  name = \"only-name\";\n}\n";
    assert!(add_extern_entry_to_source(src, r#"name = "x""#).is_none());
  }

  #[test]
  fn add_extern_then_re_add_produces_three_entries() {
    let src = "{\n  externs = [\n    { name = \"a\"; }\n  ];\n}\n";
    let step1 = add_extern_entry_to_source(src, r#"name = "b""#).expect("step1");
    let step2 = add_extern_entry_to_source(&step1, r#"name = "c""#).expect("step2");
    let g = parse_pnix_graph_mode(&step2).expect("reparse");
    let externs = g.section("externs").expect("externs");
    let body = &step2[externs.body_inner_start_byte..externs.body_inner_end_byte];
    assert!(body.contains("\"a\""));
    assert!(body.contains("\"b\""));
    assert!(body.contains("\"c\""));
    // Order preserved: a → b → c.
    let pos_a = body.find("\"a\"").unwrap();
    let pos_b = body.find("\"b\"").unwrap();
    let pos_c = body.find("\"c\"").unwrap();
    assert!(pos_a < pos_b && pos_b < pos_c);
  }

  #[test]
  fn add_extern_replay_stable_same_input_same_output() {
    let src = "{\n  externs = [\n    { name = \"a\"; }\n  ];\n}\n";
    let a = add_extern_entry_to_source(src, r#"name = "b""#).expect("a");
    let b = add_extern_entry_to_source(src, r#"name = "b""#).expect("b");
    assert_eq!(a, b);
  }

  // ─── add_node_entry_to_source (D-2) ────────────────────────────

  const NODES_GRAPH_SRC: &str = r#"{
  name = "g";
  externs = [
    { name = "builtins.add"; }
  ];
  nodes = [
    { name = "first"; uses = "builtins.add"; }
  ];
}
"#;

  #[test]
  fn add_node_into_existing_nodes_list_appends_at_end() {
    let edited =
      add_node_entry_to_source(NODES_GRAPH_SRC, r#"name = "second"; uses = "builtins.add""#)
        .expect("edit ok");
    let pos_first = edited.find("\"first\"").expect("first present");
    let pos_second = edited.find("\"second\"").expect("second present");
    assert!(pos_first < pos_second, "new node must be appended");
    // Reparse and confirm shape is still valid graph-mode.
    let g = parse_pnix_graph_mode(&edited).expect("reparse");
    assert!(g.looks_like_graph_mode());
    let nodes = g.section("nodes").expect("nodes");
    let body = &edited[nodes.body_inner_start_byte..nodes.body_inner_end_byte];
    assert!(body.contains("\"first\""));
    assert!(body.contains("\"second\""));
  }

  #[test]
  fn add_node_indents_to_match_existing_close_bracket() {
    let edited = add_node_entry_to_source(NODES_GRAPH_SRC, r#"name = "x"; uses = "builtins.add""#)
      .expect("edit");
    let new_line = edited
      .lines()
      .find(|l| l.contains("\"x\""))
      .expect("new entry line");
    assert!(
      new_line.starts_with("    "),
      "expected 4-space indent on new node entry; got: {new_line:?}"
    );
  }

  #[test]
  fn add_node_returns_none_when_no_nodes_section() {
    // Source has externs but no nodes — common in extern-only
    // manifests during early authoring.
    let src = "{\n  externs = [\n    { name = \"x\"; }\n  ];\n}\n";
    assert!(add_node_entry_to_source(src, r#"name = "y"; uses = "x""#).is_none());
  }

  #[test]
  fn add_node_replay_stable_same_input_same_output() {
    let a =
      add_node_entry_to_source(NODES_GRAPH_SRC, r#"name = "z"; uses = "builtins.add""#).expect("a");
    let b =
      add_node_entry_to_source(NODES_GRAPH_SRC, r#"name = "z"; uses = "builtins.add""#).expect("b");
    assert_eq!(a, b);
  }

  #[test]
  fn add_extern_and_add_node_compose_on_same_source() {
    // Real-world ergonomics: a graph-mode .px gets one extern + one
    // node added in the same turn. The order of operations must not
    // matter, and both sections grow independently.
    let with_extern =
      add_extern_entry_to_source(NODES_GRAPH_SRC, r#"name = "py.sub""#).expect("extern");
    let with_both = add_node_entry_to_source(&with_extern, r#"name = "sub_node"; uses = "py.sub""#)
      .expect("node");
    let g = parse_pnix_graph_mode(&with_both).expect("reparse");
    let externs_body = &with_both[g.section("externs").unwrap().body_inner_start_byte
      ..g.section("externs").unwrap().body_inner_end_byte];
    assert!(externs_body.contains("\"builtins.add\""));
    assert!(externs_body.contains("\"py.sub\""));
    let nodes_body = &with_both[g.section("nodes").unwrap().body_inner_start_byte
      ..g.section("nodes").unwrap().body_inner_end_byte];
    assert!(nodes_body.contains("\"first\""));
    assert!(nodes_body.contains("\"sub_node\""));
  }

  // ─── add_edge_entry_to_source (D-3) ────────────────────────────

  const EDGES_GRAPH_SRC: &str = r#"{
  name = "g";
  externs = [
    { name = "builtins.add"; }
  ];
  inputs = {
    a = "Num";
    b = "Num";
  };
  nodes = [
    { name = "sum"; uses = "builtins.add"; }
  ];
  edges = [
    { from = { input = "a"; }; to = { node = "sum"; port = "lhs"; }; }
  ];
}
"#;

  #[test]
  fn edge_endpoint_render_input_form() {
    let ep = EdgeEndpoint::Input { name: "current" };
    assert_eq!(ep.render_inner(), r#"input = "current""#);
  }

  #[test]
  fn edge_endpoint_render_node_with_port() {
    let ep = EdgeEndpoint::Node {
      name: "is_target",
      port: Some("a"),
    };
    assert_eq!(ep.render_inner(), r#"node = "is_target"; port = "a""#);
  }

  #[test]
  fn edge_endpoint_render_node_without_port() {
    let ep = EdgeEndpoint::Node {
      name: "result",
      port: None,
    };
    assert_eq!(ep.render_inner(), r#"node = "result""#);
  }

  #[test]
  fn build_edge_entry_text_input_to_node_port() {
    let body = build_edge_entry_text(
      &EdgeEndpoint::Input { name: "current" },
      &EdgeEndpoint::Node {
        name: "is_target",
        port: Some("a"),
      },
    );
    assert_eq!(
      body,
      r#"from = { input = "current"; }; to = { node = "is_target"; port = "a"; }"#
    );
  }

  #[test]
  fn build_edge_entry_text_node_to_node_with_ports() {
    let body = build_edge_entry_text(
      &EdgeEndpoint::Node {
        name: "sum",
        port: Some("out"),
      },
      &EdgeEndpoint::Node {
        name: "result",
        port: Some("lhs"),
      },
    );
    assert!(body.contains(r#"from = { node = "sum"; port = "out"; }"#));
    assert!(body.contains(r#"to = { node = "result"; port = "lhs"; }"#));
  }

  #[test]
  fn add_edge_into_existing_edges_list_appends_at_end() {
    let body = build_edge_entry_text(
      &EdgeEndpoint::Input { name: "b" },
      &EdgeEndpoint::Node {
        name: "sum",
        port: Some("rhs"),
      },
    );
    let edited = add_edge_entry_to_source(EDGES_GRAPH_SRC, &body).expect("edit ok");
    // First edge (a→sum:lhs) preserved.
    assert!(edited.contains(r#"from = { input = "a"; }"#));
    // New edge (b→sum:rhs) appended.
    assert!(edited.contains(r#"from = { input = "b"; }"#));
    assert!(edited.contains(r#"to = { node = "sum"; port = "rhs"; }"#));
    // Reparse — graph still valid, edges section grew.
    let g = parse_pnix_graph_mode(&edited).expect("reparse");
    assert!(g.looks_like_graph_mode());
    let edges_body = &edited[g.section("edges").unwrap().body_inner_start_byte
      ..g.section("edges").unwrap().body_inner_end_byte];
    // Both edge entries present.
    assert!(edges_body.contains(r#"input = "a""#));
    assert!(edges_body.contains(r#"input = "b""#));
  }

  #[test]
  fn add_edge_returns_none_when_no_edges_section() {
    // Source has nodes but no edges (early authoring state).
    let src = r#"{
  nodes = [
    { name = "x"; uses = "y"; }
  ];
}
"#;
    let body = build_edge_entry_text(
      &EdgeEndpoint::Input { name: "a" },
      &EdgeEndpoint::Node {
        name: "x",
        port: None,
      },
    );
    assert!(add_edge_entry_to_source(src, &body).is_none());
  }

  #[test]
  fn add_edge_replay_stable_same_input_same_output() {
    let body = build_edge_entry_text(
      &EdgeEndpoint::Input { name: "b" },
      &EdgeEndpoint::Node {
        name: "sum",
        port: Some("rhs"),
      },
    );
    let a = add_edge_entry_to_source(EDGES_GRAPH_SRC, &body).expect("a");
    let b = add_edge_entry_to_source(EDGES_GRAPH_SRC, &body).expect("b");
    assert_eq!(a, b);
  }

  #[test]
  fn full_pnix3d_authoring_flow_add_extern_node_edge_in_sequence() {
    // End-to-end: simulate a live-coding turn that adds a new
    // extern, a node that uses it, and an edge that wires the
    // existing graph input into the new node. Three D-1/D-2/D-3
    // calls compose cleanly on the same source.
    let s1 =
      add_extern_entry_to_source(EDGES_GRAPH_SRC, r#"name = "builtins.mul""#).expect("extern");
    let s2 =
      add_node_entry_to_source(&s1, r#"name = "prod"; uses = "builtins.mul""#).expect("node");
    let edge_body = build_edge_entry_text(
      &EdgeEndpoint::Input { name: "a" },
      &EdgeEndpoint::Node {
        name: "prod",
        port: Some("lhs"),
      },
    );
    let s3 = add_edge_entry_to_source(&s2, &edge_body).expect("edge");
    // Reparse final state.
    let g = parse_pnix_graph_mode(&s3).expect("reparse");
    assert!(g.looks_like_graph_mode());
    // All three sections grew exactly once each.
    let externs_body = &s3[g.section("externs").unwrap().body_inner_start_byte
      ..g.section("externs").unwrap().body_inner_end_byte];
    let nodes_body = &s3[g.section("nodes").unwrap().body_inner_start_byte
      ..g.section("nodes").unwrap().body_inner_end_byte];
    let edges_body = &s3[g.section("edges").unwrap().body_inner_start_byte
      ..g.section("edges").unwrap().body_inner_end_byte];
    assert!(externs_body.contains("builtins.add"));
    assert!(externs_body.contains("builtins.mul"));
    assert!(nodes_body.contains("\"sum\""));
    assert!(nodes_body.contains("\"prod\""));
    assert!(edges_body.contains("port = \"lhs\""));
    assert!(edges_body.contains("port = \"rhs\"") || edges_body.contains("input = \"a\""));
  }

  // ─── rename_node_id_in_source (D-4) ────────────────────────────

  const RENAME_GRAPH_SRC: &str = r#"{
  name = "g";
  externs = [
    { name = "builtins.add"; }
  ];
  inputs = {
    a = "Num";
    b = "Num";
  };
  nodes = [
    { name = "sum"; uses = "builtins.add"; }
    { name = "result"; uses = "builtins.add"; }
  ];
  edges = [
    { from = { input = "a"; }; to = { node = "sum"; port = "lhs"; }; }
    { from = { input = "b"; }; to = { node = "sum"; port = "rhs"; }; }
    { from = { node = "sum"; port = "out"; }; to = { node = "result"; port = "lhs"; }; }
  ];
}
"#;

  #[test]
  fn rename_node_id_updates_declaration_and_all_edge_refs() {
    let edited = rename_node_id_in_source(RENAME_GRAPH_SRC, "sum", "total").expect("rename ok");
    // Declaration updated in nodes section.
    assert!(edited.contains("name = \"total\""));
    assert!(!edited.contains("name = \"sum\""));
    // Other node ("result") preserved.
    assert!(edited.contains("name = \"result\""));
    // All `node = "sum"` references in edges (3 occurrences:
    // to=sum, to=sum, from=sum) updated.
    assert!(!edited.contains("node = \"sum\""));
    assert_eq!(
      edited.matches("node = \"total\"").count(),
      3,
      "expected 3 edge refs to total (2 to + 1 from); got: {}",
      edited.matches("node = \"total\"").count()
    );
    // The `uses = "builtins.add"` extern reference must NOT have
    // been touched (it's not a node id).
    assert!(edited.contains("uses = \"builtins.add\""));
    // Reparse — graph still valid.
    let g = parse_pnix_graph_mode(&edited).expect("reparse");
    assert!(g.looks_like_graph_mode());
  }

  #[test]
  fn rename_node_id_does_not_touch_graph_input_refs() {
    // Edge case: graph input "a" might also be a node name in some
    // graphs. Renaming a node "a" must NOT touch `input = "a"`
    // references in edges (those reference the graph input, not the
    // node).
    let src = r#"{
  name = "g";
  inputs = { a = "Num"; };
  nodes = [
    { name = "a"; uses = "builtins.id"; }
  ];
  edges = [
    { from = { input = "a"; }; to = { node = "a"; port = "x"; }; }
  ];
}
"#;
    let edited = rename_node_id_in_source(src, "a", "node_a").expect("rename ok");
    // Node decl renamed.
    assert!(edited.contains("name = \"node_a\""));
    // Edge ref renamed.
    assert!(edited.contains("node = \"node_a\""));
    // BUT `input = "a"` (graph input) preserved.
    assert!(
      edited.contains("input = \"a\""),
      "graph input `a` must NOT be renamed; got:\n{edited}"
    );
  }

  #[test]
  fn rename_node_id_does_not_touch_extern_name() {
    // A node `name = "x"` and an extern `name = "x"` can coexist
    // (different namespaces). Renaming the node must NOT touch the
    // extern declaration.
    let src = r#"{
  externs = [
    { name = "x"; }
  ];
  nodes = [
    { name = "x"; uses = "x"; }
  ];
  edges = [
    { from = { node = "x"; }; to = { node = "x"; port = "p"; }; }
  ];
}
"#;
    let edited = rename_node_id_in_source(src, "x", "renamed").expect("rename");
    // externs section must still have `name = "x"`. Confirm by
    // re-parsing and checking the externs body.
    let g = parse_pnix_graph_mode(&edited).expect("reparse");
    let externs_body = &edited[g.section("externs").unwrap().body_inner_start_byte
      ..g.section("externs").unwrap().body_inner_end_byte];
    assert!(
      externs_body.contains("name = \"x\""),
      "externs `name = \"x\"` must NOT be renamed; got externs body:\n{externs_body}"
    );
    // nodes body has `name = "renamed"` now.
    let nodes_body = &edited[g.section("nodes").unwrap().body_inner_start_byte
      ..g.section("nodes").unwrap().body_inner_end_byte];
    assert!(nodes_body.contains("name = \"renamed\""));
    assert!(!nodes_body.contains("name = \"x\""));
  }

  #[test]
  fn rename_node_id_refuses_empty_names() {
    assert_eq!(
      rename_node_id_in_source(RENAME_GRAPH_SRC, "", "x"),
      Err(RenameNodeIdError::EmptyName)
    );
    assert_eq!(
      rename_node_id_in_source(RENAME_GRAPH_SRC, "x", ""),
      Err(RenameNodeIdError::EmptyName)
    );
  }

  #[test]
  fn rename_node_id_refuses_same_old_and_new() {
    assert_eq!(
      rename_node_id_in_source(RENAME_GRAPH_SRC, "sum", "sum"),
      Err(RenameNodeIdError::OldEqualsNew)
    );
  }

  #[test]
  fn rename_node_id_refuses_invalid_identifier() {
    // Leading digit, space, hyphen — all invalid pnix-side
    // identifiers per the corpus convention.
    for bad in ["1abc", "has space", "with-hyphen"] {
      assert_eq!(
        rename_node_id_in_source(RENAME_GRAPH_SRC, "sum", bad),
        Err(RenameNodeIdError::InvalidIdentifier),
        "for new_name `{bad}`"
      );
    }
  }

  #[test]
  fn rename_node_id_returns_node_not_found_for_unknown_old_name() {
    assert_eq!(
      rename_node_id_in_source(RENAME_GRAPH_SRC, "no_such_node", "x"),
      Err(RenameNodeIdError::NodeNotFound)
    );
  }

  #[test]
  fn rename_node_id_refuses_collision_with_existing_name() {
    // `result` already exists in nodes — rename `sum` → `result`
    // would create a duplicate. Refuse.
    assert_eq!(
      rename_node_id_in_source(RENAME_GRAPH_SRC, "sum", "result"),
      Err(RenameNodeIdError::NewNameAlreadyDeclared)
    );
  }

  #[test]
  fn rename_node_id_returns_no_nodes_section_when_absent() {
    let src = "{\n  externs = [\n    { name = \"x\"; }\n  ];\n}\n";
    assert_eq!(
      rename_node_id_in_source(src, "x", "y"),
      Err(RenameNodeIdError::NoNodesSection)
    );
  }

  #[test]
  fn rename_node_id_handles_no_edges_section() {
    // Some early-authoring files have nodes but no edges. Rename
    // should still update the node decl and return Ok.
    let src = r#"{
  nodes = [
    { name = "lonely"; uses = "x"; }
  ];
}
"#;
    let edited = rename_node_id_in_source(src, "lonely", "renamed").expect("rename");
    assert!(edited.contains("name = \"renamed\""));
    assert!(!edited.contains("name = \"lonely\""));
  }

  #[test]
  fn rename_node_id_replay_stable_same_input_same_output() {
    let a = rename_node_id_in_source(RENAME_GRAPH_SRC, "sum", "total").expect("a");
    let b = rename_node_id_in_source(RENAME_GRAPH_SRC, "sum", "total").expect("b");
    assert_eq!(a, b);
  }

  #[test]
  fn rename_node_id_composes_with_add_node_in_same_turn() {
    // Real-world flow: add a node, realize a better name, rename
    // it. Both edits compose on the same source.
    let with_added =
      add_node_entry_to_source(RENAME_GRAPH_SRC, r#"name = "tmp"; uses = "builtins.add""#)
        .expect("add");
    let renamed = rename_node_id_in_source(&with_added, "tmp", "doubler").expect("rename");
    let g = parse_pnix_graph_mode(&renamed).expect("reparse");
    let nodes_body = &renamed[g.section("nodes").unwrap().body_inner_start_byte
      ..g.section("nodes").unwrap().body_inner_end_byte];
    assert!(nodes_body.contains("name = \"doubler\""));
    assert!(!nodes_body.contains("name = \"tmp\""));
    // Original nodes still there.
    assert!(nodes_body.contains("name = \"sum\""));
    assert!(nodes_body.contains("name = \"result\""));
  }

  // ─── remove_node_entry_by_name (D-5) ───────────────────────────

  const REMOVE_GRAPH_SRC: &str = r#"{
  name = "g";
  externs = [
    { name = "builtins.add"; }
  ];
  nodes = [
    { name = "first"; uses = "builtins.add"; }
    { name = "orphan"; uses = "builtins.add"; }
    { name = "third"; uses = "builtins.add"; }
  ];
  edges = [
    { from = { input = "a"; }; to = { node = "first"; port = "lhs"; }; }
    { from = { input = "b"; }; to = { node = "third"; port = "lhs"; }; }
  ];
}
"#;

  #[test]
  fn remove_node_orphan_succeeds_and_drops_entry() {
    // `orphan` has zero `node = "orphan"` refs in edges → strict
    // remove succeeds.
    let edited = remove_node_entry_by_name(REMOVE_GRAPH_SRC, "orphan").expect("remove ok");
    assert!(!edited.contains("name = \"orphan\""));
    // Other nodes preserved.
    assert!(edited.contains("name = \"first\""));
    assert!(edited.contains("name = \"third\""));
    // Trailing newline gobbled — no orphan blank line.
    assert!(
      !edited.contains("\n\n  ];"),
      "expected no orphan blank line before closing `]`; got:\n{edited}"
    );
    // Reparse still valid.
    let g = parse_pnix_graph_mode(&edited).expect("reparse");
    assert!(g.looks_like_graph_mode());
    let nodes = g.section("nodes").expect("nodes");
    let entries = parse_list_entries(&edited, nodes).expect("entries");
    assert_eq!(entries.len(), 2, "two nodes remain");
  }

  #[test]
  fn remove_node_refuses_when_referenced_by_edges() {
    // `first` has 1 `node = "first"` reference in edges → strict
    // refuse with the ref_count exposed.
    let err = remove_node_entry_by_name(REMOVE_GRAPH_SRC, "first").unwrap_err();
    assert_eq!(
      err,
      RemoveNodeIdError::StillReferencedByEdges { ref_count: 1 }
    );
  }

  #[test]
  fn remove_node_refuses_multi_ref_count_visible() {
    let src = r#"{
  nodes = [
    { name = "x"; uses = "y"; }
  ];
  edges = [
    { from = { node = "x"; port = "out"; }; to = { node = "a"; port = "p"; }; }
    { from = { input = "i"; }; to = { node = "x"; port = "lhs"; }; }
    { from = { node = "x"; }; to = { node = "b"; port = "rhs"; }; }
  ];
}
"#;
    let err = remove_node_entry_by_name(src, "x").unwrap_err();
    assert_eq!(
      err,
      RemoveNodeIdError::StillReferencedByEdges { ref_count: 3 }
    );
  }

  #[test]
  fn remove_node_succeeds_when_no_edges_section() {
    // Some early-authoring files have no edges section. The
    // reference check is a no-op there.
    let src = r#"{
  nodes = [
    { name = "lonely"; uses = "x"; }
    { name = "kept"; uses = "y"; }
  ];
}
"#;
    let edited = remove_node_entry_by_name(src, "lonely").expect("remove");
    assert!(!edited.contains("name = \"lonely\""));
    assert!(edited.contains("name = \"kept\""));
  }

  #[test]
  fn remove_node_unknown_name_returns_node_not_found() {
    assert_eq!(
      remove_node_entry_by_name(REMOVE_GRAPH_SRC, "no_such_node"),
      Err(RemoveNodeIdError::NodeNotFound)
    );
  }

  #[test]
  fn remove_node_refuses_empty_name() {
    assert_eq!(
      remove_node_entry_by_name(REMOVE_GRAPH_SRC, ""),
      Err(RemoveNodeIdError::EmptyName)
    );
  }

  #[test]
  fn remove_node_refuses_invalid_identifier() {
    for bad in ["1abc", "has space", "with-hyphen"] {
      assert_eq!(
        remove_node_entry_by_name(REMOVE_GRAPH_SRC, bad),
        Err(RemoveNodeIdError::InvalidIdentifier),
        "for name `{bad}`"
      );
    }
  }

  #[test]
  fn remove_node_no_nodes_section_returns_held() {
    let src = "{\n  externs = [\n    { name = \"x\"; }\n  ];\n}\n";
    assert_eq!(
      remove_node_entry_by_name(src, "x"),
      Err(RemoveNodeIdError::NoNodesSection)
    );
  }

  #[test]
  fn remove_node_replay_stable_same_input_same_output() {
    let a = remove_node_entry_by_name(REMOVE_GRAPH_SRC, "orphan").expect("a");
    let b = remove_node_entry_by_name(REMOVE_GRAPH_SRC, "orphan").expect("b");
    assert_eq!(a, b);
  }

  #[test]
  fn remove_node_composes_after_rename_in_same_turn() {
    // Add a tmp node, then rename, then remove — the same source
    // walks through three edit kinds and ends cleanly.
    let s1 = add_node_entry_to_source(REMOVE_GRAPH_SRC, r#"name = "tmp"; uses = "builtins.add""#)
      .expect("add");
    let s2 = rename_node_id_in_source(&s1, "tmp", "scratch").expect("rename");
    // `scratch` has no edges referring to it → can be removed.
    let s3 = remove_node_entry_by_name(&s2, "scratch").expect("remove");
    let g = parse_pnix_graph_mode(&s3).expect("reparse");
    let nodes_body = &s3[g.section("nodes").unwrap().body_inner_start_byte
      ..g.section("nodes").unwrap().body_inner_end_byte];
    assert!(!nodes_body.contains("\"tmp\""));
    assert!(!nodes_body.contains("\"scratch\""));
    // Originals intact.
    assert!(nodes_body.contains("\"first\""));
    assert!(nodes_body.contains("\"orphan\""));
    assert!(nodes_body.contains("\"third\""));
  }

  // ─── parse_list_entries (D-5 generic helper) ───────────────────

  #[test]
  fn parse_list_entries_returns_entry_byte_ranges() {
    let g = parse_pnix_graph_mode(REMOVE_GRAPH_SRC).expect("parse");
    let nodes = g.section("nodes").expect("nodes");
    let entries = parse_list_entries(REMOVE_GRAPH_SRC, nodes).expect("entries");
    assert_eq!(entries.len(), 3, "three node entries");
    for (s, e) in &entries {
      let slice = &REMOVE_GRAPH_SRC[*s..*e];
      assert!(slice.starts_with('{'));
      assert!(slice.ends_with('}'));
    }
  }

  #[test]
  fn parse_list_entries_refuses_attrset_shape() {
    let g = parse_pnix_graph_mode(REMOVE_GRAPH_SRC).expect("parse");
    let externs = g.section("externs").expect("externs");
    // externs IS a list, so this should work.
    assert!(parse_list_entries(REMOVE_GRAPH_SRC, externs).is_some());
    // But inputs (in a different fixture) is an Attrset — try a
    // fixture that has it.
    let with_inputs = r#"{
  externs = [];
  inputs = { a = "Num"; };
  nodes = [];
  edges = [];
}
"#;
    let g2 = parse_pnix_graph_mode(with_inputs).expect("parse2");
    let inputs = g2.section("inputs").expect("inputs");
    assert!(
      parse_list_entries(with_inputs, inputs).is_none(),
      "Attrset shape must yield None from parse_list_entries"
    );
  }

  // ─── compute_rename_node_id_patch_candidate (D-8) ──────────────

  const D8_GRAPH_SRC: &str = r#"{
  externs = [ { name = "builtins.add"; } ];
  nodes = [
    { name = "sum"; uses = "builtins.add"; }
  ];
  edges = [
    { from = { input = "a"; }; to = { node = "sum"; port = "lhs"; }; }
  ];
}
"#;

  fn d8_req(target: &str, old: &str, new: &str) -> RenameNodeIdRequest {
    RenameNodeIdRequest {
      target_path: target.to_string(),
      old_name: old.to_string(),
      new_name: new.to_string(),
    }
  }

  #[test]
  fn classify_rename_node_id_ready_for_well_formed_request() {
    let v = classify_rename_node_id(&d8_req("examples/g.px", "a", "b"));
    assert!(matches!(v, RenameNodeIdVerdict::RenameNodeIdReady));
  }

  #[test]
  fn classify_rename_node_id_ladder_catches_request_shape_problems() {
    for (req, expected_kind) in [
      (
        d8_req("examples/g.px", "", "b"),
        RenameNodeIdHeldKind::EmptyOldName,
      ),
      (
        d8_req("examples/g.px", "a", ""),
        RenameNodeIdHeldKind::EmptyNewName,
      ),
      (
        d8_req("examples/g.px", "1abc", "x"),
        RenameNodeIdHeldKind::InvalidIdentifier,
      ),
      (d8_req("", "a", "b"), RenameNodeIdHeldKind::EmptyTargetPath),
      (
        d8_req("../escape.px", "a", "b"),
        RenameNodeIdHeldKind::TargetPathOutOfProject,
      ),
    ] {
      match classify_rename_node_id(&req) {
        RenameNodeIdVerdict::RenameNodeIdHeld { held_kind, .. } => {
          assert_eq!(held_kind, expected_kind, "for req: {req:?}");
        }
        other => panic!("expected Held({expected_kind:?}), got {other:?}"),
      }
    }
  }

  #[test]
  fn classify_rename_node_id_rejects_old_equals_new() {
    let v = classify_rename_node_id(&d8_req("examples/g.px", "x", "x"));
    match v {
      RenameNodeIdVerdict::RenameNodeIdRejected { held_kind, .. } => {
        assert_eq!(held_kind, RenameNodeIdHeldKind::OldEqualsNew);
      }
      other => panic!("expected Rejected(OldEqualsNew), got {other:?}"),
    }
  }

  #[test]
  fn compute_rename_node_id_patch_candidate_ready_emits_patch_and_diff() {
    let req = d8_req("examples/g.px", "sum", "total");
    let file_input = RenameNodeIdFileInput {
      path: "examples/g.px",
      content: D8_GRAPH_SRC,
    };
    let cand = compute_rename_node_id_patch_candidate(&req, &file_input);
    assert!(matches!(
      cand.verdict,
      RenameNodeIdVerdict::RenameNodeIdReady
    ));
    assert_eq!(cand.file_patches.len(), 1);
    let fp = &cand.file_patches[0];
    assert_eq!(fp.path, "examples/g.px");
    assert_eq!(fp.before_content, D8_GRAPH_SRC);
    assert!(fp.after_content.contains("name = \"total\""));
    assert!(fp.after_content.contains("node = \"total\""));
    assert!(!fp.after_content.contains("name = \"sum\""));
    assert!(!fp.after_content.contains("node = \"sum\""));
    assert_ne!(fp.before_sha256, fp.after_sha256);
    assert!(cand.unified_diff.contains("--- a/examples/g.px"));
  }

  #[test]
  fn compute_rename_node_id_patch_candidate_downgrades_node_not_found_to_held() {
    let req = d8_req("examples/g.px", "no_such_node", "y");
    let file_input = RenameNodeIdFileInput {
      path: "examples/g.px",
      content: D8_GRAPH_SRC,
    };
    let cand = compute_rename_node_id_patch_candidate(&req, &file_input);
    match cand.verdict {
      RenameNodeIdVerdict::RenameNodeIdHeld { held_kind, .. } => {
        assert_eq!(held_kind, RenameNodeIdHeldKind::NodeNotFound);
      }
      other => panic!("expected Held(NodeNotFound), got {other:?}"),
    }
    assert!(cand.file_patches.is_empty());
    assert!(cand.unified_diff.is_empty());
  }

  #[test]
  fn compute_rename_node_id_patch_candidate_no_op_when_path_mismatch() {
    // file_input.path != request.target_path → empty patch, no
    // edit attempted. Verdict stays Ready (classifier-level).
    let req = d8_req("examples/g.px", "sum", "total");
    let file_input = RenameNodeIdFileInput {
      path: "examples/wrong.px",
      content: D8_GRAPH_SRC,
    };
    let cand = compute_rename_node_id_patch_candidate(&req, &file_input);
    assert!(matches!(
      cand.verdict,
      RenameNodeIdVerdict::RenameNodeIdReady
    ));
    assert!(cand.file_patches.is_empty());
  }

  #[test]
  fn patch_candidate_artifact_lands_in_shared_family() {
    let req = d8_req("examples/g.px", "sum", "total");
    let file_input = RenameNodeIdFileInput {
      path: "examples/g.px",
      content: D8_GRAPH_SRC,
    };
    let cand = compute_rename_node_id_patch_candidate(&req, &file_input);
    let art = build_rename_node_id_patch_candidate_artifact(&cand, 1_700_000_000_000, None);
    assert_eq!(
      art["artifact_family"].as_str(),
      Some("coding.generated-patch-candidate"),
      "MUST share family with rename-symbol / add-test-stub for cockpit pivot"
    );
    assert_eq!(
      art["source_surface"].as_str(),
      Some("code-transform.rename-node-id")
    );
    assert_eq!(art["payload"]["transform"].as_str(), Some("rename-node-id"));
    assert_eq!(
      art["payload"]["verdict"].as_str(),
      Some("rename-node-id-ready")
    );
    assert_eq!(art["payload"]["language"].as_str(), Some("pnix"));
    assert!(art["id"]
      .as_str()
      .unwrap()
      .starts_with("generated-patch.rename-node-id."));
  }

  #[test]
  fn patch_candidate_artifact_id_replay_stable_across_stored_at_ms() {
    let req = d8_req("examples/g.px", "sum", "total");
    let file_input = RenameNodeIdFileInput {
      path: "examples/g.px",
      content: D8_GRAPH_SRC,
    };
    let cand = compute_rename_node_id_patch_candidate(&req, &file_input);
    let a = build_rename_node_id_patch_candidate_artifact(&cand, 0, None);
    let b = build_rename_node_id_patch_candidate_artifact(&cand, 999_999, None);
    assert_eq!(a["id"], b["id"]);
    assert_ne!(a["stored_at_ms"], b["stored_at_ms"]);
  }

  #[test]
  fn patch_candidate_artifact_held_carries_held_kind_and_reason() {
    let req = d8_req("examples/g.px", "no_such_node", "y");
    let file_input = RenameNodeIdFileInput {
      path: "examples/g.px",
      content: D8_GRAPH_SRC,
    };
    let cand = compute_rename_node_id_patch_candidate(&req, &file_input);
    let art = build_rename_node_id_patch_candidate_artifact(&cand, 0, None);
    let payload = &art["payload"];
    assert_eq!(payload["verdict"].as_str(), Some("rename-node-id-held"));
    assert_eq!(payload["held_kind"].as_str(), Some("node-not-found"));
    assert!(payload["reason"].is_string());
    assert_eq!(payload["file_patches"].as_array().unwrap().len(), 0);
  }

  // ─── compute_remove_node_id_patch_candidate (D-12) ─────────────

  const D12_GRAPH_SRC: &str = r#"{
  externs = [ { name = "builtins.add"; } ];
  nodes = [
    { name = "first"; uses = "builtins.add"; }
    { name = "orphan"; uses = "builtins.add"; }
  ];
  edges = [
    { from = { input = "a"; }; to = { node = "first"; port = "lhs"; }; }
  ];
}
"#;

  fn d12_req(target: &str, name: &str) -> RemoveNodeIdRequest {
    RemoveNodeIdRequest {
      target_path: target.to_string(),
      name: name.to_string(),
      cascade: false,
    }
  }

  #[test]
  fn classify_remove_node_id_ready_for_well_formed_request() {
    assert!(matches!(
      classify_remove_node_id(&d12_req("examples/g.px", "orphan")),
      RemoveNodeIdVerdict::RemoveNodeIdReady
    ));
  }

  #[test]
  fn classify_remove_node_id_ladder_catches_request_shape_problems() {
    for (req, expected_kind) in [
      (
        d12_req("examples/g.px", ""),
        RemoveNodeIdHeldKind::EmptyName,
      ),
      (
        d12_req("examples/g.px", "1abc"),
        RemoveNodeIdHeldKind::InvalidIdentifier,
      ),
      (d12_req("", "x"), RemoveNodeIdHeldKind::EmptyTargetPath),
      (
        d12_req("../escape.px", "x"),
        RemoveNodeIdHeldKind::TargetPathOutOfProject,
      ),
    ] {
      match classify_remove_node_id(&req) {
        RemoveNodeIdVerdict::RemoveNodeIdHeld { held_kind, .. } => {
          assert_eq!(held_kind, expected_kind, "for req: {req:?}");
        }
        other => panic!("expected Held({expected_kind:?}), got {other:?}"),
      }
    }
  }

  #[test]
  fn compute_remove_node_id_orphan_succeeds_with_patch() {
    let cand = compute_remove_node_id_patch_candidate(
      &d12_req("examples/g.px", "orphan"),
      &RemoveNodeIdFileInput {
        path: "examples/g.px",
        content: D12_GRAPH_SRC,
      },
    );
    assert!(matches!(
      cand.verdict,
      RemoveNodeIdVerdict::RemoveNodeIdReady
    ));
    assert_eq!(cand.file_patches.len(), 1);
    let fp = &cand.file_patches[0];
    assert!(fp.after_content.contains("name = \"first\""));
    assert!(!fp.after_content.contains("name = \"orphan\""));
    assert_ne!(fp.before_sha256, fp.after_sha256);
    assert!(cand.unified_diff.contains("--- a/examples/g.px"));
    assert!(cand.edges_ref_count.is_none());
  }

  #[test]
  fn compute_remove_node_id_strict_refuses_with_ref_count() {
    // `first` has 1 edge ref → carrier returns Held with
    // edges_ref_count=1.
    let cand = compute_remove_node_id_patch_candidate(
      &d12_req("examples/g.px", "first"),
      &RemoveNodeIdFileInput {
        path: "examples/g.px",
        content: D12_GRAPH_SRC,
      },
    );
    match &cand.verdict {
      RemoveNodeIdVerdict::RemoveNodeIdHeld { held_kind, .. } => {
        assert_eq!(*held_kind, RemoveNodeIdHeldKind::StillReferencedByEdges);
      }
      other => panic!("expected Held(StillReferencedByEdges), got {other:?}"),
    }
    assert_eq!(cand.edges_ref_count, Some(1));
    assert!(cand.file_patches.is_empty());
  }

  #[test]
  fn compute_remove_node_id_node_not_found_downgrades_to_held() {
    let cand = compute_remove_node_id_patch_candidate(
      &d12_req("examples/g.px", "no_such_node"),
      &RemoveNodeIdFileInput {
        path: "examples/g.px",
        content: D12_GRAPH_SRC,
      },
    );
    match &cand.verdict {
      RemoveNodeIdVerdict::RemoveNodeIdHeld { held_kind, .. } => {
        assert_eq!(*held_kind, RemoveNodeIdHeldKind::NodeNotFound);
      }
      other => panic!("expected Held(NodeNotFound), got {other:?}"),
    }
    assert!(cand.edges_ref_count.is_none());
  }

  #[test]
  fn remove_node_id_artifact_lands_in_shared_family() {
    let cand = compute_remove_node_id_patch_candidate(
      &d12_req("examples/g.px", "orphan"),
      &RemoveNodeIdFileInput {
        path: "examples/g.px",
        content: D12_GRAPH_SRC,
      },
    );
    let art = build_remove_node_id_patch_candidate_artifact(&cand, 1_700_000_000_000, None);
    assert_eq!(
      art["artifact_family"].as_str(),
      Some("coding.generated-patch-candidate")
    );
    assert_eq!(
      art["source_surface"].as_str(),
      Some("code-transform.remove-node-id")
    );
    assert_eq!(art["payload"]["transform"].as_str(), Some("remove-node-id"));
    assert_eq!(
      art["payload"]["verdict"].as_str(),
      Some("remove-node-id-ready")
    );
    assert_eq!(art["payload"]["language"].as_str(), Some("pnix"));
    assert!(art["id"]
      .as_str()
      .unwrap()
      .starts_with("generated-patch.remove-node-id."));
  }

  #[test]
  fn remove_node_id_artifact_held_carries_edges_ref_count() {
    let cand = compute_remove_node_id_patch_candidate(
      &d12_req("examples/g.px", "first"),
      &RemoveNodeIdFileInput {
        path: "examples/g.px",
        content: D12_GRAPH_SRC,
      },
    );
    let art = build_remove_node_id_patch_candidate_artifact(&cand, 0, None);
    let payload = &art["payload"];
    assert_eq!(payload["verdict"].as_str(), Some("remove-node-id-held"));
    assert_eq!(
      payload["held_kind"].as_str(),
      Some("still-referenced-by-edges")
    );
    assert_eq!(payload["edges_ref_count"].as_u64(), Some(1));
  }

  #[test]
  fn remove_node_id_artifact_id_replay_stable_across_stored_at_ms() {
    let cand = compute_remove_node_id_patch_candidate(
      &d12_req("examples/g.px", "orphan"),
      &RemoveNodeIdFileInput {
        path: "examples/g.px",
        content: D12_GRAPH_SRC,
      },
    );
    let a = build_remove_node_id_patch_candidate_artifact(&cand, 0, None);
    let b = build_remove_node_id_patch_candidate_artifact(&cand, 999_999, None);
    assert_eq!(a["id"], b["id"]);
    assert_ne!(a["stored_at_ms"], b["stored_at_ms"]);
  }

  // ─── remove-pnix-edge transform (D-18) ─────────────────────────

  const D18_EDGE_SRC: &str = r#"{
  externs = [ { name = "builtins.add"; } ];
  nodes = [
    { name = "sum"; uses = "builtins.add"; }
  ];
  edges = [
    { from = { input = "a"; }; to = { node = "sum"; port = "lhs"; }; }
    { from = { input = "b"; }; to = { node = "sum"; port = "rhs"; }; }
    { from = { node = "sum"; port = "out"; }; to = { node = "out"; port = "result"; }; }
  ];
}
"#;

  #[test]
  fn remove_edge_entry_by_from_to_drops_matching_entry() {
    let after = remove_edge_entry_by_from_to_in_source(
      D18_EDGE_SRC,
      &OwnedEdgeEndpoint::Input {
        name: "a".to_string(),
      },
      &OwnedEdgeEndpoint::Node {
        name: "sum".to_string(),
        port: Some("lhs".to_string()),
      },
    )
    .expect("remove ok");
    // The matching edge gone.
    assert!(!after.contains("input = \"a\""));
    // Others preserved.
    assert!(after.contains("input = \"b\""));
    assert!(after.contains("port = \"out\""));
    // Reparse — still graph-mode.
    let g = parse_pnix_graph_mode(&after).expect("reparse");
    assert!(g.looks_like_graph_mode());
  }

  #[test]
  fn remove_edge_entry_returns_edge_not_found_for_unknown_pair() {
    let r = remove_edge_entry_by_from_to_in_source(
      D18_EDGE_SRC,
      &OwnedEdgeEndpoint::Input {
        name: "no_such".to_string(),
      },
      &OwnedEdgeEndpoint::Node {
        name: "sum".to_string(),
        port: Some("lhs".to_string()),
      },
    );
    assert_eq!(r.unwrap_err(), RemoveEdgeError::EdgeNotFound);
  }

  #[test]
  fn remove_edge_entry_returns_no_edges_section_when_absent() {
    let src = "{\n  nodes = [];\n}\n";
    let r = remove_edge_entry_by_from_to_in_source(
      src,
      &OwnedEdgeEndpoint::Input {
        name: "a".to_string(),
      },
      &OwnedEdgeEndpoint::Node {
        name: "x".to_string(),
        port: None,
      },
    );
    assert_eq!(r.unwrap_err(), RemoveEdgeError::NoEdgesSection);
  }

  #[test]
  fn compute_remove_pnix_edge_ready_emits_patch_and_count() {
    let req = RemovePnixEdgeRequest {
      target_path: "examples/g.px".to_string(),
      from: OwnedEdgeEndpoint::Input {
        name: "a".to_string(),
      },
      to: OwnedEdgeEndpoint::Node {
        name: "sum".to_string(),
        port: Some("lhs".to_string()),
      },
    };
    let cand = compute_remove_pnix_edge_patch_candidate(
      &req,
      &AddPnixGraphFileInput {
        path: "examples/g.px",
        content: D18_EDGE_SRC,
      },
    );
    assert!(matches!(
      cand.verdict,
      RemovePnixEdgeVerdict::RemovePnixEdgeReady
    ));
    assert_eq!(cand.file_patches.len(), 1);
    assert_eq!(cand.edges_removed, Some(1));
    assert!(!cand.file_patches[0].after_content.contains("input = \"a\""));
    assert_ne!(
      cand.file_patches[0].before_sha256,
      cand.file_patches[0].after_sha256
    );
  }

  #[test]
  fn compute_remove_pnix_edge_held_on_edge_not_found() {
    let req = RemovePnixEdgeRequest {
      target_path: "examples/g.px".to_string(),
      from: OwnedEdgeEndpoint::Input {
        name: "no_such".to_string(),
      },
      to: OwnedEdgeEndpoint::Node {
        name: "sum".to_string(),
        port: None,
      },
    };
    let cand = compute_remove_pnix_edge_patch_candidate(
      &req,
      &AddPnixGraphFileInput {
        path: "examples/g.px",
        content: D18_EDGE_SRC,
      },
    );
    match &cand.verdict {
      RemovePnixEdgeVerdict::RemovePnixEdgeHeld { held_kind, .. } => {
        assert_eq!(*held_kind, RemovePnixEdgeHeldKind::EdgeNotFound);
      }
      other => panic!("expected EdgeNotFound, got {other:?}"),
    }
    assert!(cand.file_patches.is_empty());
    assert!(cand.edges_removed.is_none());
  }

  #[test]
  fn remove_pnix_edge_artifact_lands_in_shared_family_with_edges_removed_count() {
    let req = RemovePnixEdgeRequest {
      target_path: "examples/g.px".to_string(),
      from: OwnedEdgeEndpoint::Input {
        name: "b".to_string(),
      },
      to: OwnedEdgeEndpoint::Node {
        name: "sum".to_string(),
        port: Some("rhs".to_string()),
      },
    };
    let cand = compute_remove_pnix_edge_patch_candidate(
      &req,
      &AddPnixGraphFileInput {
        path: "examples/g.px",
        content: D18_EDGE_SRC,
      },
    );
    let art = build_remove_pnix_edge_patch_candidate_artifact(&cand, 0, None);
    assert_eq!(
      art["artifact_family"].as_str(),
      Some("coding.generated-patch-candidate")
    );
    assert_eq!(
      art["source_surface"].as_str(),
      Some("code-transform.remove-pnix-edge")
    );
    assert_eq!(
      art["payload"]["transform"].as_str(),
      Some("remove-pnix-edge")
    );
    assert_eq!(art["payload"]["edges_removed"].as_u64(), Some(1));
    // Typed endpoints survive serde round-trip in payload.
    assert_eq!(art["payload"]["from"]["kind"].as_str(), Some("input"));
    assert_eq!(art["payload"]["from"]["name"].as_str(), Some("b"));
    assert_eq!(art["payload"]["to"]["kind"].as_str(), Some("node"));
    assert_eq!(art["payload"]["to"]["port"].as_str(), Some("rhs"));
  }

  #[test]
  fn owned_edge_endpoint_render_matches_borrowed_variant() {
    // Substrate-share: OwnedEdgeEndpoint::render_inner produces
    // byte-equal output to EdgeEndpoint::render_inner for the
    // same logical endpoint.
    let owned_input = OwnedEdgeEndpoint::Input {
      name: "a".to_string(),
    };
    let borrowed_input = EdgeEndpoint::Input { name: "a" };
    assert_eq!(owned_input.render_inner(), borrowed_input.render_inner());

    let owned_node = OwnedEdgeEndpoint::Node {
      name: "x".to_string(),
      port: Some("p".to_string()),
    };
    let borrowed_node = EdgeEndpoint::Node {
      name: "x",
      port: Some("p"),
    };
    assert_eq!(owned_node.render_inner(), borrowed_node.render_inner());

    let owned_node_no_port = OwnedEdgeEndpoint::Node {
      name: "y".to_string(),
      port: None,
    };
    let borrowed_node_no_port = EdgeEndpoint::Node {
      name: "y",
      port: None,
    };
    assert_eq!(
      owned_node_no_port.render_inner(),
      borrowed_node_no_port.render_inner()
    );
  }

  // ─── typed ExternSpec / NodeSpec builders (D-17) ───────────────

  #[test]
  fn extern_spec_renders_minimal_entry_body() {
    let spec = ExternSpec { name: "py.add" };
    assert_eq!(build_extern_entry_text(&spec), r#"name = "py.add""#);
  }

  #[test]
  fn node_spec_renders_canonical_entry_body() {
    let spec = NodeSpec {
      name: "is_target",
      uses: "builtins.eq",
      gate: None,
    };
    assert_eq!(
      build_node_entry_text(&spec),
      r#"name = "is_target"; uses = "builtins.eq""#
    );
  }

  #[test]
  fn node_spec_emits_gate_true_when_some_true() {
    let spec = NodeSpec {
      name: "g",
      uses: "u",
      gate: Some(true),
    };
    assert_eq!(
      build_node_entry_text(&spec),
      r#"name = "g"; uses = "u"; gate = true"#
    );
  }

  #[test]
  fn node_spec_emits_gate_false_when_some_false() {
    let spec = NodeSpec {
      name: "g",
      uses: "u",
      gate: Some(false),
    };
    assert_eq!(
      build_node_entry_text(&spec),
      r#"name = "g"; uses = "u"; gate = false"#
    );
  }

  #[test]
  fn typed_builders_compose_with_add_extern_and_add_node_helpers() {
    // End-to-end ergonomics: caller builds typed specs, hands the
    // rendered strings to the helper-tier append functions. Same
    // result as hand-writing the entry text.
    let src = r#"{
  externs = [];
  nodes = [];
}
"#;
    let after_extern = add_extern_entry_to_source(
      src,
      &build_extern_entry_text(&ExternSpec { name: "py.add" }),
    )
    .expect("extern");
    let after_node = add_node_entry_to_source(
      &after_extern,
      &build_node_entry_text(&NodeSpec {
        name: "adder",
        uses: "py.add",
        gate: None,
      }),
    )
    .expect("node");
    let g = parse_pnix_graph_mode(&after_node).expect("reparse");
    assert!(g.looks_like_graph_mode());
    let externs_body = &after_node[g.section("externs").unwrap().body_inner_start_byte
      ..g.section("externs").unwrap().body_inner_end_byte];
    let nodes_body = &after_node[g.section("nodes").unwrap().body_inner_start_byte
      ..g.section("nodes").unwrap().body_inner_end_byte];
    assert!(externs_body.contains("\"py.add\""));
    assert!(nodes_body.contains("\"adder\""));
    assert!(nodes_body.contains("\"py.add\""));
  }

  #[test]
  fn typed_builders_compose_with_dispatcher_request_layer() {
    // The dispatcher's `entry_text: String` field is what
    // `build_*_entry_text` produces — typed builder + dispatcher
    // request slot together. Same result as raw entry_text.
    let typed_extern_text = build_extern_entry_text(&ExternSpec { name: "py.sub" });
    let raw_extern_text = r#"name = "py.sub""#.to_string();
    assert_eq!(typed_extern_text, raw_extern_text);

    let typed_node_text = build_node_entry_text(&NodeSpec {
      name: "subtractor",
      uses: "py.sub",
      gate: Some(true),
    });
    let raw_node_text = r#"name = "subtractor"; uses = "py.sub"; gate = true"#.to_string();
    assert_eq!(typed_node_text, raw_node_text);
  }

  #[test]
  fn typed_builders_match_existing_edge_endpoint_pattern() {
    // Substrate-share at the builder layer: three typed builders
    // (Extern / Node / Edge) all return `String` ready for the
    // dispatcher's `entry_text` field. No `&str` lifetimes leak
    // into the dispatcher.
    let e: String = build_extern_entry_text(&ExternSpec { name: "x" });
    let n: String = build_node_entry_text(&NodeSpec {
      name: "x",
      uses: "y",
      gate: None,
    });
    let g: String = build_edge_entry_text(
      &EdgeEndpoint::Input { name: "a" },
      &EdgeEndpoint::Node {
        name: "x",
        port: None,
      },
    );
    // All three are non-empty plain strings — same return type, so
    // the dispatcher caller treats them uniformly.
    assert!(!e.is_empty());
    assert!(!n.is_empty());
    assert!(!g.is_empty());
  }

  // ─── remove-node-id cascade variant (D-14) ─────────────────────

  const D14_CASCADE_SRC: &str = r#"{
  externs = [ { name = "builtins.add"; } ];
  nodes = [
    { name = "keep"; uses = "builtins.add"; }
    { name = "remove_me"; uses = "builtins.add"; }
  ];
  edges = [
    { from = { input = "a"; }; to = { node = "keep"; port = "lhs"; }; }
    { from = { input = "b"; }; to = { node = "remove_me"; port = "lhs"; }; }
    { from = { input = "c"; }; to = { node = "remove_me"; port = "rhs"; }; }
    { from = { node = "remove_me"; port = "out"; }; to = { node = "keep"; port = "rhs"; }; }
  ];
}
"#;

  fn d14_req(target: &str, name: &str, cascade: bool) -> RemoveNodeIdRequest {
    RemoveNodeIdRequest {
      target_path: target.to_string(),
      name: name.to_string(),
      cascade,
    }
  }

  #[test]
  fn remove_node_entry_by_name_cascade_drops_node_and_referencing_edges() {
    let outcome =
      remove_node_entry_by_name_cascade(D14_CASCADE_SRC, "remove_me").expect("cascade ok");
    assert_eq!(outcome.edges_removed, 3, "3 edge refs to remove_me");
    let after = &outcome.new_source;
    assert!(!after.contains("\"remove_me\""));
    // The unrelated `keep` node + its edge survive.
    assert!(after.contains("\"keep\""));
    assert!(after.contains("input = \"a\""));
    // No `node = "remove_me"` anywhere.
    assert!(!after.contains("node = \"remove_me\""));
    // Reparse — graph still valid.
    let g = parse_pnix_graph_mode(after).expect("reparse");
    assert!(g.looks_like_graph_mode());
  }

  #[test]
  fn remove_node_entry_by_name_cascade_zero_refs_returns_zero_count() {
    // Node `keep` has no `node = "keep"` refs (the existing edges
    // mention it via `to`, but only `remove_me` 's `from` references
    // a node — `keep` is only `to=node` and `from=node` once. wait:
    // re-read fixture). Use a simpler fixture instead.
    let src = r#"{
  nodes = [
    { name = "orphan"; uses = "x"; }
  ];
  edges = [
    { from = { input = "a"; }; to = { node = "other"; port = "p"; }; }
  ];
}
"#;
    let outcome = remove_node_entry_by_name_cascade(src, "orphan").expect("ok");
    assert_eq!(outcome.edges_removed, 0);
    assert!(!outcome.new_source.contains("\"orphan\""));
  }

  #[test]
  fn remove_node_entry_by_name_cascade_returns_node_not_found() {
    assert_eq!(
      remove_node_entry_by_name_cascade(D14_CASCADE_SRC, "no_such")
        .map(|_| ())
        .unwrap_err(),
      RemoveNodeIdError::NodeNotFound
    );
  }

  #[test]
  fn compute_remove_node_id_cascade_mode_drops_edges_and_emits_count() {
    let req = d14_req("examples/g.px", "remove_me", true);
    let file_input = RemoveNodeIdFileInput {
      path: "examples/g.px",
      content: D14_CASCADE_SRC,
    };
    let cand = compute_remove_node_id_patch_candidate(&req, &file_input);
    assert!(matches!(
      cand.verdict,
      RemoveNodeIdVerdict::RemoveNodeIdReady
    ));
    assert_eq!(cand.cascade_edges_removed, Some(3));
    assert_eq!(cand.edges_ref_count, None);
    assert_eq!(cand.file_patches.len(), 1);
    let after = &cand.file_patches[0].after_content;
    assert!(!after.contains("\"remove_me\""));
    assert!(after.contains("\"keep\""));
  }

  #[test]
  fn compute_remove_node_id_strict_mode_still_refuses_with_ref_count() {
    // Same fixture, strict mode → Held, edges_ref_count exposed,
    // cascade_edges_removed=None.
    let req = d14_req("examples/g.px", "remove_me", false);
    let file_input = RemoveNodeIdFileInput {
      path: "examples/g.px",
      content: D14_CASCADE_SRC,
    };
    let cand = compute_remove_node_id_patch_candidate(&req, &file_input);
    match &cand.verdict {
      RemoveNodeIdVerdict::RemoveNodeIdHeld { held_kind, .. } => {
        assert_eq!(*held_kind, RemoveNodeIdHeldKind::StillReferencedByEdges);
      }
      other => panic!("expected Held, got {other:?}"),
    }
    assert_eq!(cand.edges_ref_count, Some(3));
    assert_eq!(cand.cascade_edges_removed, None);
  }

  #[test]
  fn remove_node_id_cascade_artifact_payload_carries_cascade_signals() {
    let req = d14_req("examples/g.px", "remove_me", true);
    let cand = compute_remove_node_id_patch_candidate(
      &req,
      &RemoveNodeIdFileInput {
        path: "examples/g.px",
        content: D14_CASCADE_SRC,
      },
    );
    let art = build_remove_node_id_patch_candidate_artifact(&cand, 0, None);
    let payload = &art["payload"];
    assert_eq!(payload["cascade"].as_bool(), Some(true));
    assert_eq!(payload["cascade_edges_removed"].as_u64(), Some(3));
    // No edges_ref_count on cascade success path.
    assert!(payload.get("edges_ref_count").is_none());
  }

  #[test]
  fn remove_node_id_strict_artifact_payload_carries_cascade_false_flag() {
    // Strict mode also surfaces `cascade: false` so the cockpit can
    // distinguish strict-refuse from cascade-never-attempted.
    let req = d14_req("examples/g.px", "remove_me", false);
    let cand = compute_remove_node_id_patch_candidate(
      &req,
      &RemoveNodeIdFileInput {
        path: "examples/g.px",
        content: D14_CASCADE_SRC,
      },
    );
    let art = build_remove_node_id_patch_candidate_artifact(&cand, 0, None);
    let payload = &art["payload"];
    assert_eq!(payload["cascade"].as_bool(), Some(false));
    assert_eq!(payload["edges_ref_count"].as_u64(), Some(3));
    // No cascade_edges_removed when cascade=false.
    assert!(payload.get("cascade_edges_removed").is_none());
  }

  #[test]
  fn remove_node_id_cascade_artifact_id_differs_from_strict() {
    let strict = compute_remove_node_id_patch_candidate(
      &d14_req("examples/g.px", "remove_me", false),
      &RemoveNodeIdFileInput {
        path: "examples/g.px",
        content: D14_CASCADE_SRC,
      },
    );
    let cascade = compute_remove_node_id_patch_candidate(
      &d14_req("examples/g.px", "remove_me", true),
      &RemoveNodeIdFileInput {
        path: "examples/g.px",
        content: D14_CASCADE_SRC,
      },
    );
    let strict_art = build_remove_node_id_patch_candidate_artifact(&strict, 0, None);
    let cascade_art = build_remove_node_id_patch_candidate_artifact(&cascade, 0, None);
    assert_ne!(
      strict_art["id"], cascade_art["id"],
      "strict and cascade dispatches on same source must produce distinct artifact ids"
    );
  }

  #[test]
  fn remove_node_id_request_deserializes_with_default_cascade_false() {
    // Backward compat: existing callers that omit `cascade` get
    // the strict default.
    let json = serde_json::json!({
      "target_path": "examples/g.px",
      "name": "x"
    });
    let req: RemoveNodeIdRequest = serde_json::from_value(json).expect("deserialize");
    assert_eq!(req.cascade, false);
  }

  // ─── add-pnix-* dispatcher integration carriers (D-13) ─────────

  const D13_GRAPH_SRC: &str = r#"{
  externs = [
    { name = "builtins.add"; }
  ];
  nodes = [
    { name = "sum"; uses = "builtins.add"; }
  ];
  edges = [
    { from = { input = "a"; }; to = { node = "sum"; port = "lhs"; }; }
  ];
}
"#;

  #[test]
  fn compute_add_pnix_extern_ready_emits_patch() {
    let req = AddPnixExternRequest {
      target_path: "examples/g.px".to_string(),
      entry_text: r#"name = "builtins.mul""#.to_string(),
    };
    let cand = compute_add_pnix_extern_patch_candidate(
      &req,
      &AddPnixGraphFileInput {
        path: "examples/g.px",
        content: D13_GRAPH_SRC,
      },
    );
    assert!(matches!(
      cand.verdict,
      AddPnixExternVerdict::AddPnixExternReady
    ));
    assert_eq!(cand.file_patches.len(), 1);
    assert!(cand.file_patches[0].after_content.contains("builtins.mul"));
    assert!(cand.file_patches[0].after_content.contains("builtins.add"));
    assert_ne!(
      cand.file_patches[0].before_sha256,
      cand.file_patches[0].after_sha256
    );
  }

  #[test]
  fn compute_add_pnix_node_ready_emits_patch() {
    let req = AddPnixNodeRequest {
      target_path: "examples/g.px".to_string(),
      entry_text: r#"name = "second"; uses = "builtins.add""#.to_string(),
    };
    let cand = compute_add_pnix_node_patch_candidate(
      &req,
      &AddPnixGraphFileInput {
        path: "examples/g.px",
        content: D13_GRAPH_SRC,
      },
    );
    assert!(matches!(cand.verdict, AddPnixNodeVerdict::AddPnixNodeReady));
    assert_eq!(cand.file_patches.len(), 1);
    assert!(cand.file_patches[0].after_content.contains("\"second\""));
    assert!(cand.file_patches[0].after_content.contains("\"sum\""));
  }

  #[test]
  fn compute_add_pnix_edge_ready_emits_patch_with_typed_builder() {
    // Caller uses the D-3 `EdgeEndpoint` typed builder to construct
    // entry_text — the dispatcher pipeline accepts whatever
    // entry_text the helper produced.
    let entry = build_edge_entry_text(
      &EdgeEndpoint::Input { name: "b" },
      &EdgeEndpoint::Node {
        name: "sum",
        port: Some("rhs"),
      },
    );
    let req = AddPnixEdgeRequest {
      target_path: "examples/g.px".to_string(),
      entry_text: entry,
    };
    let cand = compute_add_pnix_edge_patch_candidate(
      &req,
      &AddPnixGraphFileInput {
        path: "examples/g.px",
        content: D13_GRAPH_SRC,
      },
    );
    assert!(matches!(cand.verdict, AddPnixEdgeVerdict::AddPnixEdgeReady));
    let after = &cand.file_patches[0].after_content;
    assert!(after.contains("input = \"a\""));
    assert!(after.contains("input = \"b\""));
    assert!(after.contains("port = \"rhs\""));
  }

  #[test]
  fn compute_add_pnix_extern_held_on_no_externs_section() {
    let src = "{\n  nodes = [];\n}\n";
    let req = AddPnixExternRequest {
      target_path: "g.px".to_string(),
      entry_text: r#"name = "x""#.to_string(),
    };
    let cand = compute_add_pnix_extern_patch_candidate(
      &req,
      &AddPnixGraphFileInput {
        path: "g.px",
        content: src,
      },
    );
    match cand.verdict {
      AddPnixExternVerdict::AddPnixExternHeld { held_kind, .. } => {
        assert_eq!(held_kind, AddPnixGraphHeldKind::NoSectionInGraph);
      }
      other => panic!("expected NoSectionInGraph, got {other:?}"),
    }
    assert!(cand.file_patches.is_empty());
  }

  #[test]
  fn compute_add_pnix_node_held_on_empty_entry_text() {
    let req = AddPnixNodeRequest {
      target_path: "examples/g.px".to_string(),
      entry_text: "".to_string(),
    };
    let cand = compute_add_pnix_node_patch_candidate(
      &req,
      &AddPnixGraphFileInput {
        path: "examples/g.px",
        content: D13_GRAPH_SRC,
      },
    );
    match cand.verdict {
      AddPnixNodeVerdict::AddPnixNodeHeld { held_kind, .. } => {
        assert_eq!(held_kind, AddPnixGraphHeldKind::EmptyEntryText);
      }
      other => panic!("expected EmptyEntryText, got {other:?}"),
    }
  }

  #[test]
  fn add_pnix_extern_artifact_shape_and_replay_stability() {
    let req = AddPnixExternRequest {
      target_path: "examples/g.px".to_string(),
      entry_text: r#"name = "x""#.to_string(),
    };
    let cand = compute_add_pnix_extern_patch_candidate(
      &req,
      &AddPnixGraphFileInput {
        path: "examples/g.px",
        content: D13_GRAPH_SRC,
      },
    );
    let art = build_add_pnix_extern_patch_candidate_artifact(&cand, 0, None);
    assert_eq!(
      art["artifact_family"].as_str(),
      Some("coding.generated-patch-candidate")
    );
    assert_eq!(
      art["source_surface"].as_str(),
      Some("code-transform.add-pnix-extern")
    );
    assert_eq!(
      art["payload"]["transform"].as_str(),
      Some("add-pnix-extern")
    );
    assert_eq!(
      art["payload"]["verdict"].as_str(),
      Some("add-pnix-extern-ready")
    );
    assert_eq!(art["payload"]["language"].as_str(), Some("pnix"));
    assert!(art["id"]
      .as_str()
      .unwrap()
      .starts_with("generated-patch.add-pnix-extern."));
    let a = build_add_pnix_extern_patch_candidate_artifact(&cand, 0, None);
    let b = build_add_pnix_extern_patch_candidate_artifact(&cand, 999, None);
    assert_eq!(a["id"], b["id"]);
  }

  #[test]
  fn add_pnix_node_artifact_distinct_source_surface() {
    let req = AddPnixNodeRequest {
      target_path: "examples/g.px".to_string(),
      entry_text: r#"name = "n"; uses = "u""#.to_string(),
    };
    let cand = compute_add_pnix_node_patch_candidate(
      &req,
      &AddPnixGraphFileInput {
        path: "examples/g.px",
        content: D13_GRAPH_SRC,
      },
    );
    let art = build_add_pnix_node_patch_candidate_artifact(&cand, 0, None);
    assert_eq!(
      art["source_surface"].as_str(),
      Some("code-transform.add-pnix-node")
    );
    assert_eq!(art["payload"]["transform"].as_str(), Some("add-pnix-node"));
  }

  #[test]
  fn add_pnix_edge_artifact_distinct_source_surface() {
    let req = AddPnixEdgeRequest {
      target_path: "examples/g.px".to_string(),
      entry_text: "from = { input = \"a\"; }; to = { node = \"sum\"; }".to_string(),
    };
    let cand = compute_add_pnix_edge_patch_candidate(
      &req,
      &AddPnixGraphFileInput {
        path: "examples/g.px",
        content: D13_GRAPH_SRC,
      },
    );
    let art = build_add_pnix_edge_patch_candidate_artifact(&cand, 0, None);
    assert_eq!(
      art["source_surface"].as_str(),
      Some("code-transform.add-pnix-edge")
    );
    assert_eq!(art["payload"]["transform"].as_str(), Some("add-pnix-edge"));
  }

  #[test]
  fn add_pnix_three_transforms_compose_in_single_pipeline() {
    // Authoring a fresh node + its hooking edge in two dispatcher
    // turns over the same file content.
    let extern_req = AddPnixExternRequest {
      target_path: "examples/g.px".to_string(),
      entry_text: r#"name = "builtins.mul""#.to_string(),
    };
    let extern_cand = compute_add_pnix_extern_patch_candidate(
      &extern_req,
      &AddPnixGraphFileInput {
        path: "examples/g.px",
        content: D13_GRAPH_SRC,
      },
    );
    let after_extern = extern_cand.file_patches[0].after_content.clone();
    let node_req = AddPnixNodeRequest {
      target_path: "examples/g.px".to_string(),
      entry_text: r#"name = "prod"; uses = "builtins.mul""#.to_string(),
    };
    let node_cand = compute_add_pnix_node_patch_candidate(
      &node_req,
      &AddPnixGraphFileInput {
        path: "examples/g.px",
        content: &after_extern,
      },
    );
    let after_node = node_cand.file_patches[0].after_content.clone();
    assert!(after_node.contains("builtins.mul"));
    assert!(after_node.contains("\"prod\""));
    assert!(after_node.contains("\"sum\""));
  }

  // ─── generic helper invariants ─────────────────────────────────

  #[test]
  fn add_entry_to_list_section_refuses_attrset_shape() {
    // `inputs` is `ValueShape::Attrset`, not `List`. The generic
    // helper must refuse to append to it rather than silently
    // mis-edit. (`inputs` uses a different syntactic shape — caller
    // would need a separate `add_input_binding_to_source`.)
    let edited = add_entry_to_list_section(NODES_GRAPH_SRC, "inputs", r#"x = "Num""#);
    assert!(
      edited.is_none(),
      "Attrset-shape section must not accept list-append"
    );
  }

  #[test]
  fn add_entry_to_list_section_refuses_unknown_section() {
    // A section name that isn't in `GRAPH_SECTIONS` returns None
    // (parser doesn't surface it, so the helper has nothing to
    // append to).
    let edited = add_entry_to_list_section(NODES_GRAPH_SRC, "fictional_section", r#"x = 1"#);
    assert!(edited.is_none());
  }
}
