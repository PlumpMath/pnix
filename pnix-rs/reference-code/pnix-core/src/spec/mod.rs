//! Spec catalog (data only, no execution)
//!
//! ## 헌법 준수 (P0-1)
//!
//! Spec은 **데이터 구조만** 포함합니다:
//! - builtin/stdlib 선언 카탈로그
//! - effect/capability 선언
//! - lowering 규칙 레지스트리 (데이터)
//! - 계약 검증 기준
//!
//! 실행 로직은 포함하지 않습니다.

pub mod builtin;
pub mod capability;
pub mod contracts;
pub mod fxcore_link;
pub mod lowering;
pub mod operators;
pub mod stdlib;

#[cfg(test)]
mod test_spec_validation;

use serde::{Deserialize, Serialize};

/// Spec version (frozen)
pub const SPEC_VERSION: &str = "spec@0.1";

/// Unified spec catalog
///
/// 모든 spec을 통합하는 최상위 타입입니다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spec {
  /// Spec 버전
  pub version: String,
  /// Builtin 함수 선언 카탈로그
  pub builtins: builtin::BuiltinCatalog,
  /// 표준 라이브러리 타입/함수 선언
  pub stdlib: stdlib::StdlibCatalog,
  /// Operator 레지스트리 (레이어 토큰 정의)
  #[serde(default)]
  pub operators: operators::OperatorCatalog,
  /// Capability 선언
  pub capabilities: capability::CapabilityCatalog,
  /// Lowering 규칙 레지스트리
  pub lowering_rules: lowering::LoweringRules,
  /// 계약 검증 기준
  pub contract_rules: contracts::ContractRules,
}

impl Spec {
  /// 기본 spec 생성 (빈 카탈로그)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new() -> Self {
    Self {
      version: SPEC_VERSION.to_string(),
      builtins: builtin::BuiltinCatalog::new(),
      stdlib: stdlib::StdlibCatalog::new(),
      operators: operators::OperatorCatalog::new(),
      capabilities: capability::CapabilityCatalog::new(),
      lowering_rules: lowering::LoweringRules::new(),
      contract_rules: contracts::ContractRules::new(),
    }
  }

  /// 기본 spec 생성 (기본 builtin/stdlib 포함)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn with_defaults() -> Self {
    let mut spec = Self::new();
    spec.builtins = builtin::BuiltinCatalog::with_defaults();
    spec.stdlib = stdlib::StdlibCatalog::with_defaults();
    spec.operators = operators::OperatorCatalog::with_defaults();
    spec.capabilities = capability::CapabilityCatalog::with_defaults();
    spec.lowering_rules = lowering::LoweringRules::with_defaults();
    spec.contract_rules = contracts::ContractRules::with_defaults();
    spec
  }
}

impl Default for Spec {
  fn default() -> Self {
    Self::with_defaults()
  }
}

/// Spec canonical JSON 산출
///
/// 결정론 보장을 위해 정규화된 JSON을 생성합니다.
///
/// ## 헌법 준수 (P0-1, C1)
///
/// 텍스트 생성만, 파일 I/O 없음
pub fn emit_spec_canonical(spec: &Spec) -> crate::MeaningResult<String> {
  use crate::codegen::normalize;

  // 1) serialize
  let spec_v = serde_json::to_value(spec)
    .map_err(|e| crate::MeaningError::Internal(format!("spec json: {e}"), None))?;

  // 2) normalize + canonicalize
  let spec_n = normalize::canonicalize(spec_v);

  // 3) stable string
  let spec_json = normalize::to_pretty(&spec_n);

  Ok(spec_json)
}

/// Spec 해시 산출
///
/// 결정론 보장을 위해 canonical JSON의 해시를 생성합니다.
///
/// ## 헌법 준수 (P0-1, C1)
///
/// 텍스트 생성만, 파일 I/O 없음
pub fn spec_hash(spec: &Spec) -> crate::MeaningResult<String> {
  use crate::codegen::normalize;
  use pnix_hash::{Digest, Sha256};

  // 1) serialize
  let spec_v = serde_json::to_value(spec)
    .map_err(|e| crate::MeaningError::Internal(format!("spec json: {e}"), None))?;

  // 2) normalize + canonicalize
  let spec_n = normalize::canonicalize(spec_v);

  // 3) canonical JSON bytes (compact)
  let spec_b = serde_json::to_vec(&spec_n)
    .map_err(|e| crate::MeaningError::Internal(format!("spec bytes: {e}"), None))?;

  // 4) hash
  let mut hasher = Sha256::new();
  hasher.update(&spec_b);

  Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_spec_creation() {
    let spec = Spec::new();
    assert_eq!(spec.version, SPEC_VERSION);
  }

  #[test]
  fn test_spec_with_defaults() {
    let spec = Spec::with_defaults();
    assert_eq!(spec.version, SPEC_VERSION);
    // 기본 builtin이 있는지 확인
    assert!(!spec.builtins.functions.is_empty());
  }

  #[test]
  fn test_spec_serialization() {
    let spec = Spec::with_defaults();
    let json = serde_json::to_string(&spec).unwrap();
    let deserialized: Spec = serde_json::from_str(&json).unwrap();
    assert_eq!(spec.version, deserialized.version);
  }

  #[test]
  fn test_spec_canonical_json() {
    let spec = Spec::with_defaults();
    let json1 = emit_spec_canonical(&spec).unwrap();
    let json2 = emit_spec_canonical(&spec).unwrap();
    // 동일한 spec은 동일한 canonical JSON을 생성해야 함
    assert_eq!(json1, json2);
  }

  #[test]
  fn test_spec_hash() {
    let spec = Spec::with_defaults();
    let hash1 = spec_hash(&spec).unwrap();
    let hash2 = spec_hash(&spec).unwrap();
    // 동일한 spec은 동일한 해시를 생성해야 함
    assert_eq!(hash1, hash2);
  }

  #[test]
  fn test_spec_hash_deterministic() {
    let spec1 = Spec::with_defaults();
    let spec2 = Spec::with_defaults();
    let hash1 = spec_hash(&spec1).unwrap();
    let hash2 = spec_hash(&spec2).unwrap();
    // 동일한 기본값 spec은 동일한 해시를 생성해야 함
    assert_eq!(hash1, hash2);
  }

  #[test]
  fn test_spec_canon_includes_operators_and_rat() {
    let spec = Spec::with_defaults();
    let json = emit_spec_canonical(&spec).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert!(value.get("operators").is_some());
    let rat = value
      .get("stdlib")
      .and_then(|stdlib| stdlib.get("types"))
      .and_then(|types| types.get("Rat"));
    assert!(rat.is_some());
  }
}
