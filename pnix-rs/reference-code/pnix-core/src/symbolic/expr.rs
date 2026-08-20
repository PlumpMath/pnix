//! Pnix 공식 Symbolic AST
//!
//! pnix-old의 symbolic_core/ast/expr.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 순수 AST 구조 정의, 값 계산 없음
//!
//! ## 직렬화 포맷
//!
//! 모든 AST 타입은 serde를 통해 JSON 직렬화를 지원합니다.

use super::number::{NumberValue, NumberValueError};
use serde::{Deserialize, Serialize};

/// 소스 위치 정보 (에러 메시지용)
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
  pub start: usize,
  pub end: usize,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub file: Option<String>,
}

impl Span {
  /// 새 Span 생성
  pub fn new(start: usize, end: usize) -> Self {
    Self {
      start,
      end,
      file: None,
    }
  }

  /// 파일 정보 포함 Span 생성
  pub fn with_file(start: usize, end: usize, file: impl Into<String>) -> Self {
    Self {
      start,
      end,
      file: Some(file.into()),
    }
  }

  /// 길이
  pub fn len(&self) -> usize {
    self.end.saturating_sub(self.start)
  }

  /// 비어있는지 확인
  pub fn is_empty(&self) -> bool {
    self.start >= self.end
  }
}

/// Zone: Symbolic / Numeric 구분
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Zone {
  #[default]
  Symbolic,
  Numeric,
}

/// 텐서 심볼
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TensorSymbol {
  /// 텐서 이름
  pub name: String,
  /// 인덱스 목록
  pub indices: Vec<TensorIndex>,
  /// 대칭성 정보
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub symmetries: Vec<Symmetry>,
}

/// 텐서 인덱스
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TensorIndex {
  /// 인덱스 이름
  pub name: String,
  /// 인덱스 위치 (상, 하)
  pub position: IndexPosition,
  /// 인덱스가 속한 벡터 공간 (CT 검증용)
  #[serde(default, skip_serializing_if = "String::is_empty")]
  pub space: String,
}

impl TensorIndex {
  /// 새 인덱스 생성
  pub fn new(name: impl Into<String>, position: IndexPosition) -> Self {
    Self {
      name: name.into(),
      position,
      space: String::new(),
    }
  }

  /// 공간 지정하여 새 인덱스 생성
  pub fn with_space(
    name: impl Into<String>,
    position: IndexPosition,
    space: impl Into<String>,
  ) -> Self {
    Self {
      name: name.into(),
      position,
      space: space.into(),
    }
  }

  /// 상 인덱스 생성 (편의 함수)
  pub fn up(name: impl Into<String>, space: impl Into<String>) -> Self {
    Self::with_space(name, IndexPosition::Upper, space)
  }

  /// 하 인덱스 생성 (편의 함수)
  pub fn down(name: impl Into<String>, space: impl Into<String>) -> Self {
    Self::with_space(name, IndexPosition::Lower, space)
  }

  /// 수축 가능한지 확인 (반대 position, 같은 space)
  pub fn contracts_with(&self, other: &TensorIndex) -> bool {
    self.name == other.name
      && self.position != other.position
      && (self.space.is_empty() || other.space.is_empty() || self.space == other.space)
  }
}

/// 인덱스 위치
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IndexPosition {
  Upper,
  Lower,
}

/// 텐서 대칭성
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "indices")]
pub enum Symmetry {
  /// 특정 인덱스 위치들이 대칭: g_{μν} = g_{νμ}
  Symmetric(Vec<usize>),
  /// 특정 인덱스 위치들이 반대칭: F_{μν} = -F_{νμ}
  AntiSymmetric(Vec<usize>),
}

/// 심볼릭 표현의 메인 타입
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SymExpr {
  pub kind: SymKind,
  pub zone: Zone,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub span: Option<Span>,
}

impl SymExpr {
  // ========== 변수/상수 생성자 ==========

  pub fn var(name: impl Into<String>) -> Self {
    Self {
      kind: SymKind::Var(name.into()),
      zone: Zone::Symbolic,
      span: None,
    }
  }

  pub fn constant(value: f64) -> Self {
    Self {
      kind: SymKind::Const(value),
      zone: Zone::Symbolic,
      span: None,
    }
  }

  /// 정수 상수 (정확한 연산)
  pub fn int(value: i64) -> Self {
    Self {
      kind: SymKind::Exact(NumberValue::int(value)),
      zone: Zone::Symbolic,
      span: None,
    }
  }

