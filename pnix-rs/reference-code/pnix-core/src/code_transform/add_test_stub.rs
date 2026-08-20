//! Add-test-stub deterministic code-transform host carrier.
//!
//! OWNER-LAW (2026-05-11): the third deterministic code-transform —
//! proves the canonical chain pattern generalizes beyond rename-symbol
//! and remove-unused-import. Mirrors
//! `stdlib/lib/gate/code-transform/add-test-stub.px`.
//!
//! Lowest-risk transform of the three: never modifies production
//! source. Only adds a typed test stub at the canonical test surface
//! for the target language (`tests/`, `*_test.rs`, `*.spec.ts`, etc.).
//! Therefore maps to `CodeEditCapability::EditTestOnly` (relaxed
//! approval) per `freecat-cli/docs/security-model.md` §3.1.
//!
//! Like remove-unused-import, this first slice owns the **verdict
//! ladder + canonical artifact builders only**. The host (with CST
//! emitter / language-specific test-surface conventions) is
//! responsible for the actual stub emission under
//! `ToolActionApproval`.

use serde::{Deserialize, Serialize};
use pnix_hash::{Digest, Sha256};

/// Test-stub placement strategy. Mirrors the `.px` `validPlaces` set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AddTestStubPlace {
  /// Sibling file (`foo.ts` → `foo.spec.ts`; `pkg.go` → `pkg_test.go`).
  /// Default for TypeScript / JavaScript / Go.
  Sibling,
  /// External `tests/` directory. Default for Python.
  TestsDir,
  /// Inline `#[cfg(test)] mod tests { ... }` block. Default for Rust.
  InlineCfgTest,
}

impl AddTestStubPlace {
  pub const ALL: &'static [Self] = &[Self::Sibling, Self::TestsDir, Self::InlineCfgTest];

  pub fn as_str(self) -> &'static str {
    match self {
      Self::Sibling => "sibling",
      Self::TestsDir => "tests-dir",
      Self::InlineCfgTest => "inline-cfg-test",
    }
  }

  /// Default placement strategy for a given language, mirroring the
  /// `.px` `defaultPlaceFor` table.
  pub fn default_for(language: &str) -> Option<Self> {
    match language {
      "rust" => Some(Self::InlineCfgTest),
      "python" => Some(Self::TestsDir),
      "typescript" | "javascript" => Some(Self::Sibling),
      "go" => Some(Self::Sibling),
      _ => None,
    }
  }
}

/// Held / Rejected outcome kinds from
/// [`classify_add_test_stub`]. Each variant maps 1:1 to a kebab-case
/// string mirroring the `.px` `held_kind` ladder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AddTestStubHeldKind {
  MissingTargetModule,
  TargetModuleOutOfProject,
  MissingTestName,
  InvalidTestName,
  LanguageNotSupported,
  PlaceNotSupported,
}

impl AddTestStubHeldKind {
  pub const ALL: &'static [Self] = &[
    Self::MissingTargetModule,
    Self::TargetModuleOutOfProject,
    Self::MissingTestName,
    Self::InvalidTestName,
    Self::LanguageNotSupported,
    Self::PlaceNotSupported,
  ];
  pub fn as_str(self) -> &'static str {
    match self {
      Self::MissingTargetModule => "missing-target-module",
      Self::TargetModuleOutOfProject => "target-module-out-of-project",
      Self::MissingTestName => "missing-test-name",
      Self::InvalidTestName => "invalid-test-name",
      Self::LanguageNotSupported => "language-not-supported",
      Self::PlaceNotSupported => "place-not-supported",
    }
  }
}

pub const SUPPORTED_LANGUAGES: &[&str] = &["rust", "python", "typescript", "javascript", "go"];

/// An add-test-stub request — the input the `.px` owner law's
/// `classify` function inspects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddTestStubRequest {
  pub target_module: String,
  pub test_name: String,
  pub language: String,
  /// Free-form description used as a doc comment in the emitted stub.
  /// Not constrained by the classifier — the host's CST emitter uses
  /// this verbatim.
  pub intent: String,
  /// Optional placement override. When `None` the classifier picks
  /// the language default via [`AddTestStubPlace::default_for`].
  pub place: Option<AddTestStubPlace>,
}

/// Verdict from [`classify_add_test_stub`], mirroring the `.px`
/// owner law's three outcomes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "verdict")]
pub enum AddTestStubVerdict {
  AddTestStubReady {
    resolved_place: AddTestStubPlace,
  },
  AddTestStubHeld {
    held_kind: AddTestStubHeldKind,
    reason: String,
  },
  AddTestStubRejected {
    held_kind: AddTestStubHeldKind,
    reason: String,
  },
}

fn is_supported_language(lang: &str) -> bool {
  matches!(lang, "rust" | "python" | "typescript" | "javascript" | "go")
}

fn is_path_in_project(p: &str) -> bool {
  !p.is_empty() && !p.contains("..") && !p.contains('\u{0}')
}

/// ASCII identifier check: `[a-zA-Z_][a-zA-Z0-9_]*`. Mirrors the
/// `.px` `isValidIdentifier` predicate.
fn is_valid_identifier(name: &str) -> bool {
  let mut chars = name.chars();
  match chars.next() {
    None => return false,
    Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
    _ => return false,
  }
  chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Pure classifier — Rust mirror of the `.px` owner law's
/// `classify`. Returns the same verdict the law would emit for the
/// same request.
///
/// OWNER-LAW (2026-05-11): MUST stay in lockstep with
/// `stdlib/lib/gate/code-transform/add-test-stub.px`. The sync guard
/// at `scripts/check-code-transform-owner-carrier-sync.sh` will be
/// extended to cover this transform in a follow-up.
///
/// Ladder order matches the `.px`:
///   1. target_module empty → Held(MissingTargetModule)
///   2. target_module out-of-project (`..`, empty, embedded null) →
///      Held(TargetModuleOutOfProject)
///   3. test_name empty → Held(MissingTestName)
///   4. test_name not a valid ASCII identifier → **Rejected**
///      (InvalidTestName) — structural rule, restructure required
///   5. language not in supported set → Held(LanguageNotSupported)
///   6. place not in {sibling, tests-dir, inline-cfg-test} → Held
///      (PlaceNotSupported) — only reachable if caller invented a
///      new variant; the Rust enum is closed so this case is
///      structurally unreachable from typed callers
///
/// Ready carries the resolved place (defaults applied when caller
/// passed `None`).
pub fn classify_add_test_stub(req: &AddTestStubRequest) -> AddTestStubVerdict {
  if req.target_module.is_empty() {
    return AddTestStubVerdict::AddTestStubHeld {
      held_kind: AddTestStubHeldKind::MissingTargetModule,
      reason: "add-test-stub requires a non-empty `target_module`".to_string(),
    };
  }
  if !is_path_in_project(&req.target_module) {
    return AddTestStubVerdict::AddTestStubHeld {
      held_kind: AddTestStubHeldKind::TargetModuleOutOfProject,
      reason: "target_module must be within the project root and must not contain `..`".to_string(),
    };
  }
  if req.test_name.is_empty() {
    return AddTestStubVerdict::AddTestStubHeld {
      held_kind: AddTestStubHeldKind::MissingTestName,
      reason: "add-test-stub requires a non-empty `test_name`".to_string(),
    };
  }
  if !is_valid_identifier(&req.test_name) {
    return AddTestStubVerdict::AddTestStubRejected {
      held_kind: AddTestStubHeldKind::InvalidTestName,
      reason: "test_name must be a valid ASCII identifier".to_string(),
    };
  }
  if !is_supported_language(&req.language) {
    return AddTestStubVerdict::AddTestStubHeld {
      held_kind: AddTestStubHeldKind::LanguageNotSupported,
      reason: format!(
        "add-test-stub owner currently supports rust|python|typescript|javascript|go; got `{}`",
        req.language
      ),
    };
  }
  // Resolve place. Caller-supplied `place` is already a typed
  // variant; when None, fall back to the language default.
  let resolved_place = match req.place {
    Some(p) => p,
    None => match AddTestStubPlace::default_for(&req.language) {
      Some(p) => p,
      None => {
        // Defensive: should be unreachable since we already verified
        // the language is supported.
        return AddTestStubVerdict::AddTestStubHeld {
          held_kind: AddTestStubHeldKind::PlaceNotSupported,
          reason: "could not derive a default place for the requested language".to_string(),
        };
      }
    },
  };
  AddTestStubVerdict::AddTestStubReady { resolved_place }
}

/// Result of [`compute_add_test_stub_candidate`]: the request +
/// verdict, with the resolved place threaded through for
/// downstream consumers (the host's CST emitter).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddTestStubCandidate {
  pub request: AddTestStubRequest,
  pub verdict: AddTestStubVerdict,
}

