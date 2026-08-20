//! BuiltinRegistry 구조 정의
//!
//! pnix-old의 pnix_runtime_builtins/src/lib.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 실행 로직 제외
//! - BuiltinRegistry: 빌트인 함수 레지스트리 구조 정의
//! - BuiltinFunction: 빌트인 함수 메타데이터
//! - 실제 함수 호출 및 값 계산은 executor에서 구현

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 빌트인 함수 메타데이터
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuiltinFunction {
  /// 함수 이름
  pub name: String,
  /// 함수 설명
  pub description: String,
  /// 파라미터 개수 (선택적, -1이면 가변)
  pub arity: Option<usize>,
  /// 함수 카테고리
  pub category: BuiltinCategory,
}

/// 빌트인 함수 카테고리
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuiltinCategory {
  /// 수학 연산
  Math,
  /// 문자열 연산
  String,
  /// IO 연산
  Io,
  /// 비교 연산
  Comparison,
  /// 논리 연산
  Logic,
  /// 타입 변환
  TypeConversion,
}

/// 빌트인 함수 레지스트리 구조
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 정의만, 실행 로직 제외
/// - functions: 함수 메타데이터 맵 (구조 정의)
/// - 실제 함수 호출 및 값 계산은 executor에서 구현
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuiltinRegistry {
  /// 등록된 빌트인 함수들 (메타데이터만)
  pub functions: HashMap<String, BuiltinFunction>,
}

impl BuiltinRegistry {
  /// 빈 레지스트리 생성
  pub fn new() -> Self {
    Self {
      functions: HashMap::new(),
    }
  }

  /// 빌트인 함수 등록 (메타데이터만)
  pub fn register(&mut self, func: BuiltinFunction) {
    self.functions.insert(func.name.clone(), func);
  }

  /// 함수 메타데이터 조회
  pub fn get(&self, name: &str) -> Option<&BuiltinFunction> {
    self.functions.get(name)
  }

  /// 등록된 함수 이름 목록 반환
  pub fn function_names(&self) -> Vec<String> {
    self.functions.keys().cloned().collect()
  }

  /// 함수가 등록되어 있는지 확인
  pub fn contains(&self, name: &str) -> bool {
    self.functions.contains_key(name)
  }
}

impl Default for BuiltinRegistry {
  fn default() -> Self {
    Self::new()
  }
}

// 헌법 준수 (P0-1): 실행 로직 제거
// 다음 함수들은 executor/runtime 계층에서 구현하세요:
// - call(name, args) -> String
// - register_defaults() (실제 함수 구현 등록)
// - builtin_add, builtin_sub 등 실제 함수 구현
//
// 이 함수들은 값 계산 및 실행을 수행하므로 pnix-core에서 제외됩니다.

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_builtin_registry_creation() {
    let registry = BuiltinRegistry::new();
    assert!(registry.functions.is_empty());
  }

  #[test]
  fn test_builtin_registry_register() {
    let mut registry = BuiltinRegistry::new();
    let func = BuiltinFunction {
      name: "add".to_string(),
      description: "Add two numbers".to_string(),
      arity: Some(2),
      category: BuiltinCategory::Math,
    };
    registry.register(func);
    assert!(registry.contains("add"));
  }

  #[test]
  fn test_builtin_function_names() {
    let mut registry = BuiltinRegistry::new();
    registry.register(BuiltinFunction {
      name: "add".to_string(),
      description: "Add".to_string(),
      arity: Some(2),
      category: BuiltinCategory::Math,
    });
    registry.register(BuiltinFunction {
      name: "sub".to_string(),
      description: "Subtract".to_string(),
      arity: Some(2),
      category: BuiltinCategory::Math,
    });
    let names = registry.function_names();
    assert!(names.contains(&"add".to_string()));
    assert!(names.contains(&"sub".to_string()));
  }
}
