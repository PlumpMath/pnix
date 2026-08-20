//! Rename-symbol deterministic code transform host carrier.
//!
//! OWNER-LAW (2026-05-11, mirroring
//! `stdlib/lib/gate/code-transform/rename-symbol.px`): given a request
//! to rename `old_name` to `new_name` across a bounded set of target
//! paths, this module:
//!
//! 1. Classifies the request into `Ready` / `Held` / `Rejected` via
//!    [`classify_rename`] — pure function, no I/O. Mirrors the `.px`
//!    classifier.
//! 2. When `Ready`, computes whole-word identifier edits per file via
//!    [`compute_file_rename_edits`] — also pure. Returns byte
//!    offsets / lengths so a downstream patch-emitter can render a
//!    unified diff, JSON patch, or apply in-place.
//!
//! What this carrier does NOT do (delegated to other layers):
//!
//! - Read files from disk. Callers pass `(path, content)` tuples.
//! - Tree-sitter / CST parsing. The first slice is token-based —
//!   any identifier token equal to `old_name` (with letter/digit/
//!   underscore boundaries) is renamed. String-literal and comment
//!   awareness is a later upgrade (`held_kind = "ambiguous-symbol-resolution"`
//!   would lift to the gate when the host adds it).
//! - Apply edits. Edits are pure data; a separate
//!   `ToolActionApproval` gate decides whether to write them to disk.
//! - LLM anything. No model is asked to "improve the rename" — that
//!   would be a constitution violation.
//!
//! The carrier is intentionally minimal so the audit story stays
//! crisp: every produced edit is a `(byte_offset, byte_len)` over a
//! specific input string, reproducible from the same inputs.

use serde::{Deserialize, Serialize};
use pnix_hash::{Digest, Sha256};

/// Scope qualifier from the `.px` owner law. `WorkspaceWide` is
/// Held-by-default; the host carrier won't emit edits without an
/// explicit owner approval slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RenameScope {
  LocalTargetPaths,
  CrateWide,
  WorkspaceWide,
}

impl RenameScope {
  pub const ALL: &'static [Self] = &[Self::LocalTargetPaths, Self::CrateWide, Self::WorkspaceWide];

  pub fn as_str(self) -> &'static str {
    match self {
      Self::LocalTargetPaths => "local-target-paths",
      Self::CrateWide => "crate-wide",
      Self::WorkspaceWide => "workspace-wide",
    }
  }
}

/// One of the documented Held / Rejected outcomes in the `.px`
/// owner law's held_kind ledger. Each variant maps 1:1 to the
/// `held_kind` string the law (or host strict preflight) emits.
/// Adding a variant here requires adding the matching string to the
/// `.px` ledger so the `check-code-transform-owner-carrier-sync.sh`
/// guard stays green.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RenameHeldKind {
  MissingOldName,
  MissingNewName,
  OldNameEqualsNewName,
  InvalidIdentifier,
  LanguageNotSupported,
  ScopeTooBroad,
  TargetPathOutOfProject,
  TargetPathEmpty,
  /// Host-emitted: strict preflight found that `request.target_paths`
  /// names a path which is not present in the staged input files.
  /// The host cannot rename what it cannot see.
  TargetPathContentMissing,
  /// Host-emitted: the same target path appears more than once in the
  /// staged input files. This is ambiguous — the carrier cannot
  /// decide which `content` reflects the real source. Replay would
  /// also be non-deterministic because order-of-staging would change
  /// the verdict. Resolve by deduplicating upstream.
  DuplicateTargetPathContent,
  /// Host-emitted: strict preflight scanned every staged target file
  /// and found zero standalone occurrences of `old_name`. The rename
  /// is a no-op; surface as Held so the operator can adjust the
  /// request (typo? wrong symbol name?) rather than ingest an empty
  /// patch.
  OldNameNotFound,
  NameCollisionDetected,
  MacroOrMetaBinding,
  ExternalSymbolReexport,
  AmbiguousSymbolResolution,
  /// `.px` D-23: when `target_fn_name` is supplied for a scope-aware
  /// rename, the language must support function-body scope. pnix
  /// graph-mode has no fn-body, so a scope-aware request against pnix
  /// is Held.
  ScopeAwareLanguageNotSupported,
  /// `.px` D-23: `target_fn_name` must be a valid ASCII identifier
  /// when non-empty.
  TargetFnNameInvalid,
}

impl RenameHeldKind {
  pub const ALL: &'static [Self] = &[
    Self::MissingOldName,
    Self::MissingNewName,
    Self::OldNameEqualsNewName,
    Self::InvalidIdentifier,
    Self::LanguageNotSupported,
    Self::ScopeTooBroad,
    Self::TargetPathOutOfProject,
    Self::TargetPathEmpty,
    Self::TargetPathContentMissing,
    Self::DuplicateTargetPathContent,
    Self::OldNameNotFound,
    Self::NameCollisionDetected,
    Self::MacroOrMetaBinding,
    Self::ExternalSymbolReexport,
    Self::AmbiguousSymbolResolution,
    Self::ScopeAwareLanguageNotSupported,
    Self::TargetFnNameInvalid,
  ];
  pub fn as_str(self) -> &'static str {
    match self {
      Self::MissingOldName => "missing-old-name",
      Self::MissingNewName => "missing-new-name",
      Self::OldNameEqualsNewName => "old-name-equals-new-name",
      Self::InvalidIdentifier => "invalid-identifier",
      Self::LanguageNotSupported => "language-not-supported",
      Self::ScopeTooBroad => "scope-too-broad",
      Self::TargetPathOutOfProject => "target-path-out-of-project",
      Self::TargetPathEmpty => "target-path-empty",
      Self::TargetPathContentMissing => "target-path-content-missing",
      Self::DuplicateTargetPathContent => "duplicate-target-path-content",
      Self::OldNameNotFound => "old-name-not-found",
      Self::NameCollisionDetected => "name-collision-detected",
      Self::MacroOrMetaBinding => "macro-or-meta-binding",
      Self::ExternalSymbolReexport => "external-symbol-reexport",
      Self::AmbiguousSymbolResolution => "ambiguous-symbol-resolution",
      Self::ScopeAwareLanguageNotSupported => "scope-aware-language-not-supported",
      Self::TargetFnNameInvalid => "target-fn-name-invalid",
    }
  }
}

pub const SUPPORTED_LANGUAGES: &[&str] =
  &["rust", "python", "typescript", "javascript", "go", "pnix"];

/// Verdict from [`classify_rename`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "verdict")]
pub enum RenameVerdict {
  /// All preconditions satisfied. Host may proceed to CST rewrite via
  /// [`compute_file_rename_edits`].
  RenameReady,
  /// Precondition violated, but the request might succeed after the
  /// operator adjusts inputs (e.g. provide non-empty `old_name`,
  /// widen scope).
  RenameHeld {
    held_kind: RenameHeldKind,
    reason: String,
  },
  /// Precondition violated in a way the operator cannot recover from
  /// without changing the *meaning* of the request (e.g.
  /// `old_name == new_name`).
  RenameRejected {
    held_kind: RenameHeldKind,
    reason: String,
  },
}

/// A rename request, mirroring the `.px` request payload shape.
///
/// `target_fn_name` (D-23, optional): when `Some(name)`, the rename
/// is restricted to occurrences inside the body of the
/// `fn <name>(...) { ... }` declaration. Only supported for
/// `language = "rust"` at v0 — other languages with a `Some` value
/// will surface `scope_aware_error` in the resulting patch
/// candidate. `#[serde(default)]` keeps the field backward-compatible
/// — callers that omit it get the existing whole-file rename
/// behavior.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenameRequest {
  pub old_name: String,
  pub new_name: String,
  pub target_paths: Vec<String>,
  pub language: String,
  pub scope: RenameScope,
  #[serde(default)]
  pub target_fn_name: Option<String>,
}

/// One identifier match in a file's content. Byte-indexed so callers
/// can compute a unified diff or apply the rewrite directly.
///
/// `byte_offset` is the start position of the matched `old_name`;
/// `byte_len` equals `old_name.len()`. `line` and `column` are
/// 1-indexed (byte column, not grapheme).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenameEdit {
  pub byte_offset: usize,
  pub byte_len: usize,
  pub line: usize,
  pub column: usize,
}

/// Conservative identifier validator — matches the `.px` law's
/// `isValidIdentifier`. ASCII letter or underscore at start;
/// ASCII letter / digit / underscore body.
fn is_valid_identifier(s: &str) -> bool {
  let bytes = s.as_bytes();
  if bytes.is_empty() {
    return false;
  }
  let first_ok = bytes[0].is_ascii_alphabetic() || bytes[0] == b'_';
  if !first_ok {
    return false;
  }
  bytes
    .iter()
    .skip(1)
    .all(|&b| b.is_ascii_alphanumeric() || b == b'_')
}

fn is_supported_language(lang: &str) -> bool {
  matches!(
    lang,
    "rust" | "python" | "typescript" | "javascript" | "go" | "pnix"
  )
}

/// True iff the path is project-safe: non-empty, no `..` parent
/// traversal, no null bytes. Mirrors `.px` law's `isPathInProject`.
fn is_path_in_project(p: &str) -> bool {
  !p.is_empty() && !p.contains("..") && !p.contains('\u{0}')
}

/// Pure classifier — Rust mirror of the `.px` owner law's
/// `classify`. Returns the same verdict the law would emit for the
/// same request.
///
/// OWNER-LAW (2026-05-11): this function MUST stay in lockstep with
/// `stdlib/lib/gate/code-transform/rename-symbol.px`. If the law adds
/// or removes a held_kind, the enum here updates. A future CI guard
/// will diff the two surfaces.
pub fn classify_rename(req: &RenameRequest) -> RenameVerdict {
  // Order matches the .px owner-law's `if .. else if ..` ladder so
  // the same input produces the same verdict.
  if req.old_name.is_empty() {
    return RenameVerdict::RenameHeld {
      held_kind: RenameHeldKind::MissingOldName,
      reason: "rename-symbol requires a non-empty `old_name`".to_string(),
    };
  }
  if req.new_name.is_empty() {
    return RenameVerdict::RenameHeld {
      held_kind: RenameHeldKind::MissingNewName,
      reason: "rename-symbol requires a non-empty `new_name`".to_string(),
    };
  }
  if req.old_name == req.new_name {
    return RenameVerdict::RenameRejected {
      held_kind: RenameHeldKind::OldNameEqualsNewName,
      reason: "old_name == new_name — nothing to do".to_string(),
    };
  }
  if !is_valid_identifier(&req.old_name) || !is_valid_identifier(&req.new_name) {
    return RenameVerdict::RenameHeld {
      held_kind: RenameHeldKind::InvalidIdentifier,
      reason: "old_name or new_name is not a valid ASCII identifier (host CST parser may still accept unicode; resubmit with `--allow-unicode-identifiers`)".to_string(),
    };
  }
  if !is_supported_language(&req.language) {
    return RenameVerdict::RenameHeld {
      held_kind: RenameHeldKind::LanguageNotSupported,
      reason: format!(
        "rename-symbol owner law currently supports rust|python|typescript|javascript|go; got `{}`",
        req.language
      ),
    };
  }
  if matches!(req.scope, RenameScope::WorkspaceWide) {
    return RenameVerdict::RenameHeld {
      held_kind: RenameHeldKind::ScopeTooBroad,
      reason: "scope=workspace-wide requires explicit owner approval (Held by default for safety)"
        .to_string(),
    };
  }
  if req.target_paths.is_empty() {
    return RenameVerdict::RenameHeld {
      held_kind: RenameHeldKind::TargetPathEmpty,
      reason: "target_paths must contain at least one path".to_string(),
    };
  }
  if !req.target_paths.iter().all(|p| is_path_in_project(p)) {
    return RenameVerdict::RenameHeld {
      held_kind: RenameHeldKind::TargetPathOutOfProject,
      reason:
        "every target path must be within the project root and must not contain `..` or null bytes"
          .to_string(),
    };
  }
  RenameVerdict::RenameReady
}

/// Compute whole-word identifier rename edits within a single file's
/// content. Each edit is a `(byte_offset, byte_len, line, column)`
/// tuple naming a position where `old_name` should be replaced with
/// `new_name`.
///
/// "Whole-word" means: the byte immediately before the match (or no
/// preceding byte) is not `[A-Za-z0-9_]`, and the byte immediately
/// after the match (or end of input) is not `[A-Za-z0-9_]`. This
/// prevents `foo` from matching inside `foobar` or `bar_foo_baz`.
///
/// OWNER-LAW (2026-05-11): this is the *token-level* layer. String
/// literals and comments are NOT skipped — a future CST upgrade will
/// add that awareness, and at that point this function will either
/// be replaced or marked legacy. For now the operator review at the
/// `ToolActionApproval` gate is the safety net.
pub fn compute_file_rename_edits(old_name: &str, new_name: &str, content: &str) -> Vec<RenameEdit> {
  let mut edits = Vec::new();
  if old_name.is_empty() {
    return edits;
  }
  let needle = old_name.as_bytes();
  let haystack = content.as_bytes();
  let nlen = needle.len();
  if nlen == 0 || haystack.len() < nlen {
    return edits;
  }

  // Walk the content, computing 1-indexed (line, column) on the fly.
  let mut line: usize = 1;
  let mut col: usize = 1;
  let mut i: usize = 0;
  while i + nlen <= haystack.len() {
    let byte = haystack[i];
    // Check word-boundary on both sides of a potential match.
    let prev_is_word = i > 0 && is_identifier_byte(haystack[i - 1]);
    let next_idx = i + nlen;
    let next_is_word = next_idx < haystack.len() && is_identifier_byte(haystack[next_idx]);
    if !prev_is_word && !next_is_word && &haystack[i..next_idx] == needle {
      edits.push(RenameEdit {
        byte_offset: i,
        byte_len: nlen,
        line,
        column: col,
      });
      // Advance past the match and recompute line/col below.
      for _ in 0..nlen {
        if haystack[i] == b'\n' {
          line += 1;
          col = 1;
        } else {
          col += 1;
        }
        i += 1;
      }
      let _ = new_name; // referenced only at apply-time
      continue;
    }
    // Advance one byte and update line/col.
    if byte == b'\n' {
      line += 1;
      col = 1;
    } else {
      col += 1;
    }
    i += 1;
  }
  edits
}

fn is_identifier_byte(b: u8) -> bool {
  b.is_ascii_alphanumeric() || b == b'_'
}

// ─── Rust skip-zone lexer (CST-lite for rename safety) ────────────────

/// A byte range in Rust source where identifier rewrites are NOT safe
/// because the bytes are inside a string literal, char literal,
/// lifetime, or comment.
///
/// OWNER-LAW (2026-05-11): exclusive `end`. Sorted, non-overlapping.
/// Computed by [`rust_skip_zones`] over the full source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RustSkipZone {
  pub start: usize,
  pub end: usize,
  pub kind: RustSkipZoneKind,
}

/// Why this range is a skip zone. Used by the cockpit panel to
/// explain which edits got filtered (and for tests).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RustSkipZoneKind {
  /// `// ... \n` or `//! ...` or `/// ...`
  LineComment,
  /// `/* ... */` (nested supported)
  BlockComment,
  /// `"..."` regular string literal (with backslash escapes)
  String,
  /// `r"..."` / `r#"..."#` / `r##"..."##` raw string literal
  RawString,
  /// `b"..."` byte string literal
  ByteString,
  /// `br"..."` / `br#"..."#` raw byte string literal
  RawByteString,
  /// `'a'` / `'\\n'` / `'\\u{1234}'` char literal
  Char,
  /// `b'a'` byte char literal
  ByteChar,
  /// `'foo` lifetime marker
  Lifetime,
}

/// Compute the skip zones in Rust source. Identifier rewrites whose
/// byte_offset falls inside any of these ranges are NOT safe to
/// apply (the match is inside a string/comment/lifetime/etc.).
///
/// OWNER-LAW (2026-05-11): pure function. No I/O. Hand-rolled
/// state-machine lexer covering the cases that matter for the
/// rename safety filter:
///
///   - line comments: `//` to end of line (including `///` doc and
///     `//!` inner-doc forms)
///   - block comments: `/* ... */` with proper nesting (a single
///     `/* /* */` would NOT close the outer comment)
///   - regular strings: `"..."` with `\` escape handling
///   - raw strings: `r"..."`, `r#"..."#`, ..., `r##...##"..."##...##`
///   - byte strings: `b"..."`, `br"..."`
///   - char literals: `'a'`, `'\\n'`, `'\\u{1234}'`
///   - byte char: `b'a'`
///   - lifetimes: `'foo` (single quote followed by identifier chars,
///     NOT followed by a closing quote)
///
/// Does NOT yet cover: macro-argument bodies (`foo!(bar)` — the `bar`
/// might be a code identifier or a string-like macro argument; without
/// macro expansion we can't tell, so phase-1 conservatism is to skip
/// macro **call sites** via a separate filter, not the args).
pub fn rust_skip_zones(source: &str) -> Vec<RustSkipZone> {
  let bytes = source.as_bytes();
  let mut zones: Vec<RustSkipZone> = Vec::new();
  let mut i = 0usize;
  while i < bytes.len() {
    let b = bytes[i];
    // Comments take precedence over everything else.
    if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
      let start = i;
      while i < bytes.len() && bytes[i] != b'\n' {
        i += 1;
      }
      zones.push(RustSkipZone {
        start,
        end: i,
        kind: RustSkipZoneKind::LineComment,
      });
      continue;
    }
    if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
      let start = i;
      i += 2;
      let mut depth = 1usize;
      while i < bytes.len() && depth > 0 {
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
          depth += 1;
          i += 2;
        } else if i + 1 < bytes.len() && bytes[i] == b'*' && bytes[i + 1] == b'/' {
          depth -= 1;
          i += 2;
        } else {
          i += 1;
        }
      }
      zones.push(RustSkipZone {
        start,
        end: i,
        kind: RustSkipZoneKind::BlockComment,
      });
      continue;
    }
    // String / raw-string / byte-string prefixes. `r`, `b`, `br` are
    // only string-introducers when they appear at an identifier-token
    // boundary; otherwise they're part of an identifier like `bar`.
    let preceded_by_ident_char = i > 0 && is_identifier_byte(bytes[i - 1]);
    if !preceded_by_ident_char {
      // Try `br"...`, `br#"..."#`, etc.
      if b == b'b' && i + 1 < bytes.len() && bytes[i + 1] == b'r' {
        let after_br = i + 2;
        if after_br < bytes.len() && (bytes[after_br] == b'"' || bytes[after_br] == b'#') {
          let start = i;
          i += 2;
          let end = consume_raw_string_body(bytes, &mut i);
          if end {
            zones.push(RustSkipZone {
              start,
              end: i,
              kind: RustSkipZoneKind::RawByteString,
            });
            continue;
          }
        }
      }
      // Try `r"...`, `r#"..."#`
      if b == b'r' && i + 1 < bytes.len() && (bytes[i + 1] == b'"' || bytes[i + 1] == b'#') {
        let start = i;
        i += 1;
        let end = consume_raw_string_body(bytes, &mut i);
        if end {
          zones.push(RustSkipZone {
            start,
            end: i,
            kind: RustSkipZoneKind::RawString,
          });
          continue;
        }
      }
      // Try `b"..."`
      if b == b'b' && i + 1 < bytes.len() && bytes[i + 1] == b'"' {
        let start = i;
        i += 2;
        consume_normal_string_body(bytes, &mut i);
        zones.push(RustSkipZone {
          start,
          end: i,
          kind: RustSkipZoneKind::ByteString,
        });
        continue;
      }
      // Try `b'..'`
      if b == b'b' && i + 1 < bytes.len() && bytes[i + 1] == b'\'' {
        let start = i;
        i += 2;
        let was_char = consume_char_or_lifetime_body(bytes, &mut i);
        zones.push(RustSkipZone {
          start,
          end: i,
          kind: if was_char {
            RustSkipZoneKind::ByteChar
          } else {
            // `b'foo` would be a syntax error (no byte-lifetime),
            // but classify as ByteChar for our purposes since it's
            // clearly an attempted byte literal.
            RustSkipZoneKind::ByteChar
          },
        });
        continue;
      }
    }
    // Regular string `"..."`.
    if b == b'"' {
      let start = i;
      i += 1;
      consume_normal_string_body(bytes, &mut i);
      zones.push(RustSkipZone {
        start,
        end: i,
        kind: RustSkipZoneKind::String,
      });
      continue;
    }
    // Char literal or lifetime `'...`.
    if b == b'\'' {
      let start = i;
      i += 1;
      let was_char = consume_char_or_lifetime_body(bytes, &mut i);
      zones.push(RustSkipZone {
        start,
        end: i,
        kind: if was_char {
          RustSkipZoneKind::Char
        } else {
          RustSkipZoneKind::Lifetime
        },
      });
      continue;
    }
    i += 1;
  }
  zones
}

/// Consume the body of a regular `"..."` string with `\` escape
/// handling. Cursor is on the first byte AFTER the opening `"`.
/// Advances the cursor past the closing `"`.
fn consume_normal_string_body(bytes: &[u8], i: &mut usize) {
  while *i < bytes.len() {
    match bytes[*i] {
      b'\\' => {
        // Backslash escape: consume the next byte too. Don't worry
        // about \u{...} multi-byte forms — for skip-zone purposes
        // the conservative skip past 2 bytes is fine since the
        // intervening bytes are also "inside the string".
        *i += 2;
      }
      b'"' => {
        *i += 1;
        return;
      }
      _ => {
        *i += 1;
      }
    }
  }
}

/// Consume the body of a `r#..."..."#..` raw string. Cursor is on the
/// `"` or first `#` of the raw-string opener (after the `r` or `br`
/// prefix has been consumed). Returns `true` if a closing sequence
/// was found, `false` if the source ran out.
fn consume_raw_string_body(bytes: &[u8], i: &mut usize) -> bool {
  let mut hash_count = 0usize;
  while *i < bytes.len() && bytes[*i] == b'#' {
    hash_count += 1;
    *i += 1;
  }
  if *i >= bytes.len() || bytes[*i] != b'"' {
    return false;
  }
  *i += 1; // past opening "
  while *i < bytes.len() {
    if bytes[*i] == b'"' {
      // Check for matching number of `#` after the closing `"`.
      let mut closing_hashes = 0usize;
      let probe_start = *i + 1;
      let mut probe = probe_start;
      while probe < bytes.len() && closing_hashes < hash_count && bytes[probe] == b'#' {
        closing_hashes += 1;
        probe += 1;
      }
      if closing_hashes == hash_count {
        *i = probe;
        return true;
      }
      *i += 1;
    } else {
      *i += 1;
    }
  }
  false
}

/// Consume the body of a `'...` literal. Returns `true` if it was a
/// char literal (closed with `'`), `false` if it was a lifetime
/// (just identifier chars without closing `'`).
fn consume_char_or_lifetime_body(bytes: &[u8], i: &mut usize) -> bool {
  if *i >= bytes.len() {
    return false;
  }
  // Escape: `\X` or `\xNN` or `\u{...}`.
  if bytes[*i] == b'\\' {
    *i += 1;
    if *i < bytes.len() {
      let esc = bytes[*i];
      *i += 1;
      if esc == b'u' && *i < bytes.len() && bytes[*i] == b'{' {
        while *i < bytes.len() && bytes[*i] != b'}' {
          *i += 1;
        }
        if *i < bytes.len() {
          *i += 1;
        }
      }
    }
    // Expect closing `'`.
    if *i < bytes.len() && bytes[*i] == b'\'' {
      *i += 1;
      return true;
    }
    return true; // best-effort — treat as char-like
  }
  // Walk identifier chars. If followed by `'`, it's a char literal;
  // otherwise it's a lifetime.
  let start_inner = *i;
  while *i < bytes.len() && is_identifier_byte(bytes[*i]) {
    *i += 1;
  }
  let inner_len = *i - start_inner;
  if inner_len == 0 {
    // Empty `''` — invalid Rust but be lenient. Consume single closing `'`.
    if *i < bytes.len() && bytes[*i] == b'\'' {
      *i += 1;
      return true;
    }
    return false;
  }
  // Char literal iff the next byte is `'`. (Rust char literals contain
  // exactly one code point, so `'aa'` is invalid — but for
  // skip-zone purposes `'foo` is a lifetime regardless of length.)
  if *i < bytes.len() && bytes[*i] == b'\'' && inner_len == 1 {
    *i += 1;
    true
  } else {
    // Lifetime — zone ends at the last ident char.
    false
  }
}

/// Find byte offsets of identifiers that are macro call names (i.e.
/// the identifier is followed immediately by `!`). For example, in
/// `println!("x")` the `println` token at offset N is a macro call
/// and should be skipped when renaming a non-macro identifier of the
/// same name.
///
/// Returns byte_offset of each macro-call identifier start (zero-or-
/// more bytes of the identifier, terminated by `!`).
pub fn rust_macro_call_identifier_offsets(source: &str, identifier: &str) -> Vec<usize> {
  if identifier.is_empty() {
    return Vec::new();
  }
  let bytes = source.as_bytes();
  let needle = identifier.as_bytes();
  let nlen = needle.len();
  let mut offsets = Vec::new();
  if nlen == 0 || bytes.len() < nlen + 1 {
    return offsets;
  }
  let mut i = 0;
  while i + nlen + 1 <= bytes.len() {
    if &bytes[i..i + nlen] == needle {
      let prev_is_word = i > 0 && is_identifier_byte(bytes[i - 1]);
      let next_byte = bytes[i + nlen];
      if !prev_is_word && next_byte == b'!' {
        offsets.push(i);
      }
    }
    i += 1;
  }
  offsets
}

/// Filter rename edits to only those NOT inside a Rust skip zone AND
/// NOT macro call names. Used by [`compute_file_rename_edits_rust_safe`].
///
/// OWNER-LAW (2026-05-11): pure function. Caller supplies the
/// pre-computed skip zones; this function does the linear filter.
pub fn filter_rename_edits_outside_rust_skip_zones(
  edits: Vec<RenameEdit>,
  skip_zones: &[RustSkipZone],
  macro_call_offsets: &[usize],
) -> Vec<RenameEdit> {
  edits
    .into_iter()
    .filter(|e| {
      // Inside a skip zone?
      let in_skip = skip_zones
        .iter()
        .any(|z| e.byte_offset >= z.start && e.byte_offset < z.end);
      if in_skip {
        return false;
      }
      // Macro call name?
      if macro_call_offsets.contains(&e.byte_offset) {
        return false;
      }
      true
    })
    .collect()
}

/// Rust-safe rename-edit detection. Same as
/// [`compute_file_rename_edits`] but additionally filters out edits
/// whose byte_offset is inside a string literal, char literal,
/// lifetime, or comment, AND filters out macro call names.
///
/// OWNER-LAW (2026-05-11): use this for `language == "rust"` requests.
/// Other languages should keep calling the token-based
/// `compute_file_rename_edits` until per-language skip-zone lexers
/// are added (Python, TypeScript, etc. — future slices).
pub fn compute_file_rename_edits_rust_safe(
  old_name: &str,
  new_name: &str,
  content: &str,
) -> Vec<RenameEdit> {
  let raw_edits = compute_file_rename_edits(old_name, new_name, content);
  let skip_zones = rust_skip_zones(content);
  let macro_offsets = rust_macro_call_identifier_offsets(content, old_name);
  filter_rename_edits_outside_rust_skip_zones(raw_edits, &skip_zones, &macro_offsets)
}

// ─── scope-aware Rust rename (D-22) ─────────────────────────────
//
// First step toward real scope-aware refactoring. v0 detects
// top-level `fn NAME(...) { ... }` bodies via brace-balanced scan
// (skip-zone aware so braces inside strings/comments don't count).
// A rename caller can scope-restrict edits to a single function's
// body. Other functions with the same identifier are NOT touched.
//
// v0 LIMITATIONS (documented; future slices refine):
//   - Same-name shadowing inside the target function (e.g. `let
//     foo = 1` inside `fn foo()`) is NOT detected — all `foo`
//     occurrences inside the target body are renamed.
//   - Nested `fn` declarations inside the target body inherit the
//     outer body's range and are NOT analyzed recursively.
//   - `impl` blocks, `mod` blocks, `trait` blocks, closures are NOT
//     recognized as scopes; only top-level + nested `fn`.
//   - tree-sitter / `syn` CST upgrade is the right long-term path;
//     this v0 is the foundation that the dispatcher / cockpit
//     wires through without committing to a parser dependency.

/// A detected `fn NAME(...) { ... }` scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustFunctionScope {
  pub name: String,
  /// Byte offset of the `fn` keyword's `f`.
  pub fn_keyword_start: usize,
  /// Byte offset of the function name's first byte.
  pub name_start: usize,
  /// Byte offset just past the function name.
  pub name_end: usize,
  /// Byte offset of the body's opening `{`.
  pub body_open_brace: usize,
  /// Byte offset just past the body's closing `}`.
  pub body_close_brace_exclusive: usize,
}

/// Locate every `fn NAME(...) { ... }` declaration in Rust source.
/// Skip-zone aware — `fn` inside a `// comment` or `"string"` does
/// NOT count.
pub fn find_rust_function_scopes(source: &str) -> Vec<RustFunctionScope> {
  let bytes = source.as_bytes();
  let zones = rust_skip_zones(source);
  let mut out: Vec<RustFunctionScope> = Vec::new();
  let mut i = 0usize;
  while i < bytes.len() {
    if in_rust_skip_zone(&zones, i) {
      i = rust_skip_zone_end(&zones, i).unwrap_or(i + 1);
      continue;
    }
    // Look for `fn` keyword at a token boundary.
    if i + 2 <= bytes.len() && &bytes[i..i + 2] == b"fn" {
      let preceded_by_ident = i > 0 && is_identifier_byte(bytes[i - 1]);
      let followed_by_ident_or_ok = i + 2 < bytes.len() && !is_identifier_byte(bytes[i + 2]);
      if !preceded_by_ident && followed_by_ident_or_ok {
        let fn_kw_start = i;
        // Skip whitespace after `fn`.
        let mut j = i + 2;
        while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\n') {
          j += 1;
        }
        // Function name = identifier.
        let name_start = j;
        while j < bytes.len() && is_identifier_byte(bytes[j]) {
          j += 1;
        }
        let name_end = j;
        if name_start == name_end {
          // No name — `fn` keyword in some odd position. Skip.
          i = fn_kw_start + 2;
          continue;
        }
        // Skip whitespace, generics `<...>`, args `(...)`, return
        // type `-> ...`, where clause. We approximate by scanning
        // forward until the first `{` at the same brace-depth-0
        // level, ignoring zones. Generics with `<` and `>` are not
        // separately tracked — v0 conservative.
        let mut k = name_end;
        let mut found_body = false;
        while k < bytes.len() {
          if in_rust_skip_zone(&zones, k) {
            k = rust_skip_zone_end(&zones, k).unwrap_or(k + 1);
            continue;
          }
          match bytes[k] {
            b'{' => {
              found_body = true;
              break;
            }
            // Declaration ends without a body: `fn method(self);`
            // (trait method) or `fn foo() where ...,` outer-level
            // comma/close-brace etc. Without an explicit body, this
            // is NOT a scope — abort before consuming an unrelated
            // `{` further down the source.
            b';' | b'}' => break,
            _ => {}
          }
          k += 1;
        }
        if !found_body {
          i = name_end;
          continue;
        }
        let body_open = k;
        let body_close = match find_rust_matching_close_brace(bytes, &zones, body_open) {
          Some(c) => c,
          None => {
            i = name_end;
            continue;
          }
        };
        out.push(RustFunctionScope {
          name: std::str::from_utf8(&bytes[name_start..name_end])
            .unwrap_or("")
            .to_string(),
          fn_keyword_start: fn_kw_start,
          name_start,
          name_end,
          body_open_brace: body_open,
          body_close_brace_exclusive: body_close + 1,
        });
        i = body_close + 1;
        continue;
      }
    }
    i += 1;
  }
  out
}

fn in_rust_skip_zone(zones: &[RustSkipZone], byte: usize) -> bool {
  zones.iter().any(|z| byte >= z.start && byte < z.end)
}

fn rust_skip_zone_end(zones: &[RustSkipZone], byte: usize) -> Option<usize> {
  zones
    .iter()
    .find(|z| byte >= z.start && byte < z.end)
    .map(|z| z.end)
}

