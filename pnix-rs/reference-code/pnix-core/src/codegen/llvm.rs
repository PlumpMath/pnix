//! LLVM Backend 구조 정의
//!
//! pnix-old의 meaning_core/src/llvm_backend.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, JIT 실행 로직 제외
//! - LlvmCompiledModule: 컴파일된 LLVM 모듈 구조 정의
//! - 실제 JIT 실행 로직 (run_jit_fx_main, eval_all_bindings 등)은 executor에서 구현

use serde::{Deserialize, Serialize};

/// 컴파일된 LLVM 모듈 구조
///
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 정의만, JIT 실행 로직 제외
/// - binding_names: 바인딩 이름 목록 (구조 정의)
/// - 실제 JIT 실행 및 함수 호출은 executor에서 구현
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlvmCompiledModule {
  /// 바인딩 이름 목록 (순서 보존, 구조 정의만)
  pub binding_names: Vec<String>,
}

impl LlvmCompiledModule {
  /// 새로운 컴파일된 모듈 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new(binding_names: Vec<String>) -> Self {
    Self { binding_names }
  }

  /// 바인딩 이름 목록 조회
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn binding_names(&self) -> &[String] {
    &self.binding_names
  }
}

// 헌법 준수 (P0-1): 실행 로직 제거
// 다음 함수들은 executor/runtime 계층에서 구현하세요:
// - get_fx_main() -> JitFunction (JIT 함수 핸들 얻기)
// - get_binding_fn(name) -> JitFunction (바인딩 함수 핸들 얻기)
// - eval_all_bindings(system_time, delta_time) -> HashMap<String, f64> (모든 바인딩 실행)
// - compile_ir_to_llvm(context, ir_module) -> LlvmCompiledModule (IR → LLVM 컴파일)
// - run_jit_fx_main(compiled, system_time, delta_time) -> f64 (JIT 실행)
//
// 이 함수들은 값 계산 및 실행을 수행하므로 pnix-core에서 제외됩니다.

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_llvm_compiled_module_creation() {
    let names = vec!["main".to_string(), "foo".to_string()];
    let module = LlvmCompiledModule::new(names.clone());
    assert_eq!(module.binding_names(), names.as_slice());
  }
}
