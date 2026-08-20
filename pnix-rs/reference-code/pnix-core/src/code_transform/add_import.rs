//! Add-import deterministic code-transform host carrier.
//!
//! OWNER-LAW (2026-05-12): the fourth deterministic code-transform —
//! the natural complement to `remove-unused-import`. Mirrors
//! `stdlib/lib/gate/code-transform/add-import.px`.
//!
//! Adds an import / use / require statement to a target file. Like
//! `add-test-stub`, this carrier owns the **verdict ladder + canonical
//! artifact builders only** — the host (tree-sitter + CST emitter)
//! performs the actual insertion at the canonical location under
//! `ToolActionApproval`.
//!
//! Compose with other transforms: `add-import` is typically chained
//! upstream of `rename-symbol` or `extract-function` when introducing
//! a new symbol that needs a corresponding `use` / `import` line.

use serde::{Deserialize, Serialize};
use pnix_hash::{Digest, Sha256};

/// How the host should react if the import is already present in the
/// target file. Mirrors the `.px` `if_already_present` field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AddImportIfAlreadyPresent {
  /// Default: no-op (`add-import-skip` verdict).
  Skip,
  /// Insert the import again (intentional duplicate — e.g. for a
  /// merge pre-staging step).
  Duplicate,
  /// Block: `add-import-held` until the operator decides.
  Held,
}

impl AddImportIfAlreadyPresent {
  pub const ALL: &'static [Self] = &[Self::Skip, Self::Duplicate, Self::Held];

  pub fn as_str(self) -> &'static str {
    match self {
      Self::Skip => "skip",
      Self::Duplicate => "duplicate",
      Self::Held => "held",
    }
  }
}

/// Held / Rejected outcome kinds from [`classify_add_import`]. Mirrors
/// the `.px` `held_kind` ladder 1:1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AddImportHeldKind {
  MissingTargetPath,
  TargetPathOutOfProject,
  MissingImportSpec,
  LanguageNotSupported,
  ImportSpecMalformed,
  IfAlreadyPresentInvalid,
}

impl AddImportHeldKind {
  pub const ALL: &'static [Self] = &[
    Self::MissingTargetPath,
    Self::TargetPathOutOfProject,
    Self::MissingImportSpec,
    Self::LanguageNotSupported,
    Self::ImportSpecMalformed,
    Self::IfAlreadyPresentInvalid,
  ];
  pub fn as_str(self) -> &'static str {
    match self {
      Self::MissingTargetPath => "missing-target-path",
      Self::TargetPathOutOfProject => "target-path-out-of-project",
      Self::MissingImportSpec => "missing-import-spec",
      Self::LanguageNotSupported => "language-not-supported",
      Self::ImportSpecMalformed => "import-spec-malformed",
      Self::IfAlreadyPresentInvalid => "if-already-present-invalid",
    }
  }
}

pub const SUPPORTED_LANGUAGES: &[&str] = &["rust", "python", "typescript", "javascript", "go"];

/// An add-import request. Mirrors the `.px` input shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddImportRequest {
  pub target_path: String,
  pub language: String,
  /// Language-specific spec. Examples:
  ///   - Rust: `"std::collections::HashMap"` or
  ///     `"std::collections::{HashMap, BTreeMap}"`
  ///   - Python: `"import json"` or `"from collections import OrderedDict"`
  ///   - TypeScript / JavaScript: `"import { foo } from \"bar\""`
  ///   - Go: `"\"net/http\""` (with quotes; package form)
  pub import_spec: String,
  /// Defaults to `Skip` when `None`.
  pub if_already_present: Option<AddImportIfAlreadyPresent>,
}

