//! CT Tags - Category Theory 메타데이터 태그
//!
//! pnix-old의 symbolic_core/src/ct/tags.rs에서 마이그레이션
//!
//! ## 헌법 준수 (P0-1)
//!
//! 순수 구조 타입, 값 연산 없음
//!
//! ## 사용 목적
//!
//! - Morphism에 카테고리/도메인/단위 정보 부착
//! - 타입 안전한 물리량 추적
//! - CT 타입 시스템 확장

use serde::{Deserialize, Serialize};

/// Category Theory 카테고리 종류
///
/// 수학적 공간의 분류를 나타냅니다.
///
/// # 변형
/// - `ScalarField`: 스칼라장 (R, C)
/// - `VectorSpace`: 벡터공간 (R^n)
/// - `MatrixSpace`: 행렬공간 (M(n,m))
/// - `TensorSpace`: 텐서공간 (T^{ij}_{kl})
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CtCategory {
  /// 스칼라장 (R, C)
  ScalarField,
  /// 벡터공간 (R^n)
  VectorSpace,
  /// 행렬공간 (M(n,m))
  MatrixSpace,
  /// 텐서공간 (T^{ij}_{kl})
  TensorSpace,
}

impl CtCategory {
  /// 카테고리 이름 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn name(&self) -> &'static str {
    match self {
      CtCategory::ScalarField => "ScalarField",
      CtCategory::VectorSpace => "VectorSpace",
      CtCategory::MatrixSpace => "MatrixSpace",
      CtCategory::TensorSpace => "TensorSpace",
    }
  }

  /// 스칼라인지 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn is_scalar(&self) -> bool {
    matches!(self, CtCategory::ScalarField)
  }

  /// 텐서 계열인지 확인 (벡터, 행렬, 텐서)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn is_tensor_like(&self) -> bool {
    matches!(
      self,
      CtCategory::VectorSpace | CtCategory::MatrixSpace | CtCategory::TensorSpace
    )
  }
}

impl std::fmt::Display for CtCategory {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.name())
  }
}

/// CT 태그 - Morphism에 부착되는 메타데이터
///
/// # 필드
/// - `category`: 카테고리 종류 (선택)
/// - `domain`: 정의역 표기 (선택, "R", "R^3", "M" 등)
/// - `codomain`: 공역 표기 (선택)
/// - `unit`: 물리 단위 (선택, "m", "s", "N", "J" 등)
///
/// # 예시
/// ```ignore
/// use pnix_core::ct::CtTag;
///
/// let tag = CtTag::scalar().with_unit("m/s");
/// assert!(tag.category.is_some());
/// assert_eq!(tag.unit.as_deref(), Some("m/s"));
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CtTag {
  /// 카테고리 종류
  #[serde(skip_serializing_if = "Option::is_none")]
  pub category: Option<CtCategory>,
  /// 정의역 표기 ("R", "R^3", "M" 등)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub domain: Option<String>,
  /// 공역 표기
  #[serde(skip_serializing_if = "Option::is_none")]
  pub codomain: Option<String>,
  /// 물리 단위 ("m", "s", "N", "J" 등)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub unit: Option<String>,
}

