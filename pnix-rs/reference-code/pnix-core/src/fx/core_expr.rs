//! FxCoreExpr - Language-independent Math Core AST
//!
//! All fx expressions from Nix/Clj/Py converge to this layer.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 순수 AST 구조 정의, 값 계산 없음
//!
//! ## 사용 목적
//!
//! - FxSurfaceExpr에서 lowering된 표현식
//! - 언어 독립적 수학 연산 표현
//! - Effect/Time 분석 지원

use crate::effects::{EffectZone, TimeKind};
use crate::fx::meaning_op::{MeaningMeta, MeaningOpId};
use serde::{Deserialize, Serialize};

/// Signal ID (FRP signal reference)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
/// Signal ID (FRP signal identifier)
///
/// NOTE: FrpGraph는 SignalId/StateId를 단일 카운터에서 할당해
/// 직렬화 시 충돌을 방지한다. (수동 생성은 별도 주의)
pub struct SignalId(pub usize);

/// FxCoreExpr - Math Core AST
///
/// Final representation of all fx expressions. Language-independent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FxCoreExpr {
  // ========== Literals ==========
  /// Integer constant
  ConstInt(i64),
  /// Float constant
  ConstFloat(f64),
  /// Boolean constant
  ConstBool(bool),
  /// String constant
  ConstString(String),
  /// List
  List(Vec<FxCoreExpr>),
  /// AttrSet (key-value pairs)
  AttrSet(Vec<(String, FxCoreExpr)>),

  // ========== Parameters ==========
  /// System time (param.system_time)
  ParamSysTime,
  /// Delta time (param.dt)
  ParamDeltaTime,
  /// Signal variable reference
  SignalVar(SignalId),
  /// General variable reference
  Var(String),

  // ========== Arithmetic ==========
  /// 단항 연산
  Unary {
    /// 연산 메타데이터 (연산 ID, 효과 영역, 시간 성질)
    meta: MeaningMeta,
    /// 피연산자 표현식
    arg: Box<FxCoreExpr>,
  },
  /// 이항 연산
  Binary {
    /// 연산 메타데이터 (연산 ID, 효과 영역, 시간 성질)
    meta: MeaningMeta,
    /// 왼쪽 피연산자 표현식
    lhs: Box<FxCoreExpr>,
    /// 오른쪽 피연산자 표현식
    rhs: Box<FxCoreExpr>,
  },

  // ========== Derived (Pattern Lifted) ==========
  /// 파생 연산 (패턴 리프팅된 고수준 연산)
  /// 예: SecondsFromTime, AngleFromSecond
  Derived {
    /// 연산 메타데이터 (연산 ID, 효과 영역, 시간 성질)
    meta: MeaningMeta,
    /// 인자 표현식 목록
    args: Vec<FxCoreExpr>,
  },

  // ========== Control Flow ==========
  /// 조건문
  If {
    /// 조건 표현식
    cond: Box<FxCoreExpr>,
    /// then 절 표현식
    then_: Box<FxCoreExpr>,
    /// else 절 표현식
    else_: Box<FxCoreExpr>,
  },
  /// Y08a-11: Let 바인딩 (lazy semantics 보존)
  /// let x = value in body
  Let {
    /// 바인딩 이름
    name: String,
    /// 값 표현식
    value: Box<FxCoreExpr>,
    /// 본문 표현식
    body: Box<FxCoreExpr>,
  },

  // ========== Interop ==========
  /// 언어 간 호출
  Interop {
    /// 연산 메타데이터 (연산 ID, 효과 영역, 시간 성질)
    meta: MeaningMeta,
    /// 언어 이름 ("nix", "clj", "py")
    lang: String,
    /// 원본 코드 (향후 AST로 대체 예정)
    code: String,
  },

  // ========== Lambda ==========
  /// 람다 추상화 (λparam.body)
  Lambda {
    /// 파라미터 이름
    param: String,
    /// 본문 표현식
    body: Box<FxCoreExpr>,
  },

  // ========== Attribute Access ==========
  /// 속성 선택 (expr.attr)
  Select {
    /// 기준 표현식
    expr: Box<FxCoreExpr>,
    /// 속성 이름
    attr: String,
  },

  // ========== ADT Value Constructors ==========
  /// ADT value constructor: Some(42), None, Ok(x), Err("msg")
  Construct {
    /// Variant name (e.g., "Some", "None", "Ok", "Err")
    variant: String,
    /// Arguments (empty for nullary constructors like None)
    args: Vec<FxCoreExpr>,
  },

  // ========== Runtime Errors ==========
  /// Runtime error (e.g., non-exhaustive match)
  Throw {
    /// Error message
    message: String,
  },
}

