//! 평가 패치: 평가 엔진 상태 수정 작업

use anyhow::{bail, Result};
use serde::Deserialize;
use serde_json::Value;

use crate::state_mutation_contract::enforce_state_mutation_contract;
use pnix_runtime_api::{EvalPatchResult, EvalPatchable};

/// 평가 패치 파일: 평가 엔진 상태 수정 작업 목록
#[derive(Debug, Deserialize)]
pub struct EvalPatchFile {
  /// 패치 버전
  pub version: u32,
  /// Patch identity (optional in compat mode)
  #[serde(default)]
  pub patch_id: Option<String>,
  /// Idempotency key (optional in compat mode)
  #[serde(default)]
  pub idempotency_key: Option<String>,
  /// Commit actor (must be `sequencer` when present)
  #[serde(default)]
  pub committer: Option<String>,
  /// 패치 작업 목록 (JSON 값)
  pub patches: Vec<Value>,
}

impl EvalPatchFile {
  /// JSON 문자열에서 평가 패치 파일 파싱
  ///
  /// 보안: DoS 공격 방지를 위해 입력 크기 제한
  pub fn from_json_str(input: &str) -> Result<Self> {
    // 보안: DoS 공격 방지를 위한 입력 크기 제한
    const MAX_PATCH_SIZE: usize = 10 * 1024 * 1024; // 10MB
    if input.len() > MAX_PATCH_SIZE {
      bail!(
        "eval patch JSON too large: {} bytes (max: {} bytes)",
        input.len(),
        MAX_PATCH_SIZE
      );
    }

    let patch: EvalPatchFile = serde_json::from_str(input)
      .map_err(|err| anyhow::anyhow!("invalid eval patch json: {}", err))?;
    Ok(patch)
  }
}

/// 패치 적용: 평가 엔진에 패치 작업들을 순차적으로 적용
pub fn apply_patches<E>(engine: &mut E, patch: EvalPatchFile) -> Result<Vec<EvalPatchResult>>
where
  E: EvalPatchable<Patch = Value>,
{
  enforce_state_mutation_contract(
    "eval_patch",
    patch.patch_id.as_deref(),
    patch.idempotency_key.as_deref(),
    patch.committer.as_deref(),
  )?;

  if patch.version != 1 {
    bail!("unsupported eval patch version {}", patch.version);
  }
  let mut results = Vec::with_capacity(patch.patches.len());
  for entry in patch.patches {
    match engine.apply_patch(&entry) {
      Ok(result) => results.push(result),
      Err(err) => {
        // Use Display format instead of Debug for user-friendly error messages
        results.push(EvalPatchResult::error(err.to_string()))
      }
    }
  }
  Ok(results)
}

#[cfg(test)]
mod tests {
  use super::*;
  use pnix_runtime_api::{EvalPatchResult, RuntimeError, RuntimeResult};

  struct DummyEngine {
    patches: Vec<Value>,
    fail: bool,
  }

  impl DummyEngine {
    fn new(fail: bool) -> Self {
      Self {
        patches: Vec::new(),
        fail,
      }
    }
  }

  impl EvalPatchable for DummyEngine {
    type Patch = Value;

    fn apply_patch(&mut self, patch: &Self::Patch) -> RuntimeResult<EvalPatchResult> {
      self.patches.push(patch.clone());
      if self.fail {
        Err(RuntimeError::message("boom"))
      } else {
        Ok(EvalPatchResult::ok())
      }
    }
  }

  #[test]
  fn parse_eval_patch_file() {
    let input = r#"{"version":1,"patches":[{"op":"noop"}]}"#;
    let patch = EvalPatchFile::from_json_str(input).expect("parse eval patch");
    assert_eq!(patch.version, 1);
    assert_eq!(patch.patches.len(), 1);
    assert!(patch.patch_id.is_none());
    assert!(patch.idempotency_key.is_none());
    assert!(patch.committer.is_none());
  }

  #[test]
  fn apply_patches_rejects_version() {
    let patch = EvalPatchFile {
      version: 2,
      patch_id: None,
      idempotency_key: None,
      committer: None,
      patches: vec![],
    };
    let mut engine = DummyEngine::new(false);
    let err = apply_patches(&mut engine, patch).expect_err("unsupported version");
    let msg = err.to_string();
    assert!(msg.contains("unsupported eval patch version"));
  }

  #[test]
  fn apply_patches_collects_results() {
    let patch = EvalPatchFile {
      version: 1,
      patch_id: None,
      idempotency_key: None,
      committer: Some("sequencer".to_string()),
      patches: vec![serde_json::json!({"op":"a"}), serde_json::json!({"op":"b"})],
    };
    let mut engine = DummyEngine::new(false);
    let results = apply_patches(&mut engine, patch).expect("apply patches");
    assert_eq!(engine.patches.len(), 2);
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|r| r.success));
  }

  #[test]
  fn apply_patches_records_failures() {
    let patch = EvalPatchFile {
      version: 1,
      patch_id: None,
      idempotency_key: None,
      committer: None,
      patches: vec![serde_json::json!({"op":"fail"})],
    };
    let mut engine = DummyEngine::new(true);
    let results = apply_patches(&mut engine, patch).expect("apply patches");
    assert_eq!(results.len(), 1);
    assert!(!results[0].success);
    assert!(results[0].message.contains("boom"));
  }

  #[test]
  fn apply_patches_rejects_non_sequencer_committer() {
    let patch = EvalPatchFile {
      version: 1,
      patch_id: None,
      idempotency_key: None,
      committer: Some("manual".to_string()),
      patches: vec![],
    };
    let mut engine = DummyEngine::new(false);
    let err = apply_patches(&mut engine, patch).expect_err("non-sequencer must fail");
    assert!(err
      .to_string()
      .contains("STATE_MUTATION_NON_SEQUENCER_COMMIT"));
  }
}
