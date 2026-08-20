//! SymBlock 구조 정의
//!
//! pnix-old의 pnix_runtime/src/sym_block.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 실행 로직 제외
//! - SymBlock: 심볼릭 블록 구조 정의
//! - SymBlockKind: 블록 종류 enum
//! - OutputMode: 출력 모드 enum
//! - 실제 실행 로직 (evaluate, execute, run 등)은 executor에서 구현

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// sym 블록 종류
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SymBlockKind {
  /// 단순 표현식 정규화
  Normalize {
    /// 정규화할 표현식
    expr: String,
    /// 컨텍스트 이름 (선택적)
    context: Option<String>,
  },
  /// 미분
  Diff {
    /// 미분할 표현식
    expr: String,
    /// 미분 변수
    var: String,
  },
  /// 시뮬레이션
  Simulate {
    /// 시뮬레이션할 표현식
    expr: String,
    /// 시작 시간
    t_min: f64,
    /// 종료 시간
    t_max: f64,
    /// 스텝 수
    steps: usize,
  },
  /// 텐서 수축
  TensorContract {
    /// 수축할 텐서 표현식
    expr: String,
  },
}

/// 출력 모드
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputMode {
  /// LaTeX 문자열 반환
  #[default]
  Latex,
  /// 수치 값 반환 (컨텍스트 바인딩 필요)
  Value,
  /// 시뮬레이션 전체 결과
  Values,
  /// 정규화된 심볼릭 표현식 반환
  Expr,
  /// SymbolicResult 전체
  Full,
}

/// 파싱된 sym 블록 구조
///
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 정의만, 실행 로직 제외
/// - kind: 블록 종류 (구조 정의)
/// - output_mode: 출력 모드 (구조 정의)
/// - local_bindings: 로컬 바인딩 (구조 정의, 실제 계산은 executor에서)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SymBlock {
  /// 블록 종류
  pub kind: SymBlockKind,
  /// 출력 모드 (latex, value, expr)
  pub output_mode: OutputMode,
  /// 로컬 바인딩 (sym 블록 내에서만 유효, 구조 정의만)
  pub local_bindings: HashMap<String, f64>,
}

impl SymBlock {
  /// 단순 정규화 블록 생성
  pub fn normalize(expr: &str) -> Self {
    Self {
      kind: SymBlockKind::Normalize {
        expr: expr.to_string(),
        context: None,
      },
      output_mode: OutputMode::default(),
      local_bindings: HashMap::new(),
    }
  }

  /// 컨텍스트 지정 정규화 블록
  pub fn normalize_with_context(expr: &str, context: &str) -> Self {
    Self {
      kind: SymBlockKind::Normalize {
        expr: expr.to_string(),
        context: Some(context.to_string()),
      },
      output_mode: OutputMode::default(),
      local_bindings: HashMap::new(),
    }
  }

  /// 미분 블록 생성
  pub fn diff(expr: &str, var: &str) -> Self {
    Self {
      kind: SymBlockKind::Diff {
        expr: expr.to_string(),
        var: var.to_string(),
      },
      output_mode: OutputMode::default(),
      local_bindings: HashMap::new(),
    }
  }

  /// 시뮬레이션 블록 생성
  pub fn simulate(expr: &str, t_min: f64, t_max: f64, steps: usize) -> Self {
    Self {
      kind: SymBlockKind::Simulate {
        expr: expr.to_string(),
        t_min,
        t_max,
        steps,
      },
      output_mode: OutputMode::default(),
      local_bindings: HashMap::new(),
    }
  }

  /// 텐서 수축 블록 생성
  pub fn tensor_contract(expr: &str) -> Self {
    Self {
      kind: SymBlockKind::TensorContract {
        expr: expr.to_string(),
      },
      output_mode: OutputMode::default(),
      local_bindings: HashMap::new(),
    }
  }

  /// 출력 모드 설정
  pub fn with_output(mut self, mode: OutputMode) -> Self {
    self.output_mode = mode;
    self
  }

  /// 로컬 바인딩 추가 (구조 변경만)
  pub fn with_binding(mut self, var: &str, value: f64) -> Self {
    self.local_bindings.insert(var.to_string(), value);
    self
  }

  /// 여러 바인딩 추가 (구조 변경만)
  pub fn with_bindings(mut self, bindings: HashMap<String, f64>) -> Self {
    self.local_bindings.extend(bindings);
    self
  }
}

// 헌법 준수 (P0-1): 실행 로직 제거
// 다음 함수들은 executor/runtime 계층에서 구현하세요:
// - run() -> SymBlockOutput
// - evaluate() -> f64
// - execute() -> SymbolicResult
//
// 이 함수들은 값 계산 및 실행을 수행하므로 pnix-core에서 제외됩니다.

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_sym_block_normalize() {
    let block = SymBlock::normalize("x + y");
    assert!(matches!(block.kind, SymBlockKind::Normalize { .. }));
  }

  #[test]
  fn test_sym_block_diff() {
    let block = SymBlock::diff("x^2", "x");
    assert!(matches!(block.kind, SymBlockKind::Diff { .. }));
  }

  #[test]
  fn test_sym_block_with_binding() {
    let block = SymBlock::normalize("x + y").with_binding("x", 10.0);
    assert_eq!(block.local_bindings.get("x"), Some(&10.0));
  }
}
