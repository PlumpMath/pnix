//! Add-node deterministic code-transform host carrier.
//!
//! OWNER-LAW (2026-05-13): second pnix-native code-transform.
//! Mirrors `stdlib/lib/gate/code-transform/add-node.px`.
//!
//! Appends a new computation node `{ name = ...; uses = ...; }` to
//! the `nodes = [...]` block of a target `.px` file. Pairs with
//! `add_extern_decl` (adds the primitive a node uses) and the
//! future `add_edge` (wires nodes together).
//!
//! Same verdict-ladder discipline as `add_extern_decl`:
//! deterministic precondition check, no file mutation. Host
//! materializer locates the `nodes = [` block and inserts before
//! `];` under `ToolActionApproval`.

use serde::{Deserialize, Serialize};
use pnix_hash::{Digest, Sha256};

/// How the host should react if a node with the same `name`
/// already exists in the target file's nodes block. Mirrors the
/// `.px` `if_already_present` field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AddNodeIfAlreadyPresent {
  Skip,
  Duplicate,
  Held,
}

impl AddNodeIfAlreadyPresent {
  pub const ALL: &'static [Self] = &[Self::Skip, Self::Duplicate, Self::Held];

  pub fn as_str(self) -> &'static str {
    match self {
      Self::Skip => "skip",
      Self::Duplicate => "duplicate",
      Self::Held => "held",
    }
  }
}

/// Held / Rejected outcome kinds from [`classify_add_node`].
/// Mirrors the `.px` `held_kind` ladder 1:1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AddNodeHeldKind {
  MissingTargetPath,
  TargetPathOutOfProject,
  TargetPathNotPnix,
  MissingNodeName,
  NodeNameMalformed,
  MissingNodeUses,
  NodeUsesMalformed,
  LanguageNotSupported,
  IfAlreadyPresentInvalid,
}

impl AddNodeHeldKind {
  pub const ALL: &'static [Self] = &[
    Self::MissingTargetPath,
    Self::TargetPathOutOfProject,
    Self::TargetPathNotPnix,
    Self::MissingNodeName,
    Self::NodeNameMalformed,
    Self::MissingNodeUses,
    Self::NodeUsesMalformed,
    Self::LanguageNotSupported,
    Self::IfAlreadyPresentInvalid,
  ];

  pub fn as_str(self) -> &'static str {
    match self {
      Self::MissingTargetPath => "missing-target-path",
      Self::TargetPathOutOfProject => "target-path-out-of-project",
      Self::TargetPathNotPnix => "target-path-not-pnix",
      Self::MissingNodeName => "missing-node-name",
      Self::NodeNameMalformed => "node-name-malformed",
      Self::MissingNodeUses => "missing-node-uses",
      Self::NodeUsesMalformed => "node-uses-malformed",
      Self::LanguageNotSupported => "language-not-supported",
      Self::IfAlreadyPresentInvalid => "if-already-present-invalid",
    }
  }
}

pub const SUPPORTED_LANGUAGES: &[&str] = &["pnix"];

/// An add-node request. Mirrors the `.px` input shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddNodeRequest {
  pub target_path: String,
  pub language: String,
  /// Node-local identifier (single segment, no dots).
  /// Corpus convention: `n1`, `n2`, `scene_node_1`, `merge_a_b`.
  pub node_name: String,
  /// Extern name the node dispatches to. Same shape as
  /// `add-extern-decl::extern_name` — `<prefix>.<ident>(.<ident>)*`.
  pub node_uses: String,
  pub if_already_present: Option<AddNodeIfAlreadyPresent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "verdict")]