#[allow(clippy::should_implement_trait)]
impl FxCoreExpr {
  // ========== Constructor helpers ==========

  /// 정수 리터럴 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn int(v: i64) -> Self {
    FxCoreExpr::ConstInt(v)
  }

  /// 실수 리터럴 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn float(v: f64) -> Self {
    FxCoreExpr::ConstFloat(v)
  }

  /// 불리언 리터럴 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn bool(v: bool) -> Self {
    FxCoreExpr::ConstBool(v)
  }

  /// 문자열 리터럴 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn string(v: impl Into<String>) -> Self {
    FxCoreExpr::ConstString(v.into())
  }

  /// 변수 참조 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn var(name: impl Into<String>) -> Self {
    FxCoreExpr::Var(name.into())
  }

  /// Signal 변수 참조 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn signal(id: usize) -> Self {
    FxCoreExpr::SignalVar(SignalId(id))
  }

  /// 시스템 시간 파라미터 생성 (param.system_time)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn time() -> Self {
    FxCoreExpr::ParamSysTime
  }

  /// 델타 시간 파라미터 생성 (param.dt)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn dt() -> Self {
    FxCoreExpr::ParamDeltaTime
  }

  // ========== Unary operations ==========

  fn unary(op: MeaningOpId, arg: FxCoreExpr) -> Self {
    FxCoreExpr::Unary {
      meta: MeaningMeta::pure(op),
      arg: Box::new(arg),
    }
  }

  /// 부정 연산 생성 (-arg)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn make_neg(arg: FxCoreExpr) -> Self {
    Self::unary(MeaningOpId::Neg, arg)
  }

  /// 부정 연산 생성 (-arg)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn neg(arg: FxCoreExpr) -> Self {
    Self::make_neg(arg)
  }

  /// 버림 연산 생성 (floor)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn floor(arg: FxCoreExpr) -> Self {
    Self::unary(MeaningOpId::Floor, arg)
  }

  /// 올림 연산 생성 (ceil)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn ceil(arg: FxCoreExpr) -> Self {
    Self::unary(MeaningOpId::Ceil, arg)
  }

  /// 절대값 연산 생성 (abs)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn abs(arg: FxCoreExpr) -> Self {
    Self::unary(MeaningOpId::Abs, arg)
  }

  /// 제곱근 연산 생성 (sqrt)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn sqrt(arg: FxCoreExpr) -> Self {
    Self::unary(MeaningOpId::Sqrt, arg)
  }

  /// 사인 연산 생성 (sin)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn sin(arg: FxCoreExpr) -> Self {
    Self::unary(MeaningOpId::Sin, arg)
  }

  /// 코사인 연산 생성 (cos)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn cos(arg: FxCoreExpr) -> Self {
    Self::unary(MeaningOpId::Cos, arg)
  }

  /// 탄젠트 연산 생성 (tan)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn tan(arg: FxCoreExpr) -> Self {
    Self::unary(MeaningOpId::Tan, arg)
  }

  /// 지수 연산 생성 (exp)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn exp(arg: FxCoreExpr) -> Self {
    Self::unary(MeaningOpId::Exp, arg)
  }

  /// 자연로그 연산 생성 (ln)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn ln(arg: FxCoreExpr) -> Self {
    Self::unary(MeaningOpId::Ln, arg)
  }

  /// 거듭제곱 연산 생성 (base^exponent)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn pow(base: FxCoreExpr, exponent: FxCoreExpr) -> Self {
    Self::binary(MeaningOpId::Pow, base, exponent)
  }

  /// 논리 부정 연산 생성 (!arg)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn make_not(arg: FxCoreExpr) -> Self {
    Self::unary(MeaningOpId::Not, arg)
  }

  /// 논리 부정 연산 생성 (!arg)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn not(arg: FxCoreExpr) -> Self {
    Self::make_not(arg)
  }

  // ========== Binary operations ==========

  fn binary(op: MeaningOpId, lhs: FxCoreExpr, rhs: FxCoreExpr) -> Self {
    FxCoreExpr::Binary {
      meta: MeaningMeta::pure(op),
      lhs: Box::new(lhs),
      rhs: Box::new(rhs),
    }
  }

  /// 덧셈 연산 생성 (lhs + rhs)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn make_add(lhs: FxCoreExpr, rhs: FxCoreExpr) -> Self {
    Self::binary(MeaningOpId::Add, lhs, rhs)
  }

  /// 덧셈 연산 생성 (lhs + rhs)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn add(lhs: FxCoreExpr, rhs: FxCoreExpr) -> Self {
    Self::make_add(lhs, rhs)
  }

  /// 뺄셈 연산 생성 (lhs - rhs)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn make_sub(lhs: FxCoreExpr, rhs: FxCoreExpr) -> Self {
    Self::binary(MeaningOpId::Sub, lhs, rhs)
  }

  /// 뺄셈 연산 생성 (lhs - rhs)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn sub(lhs: FxCoreExpr, rhs: FxCoreExpr) -> Self {
    Self::make_sub(lhs, rhs)
  }

  /// 곱셈 연산 생성 (lhs * rhs)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn make_mul(lhs: FxCoreExpr, rhs: FxCoreExpr) -> Self {
    Self::binary(MeaningOpId::Mul, lhs, rhs)
  }

  /// 곱셈 연산 생성 (lhs * rhs)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn mul(lhs: FxCoreExpr, rhs: FxCoreExpr) -> Self {
    Self::make_mul(lhs, rhs)
  }

  /// 나눗셈 연산 생성 (lhs / rhs)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn make_div(lhs: FxCoreExpr, rhs: FxCoreExpr) -> Self {
    Self::binary(MeaningOpId::Div, lhs, rhs)
  }

  /// 나눗셈 연산 생성 (lhs / rhs)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn div(lhs: FxCoreExpr, rhs: FxCoreExpr) -> Self {
    Self::make_div(lhs, rhs)
  }

  /// 나머지 연산 생성 (lhs % rhs)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn modulo(lhs: FxCoreExpr, rhs: FxCoreExpr) -> Self {
    Self::binary(MeaningOpId::Mod, lhs, rhs)
  }

  /// 작다 비교 연산 생성 (lhs < rhs)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn lt(lhs: FxCoreExpr, rhs: FxCoreExpr) -> Self {
    Self::binary(MeaningOpId::Lt, lhs, rhs)
  }

  /// 크다 비교 연산 생성 (lhs > rhs)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn gt(lhs: FxCoreExpr, rhs: FxCoreExpr) -> Self {
    Self::binary(MeaningOpId::Gt, lhs, rhs)
  }

  /// 작거나 같다 비교 연산 생성 (lhs <= rhs)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn le(lhs: FxCoreExpr, rhs: FxCoreExpr) -> Self {
    Self::binary(MeaningOpId::Le, lhs, rhs)
  }

  /// 크거나 같다 비교 연산 생성 (lhs >= rhs)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn ge(lhs: FxCoreExpr, rhs: FxCoreExpr) -> Self {
    Self::binary(MeaningOpId::Ge, lhs, rhs)
  }

  /// 같다 비교 연산 생성 (lhs == rhs)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn eq(lhs: FxCoreExpr, rhs: FxCoreExpr) -> Self {
    Self::binary(MeaningOpId::Eq, lhs, rhs)
  }

  /// 다르다 비교 연산 생성 (lhs != rhs)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn ne(lhs: FxCoreExpr, rhs: FxCoreExpr) -> Self {
    Self::binary(MeaningOpId::Ne, lhs, rhs)
  }

  /// 논리곱 연산 생성 (lhs && rhs)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn and(lhs: FxCoreExpr, rhs: FxCoreExpr) -> Self {
    Self::binary(MeaningOpId::And, lhs, rhs)
  }

  /// 논리합 연산 생성 (lhs || rhs)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn or(lhs: FxCoreExpr, rhs: FxCoreExpr) -> Self {
    Self::binary(MeaningOpId::Or, lhs, rhs)
  }

  // ========== Control flow ==========

  /// 조건문 생성 (if cond then then_ else else_)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn if_then_else(cond: FxCoreExpr, then_: FxCoreExpr, else_: FxCoreExpr) -> Self {
    FxCoreExpr::If {
      cond: Box::new(cond),
      then_: Box::new(then_),
      else_: Box::new(else_),
    }
  }

  // ========== Derived (high-level) ==========

  /// 시간에서 초 추출 (floor(t) % 60)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn seconds_from_time() -> Self {
    FxCoreExpr::Derived {
      meta: MeaningMeta::continuous(MeaningOpId::SecondsFromTime),
      args: vec![],
    }
  }

  /// 시간에서 분 추출 (floor(t / 60) % 60)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn minutes_from_time() -> Self {
    FxCoreExpr::Derived {
      meta: MeaningMeta::continuous(MeaningOpId::MinutesFromTime),
      args: vec![],
    }
  }

  /// 시간에서 시 추출 (floor(t / 3600) % 12)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn hours_from_time() -> Self {
    FxCoreExpr::Derived {
      meta: MeaningMeta::continuous(MeaningOpId::HoursFromTime),
      args: vec![],
    }
  }

  // ========== Let binding ==========

  /// Y08a-11: Let binding (lazy semantics 보존)
  /// let x = value in body
  /// Let 노드를 직접 사용하여 중복 평가 방지 및 lazy semantics 보존
  pub fn let_in(name: impl Into<String>, value: FxCoreExpr, body: FxCoreExpr) -> Self {
    FxCoreExpr::Let {
      name: name.into(),
      value: Box::new(value),
      body: Box::new(body),
    }
  }

  // ========== Fractal/CT Operations (Functor/Applicative/Monad) ==========

  /// Functor map 생성 (<$>)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn ct_fmap(func: FxCoreExpr, functor: FxCoreExpr) -> Self {
    FxCoreExpr::Derived {
      meta: MeaningMeta::pure(MeaningOpId::CtFmap),
      args: vec![func, functor],
    }
  }

  /// Applicative pure 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn ct_pure(value: FxCoreExpr) -> Self {
    FxCoreExpr::Derived {
      meta: MeaningMeta::pure(MeaningOpId::CtPure),
      args: vec![value],
    }
  }

  /// Applicative apply 생성 (<*>)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn ct_ap(wrapped_func: FxCoreExpr, wrapped_arg: FxCoreExpr) -> Self {
    FxCoreExpr::Derived {
      meta: MeaningMeta::pure(MeaningOpId::CtAp),
      args: vec![wrapped_func, wrapped_arg],
    }
  }

  /// Monad bind 생성 (>>=)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn ct_bind(monad: FxCoreExpr, func: FxCoreExpr) -> Self {
    FxCoreExpr::Derived {
      meta: MeaningMeta::pure(MeaningOpId::CtBind),
      args: vec![monad, func],
    }
  }

  /// Kleisli 합성 생성 (>=>)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn ct_kleisli(f: FxCoreExpr, g: FxCoreExpr) -> Self {
    FxCoreExpr::Derived {
      meta: MeaningMeta::pure(MeaningOpId::CtCompose),
      args: vec![f, g],
    }
  }

  // ========== List/AttrSet Operations ==========

  /// 리스트 앞에 요소 추가 (element : list)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn cons(elem: FxCoreExpr, list: FxCoreExpr) -> Self {
    FxCoreExpr::Binary {
      meta: MeaningMeta::pure(MeaningOpId::ListCons),
      lhs: Box::new(elem),
      rhs: Box::new(list),
    }
  }

  /// 리스트 또는 문자열 연결 (a ++ b)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn concat(lhs: FxCoreExpr, rhs: FxCoreExpr) -> Self {
    FxCoreExpr::Binary {
      meta: MeaningMeta::pure(MeaningOpId::Concat),
      lhs: Box::new(lhs),
      rhs: Box::new(rhs),
    }
  }

  /// 속성 집합 병합 (오른쪽이 왼쪽을 덮어씀, a // b)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn update(lhs: FxCoreExpr, rhs: FxCoreExpr) -> Self {
    FxCoreExpr::Binary {
      meta: MeaningMeta::pure(MeaningOpId::AttrSetUpdate),
      lhs: Box::new(lhs),
      rhs: Box::new(rhs),
    }
  }

  /// 속성 선택 생성 (expr.attr)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn select(expr: FxCoreExpr, attr: impl Into<String>) -> Self {
    FxCoreExpr::Select {
      expr: Box::new(expr),
      attr: attr.into(),
    }
  }

  /// ADT 값 생성자 생성 (예: Some(42), None, Ok(x), Err("msg"))
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn construct(variant: impl Into<String>, args: Vec<FxCoreExpr>) -> Self {
    FxCoreExpr::Construct {
      variant: variant.into(),
      args,
    }
  }

  // ========== Query ==========

  /// 상수인지 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn is_const(&self) -> bool {
    matches!(
      self,
      FxCoreExpr::ConstInt(_) | FxCoreExpr::ConstFloat(_) | FxCoreExpr::ConstBool(_)
    )
  }

  /// 효과 영역 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn zone(&self) -> EffectZone {
    match self {
      FxCoreExpr::Interop { meta, .. } => meta.zone,
      FxCoreExpr::Unary { meta, .. } => meta.zone,
      FxCoreExpr::Binary { meta, .. } => meta.zone,
      FxCoreExpr::Derived { meta, .. } => meta.zone,
      // Construct is pure - it just wraps values into ADT variants
      FxCoreExpr::Construct { args, .. } => {
        // Join all argument zones (if any arg is effectful, the whole construct is)
        args
          .iter()
          .map(|a| a.zone())
          .fold(EffectZone::Pure, EffectZone::join)
      }
      FxCoreExpr::Let { value, body, .. } => {
        // Y08a-11: Let의 zone은 value와 body의 zone을 결합
        value.zone().join(body.zone())
      }
      _ => EffectZone::Pure,
    }
  }

  /// 시간 성질 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn time_kind(&self) -> TimeKind {
    match self {
      FxCoreExpr::ParamSysTime | FxCoreExpr::ParamDeltaTime => TimeKind::Continuous,
      FxCoreExpr::Derived { meta, .. } => meta.time,
      FxCoreExpr::Unary { arg, .. } => arg.time_kind(),
      FxCoreExpr::Binary { lhs, rhs, .. } => match (lhs.time_kind(), rhs.time_kind()) {
        (TimeKind::Static, TimeKind::Static) => TimeKind::Static,
        _ => TimeKind::Continuous,
      },
      FxCoreExpr::If { cond, then_, else_ } => {
        let c = cond.time_kind();
        let t = then_.time_kind();
        let e = else_.time_kind();
        if c == TimeKind::Static && t == TimeKind::Static && e == TimeKind::Static {
          TimeKind::Static
        } else {
          TimeKind::Continuous
        }
      }
      FxCoreExpr::Construct { args, .. } => {
        // Static if all args are static, otherwise Continuous
        if args.iter().all(|a| a.time_kind() == TimeKind::Static) {
          TimeKind::Static
        } else {
          TimeKind::Continuous
        }
      }
      FxCoreExpr::Let { value, body, .. } => {
        // Y08a-11: Let의 time_kind는 value와 body의 time_kind를 결합
        let v = value.time_kind();
        let b = body.time_kind();
        if v == TimeKind::Static && b == TimeKind::Static {
          TimeKind::Static
        } else {
          TimeKind::Continuous
        }
      }
      // Throw - 런타임 에러 (Static으로 간주)
      FxCoreExpr::Throw { .. } => TimeKind::Static,
      _ => TimeKind::Static,
    }
  }
}

