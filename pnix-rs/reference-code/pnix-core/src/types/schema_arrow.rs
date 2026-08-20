//! SchemaArrow: 타입 변환 morphism
//!
//! pnix-old의 schema_arrow.rs를 그래프 워크플로우에 맞게 적응.
//!
//! ## CT Laws
//!
//! - Identity: id: A → A
//! - Composition: (f: A → B) ∘ (g: B → C) = (g ∘ f): A → C
//! - Transitivity: A <: B ∧ B <: C ⟹ A <: C
//!
//! ## pnix-new 적용
//!
//! - Edge 타입 호환성 검증
//! - 암시적 변환 (Subtyping)
//! - 명시적 변환 (Coercion)

use super::CoreType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// 타입 변환 morphism: 타입 간 변환을 나타내는 범주론적 사상
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SchemaArrow {
  /// 항등 사상: A → A (변환 없음)
  Identity(
    /// 타입
    CoreType,
  ),

  /// Subtyping 사상: A <: B (암시적 포함, subtyping 관계)
  Subtyping {
    /// 출발 타입 (하위 타입)
    from: CoreType,
    /// 도착 타입 (상위 타입)
    to: CoreType,
    /// Subtyping 증거 (subtyping 관계의 증명)
    evidence: SubtypingEvidence,
  },

  /// Coercion 사상: A → B (명시적 타입 변환)
  Coercion {
    /// 출발 타입
    from: CoreType,
    /// 도착 타입
    to: CoreType,
    /// Coercion 종류 (변환 방법)
    kind: CoercionKind,
  },

  /// Projection 사상: Record → Field (레코드에서 필드 추출)
  Projection {
    /// 레코드 타입
    record: CoreType,
    /// 필드 이름
    field_name: String,
    /// 필드 타입
    field_type: CoreType,
  },

  /// Optional unwrap: A? → A (Optional에서 값 추출, 실패 가능)
  OptionalUnwrap {
    /// 출발 타입 (Optional)
    from: CoreType,
    /// 도착 타입
    to: CoreType,
  },

  /// Optional wrap: A → A? (값을 Optional로 래핑)
  OptionalWrap {
    /// 출발 타입
    from: CoreType,
    /// 도착 타입 (Optional)
    to: CoreType,
  },

  /// 합성: (f ∘ g) (두 사상을 합성)
  Compose(
    /// 첫 번째 arrow (f)
    Box<SchemaArrow>,
    /// 두 번째 arrow (g)
    Box<SchemaArrow>,
  ),
}

/// Subtyping 증거: Subtyping 관계의 증거 타입
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubtypingEvidence {
  /// 반사적: A <: A
  Reflexive,

  /// Optional 확장: A <: A?
  OptionalWidening,

  /// Unit 흡수: () <: A (optional 위치용)
  UnitAbsorption,

  /// List 공변성: `[A]` <: `[B]` if A <: B
  ListCovariance(
    /// 내부 타입의 Subtyping 증거
    Box<SubtypingEvidence>,
  ),

  /// Record 너비 subtyping: { a, b, c } <: { a, b }
  RecordWidth,

  /// Record 깊이 subtyping: 필드 타입 공변성
  RecordDepth(
    /// 필드별 Subtyping 증거 목록 (필드 이름, 증거)
    Vec<(String, SubtypingEvidence)>,
  ),

  /// 전이적: A <: B ∧ B <: C ⟹ A <: C
  Transitive(
    /// 첫 번째 증거
    Box<SubtypingEvidence>,
    /// 두 번째 증거
    Box<SubtypingEvidence>,
  ),
}

/// Coercion 종류: 명시적 타입 변환의 종류
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoercionKind {
  /// Product 평탄화: (A * B) * C → A * B * C
  ProductFlatten,

  /// Product 첫 번째 투영: A * B → A
  ProductFirst,

  /// Product 두 번째 투영: A * B → B
  ProductSecond,

  /// List 단일화: A → `[A]` (값을 단일 요소 리스트로)
  ListSingleton,

  /// List 평탄화: `[[A]]` → `[A]` (중첩 리스트를 평탄화)
  ListFlatten,

  /// Record를 Product로 변환: { a: A, b: B } → A * B
  RecordToProduct,

  /// 사용자 정의 coercion (이름으로 식별)
  Custom(
    /// Coercion 이름
    String,
  ),
}

