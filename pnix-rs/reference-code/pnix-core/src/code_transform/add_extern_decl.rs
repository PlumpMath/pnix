//! Add-extern-decl deterministic code-transform host carrier.
//!
//! OWNER-LAW (2026-05-13): the first **pnix-native** code-transform.
//! Mirrors `stdlib/lib/gate/code-transform/add-extern-decl.px`.
//!
//! Appends a new extern declaration `{ name = ...; inputs = [...];
//! outputs = [...]; }` to the `externs = [...]` block of a target
//! `.px` file. The corpus inventory in
//! `project-wiki/maps/pnix-algo-corpus-shape-inventory.md` shows
//! every one of the 1414 algorithm corpus files has an `externs = [`
//! block — this is the minimum-anchor, maximum-frequency operation.
//!
//! Like `add-import`, this carrier owns the **verdict ladder +
//! canonical artifact builders only** — the host materializer
//! performs the actual insertion at the canonical location (before
//! the `externs = [` block's closing `];`) under
//! `ToolActionApproval`.

use serde::{Deserialize, Serialize};
use pnix_hash::{Digest, Sha256};

/// How the host should react when an extern with the same `name`
/// already exists in the target file's externs block. Mirrors the
/// `.px` `if_already_present` field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AddExternDeclIfAlreadyPresent {
  /// Default: no-op (`add-extern-decl-skip` verdict).
  Skip,
  /// Insert the extern again (intentional duplicate — operator
  /// will resolve via a follow-up `merge-extern-decl` transform).
  Duplicate,
  /// Block: `add-extern-decl-held` until operator decides.
  Held,
}

impl AddExternDeclIfAlreadyPresent {
  pub const ALL: &'static [Self] = &[Self::Skip, Self::Duplicate, Self::Held];

  pub fn as_str(self) -> &'static str {
    match self {
      Self::Skip => "skip",
      Self::Duplicate => "duplicate",
      Self::Held => "held",
    }
  }
}

/// Held / Rejected outcome kinds from [`classify_add_extern_decl`].
/// Mirrors the `.px` `held_kind` ladder 1:1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AddExternDeclHeldKind {
  MissingTargetPath,
  TargetPathOutOfProject,
  TargetPathNotPnix,
  MissingExternName,
  LanguageNotSupported,
  ExternNameMalformed,
  IfAlreadyPresentInvalid,
}

impl AddExternDeclHeldKind {
  pub const ALL: &'static [Self] = &[
    Self::MissingTargetPath,
    Self::TargetPathOutOfProject,
    Self::TargetPathNotPnix,
    Self::MissingExternName,
    Self::LanguageNotSupported,
    Self::ExternNameMalformed,
    Self::IfAlreadyPresentInvalid,
  ];

  pub fn as_str(self) -> &'static str {
    match self {
      Self::MissingTargetPath => "missing-target-path",
      Self::TargetPathOutOfProject => "target-path-out-of-project",
      Self::TargetPathNotPnix => "target-path-not-pnix",
      Self::MissingExternName => "missing-extern-name",
      Self::LanguageNotSupported => "language-not-supported",
      Self::ExternNameMalformed => "extern-name-malformed",
      Self::IfAlreadyPresentInvalid => "if-already-present-invalid",
    }
  }
}

/// First pnix-native transform — only `"pnix"` is supported by
/// design. The `.px` owner law's `supportedLanguages` mirrors this.
pub const SUPPORTED_LANGUAGES: &[&str] = &["pnix"];

/// One typed input/output port declaration for the new extern.
/// Mirrors the `{ name; ty; }` attrset shape used across the
/// algorithm corpus.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddExternDeclPort {
  pub name: String,
  pub ty: String,
}

/// An add-extern-decl request. Mirrors the `.px` input shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddExternDeclRequest {
  pub target_path: String,
  pub language: String,
  /// Extern name following `<prefix>.<ident>(.<ident>)*` shape.
  /// Corpus inventory shows two prefixes in current use:
  /// `builtins.*` (2606 occurrences) and `py.*` (1021).
  pub extern_name: String,
  /// Optional input ports. v0 carrier passes them through to the
  /// host materializer as-is; the materializer formats them into
  /// the canonical `[{name; ty;} ...]` attrset shape. Empty list
  /// is valid (nullary externs exist).
  #[serde(default)]
  pub extern_inputs: Vec<AddExternDeclPort>,
  /// Optional output ports. Same as inputs.
  #[serde(default)]
  pub extern_outputs: Vec<AddExternDeclPort>,
  /// Defaults to `Skip` when `None`.
  pub if_already_present: Option<AddExternDeclIfAlreadyPresent>,
}

