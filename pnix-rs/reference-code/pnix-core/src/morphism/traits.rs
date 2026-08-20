//! Morphism Trait 정의
//!
//! pnix-old의 ct_morphism/src/lib.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! **중요**: Trait 메서드 시그니처는 문서화 목적으로만 포함됩니다.
//! 실제 구현(값 계산)은 executor에서 수행됩니다.
//!
//! ## 설계 원칙
//!
//! A morphism in Category Theory represents a structure-preserving map between objects.
//! In Pnix, morphisms are functions with explicit domain and codomain types.

/// Trait for all morphisms
///
/// A morphism in Category Theory represents a structure-preserving map between objects.
/// In Pnix, morphisms are functions with explicit domain and codomain types.
///
/// # Executor 구현
///
/// 다음 메서드들은 executor에서 구현됩니다:
/// - `name() -> &str`: Morphism 이름 반환
/// - `domain() -> &str`: 입력 타입 (domain) 반환
/// - `codomain() -> &str`: 출력 타입 (codomain) 반환
/// - `apply(&self, input: &CtValue) -> MorphismResult`: Morphism 적용 (값 계산)
/// - `implementation() -> Option<String>`: 구현 코드 (디버깅/시각화용, 선택적)
/// - `validate_input(&self, input: &CtValue) -> Result<(), MorphismError>`: 입력 검증 (선택적)
pub trait Morphism: Send + Sync {
  /// The name of this morphism
  /// **구현 위치**: executor
  fn name(&self) -> &str;

  /// The domain (input type) of this morphism
  /// **구현 위치**: executor
  fn domain(&self) -> &str;

  /// The codomain (output type) of this morphism
  /// **구현 위치**: executor
  fn codomain(&self) -> &str;

  /// Apply this morphism to an input value
  ///
  /// **주의**: 이 메서드는 값 계산이므로 executor에서만 구현됩니다.
  /// pnix-core에서는 시그니처만 문서화합니다.
  /// **구현 위치**: executor
  fn apply(
    &self,
    input: &serde_json::Value,
  ) -> Result<serde_json::Value, crate::morphism::MorphismError>;

  /// Optional: Get the implementation code (for debugging/visualization)
  /// **구현 위치**: executor
  fn implementation(&self) -> Option<String> {
    None
  }

  /// Optional: Validate input before applying
  /// **구현 위치**: executor
  fn validate_input(
    &self,
    _input: &serde_json::Value,
  ) -> Result<(), crate::morphism::MorphismError> {
    // Default: accept any input
    Ok(())
  }
}
