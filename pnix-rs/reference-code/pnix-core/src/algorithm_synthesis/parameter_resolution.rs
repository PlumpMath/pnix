//! Parameter-resolution carrier (v0).
//!
//! OWNER-LAW (2026-05-12): mirror of
//! `stdlib/lib/gate/algorithm-synthesis/parameter-resolution.px`.
//! Resolves a code-transform request's fields from raw NL +
//! attached code + caller-supplied context.
//!
//! Scope of v0: handles **rename-symbol**, **remove-unused-import**,
//! **add-test-stub**, and **add-import**. Other transforms emit
//! `TransformNotSupportedByResolver` Held.
//!
//! The Held path is the load-bearing output. Abstract NL routinely
//! lacks some fields ("이 함수 이름 바꿔줘" with no old/new symbols);
//! the receipt enumerates `missing_slots` so the caller knows what
//! follow-up question to ask the operator. Ready is only emitted
//! when every required field for the chosen transform is resolved
//! AND passes its sanity checks.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Transforms v0 knows how to resolve.
pub const VALID_RESOLVED_TRANSFORMS: &[&str] = &[
  "rename-symbol",
  "remove-unused-import",
  "add-test-stub",
  "add-import",
  // Math-domain transform. Substrate-sharing proof: a non-coding
  // operation that uses the same resolver dispatcher mechanism.
  "lookup-algebraic-equivalent",
  // Chemistry-domain transform. Substrate-sharing N=3.
  "lookup-chemical-reaction",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResolutionHeldKind {
  TransformNotSupportedByResolver,
  // rename-symbol gaps
  MissingOldName,
  MissingNewName,
  MissingTargetPath,
  LanguageNotDerivable,
  InvalidIdentifier,
  OldEqualsNew,
  // remove-unused-import gaps
  MissingCandidateImports,
  // add-test-stub gaps
  MissingTestName,
  // add-import gaps
  MissingImportSpec,
  // math-domain gaps — substrate-sharing proof: same Held shape,
  // different domain.
  /// User asked for an algebraic equivalent of an expression. The
  /// `canonical_form` is known (from the utterance) but the
  /// `equivalent_form` is missing; downstream retrieval must
  /// supply it (via ankh hit, external CAS adapter when wired, or
  /// operator-followup).
  MissingAlgebraicEquivalent,
  /// User asked about a chemical reaction. The `reactants` +
  /// `conditions` are known (from the utterance) but the
  /// `products` are missing. Substrate-sharing N=3 — same Held
  /// shape, third domain.
  MissingChemistryProducts,
}

impl ResolutionHeldKind {
  pub const ALL: &'static [Self] = &[
    Self::TransformNotSupportedByResolver,
    Self::MissingOldName,
    Self::MissingNewName,
    Self::MissingTargetPath,
    Self::LanguageNotDerivable,
    Self::InvalidIdentifier,
    Self::OldEqualsNew,
    Self::MissingCandidateImports,
    Self::MissingTestName,
    Self::MissingImportSpec,
    Self::MissingAlgebraicEquivalent,
    Self::MissingChemistryProducts,
  ];

  pub fn as_str(self) -> &'static str {
    match self {
      Self::TransformNotSupportedByResolver => "transform-not-supported-by-resolver",
      Self::MissingOldName => "missing-old-name",
      Self::MissingNewName => "missing-new-name",
      Self::MissingTargetPath => "missing-target-path",
      Self::LanguageNotDerivable => "language-not-derivable",
      Self::InvalidIdentifier => "invalid-identifier",
      Self::OldEqualsNew => "old-equals-new",
      Self::MissingCandidateImports => "missing-candidate-imports",
      Self::MissingTestName => "missing-test-name",
      Self::MissingImportSpec => "missing-import-spec",
      Self::MissingAlgebraicEquivalent => "missing-algebraic-equivalent",
      Self::MissingChemistryProducts => "missing-chemistry-products",
    }
  }
}

/// File-extension → language. Mirror of `.px`
/// `fileExtensionToLanguage`. Same data shape on both sides — sync
/// test compares.
pub const FILE_EXTENSION_TO_LANGUAGE: &[(&str, &str)] = &[
  (".rs", "rust"),
  (".py", "python"),
  (".ts", "typescript"),
  (".tsx", "typescript"),
  (".js", "javascript"),
  (".mjs", "javascript"),
  (".cjs", "javascript"),
  (".jsx", "javascript"),
  (".go", "go"),
];

/// Korean rename patterns: `<X>를 <Y>로 바꾸` etc. Each entry holds
/// the two marker strings.
#[derive(Debug, Clone, Copy)]
pub struct KoreanRenamePattern {
  pub from_marker: &'static str,
  pub to_marker: &'static str,
}

pub const KOREAN_RENAME_PATTERNS: &[KoreanRenamePattern] = &[
  KoreanRenamePattern {
    from_marker: "를 ",
    to_marker: "로 바꾸",
  },
  KoreanRenamePattern {
    from_marker: "를 ",
    to_marker: "로 바꿔",
  },
  KoreanRenamePattern {
    from_marker: "을 ",
    to_marker: "로 바꾸",
  },
  KoreanRenamePattern {
    from_marker: "을 ",
    to_marker: "로 바꿔",
  },
  KoreanRenamePattern {
    from_marker: " ",
    to_marker: "로 변경",
  },
];

/// English rename patterns: `rename X to Y` etc.
#[derive(Debug, Clone, Copy)]
pub struct EnglishRenamePattern {
  pub lead: &'static str,
  pub mid: &'static str,
}

pub const ENGLISH_RENAME_PATTERNS: &[EnglishRenamePattern] = &[
  EnglishRenamePattern {
    lead: "rename ",
    mid: " to ",
  },
  EnglishRenamePattern {
    lead: "rename ",
    mid: " as ",
  },
];

/// Test-name extraction markers. The resolver looks for these markers
/// in the utterance and takes the next whitespace-delimited identifier
/// token as the test name. Mirror of `.px` `testNameMarkers`.
pub const TEST_NAME_MARKERS: &[&str] = &["test ", "테스트 "];

/// Per-language import-spec extraction pattern. Mirror of `.px`
/// `importSpecPatterns`. The dispatcher iterates this table — new
/// languages = new row, no new Rust branch.
#[derive(Debug, Clone, Copy)]
pub struct ImportSpecPattern {
  pub language: &'static str,
  /// Substring marker that introduces the spec (e.g. `"from "`,
  /// `"import "`, `"use "`).
  pub lead: &'static str,
  /// For the `FromImport` shape: the keyword between module and
  /// name (e.g. `"import"` for Python). Empty for `ImportName`.
  pub middle: &'static str,
  /// Identifier shape — controls which characters are legal in a
  /// matched name token.
  pub name_kind: ImportNameKind,
  pub shape: ImportSpecShape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportSpecShape {
  /// `<lead><module> <middle> <name>` — Python `from m import x`.
  FromImport,
  /// `<lead><name>` — Python `import x`, Rust `use x::y`.
  ImportName,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportNameKind {
  /// ASCII-alnum + `_` + `.` (Python dotted modules).
  PythonIdent,
  /// ASCII-alnum + `_` + `:` (Rust path segments).
  RustPath,
}

pub const IMPORT_SPEC_PATTERNS: &[ImportSpecPattern] = &[
  ImportSpecPattern {
    language: "python",
    lead: "from ",
    middle: "import",
    name_kind: ImportNameKind::PythonIdent,
    shape: ImportSpecShape::FromImport,
  },
  ImportSpecPattern {
    language: "python",
    lead: "import ",
    middle: "",
    name_kind: ImportNameKind::PythonIdent,
    shape: ImportSpecShape::ImportName,
  },
  ImportSpecPattern {
    language: "rust",
    lead: "use ",
    middle: "",
    name_kind: ImportNameKind::RustPath,
    shape: ImportSpecShape::ImportName,
  },
];

pub const DEFAULT_RENAME_SCOPE: &str = "local-target-paths";
pub const DEFAULT_REMOVE_UNUSED_IMPORT_SCOPE: &str = "single-file";

/// Polymorphic input. v0 only consults `operation_candidate` (which
/// transform), `utterance` (for pattern-based extraction),
/// `target_path` (explicit caller-supplied), and
/// `candidate_imports_for_unused` (for remove-unused-import). Future
/// fields (attached_code, repo_state, prior_turns) are accepted but
/// not yet consulted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ResolutionInput {
  /// Transform name from operation-candidate-mapping. Must be in
  /// `VALID_RESOLVED_TRANSFORMS` or this owner emits
  /// `TransformNotSupportedByResolver`.
  pub operation_candidate: String,
  #[serde(default)]
  pub utterance: String,
  /// Explicit caller-supplied target path (e.g. from a UI file
  /// picker). When present, takes precedence over utterance-extracted
  /// paths.
  #[serde(default)]
  pub target_path: String,
  #[serde(default)]
  pub attached_code: String,
  /// For remove-unused-import: pre-resolved candidate imports from
  /// the host's symbol resolver. Empty → `MissingCandidateImports`.
  #[serde(default)]
  pub candidate_imports_for_unused: Vec<String>,
  /// For add-import: language-specific import spec the host's lint /
  /// compile / LSP discovered should be added. Empty + no NL match
  /// → `MissingImportSpec`. Examples:
  ///   - Rust: `"std::collections::HashMap"`
  ///   - Python: `"import os"` or `"from collections import deque"`
  ///   - TypeScript: `"import { foo } from \"bar\""`
  ///   - Go: `"\"net/http\""`
  #[serde(default)]
  pub candidate_import_spec: String,
  /// Operator scope hint ("local-target-paths" / "tests-also" /
  /// "crate-wide" / "single-file"). Empty → use the transform's
  /// default.
  #[serde(default)]
  pub scope_hint: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "verdict")]
