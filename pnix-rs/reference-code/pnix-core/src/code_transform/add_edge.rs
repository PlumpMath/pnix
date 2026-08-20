//! Add-edge deterministic code-transform host carrier.
//!
//! OWNER-LAW (2026-05-13): third pnix-native code-transform.
//! Mirrors `stdlib/lib/gate/code-transform/add-edge.px` byte-equal.
//!
//! Appends a new edge `{ from = <endpoint>; to = <endpoint>; }` to
//! the `edges = [...]` block of a target `.px` file. Pairs with
//! `add-extern-decl` and `add-node` to complete the graph-edit
//! primitive trio.
//!
//! Like `add-extern-decl` / `add-node`, this carrier owns the
//! **verdict ladder + canonical artifact builders only** — the
//! host materializer performs the actual insertion at the
//! canonical location (before the `edges = [` block's closing
//! `];`) under `ToolActionApproval`.

use serde::{Deserialize, Serialize};
use pnix_hash::{Digest, Sha256};

/// How the host should react when an edge with the same
/// `(from, to)` already exists. Mirrors `.px` `if_already_present`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AddEdgeIfAlreadyPresent {
  Skip,
  Duplicate,
  Held,
}

impl AddEdgeIfAlreadyPresent {
  pub const ALL: &'static [Self] = &[Self::Skip, Self::Duplicate, Self::Held];

  pub fn as_str(self) -> &'static str {
    match self {
      Self::Skip => "skip",
      Self::Duplicate => "duplicate",
      Self::Held => "held",
    }
  }
}

/// Held / Rejected outcome kinds from [`classify_add_edge`].
/// Mirrors the `.px` `held_kind` ladder 1:1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AddEdgeHeldKind {
  MissingTargetPath,
  TargetPathOutOfProject,
  TargetPathNotPnix,
  MissingEdgeFrom,
  MissingEdgeTo,
  EdgeFromMalformed,
  EdgeToMalformed,
  LanguageNotSupported,
  IfAlreadyPresentInvalid,
}

impl AddEdgeHeldKind {
  pub const ALL: &'static [Self] = &[
    Self::MissingTargetPath,
    Self::TargetPathOutOfProject,
    Self::TargetPathNotPnix,
    Self::MissingEdgeFrom,
    Self::MissingEdgeTo,
    Self::EdgeFromMalformed,
    Self::EdgeToMalformed,
    Self::LanguageNotSupported,
    Self::IfAlreadyPresentInvalid,
  ];

  pub fn as_str(self) -> &'static str {
    match self {
      Self::MissingTargetPath => "missing-target-path",
      Self::TargetPathOutOfProject => "target-path-out-of-project",
      Self::TargetPathNotPnix => "target-path-not-pnix",
      Self::MissingEdgeFrom => "missing-edge-from",
      Self::MissingEdgeTo => "missing-edge-to",
      Self::EdgeFromMalformed => "edge-from-malformed",
      Self::EdgeToMalformed => "edge-to-malformed",
      Self::LanguageNotSupported => "language-not-supported",
      Self::IfAlreadyPresentInvalid => "if-already-present-invalid",
    }
  }
}

/// Edge endpoint — `.px` attrset shape with key as discriminator.
/// Two corpus forms:
///   * `{ input = "<ident>"; }`              — graph input reference
///   * `{ node = "<ident>"; port? = "<id>"; }` — node port reference
///
/// `#[serde(untagged)]` matches the `.px` attrset shape byte-equal —
/// no `kind` tag added on the wire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AddEdgeEndpoint {
  Input {
    input: String,
  },
  Node {
    node: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    port: Option<String>,
  },
}

pub const SUPPORTED_LANGUAGES: &[&str] = &["pnix"];

/// An add-edge request. Mirrors the `.px` input shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddEdgeRequest {
  pub target_path: String,
  pub language: String,
  pub edge_from: AddEdgeEndpoint,
  pub edge_to: AddEdgeEndpoint,
  pub if_already_present: Option<AddEdgeIfAlreadyPresent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "verdict")]