impl CtTag {
  /// 빈 태그 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn empty() -> Self {
    Self::default()
  }

  /// 스칼라 태그 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn scalar() -> Self {
    Self {
      category: Some(CtCategory::ScalarField),
      ..Default::default()
    }
  }

  /// 벡터 태그 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn vector() -> Self {
    Self {
      category: Some(CtCategory::VectorSpace),
      ..Default::default()
    }
  }

  /// 행렬 태그 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn matrix() -> Self {
    Self {
      category: Some(CtCategory::MatrixSpace),
      ..Default::default()
    }
  }

  /// 텐서 태그 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn tensor() -> Self {
    Self {
      category: Some(CtCategory::TensorSpace),
      ..Default::default()
    }
  }

  /// 단위 추가
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 설정만, 값 계산 없음
  pub fn with_unit(mut self, unit: impl Into<String>) -> Self {
    self.unit = Some(unit.into());
    self
  }

  /// 정의역 추가
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 설정만, 값 계산 없음
  pub fn with_domain(mut self, domain: impl Into<String>) -> Self {
    self.domain = Some(domain.into());
    self
  }

  /// 공역 추가
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 설정만, 값 계산 없음
  pub fn with_codomain(mut self, codomain: impl Into<String>) -> Self {
    self.codomain = Some(codomain.into());
    self
  }

  /// 카테고리 추가
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 설정만, 값 계산 없음
  pub fn with_category(mut self, category: CtCategory) -> Self {
    self.category = Some(category);
    self
  }

  /// 단위가 같은지 확인
  ///
  /// 둘 다 None이면 true, 둘 다 Some이고 같으면 true
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn unit_matches(&self, other: &CtTag) -> bool {
    match (&self.unit, &other.unit) {
      (Some(a), Some(b)) => a == b,
      (None, None) => true,
      _ => false, // 하나만 있으면 불일치
    }
  }

  /// 정의역이 같은지 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn domain_matches(&self, other: &CtTag) -> bool {
    match (&self.domain, &other.domain) {
      (Some(a), Some(b)) => a == b,
      (None, None) => true,
      _ => false,
    }
  }

  /// 카테고리가 같은지 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn category_matches(&self, other: &CtTag) -> bool {
    match (&self.category, &other.category) {
      (Some(a), Some(b)) => a == b,
      (None, None) => true,
      _ => false,
    }
  }

  /// 태그가 비어있는지 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn is_empty(&self) -> bool {
    self.category.is_none()
      && self.domain.is_none()
      && self.codomain.is_none()
      && self.unit.is_none()
  }

  /// 두 태그 병합 (self 우선)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn merge(&self, other: &CtTag) -> Self {
    Self {
      category: self.category.clone().or_else(|| other.category.clone()),
      domain: self.domain.clone().or_else(|| other.domain.clone()),
      codomain: self.codomain.clone().or_else(|| other.codomain.clone()),
      unit: self.unit.clone().or_else(|| other.unit.clone()),
    }
  }
}

// ─────────────────────────────────────────────
// CtContext - Variable → Tag Mapping
// ─────────────────────────────────────────────

/// CT 컨텍스트 - 변수별 CtTag 바인딩
///
/// # 용도
/// - 변수에 카테고리/단위 정보 부착
/// - CT 검증 시 컨텍스트 참조
/// - 물리량 타입 추적
///
/// # 예시
/// ```ignore
/// use pnix_core::ct::CtContext;
///
/// let mut ctx = CtContext::new();
/// ctx.bind("velocity", CtTag::scalar().with_unit("m/s"));
/// assert!(ctx.get("velocity").is_some());
/// ```
#[derive(Clone, Debug, Default)]
pub struct CtContext {
  bindings: std::collections::HashMap<String, CtTag>,
}

