//! Draw IR - Graphics Rendering Commands
//!
//! pnix-old의 ir.rs에서 DrawCmd와 관련 타입을 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! - 구조 정의만 포함, 실행 코드 없음
//! - DrawCmd는 그래픽 명령어의 선언적 표현
//! - 실제 렌더링은 pnix-executor-graph 또는 외부 렌더러에서 수행
//!
//! ## 사용 예시
//!
//! ```ignore
//! let circle = DrawCmd::Circle {
//!     cx: Box::new(DrawExpr::ConstFloat(100.0)),
//!     cy: Box::new(DrawExpr::ConstFloat(100.0)),
//!     r: Box::new(DrawExpr::ConstFloat(50.0)),
//!     fill: Some("#ff0000".to_string()),
//!     stroke: None,
//!     stroke_width: 0.0,
//! };
//! ```

use serde::{Deserialize, Serialize};

// ============================================================
// Draw Command
// ============================================================

/// 그리기 명령 (선언적)
///
/// SVG/Canvas 렌더링을 위한 명령어 정의
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DrawCmd {
  /// 원: cx, cy, radius, fill_color (optional)
  Circle {
    /// 중심 X 좌표
    cx: Box<DrawExpr>,
    /// 중심 Y 좌표
    cy: Box<DrawExpr>,
    /// 반지름
    r: Box<DrawExpr>,
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
    x1: Box<DrawExpr>,
    /// 시작점 Y 좌표
    y1: Box<DrawExpr>,
    /// 끝점 X 좌표
    x2: Box<DrawExpr>,
    /// 끝점 Y 좌표
    y2: Box<DrawExpr>,
    /// 선 색상 (옵셔널)
    color: Option<String>,
    /// 선 두께
    width: f64,
  },
  /// 사각형: x, y, w, h
  Rect {
    /// X 좌표
    x: Box<DrawExpr>,
    /// Y 좌표
    y: Box<DrawExpr>,
    /// 너비
    w: Box<DrawExpr>,
    /// 높이
    h: Box<DrawExpr>,
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
    x: Box<DrawExpr>,
    /// Y 좌표
    y: Box<DrawExpr>,
    /// 텍스트 내용
    text: String,
    /// 폰트 크기
    font_size: f64,
    /// 텍스트 색상 (옵셔널)
    color: Option<String>,
  },
  /// 경로: SVG path data
  Path {
    /// SVG 경로 데이터 문자열
    d: String,
    /// 채우기 색상 (옵셔널)
    fill: Option<String>,
    /// 테두리 색상 (옵셔널)
    stroke: Option<String>,
    /// 테두리 두께
    stroke_width: f64,
  },
  /// 그룹: 여러 명령을 묶음
  Group {
    /// 하위 명령 목록
    children: Vec<DrawCmd>,
    /// 변환 행렬 (옵셔널)
    transform: Option<String>,
  },
}

// ============================================================
// Draw Expression (for dynamic values)
// ============================================================

/// 그리기 표현식 (동적 값)
///
/// DrawCmd의 속성값으로 사용되는 표현식
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DrawExpr {
  /// 실수 상수
  ConstFloat(f64),
  /// 정수 상수
  ConstInt(i64),
  /// 변수 참조
  VarRef(String),
  /// Signal 참조 (FRP 신호 값)
  SignalRef(usize),

  /// 시스템 시간 파라미터
  TimeParam,
  /// 델타 시간 파라미터
  DeltaTime,

  /// 산술 연산
  Add(Box<DrawExpr>, Box<DrawExpr>),
  Sub(Box<DrawExpr>, Box<DrawExpr>),
  Mul(Box<DrawExpr>, Box<DrawExpr>),
  Div(Box<DrawExpr>, Box<DrawExpr>),
  Mod(Box<DrawExpr>, Box<DrawExpr>),
  Neg(Box<DrawExpr>),

  /// 수학 함수
  Sin(Box<DrawExpr>),
  Cos(Box<DrawExpr>),
  Abs(Box<DrawExpr>),
  Sqrt(Box<DrawExpr>),
  Floor(Box<DrawExpr>),
  Ceil(Box<DrawExpr>),

  /// 조건문
  Select(Box<DrawExpr>, Box<DrawExpr>, Box<DrawExpr>),
}

