//! Inline-function deterministic code-transform host carrier.
//!
//! OWNER-LAW (2026-05-12): mirror of
//! `stdlib/lib/gate/code-transform/inline-function.px`. Verdict-ladder
//! carrier only; host CST emitter performs the actual inline +
//! definition delete under `ToolActionApproval`
//! (`CodeEditCapability::EditWithinTargetPaths`).

use serde::{Deserialize, Serialize};
use pnix_hash::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InlineFunctionHeldKind {
  TargetPathInvalid,
  LanguageNotSupported,
  InvalidFunctionName,
  RecursiveFunction,
  PublicApi,
  MultipleReturns,
  CapturesClosure,
  NoCallSites,
  TooManyCallSites,
}

impl InlineFunctionHeldKind {
  pub const ALL: &'static [Self] = &[
    Self::TargetPathInvalid,
    Self::LanguageNotSupported,
    Self::InvalidFunctionName,
    Self::RecursiveFunction,
    Self::PublicApi,
    Self::MultipleReturns,
    Self::CapturesClosure,
    Self::NoCallSites,
    Self::TooManyCallSites,
  ];
  pub fn as_str(self) -> &'static str {
    match self {
      Self::TargetPathInvalid => "target-path-invalid",
      Self::LanguageNotSupported => "language-not-supported",
      Self::InvalidFunctionName => "invalid-function-name",
      Self::RecursiveFunction => "recursive-function",
      Self::PublicApi => "public-api",
      Self::MultipleReturns => "multiple-returns",
      Self::CapturesClosure => "captures-closure",
      Self::NoCallSites => "no-call-sites",
      Self::TooManyCallSites => "too-many-call-sites",
    }
  }
}

pub const SUPPORTED_LANGUAGES: &[&str] = &["rust", "python", "typescript", "javascript", "go"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InlineFunctionRequest {
  pub target_path: String,
  pub language: String,
  pub function_name: String,
  /// Host-supplied list of call-site paths. The carrier only counts
  /// them; the host's symbol resolver is authoritative about
  /// "what is a call site".
  #[serde(default)]
  pub call_sites: Vec<String>,
  #[serde(default)]
  pub has_recursion: bool,
  #[serde(default)]
  pub is_public_api: bool,
  #[serde(default)]
  pub has_multiple_returns: bool,
  #[serde(default)]
  pub captures_env: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "verdict")]
