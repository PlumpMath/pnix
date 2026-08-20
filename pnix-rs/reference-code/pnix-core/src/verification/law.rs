//! Law types for verification layer.

/// 법칙 종류: Category Theory 법칙의 종류 타입
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LawKind {
  /// Functor Identity 법칙
  FunctorIdentity,
  /// Functor Composition 법칙
  FunctorComposition,
  /// Monad Left Identity 법칙
  MonadLeftIdentity,
  /// Monad Right Identity 법칙
  MonadRightIdentity,
  /// Monad Associativity 법칙
  MonadAssociativity,
  /// Natural Transformation 법칙
  NaturalTransformation,
  /// 사용자 정의 법칙
  Custom(
    /// 법칙 이름
    String
  ),
}

/// 법칙: Category Theory 법칙 정의
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Law {
  /// 법칙 종류
  pub kind: LawKind,
  /// 법칙 이름
  pub name: String,
  /// 법칙 공식 (문자열 표현)
  pub formula: String,
  /// 검증 완료 여부
  pub verified: bool,
}
