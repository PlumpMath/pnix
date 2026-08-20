//! Meta-Schema: Schema의 Schema를 다루는 상위 추상화 레이어
//!
//! pnix-old의 meta_schema.rs를 pnix-new 타입 시스템에 맞게 적응.
//!
//! ## Tasks (from pnix-old)
//!
//! - Task 635: Kind 시스템 (Type → Type)
//! - Task 636: ComposedSchemaArrow (고차 Arrow 합성)
//! - Task 637: Subobject Classifier (범주론적 Ω 타입)
//!
//! ## Hierarchy
//!
//! ```text
//! Level 0: Values (1, "hello", true)
//!     ↓
//! Level 1: Types/Schema (Int, String, Bool)
//!     ↓
//! Level 2: Kinds (*, * → *, (* → *) → *)
//!     ↓
//! Level 3: Sorts (□)
//! ```
//!
//! ## CT Connection
//!
//! - Kinds = Objects in a 2-category
//! - Type constructors = 1-morphisms
//! - Natural transformations = 2-morphisms

use super::core_type::CoreType;
use super::schema_arrow::SchemaArrow;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================
// Task 635: Kind System
// ============================================================

/// Kind - the "type of types"
///
/// In type theory:
/// - `*` (Star) is the kind of all types (Int : *, Bool : *)
/// - `* → *` is the kind of type constructors (List : * → *)
/// - `(* → *) → *` is the kind of higher-kinded types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Kind {
  /// Base kind: the kind of all ground types
  /// Int : *, Bool : *, String : *
  #[default]
  Star,

  /// Arrow kind: type constructors
  /// List : * → *, Maybe : * → *
  Arrow(Box<Kind>, Box<Kind>),

  /// Constraint kind: for type class constraints
  /// Eq : * → Constraint, Ord : * → Constraint
  Constraint,

  /// Row kind: for extensible records
  /// { name: String, age: Int } : Row
  Row,
}

impl Kind {
  /// 기본 kind * 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn star() -> Self {
    Kind::Star
  }

  /// Arrow kind k1 → k2 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn arrow(from: Kind, to: Kind) -> Self {
    Kind::Arrow(Box::new(from), Box::new(to))
  }

  /// 단항 타입 생성자 kind: * → *
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn unary_constructor() -> Self {
    Kind::Arrow(Box::new(Kind::Star), Box::new(Kind::Star))
  }

  /// 이항 타입 생성자 kind: * → * → *
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn binary_constructor() -> Self {
    Kind::Arrow(
      Box::new(Kind::Star),
      Box::new(Kind::Arrow(Box::new(Kind::Star), Box::new(Kind::Star))),
    )
  }

  /// Kind가 올바른 형태인지 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 검증만, 값 계산 없음
  pub fn is_well_formed(&self) -> bool {
    match self {
      Kind::Star | Kind::Constraint | Kind::Row => true,
      Kind::Arrow(k1, k2) => k1.is_well_formed() && k2.is_well_formed(),
    }
  }

  /// 타입 인자 개수 (arity) 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn arity(&self) -> usize {
    match self {
      Kind::Star | Kind::Constraint | Kind::Row => 0,
      Kind::Arrow(_, k2) => 1 + k2.arity(),
    }
  }
}

/// 타입 생성자: Kind와 함께하는 타입 생성자
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeConstructor {
  /// 생성자 이름
  pub name: String,
  /// 생성자의 Kind
  pub kind: Kind,
  /// 선택적 정의 (타입 동의어용)
  pub definition: Option<TypeConstructorDef>,
}

/// 타입 생성자 정의: 타입 생성자의 정의 타입
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypeConstructorDef {
  /// 원시 타입 (내장)
  Primitive,
  /// 타입 동의어: type Alias a = ...
  Synonym {
    /// 타입 파라미터 목록
    params: Vec<String>,
    /// 본체 타입
    body: CoreType,
  },
  /// 대수 데이터 타입
  Algebraic {
    /// 타입 파라미터 목록
    params: Vec<String>,
    /// Variant 목록 (이름, 필드 타입 목록)
    variants: Vec<(String, Vec<CoreType>)>,
  },
}

