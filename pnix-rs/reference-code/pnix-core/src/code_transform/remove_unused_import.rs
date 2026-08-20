//! Remove-unused-import deterministic code-transform host carrier.
//!
//! OWNER-LAW (2026-05-11): the second deterministic code-transform —
//! proves the rename-symbol canonical chain pattern (candidate →
//! review → apply → rollback-handle → rollback-receipt) generalizes
//! to other transform kinds. Mirrors
//! `stdlib/lib/gate/code-transform/remove-unused-import.px`.
//!
//! Unlike rename-symbol, this transform's `.px` law assumes the host
//! has *already* identified candidate unused imports via tree-sitter
//! / symbol resolver. The host attaches per-candidate flags
//! (`used_in_macro`, `behind_cfg`) so the `.px` law can Hold on
//! macro-bound / cfg-conditional imports without re-walking the
//! source.
//!
//! This first slice mirrors the `.px` classifier in Rust — the
//! file-content-aware detection walk (tree-sitter integration) is a
//! follow-up. With the classifier in place, the candidate / review /
//! apply / rollback chain wrappers can be added by the same builders
//! pattern as rename-symbol.

use serde::{Deserialize, Serialize};
use pnix_hash::{Digest, Sha256};

/// Scope qualifier mirroring the `.px` law: `single-file` (default,
/// safe), `tests-also` (operator opt-in to touch test files),
/// `crate-wide` (Held by default — too risky for an automated pass).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RemoveUnusedImportScope {
  SingleFile,
  TestsAlso,
  CrateWide,
}

impl RemoveUnusedImportScope {
  pub const ALL: &'static [Self] = &[Self::SingleFile, Self::TestsAlso, Self::CrateWide];

  pub fn as_str(self) -> &'static str {
    match self {
      Self::SingleFile => "single-file",
      Self::TestsAlso => "tests-also",
      Self::CrateWide => "crate-wide",
    }
  }
}

/// One of the documented Held / Rejected outcomes from the `.px`
/// owner law. Each variant maps 1:1 to a held_kind string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RemoveUnusedImportHeldKind {
  MissingTargetPath,
  TargetPathOutOfProject,
  LanguageNotSupported,
  MissingCandidates,
  /// One or more candidate imports has `used_in_macro=true` or
  /// `behind_cfg=true`. Deletion may break a build configuration
  /// not currently active.
  MacroBinding,
  /// Future variant for cfg-conditional imports separate from macro
  /// binding. Currently `MacroBinding` covers both per the `.px` law's
  /// `hasMacroOrCfg` predicate.
  CfgConditional,
  ScopeTooBroad,
  TestFileProtected,
}

impl RemoveUnusedImportHeldKind {
  pub const ALL: &'static [Self] = &[
    Self::MissingTargetPath,
    Self::TargetPathOutOfProject,
    Self::LanguageNotSupported,
    Self::MissingCandidates,
    Self::MacroBinding,
    Self::CfgConditional,
    Self::ScopeTooBroad,
    Self::TestFileProtected,
  ];
  pub fn as_str(self) -> &'static str {
    match self {
      Self::MissingTargetPath => "missing-target-path",
      Self::TargetPathOutOfProject => "target-path-out-of-project",
      Self::LanguageNotSupported => "language-not-supported",
      Self::MissingCandidates => "missing-candidates",
      Self::MacroBinding => "macro-binding",
      Self::CfgConditional => "cfg-conditional",
      Self::ScopeTooBroad => "scope-too-broad",
      Self::TestFileProtected => "test-file-protected",
    }
  }
}

pub const SUPPORTED_LANGUAGES: &[&str] = &["rust", "python", "typescript", "javascript", "go"];

/// A single candidate unused import as pre-identified by the host's
/// tree-sitter + symbol resolver. The host knows whether the import
/// is referenced by a macro expansion or behind a cfg attribute —
/// both flags trigger Held in the `.px` law to avoid breaking
/// non-active build configurations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnusedImportCandidate {
  /// The imported path / module name (e.g. `"std::collections::HashMap"`
  /// for Rust, `"os.path"` for Python).
  pub module: String,
  /// Set when symbol resolver finds the imported name referenced
  /// inside a macro body (not directly visible to the unused-import
  /// walk).
  #[serde(default)]
  pub used_in_macro: bool,
  /// Set when the import is behind a `#[cfg(...)]` (Rust) or
  /// equivalent conditional compilation guard. The walk only sees
  /// the active configuration.
  #[serde(default)]
  pub behind_cfg: bool,
}

/// A remove-unused-import request — the input the `.px` owner law's
/// classify function inspects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoveUnusedImportRequest {
  pub target_path: String,
  pub language: String,
  pub candidate_imports: Vec<UnusedImportCandidate>,
  pub scope: RemoveUnusedImportScope,
}

/// Verdict from [`classify_remove_unused_import`], mirroring the `.px`
/// owner law's three outcomes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "verdict")]
pub enum RemoveUnusedImportVerdict {
  RemoveUnusedImportReady,
  RemoveUnusedImportHeld {
    held_kind: RemoveUnusedImportHeldKind,
    reason: String,
  },
  RemoveUnusedImportRejected {
    held_kind: RemoveUnusedImportHeldKind,
    reason: String,
  },
}

fn is_supported_language(lang: &str) -> bool {
  matches!(lang, "rust" | "python" | "typescript" | "javascript" | "go")
}

fn is_path_in_project(p: &str) -> bool {
  !p.is_empty() && !p.contains("..") && !p.contains('\u{0}')
}

/// Mirrors the `.px` `isTestPath` predicate — file paths under a
/// `tests/` or `_test/` directory, `.spec.*` files, or `_test.<ext>`
/// files. Used to gate the `test-file-protected` Held.
fn is_test_path(p: &str) -> bool {
  // Directory segment marker: /tests/, /_test/, _tests/, _test/ etc.
  let p_lower = p; // case-sensitive — match the `.px` regex
  if p_lower.contains("/tests/")
    || p_lower.contains("/test/")
    || p_lower.contains("/_tests/")
    || p_lower.contains("/_test/")
    || p_lower.contains("_tests_")
    || p_lower.contains("_test_")
  {
    return true;
  }
  // .spec. files
  if p_lower.contains(".spec.") {
    return true;
  }
  // _test.<ext> suffix
  if let Some(idx) = p_lower.rfind("_test.") {
    let after = &p_lower[idx + 6..];
    if !after.is_empty() && after.chars().all(|c| c.is_ascii_alphabetic()) {
      return true;
    }
  }
  false
}

fn any_macro_or_cfg(candidates: &[UnusedImportCandidate]) -> bool {
  candidates.iter().any(|c| c.used_in_macro || c.behind_cfg)
}

/// Pure classifier — Rust mirror of the `.px` owner law's
/// `classify`. Returns the same verdict the law would emit for the
/// same request.
///
/// OWNER-LAW (2026-05-11): MUST stay in lockstep with
/// `stdlib/lib/gate/code-transform/remove-unused-import.px`. Adding a
/// held_kind in one side requires adding it in the other so the
/// (future) code-transform sync guard stays green.
///
/// The order matches the `.px` `if .. else if ..` ladder so the same
/// input produces the same verdict.
pub fn classify_remove_unused_import(req: &RemoveUnusedImportRequest) -> RemoveUnusedImportVerdict {
  if req.target_path.is_empty() {
    return RemoveUnusedImportVerdict::RemoveUnusedImportHeld {
      held_kind: RemoveUnusedImportHeldKind::MissingTargetPath,
      reason: "target_path required".to_string(),
    };
  }
  if !is_path_in_project(&req.target_path) {
    return RemoveUnusedImportVerdict::RemoveUnusedImportHeld {
      held_kind: RemoveUnusedImportHeldKind::TargetPathOutOfProject,
      reason: "target_path must be within project root".to_string(),
    };
  }
  if !is_supported_language(&req.language) {
    return RemoveUnusedImportVerdict::RemoveUnusedImportHeld {
      held_kind: RemoveUnusedImportHeldKind::LanguageNotSupported,
      reason: format!(
        "language `{}` not supported by remove-unused-import owner",
        req.language
      ),
    };
  }
  if req.candidate_imports.is_empty() {
    return RemoveUnusedImportVerdict::RemoveUnusedImportHeld {
      held_kind: RemoveUnusedImportHeldKind::MissingCandidates,
      reason: "host symbol resolver returned no unused-import candidates".to_string(),
    };
  }
  if any_macro_or_cfg(&req.candidate_imports) {
    return RemoveUnusedImportVerdict::RemoveUnusedImportHeld {
      held_kind: RemoveUnusedImportHeldKind::MacroBinding,
      reason: "candidate import is used by a macro or behind a cfg attribute; deletion may break a build configuration not currently active".to_string(),
    };
  }
  if matches!(req.scope, RemoveUnusedImportScope::CrateWide) {
    return RemoveUnusedImportVerdict::RemoveUnusedImportHeld {
      held_kind: RemoveUnusedImportHeldKind::ScopeTooBroad,
      reason: "scope=crate-wide requires explicit owner approval".to_string(),
    };
  }
  if is_test_path(&req.target_path) && !matches!(req.scope, RemoveUnusedImportScope::TestsAlso) {
    return RemoveUnusedImportVerdict::RemoveUnusedImportHeld {
      held_kind: RemoveUnusedImportHeldKind::TestFileProtected,
      reason: "target is a test file; pass scope=tests-also to operate on tests".to_string(),
    };
  }
  RemoveUnusedImportVerdict::RemoveUnusedImportReady
}

/// One line-level edit: the byte range (including the trailing
/// newline) of an unused import line to be removed.
///
/// OWNER-LAW (2026-05-11): same shape philosophy as `RenameEdit` but
/// line-granular instead of byte-granular — `remove-unused-import` is
/// a *line-deletion* transform, not a *byte-substitution* one.
/// `byte_offset` and `byte_len` describe the exact range to splice
/// out (inclusive of trailing `\n` when present), so
/// `apply_remove_import_edits` can deterministically reconstruct the
/// post-removal content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoveImportEdit {
  pub byte_offset: usize,
  pub byte_len: usize,
  /// 1-indexed source line number (informational; the edit body is
  /// authoritatively given by `byte_offset` + `byte_len`).
  pub line: usize,
  /// The verbatim line content being removed (without trailing `\n`).
  /// Useful for the patch-candidate / unified-diff renderer.
  pub removed_line_content: String,
  /// Which candidate's `module` this edit targets — for audit and
  /// for round-tripping (multiple candidates can match in the same
  /// file; the edit names which one).
  pub module: String,
}

/// Find the line in `content` whose body matches the import form for
/// the given language. Returns `None` when no matching line is found.
///
/// OWNER-LAW (2026-05-11): deterministic line-based pattern match per
/// language — no fuzzy matching, no semantic resolution. Each
/// language has a closed set of acceptable line forms; lines not
/// matching any form are skipped.
///
/// Per-language phase-1 forms:
///
///   - **Rust**: `use <module>;` (trimmed line exact)
///   - **Python**: `import <module>` OR `from <pkg> import <name>`
///     where `module` is split on the last `.` into `<pkg>.<name>`.
///     Multi-import (`from x import a, b`) and aliased
///     (`import x as y`, `from x import y as z`) forms are
///     intentionally NOT matched in phase 1 — they require argument
///     splitting that risks breaking unrelated imports on the same
///     line.
///   - **TypeScript / JavaScript**: `import <module> from '...'`
///     (default import) OR `import { <module> } from '...'`
///     (named single import). Multi-named-import
///     (`import { a, b } from '...'`), namespace import
///     (`import * as Foo`), and side-effect import
///     (`import '...'`) are NOT matched in phase 1.
///   - **Go**: `import "<module>"` (single-package import).
///     Block-import (`import ( ... )`) and aliased import
///     (`import alias "<module>"`) are NOT matched in phase 1.
///
/// Quoting (TS / JS / Go): single quotes (`'...'`) and double
/// quotes (`"..."`) are both accepted as string delimiters. Mixed
/// quotes within a single string are NOT supported — neither is
/// Rust syntax.
pub fn find_unused_import_line(
  content: &str,
  module: &str,
  language: &str,
) -> Option<RemoveImportEdit> {
  let predicate: fn(&str, &str) -> bool = match language {
    "rust" => matches_rust_use_line,
    "python" => matches_python_import_line,
    "typescript" => matches_typescript_import_line,
    "javascript" => matches_typescript_import_line, // same shape as TS
    "go" => matches_go_import_line,
    _ => return None,
  };
  find_import_line_matching(content, module, predicate)
}

/// Generic line walker — applies `predicate` to each line's trimmed
/// content, returns the first match as a `RemoveImportEdit`. Used by
/// every per-language detector to avoid code duplication.
fn find_import_line_matching(
  content: &str,
  module: &str,
  predicate: fn(&str, &str) -> bool,
) -> Option<RemoveImportEdit> {
  let mut byte_offset = 0usize;
  for (line_idx, line_with_nl) in content.split_inclusive('\n').enumerate() {
    let line = line_with_nl.strip_suffix('\n').unwrap_or(line_with_nl);
    let trimmed = line.trim();
    if predicate(trimmed, module) {
      return Some(RemoveImportEdit {
        byte_offset,
        byte_len: line_with_nl.len(),
        line: line_idx + 1,
        removed_line_content: line.to_string(),
        module: module.to_string(),
      });
    }
    byte_offset += line_with_nl.len();
  }
  None
}

/// Rust-specific predicate: `use <module>;`.
///
/// Avoids allocation by checking prefix `"use "` + suffix `";"` and
/// slicing the middle.
fn matches_rust_use_line(trimmed: &str, module: &str) -> bool {
  if !trimmed.starts_with("use ") {
    return false;
  }
  if !trimmed.ends_with(';') {
    return false;
  }
  // Slice between "use " (4 bytes) and the trailing ";".
  let inner = &trimmed[4..trimmed.len() - 1];
  inner == module
}

/// Backward-compatible Rust-line finder. Retained for direct callers
/// from earlier slices; internally delegates to the generic walker.
#[allow(dead_code)] // kept as a stable Rust-only entry-point for callers
fn find_rust_use_line(content: &str, module: &str) -> Option<RemoveImportEdit> {
  find_import_line_matching(content, module, matches_rust_use_line)
}

/// Python predicate. Two accepted forms:
///   - `import <module>` — trimmed line exact (Python lets you do
///     `import os.path`; the whole dotted path counts as `module`)
///   - `from <pkg> import <name>` — where `pkg + "." + name`
///     equals `module`. So `module = "os.path"` matches both
///     `import os.path` AND `from os import path`.
///
/// Multi-import + alias forms are intentionally NOT matched (would
/// require splitting the line on `,` and `as` — risky in phase 1
/// because deletion would orphan or mismatch other imports on the
/// same line).
fn matches_python_import_line(trimmed: &str, module: &str) -> bool {
  // `import <module>` exact.
  if let Some(rest) = trimmed.strip_prefix("import ") {
    if rest == module && !rest.contains(',') && !rest.contains(" as ") {
      return true;
    }
  }
  // `from <pkg> import <name>`.
  if let Some(rest) = trimmed.strip_prefix("from ") {
    if let Some(idx) = rest.find(" import ") {
      let pkg = &rest[..idx];
      let name = &rest[idx + " import ".len()..];
      // Reject multi-import / alias.
      if name.contains(',') || name.contains(" as ") || pkg.contains(" as ") {
        return false;
      }
      // Reconstruct pkg.name and compare to module.
      if module.len() == pkg.len() + 1 + name.len()
        && module.starts_with(pkg)
        && module.as_bytes().get(pkg.len()) == Some(&b'.')
        && module.ends_with(name)
      {
        return true;
      }
    }
  }
  false
}

/// TypeScript / JavaScript predicate. Two accepted forms:
///   - `import <Module> from '<spec>'` or `... from "<spec>";` —
///     default import.
///   - `import { <Module> } from '<spec>';` — single-named import
///     (multi-named like `{ a, b }` is NOT matched in phase 1).
///
/// Quoting: both `'...'` and `"..."` accepted for `<spec>`. Optional
/// trailing `;`. The `<spec>` (the module path the import comes
/// FROM) is not constrained — the predicate only checks the
/// imported NAME against `module`.
fn matches_typescript_import_line(trimmed: &str, module: &str) -> bool {
  if !trimmed.starts_with("import ") {
    return false;
  }
  // Drop optional trailing `;` and trim trailing whitespace.
  let body = trimmed.strip_suffix(';').unwrap_or(trimmed).trim_end();
  // Must end in a string literal — match `'<...>'` or `"<...>"`.
  let last = body.chars().last();
  if last != Some('\'') && last != Some('"') {
    return false;
  }
  // Find ` from <quote>...<quote>` suffix.
  let from_idx = body.rfind(" from ");
  let from_idx = match from_idx {
    Some(i) => i,
    None => return false,
  };
  let after_from = body[from_idx + " from ".len()..].trim();
  // Verify the `from <spec>` part is a quoted string literal.
  if !((after_from.starts_with('\'') && after_from.ends_with('\''))
    || (after_from.starts_with('"') && after_from.ends_with('"')))
  {
    return false;
  }
  // Imports clause: between "import " (7 chars) and " from ".
  let imports = body[7..from_idx].trim();
  // Default import: bare identifier.
  if imports == module {
    return true;
  }
  // Named single import: `{ <module> }` (with optional whitespace).
  if let Some(inner) = imports.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
    let inner = inner.trim();
    if !inner.contains(',') && inner == module {
      return true;
    }
  }
  false
}

/// Go predicate: `import "<module>"` (single-package form).
///
/// Block-imports (`import ( "a" \n "b" )`) are NOT matched in phase 1
/// — multi-line forms require parser-level handling.
fn matches_go_import_line(trimmed: &str, module: &str) -> bool {
  let rest = match trimmed.strip_prefix("import ") {
    Some(r) => r,
    None => return false,
  };
  // Optional alias before the spec: `import alias "<module>"`.
  // For phase 1, only accept the alias-less form to keep the contract
  // simple. An aliased import would need to know the host symbol
  // resolver's intent (delete based on alias name or spec).
  if let Some(inner) = rest.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
    return inner == module;
  }
  false
}

/// Walk all `candidate_imports` and produce edits for the lines
/// found. Candidates with no matching line in `content` are silently
/// skipped — the host's symbol resolver may have flagged an import
/// that doesn't exist in this file (e.g. it was already removed by
/// an earlier transform pass).
///
/// OWNER-LAW (2026-05-11): pure function. Same inputs → same edits.
/// Caller is responsible for filtering candidates beforehand
/// (`classify_remove_unused_import` should have returned `Ready`
/// before this is called).
///
/// Phase 2 (2026-05-12): the function now dispatches to per-language
/// **multi-import / aliased / block-import** handlers when applicable:
///
///   - Python `from pkg import a, b` and `import a, b` partial
///     removal (delete just the unused name + separator); aliased
///     `import a as x` / `from pkg import a as y` (match the alias
///     OR the original name); whole-line collapse when ALL names on
///     a line are flagged.
///   - TypeScript / JavaScript `import { A, B as C } from "m"`
///     partial removal; default + named (`import A, { B }`);
///     namespace (`import * as A`); type-only (`import type { A }`);
///     whole-line collapse when ALL named imports flagged AND there's
///     no default/namespace.
///   - Go `import alias "pkg"` aliased, `_ "pkg"` blank import,
///     `. "pkg"` dot import, **`import ( ... )` block** — line-level
///     removal inside the block, whole-block delete when ALL specs
///     are flagged.
///
/// Phase 1 single-line forms still work; phase 2 only kicks in when
/// the source has multi-import / aliased / block shapes that phase 1
/// would have skipped.
pub fn compute_remove_unused_import_edits(
  content: &str,
  candidates: &[UnusedImportCandidate],
  language: &str,
) -> Vec<RemoveImportEdit> {
  let mut edits: Vec<RemoveImportEdit> = match language {
    "python" => compute_python_remove_edits(content, candidates),
    "typescript" | "javascript" => compute_typescript_remove_edits(content, candidates),
    "go" => compute_go_remove_edits(content, candidates),
    _ => {
      // Rust + unrecognized languages: stay on the phase-1 line walker.
      let mut v = Vec::new();
      for c in candidates {
        if let Some(edit) = find_unused_import_line(content, &c.module, language) {
          v.push(edit);
        }
      }
      v
    }
  };
  // Sort by byte_offset so `apply_remove_import_edits` can splice in
  // order without index shifts. The dedup also collapses duplicate
  // candidates pointing at the same line.
  edits.sort_by_key(|e| e.byte_offset);
  edits.dedup_by_key(|e| e.byte_offset);
  edits
}

// ─── phase-2: per-language multi-import / alias / block dispatchers ─

/// Per-name role within an import statement. Used by phase-2.1 to
/// distinguish "all named-clause entries removed but default
/// untouched" (which should collapse `, { ... }` rather than leave
/// empty braces) from generic partial deletes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImportNameKind {
  /// `A` in `import A from "m"` or `import A, { B } from "m"`.
  Default,
  /// `A` in `import * as A from "m"` (or, defensively, in the
  /// invalid-but-handled `import * as A, { B }`).
  Namespace,
  /// A name inside `{ ... }`: `B` / `C as D` / etc.
  Named,
  /// Anything else — Python / Go / single-name forms not covered by
  /// the named-clause-collapse logic. Treated as opaque by the
  /// post-emit collapse pass; behaves exactly like the legacy
  /// "partial edit per flagged name" path.
  Other,
}

/// One imported name on a Python / TS / Go statement line. Bytes
/// `(start, end)` are absolute offsets into the file (not the line).
#[derive(Debug, Clone)]
struct ImportName {
  /// Role this name plays in its statement. Phase-2.1 uses this to
  /// detect the empty-`{}` collapse case for TS/JS default+named.
  kind: ImportNameKind,
  /// What the file source calls this — the original name, possibly
  /// followed by an `as` alias.
  ///
  /// E.g. for `from os import path as p`, two queries match: `os.path`
  /// (the canonical module path) and `p` (the alias). The host
  /// symbol resolver can hand either string as the `module`.
  match_keys: Vec<String>,
  /// Absolute byte range covering this name's full token plus its
  /// adjacent separator (comma + whitespace). Designed so that
  /// applying the deletion to the original content leaves the rest
  /// of the line syntactically valid.
  delete_range: (usize, usize),
  /// Absolute byte range of just the identifier token (without
  /// separators). Reserved for future phases that need the precise
  /// ident span (e.g. converting `import a, b as x` deletions into
  /// targeted partial rewrites of the `as x` clause).
  #[allow(dead_code)]
  ident_range: (usize, usize),
}

/// A multi-import statement spanning one or more lines: a line's
/// imported names, with the absolute byte ranges of the line itself
/// (for whole-line collapse) and each imported name.
#[derive(Debug, Clone)]
struct ImportStatement {
  /// Absolute byte range of the whole statement (including trailing
  /// newline when present). Used for whole-line collapse.
  line_range: (usize, usize),
  /// 1-indexed source line number of the first line of the statement.
  line_no: usize,
  /// `removed_line_content` field value when collapsing — the line
  /// text WITHOUT trailing newline.
  line_content: String,
  /// All imported names this statement introduces.
  names: Vec<ImportName>,
  /// TS/JS default+named (or defensively namespace+named) only:
  /// byte range of `, { ... }` — from the comma right after the
  /// default/namespace clause through the position just past the
  /// closing brace. Used when every named entry is flagged but the
  /// default is not, to collapse the empty `{}` artifact (phase-2.1,
  /// 2026-05-12). `None` when there's no named clause or no
  /// default/namespace beside it.
  named_clause_collapse_range: Option<(usize, usize)>,
}

/// True iff every name in `stmt` has at least one `match_keys` entry
/// that's listed in `candidate_modules`. Reserved as a clarity
/// helper alongside `emit_statement_edits`, which already inlines
/// this predicate.
#[allow(dead_code)]
fn all_names_flagged(stmt: &ImportStatement, candidate_modules: &[String]) -> bool {
  stmt
    .names
    .iter()
    .all(|n| n.match_keys.iter().any(|k| candidate_modules.contains(k)))
}

/// Emit edits for one statement given a set of flagged module strings.
/// Returns 0..N edits:
///   - 0 edits when no name is flagged
///   - 1 whole-line edit when ALL names are flagged
///   - 1 collapse edit covering `, { ... }` when every Named entry is
///     flagged but a Default/Namespace entry is NOT flagged (phase-2.1
///     empty-`{}` collapse for TS/JS default+named)
///   - 1 partial edit per flagged name otherwise
fn emit_statement_edits(
  stmt: &ImportStatement,
  candidate_modules: &[String],
) -> Vec<RemoveImportEdit> {
  let flagged: Vec<&ImportName> = stmt
    .names
    .iter()
    .filter(|n| n.match_keys.iter().any(|k| candidate_modules.contains(k)))
    .collect();
  if flagged.is_empty() {
    return Vec::new();
  }
  if flagged.len() == stmt.names.len() {
    // All names flagged — collapse to whole-line delete.
    return vec![RemoveImportEdit {
      byte_offset: stmt.line_range.0,
      byte_len: stmt.line_range.1 - stmt.line_range.0,
      line: stmt.line_no,
      removed_line_content: stmt.line_content.clone(),
      module: flagged[0].match_keys.first().cloned().unwrap_or_default(),
    }];
  }
  // Phase-2.1 (2026-05-12) — empty-`{}` collapse for TS/JS default+named.
  //
  // Condition: there's a `named_clause_collapse_range` (set by the TS
  // parser when the statement has a default/namespace clause paired
  // with `{ ... }`), every Named entry is flagged, and at least one
  // Default/Namespace entry exists and is NOT flagged.
  //
  // Without this, the partial path would emit per-name deletes inside
  // `{}` and leave an empty `import A, { } from "m"` artifact on disk.
  // The collapse range covers `, { ... }` so the result becomes
  // `import A from "m"`.
  if let Some((collapse_start, collapse_end)) = stmt.named_clause_collapse_range {
    let named_total = stmt
      .names
      .iter()
      .filter(|n| n.kind == ImportNameKind::Named)
      .count();
    let named_flagged = flagged
      .iter()
      .filter(|n| n.kind == ImportNameKind::Named)
      .count();
    let default_kept = stmt.names.iter().any(|n| {
      matches!(n.kind, ImportNameKind::Default | ImportNameKind::Namespace)
        && !flagged.iter().any(|f| std::ptr::eq(*f, n))
    });
    if named_total > 0 && named_flagged == named_total && default_kept {
      let module = flagged
        .iter()
        .find(|n| n.kind == ImportNameKind::Named)
        .and_then(|n| n.match_keys.first().cloned())
        .unwrap_or_default();
      return vec![RemoveImportEdit {
        byte_offset: collapse_start,
        byte_len: collapse_end - collapse_start,
        line: stmt.line_no,
        removed_line_content: stmt.line_content.clone(),
        module,
      }];
    }
  }
  // Partial: one edit per flagged name (its delete_range).
  flagged
    .iter()
    .map(|n| RemoveImportEdit {
      byte_offset: n.delete_range.0,
      byte_len: n.delete_range.1 - n.delete_range.0,
      line: stmt.line_no,
      removed_line_content: stmt.line_content.clone(),
      module: n.match_keys.first().cloned().unwrap_or_default(),
    })
    .collect()
}