/// Verdict from [`classify_add_import`]. Four outcomes mirror the
/// `.px` law — distinct from the three-outcome `remove-unused-import`
/// because add-import has the additional `Skip` (already-present)
/// terminal state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "verdict")]
pub enum AddImportVerdict {
  AddImportReady,
  AddImportSkip,
  AddImportHeld {
    held_kind: AddImportHeldKind,
    reason: String,
  },
  AddImportRejected {
    held_kind: AddImportHeldKind,
    reason: String,
  },
}

fn is_supported_language(lang: &str) -> bool {
  matches!(lang, "rust" | "python" | "typescript" | "javascript" | "go")
}

fn is_path_in_project(p: &str) -> bool {
  !p.is_empty() && !p.contains("..") && !p.contains('\u{0}')
}

/// Per-language well-formedness check. Mirrors the `.px`
/// `importSpecLooksValid` function.
fn import_spec_looks_valid(language: &str, spec: &str) -> bool {
  if spec.is_empty() {
    return false;
  }
  match language {
    "rust" => looks_valid_rust_use_spec(spec),
    "python" => looks_valid_python_import_spec(spec),
    "typescript" | "javascript" => looks_valid_ts_import_spec(spec),
    "go" => looks_valid_go_import_spec(spec),
    _ => false,
  }
}

fn looks_valid_rust_use_spec(spec: &str) -> bool {
  // `foo` / `foo::bar` / `foo::bar::*` / `foo::{a, b}`.
  // Must start with an identifier char or `_`; allow `:`, `_`,
  // `0-9`, `a-zA-Z` thereafter; optionally end in `*` or
  // `{<group>}`.
  let bytes = spec.as_bytes();
  if bytes.is_empty() {
    return false;
  }
  let first = bytes[0];
  if !(first.is_ascii_alphabetic() || first == b'_') {
    return false;
  }
  // Walk the body until we hit a closing form or a separator.
  let mut i = 1usize;
  while i < bytes.len() {
    let b = bytes[i];
    if b.is_ascii_alphanumeric() || b == b'_' || b == b':' {
      i += 1;
    } else {
      break;
    }
  }
  if i == bytes.len() {
    return true;
  }
  // Trailing `*` or `{...}`.
  if bytes[i] == b'*' && i + 1 == bytes.len() {
    return true;
  }
  if bytes[i] == b'{' {
    if let Some(end) = spec[i + 1..].find('}') {
      let group = &spec[i + 1..i + 1 + end];
      if i + 1 + end + 1 == spec.len() && !group.is_empty() {
        return true;
      }
    }
  }
  false
}

fn looks_valid_python_import_spec(spec: &str) -> bool {
  // `import X` or `from X import Y` (with optional `as` aliases).
  let s = spec.trim();
  if let Some(rest) = s.strip_prefix("import ") {
    let rest = rest.trim();
    if rest.is_empty() {
      return false;
    }
    // Allow `a.b.c` / `a, b` / `a as x` — at minimum, every comma-
    // separated entry must start with an ident char.
    return rest.split(',').all(|part| {
      let p = part.trim();
      let first_word = p.split_whitespace().next().unwrap_or("");
      !first_word.is_empty()
        && first_word
          .chars()
          .next()
          .map(|c| c.is_ascii_alphabetic() || c == '_')
          .unwrap_or(false)
    });
  }
  if let Some(rest) = s.strip_prefix("from ") {
    if let Some(idx) = rest.find(" import ") {
      let pkg = rest[..idx].trim();
      let names = rest[idx + " import ".len()..].trim();
      if pkg.is_empty() || names.is_empty() {
        return false;
      }
      // Pkg must start with an ident char.
      let pkg_first = pkg.chars().next().unwrap_or('?');
      if !(pkg_first.is_ascii_alphabetic() || pkg_first == '_') {
        return false;
      }
      return true;
    }
  }
  false
}