// ============================================================
// FxProgram - Program with multiple fx bindings
// ============================================================

/// 단일 fx 정의: name = fx { expr }
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 정의만, 값 계산 없음
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FxBinding {
  /// 바인딩 이름
  pub name: String,
  /// fx 표현식
  pub expr: FxCoreExpr,
}

impl FxBinding {
  /// 새 fx 바인딩 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(name: impl Into<String>, expr: FxCoreExpr) -> Self {
    Self {
      name: name.into(),
      expr,
    }
  }
}

/// 그리기 명령 (FxCore 레벨)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FxDrawCmd {
  /// 원 그리기
  Circle {
    /// 중심 X 좌표
    cx: Box<FxCoreExpr>,
    /// 중심 Y 좌표
    cy: Box<FxCoreExpr>,
    /// 반지름
    r: Box<FxCoreExpr>,
    /// 채우기 색상 (선택적)
    fill: Option<String>,
    /// 테두리 색상 (선택적)
    stroke: Option<String>,
    /// 테두리 두께
    stroke_width: f64,
  },
  /// 선 그리기
  Line {
    /// 시작점 X 좌표
    x1: Box<FxCoreExpr>,
    /// 시작점 Y 좌표
    y1: Box<FxCoreExpr>,
    /// 끝점 X 좌표
    x2: Box<FxCoreExpr>,
    /// 끝점 Y 좌표
    y2: Box<FxCoreExpr>,
    /// 선 색상 (선택적)
    color: Option<String>,
    /// 선 두께
    width: f64,
  },
  /// 사각형 그리기
  Rect {
    /// 왼쪽 상단 X 좌표
    x: Box<FxCoreExpr>,
    /// 왼쪽 상단 Y 좌표
    y: Box<FxCoreExpr>,
    /// 너비
    w: Box<FxCoreExpr>,
    /// 높이
    h: Box<FxCoreExpr>,
    /// 채우기 색상 (선택적)
    fill: Option<String>,
    /// 테두리 색상 (선택적)
    stroke: Option<String>,
    /// 모서리 반지름
    corner_radius: f64,
  },
  /// 텍스트 그리기
  Text {
    /// X 좌표
    x: Box<FxCoreExpr>,
    /// Y 좌표
    y: Box<FxCoreExpr>,
    /// 텍스트 내용
    text: String,
    /// 폰트 크기
    font_size: f64,
    /// 텍스트 색상 (선택적)
    color: Option<String>,
  },
}

