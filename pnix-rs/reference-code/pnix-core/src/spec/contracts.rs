//! Contract verification rules (data only)
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 실행 로직 제외
//! 실제 검증 로직은 `contracts/verify.rs`에 있지만,
//! 검증 규칙(어떤 규칙을 검증하는지)은 여기에 데이터로 선언

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 계약 검증 규칙 종류
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContractRuleKind {
  /// S2: Reference Closure
  S2ReferenceClosure,
  /// S2-A: Graph Closure
  S2GraphClosure,
  /// S2-B: Port Closure
  S2PortClosure,
  /// S2-C: Input Closure
  S2InputClosure,
  /// S2-D: EdgeCond Closure
  S2EdgeCondClosure,
  /// S3: Contract Verification
  S3ContractVerification,
  /// S3-B: Edge Type Compatibility
  S3EdgeTypeCompatibility,
  /// S4: Dependency Closure
  S4DependencyClosure,
  /// S5: Deterministic Artifacts
  S5DeterministicArtifacts,
}

/// 계약 검증 규칙 선언
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContractRule {
  /// 규칙 이름
  pub name: String,
  /// 규칙 종류
  pub kind: ContractRuleKind,
  /// 규칙 설명
  pub description: String,
  /// 규칙이 활성화되어 있는지
  pub enabled: bool,
  /// Stage 요구사항 (예: 2 = Stage 2 이상에서만 활성화)
  pub stage_requirement: u8,
}

/// 계약 검증 규칙 레지스트리
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractRules {
  /// 등록된 규칙들 (이름 → 규칙)
  pub rules: BTreeMap<String, ContractRule>,
}

impl ContractRules {
  /// 빈 레지스트리 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new() -> Self {
    Self {
      rules: BTreeMap::new(),
    }
  }

  /// 기본 규칙 포함 레지스트리 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn with_defaults() -> Self {
    let mut registry = Self::new();

    registry.register(ContractRule {
      name: "s2_reference_closure".to_string(),
      kind: ContractRuleKind::S2ReferenceClosure,
      description: "S2: All references must be resolved".to_string(),
      enabled: true,
      stage_requirement: 1,
    });
    registry.register(ContractRule {
      name: "s2_graph_closure".to_string(),
      kind: ContractRuleKind::S2GraphClosure,
      description: "S2-A: Graph closure (all nodes/edges referenced)".to_string(),
      enabled: true,
      stage_requirement: 1,
    });
    registry.register(ContractRule {
      name: "s2_port_closure".to_string(),
      kind: ContractRuleKind::S2PortClosure,
      description: "S2-B: Port closure (all ports referenced)".to_string(),
      enabled: true,
      stage_requirement: 2,
    });
    registry.register(ContractRule {
      name: "s2_input_closure".to_string(),
      kind: ContractRuleKind::S2InputClosure,
      description: "S2-C: Input closure (all inputs referenced)".to_string(),
      enabled: true,
      stage_requirement: 2,
    });
    registry.register(ContractRule {
      name: "s2_edge_cond_closure".to_string(),
      kind: ContractRuleKind::S2EdgeCondClosure,
      description: "S2-D: EdgeCond reference closure".to_string(),
      enabled: true,
      stage_requirement: 3,
    });
    registry.register(ContractRule {
      name: "s3_contract_verification".to_string(),
      kind: ContractRuleKind::S3ContractVerification,
      description: "S3: Contract verification".to_string(),
      enabled: true,
      stage_requirement: 1,
    });
    registry.register(ContractRule {
      name: "s3_edge_type_compatibility".to_string(),
      kind: ContractRuleKind::S3EdgeTypeCompatibility,
      description: "S3-B: Edge type compatibility".to_string(),
      enabled: true,
      stage_requirement: 1,
    });
    registry.register(ContractRule {
      name: "s4_dependency_closure".to_string(),
      kind: ContractRuleKind::S4DependencyClosure,
      description: "S4: Dependency closure".to_string(),
      enabled: true,
      stage_requirement: 1,
    });
    registry.register(ContractRule {
      name: "s5_deterministic_artifacts".to_string(),
      kind: ContractRuleKind::S5DeterministicArtifacts,
      description: "S5: Deterministic artifacts".to_string(),
      enabled: true,
      stage_requirement: 1,
    });

    registry
  }

  /// 규칙 등록
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn register(&mut self, rule: ContractRule) {
    self.rules.insert(rule.name.clone(), rule);
  }

  /// 규칙 조회
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn get(&self, name: &str) -> Option<&ContractRule> {
    self.rules.get(name)
  }

  /// 규칙 존재 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn contains(&self, name: &str) -> bool {
    self.rules.contains_key(name)
  }

  /// Stage에 활성화된 규칙들 조회
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 필터링만, 값 계산 없음
  pub fn enabled_for_stage(&self, stage: u8) -> Vec<&ContractRule> {
    self
      .rules
      .values()
      .filter(|r| r.enabled && r.stage_requirement <= stage)
      .collect()
  }
}

impl Default for ContractRules {
  fn default() -> Self {
    Self::with_defaults()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_contract_rules_creation() {
    let rules = ContractRules::new();
    assert!(rules.rules.is_empty());
  }

  #[test]
  fn test_contract_rules_with_defaults() {
    let rules = ContractRules::with_defaults();
    assert!(rules.contains("s2_reference_closure"));
    assert!(rules.contains("s3_contract_verification"));
  }

  #[test]
  fn test_contract_rules_enabled_for_stage() {
    let rules = ContractRules::with_defaults();
    let stage1_rules = rules.enabled_for_stage(1);
    assert!(!stage1_rules.is_empty());

    let stage2_rules = rules.enabled_for_stage(2);
    assert!(stage2_rules.len() > stage1_rules.len());
  }
}