pub enum ResolutionVerdict {
  ResolutionReady {
    transform: String,
    /// The resolved typed transform request, serialized as a JSON
    /// object. Caller passes this to `classify_<transform>`.
    request: serde_json::Value,
    resolved_fields: BTreeMap<String, String>,
  },
  ResolutionHeld {
    transform: String,
    held_kind: ResolutionHeldKind,
    /// Slot names whose values are still needed. e.g.
    /// `["old_name", "new_name", "target_path"]`. Empty for held
    /// kinds that aren't slot-missing (e.g. `InvalidIdentifier`).
    missing_slots: Vec<String>,
    /// Whatever the resolver DID manage to fill — handed to the
    /// follow-up question so the operator doesn't repeat themselves.
    partial_resolution: BTreeMap<String, String>,
    reason: String,
  },
  ResolutionRejected {
    transform: String,
    held_kind: ResolutionHeldKind,
    reason: String,
  },
}

fn is_valid_identifier(name: &str) -> bool {
  let mut chars = name.chars();
  match chars.next() {
    None => return false,
    Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
    _ => return false,
  }
  chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Find a file path inside an utterance by scanning known extensions.
/// Returns the first match; conservative — caller-supplied
/// `target_path` should always win when non-empty.
fn extract_target_path_from_utterance(utterance: &str) -> Option<String> {
  for (ext, _) in FILE_EXTENSION_TO_LANGUAGE {
    // Look for occurrences of the extension; the path is the
    // contiguous non-whitespace word ending at that extension.
    let mut start = 0;
    while let Some(pos) = utterance[start..].find(ext) {
      let abs = start + pos;
      // Walk backward to find the path start (first non-path char).
      let bytes = utterance.as_bytes();
      let mut path_start = abs;
      while path_start > 0 {
        let b = bytes[path_start - 1];
        // Path-internal chars: alnum, '/', '_', '-', '.', backslash.
        let allowed = b.is_ascii_alphanumeric()
          || b == b'/'
          || b == b'\\'
          || b == b'_'
          || b == b'-'
          || b == b'.';
        if !allowed {
          break;
        }
        path_start -= 1;
      }
      let path_end = abs + ext.len();
      let path = &utterance[path_start..path_end];
      // Skip bare extensions or single chars.
      if path.len() > ext.len() {
        return Some(path.to_string());
      }
      start = path_end;
    }
  }
  None
}

fn language_from_target_path(target_path: &str) -> Option<&'static str> {
  for (ext, lang) in FILE_EXTENSION_TO_LANGUAGE {
    if target_path.ends_with(ext) {
      return Some(lang);
    }
  }
  None
}

/// Extract `(old, new)` from utterance using the declarative pattern
/// registry. Returns the first match.
fn extract_rename_pair(utterance: &str) -> Option<(String, String)> {
  // Try English patterns first (more rigid form).
  let lowered = utterance.to_lowercase();
  for pat in ENGLISH_RENAME_PATTERNS {
    if let Some(lead_pos) = lowered.find(pat.lead) {
      let after_lead = &utterance[lead_pos + pat.lead.len()..];
      if let Some(mid_pos) = after_lead.to_lowercase().find(pat.mid) {
        let old = after_lead[..mid_pos].trim().to_string();
        let after_mid = &after_lead[mid_pos + pat.mid.len()..];
        // New name is the next whitespace-delimited token, stripped
        // of trailing punctuation.
        let new = after_mid
          .split(|c: char| c.is_whitespace() || matches!(c, '.' | ',' | ';' | '!' | '?'))
          .find(|s| !s.is_empty())
          .unwrap_or("")
          .to_string();
        if !old.is_empty() && !new.is_empty() {
          return Some((old, new));
        }
      }
    }
  }
  // Korean patterns: split on from_marker → left side's last token
  // is old; split rest on to_marker → left side's last token is new.
  for pat in KOREAN_RENAME_PATTERNS {
    if let Some(from_pos) = utterance.find(pat.from_marker) {
      let before_from = &utterance[..from_pos];
      let after_from = &utterance[from_pos + pat.from_marker.len()..];
      if let Some(to_pos) = after_from.find(pat.to_marker) {
        let between = &after_from[..to_pos];
        // old = last word of `before_from`; new = last word of `between`.
        let old = before_from
          .split_whitespace()
          .last()
          .unwrap_or("")
          .trim_matches(|c: char| matches!(c, '"' | '\'' | '`'))
          .to_string();
        let new = between
          .split_whitespace()
          .last()
          .unwrap_or("")
          .trim_matches(|c: char| matches!(c, '"' | '\'' | '`'))
          .to_string();
        if !old.is_empty() && !new.is_empty() {
          return Some((old, new));
        }
      }
    }
  }
  None
}

fn resolved_target_path(input: &ResolutionInput) -> Option<String> {
  if !input.target_path.is_empty() {
    return Some(input.target_path.clone());
  }
  extract_target_path_from_utterance(&input.utterance)
}

fn resolved_scope(input: &ResolutionInput, default: &str) -> String {
  if input.scope_hint.is_empty() {
    default.to_string()
  } else {
    input.scope_hint.clone()
  }
}

/// Extract the test name from an utterance by scanning for any marker
/// in `TEST_NAME_MARKERS`. The token after the marker is taken until
/// whitespace or trailing punctuation. Returns the first match.
fn extract_test_name(utterance: &str) -> Option<String> {
  let lowered = utterance.to_lowercase();
  for marker in TEST_NAME_MARKERS {
    // Markers are matched case-insensitively for "test " (English)
    // but Korean markers are already case-insensitive by nature.
    let search_in = if marker.is_ascii() {
      lowered.as_str()
    } else {
      utterance
    };
    if let Some(pos) = search_in.find(marker) {
      let after = &utterance[pos + marker.len()..];
      let name = after
        .split(|c: char| c.is_whitespace() || matches!(c, '.' | ',' | ';' | '!' | '?' | ':' | '/'))
        .find(|s| !s.is_empty())
        .unwrap_or("")
        .trim_matches(|c: char| matches!(c, '"' | '\'' | '`'))
        .to_string();
      if !name.is_empty() {
        return Some(name);
      }
    }
  }
  None
}

/// Main dispatcher. Reads `operation_candidate`, routes to the
/// per-transform resolver.
pub fn resolve_parameters(input: &ResolutionInput) -> ResolutionVerdict {
  match input.operation_candidate.as_str() {
    "rename-symbol" => resolve_rename_symbol(input),
    "remove-unused-import" => resolve_remove_unused_import(input),
    "add-test-stub" => resolve_add_test_stub(input),
    "add-import" => resolve_add_import(input),
    "lookup-algebraic-equivalent" => resolve_lookup_algebraic_equivalent(input),
    "lookup-chemical-reaction" => resolve_lookup_chemical_reaction(input),
    other => ResolutionVerdict::ResolutionHeld {
      transform: other.to_string(),
      held_kind: ResolutionHeldKind::TransformNotSupportedByResolver,
      missing_slots: Vec::new(),
      partial_resolution: BTreeMap::new(),
      reason: format!(
        "parameter-resolution v0 does not yet support transform `{other}`; only rename-symbol, remove-unused-import, add-test-stub, and add-import are wired"
      ),
    },
  }
}

