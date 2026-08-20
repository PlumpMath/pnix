//! Move-symbol deterministic code-transform host carrier.
//!
//! OWNER-LAW (2026-05-12): mirror of
//! `stdlib/lib/gate/code-transform/move-symbol.px`. Verdict-ladder
//! carrier only; host CST emitter performs the actual move + import
//! rewrite at each reference path under `ToolActionApproval`
//! (`CodeEditCapability::EditWithinTargetPaths`).

use serde::{Deserialize, Serialize};
use pnix_hash::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MoveSymbolHeldKind {
  MissingPaths,
  SourceEqualsDestination,
  PathOutOfProject,
  LanguageNotSupported,
  InvalidSymbolName,
  ReferenceOutOfProject,
  PrivateSymbolCrossesCrate,
  PublicApiWithReferences,
}

impl MoveSymbolHeldKind {
  pub const ALL: &'static [Self] = &[
    Self::MissingPaths,
    Self::SourceEqualsDestination,
    Self::PathOutOfProject,
    Self::LanguageNotSupported,
    Self::InvalidSymbolName,
    Self::ReferenceOutOfProject,
    Self::PrivateSymbolCrossesCrate,
    Self::PublicApiWithReferences,
  ];
  pub fn as_str(self) -> &'static str {
    match self {
      Self::MissingPaths => "missing-paths",
      Self::SourceEqualsDestination => "source-equals-destination",
      Self::PathOutOfProject => "path-out-of-project",
      Self::LanguageNotSupported => "language-not-supported",
      Self::InvalidSymbolName => "invalid-symbol-name",
      Self::ReferenceOutOfProject => "reference-out-of-project",
      Self::PrivateSymbolCrossesCrate => "private-symbol-crosses-crate",
      Self::PublicApiWithReferences => "public-api-with-references",
    }
  }
}

pub const SUPPORTED_LANGUAGES: &[&str] = &["rust", "python", "typescript", "javascript", "go"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveSymbolRequest {
  pub source_path: String,
  pub destination_path: String,
  pub symbol_name: String,
  pub language: String,
  /// Host-supplied paths where the symbol is referenced. Used by the
  /// classifier to decide `public-api-with-references` and
  /// `reference-out-of-project`.
  #[serde(default)]
  pub reference_paths: Vec<String>,
  #[serde(default)]
  pub is_public_api: bool,
  #[serde(default)]
  pub crosses_crate_boundary: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "verdict")]
pub enum MoveSymbolVerdict {
  MoveSymbolReady,
  MoveSymbolHeld {
    held_kind: MoveSymbolHeldKind,
    reason: String,
  },
  MoveSymbolRejected {
    held_kind: MoveSymbolHeldKind,
    reason: String,
  },
}

fn is_supported_language(lang: &str) -> bool {
  matches!(lang, "rust" | "python" | "typescript" | "javascript" | "go")
}

