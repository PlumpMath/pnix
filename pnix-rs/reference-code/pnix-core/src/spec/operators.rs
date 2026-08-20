//! Operator registry (data only)
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 실행 로직 제외
//! 연산자 레지스트리는 데이터 구조만 포함합니다.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

fn default_arity() -> usize {
  2
}

/// 연산자 결합성: 연산자의 결합 방향 타입
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OperatorFixity {
  /// 좌결합 중위 연산자 (예: a - b - c = (a - b) - c)
  Infixl,
  /// 우결합 중위 연산자 (예: a ^ b ^ c = a ^ (b ^ c))
  Infixr,
  /// 중위 연산자 (결합성 없음)
  Infix,
  /// 전위 연산자 (예: -x)
  Prefix,
  /// 후위 연산자 (예: x!)
  Postfix,
}

/// 연산자 법칙 항목: 연산자 법칙 참조 항목 구조
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperatorLawEntry {
  /// 법칙 이름
  pub name: String,
  /// 선택적 법칙 여부
  #[serde(default)]
  pub optional: bool,
}

/// 연산자 법칙 참조: 연산자 법칙 참조 타입
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum OperatorLawRef {
  /// 법칙 이름 (문자열)
  Name(String),
  /// 법칙 항목 (이름과 선택적 플래그 포함)
  Entry(OperatorLawEntry),
}

impl OperatorLawRef {
  /// 법칙 이름 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn name(&self) -> &str {
    match self {
      OperatorLawRef::Name(name) => name,
      OperatorLawRef::Entry(entry) => &entry.name,
    }
  }

  /// 선택적 법칙 여부 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn optional(&self) -> bool {
    match self {
      OperatorLawRef::Name(_) => false,
      OperatorLawRef::Entry(entry) => entry.optional,
    }
  }
}

/// 연산자 명세: 연산자 정의 구조
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperatorSpec {
  /// 연산자 토큰 (예: "+", "-", "*")
  pub token: String,
  /// 연산자 결합성
  pub fixity: OperatorFixity,
  /// 우선순위 (높을수록 먼저 평가)
  pub precedence: u8,
  /// 인자 개수 (기본값: 2)
  #[serde(default = "default_arity")]
  pub arity: usize,
  /// 디슈가 패턴 (예: "add(a, b)")
  pub desugar: String,
  /// 타입 시그니처 (예: "Num → Num → Num")
  pub typing: String,
  /// 적용되는 법칙 목록
  pub laws: Vec<OperatorLawRef>,
  /// 정규화 패턴 (선택적)
  #[serde(default)]
  pub normalize: Option<String>,
}

/// 연산자 카탈로그: 등록된 모든 연산자의 카탈로그
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorCatalog {
  /// 등록된 연산자들 (토큰 → 명세 매핑)
  pub operators: BTreeMap<String, OperatorSpec>,
}

impl OperatorCatalog {
  /// 빈 카탈로그 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new() -> Self {
    Self {
      operators: BTreeMap::new(),
    }
  }

  /// 기본 카탈로그 생성 (현재는 빈 카탈로그 반환)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn with_defaults() -> Self {
    Self::new()
  }

  /// 연산자 등록
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn register(&mut self, spec: OperatorSpec) {
    self.operators.insert(spec.token.clone(), spec);
  }

  /// 연산자 조회
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn get(&self, token: &str) -> Option<&OperatorSpec> {
    self.operators.get(token)
  }

  /// 연산자 존재 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn contains(&self, token: &str) -> bool {
    self.operators.contains_key(token)
  }
}

impl Default for OperatorCatalog {
  fn default() -> Self {
    Self::with_defaults()
  }
}