fn resolve_rename_symbol(input: &ResolutionInput) -> ResolutionVerdict {
  let mut partial: BTreeMap<String, String> = BTreeMap::new();
  let mut missing: Vec<String> = Vec::new();

  // target_path resolution.
  let target_path = resolved_target_path(input);
  if let Some(ref tp) = target_path {
    partial.insert("target_paths".to_string(), tp.clone());
  } else {
    missing.push("target_path".to_string());
  }

  // language from target_path.
  let language = target_path.as_deref().and_then(language_from_target_path);
  if let Some(lang) = language {
    partial.insert("language".to_string(), lang.to_string());
  }

  // (old, new) extraction.
  let pair = extract_rename_pair(&input.utterance);
  if let Some((ref o, ref n)) = pair {
    partial.insert("old_name".to_string(), o.clone());
    partial.insert("new_name".to_string(), n.clone());
  } else {
    missing.push("old_name".to_string());
    missing.push("new_name".to_string());
  }

  let scope = resolved_scope(input, DEFAULT_RENAME_SCOPE);
  partial.insert("scope".to_string(), scope.clone());

  // Held conditions, in priority order. Note: ordering matters —
  // most-foundational missing first so the caller asks the most
  // useful follow-up question first.
  if target_path.is_none() {
    return ResolutionVerdict::ResolutionHeld {
      transform: "rename-symbol".to_string(),
      held_kind: ResolutionHeldKind::MissingTargetPath,
      missing_slots: missing,
      partial_resolution: partial,
      reason: "no target file path resolved from utterance or caller input; ask the operator which file(s) to rename in".to_string(),
    };
  }
  if language.is_none() {
    return ResolutionVerdict::ResolutionHeld {
      transform: "rename-symbol".to_string(),
      held_kind: ResolutionHeldKind::LanguageNotDerivable,
      missing_slots: vec!["language".to_string()],
      partial_resolution: partial,
      reason: format!(
        "target_path `{}` has no recognized extension; ask the operator which language",
        target_path.as_deref().unwrap_or("")
      ),
    };
  }
  let Some((old, new)) = pair else {
    return ResolutionVerdict::ResolutionHeld {
      transform: "rename-symbol".to_string(),
      held_kind: if missing.contains(&"old_name".to_string()) {
        ResolutionHeldKind::MissingOldName
      } else {
        ResolutionHeldKind::MissingNewName
      },
      missing_slots: missing,
      partial_resolution: partial,
      reason:
        "could not extract old/new symbol pair from utterance; ask the operator to say it explicitly (e.g. 'foo를 bar로 바꿔줘' or 'rename foo to bar')"
          .to_string(),
    };
  };
  // Identifier sanity.
  if !is_valid_identifier(&old) || !is_valid_identifier(&new) {
    return ResolutionVerdict::ResolutionRejected {
      transform: "rename-symbol".to_string(),
      held_kind: ResolutionHeldKind::InvalidIdentifier,
      reason: format!(
        "extracted symbol pair (old=`{old}`, new=`{new}`) contains a non-identifier token"
      ),
    };
  }
  if old == new {
    return ResolutionVerdict::ResolutionRejected {
      transform: "rename-symbol".to_string(),
      held_kind: ResolutionHeldKind::OldEqualsNew,
      reason: format!("old_name and new_name are identical: `{old}`"),
    };
  }

  let request = serde_json::json!({
    "old_name": old,
    "new_name": new,
    "target_paths": [target_path.clone().unwrap()],
    "language": language.unwrap(),
    "scope": scope,
  });
  ResolutionVerdict::ResolutionReady {
    transform: "rename-symbol".to_string(),
    request,
    resolved_fields: partial,
  }
}

fn resolve_remove_unused_import(input: &ResolutionInput) -> ResolutionVerdict {
  let mut partial: BTreeMap<String, String> = BTreeMap::new();
  let mut missing: Vec<String> = Vec::new();

  let target_path = resolved_target_path(input);
  if let Some(ref tp) = target_path {
    partial.insert("target_path".to_string(), tp.clone());
  } else {
    missing.push("target_path".to_string());
  }

  let language = target_path.as_deref().and_then(language_from_target_path);
  if let Some(lang) = language {
    partial.insert("language".to_string(), lang.to_string());
  }

  let scope = resolved_scope(input, DEFAULT_REMOVE_UNUSED_IMPORT_SCOPE);
  partial.insert("scope".to_string(), scope.clone());

  if target_path.is_none() {
    return ResolutionVerdict::ResolutionHeld {
      transform: "remove-unused-import".to_string(),
      held_kind: ResolutionHeldKind::MissingTargetPath,
      missing_slots: missing,
      partial_resolution: partial,
      reason: "no target file path resolved; ask the operator which file's imports to clean"
        .to_string(),
    };
  }
  if language.is_none() {
    return ResolutionVerdict::ResolutionHeld {
      transform: "remove-unused-import".to_string(),
      held_kind: ResolutionHeldKind::LanguageNotDerivable,
      missing_slots: vec!["language".to_string()],
      partial_resolution: partial,
      reason: format!(
        "target_path `{}` has no recognized extension; ask the operator which language",
        target_path.as_deref().unwrap_or("")
      ),
    };
  }
  if input.candidate_imports_for_unused.is_empty() {
    return ResolutionVerdict::ResolutionHeld {
      transform: "remove-unused-import".to_string(),
      held_kind: ResolutionHeldKind::MissingCandidateImports,
      missing_slots: vec!["candidate_imports".to_string()],
      partial_resolution: partial,
      reason: "no candidate unused imports were supplied — host symbol resolver must produce the candidate set before this transform can proceed".to_string(),
    };
  }

  let candidate_imports: Vec<serde_json::Value> = input
    .candidate_imports_for_unused
    .iter()
    .map(|m| {
      serde_json::json!({
        "module": m,
        "used_in_macro": false,
        "behind_cfg": false,
      })
    })
    .collect();

  let request = serde_json::json!({
    "target_path": target_path.clone().unwrap(),
    "language": language.unwrap(),
    "candidate_imports": candidate_imports,
    "scope": scope,
  });
  ResolutionVerdict::ResolutionReady {
    transform: "remove-unused-import".to_string(),
    request,
    resolved_fields: partial,
  }
}

fn resolve_add_test_stub(input: &ResolutionInput) -> ResolutionVerdict {
  let mut partial: BTreeMap<String, String> = BTreeMap::new();
  let mut missing: Vec<String> = Vec::new();

  let target_path = resolved_target_path(input);
  if let Some(ref tp) = target_path {
    partial.insert("target_module".to_string(), tp.clone());
  } else {
    missing.push("target_path".to_string());
  }

  let language = target_path.as_deref().and_then(language_from_target_path);
  if let Some(lang) = language {
    partial.insert("language".to_string(), lang.to_string());
  }

  let test_name = extract_test_name(&input.utterance);
  if let Some(ref n) = test_name {
    partial.insert("test_name".to_string(), n.clone());
  } else {
    missing.push("test_name".to_string());
  }

  if target_path.is_none() {
    return ResolutionVerdict::ResolutionHeld {
      transform: "add-test-stub".to_string(),
      held_kind: ResolutionHeldKind::MissingTargetPath,
      missing_slots: missing,
      partial_resolution: partial,
      reason: "no target file path resolved; ask the operator which file to add the test stub in"
        .to_string(),
    };
  }
  if language.is_none() {
    return ResolutionVerdict::ResolutionHeld {
      transform: "add-test-stub".to_string(),
      held_kind: ResolutionHeldKind::LanguageNotDerivable,
      missing_slots: vec!["language".to_string()],
      partial_resolution: partial,
      reason: format!(
        "target_path `{}` has no recognized extension; ask the operator which language",
        target_path.as_deref().unwrap_or("")
      ),
    };
  }
  let Some(name) = test_name else {
    return ResolutionVerdict::ResolutionHeld {
      transform: "add-test-stub".to_string(),
      held_kind: ResolutionHeldKind::MissingTestName,
      missing_slots: missing,
      partial_resolution: partial,
      reason: "could not extract a test name from utterance; ask the operator to say it explicitly (e.g. 'add test foo_handles_empty' or '테스트 foo_handles_empty 추가해줘')".to_string(),
    };
  };
  if !is_valid_identifier(&name) {
    return ResolutionVerdict::ResolutionRejected {
      transform: "add-test-stub".to_string(),
      held_kind: ResolutionHeldKind::InvalidIdentifier,
      reason: format!("extracted test name `{name}` is not a valid identifier"),
    };
  }

  let request = serde_json::json!({
    "target_module": target_path.clone().unwrap(),
    "test_name": name,
    "language": language.unwrap(),
    "intent": "add test stub",
    // place: null → carrier uses language default via
    // AddTestStubPlace::default_for.
    "place": serde_json::Value::Null,
  });
  ResolutionVerdict::ResolutionReady {
    transform: "add-test-stub".to_string(),
    request,
    resolved_fields: partial,
  }
}

