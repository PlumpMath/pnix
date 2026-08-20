//! Change-signature deterministic code-transform host carrier.
//!
//! OWNER-LAW (2026-05-12): mirror of
//! `stdlib/lib/gate/code-transform/change-signature.px`. Verdict-ladder
//! carrier only; host CST emitter performs the actual signature edit +
//! call-site rewrite under `ToolActionApproval`
//! (`CodeEditCapability::EditWithinTargetPaths`).

use serde::{Deserialize, Serialize};
use pnix_hash::{Digest, Sha256};

/// Held / Rejected kinds emitted by [`classify_change_signature`].
/// Kebab-case strings must stay byte-identical to the `.px` ladder —
/// the `scripts/check-code-transform-owner-carrier-sync.sh` guard
/// catches drift in either direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChangeSignatureHeldKind {
  TargetPathInvalid,
  LanguageNotSupported,
  InvalidFunctionName,
  InvalidChangeKind,
  RemovesUsedParameter,
  AddsRequiredWithoutDefault,
  ReturnTypeChangeWithPublicApi,
  AsyncBoundaryCrossed,
  NoCallSites,
  TooManyCallSites,
}

impl ChangeSignatureHeldKind {
  pub const ALL: &'static [Self] = &[
    Self::TargetPathInvalid,
    Self::LanguageNotSupported,
    Self::InvalidFunctionName,
    Self::InvalidChangeKind,
    Self::RemovesUsedParameter,
    Self::AddsRequiredWithoutDefault,
    Self::ReturnTypeChangeWithPublicApi,
    Self::AsyncBoundaryCrossed,
    Self::NoCallSites,
    Self::TooManyCallSites,
  ];
  pub fn as_str(self) -> &'static str {
    match self {
      Self::TargetPathInvalid => "target-path-invalid",
      Self::LanguageNotSupported => "language-not-supported",
      Self::InvalidFunctionName => "invalid-function-name",
      Self::InvalidChangeKind => "invalid-change-kind",
      Self::RemovesUsedParameter => "removes-used-parameter",
      Self::AddsRequiredWithoutDefault => "adds-required-without-default",
      Self::ReturnTypeChangeWithPublicApi => "return-type-change-with-public-api",
      Self::AsyncBoundaryCrossed => "async-boundary-crossed",
      Self::NoCallSites => "no-call-sites",
      Self::TooManyCallSites => "too-many-call-sites",
    }
  }
}

pub const SUPPORTED_LANGUAGES: &[&str] = &["rust", "python", "typescript", "javascript", "go"];

/// Five recognised change kinds — must stay byte-identical to the `.px`
/// `validChangeKinds` list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChangeSignatureKind {
  AddParam,
  RemoveParam,
  ChangeParamType,
  ChangeReturnType,
  ReorderParams,
}

impl ChangeSignatureKind {
  pub const ALL: &'static [Self] = &[
    Self::AddParam,
    Self::RemoveParam,
    Self::ChangeParamType,
    Self::ChangeReturnType,
    Self::ReorderParams,
  ];
  pub fn as_str(self) -> &'static str {
    match self {
      Self::AddParam => "add-param",
      Self::RemoveParam => "remove-param",
      Self::ChangeParamType => "change-param-type",
      Self::ChangeReturnType => "change-return-type",
      Self::ReorderParams => "reorder-params",
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeSignatureRequest {
  pub target_path: String,
  pub language: String,
  pub function_name: String,
  /// Either a recognised kind or — for the `InvalidChangeKind` Held lane
  /// — an arbitrary string. We carry it as `String` so the carrier can
  /// reject unknown values explicitly instead of refusing them at
  /// serde-deserialize time.
  pub change_kind: String,
  /// Host-supplied count. The carrier only consults the count; the
  /// host's symbol resolver is authoritative about "what is a call
  /// site".
  #[serde(default)]
  pub call_sites_count: u64,
  /// Only meaningful when `change_kind == "remove-param"`.
  #[serde(default)]
  pub removed_param_used_at_callers: bool,
  /// Only meaningful when `change_kind == "add-param"`.
  #[serde(default)]
  pub adds_required_no_default: bool,
  /// Return-type change that switches sync ↔ async.
  #[serde(default)]
  pub crosses_async_boundary: bool,
  #[serde(default)]
  pub is_public_api: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "verdict")]
