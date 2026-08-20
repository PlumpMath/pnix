//! IR - Backend-agnostic Intermediate Representation
//!
//! SSA에서 최종 실행(eval/LLVM)으로 가기 전 중간 표현
//!
//! ## 헌법 준수 (P0-1)
//!
//! - 구조 정의만 포함, 실행 코드 없음
//! - IrExpr는 표현식의 선언적 표현
//! - 실제 평가는 pnix-executor-graph 또는 외부 evaluator에서 수행
//!
//! ## DrawCmd와의 관계
//!
//! - `draw_ir.rs`의 `DrawCmd`는 그래픽 명령 전용
//! - 이 모듈의 `DrawCmd`는 일반 IR 표현식(`IrExpr`)을 사용하는 더 일반적인 버전
//! - 두 모듈은 서로 다른 목적을 가짐 (중복 아님)

use serde::{Deserialize, Serialize};

/// 그리기 명령 (IrExpr 기반): 그리기 명령 타입
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DrawCmd {
  /// 원: cx, cy, radius, fill_color (optional)
  Circle {
    /// 중심 X 좌표
    cx: Box<IrExpr>,
    /// 중심 Y 좌표
    cy: Box<IrExpr>,
    /// 반지름
    r: Box<IrExpr>,
    /// 채우기 색상 (옵셔널)
    fill: Option<String>,
    /// 테두리 색상 (옵셔널)
    stroke: Option<String>,
    /// 테두리 두께
    stroke_width: f64,
  },
  /// 선: x1, y1, x2, y2, color, width
  Line {
    /// 시작점 X 좌표
    x1: Box<IrExpr>,
    /// 시작점 Y 좌표
    y1: Box<IrExpr>,
    /// 끝점 X 좌표
    x2: Box<IrExpr>,
    /// 끝점 Y 좌표
    y2: Box<IrExpr>,
    /// 선 색상 (옵셔널)
    color: Option<String>,
    /// 선 두께
    width: f64,
  },
  /// 사각형: x, y, w, h
  Rect {
    /// X 좌표
    x: Box<IrExpr>,
    /// Y 좌표
    y: Box<IrExpr>,
    /// 너비
    w: Box<IrExpr>,
    /// 높이
    h: Box<IrExpr>,
    /// 채우기 색상 (옵셔널)
    fill: Option<String>,
    /// 테두리 색상 (옵셔널)
    stroke: Option<String>,
    /// 모서리 반지름
    corner_radius: f64,
  },
  /// 텍스트: x, y, text, font_size
  Text {
    /// X 좌표
    x: Box<IrExpr>,
    /// Y 좌표
    y: Box<IrExpr>,
    /// 텍스트 내용
    text: String,
    /// 폰트 크기
    font_size: f64,
    /// 텍스트 색상 (옵셔널)
    color: Option<String>,
  },
}