/// Python phase-2 dispatcher. Recognizes both phase-1 single-line
/// forms (`import os.path`, `from os import path`) AND phase-2 forms
/// (`import a, b`, `import a as x`, `from pkg import a, b as c`).
fn compute_python_remove_edits(
  content: &str,
  candidates: &[UnusedImportCandidate],
) -> Vec<RemoveImportEdit> {
  let modules: Vec<String> = candidates.iter().map(|c| c.module.clone()).collect();
  let mut out = Vec::new();
  let mut byte_offset = 0usize;
  for (line_idx, line_with_nl) in content.split_inclusive('\n').enumerate() {
    let line = line_with_nl.strip_suffix('\n').unwrap_or(line_with_nl);
    let line_start = byte_offset;
    let line_end = byte_offset + line_with_nl.len();
    let line_content = line.to_string();
    let line_no = line_idx + 1;
    // Walk leading whitespace to find the trimmed body's absolute
    // byte offset.
    let trimmed_start_in_line = line.len() - line.trim_start().len();
    let trimmed_abs_start = line_start + trimmed_start_in_line;
    let trimmed = line.trim_start().trim_end();
    if let Some(stmt) = parse_python_import_statement(
      trimmed,
      trimmed_abs_start,
      (line_start, line_end),
      line_no,
      line_content.clone(),
    ) {
      out.extend(emit_statement_edits(&stmt, &modules));
    }
    byte_offset = line_end;
  }
  out
}

/// Parse a Python import statement on a single trimmed line. Returns
/// `None` if the line isn't an import statement (the caller's
/// per-line walker keeps moving). Aliased + multi-import forms are
/// recognized; the matched keys for each name include both the
/// canonical module path (e.g. `os.path`) and the alias (if any).
fn parse_python_import_statement(
  trimmed: &str,
  trimmed_abs_start: usize,
  line_range: (usize, usize),
  line_no: usize,
  line_content: String,
) -> Option<ImportStatement> {
  // `from <pkg> import <name>[, <name>...]` form.
  if let Some(rest) = trimmed.strip_prefix("from ") {
    let pkg_end_idx = rest.find(" import ")?;
    let pkg = &rest[..pkg_end_idx];
    let names_str_start_in_trimmed = "from ".len() + pkg_end_idx + " import ".len();
    let names_str = &rest[pkg_end_idx + " import ".len()..];
    let names = parse_python_name_list(
      names_str,
      trimmed_abs_start + names_str_start_in_trimmed,
      Some(pkg),
    );
    if names.is_empty() {
      return None;
    }
    return Some(ImportStatement {
      line_range,
      line_no,
      line_content,
      names,
      named_clause_collapse_range: None,
    });
  }
  // `import <name>[, <name>...]` form.
  if let Some(rest) = trimmed.strip_prefix("import ") {
    let names_str_start_in_trimmed = "import ".len();
    let names = parse_python_name_list(rest, trimmed_abs_start + names_str_start_in_trimmed, None);
    if names.is_empty() {
      return None;
    }
    return Some(ImportStatement {
      line_range,
      line_no,
      line_content,
      names,
      named_clause_collapse_range: None,
    });
  }
  None
}

/// Parse a Python comma-separated import name list. `pkg` is `Some(p)`
/// for `from p import a, b as c` — the `match_keys` for each name
/// include `p.<name>` AND the alias (when present). For bare `import
/// a, b as c` (pkg = None), the keys include the name itself AND the
/// alias.
///
/// Returns one `ImportName` per comma-separated entry. The
/// `delete_range` covers the name + its trailing comma + whitespace
/// (for non-last entries), or the leading comma + whitespace + name
/// (for the last entry only).
fn parse_python_name_list(
  names_str: &str,
  names_str_abs_start: usize,
  pkg: Option<&str>,
) -> Vec<ImportName> {
  // Split by ',' but preserve byte offsets.
  let bytes = names_str.as_bytes();
  let mut parts: Vec<(usize, usize)> = Vec::new(); // (start, end) within names_str
  let mut part_start = 0usize;
  for (i, b) in bytes.iter().enumerate() {
    if *b == b',' {
      parts.push((part_start, i));
      part_start = i + 1;
    }
  }
  parts.push((part_start, names_str.len()));
  if parts.is_empty() {
    return Vec::new();
  }
  let mut out: Vec<ImportName> = Vec::new();
  let part_count = parts.len();
  for (i, (s, e)) in parts.iter().enumerate() {
    let raw = &names_str[*s..*e];
    let trimmed = raw.trim();
    if trimmed.is_empty() {
      continue;
    }
    // Decompose `<name>` or `<name> as <alias>`.
    let (orig, alias) = match trimmed.find(" as ") {
      Some(idx) => (
        trimmed[..idx].trim(),
        Some(trimmed[idx + " as ".len()..].trim()),
      ),
      None => (trimmed, None),
    };
    // Compute the identifier range (the original name's bytes, not
    // including alias).
    let raw_inner_start = raw.len() - raw.trim_start().len();
    let ident_abs_start = names_str_abs_start + s + raw_inner_start;
    let ident_abs_end = ident_abs_start + orig.len();
    // Build match keys.
    let mut keys: Vec<String> = Vec::new();
    if let Some(p) = pkg {
      keys.push(format!("{p}.{orig}"));
    } else {
      keys.push(orig.to_string());
    }
    if let Some(a) = alias {
      keys.push(a.to_string());
    }
    // Compute the delete range: name + separator. For non-last
    // entries, extend past the trailing `,` AND any following
    // whitespace so the surviving names keep a single space
    // separator. For the last entry, include the preceding `,` and
    // any whitespace between it and the name.
    let (drange_start, drange_end);
    let names_bytes = names_str.as_bytes();
    if i + 1 < part_count {
      drange_start = names_str_abs_start + s;
      let mut comma_end = *e + 1; // past the ','
      while comma_end < names_str.len()
        && (names_bytes[comma_end] == b' ' || names_bytes[comma_end] == b'\t')
      {
        comma_end += 1;
      }
      drange_end = names_str_abs_start + comma_end;
    } else if i > 0 {
      let prev_end = parts[i - 1].1;
      drange_start = names_str_abs_start + prev_end;
      drange_end = names_str_abs_start + *e;
    } else {
      drange_start = names_str_abs_start + s;
      drange_end = names_str_abs_start + *e;
    }
    out.push(ImportName {
      kind: ImportNameKind::Other,
      match_keys: keys,
      delete_range: (drange_start, drange_end),
      ident_range: (ident_abs_start, ident_abs_end),
    });
  }
  out
}

/// TypeScript / JavaScript phase-2 dispatcher.
fn compute_typescript_remove_edits(
  content: &str,
  candidates: &[UnusedImportCandidate],
) -> Vec<RemoveImportEdit> {
  let modules: Vec<String> = candidates.iter().map(|c| c.module.clone()).collect();
  let mut out = Vec::new();
  let mut byte_offset = 0usize;
  for (line_idx, line_with_nl) in content.split_inclusive('\n').enumerate() {
    let line = line_with_nl.strip_suffix('\n').unwrap_or(line_with_nl);
    let line_start = byte_offset;
    let line_end = byte_offset + line_with_nl.len();
    let line_content = line.to_string();
    let line_no = line_idx + 1;
    let trimmed_start_in_line = line.len() - line.trim_start().len();
    let trimmed_abs_start = line_start + trimmed_start_in_line;
    let trimmed = line.trim_start().trim_end();
    if let Some(stmt) = parse_typescript_import_statement(
      trimmed,
      trimmed_abs_start,
      (line_start, line_end),
      line_no,
      line_content.clone(),
    ) {
      out.extend(emit_statement_edits(&stmt, &modules));
    }
    byte_offset = line_end;
  }
  out
}

/// Parse a TypeScript / JavaScript ES6 import on a single trimmed
/// line. Recognized forms (each emits one or more `ImportName`):
///   - `import A from "m"`                — default
///   - `import { B } from "m"`            — named single
///   - `import { B, C as D } from "m"`    — named multi w/ alias
///   - `import A, { B } from "m"`         — default + named
///   - `import * as A from "m"`           — namespace
///   - `import type { A } from "m"`       — type-only (treated like named)
///   - `import type A from "m"`           — type-only default
fn parse_typescript_import_statement(
  trimmed: &str,
  trimmed_abs_start: usize,
  line_range: (usize, usize),
  line_no: usize,
  line_content: String,
) -> Option<ImportStatement> {
  let mut body = trimmed.strip_prefix("import ")?;
  let body_offset_in_trimmed = "import ".len();
  // Optional `type ` modifier — skip for matching purposes.
  let type_only = body.starts_with("type ");
  if type_only {
    body = body.strip_prefix("type ")?;
  }
  let body_abs_start =
    trimmed_abs_start + body_offset_in_trimmed + if type_only { "type ".len() } else { 0 };
  // Strip trailing `;` and whitespace.
  let body_for_split = body.trim_end_matches(';').trim_end();
  let from_idx = body_for_split.rfind(" from ")?;
  let imports_part = &body_for_split[..from_idx];
  let imports_start_in_body = 0usize;
  let imports_abs_start = body_abs_start + imports_start_in_body;
  let mut names: Vec<ImportName> = Vec::new();
  // The imports clause can be:
  //   - `A`                  default
  //   - `{ B, C as D }`      named
  //   - `A, { B }`           default + named
  //   - `* as A`             namespace
  let trimmed_imports = imports_part.trim();
  // Split into "default" part (before `,`) and "named" part `{...}`.
  //
  // For default+named (`import A, { B } from "m"`) AND defensively for
  // namespace+named, we precompute the byte range of the default /
  // namespace clause from the start of the imports_part through the
  // position just before `{`. This range — "A, " in the example — is
  // the partial delete that preserves the named clause when ONLY the
  // default is flagged. Without it, falling back to whole-line would
  // silently drop the named imports (P0-2 bug, 2026-05-12).
  let has_named_braces = trimmed_imports.find('{').is_some();
  let default_partial_range_for_paired: Option<(usize, usize)> = if has_named_braces {
    // Absolute byte offset of `{` in the source.
    let brace_in_imports = imports_part.find('{').unwrap();
    let brace_abs = imports_abs_start + brace_in_imports;
    // Start at the first non-whitespace byte of imports_part — that's
    // where the default ident begins. (For `import A,` form there's
    // no leading whitespace; for `import   A,` the spaces are skipped.)
    let leading_ws = imports_part.len() - imports_part.trim_start().len();
    let default_start_abs = imports_abs_start + leading_ws;
    Some((default_start_abs, brace_abs))
  } else {
    None
  };
  // Phase-2.1 collapse range for `import A, { B } from "m"` shape:
  // covers `, { ... }` — from the position just past the default /
  // namespace ident through the position just past the closing `}`.
  // Used when every named entry is flagged but the default is not.
  // `None` when there's no named clause at all OR when the imports
  // part has no default/namespace ident (pure `import { B }` form —
  // its empty-`{}` collapse is the whole-line delete path).
  let named_clause_collapse_range: Option<(usize, usize)> = if has_named_braces {
    let trimmed_imports_abs_start =
      imports_abs_start + (imports_part.len() - imports_part.trim_start().len());
    // brace_idx is in trimmed_imports; the substring before it
    // (after trimming trailing comma + whitespace) is the default
    // clause text. If it's empty, this is a pure-named statement —
    // no collapse range needed (the all-flagged path falls through
    // to whole-line delete).
    let brace_idx = trimmed_imports.find('{').unwrap();
    let pre = trimmed_imports[..brace_idx]
      .trim()
      .trim_end_matches(',')
      .trim();
    if !pre.is_empty() {
      // Default present beside named. Collapse range: end of `pre`
      // through end of `}`.
      let pre_end_in_trimmed = pre.len();
      // pre.len() is the byte length of the trimmed default text;
      // it sits at offset 0 in trimmed_imports (after `trim()`).
      let collapse_start = trimmed_imports_abs_start + pre_end_in_trimmed;
      // Find the `}` matching brace_idx in trimmed_imports. The
      // close index inside braced is brace_idx + position_of_close.
      let braced = &trimmed_imports[brace_idx..];
      let close_idx_in_braced = braced.find('}').unwrap_or(0);
      let close_abs = trimmed_imports_abs_start + brace_idx + close_idx_in_braced + 1;
      Some((collapse_start, close_abs))
    } else {
      None
    }
  } else {
    None
  };
  let (default_part, named_part) = if let Some(brace_idx) = trimmed_imports.find('{') {
    let pre = trimmed_imports[..brace_idx]
      .trim()
      .trim_end_matches(',')
      .trim();
    let braced = &trimmed_imports[brace_idx..];
    let close_idx = braced.find('}')?;
    let inside = &braced[1..close_idx];
    let braced_start_in_imports = imports_part.find('{').unwrap();
    let inside_abs_start = imports_abs_start + braced_start_in_imports + 1;
    (
      Some((pre.to_string(), 0usize)),
      Some((inside.to_string(), inside_abs_start)),
    )
  } else {
    (Some((trimmed_imports.to_string(), 0usize)), None)
  };
  // Default / namespace name (if any).
  if let Some((default_str, _)) = default_part {
    let ds = default_str.trim();
    if !ds.is_empty() {
      // Pick the delete range:
      //   - paired with a named `{...}`: use the precomputed
      //     "default through `{`" range so partial-path deletion of
      //     only the default leaves the named clause intact.
      //   - no named clause: fall back to whole-line (existing
      //     behavior). When the default is the sole import, partial
      //     and all-flagged paths both end up at whole-line anyway.
      let default_drange = default_partial_range_for_paired.unwrap_or(line_range);
      if let Some(ns_rest) = ds.strip_prefix("* as ") {
        let ns = ns_rest.trim();
        if !ns.is_empty() {
          names.push(ImportName {
            kind: ImportNameKind::Namespace,
            match_keys: vec![ns.to_string()],
            // Same paired-vs-solo handling: defensively support the
            // (technically invalid in ES) `import * as A, { B }` form.
            delete_range: default_drange,
            ident_range: default_drange,
          });
        }
      } else if !ds.is_empty() {
        names.push(ImportName {
          kind: ImportNameKind::Default,
          match_keys: vec![ds.to_string()],
          delete_range: default_drange,
          ident_range: default_drange,
        });
      }
    }
  }
  // Named imports inside `{...}`.
  if let Some((inside, inside_abs_start)) = named_part {
    let mut part_start = 0usize;
    let mut parts: Vec<(usize, usize)> = Vec::new();
    for (i, b) in inside.as_bytes().iter().enumerate() {
      if *b == b',' {
        parts.push((part_start, i));
        part_start = i + 1;
      }
    }
    parts.push((part_start, inside.len()));
    let part_count = parts.len();
    for (i, (s, e)) in parts.iter().enumerate() {
      let raw = &inside[*s..*e];
      let trimmed_name = raw.trim();
      if trimmed_name.is_empty() {
        continue;
      }
      let (orig, alias) = match trimmed_name.find(" as ") {
        Some(idx) => (
          trimmed_name[..idx].trim(),
          Some(trimmed_name[idx + " as ".len()..].trim()),
        ),
        None => (trimmed_name, None),
      };
      let raw_inner_start = raw.len() - raw.trim_start().len();
      let ident_abs_start = inside_abs_start + s + raw_inner_start;
      let ident_abs_end = ident_abs_start + orig.len();
      let mut keys = vec![orig.to_string()];
      if let Some(a) = alias {
        keys.push(a.to_string());
      }
      let (drange_start, drange_end);
      let inside_bytes = inside.as_bytes();
      if i + 1 < part_count {
        // Non-last: delete from the ident start (preserving leading
        // whitespace from previous separator) past the comma and
        // following whitespace.
        drange_start = ident_abs_start;
        let mut comma_end = *e + 1;
        while comma_end < inside.len()
          && (inside_bytes[comma_end] == b' ' || inside_bytes[comma_end] == b'\t')
        {
          comma_end += 1;
        }
        drange_end = inside_abs_start + comma_end;
      } else if i > 0 {
        // Last entry: include preceding comma + leading whitespace
        // before this name. Walk back from `e` to skip trailing
        // whitespace so braces-formatted lists like `{ Foo, Bar }`
        // keep their pre-`}` space intact.
        let prev_end = parts[i - 1].1;
        drange_start = inside_abs_start + prev_end;
        let mut last_byte = *e;
        while last_byte > prev_end + 1
          && (inside_bytes[last_byte - 1] == b' ' || inside_bytes[last_byte - 1] == b'\t')
        {
          last_byte -= 1;
        }
        drange_end = inside_abs_start + last_byte;
      } else {
        drange_start = inside_abs_start + s;
        drange_end = inside_abs_start + *e;
      }
      names.push(ImportName {
        kind: ImportNameKind::Named,
        match_keys: keys,
        delete_range: (drange_start, drange_end),
        ident_range: (ident_abs_start, ident_abs_end),
      });
    }
  }
  if names.is_empty() {
    return None;
  }
  Some(ImportStatement {
    line_range,
    line_no,
    line_content,
    names,
    named_clause_collapse_range,
  })
}

/// Go phase-2 dispatcher. Handles single-line aliased imports
/// (`import alias "pkg"`, `import _ "pkg"`, `import . "pkg"`) AND
/// block-imports `import ( ... )`. When ALL specs in a block are
/// flagged, the whole block is deleted (open paren through close
/// paren); otherwise per-spec line deletes are emitted.
fn compute_go_remove_edits(
  content: &str,
  candidates: &[UnusedImportCandidate],
) -> Vec<RemoveImportEdit> {
  let modules: Vec<String> = candidates.iter().map(|c| c.module.clone()).collect();
  let mut out = Vec::new();
  let bytes = content.as_bytes();
  let mut byte_offset = 0usize;
  let lines: Vec<&str> = content.split_inclusive('\n').collect();
  let mut i = 0usize;
  // Track absolute byte offset per line.
  let mut line_starts: Vec<usize> = Vec::with_capacity(lines.len() + 1);
  for line in &lines {
    line_starts.push(byte_offset);
    byte_offset += line.len();
  }
  line_starts.push(byte_offset);

  while i < lines.len() {
    let line_with_nl = lines[i];
    let line = line_with_nl.strip_suffix('\n').unwrap_or(line_with_nl);
    let trimmed = line.trim();
    // Block-import opener: `import (` (possibly with trailing comment).
    if trimmed == "import (" || trimmed.starts_with("import (") {
      // Find closing `)`.
      let block_open_start = line_starts[i];
      let mut close_line = i;
      for j in i + 1..lines.len() {
        let inner = lines[j].strip_suffix('\n').unwrap_or(lines[j]).trim();
        if inner == ")" {
          close_line = j;
          break;
        }
      }
      if close_line == i {
        // Unclosed; bail (treat rest as regular lines).
        i += 1;
        continue;
      }
      // Parse each spec line inside the block.
      let mut block_names: Vec<ImportName> = Vec::new();
      let mut spec_line_ranges: Vec<(usize, (usize, usize), usize, String)> = Vec::new();
      // (line_idx, line_range, line_no, line_content)
      for j in (i + 1)..close_line {
        let inner_line_with_nl = lines[j];
        let inner_line = inner_line_with_nl
          .strip_suffix('\n')
          .unwrap_or(inner_line_with_nl);
        let inner_trim = inner_line.trim();
        if inner_trim.is_empty() || inner_trim.starts_with("//") {
          continue;
        }
        if let Some((module_str, alias_str)) = parse_go_import_spec(inner_trim) {
          // Build match keys: the module path AND the alias (if any).
          let mut keys = vec![module_str.to_string()];
          if let Some(a) = alias_str {
            keys.push(a.to_string());
          }
          let line_start = line_starts[j];
          let line_end = line_starts[j + 1];
          block_names.push(ImportName {
            kind: ImportNameKind::Other,
            match_keys: keys,
            // For block specs, delete_range is the whole spec line
            // (including trailing newline).
            delete_range: (line_start, line_end),
            ident_range: (line_start, line_end),
          });
          spec_line_ranges.push((j, (line_start, line_end), j + 1, inner_line.to_string()));
        }
      }
      // Decide: all flagged → whole-block delete; else per-spec.
      let flagged_count = block_names
        .iter()
        .filter(|n| n.match_keys.iter().any(|k| modules.contains(k)))
        .count();
      if flagged_count > 0 && flagged_count == block_names.len() {
        // Whole-block delete (from `import (` through `)\n`).
        let block_end = line_starts[close_line + 1];
        let block_first_line = lines[i].strip_suffix('\n').unwrap_or(lines[i]);
        out.push(RemoveImportEdit {
          byte_offset: block_open_start,
          byte_len: block_end - block_open_start,
          line: i + 1,
          removed_line_content: block_first_line.to_string(),
          module: block_names[0]
            .match_keys
            .first()
            .cloned()
            .unwrap_or_default(),
        });
      } else {
        // Per-spec line deletes for flagged specs only.
        for (idx, n) in block_names.iter().enumerate() {
          if !n.match_keys.iter().any(|k| modules.contains(k)) {
            continue;
          }
          let (_line_idx, line_range, line_no, line_content) = spec_line_ranges[idx].clone();
          out.push(RemoveImportEdit {
            byte_offset: line_range.0,
            byte_len: line_range.1 - line_range.0,
            line: line_no,
            removed_line_content: line_content,
            module: n.match_keys.first().cloned().unwrap_or_default(),
          });
        }
      }
      i = close_line + 1;
      continue;
    }
    // Single-line `import [alias|_|.] "<pkg>"` form.
    if let Some(rest) = trimmed.strip_prefix("import ") {
      if let Some((module_str, alias_str)) = parse_go_import_spec(rest) {
        let key_match = modules
          .iter()
          .any(|m| m == module_str || alias_str.map(|a| m == a).unwrap_or(false));
        if key_match {
          let line_start = line_starts[i];
          let line_end = line_starts[i + 1];
          out.push(RemoveImportEdit {
            byte_offset: line_start,
            byte_len: line_end - line_start,
            line: i + 1,
            removed_line_content: line.to_string(),
            module: module_str.to_string(),
          });
        }
      }
    }
    i += 1;
  }
  // bytes variable kept for symmetry with other parsers; not needed
  // for the absolute-offset arithmetic above.
  let _ = bytes;
  out
}

/// Parse a Go import spec body (the part after `import ` for
/// single-line forms, or the inner line for block forms). Returns
/// `Some((module_path, optional_alias))` if matched. Recognized:
///   - `"pkg/path"`                  — bare, no alias
///   - `alias "pkg/path"`            — named alias
///   - `_ "pkg/path"`                — blank import (alias = `_`)
///   - `. "pkg/path"`                — dot import (alias = `.`)
fn parse_go_import_spec(spec: &str) -> Option<(&str, Option<&str>)> {
  let spec = spec.trim();
  // Strip optional trailing line comment.
  let spec = match spec.find("//") {
    Some(idx) => spec[..idx].trim(),
    None => spec,
  };
  if let Some(inner) = spec.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
    return Some((inner, None));
  }
  // Aliased: `<alias> "<pkg>"`. Find the space + opening quote.
  let space_idx = spec.find(' ')?;
  let alias = spec[..space_idx].trim();
  let rest = spec[space_idx + 1..].trim();
  let inner = rest.strip_prefix('"').and_then(|s| s.strip_suffix('"'))?;
  if alias.is_empty() {
    return None;
  }
  Some((inner, Some(alias)))
}

/// Apply a list of remove-import edits to `content`, returning the
/// post-removal string. Edits must be in ascending `byte_offset`
/// order (as produced by `compute_remove_unused_import_edits`) and
/// pairwise non-overlapping.
///
/// OWNER-LAW (2026-05-11): pure function. No I/O.
pub fn apply_remove_import_edits(content: &str, edits: &[RemoveImportEdit]) -> String {
  let bytes = content.as_bytes();
  let mut out = String::with_capacity(content.len());
  let mut cursor = 0usize;
  for e in edits {
    if e.byte_offset > cursor {
      out.push_str(
        std::str::from_utf8(&bytes[cursor..e.byte_offset])
          .unwrap_or_else(|_| panic!("non-utf8 input to apply_remove_import_edits")),
      );
    }
    // Skip the removed range entirely.
    cursor = e.byte_offset + e.byte_len;
  }
  if cursor < bytes.len() {
    out.push_str(
      std::str::from_utf8(&bytes[cursor..])
        .unwrap_or_else(|_| panic!("non-utf8 input to apply_remove_import_edits")),
    );
  }
  out
}

/// One file's edits + rendered diff for remove-unused-import.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoveUnusedImportFilePatch {
  pub path: String,
  pub edits: Vec<RemoveImportEdit>,
  pub unified_diff: String,
}

/// One file's `(path, content)` input. Unlike rename-symbol which
/// works across a list of files, the `.px` owner law for
/// remove-unused-import names a *single* target file
/// (`target_path` is a String, not a Vec) — the host's symbol resolver
/// is expected to find unused imports in one file at a time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoveUnusedImportFileInput<'a> {
  pub path: &'a str,
  pub content: &'a str,
}

/// Result of [`compute_remove_unused_import_patch_candidate`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoveUnusedImportPatchCandidate {
  pub request: RemoveUnusedImportRequest,
  pub verdict: RemoveUnusedImportVerdict,
  /// Empty on Held / Rejected verdicts (per the canonical chain
  /// invariant — no edits emitted until preconditions pass).
  pub file_patches: Vec<RemoveUnusedImportFilePatch>,
  pub combined_unified_diff: String,
}

