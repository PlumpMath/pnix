//! Parametric constraint IR for WYSIWYG synthesis.
//!
//! Pure data types only.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 컨텍스트 모드: 파라미터 표현식의 실행 컨텍스트 모드
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextMode {
  /// 순수 모드 (시간 독립적)
  Pure,
  /// 실시간 모드 (시간 의존적)
  Realtime,
}

/// 단위: 물리량의 차원 정보
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Unit {
  /// 차원 맵 (차원 이름 → 지수)
  pub dims: BTreeMap<String, i32>,
}

impl Unit {
  /// 새 단위 생성 (지수 1)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(name: impl Into<String>) -> Self {
    let mut dims = BTreeMap::new();
    dims.insert(name.into(), 1);
    Self { dims }
  }

  /// 무차원 단위 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn dimensionless() -> Self {
    Self {
      dims: BTreeMap::new(),
    }
  }

  /// 무차원인지 확인
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 분석만, 값 계산 없음
  pub fn is_dimensionless(&self) -> bool {
    self.dims.is_empty()
  }

  /// 단위 곱셈
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn mul(&self, other: &Unit) -> Unit {
    let mut dims = self.dims.clone();
    for (k, v) in &other.dims {
      *dims.entry(k.clone()).or_insert(0) += *v;
      if dims.get(k) == Some(&0) {
        dims.remove(k);
      }
    }
    Unit { dims }
  }

  /// 단위 나눗셈
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn div(&self, other: &Unit) -> Unit {
    let mut dims = self.dims.clone();
    for (k, v) in &other.dims {
      *dims.entry(k.clone()).or_insert(0) -= *v;
      if dims.get(k) == Some(&0) {
        dims.remove(k);
      }
    }
    Unit { dims }
  }

  /// 단위 거듭제곱
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn pow_i32(&self, exp: i32) -> Unit {
    if exp == 0 {
      return Unit::dimensionless();
    }
    let mut dims = BTreeMap::new();
    for (k, v) in &self.dims {
      let new_v = v.saturating_mul(exp);
      if new_v != 0 {
        dims.insert(k.clone(), new_v);
      }
    }
    Unit { dims }
  }

  /// 단위 레이블 문자열 생성
  ///
  /// ## 헌법 준수 (P0-1, C1)
  ///
  /// 텍스트 생성만, 파일 I/O 없음
  pub fn label(&self) -> String {
    if self.dims.is_empty() {
      return "1".to_string();
    }
    let mut parts = Vec::new();
    for (k, v) in &self.dims {
      if *v == 1 {
        parts.push(k.clone());
      } else {
        parts.push(format!("{}^{}", k, v));
      }
    }
    parts.join("*")
  }
}

/// 단위 변환 스케일: 단위 간 변환 계수
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnitScale {
  /// 출발 단위
  pub from: Unit,
  /// 도착 단위
  pub to: Unit,
  /// 변환 계수 (from 단위 값 * factor = to 단위 값)
  pub factor: f64,
}

/// 파라미터 역할: 파라미터의 역할 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParamRole {
  /// 입력 파라미터
  Input,
  /// 출력 파라미터
  Output,
  /// 파생 파라미터 (제약으로부터 계산됨)
  Derived,
}

/// Provenance 태그: 값의 출처 정보
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceTag {
  /// 고유 식별자
  pub uid: String,
  /// 선택적 레이블 (사람이 읽을 수 있는 설명)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub label: Option<String>,
}

/// 파라미터 변수: 파라미터 변수 정의
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParamVar {
  /// 변수 이름
  pub name: String,
  /// 파라미터 역할
  pub role: ParamRole,
  /// 단위 (선택적)
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub unit: Option<Unit>,
  /// Provenance 태그 (선택적)
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub provenance: Option<ProvenanceTag>,
}

impl ParamVar {
  /// 새 파라미터 변수 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(name: impl Into<String>, role: ParamRole) -> Self {
    Self {
      name: name.into(),
      role,
      unit: None,
      provenance: None,
    }
  }
}