  /// 유리수 상수 (정확한 연산)
  ///
  /// 분모가 0이면 에러를 반환합니다.
  pub fn ratio(numer: i64, denom: i64) -> Result<Self, NumberValueError> {
    Ok(Self {
      kind: SymKind::Exact(NumberValue::ratio(numer, denom)?),
      zone: Zone::Symbolic,
      span: None,
    })
  }

  /// 정확한 수치 상수 (NumberValue)
  pub fn exact(value: NumberValue) -> Self {
    Self {
      kind: SymKind::Exact(value),
      zone: Zone::Symbolic,
      span: None,
    }
  }

  // ========== 연산 생성자 ==========

  pub fn add(exprs: Vec<SymExpr>) -> Self {
    Self {
      kind: SymKind::Add(exprs),
      zone: Zone::Symbolic,
      span: None,
    }
  }

  /// 이항 덧셈 (2개 인자)
  pub fn add2(a: SymExpr, b: SymExpr) -> Self {
    Self::add(vec![a, b])
  }

  pub fn mul(exprs: Vec<SymExpr>) -> Self {
    Self {
      kind: SymKind::Mul(exprs),
      zone: Zone::Symbolic,
      span: None,
    }
  }

  /// 이항 곱셈 (2개 인자)
  pub fn mul2(a: SymExpr, b: SymExpr) -> Self {
    Self::mul(vec![a, b])
  }

  pub fn pow(base: SymExpr, exp: SymExpr) -> Self {
    Self {
      kind: SymKind::Pow(Box::new(base), Box::new(exp)),
      zone: Zone::Symbolic,
      span: None,
    }
  }

  #[allow(clippy::should_implement_trait)]
  pub fn neg(expr: SymExpr) -> Self {
    Self {
      kind: SymKind::Neg(Box::new(expr)),
      zone: Zone::Symbolic,
      span: None,
    }
  }

  pub fn tensor(sym: TensorSymbol) -> Self {
    Self {
      kind: SymKind::Tensor(sym),
      zone: Zone::Symbolic,
      span: None,
    }
  }

  // ========== 삼각/지수 함수 ==========

  /// sin(x)
  pub fn sin(expr: SymExpr) -> Self {
    Self {
      kind: SymKind::Sin(Box::new(expr)),
      zone: Zone::Symbolic,
      span: None,
    }
  }

  /// cos(x)
  pub fn cos(expr: SymExpr) -> Self {
    Self {
      kind: SymKind::Cos(Box::new(expr)),
      zone: Zone::Symbolic,
      span: None,
    }
  }

  /// tan(x)
  pub fn tan(expr: SymExpr) -> Self {
    Self {
      kind: SymKind::Tan(Box::new(expr)),
      zone: Zone::Symbolic,
      span: None,
    }
  }

  /// exp(x)
  pub fn exp(expr: SymExpr) -> Self {
    Self {
      kind: SymKind::Exp(Box::new(expr)),
      zone: Zone::Symbolic,
      span: None,
    }
  }

  /// log(x)
  pub fn log(expr: SymExpr) -> Self {
    Self {
      kind: SymKind::Log(Box::new(expr)),
      zone: Zone::Symbolic,
      span: None,
    }
  }

  /// sqrt(x) = x^0.5
  pub fn sqrt(expr: SymExpr) -> Self {
    Self::pow(expr, Self::constant(0.5))
  }

  /// 절댓값
  pub fn abs(expr: SymExpr) -> Self {
    Self {
      kind: SymKind::Abs(Box::new(expr)),
      zone: Zone::Symbolic,
      span: None,
    }
  }

  // ========== 미분 ==========

  /// 미분
  pub fn derivative(expr: SymExpr, var: impl Into<String>) -> Self {
    Self {
      kind: SymKind::Derivative(Box::new(expr), var.into()),
      zone: Zone::Symbolic,
      span: None,
    }
  }

  // ========== 텐서 연산 ==========

  /// 텐서 수축
  pub fn contract(expr: SymExpr, idx1: impl Into<String>, idx2: impl Into<String>) -> Self {
    Self {
      kind: SymKind::Contract(Box::new(expr), idx1.into(), idx2.into()),
      zone: Zone::Symbolic,
      span: None,
    }
  }