impl TypeConstructor {
  /// 원시 타입 생성자 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn primitive(name: impl Into<String>, kind: Kind) -> Self {
    TypeConstructor {
      name: name.into(),
      kind,
      definition: Some(TypeConstructorDef::Primitive),
    }
  }

  /// 타입 동의어 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn synonym(name: impl Into<String>, params: Vec<String>, body: CoreType) -> Self {
    let kind = params.iter().fold(Kind::Star, |acc, _| {
      Kind::Arrow(Box::new(Kind::Star), Box::new(acc))
    });
    TypeConstructor {
      name: name.into(),
      kind,
      definition: Some(TypeConstructorDef::Synonym { params, body }),
    }
  }

  /// 대수 데이터 타입 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn algebraic(
    name: impl Into<String>,
    params: Vec<String>,
    variants: Vec<(String, Vec<CoreType>)>,
  ) -> Self {
    let kind = params.iter().fold(Kind::Star, |acc, _| {
      Kind::Arrow(Box::new(Kind::Star), Box::new(acc))
    });
    TypeConstructor {
      name: name.into(),
      kind,
      definition: Some(TypeConstructorDef::Algebraic { params, variants }),
    }
  }
}

/// Kind 검사기: 타입 표현식의 Kind를 검증하는 검사기
#[derive(Debug, Clone, Default)]
pub struct KindChecker {
  /// 타입 생성자 환경 (이름 → 생성자 매핑)
  constructors: HashMap<String, TypeConstructor>,
}

impl KindChecker {
  /// 새 Kind 검사기 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new() -> Self {
    let mut checker = KindChecker {
      constructors: HashMap::new(),
    };
    checker.register_primitives();
    checker
  }

  fn register_primitives(&mut self) {
    // Primitive types: * kind
    self.register(TypeConstructor::primitive("Int", Kind::Star));
    self.register(TypeConstructor::primitive("Float", Kind::Star));
    self.register(TypeConstructor::primitive("Bool", Kind::Star));
    self.register(TypeConstructor::primitive("String", Kind::Star));
    self.register(TypeConstructor::primitive("Unit", Kind::Star));

    // Type constructors: * → *
    self.register(TypeConstructor::primitive(
      "List",
      Kind::unary_constructor(),
    ));
    self.register(TypeConstructor::primitive(
      "Optional",
      Kind::unary_constructor(),
    ));
    self.register(TypeConstructor::primitive(
      "Signal",
      Kind::unary_constructor(),
    ));

    // Binary constructors: * → * → *
    self.register(TypeConstructor::primitive(
      "Either",
      Kind::binary_constructor(),
    ));
    self.register(TypeConstructor::primitive(
      "Pair",
      Kind::binary_constructor(),
    ));
    self.register(TypeConstructor::primitive(
      "Arrow",
      Kind::binary_constructor(),
    ));
  }

  /// 타입 생성자 등록
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn register(&mut self, constructor: TypeConstructor) {
    self
      .constructors
      .insert(constructor.name.clone(), constructor);
  }

  /// 이름으로 타입 생성자 조회
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn get(&self, name: &str) -> Option<&TypeConstructor> {
    self.constructors.get(name)
  }

  /// CoreType의 Kind 추론
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  #[allow(clippy::only_used_in_recursion)]
  pub fn infer_kind(&self, ty: &CoreType) -> Result<Kind, KindError> {
    match ty {
      // Base types have kind *
      CoreType::Unit | CoreType::Named(_) => Ok(Kind::Star),

      // Type variables default to kind *
      CoreType::Var(_) => Ok(Kind::Star),

      // Product has kind *
      CoreType::Product(_, _) => Ok(Kind::Star),

      // Arrow has kind *
      CoreType::Arrow(_, _) => Ok(Kind::Star),

      // Sum has kind *
      CoreType::Sum(_, _) => Ok(Kind::Star),

      // Optional: if inner has kind *, then Optional<inner> has kind *
      CoreType::Optional(inner) => {
        let inner_kind = self.infer_kind(inner)?;
        if inner_kind == Kind::Star {
          Ok(Kind::Star)
        } else {
          Err(KindError::Mismatch {
            expected: Kind::Star,
            found: inner_kind,
          })
        }
      }

      // List: if inner has kind *, then List<inner> has kind *
      CoreType::List(inner) => {
        let inner_kind = self.infer_kind(inner)?;
        if inner_kind == Kind::Star {
          Ok(Kind::Star)
        } else {
          Err(KindError::Mismatch {
            expected: Kind::Star,
            found: inner_kind,
          })
        }
      }

      // Record has kind *
      CoreType::Record(_) => Ok(Kind::Star),

      // Forall: body의 kind와 동일
      CoreType::Forall { body, .. } => self.infer_kind(body),
    }
  }

  /// 타입이 예상된 Kind를 가지는지 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 검증만, 값 계산 없음
  pub fn check_kind(&self, ty: &CoreType, expected: &Kind) -> Result<(), KindError> {
    let inferred = self.infer_kind(ty)?;
    if &inferred == expected {
      Ok(())
    } else {
      Err(KindError::Mismatch {
        expected: expected.clone(),
        found: inferred,
      })
    }
  }
}