/// Brace-balanced match honoring rust skip zones. `open_pos` must
/// be a `{`.
fn find_rust_matching_close_brace(
  bytes: &[u8],
  zones: &[RustSkipZone],
  open_pos: usize,
) -> Option<usize> {
  if open_pos >= bytes.len() || bytes[open_pos] != b'{' {
    return None;
  }
  let mut depth = 1i32;
  let mut i = open_pos + 1;
  while i < bytes.len() {
    if in_rust_skip_zone(zones, i) {
      i = rust_skip_zone_end(zones, i).unwrap_or(i + 1);
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

/// Errors raised by [`compute_file_rename_edits_rust_scope_aware`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RustScopeAwareError {
  /// No `fn <target_fn_name>(...) { ... }` declaration found.
  TargetFunctionNotFound,
  /// Multiple `fn <target_fn_name>` declarations. Caller must
  /// disambiguate (future: by byte offset / impl block path).
  MultipleTargetFunctions { count: usize },
}

/// Compute Rust rename edits restricted to a single named function's
/// body. Edits outside the target function (other fn bodies,
/// module-level declarations, etc.) are dropped.
///
/// OWNER-LAW (2026-05-13): the first scope-aware boundary. The
/// target_fn_name's own declaration token is NOT renamed (the fn
/// declaration belongs to the *outer* scope); only occurrences
/// inside the body. This matches the semantic of "rename a local
/// variable inside this function" — the most common
/// scope-restricted rename request.
///
/// v0 limitation: shadowing inside the body is not analyzed; every
/// `old_name` in the body is renamed. The future CST-aware variant
/// will trace let-binding shadowing.
pub fn compute_file_rename_edits_rust_scope_aware(
  old_name: &str,
  new_name: &str,
  content: &str,
  target_fn_name: &str,
) -> Result<Vec<RenameEdit>, RustScopeAwareError> {
  let scopes = find_rust_function_scopes(content);
  let matching: Vec<&RustFunctionScope> =
    scopes.iter().filter(|s| s.name == target_fn_name).collect();
  match matching.len() {
    0 => Err(RustScopeAwareError::TargetFunctionNotFound),
    1 => {
      let scope = matching[0];
      // All Rust-safe edits, then keep only those whose byte_offset
      // falls inside the body (between `{` exclusive and `}` exclusive).
      let raw = compute_file_rename_edits_rust_safe(old_name, new_name, content);
      let body_start = scope.body_open_brace + 1;
      let body_end = scope.body_close_brace_exclusive.saturating_sub(1);
      let filtered: Vec<RenameEdit> = raw
        .into_iter()
        .filter(|e| e.byte_offset >= body_start && e.byte_offset < body_end)
        .collect();
      Ok(filtered)
    }
    n => Err(RustScopeAwareError::MultipleTargetFunctions { count: n }),
  }
}

// ─── multi-language scope-aware rename (D-24) ─────────────────
//
// v0 brace-balanced function-scope detectors for TS / JS / Go.
// Python is NOT yet covered — indent-based blocks need a different
// algorithm and land in a later slice. The common shape generic
// helper (`compute_file_rename_edits_brace_scope_aware`) lets the
// language-specific walker focus only on "where is the
// `<keyword> NAME (...) {` declaration" and feed body byte ranges
// into the rust-style filter.

/// Generic function scope shape — same fields as
/// `RustFunctionScope` minus rust-specific terminology. Used by
/// TS/JS/Go walkers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BraceLangFunctionScope {
  pub name: String,
  pub keyword_start: usize,
  pub name_start: usize,
  pub name_end: usize,
  pub body_open_brace: usize,
  pub body_close_brace_exclusive: usize,
}

/// Generic skip-zone helpers (re-shape for `SkipZone`).
fn in_skip_zone(zones: &[SkipZone], byte: usize) -> bool {
  zones.iter().any(|z| byte >= z.start && byte < z.end)
}

fn skip_zone_end(zones: &[SkipZone], byte: usize) -> Option<usize> {
  zones
    .iter()
    .find(|z| byte >= z.start && byte < z.end)
    .map(|z| z.end)
}

fn find_matching_close_brace_generic(
  bytes: &[u8],
  zones: &[SkipZone],
  open_pos: usize,
) -> Option<usize> {
  if open_pos >= bytes.len() || bytes[open_pos] != b'{' {
    return None;
  }
  let mut depth = 1i32;
  let mut i = open_pos + 1;
  while i < bytes.len() {
    if in_skip_zone(zones, i) {
      i = skip_zone_end(zones, i).unwrap_or(i + 1);
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

/// Walk a source string for top-level `<keyword> NAME(...) { ... }`
/// declarations. `pre_name_skip` is called BEFORE the name token —
/// e.g. for Go's `func (recv Type) NAME` form, the walker skips the
/// receiver group.
fn find_brace_function_scopes_generic(
  source: &str,
  zones: &[SkipZone],
  keyword: &[u8],
  // Returns the byte offset of where to start looking for the name
  // (after the keyword + whitespace + optional receiver parens for Go).
  pre_name_skip: impl Fn(&[u8], usize) -> usize,
) -> Vec<BraceLangFunctionScope> {
  let bytes = source.as_bytes();
  let mut out: Vec<BraceLangFunctionScope> = Vec::new();
  let mut i = 0usize;
  while i < bytes.len() {
    if in_skip_zone(zones, i) {
      i = skip_zone_end(zones, i).unwrap_or(i + 1);
      continue;
    }
    if i + keyword.len() <= bytes.len() && &bytes[i..i + keyword.len()] == keyword {
      let preceded_by_ident = i > 0 && is_identifier_byte(bytes[i - 1]);
      let after_kw = i + keyword.len();
      let followed_by_non_ident = after_kw >= bytes.len() || !is_identifier_byte(bytes[after_kw]);
      if !preceded_by_ident && followed_by_non_ident {
        let kw_start = i;
        // Skip whitespace + optional pre-name region (e.g. Go's
        // receiver `(recv Type)`).
        let mut j = pre_name_skip(bytes, after_kw);
        // Skip whitespace before name.
        while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\n') {
          j += 1;
        }
        // Name = identifier.
        let name_start = j;
        while j < bytes.len() && is_identifier_byte(bytes[j]) {
          j += 1;
        }
        let name_end = j;
        if name_start == name_end {
          i = after_kw;
          continue;
        }
        // Scan forward to `{` (or abort on `;` / `}`).
        let mut k = name_end;
        let mut found_body = false;
        while k < bytes.len() {
          if in_skip_zone(zones, k) {
            k = skip_zone_end(zones, k).unwrap_or(k + 1);
            continue;
          }
          match bytes[k] {
            b'{' => {
              found_body = true;
              break;
            }
            b';' | b'}' => break,
            _ => {}
          }
          k += 1;
        }
        if !found_body {
          i = name_end;
          continue;
        }
        let body_open = k;
        let body_close = match find_matching_close_brace_generic(bytes, zones, body_open) {
          Some(c) => c,
          None => {
            i = name_end;
            continue;
          }
        };
        out.push(BraceLangFunctionScope {
          name: std::str::from_utf8(&bytes[name_start..name_end])
            .unwrap_or("")
            .to_string(),
          keyword_start: kw_start,
          name_start,
          name_end,
          body_open_brace: body_open,
          body_close_brace_exclusive: body_close + 1,
        });
        i = body_close + 1;
        continue;
      }
    }
    i += 1;
  }
  out
}

/// Skip whitespace only — used by TS/JS where there's no receiver
/// region between keyword and name.
fn pre_name_skip_whitespace_only(_bytes: &[u8], pos: usize) -> usize {
  pos
}

/// Skip whitespace + optional `(receiver Type)` — used by Go.
fn pre_name_skip_go_receiver(bytes: &[u8], pos: usize) -> usize {
  let mut j = pos;
  // Skip whitespace.
  while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\t') {
    j += 1;
  }
  if j < bytes.len() && bytes[j] == b'(' {
    // Find matching `)`. Receivers don't contain nested parens in
    // canonical Go style, but balance anyway.
    let mut depth = 1i32;
    let mut k = j + 1;
    while k < bytes.len() && depth > 0 {
      match bytes[k] {
        b'(' => depth += 1,
        b')' => depth -= 1,
        _ => {}
      }
      k += 1;
    }
    return k;
  }
  pos
}

/// TypeScript `function NAME(...) { ... }` + arrow + class-method
/// scope walker.
pub fn find_typescript_function_scopes(source: &str) -> Vec<BraceLangFunctionScope> {
  let zones = typescript_skip_zones(source);
  let mut out =
    find_brace_function_scopes_generic(source, &zones, b"function", pre_name_skip_whitespace_only);
  out.extend(find_arrow_function_scopes_generic(source, &zones));
  out.extend(find_class_method_scopes_generic(source, &zones));
  out
}

/// JavaScript `function NAME(...) { ... }` + arrow + class-method
/// scope walker. Uses the TS skip-zone lexer (same lexical
/// conventions in v0).
pub fn find_javascript_function_scopes(source: &str) -> Vec<BraceLangFunctionScope> {
  let zones = javascript_skip_zones(source);
  let mut out =
    find_brace_function_scopes_generic(source, &zones, b"function", pre_name_skip_whitespace_only);
  out.extend(find_arrow_function_scopes_generic(source, &zones));
  out.extend(find_class_method_scopes_generic(source, &zones));
  out
}

/// D-27: TypeScript / JavaScript class method scope detector.
/// Walks `class NAME { ... }` bodies and emits each `[static]
/// [async] [get|set] NAME(...) { ... }` method as a separate
/// `BraceLangFunctionScope`.
///
/// v0 LIMITATIONS:
///   - anonymous class `class { ... }` not matched (no name).
///   - private methods `#NAME` not matched.
///   - computed names `[NAME]()` not matched.
///   - `constructor` IS matched (its name is literally "constructor").
fn find_class_method_scopes_generic(
  source: &str,
  zones: &[SkipZone],
) -> Vec<BraceLangFunctionScope> {
  let bytes = source.as_bytes();
  let mut out: Vec<BraceLangFunctionScope> = Vec::new();
  let mut i = 0usize;
  while i < bytes.len() {
    if in_skip_zone(zones, i) {
      i = skip_zone_end(zones, i).unwrap_or(i + 1);
      continue;
    }
    // Match `class ` keyword at a token boundary.
    if i + 6 <= bytes.len() && &bytes[i..i + 6] == b"class " {
      let preceded_by_ident = i > 0 && is_identifier_byte(bytes[i - 1]);
      if !preceded_by_ident {
        // Skip whitespace + name + heritage clauses, find body open `{`.
        let mut j = i + 6;
        while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\t') {
          j += 1;
        }
        // Class name.
        let class_name_start = j;
        while j < bytes.len() && is_identifier_byte(bytes[j]) {
          j += 1;
        }
        if j == class_name_start {
          // Anonymous class — v0 skips.
          i += 6;
          continue;
        }
        // Scan forward to the class body's opening `{`, ignoring
        // generics / heritage clauses / type annotations.
        let mut k = j;
        let mut found_open = false;
        while k < bytes.len() {
          if in_skip_zone(zones, k) {
            k = skip_zone_end(zones, k).unwrap_or(k + 1);
            continue;
          }
          if bytes[k] == b'{' {
            found_open = true;
            break;
          }
          k += 1;
        }
        if !found_open {
          i = j;
          continue;
        }
        let class_body_open = k;
        let class_body_close =
          match find_matching_close_brace_generic(bytes, zones, class_body_open) {
            Some(c) => c,
            None => {
              i = class_body_open + 1;
              continue;
            }
          };
        // Walk class body interior, depth-1 only.
        out.extend(find_class_body_methods(
          bytes,
          zones,
          class_body_open + 1,
          class_body_close,
        ));
        i = class_body_close + 1;
        continue;
      }
    }
    i += 1;
  }
  out
}

/// Walk a class body interior (between the opening `{`'s +1 and the
/// matching `}` exclusive) and emit method scope entries. Tracks
/// `{ }` depth so methods inside nested objects / function bodies
/// don't get mistaken for class-direct methods.
fn find_class_body_methods(
  bytes: &[u8],
  zones: &[SkipZone],
  body_start: usize,
  body_end_exclusive: usize,
) -> Vec<BraceLangFunctionScope> {
  let mut out: Vec<BraceLangFunctionScope> = Vec::new();
  let mut i = body_start;
  let mut depth = 0i32;
  while i < body_end_exclusive {
    if in_skip_zone(zones, i) {
      i = skip_zone_end(zones, i).unwrap_or(i + 1);
      continue;
    }
    // Track depth so nested `{}` inside method bodies don't
    // surface methods.
    if bytes[i] == b'{' {
      depth += 1;
      i += 1;
      continue;
    }
    if bytes[i] == b'}' {
      if depth > 0 {
        depth -= 1;
      }
      i += 1;
      continue;
    }
    if depth != 0 {
      i += 1;
      continue;
    }
    // At depth-0 of the class body. Try to match a method
    // declaration: optional modifiers, identifier, `(`, ...
    let start_of_decl = i;
    let mut j = i;
    // Skip whitespace and newlines.
    while j < body_end_exclusive && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\n') {
      j += 1;
    }
    if j >= body_end_exclusive {
      break;
    }
    // Modifiers — repeat-consumed in any order: static / async /
    // get / set. Each ends at a non-identifier-char (whitespace).
    loop {
      let modifier_start = j;
      let mut mj = modifier_start;
      while mj < body_end_exclusive && is_identifier_byte(bytes[mj]) {
        mj += 1;
      }
      let word = &bytes[modifier_start..mj];
      let is_modifier = matches!(word, b"static" | b"async" | b"get" | b"set");
      if !is_modifier {
        break;
      }
      // Must be followed by whitespace.
      if mj >= body_end_exclusive
        || !(bytes[mj] == b' ' || bytes[mj] == b'\t' || bytes[mj] == b'\n')
      {
        break;
      }
      j = mj;
      while j < body_end_exclusive && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\n') {
        j += 1;
      }
    }
    // Now try identifier (method name).
    let name_start = j;
    while j < body_end_exclusive && is_identifier_byte(bytes[j]) {
      j += 1;
    }
    let name_end = j;
    if name_start == name_end {
      // Not a method declaration at this position. Advance and
      // continue depth-walk.
      i = if i + 1 > start_of_decl + 1 {
        i + 1
      } else {
        start_of_decl + 1
      };
      continue;
    }
    // Skip whitespace, optional generics `<...>`.
    while j < body_end_exclusive && (bytes[j] == b' ' || bytes[j] == b'\t') {
      j += 1;
    }
    if j < body_end_exclusive && bytes[j] == b'<' {
      let mut g = 1i32;
      j += 1;
      while j < body_end_exclusive && g > 0 {
        if in_skip_zone(zones, j) {
          j = skip_zone_end(zones, j).unwrap_or(j + 1);
          continue;
        }
        match bytes[j] {
          b'<' => g += 1,
          b'>' => g -= 1,
          _ => {}
        }
        j += 1;
      }
    }
    while j < body_end_exclusive && (bytes[j] == b' ' || bytes[j] == b'\t') {
      j += 1;
    }
    // Expect `(`.
    if j >= body_end_exclusive || bytes[j] != b'(' {
      // Not a method (could be a property assignment). Advance.
      i = name_end;
      continue;
    }
    // Paren-balanced skip to `)`.
    let mut p = 1i32;
    let mut q = j + 1;
    while q < body_end_exclusive && p > 0 {
      if in_skip_zone(zones, q) {
        q = skip_zone_end(zones, q).unwrap_or(q + 1);
        continue;
      }
      match bytes[q] {
        b'(' => p += 1,
        b')' => p -= 1,
        _ => {}
      }
      q += 1;
    }
    if p != 0 {
      i = name_end;
      continue;
    }
    // Skip whitespace + optional return type / generic.
    while q < body_end_exclusive && (bytes[q] == b' ' || bytes[q] == b'\t' || bytes[q] == b'\n') {
      q += 1;
    }
    if q < body_end_exclusive && bytes[q] == b':' {
      // Skip return type up to `{`.
      while q < body_end_exclusive {
        if in_skip_zone(zones, q) {
          q = skip_zone_end(zones, q).unwrap_or(q + 1);
          continue;
        }
        if bytes[q] == b'{' {
          break;
        }
        q += 1;
      }
    }
    if q >= body_end_exclusive || bytes[q] != b'{' {
      i = name_end;
      continue;
    }
    let method_body_open = q;
    let method_body_close = match find_matching_close_brace_generic(bytes, zones, method_body_open)
    {
      Some(c) => c,
      None => {
        i = method_body_open + 1;
        continue;
      }
    };
    out.push(BraceLangFunctionScope {
      name: std::str::from_utf8(&bytes[name_start..name_end])
        .unwrap_or("")
        .to_string(),
      keyword_start: start_of_decl,
      name_start,
      name_end,
      body_open_brace: method_body_open,
      body_close_brace_exclusive: method_body_close + 1,
    });
    i = method_body_close + 1;
  }
  out
}

/// D-26: arrow function detector — `const|let|var NAME = (...) => { ... }`
/// with optional `async` modifier before the arrow's `(`.
/// Expression-body arrows (`=> expr` without `{`) are skipped —
/// they don't have a brace-delimited body to scope-rename inside.
fn find_arrow_function_scopes_generic(
  source: &str,
  zones: &[SkipZone],
) -> Vec<BraceLangFunctionScope> {
  let bytes = source.as_bytes();
  let mut out: Vec<BraceLangFunctionScope> = Vec::new();
  let mut i = 0usize;
  while i < bytes.len() {
    if in_skip_zone(zones, i) {
      i = skip_zone_end(zones, i).unwrap_or(i + 1);
      continue;
    }
    // Try the three declaration keywords at a token boundary.
    let kw_match: Option<usize> = [b"const" as &[u8], b"let", b"var"].iter().find_map(|kw| {
      let len = kw.len();
      if i + len > bytes.len() {
        return None;
      }
      if &bytes[i..i + len] != *kw {
        return None;
      }
      let preceded_by_ident = i > 0 && is_identifier_byte(bytes[i - 1]);
      let after = i + len;
      let followed_by_non_ident = after >= bytes.len() || !is_identifier_byte(bytes[after]);
      if preceded_by_ident || !followed_by_non_ident {
        return None;
      }
      Some(after)
    });
    let after_kw = match kw_match {
      Some(p) => p,
      None => {
        i += 1;
        continue;
      }
    };

    let kw_start = i;
    let mut j = after_kw;
    // Skip whitespace.
    while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\t') {
      j += 1;
    }
    // Name.
    let name_start = j;
    while j < bytes.len() && is_identifier_byte(bytes[j]) {
      j += 1;
    }
    let name_end = j;
    if name_start == name_end {
      i = after_kw;
      continue;
    }
    // Skip whitespace, optional type annotation (`: TypeAnno`), then `=`.
    let mut k = name_end;
    while k < bytes.len() && (bytes[k] == b' ' || bytes[k] == b'\t') {
      k += 1;
    }
    // Optional TS type annotation — skip `:` and everything up to `=`
    // (at top level, ignoring zones/parens).
    if k < bytes.len() && bytes[k] == b':' {
      let mut paren = 0i32;
      let mut angle = 0i32;
      k += 1;
      while k < bytes.len() {
        if in_skip_zone(zones, k) {
          k = skip_zone_end(zones, k).unwrap_or(k + 1);
          continue;
        }
        match bytes[k] {
          b'(' => paren += 1,
          b')' => paren -= 1,
          b'<' => angle += 1,
          b'>' => angle -= 1,
          b'=' if paren == 0 && angle == 0 => break,
          _ => {}
        }
        k += 1;
      }
    }
    if k >= bytes.len() || bytes[k] != b'=' {
      i = name_end;
      continue;
    }
    // Skip `=` and whitespace.
    k += 1;
    while k < bytes.len() && (bytes[k] == b' ' || bytes[k] == b'\t' || bytes[k] == b'\n') {
      k += 1;
    }
    // Optional `async`.
    if k + 5 < bytes.len() && &bytes[k..k + 5] == b"async" {
      let after_async = k + 5;
      let preceded_ok = k == 0 || !is_identifier_byte(bytes[k - 1]);
      let followed_ok = after_async >= bytes.len() || !is_identifier_byte(bytes[after_async]);
      if preceded_ok && followed_ok {
        k = after_async;
        while k < bytes.len() && (bytes[k] == b' ' || bytes[k] == b'\t' || bytes[k] == b'\n') {
          k += 1;
        }
      }
    }
    // Expect `(`.
    if k >= bytes.len() || bytes[k] != b'(' {
      i = name_end;
      continue;
    }
    // Find matching `)`.
    let paren_open = k;
    let mut depth = 1i32;
    let mut p = paren_open + 1;
    while p < bytes.len() && depth > 0 {
      if in_skip_zone(zones, p) {
        p = skip_zone_end(zones, p).unwrap_or(p + 1);
        continue;
      }
      match bytes[p] {
        b'(' => depth += 1,
        b')' => depth -= 1,
        _ => {}
      }
      p += 1;
    }
    if depth != 0 {
      i = name_end;
      continue;
    }
    // Skip whitespace, optional return-type annotation, look for `=>`.
    let mut q = p;
    while q < bytes.len() && (bytes[q] == b' ' || bytes[q] == b'\t') {
      q += 1;
    }
    // Optional `: ReturnType` — skip until `=>`.
    if q < bytes.len() && bytes[q] == b':' {
      q += 1;
      let mut paren = 0i32;
      let mut angle = 0i32;
      while q + 1 < bytes.len() {
        if in_skip_zone(zones, q) {
          q = skip_zone_end(zones, q).unwrap_or(q + 1);
          continue;
        }
        match bytes[q] {
          b'(' => paren += 1,
          b')' => paren -= 1,
          b'<' => angle += 1,
          b'>' => angle -= 1,
          b'=' if paren == 0 && angle == 0 && bytes[q + 1] == b'>' => break,
          _ => {}
        }
        q += 1;
      }
    }
    // Expect `=>`.
    if q + 1 >= bytes.len() || bytes[q] != b'=' || bytes[q + 1] != b'>' {
      i = name_end;
      continue;
    }
    q += 2;
    while q < bytes.len() && (bytes[q] == b' ' || bytes[q] == b'\t' || bytes[q] == b'\n') {
      q += 1;
    }
    // Body must be `{` — expression-body arrows are not scope-rename
    // targets in v0.
    if q >= bytes.len() || bytes[q] != b'{' {
      i = name_end;
      continue;
    }
    let body_open = q;
    let body_close = match find_matching_close_brace_generic(bytes, zones, body_open) {
      Some(c) => c,
      None => {
        i = name_end;
        continue;
      }
    };
    out.push(BraceLangFunctionScope {
      name: std::str::from_utf8(&bytes[name_start..name_end])
        .unwrap_or("")
        .to_string(),
      keyword_start: kw_start,
      name_start,
      name_end,
      body_open_brace: body_open,
      body_close_brace_exclusive: body_close + 1,
    });
    i = body_close + 1;
  }
  out
}

/// Go `func NAME(...) { ... }` and `func (recv Type) NAME(...) { ... }`
/// scope walker.
pub fn find_go_function_scopes(source: &str) -> Vec<BraceLangFunctionScope> {
  let zones = go_skip_zones(source);
  find_brace_function_scopes_generic(source, &zones, b"func", pre_name_skip_go_receiver)
}

/// Generic helper: filter rename edits to those inside the body of a
/// named function scope detected by one of the per-language
/// walkers.
fn compute_file_rename_edits_brace_scope_aware(
  raw_edits: Vec<RenameEdit>,
  scopes: Vec<BraceLangFunctionScope>,
  target_fn_name: &str,
) -> Result<Vec<RenameEdit>, RustScopeAwareError> {
  let matching: Vec<&BraceLangFunctionScope> =
    scopes.iter().filter(|s| s.name == target_fn_name).collect();
  match matching.len() {
    0 => Err(RustScopeAwareError::TargetFunctionNotFound),
    1 => {
      let scope = matching[0];
      let body_start = scope.body_open_brace + 1;
      let body_end = scope.body_close_brace_exclusive.saturating_sub(1);
      Ok(
        raw_edits
          .into_iter()
          .filter(|e| e.byte_offset >= body_start && e.byte_offset < body_end)
          .collect(),
      )
    }
    n => Err(RustScopeAwareError::MultipleTargetFunctions { count: n }),
  }
}

/// TypeScript scope-aware rename — restricts edits to the body of
/// `function <target_fn_name>(...) { ... }`. Reuses
/// `RustScopeAwareError` shape (same error categories).
pub fn compute_file_rename_edits_typescript_scope_aware(
  old_name: &str,
  new_name: &str,
  content: &str,
  target_fn_name: &str,
) -> Result<Vec<RenameEdit>, RustScopeAwareError> {
  let raw = compute_file_rename_edits_typescript_safe(old_name, new_name, content);
  let scopes = find_typescript_function_scopes(content);
  compute_file_rename_edits_brace_scope_aware(raw, scopes, target_fn_name)
}

/// JavaScript scope-aware rename — identical conventions to TS in v0.
pub fn compute_file_rename_edits_javascript_scope_aware(
  old_name: &str,
  new_name: &str,
  content: &str,
  target_fn_name: &str,
) -> Result<Vec<RenameEdit>, RustScopeAwareError> {
  let raw = compute_file_rename_edits_javascript_safe(old_name, new_name, content);
  let scopes = find_javascript_function_scopes(content);
  compute_file_rename_edits_brace_scope_aware(raw, scopes, target_fn_name)
}

/// Go scope-aware rename — also handles `func (recv Type) NAME(...)`
/// receiver-method form via the Go-specific pre-name skip.
pub fn compute_file_rename_edits_go_scope_aware(
  old_name: &str,
  new_name: &str,
  content: &str,
  target_fn_name: &str,
) -> Result<Vec<RenameEdit>, RustScopeAwareError> {
  let raw = compute_file_rename_edits_go_safe(old_name, new_name, content);
  let scopes = find_go_function_scopes(content);
  compute_file_rename_edits_brace_scope_aware(raw, scopes, target_fn_name)
}

// ─── Python scope-aware rename (D-25) ──────────────────────────
//
// Python doesn't use braces — function bodies are delimited by
// indent. This walker detects `def NAME(...):` (and `async def`)
// at any line's start, measures the header indent, then scans
// forward for a dedent line (same-or-shallower indent that's not
// blank / comment-only). That dedent line's start byte is the
// body end (exclusive).
//
// v0 LIMITATIONS (documented):
//   - Nested `def` declarations inside the target body inherit
//     the outer body's range (not analyzed recursively).
//   - Class-method `def` inside a `class X:` block is detected as
//     a scope, but its enclosing class context is NOT recorded.
//   - Multi-line signatures (parentheses spanning newlines) are
//     handled (paren depth tracks across newlines).
//   - tabs vs spaces — v0 compares raw byte counts, so mixed-indent
//     files may behave unexpectedly. Canonical PEP8 4-space files
//     are the design target.

/// A detected `def NAME(...):` Python function scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonFunctionScope {
  pub name: String,
  /// Byte offset of the `def` (or `async`) keyword's first byte.
  pub def_keyword_start: usize,
  pub name_start: usize,
  pub name_end: usize,
  /// First byte of the body (the byte after the header line's
  /// trailing `\n`).
  pub body_start: usize,
  /// First byte past the body — the start byte of the first
  /// dedent (or EOF).
  pub body_end_exclusive: usize,
}

/// Walk Python source for `def NAME(...)` / `async def NAME(...)`
/// declarations. Skip-zone aware — `def` inside a `#` comment or
/// `"..."` string does NOT count.
pub fn find_python_function_scopes(source: &str) -> Vec<PythonFunctionScope> {
  let bytes = source.as_bytes();
  let zones = python_skip_zones(source);
  let mut out: Vec<PythonFunctionScope> = Vec::new();
  let mut i = 0usize;

  while i < bytes.len() {
    if in_skip_zone(&zones, i) {
      i = skip_zone_end(&zones, i).unwrap_or(i + 1);
      continue;
    }

    // `def` must be at the start of a line (after leading
    // whitespace). Skip any mid-line `def` occurrences.
    let is_at_line_start = i == 0 || bytes[i - 1] == b'\n';
    if !is_at_line_start {
      i += 1;
      continue;
    }

    // Measure this line's leading indent.
    let line_start = i;
    let mut indent_end = i;
    while indent_end < bytes.len() && (bytes[indent_end] == b' ' || bytes[indent_end] == b'\t') {
      indent_end += 1;
    }
    let header_indent_len = indent_end - line_start;

    // Look for `def ` or `async def ` at the indented position.
    let keyword_start = indent_end;
    let after_keyword =
      if keyword_start + 4 <= bytes.len() && &bytes[keyword_start..keyword_start + 4] == b"def " {
        keyword_start + 4
      } else if keyword_start + 10 <= bytes.len()
        && &bytes[keyword_start..keyword_start + 10] == b"async def "
      {
        keyword_start + 10
      } else {
        // Not a `def` line — skip to next line.
        let mut k = i;
        while k < bytes.len() && bytes[k] != b'\n' {
          k += 1;
        }
        i = (k + 1).min(bytes.len()).max(i + 1);
        continue;
      };

    // Skip whitespace before name.
    let mut j = after_keyword;
    while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\t') {
      j += 1;
    }
    let name_start = j;
    while j < bytes.len() && is_identifier_byte(bytes[j]) {
      j += 1;
    }
    let name_end = j;
    if name_start == name_end {
      // `def` with no name — skip line.
      let mut k = j;
      while k < bytes.len() && bytes[k] != b'\n' {
        k += 1;
      }
      i = (k + 1).min(bytes.len()).max(i + 1);
      continue;
    }

    // Find the colon that ends the signature. Multi-line signatures
    // (parens spanning newlines) are handled by tracking paren depth.
    let mut paren_depth = 0i32;
    let mut k = name_end;
    let mut found_colon: Option<usize> = None;
    while k < bytes.len() {
      if in_skip_zone(&zones, k) {
        k = skip_zone_end(&zones, k).unwrap_or(k + 1);
        continue;
      }
      match bytes[k] {
        b'(' => paren_depth += 1,
        b')' => paren_depth -= 1,
        b':' if paren_depth == 0 => {
          found_colon = Some(k);
          break;
        }
        _ => {}
      }
      k += 1;
    }
    let colon_pos = match found_colon {
      Some(p) => p,
      None => {
        // No colon — not a function signature. Skip line.
        i = name_end;
        continue;
      }
    };

    // Body starts after the header line's trailing newline.
    let mut header_eol = colon_pos + 1;
    while header_eol < bytes.len() && bytes[header_eol] != b'\n' {
      header_eol += 1;
    }
    let body_start = (header_eol + 1).min(bytes.len());

    // Scan forward for a dedent line — same-or-shallower indent
    // that's not blank / comment-only.
    let mut body_end = bytes.len();
    let mut scan = body_start;
    while scan < bytes.len() {
      let this_line_start = scan;
      let mut idx = scan;
      while idx < bytes.len() && (bytes[idx] == b' ' || bytes[idx] == b'\t') {
        idx += 1;
      }
      let this_indent = idx - this_line_start;
      let line_blank_or_comment = idx >= bytes.len() || bytes[idx] == b'\n' || bytes[idx] == b'#';
      if line_blank_or_comment {
        // Skip blank / comment line (body continuation).
        while idx < bytes.len() && bytes[idx] != b'\n' {
          idx += 1;
        }
        scan = (idx + 1).min(bytes.len());
        if idx >= bytes.len() {
          break;
        }
        continue;
      }
      if this_indent <= header_indent_len {
        body_end = this_line_start;
        break;
      }
      // Body continuation — skip to next line.
      while idx < bytes.len() && bytes[idx] != b'\n' {
        idx += 1;
      }
      scan = (idx + 1).min(bytes.len());
      if idx >= bytes.len() {
        break;
      }
    }

    out.push(PythonFunctionScope {
      name: std::str::from_utf8(&bytes[name_start..name_end])
        .unwrap_or("")
        .to_string(),
      def_keyword_start: keyword_start,
      name_start,
      name_end,
      body_start,
      body_end_exclusive: body_end,
    });
    i = body_end;
  }
  out
}

/// Python scope-aware rename — restricts edits to the body of a
/// named `def`. Same error shape as `RustScopeAwareError`.
pub fn compute_file_rename_edits_python_scope_aware(
  old_name: &str,
  new_name: &str,
  content: &str,
  target_fn_name: &str,
) -> Result<Vec<RenameEdit>, RustScopeAwareError> {
  let raw = compute_file_rename_edits_python_safe(old_name, new_name, content);
  let scopes = find_python_function_scopes(content);
  let matching: Vec<&PythonFunctionScope> =
    scopes.iter().filter(|s| s.name == target_fn_name).collect();
  match matching.len() {
    0 => Err(RustScopeAwareError::TargetFunctionNotFound),
    1 => {
      let scope = matching[0];
      Ok(
        raw
          .into_iter()
          .filter(|e| e.byte_offset >= scope.body_start && e.byte_offset < scope.body_end_exclusive)
          .collect(),
      )
    }
    n => Err(RustScopeAwareError::MultipleTargetFunctions { count: n }),
  }
}

// ─── per-language skip-zone lexers (Python / TS / JS / Go) ────────────

/// Generic skip zone for non-Rust languages. The `kind` is a
/// kebab-case string label rather than a typed enum because the
/// per-language kinds vary widely (Python `triple-quoted`, TS
/// `template-literal`, Go `raw-string`, etc.). The filter only needs
/// `(start, end)` byte ranges.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkipZone {
  pub start: usize,
  pub end: usize,
  pub kind: &'static str,
}

/// Filter rename edits to only those NOT inside any of the given
/// skip zones. Language-agnostic — the per-language safe walker
/// computes `zones` via its lexer and feeds them here.
pub fn filter_rename_edits_outside_skip_zones(
  edits: Vec<RenameEdit>,
  zones: &[SkipZone],
) -> Vec<RenameEdit> {
  edits
    .into_iter()
    .filter(|e| {
      !zones
        .iter()
        .any(|z| e.byte_offset >= z.start && e.byte_offset < z.end)
    })
    .collect()
}

/// Compute Python skip zones — `#` line comments, single/double
/// quoted strings (with backslash escapes), triple-quoted strings
/// (`"""..."""` / `'''...'''`), and prefixed string literals
/// (`r"..."`, `b"..."`, `f"..."`, plus 2-char combinations like
/// `rb"..."`, `fr"..."`).
///
/// OWNER-LAW (2026-05-11): phase-1 conservatism. F-strings are
/// treated as opaque skip zones — the `${...}` interpolated
/// expressions inside aren't parsed back into code context; a
/// future slice can refine this if false-negative rate gets too
/// high.
pub fn python_skip_zones(source: &str) -> Vec<SkipZone> {
  let bytes = source.as_bytes();
  let mut zones: Vec<SkipZone> = Vec::new();
  let mut i = 0usize;
  while i < bytes.len() {
    let b = bytes[i];
    // Line comment.
    if b == b'#' {
      let start = i;
      while i < bytes.len() && bytes[i] != b'\n' {
        i += 1;
      }
      zones.push(SkipZone {
        start,
        end: i,
        kind: "line-comment",
      });
      continue;
    }
    // String prefixes: r / b / f / R / B / F, optionally 2 chars
    // (rb, br, fr, rf, etc.) at an identifier-token boundary.
    let preceded_by_ident_char = i > 0 && is_identifier_byte(bytes[i - 1]);
    if !preceded_by_ident_char {
      // Try a 1- or 2-char prefix.
      let prefix_len = python_string_prefix_len(bytes, i);
      let quote_start = i + prefix_len;
      if quote_start < bytes.len() && (bytes[quote_start] == b'"' || bytes[quote_start] == b'\'') {
        // Triple-quoted?
        let q = bytes[quote_start];
        if quote_start + 2 < bytes.len()
          && bytes[quote_start + 1] == q
          && bytes[quote_start + 2] == q
        {
          let start = i;
          i = quote_start + 3;
          // Find closing triple quote.
          while i + 2 < bytes.len() {
            if bytes[i] == q && bytes[i + 1] == q && bytes[i + 2] == q {
              i += 3;
              break;
            }
            i += 1;
          }
          // If we ran out, set i to end.
          if i + 2 >= bytes.len()
            && !(i >= 3 && bytes[i - 3] == q && bytes[i - 2] == q && bytes[i - 1] == q)
          {
            i = bytes.len();
          }
          let kind = if prefix_len > 0 {
            "triple-prefixed-string"
          } else {
            "triple-string"
          };
          zones.push(SkipZone {
            start,
            end: i,
            kind,
          });
          continue;
        }
        // Single-line quoted string with `\` escape handling.
        let start = i;
        i = quote_start + 1;
        while i < bytes.len() {
          if bytes[i] == b'\\' {
            i += 2;
          } else if bytes[i] == q {
            i += 1;
            break;
          } else if bytes[i] == b'\n' {
            // Unterminated string at newline — Python would error
            // but be lenient.
            break;
          } else {
            i += 1;
          }
        }
        let kind = if prefix_len > 0 {
          "prefixed-string"
        } else {
          "string"
        };
        zones.push(SkipZone {
          start,
          end: i,
          kind,
        });
        continue;
      }
    }
    i += 1;
  }
  zones
}

/// Detect a Python string-literal prefix at `bytes[i..]`. Returns the
/// number of prefix bytes (0, 1, or 2). Prefix chars are
/// `r/R/b/B/f/F`; valid 2-char combinations are any pair of these
/// (Python is permissive about ordering: `rb`, `br`, `Rb`, etc.).
fn python_string_prefix_len(bytes: &[u8], i: usize) -> usize {
  fn is_py_prefix_char(b: u8) -> bool {
    matches!(b, b'r' | b'R' | b'b' | b'B' | b'f' | b'F')
  }
  if i >= bytes.len() || !is_py_prefix_char(bytes[i]) {
    return 0;
  }
  // Try 2-char prefix first (e.g., `rb"..."`).
  if i + 1 < bytes.len() && is_py_prefix_char(bytes[i + 1]) {
    let next = bytes.get(i + 2);
    if matches!(next, Some(b'"') | Some(b'\'')) {
      return 2;
    }
  }
  // 1-char prefix?
  let next = bytes.get(i + 1);
  if matches!(next, Some(b'"') | Some(b'\'')) {
    return 1;
  }
  0
}

/// Compute TypeScript / JavaScript skip zones — `//` and `/* */`
/// comments, single/double quoted strings (with `\` escapes), and
/// template literals (whole thing skipped including any `${...}`
/// expressions for phase-1 conservatism).
///
/// OWNER-LAW (2026-05-11): regex literals (`/.../flags`) are NOT
/// matched in phase 1 — disambiguating regex from division
/// requires context. False-positive risk: a regex containing the
/// identifier being renamed would get rewritten. Acceptable for
/// service-grade phase 1; phase 2 can add regex detection.
pub fn typescript_skip_zones(source: &str) -> Vec<SkipZone> {
  let bytes = source.as_bytes();
  let mut zones: Vec<SkipZone> = Vec::new();
  let mut i = 0usize;
  while i < bytes.len() {
    let b = bytes[i];
    if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
      let start = i;
      while i < bytes.len() && bytes[i] != b'\n' {
        i += 1;
      }
      zones.push(SkipZone {
        start,
        end: i,
        kind: "line-comment",
      });
      continue;
    }
    if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
      let start = i;
      i += 2;
      while i + 1 < bytes.len() {
        if bytes[i] == b'*' && bytes[i + 1] == b'/' {
          i += 2;
          break;
        }
        i += 1;
      }
      if i + 1 >= bytes.len() && !(i >= 2 && bytes[i - 2] == b'*' && bytes[i - 1] == b'/') {
        i = bytes.len();
      }
      zones.push(SkipZone {
        start,
        end: i,
        kind: "block-comment",
      });
      continue;
    }
    if b == b'"' || b == b'\'' {
      let q = b;
      let start = i;
      i += 1;
      while i < bytes.len() {
        if bytes[i] == b'\\' {
          i += 2;
        } else if bytes[i] == q {
          i += 1;
          break;
        } else if bytes[i] == b'\n' {
          break;
        } else {
          i += 1;
        }
      }
      zones.push(SkipZone {
        start,
        end: i,
        kind: "string",
      });
      continue;
    }
    if b == b'`' {
      let start = i;
      i += 1;
      // Template literal — skip until closing backtick, honoring `\`
      // escapes. `${...}` interpolations are NOT parsed back into
      // code context (phase 1).
      while i < bytes.len() {
        if bytes[i] == b'\\' {
          i += 2;
        } else if bytes[i] == b'`' {
          i += 1;
          break;
        } else {
          i += 1;
        }
      }
      zones.push(SkipZone {
        start,
        end: i,
        kind: "template-literal",
      });
      continue;
    }
    i += 1;
  }
  zones
}

/// JavaScript skip zones — same lexer as TypeScript. Kept as a
/// separate function for naming clarity at the call site.
pub fn javascript_skip_zones(source: &str) -> Vec<SkipZone> {
  typescript_skip_zones(source)
}

/// Compute Go skip zones — `//` and `/* */` comments, interpreted
/// strings `"..."` with `\` escapes, rune literals `'...'` with `\`
/// escapes, and raw strings `` `...` `` (no escapes, no
/// interpolation — backslashes are literal).
pub fn go_skip_zones(source: &str) -> Vec<SkipZone> {
  let bytes = source.as_bytes();
  let mut zones: Vec<SkipZone> = Vec::new();
  let mut i = 0usize;
  while i < bytes.len() {
    let b = bytes[i];
    if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
      let start = i;
      while i < bytes.len() && bytes[i] != b'\n' {
        i += 1;
      }
      zones.push(SkipZone {
        start,
        end: i,
        kind: "line-comment",
      });
      continue;
    }
    if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
      let start = i;
      i += 2;
      while i + 1 < bytes.len() {
        if bytes[i] == b'*' && bytes[i + 1] == b'/' {
          i += 2;
          break;
        }
        i += 1;
      }
      if i + 1 >= bytes.len() && !(i >= 2 && bytes[i - 2] == b'*' && bytes[i - 1] == b'/') {
        i = bytes.len();
      }
      zones.push(SkipZone {
        start,
        end: i,
        kind: "block-comment",
      });
      continue;
    }
    if b == b'"' {
      let start = i;
      i += 1;
      while i < bytes.len() {
        if bytes[i] == b'\\' {
          i += 2;
        } else if bytes[i] == b'"' {
          i += 1;
          break;
        } else if bytes[i] == b'\n' {
          break;
        } else {
          i += 1;
        }
      }
      zones.push(SkipZone {
        start,
        end: i,
        kind: "string",
      });
      continue;
    }
    if b == b'\'' {
      let start = i;
      i += 1;
      while i < bytes.len() {
        if bytes[i] == b'\\' {
          i += 2;
        } else if bytes[i] == b'\'' {
          i += 1;
          break;
        } else if bytes[i] == b'\n' {
          break;
        } else {
          i += 1;
        }
      }
      zones.push(SkipZone {
        start,
        end: i,
        kind: "rune",
      });
      continue;
    }
    if b == b'`' {
      let start = i;
      i += 1;
      // Raw string — closing backtick is the only terminator;
      // backslashes are literal.
      while i < bytes.len() && bytes[i] != b'`' {
        i += 1;
      }
      if i < bytes.len() {
        i += 1;
      }
      zones.push(SkipZone {
        start,
        end: i,
        kind: "raw-string",
      });
      continue;
    }
    i += 1;
  }
  zones
}

/// Compute pnix `.px` skip zones — `#` line comments, regular
/// strings `"..."` with `\` escape handling (Nix-flavored), and
/// indented strings `''...''` (multi-line). Antiquotations `${...}`
/// inside `.px` strings are not respected — the entire string,
/// including the antiquotation slot, is one skip zone. A future
/// slice can refine this; phase-1 conservatism mirrors the Python
/// f-string treatment.
///
/// OWNER-LAW (2026-05-13): pnix's own language as a host CST emit
/// target. The skip-zone shape mirrors Nix because `.px` is a
/// Nix-flavored expression language (`let X = ...; in ...`, lambda
/// `x: y`, attrset `{ ... }`). Only the literal forms relevant to
/// rename safety are recognized here:
///
///   - `#` line comment to end-of-line
///   - `"..."` regular string (backslash escapes consumed)
///   - `''...''` indented string (closing pair is the only
///     terminator; `'''` is an escape for a single `'`)
///
/// Identifier walker semantics still apply outside skip zones.
/// `.px` identifiers (in let-bindings and lambdas) follow the same
/// `[A-Za-z_][A-Za-z0-9_]*` shape the generic walker uses, so the
/// raw-edit pass works unchanged.
pub fn pnix_skip_zones(source: &str) -> Vec<SkipZone> {
  let bytes = source.as_bytes();
  let mut zones: Vec<SkipZone> = Vec::new();
  let mut i = 0usize;
  while i < bytes.len() {
    let b = bytes[i];
    // Line comment `# ... \n`.
    if b == b'#' {
      let start = i;
      while i < bytes.len() && bytes[i] != b'\n' {
        i += 1;
      }
      zones.push(SkipZone {
        start,
        end: i,
        kind: "line-comment",
      });
      continue;
    }
    // Indented string `''...''`. Must check BEFORE the single `'`
    // case so a `''` opener isn't mistaken for two empty
    // char-likes. Closing `''` is the only terminator; inside the
    // body, `'''` (three apostrophes) is the escape for a single
    // `'`, so a `''` followed by `'` is NOT the close.
    if b == b'\'' && i + 1 < bytes.len() && bytes[i + 1] == b'\'' {
      let start = i;
      i += 2;
      while i + 1 < bytes.len() {
        if bytes[i] == b'\'' && bytes[i + 1] == b'\'' {
          // Triple-apostrophe escape: the third `'` (at i+2) means
          // this `''` is a literal single `'`, not a close.
          if i + 2 < bytes.len() && bytes[i + 2] == b'\'' {
            i += 3;
            continue;
          }
          i += 2;
          break;
        }
        i += 1;
      }
      // Unterminated → consume to EOF.
      if i + 1 >= bytes.len() {
        let last_two_close = i >= 2 && bytes[i - 2] == b'\'' && bytes[i - 1] == b'\'';
        if !last_two_close {
          i = bytes.len();
        }
      }
      zones.push(SkipZone {
        start,
        end: i,
        kind: "indented-string",
      });
      continue;
    }
    // Regular string `"..."` with `\` escapes.
    if b == b'"' {
      let start = i;
      i += 1;
      while i < bytes.len() {
        if bytes[i] == b'\\' {
          // Consume the backslash + the next byte (incl. `\"`).
          i += 2;
          continue;
        }
        if bytes[i] == b'"' {
          i += 1;
          break;
        }
        i += 1;
      }
      // Unterminated string → run to EOF rather than misclassify
      // following code as in-string.
      if i > bytes.len() {
        i = bytes.len();
      }
      zones.push(SkipZone {
        start,
        end: i,
        kind: "string",
      });
      continue;
    }
    i += 1;
  }
  zones
}