/// Orchestrator: classify the request and package it as a candidate.
///
/// OWNER-LAW (2026-05-11): pure function. No I/O. Unlike rename-symbol
/// and remove-unused-import, this carrier does NOT emit a unified
/// diff — the host's CST emitter performs the stub emission under
/// `ToolActionApproval`. The candidate artifact is the verdict +
/// resolved place + intent, which the host downstream consumes.
pub fn compute_add_test_stub_candidate(request: &AddTestStubRequest) -> AddTestStubCandidate {
  let verdict = classify_add_test_stub(request);
  AddTestStubCandidate {
    request: request.clone(),
    verdict,
  }
}

/// Render an `AddTestStubCandidate` as the canonical JSON payload of
/// a `coding.code-transform.add-test-stub-*` artifact.
pub fn build_add_test_stub_candidate_payload(
  candidate: &AddTestStubCandidate,
) -> serde_json::Value {
  let request = &candidate.request;
  let (verdict_str, next_step) = match &candidate.verdict {
    AddTestStubVerdict::AddTestStubReady { .. } => (
      "add-test-stub-ready",
      "host-cst-emit-at-canonical-test-surface-then-tool-action-approval",
    ),
    AddTestStubVerdict::AddTestStubHeld { .. } => {
      ("add-test-stub-held", "operator-decision-or-resubmit")
    }
    AddTestStubVerdict::AddTestStubRejected { .. } => {
      ("add-test-stub-rejected", "operator-decision-or-resubmit")
    }
  };
  let mut payload = serde_json::json!({
    "transform": "add-test-stub",
    "owner_law": "stdlib/lib/gate/code-transform/add-test-stub.px",
    "target_module": request.target_module,
    "test_name": request.test_name,
    "language": request.language,
    "intent": request.intent,
    "verdict": verdict_str,
    "capability_required": "EditTestOnly",
    "candidate_only": true,
    "next_step": next_step,
  });
  match &candidate.verdict {
    AddTestStubVerdict::AddTestStubReady { resolved_place } => {
      payload["resolved_place"] = serde_json::Value::String(resolved_place.as_str().to_string());
    }
    AddTestStubVerdict::AddTestStubHeld { held_kind, reason }
    | AddTestStubVerdict::AddTestStubRejected { held_kind, reason } => {
      payload["held_kind"] = serde_json::Value::String(held_kind.as_str().to_string());
      payload["reason"] = serde_json::Value::String(reason.clone());
    }
  }
  // Echo the caller-supplied place when provided so audit can see
  // what the caller asked for (vs what got resolved).
  if let Some(p) = request.place {
    payload["place"] = serde_json::Value::String(p.as_str().to_string());
  }
  payload
}

/// Wrap an `AddTestStubCandidate` into a full
/// `coding.code-transform.add-test-stub-{ready,held,rejected}`
/// artifact value with a replay-stable id.
///
/// OWNER-LAW (2026-05-11): id hash binds intrinsic request identity:
/// target_module + test_name + language + intent + place (when
/// supplied) + verdict suffix.
pub fn build_add_test_stub_candidate_artifact(
  candidate: &AddTestStubCandidate,
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
) -> serde_json::Value {
  let payload = build_add_test_stub_candidate_payload(candidate);
  let suffix = match &candidate.verdict {
    AddTestStubVerdict::AddTestStubReady { .. } => "ready",
    AddTestStubVerdict::AddTestStubHeld { .. } => "held",
    AddTestStubVerdict::AddTestStubRejected { .. } => "rejected",
  };
  let artifact_family = format!("coding.code-transform.add-test-stub-{suffix}");

  let mut hasher = Sha256::new();
  hasher.update(b"add-test-stub-candidate\x1f");
  hasher.update(candidate.request.target_module.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(candidate.request.test_name.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(candidate.request.language.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(candidate.request.intent.as_bytes());
  hasher.update(b"\x1f");
  if let Some(p) = candidate.request.place {
    hasher.update(p.as_str().as_bytes());
  }
  hasher.update(b"\x1f");
  hasher.update(suffix.as_bytes());
  let digest = hasher.finalize();
  let prefix = digest
    .iter()
    .take(8)
    .map(|b| format!("{b:02x}"))
    .collect::<String>();
  let id = format!("add-test-stub.{prefix}");

  let mut artifact = serde_json::json!({
    "id": id,
    "artifact_family": artifact_family,
    "source_surface": "code-transform.add-test-stub",
    "stored_at_ms": stored_at_ms,
    "target_paths": [candidate.request.target_module.clone()],
    "command_refs": serde_json::Value::Array(Vec::new()),
    "related_refs": serde_json::json!([
      "owner-law:stdlib/lib/gate/code-transform/add-test-stub.px"
    ]),
    "payload": payload,
  });
  if let Some(snap) = repo_snapshot_ref {
    artifact["repo_snapshot_ref"] = serde_json::Value::String(snap.to_string());
  }
  artifact
}

// ─── Host CST emitter for Rust InlineCfgTest place ───────────────────
//
// OWNER-LAW (2026-05-12): the .px `add-test-stub.px` owner emits the
// `add-test-stub-ready` verdict and resolved place; the host carries
// the language-specific CST emit responsibility. This block is the
// **Rust-only host emitter** — it knows the canonical Rust convention
// (`#[cfg(test)] mod tests { ... }` inline test block) and emits a
// typed file patch with a renderable unified diff.
//
// Out of scope for this slice:
//   - python / typescript / javascript / go emit (each gets its own
//     host emitter in a follow-up — same shape, different lexical
//     conventions)
//   - extending the chain to review / apply / rollback (this slice
//     stops at "Ready + patch candidate", mirroring where
//     rename-symbol's `compute_rename_patch_candidate_*` family lives)

/// Per-file patch emitted by an `add-test-stub` host CST emitter.
/// Carries the before/after content + sha256 pair so the downstream
/// `tool-action-runtime` materializer can preflight-verify the
/// pre-apply bytes before writing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddTestStubFilePatch {
  pub path: String,
  pub before_content: String,
  pub after_content: String,
  pub before_sha256: String,
  pub after_sha256: String,
}

/// File input to the host CST emitter — borrows the on-disk content
/// so the caller stays the owner of the source bytes.
#[derive(Debug, Clone, Copy)]
pub struct AddTestStubFileInput<'a> {
  pub path: &'a str,
  pub content: &'a str,
}

/// Sealed candidate emitted by the Rust host CST emitter when the
/// classifier says Ready. Carries the typed file patch, the
/// renderable unified diff (for review / artifact payload), and the
/// upstream candidate (so the downstream apply chain has the request
/// + verdict in lockstep).
///
/// Empty `file_patches` + empty `unified_diff` is the canonical
/// "non-Ready or non-Rust" outcome — callers route Held / Rejected /
/// non-Rust through their normal candidate flow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddTestStubPatchCandidate {
  pub candidate: AddTestStubCandidate,
  pub file_patches: Vec<AddTestStubFilePatch>,
  pub unified_diff: String,
}

fn sha256_hex_bytes(bytes: &[u8]) -> String {
  let mut h = Sha256::new();
  h.update(bytes);
  format!("{:x}", h.finalize())
}