/// Render a unified diff for a single file's remove-import edits.
///
/// OWNER-LAW (2026-05-11, extended 2026-05-12): edits are grouped by
/// source line:
///   - When all edits in a line group together cover the entire line
///     (including trailing newline), emit a **1→0 hunk** —
///     `@@ -L,1 +L,0 @@\n-<line>` with no `+` counterpart.
///   - When the edits are partial (multi-import / aliased / block
///     spec removal), emit a **1→1 hunk** —
///     `@@ -L,1 +L,1 @@\n-<original>\n+<post-edit>` where the
///     `+<post-edit>` line is computed by applying the partial
///     deletes to the original line content.
pub fn render_unified_diff_for_remove_unused_import(
  path: &str,
  old_content: &str,
  edits: &[RemoveImportEdit],
) -> String {
  if edits.is_empty() {
    return String::new();
  }
  let mut out = String::new();
  out.push_str(&format!("--- a/{}\n", path));
  out.push_str(&format!("+++ b/{}\n", path));
  // Group edits by line number; within a group, sort by byte_offset.
  let mut by_line: std::collections::BTreeMap<usize, Vec<&RemoveImportEdit>> =
    std::collections::BTreeMap::new();
  for e in edits {
    by_line.entry(e.line).or_default().push(e);
  }
  for (line_no, mut line_edits) in by_line {
    line_edits.sort_by_key(|e| e.byte_offset);
    // Reconstruct the original line's byte range from old_content.
    // The first edit's `byte_offset` may not be at the line start
    // (for partial edits), so we derive line_start by walking
    // backward from byte_offset.
    let first_offset = line_edits[0].byte_offset;
    let bytes = old_content.as_bytes();
    let mut line_start = first_offset;
    while line_start > 0 && bytes[line_start - 1] != b'\n' {
      line_start -= 1;
    }
    let mut line_end_with_nl = first_offset;
    while line_end_with_nl < bytes.len() && bytes[line_end_with_nl] != b'\n' {
      line_end_with_nl += 1;
    }
    let had_trailing_newline = line_end_with_nl < bytes.len() && bytes[line_end_with_nl] == b'\n';
    let line_end_inclusive = if had_trailing_newline {
      line_end_with_nl + 1
    } else {
      line_end_with_nl
    };
    let total_line_bytes = line_end_inclusive - line_start;
    // Sum of edit byte_lens; also check whether edits' ranges are
    // entirely within this single source line.
    let mut total_edit_bytes = 0usize;
    let mut single_line_only = true;
    for e in &line_edits {
      total_edit_bytes += e.byte_len;
      let e_end = e.byte_offset + e.byte_len;
      if e.byte_offset < line_start || e_end > line_end_inclusive {
        single_line_only = false;
      }
    }
    // Whole-line delete iff edits cover the entire line (possibly
    // multi-line for Go block spans — those emit a single edit whose
    // byte range covers the whole block).
    if !single_line_only || (total_edit_bytes == total_line_bytes && line_edits.len() == 1) {
      // Multi-line block delete (Go `import ( ... )` block, etc.).
      //
      // Phase-2.1 fix (2026-05-12): the renderer previously emitted
      // `@@ -L,1 +L,0 @@\n-<opening line>` regardless of how many
      // lines the edit actually spanned. That's a *summary diff*, not
      // a unified diff — `git apply` couldn't replay it, and audit /
      // rollback evidence was thin. Now we count the actual lines
      // covered by the edit's byte range and emit one `-` line per
      // deleted source line.
      if !single_line_only && line_edits.len() == 1 {
        let e = line_edits[0];
        let edit_end = e.byte_offset + e.byte_len;
        // The deleted byte range may or may not include the final
        // newline. Walk the bytes and emit a `-` line per source
        // line — counting `\n` boundaries.
        let deleted = &bytes[e.byte_offset..edit_end];
        let mut deleted_lines: Vec<&[u8]> = Vec::new();
        let mut seg_start = 0usize;
        for (k, b) in deleted.iter().enumerate() {
          if *b == b'\n' {
            deleted_lines.push(&deleted[seg_start..k]);
            seg_start = k + 1;
          }
        }
        // Trailing remainder (no final newline) — count it as a line
        // too, but mark it with the "no newline" sentinel.
        let trailing_no_nl = seg_start < deleted.len();
        if trailing_no_nl {
          deleted_lines.push(&deleted[seg_start..]);
        }
        let n = deleted_lines.len();
        out.push_str(&format!("@@ -{line_no},{n} +{line_no},0 @@\n"));
        for (k, dl) in deleted_lines.iter().enumerate() {
          out.push('-');
          out.push_str(std::str::from_utf8(dl).unwrap_or(""));
          out.push('\n');
          if k + 1 == deleted_lines.len() && trailing_no_nl {
            out.push_str("\\ No newline at end of file\n");
          }
        }
        continue;
      }
      // Single-line whole-line delete.
      out.push_str(&format!("@@ -{line_no},1 +{line_no},0 @@\n"));
      out.push('-');
      out.push_str(&line_edits[0].removed_line_content);
      out.push('\n');
      if !had_trailing_newline {
        out.push_str("\\ No newline at end of file\n");
      }
      continue;
    }
    // Partial edit: compute post-edit line by applying deletes.
    let original_line_no_nl =
      std::str::from_utf8(&bytes[line_start..line_end_with_nl]).unwrap_or("");
    // Apply each edit's range (relative to line_start) to derive
    // the post-edit line.
    let mut new_line = String::with_capacity(original_line_no_nl.len());
    let mut cursor_in_line = 0usize;
    for e in &line_edits {
      let rel_start = e.byte_offset - line_start;
      let rel_end = rel_start + e.byte_len;
      if rel_start > cursor_in_line {
        new_line.push_str(&original_line_no_nl[cursor_in_line..rel_start]);
      }
      cursor_in_line = rel_end;
    }
    if cursor_in_line < original_line_no_nl.len() {
      new_line.push_str(&original_line_no_nl[cursor_in_line..]);
    }
    out.push_str(&format!("@@ -{line_no},1 +{line_no},1 @@\n"));
    out.push('-');
    out.push_str(original_line_no_nl);
    out.push('\n');
    out.push('+');
    out.push_str(&new_line);
    out.push('\n');
    if !had_trailing_newline {
      out.push_str("\\ No newline at end of file\n");
    }
  }
  out
}

/// Orchestrator: classify the request, and on `Ready` walk the single
/// staged file to produce edits + unified diff.
///
/// OWNER-LAW (2026-05-11): pure function. `file_input.path` should
/// equal `request.target_path` — when they don't, the carrier still
/// emits Ready but produces zero edits (the file the caller staged
/// isn't the file the request named, so there's nothing to do).
/// A future strict variant could fail-closed on this mismatch.
pub fn compute_remove_unused_import_patch_candidate(
  request: &RemoveUnusedImportRequest,
  file_input: &RemoveUnusedImportFileInput<'_>,
) -> RemoveUnusedImportPatchCandidate {
  let verdict = classify_remove_unused_import(request);
  let mut file_patches = Vec::new();
  let mut combined = String::new();

  if matches!(verdict, RemoveUnusedImportVerdict::RemoveUnusedImportReady) {
    // Only act on the file the request names — same target-bounded
    // hard boundary as rename-symbol.
    if file_input.path == request.target_path {
      let edits = compute_remove_unused_import_edits(
        file_input.content,
        &request.candidate_imports,
        &request.language,
      );
      if !edits.is_empty() {
        let diff =
          render_unified_diff_for_remove_unused_import(file_input.path, file_input.content, &edits);
        combined.push_str(&diff);
        file_patches.push(RemoveUnusedImportFilePatch {
          path: file_input.path.to_string(),
          edits,
          unified_diff: diff,
        });
      }
    }
  }
  RemoveUnusedImportPatchCandidate {
    request: request.clone(),
    verdict,
    file_patches,
    combined_unified_diff: combined,
  }
}

/// Render a `RemoveUnusedImportPatchCandidate` as the canonical JSON
/// payload of a `coding.code-transform.remove-unused-import-*`
/// artifact.
///
/// OWNER-LAW (2026-05-11): same `next_step` framing as rename-symbol
/// — Ready means proceed to ToolActionApproval gate, Held means
/// operator decision, Rejected means resubmit needed.
pub fn build_remove_unused_import_patch_candidate_payload(
  candidate: &RemoveUnusedImportPatchCandidate,
) -> serde_json::Value {
  let request = &candidate.request;
  let edits_arr: Vec<serde_json::Value> = candidate
    .file_patches
    .iter()
    .flat_map(|fp| {
      fp.edits.iter().map(|e| {
        serde_json::json!({
          "path": fp.path,
          "byte_offset": e.byte_offset,
          "byte_len": e.byte_len,
          "line": e.line,
          "module": e.module,
        })
      })
    })
    .collect();
  let (verdict_str, next_step) = match &candidate.verdict {
    RemoveUnusedImportVerdict::RemoveUnusedImportReady => (
      "remove-unused-import-ready".to_string(),
      "host-unused-import-walk-then-tool-action-approval",
    ),
    RemoveUnusedImportVerdict::RemoveUnusedImportHeld { .. } => (
      "remove-unused-import-held".to_string(),
      "operator-decision-or-resubmit",
    ),
    RemoveUnusedImportVerdict::RemoveUnusedImportRejected { .. } => (
      "remove-unused-import-rejected".to_string(),
      "operator-decision-or-resubmit",
    ),
  };
  let mut payload = serde_json::json!({
    "transform": "remove-unused-import",
    "owner_law": "stdlib/lib/gate/code-transform/remove-unused-import.px",
    "target_path": request.target_path,
    "language": request.language,
    "scope": request.scope.as_str(),
    "candidate_imports": request.candidate_imports,
    "verdict": verdict_str,
    "edits": edits_arr,
    "unified_diff": candidate.combined_unified_diff,
    "candidate_only": true,
    "next_step": next_step,
  });
  // Attach held_kind / reason when applicable.
  match &candidate.verdict {
    RemoveUnusedImportVerdict::RemoveUnusedImportHeld { held_kind, reason }
    | RemoveUnusedImportVerdict::RemoveUnusedImportRejected { held_kind, reason } => {
      payload["held_kind"] = serde_json::Value::String(held_kind.as_str().to_string());
      payload["reason"] = serde_json::Value::String(reason.clone());
    }
    RemoveUnusedImportVerdict::RemoveUnusedImportReady => {}
  }
  payload
}

/// Wrap a `RemoveUnusedImportPatchCandidate` into a full
/// `coding.code-transform.remove-unused-import-{ready,held,rejected}`
/// artifact value with a replay-stable id.
///
/// OWNER-LAW (2026-05-11): the artifact_family suffix
/// (`-ready` / `-held` / `-rejected`) is set from `candidate.verdict`
/// per the `.px` owner law's `buildReceipt`. Replay-stable id binds
/// intrinsic identity: request fields + candidate import list + per
/// edit (path + byte_offset + byte_len) + unified diff bytes.
pub fn build_remove_unused_import_patch_candidate_artifact(
  candidate: &RemoveUnusedImportPatchCandidate,
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
) -> serde_json::Value {
  let payload = build_remove_unused_import_patch_candidate_payload(candidate);
  let suffix = match &candidate.verdict {
    RemoveUnusedImportVerdict::RemoveUnusedImportReady => "ready",
    RemoveUnusedImportVerdict::RemoveUnusedImportHeld { .. } => "held",
    RemoveUnusedImportVerdict::RemoveUnusedImportRejected { .. } => "rejected",
  };
  let artifact_family = format!("coding.code-transform.remove-unused-import-{suffix}");

  // Hash the intrinsic identity. stored_at_ms / repo_snapshot_ref are
  // extrinsic.
  let mut hasher = Sha256::new();
  hasher.update(b"remove-unused-import-patch\x1f");
  hasher.update(candidate.request.target_path.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(candidate.request.language.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(candidate.request.scope.as_str().as_bytes());
  hasher.update(b"\x1f");
  for c in &candidate.request.candidate_imports {
    hasher.update(c.module.as_bytes());
    hasher.update(&[c.used_in_macro as u8, c.behind_cfg as u8]);
    hasher.update(b"\x1e");
  }
  hasher.update(b"\x1f");
  hasher.update(suffix.as_bytes());
  hasher.update(b"\x1f");
  for fp in &candidate.file_patches {
    hasher.update(fp.path.as_bytes());
    hasher.update(b"\x1e");
    for e in &fp.edits {
      hasher.update(e.byte_offset.to_le_bytes());
      hasher.update(e.byte_len.to_le_bytes());
      hasher.update(b"\x1d");
    }
    hasher.update(b"\x1c");
    hasher.update(fp.unified_diff.as_bytes());
  }
  let digest = hasher.finalize();
  let prefix = digest
    .iter()
    .take(8)
    .map(|b| format!("{b:02x}"))
    .collect::<String>();
  let id = format!("remove-unused-import-patch.{prefix}");

  let mut artifact = serde_json::json!({
    "id": id,
    "artifact_family": artifact_family,
    "source_surface": "code-transform.remove-unused-import",
    "stored_at_ms": stored_at_ms,
    "target_paths": [candidate.request.target_path],
    "command_refs": serde_json::Value::Array(Vec::new()),
    "related_refs": serde_json::json!([
      "owner-law:stdlib/lib/gate/code-transform/remove-unused-import.px"
    ]),
    "payload": payload,
  });
  if let Some(snap) = repo_snapshot_ref {
    artifact["repo_snapshot_ref"] = serde_json::Value::String(snap.to_string());
  }
  artifact
}

// ─── apply receipt ───────────────────────────────────────────────────

/// Trust seal: a `RemoveUnusedImportPatchCandidate` that has passed
/// the `verdict == RemoveUnusedImportReady` check.
///
/// OWNER-LAW (2026-05-11): only constructable through
/// [`ValidatedRemoveUnusedImportPatchCandidate::new_checked`]. Same
/// pattern as `ValidatedRenamePatchCandidate` — type-enforced
/// precondition that bypassing requires an explicit `unsafe` block
/// (and there isn't one). The apply path requires this newtype so
/// Held / Rejected candidates can never reach apply by accident.
#[derive(Debug, Clone)]
pub struct ValidatedRemoveUnusedImportPatchCandidate {
  candidate: RemoveUnusedImportPatchCandidate,
}

impl ValidatedRemoveUnusedImportPatchCandidate {
  /// Explicit checked constructor. `Ok(self)` only when
  /// `candidate.verdict == RemoveUnusedImportReady`; otherwise returns
  /// the candidate back so the caller can audit / log.
  pub fn new_checked(
    candidate: RemoveUnusedImportPatchCandidate,
  ) -> Result<Self, RemoveUnusedImportPatchCandidate> {
    if matches!(
      candidate.verdict,
      RemoveUnusedImportVerdict::RemoveUnusedImportReady
    ) {
      Ok(Self { candidate })
    } else {
      Err(candidate)
    }
  }

  pub fn candidate(&self) -> &RemoveUnusedImportPatchCandidate {
    &self.candidate
  }

  pub fn into_candidate(self) -> RemoveUnusedImportPatchCandidate {
    self.candidate
  }
}

/// An approval record for a remove-unused-import patch candidate.
///
/// OWNER-LAW (2026-05-11): apply MUST NOT happen without this. Same
/// shape as `RenameApplyApproval` — the canonical chain treats both
/// transforms identically at the approval step.
/// `candidate_artifact_id` is the replay-stable id from
/// [`build_remove_unused_import_patch_candidate_artifact`]; the apply
/// path re-computes it from the sealed candidate and refuses to
/// proceed when it doesn't match (TOCTOU defense between approval
/// time and apply time).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoveUnusedImportApplyApproval {
  pub actor_id: String,
  pub tenant_id: String,
  pub approved_at_ms: u64,
  pub candidate_artifact_id: String,
}

/// Lift a review receipt into a `RemoveUnusedImportApplyApproval`.
/// Returns `Some(approval)` only when `decision == Approve`; `Hold` /
/// `Reject` returns `None` (apply must not proceed from those).
///
/// OWNER-LAW (2026-05-11): only sanctioned bridge between the review
/// step and the apply step. The approval's `candidate_artifact_id` is
/// carried from the review receipt so the apply path's TOCTOU check
/// binds the entire chain review → approval → apply on the same
/// candidate identity.
pub fn approval_from_remove_unused_import_review(
  receipt: &RemoveUnusedImportReviewReceipt,
) -> Option<RemoveUnusedImportApplyApproval> {
  if !receipt.decision.permits_apply() {
    return None;
  }
  Some(RemoveUnusedImportApplyApproval {
    actor_id: receipt.reviewer.actor_id.clone(),
    tenant_id: receipt.reviewer.tenant_id.clone(),
    approved_at_ms: receipt.reviewed_at_ms,
    candidate_artifact_id: receipt.candidate_artifact_id.clone(),
  })
}

/// Build a `ToolActionMaterializationRequest` from a
/// remove-unused-import review receipt + apply receipt + context.
///
/// OWNER-LAW (2026-05-11): twin of
/// `code_transform::rename_symbol::build_rename_materialization_request`.
/// Delegates to the transform-agnostic
/// [`crate::tool_action::bridge_review_apply_to_materialization_request`]
/// after computing the remove-unused-import-specific artifact ids.
///
/// Same preconditions verified by the core bridge: review must be
/// Approve; review.candidate_artifact_id must match the apply's
/// derived candidate id (TOCTOU); review and apply must be in the
/// same tenant; the assembled request must classify Ready.
pub fn build_remove_unused_import_materialization_request(
  review: &RemoveUnusedImportReviewReceipt,
  apply: &RemoveUnusedImportApplyReceipt,
  capability: &str,
  repo_snapshot_ref: &str,
  deployment_mode: &str,
  content_policy: &str,
  requested_at_ms: u64,
) -> Result<
  crate::tool_action::ToolActionMaterializationRequest,
  crate::tool_action::MaterializationBridgeError,
> {
  let apply_candidate_art =
    build_remove_unused_import_patch_candidate_artifact(&apply.candidate, 0, None);
  let apply_candidate_id = apply_candidate_art
    .get("id")
    .and_then(|v| v.as_str())
    .unwrap_or("")
    .to_string();
  let apply_art = build_remove_unused_import_apply_receipt_artifact(
    apply,
    0,
    None,
    super::rename_symbol::ApplyReceiptContentPolicy::OmitContent,
  );
  let apply_receipt_id = apply_art
    .get("id")
    .and_then(|v| v.as_str())
    .unwrap_or("")
    .to_string();

  let review_decision_str = review.decision.as_str();
  crate::tool_action::bridge_review_apply_to_materialization_request(
    &crate::tool_action::MaterializationBridgeInputs {
      review_decision: review_decision_str,
      review_candidate_artifact_id: &review.candidate_artifact_id,
      review_reviewer_tenant_id: &review.reviewer.tenant_id,
      apply_candidate_artifact_id: &apply_candidate_id,
      apply_receipt_artifact_id: &apply_receipt_id,
      apply_approval_actor_id: &apply.approval.actor_id,
      apply_approval_tenant_id: &apply.approval.tenant_id,
      capability,
      repo_snapshot_ref,
      deployment_mode,
      content_policy,
      requested_at_ms,
    },
  )
}

/// The receipt of an applied remove-unused-import — pure data
/// describing the post-apply state plus an inverse diff for rollback.
///
/// OWNER-LAW (2026-05-11): the carrier produces this *value* but does
/// NOT write to disk. The downstream `ToolActionApproval` host
/// surface decides whether to materialize `per_file_after` onto the
/// filesystem. Keeping the carrier I/O-free preserves determinism and
/// replay.
///
/// Unlike rename-symbol's symmetric `-`/`+` swap, the inverse diff
/// here is *line-insertion*: removed lines are added back at the
/// recorded byte offsets, producing `@@ -L,0 +L,1 @@\n+<line>` hunks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoveUnusedImportApplyReceipt {
  pub candidate: RemoveUnusedImportPatchCandidate,
  pub approval: RemoveUnusedImportApplyApproval,
  pub applied_at_ms: u64,
  /// Per-file post-apply content: `(path, rewritten_content)`. Only
  /// files that actually had edits appear here.
  pub per_file_after: Vec<(String, String)>,
  /// Reverse unified diff — applying this to the post-apply content
  /// returns the original. Each removal hunk `@@ -L,1 +L,0 @@\n-foo`
  /// becomes its insertion inverse `@@ -L,0 +L,1 @@\n+foo`.
  pub inverse_unified_diff: String,
}

/// Errors from [`apply_remove_unused_import_patch_candidate`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoveUnusedImportApplyError {
  /// Approval's `candidate_artifact_id` does not match the sealed
  /// candidate's freshly-recomputed id (TOCTOU break).
  ApprovalCandidateIdMismatch { expected: String, got: String },
  /// `files` is missing a path that the sealed candidate names in its
  /// `file_patches`.
  MissingFileForPatch { path: String },
  /// `approval.actor_id` is empty. Apply cannot be anonymous.
  MissingApprovalActor,
  /// `approval.tenant_id` is empty. Apply cannot be tenant-less.
  MissingApprovalTenant,
}

impl std::fmt::Display for RemoveUnusedImportApplyError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::ApprovalCandidateIdMismatch { expected, got } => write!(
        f,
        "approval candidate_artifact_id mismatch: expected '{expected}', got '{got}'"
      ),
      Self::MissingFileForPatch { path } => write!(
        f,
        "apply input is missing file for sealed candidate path '{path}'"
      ),
      Self::MissingApprovalActor => write!(f, "approval.actor_id must be non-empty"),
      Self::MissingApprovalTenant => write!(f, "approval.tenant_id must be non-empty"),
    }
  }
}

impl std::error::Error for RemoveUnusedImportApplyError {}

/// Invert a remove-unused-import unified diff: each removal hunk
/// (`@@ -L,1 +L,0 @@\n-<line>`) becomes its insertion inverse
/// (`@@ -L,0 +L,1 @@\n+<line>`).
///
/// OWNER-LAW (2026-05-11): unlike rename-symbol's `-`/`+` body-line
/// swap (which leaves hunk anchors unchanged because rename is a 1↔1
/// substitution), removal hunks have asymmetric counts: 1-line → 0
/// lines forward, 0 lines → 1 line backward. The hunk anchor regex
/// `@@ -L,1 +L,0 @@` must flip to `@@ -L,0 +L,1 @@`.
fn invert_remove_unused_import_diff(forward: &str) -> String {
  if forward.is_empty() {
    return String::new();
  }
  let mut out = String::with_capacity(forward.len());
  for line in forward.split_inclusive('\n') {
    // Headers: swap a/ ↔ b/.
    if let Some(stripped) = line.strip_prefix("--- a/") {
      out.push_str("+++ b/");
      out.push_str(stripped);
      continue;
    }
    if let Some(stripped) = line.strip_prefix("+++ b/") {
      out.push_str("--- a/");
      out.push_str(stripped);
      continue;
    }
    // Hunk anchor: swap `@@ -L,N +L,M @@` to `@@ -L,M +L,N @@`.
    if line.starts_with("@@") {
      if let Some(inverted) = invert_hunk_anchor(line) {
        out.push_str(&inverted);
      } else {
        out.push_str(line);
      }
      continue;
    }
    // "No newline" marker — pass through.
    if line.starts_with("\\ No newline") {
      out.push_str(line);
      continue;
    }
    // Body line: swap leading `-` / `+`. Removal hunks have only
    // `-` body lines, so the inverse has only `+` body lines.
    if let Some(stripped) = line.strip_prefix('-') {
      out.push('+');
      out.push_str(stripped);
    } else if let Some(stripped) = line.strip_prefix('+') {
      out.push('-');
      out.push_str(stripped);
    } else {
      out.push_str(line);
    }
  }
  out
}

/// Parse `@@ -L,N +L,M @@\n` and re-render as `@@ -L,M +L,N @@\n`.
/// Returns `None` on unparseable lines so the caller can fall back to
/// pass-through (defensive — shouldn't happen for our well-formed
/// canonical diffs).
fn invert_hunk_anchor(line: &str) -> Option<String> {
  // Format: `@@ -<old_start>,<old_count> +<new_start>,<new_count> @@\n`
  let trailing_newline = line.ends_with('\n');
  let trimmed = line.strip_suffix('\n').unwrap_or(line);
  let inner = trimmed.strip_prefix("@@ ")?.strip_suffix(" @@")?;
  // `inner` should look like `-<old_start>,<old_count> +<new_start>,<new_count>`.
  let (old_part, new_part) = inner.split_once(' ')?;
  let old_part = old_part.strip_prefix('-')?;
  let new_part = new_part.strip_prefix('+')?;
  let (old_start, old_count) = old_part.split_once(',')?;
  let (new_start, new_count) = new_part.split_once(',')?;
  // Re-render with old/new start swapped to keep anchor location AND
  // counts swapped.
  let mut out = format!("@@ -{new_start},{new_count} +{old_start},{old_count} @@");
  if trailing_newline {
    out.push('\n');
  }
  Some(out)
}

