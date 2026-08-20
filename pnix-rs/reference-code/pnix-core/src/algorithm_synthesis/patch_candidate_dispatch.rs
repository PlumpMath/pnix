//! Unified patch-candidate dispatcher — the missing "any-transform
//! → patch candidate" substrate primitive.
//!
//! OWNER-LAW (2026-05-12): the algorithm-synthesis chain stops at
//! Stage 4 (parameter resolution → typed request JSON). Stage 5
//! (host CST emit → patch candidate) lives per-transform in
//! `code_transform/*.rs`. This module is the **dispatcher** that
//! routes a resolved request to the right per-transform emitter,
//! returning a uniformly-shaped `coding.generated-patch-
//! candidate` artifact regardless of transform.
//!
//! Substrate-share thesis (lattice category C "흡수" × A "의미처리"):
//! the same NL → judgement → request → patch-candidate pipeline
//! handles every registered transform. The dispatcher is the
//! polymorphism boundary — caller doesn't pick which emitter to
//! call, the transform field does.
//!
//! Currently registered transforms:
//!   - `rename-symbol`         → `compute_rename_patch_candidate_lang_safe`
//!     (per-language safe walker: rust / python / typescript /
//!     javascript / go / pnix — dispatched on `request.language`)
//!   - `add-test-stub`         → `compute_add_test_stub_patch_candidate_rust`
//!   - `remove-unused-import`  → `compute_remove_unused_import_patch_candidate`
//!   - `rename-node-id`        → `compute_rename_node_id_patch_candidate`
//!     (graph-mode `.px` first substrate-share — pnix3d node id refactor)
//!   - `remove-node-id`        → `compute_remove_node_id_patch_candidate`
//!     (graph-mode `.px` strict node removal — fails on edge refs)
//!   - `add-pnix-extern`       → `compute_add_pnix_extern_patch_candidate`
//!   - `add-pnix-node`         → `compute_add_pnix_node_patch_candidate`
//!   - `add-pnix-edge`         → `compute_add_pnix_edge_patch_candidate`
//!     (graph-mode CRUD Create — extern / node / edge append)
//!   - `remove-pnix-edge`      → `compute_remove_pnix_edge_patch_candidate`
//!     (graph-mode CRUD Delete — edge entry by (from, to) match)
//!
//! Unknown transforms return `Held::UnknownTransform` rather than
//! panicking — operators can see the dispatcher refused without
//! tripping the substrate.

use crate::code_transform::add_test_stub::{
  build_add_test_stub_patch_candidate_artifact, compute_add_test_stub_patch_candidate_rust,
  AddTestStubFileInput, AddTestStubRequest,
};
use crate::code_transform::pnix_graph::{
  build_add_pnix_edge_patch_candidate_artifact, build_add_pnix_extern_patch_candidate_artifact,
  build_add_pnix_node_patch_candidate_artifact, build_remove_node_id_patch_candidate_artifact,
  build_remove_pnix_edge_patch_candidate_artifact, build_rename_node_id_patch_candidate_artifact,
  compute_add_pnix_edge_patch_candidate, compute_add_pnix_extern_patch_candidate,
  compute_add_pnix_node_patch_candidate, compute_remove_node_id_patch_candidate,
  compute_remove_pnix_edge_patch_candidate, compute_rename_node_id_patch_candidate,
  AddPnixEdgeRequest, AddPnixExternRequest, AddPnixGraphFileInput, AddPnixNodeRequest,
  RemoveNodeIdFileInput, RemoveNodeIdRequest, RemovePnixEdgeRequest, RenameNodeIdFileInput,
  RenameNodeIdRequest,
};
use crate::code_transform::remove_unused_import::{
  build_remove_unused_import_patch_candidate_artifact,
  compute_remove_unused_import_patch_candidate, RemoveUnusedImportFileInput,
  RemoveUnusedImportRequest,
};
use crate::code_transform::rename_symbol::{
  build_rename_symbol_patch_candidate_artifact, compute_rename_patch_candidate_lang_safe,
  RenameFileInput, RenameRequest,
};

/// One file's path + content. Shared input shape — the dispatcher
/// fans these out to whichever per-transform input type the chosen
/// emitter requires.
#[derive(Debug, Clone, Copy)]
pub struct PatchInputFile<'a> {
  pub path: &'a str,
  pub content: &'a str,
}

/// Reason the dispatcher couldn't produce a patch candidate. None of
/// these are exceptions in the substrate sense — every variant maps
/// to an operator-visible Held / error that the cockpit can render.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchDispatchError {
  /// Transform name not registered with the dispatcher. Caller
  /// should re-check the operation-candidate mapping or extend the
  /// dispatcher to cover the new transform.
  UnknownTransform { transform: String },
  /// Resolved request JSON did not deserialize into the typed
  /// request struct the chosen emitter expects. Surfaces a schema-
  /// drift bug between `parameter_resolution` and the per-transform
  /// request types.
  RequestDeserializeFailed {
    transform: String,
    serde_error: String,
  },
  /// Caller passed zero file inputs but the chosen emitter needs at
  /// least one (every transform handled today needs file bytes).
  MissingFileInputs { transform: String },
  /// Caller passed inputs whose paths don't cover every path
  /// referenced by `request.target_paths` (or the moral equivalent
  /// per transform). Carries the missing path so the operator can
  /// fetch it.
  MissingFileForTargetPath {
    transform: String,
    missing_path: String,
  },
}

impl std::fmt::Display for PatchDispatchError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::UnknownTransform { transform } => {
        write!(f, "patch-candidate dispatcher: unknown transform `{transform}`")
      }
      Self::RequestDeserializeFailed {
        transform,
        serde_error,
      } => write!(
        f,
        "patch-candidate dispatcher: request JSON for `{transform}` did not deserialize: {serde_error}"
      ),
      Self::MissingFileInputs { transform } => write!(
        f,
        "patch-candidate dispatcher: transform `{transform}` needs at least one PatchInputFile"
      ),
      Self::MissingFileForTargetPath {
        transform,
        missing_path,
      } => write!(
        f,
        "patch-candidate dispatcher: transform `{transform}` references `{missing_path}` but no PatchInputFile covers it"
      ),
    }
  }
}

impl std::error::Error for PatchDispatchError {}

impl PatchDispatchError {
  /// Short canonical kind tag for cockpit projection. Stable across
  /// future error variant additions (new variants get new tags).
  pub fn kind_tag(&self) -> &'static str {
    match self {
      Self::UnknownTransform { .. } => "unknown-transform",
      Self::RequestDeserializeFailed { .. } => "request-deserialize-failed",
      Self::MissingFileInputs { .. } => "missing-file-inputs",
      Self::MissingFileForTargetPath { .. } => "missing-file-for-target-path",
    }
  }

  /// Per-variant suggested-action hint for the cockpit. Returns a
  /// short human-readable string operators can act on without
  /// digging into source.
  pub fn suggested_action(&self) -> String {
    match self {
      Self::UnknownTransform { transform } => format!(
        "register `{transform}` in pnix-core::patch_candidate_dispatch::dispatch_patch_candidate_for_request, or revisit the operation-candidate mapping"
      ),
      Self::RequestDeserializeFailed { transform, .. } => format!(
        "re-check parameter_resolution output against the `{transform}` request struct — schema drift between resolver and request type"
      ),
      Self::MissingFileInputs { transform } => format!(
        "caller must supply at least one PatchInputFile for transform `{transform}`"
      ),
      Self::MissingFileForTargetPath { transform, missing_path } => format!(
        "fetch the contents of `{missing_path}` and re-dispatch `{transform}` with that file included"
      ),
    }
  }
}