/// Fx 프로그램: 여러 fx 바인딩과 그리기 명령을 포함하는 프로그램
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FxProgram {
  /// fx 바인딩 목록
  pub bindings: Vec<FxBinding>,
  /// 그리기 명령 목록
  #[serde(default)]
  pub draw_commands: Vec<FxDrawCmd>,
}

impl FxProgram {
  /// 새 fx 프로그램 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new() -> Self {
    Self {
      bindings: Vec::new(),
      draw_commands: Vec::new(),
    }
  }

  /// 바인딩 추가
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn add(&mut self, name: impl Into<String>, expr: FxCoreExpr) -> &mut Self {
    self.bindings.push(FxBinding::new(name, expr));
    self
  }

  /// 바인딩 이름 목록 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn names(&self) -> Vec<&str> {
    self.bindings.iter().map(|b| b.name.as_str()).collect()
  }

  /// 이름으로 바인딩 조회
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn get(&self, name: &str) -> Option<&FxCoreExpr> {
    self
      .bindings
      .iter()
      .find(|b| b.name == name)
      .map(|b| &b.expr)
  }

  /// 바인딩 개수 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn len(&self) -> usize {
    self.bindings.len()
  }

  /// 빈 프로그램인지 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn is_empty(&self) -> bool {
    self.bindings.is_empty()
  }

  // ========== Effect Analysis ==========

  /// 모든 바인딩의 효과 영역 분석
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn analyze_effects(&self) -> FxEffectAnalysis {
    let mut pure_bindings = Vec::new();
    let mut interop_bindings = Vec::new();
    let mut stm_bindings = Vec::new();
    let mut world_bindings = Vec::new();

    for binding in &self.bindings {
      let zone = analyze_expr_zone(&binding.expr);
      match zone {
        // Pure and pure-equivalent zones
        EffectZone::Pure | EffectZone::Symbolic | EffectZone::Frp | EffectZone::Animation => {
          pure_bindings.push(binding.name.clone())
        }
        EffectZone::Interop => interop_bindings.push(binding.name.clone()),
        EffectZone::Stm => stm_bindings.push(binding.name.clone()),
        EffectZone::World => world_bindings.push(binding.name.clone()),
      }
    }

    FxEffectAnalysis {
      pure_bindings,
      interop_bindings,
      stm_bindings,
      world_bindings,
    }
  }

  /// 프로그램이 순수한지 확인 (모든 바인딩이 Pure)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn is_pure(&self) -> bool {
    self
      .bindings
      .iter()
      .all(|b| analyze_expr_zone(&b.expr) == EffectZone::Pure)
  }

  /// 시간 의존 바인딩 찾기
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn time_dependent_bindings(&self) -> Vec<&str> {
    self
      .bindings
      .iter()
      .filter(|b| b.expr.time_kind() != TimeKind::Static)
      .map(|b| b.name.as_str())
      .collect()
  }
}

