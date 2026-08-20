//! CT 컨텍스트: 변수 → 태그 매핑
//!
//! pnix-old의 symbolic_core/ct/context.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 순수 데이터 구조, 값 계산 없음
//!
//! ## 사용 목적
//!
//! - 변수별 CT 태그 바인딩 관리
//! - 물리량 단위 추적
//! - 타입 체크 컨텍스트

use super::tags::CtTag;
use std::collections::HashMap;

/// 변수별 CT 태그 매핑
#[derive(Clone, Debug, Default)]
pub struct CtContext {
  bindings: HashMap<String, CtTag>,
}

impl CtContext {
  /// 새 컨텍스트 생성
  pub fn new() -> Self {
    Self::default()
  }

  /// 변수에 태그 바인딩
  pub fn bind(&mut self, var: impl Into<String>, tag: CtTag) {
    self.bindings.insert(var.into(), tag);
  }

  /// 변수의 태그 조회
  pub fn get(&self, var: &str) -> Option<&CtTag> {
    self.bindings.get(var)
  }

  /// 바인딩 개수
  pub fn len(&self) -> usize {
    self.bindings.len()
  }

  /// 비어있는지 확인
  pub fn is_empty(&self) -> bool {
    self.bindings.is_empty()
  }

  /// 모든 바인딩 반복
  pub fn iter(&self) -> impl Iterator<Item = (&String, &CtTag)> {
    self.bindings.iter()
  }

  /// 물리 교육용 기본 컨텍스트 생성
  ///
  /// 표준 물리량 변수들을 포함:
  /// - x: 위치 (m)
  /// - v: 속도 (m/s)
  /// - a: 가속도 (m/s^2)
  /// - t: 시간 (s)
  /// - m: 질량 (kg)
  /// - F: 힘 (N)
  /// - E: 에너지 (J)
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

  /// 상대론 물리용 컨텍스트
  pub fn relativity_default() -> Self {
    let mut ctx = Self::new();
    ctx.bind("c", CtTag::scalar().with_unit("m/s"));
    ctx.bind("G", CtTag::scalar().with_unit("m^3/(kg*s^2)"));
    ctx.bind("g", CtTag::tensor().with_domain("spacetime"));
    ctx.bind("R", CtTag::tensor().with_domain("spacetime"));
    ctx.bind("T", CtTag::tensor().with_domain("spacetime"));
    ctx
  }
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_new_context() {
    let ctx = CtContext::new();
    assert!(ctx.is_empty());
  }

  #[test]
  fn test_bind_and_get() {
    let mut ctx = CtContext::new();
    ctx.bind("x", CtTag::scalar().with_unit("m"));

    let tag = ctx.get("x").unwrap();
    assert_eq!(tag.unit, Some("m".to_string()));
  }

  #[test]
  fn test_physics_default() {
    let ctx = CtContext::physics_default();
    assert_eq!(ctx.len(), 7);

    let x = ctx.get("x").unwrap();
    assert_eq!(x.unit, Some("m".to_string()));

    let f = ctx.get("F").unwrap();
    assert_eq!(f.unit, Some("N".to_string()));
  }

  #[test]
  fn test_relativity_default() {
    let ctx = CtContext::relativity_default();

    let g = ctx.get("g").unwrap();
    assert_eq!(g.domain, Some("spacetime".to_string()));
  }

  #[test]
  fn test_iter() {
    let ctx = CtContext::physics_default();
    let count = ctx.iter().count();
    assert_eq!(count, 7);
  }
}