pub enum AddEdgeVerdict {
  AddEdgeReady,
  AddEdgeSkip,
  AddEdgeHeld {
    held_kind: AddEdgeHeldKind,
    reason: String,
  },
  AddEdgeRejected {
    held_kind: AddEdgeHeldKind,
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

/// `[a-zA-Z_][a-zA-Z0-9_]*` — same shape as add-node's
/// `nodeNameLooksValid`. Used for input name, node name, port name.
fn ident_looks_valid(name: &str) -> bool {
  if name.is_empty() {
    return false;
  }
  let bytes = name.as_bytes();
  if !is_ident_start(bytes[0]) {
    return false;
  }
  bytes[1..].iter().all(|&b| is_ident_cont(b))
}

/// `.px` `endpointLooksValid` mirror — typed enum validation.
fn endpoint_looks_valid(endpoint: &AddEdgeEndpoint) -> bool {
  match endpoint {
    AddEdgeEndpoint::Input { input } => ident_looks_valid(input),
    AddEdgeEndpoint::Node { node, port } => {
      if !ident_looks_valid(node) {
        return false;
      }
      match port {
        Some(p) => ident_looks_valid(p),
        None => true,
      }
    }
  }
}

/// Pure classifier — Rust mirror of the `.px` owner law's `classify`.
///
/// OWNER-LAW (2026-05-13): MUST stay in lockstep with
/// `stdlib/lib/gate/code-transform/add-edge.px`. Parity test guard at
/// `crates/pnix-core/tests/code_transform_owner_carrier_sync.rs`
/// enforces held_kind + supported-language parity.
///
/// Ladder order matches `.px`:
///   1. target_path empty → Held(missing-target-path)
///   2. target_path out-of-project → Held(target-path-out-of-project)
///   3. target_path not `.px` → Held(target-path-not-pnix)
///   4. edge_from missing → Held(missing-edge-from)
///   5. edge_to missing → Held(missing-edge-to)
///   6. language not supported → Held(language-not-supported)
///   7. if_already_present invalid → Rejected(if-already-present-invalid)
///   8. edge_from malformed → Held(edge-from-malformed)
///   9. edge_to malformed → Held(edge-to-malformed)
///   otherwise → Ready
///
/// `edge_from` / `edge_to` "missing" semantics: the typed enum
/// always carries a value, but the `.px` ladder treats an absent
/// attrset field as Held. In Rust, callers wrap the optional in a
/// host shim that adds a None case before deserialization; here
/// the type system already guarantees presence, so steps 4 and 5
/// are unreachable when deserialized from a well-formed JSON
/// containing both `edge_from` and `edge_to`. The struct fields are
/// non-optional intentionally — JSON without them fails to parse,
/// which is the carrier-side analog of the `.px` Held.
pub fn classify_add_edge(req: &AddEdgeRequest) -> AddEdgeVerdict {
  if req.target_path.is_empty() {
    return AddEdgeVerdict::AddEdgeHeld {
      held_kind: AddEdgeHeldKind::MissingTargetPath,
      reason: "add-edge requires a non-empty `target_path`".to_string(),
    };
  }
  if !is_path_in_project(&req.target_path) {
    return AddEdgeVerdict::AddEdgeHeld {
      held_kind: AddEdgeHeldKind::TargetPathOutOfProject,
      reason: "target_path must be within the project root and must not contain `..`".to_string(),
    };
  }
  if !ends_with_px(&req.target_path) {
    return AddEdgeVerdict::AddEdgeHeld {
      held_kind: AddEdgeHeldKind::TargetPathNotPnix,
      reason: "add-edge is pnix-native; target_path must end in `.px`".to_string(),
    };
  }
  if !is_supported_language(&req.language) {
    return AddEdgeVerdict::AddEdgeHeld {
      held_kind: AddEdgeHeldKind::LanguageNotSupported,
      reason: format!(
        "add-edge owner currently supports `pnix` only; got `{}`",
        req.language
      ),
    };
  }
  let if_present = req
    .if_already_present
    .unwrap_or(AddEdgeIfAlreadyPresent::Skip);
  if !matches!(
    if_present,
    AddEdgeIfAlreadyPresent::Skip
      | AddEdgeIfAlreadyPresent::Duplicate
      | AddEdgeIfAlreadyPresent::Held
  ) {
    return AddEdgeVerdict::AddEdgeRejected {
      held_kind: AddEdgeHeldKind::IfAlreadyPresentInvalid,
      reason: "if_already_present must be one of skip|duplicate|held".to_string(),
    };
  }
  if !endpoint_looks_valid(&req.edge_from) {
    return AddEdgeVerdict::AddEdgeHeld {
      held_kind: AddEdgeHeldKind::EdgeFromMalformed,
      reason: "edge_from must be `{ input = \"<ident>\"; }` or `{ node = \"<ident>\"; port? = \"<ident>\"; }`".to_string(),
    };
  }
  if !endpoint_looks_valid(&req.edge_to) {
    return AddEdgeVerdict::AddEdgeHeld {
      held_kind: AddEdgeHeldKind::EdgeToMalformed,
      reason: "edge_to must be `{ input = \"<ident>\"; }` or `{ node = \"<ident>\"; port? = \"<ident>\"; }`".to_string(),
    };
  }
  AddEdgeVerdict::AddEdgeReady
}

/// Deterministic fingerprint of an add-edge request — host's TOCTOU
/// defense. Mirrors the hash shape of `add-extern-decl` / `add-node`.
pub fn fingerprint_add_edge(req: &AddEdgeRequest) -> String {
  let mut hasher = Sha256::new();
  hasher.update(b"add-edge:");
  hasher.update(req.target_path.as_bytes());
  hasher.update(b"|");
  hasher.update(req.language.as_bytes());
  hasher.update(b"|from:");
  fingerprint_endpoint(&mut hasher, &req.edge_from);
  hasher.update(b"|to:");
  fingerprint_endpoint(&mut hasher, &req.edge_to);
  hasher.update(b"|");
  hasher.update(
    req
      .if_already_present
      .unwrap_or(AddEdgeIfAlreadyPresent::Skip)
      .as_str()
      .as_bytes(),
  );
  format!("{:x}", hasher.finalize())
}

fn fingerprint_endpoint(hasher: &mut Sha256, endpoint: &AddEdgeEndpoint) {
  match endpoint {
    AddEdgeEndpoint::Input { input } => {
      hasher.update(b"input=");
      hasher.update(input.as_bytes());
    }
    AddEdgeEndpoint::Node { node, port } => {
      hasher.update(b"node=");
      hasher.update(node.as_bytes());
      if let Some(p) = port {
        hasher.update(b",port=");
        hasher.update(p.as_bytes());
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn req_input_to_node(target: &str, in_name: &str, node: &str, port: &str) -> AddEdgeRequest {
    AddEdgeRequest {
      target_path: target.to_string(),
      language: "pnix".to_string(),
      edge_from: AddEdgeEndpoint::Input {
        input: in_name.to_string(),
      },
      edge_to: AddEdgeEndpoint::Node {
        node: node.to_string(),
        port: Some(port.to_string()),
      },
      if_already_present: None,
    }
  }

  fn req_node_to_node(
    target: &str,
    from: &str,
    from_port: &str,
    to: &str,
    to_port: &str,
  ) -> AddEdgeRequest {
    AddEdgeRequest {
      target_path: target.to_string(),
      language: "pnix".to_string(),
      edge_from: AddEdgeEndpoint::Node {
        node: from.to_string(),
        port: Some(from_port.to_string()),
      },
      edge_to: AddEdgeEndpoint::Node {
        node: to.to_string(),
        port: Some(to_port.to_string()),
      },
      if_already_present: None,
    }
  }

  // ─── ready path ───────────────────────────────────────────────

  #[test]
  fn ready_on_canonical_input_to_node() {
    let v = classify_add_edge(&req_input_to_node("examples/g.px", "a", "sum", "lhs"));
    assert!(matches!(v, AddEdgeVerdict::AddEdgeReady));
  }

  #[test]
  fn ready_on_node_to_node_with_ports() {
    let v = classify_add_edge(&req_node_to_node("g.px", "sum", "out", "result", "lhs"));
    assert!(matches!(v, AddEdgeVerdict::AddEdgeReady));
  }

  #[test]
  fn ready_on_node_endpoint_without_port() {
    let r = AddEdgeRequest {
      target_path: "g.px".to_string(),
      language: "pnix".to_string(),
      edge_from: AddEdgeEndpoint::Input {
        input: "a".to_string(),
      },
      edge_to: AddEdgeEndpoint::Node {
        node: "result".to_string(),
        port: None,
      },
      if_already_present: None,
    };
    assert!(matches!(
      classify_add_edge(&r),
      AddEdgeVerdict::AddEdgeReady
    ));
  }

  // ─── held ladder ───────────────────────────────────────────────

  #[test]
  fn held_on_missing_target_path() {
    let v = classify_add_edge(&req_input_to_node("", "a", "sum", "lhs"));
    assert!(matches!(
      v,
      AddEdgeVerdict::AddEdgeHeld {
        held_kind: AddEdgeHeldKind::MissingTargetPath,
        ..
      }
    ));
  }

  #[test]
  fn held_on_parent_traversal_path() {
    let v = classify_add_edge(&req_input_to_node("../escape.px", "a", "sum", "lhs"));
    assert!(matches!(
      v,
      AddEdgeVerdict::AddEdgeHeld {
        held_kind: AddEdgeHeldKind::TargetPathOutOfProject,
        ..
      }
    ));
  }

  #[test]
  fn held_on_non_px_target() {
    let v = classify_add_edge(&req_input_to_node("src/main.rs", "a", "sum", "lhs"));
    assert!(matches!(
      v,
      AddEdgeVerdict::AddEdgeHeld {
        held_kind: AddEdgeHeldKind::TargetPathNotPnix,
        ..
      }
    ));
  }

  #[test]
  fn held_on_unsupported_language() {
    let mut r = req_input_to_node("g.px", "a", "sum", "lhs");
    r.language = "rust".to_string();
    assert!(matches!(
      classify_add_edge(&r),
      AddEdgeVerdict::AddEdgeHeld {
        held_kind: AddEdgeHeldKind::LanguageNotSupported,
        ..
      }
    ));
  }

  #[test]
  fn held_on_malformed_input_endpoint() {
    let r = AddEdgeRequest {
      target_path: "g.px".to_string(),
      language: "pnix".to_string(),
      edge_from: AddEdgeEndpoint::Input {
        input: "1invalid".to_string(),
      },
      edge_to: AddEdgeEndpoint::Node {
        node: "x".to_string(),
        port: None,
      },
      if_already_present: None,
    };
    assert!(matches!(
      classify_add_edge(&r),
      AddEdgeVerdict::AddEdgeHeld {
        held_kind: AddEdgeHeldKind::EdgeFromMalformed,
        ..
      }
    ));
  }

  #[test]
  fn held_on_malformed_node_port() {
    let r = AddEdgeRequest {
      target_path: "g.px".to_string(),
      language: "pnix".to_string(),
      edge_from: AddEdgeEndpoint::Input {
        input: "a".to_string(),
      },
      edge_to: AddEdgeEndpoint::Node {
        node: "x".to_string(),
        port: Some("1bad".to_string()),
      },
      if_already_present: None,
    };
    assert!(matches!(
      classify_add_edge(&r),
      AddEdgeVerdict::AddEdgeHeld {
        held_kind: AddEdgeHeldKind::EdgeToMalformed,
        ..
      }
    ));
  }

  #[test]
  fn held_on_empty_endpoint_node_name() {
    let r = AddEdgeRequest {
      target_path: "g.px".to_string(),
      language: "pnix".to_string(),
      edge_from: AddEdgeEndpoint::Input {
        input: "a".to_string(),
      },
      edge_to: AddEdgeEndpoint::Node {
        node: "".to_string(),
        port: None,
      },
      if_already_present: None,
    };
    assert!(matches!(
      classify_add_edge(&r),
      AddEdgeVerdict::AddEdgeHeld {
        held_kind: AddEdgeHeldKind::EdgeToMalformed,
        ..
      }
    ));
  }

  // ─── ladder order — first-violation-wins ──────────────────────

  #[test]
  fn ladder_target_path_before_endpoint() {
    // Both target_path violation AND bad endpoint: target_path wins.
    let r = AddEdgeRequest {
      target_path: "".to_string(),
      language: "pnix".to_string(),
      edge_from: AddEdgeEndpoint::Input {
        input: "1bad".to_string(),
      },
      edge_to: AddEdgeEndpoint::Node {
        node: "x".to_string(),
        port: None,
      },
      if_already_present: None,
    };
    assert!(matches!(
      classify_add_edge(&r),
      AddEdgeVerdict::AddEdgeHeld {
        held_kind: AddEdgeHeldKind::MissingTargetPath,
        ..
      }
    ));
  }

  // ─── fingerprint determinism ─────────────────────────────────

  #[test]
  fn fingerprint_is_deterministic() {
    let r = req_input_to_node("g.px", "a", "sum", "lhs");
    assert_eq!(fingerprint_add_edge(&r), fingerprint_add_edge(&r));
  }

  #[test]
  fn fingerprint_differs_per_endpoint() {
    let a = req_input_to_node("g.px", "a", "sum", "lhs");
    let b = req_input_to_node("g.px", "b", "sum", "lhs");
    assert_ne!(fingerprint_add_edge(&a), fingerprint_add_edge(&b));
  }

  #[test]
  fn fingerprint_input_vs_node_endpoint_differs() {
    let input_form = req_input_to_node("g.px", "a", "sum", "lhs");
    let node_form = req_node_to_node("g.px", "a", "out", "sum", "lhs");
    assert_ne!(
      fingerprint_add_edge(&input_form),
      fingerprint_add_edge(&node_form)
    );
  }

  // ─── serde round-trip — `.px` attrset shape compat ────────────

  #[test]
  fn endpoint_input_serializes_without_kind_tag() {
    let ep = AddEdgeEndpoint::Input {
      input: "a".to_string(),
    };
    let json = serde_json::to_string(&ep).expect("ser");
    // Must match the `.px` attrset shape: `{"input": "a"}`. No
    // `kind` discriminator field (untagged).
    assert_eq!(json, r#"{"input":"a"}"#);
  }

  #[test]
  fn endpoint_node_with_port_serializes_canonical_shape() {
    let ep = AddEdgeEndpoint::Node {
      node: "x".to_string(),
      port: Some("p".to_string()),
    };
    let json = serde_json::to_string(&ep).expect("ser");
    assert_eq!(json, r#"{"node":"x","port":"p"}"#);
  }

  #[test]
  fn endpoint_node_without_port_omits_field() {
    let ep = AddEdgeEndpoint::Node {
      node: "x".to_string(),
      port: None,
    };
    let json = serde_json::to_string(&ep).expect("ser");
    assert_eq!(json, r#"{"node":"x"}"#);
  }

  #[test]
  fn endpoint_deserializes_from_input_form() {
    let ep: AddEdgeEndpoint = serde_json::from_str(r#"{"input":"current"}"#).expect("de");
    assert!(matches!(ep, AddEdgeEndpoint::Input { ref input } if input == "current"));
  }

  #[test]
  fn endpoint_deserializes_from_node_with_port() {
    let ep: AddEdgeEndpoint = serde_json::from_str(r#"{"node":"x","port":"lhs"}"#).expect("de");
    match ep {
      AddEdgeEndpoint::Node { node, port } => {
        assert_eq!(node, "x");
        assert_eq!(port.as_deref(), Some("lhs"));
      }
      other => panic!("expected Node, got {other:?}"),
    }
  }

  #[test]
  fn endpoint_deserializes_from_node_without_port() {
    let ep: AddEdgeEndpoint = serde_json::from_str(r#"{"node":"x"}"#).expect("de");
    match ep {
      AddEdgeEndpoint::Node { node, port } => {
        assert_eq!(node, "x");
        assert!(port.is_none());
      }
      other => panic!("expected Node without port, got {other:?}"),
    }
  }

  // ─── enum exhaustiveness ─────────────────────────────────────

  #[test]
  fn all_held_kinds_have_distinct_as_str() {
    let mut seen: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for k in AddEdgeHeldKind::ALL {
      assert!(seen.insert(k.as_str()), "duplicate as_str: {}", k.as_str());
    }
    assert_eq!(seen.len(), 9);
  }

  #[test]
  fn all_if_already_present_have_distinct_as_str() {
    let mut seen: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for k in AddEdgeIfAlreadyPresent::ALL {
      assert!(seen.insert(k.as_str()));
    }
    assert_eq!(seen.len(), 3);
  }
}