/// Verdict from [`classify_add_extern_decl`]. Four outcomes mirror
/// the `.px` law — same shape as `add-import`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "verdict")]
pub enum AddExternDeclVerdict {
  AddExternDeclReady,
  AddExternDeclSkip,
  AddExternDeclHeld {
    held_kind: AddExternDeclHeldKind,
    reason: String,
  },
  AddExternDeclRejected {
    held_kind: AddExternDeclHeldKind,
    reason: String,
  },
}

fn is_supported_language(lang: &str) -> bool {
  matches!(lang, "pnix")
}

fn is_path_in_project(p: &str) -> bool {
  !p.is_empty() && !p.contains("..") && !p.contains('\u{0}')
}

fn ends_with_px(p: &str) -> bool {
  p.ends_with(".px")
}

/// Validate an extern name against `<prefix>.<ident>(.<ident>)*`.
/// Pure string scan — no regex. The `.px` mirror uses
/// `builtins.match` with the equivalent pattern.
fn extern_name_looks_valid(name: &str) -> bool {
  if name.is_empty() {
    return false;
  }
  let bytes = name.as_bytes();
  let mut i = 0usize;
  // First segment: identifier
  if !is_ident_start(bytes[0]) {
    return false;
  }
  while i < bytes.len() && is_ident_cont(bytes[i]) {
    i += 1;
  }
  // Must have at least one `.` after the first segment.
  if i == bytes.len() || bytes[i] != b'.' {
    return false;
  }
  // Subsequent segments: `. <ident>` repeated 1+ times.
  while i < bytes.len() {
    if bytes[i] != b'.' {
      return false;
    }
    i += 1;
    // Segment must start with identifier char.
    if i >= bytes.len() || !is_ident_start(bytes[i]) {
      return false;
    }
    while i < bytes.len() && is_ident_cont(bytes[i]) {
      i += 1;
    }
  }
  true
}

fn is_ident_start(b: u8) -> bool {
  b.is_ascii_alphabetic() || b == b'_'
}

fn is_ident_cont(b: u8) -> bool {
  b.is_ascii_alphanumeric() || b == b'_'
}