// ============================================================
// Draw Module
// ============================================================

/// 그리기 모듈 - 여러 명령을 포함
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 정의만, 렌더링 실행 로직 없음
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DrawModule {
  /// 바인딩들: (이름, 표현식)
  pub bindings: Vec<(String, DrawExpr)>,
  /// 그리기 명령들
  pub commands: Vec<DrawCmd>,
  /// 모듈 이름
  pub name: String,
}

impl DrawModule {
  /// 새 그리기 모듈 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(name: impl Into<String>) -> Self {
    Self {
      bindings: Vec::new(),
      commands: Vec::new(),
      name: name.into(),
    }
  }

  /// 바인딩 추가
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변경만, 값 계산 없음
  pub fn add_binding(&mut self, name: impl Into<String>, expr: DrawExpr) {
    self.bindings.push((name.into(), expr));
  }

  /// 이름으로 표현식 조회
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn get_binding(&self, name: &str) -> Option<&DrawExpr> {
    self
      .bindings
      .iter()
      .find(|(n, _)| n == name)
      .map(|(_, e)| e)
  }

  /// 그리기 명령 추가
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변경만, 값 계산 없음
  pub fn add_command(&mut self, cmd: DrawCmd) {
    self.commands.push(cmd);
  }

  /// 바인딩 이름 목록 반환
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn binding_names(&self) -> Vec<&str> {
    self.bindings.iter().map(|(n, _)| n.as_str()).collect()
  }
}

// ============================================================
// Constructors
// ============================================================

#[allow(clippy::should_implement_trait)]
impl DrawExpr {
  /// 실수 상수 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn float(v: f64) -> Self {
    DrawExpr::ConstFloat(v)
  }

  /// 정수 상수 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn int(v: i64) -> Self {
    DrawExpr::ConstInt(v)
  }

  /// 변수 참조 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn var(name: impl Into<String>) -> Self {
    DrawExpr::VarRef(name.into())
  }

  /// 시간 파라미터 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn time() -> Self {
    DrawExpr::TimeParam
  }

  /// 델타 시간 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn dt() -> Self {
    DrawExpr::DeltaTime
  }

  /// 덧셈 표현식 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn add(a: DrawExpr, b: DrawExpr) -> Self {
    DrawExpr::Add(Box::new(a), Box::new(b))
  }

  /// 뺄셈 표현식 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn sub(a: DrawExpr, b: DrawExpr) -> Self {
    DrawExpr::Sub(Box::new(a), Box::new(b))
  }

  /// 곱셈 표현식 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn mul(a: DrawExpr, b: DrawExpr) -> Self {
    DrawExpr::Mul(Box::new(a), Box::new(b))
  }

  /// 나눗셈 표현식 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn div(a: DrawExpr, b: DrawExpr) -> Self {
    DrawExpr::Div(Box::new(a), Box::new(b))
  }
}