  /// 인덱스 올리기
  pub fn raise(expr: SymExpr, idx: impl Into<String>) -> Self {
    Self {
      kind: SymKind::Raise(Box::new(expr), idx.into()),
      zone: Zone::Symbolic,
      span: None,
    }
  }

  /// 인덱스 내리기
  pub fn lower(expr: SymExpr, idx: impl Into<String>) -> Self {
    Self {
      kind: SymKind::Lower(Box::new(expr), idx.into()),
      zone: Zone::Symbolic,
      span: None,
    }
  }

  // ========== 빌더 메서드 ==========

  /// Zone 설정
  pub fn with_zone(mut self, zone: Zone) -> Self {
    self.zone = zone;
    self
  }

  /// Span 설정
  pub fn with_span(mut self, span: Span) -> Self {
    self.span = Some(span);
    self
  }

  /// 함수 호출 (name, args) -> 내장 함수로 변환
  pub fn call(name: &str, mut args: Vec<SymExpr>) -> Self {
    if args.len() == 1 {
      let arg = args.remove(0);
      match name {
        "sin" => Self::sin(arg),
        "cos" => Self::cos(arg),
        "tan" => Self::tan(arg),
        "exp" => Self::exp(arg),
        "log" | "ln" => Self::log(arg),
        "sqrt" => Self::sqrt(arg),
        "abs" => Self::abs(arg),
        _ => Self::var(name), // fallback
      }
    } else if args.len() == 2 && name == "pow" {
      let base = args.remove(0);
      let exp = args.remove(0);
      Self::pow(base, exp)
    } else {
      // 다인자 함수는 지원하지 않음
      Self::var(name)
    }
  }

  // ========== 구조 분석 ==========

  /// 변수인지 확인
  pub fn is_var(&self) -> bool {
    matches!(self.kind, SymKind::Var(_))
  }

  /// 상수인지 확인
  pub fn is_const(&self) -> bool {
    matches!(self.kind, SymKind::Const(_) | SymKind::Exact(_))
  }

  /// 덧셈인지 확인
  pub fn is_add(&self) -> bool {
    matches!(self.kind, SymKind::Add(_))
  }

  /// 곱셈인지 확인
  pub fn is_mul(&self) -> bool {
    matches!(self.kind, SymKind::Mul(_))
  }

  /// 거듭제곱인지 확인
  pub fn is_pow(&self) -> bool {
    matches!(self.kind, SymKind::Pow(_, _))
  }
}

/// 심볼릭 표현의 종류
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum SymKind {
  // ========== 리프 노드 ==========
  /// 변수
  Var(
    /// 변수 이름
    String,
  ),
  /// f64 상수 (레거시, Exact 권장)
  Const(
    /// 상수 값
    f64,
  ),
  /// 정확한 수치 (정수/유리수/부동소수점)
  Exact(
    /// 수치 값
    NumberValue,
  ),
  /// 텐서
  Tensor(
    /// 텐서 심볼
    TensorSymbol,
  ),

  // ========== 산술 연산 ==========
  /// 덧셈 (n-ary)
  Add(
    /// 덧셈할 표현식 목록
    Vec<SymExpr>,
  ),
  /// 곱셈 (n-ary)
  Mul(
    /// 곱셈할 표현식 목록
    Vec<SymExpr>,
  ),
  /// 거듭제곱
  Pow(
    /// 밑
    Box<SymExpr>,
    /// 지수
    Box<SymExpr>,
  ),
  /// 부정
  Neg(
    /// 부정할 표현식
    Box<SymExpr>,
  ),

  // ========== 삼각/지수 함수 ==========
  /// 사인 함수
  Sin(
    /// 인자 표현식
    Box<SymExpr>,
  ),
  /// 코사인 함수
  Cos(
    /// 인자 표현식
    Box<SymExpr>,
  ),
  /// 탄젠트 함수
  Tan(
    /// 인자 표현식
    Box<SymExpr>,
  ),
  /// 지수 함수 (e^x)
  Exp(
    /// 인자 표현식
    Box<SymExpr>,
  ),
  /// 자연 로그 함수
  Log(
    /// 인자 표현식
    Box<SymExpr>,
  ),
  /// 절댓값 함수
  Abs(
    /// 인자 표현식
    Box<SymExpr>,
  ),

  // ========== 미분 ==========
  /// 미분: 변수 var에 대한 expr의 미분
  Derivative(
    /// 미분할 표현식
    Box<SymExpr>,
    /// 미분 변수 이름
    String,
  ),

  // ========== 텐서 연산 ==========
  /// 텐서 수축: idx1과 idx2를 수축
  Contract(
    /// 수축할 표현식
    Box<SymExpr>,
    /// 첫 번째 인덱스 이름
    String,
    /// 두 번째 인덱스 이름
    String,
  ),
  /// 인덱스 올리기: 인덱스 idx를 올림 (metric 사용)
  Raise(
    /// 올릴 표현식
    Box<SymExpr>,
    /// 인덱스 이름
    String,
  ),
  /// 인덱스 내리기: 인덱스 idx를 내림 (metric 사용)
  Lower(
    /// 내릴 표현식
    Box<SymExpr>,
    /// 인덱스 이름
    String,
  ),
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::approx_constant)]
mod tests {
  use super::*;