fn looks_valid_ts_import_spec(spec: &str) -> bool {
  // Must start with `import ` and either contain ` from ` + a
  // quoted spec, OR be a bare `import '...'` side-effect import.
  let s = spec.trim();
  if !s.starts_with("import ") {
    return false;
  }
  if s.contains(" from ") {
    return true;
  }
  // Side-effect import: `import "side";` / `import 'side';`.
  let after = s.strip_prefix("import ").unwrap_or("");
  let after = after.trim_end_matches(';').trim_end();
  (after.starts_with('"') && after.ends_with('"'))
    || (after.starts_with('\'') && after.ends_with('\''))
}

fn looks_valid_go_import_spec(spec: &str) -> bool {
  // Must contain at least one quoted package path. Examples:
  //   `"net/http"`           — bare
  //   `myalias "net/http"`   — aliased
  //   `_ "side/effect"`      — blank
  //   `. "side/effect"`      — dot
  let s = spec.trim();
  let q1 = s.find('"');
  let q1 = match q1 {
    Some(i) => i,
    None => return false,
  };
  let q2 = s[q1 + 1..].find('"');
  let q2 = match q2 {
    Some(i) => q1 + 1 + i,
    None => return false,
  };
  q2 > q1 + 1 // non-empty quoted body
}

/// Pure classifier — Rust mirror of the `.px` owner law's `classify`.
///
/// OWNER-LAW (2026-05-12): MUST stay in lockstep with
/// `stdlib/lib/gate/code-transform/add-import.px`. The sync guard at
/// `scripts/check-code-transform-owner-carrier-sync.sh` enforces
/// held_kind and supported-language parity.
///
/// Ladder order matches the `.px`:
///   1. target_path empty → Held
///   2. target_path out-of-project → Held
///   3. import_spec empty → Held
///   4. language not supported → Held
///   5. if_already_present invalid → **Rejected** (structural rule)
///   6. import_spec malformed for language → Held
///
/// Anything else → Ready. (Phase-1 carrier does NOT inspect the
/// target file's existing content; `add-import-skip` is reserved
/// for a future phase that adds "is the import already present?"
/// detection — currently always Ready when preconditions pass.)
pub fn classify_add_import(req: &AddImportRequest) -> AddImportVerdict {
  if req.target_path.is_empty() {
    return AddImportVerdict::AddImportHeld {
      held_kind: AddImportHeldKind::MissingTargetPath,
      reason: "add-import requires a non-empty `target_path`".to_string(),
    };
  }
  if !is_path_in_project(&req.target_path) {
    return AddImportVerdict::AddImportHeld {
      held_kind: AddImportHeldKind::TargetPathOutOfProject,
      reason: "target_path must be within the project root and must not contain `..`".to_string(),
    };
  }
  if req.import_spec.is_empty() {
    return AddImportVerdict::AddImportHeld {
      held_kind: AddImportHeldKind::MissingImportSpec,
      reason: "add-import requires a non-empty `import_spec`".to_string(),
    };
  }
  if !is_supported_language(&req.language) {
    return AddImportVerdict::AddImportHeld {
      held_kind: AddImportHeldKind::LanguageNotSupported,
      reason: format!(
        "add-import owner currently supports rust|python|typescript|javascript|go; got `{}`",
        req.language
      ),
    };
  }
  // `if_already_present` is a closed enum from the Rust side — a
  // bad value would have failed to deserialize. The .px law's check
  // catches dynamic-input misuse; here the rejection branch is for
  // future flexibility if the field is widened to accept strings.
  // For phase 1 the typed enum is always valid by construction.
  if !import_spec_looks_valid(&req.language, &req.import_spec) {
    return AddImportVerdict::AddImportHeld {
      held_kind: AddImportHeldKind::ImportSpecMalformed,
      reason: format!(
        "import_spec did not match minimal well-formedness for `{}`",
        req.language
      ),
    };
  }
  AddImportVerdict::AddImportReady
}