/// Try to extract an import spec from raw NL when the host did not
/// supply one. Recognized patterns mirror the language-specific
/// forms the `add_import` carrier validates. Returns `None` when no
/// pattern fires — the resolver then Holds with `MissingImportSpec`.
/// Check whether `token` matches the shape of `kind`. Generic over
/// all identifier kinds in `IMPORT_SPEC_PATTERNS`.
fn token_matches_kind(token: &str, kind: ImportNameKind) -> bool {
  let mut chars = token.chars();
  match chars.next() {
    None => return false,
    Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
    _ => return false,
  }
  match kind {
    ImportNameKind::PythonIdent => chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.'),
    ImportNameKind::RustPath => chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':'),
  }
}

/// Take the leading run of `token`'s characters that match `kind`
/// and return the prefix. Used to strip trailing punctuation like
/// `;` or `"` from a matched name token. Empty result → caller
/// treats as "no name".
fn take_kind_prefix(token: &str, kind: ImportNameKind) -> String {
  let mut out = String::new();
  for (i, c) in token.chars().enumerate() {
    let ok = if i == 0 {
      c.is_ascii_alphabetic() || c == '_'
    } else {
      match kind {
        ImportNameKind::PythonIdent => c.is_ascii_alphanumeric() || c == '_' || c == '.',
        ImportNameKind::RustPath => c.is_ascii_alphanumeric() || c == '_' || c == ':',
      }
    };
    if !ok {
      break;
    }
    out.push(c);
  }
  out
}

/// Try one `ImportSpecPattern` row against an utterance. Returns
/// the canonical spec string on success.
fn try_one_import_pattern(utterance: &str, pat: &ImportSpecPattern) -> Option<String> {
  let trimmed = utterance.trim();
  if trimmed.is_empty() {
    return None;
  }
  let idx = trimmed.find(pat.lead)?;
  let after = &trimmed[idx + pat.lead.len()..];

  match pat.shape {
    ImportSpecShape::FromImport => {
      let mut tokens = after.split_whitespace();
      let module = tokens.next()?;
      let middle_tok = tokens.next()?;
      let name = tokens.next()?;
      if middle_tok != pat.middle
        || !token_matches_kind(module, pat.name_kind)
        || !token_matches_kind(name, pat.name_kind)
      {
        return None;
      }
      Some(format!("{}{module} {} {name}", pat.lead, pat.middle))
    }
    ImportSpecShape::ImportName => {
      // Guard: if the matched `lead` was preceded by another
      // pattern's `lead` keyword (e.g. `import ` after `from `),
      // skip this row — the from-form's pattern handles it
      // verbatim.
      let preceding = trimmed[..idx].trim_end();
      let preceded_by_from = IMPORT_SPEC_PATTERNS.iter().any(|other| {
        other.shape == ImportSpecShape::FromImport
          && other.language == pat.language
          && preceding
            .split_whitespace()
            .last()
            .map(|w| other.lead.starts_with(w) && other.lead.len() > w.len())
            .unwrap_or(false)
      });
      if preceded_by_from {
        return None;
      }
      let first_token = after.split_whitespace().next()?;
      let name = take_kind_prefix(first_token, pat.name_kind);
      if name.is_empty() {
        return None;
      }
      Some(format!("{}{name}", pat.lead))
    }
  }
}

/// Try every `IMPORT_SPEC_PATTERNS` row for the given language in
/// declarative order. First match wins. Generic over languages —
/// adding a new language is a `.px` row + a Rust mirror row, no new
/// branch.
fn extract_import_spec_from_utterance(utterance: &str, language: &str) -> Option<String> {
  for pat in IMPORT_SPEC_PATTERNS {
    if pat.language != language {
      continue;
    }
    if let Some(spec) = try_one_import_pattern(utterance, pat) {
      return Some(strip_use_path_suffix(spec, pat));
    }
  }
  None
}

/// For the Rust `use` shape, the canonical carrier form is the path
/// alone (no leading `use ` and no trailing `;`). The Rust mirror
/// of the existing `AddImportRequest` expects e.g.
/// `"std::collections::HashMap"`, not `"use std::collections::HashMap"`.
/// This is the single transform-canonical-form remap; all other
/// language patterns return their `{lead}{name}` form verbatim.
fn strip_use_path_suffix(spec: String, pat: &ImportSpecPattern) -> String {
  if pat.language == "rust" && pat.shape == ImportSpecShape::ImportName {
    spec
      .trim_start_matches("use ")
      .trim_end_matches(';')
      .to_string()
  } else {
    spec
  }
}

fn resolve_add_import(input: &ResolutionInput) -> ResolutionVerdict {
  let mut partial: BTreeMap<String, String> = BTreeMap::new();
  let mut missing: Vec<String> = Vec::new();

  let target_path = resolved_target_path(input);
  if let Some(ref tp) = target_path {
    partial.insert("target_path".to_string(), tp.clone());
  } else {
    missing.push("target_path".to_string());
  }

  let language = target_path.as_deref().and_then(language_from_target_path);
  if let Some(lang) = language {
    partial.insert("language".to_string(), lang.to_string());
  }

  // import_spec: prefer host-supplied; fall back to NL extraction.
  let import_spec = if !input.candidate_import_spec.is_empty() {
    Some(input.candidate_import_spec.clone())
  } else if let Some(lang) = language {
    extract_import_spec_from_utterance(&input.utterance, lang)
  } else {
    None
  };
  if let Some(ref spec) = import_spec {
    partial.insert("import_spec".to_string(), spec.clone());
  } else {
    missing.push("import_spec".to_string());
  }

  if target_path.is_none() {
    return ResolutionVerdict::ResolutionHeld {
      transform: "add-import".to_string(),
      held_kind: ResolutionHeldKind::MissingTargetPath,
      missing_slots: missing,
      partial_resolution: partial,
      reason: "no target file path resolved; ask the operator which file to add the import to"
        .to_string(),
    };
  }
  if language.is_none() {
    return ResolutionVerdict::ResolutionHeld {
      transform: "add-import".to_string(),
      held_kind: ResolutionHeldKind::LanguageNotDerivable,
      missing_slots: vec!["language".to_string()],
      partial_resolution: partial,
      reason: format!(
        "target_path `{}` has no recognized extension; ask the operator which language",
        target_path.as_deref().unwrap_or("")
      ),
    };
  }
  let Some(spec) = import_spec else {
    return ResolutionVerdict::ResolutionHeld {
      transform: "add-import".to_string(),
      held_kind: ResolutionHeldKind::MissingImportSpec,
      missing_slots: missing,
      partial_resolution: partial,
      reason: "no import spec resolved from NL or host evidence; ask the operator for the import to add (e.g. 'import os' for Python, 'std::collections::HashMap' for Rust)".to_string(),
    };
  };

  let request = serde_json::json!({
    "target_path": target_path.clone().unwrap(),
    "language": language.unwrap(),
    "import_spec": spec,
    "if_already_present": serde_json::Value::Null,
  });
  ResolutionVerdict::ResolutionReady {
    transform: "add-import".to_string(),
    request,
    resolved_fields: partial,
  }
}

// ─── math: lookup-algebraic-equivalent ─────────────────────────────

/// Korean question suffixes the math-question detector strips from
/// the utterance. The substring *before* the suffix is the
/// canonical_form candidate. Generic — any utterance with one of
/// these endings is treated as a math-equivalent question. The list
/// stays small (high precision); operator can disambiguate via
/// follow-up if a borderline form isn't detected.
const MATH_QUESTION_SUFFIXES: &[&str] = &[
  "는 뭐야",
  "은 뭐야",
  "는 뭐야?",
  "은 뭐야?",
  "는 무엇",
  "은 무엇",
  "는 무엇인가",
  "은 무엇인가",
  "은 무엇이야",
  "는 무엇이야",
  "은 무엇이지",
  "는 무엇이지",
  "은 어떻게 돼",
  "는 어떻게 돼",
  "전개해",
  "전개해줘",
  "전개해 줘",
  "동치인 식",
  "동치인 식은",
  "동치 표현",
  " is equivalent to what",
  " what is the equivalent of",
  " what is",
];