/// Mask Rust source for brace-balanced scanning. Replaces every byte
/// inside `// line comments`, `/* block comments */`, `"normal
/// strings"`, raw strings (`r"..."`, `r#"..."#`, etc.), and char
/// literals (`'x'`, `'\n'`, `'\u{...}'`) with a space, **preserving
/// newlines** so line-based pattern matching still works on the same
/// byte offsets.
///
/// Output length equals input length. Reading any byte at a given
/// offset in the masked string still corresponds to the same byte in
/// the original — except that braces/quotes/etc. inside lexical
/// regions have been blanked.
fn mask_rust_lexical_regions(src: &str) -> Vec<u8> {
  let bytes = src.as_bytes();
  let mut out: Vec<u8> = bytes.to_vec();
  let mut i = 0usize;
  while i < bytes.len() {
    let b = bytes[i];
    // Block comment `/* ... */` (no nesting handled for MVP — rare in
    // test modules).
    if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
      out[i] = b' ';
      out[i + 1] = b' ';
      i += 2;
      while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
        if bytes[i] != b'\n' {
          out[i] = b' ';
        }
        i += 1;
      }
      if i + 1 < bytes.len() {
        out[i] = b' ';
        out[i + 1] = b' ';
        i += 2;
      }
      continue;
    }
    // Line comment `// ... \n`.
    if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
      while i < bytes.len() && bytes[i] != b'\n' {
        out[i] = b' ';
        i += 1;
      }
      continue;
    }
    // Raw string `r"..."`, `r#"..."#`, `br#"..."#`.
    let raw_start = if b == b'r' || (b == b'b' && i + 1 < bytes.len() && bytes[i + 1] == b'r') {
      let prefix_len = if b == b'b' { 2 } else { 1 };
      let probe = i + prefix_len;
      let mut hashes = 0usize;
      let mut p = probe;
      while p < bytes.len() && bytes[p] == b'#' {
        hashes += 1;
        p += 1;
      }
      if p < bytes.len() && bytes[p] == b'"' {
        Some((prefix_len, hashes, p + 1))
      } else {
        None
      }
    } else {
      None
    };
    if let Some((prefix_len, hashes, body_start)) = raw_start {
      // Blank the prefix + opening hashes + opening quote.
      for k in i..body_start {
        out[k] = b' ';
      }
      i = body_start;
      // Scan body until matching `"` followed by `hashes` of `#`.
      while i < bytes.len() {
        if bytes[i] == b'"' {
          let mut k = i + 1;
          let mut got = 0;
          while got < hashes && k < bytes.len() && bytes[k] == b'#' {
            got += 1;
            k += 1;
          }
          if got == hashes {
            for j in i..k {
              out[j] = b' ';
            }
            i = k;
            break;
          }
        }
        if bytes[i] != b'\n' {
          out[i] = b' ';
        }
        i += 1;
      }
      let _ = prefix_len;
      continue;
    }
    // Normal string `"..."` with backslash escapes.
    if b == b'"' {
      out[i] = b' ';
      i += 1;
      while i < bytes.len() {
        if bytes[i] == b'\\' {
          out[i] = b' ';
          if i + 1 < bytes.len() {
            if bytes[i + 1] != b'\n' {
              out[i + 1] = b' ';
            }
            i += 2;
          } else {
            i += 1;
          }
          continue;
        }
        if bytes[i] == b'"' {
          out[i] = b' ';
          i += 1;
          break;
        }
        if bytes[i] != b'\n' {
          out[i] = b' ';
        }
        i += 1;
      }
      continue;
    }
    // Char literal `'X'`, `'\n'`, `'\u{1F600}'`. Conservatively scan
    // to the closing `'` within a small window; if not found, treat
    // as a lifetime and don't mask.
    if b == b'\'' {
      let scan_end = (i + 12).min(bytes.len());
      let mut j = i + 1;
      if j < scan_end && bytes[j] == b'\\' {
        j += 1;
        if j < scan_end && bytes[j] == b'u' {
          // Try to skip past `u{...}`.
          let mut k = j + 1;
          if k < scan_end && bytes[k] == b'{' {
            k += 1;
            while k < scan_end && bytes[k] != b'}' {
              k += 1;
            }
            if k < scan_end && bytes[k] == b'}' {
              j = k + 1;
            }
          }
        } else {
          j += 1;
        }
      } else if j < scan_end {
        j += 1;
      }
      if j < scan_end && bytes[j] == b'\'' {
        // It's a char literal. Mask `i..=j`.
        for k in i..=j {
          if bytes[k] != b'\n' {
            out[k] = b' ';
          }
        }
        i = j + 1;
        continue;
      }
      // Lifetime — leave unchanged.
      i += 1;
      continue;
    }
    i += 1;
  }
  out
}

/// Find a `#[cfg(test)] mod <ident> { ... }` block in Rust source.
/// Returns `(open_brace_pos, matching_close_brace_pos)` on the
/// masked source — pos is a byte offset into the original source too
/// because the mask preserves byte positions.
///
/// Scans for the first occurrence; production source rarely has more
/// than one inline cfg(test) block per file.
fn find_inline_cfg_test_block(src: &str) -> Option<(usize, usize)> {
  let masked = mask_rust_lexical_regions(src);
  // Find `#[cfg(test)]` literally (whitespace inside `#[ ... ]` is
  // tolerated by Rust but the canonical form is tight; for MVP match
  // the canonical form only).
  let needle = b"#[cfg(test)]";
  let mut search_from = 0usize;
  while let Some(rel) = window_find(&masked[search_from..], needle) {
    let attr_pos = search_from + rel;
    // Look forward for `mod ` ident `{` allowing whitespace.
    let after_attr = attr_pos + needle.len();
    if let Some(open_brace) = scan_for_mod_open_brace(&masked, after_attr) {
      // Brace-count from open_brace + 1.
      if let Some(close) = match_close_brace(&masked, open_brace) {
        return Some((open_brace, close));
      }
    }
    search_from = attr_pos + needle.len();
  }
  None
}

fn window_find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
  if needle.is_empty() || haystack.len() < needle.len() {
    return None;
  }
  for i in 0..=haystack.len() - needle.len() {
    if &haystack[i..i + needle.len()] == needle {
      return Some(i);
    }
  }
  None
}

/// From `pos`, skip whitespace, expect `mod`, skip whitespace +
/// identifier, skip whitespace, expect `{`. Returns the index of the
/// `{` if matched.
fn scan_for_mod_open_brace(masked: &[u8], pos: usize) -> Option<usize> {
  let mut i = pos;
  while i < masked.len() && masked[i].is_ascii_whitespace() {
    i += 1;
  }
  let keyword = b"mod";
  if i + keyword.len() > masked.len() || &masked[i..i + keyword.len()] != keyword {
    return None;
  }
  i += keyword.len();
  // Require whitespace between `mod` and ident.
  let mut saw_ws = false;
  while i < masked.len() && masked[i].is_ascii_whitespace() {
    saw_ws = true;
    i += 1;
  }
  if !saw_ws {
    return None;
  }
  // Identifier.
  let ident_start = i;
  while i < masked.len() && (masked[i].is_ascii_alphanumeric() || masked[i] == b'_') {
    i += 1;
  }
  if i == ident_start {
    return None;
  }
  while i < masked.len() && masked[i].is_ascii_whitespace() {
    i += 1;
  }
  if i < masked.len() && masked[i] == b'{' {
    Some(i)
  } else {
    None
  }
}

