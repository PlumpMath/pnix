//! Morphism Registry 구조 정의
//!
//! pnix-old의 ct_morphism/src/lib.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 값 계산 함수 제외
//! - `MorphismRegistry::apply()` - 값 계산 함수 제외 (executor에서 구현)
//! - `MorphismRegistry::compose()` - 값 계산 함수 제외 (executor에서 구현)
//! - `ComposedMorphism::apply()` - 값 계산 함수 제외 (executor에서 구현)
//!
//! **주의**: MorphismRegistry는 `Arc<RwLock<HashMap>>`을 사용하지만,
//! 이것은 구조 정의일 뿐이며 실제 등록/조회/적용은 executor에서 수행됩니다.

use serde::{Deserialize, Serialize};

/// Morphism information (for introspection)
///
/// Morphism의 메타데이터를 저장하는 구조체입니다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MorphismInfo {
  pub name: String,
  pub domain: String,
  pub codomain: String,
  pub implementation: Option<String>,
}

/// Registry for managing morphisms
///
/// **주의**: 실제 morphism 등록/조회/적용은 executor에서 수행됩니다.
/// pnix-core에서는 구조 정의만 포함합니다.
///
/// # Executor 구현
///
/// 다음 메서드들은 executor에서 구현됩니다:
/// - `new() -> Self`: 새 레지스트리 생성
/// - `with_defaults() -> Self`: 기본 morphism 포함 레지스트리 생성
/// - `register(&self, morphism: Box<dyn Morphism>)`: Morphism 등록
/// - `unregister(&self, name: &str) -> Option<Box<dyn Morphism>>`: Morphism 제거
/// - `contains(&self, name: &str) -> bool`: Morphism 존재 확인
/// - `get_info(&self, name: &str) -> Option<MorphismInfo>`: Morphism 정보 조회
/// - `apply(&self, name: &str, input: &CtValue) -> MorphismResult`: Morphism 적용 (값 계산)
/// - `compose(&self, g_name: &str, f_name: &str) -> Result<ComposedMorphism, MorphismError>`: Morphism 합성
/// - `list_morphisms(&self) -> Vec<String>`: 등록된 morphism 목록
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MorphismRegistry {
  /// Morphism 이름 목록 (실제 HashMap은 executor에서 관리)
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub morphism_names: Vec<String>,
}

impl MorphismRegistry {
  /// 새 레지스트리 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new() -> Self {
    Self {
      morphism_names: Vec::new(),
    }
  }

  /// 기본 morphism 포함 레지스트리 생성
  ///
  /// **주의**: 실제 기본 morphism 등록은 executor에서 수행됩니다.
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn with_defaults() -> Self {
    Self::new()
  }
}

impl Default for MorphismRegistry {
  fn default() -> Self {
    Self::with_defaults()
  }
}

/// Composed morphism: g ∘ f
///
/// 두 morphism의 합성을 나타내는 구조체입니다.
///
/// # Executor 구현
///
/// 다음 메서드들은 executor에서 구현됩니다:
/// - `apply(&self, input: &CtValue) -> MorphismResult`: 합성 morphism 적용 (값 계산)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposedMorphism {
  /// 첫 번째 morphism 이름
  pub f: String,
  /// 두 번째 morphism 이름
  pub g: String,
  /// Cached domain from f
  pub domain: String,
  /// Cached codomain from g
  pub codomain: String,
}

impl ComposedMorphism {
  /// 새 합성 morphism 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(f: String, g: String, domain: String, codomain: String) -> Self {
    Self {
      f,
      g,
      domain,
      codomain,
    }
  }
}