/// Apply a sealed remove-unused-import patch candidate, producing a
/// receipt with post-apply content and inverse diff for rollback.
///
/// OWNER-LAW (2026-05-11): preconditions verified in order:
///   1. `approval.actor_id` / `tenant_id` non-empty.
///   2. `approval.candidate_artifact_id` equals the sealed
///      candidate's freshly-recomputed id (TOCTOU defense).
///   3. Every path in `sealed.file_patches` is present in `files`.
///
/// On success returns `Ok(RemoveUnusedImportApplyReceipt)`. The
/// receipt is pure data; the host writes `per_file_after` to disk
/// under its own `ToolActionApproval` audit lane.
pub fn apply_remove_unused_import_patch_candidate(
  sealed: &ValidatedRemoveUnusedImportPatchCandidate,
  files: &[RemoveUnusedImportFileInput<'_>],
  approval: &RemoveUnusedImportApplyApproval,
  applied_at_ms: u64,
) -> Result<RemoveUnusedImportApplyReceipt, RemoveUnusedImportApplyError> {
  // 1. auth claim shape
  if approval.actor_id.is_empty() {
    return Err(RemoveUnusedImportApplyError::MissingApprovalActor);
  }
  if approval.tenant_id.is_empty() {
    return Err(RemoveUnusedImportApplyError::MissingApprovalTenant);
  }

  let candidate = sealed.candidate();

  // 2. TOCTOU: re-derive the candidate's artifact id and compare.
  let recomputed = build_remove_unused_import_patch_candidate_artifact(candidate, 0, None);
  let recomputed_id = recomputed
    .get("id")
    .and_then(|v| v.as_str())
    .unwrap_or("")
    .to_string();
  if recomputed_id != approval.candidate_artifact_id {
    return Err(RemoveUnusedImportApplyError::ApprovalCandidateIdMismatch {
      expected: approval.candidate_artifact_id.clone(),
      got: recomputed_id,
    });
  }

  // 3. every patch path must be staged.
  let mut file_content: std::collections::BTreeMap<&str, &str> = std::collections::BTreeMap::new();
  for f in files {
    file_content.insert(f.path, f.content);
  }
  let mut per_file_after: Vec<(String, String)> = Vec::new();
  for fp in &candidate.file_patches {
    let original = file_content.get(fp.path.as_str()).ok_or_else(|| {
      RemoveUnusedImportApplyError::MissingFileForPatch {
        path: fp.path.clone(),
      }
    })?;
    let rewritten = apply_remove_import_edits(original, &fp.edits);
    per_file_after.push((fp.path.clone(), rewritten));
  }

  // 4. inverse diff: removal → insertion.
  let inverse_unified_diff = invert_remove_unused_import_diff(&candidate.combined_unified_diff);

  Ok(RemoveUnusedImportApplyReceipt {
    candidate: candidate.clone(),
    approval: approval.clone(),
    applied_at_ms,
    per_file_after,
    inverse_unified_diff,
  })
}

/// Render a `RemoveUnusedImportApplyReceipt` as the JSON payload of a
/// `coding.code-transform.remove-unused-import-apply-receipt`
/// artifact.
///
/// OWNER-LAW (2026-05-11): same content-policy gate as rename-symbol —
/// `IncludeContent` embeds full post-apply file content for dev /
/// debug; `OmitContent` keeps only `path` + `content_sha256` +
/// `byte_len` for customer-release safety. Uses the shared
/// `ApplyReceiptContentPolicy` from the rename-symbol module since
/// the policy concept is identical across transforms.
pub fn build_remove_unused_import_apply_receipt_payload(
  receipt: &RemoveUnusedImportApplyReceipt,
  content_policy: super::rename_symbol::ApplyReceiptContentPolicy,
) -> serde_json::Value {
  let candidate_art =
    build_remove_unused_import_patch_candidate_artifact(&receipt.candidate, 0, None);
  let candidate_artifact_id = candidate_art
    .get("id")
    .and_then(|v| v.as_str())
    .unwrap_or("")
    .to_string();
  let files_after: Vec<serde_json::Value> = receipt
    .per_file_after
    .iter()
    .map(|(path, content)| {
      let mut hasher = Sha256::new();
      hasher.update(content.as_bytes());
      let digest = hasher.finalize();
      let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
      let mut entry = serde_json::json!({
        "path": path,
        "content_sha256": hex,
        "byte_len": content.len(),
      });
      if matches!(
        content_policy,
        super::rename_symbol::ApplyReceiptContentPolicy::IncludeContent
      ) {
        entry["content"] = serde_json::Value::String(content.clone());
      }
      entry
    })
    .collect();
  serde_json::json!({
    "transform": "remove-unused-import",
    "owner_law": "stdlib/lib/gate/code-transform/remove-unused-import.px",
    "candidate_artifact_id": candidate_artifact_id,
    "approval": {
      "actor_id": receipt.approval.actor_id,
      "tenant_id": receipt.approval.tenant_id,
      "approved_at_ms": receipt.approval.approved_at_ms,
    },
    "applied_at_ms": receipt.applied_at_ms,
    "target_paths": [receipt.candidate.request.target_path.clone()],
    "content_policy": content_policy.as_str(),
    "files_after": files_after,
    "inverse_unified_diff": receipt.inverse_unified_diff,
    "rollback_available": !receipt.inverse_unified_diff.is_empty(),
    "next_step": "verify-or-rollback",
  })
}

/// Wrap a `RemoveUnusedImportApplyReceipt` into a full
/// `coding.code-transform.remove-unused-import-apply-receipt`
/// artifact value with a replay-stable id.
///
/// OWNER-LAW (2026-05-11): same strengthened hash as rename-symbol's
/// apply-receipt id — binds candidate id + approval triple + per-file
/// (path + post-apply sha256) + inverse diff bytes. `stored_at_ms`
/// and `content_policy` are extrinsic to event identity.
pub fn build_remove_unused_import_apply_receipt_artifact(
  receipt: &RemoveUnusedImportApplyReceipt,
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
  content_policy: super::rename_symbol::ApplyReceiptContentPolicy,
) -> serde_json::Value {
  let payload = build_remove_unused_import_apply_receipt_payload(receipt, content_policy);
  let candidate_artifact_id = payload
    .get("candidate_artifact_id")
    .and_then(|v| v.as_str())
    .unwrap_or("");

  let mut hasher = Sha256::new();
  hasher.update(b"remove-unused-import-apply\x1f");
  hasher.update(candidate_artifact_id.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(receipt.approval.actor_id.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(receipt.approval.tenant_id.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(receipt.approval.approved_at_ms.to_le_bytes());
  hasher.update(b"\x1f");
  hasher.update(receipt.applied_at_ms.to_le_bytes());
  hasher.update(b"\x1f");
  for (path, content) in &receipt.per_file_after {
    hasher.update(path.as_bytes());
    hasher.update(b"\x1e");
    let mut file_hasher = Sha256::new();
    file_hasher.update(content.as_bytes());
    let file_digest = file_hasher.finalize();
    hasher.update(file_digest);
    hasher.update(b"\x1d");
  }
  hasher.update(b"\x1f");
  hasher.update(receipt.inverse_unified_diff.as_bytes());
  let digest = hasher.finalize();
  let prefix = digest
    .iter()
    .take(8)
    .map(|b| format!("{b:02x}"))
    .collect::<String>();
  let id = format!("apply-receipt.remove-unused-import.{prefix}");

  let mut artifact = serde_json::json!({
    "id": id,
    "artifact_family": "coding.code-transform.remove-unused-import-apply-receipt",
    "source_surface": "code-transform.remove-unused-import",
    "stored_at_ms": stored_at_ms,
    "target_paths": [receipt.candidate.request.target_path.clone()],
    "command_refs": serde_json::Value::Array(Vec::new()),
    "related_refs": serde_json::json!([
      "owner-law:stdlib/lib/gate/code-transform/remove-unused-import.px",
      format!("candidate-artifact:{candidate_artifact_id}")
    ]),
    "payload": payload,
  });
  if let Some(snap) = repo_snapshot_ref {
    artifact["repo_snapshot_ref"] = serde_json::Value::String(snap.to_string());
  }
  artifact
}

// ─── rollback handle ──────────────────────────────────────────────────

/// Initiator identity for a rollback handle.
///
/// OWNER-LAW (2026-05-11): same shape as
/// `RenameRollbackInitiator` — the canonical chain treats both
/// transforms identically at the rollback-intent step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoveUnusedImportRollbackInitiator {
  pub actor_id: String,
  pub tenant_id: String,
}

/// A rollback handle for an applied `remove-unused-import`.
///
/// OWNER-LAW (2026-05-11): handle is the *intent* step in the
/// canonical chain:
///
///   candidate → review → apply → ROLLBACK HANDLE (this) → rollback receipt
///
/// The handle pins both the candidate and the apply-receipt by id so
/// audit can walk the chain. The `inverse_unified_diff` from the
/// apply receipt is the rollback patch (line-insertion form for
/// remove-unused-import — distinct from rename-symbol's symmetric
/// `-` ↔ `+` swap).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoveUnusedImportRollbackHandle {
  pub apply_receipt: RemoveUnusedImportApplyReceipt,
  pub initiator: RemoveUnusedImportRollbackInitiator,
  pub reason: Option<String>,
  pub initiated_at_ms: u64,
  /// Replay-stable candidate artifact id, pinned at handle time.
  pub candidate_artifact_id: String,
  /// Replay-stable apply-receipt artifact id, pinned at handle time.
  /// The future executor binds rollback to this exact apply event.
  pub apply_receipt_artifact_id: String,
}

/// Build a `RemoveUnusedImportRollbackHandle` from an apply receipt +
/// initiator. Pins both replay-stable ids at handle time.
///
/// OWNER-LAW (2026-05-11): only constructor — caller can't make a
/// handle without an actual apply receipt and initiator identity.
/// Both ids are recomputed here (canonical hashes) so the handle is
/// self-contained for audit / replay.
pub fn build_remove_unused_import_rollback_handle(
  apply_receipt: RemoveUnusedImportApplyReceipt,
  initiator: RemoveUnusedImportRollbackInitiator,
  reason: Option<String>,
  initiated_at_ms: u64,
) -> RemoveUnusedImportRollbackHandle {
  let candidate_art =
    build_remove_unused_import_patch_candidate_artifact(&apply_receipt.candidate, 0, None);
  let candidate_artifact_id = candidate_art
    .get("id")
    .and_then(|v| v.as_str())
    .unwrap_or("")
    .to_string();
  let apply_art = build_remove_unused_import_apply_receipt_artifact(
    &apply_receipt,
    0,
    None,
    super::rename_symbol::ApplyReceiptContentPolicy::OmitContent,
  );
  let apply_receipt_artifact_id = apply_art
    .get("id")
    .and_then(|v| v.as_str())
    .unwrap_or("")
    .to_string();
  RemoveUnusedImportRollbackHandle {
    apply_receipt,
    initiator,
    reason,
    initiated_at_ms,
    candidate_artifact_id,
    apply_receipt_artifact_id,
  }
}

/// Render a `RemoveUnusedImportRollbackHandle` as the canonical JSON
/// payload of a `coding.rollback-handle` artifact.
pub fn build_remove_unused_import_rollback_handle_payload(
  handle: &RemoveUnusedImportRollbackHandle,
) -> serde_json::Value {
  let inverse_diff = &handle.apply_receipt.inverse_unified_diff;
  let mut payload = serde_json::json!({
    "transform": "remove-unused-import",
    "owner_law": "stdlib/lib/gate/code-transform/remove-unused-import.px",
    "candidate_artifact_id": handle.candidate_artifact_id,
    "apply_receipt_artifact_id": handle.apply_receipt_artifact_id,
    "initiator": {
      "actor_id": handle.initiator.actor_id,
      "tenant_id": handle.initiator.tenant_id,
    },
    "initiated_at_ms": handle.initiated_at_ms,
    "rollback_state": "handle-issued",
    "inverse_unified_diff": inverse_diff,
    "rollback_available": !inverse_diff.is_empty(),
    "next_step": "execute-rollback",
  });
  payload["reason"] = match handle.reason.as_ref() {
    Some(r) => serde_json::Value::String(r.clone()),
    None => serde_json::Value::Null,
  };
  payload
}

/// Wrap a `RemoveUnusedImportRollbackHandle` into a full
/// `coding.rollback-handle` artifact value with a replay-stable
/// id.
///
/// OWNER-LAW (2026-05-11): id hash binds intrinsic handle identity:
///   1. `apply_receipt_artifact_id` (which apply event to roll back)
///   2. `initiator.actor_id` / `tenant_id`
///   3. `initiated_at_ms`
///   4. `reason` (when present — distinct rollback intents on the
///      same apply with different reasoning are different handles)
///
/// `stored_at_ms` is extrinsic. `related_refs` carries DUAL back-refs
/// (`candidate-artifact:<id>` + `apply-receipt-artifact:<id>`).
pub fn build_remove_unused_import_rollback_handle_artifact(
  handle: &RemoveUnusedImportRollbackHandle,
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
) -> serde_json::Value {
  let payload = build_remove_unused_import_rollback_handle_payload(handle);
  let mut hasher = Sha256::new();
  hasher.update(b"remove-unused-import-rollback-handle\x1f");
  hasher.update(handle.apply_receipt_artifact_id.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(handle.initiator.actor_id.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(handle.initiator.tenant_id.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(handle.initiated_at_ms.to_le_bytes());
  hasher.update(b"\x1f");
  if let Some(r) = handle.reason.as_ref() {
    hasher.update(r.as_bytes());
  }
  let digest = hasher.finalize();
  let prefix = digest
    .iter()
    .take(8)
    .map(|b| format!("{b:02x}"))
    .collect::<String>();
  let id = format!("rollback-handle.remove-unused-import.{prefix}");

  let mut artifact = serde_json::json!({
    "id": id,
    "artifact_family": "coding.rollback-handle",
    "source_surface": "code-transform.remove-unused-import",
    "stored_at_ms": stored_at_ms,
    "target_paths": [handle.apply_receipt.candidate.request.target_path.clone()],
    "command_refs": serde_json::Value::Array(Vec::new()),
    "related_refs": serde_json::json!([
      "owner-law:stdlib/lib/gate/code-transform/remove-unused-import.px",
      format!("candidate-artifact:{}", handle.candidate_artifact_id),
      format!("apply-receipt-artifact:{}", handle.apply_receipt_artifact_id),
    ]),
    "payload": payload,
  });
  if let Some(snap) = repo_snapshot_ref {
    artifact["repo_snapshot_ref"] = serde_json::Value::String(snap.to_string());
  }
  artifact
}

// ─── rollback receipt ─────────────────────────────────────────────────

/// Executor identity for a rollback receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoveUnusedImportRollbackExecutor {
  pub actor_id: String,
  pub tenant_id: String,
}

/// The receipt of an executed rollback — pure data describing the
/// post-rollback state.
///
/// OWNER-LAW (2026-05-11): the carrier produces this *value* but does
/// NOT write to disk. The downstream `ToolActionApproval` host gate
/// decides whether to materialize `per_file_after_rollback`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoveUnusedImportRollbackReceipt {
  pub handle: RemoveUnusedImportRollbackHandle,
  pub executor: RemoveUnusedImportRollbackExecutor,
  pub executed_at_ms: u64,
  /// Per-file post-rollback content: `(path, restored_content)`.
  pub per_file_after_rollback: Vec<(String, String)>,
  /// Replay-stable rollback-handle artifact id, pinned at receipt
  /// time so a downstream actor can audit the chain.
  pub rollback_handle_artifact_id: String,
}

/// Errors from [`execute_remove_unused_import_rollback`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoveUnusedImportRollbackError {
  MissingExecutorActor,
  MissingExecutorTenant,
  MissingFileForRollback {
    path: String,
  },
  /// The current on-disk file's `content_sha256` doesn't match the
  /// apply-receipt's recorded post-apply sha256. Someone hand-edited
  /// the file between apply and rollback — refuse to clobber.
  PostApplyDriftDetected {
    path: String,
    expected_sha256: String,
    found_sha256: String,
  },
}

impl std::fmt::Display for RemoveUnusedImportRollbackError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::MissingExecutorActor => write!(f, "executor.actor_id must be non-empty"),
      Self::MissingExecutorTenant => write!(f, "executor.tenant_id must be non-empty"),
      Self::MissingFileForRollback { path } => {
        write!(f, "rollback input missing file for handle path '{path}'")
      }
      Self::PostApplyDriftDetected {
        path,
        expected_sha256,
        found_sha256,
      } => write!(
        f,
        "post-apply drift detected for '{path}': expected sha256 '{expected_sha256}', found '{found_sha256}'"
      ),
    }
  }
}

impl std::error::Error for RemoveUnusedImportRollbackError {}

/// Reverse the line-deletion edits: insert each `removed_line_content`
/// back at its recorded `byte_offset` in the original content.
///
/// OWNER-LAW (2026-05-11): the apply pass strips byte ranges out of
/// the original content. Rollback inserts those bytes back at the
/// ORIGINAL offsets walked in ascending order — since each insertion
/// shifts later content forward by exactly the same amount the
/// corresponding deletion shifted it back, the result is the
/// original content byte-for-byte.
///
/// Caller must pass `edits` sorted by ascending `byte_offset` (which
/// `compute_remove_unused_import_edits` guarantees). The
/// `removed_line_content` is the line WITHOUT trailing newline; this
/// function re-adds the newline when the recorded `byte_len`
/// indicates one was present in the original.
pub fn reverse_remove_unused_import_edits(post_apply: &str, edits: &[RemoveImportEdit]) -> String {
  let post_bytes = post_apply.as_bytes();
  let total_inserted: usize = edits.iter().map(|e| e.byte_len).sum();
  let mut out = String::with_capacity(post_apply.len() + total_inserted);
  let mut original_offset = 0usize;
  let mut post_offset = 0usize;
  for e in edits {
    // Copy bytes in `post_apply` up to where this edit's ORIGINAL
    // offset would land. The number of bytes is `e.byte_offset -
    // original_offset` (in the virtual original-content frame); the
    // bytes are present at `post_offset..post_offset + N` in
    // post_apply since we haven't restored anything yet that would
    // shift them.
    let bytes_in_between = e.byte_offset.saturating_sub(original_offset);
    if bytes_in_between > 0 {
      let end = post_offset + bytes_in_between;
      let slice = post_bytes.get(post_offset..end).unwrap_or(&[]);
      out.push_str(
        std::str::from_utf8(slice)
          .unwrap_or_else(|_| panic!("non-utf8 input to reverse_remove_unused_import_edits")),
      );
      post_offset = end;
    }
    out.push_str(&e.removed_line_content);
    // Re-add trailing newline if the recorded byte_len was greater
    // than the trimmed content (i.e. the original line had `\n`).
    if e.byte_len > e.removed_line_content.len() {
      out.push('\n');
    }
    original_offset = e.byte_offset + e.byte_len;
  }
  // Append tail of post_apply (everything after the last edit's
  // post_offset).
  if let Some(tail) = post_bytes.get(post_offset..) {
    if !tail.is_empty() {
      out.push_str(
        std::str::from_utf8(tail)
          .unwrap_or_else(|_| panic!("non-utf8 tail in reverse_remove_unused_import_edits")),
      );
    }
  }
  out
}

