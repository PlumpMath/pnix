//! W06b: Capability 체크 실제 적용

use anyhow::{Context, Result};
use pnix_core::spec::fxcore_link::UsedSpec;
use pnix_runtime_api::{CapabilityCheckResult, CapabilityChecker, RuntimeCapability};
use std::path::Path;

/// UsedSpec에서 요구되는 capabilities 추출
pub fn extract_required_capabilities(used_spec: &UsedSpec) -> Result<Vec<RuntimeCapability>> {
  let mut required = Vec::new();

  // builtin 사용 시 해당 builtin의 capability 요구사항 확인
  for builtin_decl in used_spec.used_builtins.values() {
    // builtin_decl.capabilities를 RuntimeCapability로 변환
    for cap_name in &builtin_decl.capabilities {
      if let Some(cap) = capability_name_to_runtime_capability(cap_name) {
        required.push(cap);
      } else {
        anyhow::bail!("unknown required capability in used_spec: {}", cap_name);
      }
    }
  }

  // 중복 제거
  required.sort();
  required.dedup();
  Ok(required)
}

/// Capability 이름을 RuntimeCapability로 변환
fn capability_name_to_runtime_capability(name: &str) -> Option<RuntimeCapability> {
  match cap_key(name).as_str() {
    "pure" => Some(RuntimeCapability::Pure),
    "world" => Some(RuntimeCapability::World),
    "io" => Some(RuntimeCapability::Io),
    "network" => Some(RuntimeCapability::Network),
    "filesystem" => Some(RuntimeCapability::FileSystem),
    "math" => Some(RuntimeCapability::Math),
    "arithmetic" => Some(RuntimeCapability::Arithmetic),
    "trigonometry" => Some(RuntimeCapability::Trigonometry),
    "comparison" => Some(RuntimeCapability::Comparison),
    "logic" => Some(RuntimeCapability::Logic),
    "ssaeval" => Some(RuntimeCapability::SSAEval),
    "frptick" => Some(RuntimeCapability::FRPTick),
    "ctverify" => Some(RuntimeCapability::CTVerify),
    "llvm" => Some(RuntimeCapability::LLVM),
    "llvmjit" => Some(RuntimeCapability::LLVM),
    "llvmaot" => Some(RuntimeCapability::LLVM),
    "process" => Some(RuntimeCapability::Process),
    "processspawn" | "spawnprocess" => Some(RuntimeCapability::ProcessSpawn),
    "processsignal" | "signalprocess" => Some(RuntimeCapability::ProcessSignal),
    "processobserve" | "observeprocess" => Some(RuntimeCapability::ProcessObserve),
    "schema" | "xml" | "x3d" | "x3dom" | "mathml" | "openmath" | "svg" | "ifcxml" | "sbml"
    | "cellml" | "neuroml" | "lems" | "sedml" | "omex" | "pharmml" | "cml" | "pdbml" | "sbgnml"
    | "biopax" | "vtk" | "xdmf" | "gifti" | "frp" | "html" | "hanim" | "patch" | "physics"
    | "symbolic" | "sync" | "webview" | "ontology" | "meaning" | "query" => {
      Some(RuntimeCapability::Pure)
    }
    "emit" => Some(RuntimeCapability::World),
    _ => None,
  }
}

fn cap_key(name: &str) -> String {
  name
    .chars()
    .filter(|c| c.is_ascii_alphanumeric())
    .flat_map(|c| c.to_lowercase())
    .collect()
}

/// dist에서 UsedSpec 읽기
pub fn load_used_spec_from_dist(dist_path: &Path) -> Result<Option<UsedSpec>> {
  let used_spec_path = dist_path.join("ir").join("used_spec.canon.json");

  if !used_spec_path.exists() {
    return Ok(None);
  }

  let content = std::fs::read_to_string(&used_spec_path)
    .with_context(|| format!("failed to read used_spec from {}", used_spec_path.display()))?;

  let used_spec: UsedSpec = serde_json::from_str(&content).with_context(|| {
    format!(
      "failed to parse used_spec JSON from {}",
      used_spec_path.display()
    )
  })?;

  Ok(Some(used_spec))
}

/// 엔진 capability 체크 및 명시적 에러 메시지 생성
pub fn check_engine_capabilities(
  checker: &dyn CapabilityChecker,
  required_capabilities: &[RuntimeCapability],
  required_engines: &[String],
  required_options: &[String],
) -> Result<CapabilityCheckResult> {
  let result =
    checker.check_capabilities(required_capabilities, required_engines, required_options);

  if !result.success {
    // 명시적 에러 메시지 생성
    let mut message = format!(
      "Capability check failed: missing capabilities: {:?}",
      result.missing_capabilities
    );

    if !result.missing_engines.is_empty() {
      message.push_str(&format!(", missing engines: {:?}", result.missing_engines));
    }

    if !result.missing_options.is_empty() {
      message.push_str(&format!(", missing options: {:?}", result.missing_options));
    }

    // 지원되는 capabilities/engines/options 정보 추가
    let caps = checker.capabilities();
    message.push_str(&format!(
      "\nSupported capabilities: {:?}",
      caps.capabilities
    ));
    message.push_str(&format!("\nSupported engines: {:?}", caps.engines));
    message.push_str(&format!("\nSupported options: {:?}", caps.options));
  }

  Ok(result)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn capability_name_mapping_accepts_process_variants() {
    assert_eq!(
      capability_name_to_runtime_capability("Process"),
      Some(RuntimeCapability::Process)
    );
    assert_eq!(
      capability_name_to_runtime_capability("ProcessSpawn"),
      Some(RuntimeCapability::ProcessSpawn)
    );
    assert_eq!(
      capability_name_to_runtime_capability("process_spawn"),
      Some(RuntimeCapability::ProcessSpawn)
    );
    assert_eq!(
      capability_name_to_runtime_capability("spawn_process"),
      Some(RuntimeCapability::ProcessSpawn)
    );
    assert_eq!(
      capability_name_to_runtime_capability("processSignal"),
      Some(RuntimeCapability::ProcessSignal)
    );
    assert_eq!(
      capability_name_to_runtime_capability("observe_process"),
      Some(RuntimeCapability::ProcessObserve)
    );
    assert_eq!(
      capability_name_to_runtime_capability("process_observe"),
      Some(RuntimeCapability::ProcessObserve)
    );
  }

  #[test]
  fn capability_name_mapping_accepts_domain_pure_variants() {
    for cap in [
      "Svg", "X3dom", "Patch", "Sync", "Webview", "HAnim", "Physics", "Symbolic", "Ontology",
      "Meaning", "Query",
    ] {
      assert_eq!(
        capability_name_to_runtime_capability(cap),
        Some(RuntimeCapability::Pure),
        "expected {cap} to collapse into RuntimeCapability::Pure"
      );
    }
    assert_eq!(
      capability_name_to_runtime_capability("Emit"),
      Some(RuntimeCapability::World)
    );
  }
}