/// Kind checking errors
#[derive(Debug, Clone)]
/// Kind 에러: Kind 검사 에러 타입
pub enum KindError {
  /// Kind 불일치
  Mismatch {
    /// 예상 Kind
    expected: Kind,
    /// 실제 Kind
    found: Kind,
  },
  /// 알 수 없는 타입 생성자
  UnknownConstructor(
    /// 생성자 이름
    String,
  ),
  /// 타입 인자 개수 불일치
  ArityMismatch {
    /// 예상 개수
    expected: usize,
    /// 실제 개수
    found: usize,
  },
}

// ============================================================
// Task 636: ComposedSchemaArrow
// ============================================================

/// 고차 SchemaArrow 합성: 다양한 방식으로 arrow를 합성
///
/// 합성 방법:
/// - Sequential: f; g (먼저 f, 그 다음 g)
/// - Parallel: f ⊗ g (텐서 곱)
/// - Lifted: F(f) (F는 functor)
#[derive(Debug, Clone)]
pub enum ComposedSchemaArrow {
  /// 기본 arrow (schema_arrow.rs에서)
  Base(
    /// 기본 SchemaArrow
    SchemaArrow,
  ),

  /// 순차 합성: f; g
  /// (f: A → B, g: B → C) ⟹ (f; g): A → C
  Sequential(
    /// 첫 번째 arrow (f)
    Box<ComposedSchemaArrow>,
    /// 두 번째 arrow (g)
    Box<ComposedSchemaArrow>,
  ),

  /// 병렬 합성: f ⊗ g
  /// (f: A → B, g: C → D) ⟹ (f ⊗ g): A × C → B × D
  Parallel(
    /// 첫 번째 arrow (f)
    Box<ComposedSchemaArrow>,
    /// 두 번째 arrow (g)
    Box<ComposedSchemaArrow>,
  ),

  /// Functor lift: F(f)
  /// (f: A → B, F: * → *) ⟹ F(f): F(A) → F(B)
  Lifted {
    /// Functor 이름
    functor: String,
    /// 내부 arrow
    inner: Box<ComposedSchemaArrow>,
  },

  /// Natural transformation 성분: η_A
  /// η: F ⟹ G일 때, η_A: F(A) → G(A)
  NatTransComponent {
    /// Natural transformation 이름
    nat_trans: String,
    /// 타입 인자 (A)
    type_arg: CoreType,
  },

  /// 항등 arrow: id_A
  Identity(
    /// 타입
    CoreType,
  ),
}