/// Pure classifier — Rust mirror of the `.px` owner law's `classify`.
///
/// OWNER-LAW (2026-05-13): MUST stay in lockstep with
/// `stdlib/lib/gate/code-transform/add-extern-decl.px`. The sync
/// guard at `crates/pnix-core/tests/code_transform_owner_carrier_sync.rs`
/// enforces held_kind + supported-language parity.
///
/// Ladder order matches the `.px`:
///   1. target_path empty → Held(missing-target-path)
///   2. target_path out-of-project → Held(target-path-out-of-project)
///   3. target_path not `.px` → Held(target-path-not-pnix)
///   4. extern_name empty → Held(missing-extern-name)
///   5. language not supported → Held(language-not-supported)
///   6. if_already_present invalid → **Rejected**(if-already-present-invalid)
///   7. extern_name malformed → Held(extern-name-malformed)
///
/// Anything else → Ready. The v0 carrier does NOT inspect the
/// target file's existing content; `add-extern-decl-skip` is
/// reserved for a future phase that scans for duplicates first.
pub fn classify_add_extern_decl(req: &AddExternDeclRequest) -> AddExternDeclVerdict {
  if req.target_path.is_empty() {
    return AddExternDeclVerdict::AddExternDeclHeld {
      held_kind: AddExternDeclHeldKind::MissingTargetPath,
      reason: "add-extern-decl requires a non-empty `target_path`".to_string(),
    };
  }
  if !is_path_in_project(&req.target_path) {
    return AddExternDeclVerdict::AddExternDeclHeld {
      held_kind: AddExternDeclHeldKind::TargetPathOutOfProject,
      reason: "target_path must be within the project root and must not contain `..`".to_string(),
    };
  }
  if !ends_with_px(&req.target_path) {
    return AddExternDeclVerdict::AddExternDeclHeld {
      held_kind: AddExternDeclHeldKind::TargetPathNotPnix,
      reason: "add-extern-decl is pnix-native; target_path must end in `.px`".to_string(),
    };
  }
  if req.extern_name.is_empty() {
    return AddExternDeclVerdict::AddExternDeclHeld {
      held_kind: AddExternDeclHeldKind::MissingExternName,
      reason: "add-extern-decl requires a non-empty `extern_name`".to_string(),
    };
  }
  if !is_supported_language(&req.language) {
    return AddExternDeclVerdict::AddExternDeclHeld {
      held_kind: AddExternDeclHeldKind::LanguageNotSupported,
      reason: format!(
        "add-extern-decl owner currently supports `pnix` only; got `{}`",
        req.language
      ),
    };
  }
  let if_present = req
    .if_already_present
    .unwrap_or(AddExternDeclIfAlreadyPresent::Skip);
  if !matches!(
    if_present,
    AddExternDeclIfAlreadyPresent::Skip
      | AddExternDeclIfAlreadyPresent::Duplicate
      | AddExternDeclIfAlreadyPresent::Held
  ) {
    // Defensive — enum coverage check is exhaustive at compile time
    // for the typed variant. This branch is for JSON-deserialized
    // requests where the deserializer might emit an out-of-range
    // tag (shouldn't happen with strict serde, but defensive).
    return AddExternDeclVerdict::AddExternDeclRejected {
      held_kind: AddExternDeclHeldKind::IfAlreadyPresentInvalid,
      reason: "if_already_present must be one of skip|duplicate|held".to_string(),
    };
  }
  if !extern_name_looks_valid(&req.extern_name) {
    return AddExternDeclVerdict::AddExternDeclHeld {
      held_kind: AddExternDeclHeldKind::ExternNameMalformed,
      reason: format!(
        "extern_name must match `<prefix>.<ident>(.<ident>)*`; got `{}`",
        req.extern_name
      ),
    };
  }
  AddExternDeclVerdict::AddExternDeclReady
}

