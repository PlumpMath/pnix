//! CT 태그 정의
//!
//! pnix-old의 symbolic_core/ct/tags.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 순수 메타데이터 타입 정의, 값 계산 없음
//!
//! ## 직렬화 포맷
//!
//! CT 태그는 serde JSON 직렬화를 지원합니다.

use serde::{Deserialize, Serialize};

/// Category Theory 카테고리 종류
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CtCategory {
  /// 스칼라 필드 (R, C)
  ScalarField,
  /// 벡터 공간 (R^n)
  VectorSpace,
  /// 행렬 공간 (M(n,m))
  MatrixSpace,
  /// 텐서 공간 (T^{ij}_{kl})
  TensorSpace,
  // v3 확장 예정:
  // Gauge(String),       // gauge(SU(3))
  // Bundle(String),      // fiber bundle
  // Manifold(String),    // 다양체
  // Representation(String),  // group representation
}

/// CT 태그 (표현에 부착되는 메타데이터)
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CtTag {
  /// 카테고리 종류
  #[serde(skip_serializing_if = "Option::is_none")]
  pub category: Option<CtCategory>,
  /// 도메인 ("R", "R^3", "M" ...)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub domain: Option<String>,
  /// 코도메인
  #[serde(skip_serializing_if = "Option::is_none")]
  pub codomain: Option<String>,
  /// 단위 ("m", "s", "N", "J" ...)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub unit: Option<String>,
}

impl CtTag {
  /// 스칼라 태그 생성
  pub fn scalar() -> Self {
    Self {
      category: Some(CtCategory::ScalarField),
      ..Default::default()
    }
  }

  /// 벡터 태그 생성
  pub fn vector() -> Self {
    Self {
      category: Some(CtCategory::VectorSpace),
      ..Default::default()
    }
  }

  /// 행렬 태그 생성
  pub fn matrix() -> Self {
    Self {
      category: Some(CtCategory::MatrixSpace),
      ..Default::default()
    }
  }

  /// 텐서 태그 생성
  pub fn tensor() -> Self {
    Self {
      category: Some(CtCategory::TensorSpace),
      ..Default::default()
    }
  }

  /// 단위 설정
  pub fn with_unit(mut self, unit: impl Into<String>) -> Self {
    self.unit = Some(unit.into());
    self
  }

  /// 도메인 설정
  pub fn with_domain(mut self, domain: impl Into<String>) -> Self {
    self.domain = Some(domain.into());
    self
  }

  /// 코도메인 설정
  pub fn with_codomain(mut self, codomain: impl Into<String>) -> Self {
    self.codomain = Some(codomain.into());
    self
  }

  /// 단위가 같은지 확인
  pub fn unit_matches(&self, other: &CtTag) -> bool {
    match (&self.unit, &other.unit) {
      (Some(a), Some(b)) => a == b,
      (None, None) => true,
      _ => false, // 하나만 있으면 불일치
    }
  }

  /// 카테고리가 같은지 확인
  pub fn category_matches(&self, other: &CtTag) -> bool {
    self.category == other.category
  }
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_scalar_tag() {
    let tag = CtTag::scalar();
    assert_eq!(tag.category, Some(CtCategory::ScalarField));
  }

  #[test]
  fn test_with_unit() {
    let tag = CtTag::scalar().with_unit("m");
    assert_eq!(tag.unit, Some("m".to_string()));
  }

  #[test]
  fn test_unit_matches() {
    let a = CtTag::scalar().with_unit("m");
    let b = CtTag::scalar().with_unit("m");
    let c = CtTag::scalar().with_unit("s");

    assert!(a.unit_matches(&b));
    assert!(!a.unit_matches(&c));
  }

  #[test]
  fn test_unit_matches_none() {
    let a = CtTag::scalar();
    let b = CtTag::scalar();
    let c = CtTag::scalar().with_unit("m");

    assert!(a.unit_matches(&b)); // 둘 다 None
    assert!(!a.unit_matches(&c)); // 하나만 None
  }

  #[test]
  fn test_serde() {
    let tag = CtTag::scalar().with_unit("N").with_domain("R");
    let json = serde_json::to_string(&tag).unwrap();
    let restored: CtTag = serde_json::from_str(&json).unwrap();
    assert_eq!(tag, restored);
  }
}
