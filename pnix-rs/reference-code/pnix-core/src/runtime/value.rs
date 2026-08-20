//! RuntimeValue - 런타임 값 타입 정의
//!
//! pnix-old의 meaning_core/src/eval.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 값 계산 함수 제외
//! - RuntimeValue enum: 다양한 타입의 런타임 값 표현
//! - LambdaClosure: 클로저 구조 정의
//! - 값 변환 함수 (to_f64, to_string_value, is_truthy 등)는 executor에서 구현
//!
//! ## 설계 원칙
//!
//! - RuntimeValue: 다양한 타입을 지원하는 런타임 값 표현
//! - LambdaClosure: 일급 함수를 값으로 저장하는 구조
//! - 실제 값 계산 및 변환은 executor에서 수행

use crate::ir::IrExpr;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 런타임 값 (다양한 타입 지원)
///
/// Task 35: 문자열/리스트/맵 연산 통합 파이프라인 지원을 위해 추가됨
/// 기존 f64 전용 eval_ir과 호환성을 유지하면서 다양한 타입을 지원
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RuntimeValue {
  /// 실수 값
  Float(
    /// 실수 값
    f64,
  ),
  /// 정수 값
  Int(
    /// 정수 값
    i64,
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
  /// 리스트 값
  List(
    /// 요소 목록
    Vec<RuntimeValue>,
  ),
  /// AttrSet (키-값 쌍)
  AttrSet(
    /// 키-값 쌍 목록
    Vec<(String, RuntimeValue)>,
  ),
  /// 튜플 값
  Tuple(
    /// 요소 목록
    Vec<RuntimeValue>,
  ),
  /// Null/Unit
  Null,
}

impl RuntimeValue {
  /// Float인지 확인 (구조적 검사)
  pub fn is_numeric(&self) -> bool {
    matches!(self, RuntimeValue::Float(_) | RuntimeValue::Int(_))
  }
}

// 헌법 준수 (P0-1): 값 계산 함수 제거
// 다음 함수들은 executor/runtime 계층에서 구현하세요:
// - to_f64() -> f64
// - to_string_value() -> String
// - is_truthy() -> bool
//
// 이 함수들은 값 계산을 수행하므로 pnix-core에서 제외됩니다.

/// Lambda Closure - 일급 함수를 값으로 저장
///
/// 클로저는 생성 시점의 환경을 캡처하여 나중에 실행할 수 있는 함수입니다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LambdaClosure {
  /// 함수 파라미터 목록
  pub params: Vec<String>,
  /// 함수 본문 (IR 표현식)
  pub body: Box<IrExpr>,
  /// 클로저 생성 시점의 캡처된 환경 (자유 변수)
  /// 실제 값 계산은 executor에서 수행
  pub captured: HashMap<String, f64>,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_runtime_value_is_numeric() {
    assert!(RuntimeValue::Float(2.71).is_numeric());
    assert!(RuntimeValue::Int(42).is_numeric());
    assert!(!RuntimeValue::Bool(true).is_numeric());
    assert!(!RuntimeValue::String("test".to_string()).is_numeric());
    assert!(!RuntimeValue::Null.is_numeric());
  }

  #[test]
  fn test_lambda_closure_creation() {
    let closure = LambdaClosure {
      params: vec!["x".to_string()],
      body: Box::new(IrExpr::ConstFloat(0.0)),
      captured: HashMap::new(),
    };
    assert_eq!(closure.params.len(), 1);
    assert_eq!(closure.params[0], "x");
  }
}