/// pnix `.px`-safe rename-edit detection. Filters out edits inside
/// `#` line comments, `"..."` regular strings, and `''...''`
/// indented strings.
pub fn compute_file_rename_edits_pnix_safe(
  old_name: &str,
  new_name: &str,
  content: &str,
) -> Vec<RenameEdit> {
  let raw = compute_file_rename_edits(old_name, new_name, content);
  let zones = pnix_skip_zones(content);
  filter_rename_edits_outside_skip_zones(raw, &zones)
}

/// Python-safe rename-edit detection. Same as
/// [`compute_file_rename_edits`] but additionally filters out edits
/// inside Python strings (regular, prefixed, triple) and `#` comments.
pub fn compute_file_rename_edits_python_safe(
  old_name: &str,
  new_name: &str,
  content: &str,
) -> Vec<RenameEdit> {
  let raw = compute_file_rename_edits(old_name, new_name, content);
  let zones = python_skip_zones(content);
  filter_rename_edits_outside_skip_zones(raw, &zones)
}

/// TypeScript-safe rename-edit detection. Filters edits inside `//`
/// line comments, `/* */` block comments, `"..."` / `'...'` strings,
/// and ``` `...` ``` template literals.
pub fn compute_file_rename_edits_typescript_safe(
  old_name: &str,
  new_name: &str,
  content: &str,
) -> Vec<RenameEdit> {
  let raw = compute_file_rename_edits(old_name, new_name, content);
  let zones = typescript_skip_zones(content);
  filter_rename_edits_outside_skip_zones(raw, &zones)
}

/// JavaScript-safe rename-edit detection. Same as the TypeScript
/// variant.
pub fn compute_file_rename_edits_javascript_safe(
  old_name: &str,
  new_name: &str,
  content: &str,
) -> Vec<RenameEdit> {
  let raw = compute_file_rename_edits(old_name, new_name, content);
  let zones = javascript_skip_zones(content);
  filter_rename_edits_outside_skip_zones(raw, &zones)
}

/// Go-safe rename-edit detection. Filters edits inside `//` line
/// comments, `/* */` block comments, `"..."` interpreted strings,
/// `'...'` rune literals, and `` `...` `` raw strings.
pub fn compute_file_rename_edits_go_safe(
  old_name: &str,
  new_name: &str,
  content: &str,
) -> Vec<RenameEdit> {
  let raw = compute_file_rename_edits(old_name, new_name, content);
  let zones = go_skip_zones(content);
  filter_rename_edits_outside_skip_zones(raw, &zones)
}

/// Compute rename edits for a file, applying per-language safety
/// filtering when the language has a skip-zone lexer. Falls back to
/// the token-based walker for languages without a lexer (currently
/// none in the supported set, but reserved for future languages
/// the classifier accepts).
///
/// OWNER-LAW (2026-05-11): canonical entry point for **service-grade
/// multi-language safe rename**. Wraps the per-language walkers
/// (Rust / Python / TypeScript / JavaScript / Go).
pub fn compute_file_rename_edits_lang_safe(
  language: &str,
  old_name: &str,
  new_name: &str,
  content: &str,
) -> Vec<RenameEdit> {
  match language {
    "rust" => compute_file_rename_edits_rust_safe(old_name, new_name, content),
    "python" => compute_file_rename_edits_python_safe(old_name, new_name, content),
    "typescript" => compute_file_rename_edits_typescript_safe(old_name, new_name, content),
    "javascript" => compute_file_rename_edits_javascript_safe(old_name, new_name, content),
    "go" => compute_file_rename_edits_go_safe(old_name, new_name, content),
    "pnix" => compute_file_rename_edits_pnix_safe(old_name, new_name, content),
    _ => compute_file_rename_edits(old_name, new_name, content),
  }
}

/// One file's edits + rendered diff, produced by
/// [`compute_rename_patch_candidate`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenameFilePatch {
  pub path: String,
  pub edits: Vec<RenameEdit>,
  pub unified_diff: String,
}

/// One file's input — `(path, content)` pair. The path is recorded in
/// the patch artifact; the content is what
/// [`compute_file_rename_edits`] scans. The carrier never reads files
/// from disk; the caller is responsible for staging the
/// `(path, content)` pairs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameFileInput<'a> {
  pub path: &'a str,
  pub content: &'a str,
}

/// Result of [`compute_rename_patch_candidate`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenamePatchCandidate {
  pub request: RenameRequest,
  pub verdict: RenameVerdict,
  /// `Some(...)` only when `verdict == RenameReady`. One entry per
  /// input file *that had at least one edit*. Files with zero edits
  /// are omitted — they don't belong in the patch.
  pub file_patches: Vec<RenameFilePatch>,
  /// Concatenation of every file's `unified_diff`, in input order.
  /// Empty string when `file_patches` is empty.
  pub combined_unified_diff: String,
  /// Scope-aware rename (D-23) error string when caller passed
  /// `request.target_fn_name = Some(...)` but the scope-aware
  /// pipeline could not honor it. `None` when scope-aware was not
  /// requested or when it succeeded. The verdict stays `Ready` —
  /// the rename was *attempted*; `file_patches` is empty when this
  /// is `Some` because no scope-matching edits were emitted.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub scope_aware_error: Option<String>,
}

/// Render a unified diff for a single file's rename edits.
///
/// OWNER-LAW (2026-05-11): identifier rename never changes line
/// counts (the classifier rejects identifiers containing `\n`), so
/// each modified line is its own 1-line hunk. The diff is in
/// standard `--- a/path / +++ b/path / @@ -L,1 +L,1 @@` form with no
/// context lines (the cockpit can re-render with context if needed).
///
/// Returns an empty string when `edits` is empty (nothing to diff).
pub fn render_unified_diff_for_rename(
  path: &str,
  old_content: &str,
  edits: &[RenameEdit],
  new_name: &str,
) -> String {
  if edits.is_empty() {
    return String::new();
  }
  let new_content = apply_rename_edits(old_content, edits, new_name);
  // split_inclusive keeps the trailing `\n` on each line so re-joining
  // preserves the original bytes exactly.
  let old_lines: Vec<&str> = old_content.split_inclusive('\n').collect();
  let new_lines: Vec<&str> = new_content.split_inclusive('\n').collect();

  let mut out = String::new();
  out.push_str(&format!("--- a/{}\n", path));
  out.push_str(&format!("+++ b/{}\n", path));

  // Same line count is invariant under rename (identifier has no
  // newline). Walk in parallel and emit a hunk per differing line.
  let n = old_lines.len().min(new_lines.len());
  for i in 0..n {
    if old_lines[i] == new_lines[i] {
      continue;
    }
    let line_no = i + 1; // 1-indexed
    out.push_str(&format!("@@ -{line_no},1 +{line_no},1 @@\n"));
    write_diff_line(&mut out, '-', old_lines[i]);
    write_diff_line(&mut out, '+', new_lines[i]);
  }
  // Edge case: if line counts diverge (shouldn't for rename but
  // defense-in-depth), trail the extras as added/removed.
  if old_lines.len() != new_lines.len() {
    if old_lines.len() > n {
      for i in n..old_lines.len() {
        let line_no = i + 1;
        out.push_str(&format!("@@ -{line_no},1 +{line_no},0 @@\n"));
        write_diff_line(&mut out, '-', old_lines[i]);
      }
    } else {
      for i in n..new_lines.len() {
        let line_no = i + 1;
        out.push_str(&format!("@@ -{line_no},0 +{line_no},1 @@\n"));
        write_diff_line(&mut out, '+', new_lines[i]);
      }
    }
  }
  out
}

fn write_diff_line(out: &mut String, prefix: char, line: &str) {
  out.push(prefix);
  out.push_str(line);
  if !line.ends_with('\n') {
    // Standard git-style "no newline at end of file" marker.
    out.push_str("\n\\ No newline at end of file\n");
  }
}

/// Orchestrator: classify the request, and on `RenameReady` walk the
/// staged input files to produce per-file edit lists + unified
/// diffs. Returns a `RenamePatchCandidate` packaging everything.
///
/// OWNER-LAW (2026-05-11): pure function. `files` is the *only*
/// content source — disk I/O happens elsewhere. Files outside
/// `request.target_paths` are filtered out by the per-iteration check
/// below (matches the `.px` `scope=local-target-paths` hard
/// boundary); the carrier never produces edits for non-target paths
/// even if the caller stages them.
///
/// For stricter staged-input validation — checking that every
/// requested path is staged, that `old_name` has at least one
/// occurrence, that `new_name` would not collide, and that no target
/// path is staged twice — use
/// [`compute_rename_patch_candidate_strict`]. Both functions return
/// the same `RenamePatchCandidate` shape; only the verdict differs
/// when strict checks Hold.
///
/// Held / Rejected verdicts produce an empty `file_patches` — the
/// classifier verdict is the authoritative outcome, edits are not
/// computed.
pub fn compute_rename_patch_candidate(
  request: &RenameRequest,
  files: &[RenameFileInput<'_>],
) -> RenamePatchCandidate {
  let verdict = classify_rename(request);
  let mut file_patches = Vec::new();
  let mut combined = String::new();

  if matches!(verdict, RenameVerdict::RenameReady) {
    for input in files {
      // OWNER-LAW (2026-05-11): files outside `request.target_paths`
      // must NEVER produce edits. The `.px` owner law's
      // `scope=local-target-paths` is a hard boundary — a caller may
      // hand us a broader staging set (e.g. an entire crate) and we
      // are responsible for filtering down to the requested paths.
      // This filter is also a defense against
      // `target-path-content-missing`-class accidents where the
      // strict preflight wouldn't catch a stray file that happens
      // not to be requested.
      if !request.target_paths.iter().any(|tp| tp == input.path) {
        continue;
      }
      let edits = compute_file_rename_edits(&request.old_name, &request.new_name, input.content);
      if edits.is_empty() {
        continue;
      }
      let diff =
        render_unified_diff_for_rename(input.path, input.content, &edits, &request.new_name);
      combined.push_str(&diff);
      file_patches.push(RenameFilePatch {
        path: input.path.to_string(),
        edits,
        unified_diff: diff,
      });
    }
  }
  RenamePatchCandidate {
    request: request.clone(),
    verdict,
    file_patches,
    combined_unified_diff: combined,
    scope_aware_error: None,
  }
}

/// Rust-safe orchestrator: same as [`compute_rename_patch_candidate`]
/// but uses [`compute_file_rename_edits_rust_safe`] for the edit walk
/// when `request.language == "rust"`, falling back to the token-based
/// walk for other languages. This produces SAFER rename candidates
/// for Rust source — identifiers inside string literals, char
/// literals, lifetimes, comments, and macro call names are excluded.
///
/// OWNER-LAW (2026-05-11): the canonical entry point for service-
/// grade Rust rename. Other languages should keep using
/// [`compute_rename_patch_candidate`] until per-language skip-zone
/// lexers land (Python, TypeScript, etc.).
///
/// Behavior:
///   - `language == "rust"`: use `compute_file_rename_edits_rust_safe`
///   - other languages: identical to `compute_rename_patch_candidate`
///   - all classifier verdicts still apply (Held / Rejected still
///     produce empty file_patches; same as the non-safe variant).
///
/// The returned candidate's `verdict` is the classifier's
/// `RenameVerdict::RenameReady` even when all edits got filtered out
/// — an empty edit list represents "no safe rewrite sites found",
/// not a held/rejected request. Callers should check
/// `candidate.file_patches.is_empty()` and treat that as
/// "no changes" rather than "request failed".
pub fn compute_rename_patch_candidate_rust_safe(
  request: &RenameRequest,
  files: &[RenameFileInput<'_>],
) -> RenamePatchCandidate {
  let verdict = classify_rename(request);
  let mut file_patches = Vec::new();
  let mut combined = String::new();

  if matches!(verdict, RenameVerdict::RenameReady) {
    let use_rust_lexer = request.language == "rust";
    for input in files {
      if !request.target_paths.iter().any(|tp| tp == input.path) {
        continue;
      }
      let edits = if use_rust_lexer {
        compute_file_rename_edits_rust_safe(&request.old_name, &request.new_name, input.content)
      } else {
        compute_file_rename_edits(&request.old_name, &request.new_name, input.content)
      };
      if edits.is_empty() {
        continue;
      }
      let diff =
        render_unified_diff_for_rename(input.path, input.content, &edits, &request.new_name);
      combined.push_str(&diff);
      file_patches.push(RenameFilePatch {
        path: input.path.to_string(),
        edits,
        unified_diff: diff,
      });
    }
  }
  RenamePatchCandidate {
    request: request.clone(),
    verdict,
    file_patches,
    combined_unified_diff: combined,
    scope_aware_error: None,
  }
}

/// Multi-language safe orchestrator. Same as
/// [`compute_rename_patch_candidate`] but dispatches to the
/// per-language safe walker for `language ∈ {rust, python,
/// typescript, javascript, go}`, filtering edits inside strings /
/// comments / template literals / etc. Other languages fall back
/// to the token-based walker.
///
/// OWNER-LAW (2026-05-11): canonical entry point for **service-grade
/// multi-language safe rename**. Use this instead of the
/// language-specific `_rust_safe` variant when the caller wants
/// safety across all supported languages.
///
/// Each language's safety filter is documented at its
/// `compute_file_rename_edits_<lang>_safe` function. Identifiers
/// inside strings (single, double, triple, raw, byte, template,
/// f-string) and comments (line, block, doc) are filtered out; the
/// Rust path also filters lifetimes and macro call sites.
///
/// Held / Rejected verdicts short-circuit identically to
/// [`compute_rename_patch_candidate`]: empty `file_patches`, the
/// classifier verdict is the authoritative outcome.
pub fn compute_rename_patch_candidate_lang_safe(
  request: &RenameRequest,
  files: &[RenameFileInput<'_>],
) -> RenamePatchCandidate {
  let verdict = classify_rename(request);
  let mut file_patches = Vec::new();
  let mut combined = String::new();
  let mut scope_aware_error: Option<String> = None;

  if matches!(verdict, RenameVerdict::RenameReady) {
    // D-23: scope-aware rename activates when `target_fn_name` is
    // `Some`. v0 supports `rust` only — other languages with a
    // `Some` value surface a `scope_aware_error` and emit no
    // patches (verdict stays Ready — the rename was attempted,
    // not classified as Held).
    if let Some(target_fn_name) = &request.target_fn_name {
      let scope_aware_supported = matches!(
        request.language.as_str(),
        "rust" | "typescript" | "javascript" | "go" | "python"
      );
      if !scope_aware_supported {
        scope_aware_error = Some(format!(
          "scope-aware rename (target_fn_name) is v0-only supported for `rust` / `typescript` / `javascript` / `go` / `python`; got `{}`",
          request.language
        ));
      } else {
        for input in files {
          if !request.target_paths.iter().any(|tp| tp == input.path) {
            continue;
          }
          let scope_aware_result: Result<Vec<RenameEdit>, RustScopeAwareError> =
            match request.language.as_str() {
              "rust" => compute_file_rename_edits_rust_scope_aware(
                &request.old_name,
                &request.new_name,
                input.content,
                target_fn_name,
              ),
              "typescript" => compute_file_rename_edits_typescript_scope_aware(
                &request.old_name,
                &request.new_name,
                input.content,
                target_fn_name,
              ),
              "javascript" => compute_file_rename_edits_javascript_scope_aware(
                &request.old_name,
                &request.new_name,
                input.content,
                target_fn_name,
              ),
              "go" => compute_file_rename_edits_go_scope_aware(
                &request.old_name,
                &request.new_name,
                input.content,
                target_fn_name,
              ),
              "python" => compute_file_rename_edits_python_scope_aware(
                &request.old_name,
                &request.new_name,
                input.content,
                target_fn_name,
              ),
              // scope_aware_supported gate above guarantees this is
              // unreachable.
              _ => unreachable!(),
            };
          match scope_aware_result {
            Ok(edits) => {
              if edits.is_empty() {
                continue;
              }
              let diff = render_unified_diff_for_rename(
                input.path,
                input.content,
                &edits,
                &request.new_name,
              );
              combined.push_str(&diff);
              file_patches.push(RenameFilePatch {
                path: input.path.to_string(),
                edits,
                unified_diff: diff,
              });
            }
            Err(RustScopeAwareError::TargetFunctionNotFound) => {
              scope_aware_error = Some(format!(
                "scope-aware rename: no `fn {target_fn_name}(...)` found in {}",
                input.path
              ));
              break;
            }
            Err(RustScopeAwareError::MultipleTargetFunctions { count }) => {
              scope_aware_error = Some(format!(
                "scope-aware rename: {count} `fn {target_fn_name}` declarations in {}; \
                 disambiguate by byte offset or impl path (v1 work)",
                input.path
              ));
              break;
            }
          }
        }
      }
    } else {
      // Whole-file rename (original behavior).
      for input in files {
        if !request.target_paths.iter().any(|tp| tp == input.path) {
          continue;
        }
        let edits = compute_file_rename_edits_lang_safe(
          &request.language,
          &request.old_name,
          &request.new_name,
          input.content,
        );
        if edits.is_empty() {
          continue;
        }
        let diff =
          render_unified_diff_for_rename(input.path, input.content, &edits, &request.new_name);
        combined.push_str(&diff);
        file_patches.push(RenameFilePatch {
          path: input.path.to_string(),
          edits,
          unified_diff: diff,
        });
      }
    }
  }
  RenamePatchCandidate {
    request: request.clone(),
    verdict,
    file_patches,
    combined_unified_diff: combined,
    scope_aware_error,
  }
}

/// Strict preflight orchestrator.
///
/// **Verdict ordering** (2026-05-11): request-shape verdicts from the
/// `.px` owner law's `classify` ladder always win over host-level
/// file-content checks. The order is:
///
///   1. [`classify_rename`] — request-shape ladder (missing names,
///      invalid identifier, unsupported language, bad scope, bad
///      target paths). If this holds / rejects, the strict preflight
///      returns the classifier verdict unchanged.
///   2. **S0: duplicate-target-path-content** — a target path must
///      not appear more than once in the staged inputs. Ambiguous
///      input → Held. Structural property of the staged set,
///      independent of edit content.
///   3. **S1: target-path-content-missing** — every
///      `request.target_paths` entry must appear as the `path` of
///      some `files` input. The host can't rename a file it isn't
///      staged. Also pre-base because it doesn't need edits to be
///      computed.
///   4. Compute the base patch candidate. Steps 2 and 3 already
///      filtered the staged set to clean one-content-per-target.
///   5. **S2: old-name-not-found** — across all target files, at
///      least one whole-word occurrence of `old_name` must exist.
///      Zero occurrences = no-op rename — Held so the operator can
///      correct the request.
///   6. **S3: name-collision-detected** — `new_name` must not
///      already appear as a standalone identifier in any target file.
///      (A future CST upgrade narrows this to "in a colliding
///      scope"; this slice is conservative.)
///
/// On any S* failure, returns a `RenameHeld { ... }` outcome and
/// empty file_patches / unified_diff. Files outside `target_paths`
/// in the staged input are silently ignored (the carrier trusts the
/// caller to stage exactly the paths it wants renamed).
///
/// OWNER-LAW (2026-05-11): the held_kinds emitted here are documented
/// in the `.px` owner law's held_kind ledger as *host-emitted*. The
/// `.px` classify is request-shape-only; this strict preflight is the
/// file-content-aware layer. The ordering ensures the cockpit always
/// gets the most precise verdict — a malformed request shape never
/// surfaces as a file-content Held because the classify ladder
/// catches it first.
pub fn compute_rename_patch_candidate_strict(
  request: &RenameRequest,
  files: &[RenameFileInput<'_>],
) -> RenamePatchCandidate {
  // 1. Request-shape ladder runs first. `.px` classify owns these
  //    verdicts; host file-content checks must not preempt them.
  let verdict = classify_rename(request);
  if !matches!(verdict, RenameVerdict::RenameReady) {
    return RenamePatchCandidate {
      request: request.clone(),
      verdict,
      file_patches: Vec::new(),
      combined_unified_diff: String::new(),
      scope_aware_error: None,
    };
  }

  // 2. S0: dedup check — a target path must not appear more than once
  //    in the staged inputs. Runs before S1–S3 because it's a
  //    structural property of the staged set, not of edit content.
  //    Files outside target_paths are exempt (they're filtered by
  //    `compute_rename_patch_candidate` anyway).
  {
    let mut seen: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for f in files {
      if !request.target_paths.iter().any(|tp| tp == f.path) {
        continue;
      }
      if !seen.insert(f.path) {
        return held_with_empty_patches(
          request,
          RenameHeldKind::DuplicateTargetPathContent,
          format!(
            "target path '{}' appears more than once in staged files; \
             carrier cannot deterministically choose which content is the source",
            f.path
          ),
        );
      }
    }
  }

  // 3. S1: every target path must be staged. Runs *before* the base
  //    pipeline — when target paths aren't staged we don't need to
  //    compute edits for the ones that are. This is a structural
  //    check on (request.target_paths, files) shape, independent of
  //    file content.
  {
    let staged: std::collections::BTreeSet<&str> = files.iter().map(|f| f.path).collect();
    for tp in &request.target_paths {
      if !staged.contains(tp.as_str()) {
        return held_with_empty_patches(
          request,
          RenameHeldKind::TargetPathContentMissing,
          format!("request.target_paths includes '{tp}' which is not present in the staged files"),
        );
      }
    }
  }

  // 4. Now run the base pipeline to compute edit candidates. After
  //    S0 (dedup) and S1 (target-staged), the staged set is a clean
  //    one-content-per-target-path mapping.
  let base = compute_rename_patch_candidate(request, files);
  // Sanity: classify said Ready, so base should also be Ready. If not,
  // something diverged between the two; pass through the base verdict
  // as the more specific signal.
  if !matches!(base.verdict, RenameVerdict::RenameReady) {
    return base;
  }

  // S2: at least one occurrence of old_name across target files.
  let total_old_occurrences: usize = base.file_patches.iter().map(|fp| fp.edits.len()).sum();
  if total_old_occurrences == 0 {
    return held_with_empty_patches(
      request,
      RenameHeldKind::OldNameNotFound,
      format!(
        "no whole-word occurrence of `{}` found in any target file — rename is a no-op",
        request.old_name
      ),
    );
  }

  // S3: new_name must not already appear as a standalone identifier
  // in any target file.
  for input in files {
    if !request
      .target_paths
      .iter()
      .any(|tp| tp.as_str() == input.path)
    {
      continue;
    }
    let collisions = compute_file_rename_edits(&request.new_name, "<unused>", input.content);
    if !collisions.is_empty() {
      let first = &collisions[0];
      return held_with_empty_patches(
        request,
        RenameHeldKind::NameCollisionDetected,
        format!(
          "`{}` already appears as a standalone identifier in {} at line {} column {}; rename would collide",
          request.new_name, input.path, first.line, first.column
        ),
      );
    }
  }

  base
}

fn held_with_empty_patches(
  request: &RenameRequest,
  held_kind: RenameHeldKind,
  reason: String,
) -> RenamePatchCandidate {
  RenamePatchCandidate {
    request: request.clone(),
    verdict: RenameVerdict::RenameHeld { held_kind, reason },
    file_patches: Vec::new(),
    combined_unified_diff: String::new(),
    scope_aware_error: None,
  }
}

/// Render a `RenamePatchCandidate` as the JSON payload of a
/// `coding.generated-patch-candidate` artifact.
///
/// OWNER-LAW (2026-05-11): the artifact is *candidate-only*. The
/// downstream `ToolActionApproval` gate decides whether to apply.
/// This builder does not append to any store — it returns the
/// payload Value for the caller to wrap in their preferred artifact
/// envelope shape.
///
/// Payload shape (matches the user's documented contract):
///
/// ```json
/// {
///   "transform": "rename-symbol",
///   "owner_law": "stdlib/lib/gate/code-transform/rename-symbol.px",
///   "old_name": "...",
///   "new_name": "...",
///   "target_paths": ["..."],
///   "language": "rust",
///   "scope": "local-target-paths",
///   "verdict": "rename-ready",
///   "edits": [
///     {
///       "path": "...",
///       "byte_offset": 123,
///       "byte_len": 3,
///       "line": 10,
///       "column": 5
///     }
///   ],
///   "unified_diff": "...",
///   "candidate_only": true,
///   "next_step": "host-cst-rewrite-then-tool-action-approval"
/// }
/// ```
pub fn build_rename_symbol_patch_candidate_payload(
  candidate: &RenamePatchCandidate,
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
          "column": e.column,
        })
      })
    })
    .collect();
  let verdict_str = match &candidate.verdict {
    RenameVerdict::RenameReady => "rename-ready".to_string(),
    RenameVerdict::RenameHeld { .. } => "rename-held".to_string(),
    RenameVerdict::RenameRejected { .. } => "rename-rejected".to_string(),
  };
  let mut payload = serde_json::json!({
    "transform": "rename-symbol",
    "owner_law": "stdlib/lib/gate/code-transform/rename-symbol.px",
    "old_name": request.old_name,
    "new_name": request.new_name,
    "target_paths": request.target_paths,
    "language": request.language,
    "scope": request.scope.as_str(),
    "verdict": verdict_str,
    "edits": edits_arr,
    "unified_diff": candidate.combined_unified_diff,
    "candidate_only": true,
    "next_step": "host-cst-rewrite-then-tool-action-approval",
  });
  // Attach held_kind / reason when present.
  match &candidate.verdict {
    RenameVerdict::RenameHeld { held_kind, reason }
    | RenameVerdict::RenameRejected { held_kind, reason } => {
      payload["held_kind"] = serde_json::Value::String(held_kind.as_str().to_string());
      payload["reason"] = serde_json::Value::String(reason.clone());
    }
    RenameVerdict::RenameReady => {}
  }
  // D-23: scope-aware rename — echo the caller's target_fn_name
  // and any error string the carrier emitted. Cockpit / consumer
  // reads these to know whether scope-aware was requested and
  // whether it succeeded.
  if let Some(tfn) = &request.target_fn_name {
    payload["target_fn_name"] = serde_json::Value::String(tfn.clone());
  }
  if let Some(err) = &candidate.scope_aware_error {
    payload["scope_aware_error"] = serde_json::Value::String(err.clone());
  }
  payload
}

/// Wrap a `RenamePatchCandidate` into a `coding.generated-patch-candidate`
/// artifact value, suitable for `DoghouseStore::append_coding_memory_artifact`
/// or doghouse-http projection.
///
/// OWNER-LAW (2026-05-11): produces a JSON value with all the
/// `CodingMemoryArtifact` fields populated:
///   - `id` — replay-stable: `generated-patch.rename-symbol.<sha256-prefix>`
///     keyed on (request, file_patch edits + paths). Same inputs →
///     same id.
///   - `artifact_family = "coding.generated-patch-candidate"`
///   - `source_surface = "code-transform.rename-symbol"`
///   - `target_paths` — copied from request
///   - `related_refs` — owner-law ref for audit traceability
///   - `payload` — the
///     [`build_rename_symbol_patch_candidate_payload`] output
///
/// `stored_at_ms` is provided by the caller (the host's clock — pnix
/// keeps time as an explicit input so replay is deterministic).
/// `repo_snapshot_ref` is optional; when present (e.g. a git commit
/// SHA), audit can pin the candidate to the source tree it ran against.
pub fn build_rename_symbol_patch_candidate_artifact(
  candidate: &RenamePatchCandidate,
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
) -> serde_json::Value {
  let payload = build_rename_symbol_patch_candidate_payload(candidate);
  // Replay-stable id: SHA-256 of a canonical projection of the
  // identity-bearing fields. We do NOT include stored_at_ms in the
  // hash — same inputs at different times produce the same id.
  let mut hasher = Sha256::new();
  hasher.update(b"rename-symbol\x1f");
  hasher.update(candidate.request.old_name.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(candidate.request.new_name.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(candidate.request.language.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(candidate.request.scope.as_str().as_bytes());
  hasher.update(b"\x1f");
  for tp in &candidate.request.target_paths {
    hasher.update(tp.as_bytes());
    hasher.update(b"\x1e");
  }
  hasher.update(b"\x1f");
  for fp in &candidate.file_patches {
    hasher.update(fp.path.as_bytes());
    hasher.update(b"\x1e");
    for e in &fp.edits {
      hasher.update(e.byte_offset.to_le_bytes());
      hasher.update(e.byte_len.to_le_bytes());
      hasher.update(b"\x1d");
    }
    // OWNER-LAW (2026-05-11): mix in the unified_diff bytes so the id
    // distinguishes patches that share `(offset, len)` but operate on
    // different surrounding line content. Edits alone aren't a unique
    // patch identity — same `(file, offsets)` can produce different
    // rewrites if the source line was different. The diff bytes are
    // the closest proxy to "what this candidate would actually do".
    hasher.update(b"\x1c");
    hasher.update(fp.unified_diff.as_bytes());
  }
  let digest = hasher.finalize();
  let prefix = digest
    .iter()
    .take(8)
    .map(|b| format!("{b:02x}"))
    .collect::<String>();
  let id = format!("generated-patch.rename-symbol.{prefix}");

  let mut artifact = serde_json::json!({
    "id": id,
    "artifact_family": "coding.generated-patch-candidate",
    "source_surface": "code-transform.rename-symbol",
    "stored_at_ms": stored_at_ms,
    "target_paths": candidate.request.target_paths,
    "command_refs": serde_json::Value::Array(Vec::new()),
    "related_refs": serde_json::json!([
      "owner-law:stdlib/lib/gate/code-transform/rename-symbol.px"
    ]),
    "payload": payload,
  });
  if let Some(snap) = repo_snapshot_ref {
    artifact["repo_snapshot_ref"] = serde_json::Value::String(snap.to_string());
  }
  artifact
}

/// Trust seal: a `RenamePatchCandidate` that has passed the
/// `verdict == RenameReady` check.
///
/// OWNER-LAW (2026-05-11): the *only* way to construct this value is
/// through [`ValidatedRenamePatchCandidate::new_checked`], which runs
/// the readiness gate. Same pattern as `ValidatedDerivedEnvelope` in
/// `doghouse-core::emergent_search` — type-enforced precondition that
/// caller cannot bypass.
///
/// The apply path ([`apply_rename_patch_candidate`]) requires a
/// `&ValidatedRenamePatchCandidate`, so a Held / Rejected candidate
/// can never reach apply by accident.
#[derive(Debug, Clone)]
pub struct ValidatedRenamePatchCandidate {
  candidate: RenamePatchCandidate,
}

impl ValidatedRenamePatchCandidate {
  /// Explicit checked constructor. `Ok(self)` only when
  /// `candidate.verdict == RenameReady`; otherwise returns the
  /// candidate back so the caller can audit / log without losing the
  /// reason.
  pub fn new_checked(candidate: RenamePatchCandidate) -> Result<Self, RenamePatchCandidate> {
    if matches!(candidate.verdict, RenameVerdict::RenameReady) {
      Ok(Self { candidate })
    } else {
      Err(candidate)
    }
  }

  pub fn candidate(&self) -> &RenamePatchCandidate {
    &self.candidate
  }

  pub fn into_candidate(self) -> RenamePatchCandidate {
    self.candidate
  }
}

/// An approval record for a derived rename-symbol patch candidate.
///
/// OWNER-LAW (2026-05-11): apply MUST NOT happen without this. The
/// approval is auth-claimed; the caller of
/// [`apply_rename_patch_candidate`] is responsible for verifying
/// `approval.actor_id` / `tenant_id` against a real auth context
/// upstream (same shape as `DerivedIngestAuthContext` in
/// `doghouse-core::derived_envelope_decisions`).
///
/// `candidate_artifact_id` is the replay-stable id from
/// [`build_rename_symbol_patch_candidate_artifact`]. The apply path
/// re-computes it from the sealed candidate and refuses to proceed
/// when it does not match — this is TOCTOU defense between approval
/// time and apply time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenameApplyApproval {
  pub actor_id: String,
  pub tenant_id: String,
  pub approved_at_ms: u64,
  pub candidate_artifact_id: String,
}

/// Reviewer identity for a review-receipt.
///
/// OWNER-LAW (2026-05-11): a review is a human (or background
/// cognition) decision *on* a candidate, distinct from the apply
/// approval that follows. Same actor/tenant shape as
/// [`RenameApplyApproval`] but no `candidate_artifact_id` embedded —
/// the candidate ref lives at the receipt level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenameReviewer {
  pub actor_id: String,
  pub tenant_id: String,
}

/// Reviewer's decision on a `RenamePatchCandidate`.
///
/// OWNER-LAW (2026-05-11): three outcomes — same shape as the
/// derived-envelope decision (`DerivedEnvelopeDecision` in
/// `doghouse-core::derived_envelope_decisions`) so the cockpit can
/// render review decisions uniformly.
///
///   - `Approve` — caller authorizes apply. Only this decision can
///     legitimately produce a `RenameApplyApproval` downstream.
///   - `Hold` — caller wants more evidence / context before deciding.
///     Candidate is *not* rejected, just deferred.
///   - `Reject` — caller refuses the candidate. Apply must not
///     proceed. The receipt still records the decision so future
///     cognition / audit can avoid re-deriving the same candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RenameReviewDecision {
  Approve,
  Hold,
  Reject,
}

impl RenameReviewDecision {
  pub fn as_str(self) -> &'static str {
    match self {
      Self::Approve => "approve",
      Self::Hold => "hold",
      Self::Reject => "reject",
    }
  }

  /// True only for `Approve`. Used by the apply path to gate
  /// `RenameApplyApproval` construction from a review receipt.
  pub fn permits_apply(self) -> bool {
    matches!(self, Self::Approve)
  }
}

/// The receipt of a review decision on a `RenamePatchCandidate`.
///
/// OWNER-LAW (2026-05-11): emitted *before* apply (if any) — the
/// canonical chain is:
///
///   candidate → REVIEW RECEIPT (this) → apply → APPLY RECEIPT
///
/// The receipt embeds the candidate (so audit/replay can see exactly
/// what was reviewed), the reviewer identity, the decision, an
/// optional human-readable reason, and the review timestamp. The
/// `candidate_artifact_id` is pinned at review time — a future
/// apply step that builds an `ApplyApproval` from this receipt must
/// match this id.
///
/// Only `Approve` reviews are downstream-actionable. `Hold` and
/// `Reject` are terminal for the apply chain but stay in the audit
/// graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenameReviewReceipt {
  pub candidate: RenamePatchCandidate,
  pub reviewer: RenameReviewer,
  pub decision: RenameReviewDecision,
  pub reason: Option<String>,
  pub reviewed_at_ms: u64,
  /// Replay-stable candidate artifact id, pinned at review time. The
  /// review receipt is `tied` to a specific candidate identity; if
  /// the candidate changes (different content, different offsets)
  /// between review and apply, the apply path will detect the
  /// mismatch via this id.
  pub candidate_artifact_id: String,
}

/// The receipt of an applied rename — pure data describing the
/// post-apply state plus an inverse diff for rollback.
///
/// OWNER-LAW (2026-05-11): the carrier produces this *value* but does
/// NOT write to disk. The downstream `ToolActionApproval` host
/// surface decides whether to materialize `per_file_after` onto the
/// filesystem. Keeping the carrier I/O-free preserves
/// determinism and replay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenameApplyReceipt {
  pub candidate: RenamePatchCandidate,
  pub approval: RenameApplyApproval,
  pub applied_at_ms: u64,
  /// Per-file post-apply content: `(path, rewritten_content)`. Only
  /// files that actually had edits appear here.
  pub per_file_after: Vec<(String, String)>,
  /// Reverse unified diff — applying this to the post-apply content
  /// returns the original. Computed by swapping `-` and `+` lines
  /// from the forward diff (rename is a symmetric whole-word
  /// substitution, so the inverse is just the same shape with new
  /// and old swapped).
  pub inverse_unified_diff: String,
}

/// Initiator identity for a rollback handle.
///
/// OWNER-LAW (2026-05-11): rollback is itself a privileged action —
/// it un-does an applied transform. Same actor/tenant shape as the
/// reviewer / approval roles. The initiator may be the same person
/// who originally approved the apply, or someone different (e.g. an
/// operator triggering rollback because the apply was bad).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenameRollbackInitiator {
  pub actor_id: String,
  pub tenant_id: String,
}

/// A handle that authorizes (but has NOT yet executed) the rollback
/// of a specific apply receipt.
///
/// OWNER-LAW (2026-05-11): the rollback chain step:
///
///   apply receipt → ROLLBACK HANDLE (this) → execute → rollback receipt
///
/// The handle is the "I want to roll this back" decision artifact.
/// The actual file rewrite (applying `inverse_unified_diff` to the
/// post-apply state) is a separate downstream step gated by
/// `ToolActionApproval`, just like the original apply.
///
/// The handle carries:
///   - `apply_receipt_artifact_id` — pinned at handle time; defends
///     against rollback drift (the receipt the handle authorizes
///     must be exactly the receipt the rollback path acts on).
///   - `candidate_artifact_id` — the original candidate. Useful for
///     audit graph walks: candidate ← review ← apply ← rollback.
///   - `inverse_unified_diff` — copied from the apply receipt for
///     self-containment. Audit / replay don't need to fetch the
///     apply receipt to see what rollback would do.
///   - `initiator` + `initiated_at_ms` + `reason` — who/when/why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenameRollbackHandle {
  pub apply_receipt: RenameApplyReceipt,
  pub apply_receipt_artifact_id: String,
  pub candidate_artifact_id: String,
  pub initiator: RenameRollbackInitiator,
  pub initiated_at_ms: u64,
  pub reason: Option<String>,
  pub inverse_unified_diff: String,
  pub target_paths: Vec<String>,
}

/// Executor identity for rollback execution.
///
/// OWNER-LAW (2026-05-11): same actor/tenant shape as the apply
/// approval — the rollback executor is the agent that actually runs
/// the inverse patch. May be the same actor who initiated the
/// rollback handle, or a different bounded runner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenameRollbackExecutor {
  pub actor_id: String,
  pub tenant_id: String,
}

/// The receipt of an executed rollback — pure data describing the
/// post-rollback state with per-file content + sha256.
///
/// OWNER-LAW (2026-05-11): the closing step of the canonical chain:
///
///   candidate → review → apply → rollback handle → ROLLBACK RECEIPT (this)
///
/// The receipt records:
///   - The handle that authorized the rollback (`handle` + pinned
///     `rollback_handle_artifact_id`).
///   - The executor (actor + tenant) and execution timestamp.
///   - Per-file post-rollback content (`per_file_after_rollback`):
///     applying the inverse rename to the post-apply files restores
///     the pre-apply content.
///
/// The receipt is pure data — no disk I/O. The host downstream
/// decides whether to write `per_file_after_rollback` to disk under
/// `ToolActionApproval`, same shape as the apply path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenameRollbackReceipt {
  pub handle: RenameRollbackHandle,
  pub rollback_handle_artifact_id: String,
  pub executor: RenameRollbackExecutor,
  pub executed_at_ms: u64,
  /// Per-file post-rollback content: `(path, restored_content)`.
  /// Under a clean rollback, each `restored_content` equals the
  /// pre-apply content of that file.
  pub per_file_after_rollback: Vec<(String, String)>,
}