pub enum AddNodeVerdict {
  AddNodeReady,
  AddNodeSkip,
  AddNodeHeld {
    held_kind: AddNodeHeldKind,
    reason: String,
  },
  AddNodeRejected {
    held_kind: AddNodeHeldKind,
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

fn is_ident_start(b: u8) -> bool {
  b.is_ascii_alphabetic() || b == b'_'
}

fn is_ident_cont(b: u8) -> bool {
  b.is_ascii_alphanumeric() || b == b'_'
}

/// Single-segment identifier (no dots): `[a-zA-Z_][a-zA-Z0-9_]*`.
fn node_name_looks_valid(name: &str) -> bool {
  if name.is_empty() {
    return false;
  }
  let bytes = name.as_bytes();
  if !is_ident_start(bytes[0]) {
    return false;
  }
  bytes.iter().all(|b| is_ident_cont(*b))
}

/// `<prefix>.<ident>(.<ident>)*` — same as `add_extern_decl::
/// extern_name_looks_valid`. Inlined here to keep the two carriers
/// independent (no cross-module coupling).
fn node_uses_looks_valid(uses: &str) -> bool {
  if uses.is_empty() {
    return false;
  }
  let bytes = uses.as_bytes();
  if !is_ident_start(bytes[0]) {
    return false;
  }
  let mut i = 0usize;
  while i < bytes.len() && is_ident_cont(bytes[i]) {
    i += 1;
  }
  if i == bytes.len() || bytes[i] != b'.' {
    return false;
  }
  while i < bytes.len() {
    if bytes[i] != b'.' {
      return false;
    }
    i += 1;
    if i >= bytes.len() || !is_ident_start(bytes[i]) {
      return false;
    }
    while i < bytes.len() && is_ident_cont(bytes[i]) {
      i += 1;
    }
  }
  true
}

/// Pure classifier — Rust mirror of the `.px` owner law's `classify`.
///
/// OWNER-LAW (2026-05-13): MUST stay in lockstep with
/// `stdlib/lib/gate/code-transform/add-node.px`. Parity test enforces
/// held_kind + supported-language + validIfAlreadyPresent alignment.
///
/// Ladder order matches the `.px`:
///   1. target_path empty → Held
///   2. target_path out-of-project → Held
///   3. target_path not `.px` → Held
///   4. node_name empty → Held
///   5. node_uses empty → Held
///   6. language not supported → Held
///   7. if_already_present invalid → **Rejected**
///   8. node_name malformed → Held
///   9. node_uses malformed → Held
///
/// Anything else → Ready.
pub fn classify_add_node(req: &AddNodeRequest) -> AddNodeVerdict {
  if req.target_path.is_empty() {
    return AddNodeVerdict::AddNodeHeld {
      held_kind: AddNodeHeldKind::MissingTargetPath,
      reason: "add-node requires a non-empty `target_path`".to_string(),
    };
  }
  if !is_path_in_project(&req.target_path) {
    return AddNodeVerdict::AddNodeHeld {
      held_kind: AddNodeHeldKind::TargetPathOutOfProject,
      reason: "target_path must be within the project root and must not contain `..`".to_string(),
    };
  }
  if !ends_with_px(&req.target_path) {
    return AddNodeVerdict::AddNodeHeld {
      held_kind: AddNodeHeldKind::TargetPathNotPnix,
      reason: "add-node is pnix-native; target_path must end in `.px`".to_string(),
    };
  }
  if req.node_name.is_empty() {
    return AddNodeVerdict::AddNodeHeld {
      held_kind: AddNodeHeldKind::MissingNodeName,
      reason: "add-node requires a non-empty `node_name`".to_string(),
    };
  }
  if req.node_uses.is_empty() {
    return AddNodeVerdict::AddNodeHeld {
      held_kind: AddNodeHeldKind::MissingNodeUses,
      reason: "add-node requires a non-empty `node_uses` (extern reference)".to_string(),
    };
  }
  if !is_supported_language(&req.language) {
    return AddNodeVerdict::AddNodeHeld {
      held_kind: AddNodeHeldKind::LanguageNotSupported,
      reason: format!(
        "add-node owner currently supports `pnix` only; got `{}`",
        req.language
      ),
    };
  }
  let if_present = req
    .if_already_present
    .unwrap_or(AddNodeIfAlreadyPresent::Skip);
  if !matches!(
    if_present,
    AddNodeIfAlreadyPresent::Skip
      | AddNodeIfAlreadyPresent::Duplicate
      | AddNodeIfAlreadyPresent::Held
  ) {
    return AddNodeVerdict::AddNodeRejected {
      held_kind: AddNodeHeldKind::IfAlreadyPresentInvalid,
      reason: "if_already_present must be one of skip|duplicate|held".to_string(),
    };
  }
  if !node_name_looks_valid(&req.node_name) {
    return AddNodeVerdict::AddNodeHeld {
      held_kind: AddNodeHeldKind::NodeNameMalformed,
      reason: format!(
        "node_name must match `[a-zA-Z_][a-zA-Z0-9_]*` (single segment); got `{}`",
        req.node_name
      ),
    };
  }
  if !node_uses_looks_valid(&req.node_uses) {
    return AddNodeVerdict::AddNodeHeld {
      held_kind: AddNodeHeldKind::NodeUsesMalformed,
      reason: format!(
        "node_uses must match `<prefix>.<ident>(.<ident>)*`; got `{}`",
        req.node_uses
      ),
    };
  }
  AddNodeVerdict::AddNodeReady
}

/// Deterministic fingerprint for the host materializer's TOCTOU
/// defense. Same shape as `add_extern_decl::fingerprint`.
pub fn fingerprint_add_node(req: &AddNodeRequest) -> String {
  let mut hasher = Sha256::new();
  hasher.update(b"add-node:");
  hasher.update(req.target_path.as_bytes());
  hasher.update(b"|");
  hasher.update(req.language.as_bytes());
  hasher.update(b"|");
  hasher.update(req.node_name.as_bytes());
  hasher.update(b"|");
  hasher.update(req.node_uses.as_bytes());
  hasher.update(b"|");
  hasher.update(
    req
      .if_already_present
      .unwrap_or(AddNodeIfAlreadyPresent::Skip)
      .as_str()
      .as_bytes(),
  );
  format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
  use super::*;

  fn req(target: &str, name: &str, uses: &str) -> AddNodeRequest {
    AddNodeRequest {
      target_path: target.to_string(),
      language: "pnix".to_string(),
      node_name: name.to_string(),
      node_uses: uses.to_string(),
      if_already_present: None,
    }
  }

  // ─── ready path ─────────────────────────────────────────────────

  #[test]
  fn ready_on_canonical_corpus_node() {
    let v = classify_add_node(&req(
      "examples/pnix_algo/completed/working/W1-arithmetic.px",
      "n1",
      "py.add",
    ));
    assert!(matches!(v, AddNodeVerdict::AddNodeReady));
  }

  #[test]
  fn ready_on_underscore_node_name() {
    let v = classify_add_node(&req("x.px", "scene_node_1", "freecat.upsertNode"));
    assert!(matches!(v, AddNodeVerdict::AddNodeReady));
  }

  #[test]
  fn ready_on_multi_dotted_uses() {
    let v = classify_add_node(&req("x.px", "n1", "world.edit.upsertPoint"));
    assert!(matches!(v, AddNodeVerdict::AddNodeReady));
  }

  // ─── held ladder ───────────────────────────────────────────────

  #[test]
  fn held_on_missing_target_path() {
    let v = classify_add_node(&req("", "n1", "py.add"));
    assert!(matches!(
      v,
      AddNodeVerdict::AddNodeHeld {
        held_kind: AddNodeHeldKind::MissingTargetPath,
        ..
      }
    ));
  }

  #[test]
  fn held_on_path_out_of_project() {
    let v = classify_add_node(&req("../escape.px", "n1", "py.add"));
    assert!(matches!(
      v,
      AddNodeVerdict::AddNodeHeld {
        held_kind: AddNodeHeldKind::TargetPathOutOfProject,
        ..
      }
    ));
  }

  #[test]
  fn held_on_non_px_target() {
    let v = classify_add_node(&req("src/main.rs", "n1", "py.add"));
    assert!(matches!(
      v,
      AddNodeVerdict::AddNodeHeld {
        held_kind: AddNodeHeldKind::TargetPathNotPnix,
        ..
      }
    ));
  }

  #[test]
  fn held_on_missing_node_name() {
    let v = classify_add_node(&req("x.px", "", "py.add"));
    assert!(matches!(
      v,
      AddNodeVerdict::AddNodeHeld {
        held_kind: AddNodeHeldKind::MissingNodeName,
        ..
      }
    ));
  }

  #[test]
  fn held_on_missing_node_uses() {
    let v = classify_add_node(&req("x.px", "n1", ""));
    assert!(matches!(
      v,
      AddNodeVerdict::AddNodeHeld {
        held_kind: AddNodeHeldKind::MissingNodeUses,
        ..
      }
    ));
  }

  #[test]
  fn held_on_unsupported_language() {
    let mut r = req("x.px", "n1", "py.add");
    r.language = "rust".to_string();
    let v = classify_add_node(&r);
    assert!(matches!(
      v,
      AddNodeVerdict::AddNodeHeld {
        held_kind: AddNodeHeldKind::LanguageNotSupported,
        ..
      }
    ));
  }

  #[test]
  fn held_on_node_name_with_dot() {
    // dots are not allowed in node names (those are for extern refs).
    let v = classify_add_node(&req("x.px", "scene.n1", "py.add"));
    assert!(matches!(
      v,
      AddNodeVerdict::AddNodeHeld {
        held_kind: AddNodeHeldKind::NodeNameMalformed,
        ..
      }
    ));
  }

  #[test]
  fn held_on_node_name_starts_with_digit() {
    let v = classify_add_node(&req("x.px", "1node", "py.add"));
    assert!(matches!(
      v,
      AddNodeVerdict::AddNodeHeld {
        held_kind: AddNodeHeldKind::NodeNameMalformed,
        ..
      }
    ));
  }

  #[test]
  fn held_on_node_uses_no_prefix() {
    let v = classify_add_node(&req("x.px", "n1", "add"));
    assert!(matches!(
      v,
      AddNodeVerdict::AddNodeHeld {
        held_kind: AddNodeHeldKind::NodeUsesMalformed,
        ..
      }
    ));
  }

  #[test]
  fn held_on_node_uses_trailing_dot() {
    let v = classify_add_node(&req("x.px", "n1", "py.add."));
    assert!(matches!(
      v,
      AddNodeVerdict::AddNodeHeld {
        held_kind: AddNodeHeldKind::NodeUsesMalformed,
        ..
      }
    ));
  }

  // ─── ladder order ──────────────────────────────────────────────

  #[test]
  fn ladder_path_check_before_name_check() {
    let v = classify_add_node(&req("", "", ""));
    assert!(matches!(
      v,
      AddNodeVerdict::AddNodeHeld {
        held_kind: AddNodeHeldKind::MissingTargetPath,
        ..
      }
    ));
  }

  #[test]
  fn ladder_node_name_before_node_uses() {
    let v = classify_add_node(&req("x.px", "", ""));
    assert!(matches!(
      v,
      AddNodeVerdict::AddNodeHeld {
        held_kind: AddNodeHeldKind::MissingNodeName,
        ..
      }
    ));
  }

  // ─── pnix3d live-coding scenario ───────────────────────────────

  #[test]
  fn pnix3d_scene_node_addition() {
    // Operator says "scene 에 새 노드 추가해줘 — freecat.upsertNode 를 쓰는".
    let r = AddNodeRequest {
      target_path: "world/scene.px".to_string(),
      language: "pnix".to_string(),
      node_name: "spawn_player".to_string(),
      node_uses: "freecat.upsertNode".to_string(),
      if_already_present: None,
    };
    assert!(matches!(
      classify_add_node(&r),
      AddNodeVerdict::AddNodeReady
    ));
  }

  // ─── fingerprint determinism ───────────────────────────────────

  #[test]
  fn fingerprint_is_deterministic() {
    let r = req("x.px", "n1", "py.add");
    assert_eq!(fingerprint_add_node(&r), fingerprint_add_node(&r));
  }

  #[test]
  fn fingerprint_differs_per_node_name() {
    let a = req("x.px", "n1", "py.add");
    let b = req("x.px", "n2", "py.add");
    assert_ne!(fingerprint_add_node(&a), fingerprint_add_node(&b));
  }

  #[test]
  fn fingerprint_differs_per_node_uses() {
    let a = req("x.px", "n1", "py.add");
    let b = req("x.px", "n1", "py.mul");
    assert_ne!(fingerprint_add_node(&a), fingerprint_add_node(&b));
  }

  // ─── pure validator tests ──────────────────────────────────────

  #[test]
  fn node_name_validator_accepts_single_segment() {
    assert!(node_name_looks_valid("n1"));
    assert!(node_name_looks_valid("scene_node_1"));
    assert!(node_name_looks_valid("_private"));
    assert!(node_name_looks_valid("merge_a_b"));
  }

  #[test]
  fn node_name_validator_rejects_dots() {
    // Dots are reserved for extern references (uses); node names
    // must be single-segment.
    assert!(!node_name_looks_valid("a.b"));
    assert!(!node_name_looks_valid("scene.n1"));
    assert!(!node_name_looks_valid(""));
    assert!(!node_name_looks_valid("1start"));
    assert!(!node_name_looks_valid("with space"));
  }

  #[test]
  fn node_uses_validator_accepts_extern_shapes() {
    assert!(node_uses_looks_valid("py.add"));
    assert!(node_uses_looks_valid("builtins.eq"));
    assert!(node_uses_looks_valid("world.edit.upsertPoint"));
  }

  #[test]
  fn node_uses_validator_rejects_invalid() {
    assert!(!node_uses_looks_valid(""));
    assert!(!node_uses_looks_valid("add")); // no prefix
    assert!(!node_uses_looks_valid("py.add."));
    assert!(!node_uses_looks_valid(".py.add"));
    assert!(!node_uses_looks_valid("py..add"));
  }
}