/// 효과 분석 결과: FxProgram의 효과 영역 분석 결과
#[derive(Debug, Clone)]
pub struct FxEffectAnalysis {
  /// 순수 바인딩 (CT 최적화 가능)
  pub pure_bindings: Vec<String>,
  /// 외부 호출 바인딩 (Clj/Py 호출)
  pub interop_bindings: Vec<String>,
  /// STM 바인딩 (ref/atom/agent)
  pub stm_bindings: Vec<String>,
  /// World 바인딩 (IO/channels)
  pub world_bindings: Vec<String>,
}

impl FxEffectAnalysis {
  /// 프로그램이 순수한지 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn is_pure(&self) -> bool {
    self.interop_bindings.is_empty()
      && self.stm_bindings.is_empty()
      && self.world_bindings.is_empty()
  }

  /// 전체 바인딩 개수 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn total_bindings(&self) -> usize {
    self.pure_bindings.len()
      + self.interop_bindings.len()
      + self.stm_bindings.len()
      + self.world_bindings.len()
  }
}

/// 표현식의 전체 효과 영역 분석 (재귀적)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 분석만, 값 계산 없음
pub fn analyze_expr_zone(expr: &FxCoreExpr) -> EffectZone {
  match expr {
    FxCoreExpr::ConstInt(_)
    | FxCoreExpr::ConstFloat(_)
    | FxCoreExpr::ConstBool(_)
    | FxCoreExpr::ConstString(_)
    | FxCoreExpr::ParamSysTime
    | FxCoreExpr::ParamDeltaTime
    | FxCoreExpr::SignalVar(_)
    | FxCoreExpr::Var(_) => EffectZone::Pure,

    FxCoreExpr::Unary { meta, arg } => combine_zones(meta.zone, analyze_expr_zone(arg)),

    FxCoreExpr::Binary { meta, lhs, rhs } => {
      let z1 = combine_zones(meta.zone, analyze_expr_zone(lhs));
      combine_zones(z1, analyze_expr_zone(rhs))
    }

    FxCoreExpr::Derived { meta, args } => {
      let mut zone = meta.zone;
      for arg in args {
        zone = combine_zones(zone, analyze_expr_zone(arg));
      }
      zone
    }

    FxCoreExpr::If { cond, then_, else_ } => {
      let z1 = analyze_expr_zone(cond);
      let z2 = analyze_expr_zone(then_);
      let z3 = analyze_expr_zone(else_);
      combine_zones(combine_zones(z1, z2), z3)
    }

    FxCoreExpr::Let {
      value,
      body,
      name: _,
      ..
    } => {
      // Y08a-11: Let의 zone은 value와 body의 zone을 결합
      //
      // **CRITICAL**: Lazy semantics 위반
      // 현재 구현은 value의 zone을 항상 포함하지만, lazy evaluation에서는
      // body가 name을 사용하지 않으면 value가 평가되지 않으므로 value의 zone을 포함하지 않아야 함
      //
      // 향후 개선: free variable analysis를 통해 body에서 name이 사용되는지 확인
      // if body_uses_name(body, name) {
      //   combine_zones(analyze_expr_zone(value), analyze_expr_zone(body))
      // } else {
      //   analyze_expr_zone(body) // value는 평가되지 않으므로 zone 제외
      // }
      let z1 = analyze_expr_zone(value);
      let z2 = analyze_expr_zone(body);
      combine_zones(z1, z2)
    }

    FxCoreExpr::List(items) => {
      let mut zone = EffectZone::Pure;
      for item in items {
        zone = combine_zones(zone, analyze_expr_zone(item));
      }
      zone
    }

    FxCoreExpr::AttrSet(pairs) => {
      let mut zone = EffectZone::Pure;
      for (_, v) in pairs {
        zone = combine_zones(zone, analyze_expr_zone(v));
      }
      zone
    }

    FxCoreExpr::Interop { meta, .. } => meta.zone,

    // Lambda - analyze body's zone (lambda itself is Pure)
    FxCoreExpr::Lambda { body, .. } => analyze_expr_zone(body),

    // Select - analyze target expression's zone
    FxCoreExpr::Select { expr, .. } => analyze_expr_zone(expr),

    // Construct - analyze all argument zones
    FxCoreExpr::Construct { args, .. } => {
      let mut zone = EffectZone::Pure;
      for arg in args {
        zone = combine_zones(zone, analyze_expr_zone(arg));
      }
      zone
    }

    // Throw - 런타임 에러 (Pure으로 간주)
    FxCoreExpr::Throw { .. } => EffectZone::Pure,
  }
}