/// IR 표현식
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IrExpr {
  /// 실수 상수
  ConstFloat(
    /// 실수 값
    f64,
  ),
  /// 정수 상수
  ConstInt(
    /// 정수 값
    i64,
  ),
  /// 불리언 상수
  ConstBool(
    /// 불리언 값
    bool,
  ),
  /// 문자열 상수
  ConstString(
    /// 문자열 값
    String,
  ),
  /// 리스트 (요소들)
  List(
    /// 요소 목록
    Vec<IrExpr>,
  ),
  /// 튜플 (고정 크기, 여러 값을 묶는 구조)
  /// CT의 다중 outputs나 함수의 다중 반환값을 표현하는 데 사용
  Tuple(
    /// 요소 목록
    Vec<IrExpr>,
  ),
  /// AttrSet (키-값 쌍)
  AttrSet(
    /// 키-값 쌍 목록
    Vec<(String, IrExpr)>,
  ),

  /// 시스템 시간 파라미터
  TimeParam,
  /// 델타 시간 파라미터
  DeltaTime,

  /// 산술 연산
  Add(
    /// 왼쪽 피연산자
    Box<IrExpr>,
    /// 오른쪽 피연산자
    Box<IrExpr>,
  ),
  Sub(
    /// 왼쪽 피연산자
    Box<IrExpr>,
    /// 오른쪽 피연산자
    Box<IrExpr>,
  ),
  Mul(
    /// 왼쪽 피연산자
    Box<IrExpr>,
    /// 오른쪽 피연산자
    Box<IrExpr>,
  ),
  Div(
    /// 왼쪽 피연산자
    Box<IrExpr>,
    /// 오른쪽 피연산자
    Box<IrExpr>,
  ),
  Mod(
    /// 왼쪽 피연산자
    Box<IrExpr>,
    /// 오른쪽 피연산자
    Box<IrExpr>,
  ),
  Neg(
    /// 피연산자
    Box<IrExpr>,
  ),

  /// 수학 함수
  Floor(
    /// 인자
    Box<IrExpr>,
  ),
  Ceil(
    /// 인자
    Box<IrExpr>,
  ),
  Abs(
    /// 인자
    Box<IrExpr>,
  ),
  Sqrt(
    /// 인자
    Box<IrExpr>,
  ),
  Sin(
    /// 인자
    Box<IrExpr>,
  ),
  Cos(
    /// 인자
    Box<IrExpr>,
  ),
  Tan(
    /// 인자
    Box<IrExpr>,
  ),
  /// 지수 함수: e^x
  Exp(
    /// 인자
    Box<IrExpr>,
  ),
  /// 자연 로그: ln(x)
  Log(
    /// 인자
    Box<IrExpr>,
  ),
  /// 거듭제곱: x^y
  Pow(
    /// 밑
    Box<IrExpr>,
    /// 지수
    Box<IrExpr>,
  ),

  /// 비교 연산 (결과: 0.0 또는 1.0)
  Lt(
    /// 왼쪽 피연산자
    Box<IrExpr>,
    /// 오른쪽 피연산자
    Box<IrExpr>,
  ),
  Gt(
    /// 왼쪽 피연산자
    Box<IrExpr>,
    /// 오른쪽 피연산자
    Box<IrExpr>,
  ),
  Le(
    /// 왼쪽 피연산자
    Box<IrExpr>,
    /// 오른쪽 피연산자
    Box<IrExpr>,
  ),
  Ge(
    /// 왼쪽 피연산자
    Box<IrExpr>,
    /// 오른쪽 피연산자
    Box<IrExpr>,
  ),
  Eq(
    /// 왼쪽 피연산자
    Box<IrExpr>,
    /// 오른쪽 피연산자
    Box<IrExpr>,
  ),
  Ne(
    /// 왼쪽 피연산자
    Box<IrExpr>,
    /// 오른쪽 피연산자
    Box<IrExpr>,
  ),

  /// 논리 연산
  And(
    /// 왼쪽 피연산자
    Box<IrExpr>,
    /// 오른쪽 피연산자
    Box<IrExpr>,
  ),
  Or(
    /// 왼쪽 피연산자
    Box<IrExpr>,
    /// 오른쪽 피연산자
    Box<IrExpr>,
  ),
  Not(
    /// 피연산자
    Box<IrExpr>,
  ),

  /// 조건문 (select)
  Select(
    /// 조건
    Box<IrExpr>,
    /// 참일 때 값
    Box<IrExpr>,
    /// 거짓일 때 값
    Box<IrExpr>,
  ),

  // ─────────────────────────────────────────────────────────────────
  // 문자열 연산
  // ─────────────────────────────────────────────────────────────────
  /// 문자열 연결: "hello" + "world" → "helloworld"
  Concat(
    /// 첫 번째 문자열
    Box<IrExpr>,
    /// 두 번째 문자열
    Box<IrExpr>,
  ),
  /// 부분 문자열: substring(str, start, len)
  Substring {
    /// 문자열 표현식
    str: Box<IrExpr>,
    /// 시작 위치
    start: Box<IrExpr>,
    /// 끝 위치
    end: Box<IrExpr>,
  },
  /// 문자열 길이: length("hello") → 5
  StringLength(
    /// 문자열 표현식
    Box<IrExpr>,
  ),
  /// 문자열 비교: "hello" == "world" → false
  StringEq(
    /// 첫 번째 문자열
    Box<IrExpr>,
    /// 두 번째 문자열
    Box<IrExpr>,
  ),

  // ─────────────────────────────────────────────────────────────────
  // 리스트 연산
  // ─────────────────────────────────────────────────────────────────
  /// 리스트 길이: length([1, 2, 3]) → 3
  ListLength(
    /// 리스트 표현식
    Box<IrExpr>,
  ),
  /// 리스트 요소 접근: get([1, 2, 3], 1) → 2
  ListGet {
    /// 리스트 표현식
    list: Box<IrExpr>,
    /// 인덱스 표현식
    index: Box<IrExpr>,
  },
  /// Tuple 필드 접근: getTuple((10, 20), 0) → 10
  TupleGet {
    /// 튜플 표현식
    tuple: Box<IrExpr>,
    /// 숫자 인덱스 (0, 1, 2, ...)
    index: Box<IrExpr>,
  },
  /// 리스트 연결: concat([1, 2], [3, 4]) → [1, 2, 3, 4]
  ListConcat(
    /// 첫 번째 리스트
    Box<IrExpr>,
    /// 두 번째 리스트
    Box<IrExpr>,
  ),
  /// 리스트 맵: map([1, 2, 3], f) → [f(1), f(2), f(3)]
  ListMap {
    /// 리스트 표현식
    list: Box<IrExpr>,
    /// Lambda 또는 함수 참조
    func: Box<IrExpr>,
  },
  /// 리스트 필터: filter([1, 2, 3], pred) → [x | x in list, pred(x)]
  ListFilter {
    /// 리스트 표현식
    list: Box<IrExpr>,
    /// Lambda 또는 함수 참조
    pred: Box<IrExpr>,
  },

  // ─────────────────────────────────────────────────────────────────
  // AttrSet 연산
  // ─────────────────────────────────────────────────────────────────
  /// AttrSet 속성 접근: getAttr({a=1; b=2;}, "a") → 1
  GetAttr {
    /// AttrSet 표현식
    attrs: Box<IrExpr>,
    /// 문자열 또는 문자열 표현식
    key: Box<IrExpr>,
  },
  /// AttrSet 속성 설정: setAttr({a=1;}, "b", 2) → {a=1; b=2;}
  SetAttr {
    /// AttrSet 표현식
    attrs: Box<IrExpr>,
    /// 문자열 또는 문자열 표현식
    key: Box<IrExpr>,
    /// 설정할 값
    value: Box<IrExpr>,
  },
  /// AttrSet 속성 존재 확인: hasAttr({a=1;}, "a") → true
  HasAttr {
    /// AttrSet 표현식
    attrs: Box<IrExpr>,
    /// 문자열 또는 문자열 표현식
    key: Box<IrExpr>,
  },
  /// AttrSet 키 목록: keys({a=1; b=2;}) → ["a", "b"]
  AttrSetKeys(
    /// AttrSet 표현식
    Box<IrExpr>,
  ),
  /// AttrSet 병합: merge({a=1;}, {b=2;}) → {a=1; b=2;}
  AttrSetMerge(
    /// 첫 번째 AttrSet
    Box<IrExpr>,
    /// 두 번째 AttrSet
    Box<IrExpr>,
  ),

  /// 변수 참조 (다른 바인딩의 결과)
  VarRef(
    /// 변수 이름
    String,
  ),

  /// Signal 참조 (FRP 신호 값)
  SignalRef(
    /// 시그널 ID
    usize,
  ),

  // ─────────────────────────────────────────────────────────────────
  // 함수형 프로그래밍 지원 (Lambda, Apply, Let)
  // ─────────────────────────────────────────────────────────────────
  /// Lambda 표현식 (익명 함수)
  /// `x: x + 1` 또는 `{ a, b }: a + b`
  Lambda {
    /// 파라미터 이름들 (단일 또는 패턴)
    params: Vec<String>,
    /// 함수 본문
    body: Box<IrExpr>,
  },

  /// Apply 표현식 (함수 적용)
  /// `f x` 또는 `add 1 2`
  Apply {
    /// 적용할 함수
    func: Box<IrExpr>,
    /// 인자
    arg: Box<IrExpr>,
  },

  /// Let 바인딩 표현식
  /// `let x = 1; y = 2; in x + y`
  Let {
    /// 바인딩들: (이름, 값)
    bindings: Vec<(String, IrExpr)>,
    /// 본문 표현식
    body: Box<IrExpr>,
  },

  // ─────────────────────────────────────────────────────────────────
  // 런타임 에러
  // ─────────────────────────────────────────────────────────────────
  /// 런타임 에러 (예: non-exhaustive match, assertion failure)
  /// 실행 시 에러 메시지와 함께 실패
  Throw(
    /// 에러 메시지
    String,
  ),
}