pub enum ChangeSignatureVerdict {
  ChangeSignatureReady,
  ChangeSignatureHeld {
    held_kind: ChangeSignatureHeldKind,
    reason: String,
  },
  ChangeSignatureRejected {
    held_kind: ChangeSignatureHeldKind,
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

fn parse_change_kind(s: &str) -> Option<ChangeSignatureKind> {
  match s {
    "add-param" => Some(ChangeSignatureKind::AddParam),
    "remove-param" => Some(ChangeSignatureKind::RemoveParam),
    "change-param-type" => Some(ChangeSignatureKind::ChangeParamType),
    "change-return-type" => Some(ChangeSignatureKind::ChangeReturnType),
    "reorder-params" => Some(ChangeSignatureKind::ReorderParams),
    _ => None,
  }
}

/// Mirror of `.px` `classify`. 10-step ladder.
pub fn classify_change_signature(req: &ChangeSignatureRequest) -> ChangeSignatureVerdict {
  if req.target_path.is_empty() || !is_path_in_project(&req.target_path) {
    return ChangeSignatureVerdict::ChangeSignatureHeld {
      held_kind: ChangeSignatureHeldKind::TargetPathInvalid,
      reason: "target_path missing or out of project".to_string(),
    };
  }
  if !is_supported_language(&req.language) {
    return ChangeSignatureVerdict::ChangeSignatureHeld {
      held_kind: ChangeSignatureHeldKind::LanguageNotSupported,
      reason: format!("language `{}` not supported", req.language),
    };
  }
  if !is_valid_identifier(&req.function_name) {
    return ChangeSignatureVerdict::ChangeSignatureRejected {
      held_kind: ChangeSignatureHeldKind::InvalidFunctionName,
      reason: "function_name must be a valid identifier".to_string(),
    };
  }
  let kind = match parse_change_kind(&req.change_kind) {
    Some(k) => k,
    None => {
      return ChangeSignatureVerdict::ChangeSignatureRejected {
        held_kind: ChangeSignatureHeldKind::InvalidChangeKind,
        reason: format!(
          "change_kind `{}` not in {{add-param, remove-param, change-param-type, change-return-type, reorder-params}}",
          req.change_kind
        ),
      };
    }
  };
  if matches!(kind, ChangeSignatureKind::RemoveParam) && req.removed_param_used_at_callers {
    return ChangeSignatureVerdict::ChangeSignatureHeld {
      held_kind: ChangeSignatureHeldKind::RemovesUsedParameter,
      reason:
        "remove-param: removed parameter is still referenced at one or more call sites; remove uses first"
          .to_string(),
    };
  }
  if matches!(kind, ChangeSignatureKind::AddParam)
    && req.adds_required_no_default
    && req.call_sites_count > 0
  {
    return ChangeSignatureVerdict::ChangeSignatureHeld {
      held_kind: ChangeSignatureHeldKind::AddsRequiredWithoutDefault,
      reason: format!(
        "add-param adds a required parameter without a default; {} call sites would break — supply a default or stage with separate caller-update receipt",
        req.call_sites_count
      ),
    };
  }
  if matches!(kind, ChangeSignatureKind::ChangeReturnType) && req.is_public_api {
    return ChangeSignatureVerdict::ChangeSignatureHeld {
      held_kind: ChangeSignatureHeldKind::ReturnTypeChangeWithPublicApi,
      reason:
        "change-return-type on a public-API function changes the contract for external callers — explicit owner approval required"
          .to_string(),
    };
  }
  if req.crosses_async_boundary {
    return ChangeSignatureVerdict::ChangeSignatureHeld {
      held_kind: ChangeSignatureHeldKind::AsyncBoundaryCrossed,
      reason:
        "signature change crosses sync ↔ async boundary; every caller must adopt the await/blocking shape — stage as separate slice"
          .to_string(),
    };
  }
  if req.call_sites_count == 0 {
    return ChangeSignatureVerdict::ChangeSignatureHeld {
      held_kind: ChangeSignatureHeldKind::NoCallSites,
      reason:
        "host found no call sites; consider remove-symbol owner instead if the function is unused"
          .to_string(),
    };
  }
  if req.call_sites_count > 16 {
    return ChangeSignatureVerdict::ChangeSignatureHeld {
      held_kind: ChangeSignatureHeldKind::TooManyCallSites,
      reason: format!(
        "{} call sites; signature changes with large fan-out need explicit owner approval",
        req.call_sites_count
      ),
    };
  }
  ChangeSignatureVerdict::ChangeSignatureReady
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeSignatureCandidate {
  pub request: ChangeSignatureRequest,
  pub verdict: ChangeSignatureVerdict,
}

pub fn compute_change_signature_candidate(
  request: &ChangeSignatureRequest,
) -> ChangeSignatureCandidate {
  let verdict = classify_change_signature(request);
  ChangeSignatureCandidate {
    request: request.clone(),
    verdict,
  }
}

pub fn build_change_signature_candidate_payload(
  candidate: &ChangeSignatureCandidate,
) -> serde_json::Value {
  let req = &candidate.request;
  let (verdict_str, next_step) = match &candidate.verdict {
    ChangeSignatureVerdict::ChangeSignatureReady => (
      "change-signature-ready",
      "host-cst-edit-signature-and-rewrite-call-sites-then-tool-action-approval",
    ),
    ChangeSignatureVerdict::ChangeSignatureHeld { .. } => {
      ("change-signature-held", "operator-decision-or-resubmit")
    }
    ChangeSignatureVerdict::ChangeSignatureRejected { .. } => {
      ("change-signature-rejected", "operator-decision-or-resubmit")
    }
  };
  let mut payload = serde_json::json!({
    "transform": "change-signature",
    "owner_law": "stdlib/lib/gate/code-transform/change-signature.px",
    "target_path": req.target_path,
    "language": req.language,
    "function_name": req.function_name,
    "change_kind": req.change_kind,
    "call_sites_count": req.call_sites_count,
    "removed_param_used_at_callers": req.removed_param_used_at_callers,
    "adds_required_no_default": req.adds_required_no_default,
    "crosses_async_boundary": req.crosses_async_boundary,
    "is_public_api": req.is_public_api,
    "verdict": verdict_str,
    "capability_required": "EditWithinTargetPaths",
    "candidate_only": true,
    "next_step": next_step,
  });
  match &candidate.verdict {
    ChangeSignatureVerdict::ChangeSignatureHeld { held_kind, reason }
    | ChangeSignatureVerdict::ChangeSignatureRejected { held_kind, reason } => {
      payload["held_kind"] = serde_json::Value::String(held_kind.as_str().to_string());
      payload["reason"] = serde_json::Value::String(reason.clone());
    }
    _ => {}
  }
  payload
}

pub fn build_change_signature_candidate_artifact(
  candidate: &ChangeSignatureCandidate,
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
) -> serde_json::Value {
  let payload = build_change_signature_candidate_payload(candidate);
  let suffix = match &candidate.verdict {
    ChangeSignatureVerdict::ChangeSignatureReady => "ready",
    ChangeSignatureVerdict::ChangeSignatureHeld { .. } => "held",
    ChangeSignatureVerdict::ChangeSignatureRejected { .. } => "rejected",
  };
  let artifact_family = format!("coding.code-transform.change-signature-{suffix}");
  let mut hasher = Sha256::new();
  hasher.update(b"change-signature-candidate\x1f");
  hasher.update(candidate.request.target_path.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(candidate.request.language.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(candidate.request.function_name.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(candidate.request.change_kind.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(candidate.request.call_sites_count.to_le_bytes());
  hasher.update(&[
    candidate.request.removed_param_used_at_callers as u8,
    candidate.request.adds_required_no_default as u8,
    candidate.request.crosses_async_boundary as u8,
    candidate.request.is_public_api as u8,
  ]);
  hasher.update(b"\x1f");
  hasher.update(suffix.as_bytes());
  let digest = hasher.finalize();
  let prefix = digest
    .iter()
    .take(8)
    .map(|b| format!("{b:02x}"))
    .collect::<String>();
  let id = format!("change-signature.{prefix}");
  let mut artifact = serde_json::json!({
    "id": id,
    "artifact_family": artifact_family,
    "source_surface": "code-transform.change-signature",
    "stored_at_ms": stored_at_ms,
    "target_paths": [candidate.request.target_path.clone()],
    "command_refs": serde_json::Value::Array(Vec::new()),
    "related_refs": serde_json::json!([
      "owner-law:stdlib/lib/gate/code-transform/change-signature.px"
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

  fn req() -> ChangeSignatureRequest {
    ChangeSignatureRequest {
      target_path: "src/a.rs".to_string(),
      language: "rust".to_string(),
      function_name: "frob".to_string(),
      change_kind: "change-param-type".to_string(),
      call_sites_count: 3,
      removed_param_used_at_callers: false,
      adds_required_no_default: false,
      crosses_async_boundary: false,
      is_public_api: false,
    }
  }

  #[test]
  fn ready_when_preconditions_satisfied() {
    let r = req();
    assert!(matches!(
      classify_change_signature(&r),
      ChangeSignatureVerdict::ChangeSignatureReady
    ));
  }

  #[test]
  fn held_on_missing_target_path() {
    let mut r = req();
    r.target_path = String::new();
    assert!(matches!(
      classify_change_signature(&r),
      ChangeSignatureVerdict::ChangeSignatureHeld {
        held_kind: ChangeSignatureHeldKind::TargetPathInvalid,
        ..
      }
    ));
  }

  #[test]
  fn held_on_path_with_parent_escape() {
    let mut r = req();
    r.target_path = "../outside.rs".to_string();
    assert!(matches!(
      classify_change_signature(&r),
      ChangeSignatureVerdict::ChangeSignatureHeld {
        held_kind: ChangeSignatureHeldKind::TargetPathInvalid,
        ..
      }
    ));
  }

  #[test]
  fn held_on_unsupported_language() {
    let mut r = req();
    r.language = "cobol".to_string();
    assert!(matches!(
      classify_change_signature(&r),
      ChangeSignatureVerdict::ChangeSignatureHeld {
        held_kind: ChangeSignatureHeldKind::LanguageNotSupported,
        ..
      }
    ));
  }

  #[test]
  fn rejected_on_invalid_function_name() {
    let mut r = req();
    r.function_name = "1bad".to_string();
    assert!(matches!(
      classify_change_signature(&r),
      ChangeSignatureVerdict::ChangeSignatureRejected {
        held_kind: ChangeSignatureHeldKind::InvalidFunctionName,
        ..
      }
    ));
  }

  #[test]
  fn rejected_on_unknown_change_kind() {
    let mut r = req();
    r.change_kind = "rename-param".to_string();
    assert!(matches!(
      classify_change_signature(&r),
      ChangeSignatureVerdict::ChangeSignatureRejected {
        held_kind: ChangeSignatureHeldKind::InvalidChangeKind,
        ..
      }
    ));
  }

  #[test]
  fn held_when_remove_param_is_used_at_callers() {
    let mut r = req();
    r.change_kind = "remove-param".to_string();
    r.removed_param_used_at_callers = true;
    assert!(matches!(
      classify_change_signature(&r),
      ChangeSignatureVerdict::ChangeSignatureHeld {
        held_kind: ChangeSignatureHeldKind::RemovesUsedParameter,
        ..
      }
    ));
  }

  #[test]
  fn ready_when_remove_param_is_unused_at_callers() {
    let mut r = req();
    r.change_kind = "remove-param".to_string();
    r.removed_param_used_at_callers = false;
    assert!(matches!(
      classify_change_signature(&r),
      ChangeSignatureVerdict::ChangeSignatureReady
    ));
  }

  #[test]
  fn held_on_add_param_required_no_default_with_callers() {
    let mut r = req();
    r.change_kind = "add-param".to_string();
    r.adds_required_no_default = true;
    r.call_sites_count = 3;
    assert!(matches!(
      classify_change_signature(&r),
      ChangeSignatureVerdict::ChangeSignatureHeld {
        held_kind: ChangeSignatureHeldKind::AddsRequiredWithoutDefault,
        ..
      }
    ));
  }

  #[test]
  fn add_param_required_no_default_without_callers_skips_required_check() {
    // No callers → adds-required-without-default cannot fire (it's
    // a caller-blast-radius guard). The no-call-sites Held wins.
    let mut r = req();
    r.change_kind = "add-param".to_string();
    r.adds_required_no_default = true;
    r.call_sites_count = 0;
    assert!(matches!(
      classify_change_signature(&r),
      ChangeSignatureVerdict::ChangeSignatureHeld {
        held_kind: ChangeSignatureHeldKind::NoCallSites,
        ..
      }
    ));
  }

  #[test]
  fn held_on_change_return_type_with_public_api() {
    let mut r = req();
    r.change_kind = "change-return-type".to_string();
    r.is_public_api = true;
    assert!(matches!(
      classify_change_signature(&r),
      ChangeSignatureVerdict::ChangeSignatureHeld {
        held_kind: ChangeSignatureHeldKind::ReturnTypeChangeWithPublicApi,
        ..
      }
    ));
  }

  #[test]
  fn ready_on_change_return_type_when_private() {
    let mut r = req();
    r.change_kind = "change-return-type".to_string();
    r.is_public_api = false;
    assert!(matches!(
      classify_change_signature(&r),
      ChangeSignatureVerdict::ChangeSignatureReady
    ));
  }

  #[test]
  fn held_on_crosses_async_boundary() {
    let mut r = req();
    r.crosses_async_boundary = true;
    assert!(matches!(
      classify_change_signature(&r),
      ChangeSignatureVerdict::ChangeSignatureHeld {
        held_kind: ChangeSignatureHeldKind::AsyncBoundaryCrossed,
        ..
      }
    ));
  }

  #[test]
  fn held_when_no_call_sites() {
    let mut r = req();
    r.call_sites_count = 0;
    assert!(matches!(
      classify_change_signature(&r),
      ChangeSignatureVerdict::ChangeSignatureHeld {
        held_kind: ChangeSignatureHeldKind::NoCallSites,
        ..
      }
    ));
  }

  #[test]
  fn held_when_too_many_call_sites() {
    let mut r = req();
    r.call_sites_count = 17;
    assert!(matches!(
      classify_change_signature(&r),
      ChangeSignatureVerdict::ChangeSignatureHeld {
        held_kind: ChangeSignatureHeldKind::TooManyCallSites,
        ..
      }
    ));
  }

  #[test]
  fn ready_at_boundary_of_too_many_call_sites() {
    let mut r = req();
    r.call_sites_count = 16;
    assert!(matches!(
      classify_change_signature(&r),
      ChangeSignatureVerdict::ChangeSignatureReady
    ));
  }

  #[test]
  fn ladder_order_prefers_target_path_over_language() {
    // Both target_path empty AND language unsupported — target_path wins.
    let mut r = req();
    r.target_path = String::new();
    r.language = "cobol".to_string();
    assert!(matches!(
      classify_change_signature(&r),
      ChangeSignatureVerdict::ChangeSignatureHeld {
        held_kind: ChangeSignatureHeldKind::TargetPathInvalid,
        ..
      }
    ));
  }

  #[test]
  fn ladder_order_prefers_invalid_name_over_invalid_change_kind() {
    let mut r = req();
    r.function_name = "1bad".to_string();
    r.change_kind = "nonsense".to_string();
    let v = classify_change_signature(&r);
    assert!(matches!(
      v,
      ChangeSignatureVerdict::ChangeSignatureRejected {
        held_kind: ChangeSignatureHeldKind::InvalidFunctionName,
        ..
      }
    ));
  }

  #[test]
  fn payload_carries_canonical_fields() {
    let c = compute_change_signature_candidate(&req());
    let p = build_change_signature_candidate_payload(&c);
    assert_eq!(p["transform"].as_str(), Some("change-signature"));
    assert_eq!(
      p["owner_law"].as_str(),
      Some("stdlib/lib/gate/code-transform/change-signature.px")
    );
    assert_eq!(p["target_path"].as_str(), Some("src/a.rs"));
    assert_eq!(p["function_name"].as_str(), Some("frob"));
    assert_eq!(p["change_kind"].as_str(), Some("change-param-type"));
    assert_eq!(p["call_sites_count"].as_u64(), Some(3));
    assert_eq!(p["verdict"].as_str(), Some("change-signature-ready"));
    assert_eq!(p["candidate_only"].as_bool(), Some(true));
    assert_eq!(
      p["capability_required"].as_str(),
      Some("EditWithinTargetPaths")
    );
    assert!(p.get("held_kind").is_none());
  }

  #[test]
  fn payload_includes_held_kind_and_reason_for_held() {
    let mut r = req();
    r.crosses_async_boundary = true;
    let c = compute_change_signature_candidate(&r);
    let p = build_change_signature_candidate_payload(&c);
    assert_eq!(p["verdict"].as_str(), Some("change-signature-held"));
    assert_eq!(p["held_kind"].as_str(), Some("async-boundary-crossed"));
    assert!(p["reason"].as_str().unwrap().contains("async"));
  }

  #[test]
  fn artifact_id_is_replay_stable() {
    let c1 = compute_change_signature_candidate(&req());
    let c2 = compute_change_signature_candidate(&req());
    let a1 = build_change_signature_candidate_artifact(&c1, 1000, None);
    let a2 = build_change_signature_candidate_artifact(&c2, 2000, None); // different stored_at_ms
    assert_eq!(a1["id"], a2["id"]);
  }

  #[test]
  fn artifact_family_reflects_verdict() {
    let ready = compute_change_signature_candidate(&req());
    let mut held_req = req();
    held_req.crosses_async_boundary = true;
    let held = compute_change_signature_candidate(&held_req);
    let mut rej_req = req();
    rej_req.function_name = "1bad".to_string();
    let rej = compute_change_signature_candidate(&rej_req);

    let a_ready = build_change_signature_candidate_artifact(&ready, 0, None);
    let a_held = build_change_signature_candidate_artifact(&held, 0, None);
    let a_rej = build_change_signature_candidate_artifact(&rej, 0, None);
    assert_eq!(
      a_ready["artifact_family"].as_str(),
      Some("coding.code-transform.change-signature-ready")
    );
    assert_eq!(
      a_held["artifact_family"].as_str(),
      Some("coding.code-transform.change-signature-held")
    );
    assert_eq!(
      a_rej["artifact_family"].as_str(),
      Some("coding.code-transform.change-signature-rejected")
    );
  }

  #[test]
  fn artifact_carries_repo_snapshot_ref_when_provided() {
    let c = compute_change_signature_candidate(&req());
    let a = build_change_signature_candidate_artifact(&c, 1700000000000, Some("commit-deadbeef"));
    assert_eq!(a["repo_snapshot_ref"].as_str(), Some("commit-deadbeef"));
  }

  #[test]
  fn artifact_owner_law_back_ref_present() {
    let c = compute_change_signature_candidate(&req());
    let a = build_change_signature_candidate_artifact(&c, 0, None);
    let refs: Vec<&str> = a["related_refs"]
      .as_array()
      .unwrap()
      .iter()
      .filter_map(|v| v.as_str())
      .collect();
    assert!(refs
      .iter()
      .any(|r| *r == "owner-law:stdlib/lib/gate/code-transform/change-signature.px"));
  }
}