pub enum InlineFunctionVerdict {
  InlineFunctionReady,
  InlineFunctionHeld {
    held_kind: InlineFunctionHeldKind,
    reason: String,
  },
  InlineFunctionRejected {
    held_kind: InlineFunctionHeldKind,
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

/// Mirror of `.px` `classify`. 9-step ladder.
pub fn classify_inline_function(req: &InlineFunctionRequest) -> InlineFunctionVerdict {
  if req.target_path.is_empty() || !is_path_in_project(&req.target_path) {
    return InlineFunctionVerdict::InlineFunctionHeld {
      held_kind: InlineFunctionHeldKind::TargetPathInvalid,
      reason: "target_path missing or out of project".to_string(),
    };
  }
  if !is_supported_language(&req.language) {
    return InlineFunctionVerdict::InlineFunctionHeld {
      held_kind: InlineFunctionHeldKind::LanguageNotSupported,
      reason: format!("language `{}` not supported", req.language),
    };
  }
  if !is_valid_identifier(&req.function_name) {
    return InlineFunctionVerdict::InlineFunctionRejected {
      held_kind: InlineFunctionHeldKind::InvalidFunctionName,
      reason: "function_name must be a valid identifier".to_string(),
    };
  }
  if req.has_recursion {
    return InlineFunctionVerdict::InlineFunctionRejected {
      held_kind: InlineFunctionHeldKind::RecursiveFunction,
      reason: "cannot inline a recursive function — would loop forever".to_string(),
    };
  }
  if req.is_public_api {
    return InlineFunctionVerdict::InlineFunctionHeld {
      held_kind: InlineFunctionHeldKind::PublicApi,
      reason:
        "function is part of public API; inlining removes the symbol from external callers — explicit owner approval required"
          .to_string(),
    };
  }
  if req.has_multiple_returns {
    return InlineFunctionVerdict::InlineFunctionHeld {
      held_kind: InlineFunctionHeldKind::MultipleReturns,
      reason: "function has multiple return paths; inlining requires control-flow flattening"
        .to_string(),
    };
  }
  if req.captures_env && req.language == "javascript" {
    return InlineFunctionVerdict::InlineFunctionHeld {
      held_kind: InlineFunctionHeldKind::CapturesClosure,
      reason: "function captures lexical environment; inlining changes scoping".to_string(),
    };
  }
  let cs = req.call_sites.len();
  if cs == 0 {
    return InlineFunctionVerdict::InlineFunctionHeld {
      held_kind: InlineFunctionHeldKind::NoCallSites,
      reason:
        "host found no call sites; inlining is just deletion — use remove-symbol owner instead"
          .to_string(),
    };
  }
  if cs > 16 {
    return InlineFunctionVerdict::InlineFunctionHeld {
      held_kind: InlineFunctionHeldKind::TooManyCallSites,
      reason: format!(
        "{cs} call sites; bulk inline grows binary size — explicit owner approval required"
      ),
    };
  }
  InlineFunctionVerdict::InlineFunctionReady
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InlineFunctionCandidate {
  pub request: InlineFunctionRequest,
  pub verdict: InlineFunctionVerdict,
}

pub fn compute_inline_function_candidate(
  request: &InlineFunctionRequest,
) -> InlineFunctionCandidate {
  let verdict = classify_inline_function(request);
  InlineFunctionCandidate {
    request: request.clone(),
    verdict,
  }
}

pub fn build_inline_function_candidate_payload(
  candidate: &InlineFunctionCandidate,
) -> serde_json::Value {
  let req = &candidate.request;
  let (verdict_str, next_step) = match &candidate.verdict {
    InlineFunctionVerdict::InlineFunctionReady => (
      "inline-function-ready",
      "host-cst-inline-at-call-sites-then-tool-action-approval",
    ),
    InlineFunctionVerdict::InlineFunctionHeld { .. } => {
      ("inline-function-held", "operator-decision-or-resubmit")
    }
    InlineFunctionVerdict::InlineFunctionRejected { .. } => {
      ("inline-function-rejected", "operator-decision-or-resubmit")
    }
  };
  let mut payload = serde_json::json!({
    "transform": "inline-function",
    "owner_law": "stdlib/lib/gate/code-transform/inline-function.px",
    "target_path": req.target_path,
    "language": req.language,
    "function_name": req.function_name,
    "call_sites_count": req.call_sites.len(),
    "has_recursion": req.has_recursion,
    "is_public_api": req.is_public_api,
    "has_multiple_returns": req.has_multiple_returns,
    "captures_env": req.captures_env,
    "verdict": verdict_str,
    "capability_required": "EditWithinTargetPaths",
    "candidate_only": true,
    "next_step": next_step,
  });
  match &candidate.verdict {
    InlineFunctionVerdict::InlineFunctionHeld { held_kind, reason }
    | InlineFunctionVerdict::InlineFunctionRejected { held_kind, reason } => {
      payload["held_kind"] = serde_json::Value::String(held_kind.as_str().to_string());
      payload["reason"] = serde_json::Value::String(reason.clone());
    }
    _ => {}
  }
  payload
}

pub fn build_inline_function_candidate_artifact(
  candidate: &InlineFunctionCandidate,
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
) -> serde_json::Value {
  let payload = build_inline_function_candidate_payload(candidate);
  let suffix = match &candidate.verdict {
    InlineFunctionVerdict::InlineFunctionReady => "ready",
    InlineFunctionVerdict::InlineFunctionHeld { .. } => "held",
    InlineFunctionVerdict::InlineFunctionRejected { .. } => "rejected",
  };
  let artifact_family = format!("coding.code-transform.inline-function-{suffix}");
  let mut hasher = Sha256::new();
  hasher.update(b"inline-function-candidate\x1f");
  hasher.update(candidate.request.target_path.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(candidate.request.language.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(candidate.request.function_name.as_bytes());
  hasher.update(b"\x1f");
  hasher.update((candidate.request.call_sites.len() as u64).to_le_bytes());
  hasher.update(&[
    candidate.request.has_recursion as u8,
    candidate.request.is_public_api as u8,
    candidate.request.has_multiple_returns as u8,
    candidate.request.captures_env as u8,
  ]);
  hasher.update(b"\x1f");
  hasher.update(suffix.as_bytes());
  let digest = hasher.finalize();
  let prefix = digest
    .iter()
    .take(8)
    .map(|b| format!("{b:02x}"))
    .collect::<String>();
  let id = format!("inline-function.{prefix}");
  let mut artifact = serde_json::json!({
    "id": id,
    "artifact_family": artifact_family,
    "source_surface": "code-transform.inline-function",
    "stored_at_ms": stored_at_ms,
    "target_paths": [candidate.request.target_path.clone()],
    "command_refs": serde_json::Value::Array(Vec::new()),
    "related_refs": serde_json::json!([
      "owner-law:stdlib/lib/gate/code-transform/inline-function.px"
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

  fn req(name: &str) -> InlineFunctionRequest {
    InlineFunctionRequest {
      target_path: "src/a.rs".to_string(),
      language: "rust".to_string(),
      function_name: name.to_string(),
      call_sites: vec!["src/a.rs:42".to_string(), "src/b.rs:13".to_string()],
      has_recursion: false,
      is_public_api: false,
      has_multiple_returns: false,
      captures_env: false,
    }
  }

  #[test]
  fn ready_for_well_formed() {
    assert!(matches!(
      classify_inline_function(&req("helper")),
      InlineFunctionVerdict::InlineFunctionReady
    ));
  }

  #[test]
  fn held_on_target_path_invalid() {
    let mut r = req("ok");
    r.target_path = "../bad.rs".to_string();
    assert!(matches!(
      classify_inline_function(&r),
      InlineFunctionVerdict::InlineFunctionHeld {
        held_kind: InlineFunctionHeldKind::TargetPathInvalid,
        ..
      }
    ));
  }

  #[test]
  fn held_on_language_not_supported() {
    let mut r = req("ok");
    r.language = "fortran".to_string();
    assert!(matches!(
      classify_inline_function(&r),
      InlineFunctionVerdict::InlineFunctionHeld {
        held_kind: InlineFunctionHeldKind::LanguageNotSupported,
        ..
      }
    ));
  }

  #[test]
  fn rejected_on_invalid_function_name() {
    let r = req("1bad");
    assert!(matches!(
      classify_inline_function(&r),
      InlineFunctionVerdict::InlineFunctionRejected {
        held_kind: InlineFunctionHeldKind::InvalidFunctionName,
        ..
      }
    ));
  }

  #[test]
  fn rejected_on_recursive_function() {
    let mut r = req("ok");
    r.has_recursion = true;
    assert!(matches!(
      classify_inline_function(&r),
      InlineFunctionVerdict::InlineFunctionRejected {
        held_kind: InlineFunctionHeldKind::RecursiveFunction,
        ..
      }
    ));
  }

  #[test]
  fn held_on_public_api() {
    let mut r = req("ok");
    r.is_public_api = true;
    assert!(matches!(
      classify_inline_function(&r),
      InlineFunctionVerdict::InlineFunctionHeld {
        held_kind: InlineFunctionHeldKind::PublicApi,
        ..
      }
    ));
  }

  #[test]
  fn held_on_multiple_returns() {
    let mut r = req("ok");
    r.has_multiple_returns = true;
    assert!(matches!(
      classify_inline_function(&r),
      InlineFunctionVerdict::InlineFunctionHeld {
        held_kind: InlineFunctionHeldKind::MultipleReturns,
        ..
      }
    ));
  }

  #[test]
  fn held_on_captures_env_only_in_javascript() {
    let mut r = req("ok");
    r.language = "javascript".to_string();
    r.captures_env = true;
    assert!(matches!(
      classify_inline_function(&r),
      InlineFunctionVerdict::InlineFunctionHeld {
        held_kind: InlineFunctionHeldKind::CapturesClosure,
        ..
      }
    ));
    // Same captures_env=true in Rust does NOT hold (Rust closures
    // are explicit; the .px gate only flags this for JS).
    r.language = "rust".to_string();
    assert!(matches!(
      classify_inline_function(&r),
      InlineFunctionVerdict::InlineFunctionReady
    ));
  }

  #[test]
  fn held_on_no_call_sites() {
    let mut r = req("ok");
    r.call_sites.clear();
    assert!(matches!(
      classify_inline_function(&r),
      InlineFunctionVerdict::InlineFunctionHeld {
        held_kind: InlineFunctionHeldKind::NoCallSites,
        ..
      }
    ));
  }

  #[test]
  fn held_on_too_many_call_sites() {
    let mut r = req("ok");
    r.call_sites = (0..17).map(|i| format!("src/a.rs:{i}")).collect();
    assert!(matches!(
      classify_inline_function(&r),
      InlineFunctionVerdict::InlineFunctionHeld {
        held_kind: InlineFunctionHeldKind::TooManyCallSites,
        ..
      }
    ));
  }

  #[test]
  fn ladder_recursive_wins_over_public_api() {
    let mut r = req("ok");
    r.has_recursion = true;
    r.is_public_api = true;
    // Recursive (Rejected) runs at step 4, public-api at step 5.
    match classify_inline_function(&r) {
      InlineFunctionVerdict::InlineFunctionRejected { held_kind, .. } => {
        assert_eq!(held_kind, InlineFunctionHeldKind::RecursiveFunction);
      }
      other => panic!("expected RecursiveFunction first, got {:?}", other),
    }
  }

  #[test]
  fn payload_canonical_fields() {
    let cand = compute_inline_function_candidate(&req("helper"));
    let p = build_inline_function_candidate_payload(&cand);
    assert_eq!(p["transform"].as_str(), Some("inline-function"));
    assert_eq!(p["verdict"].as_str(), Some("inline-function-ready"));
    assert_eq!(p["call_sites_count"].as_u64(), Some(2));
    assert_eq!(
      p["capability_required"].as_str(),
      Some("EditWithinTargetPaths")
    );
  }

  #[test]
  fn artifact_replay_stable_and_id_differs_per_inputs() {
    let cand_a = compute_inline_function_candidate(&req("a"));
    let cand_b = compute_inline_function_candidate(&req("b"));
    let a = build_inline_function_candidate_artifact(&cand_a, 1000, None);
    let b = build_inline_function_candidate_artifact(&cand_b, 1000, None);
    assert!(a["id"].as_str().unwrap().starts_with("inline-function."));
    assert_ne!(a["id"], b["id"]);
    let a2 = build_inline_function_candidate_artifact(&cand_a, 9999, None);
    assert_eq!(a["id"], a2["id"]); // stored_at_ms extrinsic
  }
}