/// Whether a string fragment plausibly looks like a math expression
/// in the *with-suffix* case (operator already signaled it's a math
/// question via "는 뭐야"). Lenient: needs an operator and a token.
fn looks_like_math_expression_after_suffix(s: &str) -> bool {
  let trimmed = s.trim();
  if trimmed.is_empty() {
    return false;
  }
  let has_op = trimmed
    .chars()
    .any(|c| matches!(c, '+' | '-' | '*' | '/' | '^' | '=' | '∧' | '∨' | '¬' | '⊕'));
  if !has_op {
    return false;
  }
  trimmed
    .chars()
    .any(|c| c.is_ascii_digit() || c.is_alphabetic())
}

/// Strict math-expression check for the *bare* (no-suffix) case.
/// Needs a strong math signal to avoid catching prose like
/// `"rename foo to bar in src/a.py"` (the `/` would slip past the
/// lenient check). Strong signals: `^` (exponent), boolean operators
/// `∧ ∨ ¬ ⊕`, `=` (equation), OR multiple arithmetic operators.
fn looks_like_bare_math_expression(s: &str) -> bool {
  let trimmed = s.trim();
  if trimmed.is_empty() {
    return false;
  }
  // Strong single-char signals.
  if trimmed
    .chars()
    .any(|c| matches!(c, '^' | '=' | '∧' | '∨' | '¬' | '⊕'))
  {
    return looks_like_math_expression_after_suffix(trimmed);
  }
  // Otherwise require ≥2 arithmetic operators (compound expression
  // shape) AND at least one digit. Prose like `"a + b"` alone is
  // ambiguous; `"a + b - c"` or `"2 * x"` are clearer signals.
  let arith_op_count = trimmed
    .chars()
    .filter(|c| matches!(c, '+' | '-' | '*' | '/'))
    .count();
  let has_digit = trimmed.chars().any(|c| c.is_ascii_digit());
  arith_op_count >= 2 && has_digit
}

/// Strip the leading Korean topic/possessive marker around the math
/// expression. Many Korean math phrasings put a marker right after
/// the expression: "x^2 + 2*x*y + y^2 *는* 뭐야?" — the suffix list
/// includes the marker, so what's left after `strip_suffix` is the
/// expression alone. This helper handles the slightly different case
/// where the marker is followed by trailing whitespace/punctuation.
fn trim_trailing_korean_marker(s: &str) -> &str {
  let trimmed = s.trim();
  for marker in ["는", "은", "을", "를", "이", "가"] {
    if let Some(stripped) = trimmed.strip_suffix(marker) {
      let stripped = stripped.trim_end();
      if !stripped.is_empty() {
        return stripped;
      }
    }
  }
  trimmed
}

/// Detect the algebraic sub-domain from the expression itself.
/// Order matters — boolean-algebra symbols are checked first because
/// they're disjoint from polynomial. Default = polynomial (the most
/// common math-equivalent case in practice).
pub fn detect_algebraic_language(expression: &str) -> &'static str {
  let has_bool = expression
    .chars()
    .any(|c| matches!(c, '∧' | '∨' | '¬' | '⊕'));
  if has_bool {
    return "boolean-algebra";
  }
  let has_trig = ["sin", "cos", "tan", "π"]
    .iter()
    .any(|tok| expression.contains(tok));
  if has_trig {
    return "trig";
  }
  "polynomial"
}

/// Extract `canonical_form` + `language` from a math-question
/// utterance. Returns `None` if no math-question suffix is found OR
/// if the prefix doesn't look like a math expression.
///
/// The split is intentional: pnix does not try to *evaluate* the
/// expression. It just isolates it so retrieval can ask for an
/// equivalent. The actual equivalence check is downstream
/// (CAS adapter or operator-followup).
pub fn extract_math_canonical_form(utterance: &str) -> Option<(String, &'static str)> {
  let trimmed = utterance.trim().trim_end_matches(['?', '.', '!']);
  for suffix in MATH_QUESTION_SUFFIXES {
    if let Some(prefix) = trimmed.strip_suffix(suffix) {
      let candidate = trim_trailing_korean_marker(prefix).trim();
      if looks_like_math_expression_after_suffix(candidate) {
        let language = detect_algebraic_language(candidate);
        return Some((candidate.to_string(), language));
      }
    }
  }
  // No suffix matched — fall back to strict check. Covers operator
  // typing the expression alone (e.g. "x^2 + 2*x*y + y^2") or after
  // a separate intent classifier already decided it's a math query.
  if looks_like_bare_math_expression(trimmed) {
    let language = detect_algebraic_language(trimmed);
    return Some((trimmed.to_string(), language));
  }
  None
}

// ─── chemistry: lookup-chemical-reaction ───────────────────────────

/// Korean question suffixes for chemistry questions. Recognized
/// shapes for v0:
///   `<reactants> 가 어떻게 반응해?`
///   `<reactants> 는 무엇이 돼?`
///   `<reactants> 가 생성하는 건?`
///   `<reactants> 반응 생성물은?`
///   `<reactants> products?` (English fallback)
const CHEMISTRY_QUESTION_SUFFIXES: &[&str] = &[
  "가 어떻게 반응해",
  "이 어떻게 반응해",
  "는 어떻게 반응",
  "은 어떻게 반응",
  "가 어떻게 반응",
  "이 어떻게 반응",
  "반응 생성물은",
  "반응 생성물",
  "의 생성물은",
  "의 생성물",
  "는 무엇이 돼",
  "은 무엇이 돼",
  "가 생성하는",
  "이 생성하는",
  " products?",
  " products",
  " what does ",
];

/// Whether a string fragment plausibly looks like a chemistry
/// reactant set. Heuristic: contains a chemical-formula token
/// (capital letter followed by 0+ lowercase + digits, e.g. `H2`,
/// `O2`, `NaCl`, `CO2`) AND optionally a `+` separator.
fn looks_like_chemistry_reactants(s: &str) -> bool {
  let trimmed = s.trim();
  if trimmed.is_empty() {
    return false;
  }
  // Chemical formula token detector — scan for capital letter
  // followed by 0+ lowercase + 0+ digits.
  let mut has_formula_token = false;
  let chars: Vec<char> = trimmed.chars().collect();
  let mut i = 0;
  while i < chars.len() {
    if chars[i].is_ascii_uppercase() {
      let mut j = i + 1;
      while j < chars.len() && chars[j].is_ascii_lowercase() {
        j += 1;
      }
      let mut has_digit = false;
      while j < chars.len() && chars[j].is_ascii_digit() {
        j += 1;
        has_digit = true;
      }
      // Token must be ≥ 1 letter; element-with-digit (like H2) is
      // strong signal; bare 2-letter element (like Na, Cl, Fe) also
      // valid.
      let token_len = j - i;
      if token_len >= 2 || has_digit {
        has_formula_token = true;
        break;
      }
      i = j.max(i + 1);
    } else {
      i += 1;
    }
  }
  has_formula_token
}

/// Optional conditions extraction — looks for `(...)`, `[catalyst]`,
/// or after-comma context.
fn extract_chemistry_conditions(s: &str) -> Option<String> {
  let trimmed = s.trim();
  // v0: look for parenthesized context.
  if let Some(open) = trimmed.find('(') {
    if let Some(close) = trimmed[open..].find(')') {
      let cond = trimmed[open + 1..open + close].trim();
      if !cond.is_empty() {
        return Some(cond.to_string());
      }
    }
  }
  None
}

/// Detect chemistry sub-domain. v0 default: `inorganic`. Hooks for
/// `organic` / `biochem` reserved for future cue extractors.
pub fn detect_chemistry_language(_reactants: &str) -> &'static str {
  "inorganic"
}

/// Extract `reactants` (+ optional `conditions`) + `language` from
/// a chemistry-question utterance. Returns `None` if no chemistry
/// suffix is found AND the bare expression doesn't look like
/// reactants.
pub fn extract_chemistry_canonical_form(
  utterance: &str,
) -> Option<(String, Option<String>, &'static str)> {
  let trimmed = utterance.trim().trim_end_matches(['?', '.', '!']);
  for suffix in CHEMISTRY_QUESTION_SUFFIXES {
    if let Some(prefix) = trimmed.strip_suffix(suffix) {
      let candidate = prefix.trim_end().trim();
      // Strip trailing Korean marker that may precede the suffix.
      let candidate = {
        let mut s = candidate;
        for m in ["는", "은", "을", "를", "이", "가"] {
          if let Some(stripped) = s.strip_suffix(m) {
            s = stripped.trim_end();
          }
        }
        s
      };
      if looks_like_chemistry_reactants(candidate) {
        let conditions = extract_chemistry_conditions(candidate);
        let reactants_clean = match &conditions {
          Some(_) => candidate
            .split('(')
            .next()
            .unwrap_or(candidate)
            .trim()
            .to_string(),
          None => candidate.to_string(),
        };
        let language = detect_chemistry_language(&reactants_clean);
        return Some((reactants_clean, conditions, language));
      }
    }
  }
  None
}

