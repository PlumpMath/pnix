//! dist 디렉토리 내보내기 (executor 전용)
//!
//! pnix-core는 fs I/O 금지이므로, dist 출력은 executor에서 수행한다.
#![allow(dead_code)]

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// 산출물 텍스트 묶음
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmitArtifacts {
  pub fxcore_json: String,
  pub ssa_json: String,
  pub build_ir_json: String,
  pub spec_canon_json: Option<String>,
  pub used_spec_canon_json: Option<String>,
  pub replay_hash: String,
}

/// manifest 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmitManifest {
  pub pnix_core_version: String,
  pub source: EmitSource,
  pub compile_options: EmitCompileOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmitSource {
  pub name: String,
  pub bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmitCompileOptions {
  pub target_os: String,
  pub target_arch: String,
  pub deterministic: bool,
}

/// report 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmitReport {
  pub ok: bool,
  pub closure: EmitClosure,
  pub notes: Vec<String>,
  pub diagnostics: Vec<EmitDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmitClosure {
  pub s2_reference_closure: bool,
  pub s3_contracts: bool,
  pub s4_dependency_closure: bool,
  pub s5_deterministic_artifacts: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmitDiagnostic {
  pub message: String,
  pub span: Option<EmitSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmitSpan {
  pub start: usize,
  pub end: usize,
  pub file: Option<String>,
}

/// emit 입력
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmitInput {
  pub artifacts: EmitArtifacts,
  pub manifest: EmitManifest,
  pub report: EmitReport,
}

/// dist 디렉토리로 emit
///
/// 레이아웃:
/// ```text
/// dist/
/// ├─ pnix.manifest.json
/// ├─ pnix.replay.json
/// ├─ pnix.report.json
/// ├─ ir/
/// │  ├─ fxcore.canon.json
/// │  ├─ ssa.canon.json
/// │  └─ build_ir.canon.json
/// │  ├─ spec.canon.json (optional)
/// │  └─ used_spec.canon.json (optional)
/// └─ artifacts/
/// ```
pub fn emit_to_dir(input: &EmitInput, dir: impl AsRef<Path>) -> Result<PathBuf> {
  let dir = dir.as_ref();
  let parent = dir
    .parent()
    .ok_or_else(|| anyhow::anyhow!("emit_to_dir: target directory has no parent"))?;
  fs::create_dir_all(parent)?;

  let dir_name = dir
    .file_name()
    .ok_or_else(|| anyhow::anyhow!("emit_to_dir: invalid target directory name"))?
    .to_string_lossy()
    .to_string();
  let tmp_dir = unique_hidden_dir(parent, &dir_name, "tmp")?;
  if tmp_dir.exists() {
    fs::remove_dir_all(&tmp_dir)?;
  }
  fs::create_dir_all(&tmp_dir)?;

  let ir_dir = tmp_dir.join("ir");
  let art_dir = tmp_dir.join("artifacts");
  fs::create_dir_all(&ir_dir)?;
  fs::create_dir_all(&art_dir)?;

  // 1) canonical IR dumps
  write_str(
    ir_dir.join("fxcore.canon.json"),
    &input.artifacts.fxcore_json,
  )?;
  write_str(ir_dir.join("ssa.canon.json"), &input.artifacts.ssa_json)?;
  write_str(
    ir_dir.join("build_ir.canon.json"),
    &input.artifacts.build_ir_json,
  )?;
  if let Some(spec_canon_json) = input.artifacts.spec_canon_json.as_deref() {
    write_str(ir_dir.join("spec.canon.json"), spec_canon_json)?;
  }
  if let Some(used_spec_canon_json) = input.artifacts.used_spec_canon_json.as_deref() {
    write_str(ir_dir.join("used_spec.canon.json"), used_spec_canon_json)?;
  }

  // 2) replay.json (component hashes + replay hash)
  let fx_h = sha256_hex(input.artifacts.fxcore_json.as_bytes());
  let ssa_h = sha256_hex(input.artifacts.ssa_json.as_bytes());
  let bir_h = sha256_hex(input.artifacts.build_ir_json.as_bytes());

  let replay = json!({
      "version": "1.0",
      "replay_hash": input.artifacts.replay_hash,
      "components": {
          "fxcore": fx_h,
          "ssa": ssa_h,
          "build_ir": bir_h
      }
  });
  write_json(tmp_dir.join("pnix.replay.json"), &replay)?;

  // 3) manifest.json
  write_json(tmp_dir.join("pnix.manifest.json"), &input.manifest)?;

  // 4) report.json (closure + diagnostics)
  write_json(tmp_dir.join("pnix.report.json"), &input.report)?;

  let backup_dir = unique_hidden_dir(parent, &dir_name, "bak")?;
  if dir.exists() {
    if backup_dir.exists() {
      fs::remove_dir_all(&backup_dir)?;
    }
    fs::rename(dir, &backup_dir)?;
  }

  if let Err(err) = fs::rename(&tmp_dir, dir) {
    if backup_dir.exists() {
      let _ = fs::rename(&backup_dir, dir);
    }
    return Err(err.into());
  }

  if backup_dir.exists() {
    let _ = fs::remove_dir_all(&backup_dir);
  }

  Ok(dir.to_path_buf())
}

fn write_str(path: PathBuf, content: &str) -> Result<()> {
  let mut file = fs::File::create(&path)?;
  file.write_all(content.as_bytes())?;
  file.sync_all()?;
  Ok(())
}

fn write_json<T: Serialize>(path: PathBuf, v: &T) -> Result<()> {
  let s = serde_json::to_string_pretty(v)?;
  write_str(path, &s)
}

fn sha256_hex(bytes: &[u8]) -> String {
  use pnix_hash::{Digest, Sha256};
  let mut h = Sha256::new();
  h.update(bytes);
  format!("{:x}", h.finalize())
}

fn unique_hidden_dir(parent: &Path, stem: &str, suffix: &str) -> Result<PathBuf> {
  let pid = std::process::id();
  for attempt in 0..1000 {
    let name = if attempt == 0 {
      format!(".{}-{}-{}", stem, suffix, pid)
    } else {
      format!(".{}-{}-{}-{}", stem, suffix, pid, attempt)
    };
    let candidate = parent.join(name);
    if !candidate.exists() {
      return Ok(candidate);
    }
  }
  Err(anyhow::anyhow!(
    "emit_to_dir: failed to allocate unique directory name"
  ))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_emit_to_dir_overwrites_existing_directory() {
    let base = std::env::temp_dir().join(format!("pnix-emit-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).expect("create base dir");

    let dir = base.join("dist");
    fs::create_dir_all(&dir).expect("create dist dir");
    fs::write(dir.join("old.txt"), "old").expect("write old file");

    let input = EmitInput {
      artifacts: EmitArtifacts {
        fxcore_json: "{}".to_string(),
        ssa_json: "{}".to_string(),
        build_ir_json: "{}".to_string(),
        spec_canon_json: None,
        used_spec_canon_json: None,
        replay_hash: "hash".to_string(),
      },
      manifest: EmitManifest {
        pnix_core_version: "0.0.0".to_string(),
        source: EmitSource {
          name: "test".to_string(),
          bytes: 0,
        },
        compile_options: EmitCompileOptions {
          target_os: "test-os".to_string(),
          target_arch: "test-arch".to_string(),
          deterministic: true,
        },
      },
      report: EmitReport {
        ok: true,
        closure: EmitClosure::default(),
        notes: Vec::new(),
        diagnostics: Vec::new(),
      },
    };

    emit_to_dir(&input, &dir).expect("emit_to_dir");

    assert!(dir.join("pnix.manifest.json").exists());
    assert!(dir.join("pnix.replay.json").exists());
    assert!(dir.join("pnix.report.json").exists());
    assert!(!dir.join("old.txt").exists());
    assert!(!dir.join("ir").join("spec.canon.json").exists());
    assert!(!dir.join("ir").join("used_spec.canon.json").exists());

    let _ = fs::remove_dir_all(&base);
  }

  #[test]
  fn test_emit_to_dir_writes_optional_spec_artifacts() {
    let base = std::env::temp_dir().join(format!("pnix-emit-test-spec-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).expect("create base dir");

    let dir = base.join("dist");
    let input = EmitInput {
      artifacts: EmitArtifacts {
        fxcore_json: "{}".to_string(),
        ssa_json: "{}".to_string(),
        build_ir_json: "{}".to_string(),
        spec_canon_json: Some("{\"builtins\":{}}".to_string()),
        used_spec_canon_json: Some("{\"used_builtins\":{\"add\":{}}}".to_string()),
        replay_hash: "hash".to_string(),
      },
      manifest: EmitManifest {
        pnix_core_version: "0.0.0".to_string(),
        source: EmitSource {
          name: "test".to_string(),
          bytes: 0,
        },
        compile_options: EmitCompileOptions {
          target_os: "test-os".to_string(),
          target_arch: "test-arch".to_string(),
          deterministic: true,
        },
      },
      report: EmitReport {
        ok: true,
        closure: EmitClosure::default(),
        notes: Vec::new(),
        diagnostics: Vec::new(),
      },
    };

    emit_to_dir(&input, &dir).expect("emit_to_dir");

    assert!(dir.join("ir").join("spec.canon.json").exists());
    assert!(dir.join("ir").join("used_spec.canon.json").exists());

    let _ = fs::remove_dir_all(&base);
  }
}