impl ComposedSchemaArrow {
  /// 기본 schema arrow에서 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn from_base(arrow: SchemaArrow) -> Self {
    ComposedSchemaArrow::Base(arrow)
  }

  /// 항등 arrow 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn identity(ty: CoreType) -> Self {
    ComposedSchemaArrow::Identity(ty)
  }

  /// 순차 합성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn seq(self, other: ComposedSchemaArrow) -> Self {
    ComposedSchemaArrow::Sequential(Box::new(self), Box::new(other))
  }

  /// 병렬 합성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn par(self, other: ComposedSchemaArrow) -> Self {
    ComposedSchemaArrow::Parallel(Box::new(self), Box::new(other))
  }

  /// Functor를 통한 lift
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn lift(self, functor: impl Into<String>) -> Self {
    ComposedSchemaArrow::Lifted {
      functor: functor.into(),
      inner: Box::new(self),
    }
  }

  /// 합성 arrow가 올바른 타입인지 확인
  ///
  /// 유효한 경우 (source_type, target_type) 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 검증만, 값 계산 없음
  pub fn type_check(&self) -> Result<(MetaType, MetaType), CompositionError> {
    match self {
      ComposedSchemaArrow::Base(arrow) => Ok((
        MetaType::Core(arrow.source()),
        MetaType::Core(arrow.target()),
      )),
      ComposedSchemaArrow::Sequential(f, g) => {
        let (a, b1) = f.type_check()?;
        let (b2, c) = g.type_check()?;
        if b1 == b2 {
          Ok((a, c))
        } else {
          Err(CompositionError::TypeMismatch {
            expected: b1,
            found: b2,
          })
        }
      }
      ComposedSchemaArrow::Parallel(f, g) => {
        let (a, b) = f.type_check()?;
        let (c, d) = g.type_check()?;
        Ok((
          MetaType::Product(Box::new(a), Box::new(c)),
          MetaType::Product(Box::new(b), Box::new(d)),
        ))
      }
      ComposedSchemaArrow::Lifted { functor, inner } => {
        let (a, b) = inner.type_check()?;
        Ok((
          MetaType::Applied(functor.clone(), Box::new(a)),
          MetaType::Applied(functor.clone(), Box::new(b)),
        ))
      }
      ComposedSchemaArrow::NatTransComponent { type_arg, .. } => Ok((
        MetaType::Core(type_arg.clone()),
        MetaType::Core(type_arg.clone()),
      )),
      ComposedSchemaArrow::Identity(ty) => {
        Ok((MetaType::Core(ty.clone()), MetaType::Core(ty.clone())))
      }
    }
  }
}

/// 메타 타입: 합성 타입 검사를 위한 메타 레벨 타입
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetaType {
  /// 코어 타입
  Core(
    /// CoreType 값
    CoreType,
  ),
  /// 메타 타입의 곱
  Product(
    /// 첫 번째 메타 타입
    Box<MetaType>,
    /// 두 번째 메타 타입
    Box<MetaType>,
  ),
  /// 적용된 타입 생성자
  Applied(
    /// 생성자 이름
    String,
    /// 인자 메타 타입
    Box<MetaType>,
  ),
}

/// 합성 에러: 합성 중 발생하는 에러 타입
#[derive(Debug, Clone)]
pub enum CompositionError {
  /// 순차 합성에서 타입 불일치
  TypeMismatch {
    /// 예상 메타 타입
    expected: MetaType,
    /// 실제 메타 타입
    found: MetaType,
  },
  /// 알 수 없는 functor
  UnknownFunctor(
    /// Functor 이름
    String,
  ),
  /// 잘못된 합성
  InvalidComposition(
    /// 에러 메시지
    String,
  ),
}

// ============================================================
// Task 637: Subobject Classifier
// ============================================================

/// Subobject Classifier (Ω): 범주론의 subobject classifier
///
/// 범주론에서 subobject classifier는 특별한 객체 Ω로,
/// morphism true: 1 → Ω가 있고, 모든 monomorphism m: S → A에 대해
/// pullback square를 만드는 고유한 χ: A → Ω가 존재합니다.
///
/// 타입 이론 용어:
/// - Ω ≈ Bool (Set에서)
/// - Ω ≈ Prop (topos에서)
/// - χ는 subobject의 "characteristic function"
#[derive(Debug, Clone)]
pub struct SubobjectClassifier {
  /// Classifier 타입 (보통 Bool)
  pub omega: CoreType,
}