fn sha256_hex(bytes: &[u8]) -> String {
  let mut hasher = Sha256::new();
  hasher.update(bytes);
  let digest = hasher.finalize();
  digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// Execute a remove-unused-import rollback, producing a receipt with
/// post-rollback content.
///
/// OWNER-LAW (2026-05-11): preconditions verified in order:
///
///   1. `executor.actor_id` and `executor.tenant_id` non-empty.
///   2. Every path in the handle's `apply_receipt.candidate.file_patches`
///      is present in `current_files`.
///   3. Each current file's sha256 matches the apply-receipt's
///      recorded post-apply sha256 (drift defense — if someone
///      hand-edited the file between apply and rollback, the
///      rollback would clobber third-party edits).
///
/// On success returns `Ok(RemoveUnusedImportRollbackReceipt)`.
pub fn execute_remove_unused_import_rollback(
  handle: &RemoveUnusedImportRollbackHandle,
  current_files: &[RemoveUnusedImportFileInput<'_>],
  executor: &RemoveUnusedImportRollbackExecutor,
  executed_at_ms: u64,
) -> Result<RemoveUnusedImportRollbackReceipt, RemoveUnusedImportRollbackError> {
  if executor.actor_id.is_empty() {
    return Err(RemoveUnusedImportRollbackError::MissingExecutorActor);
  }
  if executor.tenant_id.is_empty() {
    return Err(RemoveUnusedImportRollbackError::MissingExecutorTenant);
  }

  // Build a path → content map of the current on-disk state.
  let mut current: std::collections::BTreeMap<&str, &str> = std::collections::BTreeMap::new();
  for f in current_files {
    current.insert(f.path, f.content);
  }

  let apply_receipt = &handle.apply_receipt;
  let candidate = &apply_receipt.candidate;
  let mut per_file_after_rollback: Vec<(String, String)> = Vec::new();

  for fp in &candidate.file_patches {
    let current_content = current.get(fp.path.as_str()).ok_or_else(|| {
      RemoveUnusedImportRollbackError::MissingFileForRollback {
        path: fp.path.clone(),
      }
    })?;

    // Drift defense: compare current sha256 to apply-receipt's
    // recorded post-apply sha256.
    let current_sha = sha256_hex(current_content.as_bytes());
    // Find the expected sha from apply_receipt.per_file_after.
    let expected_content = apply_receipt
      .per_file_after
      .iter()
      .find(|(p, _)| p == &fp.path)
      .map(|(_, c)| c.as_str())
      .unwrap_or("");
    let expected_sha = sha256_hex(expected_content.as_bytes());
    if current_sha != expected_sha {
      return Err(RemoveUnusedImportRollbackError::PostApplyDriftDetected {
        path: fp.path.clone(),
        expected_sha256: expected_sha,
        found_sha256: current_sha,
      });
    }

    // Reverse the line-deletions: insert the removed lines back.
    let restored = reverse_remove_unused_import_edits(current_content, &fp.edits);
    per_file_after_rollback.push((fp.path.clone(), restored));
  }

  // Recompute rollback-handle artifact id for the receipt's back-ref.
  let handle_art = build_remove_unused_import_rollback_handle_artifact(handle, 0, None);
  let rollback_handle_artifact_id = handle_art
    .get("id")
    .and_then(|v| v.as_str())
    .unwrap_or("")
    .to_string();

  Ok(RemoveUnusedImportRollbackReceipt {
    handle: handle.clone(),
    executor: executor.clone(),
    executed_at_ms,
    per_file_after_rollback,
    rollback_handle_artifact_id,
  })
}

/// Render a `RemoveUnusedImportRollbackReceipt` as the canonical JSON
/// payload of a `coding.rollback-receipt` artifact.
pub fn build_remove_unused_import_rollback_receipt_payload(
  receipt: &RemoveUnusedImportRollbackReceipt,
  content_policy: super::rename_symbol::ApplyReceiptContentPolicy,
) -> serde_json::Value {
  let files_after: Vec<serde_json::Value> = receipt
    .per_file_after_rollback
    .iter()
    .map(|(path, content)| {
      let sha = sha256_hex(content.as_bytes());
      let mut entry = serde_json::json!({
        "path": path,
        "content_sha256": sha,
        "byte_len": content.len(),
      });
      if matches!(
        content_policy,
        super::rename_symbol::ApplyReceiptContentPolicy::IncludeContent
      ) {
        entry["content"] = serde_json::Value::String(content.clone());
      }
      entry
    })
    .collect();
  serde_json::json!({
    "transform": "remove-unused-import",
    "owner_law": "stdlib/lib/gate/code-transform/remove-unused-import.px",
    "candidate_artifact_id": receipt.handle.candidate_artifact_id,
    "apply_receipt_artifact_id": receipt.handle.apply_receipt_artifact_id,
    "rollback_handle_artifact_id": receipt.rollback_handle_artifact_id,
    "executor": {
      "actor_id": receipt.executor.actor_id,
      "tenant_id": receipt.executor.tenant_id,
    },
    "executed_at_ms": receipt.executed_at_ms,
    "rollback_state": "executed",
    "content_policy": content_policy.as_str(),
    "files_after_rollback": files_after,
    "target_paths": [receipt.handle.apply_receipt.candidate.request.target_path.clone()],
    "next_step": "verify-rollback-or-redo-apply",
  })
}

/// Wrap a `RemoveUnusedImportRollbackReceipt` into a full
/// `coding.rollback-receipt` artifact value with a replay-stable
/// id.
///
/// OWNER-LAW (2026-05-11): id hash binds intrinsic rollback identity:
///   1. `rollback_handle_artifact_id`
///   2. `executor.actor_id` / `tenant_id`
///   3. `executed_at_ms`
///   4. per-file `(path, post-rollback content sha256)`
///
/// `stored_at_ms` and `content_policy` are extrinsic. `related_refs`
/// carries TRIPLE back-refs (candidate + apply-receipt + rollback-
/// handle).
pub fn build_remove_unused_import_rollback_receipt_artifact(
  receipt: &RemoveUnusedImportRollbackReceipt,
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
  content_policy: super::rename_symbol::ApplyReceiptContentPolicy,
) -> serde_json::Value {
  let payload = build_remove_unused_import_rollback_receipt_payload(receipt, content_policy);
  let mut hasher = Sha256::new();
  hasher.update(b"remove-unused-import-rollback-receipt\x1f");
  hasher.update(receipt.rollback_handle_artifact_id.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(receipt.executor.actor_id.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(receipt.executor.tenant_id.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(receipt.executed_at_ms.to_le_bytes());
  hasher.update(b"\x1f");
  for (path, content) in &receipt.per_file_after_rollback {
    hasher.update(path.as_bytes());
    hasher.update(b"\x1e");
    let mut file_hasher = Sha256::new();
    file_hasher.update(content.as_bytes());
    let file_digest = file_hasher.finalize();
    hasher.update(file_digest);
    hasher.update(b"\x1d");
  }
  let digest = hasher.finalize();
  let prefix = digest
    .iter()
    .take(8)
    .map(|b| format!("{b:02x}"))
    .collect::<String>();
  let id = format!("rollback-receipt.remove-unused-import.{prefix}");

  let mut artifact = serde_json::json!({
    "id": id,
    "artifact_family": "coding.rollback-receipt",
    "source_surface": "code-transform.remove-unused-import",
    "stored_at_ms": stored_at_ms,
    "target_paths": [receipt.handle.apply_receipt.candidate.request.target_path.clone()],
    "command_refs": serde_json::Value::Array(Vec::new()),
    "related_refs": serde_json::json!([
      "owner-law:stdlib/lib/gate/code-transform/remove-unused-import.px",
      format!("candidate-artifact:{}", receipt.handle.candidate_artifact_id),
      format!("apply-receipt-artifact:{}", receipt.handle.apply_receipt_artifact_id),
      format!("rollback-handle-artifact:{}", receipt.rollback_handle_artifact_id),
    ]),
    "payload": payload,
  });
  if let Some(snap) = repo_snapshot_ref {
    artifact["repo_snapshot_ref"] = serde_json::Value::String(snap.to_string());
  }
  artifact
}

// ─── review receipt ───────────────────────────────────────────────────

/// Reviewer identity for a `RemoveUnusedImportPatchCandidate`.
///
/// OWNER-LAW (2026-05-11): same shape as the rename-symbol reviewer —
/// the canonical chain treats both transforms identically at the
/// review step. Per-transform types preserve module decoupling so
/// each transform's vertical slice is self-contained.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoveUnusedImportReviewer {
  pub actor_id: String,
  pub tenant_id: String,
}

/// Reviewer's decision on a `RemoveUnusedImportPatchCandidate`.
///
/// OWNER-LAW (2026-05-11): three outcomes mirroring rename-symbol:
///   - `Approve` — caller authorizes apply.
///   - `Hold` — caller wants more evidence / context before deciding.
///     Candidate is *not* rejected, just deferred.
///   - `Reject` — caller refuses the candidate. The receipt still
///     records the decision so future cognition / audit can avoid
///     re-deriving the same candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RemoveUnusedImportReviewDecision {
  Approve,
  Hold,
  Reject,
}

impl RemoveUnusedImportReviewDecision {
  pub fn as_str(self) -> &'static str {
    match self {
      Self::Approve => "approve",
      Self::Hold => "hold",
      Self::Reject => "reject",
    }
  }

  /// True only for `Approve`. The future apply path will gate
  /// approval construction on this predicate.
  pub fn permits_apply(self) -> bool {
    matches!(self, Self::Approve)
  }
}

/// The receipt of a review decision on a
/// `RemoveUnusedImportPatchCandidate`.
///
/// OWNER-LAW (2026-05-11): emitted *before* apply in the canonical
/// chain:
///
///   candidate → REVIEW RECEIPT (this) → apply → APPLY RECEIPT
///
/// The receipt embeds the candidate, the reviewer identity, the
/// decision, an optional human-readable reason, and the review
/// timestamp. The `candidate_artifact_id` is pinned at review time —
/// any future apply step that builds an approval from this receipt
/// must match this id (TOCTOU guard).
///
/// Only `Approve` reviews are downstream-actionable. `Hold` and
/// `Reject` are terminal for the apply chain but stay in the audit
/// graph as evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoveUnusedImportReviewReceipt {
  pub candidate: RemoveUnusedImportPatchCandidate,
  pub reviewer: RemoveUnusedImportReviewer,
  pub decision: RemoveUnusedImportReviewDecision,
  pub reason: Option<String>,
  pub reviewed_at_ms: u64,
  /// Replay-stable candidate artifact id, pinned at review time.
  pub candidate_artifact_id: String,
}

/// Build a `RemoveUnusedImportReviewReceipt` from a candidate +
/// reviewer decision. Pins the candidate's replay-stable artifact id
/// at review time.
///
/// OWNER-LAW (2026-05-11): only constructor — caller can't make a
/// review receipt without an actual candidate and reviewer identity.
/// The `candidate_artifact_id` is recomputed here (canonical hash) so
/// the receipt is self-contained for audit / replay.
pub fn build_remove_unused_import_review_receipt(
  candidate: RemoveUnusedImportPatchCandidate,
  reviewer: RemoveUnusedImportReviewer,
  decision: RemoveUnusedImportReviewDecision,
  reason: Option<String>,
  reviewed_at_ms: u64,
) -> RemoveUnusedImportReviewReceipt {
  let art = build_remove_unused_import_patch_candidate_artifact(&candidate, 0, None);
  let candidate_artifact_id = art
    .get("id")
    .and_then(|v| v.as_str())
    .unwrap_or("")
    .to_string();
  RemoveUnusedImportReviewReceipt {
    candidate,
    reviewer,
    decision,
    reason,
    reviewed_at_ms,
    candidate_artifact_id,
  }
}

/// Render a `RemoveUnusedImportReviewReceipt` as the canonical JSON
/// payload of a `coding.code-transform.remove-unused-import-review-receipt`
/// artifact.
///
/// OWNER-LAW (2026-05-11): payload shape mirrors the rename-symbol
/// review receipt; `transform` and `owner_law` fields distinguish
/// the two.
pub fn build_remove_unused_import_review_receipt_payload(
  receipt: &RemoveUnusedImportReviewReceipt,
) -> serde_json::Value {
  let next_step = match receipt.decision {
    RemoveUnusedImportReviewDecision::Approve => "apply",
    RemoveUnusedImportReviewDecision::Hold => "wait-for-evidence",
    RemoveUnusedImportReviewDecision::Reject => "rejected",
  };
  let mut payload = serde_json::json!({
    "transform": "remove-unused-import",
    "owner_law": "stdlib/lib/gate/code-transform/remove-unused-import.px",
    "candidate_artifact_id": receipt.candidate_artifact_id,
    "reviewer": {
      "actor_id": receipt.reviewer.actor_id,
      "tenant_id": receipt.reviewer.tenant_id,
    },
    "decision": receipt.decision.as_str(),
    "reviewed_at_ms": receipt.reviewed_at_ms,
    "permits_apply": receipt.decision.permits_apply(),
    "next_step": next_step,
  });
  payload["reason"] = match receipt.reason.as_ref() {
    Some(r) => serde_json::Value::String(r.clone()),
    None => serde_json::Value::Null,
  };
  payload
}

/// Wrap a `RemoveUnusedImportReviewReceipt` into a full
/// `coding.code-transform.remove-unused-import-review-receipt`
/// artifact value with a replay-stable id.
///
/// OWNER-LAW (2026-05-11): id hash binds intrinsic review identity:
///   1. `candidate_artifact_id`
///   2. `reviewer.actor_id` / `tenant_id`
///   3. `decision`
///   4. `reviewed_at_ms`
///   5. `reason` (when present)
///
/// `stored_at_ms` and `repo_snapshot_ref` are extrinsic — not in the
/// hash. `related_refs` carries `candidate-artifact:<id>` so audit can
/// walk the chain candidate → review-receipt → (future) apply-receipt.
pub fn build_remove_unused_import_review_receipt_artifact(
  receipt: &RemoveUnusedImportReviewReceipt,
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
) -> serde_json::Value {
  let payload = build_remove_unused_import_review_receipt_payload(receipt);
  let mut hasher = Sha256::new();
  hasher.update(b"remove-unused-import-review\x1f");
  hasher.update(receipt.candidate_artifact_id.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(receipt.reviewer.actor_id.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(receipt.reviewer.tenant_id.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(receipt.decision.as_str().as_bytes());
  hasher.update(b"\x1f");
  hasher.update(receipt.reviewed_at_ms.to_le_bytes());
  hasher.update(b"\x1f");
  if let Some(r) = receipt.reason.as_ref() {
    hasher.update(r.as_bytes());
  }
  let digest = hasher.finalize();
  let prefix = digest
    .iter()
    .take(8)
    .map(|b| format!("{b:02x}"))
    .collect::<String>();
  let id = format!("review-receipt.remove-unused-import.{prefix}");

  let mut artifact = serde_json::json!({
    "id": id,
    "artifact_family": "coding.code-transform.remove-unused-import-review-receipt",
    "source_surface": "code-transform.remove-unused-import",
    "stored_at_ms": stored_at_ms,
    "target_paths": [receipt.candidate.request.target_path.clone()],
    "command_refs": serde_json::Value::Array(Vec::new()),
    "related_refs": serde_json::json!([
      "owner-law:stdlib/lib/gate/code-transform/remove-unused-import.px",
      format!("candidate-artifact:{}", receipt.candidate_artifact_id),
    ]),
    "payload": payload,
  });
  if let Some(snap) = repo_snapshot_ref {
    artifact["repo_snapshot_ref"] = serde_json::Value::String(snap.to_string());
  }
  artifact
}

// ─── tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;

  fn req(
    target_path: &str,
    language: &str,
    candidates: Vec<UnusedImportCandidate>,
    scope: RemoveUnusedImportScope,
  ) -> RemoveUnusedImportRequest {
    RemoveUnusedImportRequest {
      target_path: target_path.to_string(),
      language: language.to_string(),
      candidate_imports: candidates,
      scope,
    }
  }

  fn cand(module: &str) -> UnusedImportCandidate {
    UnusedImportCandidate {
      module: module.to_string(),
      used_in_macro: false,
      behind_cfg: false,
    }
  }

  #[test]
  fn classify_returns_ready_for_well_formed_rust_request() {
    let r = req(
      "src/a.rs",
      "rust",
      vec![cand("std::collections::HashMap")],
      RemoveUnusedImportScope::SingleFile,
    );
    assert!(matches!(
      classify_remove_unused_import(&r),
      RemoveUnusedImportVerdict::RemoveUnusedImportReady
    ));
  }

  #[test]
  fn classify_holds_on_missing_target_path() {
    let r = req(
      "",
      "rust",
      vec![cand("foo")],
      RemoveUnusedImportScope::SingleFile,
    );
    match classify_remove_unused_import(&r) {
      RemoveUnusedImportVerdict::RemoveUnusedImportHeld { held_kind, .. } => {
        assert_eq!(held_kind, RemoveUnusedImportHeldKind::MissingTargetPath);
      }
      other => panic!("expected MissingTargetPath, got {:?}", other),
    }
  }

  #[test]
  fn classify_holds_on_parent_traversal_in_path() {
    let r = req(
      "../escape.rs",
      "rust",
      vec![cand("foo")],
      RemoveUnusedImportScope::SingleFile,
    );
    match classify_remove_unused_import(&r) {
      RemoveUnusedImportVerdict::RemoveUnusedImportHeld { held_kind, .. } => {
        assert_eq!(
          held_kind,
          RemoveUnusedImportHeldKind::TargetPathOutOfProject
        );
      }
      other => panic!("expected TargetPathOutOfProject, got {:?}", other),
    }
  }

  #[test]
  fn classify_holds_on_unsupported_language() {
    let r = req(
      "src/a.f90",
      "fortran",
      vec![cand("foo")],
      RemoveUnusedImportScope::SingleFile,
    );
    match classify_remove_unused_import(&r) {
      RemoveUnusedImportVerdict::RemoveUnusedImportHeld { held_kind, .. } => {
        assert_eq!(held_kind, RemoveUnusedImportHeldKind::LanguageNotSupported);
      }
      other => panic!("expected LanguageNotSupported, got {:?}", other),
    }
  }

  #[test]
  fn classify_holds_on_missing_candidates() {
    let r = req(
      "src/a.rs",
      "rust",
      vec![],
      RemoveUnusedImportScope::SingleFile,
    );
    match classify_remove_unused_import(&r) {
      RemoveUnusedImportVerdict::RemoveUnusedImportHeld { held_kind, .. } => {
        assert_eq!(held_kind, RemoveUnusedImportHeldKind::MissingCandidates);
      }
      other => panic!("expected MissingCandidates, got {:?}", other),
    }
  }

  #[test]
  fn classify_holds_when_candidate_used_in_macro() {
    let macro_bound = UnusedImportCandidate {
      module: "serde::Serialize".to_string(),
      used_in_macro: true,
      behind_cfg: false,
    };
    let r = req(
      "src/a.rs",
      "rust",
      vec![macro_bound],
      RemoveUnusedImportScope::SingleFile,
    );
    match classify_remove_unused_import(&r) {
      RemoveUnusedImportVerdict::RemoveUnusedImportHeld { held_kind, .. } => {
        assert_eq!(held_kind, RemoveUnusedImportHeldKind::MacroBinding);
      }
      other => panic!("expected MacroBinding, got {:?}", other),
    }
  }

  #[test]
  fn classify_holds_when_candidate_behind_cfg() {
    let cfg_bound = UnusedImportCandidate {
      module: "windows_only::Foo".to_string(),
      used_in_macro: false,
      behind_cfg: true,
    };
    let r = req(
      "src/a.rs",
      "rust",
      vec![cfg_bound],
      RemoveUnusedImportScope::SingleFile,
    );
    match classify_remove_unused_import(&r) {
      RemoveUnusedImportVerdict::RemoveUnusedImportHeld { held_kind, .. } => {
        assert_eq!(held_kind, RemoveUnusedImportHeldKind::MacroBinding);
      }
      other => panic!("expected MacroBinding (covers cfg too), got {:?}", other),
    }
  }

  #[test]
  fn classify_holds_on_crate_wide_scope() {
    let r = req(
      "src/a.rs",
      "rust",
      vec![cand("foo")],
      RemoveUnusedImportScope::CrateWide,
    );
    match classify_remove_unused_import(&r) {
      RemoveUnusedImportVerdict::RemoveUnusedImportHeld { held_kind, .. } => {
        assert_eq!(held_kind, RemoveUnusedImportHeldKind::ScopeTooBroad);
      }
      other => panic!("expected ScopeTooBroad, got {:?}", other),
    }
  }

  #[test]
  fn classify_holds_on_test_file_with_single_file_scope() {
    let r = req(
      "tests/integration_test.rs",
      "rust",
      vec![cand("foo")],
      RemoveUnusedImportScope::SingleFile,
    );
    match classify_remove_unused_import(&r) {
      RemoveUnusedImportVerdict::RemoveUnusedImportHeld { held_kind, .. } => {
        assert_eq!(held_kind, RemoveUnusedImportHeldKind::TestFileProtected);
      }
      other => panic!("expected TestFileProtected, got {:?}", other),
    }
  }

  #[test]
  fn classify_allows_test_file_with_tests_also_scope() {
    let r = req(
      "tests/integration_test.rs",
      "rust",
      vec![cand("foo")],
      RemoveUnusedImportScope::TestsAlso,
    );
    assert!(matches!(
      classify_remove_unused_import(&r),
      RemoveUnusedImportVerdict::RemoveUnusedImportReady
    ));
  }

  #[test]
  fn classify_detects_spec_test_files() {
    // `.spec.ts` is the JavaScript / TypeScript test convention.
    let r = req(
      "src/foo.spec.ts",
      "typescript",
      vec![cand("Bar")],
      RemoveUnusedImportScope::SingleFile,
    );
    match classify_remove_unused_import(&r) {
      RemoveUnusedImportVerdict::RemoveUnusedImportHeld { held_kind, .. } => {
        assert_eq!(held_kind, RemoveUnusedImportHeldKind::TestFileProtected);
      }
      other => panic!("expected TestFileProtected for .spec.ts, got {:?}", other),
    }
  }

  #[test]
  fn classify_detects_go_test_files() {
    // `foo_test.go` is the Go test convention.
    let r = req(
      "src/foo_test.go",
      "go",
      vec![cand("strings")],
      RemoveUnusedImportScope::SingleFile,
    );
    match classify_remove_unused_import(&r) {
      RemoveUnusedImportVerdict::RemoveUnusedImportHeld { held_kind, .. } => {
        assert_eq!(held_kind, RemoveUnusedImportHeldKind::TestFileProtected);
      }
      other => panic!("expected TestFileProtected for _test.go, got {:?}", other),
    }
  }

  #[test]
  fn classify_does_not_falsely_flag_non_test_paths() {
    // `src/testimony.rs` should not be flagged as a test file even
    // though it contains "test" as a substring.
    let r = req(
      "src/testimony.rs",
      "rust",
      vec![cand("foo")],
      RemoveUnusedImportScope::SingleFile,
    );
    assert!(matches!(
      classify_remove_unused_import(&r),
      RemoveUnusedImportVerdict::RemoveUnusedImportReady
    ));
  }

  #[test]
  fn classify_request_shape_wins_over_macro_binding() {
    // Even with a macro-bound candidate, if target_path is missing
    // the verdict is MissingTargetPath (request-shape ladder runs first).
    let macro_bound = UnusedImportCandidate {
      module: "serde::Serialize".to_string(),
      used_in_macro: true,
      behind_cfg: false,
    };
    let r = req(
      "",
      "rust",
      vec![macro_bound],
      RemoveUnusedImportScope::SingleFile,
    );
    match classify_remove_unused_import(&r) {
      RemoveUnusedImportVerdict::RemoveUnusedImportHeld { held_kind, .. } => {
        assert_eq!(held_kind, RemoveUnusedImportHeldKind::MissingTargetPath);
      }
      other => panic!("expected MissingTargetPath, got {:?}", other),
    }
  }

  // ─── edit detection (find_unused_import_line / compute / apply) ──

  #[test]
  fn find_rust_use_line_finds_simple_import() {
    let content = "use std::collections::HashMap;\nfn main() {}\n";
    let edit =
      find_unused_import_line(content, "std::collections::HashMap", "rust").expect("found");
    assert_eq!(edit.byte_offset, 0);
    assert_eq!(edit.byte_len, "use std::collections::HashMap;\n".len());
    assert_eq!(edit.line, 1);
    assert_eq!(edit.removed_line_content, "use std::collections::HashMap;");
    assert_eq!(edit.module, "std::collections::HashMap");
  }

  #[test]
  fn find_rust_use_line_finds_mid_file_import() {
    let content = "// header\nuse std::path::PathBuf;\nfn main() {}\n";
    let edit = find_unused_import_line(content, "std::path::PathBuf", "rust").expect("found");
    assert_eq!(edit.line, 2);
    assert_eq!(edit.byte_offset, "// header\n".len());
    // Reconstruction sanity: byte_offset..byte_offset+byte_len == the use line.
    let bytes = content.as_bytes();
    let slice = &bytes[edit.byte_offset..edit.byte_offset + edit.byte_len];
    assert_eq!(slice, b"use std::path::PathBuf;\n");
  }

  #[test]
  fn find_rust_use_line_handles_leading_whitespace() {
    // Imports indented inside a module block.
    let content = "mod inner {\n    use std::io::Read;\n}\n";
    let edit = find_unused_import_line(content, "std::io::Read", "rust").expect("found");
    assert_eq!(edit.line, 2);
    // Whole line including leading whitespace is removed.
    assert!(edit.removed_line_content.starts_with("    use "));
  }

  #[test]
  fn find_rust_use_line_returns_none_when_no_match() {
    let content = "fn main() {}\n";
    assert!(find_unused_import_line(content, "std::io::Read", "rust").is_none());
  }

  #[test]
  fn find_rust_use_line_strict_match_avoids_false_positives() {
    // `use std::io;` should not match a request for `std::io::Read`.
    let content = "use std::io;\nfn main() {}\n";
    assert!(find_unused_import_line(content, "std::io::Read", "rust").is_none());
  }

  #[test]
  fn find_rust_use_line_handles_no_trailing_newline_on_last_line() {
    // Final line is `use foo::Bar;` without trailing newline (e.g.
    // file ends mid-import — unusual but possible).
    let content = "fn main() {}\nuse foo::Bar;";
    let edit = find_unused_import_line(content, "foo::Bar", "rust").expect("found");
    assert_eq!(edit.line, 2);
    // No newline on the last line.
    assert_eq!(edit.byte_len, "use foo::Bar;".len());
  }

  #[test]
  fn find_unused_import_line_returns_none_for_fully_unsupported_languages() {
    // Languages outside the classifier's accept set return None
    // (e.g. fortran, brainfuck — never reached by the classifier
    // anyway because the classifier holds them first).
    let f90 = "use foo\n";
    assert!(find_unused_import_line(f90, "foo", "fortran").is_none());
  }

  // ─── per-language edit-walk tests ───────────────────────────────

  // -- Python --

  #[test]
  fn python_finds_simple_import() {
    let content = "import os.path\nprint(123)\n";
    let edit = find_unused_import_line(content, "os.path", "python").expect("found");
    assert_eq!(edit.line, 1);
    assert_eq!(edit.byte_offset, 0);
    assert_eq!(edit.removed_line_content, "import os.path");
    assert_eq!(edit.byte_len, "import os.path\n".len());
  }

  #[test]
  fn python_finds_from_import() {
    // `from os import path` should match `module = "os.path"`.
    let content = "from os import path\nprint(123)\n";
    let edit = find_unused_import_line(content, "os.path", "python").expect("found");
    assert_eq!(edit.line, 1);
    assert_eq!(edit.removed_line_content, "from os import path");
  }

  #[test]
  fn python_finds_from_import_with_nested_pkg() {
    // `from collections.abc import Iterable` matches `module = "collections.abc.Iterable"`.
    let content = "from collections.abc import Iterable\nx = 1\n";
    let edit =
      find_unused_import_line(content, "collections.abc.Iterable", "python").expect("found");
    assert_eq!(edit.line, 1);
  }

  #[test]
  fn python_handles_leading_whitespace() {
    let content = "def f():\n    import json\n    return 1\n";
    let edit = find_unused_import_line(content, "json", "python").expect("found");
    assert_eq!(edit.line, 2);
    assert!(edit.removed_line_content.starts_with("    import "));
  }

  #[test]
  fn python_does_not_match_multi_import() {
    // `from os import path, getcwd` — multi-import — NOT matched in phase 1.
    let content = "from os import path, getcwd\n";
    assert!(find_unused_import_line(content, "os.path", "python").is_none());
    assert!(find_unused_import_line(content, "os.getcwd", "python").is_none());
  }

  #[test]
  fn python_does_not_match_aliased_import() {
    // `import numpy as np` — aliased — NOT matched in phase 1.
    let content = "import numpy as np\n";
    assert!(find_unused_import_line(content, "numpy", "python").is_none());
    // `from os import path as p` — aliased from-import — NOT matched.
    let content2 = "from os import path as p\n";
    assert!(find_unused_import_line(content2, "os.path", "python").is_none());
  }

  #[test]
  fn python_does_not_match_partial_module() {
    // `import os` does not match `module = "os.path"`.
    let content = "import os\n";
    assert!(find_unused_import_line(content, "os.path", "python").is_none());
  }

  // -- TypeScript / JavaScript --

  #[test]
  fn typescript_finds_default_import() {
    let content = "import Foo from './foo';\nconst x = new Foo();\n";
    let edit = find_unused_import_line(content, "Foo", "typescript").expect("found");
    assert_eq!(edit.line, 1);
    assert_eq!(edit.removed_line_content, "import Foo from './foo';");
  }

  #[test]
  fn typescript_finds_named_single_import() {
    let content = "import { Bar } from 'bar-pkg';\nconst x: Bar = null;\n";
    let edit = find_unused_import_line(content, "Bar", "typescript").expect("found");
    assert_eq!(edit.line, 1);
    assert_eq!(edit.removed_line_content, "import { Bar } from 'bar-pkg';");
  }

  #[test]
  fn typescript_accepts_double_quoted_spec() {
    let content = "import Foo from \"./foo\";\n";
    let edit = find_unused_import_line(content, "Foo", "typescript").expect("found");
    assert_eq!(edit.line, 1);
  }

  #[test]
  fn typescript_accepts_named_import_tight_braces() {
    // `import {Foo} from '...'` — no whitespace inside braces.
    let content = "import {Foo} from './foo';\n";
    let edit = find_unused_import_line(content, "Foo", "typescript").expect("found");
    assert_eq!(edit.line, 1);
  }

  #[test]
  fn typescript_accepts_no_trailing_semicolon() {
    // TS / JS allow omitting `;` (with ASI).
    let content = "import Foo from './foo'\nconst x = Foo\n";
    let edit = find_unused_import_line(content, "Foo", "typescript").expect("found");
    assert_eq!(edit.line, 1);
  }

  #[test]
  fn typescript_does_not_match_multi_named_import() {
    // `import { Foo, Bar } from '...'` — multi — NOT matched.
    let content = "import { Foo, Bar } from './foo';\n";
    assert!(find_unused_import_line(content, "Foo", "typescript").is_none());
    assert!(find_unused_import_line(content, "Bar", "typescript").is_none());
  }

  #[test]
  fn typescript_does_not_match_namespace_import() {
    // `import * as Foo from '...'` — NOT matched.
    let content = "import * as Foo from './foo';\n";
    assert!(find_unused_import_line(content, "Foo", "typescript").is_none());
  }

  #[test]
  fn typescript_does_not_match_side_effect_import() {
    // `import './side-effect';` — no identifier to match.
    let content = "import './side-effect';\n";
    assert!(find_unused_import_line(content, "Foo", "typescript").is_none());
  }

  #[test]
  fn typescript_does_not_match_non_import_line() {
    let content = "const Foo = 1;\nlet Bar = Foo;\n";
    assert!(find_unused_import_line(content, "Foo", "typescript").is_none());
  }

  #[test]
  fn javascript_uses_typescript_predicate() {
    // JS is treated the same as TS (ES6 imports). Sanity: a default
    // import detected the same way.
    let content = "import Foo from './foo';\n";
    let edit = find_unused_import_line(content, "Foo", "javascript").expect("found");
    assert_eq!(edit.line, 1);
  }

  // -- Go --

  #[test]
  fn go_finds_simple_import() {
    let content = "package main\n\nimport \"fmt\"\n\nfunc main() {}\n";
    let edit = find_unused_import_line(content, "fmt", "go").expect("found");
    assert_eq!(edit.line, 3);
    assert_eq!(edit.removed_line_content, "import \"fmt\"");
  }

  #[test]
  fn go_finds_nested_pkg_path() {
    let content = "import \"net/http\"\nfunc main() {}\n";
    let edit = find_unused_import_line(content, "net/http", "go").expect("found");
    assert_eq!(edit.line, 1);
  }

  #[test]
  fn go_does_not_match_block_import() {
    // Block imports span multiple lines — NOT matched in phase 1.
    let content = "import (\n\t\"fmt\"\n\t\"os\"\n)\n";
    assert!(find_unused_import_line(content, "fmt", "go").is_none());
    assert!(find_unused_import_line(content, "os", "go").is_none());
  }

  #[test]
  fn go_does_not_match_aliased_import() {
    // `import myalias "pkg/path"` — aliased — NOT matched in phase 1.
    let content = "import myfmt \"fmt\"\n";
    assert!(find_unused_import_line(content, "fmt", "go").is_none());
    assert!(find_unused_import_line(content, "myfmt", "go").is_none());
  }

  // -- compute_edits + apply integration across languages --

  #[test]
  fn compute_edits_python_round_trip_via_apply() {
    let content = "import os.path\nfrom collections import OrderedDict\nx = 1\n";
    let candidates = vec![
      UnusedImportCandidate {
        module: "os.path".to_string(),
        used_in_macro: false,
        behind_cfg: false,
      },
      UnusedImportCandidate {
        module: "collections.OrderedDict".to_string(),
        used_in_macro: false,
        behind_cfg: false,
      },
    ];
    let edits = compute_remove_unused_import_edits(content, &candidates, "python");
    assert_eq!(edits.len(), 2);
    let after = apply_remove_import_edits(content, &edits);
    assert_eq!(after, "x = 1\n");
  }

  #[test]
  fn compute_edits_typescript_strips_default_and_named() {
    let content = "import Foo from './foo';\nimport { Bar } from './bar';\nconst x = 1;\n";
    let candidates = vec![
      UnusedImportCandidate {
        module: "Foo".to_string(),
        used_in_macro: false,
        behind_cfg: false,
      },
      UnusedImportCandidate {
        module: "Bar".to_string(),
        used_in_macro: false,
        behind_cfg: false,
      },
    ];
    let edits = compute_remove_unused_import_edits(content, &candidates, "typescript");
    assert_eq!(edits.len(), 2);
    let after = apply_remove_import_edits(content, &edits);
    assert_eq!(after, "const x = 1;\n");
  }

  #[test]
  fn compute_edits_go_strips_simple_import() {
    let content = "package main\n\nimport \"fmt\"\nimport \"os\"\n\nfunc main() {}\n";
    let candidates = vec![
      UnusedImportCandidate {
        module: "fmt".to_string(),
        used_in_macro: false,
        behind_cfg: false,
      },
      UnusedImportCandidate {
        module: "os".to_string(),
        used_in_macro: false,
        behind_cfg: false,
      },
    ];
    let edits = compute_remove_unused_import_edits(content, &candidates, "go");
    assert_eq!(edits.len(), 2);
    let after = apply_remove_import_edits(content, &edits);
    assert_eq!(after, "package main\n\n\nfunc main() {}\n");
  }

  #[test]
  fn rust_finder_still_works_after_refactor() {
    // Sanity: the Rust path uses the new generic walker via
    // matches_rust_use_line. Pre-existing Rust behavior is preserved.
    let content = "use std::io::Read;\nfn main() {}\n";
    let edit = find_unused_import_line(content, "std::io::Read", "rust").expect("found");
    assert_eq!(edit.line, 1);
    assert_eq!(edit.removed_line_content, "use std::io::Read;");
    // Strict match — partial paths don't match.
    assert!(find_unused_import_line(content, "std::io", "rust").is_none());
  }

  #[test]
  fn compute_edits_sorts_by_offset_and_dedups() {
    // Three candidates, two valid (one duplicate); expect 2 sorted
    // edits.
    let content = "use std::path::PathBuf;\nuse std::io::Read;\nfn main() {}\n";
    let candidates = vec![
      UnusedImportCandidate {
        module: "std::io::Read".to_string(),
        used_in_macro: false,
        behind_cfg: false,
      },
      UnusedImportCandidate {
        module: "std::path::PathBuf".to_string(),
        used_in_macro: false,
        behind_cfg: false,
      },
      // Duplicate of PathBuf — should collapse to one edit.
      UnusedImportCandidate {
        module: "std::path::PathBuf".to_string(),
        used_in_macro: false,
        behind_cfg: false,
      },
      // Unknown to the file — silently skipped.
      UnusedImportCandidate {
        module: "std::collections::HashMap".to_string(),
        used_in_macro: false,
        behind_cfg: false,
      },
    ];
    let edits = compute_remove_unused_import_edits(content, &candidates, "rust");
    assert_eq!(edits.len(), 2, "two unique matches, one skipped");
    assert!(edits[0].byte_offset < edits[1].byte_offset, "sorted");
    assert_eq!(edits[0].module, "std::path::PathBuf");
    assert_eq!(edits[1].module, "std::io::Read");
  }

  #[test]
  fn apply_remove_import_edits_strips_target_lines() {
    let content = "use std::path::PathBuf;\nuse std::io::Read;\nfn main() {}\n";
    let candidates = vec![
      UnusedImportCandidate {
        module: "std::path::PathBuf".to_string(),
        used_in_macro: false,
        behind_cfg: false,
      },
      UnusedImportCandidate {
        module: "std::io::Read".to_string(),
        used_in_macro: false,
        behind_cfg: false,
      },
    ];
    let edits = compute_remove_unused_import_edits(content, &candidates, "rust");
    let rewritten = apply_remove_import_edits(content, &edits);
    assert_eq!(rewritten, "fn main() {}\n");
  }

  #[test]
  fn apply_preserves_non_matching_lines() {
    let content =
      "// top comment\nuse std::path::PathBuf;\n\nfn main() {\n    println!(\"hi\");\n}\n";
    let candidates = vec![UnusedImportCandidate {
      module: "std::path::PathBuf".to_string(),
      used_in_macro: false,
      behind_cfg: false,
    }];
    let edits = compute_remove_unused_import_edits(content, &candidates, "rust");
    let rewritten = apply_remove_import_edits(content, &edits);
    // Only the use line removed; everything else preserved verbatim.
    assert_eq!(
      rewritten,
      "// top comment\n\nfn main() {\n    println!(\"hi\");\n}\n"
    );
  }

  #[test]
  fn round_trip_compute_then_apply_then_recompute_finds_nothing() {
    // Strong invariant: after applying remove-edits, recomputing
    // edits for the same candidates yields zero results.
    let content = "use std::path::PathBuf;\nuse std::io::Read;\nfn main() {}\n";
    let candidates = vec![
      UnusedImportCandidate {
        module: "std::path::PathBuf".to_string(),
        used_in_macro: false,
        behind_cfg: false,
      },
      UnusedImportCandidate {
        module: "std::io::Read".to_string(),
        used_in_macro: false,
        behind_cfg: false,
      },
    ];
    let edits = compute_remove_unused_import_edits(content, &candidates, "rust");
    let rewritten = apply_remove_import_edits(content, &edits);
    let post_edits = compute_remove_unused_import_edits(&rewritten, &candidates, "rust");
    assert!(
      post_edits.is_empty(),
      "post-rewrite, removed imports must not be found again"
    );
  }

  #[test]
  fn compute_edits_returns_empty_for_truly_unsupported_language() {
    // Languages outside the classifier's accept set (e.g. fortran)
    // produce no edits. (Python / TS / JS / Go are NOW supported as
    // of the multi-language slice — see python_*, typescript_*,
    // go_* tests above.)
    let content = "use foo\n";
    let candidates = vec![UnusedImportCandidate {
      module: "foo".to_string(),
      used_in_macro: false,
      behind_cfg: false,
    }];
    let edits = compute_remove_unused_import_edits(content, &candidates, "fortran");
    assert!(edits.is_empty(), "fortran is not in the supported set");
  }

  // ─── patch candidate + artifact ──────────────────────────────────

  fn ready_request_fixture() -> RemoveUnusedImportRequest {
    RemoveUnusedImportRequest {
      target_path: "src/a.rs".to_string(),
      language: "rust".to_string(),
      candidate_imports: vec![
        UnusedImportCandidate {
          module: "std::path::PathBuf".to_string(),
          used_in_macro: false,
          behind_cfg: false,
        },
        UnusedImportCandidate {
          module: "std::io::Read".to_string(),
          used_in_macro: false,
          behind_cfg: false,
        },
      ],
      scope: RemoveUnusedImportScope::SingleFile,
    }
  }

  const FIXTURE_FILE_CONTENT: &str = "use std::path::PathBuf;\nuse std::io::Read;\nfn main() {}\n";

  fn ready_file_input<'a>() -> RemoveUnusedImportFileInput<'a> {
    RemoveUnusedImportFileInput {
      path: "src/a.rs",
      content: FIXTURE_FILE_CONTENT,
    }
  }

  #[test]
  fn compute_patch_candidate_happy_path() {
    let req = ready_request_fixture();
    let candidate = compute_remove_unused_import_patch_candidate(&req, &ready_file_input());
    assert!(matches!(
      candidate.verdict,
      RemoveUnusedImportVerdict::RemoveUnusedImportReady
    ));
    assert_eq!(candidate.file_patches.len(), 1);
    assert_eq!(candidate.file_patches[0].path, "src/a.rs");
    assert_eq!(candidate.file_patches[0].edits.len(), 2);
    assert!(!candidate.combined_unified_diff.is_empty());
    assert!(candidate.combined_unified_diff.contains("--- a/src/a.rs"));
  }

  #[test]
  fn compute_patch_candidate_held_yields_empty_patches() {
    let mut req = ready_request_fixture();
    // Make request Held by clearing candidates → MissingCandidates.
    req.candidate_imports.clear();
    let candidate = compute_remove_unused_import_patch_candidate(&req, &ready_file_input());
    assert!(matches!(
      candidate.verdict,
      RemoveUnusedImportVerdict::RemoveUnusedImportHeld { .. }
    ));
    assert!(candidate.file_patches.is_empty());
    assert!(candidate.combined_unified_diff.is_empty());
  }

  #[test]
  fn compute_patch_candidate_ignores_file_outside_target_path() {
    // SAFETY: same target-bounded principle as rename-symbol — files
    // outside `request.target_path` must NEVER produce edits.
    let req = ready_request_fixture();
    let stray = RemoveUnusedImportFileInput {
      path: "src/secret.rs",
      content: FIXTURE_FILE_CONTENT,
    };
    let candidate = compute_remove_unused_import_patch_candidate(&req, &stray);
    assert!(matches!(
      candidate.verdict,
      RemoveUnusedImportVerdict::RemoveUnusedImportReady
    ));
    // Verdict is Ready but no edits — the caller staged the wrong file.
    assert!(candidate.file_patches.is_empty());
    assert!(candidate.combined_unified_diff.is_empty());
  }

  #[test]
  fn render_unified_diff_emits_minus_only_hunks() {
    let edits = vec![RemoveImportEdit {
      byte_offset: 0,
      byte_len: "use std::path::PathBuf;\n".len(),
      line: 1,
      removed_line_content: "use std::path::PathBuf;".to_string(),
      module: "std::path::PathBuf".to_string(),
    }];
    let diff = render_unified_diff_for_remove_unused_import(
      "src/a.rs",
      "use std::path::PathBuf;\nfn main() {}\n",
      &edits,
    );
    assert!(diff.contains("--- a/src/a.rs"));
    assert!(diff.contains("+++ b/src/a.rs"));
    // 1-line hunk: -line, no +line counterpart.
    assert!(diff.contains("@@ -1,1 +1,0 @@"));
    assert!(diff.contains("-use std::path::PathBuf;"));
    // No `+` body line.
    let plus_body_lines: Vec<&str> = diff
      .lines()
      .filter(|l| l.starts_with('+') && !l.starts_with("+++"))
      .collect();
    assert_eq!(plus_body_lines.len(), 0, "no + body lines for remove-only");
  }

  #[test]
  fn render_unified_diff_handles_no_trailing_newline_on_removed_line() {
    let content = "use foo::Bar;"; // no trailing newline
    let edits = vec![RemoveImportEdit {
      byte_offset: 0,
      byte_len: content.len(),
      line: 1,
      removed_line_content: "use foo::Bar;".to_string(),
      module: "foo::Bar".to_string(),
    }];
    let diff = render_unified_diff_for_remove_unused_import("a.rs", content, &edits);
    assert!(diff.contains("\\ No newline at end of file"));
  }

  #[test]
  fn render_unified_diff_empty_on_zero_edits() {
    let diff = render_unified_diff_for_remove_unused_import("a.rs", "let x = 1;\n", &[]);
    assert!(diff.is_empty());
  }

  #[test]
  fn patch_candidate_payload_canonical_fields() {
    let req = ready_request_fixture();
    let candidate = compute_remove_unused_import_patch_candidate(&req, &ready_file_input());
    let payload = build_remove_unused_import_patch_candidate_payload(&candidate);
    assert_eq!(payload["transform"].as_str(), Some("remove-unused-import"));
    assert_eq!(
      payload["owner_law"].as_str(),
      Some("stdlib/lib/gate/code-transform/remove-unused-import.px")
    );
    assert_eq!(payload["target_path"].as_str(), Some("src/a.rs"));
    assert_eq!(payload["language"].as_str(), Some("rust"));
    assert_eq!(payload["scope"].as_str(), Some("single-file"));
    assert_eq!(
      payload["verdict"].as_str(),
      Some("remove-unused-import-ready")
    );
    assert_eq!(payload["candidate_only"].as_bool(), Some(true));
    assert_eq!(
      payload["next_step"].as_str(),
      Some("host-unused-import-walk-then-tool-action-approval")
    );
    let edits = payload["edits"].as_array().expect("edits");
    assert_eq!(edits.len(), 2);
    // Each edit names the module it targets — audit traceability.
    let modules: Vec<&str> = edits.iter().filter_map(|e| e["module"].as_str()).collect();
    assert!(modules.contains(&"std::path::PathBuf"));
    assert!(modules.contains(&"std::io::Read"));
    assert!(payload["unified_diff"]
      .as_str()
      .expect("unified_diff")
      .contains("--- a/src/a.rs"));
  }

  #[test]
  fn patch_candidate_payload_held_carries_held_kind_and_reason() {
    let mut req = ready_request_fixture();
    req.candidate_imports.clear();
    let candidate = compute_remove_unused_import_patch_candidate(&req, &ready_file_input());
    let payload = build_remove_unused_import_patch_candidate_payload(&candidate);
    assert_eq!(
      payload["verdict"].as_str(),
      Some("remove-unused-import-held")
    );
    assert_eq!(payload["held_kind"].as_str(), Some("missing-candidates"));
    assert!(payload["reason"].as_str().unwrap().contains("candidates"));
    assert_eq!(payload["edits"].as_array().unwrap().len(), 0);
    assert_eq!(payload["unified_diff"].as_str(), Some(""));
  }

  #[test]
  fn patch_candidate_artifact_envelope_shape_ready() {
    let req = ready_request_fixture();
    let candidate = compute_remove_unused_import_patch_candidate(&req, &ready_file_input());
    let art = build_remove_unused_import_patch_candidate_artifact(&candidate, 1700000000000, None);
    assert_eq!(
      art["artifact_family"].as_str(),
      Some("coding.code-transform.remove-unused-import-ready")
    );
    assert_eq!(
      art["source_surface"].as_str(),
      Some("code-transform.remove-unused-import")
    );
    assert_eq!(art["stored_at_ms"].as_u64(), Some(1700000000000));
    let id = art["id"].as_str().expect("id");
    assert!(id.starts_with("remove-unused-import-patch."));
    let related = art["related_refs"].as_array().expect("related_refs");
    assert!(related.iter().any(|v| v
      .as_str()
      .map(|s| s == "owner-law:stdlib/lib/gate/code-transform/remove-unused-import.px")
      .unwrap_or(false)));
    assert_eq!(art["target_paths"][0].as_str(), Some("src/a.rs"));
    assert_eq!(
      art["payload"]["transform"].as_str(),
      Some("remove-unused-import")
    );
  }

  #[test]
  fn patch_candidate_artifact_family_changes_with_verdict() {
    // Ready, Held, Rejected verdicts → different artifact families
    // (matching the `.px` `buildReceipt` family suffix).

    // Ready
    let req = ready_request_fixture();
    let ready_cand = compute_remove_unused_import_patch_candidate(&req, &ready_file_input());
    let ready_art = build_remove_unused_import_patch_candidate_artifact(&ready_cand, 0, None);
    assert_eq!(
      ready_art["artifact_family"].as_str(),
      Some("coding.code-transform.remove-unused-import-ready")
    );

    // Held: missing-candidates
    let mut held_req = ready_request_fixture();
    held_req.candidate_imports.clear();
    let held_cand = compute_remove_unused_import_patch_candidate(&held_req, &ready_file_input());
    let held_art = build_remove_unused_import_patch_candidate_artifact(&held_cand, 0, None);
    assert_eq!(
      held_art["artifact_family"].as_str(),
      Some("coding.code-transform.remove-unused-import-held")
    );
  }

  #[test]
  fn patch_candidate_artifact_id_replay_stable() {
    let req = ready_request_fixture();
    let candidate = compute_remove_unused_import_patch_candidate(&req, &ready_file_input());
    let a = build_remove_unused_import_patch_candidate_artifact(&candidate, 1000, None);
    let b = build_remove_unused_import_patch_candidate_artifact(&candidate, 9999, None);
    assert_eq!(a["id"], b["id"], "stored_at_ms is extrinsic");
    assert_ne!(a["stored_at_ms"], b["stored_at_ms"]);
  }

  #[test]
  fn patch_candidate_artifact_id_differs_per_candidate_imports() {
    // Same request shape, but different candidate_imports → different
    // identity. Audit can distinguish a one-import removal from a
    // two-import removal even when other fields match.
    let mut req_one = ready_request_fixture();
    req_one.candidate_imports.truncate(1);
    let cand_one = compute_remove_unused_import_patch_candidate(&req_one, &ready_file_input());

    let req_two = ready_request_fixture();
    let cand_two = compute_remove_unused_import_patch_candidate(&req_two, &ready_file_input());

    let a = build_remove_unused_import_patch_candidate_artifact(&cand_one, 0, None);
    let b = build_remove_unused_import_patch_candidate_artifact(&cand_two, 0, None);
    assert_ne!(a["id"], b["id"]);
  }

  #[test]
  fn patch_candidate_artifact_includes_repo_snapshot_ref_when_provided() {
    let req = ready_request_fixture();
    let candidate = compute_remove_unused_import_patch_candidate(&req, &ready_file_input());
    let art =
      build_remove_unused_import_patch_candidate_artifact(&candidate, 0, Some("commit-rmi-1"));
    assert_eq!(art["repo_snapshot_ref"].as_str(), Some("commit-rmi-1"));
  }

  #[test]
  fn helper_predicates() {
    assert!(is_supported_language("rust"));
    assert!(is_supported_language("python"));
    assert!(!is_supported_language("fortran"));

    assert!(is_path_in_project("src/a.rs"));
    assert!(!is_path_in_project(""));
    assert!(!is_path_in_project("../escape.rs"));

    // Test file detection — mirrors `.px` regex `.*[/_]tests?[/_].*`
    // which REQUIRES a `/` or `_` boundary BEFORE `tests`/`test`. So
    // `tests/integration.rs` (leading `tests` with no preceding
    // boundary) does NOT match unless it has `_test.<ext>` suffix.
    assert!(is_test_path("crates/foo/tests/foo_test.rs"));
    assert!(is_test_path("crates/foo/tests/integration.rs")); // /tests/ boundary
    assert!(is_test_path("src/foo.spec.ts"));
    assert!(is_test_path("src/foo_test.go"));
    assert!(is_test_path("tests/integration_test.rs")); // _test.<ext> suffix
                                                        // Negative: bare `tests/` prefix without an inner boundary doesn't
                                                        // match the .px regex — owner-law parity, not common sense.
    assert!(!is_test_path("src/testimony.rs"));
    assert!(!is_test_path("src/protest.rs"));
    assert!(!is_test_path("src/contests.rs"));

    // Macro/cfg detection
    let plain = UnusedImportCandidate {
      module: "foo".to_string(),
      used_in_macro: false,
      behind_cfg: false,
    };
    let macro_used = UnusedImportCandidate {
      module: "foo".to_string(),
      used_in_macro: true,
      behind_cfg: false,
    };
    let cfg_behind = UnusedImportCandidate {
      module: "foo".to_string(),
      used_in_macro: false,
      behind_cfg: true,
    };
    assert!(!any_macro_or_cfg(&[plain.clone()]));
    assert!(any_macro_or_cfg(&[macro_used]));
    assert!(any_macro_or_cfg(&[cfg_behind]));
    assert!(any_macro_or_cfg(&[
      plain.clone(),
      UnusedImportCandidate {
        module: "bar".to_string(),
        used_in_macro: true,
        behind_cfg: false,
      }
    ]));
  }

  // ─── review receipt ──────────────────────────────────────────────

  fn fixture_reviewer() -> RemoveUnusedImportReviewer {
    RemoveUnusedImportReviewer {
      actor_id: "actor.user.1".to_string(),
      tenant_id: "tenant.alpha".to_string(),
    }
  }

  #[test]
  fn review_decision_permits_apply_only_on_approve() {
    assert!(RemoveUnusedImportReviewDecision::Approve.permits_apply());
    assert!(!RemoveUnusedImportReviewDecision::Hold.permits_apply());
    assert!(!RemoveUnusedImportReviewDecision::Reject.permits_apply());
  }

  #[test]
  fn review_decision_kebab_case_strings() {
    assert_eq!(
      RemoveUnusedImportReviewDecision::Approve.as_str(),
      "approve"
    );
    assert_eq!(RemoveUnusedImportReviewDecision::Hold.as_str(), "hold");
    assert_eq!(RemoveUnusedImportReviewDecision::Reject.as_str(), "reject");
  }

  #[test]
  fn build_review_receipt_pins_candidate_artifact_id() {
    let req = ready_request_fixture();
    let candidate = compute_remove_unused_import_patch_candidate(&req, &ready_file_input());
    let receipt = build_remove_unused_import_review_receipt(
      candidate.clone(),
      fixture_reviewer(),
      RemoveUnusedImportReviewDecision::Approve,
      Some("looks safe".to_string()),
      1700000000000,
    );
    // The receipt embeds the candidate verbatim and pins its
    // canonical artifact id.
    assert_eq!(receipt.candidate, candidate);
    assert_eq!(receipt.decision, RemoveUnusedImportReviewDecision::Approve);
    assert_eq!(receipt.reason.as_deref(), Some("looks safe"));
    assert_eq!(receipt.reviewed_at_ms, 1700000000000);
    assert!(receipt
      .candidate_artifact_id
      .starts_with("remove-unused-import-patch."));
    // Same hash the wrapper computes — TOCTOU-binding identity.
    let direct_art = build_remove_unused_import_patch_candidate_artifact(&candidate, 0, None);
    assert_eq!(
      receipt.candidate_artifact_id,
      direct_art["id"].as_str().unwrap()
    );
  }

  #[test]
  fn review_receipt_payload_canonical_fields_for_approve() {
    let req = ready_request_fixture();
    let candidate = compute_remove_unused_import_patch_candidate(&req, &ready_file_input());
    let receipt = build_remove_unused_import_review_receipt(
      candidate,
      fixture_reviewer(),
      RemoveUnusedImportReviewDecision::Approve,
      Some("ok".to_string()),
      1700000000000,
    );
    let payload = build_remove_unused_import_review_receipt_payload(&receipt);
    assert_eq!(payload["transform"].as_str(), Some("remove-unused-import"));
    assert_eq!(
      payload["owner_law"].as_str(),
      Some("stdlib/lib/gate/code-transform/remove-unused-import.px")
    );
    assert_eq!(payload["decision"].as_str(), Some("approve"));
    assert_eq!(payload["permits_apply"].as_bool(), Some(true));
    assert_eq!(payload["next_step"].as_str(), Some("apply"));
    assert_eq!(payload["reason"].as_str(), Some("ok"));
    assert_eq!(
      payload["reviewer"]["actor_id"].as_str(),
      Some("actor.user.1")
    );
    assert_eq!(
      payload["reviewer"]["tenant_id"].as_str(),
      Some("tenant.alpha")
    );
    assert_eq!(payload["reviewed_at_ms"].as_u64(), Some(1700000000000));
    assert!(payload["candidate_artifact_id"]
      .as_str()
      .unwrap()
      .starts_with("remove-unused-import-patch."));
  }

  #[test]
  fn review_receipt_payload_next_step_per_decision() {
    let req = ready_request_fixture();
    let candidate = compute_remove_unused_import_patch_candidate(&req, &ready_file_input());
    // approve
    let approve = build_remove_unused_import_review_receipt(
      candidate.clone(),
      fixture_reviewer(),
      RemoveUnusedImportReviewDecision::Approve,
      None,
      0,
    );
    let approve_payload = build_remove_unused_import_review_receipt_payload(&approve);
    assert_eq!(approve_payload["next_step"].as_str(), Some("apply"));
    assert_eq!(approve_payload["permits_apply"].as_bool(), Some(true));
    // hold
    let hold = build_remove_unused_import_review_receipt(
      candidate.clone(),
      fixture_reviewer(),
      RemoveUnusedImportReviewDecision::Hold,
      None,
      0,
    );
    let hold_payload = build_remove_unused_import_review_receipt_payload(&hold);
    assert_eq!(
      hold_payload["next_step"].as_str(),
      Some("wait-for-evidence")
    );
    assert_eq!(hold_payload["permits_apply"].as_bool(), Some(false));
    // reject
    let reject = build_remove_unused_import_review_receipt(
      candidate,
      fixture_reviewer(),
      RemoveUnusedImportReviewDecision::Reject,
      None,
      0,
    );
    let reject_payload = build_remove_unused_import_review_receipt_payload(&reject);
    assert_eq!(reject_payload["next_step"].as_str(), Some("rejected"));
    assert_eq!(reject_payload["permits_apply"].as_bool(), Some(false));
  }

  #[test]
  fn review_receipt_payload_null_reason_when_absent() {
    let req = ready_request_fixture();
    let candidate = compute_remove_unused_import_patch_candidate(&req, &ready_file_input());
    let receipt = build_remove_unused_import_review_receipt(
      candidate,
      fixture_reviewer(),
      RemoveUnusedImportReviewDecision::Hold,
      None,
      0,
    );
    let payload = build_remove_unused_import_review_receipt_payload(&receipt);
    assert!(payload["reason"].is_null());
  }

  #[test]
  fn review_receipt_artifact_envelope_shape() {
    let req = ready_request_fixture();
    let candidate = compute_remove_unused_import_patch_candidate(&req, &ready_file_input());
    let receipt = build_remove_unused_import_review_receipt(
      candidate,
      fixture_reviewer(),
      RemoveUnusedImportReviewDecision::Approve,
      Some("approved".to_string()),
      1700000000000,
    );
    let art = build_remove_unused_import_review_receipt_artifact(&receipt, 1800000000000, None);
    assert_eq!(
      art["artifact_family"].as_str(),
      Some("coding.code-transform.remove-unused-import-review-receipt")
    );
    assert_eq!(
      art["source_surface"].as_str(),
      Some("code-transform.remove-unused-import")
    );
    assert_eq!(art["stored_at_ms"].as_u64(), Some(1800000000000));
    assert_eq!(art["target_paths"][0].as_str(), Some("src/a.rs"));
    let id = art["id"].as_str().expect("id");
    assert!(id.starts_with("review-receipt.remove-unused-import."));
    let related = art["related_refs"].as_array().expect("related_refs");
    assert!(related.iter().any(|v| v
      .as_str()
      .map(|s| s == "owner-law:stdlib/lib/gate/code-transform/remove-unused-import.px")
      .unwrap_or(false)));
    // back-ref to candidate artifact.
    assert!(related.iter().any(|v| v
      .as_str()
      .map(|s| s.starts_with("candidate-artifact:remove-unused-import-patch."))
      .unwrap_or(false)));
  }

  #[test]
  fn review_receipt_artifact_id_replay_stable() {
    // stored_at_ms is extrinsic — the same review event at two
    // different storage times produces the same artifact id.
    let req = ready_request_fixture();
    let candidate = compute_remove_unused_import_patch_candidate(&req, &ready_file_input());
    let receipt = build_remove_unused_import_review_receipt(
      candidate,
      fixture_reviewer(),
      RemoveUnusedImportReviewDecision::Approve,
      Some("ok".to_string()),
      1700000000000,
    );
    let a = build_remove_unused_import_review_receipt_artifact(&receipt, 1000, None);
    let b = build_remove_unused_import_review_receipt_artifact(&receipt, 9999, None);
    assert_eq!(a["id"], b["id"]);
    assert_ne!(a["stored_at_ms"], b["stored_at_ms"]);
  }

  #[test]
  fn review_receipt_artifact_id_differs_per_decision() {
    // Same candidate + reviewer + timestamp, different decisions →
    // different artifact ids. Audit can distinguish approve vs reject
    // even when other fields match.
    let req = ready_request_fixture();
    let candidate = compute_remove_unused_import_patch_candidate(&req, &ready_file_input());
    let approve = build_remove_unused_import_review_receipt(
      candidate.clone(),
      fixture_reviewer(),
      RemoveUnusedImportReviewDecision::Approve,
      None,
      1700000000000,
    );
    let hold = build_remove_unused_import_review_receipt(
      candidate.clone(),
      fixture_reviewer(),
      RemoveUnusedImportReviewDecision::Hold,
      None,
      1700000000000,
    );
    let reject = build_remove_unused_import_review_receipt(
      candidate,
      fixture_reviewer(),
      RemoveUnusedImportReviewDecision::Reject,
      None,
      1700000000000,
    );
    let a = build_remove_unused_import_review_receipt_artifact(&approve, 0, None);
    let h = build_remove_unused_import_review_receipt_artifact(&hold, 0, None);
    let r = build_remove_unused_import_review_receipt_artifact(&reject, 0, None);
    assert_ne!(a["id"], h["id"]);
    assert_ne!(h["id"], r["id"]);
    assert_ne!(a["id"], r["id"]);
  }

  #[test]
  fn review_receipt_artifact_carries_repo_snapshot_ref() {
    let req = ready_request_fixture();
    let candidate = compute_remove_unused_import_patch_candidate(&req, &ready_file_input());
    let receipt = build_remove_unused_import_review_receipt(
      candidate,
      fixture_reviewer(),
      RemoveUnusedImportReviewDecision::Approve,
      None,
      0,
    );
    let art =
      build_remove_unused_import_review_receipt_artifact(&receipt, 0, Some("commit-review-1"));
    assert_eq!(art["repo_snapshot_ref"].as_str(), Some("commit-review-1"));
  }

  // ─── apply receipt ───────────────────────────────────────────────

  fn fixture_apply_approval(candidate_artifact_id: &str) -> RemoveUnusedImportApplyApproval {
    RemoveUnusedImportApplyApproval {
      actor_id: "actor.user.1".to_string(),
      tenant_id: "tenant.alpha".to_string(),
      approved_at_ms: 1700000000000,
      candidate_artifact_id: candidate_artifact_id.to_string(),
    }
  }

  fn fixture_apply_receipt() -> RemoveUnusedImportApplyReceipt {
    let req = ready_request_fixture();
    let candidate = compute_remove_unused_import_patch_candidate(&req, &ready_file_input());
    let art = build_remove_unused_import_patch_candidate_artifact(&candidate, 0, None);
    let approval = fixture_apply_approval(art["id"].as_str().unwrap());
    let sealed = ValidatedRemoveUnusedImportPatchCandidate::new_checked(candidate)
      .expect("ready candidate seals");
    let files = vec![ready_file_input()];
    apply_remove_unused_import_patch_candidate(&sealed, &files, &approval, 1700000000999)
      .expect("apply succeeds")
  }

  #[test]
  fn validated_candidate_seals_only_ready_verdicts() {
    let req = ready_request_fixture();
    let ready = compute_remove_unused_import_patch_candidate(&req, &ready_file_input());
    assert!(ValidatedRemoveUnusedImportPatchCandidate::new_checked(ready.clone()).is_ok());

    let mut held_req = ready_request_fixture();
    held_req.candidate_imports.clear();
    let held = compute_remove_unused_import_patch_candidate(&held_req, &ready_file_input());
    assert!(
      ValidatedRemoveUnusedImportPatchCandidate::new_checked(held).is_err(),
      "Held candidates cannot be sealed"
    );
  }

  #[test]
  fn approval_from_review_only_lifts_approve() {
    let req = ready_request_fixture();
    let candidate = compute_remove_unused_import_patch_candidate(&req, &ready_file_input());
    let receipt = build_remove_unused_import_review_receipt(
      candidate,
      fixture_reviewer(),
      RemoveUnusedImportReviewDecision::Approve,
      None,
      1700000000000,
    );
    let approval = approval_from_remove_unused_import_review(&receipt).expect("approve lifts");
    assert_eq!(approval.actor_id, "actor.user.1");
    assert_eq!(approval.tenant_id, "tenant.alpha");
    assert_eq!(approval.approved_at_ms, 1700000000000);
    assert_eq!(
      approval.candidate_artifact_id,
      receipt.candidate_artifact_id
    );

    // Hold / Reject return None.
    for decision in [
      RemoveUnusedImportReviewDecision::Hold,
      RemoveUnusedImportReviewDecision::Reject,
    ] {
      let req = ready_request_fixture();
      let candidate = compute_remove_unused_import_patch_candidate(&req, &ready_file_input());
      let rec =
        build_remove_unused_import_review_receipt(candidate, fixture_reviewer(), decision, None, 0);
      assert!(approval_from_remove_unused_import_review(&rec).is_none());
    }
  }

  #[test]
  fn apply_strips_lines_and_records_post_state() {
    let receipt = fixture_apply_receipt();
    assert_eq!(receipt.per_file_after.len(), 1);
    let (path, after) = &receipt.per_file_after[0];
    assert_eq!(path, "src/a.rs");
    // Both `use` lines should be gone, leaving only `fn main() {}\n`.
    assert_eq!(after, "fn main() {}\n");
  }

  #[test]
  fn apply_inverse_diff_reverts_when_applied_to_post_state() {
    // The inverse_unified_diff conceptually inserts back what removal
    // deleted. Hunk anchors flip from `-L,1 +L,0` to `-L,0 +L,1`, and
    // `-foo` body lines flip to `+foo`.
    let receipt = fixture_apply_receipt();
    let inverse = &receipt.inverse_unified_diff;
    // Header swap: `--- a/...` ↔ `+++ b/...` swapped.
    assert!(inverse.contains("--- a/src/a.rs"));
    assert!(inverse.contains("+++ b/src/a.rs"));
    // Hunk anchor flipped to insertion form.
    assert!(inverse.contains("@@ -1,0 +1,1 @@"));
    assert!(inverse.contains("@@ -2,0 +2,1 @@"));
    // Body lines flipped to `+`.
    assert!(inverse.contains("+use std::path::PathBuf;"));
    assert!(inverse.contains("+use std::io::Read;"));
    // No `-<line>` body lines remain (removal inverted to insertion).
    let minus_body: Vec<&str> = inverse
      .lines()
      .filter(|l| l.starts_with('-') && !l.starts_with("--- "))
      .collect();
    assert_eq!(
      minus_body.len(),
      0,
      "inverse should have no removal body lines"
    );
  }

  #[test]
  fn apply_rejects_toctou_mismatch() {
    let req = ready_request_fixture();
    let candidate = compute_remove_unused_import_patch_candidate(&req, &ready_file_input());
    let sealed = ValidatedRemoveUnusedImportPatchCandidate::new_checked(candidate).expect("ready");
    let bad_approval = RemoveUnusedImportApplyApproval {
      actor_id: "actor.user.1".to_string(),
      tenant_id: "tenant.alpha".to_string(),
      approved_at_ms: 1700000000000,
      candidate_artifact_id: "remove-unused-import-patch.wronghash".to_string(),
    };
    let files = vec![ready_file_input()];
    let err =
      apply_remove_unused_import_patch_candidate(&sealed, &files, &bad_approval, 1700000000999)
        .expect_err("TOCTOU mismatch must reject");
    matches!(
      err,
      RemoveUnusedImportApplyError::ApprovalCandidateIdMismatch { .. }
    );
  }

  #[test]
  fn apply_rejects_empty_actor_and_tenant() {
    let req = ready_request_fixture();
    let candidate = compute_remove_unused_import_patch_candidate(&req, &ready_file_input());
    let art = build_remove_unused_import_patch_candidate_artifact(&candidate, 0, None);
    let sealed = ValidatedRemoveUnusedImportPatchCandidate::new_checked(candidate).expect("ready");
    let files = vec![ready_file_input()];

    let no_actor = RemoveUnusedImportApplyApproval {
      actor_id: String::new(),
      tenant_id: "tenant.alpha".to_string(),
      approved_at_ms: 0,
      candidate_artifact_id: art["id"].as_str().unwrap().to_string(),
    };
    assert!(matches!(
      apply_remove_unused_import_patch_candidate(&sealed, &files, &no_actor, 0)
        .expect_err("empty actor must reject"),
      RemoveUnusedImportApplyError::MissingApprovalActor
    ));

    let no_tenant = RemoveUnusedImportApplyApproval {
      actor_id: "actor.user.1".to_string(),
      tenant_id: String::new(),
      approved_at_ms: 0,
      candidate_artifact_id: art["id"].as_str().unwrap().to_string(),
    };
    assert!(matches!(
      apply_remove_unused_import_patch_candidate(&sealed, &files, &no_tenant, 0)
        .expect_err("empty tenant must reject"),
      RemoveUnusedImportApplyError::MissingApprovalTenant
    ));
  }

  #[test]
  fn apply_rejects_missing_staged_file() {
    let req = ready_request_fixture();
    let candidate = compute_remove_unused_import_patch_candidate(&req, &ready_file_input());
    let art = build_remove_unused_import_patch_candidate_artifact(&candidate, 0, None);
    let sealed = ValidatedRemoveUnusedImportPatchCandidate::new_checked(candidate).expect("ready");
    let approval = fixture_apply_approval(art["id"].as_str().unwrap());
    // Stage the WRONG file path.
    let wrong = RemoveUnusedImportFileInput {
      path: "src/b.rs",
      content: "fn other() {}\n",
    };
    let err = apply_remove_unused_import_patch_candidate(&sealed, &[wrong], &approval, 0)
      .expect_err("missing file must reject");
    matches!(
      err,
      RemoveUnusedImportApplyError::MissingFileForPatch { .. }
    );
  }

  #[test]
  fn apply_receipt_payload_canonical_fields_include_content() {
    use super::super::rename_symbol::ApplyReceiptContentPolicy;
    let receipt = fixture_apply_receipt();
    let payload = build_remove_unused_import_apply_receipt_payload(
      &receipt,
      ApplyReceiptContentPolicy::IncludeContent,
    );
    assert_eq!(payload["transform"].as_str(), Some("remove-unused-import"));
    assert_eq!(
      payload["owner_law"].as_str(),
      Some("stdlib/lib/gate/code-transform/remove-unused-import.px")
    );
    assert_eq!(payload["content_policy"].as_str(), Some("include-content"));
    assert_eq!(payload["next_step"].as_str(), Some("verify-or-rollback"));
    assert_eq!(payload["rollback_available"].as_bool(), Some(true));
    let files_after = payload["files_after"].as_array().unwrap();
    assert_eq!(files_after.len(), 1);
    assert_eq!(files_after[0]["path"].as_str(), Some("src/a.rs"));
    // IncludeContent → content body present + sha256 + byte_len.
    assert_eq!(files_after[0]["content"].as_str(), Some("fn main() {}\n"));
    assert!(files_after[0]["content_sha256"].as_str().is_some());
    assert_eq!(
      files_after[0]["byte_len"].as_u64(),
      Some("fn main() {}\n".len() as u64)
    );
    // approval triple present.
    assert_eq!(
      payload["approval"]["actor_id"].as_str(),
      Some("actor.user.1")
    );
    assert_eq!(
      payload["approval"]["tenant_id"].as_str(),
      Some("tenant.alpha")
    );
    assert_eq!(
      payload["approval"]["approved_at_ms"].as_u64(),
      Some(1700000000000)
    );
    assert_eq!(payload["applied_at_ms"].as_u64(), Some(1700000000999));
  }

  #[test]
  fn apply_receipt_payload_omits_content_under_omit_policy() {
    use super::super::rename_symbol::ApplyReceiptContentPolicy;
    let receipt = fixture_apply_receipt();
    let payload = build_remove_unused_import_apply_receipt_payload(
      &receipt,
      ApplyReceiptContentPolicy::OmitContent,
    );
    let files_after = payload["files_after"].as_array().unwrap();
    // OmitContent → no `content` field, but sha256 + byte_len present.
    assert!(files_after[0].get("content").is_none());
    assert!(files_after[0]["content_sha256"].as_str().is_some());
    assert!(files_after[0]["byte_len"].as_u64().is_some());
    assert_eq!(payload["content_policy"].as_str(), Some("omit-content"));
  }

  #[test]
  fn apply_receipt_artifact_envelope_shape() {
    use super::super::rename_symbol::ApplyReceiptContentPolicy;
    let receipt = fixture_apply_receipt();
    let art = build_remove_unused_import_apply_receipt_artifact(
      &receipt,
      1800000000000,
      None,
      ApplyReceiptContentPolicy::OmitContent,
    );
    assert_eq!(
      art["artifact_family"].as_str(),
      Some("coding.code-transform.remove-unused-import-apply-receipt")
    );
    assert_eq!(
      art["source_surface"].as_str(),
      Some("code-transform.remove-unused-import")
    );
    assert_eq!(art["stored_at_ms"].as_u64(), Some(1800000000000));
    assert_eq!(art["target_paths"][0].as_str(), Some("src/a.rs"));
    let id = art["id"].as_str().expect("id");
    assert!(id.starts_with("apply-receipt.remove-unused-import."));
    let related = art["related_refs"].as_array().expect("related_refs");
    assert!(related.iter().any(|v| v
      .as_str()
      .map(|s| s == "owner-law:stdlib/lib/gate/code-transform/remove-unused-import.px")
      .unwrap_or(false)));
    assert!(related.iter().any(|v| v
      .as_str()
      .map(|s| s.starts_with("candidate-artifact:remove-unused-import-patch."))
      .unwrap_or(false)));
  }

  #[test]
  fn apply_receipt_artifact_id_replay_stable_and_policy_extrinsic() {
    use super::super::rename_symbol::ApplyReceiptContentPolicy;
    let receipt = fixture_apply_receipt();
    // stored_at_ms extrinsic.
    let a = build_remove_unused_import_apply_receipt_artifact(
      &receipt,
      1000,
      None,
      ApplyReceiptContentPolicy::IncludeContent,
    );
    let b = build_remove_unused_import_apply_receipt_artifact(
      &receipt,
      9999,
      None,
      ApplyReceiptContentPolicy::IncludeContent,
    );
    assert_eq!(a["id"], b["id"]);
    assert_ne!(a["stored_at_ms"], b["stored_at_ms"]);
    // content_policy extrinsic — same event, different rendering, same id.
    let c = build_remove_unused_import_apply_receipt_artifact(
      &receipt,
      1000,
      None,
      ApplyReceiptContentPolicy::OmitContent,
    );
    assert_eq!(a["id"], c["id"]);
  }

  #[test]
  fn invert_hunk_anchor_handles_removal_form() {
    let line = "@@ -1,1 +1,0 @@\n";
    let inverted = invert_hunk_anchor(line).expect("anchor parses");
    assert_eq!(inverted, "@@ -1,0 +1,1 @@\n");

    // Without trailing newline.
    let no_nl = "@@ -7,1 +7,0 @@";
    let inverted_no_nl = invert_hunk_anchor(no_nl).expect("anchor parses");
    assert_eq!(inverted_no_nl, "@@ -7,0 +7,1 @@");

    // Malformed: missing leading `@@ ` → None (caller falls back).
    assert!(invert_hunk_anchor("garbage\n").is_none());
  }

  // ─── reverse edits (rollback core) ───────────────────────────────

  #[test]
  fn reverse_edits_round_trip_through_apply_yields_original() {
    let original = "use std::path::PathBuf;\nuse std::io::Read;\nfn main() {}\n";
    let candidates = vec![
      UnusedImportCandidate {
        module: "std::path::PathBuf".to_string(),
        used_in_macro: false,
        behind_cfg: false,
      },
      UnusedImportCandidate {
        module: "std::io::Read".to_string(),
        used_in_macro: false,
        behind_cfg: false,
      },
    ];
    let edits = compute_remove_unused_import_edits(original, &candidates, "rust");
    let post_apply = apply_remove_import_edits(original, &edits);
    assert_eq!(post_apply, "fn main() {}\n");
    let restored = reverse_remove_unused_import_edits(&post_apply, &edits);
    assert_eq!(
      restored, original,
      "reverse must produce the byte-exact original"
    );
  }

  #[test]
  fn reverse_edits_handles_imports_with_context() {
    // Mid-file imports with header + footer content. The reverse
    // must restore them at the exact byte_offsets.
    let original =
      "// header\nuse std::io::Read;\nfn main() {}\nuse std::path::PathBuf;\nstruct X;\n";
    let candidates = vec![
      UnusedImportCandidate {
        module: "std::io::Read".to_string(),
        used_in_macro: false,
        behind_cfg: false,
      },
      UnusedImportCandidate {
        module: "std::path::PathBuf".to_string(),
        used_in_macro: false,
        behind_cfg: false,
      },
    ];
    let edits = compute_remove_unused_import_edits(original, &candidates, "rust");
    let post_apply = apply_remove_import_edits(original, &edits);
    let restored = reverse_remove_unused_import_edits(&post_apply, &edits);
    assert_eq!(restored, original);
  }

  #[test]
  fn reverse_edits_empty_edits_returns_post_apply_unchanged() {
    let post = "fn main() {}\n";
    let restored = reverse_remove_unused_import_edits(post, &[]);
    assert_eq!(restored, post);
  }

  // ─── rollback handle ─────────────────────────────────────────────

  fn fixture_rollback_initiator() -> RemoveUnusedImportRollbackInitiator {
    RemoveUnusedImportRollbackInitiator {
      actor_id: "operator.ops.1".to_string(),
      tenant_id: "tenant.alpha".to_string(),
    }
  }

  fn fixture_rollback_handle() -> RemoveUnusedImportRollbackHandle {
    let apply_receipt = fixture_apply_receipt();
    build_remove_unused_import_rollback_handle(
      apply_receipt,
      fixture_rollback_initiator(),
      Some("regression detected after apply".to_string()),
      1800000010000,
    )
  }

  #[test]
  fn build_rollback_handle_pins_candidate_and_apply_ids() {
    let apply_receipt = fixture_apply_receipt();
    let handle = build_remove_unused_import_rollback_handle(
      apply_receipt.clone(),
      fixture_rollback_initiator(),
      Some("test".to_string()),
      1800000010000,
    );
    assert!(handle
      .candidate_artifact_id
      .starts_with("remove-unused-import-patch."));
    assert!(handle
      .apply_receipt_artifact_id
      .starts_with("apply-receipt.remove-unused-import."));
    assert_eq!(handle.initiator.actor_id, "operator.ops.1");
    assert_eq!(handle.reason.as_deref(), Some("test"));
    assert_eq!(handle.initiated_at_ms, 1800000010000);
    // Re-derive ids to confirm match.
    let direct_cand =
      build_remove_unused_import_patch_candidate_artifact(&apply_receipt.candidate, 0, None);
    assert_eq!(
      handle.candidate_artifact_id,
      direct_cand["id"].as_str().unwrap()
    );
  }

  #[test]
  fn rollback_handle_payload_canonical_fields() {
    let handle = fixture_rollback_handle();
    let payload = build_remove_unused_import_rollback_handle_payload(&handle);
    assert_eq!(payload["transform"].as_str(), Some("remove-unused-import"));
    assert_eq!(payload["rollback_state"].as_str(), Some("handle-issued"));
    assert_eq!(payload["next_step"].as_str(), Some("execute-rollback"));
    assert_eq!(payload["rollback_available"].as_bool(), Some(true));
    assert_eq!(
      payload["reason"].as_str(),
      Some("regression detected after apply")
    );
    assert!(payload["candidate_artifact_id"]
      .as_str()
      .unwrap()
      .starts_with("remove-unused-import-patch."));
    assert!(payload["apply_receipt_artifact_id"]
      .as_str()
      .unwrap()
      .starts_with("apply-receipt.remove-unused-import."));
    // inverse diff comes from apply receipt (line-insertion form).
    assert!(payload["inverse_unified_diff"]
      .as_str()
      .unwrap()
      .contains("@@ -1,0 +1,1 @@"));
  }

  #[test]
  fn rollback_handle_payload_null_reason_when_absent() {
    let apply_receipt = fixture_apply_receipt();
    let handle = build_remove_unused_import_rollback_handle(
      apply_receipt,
      fixture_rollback_initiator(),
      None,
      0,
    );
    let payload = build_remove_unused_import_rollback_handle_payload(&handle);
    assert!(payload["reason"].is_null());
  }

  #[test]
  fn rollback_handle_artifact_envelope_shape() {
    let handle = fixture_rollback_handle();
    let art = build_remove_unused_import_rollback_handle_artifact(&handle, 1900000000000, None);
    assert_eq!(
      art["artifact_family"].as_str(),
      Some("coding.rollback-handle")
    );
    assert_eq!(
      art["source_surface"].as_str(),
      Some("code-transform.remove-unused-import")
    );
    let id = art["id"].as_str().expect("id");
    assert!(id.starts_with("rollback-handle.remove-unused-import."));
    let related = art["related_refs"].as_array().unwrap();
    // Dual back-refs (candidate + apply-receipt).
    assert!(related.iter().any(|v| v
      .as_str()
      .map(|s| s.starts_with("candidate-artifact:remove-unused-import-patch."))
      .unwrap_or(false)));
    assert!(related.iter().any(|v| v
      .as_str()
      .map(|s| s.starts_with("apply-receipt-artifact:apply-receipt.remove-unused-import."))
      .unwrap_or(false)));
    assert_eq!(art["target_paths"][0].as_str(), Some("src/a.rs"));
  }

  #[test]
  fn rollback_handle_artifact_id_replay_stable() {
    let handle = fixture_rollback_handle();
    let a = build_remove_unused_import_rollback_handle_artifact(&handle, 1000, None);
    let b = build_remove_unused_import_rollback_handle_artifact(&handle, 9999, None);
    assert_eq!(a["id"], b["id"]);
  }

  #[test]
  fn rollback_handle_artifact_id_differs_per_initiator() {
    let apply_receipt = fixture_apply_receipt();
    let h1 = build_remove_unused_import_rollback_handle(
      apply_receipt.clone(),
      RemoveUnusedImportRollbackInitiator {
        actor_id: "operator.ops.1".to_string(),
        tenant_id: "tenant.alpha".to_string(),
      },
      Some("r".to_string()),
      1800000010000,
    );
    let h2 = build_remove_unused_import_rollback_handle(
      apply_receipt,
      RemoveUnusedImportRollbackInitiator {
        actor_id: "operator.ops.2".to_string(),
        tenant_id: "tenant.alpha".to_string(),
      },
      Some("r".to_string()),
      1800000010000,
    );
    let a = build_remove_unused_import_rollback_handle_artifact(&h1, 0, None);
    let b = build_remove_unused_import_rollback_handle_artifact(&h2, 0, None);
    assert_ne!(a["id"], b["id"]);
  }

  // ─── rollback receipt ────────────────────────────────────────────

  fn fixture_rollback_executor() -> RemoveUnusedImportRollbackExecutor {
    RemoveUnusedImportRollbackExecutor {
      actor_id: "executor.runner.1".to_string(),
      tenant_id: "tenant.alpha".to_string(),
    }
  }

  fn fixture_rollback_receipt() -> RemoveUnusedImportRollbackReceipt {
    let handle = fixture_rollback_handle();
    let post_apply: Vec<(String, String)> = handle.apply_receipt.per_file_after.clone();
    let inputs: Vec<RemoveUnusedImportFileInput<'_>> = post_apply
      .iter()
      .map(|(p, c)| RemoveUnusedImportFileInput {
        path: p.as_str(),
        content: c.as_str(),
      })
      .collect();
    execute_remove_unused_import_rollback(
      &handle,
      &inputs,
      &fixture_rollback_executor(),
      1800000020000,
    )
    .expect("rollback executes cleanly")
  }

  #[test]
  fn execute_rollback_restores_original_content() {
    let receipt = fixture_rollback_receipt();
    assert_eq!(receipt.per_file_after_rollback.len(), 1);
    let (path, restored) = &receipt.per_file_after_rollback[0];
    assert_eq!(path, "src/a.rs");
    // The original content had both `use` lines + main; rollback
    // restored them all.
    assert_eq!(
      restored,
      "use std::path::PathBuf;\nuse std::io::Read;\nfn main() {}\n"
    );
  }

  #[test]
  fn execute_rollback_rejects_empty_executor_actor() {
    let handle = fixture_rollback_handle();
    let post_apply: Vec<(String, String)> = handle.apply_receipt.per_file_after.clone();
    let inputs: Vec<RemoveUnusedImportFileInput<'_>> = post_apply
      .iter()
      .map(|(p, c)| RemoveUnusedImportFileInput {
        path: p.as_str(),
        content: c.as_str(),
      })
      .collect();
    let bad_executor = RemoveUnusedImportRollbackExecutor {
      actor_id: String::new(),
      tenant_id: "tenant.alpha".to_string(),
    };
    assert!(matches!(
      execute_remove_unused_import_rollback(&handle, &inputs, &bad_executor, 0)
        .expect_err("empty actor must reject"),
      RemoveUnusedImportRollbackError::MissingExecutorActor
    ));
  }

  #[test]
  fn execute_rollback_rejects_empty_executor_tenant() {
    let handle = fixture_rollback_handle();
    let post_apply: Vec<(String, String)> = handle.apply_receipt.per_file_after.clone();
    let inputs: Vec<RemoveUnusedImportFileInput<'_>> = post_apply
      .iter()
      .map(|(p, c)| RemoveUnusedImportFileInput {
        path: p.as_str(),
        content: c.as_str(),
      })
      .collect();
    let bad_executor = RemoveUnusedImportRollbackExecutor {
      actor_id: "executor.runner.1".to_string(),
      tenant_id: String::new(),
    };
    assert!(matches!(
      execute_remove_unused_import_rollback(&handle, &inputs, &bad_executor, 0)
        .expect_err("empty tenant must reject"),
      RemoveUnusedImportRollbackError::MissingExecutorTenant
    ));
  }

  #[test]
  fn execute_rollback_rejects_missing_file() {
    let handle = fixture_rollback_handle();
    // Stage the WRONG file path.
    let wrong = RemoveUnusedImportFileInput {
      path: "src/b.rs",
      content: "other content\n",
    };
    let err =
      execute_remove_unused_import_rollback(&handle, &[wrong], &fixture_rollback_executor(), 0)
        .expect_err("missing file must reject");
    assert!(matches!(
      err,
      RemoveUnusedImportRollbackError::MissingFileForRollback { .. }
    ));
  }

  #[test]
  fn execute_rollback_rejects_post_apply_drift() {
    // Someone hand-edited the file between apply and rollback. The
    // current content's sha256 doesn't match the apply receipt's
    // recorded post-apply sha256 → reject.
    let handle = fixture_rollback_handle();
    let drifted = RemoveUnusedImportFileInput {
      path: "src/a.rs",
      content: "DIFFERENT CONTENT — someone hand-edited\n",
    };
    let err =
      execute_remove_unused_import_rollback(&handle, &[drifted], &fixture_rollback_executor(), 0)
        .expect_err("drift must reject");
    match err {
      RemoveUnusedImportRollbackError::PostApplyDriftDetected {
        path,
        expected_sha256,
        found_sha256,
      } => {
        assert_eq!(path, "src/a.rs");
        assert_ne!(expected_sha256, found_sha256);
      }
      other => panic!("expected PostApplyDriftDetected, got {:?}", other),
    }
  }

  #[test]
  fn execute_rollback_pins_handle_artifact_id() {
    let receipt = fixture_rollback_receipt();
    assert!(receipt
      .rollback_handle_artifact_id
      .starts_with("rollback-handle.remove-unused-import."));
    // Re-derive to confirm match.
    let handle_art = build_remove_unused_import_rollback_handle_artifact(&receipt.handle, 0, None);
    assert_eq!(
      receipt.rollback_handle_artifact_id,
      handle_art["id"].as_str().unwrap()
    );
  }

  #[test]
  fn rollback_receipt_payload_omit_content_keeps_sha_and_byte_len() {
    let receipt = fixture_rollback_receipt();
    let payload = build_remove_unused_import_rollback_receipt_payload(
      &receipt,
      super::super::rename_symbol::ApplyReceiptContentPolicy::OmitContent,
    );
    assert_eq!(payload["transform"].as_str(), Some("remove-unused-import"));
    assert_eq!(payload["rollback_state"].as_str(), Some("executed"));
    assert_eq!(
      payload["next_step"].as_str(),
      Some("verify-rollback-or-redo-apply")
    );
    assert_eq!(payload["content_policy"].as_str(), Some("omit-content"));
    let files = payload["files_after_rollback"].as_array().unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0]["path"].as_str(), Some("src/a.rs"));
    // OmitContent → no content body, but sha256 + byte_len present.
    assert!(files[0].get("content").is_none());
    assert!(files[0]["content_sha256"].as_str().is_some());
    assert!(files[0]["byte_len"].as_u64().is_some());
  }

  #[test]
  fn rollback_receipt_payload_include_content_carries_body() {
    let receipt = fixture_rollback_receipt();
    let payload = build_remove_unused_import_rollback_receipt_payload(
      &receipt,
      super::super::rename_symbol::ApplyReceiptContentPolicy::IncludeContent,
    );
    let files = payload["files_after_rollback"].as_array().unwrap();
    // IncludeContent → content body present, restored to original.
    assert_eq!(
      files[0]["content"].as_str(),
      Some("use std::path::PathBuf;\nuse std::io::Read;\nfn main() {}\n")
    );
  }

  #[test]
  fn rollback_receipt_artifact_envelope_shape() {
    let receipt = fixture_rollback_receipt();
    let art = build_remove_unused_import_rollback_receipt_artifact(
      &receipt,
      2000000000000,
      None,
      super::super::rename_symbol::ApplyReceiptContentPolicy::OmitContent,
    );
    assert_eq!(
      art["artifact_family"].as_str(),
      Some("coding.rollback-receipt")
    );
    assert_eq!(
      art["source_surface"].as_str(),
      Some("code-transform.remove-unused-import")
    );
    let id = art["id"].as_str().expect("id");
    assert!(id.starts_with("rollback-receipt.remove-unused-import."));
    // TRIPLE back-refs: candidate, apply-receipt, rollback-handle.
    let related = art["related_refs"].as_array().unwrap();
    assert!(related.iter().any(|v| v
      .as_str()
      .map(|s| s.starts_with("candidate-artifact:"))
      .unwrap_or(false)));
    assert!(related.iter().any(|v| v
      .as_str()
      .map(|s| s.starts_with("apply-receipt-artifact:"))
      .unwrap_or(false)));
    assert!(related.iter().any(|v| v
      .as_str()
      .map(|s| s.starts_with("rollback-handle-artifact:"))
      .unwrap_or(false)));
  }

  #[test]
  fn rollback_receipt_artifact_id_replay_stable_and_policy_extrinsic() {
    let receipt = fixture_rollback_receipt();
    let omit = build_remove_unused_import_rollback_receipt_artifact(
      &receipt,
      1000,
      None,
      super::super::rename_symbol::ApplyReceiptContentPolicy::OmitContent,
    );
    let include = build_remove_unused_import_rollback_receipt_artifact(
      &receipt,
      9999,
      None,
      super::super::rename_symbol::ApplyReceiptContentPolicy::IncludeContent,
    );
    assert_eq!(
      omit["id"], include["id"],
      "stored_at_ms + content_policy extrinsic"
    );
    assert_ne!(omit["stored_at_ms"], include["stored_at_ms"]);
  }

  #[test]
  fn rollback_receipt_artifact_id_differs_per_executor() {
    let handle = fixture_rollback_handle();
    let post_apply: Vec<(String, String)> = handle.apply_receipt.per_file_after.clone();
    let inputs: Vec<RemoveUnusedImportFileInput<'_>> = post_apply
      .iter()
      .map(|(p, c)| RemoveUnusedImportFileInput {
        path: p.as_str(),
        content: c.as_str(),
      })
      .collect();
    let r1 = execute_remove_unused_import_rollback(
      &handle,
      &inputs,
      &RemoveUnusedImportRollbackExecutor {
        actor_id: "executor.runner.1".to_string(),
        tenant_id: "tenant.alpha".to_string(),
      },
      1800000020000,
    )
    .expect("rollback");
    let r2 = execute_remove_unused_import_rollback(
      &handle,
      &inputs,
      &RemoveUnusedImportRollbackExecutor {
        actor_id: "executor.runner.2".to_string(),
        tenant_id: "tenant.alpha".to_string(),
      },
      1800000020000,
    )
    .expect("rollback");
    let a = build_remove_unused_import_rollback_receipt_artifact(
      &r1,
      0,
      None,
      super::super::rename_symbol::ApplyReceiptContentPolicy::OmitContent,
    );
    let b = build_remove_unused_import_rollback_receipt_artifact(
      &r2,
      0,
      None,
      super::super::rename_symbol::ApplyReceiptContentPolicy::OmitContent,
    );
    assert_ne!(a["id"], b["id"]);
  }

  // ─── review + apply → materialization request bridge ────────────

  fn fixture_bridge_review(
    decision: RemoveUnusedImportReviewDecision,
    reviewer_tenant: &str,
  ) -> RemoveUnusedImportReviewReceipt {
    let req = ready_request_fixture();
    let candidate = compute_remove_unused_import_patch_candidate(&req, &ready_file_input());
    build_remove_unused_import_review_receipt(
      candidate,
      RemoveUnusedImportReviewer {
        actor_id: "reviewer.senior.1".to_string(),
        tenant_id: reviewer_tenant.to_string(),
      },
      decision,
      Some("reviewed for bridge".to_string()),
      1700000001000,
    )
  }

  fn fixture_bridge_apply_with_tenant(tenant: &str) -> RemoveUnusedImportApplyReceipt {
    let req = ready_request_fixture();
    let candidate = compute_remove_unused_import_patch_candidate(&req, &ready_file_input());
    let candidate_art = build_remove_unused_import_patch_candidate_artifact(&candidate, 0, None);
    let approval = RemoveUnusedImportApplyApproval {
      actor_id: "applier.junior.5".to_string(),
      tenant_id: tenant.to_string(),
      approved_at_ms: 1700000002000,
      candidate_artifact_id: candidate_art["id"].as_str().unwrap().to_string(),
    };
    let sealed = ValidatedRemoveUnusedImportPatchCandidate::new_checked(candidate).expect("ready");
    let files = vec![ready_file_input()];
    apply_remove_unused_import_patch_candidate(&sealed, &files, &approval, 1700000002500)
      .expect("apply")
  }

  #[test]
  fn rmi_bridge_happy_path_yields_ready_request() {
    let review = fixture_bridge_review(RemoveUnusedImportReviewDecision::Approve, "tenant.alpha");
    let apply = fixture_bridge_apply_with_tenant("tenant.alpha");
    let req = build_remove_unused_import_materialization_request(
      &review,
      &apply,
      "edit-within-target-paths",
      "git:abc123",
      "dev",
      "include-content",
      1700000003000,
    )
    .expect("ready");
    assert!(req
      .apply_receipt_artifact_id
      .starts_with("apply-receipt.remove-unused-import."));
    assert_eq!(req.requested_by_actor_id, "applier.junior.5");
    assert_eq!(req.requested_by_tenant_id, "tenant.alpha");
    assert_eq!(req.repo_snapshot_ref, "git:abc123");
    assert_eq!(req.requested_at_ms, 1700000003000);
    assert!(matches!(
      crate::tool_action::classify_tool_action_materialization_request(&req),
      crate::tool_action::ToolActionMaterializationVerdict::Ready
    ));
  }

  #[test]
  fn rmi_bridge_rejects_hold_review() {
    let review = fixture_bridge_review(RemoveUnusedImportReviewDecision::Hold, "tenant.alpha");
    let apply = fixture_bridge_apply_with_tenant("tenant.alpha");
    let err = build_remove_unused_import_materialization_request(
      &review,
      &apply,
      "edit-within-target-paths",
      "git:abc123",
      "dev",
      "include-content",
      0,
    )
    .expect_err("hold must reject");
    match err {
      crate::tool_action::MaterializationBridgeError::ReviewNotApproved { decision } => {
        assert_eq!(decision, "hold");
      }
      other => panic!("expected ReviewNotApproved, got {:?}", other),
    }
  }

  #[test]
  fn rmi_bridge_rejects_reject_review() {
    let review = fixture_bridge_review(RemoveUnusedImportReviewDecision::Reject, "tenant.alpha");
    let apply = fixture_bridge_apply_with_tenant("tenant.alpha");
    assert!(matches!(
      build_remove_unused_import_materialization_request(
        &review,
        &apply,
        "edit-within-target-paths",
        "git:abc123",
        "dev",
        "include-content",
        0,
      ),
      Err(crate::tool_action::MaterializationBridgeError::ReviewNotApproved { .. })
    ));
  }

  #[test]
  fn rmi_bridge_rejects_cross_tenant_review_and_apply() {
    let review = fixture_bridge_review(RemoveUnusedImportReviewDecision::Approve, "tenant.alpha");
    let apply = fixture_bridge_apply_with_tenant("tenant.beta");
    let err = build_remove_unused_import_materialization_request(
      &review,
      &apply,
      "edit-within-target-paths",
      "git:abc123",
      "dev",
      "include-content",
      0,
    )
    .expect_err("cross-tenant must reject");
    match err {
      crate::tool_action::MaterializationBridgeError::ReviewTenantMismatchesApplyTenant {
        review_tenant,
        apply_tenant,
      } => {
        assert_eq!(review_tenant, "tenant.alpha");
        assert_eq!(apply_tenant, "tenant.beta");
      }
      other => panic!(
        "expected ReviewTenantMismatchesApplyTenant, got {:?}",
        other
      ),
    }
  }

  #[test]
  fn rmi_bridge_rejects_candidate_mismatch() {
    // Review is for the standard fixture; apply is for a candidate
    // built from a DIFFERENT request (different candidate_imports).
    let review = fixture_bridge_review(RemoveUnusedImportReviewDecision::Approve, "tenant.alpha");
    let different_req = RemoveUnusedImportRequest {
      target_path: "src/a.rs".to_string(),
      language: "rust".to_string(),
      // Different candidate list → different candidate id.
      candidate_imports: vec![UnusedImportCandidate {
        module: "std::collections::HashSet".to_string(),
        used_in_macro: false,
        behind_cfg: false,
      }],
      scope: RemoveUnusedImportScope::SingleFile,
    };
    let file = RemoveUnusedImportFileInput {
      path: "src/a.rs",
      content: "use std::collections::HashSet;\nfn main() {}\n",
    };
    let different_candidate = compute_remove_unused_import_patch_candidate(&different_req, &file);
    let diff_art =
      build_remove_unused_import_patch_candidate_artifact(&different_candidate, 0, None);
    let diff_approval = RemoveUnusedImportApplyApproval {
      actor_id: "applier.junior.5".to_string(),
      tenant_id: "tenant.alpha".to_string(),
      approved_at_ms: 0,
      candidate_artifact_id: diff_art["id"].as_str().unwrap().to_string(),
    };
    let sealed =
      ValidatedRemoveUnusedImportPatchCandidate::new_checked(different_candidate).expect("ready");
    let different_apply =
      apply_remove_unused_import_patch_candidate(&sealed, &[file], &diff_approval, 0)
        .expect("apply");

    let err = build_remove_unused_import_materialization_request(
      &review,
      &different_apply,
      "edit-within-target-paths",
      "git:abc123",
      "dev",
      "include-content",
      0,
    )
    .expect_err("candidate mismatch must reject");
    assert!(matches!(
      err,
      crate::tool_action::MaterializationBridgeError::ReviewCandidateMismatchesApplyCandidate { .. }
    ));
  }

  #[test]
  fn rmi_bridge_forwards_classifier_rejected_for_customer_release_with_include() {
    let review = fixture_bridge_review(RemoveUnusedImportReviewDecision::Approve, "tenant.alpha");
    let apply = fixture_bridge_apply_with_tenant("tenant.alpha");
    let err = build_remove_unused_import_materialization_request(
      &review,
      &apply,
      "edit-within-target-paths",
      "git:abc123",
      "customer-release",
      "include-content",
      0,
    )
    .expect_err("customer-release + include-content must reject");
    assert!(matches!(
      err,
      crate::tool_action::MaterializationBridgeError::RequestNotReady(
        crate::tool_action::ToolActionMaterializationVerdict::Rejected {
          held_kind:
            crate::tool_action::ToolActionMaterializationHeldKind::CustomerReleaseForbidsIncludeContent,
          ..
        }
      )
    ));
  }

  #[test]
  fn rmi_bridge_allows_different_actor_same_tenant() {
    let review = fixture_bridge_review(RemoveUnusedImportReviewDecision::Approve, "tenant.alpha");
    let apply = fixture_bridge_apply_with_tenant("tenant.alpha");
    assert_ne!(review.reviewer.actor_id, apply.approval.actor_id);
    let req = build_remove_unused_import_materialization_request(
      &review,
      &apply,
      "edit-within-target-paths",
      "git:abc123",
      "dev",
      "include-content",
      0,
    )
    .expect("ready");
    assert_eq!(req.requested_by_actor_id, "applier.junior.5");
  }

  // ─── phase 2: multi-import / alias / Go block-import ────────────
  // (`cand(module)` helper is defined earlier in this test module)

  // ── Python multi-import ──

  #[test]
  fn python_multi_import_removes_one_of_two_partial_edit() {
    let src = "from os import path, getenv\nprint(getenv('X'))\n";
    let candidates = vec![cand("os.path")];
    let edits = compute_remove_unused_import_edits(src, &candidates, "python");
    assert_eq!(edits.len(), 1, "one partial edit for `path, ` deletion");
    let post = apply_remove_import_edits(src, &edits);
    assert_eq!(post, "from os import getenv\nprint(getenv('X'))\n");
  }

  #[test]
  fn python_multi_import_removes_last_of_two_partial_edit() {
    // Removing the LAST entry should consume the preceding comma.
    let src = "from os import path, getenv\nprint(path)\n";
    let candidates = vec![cand("os.getenv")];
    let edits = compute_remove_unused_import_edits(src, &candidates, "python");
    assert_eq!(edits.len(), 1);
    let post = apply_remove_import_edits(src, &edits);
    assert_eq!(post, "from os import path\nprint(path)\n");
  }

  #[test]
  fn python_multi_import_all_flagged_collapses_to_whole_line() {
    let src = "from os import path, getenv\nx = 1\n";
    let candidates = vec![cand("os.path"), cand("os.getenv")];
    let edits = compute_remove_unused_import_edits(src, &candidates, "python");
    assert_eq!(edits.len(), 1, "whole-line collapse — one edit");
    let post = apply_remove_import_edits(src, &edits);
    assert_eq!(post, "x = 1\n");
  }

  #[test]
  fn python_aliased_import_matches_alias() {
    let src = "import numpy as np\narr = np.zeros(3)\n";
    let candidates = vec![cand("np")];
    let edits = compute_remove_unused_import_edits(src, &candidates, "python");
    assert_eq!(edits.len(), 1);
    let post = apply_remove_import_edits(src, &edits);
    assert_eq!(post, "arr = np.zeros(3)\n");
  }

  #[test]
  fn python_aliased_from_import_matches_alias() {
    // `from os import path as p` — flagging `p` (the alias) matches.
    let src = "from os import path as p\nx = p\n";
    let candidates = vec![cand("p")];
    let edits = compute_remove_unused_import_edits(src, &candidates, "python");
    assert_eq!(edits.len(), 1);
    let post = apply_remove_import_edits(src, &edits);
    assert_eq!(post, "x = p\n");
  }

  #[test]
  fn python_phase1_single_import_still_works() {
    // Sanity: phase-1 single-line forms still produce a whole-line
    // delete via the phase-2 dispatcher.
    let src = "import os.path\nprint(os.path.join('a', 'b'))\n";
    let candidates = vec![cand("os.path")];
    let edits = compute_remove_unused_import_edits(src, &candidates, "python");
    assert_eq!(edits.len(), 1);
    let post = apply_remove_import_edits(src, &edits);
    assert_eq!(post, "print(os.path.join('a', 'b'))\n");
  }

  // ── TypeScript multi-named ──

  #[test]
  fn typescript_multi_named_removes_one_partial_edit() {
    let src = "import { Foo, Bar } from './m';\nconst x: Bar = null;\n";
    let candidates = vec![cand("Foo")];
    let edits = compute_remove_unused_import_edits(src, &candidates, "typescript");
    assert_eq!(edits.len(), 1, "one partial edit");
    let post = apply_remove_import_edits(src, &edits);
    assert_eq!(post, "import { Bar } from './m';\nconst x: Bar = null;\n");
  }

  #[test]
  fn typescript_multi_named_with_alias_matches_alias() {
    // `import { Foo as F } from 'm'` — flagging the alias `F`
    // removes that name.
    let src = "import { Foo as F, Bar } from './m';\nconst x: Bar = null;\n";
    let candidates = vec![cand("F")];
    let edits = compute_remove_unused_import_edits(src, &candidates, "typescript");
    assert_eq!(edits.len(), 1);
    let post = apply_remove_import_edits(src, &edits);
    assert_eq!(post, "import { Bar } from './m';\nconst x: Bar = null;\n");
  }

  #[test]
  fn typescript_default_plus_named_removes_named_only_collapses_empty_braces() {
    // Phase-2.1 (2026-05-12): when default+named statement has all its
    // named entries flagged but the default is kept, emit a single
    // edit covering `, { ... }` so the empty `{ }` artifact doesn't
    // remain on disk.
    let src = "import A, { B } from './m';\nconst x = A;\n";
    let candidates = vec![cand("B")];
    let edits = compute_remove_unused_import_edits(src, &candidates, "typescript");
    assert_eq!(edits.len(), 1, "single collapse edit covering , {{ B }}");
    let post = apply_remove_import_edits(src, &edits);
    assert_eq!(post, "import A from './m';\nconst x = A;\n");
  }

  #[test]
  fn typescript_default_plus_multi_named_all_removed_collapses_braces() {
    // Same collapse path with multiple named entries — all flagged,
    // default kept. Output: `import A from "m"` (no empty `{ , }`).
    let src = "import A, { B, C as D } from './m';\nconst x = A;\n";
    let candidates = vec![cand("B"), cand("C")];
    let edits = compute_remove_unused_import_edits(src, &candidates, "typescript");
    assert_eq!(edits.len(), 1);
    let post = apply_remove_import_edits(src, &edits);
    assert_eq!(post, "import A from './m';\nconst x = A;\n");
  }

  #[test]
  fn typescript_named_only_partial_keeps_remaining_named() {
    // Existing partial path still works when default+named has SOME
    // named flagged. Phase-2.1 collapse must NOT fire here — only
    // when ALL named are flagged.
    let src = "import A, { B, C } from './m';\nuse(A, C);\n";
    let candidates = vec![cand("B")];
    let edits = compute_remove_unused_import_edits(src, &candidates, "typescript");
    // partial path emits per-name edit on B
    let post = apply_remove_import_edits(src, &edits);
    assert!(post.contains("import A"));
    assert!(post.contains("C"));
    assert!(!post.contains("B"));
  }

  #[test]
  fn typescript_pure_named_all_flagged_collapses_to_whole_line() {
    // Pure named (no default) — all flagged → whole-line delete
    // (existing behavior; phase-2.1 collapse range is `None` for
    // pure-named statements, so the whole-line path wins).
    let src = "import { B, C } from './m';\nconst x = 1;\n";
    let candidates = vec![cand("B"), cand("C")];
    let edits = compute_remove_unused_import_edits(src, &candidates, "typescript");
    assert_eq!(edits.len(), 1);
    let post = apply_remove_import_edits(src, &edits);
    assert_eq!(post, "const x = 1;\n");
  }

  // ─── P0-2: default-only removal must preserve named clause ──────
  //
  // Reported by code-review 2026-05-12: `import A, { B } from "m"`
  // with A flagged was over-deleting (whole-line) because the
  // default's delete_range was line_range. Loses B → real code damage.

  #[test]
  fn typescript_default_plus_named_removing_default_preserves_named() {
    let src = "import A, { B } from './m';\nuse(B);\n";
    let candidates = vec![cand("A")];
    let edits = compute_remove_unused_import_edits(src, &candidates, "typescript");
    let post = apply_remove_import_edits(src, &edits);
    assert_eq!(post, "import { B } from './m';\nuse(B);\n");
  }

  #[test]
  fn typescript_default_plus_multi_named_removing_default_preserves_all_named() {
    // Default + multiple named; default flagged. All named entries must
    // survive verbatim (including their internal `as` alias).
    let src = "import A, { B, C as D } from './m';\nuse(B); use(D);\n";
    let candidates = vec![cand("A")];
    let edits = compute_remove_unused_import_edits(src, &candidates, "typescript");
    let post = apply_remove_import_edits(src, &edits);
    assert_eq!(post, "import { B, C as D } from './m';\nuse(B); use(D);\n");
  }

  #[test]
  fn typescript_default_plus_named_default_and_one_named_flagged() {
    // Default flagged, AND one of the named entries flagged. Both
    // should be removed individually (not whole-line). C survives.
    let src = "import A, { B, C } from './m';\nuse(C);\n";
    let candidates = vec![cand("A"), cand("B")];
    let edits = compute_remove_unused_import_edits(src, &candidates, "typescript");
    let post = apply_remove_import_edits(src, &edits);
    assert_eq!(post, "import { C } from './m';\nuse(C);\n");
  }

  #[test]
  fn typescript_type_default_plus_named_removing_default_preserves_named() {
    // `import type A, { B } from 'm'` — the `type ` prefix is stripped
    // before parsing, so the same default-partial-range logic applies.
    let src = "import type A, { B } from './m';\nuse(B);\n";
    let candidates = vec![cand("A")];
    let edits = compute_remove_unused_import_edits(src, &candidates, "typescript");
    let post = apply_remove_import_edits(src, &edits);
    // Note: `type ` prefix stays — only the `A,` portion is removed.
    assert_eq!(post, "import type { B } from './m';\nuse(B);\n");
  }

  #[test]
  fn typescript_default_only_with_default_flagged_still_whole_line() {
    // Sanity: pure default with no named clause still deletes the
    // whole line. (No `{ ... }` to preserve.)
    let src = "import A from './m';\nconst x = 1;\n";
    let candidates = vec![cand("A")];
    let edits = compute_remove_unused_import_edits(src, &candidates, "typescript");
    let post = apply_remove_import_edits(src, &edits);
    assert_eq!(post, "const x = 1;\n");
  }

  #[test]
  fn typescript_namespace_only_with_flagged_still_whole_line() {
    // Sanity: pure namespace `import * as A from 'm'` with A flagged
    // → whole-line delete (no other clause to preserve).
    let src = "import * as A from './m';\nconst x = 1;\n";
    let candidates = vec![cand("A")];
    let edits = compute_remove_unused_import_edits(src, &candidates, "typescript");
    let post = apply_remove_import_edits(src, &edits);
    assert_eq!(post, "const x = 1;\n");
  }

  // ── Go block-import ──

  #[test]
  fn go_block_import_removes_one_spec_line() {
    let src = "package main\n\nimport (\n\t\"fmt\"\n\t\"os\"\n)\n\nfunc main() {}\n";
    let candidates = vec![cand("fmt")];
    let edits = compute_remove_unused_import_edits(src, &candidates, "go");
    assert_eq!(edits.len(), 1, "one spec line removed");
    let post = apply_remove_import_edits(src, &edits);
    assert_eq!(
      post,
      "package main\n\nimport (\n\t\"os\"\n)\n\nfunc main() {}\n"
    );
  }

  #[test]
  fn go_block_import_all_removed_collapses_to_whole_block() {
    let src = "package main\n\nimport (\n\t\"fmt\"\n\t\"os\"\n)\n\nfunc main() {}\n";
    let candidates = vec![cand("fmt"), cand("os")];
    let edits = compute_remove_unused_import_edits(src, &candidates, "go");
    assert_eq!(edits.len(), 1, "whole-block collapse — one edit");
    let post = apply_remove_import_edits(src, &edits);
    // The whole `import ( ... )\n` block is gone.
    assert_eq!(post, "package main\n\n\nfunc main() {}\n");
  }

  #[test]
  fn go_block_whole_delete_unified_diff_renders_every_line() {
    // Phase-2.1 fix (2026-05-12): the unified-diff renderer for a
    // multi-line Go `import ( ... )` block delete must emit one `-`
    // line per source line, with the correct hunk header count
    // (`@@ -3,4 +3,0 @@`), not just the opening line. Without this
    // fix the artifact's `unified_diff` was a "summary diff" that
    // `git apply` could not replay — failing the
    // `unified_diff = replayable` audit contract.
    let src = "package main\n\nimport (\n\t\"fmt\"\n\t\"os\"\n)\n\nfunc main() {}\n";
    let candidates = vec![cand("fmt"), cand("os")];
    let edits = compute_remove_unused_import_edits(src, &candidates, "go");
    assert_eq!(edits.len(), 1, "whole-block collapse — one edit");
    let diff = render_unified_diff_for_remove_unused_import("main.go", src, &edits);
    // Hunk header: `import (` starts at source line 3, block spans 4
    // lines (`import (` + `\t"fmt"` + `\t"os"` + `)`).
    assert!(
      diff.contains("@@ -3,4 +3,0 @@"),
      "expected @@ -3,4 +3,0 @@ hunk, got:\n{diff}"
    );
    // Every block line must appear with `-` prefix.
    for expected in &["-import (", "-\t\"fmt\"", "-\t\"os\"", "-)"] {
      assert!(
        diff.contains(expected),
        "diff missing line `{expected}`:\n{diff}"
      );
    }
    // And the diff must round-trip via the existing apply path —
    // apply removes the same 4 lines plus their trailing newlines.
    let post = apply_remove_import_edits(src, &edits);
    assert!(!post.contains("import ("));
    assert!(!post.contains("\"fmt\""));
    assert!(!post.contains("\"os\""));
  }

  #[test]
  fn go_aliased_import_matches_alias() {
    let src = "package main\n\nimport myfmt \"fmt\"\n\nfunc main() {}\n";
    let candidates = vec![cand("myfmt")];
    let edits = compute_remove_unused_import_edits(src, &candidates, "go");
    assert_eq!(edits.len(), 1);
    let post = apply_remove_import_edits(src, &edits);
    assert_eq!(post, "package main\n\n\nfunc main() {}\n");
  }

  #[test]
  fn go_block_import_with_aliases_matches_alias_inside_block() {
    let src = "import (\n\tmyfmt \"fmt\"\n\t_ \"side/effect\"\n)\n";
    let candidates = vec![cand("myfmt")];
    let edits = compute_remove_unused_import_edits(src, &candidates, "go");
    assert_eq!(edits.len(), 1);
    let post = apply_remove_import_edits(src, &edits);
    assert_eq!(post, "import (\n\t_ \"side/effect\"\n)\n");
  }

  // ── partial-edit diff rendering (1→1 hunk) ──

  #[test]
  fn diff_renderer_emits_partial_hunk_for_python_multi_import() {
    let src = "from os import path, getenv\n";
    let candidates = vec![cand("os.path")];
    let edits = compute_remove_unused_import_edits(src, &candidates, "python");
    let diff = render_unified_diff_for_remove_unused_import("src/a.py", src, &edits);
    // Partial → 1→1 hunk, not 1→0.
    assert!(
      diff.contains("@@ -1,1 +1,1 @@"),
      "expected 1→1 hunk for partial deletion, got:\n{diff}"
    );
    assert!(diff.contains("-from os import path, getenv"));
    assert!(diff.contains("+from os import getenv"));
  }

  #[test]
  fn diff_renderer_emits_whole_line_hunk_when_all_flagged_collapsed() {
    let src = "from os import path, getenv\n";
    let candidates = vec![cand("os.path"), cand("os.getenv")];
    let edits = compute_remove_unused_import_edits(src, &candidates, "python");
    let diff = render_unified_diff_for_remove_unused_import("src/a.py", src, &edits);
    // All flagged → whole-line 1→0.
    assert!(diff.contains("@@ -1,1 +1,0 @@"));
    assert!(diff.contains("-from os import path, getenv"));
    // No `+` body line for whole-line delete.
    let plus_body: Vec<&str> = diff
      .lines()
      .filter(|l| l.starts_with('+') && !l.starts_with("+++"))
      .collect();
    assert_eq!(plus_body.len(), 0);
  }

  #[test]
  fn diff_renderer_emits_partial_hunk_for_typescript_named_import() {
    let src = "import { Foo, Bar } from './m';\n";
    let candidates = vec![cand("Bar")];
    let edits = compute_remove_unused_import_edits(src, &candidates, "typescript");
    let diff = render_unified_diff_for_remove_unused_import("src/a.ts", src, &edits);
    assert!(diff.contains("@@ -1,1 +1,1 @@"));
    assert!(diff.contains("-import { Foo, Bar } from './m';"));
    // Result has Foo only; the comma + Bar are gone.
    assert!(diff.contains("+import { Foo } from './m';"));
  }

  #[test]
  fn diff_renderer_handles_go_block_whole_delete() {
    // Phase-2.1 (2026-05-12): the renderer now emits a 1→0 hunk with
    // ONE `-` line per source line in the deleted block, and the
    // hunk header records the real line count (`@@ -1,4 +1,0 @@`),
    // not a summary `@@ -1,1 +1,0 @@`. This makes `unified_diff`
    // replayable by `git apply` and faithful to the audit contract.
    let src = "import (\n\t\"fmt\"\n\t\"os\"\n)\n";
    let candidates = vec![cand("fmt"), cand("os")];
    let edits = compute_remove_unused_import_edits(src, &candidates, "go");
    let diff = render_unified_diff_for_remove_unused_import("src/a.go", src, &edits);
    assert!(
      diff.contains("@@ -1,4 +1,0 @@"),
      "expected real 4-line hunk header, got:\n{diff}"
    );
    for expected in &["-import (", "-\t\"fmt\"", "-\t\"os\"", "-)"] {
      assert!(
        diff.contains(expected),
        "expected `{expected}` in diff, got:\n{diff}"
      );
    }
  }
}
