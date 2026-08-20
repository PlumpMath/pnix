//! 런타임 계획 IR: 런타임에서 직접 실행 가능한 중간 표현

use crate::fx::meaning_op::MeaningOpId;

/// 런타임 계획 값: 런타임에서 직접 사용하는 값 타입
#[derive(Debug, Clone, PartialEq)]
pub enum RpValue {
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
  /// 불리언 값
  Bool(
    /// 불리언 값
    bool,
  ),
  /// 문자열 값
  String(
    /// 문자열 값
    String,
  ),
}

/// 런타임 계획 단항 연산자: 단항 연산자 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RpUnaryOp {
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
  /// 논리 부정 (not)
  Not,
}

/// 런타임 계획 이항 연산자: 이항 연산자 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RpBinaryOp {
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
  /// 문자열 연결 (++)
  Concat,
  /// 작음 (<)
  Lt,
  /// 큼 (>)
  Gt,
  /// 작거나 같음 (<=)
  Le,
  /// 크거나 같음 (>=)
  Ge,
  /// 같음 (==)
  Eq,
  /// 다름 (!=)
  Ne,
  /// 논리 AND (&&)
  And,
  /// 논리 OR (||)
  Or,
}

/// 런타임 계획 노드: 런타임 계획 IR의 노드 타입
#[derive(Debug, Clone, PartialEq)]
pub enum RpNode {
  /// 상수 값
  Const(
    /// 값
    RpValue,
  ),
  /// 변수 참조
  Var(
    /// 변수 이름
    String,
  ),
  /// 시그널 가져오기: FRP 시그널에서 값을 가져옴
  GetSignal {
    /// 시그널 이름
    name: String,
  },
  /// 단항 연산
  Unary {
    /// 연산자
    op: RpUnaryOp,
    /// 인자 노드
    arg: Box<RpNode>,
  },
  /// 이항 연산
  Binary {
    /// 연산자
    op: RpBinaryOp,
    /// 왼쪽 피연산자 노드
    lhs: Box<RpNode>,
    /// 오른쪽 피연산자 노드
    rhs: Box<RpNode>,
  },
  /// 조건 선택 (if-then-else)
  Select {
    /// 조건 노드
    cond: Box<RpNode>,
    /// 참일 때 노드
    then_: Box<RpNode>,
    /// 거짓일 때 노드
    else_: Box<RpNode>,
  },
  /// let 바인딩
  Let {
    /// 변수 이름
    name: String,
    /// 값 노드
    value: Box<RpNode>,
    /// 본문 노드
    body: Box<RpNode>,
  },
  /// 리스트
  List(
    /// 요소 노드 목록
    Vec<RpNode>,
  ),
  /// 속성 집합
  AttrSet(
    /// 키-값 쌍 목록 (키, 값 노드)
    Vec<(String, RpNode)>,
  ),
  /// 생성자 호출: ADT variant 생성
  Construct {
    /// variant 이름
    variant: String,
    /// 인자 노드 목록
    args: Vec<RpNode>,
  },
  /// 람다 함수: 익명 함수 정의
  Lambda {
    /// 매개변수 이름
    param: String,
    /// 본문 노드
    body: Box<RpNode>,
  },
  /// 함수 호출: 내장 함수 또는 사용자 정의 함수 호출
  Call {
    /// 함수 이름
    func: String,
    /// 인자 노드 목록
    args: Vec<RpNode>,
  },
  /// Interop 호출: 외부 언어 함수 호출
  InteropCall {
    /// 심볼 이름 (외부 함수 심볼)
    symbol: String,
    /// 인자 노드 목록
    args: Vec<RpNode>,
  },
  /// 파생 연산: 고수준 의미론 연산
  Derived {
    /// 연산 ID
    op: MeaningOpId,
    /// 인자 노드 목록
    args: Vec<RpNode>,
  },
  /// 예외 던지기: 런타임 에러 발생
  Throw {
    /// 에러 메시지
    message: String,
  },
}

/// 런타임 계획: 런타임에서 직접 실행 가능한 실행 계획
#[derive(Debug, Clone, PartialEq)]
pub struct RuntimePlan {
  /// 루트 노드 (계획의 진입점)
  pub root: RpNode,
}