fn is_path_in_project(p: &str) -> bool {
  !p.is_empty() && !p.contains("..") && !p.contains('\u{0}')
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

/// Mirror of `.px` `classify`. 8-step ladder.
pub fn classify_move_symbol(req: &MoveSymbolRequest) -> MoveSymbolVerdict {
  let missing_source = req.source_path.is_empty();
  let missing_dest = req.destination_path.is_empty();
  if missing_source || missing_dest {
    return MoveSymbolVerdict::MoveSymbolHeld {
      held_kind: MoveSymbolHeldKind::MissingPaths,
      reason: "source_path and destination_path both required".to_string(),
    };
  }
  if req.source_path == req.destination_path {
    return MoveSymbolVerdict::MoveSymbolRejected {
      held_kind: MoveSymbolHeldKind::SourceEqualsDestination,
      reason: "source_path == destination_path — nothing to move".to_string(),
    };
  }
  if !is_path_in_project(&req.source_path) || !is_path_in_project(&req.destination_path) {
    return MoveSymbolVerdict::MoveSymbolHeld {
      held_kind: MoveSymbolHeldKind::PathOutOfProject,
      reason: "source_path or destination_path is out of project root".to_string(),
    };
  }
  if !is_supported_language(&req.language) {
    return MoveSymbolVerdict::MoveSymbolHeld {
      held_kind: MoveSymbolHeldKind::LanguageNotSupported,
      reason: format!("language `{}` not supported", req.language),
    };
  }
  if !is_valid_identifier(&req.symbol_name) {
    return MoveSymbolVerdict::MoveSymbolRejected {
      held_kind: MoveSymbolHeldKind::InvalidSymbolName,
      reason: "symbol_name must be a valid identifier".to_string(),
    };
  }
  if req.reference_paths.iter().any(|p| !is_path_in_project(p)) {
    return MoveSymbolVerdict::MoveSymbolHeld {
      held_kind: MoveSymbolHeldKind::ReferenceOutOfProject,
      reason:
        "host found a reference path outside project root; cross-project moves are out of scope"
          .to_string(),
    };
  }
  if req.crosses_crate_boundary && !req.is_public_api {
    return MoveSymbolVerdict::MoveSymbolHeld {
      held_kind: MoveSymbolHeldKind::PrivateSymbolCrossesCrate,
      reason: "moving a private symbol across a crate boundary requires changing its visibility"
        .to_string(),
    };
  }
  if req.is_public_api && !req.reference_paths.is_empty() {
    return MoveSymbolVerdict::MoveSymbolHeld {
      held_kind: MoveSymbolHeldKind::PublicApiWithReferences,
      reason: format!(
        "moving a public symbol with {} references requires explicit owner approval (re-exports may break downstream callers)",
        req.reference_paths.len()
      ),
    };
  }
  MoveSymbolVerdict::MoveSymbolReady
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveSymbolCandidate {
  pub request: MoveSymbolRequest,
  pub verdict: MoveSymbolVerdict,
}

pub fn compute_move_symbol_candidate(request: &MoveSymbolRequest) -> MoveSymbolCandidate {
  let verdict = classify_move_symbol(request);
  MoveSymbolCandidate {
    request: request.clone(),
    verdict,
  }
}

pub fn build_move_symbol_candidate_payload(candidate: &MoveSymbolCandidate) -> serde_json::Value {
  let req = &candidate.request;
  let (verdict_str, next_step) = match &candidate.verdict {
    MoveSymbolVerdict::MoveSymbolReady => (
      "move-symbol-ready",
      "host-cst-move-and-rewrite-imports-then-tool-action-approval",
    ),
    MoveSymbolVerdict::MoveSymbolHeld { .. } => {
      ("move-symbol-held", "operator-decision-or-resubmit")
    }
    MoveSymbolVerdict::MoveSymbolRejected { .. } => {
      ("move-symbol-rejected", "operator-decision-or-resubmit")
    }
  };
  let mut payload = serde_json::json!({
    "transform": "move-symbol",
    "owner_law": "stdlib/lib/gate/code-transform/move-symbol.px",
    "source_path": req.source_path,
    "destination_path": req.destination_path,
    "symbol_name": req.symbol_name,
    "language": req.language,
    "reference_paths_count": req.reference_paths.len(),
    "is_public_api": req.is_public_api,
    "crosses_crate_boundary": req.crosses_crate_boundary,
    "verdict": verdict_str,
    "capability_required": "EditWithinTargetPaths",
    "candidate_only": true,
    "next_step": next_step,
  });
  match &candidate.verdict {
    MoveSymbolVerdict::MoveSymbolHeld { held_kind, reason }
    | MoveSymbolVerdict::MoveSymbolRejected { held_kind, reason } => {
      payload["held_kind"] = serde_json::Value::String(held_kind.as_str().to_string());
      payload["reason"] = serde_json::Value::String(reason.clone());
    }
    _ => {}
  }
  payload
}

pub fn build_move_symbol_candidate_artifact(
  candidate: &MoveSymbolCandidate,
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
) -> serde_json::Value {
  let payload = build_move_symbol_candidate_payload(candidate);
  let suffix = match &candidate.verdict {
    MoveSymbolVerdict::MoveSymbolReady => "ready",
    MoveSymbolVerdict::MoveSymbolHeld { .. } => "held",
    MoveSymbolVerdict::MoveSymbolRejected { .. } => "rejected",
  };
  let artifact_family = format!("coding.code-transform.move-symbol-{suffix}");
  let mut hasher = Sha256::new();
  hasher.update(b"move-symbol-candidate\x1f");
  hasher.update(candidate.request.source_path.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(candidate.request.destination_path.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(candidate.request.symbol_name.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(candidate.request.language.as_bytes());
  hasher.update(b"\x1f");
  hasher.update((candidate.request.reference_paths.len() as u64).to_le_bytes());
  hasher.update(&[
    candidate.request.is_public_api as u8,
    candidate.request.crosses_crate_boundary as u8,
  ]);
  hasher.update(b"\x1f");
  hasher.update(suffix.as_bytes());
  let digest = hasher.finalize();
  let prefix = digest
    .iter()
    .take(8)
    .map(|b| format!("{b:02x}"))
    .collect::<String>();
  let id = format!("move-symbol.{prefix}");
  // target_paths surfaces BOTH source and destination so doghouse's
  // `coding_memory_artifacts_by_path` index can find the move from
  // either side.
  let mut artifact = serde_json::json!({
    "id": id,
    "artifact_family": artifact_family,
    "source_surface": "code-transform.move-symbol",
    "stored_at_ms": stored_at_ms,
    "target_paths": [
      candidate.request.source_path.clone(),
      candidate.request.destination_path.clone(),
    ],
    "command_refs": serde_json::Value::Array(Vec::new()),
    "related_refs": serde_json::json!([
      "owner-law:stdlib/lib/gate/code-transform/move-symbol.px"
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

  fn req() -> MoveSymbolRequest {
    MoveSymbolRequest {
      source_path: "src/a.rs".to_string(),
      destination_path: "src/b.rs".to_string(),
      symbol_name: "MySymbol".to_string(),
      language: "rust".to_string(),
      reference_paths: vec![],
      is_public_api: false,
      crosses_crate_boundary: false,
    }
  }

  #[test]
  fn ready_for_well_formed() {
    assert!(matches!(
      classify_move_symbol(&req()),
      MoveSymbolVerdict::MoveSymbolReady
    ));
  }

  #[test]
  fn held_on_missing_paths() {
    let mut r = req();
    r.source_path = "".to_string();
    assert!(matches!(
      classify_move_symbol(&r),
      MoveSymbolVerdict::MoveSymbolHeld {
        held_kind: MoveSymbolHeldKind::MissingPaths,
        ..
      }
    ));
    let mut r = req();
    r.destination_path = "".to_string();
    assert!(matches!(
      classify_move_symbol(&r),
      MoveSymbolVerdict::MoveSymbolHeld {
        held_kind: MoveSymbolHeldKind::MissingPaths,
        ..
      }
    ));
  }

  #[test]
  fn rejected_on_source_equals_destination() {
    let mut r = req();
    r.destination_path = r.source_path.clone();
    assert!(matches!(
      classify_move_symbol(&r),
      MoveSymbolVerdict::MoveSymbolRejected {
        held_kind: MoveSymbolHeldKind::SourceEqualsDestination,
        ..
      }
    ));
  }

  #[test]
  fn held_on_path_out_of_project() {
    let mut r = req();
    r.destination_path = "../escape.rs".to_string();
    assert!(matches!(
      classify_move_symbol(&r),
      MoveSymbolVerdict::MoveSymbolHeld {
        held_kind: MoveSymbolHeldKind::PathOutOfProject,
        ..
      }
    ));
  }

  #[test]
  fn held_on_language_not_supported() {
    let mut r = req();
    r.language = "fortran".to_string();
    assert!(matches!(
      classify_move_symbol(&r),
      MoveSymbolVerdict::MoveSymbolHeld {
        held_kind: MoveSymbolHeldKind::LanguageNotSupported,
        ..
      }
    ));
  }

  #[test]
  fn rejected_on_invalid_symbol_name() {
    let mut r = req();
    r.symbol_name = "1bad".to_string();
    assert!(matches!(
      classify_move_symbol(&r),
      MoveSymbolVerdict::MoveSymbolRejected {
        held_kind: MoveSymbolHeldKind::InvalidSymbolName,
        ..
      }
    ));
  }

  #[test]
  fn held_on_reference_out_of_project() {
    let mut r = req();
    r.reference_paths = vec!["../outside.rs".to_string()];
    assert!(matches!(
      classify_move_symbol(&r),
      MoveSymbolVerdict::MoveSymbolHeld {
        held_kind: MoveSymbolHeldKind::ReferenceOutOfProject,
        ..
      }
    ));
  }

  #[test]
  fn held_on_private_symbol_crosses_crate() {
    let mut r = req();
    r.crosses_crate_boundary = true;
    r.is_public_api = false;
    assert!(matches!(
      classify_move_symbol(&r),
      MoveSymbolVerdict::MoveSymbolHeld {
        held_kind: MoveSymbolHeldKind::PrivateSymbolCrossesCrate,
        ..
      }
    ));
  }

  #[test]
  fn held_on_public_api_with_references() {
    let mut r = req();
    r.is_public_api = true;
    r.reference_paths = vec!["src/c.rs".to_string()];
    assert!(matches!(
      classify_move_symbol(&r),
      MoveSymbolVerdict::MoveSymbolHeld {
        held_kind: MoveSymbolHeldKind::PublicApiWithReferences,
        ..
      }
    ));
  }

  #[test]
  fn public_api_with_no_references_is_ready() {
    let mut r = req();
    r.is_public_api = true;
    // reference_paths is empty
    assert!(matches!(
      classify_move_symbol(&r),
      MoveSymbolVerdict::MoveSymbolReady
    ));
  }

  #[test]
  fn ladder_missing_paths_wins_over_invalid_name() {
    let mut r = req();
    r.source_path = "".to_string();
    r.symbol_name = "1bad".to_string();
    match classify_move_symbol(&r) {
      MoveSymbolVerdict::MoveSymbolHeld { held_kind, .. } => {
        assert_eq!(held_kind, MoveSymbolHeldKind::MissingPaths);
      }
      other => panic!("expected MissingPaths first, got {:?}", other),
    }
  }

  #[test]
  fn ladder_source_equals_dest_wins_over_invalid_name() {
    let mut r = req();
    r.destination_path = r.source_path.clone();
    r.symbol_name = "1bad".to_string();
    // SameFile (Rejected) runs at step 2, invalid-symbol-name at step 5.
    match classify_move_symbol(&r) {
      MoveSymbolVerdict::MoveSymbolRejected { held_kind, .. } => {
        assert_eq!(held_kind, MoveSymbolHeldKind::SourceEqualsDestination);
      }
      other => panic!("expected SourceEqualsDestination first, got {:?}", other),
    }
  }

  #[test]
  fn payload_canonical_fields() {
    let cand = compute_move_symbol_candidate(&req());
    let p = build_move_symbol_candidate_payload(&cand);
    assert_eq!(p["transform"].as_str(), Some("move-symbol"));
    assert_eq!(p["source_path"].as_str(), Some("src/a.rs"));
    assert_eq!(p["destination_path"].as_str(), Some("src/b.rs"));
    assert_eq!(p["verdict"].as_str(), Some("move-symbol-ready"));
    assert_eq!(p["reference_paths_count"].as_u64(), Some(0));
  }

  #[test]
  fn artifact_envelope_surfaces_both_paths() {
    let cand = compute_move_symbol_candidate(&req());
    let art = build_move_symbol_candidate_artifact(&cand, 0, None);
    let target_paths = art["target_paths"].as_array().unwrap();
    assert_eq!(target_paths.len(), 2);
    assert_eq!(target_paths[0].as_str(), Some("src/a.rs"));
    assert_eq!(target_paths[1].as_str(), Some("src/b.rs"));
    assert!(art["id"].as_str().unwrap().starts_with("move-symbol."));
  }

  #[test]
  fn artifact_id_differs_per_paths_or_name() {
    let cand_a = compute_move_symbol_candidate(&req());
    let mut req_b = req();
    req_b.destination_path = "src/c.rs".to_string();
    let cand_b = compute_move_symbol_candidate(&req_b);
    let mut req_c = req();
    req_c.symbol_name = "OtherSymbol".to_string();
    let cand_c = compute_move_symbol_candidate(&req_c);
    let a = build_move_symbol_candidate_artifact(&cand_a, 0, None)["id"].clone();
    let b = build_move_symbol_candidate_artifact(&cand_b, 0, None)["id"].clone();
    let c = build_move_symbol_candidate_artifact(&cand_c, 0, None)["id"].clone();
    assert_ne!(a, b);
    assert_ne!(a, c);
  }
}