/// IR 모듈: 여러 바인딩을 포함하는 IR 모듈
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IrModule {
  /// 바인딩들: (이름, 표현식)
  pub bindings: Vec<(String, IrExpr)>,
  /// 그리기 명령들
  pub draw_commands: Vec<DrawCmd>,
}

impl IrModule {
  /// 새 IR 모듈 생성
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
  pub fn add(&mut self, name: impl Into<String>, expr: IrExpr) {
    self.bindings.push((name.into(), expr));
  }

  /// 이름으로 표현식 조회
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn get(&self, name: &str) -> Option<&IrExpr> {
    self
      .bindings
      .iter()
      .find(|(n, _)| n == name)
      .map(|(_, e)| e)
  }

  /// 바인딩 이름 목록
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn names(&self) -> Vec<&str> {
    self.bindings.iter().map(|(n, _)| n.as_str()).collect()
  }

  /// 그리기 명령 추가
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn add_draw(&mut self, cmd: DrawCmd) {
    self.draw_commands.push(cmd);
  }
}

impl Default for IrModule {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_ir_module() {
    let mut module = IrModule::new();
    module.add("x", IrExpr::ConstFloat(42.0));
    module.add(
      "y",
      IrExpr::Add(
        Box::new(IrExpr::VarRef("x".to_string())),
        Box::new(IrExpr::ConstFloat(1.0)),
      ),
    );