/// Errors from [`execute_rename_rollback`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenameRollbackError {
  /// Executor's actor_id is empty.
  MissingExecutorActor,
  /// Executor's tenant_id is empty.
  MissingExecutorTenant,
  /// `current_files` is missing a path that the handle's apply
  /// receipt named.
  MissingFileForRollback { path: String },
  /// Post-apply content on "disk" (the `current_files` input)
  /// doesn't match the sha256 the apply receipt recorded — the
  /// file has drifted since apply. Rollback would not produce
  /// the pre-apply state. Fail-closed.
  PostApplyContentDriftDetected {
    path: String,
    expected_sha256: String,
    actual_sha256: String,
  },
}

impl std::fmt::Display for RenameRollbackError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::MissingExecutorActor => write!(f, "executor.actor_id must be non-empty"),
      Self::MissingExecutorTenant => write!(f, "executor.tenant_id must be non-empty"),
      Self::MissingFileForRollback { path } => write!(
        f,
        "rollback input is missing file '{path}' (the handle's apply receipt names it)"
      ),
      Self::PostApplyContentDriftDetected {
        path,
        expected_sha256,
        actual_sha256,
      } => write!(
        f,
        "post-apply content drift at '{path}': expected sha256={expected_sha256}, got sha256={actual_sha256}"
      ),
    }
  }
}

impl std::error::Error for RenameRollbackError {}

/// Errors from [`apply_rename_patch_candidate`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenameApplyError {
  /// Approval's `candidate_artifact_id` does not match the sealed
  /// candidate's freshly-recomputed id. Either the candidate changed
  /// between approval and apply (TOCTOU) or the approval references
  /// a different candidate. Fail-closed: do not apply.
  ApprovalCandidateIdMismatch { expected: String, got: String },
  /// `files` is missing a path that the sealed candidate names in its
  /// `file_patches`. The caller must stage the exact files the
  /// candidate touched.
  MissingFileForPatch { path: String },
  /// `approval.actor_id` is empty. Apply cannot be anonymous.
  MissingApprovalActor,
  /// `approval.tenant_id` is empty. Apply cannot be tenant-less.
  MissingApprovalTenant,
}

impl std::fmt::Display for RenameApplyError {
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

impl std::error::Error for RenameApplyError {}

/// Apply a sealed rename-symbol patch candidate, producing a
/// receipt with post-apply content and inverse diff for rollback.
///
/// OWNER-LAW (2026-05-11): preconditions verified in order:
///
///   1. `approval.actor_id` and `approval.tenant_id` non-empty
///      (TOCTOU-defense placeholders; real auth check is upstream).
///   2. `approval.candidate_artifact_id` equals the sealed
///      candidate's freshly-recomputed id (defends against the
///      candidate changing between approval and apply).
///   3. Every path in `sealed.file_patches` is present in `files`.
///
/// On success returns `Ok(RenameApplyReceipt)`. The receipt is pure
/// data; the host writes `per_file_after` to disk under its own
/// `ToolActionApproval` audit lane.
///
/// Rollback: applying `receipt.inverse_unified_diff` to the
/// post-apply state restores the pre-apply state.
pub fn apply_rename_patch_candidate(
  sealed: &ValidatedRenamePatchCandidate,
  files: &[RenameFileInput<'_>],
  approval: &RenameApplyApproval,
  applied_at_ms: u64,
) -> Result<RenameApplyReceipt, RenameApplyError> {
  // 1. auth claim shape
  if approval.actor_id.is_empty() {
    return Err(RenameApplyError::MissingApprovalActor);
  }
  if approval.tenant_id.is_empty() {
    return Err(RenameApplyError::MissingApprovalTenant);
  }

  let candidate = sealed.candidate();

  // 2. TOCTOU: re-derive the candidate's artifact id and compare to
  //    the approval's pinned id. We use the same builder a doghouse
  //    wrapper would, with placeholder stored_at_ms=0 because the
  //    builder excludes stored_at_ms from the hash.
  let recomputed = build_rename_symbol_patch_candidate_artifact(candidate, 0, None);
  let recomputed_id = recomputed
    .get("id")
    .and_then(|v| v.as_str())
    .unwrap_or("")
    .to_string();
  if recomputed_id != approval.candidate_artifact_id {
    return Err(RenameApplyError::ApprovalCandidateIdMismatch {
      expected: approval.candidate_artifact_id.clone(),
      got: recomputed_id,
    });
  }

  // 3. every patch path must be staged. Build a path→content map.
  let mut file_content: std::collections::BTreeMap<&str, &str> = std::collections::BTreeMap::new();
  for f in files {
    file_content.insert(f.path, f.content);
  }
  let mut per_file_after: Vec<(String, String)> = Vec::new();
  for fp in &candidate.file_patches {
    let original =
      file_content
        .get(fp.path.as_str())
        .ok_or_else(|| RenameApplyError::MissingFileForPatch {
          path: fp.path.clone(),
        })?;
    let rewritten = apply_rename_edits(original, &fp.edits, &candidate.request.new_name);
    per_file_after.push((fp.path.clone(), rewritten));
  }

  // 4. inverse diff: swap `-` and `+` prefixes line-by-line in the
  //    forward diff. Rename is a symmetric whole-word substitution,
  //    so the inverse is the same shape with the leading `-`/`+`
  //    flipped on body lines (headers and hunk anchors stay the
  //    same).
  let inverse_unified_diff = invert_unified_diff(&candidate.combined_unified_diff);

  Ok(RenameApplyReceipt {
    candidate: candidate.clone(),
    approval: approval.clone(),
    applied_at_ms,
    per_file_after,
    inverse_unified_diff,
  })
}

/// Policy for whether `files_after[*].content` is included verbatim
/// in the apply-receipt payload.
///
/// OWNER-LAW (2026-05-11): `files_after.content` is *the full
/// post-apply file content*. Including it is great for dev / debug
/// (the receipt is self-contained and reviewable), but in a
/// customer-release / service deployment storing every file's full
/// content in an artifact is a privacy / capacity concern. The hash
/// is always emitted (so verifiers can recompute from disk and
/// match); the body is gated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApplyReceiptContentPolicy {
  /// Emit `files_after[*].content` (full post-apply text) + sha256.
  /// Suitable for dev/debug. NOT recommended for customer-release.
  IncludeContent,
  /// Emit `files_after[*]` with `content_sha256` + `byte_len` only.
  /// `content` is omitted. Suitable for customer-release / multi-
  /// tenant / privacy-sensitive deployments. Verifiers can still
  /// validate by recomputing sha256 from disk.
  OmitContent,
}

impl ApplyReceiptContentPolicy {
  pub fn as_str(self) -> &'static str {
    match self {
      Self::IncludeContent => "include-content",
      Self::OmitContent => "omit-content",
    }
  }
}

/// Render a `RenameApplyReceipt` as the JSON payload of a
/// `coding.generated-patch-apply-receipt` artifact.
///
/// OWNER-LAW (2026-05-11): the apply receipt is the audit-and-replay
/// trail of a single apply event. Payload shape under
/// `ApplyReceiptContentPolicy::IncludeContent`:
///
/// ```json
/// {
///   "transform": "rename-symbol",
///   "owner_law": "stdlib/lib/gate/code-transform/rename-symbol.px",
///   "candidate_artifact_id": "generated-patch.rename-symbol.<hex>",
///   "approval": { "actor_id", "tenant_id", "approved_at_ms" },
///   "applied_at_ms": ...,
///   "target_paths": [...],
///   "content_policy": "include-content" | "omit-content",
///   "files_after": [
///     { "path": "...", "content": "...", "content_sha256": "...",
///       "byte_len": ... }
///   ],
///   "inverse_unified_diff": "...",
///   "rollback_available": true,
///   "next_step": "verify-or-rollback"
/// }
/// ```
///
/// Under `OmitContent`, `files_after[*].content` is dropped — only
/// `path` + `content_sha256` + `byte_len` survive. The content_sha256
/// is always present; verifiers can recompute it from disk and
/// match. `inverse_unified_diff` is the rollback patch (the apply
/// lane's `-`/`+` body-line swap).
pub fn build_rename_symbol_apply_receipt_payload(
  receipt: &RenameApplyReceipt,
  content_policy: ApplyReceiptContentPolicy,
) -> serde_json::Value {
  // candidate_artifact_id: recompute from the receipt's embedded
  // candidate (same canonical hash the apply path checked). Using
  // stored_at_ms=0 because the candidate id excludes stored_at_ms by
  // design.
  let candidate_art = build_rename_symbol_patch_candidate_artifact(&receipt.candidate, 0, None);
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
      if matches!(content_policy, ApplyReceiptContentPolicy::IncludeContent) {
        entry["content"] = serde_json::Value::String(content.clone());
      }
      entry
    })
    .collect();
  serde_json::json!({
    "transform": "rename-symbol",
    "owner_law": "stdlib/lib/gate/code-transform/rename-symbol.px",
    "candidate_artifact_id": candidate_artifact_id,
    "approval": {
      "actor_id": receipt.approval.actor_id,
      "tenant_id": receipt.approval.tenant_id,
      "approved_at_ms": receipt.approval.approved_at_ms,
    },
    "applied_at_ms": receipt.applied_at_ms,
    "target_paths": receipt.candidate.request.target_paths,
    "content_policy": content_policy.as_str(),
    "files_after": files_after,
    "inverse_unified_diff": receipt.inverse_unified_diff,
    "rollback_available": !receipt.inverse_unified_diff.is_empty(),
    "next_step": "verify-or-rollback",
  })
}

/// Wrap a `RenameApplyReceipt` into a full
/// `coding.generated-patch-apply-receipt` artifact value with a
/// replay-stable id.
///
/// OWNER-LAW (2026-05-11, strengthened): the id hash now binds the
/// **applied result identity**, not just metadata. Inputs to the
/// hash:
///
///   1. `candidate_artifact_id` (links back to the source patch)
///   2. `approval.actor_id` / `tenant_id` / `approved_at_ms`
///   3. `applied_at_ms` (event identity — each apply is distinct)
///   4. For each file in `per_file_after`: `path` + sha256 of its
///      post-apply content
///   5. `inverse_unified_diff` (the rollback patch)
///
/// Inputs (4)/(5) are the strengthening: a substrate bug that
/// produced different post-apply content for the same candidate +
/// approval + time would yield a *different* receipt id, so audit
/// can detect the divergence. The earlier hash didn't include (4) /
/// (5), so two apply events with the same metadata but different
/// outcomes would have collided.
///
/// `stored_at_ms` (storage envelope wall-clock) and the
/// `content_policy` choice are *not* in the hash — they're extrinsic
/// to event identity. The same apply event rendered with
/// `IncludeContent` vs `OmitContent` produces the same id.
pub fn build_rename_symbol_apply_receipt_artifact(
  receipt: &RenameApplyReceipt,
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
  content_policy: ApplyReceiptContentPolicy,
) -> serde_json::Value {
  let payload = build_rename_symbol_apply_receipt_payload(receipt, content_policy);
  let candidate_artifact_id = payload
    .get("candidate_artifact_id")
    .and_then(|v| v.as_str())
    .unwrap_or("");

  let mut hasher = Sha256::new();
  hasher.update(b"rename-symbol-apply\x1f");
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
  // (4) per-file applied result: path + sha256 of post-apply content.
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
  // (5) inverse diff bytes — rollback identity.
  hasher.update(receipt.inverse_unified_diff.as_bytes());
  let digest = hasher.finalize();
  let prefix = digest
    .iter()
    .take(8)
    .map(|b| format!("{b:02x}"))
    .collect::<String>();
  let id = format!("apply-receipt.rename-symbol.{prefix}");

  let mut artifact = serde_json::json!({
    "id": id,
    "artifact_family": "coding.generated-patch-apply-receipt",
    "source_surface": "code-transform.rename-symbol",
    "stored_at_ms": stored_at_ms,
    "target_paths": receipt.candidate.request.target_paths,
    "command_refs": serde_json::Value::Array(Vec::new()),
    "related_refs": serde_json::json!([
      "owner-law:stdlib/lib/gate/code-transform/rename-symbol.px",
      format!("candidate-artifact:{candidate_artifact_id}")
    ]),
    "payload": payload,
  });
  if let Some(snap) = repo_snapshot_ref {
    artifact["repo_snapshot_ref"] = serde_json::Value::String(snap.to_string());
  }
  artifact
}

/// Build a `RenameReviewReceipt` from a candidate + reviewer
/// decision. Pins the candidate's replay-stable artifact id at
/// review time.
///
/// OWNER-LAW (2026-05-11): the only constructor — caller can't make
/// a review receipt without an actual `RenamePatchCandidate` and a
/// reviewer identity. The `candidate_artifact_id` is recomputed
/// here (canonical hash) so the receipt is self-contained for
/// audit / replay.
pub fn build_rename_review_receipt(
  candidate: RenamePatchCandidate,
  reviewer: RenameReviewer,
  decision: RenameReviewDecision,
  reason: Option<String>,
  reviewed_at_ms: u64,
) -> RenameReviewReceipt {
  let art = build_rename_symbol_patch_candidate_artifact(&candidate, 0, None);
  let candidate_artifact_id = art
    .get("id")
    .and_then(|v| v.as_str())
    .unwrap_or("")
    .to_string();
  RenameReviewReceipt {
    candidate,
    reviewer,
    decision,
    reason,
    reviewed_at_ms,
    candidate_artifact_id,
  }
}

/// Lift a review receipt into a `RenameApplyApproval`. Returns
/// `Some(approval)` only when `decision == Approve`; `Hold` / `Reject`
/// returns `None` (apply must not proceed from those).
///
/// OWNER-LAW (2026-05-11): this is the one sanctioned bridge between
/// the review step and the apply step. The approval's
/// `candidate_artifact_id` is carried from the review receipt, so
/// the apply path's TOCTOU check binds the entire chain
/// review → approval → apply on the same candidate identity.
pub fn approval_from_review(receipt: &RenameReviewReceipt) -> Option<RenameApplyApproval> {
  if !receipt.decision.permits_apply() {
    return None;
  }
  Some(RenameApplyApproval {
    actor_id: receipt.reviewer.actor_id.clone(),
    tenant_id: receipt.reviewer.tenant_id.clone(),
    approved_at_ms: receipt.reviewed_at_ms,
    candidate_artifact_id: receipt.candidate_artifact_id.clone(),
  })
}

/// Build a `ToolActionMaterializationRequest` from a rename-symbol
/// review receipt + apply receipt + context.
///
/// OWNER-LAW (2026-05-11): the typed bridge between the canonical
/// chain's pure-data review/apply receipts and the host-side
/// materialization lane. Delegates to the transform-agnostic
/// [`crate::tool_action::bridge_review_apply_to_materialization_request`]
/// after computing the rename-symbol-specific artifact ids.
///
/// Verified preconditions (via the core bridge):
///   - `review.decision == Approve` — Hold and Reject can't trigger
///     a disk write.
///   - `review.candidate_artifact_id` == apply's derived candidate id
///     — TOCTOU: the apply must use the EXACT candidate the review
///     approved.
///   - `review.reviewer.tenant_id == apply.approval.tenant_id` —
///     same-tenant review/apply. Actor can differ (senior reviews,
///     junior applies).
///   - The assembled request passes
///     [`crate::tool_action::classify_tool_action_materialization_request`].
///
/// On success the caller has a Ready'd
/// `ToolActionMaterializationRequest` ready for
/// `build_tool_action_materialization_plan`.
pub fn build_rename_materialization_request(
  review: &RenameReviewReceipt,
  apply: &RenameApplyReceipt,
  capability: &str,
  repo_snapshot_ref: &str,
  deployment_mode: &str,
  content_policy: &str,
  requested_at_ms: u64,
) -> Result<
  crate::tool_action::ToolActionMaterializationRequest,
  crate::tool_action::MaterializationBridgeError,