/// Combine two effect zones (select stronger effect)
fn combine_zones(z1: EffectZone, z2: EffectZone) -> EffectZone {
  use EffectZone::*;
  match (z1, z2) {
    (Pure, z) | (z, Pure) => z,
    (Symbolic, z) | (z, Symbolic) => z.join(Symbolic),
    (World, _) | (_, World) => World,
    (Interop, _) | (_, Interop) => Interop,
    (Stm, Stm) => Stm,
    (Frp, Frp) => Frp,
    (Animation, Animation) => Animation,
    // Other combinations use join
    _ => z1.join(z2),
  }
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_fx_core_expr_construction() {
    let expr = FxCoreExpr::add(FxCoreExpr::int(1), FxCoreExpr::int(2));

    match expr {
      FxCoreExpr::Binary { meta, .. } => {
        assert_eq!(meta.op, MeaningOpId::Add);
      }
      _ => panic!("Expected Binary"),
    }
  }

  #[test]
  fn test_fx_core_time_kind() {
    let static_expr = FxCoreExpr::int(42);
    assert_eq!(static_expr.time_kind(), TimeKind::Static);

    let time_expr = FxCoreExpr::time();
    assert_eq!(time_expr.time_kind(), TimeKind::Continuous);

    let derived_expr = FxCoreExpr::add(FxCoreExpr::time(), FxCoreExpr::int(1));
    assert_eq!(derived_expr.time_kind(), TimeKind::Continuous);
  }

  #[test]
  fn test_seconds_from_time() {
    let expr = FxCoreExpr::seconds_from_time();
    assert_eq!(expr.time_kind(), TimeKind::Continuous);
    assert_eq!(expr.zone(), EffectZone::Pure);
  }

  // ========== Effect Analysis Tests ==========

  #[test]
  fn test_effect_analysis_pure() {
    let mut prog = FxProgram::new();
    prog.add("x", FxCoreExpr::int(42));
    prog.add(
      "y",
      FxCoreExpr::add(FxCoreExpr::time(), FxCoreExpr::float(1.0)),
    );

    let analysis = prog.analyze_effects();
    assert!(analysis.is_pure());
    assert_eq!(analysis.pure_bindings.len(), 2);
    assert!(analysis.interop_bindings.is_empty());
  }

  #[test]
  fn test_effect_analysis_interop() {
    let mut prog = FxProgram::new();
    prog.add("pure", FxCoreExpr::int(1));
    prog.add(
      "interop",
      FxCoreExpr::Interop {
        meta: MeaningMeta::interop(MeaningOpId::InteropClj),
        lang: "clj".to_string(),
        code: "(+ 1 2)".to_string(),
      },
    );

    let analysis = prog.analyze_effects();
    assert!(!analysis.is_pure());
    assert_eq!(analysis.pure_bindings.len(), 1);
    assert_eq!(analysis.interop_bindings.len(), 1);
  }

  #[test]
  fn test_program_is_pure() {
    let mut prog = FxProgram::new();
    prog.add("a", FxCoreExpr::floor(FxCoreExpr::time()));
    prog.add(
      "b",
      FxCoreExpr::modulo(FxCoreExpr::var("a"), FxCoreExpr::int(60)),
    );

    assert!(prog.is_pure());
  }

  #[test]
  fn test_time_dependent_bindings() {
    let mut prog = FxProgram::new();
    prog.add("static", FxCoreExpr::int(42));
    prog.add("time_dep", FxCoreExpr::floor(FxCoreExpr::time()));
    prog.add(
      "also_static",
      FxCoreExpr::add(FxCoreExpr::int(1), FxCoreExpr::int(2)),
    );

    let time_deps = prog.time_dependent_bindings();
    assert_eq!(time_deps.len(), 1);
    assert_eq!(time_deps[0], "time_dep");
  }

  #[test]
  fn test_analyze_expr_zone_recursive() {
    // Pure expression
    let pure = FxCoreExpr::add(FxCoreExpr::int(1), FxCoreExpr::int(2));
    assert_eq!(analyze_expr_zone(&pure), EffectZone::Pure);

    // Interop embedded
    let interop = FxCoreExpr::add(
      FxCoreExpr::Interop {
        meta: MeaningMeta::interop(MeaningOpId::InteropClj),
        lang: "clj".to_string(),
        code: "1".to_string(),
      },
      FxCoreExpr::int(2),
    );
    assert_eq!(analyze_expr_zone(&interop), EffectZone::Interop);
  }

  #[test]
  fn test_signal_id() {
    let id1 = SignalId(0);
    let id2 = SignalId(1);
    assert_ne!(id1, id2);
    assert_eq!(id1, SignalId(0));
  }

  #[test]
  fn test_fx_binding() {
    let binding = FxBinding::new("test", FxCoreExpr::int(42));
    assert_eq!(binding.name, "test");
    assert!(matches!(binding.expr, FxCoreExpr::ConstInt(42)));
  }

  #[test]
  fn test_program_get() {
    let mut prog = FxProgram::new();
    prog.add("x", FxCoreExpr::int(1));
    prog.add("y", FxCoreExpr::int(2));

    assert!(prog.get("x").is_some());
    assert!(prog.get("y").is_some());
    assert!(prog.get("z").is_none());
  }

  #[test]
  fn test_ct_operations() {
    let fmapped = FxCoreExpr::ct_fmap(FxCoreExpr::var("f"), FxCoreExpr::var("functor"));
    assert!(matches!(fmapped, FxCoreExpr::Derived { .. }));

    let pure = FxCoreExpr::ct_pure(FxCoreExpr::int(1));
    assert!(matches!(pure, FxCoreExpr::Derived { .. }));

    let bind = FxCoreExpr::ct_bind(FxCoreExpr::var("m"), FxCoreExpr::var("f"));
    assert!(matches!(bind, FxCoreExpr::Derived { .. }));
  }

  #[test]
  fn test_list_operations() {
    let cons = FxCoreExpr::cons(
      FxCoreExpr::int(1),
      FxCoreExpr::List(vec![FxCoreExpr::int(2)]),
    );
    assert!(matches!(cons, FxCoreExpr::Binary { .. }));

    let concat = FxCoreExpr::concat(FxCoreExpr::List(vec![]), FxCoreExpr::List(vec![]));
    assert!(matches!(concat, FxCoreExpr::Binary { .. }));
  }

  #[test]
  fn test_serde() {
    let expr = FxCoreExpr::add(FxCoreExpr::int(1), FxCoreExpr::float(2.0));
    let json = serde_json::to_string(&expr).unwrap();
    let restored: FxCoreExpr = serde_json::from_str(&json).unwrap();

    match restored {
      FxCoreExpr::Binary { meta, .. } => {
        assert_eq!(meta.op, MeaningOpId::Add);
      }
      _ => panic!("Expected Binary"),
    }
  }
}
