//! Lowering rules registry (data only)
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 실행 로직 제외
//! lowering 패스 자체는 `passes/lowering.rs`에 있지만,
//! 규칙(어떤 변환이 가능한지)은 여기에 데이터로 선언

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Lowering 규칙 종류
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoweringRuleKind {
  /// AST → Surface 변환
  AstToSurface,
  /// Surface → FxCore 변환
  SurfaceToFxCore,
  /// Builtin → FxMorphism 매핑
  BuiltinToMorphism,
  /// 타입 변환 규칙
  TypeConversion,
}

/// Lowering 규칙 선언
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoweringRule {
  /// 규칙 이름
  pub name: String,
  /// 규칙 종류
  pub kind: LoweringRuleKind,
  /// 소스 패턴 (예: "builtin:add")
  pub source_pattern: String,
  /// 타겟 패턴 (예: "morphism:fx_add")
  pub target_pattern: String,
  /// 설명
  pub description: String,
}

/// Lowering 규칙 레지스트리
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoweringRules {
  /// 등록된 규칙들 (이름 → 규칙)
  pub rules: BTreeMap<String, LoweringRule>,
}

impl LoweringRules {
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

    // Builtin → FxMorphism 매핑
    registry.register(LoweringRule {
      name: "builtin_add_to_morphism".to_string(),
      kind: LoweringRuleKind::BuiltinToMorphism,
      source_pattern: "builtin:add".to_string(),
      target_pattern: "morphism:fx_add".to_string(),
      description: "Map builtin add to fx_add morphism".to_string(),
    });
    registry.register(LoweringRule {
      name: "builtin_sub_to_morphism".to_string(),
      kind: LoweringRuleKind::BuiltinToMorphism,
      source_pattern: "builtin:sub".to_string(),
      target_pattern: "morphism:fx_sub".to_string(),
      description: "Map builtin sub to fx_sub morphism".to_string(),
    });
    registry.register(LoweringRule {
      name: "builtin_mul_to_morphism".to_string(),
      kind: LoweringRuleKind::BuiltinToMorphism,
      source_pattern: "builtin:mul".to_string(),
      target_pattern: "morphism:fx_mul".to_string(),
      description: "Map builtin mul to fx_mul morphism".to_string(),
    });
    registry.register(LoweringRule {
      name: "builtin_div_to_morphism".to_string(),
      kind: LoweringRuleKind::BuiltinToMorphism,
      source_pattern: "builtin:div".to_string(),
      target_pattern: "morphism:fx_div".to_string(),
      description: "Map builtin div to fx_div morphism".to_string(),
    });
    registry.register(LoweringRule {
      name: "builtin_sin_to_morphism".to_string(),
      kind: LoweringRuleKind::BuiltinToMorphism,
      source_pattern: "builtin:sin".to_string(),
      target_pattern: "morphism:fx_sin".to_string(),
      description: "Map builtin sin to fx_sin morphism".to_string(),
    });
    registry.register(LoweringRule {
      name: "builtin_cos_to_morphism".to_string(),
      kind: LoweringRuleKind::BuiltinToMorphism,
      source_pattern: "builtin:cos".to_string(),
      target_pattern: "morphism:fx_cos".to_string(),
      description: "Map builtin cos to fx_cos morphism".to_string(),
    });

    registry
  }

  /// 규칙 등록
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn register(&mut self, rule: LoweringRule) {
    // LOW: lowering rule target_pattern 검증 없음 수정
    // target_pattern이 비어있거나 유효하지 않은 형식인지 검증
    if rule.target_pattern.is_empty() {
      eprintln!(
        "Warning: LoweringRule '{}' has empty target_pattern",
        rule.name
      );
    } else {
      // 기본 패턴 형식 검증: "morphism:name" 또는 "type:name" 형식
      let is_valid_pattern = rule.target_pattern.contains(':')
        || rule.target_pattern.starts_with("fx_")
        || rule
          .target_pattern
          .chars()
          .all(|c| c.is_alphanumeric() || c == '_');
      if !is_valid_pattern {
        eprintln!("Warning: LoweringRule '{}' has potentially invalid target_pattern '{}' (expected format: 'morphism:name' or 'type:name')", rule.name, rule.target_pattern);
      }
    }
    self.rules.insert(rule.name.clone(), rule);
  }

  /// 규칙 조회
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn get(&self, name: &str) -> Option<&LoweringRule> {
    self.rules.get(name)
  }

  /// 패턴으로 규칙 찾기 (source_pattern 매칭)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 필터링만, 값 계산 없음
  pub fn find_by_source(&self, source: &str) -> Vec<&LoweringRule> {
    self
      .rules
      .values()
      .filter(|r| r.source_pattern == source)
      .collect()
  }

  /// 규칙 존재 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn contains(&self, name: &str) -> bool {
    self.rules.contains_key(name)
  }
}

impl Default for LoweringRules {
  fn default() -> Self {
    Self::with_defaults()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_lowering_rules_creation() {
    let rules = LoweringRules::new();
    assert!(rules.rules.is_empty());
  }

  #[test]
  fn test_lowering_rules_with_defaults() {
    let rules = LoweringRules::with_defaults();
    assert!(rules.contains("builtin_add_to_morphism"));
  }

  #[test]
  fn test_lowering_rules_find_by_source() {
    let rules = LoweringRules::with_defaults();
    let found = rules.find_by_source("builtin:add");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name, "builtin_add_to_morphism");
  }
}