impl CtContext {
  /// 새 컨텍스트 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new() -> Self {
    Self::default()
  }

  /// 변수에 태그 바인딩
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 설정만, 값 계산 없음
  pub fn bind(&mut self, var: impl Into<String>, tag: CtTag) {
    self.bindings.insert(var.into(), tag);
  }

  /// 변수의 태그 조회
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn get(&self, var: &str) -> Option<&CtTag> {
    self.bindings.get(var)
  }

  /// 변수의 태그 제거
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 수정만, 값 계산 없음
  pub fn remove(&mut self, var: &str) -> Option<CtTag> {
    self.bindings.remove(var)
  }

  /// 바인딩 개수 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn len(&self) -> usize {
    self.bindings.len()
  }

  /// 비어있는지 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn is_empty(&self) -> bool {
    self.bindings.is_empty()
  }

  /// 모든 바인딩 순회
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn iter(&self) -> impl Iterator<Item = (&String, &CtTag)> {
    self.bindings.iter()
  }

  /// 물리 교육용 기본 컨텍스트 생성
  ///
  /// 일반적인 물리량 변수들의 단위를 미리 바인딩:
  /// - x: 위치 (m)
  /// - v: 속도 (m/s)
  /// - a: 가속도 (m/s^2)
  /// - t: 시간 (s)
  /// - m: 질량 (kg)
  /// - F: 힘 (N)
  /// - E: 에너지 (J)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn physics_default() -> Self {
    let mut ctx = Self::new();
    ctx.bind("x", CtTag::scalar().with_unit("m"));
    ctx.bind("v", CtTag::scalar().with_unit("m/s"));
    ctx.bind("a", CtTag::scalar().with_unit("m/s^2"));
    ctx.bind("t", CtTag::scalar().with_unit("s"));
    ctx.bind("m", CtTag::scalar().with_unit("kg"));
    ctx.bind("F", CtTag::scalar().with_unit("N"));
    ctx.bind("E", CtTag::scalar().with_unit("J"));
    ctx
  }

  /// 두 컨텍스트 병합 (self가 우선)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn merge(&self, other: &CtContext) -> Self {
    let mut result = other.clone();
    for (var, tag) in &self.bindings {
      result.bindings.insert(var.clone(), tag.clone());
    }
    result
  }
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_ct_category_name() {
    assert_eq!(CtCategory::ScalarField.name(), "ScalarField");
    assert_eq!(CtCategory::VectorSpace.name(), "VectorSpace");
    assert_eq!(CtCategory::MatrixSpace.name(), "MatrixSpace");
    assert_eq!(CtCategory::TensorSpace.name(), "TensorSpace");
  }

  #[test]
  fn test_ct_category_is_scalar() {
    assert!(CtCategory::ScalarField.is_scalar());
    assert!(!CtCategory::VectorSpace.is_scalar());
    assert!(!CtCategory::MatrixSpace.is_scalar());
    assert!(!CtCategory::TensorSpace.is_scalar());
  }

  #[test]
  fn test_ct_category_is_tensor_like() {
    assert!(!CtCategory::ScalarField.is_tensor_like());
    assert!(CtCategory::VectorSpace.is_tensor_like());
    assert!(CtCategory::MatrixSpace.is_tensor_like());
    assert!(CtCategory::TensorSpace.is_tensor_like());
  }

  #[test]
  fn test_ct_category_display() {
    assert_eq!(format!("{}", CtCategory::ScalarField), "ScalarField");
    assert_eq!(format!("{}", CtCategory::TensorSpace), "TensorSpace");
  }

  #[test]
  fn test_ct_tag_empty() {
    let tag = CtTag::empty();
    assert!(tag.is_empty());
    assert!(tag.category.is_none());
    assert!(tag.domain.is_none());
    assert!(tag.unit.is_none());
  }

  #[test]
  fn test_ct_tag_scalar() {
    let tag = CtTag::scalar();
    assert!(!tag.is_empty());
    assert_eq!(tag.category, Some(CtCategory::ScalarField));
  }

  #[test]
  fn test_ct_tag_builders() {
    let tag = CtTag::vector().with_domain("R^3").with_unit("m");
    assert_eq!(tag.category, Some(CtCategory::VectorSpace));
    assert_eq!(tag.domain.as_deref(), Some("R^3"));
    assert_eq!(tag.unit.as_deref(), Some("m"));
  }

  #[test]
  fn test_ct_tag_unit_matches() {
    let a = CtTag::scalar().with_unit("m");
    let b = CtTag::scalar().with_unit("m");
    let c = CtTag::scalar().with_unit("s");
    let d = CtTag::scalar(); // no unit

    assert!(a.unit_matches(&b));
    assert!(!a.unit_matches(&c));
    assert!(!a.unit_matches(&d));

    let e = CtTag::scalar();
    assert!(e.unit_matches(&d)); // both None
  }

  #[test]
  fn test_ct_tag_domain_matches() {
    let a = CtTag::vector().with_domain("R^3");
    let b = CtTag::vector().with_domain("R^3");
    let c = CtTag::vector().with_domain("R^4");

    assert!(a.domain_matches(&b));
    assert!(!a.domain_matches(&c));
  }

  #[test]
  fn test_ct_tag_category_matches() {
    let a = CtTag::scalar();
    let b = CtTag::scalar();
    let c = CtTag::vector();

    assert!(a.category_matches(&b));
    assert!(!a.category_matches(&c));
  }

  #[test]
  fn test_ct_tag_merge() {
    let a = CtTag::scalar().with_unit("m");
    let b = CtTag::empty().with_domain("R").with_codomain("R");

    let merged = a.merge(&b);
    assert_eq!(merged.category, Some(CtCategory::ScalarField));
    assert_eq!(merged.unit.as_deref(), Some("m"));
    assert_eq!(merged.domain.as_deref(), Some("R"));
    assert_eq!(merged.codomain.as_deref(), Some("R"));
  }

  #[test]
  fn test_ct_tag_serde_roundtrip() {
    let tag = CtTag::vector().with_domain("R^3").with_unit("N");
    let json = serde_json::to_string(&tag).unwrap();
    let restored: CtTag = serde_json::from_str(&json).unwrap();
    assert_eq!(tag, restored);
  }

  #[test]
  fn test_ct_tag_serde_skip_none() {
    let tag = CtTag::scalar();
    let json = serde_json::to_string(&tag).unwrap();
    // domain, codomain, unit이 None이므로 JSON에 포함되지 않아야 함
    assert!(!json.contains("domain"));
    assert!(!json.contains("codomain"));
    assert!(!json.contains("unit"));
    assert!(json.contains("category"));
  }

  #[test]
  fn test_ct_category_serde_snake_case() {
    let tag = CtTag::scalar();
    let json = serde_json::to_string(&tag).unwrap();
    assert!(json.contains("scalar_field")); // snake_case 직렬화
  }

  // ─────────────────────────────────────────────
  // CtContext tests
  // ─────────────────────────────────────────────

  #[test]
  fn test_ct_context_new() {
    let ctx = CtContext::new();
    assert!(ctx.is_empty());
    assert_eq!(ctx.len(), 0);
  }

  #[test]
  fn test_ct_context_bind_get() {
    let mut ctx = CtContext::new();
    ctx.bind("x", CtTag::scalar().with_unit("m"));

    let tag = ctx.get("x");
    assert!(tag.is_some());
    assert_eq!(tag.unwrap().unit.as_deref(), Some("m"));

    assert!(ctx.get("y").is_none());
  }

  #[test]
  fn test_ct_context_remove() {
    let mut ctx = CtContext::new();
    ctx.bind("x", CtTag::scalar());

    assert_eq!(ctx.len(), 1);
    let removed = ctx.remove("x");
    assert!(removed.is_some());
    assert_eq!(ctx.len(), 0);
  }

  #[test]
  fn test_ct_context_physics_default() {
    let ctx = CtContext::physics_default();
    assert!(!ctx.is_empty());
    assert!(ctx.len() >= 7); // x, v, a, t, m, F, E

    let v = ctx.get("v").unwrap();
    assert_eq!(v.unit.as_deref(), Some("m/s"));

    let f = ctx.get("F").unwrap();
    assert_eq!(f.unit.as_deref(), Some("N"));
  }

  #[test]
  fn test_ct_context_merge() {
    let mut ctx1 = CtContext::new();
    ctx1.bind("x", CtTag::scalar().with_unit("m"));

    let mut ctx2 = CtContext::new();
    ctx2.bind("y", CtTag::scalar().with_unit("s"));
    ctx2.bind("x", CtTag::scalar().with_unit("km")); // 충돌

    let merged = ctx1.merge(&ctx2);
    assert_eq!(merged.len(), 2);
    // ctx1의 x가 우선
    assert_eq!(merged.get("x").unwrap().unit.as_deref(), Some("m"));
    assert_eq!(merged.get("y").unwrap().unit.as_deref(), Some("s"));
  }

  #[test]
  fn test_ct_context_iter() {
    let mut ctx = CtContext::new();
    ctx.bind("a", CtTag::scalar());
    ctx.bind("b", CtTag::vector());

    let vars: Vec<_> = ctx.iter().map(|(k, _)| k.clone()).collect();
    assert_eq!(vars.len(), 2);
    assert!(vars.contains(&"a".to_string()));
    assert!(vars.contains(&"b".to_string()));
  }
}
