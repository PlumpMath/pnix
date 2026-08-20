//! Parametric 구조 정의
//!
//! pnix-old의 pnix_io_runtime/src/parametric.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 평가 실행 로직 제외
//! - AttrPath: 속성 경로 구조 정의
//! - EnvBinding: 환경 바인딩 구조 정의
//! - ExprCell: 표현식 셀 구조 정의
//! - 실제 평가 로직 (eval_cell, evaluate 등)은 executor에서 구현

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 수학/물리 상수 모듈
pub mod constants {
  /// π (pi)
  pub const PI: f64 = std::f64::consts::PI;
  /// e (Euler's number)
  pub const E: f64 = std::f64::consts::E;
  /// φ (phi) - golden ratio
  pub const PHI: f64 = 1.618033988749895;
  /// √2
  pub const SQRT2: f64 = std::f64::consts::SQRT_2;
  /// τ (tau) - 2π
  pub const TAU: f64 = std::f64::consts::TAU;
  /// g - standard gravity (m/s²)
  pub const G_ACCEL: f64 = 9.80665;
  /// G - gravitational constant
  pub const G_CONST: f64 = 6.67430e-11;
  /// c - speed of light
  pub const C: f64 = 299_792_458.0;
  /// h - Planck constant
  pub const H: f64 = 6.62607015e-34;
}

/// 속성 경로 (예: "box1.posX" 또는 "world.fps")
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AttrPath {
  /// 경로 부분들
  pub parts: Vec<String>,
}

impl AttrPath {
  /// 새로운 경로 생성
  pub fn new(parts: Vec<String>) -> Self {
    Self { parts }
  }

  /// 문자열에서 파싱
  pub fn parse(s: &str) -> Self {
    Self {
      parts: s.split('.').map(String::from).collect(),
    }
  }

  /// 경로 부분들 조회
  pub fn parts(&self) -> &[String] {
    &self.parts
  }

  /// 경로에 부분 추가
  pub fn join(&self, part: &str) -> Self {
    let mut parts = self.parts.clone();
    parts.push(part.to_string());
    Self { parts }
  }

  /// 부모 경로 반환
  pub fn parent(&self) -> Option<Self> {
    if self.parts.len() > 1 {
      Some(Self {
        parts: self.parts[..self.parts.len() - 1].to_vec(),
      })
    } else {
      None
    }
  }
}

/// 표현식 값 타입
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ExprValue {
  /// 숫자 값
  Number(f64),
  /// 문자열 값
  String(String),
  /// 불리언 값
  Bool(bool),
}

/// 환경 바인딩 타입
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum EnvBinding {
  /// 상수 값
  Constant(ExprValue),
  /// 다른 속성 경로 참조
  AttrRef(AttrPath),
  /// 외부 입력 파라미터 참조 (예: param.time, param.mouseX)
  ParamRef(String),
  /// 내장 상수 ($PI, $E 등)
  BuiltinConstant(String),
}

/// 원자 핸들 (변경 가능한 값 참조)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AtomHandle {
  /// 원자 ID
  pub id: String,
}

impl AtomHandle {
  /// 새로운 핸들 생성
  pub fn new(id: String) -> Self {
    Self { id }
  }
}

/// 표현식 셀 구조
///
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 정의만, 평가 실행 로직 제외
/// - fn_expr: 함수 표현식 (구조 정의)
/// - args: 인자 바인딩 (구조 정의)
/// - 실제 평가는 executor에서 구현
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExprCell {
  /// 함수 표현식 (구조 정의만)
  pub fn_expr: String,
  /// 인자 바인딩 (구조 정의만)
  pub args: HashMap<String, EnvBinding>,
}

impl ExprCell {
  /// 새로운 표현식 셀 생성
  pub fn new(fn_expr: String, args: HashMap<String, EnvBinding>) -> Self {
    Self { fn_expr, args }
  }
}

// 헌법 준수 (P0-1): 실행 로직 제거
// 다음 함수들은 executor/runtime 계층에서 구현하세요:
// - eval_cell(cell, world, params) -> ExprValue
// - evaluate(path, world, params) -> ExprValue
// - constants::get(name) -> Option<f64> (값 반환)
//
// 이 함수들은 값 계산 및 실행을 수행하므로 pnix-core에서 제외됩니다.

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_attr_path_parse() {
    let path = AttrPath::parse("box1.posX");
    assert_eq!(path.parts(), &["box1", "posX"]);
  }

  #[test]
  fn test_attr_path_join() {
    let path = AttrPath::parse("box1");
    let joined = path.join("posX");
    assert_eq!(joined.parts(), &["box1", "posX"]);
  }

  #[test]
  fn test_expr_cell_creation() {
    let mut args = HashMap::new();
    args.insert("t".to_string(), EnvBinding::ParamRef("time".to_string()));
    let cell = ExprCell::new("sin(t)".to_string(), args);
    assert_eq!(cell.fn_expr, "sin(t)");
  }
}