/// Result of [`compute_add_import_candidate`]: request + verdict +
/// resolved `if_already_present` (defaults applied).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddImportCandidate {
  pub request: AddImportRequest,
  pub verdict: AddImportVerdict,
  /// What the carrier resolved for `if_already_present` (Skip when
  /// the request left it unset).
  pub resolved_if_already_present: AddImportIfAlreadyPresent,
}

/// Orchestrator: classify the request and package it as a candidate.
///
/// OWNER-LAW (2026-05-12): pure function. No I/O. Like add-test-stub,
/// this carrier does NOT emit a unified diff — the host's CST
/// emitter performs the insertion at the canonical location under
/// `ToolActionApproval`.
pub fn compute_add_import_candidate(request: &AddImportRequest) -> AddImportCandidate {
  let verdict = classify_add_import(request);
  let resolved = request
    .if_already_present
    .unwrap_or(AddImportIfAlreadyPresent::Skip);
  AddImportCandidate {
    request: request.clone(),
    verdict,
    resolved_if_already_present: resolved,
  }
}

/// Render an `AddImportCandidate` as the canonical JSON payload.
pub fn build_add_import_candidate_payload(candidate: &AddImportCandidate) -> serde_json::Value {
  let request = &candidate.request;
  let (verdict_str, next_step) = match &candidate.verdict {
    AddImportVerdict::AddImportReady => (
      "add-import-ready",
      "host-cst-insert-at-canonical-location-then-tool-action-approval",
    ),
    AddImportVerdict::AddImportSkip => ("add-import-skip", "no-op-import-already-present"),
    AddImportVerdict::AddImportHeld { .. } => ("add-import-held", "operator-decision-or-resubmit"),
    AddImportVerdict::AddImportRejected { .. } => {
      ("add-import-rejected", "operator-decision-or-resubmit")
    }
  };
  let mut payload = serde_json::json!({
    "transform": "add-import",
    "owner_law": "stdlib/lib/gate/code-transform/add-import.px",
    "target_path": request.target_path,
    "language": request.language,
    "import_spec": request.import_spec,
    "if_already_present": candidate.resolved_if_already_present.as_str(),
    "verdict": verdict_str,
    "candidate_only": true,
    "next_step": next_step,
  });
  match &candidate.verdict {
    AddImportVerdict::AddImportHeld { held_kind, reason }
    | AddImportVerdict::AddImportRejected { held_kind, reason } => {
      payload["held_kind"] = serde_json::Value::String(held_kind.as_str().to_string());
      payload["reason"] = serde_json::Value::String(reason.clone());
    }
    _ => {}
  }
  payload
}