impl SubobjectClassifier {
  /// 표준 subobject classifier 생성 (Bool 사용)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn standard() -> Self {
    SubobjectClassifier {
      omega: CoreType::Named("Bool".into()),
    }
  }

  /// Predicate에 대한 characteristic function 생성
  ///
  /// 타입 A에 대한 predicate P가 주어지면, χ_P: A → Ω를 반환합니다.
  /// 여기서 χ_P(x) = true iff P(x)입니다.
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn characteristic_function(
    &self,
    domain: CoreType,
    predicate_name: String,
  ) -> CharacteristicFunction {
    CharacteristicFunction {
      domain,
      classifier: self.omega.clone(),
      predicate: predicate_name,
    }
  }
}

impl Default for SubobjectClassifier {
  fn default() -> Self {
    Self::standard()
  }
}

/// Characteristic function: χ: A → Ω (subobject의 특성 함수)
#[derive(Debug, Clone)]
pub struct CharacteristicFunction {
  /// 정의역 타입 A
  pub domain: CoreType,
  /// Classifier 타입 Ω
  pub classifier: CoreType,
  /// 분류되는 predicate 이름
  pub predicate: String,
}

/// Subobject: classifier 데이터와 함께하는 subobject
#[derive(Debug, Clone)]
pub struct Subobject {
  /// 하위 타입 S
  pub sub_type: CoreType,
  /// 상위 타입 A
  pub super_type: CoreType,
  /// Characteristic function χ: A → Ω
  pub characteristic: CharacteristicFunction,
}

// ============================================================
// Integration: Meta-Schema Registry
// ============================================================

/// Meta-schema 구조체 레지스트리: meta-schema 구성 요소들의 레지스트리
#[derive(Debug, Clone, Default)]
pub struct MetaSchemaRegistry {
  /// Kind 검사기
  pub kind_checker: KindChecker,
  /// Subobject classifier
  pub subobject_classifier: SubobjectClassifier,
  /// 등록된 natural transformation 목록
  nat_transformations: HashMap<String, NatTransformation>,
}

/// Natural transformation 기록: natural transformation 정보
#[derive(Debug, Clone)]
pub struct NatTransformation {
  /// Natural transformation 이름
  pub name: String,
  /// 소스 functor 이름
  pub source: String,
  /// 타겟 functor 이름
  pub target: String,
}