impl SchemaArrow {
  /// Source 타입
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn source(&self) -> CoreType {
    match self {
      SchemaArrow::Identity(t) => t.clone(),
      SchemaArrow::Subtyping { from, .. } => from.clone(),
      SchemaArrow::Coercion { from, .. } => from.clone(),
      SchemaArrow::Projection { record, .. } => record.clone(),
      SchemaArrow::OptionalUnwrap { from, .. } => from.clone(),
      SchemaArrow::OptionalWrap { from, .. } => from.clone(),
      SchemaArrow::Compose(first, _) => first.source(),
    }
  }

  /// Target 타입
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn target(&self) -> CoreType {
    match self {
      SchemaArrow::Identity(t) => t.clone(),
      SchemaArrow::Subtyping { to, .. } => to.clone(),
      SchemaArrow::Coercion { to, .. } => to.clone(),
      SchemaArrow::Projection { field_type, .. } => field_type.clone(),
      SchemaArrow::OptionalUnwrap { to, .. } => to.clone(),
      SchemaArrow::OptionalWrap { to, .. } => to.clone(),
      SchemaArrow::Compose(_, second) => second.target(),
    }
  }

  /// Identity arrow
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn identity(ty: CoreType) -> Self {
    SchemaArrow::Identity(ty)
  }

  /// Compose: self ; other
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 합성만, 값 계산 없음
  pub fn then(self, other: SchemaArrow) -> Result<SchemaArrow, SchemaArrowError> {
    if self.target() == other.source() {
      Ok(SchemaArrow::Compose(Box::new(self), Box::new(other)))
    } else {
      Err(SchemaArrowError::CompositionMismatch {
        expected: self.target(),
        found: other.source(),
      })
    }
  }

  /// Record field projection
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn project(record: CoreType, field_name: impl Into<String>) -> Option<Self> {
    let field_name = field_name.into();
    match &record {
      CoreType::Record(fields) => {
        fields
          .iter()
          .find(|(name, _)| name == &field_name)
          .map(|(_, ty)| SchemaArrow::Projection {
            record: record.clone(),
            field_name: field_name.clone(),
            field_type: ty.clone(),
          })
      }
      _ => None,
    }
  }

  /// Optional wrap: A → A?
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn wrap_optional(ty: CoreType) -> Self {
    SchemaArrow::OptionalWrap {
      from: ty.clone(),
      to: CoreType::Optional(Box::new(ty)),
    }
  }

  /// Optional unwrap: A? → A
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn unwrap_optional(ty: CoreType) -> Option<Self> {
    match ty {
      CoreType::Optional(inner) => Some(SchemaArrow::OptionalUnwrap {
        from: CoreType::Optional(inner.clone()),
        to: inner.as_ref().clone(),
      }),
      _ => None,
    }
  }
}

/// Subtyping 검사기
/// Subtyping 검사기: 타입 간 subtyping 관계를 검사하고 캐싱
#[derive(Debug, Default)]
pub struct SubtypingChecker {
  /// Subtyping 검사 결과 캐시 (타입 쌍 → 증거)
  cache: HashMap<(CoreType, CoreType), Option<SubtypingEvidence>>,
}