/// Wrap an `AddImportCandidate` into a full
/// `coding.code-transform.add-import-{ready,held,rejected,skip}`
/// artifact value with a replay-stable id.
///
/// OWNER-LAW (2026-05-12): id hash binds intrinsic request identity:
/// target_path + language + import_spec + if_already_present +
/// verdict suffix.
pub fn build_add_import_candidate_artifact(
  candidate: &AddImportCandidate,
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
) -> serde_json::Value {
  let payload = build_add_import_candidate_payload(candidate);
  let suffix = match &candidate.verdict {
    AddImportVerdict::AddImportReady => "ready",
    AddImportVerdict::AddImportSkip => "skip",
    AddImportVerdict::AddImportHeld { .. } => "held",
    AddImportVerdict::AddImportRejected { .. } => "rejected",
  };
  let artifact_family = format!("coding.code-transform.add-import-{suffix}");

  let mut hasher = Sha256::new();
  hasher.update(b"add-import-candidate\x1f");
  hasher.update(candidate.request.target_path.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(candidate.request.language.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(candidate.request.import_spec.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(candidate.resolved_if_already_present.as_str().as_bytes());
  hasher.update(b"\x1f");
  hasher.update(suffix.as_bytes());
  let digest = hasher.finalize();
  let prefix = digest
    .iter()
    .take(8)
    .map(|b| format!("{b:02x}"))
    .collect::<String>();
  let id = format!("add-import.{prefix}");

  let mut artifact = serde_json::json!({
    "id": id,
    "artifact_family": artifact_family,
    "source_surface": "code-transform.add-import",
    "stored_at_ms": stored_at_ms,
    "target_paths": [candidate.request.target_path.clone()],
    "command_refs": serde_json::Value::Array(Vec::new()),
    "related_refs": serde_json::json!([
      "owner-law:stdlib/lib/gate/code-transform/add-import.px"
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

  fn req(target: &str, language: &str, spec: &str) -> AddImportRequest {
    AddImportRequest {
      target_path: target.to_string(),
      language: language.to_string(),
      import_spec: spec.to_string(),
      if_already_present: None,
    }
  }

  // ─── classifier ──────────────────────────────────────────────────

  #[test]
  fn classify_ready_for_well_formed_rust_use_spec() {
    let r = req("src/a.rs", "rust", "std::collections::HashMap");
    assert!(matches!(
      classify_add_import(&r),
      AddImportVerdict::AddImportReady
    ));
  }

  #[test]
  fn classify_ready_for_rust_grouped_use() {
    let r = req("src/a.rs", "rust", "std::collections::{HashMap, BTreeMap}");
    assert!(matches!(
      classify_add_import(&r),
      AddImportVerdict::AddImportReady
    ));
  }

  #[test]
  fn classify_ready_for_rust_glob_use() {
    let r = req("src/a.rs", "rust", "std::collections::*");
    assert!(matches!(
      classify_add_import(&r),
      AddImportVerdict::AddImportReady
    ));
  }

  #[test]
  fn classify_ready_for_python_import_form() {
    let r = req("src/a.py", "python", "import json");
    assert!(matches!(
      classify_add_import(&r),
      AddImportVerdict::AddImportReady
    ));
  }

  #[test]
  fn classify_ready_for_python_from_import_form() {
    let r = req("src/a.py", "python", "from collections import OrderedDict");
    assert!(matches!(
      classify_add_import(&r),
      AddImportVerdict::AddImportReady
    ));
  }

  #[test]
  fn classify_ready_for_python_aliased_form() {
    let r = req("src/a.py", "python", "import numpy as np");
    assert!(matches!(
      classify_add_import(&r),
      AddImportVerdict::AddImportReady
    ));
  }

  #[test]
  fn classify_ready_for_typescript_named_form() {
    let r = req("src/a.ts", "typescript", "import { foo } from \"bar\"");
    assert!(matches!(
      classify_add_import(&r),
      AddImportVerdict::AddImportReady
    ));
  }

  #[test]
  fn classify_ready_for_typescript_side_effect_import() {
    let r = req("src/a.ts", "typescript", "import './side-effect.js'");
    assert!(matches!(
      classify_add_import(&r),
      AddImportVerdict::AddImportReady
    ));
  }

  #[test]
  fn classify_ready_for_go_quoted_pkg() {
    let r = req("src/a.go", "go", "\"net/http\"");
    assert!(matches!(
      classify_add_import(&r),
      AddImportVerdict::AddImportReady
    ));
  }

  #[test]
  fn classify_ready_for_go_aliased_pkg() {
    let r = req("src/a.go", "go", "myalias \"net/http\"");
    assert!(matches!(
      classify_add_import(&r),
      AddImportVerdict::AddImportReady
    ));
  }

  #[test]
  fn classify_holds_on_missing_target_path() {
    let r = req("", "rust", "std::io::Read");
    match classify_add_import(&r) {
      AddImportVerdict::AddImportHeld { held_kind, .. } => {
        assert_eq!(held_kind, AddImportHeldKind::MissingTargetPath);
      }
      other => panic!("expected MissingTargetPath, got {:?}", other),
    }
  }

  #[test]
  fn classify_holds_on_parent_traversal_in_path() {
    let r = req("../escape.rs", "rust", "std::io::Read");
    match classify_add_import(&r) {
      AddImportVerdict::AddImportHeld { held_kind, .. } => {
        assert_eq!(held_kind, AddImportHeldKind::TargetPathOutOfProject);
      }
      other => panic!("expected TargetPathOutOfProject, got {:?}", other),
    }
  }

  #[test]
  fn classify_holds_on_missing_spec() {
    let r = req("src/a.rs", "rust", "");
    match classify_add_import(&r) {
      AddImportVerdict::AddImportHeld { held_kind, .. } => {
        assert_eq!(held_kind, AddImportHeldKind::MissingImportSpec);
      }
      other => panic!("expected MissingImportSpec, got {:?}", other),
    }
  }

  #[test]
  fn classify_holds_on_unsupported_language() {
    let r = req("src/a.f90", "fortran", "use foo");
    match classify_add_import(&r) {
      AddImportVerdict::AddImportHeld { held_kind, reason } => {
        assert_eq!(held_kind, AddImportHeldKind::LanguageNotSupported);
        assert!(reason.contains("fortran"));
      }
      other => panic!("expected LanguageNotSupported, got {:?}", other),
    }
  }

  #[test]
  fn classify_holds_on_malformed_rust_spec() {
    // Leading digit, leading whitespace, embedded special chars.
    for bad in ["1abc", "  std::io", "use std::io::Read;"] {
      let r = req("src/a.rs", "rust", bad);
      match classify_add_import(&r) {
        AddImportVerdict::AddImportHeld { held_kind, .. } => {
          assert_eq!(
            held_kind,
            AddImportHeldKind::ImportSpecMalformed,
            "for spec '{bad}'"
          );
        }
        other => panic!("expected ImportSpecMalformed for '{bad}', got {:?}", other),
      }
    }
  }

  #[test]
  fn classify_holds_on_malformed_python_spec() {
    // Missing leading `import ` / `from ` keyword.
    for bad in ["json", "collections.OrderedDict"] {
      let r = req("src/a.py", "python", bad);
      match classify_add_import(&r) {
        AddImportVerdict::AddImportHeld { held_kind, .. } => {
          assert_eq!(held_kind, AddImportHeldKind::ImportSpecMalformed);
        }
        other => panic!("expected ImportSpecMalformed for '{bad}', got {:?}", other),
      }
    }
  }

  #[test]
  fn classify_holds_on_malformed_ts_spec() {
    // No `import` keyword at start.
    let r = req("src/a.ts", "typescript", "const x = 1");
    match classify_add_import(&r) {
      AddImportVerdict::AddImportHeld { held_kind, .. } => {
        assert_eq!(held_kind, AddImportHeldKind::ImportSpecMalformed);
      }
      other => panic!("expected ImportSpecMalformed, got {:?}", other),
    }
  }

  #[test]
  fn classify_holds_on_malformed_go_spec() {
    // No quoted package path.
    let r = req("src/a.go", "go", "net/http");
    match classify_add_import(&r) {
      AddImportVerdict::AddImportHeld { held_kind, .. } => {
        assert_eq!(held_kind, AddImportHeldKind::ImportSpecMalformed);
      }
      other => panic!("expected ImportSpecMalformed, got {:?}", other),
    }
  }

  #[test]
  fn classify_ladder_target_missing_wins_over_spec_malformed() {
    let r = req("", "rust", "BAD!!!SPEC");
    match classify_add_import(&r) {
      AddImportVerdict::AddImportHeld { held_kind, .. } => {
        assert_eq!(held_kind, AddImportHeldKind::MissingTargetPath);
      }
      other => panic!("expected MissingTargetPath, got {:?}", other),
    }
  }

  // ─── candidate + payload + artifact ──────────────────────────────

  #[test]
  fn candidate_resolves_if_already_present_default_to_skip() {
    let r = req("src/a.rs", "rust", "std::io::Read");
    let cand = compute_add_import_candidate(&r);
    assert_eq!(
      cand.resolved_if_already_present,
      AddImportIfAlreadyPresent::Skip
    );
  }

  #[test]
  fn candidate_respects_caller_supplied_if_already_present() {
    let mut r = req("src/a.rs", "rust", "std::io::Read");
    r.if_already_present = Some(AddImportIfAlreadyPresent::Held);
    let cand = compute_add_import_candidate(&r);
    assert_eq!(
      cand.resolved_if_already_present,
      AddImportIfAlreadyPresent::Held
    );
  }

  #[test]
  fn payload_canonical_fields_for_ready() {
    let r = req("src/a.rs", "rust", "std::collections::HashMap");
    let cand = compute_add_import_candidate(&r);
    let payload = build_add_import_candidate_payload(&cand);
    assert_eq!(payload["transform"].as_str(), Some("add-import"));
    assert_eq!(
      payload["owner_law"].as_str(),
      Some("stdlib/lib/gate/code-transform/add-import.px")
    );
    assert_eq!(payload["target_path"].as_str(), Some("src/a.rs"));
    assert_eq!(payload["language"].as_str(), Some("rust"));
    assert_eq!(
      payload["import_spec"].as_str(),
      Some("std::collections::HashMap")
    );
    assert_eq!(payload["if_already_present"].as_str(), Some("skip"));
    assert_eq!(payload["verdict"].as_str(), Some("add-import-ready"));
    assert_eq!(payload["candidate_only"].as_bool(), Some(true));
    assert_eq!(
      payload["next_step"].as_str(),
      Some("host-cst-insert-at-canonical-location-then-tool-action-approval")
    );
    assert!(payload.get("held_kind").is_none());
  }

  #[test]
  fn payload_carries_held_kind_and_reason_for_held() {
    let r = req("", "rust", "std::io::Read");
    let cand = compute_add_import_candidate(&r);
    let payload = build_add_import_candidate_payload(&cand);
    assert_eq!(payload["verdict"].as_str(), Some("add-import-held"));
    assert_eq!(payload["held_kind"].as_str(), Some("missing-target-path"));
    assert!(payload["reason"].as_str().unwrap().contains("target_path"));
  }

  #[test]
  fn artifact_envelope_shape_ready() {
    let r = req("src/a.rs", "rust", "std::io::Read");
    let cand = compute_add_import_candidate(&r);
    let art = build_add_import_candidate_artifact(&cand, 1700000000000, None);
    assert_eq!(
      art["artifact_family"].as_str(),
      Some("coding.code-transform.add-import-ready")
    );
    assert_eq!(
      art["source_surface"].as_str(),
      Some("code-transform.add-import")
    );
    assert_eq!(art["stored_at_ms"].as_u64(), Some(1700000000000));
    let id = art["id"].as_str().expect("id");
    assert!(id.starts_with("add-import."));
    assert_eq!(art["target_paths"][0].as_str(), Some("src/a.rs"));
    let related = art["related_refs"].as_array().unwrap();
    assert!(related.iter().any(|v| v
      .as_str()
      .map(|s| s == "owner-law:stdlib/lib/gate/code-transform/add-import.px")
      .unwrap_or(false)));
  }

  #[test]
  fn artifact_family_changes_with_verdict() {
    let ready = compute_add_import_candidate(&req("src/a.rs", "rust", "std::io::Read"));
    let held = compute_add_import_candidate(&req("", "rust", "std::io::Read"));
    let ready_art = build_add_import_candidate_artifact(&ready, 0, None);
    let held_art = build_add_import_candidate_artifact(&held, 0, None);
    assert_eq!(
      ready_art["artifact_family"].as_str(),
      Some("coding.code-transform.add-import-ready")
    );
    assert_eq!(
      held_art["artifact_family"].as_str(),
      Some("coding.code-transform.add-import-held")
    );
  }

  #[test]
  fn artifact_id_replay_stable_across_stored_at_ms() {
    let cand = compute_add_import_candidate(&req("src/a.rs", "rust", "std::io::Read"));
    let a = build_add_import_candidate_artifact(&cand, 1000, None);
    let b = build_add_import_candidate_artifact(&cand, 9999, None);
    assert_eq!(a["id"], b["id"]);
    assert_ne!(a["stored_at_ms"], b["stored_at_ms"]);
  }

  #[test]
  fn artifact_id_differs_per_target_or_language_or_spec() {
    let base = compute_add_import_candidate(&req("src/a.rs", "rust", "std::io::Read"));
    let diff_target = compute_add_import_candidate(&req("src/b.rs", "rust", "std::io::Read"));
    let diff_language = compute_add_import_candidate(&req("src/a.py", "python", "import json"));
    let diff_spec = compute_add_import_candidate(&req("src/a.rs", "rust", "std::io::Write"));
    let id_base = build_add_import_candidate_artifact(&base, 0, None)["id"].clone();
    let id_t = build_add_import_candidate_artifact(&diff_target, 0, None)["id"].clone();
    let id_l = build_add_import_candidate_artifact(&diff_language, 0, None)["id"].clone();
    let id_s = build_add_import_candidate_artifact(&diff_spec, 0, None)["id"].clone();
    assert_ne!(id_base, id_t);
    assert_ne!(id_base, id_l);
    assert_ne!(id_base, id_s);
  }

  #[test]
  fn artifact_id_differs_per_if_already_present_choice() {
    let mut a = req("src/a.rs", "rust", "std::io::Read");
    a.if_already_present = Some(AddImportIfAlreadyPresent::Skip);
    let mut b = req("src/a.rs", "rust", "std::io::Read");
    b.if_already_present = Some(AddImportIfAlreadyPresent::Duplicate);
    let cand_a = compute_add_import_candidate(&a);
    let cand_b = compute_add_import_candidate(&b);
    let id_a = build_add_import_candidate_artifact(&cand_a, 0, None)["id"].clone();
    let id_b = build_add_import_candidate_artifact(&cand_b, 0, None)["id"].clone();
    assert_ne!(id_a, id_b);
  }

  #[test]
  fn artifact_carries_repo_snapshot_ref_when_provided() {
    let cand = compute_add_import_candidate(&req("src/a.rs", "rust", "std::io::Read"));
    let art = build_add_import_candidate_artifact(&cand, 0, Some("commit-xyz"));
    assert_eq!(art["repo_snapshot_ref"].as_str(), Some("commit-xyz"));
  }

  #[test]
  fn helper_predicates() {
    assert!(is_supported_language("rust"));
    assert!(is_supported_language("python"));
    assert!(!is_supported_language("fortran"));

    assert!(is_path_in_project("src/a.rs"));
    assert!(!is_path_in_project(""));
    assert!(!is_path_in_project("../escape.rs"));

    assert!(looks_valid_rust_use_spec("std::io::Read"));
    assert!(looks_valid_rust_use_spec("std::collections::*"));
    assert!(looks_valid_rust_use_spec("std::{io, fs}"));
    assert!(!looks_valid_rust_use_spec(""));
    assert!(!looks_valid_rust_use_spec("1abc"));

    assert!(looks_valid_python_import_spec("import json"));
    assert!(looks_valid_python_import_spec("from os import path"));
    assert!(!looks_valid_python_import_spec("json"));

    assert!(looks_valid_ts_import_spec("import { Foo } from \"bar\""));
    assert!(looks_valid_ts_import_spec("import './side'"));
    assert!(!looks_valid_ts_import_spec("const x = 1"));

    assert!(looks_valid_go_import_spec("\"net/http\""));
    assert!(looks_valid_go_import_spec("myalias \"net/http\""));
    assert!(!looks_valid_go_import_spec("net/http"));
  }
}