impl MetaSchemaRegistry {
  /// 새 Meta-schema 레지스트리 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new() -> Self {
    MetaSchemaRegistry {
      kind_checker: KindChecker::new(),
      subobject_classifier: SubobjectClassifier::standard(),
      nat_transformations: HashMap::new(),
    }
  }

  /// Natural transformation 등록
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn register_nat_trans(&mut self, nat: NatTransformation) {
    self.nat_transformations.insert(nat.name.clone(), nat);
  }

  /// 이름으로 natural transformation 조회
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn get_nat_trans(&self, name: &str) -> Option<&NatTransformation> {
    self.nat_transformations.get(name)
  }

  /// 타입의 Kind 추론
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn infer_kind(&self, ty: &CoreType) -> Result<Kind, KindError> {
    self.kind_checker.infer_kind(ty)
  }

  /// Characteristic function 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn characteristic_fn(
    &self,
    domain: CoreType,
    predicate: impl Into<String>,
  ) -> CharacteristicFunction {
    self
      .subobject_classifier
      .characteristic_function(domain, predicate.into())
  }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_kind_star() {
    let checker = KindChecker::new();
    let int_ty = CoreType::Named("Int".into());

    let kind = checker.infer_kind(&int_ty).unwrap();
    assert_eq!(kind, Kind::Star);
  }

  #[test]
  fn test_kind_arity() {
    assert_eq!(Kind::Star.arity(), 0);
    assert_eq!(Kind::unary_constructor().arity(), 1);
    assert_eq!(Kind::binary_constructor().arity(), 2);
  }

  #[test]
  fn test_kind_well_formed() {
    assert!(Kind::Star.is_well_formed());
    assert!(Kind::unary_constructor().is_well_formed());
    assert!(Kind::binary_constructor().is_well_formed());
    assert!(Kind::Constraint.is_well_formed());
    assert!(Kind::Row.is_well_formed());
  }

  #[test]
  fn test_type_constructor_primitive() {
    let int_ctor = TypeConstructor::primitive("Int", Kind::Star);
    assert_eq!(int_ctor.name, "Int");
    assert_eq!(int_ctor.kind, Kind::Star);
  }

  #[test]
  fn test_type_constructor_synonym() {
    let string_ty = CoreType::Named("String".into());
    let alias = TypeConstructor::synonym("MyString", vec![], string_ty);
    assert_eq!(alias.name, "MyString");
    assert_eq!(alias.kind, Kind::Star);
  }

  #[test]
  fn test_composed_arrow_identity() {
    let int_ty = CoreType::Named("Int".into());
    let id = ComposedSchemaArrow::identity(int_ty.clone());

    let (src, tgt) = id.type_check().unwrap();
    assert_eq!(src, MetaType::Core(int_ty.clone()));
    assert_eq!(tgt, MetaType::Core(int_ty));
  }

  #[test]
  fn test_composed_arrow_sequential() {
    let int_ty = CoreType::Named("Int".into());

    let id1 = ComposedSchemaArrow::identity(int_ty.clone());
    let id2 = ComposedSchemaArrow::Identity(int_ty.clone());

    let seq = id1.seq(id2);
    let result = seq.type_check();
    assert!(result.is_ok());
  }

  #[test]
  fn test_composed_arrow_parallel() {
    let int_ty = CoreType::Named("Int".into());
    let string_ty = CoreType::Named("String".into());

    let id_int = ComposedSchemaArrow::identity(int_ty);
    let id_string = ComposedSchemaArrow::identity(string_ty);

    let parallel = id_int.par(id_string);
    let result = parallel.type_check();
    assert!(result.is_ok());

    let (src, tgt) = result.unwrap();
    assert!(matches!(src, MetaType::Product(_, _)));
    assert!(matches!(tgt, MetaType::Product(_, _)));
  }

  #[test]
  fn test_composed_arrow_lift() {
    let int_ty = CoreType::Named("Int".into());
    let id = ComposedSchemaArrow::identity(int_ty);
    let lifted = id.lift("List");

    let (src, tgt) = lifted.type_check().unwrap();
    assert!(matches!(src, MetaType::Applied(f, _) if f == "List"));
    assert!(matches!(tgt, MetaType::Applied(f, _) if f == "List"));
  }

  #[test]
  fn test_subobject_classifier() {
    let classifier = SubobjectClassifier::standard();
    assert_eq!(classifier.omega, CoreType::Named("Bool".into()));
  }

  #[test]
  fn test_characteristic_function() {
    let classifier = SubobjectClassifier::standard();
    let int_ty = CoreType::Named("Int".into());

    let char_fn = classifier.characteristic_function(int_ty.clone(), "positive".into());
    assert_eq!(char_fn.domain, int_ty);
    assert_eq!(char_fn.predicate, "positive");
  }

  #[test]
  fn test_meta_schema_registry() {
    let registry = MetaSchemaRegistry::new();

    // Kind checking
    let int_ty = CoreType::Named("Int".into());
    let kind = registry.infer_kind(&int_ty).unwrap();
    assert_eq!(kind, Kind::Star);
  }

  #[test]
  fn test_type_constructor_arity() {
    let checker = KindChecker::new();

    // List has arity 1
    let list_ctor = checker.get("List").unwrap();
    assert_eq!(list_ctor.kind.arity(), 1);

    // Either has arity 2
    let either_ctor = checker.get("Either").unwrap();
    assert_eq!(either_ctor.kind.arity(), 2);
  }

  #[test]
  fn test_register_nat_trans() {
    let mut registry = MetaSchemaRegistry::new();

    let nat = NatTransformation {
      name: "head".into(),
      source: "List".into(),
      target: "Optional".into(),
    };
    registry.register_nat_trans(nat);

    let found = registry.get_nat_trans("head");
    assert!(found.is_some());
    assert_eq!(found.unwrap().source, "List");
  }
}