/// Deterministic fingerprint of an add-extern-decl request — the
/// host materializer's TOCTOU defense. Mirrors the same hash
/// shape as `add-import`'s fingerprint (target + spec).
pub fn fingerprint_add_extern_decl(req: &AddExternDeclRequest) -> String {
  let mut hasher = Sha256::new();
  hasher.update(b"add-extern-decl:");
  hasher.update(req.target_path.as_bytes());
  hasher.update(b"|");
  hasher.update(req.language.as_bytes());
  hasher.update(b"|");
  hasher.update(req.extern_name.as_bytes());
  hasher.update(b"|inputs:");
  for p in &req.extern_inputs {
    hasher.update(p.name.as_bytes());
    hasher.update(b":");
    hasher.update(p.ty.as_bytes());
    hasher.update(b";");
  }
  hasher.update(b"|outputs:");
  for p in &req.extern_outputs {
    hasher.update(p.name.as_bytes());
    hasher.update(b":");
    hasher.update(p.ty.as_bytes());
    hasher.update(b";");
  }
  hasher.update(b"|");
  hasher.update(
    req
      .if_already_present
      .unwrap_or(AddExternDeclIfAlreadyPresent::Skip)
      .as_str()
      .as_bytes(),
  );
  format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
  use super::*;

  fn req(target: &str, name: &str) -> AddExternDeclRequest {
    AddExternDeclRequest {
      target_path: target.to_string(),
      language: "pnix".to_string(),
      extern_name: name.to_string(),
      extern_inputs: vec![],
      extern_outputs: vec![],
      if_already_present: None,
    }
  }

  // ─── ready path ─────────────────────────────────────────────────

  #[test]
  fn ready_on_canonical_corpus_extern() {
    let v = classify_add_extern_decl(&req(
      "examples/pnix_algo/completed/working/W1-arithmetic_chain.px",
      "py.add",
    ));
    assert!(matches!(v, AddExternDeclVerdict::AddExternDeclReady));
  }

  #[test]
  fn ready_on_builtins_prefix() {
    let v = classify_add_extern_decl(&req("stdlib/lib/example.px", "builtins.add"));
    assert!(matches!(v, AddExternDeclVerdict::AddExternDeclReady));
  }

  #[test]
  fn ready_on_freecat_pnix3d_extern() {
    let v = classify_add_extern_decl(&req("world/scene.px", "freecat.upsertNode"));
    assert!(matches!(v, AddExternDeclVerdict::AddExternDeclReady));
  }

  #[test]
  fn ready_on_multi_dotted_extern() {
    let v = classify_add_extern_decl(&req("scene.px", "world.edit.upsertPoint"));
    assert!(matches!(v, AddExternDeclVerdict::AddExternDeclReady));
  }

  #[test]
  fn ready_with_typed_input_output_ports() {
    let mut r = req("x.px", "py.mul");
    r.extern_inputs = vec![
      AddExternDeclPort {
        name: "a".into(),
        ty: "Num".into(),
      },
      AddExternDeclPort {
        name: "b".into(),
        ty: "Num".into(),
      },
    ];
    r.extern_outputs = vec![AddExternDeclPort {
      name: "out".into(),
      ty: "Num".into(),
    }];
    assert!(matches!(
      classify_add_extern_decl(&r),
      AddExternDeclVerdict::AddExternDeclReady
    ));
  }

  // ─── held ladder ───────────────────────────────────────────────

  #[test]
  fn held_on_missing_target_path() {
    let v = classify_add_extern_decl(&req("", "py.add"));
    assert!(matches!(
      v,
      AddExternDeclVerdict::AddExternDeclHeld {
        held_kind: AddExternDeclHeldKind::MissingTargetPath,
        ..
      }
    ));
  }

  #[test]
  fn held_on_path_out_of_project() {
    let v = classify_add_extern_decl(&req("../escape.px", "py.add"));
    assert!(matches!(
      v,
      AddExternDeclVerdict::AddExternDeclHeld {
        held_kind: AddExternDeclHeldKind::TargetPathOutOfProject,
        ..
      }
    ));
  }

  #[test]
  fn held_on_non_px_target() {
    let v = classify_add_extern_decl(&req("src/main.rs", "py.add"));
    assert!(matches!(
      v,
      AddExternDeclVerdict::AddExternDeclHeld {
        held_kind: AddExternDeclHeldKind::TargetPathNotPnix,
        ..
      }
    ));
  }

  #[test]
  fn held_on_missing_extern_name() {
    let v = classify_add_extern_decl(&req("x.px", ""));
    assert!(matches!(
      v,
      AddExternDeclVerdict::AddExternDeclHeld {
        held_kind: AddExternDeclHeldKind::MissingExternName,
        ..
      }
    ));
  }

  #[test]
  fn held_on_unsupported_language() {
    let mut r = req("x.px", "py.add");
    r.language = "rust".to_string();
    let v = classify_add_extern_decl(&r);
    assert!(matches!(
      v,
      AddExternDeclVerdict::AddExternDeclHeld {
        held_kind: AddExternDeclHeldKind::LanguageNotSupported,
        ..
      }
    ));
  }

  #[test]
  fn held_on_extern_name_no_prefix() {
    // No dot — not <prefix>.<ident>.
    let v = classify_add_extern_decl(&req("x.px", "add"));
    assert!(matches!(
      v,
      AddExternDeclVerdict::AddExternDeclHeld {
        held_kind: AddExternDeclHeldKind::ExternNameMalformed,
        ..
      }
    ));
  }

  #[test]
  fn held_on_extern_name_starts_with_digit() {
    let v = classify_add_extern_decl(&req("x.px", "1py.add"));
    assert!(matches!(
      v,
      AddExternDeclVerdict::AddExternDeclHeld {
        held_kind: AddExternDeclHeldKind::ExternNameMalformed,
        ..
      }
    ));
  }

  #[test]
  fn held_on_extern_name_empty_segment() {
    // `py..add` — empty middle segment.
    let v = classify_add_extern_decl(&req("x.px", "py..add"));
    assert!(matches!(
      v,
      AddExternDeclVerdict::AddExternDeclHeld {
        held_kind: AddExternDeclHeldKind::ExternNameMalformed,
        ..
      }
    ));
  }

  #[test]
  fn held_on_extern_name_trailing_dot() {
    let v = classify_add_extern_decl(&req("x.px", "py.add."));
    assert!(matches!(
      v,
      AddExternDeclVerdict::AddExternDeclHeld {
        held_kind: AddExternDeclHeldKind::ExternNameMalformed,
        ..
      }
    ));
  }

  // ─── ladder order — first-violation-wins ───────────────────────

  #[test]
  fn ladder_target_path_before_extern_name() {
    // Both missing: target_path violation wins.
    let v = classify_add_extern_decl(&req("", ""));
    assert!(matches!(
      v,
      AddExternDeclVerdict::AddExternDeclHeld {
        held_kind: AddExternDeclHeldKind::MissingTargetPath,
        ..
      }
    ));
  }

  #[test]
  fn ladder_px_check_before_extern_name() {
    // .rs path AND empty extern_name: path-not-pnix wins.
    let v = classify_add_extern_decl(&req("src/main.rs", ""));
    assert!(matches!(
      v,
      AddExternDeclVerdict::AddExternDeclHeld {
        held_kind: AddExternDeclHeldKind::TargetPathNotPnix,
        ..
      }
    ));
  }

  // ─── pnix3d live-coding scenario ───────────────────────────────

  #[test]
  fn pnix3d_world_scene_extern_addition() {
    let r = AddExternDeclRequest {
      target_path: "world/scene.px".to_string(),
      language: "pnix".to_string(),
      extern_name: "freecat.upsertNode".to_string(),
      extern_inputs: vec![
        AddExternDeclPort {
          name: "id".into(),
          ty: "String".into(),
        },
        AddExternDeclPort {
          name: "position".into(),
          ty: "Vec3".into(),
        },
      ],
      extern_outputs: vec![AddExternDeclPort {
        name: "node".into(),
        ty: "NodeRef".into(),
      }],
      if_already_present: None,
    };
    assert!(matches!(
      classify_add_extern_decl(&r),
      AddExternDeclVerdict::AddExternDeclReady
    ));
  }

  // ─── fingerprint determinism ───────────────────────────────────

  #[test]
  fn fingerprint_is_deterministic() {
    let r = req("x.px", "py.add");
    assert_eq!(
      fingerprint_add_extern_decl(&r),
      fingerprint_add_extern_decl(&r)
    );
  }

  #[test]
  fn fingerprint_differs_per_extern_name() {
    let a = req("x.px", "py.add");
    let b = req("x.px", "py.mul");
    assert_ne!(
      fingerprint_add_extern_decl(&a),
      fingerprint_add_extern_decl(&b)
    );
  }

  #[test]
  fn fingerprint_differs_per_target_path() {
    let a = req("a.px", "py.add");
    let b = req("b.px", "py.add");
    assert_ne!(
      fingerprint_add_extern_decl(&a),
      fingerprint_add_extern_decl(&b)
    );
  }

  #[test]
  fn fingerprint_includes_input_output_ports() {
    let plain = req("x.px", "py.add");
    let mut typed = plain.clone();
    typed.extern_inputs = vec![AddExternDeclPort {
      name: "a".into(),
      ty: "Num".into(),
    }];
    assert_ne!(
      fingerprint_add_extern_decl(&plain),
      fingerprint_add_extern_decl(&typed)
    );
  }

  // ─── extern name validator pure tests ──────────────────────────

  #[test]
  fn extern_name_validator_accepts_valid_shapes() {
    assert!(extern_name_looks_valid("py.add"));
    assert!(extern_name_looks_valid("builtins.add"));
    assert!(extern_name_looks_valid("world.edit.upsertPoint"));
    assert!(extern_name_looks_valid("_underscore.start"));
    assert!(extern_name_looks_valid("a.b.c.d.e"));
  }

  #[test]
  fn extern_name_validator_rejects_invalid_shapes() {
    assert!(!extern_name_looks_valid(""));
    assert!(!extern_name_looks_valid("add")); // no dot
    assert!(!extern_name_looks_valid("1py.add")); // starts with digit
    assert!(!extern_name_looks_valid("py..add")); // empty segment
    assert!(!extern_name_looks_valid("py.add.")); // trailing dot
    assert!(!extern_name_looks_valid(".py.add")); // leading dot
    assert!(!extern_name_looks_valid("py.1add")); // segment starts with digit
    assert!(!extern_name_looks_valid("py.add!")); // illegal char
  }
}