impl SubtypingChecker {
  /// 새 Subtyping 검사기 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new() -> Self {
    Self::default()
  }

  /// A <: B 검사
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 검증만, 값 계산 없음
  pub fn check(&mut self, from: &CoreType, to: &CoreType) -> Option<SubtypingEvidence> {
    let key = (from.clone(), to.clone());
    if let Some(cached) = self.cache.get(&key) {
      return cached.clone();
    }

    let result = self.check_impl(from, to);
    self.cache.insert(key, result.clone());
    result
  }

  fn check_impl(&mut self, from: &CoreType, to: &CoreType) -> Option<SubtypingEvidence> {
    // Rule 1: Reflexivity (A <: A)
    if from == to {
      return Some(SubtypingEvidence::Reflexive);
    }

    match (from, to) {
      // Rule 2: Optional widening (A <: A?)
      (inner, CoreType::Optional(opt_inner)) if inner == opt_inner.as_ref() => {
        Some(SubtypingEvidence::OptionalWidening)
      }

      // Rule 3: Unit absorption (() <: A?)
      (CoreType::Unit, CoreType::Optional(_)) => Some(SubtypingEvidence::UnitAbsorption),

      // Rule 4: List covariance ([A] <: [B] if A <: B)
      (CoreType::List(a), CoreType::List(b)) => self
        .check(a, b)
        .map(|ev| SubtypingEvidence::ListCovariance(Box::new(ev))),

      // Rule 5: Optional covariance (A? <: B? if A <: B)
      (CoreType::Optional(a), CoreType::Optional(b)) => self.check(a, b),

      // Rule 6: Record width subtyping
      (CoreType::Record(from_fields), CoreType::Record(to_fields)) => {
        // All fields in 'to' must exist in 'from'
        let all_present = to_fields
          .iter()
          .all(|(name, _)| from_fields.iter().any(|(n, _)| n == name));

        if !all_present {
          return None;
        }

        // Check depth subtyping for common fields
        let mut depth_evidence = Vec::new();
        for (name, to_ty) in to_fields {
          if let Some((_, from_ty)) = from_fields.iter().find(|(n, _)| n == name) {
            if let Some(ev) = self.check(from_ty, to_ty) {
              if !matches!(ev, SubtypingEvidence::Reflexive) {
                depth_evidence.push((name.clone(), ev));
              }
            } else {
              return None;
            }
          }
        }

        if depth_evidence.is_empty() {
          Some(SubtypingEvidence::RecordWidth)
        } else {
          Some(SubtypingEvidence::RecordDepth(depth_evidence))
        }
      }

      // Rule 7: Transitivity through Optional
      (from_ty, CoreType::Optional(to_inner)) => self.check(from_ty, to_inner).map(|ev| {
        SubtypingEvidence::Transitive(Box::new(ev), Box::new(SubtypingEvidence::OptionalWidening))
      }),

      _ => None,
    }
  }

  /// Subtyping arrow 생성
  pub fn make_arrow(&mut self, from: CoreType, to: CoreType) -> Option<SchemaArrow> {
    self
      .check(&from, &to)
      .map(|evidence| SchemaArrow::Subtyping { from, to, evidence })
  }
}

/// SchemaArrow 에러: SchemaArrow 관련 에러 타입
///
/// # Example
/// ```rust
/// use pnix_core::types::{CoreType, SchemaArrowError};
/// let err = SchemaArrowError::NotOptional { ty: CoreType::named("Int") };
/// assert!(matches!(err, SchemaArrowError::NotOptional { .. }));
/// ```
#[derive(Debug, Error)]
pub enum SchemaArrowError {
  /// Subtyping 관계 없음: from <: to가 성립하지 않음
  #[error("No subtyping relation: {from} <: {to}")]
  NoSubtyping {
    /// 출발 타입
    from: CoreType,
    /// 도착 타입
    to: CoreType,
  },

  /// 합성에서 타입 불일치: 두 arrow를 합성할 때 타입이 맞지 않음
  #[error("Type mismatch in composition: expected {expected}, found {found}")]
  CompositionMismatch {
    /// 예상 타입
    expected: CoreType,
    /// 실제 타입
    found: CoreType,
  },

  /// 필드를 찾을 수 없음: 레코드에 필드가 존재하지 않음
  #[error("Field not found: {field} in {record}")]
  FieldNotFound {
    /// 필드 이름
    field: String,
    /// 레코드 타입
    record: CoreType,
  },

