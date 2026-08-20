//! CoreType: pnix-new 기본 타입 시스템
//!
//! pnix-old의 CTType을 그래프 워크플로우에 맞게 단순화.
//!
//! ## 설계 원칙
//!
//! 1. **Graph-oriented**: 노드 입출력 타입 표현
//! 2. **String-compatible**: DSL의 문자열 타입과 호환
//! 3. **CT Laws**: Identity, Composition 보장

use serde::{Deserialize, Serialize};
use std::fmt;

/// Maximum recursion depth for type parsing to prevent stack overflow
const MAX_TYPE_PARSE_DEPTH: usize = 64;

/// pnix-new 기본 타입
///
/// DSL에서 선언된 타입을 구조화된 형태로 표현.
/// 문자열 기반이지만 CT 법칙을 따름.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum CoreType {
  /// Unit type (빈 입출력)
  #[default]
  Unit,

  /// Named type (DSL에서 선언된 타입)
  /// e.g., "Position", "Velocity", "Time"
  Named(String),

  /// Product type (A × B)
  /// 다중 입력/출력을 위한 튜플
  Product(Box<CoreType>, Box<CoreType>),

  /// Arrow type (A → B)
  /// 함수 타입: A를 받아 B를 반환
  Arrow(Box<CoreType>, Box<CoreType>),

  /// Sum type (A + B)
  /// 조건부 분기의 결과 타입
  Sum(Box<CoreType>, Box<CoreType>),

  /// Optional type (A?)
  /// optional 노드의 출력
  Optional(Box<CoreType>),

  /// List type (`[A]`)
  /// 배치 처리용
  List(Box<CoreType>),

  /// Record type ({ field: Type, ... })
  /// 포트 기반 morphism 시그니처
  Record(Vec<(String, CoreType)>),

  /// Type variable (다형성)
  Var(String),

  /// Universal quantification (∀a. T)
  /// let-polymorphism을 위한 타입 추상화
  /// 예: `let id = λx.x in (id 1, id "hi")` → `id : ∀a. a → a`
  Forall {
    /// quantified 타입 변수들
    vars: Vec<String>,
    /// 본체 타입
    body: Box<CoreType>,
  },
}

impl CoreType {
  /// Unit 타입 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn unit() -> Self {
    CoreType::Unit
  }

  /// Named 타입 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn named(name: impl Into<String>) -> Self {
    CoreType::Named(name.into())
  }

  /// 문자열에서 파싱
  ///
  /// DSL 타입 문자열을 CoreType으로 변환.
  /// 복잡한 타입은 추후 파서 확장 필요.
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 파싱만, 값 계산 없음
  pub fn parse(s: &str) -> Self {
    Self::parse_with_depth(s, 0)
  }

  /// 깊이 제한이 있는 내부 파싱 헬퍼
  fn parse_with_depth(s: &str, depth: usize) -> Self {
    let s = s.trim();

    // Depth limit check to prevent stack overflow
    if depth > MAX_TYPE_PARSE_DEPTH {
      // Return as Named type when depth exceeded (fallback)
      return CoreType::Named(s.to_string());
    }

    if s.is_empty() || s == "()" || s == "Unit" {
      return CoreType::Unit;
    }

    // Optional: "Type?"
    if let Some(inner) = s.strip_suffix('?') {
      return CoreType::Optional(Box::new(Self::parse_with_depth(inner, depth + 1)));
    }

    // List: "[Type]"
    if s.starts_with('[') && s.ends_with(']') {
      let inner = &s[1..s.len() - 1];
      return CoreType::List(Box::new(Self::parse_with_depth(inner, depth + 1)));
    }

    // Arrow: "A -> B" (함수 타입, Product보다 우선)
    if let Some(pos) = s.find(" -> ") {
      let (a, b) = s.split_at(pos);
      return CoreType::Arrow(
        Box::new(Self::parse_with_depth(a, depth + 1)),
        Box::new(Self::parse_with_depth(&b[4..], depth + 1)),
      );
    }

    // Product: "A * B" or "(A, B)"
    if let Some(pos) = s.find(" * ") {
      let (a, b) = s.split_at(pos);
      return CoreType::Product(
        Box::new(Self::parse_with_depth(a, depth + 1)),
        Box::new(Self::parse_with_depth(&b[3..], depth + 1)),
      );
    }

    // Sum: "A | B"
    if let Some(pos) = s.find(" | ") {
      let (a, b) = s.split_at(pos);
      return CoreType::Sum(
        Box::new(Self::parse_with_depth(a, depth + 1)),
        Box::new(Self::parse_with_depth(&b[3..], depth + 1)),
      );
    }

    // Type variable: starts with lowercase
    if s.starts_with(|c: char| c.is_ascii_lowercase()) {
      return CoreType::Var(s.to_string());
    }

    // Named type (default)
    CoreType::Named(s.to_string())
  }

  /// Record 타입 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn record(fields: Vec<(impl Into<String>, CoreType)>) -> Self {
    CoreType::Record(fields.into_iter().map(|(k, v)| (k.into(), v)).collect())
  }

  /// Product 타입 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn product(a: CoreType, b: CoreType) -> Self {
    CoreType::Product(Box::new(a), Box::new(b))
  }

  /// Arrow 타입 생성 (함수 타입)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn arrow(input: CoreType, output: CoreType) -> Self {
    CoreType::Arrow(Box::new(input), Box::new(output))
  }

  /// 여러 타입의 Product
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn product_of(types: impl IntoIterator<Item = CoreType>) -> Self {
    types
      .into_iter()
      .reduce(|a, b| CoreType::Product(Box::new(a), Box::new(b)))
      .unwrap_or(CoreType::Unit)
  }

  /// Optional 타입 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn optional(inner: CoreType) -> Self {
    CoreType::Optional(Box::new(inner))
  }

  /// List 타입 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn list(inner: CoreType) -> Self {
    CoreType::List(Box::new(inner))
  }

  /// 타입이 Unit인지 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn is_unit(&self) -> bool {
    matches!(self, CoreType::Unit)
  }

  /// 타입이 Optional인지 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn is_optional(&self) -> bool {
    matches!(self, CoreType::Optional(_))
  }

  /// Optional 내부 타입 추출
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn unwrap_optional(&self) -> Option<&CoreType> {
    match self {
      CoreType::Optional(inner) => Some(inner),
      _ => None,
    }
  }

  /// Record 필드 조회
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn get_field(&self, name: &str) -> Option<&CoreType> {
    match self {
      CoreType::Record(fields) => fields.iter().find(|(n, _)| n == name).map(|(_, t)| t),
      _ => None,
    }
  }
}

