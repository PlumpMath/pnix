//! 의미론적 연산 인터프리터 구조 정의
//!
//! pnix-old의 meaning_core/src/unified_meaning/meaning_interp.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 실행 로직은 executor로 이동
//!
//! ## 설계 원칙
//!
//! - Backend: 인터프리터 백엔드 타입
//! - EffectDescriptor: 지연된 효과 연산 설명자
//! - InterpValue: 인터프리터 값 타입
//! - InterpError: 인터프리터 에러 타입
//! - 실제 실행 로직은 executor에서 구현

use crate::effects::EffectZone;
use serde::{Deserialize, Serialize};

/// 인터프리터 백엔드 타겟
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Backend {
  /// 직접 인터프리터 평가
  Interpreter,
  /// 범주론 변환 (최적화)
  CT,
  /// SSA IR 생성
  IR,
  /// LLVM 네이티브 코드 생성
  LLVM,
}

/// 지연된 효과 연산 설명자 (Free Monad 패턴)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectDescriptor {
  /// 효과 타입/이름
  pub effect_type: String,
  /// 효과가 작동하는 Zone
  pub zone: EffectZone,
  /// 효과 인자들
  pub args: Vec<InterpValue>,
}

/// 인터프리터 결과 값
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InterpValue {
  /// 정수 값
  Int(i64),
  /// 실수 값
  Float(f64),
  /// 불리언 값
  Bool(bool),
  /// 문자열 값
  String(String),
  /// 리스트 값
  List(Vec<InterpValue>),
  /// 속성 집합 (키-값 쌍)
  AttrSet(Vec<(String, InterpValue)>),
  /// 함수/클로저 (불투명)
  Function(String), // 함수 이름/참조
  /// Unit 값
  Unit,
  /// IR 표현식 (IR 백엔드용)
  IrExpr(String),
  /// LLVM 값 참조 (LLVM 백엔드용)
  LlvmRef(String),
  /// 지연 실행을 위한 효과 플레이스홀더 (Free Monad 패턴)
  Effect(EffectDescriptor),
}

// 헌법 준수 (P0-1): 값 계산 함수 제거
// as_int(), as_float(), as_bool() 등의 값 변환 함수는 executor/runtime 계층에서 구현하세요.

/// 인터프리터 에러
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterpError {
  /// 타입 불일치
  TypeError { expected: &'static str, got: String },
  /// 인자 개수 불일치
  ArityError { expected: u8, got: usize },
  /// 백엔드에서 지원하지 않는 연산
  UnsupportedOp {
    op: String, // UnifiedMeaningOp 대신 문자열로 표현
    backend: Backend,
  },
  /// Zone 위반 (순수 컨텍스트에서 효과 연산)
  ZoneViolation {
    op: String, // UnifiedMeaningOp 대신 문자열로 표현
    zone: EffectZone,
  },
  /// 런타임 에러
  RuntimeError(String),
}

impl std::fmt::Display for InterpError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::TypeError { expected, got } => {
        write!(f, "Type error: expected {}, got {}", expected, got)
      }
      Self::ArityError { expected, got } => {
        write!(
          f,
          "Arity error: expected {} arguments, got {}",
          expected, got
        )
      }
      Self::UnsupportedOp { op, backend } => {
        write!(
          f,
          "Unsupported operation {:?} for backend {:?}",
          op, backend
        )
      }
      Self::ZoneViolation { op, zone } => {
        write!(
          f,
          "Zone violation: operation {:?} not allowed in zone {:?}",
          op, zone
        )
      }
      Self::RuntimeError(msg) => write!(f, "Runtime error: {}", msg),
    }
  }
}

impl std::error::Error for InterpError {}

// ─────────────────────────────────────────────
// 참고: meaning_interpreter 실행 로직은 executor로 이동
// ─────────────────────────────────────────────
//
// 다음 함수들은 executor/runtime 계층에서 구현하세요:
// - meaning_interpreter(op, args, backend) -> Result<InterpValue, InterpError>
// - interp_eval(), ct_transform(), ir_generate(), llvm_codegen()
//
// 이 함수들은 값 계산을 수행하므로 pnix-core에서 제외됩니다.

#[cfg(test)]
mod tests {
  use super::*;

  // 헌법 준수 (P0-1): 값 계산 테스트 제거
  // as_int(), as_float(), as_bool() 테스트는 executor에서 수행하세요.

  #[test]
  fn test_effect_descriptor() {
    let desc = EffectDescriptor {
      effect_type: "atom_new".to_string(),
      zone: EffectZone::Stm,
      args: vec![InterpValue::Int(0)],
    };
    assert_eq!(desc.effect_type, "atom_new");
    assert_eq!(desc.zone, EffectZone::Stm);
  }

  #[test]
  fn test_interp_error_display() {
    let err = InterpError::TypeError {
      expected: "int",
      got: "float".to_string(),
    };
    assert!(err.to_string().contains("Type error"));
  }
}
