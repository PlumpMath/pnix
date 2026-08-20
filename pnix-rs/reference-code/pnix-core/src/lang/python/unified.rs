//! PythonNode -> UnifiedExpr bridge (expression subset only).

use crate::lang::pnix::{fx_core_to_unified, PnixError, UnifiedExpr};
use crate::lang::python::convert::{convert_python_to_fx_core, PythonConvertError};
use crate::lang::python_types::PythonNode;

/// Python Unified 에러: Python 노드를 UnifiedExpr로 변환 중 발생하는 에러 타입
#[derive(Debug, Clone)]
pub enum PythonUnifiedError {
  /// 변환 에러: Python AST 변환 중 발생한 에러
  Convert(
    /// Python 변환 에러
    PythonConvertError,
  ),
  /// Lowering 에러: UnifiedExpr lowering 중 발생한 에러
  Lowering(
    /// Pnix 에러
    PnixError,
  ),
}

impl std::fmt::Display for PythonUnifiedError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Convert(err) => write!(f, "Python convert error: {}", err),
      Self::Lowering(err) => write!(f, "Unified lowering error: {}", err),
    }
  }
}

impl std::error::Error for PythonUnifiedError {}

/// PythonNode를 UnifiedExpr로 변환
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 변환만, 값 계산 없음
pub fn python_node_to_unified(node: &PythonNode) -> Result<UnifiedExpr, PythonUnifiedError> {
  let fx = convert_python_to_fx_core(node).map_err(PythonUnifiedError::Convert)?;
  fx_core_to_unified(&fx).map_err(PythonUnifiedError::Lowering)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::lang::python_types::{BinOperator, PythonConstant};

  #[test]
  fn python_node_to_unified_add() {
    let node = PythonNode::BinOp {
      left: Box::new(PythonNode::Constant {
        value: PythonConstant::Int(1),
      }),
      op: BinOperator::Add,
      right: Box::new(PythonNode::Constant {
        value: PythonConstant::Int(2),
      }),
    };

    let unified = python_node_to_unified(&node).unwrap();
    assert!(matches!(unified, UnifiedExpr::Add(_, _)));
  }
}