impl DrawCmd {
  /// 원 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn circle(cx: f64, cy: f64, r: f64) -> Self {
    DrawCmd::Circle {
      cx: Box::new(DrawExpr::float(cx)),
      cy: Box::new(DrawExpr::float(cy)),
      r: Box::new(DrawExpr::float(r)),
      fill: None,
      stroke: None,
      stroke_width: 0.0,
    }
  }

  /// 원 생성 (색상 포함)
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn circle_filled(cx: f64, cy: f64, r: f64, fill: &str) -> Self {
    DrawCmd::Circle {
      cx: Box::new(DrawExpr::float(cx)),
      cy: Box::new(DrawExpr::float(cy)),
      r: Box::new(DrawExpr::float(r)),
      fill: Some(fill.to_string()),
      stroke: None,
      stroke_width: 0.0,
    }
  }

  /// 선 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn line(x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
    DrawCmd::Line {
      x1: Box::new(DrawExpr::float(x1)),
      y1: Box::new(DrawExpr::float(y1)),
      x2: Box::new(DrawExpr::float(x2)),
      y2: Box::new(DrawExpr::float(y2)),
      color: None,
      width: 1.0,
    }
  }

  /// 사각형 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn rect(x: f64, y: f64, w: f64, h: f64) -> Self {
    DrawCmd::Rect {
      x: Box::new(DrawExpr::float(x)),
      y: Box::new(DrawExpr::float(y)),
      w: Box::new(DrawExpr::float(w)),
      h: Box::new(DrawExpr::float(h)),
      fill: None,
      stroke: None,
      corner_radius: 0.0,
    }
  }

  /// 텍스트 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn text(x: f64, y: f64, text: &str, font_size: f64) -> Self {
    DrawCmd::Text {
      x: Box::new(DrawExpr::float(x)),
      y: Box::new(DrawExpr::float(y)),
      text: text.to_string(),
      font_size,
      color: None,
    }
  }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_draw_module() {
    let mut module = DrawModule::new("test");
    module.add_binding("x", DrawExpr::float(42.0));
    module.add_command(DrawCmd::circle(100.0, 100.0, 50.0));

    assert_eq!(module.binding_names(), vec!["x"]);
    assert!(module.get_binding("x").is_some());
    assert_eq!(module.commands.len(), 1);
  }

  #[test]
  fn test_circle_filled() {
    let circle = DrawCmd::circle_filled(100.0, 100.0, 50.0, "#ff0000");

    match circle {
      DrawCmd::Circle { fill, .. } => {
        assert_eq!(fill, Some("#ff0000".to_string()));
      }
      _ => panic!("Expected Circle"),
    }
  }

  #[test]
  fn test_draw_expr_constructors() {
    let expr = DrawExpr::add(
      DrawExpr::mul(DrawExpr::time(), DrawExpr::float(2.0)),
      DrawExpr::float(100.0),
    );

    match expr {
      DrawExpr::Add(left, right) => {
        assert!(matches!(*left, DrawExpr::Mul(_, _)));
        assert!(matches!(*right, DrawExpr::ConstFloat(100.0)));
      }
      _ => panic!("Expected Add"),
    }
  }

  #[test]
  fn test_animated_circle() {
    // 시간에 따라 움직이는 원
    let animated = DrawCmd::Circle {
      cx: Box::new(DrawExpr::add(
        DrawExpr::float(100.0),
        DrawExpr::mul(
          DrawExpr::Sin(Box::new(DrawExpr::time())),
          DrawExpr::float(50.0),
        ),
      )),
      cy: Box::new(DrawExpr::float(100.0)),
      r: Box::new(DrawExpr::float(25.0)),
      fill: Some("#3366ff".to_string()),
      stroke: None,
      stroke_width: 0.0,
    };

    match animated {
      DrawCmd::Circle { cx, .. } => {
        assert!(matches!(*cx, DrawExpr::Add(_, _)));
      }
      _ => panic!("Expected Circle"),
    }
  }

  #[test]
  fn test_draw_module_serialization() {
    let mut module = DrawModule::new("animation");
    module.add_command(DrawCmd::circle(50.0, 50.0, 25.0));

    let json = serde_json::to_string(&module).unwrap();
    let parsed: DrawModule = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.name, "animation");
    assert_eq!(parsed.commands.len(), 1);
  }

  #[test]
  fn test_group_command() {
    let group = DrawCmd::Group {
      children: vec![
        DrawCmd::circle(50.0, 50.0, 25.0),
        DrawCmd::rect(100.0, 100.0, 50.0, 50.0),
      ],
      transform: Some("translate(10, 10)".to_string()),
    };

    match group {
      DrawCmd::Group {
        children,
        transform,
      } => {
        assert_eq!(children.len(), 2);
        assert!(transform.is_some());
      }
      _ => panic!("Expected Group"),
    }
  }
}