  /// Optional이 아님: Optional이 아닌 타입을 unwrap하려고 시도
  #[error("Cannot unwrap non-optional type: {ty}")]
  NotOptional {
    /// 타입
    ty: CoreType,
  },
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_subtyping_reflexivity() {
    let mut checker = SubtypingChecker::new();
    let pos = CoreType::named("Position");

    let ev = checker.check(&pos, &pos);
    assert!(matches!(ev, Some(SubtypingEvidence::Reflexive)));
  }

  #[test]
  fn test_subtyping_optional_widening() {
    let mut checker = SubtypingChecker::new();
    let pos = CoreType::named("Position");
    let opt_pos = CoreType::optional(pos.clone());

    let ev = checker.check(&pos, &opt_pos);
    assert!(matches!(ev, Some(SubtypingEvidence::OptionalWidening)));
  }

  #[test]
  fn test_subtyping_unit_absorption() {
    let mut checker = SubtypingChecker::new();
    let unit = CoreType::Unit;
    let opt_pos = CoreType::optional(CoreType::named("Position"));

    let ev = checker.check(&unit, &opt_pos);
    assert!(matches!(ev, Some(SubtypingEvidence::UnitAbsorption)));
  }

  #[test]
  fn test_subtyping_list_covariance() {
    let mut checker = SubtypingChecker::new();
    let list_pos = CoreType::list(CoreType::named("Position"));
    let list_opt_pos = CoreType::list(CoreType::optional(CoreType::named("Position")));

    let ev = checker.check(&list_pos, &list_opt_pos);
    assert!(matches!(ev, Some(SubtypingEvidence::ListCovariance(_))));
  }

  #[test]
  fn test_subtyping_record_width() {
    let mut checker = SubtypingChecker::new();

    let wider = CoreType::record(vec![
      ("x", CoreType::named("Float")),
      ("y", CoreType::named("Float")),
      ("z", CoreType::named("Float")),
    ]);

    let narrower = CoreType::record(vec![
      ("x", CoreType::named("Float")),
      ("y", CoreType::named("Float")),
    ]);

    let ev = checker.check(&wider, &narrower);
    assert!(matches!(ev, Some(SubtypingEvidence::RecordWidth)));
  }

  #[test]
  fn test_arrow_identity() {
    let pos = CoreType::named("Position");
    let arrow = SchemaArrow::identity(pos.clone());

    assert_eq!(arrow.source(), pos);
    assert_eq!(arrow.target(), pos);
  }

  #[test]
  fn test_arrow_composition() {
    let a = CoreType::named("A");

    let mut checker = SubtypingChecker::new();

    // A <: A? <: A?
    let opt_a = CoreType::optional(a.clone());
    let arrow1 = checker.make_arrow(a.clone(), opt_a.clone()).unwrap();
    let arrow2 = SchemaArrow::identity(opt_a.clone());

    let composed = arrow1.then(arrow2);
    assert!(composed.is_ok());

    let composed = composed.unwrap();
    assert_eq!(composed.source(), a);
    assert_eq!(composed.target(), opt_a);
  }

  #[test]
  fn test_arrow_projection() {
    let record = CoreType::record(vec![
      ("pos", CoreType::named("Position")),
      ("vel", CoreType::named("Velocity")),
    ]);

    let arrow = SchemaArrow::project(record.clone(), "pos");
    assert!(arrow.is_some());

    let arrow = arrow.unwrap();
    assert_eq!(arrow.source(), record);
    assert_eq!(arrow.target(), CoreType::named("Position"));
  }

  #[test]
  fn test_optional_wrap_unwrap() {
    let pos = CoreType::named("Position");
    let opt_pos = CoreType::optional(pos.clone());

    let wrap = SchemaArrow::wrap_optional(pos.clone());
    assert_eq!(wrap.source(), pos);
    assert_eq!(wrap.target(), opt_pos.clone());

    let unwrap = SchemaArrow::unwrap_optional(opt_pos.clone());
    assert!(unwrap.is_some());
    assert_eq!(unwrap.as_ref().unwrap().source(), opt_pos);
    assert_eq!(unwrap.unwrap().target(), pos);
  }
}