/// Balanced brace match on `masked`. `open` is the index of `{`.
/// Returns index of the matching `}`.
fn match_close_brace(masked: &[u8], open: usize) -> Option<usize> {
  if open >= masked.len() || masked[open] != b'{' {
    return None;
  }
  let mut depth = 1usize;
  let mut i = open + 1;
  while i < masked.len() {
    match masked[i] {
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

/// Build the inserted `#[test] fn <name>() { ... }` snippet with
/// `indent_unit` spaces of indentation per nesting level. Always uses
/// `todo!()` as the body. The intent (when non-empty) becomes a
/// single-line doc comment above the `#[test]` attribute.
fn build_rust_test_stub(test_name: &str, intent: &str, indent_unit: &str) -> String {
  let mut out = String::new();
  let intent_trim = intent.trim();
  if !intent_trim.is_empty() {
    out.push_str(indent_unit);
    out.push_str("// ");
    out.push_str(intent_trim);
    out.push('\n');
  }
  out.push_str(indent_unit);
  out.push_str("#[test]\n");
  out.push_str(indent_unit);
  out.push_str("fn ");
  out.push_str(test_name);
  out.push_str("() {\n");
  out.push_str(indent_unit);
  out.push_str(indent_unit);
  out.push_str("todo!();\n");
  out.push_str(indent_unit);
  out.push_str("}\n");
  out
}

/// Render a unified diff for the add-test-stub patch — minimal
/// canonical format consumable by `tool-action-runtime` and review
/// surfaces. Same `--- a/<path>` / `+++ b/<path>` shape as
/// rename-symbol's diff renderer (no line numbers — the materializer
/// uses content + sha256 round-trip, not line-anchored hunks).
fn render_unified_diff_add_test_stub(patches: &[AddTestStubFilePatch]) -> String {
  let mut out = String::new();
  for p in patches {
    out.push_str(&format!("--- a/{}\n", p.path));
    out.push_str(&format!("+++ b/{}\n", p.path));
    let before_lines: Vec<&str> = p.before_content.split_inclusive('\n').collect();
    let after_lines: Vec<&str> = p.after_content.split_inclusive('\n').collect();
    // Naive whole-file hunk: emit context-prefixed lines for both
    // sides. Sufficient as a human-readable summary; the canonical
    // proof is the sha256 pair.
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

/// Host CST emit for Rust `InlineCfgTest` place. Given the
/// classifier-Ready request + the target file's current content,
/// emit a typed patch that:
///   - extends an existing `#[cfg(test)] mod <ident> { ... }` block
///     by inserting a `#[test] fn <name>() { todo!() }` before the
///     block's closing `}`;
///   - or, when no such block exists, appends a fresh
///     `#[cfg(test)] mod tests { ... }` block at file end.
///
/// Returns an `AddTestStubPatchCandidate`. Empty `file_patches` +
/// empty `unified_diff` for non-Ready or non-Rust-InlineCfgTest
/// requests — caller's normal candidate flow (build_*_candidate_*
/// artifact) consumes these as Held/Rejected.
pub fn compute_add_test_stub_patch_candidate_rust(
  request: &AddTestStubRequest,
  file_input: &AddTestStubFileInput<'_>,
) -> AddTestStubPatchCandidate {
  let candidate = compute_add_test_stub_candidate(request);
  let resolved_place = match &candidate.verdict {
    AddTestStubVerdict::AddTestStubReady { resolved_place } => *resolved_place,
    _ => {
      return AddTestStubPatchCandidate {
        candidate,
        file_patches: Vec::new(),
        unified_diff: String::new(),
      };
    }
  };
  if request.language != "rust" || resolved_place != AddTestStubPlace::InlineCfgTest {
    return AddTestStubPatchCandidate {
      candidate,
      file_patches: Vec::new(),
      unified_diff: String::new(),
    };
  }
  let before = file_input.content;
  let after = emit_rust_inline_cfg_test_block(before, &request.test_name, &request.intent);
  if after == before {
    // No change emitted (e.g. emitter detected the test already
    // exists by name — for MVP we don't dedupe, but the equality
    // check is a safety net). Return empty patch.
    return AddTestStubPatchCandidate {
      candidate,
      file_patches: Vec::new(),
      unified_diff: String::new(),
    };
  }
  let before_sha256 = sha256_hex_bytes(before.as_bytes());
  let after_sha256 = sha256_hex_bytes(after.as_bytes());
  let patch = AddTestStubFilePatch {
    path: file_input.path.to_string(),
    before_content: before.to_string(),
    after_content: after,
    before_sha256,
    after_sha256,
  };
  let unified_diff = render_unified_diff_add_test_stub(std::slice::from_ref(&patch));
  AddTestStubPatchCandidate {
    candidate,
    file_patches: vec![patch],
    unified_diff,
  }
}

/// Pure emit for the Rust `InlineCfgTest` place. Public so other
/// hosts (e.g. the doghouse adapter) can call directly when they
/// already hold a request + content.
pub fn emit_rust_inline_cfg_test_block(source: &str, test_name: &str, intent: &str) -> String {
  // Indent unit: 4 spaces is the rustfmt default. Two-tab projects
  // would need a probe of the source's existing indent; out of scope
  // for MVP.
  let indent_unit = "    ";
  let stub = build_rust_test_stub(test_name, intent, indent_unit);

  if let Some((_open, close)) = find_inline_cfg_test_block(source) {
    // Insert before the closing `}`. Preserve whatever the file had
    // before `}` (likely a newline + indent of the closing `}`).
    let bytes = source.as_bytes();
    // Walk backwards from `close` to the previous non-whitespace
    // byte. The "natural" insertion point is right before the
    // newline that precedes `}` (or at `close` if `}` is at start of
    // line).
    let mut insert_at = close;
    // Step back over leading whitespace on the closing line so the
    // stub sits properly indented and the existing closing `}` stays
    // at its column.
    while insert_at > 0 && (bytes[insert_at - 1] == b' ' || bytes[insert_at - 1] == b'\t') {
      insert_at -= 1;
    }
    let mut out = String::with_capacity(source.len() + stub.len() + 2);
    out.push_str(&source[..insert_at]);
    if insert_at > 0 && bytes[insert_at - 1] != b'\n' {
      out.push('\n');
    }
    out.push_str(&stub);
    out.push_str(&source[insert_at..]);
    return out;
  }
  // No existing block — append a fresh one at file end.
  let mut out = String::with_capacity(source.len() + stub.len() + 64);
  out.push_str(source);
  if !source.ends_with('\n') {
    out.push('\n');
  }
  out.push('\n');
  out.push_str("#[cfg(test)]\n");
  out.push_str("mod tests {\n");
  out.push_str(&stub);
  out.push_str("}\n");
  out
}

/// Render an `AddTestStubPatchCandidate` as the canonical JSON
/// payload of a `coding.generated-patch-candidate` artifact for
/// the add-test-stub transform. Mirrors
/// `build_rename_symbol_patch_candidate_payload` shape so the cockpit
/// can render patch candidates uniformly across transforms.
pub fn build_add_test_stub_patch_candidate_payload(
  candidate: &AddTestStubPatchCandidate,
) -> serde_json::Value {
  let request = &candidate.candidate.request;
  let verdict_str = match &candidate.candidate.verdict {
    AddTestStubVerdict::AddTestStubReady { .. } => "add-test-stub-ready",
    AddTestStubVerdict::AddTestStubHeld { .. } => "add-test-stub-held",
    AddTestStubVerdict::AddTestStubRejected { .. } => "add-test-stub-rejected",
  };
  // `file_patches` projection: path + sha pair + byte counts. The
  // before/after content is intentionally NOT included by default —
  // diff payload below already carries the visible change, and the
  // sha256 round-trip is what the materializer pins (not the raw
  // bytes in the payload).
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
    "transform": "add-test-stub",
    "owner_law": "stdlib/lib/gate/code-transform/add-test-stub.px",
    "target_module": request.target_module,
    "test_name": request.test_name,
    "language": request.language,
    "intent": request.intent,
    "verdict": verdict_str,
    "capability_required": "EditTestOnly",
    "file_patches": file_patches_arr,
    "unified_diff": candidate.unified_diff,
    "candidate_only": true,
    "next_step": match candidate.candidate.verdict {
      AddTestStubVerdict::AddTestStubReady { .. } => "tool-action-approval-then-materialize",
      _ => "operator-decision-or-resubmit",
    },
  });
  match &candidate.candidate.verdict {
    AddTestStubVerdict::AddTestStubReady { resolved_place } => {
      payload["resolved_place"] = serde_json::Value::String(resolved_place.as_str().to_string());
    }
    AddTestStubVerdict::AddTestStubHeld { held_kind, reason }
    | AddTestStubVerdict::AddTestStubRejected { held_kind, reason } => {
      payload["held_kind"] = serde_json::Value::String(held_kind.as_str().to_string());
      payload["reason"] = serde_json::Value::String(reason.clone());
    }
  }
  if let Some(p) = request.place {
    payload["place"] = serde_json::Value::String(p.as_str().to_string());
  }
  payload
}

/// Wrap an `AddTestStubPatchCandidate` as a
/// `coding.generated-patch-candidate` artifact. Replay-stable
/// id mixes intrinsic request identity + per-file sha256 pair (so
/// the same request applied to byte-identical source produces the
/// same id at any wall-clock time).
pub fn build_add_test_stub_patch_candidate_artifact(
  candidate: &AddTestStubPatchCandidate,
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
) -> serde_json::Value {
  let payload = build_add_test_stub_patch_candidate_payload(candidate);
  let req = &candidate.candidate.request;

  let mut hasher = Sha256::new();
  hasher.update(b"add-test-stub-patch\x1f");
  hasher.update(req.target_module.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(req.test_name.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(req.language.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(req.intent.as_bytes());
  hasher.update(b"\x1f");
  if let Some(p) = req.place {
    hasher.update(p.as_str().as_bytes());
  }
  hasher.update(b"\x1f");
  // Per-file sha256 pair: same source bytes → same digest.
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
  let id = format!("generated-patch.add-test-stub.{prefix}");

  let mut artifact = serde_json::json!({
    "id": id,
    "artifact_family": "coding.generated-patch-candidate",
    "source_surface": "code-transform.add-test-stub",
    "stored_at_ms": stored_at_ms,
    "target_paths": [req.target_module.clone()],
    "command_refs": serde_json::Value::Array(Vec::new()),
    "related_refs": serde_json::json!([
      "owner-law:stdlib/lib/gate/code-transform/add-test-stub.px"
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

  fn req(target: &str, name: &str, language: &str) -> AddTestStubRequest {
    AddTestStubRequest {
      target_module: target.to_string(),
      test_name: name.to_string(),
      language: language.to_string(),
      intent: "checks the happy path".to_string(),
      place: None,
    }
  }

  // ─── classifier ──────────────────────────────────────────────────

  #[test]
  fn classify_ready_for_well_formed_rust_request() {
    let r = req("crates/foo/src/lib.rs", "happy_path", "rust");
    match classify_add_test_stub(&r) {
      AddTestStubVerdict::AddTestStubReady { resolved_place } => {
        assert_eq!(resolved_place, AddTestStubPlace::InlineCfgTest);
      }
      other => panic!("expected Ready, got {:?}", other),
    }
  }

  #[test]
  fn classify_ready_resolves_default_place_per_language() {
    let cases = [
      ("rust", AddTestStubPlace::InlineCfgTest),
      ("python", AddTestStubPlace::TestsDir),
      ("typescript", AddTestStubPlace::Sibling),
      ("javascript", AddTestStubPlace::Sibling),
      ("go", AddTestStubPlace::Sibling),
    ];
    for (lang, expected) in cases {
      let r = req("src/foo", "happy_path", lang);
      match classify_add_test_stub(&r) {
        AddTestStubVerdict::AddTestStubReady { resolved_place } => {
          assert_eq!(resolved_place, expected, "language {lang}");
        }
        other => panic!("expected Ready for {lang}, got {:?}", other),
      }
    }
  }

  #[test]
  fn classify_ready_respects_caller_supplied_place() {
    let mut r = req("src/foo.rs", "happy_path", "rust");
    r.place = Some(AddTestStubPlace::TestsDir);
    match classify_add_test_stub(&r) {
      AddTestStubVerdict::AddTestStubReady { resolved_place } => {
        // Caller's place wins over language default.
        assert_eq!(resolved_place, AddTestStubPlace::TestsDir);
      }
      other => panic!("expected Ready with caller place, got {:?}", other),
    }
  }

  #[test]
  fn classify_holds_on_missing_target_module() {
    let r = req("", "happy_path", "rust");
    match classify_add_test_stub(&r) {
      AddTestStubVerdict::AddTestStubHeld { held_kind, .. } => {
        assert_eq!(held_kind, AddTestStubHeldKind::MissingTargetModule);
      }
      other => panic!("expected MissingTargetModule, got {:?}", other),
    }
  }

  #[test]
  fn classify_holds_on_parent_traversal_in_target() {
    let r = req("../escape/lib.rs", "happy_path", "rust");
    match classify_add_test_stub(&r) {
      AddTestStubVerdict::AddTestStubHeld { held_kind, .. } => {
        assert_eq!(held_kind, AddTestStubHeldKind::TargetModuleOutOfProject);
      }
      other => panic!("expected TargetModuleOutOfProject, got {:?}", other),
    }
  }

  #[test]
  fn classify_holds_on_missing_test_name() {
    let r = req("src/foo.rs", "", "rust");
    match classify_add_test_stub(&r) {
      AddTestStubVerdict::AddTestStubHeld { held_kind, .. } => {
        assert_eq!(held_kind, AddTestStubHeldKind::MissingTestName);
      }
      other => panic!("expected MissingTestName, got {:?}", other),
    }
  }

  #[test]
  fn classify_rejects_invalid_test_name() {
    // Leading digit, spaces, hyphens — none are valid ASCII idents.
    let bad_names = ["1abc", "has space", "with-hyphen", "non-ASCII-ümlaut"];
    for bad in bad_names {
      let r = req("src/foo.rs", bad, "rust");
      match classify_add_test_stub(&r) {
        AddTestStubVerdict::AddTestStubRejected { held_kind, .. } => {
          assert_eq!(
            held_kind,
            AddTestStubHeldKind::InvalidTestName,
            "for test_name '{bad}'"
          );
        }
        other => panic!("expected InvalidTestName for '{bad}', got {:?}", other),
      }
    }
  }

  #[test]
  fn classify_accepts_valid_identifiers() {
    let good = ["foo", "_bar", "x1", "X_Y_Z", "test_with_underscores"];
    for name in good {
      let r = req("src/foo.rs", name, "rust");
      assert!(
        matches!(
          classify_add_test_stub(&r),
          AddTestStubVerdict::AddTestStubReady { .. }
        ),
        "expected Ready for valid name '{name}'"
      );
    }
  }

  #[test]
  fn classify_holds_on_unsupported_language() {
    let r = req("src/foo.rs", "happy_path", "fortran");
    match classify_add_test_stub(&r) {
      AddTestStubVerdict::AddTestStubHeld { held_kind, reason } => {
        assert_eq!(held_kind, AddTestStubHeldKind::LanguageNotSupported);
        assert!(reason.contains("fortran"));
      }
      other => panic!("expected LanguageNotSupported, got {:?}", other),
    }
  }

  #[test]
  fn classify_ladder_missing_target_wins_over_invalid_name() {
    // Both target_module empty AND test_name invalid → target check
    // runs first.
    let r = req("", "1bad", "rust");
    match classify_add_test_stub(&r) {
      AddTestStubVerdict::AddTestStubHeld { held_kind, .. } => {
        assert_eq!(held_kind, AddTestStubHeldKind::MissingTargetModule);
      }
      other => panic!("expected MissingTargetModule first, got {:?}", other),
    }
  }

  #[test]
  fn classify_ladder_missing_test_name_wins_over_invalid_name_when_both_empty() {
    // Empty test_name → MissingTestName (not InvalidTestName) since
    // is_valid_identifier("") is false but the missing check runs
    // first.
    let r = req("src/foo.rs", "", "rust");
    match classify_add_test_stub(&r) {
      AddTestStubVerdict::AddTestStubHeld { held_kind, .. } => {
        assert_eq!(held_kind, AddTestStubHeldKind::MissingTestName);
      }
      other => panic!("expected MissingTestName, got {:?}", other),
    }
  }

  #[test]
  fn classify_ladder_invalid_name_wins_over_language_when_name_bad() {
    // Bad name AND unsupported language → invalid-name (Rejected)
    // runs first per the ladder (4 before 5).
    let r = req("src/foo.rs", "1bad", "fortran");
    match classify_add_test_stub(&r) {
      AddTestStubVerdict::AddTestStubRejected { held_kind, .. } => {
        assert_eq!(held_kind, AddTestStubHeldKind::InvalidTestName);
      }
      other => panic!("expected InvalidTestName rejection, got {:?}", other),
    }
  }

  // ─── candidate + canonical artifact ──────────────────────────────

  #[test]
  fn compute_candidate_carries_verdict_and_request() {
    let r = req("src/foo.rs", "happy_path", "rust");
    let cand = compute_add_test_stub_candidate(&r);
    assert_eq!(cand.request, r);
    assert!(matches!(
      cand.verdict,
      AddTestStubVerdict::AddTestStubReady { .. }
    ));
  }

  #[test]
  fn payload_canonical_fields_for_ready() {
    let r = req("crates/foo/src/lib.rs", "happy_path", "rust");
    let cand = compute_add_test_stub_candidate(&r);
    let payload = build_add_test_stub_candidate_payload(&cand);
    assert_eq!(payload["transform"].as_str(), Some("add-test-stub"));
    assert_eq!(
      payload["owner_law"].as_str(),
      Some("stdlib/lib/gate/code-transform/add-test-stub.px")
    );
    assert_eq!(
      payload["target_module"].as_str(),
      Some("crates/foo/src/lib.rs")
    );
    assert_eq!(payload["test_name"].as_str(), Some("happy_path"));
    assert_eq!(payload["language"].as_str(), Some("rust"));
    assert_eq!(payload["intent"].as_str(), Some("checks the happy path"));
    assert_eq!(payload["verdict"].as_str(), Some("add-test-stub-ready"));
    assert_eq!(payload["resolved_place"].as_str(), Some("inline-cfg-test"));
    assert_eq!(
      payload["capability_required"].as_str(),
      Some("EditTestOnly")
    );
    assert_eq!(payload["candidate_only"].as_bool(), Some(true));
    assert_eq!(
      payload["next_step"].as_str(),
      Some("host-cst-emit-at-canonical-test-surface-then-tool-action-approval")
    );
    // No held_kind / reason on Ready.
    assert!(payload.get("held_kind").is_none());
  }

  #[test]
  fn payload_carries_held_kind_and_reason_for_held() {
    let r = req("", "x", "rust");
    let cand = compute_add_test_stub_candidate(&r);
    let payload = build_add_test_stub_candidate_payload(&cand);
    assert_eq!(payload["verdict"].as_str(), Some("add-test-stub-held"));
    assert_eq!(payload["held_kind"].as_str(), Some("missing-target-module"));
    assert!(payload["reason"]
      .as_str()
      .unwrap()
      .contains("target_module"));
    assert_eq!(
      payload["next_step"].as_str(),
      Some("operator-decision-or-resubmit")
    );
    // No resolved_place on non-Ready.
    assert!(payload.get("resolved_place").is_none());
  }

  #[test]
  fn payload_carries_held_kind_and_reason_for_rejected() {
    let r = req("src/foo.rs", "1bad", "rust");
    let cand = compute_add_test_stub_candidate(&r);
    let payload = build_add_test_stub_candidate_payload(&cand);
    assert_eq!(payload["verdict"].as_str(), Some("add-test-stub-rejected"));
    assert_eq!(payload["held_kind"].as_str(), Some("invalid-test-name"));
  }

  #[test]
  fn payload_echoes_caller_supplied_place_when_set() {
    let mut r = req("src/foo.rs", "happy_path", "rust");
    r.place = Some(AddTestStubPlace::Sibling);
    let cand = compute_add_test_stub_candidate(&r);
    let payload = build_add_test_stub_candidate_payload(&cand);
    // Caller's place echoed.
    assert_eq!(payload["place"].as_str(), Some("sibling"));
    // Resolved place reflects it (caller wins over default).
    assert_eq!(payload["resolved_place"].as_str(), Some("sibling"));
  }

  #[test]
  fn artifact_envelope_shape_ready() {
    let r = req("src/foo.rs", "happy_path", "rust");
    let cand = compute_add_test_stub_candidate(&r);
    let art = build_add_test_stub_candidate_artifact(&cand, 1700000000000, None);
    assert_eq!(
      art["artifact_family"].as_str(),
      Some("coding.code-transform.add-test-stub-ready")
    );
    assert_eq!(
      art["source_surface"].as_str(),
      Some("code-transform.add-test-stub")
    );
    assert_eq!(art["stored_at_ms"].as_u64(), Some(1700000000000));
    let id = art["id"].as_str().expect("id");
    assert!(id.starts_with("add-test-stub."));
    assert_eq!(art["target_paths"][0].as_str(), Some("src/foo.rs"));
    let related = art["related_refs"].as_array().expect("related_refs");
    assert!(related.iter().any(|v| v
      .as_str()
      .map(|s| s == "owner-law:stdlib/lib/gate/code-transform/add-test-stub.px")
      .unwrap_or(false)));
  }

  #[test]
  fn artifact_family_changes_with_verdict() {
    // Ready
    let ready_cand = compute_add_test_stub_candidate(&req("src/foo.rs", "ok", "rust"));
    let ready_art = build_add_test_stub_candidate_artifact(&ready_cand, 0, None);
    assert_eq!(
      ready_art["artifact_family"].as_str(),
      Some("coding.code-transform.add-test-stub-ready")
    );
    // Held: missing target_module
    let held_cand = compute_add_test_stub_candidate(&req("", "ok", "rust"));
    let held_art = build_add_test_stub_candidate_artifact(&held_cand, 0, None);
    assert_eq!(
      held_art["artifact_family"].as_str(),
      Some("coding.code-transform.add-test-stub-held")
    );
    // Rejected: invalid test_name
    let rej_cand = compute_add_test_stub_candidate(&req("src/foo.rs", "1bad", "rust"));
    let rej_art = build_add_test_stub_candidate_artifact(&rej_cand, 0, None);
    assert_eq!(
      rej_art["artifact_family"].as_str(),
      Some("coding.code-transform.add-test-stub-rejected")
    );
  }

  #[test]
  fn artifact_id_replay_stable_across_stored_at_ms() {
    let cand = compute_add_test_stub_candidate(&req("src/foo.rs", "ok", "rust"));
    let a = build_add_test_stub_candidate_artifact(&cand, 1000, None);
    let b = build_add_test_stub_candidate_artifact(&cand, 9999, None);
    assert_eq!(a["id"], b["id"], "stored_at_ms is extrinsic");
    assert_ne!(a["stored_at_ms"], b["stored_at_ms"]);
  }

  #[test]
  fn artifact_id_differs_per_target_or_name_or_language() {
    let base = compute_add_test_stub_candidate(&req("src/a.rs", "ok", "rust"));
    let diff_target = compute_add_test_stub_candidate(&req("src/b.rs", "ok", "rust"));
    let diff_name = compute_add_test_stub_candidate(&req("src/a.rs", "different", "rust"));
    let diff_lang = compute_add_test_stub_candidate(&req("src/a.rs", "ok", "python"));
    let id_base = build_add_test_stub_candidate_artifact(&base, 0, None)["id"].clone();
    let id_t = build_add_test_stub_candidate_artifact(&diff_target, 0, None)["id"].clone();
    let id_n = build_add_test_stub_candidate_artifact(&diff_name, 0, None)["id"].clone();
    let id_l = build_add_test_stub_candidate_artifact(&diff_lang, 0, None)["id"].clone();
    assert_ne!(id_base, id_t);
    assert_ne!(id_base, id_n);
    assert_ne!(id_base, id_l);
  }

  #[test]
  fn artifact_id_differs_per_caller_supplied_place() {
    let mut a_req = req("src/foo.rs", "ok", "rust");
    a_req.place = Some(AddTestStubPlace::InlineCfgTest);
    let mut b_req = req("src/foo.rs", "ok", "rust");
    b_req.place = Some(AddTestStubPlace::TestsDir);
    let a = compute_add_test_stub_candidate(&a_req);
    let b = compute_add_test_stub_candidate(&b_req);
    let id_a = build_add_test_stub_candidate_artifact(&a, 0, None)["id"].clone();
    let id_b = build_add_test_stub_candidate_artifact(&b, 0, None)["id"].clone();
    assert_ne!(id_a, id_b, "caller's place should affect id");
  }

  #[test]
  fn artifact_carries_repo_snapshot_ref_when_provided() {
    let cand = compute_add_test_stub_candidate(&req("src/foo.rs", "ok", "rust"));
    let art = build_add_test_stub_candidate_artifact(&cand, 0, Some("commit-abc"));
    assert_eq!(art["repo_snapshot_ref"].as_str(), Some("commit-abc"));
  }

  #[test]
  fn helper_predicates() {
    assert!(is_supported_language("rust"));
    assert!(is_supported_language("typescript"));
    assert!(!is_supported_language("fortran"));

    assert!(is_path_in_project("crates/foo/src/lib.rs"));
    assert!(!is_path_in_project(""));
    assert!(!is_path_in_project("../escape.rs"));

    assert!(is_valid_identifier("foo"));
    assert!(is_valid_identifier("_bar"));
    assert!(is_valid_identifier("x1"));
    assert!(!is_valid_identifier(""));
    assert!(!is_valid_identifier("1abc"));
    assert!(!is_valid_identifier("has space"));
    assert!(!is_valid_identifier("with-hyphen"));

    assert_eq!(
      AddTestStubPlace::default_for("rust"),
      Some(AddTestStubPlace::InlineCfgTest)
    );
    assert_eq!(AddTestStubPlace::default_for("fortran"), None);
  }

  // ─── Rust host CST emitter ──────────────────────────────────────

  fn ready_rust_request(test_name: &str, intent: &str) -> AddTestStubRequest {
    AddTestStubRequest {
      target_module: "src/lib.rs".into(),
      test_name: test_name.into(),
      language: "rust".into(),
      intent: intent.into(),
      place: None, // resolves to InlineCfgTest for rust
    }
  }

  #[test]
  fn rust_emit_appends_new_cfg_test_block_when_absent() {
    let source = "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n";
    let request = ready_rust_request("adds_two_positive_integers", "1+2 should equal 3");
    let file_input = AddTestStubFileInput {
      path: "src/lib.rs",
      content: source,
    };
    let cand = compute_add_test_stub_patch_candidate_rust(&request, &file_input);
    assert_eq!(cand.file_patches.len(), 1);
    let p = &cand.file_patches[0];
    assert_eq!(p.path, "src/lib.rs");
    assert_eq!(p.before_content, source);
    // Original content preserved at the top.
    assert!(p
      .after_content
      .starts_with("pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n"));
    // New cfg(test) block at end.
    assert!(p.after_content.contains("\n#[cfg(test)]\nmod tests {\n"));
    // Test fn with intent doc.
    assert!(p.after_content.contains("// 1+2 should equal 3\n"));
    assert!(p.after_content.contains("#[test]\n"));
    assert!(p
      .after_content
      .contains("fn adds_two_positive_integers() {\n"));
    assert!(p.after_content.contains("todo!();"));
    // sha256 pair distinct.
    assert_ne!(p.before_sha256, p.after_sha256);
    // Unified diff has both halves.
    assert!(cand.unified_diff.contains("--- a/src/lib.rs"));
    assert!(cand.unified_diff.contains("+++ b/src/lib.rs"));
    assert!(cand.unified_diff.contains("+#[cfg(test)]"));
  }

  #[test]
  fn rust_emit_extends_existing_cfg_test_block() {
    let source = "\
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn existing_test_stays() {
        assert_eq!(add(1, 1), 2);
    }
}
";
    let request = ready_rust_request("adds_negative_numbers", "");
    let file_input = AddTestStubFileInput {
      path: "src/lib.rs",
      content: source,
    };
    let cand = compute_add_test_stub_patch_candidate_rust(&request, &file_input);
    assert_eq!(cand.file_patches.len(), 1);
    let after = &cand.file_patches[0].after_content;
    // Existing test preserved.
    assert!(after.contains("fn existing_test_stays()"));
    // New test inside the same block.
    assert!(after.contains("fn adds_negative_numbers()"));
    // Critical: only ONE cfg(test) block (we didn't create a second one).
    let cfg_count = after.matches("#[cfg(test)]").count();
    assert_eq!(
      cfg_count, 1,
      "must reuse existing block; got {} blocks",
      cfg_count
    );
    // Critical: only ONE `mod tests` keyword pair.
    let mod_tests_count = after.matches("mod tests {").count();
    assert_eq!(mod_tests_count, 1);
  }

  #[test]
  fn rust_emit_returns_empty_patches_for_held_request() {
    let mut request = ready_rust_request("ok", "");
    request.target_module = String::new(); // → Held(MissingTargetModule)
    let file_input = AddTestStubFileInput {
      path: "src/lib.rs",
      content: "fn x() {}\n",
    };
    let cand = compute_add_test_stub_patch_candidate_rust(&request, &file_input);
    assert!(cand.file_patches.is_empty());
    assert!(cand.unified_diff.is_empty());
    assert!(matches!(
      cand.candidate.verdict,
      AddTestStubVerdict::AddTestStubHeld { .. }
    ));
  }

  #[test]
  fn rust_emit_returns_empty_patches_for_non_rust_language() {
    let request = AddTestStubRequest {
      target_module: "src/foo.py".into(),
      test_name: "ok".into(),
      language: "python".into(),
      intent: "".into(),
      place: None,
    };
    let file_input = AddTestStubFileInput {
      path: "src/foo.py",
      content: "def foo():\n    return 1\n",
    };
    let cand = compute_add_test_stub_patch_candidate_rust(&request, &file_input);
    assert!(
      cand.file_patches.is_empty(),
      "Rust host emitter must no-op for non-Rust requests; got {} patches",
      cand.file_patches.len()
    );
    assert!(matches!(
      cand.candidate.verdict,
      AddTestStubVerdict::AddTestStubReady { .. }
    ));
  }

  #[test]
  fn rust_emit_returns_empty_patches_when_place_not_inline_cfg_test() {
    let mut request = ready_rust_request("ok", "");
    request.place = Some(AddTestStubPlace::TestsDir);
    let file_input = AddTestStubFileInput {
      path: "src/lib.rs",
      content: "fn x() {}\n",
    };
    let cand = compute_add_test_stub_patch_candidate_rust(&request, &file_input);
    assert!(cand.file_patches.is_empty());
  }

  #[test]
  fn rust_emit_brace_counter_ignores_braces_in_strings_and_comments() {
    // A pre-existing cfg(test) block whose body contains `{` and `}`
    // inside strings, char literals, and comments. The naive
    // brace-balanced scanner would miscount these without lexical
    // masking — this test pins that behavior.
    let source = "\
#[cfg(test)]
mod tests {
    #[test]
    fn lexical_chaos() {
        let s = \"contains } and { braces\";
        let c = '{';
        // and a trailing } in a line comment
        /* and a } in a block comment */
        let raw = r#\"also } here\"#;
        assert_eq!(s.len() + (c == '{') as usize, 24);
        let _ = raw;
    }
}
";
    let request = ready_rust_request("second_test", "");
    let file_input = AddTestStubFileInput {
      path: "src/lib.rs",
      content: source,
    };
    let cand = compute_add_test_stub_patch_candidate_rust(&request, &file_input);
    assert_eq!(cand.file_patches.len(), 1);
    let after = &cand.file_patches[0].after_content;
    // Only one cfg(test) block — confirming we found the existing
    // matching `}` correctly, not a `}` inside a string.
    assert_eq!(after.matches("#[cfg(test)]").count(), 1);
    // Both tests present.
    assert!(after.contains("fn lexical_chaos()"));
    assert!(after.contains("fn second_test()"));
    // Lexical content preserved.
    assert!(after.contains("\"contains } and { braces\""));
    assert!(after.contains("r#\"also } here\"#"));
  }

  #[test]
  fn rust_emit_replay_stable_same_inputs_same_output() {
    let source = "fn x() {}\n";
    let request = ready_rust_request("y", "intent goes here");
    let file_input = AddTestStubFileInput {
      path: "src/lib.rs",
      content: source,
    };
    let a = compute_add_test_stub_patch_candidate_rust(&request, &file_input);
    let b = compute_add_test_stub_patch_candidate_rust(&request, &file_input);
    assert_eq!(a, b, "same inputs must produce byte-identical output");
  }

  // ─── patch-candidate artifact wrappers ───────────────────────

  #[test]
  fn patch_candidate_payload_canonical_fields_for_ready() {
    let source = "fn x() {}\n";
    let request = ready_rust_request("happy", "intent goes here");
    let file_input = AddTestStubFileInput {
      path: "src/lib.rs",
      content: source,
    };
    let cand = compute_add_test_stub_patch_candidate_rust(&request, &file_input);
    let payload = build_add_test_stub_patch_candidate_payload(&cand);
    assert_eq!(payload["transform"].as_str(), Some("add-test-stub"));
    assert_eq!(
      payload["owner_law"].as_str(),
      Some("stdlib/lib/gate/code-transform/add-test-stub.px")
    );
    assert_eq!(payload["verdict"].as_str(), Some("add-test-stub-ready"));
    assert_eq!(payload["candidate_only"].as_bool(), Some(true));
    assert_eq!(
      payload["capability_required"].as_str(),
      Some("EditTestOnly")
    );
    assert_eq!(
      payload["next_step"].as_str(),
      Some("tool-action-approval-then-materialize")
    );
    assert_eq!(payload["language"].as_str(), Some("rust"));
    assert_eq!(payload["test_name"].as_str(), Some("happy"));
    assert_eq!(payload["target_module"].as_str(), Some("src/lib.rs"));
    assert_eq!(payload["intent"].as_str(), Some("intent goes here"));
    assert_eq!(payload["resolved_place"].as_str(), Some("inline-cfg-test"));

    let fps = payload["file_patches"].as_array().expect("array");
    assert_eq!(fps.len(), 1);
    assert_eq!(fps[0]["path"].as_str(), Some("src/lib.rs"));
    assert!(fps[0]["before_sha256"].is_string());
    assert!(fps[0]["after_sha256"].is_string());
    assert_ne!(fps[0]["before_sha256"], fps[0]["after_sha256"]);
    assert!(payload["unified_diff"].as_str().unwrap().contains("--- a/"));
  }

  #[test]
  fn patch_candidate_payload_held_carries_held_kind_and_reason() {
    let mut request = ready_rust_request("ok", "");
    request.target_module = String::new();
    let file_input = AddTestStubFileInput {
      path: "src/lib.rs",
      content: "fn x() {}\n",
    };
    let cand = compute_add_test_stub_patch_candidate_rust(&request, &file_input);
    let payload = build_add_test_stub_patch_candidate_payload(&cand);
    assert_eq!(payload["verdict"].as_str(), Some("add-test-stub-held"));
    assert_eq!(payload["held_kind"].as_str(), Some("missing-target-module"));
    assert!(payload["reason"]
      .as_str()
      .unwrap()
      .contains("target_module"));
    assert_eq!(
      payload["next_step"].as_str(),
      Some("operator-decision-or-resubmit")
    );
    assert_eq!(payload["file_patches"].as_array().unwrap().len(), 0);
  }

  #[test]
  fn patch_candidate_artifact_id_is_replay_stable_across_stored_at_ms() {
    let source = "fn x() {}\n";
    let request = ready_rust_request("y", "intent");
    let file_input = AddTestStubFileInput {
      path: "src/lib.rs",
      content: source,
    };
    let cand = compute_add_test_stub_patch_candidate_rust(&request, &file_input);
    let a = build_add_test_stub_patch_candidate_artifact(&cand, 0, None);
    let b = build_add_test_stub_patch_candidate_artifact(&cand, 9_999_999, None);
    assert_eq!(a["id"], b["id"]);
    assert_eq!(a["artifact_family"], b["artifact_family"]);
    assert_ne!(a["stored_at_ms"], b["stored_at_ms"]);
  }

  #[test]
  fn patch_candidate_artifact_id_differs_per_source_or_request() {
    let request = ready_rust_request("name1", "");
    let fi_a = AddTestStubFileInput {
      path: "src/lib.rs",
      content: "fn a() {}\n",
    };
    let fi_b = AddTestStubFileInput {
      path: "src/lib.rs",
      content: "fn b() {}\n", // different source content
    };
    let cand_a = compute_add_test_stub_patch_candidate_rust(&request, &fi_a);
    let cand_b = compute_add_test_stub_patch_candidate_rust(&request, &fi_b);
    let id_a = build_add_test_stub_patch_candidate_artifact(&cand_a, 0, None)["id"]
      .as_str()
      .unwrap()
      .to_string();
    let id_b = build_add_test_stub_patch_candidate_artifact(&cand_b, 0, None)["id"]
      .as_str()
      .unwrap()
      .to_string();
    assert_ne!(
      id_a, id_b,
      "different source → different sha pair → different id"
    );

    let req2 = ready_rust_request("name2", "");
    let cand_c = compute_add_test_stub_patch_candidate_rust(&req2, &fi_a);
    let id_c = build_add_test_stub_patch_candidate_artifact(&cand_c, 0, None)["id"]
      .as_str()
      .unwrap()
      .to_string();
    assert_ne!(id_a, id_c, "different test_name → different id");
  }

  #[test]
  fn patch_candidate_artifact_envelope_canonical_fields() {
    let source = "fn x() {}\n";
    let request = ready_rust_request("env", "");
    let file_input = AddTestStubFileInput {
      path: "src/lib.rs",
      content: source,
    };
    let cand = compute_add_test_stub_patch_candidate_rust(&request, &file_input);
    let art =
      build_add_test_stub_patch_candidate_artifact(&cand, 1_700_000_000_000, Some("commit-xyz"));
    assert_eq!(
      art["artifact_family"].as_str(),
      Some("coding.generated-patch-candidate")
    );
    assert_eq!(
      art["source_surface"].as_str(),
      Some("code-transform.add-test-stub")
    );
    assert!(art["id"]
      .as_str()
      .unwrap()
      .starts_with("generated-patch.add-test-stub."));
    assert_eq!(art["repo_snapshot_ref"].as_str(), Some("commit-xyz"));
    assert_eq!(art["target_paths"][0].as_str(), Some("src/lib.rs"));
    let rrefs = art["related_refs"].as_array().expect("related_refs");
    assert!(rrefs
      .iter()
      .any(|v| v.as_str().map_or(false, |s| s.contains("add-test-stub.px"))));
  }

  #[test]
  fn rust_emit_empty_intent_omits_doc_comment() {
    let source = "fn x() {}\n";
    let request = ready_rust_request("no_doc", "");
    let file_input = AddTestStubFileInput {
      path: "src/lib.rs",
      content: source,
    };
    let cand = compute_add_test_stub_patch_candidate_rust(&request, &file_input);
    let after = &cand.file_patches[0].after_content;
    assert!(after.contains("fn no_doc()"));
    // No `// ` doc line above the #[test] attr.
    let before_attr = after
      .split("#[test]\n")
      .next()
      .expect("split before #[test]");
    assert!(
      !before_attr.contains("// "),
      "empty intent must not emit a doc comment line; saw: {before_attr:?}"
    );
  }
}
