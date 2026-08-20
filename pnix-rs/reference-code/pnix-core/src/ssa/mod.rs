//! SSA IR: Static Single Assignment representation (pure)
//!
//! pnix-old의 meaning_core/src/ssa.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의 및 lowering 함수만, execute() 실행 로직 없음

use serde::{Deserialize, Serialize};

mod context;
pub mod opt;
mod types;

pub use context::SSAEvalContext;
pub use types::{
  lower_fx_program_to_ssa_checked, lower_fx_program_to_ssa_unified,
  lower_fx_program_to_ssa_with_main, lower_fx_to_ssa, SSABlock, SSABuilder, SSAOp,
  SSAProgramUnified, SSAValue, SsaLoweringError, SsaProgramError,
};

// 하위 호환성을 위한 별칭 및 레거시 타입
pub type SsaBlock = SSABlock;
pub type SsaOp = SSAOp;

/// 레거시 SSA 모듈 (하위 호환성)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 정의만, 값 계산 없음
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsaModule {
  /// 모듈 이름
  pub name: String,
  /// SSA 블록 목록
  pub blocks: Vec<SsaBlock>,
}

impl From<SSAProgramUnified> for SsaModule {
  fn from(unified: SSAProgramUnified) -> Self {
    Self {
      name: unified.main_binding.clone().unwrap_or_default(),
      blocks: vec![unified.block],
    }
  }
}
