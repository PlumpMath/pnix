//! ClojureRuntime 구조 정의
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 실행 로직 제외
//! - ClojureRuntime: Clojure 런타임 구조 정의
//! - RuntimeError: 에러 타입 정의
//! - 실제 실행 로직 (eval_at, tick, run_ticks 등)은 executor에서 구현

use serde::{Deserialize, Serialize};

use crate::ir::IrModule;

/// Clojure 런타임 에러 타입
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClojureRuntimeError {
  /// 파싱 에러
  Parse(String),
  /// 평가 에러
  Eval(String),
  /// 바인딩을 찾을 수 없음
  BindingNotFound(String),
}

/// Clojure 런타임 구조
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 정의만, 실행 로직 제외
/// - ir_module: 컴파일된 IR 모듈 (구조 정의)
/// - binding_names: 바인딩 이름 목록 (구조 정의)
/// - 실제 평가 및 실행은 executor에서 구현
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClojureRuntime {
  /// 컴파일된 IR 모듈
  pub ir_module: IrModule,
  /// 바인딩 이름 목록
  pub binding_names: Vec<String>,
}

impl ClojureRuntime {
  /// IR 모듈과 바인딩 이름으로 런타임 생성
  pub fn new(ir_module: IrModule, binding_names: Vec<String>) -> Self {
    Self {
      ir_module,
      binding_names,
    }
  }

  /// 모든 바인딩 이름 반환
  pub fn binding_names(&self) -> &[String] {
    &self.binding_names
  }

  /// IR 모듈 참조
  pub fn ir_module(&self) -> &IrModule {
    &self.ir_module
  }
}

// 헌법 준수 (P0-1): 실행 로직 제거
// 다음 함수들은 executor/runtime 계층에서 구현하세요:
// - from_source(source) -> Result<Self>
// - from_fx_program(prog) -> Result<Self>
// - eval_at(time) -> HashMap<String, f64>
// - get(name) -> Option<f64>
// - run_ticks(t0, dt, steps, on_tick)
// - current_time() -> f64
// - tick(dt)
// - eval_clj(source, time) -> Result<f64>
// - compile_clj(source) -> Result<FxProgram>
//
// 이 함수들은 값 계산 및 실행을 수행하므로 pnix-core에서 제외됩니다.

// 하위 호환: AnkhRuntime → ClojureRuntime 별칭
#[deprecated(since = "0.1.0", note = "use `ClojureRuntime` instead")]
pub type AnkhRuntime = ClojureRuntime;

#[deprecated(since = "0.1.0", note = "use `ClojureRuntimeError` instead")]
pub type AnkhRuntimeError = ClojureRuntimeError;

#[cfg(test)]
mod tests {
  use super::*;
  use crate::ir::IrModule;

  #[test]
  fn test_clojure_runtime_creation() {
    let module = IrModule {
      bindings: vec![],
      draw_commands: vec![],
    };
    let runtime = ClojureRuntime::new(module, vec!["main".to_string()]);
    assert_eq!(runtime.binding_names(), &["main"]);
  }

  #[test]
  fn test_runtime_error_parse() {
    let err = ClojureRuntimeError::Parse("syntax error".to_string());
    assert!(matches!(err, ClojureRuntimeError::Parse(_)));
  }
}