/// Build a canonical JSON artifact representing a refused dispatch.
/// The cockpit's `patch-dispatch-held` panel renders this; doghouse
/// can store it via `json_to_coding_memory_artifact`.
///
/// OWNER-LAW (2026-05-12): refused dispatches are operator-visible
/// audit events, not silent errors. Every dispatcher refusal can be
/// promoted to a typed `coding.patch-dispatch-held` artifact so:
///   - audit chains know which transforms got refused on a given turn,
///   - operators can review the suggested action in the cockpit,
///   - replay reconstructs the exact dispatch attempt.
///
/// `request_summary` is a 1-3 field projection of the offending
/// request JSON (e.g. `{transform, target_paths}`) — the caller
/// chooses what to include. The dispatcher itself doesn't extract
/// this because the request JSON may not have deserialized.
///
/// Replay-stable id: SHA-256 of (kind_tag + transform +
/// canonical_reason). `stored_at_ms` is envelope metadata only.
pub fn build_patch_dispatch_held_artifact(
  error: &PatchDispatchError,
  stored_at_ms: u64,
  request_summary: Option<serde_json::Value>,
  repo_snapshot_ref: Option<&str>,
) -> serde_json::Value {
  let kind = error.kind_tag();
  let transform = match error {
    PatchDispatchError::UnknownTransform { transform } => transform.clone(),
    PatchDispatchError::RequestDeserializeFailed { transform, .. } => transform.clone(),
    PatchDispatchError::MissingFileInputs { transform } => transform.clone(),
    PatchDispatchError::MissingFileForTargetPath { transform, .. } => transform.clone(),
  };
  let reason = error.to_string();
  let suggested = error.suggested_action();

  let mut payload = serde_json::json!({
    "artifact": "patch-dispatch-held",
    "owner_law": "pnix-core::patch_candidate_dispatch",
    "transform": transform,
    "kind": kind,
    "reason": reason,
    "suggested_action": suggested,
    "candidate_only": true,
    "next_step": "operator-decision-or-resubmit",
  });
  // Per-variant secondary fields — only the ones meaningful for the
  // variant. Cockpit reads them when present, ignores when absent.
  match error {
    PatchDispatchError::RequestDeserializeFailed { serde_error, .. } => {
      payload["serde_error"] = serde_json::Value::String(serde_error.clone());
    }
    PatchDispatchError::MissingFileForTargetPath { missing_path, .. } => {
      payload["missing_path"] = serde_json::Value::String(missing_path.clone());
    }
    _ => {}
  }
  if let Some(summary) = &request_summary {
    payload["request_summary"] = summary.clone();
  }

  // Replay-stable id.
  use pnix_hash::{Digest, Sha256};
  let mut hasher = Sha256::new();
  hasher.update(b"patch-dispatch-held\x1f");
  hasher.update(kind.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(transform.as_bytes());
  hasher.update(b"\x1f");
  hasher.update(reason.as_bytes());
  if let PatchDispatchError::MissingFileForTargetPath { missing_path, .. } = error {
    hasher.update(b"\x1f");
    hasher.update(missing_path.as_bytes());
  }
  let digest = hasher.finalize();
  let prefix = digest
    .iter()
    .take(8)
    .map(|b| format!("{b:02x}"))
    .collect::<String>();
  let id = format!("patch-dispatch-held.{prefix}");

  let mut artifact = serde_json::json!({
    "id": id,
    "artifact_family": "coding.patch-dispatch-held",
    "source_surface": "algorithm-synthesis.patch-candidate-dispatch",
    "stored_at_ms": stored_at_ms,
    "target_paths": serde_json::Value::Array(Vec::new()),
    "command_refs": serde_json::Value::Array(Vec::new()),
    "related_refs": serde_json::json!([
      "owner-law:pnix-core::patch_candidate_dispatch"
    ]),
    "payload": payload,
  });
  if let Some(snap) = repo_snapshot_ref {
    artifact["repo_snapshot_ref"] = serde_json::Value::String(snap.to_string());
  }
  artifact
}

/// Dispatch a resolved request to the right per-transform host CST
/// emitter. Returns the canonical
/// `coding.generated-patch-candidate` artifact JSON.
///
/// `stored_at_ms` and `repo_snapshot_ref` are envelope metadata only
/// — they do NOT affect the artifact's replay-stable id.
pub fn dispatch_patch_candidate_for_request(
  transform: &str,
  request_json: &serde_json::Value,
  inputs: &[PatchInputFile<'_>],
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
) -> Result<serde_json::Value, PatchDispatchError> {
  if inputs.is_empty() {
    return Err(PatchDispatchError::MissingFileInputs {
      transform: transform.to_string(),
    });
  }
  match transform {
    "rename-symbol" => {
      dispatch_rename_symbol(request_json, inputs, stored_at_ms, repo_snapshot_ref)
    }
    "add-test-stub" => {
      dispatch_add_test_stub(request_json, inputs, stored_at_ms, repo_snapshot_ref)
    }
    "remove-unused-import" => {
      dispatch_remove_unused_import(request_json, inputs, stored_at_ms, repo_snapshot_ref)
    }
    "rename-node-id" => {
      dispatch_rename_node_id(request_json, inputs, stored_at_ms, repo_snapshot_ref)
    }
    "remove-node-id" => {
      dispatch_remove_node_id(request_json, inputs, stored_at_ms, repo_snapshot_ref)
    }
    "add-pnix-extern" => {
      dispatch_add_pnix_extern(request_json, inputs, stored_at_ms, repo_snapshot_ref)
    }
    "add-pnix-node" => {
      dispatch_add_pnix_node(request_json, inputs, stored_at_ms, repo_snapshot_ref)
    }
    "add-pnix-edge" => {
      dispatch_add_pnix_edge(request_json, inputs, stored_at_ms, repo_snapshot_ref)
    }
    "remove-pnix-edge" => {
      dispatch_remove_pnix_edge(request_json, inputs, stored_at_ms, repo_snapshot_ref)
    }
    other => Err(PatchDispatchError::UnknownTransform {
      transform: other.to_string(),
    }),
  }
}

fn dispatch_rename_symbol(
  request_json: &serde_json::Value,
  inputs: &[PatchInputFile<'_>],
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
) -> Result<serde_json::Value, PatchDispatchError> {
  let request: RenameRequest = serde_json::from_value(request_json.clone()).map_err(|e| {
    PatchDispatchError::RequestDeserializeFailed {
      transform: "rename-symbol".to_string(),
      serde_error: e.to_string(),
    }
  })?;
  // Every target_path must be covered by an input.
  for tp in &request.target_paths {
    if !inputs.iter().any(|fi| fi.path == tp.as_str()) {
      return Err(PatchDispatchError::MissingFileForTargetPath {
        transform: "rename-symbol".to_string(),
        missing_path: tp.clone(),
      });
    }
  }
  let rename_inputs: Vec<RenameFileInput<'_>> = inputs
    .iter()
    .map(|fi| RenameFileInput {
      path: fi.path,
      content: fi.content,
    })
    .collect();
  let candidate = compute_rename_patch_candidate_lang_safe(&request, &rename_inputs);
  Ok(build_rename_symbol_patch_candidate_artifact(
    &candidate,
    stored_at_ms,
    repo_snapshot_ref,
  ))
}

fn dispatch_remove_unused_import(
  request_json: &serde_json::Value,
  inputs: &[PatchInputFile<'_>],
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
) -> Result<serde_json::Value, PatchDispatchError> {
  let request: RemoveUnusedImportRequest =
    serde_json::from_value(request_json.clone()).map_err(|e| {
      PatchDispatchError::RequestDeserializeFailed {
        transform: "remove-unused-import".to_string(),
        serde_error: e.to_string(),
      }
    })?;
  // remove-unused-import names a single file via `target_path`.
  let file_input = inputs
    .iter()
    .find(|fi| fi.path == request.target_path.as_str())
    .ok_or_else(|| PatchDispatchError::MissingFileForTargetPath {
      transform: "remove-unused-import".to_string(),
      missing_path: request.target_path.clone(),
    })?;
  let rmi_input = RemoveUnusedImportFileInput {
    path: file_input.path,
    content: file_input.content,
  };
  let candidate = compute_remove_unused_import_patch_candidate(&request, &rmi_input);
  Ok(build_remove_unused_import_patch_candidate_artifact(
    &candidate,
    stored_at_ms,
    repo_snapshot_ref,
  ))
}

fn dispatch_remove_pnix_edge(
  request_json: &serde_json::Value,
  inputs: &[PatchInputFile<'_>],
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
) -> Result<serde_json::Value, PatchDispatchError> {
  let request: RemovePnixEdgeRequest =
    serde_json::from_value(request_json.clone()).map_err(|e| {
      PatchDispatchError::RequestDeserializeFailed {
        transform: "remove-pnix-edge".to_string(),
        serde_error: e.to_string(),
      }
    })?;
  let file_input = inputs
    .iter()
    .find(|fi| fi.path == request.target_path.as_str())
    .ok_or_else(|| PatchDispatchError::MissingFileForTargetPath {
      transform: "remove-pnix-edge".to_string(),
      missing_path: request.target_path.clone(),
    })?;
  let graph_input = AddPnixGraphFileInput {
    path: file_input.path,
    content: file_input.content,
  };
  let candidate = compute_remove_pnix_edge_patch_candidate(&request, &graph_input);
  Ok(build_remove_pnix_edge_patch_candidate_artifact(
    &candidate,
    stored_at_ms,
    repo_snapshot_ref,
  ))
}

fn dispatch_add_pnix_extern(
  request_json: &serde_json::Value,
  inputs: &[PatchInputFile<'_>],
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
) -> Result<serde_json::Value, PatchDispatchError> {
  let request: AddPnixExternRequest =
    serde_json::from_value(request_json.clone()).map_err(|e| {
      PatchDispatchError::RequestDeserializeFailed {
        transform: "add-pnix-extern".to_string(),
        serde_error: e.to_string(),
      }
    })?;
  let file_input = inputs
    .iter()
    .find(|fi| fi.path == request.target_path.as_str())
    .ok_or_else(|| PatchDispatchError::MissingFileForTargetPath {
      transform: "add-pnix-extern".to_string(),
      missing_path: request.target_path.clone(),
    })?;
  let graph_input = AddPnixGraphFileInput {
    path: file_input.path,
    content: file_input.content,
  };
  let candidate = compute_add_pnix_extern_patch_candidate(&request, &graph_input);
  Ok(build_add_pnix_extern_patch_candidate_artifact(
    &candidate,
    stored_at_ms,
    repo_snapshot_ref,
  ))
}

fn dispatch_add_pnix_node(
  request_json: &serde_json::Value,
  inputs: &[PatchInputFile<'_>],
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
) -> Result<serde_json::Value, PatchDispatchError> {
  let request: AddPnixNodeRequest = serde_json::from_value(request_json.clone()).map_err(|e| {
    PatchDispatchError::RequestDeserializeFailed {
      transform: "add-pnix-node".to_string(),
      serde_error: e.to_string(),
    }
  })?;
  let file_input = inputs
    .iter()
    .find(|fi| fi.path == request.target_path.as_str())
    .ok_or_else(|| PatchDispatchError::MissingFileForTargetPath {
      transform: "add-pnix-node".to_string(),
      missing_path: request.target_path.clone(),
    })?;
  let graph_input = AddPnixGraphFileInput {
    path: file_input.path,
    content: file_input.content,
  };
  let candidate = compute_add_pnix_node_patch_candidate(&request, &graph_input);
  Ok(build_add_pnix_node_patch_candidate_artifact(
    &candidate,
    stored_at_ms,
    repo_snapshot_ref,
  ))
}

fn dispatch_add_pnix_edge(
  request_json: &serde_json::Value,
  inputs: &[PatchInputFile<'_>],
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
) -> Result<serde_json::Value, PatchDispatchError> {
  let request: AddPnixEdgeRequest = serde_json::from_value(request_json.clone()).map_err(|e| {
    PatchDispatchError::RequestDeserializeFailed {
      transform: "add-pnix-edge".to_string(),
      serde_error: e.to_string(),
    }
  })?;
  let file_input = inputs
    .iter()
    .find(|fi| fi.path == request.target_path.as_str())
    .ok_or_else(|| PatchDispatchError::MissingFileForTargetPath {
      transform: "add-pnix-edge".to_string(),
      missing_path: request.target_path.clone(),
    })?;
  let graph_input = AddPnixGraphFileInput {
    path: file_input.path,
    content: file_input.content,
  };
  let candidate = compute_add_pnix_edge_patch_candidate(&request, &graph_input);
  Ok(build_add_pnix_edge_patch_candidate_artifact(
    &candidate,
    stored_at_ms,
    repo_snapshot_ref,
  ))
}

fn dispatch_remove_node_id(
  request_json: &serde_json::Value,
  inputs: &[PatchInputFile<'_>],
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
) -> Result<serde_json::Value, PatchDispatchError> {
  let request: RemoveNodeIdRequest = serde_json::from_value(request_json.clone()).map_err(|e| {
    PatchDispatchError::RequestDeserializeFailed {
      transform: "remove-node-id".to_string(),
      serde_error: e.to_string(),
    }
  })?;
  let file_input = inputs
    .iter()
    .find(|fi| fi.path == request.target_path.as_str())
    .ok_or_else(|| PatchDispatchError::MissingFileForTargetPath {
      transform: "remove-node-id".to_string(),
      missing_path: request.target_path.clone(),
    })?;
  let graph_input = RemoveNodeIdFileInput {
    path: file_input.path,
    content: file_input.content,
  };
  let candidate = compute_remove_node_id_patch_candidate(&request, &graph_input);
  Ok(build_remove_node_id_patch_candidate_artifact(
    &candidate,
    stored_at_ms,
    repo_snapshot_ref,
  ))
}

fn dispatch_rename_node_id(
  request_json: &serde_json::Value,
  inputs: &[PatchInputFile<'_>],
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
) -> Result<serde_json::Value, PatchDispatchError> {
  let request: RenameNodeIdRequest = serde_json::from_value(request_json.clone()).map_err(|e| {
    PatchDispatchError::RequestDeserializeFailed {
      transform: "rename-node-id".to_string(),
      serde_error: e.to_string(),
    }
  })?;
  // graph-mode rename touches a single graph file via target_path.
  let file_input = inputs
    .iter()
    .find(|fi| fi.path == request.target_path.as_str())
    .ok_or_else(|| PatchDispatchError::MissingFileForTargetPath {
      transform: "rename-node-id".to_string(),
      missing_path: request.target_path.clone(),
    })?;
  let graph_input = RenameNodeIdFileInput {
    path: file_input.path,
    content: file_input.content,
  };
  let candidate = compute_rename_node_id_patch_candidate(&request, &graph_input);
  Ok(build_rename_node_id_patch_candidate_artifact(
    &candidate,
    stored_at_ms,
    repo_snapshot_ref,
  ))
}

fn dispatch_add_test_stub(
  request_json: &serde_json::Value,
  inputs: &[PatchInputFile<'_>],
  stored_at_ms: u64,
  repo_snapshot_ref: Option<&str>,
) -> Result<serde_json::Value, PatchDispatchError> {
  let request: AddTestStubRequest = serde_json::from_value(request_json.clone()).map_err(|e| {
    PatchDispatchError::RequestDeserializeFailed {
      transform: "add-test-stub".to_string(),
      serde_error: e.to_string(),
    }
  })?;
  // add-test-stub touches a single target_module. Look it up among
  // inputs; if absent, return MissingFileForTargetPath.
  let file_input = inputs
    .iter()
    .find(|fi| fi.path == request.target_module.as_str())
    .ok_or_else(|| PatchDispatchError::MissingFileForTargetPath {
      transform: "add-test-stub".to_string(),
      missing_path: request.target_module.clone(),
    })?;
  let stub_input = AddTestStubFileInput {
    path: file_input.path,
    content: file_input.content,
  };
  let candidate = compute_add_test_stub_patch_candidate_rust(&request, &stub_input);
  Ok(build_add_test_stub_patch_candidate_artifact(
    &candidate,
    stored_at_ms,
    repo_snapshot_ref,
  ))
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  // ─── unknown / missing-input errors ─────────────────────────

  #[test]
  fn dispatch_unknown_transform_returns_held() {
    let r = dispatch_patch_candidate_for_request(
      "what-is-this",
      &json!({}),
      &[PatchInputFile {
        path: "x.rs",
        content: "",
      }],
      0,
      None,
    );
    match r {
      Err(PatchDispatchError::UnknownTransform { transform }) => {
        assert_eq!(transform, "what-is-this");
      }
      other => panic!("expected UnknownTransform, got {other:?}"),
    }
  }

  #[test]
  fn dispatch_missing_inputs_returns_held() {
    let r = dispatch_patch_candidate_for_request("rename-symbol", &json!({}), &[], 0, None);
    match r {
      Err(PatchDispatchError::MissingFileInputs { transform }) => {
        assert_eq!(transform, "rename-symbol");
      }
      other => panic!("expected MissingFileInputs, got {other:?}"),
    }
  }

  // ─── rename-symbol dispatch ───────────────────────────────────

  #[test]
  fn dispatch_rename_symbol_produces_canonical_artifact() {
    let request_json = json!({
      "old_name": "foo",
      "new_name": "bar",
      "language": "rust",
      "scope": "local-target-paths",
      "target_paths": ["src/a.rs"]
    });
    let inputs = [PatchInputFile {
      path: "src/a.rs",
      content: "fn foo() { foo() }\n",
    }];
    let art = dispatch_patch_candidate_for_request(
      "rename-symbol",
      &request_json,
      &inputs,
      1_700_000_000_000,
      Some("commit-abc"),
    )
    .expect("dispatch ok");
    assert_eq!(
      art["artifact_family"].as_str(),
      Some("coding.generated-patch-candidate")
    );
    assert_eq!(
      art["source_surface"].as_str(),
      Some("code-transform.rename-symbol")
    );
    assert_eq!(art["payload"]["transform"].as_str(), Some("rename-symbol"));
    assert_eq!(art["payload"]["verdict"].as_str(), Some("rename-ready"));
    assert_eq!(art["payload"]["old_name"].as_str(), Some("foo"));
    assert_eq!(art["payload"]["new_name"].as_str(), Some("bar"));
    assert_eq!(art["repo_snapshot_ref"].as_str(), Some("commit-abc"));
  }

  #[test]
  fn dispatch_rename_symbol_missing_target_path_in_inputs_returns_held() {
    let request_json = json!({
      "old_name": "foo",
      "new_name": "bar",
      "language": "rust",
      "scope": "local-target-paths",
      "target_paths": ["src/a.rs", "src/b.rs"]
    });
    let inputs = [PatchInputFile {
      path: "src/a.rs",
      content: "fn foo() {}\n",
    }]; // src/b.rs missing
    let r = dispatch_patch_candidate_for_request("rename-symbol", &request_json, &inputs, 0, None);
    match r {
      Err(PatchDispatchError::MissingFileForTargetPath {
        transform,
        missing_path,
      }) => {
        assert_eq!(transform, "rename-symbol");
        assert_eq!(missing_path, "src/b.rs");
      }
      other => panic!("expected MissingFileForTargetPath, got {other:?}"),
    }
  }

  #[test]
  fn dispatch_rename_symbol_bad_request_json_returns_held() {
    // `old_name` missing → deserialize fails.
    let request_json = json!({
      "new_name": "bar",
      "language": "rust",
      "scope": "local-target-paths",
      "target_paths": ["src/a.rs"]
    });
    let inputs = [PatchInputFile {
      path: "src/a.rs",
      content: "fn foo() {}\n",
    }];
    let r = dispatch_patch_candidate_for_request("rename-symbol", &request_json, &inputs, 0, None);
    match r {
      Err(PatchDispatchError::RequestDeserializeFailed {
        transform,
        serde_error,
      }) => {
        assert_eq!(transform, "rename-symbol");
        assert!(serde_error.contains("old_name") || serde_error.contains("missing field"));
      }
      other => panic!("expected RequestDeserializeFailed, got {other:?}"),
    }
  }

  // ─── add-test-stub dispatch ──────────────────────────────────

  #[test]
  fn dispatch_add_test_stub_produces_canonical_artifact() {
    let request_json = json!({
      "target_module": "src/lib.rs",
      "test_name": "happy_path",
      "language": "rust",
      "intent": "checks the happy path",
      "place": null
    });
    let inputs = [PatchInputFile {
      path: "src/lib.rs",
      content: "pub fn add(a: i32, b: i32) -> i32 { a + b }\n",
    }];
    let art =
      dispatch_patch_candidate_for_request("add-test-stub", &request_json, &inputs, 0, None)
        .expect("dispatch ok");
    assert_eq!(
      art["artifact_family"].as_str(),
      Some("coding.generated-patch-candidate"),
      "must emit into SHARED family (substrate-share with rename-symbol)"
    );
    assert_eq!(
      art["source_surface"].as_str(),
      Some("code-transform.add-test-stub")
    );
    assert_eq!(art["payload"]["transform"].as_str(), Some("add-test-stub"));
    assert_eq!(
      art["payload"]["verdict"].as_str(),
      Some("add-test-stub-ready")
    );
    assert_eq!(
      art["payload"]["resolved_place"].as_str(),
      Some("inline-cfg-test")
    );
    let fps = art["payload"]["file_patches"]
      .as_array()
      .expect("file_patches array");
    assert_eq!(fps.len(), 1);
    assert_eq!(fps[0]["path"].as_str(), Some("src/lib.rs"));
  }

  #[test]
  fn dispatch_add_test_stub_missing_target_module_in_inputs_returns_held() {
    let request_json = json!({
      "target_module": "src/lib.rs",
      "test_name": "ok",
      "language": "rust",
      "intent": "",
      "place": null
    });
    let inputs = [PatchInputFile {
      path: "src/other.rs",
      content: "",
    }];
    let r = dispatch_patch_candidate_for_request("add-test-stub", &request_json, &inputs, 0, None);
    match r {
      Err(PatchDispatchError::MissingFileForTargetPath {
        transform,
        missing_path,
      }) => {
        assert_eq!(transform, "add-test-stub");
        assert_eq!(missing_path, "src/lib.rs");
      }
      other => panic!("expected MissingFileForTargetPath, got {other:?}"),
    }
  }

  #[test]
  fn dispatch_add_test_stub_bad_request_json_returns_held() {
    // Missing `test_name`.
    let request_json = json!({
      "target_module": "src/lib.rs",
      "language": "rust",
      "intent": "",
      "place": null
    });
    let inputs = [PatchInputFile {
      path: "src/lib.rs",
      content: "fn x() {}\n",
    }];
    let r = dispatch_patch_candidate_for_request("add-test-stub", &request_json, &inputs, 0, None);
    match r {
      Err(PatchDispatchError::RequestDeserializeFailed { transform, .. }) => {
        assert_eq!(transform, "add-test-stub");
      }
      other => panic!("expected RequestDeserializeFailed, got {other:?}"),
    }
  }

  // ─── dispatcher language-aware rename ────────────────────────
  //
  // The dispatcher's rename-symbol arm now routes through
  // `compute_rename_patch_candidate_lang_safe`, which picks the
  // per-language safe walker based on `request.language`. These
  // tests pin that routing — same dispatcher call, different
  // language → different lexer skips → different edit count.

  #[test]
  fn dispatch_rename_pnix_routes_through_pnix_safe_lexer() {
    // Pnix `.px` source with the symbol `foo` mentioned in:
    //   - a let-binding declaration (real edit site)
    //   - a `#` line comment (must NOT be edited)
    //   - a `"..."` string body (must NOT be edited)
    //   - a body reference (real edit site)
    let request_json = json!({
      "old_name": "foo",
      "new_name": "bar",
      "language": "pnix",
      "scope": "local-target-paths",
      "target_paths": ["stdlib/lib/example.px"]
    });
    let src = "\
let
  foo = 1;
  # don't rename inside the comment: foo
  msg = \"don't rename inside string: foo\";
in foo + foo
";
    let inputs = [PatchInputFile {
      path: "stdlib/lib/example.px",
      content: src,
    }];
    let art =
      dispatch_patch_candidate_for_request("rename-symbol", &request_json, &inputs, 0, None)
        .expect("dispatch ok");
    let edits = art["payload"]["edits"].as_array().expect("edits array");
    // 3 real edit sites: binding + 2 in `in foo + foo`.
    assert_eq!(
      edits.len(),
      3,
      "pnix-safe lexer must filter comment + string occurrences; got edits: {edits:?}"
    );
    let diff = art["payload"]["unified_diff"].as_str().unwrap();
    assert!(diff.contains("-  foo = 1;"));
    assert!(diff.contains("+  bar = 1;"));
    // Comment / string lines unchanged → not in diff.
    assert!(!diff.contains("-  # don't"));
    assert!(!diff.contains("-  msg ="));
  }

  #[test]
  fn dispatch_rename_python_routes_through_python_safe_lexer() {
    // Backwards-compat proof: python NL goes through python-safe
    // walker, which skips `#` comments and `"..."` / `'...'` strings.
    let request_json = json!({
      "old_name": "foo",
      "new_name": "renamed",
      "language": "python",
      "scope": "local-target-paths",
      "target_paths": ["src/main.py"]
    });
    let src = "\
foo = 1
# foo in comment
bar = \"foo in string\"
baz = foo
";
    let inputs = [PatchInputFile {
      path: "src/main.py",
      content: src,
    }];
    let art =
      dispatch_patch_candidate_for_request("rename-symbol", &request_json, &inputs, 0, None)
        .expect("dispatch ok");
    let edits = art["payload"]["edits"].as_array().expect("edits");
    // 2 real edit sites: line 1 + line 4 (baz = foo).
    assert_eq!(edits.len(), 2);
  }

  #[test]
  fn dispatch_rename_rust_still_works_after_lang_aware_switch() {
    // The dispatcher used to hard-call `compute_rename_patch_candidate_rust_safe`;
    // after switching to the lang-aware variant the rust path must
    // still produce the same shape.
    let request_json = json!({
      "old_name": "foo",
      "new_name": "bar",
      "language": "rust",
      "scope": "local-target-paths",
      "target_paths": ["src/a.rs"]
    });
    let inputs = [PatchInputFile {
      path: "src/a.rs",
      content: "fn foo() { foo() }\n// foo in comment\n",
    }];
    let art =
      dispatch_patch_candidate_for_request("rename-symbol", &request_json, &inputs, 0, None)
        .expect("dispatch ok");
    assert_eq!(art["payload"]["verdict"].as_str(), Some("rename-ready"));
    let edits = art["payload"]["edits"].as_array().expect("edits");
    // 2 real edit sites (fn name + call); 1 in comment filtered.
    assert_eq!(edits.len(), 2);
  }

  // ─── remove-unused-import dispatch ───────────────────────────

  #[test]
  fn dispatch_remove_unused_import_produces_canonical_artifact() {
    // Python source with one unused import. RemoveUnusedImport
    // identifies `os` as unused (only `sys` is referenced).
    let source = "import os\nimport sys\nprint(sys.argv)\n";
    let request_json = json!({
      "target_path": "src/main.py",
      "language": "python",
      "scope": "single-file",
      "candidate_imports": [
        {
          "module": "os",
          "alias": null,
          "import_line": 1
        }
      ]
    });
    let inputs = [PatchInputFile {
      path: "src/main.py",
      content: source,
    }];
    let art =
      dispatch_patch_candidate_for_request("remove-unused-import", &request_json, &inputs, 0, None)
        .expect("dispatch ok");
    // OWNER-LAW (2026-05-12): remove-unused-import emits into a
    // verdict-suffixed family by historical convention
    // (`coding.code-transform.remove-unused-import-{ready,held,
    // rejected}`), distinct from rename-symbol + add-test-stub which
    // both use `coding.generated-patch-candidate`. The dispatcher
    // is neutral on family naming — it's a delegation seam, not a
    // family-name unifier. Substrate-share is asserted at the
    // payload-pivot layer (next test), not at the family-name layer.
    assert_eq!(
      art["artifact_family"].as_str(),
      Some("coding.code-transform.remove-unused-import-ready")
    );
    assert_eq!(
      art["source_surface"].as_str(),
      Some("code-transform.remove-unused-import")
    );
    assert_eq!(
      art["payload"]["transform"].as_str(),
      Some("remove-unused-import")
    );
    assert_eq!(
      art["payload"]["verdict"].as_str(),
      Some("remove-unused-import-ready")
    );
    assert_eq!(art["payload"]["language"].as_str(), Some("python"));
  }

  #[test]
  fn dispatch_remove_unused_import_missing_target_path_in_inputs_returns_held() {
    let request_json = json!({
      "target_path": "src/main.py",
      "language": "python",
      "scope": "single-file",
      "candidate_imports": []
    });
    let inputs = [PatchInputFile {
      path: "src/other.py",
      content: "",
    }];
    let r =
      dispatch_patch_candidate_for_request("remove-unused-import", &request_json, &inputs, 0, None);
    match r {
      Err(PatchDispatchError::MissingFileForTargetPath {
        transform,
        missing_path,
      }) => {
        assert_eq!(transform, "remove-unused-import");
        assert_eq!(missing_path, "src/main.py");
      }
      other => panic!("expected MissingFileForTargetPath, got {other:?}"),
    }
  }

  #[test]
  fn dispatch_remove_unused_import_bad_request_json_returns_held() {
    // Missing required `target_path`.
    let request_json = json!({
      "language": "python",
      "scope": "single-file",
      "candidate_imports": []
    });
    let inputs = [PatchInputFile {
      path: "src/main.py",
      content: "import os\n",
    }];
    let r =
      dispatch_patch_candidate_for_request("remove-unused-import", &request_json, &inputs, 0, None);
    match r {
      Err(PatchDispatchError::RequestDeserializeFailed { transform, .. }) => {
        assert_eq!(transform, "remove-unused-import");
      }
      other => panic!("expected RequestDeserializeFailed, got {other:?}"),
    }
  }

  // ─── rename-node-id dispatch (D-8) ───────────────────────────

  #[test]
  fn dispatch_rename_node_id_produces_canonical_artifact() {
    let request_json = json!({
      "target_path": "examples/g.px",
      "old_name": "sum",
      "new_name": "total"
    });
    let src = r#"{
  externs = [ { name = "builtins.add"; } ];
  nodes = [
    { name = "sum"; uses = "builtins.add"; }
  ];
  edges = [
    { from = { input = "a"; }; to = { node = "sum"; port = "lhs"; }; }
  ];
}
"#;
    let inputs = [PatchInputFile {
      path: "examples/g.px",
      content: src,
    }];
    let art =
      dispatch_patch_candidate_for_request("rename-node-id", &request_json, &inputs, 0, None)
        .expect("dispatch ok");
    assert_eq!(
      art["artifact_family"].as_str(),
      Some("coding.generated-patch-candidate"),
      "substrate-share: shared family with rename-symbol + add-test-stub"
    );
    assert_eq!(
      art["source_surface"].as_str(),
      Some("code-transform.rename-node-id")
    );
    assert_eq!(art["payload"]["transform"].as_str(), Some("rename-node-id"));
    assert_eq!(
      art["payload"]["verdict"].as_str(),
      Some("rename-node-id-ready")
    );
    assert_eq!(art["payload"]["old_name"].as_str(), Some("sum"));
    assert_eq!(art["payload"]["new_name"].as_str(), Some("total"));
    let diff = art["payload"]["unified_diff"].as_str().unwrap();
    assert!(diff.contains("-    { name = \"sum\""));
    assert!(diff.contains("+    { name = \"total\""));
  }

  #[test]
  fn dispatch_rename_node_id_missing_target_path_in_inputs_returns_held() {
    let request_json = json!({
      "target_path": "examples/g.px",
      "old_name": "sum",
      "new_name": "total"
    });
    let inputs = [PatchInputFile {
      path: "examples/wrong.px",
      content: "",
    }];
    let r = dispatch_patch_candidate_for_request("rename-node-id", &request_json, &inputs, 0, None);
    match r {
      Err(PatchDispatchError::MissingFileForTargetPath {
        transform,
        missing_path,
      }) => {
        assert_eq!(transform, "rename-node-id");
        assert_eq!(missing_path, "examples/g.px");
      }
      other => panic!("expected MissingFileForTargetPath, got {other:?}"),
    }
  }

  #[test]
  fn dispatch_rename_node_id_bad_request_json_returns_held() {
    // Missing required `new_name`.
    let request_json = json!({
      "target_path": "examples/g.px",
      "old_name": "sum"
    });
    let inputs = [PatchInputFile {
      path: "examples/g.px",
      content: "{}",
    }];
    let r = dispatch_patch_candidate_for_request("rename-node-id", &request_json, &inputs, 0, None);
    match r {
      Err(PatchDispatchError::RequestDeserializeFailed { transform, .. }) => {
        assert_eq!(transform, "rename-node-id");
      }
      other => panic!("expected RequestDeserializeFailed, got {other:?}"),
    }
  }

  #[test]
  fn dispatch_rename_node_id_held_node_not_found_still_emits_artifact() {
    // Operator's request goes through; the file just doesn't have
    // that node. Dispatcher returns Ok(artifact) with Held verdict
    // — the Held is operator-visible in the cockpit, not a
    // dispatcher-level error.
    let request_json = json!({
      "target_path": "examples/g.px",
      "old_name": "no_such",
      "new_name": "y"
    });
    let src = r#"{ nodes = [ { name = "x"; uses = "u"; } ]; }
"#;
    let inputs = [PatchInputFile {
      path: "examples/g.px",
      content: src,
    }];
    let art =
      dispatch_patch_candidate_for_request("rename-node-id", &request_json, &inputs, 0, None)
        .expect("dispatcher itself succeeds");
    assert_eq!(
      art["payload"]["verdict"].as_str(),
      Some("rename-node-id-held")
    );
    assert_eq!(art["payload"]["held_kind"].as_str(), Some("node-not-found"));
  }

  // ─── remove-node-id dispatch (D-12) ──────────────────────────

  #[test]
  fn dispatch_remove_node_id_produces_canonical_artifact() {
    let request_json = json!({
      "target_path": "examples/g.px",
      "name": "orphan"
    });
    let src = r#"{
  nodes = [
    { name = "kept"; uses = "x"; }
    { name = "orphan"; uses = "x"; }
  ];
}
"#;
    let inputs = [PatchInputFile {
      path: "examples/g.px",
      content: src,
    }];
    let art =
      dispatch_patch_candidate_for_request("remove-node-id", &request_json, &inputs, 0, None)
        .expect("dispatch ok");
    assert_eq!(
      art["artifact_family"].as_str(),
      Some("coding.generated-patch-candidate")
    );
    assert_eq!(art["payload"]["transform"].as_str(), Some("remove-node-id"));
    assert_eq!(
      art["payload"]["verdict"].as_str(),
      Some("remove-node-id-ready")
    );
    let diff = art["payload"]["unified_diff"].as_str().unwrap();
    assert!(diff.contains("-    { name = \"orphan\""));
  }

  #[test]
  fn dispatch_remove_node_id_strict_refuse_emits_held_with_ref_count() {
    let request_json = json!({
      "target_path": "examples/g.px",
      "name": "used"
    });
    let src = r#"{
  nodes = [
    { name = "used"; uses = "x"; }
  ];
  edges = [
    { from = { input = "a"; }; to = { node = "used"; port = "lhs"; }; }
    { from = { input = "b"; }; to = { node = "used"; port = "rhs"; }; }
  ];
}
"#;
    let inputs = [PatchInputFile {
      path: "examples/g.px",
      content: src,
    }];
    let art =
      dispatch_patch_candidate_for_request("remove-node-id", &request_json, &inputs, 0, None)
        .expect("dispatcher succeeds even when carrier Held");
    assert_eq!(
      art["payload"]["verdict"].as_str(),
      Some("remove-node-id-held")
    );
    assert_eq!(
      art["payload"]["held_kind"].as_str(),
      Some("still-referenced-by-edges")
    );
    assert_eq!(art["payload"]["edges_ref_count"].as_u64(), Some(2));
  }

  #[test]
  fn dispatch_remove_node_id_missing_target_path_in_inputs_returns_held() {
    let request_json = json!({
      "target_path": "examples/g.px",
      "name": "x"
    });
    let inputs = [PatchInputFile {
      path: "examples/wrong.px",
      content: "",
    }];
    let r = dispatch_patch_candidate_for_request("remove-node-id", &request_json, &inputs, 0, None);
    match r {
      Err(PatchDispatchError::MissingFileForTargetPath {
        transform,
        missing_path,
      }) => {
        assert_eq!(transform, "remove-node-id");
        assert_eq!(missing_path, "examples/g.px");
      }
      other => panic!("expected MissingFileForTargetPath, got {other:?}"),
    }
  }

  // ─── remove-pnix-edge dispatch (D-18) ─────────────────────────

  #[test]
  fn dispatch_remove_pnix_edge_produces_canonical_artifact() {
    let request_json = json!({
      "target_path": "examples/g.px",
      "from": { "kind": "input", "name": "a" },
      "to":   { "kind": "node",  "name": "sum", "port": "lhs" }
    });
    let src = r#"{
  nodes = [
    { name = "sum"; uses = "u"; }
  ];
  edges = [
    { from = { input = "a"; }; to = { node = "sum"; port = "lhs"; }; }
    { from = { input = "b"; }; to = { node = "sum"; port = "rhs"; }; }
  ];
}
"#;
    let inputs = [PatchInputFile {
      path: "examples/g.px",
      content: src,
    }];
    let art =
      dispatch_patch_candidate_for_request("remove-pnix-edge", &request_json, &inputs, 0, None)
        .expect("dispatch ok");
    assert_eq!(
      art["payload"]["transform"].as_str(),
      Some("remove-pnix-edge")
    );
    assert_eq!(
      art["payload"]["verdict"].as_str(),
      Some("remove-pnix-edge-ready")
    );
    assert_eq!(art["payload"]["edges_removed"].as_u64(), Some(1));
    let diff = art["payload"]["unified_diff"].as_str().unwrap();
    assert!(diff.contains("input = \"a\""));
  }

  #[test]
  fn dispatch_remove_pnix_edge_held_on_edge_not_found() {
    let request_json = json!({
      "target_path": "examples/g.px",
      "from": { "kind": "input", "name": "no_such" },
      "to":   { "kind": "node",  "name": "sum" }
    });
    let src = r#"{
  edges = [
    { from = { input = "a"; }; to = { node = "sum"; port = "lhs"; }; }
  ];
  nodes = [ { name = "sum"; uses = "u"; } ];
}
"#;
    let inputs = [PatchInputFile {
      path: "examples/g.px",
      content: src,
    }];
    let art =
      dispatch_patch_candidate_for_request("remove-pnix-edge", &request_json, &inputs, 0, None)
        .expect("dispatcher itself succeeds");
    assert_eq!(
      art["payload"]["verdict"].as_str(),
      Some("remove-pnix-edge-held")
    );
    assert_eq!(art["payload"]["held_kind"].as_str(), Some("edge-not-found"));
  }

  #[test]
  fn dispatch_remove_pnix_edge_bad_request_returns_dispatch_held() {
    // Missing `from` → request deserialize fails.
    let request_json = json!({
      "target_path": "examples/g.px",
      "to": { "kind": "node", "name": "x" }
    });
    let inputs = [PatchInputFile {
      path: "examples/g.px",
      content: "",
    }];
    let r =
      dispatch_patch_candidate_for_request("remove-pnix-edge", &request_json, &inputs, 0, None);
    match r {
      Err(PatchDispatchError::RequestDeserializeFailed { transform, .. }) => {
        assert_eq!(transform, "remove-pnix-edge");
      }
      other => panic!("expected RequestDeserializeFailed, got {other:?}"),
    }
  }

  // ─── remove-node-id cascade dispatch (D-14) ───────────────────

  #[test]
  fn dispatch_remove_node_id_cascade_drops_node_and_edges() {
    let request_json = json!({
      "target_path": "examples/g.px",
      "name": "x",
      "cascade": true
    });
    let src = r#"{
  nodes = [
    { name = "x"; uses = "u"; }
    { name = "y"; uses = "u"; }
  ];
  edges = [
    { from = { input = "a"; }; to = { node = "x"; port = "lhs"; }; }
    { from = { input = "b"; }; to = { node = "x"; port = "rhs"; }; }
    { from = { input = "c"; }; to = { node = "y"; port = "lhs"; }; }
  ];
}
"#;
    let inputs = [PatchInputFile {
      path: "examples/g.px",
      content: src,
    }];
    let art =
      dispatch_patch_candidate_for_request("remove-node-id", &request_json, &inputs, 0, None)
        .expect("dispatch ok");
    let payload = &art["payload"];
    assert_eq!(payload["verdict"].as_str(), Some("remove-node-id-ready"));
    assert_eq!(payload["cascade"].as_bool(), Some(true));
    assert_eq!(payload["cascade_edges_removed"].as_u64(), Some(2));
    // The y-related edge (input "c") must survive — confirm via
    // file_patches' after_sha256 vs before_sha256 differing, AND
    // the diff renderer's `+` (after) side preserving the `c` edge.
    let diff = payload["unified_diff"].as_str().unwrap();
    // After side keeps `c` edge (`+    { from = { input = "c"`).
    assert!(
      diff.contains("+    { from = { input = \"c\""),
      "y-edge (input c) must be preserved in the after-content; diff was:\n{diff}"
    );
    // After side does NOT keep removed `x`-edges.
    let after_section: Vec<&str> = diff.lines().filter(|l| l.starts_with('+')).collect();
    let after_blob = after_section.join("\n");
    assert!(
      !after_blob.contains("node = \"x\""),
      "removed x-node refs must NOT appear in the after side; got:\n{after_blob}"
    );
  }

  #[test]
  fn dispatch_remove_node_id_strict_default_when_cascade_field_omitted() {
    // No `cascade` field → defaults to false (strict) → Held with
    // edges_ref_count.
    let request_json = json!({
      "target_path": "examples/g.px",
      "name": "used"
    });
    let src = r#"{
  nodes = [ { name = "used"; uses = "u"; } ];
  edges = [
    { from = { input = "a"; }; to = { node = "used"; port = "lhs"; }; }
  ];
}
"#;
    let inputs = [PatchInputFile {
      path: "examples/g.px",
      content: src,
    }];
    let art =
      dispatch_patch_candidate_for_request("remove-node-id", &request_json, &inputs, 0, None)
        .expect("dispatch ok");
    let payload = &art["payload"];
    assert_eq!(payload["verdict"].as_str(), Some("remove-node-id-held"));
    assert_eq!(payload["cascade"].as_bool(), Some(false));
    assert_eq!(payload["edges_ref_count"].as_u64(), Some(1));
  }

  // ─── add-pnix-* dispatch (D-13) ──────────────────────────────

  const D13_DISPATCH_SRC: &str = r#"{
  externs = [
    { name = "builtins.add"; }
  ];
  nodes = [
    { name = "sum"; uses = "builtins.add"; }
  ];
  edges = [
    { from = { input = "a"; }; to = { node = "sum"; port = "lhs"; }; }
  ];
}
"#;

  #[test]
  fn dispatch_add_pnix_extern_produces_canonical_artifact() {
    let request_json = json!({
      "target_path": "examples/g.px",
      "entry_text": "name = \"py.sub\""
    });
    let inputs = [PatchInputFile {
      path: "examples/g.px",
      content: D13_DISPATCH_SRC,
    }];
    let art =
      dispatch_patch_candidate_for_request("add-pnix-extern", &request_json, &inputs, 0, None)
        .expect("dispatch ok");
    assert_eq!(
      art["artifact_family"].as_str(),
      Some("coding.generated-patch-candidate")
    );
    assert_eq!(
      art["payload"]["transform"].as_str(),
      Some("add-pnix-extern")
    );
    assert_eq!(
      art["payload"]["verdict"].as_str(),
      Some("add-pnix-extern-ready")
    );
  }

  #[test]
  fn dispatch_add_pnix_node_produces_canonical_artifact() {
    let request_json = json!({
      "target_path": "examples/g.px",
      "entry_text": "name = \"second\"; uses = \"builtins.add\""
    });
    let inputs = [PatchInputFile {
      path: "examples/g.px",
      content: D13_DISPATCH_SRC,
    }];
    let art =
      dispatch_patch_candidate_for_request("add-pnix-node", &request_json, &inputs, 0, None)
        .expect("dispatch ok");
    assert_eq!(art["payload"]["transform"].as_str(), Some("add-pnix-node"));
    assert_eq!(
      art["payload"]["verdict"].as_str(),
      Some("add-pnix-node-ready")
    );
    let diff = art["payload"]["unified_diff"].as_str().unwrap();
    assert!(diff.contains("\"second\""));
  }

  #[test]
  fn dispatch_add_pnix_edge_produces_canonical_artifact() {
    let request_json = json!({
      "target_path": "examples/g.px",
      "entry_text": "from = { input = \"a\"; }; to = { node = \"sum\"; port = \"rhs\"; }"
    });
    let inputs = [PatchInputFile {
      path: "examples/g.px",
      content: D13_DISPATCH_SRC,
    }];
    let art =
      dispatch_patch_candidate_for_request("add-pnix-edge", &request_json, &inputs, 0, None)
        .expect("dispatch ok");
    assert_eq!(art["payload"]["transform"].as_str(), Some("add-pnix-edge"));
    assert_eq!(
      art["payload"]["verdict"].as_str(),
      Some("add-pnix-edge-ready")
    );
  }

  #[test]
  fn dispatch_add_pnix_extern_held_for_missing_section() {
    let request_json = json!({
      "target_path": "g.px",
      "entry_text": "name = \"x\""
    });
    let inputs = [PatchInputFile {
      path: "g.px",
      content: "{\n  nodes = [];\n}\n",
    }];
    let art =
      dispatch_patch_candidate_for_request("add-pnix-extern", &request_json, &inputs, 0, None)
        .expect("dispatcher itself succeeds");
    assert_eq!(
      art["payload"]["held_kind"].as_str(),
      Some("no-section-in-graph")
    );
  }

  // ─── substrate-share at the payload-pivot layer ─────────────
  //
  // The dispatcher is a delegation seam, not a family-name unifier.
  // What it DOES guarantee: every dispatched artifact carries
  // `payload.transform` + `payload.verdict` + `payload.candidate_only`
  // — the canonical cockpit-pivot fields. The patch-candidate panel
  // pivots on `payload.transform` to pick per-transform field labels.
  //
  // Family-name conventions vary by historical artifact wiring:
  //   - `coding.generated-patch-candidate` for rename-symbol +
  //     add-test-stub (newer pattern: verdict in payload, not family).
  //   - `coding.code-transform.remove-unused-import-{ready,held,
  //     rejected}` for remove-unused-import (older pattern: family
  //     name encodes verdict).
  //
  // Both shapes share the payload-pivot canon — that's the seam
  // cockpit consumers depend on.

  #[test]
  fn three_transforms_share_payload_pivot_canon_via_dispatcher() {
    let rename = dispatch_patch_candidate_for_request(
      "rename-symbol",
      &json!({
        "old_name": "foo",
        "new_name": "bar",
        "language": "rust",
        "scope": "local-target-paths",
        "target_paths": ["src/a.rs"]
      }),
      &[PatchInputFile {
        path: "src/a.rs",
        content: "fn foo() {}\n",
      }],
      0,
      None,
    )
    .expect("rename");
    let stub = dispatch_patch_candidate_for_request(
      "add-test-stub",
      &json!({
        "target_module": "src/lib.rs",
        "test_name": "ok",
        "language": "rust",
        "intent": "",
        "place": null
      }),
      &[PatchInputFile {
        path: "src/lib.rs",
        content: "fn x() {}\n",
      }],
      0,
      None,
    )
    .expect("stub");
    let rmi = dispatch_patch_candidate_for_request(
      "remove-unused-import",
      &json!({
        "target_path": "src/main.py",
        "language": "python",
        "scope": "single-file",
        "candidate_imports": [{"module": "os", "alias": null, "import_line": 1}]
      }),
      &[PatchInputFile {
        path: "src/main.py",
        content: "import os\nimport sys\nprint(sys.argv)\n",
      }],
      0,
      None,
    )
    .expect("rmi");

    // ── Payload-pivot canon: every dispatched artifact carries the
    // ── same canonical pivot fields, regardless of family.
    for (label, art) in [
      ("rename-symbol", &rename),
      ("add-test-stub", &stub),
      ("remove-unused-import", &rmi),
    ] {
      let payload = &art["payload"];
      assert!(
        payload["transform"].is_string(),
        "{label} must carry payload.transform"
      );
      assert!(
        payload["verdict"].is_string(),
        "{label} must carry payload.verdict"
      );
      assert_eq!(
        payload["candidate_only"].as_bool(),
        Some(true),
        "{label} must mark candidate_only=true"
      );
      // Owner-law ref must be present for audit traceability.
      let related = art["related_refs"].as_array().expect("related_refs array");
      assert!(
        related
          .iter()
          .any(|v| v.as_str().map_or(false, |s| s.starts_with("owner-law:"))),
        "{label} must carry an owner-law ref"
      );
    }

    // ── Three distinct payload.transform values — identity isn't
    // ── collapsed across transforms.
    let transforms: std::collections::BTreeSet<&str> = [
      rename["payload"]["transform"].as_str().unwrap(),
      stub["payload"]["transform"].as_str().unwrap(),
      rmi["payload"]["transform"].as_str().unwrap(),
    ]
    .into_iter()
    .collect();
    assert_eq!(transforms.len(), 3);

    // ── Three distinct ids and source_surfaces.
    let ids: std::collections::BTreeSet<&str> = [
      rename["id"].as_str().unwrap(),
      stub["id"].as_str().unwrap(),
      rmi["id"].as_str().unwrap(),
    ]
    .into_iter()
    .collect();
    assert_eq!(ids.len(), 3);
    let surfaces: std::collections::BTreeSet<&str> = [
      rename["source_surface"].as_str().unwrap(),
      stub["source_surface"].as_str().unwrap(),
      rmi["source_surface"].as_str().unwrap(),
    ]
    .into_iter()
    .collect();
    assert_eq!(surfaces.len(), 3);

    // ── Two-vs-one family split is the documented asymmetry.
    let rename_fam = rename["artifact_family"].as_str().unwrap();
    let stub_fam = stub["artifact_family"].as_str().unwrap();
    let rmi_fam = rmi["artifact_family"].as_str().unwrap();
    assert_eq!(rename_fam, "coding.generated-patch-candidate");
    assert_eq!(stub_fam, "coding.generated-patch-candidate");
    assert_eq!(rmi_fam, "coding.code-transform.remove-unused-import-ready");
  }

  // ─── substrate-share: both transforms land in the same family ──

  #[test]
  fn rename_symbol_and_add_test_stub_artifacts_share_family_and_panel_shape() {
    let rename_art = dispatch_patch_candidate_for_request(
      "rename-symbol",
      &json!({
        "old_name": "foo",
        "new_name": "bar",
        "language": "rust",
        "scope": "local-target-paths",
        "target_paths": ["src/a.rs"]
      }),
      &[PatchInputFile {
        path: "src/a.rs",
        content: "fn foo() {}\n",
      }],
      0,
      None,
    )
    .expect("rename ok");

    let stub_art = dispatch_patch_candidate_for_request(
      "add-test-stub",
      &json!({
        "target_module": "src/lib.rs",
        "test_name": "ok",
        "language": "rust",
        "intent": "",
        "place": null
      }),
      &[PatchInputFile {
        path: "src/lib.rs",
        content: "fn x() {}\n",
      }],
      0,
      None,
    )
    .expect("stub ok");

    // Same artifact_family (cockpit doesn't need a new panel per
    // transform — it pivots on `transform` field in the payload).
    assert_eq!(
      rename_art["artifact_family"], stub_art["artifact_family"],
      "both must land in coding.generated-patch-candidate"
    );
    // Distinct ids (different transforms + different content).
    assert_ne!(rename_art["id"], stub_art["id"]);
    // Distinct source_surfaces — the per-transform owner-law ref.
    assert_ne!(rename_art["source_surface"], stub_art["source_surface"]);
    // Both have a payload with a `transform` field — substrate-share
    // at the panel-pivot layer.
    assert!(rename_art["payload"]["transform"].is_string());
    assert!(stub_art["payload"]["transform"].is_string());
  }

  // ─── patch-dispatch-held audit artifact ──────────────────────

  #[test]
  fn held_artifact_for_unknown_transform_canonical_shape() {
    let err = PatchDispatchError::UnknownTransform {
      transform: "what-is-this".to_string(),
    };
    let art = build_patch_dispatch_held_artifact(&err, 1_700_000_000_000, None, None);
    assert_eq!(
      art["artifact_family"].as_str(),
      Some("coding.patch-dispatch-held")
    );
    assert_eq!(
      art["source_surface"].as_str(),
      Some("algorithm-synthesis.patch-candidate-dispatch")
    );
    assert!(art["id"]
      .as_str()
      .unwrap()
      .starts_with("patch-dispatch-held."));
    let payload = &art["payload"];
    assert_eq!(payload["kind"].as_str(), Some("unknown-transform"));
    assert_eq!(payload["transform"].as_str(), Some("what-is-this"));
    assert!(payload["reason"]
      .as_str()
      .unwrap()
      .contains("unknown transform"));
    assert!(payload["suggested_action"]
      .as_str()
      .unwrap()
      .contains("register"));
    assert_eq!(payload["candidate_only"].as_bool(), Some(true));
    assert_eq!(
      payload["next_step"].as_str(),
      Some("operator-decision-or-resubmit")
    );
  }

  #[test]
  fn held_artifact_for_missing_file_for_target_path_carries_path() {
    let err = PatchDispatchError::MissingFileForTargetPath {
      transform: "rename-symbol".to_string(),
      missing_path: "src/b.rs".to_string(),
    };
    let art = build_patch_dispatch_held_artifact(&err, 0, None, None);
    assert_eq!(
      art["payload"]["kind"].as_str(),
      Some("missing-file-for-target-path")
    );
    assert_eq!(art["payload"]["missing_path"].as_str(), Some("src/b.rs"));
    assert!(art["payload"]["suggested_action"]
      .as_str()
      .unwrap()
      .contains("src/b.rs"));
  }

  #[test]
  fn held_artifact_for_deserialize_failed_carries_serde_error() {
    let err = PatchDispatchError::RequestDeserializeFailed {
      transform: "rename-symbol".to_string(),
      serde_error: "missing field `old_name`".to_string(),
    };
    let art = build_patch_dispatch_held_artifact(&err, 0, None, None);
    assert_eq!(
      art["payload"]["kind"].as_str(),
      Some("request-deserialize-failed")
    );
    assert_eq!(
      art["payload"]["serde_error"].as_str(),
      Some("missing field `old_name`")
    );
  }

  #[test]
  fn held_artifact_carries_request_summary_when_provided() {
    let err = PatchDispatchError::MissingFileInputs {
      transform: "add-test-stub".to_string(),
    };
    let summary = serde_json::json!({"target_module": "src/lib.rs", "test_name": "ok"});
    let art = build_patch_dispatch_held_artifact(&err, 0, Some(summary.clone()), None);
    assert_eq!(art["payload"]["request_summary"], summary);
  }

  #[test]
  fn held_artifact_id_is_replay_stable_across_stored_at_ms() {
    let err = PatchDispatchError::UnknownTransform {
      transform: "x".to_string(),
    };
    let a = build_patch_dispatch_held_artifact(&err, 0, None, None);
    let b = build_patch_dispatch_held_artifact(&err, 9_999_999, None, None);
    assert_eq!(a["id"], b["id"]);
    assert_ne!(a["stored_at_ms"], b["stored_at_ms"]);
  }

  #[test]
  fn held_artifact_id_differs_per_error_variant() {
    let a = build_patch_dispatch_held_artifact(
      &PatchDispatchError::UnknownTransform {
        transform: "x".to_string(),
      },
      0,
      None,
      None,
    );
    let b = build_patch_dispatch_held_artifact(
      &PatchDispatchError::MissingFileInputs {
        transform: "x".to_string(),
      },
      0,
      None,
      None,
    );
    assert_ne!(a["id"], b["id"]);
  }

  #[test]
  fn dispatch_is_replay_stable_across_stored_at_ms() {
    let request_json = json!({
      "old_name": "foo",
      "new_name": "bar",
      "language": "rust",
      "scope": "local-target-paths",
      "target_paths": ["src/a.rs"]
    });
    let inputs = [PatchInputFile {
      path: "src/a.rs",
      content: "fn foo() {}\n",
    }];
    let a = dispatch_patch_candidate_for_request("rename-symbol", &request_json, &inputs, 0, None)
      .unwrap();
    let b = dispatch_patch_candidate_for_request(
      "rename-symbol",
      &request_json,
      &inputs,
      9_999_999,
      None,
    )
    .unwrap();
    assert_eq!(a["id"], b["id"], "id must not depend on stored_at_ms");
    assert_ne!(
      a["stored_at_ms"], b["stored_at_ms"],
      "envelope metadata still differs"
    );
  }
}
