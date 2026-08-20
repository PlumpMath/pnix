//! GH Codegen 구조 정의
//!
//! pnix-old의 pnix_gh_codegen/src/lib.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 실행 로직 제외
//! - GhNodeType: GH 노드 타입 enum 정의
//! - GhNode: GH 노드 구조 정의
//! - GhValue: GH 값 타입 enum 정의
//! - 실제 코드 생성 로직은 executor에서 구현

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// GH 노드 타입
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GhNodeType {
  /// 수학 연산 (+, -, *, / 등)
  Math,
  /// 리스트 연산 (map, filter, reduce 등)
  List,
  /// 조건부 (if, switch 등)
  Conditional,
  /// 함수 정의
  Function,
  /// 모나드 연산 (>>=, fmap 등)
  Monad,
  /// 펑터 연산
  Functor,
  /// 커스텀 컴포넌트
  Custom(String),
}

/// GH 노드 구조 (순수 데이터)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GhNode {
  /// 노드 ID
  pub id: String,
  /// 노드 타입
  pub node_type: GhNodeType,
  /// 입력값들
  pub inputs: HashMap<String, GhValue>,
  /// 출력값들
  pub outputs: HashMap<String, GhValue>,
}

impl GhNode {
  /// 새로운 GH 노드 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(id: impl Into<String>, node_type: GhNodeType) -> Self {
    Self {
      id: id.into(),
      node_type,
      inputs: HashMap::new(),
      outputs: HashMap::new(),
    }
  }

  /// 입력 추가
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변경만, 값 계산 없음
  pub fn with_input(mut self, name: impl Into<String>, value: GhValue) -> Self {
    self.inputs.insert(name.into(), value);
    self
  }

  /// 출력 추가
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변경만, 값 계산 없음
  pub fn with_output(mut self, name: impl Into<String>, value: GhValue) -> Self {
    self.outputs.insert(name.into(), value);
    self
  }
}

/// GH 값 타입
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GhValue {
  /// 숫자 값
  Number(f64),
  /// 정수 값
  Integer(i64),
  /// 문자열 값
  String(String),
  /// 불리언 값
  Boolean(bool),
  /// 리스트 값
  List(Vec<GhValue>),
  /// 지오메트리 값 (문자열로 표현)
  Geometry(String),
  /// 함수 값 (문자열로 표현)
  Function(String),
}

impl GhValue {
  /// 숫자 값 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn number(n: f64) -> Self {
    Self::Number(n)
  }

  /// 정수 값 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn integer(i: i64) -> Self {
    Self::Integer(i)
  }

  /// 문자열 값 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn string(s: impl Into<String>) -> Self {
    Self::String(s.into())
  }

  /// 불리언 값 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn boolean(b: bool) -> Self {
    Self::Boolean(b)
  }
}

// 헌법 준수 (P0-1): 실행 로직 제거
// 다음 함수들은 executor/runtime 계층에서 구현하세요:
// - GhCodeGenerator 구조체 및 메서드들 (코드 생성 로직)
// - convert_node(), convert_math_node() 등 (노드 변환 로직)
// - gh_value_to_pnix() (값 변환 로직)
//
// 이 함수들은 코드 생성, 값 변환, 또는 실행 로직을 수행하므로 pnix-core에서 제외됩니다.

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_gh_node_creation() {
    let node = GhNode::new("node1", GhNodeType::Math);
    assert_eq!(node.id, "node1");
    assert_eq!(node.node_type, GhNodeType::Math);
  }

  #[test]
  fn test_gh_node_with_input() {
    let node = GhNode::new("node1", GhNodeType::Math).with_input("a", GhValue::number(10.0));
    assert_eq!(node.inputs.get("a"), Some(&GhValue::Number(10.0)));
  }

  #[test]
  fn test_gh_value_creation() {
    let val = GhValue::number(42.0);
    assert_eq!(val, GhValue::Number(42.0));
  }
}
