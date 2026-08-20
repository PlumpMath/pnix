//! Extract-function deterministic code-transform host carrier.
//!
//! OWNER-LAW (2026-05-12): mirror of
//! `stdlib/lib/gate/code-transform/extract-function.px`. Like
//! `add-test-stub` and `add-import`, this carrier owns the **verdict
//! ladder only** — the host's CST emitter performs the actual
//! extraction (new function signature from captured_locals +
//! selection_returns) under `ToolActionApproval`
//! (`CodeEditCapability::EditWithinTargetPaths`).

use serde::{Deserialize, Serialize};
use pnix_hash::{Digest, Sha256};

/// Selection range (1-indexed line bounds) for the code block to
/// extract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtractFunctionSelection {
  pub start_line: u64,
  pub end_line: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExtractFunctionHeldKind {
  TargetPathInvalid,
  LanguageNotSupported,
  InvalidFunctionName,
  InvalidSelection,
  SelectionHasEarlyReturn,
  SelectionHasAwait,
  CapturedLocalsTooMany,
}

impl ExtractFunctionHeldKind {
  /// Every variant of this enum, in `.px` ladder order. The
  /// `code_transform_owner_carrier_sync` integration test reads
  /// this slice as the canonical "what kebab strings this carrier
  /// emits", then compares against the `.px` owner law to detect
  /// drift in either direction.
  pub const ALL: &'static [Self] = &[
    Self::TargetPathInvalid,
    Self::LanguageNotSupported,
    Self::InvalidFunctionName,
    Self::InvalidSelection,
    Self::SelectionHasEarlyReturn,
    Self::SelectionHasAwait,
    Self::CapturedLocalsTooMany,
  ];
  pub fn as_str(self) -> &'static str {
    match self {
      Self::TargetPathInvalid => "target-path-invalid",
      Self::LanguageNotSupported => "language-not-supported",
      Self::InvalidFunctionName => "invalid-function-name",
      Self::InvalidSelection => "invalid-selection",
      Self::SelectionHasEarlyReturn => "selection-has-early-return",
      Self::SelectionHasAwait => "selection-has-await",
      Self::CapturedLocalsTooMany => "captured-locals-too-many",
    }
  }
}

/// Supported source languages for this transform. Must stay byte-
/// identical to the `.px` owner law's `supportedLanguages` list.
pub const SUPPORTED_LANGUAGES: &[&str] = &["rust", "python", "typescript", "javascript", "go"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtractFunctionRequest {
  pub target_path: String,
  pub language: String,
  pub new_function_name: String,
  pub selection: ExtractFunctionSelection,
  #[serde(default)]
  pub captured_locals: Vec<String>,
  #[serde(default)]
  pub has_early_return: bool,
  #[serde(default)]
  pub has_await: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "verdict")]