/// 시그널 참조: FRP 시그널 참조
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignalRef {
  /// 시그널 이름
  pub name: String,
  /// 단위 (선택적)
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub unit: Option<Unit>,
  /// Provenance 태그 (선택적)
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub provenance: Option<ProvenanceTag>,
}

impl SignalRef {
  /// 새 시그널 참조 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(name: impl Into<String>) -> Self {
    Self {
      name: name.into(),
      unit: None,
      provenance: None,
    }
  }
}

/// 파라미터 값: 파라미터 표현식의 상수 값
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum ParamValue {
  /// 정수 값
  Int(
    /// 정수 값
    i64,
  ),
  /// 실수 값
  Float(
    /// 실수 값
    f64,
  ),
}

/// 파라미터 단항 연산자: 파라미터 표현식의 단항 연산자
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParamUnaryOp {
  /// 부호 반전 (-)
  Neg,
  /// 내림 (floor)
  Floor,
  /// 올림 (ceil)
  Ceil,
  /// 절댓값 (abs)
  Abs,
  /// 제곱근 (sqrt)
  Sqrt,
  /// 사인 (sin)
  Sin,
  /// 코사인 (cos)
  Cos,
  /// 탄젠트 (tan)
  Tan,
  /// 지수 (exp)
  Exp,
  /// 자연 로그 (ln)
  Ln,
}

/// 파라미터 이항 연산자: 파라미터 표현식의 이항 연산자
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParamBinaryOp {
  /// 덧셈 (+)
  Add,
  /// 뺄셈 (-)
  Sub,
  /// 곱셈 (*)
  Mul,
  /// 나눗셈 (/)
  Div,
  /// 나머지 (%)
  Mod,
  /// 거듭제곱 (^)
  Pow,
}

/// 파라미터 표현식: 파라미터 제약 표현식
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParamExpr {
  /// 표현식 종류
  pub kind: ParamExprKind,
  /// Provenance 태그 (선택적)
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub provenance: Option<ProvenanceTag>,
}

/// 파라미터 표현식 종류: 파라미터 표현식의 종류
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ParamExprKind {
  /// 상수 값
  Const(
    /// 값
    ParamValue,
  ),
  /// 변수 참조
  Var(
    /// 변수 이름
    String,
  ),
  /// 시그널 참조
  Signal(
    /// 시그널 이름
    String,
  ),
  /// 단항 연산
  Unary {
    /// 연산자
    op: ParamUnaryOp,
    /// 인자 표현식
    arg: Box<ParamExpr>,
  },
  /// 이항 연산
  Binary {
    /// 연산자
    op: ParamBinaryOp,
    /// 왼쪽 피연산자
    lhs: Box<ParamExpr>,
    /// 오른쪽 피연산자
    rhs: Box<ParamExpr>,
  },
  /// 단위 변환
  Convert {
    /// 변환할 표현식
    arg: Box<ParamExpr>,
    /// 출발 단위
    from: Unit,
    /// 도착 단위
    to: Unit,
    /// 변환 계수
    factor: f64,
  },
  /// 함수 호출
  Call {
    /// 함수 이름
    func: String,
    /// 인자 표현식 목록
    args: Vec<ParamExpr>,
  },
}

