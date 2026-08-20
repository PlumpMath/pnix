//! SymbolicBridge 구조 정의
//!
//! pnix-old의 pnix_runtime/src/bridge.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 실행 로직 제외
//! - SymbolicBridge: 심볼릭 브릿지 구조 정의
//! - ResourceLimits: 리소스 제한 구조 정의 (symbolic_core에서 재사용)
//! - 실제 실행 로직 (normalize, diff, simulate 등)은 executor에서 구현

use serde::{Deserialize, Serialize};

/// 심볼릭 연산 리소스 제한
///
/// e-graph 연산 제한을 정의합니다.
/// 실제 제한 적용은 executor에서 수행됩니다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
  /// 최대 e-graph 노드 수
  pub max_nodes: Option<usize>,
  /// 최대 반복 횟수
  pub max_iterations: Option<usize>,
  /// 최대 깊이
  pub max_depth: Option<usize>,
  /// 타임아웃 (초)
  pub timeout_seconds: Option<f64>,
}

impl ResourceLimits {
  /// 엄격한 제한 생성
  pub fn strict() -> Self {
    Self {
      max_nodes: Some(1000),
      max_iterations: Some(50),
      max_depth: Some(20),
      timeout_seconds: Some(1.0),
    }
  }

  /// 제한 없음
  pub fn unlimited() -> Self {
    Self {
      max_nodes: None,
      max_iterations: None,
      max_depth: None,
      timeout_seconds: None,
    }
  }
}

impl Default for ResourceLimits {
  fn default() -> Self {
    // Keep defaults explicit to avoid accidental recursion.
    ResourceLimits {
      max_nodes: Some(10000),
      max_iterations: Some(100),
      max_depth: Some(50),
      timeout_seconds: Some(5.0),
    }
  }
}

/// Symbolic 브릿지 구조
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 정의만, 실행 로직 제외
/// - limits: 리소스 제한 설정 (구조 정의)
/// - 실제 심볼릭 연산 실행은 executor에서 구현
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolicBridge {
  /// 리소스 제한 설정
  pub limits: ResourceLimits,
}

impl SymbolicBridge {
  /// 기본 제한으로 브릿지 생성
  pub fn new() -> Self {
    Self {
      limits: ResourceLimits::default(),
    }
  }

  /// 명시적 리소스 제한으로 브릿지 생성
  pub fn with_limits(limits: ResourceLimits) -> Self {
    Self { limits }
  }

  /// 리소스 제한 조회
  pub fn limits(&self) -> &ResourceLimits {
    &self.limits
  }
}

impl Default for SymbolicBridge {
  fn default() -> Self {
    Self::new()
  }
}

// 헌법 준수 (P0-1): 실행 로직 제거
// 다음 함수들은 executor/runtime 계층에서 구현하세요:
// - normalize(expr, ctx) -> NormalizeResponse
// - diff(expr, var) -> DiffResponse
// - simulate(expr, ctx, t_min, t_max, steps) -> SimulateResponse
// - tensor_contract(expr, ctx) -> TensorContractResponse
// - to_latex(expr) -> String
// - is_scalar(expr) -> bool
//
// 이 함수들은 값 계산 및 실행을 수행하므로 pnix-core에서 제외됩니다.

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_symbolic_bridge_creation() {
    let bridge = SymbolicBridge::new();
    assert_eq!(bridge.limits.max_nodes, Some(10000));
  }

  #[test]
  fn test_symbolic_bridge_with_limits() {
    let limits = ResourceLimits::strict();
    let bridge = SymbolicBridge::with_limits(limits);
    assert_eq!(bridge.limits.max_nodes, Some(1000));
  }

  #[test]
  fn test_resource_limits_default() {
    let limits = ResourceLimits::default();
    assert!(limits.max_nodes.is_some());
    assert!(limits.max_iterations.is_some());
  }

  #[test]
  fn test_resource_limits_unlimited() {
    let limits = ResourceLimits::unlimited();
    assert!(limits.max_nodes.is_none());
    assert!(limits.max_iterations.is_none());
  }
}