fn resolve_lookup_chemical_reaction(input: &ResolutionInput) -> ResolutionVerdict {
  let mut partial: BTreeMap<String, String> = BTreeMap::new();

  let Some((reactants, conditions, language)) = extract_chemistry_canonical_form(&input.utterance)
  else {
    return ResolutionVerdict::ResolutionRejected {
      transform: "lookup-chemical-reaction".to_string(),
      held_kind: ResolutionHeldKind::InvalidIdentifier,
      reason: format!(
        "utterance `{}` does not contain recognizable chemistry reactants; ask the operator to rephrase",
        input.utterance
      ),
    };
  };
  partial.insert("reactants".to_string(), reactants.clone());
  partial.insert("language".to_string(), language.to_string());
  // conditions: default empty string if not extracted (substrate
  // distinguishes "no conditions specified" from "unknown" via
  // empty vs absent; v0 uses empty).
  partial.insert(
    "conditions".to_string(),
    conditions.clone().unwrap_or_default(),
  );

  ResolutionVerdict::ResolutionHeld {
    transform: "lookup-chemical-reaction".to_string(),
    held_kind: ResolutionHeldKind::MissingChemistryProducts,
    missing_slots: vec!["products".to_string()],
    partial_resolution: partial,
    reason: format!(
      "operator asked about reactants `{reactants}` (cond `{}`, lang `{language}`); retrieve products via ankh / external CAS / operator follow-up",
      conditions.as_deref().unwrap_or(""),
    ),
  }
}