  #[test]
  fn test_var() {
    let x = SymExpr::var("x");
    assert!(x.is_var());
    assert!(!x.is_const());
  }

  #[test]
  fn test_const() {
    let c = SymExpr::constant(3.14);
    assert!(c.is_const());
    assert!(!c.is_var());
  }

  #[test]
  fn test_int() {
    let n = SymExpr::int(42);
    assert!(n.is_const());
    match n.kind {
      SymKind::Exact(NumberValue::Integer(v)) => assert_eq!(v, 42),
      _ => panic!("Expected Exact(Integer)"),
    }
  }

  #[test]
  fn test_ratio() {
    let r = SymExpr::ratio(1, 2).unwrap();
    assert!(r.is_const());
    match r.kind {
      SymKind::Exact(NumberValue::Rational { numer, denom }) => {
        assert_eq!(numer, 1);
        assert_eq!(denom, 2);
      }
      _ => panic!("Expected Exact(Rational)"),
    }
  }

  #[test]
  fn test_add() {
    let sum = SymExpr::add2(SymExpr::var("x"), SymExpr::var("y"));
    assert!(sum.is_add());
  }

  #[test]
  fn test_mul() {
    let prod = SymExpr::mul2(SymExpr::var("x"), SymExpr::var("y"));
    assert!(prod.is_mul());
  }

  #[test]
  fn test_pow() {
    let p = SymExpr::pow(SymExpr::var("x"), SymExpr::int(2));
    assert!(p.is_pow());
  }

  #[test]
  fn test_sin_cos() {
    let s = SymExpr::sin(SymExpr::var("x"));
    assert!(matches!(s.kind, SymKind::Sin(_)));

    let c = SymExpr::cos(SymExpr::var("x"));
    assert!(matches!(c.kind, SymKind::Cos(_)));
  }

  #[test]
  fn test_derivative() {
    let d = SymExpr::derivative(SymExpr::pow(SymExpr::var("x"), SymExpr::int(2)), "x");
    match d.kind {
      SymKind::Derivative(_, var) => assert_eq!(var, "x"),
      _ => panic!("Expected Derivative"),
    }
  }

  #[test]
  fn test_call() {
    let s = SymExpr::call("sin", vec![SymExpr::var("x")]);
    assert!(matches!(s.kind, SymKind::Sin(_)));

    let sq = SymExpr::call("sqrt", vec![SymExpr::var("x")]);
    assert!(sq.is_pow()); // sqrt is x^0.5
  }

  #[test]
  fn test_span() {
    let span = Span::new(0, 10);
    assert_eq!(span.len(), 10);
    assert!(!span.is_empty());

    let empty = Span::new(5, 5);
    assert!(empty.is_empty());
  }

  #[test]
  fn test_with_span() {
    let x = SymExpr::var("x").with_span(Span::new(0, 1));
    assert!(x.span.is_some());
  }

  #[test]
  fn test_zone() {
    let x = SymExpr::var("x");
    assert_eq!(x.zone, Zone::Symbolic);

    let y = SymExpr::var("y").with_zone(Zone::Numeric);
    assert_eq!(y.zone, Zone::Numeric);
  }

  #[test]
  fn test_serde() {
    let expr = SymExpr::add2(SymExpr::var("x"), SymExpr::int(1));
    let json = serde_json::to_string(&expr).unwrap();
    let restored: SymExpr = serde_json::from_str(&json).unwrap();
    assert_eq!(expr, restored);
  }
}