impl fmt::Display for CoreType {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      CoreType::Unit => write!(f, "()"),
      CoreType::Named(name) => write!(f, "{}", name),
      CoreType::Product(a, b) => write!(f, "{} * {}", a, b),
      CoreType::Arrow(input, output) => write!(f, "{} -> {}", input, output),
      CoreType::Sum(a, b) => write!(f, "{} | {}", a, b),
      CoreType::Optional(inner) => write!(f, "{}?", inner),
      CoreType::List(inner) => write!(f, "[{}]", inner),
      CoreType::Record(fields) => {
        write!(f, "{{ ")?;
        for (i, (name, ty)) in fields.iter().enumerate() {
          if i > 0 {
            write!(f, ", ")?;
          }
          write!(f, "{}: {}", name, ty)?;
        }
        write!(f, " }}")
      }
      CoreType::Var(name) => write!(f, "{}", name),
      CoreType::Forall { vars, body } => {
        if vars.is_empty() {
          write!(f, "{}", body)
        } else {
          write!(f, "∀{}. {}", vars.join(", "), body)
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_parse_unit() {
    assert_eq!(CoreType::parse(""), CoreType::Unit);
    assert_eq!(CoreType::parse("()"), CoreType::Unit);
    assert_eq!(CoreType::parse("Unit"), CoreType::Unit);
  }

  #[test]
  fn test_parse_named() {
    assert_eq!(
      CoreType::parse("Position"),
      CoreType::Named("Position".into())
    );
    assert_eq!(
      CoreType::parse("Velocity"),
      CoreType::Named("Velocity".into())
    );
  }

  #[test]
  fn test_parse_optional() {
    assert_eq!(
      CoreType::parse("Position?"),
      CoreType::Optional(Box::new(CoreType::Named("Position".into())))
    );
  }

  #[test]
  fn test_parse_list() {
    assert_eq!(
      CoreType::parse("[Position]"),
      CoreType::List(Box::new(CoreType::Named("Position".into())))
    );
  }

  #[test]
  fn test_parse_product() {
    assert_eq!(
      CoreType::parse("Position * Velocity"),
      CoreType::Product(
        Box::new(CoreType::Named("Position".into())),
        Box::new(CoreType::Named("Velocity".into()))
      )
    );
  }

  #[test]
  fn test_parse_arrow() {
    assert_eq!(
      CoreType::parse("Int -> String"),
      CoreType::Arrow(
        Box::new(CoreType::Named("Int".into())),
        Box::new(CoreType::Named("String".into()))
      )
    );
  }

  #[test]
  fn test_parse_sum() {
    assert_eq!(
      CoreType::parse("Success | Error"),
      CoreType::Sum(
        Box::new(CoreType::Named("Success".into())),
        Box::new(CoreType::Named("Error".into()))
      )
    );
  }

  #[test]
  fn test_parse_var() {
    assert_eq!(CoreType::parse("a"), CoreType::Var("a".into()));
    assert_eq!(CoreType::parse("elem"), CoreType::Var("elem".into()));
  }

  #[test]
  fn test_display() {
    assert_eq!(CoreType::Unit.to_string(), "()");
    assert_eq!(CoreType::named("Pos").to_string(), "Pos");
    assert_eq!(
      CoreType::product(CoreType::named("A"), CoreType::named("B")).to_string(),
      "A * B"
    );
  }

  #[test]
  fn test_record() {
    let rec = CoreType::record(vec![
      ("x", CoreType::named("Float")),
      ("y", CoreType::named("Float")),
    ]);

    assert!(matches!(rec, CoreType::Record(_)));
    assert_eq!(rec.get_field("x"), Some(&CoreType::named("Float")));
    assert_eq!(rec.get_field("z"), None);
  }
}