pub enum ExtractFunctionVerdict {
  ExtractFunctionReady,
  ExtractFunctionHeld {
    held_kind: ExtractFunctionHeldKind,
    reason: String,
  },
  ExtractFunctionRejected {
    held_kind: ExtractFunctionHeldKind,
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

/// Mirror of `.px` `classify`. Captures the 7-step ladder.
pub fn classify_extract_function(req: &ExtractFunctionRequest) -> ExtractFunctionVerdict {
  if req.target_path.is_empty() || !is_path_in_project(&req.target_path) {
    return ExtractFunctionVerdict::ExtractFunctionHeld {
      held_kind: ExtractFunctionHeldKind::TargetPathInvalid,
      reason: "target_path missing or out of project".to_string(),
    };
  }
  if !is_supported_language(&req.language) {
    return ExtractFunctionVerdict::ExtractFunctionHeld {
      held_kind: ExtractFunctionHeldKind::LanguageNotSupported,
      reason: format!("language `{}` not supported", req.language),
    };
  }
  if !is_valid_identifier(&req.new_function_name) {
    return ExtractFunctionVerdict::ExtractFunctionRejected {
      held_kind: ExtractFunctionHeldKind::InvalidFunctionName,
      reason: "new_function_name must be a valid identifier".to_string(),
    };
  }
  if req.selection.start_line == 0
    || req.selection.end_line == 0
    || req.selection.end_line < req.selection.start_line
  {
    return ExtractFunctionVerdict::ExtractFunctionHeld {
      held_kind: ExtractFunctionHeldKind::InvalidSelection,
      reason: "selection.{start_line,end_line} required and end_line >= start_line".to_string(),
    };
  }
  if req.has_early_return {
    return ExtractFunctionVerdict::ExtractFunctionHeld {
      held_kind: ExtractFunctionHeldKind::SelectionHasEarlyReturn,
      reason:
        "selection contains return/throw/break — extracting changes control flow; rewriter must add explicit return value or operator must split selection"
          .to_string(),
    };
  }
  if req.has_await && req.language != "rust" {
    return ExtractFunctionVerdict::ExtractFunctionHeld {
      held_kind: ExtractFunctionHeldKind::SelectionHasAwait,
      reason: "selection contains `await`; extracting requires async-aware rewriter".to_string(),
    };
  }
  if req.captured_locals.len() > 8 {
    return ExtractFunctionVerdict::ExtractFunctionHeld {
      held_kind: ExtractFunctionHeldKind::CapturedLocalsTooMany,
      reason: format!(
        "selection captures {} locals (>8); split selection or use a struct",
        req.captured_locals.len()
      ),
    };
  }
  ExtractFunctionVerdict::ExtractFunctionReady
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtractFunctionCandidate {
  pub request: ExtractFunctionRequest,
  pub verdict: ExtractFunctionVerdict,
}

pub fn compute_extract_function_candidate(
  request: &ExtractFunctionRequest,
) -> ExtractFunctionCandidate {
  let verdict = classify_extract_function(request);
  ExtractFunctionCandidate {
    request: request.clone(),
    verdict,
  }
}

pub fn build_extract_function_candidate_payload(
  candidate: &ExtractFunctionCandidate,
) -> serde_json::Value {
  let req = &candidate.request;
  let (verdict_str, next_step) = match &candidate.verdict {
    ExtractFunctionVerdict::ExtractFunctionReady => (
      "extract-function-ready",
      "host-cst-rewrite-then-tool-action-approval",
    ),
    ExtractFunctionVerdict::ExtractFunctionHeld { .. } => {
      ("extract-function-held", "operator-decision-or-resubmit")
    }
    ExtractFunctionVerdict::ExtractFunctionRejected { .. } => {
      ("extract-function-rejected", "operator-decision-or-resubmit")
    }
  };
  let mut payload = serde_json::json!({
    "transform": "extract-function",
    "owner_law": "stdlib/lib/gate/code-transform/extract-function.px",
    "target_path": req.target_path,
    "language": req.language,
    "new_function_name": req.new_function_name,
    "selection": {
      "start_line": req.selection.start_line,
      "end_line": req.selection.end_line,
    },
    "captured_locals_count": req.captured_locals.len(),
    "has_early_return": req.has_early_return,
    "has_await": req.has_await,
    "verdict": verdict_str,
    "capability_required": "EditWithinTargetPaths",
    "candidate_only": true,
    "next_step": next_step,
  });
  match &candidate.verdict {
    ExtractFunctionVerdict::ExtractFunctionHeld { held_kind, reason }
    | ExtractFunctionVerdict::ExtractFunctionRejected { held_kind, reason } => {
      payload["held_kind"] = serde_json::Value::String(held_kind.as_str().to_string());
      payload["reason"] = serde_json::Value::String(reason.clone());
    }
    _ => {}
  }
  payload
}

pub fn build_extract_function_candidate_artifact(
  candidate: &ExtractFunctionCandidate,
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
) -> serde_json::Value {
  let payload = build_extract_function_candidate_payload(candidate);
  let suffix = match &candidate.verdict {
    ExtractFunctionVerdict::ExtractFunctionReady => "ready",
    ExtractFunctionVerdict::ExtractFunctionHeld { .. } => "held",
    ExtractFunctionVerdict::ExtractFunctionRejected { .. } => "rejected",
  };
  let artifact_family = format!("coding.code-transform.extract-function-{suffix}");
  let mut hasher = Sha256::new();
  hasher.update(b"extract-function-candidate\x1f");
  hasher.update(candidate.request.target_path.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(candidate.request.language.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(candidate.request.new_function_name.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(candidate.request.selection.start_line.to_le_bytes());
  hasher.update(candidate.request.selection.end_line.to_le_bytes());
  hasher.update(b"\x1f");
  hasher.update((candidate.request.captured_locals.len() as u64).to_le_bytes());
  hasher.update(&[
    candidate.request.has_early_return as u8,
    candidate.request.has_await as u8,
  ]);
  hasher.update(b"\x1f");
  hasher.update(suffix.as_bytes());
  let digest = hasher.finalize();
  let prefix = digest
    .iter()
    .take(8)
    .map(|b| format!("{b:02x}"))
    .collect::<String>();
  let id = format!("extract-function.{prefix}");
  let mut artifact = serde_json::json!({
    "id": id,
    "artifact_family": artifact_family,
    "source_surface": "code-transform.extract-function",
    "stored_at_ms": stored_at_ms,
    "target_paths": [candidate.request.target_path.clone()],
    "command_refs": serde_json::Value::Array(Vec::new()),
    "related_refs": serde_json::json!([
      "owner-law:stdlib/lib/gate/code-transform/extract-function.px"
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

  fn req(name: &str) -> ExtractFunctionRequest {
    ExtractFunctionRequest {
      target_path: "src/a.rs".to_string(),
      language: "rust".to_string(),
      new_function_name: name.to_string(),
      selection: ExtractFunctionSelection {
        start_line: 10,
        end_line: 15,
      },
      captured_locals: vec!["x".to_string(), "y".to_string()],
      has_early_return: false,
      has_await: false,
    }
  }

  #[test]
  fn ready_for_well_formed() {
    assert!(matches!(
      classify_extract_function(&req("extracted")),
      ExtractFunctionVerdict::ExtractFunctionReady
    ));
  }

  #[test]
  fn held_on_target_path_invalid() {
    let mut r = req("ok");
    r.target_path = "".to_string();
    assert!(matches!(
      classify_extract_function(&r),
      ExtractFunctionVerdict::ExtractFunctionHeld {
        held_kind: ExtractFunctionHeldKind::TargetPathInvalid,
        ..
      }
    ));
    r.target_path = "../escape.rs".to_string();
    assert!(matches!(
      classify_extract_function(&r),
      ExtractFunctionVerdict::ExtractFunctionHeld {
        held_kind: ExtractFunctionHeldKind::TargetPathInvalid,
        ..
      }
    ));
  }

  #[test]
  fn held_on_language_not_supported() {
    let mut r = req("ok");
    r.language = "fortran".to_string();
    assert!(matches!(
      classify_extract_function(&r),
      ExtractFunctionVerdict::ExtractFunctionHeld {
        held_kind: ExtractFunctionHeldKind::LanguageNotSupported,
        ..
      }
    ));
  }

  #[test]
  fn rejected_on_invalid_function_name() {
    for bad in ["", "1bad", "has space", "with-hyphen"] {
      let r = req(bad);
      assert!(
        matches!(
          classify_extract_function(&r),
          ExtractFunctionVerdict::ExtractFunctionRejected {
            held_kind: ExtractFunctionHeldKind::InvalidFunctionName,
            ..
          }
        ),
        "for name '{bad}'"
      );
    }
  }

  #[test]
  fn held_on_invalid_selection() {
    let mut r = req("ok");
    r.selection = ExtractFunctionSelection {
      start_line: 0,
      end_line: 0,
    };
    assert!(matches!(
      classify_extract_function(&r),
      ExtractFunctionVerdict::ExtractFunctionHeld {
        held_kind: ExtractFunctionHeldKind::InvalidSelection,
        ..
      }
    ));
    r.selection = ExtractFunctionSelection {
      start_line: 20,
      end_line: 10,
    };
    assert!(matches!(
      classify_extract_function(&r),
      ExtractFunctionVerdict::ExtractFunctionHeld {
        held_kind: ExtractFunctionHeldKind::InvalidSelection,
        ..
      }
    ));
  }

  #[test]
  fn held_on_early_return() {
    let mut r = req("ok");
    r.has_early_return = true;
    assert!(matches!(
      classify_extract_function(&r),
      ExtractFunctionVerdict::ExtractFunctionHeld {
        held_kind: ExtractFunctionHeldKind::SelectionHasEarlyReturn,
        ..
      }
    ));
  }

  #[test]
  fn held_on_await_in_non_rust() {
    let mut r = req("ok");
    r.language = "javascript".to_string();
    r.has_await = true;
    assert!(matches!(
      classify_extract_function(&r),
      ExtractFunctionVerdict::ExtractFunctionHeld {
        held_kind: ExtractFunctionHeldKind::SelectionHasAwait,
        ..
      }
    ));
  }

  #[test]
  fn await_in_rust_passes_through() {
    // Rust async fns are first-class; the .px excludes Rust from
    // the await Hold.
    let mut r = req("ok");
    r.has_await = true; // language is rust
    assert!(matches!(
      classify_extract_function(&r),
      ExtractFunctionVerdict::ExtractFunctionReady
    ));
  }

  #[test]
  fn held_on_captured_locals_too_many() {
    let mut r = req("ok");
    r.captured_locals = (0..9).map(|i| format!("v{i}")).collect();
    assert!(matches!(
      classify_extract_function(&r),
      ExtractFunctionVerdict::ExtractFunctionHeld {
        held_kind: ExtractFunctionHeldKind::CapturedLocalsTooMany,
        ..
      }
    ));
  }

  #[test]
  fn ladder_target_path_wins_over_invalid_name() {
    let mut r = req("1bad");
    r.target_path = "".to_string();
    match classify_extract_function(&r) {
      ExtractFunctionVerdict::ExtractFunctionHeld { held_kind, .. } => {
        assert_eq!(held_kind, ExtractFunctionHeldKind::TargetPathInvalid);
      }
      other => panic!("expected TargetPathInvalid first, got {:?}", other),
    }
  }

  #[test]
  fn payload_canonical_fields() {
    let cand = compute_extract_function_candidate(&req("extracted"));
    let p = build_extract_function_candidate_payload(&cand);
    assert_eq!(p["transform"].as_str(), Some("extract-function"));
    assert_eq!(p["verdict"].as_str(), Some("extract-function-ready"));
    assert_eq!(
      p["capability_required"].as_str(),
      Some("EditWithinTargetPaths")
    );
    assert_eq!(p["captured_locals_count"].as_u64(), Some(2));
    assert_eq!(p["selection"]["start_line"].as_u64(), Some(10));
    assert_eq!(p["selection"]["end_line"].as_u64(), Some(15));
  }

  #[test]
  fn artifact_envelope_and_id_replay_stable() {
    let cand = compute_extract_function_candidate(&req("extracted"));
    let a = build_extract_function_candidate_artifact(&cand, 1000, None);
    let b = build_extract_function_candidate_artifact(&cand, 9999, None);
    assert_eq!(a["id"], b["id"]);
    assert!(a["id"].as_str().unwrap().starts_with("extract-function."));
    assert_eq!(
      a["artifact_family"].as_str(),
      Some("coding.code-transform.extract-function-ready")
    );
  }

  #[test]
  fn artifact_id_differs_per_selection_or_name() {
    let base = compute_extract_function_candidate(&req("a"));
    let diff_name = compute_extract_function_candidate(&req("b"));
    let mut req_diff_sel = req("a");
    req_diff_sel.selection = ExtractFunctionSelection {
      start_line: 20,
      end_line: 25,
    };
    let diff_sel = compute_extract_function_candidate(&req_diff_sel);
    let id_base = build_extract_function_candidate_artifact(&base, 0, None)["id"].clone();
    let id_n = build_extract_function_candidate_artifact(&diff_name, 0, None)["id"].clone();
    let id_s = build_extract_function_candidate_artifact(&diff_sel, 0, None)["id"].clone();
    assert_ne!(id_base, id_n);
    assert_ne!(id_base, id_s);
  }
}