impl ParamExpr {
  /// 정수 리터럴 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn int(v: i64) -> Self {
    Self {
      kind: ParamExprKind::Const(ParamValue::Int(v)),
      provenance: None,
    }
  }

  /// 실수 리터럴 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn float(v: f64) -> Self {
    Self {
      kind: ParamExprKind::Const(ParamValue::Float(v)),
      provenance: None,
    }
  }

  /// 변수 참조 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn var(name: impl Into<String>) -> Self {
    Self {
      kind: ParamExprKind::Var(name.into()),
      provenance: None,
    }
  }

  /// 시그널 참조 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn signal(name: impl Into<String>) -> Self {
    Self {
      kind: ParamExprKind::Signal(name.into()),
      provenance: None,
    }
  }

  /// 단항 연산 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn unary(op: ParamUnaryOp, arg: ParamExpr) -> Self {
    Self {
      kind: ParamExprKind::Unary {
        op,
        arg: Box::new(arg),
      },
      provenance: None,
    }
  }

  /// 이항 연산 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn binary(op: ParamBinaryOp, lhs: ParamExpr, rhs: ParamExpr) -> Self {
    Self {
      kind: ParamExprKind::Binary {
        op,
        lhs: Box::new(lhs),
        rhs: Box::new(rhs),
      },
      provenance: None,
    }
  }

  /// 단위 변환 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn convert(arg: ParamExpr, from: Unit, to: Unit, factor: f64) -> Self {
    Self {
      kind: ParamExprKind::Convert {
        arg: Box::new(arg),
        from,
        to,
        factor,
      },
      provenance: None,
    }
  }

  /// 함수 호출 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn call(func: impl Into<String>, args: Vec<ParamExpr>) -> Self {
    Self {
      kind: ParamExprKind::Call {
        func: func.into(),
        args,
      },
      provenance: None,
    }
  }
}

/// 제약 표현식: 파라미터 제약 조건 표현식
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ConstraintExpr {
  /// 등식: left = right
  Eq {
    /// 왼쪽 표현식
    left: ParamExpr,
    /// 오른쪽 표현식
    right: ParamExpr,
  },
  /// 부등식 (≤): left ≤ right
  Le {
    /// 왼쪽 표현식
    left: ParamExpr,
    /// 오른쪽 표현식
    right: ParamExpr,
  },
  /// 부등식 (≥): left ≥ right
  Ge {
    /// 왼쪽 표현식
    left: ParamExpr,
    /// 오른쪽 표현식
    right: ParamExpr,
  },
  /// 범위 제약: min ≤ expr ≤ max
  Range {
    /// 대상 표현식
    expr: ParamExpr,
    /// 최소값 표현식
    min: ParamExpr,
    /// 최대값 표현식
    max: ParamExpr,
  },
}

/// 제약: 파라미터 제약 조건
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Constraint {
  /// 제약 ID
  pub id: String,
  /// 제약 표현식
  pub expr: ConstraintExpr,
  /// Provenance 태그 (선택적)
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub provenance: Option<ProvenanceTag>,
}

/// 타겟 변수: 합성 대상 파라미터 변수
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetVar {
  /// 변수 이름
  pub name: String,
  /// 단위 (선택적)
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub unit: Option<Unit>,
  /// Provenance 태그 (선택적)
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub provenance: Option<ProvenanceTag>,
}

impl TargetVar {
  /// 새 타겟 변수 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(name: impl Into<String>) -> Self {
    Self {
      name: name.into(),
      unit: None,
      provenance: None,
    }
  }
}

/// 파라미터 스펙: 파라미터 합성 명세
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParametricSpec {
  /// 컨텍스트 모드
  pub context: ContextMode,
  /// 파라미터 변수 목록
  pub params: Vec<ParamVar>,
  /// 시그널 참조 목록
  pub signals: Vec<SignalRef>,
  /// 단위 변환 스케일 목록
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub unit_scales: Vec<UnitScale>,
  /// Fixture 목록 (테스트 케이스)
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub fixtures: Vec<Fixture>,
  /// 제약 조건 목록
  pub constraints: Vec<Constraint>,
  /// 타겟 변수 (합성 대상)
  pub target: TargetVar,
}

/// Fixture: 테스트 케이스 (파라미터와 시그널 값)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fixture {
  /// Fixture ID
  pub id: String,
  /// 파라미터 값 맵 (파라미터 이름 → 값)
  #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
  pub params: BTreeMap<String, ParamValue>,
  /// 시그널 값 맵 (시그널 이름 → 값)
  #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
  pub signals: BTreeMap<String, ParamValue>,
}