fn resolve_lookup_algebraic_equivalent(input: &ResolutionInput) -> ResolutionVerdict {
  let mut partial: BTreeMap<String, String> = BTreeMap::new();

  // Extract canonical_form from utterance. If the utterance doesn't
  // carry a math expression, this is a hard rejection: operator
  // asked for an equivalent but didn't give us the expression.
  let Some((canonical, language)) = extract_math_canonical_form(&input.utterance) else {
    return ResolutionVerdict::ResolutionRejected {
      transform: "lookup-algebraic-equivalent".to_string(),
      held_kind: ResolutionHeldKind::InvalidIdentifier,
      reason: format!(
        "utterance `{}` does not contain a recognizable math expression; ask the operator to rephrase",
        input.utterance
      ),
    };
  };
  partial.insert("canonical_form".to_string(), canonical.clone());
  partial.insert("language".to_string(), language.to_string());

  // The equivalent_form is what retrieval must recover. Always
  // missing at lift time — this is the math lane's load-bearing
  // Held shape.
  ResolutionVerdict::ResolutionHeld {
    transform: "lookup-algebraic-equivalent".to_string(),
    held_kind: ResolutionHeldKind::MissingAlgebraicEquivalent,
    missing_slots: vec!["equivalent_form".to_string()],
    partial_resolution: partial,
    reason: format!(
      "operator asked for an equivalent of `{canonical}` (lang `{language}`); retrieve via ankh / external CAS / operator follow-up"
    ),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn inp(transform: &str, utt: &str) -> ResolutionInput {
    ResolutionInput {
      operation_candidate: transform.to_string(),
      utterance: utt.to_string(),
      ..Default::default()
    }
  }

  // ─── transform-not-supported ──────────────────────────────────

  #[test]
  fn unsupported_transform_is_held() {
    let v = resolve_parameters(&inp("change-signature", "anything"));
    assert!(matches!(
      v,
      ResolutionVerdict::ResolutionHeld {
        held_kind: ResolutionHeldKind::TransformNotSupportedByResolver,
        ..
      }
    ));
  }

  // ─── rename-symbol Ready path ─────────────────────────────────

  #[test]
  fn rename_symbol_ready_korean_explicit_request() {
    let v = resolve_parameters(&inp(
      "rename-symbol",
      "이 함수 이름 foo를 bar로 바꿔줘 src/a.rs",
    ));
    match v {
      ResolutionVerdict::ResolutionReady {
        transform, request, ..
      } => {
        assert_eq!(transform, "rename-symbol");
        assert_eq!(request["old_name"], "foo");
        assert_eq!(request["new_name"], "bar");
        assert_eq!(request["language"], "rust");
        assert_eq!(request["target_paths"][0], "src/a.rs");
      }
      other => panic!("expected Ready, got {other:?}"),
    }
  }

  #[test]
  fn rename_symbol_ready_english_explicit_request() {
    let v = resolve_parameters(&inp("rename-symbol", "rename frob to glob in src/main.rs"));
    match v {
      ResolutionVerdict::ResolutionReady { request, .. } => {
        assert_eq!(request["old_name"], "frob");
        assert_eq!(request["new_name"], "glob");
        assert_eq!(request["language"], "rust");
      }
      other => panic!("expected Ready, got {other:?}"),
    }
  }

  #[test]
  fn rename_symbol_ready_uses_explicit_target_path_over_utterance() {
    let mut i = inp("rename-symbol", "rename foo to bar");
    i.target_path = "src/explicit.py".into();
    match resolve_parameters(&i) {
      ResolutionVerdict::ResolutionReady { request, .. } => {
        assert_eq!(request["target_paths"][0], "src/explicit.py");
        assert_eq!(request["language"], "python");
      }
      other => panic!("expected Ready, got {other:?}"),
    }
  }

  // ─── rename-symbol Held path (the load-bearing case) ──────────

  #[test]
  fn rename_symbol_held_when_no_target_path() {
    let v = resolve_parameters(&inp("rename-symbol", "이 함수 이름 바꿔줘"));
    match v {
      ResolutionVerdict::ResolutionHeld {
        held_kind: ResolutionHeldKind::MissingTargetPath,
        missing_slots,
        ..
      } => {
        assert!(missing_slots.iter().any(|s| s == "target_path"));
      }
      other => panic!("expected MissingTargetPath, got {other:?}"),
    }
  }

  #[test]
  fn rename_symbol_held_when_no_symbol_pair_but_target_path_present() {
    // target_path resolves from "src/a.rs" but no foo→bar.
    let v = resolve_parameters(&inp("rename-symbol", "이 함수 이름 바꿔줘 src/a.rs"));
    match v {
      ResolutionVerdict::ResolutionHeld {
        held_kind: ResolutionHeldKind::MissingOldName,
        missing_slots,
        partial_resolution,
        ..
      } => {
        // Partial info still surfaced — operator doesn't re-type the path.
        assert_eq!(partial_resolution.get("target_paths").unwrap(), "src/a.rs");
        assert_eq!(partial_resolution.get("language").unwrap(), "rust");
        assert!(missing_slots.iter().any(|s| s == "old_name"));
        assert!(missing_slots.iter().any(|s| s == "new_name"));
      }
      other => panic!("expected MissingOldName Held, got {other:?}"),
    }
  }

  #[test]
  fn rename_symbol_held_when_language_not_derivable() {
    let mut i = inp("rename-symbol", "rename foo to bar");
    i.target_path = "src/unknown.xyz".into();
    match resolve_parameters(&i) {
      ResolutionVerdict::ResolutionHeld {
        held_kind: ResolutionHeldKind::LanguageNotDerivable,
        ..
      } => {}
      other => panic!("expected LanguageNotDerivable Held, got {other:?}"),
    }
  }

  // ─── rename-symbol Rejected path ──────────────────────────────

  #[test]
  fn rename_symbol_rejected_when_old_equals_new() {
    let v = resolve_parameters(&inp(
      "rename-symbol",
      "이 함수 이름 foo를 foo로 바꿔줘 src/a.rs",
    ));
    assert!(matches!(
      v,
      ResolutionVerdict::ResolutionRejected {
        held_kind: ResolutionHeldKind::OldEqualsNew,
        ..
      }
    ));
  }

  #[test]
  fn rename_symbol_rejected_when_invalid_identifier() {
    let v = resolve_parameters(&inp(
      "rename-symbol",
      "이 함수 이름 1bad를 newer로 바꿔줘 src/a.rs",
    ));
    assert!(matches!(
      v,
      ResolutionVerdict::ResolutionRejected {
        held_kind: ResolutionHeldKind::InvalidIdentifier,
        ..
      }
    ));
  }

  // ─── remove-unused-import paths ───────────────────────────────

  #[test]
  fn remove_unused_import_held_when_no_candidate_imports_supplied() {
    let mut i = inp("remove-unused-import", "src/a.py에서 안 쓰는 import 지워줘");
    // No candidate_imports_for_unused supplied — Held.
    let v = resolve_parameters(&i);
    match v {
      ResolutionVerdict::ResolutionHeld {
        held_kind: ResolutionHeldKind::MissingCandidateImports,
        partial_resolution,
        ..
      } => {
        assert_eq!(partial_resolution.get("target_path").unwrap(), "src/a.py");
        assert_eq!(partial_resolution.get("language").unwrap(), "python");
      }
      other => panic!("expected MissingCandidateImports Held, got {other:?}"),
    };
    // Now supply candidate imports — should become Ready.
    i.candidate_imports_for_unused = vec!["os".to_string(), "sys".to_string()];
    match resolve_parameters(&i) {
      ResolutionVerdict::ResolutionReady { request, .. } => {
        assert_eq!(request["target_path"], "src/a.py");
        assert_eq!(request["language"], "python");
        assert_eq!(request["candidate_imports"].as_array().unwrap().len(), 2);
        assert_eq!(request["candidate_imports"][0]["module"], "os");
      }
      other => panic!("expected Ready, got {other:?}"),
    }
  }

  #[test]
  fn remove_unused_import_held_when_no_target_path() {
    let v = resolve_parameters(&inp("remove-unused-import", "안 쓰는 import 지워줘"));
    assert!(matches!(
      v,
      ResolutionVerdict::ResolutionHeld {
        held_kind: ResolutionHeldKind::MissingTargetPath,
        ..
      }
    ));
  }

  // ─── target-path extraction sanity ────────────────────────────

  #[test]
  fn extract_path_with_directory_prefix() {
    let p = extract_target_path_from_utterance("foo crates/bar/src/main.rs baz");
    assert_eq!(p.as_deref(), Some("crates/bar/src/main.rs"));
  }

  #[test]
  fn extract_path_picks_first_match() {
    let p = extract_target_path_from_utterance("touch a.py and b.py");
    assert_eq!(p.as_deref(), Some("a.py"));
  }

  #[test]
  fn extract_path_returns_none_when_no_extension() {
    assert!(extract_target_path_from_utterance("just plain text").is_none());
  }

  #[test]
  fn language_from_target_path_handles_typescript_variants() {
    assert_eq!(language_from_target_path("foo.tsx"), Some("typescript"));
    assert_eq!(language_from_target_path("foo.mjs"), Some("javascript"));
    assert_eq!(language_from_target_path("foo.unknown"), None);
  }

  // ─── add-test-stub paths ───────────────────────────────────────

  #[test]
  fn add_test_stub_ready_english() {
    let v = resolve_parameters(&inp(
      "add-test-stub",
      "add test handles_empty_input in tests/foo.rs",
    ));
    match v {
      ResolutionVerdict::ResolutionReady {
        transform, request, ..
      } => {
        assert_eq!(transform, "add-test-stub");
        assert_eq!(request["test_name"], "handles_empty_input");
        assert_eq!(request["target_module"], "tests/foo.rs");
        assert_eq!(request["language"], "rust");
      }
      other => panic!("expected Ready, got {other:?}"),
    }
  }

  #[test]
  fn add_test_stub_ready_korean() {
    let v = resolve_parameters(&inp(
      "add-test-stub",
      "테스트 handles_empty 추가해줘 tests/foo.rs",
    ));
    match v {
      ResolutionVerdict::ResolutionReady { request, .. } => {
        assert_eq!(request["test_name"], "handles_empty");
        assert_eq!(request["target_module"], "tests/foo.rs");
        assert_eq!(request["language"], "rust");
      }
      other => panic!("expected Ready, got {other:?}"),
    }
  }

  #[test]
  fn add_test_stub_held_when_no_target_path() {
    let v = resolve_parameters(&inp("add-test-stub", "add test foo_works"));
    match v {
      ResolutionVerdict::ResolutionHeld {
        held_kind: ResolutionHeldKind::MissingTargetPath,
        missing_slots,
        ..
      } => {
        assert!(missing_slots.iter().any(|s| s == "target_path"));
      }
      other => panic!("expected MissingTargetPath Held, got {other:?}"),
    }
  }

  #[test]
  fn add_test_stub_rejected_when_marker_captures_non_identifier() {
    // "테스트 " marker captures "케이스" (Korean) which is not a valid
    // ASCII identifier → Rejected(InvalidIdentifier).
    let v = resolve_parameters(&inp(
      "add-test-stub",
      "테스트 케이스 하나 추가해줘 tests/foo.rs",
    ));
    assert!(matches!(
      v,
      ResolutionVerdict::ResolutionRejected {
        held_kind: ResolutionHeldKind::InvalidIdentifier,
        ..
      }
    ));
  }

  #[test]
  fn add_test_stub_held_when_no_marker_in_utterance() {
    // No "test " / "테스트 " marker → MissingTestName.
    let v = resolve_parameters(&inp("add-test-stub", "tests/foo.rs 에 검증 케이스 추가"));
    match v {
      ResolutionVerdict::ResolutionHeld {
        held_kind: ResolutionHeldKind::MissingTestName,
        missing_slots,
        partial_resolution,
        ..
      } => {
        assert!(missing_slots.iter().any(|s| s == "test_name"));
        assert_eq!(
          partial_resolution.get("target_module").unwrap(),
          "tests/foo.rs"
        );
        assert_eq!(partial_resolution.get("language").unwrap(), "rust");
      }
      other => panic!("expected MissingTestName Held, got {other:?}"),
    }
  }

  #[test]
  fn add_test_stub_held_when_language_not_derivable() {
    let mut i = inp("add-test-stub", "add test foo_works");
    i.target_path = "tests/foo.xyz".into();
    match resolve_parameters(&i) {
      ResolutionVerdict::ResolutionHeld {
        held_kind: ResolutionHeldKind::LanguageNotDerivable,
        ..
      } => {}
      other => panic!("expected LanguageNotDerivable Held, got {other:?}"),
    }
  }

  #[test]
  fn add_test_stub_rejected_when_invalid_test_name_identifier() {
    let v = resolve_parameters(&inp("add-test-stub", "add test 1bad_name in tests/foo.rs"));
    assert!(matches!(
      v,
      ResolutionVerdict::ResolutionRejected {
        held_kind: ResolutionHeldKind::InvalidIdentifier,
        ..
      }
    ));
  }

  #[test]
  fn extract_test_name_finds_english_marker() {
    assert_eq!(
      extract_test_name("please add test foo_works to it").as_deref(),
      Some("foo_works")
    );
  }

  #[test]
  fn extract_test_name_finds_korean_marker() {
    assert_eq!(
      extract_test_name("테스트 bar_handles_zero 추가해줘").as_deref(),
      Some("bar_handles_zero")
    );
  }

  #[test]
  fn extract_test_name_none_when_no_marker() {
    assert!(extract_test_name("이 파일에 검증 케이스 추가").is_none());
  }

  // ─── add-import paths ──────────────────────────────────────────

  #[test]
  fn add_import_ready_python_host_supplied() {
    let mut i = inp("add-import", "src/util.py 에 import 추가해줘");
    i.candidate_import_spec = "import os".to_string();
    match resolve_parameters(&i) {
      ResolutionVerdict::ResolutionReady {
        transform, request, ..
      } => {
        assert_eq!(transform, "add-import");
        assert_eq!(request["target_path"], "src/util.py");
        assert_eq!(request["language"], "python");
        assert_eq!(request["import_spec"], "import os");
      }
      other => panic!("expected Ready, got {other:?}"),
    }
  }

  #[test]
  fn add_import_ready_python_extracted_from_nl() {
    let v = resolve_parameters(&inp("add-import", "src/util.py 에 import os 추가"));
    match v {
      ResolutionVerdict::ResolutionReady { request, .. } => {
        assert_eq!(request["target_path"], "src/util.py");
        assert_eq!(request["language"], "python");
        assert_eq!(request["import_spec"], "import os");
      }
      other => panic!("expected Ready, got {other:?}"),
    }
  }

  #[test]
  fn add_import_ready_python_from_import_extracted() {
    let v = resolve_parameters(&inp(
      "add-import",
      "src/util.py 에 from collections import deque 넣어줘",
    ));
    match v {
      ResolutionVerdict::ResolutionReady { request, .. } => {
        assert_eq!(request["import_spec"], "from collections import deque");
      }
      other => panic!("expected Ready, got {other:?}"),
    }
  }

  #[test]
  fn add_import_ready_rust_use_extracted() {
    let v = resolve_parameters(&inp(
      "add-import",
      "src/main.rs 에 use std::collections::HashMap; 추가",
    ));
    match v {
      ResolutionVerdict::ResolutionReady { request, .. } => {
        assert_eq!(request["language"], "rust");
        assert_eq!(request["import_spec"], "std::collections::HashMap");
      }
      other => panic!("expected Ready, got {other:?}"),
    }
  }

  #[test]
  fn add_import_held_when_no_target_path() {
    let mut i = inp("add-import", "import os 추가해줘");
    i.candidate_import_spec = "import os".to_string();
    assert!(matches!(
      resolve_parameters(&i),
      ResolutionVerdict::ResolutionHeld {
        held_kind: ResolutionHeldKind::MissingTargetPath,
        ..
      }
    ));
  }

  #[test]
  fn add_import_held_when_no_spec() {
    let v = resolve_parameters(&inp("add-import", "src/util.py 에 뭔가 추가"));
    match v {
      ResolutionVerdict::ResolutionHeld {
        held_kind: ResolutionHeldKind::MissingImportSpec,
        missing_slots,
        partial_resolution,
        ..
      } => {
        assert!(missing_slots.iter().any(|s| s == "import_spec"));
        assert_eq!(
          partial_resolution.get("target_path").unwrap(),
          "src/util.py"
        );
        assert_eq!(partial_resolution.get("language").unwrap(), "python");
      }
      other => panic!("expected MissingImportSpec Held, got {other:?}"),
    }
  }

  #[test]
  fn add_import_held_language_unknown() {
    // Explicit target_path with unsupported extension — `target_path`
    // resolves but `language_from_target_path` does not.
    let mut i = inp("add-import", "import 추가");
    i.target_path = "src/util.xyz".to_string();
    i.candidate_import_spec = "import os".to_string();
    assert!(matches!(
      resolve_parameters(&i),
      ResolutionVerdict::ResolutionHeld {
        held_kind: ResolutionHeldKind::LanguageNotDerivable,
        ..
      }
    ));
  }

  #[test]
  fn extract_import_spec_python_from_form() {
    assert_eq!(
      extract_import_spec_from_utterance(
        "src/x.py 에 from collections import deque 좀 넣어줘",
        "python",
      )
      .as_deref(),
      Some("from collections import deque"),
    );
  }

  #[test]
  fn extract_import_spec_returns_none_for_unsupported_language() {
    assert!(extract_import_spec_from_utterance("import os", "go").is_none());
  }

  // ─── math lift: korean NL → canonical_form ──────────────────

  #[test]
  fn extract_math_canonical_from_korean_question() {
    let (canonical, lang) =
      extract_math_canonical_form("x^2 + 2*x*y + y^2 는 뭐야?").expect("extracted");
    assert_eq!(canonical, "x^2 + 2*x*y + y^2");
    assert_eq!(lang, "polynomial");
  }

  #[test]
  fn extract_math_canonical_with_eun_marker() {
    let (canonical, lang) = extract_math_canonical_form("a^2 - b^2 은 뭐야?").expect("extracted");
    assert_eq!(canonical, "a^2 - b^2");
    assert_eq!(lang, "polynomial");
  }

  #[test]
  fn extract_math_canonical_detects_boolean_algebra() {
    let (canonical, lang) =
      extract_math_canonical_form("(p ∧ q) ∨ (p ∧ r) 는 뭐야?").expect("extracted");
    assert_eq!(canonical, "(p ∧ q) ∨ (p ∧ r)");
    assert_eq!(lang, "boolean-algebra");
  }

  #[test]
  fn extract_math_canonical_detects_trig() {
    let (canonical, lang) = extract_math_canonical_form("sin(2*x) 는 뭐야?").expect("extracted");
    assert_eq!(canonical, "sin(2*x)");
    assert_eq!(lang, "trig");
  }

  #[test]
  fn extract_math_canonical_accepts_bare_expression_no_suffix() {
    // Fallback: when caller has already classified intent as math
    // (or operator just typed the expression alone), accept the
    // bare expression.
    let (canonical, _) = extract_math_canonical_form("x^2 + 2*x*y + y^2").expect("extracted");
    assert_eq!(canonical, "x^2 + 2*x*y + y^2");
  }

  #[test]
  fn extract_math_canonical_returns_none_for_pure_prose() {
    assert!(extract_math_canonical_form("이게 뭐야?").is_none());
    assert!(extract_math_canonical_form("hello world").is_none());
    assert!(extract_math_canonical_form("rename foo to bar in src/a.py").is_none());
  }

  #[test]
  fn extract_math_canonical_strips_trailing_punctuation() {
    let (canonical, _) = extract_math_canonical_form("x + y 는 뭐야!").expect("extracted");
    assert_eq!(canonical, "x + y");
  }

  // ─── math resolver: korean NL → ResolutionVerdict ────────────

  #[test]
  fn resolve_math_lookup_holds_with_canonical_and_language() {
    let v = resolve_parameters(&ResolutionInput {
      operation_candidate: "lookup-algebraic-equivalent".to_string(),
      utterance: "x^2 + 2*x*y + y^2 는 뭐야?".to_string(),
      ..Default::default()
    });
    match v {
      ResolutionVerdict::ResolutionHeld {
        transform,
        held_kind,
        partial_resolution,
        missing_slots,
        ..
      } => {
        assert_eq!(transform, "lookup-algebraic-equivalent");
        assert_eq!(held_kind, ResolutionHeldKind::MissingAlgebraicEquivalent);
        assert_eq!(
          partial_resolution.get("canonical_form").unwrap(),
          "x^2 + 2*x*y + y^2"
        );
        assert_eq!(partial_resolution.get("language").unwrap(), "polynomial");
        assert!(missing_slots.contains(&"equivalent_form".to_string()));
      }
      other => panic!("expected Held(MissingAlgebraicEquivalent), got {other:?}"),
    }
  }

  // ─── chemistry lift: korean NL → reactants ───────────────────

  #[test]
  fn extract_chemistry_canonical_from_korean_question() {
    let (reactants, conditions, lang) =
      extract_chemistry_canonical_form("2 H2 + O2 가 어떻게 반응해?").expect("extracted");
    assert_eq!(reactants, "2 H2 + O2");
    assert!(conditions.is_none());
    assert_eq!(lang, "inorganic");
  }

  #[test]
  fn extract_chemistry_canonical_with_conditions_in_parens() {
    let (reactants, conditions, _) =
      extract_chemistry_canonical_form("2 H2 + O2 (spark, 25C) 는 어떻게 반응?")
        .expect("extracted");
    assert_eq!(reactants, "2 H2 + O2");
    assert_eq!(conditions.as_deref(), Some("spark, 25C"));
  }

  #[test]
  fn extract_chemistry_canonical_returns_none_for_pure_prose() {
    // Prose like "이게 뭐야" doesn't contain chemistry formula tokens.
    assert!(extract_chemistry_canonical_form("이게 뭐야?").is_none());
    assert!(extract_chemistry_canonical_form("hello").is_none());
  }

  #[test]
  fn resolve_chemistry_lookup_holds_with_reactants_and_language() {
    let v = resolve_parameters(&ResolutionInput {
      operation_candidate: "lookup-chemical-reaction".to_string(),
      utterance: "2 H2 + O2 가 어떻게 반응해?".to_string(),
      ..Default::default()
    });
    match v {
      ResolutionVerdict::ResolutionHeld {
        transform,
        held_kind,
        partial_resolution,
        missing_slots,
        ..
      } => {
        assert_eq!(transform, "lookup-chemical-reaction");
        assert_eq!(held_kind, ResolutionHeldKind::MissingChemistryProducts);
        assert_eq!(partial_resolution.get("reactants").unwrap(), "2 H2 + O2");
        assert_eq!(partial_resolution.get("language").unwrap(), "inorganic");
        assert!(missing_slots.contains(&"products".to_string()));
      }
      other => panic!("expected Held(MissingChemistryProducts), got {other:?}"),
    }
  }

  #[test]
  fn resolve_chemistry_lookup_rejects_when_no_reactants() {
    let v = resolve_parameters(&ResolutionInput {
      operation_candidate: "lookup-chemical-reaction".to_string(),
      utterance: "도와줘".to_string(),
      ..Default::default()
    });
    assert!(matches!(
      v,
      ResolutionVerdict::ResolutionRejected {
        held_kind: ResolutionHeldKind::InvalidIdentifier,
        ..
      }
    ));
  }

  #[test]
  fn resolve_math_lookup_rejects_when_no_expression_in_utterance() {
    let v = resolve_parameters(&ResolutionInput {
      operation_candidate: "lookup-algebraic-equivalent".to_string(),
      utterance: "도와줘".to_string(),
      ..Default::default()
    });
    assert!(matches!(
      v,
      ResolutionVerdict::ResolutionRejected {
        held_kind: ResolutionHeldKind::InvalidIdentifier,
        ..
      }
    ));
  }
}
