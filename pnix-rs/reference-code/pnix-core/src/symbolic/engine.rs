//! SymbolicEngine 구조 정의
//!
//! pnix-old의 symbolic_core/src/api/engine.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 실행 로직 제외
//! - SymbolicEngine: 심볼릭 엔진 구조 정의
//! - 실제 실행 로직 (normalize, diff, simulate 등)은 executor에서 구현

use serde::{Deserialize, Serialize};

use crate::runtime::symbolic_bridge::ResourceLimits;

/// SymbolicEngine 구조
///
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 정의만, 실행 로직 제외
/// - limits: 리소스 제한 설정 (구조 정의)
/// - 실제 심볼릭 연산 실행은 executor에서 구현
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolicEngine {
  /// 리소스 제한 설정 (구조 정의만)
  pub limits: ResourceLimits,
}

impl SymbolicEngine {
  /// 기본 제한으로 엔진 생성
  pub fn new() -> Self {
    Self {
      limits: ResourceLimits::default(),
    }
  }

  /// 명시적 리소스 제한으로 엔진 생성
  pub fn with_limits(limits: ResourceLimits) -> Self {
    Self { limits }
  }

  /// 리소스 제한 조회
  pub fn limits(&self) -> &ResourceLimits {
    &self.limits
  }
}

impl Default for SymbolicEngine {
  fn default() -> Self {
    Self::new()
  }
}

// 헌법 준수 (P0-1): 실행 로직 제거
// 다음 함수들은 executor/runtime 계층에서 구현하세요:
// - parse_expr(src) -> SymExpr (표현식 파싱)
// - parse_tensor(src, ctx) -> SymExpr (텐서 표현식 파싱)
// - normalize(req) -> NormalizeResponse (수식 정규화)
// - diff(req) -> DiffResponse (수식 미분)
// - simulate(req) -> SimulateResponse (시뮬레이션)
// - tensor_contract(req) -> TensorContractResponse (텐서 수축)
// - to_latex(expr) -> String (LaTeX 변환)
// - is_scalar(expr) -> bool (스칼라 확인)
//
// 이 함수들은 값 계산 및 실행을 수행하므로 pnix-core에서 제외됩니다.

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_symbolic_engine_creation() {
    let engine = SymbolicEngine::new();
    assert_eq!(engine.limits.max_nodes, Some(10000));
  }

  #[test]
  fn test_symbolic_engine_with_limits() {
    let limits = ResourceLimits::strict();
    let engine = SymbolicEngine::with_limits(limits);
    assert_eq!(engine.limits.max_nodes, Some(1000));
  }
}