> {
  // Re-derive the candidate artifact id from the apply receipt's
  // embedded candidate. This is the same canonical hash used at
  // candidate-build time; the bridge core compares it to
  // `review.candidate_artifact_id` for the TOCTOU gate.
  let apply_candidate_art = build_rename_symbol_patch_candidate_artifact(&apply.candidate, 0, None);
  let apply_candidate_id = apply_candidate_art
    .get("id")
    .and_then(|v| v.as_str())
    .unwrap_or("")
    .to_string();

  // Re-derive the apply receipt's own canonical id. Content policy is
  // extrinsic to the id, so we use OmitContent here arbitrarily.
  let apply_art = build_rename_symbol_apply_receipt_artifact(
    apply,
    0,
    None,
    ApplyReceiptContentPolicy::OmitContent,
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

/// Render a `RenameReviewReceipt` as the canonical JSON payload of a
/// `coding.generated-patch-review-receipt` artifact.
///
/// OWNER-LAW (2026-05-11): payload shape:
///
/// ```json
/// {
///   "transform": "rename-symbol",
///   "owner_law": "stdlib/lib/gate/code-transform/rename-symbol.px",
///   "candidate_artifact_id": "generated-patch.rename-symbol.<hex>",
///   "reviewer": { "actor_id": "...", "tenant_id": "..." },
///   "decision": "approve" | "hold" | "reject",
///   "reason": "..." | null,
///   "reviewed_at_ms": ...,
///   "permits_apply": true | false,
///   "next_step": "apply" | "wait-for-evidence" | "rejected"
/// }
/// ```
pub fn build_rename_symbol_review_receipt_payload(
  receipt: &RenameReviewReceipt,
) -> serde_json::Value {
  let next_step = match receipt.decision {
    RenameReviewDecision::Approve => "apply",
    RenameReviewDecision::Hold => "wait-for-evidence",
    RenameReviewDecision::Reject => "rejected",
  };
  let mut payload = serde_json::json!({
    "transform": "rename-symbol",
    "owner_law": "stdlib/lib/gate/code-transform/rename-symbol.px",
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

/// Wrap a `RenameReviewReceipt` into a full
/// `coding.generated-patch-review-receipt` artifact value with
/// a replay-stable id.
///
/// OWNER-LAW (2026-05-11): the id hash binds intrinsic decision
/// identity:
///
///   1. `candidate_artifact_id` (which candidate was reviewed)
///   2. `reviewer.actor_id` / `tenant_id`
///   3. `decision` (kebab-case string)
///   4. `reviewed_at_ms`
///   5. `reason` (when present — distinct decisions on the same
///      candidate with different reasoning are different reviews)
///
/// `stored_at_ms` and `repo_snapshot_ref` are extrinsic — not in the
/// hash. `related_refs` carries `candidate-artifact:<id>` so audit
/// can walk the chain candidate → review-receipt → apply-receipt.
pub fn build_rename_symbol_review_receipt_artifact(
  receipt: &RenameReviewReceipt,
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
) -> serde_json::Value {
  let payload = build_rename_symbol_review_receipt_payload(receipt);
  let mut hasher = Sha256::new();
  hasher.update(b"rename-symbol-review\x1f");
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
  let id = format!("review-receipt.rename-symbol.{prefix}");

  let mut artifact = serde_json::json!({
    "id": id,
    "artifact_family": "coding.generated-patch-review-receipt",
    "source_surface": "code-transform.rename-symbol",
    "stored_at_ms": stored_at_ms,
    "target_paths": receipt.candidate.request.target_paths,
    "command_refs": serde_json::Value::Array(Vec::new()),
    "related_refs": serde_json::json!([
      "owner-law:stdlib/lib/gate/code-transform/rename-symbol.px",
      format!("candidate-artifact:{}", receipt.candidate_artifact_id),
    ]),
    "payload": payload,
  });
  if let Some(snap) = repo_snapshot_ref {
    artifact["repo_snapshot_ref"] = serde_json::Value::String(snap.to_string());
  }
  artifact
}

/// Build a `RenameRollbackHandle` from an apply receipt + initiator.
///
/// OWNER-LAW (2026-05-11): the only constructor — caller can't make
/// a rollback handle without an actual `RenameApplyReceipt` to
/// rollback. The `apply_receipt_artifact_id` and
/// `candidate_artifact_id` are pinned at handle time via canonical
/// hashes; the rollback execution path must match these exactly
/// (TOCTOU defense).
///
/// `stored_at_ms` here is the hash-time stored_at_ms (0); it does
/// not affect the resulting ids since both apply-receipt and
/// candidate hashes exclude stored_at_ms by design.
pub fn build_rename_rollback_handle(
  apply_receipt: RenameApplyReceipt,
  initiator: RenameRollbackInitiator,
  reason: Option<String>,
  initiated_at_ms: u64,
) -> RenameRollbackHandle {
  // Recompute the apply receipt's artifact id (replay-stable, both
  // policies give the same id since policy is presentation-extrinsic).
  let apply_art = build_rename_symbol_apply_receipt_artifact(
    &apply_receipt,
    0,
    None,
    ApplyReceiptContentPolicy::OmitContent,
  );
  let apply_receipt_artifact_id = apply_art
    .get("id")
    .and_then(|v| v.as_str())
    .unwrap_or("")
    .to_string();
  // Recompute the candidate id (carried through the chain).
  let cand_art = build_rename_symbol_patch_candidate_artifact(&apply_receipt.candidate, 0, None);
  let candidate_artifact_id = cand_art
    .get("id")
    .and_then(|v| v.as_str())
    .unwrap_or("")
    .to_string();
  let inverse_unified_diff = apply_receipt.inverse_unified_diff.clone();
  let target_paths = apply_receipt.candidate.request.target_paths.clone();
  RenameRollbackHandle {
    apply_receipt,
    apply_receipt_artifact_id,
    candidate_artifact_id,
    initiator,
    initiated_at_ms,
    reason,
    inverse_unified_diff,
    target_paths,
  }
}

/// Render a `RenameRollbackHandle` as the canonical JSON payload of
/// a `coding.rollback-handle` artifact.
///
/// OWNER-LAW (2026-05-11): payload shape:
///
/// ```json
/// {
///   "transform": "rename-symbol",
///   "owner_law": "stdlib/lib/gate/code-transform/rename-symbol.px",
///   "apply_receipt_artifact_id": "apply-receipt.rename-symbol.<hex>",
///   "candidate_artifact_id": "generated-patch.rename-symbol.<hex>",
///   "initiator": { "actor_id", "tenant_id" },
///   "initiated_at_ms": ...,
///   "reason": "..." | null,
///   "target_paths": [...],
///   "inverse_unified_diff": "...",
///   "rollback_state": "handle-issued",
///   "next_step": "execute-rollback"
/// }
/// ```
pub fn build_rename_symbol_rollback_handle_payload(
  handle: &RenameRollbackHandle,
) -> serde_json::Value {
  let mut payload = serde_json::json!({
    "transform": "rename-symbol",
    "owner_law": "stdlib/lib/gate/code-transform/rename-symbol.px",
    "apply_receipt_artifact_id": handle.apply_receipt_artifact_id,
    "candidate_artifact_id": handle.candidate_artifact_id,
    "initiator": {
      "actor_id": handle.initiator.actor_id,
      "tenant_id": handle.initiator.tenant_id,
    },
    "initiated_at_ms": handle.initiated_at_ms,
    "target_paths": handle.target_paths,
    "inverse_unified_diff": handle.inverse_unified_diff,
    "rollback_state": "handle-issued",
    "next_step": "execute-rollback",
  });
  payload["reason"] = match handle.reason.as_ref() {
    Some(r) => serde_json::Value::String(r.clone()),
    None => serde_json::Value::Null,
  };
  payload
}

/// Wrap a `RenameRollbackHandle` into a full
/// `coding.rollback-handle` artifact value with a replay-stable
/// id.
///
/// OWNER-LAW (2026-05-11): id hash binds rollback intent identity:
///   - `apply_receipt_artifact_id` (which apply is being rolled back)
///   - `initiator.actor_id` / `tenant_id`
///   - `initiated_at_ms`
///   - `reason` (when present)
///
/// `inverse_unified_diff` is *not* in the hash — it's already
/// implicitly bound by `apply_receipt_artifact_id` (the apply
/// receipt's id includes the diff bytes). So the rollback handle id
/// is intrinsic to the *intent*, not redundant with the apply
/// receipt's identity.
///
/// `related_refs` carries both `candidate-artifact:<id>` and
/// `apply-receipt-artifact:<id>` back-refs so audit can walk the
/// full chain.
pub fn build_rename_symbol_rollback_handle_artifact(
  handle: &RenameRollbackHandle,
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
) -> serde_json::Value {
  let payload = build_rename_symbol_rollback_handle_payload(handle);
  let mut hasher = Sha256::new();
  hasher.update(b"rename-symbol-rollback-handle\x1f");
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
  let id = format!("rollback-handle.rename-symbol.{prefix}");

  let mut artifact = serde_json::json!({
    "id": id,
    "artifact_family": "coding.rollback-handle",
    "source_surface": "code-transform.rename-symbol",
    "stored_at_ms": stored_at_ms,
    "target_paths": handle.target_paths,
    "command_refs": serde_json::Value::Array(Vec::new()),
    "related_refs": serde_json::json!([
      "owner-law:stdlib/lib/gate/code-transform/rename-symbol.px",
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

/// Execute the rollback authorized by a `RenameRollbackHandle`.
///
/// OWNER-LAW (2026-05-11): pure function. Inputs:
///   - `handle` — the rollback intent (already authorized).
///   - `current_files` — current file content (must match the
///     handle's apply receipt's `per_file_after` sha256s).
///   - `executor` — who actually ran the inverse rewrite.
///   - `executed_at_ms` — when.
///
/// Preconditions verified:
///   1. `executor.actor_id` and `executor.tenant_id` non-empty
///      (placeholder auth check; real auth is upstream).
///   2. Every path the handle names in
///      `handle.apply_receipt.per_file_after` is present in
///      `current_files`.
///   3. For each such path, `sha256(current_content)` matches the
///      apply receipt's recorded `content_sha256` (no drift since
///      apply).
///
/// On success: produces a `RenameRollbackReceipt` whose
/// `per_file_after_rollback` is computed by *reverse renaming*
/// (rename `new_name` back to `old_name`) the current post-apply
/// content. The result equals the pre-apply content of each file
/// (verified by the round-trip invariant — see the test
/// `rollback_receipt_restores_pre_apply_content`).
///
/// On any precondition violation, returns `Err` and produces no
/// receipt. Fail-closed: rollback that would silently produce a
/// wrong state never materializes.
pub fn execute_rename_rollback(
  handle: &RenameRollbackHandle,
  current_files: &[RenameFileInput<'_>],
  executor: &RenameRollbackExecutor,
  executed_at_ms: u64,
) -> Result<RenameRollbackReceipt, RenameRollbackError> {
  if executor.actor_id.is_empty() {
    return Err(RenameRollbackError::MissingExecutorActor);
  }
  if executor.tenant_id.is_empty() {
    return Err(RenameRollbackError::MissingExecutorTenant);
  }

  // Build a path → current content index.
  let mut current_index: std::collections::BTreeMap<&str, &str> = std::collections::BTreeMap::new();
  for f in current_files {
    current_index.insert(f.path, f.content);
  }

  // For each file the apply receipt touched: verify current content
  // matches the recorded post-apply sha256; then reverse-rename.
  let request = &handle.apply_receipt.candidate.request;
  let mut per_file_after_rollback: Vec<(String, String)> = Vec::new();
  for (path, post_apply_content) in &handle.apply_receipt.per_file_after {
    let current = current_index
      .get(path.as_str())
      .ok_or_else(|| RenameRollbackError::MissingFileForRollback { path: path.clone() })?;
    // Verify no drift: sha256(current) must equal sha256(post_apply).
    let mut expected_h = Sha256::new();
    expected_h.update(post_apply_content.as_bytes());
    let expected_hex: String = expected_h
      .finalize()
      .iter()
      .map(|b| format!("{b:02x}"))
      .collect();
    let mut actual_h = Sha256::new();
    actual_h.update(current.as_bytes());
    let actual_hex: String = actual_h
      .finalize()
      .iter()
      .map(|b| format!("{b:02x}"))
      .collect();
    if expected_hex != actual_hex {
      return Err(RenameRollbackError::PostApplyContentDriftDetected {
        path: path.clone(),
        expected_sha256: expected_hex,
        actual_sha256: actual_hex,
      });
    }
    // Compute the reverse-rename edits: rename new_name → old_name.
    let reverse_edits = compute_file_rename_edits(&request.new_name, &request.old_name, current);
    let restored = apply_rename_edits(current, &reverse_edits, &request.old_name);
    per_file_after_rollback.push((path.clone(), restored));
  }

  // Recompute the rollback-handle artifact id (canonical hash) — the
  // receipt pins this for chain audit.
  let handle_art = build_rename_symbol_rollback_handle_artifact(handle, 0, None);
  let rollback_handle_artifact_id = handle_art
    .get("id")
    .and_then(|v| v.as_str())
    .unwrap_or("")
    .to_string();

  Ok(RenameRollbackReceipt {
    handle: handle.clone(),
    rollback_handle_artifact_id,
    executor: executor.clone(),
    executed_at_ms,
    per_file_after_rollback,
  })
}

/// Render a `RenameRollbackReceipt` as the canonical JSON payload of
/// a `coding.rollback-receipt` artifact.
///
/// OWNER-LAW (2026-05-11): same `ApplyReceiptContentPolicy` shape as
/// the apply receipt — customer-release safety: `OmitContent` keeps
/// only `path` + `content_sha256` + `byte_len`.
pub fn build_rename_symbol_rollback_receipt_payload(
  receipt: &RenameRollbackReceipt,
  content_policy: ApplyReceiptContentPolicy,
) -> serde_json::Value {
  let files_after_rollback: Vec<serde_json::Value> = receipt
    .per_file_after_rollback
    .iter()
    .map(|(path, content)| {
      let mut hasher = Sha256::new();
      hasher.update(content.as_bytes());
      let hex: String = hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
      let mut entry = serde_json::json!({
        "path": path,
        "content_sha256": hex,
        "byte_len": content.len(),
      });
      if matches!(content_policy, ApplyReceiptContentPolicy::IncludeContent) {
        entry["content"] = serde_json::Value::String(content.clone());
      }
      entry
    })
    .collect();
  serde_json::json!({
    "transform": "rename-symbol",
    "owner_law": "stdlib/lib/gate/code-transform/rename-symbol.px",
    "rollback_handle_artifact_id": receipt.rollback_handle_artifact_id,
    "apply_receipt_artifact_id": receipt.handle.apply_receipt_artifact_id,
    "candidate_artifact_id": receipt.handle.candidate_artifact_id,
    "executor": {
      "actor_id": receipt.executor.actor_id,
      "tenant_id": receipt.executor.tenant_id,
    },
    "executed_at_ms": receipt.executed_at_ms,
    "target_paths": receipt.handle.target_paths,
    "content_policy": content_policy.as_str(),
    "files_after_rollback": files_after_rollback,
    "rollback_state": "executed",
    "next_step": "verify-rollback-or-redo-apply",
  })
}

/// Wrap a `RenameRollbackReceipt` into a full
/// `coding.rollback-receipt` artifact value with a replay-stable
/// id.
///
/// OWNER-LAW (2026-05-11): id hash binds the rollback *execution*
/// identity:
///
///   1. `rollback_handle_artifact_id` (which handle was executed)
///   2. `executor.actor_id` / `tenant_id`
///   3. `executed_at_ms`
///   4. Per-file (`path` + sha256 of post-rollback content) — same
///      strengthening as apply receipt id
///
/// `related_refs` carries TRIPLE back-refs: candidate, apply receipt,
/// AND rollback handle. Audit can walk the full 5-stage chain.
pub fn build_rename_symbol_rollback_receipt_artifact(
  receipt: &RenameRollbackReceipt,
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
  content_policy: ApplyReceiptContentPolicy,
) -> serde_json::Value {
  let payload = build_rename_symbol_rollback_receipt_payload(receipt, content_policy);
  let mut hasher = Sha256::new();
  hasher.update(b"rename-symbol-rollback-receipt\x1f");
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
    let mut file_h = Sha256::new();
    file_h.update(content.as_bytes());
    hasher.update(file_h.finalize());
    hasher.update(b"\x1d");
  }
  let digest = hasher.finalize();
  let prefix = digest
    .iter()
    .take(8)
    .map(|b| format!("{b:02x}"))
    .collect::<String>();
  let id = format!("rollback-receipt.rename-symbol.{prefix}");

  let mut artifact = serde_json::json!({
    "id": id,
    "artifact_family": "coding.rollback-receipt",
    "source_surface": "code-transform.rename-symbol",
    "stored_at_ms": stored_at_ms,
    "target_paths": receipt.handle.target_paths,
    "command_refs": serde_json::Value::Array(Vec::new()),
    "related_refs": serde_json::json!([
      "owner-law:stdlib/lib/gate/code-transform/rename-symbol.px",
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

/// Invert a unified diff by swapping `-` and `+` prefixes on body
/// lines. Headers (`---`, `+++`, `@@`, `\ No newline...`) pass
/// through unchanged.
///
/// OWNER-LAW (2026-05-11): rename is a symmetric whole-word
/// substitution. The inverse of "rename foo to bar" is "rename bar to
/// foo", which under our 1-line-per-hunk renderer is exactly the
/// forward diff with `-` and `+` body lines swapped.
fn invert_unified_diff(forward: &str) -> String {
  if forward.is_empty() {
    return String::new();
  }
  let mut out = String::with_capacity(forward.len());
  for line in forward.split_inclusive('\n') {
    // Headers and "no newline" markers pass through.
    if line.starts_with("--- ") || line.starts_with("+++ ") {
      // Swap the a/ and b/ prefixes so the inverse diff reads
      // naturally as "from post-apply to pre-apply".
      if let Some(stripped) = line.strip_prefix("--- a/") {
        out.push_str("+++ b/");
        out.push_str(stripped);
      } else if let Some(stripped) = line.strip_prefix("+++ b/") {
        out.push_str("--- a/");
        out.push_str(stripped);
      } else {
        out.push_str(line);
      }
      continue;
    }
    if line.starts_with("@@") || line.starts_with("\\ No newline") {
      out.push_str(line);
      continue;
    }
    // Body line: swap leading - / +.
    if let Some(stripped) = line.strip_prefix('-') {
      out.push('+');
      out.push_str(stripped);
    } else if let Some(stripped) = line.strip_prefix('+') {
      out.push('-');
      out.push_str(stripped);
    } else {
      // Context line (no leading -/+): pass through unchanged.
      out.push_str(line);
    }
  }
  out
}

/// Apply a list of edits to content, returning the rewritten string.
/// Edits must be in ascending `byte_offset` order (as produced by
/// [`compute_file_rename_edits`]) and pairwise non-overlapping.
///
/// Pure function: no I/O. Caller is responsible for deciding whether
/// to write the result back to disk (via the `ToolActionApproval`
/// gate).
pub fn apply_rename_edits(content: &str, edits: &[RenameEdit], new_name: &str) -> String {
  let mut out = String::with_capacity(content.len() + edits.len() * new_name.len());
  let bytes = content.as_bytes();
  let mut cursor = 0usize;
  for e in edits {
    // Copy verbatim up to the edit start.
    if e.byte_offset > cursor {
      out.push_str(
        std::str::from_utf8(&bytes[cursor..e.byte_offset])
          .unwrap_or_else(|_| panic!("non-utf8 input to apply_rename_edits at byte {cursor}")),
      );
    }
    out.push_str(new_name);
    cursor = e.byte_offset + e.byte_len;
  }
  if cursor < bytes.len() {
    out.push_str(
      std::str::from_utf8(&bytes[cursor..])
        .unwrap_or_else(|_| panic!("non-utf8 input to apply_rename_edits at byte {cursor}")),
    );
  }
  out
}

// ─── tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;

  fn req(old: &str, new: &str, paths: &[&str], lang: &str, scope: RenameScope) -> RenameRequest {
    RenameRequest {
      old_name: old.to_string(),
      new_name: new.to_string(),
      target_paths: paths.iter().map(|s| s.to_string()).collect(),
      language: lang.to_string(),
      scope,
      target_fn_name: None,
    }
  }

  // ─── classify_rename ──────────────────────────────────────────────

  #[test]
  fn classify_returns_ready_for_well_formed_rust_request() {
    let r = req(
      "foo",
      "bar",
      &["src/a.rs"],
      "rust",
      RenameScope::LocalTargetPaths,
    );
    assert!(matches!(classify_rename(&r), RenameVerdict::RenameReady));
  }

  #[test]
  fn classify_holds_on_missing_old_name() {
    let r = req("", "bar", &["x.rs"], "rust", RenameScope::LocalTargetPaths);
    match classify_rename(&r) {
      RenameVerdict::RenameHeld { held_kind, .. } => {
        assert_eq!(held_kind, RenameHeldKind::MissingOldName);
      }
      other => panic!("expected Held, got {:?}", other),
    }
  }

  #[test]
  fn classify_holds_on_missing_new_name() {
    let r = req("foo", "", &["x.rs"], "rust", RenameScope::LocalTargetPaths);
    match classify_rename(&r) {
      RenameVerdict::RenameHeld { held_kind, .. } => {
        assert_eq!(held_kind, RenameHeldKind::MissingNewName);
      }
      other => panic!("expected Held, got {:?}", other),
    }
  }

  #[test]
  fn classify_rejects_old_equals_new() {
    let r = req(
      "foo",
      "foo",
      &["x.rs"],
      "rust",
      RenameScope::LocalTargetPaths,
    );
    match classify_rename(&r) {
      RenameVerdict::RenameRejected { held_kind, .. } => {
        assert_eq!(held_kind, RenameHeldKind::OldNameEqualsNewName);
      }
      other => panic!("expected Rejected, got {:?}", other),
    }
  }

  #[test]
  fn classify_holds_on_invalid_identifier_old() {
    let r = req(
      "123bad",
      "good",
      &["x.rs"],
      "rust",
      RenameScope::LocalTargetPaths,
    );
    match classify_rename(&r) {
      RenameVerdict::RenameHeld { held_kind, .. } => {
        assert_eq!(held_kind, RenameHeldKind::InvalidIdentifier);
      }
      other => panic!("expected Held, got {:?}", other),
    }
  }

  #[test]
  fn classify_holds_on_invalid_identifier_new() {
    let r = req(
      "good",
      "has space",
      &["x.rs"],
      "rust",
      RenameScope::LocalTargetPaths,
    );
    match classify_rename(&r) {
      RenameVerdict::RenameHeld { held_kind, .. } => {
        assert_eq!(held_kind, RenameHeldKind::InvalidIdentifier);
      }
      other => panic!("expected Held, got {:?}", other),
    }
  }

  #[test]
  fn classify_holds_on_unsupported_language() {
    let r = req(
      "foo",
      "bar",
      &["x.f90"],
      "fortran",
      RenameScope::LocalTargetPaths,
    );
    match classify_rename(&r) {
      RenameVerdict::RenameHeld { held_kind, .. } => {
        assert_eq!(held_kind, RenameHeldKind::LanguageNotSupported);
      }
      other => panic!("expected Held, got {:?}", other),
    }
  }

  #[test]
  fn classify_holds_on_workspace_wide_scope() {
    let r = req("foo", "bar", &["x.rs"], "rust", RenameScope::WorkspaceWide);
    match classify_rename(&r) {
      RenameVerdict::RenameHeld { held_kind, .. } => {
        assert_eq!(held_kind, RenameHeldKind::ScopeTooBroad);
      }
      other => panic!("expected Held, got {:?}", other),
    }
  }

  #[test]
  fn classify_holds_on_empty_target_paths() {
    let r = req("foo", "bar", &[], "rust", RenameScope::LocalTargetPaths);
    match classify_rename(&r) {
      RenameVerdict::RenameHeld { held_kind, .. } => {
        assert_eq!(held_kind, RenameHeldKind::TargetPathEmpty);
      }
      other => panic!("expected Held, got {:?}", other),
    }
  }

  #[test]
  fn classify_holds_on_parent_traversal_in_path() {
    let r = req(
      "foo",
      "bar",
      &["../escape.rs"],
      "rust",
      RenameScope::LocalTargetPaths,
    );
    match classify_rename(&r) {
      RenameVerdict::RenameHeld { held_kind, .. } => {
        assert_eq!(held_kind, RenameHeldKind::TargetPathOutOfProject);
      }
      other => panic!("expected Held, got {:?}", other),
    }
  }

  #[test]
  fn classify_holds_on_empty_path_in_list() {
    let r = req(
      "foo",
      "bar",
      &["src/a.rs", ""],
      "rust",
      RenameScope::LocalTargetPaths,
    );
    match classify_rename(&r) {
      RenameVerdict::RenameHeld { held_kind, .. } => {
        assert_eq!(held_kind, RenameHeldKind::TargetPathOutOfProject);
      }
      other => panic!("expected Held, got {:?}", other),
    }
  }

  #[test]
  fn classify_allows_crate_wide_scope() {
    let r = req("foo", "bar", &["src/a.rs"], "rust", RenameScope::CrateWide);
    assert!(matches!(classify_rename(&r), RenameVerdict::RenameReady));
  }

  // ─── compute_file_rename_edits ────────────────────────────────────

  #[test]
  fn rename_finds_whole_word_match() {
    let content = "let foo = 1;\nfoo()\n";
    let edits = compute_file_rename_edits("foo", "bar", content);
    assert_eq!(edits.len(), 2);
    assert_eq!(edits[0].byte_offset, 4);
    assert_eq!(edits[0].line, 1);
    assert_eq!(edits[0].column, 5);
    assert_eq!(edits[1].byte_offset, 13);
    assert_eq!(edits[1].line, 2);
    assert_eq!(edits[1].column, 1);
  }

  #[test]
  fn rename_does_not_match_inside_longer_identifier() {
    // `foo` must not match inside `foobar`, `foo_bar`, `bar_foo`,
    // `_foo_`.
    let content = "foobar foo_bar bar_foo _foo_ foo";
    let edits = compute_file_rename_edits("foo", "x", content);
    // Only the trailing standalone `foo` at the end matches.
    assert_eq!(edits.len(), 1, "expected exactly one whole-word match");
    assert_eq!(
      &content[edits[0].byte_offset..edits[0].byte_offset + edits[0].byte_len],
      "foo"
    );
  }

  #[test]
  fn rename_matches_at_string_start_and_end() {
    let content = "foo";
    let edits = compute_file_rename_edits("foo", "bar", content);
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].byte_offset, 0);
    assert_eq!(edits[0].line, 1);
    assert_eq!(edits[0].column, 1);
  }

  #[test]
  fn rename_returns_empty_when_no_match() {
    let content = "let a = 1;";
    let edits = compute_file_rename_edits("foo", "bar", content);
    assert!(edits.is_empty());
  }

  #[test]
  fn rename_returns_empty_for_empty_old_name() {
    let content = "anything goes here";
    let edits = compute_file_rename_edits("", "bar", content);
    assert!(edits.is_empty());
  }

  #[test]
  fn rename_handles_punctuation_around_identifier() {
    let content = "fn foo() { foo(); foo,foo:foo; }";
    let edits = compute_file_rename_edits("foo", "bar", content);
    // 5 occurrences: fn foo, { foo(), foo,, ,foo:, :foo;
    assert_eq!(edits.len(), 5);
  }

  #[test]
  fn rename_line_column_tracking_across_multiple_lines() {
    let content = "a\nfoo\nbb foo cc\nfoo";
    let edits = compute_file_rename_edits("foo", "bar", content);
    assert_eq!(edits.len(), 3);
    // Line 2, col 1
    assert_eq!((edits[0].line, edits[0].column), (2, 1));
    // Line 3, col 4
    assert_eq!((edits[1].line, edits[1].column), (3, 4));
    // Line 4, col 1
    assert_eq!((edits[2].line, edits[2].column), (4, 1));
  }

  // ─── apply_rename_edits ───────────────────────────────────────────

  #[test]
  fn apply_edits_produces_expected_rewrite() {
    let content = "let foo = 1;\nfoo()\n";
    let edits = compute_file_rename_edits("foo", "bar", content);
    let rewritten = apply_rename_edits(content, &edits, "bar");
    assert_eq!(rewritten, "let bar = 1;\nbar()\n");
  }

  #[test]
  fn apply_edits_preserves_non_matching_content() {
    let content = "foobar foo baz";
    let edits = compute_file_rename_edits("foo", "X", content);
    let rewritten = apply_rename_edits(content, &edits, "X");
    assert_eq!(
      rewritten, "foobar X baz",
      "must not touch `foobar` or `baz`"
    );
  }

  #[test]
  fn apply_edits_on_empty_edit_list_returns_unchanged_content() {
    let content = "no matches here";
    let rewritten = apply_rename_edits(content, &[], "anything");
    assert_eq!(rewritten, content);
  }

  #[test]
  fn round_trip_compute_then_apply_then_recompute_finds_nothing_with_old_name() {
    // The most important deterministic invariant: after applying the
    // rewrite, scanning the result for `old_name` (whole-word) yields
    // no matches.
    let content = "fn foo() { foo(); foo,foo:foo; }";
    let edits = compute_file_rename_edits("foo", "bar", content);
    let rewritten = apply_rename_edits(content, &edits, "bar");
    let post_edits = compute_file_rename_edits("foo", "X", &rewritten);
    assert!(
      post_edits.is_empty(),
      "post-rewrite, `foo` must not appear as a whole word; got {} stragglers",
      post_edits.len()
    );
  }

  // ─── unified diff + patch candidate ───────────────────────────────

  #[test]
  fn render_unified_diff_empty_on_zero_edits() {
    let diff = render_unified_diff_for_rename("a.rs", "let x = 1;\n", &[], "bar");
    assert!(diff.is_empty());
  }

  #[test]
  fn render_unified_diff_emits_per_line_hunks() {
    let content = "let foo = 1;\nfoo()\n";
    let edits = compute_file_rename_edits("foo", "bar", content);
    let diff = render_unified_diff_for_rename("src/a.rs", content, &edits, "bar");
    // Two changed lines → two hunks.
    let hunk_count = diff.matches("@@ ").count();
    assert_eq!(
      hunk_count, 2,
      "expected 2 hunks for 2 modified lines; got:\n{diff}"
    );
    // Header is present.
    assert!(diff.starts_with("--- a/src/a.rs\n+++ b/src/a.rs\n"));
    // Each modified line appears as `-old` and `+new`.
    assert!(diff.contains("-let foo = 1;\n"));
    assert!(diff.contains("+let bar = 1;\n"));
    assert!(diff.contains("-foo()\n"));
    assert!(diff.contains("+bar()\n"));
  }

  #[test]
  fn render_unified_diff_handles_no_trailing_newline() {
    let content = "fn foo() {}"; // no trailing newline
    let edits = compute_file_rename_edits("foo", "bar", content);
    let diff = render_unified_diff_for_rename("a.rs", content, &edits, "bar");
    // git-style "no newline at end of file" marker must appear after
    // both the `-` and `+` lines.
    let no_newline_count = diff.matches("\\ No newline at end of file").count();
    assert_eq!(
      no_newline_count, 2,
      "expected 2 \"no newline\" markers (one each for - and +); got:\n{diff}"
    );
  }

  #[test]
  fn render_unified_diff_only_emits_changed_lines() {
    // Multi-line content where only some lines have the identifier.
    let content = "let a = 1;\nlet foo = 2;\nlet b = 3;\n";
    let edits = compute_file_rename_edits("foo", "bar", content);
    let diff = render_unified_diff_for_rename("a.rs", content, &edits, "bar");
    let hunk_count = diff.matches("@@ ").count();
    assert_eq!(
      hunk_count, 1,
      "only line 2 changed; expected 1 hunk:\n{diff}"
    );
    assert!(diff.contains("@@ -2,1 +2,1 @@"));
    // Unchanged lines must not appear in the diff body.
    assert!(!diff.contains("-let a = 1;"));
    assert!(!diff.contains("-let b = 3;"));
  }

  #[test]
  fn compute_rename_patch_candidate_happy_path() {
    let req = req(
      "foo",
      "bar",
      &["src/a.rs", "src/b.rs"],
      "rust",
      RenameScope::LocalTargetPaths,
    );
    let a = RenameFileInput {
      path: "src/a.rs",
      content: "fn foo() { foo() }",
    };
    let b = RenameFileInput {
      path: "src/b.rs",
      content: "let x = 1;\n", // no `foo` anywhere
    };
    let candidate = compute_rename_patch_candidate(&req, &[a, b]);
    assert!(matches!(candidate.verdict, RenameVerdict::RenameReady));
    // Only src/a.rs had matches → exactly one file_patch.
    assert_eq!(candidate.file_patches.len(), 1);
    assert_eq!(candidate.file_patches[0].path, "src/a.rs");
    assert_eq!(candidate.file_patches[0].edits.len(), 2);
    assert!(!candidate.combined_unified_diff.is_empty());
    assert!(candidate.combined_unified_diff.contains("--- a/src/a.rs"));
    assert!(!candidate.combined_unified_diff.contains("--- a/src/b.rs"));
  }

  #[test]
  fn compute_rename_patch_candidate_held_request_produces_empty_patches() {
    // Held verdict → no edits computed, no diffs rendered.
    let req = req("", "bar", &["x.rs"], "rust", RenameScope::LocalTargetPaths);
    let f = RenameFileInput {
      path: "x.rs",
      content: "foo",
    };
    let candidate = compute_rename_patch_candidate(&req, &[f]);
    assert!(matches!(
      candidate.verdict,
      RenameVerdict::RenameHeld { .. }
    ));
    assert!(candidate.file_patches.is_empty());
    assert!(candidate.combined_unified_diff.is_empty());
  }

  #[test]
  fn compute_rename_patch_candidate_rejected_request_produces_empty_patches() {
    let req = req(
      "foo",
      "foo",
      &["x.rs"],
      "rust",
      RenameScope::LocalTargetPaths,
    );
    let f = RenameFileInput {
      path: "x.rs",
      content: "foo",
    };
    let candidate = compute_rename_patch_candidate(&req, &[f]);
    assert!(matches!(
      candidate.verdict,
      RenameVerdict::RenameRejected { .. }
    ));
    assert!(candidate.file_patches.is_empty());
  }

  #[test]
  fn build_payload_carries_canonical_artifact_fields() {
    let req = req(
      "foo",
      "bar",
      &["src/a.rs"],
      "rust",
      RenameScope::LocalTargetPaths,
    );
    let f = RenameFileInput {
      path: "src/a.rs",
      content: "fn foo() {}",
    };
    let candidate = compute_rename_patch_candidate(&req, &[f]);
    let payload = build_rename_symbol_patch_candidate_payload(&candidate);
    assert_eq!(payload["transform"].as_str(), Some("rename-symbol"));
    assert_eq!(
      payload["owner_law"].as_str(),
      Some("stdlib/lib/gate/code-transform/rename-symbol.px")
    );
    assert_eq!(payload["old_name"].as_str(), Some("foo"));
    assert_eq!(payload["new_name"].as_str(), Some("bar"));
    assert_eq!(payload["language"].as_str(), Some("rust"));
    assert_eq!(payload["scope"].as_str(), Some("local-target-paths"));
    assert_eq!(payload["verdict"].as_str(), Some("rename-ready"));
    assert_eq!(payload["candidate_only"].as_bool(), Some(true));
    assert_eq!(
      payload["next_step"].as_str(),
      Some("host-cst-rewrite-then-tool-action-approval")
    );
    let edits = payload["edits"].as_array().expect("edits array");
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0]["path"].as_str(), Some("src/a.rs"));
    assert_eq!(edits[0]["byte_len"].as_u64(), Some(3));
    assert!(payload["unified_diff"]
      .as_str()
      .expect("unified_diff string")
      .contains("--- a/src/a.rs"));
  }

  #[test]
  fn build_payload_held_carries_held_kind_and_reason() {
    let req = req("", "bar", &["x.rs"], "rust", RenameScope::LocalTargetPaths);
    let candidate = compute_rename_patch_candidate(&req, &[]);
    let payload = build_rename_symbol_patch_candidate_payload(&candidate);
    assert_eq!(payload["verdict"].as_str(), Some("rename-held"));
    assert_eq!(payload["held_kind"].as_str(), Some("missing-old-name"));
    assert!(payload["reason"].as_str().unwrap().contains("old_name"));
    // No edits emitted on Held.
    assert_eq!(payload["edits"].as_array().unwrap().len(), 0);
    assert_eq!(payload["unified_diff"].as_str(), Some(""));
  }

  // ─── strict preflight ─────────────────────────────────────────────

  // ─── target_paths boundary (security-critical) ───────────────────

  #[test]
  fn patch_candidate_ignores_files_outside_target_paths() {
    // SAFETY: `compute_rename_patch_candidate` must never touch files
    // outside `request.target_paths`, even if they are present in the
    // staged input. A caller (e.g. a doghouse ingest layer) may hand
    // in a wider staging set than the explicit rename request; the
    // carrier filters down.
    let req = req(
      "foo",
      "bar",
      &["src/a.rs"],
      "rust",
      RenameScope::LocalTargetPaths,
    );
    let target = RenameFileInput {
      path: "src/a.rs",
      content: "fn foo() { foo() }\n",
    };
    let stray = RenameFileInput {
      path: "src/secret.rs",
      content: "fn foo() { foo() }\n",
    };
    let candidate = compute_rename_patch_candidate(&req, &[target, stray]);
    assert!(matches!(candidate.verdict, RenameVerdict::RenameReady));
    assert_eq!(
      candidate.file_patches.len(),
      1,
      "exactly one file_patch (the target) — stray file must be excluded"
    );
    assert_eq!(candidate.file_patches[0].path, "src/a.rs");
    assert!(
      !candidate.combined_unified_diff.contains("src/secret.rs"),
      "stray file path must not appear in combined_unified_diff"
    );
    assert!(
      !candidate
        .combined_unified_diff
        .contains("--- a/src/secret.rs"),
      "stray file diff header must not be present"
    );
  }

  #[test]
  fn strict_preflight_also_ignores_files_outside_target_paths() {
    // The strict wrapper inherits the filter from the base.
    let req = req(
      "foo",
      "bar",
      &["src/a.rs"],
      "rust",
      RenameScope::LocalTargetPaths,
    );
    let target = RenameFileInput {
      path: "src/a.rs",
      content: "fn foo() {}\n",
    };
    let stray = RenameFileInput {
      path: "src/secret.rs",
      content: "fn foo() {}\n",
    };
    let candidate = compute_rename_patch_candidate_strict(&req, &[target, stray]);
    assert!(matches!(candidate.verdict, RenameVerdict::RenameReady));
    assert_eq!(candidate.file_patches.len(), 1);
    assert_eq!(candidate.file_patches[0].path, "src/a.rs");
  }

  #[test]
  fn artifact_payload_paths_subset_of_request_target_paths() {
    // Stronger invariant: every path that appears in the artifact
    // payload's `edits[*].path` must be a member of
    // `request.target_paths`. This is the "no escape" check that
    // downstream consumers (doghouse, freecat-cli) can rely on.
    let req = req(
      "foo",
      "bar",
      &["src/a.rs", "src/b.rs"],
      "rust",
      RenameScope::LocalTargetPaths,
    );
    let a = RenameFileInput {
      path: "src/a.rs",
      content: "fn foo() {}\n",
    };
    let b = RenameFileInput {
      path: "src/b.rs",
      content: "fn foo() {}\n",
    };
    let stray = RenameFileInput {
      path: "src/elsewhere.rs",
      content: "fn foo() {}\n",
    };
    let candidate = compute_rename_patch_candidate(&req, &[a, b, stray]);
    let payload = build_rename_symbol_patch_candidate_payload(&candidate);
    let edits = payload["edits"].as_array().expect("edits array");
    let target_set: std::collections::BTreeSet<_> =
      req.target_paths.iter().map(|s| s.as_str()).collect();
    for edit in edits {
      let path = edit["path"].as_str().expect("edit.path");
      assert!(
        target_set.contains(path),
        "edit path {path} must be in target_paths {target_set:?}"
      );
    }
  }

  #[test]
  fn strict_preflight_classify_verdict_wins_over_dedup() {
    // Verdict ordering: request-shape ladder runs first. Even if the
    // staged input has duplicate target paths, a malformed request
    // shape (e.g. missing old_name) must surface as
    // MissingOldName, not DuplicateTargetPathContent. The cockpit
    // gets the most precise verdict for the operator to fix.
    let req = req(
      "",
      "bar",
      &["src/a.rs"],
      "rust",
      RenameScope::LocalTargetPaths,
    );
    let dup1 = RenameFileInput {
      path: "src/a.rs",
      content: "fn x() {}",
    };
    let dup2 = RenameFileInput {
      path: "src/a.rs",
      content: "fn y() {}",
    };
    let candidate = compute_rename_patch_candidate_strict(&req, &[dup1, dup2]);
    match &candidate.verdict {
      RenameVerdict::RenameHeld { held_kind, .. } => {
        assert_eq!(
          *held_kind,
          RenameHeldKind::MissingOldName,
          "request-shape error must win over host-level dedup; got {:?}",
          held_kind
        );
      }
      other => panic!("expected Held(MissingOldName), got {:?}", other),
    }
  }

  #[test]
  fn strict_preflight_classify_verdict_wins_over_other_strict_checks() {
    // Same ordering invariant for the other strict checks: a
    // malformed request never produces TargetPathContentMissing /
    // OldNameNotFound / NameCollisionDetected.
    let req = req(
      "123bad", // invalid identifier
      "bar",
      &[], // also empty target_paths (S1 would fire)
      "rust",
      RenameScope::LocalTargetPaths,
    );
    let candidate = compute_rename_patch_candidate_strict(&req, &[]);
    match &candidate.verdict {
      RenameVerdict::RenameHeld { held_kind, .. } => {
        // classify sees `123bad` first (InvalidIdentifier comes before
        // TargetPathEmpty in the .px ladder), so that verdict wins.
        assert_eq!(*held_kind, RenameHeldKind::InvalidIdentifier);
      }
      other => panic!("expected Held(InvalidIdentifier), got {:?}", other),
    }
  }

  #[test]
  fn strict_preflight_holds_on_duplicate_target_path_content() {
    // Same target path appears twice in staged input with different
    // content → ambiguous → Held.
    let req = req(
      "foo",
      "bar",
      &["src/a.rs"],
      "rust",
      RenameScope::LocalTargetPaths,
    );
    let v1 = RenameFileInput {
      path: "src/a.rs",
      content: "fn foo() { /* v1 */ }",
    };
    let v2 = RenameFileInput {
      path: "src/a.rs",
      content: "fn foo() { /* v2 */ }",
    };
    let candidate = compute_rename_patch_candidate_strict(&req, &[v1, v2]);
    match &candidate.verdict {
      RenameVerdict::RenameHeld { held_kind, reason } => {
        assert_eq!(*held_kind, RenameHeldKind::DuplicateTargetPathContent);
        assert!(reason.contains("src/a.rs"));
      }
      other => panic!("expected Held(DuplicateTargetPathContent), got {:?}", other),
    }
    assert!(candidate.file_patches.is_empty());
  }

  #[test]
  fn strict_preflight_holds_on_duplicate_even_when_contents_are_identical() {
    // Even identical duplicates are ambiguous — replay should reject
    // input shape that has no canonical interpretation.
    let req = req(
      "foo",
      "bar",
      &["src/a.rs"],
      "rust",
      RenameScope::LocalTargetPaths,
    );
    let same = RenameFileInput {
      path: "src/a.rs",
      content: "fn foo() {}",
    };
    let same_again = RenameFileInput {
      path: "src/a.rs",
      content: "fn foo() {}",
    };
    let candidate = compute_rename_patch_candidate_strict(&req, &[same, same_again]);
    assert!(matches!(
      candidate.verdict,
      RenameVerdict::RenameHeld {
        held_kind: RenameHeldKind::DuplicateTargetPathContent,
        ..
      }
    ));
  }

  #[test]
  fn strict_preflight_ignores_duplicate_when_path_is_not_a_target() {
    // Same non-target path appears twice — irrelevant. The dedup
    // check is scoped to `request.target_paths`; stray duplicates
    // outside the request are out of scope.
    let req = req(
      "foo",
      "bar",
      &["src/a.rs"],
      "rust",
      RenameScope::LocalTargetPaths,
    );
    let target = RenameFileInput {
      path: "src/a.rs",
      content: "fn foo() {}",
    };
    let stray_a = RenameFileInput {
      path: "src/elsewhere.rs",
      content: "fn foo() {}",
    };
    let stray_b = RenameFileInput {
      path: "src/elsewhere.rs",
      content: "fn foo() {}",
    };
    let candidate = compute_rename_patch_candidate_strict(&req, &[target, stray_a, stray_b]);
    assert!(matches!(candidate.verdict, RenameVerdict::RenameReady));
    assert_eq!(candidate.file_patches.len(), 1);
    assert_eq!(candidate.file_patches[0].path, "src/a.rs");
  }

  #[test]
  fn strict_preflight_holds_when_target_path_not_staged() {
    let req = req(
      "foo",
      "bar",
      &["src/a.rs", "src/missing.rs"],
      "rust",
      RenameScope::LocalTargetPaths,
    );
    let only_a = RenameFileInput {
      path: "src/a.rs",
      content: "fn foo() {}",
    };
    let candidate = compute_rename_patch_candidate_strict(&req, &[only_a]);
    match &candidate.verdict {
      RenameVerdict::RenameHeld { held_kind, reason } => {
        assert_eq!(*held_kind, RenameHeldKind::TargetPathContentMissing);
        assert!(reason.contains("src/missing.rs"));
      }
      other => panic!("expected Held(TargetPathContentMissing), got {:?}", other),
    }
    assert!(candidate.file_patches.is_empty());
  }

  #[test]
  fn strict_preflight_holds_when_old_name_has_no_occurrences() {
    let req = req(
      "foo",
      "bar",
      &["src/a.rs"],
      "rust",
      RenameScope::LocalTargetPaths,
    );
    let no_foo = RenameFileInput {
      path: "src/a.rs",
      content: "let x = 1;\nfn quux() {}\n",
    };
    let candidate = compute_rename_patch_candidate_strict(&req, &[no_foo]);
    match &candidate.verdict {
      RenameVerdict::RenameHeld { held_kind, reason } => {
        assert_eq!(*held_kind, RenameHeldKind::OldNameNotFound);
        assert!(reason.contains("foo"));
      }
      other => panic!("expected Held(OldNameNotFound), got {:?}", other),
    }
  }

  #[test]
  fn strict_preflight_holds_on_new_name_collision() {
    // `bar` is already present as a standalone identifier in the file
    // → renaming `foo` to `bar` would collide.
    let req = req(
      "foo",
      "bar",
      &["src/a.rs"],
      "rust",
      RenameScope::LocalTargetPaths,
    );
    let f = RenameFileInput {
      path: "src/a.rs",
      content: "fn foo() {}\nfn bar() {}\n", // both names exist
    };
    let candidate = compute_rename_patch_candidate_strict(&req, &[f]);
    match &candidate.verdict {
      RenameVerdict::RenameHeld { held_kind, reason } => {
        assert_eq!(*held_kind, RenameHeldKind::NameCollisionDetected);
        assert!(reason.contains("bar"));
        assert!(reason.contains("src/a.rs"));
      }
      other => panic!("expected Held(NameCollisionDetected), got {:?}", other),
    }
  }

  #[test]
  fn strict_preflight_does_not_falsely_trigger_collision_on_substring_match() {
    // `barometer` contains the substring "bar" but is not a standalone
    // identifier `bar` — the whole-word check should not flag it.
    let req = req(
      "foo",
      "bar",
      &["src/a.rs"],
      "rust",
      RenameScope::LocalTargetPaths,
    );
    let f = RenameFileInput {
      path: "src/a.rs",
      content: "fn foo() { let barometer = 1; }\n",
    };
    let candidate = compute_rename_patch_candidate_strict(&req, &[f]);
    assert!(matches!(candidate.verdict, RenameVerdict::RenameReady));
    assert_eq!(candidate.file_patches.len(), 1);
  }

  #[test]
  fn strict_preflight_passes_clean_case_with_all_inputs_aligned() {
    let req = req(
      "foo",
      "renamed_foo",
      &["src/a.rs"],
      "rust",
      RenameScope::LocalTargetPaths,
    );
    let f = RenameFileInput {
      path: "src/a.rs",
      content: "fn foo() { foo() }\n",
    };
    let candidate = compute_rename_patch_candidate_strict(&req, &[f]);
    assert!(matches!(candidate.verdict, RenameVerdict::RenameReady));
    assert_eq!(candidate.file_patches.len(), 1);
    assert_eq!(candidate.file_patches[0].edits.len(), 2);
  }

  #[test]
  fn strict_preflight_passes_through_classify_held_verdicts() {
    // If classify_rename already holds (e.g. missing old_name), strict
    // preflight returns the classify verdict unchanged — the
    // file-content checks don't run.
    let req = req(
      "",
      "bar",
      &["src/a.rs"],
      "rust",
      RenameScope::LocalTargetPaths,
    );
    let f = RenameFileInput {
      path: "src/a.rs",
      content: "fn foo() {}",
    };
    let candidate = compute_rename_patch_candidate_strict(&req, &[f]);
    match &candidate.verdict {
      RenameVerdict::RenameHeld { held_kind, .. } => {
        assert_eq!(*held_kind, RenameHeldKind::MissingOldName);
      }
      other => panic!("expected Held(MissingOldName), got {:?}", other),
    }
  }

  // ─── artifact builder ─────────────────────────────────────────────

  #[test]
  fn artifact_builder_produces_canonical_envelope_shape() {
    let req = req(
      "foo",
      "bar",
      &["src/a.rs"],
      "rust",
      RenameScope::LocalTargetPaths,
    );
    let f = RenameFileInput {
      path: "src/a.rs",
      content: "fn foo() {}",
    };
    let candidate = compute_rename_patch_candidate(&req, &[f]);
    let art = build_rename_symbol_patch_candidate_artifact(&candidate, 1700000000000, None);
    assert_eq!(
      art["artifact_family"].as_str(),
      Some("coding.generated-patch-candidate")
    );
    assert_eq!(
      art["source_surface"].as_str(),
      Some("code-transform.rename-symbol")
    );
    assert_eq!(art["stored_at_ms"].as_u64(), Some(1700000000000));
    // target_paths preserved.
    let tps = art["target_paths"].as_array().expect("target_paths array");
    assert_eq!(tps[0].as_str(), Some("src/a.rs"));
    // related_refs has the owner-law tag.
    let rrs = art["related_refs"].as_array().expect("related_refs array");
    assert!(rrs.iter().any(|v| v
      .as_str()
      .map(|s| s == "owner-law:stdlib/lib/gate/code-transform/rename-symbol.px")
      .unwrap_or(false)));
    // id is replay-stable shape.
    let id = art["id"].as_str().expect("id string");
    assert!(id.starts_with("generated-patch.rename-symbol."));
    // Inner payload is reachable.
    assert_eq!(art["payload"]["transform"].as_str(), Some("rename-symbol"));
  }

  #[test]
  fn artifact_id_is_replay_stable_for_same_inputs() {
    let req = req(
      "foo",
      "bar",
      &["src/a.rs"],
      "rust",
      RenameScope::LocalTargetPaths,
    );
    let f = RenameFileInput {
      path: "src/a.rs",
      content: "fn foo() { foo() }",
    };
    let c1 = compute_rename_patch_candidate(&req, &[f.clone()]);
    let c2 = compute_rename_patch_candidate(&req, &[f]);
    // Different timestamps must not change the id.
    let a1 = build_rename_symbol_patch_candidate_artifact(&c1, 1700000000000, None);
    let a2 = build_rename_symbol_patch_candidate_artifact(&c2, 1900000000000, None);
    assert_eq!(a1["id"], a2["id"], "same inputs must yield same id");
    assert_ne!(a1["stored_at_ms"], a2["stored_at_ms"]);
  }

  #[test]
  fn artifact_id_differs_for_different_inputs() {
    let req_a = req(
      "foo",
      "bar",
      &["src/a.rs"],
      "rust",
      RenameScope::LocalTargetPaths,
    );
    let req_b = req(
      "foo",
      "baz",
      &["src/a.rs"],
      "rust",
      RenameScope::LocalTargetPaths,
    );
    let f = RenameFileInput {
      path: "src/a.rs",
      content: "fn foo() {}",
    };
    let a = build_rename_symbol_patch_candidate_artifact(
      &compute_rename_patch_candidate(&req_a, &[f.clone()]),
      0,
      None,
    );
    let b = build_rename_symbol_patch_candidate_artifact(
      &compute_rename_patch_candidate(&req_b, &[f]),
      0,
      None,
    );
    assert_ne!(a["id"], b["id"]);
  }

  #[test]
  fn artifact_includes_repo_snapshot_ref_when_provided() {
    let req = req(
      "foo",
      "bar",
      &["src/a.rs"],
      "rust",
      RenameScope::LocalTargetPaths,
    );
    let f = RenameFileInput {
      path: "src/a.rs",
      content: "fn foo() {}",
    };
    let candidate = compute_rename_patch_candidate(&req, &[f]);
    let art =
      build_rename_symbol_patch_candidate_artifact(&candidate, 1700000000000, Some("abc123def456"));
    assert_eq!(art["repo_snapshot_ref"].as_str(), Some("abc123def456"));
  }

  #[test]
  fn artifact_id_differs_when_surrounding_content_differs() {
    // OWNER-LAW (2026-05-11): same request, same edit offsets, but
    // *different surrounding line content* must yield different ids.
    // Without unified_diff in the hash, both files would hash to the
    // same offsets and produce the same id even though the resulting
    // patches are semantically different.
    let req = req(
      "foo",
      "bar",
      &["src/a.rs"],
      "rust",
      RenameScope::LocalTargetPaths,
    );
    // Two contents where `foo` starts at the same byte offset but the
    // rest of the line is different. Both produce one edit at offset
    // 3 (after "fn ") but the unified diff lines differ.
    let f1 = RenameFileInput {
      path: "src/a.rs",
      content: "fn foo() { /* path one */ }\n",
    };
    let f2 = RenameFileInput {
      path: "src/a.rs",
      content: "fn foo() { /* path two */ }\n",
    };
    let c1 = compute_rename_patch_candidate(&req, &[f1]);
    let c2 = compute_rename_patch_candidate(&req, &[f2]);
    // Sanity: same offset+len
    assert_eq!(
      c1.file_patches[0].edits[0].byte_offset,
      c2.file_patches[0].edits[0].byte_offset
    );
    assert_eq!(
      c1.file_patches[0].edits[0].byte_len,
      c2.file_patches[0].edits[0].byte_len
    );
    // But the surrounding line content differs → diff bytes differ →
    // id must differ.
    assert_ne!(
      c1.file_patches[0].unified_diff,
      c2.file_patches[0].unified_diff
    );
    let a1 = build_rename_symbol_patch_candidate_artifact(&c1, 0, None);
    let a2 = build_rename_symbol_patch_candidate_artifact(&c2, 0, None);
    assert_ne!(
      a1["id"], a2["id"],
      "different surrounding content must produce different artifact id"
    );
  }

  #[test]
  fn artifact_omits_repo_snapshot_ref_when_not_provided() {
    let req = req(
      "foo",
      "bar",
      &["src/a.rs"],
      "rust",
      RenameScope::LocalTargetPaths,
    );
    let f = RenameFileInput {
      path: "src/a.rs",
      content: "fn foo() {}",
    };
    let candidate = compute_rename_patch_candidate(&req, &[f]);
    let art = build_rename_symbol_patch_candidate_artifact(&candidate, 1700000000000, None);
    assert!(art.get("repo_snapshot_ref").is_none());
  }

  #[test]
  fn build_payload_rejected_carries_held_kind_and_reason() {
    let req = req(
      "foo",
      "foo",
      &["x.rs"],
      "rust",
      RenameScope::LocalTargetPaths,
    );
    let candidate = compute_rename_patch_candidate(&req, &[]);
    let payload = build_rename_symbol_patch_candidate_payload(&candidate);
    assert_eq!(payload["verdict"].as_str(), Some("rename-rejected"));
    assert_eq!(
      payload["held_kind"].as_str(),
      Some("old-name-equals-new-name")
    );
  }

  // ─── apply lane (sealed candidate → approval → receipt) ───────────

  fn fixture_ready_candidate() -> (
    RenameRequest,
    Vec<RenameFileInput<'static>>,
    RenamePatchCandidate,
  ) {
    let req = RenameRequest {
      old_name: "foo".to_string(),
      new_name: "bar".to_string(),
      target_paths: vec!["src/a.rs".to_string()],
      language: "rust".to_string(),
      scope: RenameScope::LocalTargetPaths,
      target_fn_name: None,
    };
    let files: Vec<RenameFileInput<'static>> = vec![RenameFileInput {
      path: "src/a.rs",
      content: "fn foo() { foo() }\n",
    }];
    let candidate = compute_rename_patch_candidate(&req, &files);
    (req, files, candidate)
  }

  fn fixture_approval(candidate: &RenamePatchCandidate) -> RenameApplyApproval {
    let art = build_rename_symbol_patch_candidate_artifact(candidate, 0, None);
    RenameApplyApproval {
      actor_id: "actor.user.1".to_string(),
      tenant_id: "tenant.alpha".to_string(),
      approved_at_ms: 1700000000000,
      candidate_artifact_id: art["id"].as_str().expect("id").to_string(),
    }
  }

  #[test]
  fn validated_rename_patch_candidate_accepts_ready() {
    let (_, _, candidate) = fixture_ready_candidate();
    assert!(ValidatedRenamePatchCandidate::new_checked(candidate).is_ok());
  }

  #[test]
  fn validated_rename_patch_candidate_rejects_held() {
    // Build a Held candidate by passing an empty old_name.
    let req = RenameRequest {
      old_name: "".to_string(),
      new_name: "bar".to_string(),
      target_paths: vec!["src/a.rs".to_string()],
      language: "rust".to_string(),
      scope: RenameScope::LocalTargetPaths,
      target_fn_name: None,
    };
    let candidate = compute_rename_patch_candidate(&req, &[]);
    let result = ValidatedRenamePatchCandidate::new_checked(candidate);
    assert!(result.is_err(), "Held candidate must not seal");
    // The original candidate is returned for audit.
    let returned = result.unwrap_err();
    assert!(matches!(returned.verdict, RenameVerdict::RenameHeld { .. }));
  }

  #[test]
  fn apply_rename_patch_candidate_happy_path() {
    let (_req, files, candidate) = fixture_ready_candidate();
    let approval = fixture_approval(&candidate);
    let sealed = ValidatedRenamePatchCandidate::new_checked(candidate).expect("ready");
    let receipt = apply_rename_patch_candidate(&sealed, &files, &approval, 1700000000999)
      .expect("apply succeeds");
    assert_eq!(receipt.applied_at_ms, 1700000000999);
    assert_eq!(receipt.per_file_after.len(), 1);
    assert_eq!(receipt.per_file_after[0].0, "src/a.rs");
    assert_eq!(receipt.per_file_after[0].1, "fn bar() { bar() }\n");
    assert!(!receipt.inverse_unified_diff.is_empty());
  }

  #[test]
  fn apply_rename_patch_candidate_rejects_actor_empty() {
    let (_req, files, candidate) = fixture_ready_candidate();
    let mut approval = fixture_approval(&candidate);
    approval.actor_id = "".to_string();
    let sealed = ValidatedRenamePatchCandidate::new_checked(candidate).expect("ready");
    let err = apply_rename_patch_candidate(&sealed, &files, &approval, 0)
      .expect_err("empty actor must fail");
    assert!(matches!(err, RenameApplyError::MissingApprovalActor));
  }

  #[test]
  fn apply_rename_patch_candidate_rejects_tenant_empty() {
    let (_req, files, candidate) = fixture_ready_candidate();
    let mut approval = fixture_approval(&candidate);
    approval.tenant_id = "".to_string();
    let sealed = ValidatedRenamePatchCandidate::new_checked(candidate).expect("ready");
    let err = apply_rename_patch_candidate(&sealed, &files, &approval, 0)
      .expect_err("empty tenant must fail");
    assert!(matches!(err, RenameApplyError::MissingApprovalTenant));
  }

  #[test]
  fn apply_rename_patch_candidate_rejects_candidate_id_mismatch() {
    let (_req, files, candidate) = fixture_ready_candidate();
    let mut approval = fixture_approval(&candidate);
    approval.candidate_artifact_id = "generated-patch.rename-symbol.deadbeef".to_string();
    let sealed = ValidatedRenamePatchCandidate::new_checked(candidate).expect("ready");
    let err = apply_rename_patch_candidate(&sealed, &files, &approval, 0)
      .expect_err("candidate id mismatch must fail");
    match err {
      RenameApplyError::ApprovalCandidateIdMismatch { expected, got } => {
        assert_eq!(expected, "generated-patch.rename-symbol.deadbeef");
        assert!(got.starts_with("generated-patch.rename-symbol."));
      }
      other => panic!("expected ApprovalCandidateIdMismatch, got {:?}", other),
    }
  }

  #[test]
  fn apply_rename_patch_candidate_rejects_missing_file() {
    let (_req, _files, candidate) = fixture_ready_candidate();
    let approval = fixture_approval(&candidate);
    let sealed = ValidatedRenamePatchCandidate::new_checked(candidate).expect("ready");
    // Pass empty files — candidate has src/a.rs but nothing is staged.
    let err = apply_rename_patch_candidate(&sealed, &[], &approval, 0)
      .expect_err("missing staged file must fail");
    match err {
      RenameApplyError::MissingFileForPatch { path } => assert_eq!(path, "src/a.rs"),
      other => panic!("expected MissingFileForPatch, got {:?}", other),
    }
  }

  #[test]
  fn invert_unified_diff_swaps_minus_and_plus_body_lines() {
    let forward = "--- a/x.rs\n+++ b/x.rs\n@@ -1,1 +1,1 @@\n-fn foo() {}\n+fn bar() {}\n";
    let inverse = invert_unified_diff(forward);
    // a/b swap on file headers + body line swap.
    assert!(inverse.contains("+++ b/x.rs"));
    assert!(inverse.contains("--- a/x.rs"));
    assert!(inverse.contains("+fn foo() {}"));
    assert!(inverse.contains("-fn bar() {}"));
  }

  #[test]
  fn apply_then_invert_round_trips_to_original() {
    // The strongest invariant: applying the inverse diff conceptually
    // restores the original. We verify by applying the rename in
    // reverse (rename new_name back to old_name) on the post-apply
    // content and comparing to the pre-apply content.
    let (req, files, candidate) = fixture_ready_candidate();
    let approval = fixture_approval(&candidate);
    let sealed = ValidatedRenamePatchCandidate::new_checked(candidate).expect("ready");
    let receipt =
      apply_rename_patch_candidate(&sealed, &files, &approval, 0).expect("apply succeeds");

    // Reverse rename: bar → foo on the post-apply content.
    let post = &receipt.per_file_after[0].1;
    let reverse_edits = compute_file_rename_edits(&req.new_name, &req.old_name, post);
    let restored = apply_rename_edits(post, &reverse_edits, &req.old_name);
    let original = files[0].content;
    assert_eq!(restored, original, "reverse rename must restore original");
  }

  // ─── apply receipt artifact ───────────────────────────────────────

  fn fixture_apply_receipt() -> RenameApplyReceipt {
    let (_, files, candidate) = fixture_ready_candidate();
    let approval = fixture_approval(&candidate);
    let sealed = ValidatedRenamePatchCandidate::new_checked(candidate).expect("ready");
    apply_rename_patch_candidate(&sealed, &files, &approval, 1700000000999).expect("apply")
  }

  #[test]
  fn apply_receipt_payload_carries_canonical_fields() {
    let receipt = fixture_apply_receipt();
    let payload = build_rename_symbol_apply_receipt_payload(
      &receipt,
      ApplyReceiptContentPolicy::IncludeContent,
    );
    assert_eq!(payload["transform"].as_str(), Some("rename-symbol"));
    assert_eq!(
      payload["owner_law"].as_str(),
      Some("stdlib/lib/gate/code-transform/rename-symbol.px")
    );
    // candidate_artifact_id is non-empty and well-formed.
    let cand_id = payload["candidate_artifact_id"].as_str().expect("id");
    assert!(cand_id.starts_with("generated-patch.rename-symbol."));
    // approval echoed.
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
    // files_after content matches the apply receipt's per_file_after.
    let files_after = payload["files_after"].as_array().expect("files_after");
    assert_eq!(files_after.len(), 1);
    assert_eq!(files_after[0]["path"].as_str(), Some("src/a.rs"));
    assert_eq!(
      files_after[0]["content"].as_str(),
      Some("fn bar() { bar() }\n")
    );
    // content_sha256 is a 64-char hex string.
    let hash = files_after[0]["content_sha256"].as_str().expect("hash");
    assert_eq!(hash.len(), 64);
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    // Rollback hint
    assert_eq!(payload["rollback_available"].as_bool(), Some(true));
    assert_eq!(payload["next_step"].as_str(), Some("verify-or-rollback"));
    // inverse_unified_diff is non-empty and contains the swap.
    let inv = payload["inverse_unified_diff"].as_str().expect("inverse");
    assert!(inv.contains("-fn bar() { bar() }"));
    assert!(inv.contains("+fn foo() { foo() }"));
  }

  #[test]
  fn apply_receipt_artifact_envelope_shape() {
    let receipt = fixture_apply_receipt();
    let art = build_rename_symbol_apply_receipt_artifact(
      &receipt,
      1800000000000,
      None,
      ApplyReceiptContentPolicy::IncludeContent,
    );
    assert_eq!(
      art["artifact_family"].as_str(),
      Some("coding.generated-patch-apply-receipt")
    );
    assert_eq!(
      art["source_surface"].as_str(),
      Some("code-transform.rename-symbol")
    );
    assert_eq!(art["stored_at_ms"].as_u64(), Some(1800000000000));
    let id = art["id"].as_str().expect("id");
    assert!(id.starts_with("apply-receipt.rename-symbol."));
    let related = art["related_refs"].as_array().expect("related_refs");
    assert!(related.iter().any(|v| v
      .as_str()
      .map(|s| s == "owner-law:stdlib/lib/gate/code-transform/rename-symbol.px")
      .unwrap_or(false)));
    assert!(related.iter().any(|v| v
      .as_str()
      .map(|s| s.starts_with("candidate-artifact:generated-patch.rename-symbol."))
      .unwrap_or(false)));
    assert_eq!(art["target_paths"][0].as_str(), Some("src/a.rs"));
    assert_eq!(art["payload"]["transform"].as_str(), Some("rename-symbol"));
  }

  #[test]
  fn apply_receipt_id_is_replay_stable_for_same_apply_event() {
    // Two builds of the same receipt at different storage times must
    // produce the same id — apply event identity is intrinsic, not
    // dependent on storage timestamp.
    let receipt = fixture_apply_receipt();
    let a = build_rename_symbol_apply_receipt_artifact(
      &receipt,
      1000,
      None,
      ApplyReceiptContentPolicy::IncludeContent,
    );
    let b = build_rename_symbol_apply_receipt_artifact(
      &receipt,
      9999,
      None,
      ApplyReceiptContentPolicy::IncludeContent,
    );
    assert_eq!(a["id"], b["id"]);
    assert_ne!(a["stored_at_ms"], b["stored_at_ms"]);
  }

  #[test]
  fn apply_receipt_id_differs_when_applied_at_ms_differs() {
    // Same candidate + same approval applied at different times →
    // different apply events → different ids. The `applied_at_ms` IS
    // part of the receipt identity (unlike stored_at_ms).
    let (_, files, candidate) = fixture_ready_candidate();
    let approval = fixture_approval(&candidate);
    let sealed = ValidatedRenamePatchCandidate::new_checked(candidate.clone()).expect("ready");
    let r1 =
      apply_rename_patch_candidate(&sealed, &files, &approval, 1_700_000_000_000).expect("apply 1");
    let sealed2 = ValidatedRenamePatchCandidate::new_checked(candidate).expect("ready");
    let r2 = apply_rename_patch_candidate(&sealed2, &files, &approval, 1_700_000_000_999)
      .expect("apply 2");
    let a = build_rename_symbol_apply_receipt_artifact(
      &r1,
      0,
      None,
      ApplyReceiptContentPolicy::IncludeContent,
    );
    let b = build_rename_symbol_apply_receipt_artifact(
      &r2,
      0,
      None,
      ApplyReceiptContentPolicy::IncludeContent,
    );
    assert_ne!(a["id"], b["id"]);
  }

  #[test]
  fn apply_receipt_id_differs_when_approval_actor_differs() {
    // Same candidate, different actor → different approval → different
    // apply event identity.
    let (_, files, candidate) = fixture_ready_candidate();
    let approval_a = fixture_approval(&candidate);
    let mut approval_b = fixture_approval(&candidate);
    approval_b.actor_id = "actor.user.2".to_string();
    let sealed_a = ValidatedRenamePatchCandidate::new_checked(candidate.clone()).expect("ready");
    let sealed_b = ValidatedRenamePatchCandidate::new_checked(candidate).expect("ready");
    let r_a = apply_rename_patch_candidate(&sealed_a, &files, &approval_a, 1_000).expect("apply");
    let r_b = apply_rename_patch_candidate(&sealed_b, &files, &approval_b, 1_000).expect("apply");
    let a = build_rename_symbol_apply_receipt_artifact(
      &r_a,
      0,
      None,
      ApplyReceiptContentPolicy::IncludeContent,
    );
    let b = build_rename_symbol_apply_receipt_artifact(
      &r_b,
      0,
      None,
      ApplyReceiptContentPolicy::IncludeContent,
    );
    assert_ne!(a["id"], b["id"]);
  }

  #[test]
  fn apply_receipt_artifact_includes_repo_snapshot_when_provided() {
    let receipt = fixture_apply_receipt();
    let art = build_rename_symbol_apply_receipt_artifact(
      &receipt,
      0,
      Some("sha-abc123"),
      ApplyReceiptContentPolicy::IncludeContent,
    );
    assert_eq!(art["repo_snapshot_ref"].as_str(), Some("sha-abc123"));
  }

  #[test]
  fn apply_receipt_artifact_omits_repo_snapshot_when_absent() {
    let receipt = fixture_apply_receipt();
    let art = build_rename_symbol_apply_receipt_artifact(
      &receipt,
      0,
      None,
      ApplyReceiptContentPolicy::IncludeContent,
    );
    assert!(art.get("repo_snapshot_ref").is_none());
  }

  // ─── content policy ──────────────────────────────────────────────

  #[test]
  fn apply_receipt_payload_omits_content_under_omit_content_policy() {
    // Customer-release safety: `content` body is dropped, only
    // `path` + `content_sha256` + `byte_len` survive. Verifiers can
    // still recompute sha256 from disk and check.
    let receipt = fixture_apply_receipt();
    let payload =
      build_rename_symbol_apply_receipt_payload(&receipt, ApplyReceiptContentPolicy::OmitContent);
    assert_eq!(payload["content_policy"].as_str(), Some("omit-content"));
    let files_after = payload["files_after"].as_array().expect("files_after");
    let entry = &files_after[0];
    assert_eq!(entry["path"].as_str(), Some("src/a.rs"));
    // sha256 + byte_len always present
    assert!(entry["content_sha256"].as_str().is_some());
    assert!(entry["byte_len"].as_u64().is_some());
    // content omitted
    assert!(
      entry.get("content").is_none(),
      "OmitContent must drop content field; got {:?}",
      entry
    );
  }

  #[test]
  fn apply_receipt_payload_includes_content_under_include_content_policy() {
    // Dev / debug mode: content is present.
    let receipt = fixture_apply_receipt();
    let payload = build_rename_symbol_apply_receipt_payload(
      &receipt,
      ApplyReceiptContentPolicy::IncludeContent,
    );
    assert_eq!(payload["content_policy"].as_str(), Some("include-content"));
    let entry = &payload["files_after"][0];
    assert_eq!(entry["content"].as_str(), Some("fn bar() { bar() }\n"));
    assert!(entry["content_sha256"].as_str().is_some());
    assert!(entry["byte_len"].as_u64().is_some());
  }

  #[test]
  fn apply_receipt_id_is_invariant_under_content_policy() {
    // The id hash is *intrinsic event identity* — it must not change
    // between IncludeContent and OmitContent. Both render the *same*
    // apply event, just with different audit verbosity.
    let receipt = fixture_apply_receipt();
    let a = build_rename_symbol_apply_receipt_artifact(
      &receipt,
      0,
      None,
      ApplyReceiptContentPolicy::IncludeContent,
    );
    let b = build_rename_symbol_apply_receipt_artifact(
      &receipt,
      0,
      None,
      ApplyReceiptContentPolicy::OmitContent,
    );
    assert_eq!(
      a["id"], b["id"],
      "content policy is presentation-extrinsic; same event must yield same id"
    );
    // But the payload shape differs.
    assert_eq!(a["payload"]["content_policy"], "include-content");
    assert_eq!(b["payload"]["content_policy"], "omit-content");
    assert!(a["payload"]["files_after"][0]["content"].as_str().is_some());
    assert!(b["payload"]["files_after"][0].get("content").is_none());
  }

  // ─── id hash strengthening ───────────────────────────────────────

  #[test]
  fn apply_receipt_id_differs_when_post_apply_content_differs() {
    // Same candidate + same approval + same applied_at_ms but
    // *different post-apply content* must yield different receipt
    // ids. This is the strengthening: a substrate bug producing a
    // different applied result for the same metadata would be
    // detectable as an id divergence.
    let receipt_a = fixture_apply_receipt();
    let mut receipt_b = receipt_a.clone();
    // Tamper with the post-apply content (simulates a substrate bug
    // or a divergent apply path).
    receipt_b.per_file_after[0].1 = "fn tampered() { tampered() }\n".to_string();
    let a = build_rename_symbol_apply_receipt_artifact(
      &receipt_a,
      0,
      None,
      ApplyReceiptContentPolicy::IncludeContent,
    );
    let b = build_rename_symbol_apply_receipt_artifact(
      &receipt_b,
      0,
      None,
      ApplyReceiptContentPolicy::IncludeContent,
    );
    assert_ne!(
      a["id"], b["id"],
      "post-apply content divergence must produce different receipt ids"
    );
  }

  #[test]
  fn apply_receipt_id_differs_when_inverse_diff_differs() {
    // Same candidate / approval / time / content, but different
    // inverse diff (e.g. a substrate bug that mis-rendered the
    // rollback patch) must yield different ids.
    let receipt_a = fixture_apply_receipt();
    let mut receipt_b = receipt_a.clone();
    receipt_b.inverse_unified_diff =
      "--- a/x\n+++ b/x\n@@ -1,1 +1,1 @@\n-tampered\n+inverse\n".to_string();
    let a = build_rename_symbol_apply_receipt_artifact(
      &receipt_a,
      0,
      None,
      ApplyReceiptContentPolicy::IncludeContent,
    );
    let b = build_rename_symbol_apply_receipt_artifact(
      &receipt_b,
      0,
      None,
      ApplyReceiptContentPolicy::IncludeContent,
    );
    assert_ne!(
      a["id"], b["id"],
      "inverse diff divergence must produce different receipt ids"
    );
  }

  // ─── review-receipt builders ─────────────────────────────────────

  fn fixture_reviewer() -> RenameReviewer {
    RenameReviewer {
      actor_id: "actor.user.1".to_string(),
      tenant_id: "tenant.alpha".to_string(),
    }
  }

  #[test]
  fn review_decision_permits_apply_only_for_approve() {
    assert!(RenameReviewDecision::Approve.permits_apply());
    assert!(!RenameReviewDecision::Hold.permits_apply());
    assert!(!RenameReviewDecision::Reject.permits_apply());
  }

  #[test]
  fn review_decision_as_str_kebab_case() {
    assert_eq!(RenameReviewDecision::Approve.as_str(), "approve");
    assert_eq!(RenameReviewDecision::Hold.as_str(), "hold");
    assert_eq!(RenameReviewDecision::Reject.as_str(), "reject");
  }

  #[test]
  fn build_review_receipt_pins_candidate_artifact_id() {
    let (_, _, candidate) = fixture_ready_candidate();
    let expected_id = build_rename_symbol_patch_candidate_artifact(&candidate, 0, None)["id"]
      .as_str()
      .expect("id")
      .to_string();
    let receipt = build_rename_review_receipt(
      candidate,
      fixture_reviewer(),
      RenameReviewDecision::Approve,
      Some("looks good".to_string()),
      1700000000000,
    );
    assert_eq!(receipt.candidate_artifact_id, expected_id);
    assert_eq!(receipt.decision, RenameReviewDecision::Approve);
    assert_eq!(receipt.reason.as_deref(), Some("looks good"));
  }

  #[test]
  fn approval_from_review_succeeds_only_on_approve() {
    let (_, _, candidate) = fixture_ready_candidate();
    let receipt_approve = build_rename_review_receipt(
      candidate.clone(),
      fixture_reviewer(),
      RenameReviewDecision::Approve,
      None,
      1700000000000,
    );
    let approval = approval_from_review(&receipt_approve).expect("Approve must yield approval");
    assert_eq!(approval.actor_id, "actor.user.1");
    assert_eq!(approval.tenant_id, "tenant.alpha");
    assert_eq!(approval.approved_at_ms, 1700000000000);
    assert_eq!(
      approval.candidate_artifact_id,
      receipt_approve.candidate_artifact_id
    );

    let receipt_hold = build_rename_review_receipt(
      candidate.clone(),
      fixture_reviewer(),
      RenameReviewDecision::Hold,
      None,
      0,
    );
    assert!(approval_from_review(&receipt_hold).is_none());

    let receipt_reject = build_rename_review_receipt(
      candidate,
      fixture_reviewer(),
      RenameReviewDecision::Reject,
      None,
      0,
    );
    assert!(approval_from_review(&receipt_reject).is_none());
  }

  #[test]
  fn review_receipt_payload_carries_canonical_fields() {
    let (_, _, candidate) = fixture_ready_candidate();
    let receipt = build_rename_review_receipt(
      candidate,
      fixture_reviewer(),
      RenameReviewDecision::Approve,
      Some("reviewed manually".to_string()),
      1700000000000,
    );
    let payload = build_rename_symbol_review_receipt_payload(&receipt);
    assert_eq!(payload["transform"].as_str(), Some("rename-symbol"));
    assert_eq!(
      payload["owner_law"].as_str(),
      Some("stdlib/lib/gate/code-transform/rename-symbol.px")
    );
    assert!(payload["candidate_artifact_id"]
      .as_str()
      .expect("cand id")
      .starts_with("generated-patch.rename-symbol."));
    assert_eq!(
      payload["reviewer"]["actor_id"].as_str(),
      Some("actor.user.1")
    );
    assert_eq!(
      payload["reviewer"]["tenant_id"].as_str(),
      Some("tenant.alpha")
    );
    assert_eq!(payload["decision"].as_str(), Some("approve"));
    assert_eq!(payload["reviewed_at_ms"].as_u64(), Some(1700000000000));
    assert_eq!(payload["permits_apply"].as_bool(), Some(true));
    assert_eq!(payload["next_step"].as_str(), Some("apply"));
    assert_eq!(payload["reason"].as_str(), Some("reviewed manually"));
  }

  #[test]
  fn review_receipt_payload_next_step_per_decision() {
    let (_, _, candidate) = fixture_ready_candidate();
    let cases = [
      (RenameReviewDecision::Approve, "apply", true),
      (RenameReviewDecision::Hold, "wait-for-evidence", false),
      (RenameReviewDecision::Reject, "rejected", false),
    ];
    for (decision, expected_next, expected_permits) in cases {
      let receipt =
        build_rename_review_receipt(candidate.clone(), fixture_reviewer(), decision, None, 0);
      let payload = build_rename_symbol_review_receipt_payload(&receipt);
      assert_eq!(
        payload["next_step"].as_str(),
        Some(expected_next),
        "decision {decision:?} should map to next_step={expected_next}"
      );
      assert_eq!(payload["permits_apply"].as_bool(), Some(expected_permits));
    }
  }

  #[test]
  fn review_receipt_payload_reason_is_null_when_absent() {
    let (_, _, candidate) = fixture_ready_candidate();
    let receipt = build_rename_review_receipt(
      candidate,
      fixture_reviewer(),
      RenameReviewDecision::Approve,
      None,
      0,
    );
    let payload = build_rename_symbol_review_receipt_payload(&receipt);
    assert!(payload["reason"].is_null());
  }

  #[test]
  fn review_receipt_artifact_envelope_shape() {
    let (_, _, candidate) = fixture_ready_candidate();
    let receipt = build_rename_review_receipt(
      candidate,
      fixture_reviewer(),
      RenameReviewDecision::Approve,
      None,
      1700000000000,
    );
    let art = build_rename_symbol_review_receipt_artifact(&receipt, 1800000000000, None);
    assert_eq!(
      art["artifact_family"].as_str(),
      Some("coding.generated-patch-review-receipt")
    );
    assert_eq!(
      art["source_surface"].as_str(),
      Some("code-transform.rename-symbol")
    );
    assert_eq!(art["stored_at_ms"].as_u64(), Some(1800000000000));
    let id = art["id"].as_str().expect("id");
    assert!(id.starts_with("review-receipt.rename-symbol."));
    // related_refs link back to the candidate.
    let related = art["related_refs"].as_array().expect("related_refs");
    assert!(related.iter().any(|v| v
      .as_str()
      .map(|s| s.starts_with("candidate-artifact:generated-patch.rename-symbol."))
      .unwrap_or(false)));
    // target_paths carried at top-level for storage indexing.
    assert_eq!(art["target_paths"][0].as_str(), Some("src/a.rs"));
  }

  #[test]
  fn review_receipt_id_replay_stable_across_storage_times() {
    let (_, _, candidate) = fixture_ready_candidate();
    let receipt = build_rename_review_receipt(
      candidate,
      fixture_reviewer(),
      RenameReviewDecision::Approve,
      None,
      1700000000000,
    );
    let a = build_rename_symbol_review_receipt_artifact(&receipt, 1000, None);
    let b = build_rename_symbol_review_receipt_artifact(&receipt, 9999, None);
    assert_eq!(a["id"], b["id"]);
    assert_ne!(a["stored_at_ms"], b["stored_at_ms"]);
  }

  #[test]
  fn review_receipt_id_differs_per_decision() {
    let (_, _, candidate) = fixture_ready_candidate();
    let approve = build_rename_review_receipt(
      candidate.clone(),
      fixture_reviewer(),
      RenameReviewDecision::Approve,
      None,
      0,
    );
    let hold = build_rename_review_receipt(
      candidate.clone(),
      fixture_reviewer(),
      RenameReviewDecision::Hold,
      None,
      0,
    );
    let reject = build_rename_review_receipt(
      candidate,
      fixture_reviewer(),
      RenameReviewDecision::Reject,
      None,
      0,
    );
    let a_art = build_rename_symbol_review_receipt_artifact(&approve, 0, None);
    let h_art = build_rename_symbol_review_receipt_artifact(&hold, 0, None);
    let r_art = build_rename_symbol_review_receipt_artifact(&reject, 0, None);
    assert_ne!(a_art["id"], h_art["id"]);
    assert_ne!(a_art["id"], r_art["id"]);
    assert_ne!(h_art["id"], r_art["id"]);
  }

  #[test]
  fn review_receipt_id_differs_per_reason() {
    // Same decision + same reviewer + same time + different reason →
    // different id. Distinct rationale = distinct review event.
    let (_, _, candidate) = fixture_ready_candidate();
    let r1 = build_rename_review_receipt(
      candidate.clone(),
      fixture_reviewer(),
      RenameReviewDecision::Approve,
      Some("reason one".to_string()),
      1000,
    );
    let r2 = build_rename_review_receipt(
      candidate,
      fixture_reviewer(),
      RenameReviewDecision::Approve,
      Some("reason two".to_string()),
      1000,
    );
    let a = build_rename_symbol_review_receipt_artifact(&r1, 0, None);
    let b = build_rename_symbol_review_receipt_artifact(&r2, 0, None);
    assert_ne!(a["id"], b["id"]);
  }

  #[test]
  fn end_to_end_candidate_to_review_to_apply_chain() {
    // The canonical chain: candidate → review (Approve) → approval →
    // apply receipt. All three share the same `candidate_artifact_id`.
    let (_req, files, candidate) = fixture_ready_candidate();
    let receipt_review = build_rename_review_receipt(
      candidate.clone(),
      fixture_reviewer(),
      RenameReviewDecision::Approve,
      Some("LGTM".to_string()),
      1700000000000,
    );
    let approval = approval_from_review(&receipt_review).expect("approve permits apply");
    let sealed = ValidatedRenamePatchCandidate::new_checked(candidate).expect("ready");
    let apply_receipt =
      apply_rename_patch_candidate(&sealed, &files, &approval, 1700000000999).expect("apply");
    // Same candidate id flows through review → approval → apply.
    assert_eq!(
      receipt_review.candidate_artifact_id,
      approval.candidate_artifact_id
    );
    assert_eq!(
      approval.candidate_artifact_id,
      build_rename_symbol_patch_candidate_artifact(&apply_receipt.candidate, 0, None)["id"]
        .as_str()
        .expect("id")
    );
  }

  // ─── rollback-handle artifact ────────────────────────────────────

  fn fixture_initiator() -> RenameRollbackInitiator {
    RenameRollbackInitiator {
      actor_id: "operator.ops.1".to_string(),
      tenant_id: "tenant.alpha".to_string(),
    }
  }

  #[test]
  fn build_rollback_handle_pins_apply_receipt_and_candidate_ids() {
    let receipt = fixture_apply_receipt();
    let expected_apply_id = build_rename_symbol_apply_receipt_artifact(
      &receipt,
      0,
      None,
      ApplyReceiptContentPolicy::OmitContent,
    )["id"]
      .as_str()
      .expect("apply id")
      .to_string();
    let expected_cand_id =
      build_rename_symbol_patch_candidate_artifact(&receipt.candidate, 0, None)["id"]
        .as_str()
        .expect("cand id")
        .to_string();
    let handle = build_rename_rollback_handle(
      receipt,
      fixture_initiator(),
      Some("apply produced broken state".to_string()),
      1800000001000,
    );
    assert_eq!(handle.apply_receipt_artifact_id, expected_apply_id);
    assert_eq!(handle.candidate_artifact_id, expected_cand_id);
    assert_eq!(handle.initiator.actor_id, "operator.ops.1");
    assert_eq!(handle.initiator.tenant_id, "tenant.alpha");
    assert_eq!(handle.initiated_at_ms, 1800000001000);
    assert_eq!(
      handle.reason.as_deref(),
      Some("apply produced broken state")
    );
    assert!(handle.inverse_unified_diff.contains("-fn bar() { bar() }"));
    assert_eq!(handle.target_paths, vec!["src/a.rs".to_string()]);
  }

  #[test]
  fn rollback_handle_payload_carries_canonical_fields() {
    let receipt = fixture_apply_receipt();
    let handle = build_rename_rollback_handle(
      receipt,
      fixture_initiator(),
      Some("test rollback".to_string()),
      1800000001000,
    );
    let payload = build_rename_symbol_rollback_handle_payload(&handle);
    assert_eq!(payload["transform"].as_str(), Some("rename-symbol"));
    assert_eq!(
      payload["owner_law"].as_str(),
      Some("stdlib/lib/gate/code-transform/rename-symbol.px")
    );
    assert!(payload["apply_receipt_artifact_id"]
      .as_str()
      .expect("apply id")
      .starts_with("apply-receipt.rename-symbol."));
    assert!(payload["candidate_artifact_id"]
      .as_str()
      .expect("cand id")
      .starts_with("generated-patch.rename-symbol."));
    assert_eq!(
      payload["initiator"]["actor_id"].as_str(),
      Some("operator.ops.1")
    );
    assert_eq!(
      payload["initiator"]["tenant_id"].as_str(),
      Some("tenant.alpha")
    );
    assert_eq!(payload["initiated_at_ms"].as_u64(), Some(1800000001000));
    assert_eq!(payload["reason"].as_str(), Some("test rollback"));
    assert_eq!(payload["rollback_state"].as_str(), Some("handle-issued"));
    assert_eq!(payload["next_step"].as_str(), Some("execute-rollback"));
    assert_eq!(payload["target_paths"][0].as_str(), Some("src/a.rs"));
    let inv = payload["inverse_unified_diff"].as_str().expect("inverse");
    assert!(!inv.is_empty());
  }

  #[test]
  fn rollback_handle_payload_reason_null_when_absent() {
    let receipt = fixture_apply_receipt();
    let handle = build_rename_rollback_handle(receipt, fixture_initiator(), None, 0);
    let payload = build_rename_symbol_rollback_handle_payload(&handle);
    assert!(payload["reason"].is_null());
  }

  #[test]
  fn rollback_handle_artifact_envelope_shape() {
    let receipt = fixture_apply_receipt();
    let handle = build_rename_rollback_handle(
      receipt,
      fixture_initiator(),
      Some("safety rollback".to_string()),
      1800000001000,
    );
    let art = build_rename_symbol_rollback_handle_artifact(&handle, 1900000000000, None);
    assert_eq!(
      art["artifact_family"].as_str(),
      Some("coding.rollback-handle")
    );
    assert_eq!(
      art["source_surface"].as_str(),
      Some("code-transform.rename-symbol")
    );
    assert_eq!(art["stored_at_ms"].as_u64(), Some(1900000000000));
    let id = art["id"].as_str().expect("id");
    assert!(id.starts_with("rollback-handle.rename-symbol."));
    let related = art["related_refs"].as_array().expect("related_refs");
    assert!(related.iter().any(|v| v
      .as_str()
      .map(|s| s == "owner-law:stdlib/lib/gate/code-transform/rename-symbol.px")
      .unwrap_or(false)));
    // Two back-refs: candidate and apply-receipt.
    assert!(related.iter().any(|v| v
      .as_str()
      .map(|s| s.starts_with("candidate-artifact:generated-patch.rename-symbol."))
      .unwrap_or(false)));
    assert!(related.iter().any(|v| v
      .as_str()
      .map(|s| s.starts_with("apply-receipt-artifact:apply-receipt.rename-symbol."))
      .unwrap_or(false)));
    assert_eq!(art["target_paths"][0].as_str(), Some("src/a.rs"));
  }

  #[test]
  fn rollback_handle_id_replay_stable_across_storage_times() {
    let receipt = fixture_apply_receipt();
    let handle = build_rename_rollback_handle(
      receipt,
      fixture_initiator(),
      Some("test".to_string()),
      1800000001000,
    );
    let a = build_rename_symbol_rollback_handle_artifact(&handle, 1000, None);
    let b = build_rename_symbol_rollback_handle_artifact(&handle, 9999, None);
    assert_eq!(a["id"], b["id"]);
    assert_ne!(a["stored_at_ms"], b["stored_at_ms"]);
  }

  #[test]
  fn rollback_handle_id_differs_per_initiator() {
    // Same apply receipt rolled back by two different actors → two
    // distinct rollback intents → different ids.
    let receipt = fixture_apply_receipt();
    let initiator_a = RenameRollbackInitiator {
      actor_id: "operator.a".to_string(),
      tenant_id: "tenant.alpha".to_string(),
    };
    let initiator_b = RenameRollbackInitiator {
      actor_id: "operator.b".to_string(),
      tenant_id: "tenant.alpha".to_string(),
    };
    let h_a =
      build_rename_rollback_handle(receipt.clone(), initiator_a, Some("ops".to_string()), 1000);
    let h_b = build_rename_rollback_handle(receipt, initiator_b, Some("ops".to_string()), 1000);
    let a = build_rename_symbol_rollback_handle_artifact(&h_a, 0, None);
    let b = build_rename_symbol_rollback_handle_artifact(&h_b, 0, None);
    assert_ne!(a["id"], b["id"]);
  }

  #[test]
  fn rollback_handle_id_differs_per_initiated_at_ms() {
    // Two rollback attempts at different times → different events.
    let receipt = fixture_apply_receipt();
    let h1 = build_rename_rollback_handle(
      receipt.clone(),
      fixture_initiator(),
      Some("test".to_string()),
      1000,
    );
    let h2 =
      build_rename_rollback_handle(receipt, fixture_initiator(), Some("test".to_string()), 9999);
    let a = build_rename_symbol_rollback_handle_artifact(&h1, 0, None);
    let b = build_rename_symbol_rollback_handle_artifact(&h2, 0, None);
    assert_ne!(a["id"], b["id"]);
  }

  #[test]
  fn end_to_end_chain_candidate_review_apply_rollback() {
    // Full canonical chain: candidate → review (Approve) → apply →
    // rollback. All four artifacts cross-link via candidate_artifact_id;
    // rollback also back-refs apply_receipt_artifact_id.
    let (_req, files, candidate) = fixture_ready_candidate();

    // Review (Approve)
    let review = build_rename_review_receipt(
      candidate.clone(),
      RenameReviewer {
        actor_id: "reviewer.alice".to_string(),
        tenant_id: "tenant.alpha".to_string(),
      },
      RenameReviewDecision::Approve,
      Some("LGTM".to_string()),
      1700000000000,
    );
    let approval = approval_from_review(&review).expect("approve");

    // Apply
    let sealed = ValidatedRenamePatchCandidate::new_checked(candidate).expect("ready");
    let apply_receipt =
      apply_rename_patch_candidate(&sealed, &files, &approval, 1700000000999).expect("apply");

    // Rollback
    let handle = build_rename_rollback_handle(
      apply_receipt,
      RenameRollbackInitiator {
        actor_id: "operator.bob".to_string(),
        tenant_id: "tenant.alpha".to_string(),
      },
      Some("regression in CI".to_string()),
      1700000010000,
    );

    // Chain cross-refs: rollback handle's candidate_artifact_id ==
    // review's candidate_artifact_id == approval's candidate_artifact_id.
    assert_eq!(handle.candidate_artifact_id, review.candidate_artifact_id);
    assert_eq!(handle.candidate_artifact_id, approval.candidate_artifact_id);

    // Build all four artifacts and verify the graph cross-refs.
    let cand_art =
      build_rename_symbol_patch_candidate_artifact(&handle.apply_receipt.candidate, 0, None);
    let review_art = build_rename_symbol_review_receipt_artifact(&review, 0, None);
    let apply_art = build_rename_symbol_apply_receipt_artifact(
      &handle.apply_receipt,
      0,
      None,
      ApplyReceiptContentPolicy::OmitContent,
    );
    let rollback_art = build_rename_symbol_rollback_handle_artifact(&handle, 0, None);

    // Distinct families.
    let families: std::collections::BTreeSet<&str> = [
      cand_art["artifact_family"].as_str().unwrap(),
      review_art["artifact_family"].as_str().unwrap(),
      apply_art["artifact_family"].as_str().unwrap(),
      rollback_art["artifact_family"].as_str().unwrap(),
    ]
    .into_iter()
    .collect();
    assert_eq!(families.len(), 4, "four distinct artifact families");

    // Rollback handle back-refs both candidate and apply receipt.
    let rb_related = rollback_art["related_refs"].as_array().unwrap();
    let cand_id = cand_art["id"].as_str().unwrap();
    let apply_id = apply_art["id"].as_str().unwrap();
    assert!(rb_related.iter().any(|v| {
      v.as_str()
        .map(|s| s == format!("candidate-artifact:{cand_id}"))
        .unwrap_or(false)
    }));
    assert!(rb_related.iter().any(|v| {
      v.as_str()
        .map(|s| s == format!("apply-receipt-artifact:{apply_id}"))
        .unwrap_or(false)
    }));
  }

  // ─── rollback execution + receipt ────────────────────────────────

  fn fixture_executor() -> RenameRollbackExecutor {
    RenameRollbackExecutor {
      actor_id: "executor.runner.1".to_string(),
      tenant_id: "tenant.alpha".to_string(),
    }
  }

  fn fixture_rollback_setup() -> (Vec<RenameFileInput<'static>>, RenameRollbackHandle) {
    let (_, files, candidate) = fixture_ready_candidate();
    let approval = fixture_approval(&candidate);
    let sealed = ValidatedRenamePatchCandidate::new_checked(candidate).expect("ready");
    let apply_receipt =
      apply_rename_patch_candidate(&sealed, &files, &approval, 1700000000999).expect("apply");
    let handle = build_rename_rollback_handle(
      apply_receipt,
      RenameRollbackInitiator {
        actor_id: "operator.ops.1".to_string(),
        tenant_id: "tenant.alpha".to_string(),
      },
      Some("regression".to_string()),
      1800000010000,
    );
    (files, handle)
  }

  /// `current_files` shaped from the handle's apply receipt — what
  /// would be on disk immediately after apply, before any drift.
  fn post_apply_files_from_handle(handle: &RenameRollbackHandle) -> Vec<(String, String)> {
    handle.apply_receipt.per_file_after.clone()
  }

  #[test]
  fn execute_rollback_happy_path_restores_pre_apply_content() {
    let (original_files, handle) = fixture_rollback_setup();
    // current_files: the post-apply state (what's on disk now).
    let post_apply: Vec<(String, String)> = post_apply_files_from_handle(&handle);
    let inputs: Vec<RenameFileInput<'_>> = post_apply
      .iter()
      .map(|(p, c)| RenameFileInput {
        path: p.as_str(),
        content: c.as_str(),
      })
      .collect();
    let receipt = execute_rename_rollback(&handle, &inputs, &fixture_executor(), 1800000020000)
      .expect("rollback succeeds");
    // Per-file content after rollback must equal the original pre-apply content.
    assert_eq!(receipt.per_file_after_rollback.len(), 1);
    assert_eq!(receipt.per_file_after_rollback[0].0, "src/a.rs");
    assert_eq!(
      receipt.per_file_after_rollback[0].1, original_files[0].content,
      "rollback must restore pre-apply content"
    );
    assert_eq!(receipt.executor.actor_id, "executor.runner.1");
    assert_eq!(receipt.executed_at_ms, 1800000020000);
    assert!(receipt
      .rollback_handle_artifact_id
      .starts_with("rollback-handle.rename-symbol."));
  }

  #[test]
  fn execute_rollback_rejects_empty_executor_actor() {
    let (_, handle) = fixture_rollback_setup();
    let post_apply = post_apply_files_from_handle(&handle);
    let inputs: Vec<RenameFileInput<'_>> = post_apply
      .iter()
      .map(|(p, c)| RenameFileInput {
        path: p.as_str(),
        content: c.as_str(),
      })
      .collect();
    let bad = RenameRollbackExecutor {
      actor_id: "".to_string(),
      tenant_id: "tenant.alpha".to_string(),
    };
    let err =
      execute_rename_rollback(&handle, &inputs, &bad, 0).expect_err("empty actor must fail");
    assert!(matches!(err, RenameRollbackError::MissingExecutorActor));
  }

  #[test]
  fn execute_rollback_rejects_empty_executor_tenant() {
    let (_, handle) = fixture_rollback_setup();
    let post_apply = post_apply_files_from_handle(&handle);
    let inputs: Vec<RenameFileInput<'_>> = post_apply
      .iter()
      .map(|(p, c)| RenameFileInput {
        path: p.as_str(),
        content: c.as_str(),
      })
      .collect();
    let bad = RenameRollbackExecutor {
      actor_id: "executor.runner.1".to_string(),
      tenant_id: "".to_string(),
    };
    let err =
      execute_rename_rollback(&handle, &inputs, &bad, 0).expect_err("empty tenant must fail");
    assert!(matches!(err, RenameRollbackError::MissingExecutorTenant));
  }

  #[test]
  fn execute_rollback_rejects_missing_file() {
    let (_, handle) = fixture_rollback_setup();
    // Pass no files — handle names src/a.rs but inputs are empty.
    let err = execute_rename_rollback(&handle, &[], &fixture_executor(), 0)
      .expect_err("missing file must fail");
    match err {
      RenameRollbackError::MissingFileForRollback { path } => assert_eq!(path, "src/a.rs"),
      other => panic!("expected MissingFileForRollback, got {:?}", other),
    }
  }

  #[test]
  fn execute_rollback_rejects_post_apply_content_drift() {
    // Current file content doesn't match the apply receipt's recorded
    // sha256 — file has drifted. Fail-closed.
    let (_, handle) = fixture_rollback_setup();
    let drifted = RenameFileInput {
      path: "src/a.rs",
      content: "fn drifted_content_unrelated_to_rename() {}\n",
    };
    let err = execute_rename_rollback(&handle, &[drifted], &fixture_executor(), 0)
      .expect_err("drift must fail");
    match err {
      RenameRollbackError::PostApplyContentDriftDetected {
        path,
        expected_sha256,
        actual_sha256,
      } => {
        assert_eq!(path, "src/a.rs");
        assert_eq!(expected_sha256.len(), 64);
        assert_eq!(actual_sha256.len(), 64);
        assert_ne!(expected_sha256, actual_sha256);
      }
      other => panic!("expected PostApplyContentDriftDetected, got {:?}", other),
    }
  }

  #[test]
  fn rollback_receipt_payload_canonical_fields() {
    let (_, handle) = fixture_rollback_setup();
    let post_apply = post_apply_files_from_handle(&handle);
    let inputs: Vec<RenameFileInput<'_>> = post_apply
      .iter()
      .map(|(p, c)| RenameFileInput {
        path: p.as_str(),
        content: c.as_str(),
      })
      .collect();
    let receipt = execute_rename_rollback(&handle, &inputs, &fixture_executor(), 1800000020000)
      .expect("rollback succeeds");
    let payload = build_rename_symbol_rollback_receipt_payload(
      &receipt,
      ApplyReceiptContentPolicy::IncludeContent,
    );
    assert_eq!(payload["transform"].as_str(), Some("rename-symbol"));
    assert_eq!(payload["rollback_state"].as_str(), Some("executed"));
    assert_eq!(
      payload["next_step"].as_str(),
      Some("verify-rollback-or-redo-apply")
    );
    assert_eq!(
      payload["executor"]["actor_id"].as_str(),
      Some("executor.runner.1")
    );
    assert_eq!(payload["executed_at_ms"].as_u64(), Some(1800000020000));
    assert!(payload["rollback_handle_artifact_id"]
      .as_str()
      .expect("id")
      .starts_with("rollback-handle.rename-symbol."));
    let files_arr = payload["files_after_rollback"]
      .as_array()
      .expect("files array");
    assert_eq!(files_arr[0]["path"].as_str(), Some("src/a.rs"));
    // IncludeContent → content present
    assert_eq!(
      files_arr[0]["content"].as_str(),
      Some("fn foo() { foo() }\n")
    );
    assert_eq!(payload["content_policy"].as_str(), Some("include-content"));
  }

  #[test]
  fn rollback_receipt_payload_omits_content_under_omit_policy() {
    let (_, handle) = fixture_rollback_setup();
    let post_apply = post_apply_files_from_handle(&handle);
    let inputs: Vec<RenameFileInput<'_>> = post_apply
      .iter()
      .map(|(p, c)| RenameFileInput {
        path: p.as_str(),
        content: c.as_str(),
      })
      .collect();
    let receipt =
      execute_rename_rollback(&handle, &inputs, &fixture_executor(), 0).expect("rollback");
    let payload = build_rename_symbol_rollback_receipt_payload(
      &receipt,
      ApplyReceiptContentPolicy::OmitContent,
    );
    assert_eq!(payload["content_policy"].as_str(), Some("omit-content"));
    let entry = &payload["files_after_rollback"][0];
    assert!(entry.get("content").is_none());
    assert!(entry["content_sha256"].as_str().is_some());
    assert!(entry["byte_len"].as_u64().is_some());
  }

  #[test]
  fn rollback_receipt_artifact_envelope_shape() {
    let (_, handle) = fixture_rollback_setup();
    let post_apply = post_apply_files_from_handle(&handle);
    let inputs: Vec<RenameFileInput<'_>> = post_apply
      .iter()
      .map(|(p, c)| RenameFileInput {
        path: p.as_str(),
        content: c.as_str(),
      })
      .collect();
    let receipt = execute_rename_rollback(&handle, &inputs, &fixture_executor(), 1800000020000)
      .expect("rollback");
    let art = build_rename_symbol_rollback_receipt_artifact(
      &receipt,
      1900000000000,
      None,
      ApplyReceiptContentPolicy::OmitContent,
    );
    assert_eq!(
      art["artifact_family"].as_str(),
      Some("coding.rollback-receipt")
    );
    assert_eq!(
      art["source_surface"].as_str(),
      Some("code-transform.rename-symbol")
    );
    assert_eq!(art["stored_at_ms"].as_u64(), Some(1900000000000));
    let id = art["id"].as_str().expect("id");
    assert!(id.starts_with("rollback-receipt.rename-symbol."));
    // TRIPLE back-refs: candidate, apply-receipt, rollback-handle.
    let related = art["related_refs"].as_array().expect("related");
    assert!(related.iter().any(|v| v
      .as_str()
      .map(|s| s.starts_with("candidate-artifact:generated-patch.rename-symbol."))
      .unwrap_or(false)));
    assert!(related.iter().any(|v| v
      .as_str()
      .map(|s| s.starts_with("apply-receipt-artifact:apply-receipt.rename-symbol."))
      .unwrap_or(false)));
    assert!(related.iter().any(|v| v
      .as_str()
      .map(|s| s.starts_with("rollback-handle-artifact:rollback-handle.rename-symbol."))
      .unwrap_or(false)));
  }

  #[test]
  fn rollback_receipt_id_replay_stable_across_storage_times() {
    let (_, handle) = fixture_rollback_setup();
    let post_apply = post_apply_files_from_handle(&handle);
    let inputs: Vec<RenameFileInput<'_>> = post_apply
      .iter()
      .map(|(p, c)| RenameFileInput {
        path: p.as_str(),
        content: c.as_str(),
      })
      .collect();
    let receipt = execute_rename_rollback(&handle, &inputs, &fixture_executor(), 1800000020000)
      .expect("rollback");
    let a = build_rename_symbol_rollback_receipt_artifact(
      &receipt,
      1000,
      None,
      ApplyReceiptContentPolicy::IncludeContent,
    );
    let b = build_rename_symbol_rollback_receipt_artifact(
      &receipt,
      9999,
      None,
      ApplyReceiptContentPolicy::OmitContent,
    );
    // stored_at_ms AND content_policy are extrinsic — same event → same id.
    assert_eq!(a["id"], b["id"]);
  }

  #[test]
  fn full_five_stage_chain_candidate_review_apply_rollback_handle_receipt() {
    // The full canonical chain — all five artifact families produced
    // and cross-linked.
    let (_req, files, candidate) = fixture_ready_candidate();
    let review = build_rename_review_receipt(
      candidate.clone(),
      RenameReviewer {
        actor_id: "reviewer.alice".to_string(),
        tenant_id: "tenant.alpha".to_string(),
      },
      RenameReviewDecision::Approve,
      Some("LGTM".to_string()),
      1700000000000,
    );
    let approval = approval_from_review(&review).expect("approve");
    let sealed = ValidatedRenamePatchCandidate::new_checked(candidate).expect("ready");
    let apply_receipt =
      apply_rename_patch_candidate(&sealed, &files, &approval, 1700000000999).expect("apply");
    let handle = build_rename_rollback_handle(
      apply_receipt.clone(),
      RenameRollbackInitiator {
        actor_id: "operator.bob".to_string(),
        tenant_id: "tenant.alpha".to_string(),
      },
      Some("regression in CI".to_string()),
      1700000010000,
    );
    let post_apply_inputs: Vec<RenameFileInput<'_>> = apply_receipt
      .per_file_after
      .iter()
      .map(|(p, c)| RenameFileInput {
        path: p.as_str(),
        content: c.as_str(),
      })
      .collect();
    let rollback_receipt = execute_rename_rollback(
      &handle,
      &post_apply_inputs,
      &RenameRollbackExecutor {
        actor_id: "executor.runner.1".to_string(),
        tenant_id: "tenant.alpha".to_string(),
      },
      1700000020000,
    )
    .expect("rollback");

    // Build all 5 artifacts.
    let cand_art =
      build_rename_symbol_patch_candidate_artifact(&handle.apply_receipt.candidate, 0, None);
    let review_art = build_rename_symbol_review_receipt_artifact(&review, 0, None);
    let apply_art = build_rename_symbol_apply_receipt_artifact(
      &handle.apply_receipt,
      0,
      None,
      ApplyReceiptContentPolicy::OmitContent,
    );
    let handle_art = build_rename_symbol_rollback_handle_artifact(&handle, 0, None);
    let rb_art = build_rename_symbol_rollback_receipt_artifact(
      &rollback_receipt,
      0,
      None,
      ApplyReceiptContentPolicy::OmitContent,
    );

    // 5 distinct artifact families.
    let families: std::collections::BTreeSet<&str> = [
      cand_art["artifact_family"].as_str().unwrap(),
      review_art["artifact_family"].as_str().unwrap(),
      apply_art["artifact_family"].as_str().unwrap(),
      handle_art["artifact_family"].as_str().unwrap(),
      rb_art["artifact_family"].as_str().unwrap(),
    ]
    .into_iter()
    .collect();
    assert_eq!(families.len(), 5, "five distinct artifact families");

    // The rollback receipt cross-refs all three upstream artifacts.
    let rb_related = rb_art["related_refs"].as_array().unwrap();
    let cand_id = cand_art["id"].as_str().unwrap();
    let apply_id = apply_art["id"].as_str().unwrap();
    let handle_id = handle_art["id"].as_str().unwrap();
    assert!(rb_related.iter().any(|v| v
      .as_str()
      .map(|s| s == format!("candidate-artifact:{cand_id}"))
      .unwrap_or(false)));
    assert!(rb_related.iter().any(|v| v
      .as_str()
      .map(|s| s == format!("apply-receipt-artifact:{apply_id}"))
      .unwrap_or(false)));
    assert!(rb_related.iter().any(|v| v
      .as_str()
      .map(|s| s == format!("rollback-handle-artifact:{handle_id}"))
      .unwrap_or(false)));

    // Round-trip invariant: post-rollback content == pre-apply
    // content (the original `files[0].content` from the fixture).
    assert_eq!(
      rollback_receipt.per_file_after_rollback[0].1, files[0].content,
      "rollback closes the chain by restoring pre-apply state"
    );
  }

  #[test]
  fn helper_validators() {
    // Tiny sanity tests for the private validators — exercising the
    // edge cases the .px owner-law also explicitly mentions.
    assert!(is_valid_identifier("foo"));
    assert!(is_valid_identifier("_bar"));
    assert!(is_valid_identifier("x1"));
    assert!(is_valid_identifier("X_Y_Z"));
    assert!(!is_valid_identifier(""));
    assert!(!is_valid_identifier("1abc"));
    assert!(!is_valid_identifier("has space"));
    assert!(!is_valid_identifier("ümlaut")); // ASCII-only

    assert!(is_supported_language("rust"));
    assert!(is_supported_language("python"));
    assert!(!is_supported_language("fortran"));

    assert!(is_path_in_project("src/a.rs"));
    assert!(is_path_in_project("a/b/c.rs"));
    assert!(!is_path_in_project(""));
    assert!(!is_path_in_project("../escape.rs"));
    assert!(!is_path_in_project("ok/then/../bad.rs"));
  }

  // ─── Rust skip-zone lexer tests ──────────────────────────────────

  #[test]
  fn skip_zones_line_comment_extends_to_eol() {
    let src = "let x = foo; // foo bar\nlet y = foo;\n";
    let zones = rust_skip_zones(src);
    let line_comments: Vec<_> = zones
      .iter()
      .filter(|z| z.kind == RustSkipZoneKind::LineComment)
      .collect();
    assert_eq!(line_comments.len(), 1);
    let z = line_comments[0];
    // Zone starts at the `//` and ends at the `\n` (exclusive).
    assert_eq!(&src[z.start..z.end], "// foo bar");
  }

  #[test]
  fn skip_zones_block_comment_supports_nesting() {
    let src = "let x = /* outer /* inner */ still outer */ 1;\n";
    let zones = rust_skip_zones(src);
    let block_comments: Vec<_> = zones
      .iter()
      .filter(|z| z.kind == RustSkipZoneKind::BlockComment)
      .collect();
    // Only ONE block comment zone (the nesting is properly tracked).
    assert_eq!(block_comments.len(), 1);
    let z = block_comments[0];
    assert_eq!(&src[z.start..z.end], "/* outer /* inner */ still outer */");
  }

  #[test]
  fn skip_zones_string_literal_with_escapes() {
    let src = r#"let s = "hello \"foo\" world"; let t = foo;"#;
    let zones = rust_skip_zones(src);
    let strings: Vec<_> = zones
      .iter()
      .filter(|z| z.kind == RustSkipZoneKind::String)
      .collect();
    assert_eq!(strings.len(), 1);
    let z = strings[0];
    assert_eq!(&src[z.start..z.end], r#""hello \"foo\" world""#);
  }

  #[test]
  fn skip_zones_raw_string_with_hashes() {
    let src = r####"let s = r#"this " has quote"#; let t = foo;"####;
    let zones = rust_skip_zones(src);
    let raw: Vec<_> = zones
      .iter()
      .filter(|z| z.kind == RustSkipZoneKind::RawString)
      .collect();
    assert_eq!(raw.len(), 1);
    let z = raw[0];
    assert_eq!(&src[z.start..z.end], r####"r#"this " has quote"#"####);
  }

  #[test]
  fn skip_zones_byte_string() {
    let src = r#"let bs = b"foo bytes"; let id = foo;"#;
    let zones = rust_skip_zones(src);
    let byte_strings: Vec<_> = zones
      .iter()
      .filter(|z| z.kind == RustSkipZoneKind::ByteString)
      .collect();
    assert_eq!(byte_strings.len(), 1);
    assert_eq!(
      &src[byte_strings[0].start..byte_strings[0].end],
      r#"b"foo bytes""#
    );
  }

  #[test]
  fn skip_zones_raw_byte_string() {
    let src = r####"let bs = br#"foo"#; let id = foo;"####;
    let zones = rust_skip_zones(src);
    let raw_bytes: Vec<_> = zones
      .iter()
      .filter(|z| z.kind == RustSkipZoneKind::RawByteString)
      .collect();
    assert_eq!(raw_bytes.len(), 1);
    assert_eq!(
      &src[raw_bytes[0].start..raw_bytes[0].end],
      r####"br#"foo"#"####
    );
  }

  #[test]
  fn skip_zones_char_literal() {
    let src = "let c = 'a'; let d = '\\n'; let e = '\\u{1234}'; let f = foo;";
    let zones = rust_skip_zones(src);
    let chars: Vec<_> = zones
      .iter()
      .filter(|z| z.kind == RustSkipZoneKind::Char)
      .collect();
    assert_eq!(chars.len(), 3);
    assert_eq!(&src[chars[0].start..chars[0].end], "'a'");
    assert_eq!(&src[chars[1].start..chars[1].end], "'\\n'");
    assert_eq!(&src[chars[2].start..chars[2].end], "'\\u{1234}'");
  }

  #[test]
  fn skip_zones_byte_char() {
    let src = "let b = b'A'; let id = foo;";
    let zones = rust_skip_zones(src);
    let byte_chars: Vec<_> = zones
      .iter()
      .filter(|z| z.kind == RustSkipZoneKind::ByteChar)
      .collect();
    assert_eq!(byte_chars.len(), 1);
    assert_eq!(&src[byte_chars[0].start..byte_chars[0].end], "b'A'");
  }

  #[test]
  fn skip_zones_lifetime_not_char() {
    let src = "fn f<'a, 'static>(x: &'a str) {}";
    let zones = rust_skip_zones(src);
    let lifetimes: Vec<_> = zones
      .iter()
      .filter(|z| z.kind == RustSkipZoneKind::Lifetime)
      .collect();
    // 'a (twice) + 'static (once) = 3 lifetime zones.
    assert_eq!(lifetimes.len(), 3);
    let bodies: Vec<&str> = lifetimes.iter().map(|z| &src[z.start..z.end]).collect();
    assert!(bodies.contains(&"'a"));
    assert!(bodies.contains(&"'static"));
  }

  #[test]
  fn skip_zones_does_not_match_ident_starting_with_r_or_b() {
    // `rust` and `bar` are regular identifiers, not raw-string or byte-string prefixes.
    let src = "let rust = 1; let bar = 2; let r2d2 = 3;";
    let zones = rust_skip_zones(src);
    // No string / raw-string / byte-string zones.
    assert!(zones.iter().all(|z| !matches!(
      z.kind,
      RustSkipZoneKind::String
        | RustSkipZoneKind::RawString
        | RustSkipZoneKind::ByteString
        | RustSkipZoneKind::RawByteString
    )));
  }

  #[test]
  fn skip_zones_b_or_r_after_ident_char_is_part_of_ident() {
    // `xb"x"` — the `b` here is part of an identifier, NOT a byte-string
    // introducer. Without the boundary check the lexer would
    // misclassify it.
    let src = r#"let xb = "x"; let abr = 0;"#;
    let zones = rust_skip_zones(src);
    // The `"x"` is a regular string (one zone).
    let strings: Vec<_> = zones
      .iter()
      .filter(|z| z.kind == RustSkipZoneKind::String)
      .collect();
    assert_eq!(strings.len(), 1);
    assert_eq!(&src[strings[0].start..strings[0].end], r#""x""#);
  }

  // ─── filter + rust_safe edits ────────────────────────────────────

  #[test]
  fn rust_safe_edits_filter_string_literal_matches() {
    let src = "let foo = 1; let s = \"foo bar\"; let g = foo;";
    let unsafe_edits = compute_file_rename_edits("foo", "renamed", src);
    let safe_edits = compute_file_rename_edits_rust_safe("foo", "renamed", src);
    // Unsafe (token-based) catches all 3 `foo` occurrences.
    assert_eq!(unsafe_edits.len(), 3);
    // Safe excludes the one inside the string literal.
    assert_eq!(safe_edits.len(), 2);
    for e in &safe_edits {
      let in_string = &src[e.byte_offset..e.byte_offset + e.byte_len] == "foo"
        && src.as_bytes().get(e.byte_offset.saturating_sub(1)) == Some(&b'"');
      assert!(!in_string, "edit at {:?} is inside string", e);
    }
  }

  #[test]
  fn rust_safe_edits_filter_line_comment_matches() {
    let src = "let foo = 1;\n// foo is wonderful\nlet bar = foo;\n";
    let safe = compute_file_rename_edits_rust_safe("foo", "renamed", src);
    // Only 2 real occurrences (lines 1 and 3); the comment is filtered.
    assert_eq!(safe.len(), 2);
  }

  #[test]
  fn rust_safe_edits_filter_block_comment_matches() {
    let src = "let foo = 1; /* mention of foo here */ let bar = foo;";
    let safe = compute_file_rename_edits_rust_safe("foo", "renamed", src);
    assert_eq!(safe.len(), 2);
  }

  #[test]
  fn rust_safe_edits_filter_doc_comment_matches() {
    let src = "/// docs that mention foo\nfn foo() {}\nfn other() { foo(); }\n";
    let safe = compute_file_rename_edits_rust_safe("foo", "renamed", src);
    // foo in the doc comment is filtered; the fn definition + call remain.
    assert_eq!(safe.len(), 2);
  }

  #[test]
  fn rust_safe_edits_filter_char_literal_matches() {
    // A char literal can't contain a multi-byte identifier, but it
    // CAN start with the same letters. Sanity: the char zone is
    // skipped regardless of contents.
    let src = "let c = 'f'; let foo = 1;";
    let safe = compute_file_rename_edits_rust_safe("foo", "renamed", src);
    // Only the `foo` ident is matched.
    assert_eq!(safe.len(), 1);
    assert_eq!(safe[0].byte_offset, "let c = 'f'; let ".len());
  }

  #[test]
  fn rust_safe_edits_filter_macro_call_name() {
    let src = "foo!(\"x\");\nfn foo() {}\nfoo();\n";
    let safe = compute_file_rename_edits_rust_safe("foo", "renamed", src);
    // 3 token matches; the `foo!` macro call is filtered.
    assert_eq!(safe.len(), 2);
    // Byte offsets: `fn foo()` and `foo()`.
    let bodies: Vec<&str> = safe
      .iter()
      .map(|e| &src[e.byte_offset..e.byte_offset + e.byte_len])
      .collect();
    assert!(bodies.iter().all(|b| *b == "foo"));
  }

  #[test]
  fn rust_safe_edits_filter_raw_string_matches() {
    let src = r####"let raw = r#"foo inside raw"#; let real = foo;"####;
    let safe = compute_file_rename_edits_rust_safe("foo", "renamed", src);
    // Only the bare `foo` ident.
    assert_eq!(safe.len(), 1);
  }

  #[test]
  fn rust_safe_edits_filter_lifetime_named_foo() {
    let src = "fn f<'foo>(x: &'foo str) -> &'foo str { x }\nfn foo() {}\n";
    let safe = compute_file_rename_edits_rust_safe("foo", "renamed", src);
    // The lifetime `'foo` (3 occurrences) is filtered; only `fn foo()`
    // remains.
    assert_eq!(safe.len(), 1);
  }

  #[test]
  fn rust_safe_edits_preserves_real_identifier_matches() {
    // Sanity: clean source with no strings/comments/macros yields
    // identical results to the token-based walker.
    let src = "fn foo() { foo(); }\n";
    let unsafe_edits = compute_file_rename_edits("foo", "bar", src);
    let safe_edits = compute_file_rename_edits_rust_safe("foo", "bar", src);
    assert_eq!(unsafe_edits, safe_edits);
    assert_eq!(safe_edits.len(), 2);
  }

  // ─── compute_rename_patch_candidate_rust_safe orchestrator ───────

  #[test]
  fn rust_safe_candidate_uses_lexer_for_rust_language() {
    let req = RenameRequest {
      old_name: "foo".to_string(),
      new_name: "bar".to_string(),
      target_paths: vec!["src/a.rs".to_string()],
      language: "rust".to_string(),
      scope: RenameScope::LocalTargetPaths,
      target_fn_name: None,
    };
    let f = RenameFileInput {
      path: "src/a.rs",
      content: "let foo = 1; let s = \"foo\"; let g = foo;",
    };
    let cand = compute_rename_patch_candidate_rust_safe(&req, &[f]);
    assert!(matches!(cand.verdict, RenameVerdict::RenameReady));
    assert_eq!(cand.file_patches.len(), 1);
    // 2 edits (the string occurrence is filtered).
    assert_eq!(cand.file_patches[0].edits.len(), 2);
  }

  #[test]
  fn rust_safe_candidate_falls_back_to_token_based_for_non_rust() {
    let req = RenameRequest {
      old_name: "foo".to_string(),
      new_name: "bar".to_string(),
      target_paths: vec!["src/a.py".to_string()],
      language: "python".to_string(),
      scope: RenameScope::LocalTargetPaths,
      target_fn_name: None,
    };
    let f = RenameFileInput {
      path: "src/a.py",
      // Python "comment" with foo — token-based walker doesn't filter
      // because we don't have a Python lexer yet.
      content: "foo = 1\n# foo is good\nbar = foo\n",
    };
    let cand = compute_rename_patch_candidate_rust_safe(&req, &[f]);
    assert!(matches!(cand.verdict, RenameVerdict::RenameReady));
    // 3 `foo` token matches (no Python lexer to filter the comment).
    assert_eq!(cand.file_patches[0].edits.len(), 3);
  }

  #[test]
  fn rust_safe_candidate_empty_edits_still_yields_ready_verdict() {
    // All `foo` occurrences are inside strings/comments → empty edits
    // but still RenameReady.
    let req = RenameRequest {
      old_name: "foo".to_string(),
      new_name: "bar".to_string(),
      target_paths: vec!["src/a.rs".to_string()],
      language: "rust".to_string(),
      scope: RenameScope::LocalTargetPaths,
      target_fn_name: None,
    };
    let f = RenameFileInput {
      path: "src/a.rs",
      content: "// foo here\nlet s = \"foo\";\n",
    };
    let cand = compute_rename_patch_candidate_rust_safe(&req, &[f]);
    assert!(matches!(cand.verdict, RenameVerdict::RenameReady));
    // No file_patches because no edits survived filtering.
    assert!(cand.file_patches.is_empty());
    assert!(cand.combined_unified_diff.is_empty());
  }

  // ─── review + apply → materialization request bridge ────────────

  fn fixture_review_receipt(decision: RenameReviewDecision) -> RenameReviewReceipt {
    // Build a minimal review receipt anchored to a known candidate.
    let req = RenameRequest {
      old_name: "foo".to_string(),
      new_name: "bar".to_string(),
      target_paths: vec!["src/a.rs".to_string()],
      language: "rust".to_string(),
      scope: RenameScope::LocalTargetPaths,
      target_fn_name: None,
    };
    let f = RenameFileInput {
      path: "src/a.rs",
      content: "fn foo() { foo() }\n",
    };
    let candidate = compute_rename_patch_candidate(&req, &[f]);
    build_rename_review_receipt(
      candidate,
      RenameReviewer {
        actor_id: "reviewer.senior.1".to_string(),
        tenant_id: "tenant.alpha".to_string(),
      },
      decision,
      Some("reviewed for bridge test".to_string()),
      1700000001000,
    )
  }

  fn fixture_apply_receipt_for_bridge() -> RenameApplyReceipt {
    let req = RenameRequest {
      old_name: "foo".to_string(),
      new_name: "bar".to_string(),
      target_paths: vec!["src/a.rs".to_string()],
      language: "rust".to_string(),
      scope: RenameScope::LocalTargetPaths,
      target_fn_name: None,
    };
    let f = RenameFileInput {
      path: "src/a.rs",
      content: "fn foo() { foo() }\n",
    };
    let candidate = compute_rename_patch_candidate(&req, &[f]);
    let candidate_art = build_rename_symbol_patch_candidate_artifact(&candidate, 0, None);
    let approval = RenameApplyApproval {
      actor_id: "applier.junior.5".to_string(),
      tenant_id: "tenant.alpha".to_string(),
      approved_at_ms: 1700000002000,
      candidate_artifact_id: candidate_art["id"].as_str().unwrap().to_string(),
    };
    let sealed = ValidatedRenamePatchCandidate::new_checked(candidate).expect("ready");
    apply_rename_patch_candidate(
      &sealed,
      &[RenameFileInput {
        path: "src/a.rs",
        content: "fn foo() { foo() }\n",
      }],
      &approval,
      1700000002500,
    )
    .expect("apply")
  }

  #[test]
  fn rename_bridge_happy_path_yields_ready_request() {
    let review = fixture_review_receipt(RenameReviewDecision::Approve);
    let apply = fixture_apply_receipt_for_bridge();
    let req = build_rename_materialization_request(
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
      .starts_with("apply-receipt.rename-symbol."));
    // Actor is from apply (the person triggering materialization is
    // the apply approver).
    assert_eq!(req.requested_by_actor_id, "applier.junior.5");
    assert_eq!(req.requested_by_tenant_id, "tenant.alpha");
    assert_eq!(req.repo_snapshot_ref, "git:abc123");
    assert_eq!(req.capability, "edit-within-target-paths");
    assert_eq!(req.deployment_mode, "dev");
    assert_eq!(req.content_policy, "include-content");
    assert_eq!(req.requested_at_ms, 1700000003000);
    // Sanity: the assembled request itself classifies Ready.
    assert!(matches!(
      crate::tool_action::classify_tool_action_materialization_request(&req),
      crate::tool_action::ToolActionMaterializationVerdict::Ready
    ));
  }

  #[test]
  fn rename_bridge_rejects_hold_review() {
    let review = fixture_review_receipt(RenameReviewDecision::Hold);
    let apply = fixture_apply_receipt_for_bridge();
    let err = build_rename_materialization_request(
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
  fn rename_bridge_rejects_reject_review() {
    let review = fixture_review_receipt(RenameReviewDecision::Reject);
    let apply = fixture_apply_receipt_for_bridge();
    assert!(matches!(
      build_rename_materialization_request(
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
  fn rename_bridge_rejects_candidate_mismatch() {
    // Construct an Approve review pointing at a DIFFERENT candidate
    // than the apply receipt. The bridge re-derives the apply's
    // candidate id and detects the mismatch.
    let review_for_a = fixture_review_receipt(RenameReviewDecision::Approve);
    // Apply for a different rename request → different candidate id.
    let other_req = RenameRequest {
      old_name: "xx".to_string(),
      new_name: "yy".to_string(),
      target_paths: vec!["src/a.rs".to_string()],
      language: "rust".to_string(),
      scope: RenameScope::LocalTargetPaths,
      target_fn_name: None,
    };
    let other_file = RenameFileInput {
      path: "src/a.rs",
      content: "fn xx() { xx() }\n",
    };
    let other_candidate = compute_rename_patch_candidate(&other_req, &[other_file]);
    let other_candidate_art =
      build_rename_symbol_patch_candidate_artifact(&other_candidate, 0, None);
    let other_approval = RenameApplyApproval {
      actor_id: "applier.junior.5".to_string(),
      tenant_id: "tenant.alpha".to_string(),
      approved_at_ms: 0,
      candidate_artifact_id: other_candidate_art["id"].as_str().unwrap().to_string(),
    };
    let sealed = ValidatedRenamePatchCandidate::new_checked(other_candidate).expect("ready");
    let other_apply = apply_rename_patch_candidate(
      &sealed,
      &[RenameFileInput {
        path: "src/a.rs",
        content: "fn xx() { xx() }\n",
      }],
      &other_approval,
      0,
    )
    .expect("apply");

    let err = build_rename_materialization_request(
      &review_for_a,
      &other_apply,
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
  fn rename_bridge_rejects_cross_tenant_review_and_apply() {
    // Review tenant = alpha; apply tenant = beta → bridge rejects.
    let req = RenameRequest {
      old_name: "foo".to_string(),
      new_name: "bar".to_string(),
      target_paths: vec!["src/a.rs".to_string()],
      language: "rust".to_string(),
      scope: RenameScope::LocalTargetPaths,
      target_fn_name: None,
    };
    let f = RenameFileInput {
      path: "src/a.rs",
      content: "fn foo() { foo() }\n",
    };
    let candidate = compute_rename_patch_candidate(&req, &[f]);
    let candidate_art = build_rename_symbol_patch_candidate_artifact(&candidate, 0, None);
    let review = build_rename_review_receipt(
      candidate.clone(),
      RenameReviewer {
        actor_id: "reviewer.alpha.1".to_string(),
        tenant_id: "tenant.alpha".to_string(),
      },
      RenameReviewDecision::Approve,
      None,
      0,
    );
    // Cross-tenant apply.
    let approval = RenameApplyApproval {
      actor_id: "applier.beta.1".to_string(),
      tenant_id: "tenant.beta".to_string(),
      approved_at_ms: 0,
      candidate_artifact_id: candidate_art["id"].as_str().unwrap().to_string(),
    };
    let sealed = ValidatedRenamePatchCandidate::new_checked(candidate).expect("ready");
    let apply = apply_rename_patch_candidate(
      &sealed,
      &[RenameFileInput {
        path: "src/a.rs",
        content: "fn foo() { foo() }\n",
      }],
      &approval,
      0,
    )
    .expect("apply");

    let err = build_rename_materialization_request(
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
  fn rename_bridge_allows_different_actor_same_tenant() {
    // Senior reviews; junior applies. Both in tenant.alpha. Bridge
    // accepts (only tenant must match; actor may differ).
    let review = fixture_review_receipt(RenameReviewDecision::Approve);
    // review.reviewer.actor_id = "reviewer.senior.1"
    let apply = fixture_apply_receipt_for_bridge();
    // apply.approval.actor_id = "applier.junior.5"
    assert_ne!(
      review.reviewer.actor_id, apply.approval.actor_id,
      "fixture sanity"
    );
    assert_eq!(
      review.reviewer.tenant_id, apply.approval.tenant_id,
      "same tenant"
    );
    let req = build_rename_materialization_request(
      &review,
      &apply,
      "edit-within-target-paths",
      "git:abc123",
      "dev",
      "include-content",
      0,
    )
    .expect("ready");
    // The materialization request's requested_by_actor_id is the
    // apply approver (the person triggering disk write).
    assert_eq!(req.requested_by_actor_id, "applier.junior.5");
  }

  #[test]
  fn rename_bridge_forwards_classifier_rejected_for_customer_release_with_include() {
    let review = fixture_review_receipt(RenameReviewDecision::Approve);
    let apply = fixture_apply_receipt_for_bridge();
    let err = build_rename_materialization_request(
      &review,
      &apply,
      "edit-within-target-paths",
      "git:abc123",
      "customer-release",
      "include-content", // forbidden combo
      0,
    )
    .expect_err("customer-release + include-content must reject");
    match err {
      crate::tool_action::MaterializationBridgeError::RequestNotReady(verdict) => {
        assert!(matches!(
          verdict,
          crate::tool_action::ToolActionMaterializationVerdict::Rejected {
            held_kind:
              crate::tool_action::ToolActionMaterializationHeldKind::CustomerReleaseForbidsIncludeContent,
            ..
          }
        ));
      }
      other => panic!(
        "expected RequestNotReady(customer-release leak), got {:?}",
        other
      ),
    }
  }

  #[test]
  fn rename_bridge_forwards_classifier_held_for_read_only_capability() {
    let review = fixture_review_receipt(RenameReviewDecision::Approve);
    let apply = fixture_apply_receipt_for_bridge();
    let err = build_rename_materialization_request(
      &review,
      &apply,
      "read-only", // can't materialize
      "git:abc123",
      "dev",
      "include-content",
      0,
    )
    .expect_err("read-only must hold");
    assert!(matches!(
      err,
      crate::tool_action::MaterializationBridgeError::RequestNotReady(
        crate::tool_action::ToolActionMaterializationVerdict::Held {
          held_kind:
            crate::tool_action::ToolActionMaterializationHeldKind::ReadOnlyCapabilityCannotMaterialize,
          ..
        }
      )
    ));
  }

  #[test]
  fn rust_safe_candidate_held_verdict_short_circuits() {
    // A held request returns the verdict without computing any edits,
    // matching `compute_rename_patch_candidate` behavior.
    let req = RenameRequest {
      old_name: String::new(), // missing → Held
      new_name: "bar".to_string(),
      target_paths: vec!["src/a.rs".to_string()],
      language: "rust".to_string(),
      scope: RenameScope::LocalTargetPaths,
      target_fn_name: None,
    };
    let f = RenameFileInput {
      path: "src/a.rs",
      content: "let foo = 1;",
    };
    let cand = compute_rename_patch_candidate_rust_safe(&req, &[f]);
    assert!(matches!(cand.verdict, RenameVerdict::RenameHeld { .. }));
    assert!(cand.file_patches.is_empty());
  }

  // ─── Python skip-zone tests ──────────────────────────────────────

  #[test]
  fn python_skip_zones_line_comment() {
    let src = "x = foo  # foo is wonderful\ny = foo\n";
    let zones = python_skip_zones(src);
    let comments: Vec<_> = zones.iter().filter(|z| z.kind == "line-comment").collect();
    assert_eq!(comments.len(), 1);
    assert!(src[comments[0].start..comments[0].end].starts_with("# foo"));
  }

  #[test]
  fn python_skip_zones_single_and_double_quoted_strings() {
    let src = r#"a = "foo"; b = 'foo'; c = foo"#;
    let zones = python_skip_zones(src);
    let strs: Vec<_> = zones.iter().filter(|z| z.kind == "string").collect();
    assert_eq!(strs.len(), 2);
    let bodies: Vec<&str> = strs.iter().map(|z| &src[z.start..z.end]).collect();
    assert!(bodies.iter().any(|b| *b == "\"foo\""));
    assert!(bodies.iter().any(|b| *b == "'foo'"));
  }

  #[test]
  fn python_skip_zones_triple_quoted_strings() {
    let src = "x = \"\"\"foo inside triple\"\"\" ; y = foo";
    let zones = python_skip_zones(src);
    let triples: Vec<_> = zones.iter().filter(|z| z.kind == "triple-string").collect();
    assert_eq!(triples.len(), 1);
    assert_eq!(
      &src[triples[0].start..triples[0].end],
      "\"\"\"foo inside triple\"\"\""
    );
  }

  #[test]
  fn python_skip_zones_triple_single_quoted_strings() {
    let src = "x = '''foo inside triple''' ; y = foo";
    let zones = python_skip_zones(src);
    let triples: Vec<_> = zones.iter().filter(|z| z.kind == "triple-string").collect();
    assert_eq!(triples.len(), 1);
    assert_eq!(
      &src[triples[0].start..triples[0].end],
      "'''foo inside triple'''"
    );
  }

  #[test]
  fn python_skip_zones_prefixed_strings() {
    // r"...", b"...", f"...", rb"..."
    let src = r#"a = r"foo raw"; b = b"foo byte"; c = f"foo fstring"; d = rb"foo rb"; e = foo"#;
    let zones = python_skip_zones(src);
    let prefixed: Vec<_> = zones
      .iter()
      .filter(|z| z.kind == "prefixed-string")
      .collect();
    assert_eq!(prefixed.len(), 4);
  }

  #[test]
  fn python_skip_zones_does_not_match_ident_starting_with_r_b_f() {
    // `range`, `bytes`, `foo` are normal identifiers, not prefixed strings.
    let src = "range = 1; bytes = 2; foo = 3";
    let zones = python_skip_zones(src);
    assert!(zones.iter().all(|z| !z.kind.contains("string")));
  }

  #[test]
  fn python_safe_edits_filter_string_and_comment() {
    let src = "foo = 1\n# foo wonderful\nbar = \"foo\"\nbaz = foo\n";
    let unsafe_edits = compute_file_rename_edits("foo", "renamed", src);
    let safe = compute_file_rename_edits_python_safe("foo", "renamed", src);
    // 4 token matches → only 2 real identifiers (line 1 and 4).
    assert_eq!(unsafe_edits.len(), 4);
    assert_eq!(safe.len(), 2);
  }

  #[test]
  fn python_safe_edits_filter_triple_quoted_docstring() {
    let src =
      "def foo():\n    \"\"\"foo is the function name in this docstring\"\"\"\n    return foo()\n";
    let safe = compute_file_rename_edits_python_safe("foo", "bar", src);
    // 3 token matches; the one in the docstring is filtered.
    assert_eq!(safe.len(), 2);
  }

  #[test]
  fn python_safe_edits_filter_f_string_body() {
    // F-string body is treated opaquely in phase 1 — the whole zone
    // including interpolated `{foo}` is skipped.
    let src = "x = f\"value is {foo}\"\ny = foo\n";
    let safe = compute_file_rename_edits_python_safe("foo", "bar", src);
    // Only the bare `foo` on line 2 survives.
    assert_eq!(safe.len(), 1);
  }

  // ─── pnix `.px` skip-zone tests ───────────────────────────────

  #[test]
  fn pnix_skip_zones_line_comment_extends_to_eol() {
    let src = "let x = foo;  # foo is the answer\ny = foo;\n";
    let zones = pnix_skip_zones(src);
    let comments: Vec<_> = zones.iter().filter(|z| z.kind == "line-comment").collect();
    assert_eq!(comments.len(), 1);
    assert!(src[comments[0].start..comments[0].end].starts_with("# foo"));
  }

  #[test]
  fn pnix_skip_zones_regular_string() {
    let src = r#"a = "foo"; b = foo;"#;
    let zones = pnix_skip_zones(src);
    let strs: Vec<_> = zones.iter().filter(|z| z.kind == "string").collect();
    assert_eq!(strs.len(), 1);
    assert_eq!(&src[strs[0].start..strs[0].end], "\"foo\"");
  }

  #[test]
  fn pnix_skip_zones_backslash_escape_does_not_close_string() {
    // `\"` inside a string is an escaped quote, not the closing quote.
    let src = r#"a = "she said \"foo\" loudly"; b = foo;"#;
    let zones = pnix_skip_zones(src);
    let strs: Vec<_> = zones.iter().filter(|z| z.kind == "string").collect();
    assert_eq!(strs.len(), 1, "escapes must not close the string");
    let body = &src[strs[0].start..strs[0].end];
    assert!(body.ends_with("loudly\""), "body was: {body:?}");
  }

  #[test]
  fn pnix_skip_zones_indented_string() {
    // Nix-style `''...''` indented string.
    let src = "x = ''\n  foo block\n  foo line two\n''; y = foo;";
    let zones = pnix_skip_zones(src);
    let indented: Vec<_> = zones
      .iter()
      .filter(|z| z.kind == "indented-string")
      .collect();
    assert_eq!(indented.len(), 1);
    let body = &src[indented[0].start..indented[0].end];
    assert!(body.starts_with("''"));
    assert!(body.ends_with("''"));
    assert!(body.contains("foo block"));
  }

  #[test]
  fn pnix_skip_zones_indented_string_triple_apostrophe_escape() {
    // `'''` inside an indented string is a literal `'` — it must not
    // close the string. The scanner should consume `'''` and keep
    // scanning until the real `''` terminator.
    let src = "x = ''before '''embed''' after''; y = foo;";
    let zones = pnix_skip_zones(src);
    let indented: Vec<_> = zones
      .iter()
      .filter(|z| z.kind == "indented-string")
      .collect();
    assert_eq!(indented.len(), 1, "embedded `'''` must not terminate");
    let body = &src[indented[0].start..indented[0].end];
    // The whole `''before '''embed''' after''` slice should be one zone.
    assert!(body.contains("embed"));
    assert!(body.ends_with("after''"));
  }

  #[test]
  fn pnix_safe_edits_filter_string_and_comment_occurrences() {
    let src = "\
let
  foo = 1;
  # foo is mentioned in a comment
  bar = \"foo inside string\";
  baz = foo;
in foo
";
    let unsafe_edits = compute_file_rename_edits("foo", "renamed", src);
    let safe = compute_file_rename_edits_pnix_safe("foo", "renamed", src);
    // 5 raw whole-word matches: binding decl, comment mention,
    // string body, body reference (`baz = foo;`), and `in foo`.
    // Skip zones filter the comment + string occurrences; 3 real
    // identifier sites survive.
    assert_eq!(
      unsafe_edits.len(),
      5,
      "raw walker should see 5 whole-word matches; got {}",
      unsafe_edits.len()
    );
    assert_eq!(
      safe.len(),
      3,
      "skip zones must filter the comment-mention and the string-body occurrence; got {}",
      safe.len()
    );
  }

  #[test]
  fn pnix_safe_edits_filter_indented_string_body() {
    let src = "\
let
  foo = ''
    docstring talking about foo at length
  '';
in foo
";
    let safe = compute_file_rename_edits_pnix_safe("foo", "bar", src);
    // 3 raw matches; one is inside the indented string, filtered.
    assert_eq!(safe.len(), 2);
  }

  #[test]
  fn pnix_lang_safe_dispatcher_routes_pnix_language() {
    let src = "let foo = 1; in # foo here\n  foo + 2";
    let edits = compute_file_rename_edits_lang_safe("pnix", "foo", "bar", src);
    // 2 real `foo` identifier sites (let-binding + body), 1 in
    // comment which the pnix dispatch must skip.
    assert_eq!(edits.len(), 2);
  }

  #[test]
  fn pnix_safe_edits_full_patch_candidate_renames_let_binding() {
    // End-to-end through compute_rename_patch_candidate_lang_safe —
    // proves the language string "pnix" routes through the dispatcher
    // and emits a clean unified diff.
    let request = RenameRequest {
      old_name: "foo".to_string(),
      new_name: "bar".to_string(),
      target_paths: vec!["stdlib/lib/example.px".to_string()],
      language: "pnix".to_string(),
      scope: RenameScope::LocalTargetPaths,
      target_fn_name: None,
    };
    let src = "\
let
  foo = 1;
  # don't rename inside the comment: foo
  msg = \"don't rename inside string: foo\";
in foo + foo
";
    let file_input = RenameFileInput {
      path: "stdlib/lib/example.px",
      content: src,
    };
    let cand = compute_rename_patch_candidate_lang_safe(&request, &[file_input]);
    assert!(matches!(cand.verdict, RenameVerdict::RenameReady));
    assert_eq!(cand.file_patches.len(), 1);
    let edits = &cand.file_patches[0].edits;
    // foo as binding + two `foo` in `in foo + foo` = 3 real edit sites.
    assert_eq!(edits.len(), 3, "got edits: {edits:?}");
    let diff = &cand.combined_unified_diff;
    assert!(diff.contains("-  foo = 1;"));
    assert!(diff.contains("+  bar = 1;"));
    // String/comment occurrences not in the diff.
    assert!(!diff.contains("-  # don't rename"));
    assert!(!diff.contains("-  msg ="));
  }

  // ─── Rust scope-aware rename (D-22) ────────────────────────────

  #[test]
  fn find_rust_function_scopes_extracts_simple_fn() {
    let src = "fn foo() { let x = 1; x + 2 }\n";
    let scopes = find_rust_function_scopes(src);
    assert_eq!(scopes.len(), 1);
    assert_eq!(scopes[0].name, "foo");
    assert_eq!(
      &src[scopes[0].body_open_brace..=scopes[0].body_open_brace],
      "{"
    );
    assert_eq!(
      &src[scopes[0].body_close_brace_exclusive - 1..scopes[0].body_close_brace_exclusive],
      "}"
    );
  }

  #[test]
  fn find_rust_function_scopes_extracts_multiple_fns() {
    let src = "\
fn foo() {
  let x = 1;
}
fn bar(a: i32) -> i32 {
  a + 1
}
";
    let scopes = find_rust_function_scopes(src);
    assert_eq!(scopes.len(), 2);
    assert_eq!(scopes[0].name, "foo");
    assert_eq!(scopes[1].name, "bar");
    // Bodies are disjoint and ordered.
    assert!(scopes[0].body_close_brace_exclusive < scopes[1].body_open_brace);
  }

  #[test]
  fn find_rust_function_scopes_skips_fn_in_comments_and_strings() {
    let src = "\
// fn looks_like_fn() not real
fn real_fn() {
  let s = \"fn also_not_real()\";
  let _ = s;
}
";
    let scopes = find_rust_function_scopes(src);
    assert_eq!(scopes.len(), 1);
    assert_eq!(scopes[0].name, "real_fn");
  }

  #[test]
  fn find_rust_function_scopes_skips_trait_method_signature_without_body() {
    // `fn method();` is a declaration without a body — not a scope.
    let src = "\
trait T {
  fn method(self);
}
fn real() { let x = 1; }
";
    let scopes = find_rust_function_scopes(src);
    // `real` has a body; `method` does not.
    let names: Vec<&str> = scopes.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"real"));
    assert!(
      !names.contains(&"method"),
      "method without body must not be a scope; got names: {names:?}"
    );
  }

  #[test]
  fn find_rust_function_scopes_handles_nested_fn_via_brace_balance() {
    // Inner `fn inner` lives inside `outer`'s body — both should
    // be detected. v0 doesn't analyze nesting recursively but the
    // brace-balanced walker still surfaces both as separate scopes.
    let src = "\
fn outer() {
  fn inner() {
    let z = 1;
  }
  inner();
}
";
    let scopes = find_rust_function_scopes(src);
    let names: Vec<&str> = scopes.iter().map(|s| s.name.as_str()).collect();
    // v0: outer found; inner may or may not be — main contract is
    // that the outer body is correctly delimited (the inner `}`
    // doesn't prematurely close outer).
    assert!(names.contains(&"outer"));
    let outer = scopes.iter().find(|s| s.name == "outer").unwrap();
    // Outer body must contain the entire inner declaration.
    let outer_body = &src[outer.body_open_brace..outer.body_close_brace_exclusive];
    assert!(outer_body.contains("fn inner()"));
  }

  #[test]
  fn rust_scope_aware_rename_targets_body_only_skips_other_fn() {
    let src = "\
fn foo() {
  let foo = 1;
  let n = foo + 2;
}
fn bar() {
  let foo = 99;
  let n = foo + 3;
}
";
    // Rename `foo` → `renamed`, restricted to `foo` fn body.
    let edits =
      compute_file_rename_edits_rust_scope_aware("foo", "renamed", src, "foo").expect("ok");
    // Inside foo body, two `foo` occurrences: `let foo = 1` and
    // `foo + 2`. The fn declaration's `foo` is BEFORE body_open
    // → not in edits. `bar` body's `foo` is in a different fn →
    // not in edits.
    assert_eq!(edits.len(), 2);
    // Apply edits and confirm.
    let renamed = apply_rename_edits(src, &edits, "renamed");
    // foo body's two occurrences renamed.
    assert!(renamed.contains("let renamed = 1"));
    assert!(renamed.contains("let n = renamed + 2"));
    // bar body's `foo` UNTOUCHED.
    assert!(renamed.contains("let foo = 99"));
    assert!(renamed.contains("let n = foo + 3"));
    // The fn declaration itself untouched (the `foo` in `fn foo()`).
    assert!(renamed.contains("fn foo() {"));
  }

  #[test]
  fn rust_scope_aware_rename_returns_target_not_found_when_fn_absent() {
    let src = "fn other() { let x = 1; }\n";
    let r = compute_file_rename_edits_rust_scope_aware("x", "y", src, "nonexistent_fn");
    assert_eq!(r.unwrap_err(), RustScopeAwareError::TargetFunctionNotFound);
  }

  #[test]
  fn rust_scope_aware_rename_returns_multiple_targets_when_duplicate_fn_names() {
    let src = "\
mod a { fn dup() { let x = 1; } }
mod b { fn dup() { let x = 2; } }
";
    let r = compute_file_rename_edits_rust_scope_aware("x", "y", src, "dup");
    assert_eq!(
      r.unwrap_err(),
      RustScopeAwareError::MultipleTargetFunctions { count: 2 }
    );
  }

  #[test]
  fn rust_scope_aware_rename_preserves_skip_zone_safety() {
    // Inside the target body, a `foo` inside a `//` comment or
    // `"..."` string must STILL not be touched (the underlying
    // rust-safe walker filters those zones).
    let src = "\
fn target() {
  let foo = 1;
  // a comment mentioning foo
  let s = \"another foo\";
  let n = foo + 2;
}
";
    let edits = compute_file_rename_edits_rust_scope_aware("foo", "x", src, "target").expect("ok");
    // Only 2 real occurrences inside the body: let-binding + body
    // reference. Comment + string filtered by rust skip zones.
    assert_eq!(edits.len(), 2);
  }

  #[test]
  fn rust_scope_aware_rename_skips_fn_declaration_token_itself() {
    // Renaming `target` inside fn `target` does NOT rename the fn
    // declaration's `target` (it's outside the body). The body has
    // no `target` references → 0 edits returned (different from
    // an error — caller can decide what 0-edit means).
    let src = "fn target() {\n  let x = 1;\n}\n";
    let edits =
      compute_file_rename_edits_rust_scope_aware("target", "renamed", src, "target").expect("ok");
    assert_eq!(edits.len(), 0);
  }

  // ─── D-23: scope-aware rename via compute_*_lang_safe ──────────

  #[test]
  fn lang_safe_carrier_routes_to_scope_aware_when_target_fn_name_some() {
    let src = "\
fn target() {
  let foo = 1;
  let n = foo + 2;
}
fn other() {
  let foo = 99;
  let n = foo + 3;
}
";
    let mut request = req(
      "foo",
      "renamed",
      &["src/a.rs"],
      "rust",
      RenameScope::LocalTargetPaths,
    );
    request.target_fn_name = Some("target".to_string());
    let file_input = RenameFileInput {
      path: "src/a.rs",
      content: src,
    };
    let cand = compute_rename_patch_candidate_lang_safe(&request, &[file_input]);
    assert!(matches!(cand.verdict, RenameVerdict::RenameReady));
    assert_eq!(cand.scope_aware_error, None);
    assert_eq!(cand.file_patches.len(), 1);
    // Only 2 edits inside target body.
    assert_eq!(cand.file_patches[0].edits.len(), 2);
    let diff = &cand.combined_unified_diff;
    // Renamed lines from target body.
    assert!(diff.contains("+  let renamed = 1;"));
    // `other` body's `foo` UNTOUCHED.
    assert!(!diff.contains("+  let renamed = 99"));
  }

  #[test]
  fn scope_aware_supported_set_aligns_with_rename_classifier_supported_set() {
    // Invariant: every language the scope-aware pipeline accepts
    // must also be accepted by the rename classifier
    // (`SUPPORTED_LANGUAGES`). When this drifts — e.g. classifier
    // adds a language that scope-aware doesn't cover yet — the
    // carrier's scope_aware_error branch is reachable again and
    // this test should be replaced with a positive Held-emission
    // test for that lang.
    let scope_aware = ["rust", "typescript", "javascript", "go", "python"];
    for lang in scope_aware {
      assert!(
        SUPPORTED_LANGUAGES.contains(&lang),
        "lang `{lang}` is scope-aware but not in classifier SUPPORTED_LANGUAGES"
      );
    }
  }

  #[test]
  fn lang_safe_carrier_emits_scope_aware_error_when_target_fn_not_found() {
    let mut request = req(
      "foo",
      "renamed",
      &["src/a.rs"],
      "rust",
      RenameScope::LocalTargetPaths,
    );
    request.target_fn_name = Some("no_such_fn".to_string());
    let file_input = RenameFileInput {
      path: "src/a.rs",
      content: "fn other() { let foo = 1; }\n",
    };
    let cand = compute_rename_patch_candidate_lang_safe(&request, &[file_input]);
    assert!(matches!(cand.verdict, RenameVerdict::RenameReady));
    assert!(cand.file_patches.is_empty());
    let err = cand.scope_aware_error.expect("error");
    assert!(err.contains("no `fn no_such_fn"));
  }

  #[test]
  fn lang_safe_carrier_whole_file_when_target_fn_name_none() {
    // Backward compat: target_fn_name omitted → whole-file rename
    // (original behavior). The Default field's `None` default means
    // existing callers see no change.
    let request = req(
      "foo",
      "renamed",
      &["src/a.rs"],
      "rust",
      RenameScope::LocalTargetPaths,
    );
    assert_eq!(request.target_fn_name, None);
    let file_input = RenameFileInput {
      path: "src/a.rs",
      content: "fn foo() { let x = foo(); }\n",
    };
    let cand = compute_rename_patch_candidate_lang_safe(&request, &[file_input]);
    assert!(matches!(cand.verdict, RenameVerdict::RenameReady));
    assert!(cand.scope_aware_error.is_none());
    // Both `foo` occurrences (fn name + body call) renamed.
    assert_eq!(cand.file_patches[0].edits.len(), 2);
  }

  #[test]
  fn payload_carries_target_fn_name_when_set() {
    // After D-25, python IS supported scope-aware-wise, so no
    // `scope_aware_error` for a python request with a valid target
    // fn. Test now confirms only that `target_fn_name` echoes in
    // the payload.
    let mut request = req(
      "foo",
      "renamed",
      &["x.py"],
      "python",
      RenameScope::LocalTargetPaths,
    );
    request.target_fn_name = Some("target".to_string());
    let file_input = RenameFileInput {
      path: "x.py",
      content: "def target():\n    foo = 1\n    return foo\n",
    };
    let cand = compute_rename_patch_candidate_lang_safe(&request, &[file_input]);
    let payload = build_rename_symbol_patch_candidate_payload(&cand);
    assert_eq!(payload["target_fn_name"].as_str(), Some("target"));
    // No scope_aware_error since python is now supported and the
    // target fn exists.
    assert!(payload.get("scope_aware_error").is_none());
  }

  #[test]
  fn payload_carries_scope_aware_error_when_target_fn_not_found_in_python() {
    let mut request = req(
      "foo",
      "renamed",
      &["x.py"],
      "python",
      RenameScope::LocalTargetPaths,
    );
    request.target_fn_name = Some("no_such".to_string());
    let file_input = RenameFileInput {
      path: "x.py",
      content: "def other():\n    foo = 1\n",
    };
    let cand = compute_rename_patch_candidate_lang_safe(&request, &[file_input]);
    let payload = build_rename_symbol_patch_candidate_payload(&cand);
    assert!(payload["scope_aware_error"].is_string());
    assert!(payload["scope_aware_error"]
      .as_str()
      .unwrap()
      .contains("no `fn no_such"));
  }

  #[test]
  fn payload_omits_scope_aware_fields_when_unset() {
    let request = req(
      "foo",
      "renamed",
      &["src/a.rs"],
      "rust",
      RenameScope::LocalTargetPaths,
    );
    let file_input = RenameFileInput {
      path: "src/a.rs",
      content: "fn foo() { }\n",
    };
    let cand = compute_rename_patch_candidate_lang_safe(&request, &[file_input]);
    let payload = build_rename_symbol_patch_candidate_payload(&cand);
    assert!(payload.get("target_fn_name").is_none());
    assert!(payload.get("scope_aware_error").is_none());
  }

  #[test]
  fn request_deserializes_with_default_target_fn_name() {
    // Backward compat: existing callers that omit `target_fn_name`
    // get the `None` default.
    let json = serde_json::json!({
      "old_name": "foo",
      "new_name": "bar",
      "target_paths": ["src/a.rs"],
      "language": "rust",
      "scope": "local-target-paths"
    });
    let req: RenameRequest = serde_json::from_value(json).expect("deserialize");
    assert_eq!(req.target_fn_name, None);
  }

  // ─── D-24: multi-language scope-aware rename ──────────────────

  #[test]
  fn find_typescript_function_scopes_extracts_named_fn() {
    let src = "\
function target(x: number) {
  let foo = 1;
  return foo + x;
}
function other(y: number) {
  let foo = 99;
  return foo + y;
}
";
    let scopes = find_typescript_function_scopes(src);
    assert_eq!(scopes.len(), 2);
    assert_eq!(scopes[0].name, "target");
    assert_eq!(scopes[1].name, "other");
  }

  #[test]
  fn find_typescript_function_scopes_skips_function_in_comment_or_string() {
    let src = "\
// function fake() { not real }
function real() {
  let s = \"function ghost() not real\";
}
";
    let scopes = find_typescript_function_scopes(src);
    assert_eq!(scopes.len(), 1);
    assert_eq!(scopes[0].name, "real");
  }

  #[test]
  fn ts_scope_aware_rename_drops_other_fn_occurrences() {
    let src = "\
function target() {
  let foo = 1;
  return foo + 2;
}
function other() {
  let foo = 99;
  return foo + 3;
}
";
    let edits = compute_file_rename_edits_typescript_scope_aware("foo", "renamed", src, "target")
      .expect("ok");
    assert_eq!(edits.len(), 2);
    let after = apply_rename_edits(src, &edits, "renamed");
    assert!(after.contains("let renamed = 1"));
    assert!(after.contains("return renamed + 2"));
    // other body unchanged.
    assert!(after.contains("let foo = 99"));
    assert!(after.contains("return foo + 3"));
  }

  #[test]
  fn js_scope_aware_rename_drops_other_fn_occurrences() {
    // Identical conventions as TS in v0.
    let src = "\
function target() {
  var foo = 1;
  return foo;
}
function other() {
  var foo = 99;
  return foo;
}
";
    let edits =
      compute_file_rename_edits_javascript_scope_aware("foo", "x", src, "target").expect("ok");
    assert_eq!(edits.len(), 2);
  }

  #[test]
  fn find_go_function_scopes_extracts_func_and_receiver_method() {
    let src = "\
package main

func plain() {
  x := 1
  _ = x
}

func (r *Rec) method() {
  y := 2
  _ = y
}
";
    let scopes = find_go_function_scopes(src);
    let names: Vec<&str> = scopes.iter().map(|s| s.name.as_str()).collect();
    assert!(
      names.contains(&"plain"),
      "plain func must be detected; got: {names:?}"
    );
    assert!(
      names.contains(&"method"),
      "receiver method `method` must be detected past `(r *Rec)`; got: {names:?}"
    );
  }

  #[test]
  fn go_scope_aware_rename_targets_named_func_only() {
    let src = "\
package main

func target() {
  foo := 1
  _ = foo
}

func other() {
  foo := 99
  _ = foo
}
";
    let edits =
      compute_file_rename_edits_go_scope_aware("foo", "renamed", src, "target").expect("ok");
    assert_eq!(edits.len(), 2);
    let after = apply_rename_edits(src, &edits, "renamed");
    assert!(after.contains("renamed := 1"));
    assert!(after.contains("foo := 99"));
  }

  #[test]
  fn go_scope_aware_rename_targets_receiver_method_body() {
    let src = "\
package main

func (r *Rec) target() {
  foo := 1
  _ = foo
}

func other() {
  foo := 99
}
";
    let edits = compute_file_rename_edits_go_scope_aware("foo", "x", src, "target").expect("ok");
    assert_eq!(
      edits.len(),
      2,
      "receiver-method body must isolate the rename"
    );
  }

  #[test]
  fn lang_safe_carrier_routes_ts_scope_aware_when_language_typescript() {
    let mut request = req(
      "foo",
      "renamed",
      &["x.ts"],
      "typescript",
      RenameScope::LocalTargetPaths,
    );
    request.target_fn_name = Some("target".to_string());
    let src = "\
function target() {
  let foo = 1;
  return foo + 2;
}
function other() {
  let foo = 99;
  return foo + 3;
}
";
    let file_input = RenameFileInput {
      path: "x.ts",
      content: src,
    };
    let cand = compute_rename_patch_candidate_lang_safe(&request, &[file_input]);
    assert!(matches!(cand.verdict, RenameVerdict::RenameReady));
    assert_eq!(cand.scope_aware_error, None);
    assert_eq!(cand.file_patches[0].edits.len(), 2);
  }

  #[test]
  fn lang_safe_carrier_routes_go_scope_aware_when_language_go() {
    let mut request = req(
      "foo",
      "x",
      &["main.go"],
      "go",
      RenameScope::LocalTargetPaths,
    );
    request.target_fn_name = Some("target".to_string());
    let src = "\
package main

func target() {
  foo := 1
  _ = foo
}

func other() {
  foo := 99
}
";
    let file_input = RenameFileInput {
      path: "main.go",
      content: src,
    };
    let cand = compute_rename_patch_candidate_lang_safe(&request, &[file_input]);
    assert!(matches!(cand.verdict, RenameVerdict::RenameReady));
    assert_eq!(cand.scope_aware_error, None);
    assert_eq!(cand.file_patches[0].edits.len(), 2);
  }

  #[test]
  fn lang_safe_carrier_accepts_python_scope_aware_after_d25() {
    // D-25: Python joined the scope-aware supported set via
    // `find_python_function_scopes` (indent-based detector).
    let mut request = req(
      "foo",
      "x",
      &["x.py"],
      "python",
      RenameScope::LocalTargetPaths,
    );
    request.target_fn_name = Some("target".to_string());
    let src = "\
def target():
    foo = 1
    return foo + 2

def other():
    foo = 99
    return foo + 3
";
    let file_input = RenameFileInput {
      path: "x.py",
      content: src,
    };
    let cand = compute_rename_patch_candidate_lang_safe(&request, &[file_input]);
    assert!(matches!(cand.verdict, RenameVerdict::RenameReady));
    assert_eq!(cand.scope_aware_error, None);
    assert_eq!(cand.file_patches.len(), 1);
    assert_eq!(cand.file_patches[0].edits.len(), 2);
    let diff = &cand.combined_unified_diff;
    assert!(diff.contains("+    x = 1"));
    assert!(diff.contains("+    return x + 2"));
    // Other body's `foo` UNTOUCHED.
    assert!(!diff.contains("+    x = 99"));
  }

  // ─── D-26: TS/JS arrow function scope-aware rename ────────────

  #[test]
  fn find_typescript_arrow_function_scope_const_decl() {
    let src = "\
const target = (x: number) => {
  let foo = 1;
  return foo + x;
};
const other = (y: number) => {
  let foo = 99;
  return foo + y;
};
";
    let scopes = find_typescript_function_scopes(src);
    let names: Vec<&str> = scopes.iter().map(|s| s.name.as_str()).collect();
    assert!(
      names.contains(&"target"),
      "arrow `target` must be detected; got {names:?}"
    );
    assert!(names.contains(&"other"));
  }

  #[test]
  fn find_typescript_arrow_function_skips_expression_body() {
    // `=> expr` (no braces) — no body, not a scope-rename target.
    let src = "\
const inc = (n: number) => n + 1;
const target = (x: number) => {
  let foo = 1;
};
";
    let scopes = find_typescript_function_scopes(src);
    let names: Vec<&str> = scopes.iter().map(|s| s.name.as_str()).collect();
    // `inc` has no `{}` body; it must NOT appear.
    assert!(
      !names.contains(&"inc"),
      "expression-body arrow must be skipped; got {names:?}"
    );
    assert!(names.contains(&"target"));
  }

  #[test]
  fn find_typescript_arrow_function_handles_async_modifier() {
    let src = "\
const target = async (x: number) => {
  let foo = 1;
  return foo + x;
};
";
    let scopes = find_typescript_function_scopes(src);
    let names: Vec<&str> = scopes.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"target"));
  }

  #[test]
  fn find_typescript_arrow_function_with_let_and_var_keywords() {
    let src = "\
let lf = (x) => { return x; };
var vf = (y) => { return y; };
";
    let scopes = find_typescript_function_scopes(src);
    let names: Vec<&str> = scopes.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"lf"));
    assert!(names.contains(&"vf"));
  }

  #[test]
  fn ts_scope_aware_rename_targets_arrow_function_body() {
    let src = "\
const target = (x) => {
  let foo = 1;
  return foo + x;
};
const other = (y) => {
  let foo = 99;
  return foo + y;
};
";
    let edits = compute_file_rename_edits_typescript_scope_aware("foo", "renamed", src, "target")
      .expect("ok");
    assert_eq!(edits.len(), 2);
    let after = apply_rename_edits(src, &edits, "renamed");
    assert!(after.contains("let renamed = 1"));
    assert!(after.contains("return renamed + x"));
    // other arrow's `foo` UNTOUCHED.
    assert!(after.contains("let foo = 99"));
    assert!(after.contains("return foo + y"));
  }

  #[test]
  fn ts_scope_aware_rename_with_arrow_skips_function_keyword_form() {
    // Even when both arrow and function-keyword forms coexist with
    // the same name, target_fn_name picks one and isolates rename.
    let src = "\
const target = (x) => {
  let foo = 1;
};
function target2() {
  let foo = 99;
}
";
    let edits =
      compute_file_rename_edits_typescript_scope_aware("foo", "x", src, "target").expect("ok");
    assert_eq!(
      edits.len(),
      1,
      "only arrow target's foo, not target2 fn body"
    );
  }

  #[test]
  fn js_scope_aware_rename_arrow_form_via_lang_safe_carrier() {
    // End-to-end through compute_rename_patch_candidate_lang_safe
    // — proves arrow detection works through dispatcher entry.
    let mut request = req(
      "foo",
      "renamed",
      &["x.js"],
      "javascript",
      RenameScope::LocalTargetPaths,
    );
    request.target_fn_name = Some("arrowed".to_string());
    let src = "\
const arrowed = (x) => {
  var foo = 1;
  return foo;
};
const other = (y) => {
  var foo = 99;
};
";
    let file_input = RenameFileInput {
      path: "x.js",
      content: src,
    };
    let cand = compute_rename_patch_candidate_lang_safe(&request, &[file_input]);
    assert!(matches!(cand.verdict, RenameVerdict::RenameReady));
    assert_eq!(cand.scope_aware_error, None);
    assert_eq!(cand.file_patches[0].edits.len(), 2);
  }

  // ─── D-25: Python scope-aware rename ──────────────────────────

  #[test]
  fn find_python_function_scopes_extracts_simple_def() {
    let src = "\
def target():
    x = 1
    return x

def other():
    y = 2
";
    let scopes = find_python_function_scopes(src);
    assert_eq!(scopes.len(), 2);
    assert_eq!(scopes[0].name, "target");
    assert_eq!(scopes[1].name, "other");
    // target body must include both indented lines.
    let target_body = &src[scopes[0].body_start..scopes[0].body_end_exclusive];
    assert!(target_body.contains("x = 1"));
    assert!(target_body.contains("return x"));
    // target body must NOT include `def other` or the blank line
    // separator's `def` keyword.
    assert!(!target_body.contains("def other"));
  }

  #[test]
  fn find_python_function_scopes_recognizes_async_def() {
    let src = "\
async def target():
    x = 1
";
    let scopes = find_python_function_scopes(src);
    assert_eq!(scopes.len(), 1);
    assert_eq!(scopes[0].name, "target");
    let body = &src[scopes[0].body_start..scopes[0].body_end_exclusive];
    assert!(body.contains("x = 1"));
  }

  #[test]
  fn find_python_function_scopes_handles_multi_line_signature() {
    let src = "\
def target(
    a,
    b,
):
    x = a + b
    return x
";
    let scopes = find_python_function_scopes(src);
    assert_eq!(scopes.len(), 1);
    assert_eq!(scopes[0].name, "target");
    let body = &src[scopes[0].body_start..scopes[0].body_end_exclusive];
    assert!(body.contains("x = a + b"));
    assert!(!body.contains("def target("));
  }

  #[test]
  fn find_python_function_scopes_blank_lines_within_body_continue_scope() {
    let src = "\
def target():
    x = 1

    # comment line — still in body
    y = 2

def other():
    z = 3
";
    let scopes = find_python_function_scopes(src);
    let target = scopes.iter().find(|s| s.name == "target").expect("target");
    let body = &src[target.body_start..target.body_end_exclusive];
    assert!(body.contains("x = 1"));
    assert!(body.contains("y = 2"));
    assert!(!body.contains("def other"));
  }

  #[test]
  fn find_python_function_scopes_skips_def_in_string_or_comment() {
    let src = "\
# def fake(): not real
x = \"def ghost(): not real\"
def real():
    y = 1
";
    let scopes = find_python_function_scopes(src);
    assert_eq!(scopes.len(), 1);
    assert_eq!(scopes[0].name, "real");
  }

  #[test]
  fn python_scope_aware_rename_targets_body_only() {
    let src = "\
def target():
    foo = 1
    return foo + 2

def other():
    foo = 99
    return foo + 3
";
    let edits =
      compute_file_rename_edits_python_scope_aware("foo", "renamed", src, "target").expect("ok");
    assert_eq!(edits.len(), 2);
    let after = apply_rename_edits(src, &edits, "renamed");
    assert!(after.contains("    renamed = 1"));
    assert!(after.contains("    return renamed + 2"));
    // Other body untouched.
    assert!(after.contains("    foo = 99"));
    assert!(after.contains("    return foo + 3"));
  }

  #[test]
  fn python_scope_aware_rename_returns_target_not_found_when_def_absent() {
    let src = "def other():\n    pass\n";
    let r = compute_file_rename_edits_python_scope_aware("x", "y", src, "missing_fn");
    assert_eq!(r.unwrap_err(), RustScopeAwareError::TargetFunctionNotFound);
  }

  #[test]
  fn python_scope_aware_rename_returns_multiple_targets_when_duplicate_def_names() {
    let src = "\
class A:
    def dup(self):
        x = 1
class B:
    def dup(self):
        x = 2
";
    let r = compute_file_rename_edits_python_scope_aware("x", "y", src, "dup");
    assert_eq!(
      r.unwrap_err(),
      RustScopeAwareError::MultipleTargetFunctions { count: 2 }
    );
  }

  #[test]
  fn python_scope_aware_rename_class_method_body_isolated() {
    // A `def method(self):` inside `class A:` is detected as a
    // scope. Renaming inside it doesn't touch module-level code.
    let src = "\
class A:
    def method(self):
        foo = 1
        return foo

foo = \"module level\"
";
    let edits =
      compute_file_rename_edits_python_scope_aware("foo", "x", src, "method").expect("ok");
    // 2 occurrences inside method body (`foo = 1` + `return foo`).
    // Module-level `foo = \"module level\"` filtered both by string
    // skip-zone (inside `\"...\"`) AND by scope (outside method body).
    assert_eq!(edits.len(), 2);
  }

  // ─── TypeScript / JavaScript skip-zone tests ──────────────────────

  #[test]
  fn typescript_skip_zones_line_and_block_comments() {
    let src = "let x = foo; // foo here\nlet y = /* foo here */ foo;\n";
    let zones = typescript_skip_zones(src);
    assert_eq!(zones.iter().filter(|z| z.kind == "line-comment").count(), 1);
    assert_eq!(
      zones.iter().filter(|z| z.kind == "block-comment").count(),
      1
    );
  }

  #[test]
  fn typescript_skip_zones_single_and_double_quoted_strings() {
    let src = r#"let a = "foo"; let b = 'foo'; let c = foo;"#;
    let zones = typescript_skip_zones(src);
    let strs: Vec<_> = zones.iter().filter(|z| z.kind == "string").collect();
    assert_eq!(strs.len(), 2);
  }

  #[test]
  fn typescript_skip_zones_template_literal() {
    let src = "let s = `hello ${foo} world`; let t = foo;";
    let zones = typescript_skip_zones(src);
    let templates: Vec<_> = zones
      .iter()
      .filter(|z| z.kind == "template-literal")
      .collect();
    assert_eq!(templates.len(), 1);
    assert_eq!(
      &src[templates[0].start..templates[0].end],
      "`hello ${foo} world`"
    );
  }

  #[test]
  fn typescript_safe_edits_filter_all_zone_kinds() {
    let src =
      "// foo here\nconst foo = 1;\nconst s = \"foo\";\nconst t = `foo ${foo}`;\nconst g = foo;\n";
    let safe = compute_file_rename_edits_typescript_safe("foo", "renamed", src);
    // 5 token matches; only the bare `const foo = 1;` and `const g = foo;` survive.
    assert_eq!(safe.len(), 2);
  }

  #[test]
  fn javascript_safe_edits_use_same_lexer_as_typescript() {
    // JS uses the same lexer. Sanity check: JS-safe filters same way.
    let src = "// foo\nconst foo = 1;\nconst bar = foo;\n";
    let safe = compute_file_rename_edits_javascript_safe("foo", "renamed", src);
    assert_eq!(safe.len(), 2);
  }

  // ─── Go skip-zone tests ───────────────────────────────────────────

  #[test]
  fn go_skip_zones_line_and_block_comments() {
    let src = "x := foo // foo here\ny := /* foo here */ foo\n";
    let zones = go_skip_zones(src);
    assert_eq!(zones.iter().filter(|z| z.kind == "line-comment").count(), 1);
    assert_eq!(
      zones.iter().filter(|z| z.kind == "block-comment").count(),
      1
    );
  }

  #[test]
  fn go_skip_zones_interpreted_string_with_escapes() {
    let src = r#"x := "foo \"bar\""; y := foo"#;
    let zones = go_skip_zones(src);
    let strs: Vec<_> = zones.iter().filter(|z| z.kind == "string").collect();
    assert_eq!(strs.len(), 1);
    assert_eq!(&src[strs[0].start..strs[0].end], "\"foo \\\"bar\\\"\"");
  }

  #[test]
  fn go_skip_zones_raw_string_no_escapes() {
    // Raw string — backslashes are literal, no escape parsing.
    let src = "x := `foo \\n still foo`; y := foo";
    let zones = go_skip_zones(src);
    let raws: Vec<_> = zones.iter().filter(|z| z.kind == "raw-string").collect();
    assert_eq!(raws.len(), 1);
    assert!(src[raws[0].start..raws[0].end].contains("\\n"));
  }

  #[test]
  fn go_skip_zones_rune_literal() {
    let src = "r := 'f'; y := foo";
    let zones = go_skip_zones(src);
    assert_eq!(zones.iter().filter(|z| z.kind == "rune").count(), 1);
  }

  #[test]
  fn go_safe_edits_filter_all_zone_kinds() {
    let src = "// foo here\nfoo := 1\ns := \"foo\"\nt := `foo`\nr := 'f'\ng := foo\n";
    let safe = compute_file_rename_edits_go_safe("foo", "renamed", src);
    // 4 token matches; line 1 (comment), line 3 (string), line 4 (raw) filtered.
    // Surviving: line 2 (foo := 1) and line 6 (g := foo).
    assert_eq!(safe.len(), 2);
  }

  // ─── multi-language orchestrator ─────────────────────────────────

  #[test]
  fn lang_safe_orchestrator_dispatches_python() {
    let req = RenameRequest {
      old_name: "foo".to_string(),
      new_name: "bar".to_string(),
      target_paths: vec!["src/a.py".to_string()],
      language: "python".to_string(),
      scope: RenameScope::LocalTargetPaths,
      target_fn_name: None,
    };
    let f = RenameFileInput {
      path: "src/a.py",
      content: "foo = 1\n# foo wonderful\nbaz = \"foo\"\ny = foo\n",
    };
    let cand = compute_rename_patch_candidate_lang_safe(&req, &[f]);
    assert!(matches!(cand.verdict, RenameVerdict::RenameReady));
    // 2 edits (comment + string filtered).
    assert_eq!(cand.file_patches[0].edits.len(), 2);
  }

  #[test]
  fn lang_safe_orchestrator_dispatches_typescript() {
    let req = RenameRequest {
      old_name: "foo".to_string(),
      new_name: "bar".to_string(),
      target_paths: vec!["src/a.ts".to_string()],
      language: "typescript".to_string(),
      scope: RenameScope::LocalTargetPaths,
      target_fn_name: None,
    };
    let f = RenameFileInput {
      path: "src/a.ts",
      content: "// foo\nconst foo = 1;\nconst g = foo;\n",
    };
    let cand = compute_rename_patch_candidate_lang_safe(&req, &[f]);
    assert_eq!(cand.file_patches[0].edits.len(), 2);
  }

  #[test]
  fn lang_safe_orchestrator_dispatches_javascript() {
    let req = RenameRequest {
      old_name: "foo".to_string(),
      new_name: "bar".to_string(),
      target_paths: vec!["src/a.js".to_string()],
      language: "javascript".to_string(),
      scope: RenameScope::LocalTargetPaths,
      target_fn_name: None,
    };
    let f = RenameFileInput {
      path: "src/a.js",
      content: "// foo\nconst foo = 1;\nconst g = foo;\n",
    };
    let cand = compute_rename_patch_candidate_lang_safe(&req, &[f]);
    assert_eq!(cand.file_patches[0].edits.len(), 2);
  }

  #[test]
  fn lang_safe_orchestrator_dispatches_go() {
    let req = RenameRequest {
      old_name: "foo".to_string(),
      new_name: "bar".to_string(),
      target_paths: vec!["src/a.go".to_string()],
      language: "go".to_string(),
      scope: RenameScope::LocalTargetPaths,
      target_fn_name: None,
    };
    let f = RenameFileInput {
      path: "src/a.go",
      content: "// foo\nfoo := 1\ng := foo\n",
    };
    let cand = compute_rename_patch_candidate_lang_safe(&req, &[f]);
    assert_eq!(cand.file_patches[0].edits.len(), 2);
  }

  #[test]
  fn lang_safe_orchestrator_dispatches_rust() {
    let req = RenameRequest {
      old_name: "foo".to_string(),
      new_name: "bar".to_string(),
      target_paths: vec!["src/a.rs".to_string()],
      language: "rust".to_string(),
      scope: RenameScope::LocalTargetPaths,
      target_fn_name: None,
    };
    let f = RenameFileInput {
      path: "src/a.rs",
      content: "// foo\nlet foo = 1;\nlet g = foo;\n",
    };
    let cand = compute_rename_patch_candidate_lang_safe(&req, &[f]);
    assert_eq!(cand.file_patches[0].edits.len(), 2);
  }

  #[test]
  fn filter_rename_edits_outside_skip_zones_excludes_inside_zone() {
    let edits = vec![
      RenameEdit {
        byte_offset: 5,
        byte_len: 3,
        line: 1,
        column: 6,
      },
      RenameEdit {
        byte_offset: 50,
        byte_len: 3,
        line: 3,
        column: 1,
      },
    ];
    let zones = vec![SkipZone {
      start: 0,
      end: 20,
      kind: "string",
    }];
    let filtered = filter_rename_edits_outside_skip_zones(edits, &zones);
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].byte_offset, 50);
  }
}