    assert_eq!(module.names(), vec!["x", "y"]);
    assert!(module.get("x").is_some());
  }

  #[test]
  fn test_lambda_expr() {
    // x: x + 1
    let lambda = IrExpr::Lambda {
      params: vec!["x".to_string()],
      body: Box::new(IrExpr::Add(
        Box::new(IrExpr::VarRef("x".to_string())),
        Box::new(IrExpr::ConstFloat(1.0)),
      )),
    };

    match lambda {
      IrExpr::Lambda { params, body: _ } => {
        assert_eq!(params, vec!["x"]);
      }
      _ => panic!("Expected Lambda"),
    }
  }

  #[test]
  fn test_apply_expr() {
    // f 42
    let apply = IrExpr::Apply {
      func: Box::new(IrExpr::VarRef("f".to_string())),
      arg: Box::new(IrExpr::ConstFloat(42.0)),
    };

    match apply {
      IrExpr::Apply { func, arg } => {
        match *func {
          IrExpr::VarRef(name) => assert_eq!(name, "f"),
          _ => panic!("Expected VarRef"),
        }
        match *arg {
          IrExpr::ConstFloat(v) => assert_eq!(v, 42.0),
          _ => panic!("Expected ConstFloat"),
        }
      }
      _ => panic!("Expected Apply"),
    }
  }

  #[test]
  fn test_let_expr() {
    // let x = 1; y = 2; in x + y
    let let_expr = IrExpr::Let {
      bindings: vec![
        ("x".to_string(), IrExpr::ConstFloat(1.0)),
        ("y".to_string(), IrExpr::ConstFloat(2.0)),
      ],
      body: Box::new(IrExpr::Add(
        Box::new(IrExpr::VarRef("x".to_string())),
        Box::new(IrExpr::VarRef("y".to_string())),
      )),
    };

    match let_expr {
      IrExpr::Let { bindings, body: _ } => {
        assert_eq!(bindings.len(), 2);
        assert_eq!(bindings[0].0, "x");
        assert_eq!(bindings[1].0, "y");
      }
      _ => panic!("Expected Let"),
    }
  }

  #[test]
  fn test_curried_lambda() {
    // add = x: y: x + y (커링된 함수)
    let curried = IrExpr::Lambda {
      params: vec!["x".to_string()],
      body: Box::new(IrExpr::Lambda {
        params: vec!["y".to_string()],
        body: Box::new(IrExpr::Add(
          Box::new(IrExpr::VarRef("x".to_string())),
          Box::new(IrExpr::VarRef("y".to_string())),
        )),
      }),
    };

    // add 10 32 = ((add 10) 32) = 42
    let applied = IrExpr::Apply {
      func: Box::new(IrExpr::Apply {
        func: Box::new(curried),
        arg: Box::new(IrExpr::ConstFloat(10.0)),
      }),
      arg: Box::new(IrExpr::ConstFloat(32.0)),
    };

    // 구조만 확인 (실제 평가는 evaluator에서)
    match applied {
      IrExpr::Apply { func, arg } => {
        match *arg {
          IrExpr::ConstFloat(v) => assert_eq!(v, 32.0),
          _ => panic!("Expected ConstFloat(32.0)"),
        }
        match *func {
          IrExpr::Apply { .. } => {}
          _ => panic!("Expected nested Apply"),
        }
      }
      _ => panic!("Expected Apply"),
    }
  }
}
