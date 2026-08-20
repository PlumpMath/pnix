//! SymExpr → IR Lowering
//!
//! 심볼릭 표현을 수치 평가용 IR로 변환
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 변환만, 실제 수치 평가는 pnix-executor에서

mod ir;
mod to_ir;

pub use ir::{BinOpKind, IrInst, IrProgram, UnaryOpKind};
pub use to_ir::{contains_tensor, lower_to_ir, IrLowering, LowerError};
